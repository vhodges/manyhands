//! Navigation-list read scheduling, filters and result application.

use std::path::PathBuf;

use super::{Region, Workspace};
use crate::desktop::{
    model::{
        list_state::ListState,
        navigation::Resolution,
        stamp::{RepoKey, Stamp},
        tickets::{TicketQuery, read_filter},
    },
    services::ReadKind,
};
use gpui_kit::{Context, Window};
use manyhands::repository::{ClosureFilter, ReadinessFilter, ResolvedRepository};

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
        self.load_documents(key.clone(), resolved.clone(), cx);
        self.load_tickets(key, resolved, true, cx);
    }

    /// Reload retains rows and selection until their replacement arrives.
    pub(super) fn reload_lists(&mut self, cx: &mut Context<Self>) {
        let Some((key, resolved)) = self.resolved_selection() else {
            return;
        };
        if self.documents.repo() != Some(&key) {
            return self.load_lists(cx);
        }
        self.load_documents(key.clone(), resolved.clone(), cx);
        self.load_tickets(key, resolved, false, cx);
    }

    pub(super) fn clear_lists(&mut self, cx: &mut Context<Self>) {
        self.documents.clear();
        self.documents_pane.clear(cx);
        self.folder_read = None;
        self.document_read = None;
        self.tickets.clear();
        self.ticket_read = None;
        self.tickets_changed = true;
        self.clear_find(cx);
        cx.notify();
    }

    /// Folder and document reads run in parallel under one stamp.
    #[allow(clippy::result_large_err)]
    fn load_documents(
        &mut self,
        key: RepoKey,
        resolved: ResolvedRepository,
        cx: &mut Context<Self>,
    ) {
        let stamp = self.documents.load(key);
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
            // One read landed: keep the previous rows until the other does.
            (Some(_), ListState::Loading) => {}
            _ => self.documents_pane.clear(cx),
        }
        cx.notify();
    }

    #[allow(clippy::result_large_err)]
    fn load_tickets(
        &mut self,
        key: RepoKey,
        resolved: ResolvedRepository,
        clear: bool,
        cx: &mut Context<Self>,
    ) {
        let (stamp, _) = self.tickets.load(key);
        self.read_tickets(stamp, resolved, clear, cx);
    }

    /// The one read of the load: every ticket, whatever the query (R-1).
    #[allow(clippy::result_large_err)]
    fn read_tickets(
        &mut self,
        stamp: Stamp,
        resolved: ResolvedRepository,
        clear: bool,
        cx: &mut Context<Self>,
    ) {
        let filter = read_filter();
        self.tickets_changed |= clear;
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

    /// Lists `query` from now on. Nothing is read: the rows are derived
    /// from the read the model holds.
    fn show_tickets(&mut self, query: TicketQuery, cx: &mut Context<Self>) {
        self.tickets.set_query(query);
        self.tickets_changed = true;
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

    /// Lists `query`, typed text included.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn set_ticket_query(&mut self, query: TicketQuery, cx: &mut Context<Self>) {
        self.show_tickets(query, cx);
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
        let mut query = self
            .tickets
            .query()
            .clone()
            .with_closure(ClosureFilter::Open)
            .with_readiness(Some(ReadinessFilter::Ready));
        query.text.clear();
        self.show_tickets(query, cx);
        if self.navigation.selected() == Some(&key) {
            if let Some((key, resolved)) = self.resolved_selection() {
                self.load_tickets(key, resolved, false, cx);
            }
        } else {
            // Selecting clears the list first and reads it under the query.
            self.select_repository(key, cx);
        }
        self.focus_region(Region::Tickets, window, cx);
    }

    pub(in crate::desktop::view) fn open_selected_document(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.documents_pane.selected_target(cx) {
            self.open_item(target, cx);
        }
    }

    /// Kit selection needs the frame's window, so changed rows wait until here.
    pub(super) fn sync_tickets(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.tickets_changed) {
            let mut rows = self.tickets.view().rows.clone();
            // The model gives every blocked row its reasons; the panel
            // shows them only under Blocked.
            if self.tickets.query().readiness != Some(ReadinessFilter::Blocked) {
                for row in &mut rows {
                    row.reasons.clear();
                }
            }
            self.tickets_pane.set(rows, window, cx);
        }
    }
}
