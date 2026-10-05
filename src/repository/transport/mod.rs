// The public error contract retains owned context and both host identities for
// replacement approval. Operations are infrequent; keep that typed value API.
#![allow(clippy::result_large_err)]

mod callbacks;
pub(super) mod endpoint;
mod error;
mod operation;
#[cfg(test)]
pub(crate) mod operation_tests;
mod remote;
#[cfg(test)]
pub(crate) mod tests;
pub(super) mod trust;

use std::path::PathBuf;

pub use error::{SshTransportError, SshTransportErrorKind};

use super::SharedKeyId;

// Constructed by the scoped operation driver; no Git handles or secret storage.
pub(super) struct PreparedSshAttempt {
    pub(super) registration: super::keys::SharedKeyRegistration,
    pub(super) source: super::keys::KeySourceToken,
    pub(super) endpoint: endpoint::SshEndpoint,
    pub(super) trust: trust::HostTrustSnapshot,
    pub(super) approval: Option<HostApproval>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SshDirection {
    Fetch,
    Push,
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
