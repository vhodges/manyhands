//! Handoff 2g's line tints: a side's lines against Before's, by place.

use super::{Mark, marked};

/// `marked`'s lines as text slices, to compare with an expected list.
fn m(before: &str, side: &str) -> Vec<(Mark, String)> {
    marked(before, side)
}

fn l(expected: &[(Mark, &str)]) -> Vec<(Mark, String)> {
    expected
        .iter()
        .map(|(mark, text)| (*mark, (*text).to_owned()))
        .collect()
}

#[test]
fn lines_are_marked_against_before_by_place() {
    assert_eq!(
        m("One\nTwo\n", "One\nZwei\nThree\n"),
        l(&[
            (Mark::Same, "One"),
            (Mark::Changed, "Zwei"),
            (Mark::Added, "Three"),
        ])
    );
    assert_eq!(m("", "New"), l(&[(Mark::Added, "New")]));
    assert_eq!(
        [Mark::Same, Mark::Changed, Mark::Added, Mark::Removed].map(Mark::gutter),
        [' ', '~', '+', '-']
    );
}

/// M-d: a side emptied where Before had lines reads as removed, not blank;
/// an empty side of an empty Before has no lines.
#[test]
fn an_emptied_side_is_marked_removed() {
    assert_eq!(m("Gone\n", ""), l(&[(Mark::Removed, "(empty)")]));
    assert!(m("", "").is_empty());
}

/// M-d: a difference only in the trailing newline, or in CRLF line
/// endings, shows as a marked note, its lines otherwise unchanged.
#[test]
fn line_ending_differences_are_marked() {
    assert_eq!(
        m("One\n", "One"),
        l(&[(Mark::Same, "One"), (Mark::Changed, "(no newline at end)")])
    );
    assert_eq!(
        m("One", "One\n"),
        l(&[(Mark::Same, "One"), (Mark::Added, "(newline at end)")])
    );
    assert_eq!(
        m("One\nTwo\n", "One\r\nTwo\r\n"),
        l(&[
            (Mark::Same, "One"),
            (Mark::Same, "Two"),
            (Mark::Changed, "(CRLF line endings)"),
        ])
    );
    assert_eq!(
        m("One\r\n", "One\n"),
        l(&[(Mark::Same, "One"), (Mark::Changed, "(LF line endings)")])
    );
}
