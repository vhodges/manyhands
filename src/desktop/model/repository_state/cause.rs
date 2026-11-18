//! Row 6's cause (Cycle 03a Part B; Wave 04 Cycle 03b design A7): which of
//! the causes of Needs attention holds, with the action and the words it
//! gives the state. The first cause that holds wins, in `attention_cause`'s
//! order.

use manyhands::repository::PollingOutcome;

use super::{
    NEEDS_ATTENTION, Phase, Reason, RepoFacts, RepoState, RowAction, Shape, StateKind, Tone,
    TransportFailure, Unplaced, Why, action_of, last_answer, look, problems_sentence, row_sentence,
    stored_outcome, unreadable_sentence,
};

/// Why a repository's state is Needs attention: one variant per cause of
/// `attention_cause`, in its order (Wave 04 Cycle 03b design A7; R-5).
/// Activity reads it with the state's kind and, for Locked, its action,
/// instead of reading the schedule again (R-40).
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum AttentionCause {
    /// The phase is `Attention(reason)`.
    Phase(Reason),
    /// Held by the policy, with no reason in the phase.
    Held,
    /// A pending conflict.
    Conflict,
    /// `Waiting { Failed }`.
    WaitingFailed,
    /// An unreadable registration.
    Unreadable,
    /// A `Transport(Unavailable | ConfigurationRequired)` answer.
    Transport,
    /// A stored outcome.
    Stored,
    /// The unshared list's problems.
    Problems,
}

/// Row 6.
pub(super) fn needs_attention(facts: &RepoFacts<'_>, repo: &str) -> Option<RepoState> {
    let Cause {
        cause,
        action,
        sentence,
    } = attention_cause(facts, repo)?;
    Some(RepoState {
        cause: Some(cause),
        ..RepoState::labelled(
            StateKind::NeedsAttention,
            look(NEEDS_ATTENTION, "circle-alert", Tone::Failure),
            action,
            sentence,
        )
    })
}

/// What `attention_cause` found: the cause, and the action and words it
/// gives the state.
struct Cause {
    cause: AttentionCause,
    action: Option<RowAction>,
    sentence: Vec<String>,
}

/// What holds row 6, the first cause that does: its action and its words.
/// A pending conflict and a check's problems have no action (Activity has
/// them).
fn attention_cause(facts: &RepoFacts<'_>, repo: &str) -> Option<Cause> {
    use PollingOutcome::*;
    let schedule = facts.schedule;
    let attend = |reason: Reason| Some(RowAction::Attend(action_of(&reason), reason));
    let found = |cause, action, sentence| {
        Some(Cause {
            cause,
            action,
            sentence,
        })
    };
    if let Some(Phase::Attention(reason)) = schedule.map(|schedule| &schedule.phase) {
        return found(
            AttentionCause::Phase(reason.clone()),
            attend(reason.clone()),
            row_sentence(facts),
        );
    }
    if facts.row().shape == Shape::Held {
        // Held by the policy, with no reason in the phase: Resume checking
        // releases the hold for this repository.
        let held = Reason::RecoveryHeld(facts.entry.key().clone());
        return found(AttentionCause::Held, attend(held), row_sentence(facts));
    }
    if schedule.is_some_and(|schedule| schedule.conflict_pending) {
        return found(AttentionCause::Conflict, None, row_sentence(facts));
    }
    let failed_wait = matches!(
        schedule.map(|schedule| &schedule.phase),
        Some(Phase::Waiting {
            why: Why::Failed,
            ..
        })
    );
    if failed_wait {
        return found(
            AttentionCause::WaitingFailed,
            Some(RowAction::RetryNow),
            row_sentence(facts),
        );
    }
    if facts.unplaced == Some(Unplaced::Unreadable) {
        return found(
            AttentionCause::Unreadable,
            Some(RowAction::Reread),
            vec![unreadable_sentence(repo)],
        );
    }
    if matches!(
        last_answer(facts),
        Some(TransportFailure::Unavailable | TransportFailure::ConfigurationRequired)
    ) {
        return found(
            AttentionCause::Transport,
            Some(RowAction::RetryNow),
            row_sentence(facts),
        );
    }
    let stored = match stored_outcome(facts) {
        Some(SelectedKeyUnavailable | HostApprovalRequired) => Some(RowAction::CheckNow),
        Some(ConfigurationRequired | RepositoryUnavailable) => Some(RowAction::RetryNow),
        _ => None,
    };
    if stored.is_some() {
        return found(AttentionCause::Stored, stored, row_sentence(facts));
    }
    let problems = schedule?.unshared.problems.len();
    if problems > 0 {
        return found(
            AttentionCause::Problems,
            None,
            vec![problems_sentence(problems)],
        );
    }
    None
}
