//! Quitting the main window.
//!
//! The Quit action and the window's close request go through one
//! handler. First, a draft an item tab is still beginning is waited for,
//! under "Saving drafts for recovery" and Quit now; then, when item tabs
//! hold unsaved edit text that no draft holds (crash recovery off, or a
//! draft that could not begin), a dialog says how many, naming no item and
//! no text, with Quit anyway and Cancel, the default, which stays and shows
//! the first such tab (Part C fix 1, I1; the Cycle's contract 6). Quit
//! anyway goes on as below. Next, if a draft session's newest text is not yet stored,
//! it refuses, flushes every session and shows "Saving drafts for
//! recovery" with Quit now, which quits at once (the last stored revision
//! survives). If every flush succeeds, quitting goes on as below; if any
//! fails, a dialog says how many drafts could not be saved, with Quit
//! anyway and Cancel, the default, which focuses the first failed draft's
//! Retry in the operation area. Flushing happens before GPUI may quit:
//! `on_app_quit` gets at most 200 ms. While drafts are saved, and while
//! that dialog asks, no confirmation opens and a passphrase prompt is held,
//! neither shown nor answered: quitting going on answers it
//! `Unavailable`, as below; Cancel shows it again.
//!
//! Then, with no job running it quits. With one running it refuses,
//! stops the worker admitting jobs, shows "Finishing" and Quit now, and
//! quits when the job returns. Quit now quits at once: the worker thread
//! is not waited for, and the library's journals recover what it was
//! doing. Either way a prompt the running job waits on is answered
//! `Unavailable` first, so the job can return.
//!
//! When quitting starts to wait, every open dialog closes and Quit now
//! takes focus (design, Quitting: "the focused Quit now button"), so
//! nothing stands in front of it; while it waits, no dialog opens.

use std::rc::Rc;

use gpui_kit::{
    App, ClickEvent, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement,
    Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, WindowExt,
        button::{Button, ButtonVariants},
        dialog::Confirm,
        h_flex, v_flex,
    },
    div,
};

use super::{confirm::CANCEL, workspace::Workspace};
use crate::desktop::{keys::QuitNow, services::requests::DesktopRequestId};

pub(crate) const QUIT_NOW: &str = "Quit now";
pub(crate) const SAVING_DRAFTS: &str = "Saving drafts for recovery";
pub(crate) const QUIT_ANYWAY: &str = "Quit anyway";
const NOT_SAVED_TITLE: &str = "Quit";

/// Where quitting stands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum QuitPhase {
    Running,
    /// Asked to quit; flushing every draft session first.
    SavingDrafts,
    /// Asked to quit; waiting for this job to return.
    Finishing(DesktopRequestId),
    /// Quitting.
    Quit,
}

impl Workspace {
    /// What quitting waits for: "Saving drafts for recovery", or
    /// "Finishing" and the command. Fixed text only.
    pub(crate) fn finishing_text(&self, cx: &App) -> Option<String> {
        let id = match self.quit_phase() {
            QuitPhase::SavingDrafts => return Some(SAVING_DRAFTS.to_owned()),
            QuitPhase::Finishing(id) => id,
            QuitPhase::Running | QuitPhase::Quit => return None,
        };
        let operations = self.services().operations().read(cx);
        Some(match operations.request(id) {
            Some(request) => format!("Finishing {}", request.intent.label()),
            None => "Finishing".to_owned(),
        })
    }

    /// What quitting waits for, with Quit now.
    /// Quit now is one control, a tab stop whose wrapper holds the focus
    /// and is pressed by Enter, Space or a click; the Kit button inside
    /// only draws it.
    pub(super) fn finishing(&self, window: &Window, cx: &App) -> Option<impl IntoElement + use<>> {
        let ring = cx.theme().ring;
        let focused = self.quit_focus().is_focused(window);
        self.finishing_text(cx).map(|text| {
            h_flex()
                .gap_2()
                .child(
                    div()
                        .id("quit-finishing")
                        .test_support()
                        .aria_label(text.clone())
                        .child(text),
                )
                .child(
                    div()
                        .id("quit-now")
                        .test_support()
                        .aria_label(QUIT_NOW)
                        .track_focus(self.quit_focus())
                        .relative()
                        .rounded_sm()
                        .on_click(|_, window, cx| window.dispatch_action(Box::new(QuitNow), cx))
                        .child(
                            Button::new("quit-now-button")
                                .danger()
                                .label(QUIT_NOW)
                                .tab_stop(false),
                        )
                        .children(focused.then(|| {
                            div()
                                .id("quit-now-focus-ring")
                                .test_support()
                                .absolute()
                                .inset_0()
                                .rounded_sm()
                                .border_2()
                                .border_color(ring)
                        })),
                )
        })
    }
}

/// What the drafts-not-saved dialog chose.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum NotSavedChoice {
    QuitAnyway,
    /// Stay, with the first failed draft's Retry focused.
    Cancel,
}

type Choose = Rc<dyn Fn(NotSavedChoice, &mut Window, &mut App)>;

/// The dialog a failed flush on quit opens: how many drafts could not be
/// saved, naming no item and no draft, with Quit anyway and Cancel, the
/// default (Enter and Escape).
pub(crate) struct DraftsNotSaved {
    pub count: usize,
}

impl DraftsNotSaved {
    pub(crate) fn message(&self) -> String {
        match self.count {
            1 => "1 draft could not be saved for recovery".to_owned(),
            n => format!("{n} drafts could not be saved for recovery"),
        }
    }

    /// Opens the dialog; `choose` gets the choice once it has closed.
    pub(crate) fn open(
        self,
        window: &mut Window,
        cx: &mut App,
        choose: impl Fn(NotSavedChoice, &mut Window, &mut App) + 'static,
    ) {
        open_asking(DRAFTS_NOT_SAVED, self.message(), window, cx, choose);
    }
}

/// The dialog quitting opens when item tabs hold unsaved edit text that no
/// draft holds (Part C fix 1, I1): crash recovery off, or a draft that
/// could not begin. It says how many, naming no item and no text, with
/// Quit anyway and Cancel, the default (Enter and Escape), which stays and
/// shows the first such tab. D2's own question, when it applies, comes
/// after Quit anyway.
pub(crate) struct EditsNotKept {
    pub count: usize,
}

impl EditsNotKept {
    pub(crate) fn message(&self) -> String {
        unkept_tabs(self.count)
    }

    /// Opens the dialog; `choose` gets the choice once it has closed.
    pub(crate) fn open(
        self,
        window: &mut Window,
        cx: &mut App,
        choose: impl Fn(NotSavedChoice, &mut Window, &mut App) + 'static,
    ) {
        open_asking(EDITS_NOT_KEPT, self.message(), window, cx, choose);
    }
}

/// How many tabs hold unsaved changes that no draft keeps, in words that
/// name no item and no text.
pub(crate) fn unkept_tabs(count: usize) -> String {
    match count {
        1 => "1 tab has unsaved changes that are not kept as a draft".to_owned(),
        n => format!("{n} tabs have unsaved changes that are not kept as drafts"),
    }
}

/// A quit question's element IDs.
struct AskIds {
    dialog: &'static str,
    message: &'static str,
    cancel: &'static str,
    quit: &'static str,
}

const DRAFTS_NOT_SAVED: AskIds = AskIds {
    dialog: "drafts-not-saved-dialog",
    message: "drafts-not-saved-message",
    cancel: "drafts-not-saved-cancel",
    quit: "drafts-not-saved-quit",
};

const EDITS_NOT_KEPT: AskIds = AskIds {
    dialog: "edits-not-kept-dialog",
    message: "edits-not-kept-message",
    cancel: "edits-not-kept-cancel",
    quit: "edits-not-kept-quit",
};

/// Opens a quit question: `message`, with Cancel, the default, and Quit
/// anyway; `choose` gets the choice once it has closed.
fn open_asking(
    ids: AskIds,
    message: String,
    window: &mut Window,
    cx: &mut App,
    choose: impl Fn(NotSavedChoice, &mut Window, &mut App) + 'static,
) {
    let choose: Choose = Rc::new(choose);
    window.open_dialog(cx, move |base, _, _| {
        let (ok, cancel) = (choose.clone(), choose.clone());
        base.title(NOT_SAVED_TITLE)
            .close_button(false)
            .overlay_closable(false)
            .child(not_saved_body(&ids, &message, &choose))
            // Enter: Cancel, the default; the Kit closes the dialog.
            .on_ok(move |_, window, cx| {
                ok(NotSavedChoice::Cancel, window, cx);
                true
            })
            .on_cancel(move |_, window, cx| {
                cancel(NotSavedChoice::Cancel, window, cx);
                true
            })
    });
}

fn not_saved_body(ids: &AskIds, message: &str, choose: &Choose) -> impl IntoElement + use<> {
    let button = |id: &'static str, label: &'static str, choice: NotSavedChoice| {
        let choose = choose.clone();
        let button = Button::new(format!("{id}-button")).label(label);
        let button = match choice {
            NotSavedChoice::Cancel => button.primary(),
            NotSavedChoice::QuitAnyway => button.danger(),
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
        .id(ids.dialog)
        .test_support()
        .gap_2()
        .child(
            div()
                .id(ids.message)
                .test_support()
                .aria_label(message.to_owned())
                .text_sm()
                .child(message.to_owned()),
        )
        .child(
            h_flex()
                .gap_2()
                // The safe choice is the first Tab stop: one stray Tab and
                // Enter does not quit.
                .child(button(ids.cancel, CANCEL, NotSavedChoice::Cancel))
                .child(button(ids.quit, QUIT_ANYWAY, NotSavedChoice::QuitAnyway)),
        )
}
