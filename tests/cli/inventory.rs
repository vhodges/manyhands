//! C5's inventory (R-9; the design's Inventory And Evidence): every approved
//! command is accounted for. The catalog is the binary's own `--help --json`,
//! compared with the mutation boundary's `bound_commands()`, the CLI contract
//! RFC's taxonomy and the relationships RFC's table, the input schemas and
//! examples, and the output schemas of each verb's data.
//!
//! Exit behaviour is the exit tables' (`exits.rs`, `authoring_exits.rs`,
//! `collaboration_exits.rs`), and recovery is `recovery_tables.rs`'s.

use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use manyhands::repository::request_store;
use serde_json::Value;

use crate::support::{
    cli::{CliEnv, json, run_cli},
    schema,
};

const RFC: &str = include_str!("../../docs/RFC/cli-contract.md");
const RELATIONSHIPS_RFC: &str =
    include_str!("../../docs/RFC/ticket-relationships-and-short-codes.md");

/// The boundary's two commands the CLI does not catalog: the desktop's context
/// claim and release (owner question 5).
const EXEMPT: &[&str] = &["context prepare", "context release"];

/// Every catalogued verb that takes no `--input`. With the verbs that do, it is
/// the whole catalog, and the two never share a verb.
const NO_INPUT: &[&str] = &[
    // C1's reads.
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
    // C2's: selected by flags, or refusing input.
    "repo remove",
    "key select",
    "key clear",
    "key remove",
    "key delete",
    "host observe",
    "request show",
    "request cancel",
    // C3's.
    "index refresh",
    "index rebuild",
    "path observe",
    "ticket check",
    // C4's.
    "item sync",
    "repo sync",
    "item republish",
    "conflict show",
    "operation resume",
    "operation abandon",
    // C5's (R-2: the lifecycle verbs take none).
    "document promote",
    "ticket close",
    "poll once",
    "poll pause",
    "poll resume",
    "context observe-local",
    "context show-local",
];

/// The output schemas a verb's `data` is published by: a read's, a mutation's
/// result and its preview's. An empty list is a verb whose data is always null.
const DATA: &[(&str, &[&str])] = &[
    ("repo list", &["repository_list"]),
    ("repo inspect", &["repository_inspection"]),
    ("repo identity", &["identity"]),
    ("remote list", &["remote_list"]),
    ("key list", &["key_list"]),
    ("key show", &["key"]),
    ("key public", &["public_key"]),
    ("host list", &["host_pin_list"]),
    ("host inspect", &["host_pin"]),
    ("document list", &["item_list"]),
    ("document show", &["item"]),
    ("folder list", &["document_folder_list"]),
    ("ticket list", &["item_list"]),
    ("ticket show", &["item"]),
    ("ticket ready", &["item_list"]),
    ("ticket blocked", &["item_list"]),
    ("ticket deps", &["dependency_tree"]),
    ("ticket children", &["item_list"]),
    ("ticket cycles", &["cycle_list"]),
    ("ticket plan", &["plan"]),
    ("ticket critical-path", &["item_list"]),
    ("ticket find", &["item_list"]),
    ("comment list", &["comment_list"]),
    ("index status", &["index_status"]),
    ("poll status", &["polling_status"]),
    ("operation list", &["operation_list"]),
    ("operation show", &["operation"]),
    ("id new", &["new_id"]),
    // A create or enable answers with its preview and no other data.
    ("repo create", &["preview"]),
    ("repo enable", &["preview"]),
    ("repo remove", &["repository_removal_preview"]),
    (
        "repo identity-set",
        &["identity_preview", "identity_mutation"],
    ),
    ("remote add", &["remote_mutation"]),
    (
        "remote remove",
        &["remote_removal_preview", "remote_mutation"],
    ),
    ("remote select", &["remote_mutation"]),
    ("key generate", &["key_mutation"]),
    ("key import", &["key_mutation"]),
    ("key select", &["key_mutation"]),
    ("key clear", &["key_mutation"]),
    ("key remove", &["key_mutation"]),
    ("key delete", &["key_deletion_preview", "key_mutation"]),
    (
        "host approve",
        &["host_preview", "host_mutation", "host_replacement"],
    ),
    (
        "host replace",
        &["host_preview", "host_mutation", "host_replacement"],
    ),
    ("host observe", &["host_observation"]),
    ("request show", &["request"]),
    ("request cancel", &["cancel"]),
    ("folder create", &[]),
    ("document create", &["document_mutation"]),
    ("document save", &["document_mutation"]),
    ("document move", &["document_mutation"]),
    ("document repair", &["repair_preview", "repair_mutation"]),
    ("ticket create", &["ticket_mutation"]),
    ("ticket save", &["ticket_mutation"]),
    ("ticket repair", &["repair_preview", "repair_mutation"]),
    ("ticket slug-assign", &["ticket_mutation"]),
    ("index refresh", &[]),
    ("index rebuild", &[]),
    ("path observe", &["path_observation"]),
    ("ticket check", &["relationship_check"]),
    ("comment add", &["comment_mutation"]),
    (
        "item sync",
        &[
            "synchronization_mutation",
            "synchronization_conflict_mutation",
        ],
    ),
    (
        "repo sync",
        &[
            "synchronization_mutation",
            "synchronization_conflict_mutation",
        ],
    ),
    // A republish's result is its effects; only its preview has data.
    ("item republish", &["item_republish_preview"]),
    ("conflict show", &["conflict"]),
    ("conflict resolve", &["conflict_resolution_mutation"]),
    (
        "operation resume",
        &[
            "comment_mutation",
            "synchronization_conflict_mutation",
            "local_context_sync_mutation",
            "lifecycle_mutation",
        ],
    ),
    (
        "operation abandon",
        &["operation_abandon_preview", "operation_mutation"],
    ),
    (
        "document promote",
        &[
            "lifecycle_preview",
            "lifecycle_blocked",
            "lifecycle_mutation",
        ],
    ),
    (
        "ticket close",
        &[
            "lifecycle_preview",
            "lifecycle_blocked",
            "lifecycle_mutation",
        ],
    ),
    // A first call's host stop carries the synchronization's host data.
    ("poll once", &["poll_mutation", "synchronization_mutation"]),
    ("poll configure", &["poll_policy_mutation"]),
    ("poll pause", &["poll_policy_mutation"]),
    ("poll resume", &["poll_policy_mutation"]),
    ("context observe-local", &["local_context_sync_observation"]),
    (
        "context show-local",
        &[
            "local_context_sync_inspection",
            "local_context_conflict_content",
        ],
    ),
    ("context sync-local", &["local_context_sync_mutation"]),
    ("context resolve-local", &["local_context_sync_mutation"]),
];

fn manifest() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn set<'a>(values: impl IntoIterator<Item = &'a str>) -> BTreeSet<&'a str> {
    values.into_iter().collect()
}

fn help(args: &[&str]) -> Value {
    let temp = tempfile::tempdir().unwrap();
    let env = CliEnv::new(temp.path().join("absent-data"));
    let run = run_cli(&args.iter().map(OsStr::new).collect::<Vec<_>>(), &env);
    assert_eq!(run.status, 0, "{args:?}");
    let value = json(&run);
    schema::check_published("envelope.schema.json", &value).unwrap();
    schema::check_published("cli_help.schema.json", &value["data"]).unwrap();
    assert_eq!(value["command"], "cli help", "{args:?}");
    assert!(!env.data.exists(), "{args:?}: help opened no data");
    value
}

/// The root help's commands, by name.
fn catalog() -> BTreeMap<String, Value> {
    let root = help(&["--help", "--json"]);
    let commands: BTreeMap<String, Value> = root["data"]["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|command| {
            (
                command["command"].as_str().unwrap().to_owned(),
                command.clone(),
            )
        })
        .collect();
    assert_eq!(
        commands.len(),
        root["data"]["commands"].as_array().unwrap().len(),
        "a verb appears twice"
    );
    commands
}

fn options(command: &Value) -> BTreeSet<String> {
    command["options"]
        .as_array()
        .unwrap()
        .iter()
        .map(|option| option["name"].as_str().unwrap().to_owned())
        .collect()
}

/// A mutation declares `--prepare`, and a read never does (C2's grammar).
fn mutates(command: &Value) -> bool {
    options(command).contains("--prepare")
}

/// The commands of the first column of each row of the table that follows
/// `header` in `section`, up to the first blank line after it.
fn table_commands<'a>(section: &'a str, header: &str) -> BTreeSet<&'a str> {
    let table = section
        .split(header)
        .nth(1)
        .unwrap_or_else(|| panic!("no table {header:?}"))
        .split("\n\n")
        .next()
        .unwrap();
    let mut commands = BTreeSet::new();
    for row in table.lines().filter(|line| line.starts_with("| `")) {
        let first = row.split('|').nth(1).unwrap();
        commands.extend(first.split('`').skip(1).step_by(2));
    }
    assert!(!commands.is_empty(), "{header:?}");
    commands
}

/// The CLI contract RFC's taxonomy table, and the relationships RFC's.
fn rfc_commands() -> BTreeSet<&'static str> {
    let taxonomy = RFC
        .split("\n## Command taxonomy\n")
        .nth(1)
        .unwrap()
        .split("\n### ")
        .next()
        .unwrap();
    let relationships = RELATIONSHIPS_RFC
        .split("\n## CLI\n")
        .nth(1)
        .unwrap()
        .split("\n## ")
        .next()
        .unwrap();
    let header = "| Commands | Inputs and behavior |";
    let mut commands = table_commands(taxonomy, header);
    commands.extend(table_commands(relationships, header));
    commands
}

#[test]
fn every_bound_mutation_is_a_catalogued_mutating_verb_less_the_two_exempt() {
    let catalog = catalog();
    let mutating = set(catalog
        .iter()
        .filter(|(_, command)| mutates(command))
        .map(|(name, _)| name.as_str()));
    let bound = request_store::bound_commands();
    let bound = set(bound.iter().copied());
    for exempt in EXEMPT {
        assert!(bound.contains(exempt), "{exempt} is bound");
        assert!(!catalog.contains_key(*exempt), "{exempt} is not catalogued");
    }
    let expected = set(bound.difference(&set(EXEMPT.iter().copied())).copied());
    assert_eq!(mutating, expected);
    assert_eq!(mutating.len(), 41);
}

#[test]
fn the_catalog_and_the_rfc_taxonomies_name_the_same_verbs() {
    let catalog = catalog();
    let catalogued = set(catalog.keys().map(String::as_str));
    let documented = rfc_commands();
    let missing: Vec<_> = catalogued.difference(&documented).collect();
    let extra: Vec<_> = documented.difference(&catalogued).collect();
    assert!(
        missing.is_empty(),
        "catalogued, not in an RFC table: {missing:?}"
    );
    assert!(
        extra.is_empty(),
        "in an RFC table, not catalogued: {extra:?}"
    );
    assert_eq!(catalogued.len(), 77);
}

#[test]
fn no_verb_is_planned() {
    for (name, command) in catalog() {
        assert_eq!(command["available"], true, "{name}");
        let cycle = command["delivered_by"].as_str().unwrap();
        assert!(
            ["C1", "C2", "C3", "C4", "C5"].contains(&cycle),
            "{name}: {cycle}"
        );
    }
}

/// `<resource> <verb> --help --json` answers for every verb, with the root's
/// entry for it, its options included.
#[test]
fn every_verb_s_own_help_answers_with_its_catalog_options() {
    for (name, entry) in catalog() {
        let mut args: Vec<&str> = name.split(' ').collect();
        args.extend(["--help", "--json"]);
        let value = help(&args);
        let commands = value["data"]["commands"].as_array().unwrap();
        assert_eq!(commands.len(), 1, "{name}");
        assert_eq!(commands[0], entry, "{name}");
    }
}

fn input_file(name: &str) -> String {
    name.replace([' ', '-'], "_")
}

/// The JSON files of a directory: what it holds besides a placeholder such as
/// `.gitkeep`.
fn files_in(directory: &Path) -> BTreeSet<String> {
    fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".json"))
        .collect()
}

/// Every verb that takes `--input` has one closed input schema and one example
/// that the schema accepts; every other verb is listed in `NO_INPUT`.
#[test]
fn every_input_taking_verb_has_one_schema_and_one_example_and_every_other_takes_none() {
    let catalog = catalog();
    let taking = set(catalog
        .iter()
        .filter(|(_, command)| options(command).contains("--input"))
        .map(|(name, _)| name.as_str()));
    let none = set(NO_INPUT.iter().copied());
    assert_eq!(none.len(), NO_INPUT.len(), "a verb is listed twice");
    assert!(
        taking.is_disjoint(&none),
        "{:?}",
        taking.intersection(&none)
    );
    assert_eq!(
        set(taking.iter().chain(&none).copied()),
        set(catalog.keys().map(String::as_str))
    );
    let schemas_directory = manifest().join("schemas/input/v1");
    let examples_directory = manifest().join("tests/fixtures/cli_input_v1");
    let mut schemas = files_in(&schemas_directory);
    // Shared by `$ref`, and no verb's own.
    assert!(schemas.remove("commit_identity.schema.json"));
    let expected_schemas: BTreeSet<String> = taking
        .iter()
        .map(|name| format!("{}.schema.json", input_file(name)))
        .collect();
    let expected_examples: BTreeSet<String> = taking
        .iter()
        .map(|name| format!("{}.json", input_file(name)))
        .collect();
    assert_eq!(schemas, expected_schemas);
    assert_eq!(files_in(&examples_directory), expected_examples);
    for name in &taking {
        let file = input_file(name);
        let example: Value = serde_json::from_str(
            &fs::read_to_string(examples_directory.join(format!("{file}.json"))).unwrap(),
        )
        .unwrap();
        schema::check(&schemas_directory, &format!("{file}.schema.json"), &example)
            .unwrap_or_else(|problem| panic!("{name}: {problem}"));
    }
    assert_eq!(taking.len(), 24);
}

/// Every golden envelope of the CLI and of the mutation boundary, by command.
fn goldens() -> Vec<(PathBuf, Value)> {
    let mut found = Vec::new();
    for directory in ["tests/fixtures/cli_v1", "tests/fixtures/mutation_v1"] {
        for entry in fs::read_dir(manifest().join(directory)).unwrap() {
            let path = entry.unwrap().path();
            if path
                .extension()
                .is_some_and(|extension| extension == "json")
            {
                let value: Value =
                    serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
                found.push((path, value));
            }
        }
    }
    found
}

/// Every verb's data type has a published output schema, and every golden of
/// the verb whose data is not null matches one of its schemas.
#[test]
fn every_verb_s_data_has_an_output_schema_and_its_goldens_match_it() {
    let catalog = catalog();
    let data: BTreeMap<&str, &[&str]> = DATA.iter().copied().collect();
    assert_eq!(data.len(), DATA.len(), "a verb is listed twice");
    assert_eq!(
        set(data.keys().copied()),
        set(catalog.keys().map(String::as_str))
    );
    let published = files_in(&schema::schema_directory());
    for (name, schemas) in &data {
        for schema_name in *schemas {
            assert!(
                published.contains(&format!("{schema_name}.schema.json")),
                "{name}: no {schema_name}"
            );
        }
    }
    let mut checked = 0;
    for (path, golden) in goldens() {
        let Some(command) = golden["command"].as_str() else {
            continue;
        };
        let Some(schemas) = data.get(command) else {
            continue;
        };
        if golden["data"].is_null() {
            continue;
        }
        assert!(
            schemas.iter().any(|schema_name| schema::check_published(
                &format!("{schema_name}.schema.json"),
                &golden["data"]
            )
            .is_ok()),
            "{}: {command}'s data matches none of {schemas:?}",
            path.display()
        );
        checked += 1;
    }
    assert!(checked > 200, "{checked}");
}
