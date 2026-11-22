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
            // Named edit E-50: the download runs on the connection that already
            // advertised, so the fault is at its transfer (was Advertisement,
            // which only a reconnect reached).
            case.fixture.disconnect_at(FixtureBoundary::Transfer);
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
    // Named edit E-49: a transfer no longer authenticates again, so the server
    // rejects the key on the connection that uses the passphrase just supplied
    // (was on the transfer's reconnect).
    let fixture = std::rc::Rc::new(case);
    let rejecting = fixture.clone();
    let hook = install_hook(move |point| {
        if point == Checkpoint::ProviderReturned {
            rejecting.fixture.reject_client();
        }
    });
    let case = &*fixture;
    let called = Cell::new(0);
    let error = transfer(
        &case.service,
        case.request(),
        &mut session,
        Transfer::Download,
        &called,
        || {},
    )
    .unwrap_err();
    drop(hook);
    assert_eq!(called.get(), 0);
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
/// Named edit E-48: a transfer no longer reconnects, so these cases make the
/// call ask for one on its own connection after a push closed it: a second
/// push in the same call. Named edit E-52 (R-SDD-51): that transfer refuses
/// before libgit2 is called, with `ProtocolFailure`, whatever the server
/// would grant; no second connection is opened, no second key offered, and
/// a rotated host key is never even presented. The attempt's own refusal of a
/// second credential request, the second line, is pinned by the library test
/// `a_second_key_submission_on_one_connection_fails_closed`. (E-48 had the
/// second push reach libgit2's reconnect, refused by the attempt with
/// `KeyRejected` or a host kind; before E-48, a fresh reconnect before each
/// transfer was refused by a rejecting server or a rotated host key.)
#[allow(clippy::result_large_err)] // The scoped transport's owned, redacted error contract.
fn reconnect(host: bool) -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    let pushed = commit(&case)?;
    let (mut session, _) = session(vec![]);
    let mut request = case.request();
    request.direction = SshDirection::Push;
    let first = Cell::new(false);
    let sockets = case.fixture.accepted_sockets();
    let result = case
        .service
        .with_authenticated_remote(request, &mut session, |remote| {
            remote.push(&["refs/heads/pushed:refs/heads/pushed"])?;
            first.set(true);
            if host {
                case.fixture.rotate_host_key().unwrap();
            }
            remote.push(&["refs/heads/pushed:refs/heads/again"])
        });
    assert!(first.get());
    assert_eq!(
        result.unwrap_err().kind,
        SshTransportErrorKind::ProtocolFailure
    );
    // One connection in the whole call: one key offered, and accepted.
    assert_eq!(case.fixture.accepted_sockets(), sockets + 1);
    assert_eq!(case.fixture.authentication_counts(), (1, 0));
    assert_eq!(
        case.fixture.accepted_keys(),
        vec![case.fixture.allowed_client_public_key()]
    );
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    assert_eq!(fixed(server.refname_to_id("refs/heads/pushed"))?, pushed);
    assert!(server.find_reference("refs/heads/again").is_err());
    Ok(())
}
