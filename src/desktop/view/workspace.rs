//! The main window's view. For now a placeholder that holds the services,
//! renders Root's layers and owns quitting; the shell grows here.
//!
//! The Quit action and the window's close request go through one
//! handler. With no job running it quits. With one running it refuses,
//! stops the worker admitting jobs, shows "Finishing" and Quit now, and
//! quits when the job returns. Quit now quits at once: the worker thread
//! is not waited for, and the library's journals recover what it was
//! doing.

use gpui_kit::{
    App, Context, FocusHandle, InteractiveElement, IntoElement, ParentElement, Render, Styled,
    Subscription, TestSupportExt, Window,
    component::{
        Root,
        button::{Button, ButtonVariants},
        h_flex,
    },
    div,
};

use crate::desktop::{
    keys::{Quit, QuitNow},
    services::{Services, requests::DesktopRequestId, worker::QuitState},
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

pub(crate) struct Workspace {
    services: Services,
    focus: FocusHandle,
    quit: QuitPhase,
    _operations: Subscription,
}

impl Workspace {
    pub(crate) fn new(services: Services, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let operations = cx.observe(services.operations(), |this, _, cx| {
            this.operations_changed(cx)
        });
        let workspace = cx.entity().downgrade();
        window.on_window_should_close(cx, move |_, cx| {
            workspace
                .update(cx, |workspace, cx| workspace.request_quit(cx))
                .unwrap_or(true)
        });
        Self {
            services,
            focus,
            quit: QuitPhase::Running,
            _operations: operations,
        }
    }

    #[cfg(test)]
    pub(crate) fn services(&self) -> &Services {
        &self.services
    }

    #[cfg(test)]
    pub(crate) fn quit_phase(&self) -> QuitPhase {
        self.quit
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
        // Root does not draw these layers itself: the window's first view
        // below it does.
        div()
            .id("workspace")
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &Quit, _, cx| {
                this.request_quit(cx);
            }))
            .on_action(cx.listener(|this, _: &QuitNow, _, cx| this.quit_now(cx)))
            .relative()
            .size_full()
            .children(finishing)
            .children(Root::render_sheet_layer(window, cx))
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "workspace_tests.rs"]
mod tests;
