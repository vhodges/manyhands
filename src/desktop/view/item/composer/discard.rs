//! The composer's discard prompt (design, Lane C; `04-comments.md`): Cancel
//! with text asks "Discard this reply?" in a reply composer and "Discard
//! this comment?" in the new-comment composer, with Discard and Keep
//! editing, in that order. Keep editing has focus as it opens, so Enter
//! keeps the text, as Escape does, and focus goes back to the editor.
//! Discard emits `Discard`. A failed composer's prompt also says the owner's note (P7, P18). The
//! dialog closes at once, as D3's do, and holds none of the user's text.

use std::rc::Rc;

use gpui_kit::{
    App, ClickEvent, Context, FocusHandle, Focusable, Hsla, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, TestSupportExt, WeakEntity, Window,
    component::{
        ActiveTheme, WindowExt,
        button::{Button, ButtonVariants},
        dialog::Confirm as DialogConfirm,
        h_flex, v_flex,
    },
    div,
};

use super::{Composer, ComposerEvent, ComposerKind, ComposerState};

const DISCARD_REPLY: &str = "Discard this reply?";
const DISCARD_COMMENT: &str = "Discard this comment?";
const DISCARD: &str = "Discard";
const KEEP_EDITING: &str = "Keep editing";

type Press = Rc<dyn Fn(&mut Window, &mut App)>;

impl Composer {
    /// Opens the discard prompt.
    pub(super) fn ask_discard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let title = match self.kind {
            ComposerKind::Reply { .. } => DISCARD_REPLY,
            ComposerKind::New => DISCARD_COMMENT,
        };
        let note = match &self.state {
            ComposerState::Failed { discard_note, .. } => discard_note.clone(),
            _ => None,
        };
        let editor = self.editor.read(cx).focus_handle(cx);
        let me: WeakEntity<Self> = cx.entity().downgrade();
        // Keep editing's own stop, focused as the prompt opens.
        let first = cx.focus_handle().tab_stop(true);
        let keep: Press = Rc::new(move |window, cx| {
            window.close_dialog(cx);
            window.focus(&editor, cx);
        });
        let discard: Press = Rc::new(move |window, cx| {
            window.close_dialog(cx);
            // The owner may drop the composer; nothing of it is used after.
            let _ = me.update(cx, |_, cx| cx.emit(ComposerEvent::Discard));
        });
        let focused = first.clone();
        window.open_dialog(cx, move |base, window, cx| {
            let (ok, cancel) = (keep.clone(), keep.clone());
            let buttons = Buttons {
                keep: keep.clone(),
                discard: discard.clone(),
                first: first.clone(),
                ring: first.is_focused(window).then(|| cx.theme().ring),
            };
            base.title(title)
                .close_button(false)
                .overlay_closable(false)
                .child(body(title, note.clone(), buttons))
                // Enter and Escape keep editing, closing at once.
                .on_ok(move |_, window, cx| {
                    ok(window, cx);
                    false
                })
                .on_cancel(move |_, window, cx| {
                    cancel(window, cx);
                    false
                })
        });
        window.focus(&focused, cx);
    }
}

/// The prompt's buttons, and Keep editing's stop.
struct Buttons {
    keep: Press,
    discard: Press,
    first: FocusHandle,
    /// The ring around Keep editing while its stop has focus.
    ring: Option<Hsla>,
}

/// A prompt button. Enter on it presses it: its wrapper takes the dialog's
/// Enter (`Confirm`) before the dialog, whose default it would otherwise
/// give.
/// `stop` says whether the Kit's button is a Tab stop of its own.
fn button(
    id: &'static str,
    label: &'static str,
    press: &Press,
    stop: bool,
) -> impl IntoElement + use<> {
    let (enter, click) = (press.clone(), press.clone());
    let mut button = Button::new(format!("{id}-button"))
        .label(label)
        .tab_stop(stop);
    if id == "composer-discard-confirm" {
        button = button.danger();
    }
    div()
        .id(id)
        .test_support()
        .aria_label(label)
        .on_action(move |_: &DialogConfirm, window, cx| enter(window, cx))
        .child(button.on_click(move |_: &ClickEvent, window, cx| click(window, cx)))
}

/// The prompt's body, named by its question.
fn body(title: &'static str, note: Option<String>, buttons: Buttons) -> impl IntoElement + use<> {
    // Keep editing is a stop of the prompt's own, so that it can be
    // focused as the prompt opens; its button is no stop, and the ring is
    // drawn around it.
    let keep = div()
        .id("composer-discard-keep-stop")
        .test_support()
        .rounded_md()
        .border_2()
        .border_color(buttons.ring.unwrap_or_else(Hsla::transparent_black))
        .track_focus(&buttons.first)
        .child(button(
            "composer-discard-keep",
            KEEP_EDITING,
            &buttons.keep,
            false,
        ));
    v_flex()
        .id("composer-discard-dialog")
        .test_support()
        .aria_label(title)
        .gap_2()
        .children(note.map(|note| {
            div()
                .id("composer-discard-note")
                .test_support()
                .aria_label(note.clone())
                .text_sm()
                .child(note)
        }))
        .child(
            // The design's order, "Discard · Keep editing"; focus starts on
            // Keep editing, so a stray Enter keeps the text.
            h_flex()
                .gap_2()
                .child(button(
                    "composer-discard-confirm",
                    DISCARD,
                    &buttons.discard,
                    true,
                ))
                .child(keep),
        )
}
