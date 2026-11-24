//! P6's rows H1 to H8 (Wave 04 Cycle 05b design B3): where focus goes as
//! Resolve conflict opens, closes by each of its ways, closes onto an
//! origin no longer drawn, gives focus back after a dialog over it, and
//! keeps it inside when a control goes. Each read is after two draws.

use gpui_kit::TestAppContext;
use manyhands::repository::{OperationId, RequestId};

use super::support::{
    BANNER, OTHER, Rig, Table, canonical, close_everything, external, focus, has_dialog, has_sheet,
    into_the_editor, on, on_close_button, on_handle, on_own, open, own_focus, region, resolve_open,
    with_body,
};
use crate::desktop::{
    model::{activity::rows::RowKey, conflict::BodyChoice},
    services::{JobKind, requests::Intent},
    test_support::fixtures::item_id,
    view::{
        activity_support::{act, list_focus, until_offered},
        conflict::{
            PartAt, REVIEW_CHANGES,
            answer_tests::applied,
            entry_tests::{child_conflict, fabricate},
            tests::{TICKET, canned, canned_path, opened, until_present},
        },
        harness::Shell,
        needs_attention::tests::actions::press_from_popover,
        operations::OPEN_LIFECYCLE,
        workspace::Region,
    },
};

/// The close a canned child conflict names (`child_conflict`).
const PARENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY1";
/// The first choice of a canonical set: Keep mine of its first field.
const FIRST_CHOICE: (&str, usize) = ("conflict-keep-mine", 0);

pub(super) async fn rows(table: &mut Table, cx: &mut TestAppContext) {
    h1(table, cx).await;
    closes(table, cx).await;
    h5_reopens(table, cx).await;
    h6(table, cx).await;
    h7(table, cx).await;
    h8(table, cx).await;
}

/// Lets every resolution this session sent end, so the worker is idle for
/// the next step.
pub(super) async fn resolutions_end(shell: &Shell, cx: &mut TestAppContext) {
    let sent = shell.requests(cx, |request| {
        matches!(request.intent, Intent::ConflictResolve { .. })
    });
    for id in sent {
        shell.until_ended(id, cx).await;
    }
}

/// H1: opened by each way in (the banner, an Activity row, the Needs
/// attention popover's row, the lifecycle dialog's Review changes…), focus
/// is on the sheet's own handle. From it, Tab reaches the body's first Tab
/// stop: the first enabled choice of a canonical set, Open folder for an
/// external set, Read again after a failed read; Shift+Tab reaches the
/// Kit's close button, drawn before the body.
async fn h1(table: &mut Table, cx: &mut TestAppContext) {
    let rig = Rig::new(cx);
    let shell = &rig.shell;

    let sheet = rig.by_banner(Some(canonical()), "conflict-save", cx).await;
    let held = on_own(shell, &sheet, cx);
    table.row(shell, "H1 the banner", held, "the sheet's own handle", cx);
    shell.keys("tab", cx);
    let held = shell.holds_control(FIRST_CHOICE, cx);
    table.row(
        shell,
        "H1 Tab, a canonical set",
        held,
        "Keep mine, the first enabled choice",
        cx,
    );
    focus(shell, &own_focus(&sheet, cx), cx);
    shell.keys("shift-tab", cx);
    let held = on_close_button(shell, cx);
    table.row(shell, "H1 Shift+Tab", held, "the Kit's close button", cx);
    close_everything(shell, cx);

    let sheet = rig
        .by_banner(Some(external()), "conflict-open-folder", cx)
        .await;
    let held = on_own(shell, &sheet, cx);
    table.row(
        shell,
        "H1 an external set",
        held,
        "the sheet's own handle",
        cx,
    );
    shell.keys("tab", cx);
    let held = shell.holds_control("conflict-open-folder", cx);
    table.row(shell, "H1 Tab, an external set", held, "Open folder", cx);
    close_everything(shell, cx);

    // No read canned: the library's own refuses an operation the
    // repository does not hold.
    let sheet = rig.by_banner(None, "conflict-read-again", cx).await;
    let held = on_own(shell, &sheet, cx);
    table.row(
        shell,
        "H1 a failed read",
        held,
        "the sheet's own handle",
        cx,
    );
    shell.keys("tab", cx);
    let held = shell.holds_control("conflict-read-again", cx);
    table.row(shell, "H1 Tab, a failed read", held, "Read again", cx);
    close_everything(shell, cx);

    let entry = rig.entry();
    until_offered(shell, &entry, REVIEW_CHANGES, cx).await;
    shell.backend.answer_next_conflict_read(canonical());
    act(shell, &entry, REVIEW_CHANGES, cx);
    until_present(shell, "conflict-save", cx).await;
    let sheet = opened(cx).expect("the sheet is open");
    let held = on_own(shell, &sheet, cx);
    table.row(
        shell,
        "H1 an Activity row",
        held,
        "the sheet's own handle",
        cx,
    );
    close_everything(shell, cx);

    shell.backend.answer_next_conflict_read(canonical());
    press_from_popover(shell, &entry, REVIEW_CHANGES, cx);
    until_present(shell, "conflict-save", cx).await;
    let sheet = opened(cx).expect("the sheet is open");
    let held = on_own(shell, &sheet, cx);
    table.row(
        shell,
        "H1 the popover's row",
        held,
        "the sheet's own handle",
        cx,
    );
    close_everything(shell, cx);

    let close = fabricate(
        shell,
        2,
        Intent::TicketClose {
            root: rig.root.clone(),
            id: item_id(TICKET),
        },
        child_conflict(&rig.root, PARENT, RequestId::new()),
        cx,
    );
    act(shell, &RowKey::Request(close), OPEN_LIFECYCLE, cx);
    until_present(shell, "lifecycle-resolve-conflict", cx).await;
    let operation = OperationId::parse(PARENT).unwrap();
    shell
        .backend
        .answer_next_conflict_read(canned(operation, vec![canned_path(TICKET, '1')]));
    shell.click("lifecycle-resolve-conflict", cx);
    until_present(shell, "conflict-save", cx).await;
    let sheet = opened(cx).expect("the sheet is open");
    let held = on_own(shell, &sheet, cx) && !has_dialog(shell, cx);
    table.row(
        shell,
        "H1 the lifecycle dialog's Review changes…",
        held,
        "the sheet's own handle, the dialog gone",
        cx,
    );
    close_everything(shell, cx);
}

/// H2, H3, H4, H16 (Resume) and H5 (a save that closes it), each from
/// the banner, whose Review changes is the origin. H5 goes last: its
/// applied resolution clears the banner.
async fn closes(table: &mut Table, cx: &mut TestAppContext) {
    let rig = Rig::new(cx);
    let shell = &rig.shell;

    // H2: Escape, focus on the sheet's own handle.
    rig.by_banner(Some(canonical()), "conflict-save", cx).await;
    shell.keys("escape", cx);
    let held = !has_sheet(shell, cx) && on(shell, BANNER, cx);
    table.row(
        shell,
        "H2 Escape, not in the Result editor",
        held,
        "the origin, the banner's Review changes",
        cx,
    );
    close_everything(shell, cx);

    // H3: Escape in the Result editor. The editor's own element has no ID,
    // so focus in it reads as on its stop too: the editor must not hold it.
    let sheet = rig.by_banner(Some(with_body()), "conflict-save", cx).await;
    let editor = into_the_editor(shell, &sheet, cx);
    shell.keys("escape", cx);
    let held = on(shell, ("conflict-result", 0usize), cx)
        && !on_handle(shell, &editor, cx)
        && resolve_open(shell, cx);
    table.row(
        shell,
        "H3 Escape in the Result editor",
        held,
        "the Result stop, the sheet open",
        cx,
    );
    close_everything(shell, cx);

    // H4: Cancel, then the close button.
    rig.by_banner(Some(canonical()), "conflict-save", cx).await;
    shell.click("conflict-cancel", cx);
    let held = !has_sheet(shell, cx) && on(shell, BANNER, cx);
    table.row(shell, "H4 Cancel", held, "the origin", cx);
    close_everything(shell, cx);
    rig.by_banner(Some(canonical()), "conflict-save", cx).await;
    super::support::click_close_button(shell, cx);
    let held = !has_sheet(shell, cx) && on(shell, BANNER, cx);
    table.row(shell, "H4 the close button", held, "the origin", cx);
    close_everything(shell, cx);

    // H16: Resume.
    rig.by_banner(Some(super::support::resumable()), "conflict-resume", cx)
        .await;
    shell.click("conflict-resume", cx);
    let held = !has_sheet(shell, cx) && on(shell, BANNER, cx);
    table.row(shell, "H16 Resume", held, "the origin", cx);
    let resumes = shell.requests(cx, |request| {
        matches!(request.intent, Intent::OperationResume { .. })
    });
    for id in resumes {
        shell.until_ended(id, cx).await;
    }
    close_everything(shell, cx);

    // H5: a save the library applies, which closes it. Opened from
    // Activity's row, whose list, the origin, stays drawn: the applied
    // resolution clears the banner.
    let entry = rig.entry();
    until_offered(shell, &entry, REVIEW_CHANGES, cx).await;
    shell.backend.answer_next_conflict_read(canonical());
    act(shell, &entry, REVIEW_CHANGES, cx);
    until_present(shell, "conflict-save", cx).await;
    shell.click(FIRST_CHOICE, cx);
    shell
        .backend
        .hold_job_return_answering(JobKind::ConflictResolve, applied())
        .release();
    shell.click("conflict-save", cx);
    resolutions_end(shell, cx).await;
    let list = list_focus(shell, cx);
    let held = !has_sheet(shell, cx) && on_handle(shell, &list, cx);
    table.row(
        shell,
        "H5 a save that closes it",
        held,
        "the origin, Activity's list",
        cx,
    );
    close_everything(shell, cx);
}

/// H5, a lifecycle child's: the save that closes the sheet sends the close
/// on, and `applied` opens the lifecycle dialog again, which takes focus.
/// The close this session sent is refused for its missing confirmation,
/// as `answer_tests::a_lifecycle_child_resolution_sends_the_lifecycle_again`
/// sends it.
async fn h5_reopens(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, root, close) = super::focus_late::close_shell(cx).await;
    let Some(_sheet) = super::focus_late::open_close_child(&shell, &root, close, cx).await else {
        table.fact("H5 the lifecycle dialog", false, "the sheet opened");
        return;
    };
    shell.click(FIRST_CHOICE, cx);
    shell
        .backend
        .hold_job_return_answering(JobKind::ConflictResolve, applied())
        .release();
    shell.click("conflict-save", cx);
    resolutions_end(&shell, cx).await;
    super::focus_late::until_attempt(&shell, close, 2, cx).await;
    let reopened = super::focus_late::until_lifecycle(&shell, cx).await;
    let held = reopened && !has_sheet(&shell, cx) && super::focus_late::in_lifecycle(&shell, cx);
    table.row(
        &shell,
        "H5 a save that closes it, the lifecycle dialog reopened",
        held,
        "inside the lifecycle dialog that `applied` reopens",
        cx,
    );
    shell.until_ended(close, cx).await;
}

/// H6: the origin is no longer drawn when the sheet closes (a landing shows
/// another item's tab, the banner's with it gone): `focus_lost`'s fallback,
/// here the origin's region, the content region.
async fn h6(table: &mut Table, cx: &mut TestAppContext) {
    let rig = Rig::new(cx);
    let shell = &rig.shell;
    rig.by_banner(Some(canonical()), "conflict-save", cx).await;
    open(shell, &rig.root, OTHER, cx);
    let gone = resolve_open(shell, cx) && !shell.present(BANNER, cx);
    table.fact(
        "H6 setup: the origin gone under the open sheet",
        gone,
        "the sheet open, the banner not drawn",
    );
    shell.keys("escape", cx);
    let held = !has_sheet(shell, cx) && region(shell, cx) == Some(Region::Content);
    table.row(
        shell,
        "H6 the origin no longer drawn",
        held,
        "focus_lost's fallback, the content region",
        cx,
    );
}

/// H7: a dialog opened over the sheet (an Unlock key prompt) closes, and
/// focus is on the control that held it as the dialog opened.
async fn h7(table: &mut Table, cx: &mut TestAppContext) {
    let rig = Rig::new(cx);
    let shell = &rig.shell;
    rig.by_banner(Some(canonical()), "conflict-save", cx).await;
    shell.keys("tab", cx);
    let reached = shell.holds_control(FIRST_CHOICE, cx);
    table.fact(
        "H7 setup: Tab reaches Keep mine",
        reached,
        "Keep mine focused",
    );
    if !reached {
        return;
    }
    let _answer = shell.prompt(cx);
    let prompted = has_dialog(shell, cx) && shell.prompt_focused(cx) && resolve_open(shell, cx);
    table.fact(
        "H7 setup: the prompt over the sheet",
        prompted,
        "the prompt open and focused, the sheet under it",
    );
    shell.keys("escape", cx);
    let held =
        !has_dialog(shell, cx) && resolve_open(shell, cx) && shell.holds_control(FIRST_CHOICE, cx);
    table.row(
        shell,
        "H7 a dialog over the sheet closes",
        held,
        "Keep mine, which held focus as the prompt opened",
        cx,
    );
}

/// H8: a body control that goes (a choice disabled while a save is out,
/// sent by Ctrl+Enter from it) leaves focus on `conflict-sheet`'s handle; a
/// footer control that goes by itself (Save, disabled while the save
/// Ctrl+Enter sent from it is out) leaves it on `footer_focus`, which
/// `conflict-footer` tracks; and a footer press (Enter on Save, as B3's
/// "Footer presses keep focus inside" has it) focuses the sheet's own
/// handle first, so Save going leaves focus there.
async fn h8(table: &mut Table, cx: &mut TestAppContext) {
    let rig = Rig::new(cx);
    let shell = &rig.shell;

    // Keep mine, reached by Tab and pressed by Enter: a body press does not
    // move focus, so the choice holds it.
    let sheet = rig.by_banner(Some(canonical()), "conflict-save", cx).await;
    shell.keys("tab", cx);
    shell.press("enter", cx);
    let chosen = shell.holds_control(FIRST_CHOICE, cx)
        && sheet.read_with(cx, |sheet, _| sheet.choice_of(PartAt::Field(0)))
            == Some(BodyChoice::Mine);
    table.fact(
        "H8 setup: a choice holds focus",
        chosen,
        "Keep mine chosen and focused",
    );
    let held = shell.backend.hold_job(JobKind::ConflictResolve);
    shell.keys("ctrl-enter", cx);
    let kept = on_own(shell, &sheet, cx);
    table.row(
        shell,
        "H8 a body control goes",
        kept,
        "conflict-sheet's own handle",
        cx,
    );
    held.release();
    resolutions_end(shell, cx).await;
    close_everything(shell, cx);

    rig.by_banner(Some(canonical()), "conflict-save", cx).await;
    shell.click(FIRST_CHOICE, cx);
    let reached = shell.tab_reaches("conflict-save", 12, cx);
    table.fact("H8 setup: Tab reaches Save", reached, "Save focused");
    let held = shell.backend.hold_job(JobKind::ConflictResolve);
    shell.keys("ctrl-enter", cx);
    let kept = on(shell, "conflict-footer", cx);
    table.row(
        shell,
        "H8 a footer control goes by itself",
        kept,
        "footer_focus, which conflict-footer tracks",
        cx,
    );
    held.release();
    resolutions_end(shell, cx).await;
    close_everything(shell, cx);

    let sheet = rig.by_banner(Some(canonical()), "conflict-save", cx).await;
    shell.click(FIRST_CHOICE, cx);
    let reached = shell.tab_reaches("conflict-save", 12, cx);
    table.fact("H8 setup: Tab reaches Save again", reached, "Save focused");
    let held = shell.backend.hold_job(JobKind::ConflictResolve);
    shell.press("enter", cx);
    let kept = on_own(shell, &sheet, cx);
    table.row(
        shell,
        "H8 a footer press (B3, O-8)",
        kept,
        "the sheet's own handle, focused before Save went",
        cx,
    );
    held.release();
    resolutions_end(shell, cx).await;
    close_everything(shell, cx);
}
