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
    assert!(actions(&first).contains(&RecoveryActionKind::RequestRetry));
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
    assert!(actions(&again).contains(&RecoveryActionKind::RequestRetry));
    assert!(reserved(w));
    assert_eq!(
        record(w, REQUEST_1).ok_or(FixtureError)?.state,
        RequestState::Accepted
    );

    // A cancellation, then the same request.
    let cancel = w.service.cancel_request(request(REQUEST_1));
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
        },
    )?;
    assert_eq!(actions(&first), [RecoveryActionKind::RequestRetry]);
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

    let first = pin_a_held_reservation(
        &w,
        &Stopped {
            mutation: item_sync,
            code: ResultCode::RemoteBranchDeleted,
            locked: false,
        },
    )?;
    assert_eq!(actions(&first), [RecoveryActionKind::RequestRetry]);
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
        },
    )?;
    assert_eq!(actions(&first), [RecoveryActionKind::RequestRetry]);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, old);
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
    let first = pin_a_held_reservation(
        &w,
        &Stopped {
            mutation: repo_sync,
            code: ResultCode::UnlockRequired,
            locked: true,
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
