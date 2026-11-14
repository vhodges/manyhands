//! The Settings sheet's frame (Wave 04 Cycle 04 design B1 and B2): the Kit
//! sheet, 720 px wide on the right, with the heading and its description in
//! the header and no footer; the page list down the left; and the shown
//! page in a scroll of its own laid over the Kit's body, as the details
//! sheet does. The frame's root, which holds the list and the body, takes
//! the page keys and moves the body: the keys are bound in the content
//! region's context, which the sheet's focus is inside.

use gpui_kit::{
    AnyElement, ClickEvent, Context, FontWeight, InteractiveElement, IntoElement, ParentElement,
    Role, StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{ActiveTheme, h_flex, scroll::Scrollbar, sheet::Sheet, v_flex},
    div, px,
};

use super::{PAGES, SETTINGS_HEADING, SETTINGS_NOTE, SettingsHost, Shown, page_title};
use crate::desktop::{
    keys::{ScrollPageDown, ScrollPageUp, ScrollToBottom, ScrollToTop},
    model::stamp::SettingsPage,
    view::{scroll::ContentScroll, tabs::SHOWN},
};

/// The sheet's width (`05`, Settings sheet).
const WIDTH: f32 = 720.;
/// The page list's width (`05`'s left nav).
const MENU_WIDTH: f32 = 214.;
/// The heading's size, as the details sheet's title.
const TITLE_SIZE: f32 = 15.;

impl SettingsHost {
    /// The sheet as this frame draws it. The builder Root keeps calls this
    /// on every render, so all its state is the host's.
    pub(crate) fn sheet(
        &mut self,
        sheet: Sheet,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Sheet {
        // A typed passphrase does not outlive the Keys page being shown.
        self.forget_passphrase(window, cx);
        let Shown::Settings(shown) = self.shown else {
            return sheet;
        };
        let page = self.page(shown, window, cx);
        let entries = self.entries(shown, window, cx);
        let muted = cx.theme().muted_foreground;
        let handle = self.scroll.handle().clone();
        sheet
            .size(px(WIDTH))
            .title(
                v_flex()
                    .gap_0p5()
                    .child(
                        div()
                            .id("settings-heading")
                            .test_support()
                            .aria_label(SETTINGS_HEADING)
                            .text_size(px(TITLE_SIZE))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(SETTINGS_HEADING),
                    )
                    .child(
                        div()
                            .id("settings-note")
                            .test_support()
                            .aria_label(SETTINGS_NOTE)
                            .text_sm()
                            .font_weight(FontWeight::NORMAL)
                            .text_color(muted)
                            .child(SETTINGS_NOTE),
                    ),
            )
            .child(
                h_flex()
                    .id("settings")
                    .test_support()
                    .absolute()
                    .inset_0()
                    .items_start()
                    .gap_3()
                    .on_action(cx.listener(|this, _: &ScrollPageUp, _, cx| {
                        this.scroll_body(|scroll| scroll.page(-1.), cx)
                    }))
                    .on_action(cx.listener(|this, _: &ScrollPageDown, _, cx| {
                        this.scroll_body(|scroll| scroll.page(1.), cx)
                    }))
                    .on_action(cx.listener(|this, _: &ScrollToTop, _, cx| {
                        this.scroll_body(ContentScroll::to_top, cx)
                    }))
                    .on_action(cx.listener(|this, _: &ScrollToBottom, _, cx| {
                        this.scroll_body(ContentScroll::to_bottom, cx)
                    }))
                    .child(entries)
                    .child(
                        div()
                            .id("settings-body")
                            .relative()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .child(
                                div()
                                    .id("settings-scroll")
                                    .test_support()
                                    .size_full()
                                    .overflow_y_scroll()
                                    .track_scroll(&handle)
                                    .child(div().pr_4().pb_4().children(page)),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .inset_0()
                                    .child(Scrollbar::vertical(&handle).id("settings-scrollbar")),
                            ),
                    ),
            )
    }

    /// A page key moves the body, and nothing behind the sheet.
    fn scroll_body(&mut self, scroll: impl Fn(&ContentScroll) -> bool, cx: &mut Context<Self>) {
        if scroll(&self.scroll) {
            cx.notify();
        }
    }

    /// The page list: one entry per page, down the left, the shown one
    /// saying so in text. The entries are outside the body's scroll, so they
    /// carry none of its markers.
    fn entries(&self, shown: SettingsPage, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (ring, border) = (theme.ring, theme.border);
        let mut list = v_flex()
            .id("settings-pages")
            .flex_none()
            .w(px(MENU_WIDTH))
            .gap_1();
        for (ix, page) in PAGES.iter().copied().enumerate() {
            let focus = &self.entries[ix];
            let active = page == shown;
            let mut entry =
                div()
                    .id(("settings-page", ix))
                    .test_support()
                    .role(Role::Tab)
                    .aria_label(page_title(page))
                    .aria_selected(active)
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .border_1()
                    .border_color(border)
                    .track_focus(focus)
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.open(page, window, cx)
                    }))
                    .child(
                        h_flex()
                            .gap_1()
                            .child(div().text_sm().child(page_title(page)))
                            .children(active.then(|| {
                                div()
                                    .id(("settings-page-shown", ix))
                                    .test_support()
                                    .aria_label(SHOWN)
                                    .text_xs()
                                    .child(SHOWN)
                            })),
                    );
            if focus.is_focused(window) {
                entry = entry.border_2().border_color(ring);
            }
            list = list.child(entry);
        }
        list.into_any_element()
    }
}
