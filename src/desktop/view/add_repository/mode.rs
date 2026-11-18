//! The Add repository sheet's mode control (Wave 04 Cycle 05a design B5;
//! R-10): the app's own segmented choice, built as Cycle 02's filter bar
//! segments are (`tickets/filter_bar.rs`), since the Kit's segmented
//! controls have no keyboard path (fact 39). Each segment is a Tab stop
//! with `Role::Tab` that Enter or Space chooses; exactly one is chosen.
//! Each mode keeps its typed text while the other is shown.

use gpui_kit::{
    ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{ActiveTheme, Icon, h_flex},
    div,
    prelude::FluentBuilder,
    px,
};

use super::{AddRepositorySheet, MODE_EXISTING, MODE_NEW};
use crate::desktop::{assets::icon_path, view::shell::tokens::chrome_colors};

/// The form the sheet shows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Mode {
    /// Add an existing folder, which every way in shows (B2 step 6).
    Existing,
    /// Create a new repository.
    New,
}

/// The segments, in the order they are offered: the mode, its element ID,
/// its label and its icon.
pub(super) const SEGMENTS: [(Mode, &str, &str, &str); 2] = [
    (
        Mode::Existing,
        "add-mode-existing",
        MODE_EXISTING,
        "folder-input",
    ),
    (Mode::New, "add-mode-new", MODE_NEW, "folder-plus"),
];

impl AddRepositorySheet {
    /// Shows `mode`'s form; the other keeps its text. An Add waiting for
    /// its folder's check is forgotten: the person left that form, and the
    /// landed check must not confirm or move focus behind it.
    pub(super) fn set_mode(&mut self, mode: Mode, cx: &mut Context<Self>) {
        if self.mode != mode {
            self.mode = mode;
            self.submit_pending = false;
            cx.notify();
        }
    }

    /// The two segments, the chosen one marked.
    pub(super) fn mode_control(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let colors = chrome_colors(cx);
        let ring = cx.theme().ring;
        h_flex()
            .id("add-mode")
            .test_support()
            .role(Role::TabList)
            .w_full()
            .rounded(px(6.))
            .border_1()
            .border_color(colors.border_strong)
            .overflow_hidden()
            .children(SEGMENTS.map(|(mode, id, label, icon)| {
                let focus = &self.focus[id];
                let chosen = self.mode == mode;
                let focused = focus.is_focused(window);
                div()
                    .id(id)
                    .test_support()
                    .track_focus(focus)
                    .role(Role::Tab)
                    .aria_label(label)
                    .aria_selected(chosen)
                    .relative()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.))
                    .h(px(30.))
                    .px(px(10.))
                    .text_size(px(12.5))
                    .cursor_pointer()
                    .when(chosen, |segment| {
                        segment.bg(colors.border).text_color(colors.text)
                    })
                    .when(!chosen, |segment| {
                        segment
                            .text_color(colors.secondary)
                            .hover(|style| style.bg(colors.hover).text_color(colors.text))
                    })
                    .on_click(
                        cx.listener(move |this, _: &ClickEvent, _, cx| this.set_mode(mode, cx)),
                    )
                    .child(Icon::default().path(icon_path(icon)).w(px(14.)).h(px(14.)))
                    .child(label)
                    .child(self.scroll.marker(focus))
                    .when(focused, |segment| {
                        segment.child(
                            div()
                                .absolute()
                                .top_0()
                                .left_0()
                                .size_full()
                                .border_2()
                                .border_color(ring),
                        )
                    })
            }))
    }
}
