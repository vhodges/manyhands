//! Part B's state table, tooltip and Part D's menu, each as one table over
//! the facts a row is made from. Each case changes only what it is about;
//! the tables are the design's, so a case the code answers differently is a
//! finding, not an edit.

use manyhands::repository::{ConfigurationState, PollingOutcome};

use super::*;
use crate::desktop::model::list_view::{REBUILD_INDEX, REFRESH, RETRY};
use crate::desktop::{
    model::{
        navigation::Resolution,
        polling::{
            Answer, AttemptTarget, Authority, Disposition, Phase, Reason, TransportFailure, Why,
            attention::{self, Action},
            status::{self, Unplaced},
        },
    },
    services::scheduler::SharedCopy,
    view::sidebar::{SHARE_MAIN, SHARE_MAIN_SET_UP, SHARING},
};
use manyhands::results::{ProblemCode, ResultCode};

#[path = "repository_state_tests/menus.rs"]
mod menus;
#[path = "repository_state_tests/pairs.rs"]
mod pairs;
#[path = "repository_state_tests/scene.rs"]
mod scene;
#[path = "repository_state_tests/states.rs"]
mod states;
#[path = "repository_state_tests/tips.rs"]
mod tips;

use scene::{ROOT, Scene, attempt, id, item, key, main_ahead, problem, ulid, unreachable, waiting};

/// What a state's action is, without the data inside it.
#[derive(Debug, PartialEq)]
enum Act {
    None,
    Retry,
    Reread,
    Unlock,
    Attend(Action),
    CheckNow,
    RetryNow,
    Settings,
    SetUp,
}

fn act_of(action: &Option<RowAction>) -> Act {
    match action {
        None => Act::None,
        Some(RowAction::Retry) => Act::Retry,
        Some(RowAction::Reread) => Act::Reread,
        Some(RowAction::Unlock(unlock)) => {
            assert_eq!(*unlock, key(), "Unlock names the locked key");
            Act::Unlock
        }
        Some(RowAction::Attend(action, _)) => Act::Attend(*action),
        Some(RowAction::CheckNow) => Act::CheckNow,
        Some(RowAction::RetryNow) => Act::RetryNow,
        Some(RowAction::Settings) => Act::Settings,
        Some(RowAction::SetUpSharing) => Act::SetUp,
    }
}

type Set = fn(&mut Scene);

struct Case {
    name: &'static str,
    set: Set,
    kind: StateKind,
    label: Option<String>,
    act: Act,
}

fn case(name: &'static str, set: Set, kind: StateKind, label: Option<&str>, act: Act) -> Case {
    Case {
        name,
        set,
        kind,
        label: label.map(str::to_owned),
        act,
    }
}

fn attention(reason: Reason) -> Phase {
    Phase::Attention(reason)
}

fn authority() -> Authority {
    Authority {
        host: "example.invalid".to_owned(),
        port: 22,
    }
}

fn dirty() -> AttemptTarget {
    item(9, Disposition::Dirty)
}

/// Part B's rows 1 to 13, each alone, then the pairs that decide the order.
#[test]
fn each_state_and_its_precedence() {
    for case in states::alone().into_iter().chain(pairs::pairs()) {
        let mut scene = Scene::new();
        (case.set)(&mut scene);
        let state = scene.state();
        assert_eq!(state.kind, case.kind, "{}: kind", case.name);
        assert_eq!(state.label, case.label, "{}: label", case.name);
        assert_eq!(act_of(&state.action), case.act, "{}: action", case.name);
        // Colour is never the only signal: a tone comes with a label and an
        // icon, and a row with no label has neither.
        assert_eq!(
            state.tone.is_some(),
            state.label.is_some(),
            "{}: tone",
            case.name
        );
        assert_eq!(
            state.icon.is_some(),
            state.label.is_some(),
            "{}: icon",
            case.name
        );
    }
}

/// Part B's lines 1 to 4: the state's sentence, the folder, the main branch
/// when known and the shared copy when there is one, and nothing else.
#[test]
fn tooltips_say_the_state_and_the_facts() {
    for tip in tips::tips() {
        let mut scene = Scene::new();
        (tip.set)(&mut scene);
        let titles = |item: &manyhands::canonical::ItemId| {
            (1..=7)
                .find(|n| id(*n) == *item)
                .and_then(|n| tip.titles.iter().find(|(known, _)| *known == n))
                .map(|(_, title)| (*title).to_owned())
        };
        let facts = scene.facts(&titles);
        let state = repository_state(&facts);
        assert_eq!(
            repository_tooltip(&facts, &state),
            tip.lines,
            "{}",
            tip.name
        );
    }

    // Titles are asked for only the named items that have a kind.
    let mut scene = Scene::new();
    let mut kindless = tips::ahead(1);
    kindless.kind = None;
    let mut items = vec![kindless];
    items.extend((2..=8).map(tips::ahead));
    scene.checked(items, Vec::new());
    let asked = std::cell::RefCell::new(Vec::new());
    let titles = |item: &manyhands::canonical::ItemId| {
        asked.borrow_mut().push(item.clone());
        None
    };
    repository_state(&scene.facts(&titles));
    assert_eq!(
        *asked.borrow(),
        (2..=5).map(id).collect::<Vec<_>>(),
        "the five named, less the one with no kind"
    );

    // The Locked sentence is D6's headline with a full stop.
    assert_eq!(
        LOCKED_SENTENCE,
        format!("{}.", attention::headline(&Reason::KeyLocked(key())))
    );
    assert_eq!(CHECKING, status::CHECKING);
    assert_eq!(PAUSED, status::PAUSED);
}

/// Part D's table: the sentence first and disabled, the state's action
/// next and not repeated below, Check now unless it is the state's action,
/// Share main, Open folder, Settings… unless it is the state's action, and
/// Remove last, with each item's enablement.
#[test]
fn menu_items_follow_the_state() {
    use MenuChoice::*;
    for menu in menus::menus() {
        let mut scene = Scene::new();
        (menu.set)(&mut scene);
        let titles = |_: &manyhands::canonical::ItemId| None;
        let facts = scene.facts(&titles);
        let state = repository_state(&facts);
        assert_eq!(
            menus::shown(menu_items(&facts, &state)),
            menu.items,
            "{}",
            menu.name
        );
    }

    // More: nothing chosen disables all three.
    let key = "/work/alpha".to_owned();
    for (chosen, enabled) in [(Some(&key), true), (None, false)] {
        assert_eq!(
            menus::shown(more_items(chosen)),
            vec![
                menus::entry(Refresh, REFRESH, enabled),
                menus::entry(Rebuild, REBUILD_INDEX, enabled),
                menus::entry(Remove, REMOVE_FROM_MANYHANDS, enabled),
            ],
            "more with {chosen:?}"
        );
    }
}

/// Part E's Check all: which repositories it takes, one table over the
/// facts. A paused repository is taken (decision 16); one checking, one
/// whose shape does not poll, one waiting on a key or a host, live or by a
/// stored outcome that still applies, is not (R-13).
#[test]
fn check_all_takes_each_repository_that_can_check() {
    let cases: Vec<(&str, Set, bool)> = vec![
        ("an ordinary repository", |_| {}, true),
        ("a paused one", |s| s.schedule().policy.paused = true, true),
        (
            "our own Check now is running",
            |s| s.checking_now = true,
            false,
        ),
        (
            "a poll is queued",
            |s| s.schedule().phase = Phase::Queued,
            false,
        ),
        (
            "a live locked key",
            |s| s.schedule().phase = attention(Reason::KeyLocked(key())),
            false,
        ),
        (
            "a live key problem",
            |s| s.schedule().phase = attention(Reason::KeyProblem(key())),
            false,
        ),
        (
            "no key selected",
            |s| s.schedule().phase = attention(Reason::NoKeySelected(ROOT.to_owned())),
            false,
        ),
        (
            "an unapproved host",
            |s| s.schedule().phase = attention(Reason::HostUnapproved(authority())),
            false,
        ),
        (
            "a host that names none",
            |s| s.schedule().phase = attention(Reason::HostUnidentified(ROOT.to_owned())),
            false,
        ),
        (
            "locked and paused",
            |s| {
                s.schedule().policy.paused = true;
                s.schedule().phase = attention(Reason::KeyLocked(key()));
            },
            false,
        ),
        (
            "a configuration that needs its remote settings",
            |s| s.schedule().phase = attention(Reason::ConfigurationRequired(ROOT.to_owned())),
            true,
        ),
        (
            "held until recovery",
            |s| s.schedule().phase = attention(Reason::RecoveryHeld(ROOT.to_owned())),
            false,
        ),
        (
            "a stored unlock required",
            |s| s.stored = Some(PollingOutcome::UnlockRequired),
            false,
        ),
        (
            "a stored selected key unavailable",
            |s| s.stored = Some(PollingOutcome::SelectedKeyUnavailable),
            false,
        ),
        (
            "a stored host approval required",
            |s| s.stored = Some(PollingOutcome::HostApprovalRequired),
            false,
        ),
        (
            "a stored transport failure",
            |s| s.stored = Some(PollingOutcome::TransportUnavailable),
            true,
        ),
        (
            "a stored completion",
            |s| s.stored = Some(PollingOutcome::Completed),
            true,
        ),
        (
            "a stored unlock required, after an attempt this session",
            |s| {
                s.stored = Some(PollingOutcome::UnlockRequired);
                s.schedule().last = Some(attempt(Answer::Completed));
            },
            true,
        ),
        // Owner ruling 2026-11-12 (decision 16): a stored outcome that waits
        // on a key or a host is skipped whether or not the repository is
        // paused or in a wait; this row's expectation was `true` under the
        // literal "stored_applies only" reading and is now `false`.
        (
            "a stored unlock required, paused",
            |s| {
                s.stored = Some(PollingOutcome::UnlockRequired);
                s.schedule().policy.paused = true;
            },
            false,
        ),
        (
            "a stored selected key unavailable, paused",
            |s| {
                s.stored = Some(PollingOutcome::SelectedKeyUnavailable);
                s.schedule().policy.paused = true;
            },
            false,
        ),
        (
            "a stored host approval required, paused",
            |s| {
                s.stored = Some(PollingOutcome::HostApprovalRequired);
                s.schedule().policy.paused = true;
            },
            false,
        ),
        (
            "a stored unlock required, checking off",
            |s| {
                s.stored = Some(PollingOutcome::UnlockRequired);
                s.schedule().policy.enabled = false;
            },
            false,
        ),
        (
            "a stored unlock required, in a busy wait",
            |s| {
                s.stored = Some(PollingOutcome::UnlockRequired);
                s.schedule().phase = waiting(Why::Busy);
            },
            false,
        ),
        (
            "a stored unlock required, in an interrupted wait",
            |s| {
                s.stored = Some(PollingOutcome::UnlockRequired);
                s.schedule().phase = waiting(Why::Interrupted);
            },
            false,
        ),
        (
            "a stored unlock required, in a failed wait",
            |s| {
                s.stored = Some(PollingOutcome::UnlockRequired);
                s.schedule().phase = waiting(Why::Failed);
            },
            false,
        ),
        (
            "a stored transport failure, paused: not a key or a host",
            |s| {
                s.stored = Some(PollingOutcome::TransportUnavailable);
                s.schedule().policy.paused = true;
            },
            true,
        ),
        // The design states no expectation for these two; they pin what the
        // code answers: neither excludes a repository.
        ("the kept row", |s| s.kept = true, true),
        (
            "a pending conflict",
            |s| s.schedule().conflict_pending = true,
            true,
        ),
        ("no schedule, no remote", |s| s.unscheduled(None), false),
        (
            "its remotes still being read",
            |s| {
                s.unscheduled(None);
                s.unplaced = Some(Unplaced::Reading);
            },
            false,
        ),
        (
            "checking switched off",
            |s| s.schedule().policy.enabled = false,
            false,
        ),
    ];
    for (name, set, takes) in cases {
        let mut scene = Scene::new();
        set(&mut scene);
        let titles = |_: &manyhands::canonical::ItemId| None;
        assert_eq!(check_all_applies(&scene.facts(&titles)), takes, "{name}");
    }
}
