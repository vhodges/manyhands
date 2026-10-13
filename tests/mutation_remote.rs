//! The synchronization commands of the mutation boundary, their progress
//! and their cancellation, against a real authenticated server. Isolated
//! before threads, as every target that opens a transport is.
#![allow(clippy::result_large_err)]
#[allow(dead_code, unused_imports)]
#[path = "../src/lib.rs"]
mod production;
pub use production::*;
#[path = "support/conformance.rs"]
mod conformance;
#[path = "mutation_remote/host.rs"]
mod host;
#[path = "support/remote_world.rs"]
mod remote_world;
#[path = "support/schema.rs"]
mod schema;
#[path = "support/ssh_harness.rs"]
mod ssh_harness;
#[path = "support/ssh_privacy.rs"]
mod ssh_privacy;
#[path = "support/ssh_remote.rs"]
mod ssh_remote;

use remote_world::*;
use repository::{keys::*, transport::*, *};
use results::{
    DiscoveryEffect, Effects, Envelope, Outcome, PublicationEffect, RecoveryActionKind, ResultCode,
};
use ssh_remote::*;

fn main() {
    // SAFETY: real main, before any fixture, runner or runtime thread exists.
    if unsafe { ssh_harness::initialize() }.is_err() {
        eprintln!("SSH synchronization initialization failed");
        std::process::exit(1);
    }
    let cases: Vec<_> = CASES.iter().chain(host::CASES).copied().collect();
    let controls: Vec<_> = OUTPUT_CONTROLS
        .iter()
        .chain(host::OUTPUT_CONTROLS)
        .copied()
        .collect();
    ssh_harness::run_with_output_privacy(&cases, &controls);
}

const CASES: &[ssh_harness::Case] = &[
    (
        "the_sibling_reports_each_safe_point_in_order",
        the_sibling_reports_each_safe_point_in_order,
    ),
    (
        "the_sibling_refuses_divergence_and_the_existing_function_merges_the_same_fixture",
        the_sibling_refuses_divergence_and_the_existing_function_merges_the_same_fixture,
    ),
    (
        "an_observer_stops_a_synchronization_at_the_safe_point_it_asks_at",
        an_observer_stops_a_synchronization_at_the_safe_point_it_asks_at,
    ),
    (
        "the_sibling_does_not_continue_a_merge_it_is_set_to_refuse",
        the_sibling_does_not_continue_a_merge_it_is_set_to_refuse,
    ),
    (
        "a_clean_synchronization_is_published_and_the_server_holds_the_commit",
        a_clean_synchronization_is_published_and_the_server_holds_the_commit,
    ),
    (
        "an_item_synchronization_publishes_the_item_s_branch",
        an_item_synchronization_publishes_the_item_s_branch,
    ),
    (
        "a_push_whose_result_was_lost_is_reconciled_by_the_same_request",
        a_push_whose_result_was_lost_is_reconciled_by_the_same_request,
    ),
    (
        "a_published_synchronization_whose_retry_fails_before_its_row_is_read_keeps_its_result",
        a_published_synchronization_whose_retry_fails_before_its_row_is_read_keeps_its_result,
    ),
    (
        "the_same_request_id_for_another_target_is_a_mismatch",
        the_same_request_id_for_another_target_is_a_mismatch,
    ),
    (
        "a_repository_with_no_publication_remote_is_blocked",
        a_repository_with_no_publication_remote_is_blocked,
    ),
    (
        "a_divergent_remote_is_merge_required_and_holds_its_reservation",
        a_divergent_remote_is_merge_required_and_holds_its_reservation,
    ),
    (
        "a_remotely_deleted_item_branch_is_remote_branch_deleted_and_holds_its_reservation",
        a_remotely_deleted_item_branch_is_remote_branch_deleted_and_holds_its_reservation,
    ),
    (
        "a_rejected_push_is_push_rejected_and_holds_its_reservation",
        a_rejected_push_is_push_rejected_and_holds_its_reservation,
    ),
    (
        "an_unknown_host_is_host_approval_required_and_holds_its_reservation",
        an_unknown_host_is_host_approval_required_and_holds_its_reservation,
    ),
    (
        "a_rotated_host_key_is_host_replacement_required_with_both_fingerprints",
        a_rotated_host_key_is_host_replacement_required_with_both_fingerprints,
    ),
    (
        "a_locked_key_with_no_passphrase_is_unlock_required_and_holds_its_reservation",
        a_locked_key_with_no_passphrase_is_unlock_required_and_holds_its_reservation,
    ),
    (
        "a_client_the_server_rejects_is_unlock_failed_or_unlock_required",
        a_client_the_server_rejects_is_unlock_failed_or_unlock_required,
    ),
    (
        "a_server_that_takes_the_client_without_its_key_is_key_rejected",
        a_server_that_takes_the_client_without_its_key_is_key_rejected,
    ),
    (
        "a_stalled_connection_is_transport_unavailable",
        a_stalled_connection_is_transport_unavailable,
    ),
    (
        "a_poll_holding_the_reservation_is_poll_yielding_and_leaves_no_record",
        a_poll_holding_the_reservation_is_poll_yielding_and_leaves_no_record,
    ),
    (
        "a_published_commit_whose_index_hand_off_failed_is_partial_and_its_retry_pushes_nothing",
        a_published_commit_whose_index_hand_off_failed_is_partial_and_its_retry_pushes_nothing,
    ),
    (
        "the_events_of_a_synchronization_arrive_in_safe_point_order_and_hold_nothing_private",
        the_events_of_a_synchronization_arrive_in_safe_point_order_and_hold_nothing_private,
    ),
    (
        "a_token_cancelled_before_execute_does_nothing",
        a_token_cancelled_before_execute_does_nothing,
    ),
    (
        "a_token_cancelled_from_the_sink_stops_at_the_first_safe_point",
        a_token_cancelled_from_the_sink_stops_at_the_first_safe_point,
    ),
    (
        "a_cancellation_requested_while_the_transport_is_blocked_is_honored_when_it_returns",
        a_cancellation_requested_while_the_transport_is_blocked_is_honored_when_it_returns,
    ),
    (
        "a_cancellation_requested_before_the_reservation_exists_is_honored_at_the_first_safe_point",
        a_cancellation_requested_before_the_reservation_exists_is_honored_at_the_first_safe_point,
    ),
    (
        "an_interrupted_synchronization_is_cancelled_only_when_its_row_says_so",
        an_interrupted_synchronization_is_cancelled_only_when_its_row_says_so,
    ),
    (
        "a_retry_while_the_first_call_is_still_running_takes_the_synchronization_over",
        a_retry_while_the_first_call_is_still_running_takes_the_synchronization_over,
    ),
    (
        "a_synchronization_retried_after_the_cache_is_lost_is_blocked_until_each_piece_is_restored",
        a_synchronization_retried_after_the_cache_is_lost_is_blocked_until_each_piece_is_restored,
    ),
    (
        "a_synchronization_retried_once_everything_is_restored_is_current_and_pushes_nothing",
        a_synchronization_retried_once_everything_is_restored_is_current_and_pushes_nothing,
    ),
    ("a_request_cancel_during_the_push_reports_the_push", || {
        a_cancellation_during_the_push_reports_the_push(false)
    }),
    ("a_token_cancelled_during_the_push_reports_the_push", || {
        a_cancellation_during_the_push_reports_the_push(true)
    }),
    (
        "a_request_cancel_after_a_lost_push_is_answered_only_once_the_remote_was_asked",
        || a_cancellation_after_a_lost_push_reports_the_push(false),
    ),
    (
        "a_token_cancelled_after_a_restart_verified_a_lost_push_reports_the_push",
        || a_cancellation_after_a_lost_push_reports_the_push(true),
    ),
    (
        "a_lost_push_the_remote_has_since_built_on_is_merge_required_and_leaves_nothing_of_a_merge",
        a_lost_push_the_remote_has_since_built_on_is_merge_required_and_leaves_nothing_of_a_merge,
    ),
    (
        "merge_required_is_not_finished_by_the_same_request_after_the_branch_is_rebased",
        merge_required_is_not_finished_by_the_same_request_after_the_branch_is_rebased,
    ),
    (
        "request_cancel_finishes_a_request_whose_operation_was_fenced",
        request_cancel_finishes_a_request_whose_operation_was_fenced,
    ),
    (
        "a_changed_endpoint_fences_a_held_synchronization_at_the_next_call_of_any_request",
        a_changed_endpoint_fences_a_held_synchronization_at_the_next_call_of_any_request,
    ),
    (
        "a_synchronization_fenced_with_its_push_unaccounted_for_is_not_confirmed",
        a_synchronization_fenced_with_its_push_unaccounted_for_is_not_confirmed,
    ),
    (
        "a_synchronization_fenced_after_its_push_returned_is_published",
        a_synchronization_fenced_after_its_push_returned_is_published,
    ),
    (
        "a_row_cancelled_as_its_push_returns_says_the_push_returned",
        a_row_cancelled_as_its_push_returns_says_the_push_returned,
    ),
    (
        "a_cancellation_with_the_remote_unreachable_frees_the_repository_and_confirms_nothing",
        a_cancellation_with_the_remote_unreachable_frees_the_repository_and_confirms_nothing,
    ),
    (
        "with_a_separate_push_url_a_fetch_is_not_asking_the_remote",
        with_a_separate_push_url_a_fetch_is_not_asking_the_remote,
    ),
    (
        "a_first_call_cancelled_before_its_push_claims_nothing_from_the_fetch_side",
        a_first_call_cancelled_before_its_push_claims_nothing_from_the_fetch_side,
    ),
    (
        "a_cancellation_asked_before_a_crash_during_the_fetch_still_stops_the_push",
        a_cancellation_asked_before_a_crash_during_the_fetch_still_stops_the_push,
    ),
    (
        "a_request_that_cannot_read_whether_it_was_cancelled_stops_and_does_not_push",
        a_request_that_cannot_read_whether_it_was_cancelled_stops_and_does_not_push,
    ),
    (
        "a_cancellation_that_could_not_be_recorded_on_the_operation_stops_the_next_call",
        a_cancellation_that_could_not_be_recorded_on_the_operation_stops_the_next_call,
    ),
    (
        "a_resume_reconciles_a_push_whose_result_was_lost_and_pushes_nothing",
        a_resume_reconciles_a_push_whose_result_was_lost_and_pushes_nothing,
    ),
    (
        "a_resume_runs_the_hand_off_a_published_synchronization_owes",
        a_resume_runs_the_hand_off_a_published_synchronization_owes,
    ),
    (
        "a_resume_does_not_take_up_a_fenced_synchronization",
        a_resume_does_not_take_up_a_fenced_synchronization,
    ),
    (
        "a_resume_never_merges_and_leaves_nothing_of_a_merge",
        a_resume_never_merges_and_leaves_nothing_of_a_merge,
    ),
    (
        "a_resume_does_not_continue_a_merge_made_some_other_way",
        a_resume_does_not_continue_a_merge_made_some_other_way,
    ),
    (
        "a_resume_stopped_by_its_token_ends_the_synchronization_it_took_up",
        a_resume_stopped_by_its_token_ends_the_synchronization_it_took_up,
    ),
    (
        "a_resume_that_cannot_unlock_the_key_leaves_the_synchronization_to_its_request",
        a_resume_that_cannot_unlock_the_key_leaves_the_synchronization_to_its_request,
    ),
    (
        "a_resume_of_an_item_s_synchronization_is_replayed_while_its_branch_holds_the_commit",
        a_resume_of_an_item_s_synchronization_is_replayed_while_its_branch_holds_the_commit,
    ),
    (
        "a_resume_stopped_for_an_untrusted_host_names_the_host_and_offers_its_approval",
        a_resume_stopped_for_an_untrusted_host_names_the_host_and_offers_its_approval,
    ),
    (
        "a_resume_whose_answer_was_lost_says_what_the_synchronization_it_took_up_published",
        a_resume_whose_answer_was_lost_says_what_the_synchronization_it_took_up_published,
    ),
    (
        "a_resume_cancelled_with_the_remote_unreachable_confirms_nothing_and_is_replayed_as_it_is",
        a_resume_cancelled_with_the_remote_unreachable_confirms_nothing_and_is_replayed_as_it_is,
    ),
    (
        "a_resume_cancelled_during_its_push_whose_answer_was_lost_still_reports_the_push",
        a_resume_cancelled_during_its_push_whose_answer_was_lost_still_reports_the_push,
    ),
    (
        "a_resume_stopped_by_its_token_whose_answer_was_lost_is_told_it_was_cancelled",
        a_resume_stopped_by_its_token_whose_answer_was_lost_is_told_it_was_cancelled,
    ),
    (
        "a_cancellation_asked_of_the_request_that_owns_it_stops_a_resume_before_it_pushes",
        a_cancellation_asked_of_the_request_that_owns_it_stops_a_resume_before_it_pushes,
    ),
    (
        "a_cancellation_asked_of_the_owning_request_while_a_resume_runs_stops_it_before_it_pushes",
        a_cancellation_asked_of_the_owning_request_while_a_resume_runs_stops_it_before_it_pushes,
    ),
    (
        "a_resume_that_cannot_read_whether_the_owning_request_was_cancelled_stops_and_does_not_push",
        a_resume_that_cannot_read_whether_the_owning_request_was_cancelled_stops_and_does_not_push,
    ),
    (
        "a_call_of_the_owning_request_fenced_by_a_resume_that_published_settles_nothing",
        a_call_of_the_owning_request_fenced_by_a_resume_that_published_settles_nothing,
    ),
    (
        "a_call_of_the_owning_request_fenced_by_a_resume_that_was_cancelled_settles_nothing",
        a_call_of_the_owning_request_fenced_by_a_resume_that_was_cancelled_settles_nothing,
    ),
    (
        "the_output_privacy_scan_passes_what_the_boundary_emits_and_fails_what_is_private",
        the_output_privacy_scan_passes_what_the_boundary_emits_and_fails_what_is_private,
    ),
    (
        "every_command_of_this_target_keeps_the_rules_of_the_boundary",
        every_command_of_this_target_keeps_the_rules_of_the_boundary,
    ),
];

/// Cases run only as children of the case above, which says of each
/// whether the harness is to pass it.
const OUTPUT_CONTROLS: &[ssh_harness::Case] = &[
    ("control_prints_what_the_boundary_emits", || {
        control_prints(Printed::Emitted)
    }),
    ("control_prints_the_remote_location", || {
        control_prints(Printed::RemoteLocation)
    }),
    ("control_prints_the_passphrase", || {
        control_prints(Printed::Passphrase)
    }),
    ("control_prints_the_commit_message", || {
        control_prints(Printed::CommitMessage)
    }),
];

/// An observer that records the safe points it is told of, and asks for
/// a cancellation at `stop_at`.
#[derive(Default)]
struct Points {
    seen: Vec<RemoteOperationSafePoint>,
    stop_at: Option<RemoteOperationSafePoint>,
}

impl SynchronizationObserver for Points {
    fn safe_point(&mut self, point: RemoteOperationSafePoint) -> SafePointRequest {
        self.seen.push(point);
        if self.stop_at == Some(point) {
            SafePointRequest::Cancel
        } else {
            SafePointRequest::Continue
        }
    }
}

const REFUSING: SynchronizationSettings = SynchronizationSettings {
    refuse_divergence: true,
};

fn observed(
    w: &World,
    request: SynchronizeRemoteRequest,
    settings: SynchronizationSettings,
    observer: &mut Points,
) -> Result<SynchronizationResult, SynchronizationError> {
    let result = w.service.synchronize_remote_observed(
        request,
        &mut SessionCredentials::new(Provider),
        settings,
        observer,
    );
    let formatted = match &result {
        Ok(value) => format!("{value:?}"),
        Err(error) => format!("{error:?} {error}"),
    };
    ssh_privacy::clean(formatted.as_bytes(), &w.probes).expect("redacted synchronization output");
    w.privacy().expect("redacted durable synchronization state");
    result
}

/// The phase the remote journal holds for `operation`.
fn phase(w: &World, operation: OperationId) -> Result<String, FixtureError> {
    fixed(w.db()?.query_row(
        "SELECT phase FROM remote_operation_records WHERE operation_ulid=?1",
        [operation.to_string()],
        |row| row.get(0),
    ))
}

/// How many rows the journal of merges holds: windows, steps, conflicts
/// and resolutions alike. Window zero is the one every synchronization
/// is given when it reserves, and is no merge.
fn merge_rows(w: &World) -> Result<i64, FixtureError> {
    let db = w.db()?;
    let mut rows = 0;
    for table in [
        "remote_integration_windows WHERE number > 0",
        "remote_integration_steps",
        "remote_integration_merge_metadata",
        "remote_resolution_attempts",
        "remote_publication_attempts",
    ] {
        rows += fixed(
            db.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get::<_, i64>(0)
            }),
        )?;
    }
    Ok(rows)
}

/// Every local branch and where it is, in name order.
fn local_branches(repo: &git2::Repository) -> Result<Vec<(String, git2::Oid)>, FixtureError> {
    let mut branches = Vec::new();
    for reference in fixed(repo.references_glob("refs/heads/*"))? {
        let reference = fixed(reference)?;
        branches.push((
            reference.name().ok_or(FixtureError)?.to_owned(),
            reference.target().ok_or(FixtureError)?,
        ));
    }
    branches.sort();
    Ok(branches)
}

fn merge_commits(repo: &git2::Repository) -> Result<usize, FixtureError> {
    let mut walk = fixed(repo.revwalk())?;
    fixed(walk.push_glob("refs/*"))?;
    let mut merges = 0;
    for oid in walk {
        if fixed(repo.find_commit(fixed(oid)?))?.parent_count() > 1 {
            merges += 1;
        }
    }
    Ok(merges)
}

fn the_sibling_reports_each_safe_point_in_order() -> Result<(), FixtureError> {
    use RemoteOperationSafePoint as P;
    let w = World::new()?;
    let repo = w.repo()?;
    // The remote is ahead: a local update and no push.
    let advanced = advance(&w.peer, "refs/heads/main", "peer-advance")?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    let mut points = Points::default();
    outcome(
        fixed(observed(&w, w.primary(), REFUSING, &mut points))?,
        false,
        SynchronizationTarget::Primary,
        advanced,
    );
    assert_eq!(
        points.seen,
        [
            P::BeforeFetch,
            P::AfterFetch,
            P::BeforeLocalUpdate,
            P::AfterLocalUpdate,
            P::AfterPushVerification
        ]
    );
    // Local is ahead: a push and no local update.
    let local = advance(&repo, "refs/heads/main", "local-advance")?;
    let mut points = Points::default();
    outcome(
        fixed(observed(&w, w.primary(), REFUSING, &mut points))?,
        true,
        SynchronizationTarget::Primary,
        local,
    );
    assert_eq!(
        points.seen,
        [
            P::BeforeFetch,
            P::AfterFetch,
            P::BeforePush,
            P::AfterPushReturn,
            P::AfterPushVerification
        ]
    );
    Ok(())
}

fn the_sibling_refuses_divergence_and_the_existing_function_merges_the_same_fixture()
-> Result<(), FixtureError> {
    let w = World::new()?;
    let repo = w.repo()?;
    let local = advance(&repo, "refs/heads/main", "local-divergent")?;
    let incoming = advance(&w.peer, "refs/heads/main", "remote-divergent")?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    let branches = local_branches(&repo)?;
    let served = local_branches(&w.bare()?)?;
    let updates = w.server.receive_updates().len();

    let refused = w.primary();
    let mut points = Points::default();
    assert!(matches!(
        observed(&w, refused.clone(), REFUSING, &mut points),
        Err(SynchronizationError::MergeRequired {
            target: SynchronizationTarget::Primary
        })
    ));
    // It stopped after the fetch, with nothing of a merge begun.
    assert_eq!(
        points.seen,
        [
            RemoteOperationSafePoint::BeforeFetch,
            RemoteOperationSafePoint::AfterFetch
        ]
    );
    assert_eq!(merge_rows(&w)?, 0);
    assert_eq!(merge_commits(&repo)?, 0);
    assert_eq!(local_branches(&repo)?, branches);
    assert_eq!(local_branches(&w.bare()?)?, served);
    assert_eq!(w.server.receive_updates().len(), updates);
    assert!(!repo.path().join("MERGE_HEAD").exists());
    assert!(!fixed(repo.index())?.has_conflicts());

    // The refused operation keeps its reservation: its cancellation,
    // acknowledged by a call under its ID, is what frees the repository
    // for the existing function to merge the same fixture.
    fixed(
        w.service
            .cancel_remote_operation(&w.root, refused.operation_id),
    )?;
    let mut again = refused.clone();
    again.restart = true;
    // The restart that acknowledges the cancellation reports it as
    // recovery required; the row is what says the operation was
    // cancelled, and every later call under the ID says interrupted.
    assert!(matches!(
        observed(&w, again.clone(), REFUSING, &mut Points::default()),
        Err(SynchronizationError::RecoveryRequired)
    ));
    assert_eq!(phase(&w, refused.operation_id)?, "cancelled");
    assert!(matches!(
        observed(&w, again, REFUSING, &mut Points::default()),
        Err(SynchronizationError::Interrupted)
    ));

    let result = fixed(w.sync(w.primary()))?;
    let SynchronizationResult::Complete(SynchronizationOutcome::Published { oid, .. }) = result
    else {
        return Err(FixtureError);
    };
    let merge = fixed(repo.find_commit(oid))?;
    assert_eq!(merge.parent_count(), 2);
    assert_eq!(fixed(merge.parent_id(0))?, local);
    assert_eq!(fixed(merge.parent_id(1))?, incoming);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, oid);
    assert_eq!(w.server.receive_updates().len(), updates + 1);
    Ok(())
}

fn an_observer_stops_a_synchronization_at_the_safe_point_it_asks_at() -> Result<(), FixtureError> {
    use RemoteOperationSafePoint as P;
    let w = World::new()?;
    let repo = w.repo()?;
    advance(&repo, "refs/heads/main", "local-advance")?;
    let served = fixed(w.bare()?.refname_to_id("refs/heads/main"))?;
    let updates = w.server.receive_updates().len();
    let request = w.primary();
    let mut points = Points {
        stop_at: Some(P::BeforePush),
        ..Points::default()
    };
    assert!(matches!(
        observed(&w, request.clone(), REFUSING, &mut points),
        Err(SynchronizationError::Interrupted)
    ));
    assert_eq!(points.seen, [P::BeforeFetch, P::AfterFetch, P::BeforePush]);
    assert_eq!(phase(&w, request.operation_id)?, "cancelled");
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, served);
    assert_eq!(w.server.receive_updates().len(), updates);
    Ok(())
}

/// A synchronization that merges, and leaves a conflict behind, is not
/// one a caller that never merges takes up: under the same operation ID
/// it is refused before anything is read, reserved or changed.
fn the_sibling_does_not_continue_a_merge_it_is_set_to_refuse() -> Result<(), FixtureError> {
    let w = World::new()?;
    let repo = w.repo()?;
    let local_base = fixed(repo.refname_to_id("refs/heads/main"))?;
    let local = commit(&repo, local_base, "conflict", b"local conflict")?;
    fixed(repo.checkout_tree(
        &fixed(repo.find_object(local, None))?,
        Some(git2::build::CheckoutBuilder::new().safe()),
    ))?;
    fixed(repo.reference("refs/heads/main", local, true, "fixture"))?;
    let incoming_base = fixed(w.peer.refname_to_id("refs/heads/main"))?;
    let incoming = commit(&w.peer, incoming_base, "conflict", b"incoming conflict")?;
    fixed(
        w.peer
            .reference("refs/heads/main", incoming, true, "fixture"),
    )?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    let merging = w.primary();
    assert!(matches!(
        w.sync(merging.clone()),
        Err(SynchronizationError::ConflictPending { .. })
    ));
    let rows = merge_rows(&w)?;
    assert!(rows > 0);
    let before = phase(&w, merging.operation_id)?;
    let opened = w.server.accepted_keys().len();

    let mut again = merging.clone();
    again.restart = true;
    let mut points = Points::default();
    assert!(matches!(
        observed(&w, again, REFUSING, &mut points),
        Err(SynchronizationError::RecoveryRequired)
    ));
    assert!(points.seen.is_empty());
    assert_eq!(merge_rows(&w)?, rows);
    assert_eq!(phase(&w, merging.operation_id)?, before);
    assert_eq!(fixed(repo.refname_to_id("refs/heads/main"))?, local);
    assert_eq!(w.server.accepted_keys().len(), opened);
    Ok(())
}

// The commands, through the boundary.

const REQUEST_1: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX1";
const REQUEST_2: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX2";
const REQUEST_3: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX3";
const REQUEST_4: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX4";
/// A ticket no fixture holds.
const OTHER_ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAZ";

fn request(id: &str) -> RequestId {
    RequestId::parse(id).unwrap()
}

fn repo_sync(w: &World) -> Mutation {
    Mutation::RepoSync(RepoSyncInput {
        root: w.root.clone(),
    })
}

fn item_sync(w: &World) -> Mutation {
    Mutation::ItemSync(ItemSyncInput {
        root: w.root.clone(),
        kind: AuthoringKind::Ticket,
        id: ITEM.parse().unwrap(),
    })
}

/// Has the service trust the server's host key, as a host approval
/// does: a synchronization of the boundary carries none of its own.
fn approve(w: &World) -> Result<(), FixtureError> {
    fixed(w.service.verify_ssh_transport(
        VerifySshTransportRequest {
            root: w.root.clone(),
            direction: SshDirection::Fetch,
            approval: Some(HostApproval {
                authority: SshAuthority {
                    host: "127.0.0.1".into(),
                    port: w.server.address().port(),
                },
                expected: None,
                presented: w.server.host_identity(),
            }),
        },
        &mut SessionCredentials::new(Provider),
    ))?;
    Ok(())
}

/// A sink that keeps what it is sent.
#[derive(Default)]
struct Events(Vec<ProgressEvent>);

impl ProgressSink for Events {
    fn event(&mut self, event: &ProgressEvent) {
        self.0.push(event.clone());
    }
}

/// One call of `execute` on `service`, with everything a call is given.
/// What it returns and what it left in the world's stores are scanned
/// for the world's private material before anything is asserted of
/// them.
fn call(
    w: &World,
    service: &RepositoryService,
    id: &str,
    mutation: Mutation,
    provider: impl SessionCredentialProvider,
    progress: &mut dyn ProgressSink,
    cancel: &CancellationToken,
) -> Envelope<MutationDataDto> {
    let envelope =
        call_before_the_stores_hold_anything(w, service, id, mutation, provider, progress, cancel);
    w.privacy().expect("redacted durable state");
    envelope
}

/// `call`, without the scan of the world's stores, which requires them
/// to hold rows: a store that was just lost holds none.
fn call_before_the_stores_hold_anything(
    w: &World,
    service: &RepositoryService,
    id: &str,
    mutation: Mutation,
    provider: impl SessionCredentialProvider,
    progress: &mut dyn ProgressSink,
    cancel: &CancellationToken,
) -> Envelope<MutationDataDto> {
    let envelope = service.execute(
        MutationCall {
            request_id: request(id),
            confirmation: None,
            mutation,
        },
        &mut SessionCredentials::new(provider),
        progress,
        cancel,
    );
    let rendered = serde_json::to_vec(&envelope).expect("an envelope serializes");
    ssh_privacy::clean(&rendered, &w.probes).expect("redacted envelope");
    // No fixture can be kept of an envelope that names a port and a
    // commit: each is checked against the published schemas instead.
    let value = serde_json::to_value(&envelope).expect("an envelope serializes");
    schema::check_published("envelope.schema.json", &value).expect("the envelope schema");
    // A resume that took up a synchronization carries that
    // synchronization's data.
    let synchronization = matches!(
        envelope.command.as_str(),
        "item sync" | "repo sync" | "operation resume"
    );
    if synchronization && !value["data"].is_null() {
        schema::check_published("synchronization_mutation.schema.json", &value["data"])
            .expect("the synchronization data schema");
    }
    ssh_privacy::clean(format!("{envelope:?}").as_bytes(), &w.probes).expect("redacted envelope");
    envelope
}

/// A call with the world's passphrase, no sink and no cancellation.
fn execute(w: &World, id: &str, mutation: Mutation) -> Envelope<MutationDataDto> {
    call(
        w,
        &w.service,
        id,
        mutation,
        Provider,
        &mut NoProgress,
        &CancellationToken::new(),
    )
}

fn record(w: &World, id: &str) -> Option<request_store::RequestRecord> {
    w.service.request_record(request(id)).unwrap()
}

fn actions(envelope: &Envelope<MutationDataDto>) -> Vec<RecoveryActionKind> {
    envelope
        .recovery
        .iter()
        .map(|action| action.action)
        .collect()
}

/// How many times a client has authenticated to the server: each
/// connection the library opens authenticates once.
fn connections(w: &World) -> usize {
    w.server.accepted_keys().len()
}

/// Whether a remote operation holds the repository's reservation.
fn reserved(w: &World) -> bool {
    w.service
        .active_remote_operation(&w.root)
        .unwrap()
        .is_some()
}

fn published(commit: git2::Oid) -> Effects {
    Effects {
        publication: PublicationEffect::Published,
        discovery: DiscoveryEffect::Current,
        commit_oid: Some(commit.to_string()),
        ..Effects::not_requested()
    }
}

fn a_clean_synchronization_is_published_and_the_server_holds_the_commit() -> Result<(), FixtureError>
{
    let w = World::new()?;
    approve(&w)?;
    let repo = w.repo()?;
    let local = advance(&repo, "refs/heads/main", "local-advance")?;
    let pushes = accepted(&w.server);

    let first = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(first.command, "repo sync");
    assert_eq!(first.outcome, Outcome::Success);
    assert_eq!(first.code, ResultCode::Ok);
    assert_eq!(first.effects, published(local));
    assert_eq!(first.request_id.as_deref(), Some(REQUEST_1));
    assert!(first.operation_id.is_some());
    assert_eq!(first.scope.branch.as_deref(), Some("main"));
    assert_eq!(first.scope.item_id, None);
    assert!(first.data.is_none());
    assert!(first.recovery.is_empty());
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, local);
    assert_eq!(accepted(&w.server), pushes + 1);
    assert!(!reserved(&w));
    let stored = record(&w, REQUEST_1).ok_or(FixtureError)?;
    assert_eq!(stored.state, RequestState::Finished);
    assert_eq!(stored.operations.len(), 1);
    assert_eq!(stored.operations[0].family, OperationFamily::Remote);

    // The finished request is answered from its record: no connection.
    let opened = connections(&w);
    let again = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(again, first);
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes + 1);

    // A new request finds the remote current and pushes nothing.
    let current = execute(&w, REQUEST_2, repo_sync(&w));
    assert_eq!(current.outcome, Outcome::Noop);
    assert_eq!(current.code, ResultCode::AlreadyApplied);
    assert_eq!(current.effects.publication, PublicationEffect::Current);
    assert_eq!(current.effects.commit_oid, None);
    assert_eq!(accepted(&w.server), pushes + 1);
    Ok(())
}

fn an_item_synchronization_publishes_the_item_s_branch() -> Result<(), FixtureError> {
    let mut w = World::new()?;
    approve(&w)?;
    let context = w.context()?;
    let tip = fixed(w.repo()?.refname_to_id(CONTEXT))?;
    assert!(w.bare()?.find_reference(CONTEXT).is_err());

    let envelope = execute(&w, REQUEST_1, item_sync(&w));
    assert_eq!(envelope.command, "item sync");
    assert_eq!(envelope.outcome, Outcome::Success);
    assert_eq!(envelope.effects, published(tip));
    assert_eq!(envelope.scope.item_id.as_deref(), Some(ITEM));
    assert_eq!(
        envelope.scope.branch.as_deref(),
        CONTEXT.strip_prefix("refs/heads/")
    );
    assert_eq!(
        envelope.scope.worktree.as_deref(),
        context.worktree.to_str()
    );
    assert_eq!(fixed(w.bare()?.refname_to_id(CONTEXT))?, tip);
    assert_eq!(
        record(&w, REQUEST_1).ok_or(FixtureError)?.target,
        format!("ticket/{ITEM}")
    );
    Ok(())
}

fn a_push_whose_result_was_lost_is_reconciled_by_the_same_request() -> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let repo = w.repo()?;
    let candidate = advance(&repo, "refs/heads/main", "lost-output")?;
    let pushes = accepted(&w.server);
    w.server.disconnect_at(FixtureBoundary::AfterReceivePack);

    let lost = execute(&w, REQUEST_1, repo_sync(&w));
    // The server took the push and the client never learned of it.
    assert!(w.server.receive_status_withheld());
    assert_eq!(
        fixed(w.bare()?.refname_to_id("refs/heads/main"))?,
        candidate
    );
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_eq!(lost.code, ResultCode::TransportUnavailable);
    assert_eq!(lost.outcome, Outcome::Error);
    // Nothing is claimed of a push whose acceptance was not seen.
    assert_eq!(lost.effects.publication, PublicationEffect::Pending);
    assert_eq!(lost.effects.commit_oid, None);
    assert_eq!(actions(&lost), [RecoveryActionKind::RequestRetry]);
    assert_eq!(
        record(&w, REQUEST_1).ok_or(FixtureError)?.state,
        RequestState::Accepted
    );
    w.server.clear_fault();

    let retried = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(retried.outcome, Outcome::Success);
    assert_eq!(retried.code, ResultCode::Ok);
    assert_eq!(retried.effects, published(candidate));
    assert_eq!(retried.operation_id, lost.operation_id);
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_eq!(
        record(&w, REQUEST_1).ok_or(FixtureError)?.state,
        RequestState::Finished
    );
    assert!(!reserved(&w));
    Ok(())
}

fn a_published_synchronization_whose_retry_fails_before_its_row_is_read_keeps_its_result()
-> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let local = advance(&w.repo()?, "refs/heads/main", "local-advance")?;
    let pushes = accepted(&w.server);
    let failing = fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &w.data(),
        FailurePoint::BeforeRequestSettlement,
    ))?;

    // The call published and died before it recorded its end.
    let lost = call(
        &w,
        &failing,
        REQUEST_1,
        repo_sync(&w),
        Provider,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(lost.code, ResultCode::InternalError);
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, local);
    let accepted_record = record(&w, REQUEST_1).ok_or(FixtureError)?;
    assert_eq!(accepted_record.state, RequestState::Accepted);
    let operation = accepted_record.operations[0].operation_id;
    assert_eq!(phase(&w, operation)?, "completed");

    // The repository cannot be resolved for a while: its Git directory
    // is away, so the call stops before it reads the row that completed.
    let git = w.root.join(".git");
    let away = w.root.join(".git-away");
    fixed(std::fs::rename(&git, &away))?;
    let stopped = call_before_the_stores_hold_anything(
        &w,
        &w.service,
        REQUEST_1,
        repo_sync(&w),
        Provider,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    fixed(std::fs::rename(&away, &git))?;
    w.privacy()?;
    assert_ne!(stopped.outcome, Outcome::Success);
    assert_ne!(stopped.outcome, Outcome::Noop);
    assert_eq!(stopped.operation_id, Some(operation.to_string()));
    // The error is no result of the synchronization, which has one: the
    // record stays, and the same request is what is offered.
    assert_eq!(
        record(&w, REQUEST_1).map(|record| record.state),
        Some(RequestState::Accepted)
    );
    assert_eq!(actions(&stopped), [RecoveryActionKind::RequestRetry]);

    // The same request is answered with what it published. Nothing is
    // connected to.
    let opened = connections(&w);
    let found = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(found.outcome, Outcome::Success);
    assert_eq!(found.code, ResultCode::Ok);
    assert_eq!(found.effects, published(local));
    assert_eq!(found.operation_id, Some(operation.to_string()));
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_eq!(
        record(&w, REQUEST_1).ok_or(FixtureError)?.state,
        RequestState::Finished
    );
    assert_eq!(execute(&w, REQUEST_1, repo_sync(&w)), found);
    Ok(())
}

fn the_same_request_id_for_another_target_is_a_mismatch() -> Result<(), FixtureError> {
    let mut w = World::new()?;
    approve(&w)?;
    w.context()?;
    let repo = w.repo()?;
    advance(&repo, "refs/heads/main", "local-advance")?;
    // An unfinished request: its push was accepted and its result lost.
    w.server.disconnect_at(FixtureBoundary::AfterReceivePack);
    let lost = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(lost.code, ResultCode::TransportUnavailable);
    w.server.clear_fault();
    let opened = connections(&w);
    let pushes = accepted(&w.server);

    let changed = execute(&w, REQUEST_1, item_sync(&w));
    assert_eq!(changed.code, ResultCode::RequestMismatch);
    assert_eq!(changed.outcome, Outcome::Error);
    assert!(changed.recovery.is_empty());
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes);
    assert!(w.bare()?.find_reference(CONTEXT).is_err());
    assert_eq!(
        record(&w, REQUEST_1).ok_or(FixtureError)?.state,
        RequestState::Accepted
    );

    // The same for a finished one.
    let finished = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(finished.outcome, Outcome::Success);
    let opened = connections(&w);
    let changed = execute(&w, REQUEST_1, item_sync(&w));
    assert_eq!(changed.code, ResultCode::RequestMismatch);
    assert_eq!(connections(&w), opened);
    assert!(w.bare()?.find_reference(CONTEXT).is_err());
    Ok(())
}

fn a_repository_with_no_publication_remote_is_blocked() -> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let config = w.root.join(".manyhands/config.toml");
    let source = fixed(std::fs::read_to_string(&config))?;
    fixed(std::fs::write(
        &config,
        source.replace("publication_remote = \"origin\"\n", ""),
    ))?;
    let opened = connections(&w);

    let envelope = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(envelope.outcome, Outcome::Blocked);
    assert_eq!(envelope.code, ResultCode::PublicationRemoteRequired);
    assert_eq!(envelope.effects, Effects::not_requested());
    assert_eq!(envelope.operation_id, None);
    assert!(envelope.recovery.is_empty());
    assert!(record(&w, REQUEST_1).is_none());
    assert_eq!(connections(&w), opened);
    assert!(!reserved(&w));
    Ok(())
}

// What stops a synchronization, and what it leaves: open case 3.

/// A provider with no passphrase to give: nobody is there to ask.
struct NoPassphrase;

impl SessionCredentialProvider for NoPassphrase {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        PassphraseResponse::Unavailable
    }
}

fn draft() -> TicketDraft {
    TicketDraft {
        title: "Another".into(),
        body: "Another ticket.\n".into(),
        ticket_type: "task".into(),
        status: "open".into(),
        project: None,
        team: None,
    }
}

/// A save on the repository, of a ticket no synchronization is of.
fn create_other(w: &World) -> Mutation {
    Mutation::TicketCreate(TicketCreateInput {
        root: w.root.clone(),
        id: OTHER_ITEM.parse().unwrap(),
        draft: draft(),
        deps: Vec::new(),
        parent: None,
        initials: Some("FX".into()),
    })
}

/// A synchronization that stops with an error after it reserved.
struct Stopped {
    mutation: fn(&World) -> Mutation,
    /// The code it stops with.
    code: ResultCode,
    /// Whether the call has no passphrase to give.
    locked: bool,
    /// Whether the same request can go on once the cause is dealt
    /// with, and is offered. It cannot from `merge_required` or
    /// `remote_branch_deleted`.
    retry: bool,
}

impl Stopped {
    fn call(&self, w: &World, id: &str) -> Envelope<MutationDataDto> {
        let mutation = (self.mutation)(w);
        if self.locked {
            call(
                w,
                &w.service,
                id,
                mutation,
                NoPassphrase,
                &mut NoProgress,
                &CancellationToken::new(),
            )
        } else {
            execute(w, id, mutation)
        }
    }

    /// Whether the synchronization is of the item, whose reservation
    /// refuses the authoring of that item and of no other.
    fn of_item(&self, w: &World) -> bool {
        matches!((self.mutation)(w), Mutation::ItemSync(_))
    }
}

/// Pins what a synchronization that stopped after it reserved leaves,
/// and what lets it go. `w` is set up so that the synchronization stops
/// with `stopped.code`, and stays so throughout.
///
/// What is pinned, for every cause alike:
///
/// - the result is blocked, carries `request.retry`, and leaves its
///   record accepted;
/// - the reservation is still active;
/// - a save on the repository is `recovery_required`, with nothing
///   recorded, when the reservation is the primary branch's; an item's
///   reservation refuses that item's saves only;
/// - another synchronization is `busy`, with nothing recorded;
/// - the same request again stops the same way and keeps the
///   reservation, for as long as the cause stands;
/// - `request cancel` followed by the same request ends it `cancelled`,
///   for good, and lets the reservation go: the save and a new
///   synchronization then run.
///
/// Returns the first result, for what a cause adds to it.
fn pin_a_held_reservation(
    w: &World,
    stopped: &Stopped,
) -> Result<Envelope<MutationDataDto>, FixtureError> {
    let pushes = accepted(&w.server);
    let first = stopped.call(w, REQUEST_1);
    assert_eq!(first.code, stopped.code);
    assert_eq!(first.outcome, Outcome::Blocked);
    assert_eq!(first.effects.publication, PublicationEffect::Pending);
    assert_eq!(first.effects.commit_oid, None);
    assert_eq!(
        actions(&first).contains(&RecoveryActionKind::RequestRetry),
        stopped.retry
    );
    let operation = first.operation_id.clone().ok_or(FixtureError)?;
    assert_eq!(
        record(w, REQUEST_1).ok_or(FixtureError)?.state,
        RequestState::Accepted
    );
    // The reservation is still active.
    assert!(reserved(w));

    // A save on the repository.
    if stopped.of_item(w) {
        // The item's own authoring is refused, and nothing else's.
        let refused = w.service.save_ticket(SaveTicketRequest {
            target: AuthoringTarget {
                root: w.root.clone(),
                kind: AuthoringKind::Ticket,
                item_id: ITEM.parse().unwrap(),
                intent: ContextIntent::Edit,
                operation_id: OperationId::new(),
            },
            draft: draft(),
            expected_path: ExpectedPathObservation::Missing,
        });
        assert_eq!(
            refused.err().map(|error| error.kind),
            Some(RepositoryErrorKind::RecoveryRequired)
        );
    } else {
        let save = execute(w, REQUEST_2, create_other(w));
        assert_eq!(save.code, ResultCode::RecoveryRequired);
        assert_eq!(save.outcome, Outcome::Blocked);
        assert!(record(w, REQUEST_2).is_none());
    }

    // Another synchronization.
    let other = execute(w, REQUEST_3, repo_sync(w));
    assert_eq!(other.code, ResultCode::Busy);
    assert_eq!(actions(&other), [RecoveryActionKind::RequestRetry]);
    assert!(record(w, REQUEST_3).is_none());

    // The same request, with the cause still there.
    let again = stopped.call(w, REQUEST_1);
    assert_eq!(again.code, stopped.code);
    assert_eq!(again.operation_id.as_deref(), Some(operation.as_str()));
    assert_eq!(
        actions(&again).contains(&RecoveryActionKind::RequestRetry),
        stopped.retry
    );
    assert!(reserved(w));
    assert_eq!(
        record(w, REQUEST_1).ok_or(FixtureError)?.state,
        RequestState::Accepted
    );

    // A result with no way forward offers its cancellation, and nothing
    // else: the same request cannot go on. One the same request can go on
    // from does not offer it.
    for envelope in [&first, &again] {
        let offered: Vec<&str> = envelope
            .recovery
            .iter()
            .map(|action| action.action.as_str())
            .collect();
        if stopped.retry {
            assert!(!offered.contains(&"request.cancel"));
        } else {
            assert_eq!(offered, ["request.cancel"]);
        }
    }
    // The offered action is what is taken, where there is one: it names
    // the request to cancel.
    let named = again
        .recovery
        .iter()
        .find(|action| action.action.as_str() == "request.cancel")
        .map(|action| {
            assert_eq!(action.operation_id, None);
            assert_eq!(action.arguments.len(), 1);
            action.arguments["request_id"]
                .as_str()
                .map(str::to_owned)
                .ok_or(FixtureError)
        })
        .transpose()?;
    assert_eq!(named.is_some(), !stopped.retry);
    let cancelled = named.as_deref().unwrap_or(REQUEST_1);
    assert_eq!(cancelled, REQUEST_1);

    // A cancellation, then the same request.
    let cancel = w.service.cancel_request(request(cancelled));
    assert_eq!(cancel.code, ResultCode::Ok);
    let asked = cancel.data.ok_or(FixtureError)?;
    assert!(asked.requested);
    assert!(!asked.acknowledged);
    // Asking is not stopping: nothing has run since.
    assert!(reserved(w));
    let ended = stopped.call(w, REQUEST_1);
    assert_eq!(ended.code, ResultCode::Cancelled);
    assert_eq!(ended.outcome, Outcome::Cancelled);
    assert_eq!(ended.operation_id.as_deref(), Some(operation.as_str()));
    assert!(ended.recovery.is_empty());
    assert!(!reserved(w));
    assert_eq!(
        record(w, REQUEST_1).ok_or(FixtureError)?.state,
        RequestState::Finished
    );
    assert_eq!(stopped.call(w, REQUEST_1), ended);
    let cancel = w.service.cancel_request(request(REQUEST_1));
    assert!(cancel.data.ok_or(FixtureError)?.acknowledged);

    // The repository is free again.
    let save = execute(w, REQUEST_2, create_other(w));
    assert_eq!(save.outcome, Outcome::Success);
    assert_eq!(accepted(&w.server), pushes);
    Ok(first)
}

fn a_divergent_remote_is_merge_required_and_holds_its_reservation() -> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let repo = w.repo()?;
    let local = advance(&repo, "refs/heads/main", "local-divergent")?;
    let incoming = advance(&w.peer, "refs/heads/main", "remote-divergent")?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    let branches = local_branches(&repo)?;
    let served = local_branches(&w.bare()?)?;
    let updates = w.server.receive_updates().len();

    let first = pin_a_held_reservation(
        &w,
        &Stopped {
            mutation: repo_sync,
            code: ResultCode::MergeRequired,
            locked: false,
            retry: false,
        },
    )?;
    assert!(first.data.is_none());
    // No merge commit, integration window or conflict exists, and
    // neither side's branches moved.
    assert_eq!(merge_rows(&w)?, 0);
    assert_eq!(merge_commits(&repo)?, 0);
    assert!(!repo.path().join("MERGE_HEAD").exists());
    assert!(!fixed(repo.index())?.has_conflicts());
    assert_eq!(fixed(repo.refname_to_id("refs/heads/main"))?, local);
    assert_eq!(
        local_branches(&repo)?
            .into_iter()
            .filter(|(name, _)| branches.iter().any(|(known, _)| known == name))
            .collect::<Vec<_>>(),
        branches
    );
    assert_eq!(local_branches(&w.bare()?)?, served);
    assert_eq!(w.server.receive_updates().len(), updates);

    // The existing function still merges the same fixture.
    let result = fixed(w.sync(w.primary()))?;
    let SynchronizationResult::Complete(SynchronizationOutcome::Published { oid, .. }) = result
    else {
        return Err(FixtureError);
    };
    let merge = fixed(repo.find_commit(oid))?;
    assert_eq!(merge.parent_count(), 2);
    assert_eq!(fixed(merge.parent_id(0))?, local);
    assert_eq!(fixed(merge.parent_id(1))?, incoming);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, oid);
    Ok(())
}

fn a_remotely_deleted_item_branch_is_remote_branch_deleted_and_holds_its_reservation()
-> Result<(), FixtureError> {
    let mut w = World::new()?;
    approve(&w)?;
    w.context()?;
    let published = execute(&w, REQUEST_4, item_sync(&w));
    assert_eq!(published.outcome, Outcome::Success);
    fixed(fixed(w.bare()?.find_reference(CONTEXT))?.delete())?;

    pin_a_held_reservation(
        &w,
        &Stopped {
            mutation: item_sync,
            code: ResultCode::RemoteBranchDeleted,
            locked: false,
            retry: false,
        },
    )?;
    // The branch is not published again.
    assert!(w.bare()?.find_reference(CONTEXT).is_err());
    Ok(())
}

fn a_rejected_push_is_push_rejected_and_holds_its_reservation() -> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let repo = w.repo()?;
    let old = fixed(w.bare()?.refname_to_id("refs/heads/main"))?;
    advance(&repo, "refs/heads/main", "rejected-local")?;
    w.server.reject_primary_updates(true)?;
    w.server.hostile_rejection(HOSTILE);

    let first = pin_a_held_reservation(
        &w,
        &Stopped {
            mutation: repo_sync,
            code: ResultCode::PushRejected,
            locked: false,
            retry: true,
        },
    )?;
    assert_eq!(actions(&first), [RecoveryActionKind::RequestRetry]);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, old);
    // The same request does go on from a rejection, once the server
    // takes the push: the action it offers can succeed.
    let rejected = execute(&w, REQUEST_4, repo_sync(&w));
    assert_eq!(rejected.code, ResultCode::PushRejected);
    w.server.reject_primary_updates(false)?;
    let taken = execute(&w, REQUEST_4, repo_sync(&w));
    assert_eq!(taken.outcome, Outcome::Success);
    assert_eq!(taken.operation_id, rejected.operation_id);
    assert_eq!(
        fixed(w.bare()?.refname_to_id("refs/heads/main"))?,
        fixed(repo.refname_to_id("refs/heads/main"))?
    );
    Ok(())
}

fn synchronization_data(
    envelope: &Envelope<MutationDataDto>,
) -> Result<&SynchronizationMutationDto, FixtureError> {
    match envelope.data.as_ref() {
        Some(MutationDataDto::Synchronization(data)) => Ok(data),
        _ => Err(FixtureError),
    }
}

/// The action that approves a host, checked for naming the server and
/// the repository.
fn assert_host_action(
    w: &World,
    envelope: &Envelope<MutationDataDto>,
    action: RecoveryActionKind,
) -> Result<(), FixtureError> {
    assert_eq!(
        actions(envelope),
        [action, RecoveryActionKind::RequestRetry]
    );
    let offered = &envelope.recovery[0];
    assert_eq!(
        offered
            .arguments
            .get("authority")
            .and_then(|value| value.as_str()),
        Some(format!("127.0.0.1:{}", w.server.address().port()).as_str())
    );
    assert_eq!(
        offered
            .arguments
            .get("root")
            .and_then(|value| value.as_str()),
        envelope.scope.repository.as_deref()
    );
    Ok(())
}

fn an_unknown_host_is_host_approval_required_and_holds_its_reservation() -> Result<(), FixtureError>
{
    let w = World::new()?;
    // No approval: the service has never trusted the server.
    let first = pin_a_held_reservation(
        &w,
        &Stopped {
            mutation: repo_sync,
            code: ResultCode::HostApprovalRequired,
            locked: false,
            retry: true,
        },
    )?;
    let data = synchronization_data(&first)?;
    let presented = w.server.host_identity();
    assert_eq!(data.host, "127.0.0.1");
    assert_eq!(data.port, w.server.address().port());
    assert_eq!(data.algorithm, presented.algorithm);
    assert_eq!(data.presented_fingerprint, presented.sha256);
    assert_eq!(data.expected_fingerprint, None);
    assert_host_action(&w, &first, RecoveryActionKind::HostApprove)?;

    // With the host approved, a new request synchronizes.
    approve(&w)?;
    let current = execute(&w, REQUEST_4, repo_sync(&w));
    assert_eq!(current.outcome, Outcome::Noop);
    assert_eq!(current.effects.publication, PublicationEffect::Current);
    Ok(())
}

fn a_rotated_host_key_is_host_replacement_required_with_both_fingerprints()
-> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let pinned = w.server.host_identity();
    w.server.rotate_host_key()?;
    let presented = w.server.host_identity();
    assert!(pinned != presented);

    let envelope = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(envelope.code, ResultCode::HostReplacementRequired);
    assert_eq!(envelope.outcome, Outcome::Blocked);
    let data = synchronization_data(&envelope)?;
    assert_eq!(data.presented_fingerprint, presented.sha256);
    assert_eq!(
        data.expected_fingerprint.as_deref(),
        Some(pinned.sha256.as_str())
    );
    assert_host_action(&w, &envelope, RecoveryActionKind::HostReplace)?;
    assert!(reserved(&w));
    Ok(())
}

fn a_locked_key_with_no_passphrase_is_unlock_required_and_holds_its_reservation()
-> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    // The same request does go on from this one, once a passphrase is
    // to be had: the action it offers can succeed.
    let locked = call(
        &w,
        &w.service,
        REQUEST_4,
        repo_sync(&w),
        NoPassphrase,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(locked.code, ResultCode::UnlockRequired);
    let unlocked = execute(&w, REQUEST_4, repo_sync(&w));
    assert_current(&unlocked);
    assert_eq!(unlocked.operation_id, locked.operation_id);
    let first = pin_a_held_reservation(
        &w,
        &Stopped {
            mutation: repo_sync,
            code: ResultCode::UnlockRequired,
            locked: true,
            retry: true,
        },
    )?;
    assert_eq!(actions(&first), [RecoveryActionKind::RequestRetry]);
    assert!(first.data.is_none());
    Ok(())
}

/// What the brief calls `key_rejected` is not what a server's refusal
/// of the client's key comes to. The transport cannot tell a key the
/// server refused from one that was not unlocked, so it asks for a
/// passphrase and tries once more: the result is `unlock_failed` when
/// one was given and `unlock_required` when none could be. `key_rejected`
/// is a server that let the client in without its key.
fn a_client_the_server_rejects_is_unlock_failed_or_unlock_required() -> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    w.server.reject_client();

    let supplied = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(supplied.code, ResultCode::UnlockFailed);
    assert_eq!(supplied.outcome, Outcome::Blocked);
    assert_eq!(actions(&supplied), [RecoveryActionKind::RequestRetry]);
    assert!(supplied.data.is_none());
    assert!(reserved(&w));

    // The same request, from a caller with no passphrase to give.
    let unavailable = call(
        &w,
        &w.service,
        REQUEST_1,
        repo_sync(&w),
        NoPassphrase,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(unavailable.code, ResultCode::UnlockRequired);
    assert_eq!(unavailable.operation_id, supplied.operation_id);
    assert!(reserved(&w));

    // And once the server takes the key, the same request goes on.
    w.server.restore_client();
    let retried = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(retried.outcome, Outcome::Noop);
    assert_eq!(retried.operation_id, supplied.operation_id);
    assert!(!reserved(&w));
    Ok(())
}

fn a_server_that_takes_the_client_without_its_key_is_key_rejected() -> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    w.server.accept_anonymous();
    let envelope = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(envelope.code, ResultCode::KeyRejected);
    assert_eq!(envelope.outcome, Outcome::Blocked);
    assert_eq!(actions(&envelope), [RecoveryActionKind::RequestRetry]);
    assert!(reserved(&w));
    Ok(())
}

fn a_stalled_connection_is_transport_unavailable() -> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    // Longer than the transport waits for a server.
    w.server.stall_at(
        FixtureBoundary::Advertisement,
        std::time::Duration::from_secs(45),
    );
    let envelope = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(envelope.code, ResultCode::TransportUnavailable);
    assert_eq!(envelope.outcome, Outcome::Error);
    assert_eq!(actions(&envelope), [RecoveryActionKind::RequestRetry]);
    // It stopped after it reserved, and the same request takes it up.
    assert!(reserved(&w));
    w.server.clear_fault();
    let retried = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(retried.outcome, Outcome::Noop);
    assert!(!reserved(&w));
    Ok(())
}

fn a_poll_holding_the_reservation_is_poll_yielding_and_leaves_no_record() -> Result<(), FixtureError>
{
    let w = World::new()?;
    approve(&w)?;
    // A synchronization first, so that the repository's remote is known
    // to the journal a poll reserves in.
    assert_eq!(execute(&w, REQUEST_4, repo_sync(&w)).outcome, Outcome::Noop);
    let plan = fixed(RemoteRefPlan::from_configuration("origin", "main"))?;
    let poll = fixed(w.service.reserve_remote_operation_with_priority(
        &w.root,
        OperationId::new(),
        &RemoteOperationTarget::for_poll(&plan),
        RemoteOperationPriority::Poll,
    ))?;
    assert!(matches!(poll, RemoteReservationOutcome::Reserved(_)));
    let opened = connections(&w);

    let envelope = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(envelope.code, ResultCode::PollYielding);
    assert_eq!(envelope.outcome, Outcome::Error);
    assert_eq!(envelope.effects, Effects::not_requested());
    assert_eq!(actions(&envelope), [RecoveryActionKind::RequestRetry]);
    assert!(record(&w, REQUEST_1).is_none());
    assert_eq!(connections(&w), opened);
    Ok(())
}

fn a_published_commit_whose_index_hand_off_failed_is_partial_and_its_retry_pushes_nothing()
-> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let repo = w.repo()?;
    let commit = advance(&repo, "refs/heads/main", "index-candidate")?;
    let pushes = accepted(&w.server);
    let db = w.db()?;
    fixed(db.execute_batch("CREATE TRIGGER fail_handoff BEFORE UPDATE ON operation_records WHEN NEW.state='completed' AND NEW.action='refresh' BEGIN SELECT RAISE(ABORT,'fixed failure'); END"))?;

    let partial = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(partial.outcome, Outcome::Partial);
    assert_eq!(partial.code, ResultCode::DiscoveryPending);
    assert_eq!(
        partial.effects,
        Effects {
            publication: PublicationEffect::Published,
            discovery: DiscoveryEffect::Pending,
            commit_oid: Some(commit.to_string()),
            ..Effects::not_requested()
        }
    );
    assert_eq!(actions(&partial), [RecoveryActionKind::OperationResume]);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, commit);
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_eq!(
        record(&w, REQUEST_1).ok_or(FixtureError)?.state,
        RequestState::Accepted
    );
    // Published: the remote's reservation is free, and a cancellation
    // asked for now changes nothing.
    assert!(!reserved(&w));
    let cancel = w.service.cancel_request(request(REQUEST_1));
    assert!(cancel.data.ok_or(FixtureError)?.requested);

    fixed(db.execute_batch("DROP TRIGGER fail_handoff"))?;
    let opened = connections(&w);
    let finished = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(finished.outcome, Outcome::Success);
    assert_eq!(finished.effects, published(commit));
    assert_eq!(finished.operation_id, partial.operation_id);
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_eq!(connections(&w), opened);
    assert_eq!(
        record(&w, REQUEST_1).ok_or(FixtureError)?.state,
        RequestState::Finished
    );
    Ok(())
}

// Progress.

/// Planted in the message of the commit a synchronization publishes.
const MESSAGE: &str = "commit-message-private-progress-sentinel";

fn stages(events: &[ProgressEvent]) -> Vec<ProgressStage> {
    events.iter().map(|event| event.stage).collect()
}

/// Commits a file to `reference` under a message that is private.
fn advance_with_message(
    repo: &git2::Repository,
    reference: &str,
    name: &str,
) -> Result<git2::Oid, FixtureError> {
    let parent = fixed(repo.find_commit(fixed(repo.refname_to_id(reference))?))?;
    let mut builder = fixed(repo.treebuilder(Some(&fixed(parent.tree())?)))?;
    fixed(builder.insert(name, fixed(repo.blob(name.as_bytes()))?, 0o100644))?;
    let tree = fixed(repo.find_tree(fixed(builder.write())?))?;
    let signature = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    let new = fixed(repo.commit(None, &signature, &signature, MESSAGE, &tree, &[&parent]))?;
    fixed(repo.checkout_tree(
        &fixed(repo.find_object(new, None))?,
        Some(git2::build::CheckoutBuilder::new().safe()),
    ))?;
    fixed(repo.reference(reference, new, true, "owned fixture advance"))?;
    Ok(new)
}

fn the_events_of_a_synchronization_arrive_in_safe_point_order_and_hold_nothing_private()
-> Result<(), FixtureError> {
    use ProgressStage as S;
    let mut w = World::new()?;
    approve(&w)?;
    w.probes.push(MESSAGE.as_bytes().to_vec());
    ssh_privacy::save(&w.probes)?;
    // The remote is ahead by one commit and the local branch by none:
    // the first synchronization updates the local branch.
    let incoming = advance_with_message(&w.peer, "refs/heads/main", "peer-advance")?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    let mut events = Events::default();
    let updated = call(
        &w,
        &w.service,
        REQUEST_1,
        repo_sync(&w),
        Provider,
        &mut events,
        &CancellationToken::new(),
    );
    assert_eq!(updated.outcome, Outcome::Noop);
    assert_eq!(
        stages(&events.0),
        [
            S::Accepted,
            S::Connecting,
            S::Fetching,
            S::Observing,
            S::UpdatingLocal,
            S::Observing,
            S::Indexing
        ]
    );
    assert_eq!(fixed(w.repo()?.refname_to_id("refs/heads/main"))?, incoming);

    // Then the local branch is ahead: the second pushes.
    let repo = w.repo()?;
    let local = advance_with_message(&repo, "refs/heads/main", "local-advance")?;
    let mut events = Events::default();
    let pushed = call(
        &w,
        &w.service,
        REQUEST_2,
        repo_sync(&w),
        Provider,
        &mut events,
        &CancellationToken::new(),
    );
    // Every event was sent before the result was returned: the sink
    // holds them all here, and nothing is sent afterwards.
    assert_eq!(pushed.effects, published(local));
    assert_eq!(
        stages(&events.0),
        [
            S::Accepted,
            S::Connecting,
            S::Fetching,
            S::Observing,
            S::Pushing,
            S::Verifying,
            S::Indexing
        ]
    );
    for event in &events.0 {
        let value = fixed(serde_json::to_value(event))?;
        fixed(schema::check_published(
            "progress_event.schema.json",
            &value,
        ))?;
        assert_eq!(event.request_id, REQUEST_2);
        assert_eq!(event.operation_id, pushed.operation_id);
        assert_eq!(event.command, "repo sync");
        assert_eq!(event.scope, pushed.scope);
        let rendered = fixed(serde_json::to_vec(event))?;
        ssh_privacy::clean(&rendered, &w.probes)?;
        ssh_privacy::clean(format!("{event:?}").as_bytes(), &w.probes)?;
    }
    // Nothing is published until the push has returned, and then the
    // commit is.
    for event in &events.0[..5] {
        assert_eq!(event.effects.commit_oid, None, "{:?}", event.stage);
    }
    for event in &events.0[5..] {
        assert_eq!(
            event.effects.publication,
            PublicationEffect::Published,
            "{:?}",
            event.stage
        );
        assert_eq!(event.effects.commit_oid, Some(local.to_string()));
    }
    Ok(())
}

// Cancellation.

fn a_token_cancelled_before_execute_does_nothing() -> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    advance(&w.repo()?, "refs/heads/main", "local-advance")?;
    let opened = connections(&w);
    let pushes = accepted(&w.server);
    let token = CancellationToken::new();
    token.cancel();
    let mut events = Events::default();
    let envelope = call(
        &w,
        &w.service,
        REQUEST_1,
        repo_sync(&w),
        Provider,
        &mut events,
        &token,
    );
    assert_eq!(envelope.outcome, Outcome::Cancelled);
    assert_eq!(envelope.code, ResultCode::Cancelled);
    assert_eq!(envelope.effects, Effects::not_requested());
    assert_eq!(envelope.operation_id, None);
    assert!(events.0.is_empty());
    assert!(record(&w, REQUEST_1).is_none());
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes);
    assert!(!reserved(&w));
    Ok(())
}

/// A sink that cancels its call's token when it is told of `stage`.
struct CancelAt {
    stage: ProgressStage,
    token: CancellationToken,
    seen: Vec<ProgressStage>,
}

impl ProgressSink for CancelAt {
    fn event(&mut self, event: &ProgressEvent) {
        self.seen.push(event.stage);
        if event.stage == self.stage {
            self.token.cancel();
        }
    }
}

fn a_token_cancelled_from_the_sink_stops_at_the_first_safe_point() -> Result<(), FixtureError> {
    use ProgressStage as S;
    let w = World::new()?;
    approve(&w)?;
    let local = advance(&w.repo()?, "refs/heads/main", "local-advance")?;
    let served = fixed(w.bare()?.refname_to_id("refs/heads/main"))?;
    let opened = connections(&w);
    let pushes = accepted(&w.server);
    let token = CancellationToken::new();
    // The first safe point is the one before the fetch.
    let mut sink = CancelAt {
        stage: S::Fetching,
        token: token.clone(),
        seen: Vec::new(),
    };
    let cancelled = call(
        &w,
        &w.service,
        REQUEST_1,
        repo_sync(&w),
        Provider,
        &mut sink,
        &token,
    );
    assert_eq!(cancelled.outcome, Outcome::Cancelled);
    assert_eq!(cancelled.code, ResultCode::Cancelled);
    assert_eq!(cancelled.effects.commit_oid, None);
    assert!(cancelled.operation_id.is_some());
    assert!(cancelled.recovery.is_empty());
    assert_eq!(
        sink.seen,
        [S::Accepted, S::Connecting, S::Fetching, S::Stopping]
    );
    // It stopped before it opened a connection, and pushed nothing.
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, served);
    assert!(!reserved(&w));
    assert_eq!(
        record(&w, REQUEST_1).ok_or(FixtureError)?.state,
        RequestState::Finished
    );

    // The same request returns the stored result, whatever its token.
    let again = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(again, cancelled);
    assert_eq!(connections(&w), opened);
    // A new request synchronizes.
    let fresh = execute(&w, REQUEST_2, repo_sync(&w));
    assert_eq!(fresh.effects, published(local));
    assert_eq!(accepted(&w.server), pushes + 1);
    Ok(())
}

/// A call of `execute` on another thread, by a service of its own on
/// the world's data. The world itself stays on the test's thread.
fn execute_elsewhere(
    w: &World,
    id: &'static str,
) -> Result<std::thread::JoinHandle<Envelope<MutationDataDto>>, FixtureError> {
    let service = fixed(RepositoryService::open_at(&w.data()))?;
    let root = w.root.clone();
    Ok(std::thread::spawn(move || {
        service.execute(
            MutationCall {
                request_id: request(id),
                confirmation: None,
                mutation: Mutation::RepoSync(RepoSyncInput { root }),
            },
            &mut SessionCredentials::new(Provider),
            &mut NoProgress,
            &CancellationToken::new(),
        )
    }))
}

/// What a call on another thread returned, scanned as `call` scans.
fn joined(
    w: &World,
    handle: std::thread::JoinHandle<Envelope<MutationDataDto>>,
) -> Result<Envelope<MutationDataDto>, FixtureError> {
    let envelope = handle.join().map_err(|_| FixtureError)?;
    ssh_privacy::clean(&fixed(serde_json::to_vec(&envelope))?, &w.probes)?;
    w.privacy()?;
    Ok(envelope)
}

fn a_cancellation_requested_while_the_transport_is_blocked_is_honored_when_it_returns()
-> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    advance(&w.repo()?, "refs/heads/main", "local-advance")?;
    let served = fixed(w.bare()?.refname_to_id("refs/heads/main"))?;
    let pushes = accepted(&w.server);
    let hold = w.server.hold_advertisement()?;
    let running = execute_elsewhere(&w, REQUEST_1)?;
    hold.wait_until_held()?;

    // The synchronization is inside its first transport call, with the
    // reservation its own.
    assert!(reserved(&w));
    let opened = connections(&w);
    let cancel = w.service.cancel_request(request(REQUEST_1));
    assert_eq!(cancel.command, "request cancel");
    assert_eq!(cancel.code, ResultCode::Ok);
    assert_eq!(cancel.request_id, None);
    let asked = cancel.data.ok_or(FixtureError)?;
    assert_eq!(asked.request_id, REQUEST_1);
    assert_eq!(asked.state, RequestState::Accepted);
    assert!(asked.requested);
    // The transport call has not returned: nothing has acknowledged it.
    assert!(!asked.acknowledged);
    assert!(reserved(&w));

    hold.release()?;
    let cancelled = joined(&w, running)?;
    assert_eq!(cancelled.outcome, Outcome::Cancelled);
    assert_eq!(cancelled.code, ResultCode::Cancelled);
    // The operation itself was asked, so it stopped as soon as the
    // transport call returned: it did not go on to fetch and stop at
    // the safe point after that.
    assert_eq!(connections(&w), opened);
    assert_eq!(cancelled.effects.commit_oid, None);
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, served);
    assert!(!reserved(&w));
    assert_eq!(
        record(&w, REQUEST_1).ok_or(FixtureError)?.state,
        RequestState::Finished
    );
    let after = w.service.cancel_request(request(REQUEST_1));
    let after = after.data.ok_or(FixtureError)?;
    assert_eq!(after.state, RequestState::Finished);
    assert!(!after.requested);
    assert!(after.acknowledged);
    // A request no record holds.
    let unknown = w.service.cancel_request(request(REQUEST_3));
    assert_eq!(unknown.code, ResultCode::RequestNotFound);
    assert!(unknown.data.is_none());
    Ok(())
}

fn a_cancellation_requested_before_the_reservation_exists_is_honored_at_the_first_safe_point()
-> Result<(), FixtureError> {
    use ProgressStage as S;
    let w = World::new()?;
    approve(&w)?;
    advance(&w.repo()?, "refs/heads/main", "local-advance")?;
    let opened = connections(&w);
    let pushes = accepted(&w.server);
    // The call is stopped where the boundary has accepted the request
    // and has not yet made its domain call: there is no reservation.
    let elsewhere = fixed(RepositoryService::open_at(&w.data()))?;
    let root = w.root.clone();
    let seen = std::sync::Arc::new(std::sync::Mutex::new(None));
    let report = seen.clone();
    w.service.set_request_hook_for_testing(move || {
        let reserved = elsewhere.active_remote_operation(&root).unwrap().is_some();
        let cancel = elsewhere.cancel_request(request(REQUEST_1));
        *report.lock().unwrap() = Some((reserved, cancel.data));
    });
    let mut events = Events::default();
    let cancelled = call(
        &w,
        &w.service,
        REQUEST_1,
        repo_sync(&w),
        Provider,
        &mut events,
        &CancellationToken::new(),
    );
    let (was_reserved, asked) = seen.lock().unwrap().take().ok_or(FixtureError)?;
    assert!(!was_reserved);
    let asked = asked.ok_or(FixtureError)?;
    assert!(asked.requested);
    assert!(!asked.acknowledged);

    assert_eq!(cancelled.outcome, Outcome::Cancelled);
    assert_eq!(cancelled.code, ResultCode::Cancelled);
    assert_eq!(
        stages(&events.0),
        [S::Accepted, S::Connecting, S::Fetching, S::Stopping]
    );
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes);
    assert!(!reserved(&w));
    let stored = record(&w, REQUEST_1).ok_or(FixtureError)?;
    assert_eq!(stored.state, RequestState::Finished);
    // The flag was cleared as it was honored.
    assert!(!stored.cancel_requested);
    Ok(())
}

/// A sink that, when it is told of `stage`, has the request's operation
/// asked to give way, as a poll is: the synchronization then reports
/// itself interrupted, and its row is not cancelled.
struct YieldAt {
    stage: ProgressStage,
    registry: std::path::PathBuf,
}

impl ProgressSink for YieldAt {
    fn event(&mut self, event: &ProgressEvent) {
        if event.stage == self.stage {
            rusqlite::Connection::open(&self.registry)
                .unwrap()
                .execute(
                    "UPDATE remote_operation_records SET yield_requested=1 WHERE operation_ulid=?1",
                    [event.operation_id.as_deref().unwrap()],
                )
                .unwrap();
        }
    }
}

fn an_interrupted_synchronization_is_cancelled_only_when_its_row_says_so()
-> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let local = advance(&w.repo()?, "refs/heads/main", "local-advance")?;
    let pushes = accepted(&w.server);
    let mut sink = YieldAt {
        stage: ProgressStage::Pushing,
        registry: w.data().join(REGISTRY_FILE),
    };
    let interrupted = call(
        &w,
        &w.service,
        REQUEST_1,
        repo_sync(&w),
        Provider,
        &mut sink,
        &CancellationToken::new(),
    );
    let operation = OperationId::parse(interrupted.operation_id.as_deref().ok_or(FixtureError)?)
        .map_err(|_| FixtureError)?;
    assert_eq!(phase(&w, operation)?, "interrupted");
    // Interrupted, and not cancelled: the row is what says which.
    assert_eq!(interrupted.code, ResultCode::RecoveryRequired);
    assert_eq!(interrupted.outcome, Outcome::Blocked);
    assert_eq!(
        actions(&interrupted),
        [
            RecoveryActionKind::RequestRetry,
            RecoveryActionKind::RepoInspect
        ]
    );
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(
        record(&w, REQUEST_1).ok_or(FixtureError)?.state,
        RequestState::Accepted
    );
    // The same request takes it up and finishes it.
    let finished = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(finished.effects, published(local));
    assert_eq!(finished.operation_id, interrupted.operation_id);
    assert_eq!(accepted(&w.server), pushes + 1);
    Ok(())
}

fn a_retry_while_the_first_call_is_still_running_takes_the_synchronization_over()
-> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let local = advance(&w.repo()?, "refs/heads/main", "local-advance")?;
    let pushes = accepted(&w.server);
    let hold = w.server.hold_advertisement()?;
    let first = execute_elsewhere(&w, REQUEST_1)?;
    hold.wait_until_held()?;
    assert_eq!(
        record(&w, REQUEST_1).ok_or(FixtureError)?.state,
        RequestState::Accepted
    );

    // The retry enters the same record and takes the operation from
    // the call that is still inside its transport.
    let retried = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(retried.outcome, Outcome::Success);
    assert_eq!(retried.effects, published(local));
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_eq!(
        record(&w, REQUEST_1).ok_or(FixtureError)?.state,
        RequestState::Finished
    );

    // The first call comes back fenced: it pushes nothing and settles
    // nothing. What it reports is the operation as its row then stands,
    // which the retry has finished: the commit is published, and the row,
    // completed and owing no work, shows that its index hand-off ran.
    // Its error is no result of an operation that has completed, so the
    // same request is what it offers first: the record answers it.
    hold.release()?;
    let fenced = joined(&w, first)?;
    assert_eq!(fenced.code, ResultCode::RecoveryRequired);
    assert_eq!(fenced.outcome, Outcome::Partial);
    assert_eq!(fenced.operation_id, retried.operation_id);
    assert_eq!(fenced.effects, published(local));
    assert_eq!(
        actions(&fenced),
        [
            RecoveryActionKind::RequestRetry,
            RecoveryActionKind::RepoInspect
        ]
    );
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, local);
    // The record is the retry's, and every later call is answered
    // from it.
    assert_eq!(execute(&w, REQUEST_1, repo_sync(&w)), retried);
    Ok(())
}

// After the cache is lost.

fn a_synchronization_retried_after_the_cache_is_lost_is_blocked_until_each_piece_is_restored()
-> Result<(), FixtureError> {
    let mut w = World::new()?;
    approve(&w)?;
    let local = advance(&w.repo()?, "refs/heads/main", "local-advance")?;
    let first = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(first.effects, published(local));
    let pushes = accepted(&w.server);

    // The database is lost, and with it the request's record, the
    // journals, the registration, the selected key and the host's pin.
    lose_the_cache(&mut w)?;
    let unscanned = |id: &str, mutation: Mutation| {
        call_before_the_stores_hold_anything(
            &w,
            &w.service,
            id,
            mutation,
            Provider,
            &mut NoProgress,
            &CancellationToken::new(),
        )
    };

    // The repository is not registered.
    let opened = connections(&w);
    let unregistered = unscanned(REQUEST_1, repo_sync(&w));
    assert_eq!(unregistered.code, ResultCode::RepositoryNotRegistered);
    assert_eq!(unregistered.outcome, Outcome::Blocked);
    assert_eq!(actions(&unregistered), [RecoveryActionKind::IndexRebuild]);
    assert_eq!(unregistered.operation_id, None);
    assert!(fixed(w.service.request_record(request(REQUEST_1)))?.is_none());
    assert_eq!(connections(&w), opened);
    let rebuilt = unscanned(
        REQUEST_2,
        Mutation::IndexRebuild(IndexRebuildInput {
            root: w.root.clone(),
        }),
    );
    assert_eq!(rebuilt.outcome, Outcome::Success);

    // No key is selected. The request is new: it has no record, and is
    // accepted under a new operation, which reserves before it finds
    // that out.
    let keyless = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(keyless.code, ResultCode::NoSelectedKey);
    assert_eq!(keyless.outcome, Outcome::Blocked);
    assert_eq!(actions(&keyless), [RecoveryActionKind::RequestRetry]);
    assert!(keyless.operation_id.is_some());
    assert_ne!(keyless.operation_id, first.operation_id);
    assert_eq!(connections(&w), opened);
    assert!(reserved(&w));
    select_the_world_s_key(&w)?;

    // Selecting a key fences every remote operation of the registry:
    // the one this request reserved under is interrupted, its
    // reservation let go, and nothing takes it up again. The request
    // that stopped for want of a key is therefore not the one that goes
    // on once there is a key. It had published nothing, and is finished
    // as an external change, which does not offer the same request.
    assert!(!reserved(&w));
    let fenced = execute(&w, REQUEST_1, repo_sync(&w));
    assert_fenced_before_a_push(&w, REQUEST_1, &fenced)?;
    assert_eq!(fenced.operation_id, keyless.operation_id);
    assert_eq!(connections(&w), opened);

    // A new request goes on: the host is not approved.
    let untrusted = execute(&w, REQUEST_3, repo_sync(&w));
    assert_eq!(untrusted.code, ResultCode::HostApprovalRequired);
    assert_host_action(&w, &untrusted, RecoveryActionKind::HostApprove)?;
    assert_eq!(
        synchronization_data(&untrusted)?.presented_fingerprint,
        w.server.host_identity().sha256
    );
    assert!(reserved(&w));
    approve(&w)?;

    // An approval fences nothing: with every piece restored, the same
    // request finds what a fresh synchronization finds. The remote holds
    // the commit already, and nothing is pushed.
    let current = execute(&w, REQUEST_3, repo_sync(&w));
    assert_current(&current);
    assert_eq!(current.operation_id, untrusted.operation_id);
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, local);
    assert!(!reserved(&w));
    assert_eq!(
        record(&w, REQUEST_3).ok_or(FixtureError)?.state,
        RequestState::Finished
    );
    Ok(())
}

fn select_the_world_s_key(w: &World) -> Result<(), FixtureError> {
    let RegisterSharedKeyOutcome::Registered(key) =
        fixed(w.service.register_shared_key(RegisterSharedKeyRequest {
            label: "acceptance-selected".into(),
            ownership: SharedKeyOwnership::Imported,
            private_key_path: w.server.client_key_path().into(),
            public_key_path: None,
        }))?
    else {
        return Err(FixtureError);
    };
    fixed(w.service.select_shared_key(key.id))?;
    Ok(())
}

/// The remote held the commit already: a no-op that names no commit.
fn assert_current(envelope: &Envelope<MutationDataDto>) {
    assert_eq!(envelope.outcome, Outcome::Noop);
    assert_eq!(envelope.code, ResultCode::AlreadyApplied);
    assert_eq!(
        envelope.effects,
        Effects {
            publication: PublicationEffect::Current,
            discovery: DiscoveryEffect::Current,
            ..Effects::not_requested()
        }
    );
}

/// The database, with everything it held, is deleted, and the service
/// opened again on what is left.
fn lose_the_cache(w: &mut World) -> Result<(), FixtureError> {
    for entry in fixed(std::fs::read_dir(w.data()))? {
        let path = fixed(entry)?.path();
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(REGISTRY_FILE))
        {
            fixed(std::fs::remove_file(path))?;
        }
    }
    w.service = fixed(RepositoryService::open_at(&w.data()))?;
    Ok(())
}

/// The design's own sequence: everything is restored before the request
/// is sent again. It has no record, is taken as new, and reports what a
/// fresh synchronization finds.
fn a_synchronization_retried_once_everything_is_restored_is_current_and_pushes_nothing()
-> Result<(), FixtureError> {
    let mut w = World::new()?;
    approve(&w)?;
    let local = advance(&w.repo()?, "refs/heads/main", "local-advance")?;
    let first = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(first.effects, published(local));
    let pushes = accepted(&w.server);

    lose_the_cache(&mut w)?;
    let rebuilt = call_before_the_stores_hold_anything(
        &w,
        &w.service,
        REQUEST_2,
        Mutation::IndexRebuild(IndexRebuildInput {
            root: w.root.clone(),
        }),
        Provider,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(rebuilt.outcome, Outcome::Success);
    select_the_world_s_key(&w)?;
    approve(&w)?;

    let current = execute(&w, REQUEST_1, repo_sync(&w));
    assert_current(&current);
    assert_ne!(current.operation_id, first.operation_id);
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, local);
    assert!(!reserved(&w));
    Ok(())
}

// The harness's scan of everything a case writes to its output.

/// What a control writes to its output besides what the boundary gave
/// it.
#[derive(Clone, Copy)]
enum Printed {
    /// Nothing: every event and the result, as they are.
    Emitted,
    RemoteLocation,
    Passphrase,
    CommitMessage,
}

/// A synchronization that publishes a commit whose message is private,
/// with everything the boundary emitted of it written to the case's
/// output, and then what `printed` names.
fn control_prints(printed: Printed) -> Result<(), FixtureError> {
    let mut w = World::new()?;
    approve(&w)?;
    w.probes.push(MESSAGE.as_bytes().to_vec());
    ssh_privacy::save(&w.probes)?;
    let local = advance_with_message(&w.repo()?, "refs/heads/main", "local-advance")?;
    let mut events = Events::default();
    let envelope = call(
        &w,
        &w.service,
        REQUEST_1,
        repo_sync(&w),
        Provider,
        &mut events,
        &CancellationToken::new(),
    );
    assert_eq!(envelope.effects, published(local));
    assert_eq!(events.0.len(), 7);
    for event in &events.0 {
        println!("{}", fixed(serde_json::to_string(event))?);
        eprintln!("{event:?}");
    }
    println!("{}", fixed(serde_json::to_string(&envelope))?);
    eprintln!("{envelope:?}");
    match printed {
        Printed::Emitted => {}
        Printed::RemoteLocation => println!("{}", w.server.url()),
        Printed::Passphrase => eprintln!("{PASSWORD}"),
        Printed::CommitMessage => println!("{MESSAGE}"),
    }
    Ok(())
}

fn the_output_privacy_scan_passes_what_the_boundary_emits_and_fails_what_is_private()
-> Result<(), FixtureError> {
    ssh_privacy::save(&[b"runner-private-capture-control".to_vec()])?;
    // The scan passes a case that prints every event and its result.
    assert_eq!(
        ssh_harness::run_isolated_with_output_privacy("control_prints_what_the_boundary_emits"),
        Ok(())
    );
    // And it can fail: the same case, with one private thing more.
    for control in [
        "control_prints_the_remote_location",
        "control_prints_the_passphrase",
        "control_prints_the_commit_message",
    ] {
        assert_eq!(
            ssh_harness::run_isolated_with_output_privacy(control),
            Err(ssh_harness::IsolationFailure::OutputPrivacy),
            "{control}"
        );
    }
    Ok(())
}

// A final answer never understates a push.

/// What a synchronization that was stopped after the remote took its
/// push reports: cancelled, with the publication and its commit.
fn published_and_cancelled(commit: git2::Oid) -> Effects {
    Effects {
        publication: PublicationEffect::Published,
        discovery: DiscoveryEffect::Pending,
        commit_oid: Some(commit.to_string()),
        ..Effects::not_requested()
    }
}

/// Runs `during` once, inside the call's push: after everything that
/// could stop the synchronization before it pushes, and before a byte
/// is sent. This is the transport's own test seam, on the thread that
/// runs the call; what `during` does lands while the push is under way.
fn during_the_push(
    during: impl FnOnce() + 'static,
) -> repository::transport::operation_tests::HookGuard {
    let mut during = Some(during);
    repository::transport::operation_tests::install_hook(move |point| {
        if point == repository::transport::operation_tests::Checkpoint::ExactPushStarted
            && let Some(during) = during.take()
        {
            during();
        }
    })
}

/// A cancellation that arrives while the call is inside its push, by
/// the call's token or by `request cancel`. The push goes through, and
/// either way the answer says so, and is the same answer.
fn a_cancellation_during_the_push_reports_the_push(by_token: bool) -> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let local = advance(&w.repo()?, "refs/heads/main", "local-advance")?;
    let pushes = accepted(&w.server);
    let token = CancellationToken::new();
    let asked = std::rc::Rc::new(std::cell::RefCell::new(None));
    let hook = {
        let token = token.clone();
        let asked = asked.clone();
        let elsewhere = fixed(RepositoryService::open_at(&w.data()))?;
        during_the_push(move || {
            if by_token {
                token.cancel();
            } else {
                *asked.borrow_mut() = elsewhere.cancel_request(request(REQUEST_1)).data;
            }
        })
    };
    let cancelled = call(
        &w,
        &w.service,
        REQUEST_1,
        repo_sync(&w),
        Provider,
        &mut NoProgress,
        &token,
    );
    drop(hook);
    if !by_token {
        let asked = asked.borrow_mut().take().ok_or(FixtureError)?;
        assert!(asked.requested);
        assert!(!asked.acknowledged);
    }

    // The server took the push.
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, local);
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_eq!(cancelled.code, ResultCode::Cancelled);
    assert_eq!(cancelled.outcome, Outcome::Cancelled);
    assert_eq!(cancelled.effects, published_and_cancelled(local));
    assert!(cancelled.recovery.is_empty());
    assert!(!reserved(&w));
    let stored = record(&w, REQUEST_1).ok_or(FixtureError)?;
    assert_eq!(stored.state, RequestState::Finished);
    assert!(!stored.cancel_requested);
    assert_eq!(execute(&w, REQUEST_1, repo_sync(&w)), cancelled);
    assert_eq!(accepted(&w.server), pushes + 1);
    Ok(())
}

/// A push the server took and whose result was lost, and then a
/// cancellation: `request cancel` before the retry, or the retry's own
/// token once it has verified the push. The retry asks the remote
/// before it answers, and the answer names what the remote holds.
fn a_cancellation_after_a_lost_push_reports_the_push(by_token: bool) -> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let local = advance(&w.repo()?, "refs/heads/main", "lost-output")?;
    let pushes = accepted(&w.server);
    w.server.disconnect_at(FixtureBoundary::AfterReceivePack);
    let lost = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(lost.code, ResultCode::TransportUnavailable);
    assert_eq!(lost.effects.publication, PublicationEffect::Pending);
    w.server.clear_fault();
    assert_eq!(accepted(&w.server), pushes + 1);

    let token = CancellationToken::new();
    let cancelled = if by_token {
        // The retry's sink cancels once the push is verified.
        let mut sink = CancelAt {
            stage: ProgressStage::Indexing,
            token: token.clone(),
            seen: Vec::new(),
        };
        call(
            &w,
            &w.service,
            REQUEST_1,
            repo_sync(&w),
            Provider,
            &mut sink,
            &token,
        )
    } else {
        let asked = w
            .service
            .cancel_request(request(REQUEST_1))
            .data
            .ok_or(FixtureError)?;
        assert!(asked.requested);
        assert!(!asked.acknowledged);
        execute(&w, REQUEST_1, repo_sync(&w))
    };
    assert_eq!(cancelled.code, ResultCode::Cancelled);
    assert_eq!(cancelled.outcome, Outcome::Cancelled);
    assert_eq!(cancelled.effects, published_and_cancelled(local));
    assert_eq!(cancelled.operation_id, lost.operation_id);
    assert_eq!(accepted(&w.server), pushes + 1);
    assert!(!reserved(&w));
    assert_eq!(
        record(&w, REQUEST_1).ok_or(FixtureError)?.state,
        RequestState::Finished
    );
    assert_eq!(execute(&w, REQUEST_1, repo_sync(&w)), cancelled);
    Ok(())
}

/// The review's I3. A push is taken and its result lost; a peer then
/// builds on the pushed commit; the request is retried. Taking that up
/// under the same operation would need the journal of merges, which a
/// caller that never merges does not open: the retry is
/// `merge_required`, nothing of a merge or of a publication attempt is
/// left, and the result still says the remote holds the commit.
fn a_lost_push_the_remote_has_since_built_on_is_merge_required_and_leaves_nothing_of_a_merge()
-> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let repo = w.repo()?;
    let local = advance(&repo, "refs/heads/main", "lost-output")?;
    let pushes = accepted(&w.server);
    w.server.disconnect_at(FixtureBoundary::AfterReceivePack);
    let lost = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(lost.code, ResultCode::TransportUnavailable);
    w.server.clear_fault();
    // A peer builds on what the server took.
    let server = w.bare()?;
    let built = commit(&server, local, "peer-on-top", b"peer-on-top")?;
    fixed(server.reference("refs/heads/main", built, true, "fixture"))?;

    for _ in 0..2 {
        // The retry, and the retry of that: a re-entry both times.
        let refused = execute(&w, REQUEST_1, repo_sync(&w));
        assert_eq!(refused.code, ResultCode::MergeRequired);
        // The fetch showed that the remote holds the commit.
        assert_eq!(refused.outcome, Outcome::Partial);
        assert_eq!(refused.effects, published_and_cancelled(local));
        assert_eq!(actions(&refused), [RecoveryActionKind::RequestCancel]);
        assert_eq!(refused.operation_id, lost.operation_id);
        assert_eq!(merge_rows(&w)?, 0);
        assert_eq!(merge_commits(&repo)?, 0);
        assert_eq!(fixed(repo.refname_to_id("refs/heads/main"))?, local);
        assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, built);
        assert!(reserved(&w));
    }
    // It is ended as every held reservation is, and still says what the
    // remote holds.
    assert!(
        w.service
            .cancel_request(request(REQUEST_1))
            .data
            .ok_or(FixtureError)?
            .requested
    );
    let ended = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(ended.code, ResultCode::Cancelled);
    assert_eq!(ended.effects, published_and_cancelled(local));
    assert_eq!(merge_rows(&w)?, 0);
    assert!(!reserved(&w));
    assert_eq!(accepted(&w.server), pushes + 1);
    // A new request brings the local branch up to the remote's.
    let fresh = execute(&w, REQUEST_2, repo_sync(&w));
    assert_current(&fresh);
    assert_eq!(fixed(repo.refname_to_id("refs/heads/main"))?, built);
    Ok(())
}

/// The review's I5. `merge_required` is dealt with by rebasing the
/// local branch onto the remote's, and the same request does not go on
/// from there: its operation recorded the branch as it was. So the
/// result does not offer the same request: it offers its cancellation.
fn merge_required_is_not_finished_by_the_same_request_after_the_branch_is_rebased()
-> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let repo = w.repo()?;
    advance(&repo, "refs/heads/main", "local-divergent")?;
    let incoming = advance(&w.peer, "refs/heads/main", "remote-divergent")?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    let refused = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(refused.code, ResultCode::MergeRequired);
    assert_eq!(actions(&refused), [RecoveryActionKind::RequestCancel]);

    // The user rebases: the local change, on top of the remote's.
    let rebased = commit(&repo, incoming, "local-divergent", b"local-divergent")?;
    fixed(repo.reference("refs/heads/main", rebased, true, "fixture"))?;
    fixed(repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force())))?;

    let again = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(again.code, ResultCode::RecoveryRequired);
    assert!(reserved(&w));
    assert!(
        w.service
            .cancel_request(request(REQUEST_1))
            .data
            .ok_or(FixtureError)?
            .requested
    );
    assert_eq!(
        execute(&w, REQUEST_1, repo_sync(&w)).code,
        ResultCode::Cancelled
    );
    let fresh = execute(&w, REQUEST_2, repo_sync(&w));
    assert_eq!(fresh.effects, published(rebased));
    Ok(())
}

// Fenced operations: begun under a configuration the registration has
// since left.

/// The final answer of a request whose operation was fenced before it
/// intended any push: an external change, with nothing published and
/// nothing offered, stored and replayed without a connection.
fn assert_fenced_before_a_push(
    w: &World,
    id: &str,
    fenced: &Envelope<MutationDataDto>,
) -> Result<(), FixtureError> {
    assert_eq!(fenced.code, ResultCode::ExternalChange);
    assert_eq!(fenced.outcome, Outcome::Blocked);
    assert_eq!(fenced.effects, Effects::not_requested());
    assert!(fenced.recovery.is_empty());
    assert_final(w, id, fenced)
}

/// The result is stored, and every later call of the request returns
/// it without opening a connection.
fn assert_final(
    w: &World,
    id: &str,
    result: &Envelope<MutationDataDto>,
) -> Result<(), FixtureError> {
    let stored = record(w, id).ok_or(FixtureError)?;
    assert_eq!(stored.state, RequestState::Finished);
    let opened = connections(w);
    let mutation = match stored.command.as_str() {
        "item sync" => item_sync(w),
        _ => repo_sync(w),
    };
    assert_eq!(&execute(w, id, mutation), result);
    assert_eq!(connections(w), opened);
    // Asking it to stop asks nothing.
    let asked = w
        .service
        .cancel_request(request(id))
        .data
        .ok_or(FixtureError)?;
    assert_eq!(asked.state, RequestState::Finished);
    assert!(!asked.requested);
    // Acknowledged is said of a request that ended cancelled, and of
    // no other.
    assert_eq!(asked.acknowledged, result.code == ResultCode::Cancelled);
    Ok(())
}

/// Selects the selected key again, by way of no selection: what a user
/// who changes the key does, and what fences every remote operation of
/// the registry.
fn select_the_key_again(w: &World) -> Result<(), FixtureError> {
    let selected = fixed(w.service.list_shared_keys())?
        .into_iter()
        .find(|key| key.selected)
        .ok_or(FixtureError)?;
    fixed(w.service.clear_shared_key_selection())?;
    fixed(w.service.select_shared_key(selected.id))?;
    Ok(())
}

/// A synchronization that stopped, holding the reservation, before it
/// intended any push: there is no passphrase to unlock its key with.
fn held_before_a_push(w: &World, id: &str) -> Envelope<MutationDataDto> {
    let held = call(
        w,
        &w.service,
        id,
        repo_sync(w),
        NoPassphrase,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(held.code, ResultCode::UnlockRequired);
    assert!(reserved(w));
    held
}

fn request_cancel_finishes_a_request_whose_operation_was_fenced() -> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    advance(&w.repo()?, "refs/heads/main", "local-advance")?;
    let held = held_before_a_push(&w, REQUEST_1);
    select_the_key_again(&w)?;
    assert!(!reserved(&w));

    // It is not asked something nobody will answer: the record is
    // finished, with the answer the request's own call would give.
    let cancel = w.service.cancel_request(request(REQUEST_1));
    assert_eq!(cancel.code, ResultCode::Ok);
    let asked = cancel.data.ok_or(FixtureError)?;
    assert_eq!(asked.state, RequestState::Finished);
    assert!(!asked.requested);
    assert!(!asked.acknowledged);
    let stored = record(&w, REQUEST_1).ok_or(FixtureError)?;
    assert_eq!(stored.state, RequestState::Finished);
    assert!(!stored.cancel_requested);

    let fenced = execute(&w, REQUEST_1, repo_sync(&w));
    assert_fenced_before_a_push(&w, REQUEST_1, &fenced)?;
    assert_eq!(fenced.operation_id, held.operation_id);
    // A new request goes on.
    assert_eq!(
        execute(&w, REQUEST_2, repo_sync(&w)).outcome,
        Outcome::Success
    );
    Ok(())
}

fn a_changed_endpoint_fences_a_held_synchronization_at_the_next_call_of_any_request()
-> Result<(), FixtureError> {
    let mut w = World::new()?;
    approve(&w)?;
    let held = held_before_a_push(&w, REQUEST_1);
    // The remote is pointed somewhere else. Nothing is fenced yet: the
    // change is found by the next synchronization that is called.
    let elsewhere = SshRemoteFixture::start()?;
    w.probes.push(elsewhere.url().into_bytes());
    ssh_privacy::save(&w.probes)?;
    fixed(w.repo()?.remote_set_url("origin", &elsewhere.url()))?;
    assert!(reserved(&w));

    // The same request's own next call finds it.
    let fenced = execute(&w, REQUEST_1, repo_sync(&w));
    assert_fenced_before_a_push(&w, REQUEST_1, &fenced)?;
    assert_eq!(fenced.operation_id, held.operation_id);
    assert!(!reserved(&w));

    // And so does another request's: it fences the one that holds the
    // reservation, and takes the reservation itself.
    fixed(w.repo()?.remote_set_url("origin", &w.server.url()))?;
    let held = held_before_a_push(&w, REQUEST_2);
    fixed(w.repo()?.remote_set_url("origin", &elsewhere.url()))?;
    let other = execute(&w, REQUEST_3, repo_sync(&w));
    assert_eq!(other.code, ResultCode::HostApprovalRequired);
    assert_ne!(other.operation_id, held.operation_id);
    let fenced = execute(&w, REQUEST_2, repo_sync(&w));
    assert_fenced_before_a_push(&w, REQUEST_2, &fenced)?;
    assert_eq!(fenced.operation_id, held.operation_id);
    Ok(())
}

fn a_synchronization_fenced_with_its_push_unaccounted_for_is_not_confirmed()
-> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    advance(&w.repo()?, "refs/heads/main", "lost-output")?;
    w.server.disconnect_at(FixtureBoundary::AfterReceivePack);
    let lost = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(lost.code, ResultCode::TransportUnavailable);
    w.server.clear_fault();
    select_the_key_again(&w)?;
    let opened = connections(&w);

    // The push may have been taken. Nothing says it was, and nothing
    // says it was not: the answer is neither.
    let fenced = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(fenced.code, ResultCode::RecoveryRequired);
    assert_eq!(fenced.outcome, Outcome::Blocked);
    assert_eq!(
        fenced.effects,
        Effects {
            publication: PublicationEffect::Pending,
            ..Effects::not_requested()
        }
    );
    assert_eq!(actions(&fenced), [RecoveryActionKind::RepoInspect]);
    assert_eq!(fenced.operation_id, lost.operation_id);
    assert_eq!(connections(&w), opened);
    assert_final(&w, REQUEST_1, &fenced)
}

fn a_synchronization_fenced_after_its_push_returned_is_published() -> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let local = advance(&w.repo()?, "refs/heads/main", "returned")?;
    let pushes = accepted(&w.server);
    // The push returns and is recorded as returned; the step after it
    // fails.
    let db = w.db()?;
    fixed(db.execute_batch("CREATE TRIGGER fail_verify BEFORE UPDATE ON remote_operation_records WHEN NEW.sync_checkpoint='push_verified' BEGIN SELECT RAISE(ABORT,'fixed failure'); END"))?;
    let stopped = execute(&w, REQUEST_1, repo_sync(&w));
    fixed(db.execute_batch("DROP TRIGGER fail_verify"))?;
    assert!(stopped.code.failure_class().is_some());
    // The row says the remote took the push, and so does the result.
    assert_eq!(stopped.outcome, Outcome::Partial);
    assert_eq!(stopped.effects, published_and_cancelled(local));
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_eq!(
        record(&w, REQUEST_1).ok_or(FixtureError)?.state,
        RequestState::Accepted
    );
    select_the_key_again(&w)?;
    let opened = connections(&w);

    let fenced = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(fenced.code, ResultCode::ExternalChange);
    assert_eq!(fenced.outcome, Outcome::Partial);
    assert_eq!(fenced.effects, published_and_cancelled(local));
    assert!(fenced.recovery.is_empty());
    assert_eq!(fenced.operation_id, stopped.operation_id);
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_final(&w, REQUEST_1, &fenced)
}

// The domain, under the boundary: what the row says of a push.

/// A cancellation asked of the operation itself while its push is in
/// the transport is honored at the boundary after the transport call.
/// By then the row says the push returned: the checkpoint is written
/// the moment it does, before anything can stop the operation.
fn a_row_cancelled_as_its_push_returns_says_the_push_returned() -> Result<(), FixtureError> {
    let w = World::new()?;
    let local = advance(&w.repo()?, "refs/heads/main", "local-advance")?;
    let synchronization = w.primary();
    let operation = synchronization.operation_id;
    let hook = {
        let elsewhere = fixed(RepositoryService::open_at(&w.data()))?;
        let root = w.root.clone();
        during_the_push(move || {
            elsewhere.cancel_remote_operation(&root, operation).unwrap();
        })
    };
    let interrupted = observed(&w, synchronization, REFUSING, &mut Points::default());
    drop(hook);
    assert!(matches!(
        interrupted,
        Err(SynchronizationError::Interrupted)
    ));
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, local);
    assert_eq!(phase(&w, operation)?, "cancelled");
    let checkpoint: String = fixed(w.db()?.query_row(
        "SELECT sync_checkpoint FROM remote_operation_records WHERE operation_ulid=?1",
        [operation.to_string()],
        |row| row.get(0),
    ))?;
    assert_eq!(checkpoint, "push_returned");
    Ok(())
}

// A cancellation of an operation whose push is unaccounted for.

/// The final answer of a request that was stopped, or fenced, with a
/// push intended and nothing known of it: not confirmed, which is
/// neither published nor not published. It offers a look at the
/// repository and not the same request, and replays as it is.
fn assert_not_confirmed(
    w: &World,
    id: &str,
    answer: &Envelope<MutationDataDto>,
) -> Result<(), FixtureError> {
    assert_eq!(answer.code, ResultCode::RecoveryRequired);
    assert_eq!(answer.outcome, Outcome::Blocked);
    assert_eq!(
        answer.effects,
        Effects {
            publication: PublicationEffect::Pending,
            ..Effects::not_requested()
        }
    );
    assert_eq!(actions(answer), [RecoveryActionKind::RepoInspect]);
    assert_final(w, id, answer)
}

/// The network drops during the push, and the user cancels offline.
/// The remote cannot be asked what it holds, so nothing is said of the
/// push either way; but the cancellation is honored, the reservation is
/// let go, and the repository can be worked in again.
fn a_cancellation_with_the_remote_unreachable_frees_the_repository_and_confirms_nothing()
-> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    advance(&w.repo()?, "refs/heads/main", "lost-output")?;
    w.server.disconnect_at(FixtureBoundary::AfterReceivePack);
    let lost = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(lost.code, ResultCode::TransportUnavailable);
    // From here on the server cannot be reached at all.
    w.server.disconnect_at(FixtureBoundary::Handshake);
    assert!(reserved(&w));
    assert_eq!(
        execute(&w, REQUEST_2, create_other(&w)).code,
        ResultCode::RecoveryRequired
    );

    let asked = w
        .service
        .cancel_request(request(REQUEST_1))
        .data
        .ok_or(FixtureError)?;
    assert!(asked.requested);
    let ended = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(ended.operation_id, lost.operation_id);
    assert!(!reserved(&w));
    assert_not_confirmed(&w, REQUEST_1, &ended)?;
    assert!(!record(&w, REQUEST_1).ok_or(FixtureError)?.cancel_requested);
    // The repository is free, offline.
    assert_eq!(
        execute(&w, REQUEST_2, create_other(&w)).outcome,
        Outcome::Success
    );
    Ok(())
}

/// Gives the publication remote a push URL of its own: a second server
/// whose primary branch stands where the first's does, trusted for
/// pushing.
fn separate_push_endpoint(w: &mut World) -> Result<SshRemoteFixture, FixtureError> {
    let destination = SshRemoteFixture::start()?;
    destination.allow_client_public_key(fixed(russh::keys::PublicKey::from_bytes(
        &w.server.allowed_client_public_key(),
    ))?);
    w.probes.push(destination.url().into_bytes());
    ssh_privacy::save(&w.probes)?;
    {
        let bare = fixed(git2::Repository::open_bare(destination.repository_path()))?;
        let source = w.server.repository_path().to_str().ok_or(FixtureError)?;
        fixed(fixed(bare.remote_anonymous(source))?.fetch(
            &["+refs/heads/main:refs/heads/main"],
            None,
            None,
        ))?;
    }
    fixed(
        w.repo()?
            .remote_set_pushurl("origin", Some(&destination.url())),
    )?;
    fixed(w.service.verify_ssh_transport(
        VerifySshTransportRequest {
            root: w.root.clone(),
            direction: SshDirection::Push,
            approval: Some(HostApproval {
                authority: SshAuthority {
                    host: "127.0.0.1".into(),
                    port: destination.address().port(),
                },
                expected: None,
                presented: destination.host_identity(),
            }),
        },
        &mut SessionCredentials::new(Provider),
    ))?;
    Ok(destination)
}

fn pushed_to(server: &SshRemoteFixture) -> Result<git2::Oid, FixtureError> {
    fixed(
        fixed(git2::Repository::open_bare(server.repository_path()))?
            .refname_to_id("refs/heads/main"),
    )
}

/// The remote is fetched from one place and pushed to another. A push
/// is taken by the push endpoint and its result lost; then the push
/// endpoint cannot be reached while the fetch endpoint can. Fetching is
/// not asking the remote that was pushed to: the cancelled request does
/// not say that nothing was published.
fn with_a_separate_push_url_a_fetch_is_not_asking_the_remote() -> Result<(), FixtureError> {
    let mut w = World::new()?;
    approve(&w)?;
    let destination = separate_push_endpoint(&mut w)?;
    let local = advance(&w.repo()?, "refs/heads/main", "lost-output")?;
    destination.disconnect_at(FixtureBoundary::AfterReceivePack);
    let lost = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(lost.code, ResultCode::TransportUnavailable);
    // The push endpoint holds the commit, by this request's push. The
    // fetch endpoint never will.
    assert_eq!(pushed_to(&destination)?, local);
    assert_ne!(pushed_to(&w.server)?, local);
    destination.disconnect_at(FixtureBoundary::Handshake);

    assert!(
        w.service
            .cancel_request(request(REQUEST_1))
            .data
            .ok_or(FixtureError)?
            .requested
    );
    let ended = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(ended.operation_id, lost.operation_id);
    assert!(!reserved(&w));
    assert_not_confirmed(&w, REQUEST_1, &ended)
}

/// The reverse. The fetch endpoint already holds the commit and the
/// push endpoint does not; a first call is cancelled before it pushes.
/// It pushed nothing, and what the fetch side holds is no push of its.
fn a_first_call_cancelled_before_its_push_claims_nothing_from_the_fetch_side()
-> Result<(), FixtureError> {
    let mut w = World::new()?;
    approve(&w)?;
    let destination = separate_push_endpoint(&mut w)?;
    let behind = pushed_to(&destination)?;
    let repo = w.repo()?;
    let local = advance(&repo, "refs/heads/main", "local-advance")?;
    // The commit reaches the fetch endpoint some other way.
    push_peer(&repo, &w.server, "refs/heads/main")?;
    assert_eq!(pushed_to(&w.server)?, local);

    let token = CancellationToken::new();
    let mut sink = CancelAt {
        stage: ProgressStage::Pushing,
        token: token.clone(),
        seen: Vec::new(),
    };
    let cancelled = call(
        &w,
        &w.service,
        REQUEST_1,
        repo_sync(&w),
        Provider,
        &mut sink,
        &token,
    );
    assert_eq!(cancelled.code, ResultCode::Cancelled);
    assert_eq!(cancelled.outcome, Outcome::Cancelled);
    assert_eq!(
        cancelled.effects,
        Effects {
            publication: PublicationEffect::Pending,
            ..Effects::not_requested()
        }
    );
    assert_eq!(pushed_to(&destination)?, behind);
    assert_final(&w, REQUEST_1, &cancelled)
}

/// A cancellation is asked of a request whose push is intended and was
/// not taken. The request's next call dies during its fetch, before it
/// has asked the remote anything. The cancellation is still there for
/// the call after that, which pushes nothing.
fn a_cancellation_asked_before_a_crash_during_the_fetch_still_stops_the_push()
-> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let old = fixed(w.bare()?.refname_to_id("refs/heads/main"))?;
    advance(&w.repo()?, "refs/heads/main", "rejected-local")?;
    w.server.reject_primary_updates(true)?;
    let rejected = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(rejected.code, ResultCode::PushRejected);
    w.server.reject_primary_updates(false)?;
    let pushes = accepted(&w.server);
    assert!(
        w.service
            .cancel_request(request(REQUEST_1))
            .data
            .ok_or(FixtureError)?
            .requested
    );

    // The next call dies inside its first transport call, as a process
    // that is killed does: nothing after that point runs.
    {
        let dying = fixed(RepositoryService::open_at(&w.data()))?;
        let root = w.root.clone();
        let hook = repository::transport::operation_tests::install_hook(|point| {
            if point == repository::transport::operation_tests::Checkpoint::Prepared {
                panic!("the process dies during its fetch");
            }
        });
        let died = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            dying.execute(
                MutationCall {
                    request_id: request(REQUEST_1),
                    confirmation: None,
                    mutation: Mutation::RepoSync(RepoSyncInput { root }),
                },
                &mut SessionCredentials::new(Provider),
                &mut NoProgress,
                &CancellationToken::new(),
            )
        }));
        drop(hook);
        assert!(died.is_err());
    }
    assert_eq!(
        record(&w, REQUEST_1).ok_or(FixtureError)?.state,
        RequestState::Accepted
    );

    let ended = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, old);
    assert_eq!(ended.code, ResultCode::Cancelled);
    assert_eq!(ended.effects.publication, PublicationEffect::Pending);
    assert!(!reserved(&w));
    assert_final(&w, REQUEST_1, &ended)
}

// The request's flag: read without clearing, and never guessed.

/// A request that cannot tell whether it was asked to stop does not go
/// on to push, and is not ended as cancelled either: nobody may have
/// cancelled it. It stops where it is, with its operation as it was,
/// and the same request goes on once the read works.
fn a_request_that_cannot_read_whether_it_was_cancelled_stops_and_does_not_push()
-> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let local = advance(&w.repo()?, "refs/heads/main", "local-advance")?;
    let pushes = accepted(&w.server);
    let opened = connections(&w);
    let failing = fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &w.data(),
        FailurePoint::BeforeRequestCancellationRead,
    ))?;
    let stopped = call(
        &w,
        &failing,
        REQUEST_1,
        repo_sync(&w),
        Provider,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(connections(&w), opened);
    assert_eq!(stopped.code, ResultCode::RecoveryRequired);
    assert_eq!(stopped.outcome, Outcome::Blocked);
    assert_eq!(stopped.effects.commit_oid, None);
    let operation = OperationId::parse(stopped.operation_id.as_deref().ok_or(FixtureError)?)
        .map_err(|_| FixtureError)?;
    // The operation was not cancelled: it holds the reservation as it
    // did, and the request's record is open.
    assert_ne!(phase(&w, operation)?, "cancelled");
    assert!(reserved(&w));
    let stored = record(&w, REQUEST_1).ok_or(FixtureError)?;
    assert_eq!(stored.state, RequestState::Accepted);
    assert!(!stored.cancel_requested);
    assert!(actions(&stopped).contains(&RecoveryActionKind::RequestRetry));

    // The same request goes on.
    let finished = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(finished.effects, published(local));
    assert_eq!(finished.operation_id, stopped.operation_id);
    assert_eq!(accepted(&w.server), pushes + 1);
    Ok(())
}

/// The request's flag is seen, and recording the cancellation on the
/// operation then fails. The flag was not cleared by being seen: the
/// next call of the request sees it again, and stops without pushing.
fn a_cancellation_that_could_not_be_recorded_on_the_operation_stops_the_next_call()
-> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    advance(&w.repo()?, "refs/heads/main", "local-advance")?;
    let old = fixed(w.bare()?.refname_to_id("refs/heads/main"))?;
    let pushes = accepted(&w.server);
    let failing = fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &w.data(),
        FailurePoint::BeforeSynchronizationCancellation,
    ))?;
    // Asked to stop once it is accepted, before it has reserved.
    let elsewhere = fixed(RepositoryService::open_at(&w.data()))?;
    failing.set_request_hook_for_testing(move || {
        assert!(
            elsewhere
                .cancel_request(request(REQUEST_1))
                .data
                .unwrap()
                .requested
        );
    });
    let failed = call(
        &w,
        &failing,
        REQUEST_1,
        repo_sync(&w),
        Provider,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    // The write failed: nothing was acknowledged, and nothing pushed.
    assert!(failed.code.failure_class().is_some());
    assert_ne!(failed.code, ResultCode::Cancelled);
    assert_eq!(accepted(&w.server), pushes);
    let stored = record(&w, REQUEST_1).ok_or(FixtureError)?;
    assert_eq!(stored.state, RequestState::Accepted);
    assert!(stored.cancel_requested);

    let ended = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, old);
    assert_eq!(ended.code, ResultCode::Cancelled);
    assert_eq!(ended.operation_id, failed.operation_id);
    assert!(!reserved(&w));
    let stored = record(&w, REQUEST_1).ok_or(FixtureError)?;
    assert_eq!(stored.state, RequestState::Finished);
    // Cleared once the operation was cancelled.
    assert!(!stored.cancel_requested);
    assert_final(&w, REQUEST_1, &ended)
}

// Resume: a synchronization taken up by a request that did not begin it.

fn resume(w: &World, operation: &str) -> Mutation {
    Mutation::OperationResume(OperationResumeInput {
        root: w.root.clone(),
        operation_id: OperationId::parse(operation).unwrap(),
    })
}

/// A synchronization of the primary branch whose push the server took
/// and whose result was lost: the commit it pushed, and its envelope.
/// It holds the reservation, and its record is accepted.
fn lost_push(w: &World, id: &str) -> Result<(git2::Oid, Envelope<MutationDataDto>), FixtureError> {
    let candidate = advance(&w.repo()?, "refs/heads/main", "lost-output")?;
    w.server.disconnect_at(FixtureBoundary::AfterReceivePack);
    let lost = execute(w, id, repo_sync(w));
    assert!(w.server.receive_status_withheld());
    assert_eq!(lost.code, ResultCode::TransportUnavailable);
    assert_eq!(lost.effects.publication, PublicationEffect::Pending);
    w.server.clear_fault();
    Ok((candidate, lost))
}

/// Asserts that a resume stopped with nothing recorded of it.
fn assert_no_record_of(w: &World, id: &str, envelope: &Envelope<MutationDataDto>) {
    assert_eq!(envelope.command, "operation resume");
    assert_eq!(envelope.request_id.as_deref(), Some(id));
    // The operation is another request's.
    assert_eq!(envelope.operation_id, None);
    assert!(record(w, id).is_none());
}

fn a_resume_reconciles_a_push_whose_result_was_lost_and_pushes_nothing() -> Result<(), FixtureError>
{
    use ProgressStage as S;
    let w = World::new()?;
    approve(&w)?;
    let pushes = accepted(&w.server);
    let (candidate, lost) = lost_push(&w, REQUEST_1)?;
    let operation = lost.operation_id.clone().ok_or(FixtureError)?;
    assert_eq!(accepted(&w.server), pushes + 1);
    assert!(reserved(&w));

    let mut events = Events::default();
    let resumed = call(
        &w,
        &w.service,
        REQUEST_2,
        resume(&w, &operation),
        Provider,
        &mut events,
        &CancellationToken::new(),
    );
    assert_eq!(resumed.command, "operation resume");
    assert_eq!(resumed.outcome, Outcome::Success);
    assert_eq!(resumed.code, ResultCode::Ok);
    // What the operation shows of itself: the remote holds the commit,
    // and its first push is what put it there.
    assert_eq!(resumed.effects, published(candidate));
    assert_eq!(resumed.request_id.as_deref(), Some(REQUEST_2));
    assert_eq!(resumed.operation_id, None);
    assert!(resumed.data.is_none());
    assert!(resumed.recovery.is_empty());
    // The remote was asked, and nothing was pushed a second time.
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_eq!(
        fixed(w.bare()?.refname_to_id("refs/heads/main"))?,
        candidate
    );
    assert!(!reserved(&w));
    assert_eq!(
        phase(&w, OperationId::parse(&operation).unwrap())?,
        "completed"
    );
    assert_eq!(merge_rows(&w)?, 0);
    let stored = record(&w, REQUEST_2).ok_or(FixtureError)?;
    assert_eq!(stored.state, RequestState::Finished);
    assert_eq!(stored.target, operation);
    assert!(stored.operations.is_empty());
    assert_eq!(stored.base_ref.as_deref(), Some("refs/heads/main"));
    // Its events are the resume's own, in the order of the safe points
    // it reached, and none of them is a push.
    let seen = stages(&events.0);
    assert_eq!(seen[..3], [S::Accepted, S::Observing, S::Fetching]);
    assert!(!seen.contains(&S::Pushing), "{seen:?}");
    for event in &events.0 {
        assert_eq!(event.request_id, REQUEST_2);
        assert_eq!(event.command, "operation resume");
        assert_eq!(event.operation_id, None);
        assert_eq!(event.scope, resumed.scope);
        let rendered = serde_json::to_value(event).expect("an event serializes");
        schema::check_published("progress_event.schema.json", &rendered)
            .expect("the progress event schema");
    }

    // The resume is replayed from its record: no connection.
    let opened = connections(&w);
    assert_eq!(execute(&w, REQUEST_2, resume(&w, &operation)), resumed);
    assert_eq!(connections(&w), opened);
    // The same request ID for another operation is another request.
    let mismatch = execute(&w, REQUEST_2, resume(&w, "01ARZ3NDEKTSV4RRFFQ69G5FZZ"));
    assert_eq!(mismatch.code, ResultCode::RequestMismatch);
    assert_eq!(mismatch.effects, Effects::not_requested());

    // The synchronization's own request, sent again, is finished from
    // what its operation came to: it connects to nothing and pushes
    // nothing.
    let finished = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(finished.outcome, Outcome::Success);
    assert_eq!(finished.code, ResultCode::Ok);
    assert_eq!(finished.effects, published(candidate));
    assert_eq!(finished.operation_id, lost.operation_id);
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_eq!(
        record(&w, REQUEST_1).ok_or(FixtureError)?.state,
        RequestState::Finished
    );
    // An operation that has ended owes nothing.
    let noop = execute(&w, REQUEST_3, resume(&w, &operation));
    assert_eq!(noop.outcome, Outcome::Noop);
    assert_eq!(noop.code, ResultCode::AlreadyApplied);
    assert_eq!(noop.effects, Effects::not_requested());
    assert_no_record_of(&w, REQUEST_3, &noop);
    assert_eq!(connections(&w), opened);
    Ok(())
}

fn a_resume_runs_the_hand_off_a_published_synchronization_owes() -> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let commit = advance(&w.repo()?, "refs/heads/main", "index-candidate")?;
    let pushes = accepted(&w.server);
    let db = w.db()?;
    fixed(db.execute_batch("CREATE TRIGGER fail_handoff BEFORE UPDATE ON operation_records WHEN NEW.state='completed' AND NEW.action='refresh' BEGIN SELECT RAISE(ABORT,'fixed failure'); END"))?;
    let partial = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(partial.code, ResultCode::DiscoveryPending);
    let operation = partial.operation_id.clone().ok_or(FixtureError)?;
    // What the result offered is this request.
    assert_eq!(actions(&partial), [RecoveryActionKind::OperationResume]);
    assert_eq!(partial.recovery[0].operation_id, partial.operation_id);
    let opened = connections(&w);
    let owed = Effects {
        publication: PublicationEffect::Published,
        discovery: DiscoveryEffect::Pending,
        commit_oid: Some(commit.to_string()),
        ..Effects::not_requested()
    };

    // While the hand-off still fails, the resume says what the
    // operation's own request says, and offers the resume again.
    let still = execute(&w, REQUEST_2, resume(&w, &operation));
    assert_eq!(still.outcome, Outcome::Partial);
    assert_eq!(still.code, ResultCode::DiscoveryPending);
    assert_eq!(still.effects, owed);
    assert_eq!(actions(&still), [RecoveryActionKind::OperationResume]);
    assert_eq!(still.recovery[0].operation_id.as_deref(), Some(&*operation));
    assert!(still.recovery[0].arguments.is_empty());
    assert_no_record_of(&w, REQUEST_2, &still);

    fixed(db.execute_batch("DROP TRIGGER fail_handoff"))?;
    let resumed = execute(&w, REQUEST_2, resume(&w, &operation));
    assert_eq!(resumed.outcome, Outcome::Success);
    assert_eq!(resumed.code, ResultCode::Ok);
    assert_eq!(resumed.effects, published(commit));
    assert!(resumed.recovery.is_empty());
    assert_eq!(
        record(&w, REQUEST_2).ok_or(FixtureError)?.state,
        RequestState::Finished
    );
    // The hand-off is local: nothing was connected to or pushed.
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes + 1);

    let finished = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(finished.outcome, Outcome::Success);
    assert_eq!(finished.effects, published(commit));
    assert_eq!(finished.operation_id, partial.operation_id);
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_eq!(
        record(&w, REQUEST_1).ok_or(FixtureError)?.state,
        RequestState::Finished
    );
    Ok(())
}

fn a_resume_does_not_take_up_a_fenced_synchronization() -> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    advance(&w.repo()?, "refs/heads/main", "local-advance")?;
    let pushes = accepted(&w.server);
    let held = held_before_a_push(&w, REQUEST_1);
    let operation = held.operation_id.clone().ok_or(FixtureError)?;
    select_the_key_again(&w)?;
    assert!(!reserved(&w));
    let before = phase(&w, OperationId::parse(&operation).unwrap())?;
    let opened = connections(&w);

    let refused = execute(&w, REQUEST_2, resume(&w, &operation));
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert_eq!(refused.code, ResultCode::OriginalRequestRequired);
    assert_eq!(refused.effects, Effects::not_requested());
    // The request that began it is the one that is given its answer.
    assert_eq!(actions(&refused), [RecoveryActionKind::RequestRetry]);
    assert_eq!(refused.recovery[0].arguments["request_id"], REQUEST_1);
    assert_no_record_of(&w, REQUEST_2, &refused);
    assert_eq!(phase(&w, OperationId::parse(&operation).unwrap())?, before);
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes);

    let fenced = execute(&w, REQUEST_1, repo_sync(&w));
    assert_fenced_before_a_push(&w, REQUEST_1, &fenced)?;
    assert_eq!(fenced.operation_id, held.operation_id);

    // The review's I2. Its request is finished now, and sending it
    // again only replays that answer: a resume no longer says a request
    // must be sent. It says what the request was told, and offers
    // nothing, since nothing goes on with this operation.
    let after = execute(&w, REQUEST_2, resume(&w, &operation));
    assert_eq!(after.outcome, fenced.outcome);
    assert_eq!(after.code, ResultCode::ExternalChange);
    assert_eq!(after.effects, Effects::not_requested());
    assert!(after.recovery.is_empty());
    assert_no_record_of(&w, REQUEST_2, &after);
    assert_eq!(phase(&w, OperationId::parse(&operation).unwrap())?, before);
    assert_eq!(connections(&w), opened);
    Ok(())
}

fn a_resume_never_merges_and_leaves_nothing_of_a_merge() -> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let repo = w.repo()?;
    let pushes = accepted(&w.server);
    let (local, lost) = lost_push(&w, REQUEST_1)?;
    let operation = lost.operation_id.clone().ok_or(FixtureError)?;
    // A peer builds on what the server took.
    let server = w.bare()?;
    let built = commit(&server, local, "peer-on-top", b"peer-on-top")?;
    fixed(server.reference("refs/heads/main", built, true, "fixture"))?;
    let branches = local_branches(&repo)?;

    let refused = execute(&w, REQUEST_2, resume(&w, &operation));
    assert_eq!(refused.code, ResultCode::MergeRequired);
    // The remote was asked, and holds the commit the operation pushed.
    assert_eq!(refused.outcome, Outcome::Partial);
    assert_eq!(refused.effects, published_and_cancelled(local));
    // Neither the resume nor the synchronization's own request gets
    // past history that has to be merged: neither is offered.
    assert!(refused.recovery.is_empty());
    assert_no_record_of(&w, REQUEST_2, &refused);
    assert_eq!(merge_rows(&w)?, 0);
    assert_eq!(merge_commits(&repo)?, 0);
    assert_eq!(local_branches(&repo)?, branches);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, built);
    assert_eq!(accepted(&w.server), pushes + 1);
    // The reservation is the operation's, and its request still holds
    // it.
    assert!(reserved(&w));
    assert_eq!(
        record(&w, REQUEST_1).ok_or(FixtureError)?.state,
        RequestState::Accepted
    );
    Ok(())
}

/// F2's bindings cannot leave a merge. A synchronization that was begun
/// some other way and stopped on a conflict is not a resume's to
/// continue.
fn a_resume_does_not_continue_a_merge_made_some_other_way() -> Result<(), FixtureError> {
    let w = World::new()?;
    let repo = w.repo()?;
    let local_base = fixed(repo.refname_to_id("refs/heads/main"))?;
    let local = commit(&repo, local_base, "conflict", b"local conflict")?;
    fixed(repo.checkout_tree(
        &fixed(repo.find_object(local, None))?,
        Some(git2::build::CheckoutBuilder::new().safe()),
    ))?;
    fixed(repo.reference("refs/heads/main", local, true, "fixture"))?;
    let incoming_base = fixed(w.peer.refname_to_id("refs/heads/main"))?;
    let incoming = commit(&w.peer, incoming_base, "conflict", b"incoming conflict")?;
    fixed(
        w.peer
            .reference("refs/heads/main", incoming, true, "fixture"),
    )?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    let merging = w.primary();
    assert!(matches!(
        w.sync(merging.clone()),
        Err(SynchronizationError::ConflictPending { .. })
    ));
    let rows = merge_rows(&w)?;
    assert!(rows > 0);
    let before = phase(&w, merging.operation_id)?;
    // What a restart changes of a row before anything else: it takes
    // the operation from its executor, which raises the owner epoch.
    let touched = || -> Result<(i64, i64), FixtureError> {
        fixed(w.db()?.query_row(
            "SELECT owner_epoch, updated_at FROM remote_operation_records
              WHERE operation_ulid = ?1",
            [merging.operation_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ))
    };
    let untouched = touched()?;
    let opened = connections(&w);
    let pushes = accepted(&w.server);

    let refused = execute(&w, REQUEST_1, resume(&w, &merging.operation_id.to_string()));
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert_eq!(refused.code, ResultCode::RecoveryRequired);
    assert_eq!(refused.effects.commit_oid, None);
    assert_eq!(actions(&refused), [RecoveryActionKind::RepoInspect]);
    assert_no_record_of(&w, REQUEST_1, &refused);
    assert_eq!(merge_rows(&w)?, rows);
    assert_eq!(phase(&w, merging.operation_id)?, before);
    // It was refused before it was restarted: the row was not taken
    // from its executor, and nothing of it was written.
    assert_eq!(touched()?, untouched);
    assert_eq!(fixed(repo.refname_to_id("refs/heads/main"))?, local);
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes);
    Ok(())
}

fn a_resume_stopped_by_its_token_ends_the_synchronization_it_took_up() -> Result<(), FixtureError> {
    use ProgressStage as S;
    let w = World::new()?;
    approve(&w)?;
    advance(&w.repo()?, "refs/heads/main", "local-advance")?;
    let served = fixed(w.bare()?.refname_to_id("refs/heads/main"))?;
    let pushes = accepted(&w.server);
    let held = held_before_a_push(&w, REQUEST_1);
    let operation = held.operation_id.clone().ok_or(FixtureError)?;
    let opened = connections(&w);

    let token = CancellationToken::new();
    let mut sink = CancelAt {
        stage: S::Fetching,
        token: token.clone(),
        seen: Vec::new(),
    };
    let cancelled = call(
        &w,
        &w.service,
        REQUEST_2,
        resume(&w, &operation),
        Provider,
        &mut sink,
        &token,
    );
    assert_eq!(cancelled.command, "operation resume");
    assert_eq!(cancelled.outcome, Outcome::Cancelled);
    assert_eq!(cancelled.code, ResultCode::Cancelled);
    assert_eq!(cancelled.effects.commit_oid, None);
    assert!(cancelled.recovery.is_empty());
    assert_eq!(
        sink.seen,
        [S::Accepted, S::Observing, S::Fetching, S::Stopping]
    );
    // It stopped before it opened a connection, and pushed nothing.
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, served);
    assert!(!reserved(&w));
    assert_eq!(
        phase(&w, OperationId::parse(&operation).unwrap())?,
        "cancelled"
    );
    // That is final for the operation, and so for the resume.
    assert_eq!(
        record(&w, REQUEST_2).ok_or(FixtureError)?.state,
        RequestState::Finished
    );
    assert_eq!(execute(&w, REQUEST_2, resume(&w, &operation)), cancelled);

    // And for the request that began the operation.
    let ended = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(ended.code, ResultCode::Cancelled);
    assert_eq!(ended.operation_id, held.operation_id);
    assert_eq!(
        record(&w, REQUEST_1).ok_or(FixtureError)?.state,
        RequestState::Finished
    );
    assert_eq!(accepted(&w.server), pushes);
    // A cancelled operation is never taken up again. Its row says that
    // Git may hold something of what it began, so the repository is
    // what is looked at.
    let opened = connections(&w);
    let left = execute(&w, REQUEST_3, resume(&w, &operation));
    assert_eq!(left.outcome, Outcome::Blocked);
    assert_eq!(left.code, ResultCode::RecoveryRequired);
    assert_eq!(actions(&left), [RecoveryActionKind::RepoInspect]);
    assert_no_record_of(&w, REQUEST_3, &left);
    assert_eq!(
        phase(&w, OperationId::parse(&operation).unwrap())?,
        "cancelled"
    );
    assert_eq!(connections(&w), opened);
    Ok(())
}

fn a_resume_that_cannot_unlock_the_key_leaves_the_synchronization_to_its_request()
-> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let local = advance(&w.repo()?, "refs/heads/main", "local-advance")?;
    let pushes = accepted(&w.server);
    let held = held_before_a_push(&w, REQUEST_1);
    let operation = held.operation_id.clone().ok_or(FixtureError)?;

    // No passphrase this time either.
    let stopped = call(
        &w,
        &w.service,
        REQUEST_2,
        resume(&w, &operation),
        NoPassphrase,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(stopped.code, ResultCode::UnlockRequired);
    assert_eq!(stopped.outcome, held.outcome);
    assert_eq!(stopped.effects, held.effects);
    // Nothing is held for the resume: the reservation is the
    // operation's, and its own request is what holds that.
    assert!(stopped.recovery.is_empty());
    assert_no_record_of(&w, REQUEST_2, &stopped);
    assert!(reserved(&w));
    assert_eq!(
        record(&w, REQUEST_1).ok_or(FixtureError)?.state,
        RequestState::Accepted
    );
    assert_eq!(accepted(&w.server), pushes);

    // With the passphrase, a resume under the same request ID goes on.
    let resumed = execute(&w, REQUEST_2, resume(&w, &operation));
    assert_eq!(resumed.outcome, Outcome::Success);
    assert_eq!(resumed.effects, published(local));
    assert_eq!(accepted(&w.server), pushes + 1);
    assert!(!reserved(&w));
    let finished = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(finished.effects, published(local));
    assert_eq!(finished.operation_id, held.operation_id);
    assert_eq!(accepted(&w.server), pushes + 1);
    Ok(())
}

fn a_resume_of_an_item_s_synchronization_is_replayed_while_its_branch_holds_the_commit()
-> Result<(), FixtureError> {
    let mut w = World::new()?;
    approve(&w)?;
    w.context()?;
    let tip = fixed(w.repo()?.refname_to_id(CONTEXT))?;
    let pushes = accepted(&w.server);
    w.server.disconnect_at(FixtureBoundary::AfterReceivePack);
    let lost = execute(&w, REQUEST_1, item_sync(&w));
    assert_eq!(lost.code, ResultCode::TransportUnavailable);
    w.server.clear_fault();
    let operation = lost.operation_id.clone().ok_or(FixtureError)?;

    let resumed = execute(&w, REQUEST_2, resume(&w, &operation));
    assert_eq!(resumed.outcome, Outcome::Success);
    assert_eq!(resumed.effects, published(tip));
    assert_eq!(fixed(w.bare()?.refname_to_id(CONTEXT))?, tip);
    assert_eq!(accepted(&w.server), pushes + 1);
    // The resume is placed on the branch the operation synchronized:
    // the commit it names is the item's, and no commit of primary.
    assert_eq!(
        record(&w, REQUEST_2)
            .ok_or(FixtureError)?
            .base_ref
            .as_deref(),
        Some(CONTEXT)
    );
    assert!(!fixed(w.repo()?.graph_descendant_of(
        fixed(w.repo()?.refname_to_id("refs/heads/main"))?,
        tip
    ))?);
    // What it names of an item is the operation's ID alone.
    assert_eq!(resumed.scope.item_id, None);
    assert_eq!(resumed.scope.branch, None);

    let opened = connections(&w);
    assert_eq!(execute(&w, REQUEST_2, resume(&w, &operation)), resumed);
    assert_eq!(connections(&w), opened);
    let finished = execute(&w, REQUEST_1, item_sync(&w));
    assert_eq!(finished.effects, published(tip));
    assert_eq!(finished.operation_id, lost.operation_id);
    assert_eq!(accepted(&w.server), pushes + 1);
    Ok(())
}

fn a_resume_stopped_for_an_untrusted_host_names_the_host_and_offers_its_approval()
-> Result<(), FixtureError> {
    // No approval: the service has never trusted the server.
    let w = World::new()?;
    let held = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(held.code, ResultCode::HostApprovalRequired);
    assert!(reserved(&w));
    let operation = held.operation_id.clone().ok_or(FixtureError)?;

    let stopped = execute(&w, REQUEST_2, resume(&w, &operation));
    assert_eq!(stopped.code, ResultCode::HostApprovalRequired);
    assert_eq!(stopped.outcome, held.outcome);
    // The host it reached, as the synchronization's own request names
    // it, and the approval that deals with it.
    let data = synchronization_data(&stopped)?;
    assert_eq!(Some(data), synchronization_data(&held).ok());
    assert_eq!(data.port, w.server.address().port());
    assert_eq!(actions(&stopped), [RecoveryActionKind::HostApprove]);
    let offered = &stopped.recovery[0];
    assert_eq!(
        offered
            .arguments
            .get("authority")
            .and_then(|value| value.as_str()),
        Some(format!("127.0.0.1:{}", w.server.address().port()).as_str())
    );
    assert_eq!(
        offered
            .arguments
            .get("root")
            .and_then(|value| value.as_str()),
        stopped.scope.repository.as_deref()
    );
    assert_no_record_of(&w, REQUEST_2, &stopped);
    assert!(reserved(&w));

    // With the host approved the resume goes on: the remote holds what
    // the branch holds, and nothing is pushed.
    approve(&w)?;
    let pushes = accepted(&w.server);
    let resumed = execute(&w, REQUEST_2, resume(&w, &operation));
    assert_current(&resumed);
    assert_eq!(resumed.command, "operation resume");
    assert_eq!(accepted(&w.server), pushes);
    assert!(!reserved(&w));
    assert_eq!(
        record(&w, REQUEST_2).ok_or(FixtureError)?.state,
        RequestState::Finished
    );
    let finished = execute(&w, REQUEST_1, repo_sync(&w));
    assert_current(&finished);
    assert_eq!(finished.operation_id, held.operation_id);
    Ok(())
}

/// A final answer never understates a push: a resume that pushed and
/// lost its answer is not answered as a no-op by the row it completed.
fn a_resume_whose_answer_was_lost_says_what_the_synchronization_it_took_up_published()
-> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let local = advance(&w.repo()?, "refs/heads/main", "local-advance")?;
    let pushes = accepted(&w.server);
    let held = held_before_a_push(&w, REQUEST_1);
    let operation = held.operation_id.clone().ok_or(FixtureError)?;
    let failing = fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &w.data(),
        FailurePoint::BeforeRequestSettlement,
    ))?;

    // The call pushed and died before it recorded its end.
    let lost = call(
        &w,
        &failing,
        REQUEST_2,
        resume(&w, &operation),
        Provider,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(lost.code, ResultCode::InternalError);
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, local);
    assert_eq!(
        phase(&w, OperationId::parse(&operation).unwrap())?,
        "completed"
    );
    assert_eq!(
        record(&w, REQUEST_2).ok_or(FixtureError)?.state,
        RequestState::Accepted
    );

    // The same request finds the operation ended, and says what the
    // operation itself shows that it came to. Nothing is connected to.
    let opened = connections(&w);
    let found = execute(&w, REQUEST_2, resume(&w, &operation));
    assert_eq!(found.outcome, Outcome::Success);
    assert_eq!(found.code, ResultCode::Ok);
    assert_eq!(found.effects, published(local));
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_eq!(
        record(&w, REQUEST_2).ok_or(FixtureError)?.state,
        RequestState::Finished
    );
    assert_eq!(execute(&w, REQUEST_2, resume(&w, &operation)), found);
    Ok(())
}

/// A resume that takes up a synchronization whose push is intended and
/// not accounted for, and is cancelled while the remote cannot be
/// asked: the operation is ended and the repository freed, and the
/// answer says of the push only that it is not confirmed. That answer
/// is final, and is given again exactly as it was.
fn a_resume_cancelled_with_the_remote_unreachable_confirms_nothing_and_is_replayed_as_it_is()
-> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let (_, lost) = lost_push(&w, REQUEST_1)?;
    let operation = lost.operation_id.clone().ok_or(FixtureError)?;
    // From here on the server cannot be reached at all.
    w.server.disconnect_at(FixtureBoundary::Handshake);
    assert!(reserved(&w));

    let token = CancellationToken::new();
    let mut sink = CancelAt {
        stage: ProgressStage::Observing,
        token: token.clone(),
        seen: Vec::new(),
    };
    let ended = call(
        &w,
        &w.service,
        REQUEST_2,
        resume(&w, &operation),
        Provider,
        &mut sink,
        &token,
    );
    assert_eq!(ended.command, "operation resume");
    assert_eq!(ended.code, ResultCode::RecoveryRequired);
    assert_eq!(ended.outcome, Outcome::Blocked);
    // Neither published nor not published.
    assert_eq!(
        ended.effects,
        Effects {
            publication: PublicationEffect::Pending,
            ..Effects::not_requested()
        }
    );
    assert_eq!(actions(&ended), [RecoveryActionKind::RepoInspect]);
    assert!(!reserved(&w));
    assert_eq!(
        phase(&w, OperationId::parse(&operation).unwrap())?,
        "cancelled"
    );
    assert_eq!(
        record(&w, REQUEST_2).ok_or(FixtureError)?.state,
        RequestState::Finished
    );

    // Given again, it is the same answer, with what it offers.
    let opened = connections(&w);
    assert_eq!(execute(&w, REQUEST_2, resume(&w, &operation)), ended);
    assert_eq!(connections(&w), opened);
    // The synchronization's own request is told the same of its push.
    let own = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(own.operation_id, lost.operation_id);
    assert_not_confirmed(&w, REQUEST_1, &own)?;
    Ok(())
}

/// A resume whose own call pushed, was cancelled as the push returned,
/// and died before it recorded its end. The row it left is a cancelled
/// one, which nothing takes up; the same resume, sent again, is still
/// told what the operation's row shows of the push.
fn a_resume_cancelled_during_its_push_whose_answer_was_lost_still_reports_the_push()
-> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let local = advance(&w.repo()?, "refs/heads/main", "local-advance")?;
    let pushes = accepted(&w.server);
    let held = held_before_a_push(&w, REQUEST_1);
    let operation = held.operation_id.clone().ok_or(FixtureError)?;
    let failing = fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &w.data(),
        FailurePoint::BeforeRequestSettlement,
    ))?;
    let token = CancellationToken::new();
    let hook = {
        let token = token.clone();
        during_the_push(move || token.cancel())
    };
    let lost = call(
        &w,
        &failing,
        REQUEST_2,
        resume(&w, &operation),
        Provider,
        &mut NoProgress,
        &token,
    );
    drop(hook);
    assert_eq!(lost.code, ResultCode::InternalError);
    // The server took the push, and the operation was cancelled.
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, local);
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_eq!(
        phase(&w, OperationId::parse(&operation).unwrap())?,
        "cancelled"
    );
    assert_eq!(
        record(&w, REQUEST_2).ok_or(FixtureError)?.state,
        RequestState::Accepted
    );

    let opened = connections(&w);
    let found = execute(&w, REQUEST_2, resume(&w, &operation));
    assert_eq!(found.code, ResultCode::Cancelled);
    assert_eq!(found.outcome, Outcome::Cancelled);
    assert_eq!(found.effects, published_and_cancelled(local));
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_eq!(
        record(&w, REQUEST_2).ok_or(FixtureError)?.state,
        RequestState::Finished
    );
    assert_eq!(execute(&w, REQUEST_2, resume(&w, &operation)), found);
    // A resume that did nothing under the operation claims nothing of
    // it: the operation has ended, and its own request is where its
    // push is reported.
    let other = execute(&w, REQUEST_3, resume(&w, &operation));
    assert_eq!(other.code, ResultCode::AlreadyApplied);
    assert_eq!(other.effects, Effects::not_requested());
    assert_no_record_of(&w, REQUEST_3, &other);
    let own = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(own.code, ResultCode::Cancelled);
    assert_eq!(own.effects, published_and_cancelled(local));
    Ok(())
}

/// A resume that stopped the synchronization it took up and died before
/// it recorded its end. The cancelled row is one a resume does not take
/// up; the resume that cancelled it is all the same told so when it is
/// sent again, as the operation's own request is.
fn a_resume_stopped_by_its_token_whose_answer_was_lost_is_told_it_was_cancelled()
-> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    advance(&w.repo()?, "refs/heads/main", "local-advance")?;
    let pushes = accepted(&w.server);
    let held = held_before_a_push(&w, REQUEST_1);
    let operation = held.operation_id.clone().ok_or(FixtureError)?;
    let failing = fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &w.data(),
        FailurePoint::BeforeRequestSettlement,
    ))?;
    let token = CancellationToken::new();
    let mut sink = CancelAt {
        stage: ProgressStage::Fetching,
        token: token.clone(),
        seen: Vec::new(),
    };
    let lost = call(
        &w,
        &failing,
        REQUEST_2,
        resume(&w, &operation),
        Provider,
        &mut sink,
        &token,
    );
    assert_eq!(lost.code, ResultCode::InternalError);
    assert_eq!(
        phase(&w, OperationId::parse(&operation).unwrap())?,
        "cancelled"
    );
    assert!(!reserved(&w));
    assert_eq!(
        record(&w, REQUEST_2).ok_or(FixtureError)?.state,
        RequestState::Accepted
    );

    let opened = connections(&w);
    let found = execute(&w, REQUEST_2, resume(&w, &operation));
    assert_eq!(found.code, ResultCode::Cancelled);
    assert_eq!(found.outcome, Outcome::Cancelled);
    assert_eq!(found.effects.commit_oid, None);
    assert!(found.recovery.is_empty());
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(
        record(&w, REQUEST_2).ok_or(FixtureError)?.state,
        RequestState::Finished
    );
    assert_eq!(execute(&w, REQUEST_2, resume(&w, &operation)), found);
    Ok(())
}

// A resume and the request that owns the operation it takes up.

/// A synchronization of the primary branch whose push the server
/// rejected: it holds the reservation with a push intended and not
/// accepted, and its record is accepted. The server takes pushes again.
fn rejected_push(w: &World, id: &str) -> Result<Envelope<MutationDataDto>, FixtureError> {
    advance(&w.repo()?, "refs/heads/main", "rejected-local")?;
    w.server.reject_primary_updates(true)?;
    let rejected = execute(w, id, repo_sync(w));
    assert_eq!(rejected.code, ResultCode::PushRejected);
    w.server.reject_primary_updates(false)?;
    assert!(reserved(w));
    Ok(rejected)
}

/// Asserts that a resume ended the operation cancelled before any push,
/// and that the request that owns the operation is then told so, with
/// its flag cleared, and replays that answer.
fn assert_cancelled_before_a_push(
    w: &World,
    old: git2::Oid,
    pushes: usize,
    owner: &Envelope<MutationDataDto>,
    resumed: &Envelope<MutationDataDto>,
) -> Result<(), FixtureError> {
    let operation = owner.operation_id.clone().ok_or(FixtureError)?;
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, old);
    assert_eq!(resumed.command, "operation resume");
    assert_eq!(resumed.code, ResultCode::Cancelled);
    assert_eq!(resumed.outcome, Outcome::Cancelled);
    assert_eq!(resumed.effects.publication, PublicationEffect::Pending);
    assert_eq!(resumed.effects.commit_oid, None);
    assert!(!reserved(w));
    assert_eq!(
        phase(w, OperationId::parse(&operation).unwrap())?,
        "cancelled"
    );
    assert_eq!(
        record(w, REQUEST_2).ok_or(FixtureError)?.state,
        RequestState::Finished
    );
    // The call that ended the operation is the one that asked the push
    // endpoint what it holds, so its answer is the owner's too: the
    // owner's record is finished with it. Its flag has done its work.
    let stored = record(w, REQUEST_1).ok_or(FixtureError)?;
    assert_eq!(stored.state, RequestState::Finished);
    assert!(!stored.cancel_requested);
    let opened = connections(w);

    let ended = execute(w, REQUEST_1, repo_sync(w));
    assert_eq!(ended.code, ResultCode::Cancelled);
    assert_eq!(ended.effects.publication, PublicationEffect::Pending);
    assert_eq!(ended.outcome, Outcome::Cancelled);
    assert_eq!(ended.effects, resumed.effects);
    assert_eq!(ended.command, "repo sync");
    assert_eq!(ended.operation_id, owner.operation_id);
    assert_eq!(connections(w), opened);
    assert_eq!(accepted(&w.server), pushes);
    assert_final(w, REQUEST_1, &ended)
}

/// The review's C1. The cancellation of a request lives on its record
/// once its operation intends a push. A resume of that operation reads
/// it there, and does not push.
fn a_cancellation_asked_of_the_request_that_owns_it_stops_a_resume_before_it_pushes()
-> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let old = fixed(w.bare()?.refname_to_id("refs/heads/main"))?;
    let rejected = rejected_push(&w, REQUEST_1)?;
    let operation = rejected.operation_id.clone().ok_or(FixtureError)?;
    let pushes = accepted(&w.server);
    assert!(
        w.service
            .cancel_request(request(REQUEST_1))
            .data
            .ok_or(FixtureError)?
            .requested
    );
    assert!(record(&w, REQUEST_1).ok_or(FixtureError)?.cancel_requested);

    let resumed = execute(&w, REQUEST_2, resume(&w, &operation));
    assert_cancelled_before_a_push(&w, old, pushes, &rejected, &resumed)
}

/// The same, asked once the resume is accepted and before it reaches a
/// safe point: the flag is read at each one, not once.
fn a_cancellation_asked_of_the_owning_request_while_a_resume_runs_stops_it_before_it_pushes()
-> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let old = fixed(w.bare()?.refname_to_id("refs/heads/main"))?;
    let rejected = rejected_push(&w, REQUEST_1)?;
    let operation = rejected.operation_id.clone().ok_or(FixtureError)?;
    let pushes = accepted(&w.server);
    let elsewhere = fixed(RepositoryService::open_at(&w.data()))?;
    w.service.set_request_hook_for_testing(move || {
        assert!(
            elsewhere
                .cancel_request(request(REQUEST_1))
                .data
                .unwrap()
                .requested
        );
    });

    let resumed = execute(&w, REQUEST_2, resume(&w, &operation));
    assert_cancelled_before_a_push(&w, old, pushes, &rejected, &resumed)
}

/// Whether the owning request was asked to stop cannot be read: before
/// the push that is not taken for "no".
fn a_resume_that_cannot_read_whether_the_owning_request_was_cancelled_stops_and_does_not_push()
-> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let old = fixed(w.bare()?.refname_to_id("refs/heads/main"))?;
    let rejected = rejected_push(&w, REQUEST_1)?;
    let operation = rejected.operation_id.clone().ok_or(FixtureError)?;
    let pushes = accepted(&w.server);
    let failing = fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &w.data(),
        FailurePoint::BeforeRequestCancellationRead,
    ))?;
    let stopped = call(
        &w,
        &failing,
        REQUEST_2,
        resume(&w, &operation),
        Provider,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(stopped.code, ResultCode::RecoveryRequired);
    assert_eq!(stopped.effects.commit_oid, None);
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, old);
    assert_ne!(
        phase(&w, OperationId::parse(&operation).unwrap())?,
        "cancelled"
    );
    assert!(reserved(&w));
    assert_no_record_of(&w, REQUEST_2, &stopped);
    Ok(())
}

/// A resume from another thread's service, as `execute_elsewhere`.
fn the_operation_of(w: &World, id: &str) -> Result<String, FixtureError> {
    Ok(record(w, id)
        .ok_or(FixtureError)?
        .operations
        .first()
        .ok_or(FixtureError)?
        .operation_id
        .to_string())
}

/// The review's M3. A call of the owning request that is still inside
/// its transport when a resume takes the operation over comes back
/// fenced. It settles nothing: the record stays the request's own, and
/// the request, sent again, is answered from what the operation came
/// to, with its commit, and not as a new request.
fn a_call_of_the_owning_request_fenced_by_a_resume_that_published_settles_nothing()
-> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    let local = advance(&w.repo()?, "refs/heads/main", "local-advance")?;
    let pushes = accepted(&w.server);
    let hold = w.server.hold_advertisement()?;
    let first = execute_elsewhere(&w, REQUEST_1)?;
    hold.wait_until_held()?;
    let operation = the_operation_of(&w, REQUEST_1)?;
    assert_eq!(record(&w, REQUEST_1).ok_or(FixtureError)?.attempt, 1);

    let resumed = execute(&w, REQUEST_2, resume(&w, &operation));
    assert_eq!(resumed.outcome, Outcome::Success);
    assert_eq!(resumed.effects, published(local));
    assert_eq!(accepted(&w.server), pushes + 1);
    // The takeover is recorded on the owner's record: the call that was
    // running under it is no longer the one that may settle it. The
    // resume's own outcome, which is final for the operation, has.
    let taken = record(&w, REQUEST_1).ok_or(FixtureError)?;
    assert_eq!((taken.state, taken.attempt), (RequestState::Finished, 2));

    hold.release()?;
    let fenced = joined(&w, first)?;
    assert_eq!(fenced.code, ResultCode::RecoveryRequired);
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_eq!(record(&w, REQUEST_1), Some(taken));

    let opened = connections(&w);
    let own = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(own.outcome, Outcome::Success);
    assert_eq!(own.code, ResultCode::Ok);
    assert_eq!(own.effects, published(local));
    assert_eq!(own.operation_id.as_deref(), Some(&*operation));
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_final(&w, REQUEST_1, &own)
}

fn a_call_of_the_owning_request_fenced_by_a_resume_that_was_cancelled_settles_nothing()
-> Result<(), FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    advance(&w.repo()?, "refs/heads/main", "local-advance")?;
    let old = fixed(w.bare()?.refname_to_id("refs/heads/main"))?;
    let pushes = accepted(&w.server);
    let hold = w.server.hold_advertisement()?;
    let first = execute_elsewhere(&w, REQUEST_1)?;
    hold.wait_until_held()?;
    let operation = the_operation_of(&w, REQUEST_1)?;

    let token = CancellationToken::new();
    let mut sink = CancelAt {
        stage: ProgressStage::Fetching,
        token: token.clone(),
        seen: Vec::new(),
    };
    let cancelled = call(
        &w,
        &w.service,
        REQUEST_2,
        resume(&w, &operation),
        Provider,
        &mut sink,
        &token,
    );
    assert_eq!(cancelled.code, ResultCode::Cancelled);
    let taken = record(&w, REQUEST_1).ok_or(FixtureError)?;
    assert_eq!((taken.state, taken.attempt), (RequestState::Finished, 2));

    hold.release()?;
    let fenced = joined(&w, first)?;
    assert!(fenced.code.failure_class().is_some());
    assert_eq!(record(&w, REQUEST_1), Some(taken));

    let own = execute(&w, REQUEST_1, repo_sync(&w));
    assert_eq!(own.code, ResultCode::Cancelled);
    assert_eq!(own.operation_id.as_deref(), Some(&*operation));
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, old);
    assert_final(&w, REQUEST_1, &own)
}

// The conformance table, for the two commands that need a server; see
// `conformance`.

/// One call of `execute` that presents `confirmation`, scanned as
/// `call` scans.
fn presenting(
    w: &World,
    id: &str,
    confirmation: Option<&str>,
    mutation: Mutation,
) -> Envelope<MutationDataDto> {
    let envelope = w.service.execute(
        MutationCall {
            request_id: request(id),
            confirmation: confirmation.map(|id| ConfirmationId::parse(id).unwrap()),
            mutation,
        },
        &mut SessionCredentials::new(Provider),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    let rendered = serde_json::to_vec(&envelope).expect("an envelope serializes");
    ssh_privacy::clean(&rendered, &w.probes).expect("redacted envelope");
    w.privacy().expect("redacted durable state");
    envelope
}

/// A world whose host is trusted and whose primary branch and item
/// branch each hold a commit the server does not, with `rule` driven
/// through the requests `request` gives of it.
fn conforming(
    request: fn(&World, conformance::Ask) -> Mutation,
    rule: &mut dyn FnMut(&conformance::Subject<'_>),
) -> Result<(), String> {
    use conformance::{Ask, NO_SUCH_CONFIRMATION, Recorded, Subject};
    let built = || -> Result<World, FixtureError> {
        let mut w = World::new()?;
        approve(&w)?;
        w.context()?;
        advance(&w.repo()?, "refs/heads/main", "local-advance")?;
        Ok(w)
    };
    let w = built().map_err(|_| "a world with a server".to_owned())?;
    rule(&Subject {
        call: &|id, ask| {
            let confirmation = match ask {
                Ask::Same | Ask::Changed | Ask::Refused => None,
                Ask::Unconsented => Some(NO_SUCH_CONFIRMATION),
            };
            let envelope = presenting(&w, id, confirmation, request(&w, ask));
            serde_json::to_vec(&envelope).expect("an envelope serializes")
        },
        record: &|id| {
            record(&w, id).map(|record| Recorded {
                finished: record.state == RequestState::Finished,
                image: format!("{record:?}"),
            })
        },
        request_rows: &|| {
            w.db()
                .expect("the database")
                .query_row("SELECT COUNT(*) FROM request_records", [], |row| row.get(0))
                .expect("the request records")
        },
    });
    Ok(())
}

const CONFORMANCE: [conformance::Row; 2] = [
    conformance::Row::new("item sync", false, "item_not_found", |rule| {
        conforming(
            |w, ask| match ask {
                conformance::Ask::Same | conformance::Ask::Unconsented => item_sync(w),
                // Another item, which no fixture holds: a changed input
                // under a finished request's ID, and a request that is
                // refused under a new one.
                conformance::Ask::Changed | conformance::Ask::Refused => {
                    Mutation::ItemSync(ItemSyncInput {
                        root: w.root.clone(),
                        kind: AuthoringKind::Ticket,
                        id: OTHER_ITEM.parse().unwrap(),
                    })
                }
            },
            rule,
        )
    }),
    conformance::Row::new("repo sync", false, "not_repository", |rule| {
        conforming(
            |w, ask| match ask {
                conformance::Ask::Same | conformance::Ask::Unconsented => repo_sync(w),
                // The command has no input but its repository: another
                // command.
                conformance::Ask::Changed => item_sync(w),
                // A directory that is no repository.
                conformance::Ask::Refused => Mutation::RepoSync(RepoSyncInput { root: w.data() }),
            },
            rule,
        )
    }),
];

fn every_command_of_this_target_keeps_the_rules_of_the_boundary() -> Result<(), FixtureError> {
    // The rows of the two commands that approve a host are driven by
    // a case of their own, in `host`: each rule of each row builds a
    // world with a server, and a case has a time it must end within.
    conformance::run_part(
        conformance::Home::Remote,
        &request_store::bound_commands(),
        &CONFORMANCE,
        &host::CONFORMANCE.map(|row| row.command),
    );
    Ok(())
}
