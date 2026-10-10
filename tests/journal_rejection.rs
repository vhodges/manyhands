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
