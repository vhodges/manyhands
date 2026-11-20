//! A write's own index step: what `handoff_post_authoritative` runs once a
//! write has committed, and what `operation resume` and the write's retry run
//! for a row that still owes it (the fast-writes design, "What each write
//! indexes" and "Crash and retry"; P-3). The step indexes the contexts the
//! row records it owes, observing them outside the lease and storing them
//! under it after a basis re-check.

use std::{
    cell::Cell,
    fs,
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, SystemTime},
};

use git2::Repository;
use rusqlite::{Connection, OptionalExtension, params};

use super::*;
use crate::{
    repository::{
        coordination::leases_are_free,
        discovery::{clear_scan_hook, set_context_scan_hook, set_scan_hook},
        keys::{PassphraseResponse, SessionCredentialProvider, SessionCredentials, UnlockRequest},
    },
    results::{DiscoveryEffect, Envelope, RecoveryActionKind, ResultCode},
};

/// A ticket on the primary, which the fixture's first save gives a context.
const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD0";
/// A ticket no context or branch holds until a test creates it.
const NEW: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD1";

fn ticket_path(id: &str) -> String {
    format!(".manyhands/tickets/{id}/ticket.md")
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

fn draft(title: &str) -> TicketDraft {
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

/// Counts each time `observe_context` reads the context at `worktree`, and,
/// for the root, each whole observation of the repository.
struct Reads {
    worktree: PathBuf,
    count: Rc<Cell<usize>>,
}

impl Reads {
    fn of_context(worktree: &Path) -> Self {
        let count = Rc::new(Cell::new(0));
        let counted = count.clone();
        set_context_scan_hook(worktree.to_owned(), move || counted.set(counted.get() + 1));
        Self {
            worktree: worktree.to_owned(),
            count,
        }
    }

    fn of_repository(root: &Path) -> Self {
        let count = Rc::new(Cell::new(0));
        let counted = count.clone();
        set_scan_hook(root.to_owned(), move || counted.set(counted.get() + 1));
        Self {
            worktree: root.to_owned(),
            count,
        }
    }

    fn get(&self) -> usize {
        self.count.get()
    }
}

impl Drop for Reads {
    fn drop(&mut self) {
        clear_scan_hook(&self.worktree);
    }
}

/// An enabled repository whose primary, `main`, holds `TICKET`, indexed.
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
        write(
            &root,
            &ticket_path(TICKET),
            &format!(
                "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {TICKET}\n\
                 title: Primary copy\ntype: task\nstatus: open\n---\nBody.\n"
            ),
        );
        // Enabling leaves its configuration for the caller to commit.
        commit(&repository, &[canonical::CONFIG_PATH, &ticket_path(TICKET)]);
        let fixture = Self {
            _directory: directory,
            data,
            root,
            service,
        };
        fixture.refresh();
        fixture
    }

    /// `new`, with `TICKET` saved once: its first save made its context,
    /// which its index step stored.
    fn with_context() -> Self {
        let fixture = Self::new();
        let saved = fixture.save(RequestId::new(), &fixture.save_input("First save"));
        assert_eq!(saved.code, ResultCode::Ok, "{saved:?}");
        assert_eq!(fixture.title(TICKET), Some("First save".to_owned()));
        assert_eq!(fixture.mark(TICKET), Some(0));
        fixture
    }

    fn worktree(&self, id: &str) -> PathBuf {
        self.root.join(".manyhands/worktrees").join(id)
    }

    fn index(&self) -> Connection {
        Connection::open(self.data.path().join(REGISTRY_FILE)).unwrap()
    }

    fn fail_at(&self, point: FailurePoint) {
        *self.service.failure_point.lock().unwrap() = Some(point);
    }

    fn refresh(&self) {
        let outcome = self
            .service
            .refresh_repository(RefreshRepositoryRequest {
                root: self.root.clone(),
                operation_id: OperationId::new(),
            })
            .unwrap();
        assert!(
            matches!(outcome, RefreshOutcome::Refreshed { .. }),
            "{outcome:?}"
        );
    }

    fn execute(&self, request_id: RequestId, mutation: Mutation) -> Envelope<MutationDataDto> {
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

    /// A save of `TICKET` with `title`, against the ticket as it reads now.
    fn save_input(&self, title: &str) -> TicketSaveInput {
        let repository = self.service.resolve_repository(&self.root).unwrap();
        let observation = self
            .service
            .show_item(&repository, &item(TICKET))
            .unwrap()
            .observation
            .unwrap();
        TicketSaveInput {
            root: self.root.clone(),
            id: item(TICKET),
            draft: draft(title),
            deps: RelationshipWrite::Unchanged,
            parent: RelationshipWrite::Unchanged,
            observation,
        }
    }

    fn save(&self, request_id: RequestId, input: &TicketSaveInput) -> Envelope<MutationDataDto> {
        self.execute(request_id, Mutation::TicketSave(input.clone()))
    }

    fn create(&self, request_id: RequestId, id: &str) -> Envelope<MutationDataDto> {
        self.execute(
            request_id,
            Mutation::TicketCreate(TicketCreateInput {
                root: self.root.clone(),
                id: item(id),
                draft: draft("Created"),
                deps: Vec::new(),
                parent: None,
                initials: None,
            }),
        )
    }

    fn resume(&self, operation: &str) -> Envelope<MutationDataDto> {
        self.execute(
            RequestId::new(),
            Mutation::OperationResume(OperationResumeInput {
                root: self.root.clone(),
                operation_id: OperationId::parse(operation).unwrap(),
            }),
        )
    }

    fn abandon(&self, operation: &str) {
        let outcome = self
            .service
            .abandon_operation(AbandonOperationRequest {
                root: self.root.clone(),
                operation_id: OperationId::parse(operation).unwrap(),
                expected: None,
            })
            .unwrap();
        assert!(
            matches!(outcome, AbandonOperationOutcome::Abandoned(_)),
            "{outcome:?}"
        );
    }

    /// The commit the item's branch is at, in its worktree.
    fn head(&self, id: &str) -> String {
        Repository::open(self.worktree(id))
            .unwrap()
            .head()
            .unwrap()
            .target()
            .unwrap()
            .to_string()
    }

    /// The title the index holds for `id` in its own context.
    fn title(&self, id: &str) -> Option<String> {
        self.index()
            .query_row(
                "SELECT discovered_items.title
                   FROM discovered_items JOIN contexts ON contexts.id = discovered_items.context_id
                  WHERE contexts.item_id = ?1 AND discovered_items.item_id = ?1",
                [id],
                |row| row.get(0),
            )
            .optional()
            .unwrap()
    }

    /// The dirty mark of the context of `id`; `None` while none is stored.
    fn mark(&self, id: &str) -> Option<i64> {
        self.index()
            .query_row(
                "SELECT dirty FROM contexts WHERE item_id = ?1",
                [id],
                |row| row.get(0),
            )
            .optional()
            .unwrap()
    }

    /// What the context of `id` was stored with: its basis and indexed time.
    fn stored(&self, id: &str) -> (Option<String>, Option<i64>) {
        self.index()
            .query_row(
                "SELECT basis, indexed_at FROM contexts WHERE item_id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap()
    }

    fn mark_dirty(&self, id: &str) {
        self.index()
            .execute("UPDATE contexts SET dirty = 1 WHERE item_id = ?1", [id])
            .unwrap();
    }

    /// Whether the repository is marked behind as a whole.
    fn behind(&self) -> bool {
        self.index()
            .query_row("SELECT refresh_required FROM repositories", [], |row| {
                row.get(0)
            })
            .unwrap()
    }

    fn status(&self) -> IndexStatusState {
        let repository = self.service.resolve_repository(&self.root).unwrap();
        self.service.index_status(&repository).unwrap().state
    }

    /// The state, step and owed contexts of the row of `operation`.
    fn row(&self, operation: &str) -> (String, Option<String>, Option<String>) {
        self.index()
            .query_row(
                "SELECT state, completed_step, owed_contexts FROM operation_records
                  WHERE operation_ulid = ?1",
                [operation],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap()
    }

    /// The operation of the one pending row.
    fn pending(&self) -> String {
        self.index()
            .query_row(
                "SELECT operation_ulid FROM operation_records WHERE state != 'completed'",
                [],
                |row| row.get(0),
            )
            .unwrap()
    }

    /// A row of a save of `TICKET` whose commit is made and that owes only
    /// its hand-off, as a crashed save leaves it, recording `owed`.
    fn owing_row(&self, owed: Option<&str>) -> (String, RecoveryRecord) {
        let operation = OperationId::new().to_string();
        let index = self.index();
        index
            .execute(
                "INSERT INTO operation_records (
                    repository_id, root_path, operation_ulid, action, target, item_id, state,
                    completed_step, observed_at, owed_contexts
                 ) SELECT id, root_path, ?1, 'save_ticket', ?2, ?3, 'authoritative_observed',
                          'authoring_checkpoint_observed', 1, ?4
                     FROM repositories",
                params![
                    operation,
                    format!("authoring-context-v1/ticket/{TICKET}/edit"),
                    TICKET,
                    owed
                ],
            )
            .unwrap();
        let record = RecoveryRecord {
            id: index.last_insert_rowid(),
            is_new: false,
            is_pending: true,
            completed_step: Some(LocalStep::AuthoringCheckpointObserved),
        };
        (operation, record)
    }
}

fn owed_by(id: &str) -> Option<String> {
    Some(format!("[\"{id}\"]"))
}

/// A crash after a save's commit and before its index step leaves its row
/// owing only the hand-off, recording the context it owes, and that context
/// dirty, holding what was stored before. `operation resume` indexes that
/// context alone (no whole observation of the repository) and completes the
/// row, with no commit of its own; the save, sent again, answers `ok` with
/// its commit.
#[test]
fn a_crash_after_the_commit_leaves_the_context_dirty_and_resume_completes_it() {
    let fixture = Fixture::with_context();
    fixture.fail_at(FailurePoint::AfterCheckpointBeforeIndexStep);
    let request = RequestId::new();
    let input = fixture.save_input("Crashed save");

    let crashed = fixture.save(request, &input);
    assert_eq!(crashed.code, ResultCode::DiscoveryPending, "{crashed:?}");
    let commit = crashed.effects.commit_oid.clone().expect("a commit");
    let operation = crashed.operation_id.clone().expect("an operation");
    assert_eq!(fixture.head(TICKET), commit);
    assert_eq!(
        fixture.row(&operation),
        (
            "authoritative_observed".to_owned(),
            Some("authoring_checkpoint_observed".to_owned()),
            owed_by(TICKET)
        )
    );
    assert_eq!(fixture.title(TICKET), Some("First save".to_owned()));
    assert_eq!(fixture.mark(TICKET), Some(1));
    assert!(
        !fixture
            .service
            .contexts_indexed(&fixture.root, &[ContextKey::Item(item(TICKET))])
    );
    assert_eq!(fixture.status(), IndexStatusState::Stale);

    let whole = Reads::of_repository(&fixture.root);
    let own = Reads::of_context(&fixture.worktree(TICKET));
    let resumed = fixture.resume(&operation);
    assert_eq!(resumed.code, ResultCode::Ok, "{resumed:?}");
    assert_eq!((whole.get(), own.get()), (0, 1));
    assert_eq!(fixture.row(&operation).0, "completed");
    assert_eq!(fixture.head(TICKET), commit);
    assert_eq!(fixture.title(TICKET), Some("Crashed save".to_owned()));
    assert_eq!(fixture.mark(TICKET), Some(0));
    assert!(!fixture.behind());
    assert_eq!(fixture.status(), IndexStatusState::Current);

    let replay = fixture.save(request, &input);
    assert_eq!(
        (
            replay.code,
            replay.effects.discovery,
            replay.effects.commit_oid.as_deref()
        ),
        (
            ResultCode::Ok,
            DiscoveryEffect::Current,
            Some(commit.as_str())
        ),
        "{replay:?}"
    );
    assert_eq!(fixture.head(TICKET), commit);
}

/// A save whose row records the context it owes, and that stops after
/// writing its file and before its commit, leaves that context reading
/// dirty though no mark was written and nothing committed. Its row is
/// mid-write, so a whole refresh is refused behind it; `operation abandon`
/// closes the row and turns what it owed into the context's mark, touching
/// no file, and the next refresh indexes the context as it is on disk.
#[test]
fn a_crash_between_the_dirty_mark_and_the_commit_is_settled_by_the_next_refresh() {
    let fixture = Fixture::with_context();
    let head = fixture.head(TICKET);
    let stored = fixture.stored(TICKET);
    fixture.fail_at(FailurePoint::BeforeCheckpointCommit);

    let stopped = fixture.save(RequestId::new(), &fixture.save_input("Uncommitted save"));
    assert_ne!(stopped.code, ResultCode::Ok, "{stopped:?}");
    let operation = fixture.pending();
    assert_eq!(fixture.row(&operation).2, owed_by(TICKET));
    assert_eq!(fixture.head(TICKET), head);
    assert_eq!(fixture.stored(TICKET), stored);
    assert_eq!(fixture.mark(TICKET), Some(0));
    assert_eq!(fixture.status(), IndexStatusState::Stale);
    let refused = fixture
        .service
        .refresh_repository(RefreshRepositoryRequest {
            root: fixture.root.clone(),
            operation_id: OperationId::new(),
        })
        .unwrap_err();
    assert_eq!(refused.kind, RepositoryErrorKind::RecoveryRequired);

    let file = fs::read(fixture.worktree(TICKET).join(ticket_path(TICKET))).unwrap();
    fixture.abandon(&operation);
    assert_eq!(fixture.row(&operation).0, "completed");
    assert_eq!(fixture.mark(TICKET), Some(1));
    assert!(!fixture.behind());
    assert_eq!(fixture.status(), IndexStatusState::Stale);
    assert_eq!(fixture.head(TICKET), head);
    assert_eq!(
        fs::read(fixture.worktree(TICKET).join(ticket_path(TICKET))).unwrap(),
        file
    );

    fixture.refresh();
    assert_eq!(fixture.title(TICKET), Some("Uncommitted save".to_owned()));
    assert_eq!(fixture.mark(TICKET), Some(0));
    assert_eq!(fixture.status(), IndexStatusState::Current);
    assert_eq!(fixture.head(TICKET), head);
}

/// An index step that fails inside its transaction stores nothing: the
/// context keeps what was stored for it, marked dirty by the checkpoint, and
/// the row still owes its hand-off. The save, sent again, indexes it and
/// completes with its commit.
#[test]
fn a_crash_inside_the_index_transaction_stores_nothing() {
    let fixture = Fixture::with_context();
    let stored = fixture.stored(TICKET);
    fixture.fail_at(FailurePoint::BeforeIndexTransactionCommit);
    let request = RequestId::new();
    let input = fixture.save_input("Unstored save");

    let pending = fixture.save(request, &input);
    assert_eq!(pending.code, ResultCode::DiscoveryPending, "{pending:?}");
    let commit = pending.effects.commit_oid.clone().expect("a commit");
    let operation = pending.operation_id.clone().expect("an operation");
    assert_eq!(fixture.stored(TICKET), stored);
    assert_eq!(fixture.title(TICKET), Some("First save".to_owned()));
    assert_eq!(fixture.mark(TICKET), Some(1));
    let (state, _, owed) = fixture.row(&operation);
    assert_eq!((state.as_str(), owed), ("failed", owed_by(TICKET)));

    let retried = fixture.save(request, &input);
    assert_eq!(
        (retried.code, retried.effects.commit_oid.as_deref()),
        (ResultCode::Ok, Some(commit.as_str())),
        "{retried:?}"
    );
    assert_eq!(fixture.title(TICKET), Some("Unstored save".to_owned()));
    assert_eq!(fixture.mark(TICKET), Some(0));
    assert_eq!(fixture.row(&operation).0, "completed");
}

/// A row whose context is gone by the time it is resumed (its worktree
/// removed outside Manyhands: retiring the row's own item is refused behind
/// the row) treats that context as indexed: its rows are deleted, the row
/// completes, and the index reads current.
#[test]
fn a_resume_whose_context_was_retired_completes() {
    let fixture = Fixture::with_context();
    fixture.fail_at(FailurePoint::AfterCheckpointBeforeIndexStep);
    let crashed = fixture.save(RequestId::new(), &fixture.save_input("Crashed save"));
    assert_eq!(crashed.code, ResultCode::DiscoveryPending, "{crashed:?}");
    let operation = crashed.operation_id.clone().expect("an operation");
    fs::remove_dir_all(fixture.worktree(TICKET)).unwrap();

    let resumed = fixture.resume(&operation);
    assert_eq!(resumed.code, ResultCode::Ok, "{resumed:?}");
    assert_eq!(fixture.row(&operation).0, "completed");
    assert_eq!(fixture.mark(TICKET), None);
    assert!(!fixture.behind());
    assert_eq!(fixture.status(), IndexStatusState::Current);
}

/// `operation abandon` of a row that owes its hand-off leaves what it owed
/// marked: the context's dirty mark where one is stored, and the repository
/// marked behind for a new context nothing is stored for yet. It reads
/// stale before the abandon and after it, and touches no file or commit; the
/// next refresh indexes what the abandoned operation left.
#[test]
fn an_abandon_leaves_the_dirty_marks() {
    for case in ["a stored context", "a new context"] {
        let fixture = Fixture::with_context();
        fixture.fail_at(FailurePoint::AfterCheckpointBeforeIndexStep);
        let (id, crashed) = if case == "a new context" {
            (NEW, fixture.create(RequestId::new(), NEW))
        } else {
            (
                TICKET,
                fixture.save(RequestId::new(), &fixture.save_input("Crashed save")),
            )
        };
        assert_eq!(
            crashed.code,
            ResultCode::DiscoveryPending,
            "{case}: {crashed:?}"
        );
        let operation = crashed.operation_id.clone().expect("an operation");
        let (mark, behind) = if case == "a new context" {
            (None, true)
        } else {
            (Some(1), false)
        };
        // Before the abandon, the row's list is what marks a new context.
        assert_eq!(fixture.mark(id), mark, "{case}");
        assert!(!fixture.behind(), "{case}");
        assert_eq!(fixture.status(), IndexStatusState::Stale, "{case}");
        let head = fixture.head(id);
        let file = fs::read(fixture.worktree(id).join(ticket_path(id))).unwrap();

        fixture.abandon(&operation);
        assert_eq!(fixture.mark(id), mark, "{case}");
        assert_eq!(fixture.behind(), behind, "{case}");
        assert_eq!(fixture.status(), IndexStatusState::Stale, "{case}");
        assert_eq!(fixture.head(id), head, "{case}");
        assert_eq!(
            fs::read(fixture.worktree(id).join(ticket_path(id))).unwrap(),
            file,
            "{case}"
        );

        fixture.refresh();
        assert_eq!(fixture.mark(id), Some(0), "{case}");
        assert!(!fixture.behind(), "{case}");
        assert_eq!(fixture.status(), IndexStatusState::Current, "{case}");
    }
}

/// An index step observes its context with no lease held, then takes the
/// lease and checks the context's basis again before it stores. A change
/// that lands in between (here the ticket's file written again, with the
/// same bytes and another time) stores nothing: the context is left dirty
/// and the row still owes its hand-off. The save, sent again while the file
/// still moves, answers `discovery_pending` again; sent once it has settled,
/// it indexes the context and answers `ok` with its one commit.
#[test]
fn an_index_step_stores_only_after_a_basis_recheck_under_the_lease() {
    let fixture = Fixture::with_context();
    let request = RequestId::new();
    let input = fixture.save_input("Raced save");
    let moved = |fixture: &Fixture| {
        let path = fixture.worktree(TICKET).join(ticket_path(TICKET));
        let (root, data) = (fixture.root.clone(), fixture.data.path().to_owned());
        fixture.service.set_observation_hook_for_testing(move || {
            assert!(leases_are_free(&data, &root), "observed under the lease");
            let bytes = fs::read(&path).unwrap();
            fs::write(&path, &bytes).unwrap();
            fs::File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_modified(SystemTime::now() - Duration::from_secs(10))
                .unwrap();
        });
    };

    moved(&fixture);
    let raced = fixture.save(request, &input);
    assert_eq!(raced.code, ResultCode::DiscoveryPending, "{raced:?}");
    let commit = raced.effects.commit_oid.clone().expect("a commit");
    let operation = raced.operation_id.clone().expect("an operation");
    assert_eq!(fixture.title(TICKET), Some("First save".to_owned()));
    assert_eq!(fixture.mark(TICKET), Some(1));
    let (state, _, owed) = fixture.row(&operation);
    assert_eq!((state.as_str(), owed), ("retry", owed_by(TICKET)));

    moved(&fixture);
    let replay = fixture.save(request, &input);
    assert_eq!(replay.code, ResultCode::DiscoveryPending, "{replay:?}");
    assert_eq!(fixture.title(TICKET), Some("First save".to_owned()));
    assert_eq!(fixture.mark(TICKET), Some(1));
    assert_eq!(fixture.row(&operation).0, "retry");

    let settled = fixture.save(request, &input);
    assert_eq!(
        (settled.code, settled.effects.commit_oid.as_deref()),
        (ResultCode::Ok, Some(commit.as_str())),
        "{settled:?}"
    );
    assert_eq!(fixture.head(TICKET), commit);
    assert_eq!(fixture.title(TICKET), Some("Raced save".to_owned()));
    assert_eq!(fixture.mark(TICKET), Some(0));
    assert_eq!(fixture.row(&operation).0, "completed");
}

/// An index step that cannot take the repository's lease to store (another
/// call holds it past the lease's timeout) stores nothing. The write has
/// committed, so it answers as a raced one does, `discovery_pending` with its
/// commit, not `busy`. Its context is left dirty and its row still owes the
/// hand-off, which `operation resume` completes once the lease is free.
#[test]
fn an_index_step_that_cannot_take_the_lease_leaves_the_write_owing_its_hand_off() {
    let fixture = Fixture::with_context();
    let held = std::sync::Arc::new(std::sync::Mutex::new(None));
    {
        let (held, root, data) = (
            held.clone(),
            fixture.root.clone(),
            fixture.data.path().to_owned(),
        );
        fixture.service.set_observation_hook_for_testing(move || {
            let lease =
                RepositoryService::hold_lease_for_testing(&root, &data, LeaseKind::Repository)
                    .unwrap();
            *held.lock().unwrap() = Some(lease);
        });
    }

    let pending = fixture.save(RequestId::new(), &fixture.save_input("Unleased save"));
    assert!(held.lock().unwrap().is_some(), "the lease was not held");
    assert_eq!(
        (pending.code, pending.effects.discovery),
        (ResultCode::DiscoveryPending, DiscoveryEffect::Pending),
        "{pending:?}"
    );
    let commit = pending.effects.commit_oid.clone().expect("a commit");
    let operation = pending.operation_id.clone().expect("an operation");
    assert_eq!(fixture.head(TICKET), commit);
    let (state, _, owed) = fixture.row(&operation);
    assert_eq!((state.as_str(), owed), ("retry", owed_by(TICKET)));
    assert_eq!(fixture.title(TICKET), Some("First save".to_owned()));
    assert_eq!(fixture.mark(TICKET), Some(1));
    assert_eq!(fixture.status(), IndexStatusState::Stale);

    drop(held.lock().unwrap().take());
    let resumed = fixture.resume(&operation);
    assert_eq!(resumed.code, ResultCode::Ok, "{resumed:?}");
    assert_eq!(fixture.row(&operation).0, "completed");
    assert_eq!(fixture.head(TICKET), commit);
    assert_eq!(fixture.title(TICKET), Some("Unleased save".to_owned()));
    assert_eq!(fixture.mark(TICKET), Some(0));
    assert_eq!(fixture.status(), IndexStatusState::Current);
}

/// A create's index step stores its new context and nothing else: the
/// primary is not observed, and the repository is not marked behind, since
/// the create's own row owes the context while nothing is stored for it.
/// (The create's check that no other source holds its ID still reads the
/// primary's sources, before its commit: that is no observation the index
/// stores.)
#[test]
fn a_create_indexes_its_new_context_and_leaves_the_repository_current() {
    let fixture = Fixture::new();
    let primary = Reads::of_context(&fixture.root);
    let new = Reads::of_context(&fixture.worktree(NEW));

    let created = fixture.create(RequestId::new(), NEW);
    assert_eq!(
        (created.code, created.effects.discovery),
        (ResultCode::Ok, DiscoveryEffect::Current),
        "{created:?}"
    );
    assert_eq!((primary.get(), new.get()), (0, 1));
    assert_eq!(fixture.title(NEW), Some("Created".to_owned()));
    assert_eq!(fixture.mark(NEW), Some(0));
    assert!(!fixture.behind());
    assert_eq!(fixture.status(), IndexStatusState::Current);
}

/// A replay of a save whose row completed reads the marks of the contexts
/// the row recorded before it answers that the index holds its commit:
/// while one is dirty again, it answers `discovery_pending` with its commit
/// and offers a refresh, which no row owes; once indexed, `ok`.
#[test]
fn a_replay_of_a_completed_save_reads_its_contexts_marks() {
    let fixture = Fixture::with_context();
    fixture.fail_at(FailurePoint::AfterCheckpointBeforeIndexStep);
    let request = RequestId::new();
    let input = fixture.save_input("Crashed save");
    let crashed = fixture.save(request, &input);
    assert_eq!(crashed.code, ResultCode::DiscoveryPending, "{crashed:?}");
    let commit = crashed.effects.commit_oid.clone().expect("a commit");
    let operation = crashed.operation_id.clone().expect("an operation");
    let resumed = fixture.resume(&operation);
    assert_eq!(resumed.code, ResultCode::Ok, "{resumed:?}");
    // As a later write of the context that has not been indexed leaves it.
    fixture.mark_dirty(TICKET);

    let replay = fixture.save(request, &input);
    assert_eq!(
        (
            replay.code,
            replay.effects.discovery,
            replay.effects.commit_oid.as_deref()
        ),
        (
            ResultCode::DiscoveryPending,
            DiscoveryEffect::Pending,
            Some(commit.as_str())
        ),
        "{replay:?}"
    );
    assert_eq!(
        replay
            .recovery
            .iter()
            .map(|action| action.action)
            .collect::<Vec<_>>(),
        [RecoveryActionKind::IndexRefresh]
    );

    fixture.refresh();
    let replay = fixture.save(request, &input);
    assert_eq!(
        (
            replay.code,
            replay.effects.discovery,
            replay.effects.commit_oid.as_deref()
        ),
        (
            ResultCode::Ok,
            DiscoveryEffect::Current,
            Some(commit.as_str())
        ),
        "{replay:?}"
    );
}

/// A row that records no contexts, as one an older build began does, is
/// resumed as before: a whole refresh under its ID completes it.
#[test]
fn a_row_that_records_no_contexts_hands_off_to_a_whole_refresh() {
    let fixture = Fixture::with_context();
    let (operation, _) = fixture.owing_row(None);
    let whole = Reads::of_repository(&fixture.root);

    let resumed = fixture.resume(&operation);
    assert_eq!(resumed.code, ResultCode::Ok, "{resumed:?}");
    assert!(whole.get() > 0);
    assert_eq!(fixture.row(&operation).0, "completed");
    assert_eq!(fixture.status(), IndexStatusState::Current);
}

/// A hand-off whose context has not moved since it was stored, and is not
/// dirty, completes its row with no observation; one whose context is
/// dirty, or has moved, indexes it first.
#[test]
fn a_hand_off_whose_context_has_not_moved_completes_without_observing_it() {
    // What changed since the context was stored, and how often it is read.
    for (changed, reads) in [("nothing", 0), ("its mark", 1), ("its file", 1)] {
        let fixture = Fixture::with_context();
        let (operation, record) = fixture.owing_row(None);
        fixture
            .service
            .record_owed_contexts(&fixture.root, record, &[ContextKey::Item(item(TICKET))])
            .unwrap();
        assert_eq!(fixture.row(&operation).2, owed_by(TICKET), "{changed}");
        let mut title = "First save";
        match changed {
            "its mark" => fixture.mark_dirty(TICKET),
            "its file" => {
                title = "Edited by hand";
                let path = fixture.worktree(TICKET).join(ticket_path(TICKET));
                let text = fs::read_to_string(&path).unwrap();
                fs::write(&path, text.replace("First save", title)).unwrap();
            }
            _ => {}
        }
        let read = Reads::of_context(&fixture.worktree(TICKET));

        let outcome = fixture
            .service
            .complete_hand_off_unobserved(&fixture.root, record, ContextKey::Item(item(TICKET)))
            .unwrap();
        assert_eq!(outcome, IndexOutcome::Current, "{changed}");
        assert_eq!(read.get(), reads, "{changed}");
        assert_eq!(fixture.row(&operation).0, "completed", "{changed}");
        assert_eq!(fixture.mark(TICKET), Some(0), "{changed}");
        assert_eq!(fixture.title(TICKET), Some(title.to_owned()), "{changed}");
    }
}
