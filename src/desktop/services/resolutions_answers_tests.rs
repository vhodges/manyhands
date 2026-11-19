//! What a resolution's answer, its removal and the launch sweep mean for
//! its kept set (R-25; Task 11a review fixes M-1 and M-3).
//!
//! The SSH harness does not reach the desktop's tests, so a remote
//! resolution's answer is canned (`hold_job_return_answering`) and so is a
//! remote conflict read (`answer_next_conflict_read`), each built from
//! library types and naming the library test that produces the real one.
//! The real call still runs first, against a real repository and an
//! operation it does not hold, and is refused with no effect. Local
//! answers are real.

use std::fs;

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{
        ConflictDto, ConflictReadEligibility, ConflictReadNextAction, ConflictStage, ConflictState,
        ConflictTarget, FailurePoint, MutationDataDto, OperationId, RepositoryService, RequestId,
        RequestState as LibraryState,
    },
    results::{
        CheckpointEffect, DiscoveryEffect, Effects, Envelope, IntegrationEffect, PublicationEffect,
        RecoveryAction, RecoveryActionKind, ResultCode, Scope, WriteEffect,
    },
};

use super::tests::{a_local_resolution, a_set, fields, finished, kept, start, until};
use crate::desktop::{
    services::{
        BUSY_RETRY_DELAYS, JobKind, Services,
        conflicts::ConflictSource,
        requests::{DesktopRequestId, RequestState},
    },
    test_support::{delaying::DelayingBackend, fixtures::Fixture},
};

/// A canned `conflict resolve` answer with `code` and `effects`, as
/// `Envelope::classified_mutation` gives the library's.
fn answer(code: ResultCode, effects: Effects) -> Envelope<MutationDataDto> {
    Envelope::classified_mutation(
        "conflict resolve",
        Scope::default(),
        code == ResultCode::Cancelled,
        code,
        effects,
        false,
    )
}

/// The effects of a resolution checkpointed and not yet published, as
/// `bind/conflict.rs` `resolved` gives them.
fn checkpointed() -> Effects {
    Effects {
        write: WriteEffect::Written,
        checkpoint: CheckpointEffect::Committed,
        integration: IntegrationEffect::Complete,
        publication: PublicationEffect::Pending,
        discovery: DiscoveryEffect::Pending,
        commit_oid: Some("3".repeat(40)),
        ..Effects::not_requested()
    }
}

/// Sends a remote set for an operation `root` does not hold, answered
/// `canned` in place of the library's refusal, and waits for its answer
/// and what follows it.
async fn answered(
    backend: &DelayingBackend,
    services: &Services,
    root: &std::path::Path,
    canned: Envelope<MutationDataDto>,
    cx: &mut TestAppContext,
) -> RequestId {
    let set = a_set(root, OperationId::new(), ConflictSource::Remote, "Mine");
    answered_set(backend, services, set, canned, cx).await
}

/// `answered` of `set`.
async fn answered_set(
    backend: &DelayingBackend,
    services: &Services,
    set: super::KeptSet,
    canned: Envelope<MutationDataDto>,
    cx: &mut TestAppContext,
) -> RequestId {
    let kind = match set.source {
        ConflictSource::Remote => JobKind::ConflictResolve,
        ConflictSource::Local => JobKind::LocalContextResolve,
    };
    let request = set.request_id;
    // The worker takes a `busy` job again before its answer stands.
    let attempts = if canned.code == ResultCode::Busy {
        BUSY_RETRY_DELAYS.len() + 1
    } else {
        1
    };
    for _ in 0..attempts {
        backend
            .hold_job_return_answering(kind, canned.clone())
            .release();
    }
    let submitted = cx.update(|cx| services.submit_resolution(set, cx));
    cx.run_until_parked();
    let id = submitted.await.expect("kept");
    finished(services, id, cx).await;
    cx.run_until_parked();
    request
}

/// R-25: a remote answer after which the request's attempt may still
/// stand keeps its set, since a later request is answered
/// `original_request_required` naming it, and only this set finishes it.
///
/// Canned: a failure after the paths were written is the answer
/// `tests/mutation_remote/resolution.rs` `c4_resolution_after_paths_written`
/// produces (`Partial`, a failure code, the paths written; `conflict show`
/// then names the request), as `tests/mutation_remote/conflict_read.rs`
/// `c4_conflict_read_resolution_prepared` reads it. `external_change` with
/// nothing written is what the same request is answered in
/// `c4_resolution_original_request_after_an_edit` while its attempt
/// stands; `recovery_required` with nothing written is what
/// `c4_resolution_before_any_path_write` answers, with the request
/// recorded as its attempt's owner.
#[gpui_kit::test]
async fn a_remote_failure_after_a_write_keeps_its_set(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let (backend, services) = start(&fixture, cx);
    let store = fixture.resolution_store();
    for canned in [
        answer(
            ResultCode::InternalError,
            Effects {
                write: WriteEffect::Written,
                ..Effects::not_requested()
            },
        ),
        answer(ResultCode::ExternalChange, Effects::not_requested()),
        answer(ResultCode::RecoveryRequired, Effects::not_requested()),
    ] {
        let code = canned.code;
        let request = answered(&backend, &services, &root, canned, cx).await;
        assert!(
            store.kept(request).unwrap().is_some(),
            "{} keeps the set",
            code.as_str()
        );
    }
}

/// R-25: a refusal that wrote nothing and left this request no attempt
/// deletes its set: `original_request_required`, which a new request is
/// answered before it owns an attempt or once it is disowned
/// (`bind/conflict.rs` `fresh`).
///
/// Canned: the answer `tests/mutation_remote/resolution.rs`
/// `c4_resolution_new_request_after_a_written_path_is_edited`
/// (`superseding_after_an_edit`) produces: `Blocked`, nothing written, no
/// record, and Retry of the attempt's own request.
#[gpui_kit::test]
async fn a_remote_refusal_before_a_write_deletes_its_set(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let (backend, services) = start(&fixture, cx);
    let store = fixture.resolution_store();
    let canned = answer(
        ResultCode::OriginalRequestRequired,
        Effects::not_requested(),
    )
    .with_recovery(vec![RecoveryAction::new(
        RecoveryActionKind::RequestRetry,
        [("request_id", RequestId::new().to_string().into())],
    )]);
    let request = answered(&backend, &services, &root, canned, cx).await;
    assert!(store.kept(request).unwrap().is_none());
}

/// A local resolution refused `busy` before the library accepted it left no
/// record, yet its Retry sends the same request again, and only a set kept
/// under that request carries the paths the sheet chose (V12): the set stays
/// until the retry's own answer settles it. A busy answer that offers no
/// Retry has nothing to send again, and a set no record stands behind goes.
///
/// Canned: `busy` is what a local resolution is answered while another
/// holder has the repository (`tests/local_sync` `busy`), with
/// `request.retry` naming the request.
#[gpui_kit::test]
async fn a_busy_local_refusal_that_offers_retry_keeps_its_set(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let (backend, services) = start(&fixture, cx);
    let store = fixture.resolution_store();
    let busy = |request: Option<RequestId>| {
        let answer = answer(ResultCode::Busy, Effects::not_requested());
        match request {
            Some(request) => answer.with_recovery(vec![RecoveryAction::new(
                RecoveryActionKind::RequestRetry,
                [("request_id", request.to_string().into())],
            )]),
            None => answer,
        }
    };
    let set = a_set(&root, OperationId::new(), ConflictSource::Local, "Mine");
    let offered = busy(Some(set.request_id));
    let request = answered_set(&backend, &services, set, offered, cx).await;
    assert!(
        store.kept(request).unwrap().is_some(),
        "busy with Retry keeps the set"
    );

    let set = a_set(&root, OperationId::new(), ConflictSource::Local, "Mine");
    let request = answered_set(&backend, &services, set, busy(None), cx).await;
    assert!(
        store.kept(request).unwrap().is_none(),
        "busy with nothing to retry settles as before"
    );
}

/// R-25: an answer that ends the attempt deletes its set: applied,
/// published by another request, or stopped at the call's token with the
/// synchronization left resolved for Resume.
///
/// Canned: `ok` as `tests/mutation_remote/resolution.rs`
/// `c4_resolution_publishes` answers; `already_applied` as
/// `c4_resolution_published_by_another_request`; `cancelled` with the
/// resolution checkpointed as `c4_resolution_cancelled_before_publication`.
#[gpui_kit::test]
async fn a_remote_answer_that_ends_its_attempt_deletes_its_set(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let (backend, services) = start(&fixture, cx);
    let store = fixture.resolution_store();
    let published = Effects {
        publication: PublicationEffect::Published,
        discovery: DiscoveryEffect::Current,
        ..checkpointed()
    };
    let current = Effects {
        publication: PublicationEffect::Current,
        commit_oid: Some("3".repeat(40)),
        ..Effects::not_requested()
    };
    for canned in [
        answer(ResultCode::Ok, published),
        answer(ResultCode::AlreadyApplied, current),
        answer(ResultCode::Cancelled, checkpointed()),
    ] {
        let code = canned.code;
        let request = answered(&backend, &services, &root, canned, cx).await;
        assert!(
            store.kept(request).unwrap().is_none(),
            "{} deletes the set",
            code.as_str()
        );
    }
}

/// A remote conflict read of `operation` in `state`, naming `request` as
/// the owner of its unfinished attempt.
fn conflict(
    operation: OperationId,
    state: ConflictState,
    next_action: ConflictReadNextAction,
    request: Option<RequestId>,
) -> ConflictDto {
    let unfinished = request.is_some();
    ConflictDto {
        operation_id: operation.to_string(),
        comment_operation_id: None,
        target: ConflictTarget::Primary,
        item_id: None,
        stage: ConflictStage::Primary,
        state,
        next_action,
        request_id: request.map(|request| request.to_string()),
        eligibility: unfinished.then_some(ConflictReadEligibility::EligibleCanonical),
        local_parent: "1".repeat(40),
        incoming_parent: "2".repeat(40),
        observation: unfinished.then(|| format!("conflict-v1:{}", "0".repeat(64))),
        historical: unfinished,
        paths: Vec::new(),
    }
}

/// R-25 at launch: a remote set the library has no record of is kept
/// while `show_conflict` names its request as the unfinished attempt's,
/// deleted once the conflict is resolved, and kept when the read fails.
///
/// Canned: the attempt that stands is `tests/mutation_remote/conflict_read.rs`
/// `c4_conflict_read_resolution_prepared` (`resolution_prepared`,
/// `retry_resolution`, historical, its request named); the resolved one is
/// `c4_conflict_read_resolved` (`resolved`, `resume`, no request). The
/// failed read is real: the repository holds no such operation.
#[gpui_kit::test]
async fn the_sweep_keeps_a_remote_set_while_its_attempt_stands(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let store = fixture.resolution_store();
    let stands = a_set(&root, OperationId::new(), ConflictSource::Remote, "Stands");
    let gone = a_set(&root, OperationId::new(), ConflictSource::Remote, "Gone");
    let unread = a_set(&root, OperationId::new(), ConflictSource::Remote, "Unread");
    for set in [&stands, &gone, &unread] {
        store.keep(set).unwrap();
    }
    let backend = fixture.delaying_backend();
    backend.answer_next_conflict_read(conflict(
        stands.operation_id,
        ConflictState::ResolutionPrepared,
        ConflictReadNextAction::RetryResolution,
        Some(stands.request_id),
    ));
    backend.answer_next_conflict_read(conflict(
        gone.operation_id,
        ConflictState::Resolved,
        ConflictReadNextAction::Resume,
        None,
    ));

    let _services = cx.update(|cx| Services::start(backend, cx).without_launch_refresh());
    until(cx, "the sweep", |_| {
        store.kept(gone.request_id).unwrap().is_none()
    })
    .await;
    assert_eq!(kept(&store, stands.request_id), Some(fields(&stands)));
    assert_eq!(kept(&store, unread.request_id), Some(fields(&unread)));
}

/// M-1: a resolution removed from the queue before it ran never reached
/// the library; its set is settled from the library's state. A local one
/// goes, as the library has no record of it; so does a remote one whose
/// conflict names no attempt of its request, while one whose conflict
/// still names its request as the unfinished attempt's is kept (M-e).
///
/// Canned conflicts: as in `the_sweep_keeps_a_remote_set_while_its_attempt_stands`.
#[gpui_kit::test]
async fn a_removed_resolution_deletes_its_set(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let (backend, services) = start(&fixture, cx);
    let store = fixture.resolution_store();
    let first = a_set(&root, OperationId::new(), ConflictSource::Local, "First");
    let second = a_set(&root, OperationId::new(), ConflictSource::Local, "Second");
    let (held_request, removed_request) = (first.request_id, second.request_id);

    let held = backend.hold_job(JobKind::LocalContextResolve);
    let sent = cx.update(|cx| services.submit_resolution(first, cx));
    cx.run_until_parked();
    let held_id = sent.await.expect("kept");
    held.wait_reached();
    let queued = cx.update(|cx| services.submit_resolution(second, cx));
    cx.run_until_parked();
    let removed = queued.await.expect("kept");
    assert!(store.kept(removed_request).unwrap().is_some());
    services.worker().cancel(removed);
    until(cx, "the removal", |cx| {
        removed_from_queue(&services, removed, cx)
    })
    .await;
    until(cx, "the removed set's deletion", |_| {
        store.kept(removed_request).unwrap().is_none()
    })
    .await;
    assert!(
        store.kept(held_request).unwrap().is_some(),
        "the running one's stays"
    );

    // Remote: one whose conflict is resolved, one whose attempt stands.
    let gone = a_set(&root, OperationId::new(), ConflictSource::Remote, "Gone");
    let stands = a_set(&root, OperationId::new(), ConflictSource::Remote, "Stands");
    backend.answer_next_conflict_read(conflict(
        gone.operation_id,
        ConflictState::Resolved,
        ConflictReadNextAction::Resume,
        None,
    ));
    backend.answer_next_conflict_read(conflict(
        stands.operation_id,
        ConflictState::ResolutionPrepared,
        ConflictReadNextAction::RetryResolution,
        Some(stands.request_id),
    ));
    for set in [gone.clone(), stands.clone()] {
        let queued = cx.update(|cx| services.submit_resolution(set, cx));
        cx.run_until_parked();
        let removed = queued.await.expect("kept");
        services.worker().cancel(removed);
        until(cx, "the removal", |cx| {
            removed_from_queue(&services, removed, cx)
        })
        .await;
    }
    until(cx, "the resolved one's deletion", |_| {
        store.kept(gone.request_id).unwrap().is_none()
    })
    .await;
    assert_eq!(kept(&store, stands.request_id), Some(fields(&stands)));

    held.release();
    finished(&services, held_id, cx).await;
}

/// I-1 (R-25 as refined): a remote resolution cancelled before the library
/// accepted it, as the requests panel's Cancel or quitting does by
/// signalling its token, answers `cancelled` with nothing run or written
/// (`execute.rs` stops it before acceptance). Nothing is checkpointed, so
/// the answer proves nothing of its attempt: its set is settled as one the
/// library has no record of, kept while its conflict names its request
/// and deleted once it does not.
///
/// Canned: the real call here, of an operation the repository does not
/// hold, is refused before the token is looked at, so the answer stands in
/// for the one `tests/mutation_replay/execute.rs`
/// `a_request_cancelled_before_it_is_accepted_does_nothing` produces:
/// `Cancelled`, nothing written or committed. Canned conflicts: as in
/// `the_sweep_keeps_a_remote_set_while_its_attempt_stands`.
#[gpui_kit::test]
async fn a_cancel_before_acceptance_keeps_a_set_whose_attempt_stands(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let (backend, services) = start(&fixture, cx);
    let store = fixture.resolution_store();
    for stands in [true, false] {
        let set = a_set(&root, OperationId::new(), ConflictSource::Remote, "Retry");
        let request = set.request_id;
        backend.answer_next_conflict_read(if stands {
            conflict(
                set.operation_id,
                ConflictState::ResolutionPrepared,
                ConflictReadNextAction::RetryResolution,
                Some(request),
            )
        } else {
            conflict(
                set.operation_id,
                ConflictState::Resolved,
                ConflictReadNextAction::Resume,
                None,
            )
        });
        let canned = answer(ResultCode::Cancelled, Effects::not_requested());
        let sent = answered_set(&backend, &services, set, canned, cx).await;
        assert_eq!(sent, request);
        assert_eq!(
            store.kept(request).unwrap().is_some(),
            stands,
            "kept exactly while its attempt stands"
        );
    }
}

fn removed_from_queue(services: &Services, id: DesktopRequestId, cx: &TestAppContext) -> bool {
    services.operations().read_with(cx, |operations, _| {
        operations
            .request(id)
            .is_some_and(|request| matches!(request.state, RequestState::Removed))
    })
}

/// M-3: a local resolution stopped after it wrote a path keeps its record
/// accepted (owed), so the drain keeps its set; the same request, sent
/// again, finishes it, and its set goes. Real, through the library's
/// failure point after a resolution path.
#[gpui_kit::test]
async fn the_drain_keeps_a_set_after_a_partial_local_answer(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let (backend, services) = start(&fixture, cx);
    let store = fixture.resolution_store();
    let set = a_local_resolution(&fixture, &root);
    let request = set.request_id;
    backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            fixture.data_dir(),
            FailurePoint::LocalSyncAfterResolutionPath,
        )
        .unwrap(),
    );
    let submitted = cx.update(|cx| services.submit_resolution(set, cx));
    cx.run_until_parked();
    let id = submitted.await.expect("kept");
    finished(&services, id, cx).await;
    assert_ne!(code_of(&services, id, cx), ResultCode::Ok);
    let shown = fixture.service().show_request(request);
    assert_eq!(
        shown.data.map(|record| record.state),
        Some(LibraryState::Accepted),
        "owed"
    );
    cx.run_until_parked();
    assert!(store.kept(request).unwrap().is_some(), "kept for its retry");

    backend.replace_service(RepositoryService::open_at(fixture.data_dir()).unwrap());
    assert!(services.worker().retry(id, None));
    until(cx, "the retry's answer", |cx| {
        services.operations().read_with(cx, |operations, _| {
            operations.request(id).is_some_and(|request| {
                request.attempt == 2 && matches!(request.state, RequestState::Finished(..))
            })
        })
    })
    .await;
    assert_eq!(code_of(&services, id, cx), ResultCode::Ok);
    until(cx, "the finished set's deletion", |_| {
        store.kept(request).unwrap().is_none()
    })
    .await;
}

fn code_of(services: &Services, id: DesktopRequestId, cx: &TestAppContext) -> ResultCode {
    services.operations().read_with(cx, |operations, _| {
        match &operations.request(id).unwrap().state {
            RequestState::Finished(_, code, ..) => *code,
            _ => panic!("the request must have finished"),
        }
    })
}

/// M-3: where `show_request` fails, the record's state is unknown and the
/// set is kept; once it reads, a request it has no record of lets a local
/// set go. The read fails while another holder has the index's recovery
/// lock, which no read waits for (as P17 makes it fail).
#[gpui_kit::test]
fn settle_keeps_the_set_when_its_read_fails(_cx: &mut TestAppContext) {
    use fs4::fs_std::FileExt;

    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let backend = fixture.delaying_backend();
    let store = fixture.resolution_store();
    let set = a_set(&root, OperationId::new(), ConflictSource::Local, "Unknown");
    store.keep(&set).unwrap();

    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(fixture.data_dir().join("manyhands.sqlite3.recovery.lock"))
        .unwrap();
    assert!(FileExt::try_lock_exclusive(&lock).unwrap());
    assert!(!store.settle(backend.as_ref(), set.request_id));
    assert!(
        store.kept(set.request_id).unwrap().is_some(),
        "unknown: kept"
    );
    drop(lock);
    assert!(store.settle(backend.as_ref(), set.request_id));
    assert!(store.kept(set.request_id).unwrap().is_none());
}
