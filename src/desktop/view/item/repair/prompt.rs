//! The question edited repair text asks before it is lost (pre-flight
//! ruling, Task 25): "Discard your repair?", with Cancel, the default
//! (Enter and Escape) and first Tab stop, and Discard. It says the changes
//! are saved nowhere, or, once the repair was sent and may already be made
//! (it runs, or its answer's effect is unknown), that closing the tab does
//! not undo it (Task 27 fix 1, A3). It names no file and holds no text.
//! Focus goes back where it was when it closes.

use std::rc::Rc;

use gpui_kit::{
    App, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        WindowExt,
        button::{Button, ButtonVariants},
        dialog::Confirm,
        h_flex, v_flex,
    },
    div,
};

use super::Repairing;
use crate::desktop::view::{
    close_draft::DISCARD,
    confirm::CANCEL,
    item::{CloseThisTab, ItemPane},
};

pub(crate) const DISCARD_TITLE: &str = "Discard your repair?";
pub(crate) const DISCARD_BODY: &str = "Your changes to this file are not saved anywhere.";
/// The question's words once the repair was sent: while it runs, or after
/// an answer whose effect is unknown.
pub(crate) const SENT_BODY: &str =
    "Your repair was sent and may already be made. Closing the tab does not undo it.";

/// What the question chose.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Choice {
    Discard,
    Cancel,
}

type Choose = Rc<dyn Fn(Choice, &mut Window, &mut App)>;

impl ItemPane {
    /// Asks before edited repair text is lost, which no draft keeps:
    /// Discard drops it, and closes the tab when `closing`; Cancel, the
    /// default, keeps both. Once the repair was sent, the question says so;
    /// a repair Discard leaves running goes on, and its answer, for a view
    /// or a tab gone, opens nothing.
    pub(in crate::desktop::view) fn ask_discard_repair(
        &mut self,
        closing: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let sent = self.repair.as_ref().is_some_and(Repairing::locked);
        let pane = cx.entity().downgrade();
        ask(sent, window, cx, move |choice, window, cx| {
            if choice != Choice::Discard {
                return;
            }
            let _ = pane.update(cx, |pane, cx| {
                if closing {
                    pane.repair = None;
                    cx.emit(CloseThisTab);
                } else {
                    pane.close_repair(window, cx);
                }
            });
        });
    }
}

/// Opens the question, saying the repair was `sent` when it was; `choose`
/// gets the choice once it has closed.
fn ask(
    sent: bool,
    window: &mut Window,
    cx: &mut App,
    choose: impl Fn(Choice, &mut Window, &mut App) + 'static,
) {
    let choose: Choose = Rc::new(choose);
    let origin = window.focused(cx);
    let words = if sent { SENT_BODY } else { DISCARD_BODY };
    window.open_dialog(cx, move |base, _, _| {
        let (ok, cancel, close) = (choose.clone(), choose.clone(), origin.clone());
        base.title(DISCARD_TITLE)
            .close_button(false)
            .overlay_closable(false)
            .child(body(&choose, words))
            // Enter: Cancel, the default; the Kit closes the dialog.
            .on_ok(move |_, window, cx| {
                ok(Choice::Cancel, window, cx);
                true
            })
            .on_cancel(move |_, window, cx| {
                cancel(Choice::Cancel, window, cx);
                true
            })
            .on_close(move |_, window, cx| {
                if let Some(origin) = &close {
                    window.focus(origin, cx);
                }
            })
    });
}

fn body(choose: &Choose, words: &'static str) -> impl IntoElement + use<> {
    let button = |part: &str, label: &'static str, choice: Choice| {
        let id = format!("repair-discard-{part}");
        let choose = choose.clone();
        let button = Button::new(format!("{id}-button")).label(label);
        let button = match choice {
            Choice::Cancel => button.primary(),
            Choice::Discard => button.danger(),
        };
        let press = Rc::new(move |window: &mut Window, cx: &mut App| {
            window.close_dialog(cx);
            choose(choice, window, cx);
        });
        let enter = press.clone();
        // Enter on a focused button presses that button: its wrapper takes
        // the dialog's Enter (`Confirm`, on key down), whose default is
        // Cancel.
        div()
            .id(id)
            .test_support()
            .aria_label(label)
            .on_action(move |_: &Confirm, window, cx| enter(window, cx))
            .child(button.on_click(move |_: &ClickEvent, window, cx| press(window, cx)))
    };
    v_flex()
        .id("repair-discard-dialog")
        .test_support()
        // Named by its title, which the Kit draws above it.
        .aria_label(DISCARD_TITLE)
        .gap_2()
        .child(
            div()
                .id("repair-discard-message")
                .test_support()
                .aria_label(words)
                .text_sm()
                .child(words),
        )
        .child(
            h_flex()
                .gap_2()
                // The safe choice is the first Tab stop.
                .child(button("cancel", CANCEL, Choice::Cancel))
                .child(button("discard", DISCARD, Choice::Discard)),
        )
}
