//! P22: the footer item opens the popover and closes it.

use manyhands::results::ResultCode;

use super::*;
use crate::desktop::{
    model::activity::rows::RowKey,
    services::requests::Intent,
    test_support::fixtures::item_id,
    view::{
        activity::tests::{
            cases::{TICKET, answered, fabricate},
            list_support::{OP_A, save},
            pending_cases::{listing, read_pending},
        },
        activity_support::refresh,
        harness::Shell,
        settings::polling_support::Rig,
    },
};

/// P22: a click opens the popover with the attention rows, newest first, and
/// a second click closes it; Enter opens it and Escape closes it, with focus
/// back on the item; Space the same. The popover lists only the rows that
/// need attention (the window also has rows that do not), focuses its list
/// and highlights the first row.
#[gpui_kit::test]
fn the_popover_opens_lists_and_closes(cx: &mut TestAppContext) {
    // Rows that arrived a minute apart, on the window's own clock, so that
    // "newest first" is an order and not a tie: a quiet request, two failed
    // ones and a pending operation, oldest first.
    let rig = Rig::new(&[("a", false)], cx);
    let shell = &rig.shell;
    let root = rig.roots[0].clone();
    let step = |cx: &mut TestAppContext| {
        refresh(shell, cx);
        rig.clock.advance(std::time::Duration::from_secs(60));
    };
    let quiet = RowKey::Request(fabricate(
        shell,
        1,
        Intent::IndexRefresh { root: root.clone() },
        answered("index refresh", ResultCode::Ok),
        cx,
    ));
    step(cx);
    let first = RowKey::Request(fabricate(
        shell,
        2,
        Intent::IndexRefresh { root: root.clone() },
        answered("index refresh", ResultCode::IndexUnavailable),
        cx,
    ));
    step(cx);
    let second = RowKey::Request(fabricate(
        shell,
        3,
        Intent::TicketClose {
            root: root.clone(),
            id: item_id(TICKET),
        },
        answered("ticket close", ResultCode::PushRejected),
        cx,
    ));
    step(cx);
    let listed = crate::desktop::view::harness::key(&root);
    read_pending(shell, &listed, listing(vec![save(OP_A)]), cx);
    let pending = RowKey::Pending {
        root: listed,
        operation: OP_A.to_owned(),
    };
    step(cx);
    settle(shell, cx);
    let attention = attention_rows(shell, cx);
    let keys = keys_of(&attention);
    assert_eq!(
        keys,
        [pending.clone(), second, first],
        "the attention rows, newest first"
    );
    assert!(
        attention.windows(2).all(|pair| pair[0].at > pair[1].at),
        "at distinct times: {:?}",
        attention.iter().map(|row| row.at).collect::<Vec<_>>()
    );
    assert!(
        crate::desktop::view::activity_support::row(shell, &quiet, cx)
            .is_some_and(|row| !row.row.attention),
        "a quiet row is in the log"
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
