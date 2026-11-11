//! A running Check now reads its stage in words, as a share does (D6
//! Task 6, Fact 14), and a policy verb reads none: it reports `writing`
//! and nothing else. The requests are fabricated through
//! `Operations::apply`; no poll runs.

use gpui_kit::TestAppContext;
use manyhands::{
    canonical::CONFIG_PATH,
    repository::{ProgressEvent, ProgressStage, RequestId},
    results::{Effects, Scope},
};

use super::{FINISHING, request_id};
use crate::desktop::{
    services::{
        requests::{DesktopRequest, DesktopRequestId, Intent, Lane, RequestState},
        worker::WorkerEvent,
    },
    test_support::fixtures::Fixture,
    view::harness::Shell,
};

/// Fabricated requests stand far above the worker's own numbering.
const FABRICATED: u64 = 2_000_000;

/// Repository `a` published over SSH through `origin`, in an open window.
fn shell(cx: &mut TestAppContext) -> (Shell, std::path::PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.publish_over_ssh(&root);
    let config = std::fs::read(root.join(CONFIG_PATH)).unwrap();
    fixture.commit_file(&root, CONFIG_PATH, &config);
    (Shell::open(fixture, cx), root)
}

/// A request of `intent`, numbered `n` above the worker's, running.
fn running(shell: &Shell, n: u64, intent: Intent, cx: &mut TestAppContext) -> DesktopRequest {
    let request = DesktopRequest {
        id: DesktopRequestId(FABRICATED + n),
        request_id: RequestId::new(),
        intent,
        confirmation: None,
        lane: Lane::User,
        attempt: 1,
        state: RequestState::Queued,
    };
    let queued = request.clone();
    shell.operations(cx).update(cx, |operations, cx| {
        operations.apply(WorkerEvent::Queued(queued));
        operations.apply(WorkerEvent::Started(request.id));
        cx.notify();
    });
    shell.run(cx);
    request
}

fn reach(shell: &Shell, request: &DesktopRequest, stage: ProgressStage, cx: &mut TestAppContext) {
    let event = ProgressEvent {
        request_id: request.request_id.to_string(),
        operation_id: None,
        command: "poll once".to_owned(),
        scope: Scope::default(),
        stage,
        effects: Effects::not_requested(),
    };
    let id = request.id;
    shell.operations(cx).update(cx, |operations, cx| {
        operations.apply(WorkerEvent::Progress(id, event));
        cx.notify();
    });
    shell.run(cx);
}

/// Each stage a Check now reports reads as "Checking · {words}" in its
/// Activity entry, naming the publication remote while connecting. A
/// policy verb reads "Finishing" from its `writing`, whatever the stage:
/// it has no stage words.
#[gpui_kit::test]
fn a_running_check_now_reads_its_stage_words(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx);
    let check = running(&shell, 1, Intent::PollOnce { root: root.clone() }, cx);
    for (stage, text) in [
        (ProgressStage::Accepted, "Getting ready…"),
        (ProgressStage::Connecting, "Connecting to origin…"),
        (ProgressStage::Fetching, "Bringing in changes…"),
        (ProgressStage::UpdatingLocal, "Merging changes…"),
        (ProgressStage::Indexing, "Updating the list…"),
        (ProgressStage::Stopping, "Stopping at a safe point…"),
        // A poll writes no words of its own: the generic ones stand.
        (ProgressStage::Writing, "Getting ready…"),
    ] {
        reach(&shell, &check, stage, cx);
        assert_eq!(
            shell.label(request_id(check.id, "state"), cx).as_deref(),
            Some(format!("Checking · {text}").as_str()),
            "{stage:?}"
        );
    }

    for (n, verb) in [
        Intent::PollConfigure {
            root: root.clone(),
            interval_seconds: 600,
        },
        Intent::PollPause { root: root.clone() },
        Intent::PollResume { root: root.clone() },
    ]
    .into_iter()
    .enumerate()
    {
        let request = running(&shell, 2 + n as u64, verb, cx);
        reach(&shell, &request, ProgressStage::Writing, cx);
        assert_eq!(
            shell.label(request_id(request.id, "state"), cx).as_deref(),
            Some(FINISHING),
            "{}",
            request.intent.label()
        );
    }
}
