//! Quitting the main window.
//!
//! The Quit action and the window's close request go through one
//! handler. With no job running it quits. With one running it refuses,
//! stops the worker admitting jobs, shows "Finishing" and Quit now, and
//! quits when the job returns. Quit now quits at once: the worker thread
//! is not waited for, and the library's journals recover what it was
//! doing. Either way a prompt the running job waits on is answered
//! `Unavailable` first, so the job can return.
//!
//! When quitting starts to wait, every open dialog closes and Quit now
//! takes focus (design, Quitting: "the focused Quit now button"), so
//! nothing stands in front of it; while it waits, no dialog opens.

use gpui_kit::{
    App, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    TestSupportExt, Window,
    component::{
        ActiveTheme,
        button::{Button, ButtonVariants},
        h_flex,
    },
    div,
};

use super::workspace::Workspace;
use crate::desktop::{keys::QuitNow, services::requests::DesktopRequestId};

pub(crate) const QUIT_NOW: &str = "Quit now";

/// Where quitting stands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum QuitPhase {
    Running,
    /// Asked to quit; waiting for this job to return.
    Finishing(DesktopRequestId),
    /// Quitting.
    Quit,
}

impl Workspace {
    /// "Finishing" and the command quitting waits for. Fixed text only.
    pub(crate) fn finishing_text(&self, cx: &App) -> Option<String> {
        let QuitPhase::Finishing(id) = self.quit_phase() else {
            return None;
        };
        let operations = self.services().operations().read(cx);
        Some(match operations.request(id) {
            Some(request) => format!("Finishing {}", request.intent.label()),
            None => "Finishing".to_owned(),
        })
    }

    /// "Finishing" with Quit now, while quitting waits for a job.
    /// Quit now is one control, a tab stop whose wrapper holds the focus
    /// and is pressed by Enter, Space or a click; the Kit button inside
    /// only draws it.
    pub(super) fn finishing(&self, window: &Window, cx: &App) -> Option<impl IntoElement + use<>> {
        let ring = cx.theme().ring;
        let focused = self.quit_focus().is_focused(window);
        self.finishing_text(cx).map(|text| {
            h_flex()
                .gap_2()
                .child(div().id("quit-finishing").test_support().child(text))
                .child(
                    div()
                        .id("quit-now")
                        .test_support()
                        .aria_label(QUIT_NOW)
                        .track_focus(self.quit_focus())
                        .relative()
                        .rounded_sm()
                        .on_click(|_, window, cx| window.dispatch_action(Box::new(QuitNow), cx))
                        .child(
                            Button::new("quit-now-button")
                                .danger()
                                .label(QUIT_NOW)
                                .tab_stop(false),
                        )
                        .children(focused.then(|| {
                            div()
                                .id("quit-now-focus-ring")
                                .test_support()
                                .absolute()
                                .inset_0()
                                .rounded_sm()
                                .border_2()
                                .border_color(ring)
                        })),
                )
        })
    }
}
