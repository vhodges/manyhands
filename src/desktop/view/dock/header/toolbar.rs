//! A panel's toolbar, drawn in its header (Part G, R-13): Tickets' New
//! ticket…; Documents' New document…, New folder… and Collapse all;
//! Repositories' Add or create a repository…, Check all and More;
//! Activity's filter and ⋯.
//!
//! The buttons are the base Kit's, tracking handles the Workspace owns, so a
//! dock `load` or move, which changes the header's element paths, keeps
//! them (R-37). A focused button counts as its panel's region
//! (`Workspace::focused_region`), so its ring shows, F6 moves on from it and
//! `NewItem` makes its kind. The wrapper carries the `Navigation` key
//! context, for `secondary-n`, and the Repositories panel's own context,
//! for `menu` and `shift-f10` (R-14, R-40); it stops a mouse-down from
//! reaching the header, so pressing a button never drags the panel.

use gpui_kit::{
    AnyElement, App, ClickEvent, Context, InteractiveElement, IntoElement, MouseButton,
    ParentElement, StatefulInteractiveElement, Styled, TestSupportExt, Toggled, Window,
    base::Button,
    canvas,
    component::{ActiveTheme, Icon, h_flex, tooltip::Tooltip},
    div,
    prelude::FluentBuilder,
    px, transparent_black,
};

use crate::desktop::{
    assets::icon_path,
    keys::{NAVIGATION_CONTEXT, NewItem, REPOSITORIES_CONTEXT},
    model::{activity::copy::SHOW_ONLY_ATTENTION, layout::PanelKind, navigation::Pane},
    view::{
        create::{NEW_DOCUMENT, NEW_TICKET},
        documents::{COLLAPSE_ALL, NEW_FOLDER},
        repositories::{ADD_OR_CREATE, CHECK_ALL, MORE, check_all_roots},
        settings::SettingsRoute,
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
            Self::AddRepository => ADD_OR_CREATE,
            Self::CheckAll => CHECK_ALL,
            Self::RepositoriesMore | Self::ActivityMore => MORE,
            Self::ActivityFilter => SHOW_ONLY_ATTENTION,
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::NewTicket => "plus",
            Self::NewDocument => "file-plus",
            Self::NewFolder => "folder-plus",
            Self::CollapseAll => "chevrons-down-up",
            Self::AddRepository => "plus",
            Self::CheckAll => "refresh-cw",
            Self::RepositoriesMore | Self::ActivityMore => "ellipsis",
            Self::ActivityFilter => "list-filter",
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
            PanelKind::Repositories => {
                &[Self::AddRepository, Self::CheckAll, Self::RepositoriesMore]
            }
            PanelKind::Activity => &[Self::ActivityFilter, Self::ActivityMore],
        }
    }
}

impl Workspace {
    /// The toolbar of the panel `kind`, for its header to draw; nothing for
    /// a panel with no buttons.
    pub(in crate::desktop::view) fn panel_toolbar(
        &mut self,
        kind: PanelKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let buttons = ToolbarButton::of(kind);
        if buttons.is_empty() {
            return None;
        }
        let colors = chrome_colors(cx);
        let ring = cx.theme().ring;
        let more_bounds = self.more_bounds_cell();
        let activity_more_bounds = self.activity_more_bounds_cell();
        let filtered = self.activity_filter();
        let buttons = buttons.iter().map(|button| {
            let button = *button;
            let enabled = self.toolbar_enabled(button, cx);
            if button == ToolbarButton::CheckAll && !enabled {
                // Check all disabled itself while it held focus (R-22):
                // after this frame, as focus is not moved while drawing.
                cx.defer_in(window, |this, window, cx| {
                    this.release_check_all(window, cx)
                });
            }
            let bounds = match button {
                ToolbarButton::RepositoriesMore => Some(more_bounds.clone()),
                ToolbarButton::ActivityMore => Some(activity_more_bounds.clone()),
                _ => None,
            };
            let tooltip = move |window: &mut Window, cx: &mut gpui_kit::App| {
                let tip = Tooltip::new(button.name());
                if button.has_key() {
                    tip.action(&NewItem, Some(NAVIGATION_CONTEXT))
                } else {
                    tip
                }
                .build(window, cx)
            };
            let element = Button::new(button.id())
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
                );
            if button == ToolbarButton::ActivityFilter {
                // The state is the wrapper's, as the footer toggles' are.
                return div()
                    .id("activity-filter")
                    .test_support()
                    .flex_none()
                    .aria_toggled(if filtered {
                        Toggled::True
                    } else {
                        Toggled::False
                    })
                    .child(element)
                    .into_any_element();
            }
            let Some(bounds) = bounds else {
                return element.into_any_element();
            };
            // Where a menu opens: below the button, as drawn.
            div()
                .relative()
                .flex_none()
                .child(element)
                .child(
                    canvas(move |at, _, _| bounds.set(Some(at)), |_, (), _, _| {})
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full(),
                )
                .into_any_element()
        });
        // Activity's buttons have no key of their own: `NewItem` makes the
        // focused list's kind, and Activity is no list of items.
        let context = match kind {
            PanelKind::Repositories => Some(REPOSITORIES_CONTEXT),
            PanelKind::Activity => None,
            _ => Some(NAVIGATION_CONTEXT),
        };
        Some(
            h_flex()
                .flex_none()
                .gap_1()
                .pr(px(6.))
                .when_some(context, |this, context| this.key_context(context))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .children(buttons)
                .into_any_element(),
        )
    }

    /// Whether `button` can act (Parts E and G): the New buttons with a
    /// repository resolved; Collapse all with a folder open; Check all when
    /// one repository qualifies (R-13); Add and More always, More's items
    /// disabled with no repository chosen.
    fn toolbar_enabled(&self, button: ToolbarButton, cx: &App) -> bool {
        match button {
            ToolbarButton::NewTicket => self.tickets().repo().is_some(),
            ToolbarButton::NewDocument | ToolbarButton::NewFolder => {
                self.documents().repo().is_some()
            }
            ToolbarButton::CollapseAll => self.documents().any_folder_open(),
            ToolbarButton::AddRepository
            | ToolbarButton::RepositoriesMore
            | ToolbarButton::ActivityFilter
            | ToolbarButton::ActivityMore => true,
            ToolbarButton::CheckAll => !check_all_roots(self, cx).is_empty(),
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
        if modal_open(window, cx) || !self.toolbar_enabled(button, cx) {
            return;
        }
        match button {
            ToolbarButton::NewTicket => self.new_item(Pane::Tickets, window, cx),
            ToolbarButton::NewDocument => self.new_item(Pane::Documents, window, cx),
            ToolbarButton::NewFolder => self.new_folder(window, cx),
            ToolbarButton::CollapseAll => self.collapse_all_folders(window, cx),
            // On the Add folder (W-6; Wave 04 Cycle 04 design Part D).
            ToolbarButton::AddRepository => {
                self.show_settings(SettingsRoute::AddRepository, None, window, cx)
            }
            ToolbarButton::CheckAll => self.check_all(window, cx),
            ToolbarButton::RepositoriesMore => self.open_more_menu(window, cx),
            ToolbarButton::ActivityFilter => self.toggle_activity_filter(cx),
            ToolbarButton::ActivityMore => self.open_activity_more(window, cx),
        }
    }
}
