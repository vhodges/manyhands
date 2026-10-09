use std::fs;

use manyhands::repository::{
    AddRemoteRequest, AuthoringKind, AuthoringTarget, CommitIdentity, ContextIntent,
    CreateRepositoryRequest, DocumentDraft, EnableRepositoryRequest, ExpectedPathObservation,
    RemoveRegistrationRequest, RemoveRemoteRequest, RepositoryError, RepositoryErrorKind,
    RepositoryService, SaveDocumentRequest, SetPublicationRemoteRequest,
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
    eprintln!("rejection: {kind:?}");

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
    eprintln!("rejection: {kind:?}");

    assert_eq!(probe_remote(&enabled.service, &fixture.root), None);
    assert_eq!(remove_registration(&enabled.service, &fixture.root), None);
}

#[test]
fn a_rejected_remote_selection_does_not_block_the_repository() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);

    let kind = rejection(
        enabled
            .service
            .set_publication_remote(SetPublicationRemoteRequest {
                root: fixture.root.clone(),
                name: Some("not a name".to_owned()),
                operation_id: support::new_operation_id(),
            }),
    );
    eprintln!("rejection: {kind:?}");

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
    eprintln!("rejection: {kind:?}");

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
    eprintln!("rejection: {kind:?}");
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
    eprintln!("rejection: {kind:?}");

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
    eprintln!("rejection: {kind:?}");
    fixture.repository.set_head("refs/heads/main").unwrap();

    assert_eq!(probe_remote(&enabled.service, &fixture.root), None);
    assert_eq!(remove_registration(&enabled.service, &fixture.root), None);
}
