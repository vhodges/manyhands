use rusqlite::params;
use std::path::Path;
use time::OffsetDateTime;

use super::{RemoteOutcomeCategory, RemotePollInvocation, RemotePollingConfiguration, state};
use crate::repository::{RepositoryError, RepositoryService};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PollIneligibility {
    Disabled,
    Paused,
    RecoverySuspended,
    NotDue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutomaticPollEligibility {
    Eligible,
    NotEligible {
        reason: PollIneligibility,
        next_eligible_at: Option<OffsetDateTime>,
    },
}

pub(super) fn eligibility(
    policy: &RemotePollingConfiguration,
    next: Option<OffsetDateTime>,
    now: OffsetDateTime,
) -> AutomaticPollEligibility {
    let reason = if !policy.enabled() {
        Some(PollIneligibility::Disabled)
    } else if policy.paused() {
        Some(PollIneligibility::Paused)
    } else if policy.recovery_suspended() {
        Some(PollIneligibility::RecoverySuspended)
    } else {
        None
    };
    if let Some(reason) = reason {
        return AutomaticPollEligibility::NotEligible {
            reason,
            next_eligible_at: next,
        };
    }
    match next {
        Some(next) if next > now => {
            let bound = now.saturating_add(time::Duration::seconds(
                policy.interval().as_secs().max(900) as i64,
            ));
            AutomaticPollEligibility::NotEligible {
                reason: PollIneligibility::NotDue,
                next_eligible_at: Some(next.min(bound)),
            }
        }
        _ => AutomaticPollEligibility::Eligible,
    }
}

/// `None` means keep the stored deadline, not clear it. Busy, yield, interruption,
/// conflict-pending and not-eligible have no outcome category and do not call this.
/// The policy is the pre-finish policy; reservation finish still owns backoff writes.
#[allow(dead_code)] // Consumed by reservation finish in Cycle 08 Task 2.
pub(super) fn next_eligible(
    category: RemoteOutcomeCategory,
    invocation: RemotePollInvocation,
    completed_at: OffsetDateTime,
    policy: &RemotePollingConfiguration,
) -> Option<OffsetDateTime> {
    let seconds = match category {
        RemoteOutcomeCategory::Cancelled => return None,
        RemoteOutcomeCategory::Completed => policy.interval().as_secs(),
        _ if invocation == RemotePollInvocation::Explicit => return None,
        RemoteOutcomeCategory::TransportUnavailable | RemoteOutcomeCategory::ProtocolRejected => {
            policy
                .automatic_backoff()
                .map_or(60, |backoff| backoff.as_secs().saturating_mul(2).min(900))
        }
        RemoteOutcomeCategory::SelectedKeyUnavailable
        | RemoteOutcomeCategory::UnlockRequired
        | RemoteOutcomeCategory::HostApprovalRequired
        | RemoteOutcomeCategory::ConfigurationRequired
        | RemoteOutcomeCategory::RepositoryUnavailable => policy.interval().as_secs(),
    };
    Some(completed_at.saturating_add(time::Duration::seconds(seconds as i64)))
}

fn automatic_poll_eligibility_at(
    service: &RepositoryService,
    root: &Path,
    now: OffsetDateTime,
) -> Result<AutomaticPollEligibility, RepositoryError> {
    state::with_transaction(service, root, |tx, id| {
        let (policy, _, next) = state::select_polling_status(tx, id)
            .map_err(|_| state::recovery_required())?
            .ok_or_else(state::recovery_required)??;
        let result = eligibility(&policy, next, now);
        if let AutomaticPollEligibility::NotEligible {
            reason: PollIneligibility::NotDue,
            next_eligible_at: Some(clamped),
        } = result
            && Some(clamped) != next
        {
            tx.execute(
                "UPDATE remote_polling_state SET next_eligible_at=?2 WHERE repository_id=?1",
                params![id, clamped.unix_timestamp()],
            )
            .map_err(|_| state::recovery_required())?;
        }
        Ok(result)
    })
}

impl RepositoryService {
    /// Check automatic polling policy without contacting or configuring an
    /// endpoint or reserving an operation. Only a clock-rollback clamp is written.
    pub fn automatic_poll_eligibility(
        &self,
        root: &Path,
    ) -> Result<AutomaticPollEligibility, RepositoryError> {
        automatic_poll_eligibility_at(self, root, OffsetDateTime::now_utc())
    }
}

#[cfg(test)]
#[path = "poll_tests.rs"]
mod tests;
