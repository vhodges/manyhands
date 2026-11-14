//! D1's passphrase prompt by keyboard: Cancel, reached by Tab from the
//! input past Unlock and pressed by Enter, cancels, where Enter used to
//! submit the typed passphrase; no passphrase reaches the key. Unlock reached by Tab and Enter in the input each
//! submit once.
//!
//! A plain thread calls the window's own provider, as the worker does
//! inside a job; no transport is initialized (ruling R11).

use std::{
    fs,
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
    time::Duration,
};

use gpui_kit::{ElementId, TestAppContext, test::TestWindowExt};
use manyhands::repository::{
    SharedKeyId,
    keys::{
        KeySourceToken, PassphraseResponse, SessionCredentialProvider, UnlockReason, UnlockRequest,
    },
};

use crate::desktop::{
    test_support::fixtures::Fixture,
    view::{
        harness::{Shell, wait_until},
        workspace::REGIONS,
    },
};

const TYPED: &str = "typed-passphrase-3b9d";
const WAIT: Duration = Duration::from_secs(10);
/// How many Tab presses may reach a button.
const TAB_LIMIT: usize = 10;

/// A provider call in flight on a plain thread.
struct Asked {
    answer: Receiver<&'static str>,
    caller: JoinHandle<()>,
}

impl Asked {
    fn answered(self) -> &'static str {
        let answer = self
            .answer
            .recv_timeout(WAIT)
            .expect("the provider returned");
        self.caller.join().unwrap();
        answer
    }
}

/// One key: its ID and its file, asked for by `ask`.
struct Key {
    id: SharedKeyId,
    _directory: tempfile::TempDir,
    path: std::path::PathBuf,
}

impl Key {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("key");
        fs::write(&path, b"opaque fixture bytes").unwrap();
        Self {
            id: SharedKeyId::new(),
            _directory: directory,
            path,
        }
    }

    /// Asks the window's provider for this key's passphrase.
    fn ask(&self, shell: &Shell, cx: &mut TestAppContext) -> Asked {
        let request = UnlockRequest {
            key_id: self.id,
            label: "primary".to_owned(),
            source: KeySourceToken::observe(&self.path).unwrap(),
            reason: UnlockReason::ProtectedKey,
        };
        let mut provider = shell
            .workspace
            .read_with(cx, |workspace, _| workspace.services().worker().provider());
        let (done, answer) = mpsc::channel();
        let caller = thread::spawn(move || {
            let answer = match provider.request_passphrase(&request) {
                PassphraseResponse::Supplied(_) => "supplied",
                PassphraseResponse::Cancelled => "cancelled",
                PassphraseResponse::Unavailable => "unavailable",
            };
            let _ = done.send(answer);
        });
        Asked { answer, caller }
    }
}

fn shell(cx: &mut TestAppContext) -> Shell {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    fixture.create_repository("a");
    Shell::open(fixture, cx)
}

async fn until_shown(shell: &Shell, cx: &mut TestAppContext) {
    wait_until(cx, &shell.workspace, |workspace, _| {
        workspace.passphrase().is_some()
    })
    .await;
    shell.run(cx);
}

fn shown(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.passphrase().is_some())
}

/// Types `text` into the dialog's input, which holds focus.
fn type_text(shell: &Shell, text: &str, cx: &mut TestAppContext) {
    shell.visual(cx).simulate_input(text);
    shell.run(cx);
}

/// Tab until the Kit button of `id` holds focus, on screen, with no
/// region holding it: the dialog traps it.
fn tab_to(shell: &Shell, id: &'static str, cx: &mut TestAppContext) {
    let button = ElementId::from((id, 0usize));
    let focused = |cx: &mut TestAppContext| {
        shell.frame(cx, |window, _| {
            window
                .try_find(button.clone())
                .and_then(|element| element.focused())
                == Some(true)
        })
    };
    let mut presses = 0;
    while !focused(cx) {
        presses += 1;
        assert!(presses <= TAB_LIMIT, "Tab never reached {id}");
        shell.keys("tab", cx);
    }
    shell.frame(cx, |window, _| {
        for region in REGIONS {
            assert_ne!(window.find(region.id()).focused(), Some(true));
        }
        assert!(window.find(button.clone()).visible(), "{id} is on screen");
    });
}

/// With a passphrase typed, Tab to Cancel and Enter answers `Cancelled`
/// once and closes the prompt: no passphrase reaches the key, which stays
/// locked, and nothing else runs.
#[gpui_kit::test]
async fn cancel_by_tab_and_enter_cancels_and_the_key_stays_locked(cx: &mut TestAppContext) {
    let shell = shell(cx);
    let key = Key::new();
    let asked = key.ask(&shell, cx);
    until_shown(&shell, cx).await;
    type_text(&shell, TYPED, cx);

    tab_to(&shell, "passphrase-cancel", cx);
    shell.press("enter", cx);
    assert_eq!(asked.answered(), "cancelled");
    assert!(!shell.present("passphrase-dialog", cx));
    assert!(!shown(&shell, cx));
    assert!(shell.requests(cx, |_| true).is_empty(), "nothing else ran");
}

/// Enter in the input, a passphrase typed, still submits, once: the
/// prompt closes, and the next ask is answered from its own prompt.
#[gpui_kit::test]
async fn enter_in_the_input_still_submits_once(cx: &mut TestAppContext) {
    let shell = shell(cx);
    let key = Key::new();
    let asked = key.ask(&shell, cx);
    until_shown(&shell, cx).await;
    type_text(&shell, TYPED, cx);
    shell.press("enter", cx);
    assert_eq!(asked.answered(), "supplied");
    assert!(!shown(&shell, cx));

    shell.run(cx);
    assert!(!shown(&shell, cx), "no second prompt");
    assert!(shell.requests(cx, |_| true).is_empty(), "nothing else ran");
}

/// Unlock reached by Tab and pressed by Enter submits once: no second
/// prompt, nothing pressed behind the closed dialog.
#[gpui_kit::test]
async fn unlock_by_tab_and_enter_submits_once(cx: &mut TestAppContext) {
    let shell = shell(cx);
    let key = Key::new();
    let asked = key.ask(&shell, cx);
    until_shown(&shell, cx).await;
    type_text(&shell, TYPED, cx);
    tab_to(&shell, "passphrase-ok", cx);
    shell.press("enter", cx);
    assert_eq!(asked.answered(), "supplied");
    shell.run(cx);
    assert!(!shell.present("passphrase-dialog", cx));
    assert!(!shown(&shell, cx));
    assert!(shell.requests(cx, |_| true).is_empty(), "nothing else ran");
}
