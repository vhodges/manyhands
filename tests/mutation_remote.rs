//! The synchronization commands of the mutation boundary, their progress
//! and their cancellation, against a real authenticated server. Isolated
//! before threads, as every target that opens a transport is.
#![allow(clippy::result_large_err)]
#[allow(dead_code, unused_imports)]
#[path = "../src/lib.rs"]
mod production;
pub use production::*;
#[path = "support/remote_world.rs"]
mod remote_world;
#[path = "support/ssh_harness.rs"]
mod ssh_harness;
#[path = "support/ssh_privacy.rs"]
mod ssh_privacy;
#[path = "support/ssh_remote.rs"]
mod ssh_remote;

use remote_world::*;
use repository::{keys::*, *};
use ssh_remote::*;

fn main() {
    // SAFETY: real main, before any fixture, runner or runtime thread exists.
    if unsafe { ssh_harness::initialize() }.is_err() {
        eprintln!("SSH synchronization initialization failed");
        std::process::exit(1);
    }
    ssh_harness::run_with_output_privacy(CASES, OUTPUT_CONTROLS);
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
];

/// Cases run only as children of a case above, each of which expects
/// the harness to fail them.
const OUTPUT_CONTROLS: &[ssh_harness::Case] = &[];

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
