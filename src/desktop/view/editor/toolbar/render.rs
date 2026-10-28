//! How the toolbar is drawn (`item_view/02-editor-and-toolbar.md`): its
//! row, the segmented mode switch, the controls with their separators,
//! and the status item on the right. Sizes and colours are the handoff's.

use gpui_kit::{
    AnyElement, AnyView, App, ClickEvent, Context, Div, Hsla, InteractiveElement, IntoElement,
    Keystroke, MouseButton, ParentElement, Render, Role, Stateful, StatefulInteractiveElement,
    Styled, TestSupportExt, Window,
    accesskit::Toggled,
    component::{ActiveTheme, Icon, kbd::Kbd, tooltip::Tooltip},
    div, px, rgb,
};

use time::OffsetDateTime;

use super::{
    Step, Toolbar,
    controls::Control,
    popovers::{self, style_name},
};
use crate::desktop::{
    assets::icon_path,
    keys::{TOOLBAR_CONTEXT, ToggleRich, ToolbarFirst, ToolbarLast, ToolbarNext, ToolbarPrevious},
    model::{
        edit::{draft_indicator, indicator_words},
        markdown_edit::Mark,
    },
    view::item::tokens::{ItemColors, item_colors},
};

/// A control's height and least width.
pub(super) const BUTTON: f32 = 28.;
const ICON: f32 = 15.;
const CARET_ICON: f32 = 12.;
const RADIUS: f32 = 4.;
const TEXT_SIZE: f32 = 12.;
/// The gap between controls, and inside one.
const GAP: f32 = 2.;
const INNER_GAP: f32 = 4.;
/// The padding of a control drawn with words.
const TEXT_PADDING: f32 = 8.;
/// The toolbar's padding, vertical and horizontal.
const PADDING: (f32, f32) = (5., 20.);
/// A separator's width and height, and its margin each side.
const SEPARATOR: (f32, f32) = (1., 18.);
const SEPARATOR_MARGIN: f32 = 6.;
/// The mode switch's track: its radius, padding and right margin, and
/// its segments' padding.
const TRACK: (f32, f32, f32) = (5., 2., 8.);
const SEGMENT: (f32, f32) = (3., 9.);
/// How strongly Markdown mode draws a formatting control.
const DIM: f32 = 0.35;
/// The toolbar's icon and text colour, from the handoff; no token holds it.
const INK: u32 = 0x3b4047;
/// The line under the toolbar (contract 13): Tab stays in the editor, so
/// it says how to leave. The product owner's words, 2026-10-27; the dot is
/// U+00B7.
const HINT: &str = "Esc: Done \u{b7} F6: next region";

/// A control or popover as drawn: the Kit observes it for the test
/// shell's lookups in a test-support build, and is the element itself in
/// any other.
pub(super) type Observed = gpui_kit::base::ObservedElement<Stateful<Div>>;

impl Toolbar {
    /// Whether `control` is drawn pressed: the mode in use, or a mark the
    /// selection has. `None` for a control that does not toggle.
    fn toggled(&self, control: Control, rich: bool) -> Option<bool> {
        let mark = match control {
            Control::Rich => return Some(rich),
            Control::Markdown => return Some(!rich),
            Control::Bold => Mark::Bold,
            Control::Italic => Mark::Italic,
            Control::Strike => Mark::Strike,
            Control::Code => Mark::Code,
            _ => return None,
        };
        Some(self.active.marks.contains(&mark))
    }

    /// `control`'s element: focusable, named, with its tooltip, pressed or
    /// dimmed as it should be.
    fn control(
        &mut self,
        control: Control,
        rich: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Observed {
        let colors = item_colors(cx);
        let handle = self
            .handle(control)
            .tab_stop(self.is_tab_stop(control, window));
        let opacity = if control.formatting() && !rich {
            DIM
        } else {
            1.
        };
        #[cfg(test)]
        self.drawn.insert(control, opacity);
        let ring = if handle.is_focused(window) {
            cx.theme().ring
        } else {
            Hsla::transparent_black()
        };
        let hover = colors.toolbar_hover;
        let mut element = div()
            .id(control.id())
            .test_support()
            .track_focus(&handle)
            .role(Role::Button)
            .aria_label(control.label())
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .gap(px(INNER_GAP))
            .h(px(BUTTON))
            .min_w(px(BUTTON))
            .rounded(px(RADIUS))
            .border_2()
            .border_color(ring)
            .opacity(opacity)
            .hover(move |style| style.bg(hover))
            // A click leaves focus where it was, so the editor keeps it.
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.activate(control, window, cx)
            }))
            .tooltip(tooltip(control));
        let key = control.key().and_then(|key| Keystroke::parse(key).ok());
        if let Some(key) = &key {
            element = element.aria_keyshortcuts(Kbd::format(key));
        }
        if let Some(toggled) = self.toggled(control, rich) {
            element = element.aria_toggled(if toggled {
                Toggled::True
            } else {
                Toggled::False
            });
            if toggled {
                element = match control {
                    Control::Rich | Control::Markdown => element.bg(colors.page).shadow_sm(),
                    _ => element.bg(hover),
                };
            }
        }
        if let Some(icon) = control.icon() {
            element = element.child(
                Icon::default()
                    .path(icon_path(icon))
                    .w(px(ICON))
                    .h(px(ICON)),
            );
        }
        if let Some(text) = control.text() {
            element = element.px(px(TEXT_PADDING)).child(text);
        }
        element
    }

    /// Rich text | Markdown, as one segmented switch.
    fn mode_switch(&mut self, rich: bool, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let (radius, padding, margin) = TRACK;
        let (vertical, horizontal) = SEGMENT;
        let mut track = div()
            .flex()
            .flex_none()
            .items_center()
            .rounded(px(radius))
            .p(px(padding))
            .mr(px(margin))
            .bg(item_colors(cx).segmented);
        for control in [Control::Rich, Control::Markdown] {
            let segment = self
                .control(control, rich, window, cx)
                .h_auto()
                .py(px(vertical))
                .px(px(horizontal));
            track = track.child(segment);
        }
        track.into_any_element()
    }

    /// The Style control: the caret line's style by name, with its menu.
    fn style_control(&mut self, rich: bool, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let name = style_name(self.active.style);
        let open = self
            .popover
            .as_ref()
            .is_some_and(|popover| matches!(popover.kind, popovers::Kind::Style(_)));
        let button = self
            .control(Control::Style, rich, window, cx)
            .px(px(TEXT_PADDING))
            .aria_value(name.clone())
            .aria_expanded(open)
            .child(name)
            .child(
                Icon::default()
                    .path(icon_path("chevron-down"))
                    .w(px(CARET_ICON))
                    .h(px(CARET_ICON)),
            );
        let menu = self.popover_under(true, window, cx);
        div()
            .relative()
            .flex_none()
            .child(button)
            .children(menu)
            .into_any_element()
    }

    /// The status item: D2's state of the text in words, at this moment's
    /// age, or nothing before the first write. Its words are its name.
    fn status_item(&self, colors: &ItemColors) -> impl IntoElement + use<> {
        let words = draft_indicator(self.status, OffsetDateTime::now_utc()).map(indicator_words);
        let mut item = div()
            .id("toolbar-status")
            .test_support()
            .flex_none()
            .whitespace_nowrap()
            .text_color(colors.page_muted);
        if let Some(words) = words {
            item = item.aria_label(words.clone()).child(words);
        }
        item
    }

    /// The Link control, with its popover.
    fn link_control(&mut self, rich: bool, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let button = self.control(Control::Link, rich, window, cx);
        let popover = self.popover_under(false, window, cx);
        div()
            .relative()
            .flex_none()
            .child(button)
            .children(popover)
            .into_any_element()
    }
}

/// `control`'s tooltip: its name and, when it has one, its key.
fn tooltip(control: Control) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    move |window, cx| {
        let key = control.key().and_then(|key| Keystroke::parse(key).ok());
        Tooltip::new(control.label())
            .key_binding(key.map(Kbd::new))
            .build(window, cx)
    }
}

fn separator(colors: &ItemColors) -> Div {
    let (width, height) = SEPARATOR;
    div()
        .flex_none()
        .w(px(width))
        .h(px(height))
        .mx(px(SEPARATOR_MARGIN))
        .bg(colors.page_border)
}

impl Render for Toolbar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(test)]
        self.drawn.clear();
        self.keep_focus(window, cx);
        self.close_if_left(window, cx);
        let colors = item_colors(cx);
        let rich = self.host.read(cx).is_rich();
        let mut row =
            div()
                .id("toolbar")
                .key_context(TOOLBAR_CONTEXT)
                .role(Role::Toolbar)
                .aria_label("Formatting")
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(GAP))
                .py(px(PADDING.0))
                .px(px(PADDING.1))
                .bg(colors.page_fill)
                .border_b_1()
                .border_color(colors.page_border)
                .text_color(rgb(INK))
                .text_size(px(TEXT_SIZE))
                .on_action(cx.listener(|this, _: &ToolbarNext, window, cx| {
                    this.step(Step::Next, window, cx)
                }))
                .on_action(cx.listener(|this, _: &ToolbarPrevious, window, cx| {
                    this.step(Step::Previous, window, cx)
                }))
                .on_action(cx.listener(|this, _: &ToolbarFirst, window, cx| {
                    this.step(Step::First, window, cx)
                }))
                .on_action(cx.listener(|this, _: &ToolbarLast, window, cx| {
                    this.step(Step::Last, window, cx)
                }))
                .on_action(cx.listener(|this, _: &ToggleRich, _, cx| {
                    this.host
                        .update(cx, |host, cx| host.set_rich(!host.is_rich(), cx))
                }))
                .child(self.mode_switch(rich, window, cx));
        for control in self.visible().into_iter().skip(2) {
            if control.starts_group() {
                row = row.child(separator(&colors));
            }
            let element = match control {
                Control::Style => self.style_control(rich, window, cx),
                Control::Link => self.link_control(rich, window, cx),
                _ => self.control(control, rich, window, cx).into_any_element(),
            };
            row = row.child(element);
        }
        let row = row.child(div().flex_1()).child(self.status_item(&colors));
        div().flex().flex_col().child(row).child(hint(&colors))
    }
}

/// The hint, one muted line under the toolbar at its padding. It takes no
/// focus and no pointer events.
fn hint(colors: &ItemColors) -> impl IntoElement {
    let (vertical, horizontal) = PADDING;
    div()
        .id("toolbar-hint")
        .test_support()
        .aria_label(HINT)
        .py(px(vertical))
        .px(px(horizontal))
        .bg(colors.page)
        .text_size(px(TEXT_SIZE))
        .text_color(colors.page_muted)
        .child(HINT)
}
