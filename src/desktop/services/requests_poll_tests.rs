//! The poll intents (D6 Task 6): what each builds. How the worker and the
//! scheduler treat them is `requests_poll_worker_tests` and
//! `scheduler_intent_tests`.

use std::path::PathBuf;

use manyhands::repository::Mutation;

use super::Intent;
use crate::desktop::services::{JobKind, KeyStoreUnavailable};

/// Check now and the three policy verbs each build the library command of
/// the same name, over their repository, with no key store asked for, no
/// confirmation and no secret (Fact 14, C1).
#[test]
fn intent_poll_build() {
    let root = PathBuf::from("/repo");
    let cases = [
        (
            Intent::PollOnce { root: root.clone() },
            "poll once",
            JobKind::PollOnce,
            "Check now",
        ),
        (
            Intent::PollConfigure {
                root: root.clone(),
                interval_seconds: 900,
            },
            "poll configure",
            JobKind::PollConfigure,
            "Set check interval",
        ),
        (
            Intent::PollPause { root: root.clone() },
            "poll pause",
            JobKind::PollPause,
            "Pause checking",
        ),
        (
            Intent::PollResume { root: root.clone() },
            "poll resume",
            JobKind::PollResume,
            "Resume checking",
        ),
    ];
    let mut kinds = Vec::new();
    for (intent, command, kind, label) in cases {
        // The store closure is never asked: none is available.
        let mutation = intent
            .build(None, || Err(KeyStoreUnavailable))
            .ok()
            .unwrap_or_else(|| panic!("{command} builds"));
        assert_eq!(mutation.command(), command);
        match (&intent, &mutation) {
            (Intent::PollOnce { .. }, Mutation::PollOnce(input)) => {
                assert_eq!(input.root, root);
            }
            (
                Intent::PollConfigure { .. },
                Mutation::PollConfigure(manyhands::repository::PollConfigureInput {
                    root: built,
                    interval_seconds,
                }),
            ) => {
                assert_eq!(built, &root);
                // The interval the person chose, not a default.
                assert_eq!(*interval_seconds, 900);
            }
            (Intent::PollPause { .. }, Mutation::PollPause(input)) => {
                assert_eq!(input.root, root);
            }
            (Intent::PollResume { .. }, Mutation::PollResume(input)) => {
                assert_eq!(input.root, root);
            }
            _ => panic!("{command} built another command"),
        }
        assert_eq!(intent.job_kind(), kind);
        assert!(!kinds.contains(&kind), "{command}: a kind of its own");
        kinds.push(kind);
        assert_eq!(intent.label(), label);
        assert_eq!(intent.root(), Some(root.as_path()));
        assert!(!intent.confirmed(), "{command} needs no confirmation");
        assert_eq!(intent.onboarded_root(), None);
        // Only a poll brings items in; a policy write changes none.
        assert_eq!(
            intent.changes_items(),
            matches!(intent, Intent::PollOnce { .. }),
            "{command}"
        );
    }
}
