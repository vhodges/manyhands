//! Actions and key bindings, defined once here (design, Layout And
//! Keyboard). `secondary` is Ctrl on Linux and Windows, Cmd on macOS.

use gpui_kit::{App, KeyBinding};

gpui_kit::actions!(
    manyhands,
    [
        /// Focus the next region.
        NextRegion,
        /// Focus the previous region.
        PreviousRegion,
        /// Focus the repository selector.
        FocusRepositories,
        /// Focus the documents and tickets navigation.
        FocusNavigation,
        /// Focus the content: the open tabs.
        FocusContent,
        /// Focus the operation area.
        FocusOperations,
        /// Find an item by short code or ID.
        Find,
        /// Show the next tab.
        NextTab,
        /// Show the previous tab.
        PreviousTab,
        /// Close the active tab.
        CloseTab,
        /// Refresh the selected repository's index.
        Refresh,
        /// Rebuild the selected repository's index.
        Rebuild,
        /// Open the settings.
        OpenSettings,
        /// Quit, once the running job has returned.
        Quit,
        /// Quit at once, without waiting for the running job.
        QuitNow,
        /// Open the selected row of the document tree. The ticket list is
        /// the Kit's, whose own `enter` binding opens its row.
        OpenSelected,
    ]
);

/// The key context of the document tree's wrapper, where `enter` opens
/// the selected row.
pub(crate) const DOCUMENTS_CONTEXT: &str = "Documents";

/// Binds the desktop's keys. Quit now has no key: it is the focused
/// Quit now button while quitting waits.
pub(crate) fn bind(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("f6", NextRegion, None),
        KeyBinding::new("shift-f6", PreviousRegion, None),
        KeyBinding::new("secondary-1", FocusRepositories, None),
        KeyBinding::new("secondary-2", FocusNavigation, None),
        KeyBinding::new("secondary-3", FocusContent, None),
        KeyBinding::new("secondary-4", FocusOperations, None),
        KeyBinding::new("secondary-p", Find, None),
        KeyBinding::new("ctrl-tab", NextTab, None),
        KeyBinding::new("ctrl-shift-tab", PreviousTab, None),
        KeyBinding::new("secondary-w", CloseTab, None),
        KeyBinding::new("secondary-r", Refresh, None),
        KeyBinding::new("secondary-shift-r", Rebuild, None),
        KeyBinding::new("secondary-,", OpenSettings, None),
        KeyBinding::new("secondary-q", Quit, None),
        KeyBinding::new("enter", OpenSelected, Some(DOCUMENTS_CONTEXT)),
    ]);
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "keys_tests.rs"]
mod tests;
