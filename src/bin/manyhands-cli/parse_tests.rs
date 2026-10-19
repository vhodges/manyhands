use super::*;
use crate::catalog::{self, Availability};

fn invoke(args: &[&str]) -> Invocation {
    parse(
        std::iter::once("manyhands-cli")
            .chain(args.iter().copied())
            .map(OsString::from)
            .collect(),
    )
}

fn run(args: &[&str]) -> clap::ArgMatches {
    let invocation = invoke(args);
    match invocation.action {
        Action::Run {
            command,
            args: matches,
        } => {
            let words: Vec<_> = args
                .iter()
                .copied()
                .filter(|arg| !arg.starts_with('-'))
                .take(2)
                .collect();
            assert_eq!([command.resource, command.verb], words.as_slice());
            matches
        }
        other => panic!("expected Run for {args:?}, got {other:?}"),
    }
}

fn usage(args: &[&str], expected: Option<(&str, &str)>) -> Invocation {
    let invocation = invoke(args);
    match &invocation.action {
        Action::Usage { command, detail } => {
            assert_eq!(
                command.map(|command| (command.resource, command.verb)),
                expected,
                "{args:?}"
            );
            assert!(!detail.is_empty());
        }
        other => panic!("expected Usage for {args:?}, got {other:?}"),
    }
    invocation
}

fn help(args: &[&str], expected: &str) -> Invocation {
    let invocation = invoke(args);
    let target = match &invocation.action {
        Action::Help(HelpTarget::Root) => "root".to_owned(),
        Action::Help(HelpTarget::Resource(resource)) => resource.to_string(),
        Action::Help(HelpTarget::Command(command)) => {
            format!("{} {}", command.resource, command.verb)
        }
        other => panic!("expected Help for {args:?}, got {other:?}"),
    };
    assert_eq!(target, expected, "{args:?}");
    invocation
}

// Minimum successful invocations, independently enumerated from the design.
const AVAILABLE: &[&[&str]] = &[
    &["repo", "list"],
    &["repo", "inspect", "--repo", "r"],
    &["repo", "identity", "--repo", "r"],
    &["remote", "list", "--repo", "r"],
    &["key", "list"],
    &["key", "show", "--key-id", "K"],
    &["key", "public", "--key-id", "K"],
    &["host", "list"],
    &["host", "inspect", "--authority", "example.com"],
    &["document", "list", "--repo", "r"],
    &["document", "show", "--repo", "r", "--id", "I"],
    &["folder", "list", "--repo", "r"],
    &["ticket", "list", "--repo", "r"],
    &["ticket", "show", "--repo", "r", "--id", "I"],
    &["ticket", "ready", "--repo", "r"],
    &["ticket", "blocked", "--repo", "r"],
    &["ticket", "deps", "--repo", "r", "--id", "I"],
    &["ticket", "children", "--repo", "r", "--id", "I"],
    &["ticket", "cycles", "--repo", "r"],
    &["ticket", "plan", "--repo", "r"],
    &["ticket", "critical-path", "--repo", "r"],
    &["ticket", "find", "--repo", "r", "--slug", "X"],
    &["comment", "list", "--repo", "r", "--id", "I"],
    &["index", "status", "--repo", "r"],
    &["poll", "status", "--repo", "r"],
    &["operation", "list", "--repo", "r"],
    &["operation", "show", "--repo", "r", "--operation-id", "O"],
    &["id", "new"],
];

#[test]
fn every_available_verb_accepts_its_minimum_inputs_and_requires_each_one() {
    assert_eq!(
        AVAILABLE.len(),
        catalog::COMMANDS
            .iter()
            .filter(|command| command.availability == Availability::Available)
            .count()
    );
    for args in AVAILABLE {
        run(args);
        for option in (2..args.len()).step_by(2) {
            let mut missing = args.to_vec();
            missing.drain(option..option + 2);
            usage(&missing, Some((args[0], args[1])));
        }
    }
}

#[test]
fn global_verbs_refuse_repository_selection() {
    for args in AVAILABLE.iter().filter(|args| !args.contains(&"--repo")) {
        let mut refused = args.to_vec();
        refused.extend(["--repo", "r"]);
        usage(&refused, Some((args[0], args[1])));
    }
}

#[test]
fn show_requires_exactly_one_selector_and_context_requires_path() {
    for resource in ["document", "ticket"] {
        run(&[resource, "show", "--repo", "r", "--path", "a.md"]);
        run(&[
            resource,
            "show",
            "--repo",
            "r",
            "--path",
            "a.md",
            "--context",
            "worktree",
        ]);
        for selectors in [
            vec![],
            vec!["--id", "I", "--path", "a.md"],
            vec!["--context", "worktree"],
            vec!["--id", "I", "--context", "worktree"],
        ] {
            let mut args = vec![resource, "show", "--repo", "r"];
            args.extend(selectors);
            usage(&args, Some((resource, "show")));
        }
    }
}

#[test]
fn readiness_flags_conflict_only_when_both_are_set() {
    for flag in ["--ready", "--blocked"] {
        assert!(run(&["ticket", "list", "--repo", "r", flag]).get_flag(&flag[2..]));
    }
    usage(
        &["ticket", "list", "--repo", "r", "--ready", "--blocked"],
        Some(("ticket", "list")),
    );
}

#[test]
fn bare_invocations_and_explicit_help_choose_the_right_target() {
    help(&[], "root");
    assert!(matches!(
        parse(vec![]).action,
        Action::Help(HelpTarget::Root)
    ));
    help(&["--help"], "root");
    help(&["ticket"], "ticket");
    help(&["ticket", "--help"], "ticket");
    assert!(help(&["ticket", "list", "--help", "--json"], "ticket list").json);
    help(&["ticket", "deps", "--help"], "ticket deps");
    for command in catalog::COMMANDS {
        help(
            &[command.resource, command.verb, "--help"],
            &format!("{} {}", command.resource, command.verb),
        );
    }
}

#[test]
fn help_wins_over_errors_version_and_per_verb_validation() {
    help(&["frobnicate", "--help"], "root");
    help(&["ticket", "frobnicate", "--help"], "ticket");
    help(&["ticket", "list", "--bogus", "--help"], "ticket list");
    help(&["ticket", "list", "--repo", "--help"], "ticket list");
    help(&["--version", "--help"], "root");
    help(
        &["ticket", "list", "--ready", "--blocked", "--help"],
        "ticket list",
    );
    assert!(help(&["--json", "frobnicate", "--help"], "root").json);
}

#[test]
fn version_precedes_bare_resource_planned_and_validation_but_not_clap_errors() {
    for args in [
        vec!["--version"],
        vec!["ticket", "--version"],
        vec!["ticket", "deps", "--version"],
        vec!["ticket", "close", "--version"],
    ] {
        assert!(matches!(invoke(&args).action, Action::Version), "{args:?}");
    }
    assert!(invoke(&["--json", "--version"]).json);
    usage(&["--version", "frobnicate"], None);
}

#[test]
fn planned_verbs_accept_future_options_and_raw_json_and_help() {
    for command in catalog::COMMANDS
        .iter()
        .filter(|command| matches!(command.availability, Availability::Planned(_)))
    {
        let invocation = invoke(&[
            command.resource,
            command.verb,
            "--id",
            "X",
            "--prepare",
            "--json",
        ]);
        assert!(invocation.json);
        match invocation.action {
            Action::Unavailable { command: actual } => assert_eq!(
                (actual.resource, actual.verb),
                (command.resource, command.verb)
            ),
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }
    help(&["ticket", "close", "--id", "X", "--help"], "ticket close");
    let invocation = invoke(&["ticket", "close", "--id", "X", "--", "--json", "--help"]);
    assert!(!invocation.json);
    assert!(matches!(invocation.action, Action::Unavailable { .. }));
}

#[test]
fn usage_errors_name_only_known_commands_and_keep_raw_json() {
    usage(&["frobnicate"], None);
    usage(&["ticket", "frobnicate"], None);
    usage(&["ticket", "list", "--bogus"], Some(("ticket", "list")));
    usage(&["ticket", "list", "--repo"], Some(("ticket", "list")));
    assert!(
        usage(
            &["--json", "--non-interactive", "ticket", "list", "--bogus"],
            Some(("ticket", "list"))
        )
        .json
    );
    assert!(usage(&["frobnicate", "--json"], None).json);
    assert!(!usage(&["frobnicate", "--", "--json", "--help"], None).json);
}

#[test]
fn mutation_options_and_unlisted_inputs_are_refused_on_available_verbs() {
    for args in AVAILABLE {
        for suffix in [
            vec!["--input", "x"],
            vec!["--request-id", "X"],
            vec!["--prepare"],
            vec!["--confirm", "X"],
            vec!["--bogus"],
        ] {
            let mut refused = args.to_vec();
            refused.extend(suffix);
            usage(&refused, Some((args[0], args[1])));
        }
    }
    usage(
        &["ticket", "list", "--request-id", "X"],
        Some(("ticket", "list")),
    );
    for (resource, verb, option, value) in [
        ("repo", "inspect", "--id", "I"),
        ("document", "list", "--status", "open"),
        ("ticket", "plan", "--closure", "all"),
        ("ticket", "ready", "--slug", "X"),
        ("ticket", "children", "--depth", "2"),
        ("ticket", "critical-path", "--project", "p"),
    ] {
        usage(
            &[resource, verb, "--repo", "r", option, value],
            Some((resource, verb)),
        );
    }
}

#[test]
fn only_the_four_globals_can_precede_the_verb_and_help_is_not_a_subcommand() {
    for args in [
        vec!["--repo", "r", "ticket", "list"],
        vec!["ticket", "--repo", "r", "list"],
    ] {
        usage(&args, None);
    }
    usage(&["help"], None);
    usage(&["ticket", "help"], None);
    for global in ["--json", "--non-interactive"] {
        run(&[global, "ticket", "list", "--repo", "r"]);
        run(&["ticket", global, "list", "--repo", "r"]);
        run(&["ticket", "list", "--repo", "r", global]);
    }
}

#[test]
fn repeated_flags_and_values_keep_the_last_value() {
    assert!(invoke(&["--json", "ticket", "list", "--repo", "r", "--json"]).json);
    run(&[
        "--json",
        "--json",
        "ticket",
        "list",
        "--repo",
        "r",
        "--non-interactive",
        "--non-interactive",
    ]);
    assert_eq!(
        run(&["ticket", "show", "--repo", "r", "--id", "A", "--id", "B"])
            .get_one::<String>("id")
            .unwrap(),
        "B"
    );
    assert_eq!(
        run(&["ticket", "list", "--repo", "a", "--repo", "b"])
            .get_one::<String>("repo")
            .unwrap(),
        "b"
    );
    assert_eq!(
        run(&[
            "ticket", "deps", "--repo", "r", "--id", "I", "--depth", "1", "--depth", "2"
        ])
        .get_one::<u32>("depth"),
        Some(&2)
    );
}

#[test]
fn filters_keep_exact_text_and_slug_is_not_validated() {
    for verb in ["list", "ready", "blocked", "plan"] {
        let args = run(&[
            "ticket",
            verb,
            "--repo",
            "r",
            "--status",
            "In Progress",
            "--type",
            "BUG",
            "--project",
            "é",
        ]);
        assert_eq!(args.get_one::<String>("status").unwrap(), "In Progress");
        assert_eq!(args.get_one::<String>("type").unwrap(), "BUG");
        assert_eq!(args.get_one::<String>("project").unwrap(), "é");
    }
    for verb in ["list", "find"] {
        for slug in ["", "not a valid code"] {
            assert_eq!(
                run(&["ticket", verb, "--repo", "r", "--slug", slug])
                    .get_one::<String>("slug")
                    .unwrap(),
                slug
            );
        }
    }
}

#[test]
fn closure_and_direction_use_value_lists_and_defaults() {
    assert_eq!(
        run(&["ticket", "list", "--repo", "r"])
            .get_one::<String>("closure")
            .unwrap(),
        "all"
    );
    assert_eq!(
        run(&["ticket", "deps", "--repo", "r", "--id", "I"])
            .get_one::<String>("direction")
            .unwrap(),
        "down"
    );
    for closure in ["open", "closed", "all"] {
        assert_eq!(
            run(&["ticket", "list", "--repo", "r", "--closure", closure])
                .get_one::<String>("closure")
                .unwrap(),
            closure
        );
    }
    for direction in ["down", "up", "both"] {
        assert_eq!(
            run(&[
                "ticket",
                "deps",
                "--repo",
                "r",
                "--id",
                "I",
                "--direction",
                direction
            ])
            .get_one::<String>("direction")
            .unwrap(),
            direction
        );
    }
    for invalid in ["", "ALL", "other"] {
        usage(
            &["ticket", "list", "--repo", "r", "--closure", invalid],
            Some(("ticket", "list")),
        );
        usage(
            &[
                "ticket",
                "deps",
                "--repo",
                "r",
                "--id",
                "I",
                "--direction",
                invalid,
            ],
            Some(("ticket", "deps")),
        );
    }
}

#[test]
fn depth_is_an_optional_u32_including_zero() {
    assert_eq!(
        run(&["ticket", "deps", "--repo", "r", "--id", "I"]).get_one::<u32>("depth"),
        None
    );
    for (text, value) in [("0", 0), ("42", 42), ("4294967295", u32::MAX)] {
        assert_eq!(
            run(&[
                "ticket", "deps", "--repo", "r", "--id", "I", "--depth", text
            ])
            .get_one::<u32>("depth"),
            Some(&value)
        );
    }
    for invalid in ["-1", "4294967296", "1.5", "x", ""] {
        usage(
            &[
                "ticket", "deps", "--repo", "r", "--id", "I", "--depth", invalid,
            ],
            Some(("ticket", "deps")),
        );
    }
}

#[test]
fn raw_flag_scans_are_exact_and_stop_at_the_separator() {
    for (flag, scan) in [
        ("--json", json_requested as fn(&[OsString]) -> bool),
        ("--help", help_requested as fn(&[OsString]) -> bool),
    ] {
        for args in [
            vec![flag],
            vec!["ticket", "close", "--id", "X", flag],
            vec![flag, "--"],
        ] {
            assert!(scan(&args.iter().map(OsString::from).collect::<Vec<_>>()));
        }
        for args in [
            vec![],
            vec!["--", flag],
            vec!["--json=1"],
            vec!["--help=1"],
            vec!["--hel"],
            vec!["--jso"],
            vec!["-h"],
        ] {
            assert!(!scan(&args.iter().map(OsString::from).collect::<Vec<_>>()));
        }
    }
    for flag in ["--json=1", "--help=1", "--hel", "--jso", "-h", "-V"] {
        assert!(!usage(&[flag], None).json);
    }
}

#[test]
fn authorities_default_the_port_and_strip_ipv6_brackets() {
    for (text, host, port) in [
        ("example.com", "example.com", 22),
        ("example.com:2222", "example.com", 2222),
        ("EXAMPLE.com:1", "EXAMPLE.com", 1),
        ("host:65535", "host", 65535),
        ("[::1]", "::1", 22),
        ("[::1]:22", "::1", 22),
        ("[::1]:2222", "::1", 2222),
    ] {
        assert_eq!(authority(text).unwrap(), (host.to_owned(), port));
        assert_eq!(
            run(&["host", "inspect", "--authority", text])
                .get_one::<String>("authority")
                .unwrap(),
            text
        );
    }
}

#[test]
fn malformed_authorities_are_usage_errors_before_dispatch() {
    for text in [
        "host:0",
        "host:70000",
        ":22",
        "[::1",
        "::1",
        "",
        "[]",
        "[host]",
        "[::1]suffix",
        "[::1]:",
        "host:",
        "host:x",
        "host:-1",
        "host:22:33",
        "host]",
        "[::1]:0",
        "user@example.com",
        "user@example.com:2222",
        "example.com/path",
        "example.com/path:2222",
    ] {
        assert!(authority(text).is_err(), "{text}");
        usage(
            &["host", "inspect", "--authority", text],
            Some(("host", "inspect")),
        );
    }
}

#[test]
fn clap_error_usage_uses_the_fixed_binary_name_without_color() {
    let invocation = parse(
        ["manyhands-cli.exe", "ticket", "list", "--bogus"]
            .map(OsString::from)
            .to_vec(),
    );
    let Action::Usage { detail, .. } = invocation.action else {
        panic!("expected Usage")
    };
    assert!(detail.contains("manyhands-cli ticket list"), "{detail}");
    assert!(!detail.contains("manyhands-cli.exe"));
    assert!(!detail.contains('\u{1b}'));
}

#[cfg(unix)]
#[test]
fn non_utf8_values_are_usage_errors_including_planned_arguments() {
    use std::os::unix::ffi::OsStringExt;
    for args in [
        vec!["ticket", "list", "--repo"],
        vec!["ticket", "close", "--id"],
    ] {
        let mut argv: Vec<_> = std::iter::once("manyhands-cli")
            .chain(args)
            .map(OsString::from)
            .collect();
        argv.push(OsString::from_vec(vec![0xff]));
        argv.push(OsString::from("--json"));
        let invocation = parse(argv);
        assert!(invocation.json);
        assert!(matches!(
            invocation.action,
            Action::Usage {
                command: Some(_),
                ..
            }
        ));
    }
}
