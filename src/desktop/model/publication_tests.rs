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
    Attempt, CommentOffer as C, CommentOutcome, NOTHING_TO_SHARE, ShareAttempt, ShareTarget,
    SyncOffer as S, SyncOutcome, comment_outcome, sync_outcome,
};
use crate::desktop::services::worker::RecordState;

const fn attempt(resend: bool, record: RecordState) -> Attempt {
    Attempt {
        resend,
        record,
        after_give_up: false,
    }
}

const ACCEPTED: Attempt = attempt(false, RecordState::Accepted);
const FINISHED: Attempt = attempt(false, RecordState::Finished);
const UNKNOWN: Attempt = attempt(false, RecordState::Unknown);
const RESENT: Attempt = attempt(true, RecordState::Finished);

fn outcome(envelope: &Envelope<MutationDataDto>, attempt: Attempt) -> CommentOutcome {
    comment_outcome(envelope, attempt).outcome
}

fn offers(envelope: &Envelope<MutationDataDto>, attempt: Attempt) -> Vec<C> {
    comment_outcome(envelope, attempt).offers()
}

const fn share_attempt(target: ShareTarget, record: RecordState) -> ShareAttempt {
    ShareAttempt {
        target,
        record,
        after_give_up: false,
    }
}

fn sync(
    envelope: &Envelope<MutationDataDto>,
    target: ShareTarget,
    record: RecordState,
) -> SyncOutcome {
    sync_outcome(envelope, share_attempt(target, record)).outcome
}

fn sync_offers(
    envelope: &Envelope<MutationDataDto>,
    target: ShareTarget,
    record: RecordState,
) -> Vec<S> {
    sync_outcome(envelope, share_attempt(target, record)).offers()
}

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

fn pending() -> Effects {
    saved(PublicationEffect::Pending)
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

// --- comment_outcome, row by row ---

#[test]
fn row_0_request_mismatch_is_mismatched() {
    let unsaved = post(ResultCode::RequestMismatch, nothing());
    assert_eq!(
        outcome(&unsaved, ACCEPTED),
        CommentOutcome::Mismatched { saved: false }
    );
    assert_eq!(offers(&unsaved, ACCEPTED), [C::Discard, C::CopyText]);
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
        let result = comment_outcome(&committed, attempt);
        assert_eq!(result.outcome, CommentOutcome::Mismatched { saved: true });
        assert!(result.outcome.is_saved());
        assert_eq!(result.offers(), [C::Discard, C::CopyText]);
    }
}

#[test]
fn row_1_a_published_or_current_comment_is_shared() {
    for publication in [PublicationEffect::Published, PublicationEffect::Current] {
        let envelope = post(ResultCode::Ok, saved(publication));
        assert_eq!(outcome(&envelope, ACCEPTED), CommentOutcome::Shared);
        assert!(offers(&envelope, FINISHED).is_empty());
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
    assert_eq!(outcome(&envelope, FINISHED), CommentOutcome::SavedHere);
    assert!(offers(&envelope, FINISHED).is_empty());
}

#[test]
fn row_3_a_saved_stopped_post_offers_share() {
    let envelope = post(ResultCode::Cancelled, pending());
    assert_eq!(outcome(&envelope, FINISHED), CommentOutcome::SavedStopped);
    assert_eq!(offers(&envelope, FINISHED), [C::Share]);
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
    assert_eq!(
        outcome(&envelope, ACCEPTED),
        CommentOutcome::SavedConflict { next: next() }
    );
    assert!(offers(&envelope, ACCEPTED).is_empty());
}

#[test]
fn row_5_a_deleted_branch_offers_give_up_and_no_share() {
    let envelope = post(ResultCode::RemoteBranchDeleted, pending());
    assert_eq!(
        outcome(&envelope, ACCEPTED),
        CommentOutcome::SavedBranchDeleted
    );
    assert_eq!(offers(&envelope, ACCEPTED), [C::GiveUp]);
    assert!(offers(&envelope, FINISHED).is_empty());
}

#[test]
fn row_6_a_locked_key_offers_unlock_and_give_up() {
    for code in [ResultCode::UnlockRequired, ResultCode::UnlockFailed] {
        let envelope = post(code, pending());
        assert_eq!(
            outcome(&envelope, ACCEPTED),
            CommentOutcome::SavedNeedsUnlock
        );
        assert_eq!(offers(&envelope, ACCEPTED), [C::Unlock, C::GiveUp]);
    }
}

#[test]
fn row_7_a_key_problem_offers_sharing_and_keys() {
    for code in [
        ResultCode::NoSelectedKey,
        ResultCode::KeyUnavailable,
        ResultCode::KeyRejected,
    ] {
        let envelope = post(code, pending());
        assert_eq!(outcome(&envelope, ACCEPTED), CommentOutcome::SavedNeedsKey);
        assert_eq!(
            offers(&envelope, ACCEPTED),
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
        let envelope = post(code, pending());
        assert_eq!(outcome(&envelope, ACCEPTED), CommentOutcome::SavedNeedsHost);
        assert_eq!(
            offers(&envelope, ACCEPTED),
            [C::CheckHost, C::RetrySharing, C::GiveUp]
        );
    }
}

/// Row 9c: a saved comment whose child merge stopped on the identity.
#[test]
fn identity_required_routes_to_set_identity() {
    let envelope = post(ResultCode::IdentityRequired, pending());
    assert_eq!(
        outcome(&envelope, ACCEPTED),
        CommentOutcome::SavedNeedsIdentity
    );
    assert_eq!(
        offers(&envelope, ACCEPTED),
        [C::SetIdentity, C::RetrySharing, C::GiveUp]
    );
}

/// Rows 9a and 9b.
#[test]
fn an_owed_unshared_comment_offers_retry_and_a_final_one_offers_share() {
    let envelope = post(ResultCode::TransportUnavailable, pending());
    assert_eq!(
        outcome(&envelope, ACCEPTED),
        CommentOutcome::SavedNotShared {
            code: ResultCode::TransportUnavailable
        }
    );
    assert_eq!(offers(&envelope, ACCEPTED), [C::RetrySharing, C::GiveUp]);
    assert_eq!(
        outcome(&envelope, FINISHED),
        CommentOutcome::SavedNotSharedFinal {
            code: ResultCode::TransportUnavailable
        }
    );
    assert_eq!(offers(&envelope, FINISHED), [C::Share]);
}

#[test]
fn row_10_a_post_stopped_before_its_checkpoint_offers_edit_text() {
    let envelope = post(ResultCode::Cancelled, nothing());
    assert_eq!(
        outcome(&envelope, FINISHED),
        CommentOutcome::NotSavedStopped
    );
    assert_eq!(offers(&envelope, FINISHED), [C::EditText]);
}

#[test]
fn row_11_an_unsaved_post_needing_an_identity_offers_set_identity() {
    let envelope = post(ResultCode::IdentityRequired, nothing());
    assert_eq!(
        outcome(&envelope, ACCEPTED),
        CommentOutcome::NotSavedIdentity
    );
    assert_eq!(
        offers(&envelope, ACCEPTED),
        [C::SetIdentity, C::Retry, C::Discard, C::CopyText]
    );
}

#[test]
fn row_12_an_occupied_path_keeps_the_text_and_offers_discard() {
    let envelope = post(ResultCode::OccupiedPath, nothing());
    assert_eq!(
        outcome(&envelope, FINISHED),
        CommentOutcome::NotSavedConflicting
    );
    assert_eq!(offers(&envelope, FINISHED), [C::Discard]);
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
        let resent = attempt(true, record);
        assert_eq!(outcome(&envelope, resent), CommentOutcome::MaybeSaved);
        assert_eq!(offers(&envelope, resent), [C::CheckAgain]);
    }
    assert_eq!(
        outcome(&envelope, FINISHED),
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
    for (envelope, attempt) in [(&written, ACCEPTED), (&accepted, ACCEPTED)] {
        assert_eq!(
            outcome(envelope, attempt),
            CommentOutcome::NotSavedOwed {
                code: ResultCode::TransportUnavailable
            }
        );
        assert_eq!(
            offers(envelope, attempt),
            [C::Retry, C::Discard, C::CopyText]
        );
    }
}

#[test]
fn row_15_an_unwritten_finished_failure_offers_edit_text() {
    let envelope = post(ResultCode::TransportUnavailable, nothing());
    assert_eq!(
        outcome(&envelope, FINISHED),
        CommentOutcome::NotSaved {
            code: ResultCode::TransportUnavailable
        }
    );
    assert_eq!(offers(&envelope, FINISHED), [C::EditText]);
}

/// P17: where the record stands is unknown, a post offers Retry and
/// Check again, never Edit text or Give up.
#[test]
fn an_unknown_record_offers_retry_and_check_again_only() {
    let unsaved = post(ResultCode::TransportUnavailable, nothing());
    assert_eq!(
        outcome(&unsaved, UNKNOWN),
        CommentOutcome::NotSavedOwed {
            code: ResultCode::TransportUnavailable
        }
    );
    assert_eq!(offers(&unsaved, UNKNOWN), [C::Retry, C::CheckAgain]);
    let saved = post(ResultCode::TransportUnavailable, pending());
    assert_eq!(
        outcome(&saved, UNKNOWN),
        CommentOutcome::SavedNotShared {
            code: ResultCode::TransportUnavailable
        }
    );
    assert_eq!(offers(&saved, UNKNOWN), [C::RetrySharing, C::CheckAgain]);
    let locked = post(ResultCode::UnlockRequired, pending());
    assert_eq!(offers(&locked, UNKNOWN), [C::Unlock, C::CheckAgain]);
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
        let envelope = post(code, pending());
        assert!(!offers(&envelope, FINISHED).contains(&C::GiveUp));
        assert!(!offers(&envelope, UNKNOWN).contains(&C::GiveUp));
        assert!(offers(&envelope, ACCEPTED).contains(&C::GiveUp));
    }
}

/// Orchestrator ruling: a finished record only replays, so no row
/// offers the same request on it (Retry sharing, Unlock…, Retry); a new
/// share takes its place. Only the transient codes keep Retry (fact 14).
#[test]
fn a_finished_record_never_offers_a_same_request_resend() {
    // Row 14: written, not committed, record finished.
    let written = post(
        ResultCode::TransportUnavailable,
        effects(
            WriteEffect::Written,
            CheckpointEffect::Pending,
            PublicationEffect::NotRequested,
        ),
    );
    assert_eq!(
        outcome(&written, FINISHED),
        CommentOutcome::NotSavedOwed {
            code: ResultCode::TransportUnavailable
        }
    );
    assert_eq!(
        offers(&written, FINISHED),
        [C::Discard, C::CopyText, C::CheckAgain]
    );
    let comment_rows = [
        (ResultCode::UnlockRequired, vec![C::Share]),
        (
            ResultCode::NoSelectedKey,
            vec![C::OpenSharingAndKeys, C::Share],
        ),
        (
            ResultCode::HostApprovalRequired,
            vec![C::CheckHost, C::Share],
        ),
        (ResultCode::IdentityRequired, vec![C::SetIdentity, C::Share]),
    ];
    for (code, expected) in comment_rows {
        assert_eq!(
            offers(&post(code, pending()), FINISHED),
            expected,
            "{code:?}"
        );
    }
    let sync_rows = [
        (ResultCode::UnlockRequired, vec![S::Share]),
        (ResultCode::IdentityRequired, vec![S::SetIdentity, S::Share]),
        (
            ResultCode::KeyRejected,
            vec![S::OpenSharingAndKeys, S::Share],
        ),
        (
            ResultCode::HostReplacementRequired,
            vec![S::CheckHost, S::Share],
        ),
        (ResultCode::PushRejected, vec![S::Share]),
    ];
    for (code, expected) in sync_rows {
        let envelope = share(code, nothing());
        assert_eq!(
            sync_offers(&envelope, ShareTarget::Item, RecordState::Finished),
            expected,
            "{code:?}"
        );
    }
    for code in [
        ResultCode::Busy,
        ResultCode::PollYielding,
        ResultCode::RemoteUnavailable,
        ResultCode::TransportUnavailable,
        ResultCode::RegistrationPending,
    ] {
        let offered = sync_offers(
            &share(code, nothing()),
            ShareTarget::Main,
            RecordState::Finished,
        );
        assert!(offered.contains(&S::Retry), "{code:?}");
    }
}

/// P14: the answer of a Give up never offers Give up again; the row's
/// own way to finish stays.
#[test]
fn after_a_give_up_give_up_is_not_offered_again() {
    let given_up = Attempt {
        after_give_up: true,
        ..ACCEPTED
    };
    let locked = post(ResultCode::UnlockRequired, pending());
    assert_eq!(offers(&locked, given_up), [C::Unlock]);
    let key = post(ResultCode::NoSelectedKey, pending());
    assert_eq!(
        offers(&key, given_up),
        [C::OpenSharingAndKeys, C::RetrySharing]
    );
    let transient = post(ResultCode::TransportUnavailable, pending());
    assert_eq!(offers(&transient, given_up), [C::RetrySharing]);
    let unknown = Attempt {
        after_give_up: true,
        ..UNKNOWN
    };
    assert_eq!(offers(&locked, unknown), [C::Unlock, C::CheckAgain]);

    let share_given_up = ShareAttempt {
        after_give_up: true,
        ..share_attempt(ShareTarget::Item, RecordState::Accepted)
    };
    let cases = [
        (ResultCode::UnlockRequired, vec![S::Unlock]),
        (ResultCode::KeyRejected, vec![S::OpenSharingAndKeys]),
        (ResultCode::TransportUnavailable, vec![S::Retry]),
        // E-6: Republish… after a Give up (D5 Part C).
        (ResultCode::RemoteBranchDeleted, vec![S::Republish]),
    ];
    for (code, expected) in cases {
        let result = sync_outcome(&share(code, nothing()), share_given_up);
        assert_eq!(result.offers(), expected, "{code:?}");
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
        outcome(&mismatch, ACCEPTED),
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
    assert_eq!(outcome(&no_remote, FINISHED), CommentOutcome::SavedHere);
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
    assert_eq!(outcome(&stopped, FINISHED), CommentOutcome::SavedStopped);
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
        outcome(&conflict, ACCEPTED),
        CommentOutcome::SavedConflict { next: next() }
    );
    // Row 9c before row 9a; row 11 before row 14.
    assert_eq!(
        outcome(&post(ResultCode::IdentityRequired, pending()), ACCEPTED),
        CommentOutcome::SavedNeedsIdentity
    );
    assert_eq!(
        outcome(&post(ResultCode::IdentityRequired, nothing()), ACCEPTED),
        CommentOutcome::NotSavedIdentity
    );
    // Row 10 before row 14: stopped after writing, before committing.
    let written = effects(
        WriteEffect::Written,
        CheckpointEffect::Pending,
        PublicationEffect::NotRequested,
    );
    assert_eq!(
        outcome(&post(ResultCode::Cancelled, written.clone()), ACCEPTED),
        CommentOutcome::NotSavedStopped
    );
    // Row 12 before row 13: an occupied path on a re-send.
    assert_eq!(
        outcome(&post(ResultCode::OccupiedPath, nothing()), RESENT),
        CommentOutcome::NotSavedConflicting
    );
    // Row 13 before row 14: an internal error on a re-send after writing.
    assert_eq!(
        outcome(&post(ResultCode::InternalError, written), RESENT),
        CommentOutcome::MaybeSaved
    );
    // Sync: cancelled before conflict data, conflict data before codes.
    let mut cancelled = share(ResultCode::Cancelled, pending());
    cancelled.data = Some(conflict_data());
    assert_eq!(
        sync(&cancelled, ShareTarget::Item, RecordState::Finished),
        SyncOutcome::Stopped { completed: true }
    );
    let conflicted = share(ResultCode::UnlockRequired, pending()).with_data(conflict_data());
    assert_eq!(
        sync(&conflicted, ShareTarget::Item, RecordState::Accepted),
        SyncOutcome::Conflict { next: next() }
    );
}

/// A stop that came after the push still reports the work shared.
#[test]
fn a_stop_after_the_push_is_shared() {
    let posted = post(ResultCode::Cancelled, saved(PublicationEffect::Published));
    assert_eq!(outcome(&posted, FINISHED), CommentOutcome::Shared);
    let synced = share(ResultCode::Cancelled, saved(PublicationEffect::Published));
    assert_eq!(
        sync(&synced, ShareTarget::Item, RecordState::Finished),
        SyncOutcome::Published
    );
}

// --- sync_outcome, row by row ---

#[test]
fn sync_published_is_published() {
    let envelope = share(ResultCode::Ok, synced(PublicationEffect::Published));
    assert_eq!(
        sync(&envelope, ShareTarget::Main, RecordState::Finished),
        SyncOutcome::Published
    );
    assert!(sync_offers(&envelope, ShareTarget::Main, RecordState::Finished).is_empty());
}

#[test]
fn sync_current_is_current() {
    let envelope = share(ResultCode::Ok, synced(PublicationEffect::Current));
    assert_eq!(
        sync(&envelope, ShareTarget::Item, RecordState::Finished),
        SyncOutcome::Current
    );
    assert!(sync_offers(&envelope, ShareTarget::Item, RecordState::Finished).is_empty());
}

#[test]
fn sync_cancelled_is_stopped_and_says_what_completed() {
    let before = share(ResultCode::Cancelled, synced(PublicationEffect::Pending));
    assert_eq!(
        sync(&before, ShareTarget::Item, RecordState::Finished),
        SyncOutcome::Stopped { completed: false }
    );
    assert_eq!(
        sync_offers(&before, ShareTarget::Item, RecordState::Finished),
        [S::Share]
    );
    let after = share(ResultCode::Cancelled, pending());
    assert_eq!(
        sync(&after, ShareTarget::Item, RecordState::Finished),
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
    assert_eq!(
        sync(&envelope, ShareTarget::Item, RecordState::Finished),
        SyncOutcome::Conflict { next: next() }
    );
    assert!(sync_offers(&envelope, ShareTarget::Item, RecordState::Finished).is_empty());
}

/// E-6 (D5 Part C): an item's deleted shared branch offers Stop and
/// republish… beside Give up while its record is accepted, Republish… once
/// it is not, and never Share. A share of main has no item to republish.
#[test]
fn sync_a_deleted_branch_offers_give_up_and_no_share() {
    let envelope = share(ResultCode::RemoteBranchDeleted, nothing());
    assert_eq!(
        sync(&envelope, ShareTarget::Item, RecordState::Accepted),
        SyncOutcome::BranchDeleted
    );
    assert_eq!(
        sync_offers(&envelope, ShareTarget::Item, RecordState::Accepted),
        [S::StopAndRepublish, S::GiveUp]
    );
    assert_eq!(
        sync_offers(&envelope, ShareTarget::Item, RecordState::Finished),
        [S::Republish]
    );
    assert_eq!(
        sync_offers(&envelope, ShareTarget::Item, RecordState::Unknown),
        [S::Republish]
    );
    assert_eq!(
        sync_offers(&envelope, ShareTarget::Main, RecordState::Accepted),
        [S::GiveUp]
    );
    assert!(sync_offers(&envelope, ShareTarget::Main, RecordState::Finished).is_empty());
}

#[test]
fn sync_a_locked_key_needs_unlock() {
    for code in [ResultCode::UnlockRequired, ResultCode::UnlockFailed] {
        let envelope = share(code, nothing());
        assert_eq!(
            sync(&envelope, ShareTarget::Main, RecordState::Accepted),
            SyncOutcome::NeedsUnlock
        );
        assert_eq!(
            sync_offers(&envelope, ShareTarget::Main, RecordState::Accepted),
            [S::Unlock, S::GiveUp]
        );
    }
}

#[test]
fn sync_a_key_problem_needs_a_key() {
    for code in [
        ResultCode::NoSelectedKey,
        ResultCode::KeyUnavailable,
        ResultCode::KeyRejected,
    ] {
        let envelope = share(code, nothing());
        assert_eq!(
            sync(&envelope, ShareTarget::Item, RecordState::Accepted),
            SyncOutcome::NeedsKey
        );
        assert_eq!(
            sync_offers(&envelope, ShareTarget::Item, RecordState::Accepted),
            [S::OpenSharingAndKeys, S::GiveUp]
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
        assert_eq!(
            sync(&envelope, ShareTarget::Item, RecordState::Accepted),
            SyncOutcome::NeedsHost {
                authority: Some(SshAuthority {
                    host: "git.example.com".to_owned(),
                    port: 2222,
                })
            }
        );
        assert_eq!(
            sync_offers(&envelope, ShareTarget::Item, RecordState::Accepted),
            [S::CheckHost, S::GiveUp]
        );
        // With no host data it names none.
        assert_eq!(
            sync(
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
    assert_eq!(
        sync(&envelope, ShareTarget::Main, RecordState::Accepted),
        SyncOutcome::NeedsIdentity
    );
    assert_eq!(
        sync_offers(&envelope, ShareTarget::Main, RecordState::Accepted),
        [S::SetIdentity, S::Retry, S::GiveUp]
    );
}

#[test]
fn sync_busy_offers_its_holder_and_retry() {
    let envelope = share(ResultCode::Busy, nothing());
    assert_eq!(
        sync(&envelope, ShareTarget::Item, RecordState::Finished),
        SyncOutcome::Busy
    );
    assert_eq!(
        sync_offers(&envelope, ShareTarget::Item, RecordState::Finished),
        [S::ShowHolder, S::Retry]
    );
}

#[test]
fn sync_without_a_publication_remote_has_no_remote() {
    let envelope = share(ResultCode::PublicationRemoteRequired, nothing());
    assert_eq!(
        sync(&envelope, ShareTarget::Main, RecordState::Finished),
        SyncOutcome::NoRemote
    );
    assert_eq!(
        sync_offers(&envelope, ShareTarget::Main, RecordState::Finished),
        [S::SetUpSharing]
    );
}

#[test]
fn item_not_found_on_share_item_is_nothing_to_share() {
    let envelope = share(ResultCode::ItemNotFound, nothing());
    assert_eq!(
        sync(&envelope, ShareTarget::Item, RecordState::Finished),
        SyncOutcome::NothingToShare
    );
    assert_eq!(
        NOTHING_TO_SHARE,
        "Nothing to share yet: this item has no saved changes."
    );
    assert!(sync_offers(&envelope, ShareTarget::Item, RecordState::Finished).is_empty());
    // Share main has no item to miss: it is an ordinary failure.
    assert_eq!(
        sync(&envelope, ShareTarget::Main, RecordState::Finished),
        SyncOutcome::Failed {
            code: ResultCode::ItemNotFound,
            retry: false
        }
    );
}

#[test]
fn sync_otherwise_failed_retries_while_owed_and_shares_when_final() {
    let envelope = share(ResultCode::PushRejected, nothing());
    assert_eq!(
        sync(&envelope, ShareTarget::Item, RecordState::Accepted),
        SyncOutcome::Failed {
            code: ResultCode::PushRejected,
            retry: true
        }
    );
    assert_eq!(
        sync_offers(&envelope, ShareTarget::Item, RecordState::Accepted),
        [S::Retry, S::GiveUp]
    );
    assert_eq!(
        sync(&envelope, ShareTarget::Item, RecordState::Finished),
        SyncOutcome::Failed {
            code: ResultCode::PushRejected,
            retry: false
        }
    );
    assert_eq!(
        sync_offers(&envelope, ShareTarget::Item, RecordState::Finished),
        [S::Share]
    );
    // A transient code is retried whatever the record (fact 14).
    let transient = share(ResultCode::RemoteUnavailable, nothing());
    assert_eq!(
        sync(&transient, ShareTarget::Main, RecordState::Finished),
        SyncOutcome::Failed {
            code: ResultCode::RemoteUnavailable,
            retry: true
        }
    );
    assert_eq!(
        sync_offers(&transient, ShareTarget::Main, RecordState::Finished),
        [S::Retry]
    );
}
