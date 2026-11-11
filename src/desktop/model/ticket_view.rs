//! The Tickets panel's view of one read: the rows listed under the query,
//! the counts beside them and the words that go with them (Part B).
//!
//! The panel reads once per load. Lifecycle, readiness, the text filter
//! and the sort are applied here, by pure functions, so a filter change
//! never reads again.

use std::cmp::Reverse;

use manyhands::repository::{ClosureFilter, ReadinessFilter, ReadinessState};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use super::tickets::{Lifecycle, SortColumn, TicketQuery, TicketRow};

pub(crate) const NO_SHORT_CODE: &str = "No short code";
/// The tooltip's lines: `ID {ULID}`, `Type {type}`, `Closed`.
pub(crate) const ID_PREFIX: &str = "ID ";
pub(crate) const TYPE_PREFIX: &str = "Type ";
pub(crate) const CLOSED_LINE: &str = "Closed";
/// A nonconforming entry: `Needs fixing: {guidance} Open it to fix.`
pub(crate) const NEEDS_FIXING_PREFIX: &str = "Needs fixing: ";
pub(crate) const OPEN_IT_TO_FIX: &str = " Open it to fix.";
/// For a nonconforming entry with no problem code. This is copy the
/// design does not give, in the words of the line above.
pub(crate) const NEEDS_FIXING_NO_GUIDANCE: &str = "Needs fixing. Open it to fix.";
/// A nonconforming entry's status pill and the name of its type cell.
pub(crate) const NEEDS_FIXING_PILL: &str = "needs fixing";
pub(crate) const NEEDS_FIXING_TYPE: &str = "Needs fixing";
/// An unreadable file's status pill and the name of its type cell.
#[allow(dead_code)] // Used from Task 7; no test reads it before.
pub(crate) const CANT_BE_READ_PILL: &str = "can't be read";
#[allow(dead_code)] // Used from Task 7; no test reads it before.
pub(crate) const CANT_BE_READ_TYPE: &str = "Can't be read";
/// The markers' tooltips, which are also their accessible names. The
/// Closed marker's is `CLOSED_LINE`.
pub(crate) const UNSAVED_MARKER: &str = "You have unsaved work on this.";
pub(crate) const CONFLICT_MARKER: &str =
    "This has a conflict to resolve. Open it to choose what to keep.";
/// The accessible name of a row's age: `Updated {relative time}`.
pub(crate) const UPDATED_PREFIX: &str = "Updated ";
/// The hint above the rows, for one ticket and for `{k}`.
pub(crate) const MORE_MATCH_ONE: &str = "1 more ticket matches with other filters.";
pub(crate) fn more_match_text(k: usize) -> String {
    format!("{k} more tickets match with other filters.")
}

/// The tickets each lifecycle segment counts, among those the text
/// matches.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SegmentCounts {
    pub open: usize,
    pub closed: usize,
    pub all: usize,
}

/// What a read shows under a query.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct TicketView {
    /// Every row the panel draws, in order: the conforming ones sorted,
    /// then the nonconforming ones in read order.
    pub rows: Vec<TicketRow>,
    /// `None` with nothing to count: no rows read, or none kept.
    pub counts: Option<SegmentCounts>,
    /// Conforming tickets the text matches that the lifecycle and
    /// readiness choices leave out.
    pub elsewhere: usize,
    /// Rows drawn.
    pub listed: usize,
    /// The conforming ones.
    pub shown: usize,
    pub needs_fixing: usize,
    pub unreadable: usize,
}

fn open(row: &TicketRow) -> bool {
    row.lifecycle != Some(Lifecycle::Closed)
}

/// R-16: a trimmed text matches a title or short code by a
/// case-insensitive substring, or a whole ID ignoring case. Nothing
/// trimmed matches everything.
fn matches(row: &TicketRow, text: &str) -> bool {
    if text.is_empty() {
        return true;
    }
    let text = text.to_lowercase();
    let contains = |field: &Option<String>| {
        field
            .as_deref()
            .is_some_and(|field| field.to_lowercase().contains(&text))
    };
    contains(&row.title)
        || contains(&row.slug)
        || row
            .id
            .as_ref()
            .is_some_and(|id| id.to_string().to_lowercase() == text)
}

fn listed_under(row: &TicketRow, query: &TicketQuery) -> bool {
    let lifecycle = match query.closure {
        ClosureFilter::All => true,
        ClosureFilter::Open => open(row),
        ClosureFilter::Closed => !open(row),
    };
    let readiness = query.readiness.is_none_or(|wanted| {
        row.readiness
            == Some(match wanted {
                ReadinessFilter::Ready => ReadinessState::Ready,
                ReadinessFilter::Blocked => ReadinessState::Blocked,
            })
    });
    lifecycle && readiness
}

/// The text a column sorts by; nothing for a value that is not there.
fn text_key(column: SortColumn, row: &TicketRow) -> Option<String> {
    let value = match column {
        SortColumn::Type => &row.ticket_type,
        SortColumn::Code => &row.slug,
        SortColumn::Title => &row.title,
        SortColumn::Status => &row.status,
        SortColumn::Updated => return None,
    };
    value
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_lowercase)
}

fn changed(row: &TicketRow) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(row.changed_at.as_deref()?, &Rfc3339).ok()
}

/// R-10: a stable sort over the library's order. Descending reverses the
/// rows that have the value; rows without it stay last either way.
fn sort(rows: Vec<&TicketRow>, query: &TicketQuery) -> Vec<TicketRow> {
    let column = query.sort.column;
    let (mut with, without): (Vec<_>, Vec<_>) = if column == SortColumn::Updated {
        rows.into_iter().partition(|row| changed(row).is_some())
    } else {
        rows.into_iter()
            .partition(|row| text_key(column, row).is_some())
    };
    if column == SortColumn::Updated {
        // By age, so ascending is newest first.
        with.sort_by_cached_key(|row| Reverse(changed(row)));
    } else {
        with.sort_by_cached_key(|row| text_key(column, row));
    }
    if !query.sort.ascending {
        with.reverse();
    }
    with.into_iter().chain(without).cloned().collect()
}

/// The rows listed under `query`, counted and sorted. `None` is no rows
/// at all.
pub(crate) fn derive(rows: Option<&[TicketRow]>, query: &TicketQuery) -> TicketView {
    let Some(rows) = rows else {
        return TicketView::default();
    };
    let text = query.text.trim();
    let mut counts = SegmentCounts::default();
    let (mut elsewhere, mut conforming, mut nonconforming) = (0, Vec::new(), Vec::new());
    for row in rows {
        // R-9: a nonconforming entry passes every filter, the text too.
        if row.id.is_none() {
            nonconforming.push(row);
            continue;
        }
        if !matches(row, text) {
            continue;
        }
        if open(row) {
            counts.open += 1;
        } else {
            counts.closed += 1;
        }
        if listed_under(row, query) {
            conforming.push(row);
        } else if !text.is_empty() {
            elsewhere += 1;
        }
    }
    counts.all = counts.open + counts.closed;
    let shown = conforming.len();
    let needs_fixing = nonconforming.len();
    let mut rows = sort(conforming, query);
    rows.extend(nonconforming.into_iter().cloned());
    TicketView {
        listed: rows.len(),
        rows,
        counts: Some(counts),
        elsewhere,
        shown,
        needs_fixing,
        unreadable: 0,
    }
}

/// The line under the list (decision 7): what is shown, in which
/// repository, matching what, and what needs attention.
pub(crate) fn footnote(view: &TicketView, query: &TicketQuery, repo: &str) -> String {
    let word = match (query.readiness, query.closure) {
        (Some(ReadinessFilter::Ready), _) => "ready ",
        (Some(ReadinessFilter::Blocked), _) => "blocked ",
        (None, ClosureFilter::Open) => "open ",
        (None, ClosureFilter::Closed) => "closed ",
        (None, ClosureFilter::All) => "",
    };
    let n = view.shown;
    let tickets = if n == 1 { "ticket" } else { "tickets" };
    let mut line = format!("{n} {word}{tickets} in {repo}");
    let text = query.text.trim();
    if !text.is_empty() {
        let verb = if n == 1 { "matches" } else { "match" };
        line.push_str(&format!(" {verb} \"{text}\""));
    }
    if view.needs_fixing > 0 {
        let verb = if view.needs_fixing == 1 {
            "needs"
        } else {
            "need"
        };
        line.push_str(&format!(" \u{b7} {} {verb} fixing", view.needs_fixing));
    }
    if view.unreadable > 0 {
        line.push_str(&format!(" \u{b7} {} can't be read", view.unreadable));
    }
    line
}

/// The line above the rows when other filters hold more matches
/// (decision 2). The empty state says the same when nothing is listed.
pub(crate) fn hint(view: &TicketView) -> Option<String> {
    (view.listed > 0 && view.elsewhere > 0).then(|| match view.elsewhere {
        1 => MORE_MATCH_ONE.to_owned(),
        k => more_match_text(k),
    })
}

/// How long ago a ticket changed, as the prototype writes it (R-20):
/// whole minutes under an hour, rounded hours under a day, then rounded
/// days. A time after `now` is `0m`; one that cannot be read is blank.
pub(crate) fn age_label(changed_at: Option<&str>, now: OffsetDateTime) -> String {
    let Some(then) = changed_at.and_then(|at| OffsetDateTime::parse(at, &Rfc3339).ok()) else {
        return String::new();
    };
    let minutes = ((now - then).whole_seconds() / 60).max(0);
    match minutes {
        0..60 => format!("{minutes}m"),
        60..1440 => format!("{}h", (minutes + 30) / 60),
        _ => format!("{}d", (minutes + 720) / 1440),
    }
}

/// A row's tooltip, line by line, each only where it applies (R-14); its
/// description is the lines joined by "; ".
pub(crate) fn ticket_tooltip(row: &TicketRow, repo: &str) -> Vec<String> {
    if row.id.is_none() {
        let guidance = row.nonconforming.map(|problem| problem.guidance());
        return vec![match guidance {
            Some(guidance) => format!("{NEEDS_FIXING_PREFIX}{guidance}{OPEN_IT_TO_FIX}"),
            None => NEEDS_FIXING_NO_GUIDANCE.to_owned(),
        }];
    }
    let mut lines = vec![format!(
        "{} \u{b7} {repo}",
        row.slug.as_deref().unwrap_or(NO_SHORT_CODE)
    )];
    lines.extend(row.id.as_ref().map(|id| format!("{ID_PREFIX}{id}")));
    lines.extend(
        row.ticket_type
            .as_ref()
            .map(|kind| format!("{TYPE_PREFIX}{kind}")),
    );
    lines.push(row.context.label().to_owned());
    lines.push(row.context.git_line());
    lines.extend(row.reasons.iter().cloned());
    if row.lifecycle == Some(Lifecycle::Closed) {
        lines.push(CLOSED_LINE.to_owned());
    }
    lines
}

#[cfg(test)]
#[path = "ticket_view_tests.rs"]
mod tests;
