use std::collections::{BTreeMap, BTreeSet};

use super::*;

// Independent copy of the approved design's five-row catalog.
const DESIGN: &[(&str, &[&str])] = &[
    (
        "C1",
        &[
            "repo list",
            "repo inspect",
            "repo identity",
            "remote list",
            "key list",
            "key show",
            "key public",
            "host list",
            "host inspect",
            "document list",
            "document show",
            "folder list",
            "ticket list",
            "ticket show",
            "ticket ready",
            "ticket blocked",
            "ticket deps",
            "ticket children",
            "ticket cycles",
            "ticket plan",
            "ticket critical-path",
            "ticket find",
            "comment list",
            "index status",
            "poll status",
            "operation list",
            "operation show",
            "id new",
        ],
    ),
    (
        "C2",
        &[
            "repo create",
            "repo enable",
            "repo remove",
            "repo identity-set",
            "remote add",
            "remote remove",
            "remote select",
            "key generate",
            "key import",
            "key select",
            "key clear",
            "key remove",
            "key delete",
            "host approve",
            "host replace",
            "host observe",
            "request show",
            "request cancel",
        ],
    ),
    (
        "C3",
        &[
            "folder create",
            "document create",
            "document save",
            "document move",
            "document repair",
            "ticket create",
            "ticket save",
            "ticket repair",
            "ticket slug-assign",
            "index refresh",
            "index rebuild",
            "path observe",
            "ticket check",
        ],
    ),
    (
        "C4",
        &[
            "comment add",
            "item sync",
            "repo sync",
            "conflict show",
            "conflict resolve",
            "operation resume",
            "operation abandon",
        ],
    ),
    (
        "C5",
        &[
            "document promote",
            "ticket close",
            "poll once",
            "poll configure",
            "poll pause",
            "poll resume",
        ],
    ),
];

#[test]
fn catalog_matches_every_command_and_delivering_cycle_without_duplicates() {
    let expected: BTreeMap<_, _> = DESIGN
        .iter()
        .flat_map(|(cycle, commands)| {
            commands
                .iter()
                .map(move |command| (command.to_string(), *cycle))
        })
        .collect();
    let actual: BTreeMap<_, _> = COMMANDS
        .iter()
        .map(|command| {
            let cycle = match command.availability {
                Availability::Available(cycle) => cycle,
                Availability::Planned(cycle) => cycle,
            };
            (format!("{} {}", command.resource, command.verb), cycle)
        })
        .collect();
    assert_eq!(actual.len(), COMMANDS.len(), "duplicate command");
    assert_eq!(actual, expected);
}

#[test]
fn catalog_repository_rules_and_planned_metadata_match_the_design() {
    assert_eq!(REPOSITORY_VALUES, &["required", "refused"]);
    assert!(!COMMANDS.is_empty());
    for command in COMMANDS {
        assert!(!command.summary.is_empty());
        match command.availability {
            Availability::Available(_) => {
                let refused = matches!(
                    (command.resource, command.verb),
                    ("repo", "list") | ("key", _) | ("host", _) | ("id", "new")
                );
                assert_eq!(
                    command.repo,
                    Some(if refused {
                        RepoRule::Refused
                    } else {
                        RepoRule::Required
                    })
                );
            }
            Availability::Planned(_) => {
                assert_eq!(command.repo, None);
                assert!(command.options.is_empty());
            }
        }
    }
}

#[test]
fn catalog_lookup_and_resource_selection_are_exact() {
    for (_, commands) in DESIGN {
        for name in *commands {
            let (resource, verb) = name.split_once(' ').unwrap();
            let command = find(resource, verb).expect(name);
            assert_eq!((command.resource, command.verb), (resource, verb));
            let selected = resource_commands(resource);
            assert!(selected.iter().any(|entry| entry.verb == verb));
            assert!(selected.iter().all(|entry| entry.resource == resource));
            assert_eq!(
                selected.len(),
                COMMANDS
                    .iter()
                    .filter(|entry| entry.resource == resource)
                    .count()
            );
        }
    }
    assert!(find("ticket", "frobnicate").is_none());
    assert!(find("Ticket", "list").is_none());
    assert!(resource_commands("frobnicate").is_empty());
}

#[test]
fn catalog_publishes_only_the_available_verbs_options_and_requirements() {
    for name in DESIGN[0].1 {
        let (resource, verb) = name.split_once(' ').unwrap();
        let command = find(resource, verb).expect(name);
        let mut expected = Vec::new();
        if command.repo == Some(RepoRule::Required) {
            expected.push(("--repo", Some("path"), true));
        }
        match (resource, verb) {
            ("key", "show" | "public") => expected.push(("--key-id", Some("ULID"), true)),
            ("host", "inspect") => expected.push(("--authority", Some("host[:port]"), true)),
            ("document" | "ticket", "show") => expected.extend([
                ("--id", Some("ULID"), false),
                ("--path", Some("path"), false),
                ("--context", Some("path"), false),
            ]),
            ("comment", "list") | ("ticket", "deps" | "children") => {
                expected.push(("--id", Some("ULID"), true))
            }
            ("operation", "show") => expected.push(("--operation-id", Some("ULID"), true)),
            ("ticket", "find") => expected.push(("--slug", Some("code"), true)),
            _ => {}
        }
        if resource == "ticket" && matches!(verb, "list" | "ready" | "blocked" | "plan") {
            expected.extend([
                ("--status", Some("text"), false),
                ("--type", Some("text"), false),
                ("--project", Some("text"), false),
            ]);
        }
        if (resource, verb) == ("ticket", "list") {
            expected.extend([
                ("--closure", Some("open|closed|all"), false),
                ("--slug", Some("code"), false),
                ("--ready", None, false),
                ("--blocked", None, false),
            ]);
        }
        if (resource, verb) == ("ticket", "deps") {
            expected.extend([
                ("--direction", Some("down|up|both"), false),
                ("--depth", Some("n"), false),
            ]);
        }
        let actual: BTreeSet<_> = command
            .options
            .iter()
            .map(|option| {
                assert!(!option.summary.is_empty());
                (option.name, option.value, option.required)
            })
            .collect();
        assert_eq!(
            actual.len(),
            command.options.len(),
            "duplicate option for {name}"
        );
        assert_eq!(actual, expected.into_iter().collect(), "options for {name}");
    }
}

#[test]
fn delivered_by_reads_the_cycle_from_either_availability() {
    for (cycle, commands) in &DESIGN[..2] {
        for name in *commands {
            let (resource, verb) = name.split_once(' ').unwrap();
            let spec = find(resource, verb).unwrap();
            assert_eq!(spec.delivered_by(), *cycle, "{name}");
            if *cycle == "C1" {
                assert_eq!(spec.availability, Availability::Available("C1"));
                assert_eq!(spec.kind, Kind::Read);
            }
        }
    }
    for availability in [Availability::Planned("C2"), Availability::Available("C2")] {
        let spec = CommandSpec {
            availability,
            ..*find("repo", "create").unwrap()
        };
        assert_eq!(spec.delivered_by(), "C2");
    }
}

#[test]
fn prompts_is_true_only_for_mutations_and_unlocking_reads() {
    for (kind, expected) in [
        (Kind::Read, false),
        (Kind::UnlockingRead, true),
        (
            Kind::Mutation {
                confirmed: true,
                input: InputRule::Required,
            },
            true,
        ),
        (
            Kind::Mutation {
                confirmed: false,
                input: InputRule::Refused,
            },
            true,
        ),
    ] {
        let spec = CommandSpec {
            kind,
            ..*find("id", "new").unwrap()
        };
        assert_eq!(spec.prompts(), expected, "{kind:?}");
    }
}
