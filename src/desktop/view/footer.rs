//! The footer's controls (Part E): which controls the layout shows, in which
//! group, what each one is called, and how one is drawn.
//!
//! Each control is a Workspace-owned focus wrapper around a Kit button that
//! is not itself a tab stop, as Quit now is (R-30). The wrapper holds the
//! role, the accessible name and the toggled state, and its focus handle is
//! keyed by control, not by the group the control sits in, so it survives a
//! panel's move to another dock (fact 16). The Workspace wires the clicks
//! and the dock menu (`workspace/footer.rs`).

pub(in crate::desktop::view) mod menu;

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gpui_kit::{
    Action, App, Bounds, Context, FocusHandle, InteractiveElement, IntoElement, ParentElement,
    Pixels, Role, SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window,
    accesskit::Toggled,
    canvas,
    component::{
        ActiveTheme, Icon, Selectable, Sizable,
        button::{Button, ButtonVariants},
    },
    div, px,
};

use self::menu::DockMenu;
use crate::desktop::{
    assets::icon_path,
    keys::{OpenSettings, PANEL_TOGGLE_CONTEXT, ToggleBottomDock, ToggleLeftDock, ToggleRightDock},
    model::layout::{Layout, PanelKind, Side},
    view::{shell::tokens::chrome_colors, workspace::Workspace},
};

/// The footer's height (`02`).
pub(in crate::desktop::view) const FOOTER_HEIGHT: f32 = 28.;

pub(in crate::desktop::view) const SETTINGS: &str = "Settings";

/// One footer control.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Control {
    /// A dock's toggle: no toggle for a side with no panels (decision 6).
    Dock(Side),
    /// A panel's toggle, in the group of the dock it sits in.
    Panel(PanelKind),
    /// Hidden at 0.
    Attention,
    Settings,
}

impl Control {
    /// Every control the footer can hold, each with its own focus handle.
    pub(crate) const ALL: [Control; 9] = [
        Control::Dock(Side::Left),
        Control::Dock(Side::Right),
        Control::Dock(Side::Bottom),
        Control::Panel(PanelKind::Repositories),
        Control::Panel(PanelKind::Tickets),
        Control::Panel(PanelKind::Documents),
        Control::Panel(PanelKind::Activity),
        Control::Attention,
        Control::Settings,
    ];

    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Dock(Side::Left) => "footer-dock-left",
            Self::Dock(Side::Right) => "footer-dock-right",
            Self::Dock(Side::Bottom) => "footer-dock-bottom",
            Self::Panel(PanelKind::Repositories) => "footer-panel-repositories",
            Self::Panel(PanelKind::Tickets) => "footer-panel-tickets",
            Self::Panel(PanelKind::Documents) => "footer-panel-documents",
            Self::Panel(PanelKind::Activity) => "footer-panel-activity",
            Self::Attention => "footer-attention",
            Self::Settings => "footer-settings",
        }
    }

    /// The icon's name: the dock toggles' and Settings' are the Kit's, the
    /// panels' are R-23's.
    fn icon(self) -> &'static str {
        match self {
            Self::Dock(Side::Left) => "panel-left",
            Self::Dock(Side::Right) => "panel-right",
            Self::Dock(Side::Bottom) => "panel-bottom",
            Self::Panel(kind) => kind.icon(),
            Self::Attention => "circle-alert",
            Self::Settings => "settings",
        }
    }
}

/// "Hide Tickets" when the panel is displayed, else "Show Tickets": the
/// toggle's accessible name and its tooltip (R-24).
pub(crate) fn panel_toggle_name(kind: PanelKind, displayed: bool) -> String {
    format!("{} {}", verb(displayed), kind.label())
}

/// "Hide left dock" when the dock is open, else "Show left dock" (R-24).
pub(crate) fn dock_toggle_name(side: Side, open: bool) -> String {
    format!("{} {} dock", verb(open), side.name())
}

fn verb(on: bool) -> &'static str {
    if on { "Hide" } else { "Show" }
}

/// "1 needs attention" or "N need attention"; nothing at 0, where the
/// control is hidden (R-13).
pub(crate) fn attention_text(count: usize) -> Option<String> {
    match count {
        0 => None,
        1 => Some("1 needs attention".to_owned()),
        n => Some(format!("{n} need attention")),
    }
}

/// The controls of each footer side, in groups: a separator sits between
/// groups (`02`, "Footer").
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Plan {
    pub left: Vec<Vec<Control>>,
    pub right: Vec<Vec<Control>>,
}

/// Where the layout puts the controls: the left dock's toggle and panels,
/// then the bottom dock's; on the right, Needs attention (when `attention`
/// is above 0), the right dock's panels and toggle, and Settings. A side
/// with no panels has no toggle, and a panel's toggle follows it to its
/// dock's group (decision 6).
pub(crate) fn plan(layout: &Layout, attention: usize) -> Plan {
    let panels = |side: Side| -> Vec<Control> {
        layout
            .groups(side)
            .iter()
            .flat_map(|group| group.panels.iter().copied().map(Control::Panel))
            .collect()
    };
    let dock = |side: Side, toggle_first: bool| -> Vec<Control> {
        if !layout.has_panels(side) {
            return Vec::new();
        }
        let toggle = Control::Dock(side);
        let mut controls = panels(side);
        if toggle_first {
            controls.insert(0, toggle);
        } else {
            controls.push(toggle);
        }
        controls
    };
    let groups = |groups: Vec<Vec<Control>>| -> Vec<Vec<Control>> {
        groups
            .into_iter()
            .filter(|group| !group.is_empty())
            .collect()
    };
    Plan {
        left: groups(vec![dock(Side::Left, true), dock(Side::Bottom, true)]),
        right: groups(vec![
            if attention > 0 {
                vec![Control::Attention]
            } else {
                Vec::new()
            },
            dock(Side::Right, false),
            vec![Control::Settings],
        ]),
    }
}

/// The footer's state in the Workspace: a focus handle per control, where
/// each panel toggle was last drawn, and the open dock menu.
pub(in crate::desktop::view) struct Footer {
    pub(in crate::desktop::view) focus: HashMap<&'static str, FocusHandle>,
    /// Filled while drawing, for a menu that opens under its toggle.
    pub(in crate::desktop::view) bounds: Rc<RefCell<HashMap<&'static str, Bounds<Pixels>>>>,
    pub(in crate::desktop::view) menu: Option<DockMenu>,
}

impl Footer {
    pub(in crate::desktop::view) fn new(cx: &mut Context<Workspace>) -> Self {
        Self {
            focus: Control::ALL
                .map(|control| (control.id(), cx.focus_handle().tab_stop(true)))
                .into_iter()
                .collect(),
            bounds: Rc::default(),
            menu: None,
        }
    }

    pub(in crate::desktop::view) fn handle(&self, control: Control) -> &FocusHandle {
        &self.focus[control.id()]
    }
}

/// What a control shows.
pub(in crate::desktop::view) struct Face {
    pub control: Control,
    /// The wrapper's accessible name, and the tooltip.
    pub name: String,
    /// Text beside the icon, for Needs attention.
    pub text: Option<String>,
    /// A toggle's state; `None` for a control that is not a toggle.
    pub on: Option<bool>,
}

impl Face {
    /// The tooltip carries the key, when the control has an action.
    fn action(&self) -> Option<Box<dyn Action>> {
        match self.control {
            Control::Dock(Side::Left) => Some(Box::new(ToggleLeftDock)),
            Control::Dock(Side::Right) => Some(Box::new(ToggleRightDock)),
            Control::Dock(Side::Bottom) => Some(Box::new(ToggleBottomDock)),
            Control::Settings => Some(Box::new(OpenSettings)),
            Control::Panel(_) | Control::Attention => None,
        }
    }
}

/// The wrapper around a control's Kit button: the role, the accessible name,
/// the toggled state and the focus handle, which Enter and Space press as a
/// click does. A click leaves focus where it was (the button takes none).
pub(in crate::desktop::view) fn control(
    face: &Face,
    footer: &Footer,
    window: &Window,
    cx: &App,
) -> impl StatefulInteractiveElement + IntoElement + use<> {
    let colors = chrome_colors(cx);
    let handle = footer.handle(face.control);
    let on = face.on == Some(true);
    let tint = match (face.control, on) {
        (Control::Attention, _) => colors.red,
        (Control::Dock(_), true) => colors.secondary,
        (Control::Dock(_), false) => colors.faint,
        (_, true) => colors.text,
        (_, false) => colors.muted,
    };
    let mut button = Button::new((face.control.id(), 0usize))
        .ghost()
        .xsmall()
        .icon(Icon::default().path(icon_path(face.control.icon())))
        .selected(on)
        .text_color(tint)
        .tab_stop(false);
    if let Some(text) = &face.text {
        button = button.label(SharedString::from(text.clone()));
    }
    button = match face.action() {
        Some(action) => button.tooltip_with_action(face.name.clone(), action.as_ref(), None),
        None if face.control == Control::Attention => button,
        None => button.tooltip(face.name.clone()),
    };
    let mut wrapper = div()
        .id(face.control.id())
        .test_support()
        .track_focus(handle)
        .role(Role::Button)
        .aria_label(face.name.clone())
        .relative()
        .rounded_sm();
    if let Some(on) = face.on {
        wrapper = wrapper.aria_toggled(if on { Toggled::True } else { Toggled::False });
    }
    if matches!(face.control, Control::Panel(_)) {
        let bounds = footer.bounds.clone();
        let id = face.control.id();
        wrapper = wrapper.key_context(PANEL_TOGGLE_CONTEXT).child(
            canvas(
                move |at, _, _| {
                    bounds.borrow_mut().insert(id, at);
                },
                |_, (), _, _| {},
            )
            .absolute()
            .size_full(),
        );
    }
    let ring = handle.is_focused(window).then(|| {
        div()
            .id((face.control.id(), 1usize))
            .test_support()
            .absolute()
            .inset_0()
            .rounded_sm()
            .border_2()
            .border_color(cx.theme().ring)
    });
    wrapper.child(button).children(ring)
}

/// One thin vertical rule between footer groups.
pub(in crate::desktop::view) fn separator(cx: &App) -> gpui_kit::Div {
    div()
        .w(px(1.))
        .h(px(14.))
        .flex_none()
        .bg(chrome_colors(cx).border)
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "footer_tests/mod.rs"]
pub(in crate::desktop::view) mod tests;
