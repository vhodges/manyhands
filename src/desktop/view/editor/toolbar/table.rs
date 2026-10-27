//! The toolbar's table controls (design, "The toolbar", the Table row):
//! the editor's own table methods (facts 36 and 80), on the table the
//! caret is in.
//!
//! Markdown mode draws no table, and `set_caret_table_align` reads the
//! drawn rows, so it does nothing there (`zorite-editor` `tables.rs:456`,
//! `element.rs:2883`). In Markdown the alignment is a host edit instead,
//! which writes the separator row exactly as the editor would: every
//! column as ` :-- `, ` :-: ` or ` --: `, the caret's set to the one asked.
//! The row and column methods read the text, and work in both modes.

use std::ops::Range;

use gpui_kit::Context;
use zorite_editor::CellAlign;

use super::{Toolbar, apply, controls::Control};
use crate::desktop::model::markdown_edit::HostEdit;

impl Toolbar {
    /// Does what the table control `control` names.
    pub(super) fn table(&mut self, control: Control, cx: &mut Context<Self>) {
        let align = match control {
            Control::RowBelow => {
                return self
                    .editor
                    .update(cx, |editor, cx| editor.insert_table_row(true, cx));
            }
            Control::ColumnRight => {
                return self
                    .editor
                    .update(cx, |editor, cx| editor.insert_table_column(true, cx));
            }
            Control::DeleteRow => {
                return self
                    .editor
                    .update(cx, |editor, cx| editor.delete_table_row(cx));
            }
            Control::DeleteColumn => {
                return self
                    .editor
                    .update(cx, |editor, cx| editor.delete_table_column(cx));
            }
            Control::AlignLeft => CellAlign::Left,
            Control::AlignCentre => CellAlign::Center,
            Control::AlignRight => CellAlign::Right,
            _ => return,
        };
        if self.host.read(cx).is_rich() {
            self.editor
                .update(cx, |editor, cx| editor.set_caret_table_align(align, cx));
            return;
        }
        let editor = self.editor.read(cx);
        if let Some(edit) = align_column(editor.text(), editor.cursor(), align) {
            apply(&self.editor, edit, cx);
        }
    }
}

/// The edit that aligns the column the caret is in, as the editor's
/// `set_caret_table_align` writes it, or nothing outside a table's header
/// or body rows. Tables are found as the editor finds them
/// (`markdown_syntax.rs:2075-2120`): a row, then a separator, then the
/// rows up to the next table's header.
pub(super) fn align_column(text: &str, cursor: usize, align: CellAlign) -> Option<HostEdit> {
    let lines = lines(text);
    let row = lines
        .iter()
        .rposition(|line| line.start <= cursor)
        .unwrap_or(0);
    let mut start = 0;
    while start + 1 < lines.len() {
        let separator = &text[lines[start + 1].clone()];
        let Some(mut aligns) = is_row(&text[lines[start].clone()])
            .then(|| separator_aligns(separator))
            .flatten()
        else {
            start += 1;
            continue;
        };
        let mut end = start + 2;
        while end < lines.len()
            && is_row(&text[lines[end].clone()])
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
        let column = cell_ranges(line)
            .iter()
            .position(|cell| cursor - lines[row].start <= cell.end)?;
        *aligns.get_mut(column)? = align;
        let mut replacement = String::from("|");
        for align in aligns {
            replacement.push_str(match align {
                CellAlign::Left => " :-- |",
                CellAlign::Center => " :-: |",
                CellAlign::Right => " --: |",
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

/// A table row: a line whose text starts with `|`.
fn is_row(line: &str) -> bool {
    line.trim_start().starts_with('|')
}

/// A separator row's alignment per column: every cell dashes, with a
/// colon on the left, the right or both.
fn separator_aligns(line: &str) -> Option<Vec<CellAlign>> {
    if !is_row(line) {
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
                    (true, true) => CellAlign::Center,
                    (false, true) => CellAlign::Right,
                    _ => CellAlign::Left,
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
