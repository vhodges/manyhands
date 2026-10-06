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
    Reconnected,
    TrackingDownloaded,
    BeforeTrackingWrite,
    TrackingWritten,
    /// Client attempt boundary, not receive-side transaction evidence.
    ExactPushStarted,
    /// Successful fresh receive-pack list on the independently resolved Push endpoint.
    PushAdvertisementObserved,
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
pub(crate) fn transfer<P: SessionCredentialProvider>(
    service: &RepositoryService,
    request: VerifySshTransportRequest,
    session: &mut SessionCredentials<P>,
    kind: Transfer,
    called: &std::cell::Cell<usize>,
    before_transfer: impl FnOnce(),
) -> Result<Vec<git2::Oid>, SshTransportError> {
    service.with_authenticated_remote(request, session, |remote| {
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
    })
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
