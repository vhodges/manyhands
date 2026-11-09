//! The renderer of the dock area (R-2): everything the Kit's skin draws
//! except the group header, which is the shell's own.

mod tabs;

use std::{rc::Rc, sync::Arc};

use gpui_kit::{
    AnyElement, AnyView, App, Axis, Div, Empty, InteractiveElement, IntoElement, ParentElement,
    Stateful, Styled, Window,
    component::{
        ActiveTheme,
        dock::{
            BasePanelView, DockAreaRenderer, DockContext, DockSkin, DropIndicator, NodeId,
            PanelHandle, PanelState, TabGroupContext, TabGroupRenderer,
        },
    },
    div,
};

use crate::desktop::view::shell::tokens::chrome_colors;

/// Hands the area's frames, dock chrome and placeholder to the Kit's skin,
/// which keeps dock-edge resizing and the `LayoutChanged` that follows it
/// (facts 6 and 9). Only the group header is the shell's.
///
/// It is built inside the area's constructor, as `DockSkin::dock_area`
/// does, since the skin needs the area's own context.
pub(super) struct ShellRenderer {
    skin: Rc<DockSkin>,
}

impl ShellRenderer {
    pub(super) fn new(skin: Rc<DockSkin>) -> Self {
        Self { skin }
    }
}

impl DockAreaRenderer for ShellRenderer {
    fn frame(&self, window: &mut Window, cx: &mut App) -> Stateful<Div> {
        self.skin.frame(window, cx)
    }

    fn center_frame(&self, window: &mut Window, cx: &mut App) -> Stateful<Div> {
        self.skin.center_frame(window, cx)
    }

    fn split_frame(
        &self,
        node: NodeId,
        axis: Axis,
        window: &mut Window,
        cx: &mut App,
    ) -> Stateful<Div> {
        self.skin.split_frame(node, axis, window, cx)
    }

    fn render_dock(
        &self,
        dock: &DockContext,
        content: AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        self.skin.render_dock(dock, content, window, cx)
    }

    fn build_placeholder(
        &self,
        state: &PanelState,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Arc<dyn BasePanelView>> {
        self.skin.build_placeholder(state, window, cx)
    }

    fn tab_group_renderer(&self) -> Rc<dyn TabGroupRenderer> {
        Rc::new(PanelHeader {
            skin: self.skin.tab_group_renderer(),
        })
    }
}

/// A group's header and body: the header is `tabs::header`.
struct PanelHeader {
    /// The Kit's own group renderer, for the drop indicator it draws.
    skin: Rc<dyn TabGroupRenderer>,
}

impl TabGroupRenderer for PanelHeader {
    /// Without the skin's frame, whose zoom and close actions do not apply.
    /// The panels' bodies sit on `panel`; the item area keeps the theme's
    /// background.
    fn frame(&self, group: &TabGroupContext, _: &mut Window, cx: &mut App) -> Stateful<Div> {
        let own_chrome = group
            .active_panel()
            .and_then(PanelHandle::of)
            .is_some_and(|handle| !handle.title_bar(cx));
        let frame = div().id(("panel-group", group.node().as_u64()));
        if own_chrome {
            frame.bg(cx.theme().tokens.background)
        } else {
            frame.bg(chrome_colors(cx).panel)
        }
    }

    fn render_tab_bar(
        &self,
        group: &TabGroupContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        tabs::header(group, window, cx)
    }

    /// Uncached, so the panel sees the Workspace's changes (R-1); a collapsed
    /// group draws no panel, so no hidden control can take focus (R-35).
    fn render_active_panel(
        &self,
        panel: AnyView,
        group: &TabGroupContext,
        _: &mut Window,
        _: &mut App,
    ) -> AnyElement {
        if group.is_collapsed() {
            return Empty.into_any_element();
        }
        div()
            .flex_1()
            .min_h_0()
            .w_full()
            .child(panel)
            .into_any_element()
    }

    fn render_drop_indicator(
        &self,
        indicator: DropIndicator,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        self.skin.render_drop_indicator(indicator, window, cx)
    }
}
