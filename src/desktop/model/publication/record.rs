//! What a post's request record says, as the views use it (design, P5,
//! P12, P16; orchestrator changes 1-3; owner decisions 23 and 24): a
//! stored result classified as a `CommentOutcome`, and what Check again
//! comes to from the record, the listing and the post's last answer.
//!
//! The views never read a stored result's fields: they ask here. A record
//! once found ends Check again in a final state, never in another bare
//! "may already be saved", so it cannot loop.

use manyhands::{
    repository::{
        CommentMutationDto, CommentMutationType, CommentPendingReason, ConflictNextAction,
        ConflictNextDto, MutationDataDto, RequestResultDto,
    },
    results::{CheckpointEffect, Envelope, PublicationEffect, ResultCode, Scope, WriteEffect},
};
use serde_json::Value;

use super::{Attempt, CommentResult, comment_outcome};
use crate::desktop::services::worker::RecordState;

/// A stored result as the envelope it was: its outcome, code, effects
/// and, for a comment, its data.
fn envelope_of(result: &RequestResultDto) -> Envelope<MutationDataDto> {
    let mut envelope = Envelope::failure(
        "comment add",
        Scope::default(),
        ResultCode::InternalError,
        vec![],
    );
    envelope.outcome = result.outcome;
    envelope.code = result.code;
    envelope.effects = result.effects.clone();
    envelope.data = result
        .data
        .as_ref()
        .and_then(comment_of)
        .map(MutationDataDto::Comment);
    envelope
}

/// A comment's data as a record stores it; `None` for any other.
fn comment_of(stored: &Value) -> Option<CommentMutationDto> {
    let text = |key: &str| stored.get(key)?.as_str().map(str::to_owned);
    if stored.get("type")?.as_str()? != CommentMutationType::Comment.as_str() {
        return None;
    }
    let publication = stored.get("publication")?.as_str()?;
    let publication = PublicationEffect::ALL
        .into_iter()
        .find(|known| known.as_str() == publication)?;
    let pending_reason = stored
        .get("pending_reason")
        .and_then(Value::as_str)
        .and_then(|reason| {
            CommentPendingReason::ALL
                .into_iter()
                .find(|known| known.as_str() == reason)
        });
    let conflict = stored.get("conflict").and_then(|conflict| {
        let next_action = conflict.get("next_action")?.as_str()?;
        Some(ConflictNextDto {
            operation_id: conflict.get("operation_id")?.as_str()?.to_owned(),
            next_action: ConflictNextAction::ALL
                .into_iter()
                .find(|known| known.as_str() == next_action)?,
        })
    });
    Some(CommentMutationDto {
        r#type: CommentMutationType::Comment,
        item_id: text("item_id")?,
        comment_id: text("comment_id")?,
        parent_id: text("parent_id"),
        created_by: text("created_by"),
        synchronization_id: text("synchronization_id").unwrap_or_default(),
        publication,
        pending_reason,
        conflict,
    })
}

/// The outcome a stored result of a post comes to, under `attempt`.
pub(crate) fn comment_outcome_of_record(
    result: &RequestResultDto,
    attempt: Attempt,
) -> CommentResult {
    comment_outcome(&envelope_of(result), attempt)
}

/// The envelope a stored result was, for a post's state.
pub(crate) fn envelope_of_record(result: &RequestResultDto) -> Envelope<MutationDataDto> {
    envelope_of(result)
}

/// Whether a stored result committed the comment's checkpoint.
pub(crate) fn record_saved(result: &RequestResultDto) -> bool {
    result.effects.checkpoint == CheckpointEffect::Committed
}

/// Whether a stored result wrote nothing and committed nothing.
fn record_untouched(result: &RequestResultDto) -> bool {
    result.effects.write != WriteEffect::Written
        && result.effects.checkpoint != CheckpointEffect::Committed
}

/// Where a post's record stands, as `show_request` answered.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum RecordRead {
    /// `show_request` failed.
    Unknown,
    /// `request_not_found`: no record holds the comment ID.
    None,
    Accepted,
    Finished(RequestResultDto),
}

/// What a refreshed listing says of the comment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Listing {
    /// The refresh or the listing failed, or the listing is incomplete or
    /// its index not current: it proves nothing.
    Unproven,
    Listed,
    NotListed,
}

/// What Check again comes to.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Checked {
    /// The record committed the comment: it is saved, with this outcome.
    Saved(CommentResult),
    /// A record that did not save it, final: the text is kept with Copy
    /// text and Discard; Retry only while the same request can still
    /// finish it; Post as a new comment when nothing of it exists
    /// (decision 23).
    Final { retry: bool, post_as_new: bool },
    /// No record, but the comment is listed: the draft retires, and
    /// neither Saved nor the not-shared chip is claimed (decision 24).
    Listed,
    /// Nothing proven: Check again stays, and Post as a new comment is
    /// offered only when nothing of the comment exists (P16).
    Maybe { post_as_new: bool },
}

/// Check again's answer from the record, the listing and the code of the
/// post's last answer.
pub(crate) fn check_again(record: &RecordRead, listing: Listing, last: ResultCode) -> Checked {
    match record {
        RecordRead::Finished(result) if record_saved(result) => {
            Checked::Saved(comment_outcome_of_record(
                result,
                Attempt {
                    resend: true,
                    record: RecordState::Finished,
                    after_give_up: false,
                },
            ))
        }
        RecordRead::Finished(result) => Checked::Final {
            retry: false,
            post_as_new: record_untouched(result) && listing == Listing::NotListed,
        },
        RecordRead::Accepted => Checked::Final {
            retry: last != ResultCode::InternalError,
            post_as_new: false,
        },
        RecordRead::None => match listing {
            Listing::Listed => Checked::Listed,
            Listing::NotListed => Checked::Maybe { post_as_new: true },
            Listing::Unproven => Checked::Maybe { post_as_new: false },
        },
        RecordRead::Unknown => Checked::Maybe { post_as_new: false },
    }
}

/// Whether a mismatched post is saved (change 1): an accepted record's
/// refusal carries its checkpoint; a finished one's is read from the
/// record; an unknown one is not said to be saved.
pub(crate) fn mismatch_saved(
    record: RecordState,
    refusal_committed: bool,
    stored: Option<&RecordRead>,
) -> bool {
    match record {
        RecordState::Accepted => refusal_committed,
        RecordState::Finished => {
            matches!(stored, Some(RecordRead::Finished(result)) if record_saved(result))
        }
        RecordState::Unknown => false,
    }
}

#[cfg(test)]
#[path = "record_tests.rs"]
mod tests;
