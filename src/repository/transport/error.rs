use std::{fmt, path::PathBuf};

use super::{HostKeyIdentity, SshAuthority, SshDirection};
use crate::repository::SharedKeyId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SshTransportError {
    pub root: PathBuf,
    pub remote_name: String,
    pub direction: SshDirection,
    pub selected_key_id: Option<SharedKeyId>,
    pub authority: Option<SshAuthority>,
    pub kind: SshTransportErrorKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SshTransportErrorKind {
    ConfigurationInvalid,
    PublicationRemoteMissing,
    UsernameRequired,
    NoSelectedKey,
    KeyMissing,
    KeyUnreadable,
    KeySourceChanged,
    SelectionChanged,
    EndpointChanged,
    KeyInvalidOrUnsupported,
    KeyRejected,
    UnlockCancelled,
    ProviderUnavailable,
    UnlockFailed,
    HostApprovalRequired {
        presented: HostKeyIdentity,
    },
    HostReplacementRequired {
        expected: HostKeyIdentity,
        presented: HostKeyIdentity,
    },
    HostTrustChanged,
    HostVerificationUnavailable,
    RegistryUnavailable,
    RuntimeUninitialized,
    TransportUnavailable,
    RemoteUnavailable,
    PushRejected,
    ProtocolFailure,
}

impl SshTransportError {
    pub fn guidance(&self) -> &'static str {
        match self.kind {
            SshTransportErrorKind::ConfigurationInvalid => {
                "configure an explicit, valid SSH publication destination"
            }
            SshTransportErrorKind::PublicationRemoteMissing => {
                "configure an available publication remote and retry"
            }
            SshTransportErrorKind::UsernameRequired => {
                "configure the SSH username in the publication remote URL"
            }
            SshTransportErrorKind::NoSelectedKey => "select a shared SSH key and retry",
            SshTransportErrorKind::KeyMissing => "restore the selected private key and retry",
            SshTransportErrorKind::KeyUnreadable => {
                "make the selected private key readable and retry"
            }
            SshTransportErrorKind::KeySourceChanged => {
                "review the changed private key source and retry"
            }
            SshTransportErrorKind::SelectionChanged => {
                "retry with the currently selected shared SSH key"
            }
            SshTransportErrorKind::EndpointChanged => {
                "review the changed publication endpoint and retry"
            }
            SshTransportErrorKind::KeyInvalidOrUnsupported => {
                "select a valid supported SSH private key and retry"
            }
            SshTransportErrorKind::KeyRejected => {
                "check the key format, passphrase, and server authorization, then retry"
            }
            SshTransportErrorKind::UnlockCancelled => "unlock was cancelled; retry when ready",
            SshTransportErrorKind::ProviderUnavailable => {
                "make the credential provider available and retry"
            }
            SshTransportErrorKind::UnlockFailed => {
                "check the passphrase or server key authorization and retry"
            }
            SshTransportErrorKind::HostApprovalRequired { .. } => {
                "review and approve the presented host key before retrying"
            }
            SshTransportErrorKind::HostReplacementRequired { .. } => {
                "review and approve the host key replacement before retrying"
            }
            SshTransportErrorKind::HostTrustChanged => {
                "review the changed host trust state and retry"
            }
            SshTransportErrorKind::HostVerificationUnavailable => {
                "make host verification available and retry"
            }
            SshTransportErrorKind::RegistryUnavailable => {
                "make the Manyhands registry available and retry"
            }
            SshTransportErrorKind::RuntimeUninitialized => {
                "initialize the SSH runtime before using transport"
            }
            SshTransportErrorKind::TransportUnavailable => {
                "check the SSH endpoint and network availability, then retry"
            }
            SshTransportErrorKind::RemoteUnavailable => {
                "check publication remote availability and retry"
            }
            SshTransportErrorKind::PushRejected => {
                "review remote push policy and retry with an accepted update"
            }
            SshTransportErrorKind::ProtocolFailure => {
                "retry the SSH operation; review endpoint compatibility if it persists"
            }
        }
    }
}

impl fmt::Display for SshTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            SshTransportErrorKind::ConfigurationInvalid => "SSH configuration is invalid",
            SshTransportErrorKind::PublicationRemoteMissing => "publication remote is missing",
            SshTransportErrorKind::UsernameRequired => "SSH username is required",
            SshTransportErrorKind::NoSelectedKey => "no shared SSH key is selected",
            SshTransportErrorKind::KeyMissing => "selected SSH key is missing",
            SshTransportErrorKind::KeyUnreadable => "selected SSH key is unreadable",
            SshTransportErrorKind::KeySourceChanged => "selected SSH key source changed",
            SshTransportErrorKind::SelectionChanged => "selected SSH key changed",
            SshTransportErrorKind::EndpointChanged => "SSH endpoint changed",
            SshTransportErrorKind::KeyInvalidOrUnsupported => {
                "selected SSH key is invalid or unsupported"
            }
            SshTransportErrorKind::KeyRejected => "selected SSH key was rejected",
            SshTransportErrorKind::UnlockCancelled => "SSH key unlock was cancelled",
            SshTransportErrorKind::ProviderUnavailable => "credential provider is unavailable",
            SshTransportErrorKind::UnlockFailed => "SSH key unlock failed",
            SshTransportErrorKind::HostApprovalRequired { .. } => "SSH host approval is required",
            SshTransportErrorKind::HostReplacementRequired { .. } => {
                "SSH host key replacement approval is required"
            }
            SshTransportErrorKind::HostTrustChanged => "SSH host trust changed",
            SshTransportErrorKind::HostVerificationUnavailable => {
                "SSH host verification is unavailable"
            }
            SshTransportErrorKind::RegistryUnavailable => "SSH key registry is unavailable",
            SshTransportErrorKind::RuntimeUninitialized => "SSH runtime is not initialized",
            SshTransportErrorKind::TransportUnavailable => "SSH transport is unavailable",
            SshTransportErrorKind::RemoteUnavailable => "SSH remote is unavailable",
            SshTransportErrorKind::PushRejected => "SSH push was rejected",
            SshTransportErrorKind::ProtocolFailure => "SSH protocol failed",
        })
    }
}

impl std::error::Error for SshTransportError {}

#[cfg(test)]
mod tests {
    use std::{error::Error, path::PathBuf};

    use super::*;
    use crate::repository::transport::SshDirection;

    #[test]
    fn formatting_and_source_expose_only_fixed_category_text() {
        let error = SshTransportError {
            root: PathBuf::from("/safe/root"),
            remote_name: "origin".to_owned(),
            direction: SshDirection::Fetch,
            selected_key_id: None,
            authority: None,
            kind: SshTransportErrorKind::TransportUnavailable,
        };

        assert_eq!(error.to_string(), "SSH transport is unavailable");
        assert_eq!(
            error.guidance(),
            "check the SSH endpoint and network availability, then retry"
        );
        assert!(error.source().is_none());
        assert!(!format!("{error:?}").contains("backend-secret"));
        assert!(!error.to_string().contains("backend-secret"));
    }
}
