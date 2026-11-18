//! The Needs attention popover and a row's menu in `polling_privacy`
//! (Wave 04 Cycle 03b, Task 12; E-14, P28): each is opened as a person opens
//! it, with the sweep run while it is open, so what it draws and what it
//! builds is read against every sentinel. The popover lists the rows that
//! need attention, so the real Check nows of the second and third
//! repositories (`check_the_others`) give it rows that name them, whose
//! folder and remote sentinels it must not show.

use gpui_kit::TestAppContext;

use super::{Intent, Lane, PollingWindow, RequestState, RowKey, Seen, TITLE, refresh, to_row};
use crate::desktop::view::privacy_layers_tests::{is_open, open_popover};

/// How many Check nows have been asked, and whether every request has ended.
fn checks(window: &PollingWindow, cx: &mut TestAppContext) -> (usize, bool) {
    let asked = window
        .shell
        .requests(cx, |request| {
            matches!(request.intent, Intent::PollOnce { .. })
        })
        .len();
    let ended = window.shell.operations(cx).read_with(cx, |operations, _| {
        operations.requests().iter().all(|request| {
            !matches!(
                request.state,
                RequestState::Queued | RequestState::Running { .. }
            )
        })
    });
    (asked, ended)
}

/// A Check now of each of `roots`, which the real library answers (neither
/// has a transport), waited for. Each leaves a failed request, which is a
/// row that needs attention and names its repository.
pub(super) async fn check_the_others(
    window: &PollingWindow,
    roots: &[&std::path::Path],
    cx: &mut TestAppContext,
) {
    let (before, _) = checks(window, cx);
    for root in roots {
        window.services.worker().submit(
            Intent::PollOnce {
                root: root.to_path_buf(),
            },
            Lane::User,
            None,
            None,
        );
    }
    window
        .shell
        .until_drawn(cx, "the other repositories' Check nows", |_, cx| {
            checks(window, cx) == (before + roots.len(), true)
        })
        .await;
    refresh(&window.shell, cx);
}

/// The popover is opened, swept and closed; then, when `menu_of` is given,
/// that row's menu is opened below the row by the keyboard, swept and closed.
pub(super) fn visit(
    window: &PollingWindow,
    seen: &mut Seen,
    menu_of: Option<&RowKey>,
    cx: &mut TestAppContext,
) {
    let shell = &window.shell;
    open_popover(shell, cx);
    let listed = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.attention_popover_rows());
    assert!(!listed.is_empty(), "the popover lists rows");
    assert!(
        listed.iter().all(|row| row.row.attention),
        "and only rows that need attention"
    );
    seen.sweep(window, cx);
    shell.keys("escape", cx);
    assert!(!is_open(shell, cx), "Escape closes the popover");

    let Some(key) = menu_of else {
        return;
    };
    refresh(shell, cx);
    to_row(shell, key, cx);
    shell.keys("shift-f10", cx);
    shell.frame(cx, |_, _| ());
    let open = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.activity_menu_key());
    assert_eq!(open.as_ref(), Some(key), "the row's menu is open");
    seen.sweep(window, cx);
    shell.keys("escape", cx);
    shell.frame(cx, |_, _| ());
    assert!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.activity_menu_key())
            .is_none(),
        "Escape closes the menu"
    );
}

/// The positive controls of the sweeps of Activity and its layers: each of
/// them read text that holds what it is swept for. A sweep that read nothing
/// would pass.
pub(super) fn controls(seen: &Seen, window: &PollingWindow, cx: &mut TestAppContext) {
    // The title is drawn where decision 21 allows it, in each place that
    // draws it, and each is noted apart, so that the model's rows do not
    // stand for the drawn ones. A body, in the same items, is in the data
    // the desktop read (above) and in none of these.
    for (at, what) in [
        ("activity-row", "a drawn Activity row"),
        ("model-row", "a row of the model"),
        ("row-menu", "the drawn row menu"),
        ("menu-item", "the row menu's items"),
    ] {
        assert!(
            seen.allowed.contains(&(at, TITLE)),
            "{TITLE} is in {what}, where decision 21 puts it"
        );
    }
    // The drawn rows were read, with the title taken out of them, so that
    // the scan has read text a title was in.
    for surface in ["activity row", "row menu"] {
        assert!(
            seen.read_at(surface, "{title}"),
            "{surface} read a row that names an item"
        );
    }
    // A row names the second repository by its remote's name only, where
    // the design puts it (S4, R-48): drawn and in the model, and it is the
    // second repository's own row. Its address, the path of which and the
    // user-info of which are on disk (above), is in none.
    let remote = "origin isn't an SSH shared copy";
    for surface in ["activity row", "model row"] {
        assert!(seen.read_at(surface, remote), "{surface} names the remote");
    }
    assert!(
        crate::desktop::view::activity_support::rows(&window.shell, cx)
            .iter()
            .any(|row| row.row.repo.as_deref() == Some("b") && row.row.title.contains(remote)),
        "the row that names the remote is the second repository's"
    );
    // The popover listed rows that name the second and the third
    // repository, the third lying in the sentinel's folder: drawn, in its
    // tooltips and in its descriptions.
    for repo in ["b", "c"] {
        let words = format!("Couldn't check {repo} for updates");
        for surface in ["popover row", "popover tooltip", "popover description"] {
            assert!(seen.read_at(surface, &words), "{surface} names {repo}");
        }
    }
}
