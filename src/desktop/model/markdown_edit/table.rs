//! Pipe tables, read as the editor reads them: whether the caret is on a
//! row of one, and the alignment edit Markdown mode needs (R-55).
//!
//! Markdown mode draws no table, and the editor's `set_caret_table_align`
//! reads the drawn rows, so it does nothing there (`zorite-editor`
//! `tables.rs:456`, `element.rs:2883`). [`align_column`] is that edit as a
//! host edit: it writes the separator row exactly as the editor would.

use std::ops::Range;

use super::{HostEdit, floor, line_end, line_start};

/// A column's alignment, as a separator row writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ColumnAlign {
    Left,
    Centre,
    Right,
}

/// Whether the caret sits on a row of a pipe table: a run of `|` lines with
/// a `| --- |` separator row. Like the editor's own check, the separator
/// row itself does not count.
pub(crate) fn table_at(text: &str, cursor: usize) -> bool {
    let cursor = floor(text, cursor);
    let ls = line_start(text, cursor);
    let le = line_end(text, cursor);
    let line = &text[ls..le];
    if !is_pipe_row(line) || is_separator_row(line) || in_code_fence(text, ls) {
        return false;
    }
    let mut begin = ls;
    while begin > 0 {
        let prev = line_start(text, begin - 1);
        if !is_pipe_row(&text[prev..begin - 1]) {
            break;
        }
        begin = prev;
    }
    let mut pos = begin;
    loop {
        let end = line_end(text, pos);
        let row = &text[pos..end];
        if !is_pipe_row(row) {
            return false;
        }
        if is_separator_row(row) {
            return true;
        }
        if end >= text.len() || end > le {
            return false;
        }
        pos = end + 1;
    }
}

/// The edit that aligns the column the caret is in, as the editor's
/// `set_caret_table_align` writes it, or nothing outside a table's header
/// or body rows. Tables are found as the editor finds them
/// (`markdown_syntax.rs:2075-2120`): a row, then a separator, then the
/// rows up to the next table's header. A caret past a row's last cell is
/// in that cell, as in the editor's `table_cell_at` (`tables.rs:1226`).
/// Inside a code fence the editor draws no table, and nothing is aligned.
pub(crate) fn align_column(text: &str, cursor: usize, align: ColumnAlign) -> Option<HostEdit> {
    let cursor = floor(text, cursor);
    let lines = lines(text);
    let row = lines
        .iter()
        .rposition(|line| line.start <= cursor)
        .unwrap_or(0);
    if in_code_fence(text, lines[row].start) {
        return None;
    }
    let mut start = 0;
    while start + 1 < lines.len() {
        let separator = &text[lines[start + 1].clone()];
        let Some(mut aligns) = is_pipe_row(&text[lines[start].clone()])
            .then(|| separator_aligns(separator))
            .flatten()
        else {
            start += 1;
            continue;
        };
        let mut end = start + 2;
        while end < lines.len()
            && is_pipe_row(&text[lines[end].clone()])
            && lines
                .get(end + 1)
                .is_none_or(|next| separator_aligns(&text[next.clone()]).is_none())
        {
            end += 1;
        }
        if !(start..end).contains(&row) {
            start = end;
            continue;
        }
        if row == start + 1 {
            return None;
        }
        let line = &text[lines[row].clone()];
        let cells = cell_ranges(line);
        let offset = cursor - lines[row].start;
        let column = cells
            .iter()
            .position(|cell| offset <= cell.end)
            .unwrap_or(cells.len().checked_sub(1)?);
        *aligns.get_mut(column)? = align;
        let mut replacement = String::from("|");
        for align in aligns {
            replacement.push_str(match align {
                ColumnAlign::Left => " :-- |",
                ColumnAlign::Centre => " :-: |",
                ColumnAlign::Right => " --: |",
            });
        }
        let range = lines[start + 1].clone();
        let cursor = if cursor >= range.end {
            cursor + replacement.len() - range.len()
        } else {
            cursor
        };
        return Some(HostEdit {
            range,
            replacement,
            cursor,
        });
    }
    None
}

/// Each line's byte range, without its `\n`.
fn lines(text: &str) -> Vec<Range<usize>> {
    let mut start = 0;
    text.split('\n')
        .map(|line| {
            let range = start..start + line.len();
            start = range.end + 1;
            range
        })
        .collect()
}

/// A separator row's alignment per column: every cell dashes, with a
/// colon on the left, the right or both.
fn separator_aligns(line: &str) -> Option<Vec<ColumnAlign>> {
    if !is_pipe_row(line) {
        return None;
    }
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    let cells = cells(inner);
    if cells.is_empty() {
        return None;
    }
    cells
        .into_iter()
        .map(|cell| {
            let cell = &inner[cell];
            let cell = cell.trim();
            let dashes = cell.trim_matches(':');
            (!dashes.is_empty() && dashes.bytes().all(|byte| byte == b'-')).then_some(
                match (cell.starts_with(':'), cell.ends_with(':')) {
                    (true, true) => ColumnAlign::Centre,
                    (false, true) => ColumnAlign::Right,
                    _ => ColumnAlign::Left,
                },
            )
        })
        .collect()
}

/// The cells of a row, as line offsets of their trimmed text, between its
/// first and last `|`.
fn cell_ranges(line: &str) -> Vec<Range<usize>> {
    let (Some(first), Some(last)) = (line.find('|'), line.rfind('|')) else {
        return Vec::new();
    };
    if last <= first {
        return Vec::new();
    }
    let base = first + 1;
    let inner = &line[base..last];
    cells(inner)
        .into_iter()
        .map(|cell| {
            let piece = &inner[cell.clone()];
            let start = base + cell.start + (piece.len() - piece.trim_start().len());
            let end = base + cell.end - (piece.len() - piece.trim_end().len());
            start..end.max(start)
        })
        .collect()
}

/// `inner` split at its unescaped `|`, as ranges.
fn cells(inner: &str) -> Vec<Range<usize>> {
    let bytes = inner.as_bytes();
    let mut out = Vec::new();
    let mut start = 0;
    for (index, byte) in bytes.iter().enumerate() {
        let escaped = bytes[..index]
            .iter()
            .rev()
            .take_while(|byte| **byte == b'\\')
            .count()
            % 2
            == 1;
        if *byte == b'|' && !escaped {
            out.push(start..index);
            start = index + 1;
        }
    }
    out.push(start..inner.len());
    out
}

/// Whether the line starting at `line_pos` lies in (or is a line of) a fenced
/// code block: backticks or tildes, three or more. An unclosed fence runs on.
fn in_code_fence(text: &str, line_pos: usize) -> bool {
    let mut open: Option<(char, usize)> = None;
    let mut pos = 0;
    while pos <= line_pos {
        let end = line_end(text, pos);
        let trimmed = text[pos..end].trim_start();
        let run = trimmed.chars().next().filter(|c| matches!(c, '`' | '~'));
        let len = run.map_or(0, |c| trimmed.chars().take_while(|x| *x == c).count());
        let is_fence = len >= 3;
        match (open, run) {
            (None, Some(c)) if is_fence => {
                open = Some((c, len));
                if pos == line_pos {
                    return true;
                }
            }
            (Some((oc, ol)), Some(c))
                if is_fence && c == oc && len >= ol && trimmed[len..].trim().is_empty() =>
            {
                open = None;
                if pos == line_pos {
                    return true;
                }
            }
            _ => {}
        }
        if pos == line_pos {
            return open.is_some();
        }
        pos = end + 1;
    }
    false
}

fn is_pipe_row(line: &str) -> bool {
    line.trim_start().starts_with('|')
}

fn is_separator_row(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('|') && t.contains('-') && t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

#[cfg(test)]
#[path = "table_tests.rs"]
mod tests;
