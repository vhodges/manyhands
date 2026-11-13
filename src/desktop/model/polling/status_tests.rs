//! A Settings row's words (D6 design A4's text column, C2, C4, L-2), as a
//! table over the model's own transitions: each row reaches its state by
//! the answer that gives it, not by writing the phase.

use manyhands::{
    canonical::ItemId,
    repository::{OperationId, PollingOutcome},
};

use super::{Controls, Facts, Row, Shape, Unplaced, row};
use crate::desktop::{
    model::polling::{
        Answer, AttemptTarget, CancelCause, Clock, Disposition, KeyId, NotEligible, Phase,
        PollAttempt, TargetKind, TransportFailure, Why,
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
        stored: None,
        now: fx.clock.utc(),
    }
}

fn target(disposition: Disposition) -> AttemptTarget {
    AttemptTarget {
        item_id: None,
        kind: Some(TargetKind::Ticket),
        disposition,
        main: false,
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
            "conflict pending, then completed: the flag clears",
            {
                let mut fx = plain(Answer::ConflictPending);
                fx.answer("r", Answer::Completed, None);
                fx
            },
            "Checked just now \u{b7} up to date".to_owned(),
            false,
        ),
        (
            "conflict pending, then a failure: the flag clears",
            {
                let mut fx = plain(Answer::ConflictPending);
                fx.answer("r", transport(TransportFailure::Unreachable), None);
                fx
            },
            unreachable.to_owned(),
            true,
        ),
        (
            "conflict pending, then busy: the flag clears",
            {
                let mut fx = plain(Answer::ConflictPending);
                fx.answer("r", Answer::Busy, None);
                fx
            },
            "Checking soon".to_owned(),
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

    // The library's stored outcome speaks for a repository this process has
    // not checked (Part A, R-32): no attempt, not checking, checking on, not
    // held, not paused, idle, stopped or waiting for a deadline, and no
    // conflict pending. (status, offers Retry now)
    let bare = || {
        let mut fx = Fx::new();
        fx.add("r", policy(None));
        fx
    };
    let said = |fx: &Fx, outcome: Option<PollingOutcome>, checking_now: bool| {
        let row = row(&Facts {
            stored: outcome,
            ..facts(fx, checking_now)
        });
        (row.status.clone(), retry(&row))
    };
    use PollingOutcome::*;
    let stored_rows = [
        (TransportUnavailable, unreachable, true),
        (ProtocolRejected, unreachable, true),
        (UnlockRequired, "Needs unlocking", false),
        (SelectedKeyUnavailable, "Choose a key", false),
        (HostApprovalRequired, "Needs host approval", false),
        (ConfigurationRequired, "Can't check", true),
        (RepositoryUnavailable, "Can't check", true),
        // Today's text: nothing stored of a completed or cancelled check
        // that this process could say more of.
        (Completed, "Not checked yet", false),
        (Cancelled, "Not checked yet", false),
    ];
    for (outcome, words, retry_now) in stored_rows {
        assert_eq!(
            said(&bare(), Some(outcome), false),
            (words.to_owned(), retry_now),
            "{outcome:?}"
        );
    }
    assert_eq!(said(&bare(), None, false).0, "Not checked yet");
    // Waiting for a deadline, or stopped by a quit, is still no attempt.
    let mut not_due = Fx::new();
    let (ahead, completed) = (not_due.at(240), not_due.at(-60));
    not_due
        .schedule
        .add(root("r"), policy(Some(ahead)), Some(completed));
    not_due.answer("r", Answer::NotEligible(NotEligible::NotDue), Some(ahead));
    assert!(matches!(
        not_due.phase("r"),
        Phase::Waiting {
            why: Why::NotDue,
            ..
        }
    ));
    assert_eq!(
        said(&not_due, Some(TransportUnavailable), false),
        (unreachable.to_owned(), true)
    );
    assert_eq!(
        said(&not_due, Some(Completed), false).0,
        "Checked 1 minute ago",
        "the library's time is today's text"
    );
    let stopped = plain(Answer::Cancelled(CancelCause::Quit));
    assert_eq!(stopped.phase("r"), Phase::Stopped);
    assert_eq!(
        said(&stopped, Some(UnlockRequired), false),
        ("Needs unlocking".to_owned(), false)
    );

    // Where it does not apply, the row says what the phase says (P34, P43).
    let mut paused = bare();
    paused.schedule.set_policy(&root("r"), {
        let mut stored = policy(None);
        stored.paused = true;
        stored
    });
    let mut off = Fx::new();
    off.schedule.add(
        root("r"),
        {
            let mut stored = policy(None);
            stored.enabled = false;
            stored
        },
        None,
    );
    let mut held = Fx::new();
    held.schedule.add(
        root("r"),
        {
            let mut stored = policy(None);
            stored.recovery_suspended = true;
            stored
        },
        None,
    );
    let mut queued = bare();
    queued.schedule.mark_queued(&root("r"));
    let mut waiting = plain(Answer::Busy);
    for _ in 0..3 {
        waiting.answer("r", Answer::Busy, None);
    }
    let not_applying: Vec<(&str, Fx, bool, &str, bool)> = vec![
        ("paused", paused, false, "Paused", false),
        ("off", off, false, "Checking is off", false),
        (
            "held",
            held,
            false,
            "Checking is held until recovery",
            false,
        ),
        (
            "queued or running",
            queued,
            false,
            "Checking\u{2026}",
            false,
        ),
        ("an own Check now", bare(), true, "Checking\u{2026}", false),
        (
            "a busy wait, first step",
            plain(Answer::Busy),
            false,
            "Checking soon",
            false,
        ),
        (
            "a busy wait, after the steps",
            waiting,
            false,
            "Waiting for another operation to finish",
            false,
        ),
        (
            "interrupted",
            plain(Answer::Interrupted),
            false,
            "Checking soon",
            false,
        ),
        (
            "waiting for an unfinished operation",
            plain(Answer::Repository {
                recovery_required: true,
            }),
            false,
            "Waiting for an unfinished operation",
            false,
        ),
        (
            "a failed wait",
            plain(Answer::Repository {
                recovery_required: false,
            }),
            false,
            "Can't check",
            true,
        ),
        (
            "a pending conflict",
            plain(Answer::ConflictPending),
            false,
            "Waiting for the conflict to be resolved",
            false,
        ),
        (
            "attention",
            plain(transport(TransportFailure::HostUnapproved {
                authority: None,
            })),
            false,
            "Needs host approval",
            false,
        ),
        (
            "preempted",
            plain(Answer::Cancelled(CancelCause::Preemption)),
            false,
            "Checking soon",
            false,
        ),
        (
            "an attempt this session",
            plain(Answer::Completed),
            false,
            "Checked just now \u{b7} up to date",
            false,
        ),
    ];
    for (name, fx, checking_now, words, retry_now) in &not_applying {
        for outcome in [TransportUnavailable, UnlockRequired] {
            assert_eq!(
                said(fx, Some(outcome), *checking_now),
                (words.to_string(), *retry_now),
                "{name}: {outcome:?} does not apply"
            );
        }
        assert_eq!(
            said(fx, None, *checking_now),
            (words.to_string(), *retry_now),
            "{name}: as stored none"
        );
    }

    // "\u{b7} N not shared yet" follows the unshared list, whatever the
    // status, and before any completed check too (decision 24, P44).
    let ticket = Some(TargetKind::Ticket);
    let save = |fx: &mut Fx, count: usize| {
        let at = fx.at(1);
        for _ in 0..count {
            fx.schedule
                .saved(&root("r"), ItemId::generate(), ticket, false, at);
        }
    };
    let mut before_any = bare();
    save(&mut before_any, 1);
    let mut two = bare();
    save(&mut two, 2);
    let mut stored_and_saved = bare();
    save(&mut stored_and_saved, 1);
    let mut saved_paused = a_paused_repository();
    save(&mut saved_paused, 1);
    let mut saved_checking = bare();
    save(&mut saved_checking, 1);
    saved_checking.schedule.mark_queued(&root("r"));
    let mut failed_after = answered(Answer::Completed, vec![target(Disposition::LocalAhead)], 0);
    failed_after.answer("r", Answer::Interrupted, None);
    let brought = answered(
        Answer::Completed,
        vec![
            target(Disposition::FastForwarded),
            target(Disposition::FastForwarded),
            target(Disposition::LocalAhead),
        ],
        0,
    );
    let mut shared = answered(
        Answer::Completed,
        vec![AttemptTarget {
            item_id: None,
            kind: None,
            disposition: Disposition::LocalAhead,
            main: true,
        }],
        0,
    );
    let at = shared.at(1);
    assert!(shared.schedule.shared(&root("r"), None, true, at));
    let mut saved_off = Fx::new();
    saved_off.schedule.add(
        root("r"),
        {
            let mut stored = policy(None);
            stored.enabled = false;
            stored
        },
        None,
    );
    save(&mut saved_off, 1);
    let mut saved_held = Fx::new();
    saved_held.schedule.add(
        root("r"),
        {
            let mut stored = policy(None);
            stored.recovery_suspended = true;
            stored
        },
        None,
    );
    save(&mut saved_held, 1);
    let counted: Vec<(&str, Fx, Option<PollingOutcome>, String, bool)> = vec![
        (
            "before any check",
            before_any,
            None,
            "Not checked yet \u{b7} 1 not shared yet".to_owned(),
            false,
        ),
        (
            "two",
            two,
            None,
            "Not checked yet \u{b7} 2 not shared yet".to_owned(),
            false,
        ),
        (
            "with a stored outcome",
            stored_and_saved,
            Some(TransportUnavailable),
            format!("{unreachable} \u{b7} 1 not shared yet"),
            true,
        ),
        (
            "paused",
            saved_paused,
            None,
            "Paused \u{b7} 1 not shared yet".to_owned(),
            false,
        ),
        (
            "checking",
            saved_checking,
            None,
            "Checking\u{2026} \u{b7} 1 not shared yet".to_owned(),
            false,
        ),
        (
            "a failed check leaves the list",
            failed_after,
            None,
            "Checking soon \u{b7} 1 not shared yet".to_owned(),
            false,
        ),
        (
            "with changes brought in",
            brought,
            None,
            "Checked just now \u{b7} Brought in 2 changes \u{b7} 1 not shared yet".to_owned(),
            false,
        ),
        (
            "a share takes it off",
            shared,
            None,
            "Checked just now \u{b7} up to date".to_owned(),
            false,
        ),
        (
            "off",
            saved_off,
            None,
            "Checking is off \u{b7} 1 not shared yet".to_owned(),
            false,
        ),
        (
            "held",
            saved_held,
            None,
            "Checking is held until recovery \u{b7} 1 not shared yet".to_owned(),
            false,
        ),
    ];
    for (name, fx, outcome, words, retry_now) in &counted {
        assert_eq!(
            said(fx, *outcome, false),
            (words.clone(), *retry_now),
            "{name}"
        );
    }
}

/// A repository whose policy is paused.
fn a_paused_repository() -> Fx {
    let mut fx = Fx::new();
    fx.add("r", {
        let mut stored = policy(None);
        stored.paused = true;
        stored
    });
    fx
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
