//! What a needs-attention entry says and offers (D6 design C3): pure, from
//! the reason alone, so the Settings row and the operation area show the
//! same words and the same action.
//!
//! The words name a repository only by the names the caller passes; no text
//! here names an item, a comment or a key.

use manyhands::repository::transport::SshAuthority;

use super::{KeyId, Reason};

pub(crate) const UNLOCK: &str = "Unlock\u{2026}";
pub(crate) const CHOOSE_A_KEY: &str = "Choose a key\u{2026}";
pub(crate) const APPROVE_HOST: &str = "Approve host\u{2026}";
pub(crate) const OPEN_REMOTE_SETTINGS: &str = "Open remote settings\u{2026}";
pub(crate) const RESUME_CHECKING: &str = "Resume checking";

/// The action an entry offers (C3).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Action {
    /// Clears the key's declined mark and submits Check now, which prompts.
    Unlock,
    /// Opens the key setting; Check now follows the change.
    ChooseKey,
    /// Opens the host's approval; Check now follows the change.
    ApproveHost,
    /// Opens the remote setup; Check now follows the change.
    OpenRemoteSettings,
    /// Releases the recovery hold through the library (decision 10).
    ResumeChecking,
}

impl Action {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Action::Unlock => UNLOCK,
            Action::ChooseKey => CHOOSE_A_KEY,
            Action::ApproveHost => APPROVE_HOST,
            Action::OpenRemoteSettings => OPEN_REMOTE_SETTINGS,
            Action::ResumeChecking => RESUME_CHECKING,
        }
    }
}

/// The person's action for `reason`.
pub(crate) fn action_of(reason: &Reason) -> Action {
    match reason {
        Reason::KeyLocked(_) => Action::Unlock,
        Reason::KeyProblem(_) | Reason::NoKeySelected(_) => Action::ChooseKey,
        Reason::HostUnapproved(_) | Reason::HostUnidentified(_) => Action::ApproveHost,
        Reason::ConfigurationRequired(_) => Action::OpenRemoteSettings,
        Reason::RecoveryHeld(_) => Action::ResumeChecking,
    }
}

/// The key a locked-key entry is for.
pub(crate) fn locked_key(reason: &Reason) -> Option<KeyId> {
    match reason {
        Reason::KeyLocked(key) => Some(*key),
        _ => None,
    }
}

/// The host a host entry is for, as the library names it; none for a host
/// answer that named none.
pub(crate) fn ssh_authority(reason: &Reason) -> Option<SshAuthority> {
    match reason {
        Reason::HostUnapproved(authority) => Some(SshAuthority {
            host: authority.host.clone(),
            port: authority.port,
        }),
        _ => None,
    }
}

/// What an entry in the operation area says first.
pub(crate) fn headline(reason: &Reason) -> &'static str {
    match reason {
        Reason::KeyLocked(_) => "Checking is waiting for your key to be unlocked",
        Reason::KeyProblem(_) | Reason::NoKeySelected(_) => "Checking needs a key to be chosen",
        Reason::HostUnapproved(_) | Reason::HostUnidentified(_) => {
            "Checking needs a host to be approved"
        }
        Reason::ConfigurationRequired(_) => "Checking can't start until sharing is set up",
        Reason::RecoveryHeld(_) => super::status::HELD,
    }
}

/// "Holding back a, b": the repositories an entry waits for.
pub(crate) fn holding_back(names: &[String]) -> String {
    format!("Holding back {}", names.join(", "))
}
