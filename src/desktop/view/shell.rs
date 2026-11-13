//! The workspace shell: the window's chrome around the dock area.

pub(crate) mod title;
pub(crate) mod tokens;

use gpui_kit::{
    App, ClickEvent, FontWeight, InteractiveElement, ParentElement, SharedString, Styled,
    TestSupportExt, WeakEntity, Window,
    component::{TitleBar, h_flex},
    div, img, px,
};

use self::tokens::chrome_colors;
use super::workspace::Workspace;
use crate::desktop::assets::APP_ICON_PATH;

/// The title bar's height (`07`); the Kit's own is 34 px.
pub(crate) const TITLE_BAR_HEIGHT: f32 = 38.;
const APP_ICON_SIZE: f32 = 18.;
const TITLE_SIZE: f32 = 12.;

/// The title bar (Part D): the app icon and `title`, which is the string
/// the window's own title is set to. The Kit's bar keeps its drag, its
/// double click and, with client decorations on Linux, the window controls;
/// the close control quits as the Quit action does.
pub(crate) fn title_bar(
    title: SharedString,
    workspace: WeakEntity<Workspace>,
    cx: &App,
) -> TitleBar {
    title_bar_closing(
        title,
        move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
            let _ = workspace.update(cx, |workspace, cx| {
                workspace.request_quit(window, cx);
            });
        },
        cx,
    )
}

/// The same bar for a window with no workspace (the startup failure
/// window), whose close control does `on_close`.
pub(crate) fn title_bar_closing(
    title: SharedString,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> TitleBar {
    let colors = chrome_colors(cx);
    TitleBar::new()
        .h(px(TITLE_BAR_HEIGHT))
        .bg(colors.chrome)
        .border_color(colors.border)
        .on_close_window(on_close)
        .child(
            h_flex()
                .id("title-bar-title")
                .test_support()
                .gap_2()
                .items_center()
                .child(
                    img(APP_ICON_PATH)
                        .w(px(APP_ICON_SIZE))
                        .h(px(APP_ICON_SIZE))
                        .flex_none(),
                )
                .child(
                    div()
                        .text_size(px(TITLE_SIZE))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(colors.text)
                        .whitespace_nowrap()
                        .child(title),
                ),
        )
}
