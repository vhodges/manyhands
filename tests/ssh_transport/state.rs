use crate::{
    repository::{
        keys::*,
        transport::{operation_tests::*, *},
    },
    session::*,
    ssh_remote::*,
};
use std::{cell::Cell, rc::Rc};

pub const CASES: &[crate::ssh_harness::Case] = &[
    ("state_rechecks", state_rechecks),
    ("provider_registry_reentrant", provider_registry_reentrant),
    ("source_preflight", source_preflight),
    ("no_selected_key", no_selected_key),
    ("username_required", username_required),
    ("malformed_key", malformed_key),
    ("source_replaced_new_session", source_replaced_new_session),
    ("selection_clears_cache", selection_clears_cache),
    ("unknown_host_no_prompt", unknown_host_no_prompt),
    ("anonymous_rejected", anonymous_rejected),
    (
        "normalized_url_rewrite_rejected",
        normalized_url_rewrite_rejected,
    ),
];
fn state_rechecks() -> Result<(), FixtureError> {
    for checkpoint in [
        Checkpoint::Prepared,
        Checkpoint::ProviderReturned,
        Checkpoint::Authenticated,
    ] {
        for mutation in 0..4 {
            let case = Rc::new(Case::new(checkpoint == Checkpoint::ProviderReturned)?);
            let hook_case = case.clone();
            let changed = Rc::new(Cell::new(false));
            let hook_changed = changed.clone();
            let _guard = install_hook(move |point| {
                if point != checkpoint || hook_changed.replace(true) {
                    return;
                }
                match mutation {
                    0 => {
                        hook_case.service.clear_shared_key_selection().unwrap();
                    }
                    1 => {
                        std::fs::write(hook_case.fixture.client_key_path(), b"replaced").unwrap();
                    }
                    2 => {
                        let repo = git2::Repository::open(&hook_case.root).unwrap();
                        repo.remote_set_url("origin", "ssh://fixture@127.0.0.1:9/changed.git")
                            .unwrap();
                    }
                    _ => {
                        let key = generate_key().unwrap();
                        let mut approval = hook_case.request().approval.unwrap();
                        approval.presented = HostKeyIdentity {
                            algorithm: key.algorithm().to_string(),
                            sha256: key
                                .public_key()
                                .fingerprint(russh::keys::HashAlg::Sha256)
                                .to_string(),
                        };
                        install_pin(&hook_case.service, &approval);
                    }
                }
            });
            let (mut session, _) = session(if checkpoint == Checkpoint::ProviderReturned {
                vec![secret(PASSWORD)]
            } else {
                vec![]
            });
            let called = Cell::new(0);
            let error = transfer(
                &case.service,
                case.request(),
                &mut session,
                Transfer::Advertisement,
                &called,
                || {},
            )
            .unwrap_err();
            assert!(changed.get());
            assert_eq!(called.get(), 0);
            assert_eq!(
                error.kind,
                match mutation {
                    0 => SshTransportErrorKind::SelectionChanged,
                    1 => SshTransportErrorKind::KeySourceChanged,
                    2 => SshTransportErrorKind::EndpointChanged,
                    _ => SshTransportErrorKind::HostTrustChanged,
                }
            );
        }
    }
    Ok(())
}
fn provider_registry_reentrant() -> Result<(), FixtureError> {
    struct Reentrant<'a>(&'a Case);
    impl SessionCredentialProvider for Reentrant<'_> {
        fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
            self.0.service.clear_shared_key_selection().unwrap();
            secret(PASSWORD)
        }
    }
    let case = Case::new(true)?;
    let mut session = SessionCredentials::new(Reentrant(&case));
    let called = Cell::new(0);
    let error = transfer(
        &case.service,
        case.request(),
        &mut session,
        Transfer::Advertisement,
        &called,
        || {},
    )
    .unwrap_err();
    assert_eq!(error.kind, SshTransportErrorKind::SelectionChanged);
    assert_eq!(called.get(), 0);
    Ok(())
}
fn source_preflight() -> Result<(), FixtureError> {
    for regular in [false, true] {
        let case = Case::new(false)?;
        fixed(std::fs::remove_file(case.fixture.client_key_path()))?;
        if regular {
            fixed(std::fs::create_dir(case.fixture.client_key_path()))?;
        }
        let (mut session, requests) = session(vec![]);
        let error = case.verify(&mut session).unwrap_err();
        assert_eq!(
            error.kind,
            if regular {
                SshTransportErrorKind::KeyUnreadable
            } else {
                SshTransportErrorKind::KeyMissing
            }
        );
        assert_eq!(error.selected_key_id, Some(case.registration.id));
        assert!(requests.borrow().is_empty());
        assert_eq!(case.fixture.helper_invocations(), 0);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let case = Case::new(false)?;
        fixed(std::fs::set_permissions(
            case.fixture.client_key_path(),
            std::fs::Permissions::from_mode(0o000),
        ))?;
        let (mut session, _) = session(vec![]);
        let result = case.verify(&mut session);
        fixed(std::fs::set_permissions(
            case.fixture.client_key_path(),
            std::fs::Permissions::from_mode(0o600),
        ))?;
        assert_eq!(
            result.unwrap_err().kind,
            SshTransportErrorKind::KeyUnreadable
        );
    }
    Ok(())
}
fn no_selected_key() -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    fixed(case.service.clear_shared_key_selection())?;
    let (mut session, requests) = session(vec![]);
    let error = case.verify(&mut session).unwrap_err();
    assert_eq!(error.kind, SshTransportErrorKind::NoSelectedKey);
    assert_eq!(error.selected_key_id, None);
    assert!(requests.borrow().is_empty());
    assert_eq!(case.fixture.helper_invocations(), 0);
    Ok(())
}
fn username_required() -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    let repo = fixed(git2::Repository::open(&case.root))?;
    fixed(repo.remote_set_url("origin", &case.fixture.url().replace("fixture@", "")))?;
    let (mut session, _) = session(vec![]);
    let error = case.verify(&mut session).unwrap_err();
    assert_eq!(error.kind, SshTransportErrorKind::UsernameRequired);
    assert_eq!(error.selected_key_id, Some(case.registration.id));
    assert_eq!(case.fixture.helper_invocations(), 0);
    Ok(())
}
fn malformed_key() -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    fixed(std::fs::write(
        case.fixture.client_key_path(),
        b"not a private key",
    ))?;
    let before = fixed(case.service.list_shared_keys())?;
    let (mut session, requests) = session(vec![PassphraseResponse::Cancelled]);
    assert_eq!(
        case.verify(&mut session).unwrap_err().kind,
        SshTransportErrorKind::UnlockCancelled
    );
    assert_eq!(requests.borrow().len(), 1);
    assert_eq!(before, fixed(case.service.list_shared_keys())?);
    assert!(fixed(std::fs::read(case.fixture.client_key_path()))? == b"not a private key");
    Ok(())
}
fn source_replaced_new_session() -> Result<(), FixtureError> {
    let case = Case::new(true)?;
    let (mut session1, requests1) = session(vec![secret(PASSWORD), secret(PASSWORD)]);
    fixed(case.verify(&mut session1))?;
    let key = generate_key()?;
    case.fixture
        .allow_client_public_key(key.public_key().clone());
    let plain = fixed(key.to_openssh(russh::keys::ssh_key::LineEnding::LF))?;
    let key = fixed(ssh_key::PrivateKey::from_openssh(plain.as_bytes()))?;
    let key = fixed(key.encrypt(&mut ssh_key::rand_core::OsRng, PASSWORD))?;
    fixed(std::fs::write(
        case.fixture.client_key_path(),
        fixed(key.to_openssh(ssh_key::LineEnding::LF))?.as_bytes(),
    ))?;
    fixed(case.verify(&mut session1))?;
    assert_eq!(requests1.borrow().len(), 2);
    let (mut session2, requests2) = session(vec![secret(PASSWORD)]);
    fixed(case.verify(&mut session2))?;
    assert_eq!(requests2.borrow().len(), 1);
    Ok(())
}
fn selection_clears_cache() -> Result<(), FixtureError> {
    let case = Case::new(true)?;
    let (mut session, requests) = session(vec![secret(PASSWORD)]);
    fixed(case.verify(&mut session))?;
    let key = generate_key()?;
    let path = case.directory.path().join("new plain");
    fixed(std::fs::write(
        &path,
        fixed(key.to_openssh(russh::keys::ssh_key::LineEnding::LF))?.as_bytes(),
    ))?;
    case.fixture
        .allow_client_public_key(key.public_key().clone());
    let RegisterSharedKeyOutcome::Registered(registration) =
        fixed(case.service.register_shared_key(RegisterSharedKeyRequest {
            label: "new".into(),
            ownership: SharedKeyOwnership::Imported,
            private_key_path: path,
            public_key_path: None,
        }))?
    else {
        return Err(FixtureError);
    };
    fixed(case.service.select_shared_key(registration.id))?;
    fixed(case.verify(&mut session))?;
    assert!(!session.has_cached_passphrase(&requests.borrow()[0]));
    assert_eq!(requests.borrow().len(), 1);
    Ok(())
}
fn unknown_host_no_prompt() -> Result<(), FixtureError> {
    let case = Case::new(true)?;
    let mut request = case.request();
    request.approval = None;
    let (mut session, requests) = session(vec![]);
    let error = case
        .service
        .verify_ssh_transport(request, &mut session)
        .unwrap_err();
    assert_eq!(
        error.kind,
        SshTransportErrorKind::HostApprovalRequired {
            presented: case.fixture.host_identity()
        }
    );
    assert!(requests.borrow().is_empty());
    assert_eq!(case.fixture.helper_invocations(), 0);
    Ok(())
}
fn anonymous_rejected() -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    case.fixture.accept_anonymous();
    let (mut session, requests) = session(vec![]);
    let called = Cell::new(0);
    let error = transfer(
        &case.service,
        case.request(),
        &mut session,
        Transfer::Advertisement,
        &called,
        || {},
    )
    .unwrap_err();
    assert_eq!(error.kind, SshTransportErrorKind::KeyRejected);
    assert_eq!(called.get(), 0);
    assert!(requests.borrow().is_empty());
    assert!(case.fixture.accepted_keys().is_empty());
    Ok(())
}
fn normalized_url_rewrite_rejected() -> Result<(), FixtureError> {
    for direction in [SshDirection::Fetch, SshDirection::Push] {
        let case = Case::new(false)?;
        let repo = fixed(git2::Repository::open(&case.root))?;
        let raw = format!(
            "ssh://fixture@127.0.0.1:0{}/fixture.git",
            case.fixture.address().port()
        );
        fixed(repo.remote_set_url("origin", &raw))?;
        let changed = case.fixture.url().replace("fixture.git", "different.git");
        let rule = if direction == SshDirection::Fetch {
            "insteadOf"
        } else {
            "pushInsteadOf"
        };
        fixed(
            fixed(repo.config())?.set_str(&format!("url.{changed}.{rule}"), &case.fixture.url()),
        )?;
        let (mut session, requests) = session(vec![]);
        let mut request = case.request();
        request.direction = direction;
        let error = case
            .service
            .verify_ssh_transport(request, &mut session)
            .unwrap_err();
        assert_eq!(error.kind, SshTransportErrorKind::ConfigurationInvalid);
        assert!(requests.borrow().is_empty());
        assert!(case.fixture.accepted_keys().is_empty());
        assert_eq!(case.fixture.helper_invocations(), 0);
    }
    Ok(())
}
