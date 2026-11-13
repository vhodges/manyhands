//! Key commands and attempt-scoped secret handling.

use std::path::PathBuf;

use gpui_kit::{Context, Window, component::WindowExt};
use manyhands::{
    repository::{KeyOwnership, SharedKeyId, keys::SecretPassphrase},
    results::{Outcome, RecoveryActionKind, ResultCode},
};

use super::{
    CONFIRMATION_FIELD, IMPORT_LABEL_FIELD, KeysPage, LABEL_FIELD, MISMATCH, NEED_LABEL,
    NEED_PASSPHRASE, NEED_PRIVATE, PASSPHRASE_FIELD, PRIVATE_FIELD, PUBLIC_FIELD, REFUSED,
    Retrying,
};
use crate::desktop::{
    model::read_state::Read,
    services::requests::{DesktopRequestId, Intent, Lane, NotSent, RequestState},
    view::confirm::confirm,
};

fn key_command(intent: &Intent) -> bool {
    matches!(
        intent,
        Intent::KeyGenerate { .. }
            | Intent::KeyImport { .. }
            | Intent::KeySelect { .. }
            | Intent::KeyClear
            | Intent::KeyRemove { .. }
            | Intent::KeyDelete { .. }
    )
}

fn protected_generation(intent: &Intent) -> Option<&str> {
    match intent {
        Intent::KeyGenerate {
            label,
            protected: true,
        } => Some(label),
        _ => None,
    }
}

impl KeysPage {
    /// Every submit, refused or sent, empties both masked inputs.
    pub(crate) fn submit_generate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            return;
        }
        let label = match &self.retrying {
            Some(retrying) => retrying.label.clone(),
            None => self.text(LABEL_FIELD, cx),
        };
        let secret = self.take_secret(window, cx);
        let why = match (&self.retrying, &secret) {
            (_, Err(why)) => Some(*why),
            (Some(_), Ok(None)) => Some(NEED_PASSPHRASE),
            (None, _) if label.is_empty() => Some(NEED_LABEL),
            _ => None,
        };
        if let Some(why) = why {
            self.generate_message = Some(why);
            cx.notify();
            return;
        }
        let Ok(secret) = secret else { return };
        self.generate_message = None;
        let worker = self.services.worker();
        self.submitted = Some(match self.retrying.take() {
            Some(retrying) => {
                worker.retry(retrying.id, secret);
                retrying.id
            }
            None => {
                self.clear(&[LABEL_FIELD], window, cx);
                let intent = Intent::KeyGenerate {
                    label,
                    protected: secret.is_some(),
                };
                debug_assert!(!intent.confirmed());
                worker.submit(intent, Lane::User, None, secret)
            }
        });
        cx.notify();
    }

    /// Compare in place, then move text into the single zeroizing holder.
    /// No typed passphrase survives a submit in either input or its undo history.
    fn take_secret(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Option<SecretPassphrase>, &'static str> {
        let (passphrase, confirmation) = (
            self.inputs[PASSPHRASE_FIELD].read(cx).text(),
            self.inputs[CONFIRMATION_FIELD].read(cx).text(),
        );
        let answer = if passphrase != confirmation {
            Err(MISMATCH)
        } else if passphrase.len() == 0 {
            Ok(None)
        } else {
            SecretPassphrase::new(passphrase.to_string())
                .map(Some)
                .map_err(|_| REFUSED)
        };
        self.clear(&[PASSPHRASE_FIELD, CONFIRMATION_FIELD], window, cx);
        answer
    }

    pub(crate) fn ask_again(
        &mut self,
        id: DesktopRequestId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) {
            return;
        }
        let label = {
            let operations = self.services.operations().read(cx);
            let Some(label) = operations
                .request(id)
                .and_then(|request| protected_generation(&request.intent))
            else {
                return;
            };
            label.to_owned()
        };
        self.retrying = Some(Retrying { id, label });
        self.submitted = Some(id);
        self.generate_message = None;
        self.clear(&[PASSPHRASE_FIELD, CONFIRMATION_FIELD], window, cx);
        self.inputs[PASSPHRASE_FIELD].update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    pub(super) fn cancel_retry(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            return;
        }
        self.retrying = None;
        self.generate_message = None;
        self.clear(&[PASSPHRASE_FIELD, CONFIRMATION_FIELD], window, cx);
        cx.notify();
    }

    pub(crate) fn submit_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            return;
        }
        let label = self.text(IMPORT_LABEL_FIELD, cx);
        let private = self.text(PRIVATE_FIELD, cx);
        let public = self.text(PUBLIC_FIELD, cx);
        let why = if label.is_empty() {
            Some(NEED_LABEL)
        } else if private.is_empty() {
            Some(NEED_PRIVATE)
        } else {
            None
        };
        if let Some(why) = why {
            self.import_message = Some(why);
            cx.notify();
            return;
        }
        self.import_message = None;
        self.clear(
            &[IMPORT_LABEL_FIELD, PRIVATE_FIELD, PUBLIC_FIELD],
            window,
            cx,
        );
        let intent = Intent::KeyImport {
            label,
            private_key_path: PathBuf::from(private),
            public_key_path: (!public.is_empty()).then(|| PathBuf::from(public)),
        };
        self.submit(intent, cx);
    }

    fn submit(&mut self, intent: Intent, cx: &mut Context<Self>) {
        debug_assert!(!intent.confirmed());
        self.submitted = Some(
            self.services
                .worker()
                .submit(intent, Lane::User, None, None),
        );
        cx.notify();
    }

    pub(crate) fn submit_select(
        &mut self,
        id: SharedKeyId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) || self.listed(id).is_none() {
            return;
        }
        self.submit(Intent::KeySelect { id }, cx);
    }

    pub(crate) fn submit_clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) || !matches!(self.keys, Read::Ready(_)) {
            return;
        }
        self.submit(Intent::KeyClear, cx);
    }

    pub(crate) fn submit_unregister(
        &mut self,
        id: SharedKeyId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) || self.listed(id).is_none() {
            return;
        }
        self.submit(Intent::KeyRemove { id }, cx);
    }

    /// Confirm generated-key deletion, keeping its existing focus fallback.
    pub(crate) fn submit_delete(
        &mut self,
        id: SharedKeyId,
        ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let generated = self
            .listed(id)
            .is_some_and(|key| key.ownership == KeyOwnership::Generated);
        if window.has_active_dialog(cx) || !generated {
            return;
        }
        let intent = Intent::KeyDelete { id };
        debug_assert!(intent.confirmed());
        let origin = match window.focused(cx) {
            Some(focused) => focused,
            None => self.focus(format!("key-delete-{ix}"), cx),
        };
        confirm(intent, origin, &self.services, window, cx);
    }

    pub(super) fn operations_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut reload = false;
        let mut ask = None;
        let operations = self.services.operations().clone();
        // Newest first: ask for the first generation waiting for a secret.
        for request in operations.read(cx).requests() {
            if !key_command(&request.intent) {
                continue;
            }
            match &request.state {
                RequestState::Finished(..) => {
                    reload |= self.seen.insert((request.id, request.attempt));
                }
                RequestState::NotSent(NotSent::SecretRequired) => {
                    let Some(label) = protected_generation(&request.intent) else {
                        continue;
                    };
                    if self.seen.insert((request.id, request.attempt)) && ask.is_none() {
                        ask = Some(Retrying {
                            id: request.id,
                            label: label.to_owned(),
                        });
                    }
                }
                _ => {}
            }
        }
        if let Some(ask) = ask {
            // R-T15b: a new key's typed secret never becomes an old request's.
            self.clear(&[PASSPHRASE_FIELD, CONFIRMATION_FIELD], window, cx);
            self.submitted = Some(ask.id);
            self.generate_message = None;
            self.retrying = Some(ask);
        }
        if reload {
            self.load(cx);
        }
        cx.notify();
    }

    pub(super) fn retry_offered(&self, cx: &Context<Self>) -> bool {
        let Some(id) = self.submitted else {
            return false;
        };
        if self.retrying.is_some() {
            return false;
        }
        let operations = self.services.operations().read(cx);
        let Some(request) = operations.request(id) else {
            return false;
        };
        if protected_generation(&request.intent).is_none() {
            return false;
        }
        match &request.state {
            RequestState::Finished(outcome, code, _, recovery, _) => {
                !matches!(outcome, Outcome::Success | Outcome::Noop)
                    && (*code == ResultCode::Busy
                        || recovery
                            .iter()
                            .any(|action| action.action == RecoveryActionKind::RequestRetry))
            }
            RequestState::NotSent(NotSent::SecretRequired) => true,
            _ => false,
        }
    }
}
