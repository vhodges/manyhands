//! A row's menu in the Workspace (Part D): opened by a right press on a
//! row and by `shift-f10` and `menu` on the highlighted one, its choice
//! checked again and done, its dismissal, and its layer. Every entry point
//! refuses behind a dialog or the details sheet (R-60): the keys through
//! `unless_dialog`, the row's right press through the opener's own guard.
//! One layer at a time (R-13): opening the menu closes the repository menu
//! and the dock menu, and each of them closes it.

use gpui_kit::{AnyElement, Context, Pixels, Point, Window};

use super::{Workspace, modal_open};
use crate::desktop::{
    keys::ActivityMenuChoice,
    model::activity::rows::RowKey,
    view::{activity::menu::ActivityMenu, repositories::MenuAnchor},
};

impl Workspace {
    /// A right press on the row `key`: it is highlighted, and its menu opens
    /// at the pointer. It remembers where focus was, for Escape; a menu
    /// already open passes on what it remembered.
    pub(in crate::desktop::view) fn open_activity_menu_at(
        &mut self,
        key: RowKey,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if modal_open(window, cx) {
            return;
        }
        let context = self.activity.pane.list_focus().clone();
        let restore = match &self.activity.menu {
            Some(open) => open.restore.clone(),
            None => window.focused(cx),
        };
        self.show_activity_menu(key, MenuAnchor::Pointer(at), context, restore, window, cx);
    }

    /// `menu` or `shift-f10` in Activity's list: the highlighted row's menu,
    /// below the row where it was drawn, else at the list.
    pub(in crate::desktop::view) fn open_focused_activity_menu(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(key) = self.activity.pane.highlighted_key().cloned() else {
            return;
        };
        let pane = &self.activity.pane;
        let anchor = pane
            .row_bounds(&key)
            .or_else(|| pane.list_bounds())
            .map_or(MenuAnchor::Pointer(Point::default()), MenuAnchor::Bounds);
        let context = pane.list_focus().clone();
        self.show_activity_menu(key, anchor, context, None, window, cx);
    }

    /// Opens the menu of the row `key`, as the pane drew it, replacing any
    /// layer that is open, when no modal is open.
    fn show_activity_menu(
        &mut self,
        key: RowKey,
        anchor: MenuAnchor,
        context: gpui_kit::FocusHandle,
        restore: Option<gpui_kit::FocusHandle>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if modal_open(window, cx) {
            return;
        }
        let Some(row) = self.activity.pane.row(&key).cloned() else {
            return;
        };
        self.activity.pane.highlight(&key);
        self.repository_menu = None;
        self.footer.menu = None;
        self.activity.more = None;
        self.activity.menu = Some(ActivityMenu::open(
            &row, context, anchor, restore, window, cx,
        ));
        cx.notify();
    }

    /// An item chosen. It acts only while the log's row still offers what
    /// the row showed when the menu opened (`choose_activity`, T8, T11,
    /// T12); either way the menu is marked chosen, since the Kit has put
    /// focus on the list, where it stays.
    pub(in crate::desktop::view) fn activity_menu_chosen(
        &mut self,
        choice: &ActivityMenuChoice,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let origin = match &mut self.activity.menu {
            Some(menu) => {
                menu.chosen = true;
                menu.context.clone()
            }
            None => self.activity.pane.list_focus().clone(),
        };
        self.activity.pane.highlight(&choice.key);
        self.choose_activity(choice, origin, window, cx);
        cx.notify();
    }

    /// The menu closed: by a choice, Escape or a click elsewhere. Closed
    /// with no choice, a pointer's menu gives focus back to where it was
    /// (the Kit gives it to the list).
    pub(in crate::desktop::view) fn activity_menu_dismissed(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(menu) = self.activity.menu.take() else {
            return;
        };
        if let Some(previous) = menu.restore.filter(|_| !menu.chosen) {
            window.focus(&previous, cx);
        }
        cx.notify();
    }

    /// The open menu's layer, drawn by the root. A dialog or the sheet that
    /// opens over it closes it without a choice; so does its row leaving the
    /// log, which gives focus to the list.
    pub(in crate::desktop::view) fn activity_menu_layer(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if modal_open(window, cx) {
            self.activity.menu = None;
            return None;
        }
        let menu = self.activity.menu.as_ref()?;
        if !self
            .activity
            .log
            .rows()
            .iter()
            .any(|row| row.row.key == menu.key)
        {
            let context = menu.context.clone();
            self.activity.menu = None;
            // Now, not deferred: `focus_lost` fires in this draw and only
            // focuses a region.
            window.focus(&context, cx);
            return None;
        }
        Some(menu.layer(window, cx))
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
impl Workspace {
    /// The row whose menu is open.
    pub(crate) fn activity_menu_key(&self) -> Option<RowKey> {
        self.activity.menu.as_ref().map(|menu| menu.key.clone())
    }

    /// Whether a row menu is open. The sweeps read it through here and the
    /// menu entity, never by the Kit's shared `"popup-menu"` id.
    pub(crate) fn activity_menu_open(&self) -> bool {
        self.activity.menu.is_some()
    }

    /// Where the open row menu's top left corner is.
    pub(crate) fn activity_menu_at(&self) -> Option<Point<Pixels>> {
        self.activity.menu.as_ref().map(ActivityMenu::at)
    }
}
