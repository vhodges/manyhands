pub(super) mod endpoint;
mod error;

use std::path::PathBuf;

pub use error::{SshTransportError, SshTransportErrorKind};

use super::SharedKeyId;

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
