use std::{ffi::OsStr, fs, path::Path, process::Command};

use manyhands::results::Scope;
use serde_json::{Value, json};

use crate::support::{
    self,
    cli::{self, CliEnv, CliRun},
    items, schema,
};

fn environment(data: &Path) -> CliEnv {
    let env = CliEnv::new(data.to_owned());
    fs::write(
        env.home.path().join(".gitconfig"),
        "[core]\n\tautocrlf = false\n",
    )
    .unwrap();
    env
}

fn refs(env: &CliEnv, root: &Path) -> Vec<u8> {
    let mut command = Command::new("git");
    cli::cli_environment(&mut command, env);
    let output = command
        .arg("-C")
        .arg(root)
        .arg("for-each-ref")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    output.stdout
}

// Every invocation below, including failures and human output, must be read-only.
fn run(env: &CliEnv, root: &Path, args: &[&str], json: bool) -> CliRun {
    let before = support::repository_and_worktree_snapshot_at(root);
    let before_refs = refs(env, root);
    let mut args: Vec<_> = args.iter().map(OsStr::new).collect();
    args.extend([OsStr::new("--repo"), root.as_os_str()]);
    if json {
        args.push(OsStr::new("--json"));
    }
    let output = cli::run_cli(&args, env);
    assert_eq!(support::repository_and_worktree_snapshot_at(root), before);
    assert_eq!(refs(env, root), before_refs);
    output
}

fn checked(
    env: &CliEnv,
    root: &Path,
    args: &[&str],
    code: &str,
    status: i32,
    data_schema: Option<&str>,
    scope: Scope,
) -> Value {
    let output = run(env, root, args, true);
    let value = cli::json(&output);
    assert_eq!(value["code"], code, "{args:?}: {value}");
    assert_eq!(output.status, status);
    assert_eq!(value["command"], format!("{} {}", args[0], args[1]));
    assert_eq!(value["scope"], serde_json::to_value(scope).unwrap());
    schema::check_published("envelope.schema.json", &value).unwrap();
    if let Some(name) = data_schema {
        schema::check_published(name, &value["data"]).unwrap();
    } else {
        assert!(value["data"].is_null());
    }
    value
}

fn golden(name: &str, value: &Value, root: &Path) {
    fn volatile(value: &Value, pairs: &mut Vec<(String, &'static str)>) {
        match value {
            Value::Object(map) => {
                for (key, value) in map {
                    if let Some(text) = value.as_str() {
                        match key.as_str() {
                            "refreshed_at" => pairs.push((text.into(), "<refreshed-at>")),
                            "head_oid" => pairs.push((text.into(), "<head-oid>")),
                            _ => {}
                        }
                    }
                    volatile(value, pairs);
                }
            }
            Value::Array(values) => {
                for value in values {
                    volatile(value, pairs);
                }
            }
            _ => {}
        }
    }
    let mut pairs = vec![(
        root.canonicalize().unwrap().to_str().unwrap().to_owned(),
        "<repository>",
    )];
    volatile(value, &mut pairs);
    pairs.sort();
    pairs.dedup();
    cli::assert_cli_golden(
        name,
        value,
        &pairs
            .iter()
            .map(|(v, p)| (v.as_str(), *p))
            .collect::<Vec<_>>(),
    );
}

fn item_scope(mut scope: Scope, id: &str) -> Scope {
    scope.item_id = Some(id.to_owned());
    scope
}

#[test]
fn path_observe_gives_the_absent_token_for_an_empty_document_path() {
    let (fixture, enabled) = items::contract_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let env = environment(enabled.data_directory.path());
    let args = ["path", "observe", "--path", "docs/absent.md"];
    let value = checked(
        &env,
        &fixture.root,
        &args,
        "ok",
        0,
        Some("path_observation.schema.json"),
        repo.scope(),
    );
    assert_eq!(value["data"]["exists"], false);
    let expected = enabled
        .service
        .observe_path(&repo, None, Path::new("docs/absent.md"))
        .unwrap();
    assert_eq!(value["data"]["observation"], expected.observation);
    golden("path_observe_ok", &value, &fixture.root);
    let human = run(&env, &fixture.root, &args, false);
    assert_eq!(human.status, 0);
    assert!(human.stderr.is_empty());
    cli::assert_cli_text_golden(
        "path_observe_text",
        std::str::from_utf8(&human.stdout).unwrap(),
        &[(repo.root().to_str().unwrap(), "<repository>")],
    );
}

#[test]
fn path_observe_gives_the_same_token_as_document_show_for_a_file() {
    let (fixture, enabled) = items::contract_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let env = environment(enabled.data_directory.path());
    let observed = checked(
        &env,
        &fixture.root,
        &["path", "observe", "--path", "docs/guide.md"],
        "ok",
        0,
        Some("path_observation.schema.json"),
        repo.scope(),
    );
    let shown = checked(
        &env,
        &fixture.root,
        &["document", "show", "--path", "docs/guide.md"],
        "ok",
        0,
        Some("item.schema.json"),
        repo.scope(),
    );
    assert_eq!(observed["data"]["exists"], true);
    assert_eq!(
        observed["data"]["observation"],
        shown["data"]["observation"]
    );
}

#[test]
fn path_observe_reports_another_items_file_in_a_named_context() {
    let (fixture, enabled) = items::contract_repository();
    items::create_document_context(
        &enabled.service,
        &fixture.root,
        items::DOCUMENT_C,
        "docs/draft.md",
    );
    let context = items::context_worktree(&fixture.root, items::DOCUMENT_C);
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let env = environment(enabled.data_directory.path()).in_dir(fixture.root.clone());
    let relative = format!(".manyhands/worktrees/{}", items::DOCUMENT_C);
    let value = checked(
        &env,
        &fixture.root,
        &[
            "path",
            "observe",
            "--path",
            "docs/guide.md",
            "--context",
            &relative,
        ],
        "ok",
        0,
        Some("path_observation.schema.json"),
        repo.scope(),
    );
    assert_eq!(value["data"]["exists"], true);
    assert_eq!(
        value["data"]["context"]["worktree"],
        context.to_str().unwrap()
    );
}

#[test]
fn path_observe_refuses_an_inactive_context_and_escaping_paths() {
    let (fixture, enabled) = items::contract_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let env = environment(enabled.data_directory.path());
    let inactive = fixture.root.join("inactive-context");
    fs::create_dir(&inactive).unwrap();
    let absolute = fixture.root.join("x.md");
    for args in [
        vec![
            "path",
            "observe",
            "--path",
            "docs/x.md",
            "--context",
            inactive.to_str().unwrap(),
        ],
        vec!["path", "observe", "--path", "../x.md"],
        vec!["path", "observe", "--path", absolute.to_str().unwrap()],
        vec!["path", "observe", "--path", "docs/../../x.md"],
    ] {
        let value = checked(
            &env,
            &fixture.root,
            &args,
            "invalid_path",
            2,
            None,
            repo.scope(),
        );
        golden("path_observe_invalid_path", &value, &fixture.root);
        let human = run(&env, &fixture.root, &args, false);
        assert_eq!(human.status, 2);
        assert!(human.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&human.stderr).contains(args.last().unwrap()));
    }
}

#[test]
fn path_observe_escapes_a_path_holding_control_characters() {
    let (fixture, enabled) = items::contract_repository();
    let env = environment(enabled.data_directory.path());
    let human = run(
        &env,
        &fixture.root,
        &["path", "observe", "--path", "docs/a\u{1b}[2J\nb.md"],
        false,
    );
    assert_eq!(human.status, 0);
    assert!(human.stderr.is_empty());
    let text = std::str::from_utf8(&human.stdout).unwrap();
    assert!(!text.contains('\u{1b}'));
    assert!(!text.contains("\nb.md"));
    assert_eq!(text.lines().count(), 4);
    assert!(text.contains("docs/a\\u{1b}[2J\\nb.md"));
}

#[test]
fn ticket_check_reports_resolved_unresolved_and_a_dependency_cycle() {
    let (fixture, enabled) = items::relationship_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let env = environment(enabled.data_directory.path());
    let deps = format!("{},{}", items::RELATED_D, items::TICKET_ABSENT);
    let args = ["ticket", "check", "--id", items::RELATED_C, "--deps", &deps];
    let value = checked(
        &env,
        &fixture.root,
        &args,
        "ok",
        0,
        Some("relationship_check.schema.json"),
        item_scope(repo.scope(), items::RELATED_C),
    );
    assert_eq!(
        value["data"]["deps"],
        json!([items::TICKET_ABSENT, items::RELATED_D])
    );
    assert_eq!(value["data"]["unresolved"], json!([items::TICKET_ABSENT]));
    assert_eq!(value["data"]["rejection"]["code"], "relationship_cycle");
    assert_eq!(
        value["data"]["rejection"]["ids"],
        json!([items::RELATED_C, items::RELATED_D])
    );
    golden("ticket_check_ok", &value, &fixture.root);
    let human = run(&env, &fixture.root, &args, false);
    assert_eq!(human.status, 0);
    assert!(human.stderr.is_empty());
    cli::assert_cli_text_golden(
        "ticket_check_text",
        std::str::from_utf8(&human.stdout).unwrap(),
        &[],
    );
}

#[test]
fn ticket_check_reports_a_parent_cycle() {
    let (fixture, enabled) = items::relationship_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let env = environment(enabled.data_directory.path());
    let value = checked(
        &env,
        &fixture.root,
        &[
            "ticket",
            "check",
            "--id",
            items::RELATED_B,
            "--parent",
            items::RELATED_C,
        ],
        "ok",
        0,
        Some("relationship_check.schema.json"),
        item_scope(repo.scope(), items::RELATED_B),
    );
    assert_eq!(value["data"]["deps"], json!([]));
    assert_eq!(value["data"]["rejection"]["code"], "relationship_cycle");
    assert_eq!(
        value["data"]["rejection"]["ids"],
        json!([items::RELATED_B, items::RELATED_C])
    );
}

#[test]
fn ticket_check_proposes_exactly_what_it_is_given() {
    let (fixture, enabled) = items::relationship_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let env = environment(enabled.data_directory.path());
    for (flag, target, rejected) in [
        ("--parent", items::RELATED_B, false),
        ("--deps", items::RELATED_F, true),
    ] {
        let value = checked(
            &env,
            &fixture.root,
            &["ticket", "check", "--id", items::RELATED_E, flag, target],
            "ok",
            0,
            Some("relationship_check.schema.json"),
            item_scope(repo.scope(), items::RELATED_E),
        );
        if rejected {
            assert_eq!(value["data"]["rejection"]["code"], "relationship_cycle");
            assert_eq!(
                value["data"]["rejection"]["ids"],
                json!([items::RELATED_E, items::RELATED_F])
            );
            assert!(value["data"]["parent"].is_null());
        } else {
            assert!(value["data"]["rejection"].is_null());
            assert_eq!(value["data"]["deps"], json!([]));
            assert_eq!(value["data"]["parent"], target);
        }
    }
}

#[test]
fn ticket_check_refuses_malformed_or_empty_elements() {
    let (fixture, enabled) = items::relationship_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let env = environment(enabled.data_directory.path());
    let empty_element = format!("{},,{}", items::RELATED_A, items::RELATED_B);
    for (flag, value) in [
        ("--deps", empty_element.as_str()),
        ("--deps", "not-a-ulid"),
        ("--deps", ""),
        ("--parent", "not-a-ulid"),
    ] {
        let args = ["ticket", "check", "--id", items::RELATED_C, flag, value];
        let result = checked(
            &env,
            &fixture.root,
            &args,
            "invalid_id",
            2,
            None,
            repo.scope(),
        );
        golden("ticket_check_invalid_id", &result, &fixture.root);
        let human = run(&env, &fixture.root, &args, false);
        assert_eq!(human.status, 2);
        assert!(human.stdout.is_empty());
        if !value.is_empty() {
            assert!(!String::from_utf8_lossy(&human.stderr).contains(value));
        }
    }
    checked(
        &env,
        &fixture.root,
        &["ticket", "check", "--id", items::RELATED_C],
        "usage_error",
        2,
        None,
        Scope::default(),
    );
}

#[test]
fn both_reads_write_nothing() {
    // `run` enforces byte-for-byte worktree and for-each-ref equality for all cases.
    let (fixture, enabled) = items::relationship_repository();
    let env = environment(enabled.data_directory.path());
    for args in [
        vec!["path", "observe", "--path", "docs/new.md"],
        vec![
            "ticket",
            "check",
            "--id",
            items::RELATED_C,
            "--parent",
            items::TICKET_ABSENT,
        ],
    ] {
        let output = run(&env, &fixture.root, &args, true);
        assert_eq!(output.status, 0);
        assert_eq!(
            cli::json(&output)["effects"],
            serde_json::to_value(manyhands::results::Effects::not_requested()).unwrap()
        );
    }
}
