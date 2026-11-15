//! P1 (Cycle 05b, A1, A2): the lifecycle dialog's stage, and the title and
//! subtitle its header shows there, for both commands. A real ticket and
//! document give the stages that hold the preview (confirm, running, done,
//! stopped), where the Git line is drawn too (A5, A6); canned requests give
//! the stages of a run shown without it (reopened from Activity, K-15), and
//! the message-only ones that take the Ask title (K-10).

use std::path::Path;

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{ItemDtoKind, OperationId, ProgressStage, RequestId},
    results::{CleanupEffect, Effects, Envelope, Outcome, ResultCode},
};

use super::{
    CLOSED, CLOSING, COULDNT_CLOSE, COULDNT_MERGE, Heading, MERGING, STILL_RUNNING, approve_title,
    close_title,
    outcome::{ENDING_STOPPED, merged},
    outcome_canned_tests::{
        answer, finished, intent, reached, reached_effects, reopen as open_entry, retry,
        short_of_main,
    },
    outcome_tests::{at_stage, stopped_at_integrating},
    privacy_promote_tests::saved_document,
    privacy_tests::TITLE,
    short_id,
    stages::Stage,
    tests::{DOCUMENT, SLUG, TICKET, dialog, open, saved_ticket, shell, shown, until_present},
};
use crate::desktop::{
    model::navigation::OpenTarget,
    services::{
        JobKind,
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
