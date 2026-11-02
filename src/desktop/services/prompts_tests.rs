//! The provider bridge from a plain thread: each answer reaches the
//! library's call as designed, the two that have nobody to answer return
//! at once, and the session's cache keeps a second unlock of the same key
//! from asking again (no network: the session is driven directly).

use std::{
    fs,
    path::Path,
    sync::mpsc as std_mpsc,
    thread,
    time::{Duration, Instant},
};

use futures::{StreamExt, channel::mpsc, executor::block_on};
use manyhands::repository::{
    SharedKeyId,
    keys::{
        KeySourceToken, PassphraseResponse, SecretPassphrase, SessionCredentialProvider,
        SessionCredentials, UnlockReason, UnlockRequest,
    },
};

use super::{DesktopProvider, PromptReply};
use crate::desktop::services::worker::WorkerEvent;

/// Longer than any answer here should take on a loaded machine.
const WAIT: Duration = Duration::from_secs(10);

/// An unlock request for a key whose private file is `path`.
fn unlock(path: &Path, key_id: SharedKeyId, label: &str) -> UnlockRequest {
    fs::write(path, b"opaque fixture bytes").unwrap();
    UnlockRequest {
        key_id,
        label: label.to_owned(),
        source: KeySourceToken::observe(path).unwrap(),
        reason: UnlockReason::ProtectedKey,
    }
}

/// What a provider call made on its own thread answered.
fn answer_of(response: PassphraseResponse) -> &'static str {
    match response {
        PassphraseResponse::Supplied(_) => "supplied",
        PassphraseResponse::Cancelled => "cancelled",
        PassphraseResponse::Unavailable => "unavailable",
    }
}

/// Calls the provider on a plain thread, answers its prompt with
/// `reply` (dropping the prompt when `reply` is `None`), and gives what
/// the call returned.
fn ask(reply: Option<PromptReply>) -> &'static str {
    let directory = tempfile::tempdir().unwrap();
    let request = unlock(&directory.path().join("key"), SharedKeyId::new(), "primary");
    let (events, mut receiver) = mpsc::unbounded();
    let (done, returned) = std_mpsc::channel();
    let caller = thread::spawn(move || {
        let mut provider = DesktopProvider::new(events);
        let _ = done.send(answer_of(provider.request_passphrase(&request)));
    });
    let Some(WorkerEvent::Prompt(prompt)) = block_on(receiver.next()) else {
        panic!("the provider sent no prompt");
    };
    assert_eq!(prompt.label, "primary");
    assert_eq!(prompt.reason, UnlockReason::ProtectedKey);
    match reply {
        Some(reply) => prompt.answer(reply),
        None => drop(prompt),
    }
    let answer = returned.recv_timeout(WAIT).expect("the provider returned");
    caller.join().unwrap();
    answer
}

#[test]
fn a_supplied_passphrase_reaches_the_call() {
    let secret = SecretPassphrase::new("correct horse".to_owned()).unwrap();
    assert_eq!(ask(Some(PromptReply::Supplied(secret))), "supplied");
}

#[test]
fn a_cancelled_prompt_answers_cancelled() {
    assert_eq!(ask(Some(PromptReply::Cancelled)), "cancelled");
}

#[test]
fn an_unavailable_reply_answers_unavailable() {
    assert_eq!(ask(Some(PromptReply::Unavailable)), "unavailable");
}

/// A prompt dropped unanswered (its dialog gone with its window) wakes
/// the call at once with `Unavailable`.
#[test]
fn a_dropped_reply_sender_answers_unavailable_at_once() {
    let started = Instant::now();
    assert_eq!(ask(None), "unavailable");
    assert!(started.elapsed() < WAIT);
}

/// With the UI gone the send fails, and the call returns without waiting
/// for anybody: on this very thread, with nothing to answer it.
#[test]
fn a_failed_send_answers_unavailable_without_blocking() {
    let directory = tempfile::tempdir().unwrap();
    let request = unlock(&directory.path().join("key"), SharedKeyId::new(), "primary");
    let (events, receiver) = mpsc::unbounded();
    drop(receiver);
    let mut provider = DesktopProvider::new(events);
    assert_eq!(
        answer_of(provider.request_passphrase(&request)),
        "unavailable"
    );
}

/// The worker's session over the desktop provider, driven from a plain
/// thread as the worker drives it: after one supplied unlock, a second use
/// of the same key does not prompt; another key does. Answers are never
/// shared: each prompt gets its own.
#[test]
fn a_supplied_unlock_is_cached_for_the_key_and_another_key_prompts() {
    let directory = tempfile::tempdir().unwrap();
    let first = unlock(&directory.path().join("first"), SharedKeyId::new(), "first");
    let second = unlock(
        &directory.path().join("second"),
        SharedKeyId::new(),
        "second",
    );
    let (first_id, second_id) = (first.key_id, second.key_id);
    let (events, mut receiver) = mpsc::unbounded();
    let caller = thread::spawn(move || {
        let mut session = SessionCredentials::new(DesktopProvider::new(events));
        let mut uses = Vec::new();
        for request in [first.clone(), first, second] {
            uses.push(session.with_passphrase(request, |_| Ok(())).is_ok());
        }
        uses
    });
    // Answer every prompt until the session, and its provider, are gone.
    let mut prompted = Vec::new();
    while let Some(event) = block_on(receiver.next()) {
        let WorkerEvent::Prompt(prompt) = event else {
            panic!("the provider sends only prompts");
        };
        prompted.push(prompt.key_id);
        let secret = SecretPassphrase::new("correct horse".to_owned()).unwrap();
        prompt.answer(PromptReply::Supplied(secret));
    }
    assert_eq!(caller.join().unwrap(), [true, true, true]);
    assert_eq!(prompted, [first_id, second_id]);
}
