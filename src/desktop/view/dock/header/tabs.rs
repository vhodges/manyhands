//! What a group's header draws (`02`, "Panel header"): a single panel's
//! icon, title and count; a tabbed group's tabs, the active one on the
//! `panel` background with the amber line; and the strip a closed bottom
//! dock leaves, one tab per panel (R-35).
//!
//! Tabs and titles are drag sources and drop targets through the group's
//! context, as the Kit's own skin wires them, with a preview of the app's
//! own (fact 9).

use std::sync::Arc;

use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Empty, FontWeight, InteractiveElement, IntoElement,
    ParentElement, Render, Role, SharedString, StatefulInteractiveElement, Styled, TestSupportExt,
    WeakEntity, Window,
    component::{
        ActiveTheme, Icon,
        dock::{BasePanelView, DragPanel, PanelHandle, TabGroupContext},
        h_flex,
    },
    div,
    prelude::FluentBuilder,
    px,
};

use crate::desktop::{
    assets::icon_path,
    model::layout::PanelKind,
    view::{
        shell::tokens::{ChromeColors, chrome_colors},
        workspace::Workspace,
    },
};

/// The header's height (`02`).
const HEIGHT: f32 = 32.;
const ICON_SIZE: f32 = 13.;
const TITLE_SIZE: f32 = 12.;
/// A group of more than this many tabs uses short labels (R-32).
const FULL_LABEL_TABS: usize = 2;
/// The size the drag preview occupies, reported so a drop placeholder knows
/// where to fly in from.
const PREVIEW_SIZE: gpui_kit::Size<gpui_kit::Pixels> = gpui_kit::size(px(96.), px(28.));

/// The panels of a group that are drawn, with their index in the group.
type Shown = Vec<(usize, Arc<dyn BasePanelView>)>;

/// The header of `group`: nothing when no panel is drawn or when the only
/// one draws its own chrome (the item area).
pub(super) fn header(
    group: &TabGroupContext,
    workspace: &WeakEntity<Workspace>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let shown: Shown = group
        .panels()
        .iter()
        .enumerate()
        .filter(|(_, panel)| panel.visible(cx))
        .map(|(ix, panel)| (ix, panel.clone()))
        .collect();
    if shown.is_empty() {
        return Empty.into_any_element();
    }
    if group.is_collapsed() {
        return strip(group, workspace, &shown, cx);
    }
    match shown.as_slice() {
        [(_, only)] if PanelHandle::of(only).is_some_and(|handle| !handle.title_bar(cx)) => {
            Empty.into_any_element()
        }
        [(ix, only)] => single(group, *ix, only, window, cx),
        _ => tabs(group, &shown, cx),
    }
}

/// The row every header is: `chrome`, 32 px, a 1 px border below.
fn row(cx: &App) -> gpui_kit::Div {
    let colors = chrome_colors(cx);
    h_flex()
        .flex_none()
        .w_full()
        .h(px(HEIGHT))
        .overflow_hidden()
        .bg(colors.chrome)
        .border_b_1()
        .border_color(colors.border)
}

fn kind_of(panel: &Arc<dyn BasePanelView>, cx: &App) -> Option<PanelKind> {
    PanelKind::from_name(panel.panel_name(cx))
}

/// The id a tab goes by: `panel-tab-repositories`, `-tickets`, `-documents`,
/// `-activity`.
fn tab_id(panel: &Arc<dyn BasePanelView>, cx: &App) -> SharedString {
    let slug = match kind_of(panel, cx) {
        Some(kind) => kind.label().to_lowercase(),
        None => panel.panel_name(cx).to_lowercase(),
    };
    format!("panel-tab-{slug}").into()
}

/// A tab's text: the short label when the group has more than two tabs.
fn tab_label(panel: &Arc<dyn BasePanelView>, tabs: usize, cx: &App) -> SharedString {
    match kind_of(panel, cx) {
        Some(kind) if tabs > FULL_LABEL_TABS => kind.short_label().into(),
        Some(kind) => kind.label().into(),
        None => panel.panel_name(cx).into(),
    }
}

/// One panel alone in its group: icon, title and count, with no tab
/// treatment.
fn single(
    group: &TabGroupContext,
    ix: usize,
    panel: &Arc<dyn BasePanelView>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let colors = chrome_colors(cx);
    let handle = PanelHandle::of(panel);
    let kind = kind_of(panel, cx);
    let icon = kind.map(|kind| {
        Icon::default()
            .path(icon_path(kind.icon()))
            .w(px(ICON_SIZE))
            .h(px(ICON_SIZE))
            .flex_none()
            .text_color(colors.secondary)
    });
    let title = handle.map(|handle| handle.title(window, cx));
    let count = handle.and_then(|handle| handle.title_suffix(window, cx));
    let label = tab_label(panel, FULL_LABEL_TABS, cx);
    let node = group.node().as_u64();
    let element = row(cx)
        .id(("panel-title", node))
        .test_support()
        .gap_2()
        .px(px(10.))
        .children(icon)
        .child(
            div()
                .text_size(px(TITLE_SIZE))
                .font_weight(FontWeight::MEDIUM)
                .text_color(colors.text)
                .whitespace_nowrap()
                .children(title),
        )
        .children(count);
    drag_source(element, group, ix, label, cx)
        .when(group.is_droppable(), |element| {
            element
                .drag_over::<DragPanel>(|style, _, _, cx| style.bg(cx.theme().tokens.drop_target))
                .on_drop({
                    let group = group.clone();
                    move |drag: &DragPanel, window, cx| {
                        group.drop_panel(drag.clone(), None, true, window, cx);
                    }
                })
        })
        .into_any_element()
}

/// One tab per drawn panel; the active one shows the amber line.
fn tabs(group: &TabGroupContext, shown: &Shown, cx: &mut App) -> AnyElement {
    let colors = chrome_colors(cx);
    let count = shown.len();
    let displayed = group.active_panel().map(|panel| panel.panel_id(cx));
    let bar = shown.iter().fold(row(cx), |bar, (ix, panel)| {
        let active = Some(panel.panel_id(cx)) == displayed;
        let label = tab_label(panel, count, cx);
        let tab = div()
            .id(tab_id(panel, cx))
            .test_support()
            .role(Role::Tab)
            .aria_label(label.clone())
            .aria_selected(active)
            .flex()
            .items_center()
            .h_full()
            .px(px(10.))
            .text_size(px(TITLE_SIZE))
            .whitespace_nowrap()
            .cursor_pointer()
            .border_b_2()
            .map_active(active, &colors)
            .on_click({
                let group = group.clone();
                let ix = *ix;
                move |_, window, cx| group.select_tab(ix, window, cx)
            })
            .child(label.clone());
        let tab = drag_source(tab, group, *ix, label, cx).when(group.is_droppable(), |tab| {
            tab.drag_over::<DragPanel>(|style, _, _, cx| {
                style
                    .rounded_l_none()
                    .border_l_2()
                    .border_r_0()
                    .border_color(cx.theme().drag_border)
            })
            .on_drop({
                let group = group.clone();
                let ix = *ix;
                move |drag: &DragPanel, window, cx| {
                    group.drop_panel(drag.clone(), Some(ix), true, window, cx);
                }
            })
        });
        bar.child(tab)
    });
    // The slot a drop addresses indexes the group's full panel list, hidden
    // panels included, not the tabs drawn: with [A, hidden, B], a panel
    // dropped past the last tab must land after B.
    bar.child(past_the_last_tab(group, group.panels().len()))
        .into_any_element()
}

/// Empty room after the last tab, so a panel can be moved past it.
fn past_the_last_tab(group: &TabGroupContext, tabs: usize) -> impl IntoElement {
    div()
        .id("panel-tabs-end")
        .test_support()
        .h_full()
        .flex_1()
        .min_w(px(16.))
        .when(group.is_droppable(), |end| {
            end.drag_over::<DragPanel>(|style, _, _, cx| style.bg(cx.theme().tokens.drop_target))
                .on_drop({
                    let group = group.clone();
                    let node = group.node();
                    move |drag: &DragPanel, window, cx| {
                        // A panel dropped past its own last tab lands in the
                        // final slot; one from elsewhere is appended in the
                        // background.
                        let ix = (drag.source() == node).then(|| tabs - 1);
                        group.drop_panel(drag.clone(), ix, false, window, cx);
                    }
                })
        })
}

/// A closed bottom dock's strip: every panel named, one tab each, none
/// active, and nothing to drag or drop on (R-35). A click reveals the panel
/// it names, which opens the dock, and focuses it, as a tab click does;
/// the Kit's own `select_tab` would focus a panel the collapsed group does
/// not draw.
fn strip(
    group: &TabGroupContext,
    workspace: &WeakEntity<Workspace>,
    shown: &Shown,
    cx: &mut App,
) -> AnyElement {
    let colors = chrome_colors(cx);
    let count = shown.len();
    // The strip is as tall as the Kit leaves it, which is under 32 px.
    row(cx)
        .h_full()
        .children(shown.iter().map(|(_, panel)| {
            let label = tab_label(panel, count, cx);
            div()
                .id(tab_id(panel, cx))
                .test_support()
                .role(Role::Tab)
                .aria_label(label.clone())
                .flex()
                .items_center()
                .h_full()
                .px(px(10.))
                .text_size(px(TITLE_SIZE))
                .whitespace_nowrap()
                .text_color(colors.secondary)
                .hover(|style| style.bg(colors.hover).text_color(colors.text))
                .cursor_pointer()
                .when_some(kind_of(panel, cx), |tab, kind| {
                    let workspace = workspace.clone();
                    tab.on_click(move |_, window, cx| {
                        _ = workspace.update(cx, |workspace, cx| {
                            workspace.reveal_and_focus(kind, window, cx)
                        });
                    })
                })
                .child(label)
        }))
        .id(("panel-strip", group.node().as_u64()))
        .test_support()
        .into_any_element()
}

/// Makes `element` a drag source when the group lets its panels be dragged.
fn drag_source<E>(
    element: E,
    group: &TabGroupContext,
    ix: usize,
    label: SharedString,
    cx: &App,
) -> E
where
    E: StatefulInteractiveElement,
{
    match group
        .is_draggable()
        .then(|| group.drag_panel(ix, cx))
        .flatten()
    {
        Some(drag) => element.on_drag(drag, move |drag, offset, _, cx| {
            cx.stop_propagation();
            drag.set_drag_offset(offset);
            drag.set_preview_size(PREVIEW_SIZE);
            let label = label.clone();
            cx.new(|_| DragPreview { label })
        }),
        None => element,
    }
}

/// Conditional styling that reads as the tab's states.
trait TabStyle: Styled + Sized {
    /// The active tab: `panel` background, `text` colour and the 2 px amber
    /// line; any other: `secondary` text over a line that is not drawn.
    fn map_active(self, active: bool, colors: &ChromeColors) -> Self {
        if active {
            self.bg(colors.panel)
                .text_color(colors.text)
                .border_color(colors.accent)
        } else {
            self.text_color(colors.secondary)
                .border_color(gpui_kit::transparent_black())
        }
    }
}

impl<E: Styled + Sized> TabStyle for E {}

/// What follows the cursor while a panel is dragged: its name in a small
/// box.
struct DragPreview {
    label: SharedString,
}

impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = chrome_colors(cx);
        div()
            .id("drag-panel")
            .cursor_grab()
            .w_24()
            .py_1()
            .px_3()
            .overflow_hidden()
            .whitespace_nowrap()
            .text_size(px(TITLE_SIZE))
            .text_color(colors.text)
            .bg(colors.panel)
            .border_1()
            .border_color(colors.border_strong)
            .rounded(cx.theme().radius)
            .opacity(0.85)
            .child(self.label.clone())
    }
}
