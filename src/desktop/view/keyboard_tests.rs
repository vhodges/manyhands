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
    harness::{Shell, key},
    quit::QuitPhase,
    settings::{PAGES, Shown},
    tickets::LIFECYCLE_CLOSED,
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
        self.shell.frame(cx, |window, cx| {
            ringed(window, self.held_region(window, cx), region)
        });
    }

    /// The region the Workspace says holds focus: a region's own element,
    /// or the toolbar button of its panel in the header (E-14).
    fn held_region(&self, window: &gpui_kit::Window, cx: &gpui_kit::App) -> Option<Region> {
        self.shell.workspace.read(cx).held_region(window, cx)
    }

    /// The element `id` holds focus and is drawn on screen, inside
    /// `region`, whose ring shows it.
    fn on(&self, id: impl Into<ElementId>, region: Region, cx: &mut TestAppContext) {
        let id = id.into();
        self.shell.frame(cx, |window, cx| {
            ringed(window, self.held_region(window, cx), region);
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
        let mut presses = 0;
        while !self.holds_focus(&id, cx) {
            presses += 1;
            assert!(presses <= TAB_LIMIT, "{key} never reached {id:?}");
            self.keys(key, cx);
        }
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

    /// From the Tickets list, shift-Tab back to the choice `id` and press it.
    fn choose(&self, id: &'static str, cx: &mut TestAppContext) {
        self.tab_to("shift-tab", id, cx);
        self.on(id, Region::Tickets, cx);
        self.enter(cx);
        assert!(self.shell.selected(id, cx), "{id} is chosen");
    }

    /// The Documents and Tickets lists: `secondary-2` focuses the list used
    /// last, Documents at first; F6 reaches Tickets through Content, which
    /// makes it the list used last, and `secondary-2` follows it, then
    /// follows Documents back.
    fn switch_panes(&self, cx: &mut TestAppContext) {
        self.keys("secondary-2", cx);
        self.region(Region::Documents, cx);
        self.keys("f6 f6", cx);
        self.list_focused(
            |workspace, cx| workspace.tickets_list_focus(cx),
            Region::Tickets,
            cx,
        );
        self.keys("secondary-1", cx);
        self.keys("secondary-2", cx);
        self.list_focused(
            |workspace, cx| workspace.tickets_list_focus(cx),
            Region::Tickets,
            cx,
        );
        self.keys("shift-f6 shift-f6", cx);
        self.region(Region::Documents, cx);
        self.keys("secondary-1", cx);
        self.keys("secondary-2", cx);
        self.region(Region::Documents, cx);
    }

    /// The document tree: the arrows move to the document and Enter opens
    /// it into a tab. Focus stays in the tree.
    fn open_a_document(&self, cx: &mut TestAppContext) {
        let nodes = self.shell.rows("doc-node", cx);
        let at = nodes
            .iter()
            .position(|node| node == "Guide")
            .expect("the document is listed");
        let mut presses = 0;
        while !self.shell.selected(("doc-node", at), cx) {
            presses += 1;
            assert!(presses <= nodes.len() + 1, "the arrows never reached it");
            self.keys("down", cx);
        }
        self.enter(cx);
        assert_eq!(
            self.opened(cx),
            Some(OpenTarget::Item {
                repo: key(&self.a),
                kind: ItemDtoKind::Document,
                id: item_id(DOCUMENT),
            })
        );
        assert_eq!(self.tab_count(cx), 1);
        self.region(Region::Documents, cx);
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

    /// The closure segments, each by shift-Tab and Enter from the ticket
    /// list, and the readiness menu: Tab to its button, Enter to open it,
    /// Down to an item and Enter to choose it. The rows change with each
    /// choice. Back to Open and any readiness at the end.
    fn change_the_filters(&self, cx: &mut TestAppContext) {
        // F6 from the document tree, through Content, to the ticket list.
        self.keys("f6 f6", cx);
        self.list_focused(
            |workspace, cx| workspace.tickets_list_focus(cx),
            Region::Tickets,
            cx,
        );
        let rows = |cx: &mut TestAppContext| self.shell.rows("ticket-row", cx);
        assert!(!rows(cx).contains(&"Closed one".to_owned()));

        self.choose("filter-closed", cx);
        assert_eq!(rows(cx), ["Closed one"]);
        self.on("filter-closed", Region::Tickets, cx);

        self.tab_to("shift-tab", "filter-open", cx);
        self.enter(cx);
        assert!(self.shell.selected("filter-open", cx));
        assert!(rows(cx).contains(&"Waits".to_owned()));

        // The toolbar is first in the Tab order (Part H, E-6): Shift-Tab
        // from the first segment reaches New ticket…, in the header, which
        // counts as the Tickets region (E-14).
        self.tab_to("shift-tab", "new-ticket-button", cx);
        self.on("new-ticket-button", Region::Tickets, cx);

        // The menu: Enter opens it, three Downs reach Blocked, and Enter
        // chooses it. Focus is back on the button, which names the choice.
        self.tab_to("tab", "filter-readiness", cx);
        self.on("filter-readiness", Region::Tickets, cx);
        self.keys("enter down down down enter", cx);
        assert_eq!(
            self.shell.label("filter-readiness", cx).as_deref(),
            Some("Readiness: blocked")
        );
        assert_eq!(rows(cx), ["Waits"]);
        assert!(self.shell.present("why-blocked", cx));
        self.on("filter-readiness", Region::Tickets, cx);

        self.keys("enter down enter", cx);
        assert_eq!(
            self.shell.label("filter-readiness", cx).as_deref(),
            Some("Readiness: any")
        );
        assert!(!self.shell.present("why-blocked", cx));
        self.on("filter-readiness", Region::Tickets, cx);
    }

    /// The ticket list: `secondary-2` focuses it, the arrows move to
    /// Waits and Enter opens it into a second tab.
    fn open_a_ticket(&self, cx: &mut TestAppContext) {
        self.keys("secondary-2", cx);
        self.list_focused(
            |workspace, cx| workspace.tickets_list_focus(cx),
            Region::Tickets,
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

    /// `secondary-p` focuses the filter input; its text is selected and
    /// typed over. Enter is the caller's.
    fn filter(&self, text: &str, cx: &mut TestAppContext) {
        self.keys("secondary-p", cx);
        let input = self
            .shell
            .workspace
            .read_with(cx, |workspace, _| workspace.find_input().clone());
        let focus = input.read_with(cx, gpui_kit::Focusable::focus_handle);
        self.on_handle(&focus, Region::Tickets, cx);
        self.keys("secondary-a backspace", cx);
        self.type_text(text, cx);
        assert_eq!(
            input.read_with(cx, |input, _| input.value().to_string()),
            text
        );
    }

    /// Opens the item whose ID is `id` through the filter input: Enter
    /// opens it, and focus stays in the input.
    fn find(&self, id: &str, cx: &mut TestAppContext) {
        self.filter(id, cx);
        self.keys("enter", cx);
        let input = self
            .shell
            .workspace
            .read_with(cx, |workspace, _| workspace.find_input().clone());
        let focus = input.read_with(cx, gpui_kit::Focusable::focus_handle);
        self.on_handle(&focus, Region::Tickets, cx);
    }

    /// Enter on a short code two tickets share opens neither: focus moves
    /// to the list with the first row selected, and Enter there opens that
    /// row. Enter on a ULID opens the closed ticket and focus stays in the
    /// input.
    fn find_by_short_code_and_ulid(&self, cx: &mut TestAppContext) {
        self.filter(SLUG, cx);
        self.keys("enter", cx);
        assert_eq!(self.tab_count(cx), 3, "no tab opened");
        self.list_focused(
            |workspace, cx| workspace.tickets_list_focus(cx),
            Region::Tickets,
            cx,
        );
        assert_eq!(self.shell.rows("ticket-row", cx).len(), 2);
        assert!(self.shell.selected(("ticket-row", 0usize), cx));
        let first = match self.shell.label(("ticket-row", 0usize), cx).as_deref() {
            Some("First sharer") => FIRST,
            Some("Second sharer") => SECOND,
            other => panic!("a sharer is listed first, not {other:?}"),
        };
        self.enter(cx);
        assert_eq!(self.opened(cx), Some(self.ticket(first)));
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

    /// `secondary-4`, the arrows to the pending row of `a` whose command is
    /// `action`, `shift-f10`, the arrows to Review changes and Enter: the
    /// conflict sheet opens. Waits for the row to be listed and drawn.
    /// Gives its key.
    async fn to_pending_review(
        &self,
        action: &str,
        cx: &mut TestAppContext,
    ) -> crate::desktop::model::activity::rows::RowKey {
        let row =
            super::activity_support::until_pending(&self.shell, &key(&self.a), action, cx).await;
        super::activity_support::to_row(&self.shell, &row, cx);
        self.choose_review(cx);
        row
    }

    /// With the list focused on a pending row, `shift-f10`, the arrows to
    /// Review changes and Enter.
    fn choose_review(&self, cx: &mut TestAppContext) {
        self.keys("shift-f10", cx);
        super::repositories::tests::support::highlight(
            &mut self.shell.visual(cx),
            super::conflict::REVIEW_CHANGES,
        );
        self.keys("enter", cx);
    }

    /// The pending operation's row, which `b` has stranded.
    fn pending_row(
        &self,
        cx: &mut TestAppContext,
    ) -> crate::desktop::model::activity::rows::RowKey {
        use super::activity_support::{refresh, rows};
        use crate::desktop::model::activity::rows::RowKey;
        refresh(&self.shell, cx);
        let pending: Vec<RowKey> = rows(&self.shell, cx)
            .into_iter()
            .map(|row| row.row.key)
            .filter(|key| matches!(key, RowKey::Pending { .. }))
            .collect();
        assert_eq!(pending.len(), 1, "one pending row: {pending:?}");
        pending[0].clone()
    }

    /// `secondary-4` focuses the operation area, which lists this
    /// session's refresh and `b`'s pending operation.
    fn open_the_operation_area(&self, cx: &mut TestAppContext) {
        self.keys("secondary-4", cx);
        self.region(Region::Operations, cx);
        self.pending_row(cx);
        assert!(!self.shell.requests(cx, |_| true).is_empty());
    }

    /// From the operation area, the arrows reach the pending operation's
    /// row, and its menu's Abandon (`shift-f10`, the arrows and Enter) opens
    /// its confirmation dialog, which holds focus. Escape closes it with
    /// nothing asked, and focus is back on the list, the row highlighted.
    fn start_a_confirmed_action_and_escape(&self, cx: &mut TestAppContext) {
        let requests = self.shell.requests(cx, |_| true).len();
        let row = self.pending_row(cx);
        super::activity_support::to_row(&self.shell, &row, cx);
        self.on_handle(
            &super::activity_support::list_focus(&self.shell, cx),
            Region::Operations,
            cx,
        );
        self.keys("shift-f10", cx);
        super::repositories::tests::support::highlight(
            &mut self.shell.visual(cx),
            super::operations::ABANDON,
        );
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
        self.on_handle(
            &super::activity_support::list_focus(&self.shell, cx),
            Region::Operations,
            cx,
        );
        assert_eq!(
            super::activity_support::highlighted(&self.shell, cx),
            Some(row),
            "its row is highlighted"
        );
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
/// draws its ring, which is visible. A region holds focus when its element
/// does, or when `held`, the Workspace's own answer, names it: the focus of
/// a panel's toolbar button in the header counts as the panel's (E-14,
/// decision 18).
fn ringed(window: &gpui_kit::Window, held: Option<Region>, region: Region) {
    for each in REGIONS {
        let focused = window.find(each.id()).focused() == Some(true) || held == Some(each);
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
mod conflict;
mod discussion;
mod drafts;
mod editing;
mod frame;
mod lifecycle;
mod passphrase;
mod settings;
mod shell;
