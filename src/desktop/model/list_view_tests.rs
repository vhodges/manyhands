use manyhands::{
    repository::{ClosureFilter, IndexState, IndexStateDto},
    results::ResultCode,
};

use super::{Alert, AlertTone, BodyInput, ListAction, ListBody, ListKind, list_body, loading_text};
use crate::desktop::{
    model::list_state::{KeptList, ListState},
    services::ReadFailure,
};

const REPO: &str = "manyhands";

fn ready(rows: Vec<u8>, complete: bool, index: Option<IndexState>) -> ListState<Vec<u8>> {
    ListState::Ready {
        rows,
        complete,
        index: index.map(|state| IndexStateDto {
            state,
            refreshed_at: None,
        }),
    }
}

fn failed(code: ResultCode) -> ListState<Vec<u8>> {
    ListState::Failed(ReadFailure {
        code,
        recovery: Vec::new(),
    })
}

/// How a test list got into its state.
#[derive(Clone)]
enum Setup {
    /// A finished state applied to a fresh list.
    State(ListState<Vec<u8>>),
    /// A ready state, then a busy read over it.
    BusyOverKept(ListState<Vec<u8>>),
    /// A ready state, then a reload that has not landed.
    ReloadOverKept(ListState<Vec<u8>>),
    /// Nothing read yet.
    Fresh,
}

fn list(setup: &Setup) -> KeptList<Vec<u8>> {
    let mut list = KeptList::default();
    match setup.clone() {
        Setup::State(state) => list.apply_state(state),
        Setup::BusyOverKept(state) => {
            list.apply_state(state);
            list.reload();
            list.apply_state(ListState::Busy);
        }
        Setup::ReloadOverKept(state) => {
            list.apply_state(state);
            list.reload();
        }
        Setup::Fresh => {}
    }
    list
}

/// What a row of the table expects, with the text for Tickets and for
/// Documents (`[tickets, documents]`).
enum Want {
    NoRepository,
    Skeleton([&'static str; 2]),
    Rows(Option<(AlertTone, &'static str, ListAction)>),
    Empty([&'static str; 2], [Option<ListAction>; 2]),
    Problem(
        AlertTone,
        [&'static str; 2],
        Option<&'static str>,
        ListAction,
    ),
}

fn same(text: &'static str) -> [&'static str; 2] {
    [text, text]
}

struct Case {
    name: &'static str,
    setup: Setup,
    repo: Option<&'static str>,
    listed: usize,
    elsewhere: usize,
    closure: ClosureFilter,
    narrowed: bool,
    want: Want,
}

fn case(name: &'static str, setup: Setup, listed: usize, want: Want) -> Case {
    Case {
        name,
        setup,
        repo: Some(REPO),
        listed,
        elsewhere: 0,
        closure: ClosureFilter::Open,
        narrowed: false,
        want,
    }
}

fn cases() -> Vec<Case> {
    use ListAction::{ClearFilters, NewItem, RebuildIndex, Refresh, Retry, ShowAll};
    let current = || ready(vec![1], true, Some(IndexState::Current));
    let busy_kept = "manyhands was busy, so this list wasn't updated.";
    let never = "manyhands hasn't been scanned yet, so this list may be empty.";
    let stale = "The list for manyhands may be out of date.";
    let incomplete = "A folder in manyhands couldn't be read, so this list may be missing items.";
    let warn = |text, action| Want::Rows(Some((AlertTone::Warning, text, action)));
    vec![
        Case {
            repo: None,
            ..case(
                "no repository selected",
                Setup::State(current()),
                3,
                Want::NoRepository,
            )
        },
        case(
            "first load",
            Setup::Fresh,
            0,
            Want::Skeleton(["Loading tickets", "Loading documents"]),
        ),
        case(
            "a reload over kept rows shows them with no alert",
            Setup::ReloadOverKept(current()),
            3,
            Want::Rows(None),
        ),
        case(
            "a reload over kept stale rows still has no alert",
            Setup::ReloadOverKept(ready(vec![1], true, Some(IndexState::Stale))),
            3,
            Want::Rows(None),
        ),
        case(
            "ready and current with rows",
            Setup::State(current()),
            3,
            Want::Rows(None),
        ),
        case(
            "ready with a listed row that is not conforming is rows, not empty",
            Setup::State(current()),
            1,
            Want::Rows(None),
        ),
        case(
            "ready with no rows but one listed is rows, not empty",
            Setup::State(ready(Vec::new(), true, Some(IndexState::Current))),
            1,
            Want::Rows(None),
        ),
        case(
            "ready with no index state is current",
            Setup::State(ready(vec![1], true, None)),
            3,
            Want::Rows(None),
        ),
        case(
            "empty under Open",
            Setup::State(current()),
            0,
            Want::Empty(
                [
                    "No open tickets in manyhands.",
                    "No documents in manyhands yet.",
                ],
                [Some(NewItem), Some(NewItem)],
            ),
        ),
        Case {
            closure: ClosureFilter::Closed,
            ..case(
                "empty under Closed has no action for tickets",
                Setup::State(current()),
                0,
                Want::Empty(
                    [
                        "No closed tickets in manyhands.",
                        "No documents in manyhands yet.",
                    ],
                    [None, Some(NewItem)],
                ),
            )
        },
        Case {
            closure: ClosureFilter::All,
            ..case(
                "empty under All",
                Setup::State(current()),
                0,
                Want::Empty(
                    [
                        "No tickets in manyhands yet.",
                        "No documents in manyhands yet.",
                    ],
                    [Some(NewItem), Some(NewItem)],
                ),
            )
        },
        Case {
            narrowed: true,
            ..case(
                "narrowed and empty, no match elsewhere",
                Setup::State(current()),
                0,
                Want::Empty(
                    [
                        "No tickets match these filters.",
                        // Documents ignore `narrowed`.
                        "No documents in manyhands yet.",
                    ],
                    [Some(ClearFilters), Some(NewItem)],
                ),
            )
        },
        Case {
            narrowed: true,
            closure: ClosureFilter::Closed,
            ..case(
                "narrowed under Closed still clears filters",
                Setup::State(current()),
                0,
                Want::Empty(
                    [
                        "No tickets match these filters.",
                        // Documents ignore `narrowed`.
                        "No documents in manyhands yet.",
                    ],
                    [Some(ClearFilters), Some(NewItem)],
                ),
            )
        },
        Case {
            narrowed: true,
            elsewhere: 1,
            ..case(
                "narrowed and empty, one match elsewhere",
                Setup::State(current()),
                0,
                Want::Empty(
                    [
                        "No tickets here match. 1 more ticket matches with other filters.",
                        // Documents ignore `narrowed` and `elsewhere`.
                        "No documents in manyhands yet.",
                    ],
                    [Some(ShowAll), Some(NewItem)],
                ),
            )
        },
        Case {
            narrowed: true,
            elsewhere: 3,
            ..case(
                "narrowed and empty, several matches elsewhere",
                Setup::State(current()),
                0,
                Want::Empty(
                    [
                        "No tickets here match. 3 more tickets match with other filters.",
                        "No documents in manyhands yet.",
                    ],
                    [Some(ShowAll), Some(NewItem)],
                ),
            )
        },
        Case {
            narrowed: true,
            elsewhere: 2,
            ..case(
                "narrowed with rows listed is rows",
                Setup::State(current()),
                1,
                Want::Rows(None),
            )
        },
        case(
            "busy over kept rows",
            Setup::BusyOverKept(current()),
            3,
            warn(busy_kept, Retry),
        ),
        case(
            "busy over kept rows beats a stale alert",
            Setup::BusyOverKept(ready(vec![1], false, Some(IndexState::Stale))),
            3,
            warn(busy_kept, Retry),
        ),
        case(
            "busy over kept rows with none listed is an alert, not empty",
            Setup::BusyOverKept(current()),
            0,
            warn(busy_kept, Retry),
        ),
        case(
            "never refreshed",
            Setup::State(ready(vec![1], true, Some(IndexState::NeverRefreshed))),
            3,
            warn(never, Refresh),
        ),
        case(
            "never refreshed with none listed is an alert, not empty",
            Setup::State(ready(vec![1], true, Some(IndexState::NeverRefreshed))),
            0,
            warn(never, Refresh),
        ),
        case(
            "never refreshed wins over incomplete",
            Setup::State(ready(vec![1], false, Some(IndexState::NeverRefreshed))),
            3,
            warn(never, Refresh),
        ),
        case(
            "stale",
            Setup::State(ready(vec![1], true, Some(IndexState::Stale))),
            3,
            warn(stale, Refresh),
        ),
        case(
            "stale with none listed is an alert, not empty",
            Setup::State(ready(vec![1], true, Some(IndexState::Stale))),
            0,
            warn(stale, Refresh),
        ),
        case(
            "stale wins over incomplete",
            Setup::State(ready(vec![1], false, Some(IndexState::Stale))),
            3,
            warn(stale, Refresh),
        ),
        case(
            "incomplete",
            Setup::State(ready(vec![1], false, Some(IndexState::Current))),
            3,
            warn(incomplete, Refresh),
        ),
        case(
            "incomplete with no index state",
            Setup::State(ready(vec![1], false, None)),
            3,
            warn(incomplete, Refresh),
        ),
        case(
            "busy with nothing kept",
            Setup::State(ListState::Busy),
            0,
            Want::Problem(
                AlertTone::Warning,
                same("manyhands is busy right now."),
                None,
                Retry,
            ),
        ),
        case(
            "failed carries the failure's message",
            Setup::State(failed(ResultCode::RepositoryInaccessible)),
            0,
            Want::Problem(
                AlertTone::Failure,
                [
                    "Couldn't read the tickets in manyhands. Nothing was changed.",
                    "Couldn't read the documents in manyhands. Nothing was changed.",
                ],
                Some(ResultCode::RepositoryInaccessible.message()),
                Retry,
            ),
        ),
        case(
            "index unavailable",
            Setup::State(failed(ResultCode::IndexUnavailable)),
            0,
            Want::Problem(
                AlertTone::Failure,
                same("The search index for manyhands can't be read. Nothing was changed."),
                None,
                RebuildIndex,
            ),
        ),
    ]
}

#[test]
fn each_state_has_its_own_body() {
    for (ix, kind) in [ListKind::Tickets, ListKind::Documents]
        .into_iter()
        .enumerate()
    {
        for case in cases() {
            let body = list_body(
                &list(&case.setup),
                BodyInput {
                    kind,
                    repo: case.repo,
                    listed: case.listed,
                    elsewhere: case.elsewhere,
                    closure: case.closure,
                    narrowed: case.narrowed,
                },
            );
            let name = format!("{kind:?}: {}", case.name);
            let alert = |tone, text: &str, detail: Option<&str>, action| Alert {
                tone,
                text: text.to_string(),
                detail: detail.map(str::to_string),
                action,
            };
            let expected = match case.want {
                Want::NoRepository => ListBody::NoRepository,
                Want::Skeleton(text) => {
                    assert_eq!(loading_text(kind), text[ix], "{name}");
                    ListBody::Skeleton
                }
                Want::Rows(None) => ListBody::Rows { alert: None },
                Want::Rows(Some((tone, text, action))) => ListBody::Rows {
                    alert: Some(alert(tone, text, None, action)),
                },
                Want::Empty(text, action) => ListBody::Empty {
                    text: text[ix].to_string(),
                    action: action[ix],
                },
                Want::Problem(tone, text, detail, action) => {
                    ListBody::Problem(alert(tone, text[ix], detail, action))
                }
            };
            assert_eq!(body, expected, "{name}");
        }
    }
}
