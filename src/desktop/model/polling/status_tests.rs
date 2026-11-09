//! A Settings row's words (D6 design A4's text column, C2, C4, L-2), as a
//! table over the model's own transitions: each row reaches its state by
//! the answer that gives it, not by writing the phase.

use manyhands::repository::OperationId;

use super::{Controls, Facts, Row, Shape, Unplaced, row};
use crate::desktop::{
    model::polling::{
        Answer, AttemptTarget, CancelCause, Clock, Disposition, KeyId, NotEligible, Phase,
        PollAttempt, TargetKind, TransportFailure,
        tests::{Fx, policy, root},
    },
    services::scheduler::Holding,
};

const NAME: &str = "alpha";

fn facts<'a>(fx: &'a Fx, checking_now: bool) -> Facts<'a> {
    Facts {
        name: NAME,
        schedule: fx.schedule.repo(&root("r")),
        unplaced: None,
        holding: None,
        checking_now,
        now: fx.clock.utc(),
    }
}

fn target(disposition: Disposition) -> AttemptTarget {
    AttemptTarget {
        item_id: None,
        kind: Some(TargetKind::Ticket),
        disposition,
    }
}

/// A repository `r` that answered, and `minutes` have passed since.
fn answered(answer: Answer, targets: Vec<AttemptTarget>, minutes: u64) -> Fx {
    let mut fx = Fx::new();
    fx.add("r", policy(Some(fx.at(-100))));
    let attempt = PollAttempt {
        root: root("r"),
        operation: OperationId::new(),
        completed_at: (answer == Answer::Completed).then(|| fx.clock.utc()),
        answer,
        next_eligible_at: Some(fx.at(300)),
        targets,
        index_pending: false,
    };
    fx.schedule.after(&fx.now(), &attempt);
    fx.clock
        .advance(std::time::Duration::from_secs(minutes * 60));
    fx
}

fn plain(answer: Answer) -> Fx {
    answered(answer, Vec::new(), 0)
}

fn text(fx: &Fx) -> String {
    row(&facts(fx, false)).status
}

fn retry(row: &Row) -> bool {
    matches!(&row.shape, Shape::Polling(Controls { retry: true, .. }))
}

/// A4's text column, row by row, from the answer that reaches it; the
/// phases that are not answers (queued, retiring) and the facts around the
/// model follow (C2, C4, L-2). The words are the doc's.
#[test]
fn status_text_table() {
    let key = KeyId::new();
    let transport = |failure| Answer::Transport(failure);
    let changes = |n| vec![target(Disposition::FastForwarded); n];
    let unreachable = "Can't reach alpha. Your work is safe.";
    // (name, state, status, offers Retry now)
    let rows: Vec<(&str, Fx, String, bool)> = vec![
        (
            "completed, nothing to bring in",
            plain(Answer::Completed),
            "Checked just now \u{b7} up to date".to_owned(),
            false,
        ),
        (
            "completed, changes brought in",
            answered(Answer::Completed, changes(2), 0),
            "Checked just now \u{b7} Brought in 2 changes".to_owned(),
            false,
        ),
        (
            "one change",
            answered(Answer::Completed, changes(1), 0),
            "Checked just now \u{b7} Brought in 1 change".to_owned(),
            false,
        ),
        (
            "local ahead",
            answered(Answer::Completed, vec![target(Disposition::LocalAhead)], 0),
            "Checked just now \u{b7} up to date \u{b7} 1 not shared yet".to_owned(),
            false,
        ),
        (
            "completed, minutes ago",
            answered(Answer::Completed, Vec::new(), 5),
            "Checked 5 minutes ago \u{b7} up to date".to_owned(),
            false,
        ),
        (
            "completed, a minute ago",
            answered(Answer::Completed, Vec::new(), 1),
            "Checked 1 minute ago \u{b7} up to date".to_owned(),
            false,
        ),
        (
            "completed, hours ago",
            answered(Answer::Completed, Vec::new(), 180),
            "Checked 3 hours ago \u{b7} up to date".to_owned(),
            false,
        ),
        (
            "completed, days ago",
            answered(Answer::Completed, Vec::new(), 60 * 24 * 2),
            "Checked 2 days ago \u{b7} up to date".to_owned(),
            false,
        ),
        (
            "transport unavailable",
            plain(transport(TransportFailure::Unreachable)),
            unreachable.to_owned(),
            true,
        ),
        (
            "key locked",
            plain(transport(TransportFailure::KeyLocked { key: Some(key) })),
            "Needs unlocking".to_owned(),
            false,
        ),
        (
            "key locked, no key named",
            plain(transport(TransportFailure::KeyLocked { key: None })),
            "Choose a key".to_owned(),
            false,
        ),
        (
            "key problem",
            plain(transport(TransportFailure::KeyProblem { key: Some(key) })),
            "Choose a key".to_owned(),
            false,
        ),
        (
            "host unapproved",
            plain(transport(TransportFailure::HostUnapproved {
                authority: None,
            })),
            "Needs host approval".to_owned(),
            false,
        ),
        (
            "configuration required",
            plain(transport(TransportFailure::ConfigurationRequired)),
            "Can't check".to_owned(),
            true,
        ),
        (
            "key source, host trust or endpoint changed",
            plain(transport(TransportFailure::Changed)),
            "Checking soon".to_owned(),
            false,
        ),
        (
            "registry unavailable",
            plain(transport(TransportFailure::Unavailable)),
            "Can't check".to_owned(),
            true,
        ),
        (
            "preempted by a user job",
            plain(Answer::Cancelled(CancelCause::Preemption)),
            "Checking soon".to_owned(),
            false,
        ),
        (
            "interrupted",
            plain(Answer::Interrupted),
            "Checking soon".to_owned(),
            false,
        ),
        (
            "busy, first step",
            plain(Answer::Busy),
            "Checking soon".to_owned(),
            false,
        ),
        (
            "busy, after the steps",
            {
                let mut fx = plain(Answer::Busy);
                for _ in 0..3 {
                    fx.answer("r", Answer::Busy, None);
                }
                fx
            },
            "Waiting for another operation to finish".to_owned(),
            false,
        ),
        (
            "interrupted, after the steps",
            {
                let mut fx = plain(Answer::Interrupted);
                for _ in 0..3 {
                    fx.answer("r", Answer::Interrupted, None);
                }
                fx
            },
            "Checking soon".to_owned(),
            false,
        ),
        (
            "a local operation is pending",
            plain(Answer::Repository {
                recovery_required: true,
            }),
            "Waiting for an unfinished operation".to_owned(),
            false,
        ),
        (
            "any other repository error",
            plain(Answer::Repository {
                recovery_required: false,
            }),
            "Can't check".to_owned(),
            true,
        ),
        (
            "operation mismatch",
            plain(Answer::OperationMismatch),
            "Can't check".to_owned(),
            true,
        ),
        (
            "conflict pending",
            plain(Answer::ConflictPending),
            "Waiting for the conflict to be resolved".to_owned(),
            false,
        ),
        (
            "not eligible, paused",
            plain(Answer::NotEligible(NotEligible::Paused)),
            "Paused".to_owned(),
            false,
        ),
        (
            "not eligible, disabled",
            plain(Answer::NotEligible(NotEligible::Disabled)),
            "Checking is off".to_owned(),
            false,
        ),
        (
            "not eligible, recovery suspended",
            plain(Answer::NotEligible(NotEligible::RecoverySuspended)),
            "Checking is held until recovery".to_owned(),
            false,
        ),
        (
            "not eligible, not due",
            {
                let mut fx = Fx::new();
                let (ahead, completed) = (fx.at(240), fx.at(-60));
                fx.schedule
                    .add(root("r"), policy(Some(ahead)), Some(completed));
                fx.answer("r", Answer::NotEligible(NotEligible::NotDue), Some(ahead));
                fx
            },
            "Checked 1 minute ago".to_owned(),
            false,
        ),
        (
            "quit stopped it",
            plain(Answer::Cancelled(CancelCause::Quit)),
            "Not checked yet".to_owned(),
            false,
        ),
    ];
    for (name, fx, status, retry_now) in &rows {
        let row = row(&facts(fx, false));
        assert_eq!(&row.status, status, "{name}");
        assert_eq!(retry(&row), *retry_now, "{name}: Retry now");
        // Only a wait for an unfinished operation links to its pending row.
        assert_eq!(
            row.pending,
            status == "Waiting for an unfinished operation",
            "{name}: link"
        );
    }

    // Never checked, and one restored from the library's stored time.
    let mut fx = Fx::new();
    fx.add("r", policy(None));
    assert_eq!(text(&fx), "Not checked yet");
    let restored = fx.at(-7200);
    fx.schedule.add(root("r"), policy(None), Some(restored));
    assert_eq!(text(&fx), "Checked 2 hours ago");
    // Queued or running, or this process's own Check now: checking.
    fx.schedule.mark_queued(&root("r"));
    assert_eq!(text(&fx), "Checking\u{2026}");
    fx.schedule.abandon(&root("r"));
    assert_eq!(
        row(&facts(&fx, true)).status,
        "Checking\u{2026}",
        "an own Check now"
    );
    assert!(matches!(
        row(&facts(&fx, true)).shape,
        Shape::Polling(Controls { checking: true, .. })
    ));
    assert!(matches!(
        row(&facts(&fx, false)).shape,
        Shape::Polling(Controls {
            checking: false,
            paused: false,
            interval_seconds: 300,
            ..
        })
    ));
    // Retiring (Part E) is not a poll, and the row says what runs.
    fx.schedule.begin_retiring(&root("r"));
    assert_eq!(fx.phase("r"), Phase::Retiring);
    assert_eq!(text(&fx), "Removing finished copies\u{2026}");

    // The policy decides before the phase: off, held, paused.
    let mut off = Fx::new();
    let mut stored = policy(None);
    stored.enabled = false;
    off.schedule.add(root("r"), stored, None);
    let off = row(&facts(&off, false));
    assert_eq!(
        (off.status.as_str(), &off.shape),
        ("Checking is off", &Shape::Off)
    );
    let mut held = Fx::new();
    let mut stored = policy(None);
    stored.recovery_suspended = true;
    held.schedule.add(root("r"), stored, None);
    let held = row(&facts(&held, false));
    assert_eq!(
        (held.status.as_str(), &held.shape),
        ("Checking is held until recovery", &Shape::Held)
    );
    let mut paused = plain(Answer::Completed);
    paused.schedule.set_policy(&root("r"), {
        let mut stored = policy(None);
        stored.paused = true;
        stored.interval = std::time::Duration::from_secs(900);
        stored
    });
    let paused = row(&facts(&paused, false));
    assert_eq!(paused.status, "Paused");
    assert_eq!(
        paused.shape,
        Shape::Polling(Controls {
            paused: true,
            interval_seconds: 900,
            retry: false,
            checking: false,
        })
    );

    // Pause and attention are independent (A3): both are said.
    let mut both = plain(transport(TransportFailure::KeyLocked { key: Some(key) }));
    both.schedule.set_policy(&root("r"), {
        let mut stored = policy(None);
        stored.paused = true;
        stored
    });
    assert_eq!(text(&both), "Paused \u{b7} Needs unlocking");

    // No schedule: the repository has no shared copy, or its remotes are
    // being read, or could not be.
    let none = Fx::new();
    let no_copy = Facts {
        schedule: None,
        ..facts(&none, false)
    };
    let no_copy = row(&no_copy);
    assert_eq!(
        (no_copy.status.as_str(), &no_copy.shape),
        ("No shared copy, nothing to check", &Shape::NoSharedCopy)
    );
    for (unplaced, words) in [
        (Unplaced::Reading, "Reading the remotes"),
        (Unplaced::Unreadable, "Can't check"),
    ] {
        let facts = Facts {
            schedule: None,
            unplaced: Some(unplaced),
            ..facts(&none, false)
        };
        let row = row(&facts);
        assert_eq!((row.status.as_str(), &row.shape), (words, &Shape::Pending));
    }
}

/// L-2: a repository an interrupted check holds says why it waits and what
/// finishes it, beside its status.
#[test]
fn status_detail_for_an_interrupted_check() {
    let fx = plain(Answer::Completed);
    for (holding, detail) in [
        (
            Holding::Paused,
            "An interrupted check is holding this repository. Turn checking on to finish it.",
        ),
        (
            Holding::Held,
            "An interrupted check is holding this repository. Resume checking to finish it.",
        ),
        (
            Holding::NotDue,
            "An interrupted check is holding this repository. It finishes at the next check.",
        ),
        (
            Holding::Off,
            "An interrupted check is holding this repository. It finishes once checking is on.",
        ),
        (
            Holding::Attention,
            "An interrupted check is holding this repository. It finishes once what needs your attention is dealt with.",
        ),
    ] {
        let facts = Facts {
            holding: Some(holding),
            ..facts(&fx, false)
        };
        assert_eq!(row(&facts).detail.as_deref(), Some(detail), "{holding:?}");
    }
    assert_eq!(row(&facts(&fx, false)).detail, None);
}
