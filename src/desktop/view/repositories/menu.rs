//! A repository's menu (Part D, R-11): a Kit `PopupMenu` the Workspace
//! draws deferred and anchored, as it draws the dock menu
//! (`view::footer::menu`); the Kit's `ContextMenu` has no keyboard path.
//!
//! It is built when it opens, from the row's `menu_items`. Each item's
//! choice carries what the item showed, and is checked again against the
//! rows when it is chosen (R-35), so a check that changed the row in
//! between cannot make it do something it did not say.

use gpui_kit::{
    Anchor, AnyElement, App, Bounds, Context, DismissEvent, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Pixels, Point, Styled, Subscription, Window,
    anchored, component::menu::PopupMenu, deferred, div, px,
};

use super::RepositoryRow;
use crate::desktop::{
    keys::{RepositoryMenuChoice, Shown},
    model::{
        repository_state::{MenuChoice, MenuItem, RowAction},
        stamp::RepoKey,
    },
    view::workspace::Workspace,
};

/// Where a repository's menu opens. Not `Anchor`, which is the Kit's.
#[derive(Clone, Copy, Debug)]
pub(crate) enum MenuAnchor {
    /// At the pointer: a right click on a row.
    Pointer(Point<Pixels>),
    /// Below a row, or the list, as drawn last: the menu keys.
    Bounds(Bounds<Pixels>),
}

/// What `item` shows: the state's action as its `RowAction` (`action`),
/// any other item as its label.
fn shown(item: &MenuItem, action: Option<&RowAction>) -> Shown {
    match (item.choice, action) {
        (MenuChoice::StateAction, Some(action)) => Shown::Action(action.clone()),
        _ => Shown::Label(item.label.clone()),
    }
}

/// The menu `rows` give the repository at `root`, with its state's action;
/// `None` when it is neither listed nor kept.
pub(crate) fn menu_of(
    rows: &[RepositoryRow],
    root: &RepoKey,
) -> Option<(Vec<MenuItem>, Option<RowAction>)> {
    rows.iter()
        .find(|row| row.key == *root)
        .map(|row| (row.menu.clone(), row.state.action.clone()))
}

/// Whether `choice` still stands against `rows` as they are now (R-35): its
/// repository is listed or kept, its item is still there and enabled, and
/// shows what it showed when the menu opened. A "Retry now" that has
/// become "Resume checking" or "Unlock…" does not.
pub(crate) fn still_offered(rows: &[RepositoryRow], choice: &RepositoryMenuChoice) -> bool {
    menu_of(rows, &choice.root).is_some_and(|(items, action)| {
        items.iter().any(|item| {
            item.choice == choice.choice
                && item.enabled
                && shown(item, action.as_ref()) == choice.shown
        })
    })
}

/// An open repository menu.
pub(crate) struct RepositoryMenu {
    /// Whose menu it is: it closes when the repository leaves the rows
    /// (R-23).
    pub(crate) root: RepoKey,
    view: Entity<PopupMenu>,
    /// The menu's top left corner.
    at: Point<Pixels>,
    /// Where it was opened from: the list, or a Repositories toolbar
    /// button. It is the menu's action context, so a choice and Escape
    /// put focus there, and Remove's dialog gives focus back to it (R-12,
    /// R-25).
    pub(crate) context: FocusHandle,
    /// Where focus was before a menu opened by the pointer, and where it
    /// goes back to when that menu closes without a choice.
    pub(crate) restore: Option<FocusHandle>,
    /// An item was chosen: its action has put focus where it belongs.
    pub(crate) chosen: bool,
    _dismissed: Subscription,
}

impl RepositoryMenu {
    /// Builds the menu of `root` from `items`, whose state's action is
    /// `action`; a separator comes before Remove from Manyhands…. The
    /// Workspace says what to do once it is dismissed.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn open(
        root: RepoKey,
        items: Vec<MenuItem>,
        action: Option<RowAction>,
        context: FocusHandle,
        anchor: MenuAnchor,
        restore: Option<FocusHandle>,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Self {
        let choices = root.clone();
        let view = PopupMenu::build(window, cx, |menu, _, _| {
            items
                .iter()
                .fold(menu.action_context(context.clone()), |menu, item| {
                    let menu = if item.choice == MenuChoice::Remove {
                        menu.separator()
                    } else {
                        menu
                    };
                    let choice = RepositoryMenuChoice {
                        root: choices.clone(),
                        choice: item.choice,
                        shown: shown(item, action.as_ref()),
                    };
                    menu.menu_with_disabled(item.label.clone(), Box::new(choice), !item.enabled)
                })
        });
        let dismissed = cx.subscribe_in(
            &view,
            window,
            |workspace, _, _: &DismissEvent, window, cx| {
                workspace.repository_menu_dismissed(window, cx)
            },
        );
        let at = match anchor {
            MenuAnchor::Pointer(at) => at,
            MenuAnchor::Bounds(bounds) => bounds.bottom_left(),
        };
        Self {
            root,
            view,
            at,
            context,
            restore,
            chosen: false,
            _dismissed: dismissed,
        }
    }

    /// The menu's top left corner.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn at(&self) -> Point<Pixels> {
        self.at
    }

    /// The menu drawn above everything but tooltips, anchored at its
    /// position and kept inside the window, with focus in it, as the dock
    /// menu is.
    pub(crate) fn layer(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        let focus = self.view.focus_handle(cx);
        if !focus.contains_focused(window, cx) {
            focus.focus(window, cx);
        }
        deferred(
            anchored().child(
                div()
                    .w(window.bounds().size.width)
                    .h(window.bounds().size.height)
                    .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                    .child(
                        anchored()
                            .position(self.at)
                            .snap_to_window_with_margin(px(8.))
                            .anchor(Anchor::TopLeft)
                            .child(self.view.clone()),
                    ),
            ),
        )
        .with_priority(gpui_kit::base::POPUP_PRIORITY)
        .into_any_element()
    }
}
