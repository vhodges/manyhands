//! The row both list panels draw (Part A), and the elements that stand in
//! for rows: skeleton rows, an alert row and an empty state.
//!
//! `ListRow` implements the Kit's `Selectable`, so the Kit `List` uses it
//! as its item, and the tree draws it directly. It draws nothing of the
//! Kit's `ListItem`, whose selected background cannot be replaced. Colours
//! come from `chrome_colors` (fact 33).

use gpui_kit::{
    AnyElement, App, ClickEvent, Div, ElementId, FontWeight, InteractiveElement, IntoElement,
    ParentElement, Pixels, RenderOnce, SharedString, Stateful, StatefulInteractiveElement,
    StyleRefinement, Styled, TestSupportExt, Window,
    base::ObservedElement,
    component::{
        ActiveTheme, Icon, Selectable, Sizable, StyledExt,
        button::{Button, ButtonVariants},
        h_flex,
        tooltip::Tooltip,
        v_flex,
    },
    div,
    prelude::FluentBuilder,
    px, relative,
};

use crate::desktop::{
    assets::icon_path,
    model::list_view::{Alert, AlertTone, ListKind},
    view::shell::tokens::chrome_colors,
};

/// A row's height (`03`: comfortable).
pub(crate) const ROW_HEIGHT: Pixels = px(28.);

const TEXT_SIZE: f32 = 13.;
const DETAIL_SIZE: f32 = 12.;
const ICON_SIZE: f32 = 13.;
const LEFT_PADDING: f32 = 12.;
const RIGHT_PADDING: f32 = 10.;
/// The selected row's left line and focus ring.
const MARK_WIDTH: f32 = 2.;
/// The static bars of a skeleton row, as a share of the row's width.
const SKELETON_WIDTHS: [f32; 6] = [0.72, 0.58, 0.66, 0.48, 0.62, 0.54];

type Handler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// An empty state's button: its label and what it does.
pub(crate) type EmptyAction = (SharedString, Box<dyn Fn(&mut Window, &mut App)>);

/// One row of the Tickets list or the Documents tree.
#[derive(IntoElement)]
pub(crate) struct ListRow {
    base: ObservedElement<Stateful<Div>>,
    style: StyleRefinement,
    children: Vec<AnyElement>,
    selected: bool,
    focused: bool,
    expanded: Option<bool>,
    indent: Pixels,
    gap: Pixels,
    label: Option<SharedString>,
    description: Option<SharedString>,
    tooltip: Option<SharedString>,
    on_click: Option<Handler>,
}

impl ListRow {
    pub(crate) fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: h_flex().id(id).test_support(),
            style: StyleRefinement::default(),
            children: Vec::new(),
            selected: false,
            focused: false,
            expanded: None,
            indent: px(LEFT_PADDING),
            gap: px(8.),
            label: None,
            description: None,
            tooltip: None,
            on_click: None,
        }
    }

    /// The left padding, in place of the standard 12 px: the tree's
    /// indent for the row's depth.
    #[allow(dead_code)] // Used from Task 6 (the tree).
    pub(crate) fn indent(mut self, left: Pixels) -> Self {
        self.indent = left;
        self
    }

    /// The gap between the row's cells: 8 in Tickets, 5 in Documents.
    #[allow(dead_code)] // Used from Task 6 (the tree).
    pub(crate) fn gap(mut self, gap: Pixels) -> Self {
        self.gap = gap;
        self
    }

    /// Whether the row's list has focus, so a selected row draws its ring.
    pub(crate) fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// A tree folder's `aria_expanded`; `None` for a leaf.
    #[allow(dead_code)] // Used from Task 6 (the tree).
    pub(crate) fn expanded(mut self, expanded: Option<bool>) -> Self {
        self.expanded = expanded;
        self
    }

    /// The accessible name.
    pub(crate) fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub(crate) fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub(crate) fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    #[allow(dead_code)] // Used from Task 6 (the tree).
    pub(crate) fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl Styled for ListRow {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for ListRow {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Selectable for ListRow {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl RenderOnce for ListRow {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = chrome_colors(cx);
        let selected = self.selected;
        let tooltip = self.tooltip;
        self.base
            .relative()
            .flex_none()
            .w_full()
            .h(ROW_HEIGHT)
            .pl(self.indent)
            .pr(px(RIGHT_PADDING))
            .gap(self.gap)
            .items_center()
            .text_size(px(TEXT_SIZE))
            .text_color(colors.text)
            .refine_style(&self.style)
            .aria_selected(selected)
            .when_some(self.label, |this, label| this.aria_label(label))
            .when_some(self.description, |this, text| this.aria_description(text))
            .when_some(self.expanded, |this, expanded| this.aria_expanded(expanded))
            .when_some(self.on_click, |this, on_click| this.on_click(on_click))
            .when_some(tooltip, |this, text| {
                this.tooltip(move |window, cx| Tooltip::new(text.clone()).build(window, cx))
            })
            .when(selected, |this| {
                this.bg(colors.selection).font_weight(FontWeight::SEMIBOLD)
            })
            .when(!selected, |this| {
                this.hover(move |style| style.bg(colors.hover))
            })
            .children(self.children)
            .when(selected, |this| {
                this.child(
                    div()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left_0()
                        .w(px(MARK_WIDTH))
                        .bg(colors.accent),
                )
            })
            .when(selected && self.focused, |this| {
                this.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .border(px(MARK_WIDTH))
                        .border_color(colors.teal),
                )
            })
    }
}

/// `count` static bars where a first load's rows will be: plain bars in
/// `theme.skeleton`, not the Kit's `Skeleton`, which pulses (R-6). The
/// root carries `label` as its accessible name; the caller passes
/// `loading_text(kind)` ("Loading tickets"). The `label` parameter is a
/// deliberate small deviation from the plan's signature (Task 3 is the
/// only caller).
pub(crate) fn skeleton_rows(
    id: impl Into<ElementId>,
    count: usize,
    label: impl Into<SharedString>,
    cx: &App,
) -> impl IntoElement {
    let bar = cx.theme().skeleton;
    v_flex()
        .id(id)
        .test_support()
        .aria_label(label)
        .w_full()
        .children((0..count).map(move |ix| {
            h_flex()
                .w_full()
                .h(ROW_HEIGHT)
                .pl(px(LEFT_PADDING))
                .pr(px(RIGHT_PADDING))
                .gap(px(8.))
                .items_center()
                .child(div().flex_none().w(px(14.)).h(px(14.)).rounded_sm().bg(bar))
                .child(
                    div()
                        .h(px(10.))
                        .w(relative(SKELETON_WIDTHS[ix % SKELETON_WIDTHS.len()]))
                        .rounded_sm()
                        .bg(bar),
                )
        }))
}

/// The alert row above a list's rows, or alone: an icon, the text, a
/// failure's message on a second visible line, and one action button.
pub(crate) fn alert_row(
    id: impl Into<ElementId>,
    alert: &Alert,
    on_action: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let id: ElementId = id.into();
    let colors = chrome_colors(cx);
    let (icon, color) = match alert.tone {
        AlertTone::Warning => ("triangle-alert", colors.accent),
        AlertTone::Failure => ("circle-x", colors.red),
    };
    // An alert's action is never New item, so the panel does not matter.
    let action = alert.action.label(ListKind::Tickets);
    h_flex()
        .id(id.clone())
        .test_support()
        .aria_label(alert.text.clone())
        .w_full()
        .min_h(ROW_HEIGHT)
        .py_1()
        .pl(px(LEFT_PADDING))
        .pr(px(RIGHT_PADDING))
        .gap(px(8.))
        .items_start()
        .text_size(px(TEXT_SIZE))
        .child(
            div()
                .flex_none()
                .h(ROW_HEIGHT - px(8.))
                .flex()
                .items_center()
                .child(
                    Icon::default()
                        .path(icon_path(icon))
                        .w(px(ICON_SIZE))
                        .h(px(ICON_SIZE))
                        .text_color(color),
                ),
        )
        .child(
            v_flex()
                .flex_1()
                .min_w(px(0.))
                .child(div().text_color(colors.text).child(alert.text.clone()))
                .children(alert.detail.clone().map(|detail| {
                    div()
                        .id((id.clone(), "detail"))
                        .test_support()
                        .aria_label(detail.clone())
                        .text_size(px(DETAIL_SIZE))
                        .text_color(colors.secondary)
                        .child(detail)
                })),
        )
        .child(
            Button::new((id, "action"))
                .small()
                .ghost()
                .label(action)
                .on_click(move |_, window, cx| on_action(window, cx)),
        )
}

/// A list with nothing to show: the text, and the action when there is
/// one, as its label and handler.
pub(crate) fn empty_state(
    id: impl Into<ElementId>,
    text: &str,
    action: Option<EmptyAction>,
    cx: &App,
) -> impl IntoElement {
    let id: ElementId = id.into();
    let colors = chrome_colors(cx);
    v_flex()
        .id(id.clone())
        .test_support()
        .aria_label(text.to_string())
        .w_full()
        .items_center()
        .gap_2()
        .px(px(LEFT_PADDING))
        .py_4()
        .text_size(px(TEXT_SIZE))
        .text_color(colors.secondary)
        .child(div().child(text.to_string()))
        .children(action.map(|(label, on_action)| {
            Button::new((id, "action"))
                .small()
                .label(label)
                .on_click(move |_, window, cx| on_action(window, cx))
        }))
}
