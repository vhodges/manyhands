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

use super::worker::WorkerEvent;

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

/// The provider of the worker's `SessionCredentials`: it asks the UI.
pub(crate) struct DesktopProvider {
    tx: UnboundedSender<WorkerEvent>,
}

impl DesktopProvider {
    pub(crate) fn new(tx: UnboundedSender<WorkerEvent>) -> Self {
        Self { tx }
    }
}

impl SessionCredentialProvider for DesktopProvider {
    /// Runs on the worker thread, which no executor uses: blocking it on
    /// the reply stalls nothing else.
    fn request_passphrase(&mut self, request: &UnlockRequest) -> PassphraseResponse {
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
            Ok(PromptReply::Cancelled) => PassphraseResponse::Cancelled,
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
