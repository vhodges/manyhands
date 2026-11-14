//! Settings as a sheet the workspace keeps (Wave 04 Cycle 04 design Part
//! A, B2 and C3), over real repositories: the workspace keeps the sheet
//! whatever the item area shows (P1); the item, its scroll and its draft are
//! as they were once it closes (P2); it replaces the details sheet, whose
//! edits the draft keeps (P3); it closes every way, with a typed passphrase
//! emptied (P6); nothing acts behind it (P7); it scrolls on its own (P8);
//! Show in Activity closes it and focuses Activity (P14); and no route opens
//! it under a dialog or while quitting (P15).
//!
//! Design A6's focus rows, written before the focus code (Task 2's design
//! checkpoint), are tables in files of their own, run from here so that
//! each test is named here: where focus returns (P5, `returns` and
//! `returns_activity`), late work (P22, `late`), focus kept inside (P23,
//! `inside`), and where every way in lands (P4, `routes` and `ways`; P20
//! by keyboard, `keyboard`).
//!
//! A focus that design A6 names is read after two draws (Ruling 2). Guards
//! are tested with dispatched actions, which the scrim does not take. A
//! test that waits on the worker allows parking. Draft fields and typed
//! secrets would print through `Debug`, so they are compared with
//! `assert!`.

use gpui_kit::{
    Action, FocusHandle, Modifiers, TestAppContext, component::WindowExt, point, px,
    test::TestWindowExt,
};

use super::SheetOwnerKind;
use crate::desktop::{
    keys::{
        CloseTab, FocusContent, FocusNavigation, FocusOperations, FocusRepositories, NewItem,
        NextRegion, NextTab, OpenDrafts, OpenSettings, PreviousRegion, PreviousTab,
        ToggleBottomDock, ToggleLeftDock, ToggleRightDock,
    },
    model::{
        layout::{Layout, PanelKind},
        polling::Answer,
        stamp::{SettingsPage, TabId},
        tabs::TabKey,
    },
    services::requests::RequestState,
    test_support::fixtures::Fixture,
    view::{
        footer::Control,
        harness::{Shell, key},
        item::ChipField,
        settings::{SettingsRoute, Shown, polling_support::Rig},
        workspace::{QuitPhase, Region},
    },
};

#[path = "settings_sheet_tests/behind.rs"]
mod behind;
#[path = "settings_sheet_tests/inside.rs"]
mod inside;
#[path = "settings_sheet_tests/keyboard.rs"]
mod keyboard;
#[path = "settings_sheet_tests/late.rs"]
mod late;
#[path = "settings_sheet_tests/returns.rs"]
mod returns;
#[path = "settings_sheet_tests/returns_activity.rs"]
mod returns_activity;
#[path = "settings_sheet_tests/routes.rs"]
mod routes;
#[path = "settings_sheet_tests/support.rs"]
mod support;
#[path = "settings_sheet_tests/table.rs"]
mod table;
#[path = "settings_sheet_tests/ways.rs"]
mod ways;

use support::*;

/// P1 (A1, R-2, R-31, R-33, R-51): after Ctrl+, the owner is Settings and a
/// sheet is active frame after frame: with an item shown, with Drafts shown
/// under it, and after `open_item` lands, as the filter's ID read does. A
/// details sheet asked for in the same frame as Ctrl+, never opens.
#[gpui_kit::test]
fn settings_is_a_sheet_the_workspace_keeps(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, MAIN, "Main", "project: p\n");
        fixture.commit_ticket(root, OTHER, "Other", "");
    });
    open(&shell, ticket(&root, MAIN), cx);

    // With an item shown, which stays shown under the sheet.
    shell.keys("secondary-,", cx);
    kept(&shell, SettingsPage::Repositories, cx);
    assert_eq!(opened(&shell, cx), Some(ticket(&root, MAIN)));
    // An item opened by a landing leaves Settings open, the item under it.
    open(&shell, ticket(&root, OTHER), cx);
    kept(&shell, SettingsPage::Repositories, cx);
    assert_eq!(opened(&shell, cx), Some(ticket(&root, OTHER)));
    shell.keys("escape", cx);
    closed(&shell, "Escape over an item", cx);

    // With Drafts shown, which stays shown under the sheet and after it.
    shell.keys("secondary-shift-d", cx);
    assert!(drafts_shown(&shell, cx));
    shell.keys("secondary-,", cx);
    kept(&shell, SettingsPage::Repositories, cx);
    assert!(
        drafts_shown(&shell, cx),
        "Drafts stays shown under the sheet"
    );
    shell.keys("escape", cx);
    closed(&shell, "Escape over Drafts", cx);
    assert!(
        drafts_shown(&shell, cx),
        "Drafts is shown once Settings closes"
    );

    // A details sheet asked for in the same frame as Ctrl+, is dropped.
    open(&shell, ticket(&root, MAIN), cx);
    let pane = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .expect("an item tab is shown");
    let workspace = shell.workspace.clone();
    shell.visual(cx).update(|window, cx| {
        workspace.update(cx, |workspace, cx| {
            workspace.ask_details(&pane, ChipField::Project, cx)
        });
        window.dispatch_action(OpenSettings.boxed_clone(), cx);
    });
    shell.run(cx);
    kept(&shell, SettingsPage::Repositories, cx);
    assert!(
        !shell.present("sheet-heading", cx),
        "no details sheet opened"
    );
    shell.keys("escape", cx);
    closed(&shell, "Escape", cx);
    assert!(
        !shell.present("sheet-heading", cx),
        "nor does one once Settings closes"
    );
}

/// P2 (contract 2; the Wave's exit evidence): with an item open, its region
/// scrolled and a field edited into its draft, Ctrl+, then Tab to a control
/// below the fold, then the `AddRepository` route (W-6's) to the Add
/// folder, then Escape, leave the same tab shown, the tabs as they were,
/// the item's offset as it was, and the draft holding the edit.
#[gpui_kit::test]
async fn settings_leaves_the_item_as_it_was(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, long_and_other);
    shell.resize_keeping_content(1440., SHORT, cx);
    shell.set_activity_height(SHORT_ACTIVITY, cx);
    open(&shell, ticket(&root, OTHER), cx);
    open(&shell, ticket(&root, MAIN), cx);
    edit(&shell, cx).await;
    let draft = edit_the_title(&shell, &root, cx).await;
    shell.keys("escape", cx);
    assert!(!has_sheet(&shell, cx), "the details sheet closed");
    shell.keys("secondary-3", cx);
    shell.keys("pagedown", cx);
    let offset = item_offset(&shell, cx);
    assert!(offset < px(0.), "the item's region scrolled: {offset:?}");
    let (tab, tabs): (Option<TabId>, Vec<TabKey>) =
        shell.workspace.read_with(cx, |workspace, _| {
            (workspace.shown_tab(), workspace.tab_keys())
        });

    shell.keys("secondary-,", cx);
    kept(&shell, SettingsPage::Repositories, cx);
    tab_to_create(&shell, cx);
    assert!(
        settings_offset(&shell, cx) < px(0.),
        "Settings scrolled to the control"
    );
    assert_eq!(item_offset(&shell, cx), offset, "the item did not move");
    // W-6's route, raised while Settings is open, arrives in place.
    route(&shell, cx, |workspace, window, cx| {
        workspace.show_settings(SettingsRoute::AddRepository, None, window, cx)
    });
    assert!(
        holds(&shell, "add-path", cx),
        "the AddRepository route lands on the Add folder"
    );
    assert_eq!(item_offset(&shell, cx), offset, "the item still did not");
    shell.keys("escape", cx);
    closed(&shell, "Escape", cx);

    let (now_tab, now_tabs) = shell.workspace.read_with(cx, |workspace, _| {
        (workspace.shown_tab(), workspace.tab_keys())
    });
    assert_eq!(now_tab, tab, "the same tab is shown");
    assert_eq!(now_tabs, tabs, "the tabs are as they were");
    assert_eq!(item_offset(&shell, cx), offset, "the item kept its place");
    assert!(
        titled(&shell, draft, "Mainx", cx),
        "the draft holds the edit"
    );
}

/// P3 (contract 3; Wave decision 6; R-6, R-8; A6 F3): with the details sheet
/// open and a field edited, Ctrl+, opens Settings in its place; the details
/// sheet is gone and the draft holds the edit. Focus is on the page's
/// entry, and Escape returns it to the chip that opened the details sheet.
#[gpui_kit::test]
async fn settings_replaces_the_details_sheet_and_keeps_its_edits(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, MAIN, "Main", "project: p\n");
    });
    open(&shell, ticket(&root, MAIN), cx);
    edit(&shell, cx).await;
    let draft = edit_the_title(&shell, &root, cx).await;

    shell.keys("secondary-,", cx);
    assert_eq!(
        shown(&shell, cx),
        Shown::Settings(SettingsPage::Repositories)
    );
    assert_eq!(owner(&shell, cx), Some(SheetOwnerKind::Settings));
    assert!(has_sheet(&shell, cx));
    assert!(
        !shell.present("sheet-heading", cx),
        "the details sheet is gone"
    );
    assert!(
        titled(&shell, draft, "Mainx", cx),
        "the draft holds the edit"
    );
    assert!(
        holds(&shell, ("settings-page", 0usize), cx),
        "F3: the page's entry"
    );

    shell.keys("escape", cx);
    closed(&shell, "Escape", cx);
    assert!(
        holds(&shell, ChipField::All.id(), cx),
        "F3: on close, the chip"
    );
    assert!(
        titled(&shell, draft, "Mainx", cx),
        "the draft still holds it"
    );
}

/// How P6 closes Settings.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Way {
    Escape,
    CloseButton,
    Scrim,
    QuitWithActivity,
    QuitWithoutActivity,
    QuitOverConfirmation,
}

/// Closes the open Settings `way`, for the ways that do not quit.
fn close(shell: &Shell, way: Way, cx: &mut TestAppContext) {
    match way {
        Way::Escape => shell.keys("escape", cx),
        Way::CloseButton => {
            shell.frame(cx, |window, cx| {
                window.within("sheet-content").click("close", cx)
            });
            shell.run(cx);
        }
        Way::Scrim => {
            // Left of the 720 px sheet, on its overlay.
            let mut visual = shell.visual(cx);
            visual.simulate_click(point(px(120.), px(600.)), Modifiers::default());
            visual.run_until_parked();
        }
        _ => unreachable!("{way:?} quits"),
    }
}

/// P6 (A3, R-10, R-28, R-29, R-52; A6 F11 to F14): Escape, the close button,
/// the scrim, Show in Activity, quitting with Activity displayed, quitting
/// with Activity hidden, quitting with a confirmation open inside Settings
/// and quitting with unkept edits each leave `Shown::Tabs`, no owner, no
/// active sheet, the host closed and a typed passphrase empty. Show in
/// Activity focuses the Activity list (F11); quitting focuses Quit now while
/// Activity is displayed (F12), else the origin (F13); with unkept edits the
/// EditsNotKept dialog holds focus, and its Cancel focuses the content
/// region as before (F14).
#[gpui_kit::test]
async fn settings_closes_every_way(cx: &mut TestAppContext) {
    // A repository waiting for an unfinished operation: its row on Checking
    // for updates offers Show in Activity.
    let rig = Rig::new(&[("a", true)], cx);
    rig.schedule(0, cx);
    rig.answer(
        0,
        Answer::Repository {
            recovery_required: true,
        },
        cx,
    );
    let shell = &rig.shell;
    for way in [Way::Escape, Way::CloseButton, Way::Scrim] {
        let inputs = open_on_keys(shell, cx);
        kept(shell, SettingsPage::Keys, cx);
        close(shell, way, cx);
        closed(shell, &format!("{way:?}"), cx);
        assert!(emptied(&inputs, cx), "{way:?}: the passphrase is emptied");
    }
    // Show in Activity (F11).
    let inputs = open_on_keys(shell, cx);
    shell.click(("settings-page", 2usize), cx);
    shell
        .until_drawn(cx, "the page's rows", |shell, cx| {
            shell.present(("polling-name", 0usize), cx)
        })
        .await;
    let row = rig.row("a", cx);
    shell.click(("polling-pending", row), cx);
    closed(shell, "Show in Activity", cx);
    assert!(emptied(&inputs, cx), "Show in Activity: the passphrase");
    let list = activity_list(shell, cx);
    assert!(holds_handle(shell, &list, cx), "F11: the Activity list");

    for way in [
        Way::QuitWithActivity,
        Way::QuitWithoutActivity,
        Way::QuitOverConfirmation,
    ] {
        let (shell, root) = self::shell(cx, |fixture, root| {
            fixture.commit_ticket(root, MAIN, "Main", "");
        });
        if way == Way::QuitWithoutActivity {
            shell.click(Control::Panel(PanelKind::Activity).id(), cx);
            assert!(!displayed(&shell, PanelKind::Activity, cx));
        }
        let (held, id) = hold_a_job(&shell, &root, cx);
        let origin = shell
            .frame(cx, |window, cx| window.focused(cx))
            .expect("something holds focus");
        let inputs = open_on_keys(&shell, cx);
        if way == Way::QuitOverConfirmation {
            shell.click(("settings-page", 0usize), cx);
            let rows = shell.rows("managed-row", cx);
            let ix = rows
                .iter()
                .position(|row| *row == key(&root))
                .expect("a is listed");
            shell.click(("managed-choose", ix), cx);
            shell.click("remove-submit", cx);
            assert!(has_dialog(&shell, cx), "the confirmation is open");
        }
        shell.keys("secondary-q", cx);
        let phase = shell
            .workspace
            .read_with(cx, |workspace, _| workspace.quit_phase());
        assert_eq!(phase, QuitPhase::Finishing(id), "{way:?}");
        closed(&shell, &format!("{way:?}"), cx);
        assert!(!has_dialog(&shell, cx), "{way:?}: no dialog is open");
        assert!(emptied(&inputs, cx), "{way:?}: the passphrase is emptied");
        if way == Way::QuitWithoutActivity {
            assert!(holds_handle(&shell, &origin, cx), "F13: the origin");
        } else {
            assert!(holds(&shell, "quit-now", cx), "F12: Quit now");
        }
        // The worker must not outlive the test: let the held job end.
        held.release();
        shell
            .until(cx, move |operations| {
                matches!(
                    operations.request(id).map(|request| &request.state),
                    Some(RequestState::Finished(..))
                )
            })
            .await;
    }

    #[cfg(unix)]
    quit_with_unkept_edits(cx).await;
}

/// P6's last row (F14): a tab's edit that no draft holds, crash recovery
/// being off (the drafts folder is a link, so the store does not open).
#[cfg(unix)]
async fn quit_with_unkept_edits(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), fixture.data_dir().join("drafts")).unwrap();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, MAIN, "Main", "");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&root, MAIN), cx);
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
    assert_eq!(draft(&shell, &root, cx), None, "no draft holds the edit");

    let inputs = open_on_keys(&shell, cx);
    shell.keys("secondary-q", cx);
    assert!(shell.present("edits-not-kept-dialog", cx), "quitting asks");
    closed(&shell, "QuitWithUnkeptEdits", cx);
    assert!(
        emptied(&inputs, cx),
        "unkept edits: the passphrase is emptied"
    );
    assert!(has_dialog(&shell, cx));
    assert!(
        in_context(&shell, "Dialog", cx),
        "F14: the EditsNotKept dialog"
    );
    shell.click("edits-not-kept-cancel", cx);
    assert!(!has_dialog(&shell, cx));
    let phase = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.quit_phase());
    assert_eq!(phase, QuitPhase::Running, "Cancel stays");
    settle(&shell, cx);
    assert!(
        shell.present(Region::Content.ring_id(), cx),
        "F14: Cancel focuses the content region, as before"
    );
}

/// P7 (A5, R-60, R-8; contract 7): with Settings open on Sharing & keys, the
/// panel and region actions, `FocusContent`, `NewItem`, the tab actions and
/// `OpenDrafts`, dispatched where focus is, change nothing; `OpenSettings`
/// leaves the page as it was.
#[gpui_kit::test]
fn nothing_acts_behind_settings(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, MAIN, "Main", "");
        fixture.commit_ticket(root, OTHER, "Other", "");
    });
    shell.select(&root, cx);
    open(&shell, ticket(&root, OTHER), cx);
    open(&shell, ticket(&root, MAIN), cx);
    shell.keys("secondary-,", cx);
    shell.click(("settings-page", 1usize), cx);
    assert_eq!(shown(&shell, cx), Shown::Settings(SettingsPage::Keys));

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
        requests: usize,
    }
    let footprint = |cx: &mut TestAppContext| {
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
            requests: shell.requests(cx, |_| true).len(),
        }
    };
    let before = footprint(cx);
    assert_eq!(before.owner, Some(SheetOwnerKind::Settings));
    let actions: [(&str, Box<dyn Action>); 15] = [
        ("FocusRepositories", FocusRepositories.boxed_clone()),
        ("FocusNavigation", FocusNavigation.boxed_clone()),
        ("FocusContent", FocusContent.boxed_clone()),
        ("FocusOperations", FocusOperations.boxed_clone()),
        ("NextRegion", NextRegion.boxed_clone()),
        ("PreviousRegion", PreviousRegion.boxed_clone()),
        ("ToggleLeftDock", ToggleLeftDock.boxed_clone()),
        ("ToggleRightDock", ToggleRightDock.boxed_clone()),
        ("ToggleBottomDock", ToggleBottomDock.boxed_clone()),
        ("NewItem", NewItem.boxed_clone()),
        ("NextTab", NextTab.boxed_clone()),
        ("PreviousTab", PreviousTab.boxed_clone()),
        ("CloseTab", CloseTab.boxed_clone()),
        ("OpenDrafts", OpenDrafts.boxed_clone()),
        ("OpenSettings", OpenSettings.boxed_clone()),
    ];
    for (name, action) in actions {
        shell
            .visual(cx)
            .update(|window, cx| window.dispatch_action(action, cx));
        shell.run(cx);
        assert_eq!(footprint(cx), before, "{name} changed something");
    }
}

/// P8 (B2, R-12, R-40; Part G item 1): PageDown from a menu entry and End
/// from a page control move Settings' own offset, not the content column's
/// or the item's; Tab to a control below the fold reveals it in Settings.
#[gpui_kit::test]
fn settings_scrolls_on_its_own(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, long_and_other);
    shell.resize_keeping_content(1440., SHORT, cx);
    shell.set_activity_height(SHORT_ACTIVITY, cx);
    open(&shell, ticket(&root, MAIN), cx);
    shell.keys("secondary-3", cx);
    shell.keys("pagedown", cx);
    let item = item_offset(&shell, cx);
    assert!(item < px(0.), "the item's region scrolled: {item:?}");
    let column = column_offset(&shell, cx);

    shell.keys("secondary-,", cx);
    assert_eq!(settings_offset(&shell, cx), px(0.));
    // PageDown, from the page's entry in the menu.
    shell.keys("pagedown", cx);
    let paged = settings_offset(&shell, cx);
    assert!(paged < px(0.), "PageDown scrolled Settings: {paged:?}");
    assert_eq!(column_offset(&shell, cx), column, "the column stays");
    assert_eq!(item_offset(&shell, cx), item, "the item stays");
    shell.keys("home", cx);
    assert_eq!(settings_offset(&shell, cx), px(0.), "Home, the top");
    // End, from a page control in view: the Add form's Browse.
    let host = host(&shell, cx);
    let browse = shell.visual(cx).update(|window, cx| {
        let page = host.update(cx, |host, cx| host.repositories(window, cx));
        page.read(cx).control_focus("add-browse")
    });
    shell.frame(cx, |window, cx| window.focus(&browse, cx));
    assert_eq!(settings_offset(&shell, cx), px(0.), "Browse is in view");
    shell.keys("end", cx);
    let bottom = settings_offset(&shell, cx);
    assert!(bottom < px(0.), "End scrolled Settings: {bottom:?}");
    assert_eq!(column_offset(&shell, cx), column);
    assert_eq!(item_offset(&shell, cx), item);
    shell.keys("home", cx);
    assert_eq!(settings_offset(&shell, cx), px(0.), "Home, the top");

    // Tab to a control below the fold: Settings brings it into view.
    tab_to_create(&shell, cx);
    let (viewport, submit) = shell.frame(cx, |window, _| {
        (
            window.find("settings-scroll").bounds(),
            window.find("create-submit").bounds(),
        )
    });
    assert!(
        submit.intersects(&viewport),
        "the focused control is in view: {submit:?} {viewport:?}"
    );
    assert!(settings_offset(&shell, cx) < px(0.));
    assert_eq!(column_offset(&shell, cx), column);
    assert_eq!(item_offset(&shell, cx), item);
}

/// P14 (C3, R-20, decision 5; A6 F11): with Activity hidden, Show in
/// Activity closes Settings, shows Activity and focuses its list, still so
/// after a draw.
#[gpui_kit::test]
async fn show_in_activity_closes_settings_and_focuses_activity(cx: &mut TestAppContext) {
    let rig = Rig::new(&[("a", true)], cx);
    rig.schedule(0, cx);
    rig.answer(
        0,
        Answer::Repository {
            recovery_required: true,
        },
        cx,
    );
    let shell = &rig.shell;
    shell.click(Control::Panel(PanelKind::Activity).id(), cx);
    assert!(
        !displayed(shell, PanelKind::Activity, cx),
        "Activity hidden"
    );
    rig.open(1, cx).await;
    let row = rig.row("a", cx);
    shell.click(("polling-pending", row), cx);
    closed(shell, "Show in Activity", cx);
    assert!(
        displayed(shell, PanelKind::Activity, cx),
        "Activity is shown"
    );
    let list = activity_list(shell, cx);
    assert!(holds_handle(shell, &list, cx), "F11: the Activity list");
    assert!(holds_handle(shell, &list, cx), "still so after a draw");
}

/// P15 (R-9, R-29): a route raised under an open dialog, and one raised
/// while quitting, open nothing.
#[gpui_kit::test]
async fn settings_does_not_open_behind_a_dialog_or_while_quitting(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, MAIN, "Main", "");
    });

    // Under a dialog: Share…'s route to Remotes.
    shell
        .visual(cx)
        .update(|window, cx| window.open_dialog(cx, |dialog, _, _| dialog.title("Probe")));
    shell.run(cx);
    route(&shell, cx, |workspace, window, cx| {
        workspace.open_remotes(root.clone(), window, cx)
    });
    assert_eq!(shown(&shell, cx), Shown::Tabs, "under a dialog");
    assert_eq!(owner(&shell, cx), None);
    assert!(!has_sheet(&shell, cx));
    shell
        .visual(cx)
        .update(|window, cx| window.close_dialog(cx));
    shell.run(cx);

    // While quitting: the footer's route to Checking for updates.
    let (held, id) = hold_a_job(&shell, &root, cx);
    shell.keys("secondary-q", cx);
    let phase = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.quit_phase());
    assert_eq!(phase, QuitPhase::Finishing(id));
    route(&shell, cx, |workspace, window, cx| {
        workspace.open_settings(SettingsPage::Polling, window, cx)
    });
    assert_eq!(shown(&shell, cx), Shown::Tabs, "while quitting");
    assert_eq!(owner(&shell, cx), None);
    assert!(!has_sheet(&shell, cx));
    assert!(holds(&shell, "quit-now", cx), "Quit now keeps focus");

    held.release();
    shell
        .until(cx, move |operations| {
            matches!(
                operations.request(id).map(|request| &request.state),
                Some(RequestState::Finished(..))
            )
        })
        .await;
}

/// P5 (design A6 F4 to F10; Part D's origins; Ruling 8): Settings closed by
/// Escape gives focus back to what opened it, by each origin of Part D's
/// table, read after two draws. The rows are in `returns` and
/// `returns_activity`.
#[gpui_kit::test]
async fn focus_returns_to_what_opened_settings(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let mut table = table::Table::new("P5");
    returns::rows(&mut table, cx).await;
    returns_activity::rows(&mut table, cx);
    table.done();
}

/// P22 (design A6 F17; A2 step 2; A4): work that lands after Settings
/// closed moves no focus and opens nothing. The rows are in `late`.
#[gpui_kit::test]
fn late_work_moves_no_focus(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let mut table = table::Table::new("P22");
    late::rows(&mut table, cx);
    table.done();
}

/// P23 (design A6 F15, F16, F18 and F19; Part H): focus stays inside the
/// open sheet. The rows are in `inside`.
#[gpui_kit::test]
async fn focus_stays_inside_settings(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let mut table = table::Table::new("P23");
    inside::rows(&mut table, cx).await;
    table.done();
}

/// P4 (design Part D; A6 F1 and F2; R-14, R-22, R-23, R-24, R-53, R-55):
/// the ten routes through `show_settings`, and each way in no other test
/// raises through its own handler, land on their page, with focus on the
/// control Part D names after two draws and the repository Settings works
/// on. The rows are in `routes` and `ways`; Ruling 3's F1 and F2 rows are
/// among them.
#[gpui_kit::test]
async fn each_route_lands_on_its_page_and_control(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let mut table = table::Table::new("P4");
    routes::rows(&mut table, cx).await;
    ways::rows(&mut table, cx).await;
    table.done();
}

/// P20 (contract 5; design Part D): one trigger of each kind, focused and
/// pressed with Enter, lands as Part D says. The rows are in `keyboard`.
#[gpui_kit::test]
async fn routes_by_keyboard(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let mut table = table::Table::new("P20");
    keyboard::rows(&mut table, cx).await;
    table.done();
}

/// Design A6 F16 (fix round 1): a landing that a view behind the open sheet
/// applies in its own render, Drafts' Discard… focusing a neighbour row,
/// leaves focus on the page's entry, read after GPUI's own draws only. The
/// rows are in `behind`.
#[gpui_kit::test]
fn a_landing_drawn_behind_settings_leaves_focus_inside(cx: &mut TestAppContext) {
    behind::landing(cx);
}
