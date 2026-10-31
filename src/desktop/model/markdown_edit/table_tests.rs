use super::*;

/// A paragraph and a pipe table of two columns.
const T: &str = "Intro\n\n| A | B |\n| --- | --- |\n| 1 | 2 |\n";

/// `text` with `edit` put in.
fn applied(text: &str, edit: &HostEdit) -> String {
    let mut out = text.to_owned();
    out.replace_range(edit.range.clone(), &edit.replacement);
    out
}

/// `text` aligned at `cursor`, and the caret after, or `None` for no edit.
fn aligned(text: &str, cursor: usize, align: ColumnAlign) -> Option<(String, usize)> {
    align_column(text, cursor, align).map(|edit| (applied(text, &edit), edit.cursor))
}

/// Just the text of `aligned`.
fn aligned_text(text: &str, cursor: usize, align: ColumnAlign) -> Option<String> {
    aligned(text, cursor, align).map(|(text, _)| text)
}

fn at(text: &str, needle: &str) -> usize {
    text.find(needle)
        .unwrap_or_else(|| panic!("{needle:?} is in the text"))
}

/// As `set_caret_table_align` writes it (`tables.rs:479-505`): every column
/// as ` :-- `, ` :-: ` or ` --: `, the caret's set, the others kept. A
/// caret after the separator moves with its new length; one before it
/// stays.
#[test]
fn align_column_writes_the_separator_as_the_editor_does() {
    let one = at(T, "1");
    assert_eq!(
        aligned(T, one, ColumnAlign::Centre),
        Some((
            "Intro\n\n| A | B |\n| :-: | :-- |\n| 1 | 2 |\n".to_owned(),
            one
        ))
    );
    assert_eq!(
        aligned_text(T, at(T, "B"), ColumnAlign::Right).as_deref(),
        Some("Intro\n\n| A | B |\n| :-- | --: |\n| 1 | 2 |\n"),
        "from the header row"
    );
    assert_eq!(
        aligned_text(T, at(T, "A"), ColumnAlign::Right).as_deref(),
        Some("Intro\n\n| A | B |\n| --: | :-- |\n| 1 | 2 |\n")
    );

    let set = "| A | B |\n| :-: | --- |\n| 1 | 2 |\n";
    assert_eq!(
        aligned_text(set, at(set, "2"), ColumnAlign::Right).as_deref(),
        Some("| A | B |\n| :-: | --: |\n| 1 | 2 |\n"),
        "the other column keeps its alignment"
    );

    let short = "| A | B |\n|---|---|\n| 1 | 2 |\n";
    let one = at(short, "1");
    assert_eq!(
        aligned(short, one, ColumnAlign::Left),
        Some(("| A | B |\n| :-- | :-- |\n| 1 | 2 |\n".to_owned(), one + 4)),
        "a body row's caret moves with the separator"
    );
    let a = at(short, "A");
    assert_eq!(
        aligned(short, a, ColumnAlign::Left).map(|(_, cursor)| cursor),
        Some(a),
        "the header's caret stays"
    );
}

/// Past a row's last cell, before its closing pipe or at its end, the
/// caret aligns the last column, as the editor's `table_cell_at` clamps
/// (`tables.rs:1226-1231`). A row with no closing pipe has its cells
/// between its first and last pipe, as the editor reads it, and a caret
/// past them takes the last of those.
#[test]
fn align_column_takes_the_last_cell_past_the_rows_end() {
    let last_centred = "Intro\n\n| A | B |\n| :-- | :-: |\n| 1 | 2 |\n";
    for row in ["| A | B |", "| 1 | 2 |"] {
        let end = at(T, row) + row.len();
        for cursor in [end - 1, end] {
            assert_eq!(
                aligned_text(T, cursor, ColumnAlign::Centre).as_deref(),
                Some(last_centred),
                "{row:?} at {cursor}"
            );
        }
    }

    let open = "| A | B\n| --- | --- |\n| 1 | 2\n";
    assert_eq!(
        aligned_text(open, open.len() - 1, ColumnAlign::Right).as_deref(),
        Some("| A | B\n| --: | :-- |\n| 1 | 2\n"),
        "no closing pipe"
    );
}

/// Tables are found as the editor finds them (`markdown_syntax.rs:2075-2120`):
/// a row jammed under a table that is followed by a separator starts the
/// next table, and an escaped pipe is cell text. The separator row itself,
/// a paragraph, a table inside a code fence and pipes with no separator
/// take no edit.
#[test]
fn align_column_finds_tables_as_the_editor_does() {
    let two = "| A |\n| --- |\n| 1 |\n| B |\n| --- |\n| 2 |\n";
    assert_eq!(
        aligned_text(two, at(two, "1"), ColumnAlign::Right).as_deref(),
        Some("| A |\n| --: |\n| 1 |\n| B |\n| --- |\n| 2 |\n"),
        "the first table"
    );
    let second = "| A |\n| --- |\n| 1 |\n| B |\n| --: |\n| 2 |\n";
    for needle in ["B", "2"] {
        assert_eq!(
            aligned_text(two, at(two, needle), ColumnAlign::Right).as_deref(),
            Some(second),
            "the second table, from {needle:?}"
        );
    }

    let escaped = "| a \\| b | c |\n| --- | --- |\n| 1 | 2 |\n";
    assert_eq!(
        aligned_text(escaped, at(escaped, "c"), ColumnAlign::Right).as_deref(),
        Some("| a \\| b | c |\n| :-- | --: |\n| 1 | 2 |\n"),
        "an escaped pipe"
    );

    assert!(
        align_column(T, at(T, "| ---") + 3, ColumnAlign::Left).is_none(),
        "the separator row"
    );
    assert!(
        align_column(T, 0, ColumnAlign::Left).is_none(),
        "a paragraph"
    );
    let fenced = "```\n| A | B |\n| --- | --- |\n| 1 | 2 |\n```\n";
    assert!(
        align_column(fenced, at(fenced, "1"), ColumnAlign::Left).is_none(),
        "inside a code fence, where the editor draws no table"
    );
    assert!(align_column("| a | b |\nno separator\n", 2, ColumnAlign::Left).is_none());
    assert!(align_column("", 0, ColumnAlign::Left).is_none());
}
