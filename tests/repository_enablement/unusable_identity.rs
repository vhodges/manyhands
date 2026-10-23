//! An identity Git cannot commit as is refused before onboarding writes
//! anything: a supplied one as invalid, and one from Git's configuration
//! as no identity at all.

use super::*;

const EMAIL: &str = "example@example.invalid";

/// The supplied identities a commit would not carry as given, with the
/// plain one beside them that it would.
fn bracketed() -> [(&'static str, CommitIdentity); 2] {
    [
        (
            "a bracketed name",
            CommitIdentity {
                name: format!("Example Person <{EMAIL}>"),
                email: EMAIL.to_owned(),
            },
        ),
        (
            "a bracketed email",
            CommitIdentity {
                name: "Example Person".to_owned(),
                email: format!("<{EMAIL}>"),
            },
        ),
    ]
}

fn plain() -> CommitIdentity {
    CommitIdentity {
        name: "Example Person".to_owned(),
        email: EMAIL.to_owned(),
    }
}

const BRACKETED_CONFIGURATION: &str = "[user]\n\tname = \"Example Person <example@example.invalid>\"\n\temail = example@example.invalid\n";

fn assert_nothing_recorded(
    service: &RepositoryService,
    data: &std::path::Path,
    root: &std::path::Path,
) {
    assert_eq!(registry_row_count(data), 0);
    assert_eq!(service.recovery_inspection(root).unwrap(), Vec::new());
}

#[test]
fn create_refuses_an_identity_git_cannot_commit_and_leaves_the_target_as_it_was() {
    for exists in [false, true] {
        for (what, identity) in bracketed() {
            let data = tempfile::tempdir().unwrap();
            let parent = tempfile::tempdir().unwrap();
            let service = RepositoryService::open_at(data.path()).unwrap();
            let root = parent.path().join("new");
            if exists {
                std::fs::create_dir(&root).unwrap();
            }

            let error = service
                .create_and_enable(CreateRepositoryRequest {
                    identity: Some(identity.clone()),
                    ..create_request(&root, "main")
                })
                .unwrap_err();

            let what = format!("{what}, target exists: {exists}");
            assert_eq!(
                error.operation,
                RepositoryOperation::CreateAndEnable,
                "{what}"
            );
            assert_eq!(error.kind, RepositoryErrorKind::InvalidIdentity, "{what}");
            if exists {
                assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0, "{what}");
                assert_nothing_recorded(&service, data.path(), &root);
            } else {
                assert!(!root.exists(), "{what}");
                assert_eq!(registry_row_count(data.path()), 0, "{what}");
            }

            // The same target and the plain name: created and enabled.
            let created = service
                .create_and_enable(CreateRepositoryRequest {
                    identity: Some(plain()),
                    ..create_request(&root, "main")
                })
                .unwrap();
            assert!(
                matches!(created, EnableRepositoryOutcome::Enabled { .. }),
                "{what}: {created:?}"
            );
            let local = Config::open(&root.join(".git/config")).unwrap();
            assert_eq!(
                local.get_string("user.name").ok().as_deref(),
                Some("Example Person"),
                "{what}"
            );
        }
    }
}

#[test]
fn first_enable_refuses_a_supplied_identity_git_cannot_commit_and_writes_nothing() {
    for unborn in [false, true] {
        for (what, identity) in bracketed() {
            let data = tempfile::tempdir().unwrap();
            let service = RepositoryService::open_at(data.path()).unwrap();
            let fixture = if unborn {
                support::unborn_repository()
            } else {
                support::repository_without_local_identity()
            };
            let before = repository_snapshot(&fixture.repository, &fixture.root);

            let error = service
                .enable(EnableRepositoryRequest {
                    identity: Some(identity.clone()),
                    ..enable_request(&fixture.root, "main")
                })
                .unwrap_err();

            let what = format!("{what}, unborn: {unborn}");
            assert_eq!(error.operation, RepositoryOperation::Enable, "{what}");
            assert_eq!(error.kind, RepositoryErrorKind::InvalidIdentity, "{what}");
            assert_eq!(
                repository_snapshot(&fixture.repository, &fixture.root),
                before,
                "{what}"
            );
            assert!(!fixture.root.join(".manyhands").exists(), "{what}");
            assert_nothing_recorded(&service, data.path(), &fixture.root);
        }
    }
}

#[test]
fn first_enable_with_a_configured_identity_git_cannot_commit_is_identity_required() {
    // From the repository's own configuration, and from the global one.
    for local in [true, false] {
        let data = tempfile::tempdir().unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        let fixture = support::repository_without_local_identity();
        let (_directory, mut effective_config) = if local {
            let mut config = Config::open(&fixture.repository.path().join("config")).unwrap();
            config
                .set_str("user.name", &format!("Example Person <{EMAIL}>"))
                .unwrap();
            config.set_str("user.email", EMAIL).unwrap();
            isolated_effective_config("")
        } else {
            isolated_effective_config(BRACKETED_CONFIGURATION)
        };
        let before = repository_snapshot(&fixture.repository, &fixture.root);

        let inspection = service
            .inspect_with_identity_config_for_testing(&fixture.root, &mut effective_config)
            .unwrap();
        let outcome = service
            .enable_with_identity_config_for_testing(
                enable_request(&fixture.root, "main"),
                &mut effective_config,
            )
            .unwrap();

        assert_eq!(
            inspection.identity,
            IdentityInspection::Required,
            "local: {local}"
        );
        assert_eq!(
            outcome,
            EnableRepositoryOutcome::IdentityRequired,
            "local: {local}"
        );
        assert_eq!(
            repository_snapshot(&fixture.repository, &fixture.root),
            before,
            "local: {local}"
        );
        assert_eq!(support::tracked_configuration(&fixture.root), None);
        assert_nothing_recorded(&service, data.path(), &fixture.root);
    }
}

/// A create already left partial keeps its pending row when it is taken
/// up with an identity Git cannot commit as, and the plain one still
/// completes it.
#[test]
fn an_outstanding_partial_create_keeps_its_recovery_when_its_retry_is_refused() {
    let data = tempfile::tempdir().unwrap();
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("new");
    let operation_id = support::operation_id();
    let failing = support::FailOnce::at(FailurePoint::AfterRepositoryInitialization)
        .open_service(data.path());
    assert!(
        failing
            .create_and_enable(create_request_with_operation_id(
                &root,
                "main",
                operation_id
            ))
            .is_err()
    );
    drop(failing);
    let service = RepositoryService::open_at(data.path()).unwrap();
    let pending = service.recovery_inspection(&root).unwrap();
    assert!(matches!(
        pending.as_slice(),
        [RecoveryInspection::Pending {
            operation: RepositoryOperation::CreateAndEnable,
            ..
        }]
    ));

    for (what, identity) in bracketed() {
        let error = service
            .enable(EnableRepositoryRequest {
                identity: Some(identity.clone()),
                ..enable_request_with_operation_id(&root, "main", operation_id)
            })
            .unwrap_err();

        assert_eq!(error.kind, RepositoryErrorKind::InvalidIdentity, "{what}");
        assert_eq!(
            service.recovery_inspection(&root).unwrap(),
            pending,
            "{what}"
        );
        assert!(!root.join(".manyhands").exists(), "{what}");
    }

    let enabled = service
        .enable(EnableRepositoryRequest {
            identity: Some(plain()),
            ..enable_request_with_operation_id(&root, "main", operation_id)
        })
        .unwrap();
    assert!(
        matches!(enabled, EnableRepositoryOutcome::Enabled { .. }),
        "{enabled:?}"
    );
    assert_eq!(service.recovery_inspection(&root).unwrap(), Vec::new());
}
