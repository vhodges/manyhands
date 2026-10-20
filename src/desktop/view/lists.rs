//! The navigation region's lists: the selected repository's document tree
//! and ticket list, how they are read and read again, and the pane that
//! shows them.

use gpui_kit::{
    App, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{ActiveTheme, Selectable, button::Button, h_flex, v_flex},
    div,
};
use manyhands::repository::{ClosureFilter, ReadinessFilter, ResolvedRepository};

use super::{
    sidebar,
    tickets::{CLOSURE_FILTERS, READINESS_FILTERS},
    workspace::Workspace,
};
use crate::desktop::{
    model::{
        list_state::{ListState, Rows},
        navigation::{Pane, Resolution},
        stamp::{RepoKey, Stamp},
        tickets::TicketRead,
    },
    services::ReadKind,
};

pub(crate) const PANES: [(&str, &str, Pane); 2] = [
    ("pane-documents", "Documents", Pane::Documents),
    ("pane-tickets", "Tickets", Pane::Tickets),
];

impl Workspace {
    /// The selected repository, when it resolved.
    pub(super) fn resolved_selection(&self) -> Option<(RepoKey, ResolvedRepository)> {
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
        self.load_tickets(key, resolved, true, cx);
    }

    /// Reads the shown lists again after something changed them: the
    /// rows stay, and so does the selected row where it still exists,
    /// until the new reads land.
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

    /// No repository's lists are shown; reads still running are dropped.
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

    /// The folder and document reads run in parallel under one stamp.
    #[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
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
            // One of the two reads has landed: a reload keeps the rows,
            // and their selection, until the other does. A first load's
            // pane is already empty.
            (Some(_), ListState::Loading) => {}
            _ => self.documents_pane.clear(cx),
        }
        cx.notify();
    }

    #[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
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

    /// Reads the tickets under `stamp`. With `clear`, the list is emptied
    /// at once; without, its rows stay until the read lands.
    #[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
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
            // Under Blocked, the open tickets too, to name the blockers by.
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

    /// Lists `closure` from now on. A read under the last filter is
    /// dropped when it completes.
    pub(crate) fn set_closure(&mut self, closure: ClosureFilter, cx: &mut Context<Self>) {
        let stamp = self.tickets.set_closure(closure);
        match (stamp, self.resolved_selection()) {
            (Some(stamp), Some((_, resolved))) => self.read_tickets(stamp, resolved, true, cx),
            _ => cx.notify(),
        }
    }

    /// Lists the tickets of `readiness` from now on. A read under the last
    /// readiness is dropped when it completes.
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

    /// Enter in the document tree: opens the selected document.
    pub(super) fn open_selected_document(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.documents_pane.selected_target(cx) {
            self.open_item(target, cx);
        }
    }

    /// Gives the Kit's ticket list the rows read since the last frame,
    /// which has the window its selection needs.
    pub(super) fn sync_tickets(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.tickets_changed) {
            let rows = self.tickets.state().rows().cloned().unwrap_or_default();
            self.tickets_pane.set(rows, window, cx);
        }
    }

    /// The navigation region: the pane choice and the shown pane.
    pub(super) fn navigation_pane(&mut self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
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
                let readiness = self.tickets.query().readiness;
                let readiness_filters =
                    h_flex()
                        .gap_1()
                        .children(READINESS_FILTERS.map(|(id, label, _, filter)| {
                            let workspace = cx.entity().downgrade();
                            choice(id, label, readiness == filter, move |_, _, cx| {
                                let _ =
                                    workspace.update(cx, |this, cx| this.set_readiness(filter, cx));
                            })
                        }));
                let readiness_showing = READINESS_FILTERS
                    .iter()
                    .find(|(.., filter)| *filter == readiness)
                    .map(|(_, _, showing, _)| *showing);
                let showing_text = |id: &'static str, text: &'static str| {
                    div()
                        .id(id)
                        .test_support()
                        .aria_label(text)
                        .text_xs()
                        .child(text)
                };
                let header = v_flex()
                    .gap_1()
                    .child(filters)
                    .children(showing.map(|text| showing_text("tickets-showing", text)))
                    .child(readiness_filters)
                    .children(
                        readiness_showing
                            .map(|text| showing_text("tickets-readiness-showing", text)),
                    );
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
                        .child(self.tickets_pane.render())
                        .children(
                            (readiness == Some(ReadinessFilter::Blocked))
                                .then(|| self.tickets_pane.why_blocked(cx)),
                        ),
                }
            }
        };
        v_flex()
            .id("navigation-pane")
            .test_support()
            .gap_2()
            .size_full()
            .child(self.find_bar(cx))
            .child(choices)
            .child(body)
    }
}

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
