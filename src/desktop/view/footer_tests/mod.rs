//! The footer's controls (Part E), one test per kind of control: a panel
//! toggle, a dock toggle, the dock menu by secondary click, Needs attention
//! and Settings (P3, P6, P8, P9, P20, P21), a control refused behind a
//! dialog (P18), and the names, which are a pure function (R-24).
//!
//! A toggle's name and state are read as a screen reader reads them: the
//! snapshot's `label()` and `checked()`.

use gpui_kit::{
    InputEvent, Modifiers, MouseButton, MouseDownEvent, TestAppContext, component::WindowExt,
    test::TestWindowExt,
};
use manyhands::results::Outcome;

use super::{
    Control, FooterStatus, attention_text, dock_toggle_name, menu, panel_toggle_name, plan,
};
use crate::desktop::{
    keys::{DockPanel, OpenDockMenu},
    model::{
        layout::{Layout, PanelKind, Side},
        stamp::SettingsPage,
    },
    test_support::fixtures::Fixture,
    view::{harness::Shell, settings::Shown, workspace::Region},
};

mod status;
mod status_table;
pub(in crate::desktop::view) mod support;

use PanelKind::{Activity, Documents, Repositories, Tickets};
use support::*;

/// A panel toggle reads on only when its panel is displayed, and a click
/// hides it or reveals it, a panel behind another tab included (P2, P3, R-4).
#[gpui_kit::test]
fn a_panel_toggle_hides_and_reveals_its_panel(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    let toggle = Control::Panel(Tickets);
    assert_eq!(
        read(&shell, toggle, cx),
        Some(("Hide Tickets".to_owned(), Some(true)))
    );
    let saved = saves(&shell, cx);

    shell.click(toggle.id(), cx);
    assert!(
        !shell.present(Region::Tickets.id(), cx),
        "the panel is gone"
    );
    assert_eq!(
        read(&shell, toggle, cx),
        Some(("Show Tickets".to_owned(), Some(false)))
    );
    assert!(saves(&shell, cx) > saved, "a save was scheduled");
    // The toggles of the others, and the dock's, are as they were.
    assert_eq!(
        read(&shell, Control::Panel(Activity), cx),
        Some(("Hide Activity".to_owned(), Some(true)))
    );

    shell.click(toggle.id(), cx);
    assert!(shell.present(Region::Tickets.id(), cx));
    assert_eq!(
        read(&shell, toggle, cx),
        Some(("Hide Tickets".to_owned(), Some(true)))
    );

    // Documents behind Repositories is on, and reads off: a click reveals
    // it, and then Repositories is the one behind.
    put_behind(&shell, Documents, Repositories, cx);
    let (documents, repositories) = (Control::Panel(Documents), Control::Panel(Repositories));
    assert!(layout(&shell, cx).is_on(Documents));
    assert_eq!(
        read(&shell, documents, cx),
        Some(("Show Documents".to_owned(), Some(false)))
    );
    assert_eq!(
        read(&shell, repositories, cx),
        Some(("Hide Repositories".to_owned(), Some(true)))
    );
    shell.click(documents.id(), cx);
    assert!(shell.present(Region::Documents.id(), cx));
    assert_eq!(
        read(&shell, documents, cx),
        Some(("Hide Documents".to_owned(), Some(true)))
    );
    assert_eq!(
        read(&shell, repositories, cx),
        Some(("Show Repositories".to_owned(), Some(false)))
    );
}

/// A dock toggle reads on while its dock is open, closing the dock leaves
/// its panels on, and opening a dock whose panels are all off turns them on
/// (P6, R-4).
#[gpui_kit::test]
fn a_dock_toggle_reveals_its_hidden_panels(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    let (left, right) = (Control::Dock(Side::Left), Control::Dock(Side::Right));
    assert_eq!(
        read(&shell, left, cx),
        Some(("Hide left dock".to_owned(), Some(true)))
    );

    // Closing keeps what is on.
    shell.click(right.id(), cx);
    assert_eq!(
        read(&shell, right, cx),
        Some(("Show right dock".to_owned(), Some(false)))
    );
    assert!(!shell.present(Region::Tickets.id(), cx));
    assert!(layout(&shell, cx).is_on(Tickets));
    assert_eq!(
        read(&shell, Control::Panel(Tickets), cx),
        Some(("Show Tickets".to_owned(), Some(false)))
    );
    shell.click(right.id(), cx);
    assert!(shell.present(Region::Tickets.id(), cx));

    // Every panel of the left dock off closes it; its toggle then turns them
    // all on.
    shell.click(Control::Panel(Repositories).id(), cx);
    shell.click(Control::Panel(Documents).id(), cx);
    assert_eq!(
        read(&shell, left, cx),
        Some(("Show left dock".to_owned(), Some(false)))
    );
    assert!(!layout(&shell, cx).is_on(Repositories) && !layout(&shell, cx).is_on(Documents));
    shell.click(left.id(), cx);
    assert!(shell.present(Region::Repositories.id(), cx));
    assert!(shell.present(Region::Documents.id(), cx));
    assert_eq!(
        read(&shell, left, cx),
        Some(("Hide left dock".to_owned(), Some(true)))
    );
}

/// A secondary click on a panel toggle opens the dock menu with the panel's
/// side checked. Choosing another side moves the panel there and its toggle
/// to that side's group, with focus on it; choosing the current side changes
/// nothing (P8, P9). The same menu opens under a focused toggle (its keys are
/// tested in `keyboard_tests::shell`) and Escape gives focus back to the toggle (P19).
#[gpui_kit::test]
fn a_secondary_click_opens_the_dock_menu_with_its_side_checked(cx: &mut TestAppContext) {
    // The checked item is the one for the panel's side.
    for side in Side::ALL {
        let checked: Vec<Side> = menu::items(side)
            .into_iter()
            .filter(|(.., checked)| *checked)
            .map(|(_, side, _)| side)
            .collect();
        assert_eq!(checked, [side]);
    }
    assert_eq!(
        menu::items(Side::Bottom).map(|(label, ..)| label),
        ["Dock left", "Dock right", "Dock bottom"]
    );

    let shell = Shell::open(Fixture::new(), cx);
    let activity = Control::Panel(Activity);
    assert!(!open(&shell, cx));

    // The current side: the menu closes and nothing changes.
    let (before, saved) = (layout(&shell, cx), saves(&shell, cx));
    shell.frame(cx, |window, cx| window.right_click(activity.id(), cx));
    shell.run(cx);
    assert!(open(&shell, cx));
    let labels = shell.frame(cx, |window, _| {
        (0usize..3)
            .map(|ix| window.find(ix).label().unwrap_or_default().to_owned())
            .collect::<Vec<_>>()
    });
    assert_eq!(labels, ["Dock left", "Dock right", "Dock bottom"]);
    // `click_settled`: the Kit's `click` draws between the mouse events and
    // inside one update, so the menu would take focus back from the toggle
    // before the choice reaches the Workspace; in the application each event
    // is its own update.
    shell.click_settled(2usize, cx);
    assert!(!open(&shell, cx), "a choice closes the menu");
    assert_eq!(layout(&shell, cx), before, "its own side changes nothing");
    assert_eq!(saves(&shell, cx), saved);
    assert!(focused(&shell, activity.id(), cx), "the toggle has focus");

    // Another side: the panel moves, revealed, and its toggle with it.
    shell.frame(cx, |window, cx| window.right_click(activity.id(), cx));
    shell.run(cx);
    shell.click_settled(1usize, cx);
    assert!(!open(&shell, cx));
    let after = layout(&shell, cx);
    assert_eq!(after.side_of(Activity), Some(Side::Right));
    assert!(after.is_displayed(Activity));
    assert!(shell.present(Region::Operations.id(), cx));
    let footer = plan(&after, &FooterStatus::default());
    assert_eq!(
        footer.right[0],
        [
            Control::Panel(Tickets),
            Control::Panel(Activity),
            Control::Dock(Side::Right)
        ]
    );
    assert!(
        footer.left.iter().flatten().all(|c| *c != activity),
        "the toggle left the bottom group"
    );
    assert!(
        !shell.present(Control::Dock(Side::Bottom).id(), cx),
        "a side with no panels has no dock toggle"
    );
    assert_eq!(
        read(&shell, activity, cx),
        Some(("Hide Activity".to_owned(), Some(true)))
    );
    assert!(focused(&shell, activity.id(), cx), "focus stays on it");
    // Drawn in the right group: after Tickets' toggle, before the dock's.
    let x = |control: Control, cx: &mut TestAppContext| {
        shell.frame(cx, |window, _| window.find(control.id()).bounds().origin.x)
    };
    let (tickets_x, activity_x, dock_x) = (
        x(Control::Panel(Tickets), cx),
        x(activity, cx),
        x(Control::Dock(Side::Right), cx),
    );
    assert!(tickets_x < activity_x && activity_x < dock_x);

    // Under a focused toggle: focus in the menu; Escape gives it back to the
    // toggle. A secondary click leaves focus where it was.
    let tickets = Control::Panel(Tickets);
    let handle = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.footer_focus(tickets));
    shell.frame(cx, |window, cx| {
        handle.focus(window, cx);
        window.dispatch_action(Box::new(OpenDockMenu), cx)
    });
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert!(open(&shell, cx));
    assert!(focused(&shell, "popup-menu", cx), "focus is in the menu");
    shell.keys("escape", cx);
    assert!(!open(&shell, cx));
    assert!(focused(&shell, tickets.id(), cx), "back on the toggle");

    // A secondary click on another toggle leaves focus on Tickets' toggle
    // while the menu opens, before any draw gives the menu focus: nothing
    // inside the control takes it (the Kit's button would, for any button
    // but the primary).
    let position = shell.frame(cx, |window, _| window.find(activity.id()).bounds().center());
    let mut visual = shell.visual(cx);
    let kept = visual.update(|window, cx| {
        let down = MouseDownEvent {
            button: MouseButton::Right,
            position,
            modifiers: Modifiers::default(),
            click_count: 1,
            first_mouse: false,
        };
        window.dispatch_event(down.to_platform_input(), cx);
        handle.is_focused(window)
    });
    assert!(kept, "a secondary click moves no focus");
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert!(open(&shell, cx));
    assert!(focused(&shell, "popup-menu", cx));
    shell.keys("escape", cx);
    assert!(!open(&shell, cx));
    assert!(
        focused(&shell, tickets.id(), cx),
        "focus returns to where it was before the pointer's menu"
    );
}

/// Needs attention is hidden at 0, says how many, and leads to Activity,
/// revealing it first (P20).
#[gpui_kit::test]
fn needs_attention_counts_and_leads_to_activity(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    let attention = Control::Attention;
    assert_eq!(read(&shell, attention, cx), None, "hidden at 0");

    finish(&shell, 1, Outcome::Error, cx);
    assert_eq!(
        read(&shell, attention, cx),
        Some(("1 needs attention".to_owned(), None))
    );
    // A request that did not go wrong awaits nothing.
    finish(&shell, 2, Outcome::Success, cx);
    finish(&shell, 3, Outcome::Blocked, cx);
    assert_eq!(
        read(&shell, attention, cx),
        Some(("2 need attention".to_owned(), None))
    );

    shell.click(Control::Panel(Activity).id(), cx);
    assert!(!shell.present(Region::Operations.id(), cx));
    shell.click(attention.id(), cx);
    assert!(layout(&shell, cx).is_displayed(Activity));
    assert!(
        focused(&shell, Region::Operations.id(), cx),
        "Activity has focus"
    );
}

/// Settings opens the page `secondary-,` opens (P21).
#[gpui_kit::test]
fn settings_opens_the_settings_page(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    assert_eq!(
        read(&shell, Control::Settings, cx),
        Some(("Settings".to_owned(), None))
    );
    let shown = |shell: &Shell, cx: &mut TestAppContext| {
        shell
            .workspace
            .read_with(cx, |workspace, cx| workspace.settings_shown(cx))
    };
    assert_eq!(shown(&shell, cx), Shown::Tabs);
    shell.click(Control::Settings.id(), cx);
    assert_eq!(
        shown(&shell, cx),
        Shown::Settings(SettingsPage::Repositories)
    );
}

/// With a dialog or the sheet open, no footer control acts: the layout is
/// unchanged and the modal stays (P18, R-30). A click on the footer lands
/// on the overlay; a control pressed any other way (Enter or Space on a
/// focused one) reaches the Workspace, which refuses it, as it refuses the
/// dock actions.
#[gpui_kit::test]
fn footer_controls_do_nothing_behind_a_dialog(cx: &mut TestAppContext) {
    let shell = shell_with_a_ticket(cx);
    let controls = [
        Control::Panel(Tickets),
        Control::Dock(Side::Left),
        Control::Attention,
        Control::Settings,
    ];
    finish(&shell, 1, Outcome::Error, cx);
    let (before, saved) = (layout(&shell, cx), saves(&shell, cx));

    for sheet in [false, true] {
        open_modal(&shell, sheet, cx);
        for control in controls {
            // A click on the footer closes the sheet (its overlay takes it,
            // fact 33), so only the dialog is clicked.
            if !sheet {
                shell.click(control.id(), cx);
            }
            shell.frame(cx, |window, cx| {
                shell.workspace.update(cx, |workspace, cx| {
                    workspace.press(control, window, cx);
                    if let Control::Panel(kind) = control {
                        let at = gpui_kit::point(gpui_kit::px(10.), gpui_kit::px(10.));
                        workspace.open_dock_menu(kind, menu::Placement::Pointer(at), window, cx);
                    }
                })
            });
            shell.run(cx);
            assert_eq!(layout(&shell, cx), before, "{control:?}, sheet {sheet}");
        }
        // The actions, from where the modal holds focus: the menu key and a
        // move of Tickets to the left.
        shell.frame(cx, |window, cx| {
            window.dispatch_action(Box::new(OpenDockMenu), cx);
            let move_it = DockPanel {
                panel: Tickets,
                side: Side::Left,
            };
            window.dispatch_action(Box::new(move_it), cx);
        });
        shell.run(cx);
        assert_eq!(layout(&shell, cx), before, "actions, sheet {sheet}");
        assert_eq!(saves(&shell, cx), saved, "no save, sheet {sheet}");
        assert!(!open(&shell, cx), "no menu, sheet {sheet}");
        assert!(modal_open(&shell, cx), "the modal stays, sheet {sheet}");
        shell.frame(cx, |window, cx| {
            window.close_all_dialogs(cx);
            window.close_sheet(cx);
        });
        shell.run(cx);
    }
    let shown = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.settings_shown(cx));
    assert_eq!(shown, Shown::Tabs, "Settings did not open");
}

/// A dock menu opened by the pointer does not outlive a dialog or the sheet
/// that opens over it: it goes without a choice, and the next draw does not
/// take focus from the modal (P18).
#[gpui_kit::test]
fn a_dock_menu_closes_when_a_modal_opens_over_it(cx: &mut TestAppContext) {
    let shell = shell_with_a_ticket(cx);
    let before = layout(&shell, cx);
    for sheet in [false, true] {
        shell.frame(cx, |window, cx| {
            window.right_click(Control::Panel(Activity).id(), cx)
        });
        shell.run(cx);
        shell.frame(cx, |_, _| ());
        assert!(open(&shell, cx), "sheet {sheet}: the menu is open");

        open_modal(&shell, sheet, cx);
        assert!(!open(&shell, cx), "sheet {sheet}: the menu is gone");
        let toggles_focused = Control::ALL
            .into_iter()
            .filter(|control| focused(&shell, control.id(), cx))
            .count();
        assert_eq!(toggles_focused, 0, "sheet {sheet}: focus is not behind");
        assert_eq!(layout(&shell, cx), before, "nothing was chosen");
        shell.frame(cx, |window, cx| {
            window.close_all_dialogs(cx);
            window.close_sheet(cx);
        });
        shell.run(cx);
    }
}

/// Names and tooltips follow the state (R-24); Needs attention's text is the
/// count's (R-13); no control is named twice.
#[gpui_kit::test]
fn toggle_names_follow_their_state(_cx: &mut TestAppContext) {
    for (kind, label) in [
        (Repositories, "Repositories"),
        (Tickets, "Tickets"),
        (Documents, "Documents"),
        (Activity, "Activity"),
    ] {
        assert_eq!(panel_toggle_name(kind, true), format!("Hide {label}"));
        assert_eq!(panel_toggle_name(kind, false), format!("Show {label}"));
    }
    for (side, word) in [
        (Side::Left, "left"),
        (Side::Right, "right"),
        (Side::Bottom, "bottom"),
    ] {
        assert_eq!(dock_toggle_name(side, true), format!("Hide {word} dock"));
        assert_eq!(dock_toggle_name(side, false), format!("Show {word} dock"));
    }
    assert_eq!(attention_text(0), None);
    assert_eq!(attention_text(1).as_deref(), Some("1 needs attention"));
    assert_eq!(attention_text(2).as_deref(), Some("2 need attention"));
    assert_eq!(attention_text(30).as_deref(), Some("30 need attention"));
    // The default layout: the footer's groups (`02`, "Footer").
    let default = plan(&Layout::default(), &FooterStatus::default());
    assert_eq!(
        default.left,
        [
            vec![
                Control::Dock(Side::Left),
                Control::Panel(Repositories),
                Control::Panel(Documents)
            ],
            vec![Control::Dock(Side::Bottom), Control::Panel(Activity)],
        ]
    );
    assert_eq!(
        default.right,
        [
            vec![Control::Panel(Tickets), Control::Dock(Side::Right)],
            vec![Control::Settings]
        ]
    );
    let three = FooterStatus {
        attention: 3,
        ..FooterStatus::default()
    };
    assert_eq!(
        plan(&Layout::default(), &three).right[0],
        [Control::Attention]
    );
}

/// The footer's two status items, Update check and To share (Part G): their
/// texts, names and choices as pure functions (one table), then in a window
/// with polling on: Tab reaches both, Enter on To share chooses the next
/// repository with work each time and focus stays, the only repository with
/// work being chosen reads nothing and changes nothing, a hidden Repositories
/// panel stays hidden, a modal refuses both, a share that empties the count
/// moves focus on, two folders of the same name are told apart, a save
/// before any check counts, and Update check opens Settings (P12, P15, P25,
/// P37, P41, P44, R-38, R-39).
#[gpui_kit::test]
async fn status_items_say_and_choose(cx: &mut TestAppContext) {
    status_table::table();
    status::journey(cx).await;
}
