//! Activity's ⋯ menu (Wave 04 Cycle 03b, C1): a Kit `PopupMenu` the
//! Workspace draws deferred and anchored below the button, as Cycle 03a's
//! More menu is. It has one item, "Clear finished", disabled when no row is
//! `clearable` (L8). Focus belongs to the button: a choice and Escape give
//! it back there.

use gpui_kit::{
    AnyElement, App, Bounds, Context, DismissEvent, Entity, FocusHandle, Pixels, Point,
    Subscription, Window, component::menu::PopupMenu,
};

use super::menu::popup_layer;
use crate::desktop::{
    keys::ClearFinished, model::activity::copy::CLEAR_FINISHED, view::workspace::Workspace,
};

/// The open ⋯ menu.
pub(crate) struct ActivityMore {
    view: Entity<PopupMenu>,
    /// The menu's top left corner: below the button, as it was drawn.
    at: Point<Pixels>,
    _dismissed: Subscription,
}

impl ActivityMore {
    /// Builds the menu, its one item enabled when `clearable` rows exist.
    /// `button` is where it opened from, and the menu's action context, so a
    /// choice and Escape put focus back on it; `bounds` is the button as
    /// drawn.
    pub(crate) fn open(
        clearable: bool,
        button: FocusHandle,
        bounds: Option<Bounds<Pixels>>,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Self {
        let view = PopupMenu::build(window, cx, |menu, _, _| {
            menu.action_context(button.clone()).menu_with_disabled(
                CLEAR_FINISHED,
                Box::new(ClearFinished),
                !clearable,
            )
        });
        let dismissed = cx.subscribe_in(
            &view,
            window,
            |workspace, _, _: &DismissEvent, window, cx| {
                workspace.activity_more_dismissed(window, cx)
            },
        );
        Self {
            view,
            at: bounds.map_or_else(Point::default, |bounds| bounds.bottom_left()),
            _dismissed: dismissed,
        }
    }

    pub(crate) fn layer(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        popup_layer(&self.view, self.at, window, cx)
    }
}
