//! How a composer is drawn (`item_view/04-comments.md`): a white card with
//! the mini toolbar, the text area and the footer; a reply's has the
//! user's avatar beside it. Its footer says what posting does and holds
//! the buttons (design, Lane C, Footer; R-9).

use gpui_kit::{
    App, BoxShadow, Context, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement,
    Render, StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, Sizable,
        button::{Button, ButtonCustomVariant, ButtonVariants},
        h_flex, v_flex,
    },
    div, point, px, white,
};
use manyhands::drafts::ItemKind;

use super::{Composer, ComposerKind};
use crate::desktop::{
    keys::{COMMENT_COMPOSER_CONTEXT, ComposerToggleMode},
    model::discussion::{avatar_color, avatar_initials},
    view::item::tokens::item_colors,
};

/// The text area's least height: a new comment's, and a reply's.
const NEW_MIN_HEIGHT: f32 = 70.;
const REPLY_MIN_HEIGHT: f32 = 64.;
/// The text area's padding, vertical and horizontal.
const AREA_PADDING: (f32, f32) = (8., 12.);
const TEXT_SIZE: f32 = 14.;
/// Markdown's font size, as the body editor's.
const MARKDOWN_SIZE: f32 = 13.5;
const FOOTER_SIZE: f32 = 12.;
const RADIUS: f32 = 8.;
const AVATAR: f32 = 26.;
const AVATAR_TEXT: f32 = 10.5;

/// The footer's words with a publication remote, before the item's kind.
const SHARES: &str = "Posting also shares other saved work in this";
/// The footer's words after it, and all of them without a remote.
const UNSAVED: &str = "Unsaved edits above are not included.";
const LOCAL: &str = "Saved on this computer.";
/// A reply's hint. The dot is U+00B7.
const REPLY_HINT: &str = "Ctrl Enter to post \u{b7} Esc to cancel";
const CANCEL: &str = "Cancel";

impl Composer {
    /// The post button's words, by kind and remote.
    fn post_label(&self) -> &'static str {
        match (&self.kind, self.remote) {
            (ComposerKind::New, true) => "Post and sync",
            (ComposerKind::New, false) => "Save comment locally",
            (ComposerKind::Reply { .. }, true) => "Reply and sync",
            (ComposerKind::Reply { .. }, false) => "Save reply locally",
        }
    }

    /// The footer's explanation (R-9).
    fn footer_words(&self) -> String {
        if self.remote {
            let noun = match self.item {
                ItemKind::Ticket => "ticket",
                ItemKind::Document => "document",
            };
            format!("{SHARES} {noun}. {UNSAVED}")
        } else {
            format!("{LOCAL} {UNSAVED}")
        }
    }

    /// The text area: the composer's tab stop around its editor.
    fn area(&self, cx: &App) -> impl IntoElement + use<> {
        let min = match self.kind {
            ComposerKind::New => NEW_MIN_HEIGHT,
            ComposerKind::Reply { .. } => REPLY_MIN_HEIGHT,
        };
        let placeholder = self.placeholder();
        let mut area = div()
            .id("composer-editor")
            .test_support()
            .track_focus(&self.focus)
            .aria_label(placeholder)
            .aria_placeholder(placeholder)
            .w_full()
            .min_h(px(min))
            .py(px(AREA_PADDING.0))
            .px(px(AREA_PADDING.1))
            .text_size(px(TEXT_SIZE));
        if !self.rich {
            area = area
                .font_family(cx.theme().mono_font_family.clone())
                .text_size(px(MARKDOWN_SIZE))
                .text_color(item_colors(cx).page_text);
        }
        area.child(self.editor.clone())
    }

    /// The footer: a reply's hint, the explanation, then Cancel (a reply's)
    /// and the post button.
    fn footer(&self, cx: &App) -> impl IntoElement + use<> {
        let colors = item_colors(cx);
        let reply = matches!(self.kind, ComposerKind::Reply { .. });
        let words = self.footer_words();
        let post = self.post_label();
        let amber = ButtonCustomVariant::new(cx)
            .color(colors.page_amber)
            .foreground(white())
            .hover(colors.page_amber_text)
            .active(colors.page_amber_text);
        let explanation = v_flex()
            .flex_1()
            .min_w_0()
            .gap(px(2.))
            .children(reply.then(|| {
                div()
                    .id("composer-hint")
                    .test_support()
                    .aria_label(REPLY_HINT)
                    .child(REPLY_HINT)
            }))
            .child(
                div()
                    .id("composer-footer")
                    .test_support()
                    .aria_label(words.clone())
                    .child(words),
            );
        h_flex()
            .w_full()
            .items_center()
            .gap(px(8.))
            .px(px(AREA_PADDING.1))
            .py(px(AREA_PADDING.0))
            .text_size(px(FOOTER_SIZE))
            .text_color(colors.page_muted)
            .child(explanation)
            .children(reply.then(|| {
                div()
                    .id("composer-cancel")
                    .test_support()
                    .aria_label(CANCEL)
                    .flex_none()
                    .child(
                        Button::new("composer-cancel-button")
                            .label(CANCEL)
                            .small()
                            .outline(),
                    )
            }))
            .child(
                div()
                    .id("composer-post")
                    .test_support()
                    .aria_label(post)
                    .flex_none()
                    .child(
                        Button::new("composer-post-button")
                            .label(post)
                            .small()
                            .custom(amber),
                    ),
            )
    }

    /// A reply's avatar: the user's initials on their colour.
    fn avatar(&self) -> Option<impl IntoElement + use<>> {
        let ComposerKind::Reply { me, .. } = &self.kind else {
            return None;
        };
        let letters = avatar_initials(me.as_deref());
        Some(
            div()
                .id("composer-avatar")
                .test_support()
                .aria_label(letters.clone())
                .flex()
                .flex_none()
                .size(px(AVATAR))
                .rounded_full()
                .items_center()
                .justify_center()
                .bg(avatar_color(me.as_deref().unwrap_or_default()))
                .text_color(white())
                .text_size(px(AVATAR_TEXT))
                .font_weight(FontWeight::SEMIBOLD)
                .child(letters),
        )
    }
}

impl Render for Composer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.close_link_if_left(window, cx);
        if self.focus.is_focused(window) {
            // Tab landed on the composer's stop: focus belongs in the
            // editor. It moves once this frame is drawn, as a focus
            // listener's would; GPUI sends those only to an active window.
            cx.defer_in(window, |this, window, cx| {
                if this.focus.is_focused(window) {
                    this.editor
                        .update(cx, |editor, cx| editor.focus(window, cx));
                }
            });
        }
        let colors = item_colors(cx);
        let focused = self.contains_focus(window, cx);
        let border = if focused {
            colors.page_teal
        } else {
            colors.page_border_strong
        };
        let mut card = v_flex()
            .flex_1()
            .min_w_0()
            .bg(colors.page)
            .text_color(colors.page_text)
            .border_1()
            .border_color(border)
            .rounded(px(RADIUS));
        if focused {
            card = card.shadow(ring(colors.page_teal));
        }
        let card = card
            .child(self.toolbar(window, cx))
            .children(self.link_row(cx))
            .child(self.area(cx))
            .child(self.footer(cx));
        h_flex()
            .id("composer")
            .test_support()
            .key_context(COMMENT_COMPOSER_CONTEXT)
            .w_full()
            .items_start()
            .gap(px(10.))
            .on_action(
                cx.listener(|this, _: &ComposerToggleMode, _, cx| this.set_rich(!this.rich, cx)),
            )
            .children(self.avatar())
            .child(card)
    }
}

/// The focus ring's soft outer ring (`04-comments.md`: 3 px of the teal at
/// 14%), drawn as a spread shadow.
fn ring(teal: Hsla) -> Vec<BoxShadow> {
    let mut color = teal;
    color.a = 0.14;
    vec![BoxShadow {
        color,
        offset: point(px(0.), px(0.)),
        blur_radius: px(0.),
        spread_radius: px(3.),
        inset: false,
    }]
}
