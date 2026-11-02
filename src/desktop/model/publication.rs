//! What a post or a share answered, as the views show it: a comment's
//! `CommentOutcome` and a share's `SyncOutcome`, each with the controls
//! it offers. The views render these and never read envelope fields
//! themselves.
//!
//! Each function takes the design's rows in precedence order, and the
//! first that matches wins. "Saved" means the checkpoint committed. The
//! outcomes hold no comment text.
//!
//! Give up ("Stop trying to share") is offered only while the request's
//! library record is still accepted (decision 8), on the rows that name
//! it. Where `show_request` failed and the record's state is unknown, a
//! post offers no Give up, no Edit text and no Share in place of a retry,
//! and adds Check again (P17).

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

use super::operations::parse_authority;
use crate::desktop::services::worker::RecordState;

/// What Share on an item says when the item has nothing saved to share
/// (decision 18, U-1).
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 9.
pub(crate) const NOTHING_TO_SHARE: &str = "Nothing to share yet: this item has no saved changes.";

/// One finished attempt of a post: whether it re-sent a comment ID
/// already sent (an adopted draft's first post, or a Retry), and where
/// its record stood afterwards.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 13.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Attempt {
    pub(crate) resend: bool,
    pub(crate) record: RecordState,
}

/// Where a post stands, by the design's comment outcome rows.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 13.
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

/// A control a post's outcome offers.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 13.
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
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 13.
pub(crate) fn comment_outcome(
    envelope: &Envelope<MutationDataDto>,
    attempt: Attempt,
) -> CommentOutcome {
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

#[cfg_attr(not(test), allow(dead_code))] // Called from Task 13.
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

    /// The controls this outcome offers, where the request's record
    /// stands as `record` says.
    pub(crate) fn offers(&self, record: RecordState) -> Vec<CommentOffer> {
        use CommentOffer as O;
        let unknown = record == RecordState::Unknown;
        let (mut offers, record_dependent) = match self {
            Self::Mismatched { .. } => (vec![O::Discard, O::CopyText], false),
            Self::Shared | Self::SavedHere | Self::SavedConflict { .. } => (Vec::new(), false),
            Self::SavedStopped | Self::SavedNotSharedFinal { .. } => (vec![O::Share], false),
            Self::SavedBranchDeleted => (Vec::new(), true),
            Self::SavedNeedsUnlock => (vec![O::Unlock], true),
            Self::SavedNeedsKey => (vec![O::OpenSharingAndKeys, O::RetrySharing], true),
            Self::SavedNeedsHost => (vec![O::CheckHost, O::RetrySharing], true),
            Self::SavedNeedsIdentity => (vec![O::SetIdentity, O::RetrySharing], true),
            Self::SavedNotShared { .. } => (vec![O::RetrySharing], true),
            Self::NotSavedStopped | Self::NotSaved { .. } => (vec![O::EditText], false),
            Self::NotSavedIdentity => (
                vec![O::SetIdentity, O::Retry, O::Discard, O::CopyText],
                false,
            ),
            Self::NotSavedConflicting => (vec![O::Discard], false),
            Self::MaybeSaved => (vec![O::CheckAgain], false),
            Self::NotSavedOwed { .. } if unknown => (vec![O::Retry], true),
            Self::NotSavedOwed { .. } => (vec![O::Retry, O::Discard, O::CopyText], false),
        };
        if record_dependent {
            match record {
                RecordState::Accepted => offers.push(O::GiveUp),
                RecordState::Unknown => offers.push(O::CheckAgain),
                RecordState::Finished => {}
            }
        }
        offers
    }
}

/// Which share a sync's answer is of.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 9.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ShareTarget {
    /// Share on an item (`item sync`).
    Item,
    /// Share main (`repo sync`).
    Main,
}

/// Where a share stands, by the design's sync outcome rows.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 9.
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

/// A control a share's outcome offers.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 9.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SyncOffer {
    /// A new share of the same target.
    Share,
    /// The same request again.
    Retry,
    GiveUp,
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

/// The outcome of a share's attempt, where its record stood as `record`
/// says.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 9.
pub(crate) fn sync_outcome(
    envelope: &Envelope<MutationDataDto>,
    target: ShareTarget,
    record: RecordState,
) -> SyncOutcome {
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
        ResultCode::ItemNotFound if target == ShareTarget::Item => SyncOutcome::NothingToShare,
        code => SyncOutcome::Failed {
            code,
            retry: TRANSIENT_CODES.contains(&code) || record != RecordState::Finished,
        },
    }
}

#[cfg_attr(not(test), allow(dead_code))] // Called from Task 9.
impl SyncOutcome {
    /// The controls this outcome offers, where the request's record
    /// stands as `record` says: every outcome of an accepted record also
    /// offers Give up, and a failure that cannot be retried offers a new
    /// share.
    pub(crate) fn offers(&self, record: RecordState) -> Vec<SyncOffer> {
        use SyncOffer as O;
        let mut offers = match self {
            Self::Published
            | Self::Current
            | Self::Conflict { .. }
            | Self::BranchDeleted
            | Self::NothingToShare => Vec::new(),
            Self::Stopped { .. } => vec![O::Share],
            Self::NeedsUnlock => vec![O::Unlock],
            Self::NeedsKey => vec![O::OpenSharingAndKeys],
            Self::NeedsHost { .. } => vec![O::CheckHost],
            Self::NeedsIdentity => vec![O::SetIdentity, O::Retry],
            Self::Busy => vec![O::ShowHolder, O::Retry],
            Self::NoRemote => vec![O::SetUpSharing],
            Self::Failed { retry: true, .. } => vec![O::Retry],
            Self::Failed { retry: false, .. } => vec![O::Share],
        };
        if record == RecordState::Accepted {
            offers.push(O::GiveUp);
        }
        offers
    }
}

/// The codes of a key the share could not use.
const KEY_CODES: [ResultCode; 3] = [
    ResultCode::NoSelectedKey,
    ResultCode::KeyUnavailable,
    ResultCode::KeyRejected,
];

/// The codes of a host that is not trusted.
const HOST_CODES: [ResultCode; 2] = [
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
fn host_authority(envelope: &Envelope<MutationDataDto>) -> Option<SshAuthority> {
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
