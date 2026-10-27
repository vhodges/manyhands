//! The item colours: header tokens by mode, the page white in both.

use gpui_kit::{TestAppContext, component::Theme, rgb};

use super::{colors, item_colors};

#[gpui_kit::test]
fn header_colours_follow_dark_and_light_and_the_page_stays_white(cx: &mut TestAppContext) {
    let dark = colors(true);
    let light = colors(false);
    assert_eq!(dark.header_block, rgb(0x262a30).into());
    assert_eq!(dark.header_border, rgb(0x353a41).into());
    assert_eq!(dark.header_title, rgb(0xeef0f2).into());
    assert_eq!(light.header_block, rgb(0xebe8e3).into());
    assert_eq!(light.header_border, rgb(0xe2dfda).into());
    assert_eq!(light.header_title, rgb(0x15181c).into());
    // The page is the same in both modes, and white.
    assert_eq!(dark.page, rgb(0xffffff).into());
    assert_eq!(dark.page, light.page);
    assert_eq!(dark.page_text, rgb(0x1f2328).into());
    assert_eq!(dark.page_text, light.page_text);
    assert_eq!(dark.page_border_strong, light.page_border_strong);
    assert_eq!(dark.selection, light.selection);

    // `item_colors` reads the mode from the theme.
    cx.update(|cx| {
        gpui_kit::init(cx);
        Theme::change(gpui_kit::component::ThemeMode::Dark, None, cx);
        assert_eq!(item_colors(cx), dark);
        Theme::change(gpui_kit::component::ThemeMode::Light, None, cx);
        assert_eq!(item_colors(cx), light);
    });
}
