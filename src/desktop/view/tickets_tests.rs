//! The ticket list: the closure filter keeps every nonconforming entry,
//! the rows are the library's, a filter change drops the last filter's
//! read, and the keyboard moves and opens.

use std::path::{Path, PathBuf};

use gpui_kit::TestAppContext;
use manyhands::repository::{ClosureFilter, ItemDto, ItemDtoKind, ItemListDto};

use super::{LIFECYCLE_CLOSED, LIFECYCLE_OPEN, NONCONFORMING, row_label};
use crate::desktop::{
    model::{navigation::OpenTarget, tickets::TicketQuery},
    services::ReadKind,
    test_support::fixtures::Fixture,
    view::harness::{Shell, key},
};

const SAYS_CLOSED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC0";
const CLOSED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC1";
const PLAIN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC2";
const BROKEN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC3";

/// An open ticket whose status is `closed`, a lifecycle-closed ticket, a
/// plain open ticket and a nonconforming file under `.manyhands/tickets/`,
/// then a library refresh. The repository is selected and the Tickets
/// pane shown.
fn four_tickets(cx: &mut TestAppContext) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_ticket(&root, SAYS_CLOSED, "Says closed", "closed");
    fixture.commit_closed_ticket(&root, CLOSED, "Closed for good");
    fixture.create_ticket(&root, PLAIN, "Plain", "open");
    fixture.commit_nonconforming_ticket(&root, BROKEN);
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    shell.click(("repo-row", 0usize), cx);
    shell.click("pane-tickets", cx);
    (shell, root)
}

fn library(shell: &Shell, root: &Path, closure: ClosureFilter) -> ItemListDto {
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    let query = TicketQuery {
        closure,
        ..TicketQuery::default()
    };
    service.list_tickets(&repo, &query.to_filter()).unwrap()
}

/// What a test reads of one row: its label, status and lifecycle.
type Seen = (String, Option<String>, Option<String>);

fn visible(shell: &Shell, cx: &mut TestAppContext) -> Vec<Seen> {
    shell
        .rows("ticket-row", cx)
        .into_iter()
        .enumerate()
        .map(|(index, label)| {
            (
                label,
                shell.label(("ticket-status", index), cx),
                shell.label(("ticket-lifecycle", index), cx),
            )
        })
        .collect()
}

/// What the row for `item` must show.
fn expected(item: &ItemDto) -> Seen {
    let lifecycle = match &item.closure {
        Some(closure) if closure.closed_at.is_some() => Some(LIFECYCLE_CLOSED.to_owned()),
        Some(_) => Some(LIFECYCLE_OPEN.to_owned()),
        None => None,
    };
    (row_label(item), item.status.clone(), lifecycle)
}

fn labels(seen: &[Seen]) -> Vec<&str> {
    seen.iter().map(|(label, ..)| label.as_str()).collect()
}

/// Review Focus 3, and no fabrication: under each closure filter the
/// visible rows are exactly the library's, and the nonconforming entry
/// is always among them.
#[gpui_kit::test]
fn each_closure_filter_shows_the_librarys_rows_and_keeps_the_nonconforming_entry(
    cx: &mut TestAppContext,
) {
    let (shell, root) = four_tickets(cx);
    let broken = format!(".manyhands/tickets/{BROKEN}/ticket.md");

    // Open is the default.
    assert!(shell.selected("filter-open", cx));
    for (filter, closure, mut wanted) in [
        (
            "filter-open",
            ClosureFilter::Open,
            vec!["Says closed", "Plain", broken.as_str()],
        ),
        (
            "filter-closed",
            ClosureFilter::Closed,
            vec!["Closed for good", broken.as_str()],
        ),
        (
            "filter-all",
            ClosureFilter::All,
            vec!["Says closed", "Closed for good", "Plain", broken.as_str()],
        ),
    ] {
        shell.click(filter, cx);
        assert!(shell.selected(filter, cx), "{filter}");
        let list = library(&shell, &root, closure);
        let seen = visible(&shell, cx);
        let library_rows: Vec<Seen> = list.items.iter().map(expected).collect();
        assert_eq!(seen, library_rows, "{filter}");
        let mut shown = labels(&seen);
        shown.sort_unstable();
        wanted.sort_unstable();
        assert_eq!(shown, wanted, "{filter}");
    }

    // An open ticket whose status says closed: status as written,
    // lifecycle Open, in text.
    shell.click("filter-all", cx);
    let seen = visible(&shell, cx);
    let says = seen
        .iter()
        .find(|(label, ..)| label == "Says closed")
        .unwrap();
    assert_eq!(says.1.as_deref(), Some("closed"));
    assert_eq!(says.2.as_deref(), Some(LIFECYCLE_OPEN));
    // The nonconforming entry says so in text, with its problem.
    let at = labels(&seen)
        .iter()
        .position(|label| *label == broken)
        .unwrap();
    assert_eq!(
        shell.label(("ticket-nonconforming", at), cx).as_deref(),
        Some(NONCONFORMING)
    );
    assert_eq!(
        shell.label(("ticket-problem", at), cx).as_deref(),
        Some("malformed_front_matter")
    );
}

/// Review Focus 1: the Closed read is held while the user moves on to
/// All; releasing it leaves All's rows.
#[gpui_kit::test]
fn a_filter_change_drops_the_held_read_of_the_last_filter(cx: &mut TestAppContext) {
    let (shell, root) = four_tickets(cx);
    let held = shell.backend.hold_read(ReadKind::Tickets);
    shell.click("filter-closed", cx);
    shell.click("filter-all", cx);
    let all: Vec<Seen> = library(&shell, &root, ClosureFilter::All)
        .items
        .iter()
        .map(expected)
        .collect();
    assert_eq!(visible(&shell, cx), all);

    // The Closed read may already be dropped with its task; either way it
    // is let go.
    let _ = held.send(());
    shell.run(cx);
    assert_eq!(visible(&shell, cx), all);
    assert!(shell.selected("filter-all", cx));
    assert!(!shell.selected("filter-closed", cx));
}

/// Arrows move through the list and Enter opens the row by its ID, or by
/// path for a nonconforming entry.
#[gpui_kit::test]
fn arrows_move_in_the_list_and_enter_opens(cx: &mut TestAppContext) {
    let (shell, root) = four_tickets(cx);
    shell.click("filter-all", cx);
    let list = library(&shell, &root, ClosureFilter::All);
    shell.keys("secondary-2", cx);

    shell.keys("down", cx);
    assert!(shell.selected(("ticket-row", 0usize), cx));
    shell.keys("down", cx);
    assert!(shell.selected(("ticket-row", 1usize), cx));
    assert!(!shell.selected(("ticket-row", 0usize), cx));
    shell.keys("up", cx);
    assert!(shell.selected(("ticket-row", 0usize), cx));

    shell.keys("enter", cx);
    let first = &list.items[0];
    let opened = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.opened(cx));
    assert_eq!(
        opened,
        Some(OpenTarget::Item {
            repo: key(&root),
            kind: ItemDtoKind::Ticket,
            id: first.id.as_deref().unwrap().parse().unwrap(),
        })
    );
    assert_eq!(shell.label("item-kind", cx).as_deref(), Some("Ticket"));
    assert_eq!(shell.label("item-id", cx), first.id.clone());

    // The last row is the nonconforming entry: it opens by path.
    let last = list.items.len() - 1;
    for _ in 0..last {
        shell.keys("down", cx);
    }
    assert!(shell.selected(("ticket-row", last), cx));
    shell.keys("enter", cx);
    let broken = &list.items[last];
    assert_eq!(broken.id, None);
    let opened = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.opened(cx));
    assert_eq!(
        opened,
        Some(OpenTarget::Inspect {
            repo: key(&root),
            worktree: broken.context.worktree.clone(),
            path: broken.path.clone(),
        })
    );
    assert_eq!(shell.label("item-path", cx), Some(broken.path.clone()));
}
