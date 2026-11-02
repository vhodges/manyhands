//! Navigation-list read scheduling, filters and result application.

use super::Workspace;
use crate::desktop::{
    model::{
        list_state::ListState,
        navigation::Resolution,
        stamp::{RepoKey, Stamp},
        tickets::TicketRead,
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
        let stamp = self.tickets.load(key);
        self.read_tickets(stamp, resolved, clear, cx);
    }

    #[allow(clippy::result_large_err)]
    fn read_tickets(
        &mut self,
        stamp: Stamp,
        resolved: ResolvedRepository,
        clear: bool,
        cx: &mut Context<Self>,
    ) {
        let query = self.tickets.query();
        let (filter, names) = (query.to_filter(), query.names_filter());
        self.tickets_changed |= clear;
        self.ticket_read = Some(self.services.read(
            ReadKind::Tickets,
            stamp,
            move |service| {
                let list = service.list_tickets(&resolved, &filter)?;
                let names = match &names {
                    Some(names) => Some(service.list_tickets(&resolved, names)?),
                    None => None,
                };
                Ok(TicketRead { list, names })
            },
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

    pub(crate) fn set_closure(&mut self, closure: ClosureFilter, cx: &mut Context<Self>) {
        let stamp = self.tickets.set_closure(closure);
        match (stamp, self.resolved_selection()) {
            (Some(stamp), Some((_, resolved))) => self.read_tickets(stamp, resolved, true, cx),
            _ => cx.notify(),
        }
    }

    pub(crate) fn set_readiness(
        &mut self,
        readiness: Option<ReadinessFilter>,
        cx: &mut Context<Self>,
    ) {
        let stamp = self.tickets.set_readiness(readiness);
        match (stamp, self.resolved_selection()) {
            (Some(stamp), Some((_, resolved))) => self.read_tickets(stamp, resolved, true, cx),
            _ => cx.notify(),
        }
    }

    pub(in crate::desktop::view) fn open_selected_document(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.documents_pane.selected_target(cx) {
            self.open_item(target, cx);
        }
    }

    /// Kit selection needs the frame's window, so changed rows wait until here.
    pub(super) fn sync_tickets(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.tickets_changed) {
            let rows = self.tickets.state().rows().cloned().unwrap_or_default();
            self.tickets_pane.set(rows, window, cx);
        }
    }
}
