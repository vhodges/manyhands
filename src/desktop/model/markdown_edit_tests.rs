use super::*;

fn sel(start: usize, end: usize) -> Selection {
    Selection { start, end }
}

fn caret(at: usize) -> Selection {
    sel(at, at)
}

fn apply(text: &str, edit: &HostEdit) -> String {
    let mut out = text.to_owned();
    out.replace_range(edit.range.clone(), &edit.replacement);
    out
}

/// Applies the edit and marks the caret with `|`.
fn shown(text: &str, edit: &HostEdit) -> String {
    let mut out = apply(text, edit);
    out.insert(edit.cursor, '|');
    out
}

#[test]
fn headings_are_set_replaced_and_cleared_on_the_caret_line() {
    let text = "intro\nTitle\nbody";
    let e = set_block_style(text, caret(8), BlockStyle::Heading(2));
    assert_eq!(shown(text, &e), "intro\n## Ti|tle\nbody");

    let text = "# Title";
    let e = set_block_style(text, caret(3), BlockStyle::Heading(3));
    assert_eq!(shown(text, &e), "### T|itle");

    let text = "### Title\nnext";
    let e = set_block_style(text, caret(6), BlockStyle::Paragraph);
    assert_eq!(shown(text, &e), "Ti|tle\nnext");
    assert_eq!(apply(text, &e), "Title\nnext");

    // The caret inside the prefix lands after the new prefix.
    let e = set_block_style("# Title", caret(1), BlockStyle::Heading(2));
    assert_eq!(shown("# Title", &e), "## |Title");

    // An empty text and a #-less hash word.
    let e = set_block_style("", caret(0), BlockStyle::Heading(1));
    assert_eq!(shown("", &e), "# |");
    let e = set_block_style("#tag", caret(0), BlockStyle::Heading(1));
    assert_eq!(apply("#tag", &e), "# #tag");
}

#[test]
fn each_prefix_toggles_on_every_selected_line() {
    let text = "one\ntwo\nthree";
    let on = toggle_prefix(text, sel(0, 7), LinePrefix::Bullet);
    assert_eq!(apply(text, &on), "- one\n- two\nthree");
    assert_eq!(on.cursor, 11);

    // Any line lacking the prefix turns it on for the lacking lines.
    let mixed = "- one\ntwo";
    let e = toggle_prefix(mixed, sel(0, 9), LinePrefix::Bullet);
    assert_eq!(apply(mixed, &e), "- one\n- two");

    // All having it turns it off.
    let all = "- one\n- two";
    let e = toggle_prefix(all, sel(0, 11), LinePrefix::Bullet);
    assert_eq!(apply(all, &e), "one\ntwo");
    assert_eq!(e.cursor, 7);

    // A caret keeps its place in the text.
    let e = toggle_prefix("one\ntwo", caret(5), LinePrefix::Quote);
    assert_eq!(shown("one\ntwo", &e), "one\n> t|wo");
    let e = toggle_prefix("> one", caret(3), LinePrefix::Quote);
    assert_eq!(shown("> one", &e), "o|ne");
    let e = toggle_prefix("> one", caret(1), LinePrefix::Quote);
    assert_eq!(shown("> one", &e), "|one");

    // Checklist, indentation, other list markers, blank lines.
    let e = toggle_prefix("  item", caret(2), LinePrefix::Task);
    assert_eq!(shown("  item", &e), "  - [ ] |item");
    let t = "- [x] done\n1. num";
    let e = toggle_prefix(t, sel(0, t.len()), LinePrefix::Bullet);
    assert_eq!(apply(t, &e), "- done\n- num");
    let t = "- one\n- [ ] two";
    let e = toggle_prefix(t, sel(0, t.len()), LinePrefix::Task);
    assert_eq!(apply(t, &e), "- [ ] one\n- [ ] two");
    let t = "a\n\nb";
    let e = toggle_prefix(t, sel(0, 4), LinePrefix::Bullet);
    assert_eq!(apply(t, &e), "- a\n\n- b");
    let e = toggle_prefix("", caret(0), LinePrefix::Bullet);
    assert_eq!(shown("", &e), "- |");

    // A quote line is not a list line.
    let e = toggle_prefix("> q", caret(0), LinePrefix::Bullet);
    assert_eq!(apply("> q", &e), "- > q");

    // A range ending at a line start leaves that line out.
    let t = "a\nb\nc";
    let e = toggle_prefix(t, sel(0, 2), LinePrefix::Quote);
    assert_eq!(apply(t, &e), "> a\nb\nc");
}

#[test]
fn a_numbered_list_counts_from_one() {
    let t = "a\nb\nc";
    let e = toggle_prefix(t, sel(2, 5), LinePrefix::Numbered);
    assert_eq!(apply(t, &e), "a\n1. b\n2. c");
    let t = "a\nb\nc";
    let e = toggle_prefix(t, sel(0, 5), LinePrefix::Numbered);
    assert_eq!(apply(t, &e), "1. a\n2. b\n3. c");
    assert_eq!(e.cursor, 14);
    // Renumbers a partly numbered run, and turns all-numbered off.
    let t = "7. a\nb";
    let e = toggle_prefix(t, sel(0, 6), LinePrefix::Numbered);
    assert_eq!(apply(t, &e), "1. a\n2. b");
    let t = "1. a\n2. b";
    let e = toggle_prefix(t, sel(0, 9), LinePrefix::Numbered);
    assert_eq!(apply(t, &e), "a\nb");
}

#[test]
fn a_code_block_wraps_the_selected_lines_or_inserts_an_empty_fence() {
    let t = "x\nfn a()\nlet b\ny";
    let e = code_block(t, sel(3, 12));
    assert_eq!(apply(t, &e), "x\n```\nfn a()\nlet b\n```\ny");
    assert_eq!(e.cursor, 2 + "```\nfn a()\nlet b\n```".len());

    let e = code_block("", caret(0));
    assert_eq!(shown("", &e), "```\n|\n```");

    let t = "para\n\nrest";
    let e = code_block(t, caret(5));
    assert_eq!(shown(t, &e), "para\n```\n|\n```\nrest");

    let t = "para";
    let e = code_block(t, caret(2));
    assert_eq!(shown(t, &e), "para\n```\n|\n```");
}

#[test]
fn a_table_template_is_inserted_on_its_own_lines() {
    let tpl = "| Column 1 | Column 2 |\n| --- | --- |\n|  |  |";
    let e = insert_table("", caret(0));
    assert_eq!(apply("", &e), tpl);
    assert_eq!(e.cursor, 2);

    let t = "para\nnext";
    let e = insert_table(t, caret(2));
    assert_eq!(apply(t, &e), format!("para\n\n{tpl}\n\nnext"));
    assert_eq!(e.cursor, 6 + 2);

    let t = "para\n\nrest";
    let e = insert_table(t, caret(5));
    assert_eq!(apply(t, &e), format!("para\n\n{tpl}\n\nrest"));
    assert!(table_at(&apply(t, &e), e.cursor));

    let t = "para\n";
    let e = insert_table(t, caret(5));
    assert_eq!(apply(t, &e), format!("para\n\n{tpl}"));
}

#[test]
fn a_link_wraps_the_selection_or_uses_the_address_as_its_text() {
    let t = "see docs now";
    let e = insert_link(t, sel(4, 8), "https://x.test");
    assert_eq!(apply(t, &e), "see [docs](https://x.test) now");
    assert_eq!(e.cursor, 4 + "[docs](https://x.test)".len());

    let e = insert_link(t, caret(4), "https://x.test");
    assert_eq!(apply(t, &e), "see [https://x.test](https://x.test)docs now");
    assert_eq!(e.cursor, 4 + "[https://x.test](https://x.test)".len());
}

#[test]
fn utf16_selections_convert_at_non_bmp_and_combining_text() {
    // "a" 1 unit, U+1F600 2 units / 4 bytes, "e" + U+0301 1+1 units / 1+2 bytes.
    let t = "a\u{1F600}e\u{301}z";
    assert_eq!(selection_from_utf16(t, 0..0), sel(0, 0));
    assert_eq!(selection_from_utf16(t, 1..3), sel(1, 5));
    assert_eq!(selection_from_utf16(t, 3..5), sel(5, 8));
    assert_eq!(selection_from_utf16(t, 4..5), sel(6, 8));
    // Inside the surrogate pair: start rounds down, end rounds up.
    assert_eq!(selection_from_utf16(t, 2..2), sel(1, 5));
    assert_eq!(selection_from_utf16(t, 2..3), sel(1, 5));
    assert_eq!(selection_from_utf16(t, 1..2), sel(1, 5));
    // Past the end clamps to the text length; reversed ranges are ordered.
    assert_eq!(selection_from_utf16(t, 3..99), sel(5, t.len()));
    assert_eq!(selection_from_utf16(t, 99..99), sel(t.len(), t.len()));
    let reversed = Range { start: 3, end: 1 };
    assert_eq!(selection_from_utf16(t, reversed), sel(1, 5));
    for u in 0..12 {
        let s = selection_from_utf16(t, u..u + 1);
        assert!(t.is_char_boundary(s.start) && t.is_char_boundary(s.end));
    }
}

#[test]
fn the_block_style_follows_the_caret_line() {
    let t = "# One\n## Two\n###### Six\n#tag\nplain";
    assert_eq!(block_style_at(t, 0), BlockStyle::Heading(1));
    assert_eq!(block_style_at(t, 8), BlockStyle::Heading(2));
    assert_eq!(block_style_at(t, 20), BlockStyle::Heading(6));
    assert_eq!(block_style_at(t, 25), BlockStyle::Paragraph);
    assert_eq!(block_style_at(t, t.len()), BlockStyle::Paragraph);
    assert_eq!(block_style_at("", 0), BlockStyle::Paragraph);
    assert_eq!(block_style_at("#### Deep", 3), BlockStyle::Heading(4));
    assert_eq!(block_style_at("####### seven", 0), BlockStyle::Paragraph);
}

#[test]
fn table_at_finds_the_caret_in_a_pipe_table_and_not_in_a_paragraph() {
    let t = "intro\n\n| a | b |\n| --- | --- |\n| 1 | 2 |\n\nafter | pipe";
    assert!(!table_at(t, 2));
    assert!(table_at(t, 9));
    assert!(!table_at(t, 20));
    assert!(table_at(t, 38));
    assert!(!table_at(t, t.len()));
    assert!(!table_at("| a | b |\nno separator", 3));
    assert!(!table_at("", 0));
}

#[test]
fn active_marks_follow_the_editors_unwrap_rule() {
    // zorite-editor 0.10.0 `toggle_wrap_pair`: lib.rs:2393-2420.
    let t = "x **bold** y";
    // Markers inside the selection.
    assert_eq!(active_marks(t, sel(2, 10)), vec![Mark::Bold, Mark::Italic]);
    // Markers just outside the selection.
    assert_eq!(active_marks(t, sel(4, 8)), vec![Mark::Bold, Mark::Italic]);
    // An empty selection is a no-op for the editor.
    assert_eq!(active_marks(t, caret(5)), vec![]);
    assert_eq!(active_marks("****", caret(2)), vec![]);
    // Plain text.
    assert_eq!(active_marks(t, sel(0, 1)), vec![]);
    assert_eq!(active_marks("*it*", sel(0, 4)), vec![Mark::Italic]);
    assert_eq!(active_marks("a *it* b", sel(3, 5)), vec![Mark::Italic]);
    assert_eq!(active_marks("~~s~~", sel(0, 5)), vec![Mark::Strike]);
    assert_eq!(active_marks("`c`", sel(1, 2)), vec![Mark::Code]);
    // The selection must hold both markers: a lone marker is not enough.
    assert_eq!(active_marks("**", sel(0, 2)), vec![Mark::Italic]);
    assert_eq!(active_marks("*", sel(0, 1)), vec![]);
    // Only one side outside is not marked.
    assert_eq!(active_marks("*it b", sel(1, 3)), vec![]);
    // Several marks at once, and a char boundary inside multibyte text.
    assert_eq!(
        active_marks("`~~s~~`", sel(1, 6)),
        vec![Mark::Strike, Mark::Code]
    );
    assert_eq!(active_marks("é", sel(1, 1)), vec![]);
}
