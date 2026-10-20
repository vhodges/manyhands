//! Tabs: keyed by repository and item, opened once, closed with a
//! neighbor taking over, and closed by repository.

use manyhands::repository::ItemDtoKind;

use super::{TabKey, Tabs};
use crate::desktop::{model::navigation::OpenTarget, test_support::fixtures::item_id};

const ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD0";
const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD1";

fn item(repo: &str, id: &str) -> TabKey {
    TabKey::Item {
        repo: repo.to_owned(),
        kind: ItemDtoKind::Ticket,
        id: id.to_owned(),
    }
}

fn inspect(repo: &str, path: &str) -> TabKey {
    TabKey::Inspect {
        repo: repo.to_owned(),
        worktree: repo.to_owned(),
        path: path.to_owned(),
    }
}

fn keys(tabs: &Tabs) -> Vec<TabKey> {
    tabs.tabs().iter().map(|(_, key)| key.clone()).collect()
}

#[test]
fn opening_appends_a_tab_and_makes_it_active() {
    let mut tabs = Tabs::default();
    assert_eq!(tabs.active(), None);
    let (first, new) = tabs.open(item("/a", ID));
    assert!(new);
    assert_eq!(tabs.active(), Some(first));
    let (second, new) = tabs.open(item("/a", OTHER));
    assert!(new);
    assert_ne!(first, second);
    assert_eq!(tabs.active(), Some(second));
    assert_eq!(keys(&tabs), [item("/a", ID), item("/a", OTHER)]);
}

#[test]
fn opening_an_open_key_focuses_its_tab_and_returns_false() {
    let mut tabs = Tabs::default();
    let (first, _) = tabs.open(item("/a", ID));
    tabs.open(item("/a", OTHER));
    let (again, new) = tabs.open(item("/a", ID));
    assert!(!new);
    assert_eq!(again, first);
    assert_eq!(tabs.active(), Some(first));
    assert_eq!(tabs.tabs().len(), 2, "no second tab for the same key");
}

/// Two repositories can hold the same ID: the repository is part of the
/// key, so both stay open.
#[test]
fn items_from_two_repositories_stay_open_together() {
    let mut tabs = Tabs::default();
    let (a, _) = tabs.open(item("/a", ID));
    let (b, new) = tabs.open(item("/b", ID));
    assert!(new);
    assert_ne!(a, b);
    assert_eq!(keys(&tabs), [item("/a", ID), item("/b", ID)]);
    // An inspection of a path and an item are different keys too.
    let (path, new) = tabs.open(inspect("/a", "tickets/x.md"));
    assert!(new);
    assert_eq!(tabs.tabs().len(), 3);
    assert_eq!(tabs.key(path), Some(&inspect("/a", "tickets/x.md")));
}

#[test]
fn a_key_follows_its_target() {
    let target = OpenTarget::Item {
        repo: "/a".to_owned(),
        kind: ItemDtoKind::Ticket,
        id: item_id(ID),
    };
    assert_eq!(TabKey::for_target(&target), item("/a", ID));
    let target = OpenTarget::Inspect {
        repo: "/a".to_owned(),
        worktree: "/a".to_owned(),
        path: "tickets/x.md".to_owned(),
    };
    assert_eq!(TabKey::for_target(&target), inspect("/a", "tickets/x.md"));
    assert_eq!(TabKey::for_target(&target).repo(), "/a");
}

#[test]
fn closing_the_active_tab_activates_its_neighbor_and_the_last_empties() {
    let mut tabs = Tabs::default();
    let (one, _) = tabs.open(item("/a", "1"));
    let (two, _) = tabs.open(item("/a", "2"));
    let (three, _) = tabs.open(item("/a", "3"));
    tabs.activate(two);

    // The tab after it takes its place.
    assert!(tabs.close(two));
    assert_eq!(tabs.active(), Some(three));
    // The last one's neighbor is the one before it.
    assert!(tabs.close(three));
    assert_eq!(tabs.active(), Some(one));
    assert!(!tabs.close(three), "already closed");
    assert!(tabs.close(one));
    assert_eq!(tabs.active(), None);
    assert!(tabs.tabs().is_empty());
}

#[test]
fn closing_an_inactive_tab_keeps_the_active_one() {
    let mut tabs = Tabs::default();
    let (one, _) = tabs.open(item("/a", "1"));
    let (two, _) = tabs.open(item("/a", "2"));
    assert!(tabs.close(one));
    assert_eq!(tabs.active(), Some(two));
}

#[test]
fn next_and_previous_wrap() {
    let mut tabs = Tabs::default();
    assert_eq!(tabs.next(), None);
    assert_eq!(tabs.previous(), None);
    let (one, _) = tabs.open(item("/a", "1"));
    let (two, _) = tabs.open(item("/a", "2"));
    let (three, _) = tabs.open(item("/a", "3"));
    assert_eq!(tabs.next(), Some(one));
    assert_eq!(tabs.active(), Some(one));
    assert_eq!(tabs.next(), Some(two));
    assert_eq!(tabs.previous(), Some(one));
    assert_eq!(tabs.previous(), Some(three));
    // Moving keeps the order.
    assert_eq!(
        keys(&tabs),
        [item("/a", "1"), item("/a", "2"), item("/a", "3")]
    );
}

#[test]
fn close_repository_closes_every_tab_of_that_root_only() {
    let mut tabs = Tabs::default();
    let (a1, _) = tabs.open(item("/a", "1"));
    let (b1, _) = tabs.open(item("/b", "1"));
    let (a2, _) = tabs.open(inspect("/a", "x.md"));
    // `/a-other` shares a prefix with `/a`, not a root.
    let (other, _) = tabs.open(item("/a-other", "1"));
    tabs.activate(a2);

    let closed = tabs.close_repository("/a");
    assert_eq!(closed, [a1, a2]);
    assert_eq!(keys(&tabs), [item("/b", "1"), item("/a-other", "1")]);
    assert_eq!(tabs.active(), Some(other), "the active tab's neighbor");
    assert!(tabs.close_repository("/a").is_empty());

    // A later registration at the same root inherits nothing: opening
    // its item again makes a new tab.
    let (again, new) = tabs.open(item("/a", "1"));
    assert!(new);
    assert_ne!(again, a1);
    assert_eq!(tabs.of_repository("/b"), [b1]);
}

#[test]
fn ids_are_not_reused() {
    let mut tabs = Tabs::default();
    let (one, _) = tabs.open(item("/a", "1"));
    tabs.close(one);
    let (two, _) = tabs.open(item("/a", "1"));
    assert_ne!(one, two);
}
