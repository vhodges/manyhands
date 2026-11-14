//! A row's menu (Wave 04 Cycle 03b, Part D, R-27): a Kit `PopupMenu` the
//! Workspace draws deferred and anchored, as it draws the repository menu
//! (`view::repositories::menu`). Its first items are the row's full message
//! and its detail lines, each disabled; a separator; then the row's offers,
//! the button's first, each enabled or disabled as the row says, Dismiss
//! last where the row can be dismissed.
//!
//! It is built when it opens, from the row as the pane drew it. Each offer's
//! choice carries that row's revision, and is checked again against the
//! log's row when chosen (`choose_activity`; R-46, R-55), so a change in
//! between cannot make it do something the row did not say.

use gpui_kit::{
    Anchor, AnyElement, App, Context, DismissEvent, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Pixels, Point, Styled, Subscription, Window,
    anchored,
    component::menu::{PopupMenu, PopupMenuItem},
    deferred, div, px,
};

use super::row::choice_of;
use crate::desktop::{
    model::activity::{
        copy::message_of,
        rows::{ActivityRow, RowKey},
    },
    view::{repositories::MenuAnchor, workspace::Workspace},
};

/// An open row menu.
pub(crate) struct ActivityMenu {
    /// Whose menu it is: it closes when the row leaves the log.
    pub(crate) key: RowKey,
    view: Entity<PopupMenu>,
    /// The menu's top left corner.
    at: Point<Pixels>,
    /// Where it was opened from: the list. It is the menu's action context,
    /// so a choice and Escape put focus there, and a dialog the choice opens
    /// gives focus back to it (C3).
    pub(crate) context: FocusHandle,
    /// Where focus was before a menu opened by the pointer, and where it
    /// goes back to when that menu closes without a choice.
    pub(crate) restore: Option<FocusHandle>,
    /// An item was chosen: its action has put focus where it belongs.
    pub(crate) chosen: bool,
    _dismissed: Subscription,
}

impl ActivityMenu {
    /// Builds the menu of `row`, as the pane drew it. The Workspace says what
    /// to do once it is dismissed.
    pub(crate) fn open(
        row: &ActivityRow,
        context: FocusHandle,
        anchor: MenuAnchor,
        restore: Option<FocusHandle>,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Self {
        let key = row.row.key.clone();
        let lines: Vec<String> = std::iter::once(message_of(&row.row))
            .chain(row.row.detail.iter().cloned())
            .collect();
        let view = PopupMenu::build(window, cx, |menu, _, _| {
            let menu = lines
                .iter()
                .fold(menu.action_context(context.clone()), |menu, text| {
                    // A read-only line: disabled, and it carries no action.
                    menu.item(PopupMenuItem::new(text.clone()).disabled(true))
                });
            let menu = if row.row.menu.is_empty() {
                menu
            } else {
                menu.separator()
            };
            row.row.menu.iter().fold(menu, |menu, offer| {
                menu.menu_with_disabled(
                    offer.label.clone(),
                    Box::new(choice_of(row, offer)),
                    !offer.enabled,
                )
            })
        });
        let dismissed = cx.subscribe_in(
            &view,
            window,
            |workspace, _, _: &DismissEvent, window, cx| {
                workspace.activity_menu_dismissed(window, cx)
            },
        );
        let at = match anchor {
            MenuAnchor::Pointer(at) => at,
            MenuAnchor::Bounds(bounds) => bounds.bottom_left(),
        };
        Self {
            key,
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
        popup_layer(&self.view, self.at, window, cx)
    }
}

/// A Kit menu drawn above everything but tooltips at `at`, kept inside the
/// window, with focus in it: the row menu's layer and the ⋯ menu's.
pub(super) fn popup_layer(
    view: &Entity<PopupMenu>,
    at: Point<Pixels>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let focus = view.focus_handle(cx);
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
                        .position(at)
                        .snap_to_window_with_margin(px(8.))
                        .anchor(Anchor::TopLeft)
                        .child(view.clone()),
                ),
        ),
    )
    .with_priority(gpui_kit::base::POPUP_PRIORITY)
    .into_any_element()
}
