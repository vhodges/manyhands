//! The footer (Part E) and the dock menu, wired to the layout operations.
//! Every control does nothing while a dialog or the details sheet is open
//! (R-30): the guard is the one the workspace actions have.

use gpui_kit::{
    AnyElement, ClickEvent, Context, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    StatefulInteractiveElement, Styled, Window,
    component::{WindowExt, status_bar::StatusBar},
    px,
};

use super::Workspace;
use crate::desktop::{
    model::{
        attention::needs_attention,
        layout::{PanelKind, Side},
        stamp::SettingsPage,
    },
    view::{
        footer::{
            Control, FOOTER_HEIGHT, Face, SETTINGS, attention_text, control, dock_toggle_name,
            menu::{DockMenu, Placement},
            panel_toggle_name, plan, separator,
        },
        shell::tokens::chrome_colors,
    },
};

impl Workspace {
    /// How many things await the person (decisions 3 and 14).
    pub(in crate::desktop::view) fn attention(&self, cx: &gpui_kit::App) -> usize {
        needs_attention(
            self.services.operations().read(cx),
            self.services.drafts().read(cx),
        )
    }

    /// The footer, below the dock area.
    pub(in crate::desktop::view) fn footer(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let colors = chrome_colors(cx);
        let plan = plan(&self.layout, self.attention(cx));
        let mut bar = StatusBar::new()
            .h(px(FOOTER_HEIGHT))
            .w_full()
            .flex_none()
            .py_0()
            .bg(colors.chrome)
            .border_color(colors.border);
        for (ix, group) in plan.left.iter().enumerate() {
            if ix > 0 {
                bar = bar.left(separator(cx));
            }
            for control in group {
                bar = bar.left(self.footer_control(*control, window, cx));
            }
        }
        for (ix, group) in plan.right.iter().enumerate() {
            if ix > 0 {
                bar = bar.right(separator(cx));
            }
            for control in group {
                bar = bar.right(self.footer_control(*control, window, cx));
            }
        }
        bar
    }

    fn footer_control(
        &mut self,
        which: Control,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let layout = &self.layout;
        let face = match which {
            Control::Dock(side) => {
                let open = layout.is_open(side);
                Face {
                    control: which,
                    name: dock_toggle_name(side, open),
                    text: None,
                    on: Some(open),
                }
            }
            Control::Panel(kind) => {
                let displayed = layout.is_displayed(kind);
                Face {
                    control: which,
                    name: panel_toggle_name(kind, displayed),
                    text: None,
                    on: Some(displayed),
                }
            }
            Control::Attention => {
                let text = attention_text(self.attention(cx)).unwrap_or_default();
                Face {
                    control: which,
                    name: text.clone(),
                    text: Some(text),
                    on: None,
                }
            }
            Control::Settings => Face {
                control: which,
                name: SETTINGS.to_owned(),
                text: None,
                on: None,
            },
        };
        let wrapper = control(&face, &self.footer, window, cx).on_click(
            cx.listener(move |this, _: &ClickEvent, window, cx| this.press(which, window, cx)),
        );
        match which {
            // A secondary click opens the dock menu at the pointer. It is
            // taken before the controls inside can take focus, which a
            // secondary click leaves where it was.
            Control::Panel(kind) => wrapper
                .capture_any_mouse_down(cx.listener(
                    move |this, event: &MouseDownEvent, window, cx| {
                        if event.button == MouseButton::Right {
                            window.prevent_default();
                            let at = Placement::Pointer(event.position);
                            this.open_dock_menu(kind, at, window, cx);
                        }
                    },
                ))
                .into_any_element(),
            _ => wrapper.into_any_element(),
        }
    }

    /// A control pressed by a click, Enter or Space (Part B's rules). It does
    /// nothing while a dialog or the details sheet is open (R-30).
    pub(in crate::desktop::view) fn press(
        &mut self,
        which: Control,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) || window.has_active_sheet(cx) {
            return;
        }
        match which {
            Control::Panel(kind) => self.toggle_panel(kind, window, cx),
            Control::Dock(side) => self.toggle_dock(side, window, cx),
            Control::Attention => self.reveal_and_focus(PanelKind::Activity, window, cx),
            Control::Settings => self.open_settings(SettingsPage::Repositories, window, cx),
        }
    }

    /// Opens the dock menu of `kind`'s toggle, replacing one that is open.
    /// A pointer's menu remembers where focus was.
    pub(in crate::desktop::view) fn open_dock_menu(
        &mut self,
        kind: PanelKind,
        placement: Placement,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) || window.has_active_sheet(cx) {
            return;
        }
        let Some(current) = self.layout.side_of(kind) else {
            return;
        };
        let toggle = self.footer.handle(Control::Panel(kind)).clone();
        let restore = match (&self.footer.menu, placement) {
            (Some(open), _) => open.restore.clone(),
            (None, Placement::Pointer(_)) => window.focused(cx),
            (None, Placement::Toggle(_)) => None,
        };
        self.footer.menu = Some(DockMenu::open(
            kind, current, toggle, placement, restore, window, cx,
        ));
        cx.notify();
    }

    /// `menu` or `shift-f10` on a focused panel toggle: its menu opens at
    /// the toggle, with focus in it.
    pub(in crate::desktop::view) fn open_focused_dock_menu(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let focused = PanelKind::ALL
            .into_iter()
            .find(|kind| self.footer.handle(Control::Panel(*kind)).is_focused(window));
        let Some(kind) = focused else {
            return;
        };
        let bounds = self
            .footer
            .bounds
            .borrow()
            .get(Control::Panel(kind).id())
            .copied();
        if let Some(bounds) = bounds {
            self.open_dock_menu(kind, Placement::Toggle(bounds), window, cx);
        }
    }

    /// The menu's choice: it has put focus on the toggle, which stays there
    /// after the panel's move.
    pub(in crate::desktop::view) fn dock_chosen(
        &mut self,
        panel: PanelKind,
        side: Side,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(menu) = &mut self.footer.menu {
            menu.chosen = true;
        }
        self.dock_panel(panel, side, window, cx);
    }

    /// The menu closed: by a choice, Escape or a click elsewhere.
    pub(in crate::desktop::view) fn dock_menu_dismissed(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(menu) = self.footer.menu.take() else {
            return;
        };
        // Closed with no choice: back where focus was before a pointer's
        // menu opened (the Kit returns it to the toggle).
        if let Some(previous) = menu.restore.filter(|_| !menu.chosen) {
            window.focus(&previous, cx);
        }
        cx.notify();
    }

    /// The open menu's layer, drawn by the root. A dialog or the sheet that
    /// opens over a menu closes it without a choice: its layer would take
    /// focus back from the modal on every draw (P18).
    pub(in crate::desktop::view) fn dock_menu_layer(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if window.has_active_dialog(cx) || window.has_active_sheet(cx) {
            self.footer.menu = None;
            return None;
        }
        self.footer.menu.as_ref().map(|menu| menu.layer(window, cx))
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
impl Workspace {
    /// A footer control's focus handle.
    pub(crate) fn footer_focus(&self, control: Control) -> gpui_kit::FocusHandle {
        self.footer.handle(control).clone()
    }
}
