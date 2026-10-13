//! Progress and cancellation, for the commands that need no server: what
//! a local command reports, what becomes of a sink that panics, and what
//! `request cancel` does for a request that has no safe point. A
//! synchronization's own are in the test target `mutation_remote`.

use std::fs;

use manyhands::{
    repository::{
        CancelDto, CancellationToken, FailurePoint, IndexRefreshInput, Mutation, MutationCall,
        MutationDataDto, OperationFamily, OperationId, ProgressEvent, ProgressSink, ProgressStage,
        RequestId, RequestState,
        keys::SessionCredentials,
        request_store::{DigestSalt, IntentDigest, NewRequest, RequestOperation, ScopeKey},
    },
    results::{Effects, Envelope, Outcome, ResultCode},
};

use crate::support::{
    FailOnce,
    golden::{self, ContractCase},
    items::TICKET_A,
    mutation::{
        self, Events, NoPassphrase, REQUEST_1, REQUEST_2, REQUEST_3, SENTINELS, TICKET_NEW, TITLE,
        World,
    },
    schema,
};

const PROGRESS_SCHEMA: &str = "progress_event.schema.json";
const CANCEL_SCHEMA: &str = "cancel.schema.json";

fn request(id: &str) -> RequestId {
    RequestId::parse(id).unwrap()
}

fn execute(
    world: &World,
    id: &str,
    mutation: Mutation,
    progress: &mut dyn ProgressSink,
) -> Envelope<MutationDataDto> {
    world.service.execute(
        MutationCall {
            request_id: request(id),
            confirmation: None,
            mutation,
        },
        &mut SessionCredentials::new(NoPassphrase),
        progress,
        &CancellationToken::new(),
    )
}

fn stages(events: &[ProgressEvent]) -> Vec<ProgressStage> {
    events.iter().map(|event| event.stage).collect()
}

/// Every event names the request its envelope names, and nothing the
/// request carried.
fn assert_events_are_of(events: &[ProgressEvent], envelope: &Envelope<MutationDataDto>) {
    for event in events {
        assert_eq!(Some(&event.request_id), envelope.request_id.as_ref());
        assert_eq!(event.operation_id, envelope.operation_id);
        assert_eq!(event.command, envelope.command);
        assert_eq!(event.scope, envelope.scope);
        let value = serde_json::to_value(event).unwrap();
        if let Some((sentinel, location)) = golden::find_sentinel(&value, &SENTINELS) {
            panic!("an event contains {sentinel:?} at {location}");
        }
        if let Err(problem) = schema::check_published(PROGRESS_SCHEMA, &value) {
            panic!("an event does not match {PROGRESS_SCHEMA}: {problem}");
        }
    }
}

/// Compares the events of one call with their golden fixture, as
/// `mutation::assert_contract` compares an envelope: the fixture is the
/// array of events under `tests/fixtures/progress_v1`, and its name is one
/// of `mutation::PROGRESS_CASES`.
fn assert_progress_golden(name: &str, world: &World, events: &[ProgressEvent]) {
    let value = serde_json::to_value(events).unwrap();
    let root = world.root.to_str().unwrap();
    let operation = events
        .iter()
        .find_map(|event| event.operation_id.clone())
        .unwrap_or_default();
    let mut placeholders = vec![(root, "<repository>")];
    if !operation.is_empty() {
        placeholders.push((operation.as_str(), "<operation>"));
    }
    let actual = golden::render(&value, &placeholders);
    let path = mutation::progress_fixture(name);
    if golden::update_requested(
        std::env::var_os(golden::UPDATE_VARIABLE).as_deref(),
        std::env::var_os("CI").as_deref(),
    ) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &actual).unwrap();
        return;
    }
    let expected = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "{name}: cannot read {}: {error}; set {}=1 to write it",
            path.display(),
            golden::UPDATE_VARIABLE
        )
    });
    assert_eq!(expected, actual, "{name}");
}

#[test]
fn a_local_command_reports_that_it_was_accepted_and_what_it_is_about_to_do() {
    let world = World::new();
    let mut events = Events::default();
    let created = execute(&world, REQUEST_1, world.create(TICKET_NEW), &mut events);
    assert_eq!(created.outcome, Outcome::Success);
    // Both were sent before the result was returned, and nothing since.
    assert_eq!(
        stages(&events.0),
        [ProgressStage::Accepted, ProgressStage::Writing]
    );
    assert_events_are_of(&events.0, &created);
    for event in &events.0 {
        assert_eq!(event.effects, Effects::not_requested());
    }
    assert_progress_golden("ticket_create", &world, &events.0);

    let mut events = Events::default();
    let refreshed = execute(
        &world,
        REQUEST_2,
        Mutation::IndexRefresh(IndexRefreshInput {
            root: world.root.clone(),
        }),
        &mut events,
    );
    assert_eq!(refreshed.outcome, Outcome::Success);
    assert_eq!(
        stages(&events.0),
        [ProgressStage::Accepted, ProgressStage::Indexing]
    );
    assert_events_are_of(&events.0, &refreshed);
    assert_progress_golden("index_refresh", &world, &events.0);
}

#[test]
fn a_request_answered_without_running_reports_nothing() {
    let world = World::new();
    // Refused before it is accepted.
    let mut events = Events::default();
    let refused = execute(
        &world,
        REQUEST_1,
        world.save(TICKET_A, "not a token"),
        &mut events,
    );
    assert!(refused.code.failure_class().is_some());
    assert!(world.record(REQUEST_1).is_none());
    assert!(events.0.is_empty());

    // Finished, and answered from its record.
    let token = world.token(TICKET_A);
    let saved = execute(&world, REQUEST_2, world.save(TICKET_A, &token), &mut events);
    assert_eq!(saved.outcome, Outcome::Success);
    assert_eq!(events.0.len(), 2);
    let mut events = Events::default();
    let replayed = execute(&world, REQUEST_2, world.save(TICKET_A, &token), &mut events);
    assert_eq!(replayed, saved);
    assert!(events.0.is_empty());

    // Refused as another request under the same ID.
    let mismatch = execute(&world, REQUEST_2, world.create(TICKET_NEW), &mut events);
    assert_eq!(mismatch.code, ResultCode::RequestMismatch);
    assert!(events.0.is_empty());
}

#[test]
fn a_request_entered_again_reports_its_stage_and_not_that_it_was_accepted() {
    let world = World::with_service(|data| {
        FailOnce::at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence).open_service(data)
    });
    let token = world.token(TICKET_A);
    let mut events = Events::default();
    let partial = execute(&world, REQUEST_1, world.save(TICKET_A, &token), &mut events);
    assert_eq!(partial.outcome, Outcome::Partial);
    assert_eq!(
        stages(&events.0),
        [ProgressStage::Accepted, ProgressStage::Writing]
    );

    let mut events = Events::default();
    let finished = execute(&world, REQUEST_1, world.save(TICKET_A, &token), &mut events);
    assert_eq!(finished.outcome, Outcome::Success);
    // It was accepted once, by the first call.
    assert_eq!(stages(&events.0), [ProgressStage::Writing]);
    assert_events_are_of(&events.0, &finished);
}

/// A sink that panics the first time it is told anything, and counts
/// how often it is told.
#[derive(Default)]
struct Panics(usize);

impl ProgressSink for Panics {
    fn event(&mut self, _event: &ProgressEvent) {
        self.0 += 1;
        panic!("a sink that panics");
    }
}

#[test]
fn a_sink_that_panics_is_sent_nothing_more_and_its_request_runs_on() {
    let world = World::new();
    let before = world.branch_commits(TICKET_A);
    let token = world.token(TICKET_A);
    let mut sink = Panics::default();
    // The panic's own report goes to the test's output and nowhere else.
    let envelope = execute(&world, REQUEST_1, world.save(TICKET_A, &token), &mut sink);

    // The request did all it does with a sink that returns.
    assert_eq!(envelope.outcome, Outcome::Success);
    assert_eq!(envelope.code, ResultCode::Ok);
    assert_eq!(world.branch_commits(TICKET_A), before + 1);
    assert!(world.worktree_source(TICKET_A).contains(TITLE));
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Finished
    );
    assert_eq!(world.pending_journal_rows(), 0);
    // It was told once: the stage that follows was not sent to it.
    assert_eq!(sink.0, 1);

    // The next call's sink is its own, and is sent everything.
    let mut events = Events::default();
    let created = execute(&world, REQUEST_2, world.create(TICKET_NEW), &mut events);
    assert_eq!(created.outcome, Outcome::Success);
    assert_eq!(events.0.len(), 2);
}

fn cancel(world: &World, id: &str) -> Envelope<CancelDto> {
    let envelope = world.service.cancel_request(request(id));
    assert_eq!(envelope.command, "request cancel");
    // The call has no request ID of its own, and names no operation.
    assert_eq!(envelope.request_id, None);
    assert_eq!(envelope.operation_id, None);
    assert_eq!(envelope.effects, Effects::not_requested());
    envelope
}

fn assert_cancel_contract(name: &str, world: &World, envelope: &Envelope<CancelDto>) {
    mutation::assert_contract(
        &ContractCase {
            name,
            data_schema: envelope.data.as_ref().map(|_| CANCEL_SCHEMA),
            placeholders: &[(world.root.to_str().unwrap(), "<repository>")],
            sentinels: &SENTINELS,
        },
        envelope,
    );
}

/// A save has no safe point once it is accepted. Asking it to stop
/// changes nothing: its record stays accepted, with no flag set, for the
/// retry that finishes it.
#[test]
fn a_cancellation_asked_of_a_save_whose_row_is_pending_changes_nothing() {
    let world = World::with_service(|data| {
        FailOnce::at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence).open_service(data)
    });
    let token = world.token(TICKET_A);
    let partial = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(partial.outcome, Outcome::Partial);
    assert_eq!(world.pending_journal_rows(), 1);
    let record = world.record(REQUEST_1).unwrap();
    assert_eq!(record.state, RequestState::Accepted);

    let envelope = cancel(&world, REQUEST_1);
    assert_eq!(envelope.outcome, Outcome::Success);
    assert_eq!(
        envelope.data,
        Some(CancelDto {
            request_id: REQUEST_1.to_owned(),
            state: RequestState::Accepted,
            requested: false,
            acknowledged: false,
        })
    );
    assert_eq!(envelope.scope.repository.as_deref(), world.root.to_str());
    assert_cancel_contract("request_cancel_not_stoppable", &world, &envelope);
    // Nothing of the record or the journal changed.
    assert_eq!(world.record(REQUEST_1).unwrap(), record);
    assert!(!record.cancel_requested);
    assert_eq!(world.pending_journal_rows(), 1);

    // Its retry completes.
    let finished = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(finished.outcome, Outcome::Success);
    assert_eq!(finished.operation_id, partial.operation_id);
    assert_eq!(world.branch_commits(TICKET_A), 1);
    assert_eq!(world.pending_journal_rows(), 0);
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Finished
    );

    // A finished request is not asked anything either.
    let finished_record = world.record(REQUEST_1).unwrap();
    let envelope = cancel(&world, REQUEST_1);
    assert_eq!(
        envelope.data,
        Some(CancelDto {
            request_id: REQUEST_1.to_owned(),
            state: RequestState::Finished,
            requested: false,
            acknowledged: false,
        })
    );
    assert_eq!(world.record(REQUEST_1).unwrap(), finished_record);
}

#[test]
fn a_cancellation_asked_of_a_request_no_record_holds_is_not_found() {
    let world = World::new();
    let envelope = cancel(&world, REQUEST_3);
    assert_eq!(envelope.outcome, Outcome::Error);
    assert_eq!(envelope.code, ResultCode::RequestNotFound);
    assert_eq!(envelope.data, None);
    assert!(envelope.recovery.is_empty());
    assert_cancel_contract("request_cancel_not_found", &world, &envelope);
    assert_eq!(world.request_rows(), 0);
}

/// The record of a synchronization that was accepted and has not yet
/// reserved: there is no row in the remote journal under its operation.
fn accepted_synchronization(world: &World, id: &str) -> NewRequest {
    let scope = ScopeKey::for_repository(&world.root).unwrap();
    let request_id = request(id);
    NewRequest {
        request_id,
        intent_digest: IntentDigest::builder(
            DigestSalt::Request(request_id),
            "repo sync",
            &scope,
            "",
        )
        .finish(),
        scope,
        command: "repo sync".to_owned(),
        target: String::new(),
        confirmation: None,
        base_ref: Some("refs/heads/main".to_owned()),
        base_oid: Some(world.primary_head()),
        expected_digest: None,
        operations: vec![RequestOperation {
            family: OperationFamily::Remote,
            operation_id: OperationId::new(),
        }],
    }
}

/// A synchronization is asked by a flag on its record, which its own
/// call reads at its next safe point. Asking finishes nothing.
#[test]
fn a_cancellation_asked_of_an_accepted_synchronization_is_requested_and_not_acknowledged() {
    let world = World::new();
    world
        .service
        .insert_request(&accepted_synchronization(&world, REQUEST_1))
        .unwrap();
    assert!(!world.record(REQUEST_1).unwrap().cancel_requested);

    let envelope = cancel(&world, REQUEST_1);
    assert_eq!(envelope.outcome, Outcome::Success);
    assert_eq!(
        envelope.data,
        Some(CancelDto {
            request_id: REQUEST_1.to_owned(),
            state: RequestState::Accepted,
            requested: true,
            acknowledged: false,
        })
    );
    assert_cancel_contract("request_cancel", &world, &envelope);
    let record = world.record(REQUEST_1).unwrap();
    assert!(record.cancel_requested);
    assert_eq!(record.state, RequestState::Accepted);
    assert_eq!(record.attempt, 1);
    // Asked again, it is the same.
    assert_eq!(cancel(&world, REQUEST_1).data, envelope.data);
    assert_eq!(world.record(REQUEST_1).unwrap(), record);
}
