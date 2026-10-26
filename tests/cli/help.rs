use std::{collections::BTreeSet, ffi::OsStr, fs};

use manyhands::results::{Effects, ResultCode, Scope};
use serde_json::Value;

use crate::support::{
    cli::{
        CLI_REPOSITORY_VALUES, CliEnv, CliRun, assert_cli_golden, assert_cli_text_golden,
        cli_command, json, run_cli,
    },
    schema,
};

fn run(args: &[&str], env: &CliEnv) -> CliRun {
    run_cli(&args.iter().map(OsStr::new).collect::<Vec<_>>(), env)
}

fn checked(run: &CliRun, command: &str, code: ResultCode, data_schema: Option<&str>) -> Value {
    let value = json(run);
    schema::check_published("envelope.schema.json", &value).unwrap();
    assert_eq!(value["command"], command);
    assert_eq!(value["code"], code.as_str());
    assert_eq!(value["message"], code.message());
    assert_eq!(
        value["scope"],
        serde_json::to_value(Scope::default()).unwrap()
    );
    assert_eq!(
        value["effects"],
        serde_json::to_value(Effects::not_requested()).unwrap()
    );
    assert_eq!(value["recovery"], serde_json::json!([]));
    if let Some(schema_name) = data_schema {
        schema::check_published(schema_name, &value["data"]).unwrap();
    } else {
        assert!(value["data"].is_null());
    }
    value
}

#[test]
fn root_help_and_bare_call_match_in_both_modes_without_data() {
    let temp = tempfile::tempdir().unwrap();
    let env = CliEnv::new(temp.path().join("absent-data"));
    let output = run(&["--help", "--json"], &env);
    assert_eq!(output.status, 0);
    let value = checked(
        &output,
        "cli help",
        ResultCode::Ok,
        Some("cli_help.schema.json"),
    );
    assert_cli_golden("help_root", &value, &[]);
    let repositories: BTreeSet<_> = value["data"]["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|command| command["repository"].as_str())
        .collect();
    let expected: BTreeSet<_> = CLI_REPOSITORY_VALUES
        .iter()
        .copied()
        .map(Some)
        .chain([None])
        .collect();
    assert_eq!(repositories, expected);
    let bare_json = run(&["--json"], &env);
    assert_eq!(bare_json.status, 0);
    assert_eq!(
        checked(
            &bare_json,
            "cli help",
            ResultCode::Ok,
            Some("cli_help.schema.json")
        ),
        value
    );
    let text = run(&["--help"], &env);
    assert_eq!(text.status, 0);
    assert!(text.stderr.is_empty());
    assert_cli_text_golden(
        "help_root_text",
        std::str::from_utf8(&text.stdout).unwrap(),
        &[],
    );
    let bare = run(&[], &env);
    assert_eq!(bare.status, 0);
    assert!(bare.stderr.is_empty());
    assert_eq!(bare.stdout, text.stdout);
    assert!(!env.data.exists());
}

#[test]
fn human_help_is_identical_at_small_and_large_columns_and_matches_root_golden() {
    let temp = tempfile::tempdir().unwrap();
    let environments = [
        (
            20,
            CliEnv::new(temp.path().join("small-data")).with_columns(20),
        ),
        (
            200,
            CliEnv::new(temp.path().join("large-data")).with_columns(200),
        ),
    ];
    for args in [
        vec!["--help"],
        vec!["ticket", "--help"],
        vec!["ticket", "deps", "--help"],
        vec!["ticket", "close", "--help"],
    ] {
        let mut outputs = Vec::new();
        for (columns, env) in &environments {
            let argv: Vec<_> = args.iter().map(OsStr::new).collect();
            let command = cli_command(&argv, env);
            let width = columns.to_string();
            // Verify distinct widths really reach the child command, so an
            // unwired builder cannot turn this into two identical default runs.
            assert_eq!(
                command.get_envs().find(|(name, _)| *name == "COLUMNS"),
                Some((OsStr::new("COLUMNS"), Some(OsStr::new(&width))))
            );
            let output = run_cli(&argv, env);
            assert_eq!(output.status, 0);
            assert!(output.stderr.is_empty());
            assert!(!output.stdout.is_empty());
            if args == ["--help"] {
                assert_cli_text_golden(
                    "help_root_text",
                    std::str::from_utf8(&output.stdout).unwrap(),
                    &[],
                );
            }
            assert!(!env.data.exists());
            outputs.push(output.stdout);
        }
        assert_eq!(outputs[0], outputs[1], "{args:?}");
    }
}

#[test]
fn resource_and_command_help_have_scoped_goldens_without_data() {
    let temp = tempfile::tempdir().unwrap();
    let env = CliEnv::new(temp.path().join("absent-data"));
    for (args, name, only) in [
        (
            vec!["ticket", "--help", "--json"],
            "help_resource_ticket",
            None,
        ),
        (
            vec!["ticket", "list", "--help", "--json"],
            "help_ticket_list",
            Some("ticket list"),
        ),
        (
            vec!["ticket", "close", "--help", "--json"],
            "help_ticket_close",
            Some("ticket close"),
        ),
    ] {
        let output = run(&args, &env);
        assert_eq!(output.status, 0);
        let value = checked(
            &output,
            "cli help",
            ResultCode::Ok,
            Some("cli_help.schema.json"),
        );
        let commands = value["data"]["commands"].as_array().unwrap();
        assert!(!commands.is_empty());
        assert!(
            commands
                .iter()
                .all(|command| command["command"].as_str().unwrap().starts_with("ticket "))
        );
        if let Some(only) = only {
            assert_eq!(commands.len(), 1);
            assert_eq!(commands[0]["command"], only);
        }
        assert_cli_golden(name, &value, &[]);
        assert!(!env.data.exists());
    }
}

#[test]
fn version_in_both_modes_has_no_data_directory() {
    let temp = tempfile::tempdir().unwrap();
    let env = CliEnv::new(temp.path().join("absent-data"));
    let output = run(&["--version", "--json"], &env);
    assert_eq!(output.status, 0);
    let value = checked(
        &output,
        "cli version",
        ResultCode::Ok,
        Some("cli_version.schema.json"),
    );
    assert_eq!(value["data"]["name"], "manyhands-cli");
    assert_eq!(value["data"]["version"], env!("CARGO_PKG_VERSION"));
    assert_cli_golden("version", &value, &[]);
    let text = run(&["--version"], &env);
    assert_eq!(text.status, 0);
    assert!(text.stderr.is_empty());
    assert_eq!(
        text.stdout,
        format!("manyhands-cli {}\n", env!("CARGO_PKG_VERSION")).as_bytes()
    );
    assert_cli_text_golden(
        "version_text",
        std::str::from_utf8(&text.stdout).unwrap(),
        &[],
    );
    assert!(!env.data.exists());
}

#[test]
fn usage_errors_in_both_modes_do_not_open_data() {
    let temp = tempfile::tempdir().unwrap();
    let env = CliEnv::new(temp.path().join("absent-data"));
    for (args, command, name) in [
        (vec!["frobnicate"], "cli usage", "usage_unknown_command"),
        (
            vec!["ticket", "list", "--bogus"],
            "ticket list",
            "usage_unknown_option",
        ),
    ] {
        let human = run(&args, &env);
        assert_eq!(human.status, 2);
        assert!(human.stdout.is_empty());
        let stderr = std::str::from_utf8(&human.stderr).unwrap();
        assert!(stderr.contains(ResultCode::UsageError.message()));
        assert!(stderr.contains("code: usage_error\n"));
        assert!(stderr.contains(args.last().unwrap()));
        let mut args = args;
        args.push("--json");
        let output = run(&args, &env);
        assert_eq!(output.status, 2);
        let value = checked(&output, command, ResultCode::UsageError, None);
        assert_cli_golden(name, &value, &[]);
        assert!(!env.data.exists());
    }
}

#[test]
fn mutation_only_options_on_reads_are_usage_errors_in_both_modes() {
    let temp = tempfile::tempdir().unwrap();
    let env = CliEnv::new(temp.path().join("absent-data"));
    for extra in [
        vec!["--input", "{}"],
        vec!["--request-id", "X"],
        vec!["--prepare"],
        vec!["--confirm", "X"],
    ] {
        for is_json in [false, true] {
            let mut args = vec!["ticket", "list", "--repo", "unused"];
            args.extend(extra.iter().copied());
            if is_json {
                args.push("--json");
            }
            let output = run(&args, &env);
            assert_eq!(output.status, 2);
            if is_json {
                checked(&output, "ticket list", ResultCode::UsageError, None);
            } else {
                assert!(output.stdout.is_empty());
                let stderr = std::str::from_utf8(&output.stderr).unwrap();
                assert!(stderr.contains("code: usage_error\n"));
                assert!(stderr.contains(extra[0]));
            }
            assert!(!env.data.exists());
        }
    }
}

#[test]
fn planned_command_recognizes_trailing_json_and_names_its_cycle_in_human_mode() {
    let temp = tempfile::tempdir().unwrap();
    let env = CliEnv::new(temp.path().join("absent-data"));
    let args = [
        "ticket",
        "close",
        "--id",
        "01K7F6H9J2N4Q6S8V0X2Z4B6DJ",
        "--prepare",
    ];
    let human = run(&args, &env);
    assert_eq!(human.status, 2);
    assert!(human.stdout.is_empty());
    let stderr = std::str::from_utf8(&human.stderr).unwrap();
    assert!(stderr.contains(ResultCode::CommandUnavailable.message()));
    assert!(stderr.contains("code: command_unavailable\n"));
    assert!(stderr.contains("ticket close arrives with C5."));
    let mut args = args.to_vec();
    args.push("--json");
    let output = run(&args, &env);
    assert_eq!(output.status, 2);
    let value = checked(
        &output,
        "ticket close",
        ResultCode::CommandUnavailable,
        None,
    );
    assert_cli_golden("unavailable_ticket_close", &value, &[]);
    assert!(!env.data.exists());
}

#[test]
fn repeated_json_produces_one_id_envelope() {
    let temp = tempfile::tempdir().unwrap();
    let env = CliEnv::new(temp.path().join("data"));
    let output = run(&["--json", "id", "new", "--json"], &env);
    assert_eq!(output.status, 0);
    let value = checked(
        &output,
        "id new",
        ResultCode::Ok,
        Some("new_id.schema.json"),
    );
    let id = value["data"]["id"].as_str().unwrap();
    assert!(id.parse::<manyhands::canonical::ItemId>().is_ok());
}

#[test]
fn data_directory_that_is_a_file_is_one_internal_error_without_panic() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("regular-file");
    fs::write(&data, b"leave this file alone").unwrap();
    let env = CliEnv::new(data);
    let output = run(&["repo", "list", "--json"], &env);
    assert_eq!(output.status, 1);
    checked(&output, "repo list", ResultCode::InternalError, None);
    assert!(!String::from_utf8_lossy(&output.stderr).contains("panicked"));
    assert_eq!(fs::read(&env.data).unwrap(), b"leave this file alone");
}
