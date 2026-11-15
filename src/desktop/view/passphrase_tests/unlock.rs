//! Unlock key's key card and its refusal (Wave 04 Cycle 05a design Part D,
//! R-16 and R-17): P18 and P19. Each table's rows are read in a window of
//! their own, the prompt placed in the worker's channel as the provider
//! places it, for a key the fixture registered through the library. Focus
//! is read after two draws.
//!
//! P18's failed read is applied through the dialog's test hook
//! (`answer_key_read`, U-8): no test backend can fail a read. No message
//! prints a typed passphrase; P18 types none, and P19 submits an empty one.

use futures::channel::oneshot;
use gpui_kit::{ElementId, TestAppContext, test::TestWindowExt};
use manyhands::{
    repository::{KeyListDto, SharedKeyId, keys::UnlockReason},
    results::ResultCode,
};

use super::REFUSED;
use crate::desktop::{
    keys::Quit,
    services::{
        ReadFailure, ReadKind, ReadResult,
        prompts::{PromptReply, PromptRequest},
    },
    test_support::fixtures::Fixture,
    view::{frame::tests::Table, harness::Shell, workspace::QuitPhase},
};

/// The labels of the keys the rows register.
const SIGNING: &str = "signing";
const NO_PUBLIC: &str = "laptop";
/// A prompt's label for a key the list does not hold.
const ELSEWHERE: &str = "elsewhere";

/// How many Tab presses may reach a button.
const TAB_LIMIT: usize = 8;

/// What the key card shows: the key's label and its fingerprint, each when
/// drawn.
#[derive(Debug, PartialEq)]
struct Card {
    label: Option<String>,
    fingerprint: Option<String>,
}

impl Card {
    fn label_only(label: &str) -> Self {
        Self {
            label: Some(label.to_owned()),
            fingerprint: None,
        }
    }

    fn read(shell: &Shell, cx: &mut TestAppContext) -> Self {
        Self {
            label: shell.label("passphrase-key-label", cx),
            fingerprint: shell.label("passphrase-fingerprint", cx),
        }
    }
}

/// The fingerprint the library lists for `key`, which it must have.
fn listed_fingerprint(shell: &Shell, key: SharedKeyId) -> String {
    let id = key.to_string();
    shell
        .fixture
        .service()
        .list_keys()
        .unwrap()
        .items
        .into_iter()
        .find(|listed| listed.id == id)
        .and_then(|listed| listed.fingerprint)
        .expect("the generated key records a fingerprint")
}

/// Places a prompt for `key` labelled `label`, as the provider places one,
/// and checks that its dialog opened.
fn ask(
    shell: &Shell,
    key: SharedKeyId,
    label: &str,
    cx: &mut TestAppContext,
) -> oneshot::Receiver<PromptReply> {
    let (reply, answer) = oneshot::channel();
    shell.workspace.read_with(cx, |workspace, _| {
        workspace.services().worker().send_prompt(PromptRequest {
            key_id: key,
            label: label.to_owned(),
            reason: UnlockReason::ProtectedKey,
            reply,
        })
    });
    shell.run(cx);
    assert!(
        shell.present("passphrase-dialog", cx),
        "the dialog for {label} opened"
    );
    answer
}

/// A window over a fixture with the key `SIGNING` generated, and its ID.
fn signing(cx: &mut TestAppContext) -> (Shell, SharedKeyId) {
    let fixture = Fixture::new();
    let key = fixture.generate_key(SIGNING);
    (Shell::open(fixture, cx), key)
}

/// P18: the key card names the key by its label and, once the key list has
/// it, its fingerprint; the label alone before the list lands, when the
/// read fails, when the list holds no such key, or when the key records no
/// fingerprint; and a list landing after the prompt was withdrawn or
/// answered changes nothing.
#[gpui_kit::test]
async fn the_unlock_dialog_names_the_key(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let mut table = Table::new("P18");

    // After the read: the label and the listed fingerprint.
    let (shell, key) = signing(cx);
    let fingerprint = listed_fingerprint(&shell, key);
    let _answer = ask(&shell, key, SIGNING, cx);
    let card = Card::read(&shell, cx);
    table.fact(
        "after the read",
        card == Card {
            label: Some(SIGNING.to_owned()),
            fingerprint: Some(fingerprint.clone()),
        },
        &format!("the label and the fingerprint shown (it shows {card:?})"),
    );

    // Before the read: the label only, then the fingerprint once it lands.
    let (shell, key) = signing(cx);
    let fingerprint = listed_fingerprint(&shell, key);
    let held = shell.backend.hold_read(ReadKind::Keys);
    let _answer = ask(&shell, key, SIGNING, cx);
    let card = Card::read(&shell, cx);
    table.fact(
        "before the read",
        card == Card::label_only(SIGNING),
        &format!("the label only (it shows {card:?})"),
    );
    let _ = held.send(());
    shell.run(cx);
    let card = Card::read(&shell, cx);
    table.fact(
        "before the read, once it lands",
        card.fingerprint.as_deref() == Some(fingerprint.as_str()),
        &format!("the fingerprint shown (it shows {card:?})"),
    );

    // The read fails: the label only, also once the held read is let go.
    let (shell, key) = signing(cx);
    let held = shell.backend.hold_read(ReadKind::Keys);
    let _answer = ask(&shell, key, SIGNING, cx);
    let dialog = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.passphrase())
        .expect("the dialog is open");
    dialog.update(cx, |dialog, cx| {
        dialog.answer_key_read(
            ReadResult::<KeyListDto>::Failed(ReadFailure {
                code: ResultCode::IndexUnavailable,
                recovery: Vec::new(),
            }),
            cx,
        )
    });
    let _ = held.send(());
    shell.run(cx);
    let card = Card::read(&shell, cx);
    table.fact(
        "the read fails",
        card == Card::label_only(SIGNING),
        &format!("the label only (it shows {card:?})"),
    );

    // No key in the list has the prompt's ID.
    let (shell, _key) = signing(cx);
    let _answer = ask(&shell, SharedKeyId::new(), ELSEWHERE, cx);
    let card = Card::read(&shell, cx);
    table.fact(
        "no matching key",
        card == Card::label_only(ELSEWHERE),
        &format!("the label only (it shows {card:?})"),
    );

    // The key records no fingerprint.
    let fixture = Fixture::new();
    let bare = fixture.import_key_without_public_key(NO_PUBLIC);
    let shell = Shell::open(fixture, cx);
    let recorded = shell.fixture.service().list_keys().unwrap().items;
    assert!(
        recorded
            .iter()
            .any(|listed| listed.id == bare.to_string() && listed.fingerprint.is_none()),
        "the key without a public key records no fingerprint"
    );
    let _answer = ask(&shell, bare, NO_PUBLIC, cx);
    let card = Card::read(&shell, cx);
    table.fact(
        "no fingerprint",
        card == Card::label_only(NO_PUBLIC),
        &format!("the label only (it shows {card:?})"),
    );

    // Answered (Not now) while the read is held: its landing changes
    // nothing. The test holds the dialog past its close.
    let (shell, key) = signing(cx);
    let held = shell.backend.hold_read(ReadKind::Keys);
    let mut answer = ask(&shell, key, SIGNING, cx);
    let dialog = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.passphrase())
        .expect("the dialog is open");
    shell.keys("escape", cx);
    assert!(
        matches!(answer.try_recv(), Ok(Some(PromptReply::Cancelled))),
        "Not now answered the prompt"
    );
    let _ = held.send(());
    shell.run(cx);
    let kept = dialog.read_with(cx, |dialog, _| dialog.fingerprint().map(str::to_owned));
    table.fact(
        "answered, then the read lands",
        kept.is_none(),
        &format!("no fingerprint taken (it took {kept:?})"),
    );

    // Withdrawn while the read is held, as quitting holds a prompt while it
    // saves drafts: its landing changes nothing. Last, since it quits.
    let fixture = Fixture::new();
    let key = fixture.generate_key(SIGNING);
    let shell = Shell::open(fixture, cx);
    shell.unsaved_draft(cx).await;
    let held = shell.backend.hold_read(ReadKind::Keys);
    let mut answer = ask(&shell, key, SIGNING, cx);
    let dialog = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.passphrase())
        .expect("the dialog is open");
    let drafts = shell.backend.hold_read(ReadKind::DraftWrite);
    shell.visual(cx).dispatch_action(Quit);
    shell.run(cx);
    assert!(
        !shell.present("passphrase-dialog", cx),
        "quitting closed the dialog"
    );
    assert!(
        matches!(answer.try_recv(), Ok(None)),
        "the prompt is held, not answered"
    );
    let _ = held.send(());
    shell.run(cx);
    let kept = dialog.read_with(cx, |dialog, _| dialog.fingerprint().map(str::to_owned));
    table.fact(
        "withdrawn, then the read lands",
        kept.is_none(),
        &format!("no fingerprint taken (it took {kept:?})"),
    );
    let _ = drafts.send(());
    shell.run(cx);
    let phase = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.quit_phase());
    assert_eq!(phase, QuitPhase::Quit, "quitting went on");

    table.done();
}

/// Whether focus is on the element `id` or inside it, after two draws.
fn focus_in(shell: &Shell, id: &'static str, cx: &mut TestAppContext) -> bool {
    for _ in 0..2 {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
    }
    shell.frame(cx, |window, _| {
        window.try_find(id).is_some_and(|wrapper| {
            gpui_kit::base::test_support::has_observed_focus(window, wrapper.path())
        })
    })
}

/// Tab until the Kit button of `id` holds focus.
fn tab_to(shell: &Shell, id: &'static str, cx: &mut TestAppContext) {
    let button = ElementId::from((id, 0usize));
    for _ in 0..TAB_LIMIT {
        let on = shell.frame(cx, |window, _| {
            window
                .try_find(button.clone())
                .and_then(|element| element.focused())
                == Some(true)
        });
        if on {
            return;
        }
        shell.keys("tab", cx);
    }
    panic!("Tab never reached {id}");
}

/// P19: an empty passphrase submitted by the Unlock button, pressed by
/// Enter or by Space while it holds focus, is refused: the refusal shows,
/// focus goes back into the passphrase field, and the prompt is not
/// answered.
#[gpui_kit::test]
fn a_refused_passphrase_keeps_focus_in_the_field(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let mut table = Table::new("P19");
    for key in ["enter", "space"] {
        let (shell, id) = signing(cx);
        let mut answer = ask(&shell, id, SIGNING, cx);
        tab_to(&shell, "passphrase-ok", cx);
        shell.press(key, cx);
        let row = format!("Unlock by {key}");
        table.fact(
            &format!("{row}, the dialog stays"),
            shell.present("passphrase-dialog", cx),
            "the dialog open",
        );
        let refused = shell.label("passphrase-refused", cx);
        table.fact(
            &format!("{row}, refused"),
            refused.as_deref() == Some(REFUSED),
            &format!("the refusal shown (it shows {refused:?})"),
        );
        table.fact(
            &format!("{row}, focus"),
            focus_in(&shell, "passphrase-input", cx),
            "focus in the passphrase field",
        );
        table.fact(
            &format!("{row}, no reply"),
            matches!(answer.try_recv(), Ok(None)),
            "the prompt not answered",
        );
    }
    table.done();
}
