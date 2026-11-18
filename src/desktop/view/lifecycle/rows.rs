//! The lifecycle dialog's step rows and boxes (Cycle 05b, A3, A6).
//!
//! One drawer serves every stage: the confirm list, the running steps and
//! the ended ones. A row is its mark, its title and its note.
//!
//! - **The mark.** The step's number on confirm, and on later stages for a
//!   step not started or not done; a green check for Done, a spinner for
//!   Running, a red cross for Failed. Past confirm its wrapper keeps the id
//!   `("lifecycle-step-state", ix)` and is labelled with `StepState::words`,
//!   so tests and screen readers read the state as they always have (R-2).
//! - **The note**, `("lifecycle-step-note", ix)`, when it is not empty. A
//!   failed step's own note gives way to the answer's message, which keeps
//!   the id `lifecycle-message`.
//!
//! The boxes are the outcome box of an ended run, with its tone's icon, and
//! the tinted box the made-ready part sits in.

use gpui_kit::{
    AnyElement, App, Div, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt,
    component::{ActiveTheme, Icon, h_flex, spinner::Spinner, v_flex},
    div, px,
};

use super::{
    outcome::{Ending, StepState},
    text,
};
use crate::desktop::{
    assets::icon_path,
    view::shell::tokens::{ChromeColors, chrome_colors},
};

/// The mark's cell, and its glyphs.
const MARK: f32 = 20.;
const MARK_ICON: f32 = 18.;
/// A box's corners, its tint, and its icon.
const BOX_RADIUS: f32 = 6.;
const BOX_TINT: f32 = 0.08;
const BOX_ICON: f32 = 16.;

/// What a step's mark shows.
#[derive(Clone, Copy)]
pub(super) enum Mark {
    /// The confirm list: the step's number.
    Planned,
    /// A step of a run sent, by its state.
    State(StepState),
}

/// What a step's note shows.
pub(super) enum Note {
    /// The step's words: its own, or the stage a run is at. Drawn when not
    /// empty.
    Words(String),
    /// The failure's message, in place of the step's own note.
    Failure(String),
}

/// One row of the list.
pub(super) struct StepRow<'a> {
    pub title: &'a str,
    pub mark: Mark,
    pub note: Note,
}

/// The bordered list of `rows`.
pub(super) fn step_rows(rows: Vec<StepRow<'_>>, cx: &App) -> AnyElement {
    let colors = chrome_colors(cx);
    v_flex()
        .gap_2()
        .p_2()
        .border_1()
        .border_color(cx.theme().border)
        .rounded_md()
        .children(
            rows.into_iter()
                .enumerate()
                .map(|(ix, row)| step_row(ix, row, &colors, cx)),
        )
        .into_any_element()
}

fn step_row(ix: usize, row: StepRow<'_>, colors: &ChromeColors, cx: &App) -> impl IntoElement {
    let muted = cx.theme().muted_foreground;
    let failed = matches!(row.mark, Mark::State(StepState::Failed));
    let title = text(("lifecycle-step", ix), row.title.to_owned())
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD);
    let title = if failed {
        title.text_color(colors.red)
    } else {
        title
    };
    let note = match row.note {
        Note::Words(words) if !words.is_empty() => Some(
            text(("lifecycle-step-note", ix), words)
                .text_xs()
                .text_color(muted)
                .into_any_element(),
        ),
        Note::Failure(message) => Some(
            text("lifecycle-message", message)
                .text_xs()
                .text_color(colors.red)
                .into_any_element(),
        ),
        Note::Words(_) => None,
    };
    h_flex()
        .id(("lifecycle-step-row", ix))
        .test_support()
        .gap_2()
        .items_start()
        .child(mark(ix, row.mark, colors, cx))
        .child(v_flex().min_w_0().child(title).children(note))
}

/// The mark of the step at `ix`.
fn mark(ix: usize, mark: Mark, colors: &ChromeColors, cx: &App) -> AnyElement {
    let number = || {
        div()
            .flex_none()
            .w(px(MARK))
            .h(px(MARK))
            .rounded_full()
            .border_1()
            .border_color(cx.theme().border)
            .text_color(cx.theme().muted_foreground)
            .flex()
            .items_center()
            .justify_center()
            .text_xs()
            .child((ix + 1).to_string())
            .into_any_element()
    };
    let icon = |name: &str, hue: Hsla| {
        Icon::default()
            .path(icon_path(name))
            .w(px(MARK_ICON))
            .h(px(MARK_ICON))
            .text_color(hue)
            .into_any_element()
    };
    let state = match mark {
        Mark::Planned => return number(),
        Mark::State(state) => state,
    };
    let glyph = match state {
        StepState::Done => icon("circle-check", colors.green),
        StepState::Running => Spinner::new().color(colors.teal).into_any_element(),
        StepState::Failed => icon("circle-x", colors.red),
        StepState::NotStarted | StepState::NotDone => number(),
    };
    div()
        .id(("lifecycle-step-state", ix))
        .test_support()
        .aria_label(state.words())
        .flex_none()
        .w(px(MARK))
        .h(px(MARK))
        .flex()
        .items_center()
        .justify_center()
        .child(glyph)
        .into_any_element()
}

/// A box of radius 6, a 1 px border in `tone` and `tone` at 8% behind.
fn tinted(tone: Hsla) -> Div {
    div()
        .rounded(px(BOX_RADIUS))
        .border_1()
        .border_color(tone)
        .bg(tone.opacity(BOX_TINT))
        .p_3()
}

/// The outcome box of a run that `ending` ended (A6): `lines` beside the
/// tone's icon, the ending word first.
pub(super) fn outcome_box(ending: Ending, lines: Vec<AnyElement>, cx: &App) -> AnyElement {
    let colors = chrome_colors(cx);
    let (icon, tone) = match ending {
        Ending::Done => ("circle-check", colors.green),
        Ending::Failed => ("shield-check", colors.red),
        Ending::Stopped | Ending::StillRunning => ("shield-check", colors.accent),
    };
    tinted(tone)
        .id("lifecycle-box")
        .test_support()
        .flex()
        .gap_2()
        .items_start()
        .child(
            Icon::default()
                .path(icon_path(icon))
                .flex_none()
                .w(px(BOX_ICON))
                .h(px(BOX_ICON))
                .text_color(tone),
        )
        .child(v_flex().flex_1().min_w_0().gap_1().children(lines))
        .into_any_element()
}

/// The made-ready part (A6), in its teal box.
pub(super) fn made_ready_box(part: AnyElement, cx: &App) -> AnyElement {
    tinted(chrome_colors(cx).teal)
        .child(part)
        .into_any_element()
}
