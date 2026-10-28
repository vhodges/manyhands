//! The dialog that leaving an item tab's unsaved work opens: closing a tab
//! with a restored draft (D2's), closing a tab whose edit session has
//! unsaved work, a kept draft or one being stored (design, Done and
//! leaving: Closing the tab), and Done when the draft cannot be written or
//! the item was never created (step 3 and 5).
//!
//! It offers Keep draft, the default (Enter) and first Tab stop where it
//! is offered, then Save where it can be chosen (R-16), Discard and Cancel
//! (Escape); a Save not available yet is shown last, unavailable, with its
//! note. What it offers is read again as each frame is drawn, so Keep
//! draft waits while a draft is being stored, and Discard while a save is
//! in flight. Without Keep draft, Enter is Save where it can be chosen,
//! else Cancel. The messages hold no draft text: D2's names the item by
//! its title, never the draft's (ruling R25); an edit session's names
//! nothing. Focus goes back where it was when the dialog closes.

use std::rc::Rc;

use gpui_kit::{
    App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, Disableable, WindowExt,
        button::{Button, ButtonVariants},
        dialog::Confirm,
        h_flex, v_flex,
    },
    div,
};

use super::{confirm::CANCEL, item::EDITOR_LATER};
use crate::desktop::model::item::LATER_NOTE;

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

/// The dialog to open.
pub(crate) struct Ask {
    /// Its element IDs' prefix.
    pub(crate) id: &'static str,
    pub(crate) title: &'static str,
    pub(crate) message: String,
    /// The note of a Save not available yet.
    pub(crate) later: &'static str,
    /// What it offers, read as each frame is drawn.
    pub(crate) offers: Rc<dyn Fn(&App) -> Offers>,
}

impl Ask {
    /// D2's dialog for the tab showing `item`, by its title or ID, with a
    /// restored draft: Keep draft, Discard and Cancel, and Save shown
    /// unavailable.
    pub(crate) fn restored(item: &str) -> Self {
        Self {
            id: "close-draft",
            title: TITLE,
            message: message(item),
            later: EDITOR_LATER,
            offers: Rc::new(|_| Offers {
                keep: Some(true),
                save: None,
                discard: true,
            }),
        }
    }

    /// Closing a tab whose edit session has unsaved work: Keep draft where
    /// a draft is stored or being stored, Discard and Cancel, and Save shown
    /// unavailable until the close dialog saves (Task 18).
    pub(crate) fn session(keeps: bool, offers: Rc<dyn Fn(&App) -> Offers>) -> Self {
        Self {
            id: "close-draft",
            title: SESSION_TITLE,
            message: if keeps { SESSION_KEEP } else { SESSION_UNKEPT }.to_owned(),
            later: LATER_NOTE,
            offers,
        }
    }

    /// Done, when the draft cannot be written, or on an item never created.
    pub(crate) fn done(never_created: bool, offers: Rc<dyn Fn(&App) -> Offers>) -> Self {
        let (title, message) = if never_created {
            (NOT_CREATED_TITLE, NOT_CREATED)
        } else {
            (NOT_KEPT_TITLE, NOT_KEPT)
        };
        Self {
            id: "done-draft",
            title,
            message: message.to_owned(),
            later: LATER_NOTE,
            offers,
        }
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

/// Opens `ask`. `choose` gets the choice once the dialog has closed;
/// Cancel and Escape give `Cancel`.
pub(crate) fn open(
    ask: Ask,
    window: &mut Window,
    cx: &mut App,
    choose: impl Fn(CloseWithDraft, &mut Window, &mut App) + 'static,
) {
    let choose: Choose = Rc::new(choose);
    let origin = window.focused(cx);
    let ask = Rc::new(ask);
    window.open_dialog(cx, move |base, _, cx| {
        let (enter, cancel, close) = (choose.clone(), choose.clone(), origin.clone());
        let offers = (ask.offers)(cx);
        base.title(ask.title)
            .close_button(false)
            .overlay_closable(false)
            .child(body(&ask, offers, &choose, cx))
            // Enter: the default; the Kit closes the dialog, unless Keep
            // draft still waits.
            .on_ok(move |_, window, cx| match default_choice(offers) {
                Some(choice) => {
                    enter(choice, window, cx);
                    true
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

fn body(ask: &Ask, offers: Offers, choose: &Choose, cx: &App) -> impl IntoElement + use<> {
    let muted = cx.theme().muted_foreground;
    let prefix = ask.id;
    let default = default_choice(offers);
    let button = |part: &str, label: &'static str, choice: CloseWithDraft, enabled: bool| {
        let id = format!("{prefix}-{part}");
        let choose = choose.clone();
        let mut button = Button::new(ElementId::from(format!("{id}-button")))
            .label(label)
            .disabled(!enabled);
        if default == Some(choice) && choice != CloseWithDraft::Cancel {
            button = button.primary();
        }
        let press = enabled.then(|| {
            Rc::new(move |window: &mut Window, cx: &mut App| {
                window.close_dialog(cx);
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
    let later = offers.save.is_none().then(|| {
        v_flex()
            .child(button("save", SAVE, CloseWithDraft::Save, false))
            .child(
                div()
                    .id(ElementId::from(format!("{prefix}-save-note")))
                    .test_support()
                    .aria_label(ask.later)
                    .text_xs()
                    .text_color(muted)
                    .child(ask.later),
            )
    });
    v_flex()
        .id(ElementId::from(format!("{prefix}-dialog")))
        .test_support()
        .gap_2()
        .child(
            div()
                .id(ElementId::from(format!("{prefix}-message")))
                .test_support()
                .aria_label(ask.message.clone())
                .text_sm()
                .child(ask.message.clone()),
        )
        .child(
            h_flex()
                .gap_2()
                .children(keep)
                .children(save)
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
