//! The provider bridge from a plain thread: each answer reaches the
//! library's call as designed, the two that have nobody to answer return
//! at once, and the session's cache keeps a second unlock of the same key
//! from asking again (no network: the session is driven directly). A key
//! declined with Not now is not asked for again until the worker's handle
//! clears it (R-13).

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
use crate::desktop::{
    services::{
        requests::DesktopRequestId,
        worker::{WorkerEvent, WorkerHandle},
    },
    test_support::fixtures::Fixture,
};

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

/// Calls `provider` on a plain thread for `request`, answers a prompt it
/// sends with `reply`, and gives what the call returned and whether it
/// prompted.
fn ask_through(
    provider: DesktopProvider,
    request: UnlockRequest,
    events: &mut mpsc::UnboundedReceiver<WorkerEvent>,
    reply: PromptReply,
) -> (&'static str, bool) {
    let (done, returned) = std_mpsc::channel();
    let caller = thread::spawn(move || {
        let mut provider = provider;
        let _ = done.send(answer_of(provider.request_passphrase(&request)));
    });
    let deadline = Instant::now() + WAIT;
    let mut prompted = false;
    let mut reply = Some(reply);
    let answer = loop {
        if let Ok(answer) = returned.try_recv() {
            break answer;
        }
        if let Ok(WorkerEvent::Prompt(prompt)) = events.try_recv() {
            prompted = true;
            prompt.answer(reply.take().expect("one prompt per call"));
        }
        assert!(Instant::now() < deadline, "the provider did not return");
        thread::sleep(Duration::from_millis(5));
    };
    caller.join().unwrap();
    (answer, prompted)
}

/// R-13: Not now marks the key declined for the session, so a later
/// request for it answers `Cancelled` at once, with no prompt.
#[test]
fn a_declined_key_answers_cancelled_without_a_prompt() {
    let fixture = Fixture::new();
    let (worker, mut events) = WorkerHandle::spawn(fixture.delaying_backend());
    let directory = tempfile::tempdir().unwrap();
    let request = unlock(&directory.path().join("key"), SharedKeyId::new(), "primary");

    let first = ask_through(
        worker.provider(),
        request.clone(),
        &mut events,
        PromptReply::Cancelled,
    );
    assert_eq!(first, ("cancelled", true));
    let secret = SecretPassphrase::new("correct horse".to_owned()).unwrap();
    let second = ask_through(
        worker.provider(),
        request,
        &mut events,
        PromptReply::Supplied(secret),
    );
    assert_eq!(second, ("cancelled", false));
}

/// R-3: Unlock… clears the declined mark through the worker's handle, and
/// the next request for the key prompts again.
#[test]
fn clearing_a_declined_key_through_the_handle_prompts_again() {
    let fixture = Fixture::new();
    let (worker, mut events) = WorkerHandle::spawn(fixture.delaying_backend());
    let directory = tempfile::tempdir().unwrap();
    let key_id = SharedKeyId::new();
    let request = unlock(&directory.path().join("key"), key_id, "primary");

    let first = ask_through(
        worker.provider(),
        request.clone(),
        &mut events,
        PromptReply::Cancelled,
    );
    assert_eq!(first, ("cancelled", true));
    assert!(worker.clear_declined(key_id));
    let secret = SecretPassphrase::new("correct horse".to_owned()).unwrap();
    let again = ask_through(
        worker.provider(),
        request,
        &mut events,
        PromptReply::Supplied(secret),
    );
    assert_eq!(again, ("supplied", true));
}

/// R-3, R-13, R-17: a key answered `Cancelled` for a request's attempt,
/// declined then or before, is recorded against that request, and Unlock…
/// on its outcome clears it through the request alone; the next request
/// for the key prompts again. Another request's record is not touched.
#[test]
fn a_declined_key_is_cleared_through_its_request() {
    let fixture = Fixture::new();
    let (worker, mut events) = WorkerHandle::spawn(fixture.delaying_backend());
    let directory = tempfile::tempdir().unwrap();
    let key_id = SharedKeyId::new();
    let request = unlock(&directory.path().join("key"), key_id, "primary");
    let (first, second) = (DesktopRequestId(901), DesktopRequestId(902));

    worker.attempting_for_testing(Some(first));
    let declined = ask_through(
        worker.provider(),
        request.clone(),
        &mut events,
        PromptReply::Cancelled,
    );
    assert_eq!(declined, ("cancelled", true));
    // A later request is answered at once, and records the key too.
    worker.attempting_for_testing(Some(second));
    let secret = SecretPassphrase::new("correct horse".to_owned()).unwrap();
    let at_once = ask_through(
        worker.provider(),
        request.clone(),
        &mut events,
        PromptReply::Supplied(secret),
    );
    assert_eq!(at_once, ("cancelled", false));
    worker.attempting_for_testing(None);

    assert!(worker.clear_declined_for(second));
    // Cleared already: the first request's record finds nothing declined.
    assert!(!worker.clear_declined_for(first));
    assert!(!worker.clear_declined_for(second));
    let secret = SecretPassphrase::new("correct horse".to_owned()).unwrap();
    let again = ask_through(
        worker.provider(),
        request,
        &mut events,
        PromptReply::Supplied(secret),
    );
    assert_eq!(again, ("supplied", true));
}

/// C3, K-23: Unlock… clears the mark of the key it names, and only that
/// one. With two keys declined, unlocking A prompts for A again and leaves B
/// answered `Cancelled` at once.
#[test]
fn unlocking_one_key_leaves_another_declined() {
    let fixture = Fixture::new();
    let (worker, mut events) = WorkerHandle::spawn(fixture.delaying_backend());
    let directory = tempfile::tempdir().unwrap();
    let (a_id, b_id) = (SharedKeyId::new(), SharedKeyId::new());
    let a = unlock(&directory.path().join("a"), a_id, "a");
    let b = unlock(&directory.path().join("b"), b_id, "b");
    for request in [&a, &b] {
        let declined = ask_through(
            worker.provider(),
            request.clone(),
            &mut events,
            PromptReply::Cancelled,
        );
        assert_eq!(declined, ("cancelled", true));
    }
    assert!(worker.clear_declined(a_id));

    let secret = SecretPassphrase::new("correct horse".to_owned()).unwrap();
    let again = ask_through(
        worker.provider(),
        a,
        &mut events,
        PromptReply::Supplied(secret),
    );
    assert_eq!(again, ("supplied", true), "A prompts again");
    let secret = SecretPassphrase::new("correct horse".to_owned()).unwrap();
    let still = ask_through(
        worker.provider(),
        b,
        &mut events,
        PromptReply::Supplied(secret),
    );
    assert_eq!(still, ("cancelled", false), "B is still declined");
}
