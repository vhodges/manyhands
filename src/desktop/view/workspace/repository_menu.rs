//! A repository's menu (Part D): opened by a right click on a row and by
//! `menu` or `shift-f10`, its choice checked again and done, its
//! dismissal, and its layer. Every entry point refuses behind a dialog or
//! the details sheet (R-36): the actions through `unless_dialog`, the row's
//! right press through the opener's own guard.

use std::path::{Path, PathBuf};

use gpui_kit::{AnyElement, App, Context, FocusHandle, Point, Window};

use super::{Workspace, modal_open};
use crate::desktop::{
    keys::{RepositoryMenuChoice, Shown},
    model::{
        repository_state::{MenuChoice, RowAction},
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
            root, items, action, context, anchor, restore, window, cx,
        ));
        cx.notify();
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
            // Read, never chosen; Refresh and Rebuild are the More menu's.
            MenuChoice::Sentence | MenuChoice::Refresh | MenuChoice::Rebuild => {}
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
        if !rows.iter().any(|row| row.key == menu.root) {
            let context = menu.context.clone();
            self.repository_menu = None;
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
        self.repository_menu.as_ref().map(|menu| menu.root.clone())
    }

    pub(crate) fn remove_focus(&self) -> FocusHandle {
        self.remove_focus.clone()
    }
}
