//! P1 for S2-S14 and P6: the state rows. `World` is what the Workspace
//! hands `state_rows`; each case sets one source up and lists every row it
//! must make, so a row that is made and should not be fails as well as one
//! that is missing.

use super::support::ROOT;
use crate::desktop::model::{
    activity::rows::{ActivityAction as Act, Kind, RowKey},
    polling::{
        Answer, Authority, Disposition, Reason, TransportFailure,
        attention::{Action, headline},
        status,
    },
};

#[path = "states_copy.rs"]
mod copy;
#[path = "states_drafts.rs"]
mod drafts;
#[path = "states_items.rs"]
mod items;
#[path = "states_pending.rs"]
mod pending;
#[path = "states_repos.rs"]
mod repos;
#[path = "states_retire.rs"]
mod retire;
#[path = "world.rs"]
pub(super) mod world;
use world::{OTHER, World, assert_rows, counted, key_id, want};

/// P1, S2 to S14.
pub(super) fn each_state_source_makes_its_rows() {
    brought_in();
    holds();
    repos::repositories();
    items::problems();
    items::unshared();
    retire::retire_offers();
    retire::retire_events();
    pending::pending_rows();
    drafts::pending_lists();
    drafts::drafts();
    drafts::notes();
}

/// S2: a check that brought something in, from the schedule's own list; an
/// automatic one and a Check now say it differently, and quiet ones none.
fn brought_in() {
    let mut world = World::new();
    world.schedule_root(ROOT);
    assert!(world.rows().is_empty(), "nothing yet");
    let completed = || Answer::Completed;

    world.automatic(ROOT, completed(), &[Disposition::Current]);
    world.check_now(ROOT, completed(), &[Disposition::LocalAhead]);
    assert!(world.rows().is_empty(), "quiet checks add no row");
    world.automatic(ROOT, Answer::Transport(TransportFailure::Unreachable), &[]);
    assert!(world.rows().is_empty(), "a failed check adds no row");

    let first = world.utc();
    world.automatic(ROOT, completed(), &[Disposition::FastForwarded]);
    world.tick();
    let second = world.utc();
    world.automatic(
        ROOT,
        completed(),
        &[
            Disposition::FastForwarded,
            Disposition::Materialized,
            Disposition::Current,
        ],
    );
    world.tick();
    let third = world.utc();
    world.check_now(ROOT, completed(), &[Disposition::Materialized]);
    world.tick();
    let fourth = world.utc();
    world.check_now(
        ROOT,
        completed(),
        &[
            Disposition::FastForwarded,
            Disposition::FastForwarded,
            Disposition::Materialized,
        ],
    );

    let row = |seq: u64, title: &str, description: Option<&str>, at| {
        let brought = want(
            RowKey::Brought {
                root: ROOT.to_owned(),
                seq,
            },
            Kind::Sharing,
            title,
        )
        .repo("demo")
        .then(
            Act::Dismiss(RowKey::Brought {
                root: ROOT.to_owned(),
                seq,
            }),
            "Dismiss",
            true,
        )
        .clearable()
        .at(at);
        match description {
            Some(description) => brought.desc(description),
            None => brought,
        }
    };
    assert_rows(
        &world.rows(),
        &[
            row(
                0,
                "Checked demo for updates",
                Some("Brought in 1 change."),
                first,
            ),
            row(
                1,
                "Checked demo for updates",
                Some("Brought in 2 changes."),
                second,
            ),
            row(2, "Brought in 1 change to demo", None, third),
            row(3, "Brought in 3 changes to demo", None, fourth),
        ],
        "S2",
    );
}

/// What a hold case sets up: how the schedule comes to hold the roots, and
/// the row the hold must make.
struct Hold {
    name: &'static str,
    roots: Vec<&'static str>,
    answer: TransportFailure,
    key: String,
    kind: Kind,
    title: String,
    description: &'static str,
    action: Action,
    reason: Reason,
    label: &'static str,
}

fn holds_table() -> Vec<Hold> {
    let key = key_id();
    let host = Authority {
        host: "git.example.invalid".to_owned(),
        port: 22,
    };
    vec![
        Hold {
            name: "a locked key over two repositories",
            roots: vec![ROOT, OTHER],
            answer: TransportFailure::KeyLocked { key: Some(key) },
            key: format!("KeyLocked:{key}"),
            kind: Kind::Locked,
            title: "Your key is locked".to_owned(),
            description: "Checking demo, other is waiting for the passphrase.",
            action: Action::Unlock,
            reason: Reason::KeyLocked(key),
            label: "Unlock…",
        },
        Hold {
            name: "a key problem",
            roots: vec![ROOT],
            answer: TransportFailure::KeyProblem { key: Some(key) },
            key: format!("KeyProblem:{key}"),
            kind: Kind::Failure,
            title: "Checking needs a key to be chosen".to_owned(),
            description: "Holding back demo.",
            action: Action::ChooseKey,
            reason: Reason::KeyProblem(key),
            label: "Choose a key…",
        },
        Hold {
            name: "no key selected",
            roots: vec![ROOT],
            answer: TransportFailure::KeyProblem { key: None },
            key: format!("NoKeySelected:{ROOT}"),
            kind: Kind::Failure,
            title: "Checking needs a key to be chosen".to_owned(),
            description: "Holding back demo.",
            action: Action::ChooseKey,
            reason: Reason::NoKeySelected(ROOT.to_owned()),
            label: "Choose a key…",
        },
        Hold {
            name: "a host that needs approval",
            roots: vec![ROOT],
            answer: TransportFailure::HostUnapproved {
                authority: Some(host.clone()),
            },
            key: "HostUnapproved:git.example.invalid:22".to_owned(),
            kind: Kind::Failure,
            title: "git.example.invalid needs your approval".to_owned(),
            description: "Checking demo waits until you approve it.",
            action: Action::ApproveHost,
            reason: Reason::HostUnapproved(host),
            label: "Approve host…",
        },
        Hold {
            name: "a server with no name",
            roots: vec![ROOT],
            answer: TransportFailure::HostUnapproved { authority: None },
            key: format!("HostUnidentified:{ROOT}"),
            kind: Kind::Failure,
            title: "A server needs your approval".to_owned(),
            description: "Checking demo waits until you approve it.",
            action: Action::ApproveHost,
            reason: Reason::HostUnidentified(ROOT.to_owned()),
            label: "Approve host…",
        },
        Hold {
            name: "sharing is not set up",
            roots: vec![ROOT],
            answer: TransportFailure::ConfigurationRequired,
            key: format!("ConfigurationRequired:{ROOT}"),
            kind: Kind::Failure,
            title: "Sharing isn't set up for demo".to_owned(),
            description: "Checking can't start until it is.",
            action: Action::OpenRemoteSettings,
            reason: Reason::ConfigurationRequired(ROOT.to_owned()),
            label: "Open remote settings…",
        },
    ]
}

/// S3: each reason a hold has, its words and action; Unlock… is enabled by
/// `unlock_target`; the hold covers every root it holds back.
fn holds() {
    for hold in holds_table() {
        for unlockable in [true, false] {
            let mut world = World::new();
            world.unlockable = unlockable;
            for root in &hold.roots {
                world.schedule_root(root);
                world.automatic(root, Answer::Transport(hold.answer.clone()), &[]);
            }
            let names = hold
                .roots
                .iter()
                .map(|root| if *root == ROOT { "demo" } else { "other" })
                .collect::<Vec<_>>()
                .join(", ");
            let tag = match hold.roots.len() {
                1 => "demo".to_owned(),
                more => format!("demo +{}", more - 1),
            };
            let enabled = unlockable || hold.action != Action::Unlock;
            let mut detail = vec![
                headline(&hold.reason).to_owned(),
                format!("Holding back {names}"),
            ];
            if !enabled {
                detail
                    .push("Finish the pending operations of these repositories first.".to_owned());
            }
            let row = want(RowKey::Hold(hold.key.clone()), hold.kind, &hold.title)
                .desc(hold.description)
                .repo(&tag)
                .button(
                    Act::Attend(hold.action, hold.reason.clone(), ROOT.to_owned()),
                    hold.label,
                    enabled,
                )
                .detail(&detail.iter().map(String::as_str).collect::<Vec<_>>())
                .group(counted(&hold.roots));
            assert_rows(
                &world.rows(),
                &[row],
                &format!("S3 {} {unlockable}", hold.name),
            );
        }
    }

    // A recovery hold: the policy holds the repository, the phase waits.
    let mut world = World::new();
    world.schedule_root(ROOT);
    world.schedule.set_policy(&ROOT.to_owned(), {
        let mut policy = world::policy();
        policy.recovery_suspended = true;
        policy
    });
    world.schedule.sync_recovery(&ROOT.to_owned());
    let reason = Reason::RecoveryHeld(ROOT.to_owned());
    let row = want(
        RowKey::Hold(format!("RecoveryHeld:{ROOT}")),
        Kind::Failure,
        "Checking demo is held until recovery",
    )
    .desc("Your work is safe.")
    .repo("demo")
    .button(
        Act::Attend(Action::ResumeChecking, reason, ROOT.to_owned()),
        "Resume checking",
        true,
    )
    .detail(&[status::HELD, "Holding back demo"])
    .group(counted(&[ROOT]));
    assert_rows(&world.rows(), &[row], "S3 recovery held");
}
