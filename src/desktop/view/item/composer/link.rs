//! The Link control's address field (design, Lane C; as the edit
//! toolbar's link popover): drawn under the mini toolbar, focused when it
//! opens. Enter makes the selection a link (`insert_link`), or puts the
//! address in as its own text with no selection; Escape closes it; either
//! gives focus back to where it was. Focus leaving it closes it too. Tab
//! and Shift-Tab stay on it, its one stop. The address is the user's
//! text: it stays in its field and goes nowhere else.

use gpui_kit::{
    AnyElement, App, AppContext, Context, Entity, FocusHandle, Focusable, FontWeight,
    InteractiveElement, IntoElement, ParentElement, Role, StatefulInteractiveElement, Styled,
    Subscription, TestSupportExt, Window,
    component::{
        Sizable,
        input::{Input, InputEvent, InputState},
    },
    div, px,
};

use super::{
    Composer,
    toolbar::{PADDING, TEXT_SIZE},
};
use crate::desktop::{
    keys::{ClosePopover, CycleLinkPopover, TOOLBAR_LINK_CONTEXT},
    model::markdown_edit::insert_link,
    view::item::tokens::item_colors,
};

/// The address field's own words.
const LINK_TITLE: &str = "Link address";
const LINK_PLACEHOLDER: &str = "https://";

/// The Link control's open address field. It holds the user's address,
/// so it derives nothing.
pub(crate) struct LinkField {
    input: Entity<InputState>,
    /// Where focus was when it opened.
    origin: Option<FocusHandle>,
    /// Drawn once already: its focus can be looked for in the last frame.
    drawn: bool,
    _enter: Subscription,
}

impl LinkField {
    pub(super) fn contains_focus(&self, window: &Window, cx: &App) -> bool {
        self.input.read(cx).focus_handle(cx).is_focused(window)
    }
}

impl Composer {
    /// The address field, while it is open.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn link_input(&self) -> Option<Entity<InputState>> {
        self.link.as_ref().map(|link| link.input.clone())
    }

    /// Opens the address field, focused, or closes it.
    pub(super) fn toggle_link(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.link.is_some() {
            self.close_link(window, cx);
            return;
        }
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(LINK_PLACEHOLDER));
        let enter = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = event {
                this.insert_link(window, cx);
            }
        });
        let origin = window.focused(cx);
        input.update(cx, |input, cx| input.focus(window, cx));
        self.link = Some(LinkField {
            input,
            origin,
            drawn: false,
            _enter: enter,
        });
        cx.notify();
    }

    /// Closes the address field, and gives focus back to where it was.
    pub(super) fn close_link(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(link) = self.link.take() {
            if let Some(origin) = link.origin {
                window.focus(&origin, cx);
            }
            cx.notify();
        }
    }

    /// The address typed, trimmed, makes the selection a link, or is the
    /// link's text with no selection. An empty field does nothing.
    fn insert_link(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(link) = &self.link else {
            return;
        };
        let address = link.input.read(cx).value().trim().to_owned();
        if address.is_empty() {
            return;
        }
        self.host_edit(window, cx, |text, selection| {
            insert_link(text, selection, &address)
        });
        self.close_link(window, cx);
    }

    /// As a frame is drawn: an address field that focus has left since its
    /// last frame closes, leaving focus where it went.
    pub(super) fn close_link_if_left(&mut self, window: &Window, cx: &App) {
        let left = self
            .link
            .as_ref()
            .is_some_and(|link| link.drawn && !link.contains_focus(window, cx));
        if left {
            self.link = None;
        } else if let Some(link) = &mut self.link {
            link.drawn = true;
        }
    }

    /// The address field under the toolbar, while it is open.
    pub(super) fn link_row(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let link = self.link.as_ref()?;
        let colors = item_colors(cx);
        Some(
            div()
                .id("composer-link-field")
                .test_support()
                .key_context(TOOLBAR_LINK_CONTEXT)
                .role(Role::Dialog)
                .aria_label(LINK_TITLE)
                .flex()
                .items_center()
                .gap(px(6.))
                .py(px(PADDING.0))
                .px(px(PADDING.1))
                .border_b_1()
                .border_color(colors.page_border_soft)
                .text_size(px(TEXT_SIZE))
                .on_action(
                    cx.listener(|this, _: &ClosePopover, window, cx| this.close_link(window, cx)),
                )
                // The field is its one stop: Tab and Shift-Tab stay on it.
                .on_action(cx.listener(|_, _: &CycleLinkPopover, _, _| {}))
                .child(
                    div()
                        .flex_none()
                        .font_weight(FontWeight::MEDIUM)
                        .child(LINK_TITLE),
                )
                .child(div().flex_1().child(Input::new(&link.input).small()))
                .into_any_element(),
        )
    }
}
