//! Activity's toolbar (Wave 04 Cycle 03b, C1): the filter (the filter half of
//! P14), the Tab order through the toolbar and the list (P15's Tab clause),
//! with the ⋯ menu and its Clear finished in `clear.rs` (P19) and the layers
//! in `layers.rs` (R-13, P31).

use gpui_kit::TestAppContext;
use manyhands::results::ResultCode;

use super::{
    cases::{answered, fabricate},
    list_support::*,
};
use crate::desktop::{
    model::activity::{
        copy::{NOTHING_NEEDS_ATTENTION, NOTHING_YET},
        rows::{ActivityRow, RowKey},
    },
    services::requests::{DesktopRequestId, Intent},
    test_support::fixtures::Fixture,
    view::{
        activity_support::{drawn, highlighted, list_focused, row},
        harness::Shell,
        workspace::ToolbarButton,
    },
};

/// The filter's state as a screen reader reads it: the wrapper's toggled
/// state.
pub(super) fn filter_state(shell: &Shell, cx: &mut TestAppContext) -> Option<bool> {
    shell.checked("activity-filter", cx)
}

/// A click on the filter's button, then the pane drawn again. (`settle`
/// asserts the pane holds every row, which a filter on does not.)
pub(super) fn press_filter(shell: &Shell, cx: &mut TestAppContext) {
    shell.click(ToolbarButton::ActivityFilter.id(), cx);
    for _ in 0..3 {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
    }
}

fn keys_of(rows: &[ActivityRow]) -> Vec<RowKey> {
    rows.iter().map(|row| row.row.key.clone()).collect()
}

/// P14, the filter: it is off at launch and its state is the wrapper's;
/// on, the list shows the rows that need attention, in the log's order, and
/// the rows it hides are still the log's; off again, every row is back.
/// The two empty states.
pub(super) fn filter_shows_attention_rows(rows: &Rows, cx: &mut TestAppContext) {
    let shell = &rows.shell;
    settle(shell, cx);
    assert_eq!(filter_state(shell, cx), Some(false), "off at launch");
    let all = drawn(shell, cx);
    let attention: Vec<RowKey> = all
        .iter()
        .filter(|row| row.row.attention)
        .map(|row| row.row.key.clone())
        .collect();
    assert!(
        !attention.is_empty() && attention.len() < all.len(),
        "the window holds rows of both kinds: {all:?}"
    );

    press_filter(shell, cx);
    assert_eq!(filter_state(shell, cx), Some(true), "on");
    assert_eq!(keys_of(&drawn(shell, cx)), attention, "only attention rows");
    for hidden in [&rows.plain, &rows.item, &rows.disabled] {
        assert!(
            row(shell, hidden, cx).is_some(),
            "{hidden:?} is hidden, not gone: the log keeps it"
        );
    }
    assert!(!shell.present("activity-empty", cx), "rows are shown");

    press_filter(shell, cx);
    assert_eq!(filter_state(shell, cx), Some(false), "off again");
    assert_eq!(
        keys_of(&drawn(shell, cx)),
        keys_of(&all),
        "every row is back"
    );
}

/// P14's empty states: "Nothing yet this session" with the filter off and
/// "Nothing needs attention" with it on, in a window with no rows and in one
/// whose only row needs no attention.
pub(super) fn filter_empty_states(cx: &mut TestAppContext) {
    let empty = Shell::open(Fixture::new(), cx);
    settle(&empty, cx);
    press_filter(&empty, cx);
    assert_eq!(
        empty.label("activity-empty", cx).as_deref(),
        Some(NOTHING_NEEDS_ATTENTION)
    );
    press_filter(&empty, cx);
    assert_eq!(
        empty.label("activity-empty", cx).as_deref(),
        Some(NOTHING_YET)
    );

    // A row that needs nothing: the filter hides it and says so.
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let shell = Shell::open(fixture, cx);
    let done = fabricate(
        &shell,
        1,
        Intent::IndexRefresh { root },
        answered("index refresh", ResultCode::Ok),
        cx,
    );
    settle(&shell, cx);
    assert!(!shell.present("activity-empty", cx), "the row is drawn");
    press_filter(&shell, cx);
    assert!(drawn(&shell, cx).is_empty(), "the filter hides it");
    assert!(
        row(&shell, &RowKey::Request(done), cx).is_some(),
        "the log still has it"
    );
    assert_eq!(
        shell.label("activity-empty", cx).as_deref(),
        Some(NOTHING_NEEDS_ATTENTION)
    );
}

/// Which of the toolbar and the list holds focus: the filter, ⋯, the list or
/// the highlighted row's button.
fn held(shell: &Shell, cx: &mut TestAppContext) -> Option<&'static str> {
    let (filter, more, list, button) = shell.workspace.read_with(cx, |workspace, _| {
        (
            workspace
                .toolbar_focus(ToolbarButton::ActivityFilter)
                .clone(),
            workspace.toolbar_focus(ToolbarButton::ActivityMore).clone(),
            workspace.activity_pane().list_focus().clone(),
            workspace.activity_pane().button_focus().clone(),
        )
    });
    shell.frame(cx, |window, _| {
        [
            ("filter", filter),
            ("more", more),
            ("list", list),
            ("button", button),
        ]
        .into_iter()
        .find(|(_, handle)| handle.is_focused(window))
        .map(|(name, _)| name)
    })
}

/// Puts focus on the toolbar button `button`, as a click on it would.
fn focus_button(shell: &Shell, button: ToolbarButton, cx: &mut TestAppContext) {
    let handle = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.toolbar_focus(button).clone());
    shell.frame(cx, |window, cx| handle.focus(window, cx));
}

/// P15's Tab clause: Tab runs the filter, ⋯, the list and the highlighted
/// row's button, and Shift-Tab runs back. A Tab into the list, a route with
/// no `secondary-4`, starts the highlight on the first row, so Enter and the
/// menu key act on it at once (ruling T9-2).
pub(super) fn tab_order(cx: &mut TestAppContext) {
    let rows = rows(cx);
    let shell = &rows.shell;
    settle(shell, cx);
    let first = drawn(shell, cx).remove(0);
    assert!(
        first.row.button.as_ref().is_some_and(|offer| offer.enabled),
        "the first row has an enabled button: {first:?}"
    );
    assert_eq!(highlighted(shell, cx), None, "nothing is highlighted yet");
    focus_button(shell, ToolbarButton::ActivityFilter, cx);
    assert_eq!(held(shell, cx), Some("filter"));
    for (key, at) in [("tab", "more"), ("tab", "list")] {
        shell.keys(key, cx);
        assert_eq!(held(shell, cx), Some(at), "{key} to {at}");
    }
    shell.frame(cx, |_, _| ());
    assert_eq!(
        highlighted(shell, cx).as_ref(),
        Some(&first.row.key),
        "Tab into the list highlights the first row"
    );
    shell.keys("tab", cx);
    assert_eq!(held(shell, cx), Some("button"), "the row's button");
    for (key, at) in [
        ("shift-tab", "list"),
        ("shift-tab", "more"),
        ("shift-tab", "filter"),
    ] {
        shell.keys(key, cx);
        assert_eq!(held(shell, cx), Some(at), "{key} to {at}");
    }

    // A Tab into the list, and Enter at once: the first row's button.
    let rows = self::rows(cx);
    let shell = &rows.shell;
    settle(shell, cx);
    let first = drawn(shell, cx).remove(0);
    focus_button(shell, ToolbarButton::ActivityMore, cx);
    let before = pressed(shell, cx);
    shell.keys("tab", cx);
    assert_eq!(held(shell, cx), Some("list"));
    shell.keys("enter", cx);
    assert_eq!(pressed(shell, cx), before + 1, "Enter pressed a row");
    let RowKey::Pending { operation, .. } = &first.row.key else {
        panic!("the first row is a pending operation: {first:?}");
    };
    assert_eq!(resumes(shell, operation, cx), 1, "the first row's Resume");

    // And the menu key.
    let rows = self::rows(cx);
    let shell = &rows.shell;
    settle(shell, cx);
    let first = drawn(shell, cx).remove(0);
    focus_button(shell, ToolbarButton::ActivityMore, cx);
    shell.keys("tab", cx);
    shell.keys("shift-f10", cx);
    shell.frame(cx, |_, _| ());
    assert_eq!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.activity_menu_key()),
        Some(first.row.key),
        "the menu key opened the first row's menu"
    );
}

/// Whether ⋯'s menu is open, read after a frame.
pub(super) fn more_open(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |_, _| ());
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.activity_more_open())
}

/// C3: "Show it" turns the filter off when it hides the holder's row, and
/// highlights and focuses that row; a holder the filter does not hide leaves
/// it as it is.
#[gpui_kit::test]
fn show_it_turns_the_filter_off_for_a_hidden_holder(cx: &mut TestAppContext) {
    let rows = rows(cx);
    let shell = &rows.shell;
    let RowKey::Request(plain) = rows.plain.clone() else {
        panic!("a request's row");
    };
    settle(shell, cx);
    let show = |shell: &Shell, holder: Option<DesktopRequestId>, cx: &mut TestAppContext| {
        let root = rows.root.clone();
        shell.frame(cx, |window, cx| {
            shell.workspace.update(cx, |workspace, cx| {
                workspace.show_holder(root, holder, window, cx)
            })
        });
        for _ in 0..3 {
            shell.frame(cx, |_, _| ());
            shell.run(cx);
        }
    };

    // On, the filter hides the plain row; the pending row the fallback
    // names needs attention, so it stays shown and the filter stays on.
    press_filter(shell, cx);
    assert_eq!(filter_state(shell, cx), Some(true));
    assert!(
        !drawn(shell, cx).iter().any(|row| row.row.key == rows.plain),
        "the filter hides the holder's row"
    );
    show(shell, None, cx);
    assert_eq!(filter_state(shell, cx), Some(true), "nothing hid the row");
    assert!(
        matches!(highlighted(shell, cx), Some(RowKey::Pending { .. })),
        "the first pending row is highlighted: {:?}",
        highlighted(shell, cx)
    );

    // The holder the filter hides: it goes off and the row is shown,
    // highlighted, with the list focused.
    show(shell, Some(plain), cx);
    assert_eq!(filter_state(shell, cx), Some(false), "the filter went off");
    assert!(drawn(shell, cx).iter().any(|row| row.row.key == rows.plain));
    assert_eq!(highlighted(shell, cx), Some(rows.plain.clone()));
    assert!(list_focused(shell, cx), "the list has the focus");
}
