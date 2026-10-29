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
        Edit,
        /// Save the item being edited.
        Save,
        /// Switch the edit editor between Rich text and Markdown.
        ToggleRich,
        /// Escape in the edit editor: close an editor menu the user just
        /// opened, or else focus the header's Done (R-59).
        EditorEscape,
        /// Make the edit editor's selection a link: the toolbar asks for
        /// the address.
        InsertLink,
        /// New document or New ticket, the shown list's.
        #[cfg_attr(not(test), allow(dead_code))] // Called from Task 23.
        NewItem,
        /// Focus the edit toolbar's next control.
        ToolbarNext,
        /// Focus the edit toolbar's previous control.
        ToolbarPrevious,
        /// Focus the edit toolbar's first control.
        ToolbarFirst,
        /// Focus the edit toolbar's last control.
        ToolbarLast,
        /// Focus the toolbar menu's next item.
        MenuNext,
        /// Focus the toolbar menu's previous item.
        MenuPrevious,
        /// Focus the toolbar menu's first item.
        MenuFirst,
        /// Focus the toolbar menu's last item.
        MenuLast,
        /// Close the edit toolbar's open menu or link popover, and give
        /// focus back to where it was.
        ClosePopover,
        /// Tab or Shift-Tab in the toolbar's link popover: focus its other
        /// stop, the address or Insert, so focus stays in it.
        CycleLinkPopover,
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

/// The key context of an item tab (`view::item`): `secondary-shift-e`
/// edits while viewing, and the mode key and `secondary-s` reach the edit
/// editor from anywhere in the tab while editing.
pub(crate) const ITEM_TAB_CONTEXT: &str = "ItemTab";

/// The key context of an item tab's details sheet, which renders in Root's
/// layer outside the tab: `secondary-s` saves the tab from it (R-74).
pub(crate) const ITEM_SHEET_CONTEXT: &str = "ItemSheet";

/// The key context of an item tab's page, set only while it is viewing:
/// `e` edits there, the read editor included, whose guard stops the
/// letter's text and not its binding (fact 76). A dialog, the details
/// sheet, a text field outside the page, the toolbar and the edit editor
/// are outside it.
pub(crate) const ITEM_PAGE_CONTEXT: &str = "ItemPage";

/// The key context of the edit editor's host (`view::editor::host`), where
/// the mode key switches Rich text and Markdown.
pub(crate) const EDIT_HOST_CONTEXT: &str = "EditHost";

/// The edit editor itself, inside its host: Escape and the link key are
/// bound here, so read and comment editors never see them (R-59).
const EDIT_HOST_EDITOR: &str = "EditHost > Editor";

/// The key context of the edit toolbar's row (`view::editor::toolbar`):
/// Left, Right, Home and End move along it (fact 67), and the mode key
/// reaches the host from a toolbar control. It sits inside the content
/// region, so its Home and End come before the region's.
pub(crate) const TOOLBAR_CONTEXT: &str = "EditToolbar";

/// The key context of the toolbar's Style menu: Up, Down, Home and End
/// move through it, and Escape closes it. Left and Right, the toolbar's,
/// do nothing there.
pub(crate) const TOOLBAR_MENU_CONTEXT: &str = "EditToolbarMenu";

/// The key context of the toolbar's link popover: Escape closes it, and Tab
/// and Shift-Tab stay in it, between its address and Insert.
pub(crate) const TOOLBAR_LINK_CONTEXT: &str = "EditToolbarLink";

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
        // Edit: `e` on the page while viewing, `secondary-shift-e` in the
        // item tab. The mode key reaches the editor from the whole tab while
        // editing; the tab forwards it to the host.
        KeyBinding::new("e", Edit, Some(ITEM_PAGE_CONTEXT)),
        KeyBinding::new("secondary-shift-e", Edit, Some(ITEM_TAB_CONTEXT)),
        KeyBinding::new("secondary-shift-m", ToggleRich, Some(ITEM_TAB_CONTEXT)),
        // Save, from anywhere in the tab while editing; the tab lets it go
        // on otherwise.
        KeyBinding::new("secondary-s", Save, Some(ITEM_TAB_CONTEXT)),
        // And from the tab's details sheet, outside it (R-74).
        KeyBinding::new("secondary-s", Save, Some(ITEM_SHEET_CONTEXT)),
        // The edit toolbar: its controls, its Style menu and its link
        // popover. The mode key is the host's, forwarded from a control.
        KeyBinding::new("left", ToolbarPrevious, Some(TOOLBAR_CONTEXT)),
        KeyBinding::new("right", ToolbarNext, Some(TOOLBAR_CONTEXT)),
        KeyBinding::new("home", ToolbarFirst, Some(TOOLBAR_CONTEXT)),
        KeyBinding::new("end", ToolbarLast, Some(TOOLBAR_CONTEXT)),
        KeyBinding::new("secondary-shift-m", ToggleRich, Some(TOOLBAR_CONTEXT)),
        KeyBinding::new("up", MenuPrevious, Some(TOOLBAR_MENU_CONTEXT)),
        KeyBinding::new("down", MenuNext, Some(TOOLBAR_MENU_CONTEXT)),
        KeyBinding::new("home", MenuFirst, Some(TOOLBAR_MENU_CONTEXT)),
        KeyBinding::new("end", MenuLast, Some(TOOLBAR_MENU_CONTEXT)),
        KeyBinding::new("escape", ClosePopover, Some(TOOLBAR_MENU_CONTEXT)),
        KeyBinding::new("escape", ClosePopover, Some(TOOLBAR_LINK_CONTEXT)),
        KeyBinding::new("tab", CycleLinkPopover, Some(TOOLBAR_LINK_CONTEXT)),
        KeyBinding::new("shift-tab", CycleLinkPopover, Some(TOOLBAR_LINK_CONTEXT)),
    ]);
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "keys_tests.rs"]
mod tests;
