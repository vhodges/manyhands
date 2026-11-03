//! The editor's keys (R-8, R-59), bound at startup and in the test shell,
//! because read editors need them too.

use gpui_kit::{App, KeyBinding, NoAction, Unbind};

/// The key context the editor binds its keys in.
const EDITOR: &str = "Editor";

/// Binds every editing key of the editor, then takes two back:
/// - underline, which writes HTML (`<u>`);
/// - Escape's `Dismiss`, which never propagates, so Escape reaches the
///   bindings around the editor. The editor's menus still close on a click
///   outside them.
pub(crate) fn bind(cx: &mut App) {
    zorite_editor::bind_keys(cx);
    cx.bind_keys([
        KeyBinding::new("cmd-u", NoAction, Some(EDITOR)),
        KeyBinding::new("ctrl-u", NoAction, Some(EDITOR)),
        KeyBinding::new(
            "escape",
            Unbind("zorite_editor::Dismiss".into()),
            Some(EDITOR),
        ),
    ]);
}
