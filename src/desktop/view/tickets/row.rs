//! A ticket's row in the Tickets list (Part C): the type icon, the short
//! code, the title with its markers, the status pill and the age, in one
//! 28 px line.
//!
//! The row is a `ListRow`, so the Kit list can use it as its item. Its
//! name is the title, or for an entry with no title its path; its tooltip,
//! which is also its description, is `ticket_tooltip`'s lines. Each marker
//! is an icon with its own tooltip, which is also its accessible name.

use gpui_kit::{
    App, InteractiveElement, ParentElement, StatefulInteractiveElement, Styled, TestSupportExt,
    component::{ActiveTheme, h_flex},
    div,
    prelude::FluentBuilder,
    px,
};
use time::OffsetDateTime;

use super::ShownTicket;
use crate::desktop::{
    model::{
        status::{Tint, tint},
        ticket_view::{
            CANT_BE_READ_PILL, CANT_BE_READ_TYPE, CLOSED_LINE, CONFLICT_MARKER, NEEDS_FIXING_PILL,
            NEEDS_FIXING_TYPE, NO_SHORT_CODE, TYPE_PREFIX, UNSAVED_MARKER, UPDATED_PREFIX,
            age_label, ticket_tooltip,
        },
        tickets::{Lifecycle, TicketRow},
    },
    view::{
        item::relative_time,
        rows::{ListRow, named_icon},
        shell::tokens::{ChromeColors, chrome_colors},
    },
};

/// The widths of the cells that do not take the rest of the row (Part C).
const TYPE_WIDTH: f32 = 14.;
const CODE_WIDTH: f32 = 78.;
const STATUS_WIDTH: f32 = 92.;
const AGE_WIDTH: f32 = 30.;
/// The title keeps this much before the code and the status clip (R-25).
const TITLE_MIN_WIDTH: f32 = 96.;
const SMALL_TEXT: f32 = 11.;
const ICON_SIZE: f32 = 14.;
const MARKER_SIZE: f32 = 12.;

/// What a row is named by: its title, or for a nonconforming entry, which
/// has none, its path.
pub(crate) fn label(row: &TicketRow) -> String {
    row.name().title_or(&row.path).to_owned()
}

/// The icon a ticket's type is drawn with: `ticket` for any type the
/// design does not name.
fn type_icon(ticket_type: Option<&str>) -> &'static str {
    match ticket_type.map(str::to_lowercase).as_deref() {
        Some("cycle") => "repeat",
        Some("defect") => "bug",
        Some("feature") => "sparkles",
        Some("task") => "list-checks",
        _ => "ticket",
    }
}

/// The pill's text colour and the fill behind it.
fn pill_colors(status: &str, colors: &ChromeColors, cx: &App) -> (gpui_kit::Hsla, gpui_kit::Hsla) {
    let theme = cx.theme();
    match tint(status) {
        Some(tone) => {
            let tone = match tone {
                Tint::Working => theme.info,
                Tint::Done => theme.success,
                Tint::Draft => theme.warning,
                Tint::Blocked => theme.danger,
            };
            (tone, tone.opacity(0.13))
        }
        // An untinted status is neutral.
        None => (colors.text, theme.secondary),
    }
}

/// The row `shown` makes at `index`, with ages from `now` and `repo`, the
/// repository's display name, in its tooltip.
pub(super) fn ticket_row(
    index: usize,
    shown: &ShownTicket,
    repo: &str,
    now: OffsetDateTime,
    cx: &App,
) -> ListRow {
    let colors = chrome_colors(cx);
    let row = &shown.row;
    let conforming = row.id.is_some();
    let closed = row.lifecycle == Some(Lifecycle::Closed);

    let (type_name, type_color, type_text) = if conforming {
        let text = row.ticket_type.as_ref().map_or_else(
            || TYPE_PREFIX.trim_end().to_owned(),
            |kind| format!("{TYPE_PREFIX}{kind}"),
        );
        (
            type_icon(row.ticket_type.as_deref()),
            colors.secondary,
            text,
        )
    } else if row.unreadable.is_some() {
        ("circle-x", colors.red, CANT_BE_READ_TYPE.to_owned())
    } else {
        (
            "triangle-alert",
            colors.accent,
            NEEDS_FIXING_TYPE.to_owned(),
        )
    };
    let type_cell = div().flex_none().w(px(TYPE_WIDTH)).child(named_icon(
        ("ticket-type", index),
        type_name,
        ICON_SIZE,
        type_color,
        type_text,
    ));

    // A nonconforming entry has no short code; its cell keeps the columns
    // in line.
    let code_cell = div()
        .flex_shrink_1()
        .w(px(CODE_WIDTH))
        .min_w(px(0.))
        .overflow_hidden()
        .text_ellipsis()
        .whitespace_nowrap()
        .font_family(cx.theme().mono_font_family.clone())
        .text_size(px(SMALL_TEXT))
        .text_color(colors.secondary)
        .when(conforming, |cell| {
            let (text, name) = match row.slug.as_deref() {
                Some(slug) => (slug.to_owned(), slug.to_owned()),
                None => ("\u{2014}".to_owned(), NO_SHORT_CODE.to_owned()),
            };
            cell.child(
                div()
                    .id(("ticket-slug", index))
                    .test_support()
                    .aria_label(name)
                    .child(text),
            )
        });

    let markers = [
        (shown.markers.unsaved).then_some((
            "ticket-unsaved",
            "pencil",
            colors.accent,
            UNSAVED_MARKER,
        )),
        (shown.markers.conflict).then_some((
            "ticket-conflict",
            "circle-alert",
            colors.red,
            CONFLICT_MARKER,
        )),
        (shown.markers.closed).then_some((
            "ticket-closed",
            "circle-check",
            colors.secondary,
            CLOSED_LINE,
        )),
    ];
    let title_cell = h_flex()
        .flex_1()
        .min_w(px(TITLE_MIN_WIDTH))
        .gap(px(5.))
        .items_center()
        .child(
            div()
                .min_w(px(0.))
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .text_color(if closed {
                    colors.secondary
                } else {
                    colors.text
                })
                .child(label(row)),
        )
        .children(
            markers
                .into_iter()
                .flatten()
                .map(|(id, name, color, text)| {
                    named_icon((id, index), name, MARKER_SIZE, color, text)
                }),
        );

    let (status_text, status_text_color, status_fill) = if conforming {
        match row.status.as_deref() {
            Some(status) => {
                let (text_color, fill) = pill_colors(status, &colors, cx);
                (Some(status.to_owned()), text_color, fill)
            }
            None => (None, colors.text, cx.theme().secondary),
        }
    } else if row.unreadable.is_some() {
        (
            Some(CANT_BE_READ_PILL.to_owned()),
            colors.red,
            colors.red.opacity(0.13),
        )
    } else {
        (
            Some(NEEDS_FIXING_PILL.to_owned()),
            colors.accent,
            colors.accent.opacity(0.13),
        )
    };
    let status_cell = div()
        .flex_shrink_1()
        .w(px(STATUS_WIDTH))
        .min_w(px(0.))
        .overflow_hidden()
        .children(status_text.map(|text| {
            div()
                .id(("ticket-status", index))
                .test_support()
                .aria_label(text.clone())
                .px(px(6.))
                .py(px(1.))
                .rounded(px(4.))
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .text_size(px(SMALL_TEXT))
                .text_color(status_text_color)
                .bg(status_fill)
                .child(text)
        }));

    let age = age_label(row.changed_at.as_deref(), now);
    let age_cell = div()
        .flex_none()
        .w(px(AGE_WIDTH))
        .text_right()
        .text_size(px(SMALL_TEXT))
        .text_color(colors.muted)
        .when_some(row.changed_at.as_deref(), |cell, at| {
            let name = format!("{UPDATED_PREFIX}{}", relative_time(at, now));
            cell.child(
                div()
                    .id(("ticket-changed", index))
                    .test_support()
                    .aria_label(name)
                    .child(age.clone()),
            )
        });

    let lines = ticket_tooltip(row, repo);
    ListRow::new(("ticket-row", index))
        .label(label(row))
        .description(lines.join("; "))
        .tooltip(lines.join("\n"))
        .child(type_cell)
        .child(code_cell)
        .child(title_cell)
        .child(status_cell)
        .child(age_cell)
}
