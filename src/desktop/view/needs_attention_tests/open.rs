//! P22: the footer item opens the popover and closes it.

use super::*;
use crate::desktop::view::harness::Shell;

/// P22: a click opens the popover with the attention rows, newest first, and
/// a second click closes it; Enter opens it and Escape closes it, with focus
/// back on the item; Space the same. The popover lists only the rows that
/// need attention (the window also has rows that do not), focuses its list
/// and highlights the first row.
#[gpui_kit::test]
fn the_popover_opens_lists_and_closes(cx: &mut TestAppContext) {
    let world = rows(cx);
    let shell = &world.shell;
    settle(shell, cx);
    let attention = attention_rows(shell, cx);
    let keys = keys_of(&attention);
    assert!(keys.len() >= 4, "attention rows: {keys:?}");
    for pending in &world.pending {
        assert!(keys.contains(pending), "{pending:?} needs attention");
    }
    for quiet in [&world.plain, &world.item, &world.disabled] {
        assert!(!keys.contains(quiet), "{quiet:?} does not need attention");
    }
    assert!(
        attention.windows(2).all(|pair| pair[0].at >= pair[1].at),
        "newest first"
    );
    assert!(!is_open(shell, cx) && !drawn(shell, cx), "closed at first");

    // By click, twice.
    open_by_click(shell, cx);
    assert!(is_open(shell, cx), "a click opens it");
    assert!(drawn(shell, cx), "and it is drawn");
    assert_eq!(
        keys_of(&applied(shell, cx)),
        keys,
        "it lists the attention rows, newest first, and no other"
    );
    for ix in 0..keys.len() {
        assert!(
            shell.present(("needs-attention-row", ix), cx),
            "row {ix} is drawn"
        );
    }
    assert!(
        !shell.present(("needs-attention-row", keys.len()), cx),
        "and no more than the attention rows"
    );
    assert_eq!(spot(shell, cx), Spot::List, "opening focuses its list");
    assert_eq!(
        cursor(shell, cx).as_ref(),
        keys.first(),
        "with the first row highlighted"
    );
    shell.click(Control::Attention.id(), cx);
    assert!(!is_open(shell, cx), "a second click closes it");
    assert!(!drawn(shell, cx), "and it is gone");
    assert_eq!(spot(shell, cx), Spot::Item, "focus is on the item");

    // By Enter, then Escape; by Space, then Escape.
    for key in ["enter", "space"] {
        focus_item(shell, cx);
        shell.press(key, cx);
        shell.frame(cx, |_, _| ());
        assert!(is_open(shell, cx), "{key} opens it");
        assert!(drawn(shell, cx), "{key}: it is drawn");
        assert_eq!(spot(shell, cx), Spot::List, "{key}: its list has focus");
        shell.keys("escape", cx);
        assert!(!is_open(shell, cx), "Escape closes it ({key})");
        assert!(!drawn(shell, cx));
        assert_eq!(spot(shell, cx), Spot::Item, "focus is back on the item");
    }
    let _: &Shell = shell;
}
