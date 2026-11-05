//! The sheet's small pieces (handoff 2g): labelled text, a part's state,
//! and a side's mono lines with their tints and gutters.

use std::cell::Cell;

use gpui_kit::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement,
    Styled, TestSupportExt,
    component::{ActiveTheme, v_flex},
    div,
};

use super::super::{
    CHOOSE_ONE, KEEPING_BOTH, KEEPING_EDITED, KEEPING_MINE, KEEPING_THEIRS, TAKEN_MINE,
    TAKEN_THEIRS,
};
use crate::desktop::model::conflict::{BodyChoice, Choice, Mark, marked};

/// A side with no value.
const NONE: &str = "(none)";

/// A piece of text a test reads by `id`.
pub(super) fn text(
    id: impl Into<ElementId>,
    value: impl Into<String>,
) -> gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>> {
    let value = value.into();
    div()
        .id(id)
        .test_support()
        .aria_label(value.clone())
        .child(value)
}

pub(super) fn taken_words(from: Choice) -> &'static str {
    match from {
        Choice::Mine => TAKEN_MINE,
        Choice::Theirs => TAKEN_THEIRS,
    }
}

pub(super) fn field_state(choice: Option<Choice>) -> &'static str {
    match choice {
        None => CHOOSE_ONE,
        Some(Choice::Mine) => KEEPING_MINE,
        Some(Choice::Theirs) => KEEPING_THEIRS,
    }
}

pub(super) fn body_state(choice: Option<BodyChoice>) -> &'static str {
    match choice {
        None => CHOOSE_ONE,
        Some(BodyChoice::Mine) => KEEPING_MINE,
        Some(BodyChoice::Theirs) => KEEPING_THEIRS,
        Some(BodyChoice::Both) => KEEPING_BOTH,
        Some(BodyChoice::Edited) => KEEPING_EDITED,
    }
}

/// A part's state: amber while it waits for a choice, green once chosen.
pub(super) fn state(id: (&'static str, usize), words: &'static str, cx: &App) -> impl IntoElement {
    let color = if words == CHOOSE_ONE {
        cx.theme().warning
    } else {
        cx.theme().success
    };
    text(id, words).text_xs().text_color(color)
}

/// One side's text in mono lines (handoff 2g). Before's lines are plain.
/// Against `before`, each of Yours' or Theirs' lines is unchanged, or
/// tinted amber with `~` where Before's differs, or green with `+` where
/// Before has none; each such line is a `conflict-line` row, numbered
/// across the sheet. No side is "(none)".
pub(super) fn column(
    label: &'static str,
    value: &Option<String>,
    before: Option<&Option<String>>,
    line: &Cell<usize>,
    cx: &App,
) -> impl IntoElement {
    let mut lines = v_flex().min_w_0().font_family("monospace").text_sm();
    match (value, before) {
        (None, _) => lines = lines.child(div().child(NONE)),
        (Some(value), None) => {
            lines = lines.children(value.lines().map(|text| div().child(text.to_owned())));
        }
        (Some(value), Some(before)) => {
            let before = before.as_deref().unwrap_or("");
            for (mark, text) in marked(before, value) {
                let at = line.get();
                line.set(at + 1);
                let row = div()
                    .id(("conflict-line", at))
                    .test_support()
                    .aria_label(format!("{} {text}", mark.gutter()));
                let row = match mark {
                    Mark::Same => row.text_color(cx.theme().muted_foreground),
                    Mark::Changed => row.bg(cx.theme().warning.opacity(0.14)),
                    Mark::Added => row.bg(cx.theme().success.opacity(0.14)),
                };
                lines = lines.child(row.child(format!("{} {text}", mark.gutter())));
            }
        }
    }
    v_flex()
        .flex_1()
        .min_w_0()
        .gap_1()
        .child(div().text_xs().child(label))
        .child(lines)
}
