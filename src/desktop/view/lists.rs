//! Rendering the navigation lists; read transitions live in `workspace/lists`.
//! The ticket list opens with New ticket…, its create dialog
//! (`view::create`), as the document tree does with New document….

use super::{
    create::NEW_TICKET,
    sidebar,
    tickets::{CLOSURE_FILTERS, READINESS_FILTERS},
    workspace::Workspace,
};
use crate::desktop::model::{
    list_state::{ListState, Rows},
    navigation::Pane,
};
use gpui_kit::{
    App, ClickEvent, Context, FocusHandle, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{ActiveTheme, Selectable, button::Button, h_flex, v_flex},
    div,
};
use manyhands::repository::ReadinessFilter;

pub(crate) const SHOWING_DOCUMENTS: &str = "Showing documents";
pub(crate) const SHOWING_TICKETS: &str = "Showing tickets";
pub(crate) const PANES: [(&str, &str, Pane); 2] = [
    ("pane-documents", "Documents", Pane::Documents),
    ("pane-tickets", "Tickets", Pane::Tickets),
];

pub(super) fn choice_ids() -> impl Iterator<Item = &'static str> {
    PANES
        .iter()
        .map(|(id, ..)| *id)
        .chain(CLOSURE_FILTERS.iter().map(|(id, ..)| *id))
        .chain(READINESS_FILTERS.iter().map(|(id, ..)| *id))
}

impl Workspace {
    pub(super) fn navigation_pane(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let pane = self.navigation().pane();
        let choices = h_flex().gap_1().children(PANES.map(|(id, label, shown)| {
            let workspace = cx.entity().downgrade();
            choice(
                id,
                label,
                pane == shown,
                self.choice_focus(id),
                move |_, window, cx| {
                    let _ = workspace.update(cx, |this, cx| this.show_pane(shown, window, cx));
                },
            )
        }));
        let body = match pane {
            Pane::Documents => match self.documents().repo() {
                None => v_flex().child(div().text_sm().child(sidebar::NOTHING_SELECTED)),
                Some(_) => v_flex()
                    .gap_1()
                    .flex_1()
                    .min_h_0()
                    .child(list_status("documents-state", self.documents().state(), cx))
                    .child(self.documents_pane().render(cx.entity().downgrade(), cx)),
            },
            Pane::Tickets => {
                let closure = self.tickets().query().closure;
                let filters =
                    h_flex()
                        .gap_1()
                        .children(CLOSURE_FILTERS.map(|(id, label, _, filter)| {
                            let workspace = cx.entity().downgrade();
                            choice(
                                id,
                                label,
                                closure == filter,
                                self.choice_focus(id),
                                move |_, _, cx| {
                                    let _ = workspace
                                        .update(cx, |this, cx| this.set_closure(filter, cx));
                                },
                            )
                        }));
                let showing = CLOSURE_FILTERS
                    .iter()
                    .find(|(.., filter)| *filter == closure)
                    .map(|(_, _, showing, _)| *showing);
                let readiness = self.tickets().query().readiness;
                let readiness_filters =
                    h_flex()
                        .gap_1()
                        .children(READINESS_FILTERS.map(|(id, label, _, filter)| {
                            let workspace = cx.entity().downgrade();
                            choice(
                                id,
                                label,
                                readiness == filter,
                                self.choice_focus(id),
                                move |_, _, cx| {
                                    let _ = workspace
                                        .update(cx, |this, cx| this.set_readiness(filter, cx));
                                },
                            )
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
                // New ticket…, above the filters, where the Documents pane
                // has New document… (`03-panels.md`): its dialog
                // (`view::create`).
                let new_ticket = {
                    let workspace = cx.entity().downgrade();
                    h_flex().child(div().id("new-ticket").test_support().child(
                        Button::new("new-ticket-button").label(NEW_TICKET).on_click(
                            move |_, window, cx| {
                                let _ = workspace.update(cx, |this, cx| this.new_item(window, cx));
                            },
                        ),
                    ))
                };
                match self.tickets().repo() {
                    None => v_flex()
                        .gap_1()
                        .child(header)
                        .child(div().text_sm().child(sidebar::NOTHING_SELECTED)),
                    Some(_) => v_flex()
                        .gap_1()
                        .flex_1()
                        .min_h_0()
                        .child(new_ticket)
                        .child(header)
                        .child(list_status("tickets-state", self.tickets().state(), cx))
                        .child(self.tickets_pane().render())
                        .children(
                            (readiness == Some(ReadinessFilter::Blocked))
                                .then(|| self.tickets_pane().why_blocked(window, cx)),
                        ),
                }
            }
        };
        let showing = match pane {
            Pane::Documents => SHOWING_DOCUMENTS,
            Pane::Tickets => SHOWING_TICKETS,
        };
        v_flex()
            .id("navigation-pane")
            .test_support()
            .gap_2()
            .size_full()
            .child(self.find_bar(cx))
            .child(choices)
            .child(
                div()
                    .id("pane-showing")
                    .test_support()
                    .aria_label(showing)
                    .text_xs()
                    .child(showing),
            )
            .child(body)
    }
}

/// Choice state remains text as well as a selected accessibility signal.
fn choice(
    id: &'static str,
    label: &'static str,
    chosen: bool,
    focus: Option<&FocusHandle>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let mut wrapper = div().id(id).test_support();
    if let Some(focus) = focus {
        wrapper = wrapper.track_focus(focus);
    }
    wrapper
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
