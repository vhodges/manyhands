//! The setup channel as the setup operations use it: one method per channel
//! operation, each opening a connection of its own in the direction the
//! operation needs (design Part C and Part D). Tests stub the trait; the
//! production implementation wraps the transport's `with_setup_remote` and
//! its four methods.

use std::path::PathBuf;

use git2::Oid;

use crate::repository::{
    RepositoryService,
    keys::{SessionCredentialProvider, SessionCredentials},
    transport::{
        SetupAdvertisement, SetupPush, SetupScope, SshDirection, SshTransportError,
        VerifySshTransportRequest,
    },
};

/// What a setup operation asks of the remote. Every method is one
/// connection to the setup's remote and can reach only the setup's branch
/// (and read the primary), whatever implements it.
pub(crate) trait SetupChannel {
    /// A fetch connection: fetches the remote's primary into the setup's
    /// private base ref and answers what the advertisement held of the
    /// primary and of the setup's branch.
    fn fetch_base(&mut self, scope: &SetupScope) -> Result<SetupAdvertisement, SshTransportError>;

    /// A push connection: pushes the setup's branch, as the one update from
    /// no ref to `commit`. A `PushRejected`, `TransportUnavailable` or
    /// `RemoteUnavailable` may follow a push that landed, so the caller
    /// observes after it whatever it answered.
    fn push_branch(
        &mut self,
        scope: &SetupScope,
        commit: Oid,
    ) -> Result<SetupPush, SshTransportError>;

    /// A fetch connection of its own: the setup's branch as a fresh
    /// advertisement has it, the only evidence of what a push left.
    fn observe_branch(&mut self, scope: &SetupScope) -> Result<Option<Oid>, SshTransportError>;
}

/// The channel over the real transport, with a call's credentials.
pub(crate) struct ServiceChannel<'a, P> {
    service: &'a RepositoryService,
    root: PathBuf,
    credentials: &'a mut SessionCredentials<P>,
}

impl<'a, P> ServiceChannel<'a, P> {
    pub(crate) fn new(
        service: &'a RepositoryService,
        root: PathBuf,
        credentials: &'a mut SessionCredentials<P>,
    ) -> Self {
        Self {
            service,
            root,
            credentials,
        }
    }

    fn request(&self, direction: SshDirection) -> VerifySshTransportRequest {
        VerifySshTransportRequest {
            root: self.root.clone(),
            direction,
            approval: None,
        }
    }
}

impl<P: SessionCredentialProvider> SetupChannel for ServiceChannel<'_, P> {
    fn fetch_base(&mut self, scope: &SetupScope) -> Result<SetupAdvertisement, SshTransportError> {
        let request = self.request(SshDirection::Fetch);
        self.service
            .with_setup_remote(request, scope, false, self.credentials, |remote| {
                remote.fetch_setup_base()
            })
    }

    fn push_branch(
        &mut self,
        scope: &SetupScope,
        commit: Oid,
    ) -> Result<SetupPush, SshTransportError> {
        let request = self.request(SshDirection::Push);
        self.service
            .with_setup_remote(request, scope, false, self.credentials, |remote| {
                remote.push_setup_branch(commit)
            })
    }

    fn observe_branch(&mut self, scope: &SetupScope) -> Result<Option<Oid>, SshTransportError> {
        let request = self.request(SshDirection::Fetch);
        self.service
            .with_setup_remote(request, scope, false, self.credentials, |remote| {
                remote.observe_setup_branch()
            })
    }
}
