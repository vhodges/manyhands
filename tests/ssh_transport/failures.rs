use crate::{
    repository::transport::{operation_tests::*, *},
    session::*,
    ssh_remote::*,
};
use std::cell::Cell;
pub const CASES: &[crate::ssh_harness::Case] = &[
    ("unavailable_endpoint", unavailable_endpoint),
    ("inaccessible_remote", inaccessible_remote),
    ("disconnect_before_auth", disconnect_before_auth),
    ("disconnect_after_auth", disconnect_after_auth),
    ("disconnect_fetch", disconnect_fetch),
    ("disconnect_after_receive", disconnect_after_receive),
    ("distinct_authority_pins", distinct_authority_pins),
];
pub struct Preservation {
    repository: Vec<(std::path::PathBuf, Vec<u8>)>,
    private: Vec<u8>,
    registrations: Vec<crate::repository::keys::SharedKeyRegistration>,
}
impl Preservation {
    pub fn capture(case: &Case) -> Result<Self, FixtureError> {
        Ok(Self {
            repository: snapshot(&case.root)?,
            private: fixed(std::fs::read(case.fixture.client_key_path()))?,
            registrations: fixed(case.service.list_shared_keys())?,
        })
    }
    pub fn check(&self, case: &Case) -> Result<(), FixtureError> {
        assert!(
            self.repository == snapshot(&case.root)?,
            "repository bytes changed before transfer"
        );
        self.key_state(case)
    }
    pub fn key_state(&self, case: &Case) -> Result<(), FixtureError> {
        assert!(
            self.private == fixed(std::fs::read(case.fixture.client_key_path()))?,
            "selected private source changed"
        );
        assert!(
            self.registrations == fixed(case.service.list_shared_keys())?,
            "key registrations changed"
        );
        Ok(())
    }
}
pub fn seed(case: &Case) -> Result<(), FixtureError> {
    let repo = fixed(git2::Repository::open(&case.root))?;
    fixed(std::fs::write(case.root.join("tracked"), b"baseline\n"))?;
    let mut index = fixed(repo.index())?;
    fixed(index.add_path(std::path::Path::new("tracked")))?;
    fixed(index.write())?;
    let tree = fixed(repo.find_tree(fixed(index.write_tree())?))?;
    let signature = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    fixed(repo.commit(Some("HEAD"), &signature, &signature, "baseline", &tree, &[]))?;
    fixed(std::fs::write(
        case.root.join("tracked"),
        b"dirty worktree\n",
    ))?;
    fixed(std::fs::write(
        case.root.join(".git/FETCH_HEAD"),
        b"prior fetch evidence\n",
    ))?;
    fixed(std::fs::write(
        case.root.join(".manyhands/canonical"),
        b"canonical evidence\n",
    ))?;
    Ok(())
}
fn transport_failure(kind: &SshTransportErrorKind) {
    assert!(
        matches!(
            kind,
            SshTransportErrorKind::TransportUnavailable
                | SshTransportErrorKind::RemoteUnavailable
                | SshTransportErrorKind::ProtocolFailure
        ),
        "disconnect must retain a conservative transport category"
    );
}
fn unavailable_endpoint() -> Result<(), FixtureError> {
    let mut case = Case::new(false)?;
    seed(&case)?;
    case.fixture.shutdown()?;
    let before = Preservation::capture(&case)?;
    let (mut session, requests) = session(vec![]);
    transport_failure(&case.verify(&mut session).unwrap_err().kind);
    assert!(
        requests.borrow().is_empty(),
        "unavailable endpoint must not prompt"
    );
    before.check(&case)
}
fn inaccessible_remote() -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    seed(&case)?;
    let repo = fixed(git2::Repository::open(&case.root))?;
    fixed(repo.remote_set_url(
        "origin",
        &case.fixture.url().replace("fixture.git", "missing.git"),
    ))?;
    let before = Preservation::capture(&case)?;
    let (mut session, requests) =
        session(vec![crate::repository::keys::PassphraseResponse::Cancelled]);
    let error = case.verify(&mut session).unwrap_err();
    if requests.borrow().is_empty() {
        transport_failure(&error.kind);
    } else {
        assert!(
            error.kind == SshTransportErrorKind::UnlockCancelled,
            "ambiguous exec denial may prompt once"
        );
        assert!(
            requests.borrow().len() == 1,
            "ambiguous remote failure must bound its prompt"
        );
    }
    assert!(
        case.fixture.helper_invocations() == 0,
        "inaccessible repository must not execute a helper"
    );
    before.check(&case)
}
fn disconnect_before_auth() -> Result<(), FixtureError> {
    disconnect(FixtureBoundary::Handshake)
}
fn disconnect_after_auth() -> Result<(), FixtureError> {
    disconnect(FixtureBoundary::Advertisement)
}
fn disconnect(boundary: FixtureBoundary) -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    seed(&case)?;
    case.fixture.disconnect_at(boundary);
    let before = Preservation::capture(&case)?;
    let (mut session, requests) = session(vec![]);
    transport_failure(&case.verify(&mut session).unwrap_err().kind);
    assert!(
        requests.borrow().is_empty(),
        "reliable disconnect must not prompt"
    );
    if boundary == FixtureBoundary::Advertisement {
        assert!(
            case.fixture.accepted_keys().len() == 1,
            "disconnect must follow authentication"
        );
    }
    before.check(&case)
}
fn disconnect_fetch() -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    seed(&case)?;
    let before = Preservation::capture(&case)?;
    let (mut session, _) = session(vec![]);
    let called = Cell::new(0);
    let error = transfer(
        &case.service,
        case.request(),
        &mut session,
        Transfer::Download,
        &called,
        || case.fixture.disconnect_at(FixtureBoundary::Transfer),
    )
    .unwrap_err();
    transport_failure(&error.kind);
    assert!(called.get() == 1, "fetch must enter the scoped operation");
    // This fixture disconnects at the first transfer output, before any pack bytes.
    before.check(&case)
}
fn disconnect_after_receive() -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    seed(&case)?;
    let oid = crate::transfer::commit(&case)?;
    let before = Preservation::capture(&case)?;
    let (mut session, _) = session(vec![]);
    let mut request = case.request();
    request.direction = SshDirection::Push;
    let called = Cell::new(0);
    let (_guard, phases) = diagnostic_phases();
    let outcome = transfer(
        &case.service,
        request,
        &mut session,
        Transfer::Push,
        &called,
        || {
            case.fixture
                .disconnect_at(FixtureBoundary::AfterReceivePack)
        },
    );
    let status_withheld = case.fixture.receive_status_withheld();
    crate::ssh_harness::observation(&[506, u128::from(outcome.is_ok())]);
    if !status_withheld {
        observe_transport_failure(&case, &outcome, called.get(), &phases);
        // Fixed categories only: never expose a backend diagnostic, path or OID.
        let category = outcome_category(&outcome);
        let remote_matches = git2::Repository::open_bare(case.fixture.repository_path())
            .and_then(|server| server.refname_to_id("refs/heads/pushed"))
            .is_ok_and(|remote| remote == oid);
        crate::ssh_harness::observation(&[
            507,
            category,
            called.get() as u128,
            case.fixture.helper_invocations() as u128,
            case.fixture.accepted_keys().len() as u128,
            case.fixture.active_helpers() as u128,
            case.fixture.completed_helpers() as u128,
            u128::from(remote_matches),
        ]);
    }
    assert!(
        status_withheld,
        "fixture must withhold the actual receive-pack status"
    );
    if let Err(error) = outcome {
        transport_failure(&error.kind);
    }
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    assert!(
        fixed(server.refname_to_id("refs/heads/pushed"))? == oid,
        "receive-pack must have updated the remote despite the lost result"
    );
    before.check(&case)?;
    // Observed remote success with local failure is deliberately not retried.
    Ok(())
}
fn distinct_authority_pins() -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    seed(&case)?;
    let push = SshRemoteFixture::start()?;
    push.allow_client_public_key(
        fixed(russh::keys::load_secret_key(
            case.fixture.client_key_path(),
            None,
        ))?
        .public_key()
        .clone(),
    );
    let repo = fixed(git2::Repository::open(&case.root))?;
    fixed(repo.remote_set_pushurl("origin", Some(&push.url())))?;
    let before = Preservation::capture(&case)?;
    let (mut session, requests) = session(vec![]);
    fixed(case.verify(&mut session))?;
    let mut request = case.request();
    request.direction = SshDirection::Push;
    request.approval = None;
    let error = case
        .service
        .verify_ssh_transport(request.clone(), &mut session)
        .unwrap_err();
    assert!(
        matches!(
            error.kind,
            SshTransportErrorKind::HostApprovalRequired { .. }
        ),
        "fetch pin must not authorize a different push port"
    );
    request.approval = Some(HostApproval {
        authority: SshAuthority {
            host: "127.0.0.1".into(),
            port: push.address().port(),
        },
        expected: None,
        presented: push.host_identity(),
    });
    fixed(
        case.service
            .verify_ssh_transport(request.clone(), &mut session),
    )?;
    push.rotate_host_key()?;
    request.approval = None;
    assert!(
        matches!(
            case.service
                .verify_ssh_transport(request, &mut session)
                .unwrap_err()
                .kind,
            SshTransportErrorKind::HostReplacementRequired { .. }
        ),
        "push authority must retain its own pin"
    );
    fixed(case.verify(&mut session))?;
    assert!(
        requests.borrow().is_empty(),
        "host policy must not prompt for credentials"
    );
    before.check(&case)
}
