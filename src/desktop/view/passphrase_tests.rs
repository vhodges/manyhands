//! The passphrase dialog over the real bridge. A plain thread calls the
//! window's own provider, as the worker thread does inside a job; its
//! prompt comes through the worker's event channel and the drain task to
//! the workspace, which shows the dialog. No transport is initialized and
//! nothing reaches a network (ruling R11): the library asks for a
//! passphrase only over SSH, so the prompt is raised by the provider
//! itself.

use std::{
    fs,
    path::Path,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread::{self, JoinHandle},
    time::Duration,
};

use futures::channel::oneshot;
use gpui_kit::{
    FocusHandle, TestAppContext,
    component::{WindowExt, input::InputState},
};
use manyhands::repository::{
    SharedKeyId,
    keys::{
        KeySourceToken, PassphraseResponse, SessionCredentialProvider, UnlockReason, UnlockRequest,
    },
    transport::SshAuthority,
};

use super::{CANCEL, REFUSED, UNLOCK, label_for};
use crate::desktop::{
    keys::Quit,
    services::{
        JobKind, ReadKind,
        prompts::{PromptReply, PromptRequest},
        requests::{Intent, Lane, RequestState},
    },
    test_support::{
        delaying::JobGate,
        fixtures::{Fixture, SSH_HOST, SSH_PORT},
    },
    view::{
        confirm::{CONFIRM, confirm},
        harness::{Shell, wait_until},
        workspace::{QuitPhase, Region},
    },
};

/// A host key fingerprint in the one form a pin holds.
const FINGERPRINT: &str = "SHA256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

const SENTINEL: &str = "sentinel-passphrase-7f3a";
const WAIT: Duration = Duration::from_secs(10);

/// A provider call in flight on a plain thread.
struct Asked {
    answer: Receiver<&'static str>,
    /// The answer, once a look found it.
    got: Option<&'static str>,
    caller: JoinHandle<()>,
    _directory: tempfile::TempDir,
}

impl Asked {
    /// What the call returned, once it has.
    fn answered(self) -> &'static str {
        let answer = match self.got {
            Some(answer) => answer,
            None => self
                .answer
                .recv_timeout(WAIT)
                .expect("the provider returned"),
        };
        self.caller.join().unwrap();
        answer
    }

    /// Whether the call is still waiting for its reply.
    fn waiting(&mut self) -> bool {
        if self.got.is_none() {
            match self.answer.try_recv() {
                Ok(answer) => self.got = Some(answer),
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => panic!("the provider call ended"),
            }
        }
        self.got.is_none()
    }
}

/// Asks the window's provider for the passphrase of the key labelled
/// `label`, from a plain thread.
fn ask(shell: &Shell, label: &str, cx: &mut TestAppContext) -> Asked {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("key");
    fs::write(&path, b"opaque fixture bytes").unwrap();
    let request = UnlockRequest {
        key_id: SharedKeyId::new(),
        label: label.to_owned(),
        source: KeySourceToken::observe(&path).unwrap(),
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
    Asked {
        answer,
        got: None,
        caller,
        _directory: directory,
    }
}

/// Waits until the workspace shows the passphrase dialog.
async fn until_shown(shell: &Shell, cx: &mut TestAppContext) {
    wait_until(cx, &shell.workspace, |workspace, _| {
        workspace.passphrase().is_some()
    })
    .await;
    shell.run(cx);
}

/// Types `text` into the dialog's input, which holds focus.
fn type_text(shell: &Shell, text: &str, cx: &mut TestAppContext) {
    shell.visual(cx).simulate_input(text);
    shell.run(cx);
}

fn input(shell: &Shell, cx: &mut TestAppContext) -> gpui_kit::Entity<InputState> {
    let dialog = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.passphrase())
        .expect("the dialog is open");
    dialog.read_with(cx, |dialog, _| dialog.input().expect("the input is held"))
}

fn focused(shell: &Shell, cx: &mut TestAppContext) -> Option<FocusHandle> {
    shell.frame(cx, |window, cx| window.focused(cx))
}

/// The dialog shows "Passphrase for <label>" over a masked input; Enter
/// supplies exactly one passphrase. The input is emptied and released,
/// and the dialog with it.
#[gpui_kit::test]
async fn submit_supplies_the_passphrase_and_releases_the_input(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    fixture.create_repository("a");
    let shell = Shell::open(fixture, cx);
    let mut asked = ask(&shell, "primary", cx);
    until_shown(&shell, cx).await;

    assert_eq!(
        shell.label("passphrase-label", cx).as_deref(),
        Some(label_for("primary").as_str())
    );
    assert_eq!(label_for("primary"), "Passphrase for primary");
    assert_eq!(
        shell.label("passphrase-reason", cx).as_deref(),
        Some(UnlockReason::ProtectedKey.guidance())
    );
    assert_eq!(shell.label("passphrase-ok", cx).as_deref(), Some(UNLOCK));
    assert_eq!(
        shell.label("passphrase-cancel", cx).as_deref(),
        Some(CANCEL)
    );
    // No mask toggle: the input offers nothing but its masked text.
    assert!(!shell.present("passphrase-mask-toggle", cx));

    // Only weak handles: the test must not keep them alive itself.
    let input = input(&shell, cx).downgrade();
    let dialog = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.passphrase())
        .unwrap()
        .downgrade();
    type_text(&shell, SENTINEL, cx);
    assert!(asked.waiting());
    shell.keys("enter", cx);

    assert_eq!(asked.answered(), "supplied");
    assert!(!shell.present("passphrase-dialog", cx));
    shell.run(cx);
    assert!(input.upgrade().is_none(), "the input entity is released");
    assert!(dialog.upgrade().is_none(), "the dialog entity is released");
    assert!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.passphrase())
            .is_none()
    );
}

/// The input is set empty before it is let go, whoever still holds it.
#[gpui_kit::test]
async fn submit_empties_the_input_before_letting_it_go(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    fixture.create_repository("a");
    let shell = Shell::open(fixture, cx);
    let asked = ask(&shell, "primary", cx);
    until_shown(&shell, cx).await;
    // The test keeps the input alive past the dialog, to read it.
    let input = input(&shell, cx);
    type_text(&shell, SENTINEL, cx);
    shell.keys("enter", cx);

    assert_eq!(asked.answered(), "supplied");
    assert_eq!(
        input.read_with(cx, |input, _| input.value().to_string()),
        ""
    );
    let released = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.passphrase())
        .is_none();
    assert!(released);
}

/// An empty passphrase and one containing NUL are refused in the dialog,
/// as `SecretPassphrase::new` would refuse them: nothing is replied, the
/// dialog stays, and says why.
#[gpui_kit::test]
async fn empty_and_nul_passphrases_are_refused(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    fixture.create_repository("a");
    let shell = Shell::open(fixture, cx);
    let mut asked = ask(&shell, "primary", cx);
    until_shown(&shell, cx).await;
    assert!(!shell.present("passphrase-refused", cx));

    shell.keys("enter", cx);
    assert!(shell.present("passphrase-dialog", cx));
    assert_eq!(
        shell.label("passphrase-refused", cx).as_deref(),
        Some(REFUSED)
    );
    assert!(asked.waiting());

    let input = input(&shell, cx);
    shell.visual(cx).update(|window, cx| {
        input.update(cx, |input, cx| input.set_value("bad\0pass", window, cx));
    });
    shell.click("passphrase-ok", cx);
    assert!(shell.present("passphrase-dialog", cx));
    assert_eq!(
        shell.label("passphrase-refused", cx).as_deref(),
        Some(REFUSED)
    );
    assert!(asked.waiting());

    shell.click("passphrase-cancel", cx);
    assert_eq!(asked.answered(), "cancelled");
}

/// Escape replies `Cancelled`, and focus goes back where it was when the
/// prompt opened.
#[gpui_kit::test]
async fn escape_cancels_and_focus_returns(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    fixture.create_repository("a");
    let shell = Shell::open(fixture, cx);
    shell.keys("secondary-4", cx);
    let before = focused(&shell, cx).expect("the operation area holds focus");
    let asked = ask(&shell, "primary", cx);
    until_shown(&shell, cx).await;
    assert_ne!(focused(&shell, cx), Some(before.clone()));

    shell.keys("escape", cx);
    assert_eq!(asked.answered(), "cancelled");
    assert!(!shell.present("passphrase-dialog", cx));
    assert_eq!(focused(&shell, cx), Some(before));
    assert!(shell.present(Region::Operations.ring_id(), cx));
}

/// Cancel replies `Cancelled` too.
#[gpui_kit::test]
async fn cancel_replies_cancelled(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    fixture.create_repository("a");
    let shell = Shell::open(fixture, cx);
    let asked = ask(&shell, "primary", cx);
    until_shown(&shell, cx).await;
    shell.click("passphrase-cancel", cx);
    assert_eq!(asked.answered(), "cancelled");
    assert!(!shell.present("passphrase-dialog", cx));
}

/// A prompt that arrives while the confirmation dialog is open shows
/// above it. Escape answers the prompt, and only the prompt: the
/// confirmation dialog stays, and its own Escape then gives focus back to
/// the control that started it.
#[gpui_kit::test]
async fn a_prompt_above_the_confirmation_dialog_is_answered_by_its_own_dialog(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let ticket = "01ARZ3NDEKTSV4RRFFQ69G5FH0";
    fixture.create_ticket(&root, ticket, "Stranded", "open");
    let operation = fixture.strand_ticket_save(&root, ticket, "Edited");
    let shell = Shell::open(fixture, cx);
    shell.click(("pending-abandon", 0usize), cx);
    assert!(shell.present("confirm-dialog", cx));

    let asked = ask(&shell, "primary", cx);
    until_shown(&shell, cx).await;
    assert!(shell.present("passphrase-dialog", cx));
    assert!(shell.present("confirm-dialog", cx));

    shell.keys("escape", cx);
    assert_eq!(asked.answered(), "cancelled");
    assert!(!shell.present("passphrase-dialog", cx));
    assert!(shell.present("confirm-dialog", cx));

    shell.keys("escape", cx);
    assert!(!shell.present("confirm-dialog", cx));
    let abandon = shell
        .workspace
        .read_with(cx, |workspace, _| {
            workspace.abandon_focus(root.to_str().unwrap(), &operation)
        })
        .expect("the Abandon control has a focus");
    assert!(shell.frame(cx, |window, _| abandon.is_focused(window)));
    // Nothing was submitted by either dialog.
    assert!(shell.requests(cx, |_| true).is_empty());
}

/// Holds a refresh of `root` on the worker, so quitting waits for it.
fn hold_refresh(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> JobGate {
    let root = root.to_owned();
    let held = shell.backend.hold_job(JobKind::IndexRefresh);
    shell.workspace.read_with(cx, |workspace, _| {
        workspace
            .services()
            .worker()
            .submit(Intent::IndexRefresh { root }, Lane::User, None, None)
    });
    held.wait_reached();
    held
}

/// Quitting with a prompt open answers it `Unavailable` and closes it:
/// "Finishing" shows, and the app quits once the job returns.
#[gpui_kit::test]
async fn quit_answers_an_open_prompt_unavailable(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let shell = Shell::open(fixture, cx);
    let held = hold_refresh(&shell, &root, cx);
    let asked = ask(&shell, "primary", cx);
    until_shown(&shell, cx).await;

    shell.visual(cx).dispatch_action(Quit);
    shell.run(cx);
    assert_eq!(asked.answered(), "unavailable");
    assert!(!shell.present("passphrase-dialog", cx));
    assert!(shell.present("quit-finishing", cx));

    held.release();
    wait_until(cx, &shell.workspace, |workspace, _| {
        workspace.quit_phase() == QuitPhase::Quit
    })
    .await;
}

/// The window's close request goes through the same handler: it answers
/// the open prompt `Unavailable`.
#[gpui_kit::test]
async fn closing_the_window_answers_an_open_prompt_unavailable(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let shell = Shell::open(fixture, cx);
    let held = hold_refresh(&shell, &root, cx);
    let asked = ask(&shell, "primary", cx);
    until_shown(&shell, cx).await;

    assert!(!shell.visual(cx).simulate_close());
    assert_eq!(asked.answered(), "unavailable");
    assert!(!shell.present("passphrase-dialog", cx));

    held.release();
    wait_until(cx, &shell.workspace, |workspace, _| {
        workspace.quit_phase() == QuitPhase::Quit
    })
    .await;
}

/// Quitting saves an unsaved draft first: an open prompt's dialog closes,
/// so nothing stands in front of Quit now, but the prompt is held, not
/// answered. When the drafts are saved and quitting proceeds, it is
/// answered `Unavailable`.
#[gpui_kit::test]
async fn a_prompt_held_while_drafts_save_is_answered_once_quitting_proceeds(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let shell = Shell::open(Fixture::new(), cx);
    shell.unsaved_draft(cx).await;
    let mut asked = ask(&shell, "primary", cx);
    until_shown(&shell, cx).await;
    let gate = shell.backend.hold_read(ReadKind::DraftWrite);

    shell.visual(cx).dispatch_action(Quit);
    shell.run(cx);
    let phase = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.quit_phase());
    assert_eq!(phase, QuitPhase::SavingDrafts);
    assert!(!shell.present("passphrase-dialog", cx));
    assert!(shell.present("quit-now", cx));
    assert!(asked.waiting(), "the prompt is held, not answered");

    let _ = gate.send(());
    shell.run(cx);
    let phase = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.quit_phase());
    assert_eq!(phase, QuitPhase::Quit);
    assert_eq!(asked.answered(), "unavailable");
}

/// A prompt that arrives while drafts save is held: no dialog, no answer,
/// until quitting proceeds and answers it `Unavailable`.
#[gpui_kit::test]
async fn a_prompt_arriving_while_drafts_save_is_held(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let shell = Shell::open(Fixture::new(), cx);
    shell.unsaved_draft(cx).await;
    let gate = shell.backend.hold_read(ReadKind::DraftWrite);
    shell.visual(cx).dispatch_action(Quit);
    shell.run(cx);

    let mut asked = ask(&shell, "primary", cx);
    let operations = shell.operations(cx);
    wait_until(cx, &operations, |operations, _| operations.has_prompt()).await;
    shell.run(cx);
    assert!(asked.waiting(), "the prompt is held, not answered");
    assert!(!shell.present("passphrase-dialog", cx));

    let _ = gate.send(());
    shell.run(cx);
    assert_eq!(asked.answered(), "unavailable");
}

/// Quitting abandoned: with `drafts/` read-only the flush fails and the
/// dialog asks; Cancel shows the prompt held meanwhile again, unanswered,
/// and it is still the prompt's own to answer.
#[cfg(unix)]
#[gpui_kit::test]
async fn a_prompt_held_while_drafts_save_shows_again_on_cancel(cx: &mut TestAppContext) {
    use std::os::unix::fs::PermissionsExt;

    cx.executor().allow_parking();
    let shell = Shell::open(Fixture::new(), cx);
    shell.unsaved_draft(cx).await;
    let mut asked = ask(&shell, "primary", cx);
    until_shown(&shell, cx).await;
    let folder = shell.fixture.data_dir().join("drafts");
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o500)).unwrap();

    shell.visual(cx).dispatch_action(Quit);
    shell.run(cx);
    assert!(shell.present("drafts-not-saved-message", cx));
    assert!(!shell.present("passphrase-dialog", cx));
    assert!(asked.waiting(), "the prompt is held through the dialog");

    shell.click("drafts-not-saved-cancel", cx);
    shell.run(cx);
    assert!(!shell.present("drafts-not-saved-message", cx));
    assert!(shell.present("passphrase-dialog", cx));
    assert!(asked.waiting(), "Cancel shows the prompt, unanswered");
    let phase = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.quit_phase());
    assert_eq!(phase, QuitPhase::Running);

    shell.keys("escape", cx);
    assert_eq!(asked.answered(), "cancelled");
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o700)).unwrap();
}

/// A prompt that arrives once quitting has begun is answered
/// `Unavailable` at once, with no dialog.
#[gpui_kit::test]
async fn a_prompt_while_finishing_is_answered_unavailable(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let shell = Shell::open(fixture, cx);
    let held = hold_refresh(&shell, &root, cx);
    shell.visual(cx).dispatch_action(Quit);
    shell.run(cx);
    assert!(shell.present("quit-finishing", cx));

    let mut asked = ask(&shell, "primary", cx);
    // The prompt reaches the window through the drain task, which the
    // test runs while it waits.
    let deadline = std::time::Instant::now() + WAIT;
    while asked.waiting() {
        assert!(
            std::time::Instant::now() < deadline,
            "no answer in {WAIT:?}"
        );
        shell.run(cx);
        cx.executor().timer(Duration::from_millis(10)).await;
    }
    assert_eq!(asked.answered(), "unavailable");
    shell.run(cx);
    assert!(!shell.present("passphrase-dialog", cx));

    held.release();
    wait_until(cx, &shell.workspace, |workspace, _| {
        workspace.quit_phase() == QuitPhase::Quit
    })
    .await;
}

/// Every piece of text a test can read off the window: each observed
/// element's label and accessibility value.
fn window_text(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    shell.frame(cx, |window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .flat_map(|snapshot| [snapshot.label(), snapshot.value()])
            .flatten()
            .map(str::to_owned)
            .collect()
    })
}

/// Review Focus 5: a supplied passphrase reaches no request field, no
/// operation-area text, no notification and nothing the window shows,
/// while it is typed and after it is supplied, with a job run alongside.
#[gpui_kit::test]
async fn a_supplied_passphrase_is_shown_or_kept_nowhere(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    let held = hold_refresh(&shell, &root, cx);
    let asked = ask(&shell, "primary", cx);
    until_shown(&shell, cx).await;
    type_text(&shell, SENTINEL, cx);
    // Masked: not even the input's accessibility value carries it.
    let typed = window_text(&shell, cx);
    assert!(
        typed.iter().all(|text| !text.contains(SENTINEL)),
        "{typed:?}"
    );

    shell.keys("enter", cx);
    assert_eq!(asked.answered(), "supplied");
    held.release();
    let id = shell.requests(cx, |_| true)[0];
    shell.until_ended(id, cx).await;

    let operations = shell.operations(cx);
    let fields = operations.read_with(cx, |operations, _| {
        let mut fields = Vec::new();
        for request in operations.requests() {
            fields.push(format!("{:?}", request.id));
            fields.push(request.request_id.to_string());
            fields.push(request.intent.label().to_owned());
            fields.extend(request.intent.root().map(|root| root.display().to_string()));
            fields.extend(request.confirmation.map(|id| id.to_string()));
            fields.push(format!("{:?} {}", request.lane, request.attempt));
            match &request.state {
                RequestState::Finished(outcome, code, effects, recovery, message) => {
                    fields.push(format!("{outcome:?} {code:?} {effects:?} {recovery:?}"));
                    fields.push(message.clone());
                }
                RequestState::NotSent(why) => fields.push(format!("{why:?}")),
                _ => {}
            }
        }
        fields
    });
    assert!(!fields.is_empty());
    assert!(
        fields.iter().all(|field| !field.contains(SENTINEL)),
        "{fields:?}"
    );

    shell.keys("secondary-4", cx);
    let shown = window_text(&shell, cx);
    assert!(!shown.is_empty());
    assert!(
        shown.iter().all(|text| !text.contains(SENTINEL)),
        "{shown:?}"
    );
    let notifications = shell.frame(cx, |window, cx| window.notifications(cx).len());
    assert_eq!(notifications, 0);
}

/// Carried from Task 11: one drain batch holds the `Finished` of the
/// confirmation dialog's own request and the prompt of the job after it.
/// The confirmation dialog closes, and only it: the prompt is shown, stays
/// open, and is answered from its own dialog. The prompt is placed in the
/// worker's channel directly, as the provider places it, so that the two
/// events are drained together; no transport is initialized, so the host
/// approval fails at once and reaches no network (ruling R11).
#[gpui_kit::test]
async fn a_finished_confirmation_and_a_prompt_in_one_batch_close_only_the_confirmation(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.publish_over_ssh(&root);
    let shell = Shell::open(fixture, cx);
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    let returned = shell.backend.hold_job_return(JobKind::HostApprove);
    let next = shell.backend.hold_job(JobKind::IndexRefresh);
    let origin = cx.update(|cx| cx.focus_handle());
    let intent = Intent::HostApprove {
        root: root.clone(),
        authority: SshAuthority {
            host: SSH_HOST.to_owned(),
            port: SSH_PORT,
        },
        algorithm: "ssh-ed25519".to_owned(),
        fingerprint: FINGERPRINT.to_owned(),
    };
    shell
        .visual(cx)
        .update(|window, cx| confirm(intent, origin, &services, window, cx));
    shell.run(cx);
    assert_eq!(shell.label("confirm-ok", cx).as_deref(), Some(CONFIRM));
    shell.click("confirm-ok", cx);

    // The approval has returned on the worker; its answer is not yet sent.
    returned.wait_reached();
    services.worker().submit(
        Intent::IndexRefresh { root: root.clone() },
        Lane::User,
        None,
        None,
    );
    let (reply, mut answer) = oneshot::channel();
    services.worker().send_prompt(PromptRequest {
        key_id: SharedKeyId::new(),
        label: "next".to_owned(),
        reason: UnlockReason::ProtectedKey,
        reply,
    });
    returned.release();
    // The worker has taken the next job, so the approval's `Finished` is
    // in the channel behind the prompt; nothing has drained either.
    next.wait_reached();
    shell.run(cx);

    assert!(!shell.present("confirm-dialog", cx));
    assert!(shell.present("passphrase-dialog", cx));
    assert_eq!(
        shell.label("passphrase-label", cx).as_deref(),
        Some(label_for("next").as_str())
    );
    assert!(matches!(answer.try_recv(), Ok(None)), "not yet answered");
    type_text(&shell, SENTINEL, cx);
    shell.keys("enter", cx);
    assert!(matches!(
        answer.try_recv(),
        Ok(Some(PromptReply::Supplied(_)))
    ));
    assert!(!shell.present("passphrase-dialog", cx));
    next.release();
}
