//! The Kit theme's slots the app sets (Wave 04 Cycle 05a design A2): every
//! primary button amber with `07`'s `on-accent` text, `05`'s secondary
//! button, one scrim colour, the Kit's focus ring in teal and the inputs'
//! `border-strong` border, in both modes.
//!
//! `apply` runs once, at startup, right after `gpui_kit::init`. For each
//! mode it clones the theme's stored config, sets these keys on it and
//! gives it to `Theme::apply_config`, so a later `Theme::change` re-applies
//! it with its name and highlight theme intact (R-2). `apply_config` also
//! sets the theme's mode to the config's, so the config of the mode the
//! theme is not in goes first and its own mode's last: the mode does not
//! change. Then `Theme::sync_base`, as the Kit asks after a change to the
//! global theme. Every colour is written `#rrggbb` or `#rrggbbaa`: one that
//! does not parse falls back to the Kit's default without a word.
//!
//! `05`'s secondary button is the Kit's default button, whose background
//! is `button.background` and whose border is `input.border`. `05` gives
//! only its resting colour, `hover`; its hover and press take `07`'s
//! `hover-strong`, "hover on hover". The Kit has no key for the inputs'
//! background (it is drawn from `input.border` and `background`), nor for
//! the sheet's border (the Kit's `border`, every divider's) or its shadow:
//! those stay the Kit's (K-13).

use std::rc::Rc;

use gpui_kit::{
    App, Hsla, Rgba, SharedString,
    component::{Theme, ThemeConfig},
};

use crate::desktop::view::shell::tokens::colors;

/// `05`'s dialog scrim, the one scrim colour the Kit has, for sheets too
/// (K-3).
const SCRIM: &str = "#0a0b0d99";

/// Sets the slots in both modes' configs, the current mode's last, and
/// leaves the theme in the mode it was in.
pub(crate) fn apply(cx: &mut App) {
    let theme = Theme::global(cx);
    let light = with_slots(&theme.light_theme);
    let dark = with_slots(&theme.dark_theme);
    let (other, current) = if theme.mode.is_dark() {
        (light, dark)
    } else {
        (dark, light)
    };
    let theme = Theme::global_mut(cx);
    theme.apply_config(&Rc::new(other));
    theme.apply_config(&Rc::new(current));
    Theme::sync_base(cx);
}

/// `config` with the slots set to `07`'s colours for its mode.
fn with_slots(config: &ThemeConfig) -> ThemeConfig {
    let tokens = colors(config.mode.is_dark());
    let mut config = config.clone();
    let slots = &mut config.colors;
    slots.button_primary = Some(hex(tokens.accent));
    slots.button_primary_hover = Some(hex(tokens.accent_hover));
    slots.button_primary_active = Some(hex(tokens.accent_hover));
    slots.button_primary_foreground = Some(hex(tokens.on_accent));
    slots.button = Some(hex(tokens.hover));
    slots.button_hover = Some(hex(tokens.hover_strong));
    slots.button_active = Some(hex(tokens.hover_strong));
    slots.overlay = Some(SCRIM.into());
    slots.ring = Some(hex(tokens.teal));
    slots.input = Some(hex(tokens.border_strong));
    config
}

/// `color` as `#rrggbb`, or `#rrggbbaa` when it is not opaque.
fn hex(color: Hsla) -> SharedString {
    let Rgba { r, g, b, a } = color.into();
    let byte = |channel: f32| (channel.clamp(0., 1.) * 255.).round() as u8;
    let (r, g, b, a) = (byte(r), byte(g), byte(b), byte(a));
    if a == u8::MAX {
        format!("#{r:02x}{g:02x}{b:02x}").into()
    } else {
        format!("#{r:02x}{g:02x}{b:02x}{a:02x}").into()
    }
}
