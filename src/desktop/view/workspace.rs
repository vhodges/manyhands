//! The main window's view: the shell of four focusable regions, the
//! repository selector, and quitting.
//!
//! The regions are focus groups: the repository list, the documents and
//! tickets navigation, the content and the operation area. `F6` and
//! `shift-F6` cycle them; `secondary-1` to `secondary-4` focus each. The
//! focused region draws a focus ring, and is the only one that does.
//!
//! The Quit action and the window's close request go through one
//! handler. With no job running it quits. With one running it refuses,
//! stops the worker admitting jobs, shows "Finishing" and Quit now, and
//! quits when the job returns. Quit now quits at once: the worker thread
//! is not waited for, and the library's journals recover what it was
//! doing.

use std::path::PathBuf;

use gpui_kit::{
    App, AppContext, ClickEvent, Context, Entity, FocusHandle, InteractiveElement, IntoElement,
    ParentElement, Render, Role, StatefulInteractiveElement, Styled, Subscription, Task,
    TestSupportExt, Window,
    component::{
        ActiveTheme, Root, Selectable,
        button::{Button, ButtonVariants},
        h_flex,
        list::ListEvent,
        v_flex,
    },
    div, px,
};
use manyhands::repository::{ClosureFilter, ResolvedRepository};

use super::{
    documents::DocumentsPane,
    item::ItemPane,
    sidebar,
    tickets::{CLOSURE_FILTERS, TicketsPane},
};
use crate::desktop::{
    keys::{
        FocusContent, FocusNavigation, FocusOperations, FocusRepositories, NextRegion,
        OpenSelected, PreviousRegion, Quit, QuitNow,
    },
    model::{
        documents::Documents,
        list_state::{ListState, Rows},
        navigation::{Navigation, OpenTarget, Pane, Resolution},
        stamp::{RepoKey, Stamp, TabId, ViewScope},
        tickets::Tickets,
    },
    services::{ReadKind, Services, requests::DesktopRequestId, worker::QuitState},
};

/// Where quitting stands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum QuitPhase {
    Running,
    /// Asked to quit; waiting for this job to return.
    Finishing(DesktopRequestId),
    /// Quitting.
    Quit,
}

/// A focus group of the shell, in `F6` order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Region {
    Repositories,
    Navigation,
    Content,
    Operations,
}

pub(crate) const REGIONS: [Region; 4] = [
    Region::Repositories,
    Region::Navigation,
    Region::Content,
    Region::Operations,
];

impl Region {
    fn index(self) -> usize {
        REGIONS
            .iter()
            .position(|region| *region == self)
            .expect("every region is listed")
    }

    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Repositories => "region-repositories",
            Self::Navigation => "region-navigation",
            Self::Content => "region-content",
            Self::Operations => "region-operations",
        }
    }

    /// The focus ring drawn while the region holds focus.
    pub(crate) fn ring_id(self) -> &'static str {
        match self {
            Self::Repositories => "region-repositories-focus-ring",
            Self::Navigation => "region-navigation-focus-ring",
            Self::Content => "region-content-focus-ring",
            Self::Operations => "region-operations-focus-ring",
        }
    }

    fn key_context(self) -> &'static str {
        match self {
            Self::Repositories => "Repositories",
            Self::Navigation => "Navigation",
            Self::Content => "Content",
            Self::Operations => "Operations",
        }
    }
}

pub(crate) struct Workspace {
    services: Services,
    /// One handle per region, in `REGIONS` order.
    regions: [FocusHandle; 4],
    navigation: Navigation,
    /// The repository list read in flight; replacing it drops the last.
    list_read: Option<Task<()>>,
    /// The selection's resolution in flight; replacing it drops the last.
    resolve_read: Option<Task<()>>,
    documents: Documents,
    documents_pane: DocumentsPane,
    /// The folder and document reads, in parallel under one stamp.
    folder_read: Option<Task<()>>,
    document_read: Option<Task<()>>,
    tickets: Tickets,
    tickets_pane: TicketsPane,
    ticket_read: Option<Task<()>>,
    /// The ticket rows changed and go to the Kit's list at the next frame,
    /// which has the window its selection needs.
    tickets_changed: bool,
    /// The item the last activated row opened, in the content region (Task 8
    /// makes it a tab).
    item: Option<Entity<ItemPane>>,
    quit: QuitPhase,
    _operations: Subscription,
    _ticket_events: Subscription,
}

impl Workspace {
    pub(crate) fn new(services: Services, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let regions = [
            cx.focus_handle(),
            cx.focus_handle(),
            cx.focus_handle(),
            cx.focus_handle(),
        ];
        window.focus(&regions[Region::Repositories.index()], cx);
        let operations = cx.observe(services.operations(), |this, _, cx| {
            this.operations_changed(cx)
        });
        let workspace = cx.entity().downgrade();
        window.on_window_should_close(cx, move |_, cx| {
            workspace
                .update(cx, |workspace, cx| workspace.request_quit(cx))
                .unwrap_or(true)
        });
        let tickets_pane = TicketsPane::new(window, cx);
        let ticket_events = cx.subscribe(tickets_pane.list(), |this, list, event, cx| {
            // A click or Enter on a row.
            if let ListEvent::Confirm(ix) = event {
                let target = this.tickets.repo().and_then(|repo| {
                    list.read(cx)
                        .delegate()
                        .row(*ix)
                        .map(|row| row.target(repo))
                });
                if let Some(target) = target {
                    this.open_item(target, cx);
                }
            }
        });
        let mut workspace = Self {
            services,
            regions,
            navigation: Navigation::default(),
            list_read: None,
            resolve_read: None,
            documents: Documents::default(),
            documents_pane: DocumentsPane::new(cx),
            folder_read: None,
            document_read: None,
            tickets: Tickets::default(),
            tickets_pane,
            ticket_read: None,
            tickets_changed: false,
            item: None,
            quit: QuitPhase::Running,
            _operations: operations,
            _ticket_events: ticket_events,
        };
        workspace.load_repositories(cx);
        workspace
    }

    #[cfg(test)]
    pub(crate) fn services(&self) -> &Services {
        &self.services
    }

    #[cfg(test)]
    pub(crate) fn navigation(&self) -> &Navigation {
        &self.navigation
    }

    #[cfg(test)]
    pub(crate) fn quit_phase(&self) -> QuitPhase {
        self.quit
    }

    #[cfg(test)]
    pub(crate) fn documents(&self) -> &Documents {
        &self.documents
    }

    /// What the last activated row opened.
    #[cfg(test)]
    pub(crate) fn opened(&self, cx: &App) -> Option<OpenTarget> {
        self.item
            .as_ref()
            .map(|item| item.read(cx).target().clone())
    }

    /// Reads the repository list again.
    #[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
    pub(crate) fn load_repositories(&mut self, cx: &mut Context<Self>) {
        let stamp = self.navigation.load_repositories();
        self.list_read = Some(self.services.read(
            ReadKind::Repositories,
            stamp,
            |service| service.list_repositories(),
            cx,
            |this: &mut Self, stamp, result, cx| {
                if this.navigation.apply_repositories(stamp, result) {
                    // A selection the list no longer gives has no lists.
                    if this.navigation.selected().is_none() {
                        this.clear_lists(cx);
                    }
                    cx.notify();
                }
            },
        ));
        cx.notify();
    }

    /// Selects the repository `key` and resolves it. The list is read
    /// again too, so the configuration and index shown are the current
    /// ones (track rule 4); the new list keeps the selection.
    pub(crate) fn select_repository(&mut self, key: String, cx: &mut Context<Self>) {
        let stamp = self.navigation.select(key.clone());
        self.clear_lists(cx);
        self.resolve(key, stamp, cx);
        self.load_repositories(cx);
    }

    /// Resolves `key` again: Retry on an unavailable or failed entry. As
    /// on selection, the list is read again.
    pub(crate) fn retry_resolution(&mut self, key: String, cx: &mut Context<Self>) {
        let stamp = self.navigation.retry_resolution(key.clone());
        self.clear_lists(cx);
        self.resolve(key, stamp, cx);
        self.load_repositories(cx);
    }

    #[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
    fn resolve(&mut self, key: String, stamp: Stamp, cx: &mut Context<Self>) {
        let root = PathBuf::from(key);
        self.resolve_read = Some(self.services.read(
            ReadKind::Resolve,
            stamp,
            move |service| service.resolve_repository(&root),
            cx,
            |this: &mut Self, stamp, result, cx| {
                if this.navigation.apply_resolution(stamp, result) {
                    this.load_lists(cx);
                    cx.notify();
                }
            },
        ));
        cx.notify();
    }

    /// The selected repository, when it resolved.
    fn resolved_selection(&self) -> Option<(RepoKey, ResolvedRepository)> {
        let entry = self.navigation.selected_entry()?;
        match &entry.resolution {
            Resolution::Resolved(resolved) => Some((entry.key().clone(), resolved.clone())),
            _ => None,
        }
    }

    /// Loads the resolved selection's document tree and ticket list; with
    /// none, shows neither. Also Retry on either.
    pub(crate) fn load_lists(&mut self, cx: &mut Context<Self>) {
        self.clear_lists(cx);
        let Some((key, resolved)) = self.resolved_selection() else {
            return;
        };
        self.load_documents(key.clone(), resolved.clone(), cx);
        self.load_tickets(key, resolved, cx);
    }

    /// No repository's lists are shown; reads still running are dropped.
    fn clear_lists(&mut self, cx: &mut Context<Self>) {
        self.documents.clear();
        self.documents_pane.clear(cx);
        self.folder_read = None;
        self.document_read = None;
        self.tickets.clear();
        self.ticket_read = None;
        self.tickets_changed = true;
        cx.notify();
    }

    /// The folder and document reads run in parallel under one stamp.
    #[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
    fn load_documents(
        &mut self,
        key: RepoKey,
        resolved: ResolvedRepository,
        cx: &mut Context<Self>,
    ) {
        let stamp = self.documents.load(key);
        self.documents_pane.clear(cx);
        let repo = resolved.clone();
        self.folder_read = Some(self.services.read(
            ReadKind::Folders,
            stamp.clone(),
            move |service| service.list_document_folders(&repo),
            cx,
            |this: &mut Self, stamp, result, cx| {
                if this.documents.apply_folders(stamp, result) {
                    this.documents_changed(cx);
                }
            },
        ));
        self.document_read = Some(self.services.read(
            ReadKind::Documents,
            stamp,
            move |service| service.list_documents(&resolved),
            cx,
            |this: &mut Self, stamp, result, cx| {
                if this.documents.apply_documents(stamp, result) {
                    this.documents_changed(cx);
                }
            },
        ));
        cx.notify();
    }

    fn documents_changed(&mut self, cx: &mut Context<Self>) {
        match (self.documents.repo(), self.documents.state()) {
            (Some(repo), ListState::Ready { rows, .. }) => self.documents_pane.set(repo, rows, cx),
            _ => self.documents_pane.clear(cx),
        }
        cx.notify();
    }

    #[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
    fn load_tickets(&mut self, key: RepoKey, resolved: ResolvedRepository, cx: &mut Context<Self>) {
        let stamp = self.tickets.load(key);
        self.read_tickets(stamp, resolved, cx);
    }

    #[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
    fn read_tickets(&mut self, stamp: Stamp, resolved: ResolvedRepository, cx: &mut Context<Self>) {
        let filter = self.tickets.query().to_filter();
        self.tickets_changed = true;
        self.ticket_read = Some(self.services.read(
            ReadKind::Tickets,
            stamp,
            move |service| service.list_tickets(&resolved, &filter),
            cx,
            |this: &mut Self, stamp, result, cx| {
                if this.tickets.apply(stamp, result) {
                    this.tickets_changed = true;
                    cx.notify();
                }
            },
        ));
        cx.notify();
    }

    /// Lists `closure` from now on. A read under the last filter is
    /// dropped when it completes.
    pub(crate) fn set_closure(&mut self, closure: ClosureFilter, cx: &mut Context<Self>) {
        let stamp = self.tickets.set_closure(closure);
        match (stamp, self.resolved_selection()) {
            (Some(stamp), Some((_, resolved))) => self.read_tickets(stamp, resolved, cx),
            _ => cx.notify(),
        }
    }

    /// Opens `target` in the content region, in place of what was open,
    /// and reads it. The single content item is stamped as tab 0 until
    /// Task 8 gives each tab its own.
    pub(crate) fn open_item(&mut self, target: OpenTarget, cx: &mut Context<Self>) {
        let services = self.services.clone();
        self.item =
            Some(cx.new(|cx| ItemPane::new(target, ViewScope::Tab(CONTENT_TAB), services, cx)));
        cx.notify();
    }

    /// Enter in the document tree: opens the selected document.
    fn open_selected_document(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.documents_pane.selected_target(cx) {
            self.open_item(target, cx);
        }
    }

    /// Shows `pane`. While the navigation region holds focus, the pane's
    /// tree or list takes it.
    fn show_pane(&mut self, pane: Pane, window: &mut Window, cx: &mut Context<Self>) {
        self.navigation.set_pane(pane);
        if self.focused_region(window, cx) == Some(Region::Navigation) {
            self.focus_region(Region::Navigation, window, cx);
        }
        cx.notify();
    }

    fn focused_region(&self, window: &Window, cx: &App) -> Option<Region> {
        REGIONS
            .into_iter()
            .find(|region| self.regions[region.index()].contains_focused(window, cx))
    }

    /// Focuses `region`. The navigation region gives its focus to the
    /// shown pane's tree or list, whose arrow keys then work, when a
    /// repository's lists are shown.
    fn focus_region(&mut self, region: Region, window: &mut Window, cx: &mut Context<Self>) {
        match (region, self.navigation.pane()) {
            (Region::Navigation, Pane::Documents) if self.documents.repo().is_some() => {
                self.documents_pane.focus(window, cx)
            }
            (Region::Navigation, Pane::Tickets) if self.tickets.repo().is_some() => {
                self.tickets_pane.focus(window, cx)
            }
            _ => window.focus(&self.regions[region.index()], cx),
        }
        cx.notify();
    }

    /// Focuses the region `step` places after the focused one, wrapping;
    /// the first region when none holds focus.
    fn cycle_region(&mut self, step: isize, window: &mut Window, cx: &mut Context<Self>) {
        let next = match self.focused_region(window, cx) {
            Some(region) => {
                let count = REGIONS.len() as isize;
                REGIONS[(region.index() as isize + step).rem_euclid(count) as usize]
            }
            None => Region::Repositories,
        };
        self.focus_region(next, window, cx);
    }

    /// The one quit handler, for the action and the close request. True
    /// when quitting goes ahead now.
    fn request_quit(&mut self, cx: &mut Context<Self>) -> bool {
        match self.quit {
            QuitPhase::Quit => return true,
            QuitPhase::Finishing(_) => return false,
            QuitPhase::Running => {}
        }
        match self.services.worker().quit() {
            QuitState::Idle => {
                self.quit_now(cx);
                true
            }
            QuitState::Finishing(id) => {
                self.quit = QuitPhase::Finishing(id);
                cx.notify();
                false
            }
        }
    }

    fn quit_now(&mut self, cx: &mut Context<Self>) {
        self.quit = QuitPhase::Quit;
        cx.notify();
        cx.quit();
    }

    /// A worker event arrived: if quitting waits for a job that has now
    /// returned, quit.
    fn operations_changed(&mut self, cx: &mut Context<Self>) {
        if matches!(self.quit, QuitPhase::Finishing(_))
            && self.services.worker().quit() == QuitState::Idle
        {
            self.quit_now(cx);
        }
    }

    /// "Finishing" and the command quitting waits for. Fixed text only.
    pub(crate) fn finishing_text(&self, cx: &App) -> Option<String> {
        let QuitPhase::Finishing(id) = self.quit else {
            return None;
        };
        let operations = self.services.operations().read(cx);
        Some(match operations.request(id) {
            Some(request) => format!("Finishing {}", request.intent.label()),
            None => "Finishing".to_owned(),
        })
    }

    /// A region: its focus handle, key context and, while it holds focus,
    /// its ring, drawn behind its content.
    fn region<C: IntoElement>(
        &self,
        region: Region,
        window: &Window,
        cx: &App,
        content: C,
    ) -> impl IntoElement + Styled + use<C> {
        let focused = self.focused_region(window, cx) == Some(region);
        let theme = cx.theme();
        let ring = focused.then(|| {
            div()
                .id(region.ring_id())
                .test_support()
                .absolute()
                .inset_0()
                .border_2()
                .border_color(theme.ring)
        });
        div()
            .id(region.id())
            .test_support()
            .track_focus(&self.regions[region.index()])
            .key_context(region.key_context())
            .relative()
            .p_2()
            .border_1()
            .border_color(theme.border)
            .children(ring)
            .child(content)
    }
}

/// The tab the single content item is stamped as, until Task 8 opens tabs.
const CONTENT_TAB: TabId = TabId(0);

pub(crate) const PANES: [(&str, &str, Pane); 2] = [
    ("pane-documents", "Documents", Pane::Documents),
    ("pane-tickets", "Tickets", Pane::Tickets),
];

/// One of a row of choices: its element says whether it is chosen, and
/// so does its text, not color alone.
fn choice(
    id: &'static str,
    label: &'static str,
    chosen: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .test_support()
        .role(Role::Tab)
        .aria_label(label)
        .aria_selected(chosen)
        .child(
            Button::new((id, 0usize))
                .label(label)
                .selected(chosen)
                .on_click(on_click),
        )
}

/// A list's state in text, when it is not simply ready, and its failure
/// with Retry.
fn list_status<T: Rows>(
    name: &'static str,
    list: &ListState<T>,
    cx: &mut Context<Workspace>,
) -> impl IntoElement + use<T> {
    let muted = cx.theme().muted_foreground;
    let state = match list {
        ListState::Ready {
            rows,
            complete,
            index,
        } if !rows.is_empty()
            && *complete
            && index
                .as_ref()
                .is_none_or(|index| index.state == manyhands::repository::IndexState::Current) =>
        {
            None
        }
        _ => Some(
            div()
                .id((name, 0usize))
                .test_support()
                .aria_label(list.label())
                .text_sm()
                .text_color(muted)
                .child(list.label()),
        ),
    };
    let failure =
        match list {
            ListState::Busy | ListState::Failed(_) => Some(
                v_flex()
                    .gap_1()
                    .children(match list {
                        ListState::Failed(failure) => Some(
                            div()
                                .id((name, 1usize))
                                .test_support()
                                .aria_label(failure.code.message())
                                .text_sm()
                                .child(failure.code.message()),
                        ),
                        _ => None,
                    })
                    .child(div().id((name, 2usize)).test_support().child(
                        Button::new((name, 3usize)).label(sidebar::RETRY).on_click(
                            cx.listener(|this, _: &ClickEvent, _, cx| this.load_lists(cx)),
                        ),
                    )),
            ),
            _ => None,
        };
    v_flex().gap_1().children(state).children(failure)
}

impl Workspace {
    /// The navigation region: the pane choice and the shown pane.
    fn navigation_pane(&mut self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let pane = self.navigation.pane();
        let choices = h_flex().gap_1().children(PANES.map(|(id, label, shown)| {
            let workspace = cx.entity().downgrade();
            choice(id, label, pane == shown, move |_, window, cx| {
                let _ = workspace.update(cx, |this, cx| this.show_pane(shown, window, cx));
            })
        }));
        let body = match pane {
            Pane::Documents => match self.documents.repo() {
                None => v_flex().child(div().text_sm().child(sidebar::NOTHING_SELECTED)),
                Some(_) => v_flex()
                    .gap_1()
                    .flex_1()
                    .min_h_0()
                    .child(list_status("documents-state", self.documents.state(), cx))
                    .child(self.documents_pane.render(cx.entity().downgrade(), cx)),
            },
            Pane::Tickets => {
                let closure = self.tickets.query().closure;
                let filters =
                    h_flex()
                        .gap_1()
                        .children(CLOSURE_FILTERS.map(|(id, label, _, filter)| {
                            let workspace = cx.entity().downgrade();
                            choice(id, label, closure == filter, move |_, _, cx| {
                                let _ =
                                    workspace.update(cx, |this, cx| this.set_closure(filter, cx));
                            })
                        }));
                let showing = CLOSURE_FILTERS
                    .iter()
                    .find(|(.., filter)| *filter == closure)
                    .map(|(_, _, showing, _)| *showing);
                let header = v_flex()
                    .gap_1()
                    .child(filters)
                    .children(showing.map(|text| {
                        div()
                            .id("tickets-showing")
                            .test_support()
                            .aria_label(text)
                            .text_xs()
                            .child(text)
                    }));
                match self.tickets.repo() {
                    None => v_flex()
                        .gap_1()
                        .child(header)
                        .child(div().text_sm().child(sidebar::NOTHING_SELECTED)),
                    Some(_) => v_flex()
                        .gap_1()
                        .flex_1()
                        .min_h_0()
                        .child(header)
                        .child(list_status("tickets-state", self.tickets.state(), cx))
                        .child(self.tickets_pane.render()),
                }
            }
        };
        v_flex()
            .id("navigation-pane")
            .test_support()
            .gap_2()
            .size_full()
            .child(choices)
            .child(body)
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if std::mem::take(&mut self.tickets_changed) {
            let rows = self.tickets.state().rows().cloned().unwrap_or_default();
            self.tickets_pane.set(rows, window, cx);
        }
        let finishing = self.finishing_text(cx).map(|text| {
            h_flex()
                .gap_2()
                .child(div().id("quit-finishing").test_support().child(text))
                .child(
                    div().id("quit-now").test_support().child(
                        Button::new("quit-now-button")
                            .danger()
                            .label("Quit now")
                            .on_click(|_, window, cx| {
                                window.dispatch_action(Box::new(QuitNow), cx)
                            }),
                    ),
                )
        });
        let repositories = sidebar::repositories(&self.navigation, cx);
        let selection = sidebar::selection(&self.navigation, cx);
        let navigation = self.navigation_pane(cx);
        let content = v_flex()
            .id("content")
            .gap_3()
            .size_full()
            .overflow_y_scroll()
            .children(self.item.clone())
            .child(selection);
        let top = h_flex()
            .flex_1()
            .min_h_0()
            .w_full()
            .child(
                self.region(Region::Repositories, window, cx, repositories)
                    .w(px(260.))
                    .h_full(),
            )
            .child(
                self.region(Region::Navigation, window, cx, navigation)
                    .w(px(320.))
                    .h_full(),
            )
            .child(
                self.region(Region::Content, window, cx, content)
                    .flex_1()
                    .h_full(),
            );
        let operations = self
            .region(
                Region::Operations,
                window,
                cx,
                v_flex()
                    .gap_1()
                    .child(div().text_sm().child("Operations"))
                    .children(finishing),
            )
            .w_full()
            .h(px(120.));
        // Root does not draw these layers itself: the window's first view
        // below it does.
        v_flex()
            .id("workspace")
            .key_context("Workspace")
            .on_action(cx.listener(|this, _: &Quit, _, cx| {
                this.request_quit(cx);
            }))
            .on_action(cx.listener(|this, _: &QuitNow, _, cx| this.quit_now(cx)))
            .on_action(cx.listener(|this, _: &OpenSelected, _, cx| this.open_selected_document(cx)))
            .on_action(
                cx.listener(|this, _: &NextRegion, window, cx| this.cycle_region(1, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &PreviousRegion, window, cx| {
                    this.cycle_region(-1, window, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &FocusRepositories, window, cx| {
                this.focus_region(Region::Repositories, window, cx)
            }))
            .on_action(cx.listener(|this, _: &FocusNavigation, window, cx| {
                this.focus_region(Region::Navigation, window, cx)
            }))
            .on_action(cx.listener(|this, _: &FocusContent, window, cx| {
                this.focus_region(Region::Content, window, cx)
            }))
            .on_action(cx.listener(|this, _: &FocusOperations, window, cx| {
                this.focus_region(Region::Operations, window, cx)
            }))
            .relative()
            .size_full()
            .bg(cx.theme().background)
            .child(top)
            .child(operations)
            .children(Root::render_sheet_layer(window, cx))
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "workspace_tests.rs"]
mod tests;
