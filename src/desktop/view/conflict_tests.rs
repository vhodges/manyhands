//! The resolve conflict sheet (D5 design, B3): its parts and progress,
//! Save resolution and its keys, what it submits, and how it reads again,
//! over real local conflicts (`Fixture::conflict_locally`). A remote
//! conflict is canned (`answer_next_conflict_read`; decision 9): the SSH
//! harness does not reach desktop tests, so the library's real
//! `show_conflict` runs against an operation the repository does not hold,
//! and a conflict built from library types, of the shape the cited library
//! test reads, stands in place of its answer. Nothing is asserted of the
//! repository in a canned test. How the sheet follows an answer is
//! `conflict/answer_tests.rs`, and its note and an unkept set
//! `conflict/sheet_tests.rs`; both share these helpers.
//!
//! Each test waits on the worker, so it allows parking. A real resolution
//! may take over twenty seconds under the whole suite: those tests wait up
//! to `RESOLVING`.

use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use gpui_kit::{Entity, Focusable, TestAppContext};
use manyhands::{
    repository::{
        ConflictDto, ConflictPathDto, ConflictPathKind, ConflictPathStructure,
        ConflictReadEligibility, ConflictReadNextAction, ConflictStage, ConflictState,
        ConflictTarget, ItemDtoKind, OperationId,
    },
    results::ResultCode,
};

use super::{
    KEEPING_THEIRS, OpenedSheet, Opening, ResolveSheet, SAVE_RESOLUTION, SheetSource, TAKEN_THEIRS,
    open,
};
use crate::desktop::{
    model::conflict::STANDARD_LAYOUT_NOTE,
    services::{
        Services,
        requests::{DesktopRequest, DesktopRequestId, Intent, RequestState},
    },
    test_support::fixtures::{Fixture, item_id, ticket_path, ticket_source},
    view::{harness::Shell, lifecycle::Heading},
};

pub(crate) const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG0";
pub(super) const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FH0";

/// How long a real resolution may take under the whole suite, as
/// `services/resolutions_tests.rs`'s `RESOLVING`.
pub(super) const RESOLVING: Duration = Duration::from_secs(90);

/// One repository, `a`, in an open window, with what `build` made in it
/// before the window opened: a real local conflict's operation.
pub(super) fn local_shell(
    cx: &mut TestAppContext,
    build: impl FnOnce(&Fixture, &Path) -> OperationId,
) -> (Shell, PathBuf, OperationId) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let operation = build(&fixture, &root);
    (Shell::open(fixture, cx), root, operation)
}

/// A repository with no conflict of its own, for a canned one.
pub(super) fn remote_shell(cx: &mut TestAppContext) -> (Shell, PathBuf) {
    let (shell, root, _) = local_shell(cx, |_, _| OperationId::new());
    (shell, root)
}

pub(super) fn services(shell: &Shell, cx: &mut TestAppContext) -> Services {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone())
}

/// Opens the sheet on `operation` of `root`, from `source`.
pub(super) fn open_sheet(
    shell: &Shell,
    source: SheetSource,
    root: &Path,
    operation: OperationId,
    cx: &mut TestAppContext,
) -> Entity<ResolveSheet> {
    open_sheet_in(shell, source, root, operation, None, cx)
}

/// Opens the sheet as `open_sheet` does, naming `folder` as the item's
/// work folder (B4's Open folder).
pub(super) fn open_sheet_in(
    shell: &Shell,
    source: SheetSource,
    root: &Path,
    operation: OperationId,
    folder: Option<PathBuf>,
    cx: &mut TestAppContext,
) -> Entity<ResolveSheet> {
    let services = services(shell, cx);
    let opening = Opening {
        source,
        root: root.to_owned(),
        operation_id: operation,
        item: item_id(TICKET),
        heading: heading(),
        folder,
        origin: cx.update(|cx| cx.focus_handle()),
    };
    let sheet = shell
        .visual(cx)
        .update(|window, cx| open(opening, &services, window, cx))
        .expect("the sheet opens");
    shell.run(cx);
    sheet
}

pub(super) const SHORT: &str = "ae-c10s3";

/// Who the sheet speaks of: the ticket, its repository `a`, and the main
/// branch and remote a reopened lifecycle dialog shows.
pub(super) fn heading() -> Heading {
    Heading {
        kind: ItemDtoKind::Ticket,
        short: SHORT.to_owned(),
        title: Some("Mine".to_owned()),
        repository: "a".to_owned(),
        branch: None,
        primary: Some("main".to_owned()),
        remote: Some(None),
    }
}

/// The sheet last opened, while it is open.
pub(super) fn opened(cx: &mut TestAppContext) -> Option<Entity<ResolveSheet>> {
    cx.update(|cx| {
        cx.try_global::<OpenedSheet>()
            .and_then(|opened| opened.0.upgrade())
    })
}

/// Draws frames until `id` is drawn.
pub(super) async fn until_present(
    shell: &Shell,
    id: impl Into<gpui_kit::ElementId>,
    cx: &mut TestAppContext,
) {
    let id = id.into();
    shell
        .until_drawn(cx, "the awaited element", |shell, cx| {
            shell.present(id.clone(), cx)
        })
        .await;
}

/// Waits, past the harness's `WAIT`, until `done` holds of the window.
pub(super) async fn until_long(
    shell: &Shell,
    what: &str,
    cx: &mut TestAppContext,
    done: impl Fn(&Shell, &mut TestAppContext) -> bool,
) {
    let deadline = Instant::now() + RESOLVING;
    loop {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
        if done(shell, cx) {
            return;
        }
        assert!(Instant::now() < deadline, "{what} did not happen");
        cx.executor().timer(Duration::from_millis(10)).await;
    }
}

pub(super) fn is_resolve(request: &DesktopRequest) -> bool {
    matches!(
        request.intent,
        Intent::ConflictResolve { .. } | Intent::LocalContextResolve { .. }
    )
}

pub(super) fn resolves(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, is_resolve)
}

pub(super) fn request(
    shell: &Shell,
    id: DesktopRequestId,
    cx: &mut TestAppContext,
) -> DesktopRequest {
    shell
        .operations(cx)
        .read_with(cx, |operations, _| operations.request(id).cloned())
        .expect("the request is listed")
}

/// The request's code, once it has its answer.
pub(super) fn code(
    shell: &Shell,
    id: DesktopRequestId,
    cx: &mut TestAppContext,
) -> Option<ResultCode> {
    match request(shell, id, cx).state {
        RequestState::Finished(_, code, ..) => Some(code),
        _ => None,
    }
}

/// The paths a resolve request submits: each token and its result.
pub(super) fn submitted(intent: &Intent) -> (String, Vec<(String, String)>) {
    match intent {
        Intent::ConflictResolve {
            observation,
            resolutions,
            ..
        }
        | Intent::LocalContextResolve {
            observation,
            resolutions,
            ..
        } => (
            observation.clone(),
            resolutions
                .iter()
                .map(|path| (path.token.clone(), path.source.clone()))
                .collect(),
        ),
        _ => panic!("a resolve request"),
    }
}

/// A canned remote conflict of `operation` over `paths`, live and to be
/// resolved, of the shape `tests/mutation_remote/conflict_read.rs`
/// `c4_conflict_read_canonical` reads.
pub(crate) fn canned(operation: OperationId, paths: Vec<ConflictPathDto>) -> ConflictDto {
    ConflictDto {
        operation_id: operation.to_string(),
        comment_operation_id: None,
        target: ConflictTarget::Primary,
        item_id: None,
        stage: ConflictStage::Primary,
        state: ConflictState::ConflictPending,
        next_action: ConflictReadNextAction::Resolve,
        request_id: None,
        eligibility: Some(ConflictReadEligibility::EligibleCanonical),
        local_parent: "1".repeat(40),
        incoming_parent: "2".repeat(40),
        observation: Some(format!("conflict-v1:{}", "0".repeat(64))),
        historical: false,
        paths,
    }
}

/// A ticket's path of a canned conflict whose title both sides changed,
/// and nothing else.
pub(super) fn canned_path(id: &str, n: char) -> ConflictPathDto {
    let source = |title: &str| same_body(&ticket_source(id, title, ""), title);
    ConflictPathDto {
        display_path: ticket_path(id),
        token: format!("conflict-path-v1:{}", n.to_string().repeat(64)),
        kind: ConflictPathKind::Ticket,
        structure: ConflictPathStructure::Regular,
        content_available: true,
        base: Some(source("Base")),
        local: Some(source("Local")),
        incoming: Some(source("Incoming")),
    }
}

/// `source`, the ticket titled `title`, with the body every side shares.
pub(super) fn same_body(source: &str, title: &str) -> String {
    source.replace(&format!("Body of {title}."), "Body.")
}

/// B3, R-5, real local: progress counts the parts, and Save resolution is
/// disabled until each is chosen: Tab, which a disabled Kit button does not
/// take, reaches it only then, and a click before sends nothing. The body, which only theirs changed, is
/// taken from theirs and says so; a chosen merge is saved in the standard
/// layout (R-9).
#[gpui_kit::test]
async fn save_is_disabled_until_every_part_is_chosen(cx: &mut TestAppContext) {
    let (shell, root, operation) = local_shell(cx, |fixture, root| {
        fixture.conflict_locally(root, TICKET, "Mine", "Theirs")
    });
    open_sheet(&shell, SheetSource::Local, &root, operation, cx);
    until_present(&shell, "conflict-save", cx).await;

    assert_eq!(
        shell.label("conflict-progress", cx).as_deref(),
        Some("0 of 1 chosen")
    );
    assert_eq!(shell.rows("conflict-field", cx), ["title"]);
    assert_eq!(
        shell.label(("conflict-body-taken", 0usize), cx),
        Some(format!("Description: {TAKEN_THEIRS}"))
    );
    assert!(shell.inert_button("conflict-save", cx));
    assert!(!shell.tab_reaches("conflict-save", 24, cx), "disabled");
    shell.click("conflict-save", cx);
    assert!(resolves(&shell, cx).is_empty(), "nothing is saved");

    shell.click(("conflict-keep-theirs", 0usize), cx);
    assert_eq!(
        shell.label("conflict-progress", cx).as_deref(),
        Some("1 of 1 chosen")
    );
    assert_eq!(
        shell.label(("conflict-choice", 0usize), cx).as_deref(),
        Some(KEEPING_THEIRS)
    );
    assert_eq!(
        shell.label("conflict-save", cx).as_deref(),
        Some(SAVE_RESOLUTION)
    );
    assert!(shell.tab_reaches("conflict-save", 24, cx), "enabled");
    assert_eq!(
        shell.label(("conflict-layout", 0usize), cx).as_deref(),
        Some(STANDARD_LAYOUT_NOTE)
    );
}

/// B3, real local: Enter in the Result editor is a newline and saves
/// nothing; Ctrl+Enter saves, with the edited result.
#[gpui_kit::test]
async fn ctrl_enter_saves_and_enter_is_a_newline(cx: &mut TestAppContext) {
    let (shell, root, operation) = local_shell(cx, |fixture, root| {
        fixture.conflict_locally_with_body(root, TICKET, "Mine", Some("Mine body.\n"), "Theirs")
    });
    let sheet = open_sheet(&shell, SheetSource::Local, &root, operation, cx);
    until_present(&shell, "conflict-save", cx).await;
    assert_eq!(
        shell.label("conflict-progress", cx).as_deref(),
        Some("0 of 2 chosen")
    );
    shell.click(("conflict-keep-mine", 0usize), cx);
    shell.click(("conflict-body-both", 0usize), cx);
    assert_eq!(
        shell.label("conflict-progress", cx).as_deref(),
        Some("2 of 2 chosen")
    );

    let editor = sheet
        .read_with(cx, |sheet, _| sheet.result_editor(0))
        .expect("the Result editor");
    let end = editor.read_with(cx, |editor, _| {
        let text = editor.text();
        text.find("Body of Theirs.").expect("theirs is kept too") + "Body of Theirs.".len()
    });
    editor.update(cx, |editor, cx| editor.set_cursor(end, cx));
    let handle = editor.read_with(cx, |editor, cx| editor.focus_handle(cx));
    shell.frame(cx, |window, cx| window.focus(&handle, cx));
    shell.frame(cx, |_, _| ());
    shell.press("enter", cx);
    shell.visual(cx).simulate_input("Added");
    shell.run(cx);
    assert!(resolves(&shell, cx).is_empty(), "Enter saves nothing");

    shell.keys("ctrl-enter", cx);
    let sent = resolves(&shell, cx);
    assert_eq!(sent.len(), 1, "Ctrl+Enter saves");
    let (_, paths) = submitted(&request(&shell, sent[0], cx).intent);
    assert!(
        paths[0].1.contains("Body of Theirs.\nAdded"),
        "the edited result, with its newline"
    );
    until_long(&shell, "the resolution's answer", cx, |shell, cx| {
        code(shell, sent[0], cx).is_some()
    })
    .await;
    assert_eq!(code(&shell, sent[0], cx), Some(ResultCode::Ok));
}

/// B3, canned (decision 9): Save resolution sends `conflict resolve` for
/// the operation, with the read's observation and every path's token and
/// result: mine's bytes where every part is mine (R-9), the standard
/// layout otherwise. The real call is refused, as the repository holds no
/// such operation; the conflict is the shape `c4_conflict_read_canonical`
/// reads, with a second path.
#[gpui_kit::test]
async fn save_submits_every_path_with_the_observation(cx: &mut TestAppContext) {
    let (shell, root) = remote_shell(cx);
    let operation = OperationId::new();
    let conflict = canned(
        operation,
        vec![canned_path(TICKET, '1'), canned_path(OTHER, '2')],
    );
    let observation = conflict.observation.clone().unwrap();
    shell.backend.answer_next_conflict_read(conflict);
    open_sheet(&shell, SheetSource::Share, &root, operation, cx);
    until_present(&shell, "conflict-save", cx).await;
    assert_eq!(
        shell.label("conflict-progress", cx).as_deref(),
        Some("0 of 2 chosen")
    );
    shell.click(("conflict-keep-mine", 0usize), cx);
    shell.click(("conflict-keep-theirs", 1usize), cx);
    shell.click("conflict-save", cx);

    let sent = resolves(&shell, cx);
    assert_eq!(sent.len(), 1);
    let sent = request(&shell, sent[0], cx);
    let Intent::ConflictResolve {
        root: sent_root,
        operation_id,
        ..
    } = &sent.intent
    else {
        panic!("a remote conflict is resolved with `conflict resolve`");
    };
    assert_eq!(sent_root, &root);
    assert_eq!(*operation_id, operation);
    let (sent_observation, paths) = submitted(&sent.intent);
    assert_eq!(sent_observation, observation);
    assert_eq!(
        paths
            .iter()
            .map(|(token, _)| token.clone())
            .collect::<Vec<_>>(),
        [
            format!("conflict-path-v1:{}", "1".repeat(64)),
            format!("conflict-path-v1:{}", "2".repeat(64)),
        ]
    );
    assert_eq!(
        paths[0].1,
        same_body(&ticket_source(TICKET, "Local", ""), "Local"),
        "mine's bytes"
    );
    assert!(paths[1].1.contains("title: Incoming"), "theirs chosen");
    shell.until_ended(sent.id, cx).await;
}

/// R-11, real local: a local resolution presents the inspection's
/// observation, and applies; the sheet then closes.
#[gpui_kit::test]
async fn a_local_resolution_uses_the_inspections_observation(cx: &mut TestAppContext) {
    let (shell, root, operation) = local_shell(cx, |fixture, root| {
        fixture.conflict_locally(root, TICKET, "Mine", "Theirs")
    });
    let (inspection, _) = shell.fixture.read_local_conflict(&root, operation);
    open_sheet(&shell, SheetSource::Local, &root, operation, cx);
    until_present(&shell, "conflict-save", cx).await;
    shell.click(("conflict-keep-mine", 0usize), cx);
    shell.click("conflict-save", cx);

    let sent = resolves(&shell, cx);
    assert_eq!(sent.len(), 1);
    let intent = request(&shell, sent[0], cx).intent;
    assert!(
        matches!(intent, Intent::LocalContextResolve { operation_id, .. } if operation_id == operation)
    );
    assert_eq!(Some(submitted(&intent).0), inspection.observation);
    until_long(&shell, "the resolution's answer", cx, |shell, cx| {
        code(shell, sent[0], cx).is_some()
    })
    .await;
    assert_eq!(code(&shell, sent[0], cx), Some(ResultCode::Ok));
    until_long(&shell, "the sheet's closing", cx, |shell, cx| {
        !shell.present("conflict-sheet", cx)
    })
    .await;
}
