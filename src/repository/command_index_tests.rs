//! What each command indexes: the fast-writes design's table, "What each
//! write indexes". A write indexes the contexts it changed and observes no
//! other; a command that removes a context deletes its rows and observes
//! nothing; the repository's own commands refresh it whole, as before.
//!
//! The rows of the other commands of the table are added by the tasks that
//! change them: `index refresh` and `poll once` by Task 12; `item sync`,
//! `repo sync`, the sync half of `comment add`, and the commands that index
//! through a synchronization (`conflict resolve`, `item republish` and the
//! comment hand-off repair) by Task 16; `ticket close` and `document
//! promote` by Task 22.

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
const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD0";
/// Another ticket whose context the fixture prepares: what a command on
/// `TICKET` leaves as it was, and what a release or a retirement removes.
const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD1";
/// A ticket on the primary only.
const SPARE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD2";
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
const CONTEXTS: [&str; 7] = [
    "primary", "TICKET", "OTHER", "SPARE", "BROKEN", "DOCUMENT", "NEW",
];

fn ticket_path(id: &str) -> String {
    format!(".manyhands/tickets/{id}/ticket.md")
}

fn ticket_source(id: &str, title: &str, extra: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {id}\ntitle: {title}\n\
         type: task\nstatus: open\n{extra}---\nBody of {title}.\n"
    )
}

fn write(directory: &Path, path: &str, text: &str) {
    let path = directory.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// Commits `paths` as they are on disk, on the checked-out branch.
fn commit(repository: &Repository, paths: &[&str]) {
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

fn item(id: &str) -> canonical::ItemId {
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
type Stored = (Option<String>, Option<i64>, i64, Vec<String>);

/// What a command did to the index, watched from just before it ran.
struct Watched {
    /// The contexts the command observed, by the table's names.
    observed: BTreeSet<&'static str>,
    before: BTreeMap<String, Stored>,
    after: BTreeMap<String, Stored>,
}

/// An enabled repository whose primary, `main`, holds `TICKET`, `OTHER`,
/// `SPARE`, `BROKEN` and `DOCUMENT`, with the contexts of `TICKET` and
/// `OTHER` prepared, and everything indexed.
struct Fixture {
    _directory: tempfile::TempDir,
    data: tempfile::TempDir,
    root: PathBuf,
    service: RepositoryService,
}

impl Fixture {
    fn new() -> Self {
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

    fn worktree(&self, id: &str) -> PathBuf {
        self.root.join(".manyhands/worktrees").join(id)
    }

    /// The worktree of the context the table names `name`.
    fn context(&self, name: &str) -> PathBuf {
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

    fn index(&self) -> Connection {
        Connection::open(self.data.path().join(REGISTRY_FILE)).unwrap()
    }

    fn status(&self) -> IndexStatusState {
        let repository = self.service.resolve_repository(&self.root).unwrap();
        self.service.index_status(&repository).unwrap().state
    }

    fn execute(&self, mutation: Mutation) -> Envelope<MutationDataDto> {
        self.execute_with(None, mutation)
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
    fn stored(&self) -> BTreeMap<String, Stored> {
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
    fn watch(&self, command: impl FnOnce()) -> Watched {
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
/// commands observe every context, as the whole refresh they keep does;
/// `repo identity-set` indexes nothing, as before.
#[test]
fn each_command_indexes_only_its_contexts() {
    const WHOLE: &[&str] = &["primary", "TICKET", "OTHER"];
    // `conflict resolve`, `item republish` and the comment hand-off repair
    // index through a synchronization: their rows are Task 16's, with the
    // syncs'. See the module's description for the other tasks' rows.
    let rows: [Row; 18] = [
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
        ("remote add", remote_add, WHOLE, &[]),
        ("remote select", remote_select, WHOLE, &[]),
        ("remote remove", remote_remove, WHOLE, &[]),
        ("repo enable", repo_enable, WHOLE, &[]),
        ("repo identity-set", repo_identity_set, &[], &[]),
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
