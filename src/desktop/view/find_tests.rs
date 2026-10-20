//! Find over real repositories: a ULID opens its item, a closed ticket
//! too; a short code two tickets share lists both, and choosing one opens
//! it by its ID; no match says so; and no tab is ever keyed by a short
//! code.

use std::path::PathBuf;

use gpui_kit::{Focusable, TestAppContext};
use manyhands::repository::ItemDtoKind;

use super::{FIND_NO_REPOSITORY, FIND_NONE, FIND_OPENED};
use crate::desktop::{
    model::{navigation::OpenTarget, tabs::TabKey},
    test_support::fixtures::{Fixture, item_id},
    view::{
        harness::{Shell, key},
        tickets::LIFECYCLE_CLOSED,
        workspace::Region,
    },
};

const OPEN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK0";
const CLOSED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK1";
const FIRST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK2";
const SECOND: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK3";
const UNKNOWN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK9";
const SHARED_SLUG: &str = "ae-k9x2b";

/// An open ticket made through the library, a lifecycle-closed one, and
/// two committed tickets carrying the same short code; refreshed and
/// selected.
fn shell(cx: &mut TestAppContext) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_ticket(&root, OPEN, "Open one", "open");
    fixture.commit_closed_ticket(&root, CLOSED, "Closed one");
    fixture.commit_duplicate_slug(
        &root,
        (FIRST, "First sharer"),
        (SECOND, "Second sharer"),
        SHARED_SLUG,
    );
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    (shell, root)
}

/// `secondary-p`, the text, then Enter in the Find input.
fn find(shell: &Shell, text: &str, cx: &mut TestAppContext) {
    shell.keys("secondary-p", cx);
    let input = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.find_input().clone());
    let mut visual = shell.visual(cx);
    visual.update(|window, cx| {
        input.update(cx, |input, cx| input.set_value("", window, cx));
    });
    visual.simulate_input(text);
    shell.keys("enter", cx);
}

fn opened(shell: &Shell, cx: &mut TestAppContext) -> Option<OpenTarget> {
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.opened(cx))
}

fn tab_keys(shell: &Shell, cx: &mut TestAppContext) -> Vec<TabKey> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.tab_keys())
}

fn ticket(root: &std::path::Path, id: &str) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: item_id(id),
    }
}

fn find_focused(shell: &Shell, cx: &mut TestAppContext) -> bool {
    let input = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.find_input().clone());
    shell.frame(cx, |window, cx| {
        input.read(cx).focus_handle(cx).is_focused(window)
    })
}

/// `secondary-p` focuses the Find input, inside the navigation region,
/// whose ring says so.
#[gpui_kit::test]
fn secondary_p_focuses_find(cx: &mut TestAppContext) {
    let (shell, _) = shell(cx);
    assert!(!find_focused(&shell, cx));
    shell.keys("secondary-p", cx);
    assert!(find_focused(&shell, cx));
    assert!(shell.present(Region::Navigation.ring_id(), cx));
}

/// A ULID opens its ticket in a tab, a lifecycle-closed one as any other,
/// in either case.
#[gpui_kit::test]
fn a_ulid_opens_its_ticket_closed_or_not(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx);
    find(&shell, OPEN, cx);
    assert_eq!(opened(&shell, cx), Some(ticket(&root, OPEN)));
    assert_eq!(shell.label("find-state", cx).as_deref(), Some(FIND_OPENED));
    assert_eq!(shell.label("item-title", cx).as_deref(), Some("Open one"));

    find(&shell, &CLOSED.to_ascii_lowercase(), cx);
    assert_eq!(opened(&shell, cx), Some(ticket(&root, CLOSED)));
    assert_eq!(shell.label("item-title", cx).as_deref(), Some("Closed one"));
    assert_eq!(
        shell.label("item-lifecycle", cx).as_deref(),
        Some(LIFECYCLE_CLOSED)
    );
    assert_eq!(
        tab_keys(&shell, cx),
        [OPEN, CLOSED].map(|id| TabKey::for_target(&ticket(&root, id)))
    );
}

/// A short code two tickets share lists both, each with its title,
/// lifecycle, context and ID; choosing one opens it by its ID, and its tab
/// is keyed by that ID, never by the short code.
#[gpui_kit::test]
fn a_shared_short_code_lists_both_and_choosing_one_opens_it_by_id(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx);
    find(&shell, &SHARED_SLUG.to_ascii_uppercase(), cx);
    let service = shell.fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let library = service.find_tickets_by_slug(&repo, SHARED_SLUG).unwrap();
    assert_eq!(library.items.len(), 2);
    assert_eq!(opened(&shell, cx), None);

    let shown = shell.rows("find-match", cx);
    let titles: Vec<String> = library
        .items
        .iter()
        .map(|item| item.title.clone().unwrap())
        .collect();
    assert_eq!(shown, titles);
    for (ix, item) in library.items.iter().enumerate() {
        assert_eq!(
            shell.label(("find-match-slug", ix), cx).as_deref(),
            Some(SHARED_SLUG)
        );
        assert_eq!(shell.label(("find-match-id", ix), cx), item.id.clone());
        assert_eq!(
            shell.label(("find-match-lifecycle", ix), cx).as_deref(),
            Some("Open")
        );
        assert_eq!(
            shell.label(("find-match-context", ix), cx).as_deref(),
            Some("Primary")
        );
        assert_eq!(
            shell.label(("find-match-worktree", ix), cx),
            Some(item.context.worktree.clone())
        );
    }

    let second = library.items[1].id.clone().unwrap();
    shell.click(("find-match-open", 1usize), cx);
    assert_eq!(opened(&shell, cx), Some(ticket(&root, &second)));
    assert_eq!(shell.label("item-id", cx), Some(second.clone()));
    for key in tab_keys(&shell, cx) {
        let TabKey::Item { id, .. } = key else {
            panic!("a ticket's tab is keyed by its ID");
        };
        assert_eq!(id, second);
        assert_ne!(id.to_ascii_lowercase(), SHARED_SLUG);
    }
    // The matches stay, to choose the other.
    assert_eq!(shell.rows("find-match", cx), titles);
}

/// No match says so, for a short code and for an ID, and opens nothing.
#[gpui_kit::test]
fn no_match_says_so_and_opens_nothing(cx: &mut TestAppContext) {
    let (shell, _) = shell(cx);
    find(&shell, "zz-zzzzz", cx);
    assert_eq!(shell.label("find-state", cx).as_deref(), Some(FIND_NONE));
    assert!(shell.rows("find-match", cx).is_empty());
    find(&shell, UNKNOWN, cx);
    assert_eq!(shell.label("find-state", cx).as_deref(), Some(FIND_NONE));
    assert!(tab_keys(&shell, cx).is_empty());
    assert_eq!(opened(&shell, cx), None);
}

/// With no repository selected there is nothing to search, and Find says
/// so; selecting another repository clears what was found.
#[gpui_kit::test]
fn find_needs_a_selected_repository_and_a_new_selection_clears_it(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let b = fixture.create_repository("b");
    fixture.commit_duplicate_slug(&a, (FIRST, "First"), (SECOND, "Second"), SHARED_SLUG);
    fixture.refresh(&a);
    let shell = Shell::open(fixture, cx);
    find(&shell, SHARED_SLUG, cx);
    assert_eq!(
        shell.label("find-state", cx).as_deref(),
        Some(FIND_NO_REPOSITORY)
    );

    shell.select(&a, cx);
    find(&shell, SHARED_SLUG, cx);
    assert_eq!(shell.rows("find-match", cx).len(), 2);
    shell.select(&b, cx);
    assert!(shell.rows("find-match", cx).is_empty());
    assert_eq!(shell.label("find-state", cx), None);
}

/// Review Focus 1: a short-code search held while the user selects another
/// repository never lands there.
#[gpui_kit::test]
fn a_held_search_never_lands_after_the_selection_changed(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let b = fixture.create_repository("b");
    fixture.commit_duplicate_slug(&a, (FIRST, "First"), (SECOND, "Second"), SHARED_SLUG);
    fixture.refresh(&a);
    let shell = Shell::open(fixture, cx);
    shell.select(&a, cx);
    let held = shell
        .backend
        .hold_read(crate::desktop::services::ReadKind::FindSlug);
    find(&shell, SHARED_SLUG, cx);
    shell.select(&b, cx);
    let _ = held.send(());
    shell.run(cx);
    assert!(shell.rows("find-match", cx).is_empty());
    assert_eq!(shell.label("find-state", cx), None);
}

/// Every dialog traps focus: with the Abandon dialog open, `secondary-p`
/// does not reach Find behind it.
#[gpui_kit::test]
fn find_does_nothing_while_a_dialog_is_open(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.create_ticket(&a, OPEN, "Stranded", "open");
    let _operation = fixture.strand_ticket_save(&a, OPEN, "Edited");
    let shell = Shell::open(fixture, cx);
    shell.click(("pending-abandon", 0usize), cx);
    assert!(shell.present("confirm-dialog", cx));
    shell.keys("secondary-p", cx);
    assert!(!find_focused(&shell, cx));
    assert!(shell.present("confirm-dialog", cx));
}
