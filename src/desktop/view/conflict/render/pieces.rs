//! The sheet's small pieces (handoff 2g; Wave 04 Cycle 05b B5): labelled
//! text, a part's card frame, its header row, which wraps in a narrow
//! window, its state and its choices, the chosen one named so, and a
//! side's mono lines with their tints and gutters.

use std::cell::Cell;

use gpui_kit::{
    AnyElement, App, Div, ElementId, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt,
    assets::IconName,
    component::{ActiveTheme, Selectable, Sizable, button::Button, h_flex, v_flex},
    div, px,
};
use manyhands::canonical::ItemKind;

use super::super::{
    CHOOSE_ONE, CHOSEN, COMMENT_DETAIL, DOCUMENT_DETAIL, EDITED, LAST_SHARED_COPY, TAKEN_MINE,
    TAKEN_THEIRS, TICKET_DETAIL, chosen_choice,
};
use crate::desktop::model::conflict::{BodyChoice, Choice, Mark, Marked};

/// A side with no value.
const NONE: &str = "(none)";
const BEFORE: &str = "Before";
const YOURS: &str = "Yours";
const THEIRS: &str = "Theirs";

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

/// A part's card (`05`, 2g): bordered, radius 6. A set that goes outside
/// draws its cards in the same frame (B5).
pub(in crate::desktop::view::conflict) fn card_frame(cx: &App) -> Div {
    v_flex()
        .gap_1()
        .p_2()
        .border_1()
        .border_color(cx.theme().border)
        .rounded(px(6.))
}

/// A card's header row (B5): the part's name, its kind, its state, then,
/// right-aligned, its choices. The app sets no minimum window size (R-28):
/// in a narrow window the row wraps, the choices onto a line of their own,
/// still right-aligned, rather than running past the card's edge, where a
/// click misses.
pub(super) fn header_row(
    parts: impl IntoIterator<Item = AnyElement>,
    choices: impl IntoIterator<Item = Button>,
) -> Div {
    h_flex()
        .flex_wrap()
        .gap_2()
        .items_center()
        .children(parts)
        .child(
            h_flex()
                .flex_wrap()
                .justify_end()
                .ml_auto()
                .gap_2()
                .children(choices),
        )
}

/// One of a part's choices. The chosen one keeps the Kit's `selected`
/// style and shows a check before its label, so the choice is not told by
/// colour alone (B5, K-8). Neither reaches a screen reader, so its
/// accessible name says it is chosen too (`chosen_choice`).
pub(super) fn choice_button(
    id: (&'static str, usize),
    label: &'static str,
    chosen: bool,
) -> Button {
    let button = Button::new(id).label(label).small().selected(chosen);
    if chosen {
        button
            .icon(IconName::Check)
            .accessibility_label(chosen_choice(label))
    } else {
        button
    }
}

/// What a field's card says of the detail it holds, by the kind of the
/// item at its path (B5).
pub(super) fn kind_words(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Ticket => TICKET_DETAIL,
        ItemKind::Document => DOCUMENT_DETAIL,
        ItemKind::Comment => COMMENT_DETAIL,
    }
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
        Some(Choice::Mine | Choice::Theirs) => CHOSEN,
    }
}

pub(super) fn body_state(choice: Option<BodyChoice>) -> &'static str {
    match choice {
        None => CHOOSE_ONE,
        Some(BodyChoice::Mine | BodyChoice::Theirs | BodyChoice::Both) => CHOSEN,
        Some(BodyChoice::Edited) => EDITED,
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

/// A part's Before, Yours and Theirs, in three equal columns of mono lines
/// (B5).
pub(super) fn columns(
    before: Option<&str>,
    yours: Option<&Marked>,
    theirs: Option<&Marked>,
    line: &Lines,
    cx: &App,
) -> Div {
    h_flex()
        .gap_2()
        .child(before_column(before, &line.before, cx))
        .child(marked_column(YOURS, yours, &line.marked, cx))
        .child(marked_column(THEIRS, theirs, &line.marked, cx))
}

/// Before's text in plain mono lines (handoff 2g), its label noting it is
/// the last shared copy (B5); "(none)" when Before has no value. Each line
/// is a `conflict-before` row, numbered across the sheet by `line`, so a
/// test reads it.
fn before_column(value: Option<&str>, line: &Cell<usize>, cx: &App) -> impl IntoElement {
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
    side(BEFORE, Some(LAST_SHARED_COPY), lines, cx)
}

/// Yours' or Theirs' lines, as the model marked them against Before once
/// per read (R-27; review M-e): unchanged lines muted, changed ones tinted
/// amber with `~`, added green with `+`, removed red with `-`. Each line is
/// a `conflict-line` row, numbered across the sheet. A side with no value
/// reads "(none)".
fn marked_column(
    label: &'static str,
    marked: Option<&Marked>,
    line: &Cell<usize>,
    cx: &App,
) -> impl IntoElement {
    let mut lines = v_flex().min_w_0().font_family("monospace").text_sm();
    let Some(marked) = marked else {
        return side(label, None, lines.child(div().child(NONE)), cx);
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
    side(label, None, lines, cx)
}

/// A column: its label, with its muted note after it, over its lines.
fn side(label: &'static str, note: Option<&'static str>, lines: Div, cx: &App) -> impl IntoElement {
    let muted = cx.theme().muted_foreground;
    v_flex()
        .flex_1()
        .min_w_0()
        .gap_1()
        .child(
            h_flex()
                .gap_1()
                .text_xs()
                .child(label)
                .children(note.map(|note| div().text_color(muted).child(note))),
        )
        .child(lines)
}
