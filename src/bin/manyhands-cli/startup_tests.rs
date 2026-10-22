use super::*;
use crate::{
    catalog::{self, Availability, RepoRule},
    parse::{self, Action},
    render,
};
use manyhands::results::{Effects, ResultCode, Scope};
use serde_json::{Value, json};

#[derive(Default)]
struct FakeStartup {
    calls: Vec<&'static str>,
    terminal_present: bool,
    transport_fails: bool,
    service: Option<RepositoryService>,
}

impl Startup for FakeStartup {
    fn open_terminal(&mut self) -> Option<Box<dyn Terminal>> {
        self.calls.push("terminal");
        self.terminal_present
            .then(|| Box::new(crate::terminal::FakeTerminal::default()) as Box<dyn Terminal>)
    }

    fn start_transport(&mut self) -> Result<(), ()> {
        self.calls.push("transport");
        if self.transport_fails {
            Err(())
        } else {
            Ok(())
        }
    }

    fn open_service(&mut self) -> Result<RepositoryService, ReadError> {
        self.calls.push("service");
        self.service
            .take()
            .ok_or_else(|| ReadError::new(ResultCode::Busy))
    }
}

fn invocation(args: &[&str], json: bool) -> Invocation {
    let mut argv = vec![OsString::from("manyhands-cli")];
    argv.extend(args.iter().map(OsString::from));
    if json {
        argv.push("--json".into());
    }
    parse::parse(argv)
}

fn output_json(output: &Output) -> Value {
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout.last(), Some(&b'\n'));
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn early_mutation_refusals_keep_ids_null_and_preserve_startup_order() {
    for request_id in ["01M4CC0VMTTNKVYS6XWQG4CK1N", "invalid-id-sentinel"] {
        for (extra, transport_fails, code, status, stages) in [
            (vec!["--unknown"], false, ResultCode::UsageError, 2, vec![]),
            (vec!["--prepare"], false, ResultCode::UsageError, 2, vec![]),
            (
                vec!["--prepare", "--confirm", request_id],
                false,
                ResultCode::UsageError,
                2,
                vec![],
            ),
            (
                vec![],
                true,
                ResultCode::InternalError,
                1,
                vec!["transport"],
            ),
            (
                vec![],
                false,
                ResultCode::Busy,
                5,
                vec!["transport", "service"],
            ),
        ] {
            for json in [false, true] {
                let mut args = vec![
                    "repo",
                    "identity-set",
                    "--repo",
                    "missing-root",
                    "--input",
                    "missing-input",
                    "--request-id",
                    request_id,
                ];
                args.extend_from_slice(&extra);
                let mut startup = FakeStartup {
                    transport_fails,
                    ..Default::default()
                };
                let output = run(invocation(&args, json), &mut startup);
                let mut expected = stages.clone();
                if !json && extra.first() != Some(&"--unknown") {
                    expected.insert(0, "terminal");
                }
                assert_eq!(startup.calls, expected);
                assert_eq!(output.status, status);
                if json {
                    let value = output_json(&output);
                    assert_eq!(value["code"], code.as_str());
                    assert!(value["request_id"].is_null());
                    assert!(value["operation_id"].is_null() && value["data"].is_null());
                    assert_eq!(
                        value["scope"],
                        serde_json::to_value(Scope::default()).unwrap()
                    );
                } else {
                    assert!(output.stdout.is_empty());
                    let stderr = String::from_utf8(output.stderr).unwrap();
                    assert!(stderr.contains(&format!("code: {}\n", code.as_str())));
                    assert!(!stderr.contains(request_id));
                    assert!(!stderr.contains("request: "));
                }
            }
        }
    }
}

#[test]
fn directory_unset_empty_absolute_and_relative() {
    let cwd = std::env::current_dir().unwrap();
    assert_eq!(data_directory(None, &cwd).unwrap(), None);
    assert_eq!(
        data_directory(Some(OsString::new()), &cwd)
            .unwrap_err()
            .code(),
        ResultCode::InvalidPath
    );
    let absolute = cwd.join("data/../unchanged");
    assert_eq!(
        data_directory(Some(absolute.clone().into_os_string()), &cwd).unwrap(),
        Some(absolute)
    );
    assert_eq!(
        data_directory(Some("relative data".into()), &cwd).unwrap(),
        Some(cwd.join("relative data"))
    );
}

#[cfg(unix)]
#[test]
fn directory_preserves_non_utf8_os_paths() {
    use std::os::unix::ffi::OsStringExt;
    let path = OsString::from_vec(vec![b'd', 0xff]);
    assert_eq!(
        data_directory(Some(path.clone()), Path::new("/cwd")).unwrap(),
        Some(Path::new("/cwd").join(path))
    );
}

#[test]
fn service_free_actions_never_call_startup_in_either_mode() {
    for args in [
        vec![],
        vec!["--help"],
        vec!["ticket"],
        vec!["ticket", "deps", "--help"],
        vec!["--version"],
        vec!["frobnicate"],
        vec!["ticket", "list", "--bogus"],
        vec!["ticket", "close", "--id", "X", "--prepare"],
    ] {
        for json in [false, true] {
            let mut startup = FakeStartup::default();
            run(invocation(&args, json), &mut startup);
            assert!(startup.calls.is_empty(), "{args:?}");
        }
    }
}

#[test]
fn every_available_run_starts_transport_before_service_and_maps_open_failure() {
    for command in catalog::COMMANDS.iter().filter(|spec| {
        matches!(spec.availability, Availability::Available(_)) && spec.kind == catalog::Kind::Read
    }) {
        for json in [false, true] {
            let mut startup = FakeStartup::default();
            let output = run(
                Invocation {
                    json,
                    action: Action::Run {
                        command,
                        args: clap::ArgMatches::default(),
                    },
                },
                &mut startup,
            );
            assert_eq!(startup.calls, ["transport", "service"]);
            assert_eq!(output.status, 5);
            if json {
                let value = output_json(&output);
                assert_eq!(value["code"], "busy");
                assert_eq!(
                    value["command"],
                    format!("{} {}", command.resource, command.verb)
                );
            } else {
                assert!(output.stdout.is_empty());
                assert!(
                    String::from_utf8(output.stderr)
                        .unwrap()
                        .contains("code: busy\n")
                );
            }
        }
    }
}

#[test]
fn transport_failure_stops_before_service_and_has_only_fixed_diagnostic() {
    for json in [false, true] {
        let mut startup = FakeStartup {
            transport_fails: true,
            ..FakeStartup::default()
        };
        let output = run(invocation(&["id", "new"], json), &mut startup);
        assert_eq!(startup.calls, ["transport"]);
        assert_eq!(output.status, 1);
        if json {
            let value = output_json(&output);
            assert_eq!(value["code"], "internal_error");
            assert_eq!(value["message"], ResultCode::InternalError.message());
            assert_eq!(value["command"], "id new");
        } else {
            assert!(output.stdout.is_empty());
            assert_eq!(
                output.stderr,
                format!(
                    "error: {}\ncode: internal_error\nGit transport initialization failed\n",
                    ResultCode::InternalError.message()
                )
                .as_bytes()
            );
        }
    }
}

#[test]
fn successful_startup_dispatches_id_new() {
    let directory = tempfile::tempdir().unwrap();
    let mut startup = FakeStartup {
        service: Some(RepositoryService::open_at(directory.path()).unwrap()),
        ..FakeStartup::default()
    };
    let output = run(invocation(&["id", "new"], true), &mut startup);
    assert_eq!(startup.calls, ["transport", "service"]);
    assert_eq!(output.status, 0);
    let value = output_json(&output);
    assert_eq!(value["command"], "id new");
    assert!(
        value["data"]["id"]
            .as_str()
            .unwrap()
            .parse::<manyhands::canonical::ItemId>()
            .is_ok()
    );
}

fn expected_command(spec: &catalog::CommandSpec) -> Value {
    json!({
        "command": format!("{} {}", spec.resource, spec.verb),
        "available": matches!(spec.availability, Availability::Available(_)),
        "delivered_by": spec.delivered_by(),
        "summary": spec.summary,
        "repository": match spec.repo { Some(RepoRule::Required) => Some("required"), Some(RepoRule::Refused) => Some("refused"), None => None },
        "options": spec.options.iter().map(|option| json!({"name": option.name, "value": option.value, "required": option.required, "summary": option.summary})).collect::<Vec<_>>()
    })
}

#[test]
fn json_help_has_exact_dto_shape_scope_effects_and_target_filter() {
    for (args, specs) in [
        (vec!["--help"], catalog::COMMANDS.iter().collect::<Vec<_>>()),
        (
            vec!["ticket", "--help"],
            catalog::resource_commands("ticket"),
        ),
        (
            vec!["ticket", "list", "--help"],
            vec![catalog::find("ticket", "list").unwrap()],
        ),
        (
            vec!["ticket", "close", "--help"],
            vec![catalog::find("ticket", "close").unwrap()],
        ),
    ] {
        let output = run(invocation(&args, true), &mut FakeStartup::default());
        assert_eq!(output.status, 0);
        let value = output_json(&output);
        assert_eq!(value["command"], "cli help");
        assert_eq!(value["outcome"], "success");
        assert_eq!(
            value["scope"],
            serde_json::to_value(Scope::default()).unwrap()
        );
        assert_eq!(
            value["effects"],
            serde_json::to_value(Effects::not_requested()).unwrap()
        );
        assert_eq!(
            value["data"],
            json!({"commands": specs.into_iter().map(expected_command).collect::<Vec<_>>()})
        );
    }
}

#[test]
fn root_text_lists_every_command_and_planned_cycle_and_data_override() {
    let output = run(invocation(&[], false), &mut FakeStartup::default());
    assert_eq!(output.status, 0);
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Usage: manyhands-cli"));
    assert!(text.contains("MANYHANDS_DATA_DIR"));
    for spec in catalog::COMMANDS {
        let name = format!("{} {}", spec.resource, spec.verb);
        let line = text
            .lines()
            .find(|line| line.trim_start().starts_with(&format!("{name} ")))
            .unwrap_or_else(|| panic!("missing {name}"));
        assert!(line.contains(spec.summary));
        if let Availability::Planned(cycle) = spec.availability {
            assert!(line.contains(&format!("Arrives with {cycle}.")));
        }
    }
    assert_eq!(
        text.as_bytes(),
        run(invocation(&["--help"], false), &mut FakeStartup::default()).stdout
    );
}

#[test]
fn resource_and_available_command_help_are_clap_rendered_with_options() {
    for (args, usage) in [
        (vec!["ticket", "--help"], "Usage: manyhands-cli ticket"),
        (
            vec!["ticket", "deps", "--help"],
            "Usage: manyhands-cli ticket deps",
        ),
    ] {
        let output = run(invocation(&args, false), &mut FakeStartup::default());
        assert_eq!(output.status, 0);
        assert!(output.stderr.is_empty());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains(usage), "{text}");
        assert!(text.contains("--json"));
        if args.len() == 3 {
            for option in catalog::find("ticket", "deps").unwrap().options {
                assert!(text.contains(option.name));
            }
        } else {
            assert!(text.contains("Arrives with C5."));
            assert!(!text.contains("repo create"));
        }
    }
}

#[test]
fn planned_help_has_only_name_summary_and_delivery_cycle() {
    for spec in catalog::COMMANDS {
        if let Availability::Planned(cycle) = spec.availability {
            let output = run(
                invocation(&[spec.resource, spec.verb, "--help"], false),
                &mut FakeStartup::default(),
            );
            assert_eq!(output.status, 0);
            assert!(output.stderr.is_empty());
            assert_eq!(
                output.stdout,
                format!(
                    "{} {}\n{}\nArrives with {cycle}.\n",
                    spec.resource, spec.verb, spec.summary
                )
                .as_bytes()
            );
        }
    }
}

#[test]
fn version_has_exact_json_data_and_human_text() {
    for json in [false, true] {
        let output = run(
            invocation(&["--version"], json),
            &mut FakeStartup::default(),
        );
        assert_eq!(output.status, 0);
        assert!(output.stderr.is_empty());
        if json {
            let value = output_json(&output);
            assert_eq!(value["command"], "cli version");
            assert_eq!(
                value["scope"],
                serde_json::to_value(Scope::default()).unwrap()
            );
            assert_eq!(
                value["effects"],
                serde_json::to_value(Effects::not_requested()).unwrap()
            );
            assert_eq!(
                value["data"],
                json!({"name": "manyhands-cli", "version": env!("CARGO_PKG_VERSION")})
            );
        } else {
            assert_eq!(
                output.stdout,
                format!("manyhands-cli {}\n", env!("CARGO_PKG_VERSION")).as_bytes()
            );
        }
    }
}

#[test]
fn usage_and_unavailable_have_canonical_names_empty_recovery_and_human_only_extra() {
    for (args, command, code, extra) in [
        (
            vec!["frobnicate"],
            "cli usage",
            render::USAGE_CODE,
            "frobnicate",
        ),
        (
            vec!["ticket", "list", "--bogus"],
            "ticket list",
            render::USAGE_CODE,
            "--bogus",
        ),
        (
            vec!["ticket", "close"],
            "ticket close",
            render::UNAVAILABLE_CODE,
            "ticket close arrives with C5.",
        ),
    ] {
        for json in [false, true] {
            let output = run(invocation(&args, json), &mut FakeStartup::default());
            assert_eq!(output.status, 2);
            if json {
                let value = output_json(&output);
                assert_eq!(value["command"], command);
                assert_eq!(value["code"], code.as_str());
                assert_eq!(value["message"], code.message());
                assert_eq!(
                    value["scope"],
                    serde_json::to_value(Scope::default()).unwrap()
                );
                assert_eq!(value["recovery"], json!([]));
                assert_eq!(value["data"], Value::Null);
            } else {
                assert!(output.stdout.is_empty());
                let text = String::from_utf8(output.stderr).unwrap();
                assert!(text.starts_with(&format!(
                    "error: {}\ncode: {}\n",
                    code.message(),
                    code.as_str()
                )));
                assert!(text.contains(extra));
            }
        }
    }
}

static LOCAL_MUTATION: CommandSpec = CommandSpec {
    resource: "test",
    verb: "mutate",
    summary: "Test-local mutation.",
    availability: Availability::Available("C2"),
    repo: None,
    options: &[catalog::REQUEST_ID, catalog::PREPARE, catalog::CONFIRM],
    kind: catalog::Kind::Mutation {
        confirmed: true,
        input: catalog::InputRule::Refused,
    },
};

fn prompting_invocation(command: &'static CommandSpec, args: &[&str], json: bool) -> Invocation {
    use clap::{Arg, ArgAction, Command};
    let mut parser = Command::new(command.verb).arg(
        Arg::new("non-interactive")
            .long("non-interactive")
            .action(ArgAction::SetTrue),
    );
    for option in command.options {
        let name = option.name.strip_prefix("--").unwrap();
        parser = parser.arg(Arg::new(name).long(name).action(if option.value.is_some() {
            ArgAction::Set
        } else {
            ArgAction::SetTrue
        }));
    }
    Invocation {
        json,
        action: Action::Run {
            command,
            args: parser
                .try_get_matches_from(std::iter::once(command.verb).chain(args.iter().copied()))
                .unwrap(),
        },
    }
}

#[test]
fn a_prompting_command_opens_the_terminal_before_the_transport() {
    let mut startup = FakeStartup {
        terminal_present: true,
        ..FakeStartup::default()
    };
    let output = run(
        prompting_invocation(&LOCAL_MUTATION, &[], false),
        &mut startup,
    );
    assert_eq!(startup.calls, ["terminal", "transport", "service"]);
    assert_eq!(output.status, 5); // Fake service refusal: mode validation passed.
}

#[test]
fn no_read_opens_the_terminal() {
    for command in catalog::COMMANDS
        .iter()
        .filter(|spec| spec.kind == catalog::Kind::Read)
    {
        let mut startup = FakeStartup {
            terminal_present: true,
            ..FakeStartup::default()
        };
        let output = run(
            Invocation {
                json: false,
                action: Action::Run {
                    command,
                    args: clap::ArgMatches::default(),
                },
            },
            &mut startup,
        );
        assert_eq!(startup.calls, ["transport", "service"]);
        assert_eq!(output.status, 5);
    }
}

#[test]
fn json_and_non_interactive_never_open_the_terminal() {
    for (json, args) in [
        (true, vec!["--prepare"]),
        (false, vec!["--prepare", "--non-interactive"]),
    ] {
        let mut startup = FakeStartup {
            terminal_present: true,
            ..FakeStartup::default()
        };
        let output = run(
            prompting_invocation(&LOCAL_MUTATION, &args, json),
            &mut startup,
        );
        assert_eq!(startup.calls, ["transport", "service"]);
        assert_eq!(output.status, 5);
    }
}

#[test]
fn a_mode_rule_failure_is_usage_and_starts_nothing() {
    let mut startup = FakeStartup::default();
    let output = run(
        prompting_invocation(&LOCAL_MUTATION, &[], false),
        &mut startup,
    );
    assert_eq!(startup.calls, ["terminal"]);
    assert_eq!(output.status, 2);
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("code: usage_error\n")
    );
}

#[test]
fn confirm_without_request_id_requires_interactive_mode() {
    for terminal_present in [false, true] {
        for json in [false, true] {
            for non_interactive in [false, true] {
                let mut args = vec!["--confirm", "01M4CC0VMTTNKVYS6XWQG4CK1N"];
                if non_interactive {
                    args.push("--non-interactive");
                }
                let mut startup = FakeStartup {
                    terminal_present,
                    ..FakeStartup::default()
                };
                let output = run(
                    prompting_invocation(&LOCAL_MUTATION, &args, json),
                    &mut startup,
                );
                let tries_terminal = !json && !non_interactive;
                let interactive = tries_terminal && terminal_present;
                let mut calls = Vec::new();
                if tries_terminal {
                    calls.push("terminal");
                }
                if interactive {
                    calls.extend(["transport", "service"]);
                }
                assert_eq!(startup.calls, calls);
                assert_eq!(output.status, if interactive { 5 } else { 2 });
                if json {
                    let value = output_json(&output);
                    assert_eq!(value["code"], "usage_error");
                    assert_eq!(value["command"], "test mutate");
                    assert_eq!(
                        value["scope"],
                        serde_json::to_value(Scope::default()).unwrap()
                    );
                }
            }
        }
    }
}

#[test]
fn unlocking_reads_decide_mode_without_mutation_arguments() {
    static UNLOCKING: CommandSpec = CommandSpec {
        resource: "host",
        verb: "observe",
        summary: "Test unlocking read.",
        availability: Availability::Available("C2"),
        repo: None,
        options: &[],
        kind: catalog::Kind::UnlockingRead,
    };
    for terminal_present in [false, true] {
        let mut startup = FakeStartup {
            terminal_present,
            ..FakeStartup::default()
        };
        let output = run(prompting_invocation(&UNLOCKING, &[], false), &mut startup);
        assert_eq!(startup.calls, ["terminal", "transport", "service"]);
        assert_eq!(output.status, 5);
    }
}
