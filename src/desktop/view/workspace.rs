//! The main window's view: the shell of four focusable regions and the
//! repository selector. The navigation lists live in `lists`, the
//! content region in `tabs`, the operation area in `operations` and
//! quitting in `quit`.
//!
//! The regions are focus groups: the repository list, the documents and
//! tickets navigation, the content and the operation area. `F6` and
//! `shift-F6` cycle them; `secondary-1` to `secondary-4` focus each. The
//! focused region draws a focus ring, and is the only one that does.

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use gpui_kit::{
    App, Context, Entity, FocusHandle, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Subscription, Task, TestSupportExt, Window,
    component::{ActiveTheme, Root, WindowExt, h_flex, list::ListEvent, v_flex},
    div, px,
};
use manyhands::repository::OperationId;

pub(crate) use super::quit::QuitPhase;
use super::{
    confirm::{confirm, prepare_again},
    documents::DocumentsPane,
    item::ItemPane,
    sidebar::{self, RepositoriesPane, RepositoryRow},
    tickets::TicketsPane,
};
use crate::desktop::{
    keys::{
        FocusContent, FocusNavigation, FocusOperations, FocusRepositories, NextRegion,
        OpenSelected, PreviousRegion, Quit, QuitNow, Rebuild, Refresh,
    },
    model::{
        documents::Documents,
        navigation::{Navigation, Pane},
        operations::RecoveryStep,
        stamp::{RepoKey, Stamp},
        tickets::Tickets,
    },
    services::{
        ReadKind, Services,
        requests::{DesktopRequestId, Intent, Lane, RequestState},
    },
};
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
    pub(super) fn index(self) -> usize {
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
    pub(super) services: Services,
    /// One handle per region, in `REGIONS` order.
    pub(super) regions: [FocusHandle; 4],
    pub(super) navigation: Navigation,
    /// The repository list read in flight; replacing it drops the last.
    list_read: Option<Task<()>>,
    /// The selection's resolution in flight; replacing it drops the last.
    resolve_read: Option<Task<()>>,
    pub(super) documents: Documents,
    pub(super) documents_pane: DocumentsPane,
    /// The folder and document reads, in parallel under one stamp.
    pub(super) folder_read: Option<Task<()>>,
    pub(super) document_read: Option<Task<()>>,
    pub(super) tickets: Tickets,
    pub(super) tickets_pane: TicketsPane,
    pub(super) ticket_read: Option<Task<()>>,
    /// The ticket rows changed and go to the Kit's list at the next frame,
    /// which has the window its selection needs.
    pub(super) tickets_changed: bool,
    /// The item the last activated row opened, in the content region (Task 8
    /// makes it a tab).
    pub(super) item: Option<Entity<ItemPane>>,
    pub(super) quit: QuitPhase,
    /// The launch refreshes were submitted: once, after the repository
    /// list first loads.
    launched: bool,
    /// Each repository's pending-operations read in flight.
    pending_reads: HashMap<RepoKey, Task<()>>,
    /// The focus of each pending row's Abandon, so the dialog can give it
    /// back.
    abandon_focus: HashMap<(RepoKey, String), FocusHandle>,
    /// The Abandon controls the frame being drawn shows.
    abandon_drawn: HashSet<(RepoKey, String)>,
    _operations: Subscription,
    _ticket_events: Subscription,
    repositories_pane: RepositoriesPane,
    _repository_events: Subscription,
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
        let repositories_pane = RepositoriesPane::new(window, cx);
        let repository_events = cx.subscribe(repositories_pane.list(), |this, list, event, cx| {
            // A click or Enter on a row selects it.
            if let ListEvent::Confirm(ix) = event {
                let key = list.read(cx).delegate().row(*ix).map(|row| row.key.clone());
                if let Some(key) = key {
                    this.select_repository(key, cx);
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
            launched: false,
            pending_reads: HashMap::new(),
            abandon_focus: HashMap::new(),
            abandon_drawn: HashSet::new(),
            _operations: operations,
            _ticket_events: ticket_events,
            repositories_pane,
            _repository_events: repository_events,
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
                    // One the list let go is resolved again, which says
                    // why it is gone; its lists go until it resolves.
                    if let Some(key) = this.navigation.take_vanished() {
                        let stamp = this.navigation.retry_resolution(key.clone());
                        this.clear_lists(cx);
                        this.resolve(key, stamp, cx);
                    }
                    this.repositories_loaded(cx);
                    cx.notify();
                }
            },
        ));
        cx.notify();
    }

    /// The listed roots, in list order.
    fn listed(&self) -> Option<Vec<RepoKey>> {
        self.navigation
            .repositories()
            .rows()
            .map(|rows| rows.iter().map(|entry| entry.key().clone()).collect())
    }

    /// The repository list loaded: the first time, every listed
    /// repository is refreshed in the background, in list order. The
    /// pending operations of each repository listed for the first time are
    /// read, and those of one no longer listed let go. A repository already
    /// read is read again only when a finished request changed it
    /// (`operations_changed`), not whenever the list is.
    fn repositories_loaded(&mut self, cx: &mut Context<Self>) {
        let Some(roots) = self.listed() else {
            return;
        };
        if !self.launched {
            self.launched = true;
            if self.services.launch_refresh() {
                for root in &roots {
                    self.services.worker().submit(
                        Intent::IndexRefresh {
                            root: PathBuf::from(root),
                        },
                        Lane::Background,
                        None,
                        None,
                    );
                }
            }
        }
        self.services.operations().update(cx, |operations, cx| {
            operations.retain_pending(&roots);
            cx.notify();
        });
        self.pending_reads.retain(|root, _| roots.contains(root));
        for root in roots {
            if !self.pending_reads.contains_key(&root) {
                self.load_pending(root, cx);
            }
        }
    }

    /// Reads `root`'s pending operations again. The read resolves the
    /// repository first; one that does not resolve lists nothing.
    #[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
    pub(crate) fn load_pending(&mut self, root: RepoKey, cx: &mut Context<Self>) {
        let stamp = self
            .services
            .operations()
            .update(cx, |operations, _| operations.load_pending(root.clone()));
        let path = PathBuf::from(&root);
        let read = self.services.read(
            ReadKind::Operations,
            stamp,
            move |service| {
                let repo = service.resolve_repository(&path)?;
                service.list_operations(&repo)
            },
            cx,
            |this: &mut Self, stamp, result, cx| {
                this.services.operations().update(cx, |operations, cx| {
                    if operations.apply_pending(stamp, result) {
                        cx.notify();
                    }
                });
            },
        );
        self.pending_reads.insert(root, read);
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

    /// Shows `pane`. While the navigation region holds focus, the pane's
    /// tree or list takes it.
    pub(super) fn show_pane(&mut self, pane: Pane, window: &mut Window, cx: &mut Context<Self>) {
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

    /// Focuses `region`. The repositories region gives its focus to the
    /// selector's list when it has rows, and the navigation region to the
    /// shown pane's tree or list when a repository's lists are shown, so
    /// their arrow keys and Enter work.
    fn focus_region(&mut self, region: Region, window: &mut Window, cx: &mut Context<Self>) {
        match (region, self.navigation.pane()) {
            (Region::Repositories, _)
                if self
                    .navigation
                    .repositories()
                    .rows()
                    .is_some_and(|rows| !rows.is_empty()) =>
            {
                self.repositories_pane.focus(window, cx)
            }
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

    /// A worker event arrived: if quitting waits for a job that has now
    /// returned, quit. A repository a finished request may have changed
    /// is read again: its pending operations, its lists when it is
    /// selected, and the repository list, whose index state it shows.
    fn operations_changed(&mut self, cx: &mut Context<Self>) {
        // The operation area shows the model.
        cx.notify();
        self.quit_if_finished(cx);
        let changed = self
            .services
            .operations()
            .update(cx, |operations, _| operations.take_changed());
        if changed.is_empty() {
            return;
        }
        let listed = self.listed().unwrap_or_default();
        for root in &changed {
            if listed.contains(root) {
                self.load_pending(root.clone(), cx);
            }
        }
        if self
            .navigation
            .selected()
            .is_some_and(|selected| changed.contains(selected))
        {
            self.reload_lists(cx);
        }
        self.load_repositories(cx);
    }

    /// Refresh or Rebuild: the selected repository's index, as a user
    /// job. With no repository selected, nothing.
    pub(crate) fn maintain_index(
        &mut self,
        rebuild: bool,
        cx: &mut Context<Self>,
    ) -> Option<DesktopRequestId> {
        let root = PathBuf::from(self.navigation.selected()?);
        let intent = if rebuild {
            Intent::IndexRebuild { root }
        } else {
            Intent::IndexRefresh { root }
        };
        let id = self
            .services
            .worker()
            .submit(intent, Lane::User, None, None);
        cx.notify();
        Some(id)
    }

    pub(super) fn cancel_request(&mut self, id: DesktopRequestId, cx: &mut Context<Self>) {
        self.services.worker().cancel(id);
        cx.notify();
    }

    pub(super) fn dismiss_request(&mut self, id: DesktopRequestId, cx: &mut Context<Self>) {
        self.services.operations().update(cx, |operations, cx| {
            operations.dismiss(id);
            cx.notify();
        });
    }

    /// Resume on a pending row: `operation resume` as a user job.
    pub(super) fn resume(&mut self, root: RepoKey, operation_id: &str, cx: &mut Context<Self>) {
        let Ok(operation_id) = OperationId::parse(operation_id) else {
            return;
        };
        self.services.worker().submit(
            Intent::OperationResume {
                root: PathBuf::from(root),
                operation_id,
            },
            Lane::User,
            None,
            None,
        );
        cx.notify();
    }

    /// Abandon on a pending row: the confirmation dialog for `operation
    /// abandon`, which gives focus back to `origin`.
    pub(super) fn abandon(
        &mut self,
        root: RepoKey,
        operation_id: &str,
        origin: FocusHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Ok(operation_id) = OperationId::parse(operation_id) else {
            return;
        };
        window.focus(&origin, cx);
        confirm(
            Intent::OperationAbandon {
                root: PathBuf::from(root),
                operation_id,
            },
            origin,
            &self.services,
            window,
            cx,
        );
    }

    /// The focus of the Abandon of `root`'s operation `operation_id`,
    /// made the first time it is drawn.
    pub(super) fn abandon_focus_for(
        &mut self,
        root: &RepoKey,
        operation_id: &str,
        cx: &mut Context<Self>,
    ) -> FocusHandle {
        let key = (root.clone(), operation_id.to_owned());
        self.abandon_drawn.insert(key.clone());
        self.abandon_focus
            .entry(key)
            .or_insert_with(|| cx.focus_handle())
            .clone()
    }

    /// Lets go of the focus of every Abandon this frame does not draw. If
    /// one of them held focus, its row is gone (abandoned, resumed, or
    /// read again without it): focus goes to the operation area itself,
    /// the region the control was in (design, Layout And Keyboard).
    fn prune_abandon_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let drawn = std::mem::take(&mut self.abandon_drawn);
        let mut lost = false;
        self.abandon_focus.retain(|key, focus| {
            let kept = drawn.contains(key);
            lost |= !kept && focus.is_focused(window);
            kept
        });
        if lost {
            let fallback = self.regions[Region::Operations.index()].clone();
            window.defer(cx, move |window, cx| window.focus(&fallback, cx));
        }
    }

    #[cfg(test)]
    pub(crate) fn abandon_focus(&self, root: &str, operation_id: &str) -> Option<FocusHandle> {
        self.abandon_focus
            .get(&(root.to_owned(), operation_id.to_owned()))
            .cloned()
    }

    /// Runs the recovery action `n` of the request `id`.
    pub(super) fn run_recovery(
        &mut self,
        id: DesktopRequestId,
        n: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let step = {
            let operations = self.services.operations().read(cx);
            let Some(RequestState::Finished(_, _, _, recovery, _)) =
                operations.request(id).map(|request| &request.state)
            else {
                return;
            };
            let Some(action) = recovery.get(n) else {
                return;
            };
            operations.recovery_step(id, action)
        };
        let origin = self.regions[Region::Operations.index()].clone();
        match step {
            Some(RecoveryStep::Submit(intent)) => {
                self.services
                    .worker()
                    .submit(intent, Lane::User, None, None);
            }
            Some(RecoveryStep::Confirm(intent)) => {
                confirm(intent, origin, &self.services, window, cx);
            }
            Some(RecoveryStep::Retry(id)) => self.services.worker().retry(id, None),
            Some(RecoveryStep::Prepare(id)) => {
                prepare_again(id, origin, &self.services, window, cx);
            }
            Some(RecoveryStep::Cancel(id)) => {
                self.services.worker().cancel(id);
            }
            None => {}
        }
        cx.notify();
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

/// A workspace action's handler that does nothing while a dialog is open.
/// The dialog layer is drawn inside the workspace, so its keys reach these
/// handlers, and the Kit's focus trap holds only Tab and shift-Tab. Every
/// dialog traps focus (design, Layout And Keyboard): a region key would
/// focus behind it, out of Escape's reach, and Refresh would submit.
fn unless_dialog<A: 'static>(
    cx: &mut Context<Workspace>,
    f: impl Fn(&mut Workspace, &A, &mut Window, &mut Context<Workspace>) + 'static,
) -> impl Fn(&A, &mut Window, &mut App) + 'static {
    cx.listener(move |this, action: &A, window, cx| {
        if !window.has_active_dialog(cx) {
            f(this, action, window, cx)
        }
    })
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_tickets(window, cx);
        let finishing = self.finishing(cx);
        self.repositories_pane
            .set(RepositoryRow::rows(&self.navigation), window, cx);
        let repositories = sidebar::repositories(&self.navigation, &self.repositories_pane, cx);
        let selection = sidebar::selection(&self.navigation, cx);
        let navigation = self.navigation_pane(cx);
        let content = v_flex()
            .id("content")
            .gap_3()
            .size_full()
            .overflow_y_scroll()
            .children(self.content_item())
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
        let session = self.session_section(cx);
        let pending = self.pending_section(window, cx);
        self.prune_abandon_focus(window, cx);
        let operations = self
            .region(
                Region::Operations,
                window,
                cx,
                v_flex()
                    .id("operation-area")
                    .gap_2()
                    .size_full()
                    .overflow_y_scroll()
                    .children(finishing)
                    .child(
                        h_flex()
                            .gap_4()
                            .items_start()
                            .child(session.flex_1())
                            .child(pending.flex_1()),
                    ),
            )
            .w_full()
            .h(px(260.));
        // Root does not draw these layers itself: the window's first view
        // below it does.
        v_flex()
            .id("workspace")
            .key_context("Workspace")
            // Quit works with a dialog open; nothing else here does.
            .on_action(cx.listener(|this, _: &Quit, _, cx| {
                this.request_quit(cx);
            }))
            .on_action(cx.listener(|this, _: &QuitNow, _, cx| this.quit_now(cx)))
            .on_action(unless_dialog(cx, |this, _: &OpenSelected, _, cx| {
                this.open_selected_document(cx)
            }))
            .on_action(unless_dialog(cx, |this, _: &Refresh, _, cx| {
                this.maintain_index(false, cx);
            }))
            .on_action(unless_dialog(cx, |this, _: &Rebuild, _, cx| {
                this.maintain_index(true, cx);
            }))
            .on_action(unless_dialog(cx, |this, _: &NextRegion, window, cx| {
                this.cycle_region(1, window, cx)
            }))
            .on_action(unless_dialog(cx, |this, _: &PreviousRegion, window, cx| {
                this.cycle_region(-1, window, cx)
            }))
            .on_action(unless_dialog(
                cx,
                |this, _: &FocusRepositories, window, cx| {
                    this.focus_region(Region::Repositories, window, cx)
                },
            ))
            .on_action(unless_dialog(
                cx,
                |this, _: &FocusNavigation, window, cx| {
                    this.focus_region(Region::Navigation, window, cx)
                },
            ))
            .on_action(unless_dialog(cx, |this, _: &FocusContent, window, cx| {
                this.focus_region(Region::Content, window, cx)
            }))
            .on_action(unless_dialog(
                cx,
                |this, _: &FocusOperations, window, cx| {
                    this.focus_region(Region::Operations, window, cx)
                },
            ))
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
