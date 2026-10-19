//! Actions and key bindings.

use gpui_kit::{App, KeyBinding};

gpui_kit::actions!(
    manyhands,
    [
        /// Quit, once the running job has returned.
        Quit,
        /// Quit at once, without waiting for the running job.
        QuitNow,
    ]
);

/// Binds the desktop's keys. Task 4 adds the rest.
pub(crate) fn bind(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("secondary-q", Quit, None)]);
}
