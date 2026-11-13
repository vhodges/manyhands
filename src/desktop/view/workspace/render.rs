//! Shell layout, action wiring and content scroll commands, and the bodies
//! of the dock's panels.

use super::{Region, Workspace};
use crate::desktop::{
    keys::{
        ActivityDown, ActivityEnd, ActivityEnter, ActivityHome, ActivityUp, CloseTab, DockPanel,
        Find as FindKey, FocusContent, FocusNavigation, FocusOperations, FocusRepositories,
        JumpToDiscussion, NewItem, NextRegion, NextTab, OpenDockMenu, OpenDrafts,
        OpenRepositoryMenu, OpenSelected, OpenSettings, PreviousRegion, PreviousTab, Quit, QuitNow,
        Rebuild, Refresh, RepositoryMenuChoice, ScrollPageDown, ScrollPageUp, ScrollToBottom,
        ScrollToTop, ToggleBottomDock, ToggleLeftDock, ToggleRightDock,
    },
    model::{
        layout::{PanelKind, Side},
        stamp::SettingsPage,
    },
    view::{
        activity::Step,
        dock::Slot,
        repositories,
        scroll::ContentScroll,
        shell::{title::window_title, title_bar},
    },
};
use gpui_kit::{
    AnyElement, App, Context, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{ActiveTheme, Root, WindowExt, v_flex},
    deferred, div,
};

impl Workspace {
    /// A page key: the shown item's scroll region moves, else the content
    /// region's column (R-47).
    fn scroll_content(&mut self, scroll: impl Fn(&ContentScroll) -> bool, cx: &mut Context<Self>) {
        if scroll(&self.scroll_target(cx)) {
            cx.notify();
        }
    }

    /// The body of the panel that draws `slot`, built when the panel
    /// renders (R-1): the same bodies as the fixed row drew, each inside its
    /// region.
    pub(in crate::desktop::view) fn region_body(
        &mut self,
        slot: Slot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match slot {
            Slot::Panel(PanelKind::Repositories) => self.repositories_region(window, cx),
            Slot::Panel(PanelKind::Documents) => {
                let documents = self.documents_body(cx);
                self.region(Region::Documents, window, cx, documents)
                    .size_full()
                    .into_any_element()
            }
            Slot::Panel(PanelKind::Tickets) => {
                let tickets = self.tickets_body(window, cx);
                self.region(Region::Tickets, window, cx, tickets)
                    .size_full()
                    .into_any_element()
            }
            Slot::Panel(PanelKind::Activity) => self.activity_region(window, cx),
            Slot::ItemArea => self.item_area_region(window, cx),
        }
    }

    fn repositories_region(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let repositories = repositories::body(&self.navigation, &self.repositories_pane, cx);
        self.region(Region::Repositories, window, cx, repositories)
            .size_full()
            .into_any_element()
    }

    /// The layout notices, the quit row and the list (C1, R-18). The list's
    /// container keeps the id `"operation-area"` (R-20), and the rows are
    /// applied to it after the frame.
    fn activity_region(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let finishing = self.finishing(window, cx);
        // The layout file's notices sit above the list (R-21).
        let notices = self.layout_notices(window, cx);
        let list = self.activity_list(window, cx);
        self.region(
            Region::Operations,
            window,
            cx,
            v_flex()
                .size_full()
                .gap_2()
                .children(notices)
                .children(finishing)
                .child(list),
        )
        .size_full()
        .into_any_element()
    }

    /// The content region with its tab strip, item view, Settings and Drafts
    /// pages and Root's sheet layer.
    fn item_area_region(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let item_shown = self.shown_item(cx).is_some();
        // The region's per-frame focus note, which its scroll-into-view
        // needs (R-33).
        self.content_scroll()
            .note_focus(window.focused(cx).as_ref());
        let content = v_flex()
            .id("content")
            .test_support()
            .gap_3()
            .size_full()
            .overflow_y_scroll()
            .track_scroll(self.content_scroll().handle());
        // Root's sheet layer is drawn inside the content region, whose item
        // tab opens the sheet: its focus then counts as the region's, and
        // its keys pass the region's context before the workspace's, where
        // R-60 refuses them. The sheet places itself over the whole window
        // (fact 61); deferred, it is painted above the rest of the window
        // and below dialogs, popups and tooltips, which are deferred higher.
        let sheet = Root::render_sheet_layer(window, cx)
            .map(|layer| div().absolute().child(deferred(layer)));
        let content = content
            .children(self.content_body(window, cx))
            .children(sheet);
        let content = self
            .region(Region::Content, window, cx, content)
            .size_full();
        // An item tab lies flush with the region's border (owner,
        // 2026-10-30). Settings, the Drafts page and the empty pane keep
        // the regions' padding.
        if item_shown {
            content.p_0().into_any_element()
        } else {
            content.into_any_element()
        }
    }
}

/// Whether a dialog or the details sheet is open (R-60).
pub(in crate::desktop::view) fn modal_open(window: &mut Window, cx: &mut App) -> bool {
    window.has_active_dialog(cx) || window.has_active_sheet(cx)
}

/// Except Quit, workspace actions must not escape a dialog's focus trap,
/// nor act behind the details sheet (R-60).
pub(in crate::desktop::view) fn unless_dialog<A: 'static>(
    cx: &mut Context<Workspace>,
    f: impl Fn(&mut Workspace, &A, &mut Window, &mut Context<Workspace>) + 'static,
) -> impl Fn(&A, &mut Window, &mut App) + 'static {
    cx.listener(move |this, action: &A, window, cx| {
        if !modal_open(window, cx) {
            f(this, action, window, cx)
        }
    })
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.apply_filter_reset(window, cx);
        self.sync_tickets(window, cx);
        self.apply_review(window, cx);
        self.open_deferred_republish(window, cx);
        self.sync_sheet(window, cx);
        if self.dock.queue_clamp() {
            // The area records its bounds as this frame is drawn, so the
            // clamp runs after it (R-37). `on_next_frame` is no hook: tests
            // never run it.
            cx.defer_in(window, |this, window, cx| this.clamp_first(window, cx));
        }
        if let Some(region) = self.focused_region(window, cx) {
            self.last_region = Some(region);
            // The list used last (R-8): a click or a key into a list.
            if let Some(pane) = region.pane() {
                self.navigation.set_pane(pane);
            }
        }
        // The rows follow the model after this frame, not during it (R-8);
        // the repository menu closes as soon as its row goes (R-23).
        let rows = repositories::rows(self, cx);
        let repository_menu = self.repository_menu_layer(&rows, window, cx);
        self.repositories_pane.show_after_frame(rows, window, cx);
        // The title bar draws the string the window is titled with, which is
        // set only when it changes (R-14).
        let title = window_title(self.navigation.selected_entry());
        if title != self.window_title {
            window.set_window_title(&title);
            self.window_title.clone_from(&title);
        }
        let title_bar = title_bar(title.into(), cx.entity().downgrade(), cx);
        let dock = div()
            .flex_1()
            .min_h_0()
            .w_full()
            .child(self.dock_area().clone());
        let footer = self.footer(window, cx);
        let dock_menu = self.dock_menu_layer(window, cx);
        // Root's first child draws its dialog and notification layers.
        v_flex()
            .id("workspace")
            .key_context("Workspace")
            .on_action(cx.listener(|this, _: &Quit, window, cx| {
                this.request_quit(window, cx);
            }))
            .on_action(cx.listener(|this, _: &QuitNow, _, cx| this.quit_now(cx)))
            // New document… or New ticket…, bound in the list regions only:
            // the focused list's kind.
            .on_action(unless_dialog(cx, |this, _: &NewItem, window, cx| {
                let list = this
                    .focused_region(window, cx)
                    .and_then(Region::pane)
                    .unwrap_or_else(|| this.navigation.pane());
                this.new_item(list, window, cx)
            }))
            .on_action(unless_dialog(cx, |this, _: &OpenSelected, _, cx| {
                this.open_selected_document(cx)
            }))
            .on_action(unless_dialog(cx, |this, _: &Refresh, _, cx| {
                this.maintain_index(false, cx);
            }))
            .on_action(unless_dialog(cx, |this, _: &Rebuild, _, cx| {
                this.maintain_index(true, cx);
            }))
            .on_action(unless_dialog(cx, |this, _: &NextRegion, window, cx| {
                this.cycle_region(1, window, cx)
            }))
            .on_action(unless_dialog(cx, |this, _: &PreviousRegion, window, cx| {
                this.cycle_region(-1, window, cx)
            }))
            .on_action(unless_dialog(
                cx,
                |this, _: &FocusRepositories, window, cx| {
                    this.focus_region(Region::Repositories, window, cx)
                },
            ))
            .on_action(unless_dialog(
                cx,
                |this, _: &FocusNavigation, window, cx| {
                    // The list used last.
                    let region = Region::of_pane(this.navigation.pane());
                    this.focus_region(region, window, cx)
                },
            ))
            .on_action(unless_dialog(cx, |this, _: &FocusContent, window, cx| {
                this.focus_region(Region::Content, window, cx)
            }))
            .on_action(unless_dialog(cx, |this, _: &OpenSettings, window, cx| {
                this.open_settings(SettingsPage::Repositories, window, cx)
            }))
            .on_action(unless_dialog(cx, |this, _: &OpenDrafts, window, cx| {
                this.open_drafts(None, window, cx)
            }))
            .on_action(unless_dialog(cx, |this, _: &FindKey, window, cx| {
                this.focus_find(window, cx)
            }))
            .on_action(unless_dialog(cx, |this, _: &NextTab, window, cx| {
                this.step_tab(1, window, cx)
            }))
            .on_action(unless_dialog(cx, |this, _: &PreviousTab, window, cx| {
                this.step_tab(-1, window, cx)
            }))
            .on_action(unless_dialog(cx, |this, _: &CloseTab, window, cx| {
                this.close_shown_tab(window, cx)
            }))
            .on_action(unless_dialog(
                cx,
                |this, _: &FocusOperations, window, cx| {
                    this.focus_region(Region::Operations, window, cx)
                },
            ))
            .on_action(unless_dialog(cx, |this, _: &ScrollPageUp, _, cx| {
                this.scroll_content(|scroll| scroll.page(-1.), cx)
            }))
            .on_action(unless_dialog(cx, |this, _: &ScrollPageDown, _, cx| {
                this.scroll_content(|scroll| scroll.page(1.), cx)
            }))
            .on_action(unless_dialog(cx, |this, _: &ScrollToTop, _, cx| {
                this.scroll_content(ContentScroll::to_top, cx)
            }))
            .on_action(unless_dialog(cx, |this, _: &ScrollToBottom, _, cx| {
                this.scroll_content(ContentScroll::to_bottom, cx)
            }))
            .on_action(unless_dialog(
                cx,
                |this, _: &JumpToDiscussion, window, cx| {
                    if let Some(pane) = this.shown_item(cx) {
                        pane.update(cx, |pane, cx| pane.jump_to_discussion(window, cx));
                    }
                },
            ))
            .on_action(unless_dialog(cx, |this, _: &ToggleLeftDock, window, cx| {
                this.toggle_dock(Side::Left, window, cx)
            }))
            .on_action(unless_dialog(
                cx,
                |this, _: &ToggleRightDock, window, cx| this.toggle_dock(Side::Right, window, cx),
            ))
            .on_action(unless_dialog(
                cx,
                |this, _: &ToggleBottomDock, window, cx| this.toggle_dock(Side::Bottom, window, cx),
            ))
            .on_action(unless_dialog(cx, |this, _: &OpenDockMenu, window, cx| {
                this.open_focused_dock_menu(window, cx)
            }))
            .on_action(unless_dialog(cx, |this, action: &DockPanel, window, cx| {
                this.dock_chosen(action.panel, action.side, window, cx)
            }))
            // Activity's list, in its own key context (C2).
            .on_action(unless_dialog(cx, |this, _: &ActivityUp, _, cx| {
                this.activity_step(Step::Up, cx)
            }))
            .on_action(unless_dialog(cx, |this, _: &ActivityDown, _, cx| {
                this.activity_step(Step::Down, cx)
            }))
            .on_action(unless_dialog(cx, |this, _: &ActivityHome, _, cx| {
                this.activity_step(Step::First, cx)
            }))
            .on_action(unless_dialog(cx, |this, _: &ActivityEnd, _, cx| {
                this.activity_step(Step::Last, cx)
            }))
            .on_action(unless_dialog(cx, |this, _: &ActivityEnter, window, cx| {
                this.activity_enter(window, cx)
            }))
            .on_action(unless_dialog(
                cx,
                |this, _: &OpenRepositoryMenu, window, cx| {
                    this.open_focused_repository_menu(window, cx)
                },
            ))
            .on_action(unless_dialog(
                cx,
                |this, action: &RepositoryMenuChoice, window, cx| {
                    this.repository_menu_chosen(action, window, cx)
                },
            ))
            .relative()
            .size_full()
            .bg(cx.theme().background)
            .child(title_bar)
            .child(dock)
            .child(footer)
            .children(dock_menu)
            .children(repository_menu)
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}
