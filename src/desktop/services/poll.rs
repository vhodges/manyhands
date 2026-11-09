//! An automatic poll, as the worker runs it (D6 design B1, B4).
//!
//! The worker holds the process's only `SessionCredentials`, so it makes
//! the calls, each through a thin [`Backend`] method so that a recording
//! double can list them. The automatic path makes exactly these five, in
//! this order, and nothing else (B4; a poll cannot push, checkpoint, merge,
//! clean up or retire):
//!
//! 1. `resolve`: the repository, as registered;
//! 2. `polling_status`: its stored deadline, the fallback for an answer
//!    that names none;
//! 3. `poll_eligibility`: the policy's answer, which contacts nothing;
//! 4. `poll_automatic`: the poll;
//! 5. `remote_snapshot`: the deadline the poll stored, which a failure's
//!    backoff is.
//!
//! When the policy says no, the poll is not made (the library would answer
//! the same `NotEligible` without contacting the remote), and nothing was
//! stored to read back, so call 5 is skipped too.
//!
//! The answer is converted by `poll_convert`. A background poll never
//! prompts: `DesktopProvider` refuses for the running job's lane (B5).
//! `cancel_poll` is not here: only the cancel task (`preempt`) calls it, for
//! a user job, a reconfiguration and quit alike (B2).
//!
//! A stop asked of the worker is checked before `poll_automatic` (P-2): the
//! library's cancel can be lost on a row that is not active (Fact 30), so a
//! poll that was asked to stop never starts, whatever the cancel did.

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, PoisonError},
};

use manyhands::{
    repository::{
        AutomaticPollEligibility, OperationId, PollPublicationRemoteRequest, PollingStatusDto,
        RemotePollInvocation, RepositoryError, RepositoryErrorKind, keys::SessionCredentials,
    },
    results::ResultCode,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use super::{
    Backend, Services,
    poll_convert::{Library, attempt_of, poll_attempt, repository_failed},
    prompts::DesktopProvider,
};
use crate::desktop::model::polling::{Answer, CancelCause, PollAttempt, Root};

/// The automatic polls that ended, as the drain heard them and in order
/// (D6 B1): the scheduler (Task 5) takes them and applies them to its
/// schedule. A poll is not a request, so the request model never sees one.
#[derive(Clone, Default)]
pub(crate) struct PollInbox(Arc<Mutex<Vec<(Root, PollAttempt)>>>);

impl PollInbox {
    pub(super) fn push(&self, root: Root, attempt: PollAttempt) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((root, attempt));
    }

    /// Every attempt heard since the last call, oldest first.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 5.
    pub(crate) fn take(&self) -> Vec<(Root, PollAttempt)> {
        std::mem::take(&mut *self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

impl Services {
    /// The automatic polls that ended since the last call.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 5.
    pub(crate) fn polls(&self) -> &PollInbox {
        &self.polls
    }
}

/// Runs the automatic poll of the repository at `root` under `operation`
/// (`restart` resends it) and gives the desktop's account of it.
///
/// `asked` says whether the worker was asked to stop this poll, and why: a
/// user job, a reconfiguration or quit. It is read before `poll_automatic`
/// (P-2), where a stop asked means the poll is not made and answers
/// `Cancelled`; and for an answer of `Cancelled`, whose cause the library's
/// answer does not give (R-3). Nobody having asked, a `Cancelled` answer is a
/// launch takeover's stored cancel.
pub(super) fn run(
    backend: &dyn Backend,
    session: &mut SessionCredentials<DesktopProvider>,
    root: &Root,
    operation: OperationId,
    restart: bool,
    asked: impl Fn() -> Option<CancelCause>,
) -> PollAttempt {
    let path = Path::new(root);
    // A call that fails before the poll reserved anything stores no row:
    // it is a repository failure to wait out (R-35), with no operation.
    let refused = |recovery_required: bool| repository_failed(root, operation, recovery_required);
    let repo = match backend.resolve(path) {
        Ok(repo) => repo,
        Err(error) => return refused(error.code() == ResultCode::RecoveryRequired),
    };
    let stored = match backend.polling_status(&repo) {
        Ok(status) => stored_deadline(&status),
        Err(error) => return refused(error.code() == ResultCode::RecoveryRequired),
    };
    match backend.poll_eligibility(path) {
        Ok(AutomaticPollEligibility::Eligible) => {}
        Ok(AutomaticPollEligibility::NotEligible {
            reason,
            next_eligible_at,
        }) => {
            let library = Library::NotEligible {
                reason,
                next_eligible_at,
                index_pending: false,
            };
            let mut attempt = attempt_of(root, operation, library, || CancelCause::Preemption);
            attempt.next_eligible_at = attempt.next_eligible_at.or(stored);
            return attempt;
        }
        Err(error) => return refused(is_recovery_required(&error)),
    }
    // The check before every `poll_automatic` (P-2).
    if let Some(cause) = asked() {
        return PollAttempt {
            root: root.clone(),
            operation,
            answer: Answer::Cancelled(cause),
            completed_at: None,
            next_eligible_at: stored,
            targets: Vec::new(),
            index_pending: false,
        };
    }
    let request = PollPublicationRemoteRequest {
        root: PathBuf::from(root),
        operation_id: operation,
        invocation: RemotePollInvocation::Automatic,
        approval: None,
        restart,
    };
    let result = backend.poll_automatic(request, session);
    let cancel = || asked().unwrap_or(CancelCause::Takeover);
    let mut attempt = match backend.after_poll(operation) {
        Some(library) => attempt_of(root, operation, library, cancel),
        None => poll_attempt(root, operation, &result, cancel),
    };
    // The deadline the poll stored: a failure's backoff, or the interval
    // after a completion. A snapshot that cannot be read leaves the one
    // read before the poll.
    let after = backend
        .remote_snapshot(path)
        .ok()
        .and_then(|snapshot| snapshot.next_eligible_at());
    attempt.next_eligible_at = attempt.next_eligible_at.or(after).or(stored);
    attempt
}

fn is_recovery_required(error: &RepositoryError) -> bool {
    error.kind == RepositoryErrorKind::RecoveryRequired
}

/// The deadline stored before the poll, when there is one.
fn stored_deadline(status: &PollingStatusDto) -> Option<OffsetDateTime> {
    status
        .next_eligible_at
        .as_deref()
        .and_then(|stored| OffsetDateTime::parse(stored, &Rfc3339).ok())
}

#[cfg(test)]
#[path = "poll_tests.rs"]
mod tests;
