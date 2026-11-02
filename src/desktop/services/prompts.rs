//! The credential provider of the worker's session, and the prompt it
//! sends the UI.
//!
//! The library asks for a passphrase on the worker thread, inside a job.
//! `DesktopProvider` sends the UI a `PromptRequest` over the worker's
//! event channel and blocks that thread, never an executor's, until the
//! reply comes. A send that fails, because the UI is gone, answers
//! `Unavailable` at once; so does a reply sender dropped unanswered, as
//! when the window that held the prompt closes.
//!
//! One worker runs one job at a time and calls the provider synchronously,
//! so at most one prompt exists at a time; each prompt has its own reply,
//! and no answer is shared or copied between requests. After a supplied
//! unlock the session caches the passphrase for that key, so a later job
//! for the same key does not ask again.
//!
//! Not now (`Cancelled`) marks the key declined for the session (R-13):
//! a later request for it answers `Cancelled` at once, with no prompt,
//! until Unlock… clears the mark through `WorkerHandle::clear_declined`.
//! The set is shared by the worker's provider and its handle. Each key the
//! provider answers `Cancelled`, declined now or before, is also recorded
//! against the request whose attempt asked for it, so Unlock… on that
//! request's outcome clears its keys (`WorkerHandle::clear_declined_for`)
//! without the outcome naming one (R-3, R-13, R-17).

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, PoisonError},
};

use futures::{
    channel::{mpsc::UnboundedSender, oneshot},
    executor::block_on,
};
use manyhands::repository::{
    SharedKeyId,
    keys::{
        PassphraseResponse, SecretPassphrase, SessionCredentialProvider, UnlockReason,
        UnlockRequest,
    },
};

use super::{requests::DesktopRequestId, worker::WorkerEvent};

/// A passphrase the library asks for while a job runs. The worker waits
/// for `reply`; a dropped sender reads as `Unavailable`.
///
/// No `Debug`: the reply carries a passphrase.
pub(crate) struct PromptRequest {
    pub key_id: SharedKeyId,
    pub label: String,
    pub reason: UnlockReason,
    pub reply: oneshot::Sender<PromptReply>,
}

impl PromptRequest {
    /// Answers the prompt. A worker that no longer waits wants nothing.
    pub(crate) fn answer(self, reply: PromptReply) {
        let _ = self.reply.send(reply);
    }
}

/// The answer to a prompt. No `Debug`: it may carry the passphrase.
pub(crate) enum PromptReply {
    Supplied(SecretPassphrase),
    Cancelled,
    Unavailable,
}

/// The keys declined with Not now this session (R-13), shared by the
/// worker's provider and its handle, and which request's attempt each
/// was answered `Cancelled` for.
#[derive(Clone, Default)]
pub(crate) struct DeclinedKeys(Arc<Mutex<Declined>>);

#[derive(Default)]
struct Declined {
    keys: HashSet<SharedKeyId>,
    /// The request whose attempt the worker is running, if any.
    attempt: Option<DesktopRequestId>,
    /// The keys each request's last attempt was answered `Cancelled` for.
    by_request: HashMap<DesktopRequestId, HashSet<SharedKeyId>>,
}

impl DeclinedKeys {
    /// The set. A panic elsewhere while it was held leaves sets of IDs,
    /// whole, so a poisoned lock is taken.
    fn lock(&self) -> std::sync::MutexGuard<'_, Declined> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Answers `Cancelled` for `key_id` when it is declined, recording it
    /// against the running attempt's request.
    fn declined(&self, key_id: SharedKeyId) -> bool {
        let mut declined = self.lock();
        if !declined.keys.contains(&key_id) {
            return false;
        }
        declined.record(key_id);
        true
    }

    /// Not now: `key_id` is declined, and recorded against the running
    /// attempt's request.
    fn decline(&self, key_id: SharedKeyId) {
        let mut declined = self.lock();
        declined.keys.insert(key_id);
        declined.record(key_id);
    }

    /// The worker begins an attempt of `id`, or, with `None`, runs no
    /// request's attempt: what a later answer is recorded against. An
    /// attempt begun forgets what the request's last one recorded.
    pub(crate) fn attempting(&self, id: Option<DesktopRequestId>) {
        let mut declined = self.lock();
        declined.attempt = id;
        if let Some(id) = id {
            declined.by_request.remove(&id);
        }
    }

    /// Clears `key_id`'s mark; true when it was declined.
    pub(crate) fn clear(&self, key_id: SharedKeyId) -> bool {
        self.lock().keys.remove(&key_id)
    }

    /// Clears the marks of the keys the request `id`'s last attempt was
    /// answered `Cancelled` for, so its next attempt prompts for them
    /// again; true when any was still declined.
    pub(crate) fn clear_for(&self, id: DesktopRequestId) -> bool {
        let mut declined = self.lock();
        let keys = declined.by_request.remove(&id).unwrap_or_default();
        // Every key is cleared, not only the first one found declined.
        let mut cleared = false;
        for key_id in &keys {
            cleared |= declined.keys.remove(key_id);
        }
        cleared
    }
}

impl Declined {
    fn record(&mut self, key_id: SharedKeyId) {
        if let Some(id) = self.attempt {
            self.by_request.entry(id).or_default().insert(key_id);
        }
    }
}

/// The provider of the worker's `SessionCredentials`: it asks the UI,
/// except for a key declined this session.
pub(crate) struct DesktopProvider {
    tx: UnboundedSender<WorkerEvent>,
    declined: DeclinedKeys,
}

impl DesktopProvider {
    /// A provider with a declined set of its own.
    #[cfg(test)]
    pub(crate) fn new(tx: UnboundedSender<WorkerEvent>) -> Self {
        Self::with_declined(tx, DeclinedKeys::default())
    }

    /// A provider that shares `declined`.
    pub(crate) fn with_declined(tx: UnboundedSender<WorkerEvent>, declined: DeclinedKeys) -> Self {
        Self { tx, declined }
    }
}

impl SessionCredentialProvider for DesktopProvider {
    /// Runs on the worker thread, which no executor uses: blocking it on
    /// the reply stalls nothing else.
    fn request_passphrase(&mut self, request: &UnlockRequest) -> PassphraseResponse {
        // Declined with Not now: not asked again until Unlock… (R-13).
        if self.declined.declined(request.key_id) {
            return PassphraseResponse::Cancelled;
        }
        let (reply, answer) = oneshot::channel();
        let prompt = PromptRequest {
            key_id: request.key_id,
            label: request.label.clone(),
            reason: request.reason,
            reply,
        };
        if self.tx.unbounded_send(WorkerEvent::Prompt(prompt)).is_err() {
            return PassphraseResponse::Unavailable;
        }
        match block_on(answer) {
            Ok(PromptReply::Supplied(passphrase)) => PassphraseResponse::Supplied(passphrase),
            Ok(PromptReply::Cancelled) => {
                self.declined.decline(request.key_id);
                PassphraseResponse::Cancelled
            }
            // A reply sender dropped unanswered reads as `Unavailable`.
            Ok(PromptReply::Unavailable) | Err(oneshot::Canceled) => {
                PassphraseResponse::Unavailable
            }
        }
    }
}

#[cfg(test)]
#[path = "prompts_tests.rs"]
mod tests;
