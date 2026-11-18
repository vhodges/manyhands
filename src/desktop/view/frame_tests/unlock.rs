//! P1's and P2's rows for Unlock key (Wave 04 Cycle 05a design A3, A4 and
//! Part D). The prompt is placed in the worker's channel, as the provider
//! places it, over a window whose list holds focus. On open, focus is in
//! the passphrase field; Escape answers Not now and closes the dialog; Enter
//! in the field, a passphrase typed, supplies it and closes the dialog; each
//! gives focus back to the list. Its footer is Not now, then Unlock, and Tab
//! from the field reaches them in that order.
//!
//! No row prints what was typed.

use futures::channel::oneshot;
use gpui_kit::{ElementId, TestAppContext};

use super::{
    Table,
    dialogs::{back_at, inside, listed, origin},
    tab_order,
};
use crate::desktop::{services::prompts::PromptReply, view::harness::Shell};

/// What the rows type into the passphrase field.
const TYPED: &str = "frame-row-passphrase-5c21";

/// The prompt's answer, as a row names it; never the passphrase.
fn said(answer: &mut oneshot::Receiver<PromptReply>) -> &'static str {
    match answer.try_recv() {
        Ok(None) => "unanswered",
        Ok(Some(PromptReply::Supplied(_))) => "supplied",
        Ok(Some(PromptReply::Cancelled)) => "cancelled",
        Ok(Some(PromptReply::Unavailable)) => "unavailable",
        Err(_) => "dropped",
    }
}

/// Places a prompt and checks that its dialog opened.
fn open(shell: &Shell, cx: &mut TestAppContext) -> oneshot::Receiver<PromptReply> {
    let answer = shell.prompt(cx);
    assert!(
        shell.present("passphrase-dialog", cx),
        "Unlock key: it opened"
    );
    answer
}

/// P1's Unlock key rows.
pub(super) fn unlock_keys(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _root) = listed(cx);
    let origin = origin(&shell, cx);

    let mut answer = open(&shell, cx);
    table.fact(
        "Unlock key, on open",
        inside(&shell, "passphrase-input", cx),
        "focus in the passphrase field",
    );
    shell.keys("escape", cx);
    table.fact(
        "Unlock key, Escape closes it",
        !shell.present("passphrase-dialog", cx),
        "the dialog closed",
    );
    let escaped = said(&mut answer);
    table.fact(
        "Unlock key, Escape answers Not now",
        escaped == "cancelled",
        &format!("the prompt answered cancelled (it is {escaped})"),
    );
    table.fact(
        "Unlock key, Escape gives focus back",
        back_at(&shell, &origin, cx),
        "focus on the list it was opened from",
    );

    let mut answer = open(&shell, cx);
    shell.visual(cx).simulate_input(TYPED);
    shell.run(cx);
    shell.keys("enter", cx);
    table.fact(
        "Unlock key, Enter closes it",
        !shell.present("passphrase-dialog", cx),
        "the dialog closed",
    );
    let entered = said(&mut answer);
    table.fact(
        "Unlock key, Enter supplies the passphrase",
        entered == "supplied",
        &format!("the prompt answered supplied (it is {entered})"),
    );
    table.fact(
        "Unlock key, Enter gives focus back",
        back_at(&shell, &origin, cx),
        "focus on the list it was opened from",
    );
}

/// P2's Unlock key row: Tab from the passphrase field reaches Not now, then
/// Unlock.
pub(super) fn unlock_tabs(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _root) = listed(cx);
    let _answer = open(&shell, cx);
    let cancel = ElementId::from(("passphrase-cancel", 0usize));
    let ok = ElementId::from(("passphrase-ok", 0usize));
    let order = tab_order(&shell, &[cancel.clone(), ok.clone()], cx);
    table.fact(
        "Unlock key",
        order == [cancel, ok],
        &format!("Tab reaching Not now, then Unlock (it reached {order:?})"),
    );
}
