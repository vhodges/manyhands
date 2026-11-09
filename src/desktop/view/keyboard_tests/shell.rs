//! The workspace shell's keys (Part F), by keystrokes: the dock keys, the
//! region keys and Find revealing a hidden panel, F6 over the panels
//! displayed, the footer reached by Tab and pressed by Enter and Space, the
//! dock menu opened by `menu` and `shift-f10`, and Needs attention by Enter.
//!
//! The windows of these tests are inactive. A toggle is hidden by a click on
//! the footer before the key under test, since a click is how the person
//! hides one; the key is then the only way back.
//!
//! The key name `menu` is inferred from xkb (Cycle U-3): GPUI's parser takes
//! any name, so these tests send `menu` as written, and the real name under
//! Wayland is on the on-screen checklist.

use gpui_kit::{Focusable, TestAppContext};
use manyhands::results::Outcome;

use super::{
    Walk,
    editing::{BODY, EDITED, edit_by_e, editable, text},
};
use crate::desktop::{
    model::{
        editor_text::load,
        layout::{PanelKind, Side},
        navigation::Pane,
        stamp::SettingsPage,
    },
    view::{
        footer::{
            Control, plan,
            tests::support::{finish, focused, put_behind, read, saves},
        },
        settings::Shown,
        workspace::Region,
    },
};

mod support;

use PanelKind::{Activity, Documents, Repositories, Tickets};
use support::*;

/// The three dock keys toggle their docks, each way, and nothing else (P17).
#[gpui_kit::test]
fn dock_keys_toggle_each_dock(cx: &mut TestAppContext) {
    let walk = plain(cx);
    for (keys, side) in [
        ("secondary-b", Side::Left),
        ("secondary-alt-b", Side::Right),
        ("secondary-j", Side::Bottom),
    ] {
        let others = Side::ALL.into_iter().filter(|each| *each != side);
        for open in [false, true] {
            let (before, saved) = (layout_of(&walk, cx), saves(&walk.shell, cx));
            walk.keys(keys, cx);
            let after = layout_of(&walk, cx);
            assert_eq!(after.is_open(side), open, "{keys}");
            for other in others.clone() {
                assert_eq!(after.is_open(other), before.is_open(other), "{keys}");
            }
            assert_eq!(
                read(&walk.shell, Control::Dock(side), cx),
                Some(said(Control::Dock(side), &after, 0)),
                "{keys}: the toggle follows"
            );
            assert!(
                saves(&walk.shell, cx) > saved,
                "{keys}: a save was scheduled"
            );
            if !open && side == Side::Left {
                // Focus was in the left dock, which is gone (R-9).
                ringed_on(&walk, Region::Content, cx);
            }
        }
    }
}

/// Ctrl B is Bold while an editor has focus, and the left dock is left
/// alone; once focus leaves the editor it toggles the dock (decision 2).
#[gpui_kit::test]
async fn ctrl_b_in_an_editor_is_bold(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let walk = editable(&[EDITED], cx);
    walk.select_a_repository(cx);
    let host = edit_by_e(&walk, EDITED, cx).await;
    // "two" selected: the caret, drawn and focused, is on the first line.
    walk.keys(
        "home right right right right shift-right shift-right shift-right",
        cx,
    );
    let before = layout_of(&walk, cx);

    walk.keys("secondary-b", cx);
    assert!(text(&host, cx).contains("**two**"), "Bold marked the text");
    assert_eq!(layout_of(&walk, cx), before, "the left dock is unchanged");
    walk.keys("secondary-b", cx);
    assert_eq!(text(&host, cx), load(BODY), "Bold again");
    assert_eq!(layout_of(&walk, cx), before);

    // Escape first: focus is on Done, outside the editor, and the key is
    // the dock's.
    walk.escape_to_done(cx);
    walk.keys("secondary-b", cx);
    assert!(
        !layout_of(&walk, cx).is_open(Side::Left),
        "the dock toggles"
    );
    assert_eq!(text(&host, cx), load(BODY), "text untouched");
}

/// How a panel is kept off the screen before its key.
#[derive(Clone, Copy)]
enum Away {
    /// Its footer toggle is clicked.
    Off,
    /// Its dock is closed by the key given.
    DockClosed(&'static str),
    /// The hidden panel is dropped behind `anchor`'s tab by the Kit's own
    /// edit: it stays on, and is not the active tab.
    Behind(PanelKind),
    /// `by` is dropped into the hidden panel's tab group by the Kit's own
    /// edit, and takes the active tab.
    Covered(PanelKind),
}

/// The region keys and Find reveal a panel that is off, in a closed dock or
/// an inactive tab, and focus it; `secondary-2` follows the list used last
/// (P13, P15).
#[gpui_kit::test]
fn region_keys_and_find_reveal_hidden_panels(cx: &mut TestAppContext) {
    let walk = plain(cx);
    // (the list used last, how it is hidden, the panel, the key)
    let rows = [
        (None, Away::Off, Repositories, "secondary-1"),
        (None, Away::Off, Documents, "secondary-2"),
        // Documents behind Repositories' tab: `secondary-2` must select it
        // (a reveal that only turned panels on would leave it unseen).
        (None, Away::Behind(Repositories), Documents, "secondary-2"),
        // The left dock closed: `secondary-2` must open it (a reveal that
        // left the dock shut would focus a panel nobody sees).
        (
            None,
            Away::DockClosed("secondary-b"),
            Documents,
            "secondary-2",
        ),
        // From here the list used last is Tickets.
        (Some(Pane::Tickets), Away::Off, Tickets, "secondary-2"),
        (None, Away::Off, Activity, "secondary-4"),
        (
            None,
            Away::DockClosed("secondary-j"),
            Activity,
            "secondary-4",
        ),
        (None, Away::Behind(Documents), Repositories, "secondary-1"),
        // Activity behind Repositories' tab in the bottom dock:
        // `secondary-4` must select Activity's tab.
        (None, Away::Covered(Repositories), Activity, "secondary-4"),
        (None, Away::Off, Tickets, "secondary-p"),
        (None, Away::Covered(Documents), Tickets, "secondary-p"),
    ];
    for (row, (list, away, hidden, keys)) in rows.into_iter().enumerate() {
        let why = format!("row {row}: {keys}, {hidden:?}");
        if let Some(pane) = list {
            walk.shell.use_list(pane, cx);
        }
        match away {
            Away::Off => walk.shell.click(Control::Panel(hidden).id(), cx),
            Away::DockClosed(keys) => walk.keys(keys, cx),
            Away::Behind(anchor) => put_behind(&walk.shell, hidden, anchor, cx),
            Away::Covered(by) => put_behind(&walk.shell, by, hidden, cx),
        }
        let region = region_of(hidden);
        assert!(!layout_of(&walk, cx).is_displayed(hidden), "{why}");
        assert!(!walk.shell.present(region.id(), cx), "{why}: it is gone");
        let saved = saves(&walk.shell, cx);

        walk.keys(keys, cx);
        assert!(layout_of(&walk, cx).is_displayed(hidden), "{why}: revealed");
        assert!(walk.shell.present(region.id(), cx), "{why}: drawn");
        assert!(
            saves(&walk.shell, cx) > saved,
            "{why}: a save was scheduled"
        );
        ringed_on(&walk, region, cx);
        if keys == "secondary-p" {
            let input = walk
                .shell
                .workspace
                .read_with(cx, |w, _| w.find_input().clone());
            let focus = input.read_with(cx, Focusable::focus_handle);
            assert!(
                walk.shell.frame(cx, |window, _| focus.is_focused(window)),
                "{why}: Find"
            );
        }
    }
}

/// F6 and Shift F6 stop at the panels displayed only: not at a panel that is
/// off, one in a closed dock or one behind another tab (P14, decision 9).
#[gpui_kit::test]
fn f6_skips_hidden_panels(cx: &mut TestAppContext) {
    use Region::{Documents as D, Operations as O, Repositories as R, Tickets as T};
    let walk = plain(cx);
    // F6 from the item area: the right dock, the bottom dock, then round to
    // the left dock.
    let all = [T, O, R, D];
    let expect = |walk: &Walk, stops: &[Region], why: &str, cx: &mut TestAppContext| {
        assert_eq!(tour(walk, "f6", cx), stops, "{why}");
        let mut back: Vec<Region> = stops.to_vec();
        back.reverse();
        assert_eq!(tour(walk, "shift-f6", cx), back, "{why}, backwards");
    };
    expect(&walk, &all, "everything displayed", cx);

    walk.keys("secondary-j", cx);
    expect(&walk, &[T, R, D], "the bottom dock closed", cx);
    walk.keys("secondary-alt-b", cx);
    expect(&walk, &[R, D], "the right dock closed too", cx);
    walk.keys("secondary-j secondary-alt-b", cx);
    expect(&walk, &all, "both docks open again", cx);

    walk.shell.click(Control::Panel(Documents).id(), cx);
    expect(&walk, &[T, O, R], "Documents off", cx);
    walk.shell.click(Control::Panel(Documents).id(), cx);
    put_behind(&walk.shell, Repositories, Documents, cx);
    assert!(!layout_of(&walk, cx).is_displayed(Repositories));
    expect(&walk, &[T, O, D], "Repositories behind Documents' tab", cx);
}

/// Tab reaches a panel toggle; Enter hides its panel, Space shows it, and
/// focus stays on the toggle. A dock toggle takes Enter, and so does
/// Settings (A2).
#[gpui_kit::test]
fn a_panel_toggle_works_by_tab_and_enter(cx: &mut TestAppContext) {
    let walk = plain(cx);
    let toggle = Control::Panel(Tickets);
    reach(&walk, toggle, cx);
    assert_eq!(
        read(&walk.shell, toggle, cx),
        Some(("Hide Tickets".to_owned(), Some(true)))
    );

    walk.shell.press("enter", cx);
    assert!(
        !walk.shell.present(Region::Tickets.id(), cx),
        "Enter hides it"
    );
    assert_eq!(
        read(&walk.shell, toggle, cx),
        Some(("Show Tickets".to_owned(), Some(false)))
    );
    on_control(&walk, toggle, cx);

    walk.shell.press("space", cx);
    assert!(
        walk.shell.present(Region::Tickets.id(), cx),
        "Space shows it"
    );
    assert_eq!(
        read(&walk.shell, toggle, cx),
        Some(("Hide Tickets".to_owned(), Some(true)))
    );
    on_control(&walk, toggle, cx);

    // A dock toggle, by Enter: Activity goes with the bottom dock.
    let dock = Control::Dock(Side::Bottom);
    reach(&walk, dock, cx);
    walk.shell.press("enter", cx);
    assert!(!layout_of(&walk, cx).is_open(Side::Bottom));
    assert_eq!(
        read(&walk.shell, dock, cx),
        Some(("Show bottom dock".to_owned(), Some(false)))
    );
    on_control(&walk, dock, cx);

    // Settings, by Enter, as `secondary-,` does.
    reach(&walk, Control::Settings, cx);
    walk.shell.press("enter", cx);
    assert_eq!(
        walk.settings_shown(cx),
        Shown::Settings(SettingsPage::Repositories)
    );
}

/// `menu` and `shift-f10` open the dock menu of a focused panel toggle, with
/// focus in it; the arrows and Enter choose a side, and the panel moves
/// there with focus on its toggle. Escape closes the menu with focus back on
/// the toggle (P19, decision 8). The key does nothing off a toggle.
#[gpui_kit::test]
fn each_side_is_reached_by_the_dock_menu_keys(cx: &mut TestAppContext) {
    let walk = plain(cx);
    let menu_open = |walk: &Walk, cx: &mut TestAppContext| walk.shell.present("popup-menu", cx);
    // The toggle is focused as Tab does (the other tests walk there); the
    // menu then follows the keys alone.
    let focus = |walk: &Walk, kind: PanelKind, cx: &mut TestAppContext| {
        let handle = walk.shell.workspace.read_with(cx, |workspace, _| {
            workspace.footer_focus(Control::Panel(kind))
        });
        walk.shell.frame(cx, |window, cx| handle.focus(window, cx));
        walk.shell.run(cx);
        on_control(walk, Control::Panel(kind), cx);
    };

    // Not on a toggle: nothing opens.
    walk.keys("menu shift-f10", cx);
    assert!(!menu_open(&walk, cx));

    // (the panel, the key, the arrows to its side's item, the side)
    let moves = [
        (Tickets, "menu", "down", Side::Left),
        (Documents, "shift-f10", "down down down", Side::Bottom),
        (Activity, "shift-f10", "down down", Side::Right),
    ];
    for (panel, open, arrows, side) in moves {
        let why = format!("{panel:?} by {open} to {side:?}");
        focus(&walk, panel, cx);
        walk.keys(open, cx);
        assert!(menu_open(&walk, cx), "{why}: the menu opens");
        assert!(
            focused(&walk.shell, "popup-menu", cx),
            "{why}: focus is in it"
        );
        walk.keys(arrows, cx);
        walk.keys("enter", cx);

        assert!(!menu_open(&walk, cx), "{why}: a choice closes it");
        let after = layout_of(&walk, cx);
        assert_eq!(after.side_of(panel), Some(side), "{why}");
        assert!(after.is_displayed(panel), "{why}: revealed");
        assert!(
            plan(&after, 0)
                .left
                .iter()
                .chain(&plan(&after, 0).right)
                .any(|group| group.contains(&Control::Panel(panel))),
            "{why}: its toggle is in the footer"
        );
        on_control(&walk, Control::Panel(panel), cx);
    }

    // Escape: nothing moves, and focus is on the toggle.
    focus(&walk, Tickets, cx);
    let before = layout_of(&walk, cx);
    walk.keys("shift-f10", cx);
    assert!(menu_open(&walk, cx));
    walk.keys("escape", cx);
    assert!(!menu_open(&walk, cx));
    assert_eq!(layout_of(&walk, cx), before);
    on_control(&walk, Control::Panel(Tickets), cx);
}

/// Needs attention takes Enter: Activity is revealed and focused, whether
/// its panel is off or its dock is closed (P20).
#[gpui_kit::test]
fn needs_attention_works_by_enter(cx: &mut TestAppContext) {
    let walk = plain(cx);
    finish(&walk.shell, 1, Outcome::Error, cx);
    assert_eq!(
        read(&walk.shell, Control::Attention, cx),
        Some(("1 needs attention".to_owned(), None))
    );
    for (row, away) in [Away::Off, Away::DockClosed("secondary-j")]
        .into_iter()
        .enumerate()
    {
        match away {
            Away::DockClosed(keys) => walk.keys(keys, cx),
            _ => walk.shell.click(Control::Panel(Activity).id(), cx),
        }
        assert!(!layout_of(&walk, cx).is_displayed(Activity), "row {row}");
        assert!(
            !walk.shell.present(Region::Operations.id(), cx),
            "row {row}"
        );

        reach(&walk, Control::Attention, cx);
        walk.shell.press("enter", cx);
        assert!(layout_of(&walk, cx).is_displayed(Activity), "row {row}");
        assert!(walk.shell.present(Region::Operations.id(), cx), "row {row}");
        ringed_on(&walk, Region::Operations, cx);
    }
}

/// Tab reaches every footer control in the order it is drawn, with focus
/// shown and the name and toggled state a screen reader reads (P29).
#[gpui_kit::test]
fn footer_controls_are_reached_by_tab(cx: &mut TestAppContext) {
    let walk = plain(cx);
    finish(&walk.shell, 1, Outcome::Error, cx);
    // A toggle that is off reads "Show".
    walk.shell.click(Control::Panel(Documents).id(), cx);
    let layout = layout_of(&walk, cx);
    let footer = plan(&layout, 1);
    let order: Vec<Control> = footer
        .left
        .into_iter()
        .chain(footer.right)
        .flatten()
        .collect();
    assert_eq!(order.len(), Control::ALL.len(), "every control is shown");

    walk.keys("secondary-3", cx);
    for (at, control) in order.iter().copied().enumerate() {
        if at == 0 {
            walk.tab_to("tab", control.id(), cx);
        } else {
            walk.keys("tab", cx);
        }
        on_control(&walk, control, cx);
        assert_eq!(
            read(&walk.shell, control, cx),
            Some(said(control, &layout, 1)),
            "{control:?}"
        );
    }
}
