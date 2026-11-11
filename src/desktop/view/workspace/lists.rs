//! Navigation-list read scheduling, filters and result application.

use std::path::PathBuf;

use super::{Region, Workspace};
use crate::desktop::{
    model::{
        documents::folder_path,
        list_state::ListState,
        list_view::{ListAction, ListKind},
        markers::row_markers,
        navigation::{Pane, Resolution},
        polling::retire_loop::Held,
        stamp::{RepoKey, Stamp},
        tickets::{Lifecycle, SortColumn, TicketQuery, read_filter},
        unreadable::split_status,
    },
    services::ReadKind,
    view::tickets::ShownTicket,
};
use gpui_kit::{App, Context, Window, component::tree::TreeEvent};
use manyhands::repository::{ClosureFilter, ItemDtoKind, ReadinessFilter, ResolvedRepository};

impl Workspace {
    pub(super) fn resolved_selection(&self) -> Option<(RepoKey, ResolvedRepository)> {
        let entry = self.navigation.selected_entry()?;
        match &entry.resolution {
            Resolution::Resolved(resolved) => Some((entry.key().clone(), resolved.clone())),
            _ => None,
        }
    }

    pub(crate) fn load_lists(&mut self, cx: &mut Context<Self>) {
        self.clear_lists(cx);
        let Some((key, resolved)) = self.resolved_selection() else {
            return;
        };
        self.load_lists_of(key, resolved, cx);
    }

    /// The lists' reads and the status read beside them, under the stamps
    /// of one load.
    fn load_lists_of(
        &mut self,
        key: RepoKey,
        resolved: ResolvedRepository,
        cx: &mut Context<Self>,
    ) {
        let documents = self.load_documents(key.clone(), resolved.clone(), cx);
        let tickets = self.load_tickets(key, resolved.clone(), cx);
        self.load_status(tickets, documents, resolved, cx);
    }

    /// Reload retains rows and selection until their replacement arrives.
    pub(in crate::desktop::view) fn reload_lists(&mut self, cx: &mut Context<Self>) {
        let Some((key, resolved)) = self.resolved_selection() else {
            return;
        };
        if self.documents.repo() != Some(&key) {
            return self.load_lists(cx);
        }
        self.load_lists_of(key, resolved, cx);
    }

    pub(super) fn clear_lists(&mut self, cx: &mut Context<Self>) {
        self.documents.clear();
        self.documents_pane.clear(cx);
        self.folder_read = None;
        self.document_read = None;
        self.status_read = None;
        self.tickets.clear();
        self.ticket_read = None;
        // A Retry or a reload of the same repository keeps the filter text
        // and a held ID read; only another repository drops both (R-35).
        let to = self.navigation.selected().cloned();
        let changed = self.tickets.is_change_to(to.as_ref());
        self.repository_change(to, changed, cx);
        cx.notify();
    }

    /// The selected repository is not the one last loaded: a held ID read
    /// is dropped now, and the filter text is cleared at the next render,
    /// which has the window `set_value` needs (R-30, R-35). Once per
    /// change: the clear before a load and the load itself both see it.
    fn repository_change(&mut self, to: Option<RepoKey>, changed: bool, cx: &mut Context<Self>) {
        if !changed {
            self.change_applied = None;
        } else if self.change_applied.as_ref() != Some(&to) {
            // `Some(None)` is the change to no selection, applied: distinct
            // from `None`, no change applied.
            self.change_applied = Some(to);
            self.clear_find(cx);
            self.filter_reset = true;
        }
    }

    /// Clears the filter text a change of repository left, now there is a
    /// window to clear it in.
    pub(super) fn apply_filter_reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.filter_reset) {
            self.set_filter_text("", window, cx);
        }
    }

    /// Folder and document reads run in parallel under one stamp.
    #[allow(clippy::result_large_err)]
    fn load_documents(
        &mut self,
        key: RepoKey,
        resolved: ResolvedRepository,
        cx: &mut Context<Self>,
    ) -> Stamp {
        let (stamp, _) = self.documents.load(key);
        let repo = resolved.clone();
        let loaded = stamp.clone();
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
        loaded
    }

    /// The status read of a load: the files the lists could not read.
    /// Its answer goes to each list whose own stamp is still current, so
    /// a busy or failed one leaves them as they were.
    #[allow(clippy::result_large_err)]
    fn load_status(
        &mut self,
        tickets: Stamp,
        documents: Stamp,
        resolved: ResolvedRepository,
        cx: &mut Context<Self>,
    ) {
        let root = resolved.root().to_string_lossy().into_owned();
        self.status_read = Some(self.services.read(
            ReadKind::IndexStatus,
            documents,
            move |service| service.index_status(&resolved),
            cx,
            move |this: &mut Self, documents, result, cx| {
                let (for_tickets, for_documents) = split_status(result, &root);
                if this.tickets.apply_unreadable(tickets, for_tickets) {
                    cx.notify();
                }
                if this.documents.apply_unreadable(documents, for_documents) {
                    this.documents_changed(cx);
                }
            },
        ));
    }

    /// The tree the pane shows is the model's: its ready rows, or the ones
    /// it kept over a reload, built under the repository's expansion.
    fn documents_changed(&mut self, cx: &mut Context<Self>) {
        match (
            self.documents.repo(),
            self.documents.list().rows(),
            self.documents.expansion(),
        ) {
            (Some(repo), Some(rows), Some(expansion)) => {
                self.documents_pane.set(repo, rows, expansion, cx)
            }
            // One read landed: keep the previous rows until the other does.
            (Some(_), None, _) if matches!(self.documents.state(), ListState::Loading) => {}
            _ => self.documents_pane.clear(cx),
        }
        cx.notify();
    }

    #[allow(clippy::result_large_err)]
    fn load_tickets(
        &mut self,
        key: RepoKey,
        resolved: ResolvedRepository,
        cx: &mut Context<Self>,
    ) -> Stamp {
        let (stamp, changed) = self.tickets.load(key.clone());
        self.repository_change(Some(key), changed, cx);
        self.read_tickets(stamp.clone(), resolved, cx);
        stamp
    }

    /// The one read of the load: every ticket, whatever the query (R-1).
    #[allow(clippy::result_large_err)]
    fn read_tickets(&mut self, stamp: Stamp, resolved: ResolvedRepository, cx: &mut Context<Self>) {
        let filter = read_filter();
        self.ticket_read = Some(self.services.read(
            ReadKind::Tickets,
            stamp,
            move |service| service.list_tickets(&resolved, &filter),
            cx,
            |this: &mut Self, stamp, result, cx| {
                if this.tickets.apply(stamp, result) {
                    cx.notify();
                }
            },
        ));
        cx.notify();
    }

    /// Lists `query` from now on. Nothing is read: the rows are derived
    /// from the read the model holds.
    fn show_tickets(&mut self, query: TicketQuery, cx: &mut Context<Self>) {
        self.tickets.set_query(query);
        cx.notify();
    }

    pub(crate) fn set_closure(&mut self, closure: ClosureFilter, cx: &mut Context<Self>) {
        let query = self.tickets.query().clone().with_closure(closure);
        self.show_tickets(query, cx);
    }

    pub(crate) fn set_readiness(
        &mut self,
        readiness: Option<ReadinessFilter>,
        cx: &mut Context<Self>,
    ) {
        let query = self.tickets.query().clone().with_readiness(readiness);
        self.show_tickets(query, cx);
    }

    /// Sorts by `column`: a new column ascending, the active one reversed.
    pub(crate) fn set_sort(&mut self, column: SortColumn, cx: &mut Context<Self>) {
        let mut query = self.tickets.query().clone();
        query.sort = query.sort.toggled(column);
        self.show_tickets(query, cx);
    }

    /// The filter text as the input holds it, after a change in the input
    /// itself: the query follows. No read, and nothing moves (R-30).
    pub(super) fn filter_text_changed(&mut self, cx: &mut Context<Self>) {
        let text = self.find_input.read(cx).value().to_string();
        if text != self.tickets.query().text {
            let mut query = self.tickets.query().clone();
            query.text = text;
            self.show_tickets(query, cx);
        }
    }

    /// Puts `text` in the filter input and in the query, the one source of
    /// the filter text (R-30).
    pub(crate) fn set_filter_text(
        &mut self,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.find_input
            .update(cx, |input, cx| input.set_value(text, window, cx));
        self.filter_text_changed(cx);
    }

    /// Clear filters: no text, Any readiness, and focus in the input
    /// (R-28). The lifecycle stays.
    pub(crate) fn clear_filters(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let query = self.tickets.query().clone().with_readiness(None);
        self.show_tickets(query, cx);
        self.set_filter_text("", window, cx);
        self.find_input
            .update(cx, |input, cx| input.focus(window, cx));
    }

    /// Show all: every ticket, ready or not, matching the text, and focus
    /// in the input (R-28).
    pub(crate) fn show_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let query = self
            .tickets
            .query()
            .clone()
            .with_readiness(None)
            .with_closure(ClosureFilter::All);
        self.show_tickets(query, cx);
        self.find_input
            .update(cx, |input, cx| input.focus(window, cx));
    }

    /// What an alert's or an empty state's button does (Part A), in the
    /// panel of `kind`: Refresh, Retry and Rebuild leave focus on that
    /// panel's list or tree (R-28).
    pub(in crate::desktop::view) fn list_action(
        &mut self,
        action: ListAction,
        kind: ListKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action {
            ListAction::Refresh => {
                self.maintain_index(false, cx);
                self.focus_list(kind, window, cx);
            }
            ListAction::Retry => {
                self.reload_lists(cx);
                self.focus_list(kind, window, cx);
            }
            ListAction::RebuildIndex => {
                self.maintain_index(true, cx);
                self.focus_list(kind, window, cx);
            }
            ListAction::ClearFilters => self.clear_filters(window, cx),
            ListAction::ShowAll => self.show_all(window, cx),
            ListAction::NewItem => self.new_item(
                match kind {
                    ListKind::Tickets => Pane::Tickets,
                    ListKind::Documents => Pane::Documents,
                },
                window,
                cx,
            ),
        }
        cx.notify();
    }

    fn focus_list(&mut self, kind: ListKind, window: &mut Window, cx: &mut Context<Self>) {
        match kind {
            ListKind::Tickets => self.tickets_pane.focus(window, cx),
            ListKind::Documents => self.documents_pane.focus(window, cx),
        }
    }

    /// Home and End in the ticket list: the first or the last row, and the
    /// Workspace is told, so Why blocked follows (R-15).
    pub(in crate::desktop::view) fn select_edge(
        &mut self,
        last: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.tickets_pane.select_edge(last, window, cx);
        cx.notify();
    }

    /// Home and End in the document tree: the first or the last row (R-15).
    pub(in crate::desktop::view) fn select_document_edge(
        &mut self,
        last: bool,
        cx: &mut Context<Self>,
    ) {
        self.documents_pane.select_edge(last, cx);
        cx.notify();
    }

    /// The selected repository's display name, once its documents are
    /// loaded.
    pub(in crate::desktop::view) fn documents_repo_name(&self) -> Option<String> {
        let key = self.documents.repo()?;
        Some(self.repo_name(key))
    }

    /// The selected repository's display name, once its tickets are loaded.
    pub(in crate::desktop::view) fn tickets_repo_name(&self) -> Option<String> {
        let key = self.tickets.repo()?;
        Some(self.repo_name(key))
    }

    /// Open ready tickets (D5 design, A3, R-20; R-29): the repository at
    /// `root` selected, its Tickets pane shown under Open and Ready with no
    /// text and the sort kept, and the navigation focused. The repository's
    /// tickets are read once. A repository not yet selected keeps that
    /// query through its selection: the list is read under it once it
    /// resolves.
    pub(in crate::desktop::view) fn open_ready_tickets(
        &mut self,
        root: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(key) = root.to_str().map(str::to_owned) else {
            return;
        };
        let query = self
            .tickets
            .query()
            .clone()
            .with_closure(ClosureFilter::Open)
            .with_readiness(Some(ReadinessFilter::Ready));
        self.show_tickets(query, cx);
        self.set_filter_text("", window, cx);
        if self.navigation.selected() == Some(&key) {
            if let Some((key, resolved)) = self.resolved_selection() {
                self.load_tickets(key, resolved, cx);
            }
        } else {
            // Selecting clears the list first and reads it under the query.
            self.select_repository(key, cx);
        }
        self.focus_region(Region::Tickets, window, cx);
    }

    /// Enter in the tree: a folder with child nodes opens or closes, as a
    /// click does (R-24), through the model and a rebuild, since the Kit's
    /// toggle is private; a document opens.
    pub(in crate::desktop::view) fn open_selected_document(&mut self, cx: &mut Context<Self>) {
        if let Some((path, open)) = self.documents_pane.toggle_target(cx) {
            self.documents.set_expanded(&path, !open);
            self.documents_changed(cx);
        } else if let Some(target) = self.documents_pane.selected_target(cx) {
            self.open_item(target, cx);
        }
    }

    /// A folder the person opened or closed in the tree, by key or by
    /// mouse: the model keeps it, so a reload or a hidden panel does too
    /// (R-12, P19). The tree's own state already shows it.
    pub(super) fn tree_event(&mut self, event: &TreeEvent, cx: &mut Context<Self>) {
        let (id, open) = match event {
            TreeEvent::Expanded(id) => (id, true),
            TreeEvent::Collapsed(id) => (id, false),
        };
        if let Some(path) = folder_path(id) {
            self.documents.set_expanded(path, open);
            cx.notify();
        }
    }

    /// The rows the panel shows, with the marks each carries, worked out
    /// now from the draft registry and the repository's pending conflicts
    /// (R-32): a new draft or conflict shows at once, with no read.
    fn shown_tickets(&self, cx: &App) -> Vec<ShownTicket> {
        let Some(repo) = self.tickets.repo() else {
            return Vec::new();
        };
        let closure = self.tickets.query().closure;
        let drafts = self.services.drafts().read(cx);
        let conflicts = self.pending_conflicts.get(repo);
        self.tickets
            .view()
            .rows
            .iter()
            .map(|row| {
                let markers = match &row.id {
                    Some(id) => row_markers(
                        drafts.held_by(repo, ItemDtoKind::Ticket, id),
                        conflicts.is_some_and(|conflicts| {
                            conflicts
                                .iter()
                                .any(|c| c.kind == ItemDtoKind::Ticket && c.item == *id)
                        }),
                        row.lifecycle == Some(Lifecycle::Closed),
                        closure,
                    ),
                    None => row_markers(Held::No, false, false, closure),
                };
                ShownTicket {
                    row: row.clone(),
                    markers,
                }
            })
            .collect()
    }

    /// Kit selection needs the frame's window, so the rows go to the list
    /// here, in every render, and only when they or their marks changed.
    pub(super) fn sync_tickets(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.tickets_repo_name().unwrap_or_default();
        self.tickets_pane.set_repository(&name, cx);
        let rows = self.shown_tickets(cx);
        self.tickets_pane.set(rows, window, cx);
    }
}
