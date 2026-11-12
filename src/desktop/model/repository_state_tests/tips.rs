//! Part B's tooltip, state by state.

use super::*;

const COPY_LOCATION: &str = "ssh://git@example.invalid/team/repo.git";

fn copy(scene: &mut Scene) {
    scene.shared_copy = Some(SharedCopy {
        name: "origin".to_owned(),
        location: COPY_LOCATION.to_owned(),
    });
}

/// The tooltip's lines after the sentence, which every state shares.
fn facts_lines(branch: bool, shared: bool) -> Vec<String> {
    let mut lines = vec![format!("Folder: {ROOT}")];
    if branch {
        lines.push("Main branch: main".to_owned());
    }
    if shared {
        lines.push(format!("Shared copy: origin \u{b7} {COPY_LOCATION}"));
    }
    lines
}

fn tip(sentence: &str) -> Vec<String> {
    let mut lines = vec![sentence.to_owned()];
    lines.extend(facts_lines(true, false));
    lines
}

pub(super) struct Tip {
    pub name: &'static str,
    pub set: Set,
    pub titles: &'static [(u32, &'static str)],
    pub lines: Vec<String>,
}

fn tipped(name: &'static str, set: Set, lines: Vec<String>) -> Tip {
    Tip {
        name,
        set,
        titles: &[],
        lines,
    }
}

pub(super) fn ahead(n: u32) -> AttemptTarget {
    item(n, Disposition::LocalAhead)
}

const EACH: &str = "Share each from its tab or from Activity.";
const MAIN_SHARES: &str = "Share main shares main.";

pub(super) fn tips() -> Vec<Tip> {
    let unchanged = "Its files are unchanged. Retry, or remove it from Manyhands.";
    let message = ResultCode::RepositoryNotRegistered.message();
    let guidance = ProblemCode::InvalidPath.guidance();
    vec![
        tipped(
            "can't open, the kept row",
            |s| s.kept = true,
            tip(&format!(
                "alpha is no longer registered with Manyhands. {unchanged}"
            )),
        ),
        tipped(
            "can't open, unavailable",
            |s| s.unavailable(),
            tip(&format!("{message} {unchanged}")),
        ),
        tipped(
            "can't open, failed",
            |s| s.failed(),
            tip(&format!(
                "{} {unchanged}",
                ResultCode::RepositoryInaccessible.message()
            )),
        ),
        tipped(
            "busy",
            |s| s.resolution(Resolution::Busy),
            tip("Manyhands was busy opening alpha. Its files are unchanged. Retry."),
        ),
        tipped(
            "needs fixing",
            |s| {
                s.invalid(
                    ConfigurationState::Invalid,
                    vec![
                        problem(ProblemCode::InvalidPath),
                        problem(ProblemCode::InvalidPath),
                    ],
                );
            },
            tip(&format!(
                "Its configuration needs fixing: {guidance} {guidance}"
            )),
        ),
        tipped(
            "checking",
            |s| s.checking_now = true,
            tip("Checking alpha for updates\u{2026}"),
        ),
        tipped(
            "locked",
            |s| s.schedule().phase = attention(Reason::KeyLocked(key())),
            tip("Checking is waiting for your key to be unlocked."),
        ),
        tipped(
            "locked, stored",
            |s| s.stored = Some(PollingOutcome::UnlockRequired),
            tip("Checking is waiting for your key to be unlocked."),
        ),
        tipped(
            "needs attention, a reason",
            |s| s.schedule().phase = attention(Reason::KeyProblem(key())),
            tip("Choose a key"),
        ),
        tipped(
            "needs attention, a pending conflict",
            |s| s.schedule().conflict_pending = true,
            tip("Waiting for the conflict to be resolved"),
        ),
        tipped(
            "needs attention, held, with D6's detail",
            |s| {
                s.schedule().policy.recovery_suspended = true;
                s.holding = Some(Holding::Held);
            },
            tip(
                "Checking is held until recovery An interrupted check is holding this repository. Resume checking to finish it.",
            ),
        ),
        tipped(
            "needs attention, can't check",
            |s| s.schedule().phase = waiting(Why::Failed),
            tip("Can't check"),
        ),
        tipped(
            "needs attention, a stored outcome",
            |s| s.stored = Some(PollingOutcome::HostApprovalRequired),
            tip("Needs host approval"),
        ),
        tipped(
            "needs attention, unreadable",
            |s| {
                s.schedule = None;
                s.unplaced = Some(Unplaced::Unreadable);
            },
            tip("Can't check alpha for updates. Retry."),
        ),
        tipped(
            "needs attention, one problem",
            |s| s.checked(Vec::new(), vec![item(9, Disposition::Dirty)]),
            tip("1 item needs a look. Activity lists them."),
        ),
        tipped(
            "needs attention, problems",
            |s| {
                s.checked(
                    Vec::new(),
                    vec![
                        item(9, Disposition::Dirty),
                        item(8, Disposition::Conflicted),
                    ],
                );
            },
            tip("2 items need a look. Activity lists them."),
        ),
        tipped(
            "can't reach",
            |s| s.schedule().last = Some(attempt(unreachable())),
            tip("Can't reach alpha. Your work is safe."),
        ),
        tipped(
            "paused",
            |s| s.schedule().policy.paused = true,
            tip("Checking for updates is paused."),
        ),
        tipped(
            "up to date, as of the completed check while the last attempt was interrupted",
            |s| {
                s.checked(Vec::new(), Vec::new());
                s.schedule().last = Some(attempt(Answer::Interrupted));
            },
            tip("Checked 7 minutes ago \u{b7} up to date"),
        ),
        tipped(
            "this computer",
            |s| s.unscheduled(None),
            tip("No shared copy, so changes stay on this computer."),
        ),
        tipped(
            "not ssh",
            |s| s.unscheduled(Some("origin")),
            tip("origin isn't an SSH shared copy, so Manyhands can't check or share it."),
        ),
        tipped("not checked yet", |_| {}, tip("Not checked yet")),
        tipped(
            "unfinished operation",
            |s| s.schedule().phase = waiting(Why::Unfinished),
            tip("Waiting for an unfinished operation"),
        ),
        tipped(
            "polling off: no sentence",
            |s| {
                s.unscheduled(Some("origin"));
                s.polling = false;
            },
            facts_lines(true, false),
        ),
        // The facts after the sentence.
        tipped(
            "a shared copy, no branch",
            |s| {
                copy(s);
                s.entry.summary.configuration.primary_branch = None;
                s.checked(Vec::new(), Vec::new());
            },
            {
                let mut lines = vec!["Checked 7 minutes ago \u{b7} up to date".to_owned()];
                lines.extend(facts_lines(false, true));
                lines
            },
        ),
        tipped(
            "a shared copy and a branch",
            |s| {
                copy(s);
                s.schedule().policy.paused = true;
            },
            {
                let mut lines = vec!["Checking for updates is paused.".to_owned()];
                lines.extend(facts_lines(true, true));
                lines
            },
        ),
        // To share: what is named, and how.
        Tip {
            name: "to share, titled, untitled and kind-less",
            set: |s| {
                let mut kindless = ahead(3);
                kindless.kind = None;
                s.checked(vec![ahead(1), ahead(2), kindless], Vec::new());
            },
            titles: &[(1, "Fix login"), (3, "Never asked")],
            lines: tip(&format!(
                "Not shared yet: Fix login, {}, {}. {EACH}",
                ulid(2),
                ulid(3)
            )),
        },
        tipped(
            "to share, main alone",
            |s| s.checked(vec![main_ahead()], Vec::new()),
            tip(&format!("Not shared yet: main. {MAIN_SHARES}")),
        ),
        Tip {
            name: "to share, items and main",
            set: |s| s.checked(vec![main_ahead(), ahead(1)], Vec::new()),
            titles: &[(1, "Fix login")],
            lines: tip(&format!(
                "Not shared yet: main, Fix login. {EACH} {MAIN_SHARES}"
            )),
        },
        Tip {
            name: "to share, seven items: five named, then and 2 more",
            set: |s| s.checked((1..=7).map(ahead).collect(), Vec::new()),
            titles: &[
                (1, "A"),
                (2, "B"),
                (3, "C"),
                (4, "D"),
                (5, "E"),
                (6, "F"),
                (7, "G"),
            ],
            lines: tip(&format!("Not shared yet: A, B, C, D, E and 2 more. {EACH}")),
        },
    ]
}
