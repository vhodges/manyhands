//! What a post or a share answered, as the views show it: a comment's
//! `CommentOutcome` and a share's `SyncOutcome`, each kept with the
//! attempt it was classified under, and the controls it offers. The
//! views render these and never read envelope fields themselves.
//!
//! Rows match in the design's precedence order, first match wins.
//! "Saved" means the checkpoint committed. Outcomes hold no comment text.
//!
//! - Give up ("Stop trying to share") is offered only while the record
//!   is still accepted (decision 8), on the rows that name it, and never
//!   again on the answer of a Give up (P14).
//! - The same request (Retry, Retry sharing, Unlock…) is never offered on
//!   a finished record, which only replays its result; a new share is,
//!   where one helps (orchestrator ruling, from R-12). Only the transient
//!   codes keep Retry whatever the record (fact 14).
//! - Where `show_request` failed and the record's state is unknown, a
//!   post offers no Give up, no Edit text and no Share in place of a
//!   retry, and adds Check again (P17). Check again classifies afresh.

use manyhands::{
    repository::{
        CommentMutationDto, CommentPendingReason, ConflictNextDto, MutationDataDto,
        transport::SshAuthority,
    },
    results::{
        CheckpointEffect, Envelope, Outcome, PublicationEffect, RecoveryActionKind, ResultCode,
        WriteEffect,
    },
};

pub(crate) mod record;

use super::operations::parse_authority;
use crate::desktop::services::worker::RecordState;

/// What Share on an item says when the item has nothing saved to share
/// (decision 18, U-1).
pub(crate) const NOTHING_TO_SHARE: &str = "Nothing to share yet: this item has no saved changes.";

/// One finished attempt of a post: whether it re-sent a comment ID
/// already sent (an adopted draft's first post, or a Retry), where its
/// record stood afterwards, and whether it was a Give up's re-send.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Attempt {
    pub(crate) resend: bool,
    pub(crate) record: RecordState,
    pub(crate) after_give_up: bool,
}

/// Where a post stands, by the design's comment outcome rows.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum CommentOutcome {
    /// Row 0: the request's input differs from its record (P12).
    Mismatched { saved: bool },
    /// Row 1.
    Shared,
    /// Row 2: there is no publication remote.
    SavedHere,
    /// Row 3: its publication was stopped, which is final.
    SavedStopped,
    /// Row 4: its synchronization stopped on a conflict (decision 13).
    SavedConflict { next: ConflictNextDto },
    /// Row 5: the shared copy of the item was deleted.
    SavedBranchDeleted,
    /// Row 6.
    SavedNeedsUnlock,
    /// Row 7.
    SavedNeedsKey,
    /// Row 8.
    SavedNeedsHost,
    /// Row 9c: the publication's merge needs an identity.
    SavedNeedsIdentity,
    /// Row 9a: not shared, and the same request can still share it.
    SavedNotShared { code: ResultCode },
    /// Row 9b: not shared, and the request is finished.
    SavedNotSharedFinal { code: ResultCode },
    /// Row 10.
    NotSavedStopped,
    /// Row 11.
    NotSavedIdentity,
    /// Row 12: the comment's path holds other content.
    NotSavedConflicting,
    /// Row 13: an internal error on a re-send (R-8, P15).
    MaybeSaved,
    /// Row 14: written, or its record is open: only the same request
    /// may finish it (R-12, P7).
    NotSavedOwed { code: ResultCode },
    /// Row 15: nothing written and nothing open (R-12, P6).
    NotSaved { code: ResultCode },
}

/// A post's outcome, with the attempt it was classified under.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct CommentResult {
    pub(crate) outcome: CommentOutcome,
    pub(crate) attempt: Attempt,
}

/// A control a post's outcome offers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CommentOffer {
    /// The same request again, for a saved comment's publication.
    RetrySharing,
    /// The same request again, for a post that is not saved.
    Retry,
    GiveUp,
    /// A new `item sync`.
    Share,
    /// Clear the declined key, then the same request again (R-3).
    Unlock,
    OpenSharingAndKeys,
    CheckHost,
    SetIdentity,
    /// Back to editing; the text posts under a new comment ID (R-16).
    EditText,
    Discard,
    CopyText,
    CheckAgain,
}

/// The outcome of a post's attempt.
pub(crate) fn comment_outcome(
    envelope: &Envelope<MutationDataDto>,
    attempt: Attempt,
) -> CommentResult {
    CommentResult {
        outcome: classify_comment(envelope, attempt),
        attempt,
    }
}

fn classify_comment(envelope: &Envelope<MutationDataDto>, attempt: Attempt) -> CommentOutcome {
    let code = envelope.code;
    let saved = envelope.effects.checkpoint == CheckpointEffect::Committed;
    let data = comment_data(envelope);
    let cancelled = envelope.outcome == Outcome::Cancelled;
    if code == ResultCode::RequestMismatch {
        return CommentOutcome::Mismatched { saved };
    }
    if data.is_some_and(|data| {
        matches!(
            data.publication,
            PublicationEffect::Published | PublicationEffect::Current
        )
    }) {
        return CommentOutcome::Shared;
    }
    if saved {
        saved_outcome(envelope, data, cancelled, attempt.record)
    } else if cancelled {
        CommentOutcome::NotSavedStopped
    } else if code == ResultCode::IdentityRequired {
        CommentOutcome::NotSavedIdentity
    } else if code == ResultCode::OccupiedPath {
        CommentOutcome::NotSavedConflicting
    } else if code == ResultCode::InternalError && attempt.resend {
        CommentOutcome::MaybeSaved
    } else if envelope.effects.write == WriteEffect::Written
        || attempt.record != RecordState::Finished
    {
        CommentOutcome::NotSavedOwed { code }
    } else {
        CommentOutcome::NotSaved { code }
    }
}

/// Rows 2 to 9b, for a post whose checkpoint committed.
fn saved_outcome(
    envelope: &Envelope<MutationDataDto>,
    data: Option<&CommentMutationDto>,
    cancelled: bool,
    record: RecordState,
) -> CommentOutcome {
    let code = envelope.code;
    if data
        .is_some_and(|data| data.pending_reason == Some(CommentPendingReason::NoPublicationRemote))
    {
        return CommentOutcome::SavedHere;
    }
    // Row 3 names a pending publication. A saved post stopped with its
    // publication `not_requested` never began publishing; it falls
    // through to the code match below, where `cancelled` is no code of
    // its own and lands on row 9a or 9b, which offer the same request
    // while it is open and Share once it is finished: what row 3 offers
    // too, once the stop has finished the record.
    if cancelled && envelope.effects.publication == PublicationEffect::Pending {
        return CommentOutcome::SavedStopped;
    }
    if let Some(next) = data.and_then(|data| data.conflict.clone()) {
        return CommentOutcome::SavedConflict { next };
    }
    match code {
        ResultCode::RemoteBranchDeleted => CommentOutcome::SavedBranchDeleted,
        ResultCode::UnlockRequired | ResultCode::UnlockFailed => CommentOutcome::SavedNeedsUnlock,
        code if KEY_CODES.contains(&code) => CommentOutcome::SavedNeedsKey,
        code if HOST_CODES.contains(&code) => CommentOutcome::SavedNeedsHost,
        ResultCode::IdentityRequired => CommentOutcome::SavedNeedsIdentity,
        code if record == RecordState::Finished => CommentOutcome::SavedNotSharedFinal { code },
        code => CommentOutcome::SavedNotShared { code },
    }
}

impl CommentOutcome {
    /// Whether the comment's checkpoint committed.
    pub(crate) fn is_saved(&self) -> bool {
        match self {
            Self::Mismatched { saved } => *saved,
            Self::NotSavedStopped
            | Self::NotSavedIdentity
            | Self::NotSavedConflicting
            | Self::MaybeSaved
            | Self::NotSavedOwed { .. }
            | Self::NotSaved { .. } => false,
            _ => true,
        }
    }
}

impl CommentResult {
    /// The controls this outcome offers under its attempt's record.
    pub(crate) fn offers(&self) -> Vec<CommentOffer> {
        use CommentOffer as O;
        use CommentOutcome as C;
        let record = self.attempt.record;
        // Rows 5 to 9a depend on the record: the row's own step, then
        // the same request (Retry sharing, or Unlock…, which is one).
        let (step, resend) = match &self.outcome {
            C::Mismatched { .. } => return vec![O::Discard, O::CopyText],
            C::Shared | C::SavedHere | C::SavedConflict { .. } => return Vec::new(),
            C::SavedStopped | C::SavedNotSharedFinal { .. } => return vec![O::Share],
            C::NotSavedStopped | C::NotSaved { .. } => return vec![O::EditText],
            C::NotSavedIdentity => {
                return vec![O::SetIdentity, O::Retry, O::Discard, O::CopyText];
            }
            C::NotSavedConflicting => return vec![O::Discard],
            C::MaybeSaved => return vec![O::CheckAgain],
            C::NotSavedOwed { .. } if record == RecordState::Unknown => {
                return vec![O::Retry, O::CheckAgain];
            }
            // A Retry only replays; the file exists, so no Edit text (P7).
            C::NotSavedOwed { .. } if record == RecordState::Finished => {
                return vec![O::Discard, O::CopyText, O::CheckAgain];
            }
            C::NotSavedOwed { .. } => return vec![O::Retry, O::Discard, O::CopyText],
            C::SavedBranchDeleted => (None, None),
            C::SavedNeedsUnlock => (None, Some(O::Unlock)),
            C::SavedNeedsKey => (Some(O::OpenSharingAndKeys), Some(O::RetrySharing)),
            C::SavedNeedsHost => (Some(O::CheckHost), Some(O::RetrySharing)),
            C::SavedNeedsIdentity => (Some(O::SetIdentity), Some(O::RetrySharing)),
            C::SavedNotShared { .. } => (None, Some(O::RetrySharing)),
        };
        let mut offers: Vec<O> = step.into_iter().collect();
        match record {
            // A finished record only replays: a new share instead.
            RecordState::Finished => offers.extend(resend.map(|_| O::Share)),
            RecordState::Accepted => {
                offers.extend(resend);
                if !self.attempt.after_give_up {
                    offers.push(O::GiveUp);
                }
            }
            RecordState::Unknown => {
                offers.extend(resend);
                offers.push(O::CheckAgain);
            }
        }
        offers
    }
}

/// Which share a sync's answer is of.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ShareTarget {
    /// Share on an item (`item sync`).
    Item,
    /// Share main (`repo sync`).
    Main,
}

/// One finished attempt of a share: its target, where its record stood
/// afterwards, and whether it was a Give up's re-send.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct ShareAttempt {
    pub(crate) target: ShareTarget,
    pub(crate) record: RecordState,
    pub(crate) after_give_up: bool,
}

/// Where a share stands, by the design's sync outcome rows.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum SyncOutcome {
    Published,
    /// The shared copy was already the same.
    Current,
    /// Stopped at a safe point; `completed` when its effects show work
    /// that finished.
    Stopped {
        completed: bool,
    },
    /// Stopped on a conflict (decision 13).
    Conflict {
        next: ConflictNextDto,
    },
    BranchDeleted,
    NeedsUnlock,
    NeedsKey,
    /// The host the recovery names, when it names one (K-3).
    NeedsHost {
        authority: Option<SshAuthority>,
    },
    NeedsIdentity,
    /// Another request holds the repository; the views look for a
    /// possible holder among this session's requests.
    Busy,
    /// The repository has no publication remote.
    NoRemote,
    /// Share on an item that has nothing saved (`NOTHING_TO_SHARE`).
    NothingToShare,
    /// Any other failure; `retry` when the same request can still get
    /// past it.
    Failed {
        code: ResultCode,
        retry: bool,
    },
}

/// A share's outcome, with the attempt it was classified under.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct SyncResult {
    pub(crate) outcome: SyncOutcome,
    pub(crate) attempt: ShareAttempt,
}

/// A control a share's outcome offers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SyncOffer {
    /// A new share of the same target.
    Share,
    /// The same request again.
    Retry,
    GiveUp,
    /// Clear the declined key, then the same request again (R-3).
    Unlock,
    OpenSharingAndKeys,
    CheckHost,
    SetIdentity,
    /// The request that may hold the repository, or the pending
    /// operations list.
    ShowHolder,
    /// The repository's Remotes page.
    SetUpSharing,
}

/// The outcome of a share's attempt.
pub(crate) fn sync_outcome(
    envelope: &Envelope<MutationDataDto>,
    attempt: ShareAttempt,
) -> SyncResult {
    SyncResult {
        outcome: classify_sync(envelope, attempt),
        attempt,
    }
}

fn classify_sync(envelope: &Envelope<MutationDataDto>, attempt: ShareAttempt) -> SyncOutcome {
    let code = envelope.code;
    match envelope.effects.publication {
        PublicationEffect::Published => return SyncOutcome::Published,
        PublicationEffect::Current => return SyncOutcome::Current,
        PublicationEffect::NotRequested | PublicationEffect::Pending => {}
    }
    if envelope.outcome == Outcome::Cancelled {
        return SyncOutcome::Stopped {
            completed: envelope.effects.is_durable(),
        };
    }
    if let Some(MutationDataDto::SynchronizationConflict(conflict)) = &envelope.data {
        return SyncOutcome::Conflict {
            next: conflict.conflict.clone(),
        };
    }
    match code {
        ResultCode::RemoteBranchDeleted => SyncOutcome::BranchDeleted,
        ResultCode::UnlockRequired | ResultCode::UnlockFailed => SyncOutcome::NeedsUnlock,
        code if KEY_CODES.contains(&code) => SyncOutcome::NeedsKey,
        code if HOST_CODES.contains(&code) => SyncOutcome::NeedsHost {
            authority: host_authority(envelope),
        },
        ResultCode::IdentityRequired => SyncOutcome::NeedsIdentity,
        ResultCode::Busy => SyncOutcome::Busy,
        ResultCode::PublicationRemoteRequired => SyncOutcome::NoRemote,
        ResultCode::ItemNotFound if attempt.target == ShareTarget::Item => {
            SyncOutcome::NothingToShare
        }
        code => SyncOutcome::Failed {
            code,
            retry: TRANSIENT_CODES.contains(&code) || attempt.record != RecordState::Finished,
        },
    }
}

impl SyncResult {
    /// The controls this outcome offers under its attempt's record: a
    /// finished record offers the fix and a new share, never the same
    /// request but for a transient code; an accepted one also offers
    /// Give up, unless this was a Give up's answer (P14).
    pub(crate) fn offers(&self) -> Vec<SyncOffer> {
        use SyncOffer as O;
        // The same request, or a new share once the record is finished.
        let finished = self.attempt.record == RecordState::Finished;
        let resend = |same: O| if finished { O::Share } else { same };
        // A fix step, then a new share once the record is finished.
        let fix = |step: O| {
            [Some(step), finished.then_some(O::Share)]
                .into_iter()
                .flatten()
                .collect()
        };
        let mut offers = match &self.outcome {
            SyncOutcome::Published
            | SyncOutcome::Current
            | SyncOutcome::Conflict { .. }
            | SyncOutcome::BranchDeleted
            | SyncOutcome::NothingToShare => Vec::new(),
            SyncOutcome::Stopped { .. } => vec![O::Share],
            SyncOutcome::NeedsUnlock => vec![resend(O::Unlock)],
            SyncOutcome::NeedsKey => fix(O::OpenSharingAndKeys),
            SyncOutcome::NeedsHost { .. } => fix(O::CheckHost),
            SyncOutcome::NeedsIdentity => vec![O::SetIdentity, resend(O::Retry)],
            SyncOutcome::Busy => vec![O::ShowHolder, O::Retry],
            SyncOutcome::NoRemote => vec![O::SetUpSharing],
            SyncOutcome::Failed { retry: true, .. } => vec![O::Retry],
            SyncOutcome::Failed { retry: false, .. } => vec![O::Share],
        };
        if self.attempt.record == RecordState::Accepted && !self.attempt.after_give_up {
            offers.push(O::GiveUp);
        }
        offers
    }
}

/// The codes of a key the share could not use.
pub(crate) const KEY_CODES: [ResultCode; 3] = [
    ResultCode::NoSelectedKey,
    ResultCode::KeyUnavailable,
    ResultCode::KeyRejected,
];

/// The codes of a host that is not trusted.
pub(crate) const HOST_CODES: [ResultCode; 2] = [
    ResultCode::HostApprovalRequired,
    ResultCode::HostReplacementRequired,
];

/// The codes the same request is always offered for, whatever its
/// record (fact 14).
const TRANSIENT_CODES: [ResultCode; 5] = [
    ResultCode::Busy,
    ResultCode::PollYielding,
    ResultCode::RemoteUnavailable,
    ResultCode::TransportUnavailable,
    ResultCode::RegistrationPending,
];

fn comment_data(envelope: &Envelope<MutationDataDto>) -> Option<&CommentMutationDto> {
    match &envelope.data {
        Some(MutationDataDto::Comment(data)) => Some(data),
        _ => None,
    }
}

/// The host a share's host recovery names, if it names one it can read.
pub(crate) fn host_authority(envelope: &Envelope<MutationDataDto>) -> Option<SshAuthority> {
    envelope
        .recovery
        .iter()
        .filter(|action| {
            matches!(
                action.action,
                RecoveryActionKind::HostApprove | RecoveryActionKind::HostReplace
            )
        })
        .find_map(|action| parse_authority(action.arguments.get("authority")?.as_str()?))
}

#[cfg(test)]
#[path = "publication_tests.rs"]
mod tests;
