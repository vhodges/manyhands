//! The dock menu (decision 8, R-11): a Kit `PopupMenu` the Workspace draws
//! deferred and anchored, as the Kit's own `ContextMenu` does internally.
//! `ContextMenuExt` opens on a right mouse-down only and has no keyboard
//! path, and `NativeMenu` is an operating system menu on macOS and Windows
//! that tests cannot drive.
//!
//! Its action context is the panel toggle's focus handle, so choosing an item
//! focuses the toggle, and then dispatches `DockPanel`.

use gpui_kit::{
    Anchor, AnyElement, App, Bounds, Context, DismissEvent, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Pixels, Point, Styled, Subscription, Window,
    anchored, component::menu::PopupMenu, deferred, div, px,
};

use crate::desktop::{
    keys::DockPanel,
    model::layout::{PanelKind, Side},
    view::workspace::Workspace,
};

/// The menu's items, in order: the label, the side it moves the panel to,
/// and whether that is the panel's current side (checked).
pub(crate) fn items(current: Side) -> [(&'static str, Side, bool); 3] {
    [
        ("Dock left", Side::Left),
        ("Dock right", Side::Right),
        ("Dock bottom", Side::Bottom),
    ]
    .map(|(label, side)| (label, side, side == current))
}

/// Where the menu opens.
#[derive(Clone, Copy, Debug)]
pub(in crate::desktop::view) enum Placement {
    /// At the pointer: a secondary click.
    Pointer(Point<Pixels>),
    /// At the toggle, as drawn last, with focus in the menu: the menu key.
    Toggle(Bounds<Pixels>),
}

/// An open dock menu.
pub(in crate::desktop::view) struct DockMenu {
    view: Entity<PopupMenu>,
    /// The menu's bottom left corner. The footer lies at the foot of the
    /// window, so the menu opens upwards from a toggle.
    at: Point<Pixels>,
    /// Where focus was before a menu opened by the pointer, and where it
    /// goes back to when that menu closes without a choice. A menu opened
    /// by key returns to its toggle through its action context.
    pub(in crate::desktop::view) restore: Option<FocusHandle>,
    /// An item was chosen: its action has put focus where it belongs.
    pub(in crate::desktop::view) chosen: bool,
    _dismissed: Subscription,
}

impl DockMenu {
    /// Builds the menu for `panel`, docked on `current`; `toggle` is its
    /// action context. The caller says what to do once it is dismissed.
    pub(in crate::desktop::view) fn open(
        panel: PanelKind,
        current: Side,
        toggle: FocusHandle,
        placement: Placement,
        restore: Option<FocusHandle>,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Self {
        let view = PopupMenu::build(window, cx, |menu, _, _| {
            items(current).into_iter().fold(
                menu.action_context(toggle),
                |menu, (label, side, checked)| {
                    menu.menu_with_check(label, checked, Box::new(DockPanel { panel, side }))
                },
            )
        });
        let dismissed = cx.subscribe_in(
            &view,
            window,
            |workspace, _, _: &DismissEvent, window, cx| workspace.dock_menu_dismissed(window, cx),
        );
        let at = match placement {
            Placement::Pointer(at) => at,
            Placement::Toggle(bounds) => bounds.origin,
        };
        Self {
            view,
            at,
            restore,
            chosen: false,
            _dismissed: dismissed,
        }
    }

    /// The menu drawn above everything but tooltips, anchored at its
    /// position and kept inside the window, with focus in it, as the Kit's
    /// context menu does.
    pub(in crate::desktop::view) fn layer(&self, window: &mut Window, cx: &mut App) -> AnyElement {
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
                            .anchor(Anchor::BottomLeft)
                            .child(self.view.clone()),
                    ),
            ),
        )
        .with_priority(gpui_kit::base::POPUP_PRIORITY)
        .into_any_element()
    }
}
