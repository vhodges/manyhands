//! The stages of `ticket close` over SSH (D5 decision 11, L3): one `d5_progress_`
//! case, the end-to-end one.
//!
//! A lifecycle reports `observing` before its run, then the stages of the steps
//! it runs: `connecting` and its context synchronization's `fetching` and
//! `pushing`, `recording` for Mark as closed, `integrating` for Merge into
//! main, `publishing` once for Share main with origin, and `cleaning_up` for
//! Tidy up. Only the remote path needs SSH. The rest of the design's orders are
//! library tests, over the steps a run takes and the stages the boundary makes
//! of them: `promotion::run_tests::a_promotion_tells_its_observer_only_the_
//! steps_that_run` (promotion's order, a context primary already holds (R-14),
//! no publication remote (R-16), a request sent again past its context's
//! synchronization), `closure::run_tests::a_closure_tells_its_observer_only_
//! the_steps_that_run` (the same for a closure, `recording` first) and
//! `mutation::bind::lifecycle::tests::a_runs_steps_are_reported_as_stages`
//! (which events are stages: the primary's safe points are none, a closure
//! already recorded is not recorded again (R-15)).

use std::path::Path;

use super::*;
use c5_lifecycle::{FIRST, confirmation_of, prepared};
use lifecycle_world::*;
use repository::ProgressStage as S;

pub(crate) const CASES: &[ssh_harness::Case] = &[(
    "d5_progress_a_closure_reports_recording_before_integrating",
    a_closure_reports_recording_before_integrating,
)];

/// One call of `execute`, with the stages its sink was sent.
fn staged(
    service: &RepositoryService,
    id: &str,
    confirmation: Option<&str>,
    mutation: Mutation,
) -> (Envelope<MutationDataDto>, Vec<ProgressStage>) {
    let mut events = Events::default();
    let envelope = service.execute(
        MutationCall {
            request_id: request(id),
            confirmation: confirmation.map(|id| ConfirmationId::parse(id).unwrap()),
            mutation,
        },
        &mut SessionCredentials::new(Provider),
        &mut events,
        &CancellationToken::new(),
    );
    for event in &events.0 {
        assert_eq!(event.request_id, id);
        assert_eq!(event.operation_id, envelope.operation_id);
    }
    (envelope, events.0.iter().map(|event| event.stage).collect())
}

/// A first run of the verb's item on `service`, as prepared now.
fn first_run(
    service: &RepositoryService,
    verb: Verb,
    root: &Path,
) -> (Envelope<MutationDataDto>, Vec<ProgressStage>) {
    let confirmation = confirmation_of(&prepared(service, verb, root, verb.id()));
    staged(
        service,
        FIRST,
        Some(&confirmation),
        verb.mutation(root, verb.id()),
    )
}

/// The stages of a run that completed.
fn completed(
    (envelope, stages): (Envelope<MutationDataDto>, Vec<ProgressStage>),
) -> Vec<ProgressStage> {
    assert_eq!(envelope.code, ResultCode::Ok, "{:?}", envelope.data);
    stages
}

/// The stages of a remote run, with its context synchronization's stages taken
/// out once they are checked: between `connecting` and the next step's stage,
/// exactly those of a context that is fetched and then published (U-4).
fn without_the_context(stages: &[ProgressStage]) -> Vec<ProgressStage> {
    let connecting = stages
        .iter()
        .position(|stage| *stage == S::Connecting)
        .expect("a context synchronization");
    let mut kept = stages[..=connecting].to_vec();
    let mut rest = stages[connecting + 1..].iter().peekable();
    let mut context = Vec::new();
    while let Some(stage) =
        rest.next_if(|stage| matches!(stage, S::Fetching | S::UpdatingLocal | S::Pushing))
    {
        context.push(*stage);
    }
    // The context of a remote world is saved and never pushed: its child
    // fetches, then publishes it.
    assert_eq!(context, [S::Fetching, S::Pushing], "{stages:?}");
    kept.extend(rest);
    kept
}

fn a_closure_reports_recording_before_integrating() -> Result<(), FixtureError> {
    let w = remote(Verb::Close)?;
    let stages = completed(first_run(&w.service, Verb::Close, &w.root));
    assert_eq!(
        without_the_context(&stages),
        [
            S::Accepted,
            S::Observing,
            S::Connecting,
            S::Recording,
            S::Integrating,
            S::Publishing,
            S::CleaningUp
        ],
        "{stages:?}"
    );
    Ok(())
}
