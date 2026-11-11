//! The ticket list: the closure filter keeps every nonconforming entry,
//! the rows are the library's, and the keyboard moves and opens.

use std::path::{Path, PathBuf};

use gpui_kit::{TestAppContext, test::TestWindowExt};
use manyhands::repository::{
    ClosureFilter, ItemDto, ItemDtoKind, ItemListDto, ReadinessFilter, TicketFilter,
};

use super::{NO_REASONS, SELECT_BLOCKED, WHY_BLOCKED, row_label};
use crate::desktop::{
    model::{
        navigation::{OpenTarget, Pane},
        ticket_view::{CLOSED_LINE, NEEDS_FIXING_PILL, NEEDS_FIXING_TYPE, ticket_tooltip},
        tickets::blocked_reasons,
    },
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
/// list shown.
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
    shell.use_list(Pane::Tickets, cx);
    (shell, root)
}

fn library(shell: &Shell, root: &Path, closure: ClosureFilter) -> ItemListDto {
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    let filter = TicketFilter {
        closure,
        ..TicketFilter::default()
    };
    service.list_tickets(&repo, &filter).unwrap()
}

/// What a test reads of one row: its label, its status pill and its
/// Closed marker.
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
                shell.label(("ticket-closed", index), cx),
            )
        })
        .collect()
}

/// What the row for `item` must show under `closure`: the status as
/// written, "needs fixing" for a nonconforming entry, and the Closed
/// marker only for a lifecycle-closed ticket under All.
fn expected(item: &ItemDto, closure: ClosureFilter) -> Seen {
    let pill = match item.id {
        Some(_) => item.status.clone(),
        None => Some(NEEDS_FIXING_PILL.to_owned()),
    };
    let closed = item
        .closure
        .as_ref()
        .is_some_and(|closure| closure.closed_at.is_some());
    (
        row_label(item),
        pill,
        (closed && closure == ClosureFilter::All).then(|| CLOSED_LINE.to_owned()),
    )
}

/// Chooses `readiness` through the menu of `filter-readiness`: a click opens
/// it, Down moves to the item and Enter chooses it. Focus is back on the
/// button.
fn choose_readiness(shell: &Shell, readiness: Option<ReadinessFilter>, cx: &mut TestAppContext) {
    let downs = match readiness {
        None => 1,
        Some(ReadinessFilter::Ready) => 2,
        Some(ReadinessFilter::Blocked) => 3,
    };
    shell.click("filter-readiness", cx);
    for _ in 0..downs {
        shell.keys("down", cx);
    }
    shell.keys("enter", cx);
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
        let library_rows: Vec<Seen> = list
            .items
            .iter()
            .map(|item| expected(item, closure))
            .collect();
        assert_eq!(seen, library_rows, "{filter}");
        let mut shown = labels(&seen);
        shown.sort_unstable();
        wanted.sort_unstable();
        assert_eq!(shown, wanted, "{filter}");
    }

    // An open ticket whose status says closed: the status as written, and
    // no Closed marker, which only a lifecycle-closed ticket gets.
    shell.click("filter-all", cx);
    let seen = visible(&shell, cx);
    let says = seen
        .iter()
        .find(|(label, ..)| label == "Says closed")
        .unwrap();
    assert_eq!(says.1.as_deref(), Some("closed"));
    assert_eq!(says.2, None);
    let closed = seen
        .iter()
        .find(|(label, ..)| label == "Closed for good")
        .unwrap();
    assert_eq!(closed.2.as_deref(), Some(CLOSED_LINE));
    // The nonconforming entry says so in text: its type cell and its
    // status pill.
    let at = labels(&seen)
        .iter()
        .position(|label| *label == broken)
        .unwrap();
    assert_eq!(
        shell.label(("ticket-type", at), cx).as_deref(),
        Some(NEEDS_FIXING_TYPE)
    );
    assert_eq!(
        shell.label(("ticket-status", at), cx).as_deref(),
        Some(NEEDS_FIXING_PILL)
    );
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
/// and a nonconforming entry. The Tickets list is shown.
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
    shell.use_list(Pane::Tickets, cx);
    (shell, root)
}

/// Review Focus 3, and no fabrication: under Ready and under Blocked the
/// rows are exactly those of the same `list_tickets` call, the
/// nonconforming entry among them; each blocked row gives its reasons in
/// Why blocked when it is selected.
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

    assert_eq!(
        shell.label("filter-readiness", cx).as_deref(),
        Some("Readiness: any")
    );
    for (name, readiness, mut wanted) in [
        (
            "Readiness: ready",
            ReadinessFilter::Ready,
            vec!["Blocker", "After done", broken.as_str()],
        ),
        (
            "Readiness: blocked",
            ReadinessFilter::Blocked,
            vec!["Waits", "Dangling", "Cycle A", "Cycle B", broken.as_str()],
        ),
    ] {
        choose_readiness(&shell, Some(readiness), cx);
        assert_eq!(shell.label("filter-readiness", cx).as_deref(), Some(name));
        let filter = TicketFilter {
            closure: ClosureFilter::Open,
            readiness: Some(readiness),
            ..TicketFilter::default()
        };
        let list = service.list_tickets(&repo, &filter).unwrap();
        let seen = visible(&shell, cx);
        let library_rows: Vec<Seen> = list
            .items
            .iter()
            .map(|item| expected(item, ClosureFilter::Open))
            .collect();
        assert_eq!(seen, library_rows, "{name}");
        let mut shown = labels(&seen);
        shown.sort_unstable();
        wanted.sort_unstable();
        assert_eq!(shown, wanted, "{name}");

        // Why blocked is under Blocked only; there each row, selected in
        // turn, gives its own reasons, and the nonconforming entry none.
        assert_eq!(
            shell.present("why-blocked", cx),
            readiness == ReadinessFilter::Blocked,
            "{name}"
        );
        if readiness == ReadinessFilter::Blocked {
            shell.keys("secondary-2", cx);
            for (index, item) in list.items.iter().enumerate() {
                shell.keys("down", cx);
                assert!(shell.selected(("ticket-row", index), cx), "{name} {index}");
                let drawn = panel(&shell, cx);
                assert_eq!(drawn, blocked_reasons(item, &by_id), "{name} {index}");
                assert_eq!(drawn.is_empty(), item.id.is_none(), "{name} {index}");
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
    let select = |index: usize, cx: &mut TestAppContext| {
        shell.keys("secondary-2", cx);
        // Up and down wrap, so the row is reached by the arrows from the top.
        shell.keys("home", cx);
        for _ in 0..index {
            shell.keys("down", cx);
        }
        assert!(shell.selected(("ticket-row", index), cx), "{index}");
        panel(&shell, cx)
    };
    assert_eq!(
        select(at(WAITS), cx),
        [format!("Waits for open ticket: {slug} Blocker")]
    );
    assert_eq!(
        select(at(DANGLING), cx),
        [format!("Waits for a ticket not found: {MISSING}")]
    );
    assert_eq!(
        select(at(CYCLE_B), cx),
        [
            format!("Waits for open ticket: {CYCLE_A} Cycle A"),
            format!("On a dependency cycle: {CYCLE_A}, {CYCLE_B}"),
        ]
    );

    // Any readiness again: Why blocked goes.
    choose_readiness(&shell, None, cx);
    assert_eq!(
        shell.label("filter-readiness", cx).as_deref(),
        Some("Readiness: any")
    );
    assert!(!shell.present("why-blocked", cx));
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
    let filter = TicketFilter {
        closure: ClosureFilter::Open,
        readiness: Some(ReadinessFilter::Blocked),
        ..TicketFilter::default()
    };
    service.list_tickets(&repo, &filter).unwrap()
}

/// Under Blocked, the panel below the list gives the selected row's
/// reasons in full, one per line: every `blocked_reasons` line of the same
/// read, as the arrows move the selection. With nothing selected it gives
/// a hint, and for an entry with no reasons it says so. The row's tooltip,
/// which is also its description, carries the same reasons.
#[gpui_kit::test]
fn the_why_blocked_panel_gives_the_selected_rows_reasons_in_full(cx: &mut TestAppContext) {
    let (shell, root) = related(cx);
    // Not under any other readiness.
    assert!(!shell.present("why-blocked", cx));
    choose_readiness(&shell, Some(ReadinessFilter::Blocked), cx);
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
            // The tooltip, and the description made of it, gives no reason.
            assert!(
                !ticket_tooltip(&rows[index], "a")
                    .iter()
                    .any(|line| line.starts_with("Waits for") || line.starts_with("On a")),
                "{index}"
            );
        } else {
            assert_eq!(shell.label("why-blocked-hint", cx), None);
            // The reasons are in the tooltip, and so in the description.
            let lines = ticket_tooltip(&rows[index], "a");
            assert!(
                wanted.iter().all(|reason| lines.contains(reason)),
                "{index}"
            );
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
    shell.use_list(Pane::Tickets, cx);
    choose_readiness(&shell, Some(ReadinessFilter::Blocked), cx);

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

/// No two choices of the Tickets region share a label, so a screen reader
/// tells them apart: each segment says its count, and the readiness menu's
/// button says which readiness is chosen, since the Kit does not expose the
/// checked item.
#[gpui_kit::test]
fn every_choice_has_its_own_label_and_the_chosen_one_is_said_in_text(cx: &mut TestAppContext) {
    let (shell, _) = four_tickets(cx);
    let ids = [
        "filter-open",
        "filter-closed",
        "filter-all",
        "filter-readiness",
    ];
    let labels: Vec<String> = ids
        .iter()
        .map(|id| shell.label(*id, cx).expect("the choice is drawn"))
        .collect();
    let mut unique = labels.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), labels.len(), "{labels:?}");
    // Two open tickets, one closed, and the nonconforming entry counted by
    // none.
    assert_eq!(
        labels,
        [
            "Open, 2 tickets",
            "Closed, 1 ticket",
            "All, 3 tickets",
            "Readiness: any"
        ]
    );
    assert!(shell.selected("filter-open", cx));
    assert!(!shell.selected("filter-closed", cx));
    choose_readiness(&shell, Some(ReadinessFilter::Ready), cx);
    assert_eq!(
        shell.label("filter-readiness", cx).as_deref(),
        Some("Readiness: ready")
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
    shell.use_list(Pane::Tickets, cx);
    choose_readiness(&shell, Some(ReadinessFilter::Blocked), cx);
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

// ---- the filter bar, list states and markers (Wave 04 Cycle 02, Task 3) ----

/// Whether the element `id` holds focus in the frame just drawn.
fn focused(shell: &Shell, id: &'static str, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |window, _| {
        window.try_find(id).and_then(|element| element.focused()) == Some(true)
    })
}

/// Presses `key`, Tab or shift-Tab, until the element `id` holds focus.
fn tab_to(shell: &Shell, key: &str, id: &'static str, cx: &mut TestAppContext) {
    for _ in 0..40 {
        if focused(shell, id, cx) {
            return;
        }
        shell.keys(key, cx);
    }
    assert!(focused(shell, id, cx), "{key} never reached {id}");
}

/// Whether the ticket list holds focus.
fn list_focused(shell: &Shell, cx: &mut TestAppContext) -> bool {
    let handle = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.tickets_list_focus(cx));
    shell.frame(cx, |window, _| handle.is_focused(window))
}

/// Whether the filter input holds focus.
fn input_focused(shell: &Shell, cx: &mut TestAppContext) -> bool {
    let input = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.find_input().clone());
    let handle = input.read_with(cx, gpui_kit::Focusable::focus_handle);
    shell.frame(cx, |window, _| handle.is_focused(window))
}

fn input_text(shell: &Shell, cx: &mut TestAppContext) -> String {
    let input = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.find_input().clone());
    input.read_with(cx, |input, _| input.value().to_string())
}

/// What a step of the journey must not change: the reads, the requests and
/// the open tabs.
#[derive(Debug, PartialEq)]
struct Quiet {
    reads: Vec<crate::desktop::services::ReadKind>,
    requests: Vec<crate::desktop::services::requests::DesktopRequestId>,
    tabs: Vec<String>,
}

fn quiet(shell: &Shell, cx: &mut TestAppContext) -> Quiet {
    Quiet {
        reads: shell.backend.reads_in_order(),
        requests: shell.requests(cx, |_| true),
        tabs: shell.rows("tab-title", cx),
    }
}

fn panel_count(shell: &Shell, cx: &mut TestAppContext) -> Option<String> {
    crate::desktop::view::dock::tests::support::panel_count(
        shell,
        crate::desktop::model::layout::PanelKind::Tickets,
        cx,
    )
}

const ALPHA: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK0";
const BETA: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK1";
const GAMMA: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK2";
const DELTA: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK3";
const SHARED_CLOSED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK4";
/// The short code Alpha and the closed ticket share.
const SHARED_CODE: &str = "fb-k9x2b";

/// Four open tickets, Gamma waiting for Alpha, and a lifecycle-closed one
/// that has Alpha's short code. The repository `a` is selected and the
/// Tickets list shown.
fn filter_tickets(cx: &mut TestAppContext) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, ALPHA, "Alpha", &format!("slug: {SHARED_CODE}\n"));
    fixture.commit_ticket(&root, BETA, "Beta", "");
    fixture.commit_ticket(&root, GAMMA, "Gamma", &format!("deps:\n  - {ALPHA}\n"));
    fixture.commit_ticket(&root, DELTA, "Delta", "");
    let closed = format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {SHARED_CLOSED}\n\
         title: Shared and closed\ntype: task\nstatus: done\nslug: {SHARED_CODE}\n\
         closed_at: 2026-09-30T12:34:56Z\nclosed_by: Ada Lovelace <ada@example.invalid>\n---\n\
         Body.\n"
    );
    fixture.commit_file(
        &root,
        &crate::desktop::test_support::fixtures::ticket_path(SHARED_CLOSED),
        closed.as_bytes(),
    );
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    shell.click(("repo-row", 0usize), cx);
    shell.use_list(Pane::Tickets, cx);
    (shell, root)
}

/// What the filter bar shows: the rows (in the order shown), the three
/// segments' names, the footnote and the header count.
fn bar(
    shell: &Shell,
    cx: &mut TestAppContext,
) -> (
    Vec<String>,
    [Option<String>; 3],
    Option<String>,
    Option<String>,
) {
    (
        shell.rows("ticket-row", cx),
        ["filter-open", "filter-closed", "filter-all"].map(|id| shell.label(id, cx)),
        shell.label("tickets-footnote", cx),
        panel_count(shell, cx),
    )
}

fn sorted(mut rows: Vec<String>) -> Vec<String> {
    rows.sort();
    rows
}

fn strings<const N: usize>(texts: [&str; N]) -> [Option<String>; N] {
    texts.map(|text| Some(text.to_owned()))
}

/// P1, P2, P7, P10 to P12, P18, P25, P29, P30: the lifecycle segments, the
/// readiness menu, text with its hint and Show all, and the sort headers
/// work by click and by key, and none of it reads, sends or opens anything
/// after the load. A filter that hides the selected row leaves Enter with
/// nothing to open.
#[gpui_kit::test]
fn the_filter_bar_works_without_a_read(cx: &mut TestAppContext) {
    use crate::desktop::services::ReadKind;

    let (shell, _root) = filter_tickets(cx);
    // One ticket read on load, whatever the filter.
    assert_eq!(shell.backend.reads_seen(ReadKind::Tickets), 1);
    let before = quiet(&shell, cx);
    let (rows, segments, footnote, count) = bar(&shell, cx);
    assert_eq!(sorted(rows), ["Alpha", "Beta", "Delta", "Gamma"]);
    assert_eq!(
        segments,
        strings(["Open, 4 tickets", "Closed, 1 ticket", "All, 5 tickets"])
    );
    assert_eq!(footnote.as_deref(), Some("4 open tickets in a"));
    assert_eq!(count.as_deref(), Some("4"));
    assert!(shell.selected("filter-open", cx));

    // Closed, by click.
    shell.click("filter-closed", cx);
    let (rows, segments, footnote, count) = bar(&shell, cx);
    assert_eq!(rows, ["Shared and closed"]);
    assert_eq!(
        segments,
        strings(["Open, 4 tickets", "Closed, 1 ticket", "All, 5 tickets"]),
        "the counts are the text's, not the lifecycle's"
    );
    assert_eq!(footnote.as_deref(), Some("1 closed ticket in a"));
    assert_eq!(count.as_deref(), Some("1"));
    assert!(shell.selected("filter-closed", cx));
    assert_eq!(quiet(&shell, cx), before);

    // All, by Tab and Enter.
    tab_to(&shell, "tab", "filter-all", cx);
    shell.press("enter", cx);
    let (rows, _, footnote, count) = bar(&shell, cx);
    assert_eq!(
        sorted(rows),
        ["Alpha", "Beta", "Delta", "Gamma", "Shared and closed"]
    );
    assert_eq!(footnote.as_deref(), Some("5 tickets in a"));
    assert_eq!(count.as_deref(), Some("5"));
    assert!(shell.selected("filter-all", cx));
    assert_eq!(quiet(&shell, cx), before);

    // Blocked, by keyboard: the button, Enter, Down three times, Enter.
    tab_to(&shell, "tab", "filter-readiness", cx);
    shell.keys("enter", cx);
    shell.keys("down down down enter", cx);
    assert_eq!(
        shell.label("filter-readiness", cx).as_deref(),
        Some("Readiness: blocked")
    );
    let (rows, _, footnote, count) = bar(&shell, cx);
    assert_eq!(rows, ["Gamma"]);
    assert_eq!(footnote.as_deref(), Some("1 blocked ticket in a"));
    assert_eq!(count.as_deref(), Some("1"));
    assert!(shell.present("why-blocked", cx));
    assert!(focused(&shell, "filter-readiness", cx), "focus is back");
    assert_eq!(quiet(&shell, cx), before);

    // Any readiness.
    shell.keys("enter down enter", cx);
    assert_eq!(
        shell.label("filter-readiness", cx).as_deref(),
        Some("Readiness: any")
    );
    assert!(!shell.present("why-blocked", cx));
    assert_eq!(bar(&shell, cx).0.len(), 5);
    assert_eq!(quiet(&shell, cx), before);

    // With Closed chosen, choosing Ready chooses Open (R-26).
    shell.click("filter-closed", cx);
    assert_eq!(bar(&shell, cx).0, ["Shared and closed"]);
    choose_readiness(&shell, Some(ReadinessFilter::Ready), cx);
    assert!(shell.selected("filter-open", cx));
    let (rows, _, footnote, count) = bar(&shell, cx);
    assert_eq!(sorted(rows), ["Alpha", "Beta", "Delta"]);
    assert_eq!(footnote.as_deref(), Some("3 ready tickets in a"));
    assert_eq!(count.as_deref(), Some("3"));
    assert_eq!(quiet(&shell, cx), before);

    // Under Open and Ready, a short code the closed ticket shares: the hint
    // names it, and Show all lists both.
    shell.keys("secondary-p", cx);
    assert!(input_focused(&shell, cx));
    shell.visual(cx).simulate_input(SHARED_CODE);
    shell.run(cx);
    let (rows, segments, footnote, count) = bar(&shell, cx);
    assert_eq!(rows, ["Alpha"]);
    assert_eq!(
        segments,
        strings(["Open, 1 ticket", "Closed, 1 ticket", "All, 2 tickets"])
    );
    assert_eq!(
        footnote.as_deref(),
        Some(format!("1 ready ticket in a matches \"{SHARED_CODE}\"").as_str())
    );
    assert_eq!(count.as_deref(), Some("1"));
    assert_eq!(
        shell.label("tickets-hint", cx).as_deref(),
        Some("1 more ticket matches with other filters.")
    );
    assert_eq!(quiet(&shell, cx), before);
    shell.click("tickets-show-all", cx);
    let (rows, _, _, count) = bar(&shell, cx);
    assert_eq!(sorted(rows), ["Alpha", "Shared and closed"]);
    assert_eq!(count.as_deref(), Some("2"));
    assert!(shell.selected("filter-all", cx));
    assert_eq!(
        shell.label("filter-readiness", cx).as_deref(),
        Some("Readiness: any")
    );
    assert!(!shell.present("tickets-hint", cx));
    assert!(input_focused(&shell, cx), "focus is in the filter input");
    assert_eq!(input_text(&shell, cx), SHARED_CODE, "the text is kept");
    assert_eq!(quiet(&shell, cx), before);

    // Back to every ticket, then the sorts. By click, ascending.
    shell.keys("secondary-a backspace", cx);
    assert_eq!(input_text(&shell, cx), "");
    assert_eq!(bar(&shell, cx).0.len(), 5);
    shell.click("sort-title", cx);
    assert_eq!(
        bar(&shell, cx).0,
        ["Alpha", "Beta", "Delta", "Gamma", "Shared and closed"]
    );
    assert_eq!(
        shell.label("sort-title", cx).as_deref(),
        Some("Sort by title, A to Z")
    );
    assert_eq!(
        shell.label("sort-updated", cx).as_deref(),
        Some("Sort by updated")
    );
    assert!(shell.selected("sort-title", cx));
    assert_eq!(quiet(&shell, cx), before);
    // By Tab and Enter, reversed.
    tab_to(&shell, "tab", "sort-title", cx);
    shell.press("enter", cx);
    assert_eq!(
        bar(&shell, cx).0,
        ["Shared and closed", "Gamma", "Delta", "Beta", "Alpha"]
    );
    assert_eq!(
        shell.label("sort-title", cx).as_deref(),
        Some("Sort by title, Z to A")
    );
    assert_eq!(quiet(&shell, cx), before);

    // Home and End in the list.
    shell.keys("secondary-2", cx);
    shell.keys("home", cx);
    assert!(shell.selected(("ticket-row", 0usize), cx));
    shell.keys("end", cx);
    assert!(shell.selected(("ticket-row", 4usize), cx));
    assert!(!shell.selected(("ticket-row", 0usize), cx));
    shell.keys("home", cx);
    assert!(shell.selected(("ticket-row", 0usize), cx));

    // Review Focus 1: the selected row is the closed ticket; Open hides it,
    // and Enter in the list opens nothing, not another ticket.
    assert_eq!(bar(&shell, cx).0[0], "Shared and closed");
    shell.click("filter-open", cx);
    assert_eq!(bar(&shell, cx).0, ["Gamma", "Delta", "Beta", "Alpha"]);
    shell.keys("secondary-2", cx);
    assert!(
        shell
            .workspace
            .read_with(cx, |workspace, cx| workspace
                .tickets_pane()
                .selected_row(cx))
            .is_none(),
        "the hidden row is no longer selected"
    );
    shell.keys("enter", cx);
    assert_eq!(
        shell
            .workspace
            .read_with(cx, |workspace, cx| workspace.opened(cx)),
        None
    );
    assert_eq!(quiet(&shell, cx), before);
    assert_eq!(shell.backend.reads_seen(ReadKind::Tickets), 1);
}

/// P14: an item with unsaved work is marked in its row at once, with no
/// read of the list.
#[gpui_kit::test]
async fn markers_appear_without_a_read(cx: &mut TestAppContext) {
    use crate::desktop::services::ReadKind;

    cx.executor().allow_parking();
    let (shell, root) = filter_tickets(cx);
    let row = |shell: &Shell, cx: &mut TestAppContext| {
        shell
            .rows("ticket-row", cx)
            .iter()
            .position(|label| label == "Beta")
            .expect("Beta is listed")
    };
    assert!(!shell.present(("ticket-unsaved", row(&shell, cx)), cx));

    // Opening the ticket, editing it and typing a change begins its draft
    // session, which holds unsaved work.
    let target = OpenTarget::Item {
        repo: key(&root),
        kind: ItemDtoKind::Ticket,
        id: BETA.parse().unwrap(),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
    shell.click("item-edit", cx);
    let host = shell.until_editing(cx).await;
    // Starting an edit is an operation of the repository, which reloads the
    // list as any does; the marker must not need another read.
    shell.run(cx);
    let reads = shell.backend.reads_seen(ReadKind::Tickets);
    shell.type_at_end(&host, " more", cx);
    shell
        .until_drawn(cx, "the unsaved marker", |shell, cx| {
            let at = row(shell, cx);
            shell.present(("ticket-unsaved", at), cx)
        })
        .await;

    let at = row(&shell, cx);
    assert_eq!(
        shell.label(("ticket-unsaved", at), cx).as_deref(),
        Some(crate::desktop::model::ticket_view::UNSAVED_MARKER)
    );
    // No other row is marked, and the list was not read again.
    let others = (0..bar(&shell, cx).0.len()).filter(|ix| *ix != at);
    for other in others {
        assert!(!shell.present(("ticket-unsaved", other), cx), "{other}");
    }
    assert_eq!(shell.backend.reads_seen(ReadKind::Tickets), reads);
}

/// The id of `name`'s child `part`, as the alert row and the empty state
/// give their action and detail.
fn child(name: &'static str, part: &'static str) -> gpui_kit::ElementId {
    (gpui_kit::ElementId::from(name), part).into()
}

/// Whether the Tickets region holds focus and draws its ring.
fn tickets_ringed(shell: &Shell, cx: &mut TestAppContext) -> bool {
    use crate::desktop::view::workspace::Region;
    shell.frame(cx, |window, _| {
        window.find(Region::Tickets.id()).focused() == Some(true)
            && window.try_find(Region::Tickets.ring_id()).is_some()
    })
}

/// P15, P16, P31: a reload over kept rows says "Updating…" and keeps them;
/// a busy one keeps them under an alert whose Retry reads once and keeps
/// the typed text and the rows; a failed one and an unavailable index say
/// so, with their action; an empty list is reached by F6 as any; Clear
/// filters, New ticket…, Refresh and Rebuild the search index each do
/// their own thing.
#[gpui_kit::test]
async fn list_states_act_and_a_reload_keeps_the_rows(cx: &mut TestAppContext) {
    use std::time::Duration;

    use manyhands::repository::{LeaseKind, RepositoryService};

    use crate::desktop::{
        model::list_view::{
            ListKind, NO_TICKETS_MATCH, busy_over_kept_text, empty_text, failed_text,
            index_unavailable_text, never_refreshed_text, stale_text,
        },
        services::{ReadKind, requests::Intent},
    };

    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.commit_ticket(&a, ALPHA, "Alpha", "");
    fixture.commit_ticket(&a, BETA, "Beta", "");
    fixture.refresh(&a);
    let b = fixture.create_repository("b");
    fixture.refresh(&b);
    // Stale, and never refreshed: states the library makes only from its
    // registry, which a test reaches beside the service's own connection.
    let stale = fixture.create_repository("s");
    fixture.commit_ticket(&stale, ALPHA, "Kept", "");
    fixture.refresh(&stale);
    let never = fixture.create_repository("n");
    fixture.commit_ticket(&never, ALPHA, "Gone", "");
    fixture.refresh(&never);
    {
        let registry = rusqlite::Connection::open(
            fixture
                .data_dir()
                .join(manyhands::repository::REGISTRY_FILE),
        )
        .unwrap();
        let stale_rows = registry
            .execute(
                "UPDATE repositories SET refresh_required = 1 WHERE root_path LIKE '%/s'",
                [],
            )
            .unwrap();
        let never_rows = registry
            .execute(
                "UPDATE repositories SET refreshed_at = NULL WHERE root_path LIKE '%/n'",
                [],
            )
            .unwrap();
        registry
            .execute(
                "DELETE FROM contexts WHERE repository_id IN \
                 (SELECT id FROM repositories WHERE root_path LIKE '%/n')",
                [],
            )
            .unwrap();
        assert_eq!((stale_rows, never_rows), (1, 1));
    }
    let shell = Shell::open(fixture, cx);
    shell.select(&a, cx);
    shell.use_list(Pane::Tickets, cx);
    shell.keys("secondary-p", cx);
    shell.visual(cx).simulate_input("a");
    shell.run(cx);
    let rows = |shell: &Shell, cx: &mut TestAppContext| sorted(shell.rows("ticket-row", cx));
    assert_eq!(rows(&shell, cx), ["Alpha", "Beta"]);
    assert_eq!(panel_count(&shell, cx).as_deref(), Some("2"));
    assert!(!shell.present("tickets-alert", cx));

    // A reload after a finished job: the old rows stay, the count says
    // Updating… until the read lands.
    let held = shell.backend.hold_read(ReadKind::Tickets);
    shell.keys("secondary-r", cx);
    let refresh = shell.requests(cx, |request| {
        matches!(request.intent, Intent::IndexRefresh { .. })
    })[0];
    shell.until_ended(refresh, cx).await;
    until_settled(&shell, cx, "the held reload", |shell, cx| {
        panel_count(shell, cx).as_deref() == Some("Updating\u{2026}")
    })
    .await;
    assert_eq!(rows(&shell, cx), ["Alpha", "Beta"]);
    assert!(!shell.present("tickets-skeleton", cx));
    let _ = held.send(());
    until_settled(&shell, cx, "the reload landed", |shell, cx| {
        panel_count(shell, cx).as_deref() == Some("2")
    })
    .await;
    assert_eq!(rows(&shell, cx), ["Alpha", "Beta"]);

    // A reload that stays busy keeps the rows under the busy alert.
    let held = shell.backend.hold_read(ReadKind::Tickets);
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.reload_lists(cx));
    shell.run(cx);
    let lease = RepositoryService::hold_lease_for_testing(
        &a,
        shell.fixture.data_dir(),
        LeaseKind::CacheWrite,
    )
    .unwrap();
    let _ = held.send(());
    until_settled(&shell, cx, "the busy alert", |shell, cx| {
        // The read waits 100, 200 and 400 ms on the executor's timer.
        cx.executor().advance_clock(Duration::from_secs(1));
        shell.present("tickets-alert", cx)
    })
    .await;
    assert_eq!(
        shell.label("tickets-alert", cx),
        Some(busy_over_kept_text("a"))
    );
    assert_eq!(rows(&shell, cx), ["Alpha", "Beta"], "the rows are kept");

    // Retry reads the tickets once, without clearing: while its read is
    // held the rows and the text stay, there is no skeleton, and the count
    // says Updating…. Focus is on the list.
    drop(lease);
    let reads = shell.backend.reads_seen(ReadKind::Tickets);
    let held = shell.backend.hold_read(ReadKind::Tickets);
    shell.click(child("tickets-alert", "action"), cx);
    assert_eq!(shell.backend.reads_seen(ReadKind::Tickets), reads + 1);
    assert_eq!(rows(&shell, cx), ["Alpha", "Beta"], "kept while held");
    assert!(!shell.present("tickets-skeleton", cx));
    assert_eq!(input_text(&shell, cx), "a");
    assert_eq!(panel_count(&shell, cx).as_deref(), Some("Updating\u{2026}"));
    assert!(list_focused(&shell, cx), "Retry focuses the list");
    let _ = held.send(());
    until_settled(&shell, cx, "Retry landed", |shell, cx| {
        !shell.present("tickets-alert", cx)
    })
    .await;
    assert_eq!(shell.backend.reads_seen(ReadKind::Tickets), reads + 1);
    assert_eq!(rows(&shell, cx), ["Alpha", "Beta"]);
    assert_eq!(input_text(&shell, cx), "a");

    // A narrowed list with nothing in it: Clear filters empties the text,
    // chooses Any readiness and focuses the input.
    choose_readiness(&shell, Some(ReadinessFilter::Ready), cx);
    shell.keys("secondary-p", cx);
    shell.keys("secondary-a backspace", cx);
    shell.visual(cx).simulate_input("zzz");
    shell.run(cx);
    assert_eq!(
        shell.label("tickets-empty", cx).as_deref(),
        Some(NO_TICKETS_MATCH)
    );
    shell.click(child("tickets-empty", "action"), cx);
    assert_eq!(input_text(&shell, cx), "");
    assert_eq!(
        shell.label("filter-readiness", cx).as_deref(),
        Some("Readiness: any")
    );
    assert!(input_focused(&shell, cx));
    assert_eq!(rows(&shell, cx), ["Alpha", "Beta"]);

    // An empty repository: its empty state offers New ticket…, which opens
    // the dialog, and F6 reaches its list.
    shell.select(&b, cx);
    until_settled(&shell, cx, "the empty state", |shell, cx| {
        shell.present("tickets-empty", cx)
    })
    .await;
    assert_eq!(
        shell.label("tickets-empty", cx),
        Some(empty_text(
            ListKind::Tickets,
            manyhands::repository::ClosureFilter::Open,
            "b"
        ))
    );
    shell.click(child("tickets-empty", "action"), cx);
    assert!(shell.present("create-dialog", cx));
    shell.keys("escape", cx);
    assert!(!shell.present("create-dialog", cx));
    shell.keys("secondary-3", cx);
    shell.keys("f6", cx);
    assert!(tickets_ringed(&shell, cx), "F6 reaches the empty list");

    // A stale list keeps its rows under an alert, and a list never
    // refreshed has none; each offers Refresh, which submits a refresh and
    // focuses the list.
    for (name, repo, alert, kept) in [
        ("s", &stale, stale_text("s"), true),
        ("n", &never, never_refreshed_text("n"), false),
    ] {
        shell.select(repo, cx);
        until_settled(&shell, cx, "the alert", |shell, cx| {
            shell.present("tickets-alert", cx)
        })
        .await;
        assert_eq!(shell.label("tickets-alert", cx), Some(alert), "{name}");
        assert_eq!(
            shell.rows("ticket-row", cx).len(),
            usize::from(kept),
            "{name}"
        );
        let refreshes = |shell: &Shell, cx: &mut TestAppContext| {
            shell.requests(cx, |request| {
                matches!(request.intent, Intent::IndexRefresh { .. })
            })
        };
        let before = refreshes(&shell, cx).len();
        shell.click(child("tickets-alert", "action"), cx);
        let after = refreshes(&shell, cx);
        assert_eq!(
            after.len(),
            before + 1,
            "{name}: Refresh submitted a refresh"
        );
        assert!(list_focused(&shell, cx), "{name}: focus is on the list");
        shell.until_ended(after[0], cx).await;
    }

    // Back to `a`, whose lists the next two states replace.
    shell.select(&a, cx);
    until_settled(&shell, cx, "a's rows", |shell, cx| {
        !shell.rows("ticket-row", cx).is_empty()
    })
    .await;

    // A failed read says so, with its message on a visible line, and no
    // row; F6 into it keeps the region's ring.
    let elsewhere = shell.fixture.empty_directory("other-data");
    shell
        .backend
        .replace_service(RepositoryService::open_at(&elsewhere).unwrap());
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.reload_lists(cx));
    until_settled(&shell, cx, "the failure", |shell, cx| {
        shell.present("tickets-alert", cx)
    })
    .await;
    assert_eq!(
        shell.label("tickets-alert", cx),
        Some(failed_text(ListKind::Tickets, "a"))
    );
    assert!(shell.label(child("tickets-alert", "detail"), cx).is_some());
    assert!(rows(&shell, cx).is_empty());
    shell.keys("secondary-3", cx);
    shell.keys("f6", cx);
    assert!(tickets_ringed(&shell, cx), "F6 reaches the failed list");

    // An index that cannot be read offers to rebuild it. Its job is held
    // at the library, so no rebuild runs on the unreadable index.
    let degraded = shell.fixture.empty_directory("degraded-data");
    std::fs::write(degraded.join("manyhands.sqlite3"), b"not sqlite").unwrap();
    shell
        .backend
        .replace_service(RepositoryService::open_at(&degraded).unwrap());
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.reload_lists(cx));
    until_settled(&shell, cx, "the unavailable index", |shell, cx| {
        shell.label("tickets-alert", cx) == Some(index_unavailable_text("a"))
    })
    .await;
    let gate = shell
        .backend
        .hold_job(crate::desktop::services::JobKind::IndexRebuild);
    shell.click(child("tickets-alert", "action"), cx);
    let rebuild = shell.requests(cx, |request| {
        matches!(request.intent, Intent::IndexRebuild { .. })
    });
    assert_eq!(
        rebuild.len(),
        1,
        "Rebuild the search index submitted a rebuild"
    );
    assert!(list_focused(&shell, cx), "Rebuild focuses the list");
    gate.release();
    shell.until_ended(rebuild[0], cx).await;
}

/// Draws frames until `done`, as `until_drawn` does, moving the executor's
/// clock on at each look: a read that met a repository busy with the
/// worker's own job waits on the timer to try again, and no real time is
/// waited for it.
async fn until_settled(
    shell: &Shell,
    cx: &mut TestAppContext,
    what: &str,
    done: impl Fn(&Shell, &mut TestAppContext) -> bool,
) {
    shell
        .until_drawn(cx, what, |shell, cx| {
            cx.executor()
                .advance_clock(std::time::Duration::from_secs(1));
            done(shell, cx)
        })
        .await;
}

const UNREADABLE_TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM0";
const FINE_DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM1";

/// P1, P28, R-38: a ticket file and a document file the library cannot read
/// are listed in their lists with their reason, once a refresh has met them,
/// and they do not open. The lists read the status once per load. A held
/// status read leaves them as they were, and so does one that stays busy
/// (Part I). The ticket is a raw directory with no item worktree, since
/// discovery drops a primary copy's problems inside a ticket directory that
/// has one (`L/repository/discovery.rs:802-804`).
#[gpui_kit::test]
async fn an_unreadable_file_is_listed_with_its_reason(cx: &mut TestAppContext) {
    use std::time::Duration;

    use manyhands::repository::{LeaseKind, RepositoryService};

    use crate::desktop::{
        model::{ticket_view::CANT_BE_READ_PILL, unreadable::UnreadableFile},
        services::{ReadKind, requests::Intent},
    };

    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, ALPHA, "Alpha", "");
    fixture.create_document(&root, FINE_DOCUMENT, "docs/fine.md", "Fine");
    fixture.refresh(&root);
    // Written after the refresh, so the index does not know them yet.
    let ticket_dir = root.join(".manyhands/tickets").join(UNREADABLE_TICKET);
    std::fs::create_dir_all(&ticket_dir).unwrap();
    std::fs::write(ticket_dir.join("ticket.md"), [0xff, 0xfe, 0x00]).unwrap();
    std::fs::create_dir_all(root.join("docs")).unwrap();
    fixture.write_non_utf8(&root, "docs/broken.md");
    let ticket_path = format!(".manyhands/tickets/{UNREADABLE_TICKET}/ticket.md");

    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    shell.use_list(Pane::Tickets, cx);
    assert_eq!(shell.rows("ticket-row", cx), ["Alpha"]);
    assert_eq!(shell.rows("doc-node", cx), ["Fine"]);
    // One status read with the lists' load (P1).
    let status_reads = |shell: &Shell| shell.backend.reads_seen(ReadKind::IndexStatus);
    assert_eq!(status_reads(&shell), 1);
    let list_reads = shell.backend.reads_seen(ReadKind::Tickets);

    // A refresh meets the files; the lists reload with one more status read.
    // It is held, so the lists land first, as they may in any order: the
    // files come in alone, and each list shows them (P1).
    let held = shell.backend.hold_read(ReadKind::IndexStatus);
    shell.keys("secondary-r", cx);
    let refresh = shell.requests(cx, |request| {
        matches!(request.intent, Intent::IndexRefresh { .. })
    })[0];
    shell.until_ended(refresh, cx).await;
    until_settled(&shell, cx, "the lists reloaded", |shell, cx| {
        panel_count(shell, cx).as_deref() == Some("1")
            && shell.backend.reads_seen(ReadKind::IndexStatus) == 2
    })
    .await;
    assert_eq!(shell.rows("ticket-row", cx), ["Alpha"]);
    assert_eq!(shell.rows("doc-node", cx), ["Fine"]);
    let _ = held.send(());
    until_settled(&shell, cx, "the files listed", |shell, cx| {
        shell.rows("ticket-row", cx).len() == 2 && shell.rows("doc-node", cx).len() == 2
    })
    .await;
    assert_eq!(status_reads(&shell), 2, "one more with the lists' reload");
    assert_eq!(shell.backend.reads_seen(ReadKind::Tickets), list_reads + 1);

    // Under every lifecycle the file is listed by its path, "can't be read".
    let unreadable_row = |shell: &Shell, cx: &mut TestAppContext| {
        let at = shell
            .rows("ticket-row", cx)
            .iter()
            .position(|label| *label == ticket_path)
            .expect("the file is listed by its path");
        (at, shell.label(("ticket-status", at), cx))
    };
    for (filter, rows) in [
        ("filter-open", 2),
        ("filter-closed", 1),
        ("filter-all", 2),
        ("filter-open", 2),
    ] {
        shell.click(filter, cx);
        assert_eq!(shell.rows("ticket-row", cx).len(), rows, "{filter}");
        let (_, pill) = unreadable_row(&shell, cx);
        assert_eq!(pill.as_deref(), Some(CANT_BE_READ_PILL), "{filter}");
    }
    let footnote = shell.label("tickets-footnote", cx).unwrap();
    assert!(footnote.ends_with(" \u{b7} 1 can't be read"), "{footnote}");
    assert_eq!(panel_count(&shell, cx).as_deref(), Some("2"));

    // What the files are: the library puts them in the primary worktree,
    // which is the repository's root, so both are primary. The raw ticket
    // directory is in the primary copy, not an item worktree.
    let root_text = root.to_str().unwrap().to_owned();
    let file = |path: &str| UnreadableFile {
        path: path.to_owned(),
        worktree: root_text.clone(),
        primary: true,
    };
    shell.workspace.read_with(cx, |workspace, _| {
        assert_eq!(
            workspace.tickets().unreadable(),
            [file(&ticket_path)],
            "the ticket file"
        );
        assert_eq!(
            workspace.documents().unreadable(),
            [file("docs/broken.md")],
            "the document file"
        );
    });

    // The document is listed by its file name, with its own mark.
    assert_eq!(shell.rows("doc-node", cx), ["broken.md", "Fine"]);
    assert!(shell.present(("doc-unreadable", 0usize), cx));
    assert!(!shell.present(("doc-unreadable", 1usize), cx));

    // A click selects each and Enter opens neither (R-38).
    let opened = |shell: &Shell, cx: &mut TestAppContext| {
        shell
            .workspace
            .read_with(cx, |workspace, cx| workspace.opened(cx))
    };
    let (at, _) = unreadable_row(&shell, cx);
    shell.click(("ticket-row", at), cx);
    assert!(shell.selected(("ticket-row", at), cx));
    shell.keys("enter", cx);
    assert_eq!(opened(&shell, cx), None, "a ticket file");
    assert!(shell.rows("tab-title", cx).is_empty());
    shell.use_list(Pane::Documents, cx);
    shell.click(("doc-node", 0usize), cx);
    assert!(shell.selected(("doc-node", 0usize), cx));
    shell.keys("enter", cx);
    assert_eq!(opened(&shell, cx), None, "a document file");
    assert!(shell.rows("tab-title", cx).is_empty());

    // A reload whose status read is held lands the lists, and the files
    // stay listed as they were (P28).
    let rows = |shell: &Shell, cx: &mut TestAppContext| {
        (
            shell.rows("ticket-row", cx),
            shell.rows("doc-node", cx),
            shell.present(("doc-unreadable", 0usize), cx),
            shell
                .label("tickets-footnote", cx)
                .is_some_and(|text| text.ends_with(" \u{b7} 1 can't be read")),
        )
    };
    let before = rows(&shell, cx);
    assert_eq!(before.0.len(), 2);
    let held = shell.backend.hold_read(ReadKind::IndexStatus);
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.reload_lists(cx));
    until_settled(&shell, cx, "the lists landed", |shell, cx| {
        panel_count(shell, cx).as_deref() == Some("2")
    })
    .await;
    assert_eq!(status_reads(&shell), 3);
    assert_eq!(rows(&shell, cx), before, "held");

    // Released while the repository is busy, the status read stays busy; it
    // never fails the lists, and the files stay.
    let lease = RepositoryService::hold_lease_for_testing(
        &root,
        shell.fixture.data_dir(),
        LeaseKind::CacheWrite,
    )
    .unwrap();
    let _ = held.send(());
    for _ in 0..4 {
        cx.executor().advance_clock(Duration::from_secs(1));
        shell.run(cx);
    }
    drop(lease);
    assert!(!shell.present("tickets-alert", cx));
    assert!(!shell.present("documents-alert", cx));
    assert_eq!(rows(&shell, cx), before, "busy");

    // One that fails does the same. Only the status read can be made to
    // fail: it is held while the lists land, then goes to a service whose
    // registry does not know the repository.
    let held = shell.backend.hold_read(ReadKind::IndexStatus);
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.reload_lists(cx));
    until_settled(&shell, cx, "the lists landed", |shell, cx| {
        panel_count(shell, cx).as_deref() == Some("2")
    })
    .await;
    shell.backend.replace_service(
        RepositoryService::open_at(&shell.fixture.parent().join("other-registry")).unwrap(),
    );
    let _ = held.send(());
    shell.run(cx);
    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());
    assert!(!shell.present("tickets-alert", cx));
    assert!(!shell.present("documents-alert", cx));
    assert_eq!(rows(&shell, cx), before, "failed");
}
