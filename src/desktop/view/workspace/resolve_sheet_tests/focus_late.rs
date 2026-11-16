//! P6's rows H9, H10 and H12 to H15 (Wave 04 Cycle 05b design B3): Tab
//! kept inside the sheet; focus taken behind it brought back; an answer
//! that lands after the sheet closed, or under a dialog over it; the
//! Result editor's own right-click menu; and the workspace's own focus moves while
//! it is open. H11, quitting, is `quitting`. Each read is after two
//! draws; the Kit's deferred restore is reached by the test clock, with no
//! real wait.

use std::{path::PathBuf, time::Duration};

use gpui_kit::{Entity, TestAppContext};
use manyhands::{
    drafts::DraftId,
    repository::{OperationId, RequestId},
};

use super::{
    focus::resolutions_end,
    support::{
        Rig, Table, all_closed, canonical, close_everything, focus, focused_id, has_dialog,
        in_context, into_the_editor, on, on_handle, on_own, open_probe, opening, owner,
        resolve_open, route, settings_open, shown, with_body,
    },
};
use crate::desktop::{
    services::{
        JobKind,
        requests::{DesktopRequestId, Intent, Lane},
    },
    test_support::fixtures::item_id,
    view::{
        activity_support::{act, list_focus, until_offered},
        conflict::{
            REVIEW_CHANGES, ResolveSheet, SheetSource,
            answer_tests::applied,
            tests::{TICKET, local_shell, opened, until_present},
        },
        harness::Shell,
        lifecycle::OpenedLifecycle,
        settings::Shown,
        workspace::Region,
    },
};

pub(super) async fn rows(table: &mut Table, cx: &mut TestAppContext) {
    h9(table, cx).await;
    h10(table, cx).await;
    super::quitting::h11(table, cx).await;
    #[cfg(unix)]
    super::quitting::h11_unkept(table, cx).await;
    h12(table, cx).await;
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    h13(table, cx).await;
    h14(table, cx).await;
    h14_stale(table, cx).await;
    h15(table, cx).await;
}

/// A repository with no conflict of its own, and a close this session
/// sent and holds, refused for its missing confirmation, which leaves no
/// record. Gives the window, the repository and the close.
pub(super) async fn close_shell(cx: &mut TestAppContext) -> (Shell, PathBuf, DesktopRequestId) {
    let (shell, root, _) = local_shell(cx, |_, _| OperationId::new());
    let close = shell.workspace.read_with(cx, |workspace, _| {
        workspace.services().worker().submit(
            Intent::TicketClose {
                root: root.clone(),
                id: item_id(TICKET),
            },
            Lane::User,
            None,
            None,
        )
    });
    shell.until_ended(close, cx).await;
    (shell, root, close)
}

/// The sheet on the close's child conflict, canned, as the close's way in
/// opens it (`SheetSource::Close`), once it has read.
pub(super) async fn open_close_child(
    shell: &Shell,
    root: &std::path::Path,
    close: DesktopRequestId,
    cx: &mut TestAppContext,
) -> Option<Entity<ResolveSheet>> {
    shell.backend.answer_next_conflict_read(canonical());
    let origin = cx.update(|cx| cx.focus_handle());
    let sheet = route(shell, cx, |workspace, window, cx| {
        workspace.show_resolve(
            opening(SheetSource::Close(Some(close)), root, TICKET, origin),
            window,
            cx,
        )
    })?;
    until_present(shell, "conflict-save", cx).await;
    Some(sheet)
}

pub(super) async fn until_attempt(
    shell: &Shell,
    id: DesktopRequestId,
    attempt: u32,
    cx: &mut TestAppContext,
) {
    shell
        .until(cx, move |operations| {
            operations
                .request(id)
                .is_some_and(|request| request.attempt == attempt)
        })
        .await;
}

/// Waits, a bounded while, until a lifecycle dialog is open; false when
/// none opened.
pub(super) async fn until_lifecycle(shell: &Shell, cx: &mut TestAppContext) -> bool {
    for _ in 0..200 {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
        if lifecycle_open(cx) {
            return true;
        }
        cx.executor().timer(Duration::from_millis(10)).await;
    }
    false
}

fn lifecycle_open(cx: &mut TestAppContext) -> bool {
    lifecycle_id(cx).is_some()
}

/// The lifecycle dialog last opened, while it is open. The global is the
/// test app's, so it may name another window's: a row compares it before
/// and after rather than reading it alone.
fn lifecycle_id(cx: &mut TestAppContext) -> Option<gpui_kit::EntityId> {
    cx.update(|cx| {
        cx.try_global::<OpenedLifecycle>()
            .and_then(|opened| opened.0.upgrade())
            .map(|dialog| dialog.entity_id())
    })
}

/// Whether focus is inside the open lifecycle dialog, after two draws: in
/// its own view, or on the Kit's dialog around it, which holds focus as the
/// dialog opens (`LifecycleDialog::focus_inside`, R-24).
pub(super) fn in_lifecycle(shell: &Shell, cx: &mut TestAppContext) -> bool {
    super::support::settle(shell, cx);
    let Some(dialog) = cx.update(|cx| {
        cx.try_global::<OpenedLifecycle>()
            .and_then(|opened| opened.0.upgrade())
    }) else {
        return false;
    };
    let handle = dialog.read_with(cx, |dialog, _| dialog.focus().clone());
    shell.frame(cx, |window, cx| {
        handle.contains_focused(window, cx) || handle.within_focused(window, cx)
    })
}

/// H9: Tab and Shift+Tab, pressed again and again from the sheet's own
/// handle, never leave `sheet-host` (focus stays in its key context,
/// `Sheet`), and reach the Kit's close button, the body's choices and the
/// footer's Cancel.
async fn h9(table: &mut Table, cx: &mut TestAppContext) {
    let rig = Rig::new(cx);
    let shell = &rig.shell;
    let sheet = rig.by_banner(Some(canonical()), "conflict-save", cx).await;
    for key in ["tab", "shift-tab"] {
        focus(shell, &super::support::own_focus(&sheet, cx), cx);
        let mut inside = true;
        let mut seen = Vec::new();
        for _ in 0..10 {
            shell.keys(key, cx);
            inside &= in_context(shell, "Sheet", cx);
            seen.push(focused_id(shell, cx));
        }
        let names: Vec<String> = seen.iter().map(|id| format!("{id:?}")).collect();
        let reached = |name: &str| names.iter().any(|seen| seen.contains(name));
        let held = inside
            && reached("\"close\"")
            && reached("conflict-keep-")
            && reached("conflict-cancel");
        table.row(
            shell,
            &format!("H9 {key}, again and again (it reached {names:?})"),
            held,
            "inside sheet-host: the close button, the choices, Cancel",
            cx,
        );
    }
}

/// H10: something behind takes focus while the sheet is open (Activity's
/// list), and the Kit's deferred restore of a dialog closed by Enter just
/// before the sheet opened, its 250 ms passed on the test clock: focus is
/// back on the sheet's own handle.
async fn h10(table: &mut Table, cx: &mut TestAppContext) {
    let rig = Rig::new(cx);
    let shell = &rig.shell;
    let sheet = rig.by_banner(Some(canonical()), "conflict-save", cx).await;
    let list = list_focus(shell, cx);
    focus(shell, &list, cx);
    let held = on_own(shell, &sheet, cx);
    table.row(
        shell,
        "H10 something behind takes focus",
        held,
        "the sheet's own handle",
        cx,
    );
    close_everything(shell, cx);

    let entry = rig.entry();
    until_offered(shell, &entry, REVIEW_CHANGES, cx).await;
    focus(shell, &list, cx);
    shell.visual(cx).update(open_probe);
    shell.run(cx);
    shell.press("enter", cx);
    let deferred = !has_dialog(shell, cx);
    table.fact(
        "H10 setup: the probe closed by Enter",
        deferred,
        "no dialog open",
    );
    shell.backend.answer_next_conflict_read(canonical());
    act(shell, &entry, REVIEW_CHANGES, cx);
    until_present(shell, "conflict-save", cx).await;
    let sheet = opened(cx).expect("the sheet is open");
    let before = on_own(shell, &sheet, cx);
    table.row(
        shell,
        "H10 setup: the sheet's own handle before the restore is due",
        before,
        "the sheet's own handle",
        cx,
    );
    cx.executor().advance_clock(Duration::from_millis(250));
    shell.run(cx);
    let held = on_own(shell, &sheet, cx);
    table.row(
        shell,
        "H10 the Kit's deferred restore of a dialog closed just before it opened",
        held,
        "the sheet's own handle, not Activity's list",
        cx,
    );
}

/// H12: a save's answer lands after Escape closed the sheet, the test
/// holding its entity, and after Settings opened in its place: nothing
/// moves, no sheet closes, no lifecycle dialog opens again (R-22, R-27). A
/// lifecycle child's, so an applied resolution would reopen its dialog.
async fn h12(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, root, close) = close_shell(cx).await;
    let Some(sheet) = open_close_child(&shell, &root, close, cx).await else {
        table.fact("H12", false, "the sheet opened");
        return;
    };
    shell.click(("conflict-keep-mine", 0usize), cx);
    let held = shell
        .backend
        .hold_job_return_answering(JobKind::ConflictResolve, applied());
    shell.click("conflict-save", cx);
    shell.keys("escape", cx);
    shell.keys("secondary-,", cx);
    let settings = settings_open(&shell, cx);
    table.fact(
        "H12 setup: Escape, then Settings in its place",
        settings,
        "Settings open",
    );
    super::support::settle(&shell, cx);
    let before = shell.frame(cx, |window, cx| window.focused(cx));
    let dialog_before = lifecycle_id(cx);
    held.release();
    resolutions_end(&shell, cx).await;
    until_attempt(&shell, close, 2, cx).await;
    for _ in 0..3 {
        super::support::settle(&shell, cx);
    }
    let now = shell.frame(cx, |window, cx| window.focused(cx));
    let quiet =
        settings_open(&shell, cx) && !has_dialog(&shell, cx) && lifecycle_id(cx) == dialog_before;
    table.fact(
        "H12 an answer after the sheet closed: no sheet closes, no dialog reopens",
        quiet,
        "Settings still open, no lifecycle dialog",
    );
    table.row(
        &shell,
        "H12 an answer after the sheet closed: nothing moves",
        now.is_some() && now == before,
        "where focus was in Settings",
        cx,
    );
    drop(sheet);
    shell.until_ended(close, cx).await;
}

/// H13 (owner ruling (a), 2026-11-15): the Result editor's own right-click
/// menu, not Root's (zorite draws it inside the editor, with no focus of
/// its own): focus stays in the editor; then Escape goes to the Result
/// stop (H3), the sheet open. The editor's own element has no ID, so the
/// stop is read as not the editor.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
async fn h13(table: &mut Table, cx: &mut TestAppContext) {
    use gpui_kit::test::TestWindowExt;

    let rig = Rig::new(cx);
    let shell = &rig.shell;
    let sheet = rig.by_banner(Some(with_body()), "conflict-save", cx).await;
    let editor = into_the_editor(shell, &sheet, cx);
    shell.frame(cx, |window, cx| {
        window.right_click(("conflict-result", 0usize), cx)
    });
    shell.run(cx);
    let held = on_handle(shell, &editor, cx) && resolve_open(shell, cx);
    table.row(
        shell,
        "H13 the Result editor's own right-click menu",
        held,
        "the Result editor, the sheet open",
        cx,
    );
    shell.keys("escape", cx);
    let held = on(shell, ("conflict-result", 0usize), cx)
        && !on_handle(shell, &editor, cx)
        && resolve_open(shell, cx);
    table.row(
        shell,
        "H13 then Escape",
        held,
        "the Result stop (H3), the sheet open",
        cx,
    );
}

/// H14: a save's answer lands while a dialog (an Unlock key prompt) is
/// over the sheet: nothing happens until that dialog closes; then H5, the
/// sheet closed and focus on the origin (R-26). Opened from Activity's
/// row, whose list, the origin, stays drawn: the applied resolution clears
/// the banner.
async fn h14(table: &mut Table, cx: &mut TestAppContext) {
    let rig = Rig::new(cx);
    let shell = &rig.shell;
    let entry = rig.entry();
    until_offered(shell, &entry, REVIEW_CHANGES, cx).await;
    shell.backend.answer_next_conflict_read(canonical());
    act(shell, &entry, REVIEW_CHANGES, cx);
    until_present(shell, "conflict-save", cx).await;
    shell.click(("conflict-keep-mine", 0usize), cx);
    let held = shell
        .backend
        .hold_job_return_answering(JobKind::ConflictResolve, applied());
    shell.click("conflict-save", cx);
    let _answer = shell.prompt(cx);
    let prompted = has_dialog(shell, cx) && shell.prompt_focused(cx);
    table.fact(
        "H14 setup: the prompt over the sheet",
        prompted,
        "the prompt open and focused",
    );
    held.release();
    resolutions_end(shell, cx).await;
    super::support::settle(shell, cx);
    let waits = resolve_open(shell, cx) && has_dialog(shell, cx) && shell.prompt_focused(cx);
    table.row(
        shell,
        "H14 the answer under the dialog: nothing yet",
        waits,
        "the prompt, the sheet still open under it",
        cx,
    );
    shell.keys("escape", cx);
    let list = list_focus(shell, cx);
    let held = all_closed(shell, cx) && on_handle(shell, &list, cx);
    table.row(
        shell,
        "H14 once the dialog closes: H5",
        held,
        "the origin, Activity's list, the sheet closed",
        cx,
    );
}

/// H14, a stale sheet: a sheet Escape closed, the test holding its entity,
/// hears an applied answer under a prompt after the sheet open now has
/// waited under it. The stale answer does not replace the open sheet's
/// waiting close: once the prompt closes, the open sheet closes and its
/// lifecycle dialog reopens (R-26).
async fn h14_stale(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, root, close) = close_shell(cx).await;
    let Some(stale) = open_close_child(&shell, &root, close, cx).await else {
        table.fact("H14 stale", false, "the first sheet opened");
        return;
    };
    shell.keys("escape", cx);
    let Some(_open) = open_close_child(&shell, &root, close, cx).await else {
        table.fact("H14 stale", false, "the second sheet opened");
        return;
    };
    shell.click(("conflict-keep-mine", 0usize), cx);
    let held = shell
        .backend
        .hold_job_return_answering(JobKind::ConflictResolve, applied());
    shell.click("conflict-save", cx);
    let _answer = shell.prompt(cx);
    held.release();
    resolutions_end(&shell, cx).await;
    super::support::settle(&shell, cx);
    let waits = resolve_open(&shell, cx) && has_dialog(&shell, cx);
    table.fact(
        "H14 stale setup: the open sheet's answer waits under the prompt",
        waits,
        "the prompt, the open sheet still open under it",
    );
    shell.frame(cx, |window, cx| {
        stale.update(cx, |sheet, cx| sheet.applied(RequestId::new(), window, cx))
    });
    shell.keys("escape", cx);
    for _ in 0..3 {
        super::support::settle(&shell, cx);
    }
    table.fact(
        "H14 a stale sheet's answer under the prompt leaves the open sheet's waiting close",
        !resolve_open(&shell, cx),
        "the open sheet closed once the prompt closed",
    );
}

/// H15: focus moves the workspace makes while the sheet is open, typing in
/// the Result editor: a draft discarded behind it (`draft_discarded`, the
/// next frame's refocus) and a tab closed with its draft behind it
/// (`focus_region(Content)`, as `close_with_draft` ends): focus stays in
/// the editor. R-25: Drafts and the identity form asked for while it is
/// open (`review_in_drafts`, `settings_for_initials`) wait, focus still in
/// the editor, and open once it closes.
async fn h15(table: &mut Table, cx: &mut TestAppContext) {
    let rig = Rig::new(cx);
    let shell = &rig.shell;
    let sheet = rig.by_banner(Some(with_body()), "conflict-save", cx).await;
    let editor = into_the_editor(shell, &sheet, cx);
    shell.visual(cx).simulate_input("x");
    shell.run(cx);

    route(shell, cx, |workspace, _, cx| workspace.draft_discarded(cx));
    let held = on_handle(shell, &editor, cx) && resolve_open(shell, cx);
    table.row(
        shell,
        "H15 a draft discarded behind it",
        held,
        "the Result editor",
        cx,
    );
    route(shell, cx, |workspace, window, cx| {
        workspace.focus_region(Region::Content, window, cx)
    });
    let held = on_handle(shell, &editor, cx) && resolve_open(shell, cx);
    table.row(
        shell,
        "H15 a tab closed with its draft behind it",
        held,
        "the Result editor",
        cx,
    );
    let root = rig.root.clone();
    route(shell, cx, |workspace, _, cx| {
        workspace.review_in_drafts(DraftId::generate(), cx);
        workspace.settings_for_initials(root, cx);
    });
    let drafts = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.drafts_shown(cx));
    let waits = on_handle(shell, &editor, cx)
        && resolve_open(shell, cx)
        && !drafts
        && shown(shell, cx) == Shown::Tabs;
    table.row(
        shell,
        "H15 R-25 Drafts and the identity form asked for while it is open",
        waits,
        "the Result editor, the sheet open, neither opened",
        cx,
    );
    // Escape leaves the editor for its stop, then closes the sheet.
    shell.keys("escape", cx);
    shell.keys("escape", cx);
    super::support::settle(shell, cx);
    let drafts = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.drafts_shown(cx));
    let opened_after = drafts && settings_open(shell, cx) && owner(shell, cx).is_some();
    table.fact(
        "H15 R-25 they open once it closes",
        opened_after,
        &format!(
            "Drafts shown ({drafts}) and Settings open over it (owner {:?})",
            owner(shell, cx)
        ),
    );
}
