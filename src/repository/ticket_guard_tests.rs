//! The closed-ticket guard (C5) through the domain operations that call it:
//! a new row's edit of a closed ticket is refused before anything is
//! provisioned, and the exempt writes are not.
use super::*;
use crate::{
    canonical::ItemId,
    repository::{
        AuthoringKind, AuthoringTarget, ContextIntent, ContextProvisionOutcome, DocumentDraft,
        EnableRepositoryRequest, ExpectedPathObservation, FailurePoint, LocalCheckpoint,
        OperationId, REGISTRY_FILE, ReadError, RelationshipWrite, RepairItemRequest,
        RepositoryError, RepositoryErrorKind, RepositoryOperation, RepositoryService,
        SaveDocumentRequest, SaveOutcome, SaveTicketRequest, SubmitCommentRequest, TicketDraft,
        TicketWriteOptions,
    },
    results::ResultCode,
};
use git2::{Oid, Repository};
use std::{fs, path::PathBuf};

const ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const OTHER: &str = "01BX5ZZKBKACTAV9WEVGEMMVRZ";
const COMMENT: &str = "01BX5ZZKBKACTAV9WEVGEMMVS0";
const CLOSURE: &str =
    "closed_at: 2026-10-02T12:34:56Z\nclosed_by: Fixture <fixture@example.invalid>\n";

fn id(text: &str) -> ItemId {
    text.parse().unwrap()
}
fn ticket_path(id: &str) -> String {
    format!(".manyhands/tickets/{id}/ticket.md")
}
/// A conforming ticket; `extra` is front matter lines.
fn source(id: &str, extra: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {id}\ntitle: Ticket\n\
         type: task\nstatus: open\n{extra}---\nOriginal body.\n"
    )
}
fn commit(repo: &Repository) -> Oid {
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.update_all(["*"], None).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "fixture",
        &tree,
        &parent.iter().collect::<Vec<_>>(),
    )
    .unwrap()
}
fn write(repo: &Repository, path: &str, bytes: impl AsRef<[u8]>) {
    let path = repo.workdir().unwrap().join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}
fn read(repo: &Repository, path: &str) -> Vec<u8> {
    fs::read(repo.workdir().unwrap().join(path)).unwrap()
}
fn kind<T>(result: Result<T, RepositoryError>) -> RepositoryErrorKind {
    match result {
        Ok(_) => panic!("the write was made"),
        Err(error) => error.kind,
    }
}
fn checkpointed(outcome: SaveOutcome) -> Oid {
    match outcome {
        SaveOutcome::Saved {
            checkpoint: LocalCheckpoint::Checkpointed { commit_oid },
            ..
        } => commit_oid,
        _ => panic!("the save made no checkpoint"),
    }
}

/// An enabled repository whose primary holds ticket `ID` with `extra`
/// front matter, and no context.
struct Fixture {
    root: tempfile::TempDir,
    data: tempfile::TempDir,
    service: RepositoryService,
    repo: Repository,
}
impl Fixture {
    fn new(extra: &str) -> Self {
        Self::with_ticket(&source(ID, extra))
    }
    fn with_ticket(ticket: &str) -> Self {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = tempfile::tempdir().unwrap();
        let repo = Repository::init(root.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_bool("core.autocrlf", false).unwrap();
        config.set_str("user.name", "Fixture").unwrap();
        config
            .set_str("user.email", "fixture@example.invalid")
            .unwrap();
        repo.set_head("refs/heads/main").unwrap();
        write(&repo, "ordinary.txt", "original\n");
        commit(&repo);
        let service = RepositoryService::open_at(data.path()).unwrap();
        service
            .enable(EnableRepositoryRequest {
                root: root.path().into(),
                primary_branch: "main".into(),
                identity: None,
                operation_id: OperationId::new(),
            })
            .unwrap();
        commit(&repo);
        write(&repo, &ticket_path(ID), ticket);
        commit(&repo);
        Self {
            root,
            data,
            service,
            repo,
        }
    }
    fn target(
        &self,
        item: &str,
        intent: ContextIntent,
        operation_id: OperationId,
    ) -> AuthoringTarget {
        AuthoringTarget {
            root: self.root.path().into(),
            kind: AuthoringKind::Ticket,
            item_id: id(item),
            intent,
            operation_id,
        }
    }
    fn worktree(&self, item: &str) -> PathBuf {
        self.root.path().join(".manyhands/worktrees").join(item)
    }
    /// A save of `item` that keeps the ticket open and changes its body.
    fn request(
        &self,
        item: &str,
        intent: ContextIntent,
        operation_id: OperationId,
    ) -> SaveTicketRequest {
        let expected_path = match intent {
            ContextIntent::Create => ExpectedPathObservation::Missing,
            _ => {
                let worktree = self.worktree(item).join(ticket_path(item));
                let bytes =
                    fs::read(&worktree).unwrap_or_else(|_| read(&self.repo, &ticket_path(item)));
                ExpectedPathObservation::from_bytes(&bytes)
            }
        };
        SaveTicketRequest {
            target: self.target(item, intent, operation_id),
            draft: TicketDraft {
                title: "Ticket".into(),
                ticket_type: "task".into(),
                status: "open".into(),
                project: None,
                team: None,
                body: "Edited body.\n".into(),
            },
            expected_path,
        }
    }
    fn save(&self, intent: ContextIntent) -> Result<SaveOutcome, RepositoryError> {
        self.service
            .save_ticket(self.request(ID, intent, OperationId::new()))
    }
    fn prepare(
        &self,
        operation_id: OperationId,
    ) -> Result<ContextProvisionOutcome, RepositoryError> {
        self.service
            .prepare_context(self.target(ID, ContextIntent::Edit, operation_id))
    }
    /// The ticket's context, made while the ticket is open.
    fn context(&self) -> Repository {
        self.prepare(OperationId::new()).unwrap();
        Repository::open(self.worktree(ID)).unwrap()
    }
    fn close_on_primary(&self) {
        write(&self.repo, &ticket_path(ID), source(ID, CLOSURE));
        commit(&self.repo);
    }
    fn branches(&self) -> Vec<String> {
        let mut names = self
            .repo
            .branches(None)
            .unwrap()
            .map(|branch| {
                let (branch, _) = branch.unwrap();
                format!(
                    "{}={:?}",
                    branch.name().unwrap().unwrap(),
                    branch.get().target()
                )
            })
            .collect::<Vec<_>>();
        names.sort();
        names
    }
    fn row(&self, operation_id: OperationId) -> (String, Option<String>) {
        rusqlite::Connection::open(self.data.path().join(REGISTRY_FILE))
            .unwrap()
            .query_row(
                "SELECT state, completed_step FROM operation_records WHERE operation_ulid=?1",
                [operation_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap()
    }
    fn assert_primary_clean(&self) {
        let mut options = git2::StatusOptions::new();
        options.include_untracked(true).recurse_untracked_dirs(true);
        assert!(self.repo.statuses(Some(&mut options)).unwrap().is_empty());
    }
}

fn rejected() -> (String, Option<String>) {
    ("completed".into(), Some("rejected".into()))
}

#[test]
fn save_edit_of_closed_ticket_is_refused_before_provisioning() {
    let f = Fixture::new(CLOSURE);
    let branches = f.branches();
    let primary = read(&f.repo, &ticket_path(ID));
    let operation_id = OperationId::new();

    let error = f
        .service
        .save_ticket(f.request(ID, ContextIntent::Edit, operation_id))
        .err()
        .expect("the save was made");

    assert_eq!(error.kind, RepositoryErrorKind::TicketClosed);
    assert_eq!(error.operation, RepositoryOperation::SaveTicket);
    assert_eq!(f.branches(), branches, "no branch is made");
    assert!(!f.worktree(ID).exists(), "no worktree is made");
    assert_eq!(read(&f.repo, &ticket_path(ID)), primary);
    f.assert_primary_clean();
    assert_eq!(f.row(operation_id), rejected());
}

#[test]
fn save_refused_when_context_copy_closed() {
    let f = Fixture::new("");
    let context = f.context();
    write(&context, &ticket_path(ID), source(ID, CLOSURE));
    let closed = commit(&context);
    let branches = f.branches();
    let operation_id = OperationId::new();

    assert_eq!(
        kind(
            f.service
                .save_ticket(f.request(ID, ContextIntent::Edit, operation_id))
        ),
        RepositoryErrorKind::TicketClosed
    );
    assert_eq!(f.row(operation_id), rejected());
    assert_eq!(context.head().unwrap().target(), Some(closed));
    assert_eq!(f.branches(), branches);
    assert_eq!(
        read(&context, &ticket_path(ID)),
        source(ID, CLOSURE).into_bytes()
    );
    // The primary copy is open: the context copy alone refused.
    assert_eq!(read(&f.repo, &ticket_path(ID)), source(ID, "").into_bytes());
}

#[test]
fn relationship_and_slug_saves_refused() {
    let f = Fixture::new(CLOSURE);
    let branches = f.branches();
    for options in [
        TicketWriteOptions {
            deps: RelationshipWrite::Set(vec![id(OTHER)]),
            ..TicketWriteOptions::default()
        },
        TicketWriteOptions {
            parent: RelationshipWrite::Set(id(OTHER)),
            ..TicketWriteOptions::default()
        },
        TicketWriteOptions {
            slug: Some("ab-1".into()),
            ..TicketWriteOptions::default()
        },
    ] {
        let operation_id = OperationId::new();
        let result = f
            .service
            .save_ticket_with(f.request(ID, ContextIntent::Edit, operation_id), options);
        assert_eq!(kind(result), RepositoryErrorKind::TicketClosed);
        assert_eq!(f.row(operation_id), rejected());
    }
    assert_eq!(f.branches(), branches);
    assert!(!f.worktree(ID).exists());
}

#[test]
fn prepare_context_edit_of_closed_ticket_refused() {
    let f = Fixture::new(CLOSURE);
    let branches = f.branches();
    let operation_id = OperationId::new();

    let error = f.prepare(operation_id).err().expect("a context was made");

    assert_eq!(error.kind, RepositoryErrorKind::TicketClosed);
    assert_eq!(error.operation, RepositoryOperation::PrepareContext);
    assert_eq!(f.branches(), branches);
    assert!(!f.worktree(ID).exists());
    f.assert_primary_clean();
    assert_eq!(f.row(operation_id), rejected());
}

#[test]
fn new_comment_on_closed_ticket_refused() {
    let f = Fixture::new(CLOSURE);
    let branches = f.branches();
    let operation_id = OperationId::new();

    let result = f.service.checkpoint_comment_locally(
        SubmitCommentRequest {
            target: f.target(ID, ContextIntent::Edit, operation_id),
            comment_id: id(COMMENT),
            parent_id: None,
            body: "A comment.\n".into(),
            expected_destination: ExpectedPathObservation::Missing,
        },
        None,
    );

    let error = result.err().expect("the comment was checkpointed");
    assert_eq!(error.kind, RepositoryErrorKind::TicketClosed);
    assert_eq!(error.operation, RepositoryOperation::SubmitComment);
    assert_eq!(f.branches(), branches);
    assert!(!f.worktree(ID).exists());
    assert!(!f.root.path().join(".manyhands/comments").exists());
    assert!(
        f.service
            .read_comment_binding(f.root.path(), operation_id)
            .unwrap()
            .is_none()
    );
    assert_eq!(f.row(operation_id), rejected());
}

/// What the call sites' guard answers for a new row of `ID` with `intent`.
fn guard_for(f: &Fixture, intent: ContextIntent) -> Result<(), RepositoryError> {
    let record = super::super::recovery::RecoveryRecord {
        id: 0,
        is_new: true,
        is_pending: false,
        completed_step: None,
    };
    super::super::refuse_closed_ticket(
        &f.repo,
        f.root.path(),
        RepositoryOperation::SaveTicket,
        &f.target(ID, intent, OperationId::new()),
        record,
    )
}

#[test]
fn create_unaffected() {
    let f = Fixture::new(CLOSURE);
    // The guard asks only an edit: a creation of the closed ID is not
    // refused by it, while an edit is.
    guard_for(&f, ContextIntent::Create).unwrap();
    assert_eq!(
        guard_for(&f, ContextIntent::Edit).unwrap_err().kind,
        RepositoryErrorKind::TicketClosed
    );
    // A creation of the closed ID meets the creation rules, not the guard.
    assert_ne!(
        kind(
            f.service
                .save_ticket(f.request(ID, ContextIntent::Create, OperationId::new()))
        ),
        RepositoryErrorKind::TicketClosed
    );
    let created = f
        .service
        .save_ticket(f.request(OTHER, ContextIntent::Create, OperationId::new()))
        .unwrap();
    checkpointed(created);
    assert!(f.worktree(OTHER).join(ticket_path(OTHER)).is_file());
}

#[test]
fn repair_of_closed_ticket_still_allowed() {
    // A closed ticket with no type: nonconforming, so a repair's to correct.
    let broken = source(ID, CLOSURE).replace("type: task\n", "");
    let f = Fixture::with_ticket(&broken);
    let corrected = source(ID, CLOSURE);
    // The guard does not ask a repair of the closed ticket.
    guard_for(&f, ContextIntent::Repair).unwrap();

    let repaired = f
        .service
        .repair_item(RepairItemRequest {
            root: f.root.path().into(),
            kind: AuthoringKind::Ticket,
            path: PathBuf::from(ticket_path(ID)),
            source: corrected.clone(),
            expected: ExpectedPathObservation::from_bytes(broken.as_bytes()),
            operation_id: OperationId::new(),
        })
        .unwrap();

    checkpointed(repaired);
    let context = Repository::open(f.worktree(ID)).unwrap();
    assert_eq!(read(&context, &ticket_path(ID)), corrected.into_bytes());
}

#[test]
fn resumed_row_not_refused() {
    let f = Fixture::new("");
    let failing = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::BeforeCheckpointCommit,
    )
    .unwrap();
    let operation_id = OperationId::new();
    let request = f.request(ID, ContextIntent::Edit, operation_id);
    assert_eq!(
        kind(failing.save_ticket(request.clone())),
        RepositoryErrorKind::InjectedFailure
    );
    // Closed after the row began: the row predates the closure.
    f.close_on_primary();

    checkpointed(f.service.save_ticket(request).unwrap());
    assert_eq!(f.row(operation_id).0, "completed");
    // A new row is refused.
    assert_eq!(
        kind(f.save(ContextIntent::Edit)),
        RepositoryErrorKind::TicketClosed
    );
}

#[test]
fn writes_to_other_tickets_that_depend_on_closed_ticket_allowed() {
    let f = Fixture::new(CLOSURE);
    write(
        &f.repo,
        &ticket_path(OTHER),
        source(OTHER, &format!("deps:\n- {ID}\nparent: {ID}\n")),
    );
    commit(&f.repo);

    checkpointed(
        f.service
            .save_ticket(f.request(OTHER, ContextIntent::Edit, OperationId::new()))
            .unwrap(),
    );
    // A new dependency on the closed ticket is a write to the other one.
    let mut request = f.request(OTHER, ContextIntent::Edit, OperationId::new());
    request.draft.body = "Edited again.\n".into();
    let result = f.service.save_ticket_with(
        request,
        TicketWriteOptions {
            deps: RelationshipWrite::Set(vec![id(ID)]),
            parent: RelationshipWrite::Clear,
            ..TicketWriteOptions::default()
        },
    );
    checkpointed(result.unwrap());
}

#[test]
fn dirty_draft_cannot_hide_closure() {
    let f = Fixture::new("");
    let context = f.context();
    write(&context, &ticket_path(ID), source(ID, CLOSURE));
    commit(&context);
    // The worktree file drops the closure; the branch tip keeps it.
    let draft = source(ID, "").replace("Original body.", "Draft body.");
    write(&context, &ticket_path(ID), &draft);
    let operation_id = OperationId::new();

    assert_eq!(
        kind(
            f.service
                .save_ticket(f.request(ID, ContextIntent::Edit, operation_id))
        ),
        RepositoryErrorKind::TicketClosed
    );
    assert_eq!(f.row(operation_id), rejected());
    assert_eq!(read(&context, &ticket_path(ID)), draft.into_bytes());
}

#[test]
fn refused_error_maps_to_ticket_closed_result_code() {
    let f = Fixture::new(CLOSURE);
    let error = f
        .save(ContextIntent::Edit)
        .err()
        .expect("the save was made");
    assert_eq!(error.kind, RepositoryErrorKind::TicketClosed);
    assert_eq!(
        crate::repository::mutation::repository_error_code(error.kind),
        ResultCode::TicketClosed
    );
    assert_eq!(ReadError::from(error).code(), ResultCode::TicketClosed);
}

/// The guard alone: it reads the two branch tips and nothing else.
#[test]
fn guard_reads_branch_tips_only() {
    let f = Fixture::new("");
    let item = id(ID);
    require_ticket_open(&f.repo, "main", &item).unwrap();
    // A closed primary working tree is not the primary branch tip.
    write(&f.repo, &ticket_path(ID), source(ID, CLOSURE));
    require_ticket_open(&f.repo, "main", &item).unwrap();
    commit(&f.repo);
    assert_eq!(
        require_ticket_open(&f.repo, "main", &item)
            .unwrap_err()
            .kind,
        RepositoryErrorKind::TicketClosed
    );
    // A primary branch that does not exist holds no closed copy.
    require_ticket_open(&f.repo, "missing", &item).unwrap();
    // One closure key alone closes, as for the closure observation.
    let other = id(OTHER);
    write(
        &f.repo,
        &ticket_path(OTHER),
        source(OTHER, "closed_at: 2026-10-02T12:34:56Z\n"),
    );
    commit(&f.repo);
    assert_eq!(
        require_ticket_open(&f.repo, "main", &other)
            .unwrap_err()
            .kind,
        RepositoryErrorKind::TicketClosed
    );
}

/// A document save given a closed ticket's target is refused as before,
/// and now before any context is provisioned.
#[test]
fn save_document_with_ticket_target_provisions_nothing() {
    let f = Fixture::new(CLOSURE);
    let branches = f.branches();
    let operation_id = OperationId::new();

    let error = f
        .service
        .save_document(SaveDocumentRequest {
            target: f.target(ID, ContextIntent::Edit, operation_id),
            source_path: None,
            destination_path: PathBuf::from("docs/note.md"),
            draft: DocumentDraft {
                title: "Note".into(),
                body: "Body.\n".into(),
            },
            expected_source: None,
            expected_destination: ExpectedPathObservation::Missing,
        })
        .err()
        .expect("the save was made");

    assert_eq!(error.kind, RepositoryErrorKind::MissingAuthoringTarget);
    assert_eq!(error.operation, RepositoryOperation::SaveDocument);
    assert_eq!(f.branches(), branches, "no branch is made");
    assert!(!f.worktree(ID).exists(), "no worktree is made");
    f.assert_primary_clean();
    assert_eq!(f.row(operation_id), rejected());
}
