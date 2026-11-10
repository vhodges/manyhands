//! What the retirement loop asks of the open tabs (D6 Task 11b, RT-3).

use gpui_kit::{AppContext as _, TestAppContext};
use manyhands::repository::ItemDtoKind;

use super::{EditTabs, TabEditing, Watch};
use crate::desktop::model::tabs::TabKey;

fn item(id: &str) -> TabKey {
    TabKey::Item {
        repo: "/repo".to_owned(),
        kind: ItemDtoKind::Ticket,
        id: id.to_owned(),
    }
}

fn says(editing: TabEditing) -> Watch {
    Box::new(move |_| Some(editing))
}

/// RT-3: an edit session (dirty or clean, a begin in flight included),
/// unsaved text and a comment being written each hold the item back; a tab
/// of another item, a tab that is gone and a quiet tab do not. A composer's
/// text is not unsaved text: it is kept by its own draft, so it stays out of
/// the quit asks' count.
#[gpui_kit::test]
fn item_busy_table(cx: &mut TestAppContext) {
    let quiet = TabEditing::default();
    let cases = [
        ("a quiet tab", quiet, false),
        (
            "an edit session",
            TabEditing {
                editing: true,
                ..quiet
            },
            true,
        ),
        (
            "unsaved text",
            TabEditing {
                unkept: true,
                ..quiet
            },
            true,
        ),
        (
            "a comment being written",
            TabEditing {
                composing: true,
                ..quiet
            },
            true,
        ),
    ];
    for (what, editing, busy) in cases {
        let tabs = cx.new(|_| EditTabs::default());
        tabs.update(cx, |tabs, _| {
            tabs.watch(item("A"), says(editing));
            tabs.watch(item("B"), says(quiet));
            tabs.watch(item("C"), Box::new(|_| None));
        });
        let seen = |id: &str, cx: &mut TestAppContext| {
            tabs.read_with(cx, |tabs, cx| tabs.item_busy(&item(id), cx))
        };
        assert_eq!(seen("A", cx), busy, "{what}");
        assert!(!seen("B", cx), "{what}: another item's tab");
        assert!(!seen("C", cx), "{what}: a tab that is gone");
        assert!(!seen("D", cx), "{what}: an item with no tab");
        // A tab of another repository is another item.
        let elsewhere = TabKey::Item {
            repo: "/other".to_owned(),
            kind: ItemDtoKind::Ticket,
            id: "A".to_owned(),
        };
        assert!(
            !tabs.read_with(cx, |tabs, cx| tabs.item_busy(&elsewhere, cx)),
            "{what}: another repository's"
        );
    }
}
