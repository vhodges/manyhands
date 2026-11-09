//! Reconfiguration (R-11), pause against attention (A3) and the attention
//! keys (decision 11).

use manyhands::repository::OperationId;

use super::*;

#[test]
fn reconfiguration_table() {
    type Setup = fn(&mut Fx);
    type Want = fn(&Phase) -> bool;
    let cases: Vec<(&str, Setup, Want)> = vec![
        ("idle", |_| {}, |phase| *phase == Phase::Idle),
        (
            "a hold",
            |fx| {
                fx.answer("r", Answer::ConflictPending, None);
            },
            |phase| *phase == Phase::Idle,
        ),
        (
            "a restart wait",
            |fx| {
                fx.answer("r", Answer::Busy, None);
            },
            |phase| *phase == Phase::Idle,
        ),
        (
            "a not-due wait",
            |fx| {
                fx.answer(
                    "r",
                    Answer::NotEligible(NotEligible::NotDue),
                    Some(fx.at(90)),
                );
            },
            |phase| *phase == Phase::Idle,
        ),
        (
            "attention",
            |fx| {
                let failure = TransportFailure::HostUnapproved { authority: None };
                fx.answer("r", Answer::Transport(failure), None);
            },
            |phase| *phase == Phase::Idle,
        ),
        (
            "preempted: the user lane decides",
            |fx| {
                fx.answer("r", Answer::Cancelled(CancelCause::Preemption), None);
            },
            |phase| *phase == Phase::Preempted,
        ),
        (
            "running: the caller cancels it",
            |fx| fx.schedule.mark_running(&root("r"), OperationId::new()),
            |phase| matches!(phase, Phase::Running { .. }),
        ),
        (
            "retiring",
            |fx| assert!(fx.schedule.begin_retiring(&root("r"))),
            |phase| *phase == Phase::Retiring,
        ),
        (
            "stopped",
            |fx| {
                fx.answer("r", Answer::Cancelled(CancelCause::Quit), None);
            },
            |phase| *phase == Phase::Stopped,
        ),
    ];
    for (name, setup, want) in cases {
        let mut fx = Fx::new();
        fx.add("r", policy(Some(fx.at(-5))));
        fx.add("other", policy(Some(fx.at(-5))));
        setup(&mut fx);
        let other = fx.phase("other");
        fx.schedule.reconfigure(&root("r"));
        assert!(want(&fx.phase("r")), "{name}: {:?}", fx.phase("r"));
        assert!(fx.repo("r").stale, "{name}: the policy is read again");
        assert_eq!(
            fx.schedule.restart_of(&root("r")),
            None,
            "{name}: a new operation"
        );
        assert_eq!(fx.repo("r").hold_until, None, "{name}: no hold");
        assert_eq!(
            fx.phase("other"),
            other,
            "{name}: other repositories are left alone"
        );
        assert!(
            !fx.repo("other").stale,
            "{name}: other policies are not stale"
        );
    }

    // Cancel on Quit's ask resumes a stopped repository (R-33).
    let mut fx = Fx::new();
    fx.add("r", policy(None));
    fx.answer("r", Answer::Cancelled(CancelCause::Quit), None);
    assert!(fx.due().is_empty());
    fx.schedule.resume_stopped();
    assert_eq!(fx.due(), vec![root("r")]);

    // The policy read again decides. A re-added remote leaves a stored
    // pause alone; a repository that was not paused polls.
    for (paused, due) in [(true, false), (false, true)] {
        let mut fx = Fx::new();
        fx.add(
            "r",
            Policy {
                paused,
                ..policy(Some(fx.at(-5)))
            },
        );
        fx.answer(
            "r",
            Answer::Transport(TransportFailure::ConfigurationRequired),
            None,
        );
        fx.schedule.reconfigure(&root("r"));
        assert!(
            fx.due().is_empty(),
            "paused={paused}: not before the policy is read"
        );
        fx.schedule.set_policy(
            &root("r"),
            Policy {
                paused,
                ..policy(Some(fx.at(-5)))
            },
        );
        assert_eq!(!fx.due().is_empty(), due, "paused={paused}");
        assert_eq!(fx.repo("r").policy.paused, paused);
    }

    // Removal drops the schedule.
    let mut fx = Fx::new();
    fx.add("r", policy(None));
    fx.schedule.remove(&root("r"));
    assert!(fx.schedule.repo(&root("r")).is_none() && fx.due().is_empty());
}

#[test]
fn pause_versus_attention() {
    let key = KeyId::new();
    let locked = Answer::Transport(TransportFailure::KeyLocked { key: Some(key) });
    let mut fx = Fx::new();
    fx.add("a", policy(None));
    fx.add(
        "b",
        Policy {
            paused: true,
            ..policy(None)
        },
    );
    let policies = [fx.repo("a").policy.clone(), fx.repo("b").policy.clone()];

    // Attention does not change the policy: a paused repository stays
    // paused, an unpaused one stays unpaused.
    fx.answer("a", locked.clone(), None);
    fx.answer("b", locked, None);
    assert_eq!(
        [fx.repo("a").policy.clone(), fx.repo("b").policy.clone()],
        policies
    );
    assert_eq!(fx.schedule.attention().len(), 1, "the key is one entry");

    // Pausing does not clear attention, and resuming does not either.
    for paused in [true, false, true, false] {
        fx.schedule.set_policy(
            &root("a"),
            Policy {
                paused,
                ..policy(None)
            },
        );
        fx.schedule.set_policy(
            &root("b"),
            Policy {
                paused,
                ..policy(None)
            },
        );
        assert_eq!(
            fx.schedule.attention()[0].roots,
            [root("a"), root("b")],
            "paused={paused}"
        );
        assert!(
            fx.due().is_empty(),
            "paused={paused}: held back all the same"
        );
    }

    // The unlock clears attention and leaves the pause as it is.
    fx.schedule.set_policy(
        &root("b"),
        Policy {
            paused: true,
            ..policy(None)
        },
    );
    let cleared = fx.schedule.clear_attention(&Reason::KeyLocked(key));
    assert_eq!(cleared, [root("a"), root("b")]);
    assert!(fx.schedule.attention().is_empty());
    assert_eq!(fx.due(), vec![root("a")], "b is still paused");
    assert!(fx.repo("b").policy.paused);

    // A pause is not attention: it never counts.
    fx.answer("b", Answer::NotEligible(NotEligible::Paused), None);
    assert!(fx.schedule.attention().is_empty());
}

#[test]
fn attention_keys() {
    let (one, two) = (KeyId::new(), KeyId::new());
    let host = |name: &str| Authority {
        host: name.to_owned(),
        port: 22,
    };
    let transport = |failure| Answer::Transport(failure);
    let mut fx = Fx::new();
    for name in ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"] {
        fx.add(name, policy(None));
    }
    let attention = [
        (
            "a",
            transport(TransportFailure::KeyLocked { key: Some(one) }),
        ),
        (
            "b",
            transport(TransportFailure::KeyLocked { key: Some(one) }),
        ),
        (
            "c",
            transport(TransportFailure::KeyLocked { key: Some(one) }),
        ),
        (
            "d",
            transport(TransportFailure::KeyLocked { key: Some(two) }),
        ),
        (
            "e",
            transport(TransportFailure::HostUnapproved {
                authority: Some(host("x")),
            }),
        ),
        (
            "f",
            transport(TransportFailure::HostUnapproved {
                authority: Some(host("x")),
            }),
        ),
        (
            "g",
            transport(TransportFailure::KeyProblem { key: Some(two) }),
        ),
        (
            "h",
            transport(TransportFailure::KeyProblem { key: Some(two) }),
        ),
        ("i", transport(TransportFailure::ConfigurationRequired)),
        ("j", Answer::NotEligible(NotEligible::RecoverySuspended)),
    ];
    for (name, answer) in attention {
        fx.answer(name, answer, None);
    }
    let entries: Vec<_> = fx
        .schedule
        .attention()
        .into_iter()
        .map(|entry| (entry.reason, entry.roots))
        .collect();
    let roots = |names: &[&str]| names.iter().map(|name| root(name)).collect::<Vec<_>>();
    assert_eq!(
        entries,
        vec![
            (Reason::KeyLocked(one), roots(&["a", "b", "c"])),
            (Reason::KeyLocked(two), roots(&["d"])),
            (Reason::HostUnapproved(host("x")), roots(&["e", "f"])),
            (Reason::KeyProblem(Some(two), root("g")), roots(&["g"])),
            (Reason::KeyProblem(Some(two), root("h")), roots(&["h"])),
            (Reason::ConfigurationRequired(root("i")), roots(&["i"])),
            (Reason::RecoveryHeld(root("j")), roots(&["j"])),
        ]
    );

    // A completed poll of one repository clears its own and no one else's.
    fx.answer("b", Answer::Completed, Some(fx.at(300)));
    assert_eq!(fx.schedule.attention()[0].roots, roots(&["a", "c"]));

    // What waits for itself, or is the person's own choice, is not attention.
    let mut quiet = Fx::new();
    let answers = [
        ("unreachable", transport(TransportFailure::Unreachable)),
        ("changed", transport(TransportFailure::Changed)),
        ("unavailable", transport(TransportFailure::Unavailable)),
        ("busy", Answer::Busy),
        ("interrupted", Answer::Interrupted),
        (
            "unfinished",
            Answer::Repository {
                recovery_required: true,
            },
        ),
        (
            "failed",
            Answer::Repository {
                recovery_required: false,
            },
        ),
        ("conflict", Answer::ConflictPending),
        ("paused", Answer::NotEligible(NotEligible::Paused)),
        ("off", Answer::NotEligible(NotEligible::Disabled)),
        ("not due", Answer::NotEligible(NotEligible::NotDue)),
        ("preempted", Answer::Cancelled(CancelCause::Preemption)),
        ("completed", Answer::Completed),
    ];
    for (name, answer) in answers {
        quiet.add(name, policy(None));
        quiet.answer(name, answer, Some(quiet.at(60)));
    }
    assert!(quiet.schedule.attention().is_empty());
}
