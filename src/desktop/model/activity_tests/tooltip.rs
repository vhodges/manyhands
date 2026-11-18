//! B5's tooltip builder (P14): the full message, the time line, then the
//! row's detail.

use super::super::{
    copy::tooltip_lines,
    rows::{Derived, Kind, RowKey},
};

/// `tooltip_lines`: the message is the title, then ". " and the description
/// when there is one; the time line follows it, and the detail's lines come
/// last, unchanged and in order. A row with no description is its title
/// alone, and one with no detail has the two first lines only.
#[test]
fn tooltip_lines_are_message_time_and_detail() {
    let time = "2026-11-13 14:05 UTC";
    for (title, description, detail, expected) in [
        (
            "Couldn't share \u{201c}Plan\u{201d}",
            Some("The shared copy changed. Your work is safe on this computer."),
            vec!["Share \u{201c}Plan\u{201d} \u{b7} demo", "Failed"],
            vec![
                "Couldn't share \u{201c}Plan\u{201d}. The shared copy changed. Your work is safe on this computer.",
                time,
                "Share \u{201c}Plan\u{201d} \u{b7} demo",
                "Failed",
            ],
        ),
        (
            "Refreshed demo",
            None,
            vec!["Refresh demo \u{b7} demo", "Done"],
            vec!["Refreshed demo", time, "Refresh demo \u{b7} demo", "Done"],
        ),
        (
            "Recovered drafts: 2",
            Some("Review them or dismiss them."),
            Vec::new(),
            vec!["Recovered drafts: 2. Review them or dismiss them.", time],
        ),
        ("Nothing else", None, Vec::new(), vec!["Nothing else", time]),
    ] {
        let mut row = Derived::new(RowKey::Recovered, Kind::Info, title.to_owned());
        row.description = description.map(str::to_owned);
        row.detail = detail.into_iter().map(str::to_owned).collect();
        assert_eq!(tooltip_lines(&row, time), expected, "{title}");
    }
}
