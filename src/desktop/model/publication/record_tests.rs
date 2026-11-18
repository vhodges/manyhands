//! A stored result classified, and what Check again comes to (changes 1-3,
//! decisions 23 and 24).

use manyhands::{
    repository::RequestResultDto,
    results::{CheckpointEffect, Effects, Outcome, PublicationEffect, ResultCode, WriteEffect},
};
use serde_json::json;

use super::{Checked, Listing, RecordRead, check_again, comment_outcome_of_record, mismatch_saved};
use crate::desktop::{
    model::publication::{Attempt, CommentOffer, CommentOutcome},
    services::worker::RecordState,
};

const FINISHED: Attempt = Attempt {
    resend: true,
    record: RecordState::Finished,
    after_give_up: false,
};

fn stored(
    code: ResultCode,
    write: WriteEffect,
    checkpoint: CheckpointEffect,
    publication: PublicationEffect,
    data: Option<serde_json::Value>,
) -> RequestResultDto {
    RequestResultDto {
        outcome: if code == ResultCode::Ok {
            Outcome::Success
        } else {
            Outcome::Error
        },
        code,
        message: String::new(),
        effects: Effects {
            write,
            checkpoint,
            publication,
            ..Effects::not_requested()
        },
        data,
    }
}

fn comment_data(publication: &str, reason: Option<&str>) -> serde_json::Value {
    json!({
        "type": "comment",
        "item_id": "01M4CC0VN1KTQFSVR99378TDHA",
        "comment_id": "01M4CC0VN1KTQFSVR99378TDHB",
        "parent_id": null,
        "created_by": "Ada <ada@example.invalid>",
        "synchronization_id": "01M4CC0VN1KTQFSVR99378TDHC",
        "publication": publication,
        "pending_reason": reason,
        "conflict": null,
    })
}

fn saved_here() -> RequestResultDto {
    stored(
        ResultCode::Ok,
        WriteEffect::Written,
        CheckpointEffect::Committed,
        PublicationEffect::Pending,
        Some(comment_data("pending", Some("no_publication_remote"))),
    )
}

fn untouched() -> RequestResultDto {
    stored(
        ResultCode::TransportUnavailable,
        WriteEffect::NotRequested,
        CheckpointEffect::NotRequested,
        PublicationEffect::NotRequested,
        None,
    )
}

/// Change 3: a stored result is classified as the envelope it was, its
/// comment data read from the record.
#[test]
fn a_stored_result_is_classified_as_its_envelope() {
    assert_eq!(
        comment_outcome_of_record(&saved_here(), FINISHED).outcome,
        CommentOutcome::SavedHere
    );
    let shared = stored(
        ResultCode::Ok,
        WriteEffect::Written,
        CheckpointEffect::Committed,
        PublicationEffect::Published,
        Some(comment_data("published", None)),
    );
    assert_eq!(
        comment_outcome_of_record(&shared, FINISHED).outcome,
        CommentOutcome::Shared
    );
    let owed = stored(
        ResultCode::InternalError,
        WriteEffect::Written,
        CheckpointEffect::Committed,
        PublicationEffect::Pending,
        Some(comment_data("pending", Some("synchronization"))),
    );
    let result = comment_outcome_of_record(&owed, FINISHED);
    assert_eq!(
        result.outcome,
        CommentOutcome::SavedNotSharedFinal {
            code: ResultCode::InternalError
        }
    );
    assert_eq!(result.offers(), vec![CommentOffer::Share]);
    assert_eq!(
        comment_outcome_of_record(&untouched(), FINISHED).outcome,
        CommentOutcome::NotSaved {
            code: ResultCode::TransportUnavailable
        }
    );
}

/// Change 2: a finished record that committed is saved; one that did not
/// is final, with Post as a new comment only when it touched nothing and
/// a complete, current listing lacks the comment (decision 23).
#[test]
fn a_finished_record_ends_check_again() {
    assert!(matches!(
        check_again(
            &RecordRead::Finished(saved_here()),
            Listing::Unproven,
            ResultCode::InternalError
        ),
        Checked::Saved(result) if result.outcome == CommentOutcome::SavedHere
    ));
    assert_eq!(
        check_again(
            &RecordRead::Finished(untouched()),
            Listing::NotListed,
            ResultCode::InternalError
        ),
        Checked::Final {
            retry: false,
            post_as_new: true
        }
    );
    for listing in [Listing::Listed, Listing::Unproven] {
        assert_eq!(
            check_again(
                &RecordRead::Finished(untouched()),
                listing,
                ResultCode::InternalError
            ),
            Checked::Final {
                retry: false,
                post_as_new: false
            }
        );
    }
    let written = stored(
        ResultCode::TransportUnavailable,
        WriteEffect::Written,
        CheckpointEffect::NotRequested,
        PublicationEffect::NotRequested,
        None,
    );
    assert_eq!(
        check_again(
            &RecordRead::Finished(written),
            Listing::NotListed,
            ResultCode::InternalError
        ),
        Checked::Final {
            retry: false,
            post_as_new: false
        },
        "a written file may still appear"
    );
}

/// Change 2: an accepted record is final too: Retry only when the last
/// answer was not `internal_error`, which a retry would only repeat.
#[test]
fn an_accepted_record_retries_only_past_a_non_internal_error() {
    assert_eq!(
        check_again(
            &RecordRead::Accepted,
            Listing::NotListed,
            ResultCode::TransportUnavailable
        ),
        Checked::Final {
            retry: true,
            post_as_new: false
        }
    );
    assert_eq!(
        check_again(
            &RecordRead::Accepted,
            Listing::NotListed,
            ResultCode::InternalError
        ),
        Checked::Final {
            retry: false,
            post_as_new: false
        }
    );
}

/// P16 and decision 24: with no record, a listed comment retires the
/// draft claiming nothing; an unlisted one, on a complete current
/// listing, may be posted anew; an unproven listing or an unknown record
/// proves nothing.
#[test]
fn no_record_follows_the_listing() {
    let none = |listing| check_again(&RecordRead::None, listing, ResultCode::InternalError);
    assert_eq!(none(Listing::Listed), Checked::Listed);
    assert_eq!(
        none(Listing::NotListed),
        Checked::Maybe { post_as_new: true }
    );
    assert_eq!(
        none(Listing::Unproven),
        Checked::Maybe { post_as_new: false }
    );
    assert_eq!(
        check_again(
            &RecordRead::Unknown,
            Listing::NotListed,
            ResultCode::InternalError
        ),
        Checked::Maybe { post_as_new: false }
    );
}

/// Change 1: a mismatch is saved by the refusal's checkpoint for an
/// accepted record, by the stored result for a finished one, and never
/// said to be saved when the record is unknown.
#[test]
fn a_mismatch_is_saved_by_its_record() {
    assert!(mismatch_saved(RecordState::Accepted, true, None));
    assert!(!mismatch_saved(RecordState::Accepted, false, None));
    assert!(mismatch_saved(
        RecordState::Finished,
        false,
        Some(&RecordRead::Finished(saved_here()))
    ));
    assert!(!mismatch_saved(
        RecordState::Finished,
        true,
        Some(&RecordRead::Finished(untouched()))
    ));
    assert!(!mismatch_saved(
        RecordState::Finished,
        true,
        Some(&RecordRead::Unknown)
    ));
    assert!(!mismatch_saved(RecordState::Unknown, true, None));
}
