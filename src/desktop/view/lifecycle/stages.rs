//! The lifecycle dialog's stages and its header (Cycle 05b, A1, A2).
//!
//! The dialog's phases group into six stages, which set its header, its
//! footer's dismiss button and hint, and whether its backdrop hides it. A
//! submitted run is read through `LifecycleDialog::shown`, as its body is,
//! so the two cannot disagree (R-1):
//!
//! | Stage | Phases |
//! | --- | --- |
//! | Ask | every phase before a run is sent; a sent run shown as a message only, or a resume answered with no lifecycle data (K-10) |
//! | Running | a sent run with no answer yet |
//! | Waiting | an answer whose `Ending` is `StillRunning` |
//! | Done, Failed, Stopped | an answer whose `Ending` is that |
//!
//! The header holds the stage's tile and title, and the item's subtitle.
//! The footer's dismiss button (A7) is Cancel while asking to confirm,
//! Close for the other questions and while waiting, Not now after a failure
//! or a stop, and Done, last, once done; a running dialog has none, and
//! hints that Escape hides it, as a click on its backdrop does (A5, R-5).

use gpui_kit::{App, SharedString, assets::IconName};
use manyhands::repository::ItemDtoKind;

use super::{
    CANCEL, CLOSE, CLOSED, CLOSING, CONFIRM_HINT, COULDNT_CLOSE, COULDNT_MERGE, DONE, HIDE_HINT,
    LifecycleDialog, MERGING, NOT_NOW, Phase, STILL_RUNNING, approve_title, close_title,
    outcome::Ending, running::Shown,
};
use crate::desktop::{
    model::activity::lifecycle_lines::{ENDING_STOPPED, merged},
    view::frame::{Header, Tone},
};

/// Where the dialog stands (design A1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Stage {
    Ask,
    Running,
    Waiting,
    Done,
    Failed,
    Stopped,
}

impl LifecycleDialog {
    /// The stage the dialog is at.
    pub(super) fn stage(&self, cx: &App) -> Stage {
        let Phase::Submitted(id) = self.phase else {
            return Stage::Ask;
        };
        match self.shown(id, cx) {
            Shown::Message(_) | Shown::Resumed(..) => Stage::Ask,
            Shown::Running => Stage::Running,
            Shown::Ended(envelope) => match Ending::of(envelope) {
                Ending::Done => Stage::Done,
                Ending::Failed => Stage::Failed,
                Ending::Stopped => Stage::Stopped,
                Ending::StillRunning => Stage::Waiting,
            },
        }
    }

    /// The header for the Kit's title slot: the stage's tile and title, and
    /// the item the dialog speaks of.
    pub(super) fn header(&self, cx: &App) -> Header {
        let stage = self.stage(cx);
        let ticket = self.heading.kind == ItemDtoKind::Ticket;
        let short = &self.heading.short;
        let icon = match (stage, ticket) {
            (Stage::Done, _) => (IconName::CircleCheck, Tone::Success),
            (Stage::Failed, _) => (IconName::CircleX, Tone::Danger),
            (_, true) => (IconName::Archive, Tone::Accent),
            (_, false) => (IconName::GitMerge, Tone::Accent),
        };
        let title = match (stage, ticket) {
            (Stage::Ask, true) => close_title(short),
            (Stage::Ask, false) => approve_title(short),
            (Stage::Running, true) => CLOSING.to_owned(),
            (Stage::Running, false) => MERGING.to_owned(),
            (Stage::Waiting, _) => STILL_RUNNING.to_owned(),
            (Stage::Done, true) => CLOSED.to_owned(),
            (Stage::Done, false) => merged(&self.primary()),
            (Stage::Failed, true) => COULDNT_CLOSE.to_owned(),
            (Stage::Failed, false) => COULDNT_MERGE.to_owned(),
            (Stage::Stopped, _) => ENDING_STOPPED.to_owned(),
        };
        Header {
            icon: Some(icon),
            title: SharedString::from(title),
            description: Some(self.heading.subtitle().into()),
            ids: ("lifecycle-title", Some("lifecycle-subtitle")),
        }
    }

    /// Whether a click on the backdrop hides the dialog, as Escape does:
    /// only while it runs (A5, R-5).
    pub(super) fn hides_on_backdrop(&self, cx: &App) -> bool {
        self.stage(cx) == Stage::Running
    }

    /// The footer's dismiss button's label at `stage`; `None` while
    /// running, where Stop takes its place (A7).
    pub(super) fn dismiss_label(&self, stage: Stage) -> Option<&'static str> {
        match (stage, &self.phase) {
            (Stage::Running, _) => None,
            (Stage::Done, _) => Some(DONE),
            (Stage::Failed | Stage::Stopped, _) => Some(NOT_NOW),
            (
                Stage::Ask,
                Phase::Saving | Phase::Preparing | Phase::Confirm { .. } | Phase::Stale { .. },
            ) => Some(CANCEL),
            (Stage::Ask | Stage::Waiting, _) => Some(CLOSE),
        }
    }

    /// The footer's hint at `stage` (A4, A5).
    pub(super) fn hint(&self, stage: Stage) -> Option<&'static str> {
        match (stage, &self.phase) {
            (Stage::Running, _) => Some(HIDE_HINT),
            (_, Phase::Confirm { .. }) => Some(CONFIRM_HINT),
            _ => None,
        }
    }
}
