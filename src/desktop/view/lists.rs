//! Rendering the Tickets and Documents lists; read transitions live in
//! `workspace/lists`. The ticket list opens with the filter bar, its create
//! dialog (`view::create`), as the document tree does with New document….

use super::{
    create::NEW_TICKET,
    rows::{EmptyAction, alert_row, empty_state, skeleton_rows},
    sidebar,
    workspace::{Workspace, unless_dialog},
};
use crate::desktop::{
    keys::{SelectFirst, SelectLast, TICKET_LIST_CONTEXT},
    model::{
        list_state::{ListState, Rows},
        list_view::{
            BodyInput, ListAction, ListBody, ListKind, SELECT_A_REPOSITORY, list_body, loading_text,
        },
        navigation::Pane,
        ticket_view::footnote,
    },
    view::shell::tokens::chrome_colors,
};
use gpui_kit::{
    AnyElement, App, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{ActiveTheme, button::Button, h_flex, v_flex},
    div, px,
};
use manyhands::repository::ReadinessFilter;

/// The rows of a first load's skeleton.
const SKELETON_ROWS: usize = 6;

impl Workspace {
    /// The Documents region's body: New document…, New folder… and the tree.
    pub(super) fn documents_body(&mut self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let body = match self.documents().repo() {
            None => v_flex().child(div().text_sm().child(sidebar::NOTHING_SELECTED)),
            Some(_) => v_flex()
                .gap_1()
                .flex_1()
                .min_h_0()
                .child(list_status("documents-state", self.documents().state(), cx))
                .child(self.documents_pane().render(cx.entity().downgrade(), cx)),
        };
        v_flex()
            .id("documents-pane")
            .test_support()
            .gap_2()
            .size_full()
            .child(body)
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
        let state = self.tickets_state(body, cx);
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

    /// What the body draws above the Tickets list: the message when no
    /// repository is selected, the skeleton of a first load, an alert row
    /// or an empty state, with the action each offers.
    fn tickets_state(&self, body: ListBody, cx: &mut Context<Self>) -> AnyElement {
        let act = |action: ListAction, cx: &mut Context<Self>| {
            let workspace = cx.entity().downgrade();
            move |window: &mut Window, cx: &mut App| {
                let _ = workspace.update(cx, |this, cx| this.list_action(action, window, cx));
            }
        };
        match body {
            ListBody::NoRepository => div()
                .px(px(12.))
                .py(px(8.))
                .text_size(px(13.))
                .child(SELECT_A_REPOSITORY)
                .into_any_element(),
            ListBody::Skeleton => skeleton_rows(
                "tickets-skeleton",
                SKELETON_ROWS,
                loading_text(ListKind::Tickets),
                cx,
            )
            .into_any_element(),
            ListBody::Rows { alert: None } => div().into_any_element(),
            ListBody::Rows { alert: Some(alert) } | ListBody::Problem(alert) => {
                let action = act(alert.action, cx);
                alert_row("tickets-alert", &alert, action, cx).into_any_element()
            }
            ListBody::Empty { text, action } => {
                let action: Option<EmptyAction> = action.map(|action| {
                    let handler = act(action, cx);
                    (
                        action.label(ListKind::Tickets).into(),
                        Box::new(handler) as Box<dyn Fn(&mut Window, &mut App)>,
                    )
                });
                empty_state("tickets-empty", &text, action, cx).into_any_element()
            }
        }
    }
}

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
