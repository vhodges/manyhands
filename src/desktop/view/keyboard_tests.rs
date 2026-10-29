//! The keyboard walk (design, Testing, Proof map: Keyboard): every core
//! action of the Cycle's Scope, by keystrokes alone. Region keys, Tab and
//! shift-Tab, the arrows, Enter and Escape; no click and no call into the
//! workspace. After each step the walk asserts which element holds focus,
//! that it is drawn on screen, and that its region draws its focus ring,
//! the one ring drawn.
//!
//! Each step is a method of `Walk`, run in order by the one test, since
//! each starts where the last left focus.

use std::path::PathBuf;

use gpui_kit::{ElementId, FocusHandle, TestAppContext, test::TestWindowExt};
use manyhands::{
    repository::ItemDtoKind,
    results::{Outcome, ResultCode},
};

use super::{
    find::FIND_MATCHES,
    harness::{Shell, key},
    lists::{SHOWING_DOCUMENTS, SHOWING_TICKETS},
    quit::QuitPhase,
    settings::{PAGES, Shown},
    tickets::{LIFECYCLE_CLOSED, READINESS_FILTERS},
    workspace::{REGIONS, Region},
};
use crate::desktop::{
    model::navigation::OpenTarget,
    services::requests::{Intent, Lane, RequestState},
    test_support::fixtures::{Fixture, item_id},
};

const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM0";
const BLOCKER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM1";
const PARENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM2";
const WAITS: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM3";
const CLOSED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM4";
const FIRST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM5";
const SECOND: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM6";
const STRANDED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM7";
const SLUG: &str = "kb-k9x2b";

/// How many Tab presses a step may take to reach its control.
const TAB_LIMIT: usize = 40;

/// The window over two repositories. `a` holds a document, a ticket
/// waiting on another with a parent, a lifecycle-closed ticket and two
/// tickets sharing a short code. `b` holds a local operation left
/// pending, which the operation area offers to abandon.
struct Walk {
    shell: Shell,
    a: PathBuf,
}

impl Walk {
    fn new(cx: &mut TestAppContext) -> Self {
        let fixture = Fixture::new();
        let a = fixture.create_repository("a");
        let b = fixture.create_repository("b");
        fixture.create_document(&a, DOCUMENT, "docs/guide.md", "Guide");
        fixture.create_ticket(&a, BLOCKER, "Blocker", "open");
        fixture.create_ticket(&a, PARENT, "Parent", "open");
        fixture.create_related_ticket(&a, WAITS, "Waits", "open", &[BLOCKER], Some(PARENT));
        fixture.commit_closed_ticket(&a, CLOSED, "Closed one");
        fixture.commit_duplicate_slug(&a, (FIRST, "First sharer"), (SECOND, "Second sharer"), SLUG);
        fixture.refresh(&a);
        fixture.create_ticket(&b, STRANDED, "Stranded", "open");
        fixture.strand_ticket_save(&b, STRANDED, "Edited");
        Self {
            shell: Shell::open(fixture, cx),
            a,
        }
    }

    /// A walk over a window the caller opened, `a` its first repository.
    fn over(shell: Shell, a: PathBuf) -> Self {
        Self { shell, a }
    }

    /// Waits, with the worker running, until the request `pick` picks has
    /// ended, and gives its outcome and code. The test allows parking.
    async fn ended(
        &self,
        pick: impl Fn(&Intent) -> bool,
        cx: &mut TestAppContext,
    ) -> Option<(Outcome, ResultCode)> {
        let id = *self
            .shell
            .requests(cx, |request| pick(&request.intent))
            .first()
            .expect("the request was made");
        self.shell.until_ended(id, cx).await;
        let operations = self.shell.operations(cx);
        operations.read_with(cx, |operations, _| {
            match &operations.request(id).unwrap().state {
                RequestState::Finished(outcome, code, ..) => Some((*outcome, *code)),
                _ => None,
            }
        })
    }

    /// Waits, with the worker running, until the element `id` is drawn.
    async fn until_drawn(&self, id: impl Into<ElementId>, cx: &mut TestAppContext) {
        let id = id.into();
        let deadline = std::time::Instant::now() + super::harness::WAIT;
        while !self.shell.present(id.clone(), cx) {
            assert!(
                std::time::Instant::now() < deadline,
                "{id:?} was never drawn"
            );
            cx.executor()
                .timer(std::time::Duration::from_millis(10))
                .await;
            self.shell.run(cx);
        }
    }

    /// Tab to the field `id`, in `region`, and type `text` into it.
    fn type_into(&self, id: &'static str, text: &str, region: Region, cx: &mut TestAppContext) {
        self.tab_to("tab", id, cx);
        self.on(id, region, cx);
        self.type_text(text, cx);
    }

    fn keys(&self, keystrokes: &str, cx: &mut TestAppContext) {
        self.shell.keys(keystrokes, cx);
    }

    /// Enter, pressed and released: a focused control is clicked on its
    /// release.
    fn enter(&self, cx: &mut TestAppContext) {
        self.shell.press("enter", cx);
    }

    /// Types `text` into the focused field, as the keyboard does.
    fn type_text(&self, text: &str, cx: &mut TestAppContext) {
        let mut visual = self.shell.visual(cx);
        visual.simulate_input(text);
        visual.run_until_parked();
    }

    /// Exactly `region` holds focus, and it alone draws its ring, which is
    /// visible.
    fn region(&self, region: Region, cx: &mut TestAppContext) {
        self.shell.frame(cx, |window, _| ringed(window, region));
    }

    /// The element `id` holds focus and is drawn on screen, inside
    /// `region`, whose ring shows it.
    fn on(&self, id: impl Into<ElementId>, region: Region, cx: &mut TestAppContext) {
        let id = id.into();
        self.shell.frame(cx, |window, _| {
            ringed(window, region);
            let element = window.find(id.clone());
            assert_eq!(element.focused(), Some(true), "{id:?} holds focus");
            assert!(element.visible(), "{id:?} is on screen");
        });
    }

    /// `focus` holds focus, inside `region`.
    fn on_handle(&self, focus: &FocusHandle, region: Region, cx: &mut TestAppContext) {
        self.region(region, cx);
        assert!(self.shell.frame(cx, |window, _| focus.is_focused(window)));
    }

    fn holds_focus(&self, id: &ElementId, cx: &mut TestAppContext) -> bool {
        let id = id.clone();
        self.shell.frame(cx, |window, _| {
            window.try_find(id).and_then(|element| element.focused()) == Some(true)
        })
    }

    /// Presses `key`, Tab or shift-Tab, until the element `id` holds focus.
    fn tab_to(&self, key: &str, id: impl Into<ElementId>, cx: &mut TestAppContext) {
        let id = id.into();
        self.press_until(key, &id, |walk, cx| walk.holds_focus(&id, cx), cx);
    }

    /// Presses `key` until `reached` holds, at most `TAB_LIMIT` times, or
    /// fails saying `key` never reached `id`.
    fn press_until(
        &self,
        key: &str,
        id: &ElementId,
        reached: impl Fn(&Self, &mut TestAppContext) -> bool,
        cx: &mut TestAppContext,
    ) {
        let mut presses = 0;
        while !reached(self, cx) {
            presses += 1;
            assert!(presses <= TAB_LIMIT, "{key} never reached {id:?}");
            self.keys(key, cx);
        }
    }

    fn opened(&self, cx: &mut TestAppContext) -> Option<OpenTarget> {
        self.shell
            .workspace
            .read_with(cx, |workspace, cx| workspace.opened(cx))
    }

    fn ticket(&self, id: &str) -> OpenTarget {
        OpenTarget::Item {
            repo: key(&self.a),
            kind: ItemDtoKind::Ticket,
            id: item_id(id),
        }
    }

    fn tab_count(&self, cx: &mut TestAppContext) -> usize {
        self.shell
            .workspace
            .read_with(cx, |workspace, _| workspace.tab_keys().len())
    }

    /// The position of the shown tab in the strip.
    fn shown_tab(&self, cx: &mut TestAppContext) -> usize {
        (0..self.tab_count(cx))
            .find(|ix| self.shell.selected(("tab", *ix), cx))
            .expect("a tab is shown")
    }

    fn list_focused(
        &self,
        focus: impl Fn(&super::workspace::Workspace, &gpui_kit::App) -> FocusHandle,
        region: Region,
        cx: &mut TestAppContext,
    ) {
        let handle = self
            .shell
            .workspace
            .read_with(cx, |workspace, cx| focus(workspace, cx));
        self.on_handle(&handle, region, cx);
    }

    /// The repository selector: `secondary-1`, then the arrows and Enter
    /// choose `a`, and focus stays in its list.
    fn select_a_repository(&self, cx: &mut TestAppContext) {
        self.keys("secondary-1", cx);
        self.list_focused(
            |workspace, cx| workspace.repositories_list_focus(cx),
            Region::Repositories,
            cx,
        );
        self.keys("down enter", cx);
        let selected = self.shell.workspace.read_with(cx, |workspace, _| {
            workspace.navigation().selected().cloned()
        });
        assert_eq!(selected, Some(key(&self.a)));
        assert!(self.shell.present(("repo-selected", 0usize), cx));
        self.list_focused(
            |workspace, cx| workspace.repositories_list_focus(cx),
            Region::Repositories,
            cx,
        );
    }

    /// From the navigation region's list, shift-Tab back to the choice
    /// `id` and press it.
    fn choose(&self, id: &'static str, cx: &mut TestAppContext) {
        self.tab_to("shift-tab", id, cx);
        self.on(id, Region::Navigation, cx);
        self.enter(cx);
        assert!(self.shell.selected(id, cx), "{id} is chosen");
    }

    /// The Documents and Tickets panes: `secondary-2`, then shift-Tab to
    /// each pane's choice and Enter. The pane shown says so in text, and
    /// its list takes focus.
    fn switch_panes(&self, cx: &mut TestAppContext) {
        self.keys("secondary-2", cx);
        self.region(Region::Navigation, cx);
        assert_eq!(
            self.shell.label("pane-showing", cx).as_deref(),
            Some(SHOWING_DOCUMENTS)
        );
        self.choose("pane-tickets", cx);
        assert_eq!(
            self.shell.label("pane-showing", cx).as_deref(),
            Some(SHOWING_TICKETS)
        );
        self.list_focused(
            |workspace, cx| workspace.tickets_list_focus(cx),
            Region::Navigation,
            cx,
        );
        self.choose("pane-documents", cx);
        assert_eq!(
            self.shell.label("pane-showing", cx).as_deref(),
            Some(SHOWING_DOCUMENTS)
        );
        self.region(Region::Navigation, cx);
    }

    /// The document tree: the arrows move to the document and Enter opens
    /// it into a tab. Focus stays in the tree.
    fn open_a_document(&self, cx: &mut TestAppContext) {
        self.open_tree_row("Guide", cx);
        assert_eq!(
            self.opened(cx),
            Some(OpenTarget::Item {
                repo: key(&self.a),
                kind: ItemDtoKind::Document,
                id: item_id(DOCUMENT),
            })
        );
        assert_eq!(self.tab_count(cx), 1);
        self.region(Region::Navigation, cx);
    }

    /// In the focused document tree, the arrows move down to the row
    /// `label` and Enter opens it.
    fn open_tree_row(&self, label: &str, cx: &mut TestAppContext) {
        let nodes = self.shell.rows("doc-node", cx);
        let at = nodes
            .iter()
            .position(|node| node == label)
            .expect("the document is listed");
        let mut presses = 0;
        while !self.shell.selected(("doc-node", at), cx) {
            presses += 1;
            assert!(presses <= nodes.len() + 1, "the arrows never reached it");
            self.keys("down", cx);
        }
        self.enter(cx);
    }

    /// The closure and readiness filters, each by shift-Tab and Enter
    /// from the ticket list. Each says what it shows in text; the rows
    /// change with it. Back to Open and any readiness at the end.
    fn change_the_filters(&self, cx: &mut TestAppContext) {
        self.choose("pane-tickets", cx);
        let rows = |cx: &mut TestAppContext| self.shell.rows("ticket-row", cx);
        assert!(!rows(cx).contains(&"Closed one".to_owned()));

        self.choose("filter-closed", cx);
        assert_eq!(
            self.shell.label("tickets-showing", cx).as_deref(),
            Some("Showing closed tickets")
        );
        assert_eq!(rows(cx), ["Closed one"]);
        self.on("filter-closed", Region::Navigation, cx);

        self.tab_to("shift-tab", "filter-open", cx);
        self.enter(cx);
        assert!(self.shell.selected("filter-open", cx));
        assert!(rows(cx).contains(&"Waits".to_owned()));

        self.tab_to("tab", "filter-blocked", cx);
        self.enter(cx);
        let blocked = READINESS_FILTERS
            .iter()
            .find(|(id, ..)| *id == "filter-blocked")
            .map(|(_, _, showing, _)| *showing);
        assert_eq!(
            self.shell.label("tickets-readiness-showing", cx).as_deref(),
            blocked
        );
        assert_eq!(rows(cx), ["Waits"]);
        assert!(self.shell.present("why-blocked", cx));
        self.on("filter-blocked", Region::Navigation, cx);

        self.tab_to("shift-tab", "filter-any-readiness", cx);
        self.enter(cx);
        assert!(self.shell.selected("filter-any-readiness", cx));
        assert!(!self.shell.present("why-blocked", cx));
    }

    /// The ticket list: `secondary-2` focuses it, the arrows move to
    /// Waits and Enter opens it into a second tab.
    fn open_a_ticket(&self, cx: &mut TestAppContext) {
        self.keys("secondary-2", cx);
        self.list_focused(
            |workspace, cx| workspace.tickets_list_focus(cx),
            Region::Navigation,
            cx,
        );
        let at = self
            .shell
            .rows("ticket-row", cx)
            .iter()
            .position(|row| row == "Waits")
            .expect("Waits is listed");
        for _ in 0..=at {
            self.keys("down", cx);
        }
        assert!(self.shell.selected(("ticket-row", at), cx));
        self.enter(cx);
        assert_eq!(self.opened(cx), Some(self.ticket(WAITS)));
        assert_eq!(self.tab_count(cx), 2);
    }

    /// The relationships panel: `secondary-3` focuses the shown tab, Tab
    /// reaches the dependency's row, and Enter opens it into a tab.
    fn follow_a_relationship(&self, cx: &mut TestAppContext) {
        self.keys("secondary-3", cx);
        self.on(("tab", 1usize), Region::Content, cx);
        // E-26: the details sheet, opened by All details, holds the row.
        self.tab_to("tab", "chip-all", cx);
        self.enter(cx);
        self.tab_to("tab", ("rel-dependency", 0usize), cx);
        self.on(("rel-dependency", 0usize), Region::Content, cx);
        self.enter(cx);
        assert_eq!(self.opened(cx), Some(self.ticket(BLOCKER)));
        assert_eq!(self.tab_count(cx), 3);
        self.region(Region::Content, cx);
    }

    /// `secondary-p` focuses Find; its text is selected and typed over.
    fn find(&self, text: &str, cx: &mut TestAppContext) {
        self.keys("secondary-p", cx);
        let input = self
            .shell
            .workspace
            .read_with(cx, |workspace, _| workspace.find_input().clone());
        let focus = input.read_with(cx, gpui_kit::Focusable::focus_handle);
        self.on_handle(&focus, Region::Navigation, cx);
        self.keys("secondary-a backspace", cx);
        self.type_text(text, cx);
        assert_eq!(
            input.read_with(cx, |input, _| input.value().to_string()),
            text
        );
        self.keys("enter", cx);
        self.on_handle(&focus, Region::Navigation, cx);
    }

    /// Find by short code lists both sharers; Tab reaches each Open and
    /// Enter opens the second. Find by ULID opens the closed ticket.
    fn find_by_short_code_and_ulid(&self, cx: &mut TestAppContext) {
        self.find(SLUG, cx);
        assert_eq!(
            self.shell.label("find-state", cx).as_deref(),
            Some(FIND_MATCHES)
        );
        assert_eq!(self.shell.rows("find-match", cx).len(), 2);
        self.keys("tab", cx);
        self.on(("find-match-open", 0usize), Region::Navigation, cx);
        self.keys("tab", cx);
        self.on(("find-match-open", 1usize), Region::Navigation, cx);
        let second = self.shell.label(("find-match-id", 1usize), cx).unwrap();
        self.enter(cx);
        assert_eq!(self.opened(cx), Some(self.ticket(&second)));
        assert_eq!(self.tab_count(cx), 4);

        self.find(CLOSED, cx);
        assert_eq!(self.opened(cx), Some(self.ticket(CLOSED)));
        // E-30: the lifecycle is in the details sheet, opened by All
        // details and closed before going on.
        self.keys("secondary-3", cx);
        self.tab_to("tab", "chip-all", cx);
        self.enter(cx);
        assert_eq!(
            self.shell.label("item-lifecycle", cx).as_deref(),
            Some(LIFECYCLE_CLOSED)
        );
        self.keys("escape", cx);
        assert_eq!(self.tab_count(cx), 5);
    }

    /// `secondary-3`, then `ctrl-tab`, `ctrl-shift-tab` and `secondary-w`:
    /// focus follows the shown tab.
    fn switch_and_close_tabs(&self, cx: &mut TestAppContext) {
        self.keys("secondary-3", cx);
        let last = self.tab_count(cx) - 1;
        assert_eq!(self.shown_tab(cx), last);
        self.on(("tab", last), Region::Content, cx);
        self.keys("ctrl-tab", cx);
        assert_eq!(self.shown_tab(cx), 0);
        self.on(("tab", 0usize), Region::Content, cx);
        self.keys("ctrl-shift-tab", cx);
        assert_eq!(self.shown_tab(cx), last);
        self.on(("tab", last), Region::Content, cx);
        self.keys("secondary-w", cx);
        assert_eq!(self.tab_count(cx), last);
        let shown = self.shown_tab(cx);
        self.on(("tab", shown), Region::Content, cx);
    }

    /// `secondary-r` refreshes the selected repository as a user job,
    /// which succeeds; focus and the tabs stay.
    async fn refresh(&self, cx: &mut TestAppContext) {
        let tabs = self.tab_count(cx);
        let shown = self.shown_tab(cx);
        self.keys("secondary-r", cx);
        let refresh = self.shell.requests(cx, |request| {
            request.lane == Lane::User && matches!(request.intent, Intent::IndexRefresh { .. })
        })[0];
        self.shell.until_ended(refresh, cx).await;
        let operations = self.shell.operations(cx);
        let finished = operations.read_with(cx, |operations, _| {
            match &operations.request(refresh).unwrap().state {
                RequestState::Finished(outcome, code, ..) => Some((*outcome, *code)),
                _ => None,
            }
        });
        assert_eq!(finished, Some((Outcome::Success, ResultCode::Ok)));
        assert_eq!(self.tab_count(cx), tabs);
        self.on(("tab", shown), Region::Content, cx);
    }

    fn settings_shown(&self, cx: &mut TestAppContext) -> Shown {
        self.shell
            .workspace
            .read_with(cx, |workspace, cx| workspace.settings_shown(cx))
    }

    /// `secondary-,` shows the settings with focus on the first page's
    /// entry; Tab and Enter show each other page. Back to tabs, by Tab
    /// and Enter, shows the tabs and focuses the shown one.
    fn open_settings_and_each_page(&self, cx: &mut TestAppContext) {
        self.keys("secondary-,", cx);
        assert_eq!(self.settings_shown(cx), Shown::Settings(PAGES[0]));
        self.on(("settings-page", 0usize), Region::Content, cx);
        for (ix, page) in PAGES.iter().copied().enumerate().skip(1) {
            self.tab_to("tab", ("settings-page", ix), cx);
            self.enter(cx);
            assert_eq!(self.settings_shown(cx), Shown::Settings(page), "{page:?}");
            self.on(("settings-page", ix), Region::Content, cx);
        }
        self.tab_to("shift-tab", "settings-close", cx);
        self.on("settings-close", Region::Content, cx);
        self.enter(cx);
        assert_eq!(self.settings_shown(cx), Shown::Tabs);
        let shown = self.shown_tab(cx);
        self.on(("tab", shown), Region::Content, cx);
    }

    /// `secondary-4` focuses the operation area, which lists this
    /// session's refresh and `b`'s pending operation.
    fn open_the_operation_area(&self, cx: &mut TestAppContext) {
        self.keys("secondary-4", cx);
        self.region(Region::Operations, cx);
        assert!(self.shell.present(("pending-row", 0usize), cx));
        assert!(!self.shell.requests(cx, |_| true).is_empty());
    }

    /// From the operation area, Tab reaches the pending operation's
    /// Abandon and Enter opens its confirmation dialog, which holds focus.
    /// Escape closes it with nothing asked, and focus is back on Abandon.
    fn start_a_confirmed_action_and_escape(&self, cx: &mut TestAppContext) {
        let requests = self.shell.requests(cx, |_| true).len();
        self.tab_to("tab", ("pending-abandon", 0usize), cx);
        self.on(("pending-abandon", 0usize), Region::Operations, cx);
        self.enter(cx);
        assert!(self.shell.present("confirm-dialog", cx));
        self.shell.frame(cx, |window, _| {
            for region in REGIONS {
                assert_ne!(
                    window.find(region.id()).focused(),
                    Some(true),
                    "the dialog holds focus, not {region:?}"
                );
            }
        });
        self.keys("escape", cx);
        assert!(!self.shell.present("confirm-dialog", cx));
        assert_eq!(self.shell.requests(cx, |_| true).len(), requests);
        self.on(("pending-abandon", 0usize), Region::Operations, cx);
    }

    /// `secondary-q` with no job running quits.
    fn quit(&self, cx: &mut TestAppContext) {
        self.keys("secondary-q", cx);
        let phase = self
            .shell
            .workspace
            .read_with(cx, |workspace, _| workspace.quit_phase());
        assert_eq!(phase, QuitPhase::Quit);
    }
}

/// Every core action of D1, by keyboard alone, with focus asserted after
/// each.
#[gpui_kit::test]
async fn every_core_action_works_by_keyboard_alone(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let walk = Walk::new(cx);
    walk.region(Region::Repositories, cx);

    walk.select_a_repository(cx);
    walk.switch_panes(cx);
    walk.open_a_document(cx);
    walk.change_the_filters(cx);
    walk.open_a_ticket(cx);
    walk.follow_a_relationship(cx);
    walk.find_by_short_code_and_ulid(cx);
    walk.switch_and_close_tabs(cx);
    walk.refresh(cx).await;
    walk.open_settings_and_each_page(cx);
    walk.open_the_operation_area(cx);
    walk.start_a_confirmed_action_and_escape(cx);
    walk.quit(cx);
}

/// In the frame just drawn, exactly `region` holds focus, and it alone
/// draws its ring, which is visible.
fn ringed(window: &gpui_kit::Window, region: Region) {
    for each in REGIONS {
        let focused = window.find(each.id()).focused() == Some(true);
        assert_eq!(focused, each == region, "{each:?} holds focus");
        let ring = window.try_find(each.ring_id());
        assert_eq!(ring.is_some(), each == region, "{each:?} draws a ring");
    }
    assert!(
        window.find(region.ring_id()).visible(),
        "the ring is visible"
    );
}

mod actions;
mod confirm;
mod drafts;
mod editing;
mod frame;
mod passphrase;
mod settings;
