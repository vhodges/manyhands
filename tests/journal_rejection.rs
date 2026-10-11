use std::fs;

use manyhands::repository::{
    AddRemoteRequest, AuthoringKind, AuthoringTarget, CommitIdentity, ContextIntent,
    CreateRepositoryRequest, DocumentDraft, EnableRepositoryRequest, ExpectedPathObservation,
    RemoveRegistrationRequest, RemoveRemoteRequest, RepositoryError, RepositoryErrorKind,
    RepositoryService, SaveDocumentRequest, SaveTicketRequest, SetPublicationRemoteRequest,
    SubmitCommentRequest, TicketDraft,
};

mod support;

fn rejection<T>(result: Result<T, RepositoryError>) -> RepositoryErrorKind {
    match result {
        Ok(_) => panic!("the operation was expected to be rejected"),
        Err(error) => error.kind,
    }
}

fn outcome<T>(result: Result<T, RepositoryError>) -> Option<RepositoryErrorKind> {
    result.err().map(|error| error.kind)
}

fn probe_remote(
    service: &RepositoryService,
    root: &std::path::Path,
) -> Option<RepositoryErrorKind> {
    outcome(service.add_remote(AddRemoteRequest {
        root: root.to_owned(),
        name: "probe".to_owned(),
        url: "https://example.invalid/probe.git".to_owned(),
        operation_id: support::new_operation_id(),
    }))
}

fn remove_registration(
    service: &RepositoryService,
    root: &std::path::Path,
) -> Option<RepositoryErrorKind> {
    outcome(service.remove_registration(RemoveRegistrationRequest {
        root: root.to_owned(),
        operation_id: support::new_operation_id(),
    }))
}

fn document_request(root: &std::path::Path) -> SaveDocumentRequest {
    SaveDocumentRequest {
        target: AuthoringTarget {
            root: root.to_owned(),
            kind: AuthoringKind::Document,
            item_id: support::document_id(),
            intent: ContextIntent::Create,
            operation_id: support::new_operation_id(),
        },
        source_path: None,
        destination_path: "docs/new.md".into(),
        draft: DocumentDraft {
            title: "Title".to_owned(),
            body: "Body\n".to_owned(),
        },
        expected_source: None,
        expected_destination: ExpectedPathObservation::Missing,
    }
}

#[test]
fn a_rejected_remote_addition_does_not_block_the_repository() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);

    let kind = rejection(enabled.service.add_remote(AddRemoteRequest {
        root: fixture.root.clone(),
        name: "not a name".to_owned(),
        url: "https://example.invalid/origin.git".to_owned(),
        operation_id: support::new_operation_id(),
    }));
    assert_eq!(kind, RepositoryErrorKind::Git);

    assert_eq!(probe_remote(&enabled.service, &fixture.root), None);
    assert_eq!(remove_registration(&enabled.service, &fixture.root), None);
}

#[test]
fn a_rejected_remote_removal_does_not_block_the_repository() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);

    let kind = rejection(enabled.service.remove_remote(RemoveRemoteRequest {
        root: fixture.root.clone(),
        name: "not a name".to_owned(),
        operation_id: support::new_operation_id(),
    }));
    assert_eq!(kind, RepositoryErrorKind::Git);

    assert_eq!(probe_remote(&enabled.service, &fixture.root), None);
    assert_eq!(remove_registration(&enabled.service, &fixture.root), None);
}

#[test]
fn a_rejected_remote_selection_does_not_block_the_repository() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let head = fixture.repository.head().unwrap().target().unwrap();
    fixture.repository.set_head_detached(head).unwrap();

    let kind = rejection(
        enabled
            .service
            .set_publication_remote(SetPublicationRemoteRequest {
                root: fixture.root.clone(),
                name: Some("origin".to_owned()),
                operation_id: support::new_operation_id(),
            }),
    );
    assert_eq!(kind, RepositoryErrorKind::DetachedHead);
    fixture.repository.set_head("refs/heads/main").unwrap();

    assert_eq!(probe_remote(&enabled.service, &fixture.root), None);
    assert_eq!(remove_registration(&enabled.service, &fixture.root), None);
}

#[test]
fn a_rejected_enablement_does_not_block_the_repository() {
    let fixture = support::born_repository();
    let data_directory = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data_directory.path()).unwrap();

    let kind = rejection(service.enable(EnableRepositoryRequest {
        root: fixture.root.clone(),
        primary_branch: "elsewhere".to_owned(),
        identity: None,
        operation_id: support::new_operation_id(),
    }));
    assert_eq!(kind, RepositoryErrorKind::WrongCheckedOutBranch);

    assert_eq!(
        outcome(service.enable(support::enable_request(&fixture.root))),
        None
    );
}

#[test]
fn a_rejected_creation_does_not_block_the_target() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("project");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("occupied.txt"), "occupied\n").unwrap();
    let data_directory = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data_directory.path()).unwrap();
    let request = |operation_id| CreateRepositoryRequest {
        root: root.clone(),
        primary_branch: "main".to_owned(),
        identity: Some(CommitIdentity {
            name: "Manyhands Test".to_owned(),
            email: "manyhands-test@example.invalid".to_owned(),
        }),
        operation_id,
    };

    let kind = rejection(service.create_and_enable(request(support::new_operation_id())));
    assert_eq!(kind, RepositoryErrorKind::InvalidPath);
    fs::remove_file(root.join("occupied.txt")).unwrap();

    assert_eq!(
        outcome(service.create_and_enable(request(support::new_operation_id()))),
        None
    );
}

#[test]
fn a_save_rejected_before_enablement_does_not_block_the_repository() {
    let fixture = support::born_repository();
    let data_directory = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data_directory.path()).unwrap();

    let kind = rejection(service.save_document(document_request(&fixture.root)));
    assert_eq!(kind, RepositoryErrorKind::RepositoryNotEnabled);

    assert_eq!(
        outcome(service.enable(support::enable_request(&fixture.root))),
        None
    );
}

#[test]
fn a_save_rejected_on_a_detached_primary_does_not_block_the_repository() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let head = fixture.repository.head().unwrap().target().unwrap();
    fixture.repository.set_head_detached(head).unwrap();

    let kind = rejection(
        enabled
            .service
            .save_document(document_request(&fixture.root)),
    );
    assert_eq!(kind, RepositoryErrorKind::DetachedHead);
    fixture.repository.set_head("refs/heads/main").unwrap();

    assert_eq!(probe_remote(&enabled.service, &fixture.root), None);
    assert_eq!(remove_registration(&enabled.service, &fixture.root), None);
}

fn target(root: &std::path::Path, kind: AuthoringKind, intent: ContextIntent) -> AuthoringTarget {
    AuthoringTarget {
        root: root.to_owned(),
        kind,
        item_id: support::ticket_id(),
        intent,
        operation_id: support::new_operation_id(),
    }
}

fn detach(fixture: &support::TestRepository) {
    let head = fixture.repository.head().unwrap().target().unwrap();
    fixture.repository.set_head_detached(head).unwrap();
}

#[test]
fn a_rejected_ticket_save_does_not_block_the_repository() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    detach(&fixture);

    let kind = rejection(enabled.service.save_ticket(SaveTicketRequest {
        target: target(&fixture.root, AuthoringKind::Ticket, ContextIntent::Create),
        draft: TicketDraft {
            title: "Title".to_owned(),
            ticket_type: "task".to_owned(),
            status: "open".to_owned(),
            project: None,
            team: None,
            body: "Body\n".to_owned(),
        },
        expected_path: ExpectedPathObservation::Missing,
    }));
    assert_eq!(kind, RepositoryErrorKind::DetachedHead);
    fixture.repository.set_head("refs/heads/main").unwrap();

    assert_eq!(probe_remote(&enabled.service, &fixture.root), None);
    assert_eq!(remove_registration(&enabled.service, &fixture.root), None);
}

#[test]
fn a_rejected_comment_does_not_block_the_repository() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    detach(&fixture);

    let kind = rejection(enabled.service.submit_comment(SubmitCommentRequest {
        target: target(&fixture.root, AuthoringKind::Ticket, ContextIntent::Edit),
        comment_id: support::root_comment_id(),
        parent_id: None,
        body: "Body\n".to_owned(),
        expected_destination: ExpectedPathObservation::Missing,
    }));
    assert_eq!(kind, RepositoryErrorKind::DetachedHead);
    fixture.repository.set_head("refs/heads/main").unwrap();

    assert_eq!(probe_remote(&enabled.service, &fixture.root), None);
    assert_eq!(remove_registration(&enabled.service, &fixture.root), None);
}

#[test]
fn a_rejected_context_preparation_does_not_block_the_repository() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    detach(&fixture);

    let kind = rejection(enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Ticket,
        ContextIntent::Create,
    )));
    assert_eq!(kind, RepositoryErrorKind::DetachedHead);
    fixture.repository.set_head("refs/heads/main").unwrap();

    assert_eq!(probe_remote(&enabled.service, &fixture.root), None);
    assert_eq!(remove_registration(&enabled.service, &fixture.root), None);
}

#[test]
fn an_enablement_rejected_on_an_unborn_head_does_not_block_the_repository() {
    let fixture = support::born_repository();
    let data_directory = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data_directory.path()).unwrap();
    fixture.repository.set_head("refs/heads/unborn").unwrap();

    let kind = rejection(service.enable(support::enable_request(&fixture.root)));
    assert_eq!(kind, RepositoryErrorKind::WrongCheckedOutBranch);
    fixture.repository.set_head("refs/heads/main").unwrap();

    assert_eq!(
        outcome(service.enable(support::enable_request(&fixture.root))),
        None
    );
}

#[test]
fn a_save_rejected_after_its_context_was_created_does_not_block_the_repository() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let mut request = document_request(&fixture.root);
    request.destination_path = "../outside.md".into();

    let kind = rejection(enabled.service.save_document(request));
    assert_eq!(kind, RepositoryErrorKind::InvalidPath);
    assert!(
        fixture
            .root
            .join(".manyhands/worktrees")
            .join(support::document_id().to_string())
            .exists(),
        "the rejection is expected to follow context creation"
    );

    assert_eq!(probe_remote(&enabled.service, &fixture.root), None);
    assert_eq!(
        outcome(
            enabled
                .service
                .save_document(document_request(&fixture.root))
        ),
        None
    );
}

fn clean_configuration_index(fixture: &support::TestRepository) {
    let mut index = fixture.repository.index().unwrap();
    index
        .add_path(std::path::Path::new(".manyhands/config.toml"))
        .unwrap();
    index.write().unwrap();
}

#[test]
fn a_repeat_of_a_rejected_operation_begins_again() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let request = document_request(&fixture.root);
    detach(&fixture);

    assert_eq!(
        rejection(enabled.service.save_document(request.clone())),
        RepositoryErrorKind::DetachedHead
    );
    assert!(
        enabled
            .service
            .recovery_inspection(&fixture.root)
            .unwrap()
            .is_empty(),
        "the rejected call leaves no journal row"
    );
    fixture.repository.set_head("refs/heads/main").unwrap();

    // The ID stays bound to its request: another target is still a mismatch.
    let mut other = request.clone();
    other.destination_path = "docs/other.md".into();
    assert_eq!(
        rejection(enabled.service.save_document(other)),
        RepositoryErrorKind::OperationMismatch
    );
    assert_eq!(probe_remote(&enabled.service, &fixture.root), None);
    assert_eq!(outcome(enabled.service.save_document(request)), None);
    assert_eq!(remove_registration(&enabled.service, &fixture.root), None);
}

#[test]
fn a_creation_that_fails_after_initialization_keeps_its_row() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("project");
    let data_directory = tempfile::tempdir().unwrap();
    let failing =
        support::FailOnce::at(manyhands::repository::FailurePoint::BeforeConfigurationWrite)
            .open_service(data_directory.path());
    let operation_id = support::new_operation_id();
    let request = || CreateRepositoryRequest {
        root: root.clone(),
        primary_branch: "main".to_owned(),
        identity: Some(CommitIdentity {
            name: "Manyhands Test".to_owned(),
            email: "manyhands-test@example.invalid".to_owned(),
        }),
        operation_id,
    };

    rejection(failing.create_and_enable(request()));

    let service = RepositoryService::open_at(data_directory.path()).unwrap();
    assert!(
        !service.recovery_inspection(&root).unwrap().is_empty(),
        "an initialized repository still needs its creation finished"
    );
    assert_eq!(outcome(service.create_and_enable(request())), None);
}

#[test]
fn an_enablement_that_restored_its_writes_does_not_block_the_repository() {
    let fixture = support::born_repository();
    let data_directory = tempfile::tempdir().unwrap();
    let failing =
        support::FailOnce::at(manyhands::repository::FailurePoint::BeforeInitializationCommit)
            .open_service(data_directory.path());

    assert_eq!(
        rejection(failing.enable(support::enable_request(&fixture.root))),
        RepositoryErrorKind::InjectedFailure
    );

    let service = RepositoryService::open_at(data_directory.path()).unwrap();
    assert!(
        service
            .recovery_inspection(&fixture.root)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        outcome(service.enable(support::enable_request(&fixture.root))),
        None
    );
}

#[test]
fn a_repeat_of_a_rejected_operation_is_a_new_call_not_a_replay() {
    assert_repeat_is_a_new_call(false);
}

#[test]
fn a_refresh_under_the_rejected_id_does_not_turn_the_repeat_into_a_replay() {
    assert_repeat_is_a_new_call(true);
}

fn assert_repeat_is_a_new_call(refresh_between: bool) {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let request = document_request(&fixture.root);
    // With the editing context already there, the repeat records no step
    // before its write.
    let mut context = request.target.clone();
    context.operation_id = support::new_operation_id();
    assert_eq!(outcome(enabled.service.prepare_context(context)), None);
    detach(&fixture);
    assert_eq!(
        rejection(enabled.service.save_document(request.clone())),
        RepositoryErrorKind::DetachedHead
    );
    fixture.repository.set_head("refs/heads/main").unwrap();
    if refresh_between {
        let _ =
            enabled
                .service
                .refresh_repository(manyhands::repository::RefreshRepositoryRequest {
                    root: fixture.root.clone(),
                    operation_id: request.target.operation_id,
                });
    }

    // The repeat writes its file and then fails to record the step. A new
    // call keeps its row for that; a replay of a closed row would not.
    let failing = support::FailOnce::at(
        manyhands::repository::FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence,
    )
    .open_service(enabled.data_directory.path());
    assert_eq!(
        rejection(failing.save_document(request.clone())),
        RepositoryErrorKind::Sqlite
    );

    let service = RepositoryService::open_at(enabled.data_directory.path()).unwrap();
    assert!(
        !service
            .recovery_inspection(&fixture.root)
            .unwrap()
            .is_empty(),
        "a written file keeps its journal row"
    );
    assert_eq!(
        probe_remote(&service, &fixture.root),
        Some(RepositoryErrorKind::RecoveryRequired)
    );
    assert_eq!(outcome(service.save_document(request)), None);
    assert_eq!(probe_remote(&service, &fixture.root), None);
}

#[test]
fn a_rejected_operation_reads_as_completed_and_rejected() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let request = document_request(&fixture.root);
    let operation_id = request.target.operation_id;
    detach(&fixture);
    assert_eq!(
        rejection(enabled.service.save_document(request)),
        RepositoryErrorKind::DetachedHead
    );
    fixture.repository.set_head("refs/heads/main").unwrap();

    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let operation = enabled.service.show_operation(&repo, operation_id).unwrap();

    assert_eq!(operation.state, "completed");
    assert_eq!(operation.completed_step.as_deref(), Some("rejected"));
    assert_eq!(operation.next_action, None);
    assert!(
        enabled
            .service
            .list_operations(&repo)
            .unwrap()
            .items
            .is_empty()
    );
}

// Abandoning an operation: the way out for a row that its own repeat
// cannot clear. Each scenario ends with another operation on the
// repository succeeding, and with every file, reference and worktree as
// it was before the abandon.

use manyhands::repository::{
    AbandonOperationOutcome, AbandonOperationRequest, AbandonableOperation, FailurePoint,
    LeaseKind, OperationId, OperationRemains, PendingStanding,
};

/// A save whose file is written and whose step was not recorded: its row
/// is pending, and refuses every other operation on the repository.
struct Stranded {
    fixture: support::TestRepository,
    data_directory: tempfile::TempDir,
    service: RepositoryService,
    request: SaveDocumentRequest,
}

impl Stranded {
    fn new() -> Self {
        let fixture = support::born_repository();
        let enabled = support::enabled_repository(&fixture);
        clean_configuration_index(&fixture);
        let request = document_request(&fixture.root);
        let failing =
            support::FailOnce::at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence)
                .open_service(enabled.data_directory.path());
        assert_eq!(
            rejection(failing.save_document(request.clone())),
            RepositoryErrorKind::Sqlite
        );
        let support::EnabledRepository {
            service: _,
            data_directory,
        } = enabled;
        let service = RepositoryService::open_at(data_directory.path()).unwrap();
        assert_eq!(
            probe_remote(&service, &fixture.root),
            Some(RepositoryErrorKind::RecoveryRequired),
            "the pending row blocks the repository"
        );
        Self {
            fixture,
            data_directory,
            service,
            request,
        }
    }

    fn operation_id(&self) -> OperationId {
        self.request.target.operation_id
    }

    fn abandon(
        &self,
        operation_id: OperationId,
        expected: Option<PendingStanding>,
    ) -> Result<AbandonOperationOutcome, RepositoryError> {
        self.service.abandon_operation(AbandonOperationRequest {
            root: self.fixture.root.clone(),
            operation_id,
            expected,
        })
    }

    /// The journal's rows, as they are stored.
    fn journal(&self) -> Vec<(String, String, Option<String>)> {
        let connection = support::items::index(self.data_directory.path());
        let mut statement = connection
            .prepare(
                "SELECT operation_ulid, state, completed_step FROM operation_records ORDER BY id",
            )
            .unwrap();
        statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    fn written_path(&self) -> String {
        format!(
            ".manyhands/worktrees/{}/docs/new.md",
            support::document_id()
        )
    }
}

fn pending(remains: AbandonableOperation) -> OperationRemains {
    match remains {
        AbandonableOperation::Pending(remains) => remains,
        other => panic!("expected a pending operation, found {other:?}"),
    }
}

#[test]
fn an_abandoned_operation_no_longer_blocks_the_repository_and_nothing_is_touched() {
    let stranded = Stranded::new();
    let root = &stranded.fixture.root;
    let before = support::repository_and_worktree_snapshot(&stranded.fixture);

    // Looking changes nothing, and names what the operation left.
    let seen = pending(
        stranded
            .service
            .abandonable_operation(root, stranded.operation_id())
            .unwrap(),
    );
    assert_eq!(seen.action, "save_document");
    assert_eq!(seen.item_id, Some(support::document_id().to_string()));
    assert_eq!(
        seen.worktree,
        Some(format!(".manyhands/worktrees/{}", support::document_id()))
    );
    assert_eq!(seen.paths, vec![stranded.written_path()]);
    assert!(seen.paths_complete);
    assert_eq!(seen.owner, None, "no request of the boundary began it");
    assert_eq!(
        probe_remote(&stranded.service, root),
        Some(RepositoryErrorKind::RecoveryRequired)
    );
    assert_eq!(
        before,
        support::repository_and_worktree_snapshot(&stranded.fixture)
    );

    let outcome = stranded
        .abandon(stranded.operation_id(), Some(seen.standing.clone()))
        .unwrap();
    assert_eq!(outcome, AbandonOperationOutcome::Abandoned(seen));
    assert_eq!(
        before,
        support::repository_and_worktree_snapshot(&stranded.fixture),
        "the abandon touched a file, a reference or a worktree"
    );

    // Another operation runs at once.
    assert_eq!(probe_remote(&stranded.service, root), None);
    assert!(
        stranded
            .service
            .recovery_inspection(root)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn a_repeat_of_an_abandoned_operation_is_refused_and_writes_nothing() {
    let stranded = Stranded::new();
    stranded.abandon(stranded.operation_id(), None).unwrap();
    let before = support::repository_and_worktree_snapshot(&stranded.fixture);
    let journal = stranded.journal();

    // Neither replayed as completed nor begun again, with any input.
    assert_eq!(
        rejection(stranded.service.save_document(stranded.request.clone())),
        RepositoryErrorKind::OperationAbandoned
    );
    assert_eq!(
        outcome(stranded.service.refresh_repository(
            manyhands::repository::RefreshRepositoryRequest {
                root: stranded.fixture.root.clone(),
                operation_id: stranded.operation_id(),
            }
        )),
        Some(RepositoryErrorKind::OperationAbandoned)
    );

    assert_eq!(
        before,
        support::repository_and_worktree_snapshot(&stranded.fixture)
    );
    assert_eq!(
        journal,
        stranded.journal(),
        "the refusal wrote a journal row"
    );
    assert_eq!(
        probe_remote(&stranded.service, &stranded.fixture.root),
        None
    );
}

#[test]
fn an_abandoned_operation_reads_as_completed_and_abandoned() {
    let stranded = Stranded::new();
    stranded.abandon(stranded.operation_id(), None).unwrap();

    let repo = stranded
        .service
        .resolve_repository(&stranded.fixture.root)
        .unwrap();
    let operation = stranded
        .service
        .show_operation(&repo, stranded.operation_id())
        .unwrap();
    assert_eq!(operation.state, "completed");
    assert_eq!(operation.completed_step.as_deref(), Some("abandoned"));
    assert_eq!(operation.next_action, None);
    assert!(
        stranded
            .service
            .list_operations(&repo)
            .unwrap()
            .items
            .is_empty()
    );
    assert_eq!(
        stranded
            .service
            .abandonable_operation(&stranded.fixture.root, stranded.operation_id())
            .unwrap(),
        AbandonableOperation::Abandoned
    );
}

#[test]
fn only_a_pending_operation_that_stands_as_it_was_seen_is_abandoned() {
    let stranded = Stranded::new();
    let root = stranded.fixture.root.clone();
    let before = support::repository_and_worktree_snapshot(&stranded.fixture);
    let journal = stranded.journal();

    // No row of the ID.
    assert_eq!(
        stranded.abandon(support::new_operation_id(), None).unwrap(),
        AbandonOperationOutcome::NotFound
    );
    // The row is not as it was seen.
    assert_eq!(
        stranded
            .abandon(
                stranded.operation_id(),
                Some(PendingStanding {
                    state: "created".to_owned(),
                    step: Some("authoring_checkpoint_observed".to_owned()),
                }),
            )
            .unwrap(),
        AbandonOperationOutcome::Changed
    );
    assert_eq!(journal, stranded.journal());
    assert_eq!(
        probe_remote(&stranded.service, &root),
        Some(RepositoryErrorKind::RecoveryRequired)
    );

    // Abandoned, and then a no-op.
    assert!(matches!(
        stranded.abandon(stranded.operation_id(), None).unwrap(),
        AbandonOperationOutcome::Abandoned(_)
    ));
    let journal = stranded.journal();
    assert_eq!(
        stranded.abandon(stranded.operation_id(), None).unwrap(),
        AbandonOperationOutcome::AlreadyAbandoned
    );

    // A completed operation and a rejected one are left as they are.
    let completed = support::new_operation_id();
    assert_eq!(
        outcome(stranded.service.add_remote(AddRemoteRequest {
            root: root.clone(),
            name: "origin".to_owned(),
            url: "https://example.invalid/origin.git".to_owned(),
            operation_id: completed,
        })),
        None
    );
    let rejected = support::new_operation_id();
    assert_eq!(
        rejection(stranded.service.add_remote(AddRemoteRequest {
            root: root.clone(),
            name: "not a name".to_owned(),
            url: "https://example.invalid/origin.git".to_owned(),
            operation_id: rejected,
        })),
        RepositoryErrorKind::Git
    );
    let journal_after = stranded.journal();
    assert_eq!(journal_after.len(), journal.len() + 2);
    assert_eq!(
        stranded.abandon(completed, None).unwrap(),
        AbandonOperationOutcome::Completed
    );
    assert_eq!(
        stranded.abandon(rejected, None).unwrap(),
        AbandonOperationOutcome::Rejected
    );
    assert_eq!(journal_after, stranded.journal());
    // A rejected ID still begins again: abandon did not spend it.
    assert_eq!(
        outcome(stranded.service.add_remote(AddRemoteRequest {
            root: root.clone(),
            name: "not a name".to_owned(),
            url: "https://example.invalid/origin.git".to_owned(),
            operation_id: rejected,
        })),
        Some(RepositoryErrorKind::Git)
    );
    // Only the remote the completed operation added differs.
    let after = support::repository_and_worktree_snapshot(&stranded.fixture);
    assert_eq!(before.root.files, after.root.files);
    assert_eq!(before.references, after.references);
    assert_eq!(before.linked_worktrees, after.linked_worktrees);
}

#[test]
fn an_operation_a_call_is_working_under_is_not_abandoned() {
    let stranded = Stranded::new();
    let root = stranded.fixture.root.clone();
    let journal = stranded.journal();

    // A call holds the repository lease.
    let held = RepositoryService::hold_lease_for_testing(
        &root,
        stranded.data_directory.path(),
        LeaseKind::Repository,
    )
    .unwrap();
    assert_eq!(
        rejection(stranded.abandon(stranded.operation_id(), None)),
        RepositoryErrorKind::RepositoryBusy
    );
    drop(held);

    // A creation holds the bootstrap lease.
    let held = RepositoryService::hold_lease_for_testing(
        &root,
        stranded.data_directory.path(),
        LeaseKind::Bootstrap,
    )
    .unwrap();
    assert_eq!(
        rejection(stranded.abandon(stranded.operation_id(), None)),
        RepositoryErrorKind::RepositoryBusy
    );
    drop(held);
    assert_eq!(journal, stranded.journal());

    // An index owner's heartbeat is fresh: it holds no lease.
    let connection = support::items::index(stranded.data_directory.path());
    let set = |observed_at: &str| {
        connection
            .execute(
                &format!(
                    "UPDATE operation_records SET state = 'indexing', observed_at = {observed_at}
                      WHERE state != 'completed'"
                ),
                [],
            )
            .unwrap()
    };
    assert_eq!(set("CAST(strftime('%s', 'now') AS INTEGER)"), 1);
    assert_eq!(
        rejection(stranded.abandon(stranded.operation_id(), None)),
        RepositoryErrorKind::RepositoryBusy
    );
    assert_eq!(
        probe_remote(&stranded.service, &root),
        Some(RepositoryErrorKind::RecoveryRequired)
    );

    // Its owner died: the heartbeat is stale, and the row is closed.
    assert_eq!(set("CAST(strftime('%s', 'now') AS INTEGER) - 60"), 1);
    drop(connection);
    assert!(matches!(
        stranded.abandon(stranded.operation_id(), None).unwrap(),
        AbandonOperationOutcome::Abandoned(_)
    ));
    assert_eq!(probe_remote(&stranded.service, &root), None);
}

#[test]
fn an_abandoned_enablement_names_the_configuration_file_it_left() {
    let fixture = support::born_repository();
    let data_directory = tempfile::tempdir().unwrap();
    let request = support::enable_request(&fixture.root);
    let operation_id = request.operation_id;
    // Stands in for a kill: the configuration file is written and the row
    // is left as a dead process leaves it.
    let service = RepositoryService::open_at(data_directory.path()).unwrap();
    assert_eq!(outcome(service.enable(request.clone())), None);
    fixture
        .repository
        .reset(
            &fixture
                .repository
                .head()
                .unwrap()
                .peel_to_commit()
                .unwrap()
                .parent(0)
                .unwrap()
                .into_object(),
            git2::ResetType::Soft,
            None,
        )
        .unwrap();
    let mut index = fixture.repository.index().unwrap();
    index
        .remove_path(std::path::Path::new(".manyhands/config.toml"))
        .unwrap();
    index.write().unwrap();
    let connection = support::items::index(data_directory.path());
    connection
        .execute(
            "UPDATE operation_records SET state = 'created', completed_step = NULL",
            [],
        )
        .unwrap();
    connection.execute("DELETE FROM repositories", []).unwrap();
    drop(connection);
    let before = support::repository_and_worktree_snapshot(&fixture);

    let outcome = service
        .abandon_operation(AbandonOperationRequest {
            root: fixture.root.clone(),
            operation_id,
            expected: None,
        })
        .unwrap();
    let AbandonOperationOutcome::Abandoned(left) = outcome else {
        panic!("expected the enablement to be abandoned, found {outcome:?}");
    };
    assert_eq!(left.action, "enable");
    assert_eq!(left.item_id, None);
    assert_eq!(left.worktree.as_deref(), Some("."));
    assert_eq!(left.paths, vec![".manyhands/config.toml".to_owned()]);
    assert_eq!(before, support::repository_and_worktree_snapshot(&fixture));
}
