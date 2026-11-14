//! A background or explicit retirement's answer, classified (D6 design E3,
//! A20): pure, so the table is tested without a worker.
//!
//! The desktop reads the worker's record state first (`Finished` or
//! `Accepted`), then the answer's state, then its code. The library decides
//! everything about safety; this only sorts what it said into what the
//! person sees and what the loop does next.
//!
//! Nothing here holds a title: an item is its ID.

use std::collections::{BTreeMap, BTreeSet};

use manyhands::{
    canonical::ItemId,
    repository::{ContextRetirementState, MutationDataDto, RetirementRemoval},
    results::{Envelope, ResultCode},
};

use super::{Authority, KeyId, Reason, Root};
use crate::desktop::services::worker::RecordState;

/// What an attention answer needs from the person, before it is keyed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Need {
    /// `unlock_required`, `unlock_failed`: the key that was refused or asked.
    KeyLocked,
    /// `key_unavailable`, `key_rejected`: the repository's selected key.
    KeyProblem,
    /// `no_selected_key`: there is no key to name.
    NoKey,
    /// `host_approval_required`, `host_replacement_required`.
    Host,
    /// `invalid_remote`, `publication_remote_required`.
    Configuration,
}

/// What an answer comes to (E3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// `retired` or `already_absent`: the copy is gone.
    Retired,
    /// `pending`, or a request left `Accepted` whatever its answer: Owed.
    /// The pending row offers Resume and Abandon, and the entry stays. An
    /// unlock, key or host code raises its attention as well.
    Owed {
        attention: Option<Need>,
        /// `discovery_pending`: "The lists are still catching up."
        catching_up: bool,
    },
    /// `refused` with an unlock, key, host or configuration code.
    Attention(Need),
    /// `refused`, `transport_unavailable` or `remote_unavailable`.
    Unreachable,
    /// `refused` with an index, external-change, cancelled or internal code,
    /// or `recovery_required` with no removal: tried again at the next
    /// attempt. `recovery_required` is counted (three join the refused set).
    Retry { recovery_required: bool },
    /// `refused` or `preserved` with any other code: the item's own answer.
    Refused,
    /// A finished request with no retirement data: an ordinary failure. It
    /// stays, counts, and is tried again at the next attempt; the loop
    /// stops (R-37).
    Failed,
}

impl Verdict {
    /// Whether the request is dismissed from Activity at once, user-lane
    /// ones included: attention or the next attempt covers it, so it is
    /// neither shown nor counted twice (R-23). Owed, refused, preserved and
    /// failed ones stay, and count.
    pub(crate) fn dismissed(&self) -> bool {
        matches!(
            self,
            Self::Retired | Self::Attention(_) | Self::Unreachable | Self::Retry { .. }
        )
    }

    /// Whether the loop goes on to the next item (RT-7), rather than
    /// stopping until the next completed attempt (RT-8, R-37). A finished
    /// request with no retirement data stops it too (owner ruling,
    /// 2026-11-10): the next item would meet the same failure.
    pub(crate) fn goes_on(&self) -> bool {
        matches!(self, Self::Retired | Self::Retry { .. } | Self::Refused)
    }

    /// The attention the answer raises, if any: a refusal's, or an Owed
    /// answer's with an unlock, key or host code.
    pub(crate) fn need(&self) -> Option<Need> {
        match self {
            Self::Attention(need) => Some(*need),
            Self::Owed { attention, .. } => *attention,
            _ => None,
        }
    }
}

/// A classified answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Retirement {
    pub verdict: Verdict,
    /// The state the library gave, when it gave data.
    pub state: Option<ContextRetirementState>,
    /// The code the library answered the request with.
    pub code: ResultCode,
    /// The journal row's operation: what the request and its pending row
    /// are grouped by.
    pub operation: Option<String>,
    /// Something was removed, or the copy was found gone: the repository's
    /// tabs read their items again (A20, Fact 25).
    pub changed: bool,
}

impl Retirement {
    /// Whether the answer got past authentication, so the key's and host's
    /// attention for the repository is proved wrong (C3): the retirement
    /// took its observation and went on. Only the answers that cannot have
    /// come before it count: removed, already absent, or preserved.
    pub(crate) fn authenticated(&self) -> bool {
        self.verdict == Verdict::Retired || self.state == Some(ContextRetirementState::Preserved)
    }
}

/// The attention an unlock, key, host or configuration code needs.
fn need_of(code: ResultCode) -> Option<Need> {
    Some(match code {
        ResultCode::UnlockRequired | ResultCode::UnlockFailed => Need::KeyLocked,
        ResultCode::NoSelectedKey => Need::NoKey,
        ResultCode::KeyUnavailable | ResultCode::KeyRejected => Need::KeyProblem,
        ResultCode::HostApprovalRequired | ResultCode::HostReplacementRequired => Need::Host,
        ResultCode::InvalidRemote | ResultCode::PublicationRemoteRequired => Need::Configuration,
        _ => return None,
    })
}

/// Sorts the answer of a retirement request (E3): the record state, then
/// the answer's state, then its code.
///
/// A request left `Accepted` is Owed whatever it was answered, and so is one
/// whose record could not be read: the entry is kept (R-31) and Resume
/// replays a final answer, so nothing is lost by waiting.
pub(crate) fn classify(record: RecordState, envelope: &Envelope<MutationDataDto>) -> Retirement {
    let data = match &envelope.data {
        Some(MutationDataDto::ContextRetirement(data)) => Some(data),
        _ => None,
    };
    let code = data.map_or(envelope.code, |data| data.code);
    let removed = data.is_some_and(|data| {
        data.worktree_removal != RetirementRemoval::None
            || data.branch_removal != RetirementRemoval::None
    });
    let owed = |code| Verdict::Owed {
        // E3 row 2: only an unlock, key or host code raises attention here;
        // a configuration code is no pending row's business.
        attention: need_of(code).filter(|need| *need != Need::Configuration),
        catching_up: code == ResultCode::DiscoveryPending,
    };
    let verdict = match (record, data) {
        (RecordState::Accepted | RecordState::Unknown, _) => owed(code),
        (RecordState::Finished, None) => Verdict::Failed,
        (RecordState::Finished, Some(data)) => match data.state {
            ContextRetirementState::Retired | ContextRetirementState::AlreadyAbsent => {
                Verdict::Retired
            }
            ContextRetirementState::Pending => owed(code),
            ContextRetirementState::Preserved => Verdict::Refused,
            ContextRetirementState::Refused => match code {
                ResultCode::TransportUnavailable | ResultCode::RemoteUnavailable => {
                    Verdict::Unreachable
                }
                ResultCode::IndexUnavailable
                | ResultCode::ExternalChange
                | ResultCode::InternalError
                | ResultCode::Cancelled => Verdict::Retry {
                    recovery_required: false,
                },
                ResultCode::RecoveryRequired if !removed => Verdict::Retry {
                    recovery_required: true,
                },
                code => need_of(code).map_or(Verdict::Refused, Verdict::Attention),
            },
        },
    };
    Retirement {
        verdict,
        state: data.map(|data| data.state),
        code,
        // The journal row's operation: the data's, else the one the envelope
        // names, so a request left Accepted with no data is still grouped
        // with its pending row (R-23).
        operation: data
            .map(|data| data.operation_id.clone())
            .or_else(|| envelope.operation_id.clone()),
        // The copy was removed or found gone, or part of it was (Fact 25).
        changed: removed || verdict == Verdict::Retired,
    }
}

/// The one key a retirement's answer is held under, from the keys the
/// background provider refused and the keys the request asked for: Task 8's
/// rule for a Check now. Exactly one is that key; with none or several the
/// answer names no key, and the repository's selected key is read instead
/// (E3). The sets come in no order, so a first of several would be a guess.
pub(crate) fn recorded_key(refused: &[KeyId], asked: &[KeyId]) -> Option<KeyId> {
    let keys = if refused.is_empty() { asked } else { refused };
    match keys {
        [only] => Some(*only),
        _ => None,
    }
}

/// The attention reason for `need` on `root`: keyed by the key the
/// background provider refused or the request asked for, else the
/// repository's selected key (E3); a host by the host the scheduler knows
/// for the repository. With no key or host known, the repository is the
/// entry, as for a poll (A4).
pub(crate) fn reason_of(
    need: Need,
    root: &Root,
    key: Option<KeyId>,
    host: Option<Authority>,
) -> Reason {
    match need {
        Need::KeyLocked => {
            key.map_or_else(|| Reason::NoKeySelected(root.clone()), Reason::KeyLocked)
        }
        Need::KeyProblem => {
            key.map_or_else(|| Reason::NoKeySelected(root.clone()), Reason::KeyProblem)
        }
        Need::NoKey => Reason::NoKeySelected(root.clone()),
        Need::Host => host.map_or_else(
            || Reason::HostUnidentified(root.clone()),
            Reason::HostUnapproved,
        ),
        Need::Configuration => Reason::ConfigurationRequired(root.clone()),
    }
}

/// The items whose retirement was refused or preserved with a code the
/// answer table does not retry (R-21): the loop skips them for the session,
/// and Retire… removes one (A19).
#[derive(Debug, Default)]
pub(crate) struct RefusedSet {
    refused: BTreeSet<(Root, ItemId)>,
    /// How many times each item was refused `recovery_required`.
    recovery: BTreeMap<(Root, ItemId), u8>,
}

/// Three `recovery_required` refusals in a session are the item's answer.
const RECOVERY_REFUSALS: u8 = 3;

impl RefusedSet {
    /// Records the answer to a retirement of `item`: a refusal or
    /// preservation joins the set, and so does the third `recovery_required`.
    pub(crate) fn answered(&mut self, root: &Root, item: &ItemId, verdict: &Verdict) {
        let key = (root.clone(), item.clone());
        match verdict {
            Verdict::Refused => {
                self.refused.insert(key);
            }
            Verdict::Retry {
                recovery_required: true,
            } => {
                let count = self.recovery.entry(key.clone()).or_default();
                *count += 1;
                if *count >= RECOVERY_REFUSALS {
                    self.refused.insert(key);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn contains(&self, root: &Root, item: &ItemId) -> bool {
        self.refused.contains(&(root.clone(), item.clone()))
    }

    /// The repository is forgotten: so is everything held for it.
    pub(crate) fn forget_root(&mut self, root: &Root) {
        self.refused.retain(|(held, _)| held != root);
        self.recovery.retain(|(held, _), _| held != root);
    }

    /// Retire… was chosen for the item: it and its count start over (A19).
    pub(crate) fn forget(&mut self, root: &Root, item: &ItemId) {
        let key = (root.clone(), item.clone());
        self.refused.remove(&key);
        self.recovery.remove(&key);
    }
}

#[cfg(test)]
#[path = "retire_tests.rs"]
mod tests;
