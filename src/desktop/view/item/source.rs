//! The text on the page: an Inspect tab's retained read-only source text,
//! selection and keyboard behavior (D1's, kept by R-64), and an item tab's
//! read view. A file that is not UTF-8 says so in either.

use gpui_kit::{
    AnyElement, App, AppContext, Context, InteractiveElement, IntoElement, ParentElement, Styled,
    TestSupportExt, Window,
    component::{
        ActiveTheme,
        input::{IndentInline, OutdentInline, Textarea, TextareaState},
    },
    div, px,
};
use manyhands::repository::ItemDto;

use super::{ItemPane, SOURCE, SOURCE_NOT_UTF8, text, tokens::item_colors};
use crate::desktop::model::navigation::OpenTarget;

impl ItemPane {
    /// Only a successful read changes the source. Keep its entity/selection
    /// during an in-flight or failed read, and remove it only for non-UTF8
    /// text. An item tab has none: it reads the body in its read view.
    pub(super) fn sync_source(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !matches!(self.model.target(), OpenTarget::Inspect { .. }) {
            return;
        }
        let Some(item) = self.model.item().rows() else {
            return;
        };
        let Some(source) = item.source.clone() else {
            self.source = None;
            return;
        };
        match &mut self.source {
            Some((shown, _)) if *shown == source => {}
            Some((shown, state)) => {
                state.update(cx, |state, cx| state.set_value(source.clone(), window, cx));
                *shown = source;
            }
            None => {
                let value = source.clone();
                let state = cx.new(|cx| {
                    let mut state = TextareaState::new(window, cx).default_value(value);
                    state.set_readonly(true, cx);
                    state
                });
                self.source = Some((source, state));
            }
        }
    }

    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn source_state(&self) -> Option<&gpui_kit::Entity<TextareaState>> {
        self.source.as_ref().map(|(_, state)| state)
    }

    /// An Inspect tab's whole file, in D1's read-only source view.
    pub(super) fn source(&self, item: &ItemDto, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        match (&item.source, &self.source) {
            (Some(_), Some((_, state))) => div()
                .id("item-source")
                .test_support()
                .capture_action(|_: &IndentInline, window, cx| {
                    window.focus_next(cx);
                    cx.stop_propagation();
                })
                .capture_action(|_: &OutdentInline, window, cx| {
                    window.focus_prev(cx);
                    cx.stop_propagation();
                })
                .child(
                    Textarea::new(state)
                        .readonly(true)
                        .aria_label(SOURCE)
                        .font_family(theme.mono_font_family.clone())
                        .text_sm()
                        .h(px(320.)),
                )
                .relative()
                .child(
                    self.scroll
                        .marker(&gpui_kit::Focusable::focus_handle(state.read(cx), cx)),
                )
                .into_any_element(),
            (Some(_), None) => div().into_any_element(),
            (None, _) => not_utf8(cx),
        }
    }

    /// An item tab's body, in its read view (decision 17). A file with no
    /// body to show says why when it is not UTF-8; a nonconforming one
    /// shows its problems in the header's "Needs fixing" instead.
    pub(super) fn read_body(&self, item: &ItemDto, cx: &App) -> AnyElement {
        match (&item.source, self.page.read_view()) {
            (None, _) => not_utf8(cx),
            (Some(_), Some(read)) => read,
            (Some(_), None) => div().into_any_element(),
        }
    }
}

/// D1's words for a file that is not UTF-8, muted on the white page.
fn not_utf8(cx: &App) -> AnyElement {
    text("item-source-missing", SOURCE_NOT_UTF8.to_owned())
        .text_sm()
        .text_color(item_colors(cx).page_muted)
        .into_any_element()
}
