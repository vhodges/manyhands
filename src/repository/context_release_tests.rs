//! `release_context`, the domain operation, against the design's
//! `context release` table: a test for each check, (a) to (d), one for
//! ownership, and the re-entry rules. Each refusal is shown to leave the
//! worktree, the branch and the index's `contexts` row as they were.

use super::*;
use crate::repository::{
    AddRemoteRequest, AuthoringTarget, ClosureExpectation, ContextIntent, ContextProvisionOutcome,
    EnableRepositoryRequest, REGISTRY_FILE, RefreshOutcome, RefreshRepositoryRequest,
    RemoteOperationAction, RemoteOperationTarget, RemoteRefPlan, RemoteReservationOutcome,
    SetPublicationRemoteRequest,
    closure::record::{self as closure, ClosureKind},
    promotion::{PromotionExpectation, record as promotion},
    recovery,
};
use git2::{Oid, Repository};
use rusqlite::{Connection, params};
use std::{fs, path::Path};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC0";
const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC2";
/// No item on primary holds this ID.
const NEW: &str = "01ARZ3NDEKTSV4RRFFQ69G5FCN";
const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD0";
/// No item on primary holds this ID either.
const NEW_DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FDN";
const DOCUMENT_PATH: &str = "docs/guide.md";
/// Front matter that closes a ticket.
const CLOSURE: &str =
    "closed_at: 2026-09-30T12:34:56Z\nclosed_by: Ada Lovelace <ada@example.invalid>\n";
/// What the fixture's primary ignores.
const IGNORED: &str = "scratch.log";

fn ticket_path(id: &str) -> String {
    format!(".manyhands/tickets/{id}/ticket.md")
}

fn ticket_source(id: &str, extra: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {id}\ntitle: Ticket {id}\n\
         type: task\nstatus: open\n{extra}---\nBody.\n"
    )
}

fn document_source(id: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {id}\ntitle: Guide\n---\n\
         # Guide\n"
    )
}

/// Commits `paths` as they are on disk, on the branch `repository` has
/// checked out.
fn commit(repository: &Repository, paths: &[&str]) -> Oid {
    let mut index = repository.index().unwrap();
    for path in paths {
        index.add_path(Path::new(path)).unwrap();
    }
    index.write().unwrap();
    let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
    let signature = git2::Signature::new(
        "Fixture",
        "fixture@example.invalid",
        &git2::Time::new(1_700_000_000, 0),
    )
    .unwrap();
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
        .unwrap()
}

fn write(directory: &Path, path: &str, text: &str) {
    let path = directory.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// An enabled repository whose primary, `main`, holds two open tickets
/// and one document, with no context.
struct Fixture {
    root: tempfile::TempDir,
    data: tempfile::TempDir,
    service: RepositoryService,
}

/// What a check reads of one context: whether its worktree is there,
/// its branch's tip, and whether the index lists it.
type ContextState = (bool, Option<Oid>, bool);

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = tempfile::tempdir().unwrap();
        let repository = Repository::init(root.path()).unwrap();
        repository.set_head("refs/heads/main").unwrap();
        let mut config = repository.config().unwrap();
        config.set_str("user.name", "Fixture").unwrap();
        config
            .set_str("user.email", "fixture@example.invalid")
            .unwrap();
        config.set_bool("core.autocrlf", false).unwrap();
        write(root.path(), ".gitignore", &format!("{IGNORED}\n"));
        commit(&repository, &[".gitignore"]);
        let service = RepositoryService::open_at(data.path()).unwrap();
        service
            .enable(EnableRepositoryRequest {
                root: root.path().into(),
                primary_branch: "main".into(),
                identity: None,
                operation_id: OperationId::new(),
            })
            .unwrap();
        let tickets = [TICKET, OTHER].map(ticket_path);
        for (id, path) in [TICKET, OTHER].iter().zip(&tickets) {
            write(root.path(), path, &ticket_source(id, ""));
        }
        write(root.path(), DOCUMENT_PATH, &document_source(DOCUMENT));
        // Enabling leaves its configuration for the caller to commit.
        commit(
            &repository,
            &[
                canonical::CONFIG_PATH,
                &tickets[0],
                &tickets[1],
                DOCUMENT_PATH,
            ],
        );
        let fixture = Self {
            root,
            data,
            service,
        };
        fixture.refresh();
        fixture
    }

    fn root(&self) -> PathBuf {
        self.root.path().to_owned()
    }

    fn repository(&self) -> Repository {
        Repository::open(self.root.path()).unwrap()
    }

    fn db(&self) -> Connection {
        Connection::open(self.data.path().join(REGISTRY_FILE)).unwrap()
    }

    fn refresh(&self) {
        let outcome = self
            .service
            .refresh_repository(RefreshRepositoryRequest {
                root: self.root(),
                operation_id: OperationId::new(),
            })
            .unwrap();
        assert!(matches!(outcome, RefreshOutcome::Refreshed { .. }));
    }

    fn primary(&self) -> Oid {
        self.repository().refname_to_id("refs/heads/main").unwrap()
    }

    /// Commits `paths` on primary, as they are on disk.
    fn commit_on_primary(&self, paths: &[&str]) -> Oid {
        commit(&self.repository(), paths)
    }

    fn worktree(&self, id: &str) -> PathBuf {
        self.root.path().join(".manyhands/worktrees").join(id)
    }

    fn branch(kind: AuthoringKind, id: &str) -> String {
        format!("manyhands/{}/{id}", authoring_kind_segment(&kind))
    }

    fn state(&self, kind: AuthoringKind, id: &str) -> ContextState {
        let branch = Self::branch(kind, id);
        let indexed: i64 = self
            .db()
            .query_row(
                "SELECT COUNT(*) FROM contexts WHERE kind = 'active' AND branch = ?1",
                [&branch],
                |row| row.get(0),
            )
            .unwrap();
        (
            self.worktree(id).exists(),
            self.repository()
                .refname_to_id(&format!("refs/heads/{branch}"))
                .ok(),
            indexed == 1,
        )
    }

    fn prepare(&self, kind: AuthoringKind, id: &str, intent: ContextIntent) -> ItemContext {
        match self
            .service
            .prepare_context(AuthoringTarget {
                root: self.root(),
                kind,
                item_id: id.parse().unwrap(),
                intent,
                operation_id: OperationId::new(),
            })
            .unwrap()
        {
            ContextProvisionOutcome::Created(context) => context,
            ContextProvisionOutcome::Reused(_) | ContextProvisionOutcome::IndexPending { .. } => {
                panic!("the context was not made")
            }
        }
    }

    fn edit(&self, id: &str) -> ItemContext {
        self.prepare(AuthoringKind::Ticket, id, ContextIntent::Edit)
    }

    fn release_with(
        &self,
        service: &RepositoryService,
        kind: AuthoringKind,
        id: &str,
        operation_id: OperationId,
    ) -> Result<ContextReleaseOutcome, RepositoryError> {
        service.release_context(ReleaseContextRequest {
            root: self.root(),
            kind,
            item_id: id.parse().unwrap(),
            operation_id,
        })
    }

    fn release(&self, id: &str) -> Result<ContextReleaseOutcome, RepositoryError> {
        self.release_with(&self.service, AuthoringKind::Ticket, id, OperationId::new())
    }

    fn failing(&self, point: FailurePoint) -> RepositoryService {
        RepositoryService::open_at_with_failure_point_for_testing(self.data.path(), point).unwrap()
    }

    /// The state and step of the operation's row.
    fn row(&self, operation_id: OperationId) -> (String, Option<String>) {
        self.db()
            .query_row(
                "SELECT state, completed_step FROM operation_records WHERE operation_ulid = ?1",
                [operation_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap()
    }

    /// Every reference and where it points.
    fn references(&self) -> Vec<(String, Option<Oid>)> {
        let repository = self.repository();
        let mut references: Vec<_> = repository
            .references()
            .unwrap()
            .map(|reference| {
                let reference = reference.unwrap();
                (reference.name().unwrap().to_owned(), reference.target())
            })
            .collect();
        references.sort();
        references
    }

    /// Adds the remote `origin` and selects it for publication.
    fn select_origin(&self) -> RemoteRefPlan {
        self.service
            .add_remote(AddRemoteRequest {
                root: self.root(),
                name: "origin".into(),
                url: "ssh://fixture@localhost/repository".into(),
                operation_id: OperationId::new(),
            })
            .unwrap();
        self.select(Some("origin"));
        RemoteRefPlan::from_configuration("origin", "main").unwrap()
    }

    fn select(&self, name: Option<&str>) {
        self.service
            .set_publication_remote(SetPublicationRemoteRequest {
                root: self.root(),
                name: name.map(str::to_owned),
                operation_id: OperationId::new(),
            })
            .unwrap();
    }

    fn evidence(&self, id: &str) -> RemotePublicationEvidence {
        self.service
            .remote_snapshot(self.root.path())
            .unwrap()
            .publication_evidence_for(AuthoringKind::Ticket, &id.parse().unwrap())
    }
}

fn kind_of<T: std::fmt::Debug>(result: Result<T, RepositoryError>) -> RepositoryErrorKind {
    match result {
        Ok(value) => panic!("not refused: {value:?}"),
        Err(error) => error.kind,
    }
}

/// Asserts that a release of `id` is refused with `kind`, that the refusal
/// left the context exactly as it was, and that its row was discarded.
fn assert_refused_and_left(f: &Fixture, id: &str, kind: RepositoryErrorKind) {
    let before = f.state(AuthoringKind::Ticket, id);
    let references = f.references();
    let operation_id = OperationId::new();
    let result = f.release_with(&f.service, AuthoringKind::Ticket, id, operation_id);
    assert_eq!(kind_of(result), kind);
    assert_eq!(f.state(AuthoringKind::Ticket, id), before);
    assert_eq!(f.references(), references);
    assert_eq!(
        f.row(operation_id),
        ("completed".to_owned(), Some("rejected".to_owned())),
        "the row is discarded"
    );
}

/// The worktree, the branch and the index row of a context that is
/// there, at primary's head.
fn present(f: &Fixture) -> ContextState {
    (true, Some(f.primary()), true)
}

const GONE: ContextState = (false, None, false);

#[test]
fn a_context_with_no_saved_work_is_released_and_the_index_lets_it_go() {
    let f = Fixture::new();
    f.edit(TICKET);
    f.edit(OTHER);
    let primary = f.primary();
    assert_eq!(f.state(AuthoringKind::Ticket, TICKET), present(&f));

    let operation_id = OperationId::new();
    let released = f.release_with(&f.service, AuthoringKind::Ticket, TICKET, operation_id);

    assert_eq!(released.unwrap(), ContextReleaseOutcome::Released);
    assert_eq!(f.state(AuthoringKind::Ticket, TICKET), GONE);
    // The other context and primary are as they were.
    assert_eq!(f.state(AuthoringKind::Ticket, OTHER), present(&f));
    assert_eq!(f.primary(), primary);
    assert_eq!(
        f.row(operation_id),
        (
            "completed".to_owned(),
            Some("context_branch_released".to_owned())
        )
    );
}

#[test]
fn a_create_context_with_no_item_is_released() {
    let f = Fixture::new();
    f.prepare(AuthoringKind::Ticket, NEW, ContextIntent::Create);
    f.prepare(AuthoringKind::Document, NEW_DOCUMENT, ContextIntent::Create);
    // A context that holds no item yet is no active context of the
    // index's: discovery lists it among the problems.
    assert_eq!(
        f.state(AuthoringKind::Ticket, NEW),
        (true, Some(f.primary()), false)
    );

    assert_eq!(f.release(NEW).unwrap(), ContextReleaseOutcome::Released);
    assert_eq!(f.state(AuthoringKind::Ticket, NEW), GONE);
    // A document's context is released the same way.
    let released = f.release_with(
        &f.service,
        AuthoringKind::Document,
        NEW_DOCUMENT,
        OperationId::new(),
    );
    assert_eq!(released.unwrap(), ContextReleaseOutcome::Released);
    assert_eq!(f.state(AuthoringKind::Document, NEW_DOCUMENT), GONE);
}

#[test]
fn a_branch_left_without_its_worktree_is_released() {
    let f = Fixture::new();
    let repository = f.repository();
    let head = repository.find_commit(f.primary()).unwrap();
    repository
        .branch(
            &Fixture::branch(AuthoringKind::Ticket, TICKET),
            &head,
            false,
        )
        .unwrap();
    assert_eq!(
        f.state(AuthoringKind::Ticket, TICKET),
        (false, Some(head.id()), false)
    );

    assert_eq!(f.release(TICKET).unwrap(), ContextReleaseOutcome::Released);
    assert_eq!(f.state(AuthoringKind::Ticket, TICKET), GONE);
}

#[test]
fn after_a_release_an_edit_makes_the_context_again() {
    let f = Fixture::new();
    f.edit(TICKET);
    assert_eq!(f.release(TICKET).unwrap(), ContextReleaseOutcome::Released);

    // `prepare` asserts that the context is made, not found.
    f.edit(TICKET);
    assert_eq!(f.state(AuthoringKind::Ticket, TICKET), present(&f));
}

#[test]
fn with_neither_the_worktree_nor_the_branch_nothing_is_touched_and_the_row_is_discarded() {
    let f = Fixture::new();
    let references = f.references();
    let operation_id = OperationId::new();

    let result = f.release_with(&f.service, AuthoringKind::Ticket, TICKET, operation_id);

    assert_eq!(result.unwrap(), ContextReleaseOutcome::AlreadyAbsent);
    assert_eq!(f.references(), references);
    assert_eq!(
        f.row(operation_id),
        ("completed".to_owned(), Some("rejected".to_owned()))
    );
}

/// Something done to a worktree.
type Change = fn(&Path);

#[test]
fn check_a_a_worktree_with_a_change_or_an_untracked_or_ignored_file_is_not_clean() {
    let changes: [(&str, Change); 3] = [
        ("untracked", |worktree| {
            write(worktree, "notes.txt", "draft\n")
        }),
        ("modified", |worktree| {
            write(worktree, &ticket_path(TICKET), "edited, not committed\n");
        }),
        ("ignored", |worktree| write(worktree, IGNORED, "scratch\n")),
    ];
    for (name, change) in changes {
        let f = Fixture::new();
        f.edit(TICKET);
        change(&f.worktree(TICKET));
        eprintln!("{name}");
        assert_refused_and_left(&f, TICKET, RepositoryErrorKind::DirtyWorktree);
    }
}

#[test]
fn check_a_a_worktree_at_another_head_or_locked_is_recovery_required() {
    // Detached from its branch.
    let f = Fixture::new();
    f.edit(TICKET);
    let linked = Repository::open(f.worktree(TICKET)).unwrap();
    linked.set_head_detached(f.primary()).unwrap();
    assert_refused_and_left(&f, TICKET, RepositoryErrorKind::RecoveryRequired);

    // Locked.
    let f = Fixture::new();
    f.edit(TICKET);
    f.repository()
        .find_worktree(TICKET)
        .unwrap()
        .lock(Some("held"))
        .unwrap();
    assert_refused_and_left(&f, TICKET, RepositoryErrorKind::RecoveryRequired);
}

#[test]
fn check_b_a_commit_on_the_branch_is_an_external_change() {
    let f = Fixture::new();
    let context = f.edit(TICKET);
    let linked = Repository::open(&context.worktree).unwrap();
    write(&context.worktree, "notes.txt", "committed in the context\n");
    commit(&linked, &["notes.txt"]);
    assert_ne!(f.state(AuthoringKind::Ticket, TICKET).1, Some(f.primary()));

    assert_refused_and_left(&f, TICKET, RepositoryErrorKind::ExternalChange);
}

#[test]
fn check_b_a_branch_behind_primary_is_released() {
    let f = Fixture::new();
    f.edit(TICKET);
    write(f.root.path(), "later.txt", "primary moved on\n");
    f.commit_on_primary(&["later.txt"]);
    assert_ne!(f.state(AuthoringKind::Ticket, TICKET).1, Some(f.primary()));

    assert_eq!(f.release(TICKET).unwrap(), ContextReleaseOutcome::Released);
    assert_eq!(f.state(AuthoringKind::Ticket, TICKET), GONE);
}

#[test]
fn check_b_a_root_with_another_branch_checked_out_is_recovery_required() {
    let f = Fixture::new();
    f.edit(TICKET);
    let repository = f.repository();
    let head = repository.find_commit(f.primary()).unwrap();
    repository.branch("elsewhere", &head, false).unwrap();
    repository.set_head("refs/heads/elsewhere").unwrap();

    assert_refused_and_left(&f, TICKET, RepositoryErrorKind::RecoveryRequired);
}

#[test]
fn check_c_a_tracking_ref_of_the_context_is_an_external_change() {
    for remote in ["origin", "mirror/deep"] {
        let f = Fixture::new();
        f.edit(TICKET);
        f.repository()
            .reference(
                &format!("refs/remotes/{remote}/manyhands/ticket/{TICKET}"),
                f.primary(),
                false,
                "fixture",
            )
            .unwrap();
        assert_refused_and_left(&f, TICKET, RepositoryErrorKind::ExternalChange);
    }
}

#[test]
fn check_c_a_verified_push_of_the_context_is_an_external_change() {
    let f = Fixture::new();
    let plan = f.select_origin();
    f.edit(TICKET);
    let target = RemoteOperationTarget::for_context(
        &plan,
        RemoteOperationAction::SynchronizeContext,
        AuthoringKind::Ticket,
        TICKET.parse().unwrap(),
    )
    .unwrap();
    let operation_id = OperationId::new();
    assert!(matches!(
        f.service
            .reserve_remote_operation(f.root.path(), operation_id, &target)
            .unwrap(),
        RemoteReservationOutcome::Reserved(_)
    ));
    // The push journal of a publication of the context's tip, as a
    // synchronization that completed records it.
    f.db()
        .execute(
            "UPDATE remote_operation_records SET phase='completed',outcome='completed',
                sync_checkpoint='discovery_pending',completed_step='before_discovery',
                expected_oid=?2,local_oid=?2,primary_tracking_oid=?2,push_oid=?2,
                push_advertised_oid=?2,authoritative_kind='published',authoritative_oid=?2,
                index_pending=1
          WHERE operation_ulid=?1",
            params![operation_id.to_string(), f.primary().to_string()],
        )
        .unwrap();
    assert_eq!(
        f.evidence(TICKET),
        RemotePublicationEvidence::ObservedPublished
    );

    assert_refused_and_left(&f, TICKET, RepositoryErrorKind::ExternalChange);
}

/// R-43: every re-selection of the remote leaves the publication history
/// unknown for good. With no tracking ref and no commit of its own, the
/// context is released.
#[test]
fn check_c_an_unknown_publication_history_with_no_tracking_ref_is_released() {
    let f = Fixture::new();
    f.select_origin();
    f.select(None);
    f.edit(TICKET);
    assert_eq!(
        f.evidence(TICKET),
        RemotePublicationEvidence::HistoryUnknown
    );

    assert_eq!(f.release(TICKET).unwrap(), ContextReleaseOutcome::Released);
    assert_eq!(f.state(AuthoringKind::Ticket, TICKET), GONE);
}

#[test]
fn check_d_a_pending_synchronization_of_the_item_is_recovery_required() {
    let f = Fixture::new();
    f.select_origin();
    f.edit(TICKET);
    let generation: i64 = f
        .db()
        .query_row(
            "SELECT configuration_generation FROM remote_polling_state",
            [],
            |row| row.get(0),
        )
        .unwrap();
    // A synchronization of the context that stopped with its merge
    // conflict pending.
    let db = f.db();
    db.execute(
        "INSERT INTO remote_operation_records(repository_id,operation_ulid,configuration_generation,
                remote_name,primary_branch,primary_ref,primary_tracking_ref,action,priority,phase,
                created_at,updated_at,context_ref,context_tracking_ref,kind,item_id,local_branch)
         SELECT id,?1,?2,'origin','main','refs/heads/main','refs/remotes/origin/main',
                'synchronize_context','manual','interrupted',1,1,?3,?4,'ticket',?5,?6
           FROM repositories",
        params![
            OperationId::new().to_string(),
            generation,
            format!("refs/heads/manyhands/ticket/{TICKET}"),
            format!("refs/remotes/origin/manyhands/ticket/{TICKET}"),
            TICKET,
            format!("refs/heads/manyhands/ticket/{TICKET}"),
        ],
    )
    .unwrap();
    let operation = db.last_insert_rowid();
    db.execute(
        "INSERT INTO remote_integration_windows(operation_record_id,number,configuration_generation,
                owner_epoch,kind) VALUES(?1,0,?2,0,'legacy')",
        params![operation, generation],
    )
    .unwrap();
    let tree = f
        .repository()
        .find_commit(f.primary())
        .unwrap()
        .tree_id()
        .to_string();
    db.execute(
        "INSERT INTO remote_integration_steps(operation_record_id,configuration_generation,
                owner_epoch,ordinal,stage,local_oid,incoming_oid,baseline_tree_oid,
                baseline_index_digest,conflict_digest,phase)
         VALUES(?1,?2,0,0,'context',?3,?3,?4,zeroblob(32),zeroblob(32),'conflict_pending')",
        params![operation, generation, f.primary().to_string(), tree],
    )
    .unwrap();
    // The remote state reads, so (c) passes: the refusal is (d)'s.
    assert_eq!(
        f.evidence(TICKET),
        RemotePublicationEvidence::NeverPublished
    );

    assert_refused_and_left(&f, TICKET, RepositoryErrorKind::RecoveryRequired);
}

#[test]
fn ownership_an_unfinished_closure_or_promotion_owns_the_context() {
    // A closure of the ticket.
    let f = Fixture::new();
    f.edit(TICKET);
    state::with_transaction(&f.service, f.root.path(), |tx, id| {
        closure::begin_record(
            tx,
            id,
            OperationId::new(),
            &TICKET.parse().unwrap(),
            ClosureExpectation::from_bytes([7; 32]),
            None,
            ClosureKind::Record,
        )
        .map(|_| ())
    })
    .unwrap();
    assert_refused_and_left(&f, TICKET, RepositoryErrorKind::RecoveryRequired);

    // A promotion of the document.
    let f = Fixture::new();
    f.prepare(AuthoringKind::Document, DOCUMENT, ContextIntent::Edit);
    state::with_transaction(&f.service, f.root.path(), |tx, id| {
        promotion::begin_record(
            tx,
            id,
            OperationId::new(),
            &DOCUMENT.parse().unwrap(),
            PromotionExpectation::from_bytes([7; 32]),
            None,
        )
        .map(|_| ())
    })
    .unwrap();
    let before = f.state(AuthoringKind::Document, DOCUMENT);
    let result = f.release_with(
        &f.service,
        AuthoringKind::Document,
        DOCUMENT,
        OperationId::new(),
    );
    assert_eq!(kind_of(result), RepositoryErrorKind::RecoveryRequired);
    assert_eq!(f.state(AuthoringKind::Document, DOCUMENT), before);
    assert_eq!(before, present(&f));
}

#[test]
fn ownership_a_closed_ticket_is_ticket_closed() {
    let f = Fixture::new();
    f.edit(TICKET);
    write(
        f.root.path(),
        &ticket_path(TICKET),
        &ticket_source(TICKET, CLOSURE),
    );
    f.commit_on_primary(&[&ticket_path(TICKET)]);

    assert_refused_and_left(&f, TICKET, RepositoryErrorKind::TicketClosed);
}

#[test]
fn another_pending_row_on_the_root_is_recovery_required() {
    let f = Fixture::new();
    f.edit(TICKET);
    // A preparation of another item, stopped before its worktree.
    let stopped = f
        .failing(FailurePoint::BeforeWorktreeCreation)
        .prepare_context(AuthoringTarget {
            root: f.root(),
            kind: AuthoringKind::Ticket,
            item_id: OTHER.parse().unwrap(),
            intent: ContextIntent::Edit,
            operation_id: OperationId::new(),
        });
    assert_eq!(
        kind_of(stopped.map(|_| ())),
        RepositoryErrorKind::InjectedFailure
    );
    let before = f.state(AuthoringKind::Ticket, TICKET);

    assert_eq!(
        kind_of(f.release(TICKET)),
        RepositoryErrorKind::RecoveryRequired
    );
    assert_eq!(f.state(AuthoringKind::Ticket, TICKET), before);
}

// Re-entry.

#[test]
fn interrupted_before_the_branch_release_the_same_operation_deletes_the_branch() {
    let f = Fixture::new();
    f.edit(TICKET);
    let operation_id = OperationId::new();
    let failing = f.failing(FailurePoint::BeforeContextBranchRelease);

    let stopped = f.release_with(&failing, AuthoringKind::Ticket, TICKET, operation_id);

    assert_eq!(kind_of(stopped), RepositoryErrorKind::InjectedFailure);
    let (worktree, branch, _) = f.state(AuthoringKind::Ticket, TICKET);
    assert_eq!((worktree, branch), (false, Some(f.primary())));
    assert_eq!(
        f.row(operation_id),
        (
            "context_worktree_released".to_owned(),
            Some("context_worktree_released".to_owned())
        )
    );
    // Every other operation waits for the row.
    assert_eq!(
        kind_of(f.release(OTHER)),
        RepositoryErrorKind::RecoveryRequired
    );

    let finished = f.release_with(&f.service, AuthoringKind::Ticket, TICKET, operation_id);

    assert_eq!(finished.unwrap(), ContextReleaseOutcome::Released);
    assert_eq!(f.state(AuthoringKind::Ticket, TICKET), GONE);
    assert_eq!(
        f.row(operation_id),
        (
            "completed".to_owned(),
            Some("context_branch_released".to_owned())
        )
    );
}

#[test]
fn a_tip_that_moved_after_the_worktree_was_released_is_an_external_change_and_the_row_stays() {
    let f = Fixture::new();
    f.edit(TICKET);
    let operation_id = OperationId::new();
    let failing = f.failing(FailurePoint::BeforeContextBranchRelease);
    let stopped = f.release_with(&failing, AuthoringKind::Ticket, TICKET, operation_id);
    assert_eq!(kind_of(stopped), RepositoryErrorKind::InjectedFailure);
    // Someone commits on the branch, with its worktree gone.
    let repository = f.repository();
    let parent = repository.find_commit(f.primary()).unwrap();
    let signature = git2::Signature::now("Someone", "someone@example.invalid").unwrap();
    let tip = repository
        .commit(
            Some(&format!(
                "refs/heads/{}",
                Fixture::branch(AuthoringKind::Ticket, TICKET)
            )),
            &signature,
            &signature,
            "elsewhere",
            &parent.tree().unwrap(),
            &[&parent],
        )
        .unwrap();

    let refused = f.release_with(&f.service, AuthoringKind::Ticket, TICKET, operation_id);

    assert_eq!(kind_of(refused), RepositoryErrorKind::ExternalChange);
    assert_eq!(f.state(AuthoringKind::Ticket, TICKET).1, Some(tip));
    assert_eq!(
        f.row(operation_id).0,
        "context_worktree_released",
        "the row stays pending"
    );
}

#[test]
fn interrupted_after_the_branch_release_the_same_operation_hands_off() {
    let f = Fixture::new();
    f.edit(TICKET);
    let operation_id = OperationId::new();
    let failing = f.failing(FailurePoint::AfterContextBranchRelease);

    let stopped = f.release_with(&failing, AuthoringKind::Ticket, TICKET, operation_id);

    assert_eq!(kind_of(stopped), RepositoryErrorKind::InjectedFailure);
    assert_eq!(f.state(AuthoringKind::Ticket, TICKET), (false, None, true));
    assert_eq!(
        f.row(operation_id),
        (
            "context_branch_released".to_owned(),
            Some("context_branch_released".to_owned())
        )
    );

    let finished = f.release_with(&f.service, AuthoringKind::Ticket, TICKET, operation_id);

    assert_eq!(finished.unwrap(), ContextReleaseOutcome::Released);
    assert_eq!(f.state(AuthoringKind::Ticket, TICKET), GONE);
    assert_eq!(f.row(operation_id).0, "completed");
}

/// The hand-off is owed to `operation resume`, which runs a refresh under
/// the row's ID once the row owes only its hand-off.
#[test]
fn a_failed_hand_off_is_index_pending_and_a_refresh_under_its_id_finishes_it() {
    let f = Fixture::new();
    f.edit(TICKET);
    let operation_id = OperationId::new();
    let failing = f.failing(FailurePoint::BeforeIndexTransactionCommit);

    let pending = f.release_with(&failing, AuthoringKind::Ticket, TICKET, operation_id);

    assert_eq!(pending.unwrap(), ContextReleaseOutcome::IndexPending);
    assert_eq!(f.state(AuthoringKind::Ticket, TICKET), (false, None, true));
    let root = f.root.path().to_str().unwrap().to_owned();
    let row = recovery::local_row(&f.db(), &root, operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(row.step.as_deref(), Some("context_branch_released"));
    assert!(row.owes_only_its_hand_off(), "state {}", row.state);
    assert_eq!(
        kind_of(f.release(OTHER)),
        RepositoryErrorKind::RecoveryRequired
    );

    let refreshed = f
        .service
        .refresh_repository(RefreshRepositoryRequest {
            root: f.root(),
            operation_id,
        })
        .unwrap();

    assert!(matches!(refreshed, RefreshOutcome::Refreshed { .. }));
    assert_eq!(f.state(AuthoringKind::Ticket, TICKET), GONE);
    assert_eq!(f.row(operation_id).0, "completed");
}

#[test]
fn a_completed_release_is_answered_again_and_nothing_is_looked_at() {
    let f = Fixture::new();
    f.edit(TICKET);
    let operation_id = OperationId::new();
    let released = f.release_with(&f.service, AuthoringKind::Ticket, TICKET, operation_id);
    assert_eq!(released.unwrap(), ContextReleaseOutcome::Released);
    // A context made since, by another operation, is not this one's.
    f.edit(TICKET);

    let again = f.release_with(&f.service, AuthoringKind::Ticket, TICKET, operation_id);

    assert_eq!(again.unwrap(), ContextReleaseOutcome::Released);
    assert_eq!(f.state(AuthoringKind::Ticket, TICKET), present(&f));
    assert_eq!(
        f.row(operation_id),
        (
            "completed".to_owned(),
            Some("context_branch_released".to_owned())
        )
    );
}

#[test]
fn neither_release_step_is_a_checkpoint_and_the_branch_step_owes_only_its_hand_off() {
    // A release commits nothing.
    assert!(!LocalStep::ContextWorktreeReleased.is_checkpoint());
    assert!(!LocalStep::ContextBranchReleased.is_checkpoint());
    let row = |step: &str| recovery::LocalRow {
        id: 1,
        action: "release_context".to_owned(),
        target: None,
        state: "failed".to_owned(),
        step: Some(step.to_owned()),
        observed_at: 0,
    };
    // With its worktree released, a release still has its branch to
    // delete: a refresh would complete a row that owes more.
    assert!(!row("context_worktree_released").owes_only_its_hand_off());
    assert!(row("context_branch_released").owes_only_its_hand_off());
    assert_eq!(
        row("context_branch_released").operation(),
        Some(RepositoryOperation::ReleaseContext)
    );
}
