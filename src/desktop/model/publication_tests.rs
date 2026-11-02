//! Every row of `comment_outcome` and `sync_outcome`, in the design's
//! precedence order, from constructed envelopes.

use manyhands::{
    repository::{
        CommentMutationDto, CommentMutationType, CommentPendingReason, ConflictNextAction,
        ConflictNextDto, MutationDataDto, SynchronizationConflictMutationDto,
        SynchronizationConflictType, transport::SshAuthority,
    },
    results::{
        CheckpointEffect, Effects, Envelope, PublicationEffect, RecoveryAction, RecoveryActionKind,
        ResultCode, Scope, WriteEffect,
    },
};

use super::{
    Attempt, CommentOffer as C, CommentOutcome, NOTHING_TO_SHARE, ShareTarget, SyncOffer as S,
    SyncOutcome, comment_outcome, sync_outcome,
};
use crate::desktop::services::worker::RecordState;

const ACCEPTED: Attempt = Attempt {
    resend: false,
    record: RecordState::Accepted,
};
const FINISHED: Attempt = Attempt {
    resend: false,
    record: RecordState::Finished,
};
const UNKNOWN: Attempt = Attempt {
    resend: false,
    record: RecordState::Unknown,
};
const RESENT: Attempt = Attempt {
    resend: true,
    record: RecordState::Finished,
};

fn effects(
    write: WriteEffect,
    checkpoint: CheckpointEffect,
    publication: PublicationEffect,
) -> Effects {
    Effects {
        write,
        checkpoint,
        publication,
        ..Effects::not_requested()
    }
}

/// A file written and committed, its publication as given.
fn saved(publication: PublicationEffect) -> Effects {
    effects(
        WriteEffect::Written,
        CheckpointEffect::Committed,
        publication,
    )
}

/// Nothing written or committed.
fn nothing() -> Effects {
    Effects::not_requested()
}

fn next() -> ConflictNextDto {
    ConflictNextDto {
        operation_id: "01M4CC0VN1KTQFSVR99378TDHV".to_owned(),
        next_action: ConflictNextAction::ALL[0],
    }
}

fn comment(
    publication: PublicationEffect,
    pending_reason: Option<CommentPendingReason>,
    conflict: Option<ConflictNextDto>,
) -> MutationDataDto {
    MutationDataDto::Comment(CommentMutationDto {
        r#type: CommentMutationType::Comment,
        item_id: "01M4CC0VN1KTQFSVR99378TDHA".to_owned(),
        comment_id: "01M4CC0VN1KTQFSVR99378TDHB".to_owned(),
        parent_id: None,
        created_by: None,
        synchronization_id: "01M4CC0VN1KTQFSVR99378TDHC".to_owned(),
        publication,
        pending_reason,
        conflict,
    })
}

/// A comment's answer: the code and effects, with comment data whose
/// publication is the effects' when the checkpoint committed.
fn post(code: ResultCode, effects: Effects) -> Envelope<MutationDataDto> {
    let committed = effects.checkpoint == CheckpointEffect::Committed;
    let publication = effects.publication;
    let envelope = Envelope::classified_mutation(
        "comment add",
        Scope::default(),
        code == ResultCode::Cancelled,
        code,
        effects,
        false,
    );
    if committed {
        let reason = (publication == PublicationEffect::Pending)
            .then_some(CommentPendingReason::Synchronization);
        envelope.with_data(comment(publication, reason, None))
    } else {
        envelope
    }
}

fn post_with(
    code: ResultCode,
    effects: Effects,
    data: MutationDataDto,
) -> Envelope<MutationDataDto> {
    let mut envelope = post(code, effects);
    envelope.data = Some(data);
    envelope
}

fn share(code: ResultCode, effects: Effects) -> Envelope<MutationDataDto> {
    Envelope::classified_mutation(
        "item sync",
        Scope::default(),
        code == ResultCode::Cancelled,
        code,
        effects,
        false,
    )
}

fn pending() -> Effects {
    saved(PublicationEffect::Pending)
}

// --- comment_outcome, row by row ---

#[test]
fn row_0_request_mismatch_is_mismatched() {
    let unsaved = post(ResultCode::RequestMismatch, nothing());
    let outcome = comment_outcome(&unsaved, ACCEPTED);
    assert_eq!(outcome, CommentOutcome::Mismatched { saved: false });
    assert_eq!(
        outcome.offers(RecordState::Accepted),
        [C::Discard, C::CopyText]
    );
}

/// The planned `request_mismatch_is_maybe_saved` predates row 0: a
/// mismatch is `Mismatched`, never retried or given up (P12), and shown
/// saved when its checkpoint committed.
#[test]
fn request_mismatch_is_mismatched() {
    let committed = Envelope::classified_mutation(
        "comment add",
        Scope::default(),
        false,
        ResultCode::RequestMismatch,
        pending(),
        false,
    );
    for attempt in [ACCEPTED, FINISHED, UNKNOWN, RESENT] {
        let outcome = comment_outcome(&committed, attempt);
        assert_eq!(outcome, CommentOutcome::Mismatched { saved: true });
        assert!(outcome.is_saved());
        let offers = outcome.offers(attempt.record);
        assert!(!offers.contains(&C::Retry));
        assert!(!offers.contains(&C::RetrySharing));
        assert!(!offers.contains(&C::GiveUp));
        assert_eq!(offers, [C::Discard, C::CopyText]);
    }
}

#[test]
fn row_1_a_published_or_current_comment_is_shared() {
    for publication in [PublicationEffect::Published, PublicationEffect::Current] {
        let envelope = post(ResultCode::Ok, saved(publication));
        let outcome = comment_outcome(&envelope, ACCEPTED);
        assert_eq!(outcome, CommentOutcome::Shared);
        assert!(outcome.offers(RecordState::Finished).is_empty());
    }
}

#[test]
fn row_2_no_publication_remote_is_saved_here() {
    let envelope = post_with(
        ResultCode::Ok,
        pending(),
        comment(
            PublicationEffect::Pending,
            Some(CommentPendingReason::NoPublicationRemote),
            None,
        ),
    );
    let outcome = comment_outcome(&envelope, FINISHED);
    assert_eq!(outcome, CommentOutcome::SavedHere);
    assert!(outcome.offers(RecordState::Finished).is_empty());
}

#[test]
fn row_3_a_saved_stopped_post_offers_share() {
    let envelope = post(ResultCode::Cancelled, pending());
    let outcome = comment_outcome(&envelope, FINISHED);
    assert_eq!(outcome, CommentOutcome::SavedStopped);
    assert_eq!(outcome.offers(RecordState::Finished), [C::Share]);
}

#[test]
fn row_4_a_saved_conflict_offers_nothing() {
    let envelope = post_with(
        ResultCode::RecoveryRequired,
        pending(),
        comment(
            PublicationEffect::Pending,
            Some(CommentPendingReason::Synchronization),
            Some(next()),
        ),
    );
    let outcome = comment_outcome(&envelope, ACCEPTED);
    assert_eq!(outcome, CommentOutcome::SavedConflict { next: next() });
    assert!(outcome.offers(RecordState::Accepted).is_empty());
}

#[test]
fn row_5_a_deleted_branch_offers_give_up_and_no_share() {
    let envelope = post(ResultCode::RemoteBranchDeleted, pending());
    let outcome = comment_outcome(&envelope, ACCEPTED);
    assert_eq!(outcome, CommentOutcome::SavedBranchDeleted);
    assert_eq!(outcome.offers(RecordState::Accepted), [C::GiveUp]);
    assert!(outcome.offers(RecordState::Finished).is_empty());
}

#[test]
fn row_6_a_locked_key_offers_unlock_and_give_up() {
    for code in [ResultCode::UnlockRequired, ResultCode::UnlockFailed] {
        let outcome = comment_outcome(&post(code, pending()), ACCEPTED);
        assert_eq!(outcome, CommentOutcome::SavedNeedsUnlock);
        assert_eq!(
            outcome.offers(RecordState::Accepted),
            [C::Unlock, C::GiveUp]
        );
    }
}

#[test]
fn row_7_a_key_problem_offers_sharing_and_keys() {
    for code in [
        ResultCode::NoSelectedKey,
        ResultCode::KeyUnavailable,
        ResultCode::KeyRejected,
    ] {
        let outcome = comment_outcome(&post(code, pending()), ACCEPTED);
        assert_eq!(outcome, CommentOutcome::SavedNeedsKey);
        assert_eq!(
            outcome.offers(RecordState::Accepted),
            [C::OpenSharingAndKeys, C::RetrySharing, C::GiveUp]
        );
    }
}

#[test]
fn row_8_a_host_problem_offers_check_host() {
    for code in [
        ResultCode::HostApprovalRequired,
        ResultCode::HostReplacementRequired,
    ] {
        let outcome = comment_outcome(&post(code, pending()), ACCEPTED);
        assert_eq!(outcome, CommentOutcome::SavedNeedsHost);
        assert_eq!(
            outcome.offers(RecordState::Accepted),
            [C::CheckHost, C::RetrySharing, C::GiveUp]
        );
    }
}

/// Row 9c: a saved comment whose child merge stopped on the identity.
#[test]
fn identity_required_routes_to_set_identity() {
    let outcome = comment_outcome(&post(ResultCode::IdentityRequired, pending()), ACCEPTED);
    assert_eq!(outcome, CommentOutcome::SavedNeedsIdentity);
    assert_eq!(
        outcome.offers(RecordState::Accepted),
        [C::SetIdentity, C::RetrySharing, C::GiveUp]
    );
}

/// Rows 9a and 9b.
#[test]
fn an_owed_unshared_comment_offers_retry_and_a_final_one_offers_share() {
    let envelope = post(ResultCode::TransportUnavailable, pending());
    let owed = comment_outcome(&envelope, ACCEPTED);
    assert_eq!(
        owed,
        CommentOutcome::SavedNotShared {
            code: ResultCode::TransportUnavailable
        }
    );
    assert_eq!(
        owed.offers(RecordState::Accepted),
        [C::RetrySharing, C::GiveUp]
    );
    let finished = comment_outcome(&envelope, FINISHED);
    assert_eq!(
        finished,
        CommentOutcome::SavedNotSharedFinal {
            code: ResultCode::TransportUnavailable
        }
    );
    assert_eq!(finished.offers(RecordState::Finished), [C::Share]);
}

#[test]
fn row_10_a_post_stopped_before_its_checkpoint_offers_edit_text() {
    let envelope = post(ResultCode::Cancelled, nothing());
    let outcome = comment_outcome(&envelope, FINISHED);
    assert_eq!(outcome, CommentOutcome::NotSavedStopped);
    assert_eq!(outcome.offers(RecordState::Finished), [C::EditText]);
}

#[test]
fn row_11_an_unsaved_post_needing_an_identity_offers_set_identity() {
    let envelope = post(ResultCode::IdentityRequired, nothing());
    let outcome = comment_outcome(&envelope, ACCEPTED);
    assert_eq!(outcome, CommentOutcome::NotSavedIdentity);
    assert_eq!(
        outcome.offers(RecordState::Accepted),
        [C::SetIdentity, C::Retry, C::Discard, C::CopyText]
    );
}

#[test]
fn row_12_an_occupied_path_keeps_the_text_and_offers_discard() {
    let envelope = post(ResultCode::OccupiedPath, nothing());
    let outcome = comment_outcome(&envelope, FINISHED);
    assert_eq!(outcome, CommentOutcome::NotSavedConflicting);
    assert_eq!(outcome.offers(RecordState::Finished), [C::Discard]);
}

/// Row 13. The planned name `internal_error_is_maybe_saved_only_for_an_adopted_first_post`
/// predates P15: `internal_error` on any re-send of a comment ID, an
/// adopted first post or a Retry, may have been saved; on a first send
/// it is an ordinary failure.
#[test]
fn internal_error_is_maybe_saved_only_on_a_resend() {
    let envelope = post(ResultCode::InternalError, nothing());
    for record in [
        RecordState::Accepted,
        RecordState::Finished,
        RecordState::Unknown,
    ] {
        let outcome = comment_outcome(
            &envelope,
            Attempt {
                resend: true,
                record,
            },
        );
        assert_eq!(outcome, CommentOutcome::MaybeSaved);
        assert_eq!(outcome.offers(record), [C::CheckAgain]);
        assert!(!outcome.offers(record).contains(&C::EditText));
    }
    assert_eq!(
        comment_outcome(&envelope, FINISHED),
        CommentOutcome::NotSaved {
            code: ResultCode::InternalError
        }
    );
}

#[test]
fn row_14_a_written_or_accepted_post_is_owed_and_offers_retry() {
    let written = post(
        ResultCode::TransportUnavailable,
        effects(
            WriteEffect::Written,
            CheckpointEffect::Pending,
            PublicationEffect::NotRequested,
        ),
    );
    let accepted = post(ResultCode::TransportUnavailable, nothing());
    for (envelope, attempt) in [(&written, FINISHED), (&accepted, ACCEPTED)] {
        let outcome = comment_outcome(envelope, attempt);
        assert_eq!(
            outcome,
            CommentOutcome::NotSavedOwed {
                code: ResultCode::TransportUnavailable
            }
        );
        let offers = outcome.offers(attempt.record);
        assert_eq!(offers, [C::Retry, C::Discard, C::CopyText]);
        assert!(!offers.contains(&C::EditText));
    }
}

#[test]
fn row_15_an_unwritten_finished_failure_offers_edit_text() {
    let envelope = post(ResultCode::TransportUnavailable, nothing());
    let outcome = comment_outcome(&envelope, FINISHED);
    assert_eq!(
        outcome,
        CommentOutcome::NotSaved {
            code: ResultCode::TransportUnavailable
        }
    );
    assert_eq!(outcome.offers(RecordState::Finished), [C::EditText]);
}

/// P17: where the record stands is unknown, a post offers Retry and
/// Check again, never Edit text or Give up.
#[test]
fn an_unknown_record_offers_retry_and_check_again_only() {
    let unsaved = comment_outcome(&post(ResultCode::TransportUnavailable, nothing()), UNKNOWN);
    assert_eq!(
        unsaved,
        CommentOutcome::NotSavedOwed {
            code: ResultCode::TransportUnavailable
        }
    );
    assert_eq!(
        unsaved.offers(RecordState::Unknown),
        [C::Retry, C::CheckAgain]
    );
    let saved = comment_outcome(&post(ResultCode::TransportUnavailable, pending()), UNKNOWN);
    assert_eq!(
        saved,
        CommentOutcome::SavedNotShared {
            code: ResultCode::TransportUnavailable
        }
    );
    assert_eq!(
        saved.offers(RecordState::Unknown),
        [C::RetrySharing, C::CheckAgain]
    );
    let locked = comment_outcome(&post(ResultCode::UnlockRequired, pending()), UNKNOWN);
    assert_eq!(
        locked.offers(RecordState::Unknown),
        [C::Unlock, C::CheckAgain]
    );
}

/// Give up is offered only while the record is accepted (decision 8).
#[test]
fn give_up_needs_an_accepted_record() {
    for code in [
        ResultCode::UnlockRequired,
        ResultCode::NoSelectedKey,
        ResultCode::HostApprovalRequired,
        ResultCode::IdentityRequired,
        ResultCode::RemoteBranchDeleted,
    ] {
        let outcome = comment_outcome(&post(code, pending()), FINISHED);
        assert!(!outcome.offers(RecordState::Finished).contains(&C::GiveUp));
        assert!(outcome.offers(RecordState::Accepted).contains(&C::GiveUp));
    }
}

#[test]
fn rows_match_in_precedence_order() {
    // Row 0 before row 1: a mismatch carrying published comment data.
    let mismatch = post_with(
        ResultCode::RequestMismatch,
        nothing(),
        comment(PublicationEffect::Published, None, None),
    );
    assert_eq!(
        comment_outcome(&mismatch, ACCEPTED),
        CommentOutcome::Mismatched { saved: false }
    );
    // Row 2 before row 3: stopped with no remote is saved here.
    let no_remote = post_with(
        ResultCode::Cancelled,
        pending(),
        comment(
            PublicationEffect::Pending,
            Some(CommentPendingReason::NoPublicationRemote),
            None,
        ),
    );
    assert_eq!(
        comment_outcome(&no_remote, FINISHED),
        CommentOutcome::SavedHere
    );
    // Row 3 before row 4: stopped with conflict data.
    let stopped = post_with(
        ResultCode::Cancelled,
        pending(),
        comment(
            PublicationEffect::Pending,
            Some(CommentPendingReason::Synchronization),
            Some(next()),
        ),
    );
    assert_eq!(
        comment_outcome(&stopped, FINISHED),
        CommentOutcome::SavedStopped
    );
    // Row 4 before row 6: a conflict answered with a locked key's code.
    let conflict = post_with(
        ResultCode::UnlockRequired,
        pending(),
        comment(
            PublicationEffect::Pending,
            Some(CommentPendingReason::Synchronization),
            Some(next()),
        ),
    );
    assert_eq!(
        comment_outcome(&conflict, ACCEPTED),
        CommentOutcome::SavedConflict { next: next() }
    );
    // Row 9c before row 9a; row 11 before row 14.
    assert_eq!(
        comment_outcome(&post(ResultCode::IdentityRequired, pending()), ACCEPTED),
        CommentOutcome::SavedNeedsIdentity
    );
    assert_eq!(
        comment_outcome(&post(ResultCode::IdentityRequired, nothing()), ACCEPTED),
        CommentOutcome::NotSavedIdentity
    );
    // Row 10 before row 14: stopped after writing, before committing.
    let written = effects(
        WriteEffect::Written,
        CheckpointEffect::Pending,
        PublicationEffect::NotRequested,
    );
    assert_eq!(
        comment_outcome(&post(ResultCode::Cancelled, written.clone()), ACCEPTED),
        CommentOutcome::NotSavedStopped
    );
    // Row 12 before row 13: an occupied path on a re-send.
    assert_eq!(
        comment_outcome(&post(ResultCode::OccupiedPath, nothing()), RESENT),
        CommentOutcome::NotSavedConflicting
    );
    // Row 13 before row 14: an internal error on a re-send after writing.
    assert_eq!(
        comment_outcome(&post(ResultCode::InternalError, written), RESENT),
        CommentOutcome::MaybeSaved
    );
    // Sync: published before cancelled, cancelled before conflict data.
    let mut cancelled = share(ResultCode::Cancelled, pending());
    cancelled.data = Some(conflict_data());
    assert_eq!(
        sync_outcome(&cancelled, ShareTarget::Item, RecordState::Finished),
        SyncOutcome::Stopped { completed: true }
    );
    let conflicted = share(ResultCode::UnlockRequired, pending()).with_data(conflict_data());
    assert_eq!(
        sync_outcome(&conflicted, ShareTarget::Item, RecordState::Accepted),
        SyncOutcome::Conflict { next: next() }
    );
}

/// A stop that came after the push still reports the work shared.
#[test]
fn a_stop_after_the_push_is_shared() {
    let comment_envelope = post(ResultCode::Cancelled, saved(PublicationEffect::Published));
    assert_eq!(
        comment_outcome(&comment_envelope, FINISHED),
        CommentOutcome::Shared
    );
    let sync = share(ResultCode::Cancelled, saved(PublicationEffect::Published));
    assert_eq!(
        sync_outcome(&sync, ShareTarget::Item, RecordState::Finished),
        SyncOutcome::Published
    );
}

// --- sync_outcome, row by row ---

fn conflict_data() -> MutationDataDto {
    MutationDataDto::SynchronizationConflict(SynchronizationConflictMutationDto {
        r#type: SynchronizationConflictType::SynchronizationConflict,
        conflict: next(),
    })
}

fn synced(publication: PublicationEffect) -> Effects {
    effects(
        WriteEffect::NotRequested,
        CheckpointEffect::NotRequested,
        publication,
    )
}

#[test]
fn sync_published_is_published() {
    let envelope = share(ResultCode::Ok, synced(PublicationEffect::Published));
    let outcome = sync_outcome(&envelope, ShareTarget::Main, RecordState::Finished);
    assert_eq!(outcome, SyncOutcome::Published);
    assert!(outcome.offers(RecordState::Finished).is_empty());
}

#[test]
fn sync_current_is_current() {
    let envelope = share(ResultCode::Ok, synced(PublicationEffect::Current));
    let outcome = sync_outcome(&envelope, ShareTarget::Item, RecordState::Finished);
    assert_eq!(outcome, SyncOutcome::Current);
    assert!(outcome.offers(RecordState::Finished).is_empty());
}

#[test]
fn sync_cancelled_is_stopped_and_says_what_completed() {
    let before = share(ResultCode::Cancelled, synced(PublicationEffect::Pending));
    let outcome = sync_outcome(&before, ShareTarget::Item, RecordState::Finished);
    assert_eq!(outcome, SyncOutcome::Stopped { completed: false });
    assert_eq!(outcome.offers(RecordState::Finished), [S::Share]);
    let after = share(ResultCode::Cancelled, pending());
    assert_eq!(
        sync_outcome(&after, ShareTarget::Item, RecordState::Finished),
        SyncOutcome::Stopped { completed: true }
    );
}

#[test]
fn sync_conflict_data_is_a_conflict_with_no_action() {
    let envelope = share(
        ResultCode::RecoveryRequired,
        synced(PublicationEffect::Pending),
    )
    .with_data(conflict_data());
    let outcome = sync_outcome(&envelope, ShareTarget::Item, RecordState::Finished);
    assert_eq!(outcome, SyncOutcome::Conflict { next: next() });
    assert!(outcome.offers(RecordState::Finished).is_empty());
}

#[test]
fn sync_a_deleted_branch_offers_give_up_and_no_share() {
    let envelope = share(ResultCode::RemoteBranchDeleted, nothing());
    let outcome = sync_outcome(&envelope, ShareTarget::Item, RecordState::Accepted);
    assert_eq!(outcome, SyncOutcome::BranchDeleted);
    assert_eq!(outcome.offers(RecordState::Accepted), [S::GiveUp]);
    assert!(outcome.offers(RecordState::Finished).is_empty());
}

#[test]
fn sync_a_locked_key_needs_unlock() {
    for code in [ResultCode::UnlockRequired, ResultCode::UnlockFailed] {
        let outcome = sync_outcome(
            &share(code, nothing()),
            ShareTarget::Main,
            RecordState::Accepted,
        );
        assert_eq!(outcome, SyncOutcome::NeedsUnlock);
        assert_eq!(
            outcome.offers(RecordState::Accepted),
            [S::Unlock, S::GiveUp]
        );
        assert_eq!(outcome.offers(RecordState::Finished), [S::Unlock]);
    }
}

#[test]
fn sync_a_key_problem_needs_a_key() {
    for code in [
        ResultCode::NoSelectedKey,
        ResultCode::KeyUnavailable,
        ResultCode::KeyRejected,
    ] {
        let outcome = sync_outcome(
            &share(code, nothing()),
            ShareTarget::Item,
            RecordState::Finished,
        );
        assert_eq!(outcome, SyncOutcome::NeedsKey);
        assert_eq!(
            outcome.offers(RecordState::Finished),
            [S::OpenSharingAndKeys]
        );
    }
}

#[test]
fn sync_a_host_problem_needs_the_host_it_names() {
    for (code, kind) in [
        (
            ResultCode::HostApprovalRequired,
            RecoveryActionKind::HostApprove,
        ),
        (
            ResultCode::HostReplacementRequired,
            RecoveryActionKind::HostReplace,
        ),
    ] {
        let envelope = share(code, nothing()).with_recovery(vec![RecoveryAction::new(
            kind,
            [
                ("authority", "git.example.com:2222".into()),
                ("root", "/fixture/repo".into()),
            ],
        )]);
        let outcome = sync_outcome(&envelope, ShareTarget::Item, RecordState::Finished);
        assert_eq!(
            outcome,
            SyncOutcome::NeedsHost {
                authority: Some(SshAuthority {
                    host: "git.example.com".to_owned(),
                    port: 2222,
                })
            }
        );
        assert_eq!(outcome.offers(RecordState::Finished), [S::CheckHost]);
        // With no host data it names none.
        assert_eq!(
            sync_outcome(
                &share(code, nothing()),
                ShareTarget::Item,
                RecordState::Finished
            ),
            SyncOutcome::NeedsHost { authority: None }
        );
    }
}

#[test]
fn sync_identity_required_needs_an_identity() {
    let envelope = share(ResultCode::IdentityRequired, nothing());
    let outcome = sync_outcome(&envelope, ShareTarget::Main, RecordState::Accepted);
    assert_eq!(outcome, SyncOutcome::NeedsIdentity);
    assert_eq!(
        outcome.offers(RecordState::Accepted),
        [S::SetIdentity, S::Retry, S::GiveUp]
    );
}

#[test]
fn sync_busy_offers_its_holder_and_retry() {
    let envelope = share(ResultCode::Busy, nothing());
    let outcome = sync_outcome(&envelope, ShareTarget::Item, RecordState::Finished);
    assert_eq!(outcome, SyncOutcome::Busy);
    assert_eq!(
        outcome.offers(RecordState::Finished),
        [S::ShowHolder, S::Retry]
    );
}

#[test]
fn sync_without_a_publication_remote_has_no_remote() {
    let envelope = share(ResultCode::PublicationRemoteRequired, nothing());
    let outcome = sync_outcome(&envelope, ShareTarget::Main, RecordState::Finished);
    assert_eq!(outcome, SyncOutcome::NoRemote);
    assert_eq!(outcome.offers(RecordState::Finished), [S::SetUpSharing]);
}

#[test]
fn item_not_found_on_share_item_is_nothing_to_share() {
    let envelope = share(ResultCode::ItemNotFound, nothing());
    let outcome = sync_outcome(&envelope, ShareTarget::Item, RecordState::Finished);
    assert_eq!(outcome, SyncOutcome::NothingToShare);
    assert_eq!(
        NOTHING_TO_SHARE,
        "Nothing to share yet: this item has no saved changes."
    );
    assert!(outcome.offers(RecordState::Finished).is_empty());
    // Share main has no item to miss: it is an ordinary failure.
    assert_eq!(
        sync_outcome(&envelope, ShareTarget::Main, RecordState::Finished),
        SyncOutcome::Failed {
            code: ResultCode::ItemNotFound,
            retry: false
        }
    );
}

#[test]
fn sync_otherwise_failed_retries_while_owed_and_shares_when_final() {
    let envelope = share(ResultCode::PushRejected, nothing());
    let owed = sync_outcome(&envelope, ShareTarget::Item, RecordState::Accepted);
    assert_eq!(
        owed,
        SyncOutcome::Failed {
            code: ResultCode::PushRejected,
            retry: true
        }
    );
    assert_eq!(owed.offers(RecordState::Accepted), [S::Retry, S::GiveUp]);
    let finished = sync_outcome(&envelope, ShareTarget::Item, RecordState::Finished);
    assert_eq!(
        finished,
        SyncOutcome::Failed {
            code: ResultCode::PushRejected,
            retry: false
        }
    );
    assert_eq!(finished.offers(RecordState::Finished), [S::Share]);
    // A transient code is retried whatever the record (fact 14).
    let transient = sync_outcome(
        &share(ResultCode::RemoteUnavailable, nothing()),
        ShareTarget::Main,
        RecordState::Finished,
    );
    assert_eq!(
        transient,
        SyncOutcome::Failed {
            code: ResultCode::RemoteUnavailable,
            retry: true
        }
    );
    assert_eq!(transient.offers(RecordState::Finished), [S::Retry]);
}
