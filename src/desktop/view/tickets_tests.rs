//! The ticket list: the closure filter keeps every nonconforming entry,
//! the rows are the library's, a filter change drops the last filter's
//! read, and the keyboard moves and opens.

use std::path::{Path, PathBuf};

use gpui_kit::{TestAppContext, test::TestWindowExt};
use manyhands::repository::{
    ClosureFilter, ItemDto, ItemDtoKind, ItemListDto, ReadinessFilter, TicketFilter,
};

use super::{
    LIFECYCLE_CLOSED, LIFECYCLE_OPEN, NO_REASONS, NONCONFORMING, READINESS_FILTERS, SELECT_BLOCKED,
    WHY_BLOCKED, row_description, row_label,
};
use crate::desktop::{
    model::{
        navigation::OpenTarget,
        tickets::{TicketQuery, blocked_reasons},
    },
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
    // E-30: the kind and ID are in the details sheet.
    shell.click("chip-all", cx);
    assert_eq!(shell.label("item-kind", cx).as_deref(), Some("Ticket"));
    assert_eq!(shell.label("item-id", cx), first.id.clone());
    shell.keys("escape", cx);
    shell.keys("secondary-2", cx);

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

const BLOCKER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD0";
const WAITS: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD1";
const MISSING: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD2";
const DANGLING: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD3";
const CYCLE_A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD4";
const CYCLE_B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD5";
const DONE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD6";
const AFTER_DONE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD7";

/// A ticket waiting for an open one, one waiting for a ticket no context
/// holds, two waiting for each other, one whose only dependency is closed,
/// and a nonconforming entry. The Tickets pane is shown.
fn related(cx: &mut TestAppContext) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_ticket(&root, BLOCKER, "Blocker", "open");
    fixture.create_ticket_with_deps(&root, WAITS, "Waits", "open", &[BLOCKER]);
    fixture.create_ticket_with_deps(&root, DANGLING, "Dangling", "open", &[MISSING]);
    fixture.commit_dependency_cycle(&root, (CYCLE_A, "Cycle A"), (CYCLE_B, "Cycle B"));
    fixture.commit_closed_ticket(&root, DONE, "Done");
    fixture.commit_nonconforming_ticket(&root, BROKEN);
    fixture.refresh(&root);
    fixture.create_ticket_with_deps(&root, AFTER_DONE, "After done", "open", &[DONE]);
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    shell.click(("repo-row", 0usize), cx);
    shell.click("pane-tickets", cx);
    (shell, root)
}

/// The reasons drawn under the row at `index`.
fn reasons(shell: &Shell, index: usize, cx: &mut TestAppContext) -> Vec<String> {
    (0..)
        .map_while(|n| shell.label(format!("ticket-reason-{index}-{n}"), cx))
        .collect()
}

/// Review Focus 3, and no fabrication: under Ready and under Blocked the
/// rows are exactly those of the same `list_tickets` call, the
/// nonconforming entry among them; each blocked row gives its reasons.
#[gpui_kit::test]
fn ready_and_blocked_show_the_librarys_rows_with_reasons_and_keep_the_nonconforming_entry(
    cx: &mut TestAppContext,
) {
    let (shell, root) = related(cx);
    let broken = format!(".manyhands/tickets/{BROKEN}/ticket.md");
    let service = shell.fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let names = service
        .list_tickets(&repo, &TicketFilter::default())
        .unwrap();
    let by_id = names
        .items
        .iter()
        .filter_map(|item| Some((item.id.clone()?, item.clone())))
        .collect();

    assert!(shell.selected("filter-any-readiness", cx));
    for (id, readiness, mut wanted) in [
        (
            "filter-ready",
            ReadinessFilter::Ready,
            vec!["Blocker", "After done", broken.as_str()],
        ),
        (
            "filter-blocked",
            ReadinessFilter::Blocked,
            vec!["Waits", "Dangling", "Cycle A", "Cycle B", broken.as_str()],
        ),
    ] {
        shell.click(id, cx);
        assert!(shell.selected(id, cx), "{id}");
        assert!(!shell.selected("filter-any-readiness", cx), "{id}");
        let showing = READINESS_FILTERS
            .iter()
            .find(|(.., filter)| *filter == Some(readiness))
            .map(|(_, _, showing, _)| *showing);
        assert_eq!(
            shell.label("tickets-readiness-showing", cx).as_deref(),
            showing
        );
        let query = TicketQuery {
            closure: ClosureFilter::Open,
            readiness: Some(readiness),
        };
        let list = service.list_tickets(&repo, &query.to_filter()).unwrap();
        let seen = visible(&shell, cx);
        let library_rows: Vec<Seen> = list.items.iter().map(expected).collect();
        assert_eq!(seen, library_rows, "{id}");
        let mut shown = labels(&seen);
        shown.sort_unstable();
        wanted.sort_unstable();
        assert_eq!(shown, wanted, "{id}");

        for (index, item) in list.items.iter().enumerate() {
            let drawn = reasons(&shell, index, cx);
            match readiness {
                ReadinessFilter::Ready => assert!(drawn.is_empty(), "{id} {index}"),
                ReadinessFilter::Blocked => {
                    assert_eq!(drawn, blocked_reasons(item, &by_id), "{id} {index}");
                    assert_eq!(drawn.is_empty(), item.id.is_none(), "{id} {index}");
                }
            }
        }
    }

    // Each kind of reason, in text.
    let list = service
        .list_tickets(
            &repo,
            &TicketFilter {
                closure: ClosureFilter::Open,
                readiness: Some(ReadinessFilter::Blocked),
                ..TicketFilter::default()
            },
        )
        .unwrap();
    let at = |id: &str| {
        list.items
            .iter()
            .position(|item| item.id.as_deref() == Some(id))
            .unwrap()
    };
    let slug = names
        .items
        .iter()
        .find(|item| item.id.as_deref() == Some(BLOCKER))
        .and_then(|item| item.slug.clone())
        .unwrap();
    assert_eq!(
        reasons(&shell, at(WAITS), cx),
        [format!("Waits for open ticket: {slug} Blocker")]
    );
    assert_eq!(
        reasons(&shell, at(DANGLING), cx),
        [format!("Waits for a ticket not found: {MISSING}")]
    );
    assert_eq!(
        reasons(&shell, at(CYCLE_B), cx),
        [
            format!("Waits for open ticket: {CYCLE_A} Cycle A"),
            format!("On a dependency cycle: {CYCLE_A}, {CYCLE_B}"),
        ]
    );

    // All again: no reasons are drawn.
    shell.click("filter-any-readiness", cx);
    assert!(shell.selected("filter-any-readiness", cx));
    let count = visible(&shell, cx).len();
    assert!((0..count).all(|index| reasons(&shell, index, cx).is_empty()));
}

/// Review Focus 1: the Blocked read is held while the user moves on to
/// Ready; releasing it leaves Ready's rows.
#[gpui_kit::test]
fn a_readiness_change_drops_the_held_read_of_the_last_one(cx: &mut TestAppContext) {
    let (shell, root) = related(cx);
    let held = shell.backend.hold_read(ReadKind::Tickets);
    shell.click("filter-blocked", cx);
    shell.click("filter-ready", cx);
    let service = shell.fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let query = TicketQuery {
        closure: ClosureFilter::Open,
        readiness: Some(ReadinessFilter::Ready),
    };
    let ready: Vec<Seen> = service
        .list_tickets(&repo, &query.to_filter())
        .unwrap()
        .items
        .iter()
        .map(expected)
        .collect();
    assert_eq!(visible(&shell, cx), ready);
    let _ = held.send(());
    shell.run(cx);
    assert_eq!(visible(&shell, cx), ready);
    assert!(shell.selected("filter-ready", cx));
    assert!(!shell.selected("filter-blocked", cx));
}

/// The lines of the Why blocked panel.
fn panel(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    shell.rows("why-blocked-reason", cx)
}

/// Every blocked ticket's name map, as `blocked_reasons` is given it.
fn names(
    shell: &Shell,
    root: &Path,
) -> std::collections::HashMap<String, manyhands::repository::ItemDto> {
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    service
        .list_tickets(
            &repo,
            &TicketFilter {
                closure: ClosureFilter::Open,
                ..TicketFilter::default()
            },
        )
        .unwrap()
        .items
        .into_iter()
        .filter_map(|item| Some((item.id.clone()?, item)))
        .collect()
}

fn blocked_list(shell: &Shell, root: &Path) -> ItemListDto {
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    let query = TicketQuery {
        closure: ClosureFilter::Open,
        readiness: Some(ReadinessFilter::Blocked),
    };
    service.list_tickets(&repo, &query.to_filter()).unwrap()
}

/// Under Blocked, the panel below the list gives the selected row's
/// reasons in full, one per line: every `blocked_reasons` line of the same
/// read, as the arrows move the selection. With nothing selected it gives
/// a hint, and for an entry with no reasons it says so. The row carries
/// the same reasons as its accessible description.
#[gpui_kit::test]
fn the_why_blocked_panel_gives_the_selected_rows_reasons_in_full(cx: &mut TestAppContext) {
    let (shell, root) = related(cx);
    // Not under any other readiness.
    assert!(!shell.present("why-blocked", cx));
    shell.click("filter-blocked", cx);
    assert_eq!(
        shell.label("why-blocked-title", cx).as_deref(),
        Some(WHY_BLOCKED)
    );
    assert_eq!(
        shell.label("why-blocked-hint", cx).as_deref(),
        Some(SELECT_BLOCKED)
    );
    assert!(panel(&shell, cx).is_empty());

    let list = blocked_list(&shell, &root);
    let by_id = names(&shell, &root);
    let rows = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.tickets_pane().rows(cx));
    shell.keys("secondary-2", cx);
    let mut shown = Vec::new();
    for (index, item) in list.items.iter().enumerate() {
        shell.keys("down", cx);
        assert!(shell.selected(("ticket-row", index), cx), "{index}");
        let wanted = blocked_reasons(item, &by_id);
        shown.push(panel(&shell, cx));
        assert_eq!(shown[index], wanted, "{index}");
        assert_eq!(rows[index].reasons, wanted, "{index}");
        if wanted.is_empty() {
            assert_eq!(item.id, None, "only the nonconforming entry has none");
            assert_eq!(
                shell.label("why-blocked-hint", cx).as_deref(),
                Some(NO_REASONS)
            );
            assert_eq!(row_description(&rows[index]), None);
        } else {
            assert_eq!(shell.label("why-blocked-hint", cx), None);
            assert_eq!(row_description(&rows[index]), Some(wanted.join("; ")));
        }
    }

    // Each kind in full: the missing ULID, the blocker's title, and a
    // cycle row's two reasons, each on its own line.
    let at = |id: &str| {
        list.items
            .iter()
            .position(|item| item.id.as_deref() == Some(id))
            .unwrap()
    };
    assert_eq!(
        shown[at(DANGLING)],
        [format!("Waits for a ticket not found: {MISSING}")]
    );
    assert_eq!(
        shown[at(CYCLE_A)],
        [
            format!("Waits for open ticket: {CYCLE_B} Cycle B"),
            format!("On a dependency cycle: {CYCLE_A}, {CYCLE_B}"),
        ]
    );
}

/// A cycle longer than the sixteen tickets a reason names: the panel says
/// the reason is incomplete, as the library does.
#[gpui_kit::test]
fn the_panel_says_incomplete_where_the_library_does(cx: &mut TestAppContext) {
    const DIGITS: &str = "0123456789ABCDEFG";
    let ids: Vec<String> = DIGITS
        .chars()
        .map(|digit| format!("01ARZ3NDEKTSV4RRFFQ69G5FE{digit}"))
        .collect();
    let titles: Vec<String> = (0..ids.len()).map(|ix| format!("Ring {ix}")).collect();
    let ring: Vec<(&str, &str)> = ids
        .iter()
        .zip(&titles)
        .map(|(id, title)| (id.as_str(), title.as_str()))
        .collect();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_dependency_ring(&root, &ring);
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    shell.click(("repo-row", 0usize), cx);
    shell.click("pane-tickets", cx);
    shell.click("filter-blocked", cx);

    let list = blocked_list(&shell, &root);
    let by_id = names(&shell, &root);
    let first = &list.items[0];
    let reasons = &first.readiness.as_ref().unwrap().reasons;
    assert!(reasons.iter().any(|reason| !reason.complete), "{reasons:?}");

    shell.keys("secondary-2", cx);
    shell.keys("down", cx);
    let shown = panel(&shell, cx);
    assert_eq!(shown, blocked_reasons(first, &by_id));
    assert!(
        shown.iter().any(|line| line.ends_with(" (incomplete)")),
        "{shown:?}"
    );
}

/// No two choices of the navigation region share a label, so a screen
/// reader tells them apart, and each row of choices says in text which
/// is chosen: the pane, the closure and the readiness.
#[gpui_kit::test]
fn every_choice_has_its_own_label_and_the_chosen_one_is_said_in_text(cx: &mut TestAppContext) {
    let (shell, _) = four_tickets(cx);
    let ids = [
        "pane-documents",
        "pane-tickets",
        "filter-open",
        "filter-closed",
        "filter-all",
        "filter-any-readiness",
        "filter-ready",
        "filter-blocked",
    ];
    let labels: Vec<String> = ids
        .iter()
        .map(|id| shell.label(*id, cx).expect("the choice is drawn"))
        .collect();
    let mut unique = labels.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), labels.len(), "{labels:?}");
    assert_eq!(
        shell.label("filter-any-readiness", cx).as_deref(),
        Some(super::ANY_READINESS)
    );

    assert_eq!(
        shell.label("pane-showing", cx).as_deref(),
        Some(super::super::lists::SHOWING_TICKETS)
    );
    shell.click("pane-documents", cx);
    assert_eq!(
        shell.label("pane-showing", cx).as_deref(),
        Some(super::super::lists::SHOWING_DOCUMENTS)
    );
}

const WAITER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJZ";

/// A ticket waiting for more open tickets than the Why blocked panel has
/// room for: the panel keeps its height and scrolls. Tab reaches it, it
/// shows that it holds focus, and End and Home scroll it to its last and
/// first reason.
#[gpui_kit::test]
fn a_long_why_blocked_panel_scrolls_by_keyboard(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let blockers: Vec<String> = "0123456789ABCDEFGHJKMNPQ"
        .chars()
        .map(|digit| format!("01ARZ3NDEKTSV4RRFFQ69G5FH{digit}"))
        .collect();
    let mut deps = String::from("deps:\n");
    for (n, id) in blockers.iter().enumerate() {
        fixture.commit_ticket(&root, id, &format!("Blocker {n}"), "");
        deps.push_str(&format!("  - {id}\n"));
    }
    fixture.commit_ticket(&root, WAITER, "Waits for many", &deps);
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    shell.click(("repo-row", 0usize), cx);
    shell.click("pane-tickets", cx);
    shell.click("filter-blocked", cx);
    shell.keys("secondary-2 down", cx);
    let reasons = panel(&shell, cx);
    assert_eq!(reasons.len(), blockers.len());
    let last = ("why-blocked-reason", reasons.len() - 1);
    let first = ("why-blocked-reason", 0usize);

    let height = shell.frame(cx, |window, _| {
        window.find("why-blocked").bounds().size.height
    });
    assert!(
        // Its border is drawn outside the scrolled part.
        height <= super::WHY_BLOCKED_MAX_HEIGHT + gpui_kit::px(2.),
        "the panel keeps its height: {height:?}"
    );
    let visible = |shell: &Shell, id: (&'static str, usize), cx: &mut TestAppContext| {
        shell.frame(cx, |window, _| window.find(id).visible())
    };
    assert!(visible(&shell, first, cx));
    assert!(
        !visible(&shell, last, cx),
        "the last reason is scrolled away"
    );

    let panel_focused = |shell: &Shell, cx: &mut TestAppContext| {
        shell.frame(cx, |window, _| {
            window.find("why-blocked").focused() == Some(true)
        })
    };
    let mut presses = 0;
    while !panel_focused(&shell, cx) {
        presses += 1;
        assert!(presses < 20, "Tab never reached the Why blocked panel");
        shell.keys("tab", cx);
    }
    assert!(shell.present("why-blocked-focus-ring", cx));
    shell.keys("end", cx);
    assert!(visible(&shell, last, cx), "End shows the last reason");
    assert!(!visible(&shell, first, cx));
    shell.keys("home", cx);
    assert!(visible(&shell, first, cx), "Home shows the first again");
    shell.keys("pagedown", cx);
    assert!(!visible(&shell, first, cx), "PageDown moves it on");
}
