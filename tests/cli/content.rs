use std::{
    ffi::OsStr,
    fs,
    path::Path,
    time::{Duration, SystemTime},
};

use manyhands::{
    repository::{ClosureFilter, ReadinessFilter, TicketFilter},
    results::Scope,
};
use serde_json::Value;

use crate::support::{
    self,
    cli::{self, CliEnv, CliRun},
    items, schema,
};

fn run(env: &CliEnv, root: &Path, args: &[&str], json: bool) -> CliRun {
    let mut args: Vec<_> = args.iter().map(OsStr::new).collect();
    args.extend([OsStr::new("--repo"), root.as_os_str()]);
    if json {
        args.push(OsStr::new("--json"));
    }
    cli::run_cli(&args, env)
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
    assert_eq!(output.status, status, "{args:?}: {value}");
    assert_eq!(value["command"], format!("{} {}", args[0], args[1]));
    assert_eq!(value["scope"], serde_json::to_value(scope).unwrap());
    schema::check_published("envelope.schema.json", &value).unwrap();
    if let Some(data_schema) = data_schema {
        schema::check_published(data_schema, &value["data"]).unwrap();
    } else {
        assert!(value["data"].is_null());
    }
    value
}

// Match F1's substitutions: only volatile roots, refresh times and head OIDs.
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
            .map(|(value, label)| (value.as_str(), *label))
            .collect::<Vec<_>>(),
    );
}

fn item_scope(mut scope: Scope, id: &str) -> Scope {
    scope.item_id = Some(id.to_owned());
    scope
}

#[test]
fn document_and_ticket_lists_match_f1_contracts_and_human_output() {
    let (fixture, enabled) = items::contract_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    for resource in ["document", "ticket"] {
        let args = [resource, "list"];
        let value = checked(
            &env,
            &fixture.root,
            &args,
            "ok",
            0,
            Some("item_list.schema.json"),
            repo.scope(),
        );
        assert_eq!(value["data"]["items"].as_array().unwrap().len(), 2);
        golden(&format!("cli_{resource}_list"), &value, &fixture.root);
        let human = run(&env, &fixture.root, &args, false);
        assert_eq!(human.status, 0);
        assert!(human.stderr.is_empty());
        let text = std::str::from_utf8(&human.stdout).unwrap();
        assert_eq!(text.lines().count(), 2);
        if resource == "ticket" {
            cli::assert_cli_text_golden("ticket_list_text", text, &[]);
        } else {
            assert!(text.contains(items::DOCUMENT_A));
            assert!(text.contains("Guide"));
        }
    }
}

#[test]
fn show_by_id_and_both_paths_with_optional_context_preserves_f1_scope_and_body() {
    let (fixture, enabled) = items::contract_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    let ticket_path = items::ticket_path(items::TICKET_B);
    for (resource, id, path, body) in [
        ("document", items::DOCUMENT_A, "docs/guide.md", "# Guide\n"),
        (
            "ticket",
            items::TICKET_B,
            ticket_path.as_str(),
            "Body of Closed ticket.\n",
        ),
    ] {
        let value = checked(
            &env,
            &fixture.root,
            &[resource, "show", "--id", id],
            "ok",
            0,
            Some("item.schema.json"),
            item_scope(repo.scope(), id),
        );
        golden(&format!("cli_{resource}_show"), &value, &fixture.root);
        for context in [false, true] {
            let mut args = vec![resource, "show", "--path", path];
            if context {
                args.extend(["--context", repo.root().to_str().unwrap()]);
            }
            let by_path = checked(
                &env,
                &fixture.root,
                &args,
                "ok",
                0,
                Some("item.schema.json"),
                repo.scope(),
            );
            assert_eq!(by_path["data"], value["data"]);
            let human = run(&env, &fixture.root, &args, false);
            assert_eq!(human.status, 0);
            assert!(human.stderr.is_empty());
            let text = std::str::from_utf8(&human.stdout).unwrap();
            assert!(text.contains(id));
            assert_eq!(text.split_once("\n\n").unwrap().1, body);
        }
    }
}

#[test]
fn explicit_linked_context_selects_that_copy() {
    let (fixture, enabled) = items::contract_repository();
    items::create_document_context(
        &enabled.service,
        &fixture.root,
        items::DOCUMENT_C,
        "docs/draft.md",
    );
    let context = items::context_worktree(&fixture.root, items::DOCUMENT_C);
    let ticket_path = items::ticket_path(items::TICKET_A);
    items::write(
        &context,
        &ticket_path,
        &items::ticket_source(items::TICKET_A, "Context ticket", ""),
    );
    items::refresh_completely(&enabled.service, &fixture.root);
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    for (resource, path, title) in [
        ("document", "docs/draft.md", "Draft of docs/draft.md"),
        ("ticket", ticket_path.as_str(), "Context ticket"),
    ] {
        let value = checked(
            &env,
            &fixture.root,
            &[
                resource,
                "show",
                "--path",
                path,
                "--context",
                context.to_str().unwrap(),
            ],
            "ok",
            0,
            Some("item.schema.json"),
            repo.scope(),
        );
        assert_eq!(value["data"]["title"], title);
        assert_eq!(
            value["data"]["context"]["worktree"],
            context.to_str().unwrap()
        );
    }
}

#[test]
fn nonconforming_document_show_and_list_expose_path_and_fixed_problem_code() {
    let (fixture, enabled) = items::contract_repository();
    items::write(&fixture.root, "docs/broken.md", "no front matter\n");
    items::refresh_completely(&enabled.service, &fixture.root);
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    let args = ["document", "show", "--path", "docs/broken.md"];
    let value = checked(
        &env,
        &fixture.root,
        &args,
        "ok",
        0,
        Some("item.schema.json"),
        repo.scope(),
    );
    assert!(value["data"]["id"].is_null());
    assert_eq!(value["data"]["source"], "no front matter\n");
    golden("cli_document_show_nonconforming", &value, &fixture.root);
    let human = run(&env, &fixture.root, &args, false);
    assert_eq!(human.status, 0);
    let text = std::str::from_utf8(&human.stdout).unwrap();
    assert!(text.contains("nonconforming"));
    assert!(text.contains("missing_front_matter"));
    assert_eq!(text.split_once("\n\n").unwrap().1, "no front matter\n");
    let list = run(&env, &fixture.root, &["document", "list"], false);
    assert_eq!(list.status, 0);
    assert!(
        std::str::from_utf8(&list.stdout)
            .unwrap()
            .lines()
            .any(|line| line.contains("nonconforming")
                && line.contains("docs/broken.md")
                && line.contains("missing_front_matter"))
    );
}

#[test]
fn folder_list_matches_f1_golden_including_empty_folders() {
    let (fixture, enabled) = items::contract_repository();
    fs::create_dir_all(fixture.root.join("docs/empty")).unwrap();
    fs::create_dir_all(fixture.root.join("docs/outer/inner")).unwrap();
    items::refresh_completely(&enabled.service, &fixture.root);
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    let args = ["folder", "list"];
    let value = checked(
        &env,
        &fixture.root,
        &args,
        "ok",
        0,
        Some("document_folder_list.schema.json"),
        repo.scope(),
    );
    assert_eq!(value["data"]["items"].as_array().unwrap().len(), 4);
    golden("cli_document_folder_list", &value, &fixture.root);
    let human = run(&env, &fixture.root, &args, false);
    assert_eq!(human.status, 0);
    assert!(human.stderr.is_empty());
    let text = std::str::from_utf8(&human.stdout).unwrap();
    assert_eq!(text.lines().count(), 4);
    assert!(text.contains("docs/outer/inner"));
}

#[test]
fn comment_list_matches_f1_threads_and_nonconforming_contract() {
    let (fixture, enabled) = items::contract_repository();
    for (id, parent, time, extra) in [
        (
            items::COMMENT_A,
            None,
            "2026-09-30T12:35:00Z",
            "created_by: Ada Lovelace <ada@example.invalid>\nreviewed: true\n",
        ),
        (
            items::COMMENT_B,
            Some(items::COMMENT_A),
            "2026-09-30T12:36:00Z",
            "",
        ),
        (items::COMMENT_C, None, "2026-09-30T12:37:00Z", ""),
    ] {
        items::write_comment(&fixture.root, items::TICKET_A, id, parent, time, extra);
    }
    let sentinel = "CLI_COMMENT_PARSER_SENTINEL";
    items::write(
        &fixture.root,
        &items::comment_path(items::TICKET_A, items::COMMENT_D),
        &format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: comment\n{sentinel}: 1\n{sentinel}: 2\n---\n{sentinel}\n"
        ),
    );
    items::refresh_completely(&enabled.service, &fixture.root);
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    let args = ["comment", "list", "--id", items::TICKET_A];
    let value = checked(
        &env,
        &fixture.root,
        &args,
        "ok",
        0,
        Some("comment_list.schema.json"),
        item_scope(repo.scope(), items::TICKET_A),
    );
    assert_eq!(value["data"]["items"].as_array().unwrap().len(), 4);
    assert_eq!(value["data"]["items"][1]["depth"], 1);
    assert!(!value.to_string().contains(sentinel));
    golden("cli_comment_list", &value, &fixture.root);
    let human = run(&env, &fixture.root, &args, false);
    assert_eq!(human.status, 0);
    assert!(human.stderr.is_empty());
    let text = std::str::from_utf8(&human.stdout).unwrap();
    assert!(
        text.lines()
            .any(|line| line.starts_with(&format!("  {}", items::COMMENT_B)))
    );
    assert!(text.contains("nonconforming"));
    assert!(!text.contains(sentinel));
    assert!(!text.contains("Body of")); // Requested bodies/sources belong to show only.
}

#[test]
fn every_ticket_filter_matches_the_same_f1_filter() {
    let (fixture, enabled) = items::contract_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    let cases = [
        (vec![], TicketFilter::default()),
        (
            vec!["--closure", "open"],
            TicketFilter {
                closure: ClosureFilter::Open,
                ..Default::default()
            },
        ),
        (
            vec!["--closure", "closed"],
            TicketFilter {
                closure: ClosureFilter::Closed,
                ..Default::default()
            },
        ),
        (
            vec!["--closure", "all"],
            TicketFilter {
                closure: ClosureFilter::All,
                ..Default::default()
            },
        ),
        (
            vec!["--status", "done"],
            TicketFilter {
                status: Some("done".into()),
                ..Default::default()
            },
        ),
        (
            vec!["--type", "bug"],
            TicketFilter {
                ticket_type: Some("bug".into()),
                ..Default::default()
            },
        ),
        (
            vec!["--project", "manyhands"],
            TicketFilter {
                project: Some("manyhands".into()),
                ..Default::default()
            },
        ),
        (
            vec!["--slug", "MH-VH-K9X2B"],
            TicketFilter {
                slug: Some("MH-VH-K9X2B".into()),
                ..Default::default()
            },
        ),
        (
            vec!["--ready"],
            TicketFilter {
                readiness: Some(ReadinessFilter::Ready),
                ..Default::default()
            },
        ),
        (
            vec!["--blocked"],
            TicketFilter {
                readiness: Some(ReadinessFilter::Blocked),
                ..Default::default()
            },
        ),
        (
            vec![
                "--status",
                "open",
                "--type",
                "task",
                "--project",
                "manyhands",
                "--slug",
                items::SHARED_SLUG,
                "--closure",
                "open",
                "--blocked",
            ],
            TicketFilter {
                status: Some("open".into()),
                ticket_type: Some("task".into()),
                project: Some("manyhands".into()),
                slug: Some(items::SHARED_SLUG.into()),
                closure: ClosureFilter::Open,
                readiness: Some(ReadinessFilter::Blocked),
            },
        ),
        (
            vec!["--status", "absent"],
            TicketFilter {
                status: Some("absent".into()),
                ..Default::default()
            },
        ),
    ];
    for (options, filter) in cases {
        let mut args = vec!["ticket", "list"];
        args.extend(options);
        let value = checked(
            &env,
            &fixture.root,
            &args,
            "ok",
            0,
            Some("item_list.schema.json"),
            repo.scope(),
        );
        let expected = enabled.service.list_tickets(&repo, &filter).unwrap();
        let expected: Vec<_> = expected
            .items
            .iter()
            .map(|item| item.id.as_deref())
            .collect();
        let actual: Vec<_> = value["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["id"].as_str())
            .collect();
        assert_eq!(actual, expected, "{args:?}");
        if expected.is_empty() {
            let human = run(&env, &fixture.root, &args, false);
            assert_eq!(human.status, 0);
            assert!(human.stdout.is_empty());
            assert!(human.stderr.is_empty());
        }
    }
    checked(
        &env,
        &fixture.root,
        &["ticket", "list", "--ready", "--blocked"],
        "usage_error",
        2,
        None,
        Scope::default(),
    );
}

#[test]
fn kind_mismatch_has_item_scope_for_ids_and_repository_scope_for_both_paths() {
    let (fixture, enabled) = items::contract_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    for (resource, id, path) in [
        (
            "document",
            items::TICKET_A,
            items::ticket_path(items::TICKET_A),
        ),
        ("ticket", items::DOCUMENT_A, "docs/guide.md".into()),
    ] {
        checked(
            &env,
            &fixture.root,
            &[resource, "show", "--id", id],
            "item_not_found",
            2,
            None,
            item_scope(repo.scope(), id),
        );
        checked(
            &env,
            &fixture.root,
            &[resource, "show", "--path", &path],
            "item_not_found",
            2,
            None,
            repo.scope(),
        );
    }
}

#[test]
fn bad_selectors_and_absent_ticket_are_scoped_input_failures() {
    let (fixture, enabled) = items::contract_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    for (args, code) in [
        (["ticket", "show", "--id", items::SHARED_SLUG], "invalid_id"),
        (
            ["document", "show", "--path", "docs/absent.md"],
            "path_not_found",
        ),
        (
            ["document", "show", "--path", "docs/../outside.md"],
            "invalid_path",
        ),
    ] {
        checked(&env, &fixture.root, &args, code, 2, None, repo.scope());
    }
    let args = ["ticket", "show", "--id", items::TICKET_ABSENT];
    checked(
        &env,
        &fixture.root,
        &args,
        "item_not_found",
        2,
        None,
        item_scope(repo.scope(), items::TICKET_ABSENT),
    );
    let human = run(&env, &fixture.root, &args, false);
    assert_eq!(human.status, 2);
    assert!(human.stdout.is_empty());
    cli::assert_cli_text_golden(
        "item_not_found_text",
        std::str::from_utf8(&human.stderr).unwrap(),
        &[(repo.root().to_str().unwrap(), "<repository>")],
    );
}

#[test]
fn content_resolution_precedes_id_parsing() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let env = CliEnv::new(data.path().to_owned());
    let scope = Scope {
        repository: Some(
            fixture
                .root
                .canonicalize()
                .unwrap()
                .to_str()
                .unwrap()
                .into(),
        ),
        ..Scope::default()
    };
    for resource in ["document", "ticket", "comment"] {
        let verb = if resource == "comment" {
            "list"
        } else {
            "show"
        };
        checked(
            &env,
            &fixture.root,
            &[resource, verb, "--id", items::SHARED_SLUG],
            "repository_not_registered",
            3,
            None,
            scope.clone(),
        );
    }
}

#[test]
fn stale_list_is_success_with_a_human_note_and_show_detects_file_ahead_of_index() {
    let (fixture, enabled) = items::contract_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    let file = fs::OpenOptions::new()
        .write(true)
        .open(fixture.root.join("docs/guide.md"))
        .unwrap();
    file.set_modified(SystemTime::now() + Duration::from_secs(30))
        .unwrap();
    let value = checked(
        &env,
        &fixture.root,
        &["document", "show", "--id", items::DOCUMENT_A],
        "ok",
        0,
        Some("item.schema.json"),
        item_scope(repo.scope(), items::DOCUMENT_A),
    );
    assert_eq!(value["data"]["index"]["state"], "stale");
    items::index(enabled.data_directory.path())
        .execute("UPDATE repositories SET refresh_required = 1", [])
        .unwrap();
    let args = ["ticket", "list"];
    let value = checked(
        &env,
        &fixture.root,
        &args,
        "ok",
        0,
        Some("item_list.schema.json"),
        repo.scope(),
    );
    assert_eq!(value["data"]["index"]["state"], "stale");
    let human = run(&env, &fixture.root, &args, false);
    assert_eq!(human.status, 0);
    assert!(!human.stdout.is_empty());
    assert_eq!(human.stderr, b"note: Index is stale.\n");
}
