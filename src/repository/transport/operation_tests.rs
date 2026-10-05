//! Private source-inclusion dispatcher for the pre-thread initialized host.
#![allow(dead_code)]
use super::*;
use crate::repository::{RepositoryService, keys::*};
use std::cell::RefCell;

/// Exercise the parser-produced connection spelling against an ephemeral port.
/// Substitute only authority; never feed the backend-only SCP port syntax back
/// into the production parser. Path bytes and URL versus SCP form stay intact.
pub(crate) fn endpoint_connection_at(
    configured: &str,
    address: std::net::SocketAddr,
) -> Result<String, SshTransportErrorKind> {
    let endpoint = endpoint::parse_ssh_endpoint(configured)?;
    let username = endpoint.username.as_deref().unwrap();
    if let Some(rest) = endpoint.connection_url.strip_prefix("ssh://") {
        let (_, path) = rest.split_once('/').unwrap();
        Ok(format!("ssh://{username}@{address}/{path}"))
    } else {
        let separator = if endpoint.authority.host.contains(':') {
            "]:"
        } else {
            ":"
        };
        let (_, path) = endpoint.connection_url.split_once(separator).unwrap();
        Ok(format!("[{username}@{address}]:{path}"))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Checkpoint {
    Prepared,
    ProviderReturned,
    Authenticated,
}
type Hook = Box<dyn FnMut(Checkpoint)>;
thread_local! { static HOOK: RefCell<Option<Hook>> = RefCell::new(None); }
pub(crate) fn checkpoint(point: Checkpoint) {
    HOOK.with(|hook| {
        if let Some(hook) = hook.borrow_mut().as_mut() {
            hook(point);
        }
    });
}
pub(crate) struct HookGuard;
impl Drop for HookGuard {
    fn drop(&mut self) {
        HOOK.with(|hook| *hook.borrow_mut() = None);
    }
}
pub(crate) fn install_hook(hook: impl FnMut(Checkpoint) + 'static) -> HookGuard {
    HOOK.with(|slot| {
        assert!(slot.borrow().is_none());
        *slot.borrow_mut() = Some(Box::new(hook));
    });
    HookGuard
}
pub(crate) enum Transfer {
    Advertisement,
    Download,
    Push,
    RejectedPush,
}
pub(crate) fn outcome_category<T>(outcome: &Result<T, SshTransportError>) -> u128 {
    match outcome.as_ref().map_err(|error| &error.kind) {
        Ok(_) => 0,
        Err(SshTransportErrorKind::TransportUnavailable) => 1,
        Err(SshTransportErrorKind::RemoteUnavailable) => 2,
        Err(SshTransportErrorKind::ProtocolFailure) => 3,
        Err(SshTransportErrorKind::KeyRejected) => 4,
        Err(SshTransportErrorKind::HostTrustChanged) => 5,
        Err(SshTransportErrorKind::KeySourceChanged) => 6,
        Err(SshTransportErrorKind::SelectionChanged) => 7,
        Err(SshTransportErrorKind::EndpointChanged) => 8,
        // 9 was the original catch-all; preserve all established category codes.
        Err(SshTransportErrorKind::ConfigurationInvalid) => 10,
        Err(SshTransportErrorKind::PublicationRemoteMissing) => 11,
        Err(SshTransportErrorKind::UsernameRequired) => 12,
        Err(SshTransportErrorKind::NoSelectedKey) => 13,
        Err(SshTransportErrorKind::KeyMissing) => 14,
        Err(SshTransportErrorKind::KeyUnreadable) => 15,
        Err(SshTransportErrorKind::KeyInvalidOrUnsupported) => 16,
        Err(SshTransportErrorKind::UnlockCancelled) => 17,
        Err(SshTransportErrorKind::ProviderUnavailable) => 18,
        Err(SshTransportErrorKind::UnlockFailed) => 19,
        Err(SshTransportErrorKind::HostApprovalRequired { .. }) => 20,
        Err(SshTransportErrorKind::HostReplacementRequired { .. }) => 21,
        Err(SshTransportErrorKind::HostVerificationUnavailable) => 22,
        Err(SshTransportErrorKind::RegistryUnavailable) => 23,
        Err(SshTransportErrorKind::RuntimeUninitialized) => 24,
        Err(SshTransportErrorKind::PushRejected) => 25,
    }
}
pub(super) fn observe_backend_failure(
    attempt: &super::callbacks::CallbackAttempt,
    error: &git2::Error,
    supplied: bool,
) {
    // Enum values and flags only; never print the backend's message or identity.
    println!(
        "SSH_OBSERVATION 511 {} {} {} {} {} {} {}",
        error.code() as u128,
        error.class() as u128,
        attempt.key_submissions(),
        u8::from(attempt.observed_host().is_some()),
        u8::from(attempt.failure().is_some()),
        u8::from(attempt.passthrough()),
        u8::from(supplied),
    );
}
pub(crate) fn transfer<P: SessionCredentialProvider>(
    service: &RepositoryService,
    request: VerifySshTransportRequest,
    session: &mut SessionCredentials<P>,
    kind: Transfer,
    called: &std::cell::Cell<usize>,
    before_transfer: impl FnOnce(),
) -> Result<Vec<git2::Oid>, SshTransportError> {
    let outcome = service.with_authenticated_remote(request, session, |remote| {
        called.set(called.get() + 1);
        let advertised = remote
            .advertisement()?
            .into_iter()
            .map(|(_, oid)| oid)
            .collect();
        before_transfer();
        match kind {
            Transfer::Advertisement => {}
            Transfer::Download => remote.download(&["refs/heads/main:refs/remotes/origin/main"])?,
            Transfer::Push => remote.push(&["refs/heads/pushed:refs/heads/pushed"])?,
            Transfer::RejectedPush => remote.push(&["+refs/heads/pushed:refs/heads/main"])?,
        }
        Ok(advertised)
    });
    // This dispatcher is cfg(test) only. Keep every returned outcome observable
    // before a case-specific assertion, without printing any error data.
    println!(
        "SSH_OBSERVATION 510 {} {}",
        outcome_category(&outcome),
        called.get()
    );
    outcome
}
pub(crate) fn install_pin(service: &RepositoryService, approval: &HostApproval) {
    let snapshot = service.read_host_trust(&approval.authority).unwrap();
    service
        .finalize_host_trust(
            &approval.authority,
            &snapshot,
            &approval.presented,
            Some(approval),
        )
        .unwrap();
}

#[test]
fn transport_requires_explicit_runtime_startup() {
    struct Provider;
    impl SessionCredentialProvider for Provider {
        fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
            panic!("must not prompt")
        }
    }
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let error = service
        .verify_ssh_transport(
            VerifySshTransportRequest {
                root: data.path().into(),
                direction: SshDirection::Fetch,
                approval: None,
            },
            &mut SessionCredentials::new(Provider),
        )
        .unwrap_err();
    assert_eq!(error.kind, SshTransportErrorKind::RuntimeUninitialized);
}
