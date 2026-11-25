// The public error contract retains owned context and both host identities for
// replacement approval. Operations are infrequent; keep that typed value API.
#![allow(clippy::result_large_err)]

mod agent;
mod agent_credential;
mod callbacks;
pub(super) mod endpoint;
mod error;
#[cfg(all(test, unix))]
pub(crate) mod fake_agent;
mod operation;
#[cfg(test)]
pub(crate) mod operation_tests;
mod remote;
pub(super) use remote::AuthenticatedSshRemote;
pub(crate) use remote::{DeleteAttempt, DeleteObservation};
#[allow(unused_imports)]
pub(crate) use remote::{SetupAdvertisement, SetupPush};
#[cfg(test)]
pub(crate) mod tests;
pub(super) mod trust;

use std::path::PathBuf;

pub use error::{SshTransportError, SshTransportErrorKind};

use super::{SharedKeyId, setup::record::SetupRecord};
use crate::canonical::ItemId;

/// Process-only action expectation. No Debug/serialization or secret material.
pub(super) struct SshScopeExpectation {
    pub(super) endpoint: endpoint::SshEndpoint,
    selected: Option<(SharedKeyId, PathBuf, Option<super::keys::KeySourceToken>)>,
}
impl SshScopeExpectation {
    pub(super) fn new(
        endpoint: endpoint::SshEndpoint,
        selected: Option<(SharedKeyId, PathBuf, Option<super::keys::KeySourceToken>)>,
    ) -> Self {
        Self { endpoint, selected }
    }
    pub(super) fn check(&self, prepared: &PreparedSshAttempt) -> Result<(), SshTransportErrorKind> {
        if self.endpoint != prepared.endpoint {
            return Err(SshTransportErrorKind::EndpointChanged);
        }
        let Some((id, path, source)) = &self.selected else {
            return Err(SshTransportErrorKind::SelectionChanged);
        };
        if *id != prepared.registration.id {
            return Err(SshTransportErrorKind::SelectionChanged);
        }
        if path != &prepared.registration.private_key_path
            || source.as_ref() != Some(&prepared.source)
        {
            return Err(SshTransportErrorKind::KeySourceChanged);
        }
        Ok(())
    }
}

/// Where a call's remote comes from: the repository's configuration file, as
/// every call has taken it, or a pending setup's record, for the setup
/// channel alone. `Setup` reaches only the four setup methods of
/// `AuthenticatedSshRemote`; no item method accepts it.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum RemoteSource {
    Configured,
    Setup(SetupScope),
}

/// What a setup call is bound to: the record's identity, its remote and
/// primary, and the digest of the remote's endpoints when it was recorded.
/// Every recheck of the call reads the record and the endpoints again and
/// refuses a change as `EndpointChanged`.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct SetupScope {
    pub setup_id: ItemId,
    pub remote_name: String,
    pub primary_branch: String,
    pub endpoint_digest: [u8; 32],
}

impl SetupScope {
    /// The scope of `record`.
    pub(crate) fn of(record: &SetupRecord) -> Self {
        Self {
            setup_id: record.setup_id.clone(),
            remote_name: record.remote_name.clone(),
            primary_branch: record.primary_branch.clone(),
            endpoint_digest: record.endpoint_digest,
        }
    }

    /// The setup's branch on the remote: the one ref a setup call can push,
    /// delete or observe.
    pub(crate) fn branch_ref(&self) -> String {
        format!("refs/heads/manyhands/setup/{}", self.setup_id)
    }

    /// The private ref the remote's primary is fetched into. It is no
    /// remote-tracking ref, so a poll never reads or writes it.
    pub(crate) fn base_ref(&self) -> String {
        format!("refs/manyhands/setups/{}/base", self.setup_id)
    }
}

impl std::fmt::Debug for SetupScope {
    /// Leaves out the digest, as the record's does.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SetupScope")
            .field("setup_id", &self.setup_id)
            .field("remote_name", &self.remote_name)
            .field("primary_branch", &self.primary_branch)
            .finish_non_exhaustive()
    }
}

// Constructed by the scoped operation driver; no Git handles or secret storage.
pub(super) struct PreparedSshAttempt {
    pub(super) registration: super::keys::SharedKeyRegistration,
    pub(super) source: super::keys::KeySourceToken,
    pub(super) endpoint: endpoint::SshEndpoint,
    pub(super) trust: trust::HostTrustSnapshot,
    pub(super) approval: Option<HostApproval>,
    /// The registry connection the call's key reads go through, its
    /// preparation's and every recheck's (design decision 36). It holds no
    /// guard, no read transaction and nothing read.
    pub(super) registry: super::CallRegistry,
    /// Where the remote's name came from, and so what each recheck holds it
    /// to.
    pub(super) remote_source: RemoteSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SshDirection {
    Fetch,
    Push,
    Delete,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SshAuthority {
    pub host: String,
    pub port: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostKeyIdentity {
    pub algorithm: String,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostApproval {
    pub authority: SshAuthority,
    pub expected: Option<HostKeyIdentity>,
    pub presented: HostKeyIdentity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifySshTransportRequest {
    pub root: PathBuf,
    pub direction: SshDirection,
    pub approval: Option<HostApproval>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SshTransportVerified {
    pub root: PathBuf,
    pub remote_name: String,
    pub selected_key_id: SharedKeyId,
    pub direction: SshDirection,
    pub authority: SshAuthority,
    pub host_key: HostKeyIdentity,
}
