//! L3's stages and the steps they light (D5 design, L3; review M-4), with
//! no repository: the remote stages a local-only run never reports, and the
//! stages a run leaves out (R-14, R-15).

use manyhands::repository::ProgressStage;

use super::outcome::{StepKey, StepState, running_states, stage_step};

const ALL: [StepKey; 6] = [
    StepKey::Save,
    StepKey::ShareContext,
    StepKey::MarkClosed,
    StepKey::Merge,
    StepKey::SharePrimary,
    StepKey::TidyUp,
];

fn words(states: Vec<StepState>) -> Vec<&'static str> {
    states.into_iter().map(StepState::words).collect()
}

#[test]
fn each_stage_lights_its_step() {
    use ProgressStage::*;
    for stage in [Connecting, Fetching, UpdatingLocal, Pushing] {
        assert!(
            stage_step(stage) == Some(StepKey::ShareContext),
            "{stage:?}"
        );
    }
    assert!(stage_step(Recording) == Some(StepKey::MarkClosed));
    assert!(stage_step(Integrating) == Some(StepKey::Merge));
    assert!(stage_step(Publishing) == Some(StepKey::SharePrimary));
    assert!(stage_step(CleaningUp) == Some(StepKey::TidyUp));
    for stage in [Accepted, Observing, Writing, Verifying, Indexing, Stopping] {
        assert!(stage_step(stage).is_none(), "{stage:?}");
    }
}

#[test]
fn a_remote_run_shares_its_copy_then_main() {
    let at = |stage| words(running_states(&ALL, stage_step(stage)));
    assert_eq!(
        at(ProgressStage::Fetching),
        [
            "Done",
            "Running",
            "Not started",
            "Not started",
            "Not started",
            "Not started"
        ]
    );
    assert_eq!(
        at(ProgressStage::Publishing),
        ["Done", "Done", "Done", "Done", "Running", "Not started"]
    );
    assert_eq!(
        words(running_states(&ALL, None)),
        [
            "Done",
            "Not started",
            "Not started",
            "Not started",
            "Not started",
            "Not started"
        ],
        "before any step, only the save is done"
    );
}

/// R-14: an already-contained merge reports no `integrating`; it is done
/// once a later stage comes.
#[test]
fn an_already_contained_merge_is_done_once_a_later_stage_comes() {
    let states = words(running_states(&ALL, stage_step(ProgressStage::CleaningUp)));
    assert_eq!(states[3], "Done");
    assert_eq!(states[5], "Running");
}

/// R-15: a closure already recorded reports no `recording`; Mark as closed
/// is done once `integrating` comes.
#[test]
fn an_existing_closure_is_marked_once_integrating_comes() {
    let states = words(running_states(&ALL, stage_step(ProgressStage::Integrating)));
    assert_eq!(states[2], "Done");
    assert_eq!(states[3], "Running");
}
