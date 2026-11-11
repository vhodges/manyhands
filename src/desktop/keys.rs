//! Actions and key bindings, defined once here (design, Layout And
//! Keyboard). `secondary` is Ctrl on Linux and Windows, Cmd on macOS.

use gpui_kit::{App, KeyBinding, NoAction};

use crate::desktop::model::layout::{PanelKind, Side};

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
        /// Select the first row of the ticket list or the document tree.
        SelectFirst,
        /// Select the last row of the ticket list or the document tree.
        SelectLast,
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
        /// New document or New ticket, the focused list's: Documents or
        /// Tickets.
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
        /// Close the details sheet's ticket picker, and give focus back to
        /// the Add… or Change… that opened it.
        ClosePicker,
        /// Share the shown item: `item sync` (design, Lane B, Share
        /// controls).
        ShareItem,
        /// Post the comment composer's text.
        PostComment,
        /// Escape in a comment composer: cancel it, asking first when it
        /// has text (R-59 as amended: the body editor's Escape still
        /// focuses the header).
        CancelComposer,
        /// Switch a comment composer between Rich text and Markdown.
        ComposerToggleMode,
        /// Save resolution, in the resolve conflict sheet (D5 B3).
        SaveResolution,
        /// Show or hide the left dock.
        ToggleLeftDock,
        /// Show or hide the right dock.
        ToggleRightDock,
        /// Show or hide the bottom dock.
        ToggleBottomDock,
        /// Open the dock menu of the focused panel toggle.
        OpenDockMenu,
        /// Escape in the sheet's Result editor: back to its tab stop, the
        /// sheet left open (R-31).
        LeaveResult,
    ]
);

/// The dock menu's choice (R-12): `panel` moves to the end of `side`'s dock
/// and is revealed. Data for the menu's items; no key and no keymap file
/// name it, hence `no_json`.
#[derive(Clone, PartialEq, Debug, gpui_kit::Action)]
#[action(namespace = manyhands, no_json)]
pub(crate) struct DockPanel {
    pub panel: PanelKind,
    pub side: Side,
}

/// The key context of the Documents and Tickets regions (the tickets list
/// with its find bar and filters): `secondary-n` opens the focused list's
/// New document… or New ticket… there, and nowhere else.
pub(crate) const NAVIGATION_CONTEXT: &str = "Navigation";

/// The key context of a footer panel toggle (`view::footer`): `menu` and
/// `shift-f10` open its dock menu there, and nowhere else.
pub(crate) const PANEL_TOGGLE_CONTEXT: &str = "PanelToggle";

/// The key context of the document tree's wrapper, where `enter` opens
/// the selected row.
pub(crate) const DOCUMENTS_CONTEXT: &str = "Documents";

/// The key context of the ticket list's wrapper, where Home and End select
/// the first and last row. The filter input, outside it, keeps its own.
pub(crate) const TICKET_LIST_CONTEXT: &str = "TicketList";

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

/// The key context of the details sheet's ticket picker, inside it:
/// Escape closes it, not the sheet, which binds Escape itself.
pub(crate) const RELATION_PICKER_CONTEXT: &str = "RelationPicker";

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

/// The key context of a comment composer (`view::item::composer`):
/// `secondary-enter` posts, Escape cancels, and the mode key switches Rich
/// text and Markdown. Tab and Shift-Tab indent and outdent in its editor,
/// as the editor binds them; F6 and Shift-F6 (`NextRegion`,
/// `PreviousRegion`) leave it.
pub(crate) const COMMENT_COMPOSER_CONTEXT: &str = "CommentComposer";

/// A comment composer's editor, inside the composer: the editor binds
/// Escape itself (dismiss) in its deeper `Editor` context, so the
/// composer's cancel is bound here too, as `EDIT_HOST_EDITOR` binds the
/// edit editor's Escape.
const COMMENT_COMPOSER_EDITOR: &str = "CommentComposer > Editor";

/// The key context of the resolve conflict sheet (D5 B3): `secondary-enter`
/// saves, from its Result editor too, where Enter is a newline.
pub(crate) const RESOLVE_SHEET_CONTEXT: &str = "ResolveSheet";

/// The sheet's Result editor, inside the sheet: its Escape goes back to the
/// editor's tab stop rather than closing the sheet (R-31).
const RESOLVE_SHEET_EDITOR: &str = "ResolveSheet > Editor";

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
        // The dock keys, in the workspace. An editor's own `ctrl-b` (Bold) is
        // bound in its "Editor" context, which lies deeper, so it wins in an
        // editable editor whichever is registered first; a read-only editor
        // has it unbound (`editor::keys`, decision 2).
        KeyBinding::new("secondary-b", ToggleLeftDock, Some("Workspace")),
        KeyBinding::new("secondary-alt-b", ToggleRightDock, Some("Workspace")),
        KeyBinding::new("secondary-j", ToggleBottomDock, Some("Workspace")),
        // On a focused panel toggle: its dock menu.
        KeyBinding::new("menu", OpenDockMenu, Some(PANEL_TOGGLE_CONTEXT)),
        KeyBinding::new("shift-f10", OpenDockMenu, Some(PANEL_TOGGLE_CONTEXT)),
        KeyBinding::new("enter", OpenSelected, Some(DOCUMENTS_CONTEXT)),
        // The first and last row of a list; the document tree handles them
        // from Task 6 of Cycle 02.
        KeyBinding::new("home", SelectFirst, Some(TICKET_LIST_CONTEXT)),
        KeyBinding::new("end", SelectLast, Some(TICKET_LIST_CONTEXT)),
        KeyBinding::new("home", SelectFirst, Some(DOCUMENTS_CONTEXT)),
        KeyBinding::new("end", SelectLast, Some(DOCUMENTS_CONTEXT)),
        // New document… or New ticket…, the focused list's, in either
        // list's region (both carry this context) and nowhere else.
        KeyBinding::new("secondary-n", NewItem, Some(NAVIGATION_CONTEXT)),
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
        // The sheet's ticket picker, inside the sheet.
        KeyBinding::new("escape", ClosePicker, Some(RELATION_PICKER_CONTEXT)),
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
        // A comment composer, inside the item tab: its keys come before the
        // tab's. Tab stays the editor's (indent); F6 leaves the composer.
        // Escape is bound in its editor too, over the editor's own.
        KeyBinding::new(
            "secondary-enter",
            PostComment,
            Some(COMMENT_COMPOSER_CONTEXT),
        ),
        KeyBinding::new("escape", CancelComposer, Some(COMMENT_COMPOSER_CONTEXT)),
        KeyBinding::new(
            "secondary-shift-m",
            ComposerToggleMode,
            Some(COMMENT_COMPOSER_CONTEXT),
        ),
        KeyBinding::new("escape", CancelComposer, Some(COMMENT_COMPOSER_EDITOR)),
        KeyBinding::new("secondary-k", InsertLink, Some(COMMENT_COMPOSER_EDITOR)),
        KeyBinding::new(
            "secondary-enter",
            SaveResolution,
            Some(RESOLVE_SHEET_CONTEXT),
        ),
        // The dialog's Enter (its Confirm, which the sheet ignores) is
        // taken back inside the sheet, so Enter presses the focused button,
        // as Space does (A21); the Result editor's Enter, bound deeper, is
        // still its newline.
        KeyBinding::new("enter", NoAction, Some(RESOLVE_SHEET_CONTEXT)),
        KeyBinding::new("escape", LeaveResult, Some(RESOLVE_SHEET_EDITOR)),
    ]);
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "keys_tests.rs"]
mod tests;
