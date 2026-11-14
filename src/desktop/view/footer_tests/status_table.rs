//! The pure half of `status_items_say_and_choose` (Cycle 03a, Part G): what
//! the footer's status items say, choose and hand focus to, each function
//! over its own table. No window, so no tooltip is needed: the names are the
//! tooltips' text.

use time::{Duration, OffsetDateTime};

use crate::desktop::{
    model::{
        layout::{Layout, PanelKind, Side},
        stamp::RepoKey,
    },
    view::footer::{
        Control, Face, FooterStatus, UPDATE_TOOLTIP, focus_after_hiding, next_to_share, plan,
        to_share_name, to_share_text, update_text, update_tooltip,
    },
};

use PanelKind::Tickets;

/// `update_text`, `to_share_text`, `to_share_name`, `next_to_share`, `plan`
/// and `focus_after_hiding`, each over its table.
pub(super) fn table() {
    // Update check: checking wins over a time, no time and no check hides it
    // (P15, P37).
    let now = OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap();
    let ago = |by: Duration| Some(now - by);
    for (checking, latest, expected) in [
        (true, None, Some("Checking for updates\u{2026}")),
        (
            true,
            ago(Duration::minutes(3)),
            Some("Checking for updates\u{2026}"),
        ),
        (false, None, None),
        (
            false,
            ago(Duration::ZERO),
            Some("Checked for updates just now"),
        ),
        (
            false,
            ago(Duration::minutes(1)),
            Some("Checked for updates 1 minute ago"),
        ),
        (
            false,
            ago(Duration::minutes(2)),
            Some("Checked for updates 2 minutes ago"),
        ),
        (
            false,
            ago(Duration::hours(1)),
            Some("Checked for updates 1 hour ago"),
        ),
        (
            false,
            ago(Duration::days(3)),
            Some("Checked for updates 3 days ago"),
        ),
        // A check dated after now (a clock stepped back) reads as just now.
        (
            false,
            ago(-Duration::minutes(5)),
            Some("Checked for updates just now"),
        ),
    ] {
        assert_eq!(
            update_text(checking, latest, now).as_deref(),
            expected,
            "checking {checking}, {latest:?}"
        );
    }
    assert_eq!(
        UPDATE_TOOLTIP,
        "Background check: fetch and fast-forward only. Never shares your changes."
    );

    assert_eq!(
        update_tooltip("Checked for updates just now"),
        format!("Checked for updates just now\n{UPDATE_TOOLTIP}")
    );

    // To share: the text, and the name, which says what a press chooses.
    assert_eq!(to_share_text(1), "1 to share");
    assert_eq!(to_share_text(12), "12 to share");
    for (n, next, only_chosen, expected) in [
        (3, "b", false, "3 to share. Select b."),
        (1, "a in sub", false, "1 to share. Select a in sub."),
        (2, "a", true, "2 to share in a."),
        (1, "a in sub", true, "1 to share in a in sub."),
    ] {
        assert_eq!(to_share_name(n, next, only_chosen), expected);
    }

    // `next_to_share`: none, one not chosen, one chosen, several with the
    // chosen among them, wrapping, a chosen one with no work, and a chosen
    // one the list does not give (the kept row), which is never the answer.
    let k = |name: &str| name.to_owned();
    let order = |counts: &[(&str, usize)]| -> Vec<(RepoKey, usize)> {
        counts.iter().map(|(name, n)| (k(name), *n)).collect()
    };
    for (counts, chosen, expected) in [
        (vec![], None, None),
        (vec![], Some("a"), None),
        (vec![("a", 0), ("b", 0)], None, None),
        (vec![("a", 0), ("b", 0)], Some("a"), None),
        (vec![("a", 0), ("b", 2)], None, Some("b")),
        (vec![("a", 0), ("b", 2)], Some("a"), Some("b")),
        (vec![("a", 0), ("b", 2)], Some("b"), None),
        (vec![("a", 1), ("b", 2), ("c", 0)], Some("b"), Some("a")),
        (vec![("a", 1), ("b", 2), ("c", 3)], Some("a"), Some("b")),
        (vec![("a", 1), ("b", 2), ("c", 3)], Some("b"), Some("c")),
        (vec![("a", 1), ("b", 2), ("c", 3)], Some("c"), Some("a")),
        (vec![("a", 1), ("b", 0), ("c", 3)], Some("b"), Some("c")),
        (vec![("a", 1), ("b", 0), ("c", 3)], None, Some("a")),
        (vec![("a", 1), ("b", 2)], Some("kept"), Some("a")),
        (vec![("a", 0), ("b", 0)], Some("kept"), None),
    ] {
        let chosen = chosen.map(k);
        assert_eq!(
            next_to_share(&order(&counts), chosen.as_ref()),
            expected.map(k),
            "{counts:?}, chosen {chosen:?}"
        );
    }

    // `plan`: the status items lead the right side's first group, in order,
    // hidden when `None` or 0; the left side never changes.
    let status = |update: Option<&str>, to_share, attention| FooterStatus {
        update: update.map(str::to_owned),
        to_share,
        attention,
    };
    let default = plan(&Layout::default(), &FooterStatus::default());
    let (dock, settings) = (
        vec![Control::Panel(Tickets), Control::Dock(Side::Right)],
        vec![Control::Settings],
    );
    use Control::{Attention, ToShare, UpdateCheck};
    for (given, first) in [
        (status(None, 0, 0), vec![]),
        (
            status(Some("Checking for updates\u{2026}"), 0, 0),
            vec![UpdateCheck],
        ),
        (status(None, 2, 0), vec![ToShare]),
        (status(None, 0, 1), vec![Attention]),
        (status(Some("x"), 2, 0), vec![UpdateCheck, ToShare]),
        (status(Some("x"), 0, 4), vec![UpdateCheck, Attention]),
        (status(None, 2, 4), vec![ToShare, Attention]),
        (
            status(Some("x"), 2, 4),
            vec![UpdateCheck, ToShare, Attention],
        ),
    ] {
        let planned = plan(&Layout::default(), &given);
        let mut right = vec![dock.clone(), settings.clone()];
        if !first.is_empty() {
            right.insert(0, first);
        }
        assert_eq!(planned.right, right, "{given:?}");
        assert_eq!(planned.left, default.left, "{given:?}");
    }
    // With no panel in the right dock, the first group is followed by
    // Settings alone, and a layout with no status keeps just Settings.
    let mut bare = Layout::default();
    bare.move_to(Tickets, Side::Left);
    assert_eq!(
        plan(&bare, &status(Some("x"), 1, 0)).right,
        [vec![UpdateCheck, ToShare], settings.clone()]
    );
    assert_eq!(
        plan(&bare, &FooterStatus::default()).right,
        std::slice::from_ref(&settings)
    );

    // `focus_after_hiding`: the next shown control to the right (R-38).
    let shown = |given: FooterStatus| plan(&Layout::default(), &given);
    let bare_plan = |given: FooterStatus| plan(&bare, &given);
    for (hidden, planned, expected) in [
        (ToShare, shown(status(None, 0, 2)), Attention),
        (ToShare, shown(status(Some("x"), 0, 2)), Attention),
        (
            ToShare,
            shown(status(Some("x"), 0, 0)),
            Control::Panel(Tickets),
        ),
        (ToShare, bare_plan(status(None, 0, 0)), Control::Settings),
        (UpdateCheck, shown(status(None, 3, 0)), ToShare),
        (UpdateCheck, shown(status(None, 0, 1)), Attention),
        (
            UpdateCheck,
            shown(status(None, 0, 0)),
            Control::Panel(Tickets),
        ),
        (
            UpdateCheck,
            bare_plan(status(None, 0, 0)),
            Control::Settings,
        ),
    ] {
        assert_eq!(
            focus_after_hiding(hidden, &planned),
            Some(expected),
            "{hidden:?} hid"
        );
    }
    // Needs attention hides to the right too (P26, R-37): the right dock
    // group's first control, else Settings, whichever status items still
    // show to its left.
    for (planned, expected) in [
        (shown(status(None, 0, 0)), Control::Panel(Tickets)),
        (shown(status(Some("x"), 2, 0)), Control::Panel(Tickets)),
        (bare_plan(status(None, 0, 0)), Control::Settings),
        (bare_plan(status(Some("x"), 2, 0)), Control::Settings),
    ] {
        assert_eq!(
            focus_after_hiding(Attention, &planned),
            Some(expected),
            "Needs attention hid: {planned:?}"
        );
    }
    // Every other control stays where it is: the rule is for the three.
    assert_eq!(focus_after_hiding(Control::Settings, &default), None);
    assert_eq!(focus_after_hiding(Control::Panel(Tickets), &default), None);

    // Needs attention says what it is for (E1, fact 43).
    let face = |control| Face {
        control,
        name: "name".to_owned(),
        text: None,
        on: None,
    };
    assert_eq!(
        face(Attention).tooltip(),
        "Things that need you. Your work is safe until you act."
    );
    assert_eq!(face(Control::Settings).tooltip(), "name");
}
