//! The main window's state and subscriptions. Private implementation children
//! own its transitions; sibling views render through queries and named actions.

mod content;
mod docks;
mod find;
mod focus;
mod footer;
mod layout_notices;
mod lifecycle;
mod lists;
mod render;
mod repositories;
mod requests;
mod titles;

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use gpui_kit::{
    AppContext, Context, Entity, FocusHandle, Subscription, Task, WeakEntity, Window,
    component::{
        dock::DockArea,
        input::{InputEvent, InputState},
        list::ListEvent,
        tree::TreeEvent,
    },
};

pub(crate) use super::quit::QuitPhase;
use super::{
    dock::{DockShell, Launch},
    documents::DocumentsPane,
    drafts::{DraftsEvent, DraftsPage},
    footer::Footer,
    item::{ChipField, ItemPane},
    passphrase::PassphraseDialog,
    repositories::RepositoriesPane,
    settings::{SettingsEvent, SettingsHost},
    tabs::TabContent,
    tickets::TicketsPane,
};
use crate::desktop::{
    model::{
        documents::Documents,
        find::Find,
        layout::{Layout, PanelKind, f6_stops},
        navigation::Navigation,
        stamp::{Generations, RepoKey},
        tickets::Tickets,
    },
    services::{Services, pending_conflicts::PendingConflict},
};
#[cfg(all(test, feature = "desktop-test-support"))]
pub(crate) use focus::REGIONS;
pub(crate) use focus::Region;
pub(in crate::desktop::view) use focus::ToolbarButton;
#[cfg(all(test, feature = "desktop-test-support"))]
use gpui_kit::App;
use manyhands::drafts::DraftId;
pub(in crate::desktop::view) use render::{modal_open, unless_dialog};

/// The tabs, the cached Settings entity and the cached Drafts page share
/// one content region; `content.rs` routes between them.
struct ContentArea {
    tabs: TabContent,
    settings: Entity<SettingsHost>,
    drafts: Entity<DraftsPage>,
    /// A tab's Review in Drafts, for the next frame to open.
    review: Option<DraftId>,
    /// A tab's create asked for Settings on this repository, where its
    /// initials are set: the next frame opens them.
    settings_for: Option<PathBuf>,
    /// A tab's restored draft was discarded: the next frame focuses the
    /// tab's content.
    refocus: bool,
    /// A tab closed, or followed its repaired item, once an answer landed:
    /// the next frame focuses the content region, as `refocus` does, but
    /// not while the details sheet is open, whose close gives focus back to
    /// its chip (Task 27 fix 2, N2).
    landed_refocus: bool,
    /// A chip's request for its tab's details sheet, for the next frame to
    /// open.
    details: Option<(WeakEntity<ItemPane>, ChipField)>,
    /// The tab whose details sheet is open: the sheet closes once another
    /// is shown.
    sheet: Option<WeakEntity<ItemPane>>,
}

pub(crate) struct Workspace {
    services: Services,
    /// One handle per region, in `REGIONS` order.
    regions: [FocusHandle; 5],
    /// The panels the person sees and how they are arranged.
    layout: Layout,
    /// The dock area, whose panels draw the regions.
    dock: DockShell,
    /// The footer's focus handles and the open dock menu.
    footer: Footer,
    navigation: Navigation,
    /// Replacing a read drops the previous task.
    list_read: Option<Task<()>>,
    resolve_read: Option<Task<()>>,
    documents: Documents,
    documents_pane: DocumentsPane,
    /// The folder and document reads, in parallel under one stamp.
    folder_read: Option<Task<()>>,
    document_read: Option<Task<()>>,
    /// The index status read beside them, for the files they could not
    /// read (Part I), under the lists' stamps.
    status_read: Option<Task<()>>,
    tickets: Tickets,
    tickets_pane: TicketsPane,
    ticket_read: Option<Task<()>>,
    /// The filter input, which was Find's: its text is the query's text
    /// (R-30).
    find_input: Entity<InputState>,
    /// The ID read behind Enter in the input, and its task.
    find: Find,
    find_read: Option<Task<()>>,
    /// The repository changed, so the filter text is cleared at the next
    /// render, which has the window `set_value` needs (R-30, R-35).
    filter_reset: bool,
    /// The selection whose change `filter_reset` already answered; `None`
    /// when no change is pending an answer, `Some(None)` for a change to no
    /// selection.
    change_applied: Option<Option<RepoKey>>,
    /// Around each lifecycle segment and sort header, by element ID: tab
    /// stops of their own.
    choice_focus: HashMap<&'static str, FocusHandle>,
    /// The panel toolbars' buttons, in `ToolbarButton::ALL` order. The
    /// Workspace owns them, so a dock `load` or move, which changes the
    /// header's element paths, keeps them (R-37).
    toolbar_focus: [FocusHandle; 4],
    _find_events: Subscription,
    content: ContentArea,
    /// The operation area's scroll: a control in it that takes focus is
    /// brought into view (E-2), sharing the focus the content region saw.
    operations_scroll: super::scroll::ContentScroll,
    _settings_events: Subscription,
    _drafts_events: Subscription,
    /// Fallback when the control holding focus is no longer drawn.
    last_region: Option<Region>,
    /// The title last given to the window, so it is set only when the
    /// derived title changes (R-14). Empty until the first frame.
    window_title: String,
    quit: QuitPhase,
    quit_focus: FocusHandle,
    /// The dialog layer owns the open passphrase dialog.
    passphrase: Option<WeakEntity<PassphraseDialog>>,
    /// While drafts are saved for quitting, and their dialog asks, a
    /// prompt waits with the request model, neither shown nor answered.
    prompts_held: bool,
    /// Launch refreshes are submitted once, after the first repository list.
    launched: bool,
    pending_reads: HashMap<RepoKey, Task<()>>,
    /// Each repository's pending rows whose conflict waits to be resolved,
    /// as its last read of them found (D5 B5).
    pending_conflicts: HashMap<RepoKey, Vec<PendingConflict>>,
    /// Each repository's read of them; a newer read drops the last.
    pending_conflict_reads: HashMap<RepoKey, Task<()>>,
    conflict_generations: Generations,
    abandon_focus: HashMap<(RepoKey, String), FocusHandle>,
    abandon_drawn: HashSet<(RepoKey, String)>,
    control_focus: HashMap<String, FocusHandle>,
    control_drawn: HashSet<String>,
    /// The items a Retire… press is in progress for, by repository and item
    /// ID: a second press waits (E4), unless the press is waiting on a
    /// question that no longer stands.
    pub(in crate::desktop::view) retire_asked:
        HashMap<(RepoKey, String), super::operations::RetireStep>,
    _operations: Subscription,
    /// A session's recovery state shows in the operation area.
    _drafts: Subscription,
    _focus_lost: Subscription,
    /// The window taking focus reads every listed repository's pending
    /// operations again: the lifecycle lock follows what another process
    /// did meanwhile (D5 A2, P26).
    _activation: Subscription,
    _ticket_events: Subscription,
    /// The folders the person opens and closes in the tree.
    _tree_events: Subscription,
    repositories_pane: RepositoriesPane,
    _repository_events: Subscription,
    remove_focus: FocusHandle,
    retry_focus: FocusHandle,
    /// Each change gives the open tabs their repository's publication
    /// remote from `navigation` (`TabContent::share_remotes`).
    _remotes: Subscription,
    /// Each Stop and republish… waiting for its Give up's answer (D5 A3).
    pub(in crate::desktop::view) stopping_to_republish:
        Vec<super::operations::republish::StopThenRepublish>,
    /// Each Republish… that stopped sequence opened while another dialog
    /// was open, waiting for it to close (R-32).
    pub(in crate::desktop::view) republish_deferred:
        Vec<(super::operations::republish::Republishing, FocusHandle)>,
    /// The titles of what a repository has not shared yet, and their reads.
    share_titles: titles::ShareTitles,
}

impl Workspace {
    pub(crate) fn new(services: Services, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let regions = [
            cx.focus_handle(),
            cx.focus_handle(),
            cx.focus_handle(),
            cx.focus_handle(),
            cx.focus_handle(),
        ];
        let operations = cx.observe_in(services.operations(), window, |this, _, window, cx| {
            this.operations_changed(window, cx)
        });
        let drafts = cx.observe(services.drafts(), |_, _, cx| cx.notify());
        // What the scheduler holds back for the person is drawn in Activity
        // (D6 C3): its entries change with its answers, not with a request.
        cx.observe(services.scheduler(), |this, _, cx| {
            cx.notify();
            this.read_share_titles(cx);
        })
        .detach();
        let focus_lost = cx.on_focus_lost(window, |this, window, cx| this.focus_lost(window, cx));
        let activation = cx.observe_window_activation(window, |this, window, cx| {
            if window.is_window_active() {
                this.reload_pending(cx);
            }
        });
        let workspace = cx.entity().downgrade();
        window.on_window_should_close(cx, move |window, cx| {
            workspace
                .update(cx, |workspace, cx| workspace.request_quit(window, cx))
                .unwrap_or(true)
        });
        let documents_pane = DocumentsPane::new(cx);
        let tree_events = cx.subscribe(documents_pane.tree(), |this, _, event: &TreeEvent, cx| {
            this.tree_event(event, cx)
        });
        let tickets_pane = TicketsPane::new(window, cx);
        let ticket_events = cx.subscribe(tickets_pane.list(), |this, list, event, cx| {
            // The Why blocked panel shows the selected row.
            cx.notify();
            if let ListEvent::Confirm(ix) = event {
                // An unreadable file selects and opens nothing (R-38).
                let target = this.tickets.repo().and_then(|repo| {
                    list.read(cx)
                        .delegate()
                        .row(*ix)
                        .and_then(|row| row.open_target(repo))
                });
                if let Some(target) = target {
                    this.open_item(target, cx);
                }
            }
        });
        let find_input = cx.new(|cx| {
            let mut input =
                InputState::new(window, cx).placeholder(super::tickets::FILTER_PLACEHOLDER);
            // Escape clears the text.
            input.set_clean_on_escape(true);
            input
        });
        let find_events = cx.subscribe_in(
            &find_input,
            window,
            |this, _, event: &InputEvent, window, cx| match event {
                InputEvent::PressEnter { .. } => this.submit_filter(window, cx),
                InputEvent::Change => this.filter_text_changed(cx),
                _ => {}
            },
        );
        let repositories_pane = RepositoriesPane::new(cx.entity().downgrade(), window, cx);
        let repository_events = cx.subscribe(repositories_pane.list(), |this, list, event, cx| {
            if let ListEvent::Confirm(ix) = event {
                let key = list.read(cx).delegate().row(*ix).map(|row| row.key.clone());
                if let Some(key) = key {
                    this.select_repository(key, cx);
                }
            }
        });
        let tabs = TabContent::default();
        let operations_scroll = tabs.scroll().region();
        let settings = {
            let services = services.clone();
            let scroll = tabs.scroll().clone();
            cx.new(|cx| SettingsHost::new(services, scroll, cx))
        };
        let settings_events = cx.subscribe_in(
            &settings,
            window,
            |this, _, event: &SettingsEvent, window, cx| match event {
                SettingsEvent::Closed => this.focus_region(Region::Content, window, cx),
                SettingsEvent::ShowActivity => this.focus_region(Region::Operations, window, cx),
            },
        );
        let drafts_page = {
            let services = services.clone();
            let scroll = tabs.scroll().clone();
            cx.new(|cx| DraftsPage::new(services, scroll, cx))
        };
        let drafts_events = cx.subscribe_in(
            &drafts_page,
            window,
            |this, _, event: &DraftsEvent, window, cx| this.drafts_event(event, window, cx),
        );
        let mut layout = Layout::default();
        let launch = Launch::read(services.backend().as_ref());
        let dock = DockShell::build(&regions, &mut layout, launch, window, cx);
        // Focus starts on Repositories if it is displayed, otherwise on the
        // first F6 stop (R-9).
        let start = if layout.is_displayed(PanelKind::Repositories) {
            Region::Repositories
        } else {
            f6_stops(&layout)
                .first()
                .map_or(Region::Content, |stop| Region::of_stop(*stop))
        };
        window.focus(&regions[start.index()], cx);
        let mut workspace = Self {
            services,
            regions,
            layout,
            dock,
            footer: Footer::new(cx),
            navigation: Navigation::default(),
            list_read: None,
            resolve_read: None,
            documents: Documents::default(),
            documents_pane,
            folder_read: None,
            document_read: None,
            status_read: None,
            tickets: Tickets::default(),
            tickets_pane,
            ticket_read: None,
            find_input,
            find: Find::default(),
            find_read: None,
            filter_reset: false,
            change_applied: None,
            choice_focus: super::tickets::choice_ids()
                .map(|id| (id, cx.focus_handle().tab_stop(true)))
                .collect(),
            toolbar_focus: [
                cx.focus_handle(),
                cx.focus_handle(),
                cx.focus_handle(),
                cx.focus_handle(),
            ],
            _find_events: find_events,
            operations_scroll,
            content: ContentArea {
                tabs,
                settings,
                drafts: drafts_page,
                review: None,
                settings_for: None,
                refocus: false,
                landed_refocus: false,
                details: None,
                sheet: None,
            },
            _settings_events: settings_events,
            _drafts_events: drafts_events,
            last_region: Some(start),
            window_title: String::new(),
            quit: QuitPhase::Running,
            quit_focus: cx.focus_handle().tab_stop(true),
            passphrase: None,
            prompts_held: false,
            launched: false,
            pending_reads: HashMap::new(),
            pending_conflicts: HashMap::new(),
            pending_conflict_reads: HashMap::new(),
            conflict_generations: Generations::default(),
            abandon_focus: HashMap::new(),
            abandon_drawn: HashSet::new(),
            control_focus: HashMap::new(),
            control_drawn: HashSet::new(),
            retire_asked: HashMap::new(),
            _operations: operations,
            _drafts: drafts,
            _focus_lost: focus_lost,
            _activation: activation,
            _ticket_events: ticket_events,
            _tree_events: tree_events,
            repositories_pane,
            _repository_events: repository_events,
            remove_focus: cx.focus_handle(),
            retry_focus: cx.focus_handle(),
            _remotes: cx
                .observe_self(|this, cx| this.content.tabs.share_remotes(&this.navigation, cx)),
            stopping_to_republish: Vec::new(),
            republish_deferred: Vec::new(),
            share_titles: Default::default(),
        };
        workspace.load_repositories(cx);
        workspace
    }

    pub(crate) fn services(&self) -> &Services {
        &self.services
    }
    pub(in crate::desktop::view) fn operations_scroll(&self) -> &super::scroll::ContentScroll {
        &self.operations_scroll
    }
    pub(crate) fn navigation(&self) -> &Navigation {
        &self.navigation
    }
    /// The pending row `operation_id` of `root`, when its conflict waits
    /// to be resolved (D5 B5).
    pub(in crate::desktop::view) fn pending_conflict(
        &self,
        root: &str,
        operation_id: &str,
    ) -> Option<&PendingConflict> {
        self.pending_conflicts
            .get(root)?
            .iter()
            .find(|conflict| conflict.operation_id.to_string() == operation_id)
    }
    pub(crate) fn documents(&self) -> &Documents {
        &self.documents
    }
    /// A test's way to leave the tree as a person would: a folder collapsed.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(super) fn documents_mut(&mut self) -> &mut Documents {
        &mut self.documents
    }
    pub(super) fn tickets(&self) -> &Tickets {
        &self.tickets
    }
    pub(super) fn documents_pane(&self) -> &DocumentsPane {
        &self.documents_pane
    }
    pub(super) fn tickets_pane(&self) -> &TicketsPane {
        &self.tickets_pane
    }
    /// The focus handle of `region`, which tells rows whether it holds
    /// focus.
    pub(in crate::desktop::view) fn region_focus(&self, region: Region) -> FocusHandle {
        self.regions[region.index()].clone()
    }
    /// The handle a toolbar button tracks (R-37).
    pub(in crate::desktop::view) fn toolbar_focus(&self, button: ToolbarButton) -> &FocusHandle {
        &self.toolbar_focus[button.index()]
    }
    pub(super) fn choice_focus(&self, id: &str) -> Option<&FocusHandle> {
        self.choice_focus.get(id)
    }
    /// The dock area, whose panels draw the regions.
    pub(crate) fn dock_area(&self) -> &Entity<DockArea> {
        self.dock.area()
    }
    pub(crate) fn quit_phase(&self) -> QuitPhase {
        self.quit
    }
    pub(super) fn quit_focus(&self) -> &FocusHandle {
        &self.quit_focus
    }

    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn layout(&self) -> &Layout {
        &self.layout
    }

    /// The dump the shell last accepted as the layout (R-6, R-38).
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn dock_accepted(&self) -> &gpui_kit::component::dock::DockAreaState {
        self.dock.accepted()
    }

    /// How many saves the layout has asked for (R-31).
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn layout_saves(&self) -> usize {
        self.dock.saves()
    }

    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn passphrase(&self) -> Option<Entity<PassphraseDialog>> {
        self.passphrase.as_ref().and_then(WeakEntity::upgrade)
    }

    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn repositories_list_focus(&self, cx: &App) -> FocusHandle {
        gpui_kit::Focusable::focus_handle(self.repositories_pane.list().read(cx), cx)
    }

    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn tickets_list_focus(&self, cx: &App) -> FocusHandle {
        gpui_kit::Focusable::focus_handle(self.tickets_pane.list().read(cx), cx)
    }

    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn remove_focus(&self) -> FocusHandle {
        self.remove_focus.clone()
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "workspace_tests.rs"]
mod tests;
