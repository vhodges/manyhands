//! Cycle 03a Part I's repositories sweep, part of `polling_privacy`: the
//! texts the Repositories panel and the footer build, read through their own
//! functions from the live workspace's facts, for every repository.
//!
//! What a snapshot reads (the rows' labels and states, the toolbar's names,
//! the footer's items) is swept with the rest of what the window draws; this
//! adds what no snapshot reads: each row's tooltip lines and accessible
//! description (`repository_tooltip`), the labels of its menu
//! (`menu_items`), the labels of More's menu (`more_items`), the panel's and
//! the toolbar's constants, and the footer's texts (`update_text`,
//! `to_share_name` as each repository would give it).
//!
//! Item titles show in a tooltip by design (the unshared items are named by
//! title), so the sweep checks every sentinel but `TITLE`.

use std::{collections::BTreeSet, path::Path};

use gpui_kit::TestAppContext;

use super::{BODY, COMMENT, DRAFT, EDIT, PASSPHRASE, PASSWORD, PATH, PRIVATE, PollingWindow};
use crate::desktop::{
    model::repository_state::more_items,
    view::{
        harness::key,
        repositories::{
            ADD_OR_CREATE, CHECK_ALL, LIST_INCOMPLETE, LOADING_REPOSITORIES, MORE, NO_REPOSITORIES,
            NOT_UPDATED_BUSY, rows,
        },
    },
};

/// Every sentinel a repository's text must not hold: all but the title.
const SECRETS: [&str; 8] = [
    BODY, COMMENT, DRAFT, EDIT, PATH, PASSPHRASE, PRIVATE, PASSWORD,
];

/// What was read, by where: `(surface, text)`.
pub(super) type Texts = BTreeSet<(&'static str, String)>;

/// The panel's and the footer's texts as they stand now.
pub(super) fn read(window: &PollingWindow, cx: &mut TestAppContext) -> Texts {
    let mut read = Texts::new();
    let (rows, footer) = window.shell.workspace.read_with(cx, |workspace, cx| {
        (rows(workspace, cx), workspace.footer_texts(cx))
    });
    for row in &rows {
        for line in row.tooltip() {
            read.insert(("tooltip", line.clone()));
        }
        read.insert(("description", row.tooltip().join(" ")));
        for item in row.menu() {
            read.insert(("menu", item.label.clone()));
        }
        for chosen in [None, Some(&row.key)] {
            for item in more_items(chosen) {
                read.insert(("more menu", item.label));
            }
        }
    }
    for constant in [
        ADD_OR_CREATE,
        CHECK_ALL,
        MORE,
        LOADING_REPOSITORIES,
        NO_REPOSITORIES,
        NOT_UPDATED_BUSY,
        LIST_INCOMPLETE,
    ] {
        read.insert(("panel constant", constant.to_owned()));
    }
    read.extend(footer.into_iter().map(|text| ("footer text", text)));
    read
}

/// No text holds a secret sentinel.
pub(super) fn holds_none(read: &Texts) {
    for (surface, text) in read {
        for sentinel in SECRETS {
            assert!(
                !text.contains(sentinel),
                "{sentinel} reached the {surface}: {text}"
            );
        }
    }
}

/// How many texts `surface` read, and how many of them start with `start`.
pub(super) fn count(read: &Texts, surface: &str, start: &str) -> (usize, usize) {
    let of = || read.iter().filter(|(at, _)| *at == surface);
    (
        of().count(),
        of().filter(|(_, text)| text.starts_with(start)).count(),
    )
}

/// The sweep read real texts (P17): the tooltip lines of both repositories
/// with a "Shared copy:" line among them, and every other kind of text.
pub(super) fn was_read(read: &Texts) {
    let (lines, copies) = count(read, "tooltip", "Shared copy:");
    assert!(
        lines >= 4 && copies > 0,
        "{lines} tooltip lines, {copies} shared copies"
    );
    for surface in [
        "description",
        "menu",
        "more menu",
        "panel constant",
        "footer text",
    ] {
        assert!(count(read, surface, "").0 > 0, "{surface} was read");
    }
}

/// U-3: the repository `b` is selected for publication, and its remote's URL
/// then gained a password (a second actor's `git remote set-url`; the library
/// refuses to select such a URL, `InvalidRemote`). Such a remote is not
/// publication eligible, so `ssh_publication` does not schedule it: it has no
/// shared copy and is never polled.
pub(super) fn password_is_not_eligible(window: &PollingWindow, b: &Path, cx: &mut TestAppContext) {
    let service = window.shell.fixture.service();
    let remotes = service
        .list_remotes_redacted(&service.resolve_repository(b).unwrap())
        .unwrap()
        .items;
    assert!(
        remotes
            .iter()
            .any(|remote| remote.selected_for_publication && !remote.publication_eligible),
        "{remotes:?}"
    );
    let root = key(b);
    let copy = window
        .services
        .scheduler()
        .read_with(cx, |scheduler, _| scheduler.shared_copy(&root).cloned());
    // Controls: the scheduler did read `b`'s remotes (so `None` is its
    // answer, not an unread repository), and `polled_roots` is live: `a`,
    // whose poll the drive scripts, is in it.
    assert!(
        window.backend.remote_read_roots().contains(&root),
        "the scheduler read the remotes of {root}"
    );
    assert_eq!(copy, None, "an ineligible remote is no shared copy");
    let polled = window.backend.polled_roots();
    assert!(
        polled.contains(&window.roots[0]),
        "the first repository was polled: {polled:?}"
    );
    assert!(
        !polled.contains(&root),
        "an ineligible remote is not polled"
    );
}
