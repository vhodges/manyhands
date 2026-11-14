//! The two questions Retire… can ask (D6 design E4, decisions 15 and 18),
//! beside the quit asks. Each names no item and no text: a count and fixed
//! words.
//!
//! - **Unkept text** (step 2): a tab of the item holds text that no draft
//!   keeps. Nothing is submitted. The dialog offers Show tab, the default
//!   (Enter), which shows the tab, and Cancel (Escape). There is no "Retire
//!   anyway": it would drop text the contract keeps.
//! - **Kept drafts** (step 3): the item has kept drafts, which stay on the
//!   Drafts page after it is retired but cannot all be restored. The dialog
//!   says so, and offers Retire and Cancel, the default (Enter and Escape).
//!
//! Both are reached by Tab, Enter and Escape alone.

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

use super::{confirm::CANCEL, quit::unkept_tabs};
use crate::desktop::model::polling::TargetKind;

pub(crate) const SHOW_TAB: &str = "Show tab";
pub(crate) const RETIRE_CHOICE: &str = "Retire";
const TITLE: &str = "Retire local copy";

/// What a Retire… question chose.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RetireChoice {
    /// Show the tab that holds the text; nothing is retired.
    ShowTab,
    /// Go on and retire.
    Retire,
    /// Stay as it was.
    Cancel,
}

/// The words of the unkept-text question: a count, no title and no text.
pub(crate) fn unkept_message(count: usize) -> String {
    format!(
        "{}, so this item is not retired. Keep or discard them first.",
        unkept_tabs(count)
    )
}

/// The words of the kept-drafts question for an item of `kind` with `kept`
/// drafts (item and comments), or an unknown number (`None`).
pub(crate) fn kept_message(kind: TargetKind, kept: Option<usize>) -> String {
    match kind {
        TargetKind::Ticket => {
            let held = match kept {
                Some(1) => "This ticket has 1 kept draft.".to_owned(),
                Some(n) => format!("This ticket has {n} kept drafts."),
                None => "This ticket may have kept drafts.".to_owned(),
            };
            format!(
                "{held} After it is retired you can still export them, but not restore them \
                 into it."
            )
        }
        TargetKind::Document => "You can still restore or export the document's drafts. Drafts \
                                 of comments can only be exported."
            .to_owned(),
    }
}

type Choose = Rc<dyn Fn(RetireChoice, &mut Window, &mut App)>;

/// One question's element IDs.
struct AskIds {
    dialog: &'static str,
    message: &'static str,
    /// The button ids, in Tab order: the default first.
    buttons: [&'static str; 2],
}

const UNKEPT: AskIds = AskIds {
    dialog: "retire-unkept-dialog",
    message: "retire-unkept-message",
    buttons: ["retire-unkept-show", "retire-unkept-cancel"],
};

const KEPT: AskIds = AskIds {
    dialog: "retire-kept-dialog",
    message: "retire-kept-message",
    buttons: ["retire-kept-cancel", "retire-kept-retire"],
};

/// Opens the unkept-text question over `count` tabs. `choose` gets the
/// choice once the dialog has closed. Show tab is the default.
pub(crate) fn ask_unkept(
    count: usize,
    window: &mut Window,
    cx: &mut App,
    choose: impl Fn(RetireChoice, &mut Window, &mut App) + 'static,
) {
    open(
        UNKEPT,
        unkept_message(count),
        [
            (RetireChoice::ShowTab, SHOW_TAB),
            (RetireChoice::Cancel, CANCEL),
        ],
        RetireChoice::ShowTab,
        window,
        cx,
        choose,
    );
}

/// Opens the kept-drafts question. Cancel is the default.
pub(crate) fn ask_kept(
    kind: TargetKind,
    kept: Option<usize>,
    window: &mut Window,
    cx: &mut App,
    choose: impl Fn(RetireChoice, &mut Window, &mut App) + 'static,
) {
    open(
        KEPT,
        kept_message(kind, kept),
        [
            (RetireChoice::Cancel, CANCEL),
            (RetireChoice::Retire, RETIRE_CHOICE),
        ],
        RetireChoice::Cancel,
        window,
        cx,
        choose,
    );
}

fn open(
    ids: AskIds,
    message: String,
    choices: [(RetireChoice, &'static str); 2],
    default: RetireChoice,
    window: &mut Window,
    cx: &mut App,
    choose: impl Fn(RetireChoice, &mut Window, &mut App) + 'static,
) {
    let choose: Choose = Rc::new(choose);
    window.open_dialog(cx, move |base, _, _| {
        let (ok, cancel) = (choose.clone(), choose.clone());
        base.title(TITLE)
            .close_button(false)
            .overlay_closable(false)
            .child(body(&ids, &message, choices, default, &choose))
            // Enter: the default; the Kit closes the dialog.
            .on_ok(move |_, window, cx| {
                ok(default, window, cx);
                true
            })
            // Escape stays as it was.
            .on_cancel(move |_, window, cx| {
                cancel(RetireChoice::Cancel, window, cx);
                true
            })
    });
}

fn body(
    ids: &AskIds,
    message: &str,
    choices: [(RetireChoice, &'static str); 2],
    default: RetireChoice,
    choose: &Choose,
) -> impl IntoElement + use<> {
    let button = |id: &'static str, choice: RetireChoice, label: &'static str| {
        let choose = choose.clone();
        let button = Button::new(format!("{id}-button")).label(label);
        let button = if choice == default {
            button.primary()
        } else {
            button
        };
        let press = Rc::new(move |window: &mut Window, cx: &mut App| {
            window.close_dialog(cx);
            choose(choice, window, cx);
        });
        let enter = press.clone();
        // Enter on a focused button presses that button, not the default.
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
                .child(button(ids.buttons[0], choices[0].0, choices[0].1))
                .child(button(ids.buttons[1], choices[1].0, choices[1].1)),
        )
}
