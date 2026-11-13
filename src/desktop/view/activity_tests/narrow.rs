//! `rows_fit_a_narrow_activity` (P21): the one containment test of Activity's
//! rows (K-19). Activity sits in the right dock at 320 px; the message is
//! long and the tag longer; every row's button, scrolled into view, lies
//! inside the list. The tests that press a button no longer check its bounds
//! (E-8): this is where it is checked, once, for every kind of row here.

use std::path::PathBuf;

use gpui_kit::{TestAppContext, component::dock::DockPlacement, px, test::TestWindowExt};
use manyhands::{canonical::ItemId, repository::AuthoringKind, results::ResultCode};

use super::{
    cases::{answered, fabricate},
    list_support::{OP_A, OP_B, save},
    pending_cases::{listing, read_pending},
};
use crate::desktop::{
    model::{
        activity::copy::message_of,
        layout::{PanelKind, Side},
    },
    services::requests::Intent,
    view::{
        activity_support::{drawn, refresh},
        dock::tests::support::{dock_panel, edit_area},
        operations::attention_support::Rig,
    },
};

/// A repository name no row can show in 320 px, and a short one beside it.
const LONG: &str = "a-repository-whose-name-is-far-too-long-for-a-narrow-activity-panel";

#[gpui_kit::test]
async fn rows_fit_a_narrow_activity(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let rig = Rig::new(&[(LONG, true), ("b", true)], cx);
    let shell = &rig.shell;
    let root = PathBuf::from(&rig.roots[0]);

    // A hold over both repositories ("long +1", Unlock…), requests that
    // offer Retry and Share…, and pending operations that offer Resume.
    let key = rig.lock();
    rig.list(cx);
    shell
        .until_drawn(cx, "the hold", |_, cx| rig.entries(cx).len() == 1)
        .await;
    let _ = key;
    let item = ItemId::generate();
    fabricate(
        shell,
        1,
        Intent::RepoSync { root: root.clone() },
        answered("repo sync", ResultCode::InternalError),
        cx,
    );
    fabricate(
        shell,
        2,
        Intent::ItemSync {
            root: root.clone(),
            kind: AuthoringKind::Ticket,
            id: item,
        },
        answered("item sync", ResultCode::Cancelled),
        cx,
    );
    fabricate(
        shell,
        3,
        Intent::IndexRebuild { root: root.clone() },
        answered("index rebuild", ResultCode::IndexUnavailable),
        cx,
    );
    read_pending(
        shell,
        &rig.roots[0],
        listing(vec![save(OP_A), save(OP_B)]),
        cx,
    );
    refresh(shell, cx);

    // Activity into the right dock, at 320 px, by the layout model's API.
    dock_panel(shell, PanelKind::Activity, Side::Right, cx);
    edit_area(shell, cx, |area, window, cx| {
        area.set_dock_size(DockPlacement::Right, px(320.), window, cx)
    });
    for _ in 0..3 {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
    }
    let width = shell.frame(cx, |window, _| {
        window.find("operation-area").bounds().size.width
    });
    assert!(
        width <= px(330.) && width > px(100.),
        "the list is {width:?} wide"
    );

    // The rows: the longest message is longer than the list is wide.
    let rows = drawn(shell, cx);
    let buttons = rows.iter().filter(|row| row.row.button.is_some()).count();
    assert!(
        rows.len() >= 6 && buttons >= 5,
        "rows of each kind: {} rows, {buttons} with a button",
        rows.len()
    );
    let longest = rows
        .iter()
        .map(|row| message_of(&row.row).len() + row.row.repo.as_ref().map_or(0, String::len))
        .max()
        .unwrap();
    assert!(
        longest > 100,
        "a message and tag too long to fit: {longest}"
    );
    assert!(
        rows.iter().any(|row| row
            .row
            .repo
            .as_deref()
            .is_some_and(|tag| tag.ends_with("+1"))),
        "a tag with +N"
    );

    // Each row scrolled into view, then its button against the list.
    for (ix, row) in rows.iter().enumerate() {
        shell.workspace.update(cx, |workspace, cx| {
            workspace.highlight_applied_row(&row.row.key);
            cx.notify();
        });
        for _ in 0..2 {
            shell.frame(cx, |_, _| ());
            shell.run(cx);
        }
        let (list, row_at, button) = shell.frame(cx, |window, _| {
            (
                window.find("operation-area").bounds(),
                window
                    .try_find(("activity-row", ix))
                    .map(|row| row.bounds()),
                window
                    .try_find(("activity-button", ix))
                    .map(|button| button.bounds()),
            )
        });
        let row_at = row_at.unwrap_or_else(|| panic!("row {ix} is drawn once scrolled to"));
        assert!(
            row_at.left() >= list.left() && row_at.right() <= list.right(),
            "row {ix} {row_at:?} lies across the list {list:?}"
        );
        match (&row.row.button, button) {
            (None, None) => {}
            (Some(offer), Some(button)) => assert!(
                button.size.width > px(0.)
                    && button.left() >= list.left()
                    && button.right() <= list.right()
                    && button.top() >= list.top()
                    && button.bottom() <= list.bottom(),
                "row {ix}'s {:?} button {button:?} lies inside the list {list:?}",
                offer.label
            ),
            (want, got) => panic!("row {ix}: button {want:?} drawn as {got:?}"),
        }
    }
}
