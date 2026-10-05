use crate::{
    repository::transport::{operation_tests::*, *},
    session::*,
    ssh_remote::*,
};
use std::cell::Cell;

pub const CASES: &[crate::ssh_harness::Case] = &[
    ("scoped_download", download),
    ("scoped_push", push),
    ("push_rejection", push_rejection),
    ("reconnect_key_policy", reconnect_key_policy),
    ("reconnect_host_policy", reconnect_host_policy),
    ("removed_pin_before_transfer", removed_pin_before_transfer),
    (
        "renewed_rejection_evicts_secret",
        renewed_rejection_evicts_secret,
    ),
    (
        "transfer_failure_preserves_valid_secret",
        transfer_failure_preserves_valid_secret,
    ),
];
fn download() -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    let (mut session, requests) = session(vec![]);
    let called = Cell::new(0);
    let heads = fixed(transfer(
        &case.service,
        case.request(),
        &mut session,
        Transfer::Download,
        &called,
        || {},
    ))?;
    assert!(heads.contains(&case.fixture.commit_id()));
    let repo = fixed(git2::Repository::open(&case.root))?;
    assert!(repo.find_commit(case.fixture.commit_id()).is_ok());
    assert!(repo.find_reference("refs/remotes/origin/main").is_err());
    fixed(repo.reference(
        "refs/remotes/origin/main",
        case.fixture.commit_id(),
        false,
        "fixture tracking update",
    ))?;
    assert_eq!(
        fixed(repo.refname_to_id("refs/remotes/origin/main"))?,
        case.fixture.commit_id()
    );
    assert_eq!(called.get(), 1);
    assert!(requests.borrow().is_empty());
    assert!(
        case.fixture
            .accepted_keys()
            .iter()
            .all(|key| key == &case.fixture.allowed_client_public_key())
    );
    Ok(())
}
pub fn commit(case: &Case) -> Result<git2::Oid, FixtureError> {
    let repo = fixed(git2::Repository::open(&case.root))?;
    let tree = fixed(fixed(repo.treebuilder(None))?.write())?;
    let tree = fixed(repo.find_tree(tree))?;
    let signature = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    fixed(repo.commit(
        Some("refs/heads/pushed"),
        &signature,
        &signature,
        "fixture",
        &tree,
        &[],
    ))
}
fn push() -> Result<(), FixtureError> {
    let case = Case::new(true)?;
    let oid = commit(&case)?;
    let (mut session, requests) = session(vec![secret(PASSWORD)]);
    let mut request = case.request();
    request.direction = SshDirection::Push;
    let called = Cell::new(0);
    fixed(transfer(
        &case.service,
        request,
        &mut session,
        Transfer::Push,
        &called,
        || {},
    ))?;
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    assert_eq!(fixed(server.refname_to_id("refs/heads/pushed"))?, oid);
    assert_eq!(called.get(), 1);
    assert_eq!(requests.borrow().len(), 1);
    Ok(())
}
fn push_rejection() -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    commit(&case)?;
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    fixed(fixed(server.config())?.set_bool("receive.denyNonFastForwards", true))?;
    // Characterize the native API: an actual per-ref server rejection can
    // coexist with a successful top-level push call.
    let repo = fixed(git2::Repository::open(&case.root))?;
    let mut raw = fixed(repo.remote_anonymous(&case.fixture.url()))?;
    let rejected = Cell::new(false);
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.certificate_check(|_, _| Ok(git2::CertificateCheckStatus::CertificateOk));
    callbacks.credentials(|_, _, _| {
        git2::Cred::ssh_key("fixture", None, case.fixture.client_key_path(), None)
    });
    callbacks.push_update_reference(|_, status| {
        rejected.set(status.is_some());
        Ok(())
    });
    let mut options = git2::PushOptions::new();
    options.remote_callbacks(callbacks);
    fixed(raw.push(&["+refs/heads/pushed:refs/heads/main"], Some(&mut options)))?;
    assert!(rejected.get());
    let (mut session, _) = session(vec![]);
    let mut request = case.request();
    request.direction = SshDirection::Push;
    let error = transfer(
        &case.service,
        request,
        &mut session,
        Transfer::RejectedPush,
        &Cell::new(0),
        || {},
    )
    .unwrap_err();
    assert_eq!(error.kind, SshTransportErrorKind::PushRejected);
    assert_eq!(
        fixed(server.refname_to_id("refs/heads/main"))?,
        case.fixture.commit_id()
    );
    Ok(())
}
fn removed_pin_before_transfer() -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    let (mut session, _) = session(vec![]);
    let error = transfer(
        &case.service,
        case.request(),
        &mut session,
        Transfer::Download,
        &Cell::new(0),
        || {
            let registry =
                rusqlite::Connection::open(case.directory.path().join("data/manyhands.sqlite3"))
                    .unwrap();
            registry.execute("DELETE FROM ssh_host_pins", []).unwrap();
        },
    )
    .unwrap_err();
    assert_eq!(error.kind, SshTransportErrorKind::HostTrustChanged);
    let repo = fixed(git2::Repository::open(&case.root))?;
    assert!(repo.find_commit(case.fixture.commit_id()).is_err());
    Ok(())
}
fn transfer_failure_preserves_valid_secret() -> Result<(), FixtureError> {
    let case = Case::new(true)?;
    let (mut session, requests) = session(vec![secret(PASSWORD)]);
    let error = transfer(
        &case.service,
        case.request(),
        &mut session,
        Transfer::Download,
        &Cell::new(0),
        || {
            case.fixture.disconnect_at(FixtureBoundary::Advertisement);
        },
    )
    .unwrap_err();
    assert!(matches!(
        error.kind,
        SshTransportErrorKind::TransportUnavailable
            | SshTransportErrorKind::RemoteUnavailable
            | SshTransportErrorKind::ProtocolFailure
    ));
    assert_eq!(requests.borrow().len(), 1);
    assert!(session.has_cached_passphrase(&requests.borrow()[0]));
    Ok(())
}
fn renewed_rejection_evicts_secret() -> Result<(), FixtureError> {
    let case = Case::new(true)?;
    let (mut session, requests) = session(vec![secret(PASSWORD)]);
    let error = transfer(
        &case.service,
        case.request(),
        &mut session,
        Transfer::Download,
        &Cell::new(0),
        || {
            case.fixture.reject_client();
        },
    )
    .unwrap_err();
    assert_eq!(error.kind, SshTransportErrorKind::UnlockFailed);
    assert_eq!(requests.borrow().len(), 1);
    assert!(!session.has_cached_passphrase(&requests.borrow()[0]));
    Ok(())
}
fn reconnect_key_policy() -> Result<(), FixtureError> {
    reconnect(false)
}
fn reconnect_host_policy() -> Result<(), FixtureError> {
    reconnect(true)
}
fn reconnect(host: bool) -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    let (mut session, _) = session(vec![]);
    let called = Cell::new(0);
    let (_guard, phases) = diagnostic_phases();
    let result = transfer(
        &case.service,
        case.request(),
        &mut session,
        Transfer::Download,
        &called,
        || {
            if host {
                case.fixture.rotate_host_key().unwrap();
            } else {
                case.fixture.reject_client();
            }
        },
    );
    let expected = result.as_ref().is_err_and(|error| {
        if host {
            matches!(
                error.kind,
                SshTransportErrorKind::HostApprovalRequired { .. }
                    | SshTransportErrorKind::HostReplacementRequired { .. }
                    | SshTransportErrorKind::HostTrustChanged
            )
        } else {
            error.kind == SshTransportErrorKind::KeyRejected
        }
    });
    if !expected {
        observe_transport_failure(&case, &result, called.get(), &phases);
    }
    let error = result.unwrap_err();
    if host {
        assert!(matches!(
            error.kind,
            SshTransportErrorKind::HostApprovalRequired { .. }
                | SshTransportErrorKind::HostReplacementRequired { .. }
                | SshTransportErrorKind::HostTrustChanged
        ));
    } else {
        assert_eq!(error.kind, SshTransportErrorKind::KeyRejected);
    }
    let repo = fixed(git2::Repository::open(&case.root))?;
    assert!(repo.find_commit(case.fixture.commit_id()).is_err());
    Ok(())
}
