use std::{ffi::OsStr, fs, path::Path};

use manyhands::{
    repository::{DependencyDirection, ReadinessFilter, TicketFilter},
    results::Scope,
};
use serde_json::Value;

use crate::support::{
    self,
    cli::{self, CliEnv, CliRun},
    golden, items, schema,
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
        assert_eq!(value["outcome"], "success");
        schema::check_published(data_schema, &value["data"]).unwrap();
    } else {
        assert!(value["data"].is_null());
    }
    value
}

fn item_scope(mut scope: Scope, id: &str) -> Scope {
    scope.item_id = Some(id.to_owned());
    scope
}

// Each case is the exact F1 relationship-golden invocation and scope. Compare
// the normalized bytes to F1 too, so update mode cannot bless contract drift.
fn contract(verb: &str, options: &[&str], data_schema: &str, id: Option<&str>, needles: &[&str]) {
    let (fixture, enabled) = items::relationship_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    let mut args = vec!["ticket", verb];
    args.extend(options);
    let scope = id.map_or_else(|| repo.scope(), |id| item_scope(repo.scope(), id));
    let value = checked(
        &env,
        &fixture.root,
        &args,
        "ok",
        0,
        Some(data_schema),
        scope,
    );
    let tickets = enabled
        .service
        .list_tickets(&repo, &TicketFilter::default())
        .unwrap();
    let mut placeholders = vec![(repo.root().to_str().unwrap(), "<repository>")];
    for ticket in &tickets.items {
        if let Some(time) = ticket.index.refreshed_at.as_deref() {
            placeholders.push((time, "<refreshed-at>"));
        }
        if let Some(oid) = ticket.context.head_oid.as_deref() {
            placeholders.push((oid, "<head-oid>"));
        }
    }
    placeholders.sort();
    placeholders.dedup();
    let name = format!("ticket_{}", verb.replace('-', "_"));
    let expected = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/read_v1/{name}.json")),
    )
    .unwrap();
    assert_eq!(golden::render(&value, &placeholders), expected);
    cli::assert_cli_golden(&format!("cli_{name}"), &value, &placeholders);
    let human = run(&env, &fixture.root, &args, false);
    assert_eq!(human.status, 0);
    assert!(human.stderr.is_empty());
    let text = std::str::from_utf8(&human.stdout).unwrap();
    for needle in needles {
        assert!(text.contains(needle), "missing {needle}: {text}");
    }
    if verb == "cycles" {
        assert_eq!(text.lines().count(), 2);
    }
}

#[test]
fn ready_matches_f1_golden_and_human() {
    contract(
        "ready",
        &[],
        "item_list.schema.json",
        None,
        &[items::RELATED_B, items::SHARED_SLUG],
    );
}

#[test]
fn blocked_matches_f1_golden_and_human_reasons() {
    contract(
        "blocked",
        &[],
        "item_list.schema.json",
        None,
        &[
            items::RELATED_C,
            items::SHARED_SLUG,
            "\n  open_dependency",
            "\n  unresolved_dependency",
            "\n  dependency_cycle",
        ],
    );
}

#[test]
fn deps_matches_f1_golden_and_human_repetition() {
    contract(
        "deps",
        &[
            "--id",
            items::RELATED_E,
            "--direction",
            "both",
            "--depth",
            "2",
        ],
        "dependency_tree.schema.json",
        Some(items::RELATED_E),
        &[
            items::RELATED_E,
            "  01ARZ3NDEKTSV4RRFFQ69G5FR5",
            "    01ARZ3NDEKTSV4RRFFQ69G5FR4",
            "(repeated)",
        ],
    );
}

#[test]
fn children_matches_f1_golden_and_human() {
    contract(
        "children",
        &["--id", items::RELATED_B],
        "item_list.schema.json",
        Some(items::RELATED_B),
        &[items::RELATED_G, items::RELATED_C, items::SHARED_SLUG],
    );
}

#[test]
fn cycles_matches_f1_golden_and_human() {
    contract(
        "cycles",
        &[],
        "cycle_list.schema.json",
        None,
        &["deps", "parent", items::RELATED_E, items::RELATED_F],
    );
}

#[test]
fn plan_matches_f1_golden_and_human_batches_and_reasons() {
    contract(
        "plan",
        &[],
        "plan.schema.json",
        None,
        &[
            "Batch 1:",
            "Batch 2:",
            "Unplannable:",
            items::RELATED_B,
            items::SHARED_SLUG,
            "unresolved_dependency",
            "unplannable_dependency",
            "dependency_cycle",
        ],
    );
}

#[test]
fn critical_path_matches_f1_golden_and_human() {
    contract(
        "critical-path",
        &[],
        "item_list.schema.json",
        None,
        &[items::RELATED_B, items::RELATED_C, items::SHARED_SLUG],
    );
}

#[test]
fn find_uppercase_shared_slug_returns_every_match_and_matches_f1_golden_and_human() {
    contract(
        "find",
        &["--slug", &items::SHARED_SLUG.to_uppercase()],
        "item_list.schema.json",
        None,
        &[items::RELATED_C, items::RELATED_B, items::SHARED_SLUG],
    );
}

#[test]
fn dependency_directions_and_depth_limits_match_f1_including_defaults() {
    let (fixture, enabled) = items::relationship_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    for (name, direction) in [
        ("up", DependencyDirection::Up),
        ("down", DependencyDirection::Down),
        ("both", DependencyDirection::Both),
    ] {
        for depth in [None, Some(0), Some(1)] {
            let depth_text = depth.map(|depth: u32| depth.to_string());
            let mut args = vec![
                "ticket",
                "deps",
                "--id",
                items::RELATED_E,
                "--direction",
                name,
            ];
            if let Some(depth) = &depth_text {
                args.extend(["--depth", depth]);
            }
            let value = checked(
                &env,
                &fixture.root,
                &args,
                "ok",
                0,
                Some("dependency_tree.schema.json"),
                item_scope(repo.scope(), items::RELATED_E),
            );
            let expected = enabled
                .service
                .ticket_dependencies(&repo, &items::item_id(items::RELATED_E), direction, depth)
                .unwrap();
            assert_eq!(value["data"], serde_json::to_value(expected).unwrap());
        }
    }
    let value = checked(
        &env,
        &fixture.root,
        &["ticket", "deps", "--id", items::RELATED_B],
        "ok",
        0,
        Some("dependency_tree.schema.json"),
        item_scope(repo.scope(), items::RELATED_B),
    );
    assert_eq!(value["data"]["direction"], "down");
    assert!(value["data"]["depth"].is_null());
    let expected = enabled
        .service
        .ticket_dependencies(
            &repo,
            &items::item_id(items::RELATED_B),
            DependencyDirection::Down,
            None,
        )
        .unwrap();
    assert_eq!(value["data"], serde_json::to_value(expected).unwrap());
}

#[test]
fn invalid_dependency_options_are_usage_errors_in_both_modes() {
    let (fixture, enabled) = items::relationship_repository();
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    for (flag, value) in [("--depth", "-1"), ("--direction", "sideways")] {
        let args = ["ticket", "deps", "--id", items::RELATED_E, flag, value];
        checked(
            &env,
            &fixture.root,
            &args,
            "usage_error",
            2,
            None,
            Scope::default(),
        );
        let human = run(&env, &fixture.root, &args, false);
        assert_eq!(human.status, 2);
        assert!(human.stdout.is_empty());
        assert!(
            String::from_utf8(human.stderr)
                .unwrap()
                .contains("code: usage_error")
        );
    }
}

#[test]
fn dependency_document_id_is_item_not_found() {
    let (fixture, enabled) = items::contract_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    checked(
        &env,
        &fixture.root,
        &["ticket", "deps", "--id", items::DOCUMENT_A],
        "item_not_found",
        2,
        None,
        item_scope(repo.scope(), items::DOCUMENT_A),
    );
}

#[test]
fn relationship_selectors_are_strict_and_resolve_repository_first() {
    let (fixture, enabled) = items::relationship_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let unregistered = support::born_repository();
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    for verb in ["deps", "children"] {
        for id in [items::SHARED_SLUG, &items::RELATED_B.to_lowercase()] {
            let args = ["ticket", verb, "--id", id];
            checked(
                &env,
                &fixture.root,
                &args,
                "invalid_id",
                2,
                None,
                repo.scope(),
            );
            checked(
                &env,
                &unregistered.root,
                &args,
                "repository_not_registered",
                3,
                None,
                Scope {
                    repository: Some(
                        unregistered
                            .root
                            .canonicalize()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .into(),
                    ),
                    ..Scope::default()
                },
            );
        }
        checked(
            &env,
            &fixture.root,
            &["ticket", verb, "--id", items::TICKET_ABSENT],
            "item_not_found",
            2,
            None,
            item_scope(repo.scope(), items::TICKET_ABSENT),
        );
    }
}

#[test]
fn unknown_and_empty_slugs_are_successful_empty_lists_in_both_modes() {
    let (fixture, enabled) = items::relationship_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    for slug in ["mh-zz-zzzzz", "", "malformed slug"] {
        let args = ["ticket", "find", "--slug", slug];
        let value = checked(
            &env,
            &fixture.root,
            &args,
            "ok",
            0,
            Some("item_list.schema.json"),
            repo.scope(),
        );
        assert_eq!(value["data"]["items"], serde_json::json!([]));
        let human = run(&env, &fixture.root, &args, false);
        assert_eq!(human.status, 0);
        assert!(human.stdout.is_empty());
        assert!(human.stderr.is_empty());
    }
}

#[test]
fn readiness_and_plan_filters_match_f1_and_human_only_shows_selected_tickets() {
    let (fixture, enabled) = items::relationship_repository();
    for (id, title, dependency) in [
        (items::RELATED_B, "B", items::RELATED_A),
        (items::RELATED_C, "C", items::RELATED_B),
    ] {
        items::write(
            &fixture.root,
            &items::ticket_path(id),
            &items::ticket_source_with(
                id,
                title,
                "feature",
                "active",
                &format!("project: selected\ndeps: [{dependency}]\n"),
            ),
        );
    }
    items::refresh_completely(&enabled.service, &fixture.root);
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    for verb in ["ready", "blocked", "plan"] {
        for (flag, text) in [
            ("--status", "active"),
            ("--type", "feature"),
            ("--project", "selected"),
            ("--status", "absent"),
            ("--type", "absent"),
            ("--project", "absent"),
        ] {
            let mut filter = TicketFilter::default();
            match flag {
                "--status" => filter.status = Some(text.into()),
                "--type" => filter.ticket_type = Some(text.into()),
                "--project" => filter.project = Some(text.into()),
                _ => unreachable!(),
            }
            filter.readiness = match verb {
                "ready" => Some(ReadinessFilter::Ready),
                "blocked" => Some(ReadinessFilter::Blocked),
                _ => None,
            };
            let expected = if verb == "plan" {
                serde_json::to_value(enabled.service.ticket_plan(&repo, &filter).unwrap()).unwrap()
            } else {
                serde_json::to_value(enabled.service.ticket_readiness(&repo, &filter).unwrap())
                    .unwrap()
            };
            let args = ["ticket", verb, flag, text];
            let value = checked(
                &env,
                &fixture.root,
                &args,
                "ok",
                0,
                Some(if verb == "plan" {
                    "plan.schema.json"
                } else {
                    "item_list.schema.json"
                }),
                repo.scope(),
            );
            assert_eq!(value["data"], expected);
            let human = run(&env, &fixture.root, &args, false);
            assert_eq!(human.status, 0);
            assert!(human.stderr.is_empty());
            let text_output = String::from_utf8(human.stdout).unwrap();
            if text == "absent" {
                assert!(text_output.is_empty());
            } else {
                assert!(text_output.contains(if verb == "blocked" {
                    items::RELATED_C
                } else {
                    items::RELATED_B
                }));
                assert!(!text_output.contains(items::RELATED_D));
                assert!(!text_output.contains(items::RELATED_E));
                if verb == "plan" {
                    assert!(text_output.contains("Batch 2:"));
                    assert!(text_output.contains(items::RELATED_C));
                }
            }
        }
    }
}
