//! A repository's menu (Part D): opened by a right click on a row and by
//! `menu` or `shift-f10`, its choice checked again and done, its
//! dismissal, and its layer; and the toolbar's More menu and Check all
//! (Part E). Every entry point refuses behind a dialog or the details
//! sheet (R-36): the actions through `unless_dialog`, the row's right press
//! through the opener's own guard, the toolbar's through `press_toolbar`.

use std::path::{Path, PathBuf};

use std::{cell::Cell, rc::Rc};

use gpui_kit::{AnyElement, App, Bounds, Context, FocusHandle, Pixels, Point, Window};

use super::{Region, ToolbarButton, Workspace, modal_open};
use crate::desktop::{
    keys::{RepositoryMenuChoice, Shown},
    model::{
        repository_state::{MenuChoice, RowAction, more_items},
        stamp::{RepoKey, SettingsPage},
    },
    view::{
        item::open_folder,
        repositories::{
            self, MenuAnchor, RepositoryMenu, RepositoryRow,
            menu::{menu_of, still_offered},
        },
    },
};

impl Workspace {
    /// The Repositories list's focus handle: where a row's menu was opened
    /// from, and where Remove's dialog gives focus back.
    pub(in crate::desktop::view) fn repositories_list_focus(&self, cx: &App) -> FocusHandle {
        self.repositories_pane.focus_handle(cx)
    }

    /// A right press on the row of `root`: its menu at the pointer, the
    /// choice unchanged. It remembers where focus was, for Esc; a menu
    /// already open passes on what it remembered.
    pub(in crate::desktop::view) fn open_repository_menu(
        &mut self,
        root: RepoKey,
        anchor: MenuAnchor,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let context = self.repositories_list_focus(cx);
        let restore = match &self.repository_menu {
            Some(open) => open.restore.clone(),
            None => window.focused(cx),
        };
        self.show_repository_menu(root, anchor, context, restore, window, cx);
    }

    /// `menu` or `shift-f10` in the Repositories context. From the list,
    /// the Kit cursor's row, else the chosen one; from a toolbar button,
    /// always the chosen one, wherever the cursor rests (R-40). The row is
    /// scrolled into view and the menu anchored where it was drawn, or at
    /// the list when it has not been (R-42).
    pub(in crate::desktop::view) fn open_focused_repository_menu(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let list = self.repositories_list_focus(cx);
        let chosen = self.navigation.selected().cloned();
        let (root, context) = if list.is_focused(window) {
            (self.repositories_pane.highlighted(cx).or(chosen), list)
        } else {
            (chosen, window.focused(cx).unwrap_or(list))
        };
        let Some(root) = root else {
            return;
        };
        let pane = &self.repositories_pane;
        pane.scroll_to(&root, window, cx);
        let anchor = pane
            .row_bounds(&root)
            .or_else(|| pane.list_bounds())
            .map_or(MenuAnchor::Pointer(Point::default()), MenuAnchor::Bounds);
        self.show_repository_menu(root, anchor, context, None, window, cx);
    }

    /// Opens the menu of `root`, replacing one that is open, when no modal
    /// is open and the repository is listed or kept.
    fn show_repository_menu(
        &mut self,
        root: RepoKey,
        anchor: MenuAnchor,
        context: FocusHandle,
        restore: Option<FocusHandle>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if modal_open(window, cx) {
            return;
        }
        let rows = repositories::rows(self, cx);
        let Some((items, action)) = menu_of(&rows, &root) else {
            return;
        };
        self.repository_menu = Some(RepositoryMenu::open(
            Some(root),
            items,
            action,
            context,
            anchor,
            restore,
            window,
            cx,
        ));
        cx.notify();
    }

    /// The More button's press (Part E): the More menu for the chosen
    /// repository, below the button, with every item disabled when none is
    /// chosen. Focus belongs to the button: a choice and Escape give it
    /// back there, and Remove's dialog does too (R-25).
    pub(in crate::desktop::view) fn open_more_menu(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if modal_open(window, cx) {
            return;
        }
        let button = self.toolbar_focus(ToolbarButton::RepositoriesMore).clone();
        let chosen = self.navigation.selected().cloned();
        let anchor = self
            .repositories_pane
            .more_bounds()
            .map_or(MenuAnchor::Pointer(Point::default()), MenuAnchor::Bounds);
        let items = more_items(chosen.as_ref());
        self.repository_menu = Some(RepositoryMenu::open(
            chosen, items, None, button, anchor, None, window, cx,
        ));
        cx.notify();
    }

    /// The cell the More button records where it was drawn into.
    pub(in crate::desktop::view) fn more_bounds_cell(&self) -> Rc<Cell<Option<Bounds<Pixels>>>> {
        self.repositories_pane.more_bounds_cell()
    }

    /// Check all (Part E, R-13): one Check now for each repository it takes,
    /// in list order, each through `Services::check_now`, which refuses one
    /// already checking. The toolbar's press refuses behind a dialog or the
    /// sheet before it gets here.
    pub(in crate::desktop::view) fn check_all(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut sent = false;
        for root in repositories::check_all_roots(self, cx) {
            sent |= self.services.check_now(Path::new(&root));
        }
        if sent {
            cx.notify();
        }
        self.release_check_all(window, cx);
    }

    /// Check all took nothing more while it held focus: focus moves to the
    /// list, as Collapse all's does (R-22).
    pub(in crate::desktop::view) fn release_check_all(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .toolbar_focus(ToolbarButton::CheckAll)
            .is_focused(window)
            && repositories::check_all_roots(self, cx).is_empty()
        {
            self.focus_region(Region::Repositories, window, cx);
        }
    }

    /// A menu item chosen. It acts only while it still stands: its
    /// repository listed or kept, the item enabled and showing what it
    /// showed (R-35). The Kit has put focus where the menu was opened from.
    pub(in crate::desktop::view) fn repository_menu_chosen(
        &mut self,
        choice: &RepositoryMenuChoice,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let opened_from = match &mut self.repository_menu {
            Some(menu) => {
                menu.chosen = true;
                menu.context.clone()
            }
            None => self.repositories_list_focus(cx),
        };
        if !still_offered(&repositories::rows(self, cx), choice) {
            return;
        }
        let root = choice.root.clone();
        match choice.choice {
            // Read, never chosen.
            MenuChoice::Sentence => {}
            // The More menu's, for the chosen repository: a menu built for
            // another is ignored, as `maintain_index` acts on the chosen.
            MenuChoice::Refresh | MenuChoice::Rebuild => {
                if self.navigation.selected() == Some(&root) {
                    self.maintain_index(choice.choice == MenuChoice::Rebuild, cx);
                }
            }
            MenuChoice::StateAction => {
                if let Shown::Action(action) = &choice.shown {
                    self.row_action(action, root, window, cx);
                }
            }
            MenuChoice::CheckNow => self.menu_check_now(&root, cx),
            MenuChoice::ShareMain => {
                let remote = self.publication_remote(&root);
                self.share_main(root, remote, window, cx);
            }
            MenuChoice::OpenFolder => open_folder(Path::new(&root), cx),
            MenuChoice::Settings => {
                self.open_settings_on(SettingsPage::Repositories, PathBuf::from(root), window, cx)
            }
            MenuChoice::Remove => self.remove_repository(root, opened_from, window, cx),
        }
    }

    /// The state's action of the repository at `root` (Part B).
    fn row_action(
        &mut self,
        action: &RowAction,
        root: RepoKey,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let scheduler = self.services.scheduler().clone();
        match action {
            RowAction::Retry => self.retry_resolution(root, cx),
            RowAction::Reread => scheduler.update(cx, |scheduler, cx| scheduler.reread(root, cx)),
            RowAction::Unlock(key) => {
                scheduler.update(cx, |scheduler, cx| scheduler.unlock(*key, cx));
            }
            RowAction::Attend(action, reason) => self.attend(*action, reason, &root, window, cx),
            RowAction::CheckNow | RowAction::RetryNow => self.menu_check_now(&root, cx),
            RowAction::Settings => {
                self.open_settings_on(SettingsPage::Repositories, PathBuf::from(root), window, cx)
            }
            RowAction::SetUpSharing => self.open_remotes(PathBuf::from(root), window, cx),
        }
    }

    fn menu_check_now(&mut self, root: &str, cx: &mut Context<Self>) {
        if self.services.check_now(Path::new(root)) {
            cx.notify();
        }
    }

    /// Whether the repository at `root`, listed or kept, names a
    /// publication remote.
    fn publication_remote(&self, root: &RepoKey) -> bool {
        let navigation = &self.navigation;
        navigation
            .repositories()
            .rows()
            .into_iter()
            .flatten()
            .chain(navigation.kept())
            .find(|entry| entry.key() == root)
            .is_some_and(|entry| entry.summary.configuration.publication_remote.is_some())
    }

    /// The menu closed: by a choice, Escape or a click elsewhere. Closed
    /// with no choice, a pointer's menu gives focus back to where it was
    /// (the Kit gives it to the list).
    pub(in crate::desktop::view) fn repository_menu_dismissed(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(menu) = self.repository_menu.take() else {
            return;
        };
        if let Some(previous) = menu.restore.filter(|_| !menu.chosen) {
            window.focus(&previous, cx);
        }
        cx.notify();
    }

    /// The open menu's layer, drawn by the root. A dialog or the sheet that
    /// opens over it closes it without a choice, as the dock menu's; so
    /// does its repository leaving `rows` (R-23), which gives focus back to
    /// where it was opened from.
    pub(in crate::desktop::view) fn repository_menu_layer(
        &mut self,
        rows: &[RepositoryRow],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if modal_open(window, cx) {
            self.repository_menu = None;
            return None;
        }
        let menu = self.repository_menu.as_ref()?;
        if let Some(root) = &menu.root
            && !rows.iter().any(|row| row.key == *root)
        {
            let context = menu.context.clone();
            self.repository_menu = None;
            // Now, not deferred: `focus_lost` fires in this draw and only focuses a region.
            window.focus(&context, cx);
            return None;
        }
        Some(menu.layer(window, cx))
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
impl Workspace {
    /// The repository whose menu is open.
    pub(crate) fn repository_menu_root(&self) -> Option<RepoKey> {
        self.repository_menu
            .as_ref()
            .and_then(|menu| menu.root.clone())
    }

    /// Whether a repository menu is open, More with no repository chosen
    /// included (it has no root).
    pub(crate) fn repository_menu_open(&self) -> bool {
        self.repository_menu.is_some()
    }

    /// Where the open repository menu's top left corner is.
    pub(crate) fn repository_menu_at(&self) -> Option<gpui_kit::Point<gpui_kit::Pixels>> {
        self.repository_menu.as_ref().map(RepositoryMenu::at)
    }
}
