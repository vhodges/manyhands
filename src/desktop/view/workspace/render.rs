//! Shell layout, action wiring and content scroll commands.

use super::{Region, Workspace};
use crate::desktop::{
    keys::{
        CloseTab, Find as FindKey, FocusContent, FocusNavigation, FocusOperations,
        FocusRepositories, NextRegion, NextTab, OpenDrafts, OpenSelected, OpenSettings,
        PreviousRegion, PreviousTab, Quit, QuitNow, Rebuild, Refresh, ScrollPageDown, ScrollPageUp,
        ScrollToBottom, ScrollToTop,
    },
    model::stamp::SettingsPage,
    view::{
        scroll::ContentScroll,
        sidebar::{self, RepositoryRow},
    },
};
use gpui_kit::{
    App, Context, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{ActiveTheme, Root, WindowExt, h_flex, v_flex},
    px,
};

impl Workspace {
    /// A page key: the shown item's scroll region moves, else the content
    /// region's column (R-47).
    fn scroll_content(&mut self, scroll: impl Fn(&ContentScroll) -> bool, cx: &mut Context<Self>) {
        if scroll(&self.scroll_target(cx)) {
            cx.notify();
        }
    }
}

/// Except Quit, workspace actions must not escape a dialog's focus trap.
pub(in crate::desktop::view) fn unless_dialog<A: 'static>(
    cx: &mut Context<Workspace>,
    f: impl Fn(&mut Workspace, &A, &mut Window, &mut Context<Workspace>) + 'static,
) -> impl Fn(&A, &mut Window, &mut App) + 'static {
    cx.listener(move |this, action: &A, window, cx| {
        if !window.has_active_dialog(cx) {
            f(this, action, window, cx)
        }
    })
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_tickets(window, cx);
        self.apply_review(window, cx);
        if let Some(region) = self.focused_region(window, cx) {
            self.last_region = Some(region);
        }
        let finishing = self.finishing(window, cx);
        self.repositories_pane
            .set(RepositoryRow::rows(&self.navigation), window, cx);
        let repositories = sidebar::repositories(&self.navigation, &self.repositories_pane, cx);
        // An item tab draws the selection at the end of its own scroll
        // region instead.
        let selection = self.shown_item(cx).is_none().then(|| {
            sidebar::selection(
                &self.navigation,
                &self.remove_focus,
                &self.retry_focus,
                self.content_scroll(),
                cx,
            )
            .into_any_element()
        });
        let navigation = self.navigation_pane(window, cx);
        self.content_scroll()
            .note_focus(window.focused(cx).as_ref());
        let content = v_flex()
            .id("content")
            .test_support()
            .gap_3()
            .size_full()
            .overflow_y_scroll()
            .track_scroll(self.content_scroll().handle());
        let content = content.children(self.content_body(selection, window, cx));
        let top = h_flex()
            .flex_1()
            .min_h_0()
            .w_full()
            .child(
                self.region(Region::Repositories, window, cx, repositories)
                    .w(px(260.))
                    .h_full(),
            )
            .child(
                self.region(Region::Navigation, window, cx, navigation)
                    .w(px(320.))
                    .h_full(),
            )
            .child(
                self.region(Region::Content, window, cx, content)
                    .flex_1()
                    .h_full(),
            );
        let session = self.session_section(window, cx);
        let pending = self.pending_section(window, cx);
        self.prune_abandon_focus();
        let operations = self
            .region(
                Region::Operations,
                window,
                cx,
                v_flex()
                    .id("operation-area")
                    .gap_2()
                    .size_full()
                    .overflow_y_scroll()
                    .children(finishing)
                    .child(
                        h_flex()
                            .gap_4()
                            .items_start()
                            .child(session.flex_1())
                            .child(pending.flex_1()),
                    ),
            )
            .w_full()
            .h(px(260.));
        // Root's first child draws its layers.
        v_flex()
            .id("workspace")
            .key_context("Workspace")
            .on_action(cx.listener(|this, _: &Quit, window, cx| {
                this.request_quit(window, cx);
            }))
            .on_action(cx.listener(|this, _: &QuitNow, _, cx| this.quit_now(cx)))
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
                    this.focus_region(Region::Navigation, window, cx)
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
            .relative()
            .size_full()
            .bg(cx.theme().background)
            .child(top)
            .child(operations)
            .children(Root::render_sheet_layer(window, cx))
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}
