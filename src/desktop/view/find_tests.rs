//! The filter input's Enter over real repositories: an ID opens its ticket
//! or document whatever the filters, held or not; the one ticket a code
//! matches opens even where a filter hides it; several matches move focus
//! to the list; and a change of repository drops the text and a held read,
//! which a Retry does not.

use std::path::PathBuf;

use gpui_kit::{Focusable, TestAppContext};
use manyhands::{repository::ItemDtoKind, results::ResultCode};

use super::{FIND_NO_REPOSITORY, FIND_OPENING, busy_text, nothing_found_text, status_text};
use crate::desktop::{
    model::{
        find::{FindResult, FindState},
        list_view::NO_TICKETS_MATCH,
        navigation::OpenTarget,
        tabs::TabKey,
    },
    services::{ReadFailure, ReadKind},
    test_support::fixtures::{Fixture, item_id},
    view::{
        harness::{Shell, key},
        workspace::Region,
    },
};

const OPEN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK0";
const CLOSED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK1";
const FIRST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK2";
const SECOND: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK3";
const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK4";
const UNKNOWN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK9";
const SHARED_SLUG: &str = "ae-k9x2b";
/// The short code only the closed ticket carries.
const CLOSED_SLUG: &str = "cl-q7m4d";

/// An open ticket made through the library, a lifecycle-closed one with a
/// short code of its own, two committed tickets carrying the same short
/// code and a document; refreshed and selected. The lists show Open.
fn shell(cx: &mut TestAppContext) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_ticket(&root, OPEN, "Open one", "open");
    fixture.commit_closed_ticket_with_slug(&root, CLOSED, "Closed one", CLOSED_SLUG);
    fixture.commit_duplicate_slug(
        &root,
        (FIRST, "First sharer"),
        (SECOND, "Second sharer"),
        SHARED_SLUG,
    );
    fixture.create_document(&root, DOCUMENT, "docs/guide.md", "Guide");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    (shell, root)
}

/// `secondary-p`, then `text` typed over what the input held.
fn type_filter(shell: &Shell, text: &str, cx: &mut TestAppContext) {
    shell.keys("secondary-p", cx);
    let input = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.find_input().clone());
    let mut visual = shell.visual(cx);
    visual.update(|window, cx| {
        input.update(cx, |input, cx| input.set_value("", window, cx));
    });
    visual.simulate_input(text);
    visual.run_until_parked();
}

/// `type_filter`, then Enter.
fn find(shell: &Shell, text: &str, cx: &mut TestAppContext) {
    type_filter(shell, text, cx);
    shell.keys("enter", cx);
}

fn status(shell: &Shell, cx: &mut TestAppContext) -> Option<String> {
    shell.label("filter-status", cx)
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

fn item(root: &std::path::Path, kind: ItemDtoKind, id: &str) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind,
        id: item_id(id),
    }
}

fn ticket(root: &std::path::Path, id: &str) -> OpenTarget {
    item(root, ItemDtoKind::Ticket, id)
}

fn input_focused(shell: &Shell, cx: &mut TestAppContext) -> bool {
    let input = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.find_input().clone());
    shell.frame(cx, |window, cx| {
        input.read(cx).focus_handle(cx).is_focused(window)
    })
}

fn list_focused(shell: &Shell, cx: &mut TestAppContext) -> bool {
    let list = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.tickets_list_focus(cx));
    shell.frame(cx, |window, _| list.is_focused(window))
}

fn input_text(shell: &Shell, cx: &mut TestAppContext) -> String {
    let input = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.find_input().clone());
    input.read_with(cx, |input, _| input.value().to_string())
}

/// The text the query filters by.
fn query_text(shell: &Shell, cx: &mut TestAppContext) -> String {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.tickets().query().text.clone())
}

/// `secondary-p` focuses the filter input, inside the Tickets region,
/// whose ring says so.
#[gpui_kit::test]
fn secondary_p_focuses_the_filter_input(cx: &mut TestAppContext) {
    let (shell, _) = shell(cx);
    assert!(!input_focused(&shell, cx));
    shell.keys("secondary-p", cx);
    assert!(input_focused(&shell, cx));
    assert!(shell.present(Region::Tickets.ring_id(), cx));
}

/// P8, Review Focus 2: under Open, a closed ticket's ULID, pasted in
/// lowercase with spaces, opens its tab and says "Opening…" while its read
/// is held; focus stays in the input. A document's ULID opens its tab. No
/// filter changes.
#[gpui_kit::test]
fn a_ulid_opens_its_item_whatever_the_filters(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx);
    assert!(shell.selected("filter-open", cx));
    assert!(
        !shell
            .rows("ticket-row", cx)
            .contains(&"Closed one".to_owned())
    );

    let held = shell.backend.hold_read(ReadKind::Item);
    find(&shell, &format!("  {}  ", CLOSED.to_ascii_lowercase()), cx);
    assert_eq!(status(&shell, cx).as_deref(), Some(FIND_OPENING));
    assert!(tab_keys(&shell, cx).is_empty(), "nothing opens while held");
    assert!(input_focused(&shell, cx), "focus stays in the input");

    let _ = held.send(());
    shell.run(cx);
    assert_eq!(opened(&shell, cx), Some(ticket(&root, CLOSED)));
    assert_eq!(shell.label("item-title", cx).as_deref(), Some("Closed one"));
    assert_eq!(status(&shell, cx), None, "nothing is said once it opens");
    assert!(input_focused(&shell, cx), "focus stays in the input");
    assert!(shell.selected("filter-open", cx), "the filters are kept");

    find(&shell, DOCUMENT, cx);
    assert_eq!(
        opened(&shell, cx),
        Some(item(&root, ItemDtoKind::Document, DOCUMENT))
    );
    assert_eq!(status(&shell, cx), None);
    assert!(input_focused(&shell, cx));
    assert_eq!(
        tab_keys(&shell, cx),
        [
            ticket(&root, CLOSED),
            item(&root, ItemDtoKind::Document, DOCUMENT)
        ]
        .map(|target| TabKey::for_target(&target))
    );
}

/// An ID nothing in the repository has says so and opens nothing; text
/// that matches no ticket says that in the list, and clears what the ID
/// said. With nothing listed, Enter moves nothing.
#[gpui_kit::test]
fn nothing_found_says_so_and_opens_nothing(cx: &mut TestAppContext) {
    let (shell, _) = shell(cx);
    find(&shell, UNKNOWN, cx);
    assert_eq!(
        status(&shell, cx).as_deref(),
        Some("Nothing in a has that ID.")
    );
    assert!(tab_keys(&shell, cx).is_empty());
    assert_eq!(opened(&shell, cx), None);
    assert!(input_focused(&shell, cx));

    find(&shell, "zz-zzzzz", cx);
    assert_eq!(status(&shell, cx), None, "the ID's answer is stale");
    assert_eq!(
        shell.label("tickets-empty", cx).as_deref(),
        Some(NO_TICKETS_MATCH)
    );
    assert!(tab_keys(&shell, cx).is_empty());
    assert!(input_focused(&shell, cx), "no rows, so focus stays");
}

/// What the status line says of each state of the ID read.
#[test]
fn the_status_line_says_where_the_id_read_is() {
    let failure = ReadFailure {
        code: ResultCode::IndexUnavailable,
        recovery: Vec::new(),
    };
    let table = [
        (FindState::Idle, None),
        (FindState::NoRepository, Some(FIND_NO_REPOSITORY.to_owned())),
        (FindState::Searching, Some("Opening\u{2026}".to_owned())),
        (FindState::Done(FindResult::Opened), None),
        (
            FindState::Done(FindResult::None),
            Some("Nothing in manyhands has that ID.".to_owned()),
        ),
        (
            FindState::Busy,
            Some("manyhands is busy right now.".to_owned()),
        ),
        (
            FindState::Failed(failure.clone()),
            Some(failure.code.message().to_owned()),
        ),
    ];
    for (state, text) in table {
        assert_eq!(status_text(&state, "manyhands"), text, "{state:?}");
    }
    assert_eq!(nothing_found_text("b"), "Nothing in b has that ID.");
    assert_eq!(busy_text("b"), "b is busy right now.");
}

/// A read held while the user selects another repository never lands
/// there: no tab, and nothing said. It is dropped at the selection, not
/// when the new repository's lists load: the new repository's resolution
/// is held here, and the read still lands on nothing.
#[gpui_kit::test]
fn a_held_id_read_is_dropped_when_another_repository_is_selected(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let b = fixture.create_repository("b");
    fixture.create_ticket(&a, OPEN, "Open one", "open");
    fixture.refresh(&a);
    let shell = Shell::open(fixture, cx);
    shell.select(&a, cx);
    let held = shell.backend.hold_read(ReadKind::Item);
    find(&shell, OPEN, cx);
    assert_eq!(status(&shell, cx).as_deref(), Some(FIND_OPENING));
    let resolving = shell.backend.hold_read(ReadKind::Resolve);
    shell.select(&b, cx);
    assert_eq!(status(&shell, cx), None, "dropped at the selection");
    let _ = held.send(());
    shell.run(cx);
    assert!(tab_keys(&shell, cx).is_empty());
    assert_eq!(opened(&shell, cx), None);
    assert_eq!(status(&shell, cx), None);
    let _ = resolving.send(());
    shell.run(cx);
    assert!(tab_keys(&shell, cx).is_empty());
}

/// P9: a code only the closed ticket carries, under Open, opens it, and
/// focus stays in the input; so does a title only one ticket has, in any
/// case. The filters stay Open.
#[gpui_kit::test]
fn the_one_match_opens_even_where_a_filter_hides_it(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx);
    find(&shell, CLOSED_SLUG, cx);
    assert!(
        shell.rows("ticket-row", cx).is_empty(),
        "Open lists none of it"
    );
    assert_eq!(opened(&shell, cx), Some(ticket(&root, CLOSED)));
    assert_eq!(shell.label("item-title", cx).as_deref(), Some("Closed one"));
    assert!(input_focused(&shell, cx), "focus stays in the input");
    assert_eq!(status(&shell, cx), None);
    assert!(shell.selected("filter-open", cx));

    find(&shell, "OPEN ONE", cx);
    assert_eq!(opened(&shell, cx), Some(ticket(&root, OPEN)));
    assert!(input_focused(&shell, cx));
    assert_eq!(
        tab_keys(&shell, cx),
        [CLOSED, OPEN].map(|id| TabKey::for_target(&ticket(&root, id)))
    );
}

/// P9: a code two tickets carry opens neither: focus moves to the list
/// with the first row selected, and Enter there opens that row by its ID.
#[gpui_kit::test]
fn several_matches_move_focus_to_the_list_on_the_first(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx);
    find(&shell, &SHARED_SLUG.to_ascii_uppercase(), cx);
    assert!(tab_keys(&shell, cx).is_empty());
    assert_eq!(opened(&shell, cx), None);
    assert!(list_focused(&shell, cx), "focus is in the list");
    assert!(shell.present(Region::Tickets.ring_id(), cx));
    let titles = shell.rows("ticket-row", cx);
    assert_eq!(titles.len(), 2);
    assert!(shell.selected(("ticket-row", 0usize), cx));
    assert!(!shell.selected(("ticket-row", 1usize), cx));

    let first = if titles[0] == "First sharer" {
        FIRST
    } else {
        SECOND
    };
    shell.press("enter", cx);
    assert_eq!(opened(&shell, cx), Some(ticket(&root, first)));
    // Never keyed by the short code.
    for key in tab_keys(&shell, cx) {
        let TabKey::Item { id, .. } = key else {
            panic!("a ticket's tab is keyed by its ID");
        };
        assert_eq!(id, first);
    }
}

/// Enter on blank text opens nothing, and drops what an ID read said
/// before; with rows listed focus goes to the list, as it does for several
/// matches.
#[gpui_kit::test]
fn blank_text_opens_nothing(cx: &mut TestAppContext) {
    let (shell, _) = shell(cx);
    find(&shell, UNKNOWN, cx);
    assert!(status(&shell, cx).is_some());
    find(&shell, "   ", cx);
    assert!(tab_keys(&shell, cx).is_empty());
    assert_eq!(opened(&shell, cx), None);
    assert_eq!(status(&shell, cx), None);
    assert!(list_focused(&shell, cx));
    assert!(shell.selected(("ticket-row", 0usize), cx));
}

/// With no repository selected there is nothing to find in, and the
/// status line says so.
#[gpui_kit::test]
fn enter_needs_a_selected_repository(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    fixture.create_repository("a");
    let shell = Shell::open(fixture, cx);
    find(&shell, OPEN, cx);
    assert_eq!(status(&shell, cx).as_deref(), Some(FIND_NO_REPOSITORY));
    assert!(tab_keys(&shell, cx).is_empty());
}

/// P17, Review Focus 4: selecting another repository clears the input and
/// the query's text and draws no row of the first repository, while the
/// lifecycle stays; the list is read afresh. The text is cleared where
/// there is a window to clear it in, the next render.
#[gpui_kit::test]
fn selecting_another_repository_clears_the_text_and_its_rows(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let b = fixture.create_repository("b");
    fixture.create_ticket(&a, OPEN, "Open one", "open");
    fixture.refresh(&a);
    let shell = Shell::open(fixture, cx);
    shell.select(&a, cx);
    shell.click("filter-all", cx);
    type_filter(&shell, "open", cx);
    assert_eq!(shell.rows("ticket-row", cx), ["Open one"]);
    assert_eq!(query_text(&shell, cx), "open");

    // b's list is held: nothing of a's is drawn meanwhile.
    let held = shell.backend.hold_read(ReadKind::Tickets);
    shell.select(&b, cx);
    assert!(shell.rows("ticket-row", cx).is_empty());
    assert_eq!(input_text(&shell, cx), "");
    assert_eq!(query_text(&shell, cx), "");
    let _ = held.send(());
    shell.run(cx);
    assert!(shell.rows("ticket-row", cx).is_empty());
    assert_eq!(input_text(&shell, cx), "");
    assert_eq!(query_text(&shell, cx), "");
    assert!(
        shell.selected("filter-all", cx),
        "the lifecycle carries over"
    );

    // Back in a, the text is not brought back.
    shell.select(&a, cx);
    assert_eq!(shell.rows("ticket-row", cx), ["Open one"]);
    assert_eq!(input_text(&shell, cx), "");
}

/// The text is cleared once per change of repository: text typed while the
/// new repository is still resolving is kept when its lists load. The
/// first selection of all is a change too: text typed before any
/// repository was selected is cleared.
#[gpui_kit::test]
fn the_text_is_cleared_once_per_change_of_repository(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let b = fixture.create_repository("b");
    let shell = Shell::open(fixture, cx);
    type_filter(&shell, "before", cx);
    shell.select(&a, cx);
    assert_eq!(
        input_text(&shell, cx),
        "",
        "the first selection is a change"
    );
    assert_eq!(query_text(&shell, cx), "");

    let resolving = shell.backend.hold_read(ReadKind::Resolve);
    shell.select(&b, cx);
    type_filter(&shell, "keep", cx);
    let _ = resolving.send(());
    shell.run(cx);
    assert_eq!(
        input_text(&shell, cx),
        "keep",
        "b's load is the same change"
    );
    assert_eq!(query_text(&shell, cx), "keep");
}

/// A Retry of the same repository keeps the text, the rows and a held ID
/// read: the read lands and opens its tab. A Retry reloads the lists, and
/// the Documents panel's Retry loads them afresh, clearing them first;
/// neither is a change of repository.
#[gpui_kit::test]
fn a_retry_keeps_the_text_and_a_held_id_read(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx);
    let held = shell.backend.hold_read(ReadKind::Item);
    find(&shell, OPEN, cx);
    assert_eq!(status(&shell, cx).as_deref(), Some(FIND_OPENING));

    let reads = shell.backend.reads_seen(ReadKind::Tickets);
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.reload_lists(cx));
    shell.run(cx);
    assert_eq!(shell.backend.reads_seen(ReadKind::Tickets), reads + 1);
    assert_eq!(input_text(&shell, cx), OPEN);
    assert_eq!(query_text(&shell, cx), OPEN);
    assert_eq!(shell.rows("ticket-row", cx), ["Open one"]);
    assert_eq!(status(&shell, cx).as_deref(), Some(FIND_OPENING));

    shell
        .workspace
        .update(cx, |workspace, cx| workspace.load_lists(cx));
    shell.run(cx);
    assert_eq!(input_text(&shell, cx), OPEN);
    assert_eq!(query_text(&shell, cx), OPEN);
    assert_eq!(shell.rows("ticket-row", cx), ["Open one"]);
    assert_eq!(status(&shell, cx).as_deref(), Some(FIND_OPENING));

    let _ = held.send(());
    shell.run(cx);
    assert_eq!(opened(&shell, cx), Some(ticket(&root, OPEN)));
    assert_eq!(input_text(&shell, cx), OPEN);
}

/// Enter brings the pane up to date before it selects (R-32): text set and
/// submitted in one frame, with no render between, still selects the first
/// row of the rows the text lists, not of the rows shown before it.
#[gpui_kit::test]
fn enter_selects_from_the_rows_the_text_lists(cx: &mut TestAppContext) {
    let (shell, _) = shell(cx);
    assert!(shell.rows("ticket-row", cx).len() > 2, "more rows before");
    shell.frame(cx, |window, cx| {
        shell.workspace.update(cx, |workspace, cx| {
            workspace.set_filter_text(SHARED_SLUG, window, cx);
            workspace.submit_filter(window, cx);
        })
    });
    shell.run(cx);
    assert_eq!(shell.rows("ticket-row", cx).len(), 2);
    assert!(list_focused(&shell, cx));
    assert!(shell.selected(("ticket-row", 0usize), cx));
}

/// Every dialog traps focus: with the Abandon dialog open, `secondary-p`
/// does not reach the input behind it.
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
    assert!(!input_focused(&shell, cx));
    assert!(shell.present("confirm-dialog", cx));
}
