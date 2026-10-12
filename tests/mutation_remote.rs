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
        "the_same_request_id_for_another_target_is_a_mismatch",
        the_same_request_id_for_another_target_is_a_mismatch,
    ),
    (
        "a_repository_with_no_publication_remote_is_blocked",
        a_repository_with_no_publication_remote_is_blocked,
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

// The commands, through the boundary.

const REQUEST_1: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX1";
const REQUEST_2: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX2";

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
    ssh_privacy::clean(format!("{envelope:?}").as_bytes(), &w.probes).expect("redacted envelope");
    w.privacy().expect("redacted durable state");
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
