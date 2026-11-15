//! The Add repository sheet as the workspace keeps it (Wave 04 Cycle 05a
//! design B1 to B3, B9, B10's "Adding", B11, B12), over real repositories,
//! written before its code (Task 4's design checkpoint).
//!
//! The workspace keeps the sheet (P3); every way in opens it on `add-path`
//! (P4); its closes give focus back to what opened it (P5) or to Settings
//! when it came from there (P6); it closes every way (P7); late work moves
//! no focus (P8); nothing acts behind it (P15); it scrolls on its own
//! (P23). B9's rows, except G16 and G22 (Task 5) and G20 (Task 6), are
//! tables in files of their own, run from here so that each test is named
//! here.
//!
//! A focus B9 names is read after two draws (Cycle 04's A6 rule). Guards
//! are tested with dispatched actions, which the scrim does not take. No
//! real waits: the Kit's deferred restore is reached by the test clock. A
//! Kit button a row lands on is read on the button itself
//! (`holds_control`).

use gpui_kit::{Action, FocusHandle, TestAppContext, component::WindowExt, px};

use super::super::settings_sheet::SheetOwnerKind;
use crate::desktop::{
    keys::{
        CloseTab, FocusContent, FocusNavigation, FocusOperations, FocusRepositories, NewItem,
        NextRegion, NextTab, OpenDrafts, OpenSettings, PreviousRegion, PreviousTab,
        ToggleBottomDock, ToggleLeftDock, ToggleRightDock,
    },
    model::{layout::Layout, stamp::TabId, tabs::TabKey},
    test_support::fixtures::{ticket_path, ticket_source},
    view::{item::ChipField, settings::Shown, workspace::add_sheet::AddRoute},
};

#[path = "add_sheet_tests/behind.rs"]
mod behind;
#[path = "add_sheet_tests/closes.rs"]
mod closes;
#[path = "add_sheet_tests/late.rs"]
mod late;
#[path = "add_sheet_tests/returns.rs"]
mod returns;
#[path = "add_sheet_tests/settings.rs"]
mod settings;
#[path = "add_sheet_tests/support.rs"]
mod support;
#[path = "add_sheet_tests/ways.rs"]
mod ways;

use support::*;

/// P3 (B1; G2): the owner is `AddRepository` and a sheet is active frame
/// after frame, with an item shown under it and after `open_item` lands;
/// a details sheet asked for while it is open never opens. G2, by a direct
/// call: opened over the details sheet, the sheet takes its place, focus
/// on `add-path`; on close, the chip.
#[gpui_kit::test]
async fn add_repository_is_a_sheet_the_workspace_keeps(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let mut table = Table::new("P3");
    kept_over_items(&mut table, cx);
    over_the_details_sheet(&mut table, cx).await;
    table.done();
}

fn kept_over_items(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, root) = one_repo(cx, |fixture, root| {
        fixture.commit_ticket(root, MAIN, "Main", "project: p\n");
        fixture.commit_ticket(root, OTHER, "Other", "");
    });
    shell.select(&root, cx);
    open(&shell, ticket(&root, MAIN), cx);
    press_plus(&shell, cx);
    let row = "B1 case 1 kept with an item shown";
    if !opened(table, &shell, row, cx) {
        return;
    }
    let kept = |shell: &crate::desktop::view::harness::Shell, cx: &mut TestAppContext| {
        (0..3).all(|_| {
            let open = add_open(shell, cx);
            shell.run(cx);
            open
        })
    };
    table.fact(row, kept(&shell, cx), "kept frame after frame");
    open(&shell, ticket(&root, OTHER), cx);
    let opened_other = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.opened(cx));
    table.fact(
        "B1 case 1 kept after open_item",
        kept(&shell, cx) && opened_other == Some(ticket(&root, OTHER)),
        "the sheet kept, the other item shown under it",
    );
    // B1 case 4: a details sheet asked for never opens under it.
    let pane = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .expect("an item tab is shown");
    shell.workspace.update(cx, |workspace, cx| {
        workspace.ask_details(&pane, ChipField::Project, cx)
    });
    shell.run(cx);
    table.fact(
        "B1 case 4 a details sheet asked for",
        add_open(&shell, cx) && !shell.present("sheet-heading", cx),
        "no details sheet opening, the Add sheet kept",
    );
    shell.keys("escape", cx);
    table.fact(
        "B1 case 4 nor once the sheet closes",
        all_closed(&shell, cx) && !shell.present("sheet-heading", cx),
        "no details sheet",
    );
}

/// G2: the details sheet open on its Title, edited; a direct call opens the
/// Add sheet in its place (R-21).
async fn over_the_details_sheet(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, root) = one_repo(cx, |fixture, root| {
        fixture.commit_ticket(root, MAIN, "Main", "project: p\n");
    });
    open(&shell, ticket(&root, MAIN), cx);
    shell.click("item-edit", cx);
    shell.until_editing(cx).await;
    shell.click(ChipField::All.id(), cx);
    assert_eq!(owner(&shell, cx), Some(SheetOwnerKind::Details));
    route(&shell, cx, |workspace, window, cx| {
        workspace.show_add_repository(AddRoute::New, None, window, cx)
    });
    let row = "G2 opened over the details sheet";
    if !opened(table, &shell, row, cx) {
        return;
    }
    table.fact(
        "G2 the details sheet gave way",
        !shell.present("sheet-heading", cx),
        "the details sheet gone",
    );
    let held = holds(&shell, "add-path", cx);
    table.row(&shell, &format!("{row}: G1"), held, "add-path", cx);
    shell.keys("escape", cx);
    let held = all_closed(&shell, cx) && holds(&shell, ChipField::All.id(), cx);
    table.row(&shell, "G2 on close", held, "the chip", cx);
}

/// P4 (B2; G1): AW-1 to AW-5, AW-5 three ways through `choose_activity`,
/// open the sheet in the existing-folder mode with focus on `add-path`; an
/// `Inspect` route fills and checks the folder, also when the sheet is
/// already open (the origin kept), and leaves Settings' chosen repository
/// alone. The rows are in `ways`.
#[gpui_kit::test]
async fn each_way_in_opens_the_add_sheet(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let mut table = Table::new("P4");
    ways::rows(&mut table, cx).await;
    table.done();
}

/// P5 (G7 to G11, G13, G14, G23): a close gives focus back to what opened
/// the sheet, one row per origin kind; the confirmation's close gives it
/// back inside the sheet; a success that closes the confirmation in the
/// same batch, or that lands under another dialog, closes the sheet only
/// once no dialog is open. The rows are in `returns`.
#[gpui_kit::test]
async fn focus_returns_to_what_opened_add(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let mut table = Table::new("P5");
    returns::rows(&mut table, cx).await;
    table.done();
}

/// P6 (G3 to G6, G17, G21; R-22): opened from Settings, the sheet replaces
/// it and empties a typed passphrase; each close returns to Settings, a
/// success with the new repository chosen; Settings' own close returns to
/// its first origin; a later open by `+` does not return to Settings; a
/// direct route into Settings closes the sheet quietly. The rows are in
/// `settings`.
#[gpui_kit::test]
async fn add_from_settings_returns_to_settings(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let mut table = Table::new("P6");
    settings::rows(&mut table, cx).await;
    table.done();
}

/// P7 (B3; G15; decision 16): every close leaves no owner, no sheet, the
/// open flag down and no pending Add; Kept keeps the typed text and Added
/// clears it; quitting closes it quietly, as Cycle 04's F12 to F14. The
/// rows are in `closes`.
#[gpui_kit::test]
async fn the_add_sheet_closes_every_way(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let mut table = Table::new("P7");
    closes::rows(&mut table, cx).await;
    table.done();
}

/// P8 (G12; R-26): a check, a Browse answer or an add that lands after the
/// sheet closed moves no focus and opens nothing; a success for an earlier
/// open does not close a reopened sheet. The rows are in `late`.
#[gpui_kit::test]
async fn late_work_moves_no_focus_after_add_closes(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let mut table = Table::new("P8");
    late::rows(&mut table, cx).await;
    table.done();
}

/// P15 (B12; R-14, R-24; G18, G19): dispatched workspace actions, Ctrl+,
/// among them, change nothing while the sheet is open; a landing that would
/// focus a control behind it puts focus on `add-path`; a landing that focuses
/// nothing behind it leaves focus where it was. The landing rows are in
/// `behind`.
#[gpui_kit::test]
async fn nothing_acts_behind_the_add_sheet(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let mut table = Table::new("P15");
    dispatched(&mut table, cx);
    behind::rows(&mut table, cx).await;
    table.done();
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
    requests: usize,
}

fn footprint(shell: &crate::desktop::view::harness::Shell, cx: &mut TestAppContext) -> Footprint {
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
}

/// B12, R-60, R-14: the panel and region actions, `FocusContent`,
/// `NewItem`, the tab actions, `OpenDrafts` and `OpenSettings`, dispatched
/// where focus is, change nothing.
fn dispatched(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, root) = one_repo(cx, |fixture, root| {
        fixture.commit_ticket(root, MAIN, "Main", "");
        fixture.commit_ticket(root, OTHER, "Other", "");
    });
    shell.select(&root, cx);
    open(&shell, ticket(&root, OTHER), cx);
    open(&shell, ticket(&root, MAIN), cx);
    press_plus(&shell, cx);
    if !opened(table, &shell, "P15 dispatched actions", cx) {
        return;
    }
    let before = footprint(&shell, cx);
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
        // R-14: Ctrl+, does nothing while the Add sheet is open.
        ("OpenSettings", OpenSettings.boxed_clone()),
    ];
    for (name, action) in actions {
        shell
            .visual(cx)
            .update(|window, cx| window.dispatch_action(action, cx));
        shell.run(cx);
        let now = footprint(&shell, cx);
        table.fact(
            &format!("P15 {name} behind the sheet"),
            now == before,
            &format!("nothing changed (now {now:?})"),
        );
    }
}

/// P23 (B12; R-8): PageDown and End from a control in the sheet's body move
/// the sheet's own offset, not the item's or the content column's.
#[gpui_kit::test]
fn the_add_sheet_scrolls_on_its_own(cx: &mut TestAppContext) {
    let mut table = Table::new("P23");
    let (shell, root) = one_repo(cx, |fixture, root| {
        let body: String = (0..200)
            .map(|line| format!("Line {line} of a long body.\n"))
            .collect();
        let source = format!("{}{body}", ticket_source(MAIN, "Main", "project: p\n"));
        fixture.commit_file(root, &ticket_path(MAIN), source.as_bytes());
    });
    // Shorter than Task 4's 250: with one mode drawn (B5) the body is
    // shorter, and must still overflow the sheet (C-7).
    shell.resize_keeping_content(1440., 150., cx);
    shell.set_activity_height(100., cx);
    shell.select(&root, cx);
    open(&shell, ticket(&root, MAIN), cx);
    shell.keys("secondary-3", cx);
    shell.keys("pagedown", cx);
    let item = |cx: &mut TestAppContext| {
        shell.frame(cx, |_, _| ());
        shell
            .workspace
            .read_with(cx, |workspace, cx| workspace.item_offset(cx))
    };
    let column = |cx: &mut TestAppContext| {
        shell.frame(cx, |_, _| ());
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.content_offset())
    };
    let (item_before, column_before) = (item(cx), column(cx));
    assert!(item_before < px(0.), "the item's region scrolled");
    press_plus(&shell, cx);
    if !opened(&mut table, &shell, "P23", cx) {
        table.done();
        return;
    }
    let offset = |cx: &mut TestAppContext| {
        shell.frame(cx, |_, _| ());
        of_sheet(&shell, cx, |sheet| sheet.scroll_offset()).unwrap_or_default()
    };
    table.fact(
        "P23 the body starts at its top",
        offset(cx) == px(0.),
        "offset 0",
    );
    // From a button in the body, which takes no key itself.
    focus_control(&shell, "add-browse", cx);
    for key in ["pagedown", "end"] {
        shell.keys("home", cx);
        shell.keys(key, cx);
        let moved = offset(cx);
        table.fact(
            &format!("P23 {key} moves the sheet's body"),
            moved < px(0.),
            &format!("the sheet's offset below 0 (it is {moved:?})"),
        );
        table.fact(
            &format!("P23 {key} leaves the item and the column"),
            item(cx) == item_before && column(cx) == column_before,
            "the item's and the column's offsets unchanged",
        );
    }
    table.done();
}
