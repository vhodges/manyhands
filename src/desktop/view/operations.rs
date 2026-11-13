//! The handlers behind Activity's rows (Wave 04 Cycle 03b, F4): each action
//! a row, its button or its menu offers runs one of them, through
//! `Workspace::choose_activity` (`view::activity::actions`). The drawing the
//! operation area used to do (the session and pending columns and their
//! entries) is gone with the swap; what stays is what a handler needs: the
//! lifecycle dialog's reopening, a share outcome's routes and offers
//! (`outcomes`), the conflict sheet's entries (`review`), a pending row's
//! Resume, lifecycle Resume and Abandon (`resume`), a hold's action
//! (`attention`), a retirement's question (`retire`), the republish flow
//! (`republish`), and the element ids and fixed words the tests still read.
//!
//! A request's state in fixed text is `state_label`, which Settings' key
//! page reads too. The words of each row are the model's
//! (`model::activity::copy`, B2 to B5).

use gpui_kit::{Context, FocusHandle, Window, component::WindowExt};

mod attention;
mod outcomes;
pub(crate) mod republish;
mod resume;
mod retire;
mod review;

pub(in crate::desktop::view) use retire::{RetireStep, Subject as RetireSubject};

pub(in crate::desktop::view) use outcomes::{already_finished, facts_words, shown_offers};
pub(crate) use outcomes::{share_result, share_words};

use super::{lifecycle, workspace::Workspace};
use crate::desktop::{
    model::operations::{RecoveryControl, stage_words},
    services::{
        KEY_STORE_UNAVAILABLE,
        requests::{DesktopRequestId, NotSent, RequestState},
    },
};

// The pure text helpers live in the model, where Activity's rows read them
// (R-41); each keeps its old path.
#[cfg(test)]
pub(crate) use crate::desktop::model::activity::copy::{CANCEL, OPEN_LIFECYCLE, STOP, STOPPING};
pub(crate) use crate::desktop::model::activity::copy::{
    CANCELLED, DISMISS, FINISHING, NOT_SENT, QUEUED, RUNNING, outcome_label, scope_name,
};

pub(crate) const DISMISSED: &str = "Dismissed";
#[cfg(test)]
pub(crate) const RESUME: &str = "Resume";
#[cfg(test)]
pub(crate) const ABANDON: &str = "Abandon";

/// A request's state in fixed text.
pub(crate) fn state_label(state: &RequestState) -> String {
    match state {
        RequestState::Queued => QUEUED.to_owned(),
        // A stage's words, accepted or not (F3); an accepted request with
        // no stage yet is finishing.
        RequestState::Running {
            stage: Some(stage), ..
        } => format!("{RUNNING}: {}", stage_words(*stage, None)),
        RequestState::Running { accepted: true, .. } => FINISHING.to_owned(),
        RequestState::Running { stage: None, .. } => RUNNING.to_owned(),
        RequestState::Finished(outcome, ..) => outcome_label(*outcome).to_owned(),
        RequestState::Removed => CANCELLED.to_owned(),
        RequestState::Dismissed => DISMISSED.to_owned(),
        RequestState::NotSent(NotSent::SecretRequired) => {
            format!("{NOT_SENT}: the passphrase is needed again")
        }
        RequestState::NotSent(NotSent::KeyStoreUnavailable) => {
            format!("{NOT_SENT}: {KEY_STORE_UNAVAILABLE}")
        }
    }
}

/// Whether the lifecycle dialog carries a recovery control as its own
/// button or guidance (Retry, Prepare again, Resume, Cancel, Inspect,
/// Check host): the entry of a close or merge leaves it to the dialog.
pub(in crate::desktop::view) fn dialog_owns(control: RecoveryControl) -> bool {
    matches!(
        control,
        RecoveryControl::Retry
            | RecoveryControl::Prepare
            | RecoveryControl::Cancel
            | RecoveryControl::Resume
            | RecoveryControl::Inspect
            | RecoveryControl::CheckHost
    )
}

impl Workspace {
    /// Open… on a lifecycle's entry: its dialog again, on that request
    /// (D5, A3). Focus goes back to `origin`, the control or the list that
    /// chose it, when it closes.
    pub(in crate::desktop::view) fn open_lifecycle(
        &mut self,
        id: DesktopRequestId,
        origin: FocusHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) {
            return;
        }
        let Some(request) = self.services().operations().read(cx).request(id).cloned() else {
            return;
        };
        // What navigation lists of its repository names its steps.
        let known = request
            .intent
            .root()
            .and_then(|root| root.to_str())
            .map(|root| self.known(root))
            .unwrap_or_default();
        let services = self.services().clone();
        lifecycle::reopen(&request, known, origin, &services, window, cx);
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "operations_tests.rs"]
mod tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "operations/check_tests.rs"]
mod check_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "operations/record_tests.rs"]
mod record_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "operations/attention_support.rs"]
mod attention_support;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "operations/attention_tests.rs"]
mod attention_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "operations/attempts_tests.rs"]
mod attempts_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "operations/retiring_tests.rs"]
mod retiring_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "operations/retire_tests.rs"]
mod retire_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "operations/retire_real_tests.rs"]
mod retire_real_tests;
