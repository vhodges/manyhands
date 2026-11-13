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
use crate::desktop::model::conflict::{BodyChoice, Choice, Mark, Marked};

/// A side with no value.
const NONE: &str = "(none)";

/// A piece of text a test reads by `id`.
pub(in crate::desktop::view::conflict) fn text(
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

/// The sheet's line counts: Yours' and Theirs' `conflict-line` rows and
/// Before's `conflict-before` rows, each numbered across the sheet.
#[derive(Default)]
pub(super) struct Lines {
    pub(super) marked: Cell<usize>,
    pub(super) before: Cell<usize>,
}

/// Before's text in plain mono lines (handoff 2g); "(none)" when Before
/// has no value. Each line is a `conflict-before` row, numbered across the
/// sheet by `line`, so a test reads it.
pub(super) fn before_column(
    label: &'static str,
    value: Option<&str>,
    line: &Cell<usize>,
) -> impl IntoElement {
    let lines = v_flex().min_w_0().font_family("monospace").text_sm();
    let lines = match value {
        None => lines.child(div().child(NONE)),
        Some(value) => lines.children(value.lines().map(|text| {
            let at = line.get();
            line.set(at + 1);
            div()
                .id(("conflict-before", at))
                .test_support()
                .aria_label(text.to_owned())
                .child(text.to_owned())
        })),
    };
    side(label, lines)
}

/// Yours' or Theirs' lines, as the model marked them against Before once
/// per read (R-27; review M-e): unchanged lines muted, changed ones tinted
/// amber with `~`, added green with `+`, removed red with `-`. Each line is
/// a `conflict-line` row, numbered across the sheet. A side with no value
/// reads "(none)".
pub(super) fn marked_column(
    label: &'static str,
    marked: Option<&Marked>,
    line: &Cell<usize>,
    cx: &App,
) -> impl IntoElement {
    let mut lines = v_flex().min_w_0().font_family("monospace").text_sm();
    let Some(marked) = marked else {
        return side(label, lines.child(div().child(NONE)));
    };
    for (mark, text) in marked {
        let at = line.get();
        line.set(at + 1);
        let words = format!("{} {text}", mark.gutter());
        let row = div()
            .id(("conflict-line", at))
            .test_support()
            .aria_label(words.clone());
        let row = match mark {
            Mark::Same => row.text_color(cx.theme().muted_foreground),
            Mark::Changed => row.bg(cx.theme().warning.opacity(0.14)),
            Mark::Added => row.bg(cx.theme().success.opacity(0.14)),
            Mark::Removed => row.bg(cx.theme().danger.opacity(0.14)),
        };
        lines = lines.child(row.child(words));
    }
    side(label, lines)
}

fn side(label: &'static str, lines: gpui_kit::Div) -> impl IntoElement {
    v_flex()
        .flex_1()
        .min_w_0()
        .gap_1()
        .child(div().text_xs().child(label))
        .child(lines)
}
