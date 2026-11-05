//! Handoff 2g's line tints: a side's lines against Before's.

use super::{Mark, marked};

#[test]
fn lines_are_marked_against_before_by_place() {
    assert_eq!(
        marked("One\nTwo\n", "One\nZwei\nThree\n"),
        [
            (Mark::Same, "One"),
            (Mark::Changed, "Zwei"),
            (Mark::Added, "Three"),
        ]
    );
    assert_eq!(marked("", "New"), [(Mark::Added, "New")]);
    assert_eq!(marked("Gone\n", ""), []);
    assert_eq!(
        [Mark::Same, Mark::Changed, Mark::Added].map(Mark::gutter),
        [' ', '~', '+']
    );
}
