//! P6's row H11 (Wave 04 Cycle 05b design B3): quitting while Resolve
//! conflict is open closes it, and Quit now takes focus while Activity is
//! displayed, as `wait_to_quit` says; with edits no draft keeps, quitting
//! asks first, as Cycle 04's F14 has it. Each read is after two draws.

use gpui_kit::TestAppContext;

use manyhands::repository::ItemDtoKind;

use super::support::{
    Rig, Table, all_closed, canonical, has_sheet, opening, owner, region, resolve_open, route,
    within,
};
use crate::desktop::{
    model::navigation::OpenTarget,
    services::{
        JobKind,
        requests::{DesktopRequestId, Intent, Lane, RequestState},
    },
    test_support::{
        delaying::JobGate,
        fixtures::{Fixture, item_id},
    },
    view::{
        conflict::{
            SheetSource,
            tests::{TICKET, until_present},
        },
        harness::{Shell, key},
        item::ChipField,
        workspace::{QuitPhase, Region},
    },
};

/// A refresh of `root` running on the worker and held there, so quitting
/// waits for it, as `settings_sheet_tests` holds one.
fn hold_a_job(
    shell: &Shell,
    root: &std::path::Path,
    cx: &mut TestAppContext,
) -> (JobGate, DesktopRequestId) {
    let held = shell.backend.hold_job(JobKind::IndexRefresh);
    let root = root.to_owned();
    let id = shell.workspace.read_with(cx, |workspace, _| {
        workspace
            .services()
            .worker()
            .submit(Intent::IndexRefresh { root }, Lane::User, None, None)
    });
    held.wait_reached();
    (held, id)
}

async fn until_finished(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) {
    shell
        .until(cx, move |operations| {
            matches!(
                operations.request(id).map(|request| &request.state),
                Some(RequestState::Finished(..))
            )
        })
        .await;
}

/// The ticket `TICKET` of `root`, as its tab is opened.
fn ticket(root: &std::path::Path) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: item_id(TICKET),
    }
}

/// Opens Resolve conflict on the share's canned conflict, as a way in
/// does; false when refused.
async fn open_resolve(shell: &Shell, root: &std::path::Path, cx: &mut TestAppContext) -> bool {
    shell.backend.answer_next_conflict_read(canonical());
    let origin = cx.update(|cx| cx.focus_handle());
    let sheet = route(shell, cx, |workspace, window, cx| {
        workspace.show_resolve(
            opening(SheetSource::Share, root, TICKET, origin),
            window,
            cx,
        )
    });
    if sheet.is_none() {
        return false;
    }
    until_present(shell, "conflict-save", cx).await;
    true
}

/// H11: quitting while it is open, a job running so quitting waits, with
/// Activity displayed: the sheet closes, and Quit now takes focus, as
/// `wait_to_quit` says.
pub(super) async fn h11(table: &mut Table, cx: &mut TestAppContext) {
    let rig = Rig::new(cx);
    let shell = &rig.shell;
    let (held, id) = hold_a_job(shell, &rig.root, cx);
    rig.by_banner(Some(canonical()), "conflict-save", cx).await;
    shell.keys("secondary-q", cx);
    let phase = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.quit_phase());
    let closed = phase == QuitPhase::Finishing(id) && all_closed(shell, cx);
    table.fact(
        "H11 quitting while it is open: it closes",
        closed,
        &format!("the sheet closed, owner cleared, quitting waits ({phase:?})"),
    );
    let held_quit = within(shell, "quit-now", cx);
    table.row(
        shell,
        "H11 Quit now takes focus, Activity displayed",
        held_quit,
        "Quit now",
        cx,
    );
    held.release();
    until_finished(shell, id, cx).await;
}

/// H11 with edits no draft keeps (crash recovery off: the drafts folder is
/// a link, so the store does not open), as Cycle 04's F14 sets them up.
/// Quitting asks first (`unkept_asks_first`), which closed the Resolve
/// dialog with `close_all_dialogs` as it asked, and closes Settings with
/// `close_settings_quietly`; it closes Resolve conflict too (owner ruling
/// (b), 2026-11-15): H11, "it closes", is read at the ask. Cancel then
/// stays, as F14; the sheet opened again, Quit anyway waits for a job, as
/// `wait_to_quit` says.
#[cfg(unix)]
pub(super) async fn h11_unkept(table: &mut Table, cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), fixture.data_dir().join("drafts")).unwrap();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Main", "");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    super::support::open(&shell, &root, TICKET, cx);
    shell.click("item-edit", cx);
    shell.click("recovery-off-continue", cx);
    shell
        .until_drawn(cx, "the edit editor", |shell, cx| {
            let pane = shell
                .workspace
                .read_with(cx, |workspace, _| workspace.item_pane())
                .expect("an item tab is shown");
            pane.read_with(cx, |pane, _| pane.edit_host().is_some())
        })
        .await;
    shell.click(ChipField::All.id(), cx);
    shell.frame(cx, |_, _| ());
    shell.visual(cx).simulate_input("x");
    shell.run(cx);
    shell.keys("escape", cx);

    if !open_resolve(&shell, &root, cx).await {
        table.fact("H11 with unkept edits", false, "the sheet opened");
        return;
    }
    let (held, id) = hold_a_job(&shell, &root, cx);
    shell.keys("secondary-q", cx);
    let asks = shell.present("edits-not-kept-dialog", cx);
    table.fact("H11 setup: quitting asks", asks, "the EditsNotKept dialog");
    table.fact(
        "H11 with unkept edits: it closes as quitting asks",
        !resolve_open(&shell, cx),
        "the sheet closed, as the dialog closed (close_all_dialogs)",
    );
    // Cancel stays, the choices gone with the sheet, as with the dialog
    // (owner ruling (b)): the first unkept tab shown, the content region
    // focused (F14).
    shell.click("edits-not-kept-cancel", cx);
    let shown = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.opened(cx))
        == Some(ticket(&root));
    let stays = !has_sheet(&shell, cx)
        && owner(&shell, cx).is_none()
        && shown
        && region(&shell, cx) == Some(Region::Content);
    table.row(
        &shell,
        "H11 with unkept edits, Cancel: the sheet closed, the unkept tab shown",
        stays,
        "the content region, no sheet, the unkept tab shown",
        cx,
    );
    // Opened again, when Cancel left none, and quitting asked again.
    if !has_sheet(&shell, cx) && !open_resolve(&shell, &root, cx).await {
        table.fact("H11 with unkept edits, again", false, "the sheet opened");
        held.release();
        until_finished(&shell, id, cx).await;
        return;
    }
    shell.keys("secondary-q", cx);
    let asks = shell.present("edits-not-kept-dialog", cx);
    table.fact(
        "H11 setup: quitting asks again",
        asks,
        "the EditsNotKept dialog",
    );
    shell.click("edits-not-kept-quit", cx);
    let phase = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.quit_phase());
    let closed = phase == QuitPhase::Finishing(id) && !has_sheet(&shell, cx);
    table.fact(
        "H11 with unkept edits, Quit anyway: it closes",
        closed,
        &format!("no sheet, quitting waits ({phase:?})"),
    );
    let held_quit = within(&shell, "quit-now", cx);
    table.row(
        &shell,
        "H11 with unkept edits, Quit anyway: Quit now",
        held_quit,
        "Quit now",
        cx,
    );
    held.release();
    until_finished(&shell, id, cx).await;
}
