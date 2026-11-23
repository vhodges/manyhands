//! The Tickets panel's filter bar (Part C): the lifecycle segments, the
//! readiness menu and the filter input, and under it the hint that other
//! filters hold more matches.
//!
//! The segments are the app's own, restyled from the choice controls they
//! replace: the Kit's segmented controls have no keyboard path (R-4). Each
//! is a tab stop that Enter or Space activates, with `Role::Tab`,
//! `aria_selected` and a name that says its count. The readiness menu is a
//! button with a dropdown menu (R-5); its name says the choice, since the
//! Kit does not expose the checked item.

use gpui_kit::{
    App, InteractiveElement, IntoElement, ParentElement, Role, SharedString,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, Icon, Sizable,
        button::{Button, ButtonVariants},
        h_flex,
        input::Input,
        menu::{DropdownMenu, PopupMenuItem},
    },
    div,
    prelude::FluentBuilder,
    px,
};
use manyhands::repository::{ClosureFilter, ReadinessFilter};

use super::columns::SORT_IDS;
use crate::desktop::{
    assets::icon_path,
    model::{
        list_view::SHOW_ALL,
        ticket_view::{SegmentCounts, hint},
        tickets::TicketQuery,
    },
    view::{shell::tokens::chrome_colors, workspace::Workspace},
};

pub(crate) const FILTER_PLACEHOLDER: &str = "Filter by title or code";
/// The readiness button's tooltip.
pub(crate) const READINESS_TOOLTIP: &str = "Show ready or blocked tickets";
/// The readiness choice that asks for none.
pub(crate) const ANY_READINESS: &str = "Any readiness";

/// The lifecycle segments, in the order they are offered: their element
/// id, label and the closure they show.
const SEGMENTS: [(&str, &str, ClosureFilter); 3] = [
    ("filter-open", "Open", ClosureFilter::Open),
    ("filter-closed", "Closed", ClosureFilter::Closed),
    ("filter-all", "All", ClosureFilter::All),
];

/// The readiness menu's items, in order: label and the readiness chosen.
const READINESS_ITEMS: [(&str, Option<ReadinessFilter>); 3] = [
    (ANY_READINESS, None),
    ("Ready", Some(ReadinessFilter::Ready)),
    ("Blocked", Some(ReadinessFilter::Blocked)),
];

/// The ids of the controls that are tab stops of their own, which the
/// Workspace gives a focus handle each: the segments and the sort headers.
pub(crate) fn choice_ids() -> impl Iterator<Item = &'static str> {
    SEGMENTS
        .iter()
        .map(|(id, ..)| *id)
        .chain(SORT_IDS.iter().copied())
}

/// A segment's accessible name: its label, and its count when it has one:
/// "Open, 8 tickets", "Open, 1 ticket".
pub(crate) fn segment_name(label: &str, count: Option<usize>) -> String {
    match count {
        Some(1) => format!("{label}, 1 ticket"),
        Some(count) => format!("{label}, {count} tickets"),
        None => label.to_owned(),
    }
}

/// The readiness button's accessible name: "Readiness: any", "Readiness:
/// ready" or "Readiness: blocked".
pub(crate) fn readiness_name(readiness: Option<ReadinessFilter>) -> &'static str {
    match readiness {
        None => "Readiness: any",
        Some(ReadinessFilter::Ready) => "Readiness: ready",
        Some(ReadinessFilter::Blocked) => "Readiness: blocked",
    }
}

/// The segment's count among the counts, when there are any.
fn segment_count(counts: Option<SegmentCounts>, filter: ClosureFilter) -> Option<usize> {
    counts.map(|counts| match filter {
        ClosureFilter::Open => counts.open,
        ClosureFilter::Closed => counts.closed,
        ClosureFilter::All => counts.all,
    })
}

impl Workspace {
    /// The filter bar: segments, the readiness button and the filter
    /// input, drawn whether or not a repository is selected.
    pub(in crate::desktop::view) fn filter_bar(
        &self,
        window: &Window,
        cx: &mut gpui_kit::Context<Self>,
    ) -> impl IntoElement + use<> {
        let colors = chrome_colors(cx);
        let query = self.tickets().query().clone();
        let counts = self.tickets().view().counts;
        let segments = h_flex()
            .flex_none()
            .rounded(px(5.))
            .border_1()
            .border_color(colors.border_strong)
            .overflow_hidden()
            .children(SEGMENTS.map(|(id, label, filter)| {
                let workspace = cx.entity().downgrade();
                let count = segment_count(counts, filter);
                let focus = self.choice_focus(id);
                let focused = focus.is_some_and(|focus| focus.is_focused(window));
                let chosen = query.closure == filter;
                let mut segment = div().id(id).test_support();
                if let Some(focus) = focus {
                    segment = segment.track_focus(focus);
                }
                segment
                    .role(Role::Tab)
                    .aria_label(segment_name(label, count))
                    .aria_selected(chosen)
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .h(px(22.))
                    .px(px(9.))
                    .text_size(px(12.))
                    .cursor_pointer()
                    .when(chosen, |segment| {
                        segment.bg(colors.border).text_color(colors.text)
                    })
                    .when(!chosen, |segment| {
                        segment
                            .text_color(colors.secondary)
                            .hover(|style| style.bg(colors.hover).text_color(colors.text))
                    })
                    .on_click(move |_, _, cx| {
                        let _ = workspace.update(cx, |this, cx| this.set_closure(filter, cx));
                    })
                    .child(label)
                    .children(
                        count.map(|count| div().text_color(colors.muted).child(count.to_string())),
                    )
                    .when(focused, |segment| segment.child(inset_ring(cx)))
            }));
        let input = div()
            .id("find-input")
            .test_support()
            .aria_label(FILTER_PLACEHOLDER)
            .flex_1()
            .min_w(px(80.))
            .child(
                Input::new(self.find_input())
                    .small()
                    .cleanable(true)
                    .aria_label(FILTER_PLACEHOLDER)
                    .prefix(
                        Icon::default()
                            .path(icon_path("list-filter"))
                            .w(px(13.))
                            .h(px(13.))
                            .text_color(colors.muted),
                    ),
            );
        h_flex()
            .id("tickets-filter-bar")
            .flex_none()
            .w_full()
            .gap(px(6.))
            .px(px(8.))
            .py(px(6.))
            .items_center()
            .border_b_1()
            .border_color(colors.border)
            .child(segments)
            .child(self.readiness_menu(&query, cx))
            .child(input)
    }

    /// The readiness button: an icon, and the word when Ready or Blocked is
    /// chosen, with a menu of the three choices. Focus comes back to the
    /// button when the menu closes.
    fn readiness_menu(
        &self,
        query: &TicketQuery,
        cx: &mut gpui_kit::Context<Self>,
    ) -> impl IntoElement + use<> {
        let readiness = query.readiness;
        let word = READINESS_ITEMS
            .iter()
            .find(|(_, choice)| choice.is_some() && *choice == readiness)
            .map(|(label, _)| *label);
        let workspace = cx.entity().downgrade();
        Button::new("filter-readiness")
            .small()
            .ghost()
            .icon(Icon::default().path(icon_path("sliders-horizontal")))
            .when_some(word, |button, word| button.label(word))
            .accessibility_label(readiness_name(readiness))
            .tooltip(READINESS_TOOLTIP)
            .dropdown_menu(move |menu, _, _| {
                READINESS_ITEMS.iter().fold(menu, |menu, (label, choice)| {
                    let workspace = workspace.clone();
                    let choice = *choice;
                    menu.item(
                        PopupMenuItem::new(*label)
                            .checked(choice == readiness)
                            .on_click(move |_, _, cx| {
                                let _ =
                                    workspace.update(cx, |this, cx| this.set_readiness(choice, cx));
                            }),
                    )
                })
            })
    }

    /// The hint under the filter bar when other filters hold more matches,
    /// with Show all (decision 2). Nothing when there is none.
    pub(in crate::desktop::view) fn tickets_hint(
        &self,
        cx: &mut gpui_kit::Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        let text: SharedString = hint(self.tickets().view())?.into();
        let colors = chrome_colors(cx);
        let workspace = cx.entity().downgrade();
        Some(
            h_flex()
                .id("tickets-hint")
                .test_support()
                .aria_label(text.clone())
                .flex_none()
                .w_full()
                .gap(px(8.))
                .px(px(10.))
                .py(px(3.))
                .items_center()
                .border_b_1()
                .border_color(colors.border)
                .text_size(px(12.))
                .text_color(colors.secondary)
                .child(div().flex_1().min_w(px(0.)).child(text))
                .child(
                    Button::new("tickets-show-all")
                        .small()
                        .ghost()
                        .label(SHOW_ALL)
                        .on_click(move |_, window, cx| {
                            let _ = workspace.update(cx, |this, cx| this.show_all(window, cx));
                        }),
                ),
        )
    }
}

/// The app's focus ring, inside the control that holds focus.
pub(super) fn inset_ring(cx: &App) -> impl IntoElement + use<> {
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .border_2()
        .border_color(cx.theme().ring)
}
