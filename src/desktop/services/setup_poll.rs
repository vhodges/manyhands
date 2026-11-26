//! A setup published on another remote, as the scheduler and the worker see
//! it (Wave 06 Cycle 02, design Parts G and J).
//!
//! A repository whose setup is on a branch for a pull request is polled
//! through the library's `poll_setup`, with the automatic policy: for a
//! first-time setup instead of today's poll (the repository has no
//! configuration to poll by), for a change on another remote after today's
//! poll of the current remote. A change on the same remote keeps today's poll
//! alone: it fast-forwards the primary, and any poll finishes a complete
//! setup (design Part B).
//!
//! [`pending_setup_of`] reads which of the two a repository is on, from the
//! one `repo inspect` read; [`setup_attempt`] gives the desktop's account of
//! the answer. Nothing here holds a URL, a key or a passphrase.

use manyhands::repository::{
    OperationId, RepositoryErrorKind, RepositoryInspectionDto, SetupPollError, SetupPollOutcome,
    SetupPollReport,
};
use manyhands::results::ResultCode;

use super::poll_convert::{Library, attempt_of};
use crate::desktop::model::polling::{
    Answer, AttemptTarget, CancelCause, Disposition, PollAttempt, Root,
};

/// The setup state a poll waits on.
const PUBLISHED: &str = "published";
/// The kind of a setup that onboards a repository with no configuration.
const FIRST_TIME: &str = "enable";

/// Which poll a repository with a published setup takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SetupPollPath {
    /// A first-time setup: the setup poll instead of today's poll.
    Replace,
    /// A change on another remote: today's poll of the current remote, then
    /// the setup poll.
    After,
}

/// A published setup the desktop polls, and the remote it is on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PendingSetup {
    pub path: SetupPollPath,
    /// The setup's remote, by name.
    pub remote: String,
    /// Whether the repository has an SSH publication remote today's poll
    /// can check. A change on another remote is polled through the setup's
    /// remote alone when it has none (no publication remote yet, or one
    /// that is not SSH).
    pub current_ssh: bool,
}

impl PendingSetup {
    /// Whether today's poll of the current remote is made too: only for a
    /// change, and only where there is a remote to check.
    pub(crate) fn polls_current_remote(&self) -> bool {
        self.path == SetupPollPath::After && self.current_ssh
    }
}

/// The setup `inspection` shows, when the desktop polls it: one that is
/// published (a recorded one is not on the remote yet, so there is nothing
/// to poll), and for a change, one on a remote other than the current
/// publication remote.
pub(crate) fn pending_setup_of(inspection: &RepositoryInspectionDto) -> Option<PendingSetup> {
    let setup = inspection.setup.as_ref()?;
    if setup.state != PUBLISHED {
        return None;
    }
    let current_ssh = inspection
        .remotes
        .iter()
        .any(|remote| remote.selected_for_publication && remote.publication_eligible);
    let path = if setup.kind == FIRST_TIME {
        SetupPollPath::Replace
    } else {
        let current = inspection
            .remotes
            .iter()
            .find(|remote| remote.selected_for_publication);
        if current.is_some_and(|current| current.name == setup.remote) {
            return None;
        }
        SetupPollPath::After
    };
    Some(PendingSetup {
        path,
        remote: setup.remote.clone(),
        current_ssh,
    })
}

/// The desktop's account of a setup poll.
///
/// Every outcome the library answers is a completed check of the remote; an
/// activation also brought the primary in, so it reports main as
/// fast-forwarded and the repository is read again as after any poll that
/// brought something in. The others report no target, which changes nothing
/// the schedule shows of the repository's items. Its deadline is the one the
/// library stored, which the caller reads back.
pub(crate) fn setup_attempt(
    root: &Root,
    operation: OperationId,
    result: &Result<SetupPollReport, SetupPollError>,
    cancel: impl FnOnce() -> CancelCause,
) -> PollAttempt {
    let library = match result {
        Ok(report) => {
            let mut attempt = completed(root, operation);
            if report.outcome == SetupPollOutcome::Activated {
                attempt.targets.push(AttemptTarget {
                    item_id: None,
                    kind: None,
                    disposition: Disposition::FastForwarded,
                    main: true,
                });
            }
            return attempt;
        }
        // The setup ended between the choice of the poll and its lock: the
        // next poll takes the ordinary path.
        Err(SetupPollError::Gone) => return completed(root, operation),
        Err(SetupPollError::Cancelled) => {
            let mut attempt = completed(root, operation);
            attempt.answer = Answer::Cancelled(cancel());
            return attempt;
        }
        Err(SetupPollError::Busy) => Library::Busy,
        Err(SetupPollError::RecoveryRequired) => Library::Repository {
            recovery_required: true,
        },
        Err(SetupPollError::Transport(error)) => Library::Transport {
            kind: error.kind.clone(),
            key: error.selected_key_id,
            authority: error.authority.clone(),
        },
        Err(SetupPollError::Repository(error)) => Library::Repository {
            recovery_required: error.kind == RepositoryErrorKind::RecoveryRequired,
        },
        Err(SetupPollError::Registry(error)) => Library::Repository {
            recovery_required: error.code() == ResultCode::RecoveryRequired,
        },
    };
    attempt_of(root, operation, library, || CancelCause::Takeover)
}

/// A check that answered, with nothing to report of any item.
fn completed(root: &Root, operation: OperationId) -> PollAttempt {
    PollAttempt {
        root: root.clone(),
        operation,
        answer: Answer::Completed,
        completed_at: None,
        next_eligible_at: None,
        targets: Vec::new(),
        index_pending: false,
    }
}

#[cfg(test)]
#[path = "setup_poll_tests.rs"]
mod tests;
