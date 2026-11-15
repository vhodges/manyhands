//! Add repository and Create repository (Wave 04 Cycle 05a design B10):
//! each builds today's intent and opens the confirmation dialog, which
//! gives focus back to the submit that opened it when it closes (G13), also
//! while its add is pending and the button disabled (Ruling 18).
//!
//! Add goes ahead only on B10's table (`checks::add_enabled`). Pressed
//! before its folder's text was checked, it checks first, and goes on once
//! the check lands only if the landed check enables it; otherwise focus
//! goes to the row's action, else the folder field, and nothing is
//! confirmed (R-T12b; G22). With Git finding no identity and none typed, it
//! asks for one first, focus in the name.

use std::{path::PathBuf, rc::Rc};

use gpui_kit::{Context, Window};

use super::{
    AddEvent, AddRepositorySheet, Form,
    checks::{Inspection, allowed_branch},
};
use crate::desktop::{
    services::requests::Intent,
    view::confirm::{Followed, confirm_followed},
};

impl AddRepositorySheet {
    /// Submit `form`. Nothing while an add of this open is pending (R-27).
    pub(super) fn submit(&mut self, form: Form, window: &mut Window, cx: &mut Context<Self>) {
        if self.adding {
            return;
        }
        let intent = match form {
            Form::Add => self.add_intent(window, cx),
            Form::Create => self.create_intent(cx),
        };
        if let Some(intent) = intent {
            self.confirm(form, intent, window, cx);
        }
        cx.notify();
    }

    /// Add's intent, when the add goes ahead now (B10).
    fn add_intent(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<Intent> {
        if !self.add_enabled(cx) {
            return None;
        }
        let path = self.text(Form::Add.path(), cx);
        // Add confirms only a folder whose current text was checked: the
        // check says whether it can go ahead (ruling R-T12b).
        if !self.inspection.landed_for(&path) {
            self.submit_pending = true;
            if !matches!(&self.inspection, Inspection::Checking(checking) if *checking == path) {
                self.inspect(cx);
            }
            return None;
        }
        let Inspection::Checked { inspection, .. } = &self.inspection else {
            return None;
        };
        let primary_branch = allowed_branch(inspection)?.to_owned();
        let (name, email) = (
            self.text(Form::Add.name(), cx),
            self.text(Form::Add.email(), cx),
        );
        let asks_identity = self.inspection.identity_required(&path);
        if asks_identity && name.is_empty() && email.is_empty() {
            self.inputs[Form::Add.name()].update(cx, |input, cx| input.focus(window, cx));
            return None;
        }
        // A folder already set up holds its prefix, and refuses another
        // (R-31): none is drawn, so none is sent.
        let prefix = Some(self.text(Form::Add.prefix(), cx))
            .filter(|prefix| !prefix.is_empty() && !self.inspection.set_up());
        Some(Intent::RepoEnable {
            root: PathBuf::from(path),
            primary_branch,
            identity: asks_identity.then_some((name, email)),
            slug_prefix: prefix,
        })
    }

    /// `intent` in the confirmation dialog, followed for the request and
    /// attempt it queues (B10, R-28).
    fn confirm(&mut self, form: Form, intent: Intent, window: &mut Window, cx: &mut Context<Self>) {
        debug_assert!(intent.confirmed());
        let origin = self.focus[form.submit()].clone();
        let dialog = confirm_followed(
            intent,
            Rc::new(move |_| origin.clone()),
            &self.services,
            window,
            cx,
        );
        if let Some(dialog) = dialog {
            self._followed = Some(cx.subscribe(&dialog, |_, _, followed: &Followed, cx| {
                if let Followed::Confirmed {
                    request, attempt, ..
                } = followed
                {
                    cx.emit(AddEvent::Confirmed {
                        request: *request,
                        attempt: *attempt,
                    });
                }
            }));
        }
    }

    /// The check Add waited for landed (ruling R-T12b): the add goes on
    /// only if the landed check enables it; otherwise focus goes to the
    /// row's action, else the folder field (G22). A check whose text changed
    /// meanwhile was dropped, and the wait with it; one that lands after the
    /// sheet closed goes no further (G12).
    pub(super) fn resume_submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !std::mem::take(&mut self.submit_pending) || !self.open.is_open(window, cx) {
            return;
        }
        self.sync_branches(window, cx);
        let path = self.text(Form::Add.path(), cx);
        if !self.inspection.landed_for(&path) {
            return;
        }
        if !self.add_enabled(cx) {
            let target = self
                .action_focus()
                .unwrap_or_else(|| self.add_path_focus(cx));
            window.focus(&target, cx);
            cx.notify();
            return;
        }
        self.submit(Form::Add, window, cx);
    }
}
