//! P1 (Cycle 05b, A1, A2): the lifecycle dialog's stage, and the title and
//! subtitle its header shows there, for both commands. A real ticket and
//! document give the stages that hold the preview (confirm, running, done,
//! stopped), where the Git line is drawn too (A5, A6); canned requests give
//! the stages of a run shown without it (reopened from Activity, K-15), and
//! the message-only ones that take the Ask title (K-10).
//!
//! P4 (A3): the step rows' marks and notes. P2 (A5, A7, R-23): each stage's
//! footer, read in drawn order from the bounds of what `lifecycle-footer`
//! holds (design, Testing). P3 (A5, R-5): the backdrop hides a running
//! dialog only.

use std::path::Path;

use gpui_kit::{ElementId, Modifiers, TestAppContext, point, px, test::TestWindowExt};
use manyhands::{
    repository::{
        FailurePoint, ItemDtoKind, LifecycleBlockedDto, LifecycleBlocker,
        LifecycleCleanupPendingReason, LifecycleCleanupState, LifecycleMadeReadyState,
        LifecycleRecording, OperationId, PrepareDataDto, ProgressEvent, ProgressStage,
        RepositoryService, RequestId,
    },
    results::{
        CleanupEffect, Effects, Envelope, Outcome, RecoveryAction, RecoveryActionKind, ResultCode,
        Scope,
    },
};

use super::{
    CANCEL, CLOSE, CLOSED, CLOSING, CONFIRM_HINT, COULDNT_CLOSE, COULDNT_MERGE, DONE, HIDE_HINT,
    Heading, MERGING, NOT_NOW, Opening, PREPARING, RUNNING_LEAD, STILL_RUNNING, approve_title,
    close_title,
    confirm::steps,
    confirm_tests::{confirming, worktree},
    outcome::{
        ENDING_STOPPED, STATE_DONE, STATE_FAILED, STATE_NOT_DONE, STATE_NOT_STARTED, STATE_RUNNING,
        merged,
    },
    outcome_canned_tests::{
        answer, finished, inspect, intent, reached, reached_effects, reopen as open_entry, retry,
        short_of_main,
    },
    outcome_tests::{at_stage, stopped_at_integrating},
    privacy_promote_tests::saved_document,
    privacy_tests::TITLE,
    short_id,
    stages::Stage,
    tests::{
        DOCUMENT, SLUG, TICKET, closes, code, dialog, edit, open, pane, request, saved_ticket,
        shell, shown, ticket, until_present,
    },
};
use crate::desktop::{
    model::{navigation::OpenTarget, operations::stage_words},
    services::{
        JobKind, ReadKind,
        requests::{DesktopRequest, DesktopRequestId, Intent, Lane, RequestState},
        worker::WorkerEvent,
    },
    test_support::fixtures::item_id,
    view::{
        harness::{Shell, key},
        item::LIFECYCLE_ACTION,
    },
};

/// The dialog's stage, and its header's title and subtitle, now.
fn header(shell: &Shell, cx: &mut TestAppContext) -> (Stage, Option<String>, Option<String>) {
    let stage = dialog(cx).read_with(cx, |dialog, cx| dialog.stage(cx));
    (
        stage,
        shell.label("lifecycle-title", cx),
        shell.label("lifecycle-subtitle", cx),
    )
}

/// The dialog is at `stage`, and its header reads `title` and `subtitle`.
fn is_at(
    shell: &Shell,
    row: &str,
    (stage, title, subtitle): (Stage, &str, &str),
    cx: &mut TestAppContext,
) {
    shell.run(cx);
    assert_eq!(
        header(shell, cx),
        (stage, Some(title.to_owned()), Some(subtitle.to_owned())),
        "{row}"
    );
}

/// The Git line names the branch the run merges, as the preview did.
fn names(shell: &Shell, branch: &str, row: &str, cx: &mut TestAppContext) {
    let git = shell.label("lifecycle-git", cx);
    assert!(
        git.as_deref().is_some_and(|git| git.contains(branch)),
        "{row}: the Git line is {git:?}"
    );
}

/// A request `n` of `intent` in the request model, with what `events` say
/// of it after it is queued.
fn fabricate(
    shell: &Shell,
    n: u64,
    intent: Intent,
    events: impl FnOnce(DesktopRequestId) -> Vec<WorkerEvent>,
    cx: &mut TestAppContext,
) -> DesktopRequestId {
    let id = DesktopRequestId(super::outcome_canned_tests::FABRICATED + n);
    let request = DesktopRequest {
        id,
        request_id: RequestId::new(),
        intent,
        confirmation: None,
        lane: Lane::User,
        attempt: 1,
        state: RequestState::Queued,
        registered_root: None,
    };
    shell.operations(cx).update(cx, |operations, cx| {
        operations.apply(WorkerEvent::Queued(request));
        for event in events(id) {
            operations.apply(event);
        }
        cx.notify();
    });
    shell.run(cx);
    id
}

/// Opens the dialog on the request `id` as Activity's Open… does, with the
/// workspace knowing main and no remote, though the entry does not offer it
/// for a run still going.
fn open_on(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) {
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    let request = shell
        .operations(cx)
        .read_with(cx, |operations, _| operations.request(id).cloned())
        .expect("the request is listed");
    shell.visual(cx).update(|window, cx| {
        let origin = cx.focus_handle();
        super::reopen(
            &request,
            (Some("main".to_owned()), Some(None)),
            origin,
            &services,
            window,
            cx,
        )
    });
}

/// Opens the dialog on the resume `resume` of the close or merge of `kind`,
/// as Resume on an entry does.
fn follow(
    shell: &Shell,
    root: &Path,
    kind: ItemDtoKind,
    resume: DesktopRequestId,
    cx: &mut TestAppContext,
) {
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    let item = item_of(kind);
    let heading = Heading {
        kind,
        short: short_id(item),
        title: None,
        repository: "a".to_owned(),
        primary: Some("main".to_owned()),
        remote: Some(None),
    };
    shell.visual(cx).update(|window, cx| {
        let origin = cx.focus_handle();
        super::follow_resume(
            intent(root, kind),
            heading,
            resume,
            origin,
            &services,
            window,
            cx,
        )
    });
}

fn item_of(kind: ItemDtoKind) -> &'static str {
    match kind {
        ItemDtoKind::Ticket => TICKET,
        ItemDtoKind::Document => DOCUMENT,
    }
}

/// How the run of a canned row stands.
enum Run {
    /// Sent, and no answer yet.
    Running,
    /// Taken off the queue before it ran.
    Removed,
    /// Answered, with lifecycle data.
    Answered(ResultCode, Outcome),
    /// A resume answered `busy`, with no lifecycle data (O-2).
    ResumedBusy,
}

/// An answer of the close or merge of `kind` in `root`, as `code` and
/// `outcome` say.
fn answered(
    root: &Path,
    kind: ItemDtoKind,
    code: ResultCode,
    outcome: Outcome,
    request: RequestId,
) -> Envelope<manyhands::repository::MutationDataDto> {
    let item = item_of(kind);
    let (data, effects, recovery) = match code {
        ResultCode::Ok => (
            reached(kind, item, vec![]),
            reached_effects(CleanupEffect::Complete),
            vec![],
        ),
        _ => (
            short_of_main(kind, item, vec![]),
            Effects::not_requested(),
            vec![retry(request)],
        ),
    };
    // The helper builds no cancellation: it is set on what it builds.
    let cancelled = code == ResultCode::Cancelled;
    let built = if cancelled { ResultCode::Busy } else { code };
    let mut envelope = answer(
        "ticket close",
        root,
        built,
        outcome,
        data,
        effects,
        recovery,
        request,
    );
    envelope.code = code;
    envelope
}

/// P1, A1, A2, O-2: each stage's header title, for both commands, and the
/// subtitle, which is today's at every stage (R-1, K-10).
#[gpui_kit::test]
async fn each_stage_has_its_title(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Coded", &format!("slug: {SLUG}\n"));
        saved_document(fixture, root);
    });
    let ticket_branch = shown(&shell, &root, TICKET).context.branch.unwrap();
    let document_branch = shown(&shell, &root, DOCUMENT).context.branch.unwrap();
    let ticket_subtitle = format!("{SLUG} \u{b7} Coded \u{b7} a");
    let document_subtitle = format!("{} \u{b7} {TITLE} \u{b7} a", short_id(DOCUMENT));

    // The run of a ticket, which holds its preview: confirm, then stopped.
    super::confirm_tests::confirming(&shell, &root, cx).await;
    is_at(
        &shell,
        "ticket, confirm",
        (Stage::Ask, &close_title(SLUG), &ticket_subtitle),
        cx,
    );
    shell.keys("escape", cx);
    stopped_at_integrating(&shell, &root, cx).await;
    is_at(
        &shell,
        "ticket, stopped",
        (Stage::Stopped, ENDING_STOPPED, &ticket_subtitle),
        cx,
    );
    names(&shell, &ticket_branch, "ticket, stopped", cx);
    shell.keys("escape", cx);

    // The run of a document: confirm, running, then done.
    open(
        &shell,
        OpenTarget::Item {
            repo: key(&root),
            kind: ItemDtoKind::Document,
            id: item_id(DOCUMENT),
        },
        cx,
    );
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    is_at(
        &shell,
        "document, confirm",
        (
            Stage::Ask,
            &approve_title(&short_id(DOCUMENT)),
            &document_subtitle,
        ),
        cx,
    );
    let gate = shell
        .backend
        .hold_at_stage(JobKind::DocumentPromote, ProgressStage::Integrating);
    shell.click("lifecycle-confirm", cx);
    gate.wait_reached();
    let id = shell.requests(cx, |request| {
        matches!(request.intent, Intent::DocumentPromote { .. })
    })[0];
    at_stage(&shell, id, ProgressStage::Integrating, cx).await;
    is_at(
        &shell,
        "document, running",
        (Stage::Running, MERGING, &document_subtitle),
        cx,
    );
    names(&shell, &document_branch, "document, running", cx);
    gate.release();
    shell.until_ended(id, cx).await;
    until_present(&shell, "lifecycle-ending", cx).await;
    is_at(
        &shell,
        "document, done",
        (Stage::Done, &merged("main"), &document_subtitle),
        cx,
    );
    names(&shell, &document_branch, "document, done", cx);
    shell.keys("escape", cx);
    assert!(!shell.present("lifecycle-dialog", cx));

    // Runs shown without their preview (reopened, K-15), and those that
    // show a message only (K-10), for both commands.
    let commands = [
        (
            ItemDtoKind::Ticket,
            close_title(&short_id(TICKET)),
            [
                CLOSING.to_owned(),
                CLOSED.to_owned(),
                COULDNT_CLOSE.to_owned(),
            ],
        ),
        (
            ItemDtoKind::Document,
            approve_title(&short_id(DOCUMENT)),
            [MERGING.to_owned(), merged("main"), COULDNT_MERGE.to_owned()],
        ),
    ];
    let mut n = 100;
    for (kind, asks, [running, done, failed]) in commands {
        let subtitle = format!("{} \u{b7} a", short_id(item_of(kind)));
        let rows = [
            ("running", Stage::Running, running, Run::Running),
            (
                "waiting",
                Stage::Waiting,
                STILL_RUNNING.to_owned(),
                Run::Answered(ResultCode::Busy, Outcome::Partial),
            ),
            (
                "done",
                Stage::Done,
                done,
                Run::Answered(ResultCode::Ok, Outcome::Success),
            ),
            (
                "failed",
                Stage::Failed,
                failed,
                Run::Answered(ResultCode::RecoveryRequired, Outcome::Blocked),
            ),
            (
                "stopped",
                Stage::Stopped,
                ENDING_STOPPED.to_owned(),
                Run::Answered(ResultCode::Cancelled, Outcome::Cancelled),
            ),
            ("removed", Stage::Ask, asks.clone(), Run::Removed),
            ("a resume answered busy", Stage::Ask, asks, Run::ResumedBusy),
        ];
        for (name, stage, title, run) in rows {
            n += 1;
            let row = format!("{kind:?}, {name}");
            let request = RequestId::new();
            match run {
                Run::Running => {
                    let id = fabricate(
                        &shell,
                        n,
                        intent(&root, kind),
                        |id| vec![WorkerEvent::Started(id)],
                        cx,
                    );
                    open_on(&shell, id, cx);
                }
                Run::Removed => {
                    let id = fabricate(
                        &shell,
                        n,
                        intent(&root, kind),
                        |id| vec![WorkerEvent::Removed(id)],
                        cx,
                    );
                    open_on(&shell, id, cx);
                }
                Run::Answered(code, outcome) => {
                    let envelope = answered(&root, kind, code, outcome, request);
                    let id = finished(&shell, n, intent(&root, kind), request, envelope, cx);
                    if code == ResultCode::Ok {
                        // A finished-ok entry offers no Open….
                        open_on(&shell, id, cx);
                    } else {
                        open_entry(&shell, id, cx).await;
                    }
                }
                Run::ResumedBusy => {
                    let operation = OperationId::parse("01ARZ3NDEKTSV4RRFFQ69G5FY1").unwrap();
                    let mut envelope =
                        answered(&root, kind, ResultCode::Busy, Outcome::Partial, request);
                    envelope.data = None;
                    let resume = Intent::OperationResume {
                        root: root.clone(),
                        operation_id: operation,
                    };
                    let id = finished(&shell, n, resume, request, envelope, cx);
                    follow(&shell, &root, kind, id, cx);
                }
            }
            until_present(&shell, "lifecycle-title", cx).await;
            is_at(&shell, &row, (stage, &title, &subtitle), cx);
            assert!(
                !shell.present("lifecycle-git", cx),
                "{row}: no preview, no Git line"
            );
            shell.keys("escape", cx);
            assert!(!shell.present("lifecycle-dialog", cx), "{row}");
        }
    }
}

/// Where the failure's message is drawn.
#[derive(Debug, PartialEq, Eq)]
enum Home {
    /// Under the step row `ix`, as that step's note.
    Row(usize),
    /// In the outcome box.
    Box,
}

/// Where `lifecycle-message` is drawn: inside a step row, or the box.
fn message_home(shell: &Shell, cx: &mut TestAppContext) -> Option<Home> {
    shell.frame(cx, |window, _| {
        let message = ElementId::from("lifecycle-message");
        let homes: Vec<Home> = gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .filter(|element| element.path().last() == Some(&message))
            .map(|element| {
                let path = element.path();
                let row =
                    (0..16).find(|ix| path.contains(&ElementId::from(("lifecycle-step-row", *ix))));
                match row {
                    Some(ix) => Home::Row(ix),
                    None => {
                        assert!(path.contains(&ElementId::from("lifecycle-box")), "{path:?}");
                        Home::Box
                    }
                }
            })
            .collect();
        assert!(homes.len() <= 1, "the message is drawn once: {homes:?}");
        homes.into_iter().next()
    })
}

/// The note under each of `count` step rows: `None` where it draws none.
fn notes(shell: &Shell, count: usize, cx: &mut TestAppContext) -> Vec<Option<String>> {
    (0..count)
        .map(|ix| shell.label(("lifecycle-step-note", ix), cx))
        .collect()
}

/// The words on each step's mark.
fn marks(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    shell.rows("lifecycle-step-state", cx)
}

/// The notes the preview gives its steps.
fn own_notes(cx: &mut TestAppContext) -> Vec<Option<String>> {
    let plan = dialog(cx)
        .read_with(cx, |dialog, _| dialog.plan.clone())
        .expect("the dialog holds its preview");
    steps(&plan)
        .into_iter()
        .map(|step| Some(step.note))
        .collect()
}

/// The words of a row of marks.
fn words(states: &[&str]) -> Vec<String> {
    states.iter().map(|state| (*state).to_owned()).collect()
}

/// A canned row of P4: a run shown without its preview, as Activity's
/// Open… shows it (K-15).
enum Canned {
    /// Sent, and held at `integrating` (the stage is sent after the dialog
    /// is open).
    Held,
    /// Answered, as `code` and `outcome` say, with `data`.
    Answered(
        ResultCode,
        Outcome,
        manyhands::repository::LifecycleMutationDto,
        Effects,
    ),
}

/// P4, A3, K-15, Review Focus 3: the step rows show their state on the mark
/// and the progress in the note. A run held at a stage, with and without the
/// preview, has the stage's words as the Running step's note; with the
/// preview the other steps keep its notes, and without it they have none. A
/// failure whose blockers name the merge shows its message under Merge, in
/// place of the step's note; with no Failed step (Waiting, or a failure
/// whose steps are all done) the message goes in the outcome box instead.
#[gpui_kit::test]
async fn steps_show_their_state_and_progress(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Coded", &format!("slug: {SLUG}\n"));
        saved_document(fixture, root);
    });

    // Real, with the preview: a promotion held at `integrating`.
    open(
        &shell,
        OpenTarget::Item {
            repo: key(&root),
            kind: ItemDtoKind::Document,
            id: item_id(DOCUMENT),
        },
        cx,
    );
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    let gate = shell
        .backend
        .hold_at_stage(JobKind::DocumentPromote, ProgressStage::Integrating);
    shell.click("lifecycle-confirm", cx);
    gate.wait_reached();
    let id = shell.requests(cx, |request| {
        matches!(request.intent, Intent::DocumentPromote { .. })
    })[0];
    at_stage(&shell, id, ProgressStage::Integrating, cx).await;
    shell.run(cx);
    assert_eq!(
        marks(&shell, cx),
        words(&[STATE_DONE, STATE_DONE, STATE_RUNNING, STATE_NOT_STARTED]),
        "held, with the preview"
    );
    let mut expected = own_notes(cx);
    expected[2] = Some(stage_words(ProgressStage::Integrating, Some("main")));
    assert_eq!(notes(&shell, 4, cx), expected, "held, with the preview");
    assert_eq!(
        shell.label("lifecycle-state", cx).as_deref(),
        Some(RUNNING_LEAD)
    );
    gate.release();
    shell.until_ended(id, cx).await;
    shell.keys("escape", cx);

    // Without the preview, held at a stage: only the Running step has a
    // note, and the others none. Failures, whose blockers name the merge
    // or none, and a Waiting answer.
    let request_id = RequestId::new();
    let mut blocked = short_of_main(
        ItemDtoKind::Ticket,
        TICKET,
        vec![LifecycleBlocker::IntegrationConflict],
    );
    blocked.recording = Some(LifecycleRecording::Record);
    let rows = [
        (
            "held",
            Canned::Held,
            [
                STATE_DONE,
                STATE_DONE,
                STATE_DONE,
                STATE_RUNNING,
                STATE_NOT_STARTED,
            ],
            None,
        ),
        (
            "blockers name the merge",
            Canned::Answered(
                ResultCode::RecoveryRequired,
                Outcome::Blocked,
                blocked,
                Effects::not_requested(),
            ),
            [
                STATE_DONE,
                STATE_DONE,
                STATE_DONE,
                STATE_FAILED,
                STATE_NOT_DONE,
            ],
            Some(Home::Row(3)),
        ),
        (
            "waiting",
            Canned::Answered(
                ResultCode::Busy,
                Outcome::Partial,
                short_of_main(ItemDtoKind::Ticket, TICKET, vec![]),
                Effects::not_requested(),
            ),
            [
                STATE_DONE,
                STATE_NOT_STARTED,
                STATE_NOT_STARTED,
                STATE_NOT_STARTED,
                STATE_NOT_STARTED,
            ],
            Some(Home::Box),
        ),
        (
            "failed, every step done",
            Canned::Answered(
                ResultCode::RecoveryRequired,
                Outcome::Blocked,
                reached(ItemDtoKind::Ticket, TICKET, vec![]),
                reached_effects(CleanupEffect::Complete),
            ),
            [STATE_DONE; 5],
            Some(Home::Box),
        ),
    ];
    for (n, (name, run, states, home)) in rows.into_iter().enumerate() {
        let n = 200 + n as u64;
        let message = match run {
            Canned::Held => {
                let id = fabricate(
                    &shell,
                    n,
                    intent(&root, ItemDtoKind::Ticket),
                    |id| vec![WorkerEvent::Started(id)],
                    cx,
                );
                open_on(&shell, id, cx);
                shell.operations(cx).update(cx, |operations, cx| {
                    operations.apply(WorkerEvent::Progress(
                        id,
                        ProgressEvent {
                            request_id: request_id.to_string(),
                            operation_id: None,
                            command: "ticket close".to_owned(),
                            scope: Scope::default(),
                            stage: ProgressStage::Integrating,
                            effects: Effects::not_requested(),
                        },
                    ));
                    cx.notify();
                });
                None
            }
            Canned::Answered(code, outcome, data, effects) => {
                let envelope = answer(
                    "ticket close",
                    &root,
                    code,
                    outcome,
                    data,
                    effects,
                    vec![retry(request_id)],
                    request_id,
                );
                let message = envelope.message.clone();
                let id = finished(
                    &shell,
                    n,
                    intent(&root, ItemDtoKind::Ticket),
                    request_id,
                    envelope,
                    cx,
                );
                open_entry(&shell, id, cx).await;
                Some(message)
            }
        };
        shell.run(cx);
        assert_eq!(marks(&shell, cx), words(&states), "{name}");
        let mut expected = vec![None; states.len()];
        if message.is_none() {
            expected[3] = Some(stage_words(ProgressStage::Integrating, Some("main")));
        }
        assert_eq!(
            notes(&shell, states.len(), cx),
            expected,
            "{name}: no other notes"
        );
        assert_eq!(message_home(&shell, cx), home, "{name}");
        if let Some(message) = message {
            assert_eq!(
                shell.label("lifecycle-message", cx),
                Some(message),
                "{name}"
            );
        }
        assert!(!shell.present("lifecycle-git", cx), "{name}: no preview");
        shell.keys("escape", cx);
        assert!(!shell.present("lifecycle-dialog", cx), "{name}");
    }

    // Real, with the preview: main turns dirty after Confirm (P19), so the
    // merge fails, and its message stands where its own note did.
    super::confirm_tests::confirming(&shell, &root, cx).await;
    let dirty = root.join("dirty-primary.txt");
    shell
        .fixture
        .service()
        .set_request_hook_for_testing(move || std::fs::write(dirty, b"dirty\n").unwrap());
    shell.click("lifecycle-confirm", cx);
    let id = closes(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    until_present(&shell, "lifecycle-ending", cx).await;
    let RequestState::Finished(.., message) = request(&shell, id, cx).state else {
        panic!("the close ended");
    };
    assert_eq!(
        marks(&shell, cx),
        words(&[
            STATE_DONE,
            STATE_NOT_DONE,
            STATE_NOT_DONE,
            STATE_FAILED,
            STATE_NOT_DONE
        ]),
        "failed at the merge, with the preview"
    );
    let mut expected = own_notes(cx);
    expected[3] = None;
    assert_eq!(
        notes(&shell, 5, cx),
        expected,
        "the merge's own note gives way"
    );
    assert_eq!(message_home(&shell, cx), Some(Home::Row(3)));
    assert_eq!(shell.label("lifecycle-message", cx), Some(message));
}

/// The footer as drawn: the ids of what `lifecycle-footer` holds, by their
/// left edge (design, Testing), with their paths, and its hint, read by the
/// frame's hint id inside the footer.
struct Drawn {
    ids: Vec<String>,
    paths: Vec<Vec<ElementId>>,
    hint: Option<String>,
}

fn drawn_footer(shell: &Shell, cx: &mut TestAppContext) -> Drawn {
    shell.frame(cx, |window, _| {
        let footer = ElementId::from("lifecycle-footer");
        let mut hint = None;
        let mut drawn = Vec::new();
        for element in gpui_kit::base::test_support::snapshots(window) {
            let path = element.path();
            let Some(at) = path.iter().position(|id| *id == footer) else {
                continue;
            };
            // The footer's own children: each button's wrapper, what stands
            // in a button's place, and the hint.
            let Some(ElementId::Name(name)) = path.last().filter(|_| path.len() == at + 2) else {
                continue;
            };
            if name.as_ref() == "frame-hint" {
                hint = element.label().map(str::to_owned);
            } else {
                drawn.push((element.bounds().left(), name.to_string(), path.to_vec()));
            }
        }
        drawn.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let (ids, paths) = drawn.into_iter().map(|(_, id, path)| (id, path)).unzip();
        Drawn { ids, paths, hint }
    })
}

/// The order in which Tab, pressed from the dialog's own focus, first puts
/// focus inside each of the footer's controls.
fn tab_order(shell: &Shell, drawn: &Drawn, cx: &mut TestAppContext) -> Vec<String> {
    let own = dialog(cx).read_with(cx, |dialog, _| dialog.focus().clone());
    shell.frame(cx, |window, cx| window.focus(&own, cx));
    let mut reached: Vec<String> = Vec::new();
    for _ in 0..2 * drawn.ids.len() + 4 {
        shell.keys("tab", cx);
        let now = shell.frame(cx, |window, _| {
            drawn
                .paths
                .iter()
                .position(|path| gpui_kit::base::test_support::has_observed_focus(window, path))
        });
        if let Some(ix) = now
            && !reached.contains(&drawn.ids[ix])
        {
            reached.push(drawn.ids[ix].clone());
        }
    }
    reached
}

/// A row of P2: the dialog's stage; what its footer holds, in drawn order;
/// the dismiss button's label, `None` with no dismiss button; the hint.
struct Foot {
    stage: Stage,
    ids: &'static [&'static str],
    dismiss: Option<&'static str>,
    hint: Option<&'static str>,
}

/// The dialog is at `foot`'s stage, its footer draws what `foot` names in
/// that order, with its dismiss label and hint, and Tab reaches them in
/// that order.
fn footed(shell: &Shell, row: &str, foot: Foot, cx: &mut TestAppContext) {
    shell.run(cx);
    let stage = dialog(cx).read_with(cx, |dialog, cx| dialog.stage(cx));
    assert_eq!(stage, foot.stage, "{row}: the stage");
    let drawn = drawn_footer(shell, cx);
    assert_eq!(drawn.ids, foot.ids, "{row}: the footer, in drawn order");
    assert_eq!(
        shell.label("lifecycle-cancel", cx).as_deref(),
        foot.dismiss,
        "{row}: the dismiss button"
    );
    assert_eq!(drawn.hint.as_deref(), foot.hint, "{row}: the hint");
    assert_eq!(
        tab_order(shell, &drawn, cx),
        foot.ids,
        "{row}: Tab reaches them in that order"
    );
}

/// Escape closes the dialog.
fn escape(shell: &Shell, row: &str, cx: &mut TestAppContext) {
    shell.keys("escape", cx);
    assert!(!shell.present("lifecycle-dialog", cx), "{row}: closed");
}

/// A canned prepare answer of the ticket's close: `code`, blocked by
/// `blockers` when it names some, with `recovery`.
fn prepared(
    root: &Path,
    code: ResultCode,
    blockers: Option<Vec<LifecycleBlocker>>,
    recovery: Vec<RecoveryAction>,
) -> Envelope<PrepareDataDto> {
    let mut envelope = Envelope::<PrepareDataDto>::failure(
        "ticket close",
        Scope {
            repository: Some(key(root)),
            item_id: Some(TICKET.to_owned()),
            ..Scope::default()
        },
        code,
        recovery,
    );
    envelope.data =
        blockers.map(|blockers| PrepareDataDto::LifecycleBlocked(LifecycleBlockedDto { blockers }));
    envelope
}

/// Whether `origin` holds focus, after two draws.
fn focused(shell: &Shell, origin: &gpui_kit::FocusHandle, cx: &mut TestAppContext) -> bool {
    for _ in 0..2 {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
    }
    shell.frame(cx, |window, _| origin.is_focused(window))
}

/// An Ask row of P2.
fn ask(ids: &'static [&'static str], dismiss: &'static str, hint: Option<&'static str>) -> Foot {
    Foot {
        stage: Stage::Ask,
        ids,
        dismiss: Some(dismiss),
        hint,
    }
}

/// A Running row of P2: no dismiss button, and the hint.
fn running(ids: &'static [&'static str]) -> Foot {
    Foot {
        stage: Stage::Running,
        ids,
        dismiss: None,
        hint: Some(HIDE_HINT),
    }
}

/// A Failed row of P2: Not now first.
fn failing(ids: &'static [&'static str]) -> Foot {
    Foot {
        stage: Stage::Failed,
        ids,
        dismiss: Some(NOT_NOW),
        hint: None,
    }
}

/// How a canned row of P2 is shown: a run sent and not answered, or taken
/// off the queue, opened as Activity's Open… would; or answered with an
/// envelope, opened by its entry's Open… (`Entry`) or, for a finished-ok
/// run its entry offers no Open… for, directly (`Direct`).
enum Opened {
    Started,
    Removed,
    Entry(Envelope<manyhands::repository::MutationDataDto>),
    Direct(Envelope<manyhands::repository::MutationDataDto>),
}

/// P2, A5, A7, R-23, Ruling O-5: each stage's footer. The dismiss button
/// (`lifecycle-cancel`, labelled by the stage) comes first, then today's
/// non-primary buttons in today's order, then today's primary ones; on
/// Done, Done comes last; while running, Stop alone, and after it is
/// pressed "Stopping…" in its place, with the hint only. A row per line of
/// A7's table, and per Review Focus input: Stop pressed; Done after Stop; a
/// failed run offering Retry alone, a locked key's Unlock… with no Retry,
/// and a locked key with Prepare again and Unlock…, both last (O-5);
/// Waiting, with Close first. After Stop is pressed, Tab and Shift+Tab stay
/// in the dialog, and Escape still hides it, with focus on the origin.
#[gpui_kit::test]
async fn each_stage_orders_its_footer(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Footers", "");
        saved_document(fixture, root);
    });
    // Ask: the save first, the prepare, then the confirm, in one dialog.
    open(&shell, ticket(&root), cx);
    let host = edit(&shell, cx).await;
    shell.type_at_end(&host, "x", cx);
    let saving = shell.backend.hold_job(JobKind::TicketSave);
    let preparing = shell.backend.hold_read(ReadKind::Prepare);
    shell.click(LIFECYCLE_ACTION, cx);
    saving.wait_reached();
    footed(
        &shell,
        "saving",
        ask(&["lifecycle-cancel"], CANCEL, None),
        cx,
    );
    saving.release();
    shell
        .until_drawn(cx, "the prepare", |shell, cx| {
            shell.label("lifecycle-state", cx).as_deref() == Some(PREPARING)
        })
        .await;
    footed(
        &shell,
        "preparing",
        ask(&["lifecycle-cancel"], CANCEL, None),
        cx,
    );
    drop(preparing);
    until_present(&shell, "lifecycle-confirm", cx).await;
    footed(
        &shell,
        "confirm",
        ask(
            &["lifecycle-cancel", "lifecycle-confirm"],
            CANCEL,
            Some(CONFIRM_HINT),
        ),
        cx,
    );
    escape(&shell, "confirm", cx);
    if shell.present("item-done", cx) {
        shell.click("item-done", cx);
    }

    // Ask: the prepare's other answers, canned.
    let holder = RequestId::new();
    let answers: [(
        &str,
        Envelope<PrepareDataDto>,
        &'static str,
        &'static [&'static str],
    ); 4] = [
        (
            "blocked",
            prepared(
                &root,
                ResultCode::RecoveryRequired,
                Some(vec![LifecycleBlocker::PrimaryAheadOfRemote]),
                vec![inspect(&root)],
            ),
            "lifecycle-prepare-again",
            &[
                "lifecycle-cancel",
                "lifecycle-check-now",
                "lifecycle-share-main",
                "lifecycle-inspect",
                "lifecycle-prepare-again",
            ],
        ),
        (
            "blocked, ticket_closed",
            prepared(
                &root,
                ResultCode::TicketClosed,
                Some(vec![LifecycleBlocker::AlreadyClosed]),
                vec![],
            ),
            "lifecycle-code",
            &["lifecycle-cancel"],
        ),
        (
            "held",
            prepared(
                &root,
                ResultCode::RecoveryRequired,
                None,
                vec![retry(holder)],
            ),
            "lifecycle-continue",
            &["lifecycle-cancel", "lifecycle-continue"],
        ),
        (
            "answered busy",
            prepared(&root, ResultCode::Busy, None, vec![]),
            "lifecycle-retry",
            &["lifecycle-cancel", "lifecycle-retry"],
        ),
    ];
    for (row, envelope, ready, ids) in answers {
        shell.backend.answer_next_prepare("ticket close", envelope);
        shell.click(LIFECYCLE_ACTION, cx);
        until_present(&shell, ready, cx).await;
        footed(&shell, row, ask(ids, CLOSE, None), cx);
        escape(&shell, row, cx);
    }

    // Ask: opened again once Repair locally… led to a resolution.
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    shell.visual(cx).update(|window, cx| {
        let opening = Opening {
            intent: intent(&root, ItemDtoKind::Ticket),
            heading: Heading {
                kind: ItemDtoKind::Ticket,
                short: short_id(TICKET),
                title: None,
                repository: "a".to_owned(),
                primary: Some("main".to_owned()),
                remote: Some(None),
            },
            origin: cx.focus_handle(),
            saving: None,
            following: None,
            repaired: true,
        };
        super::open(opening, &services, window, cx)
    });
    until_present(&shell, "lifecycle-prepare-again", cx).await;
    footed(
        &shell,
        "repaired",
        ask(
            &["lifecycle-cancel", "lifecycle-prepare-again"],
            CLOSE,
            None,
        ),
        cx,
    );
    escape(&shell, "repaired", cx);

    // Ask: a consent the library refused.
    confirming(&shell, &root, cx).await;
    shell
        .fixture
        .commit_file(&worktree(&shell, &root), "src/late.rs", b"// late\n");
    shell.click("lifecycle-confirm", cx);
    let refused = closes(&shell, cx)[0];
    shell.until_ended(refused, cx).await;
    until_present(&shell, "lifecycle-stale", cx).await;
    footed(
        &shell,
        "stale",
        ask(
            &["lifecycle-cancel", "lifecycle-prepare-again"],
            CANCEL,
            None,
        ),
        cx,
    );
    escape(&shell, "stale", cx);

    // Running, with the preview; Stop pressed; then stopped.
    confirming(&shell, &root, cx).await;
    let origin = pane(&shell, cx).read_with(cx, |pane, _| pane.lifecycle_focus());
    let gate = shell
        .backend
        .hold_at_stage(JobKind::TicketClose, ProgressStage::Integrating);
    shell.click("lifecycle-confirm", cx);
    gate.wait_reached();
    let stopped = closes(&shell, cx)[0];
    at_stage(&shell, stopped, ProgressStage::Integrating, cx).await;
    footed(&shell, "running", running(&["lifecycle-stop"]), cx);
    shell.click("lifecycle-stop", cx);
    footed(&shell, "Stop pressed", running(&["lifecycle-stopping"]), cx);
    for key in ["tab", "tab", "tab", "shift-tab", "shift-tab", "shift-tab"] {
        shell.keys(key, cx);
        assert!(
            shell.holds_focus("lifecycle-dialog", cx),
            "Stop pressed: {key} keeps focus in the dialog"
        );
    }
    shell.keys("escape", cx);
    assert!(
        !shell.present("lifecycle-dialog", cx),
        "Stop pressed: Escape hides it"
    );
    assert!(focused(&shell, &origin, cx), "Stop pressed: on the origin");
    gate.release();
    shell.until_ended(stopped, cx).await;
    assert_eq!(code(&shell, stopped, cx), Some(ResultCode::Cancelled));
    open_entry(&shell, stopped, cx).await;
    footed(
        &shell,
        "stopped",
        Foot {
            stage: Stage::Stopped,
            ids: &["lifecycle-cancel", "lifecycle-retry"],
            dismiss: Some(NOT_NOW),
            hint: None,
        },
        cx,
    );
    escape(&shell, "stopped", cx);

    // O-5 (a), P13's failed run: Retry alone, last. Then Done after Stop:
    // Stop pressed once the library has answered, so the run is done.
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            FailurePoint::BeforeRequestDomainCall,
        )
        .unwrap(),
    );
    open(
        &shell,
        OpenTarget::Item {
            repo: key(&root),
            kind: ItemDtoKind::Document,
            id: item_id(DOCUMENT),
        },
        cx,
    );
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    shell.click("lifecycle-confirm", cx);
    let promotes = |shell: &Shell, cx: &mut TestAppContext| {
        shell.requests(cx, |request| {
            matches!(request.intent, Intent::DocumentPromote { .. })
        })
    };
    let failed = promotes(&shell, cx)[0];
    shell.until_ended(failed, cx).await;
    assert_eq!(code(&shell, failed, cx), Some(ResultCode::InternalError));
    until_present(&shell, "lifecycle-retry", cx).await;
    footed(
        &shell,
        "failed, Retry alone",
        failing(&["lifecycle-cancel", "lifecycle-retry"]),
        cx,
    );
    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());
    let answered_gate = shell.backend.hold_job_return(JobKind::DocumentPromote);
    shell.click("lifecycle-retry", cx);
    answered_gate.wait_reached();
    until_present(&shell, "lifecycle-stop", cx).await;
    shell.click("lifecycle-stop", cx);
    assert!(shell.present("lifecycle-stopping", cx), "Stop pressed");
    answered_gate.release();
    let retried = promotes(&shell, cx)[0];
    shell.until_ended(retried, cx).await;
    until_present(&shell, "lifecycle-ending", cx).await;
    assert!(
        shell.present("lifecycle-stop-late", cx),
        "done after Stop: it finished before it could stop"
    );
    footed(
        &shell,
        "done after Stop",
        Foot {
            stage: Stage::Done,
            ids: &["lifecycle-cancel"],
            dismiss: Some(DONE),
            hint: None,
        },
        cx,
    );
    escape(&shell, "done after Stop", cx);

    // Canned runs of the ticket, shown without their preview.
    let request_id = RequestId::new();
    let ticket_answer = |code, outcome, data, effects, recovery| {
        answer(
            "ticket close",
            &root,
            code,
            outcome,
            data,
            effects,
            recovery,
            request_id,
        )
    };
    let mut merge_refused = short_of_main(
        ItemDtoKind::Ticket,
        TICKET,
        vec![LifecycleBlocker::IntegrationConflict],
    );
    merge_refused.recording = Some(LifecycleRecording::Record);
    let mut unlock = reached(ItemDtoKind::Ticket, TICKET, vec![]);
    unlock.cleanup = Some(LifecycleCleanupState::Pending);
    unlock.cleanup_pending = Some(LifecycleCleanupPendingReason::RemoteDeletionUnlock);
    let mut unknown_ready = reached(ItemDtoKind::Ticket, TICKET, vec![]);
    unknown_ready.made_ready_state = Some(LifecycleMadeReadyState::IndexUnavailableFinal);
    let prepare = RecoveryAction::new(RecoveryActionKind::RequestPrepare, []);
    let rows = [
        (
            "running, no preview",
            Opened::Started,
            running(&["lifecycle-stop"]),
        ),
        (
            "message only",
            Opened::Removed,
            ask(&["lifecycle-cancel"], CLOSE, None),
        ),
        (
            "waiting",
            Opened::Entry(ticket_answer(
                ResultCode::Busy,
                Outcome::Partial,
                short_of_main(ItemDtoKind::Ticket, TICKET, vec![]),
                Effects::not_requested(),
                vec![retry(request_id)],
            )),
            Foot {
                stage: Stage::Waiting,
                ids: &["lifecycle-cancel", "lifecycle-retry"],
                dismiss: Some(CLOSE),
                hint: None,
            },
        ),
        (
            "failed, a refused merge",
            Opened::Entry(ticket_answer(
                ResultCode::RecoveryRequired,
                Outcome::Partial,
                merge_refused,
                Effects::not_requested(),
                vec![retry(request_id), inspect(&root)],
            )),
            failing(&[
                "lifecycle-cancel",
                "lifecycle-repair-locally",
                "lifecycle-inspect",
                "lifecycle-retry",
            ]),
        ),
        (
            "failed, a locked key",
            Opened::Entry(ticket_answer(
                ResultCode::UnlockRequired,
                Outcome::Partial,
                unlock,
                reached_effects(CleanupEffect::Pending),
                vec![retry(request_id)],
            )),
            failing(&[
                "lifecycle-cancel",
                "lifecycle-sharing-and-keys",
                "lifecycle-unlock",
            ]),
        ),
        (
            "failed, a locked key with Prepare again",
            Opened::Entry(ticket_answer(
                ResultCode::UnlockRequired,
                Outcome::Blocked,
                short_of_main(ItemDtoKind::Ticket, TICKET, vec![]),
                Effects::not_requested(),
                vec![prepare],
            )),
            failing(&[
                "lifecycle-cancel",
                "lifecycle-sharing-and-keys",
                "lifecycle-prepare-again",
                "lifecycle-unlock",
            ]),
        ),
        (
            "done, Open ready tickets",
            Opened::Direct(ticket_answer(
                ResultCode::Ok,
                Outcome::Success,
                unknown_ready,
                reached_effects(CleanupEffect::Complete),
                vec![],
            )),
            Foot {
                stage: Stage::Done,
                ids: &["lifecycle-open-ready", "lifecycle-cancel"],
                dismiss: Some(DONE),
                hint: None,
            },
        ),
    ];
    for (n, (row, run, foot)) in rows.into_iter().enumerate() {
        let n = 300 + n as u64;
        let ticket_close = intent(&root, ItemDtoKind::Ticket);
        match run {
            Opened::Started => {
                let id = fabricate(
                    &shell,
                    n,
                    ticket_close,
                    |id| vec![WorkerEvent::Started(id)],
                    cx,
                );
                open_on(&shell, id, cx);
            }
            Opened::Removed => {
                let id = fabricate(
                    &shell,
                    n,
                    ticket_close,
                    |id| vec![WorkerEvent::Removed(id)],
                    cx,
                );
                open_on(&shell, id, cx);
            }
            Opened::Entry(envelope) => {
                let id = finished(&shell, n, ticket_close, request_id, envelope, cx);
                open_entry(&shell, id, cx).await;
            }
            Opened::Direct(envelope) => {
                let id = finished(&shell, n, ticket_close, request_id, envelope, cx);
                open_on(&shell, id, cx);
            }
        }
        until_present(&shell, "lifecycle-title", cx).await;
        footed(&shell, row, foot, cx);
        escape(&shell, row, cx);
    }
}

/// Clicks the backdrop to the left of the dialog, at its middle height:
/// outside the dialog and below the title bar (design, Testing).
fn click_backdrop(shell: &Shell, cx: &mut TestAppContext) {
    let at = shell.frame(cx, |window, _| {
        let dialog = window.find("lifecycle-dialog").bounds();
        assert!(dialog.left() > px(40.), "backdrop left of {dialog:?}");
        point(dialog.left() - px(40.), dialog.center().y)
    });
    let mut visual = shell.visual(cx);
    visual.simulate_click(at, Modifiers::default());
    visual.run_until_parked();
}

/// P3, A5, R-5: a click on the backdrop hides a running dialog as Escape
/// does; the run goes on, and focus is back on the origin. On confirm, on
/// Waiting and on a failure it does nothing.
#[gpui_kit::test]
async fn the_backdrop_hides_only_a_running_dialog(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Backdrop", "")
    });
    confirming(&shell, &root, cx).await;
    click_backdrop(&shell, cx);
    assert!(shell.present("lifecycle-confirm", cx), "confirm: kept");
    escape(&shell, "confirm", cx);

    let rows = [
        ("waiting", ResultCode::Busy, Outcome::Partial),
        ("failed", ResultCode::RecoveryRequired, Outcome::Blocked),
    ];
    for (n, (row, code, outcome)) in rows.into_iter().enumerate() {
        let request = RequestId::new();
        let envelope = answered(&root, ItemDtoKind::Ticket, code, outcome, request);
        let id = finished(
            &shell,
            400 + n as u64,
            intent(&root, ItemDtoKind::Ticket),
            request,
            envelope,
            cx,
        );
        open_entry(&shell, id, cx).await;
        click_backdrop(&shell, cx);
        assert!(shell.present("lifecycle-ending", cx), "{row}: kept");
        escape(&shell, row, cx);
    }

    confirming(&shell, &root, cx).await;
    let origin = pane(&shell, cx).read_with(cx, |pane, _| pane.lifecycle_focus());
    let gate = shell
        .backend
        .hold_at_stage(JobKind::TicketClose, ProgressStage::Integrating);
    shell.click("lifecycle-confirm", cx);
    gate.wait_reached();
    let id = closes(&shell, cx)[0];
    at_stage(&shell, id, ProgressStage::Integrating, cx).await;
    click_backdrop(&shell, cx);
    assert!(!shell.present("lifecycle-dialog", cx), "running: hidden");
    assert!(focused(&shell, &origin, cx), "running: on the origin");
    assert!(
        matches!(request(&shell, id, cx).state, RequestState::Running { .. }),
        "running: the run goes on"
    );
    gate.release();
    shell.until_ended(id, cx).await;
    assert_eq!(code(&shell, id, cx), Some(ResultCode::Ok));
}
