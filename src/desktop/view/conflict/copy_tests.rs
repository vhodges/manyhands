//! P9: the copy of Close ticket and Resolve conflict (design Part C and
//! A1-A7), one table. Each row is a constant, or a formatting function
//! called with a value, against the words the design gives. The views
//! compare these constants; this is the one place the words are written
//! out.

use crate::desktop::{
    model::{
        activity::{
            copy::{
                CHECK_HOST, CHOSEN, EDITED, KEEP_MINE, OPEN_SHARING_AND_KEYS, RESUME,
                REVIEW_CHANGES, share_main,
            },
            lifecycle_lines::{ENDING_STOPPED, STILL_RUNNING, merged},
        },
        review,
    },
    view::{
        conflict::{COMMENT_DETAIL, DOCUMENT_DETAIL, LAST_SHARED_COPY, TICKET_DETAIL},
        lifecycle::{
            CLOSED, CLOSING, CONFIRM_HINT, COULDNT_CLOSE, COULDNT_MERGE, DONE, HIDE_HINT, MERGING,
            NOT_NOW, OPEN_REPOSITORY_DETAILS, READY_TO_START, REPAIR_LOCALLY, RUNNING_LEAD,
            approve_title, close_title, closed_by, closed_by_today,
        },
    },
};

#[test]
fn close_and_resolve_copy_uses_the_vocabulary() {
    let table: Vec<(&str, String, &str)> = vec![
        // Part C, item 1: one name for the way into Resolve conflict; Repair
        // locally keeps its own.
        (
            "REVIEW_CHANGES",
            REVIEW_CHANGES.into(),
            "Review changes\u{2026}",
        ),
        (
            "REPAIR_LOCALLY",
            REPAIR_LOCALLY.into(),
            "Repair locally\u{2026}",
        ),
        // Item 2: the ellipsis rule.
        (
            "OPEN_SHARING_AND_KEYS",
            OPEN_SHARING_AND_KEYS.into(),
            "Open Sharing & keys\u{2026}",
        ),
        ("CHECK_HOST", CHECK_HOST.into(), "Check host\u{2026}"),
        (
            "OPEN_REPOSITORY_DETAILS",
            OPEN_REPOSITORY_DETAILS.into(),
            "Open repository details\u{2026}",
        ),
        // Item 3: Keep mine beside Yours, and the state words.
        ("CHOSEN", CHOSEN.into(), "Chosen"),
        ("EDITED", EDITED.into(), "Edited"),
        // Item 4: the shared labels, each defined once. The item review's
        // Keep mine and the sheet's are the model's.
        ("KEEP_MINE", KEEP_MINE.into(), "Keep mine"),
        ("review::KEEP_MINE", review::KEEP_MINE.into(), "Keep mine"),
        ("RESUME", RESUME.into(), "Resume"),
        ("share_main", share_main("main"), "Share main"),
        ("share_main (trunk)", share_main("trunk"), "Share trunk"),
        // A1: the stage titles, Close ticket's and Approve and merge's.
        ("close Ask", close_title("AB-1"), "Close AB-1?"),
        (
            "approve Ask",
            approve_title("9ZQ4"),
            "Approve and merge 9ZQ4?",
        ),
        ("close Running", CLOSING.into(), "Closing\u{2026}"),
        ("merge Running", MERGING.into(), "Merging\u{2026}"),
        ("Waiting", STILL_RUNNING.into(), "Still running\u{2026}"),
        ("close Done", CLOSED.into(), "Ticket closed"),
        ("merge Done", merged("main"), "Merged into main"),
        (
            "close Failed",
            COULDNT_CLOSE.into(),
            "Couldn't finish closing",
        ),
        (
            "merge Failed",
            COULDNT_MERGE.into(),
            "Couldn't finish merging",
        ),
        ("Stopped", ENDING_STOPPED.into(), "Stopped at a safe point"),
        // A4: the Mark as closed note.
        (
            "closed_by_today",
            closed_by_today("Ada Example"),
            "Closed by Ada Example \u{b7} today",
        ),
        (
            "closed_by",
            closed_by("Ada Example"),
            "Closed by Ada Example",
        ),
        // A5: the running lead and the hints; A7: the dismiss labels.
        (
            "RUNNING_LEAD",
            RUNNING_LEAD.into(),
            "You can keep working. This continues in the background and shows up in Activity.",
        ),
        (
            "CONFIRM_HINT",
            CONFIRM_HINT.into(),
            "Enter to confirm \u{b7} Esc to cancel",
        ),
        ("HIDE_HINT", HIDE_HINT.into(), "Esc hides this window"),
        ("NOT_NOW", NOT_NOW.into(), "Not now"),
        ("DONE", DONE.into(), "Done"),
        // A6: the made-ready box's lead.
        (
            "READY_TO_START",
            READY_TO_START.into(),
            "Now ready to start",
        ),
        // B5: the cards' words.
        ("TICKET_DETAIL", TICKET_DETAIL.into(), "Ticket detail"),
        ("DOCUMENT_DETAIL", DOCUMENT_DETAIL.into(), "Document detail"),
        ("COMMENT_DETAIL", COMMENT_DETAIL.into(), "Comment detail"),
        (
            "LAST_SHARED_COPY",
            LAST_SHARED_COPY.into(),
            "last shared copy",
        ),
    ];
    for (name, got, want) in table {
        assert_eq!(got, want, "{name}");
    }
}
