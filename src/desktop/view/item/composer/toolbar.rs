//! The composer's mini toolbar (design, Lane C; `item_view/04-comments.md`,
//! Reply composer): "Replying to {name}" on a reply, then Bold, Italic,
//! Inline code, Link and Bulleted list, the Rich text / Markdown switch,
//! and on the right the draft indicator.
//!
//! - The marks are the editor's own actions, dispatched through its focus
//!   handle; Bulleted list and Link are one `replace_range` each, as the
//!   edit toolbar's host edits are. A click leaves focus in the editor.
//! - Link opens an address field under the toolbar, focused; Enter makes
//!   the selection a link (`insert_link`), Escape closes it, and either
//!   gives focus back to where it was. Focus leaving it closes it too.
//! - Left, Right, Home and End move along the controls, which are one tab
//!   stop that roves, as the edit toolbar's are (its key context binds
//!   them). The mode key works from a control.
//! - In Markdown the formatting controls are drawn at 35% and keep
//!   working.

use gpui_kit::{
    Action, AnyElement, App, ClickEvent, Context, EntityInputHandler, FocusHandle, Focusable, Hsla,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Role, StatefulInteractiveElement,
    Styled, TestSupportExt, Window,
    accesskit::Toggled,
    component::{ActiveTheme, Icon},
    div, px,
};
use zorite_editor::{Bold, Code, Italic};

use super::{Composer, ComposerKind};
use crate::desktop::{
    assets::icon_path,
    keys::{TOOLBAR_CONTEXT, ToggleRich, ToolbarFirst, ToolbarLast, ToolbarNext, ToolbarPrevious},
    model::{
        drafts::RecoveryState,
        markdown_edit::{HostEdit, LinePrefix, Selection, selection_from_utf16, toggle_prefix},
    },
    view::item::tokens::item_colors,
};

/// One control of the mini toolbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MiniControl {
    Bold,
    Italic,
    Code,
    Link,
    Bullets,
    Rich,
    Markdown,
}

/// How many controls there are: one focus handle each.
pub(crate) const COUNT: usize = 7;

/// The controls, left to right.
pub(crate) const CONTROLS: [MiniControl; COUNT] = [
    MiniControl::Bold,
    MiniControl::Italic,
    MiniControl::Code,
    MiniControl::Link,
    MiniControl::Bullets,
    MiniControl::Rich,
    MiniControl::Markdown,
];

/// "Replying to {name}".
const REPLYING_TO: &str = "Replying to";
/// The draft indicator's words (design, Lane C).
const DRAFT_KEPT: &str = "Draft kept";
const DRAFT_NOT_KEPT: &str = "Draft not kept. Post to protect your text.";

/// A control's size (`04-comments.md`: 26 x 24).
const BUTTON: (f32, f32) = (26., 24.);
const ICON: f32 = 14.;
const RADIUS: f32 = 4.;
pub(super) const TEXT_SIZE: f32 = 11.5;
/// The padding of the mode switch's segments, and its track's.
const SEGMENT_PADDING: f32 = 7.;
const TRACK_PADDING: f32 = 2.;
/// A separator's height, and its margin each side.
const SEPARATOR: f32 = 16.;
const SEPARATOR_MARGIN: f32 = 6.;
/// How strongly Markdown draws a formatting control.
const DIM: f32 = 0.35;
/// The toolbar's padding, vertical and horizontal.
pub(super) const PADDING: (f32, f32) = (4., 8.);

impl MiniControl {
    /// Its element ID.
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Bold => "composer-bold",
            Self::Italic => "composer-italic",
            Self::Code => "composer-code",
            Self::Link => "composer-link",
            Self::Bullets => "composer-bullets",
            Self::Rich => "composer-rich",
            Self::Markdown => "composer-markdown",
        }
    }

    /// Its accessible name and tooltip.
    fn label(self) -> &'static str {
        match self {
            Self::Bold => "Bold",
            Self::Italic => "Italic",
            Self::Code => "Inline code",
            Self::Link => "Link",
            Self::Bullets => "Bulleted list",
            Self::Rich => "Rich text",
            Self::Markdown => "Markdown",
        }
    }

    /// Its Lucide icon, or `None` for a mode segment, drawn as its words.
    fn icon(self) -> Option<&'static str> {
        Some(match self {
            Self::Bold => "bold",
            Self::Italic => "italic",
            Self::Code => "code",
            Self::Link => "link",
            Self::Bullets => "list",
            Self::Rich | Self::Markdown => return None,
        })
    }

    /// Whether Markdown dims it: every control but the mode switch.
    fn formatting(self) -> bool {
        !matches!(self, Self::Rich | Self::Markdown)
    }

    fn index(self) -> usize {
        self as usize
    }
}

impl Composer {
    /// The focus handle of `control`.
    pub(crate) fn control_handle(&self, control: MiniControl) -> FocusHandle {
        self.controls[control.index()].clone()
    }

    /// Does what `control` names.
    fn activate(&mut self, control: MiniControl, window: &mut Window, cx: &mut Context<Self>) {
        match control {
            MiniControl::Bold => self.dispatch(&Bold, window, cx),
            MiniControl::Italic => self.dispatch(&Italic, window, cx),
            MiniControl::Code => self.dispatch(&Code, window, cx),
            MiniControl::Link => self.toggle_link(window, cx),
            MiniControl::Bullets => {
                self.host_edit(window, cx, |text, selection| {
                    toggle_prefix(text, selection, LinePrefix::Bullet)
                });
            }
            MiniControl::Rich => self.set_rich(true, cx),
            MiniControl::Markdown => self.set_rich(false, cx),
        }
    }

    /// One of the editor's own actions, on the editor wherever focus is.
    fn dispatch(&self, action: &dyn Action, window: &mut Window, cx: &mut App) {
        let editor = self.editor.read(cx).focus_handle(cx);
        editor.dispatch_action(action, window, cx);
    }

    /// Puts in the edit `edit` makes of the text and the selection, as one
    /// `replace_range`, one undo step. An edit that changes nothing is left
    /// out.
    pub(super) fn host_edit(
        &self,
        window: &mut Window,
        cx: &mut App,
        edit: impl FnOnce(&str, Selection) -> HostEdit,
    ) {
        let utf16 = self
            .editor
            .update(cx, |editor, cx| {
                editor.selected_text_range(false, window, cx)
            })
            .map_or(0..0, |selection| selection.range);
        let selection = selection_from_utf16(self.editor.read(cx).text(), utf16);
        let edit = edit(self.editor.read(cx).text(), selection);
        self.editor.update(cx, |editor, cx| {
            if editor.text().get(edit.range.clone()) == Some(edit.replacement.as_str()) {
                return;
            }
            editor.replace_range(edit.range, &edit.replacement, cx);
            editor.set_cursor(edit.cursor, cx);
        });
    }

    /// Whether `control` is the toolbar's one tab stop: the control that
    /// has focus, or else the first.
    fn is_tab_stop(&self, control: MiniControl, window: &Window) -> bool {
        let focused = CONTROLS
            .into_iter()
            .find(|drawn| self.controls[drawn.index()].is_focused(window));
        focused.unwrap_or(CONTROLS[0]) == control
    }

    /// Moves focus along the controls from the focused one, stopping at
    /// the ends; `to` maps the focused control's place to the new one's.
    fn step(&mut self, to: fn(usize) -> usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(at) = CONTROLS
            .iter()
            .position(|control| self.controls[control.index()].is_focused(window))
        else {
            return;
        };
        let next = to(at).min(COUNT - 1);
        window.focus(&self.controls[CONTROLS[next].index()], cx);
    }

    /// The toolbar row.
    pub(super) fn toolbar(&mut self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let colors = item_colors(cx);
        let mut row = div()
            .id("composer-toolbar")
            .test_support()
            .key_context(TOOLBAR_CONTEXT)
            .role(Role::Toolbar)
            .aria_label("Formatting")
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(2.))
            .py(px(PADDING.0))
            .px(px(PADDING.1))
            .border_b_1()
            .border_color(colors.page_border_soft)
            .text_size(px(TEXT_SIZE))
            .text_color(colors.page_text_secondary)
            .on_action(
                cx.listener(|this, _: &ToolbarNext, window, cx| this.step(|at| at + 1, window, cx)),
            )
            .on_action(cx.listener(|this, _: &ToolbarPrevious, window, cx| {
                this.step(|at| at.saturating_sub(1), window, cx)
            }))
            .on_action(
                cx.listener(|this, _: &ToolbarFirst, window, cx| this.step(|_| 0, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &ToolbarLast, window, cx| {
                    this.step(|_| COUNT - 1, window, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &ToggleRich, _, cx| this.set_rich(!this.rich, cx)));
        if let ComposerKind::Reply { to, .. } = &self.kind {
            let words = format!("{REPLYING_TO} {to}");
            row = row
                .child(
                    div()
                        .id("composer-replying-to")
                        .test_support()
                        .aria_label(words.clone())
                        .flex_none()
                        .whitespace_nowrap()
                        .child(words),
                )
                .child(separator(colors.page_border));
        }
        for control in CONTROLS.into_iter().take(5) {
            row = row.child(self.control(control, window, cx));
        }
        row.child(self.mode_switch(window, cx))
            .child(div().flex_1())
            .children(self.indicator(cx))
            .into_any_element()
    }

    /// `control`'s element: focusable, named, pressed or dimmed as it
    /// should be.
    fn control(
        &self,
        control: MiniControl,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let colors = item_colors(cx);
        let handle = self.controls[control.index()]
            .clone()
            .tab_stop(self.is_tab_stop(control, window));
        let ring = if handle.is_focused(window) {
            cx.theme().ring
        } else {
            Hsla::transparent_black()
        };
        let opacity = if control.formatting() && !self.rich {
            DIM
        } else {
            1.
        };
        let hover = colors.toolbar_hover;
        let (width, height) = BUTTON;
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
            .h(px(height))
            .min_w(px(width))
            .rounded(px(RADIUS))
            .border_2()
            .border_color(ring)
            .opacity(opacity)
            .hover(move |style| style.bg(hover))
            // A click leaves focus where it was, so the editor keeps it.
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.activate(control, window, cx)
            }));
        match control.icon() {
            Some(icon) => {
                element = element.child(
                    Icon::default()
                        .path(icon_path(icon))
                        .w(px(ICON))
                        .h(px(ICON)),
                );
            }
            None => {
                let on = (control == MiniControl::Rich) == self.rich;
                element = element
                    .px(px(SEGMENT_PADDING))
                    .aria_toggled(if on { Toggled::True } else { Toggled::False })
                    .child(control.label());
                if on {
                    element = element.bg(colors.page).shadow_sm();
                }
            }
        }
        element
    }

    /// Rich text | Markdown, as one segmented switch.
    fn mode_switch(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        div()
            .flex()
            .flex_none()
            .items_center()
            .ml(px(SEPARATOR_MARGIN))
            .rounded(px(RADIUS + 1.))
            .p(px(TRACK_PADDING))
            .bg(item_colors(cx).segmented)
            .child(self.control(MiniControl::Rich, window, cx))
            .child(self.control(MiniControl::Markdown, window, cx))
    }

    /// The draft indicator: "Draft kept" for a written draft, "Draft not
    /// kept…" for a failed one, nothing while a write is pending or before
    /// one. Its words are its name.
    fn indicator(&self, cx: &App) -> Option<impl IntoElement + use<>> {
        let colors = item_colors(cx);
        let (words, icon, color) = match self.recovery? {
            RecoveryState::Saved(_) => (DRAFT_KEPT, "shield-check", colors.page_green),
            RecoveryState::Failed(_) => (DRAFT_NOT_KEPT, "circle-alert", colors.page_amber_text),
            RecoveryState::Pending => return None,
        };
        Some(
            div()
                .id("composer-draft")
                .test_support()
                .aria_label(words)
                .flex()
                .flex_none()
                .items_center()
                .gap(px(4.))
                .whitespace_nowrap()
                .child(
                    Icon::default()
                        .path(icon_path(icon))
                        .w(px(ICON - 2.))
                        .h(px(ICON - 2.))
                        .text_color(color),
                )
                .child(words),
        )
    }
}

fn separator(color: Hsla) -> impl IntoElement {
    div()
        .flex_none()
        .w(px(1.))
        .h(px(SEPARATOR))
        .mx(px(SEPARATOR_MARGIN))
        .bg(color)
}
