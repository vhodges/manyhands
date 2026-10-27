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
        /// Show the Drafts page: the drafts kept for recovery.
        OpenDrafts,
        /// Quit, once the running job has returned.
        Quit,
        /// Quit at once, without waiting for the running job.
        QuitNow,
        /// Open the selected row of the document tree. The ticket list is
        /// the Kit's, whose own `enter` binding opens its row.
        OpenSelected,
        /// Scroll the content region up by a page.
        ScrollPageUp,
        /// Scroll the content region down by a page.
        ScrollPageDown,
        /// Scroll the content region to its top.
        ScrollToTop,
        /// Scroll the content region to its bottom.
        ScrollToBottom,
        /// Close the Drafts page's Inspect panel.
        CloseInspect,
        /// Bring the shown item tab's discussion into view and focus its
        /// heading.
        JumpToDiscussion,
        /// Edit the shown item.
        #[cfg_attr(not(test), allow(dead_code))] // Handled from Task 14.
        Edit,
        /// Save the item being edited.
        #[cfg_attr(not(test), allow(dead_code))] // Handled from Task 15.
        Save,
        /// Switch the edit editor between Rich text and Markdown.
        ToggleRich,
        /// Escape in the edit editor: close an editor menu the user just
        /// opened, or else focus the header's Done (R-59).
        EditorEscape,
        /// Make the edit editor's selection a link.
        #[cfg_attr(not(test), allow(dead_code))] // Handled from Task 10b.
        InsertLink,
        /// New document or New ticket, the shown list's.
        #[cfg_attr(not(test), allow(dead_code))] // Handled from Task 23.
        NewItem,
    ]
);

/// The key context of the document tree's wrapper, where `enter` opens
/// the selected row.
pub(crate) const DOCUMENTS_CONTEXT: &str = "Documents";

/// The key context of the content region, where PageUp, PageDown, Home
/// and End scroll it. A focused control that binds them itself, such as
/// a text field, takes them first.
pub(crate) const CONTENT_CONTEXT: &str = "Content";

/// The key context of the Why blocked panel, where PageUp, PageDown, Home
/// and End scroll it.
pub(crate) const WHY_BLOCKED_CONTEXT: &str = "WhyBlocked";

/// The key context of the Drafts page's Inspect panel, where Escape
/// closes it.
pub(crate) const DRAFT_INSPECT_CONTEXT: &str = "DraftInspect";

/// The key context of the edit editor's host (`view::editor::host`), where
/// the mode key switches Rich text and Markdown.
pub(crate) const EDIT_HOST_CONTEXT: &str = "EditHost";

/// The edit editor itself, inside its host: Escape and the link key are
/// bound here, so read and comment editors never see them (R-59).
const EDIT_HOST_EDITOR: &str = "EditHost > Editor";

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
        KeyBinding::new("secondary-shift-d", OpenDrafts, None),
        KeyBinding::new("secondary-q", Quit, None),
        KeyBinding::new("enter", OpenSelected, Some(DOCUMENTS_CONTEXT)),
        KeyBinding::new("pageup", ScrollPageUp, Some(CONTENT_CONTEXT)),
        KeyBinding::new("pagedown", ScrollPageDown, Some(CONTENT_CONTEXT)),
        KeyBinding::new("home", ScrollToTop, Some(CONTENT_CONTEXT)),
        KeyBinding::new("end", ScrollToBottom, Some(CONTENT_CONTEXT)),
        KeyBinding::new("pageup", ScrollPageUp, Some(WHY_BLOCKED_CONTEXT)),
        KeyBinding::new("pagedown", ScrollPageDown, Some(WHY_BLOCKED_CONTEXT)),
        KeyBinding::new("home", ScrollToTop, Some(WHY_BLOCKED_CONTEXT)),
        KeyBinding::new("end", ScrollToBottom, Some(WHY_BLOCKED_CONTEXT)),
        KeyBinding::new("escape", CloseInspect, Some(DRAFT_INSPECT_CONTEXT)),
        // In the content region, the item tab's strip included.
        KeyBinding::new("secondary-shift-c", JumpToDiscussion, Some(CONTENT_CONTEXT)),
        // While editing, in the edit editor's host only.
        KeyBinding::new("escape", EditorEscape, Some(EDIT_HOST_EDITOR)),
        KeyBinding::new("secondary-k", InsertLink, Some(EDIT_HOST_EDITOR)),
        KeyBinding::new("secondary-shift-m", ToggleRich, Some(EDIT_HOST_CONTEXT)),
    ]);
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "keys_tests.rs"]
mod tests;
