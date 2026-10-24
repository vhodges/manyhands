//! The dialog that closing a tab with a restored draft opens: Keep draft,
//! the default (Enter), Discard, scoped to that draft, and Cancel
//! (Escape), with Save shown unavailable until the editor arrives. The
//! dialog names the item by its title, never the draft's (ruling R25), and
//! holds no draft text. Focus goes back where it was when the dialog
//! closes.

use std::rc::Rc;

use gpui_kit::{
    App, ClickEvent, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement,
    Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, Disableable, WindowExt,
        button::{Button, ButtonVariants},
        dialog::Confirm,
        h_flex, v_flex,
    },
    div,
};

use super::{confirm::CANCEL, item::EDITOR_LATER};

pub(crate) const KEEP_DRAFT: &str = "Keep draft";
pub(crate) const DISCARD: &str = "Discard";
pub(crate) const SAVE: &str = "Save";
pub(crate) const TITLE: &str = "Close a tab with a draft";

/// What the user chose.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CloseWithDraft {
    /// Write the newest text, then let go of the draft: it stays in Drafts.
    KeepDraft,
    /// Delete this draft.
    Discard,
    /// Leave the tab open.
    Cancel,
}

type Choose = Rc<dyn Fn(CloseWithDraft, &mut Window, &mut App)>;

/// The message, naming the item by `item`, its title or ID.
pub(crate) fn message(item: &str) -> String {
    format!("{item} has a restored draft. Keep it to restore it later, or discard it.")
}

/// Opens the dialog for the tab showing `item`. `choose` gets the choice
/// once the dialog has closed; Cancel and Escape give `Cancel`.
pub(crate) fn open(
    item: String,
    window: &mut Window,
    cx: &mut App,
    choose: impl Fn(CloseWithDraft, &mut Window, &mut App) + 'static,
) {
    let choose: Choose = Rc::new(choose);
    let origin = window.focused(cx);
    window.open_dialog(cx, move |base, _, cx| {
        let (keep, cancel, close) = (choose.clone(), choose.clone(), origin.clone());
        base.title(TITLE)
            .close_button(false)
            .overlay_closable(false)
            .child(body(&item, &choose, cx))
            // Enter: Keep draft; the Kit closes the dialog.
            .on_ok(move |_, window, cx| {
                keep(CloseWithDraft::KeepDraft, window, cx);
                true
            })
            .on_cancel(move |_, window, cx| {
                cancel(CloseWithDraft::Cancel, window, cx);
                true
            })
            .on_close(move |_, window, cx| {
                if let Some(origin) = &close {
                    window.focus(origin, cx);
                }
            })
    });
}

fn body(item: &str, choose: &Choose, cx: &App) -> impl IntoElement + use<> {
    let muted = cx.theme().muted_foreground;
    let button = |id: &'static str, label: &'static str, choice: Option<CloseWithDraft>| {
        let choose = choose.clone();
        let mut button = Button::new(format!("{id}-button"))
            .label(label)
            .disabled(choice.is_none());
        if choice == Some(CloseWithDraft::KeepDraft) {
            button = button.primary();
        }
        let press = choice.map(|choice| {
            Rc::new(move |window: &mut Window, cx: &mut App| {
                window.close_dialog(cx);
                choose(choice, window, cx);
            })
        });
        if let Some(press) = press.clone() {
            button = button.on_click(move |_: &ClickEvent, window, cx| press(window, cx));
        }
        // Enter on a focused button presses that button: its wrapper takes
        // the dialog's Enter (`Confirm`, on key down), whose default is
        // Keep draft. Save, unavailable, takes it and does nothing.
        div()
            .id(id)
            .test_support()
            .aria_label(label)
            .on_action(move |_: &Confirm, window, cx| {
                if let Some(press) = &press {
                    press(window, cx)
                }
            })
            .child(button)
    };
    v_flex()
        .id("close-draft-dialog")
        .test_support()
        .gap_2()
        .child(
            div()
                .id("close-draft-message")
                .test_support()
                .aria_label(message(item))
                .text_sm()
                .child(message(item)),
        )
        .child(
            h_flex()
                .gap_2()
                .child(button(
                    "close-draft-keep",
                    KEEP_DRAFT,
                    Some(CloseWithDraft::KeepDraft),
                ))
                .child(button(
                    "close-draft-discard",
                    DISCARD,
                    Some(CloseWithDraft::Discard),
                ))
                .child(button(
                    "close-draft-cancel",
                    CANCEL,
                    Some(CloseWithDraft::Cancel),
                ))
                .child(
                    v_flex()
                        .child(button("close-draft-save", SAVE, None))
                        .child(
                            div()
                                .id("close-draft-save-note")
                                .test_support()
                                .aria_label(EDITOR_LATER)
                                .text_xs()
                                .text_color(muted)
                                .child(EDITOR_LATER),
                        ),
                ),
        )
}
