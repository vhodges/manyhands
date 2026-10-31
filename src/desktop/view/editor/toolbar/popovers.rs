//! The toolbar's two popovers, drawn under their controls: the Style menu
//! (Paragraph, Heading 1–3) and the link popover, which asks for the
//! address (design, "The toolbar": Block and Link).
//!
//! Each takes focus when it opens, by pointer or by key, and gives it back
//! to where it was when it closes: on a choice, on Escape, or on the
//! control again. Focus moving out of it closes it too, found as each
//! frame is drawn (GPUI sends focus events only to an active window). In
//! the menu, Up, Down, Home and End move through its items; Left and Right
//! do nothing, since the menu is vertical and stays open. The link
//! popover is a dialog, so it keeps focus (contract 13): Tab and
//! Shift-Tab go between its address and Insert. The address is the
//! user's text: it stays in its field, which the popover lets go of when
//! it closes, and goes nowhere else.

use gpui_kit::{
    AnyElement, AppContext, ClickEvent, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Role, StatefulInteractiveElement, Styled,
    Subscription, TestSupportExt, Window, anchored,
    component::{
        ActiveTheme, Sizable,
        button::{Button, ButtonVariants},
        input::{Input, InputEvent, InputState},
    },
    deferred, div, px,
};

use super::{
    Step, Toolbar,
    controls::Control,
    render::{BUTTON, Observed},
};
use crate::desktop::{
    keys::{
        ClosePopover, CycleLinkPopover, MenuFirst, MenuLast, MenuNext, MenuPrevious,
        TOOLBAR_LINK_CONTEXT, TOOLBAR_MENU_CONTEXT,
    },
    model::markdown_edit::{BlockStyle, insert_link, set_block_style},
};

/// The Style menu's choices, in order.
pub(super) const STYLES: [BlockStyle; 4] = [
    BlockStyle::Paragraph,
    BlockStyle::Heading(1),
    BlockStyle::Heading(2),
    BlockStyle::Heading(3),
];

/// The gap between a control and the popover under it.
const DROP: f32 = 4.;
const MENU_WIDTH: f32 = 160.;
const LINK_WIDTH: f32 = 300.;
const RADIUS: f32 = 6.;
/// The link popover's own words.
const LINK_TITLE: &str = "Link address";
const LINK_PLACEHOLDER: &str = "https://";
const INSERT: &str = "Insert";

/// What the Style control says of `style`.
pub(super) fn style_name(style: BlockStyle) -> String {
    match style {
        BlockStyle::Paragraph | BlockStyle::Heading(0) => "Paragraph".to_owned(),
        BlockStyle::Heading(level) => format!("Heading {level}"),
    }
}

/// An open popover. It holds the address field, so it derives nothing.
pub(super) struct Popover {
    pub(super) kind: Kind,
    /// The popover's own focus, which holds its items or its field.
    focus: FocusHandle,
    /// Where focus was when it opened.
    origin: Option<FocusHandle>,
    /// Drawn once already: its focus can be looked for in the last frame.
    drawn: bool,
}

pub(super) enum Kind {
    Style([FocusHandle; 4]),
    Link {
        input: Entity<InputState>,
        _enter: Subscription,
    },
}

impl Toolbar {
    /// Opens the Style menu on the caret line's style, or closes it.
    pub(super) fn toggle_style(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(self.popover.as_ref().map(|p| &p.kind), Some(Kind::Style(_))) {
            self.close_popover(window, cx);
            return;
        }
        let items = [(); 4].map(|_| cx.focus_handle());
        let current = STYLES
            .iter()
            .position(|style| *style == self.active.style)
            .unwrap_or(0);
        let origin = self.take_origin(window, cx);
        window.focus(&items[current], cx);
        self.popover = Some(Popover {
            kind: Kind::Style(items),
            focus: cx.focus_handle(),
            origin,
            drawn: false,
        });
        cx.notify();
    }

    /// Opens the link popover on an empty address, or closes it.
    pub(super) fn toggle_link(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(
            self.popover.as_ref().map(|p| &p.kind),
            Some(Kind::Link { .. })
        ) {
            self.close_popover(window, cx);
            return;
        }
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(LINK_PLACEHOLDER));
        let enter = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = event {
                this.insert_link(window, cx);
            }
        });
        let origin = self.take_origin(window, cx);
        input.update(cx, |input, cx| input.focus(window, cx));
        self.popover = Some(Popover {
            kind: Kind::Link {
                input,
                _enter: enter,
            },
            focus: cx.focus_handle(),
            origin,
            drawn: false,
        });
        cx.notify();
    }

    /// Where focus goes back to: the open popover's own origin, when one
    /// gives way to the other, or else where focus is.
    fn take_origin(&mut self, window: &Window, cx: &Context<Self>) -> Option<FocusHandle> {
        match self.popover.take() {
            Some(open) => open.origin,
            None => window.focused(cx),
        }
    }

    /// Closes the open popover, and gives focus back to where it was.
    pub(super) fn close_popover(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(popover) = self.popover.take() {
            if let Some(origin) = popover.origin {
                window.focus(&origin, cx);
            }
            cx.notify();
        }
    }

    /// As a frame is drawn: a popover that focus has left since its last
    /// frame closes, leaving focus where it went.
    pub(super) fn close_if_left(&mut self, window: &Window, cx: &Context<Self>) {
        let left = self
            .popover
            .as_ref()
            .is_some_and(|popover| popover.drawn && !popover.focus.contains_focused(window, cx));
        if left {
            self.popover = None;
        } else if let Some(popover) = &mut self.popover {
            popover.drawn = true;
        }
    }

    fn choose_style(&mut self, style: BlockStyle, window: &mut Window, cx: &mut Context<Self>) {
        self.host_edit(window, cx, |text, selection| {
            set_block_style(text, selection, style)
        });
        self.close_popover(window, cx);
    }

    /// The address typed, trimmed, makes the selection a link, or is the
    /// link's text with no selection. An empty field does nothing.
    fn insert_link(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(Popover {
            kind: Kind::Link { input, .. },
            ..
        }) = &self.popover
        else {
            return;
        };
        let address = input.read(cx).value().trim().to_owned();
        if address.is_empty() {
            return;
        }
        self.host_edit(window, cx, |text, selection| {
            insert_link(text, selection, &address)
        });
        self.close_popover(window, cx);
    }

    /// Tab or Shift-Tab in the link popover. Its two stops are the address
    /// and Insert, so either key goes to the other, and focus stays in the
    /// popover. Insert's focus is the Kit button's own, so it is reached
    /// as the stop after the address; should that stop be outside, focus
    /// stays on the address.
    fn cycle_link(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(Popover {
            kind: Kind::Link { input, .. },
            focus,
            ..
        }) = &self.popover
        else {
            return;
        };
        let field = input.read(cx).focus_handle(cx);
        if field.is_focused(window) {
            window.focus_next(cx);
            if focus.contains_focused(window, cx) && !field.is_focused(window) {
                return;
            }
        }
        window.focus(&field, cx);
    }

    /// Moves focus through the Style menu's items, stopping at its ends.
    fn menu_step(&mut self, step: Step, window: &mut Window, cx: &mut Context<Self>) {
        let Some(Popover {
            kind: Kind::Style(items),
            ..
        }) = &self.popover
        else {
            return;
        };
        let Some(at) = items.iter().position(|item| item.is_focused(window)) else {
            return;
        };
        let to = step.apply(at, items.len());
        window.focus(&items[to], cx);
    }

    /// The open popover, drawn under its control: the Style menu under
    /// Style (`style`), the link popover under Link.
    pub(super) fn popover_under(
        &self,
        style: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let popover = self.popover.as_ref()?;
        let panel = match (&popover.kind, style) {
            (Kind::Style(items), true) => self.style_menu(items, window, cx),
            (Kind::Link { input, .. }, false) => self.link_panel(input, cx),
            _ => return None,
        };
        let panel = panel
            .track_focus(&popover.focus)
            .occlude()
            .mt(px(BUTTON + DROP))
            .rounded(px(RADIUS))
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().popover)
            .text_color(cx.theme().popover_foreground)
            .shadow_md();
        Some(
            deferred(anchored().snap_to_window_with_margin(px(8.)).child(panel))
                .with_priority(1)
                .into_any_element(),
        )
    }

    fn style_menu(
        &self,
        items: &[FocusHandle; 4],
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Observed {
        let hover = cx.theme().accent;
        let current = self.active.style;
        div()
            .id("toolbar-style-menu")
            .test_support()
            .key_context(TOOLBAR_MENU_CONTEXT)
            .role(Role::Menu)
            .aria_label(Control::Style.label())
            .w(px(MENU_WIDTH))
            .py(px(4.))
            .flex()
            .flex_col()
            .on_action(
                cx.listener(|this, _: &MenuNext, window, cx| {
                    this.menu_step(Step::Next, window, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &MenuPrevious, window, cx| {
                this.menu_step(Step::Previous, window, cx)
            }))
            .on_action(cx.listener(|this, _: &MenuFirst, window, cx| {
                this.menu_step(Step::First, window, cx)
            }))
            .on_action(
                cx.listener(|this, _: &MenuLast, window, cx| {
                    this.menu_step(Step::Last, window, cx)
                }),
            )
            .on_action(
                cx.listener(|this, _: &ClosePopover, window, cx| this.close_popover(window, cx)),
            )
            .children(
                STYLES
                    .iter()
                    .zip(items)
                    .enumerate()
                    .map(|(ix, (style, item))| {
                        let style = *style;
                        let name = style_name(style);
                        let option = div()
                            .id(("toolbar-style-option", ix))
                            .test_support()
                            .track_focus(item)
                            .role(Role::MenuItem)
                            .aria_label(name.clone())
                            .aria_selected(style == current)
                            .px(px(12.))
                            .py(px(4.))
                            .hover(move |s| s.bg(hover))
                            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                this.choose_style(style, window, cx)
                            }))
                            .child(name);
                        if item.is_focused(window) {
                            option.bg(hover)
                        } else {
                            option
                        }
                    }),
            )
    }

    fn link_panel(&self, input: &Entity<InputState>, cx: &mut Context<Self>) -> Observed {
        div()
            .id("toolbar-link-popover")
            .test_support()
            .key_context(TOOLBAR_LINK_CONTEXT)
            .role(Role::Dialog)
            .aria_label(LINK_TITLE)
            .w(px(LINK_WIDTH))
            .p(px(8.))
            .flex()
            .flex_col()
            .gap(px(6.))
            .on_action(
                cx.listener(|this, _: &ClosePopover, window, cx| this.close_popover(window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &CycleLinkPopover, window, cx| this.cycle_link(window, cx)),
            )
            .child(LINK_TITLE)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(div().flex_1().child(Input::new(input).small()))
                    .child(
                        Button::new("toolbar-link-insert")
                            .label(INSERT)
                            .small()
                            .primary()
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.insert_link(window, cx)
                            })),
                    ),
            )
    }

    /// The link popover's address field, while it is open.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(super) fn link_input(&self) -> Option<Entity<InputState>> {
        match self.popover.as_ref().map(|popover| &popover.kind) {
            Some(Kind::Link { input, .. }) => Some(input.clone()),
            _ => None,
        }
    }
}
