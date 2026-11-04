//! The stages of `document promote` and `ticket close` (D5 decision 11, L3):
//! a `d5_progress_` case per order the design names.
//!
//! A lifecycle reports `observing` before its run, then the stages of the steps
//! it runs: `connecting` and its context synchronization's `fetching`,
//! `updating_local` and `pushing` for Share with origin, `recording` for Mark
//! as closed, `integrating` for Merge into main, `publishing` once for Share
//! main with origin, and `cleaning_up` for Tidy up. A step that does not run
//! reports nothing: an existing closure (R-15), a context primary already holds
//! (R-14) and a repository with no publication remote (R-16).

use std::path::Path;

use super::*;
use c5_lifecycle::{FIRST, confirmation_of, lifecycle_of, prepared, stopped_at};
use lifecycle_world::*;
use repository::ProgressStage as S;

pub(crate) const CASES: &[ssh_harness::Case] = &[
    (
        "d5_progress_a_promotion_reports_its_stages_in_order",
        a_promotion_reports_its_stages_in_order,
    ),
    (
        "d5_progress_a_closure_reports_recording_before_integrating",
        a_closure_reports_recording_before_integrating,
    ),
    (
        "d5_progress_publishing_follows_integrating",
        publishing_follows_integrating,
    ),
    (
        "d5_progress_the_primary_childs_safe_points_are_not_forwarded",
        the_primary_childs_safe_points_are_not_forwarded,
    ),
    (
        "d5_progress_a_cleanup_only_replay_reports_accepted_observing_cleaning_up",
        a_cleanup_only_replay_reports_accepted_observing_cleaning_up,
    ),
    (
        "d5_progress_a_local_only_run_never_reports_connecting_or_publishing",
        a_local_only_run_never_reports_connecting_or_publishing,
    ),
];

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

fn a_promotion_reports_its_stages_in_order() -> Result<(), FixtureError> {
    let w = remote(Verb::Promote)?;
    let stages = completed(first_run(&w.service, Verb::Promote, &w.root));
    assert_eq!(
        without_the_context(&stages),
        [
            S::Accepted,
            S::Observing,
            S::Connecting,
            S::Integrating,
            S::Publishing,
            S::CleaningUp
        ],
        "{stages:?}"
    );
    Ok(())
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

/// `publishing` once, after `integrating`; for a context primary already holds,
/// with no merge, right after `observing` (R-16); and for the same request
/// sent again past its context's synchronization, after an `integrating` with
/// no `connecting` before it.
fn publishing_follows_integrating() -> Result<(), FixtureError> {
    for verb in [Verb::Promote, Verb::Close] {
        let w = remote(verb)?;
        let stages = completed(first_run(&w.service, verb, &w.root));
        let at = |wanted| stages.iter().position(|stage| *stage == wanted);
        assert_eq!(
            stages
                .iter()
                .filter(|stage| **stage == S::Publishing)
                .count(),
            1,
            "{verb:?} {stages:?}"
        );
        assert!(at(S::Integrating) < at(S::Publishing), "{stages:?}");
        assert!(at(S::Publishing) < at(S::CleaningUp), "{stages:?}");

        // Already contained, already closed, on a primary the remote holds.
        let w = remote(verb)?;
        if verb == Verb::Close {
            closed_on_its_branch(&verb.worktree(&w.root))?;
        }
        let repo = w.repo()?;
        fast_forward_primary(&repo, verb)?;
        push_peer(&repo, &w.server, MAIN)?;
        fixed(w.sync(w.primary()))?;
        let (done, stages) = first_run(&w.service, verb, &w.root);
        assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
        assert_eq!(
            lifecycle_of(&done).and_then(|data| data.integration),
            Some(LifecycleIntegrationKind::AlreadyContained)
        );
        assert_eq!(
            stages,
            [S::Accepted, S::Observing, S::Publishing, S::CleaningUp],
            "{verb:?}"
        );

        // Sent again past the context's synchronization: no `accepted`, and
        // nothing of the context.
        let mut w = remote(verb)?;
        stopped_at(&mut w, verb, FailurePoint::AfterContextSync)?;
        let stages = completed(staged(
            &w.service,
            FIRST,
            None,
            verb.mutation(&w.root, verb.id()),
        ));
        let mut expected = vec![S::Observing];
        if verb == Verb::Close {
            expected.push(S::Recording);
        }
        expected.extend([S::Integrating, S::Publishing, S::CleaningUp]);
        assert_eq!(stages, expected, "{verb:?}");
    }
    Ok(())
}

/// The primary's publication pushes, and nothing of its synchronization is
/// sent: `cleaning_up` follows `publishing` directly.
fn the_primary_childs_safe_points_are_not_forwarded() -> Result<(), FixtureError> {
    for verb in [Verb::Promote, Verb::Close] {
        let w = remote(verb)?;
        let (done, stages) = first_run(&w.service, verb, &w.root);
        assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
        assert_eq!(done.effects.publication, PublicationEffect::Published);
        assert_eq!(
            fixed(w.bare()?.refname_to_id(MAIN))?,
            fixed(w.repo()?.refname_to_id(MAIN))?
        );
        let publishing = stages
            .iter()
            .position(|stage| *stage == S::Publishing)
            .ok_or(FixtureError)?;
        assert_eq!(
            stages[publishing..],
            [S::Publishing, S::CleaningUp],
            "{verb:?} {stages:?}"
        );
    }
    Ok(())
}

/// A context primary already holds, already closed, in a repository with no
/// publication remote: nothing is recorded, merged, shared or published, and
/// only Tidy up runs (R-16).
fn a_cleanup_only_replay_reports_accepted_observing_cleaning_up() -> Result<(), FixtureError> {
    for verb in [Verb::Promote, Verb::Close] {
        let local = Local::holding(verb)?;
        if verb == Verb::Close {
            closed_on_its_branch(&verb.worktree(&local.root))?;
        }
        fast_forward_primary(&local.repo()?, verb)?;
        let preview = prepared(&local.service, verb, &local.root, verb.id());
        let PreviewDetailsDto::Lifecycle(shown) = &preview
            .data
            .as_ref()
            .and_then(PrepareDataDto::preview)
            .ok_or(FixtureError)?
            .details
        else {
            return Err(FixtureError);
        };
        assert_eq!(
            shown.integration,
            LifecyclePlannedIntegration::AlreadyContained
        );
        if verb == Verb::Close {
            assert_eq!(
                shown.closure.as_ref().map(|closure| closure.recording),
                Some(LifecycleRecording::Existing)
            );
        }
        let (done, stages) = staged(
            &local.service,
            FIRST,
            Some(&confirmation_of(&preview)),
            verb.mutation(&local.root, verb.id()),
        );
        assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
        assert_eq!(
            stages,
            [S::Accepted, S::Observing, S::CleaningUp],
            "{verb:?}"
        );
    }
    Ok(())
}

fn a_local_only_run_never_reports_connecting_or_publishing() -> Result<(), FixtureError> {
    for verb in [Verb::Promote, Verb::Close] {
        let local = Local::holding(verb)?;
        let stages = completed(first_run(&local.service, verb, &local.root));
        let mut expected = vec![S::Accepted, S::Observing];
        if verb == Verb::Close {
            expected.push(S::Recording);
        }
        expected.extend([S::Integrating, S::CleaningUp]);
        assert_eq!(stages, expected, "{verb:?}");
    }
    Ok(())
}

/// Records the ticket in `worktree` as closed, on its branch.
fn closed_on_its_branch(worktree: &Path) -> Result<(), FixtureError> {
    let path = format!(".manyhands/tickets/{ITEM}/ticket.md");
    let source = fixed(std::fs::read_to_string(worktree.join(&path)))?;
    let closed = source.replacen(
        "---\n",
        "---\nclosed_at: 2027-01-15T08:00:00Z\nclosed_by: Fixture <fixture@example.invalid>\n",
        1,
    );
    commit_in(worktree, Verb::Close.branch(), &path, closed.as_bytes())?;
    Ok(())
}
