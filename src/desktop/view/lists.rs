//! Rendering the Tickets and Documents lists; read transitions live in
//! `workspace/lists`. The ticket list opens with the filter bar, its create
//! dialog (`view::create`), as the document tree does with New document….

use std::rc::Rc;

use super::{
    create::NEW_TICKET,
    documents::Unsaved,
    rows::{EmptyAction, alert_row, empty_state, skeleton_rows},
    workspace::{Region, Workspace, unless_dialog},
};
use crate::desktop::{
    keys::{SelectFirst, SelectLast, TICKET_LIST_CONTEXT},
    model::{
        documents::node_count,
        list_view::{
            BodyInput, ListAction, ListBody, ListKind, SELECT_A_REPOSITORY, list_body, loading_text,
        },
        markers::row_markers,
        navigation::Pane,
        ticket_view::footnote,
    },
    view::shell::tokens::chrome_colors,
};
use gpui_kit::{
    AnyElement, App, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{button::Button, h_flex, v_flex},
    div, px,
};
use manyhands::repository::{ClosureFilter, ItemDtoKind, ReadinessFilter};

/// The rows of a first load's skeleton.
const SKELETON_ROWS: usize = 6;

/// The ids of the elements a list's state is drawn as.
#[derive(Clone, Copy)]
struct StateIds {
    skeleton: &'static str,
    alert: &'static str,
    empty: &'static str,
}

const TICKET_STATE: StateIds = StateIds {
    skeleton: "tickets-skeleton",
    alert: "tickets-alert",
    empty: "tickets-empty",
};

const DOCUMENT_STATE: StateIds = StateIds {
    skeleton: "documents-skeleton",
    alert: "documents-alert",
    empty: "documents-empty",
};

impl Workspace {
    /// The Documents region's body (Part F): New document… and New folder…,
    /// the state of the tree, and the tree itself.
    ///
    /// The tree is drawn in every state while a repository is selected, so
    /// focus always has its element (R-34); the state, if any, is drawn
    /// above it.
    pub(super) fn documents_body(&mut self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let name = self.documents_repo_name();
        let list = self.documents().list();
        let body = list_body(
            list,
            BodyInput {
                kind: ListKind::Documents,
                repo: name.as_deref(),
                listed: list.rows().map_or(0, |rows| node_count(rows)),
                // Documents have no filter to narrow them.
                elsewhere: 0,
                closure: ClosureFilter::Open,
                narrowed: false,
            },
        );
        let state = self.list_state(ListKind::Documents, DOCUMENT_STATE, body, cx);
        let pane = v_flex()
            .id("documents-pane")
            .test_support()
            .gap_2()
            .size_full();
        let Some(repo) = self.documents().repo().cloned() else {
            return pane.child(state);
        };
        let drafts = self.services().drafts().clone();
        let unsaved: Unsaved = Rc::new(move |id, cx| {
            let held = drafts.read(cx).held_by(&repo, ItemDtoKind::Document, id);
            row_markers(held, false, false, ClosureFilter::Open).unsaved
        });
        let focus = self.region_focus(Region::Documents);
        pane.child(state).child(self.documents_pane().render(
            cx.entity().downgrade(),
            focus,
            unsaved,
            cx,
        ))
    }

    /// The Tickets region's body (Part C): the filter bar, what the ID read
    /// says, the hint, the column header, the state of the list,
    /// the list itself, Why blocked and the footnote.
    ///
    /// The list is drawn in every state while a repository is selected, so
    /// focus always has its element (R-34); the state, if any, is drawn
    /// above it.
    pub(super) fn tickets_body(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let colors = chrome_colors(cx);
        let name = self.tickets_repo_name();
        let view = self.tickets().view();
        let query = self.tickets().query();
        let body = list_body(
            self.tickets().list(),
            BodyInput {
                kind: ListKind::Tickets,
                repo: name.as_deref(),
                listed: view.listed,
                elsewhere: view.elsewhere,
                closure: query.closure,
                narrowed: query.narrowed(),
            },
        );
        let footnote = name
            .as_deref()
            .filter(|_| view.counts.is_some())
            .map(|name| footnote(view, query, name));
        let blocked = query.readiness == Some(ReadinessFilter::Blocked);
        let state = self.list_state(ListKind::Tickets, TICKET_STATE, body, cx);
        let pane = v_flex()
            .id("tickets-pane")
            .test_support()
            .size_full()
            .child(self.filter_bar(window, cx))
            .children(self.filter_status(cx));
        if name.is_none() {
            return pane.child(state);
        }
        // New ticket…, until the panel's toolbar replaces it, above the
        // filters where the Documents pane has New document….
        let new_ticket = {
            let workspace = cx.entity().downgrade();
            h_flex().child(div().id("new-ticket").test_support().child(
                Button::new("new-ticket-button").label(NEW_TICKET).on_click(
                    move |_, window, cx| {
                        let _ = workspace
                            .update(cx, |this, cx| this.new_item(Pane::Tickets, window, cx));
                    },
                ),
            ))
        };
        let list = self
            .tickets_pane()
            .render()
            .key_context(TICKET_LIST_CONTEXT)
            .on_action(unless_dialog(cx, |this, _: &SelectFirst, window, cx| {
                this.select_edge(false, window, cx)
            }))
            .on_action(unless_dialog(cx, |this, _: &SelectLast, window, cx| {
                this.select_edge(true, window, cx)
            }));
        pane.child(new_ticket)
            .children(self.tickets_hint(cx))
            .child(self.column_header(window, cx))
            .child(state)
            .child(list)
            .children(blocked.then(|| self.tickets_pane().why_blocked(window, cx)))
            .children(footnote.map(|text| {
                div()
                    .id("tickets-footnote")
                    .test_support()
                    .aria_label(text.clone())
                    .flex_none()
                    .w_full()
                    .px(px(10.))
                    .py(px(4.))
                    .border_t_1()
                    .border_color(colors.border)
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .text_size(px(11.))
                    .text_color(colors.muted)
                    .child(text)
            }))
    }

    /// What the body draws above a list: the message when no repository is
    /// selected, the skeleton of a first load, an alert row or an empty
    /// state, with the action each offers.
    fn list_state(
        &self,
        kind: ListKind,
        ids: StateIds,
        body: ListBody,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let act = move |action: ListAction, cx: &mut Context<Self>| {
            let workspace = cx.entity().downgrade();
            move |window: &mut Window, cx: &mut App| {
                let _ = workspace.update(cx, |this, cx| this.list_action(action, kind, window, cx));
            }
        };
        match body {
            ListBody::NoRepository => div()
                .px(px(12.))
                .py(px(8.))
                .text_size(px(13.))
                .child(SELECT_A_REPOSITORY)
                .into_any_element(),
            ListBody::Skeleton => {
                skeleton_rows(ids.skeleton, SKELETON_ROWS, loading_text(kind), cx)
                    .into_any_element()
            }
            ListBody::Rows { alert: None } => div().into_any_element(),
            ListBody::Rows { alert: Some(alert) } | ListBody::Problem(alert) => {
                let action = act(alert.action, cx);
                alert_row(ids.alert, &alert, action, cx).into_any_element()
            }
            ListBody::Empty { text, action } => {
                let action: Option<EmptyAction> = action.map(|action| {
                    let handler = act(action, cx);
                    (
                        action.label(kind).into(),
                        Box::new(handler) as Box<dyn Fn(&mut Window, &mut App)>,
                    )
                });
                empty_state(ids.empty, &text, action, cx).into_any_element()
            }
        }
    }
}
