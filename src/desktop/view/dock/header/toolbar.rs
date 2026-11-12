//! A panel's toolbar, drawn in its header (Part G, R-13): Tickets' New
//! ticket…; Documents' New document…, New folder… and Collapse all.
//!
//! The buttons are the base Kit's, tracking handles the Workspace owns, so a
//! dock `load` or move, which changes the header's element paths, keeps
//! them (R-37). A focused button counts as its panel's region
//! (`Workspace::focused_region`), so its ring shows, F6 moves on from it and
//! `NewItem` makes its kind. The wrapper carries the `Navigation` key
//! context, for `secondary-n`, and stops a mouse-down from reaching the
//! header, so pressing a button never drags the panel.

use gpui_kit::{
    AnyElement, ClickEvent, Context, InteractiveElement, IntoElement, MouseButton, ParentElement,
    StatefulInteractiveElement, Styled, Window,
    base::Button,
    component::{ActiveTheme, Icon, h_flex, tooltip::Tooltip},
    prelude::FluentBuilder,
    px, transparent_black,
};

use crate::desktop::{
    assets::icon_path,
    keys::{NAVIGATION_CONTEXT, NewItem},
    model::{layout::PanelKind, navigation::Pane},
    view::{
        create::{NEW_DOCUMENT, NEW_TICKET},
        documents::{COLLAPSE_ALL, NEW_FOLDER},
        shell::tokens::chrome_colors,
        workspace::{ToolbarButton, Workspace, modal_open},
    },
};

/// A button's size (`03`): 26 by 24, radius 4, a 14 px icon.
const WIDTH: f32 = 26.;
const HEIGHT: f32 = 24.;
const RADIUS: f32 = 4.;
const ICON_SIZE: f32 = 14.;

impl ToolbarButton {
    /// The accessible name, and the tooltip.
    fn name(self) -> &'static str {
        match self {
            Self::NewTicket => NEW_TICKET,
            Self::NewDocument => NEW_DOCUMENT,
            Self::NewFolder => NEW_FOLDER,
            Self::CollapseAll => COLLAPSE_ALL,
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::NewTicket => "plus",
            Self::NewDocument => "file-plus",
            Self::NewFolder => "folder-plus",
            Self::CollapseAll => "chevrons-down-up",
        }
    }

    /// The New buttons' tooltips show the platform's key for `NewItem`.
    fn has_key(self) -> bool {
        matches!(self, Self::NewTicket | Self::NewDocument)
    }

    /// The panel's buttons, in order.
    fn of(kind: PanelKind) -> &'static [Self] {
        match kind {
            PanelKind::Tickets => &[Self::NewTicket],
            PanelKind::Documents => &[Self::NewDocument, Self::NewFolder, Self::CollapseAll],
            PanelKind::Repositories | PanelKind::Activity => &[],
        }
    }
}

impl Workspace {
    /// The toolbar of the panel `kind`, for its header to draw; nothing for
    /// a panel with no buttons.
    pub(in crate::desktop::view) fn panel_toolbar(
        &mut self,
        kind: PanelKind,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let buttons = ToolbarButton::of(kind);
        if buttons.is_empty() {
            return None;
        }
        let colors = chrome_colors(cx);
        let ring = cx.theme().ring;
        let buttons = buttons.iter().map(|button| {
            let button = *button;
            let enabled = self.toolbar_enabled(button);
            let tooltip = move |window: &mut Window, cx: &mut gpui_kit::App| {
                let tip = Tooltip::new(button.name());
                if button.has_key() {
                    tip.action(&NewItem, Some(NAVIGATION_CONTEXT))
                } else {
                    tip
                }
                .build(window, cx)
            };
            Button::new(button.id())
                .track_focus(self.toolbar_focus(button))
                .accessibility_label(button.name())
                .disabled(!enabled)
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.press_toolbar(button, window, cx)
                }))
                .flex_none()
                .w(px(WIDTH))
                .h(px(HEIGHT))
                .rounded(px(RADIUS))
                .border_2()
                .border_color(transparent_black())
                .text_color(colors.secondary)
                .when(enabled, |this| {
                    this.hover(|style| style.bg(colors.hover).text_color(colors.text))
                        .focus_visible(move |style| style.border_color(ring))
                        .cursor_pointer()
                })
                .when(!enabled, |this| this.opacity(0.4))
                .tooltip(tooltip)
                .child(
                    Icon::default()
                        .path(icon_path(button.icon()))
                        .w(px(ICON_SIZE))
                        .h(px(ICON_SIZE)),
                )
        });
        Some(
            h_flex()
                .flex_none()
                .gap_1()
                .pr(px(6.))
                .key_context(NAVIGATION_CONTEXT)
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .children(buttons)
                .into_any_element(),
        )
    }

    /// Whether `button` can act (Part G): the New buttons with a repository
    /// resolved; Collapse all with a folder open.
    fn toolbar_enabled(&self, button: ToolbarButton) -> bool {
        match button {
            ToolbarButton::NewTicket => self.tickets().repo().is_some(),
            ToolbarButton::NewDocument | ToolbarButton::NewFolder => {
                self.documents().repo().is_some()
            }
            ToolbarButton::CollapseAll => self.documents().any_folder_open(),
        }
    }

    /// A press of `button`, by click, Enter or Space: the action of the
    /// button's panel. Nothing acts behind a dialog or the details sheet
    /// (C1 contract 4), nor for a button that is disabled.
    pub(in crate::desktop::view) fn press_toolbar(
        &mut self,
        button: ToolbarButton,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if modal_open(window, cx) || !self.toolbar_enabled(button) {
            return;
        }
        match button {
            ToolbarButton::NewTicket => self.new_item(Pane::Tickets, window, cx),
            ToolbarButton::NewDocument => self.new_item(Pane::Documents, window, cx),
            ToolbarButton::NewFolder => self.new_folder(window, cx),
            ToolbarButton::CollapseAll => self.collapse_all_folders(window, cx),
        }
    }
}
