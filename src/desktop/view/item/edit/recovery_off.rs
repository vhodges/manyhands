//! The dialog Edit opens first when the draft store did not open (Phases,
//! step 4): crash recovery is off, so nothing typed is kept for recovery
//! until it is saved, and the user chooses whether to go on. Continue, the
//! default (Enter), goes on to prepare the context; Cancel (Escape) makes
//! nothing. It follows the close dialog's pattern (`view::close_draft`),
//! names the store's fixed reason and holds no item text. Focus goes back
//! where it was when the dialog closes.

use std::rc::Rc;

use gpui_kit::{
    App, ClickEvent, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement,
    Styled, TestSupportExt, Window,
    component::{
        WindowExt,
        button::{Button, ButtonVariants},
        dialog::Confirm,
        h_flex, v_flex,
    },
    div,
};

use crate::desktop::{
    model::{edit::RECOVERY_OFF, operations::drafts_unavailable},
    services::DraftStoreUnavailable,
    view::confirm::CANCEL,
};

pub(crate) const CONTINUE: &str = "Continue";

/// The message: the store's reason, in D2's words, then what follows.
pub(crate) fn message(reason: DraftStoreUnavailable) -> String {
    format!(
        "{} Save to protect your changes.",
        drafts_unavailable(reason)
    )
}

type Go = Rc<dyn Fn(&mut App)>;

/// Opens the dialog; `go` runs once it has closed with Continue.
pub(super) fn open(
    reason: DraftStoreUnavailable,
    window: &mut Window,
    cx: &mut App,
    go: impl Fn(&mut App) + 'static,
) {
    let go: Go = Rc::new(go);
    let origin = window.focused(cx);
    window.open_dialog(cx, move |base, _, _| {
        let (ok, close) = (go.clone(), origin.clone());
        base.title(RECOVERY_OFF)
            .close_button(false)
            .overlay_closable(false)
            .child(body(reason, &go))
            // Enter: Continue; the Kit closes the dialog.
            .on_ok(move |_, _, cx| {
                ok(cx);
                true
            })
            .on_cancel(|_, _, _| true)
            .on_close(move |_, window, cx| {
                if let Some(origin) = &close {
                    window.focus(origin, cx);
                }
            })
    });
}

fn body(reason: DraftStoreUnavailable, go: &Go) -> impl IntoElement + use<> {
    let button = |id: &'static str, label: &'static str, proceed: bool| {
        let go = go.clone();
        let press = Rc::new(move |window: &mut Window, cx: &mut App| {
            window.close_dialog(cx);
            if proceed {
                go(cx);
            }
        });
        let mut button = Button::new(format!("{id}-button")).label(label);
        if proceed {
            button = button.primary();
        }
        let click = press.clone();
        // Enter on a focused button presses that button: its wrapper takes
        // the dialog's Enter (`Confirm`), whose default is Continue.
        div()
            .id(id)
            .test_support()
            .aria_label(label)
            .on_action(move |_: &Confirm, window, cx| press(window, cx))
            .child(button.on_click(move |_: &ClickEvent, window, cx| click(window, cx)))
    };
    let message = message(reason);
    v_flex()
        .id("recovery-off-dialog")
        .test_support()
        .gap_2()
        .child(
            div()
                .id("recovery-off-message")
                .test_support()
                .aria_label(message.clone())
                .text_sm()
                .child(message),
        )
        .child(
            h_flex()
                .gap_2()
                .child(button("recovery-off-continue", CONTINUE, true))
                .child(button("recovery-off-cancel", CANCEL, false)),
        )
}
