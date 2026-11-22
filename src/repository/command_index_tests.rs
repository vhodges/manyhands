//! What each command indexes: the fast-writes design's table, "What each
//! write indexes". A write indexes the contexts it changed and observes no
//! other; a command that removes a context deletes its rows and observes
//! nothing; the repository's own commands refresh it whole, as before; and
//! the whole refresh (`index refresh`, a poll's hand-off) observes only the
//! contexts that changed since they were stored.
//!
//! A synchronization (`item sync`, `repo sync`, the sync half of `comment
//! add`) indexes the context it merged into or fast-forwarded, and nothing
//! when it changed nothing locally; the comment hand-off repair indexes the
//! comment's context (Task 16). The rows of `ticket close` and `document
//! promote` are added by Task 22.

use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    rc::Rc,
};

use git2::Repository;
use rusqlite::Connection;

use super::*;
use crate::{
    repository::{
        discovery::{clear_scan_hook, set_context_scan_hook},
        keys::{PassphraseResponse, SessionCredentialProvider, SessionCredentials, UnlockRequest},
    },
    results::{Envelope, ResultCode},
};

/// A ticket on the primary whose context the fixture prepares: the one a
/// command on its own item writes.
pub(super) const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD0";
/// Another ticket whose context the fixture prepares: what a command on
/// `TICKET` leaves as it was, and what a release or a retirement removes.
pub(super) const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD1";
/// A ticket on the primary only.
pub(super) const SPARE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD2";
/// A nonconforming ticket on the primary only: it has no type.
const BROKEN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD3";
/// A document on the primary only.
const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD4";
/// A ticket nothing holds until a create makes it.
const NEW: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD5";
/// The comment a `comment add` writes.
const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD6";

const DOCUMENT_PATH: &str = "docs/guide.md";

/// Every context a command here can read, as the table names it.
pub(super) const CONTEXTS: [&str; 7] = [
    "primary", "TICKET", "OTHER", "SPARE", "BROKEN", "DOCUMENT", "NEW",
];

pub(super) fn ticket_path(id: &str) -> String {
    format!(".manyhands/tickets/{id}/ticket.md")
}

pub(super) fn ticket_source(id: &str, title: &str, extra: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {id}\ntitle: {title}\n\
         type: task\nstatus: open\n{extra}---\nBody of {title}.\n"
    )
}

pub(super) fn write(directory: &Path, path: &str, text: &str) {
    let path = directory.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// Commits `paths` as they are on disk, on the checked-out branch.
pub(super) fn commit(repository: &Repository, paths: &[&str]) {
    let mut index = repository.index().unwrap();
    for path in paths {
        index.add_path(Path::new(path)).unwrap();
    }
    index.write().unwrap();
    let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
    let signature = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    let parent = repository.head().ok().and_then(|h| h.peel_to_commit().ok());
    repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "fixture",
            &tree,
            &parent.iter().collect::<Vec<_>>(),
        )
        .unwrap();
}

pub(super) fn item(id: &str) -> canonical::ItemId {
    id.parse().unwrap()
}

fn ticket_draft(title: &str) -> TicketDraft {
    TicketDraft {
        title: title.to_owned(),
        ticket_type: "task".to_owned(),
        status: "open".to_owned(),
        project: None,
        team: None,
        body: "Body.\n".to_owned(),
    }
}

struct NoPassphrase;

impl SessionCredentialProvider for NoPassphrase {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        panic!("a local write asks for no passphrase")
    }
}

/// What the index stores for a context: its basis, when it was indexed,
/// its mark, and the titles of its items.
pub(super) type Stored = (Option<String>, Option<i64>, i64, Vec<String>);

/// What a command did to the index, watched from just before it ran.
pub(super) struct Watched {
    /// The contexts the command observed, by the table's names.
    pub(super) observed: BTreeSet<&'static str>,
    /// How many times it read each of them, by the table's names.
    pub(super) counts: BTreeMap<&'static str, usize>,
    pub(super) before: BTreeMap<String, Stored>,
    pub(super) after: BTreeMap<String, Stored>,
}

/// An enabled repository whose primary, `main`, holds `TICKET`, `OTHER`,
/// `SPARE`, `BROKEN` and `DOCUMENT`, with the contexts of `TICKET` and
/// `OTHER` prepared, and everything indexed.
pub(super) struct Fixture {
    _directory: tempfile::TempDir,
    pub(super) data: tempfile::TempDir,
    pub(super) root: PathBuf,
    pub(super) service: RepositoryService,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let directory = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let root = directory.path().to_owned();
        let data = tempfile::tempdir().unwrap();
        let repository = Repository::init(&root).unwrap();
        repository.set_head("refs/heads/main").unwrap();
        let mut config = repository.config().unwrap();
        config.set_str("user.name", "Fixture").unwrap();
        config
            .set_str("user.email", "fixture@example.invalid")
            .unwrap();
        config.set_bool("core.autocrlf", false).unwrap();
        write(&root, "fixture.txt", "fixture\n");
        commit(&repository, &["fixture.txt"]);
        let service = RepositoryService::open_at(data.path()).unwrap();
        service
            .enable(EnableRepositoryRequest {
                root: root.clone(),
                primary_branch: "main".into(),
                identity: None,
                operation_id: OperationId::new(),
            })
            .unwrap();
        for (id, title) in [(TICKET, "Ticket"), (OTHER, "Other"), (SPARE, "Spare")] {
            write(&root, &ticket_path(id), &ticket_source(id, title, ""));
        }
        write(
            &root,
            &ticket_path(BROKEN),
            &format!(
                "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {BROKEN}\n\
                 title: Broken\nstatus: open\n---\nBody.\n"
            ),
        );
        write(
            &root,
            DOCUMENT_PATH,
            &format!(
                "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {DOCUMENT}\n\
                 title: Guide\n---\n# Guide\n"
            ),
        );
        let mut paths = vec![canonical::CONFIG_PATH.to_owned(), DOCUMENT_PATH.to_owned()];
        paths.extend([TICKET, OTHER, SPARE, BROKEN].map(ticket_path));
        commit(
            &repository,
            &paths.iter().map(String::as_str).collect::<Vec<_>>(),
        );
        let fixture = Self {
            _directory: directory,
            data,
            root,
            service,
        };
        fixture
            .service
            .refresh_repository(RefreshRepositoryRequest {
                root: fixture.root.clone(),
                operation_id: OperationId::new(),
            })
            .unwrap();
        for id in [TICKET, OTHER] {
            let prepared = fixture
                .service
                .prepare_context(AuthoringTarget {
                    root: fixture.root.clone(),
                    kind: AuthoringKind::Ticket,
                    item_id: item(id),
                    intent: ContextIntent::Edit,
                    operation_id: OperationId::new(),
                })
                .unwrap();
            assert!(
                matches!(prepared, ContextProvisionOutcome::Created(_)),
                "{id}"
            );
        }
        assert_eq!(fixture.status(), IndexStatusState::Current);
        fixture
    }

    pub(super) fn worktree(&self, id: &str) -> PathBuf {
        self.root.join(".manyhands/worktrees").join(id)
    }

    /// The worktree of the context the table names `name`.
    pub(super) fn context(&self, name: &str) -> PathBuf {
        let id = match name {
            "primary" => return self.root.clone(),
            "TICKET" => TICKET,
            "OTHER" => OTHER,
            "SPARE" => SPARE,
            "BROKEN" => BROKEN,
            "DOCUMENT" => DOCUMENT,
            "NEW" => NEW,
            _ => panic!("no context {name}"),
        };
        self.worktree(id)
    }

    pub(super) fn index(&self) -> Connection {
        Connection::open(self.data.path().join(REGISTRY_FILE)).unwrap()
    }

    pub(super) fn status(&self) -> IndexStatusState {
        let repository = self.service.resolve_repository(&self.root).unwrap();
        self.service.index_status(&repository).unwrap().state
    }

    pub(super) fn execute(&self, mutation: Mutation) -> Envelope<MutationDataDto> {
        self.execute_with(None, mutation)
    }

    /// `mutation`, sent as the request `request_id`: what a retry of the
    /// same request sends again.
    pub(super) fn execute_as(
        &self,
        request_id: RequestId,
        mutation: Mutation,
    ) -> Envelope<MutationDataDto> {
        self.service.execute(
            MutationCall {
                request_id,
                confirmation: None,
                mutation,
            },
            &mut SessionCredentials::new(NoPassphrase),
            &mut NoProgress,
            &CancellationToken::new(),
        )
    }

    /// `mutation`, prepared, and then run with the confirmation it was
    /// given.
    fn execute_confirmed(&self, mutation: impl Fn() -> Mutation) -> Envelope<MutationDataDto> {
        let prepared = self.service.prepare(&mutation());
        assert_eq!(prepared.code, ResultCode::Ok, "{prepared:?}");
        let Some(PrepareDataDto::Preview(preview)) = prepared.data else {
            panic!("no preview: {prepared:?}");
        };
        let confirmation = ConfirmationId::parse(&preview.confirmation_id).unwrap();
        self.execute_with(Some(confirmation), mutation())
    }

    fn execute_with(
        &self,
        confirmation: Option<ConfirmationId>,
        mutation: Mutation,
    ) -> Envelope<MutationDataDto> {
        self.service.execute(
            MutationCall {
                request_id: RequestId::new(),
                confirmation,
                mutation,
            },
            &mut SessionCredentials::new(NoPassphrase),
            &mut NoProgress,
            &CancellationToken::new(),
        )
    }

    /// The token the item `id` reads with now.
    fn token(&self, id: &str) -> String {
        let repository = self.service.resolve_repository(&self.root).unwrap();
        self.service
            .show_item(&repository, &item(id))
            .unwrap()
            .observation
            .unwrap()
    }

    fn save(&self, id: &str, title: &str) -> Envelope<MutationDataDto> {
        self.execute(Mutation::TicketSave(TicketSaveInput {
            root: self.root.clone(),
            id: item(id),
            draft: ticket_draft(title),
            deps: RelationshipWrite::Unchanged,
            parent: RelationshipWrite::Unchanged,
            observation: self.token(id),
        }))
    }

    /// What the index stores for each context, by the table's name for
    /// it (by its path for one the table does not name).
    pub(super) fn stored(&self) -> BTreeMap<String, Stored> {
        let names = CONTEXTS
            .iter()
            .map(|name| (self.context(name), *name))
            .collect::<BTreeMap<_, _>>();
        let index = self.index();
        let mut statement = index
            .prepare(
                "SELECT id, worktree_path, basis, indexed_at, dirty FROM contexts
                  ORDER BY worktree_path",
            )
            .unwrap();
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        rows.into_iter()
            .map(|(id, worktree, basis, indexed_at, dirty)| {
                let titles = index
                    .prepare(
                        "SELECT title FROM discovered_items WHERE context_id = ?1 ORDER BY title",
                    )
                    .unwrap()
                    .query_map([id], |row| row.get::<_, Option<String>>(0))
                    .unwrap()
                    .map(|title| title.unwrap().unwrap_or_default())
                    .collect();
                let name = names
                    .get(Path::new(&worktree))
                    .map_or(worktree.clone(), |name| (*name).to_owned());
                (name, (basis, indexed_at, dirty, titles))
            })
            .collect()
    }

    /// Runs `command`, counting each context it observes, with what the
    /// index stores read before and after it.
    pub(super) fn watch(&self, command: impl FnOnce()) -> Watched {
        let counts = CONTEXTS.map(|name| {
            let count = Rc::new(Cell::new(0));
            let counted = count.clone();
            set_context_scan_hook(self.context(name), move || counted.set(counted.get() + 1));
            (name, count)
        });
        let before = self.stored();
        command();
        let after = self.stored();
        for name in CONTEXTS {
            clear_scan_hook(&self.context(name));
        }
        Watched {
            observed: counts
                .iter()
                .filter(|(_, count)| count.get() > 0)
                .map(|(name, _)| *name)
                .collect(),
            counts: counts
                .iter()
                .filter(|(_, count)| count.get() > 0)
                .map(|(name, count)| (*name, count.get()))
                .collect(),
            before,
            after,
        }
    }
}

fn ticket_save(f: &Fixture) -> Watched {
    f.watch(|| {
        let saved = f.save(TICKET, "Saved");
        assert_eq!(saved.code, ResultCode::Ok, "{saved:?}");
    })
}

fn ticket_create(f: &Fixture) -> Watched {
    f.watch(|| {
        let created = f.execute(Mutation::TicketCreate(TicketCreateInput {
            root: f.root.clone(),
            id: item(NEW),
            draft: ticket_draft("Created"),
            deps: Vec::new(),
            parent: None,
            initials: None,
        }));
        assert_eq!(created.code, ResultCode::Ok, "{created:?}");
    })
}

fn slug_assign(f: &Fixture) -> Watched {
    let observation = f.token(TICKET);
    f.watch(|| {
        let assigned = f.execute(Mutation::TicketSlugAssign(TicketSlugAssignInput {
            root: f.root.clone(),
            id: item(TICKET),
            observation,
            initials: Some("ab".to_owned()),
        }));
        assert_eq!(assigned.code, ResultCode::Ok, "{assigned:?}");
    })
}

fn comment_add(f: &Fixture) -> Watched {
    f.watch(|| {
        let added = f.execute(Mutation::CommentAdd(CommentAddInput {
            root: f.root.clone(),
            kind: AuthoringKind::Ticket,
            id: item(TICKET),
            comment_id: item(COMMENT),
            parent_id: None,
            body: "A comment.\n".to_owned(),
            created_by: None,
        }));
        assert_eq!(added.code, ResultCode::Ok, "{added:?}");
    })
}

fn context_prepare(f: &Fixture) -> Watched {
    f.watch(|| {
        let prepared = f.execute(Mutation::ContextPrepare(ContextPrepareInput {
            root: f.root.clone(),
            kind: AuthoringKind::Ticket,
            id: item(SPARE),
            intent: ContextPrepareIntent::Edit,
        }));
        assert_eq!(prepared.code, ResultCode::Ok, "{prepared:?}");
    })
}

/// Main moves on, outside Manyhands; a `context sync-local` of `TICKET`
/// brings it into the context.
fn context_sync_local(f: &Fixture) -> Watched {
    write(&f.root, "docs/incoming.md", "Incoming.\n");
    commit(&Repository::open(&f.root).unwrap(), &["docs/incoming.md"]);
    let repository = f.service.resolve_repository(&f.root).unwrap();
    let observed = f
        .service
        .observe_local_context_sync(&repository, AuthoringKind::Ticket, &item(TICKET))
        .unwrap();
    f.watch(|| {
        let synced = f.execute(Mutation::LocalContextSync(LocalContextSyncInput {
            root: f.root.clone(),
            kind: AuthoringKind::Ticket,
            id: item(TICKET),
            observation: observed.observation,
        }));
        assert_eq!(synced.code, ResultCode::Ok, "{synced:?}");
    })
}

/// `TICKET` is saved in its context and edited on main; the
/// `context sync-local` that meets the conflict stops, and a
/// `context resolve-local` resolves it.
fn context_resolve_local(f: &Fixture) -> Watched {
    let saved = f.save(TICKET, "Saved");
    assert_eq!(saved.code, ResultCode::Ok, "{saved:?}");
    write(
        &f.root,
        &ticket_path(TICKET),
        &ticket_source(TICKET, "Incoming", ""),
    );
    commit(&Repository::open(&f.root).unwrap(), &[&ticket_path(TICKET)]);
    let repository = f.service.resolve_repository(&f.root).unwrap();
    let observed = f
        .service
        .observe_local_context_sync(&repository, AuthoringKind::Ticket, &item(TICKET))
        .unwrap();
    let stopped = f.execute(Mutation::LocalContextSync(LocalContextSyncInput {
        root: f.root.clone(),
        kind: AuthoringKind::Ticket,
        id: item(TICKET),
        observation: observed.observation,
    }));
    assert_eq!(stopped.code, ResultCode::RecoveryRequired, "{stopped:?}");
    let operation_id = OperationId::parse(stopped.operation_id.as_deref().unwrap()).unwrap();
    let view = f
        .service
        .inspect_local_context_sync(&repository, operation_id)
        .unwrap();
    let input = LocalContextResolveInput {
        root: f.root.clone(),
        operation_id,
        observation: view.observation.unwrap(),
        resolutions: view
            .paths
            .into_iter()
            .map(|path| LocalConflictResolutionInput {
                token: path.token,
                source: ticket_source(TICKET, "Resolved", ""),
            })
            .collect(),
    };
    f.watch(|| {
        let resolved = f.execute(Mutation::LocalContextResolve(input));
        assert_eq!(resolved.code, ResultCode::Ok, "{resolved:?}");
    })
}

fn ticket_repair(f: &Fixture) -> Watched {
    let repository = f.service.resolve_repository(&f.root).unwrap();
    let observation = f
        .service
        .show_path(&repository, None, Path::new(&ticket_path(BROKEN)))
        .unwrap()
        .observation
        .unwrap();
    let repair = TicketRepairInput {
        root: f.root.clone(),
        path: PathBuf::from(ticket_path(BROKEN)),
        observation,
        source: ticket_source(BROKEN, "Repaired", ""),
    };
    f.watch(|| {
        let repaired = f.execute_confirmed(|| Mutation::TicketRepair(repair.clone()));
        assert_eq!(repaired.code, ResultCode::Ok, "{repaired:?}");
    })
}

fn document_move(f: &Fixture) -> Watched {
    let repository = f.service.resolve_repository(&f.root).unwrap();
    let observation = f.token(DOCUMENT);
    let destination = f
        .service
        .observe_path(&repository, None, Path::new("docs/moved.md"))
        .unwrap()
        .observation;
    f.watch(|| {
        let moved = f.execute(Mutation::DocumentMove(DocumentMoveInput {
            root: f.root.clone(),
            id: item(DOCUMENT),
            observation: Some(observation),
            destination_path: PathBuf::from("docs/moved.md"),
            destination_observation: destination,
        }));
        assert_eq!(moved.code, ResultCode::Ok, "{moved:?}");
    })
}

fn context_release(f: &Fixture) -> Watched {
    f.watch(|| {
        let released = f
            .service
            .release_context(ReleaseContextRequest {
                root: f.root.clone(),
                kind: AuthoringKind::Ticket,
                item_id: item(OTHER),
                operation_id: OperationId::new(),
            })
            .unwrap();
        assert_eq!(released, ContextReleaseOutcome::Released);
    })
}

/// `OTHER`'s worktree and branch are removed outside Manyhands, its rows
/// still stored; a `context retire` of it finds it absent.
fn context_retire(f: &Fixture) -> Watched {
    let repository = Repository::open(&f.root).unwrap();
    for name in repository.worktrees().unwrap().iter().flatten() {
        let worktree = repository.find_worktree(name).unwrap();
        if worktree.path() == f.worktree(OTHER) {
            fs::remove_dir_all(worktree.path()).unwrap();
            worktree
                .prune(Some(git2::WorktreePruneOptions::new().valid(true)))
                .unwrap();
        }
    }
    repository
        .find_branch(
            &format!("manyhands/ticket/{OTHER}"),
            git2::BranchType::Local,
        )
        .unwrap()
        .delete()
        .unwrap();
    f.watch(|| {
        let retired = f
            .service
            .retire_context(RetireContextRequest {
                root: f.root.clone(),
                kind: AuthoringKind::Ticket,
                item_id: item(OTHER),
                operation_id: OperationId::new(),
            })
            .unwrap();
        assert!(
            matches!(retired, ContextRetirementOutcome::Final(_)),
            "{retired:?}"
        );
    })
}

/// A save of `TICKET` stops after its commit, before its index step; the
/// abandon of its row closes it.
fn operation_abandon(f: &Fixture) -> Watched {
    *f.service.failure_point.lock().unwrap() = Some(FailurePoint::AfterCheckpointBeforeIndexStep);
    let crashed = f.save(TICKET, "Crashed");
    assert_eq!(crashed.code, ResultCode::DiscoveryPending, "{crashed:?}");
    let operation_id = OperationId::parse(crashed.operation_id.as_deref().unwrap()).unwrap();
    f.watch(|| {
        let abandoned = f
            .service
            .abandon_operation(AbandonOperationRequest {
                root: f.root.clone(),
                operation_id,
                expected: None,
            })
            .unwrap();
        assert!(
            matches!(abandoned, AbandonOperationOutcome::Abandoned(_)),
            "{abandoned:?}"
        );
    })
}

fn folder_create(f: &Fixture) -> Watched {
    f.watch(|| {
        let created = f
            .service
            .create_folder(CreateFolderRequest {
                root: f.root.clone(),
                path: DocumentFolderPath::parse("docs/notes").unwrap(),
                operation_id: OperationId::new(),
            })
            .unwrap();
        assert!(
            matches!(
                created,
                CreateFolderOutcome::Created(FolderDiscovery::Current)
            ),
            "{created:?}"
        );
    })
}

fn add_origin(f: &Fixture) {
    f.service
        .add_remote(AddRemoteRequest {
            root: f.root.clone(),
            name: "origin".into(),
            url: "ssh://fixture@localhost/repository".into(),
            operation_id: OperationId::new(),
        })
        .unwrap();
}

fn select_origin(f: &Fixture) {
    f.service
        .set_publication_remote(SetPublicationRemoteRequest {
            root: f.root.clone(),
            name: Some("origin".into()),
            operation_id: OperationId::new(),
        })
        .unwrap();
}

fn remote_add(f: &Fixture) -> Watched {
    f.watch(|| add_origin(f))
}

fn remote_select(f: &Fixture) -> Watched {
    add_origin(f);
    f.watch(|| select_origin(f))
}

fn remote_remove(f: &Fixture) -> Watched {
    add_origin(f);
    f.watch(|| {
        f.service
            .remove_remote(RemoveRemoteRequest {
                root: f.root.clone(),
                name: "origin".into(),
                operation_id: OperationId::new(),
            })
            .unwrap();
    })
}

/// The repository, enabled in Git, is enabled again through a data
/// directory that does not hold it yet: it is registered there and
/// indexed whole.
fn repo_enable(f: &Fixture) -> Watched {
    let elsewhere = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(elsewhere.path()).unwrap();
    f.watch(|| {
        let enabled = service
            .enable(EnableRepositoryRequest {
                root: f.root.clone(),
                primary_branch: "main".into(),
                identity: None,
                operation_id: OperationId::new(),
            })
            .unwrap();
        assert!(
            matches!(enabled, EnableRepositoryOutcome::AlreadyEnabled),
            "{enabled:?}"
        );
    })
}

fn repo_identity_set(f: &Fixture) -> Watched {
    f.watch(|| {
        f.service
            .set_local_identity(SetLocalIdentityRequest {
                root: f.root.clone(),
                name: "Someone Else".into(),
                email: "else@example.invalid".into(),
                initials: InitialsChange::Unchanged,
                expected: None,
            })
            .unwrap();
    })
}

/// `TICKET`'s file is edited outside Manyhands; `index refresh` finds it,
/// and the index reads current after it.
fn index_refresh(f: &Fixture) -> Watched {
    write(
        &f.worktree(TICKET),
        &ticket_path(TICKET),
        &ticket_source(TICKET, "Edited by hand", ""),
    );
    let watched = f.watch(|| {
        let refreshed = f.execute(Mutation::IndexRefresh(IndexRefreshInput {
            root: f.root.clone(),
        }));
        assert_eq!(refreshed.code, ResultCode::Ok, "{refreshed:?}");
    });
    assert_eq!(f.status(), IndexStatusState::Current);
    watched
}

/// The repository publishes to `origin`, whose `main` is one commit ahead,
/// fetched (the advertisement stands in for the transport): `poll once`
/// fast-forwards the primary and hands off to the whole refresh, and the
/// index reads current after it.
fn poll_once(f: &Fixture) -> Watched {
    let repository = Repository::open(&f.root).unwrap();
    repository
        .remote("origin", "ssh://fixture@127.0.0.1:1/fixture")
        .unwrap();
    let configuration = f.root.join(canonical::CONFIG_PATH);
    let text = fs::read_to_string(&configuration).unwrap() + "publication_remote = \"origin\"\n";
    fs::write(&configuration, text).unwrap();
    commit(&repository, &[canonical::CONFIG_PATH]);
    let refreshed = f.execute(Mutation::IndexRefresh(IndexRefreshInput {
        root: f.root.clone(),
    }));
    assert_eq!(refreshed.code, ResultCode::Ok, "{refreshed:?}");
    let head = repository.head().unwrap().peel_to_commit().unwrap();
    let mut tree = repository.treebuilder(Some(&head.tree().unwrap())).unwrap();
    tree.insert(
        "incoming.txt",
        repository.blob(b"incoming\n").unwrap(),
        0o100644,
    )
    .unwrap();
    let tree = repository.find_tree(tree.write().unwrap()).unwrap();
    let signature = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    let incoming = repository
        .commit(None, &signature, &signature, "incoming", &tree, &[&head])
        .unwrap();
    repository
        .reference("refs/remotes/origin/main", incoming, true, "fixture fetch")
        .unwrap();
    let watched = f.watch(|| {
        super::remote::synthetic_fetch(vec![("refs/heads/main".into(), incoming)]);
        let polled = f.execute(Mutation::PollOnce(PollOnceInput {
            root: f.root.clone(),
        }));
        assert_eq!(polled.code, ResultCode::Ok, "{polled:?}");
    });
    assert_eq!(repository.head().unwrap().target(), Some(incoming));
    assert_eq!(f.status(), IndexStatusState::Current);
    watched
}

/// The synchronization target of `TICKET`'s context.
fn ticket_target() -> SynchronizationTarget {
    SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: item(TICKET),
    }
}

/// Main moves on by a commit made outside Manyhands; its head.
fn main_moves_on(f: &Fixture) -> git2::Oid {
    let repository = Repository::open(&f.root).unwrap();
    write(&f.root, "docs/incoming.md", "Incoming.\n");
    commit(&repository, &["docs/incoming.md"]);
    repository.head().unwrap().target().unwrap()
}

/// `TICKET`'s context merges main, as a synchronization's local update
/// merges it; the merge.
fn merge_main_into_ticket(f: &Fixture) -> git2::Oid {
    let context = Repository::open(f.worktree(TICKET)).unwrap();
    let ours = context.head().unwrap().peel_to_commit().unwrap();
    let theirs = context
        .find_commit(context.refname_to_id("refs/heads/main").unwrap())
        .unwrap();
    let mut index = context.merge_commits(&ours, &theirs, None).unwrap();
    assert!(!index.has_conflicts());
    let tree = context
        .find_tree(index.write_tree_to(&context).unwrap())
        .unwrap();
    let signature = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    let merged = context
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            &format!("Merge primary into ticket {TICKET}"),
            &tree,
            &[&ours, &theirs],
        )
        .unwrap();
    context
        .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    merged
}

/// The repository's configuration names `origin` as its publication
/// remote, committed on main, and the index holds it.
fn publish_to_origin(f: &Fixture) {
    let repository = Repository::open(&f.root).unwrap();
    repository
        .remote("origin", "ssh://fixture@127.0.0.1:1/fixture")
        .unwrap();
    let configuration = f.root.join(canonical::CONFIG_PATH);
    let text = fs::read_to_string(&configuration).unwrap() + "publication_remote = \"origin\"\n";
    fs::write(&configuration, text).unwrap();
    commit(&repository, &[canonical::CONFIG_PATH]);
    let refreshed = f.execute(Mutation::IndexRefresh(IndexRefreshInput {
        root: f.root.clone(),
    }));
    assert_eq!(refreshed.code, ResultCode::Ok, "{refreshed:?}");
}

/// A synchronization of `target` that published `oid`, its push stood in
/// for by its classification, runs its hand-off: what the synchronization
/// indexes once its push is verified.
fn synchronized(f: &Fixture, target: SynchronizationTarget, oid: git2::Oid) -> Watched {
    let replay = f.service.classified_synchronization_for_testing(
        &f.root,
        &target,
        OperationId::new(),
        oid,
        true,
    );
    f.watch(|| {
        let synced = f
            .service
            .synchronize_remote(replay, &mut SessionCredentials::new(NoPassphrase))
            .unwrap();
        assert!(
            matches!(synced, SynchronizationResult::Complete(_)),
            "{synced:?}"
        );
    })
}

/// Main moves on, and `item sync` of `TICKET` merges it into the context
/// and publishes the merge: the merge is in `TICKET`'s context alone.
fn item_sync_merged(f: &Fixture) -> Watched {
    main_moves_on(f);
    let merged = merge_main_into_ticket(f);
    synchronized(f, ticket_target(), merged)
}

/// `item sync` of `TICKET` publishes its context as it is: nothing local
/// changed.
fn item_sync_pushed(f: &Fixture) -> Watched {
    let head = Repository::open(f.worktree(TICKET))
        .unwrap()
        .head()
        .unwrap()
        .target()
        .unwrap();
    synchronized(f, ticket_target(), head)
}

/// `repo sync` brings main a commit ahead (made here, standing in for
/// the fast-forward to the remote's) and publishes it.
fn repo_sync(f: &Fixture) -> Watched {
    let head = main_moves_on(f);
    synchronized(f, SynchronizationTarget::Primary, head)
}

/// `TICKET`'s comment, as `comment add` makes it.
fn comment_request() -> PublishCommentRequest {
    PublishCommentRequest {
        comment: SubmitCommentRequest {
            target: AuthoringTarget {
                root: PathBuf::new(),
                kind: AuthoringKind::Ticket,
                item_id: item(TICKET),
                intent: ContextIntent::Edit,
                operation_id: OperationId::new(),
            },
            comment_id: item(COMMENT),
            parent_id: None,
            body: "A comment.\n".to_owned(),
            expected_destination: ExpectedPathObservation::Missing,
        },
        approval: None,
        confirmed_identity: None,
    }
}

/// `comment add` with a publication remote: its local checkpoint, and then
/// its synchronization, whose push is stood in for by its classification.
fn comment_add_synced(f: &Fixture) -> Watched {
    publish_to_origin(f);
    let mut request = comment_request();
    request.comment.target.root = f.root.clone();
    f.watch(|| {
        let added = f
            .service
            .submit_comment_with(request, None, false, false, &mut |synchronization| {
                let head = Repository::open(f.worktree(TICKET))
                    .unwrap()
                    .head()
                    .unwrap()
                    .target()
                    .unwrap();
                let replay = f.service.classified_synchronization_for_testing(
                    &f.root,
                    &synchronization.target,
                    synchronization.operation_id,
                    head,
                    true,
                );
                Some(
                    f.service
                        .synchronize_remote(replay, &mut SessionCredentials::new(NoPassphrase)),
                )
            })
            .unwrap();
        let CommentSubmissionOutcome::Saved {
            publication,
            indexing,
            ..
        } = added
        else {
            panic!("the comment was not saved");
        };
        assert!(
            matches!(publication, CommentPublicationState::Published { .. }),
            "{publication:?}"
        );
        assert!(!indexing.local_pending && !indexing.remote_pending);
    })
}

/// A comment of `TICKET` whose checkpoint is committed and whose index
/// step did not run; main moves on; the comment's retry repairs its
/// hand-off before it would publish.
fn comment_handoff_repair(f: &Fixture) -> Watched {
    let mut request = comment_request();
    request.comment.target.root = f.root.clone();
    let operation_id = request.comment.target.operation_id;
    *f.service.failure_point.lock().unwrap() = Some(FailurePoint::AfterCheckpointBeforeIndexStep);
    let added = f
        .service
        .submit_comment(request, &mut SessionCredentials::new(NoPassphrase))
        .unwrap();
    let CommentSubmissionOutcome::Saved { indexing, .. } = added else {
        panic!("the comment was not saved");
    };
    assert!(indexing.local_pending);
    main_moves_on(f);
    f.watch(|| {
        let retried = f
            .service
            .retry_comment_publication(
                RetryCommentPublicationRequest {
                    root: f.root.clone(),
                    operation_id,
                    approval: None,
                    confirmed_identity: None,
                    restart: false,
                },
                &mut SessionCredentials::new(NoPassphrase),
            )
            .unwrap();
        let CommentSubmissionOutcome::Saved {
            publication,
            indexing,
            ..
        } = retried
        else {
            panic!("the comment's retry saved nothing");
        };
        assert!(!indexing.local_pending, "{indexing:?}");
        assert!(
            matches!(
                publication,
                CommentPublicationState::Pending {
                    reason: CommentPublicationPendingReason::NoPublicationRemote
                }
            ),
            "{publication:?}"
        );
    })
}

/// A row of the table: the command, how it runs, the contexts it indexes,
/// and the contexts whose rows it deletes.
type Row = (
    &'static str,
    fn(&Fixture) -> Watched,
    &'static [&'static str],
    &'static [&'static str],
);

/// Each command of the table observes the contexts it indexes and no
/// other, deletes the rows of the contexts it removes, and leaves every
/// other context's rows and basis as they were. The repository's own
/// commands keep their whole refresh, which observes only the contexts that
/// changed: none for `remote add` and `remote remove`, which write only Git's
/// configuration; the primary for `remote select`, which writes the
/// publication remote into `.manyhands/config.toml`, a file the primary's
/// observation reads; and every context for `repo enable`, into an index that
/// stores none of them. `repo identity-set` indexes nothing, as before.
#[test]
fn each_command_indexes_only_its_contexts() {
    // A synchronization's push is stood in for by its classification, as
    // `poll once`'s fetch is by its advertisement. `conflict resolve` and
    // `item republish` index through their synchronization's hand-off once
    // it publishes, as the synchronization rows do: a resolved context is
    // marked dirty and indexed alone, and a republish changes nothing
    // locally. With no transport here they come to no hand-off, so they have
    // no row; see the module's description for the other tasks' rows.
    let rows: [Row; 25] = [
        ("ticket save", ticket_save, &["TICKET"], &[]),
        ("ticket create", ticket_create, &["NEW"], &[]),
        ("ticket slug-assign", slug_assign, &["TICKET"], &[]),
        (
            "comment add (local checkpoint)",
            comment_add,
            &["TICKET"],
            &[],
        ),
        ("context prepare", context_prepare, &["SPARE"], &[]),
        ("context sync-local", context_sync_local, &["TICKET"], &[]),
        (
            "context resolve-local",
            context_resolve_local,
            &["TICKET"],
            &[],
        ),
        ("ticket repair", ticket_repair, &["BROKEN"], &[]),
        ("document move", document_move, &["DOCUMENT"], &[]),
        ("context release", context_release, &[], &["OTHER"]),
        ("context retire", context_retire, &[], &["OTHER"]),
        ("operation abandon", operation_abandon, &[], &[]),
        ("folder create", folder_create, &["primary"], &[]),
        ("remote add", remote_add, &[], &[]),
        ("remote select", remote_select, &["primary"], &[]),
        ("remote remove", remote_remove, &[], &[]),
        (
            "repo enable",
            repo_enable,
            &["primary", "TICKET", "OTHER"],
            &[],
        ),
        ("repo identity-set", repo_identity_set, &[], &[]),
        ("index refresh", index_refresh, &["TICKET"], &[]),
        ("poll once", poll_once, &["primary"], &[]),
        (
            "item sync (merged main)",
            item_sync_merged,
            &["TICKET"],
            &[],
        ),
        ("item sync (push only)", item_sync_pushed, &[], &[]),
        ("repo sync", repo_sync, &["primary"], &[]),
        (
            "comment add (with its sync)",
            comment_add_synced,
            &["TICKET"],
            &[],
        ),
        (
            "comment hand-off repair",
            comment_handoff_repair,
            &["TICKET"],
            &[],
        ),
    ];
    // Every row runs, and every row that is not as the table says is named.
    let mut wrong = Vec::new();
    for (command, run, indexed, deleted) in rows {
        let f = Fixture::new();
        let watched = run(&f);
        let expected = indexed.iter().copied().collect::<BTreeSet<_>>();
        if watched.observed != expected {
            wrong.push(format!(
                "{command}: observed {:?}, not {expected:?}",
                watched.observed
            ));
        }
        for context in deleted {
            if !watched.before.contains_key(*context) || watched.after.contains_key(*context) {
                wrong.push(format!("{command}: {context} was not stored, then deleted"));
            }
        }
        for (context, before) in &watched.before {
            let other =
                !indexed.contains(&context.as_str()) && !deleted.contains(&context.as_str());
            if other && watched.after.get(context) != Some(before) {
                wrong.push(format!("{command}: {context}'s rows and basis changed"));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
