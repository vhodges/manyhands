//! The toolbar's host edits (R-55): pure functions from Markdown text and a
//! selection to a range, its replacement and the new caret, applied later
//! with `replace_range` as one undo step. They read and write `\n` line
//! endings only: the editor's text has been through `crlf_to_lf`.
//!
//! Offsets are byte offsets into the text. The editor reports its selection
//! in UTF-16; [`selection_from_utf16`] converts once.
//!
//! No type here derives `Debug` over body text: `HostEdit` holds the
//! replacement text and so has no `Debug`.

#![cfg_attr(not(test), allow(dead_code))] // Called from Task 14.

use std::ops::Range;

/// A selection as byte offsets, `start <= end`, on char boundaries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Selection {
    pub start: usize,
    pub end: usize,
}

/// One edit: replace `range` with `replacement`, then put the caret at
/// `cursor`, a byte offset in the new text.
pub(crate) struct HostEdit {
    pub range: Range<usize>,
    pub replacement: String,
    pub cursor: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BlockStyle {
    Paragraph,
    Heading(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LinePrefix {
    Bullet,
    Numbered,
    Task,
    Quote,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mark {
    Bold,
    Italic,
    Strike,
    Code,
}

const TABLE_TEMPLATE: &str = "| Column 1 | Column 2 |\n| --- | --- |\n|  |  |";

/// Converts a UTF-16 range to byte offsets. An offset inside a surrogate
/// pair or past the end is clamped to a char boundary: the start rounds
/// down and the end rounds up, so a split char stays selected. A reversed
/// range is put in order.
pub(crate) fn selection_from_utf16(text: &str, utf16: Range<usize>) -> Selection {
    let (lo, hi) = if utf16.start <= utf16.end {
        (utf16.start, utf16.end)
    } else {
        (utf16.end, utf16.start)
    };
    let mut start = text.len();
    let mut end = text.len();
    let mut units = 0;
    let mut found_start = false;
    for (byte, ch) in text.char_indices() {
        if !found_start && units + ch.len_utf16() > lo {
            start = byte;
            found_start = true;
        }
        if units >= hi {
            end = byte;
            break;
        }
        units += ch.len_utf16();
    }
    Selection {
        start: start.min(end),
        end,
    }
}

/// Rewrites the caret line's `#` prefix: sets a heading, or clears it.
pub(crate) fn set_block_style(text: &str, sel: Selection, style: BlockStyle) -> HostEdit {
    let caret = floor(text, sel.start);
    let ls = line_start(text, caret);
    let le = line_end(text, caret);
    let old = heading_prefix_len(&text[ls..le]);
    let new_prefix = match style {
        BlockStyle::Paragraph | BlockStyle::Heading(0) => String::new(),
        BlockStyle::Heading(n) => format!("{} ", "#".repeat(usize::from(n.min(6)))),
    };
    let cursor = if caret >= ls + old {
        caret - old + new_prefix.len()
    } else {
        ls + new_prefix.len()
    };
    HostEdit {
        range: ls..ls + old,
        replacement: new_prefix,
        cursor,
    }
}

/// Toggles a prefix on every selected line: on when any line lacks it, off
/// when all have it. Blank lines are skipped unless every line is blank.
/// Indentation stays. Another list marker is replaced. With a caret only,
/// the caret keeps its place in the text; with a range, the caret lands at
/// the end of the edited lines.
pub(crate) fn toggle_prefix(text: &str, sel: Selection, prefix: LinePrefix) -> HostEdit {
    let (first, last_end) = selected_lines(text, sel);
    let lines: Vec<&str> = text[first..last_end].split('\n').collect();
    let any_text = lines.iter().any(|l| !l.trim().is_empty());
    let eligible = |l: &str| !any_text || !l.trim().is_empty();
    let all_have = lines
        .iter()
        .filter(|l| eligible(l))
        .all(|l| has_prefix(split_indent(l).1, prefix));
    let caret = floor(text, sel.start);
    let collapsed = sel.start >= sel.end;

    let mut out = String::new();
    let mut number = 0;
    let mut offset = first;
    let mut cursor = None;
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let (indent, rest) = split_indent(line);
        let new_line = if !eligible(line) {
            (*line).to_owned()
        } else {
            number += 1;
            let have = has_prefix(rest, prefix);
            let old_len = marker_len(rest, prefix);
            let new_marker = if all_have {
                String::new()
            } else if have && prefix != LinePrefix::Numbered {
                rest[..old_len].to_owned()
            } else {
                match prefix {
                    LinePrefix::Bullet => "- ".to_owned(),
                    LinePrefix::Numbered => format!("{number}. "),
                    LinePrefix::Task => "- [ ] ".to_owned(),
                    LinePrefix::Quote => "> ".to_owned(),
                }
            };
            let strip = if all_have || have {
                old_len
            } else if prefix == LinePrefix::Quote {
                0
            } else {
                other_list(rest).map_or(0, |p| marker_len(rest, p))
            };
            if collapsed && caret >= offset && caret <= offset + line.len() {
                let content = offset + indent.len();
                cursor = Some(if caret >= content + strip {
                    caret - strip + new_marker.len() + (out.len() - (offset - first))
                } else {
                    content + new_marker.len() + (out.len() - (offset - first))
                });
            }
            format!("{indent}{new_marker}{}", &rest[strip..])
        };
        out.push_str(&new_line);
        offset += line.len() + 1;
    }
    let cursor = match cursor {
        Some(c) if collapsed => c,
        _ => first + out.len(),
    };
    HostEdit {
        range: first..last_end,
        replacement: out,
        cursor,
    }
}

/// Wraps the selected lines in a backtick fence, or, with no selection,
/// inserts an empty fence on its own lines with the caret inside it.
pub(crate) fn code_block(text: &str, sel: Selection) -> HostEdit {
    if sel.start < sel.end {
        let (first, last_end) = selected_lines(text, sel);
        let replacement = format!("```\n{}\n```", &text[first..last_end]);
        let cursor = first + replacement.len();
        return HostEdit {
            range: first..last_end,
            replacement,
            cursor,
        };
    }
    insert_block(text, sel.start, "```\n\n```", false, 4)
}

/// Inserts a 2 x 2 table (a header row and one body row) on its own lines,
/// with a blank line on each side, and the caret in the first header cell.
pub(crate) fn insert_table(text: &str, sel: Selection) -> HostEdit {
    insert_block(text, sel.start, TABLE_TEMPLATE, true, 2)
}

/// Wraps the selection as `[selection](address)`, or with no selection
/// inserts `[address](address)`. The caret lands after the link.
pub(crate) fn insert_link(text: &str, sel: Selection, address: &str) -> HostEdit {
    let start = floor(text, sel.start);
    let end = floor(text, sel.end).max(start);
    let label = if start < end {
        &text[start..end]
    } else {
        address
    };
    let replacement = format!("[{label}]({address})");
    HostEdit {
        range: start..end,
        cursor: start + replacement.len(),
        replacement,
    }
}

/// The caret line's block style. A heading deeper than 3 reads as its own
/// level, though the Style menu never sets one.
pub(crate) fn block_style_at(text: &str, cursor: usize) -> BlockStyle {
    let cursor = floor(text, cursor);
    let line = &text[line_start(text, cursor)..line_end(text, cursor)];
    let hashes = heading_prefix_len(line);
    if hashes == 0 {
        return BlockStyle::Paragraph;
    }
    let level = line.bytes().take_while(|b| *b == b'#').count();
    BlockStyle::Heading(u8::try_from(level).unwrap_or(6))
}

/// Whether the caret sits on a row of a pipe table: a run of `|` lines with
/// a `| --- |` separator row. Like the editor's own check, the separator
/// row itself does not count.
pub(crate) fn table_at(text: &str, cursor: usize) -> bool {
    let cursor = floor(text, cursor);
    let ls = line_start(text, cursor);
    let le = line_end(text, cursor);
    let line = &text[ls..le];
    if !is_pipe_row(line) || is_separator_row(line) {
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

/// Marks the editor's toggle would unwrap for this selection. This follows
/// zorite-editor 0.10.0's `toggle_wrap_pair` (`src/lib.rs:2393-2420`): an
/// empty selection is no-op; the selection is marked when it starts and
/// ends with the marker (with room for both), or when the marker sits just
/// before and just after it. Markers: `**`, `*`, `~~`, `` ` `` (`lib.rs:2433-2447`).
/// So a selected `**x**` reads as Bold and Italic, as the editor's toggles do.
pub(crate) fn active_marks(text: &str, sel: Selection) -> Vec<Mark> {
    let start = floor(text, sel.start);
    let end = floor(text, sel.end);
    if start >= end {
        return Vec::new();
    }
    let selected = &text[start..end];
    let unwraps = |marker: &str| {
        (selected.len() >= marker.len() * 2
            && selected.starts_with(marker)
            && selected.ends_with(marker))
            || (text[..start].ends_with(marker) && text[end..].starts_with(marker))
    };
    [
        (Mark::Bold, "**"),
        (Mark::Italic, "*"),
        (Mark::Strike, "~~"),
        (Mark::Code, "`"),
    ]
    .into_iter()
    .filter(|(_, marker)| unwraps(marker))
    .map(|(mark, _)| mark)
    .collect()
}

fn floor(text: &str, mut pos: usize) -> usize {
    pos = pos.min(text.len());
    while !text.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
}

fn line_start(text: &str, pos: usize) -> usize {
    text[..pos].rfind('\n').map_or(0, |i| i + 1)
}

fn line_end(text: &str, pos: usize) -> usize {
    text[pos..].find('\n').map_or(text.len(), |i| pos + i)
}

/// The first selected line's start and the last one's end. A range ending
/// at the start of a line does not take that line.
fn selected_lines(text: &str, sel: Selection) -> (usize, usize) {
    let start = floor(text, sel.start);
    let end = floor(text, sel.end).max(start);
    let last = if end > start && text[..end].ends_with('\n') {
        end - 1
    } else {
        end
    };
    (line_start(text, start), line_end(text, last))
}

fn split_indent(line: &str) -> (&str, &str) {
    let n = line.len() - line.trim_start_matches([' ', '\t']).len();
    line.split_at(n)
}

/// Length of a leading `#`-run plus its space (1-6 hashes), else 0.
fn heading_prefix_len(line: &str) -> usize {
    let hashes = line.bytes().take_while(|b| *b == b'#').count();
    if (1..=6).contains(&hashes) {
        let after = &line[hashes..];
        if after.is_empty() {
            return hashes;
        }
        if after.starts_with(' ') {
            return hashes + 1;
        }
    }
    0
}

fn task_marker_len(rest: &str) -> Option<usize> {
    let b = rest.as_bytes();
    let ok = b.len() >= 6
        && matches!(b[0], b'-' | b'*' | b'+')
        && b[1] == b' '
        && b[2] == b'['
        && matches!(b[3], b' ' | b'x' | b'X')
        && b[4] == b']'
        && b[5] == b' ';
    ok.then_some(6)
}

fn bullet_marker_len(rest: &str) -> Option<usize> {
    let b = rest.as_bytes();
    (b.len() >= 2 && matches!(b[0], b'-' | b'*' | b'+') && b[1] == b' ').then_some(2)
}

fn numbered_marker_len(rest: &str) -> Option<usize> {
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    (digits > 0 && rest[digits..].starts_with(". ")).then_some(digits + 2)
}

fn has_prefix(rest: &str, prefix: LinePrefix) -> bool {
    marker_len_opt(rest, prefix).is_some()
}

fn marker_len_opt(rest: &str, prefix: LinePrefix) -> Option<usize> {
    match prefix {
        LinePrefix::Task => task_marker_len(rest),
        LinePrefix::Bullet => {
            if task_marker_len(rest).is_some() {
                None
            } else {
                bullet_marker_len(rest)
            }
        }
        LinePrefix::Numbered => numbered_marker_len(rest),
        LinePrefix::Quote => rest.starts_with("> ").then_some(2),
    }
}

fn marker_len(rest: &str, prefix: LinePrefix) -> usize {
    marker_len_opt(rest, prefix).unwrap_or(0)
}

/// The list marker the line carries, if any.
fn other_list(rest: &str) -> Option<LinePrefix> {
    [LinePrefix::Task, LinePrefix::Bullet, LinePrefix::Numbered]
        .into_iter()
        .find(|p| has_prefix(rest, *p))
}

fn is_pipe_row(line: &str) -> bool {
    line.trim_start().starts_with('|')
}

fn is_separator_row(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('|') && t.contains('-') && t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

/// Puts `block` on its own lines at the caret. A blank caret line is
/// replaced; otherwise the block goes after the caret line. `blank_around`
/// keeps a blank line between the block and any text next to it. `inner` is
/// the caret's offset inside the block.
fn insert_block(
    text: &str,
    caret: usize,
    block: &str,
    blank_around: bool,
    inner: usize,
) -> HostEdit {
    let caret = floor(text, caret);
    let ls = line_start(text, caret);
    let le = line_end(text, caret);
    let next_has_text = le < text.len() && {
        let ne = line_end(text, le + 1);
        !text[le + 1..ne].trim().is_empty()
    };
    let (range, mut lead) = if text[ls..le].trim().is_empty() {
        let prev_has_text = ls > 0 && !text[line_start(text, ls - 1)..ls - 1].trim().is_empty();
        (
            ls..le,
            if blank_around && prev_has_text {
                "\n"
            } else {
                ""
            },
        )
    } else {
        (le..le, if blank_around { "\n\n" } else { "\n" })
    };
    if range.start == range.end && range.start == 0 {
        lead = "";
    }
    let trail = if blank_around && next_has_text {
        "\n"
    } else {
        ""
    };
    HostEdit {
        cursor: range.start + lead.len() + inner,
        replacement: format!("{lead}{block}{trail}"),
        range,
    }
}

#[cfg(test)]
#[path = "markdown_edit_tests.rs"]
mod tests;
