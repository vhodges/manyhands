//! The main window's view: the shell of four focusable regions, the
//! repository selector, and quitting.
//!
//! The regions are focus groups: the repository list, the documents and
//! tickets navigation, the content and the operation area. `F6` and
//! `shift-F6` cycle them; `secondary-1` to `secondary-4` focus each. The
//! focused region draws a focus ring, and is the only one that does.
//!
//! The Quit action and the window's close request go through one
//! handler. With no job running it quits. With one running it refuses,
//! stops the worker admitting jobs, shows "Finishing" and Quit now, and
//! quits when the job returns. Quit now quits at once: the worker thread
//! is not waited for, and the library's journals recover what it was
//! doing.

use std::path::PathBuf;

use gpui_kit::{
    App, Context, FocusHandle, InteractiveElement, IntoElement, ParentElement, Render, Styled,
    Subscription, Task, TestSupportExt, Window,
    component::{
        ActiveTheme, Root,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    div, px,
};

use super::sidebar;
use crate::desktop::{
    keys::{
        FocusContent, FocusNavigation, FocusOperations, FocusRepositories, NextRegion,
        PreviousRegion, Quit, QuitNow,
    },
    model::{
        navigation::{Navigation, Pane},
        stamp::Stamp,
    },
    services::{ReadKind, Services, requests::DesktopRequestId, worker::QuitState},
};

/// Where quitting stands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum QuitPhase {
    Running,
    /// Asked to quit; waiting for this job to return.
    Finishing(DesktopRequestId),
    /// Quitting.
    Quit,
}

/// A focus group of the shell, in `F6` order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Region {
    Repositories,
    Navigation,
    Content,
    Operations,
}

pub(crate) const REGIONS: [Region; 4] = [
    Region::Repositories,
    Region::Navigation,
    Region::Content,
    Region::Operations,
];

impl Region {
    fn index(self) -> usize {
        REGIONS
            .iter()
            .position(|region| *region == self)
            .expect("every region is listed")
    }

    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Repositories => "region-repositories",
            Self::Navigation => "region-navigation",
            Self::Content => "region-content",
            Self::Operations => "region-operations",
        }
    }

    /// The focus ring drawn while the region holds focus.
    pub(crate) fn ring_id(self) -> &'static str {
        match self {
            Self::Repositories => "region-repositories-focus-ring",
            Self::Navigation => "region-navigation-focus-ring",
            Self::Content => "region-content-focus-ring",
            Self::Operations => "region-operations-focus-ring",
        }
    }

    fn key_context(self) -> &'static str {
        match self {
            Self::Repositories => "Repositories",
            Self::Navigation => "Navigation",
            Self::Content => "Content",
            Self::Operations => "Operations",
        }
    }
}

pub(crate) struct Workspace {
    services: Services,
    /// One handle per region, in `REGIONS` order.
    regions: [FocusHandle; 4],
    navigation: Navigation,
    /// The repository list read in flight; replacing it drops the last.
    list_read: Option<Task<()>>,
    /// The selection's resolution in flight; replacing it drops the last.
    resolve_read: Option<Task<()>>,
    quit: QuitPhase,
    _operations: Subscription,
}

impl Workspace {
    pub(crate) fn new(services: Services, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let regions = [
            cx.focus_handle(),
            cx.focus_handle(),
            cx.focus_handle(),
            cx.focus_handle(),
        ];
        window.focus(&regions[Region::Repositories.index()], cx);
        let operations = cx.observe(services.operations(), |this, _, cx| {
            this.operations_changed(cx)
        });
        let workspace = cx.entity().downgrade();
        window.on_window_should_close(cx, move |_, cx| {
            workspace
                .update(cx, |workspace, cx| workspace.request_quit(cx))
                .unwrap_or(true)
        });
        let mut workspace = Self {
            services,
            regions,
            navigation: Navigation::default(),
            list_read: None,
            resolve_read: None,
            quit: QuitPhase::Running,
            _operations: operations,
        };
        workspace.load_repositories(cx);
        workspace
    }

    #[cfg(test)]
    pub(crate) fn services(&self) -> &Services {
        &self.services
    }

    #[cfg(test)]
    pub(crate) fn navigation(&self) -> &Navigation {
        &self.navigation
    }

    #[cfg(test)]
    pub(crate) fn quit_phase(&self) -> QuitPhase {
        self.quit
    }

    /// Reads the repository list again.
    #[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
    pub(crate) fn load_repositories(&mut self, cx: &mut Context<Self>) {
        let stamp = self.navigation.load_repositories();
        self.list_read = Some(self.services.read(
            ReadKind::Repositories,
            stamp,
            |service| service.list_repositories(),
            cx,
            |this: &mut Self, stamp, result, cx| {
                if this.navigation.apply_repositories(stamp, result) {
                    cx.notify();
                }
            },
        ));
        cx.notify();
    }

    /// Selects the repository `key` and resolves it. The list is read
    /// again too, so the configuration and index shown are the current
    /// ones (track rule 4); the new list keeps the selection.
    pub(crate) fn select_repository(&mut self, key: String, cx: &mut Context<Self>) {
        let stamp = self.navigation.select(key.clone());
        self.resolve(key, stamp, cx);
        self.load_repositories(cx);
    }

    /// Resolves `key` again: Retry on an unavailable or failed entry. As
    /// on selection, the list is read again.
    pub(crate) fn retry_resolution(&mut self, key: String, cx: &mut Context<Self>) {
        let stamp = self.navigation.retry_resolution(key.clone());
        self.resolve(key, stamp, cx);
        self.load_repositories(cx);
    }

    #[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
    fn resolve(&mut self, key: String, stamp: Stamp, cx: &mut Context<Self>) {
        let root = PathBuf::from(key);
        self.resolve_read = Some(self.services.read(
            ReadKind::Resolve,
            stamp,
            move |service| service.resolve_repository(&root),
            cx,
            |this: &mut Self, stamp, result, cx| {
                if this.navigation.apply_resolution(stamp, result) {
                    cx.notify();
                }
            },
        ));
        cx.notify();
    }

    fn focused_region(&self, window: &Window, cx: &App) -> Option<Region> {
        REGIONS
            .into_iter()
            .find(|region| self.regions[region.index()].contains_focused(window, cx))
    }

    fn focus_region(&mut self, region: Region, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.regions[region.index()], cx);
        cx.notify();
    }

    /// Focuses the region `step` places after the focused one, wrapping;
    /// the first region when none holds focus.
    fn cycle_region(&mut self, step: isize, window: &mut Window, cx: &mut Context<Self>) {
        let next = match self.focused_region(window, cx) {
            Some(region) => {
                let count = REGIONS.len() as isize;
                REGIONS[(region.index() as isize + step).rem_euclid(count) as usize]
            }
            None => Region::Repositories,
        };
        self.focus_region(next, window, cx);
    }

    /// The one quit handler, for the action and the close request. True
    /// when quitting goes ahead now.
    fn request_quit(&mut self, cx: &mut Context<Self>) -> bool {
        match self.quit {
            QuitPhase::Quit => return true,
            QuitPhase::Finishing(_) => return false,
            QuitPhase::Running => {}
        }
        match self.services.worker().quit() {
            QuitState::Idle => {
                self.quit_now(cx);
                true
            }
            QuitState::Finishing(id) => {
                self.quit = QuitPhase::Finishing(id);
                cx.notify();
                false
            }
        }
    }

    fn quit_now(&mut self, cx: &mut Context<Self>) {
        self.quit = QuitPhase::Quit;
        cx.notify();
        cx.quit();
    }

    /// A worker event arrived: if quitting waits for a job that has now
    /// returned, quit.
    fn operations_changed(&mut self, cx: &mut Context<Self>) {
        if matches!(self.quit, QuitPhase::Finishing(_))
            && self.services.worker().quit() == QuitState::Idle
        {
            self.quit_now(cx);
        }
    }

    /// "Finishing" and the command quitting waits for. Fixed text only.
    pub(crate) fn finishing_text(&self, cx: &App) -> Option<String> {
        let QuitPhase::Finishing(id) = self.quit else {
            return None;
        };
        let operations = self.services.operations().read(cx);
        Some(match operations.request(id) {
            Some(request) => format!("Finishing {}", request.intent.label()),
            None => "Finishing".to_owned(),
        })
    }

    /// A region: its focus handle, key context and, while it holds focus,
    /// its ring, drawn behind its content.
    fn region<C: IntoElement>(
        &self,
        region: Region,
        window: &Window,
        cx: &App,
        content: C,
    ) -> impl IntoElement + Styled + use<C> {
        let focused = self.focused_region(window, cx) == Some(region);
        let theme = cx.theme();
        let ring = focused.then(|| {
            div()
                .id(region.ring_id())
                .test_support()
                .absolute()
                .inset_0()
                .border_2()
                .border_color(theme.ring)
        });
        div()
            .id(region.id())
            .test_support()
            .track_focus(&self.regions[region.index()])
            .key_context(region.key_context())
            .relative()
            .p_2()
            .border_1()
            .border_color(theme.border)
            .children(ring)
            .child(content)
    }
}

/// The navigation pane's name, until its views arrive with Task 5.
fn pane_label(pane: Pane) -> &'static str {
    match pane {
        Pane::Documents => "Documents",
        Pane::Tickets => "Tickets",
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let finishing = self.finishing_text(cx).map(|text| {
            h_flex()
                .gap_2()
                .child(div().id("quit-finishing").test_support().child(text))
                .child(
                    div().id("quit-now").test_support().child(
                        Button::new("quit-now-button")
                            .danger()
                            .label("Quit now")
                            .on_click(|_, window, cx| {
                                window.dispatch_action(Box::new(QuitNow), cx)
                            }),
                    ),
                )
        });
        let repositories = sidebar::repositories(&self.navigation, cx);
        let selection = sidebar::selection(&self.navigation, cx);
        let pane = pane_label(self.navigation.pane());
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
                self.region(
                    Region::Navigation,
                    window,
                    cx,
                    div().id("navigation-pane").test_support().child(pane),
                )
                .w(px(320.))
                .h_full(),
            )
            .child(
                self.region(Region::Content, window, cx, selection)
                    .flex_1()
                    .h_full(),
            );
        let operations = self
            .region(
                Region::Operations,
                window,
                cx,
                v_flex()
                    .gap_1()
                    .child(div().text_sm().child("Operations"))
                    .children(finishing),
            )
            .w_full()
            .h(px(120.));
        // Root does not draw these layers itself: the window's first view
        // below it does.
        v_flex()
            .id("workspace")
            .key_context("Workspace")
            .on_action(cx.listener(|this, _: &Quit, _, cx| {
                this.request_quit(cx);
            }))
            .on_action(cx.listener(|this, _: &QuitNow, _, cx| this.quit_now(cx)))
            .on_action(
                cx.listener(|this, _: &NextRegion, window, cx| this.cycle_region(1, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &PreviousRegion, window, cx| {
                    this.cycle_region(-1, window, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &FocusRepositories, window, cx| {
                this.focus_region(Region::Repositories, window, cx)
            }))
            .on_action(cx.listener(|this, _: &FocusNavigation, window, cx| {
                this.focus_region(Region::Navigation, window, cx)
            }))
            .on_action(cx.listener(|this, _: &FocusContent, window, cx| {
                this.focus_region(Region::Content, window, cx)
            }))
            .on_action(cx.listener(|this, _: &FocusOperations, window, cx| {
                this.focus_region(Region::Operations, window, cx)
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

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "workspace_tests.rs"]
mod tests;
