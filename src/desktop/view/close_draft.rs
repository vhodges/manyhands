//! The dialog that leaving an item tab's unsaved work opens: closing a tab
//! with a restored draft (D2's), closing a tab whose edit session has
//! unsaved work, a kept draft or one being stored (design, Done and
//! leaving: Closing the tab) or, while clean, a draft stored or being
//! stored (under D2's title, in words of its own: Part C fix 2, N3), and
//! Done when the draft cannot be written or the item was never created
//! (step 3 and 5).
//!
//! It offers Keep draft, the default (Enter) and first Tab stop where it
//! is offered, then Save where it can be chosen (R-16), Discard and Cancel
//! (Escape); a Save not available is shown last, unavailable, with its
//! note. What it offers, and an edit session's message with it, is read
//! again as each frame is drawn: Keep draft waits while a draft is being
//! stored, Keep draft and Discard while a save is in flight, and the
//! message names Keep draft only while it is offered. Without Keep draft,
//! Enter is Save where it can be chosen, else Cancel. The messages hold no
//! draft text: D2's names the item by its title, never the draft's (ruling
//! R25); an edit session's names nothing. Focus goes back where it was when
//! the dialog closes.
//!
//! The close dialogs' Save (Task 18; design, Restored drafts and the close
//! dialog) keeps the dialog open while it saves: the tab closes it once
//! the draft retires. A save that answers anything but committed is shown
//! in the dialog, read as each frame is drawn, with Retry for a pending
//! checkpoint and the library's message and code for a refusal; the tab
//! stays. The note holds fixed words and the library's own message.

use std::rc::Rc;

use gpui_kit::{
    App, ClickEvent, ElementId, FocusHandle, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, Disableable, WindowExt,
        button::{Button, ButtonVariants},
        dialog::Confirm,
        h_flex, v_flex,
    },
    div,
};
use manyhands::results::ResultCode;

use super::confirm::CANCEL;
use crate::desktop::model::{edit::save_state::RETRY, item::LATER_NOTE};

pub(crate) const KEEP_DRAFT: &str = "Keep draft";
pub(crate) const DISCARD: &str = "Discard";
pub(crate) const SAVE: &str = "Save";
pub(crate) const TITLE: &str = "Close a tab with a draft";
/// Closing a tab whose edit session has unsaved work.
pub(crate) const SESSION_TITLE: &str = "Close a tab with unsaved changes";
pub(crate) const SESSION_KEEP: &str = "Keep draft keeps your unsaved changes on this computer, \
                                       to continue later. Discard deletes them.";
pub(crate) const SESSION_UNKEPT: &str = "Your unsaved changes are not kept as a draft on this \
                                         computer. Discard deletes them; Cancel goes back to the \
                                         tab.";
/// Closing a tab whose edit session is clean, with a draft stored or being
/// stored (Part C fix 2, N3). It names nothing.
pub(crate) const SESSION_DRAFT: &str =
    "This tab has a draft, and no unsaved changes. Keep it to continue later, or discard it.";
/// Keep draft's words while the draft's last write failed: it writes
/// again, and closes only once the newest text is stored (Part C fix 1,
/// M2).
pub(crate) const SESSION_KEEP_FAILED: &str = "Your newest changes could not be kept as a draft \
                                              on this computer; they are only in the editor. \
                                              Keep draft tries again and closes the tab once \
                                              they are kept. Discard deletes them.";
/// Done, with a draft that cannot be written (store unavailable, or the
/// write failed).
pub(crate) const NOT_KEPT_TITLE: &str = "Changes not kept";
pub(crate) const NOT_KEPT: &str = "Your unsaved changes could not be kept as a draft on this \
                                   computer; they are only in the editor. Save them, or discard \
                                   them.";
/// Done, on an item never created.
pub(crate) const NOT_CREATED_TITLE: &str = "Not created yet";
pub(crate) const NOT_CREATED: &str = "This item is not created yet, so there is no saved copy \
                                      to view. Save creates it; Keep draft keeps it on this \
                                      computer, to continue later.";
/// The close dialog's Save while it runs.
pub(crate) const SAVING: &str = "Saving\u{2026}";

/// What the user chose.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CloseWithDraft {
    /// Write the newest text, then let go of the draft: it stays in Drafts.
    KeepDraft,
    /// Delete this draft.
    Discard,
    /// Leave the tab as it is.
    Cancel,
    /// Save the text, where the dialog offers it.
    Save,
    /// The close dialog's pending checkpoint again, under the same request.
    Retry,
}

/// What the dialog offers now.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Offers {
    /// Keep draft: not offered (`None`), or offered and whether it can be
    /// chosen now; it waits while the draft is being stored.
    pub(crate) keep: Option<bool>,
    /// Save: shown unavailable with its note (`None`), or offered and
    /// whether it can be chosen now.
    pub(crate) save: Option<bool>,
    /// Discard can be chosen now: not while a save is in flight.
    pub(crate) discard: bool,
}

/// What the close dialog's Save came to, while it is not yet closed: fixed
/// words, the library's message and code for a refusal, and whether Retry
/// is offered. It holds no draft text.
pub(crate) struct SaveNote {
    pub(crate) words: String,
    pub(crate) detail: Option<(String, ResultCode)>,
    pub(crate) retry: bool,
}

impl SaveNote {
    /// `words` alone.
    pub(crate) fn says(words: impl Into<String>) -> Self {
        Self {
            words: words.into(),
            detail: None,
            retry: false,
        }
    }
}

/// What the dialog says.
pub(crate) enum Message {
    Fixed(String),
    /// An edit session's close: Keep draft's words while it is offered,
    /// read with the offers as each frame is drawn.
    /// It reads whether the draft's last write failed: Keep draft then
    /// promises nothing the store has not done (Part C fix 1, M2).
    Session(Read<bool>),
}

impl Message {
    fn words(&self, offers: Offers, cx: &App) -> String {
        match self {
            Self::Fixed(words) => words.clone(),
            Self::Session(failed) if offers.keep.is_some() && failed(cx) => {
                SESSION_KEEP_FAILED.to_owned()
            }
            Self::Session(_) if offers.keep.is_some() => SESSION_KEEP.to_owned(),
            Self::Session(_) => SESSION_UNKEPT.to_owned(),
        }
    }
}

/// What the dialog reads of its tab as each frame is drawn.
pub(crate) type Read<T> = Rc<dyn Fn(&App) -> T>;

/// The dialog to open.
pub(crate) struct Ask {
    /// Its element IDs' prefix.
    pub(crate) id: &'static str,
    pub(crate) title: &'static str,
    pub(crate) message: Message,
    /// What it offers, read as each frame is drawn.
    pub(crate) offers: Read<Offers>,
    /// What its Save came to, read as each frame is drawn.
    pub(crate) note: Read<Option<SaveNote>>,
    /// Save and Retry keep the dialog open: the tab closes it.
    pub(crate) stays: bool,
}

impl Ask {
    /// D2's dialog for the tab showing `item`, by its title or ID, with a
    /// restored draft: Keep draft, Save, Discard and Cancel. Save goes
    /// through Continue editing's steps (the tab's).
    pub(crate) fn restored(item: &str, offers: Read<Offers>, note: Read<Option<SaveNote>>) -> Self {
        Self {
            id: "close-draft",
            title: TITLE,
            message: Message::Fixed(message(item)),
            offers,
            note,
            stays: true,
        }
    }

    /// Closing a tab whose edit session has unsaved work: Keep draft where
    /// a draft is stored or being stored, Save while dirty and nothing
    /// saves, Discard and Cancel. `failed` reads whether the draft's last
    /// write failed.
    pub(crate) fn session(
        offers: Read<Offers>,
        note: Read<Option<SaveNote>>,
        failed: Read<bool>,
    ) -> Self {
        Self {
            id: "close-draft",
            title: SESSION_TITLE,
            message: Message::Session(failed),
            offers,
            note,
            stays: true,
        }
    }

    /// Closing a tab whose edit session is clean, with a draft stored or
    /// being stored: D2's title, and words that name nothing (Part C fix 2,
    /// N3). It offers what the session's dialog offers.
    pub(crate) fn session_draft(offers: Read<Offers>, note: Read<Option<SaveNote>>) -> Self {
        Self {
            id: "close-draft",
            title: TITLE,
            message: Message::Fixed(SESSION_DRAFT.to_owned()),
            offers,
            note,
            stays: true,
        }
    }

    /// Done, when the draft cannot be written, or on an item never created.
    /// Its Save is the header's, once the dialog has gone.
    pub(crate) fn done(never_created: bool, offers: Read<Offers>) -> Self {
        let (title, message) = if never_created {
            (NOT_CREATED_TITLE, NOT_CREATED)
        } else {
            (NOT_KEPT_TITLE, NOT_KEPT)
        };
        Self {
            id: "done-draft",
            title,
            message: Message::Fixed(message.to_owned()),
            offers,
            note: Rc::new(|_| None),
            stays: false,
        }
    }

    /// Whether `choice` leaves the dialog open: the close dialogs' Save and
    /// Retry.
    fn stays_for(&self, choice: CloseWithDraft) -> bool {
        self.stays && matches!(choice, CloseWithDraft::Save | CloseWithDraft::Retry)
    }
}

type Choose = Rc<dyn Fn(CloseWithDraft, &mut Window, &mut App)>;

/// D2's message, naming the item by `item`, its title or ID.
pub(crate) fn message(item: &str) -> String {
    format!("{item} has a restored draft. Keep it to restore it later, or discard it.")
}

/// What Enter chooses: Keep draft where offered, waiting while it cannot
/// be chosen (`None`); else Save where it can be chosen; else Cancel.
fn default_choice(offers: Offers) -> Option<CloseWithDraft> {
    match (offers.keep, offers.save) {
        (Some(true), _) => Some(CloseWithDraft::KeepDraft),
        (Some(false), _) => None,
        (None, Some(true)) => Some(CloseWithDraft::Save),
        (None, _) => Some(CloseWithDraft::Cancel),
    }
}

/// Opens `ask`. `choose` gets the choice once the dialog has closed, or at
/// once for a choice that keeps it open; Cancel and Escape give `Cancel`.
/// A choice that keeps it open first gives the focus to the dialog's body,
/// which tracks a focus of its own, as Create's form does: the button
/// pressed waits from then on and holds no focus, and the dialog's keys,
/// Escape among them, act only while focus is in it (Task 27 fix 1, A1).
pub(crate) fn open(
    ask: Ask,
    window: &mut Window,
    cx: &mut App,
    choose: impl Fn(CloseWithDraft, &mut Window, &mut App) + 'static,
) {
    let choose: Choose = Rc::new(choose);
    let origin = window.focused(cx);
    let ask = Rc::new(ask);
    let held = cx.focus_handle();
    window.open_dialog(cx, move |base, _, cx| {
        let (enter, cancel, close) = (choose.clone(), choose.clone(), origin.clone());
        let offers = (ask.offers)(cx);
        let note = (ask.note)(cx);
        let (stays, parked) = (ask.clone(), held.clone());
        base.title(ask.title)
            .close_button(false)
            .overlay_closable(false)
            .child(body(&ask, offers, note, &choose, &held, cx))
            // Enter: the default; the Kit closes the dialog, unless Keep
            // draft still waits, or the default keeps it open.
            .on_ok(move |_, window, cx| match default_choice(offers) {
                Some(choice) => {
                    let stays = stays.stays_for(choice);
                    if stays {
                        window.focus(&parked, cx);
                    }
                    enter(choice, window, cx);
                    !stays
                }
                None => false,
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

fn body(
    ask: &Rc<Ask>,
    offers: Offers,
    note: Option<SaveNote>,
    choose: &Choose,
    held: &FocusHandle,
    cx: &App,
) -> impl IntoElement + use<> {
    let muted = cx.theme().muted_foreground;
    let prefix = ask.id;
    let default = default_choice(offers);
    let message = ask.message.words(offers, cx);
    let button = |part: &str, label: &'static str, choice: CloseWithDraft, enabled: bool| {
        let id = format!("{prefix}-{part}");
        let choose = choose.clone();
        let mut button = Button::new(ElementId::from(format!("{id}-button")))
            .label(label)
            .disabled(!enabled);
        if default == Some(choice) && choice != CloseWithDraft::Cancel {
            button = button.primary();
        }
        let closes = !ask.stays_for(choice);
        let parked = held.clone();
        let press = enabled.then(|| {
            Rc::new(move |window: &mut Window, cx: &mut App| {
                if closes {
                    window.close_dialog(cx);
                } else {
                    window.focus(&parked, cx);
                }
                choose(choice, window, cx);
            })
        });
        if let Some(press) = press.clone() {
            button = button.on_click(move |_: &ClickEvent, window, cx| press(window, cx));
        }
        let wrapper = div()
            .id(ElementId::from(id))
            .test_support()
            .aria_label(label);
        // Enter on a focused button presses that button: its wrapper takes
        // the dialog's Enter (`Confirm`, on key down), whose default it
        // would otherwise give. A button that cannot be chosen now takes no
        // focus, so it has no handler.
        match press {
            Some(press) => wrapper.on_action(move |_: &Confirm, window, cx| press(window, cx)),
            None => wrapper,
        }
        .child(button)
    };
    let keep = offers
        .keep
        .map(|ready| button("keep", KEEP_DRAFT, CloseWithDraft::KeepDraft, ready));
    let save = offers
        .save
        .map(|ready| button("save", SAVE, CloseWithDraft::Save, ready));
    let retry = note
        .as_ref()
        .filter(|note| note.retry)
        .map(|_| button("retry", RETRY, CloseWithDraft::Retry, true));
    let later = offers.save.is_none().then(|| {
        v_flex()
            .child(button("save", SAVE, CloseWithDraft::Save, false))
            .child(
                div()
                    .id(ElementId::from(format!("{prefix}-save-note")))
                    .test_support()
                    .aria_label(LATER_NOTE)
                    .text_xs()
                    .text_color(muted)
                    .child(LATER_NOTE),
            )
    });
    let note = note.map(|note| save_note(prefix, note, cx));
    v_flex()
        .id(ElementId::from(format!("{prefix}-dialog")))
        .test_support()
        .track_focus(held)
        // Named by its title, which the Kit draws above it.
        .aria_label(ask.title)
        .gap_2()
        .child(
            div()
                .id(ElementId::from(format!("{prefix}-message")))
                .test_support()
                .aria_label(message.clone())
                .text_sm()
                .child(message),
        )
        .children(note)
        .child(
            h_flex()
                .gap_2()
                .children(keep)
                .children(save)
                .children(retry)
                .child(button(
                    "discard",
                    DISCARD,
                    CloseWithDraft::Discard,
                    offers.discard,
                ))
                .child(button("cancel", CANCEL, CloseWithDraft::Cancel, true))
                .children(later),
        )
}

/// What the dialog's Save came to: its words, then the library's message
/// and code.
fn save_note(prefix: &'static str, note: SaveNote, cx: &App) -> impl IntoElement + use<> {
    let muted = cx.theme().muted_foreground;
    let line = move |part: &str, words: String| {
        div()
            .id(ElementId::from(format!("{prefix}-{part}")))
            .test_support()
            .aria_label(words.clone())
            .text_sm()
            .child(words)
    };
    let detail = note.detail.map(|(message, code)| {
        v_flex()
            .text_color(muted)
            .child(line("save-message", message))
            .child(line("save-code", code.as_str().to_owned()))
    });
    v_flex()
        .gap_1()
        .child(line("save-state", note.words))
        .children(detail)
}
