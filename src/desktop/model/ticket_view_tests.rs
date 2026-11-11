use manyhands::{
    repository::{ClosureFilter, ItemContextKind, ReadinessFilter},
    results::ProblemCode,
};
use time::{Duration, format_description::well_known::Rfc3339};

use super::{
    NO_SHORT_CODE, SegmentCounts, TicketView, age_label, derive, footnote, hint, sole_match,
    ticket_tooltip,
};
use crate::desktop::model::{
    item::{NO_BRANCH, context_kind_label},
    tickets::{DEPENDENCY_CYCLE, OPEN_DEPENDENCY, SortColumn, TicketQuery, TicketRow},
};

#[path = "ticket_view_tests/fixtures.rs"]
mod fixtures;
use fixtures::{
    ID_A, ID_D, REPO, Spec, a, context, id_of, library, nonconforming, now, query, sorted,
    tooltip_row,
};

struct Case {
    name: &'static str,
    /// Whether the read has landed: `false` is no rows at all.
    read: bool,
    query: TicketQuery,
    ids: &'static [&'static str],
    counts: Option<(usize, usize, usize)>,
    elsewhere: Option<usize>,
    footnote: Option<&'static str>,
    hint: Option<&'static str>,
    /// The closure the query has once built (R-26), when the case is
    /// about that.
    closure: Option<ClosureFilter>,
}

fn case(name: &'static str, query: TicketQuery, ids: &'static [&'static str]) -> Case {
    Case {
        name,
        read: true,
        query,
        ids,
        counts: None,
        elsewhere: None,
        footnote: None,
        hint: None,
        closure: None,
    }
}

const MORE_ELSEWHERE: &str = "1 more ticket matches with other filters.";

/// Part B's derive rules 1 to 8, over the library's six entries.
#[test]
fn derive_filters_counts_and_sorts() {
    let d_ulid = ID_D.to_lowercase();
    let cases = [
        Case {
            counts: Some((4, 1, 5)),
            elsewhere: Some(0),
            footnote: Some("4 open tickets in manyhands \u{b7} 1 needs fixing"),
            ..case(
                "default",
                TicketQuery::default(),
                &["a", "e", "d", "b", "n"],
            )
        },
        Case {
            footnote: Some("1 closed ticket in manyhands \u{b7} 1 needs fixing"),
            ..case(
                "Closed",
                query(ClosureFilter::Closed, None, ""),
                &["c", "n"],
            )
        },
        Case {
            footnote: Some("5 tickets in manyhands \u{b7} 1 needs fixing"),
            ..case(
                "All",
                query(ClosureFilter::All, None, ""),
                &["a", "e", "d", "c", "b", "n"],
            )
        },
        Case {
            counts: Some((1, 1, 2)),
            elsewhere: Some(1),
            footnote: Some("1 open ticket in manyhands matches \"VH-AAAAA\" \u{b7} 1 needs fixing"),
            hint: Some(MORE_ELSEWHERE),
            ..case(
                "text under Open, pasted with spaces and capitals",
                query(ClosureFilter::Open, None, " VH-AAAAA "),
                &["a", "n"],
            )
        },
        Case {
            elsewhere: Some(0),
            footnote: Some("2 tickets in manyhands match \"VH-AAAAA\" \u{b7} 1 needs fixing"),
            ..case(
                "the same text under All",
                query(ClosureFilter::All, None, " VH-AAAAA "),
                &["a", "c", "n"],
            )
        },
        Case {
            elsewhere: Some(1),
            hint: Some(MORE_ELSEWHERE),
            ..case(
                "the same text under Open and Ready",
                query(
                    ClosureFilter::Open,
                    Some(ReadinessFilter::Ready),
                    " VH-AAAAA ",
                ),
                &["a", "n"],
            )
        },
        case(
            "an ID in lowercase",
            query(ClosureFilter::Open, None, &d_ulid),
            &["d", "n"],
        ),
        Case {
            footnote: Some("0 open tickets in manyhands match \"zzz\" \u{b7} 1 needs fixing"),
            ..case(
                "text nothing matches keeps the nonconforming entry",
                query(ClosureFilter::Open, None, "zzz"),
                &["n"],
            )
        },
        Case {
            elsewhere: Some(3),
            hint: Some("3 more tickets match with other filters."),
            ..case(
                "text under Closed matches three open tickets elsewhere",
                query(ClosureFilter::Closed, None, "vh-"),
                &["c", "n"],
            )
        },
        Case {
            counts: Some((4, 1, 5)),
            footnote: Some("3 ready tickets in manyhands \u{b7} 1 needs fixing"),
            ..case(
                "Ready",
                query(ClosureFilter::Open, Some(ReadinessFilter::Ready), ""),
                &["a", "e", "d", "n"],
            )
        },
        Case {
            counts: Some((4, 1, 5)),
            footnote: Some("1 blocked ticket in manyhands \u{b7} 1 needs fixing"),
            ..case(
                "Blocked",
                query(ClosureFilter::Open, Some(ReadinessFilter::Blocked), ""),
                &["b", "n"],
            )
        },
        Case {
            closure: Some(ClosureFilter::Open),
            ..case(
                "Ready chosen while Closed chooses Open",
                TicketQuery::default()
                    .with_closure(ClosureFilter::Closed)
                    .with_readiness(Some(ReadinessFilter::Ready)),
                &["a", "e", "d", "n"],
            )
        },
        Case {
            closure: Some(ClosureFilter::Closed),
            ..case(
                "Closed chosen after Ready sets Any readiness",
                TicketQuery::default()
                    .with_readiness(Some(ReadinessFilter::Ready))
                    .with_closure(ClosureFilter::Closed),
                &["c", "n"],
            )
        },
        case(
            "Title ascending",
            sorted(SortColumn::Title, true),
            &["a", "e", "b", "d", "n"],
        ),
        case(
            "Title descending",
            sorted(SortColumn::Title, false),
            &["d", "b", "e", "a", "n"],
        ),
        case(
            "Status ascending",
            sorted(SortColumn::Status, true),
            &["e", "a", "d", "b", "n"],
        ),
        case(
            "Status descending",
            sorted(SortColumn::Status, false),
            &["d", "a", "e", "b", "n"],
        ),
        case(
            "Code ascending",
            sorted(SortColumn::Code, true),
            &["a", "b", "e", "d", "n"],
        ),
        case(
            "Code descending",
            sorted(SortColumn::Code, false),
            &["e", "b", "a", "d", "n"],
        ),
        case(
            "Type ascending",
            sorted(SortColumn::Type, true),
            &["a", "d", "e", "b", "n"],
        ),
        case(
            "Type descending",
            sorted(SortColumn::Type, false),
            &["b", "e", "d", "a", "n"],
        ),
        case(
            "Updated descending",
            sorted(SortColumn::Updated, false),
            &["d", "e", "a", "b", "n"],
        ),
        Case {
            read: false,
            counts: None,
            ..case("no rows", TicketQuery::default(), &[])
        },
    ];

    let rows = library();
    for case in cases {
        let name = case.name;
        if let Some(closure) = case.closure {
            assert_eq!(case.query.closure, closure, "{name}: the closure chosen");
        }
        let view = derive(case.read.then_some(rows.as_slice()), &case.query);
        let ids: Vec<_> = view.rows.iter().map(id_of).collect();
        assert_eq!(ids, case.ids, "{name}: the rows, in order");
        assert_eq!(view.listed, case.ids.len(), "{name}: listed");
        assert_eq!(
            view.shown,
            case.ids.len().saturating_sub(1),
            "{name}: shown"
        );
        if let Some((open, closed, all)) = case.counts {
            assert_eq!(
                view.counts,
                Some(SegmentCounts { open, closed, all }),
                "{name}: counts"
            );
        }
        if let Some(elsewhere) = case.elsewhere {
            assert_eq!(view.elsewhere, elsewhere, "{name}: elsewhere");
        }
        assert_eq!(
            hint(&view).as_deref(),
            case.hint,
            "{name}: the hint over the rows"
        );
        if let Some(text) = case.footnote {
            assert_eq!(footnote(&view, &case.query, REPO), text, "{name}: footnote");
        }
        if case.read {
            assert_eq!(view.needs_fixing, 1, "{name}: needs fixing");
            assert_eq!(view.unreadable, 0, "{name}: unreadable");
        } else {
            assert_eq!(view.counts, None, "{name}: nothing to count");
            assert_eq!(
                (view.listed, view.shown, view.elsewhere),
                (0, 0, 0),
                "{name}"
            );
        }
    }
}

/// R-20: whole minutes under an hour, rounded hours under a day, then
/// rounded days; a time after `now` is `0m`; a time that cannot be read,
/// or none, is blank.
#[test]
fn ages_read_as_the_prototype_writes_them() {
    enum At {
        Seconds(i64),
        Minutes(i64),
        Hours(i64),
        Days(i64),
        FromNow(i64),
        Text(Option<&'static str>),
    }
    for (at, label) in [
        (At::Seconds(30), "0m"),
        (At::Minutes(59), "59m"),
        (At::Minutes(60), "1h"),
        (At::Minutes(89), "1h"),
        (At::Minutes(90), "2h"),
        (At::Hours(23), "23h"),
        (At::Hours(24), "1d"),
        (At::Hours(36), "2d"),
        (At::Days(400), "400d"),
        (At::FromNow(300), "0m"),
        (At::Text(None), ""),
        (At::Text(Some("garbage")), ""),
    ] {
        let changed_at = match at {
            At::Seconds(seconds) => Some(now() - Duration::seconds(seconds)),
            At::Minutes(minutes) => Some(now() - Duration::minutes(minutes)),
            At::Hours(hours) => Some(now() - Duration::hours(hours)),
            At::Days(days) => Some(now() - Duration::days(days)),
            At::FromNow(seconds) => Some(now() + Duration::seconds(seconds)),
            At::Text(_) => None,
        };
        let text = match (&at, changed_at) {
            (At::Text(text), _) => text.map(str::to_owned),
            (_, Some(time)) => Some(time.format(&Rfc3339).unwrap()),
            (_, None) => None,
        };
        assert_eq!(age_label(text.as_deref(), now()), label, "{text:?}");
    }
}

/// R-14, decisions 4, 8 and 9: the ID, the context and the reasons are in
/// the tooltip, in that order, each only when it applies.
#[test]
fn tooltips_carry_the_id_context_and_reasons() {
    let primary = context(ItemContextKind::Primary, Some("main"), "/repo");
    let primary_label = context_kind_label(ItemContextKind::Primary);
    let primary_git = "Git: branch main \u{b7} worktree /repo".to_owned();
    let id = |id: &str| format!("ID {id}");
    let uncoded = Spec { slug: None, ..a() };
    let active_worktree = "/repo/.manyhands/worktrees/X";
    let waits = format!("{OPEN_DEPENDENCY}: vh-aaaaa Dock layout");
    let cycle = format!("{DEPENDENCY_CYCLE}: vh-aaaaa, vh-bbbbb");
    let closed = Spec {
        closed: true,
        ..a()
    };

    let mut broken = nonconforming();
    broken.context = primary.clone();
    let cases: Vec<(&str, TicketRow, Vec<String>)> = vec![
        (
            "a coded row on the primary copy",
            tooltip_row(&a(), primary.clone(), &[]),
            vec![
                format!("vh-aaaaa \u{b7} {REPO}"),
                id(ID_A),
                "Type cycle".to_owned(),
                primary_label.to_owned(),
                primary_git.clone(),
            ],
        ),
        (
            "a row with no code",
            tooltip_row(&uncoded, primary.clone(), &[]),
            vec![
                format!("{NO_SHORT_CODE} \u{b7} {REPO}"),
                id(ID_A),
                "Type cycle".to_owned(),
                primary_label.to_owned(),
                primary_git.clone(),
            ],
        ),
        (
            "an active item worktree",
            tooltip_row(
                &a(),
                context(
                    ItemContextKind::Active,
                    Some("manyhands/ticket/X"),
                    active_worktree,
                ),
                &[],
            ),
            vec![
                format!("vh-aaaaa \u{b7} {REPO}"),
                id(ID_A),
                "Type cycle".to_owned(),
                context_kind_label(ItemContextKind::Active).to_owned(),
                format!("Git: branch manyhands/ticket/X \u{b7} worktree {active_worktree}"),
            ],
        ),
        (
            "unverified with no branch",
            tooltip_row(
                &a(),
                context(ItemContextKind::Unverified, None, "/repo"),
                &[],
            ),
            vec![
                format!("vh-aaaaa \u{b7} {REPO}"),
                id(ID_A),
                "Type cycle".to_owned(),
                context_kind_label(ItemContextKind::Unverified).to_owned(),
                format!("Git: {NO_BRANCH} \u{b7} worktree /repo"),
            ],
        ),
        (
            "blocked by an open ticket",
            tooltip_row(&a(), primary.clone(), &[&waits]),
            vec![
                format!("vh-aaaaa \u{b7} {REPO}"),
                id(ID_A),
                "Type cycle".to_owned(),
                primary_label.to_owned(),
                primary_git.clone(),
                waits.clone(),
            ],
        ),
        (
            "blocked on a dependency cycle",
            tooltip_row(&a(), primary.clone(), &[&waits, &cycle]),
            vec![
                format!("vh-aaaaa \u{b7} {REPO}"),
                id(ID_A),
                "Type cycle".to_owned(),
                primary_label.to_owned(),
                primary_git.clone(),
                waits,
                cycle,
            ],
        ),
        (
            "a nonconforming entry",
            broken,
            vec![format!(
                "Needs fixing: {} Open it to fix.",
                ProblemCode::MalformedFrontMatter.guidance()
            )],
        ),
        (
            "a closed ticket",
            tooltip_row(&closed, primary, &[]),
            vec![
                format!("vh-aaaaa \u{b7} {REPO}"),
                id(ID_A),
                "Type cycle".to_owned(),
                primary_label.to_owned(),
                primary_git,
                "Closed".to_owned(),
            ],
        ),
    ];
    for (name, row, lines) in cases {
        assert_eq!(ticket_tooltip(&row, REPO), lines, "{name}");
    }
}

/// What needs attention is counted after the tickets shown, with its own
/// verb: "1 needs fixing", "{k} need fixing", "{k} can't be read".
#[test]
fn the_footnote_counts_what_needs_attention() {
    for (shown, needs_fixing, unreadable, line) in [
        (
            2,
            2,
            1,
            "2 open tickets in manyhands \u{b7} 2 need fixing \u{b7} 1 can't be read",
        ),
        (
            1,
            1,
            2,
            "1 open ticket in manyhands \u{b7} 1 needs fixing \u{b7} 2 can't be read",
        ),
        (
            0,
            0,
            3,
            "0 open tickets in manyhands \u{b7} 3 can't be read",
        ),
        (3, 0, 0, "3 open tickets in manyhands"),
    ] {
        let view = TicketView {
            shown,
            needs_fixing,
            unreadable,
            ..TicketView::default()
        };
        assert_eq!(footnote(&view, &TicketQuery::default(), REPO), line);
    }
}

/// P9: Enter opens the one conforming ticket the text matches, whatever the
/// lifecycle and readiness, even one a filter hides; blank text, no match
/// and several matches open nothing. The text is read as the filter reads
/// it: trimmed, any case, a code, a title or a whole ID.
#[test]
fn a_sole_match_is_the_one_conforming_ticket_the_text_matches() {
    let rows = library();
    let d_ulid = ID_D.to_lowercase();
    for (text, wanted) in [
        ("vh-bbbbb", Some("b")),
        (" VH-EEEEE ", Some("e")),
        ("old dock", Some("c")),
        ("ZETA", Some("d")),
        (d_ulid.as_str(), Some("d")),
        ("", None),
        (" \t ", None),
        ("vh-aaaaa", None),
        ("dock", None),
        ("zzz", None),
    ] {
        assert_eq!(
            sole_match(&rows, text).map(id_of),
            wanted,
            "{text:?} among the six entries"
        );
    }
    // The nonconforming entry matches nothing, so it never makes a second.
    assert_eq!(sole_match(&rows[5..], "vh-bbbbb"), None);
}
