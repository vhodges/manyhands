//! The filter input's actions and the stamped ID read, owned by the
//! workspace (design, Part D).

use super::Workspace;
use crate::desktop::{
    model::{
        find::{FindState, parse_id},
        layout::PanelKind,
        stamp::RepoKey,
    },
    services::ReadKind,
};
use gpui_kit::{Context, Entity, Window, component::input::InputState};

impl Workspace {
    pub(crate) fn find_input(&self) -> &Entity<InputState> {
        &self.find_input
    }
    pub(in crate::desktop::view) fn find_state(&self) -> &FindState {
        self.find.state()
    }

    /// The display name of the repository the ID read is in, for what the
    /// status line says of it.
    pub(in crate::desktop::view) fn find_repo_name(&self) -> Option<String> {
        self.find.repo().map(|key| self.repo_name(key))
    }

    /// A repository's display name, as the navigation names it.
    pub(super) fn repo_name(&self, key: &RepoKey) -> String {
        self.navigation
            .entry(key)
            .map_or_else(|| key.clone(), |entry| entry.name())
    }

    pub(super) fn focus_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // The input is in the Tickets panel, which may be hidden (R-10).
        self.reveal(PanelKind::Tickets, window, cx);
        self.find_input
            .update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    /// Enter in the filter input, after the pane is brought up to date with
    /// the query (R-32):
    ///
    /// - text that parses as a ULID opens that item by the stamped
    ///   `show_item` read, whatever the filters; focus stays in the input;
    /// - otherwise, one conforming ticket matching the text under any
    ///   lifecycle and readiness opens, even if a filter hides it; focus
    ///   stays in the input;
    /// - otherwise, with rows listed, focus moves to the list with its
    ///   first row selected, and with none nothing happens.
    pub(in crate::desktop::view) fn submit_filter(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sync_tickets(window, cx);
        let text = self.find_input.read(cx).value().to_string();
        if text.trim().is_empty() {
            self.clear_find(cx);
            return self.focus_first_row(window, cx);
        }
        let Some((key, resolved)) = self.resolved_selection() else {
            self.find.no_repository();
            self.find_read = None;
            cx.notify();
            return;
        };
        if let Some(id) = parse_id(&text) {
            return self.read_id(key, resolved, id, cx);
        }
        // Not an ID: an ID's answer from before is stale.
        self.clear_find(cx);
        match self.tickets.sole_match() {
            Some(row) => {
                // Defensive: `sole_match` takes only conforming tickets, so a
                // row that does not open (R-38) cannot reach here today.
                if let Some(target) = row.open_target(&key) {
                    self.open_item(target, cx);
                }
            }
            None => self.focus_first_row(window, cx),
        }
    }

    #[allow(clippy::result_large_err)]
    fn read_id(
        &mut self,
        key: RepoKey,
        resolved: manyhands::repository::ResolvedRepository,
        id: manyhands::canonical::ItemId,
        cx: &mut Context<Self>,
    ) {
        let stamp = self.find.start(key);
        self.find_read = Some(self.services.read(
            ReadKind::Item,
            stamp,
            move |service| service.show_item(&resolved, &id),
            cx,
            |this: &mut Self, stamp, result, cx| {
                if let Some(target) = this.find.apply_item(stamp, result) {
                    this.open_item(target, cx);
                }
                cx.notify();
            },
        ));
        cx.notify();
    }

    /// Focus in the list, its first row selected, when rows are listed.
    fn focus_first_row(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tickets.view().listed == 0 {
            return;
        }
        self.tickets_pane.select_edge(false, window, cx);
        self.tickets_pane.focus(window, cx);
        cx.notify();
    }

    /// The ID read's state goes, and a read still running is dropped.
    pub(super) fn clear_find(&mut self, cx: &mut Context<Self>) {
        self.find.clear();
        self.find_read = None;
        cx.notify();
    }
}
