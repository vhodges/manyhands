//! P7's rows (Wave 04 Cycle 05b design B1, B2, decision 9, R-18): nothing
//! opens over Resolve conflict or acts behind it, and it opens over
//! nothing. Guards are tested with dispatched actions and direct calls,
//! never pointer clicks, which the scrim would take.
//!
//! The deleted shared branch and the Give up's answer are canned, as
//! `operations/republish_tests.rs` cans them (decision 9): its share runs
//! against an SSH remote whose transport the test binary never initializes,
//! so the library accepts it and its record stays accepted.

use std::path::PathBuf;

use gpui_kit::{Action, Entity, FocusHandle, Keystroke, TestAppContext, component::WindowExt};
use manyhands::{
    canonical::CONFIG_PATH,
    repository::AuthoringKind,
    results::{Effects, Envelope, ResultCode, Scope},
};

use super::support::{
    Rig, Table, canonical, close_everything, has_dialog, has_sheet, on_own, open_probe, opening,
    owner, resolve_open, route, settings_open, settle, shown, synchronization,
};
use crate::desktop::{
    keys::{ActivityMenuChoice, FocusOperations, FocusRepositories},
    model::{
        activity::rows::RowKey,
        layout::Layout,
        stamp::{SettingsPage, TabId},
        tabs::TabKey,
    },
    services::{
        JobKind,
        requests::{DesktopRequestId, Intent, Lane, RequestState},
    },
    test_support::fixtures::{Fixture, item_id},
    view::{
        activity_support::{act, list_focus, refresh, row as activity_row, until_offered},
        conflict::{
            ResolveSheet, SheetSource,
            tests::{TICKET, canned, canned_path, opened, until_present},
        },
        harness::Shell,
        item::ChipField,
        operations::republish::{
            STOP_AND_REPUBLISH,
            tests::{TICKET as SHARED, republish_preview},
        },
        settings::{SettingsRoute, Shown},
        workspace::add_sheet::AddRoute,
    },
};

use super::super::super::settings_sheet::SheetOwnerKind;

pub(super) async fn rows(table: &mut Table, cx: &mut TestAppContext) {
    over_the_sheet(table, cx).await;
    republish_waits(table, Path::OnceStopped, cx).await;
    republish_waits(table, Path::Deferred, cx).await;
}

/// Whether `sheet` is the one the workspace keeps, open.
fn still(shell: &Shell, sheet: &Entity<ResolveSheet>, cx: &mut TestAppContext) -> bool {
    resolve_open(shell, cx) && opened(cx).is_some_and(|open| open == *sheet)
}

/// What a dispatched action behind the sheet might change.
#[derive(PartialEq, Debug)]
struct Footprint {
    shown: Shown,
    owner: Option<SheetOwnerKind>,
    sheet: bool,
    dialog: bool,
    layout: Layout,
    tabs: Vec<TabKey>,
    tab: Option<TabId>,
    drafts: bool,
    focus: Option<FocusHandle>,
    requests: Vec<(DesktopRequestId, &'static str)>,
}

fn state_name(state: &RequestState) -> &'static str {
    match state {
        RequestState::Queued => "queued",
        RequestState::Running { .. } => "running",
        RequestState::Finished(..) => "finished",
        RequestState::Removed => "removed",
        RequestState::Dismissed => "dismissed",
        RequestState::NotSent(_) => "not sent",
    }
}

fn footprint(shell: &Shell, cx: &mut TestAppContext) -> Footprint {
    settle(shell, cx);
    let (focus, sheet, dialog) = shell.frame(cx, |window, cx| {
        (
            window.focused(cx),
            window.has_active_sheet(cx),
            window.has_active_dialog(cx),
        )
    });
    let (shown, owner, layout, tabs, tab, drafts) =
        shell.workspace.read_with(cx, |workspace, cx| {
            (
                workspace.settings_shown(cx),
                workspace.sheet_owner(),
                workspace.layout().clone(),
                workspace.tab_keys(),
                workspace.shown_tab(),
                workspace.drafts_shown(cx),
            )
        });
    let requests = shell.operations(cx).read_with(cx, |operations, _| {
        operations
            .requests()
            .iter()
            .map(|request| (request.id, state_name(&request.state)))
            .collect()
    });
    Footprint {
        shown,
        owner,
        sheet,
        dialog,
        layout,
        tabs,
        tab,
        drafts,
        focus,
        requests,
    }
}

fn dispatch(shell: &Shell, action: Box<dyn Action>, cx: &mut TestAppContext) {
    shell
        .visual(cx)
        .update(|window, cx| window.dispatch_action(action, cx));
    shell.run(cx);
}

/// Over the banner's sheet: Ctrl+, does nothing, `show_settings` and
/// `show_add_repository` refuse (decision 9); a details sheet asked for
/// does not open, then or once the sheet closes (B1); a region focus key
/// and Activity's choices change nothing (R-60); `show_resolve` refuses
/// over a sheet and over a dialog (B2, R-21); Ctrl+, right after Escape,
/// with no draw between, opens Settings (B1, `resolve_open`).
async fn over_the_sheet(table: &mut Table, cx: &mut TestAppContext) {
    let rig = Rig::new(cx);
    let shell = &rig.shell;

    let sheet = rig.by_banner(Some(canonical()), "conflict-save", cx).await;
    shell.keys("secondary-,", cx);
    let refused = still(shell, &sheet, cx) && shown(shell, cx) == Shown::Tabs;
    table.fact(
        "P7 Ctrl+, does nothing",
        refused && on_own(shell, &sheet, cx),
        "the sheet kept, focus on its own handle, Settings not open",
    );
    close_everything(shell, cx);

    let sheet = rig.by_banner(Some(canonical()), "conflict-save", cx).await;
    route(shell, cx, |workspace, window, cx| {
        workspace.show_settings(
            SettingsRoute::Page(SettingsPage::Repositories),
            None,
            window,
            cx,
        )
    });
    let refused = still(shell, &sheet, cx) && shown(shell, cx) == Shown::Tabs;
    table.fact(
        "P7 show_settings refuses",
        refused && on_own(shell, &sheet, cx),
        "the sheet kept, Settings not open",
    );
    close_everything(shell, cx);

    let sheet = rig.by_banner(Some(canonical()), "conflict-save", cx).await;
    route(shell, cx, |workspace, window, cx| {
        workspace.show_add_repository(AddRoute::New, None, window, cx)
    });
    let refused = still(shell, &sheet, cx) && !shell.present("add-path", cx);
    table.fact(
        "P7 show_add_repository refuses",
        refused && on_own(shell, &sheet, cx),
        "the sheet kept, the Add sheet not open",
    );
    close_everything(shell, cx);

    let sheet = rig.by_banner(Some(canonical()), "conflict-save", cx).await;
    let pane = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .expect("an item tab is shown");
    route(shell, cx, |workspace, _, cx| {
        workspace.ask_details(&pane, ChipField::Project, cx)
    });
    settle(shell, cx);
    table.fact(
        "P7 a details sheet asked for does not open",
        still(shell, &sheet, cx) && !shell.present("sheet-heading", cx),
        "the sheet kept, no details sheet",
    );
    shell.keys("escape", cx);
    settle(shell, cx);
    table.fact(
        "P7 nor once the sheet closes",
        !has_sheet(shell, cx) && !shell.present("sheet-heading", cx),
        "no sheet, no details sheet",
    );
    close_everything(shell, cx);

    let sheet = rig.by_banner(Some(canonical()), "conflict-save", cx).await;
    let before = footprint(shell, cx);
    for (name, action) in [
        ("FocusOperations", FocusOperations.boxed_clone()),
        ("FocusRepositories", FocusRepositories.boxed_clone()),
    ] {
        dispatch(shell, action, cx);
        let now = footprint(shell, cx);
        table.fact(
            &format!("P7 a region focus key ({name})"),
            now == before,
            &format!("nothing changed (now {now:?})"),
        );
    }
    let entry = rig.entry();
    refresh(shell, cx);
    let row = activity_row(shell, &entry, cx).expect("the share's row");
    let offers: Vec<_> = row
        .row
        .menu
        .iter()
        .filter(|offer| offer.enabled)
        .cloned()
        .collect();
    table.fact(
        "P7 setup: the share's row offers something",
        !offers.is_empty(),
        "an enabled offer",
    );
    for offer in offers {
        let choice = ActivityMenuChoice {
            key: entry.clone(),
            action: offer.action.clone(),
            shown: offer.label.clone(),
            revision: row.revision,
        };
        dispatch(shell, choice.boxed_clone(), cx);
        let now = footprint(shell, cx);
        table.fact(
            &format!("P7 an Activity action ({})", offer.label),
            now == before,
            &format!("nothing changed (now {now:?})"),
        );
    }

    // `show_resolve` over the open sheet, then over Settings.
    let origin = cx.update(|cx| cx.focus_handle());
    let again = route(shell, cx, |workspace, window, cx| {
        workspace.show_resolve(
            opening(SheetSource::Share, &rig.root, TICKET, origin.clone()),
            window,
            cx,
        )
    });
    table.fact(
        "P7 show_resolve refuses over a sheet (itself)",
        again.is_none() && still(shell, &sheet, cx),
        "none opened, the first kept",
    );
    close_everything(shell, cx);
    shell.keys("secondary-,", cx);
    let settings = settings_open(shell, cx);
    let over = route(shell, cx, |workspace, window, cx| {
        workspace.show_resolve(
            opening(SheetSource::Share, &rig.root, TICKET, origin.clone()),
            window,
            cx,
        )
    });
    table.fact(
        "P7 show_resolve refuses over a sheet (Settings)",
        settings && over.is_none() && settings_open(shell, cx),
        "none opened, Settings kept",
    );
    close_everything(shell, cx);

    shell.visual(cx).update(open_probe);
    shell.run(cx);
    let under = route(shell, cx, |workspace, window, cx| {
        workspace.show_resolve(
            opening(SheetSource::Share, &rig.root, TICKET, origin.clone()),
            window,
            cx,
        )
    });
    table.fact(
        "P7 show_resolve refuses over a dialog",
        under.is_none() && !has_sheet(shell, cx) && has_dialog(shell, cx),
        "no sheet, the dialog kept",
    );
    close_everything(shell, cx);

    // Escape, then Ctrl+,'s handler in the same update, with no draw
    // between: the Kit has closed the sheet, `sync_sheet` has not cleared
    // its owner yet, and Settings opens all the same (B1, `resolve_open`).
    // The handler is called as the key calls it: GPUI draws a window that
    // needs it before it dispatches the next keystroke, which would clear
    // the owner first.
    rig.by_banner(Some(canonical()), "conflict-save", cx).await;
    let workspace = shell.workspace.clone();
    let between = shell.visual(cx).update(|window, cx| {
        window.dispatch_keystroke(Keystroke::parse("escape").unwrap(), cx);
        let between = (
            workspace.read(cx).sheet_owner(),
            window.has_active_sheet(cx),
        );
        workspace.update(cx, |workspace, cx| workspace.settings_key(window, cx));
        between
    });
    shell.run(cx);
    table.fact(
        "P7 setup: between Escape and Ctrl+, the owner is not yet cleared",
        between == (Some(SheetOwnerKind::Resolve), false),
        &format!("the owner Resolve, no sheet active ({between:?})"),
    );
    settle(shell, cx);
    table.fact(
        "P7 Ctrl+, right after Escape opens Settings",
        settings_open(shell, cx) && shown(shell, cx) == Shown::Settings(SettingsPage::Repositories),
        &format!("Settings open (owner {:?})", owner(shell, cx)),
    );
    close_everything(shell, cx);
}

/// The two ways a share's Stop and republish… opens Republish… once its
/// share has stopped (R-18).
#[derive(Clone, Copy, Debug)]
enum Path {
    /// `republish_once_stopped`: the Give up answers with no dialog open.
    OnceStopped,
    /// `open_deferred_republish`: it answers under a dialog, and that
    /// dialog then closes.
    Deferred,
}

/// An item's share, run against an SSH remote whose transport the test
/// binary never initializes, so the library accepts it and its record
/// stays accepted; its answer canned as its deleted branch, as
/// `republish_tests.rs`'s `accepted_deleted_share` runs it. Gives the
/// window, the repository and the share.
async fn deleted_share(cx: &mut TestAppContext) -> (Shell, PathBuf, DesktopRequestId) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, SHARED, "Untitled", "");
    fixture.publish_over_ssh(&root);
    let config = std::fs::read(root.join(CONFIG_PATH)).unwrap();
    fixture.commit_file(&root, CONFIG_PATH, &config);
    fixture.refresh(&root);
    fixture.save_ticket(&root, SHARED, "Shared");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    let deleted = Envelope::classified_mutation(
        "item sync",
        Scope::default(),
        false,
        ResultCode::RemoteBranchDeleted,
        Effects::not_requested(),
        false,
    );
    shell
        .backend
        .hold_job_return_answering(JobKind::SynchronizeItem, deleted)
        .release();
    let id = shell.workspace.read_with(cx, |workspace, _| {
        workspace.services().worker().submit(
            Intent::ItemSync {
                root: root.clone(),
                kind: AuthoringKind::Ticket,
                id: item_id(SHARED),
            },
            Lane::User,
            None,
            None,
        )
    });
    shell.until_ended(id, cx).await;
    (shell, root, id)
}

/// The command of each confirmation dialog open, the oldest first.
fn commands(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    let command = gpui_kit::ElementId::from("confirm-command");
    shell.frame(cx, |window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .filter(|snapshot| snapshot.path().last() == Some(&command))
            .filter_map(|snapshot| snapshot.label().map(str::to_owned))
            .collect()
    })
}

/// R-18: a share's Stop and republish…, its Give up held; Resolve conflict
/// opened meanwhile; the Give up answered `cancelled` while the sheet is
/// open (`path`). Republish… waits while the sheet is open, and opens once
/// it has closed.
async fn republish_waits(table: &mut Table, path: Path, cx: &mut TestAppContext) {
    let (shell, root, id) = deleted_share(cx).await;
    shell
        .backend
        .answer_next_prepare("item republish", republish_preview(&shell, &root));
    let entry = RowKey::Request(id);
    until_offered(&shell, &entry, STOP_AND_REPUBLISH, cx).await;
    let cancelled = Envelope::classified_mutation(
        "item sync",
        Scope::default(),
        true,
        ResultCode::Cancelled,
        Effects::not_requested(),
        false,
    );
    let give_up = shell
        .backend
        .hold_job_return_answering(JobKind::SynchronizeItem, cancelled);
    act(&shell, &entry, STOP_AND_REPUBLISH, cx);
    give_up.wait_reached();

    shell
        .backend
        .answer_next_conflict_read(canned(synchronization(), vec![canned_path(SHARED, '1')]));
    let origin = list_focus(&shell, cx);
    let sheet = route(&shell, cx, |workspace, window, cx| {
        workspace.show_resolve(
            opening(SheetSource::Share, &root, SHARED, origin),
            window,
            cx,
        )
    });
    let opened_now = sheet.is_some();
    table.fact(
        &format!("P7 {path:?} setup: the sheet opened"),
        opened_now,
        "the sheet open while the Give up is out",
    );
    if opened_now {
        until_present(&shell, "conflict-save", cx).await;
    }
    if matches!(path, Path::Deferred) {
        shell.visual(cx).update(open_probe);
        shell.run(cx);
    }
    give_up.release();
    shell
        .until(cx, move |operations| {
            operations.request(id).is_some_and(|request| {
                request.attempt == 2 && matches!(request.state, RequestState::Finished(..))
            })
        })
        .await;
    settle(&shell, cx);
    if matches!(path, Path::Deferred) {
        // The dialog it waited for closes; `open_deferred_republish` tries
        // at the next render.
        shell
            .visual(cx)
            .update(|window, cx| window.close_dialog(cx));
        shell.run(cx);
        settle(&shell, cx);
    }
    let waits = resolve_open(&shell, cx) && commands(&shell, cx).is_empty();
    table.fact(
        &format!("P7 {path:?}: Republish… waits while the sheet is open"),
        waits,
        &format!(
            "the sheet open, no confirmation (open: {:?})",
            commands(&shell, cx)
        ),
    );
    shell.keys("escape", cx);
    for _ in 0..4 {
        settle(&shell, cx);
    }
    let after = commands(&shell, cx);
    table.fact(
        &format!("P7 {path:?}: Republish… opens once it closes"),
        !has_sheet(&shell, cx) && after == ["Republish"],
        &format!("no sheet, Republish… open (open: {after:?})"),
    );
    close_everything(&shell, cx);
}
