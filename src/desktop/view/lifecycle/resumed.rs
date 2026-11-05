//! The lifecycle dialog on a resume (D5 decision 10, Part C; P5, P22):
//! Resume on a lifecycle row, the dialog's own Resume, and the resume an
//! applied lifecycle child's resolution sends (R-10).
//!
//! The dialog follows the resume as a run. When its answer names the
//! request that holds the lifecycle (`original_request_required`), the
//! drain sends that close or merge again (`Resends`), and the dialog
//! follows that run instead: Running, then how it ends. A resume answered
//! with lifecycle data ends as a run does. One answered `busy` reads "Still
//! running…", with no failure; any other answer with no lifecycle data shows
//! its message and code.

use gpui_kit::{
    AnyElement, App, Context, Entity, FocusHandle, IntoElement, ParentElement, Styled, Window,
    component::{ActiveTheme, v_flex},
};
use manyhands::results::ResultCode;

use super::{Heading, LifecycleDialog, Opening, Phase, STILL_RUNNING, open, text};
use crate::desktop::services::{
    Services,
    requests::{DesktopRequestId, Intent},
};

/// Opens the lifecycle dialog on the resume `resume` of the close or merge
/// `intent`, of the item `heading` names; focus goes back to `origin`.
/// `None` for an intent that is no lifecycle, or while quitting.
pub(crate) fn follow_resume(
    intent: Intent,
    heading: Heading,
    resume: DesktopRequestId,
    origin: FocusHandle,
    services: &Services,
    window: &mut Window,
    cx: &mut App,
) -> Option<Entity<LifecycleDialog>> {
    if !matches!(
        intent,
        Intent::TicketClose { .. } | Intent::DocumentPromote { .. }
    ) {
        return None;
    }
    let opening = Opening {
        intent,
        heading,
        origin,
        saving: None,
        following: Some(resume),
        repaired: false,
    };
    open(opening, services, window, cx)
}

impl LifecycleDialog {
    /// The shown resume was sent on as the request it named: the dialog
    /// follows that run.
    pub(super) fn follow_resend(&mut self, cx: &mut Context<Self>) {
        let Phase::Submitted(shown) = self.phase else {
            return;
        };
        let Some(sent) = self.services.resends().resent(shown) else {
            return;
        };
        if sent != shown {
            self.following_anew();
            self.set_phase(Phase::Submitted(sent), cx);
            cx.notify();
        }
    }

    /// A resume answered with no lifecycle data: `busy` is still running
    /// (P22); anything else shows its message and code.
    pub(super) fn resume_answered(
        &self,
        code: ResultCode,
        message: &str,
        cx: &Context<Self>,
    ) -> (AnyElement, Vec<AnyElement>) {
        if code == ResultCode::Busy {
            let body = text("lifecycle-still-running", STILL_RUNNING)
                .text_sm()
                .into_any_element();
            return (body, Vec::new());
        }
        let muted = cx.theme().muted_foreground;
        let body = v_flex()
            .gap_1()
            .child(text("lifecycle-message", message.to_owned()).text_sm())
            .child(
                text("lifecycle-code", code.as_str())
                    .text_xs()
                    .text_color(muted),
            )
            .into_any_element();
        (body, Vec::new())
    }
}
