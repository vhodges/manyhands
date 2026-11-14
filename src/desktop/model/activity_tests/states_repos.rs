//! P1 for S4: each repository state, with its cause and, for Locked, its
//! action, makes its row or none (R-14, R-15, R-40).

use super::{
    super::support::ROOT,
    world::{self, OTHER, World, assert_rows, counted, key_id, quiet, repository, state, want},
};
use crate::desktop::model::{
    activity::rows::{ActivityAction as Act, Kind, RowKey},
    polling::{Reason, attention::Action},
    repository_state::{AttentionCause as Cause, RowAction, StateKind as K},
};

/// One S4 case: a state, and the row it makes, if any.
struct Repo {
    name: &'static str,
    state: crate::desktop::model::repository_state::RepoState,
    row: Option<world::Want>,
}

fn repo_cases() -> Vec<Repo> {
    let key = RowKey::Repository(ROOT.to_owned());
    let detail = ["Main branch: trunk", "Shared copy: origin"];
    let button = |row: world::Want, action: RowAction, label: &str| {
        row.button(Act::Repository(ROOT.to_owned(), action), label, true)
    };
    let attention = |kind: Kind, title: &str, description: &str, action: RowAction, label: &str| {
        button(
            want(key.clone(), kind, title)
                .desc(description)
                .repo("demo")
                .detail(&detail)
                .group(counted(&[ROOT])),
            action,
            label,
        )
    };
    let held = RowAction::Attend(
        Action::ResumeChecking,
        Reason::RecoveryHeld(ROOT.to_owned()),
    );
    let attend = |name, cause: Cause, action: RowAction, label: &str| Repo {
        name,
        state: state(
            K::NeedsAttention,
            Some(action.clone()),
            &["It needs a look"],
            Some(cause),
        ),
        row: Some(attention(
            Kind::Failure,
            "Checking demo needs attention",
            "It needs a look.",
            action,
            label,
        )),
    };
    let none = |name, kind, action, cause| Repo {
        name,
        state: state(kind, action, &["Says something"], cause),
        row: None,
    };
    vec![
        Repo {
            name: "can't open",
            state: state(
                K::CantOpen,
                Some(RowAction::Retry),
                &[
                    "The repository is not on disk.",
                    "Its files are unchanged. Retry, or remove it from Manyhands.",
                ],
                None,
            ),
            row: Some(attention(
                Kind::Failure,
                "Can't open demo",
                "The repository is not on disk. Its files are unchanged. Retry, or remove it from Manyhands.",
                RowAction::Retry,
                "Retry",
            )),
        },
        Repo {
            name: "needs fixing",
            state: state(
                K::NeedsFixing,
                Some(RowAction::Settings),
                &["Its configuration needs fixing:", "Repair the path."],
                None,
            ),
            row: Some(attention(
                Kind::Failure,
                "demo needs fixing",
                "Its configuration needs fixing: Repair the path.",
                RowAction::Settings,
                "Settings…",
            )),
        },
        Repo {
            name: "can't reach",
            state: state(
                K::CantReach,
                Some(RowAction::RetryNow),
                &["Couldn't reach it"],
                None,
            ),
            row: Some(
                attention(
                    Kind::Failure,
                    "Couldn't reach demo's shared copy",
                    "Your work is safe, and Manyhands tries again.",
                    RowAction::RetryNow,
                    "Retry now",
                )
                .unreachable(),
            ),
        },
        Repo {
            name: "locked by a stored outcome",
            state: state(K::Locked, Some(RowAction::CheckNow), &["Locked"], None),
            row: Some(attention(
                Kind::Locked,
                "Checking demo needs your key's passphrase",
                "Check now asks for it.",
                RowAction::CheckNow,
                "Check now",
            )),
        },
        none(
            "locked by a live key",
            K::Locked,
            Some(RowAction::Unlock(key_id())),
            None,
        ),
        attend(
            "held, no reason in the phase",
            Cause::Held,
            held,
            "Resume checking",
        ),
        attend(
            "a failed wait",
            Cause::WaitingFailed,
            RowAction::RetryNow,
            "Retry now",
        ),
        attend(
            "an unreadable registration",
            Cause::Unreadable,
            RowAction::Reread,
            "Retry",
        ),
        attend(
            "a transport answer",
            Cause::Transport,
            RowAction::RetryNow,
            "Retry now",
        ),
        attend(
            "a stored outcome",
            Cause::Stored,
            RowAction::CheckNow,
            "Check now",
        ),
        none(
            "needs attention, a reason in its phase",
            K::NeedsAttention,
            Some(RowAction::Attend(
                Action::ChooseKey,
                Reason::KeyProblem(key_id()),
            )),
            Some(Cause::Phase(Reason::KeyProblem(key_id()))),
        ),
        none(
            "needs attention, a conflict",
            K::NeedsAttention,
            None,
            Some(Cause::Conflict),
        ),
        none(
            "needs attention, problems",
            K::NeedsAttention,
            None,
            Some(Cause::Problems),
        ),
        none("needs attention, no cause", K::NeedsAttention, None, None),
        Repo {
            name: "this computer",
            state: state(
                K::ThisComputer,
                Some(RowAction::SetUpSharing),
                &["No shared copy, so changes stay on this computer."],
                None,
            ),
            row: Some(button(
                want(
                    key.clone(),
                    Kind::Info,
                    "demo has no shared copy, so changes stay on this computer",
                )
                .repo("demo")
                .detail(&detail)
                .group(quiet(&[ROOT])),
                RowAction::SetUpSharing,
                "Set up sharing…",
            )),
        },
        Repo {
            name: "a remote that is not SSH",
            state: state(
                K::NotSsh,
                Some(RowAction::SetUpSharing),
                &["origin isn't an SSH shared copy, so Manyhands can't check or share it."],
                None,
            ),
            row: Some(button(
                want(
                    key.clone(),
                    Kind::Info,
                    "origin isn't an SSH shared copy, so Manyhands can't check or share it",
                )
                .repo("demo")
                .detail(&detail)
                .group(quiet(&[ROOT])),
                RowAction::SetUpSharing,
                "Set up sharing…",
            )),
        },
        none("busy opening", K::BusyOpening, Some(RowAction::Retry), None),
        none("checking", K::Checking, None, None),
        none("to share", K::ToShare(2), None, None),
        none("paused", K::Paused, Some(RowAction::CheckNow), None),
        none("up to date", K::UpToDate, None, None),
        none("unknown", K::Unknown, None, None),
    ]
}

/// S4: each state, with its cause and, for Locked, its action, makes its
/// row or none (R-14, R-15, R-40); the detail is `repository_detail`'s.
pub(super) fn repositories() {
    for case in repo_cases() {
        let mut world = World::new();
        let mut input = repository(ROOT, "demo", case.state);
        input.primary = Some("trunk".to_owned());
        input.remote = Some("origin".to_owned());
        world.repositories = vec![input];
        let wanted: Vec<world::Want> = case.row.into_iter().collect();
        assert_rows(&world.rows(), &wanted, &format!("S4 {}", case.name));
    }

    // Two repositories, in list order, each with its own row.
    let mut world = World::new();
    let failing = |root: &str, name: &str| {
        repository(
            root,
            name,
            state(
                K::CantReach,
                Some(RowAction::RetryNow),
                &["Couldn't reach it"],
                None,
            ),
        )
    };
    world.repositories = vec![failing(OTHER, "other"), failing(ROOT, "demo")];
    let keys: Vec<RowKey> = world.rows().into_iter().map(|row| row.key).collect();
    assert_eq!(
        keys,
        [
            RowKey::Repository(OTHER.to_owned()),
            RowKey::Repository(ROOT.to_owned())
        ]
    );
}
