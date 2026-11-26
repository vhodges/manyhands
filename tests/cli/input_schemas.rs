//! Input schemas have optional fields and a separate contract from output v1.

use std::{
    collections::BTreeSet,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use serde_json::Value;

use crate::support::{
    cli::{CliEnv, json, run_cli},
    schema,
};

const PENDING_REFERENCES: &[&str] = &[];

#[test]
fn host_input_schemas_match_closed_required_shapes_and_examples_reach_prepare() {
    for verb in ["approve", "replace"] {
        let file = format!("host_{verb}.schema.json");
        let example = read_json(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("tests/fixtures/cli_input_v1/host_{verb}.json")),
        );
        schema::check(&directory(), &file, &example).unwrap();
        for field in example.as_object().unwrap().keys() {
            let mut missing = example.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(schema::check(&directory(), &file, &missing).is_err());
            for bad in [Value::Null, serde_json::json!(1)] {
                let mut value = example.clone();
                value[field] = bad;
                assert!(schema::check(&directory(), &file, &value).is_err());
            }
        }
        let mut unknown = example;
        unknown["passphrase"] = "not-an-input-field".into();
        assert!(schema::check(&directory(), &file, &unknown).is_err());
    }
    super::mutation::host_verbs_match_prepare();
}

#[test]
fn poll_configure_schema_is_closed_and_its_example_is_accepted() {
    let file = "poll_configure.schema.json";
    let example = read_json(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/cli_input_v1/poll_configure.json"),
    );
    schema::check(&directory(), file, &example).unwrap();
    for bad in [
        serde_json::json!({}),
        serde_json::json!({"interval_seconds": "600"}),
        serde_json::json!({"interval_seconds": null}),
        serde_json::json!({"interval_seconds": 59}),
        serde_json::json!({"interval_seconds": 600.5}),
        serde_json::json!({"interval_seconds": 600, "enabled": false}),
    ] {
        assert!(schema::check(&directory(), file, &bad).is_err(), "{bad}");
    }
    super::polling::the_example_input_is_accepted(&example);
}

#[test]
fn context_sync_local_schema_is_closed_and_its_example_is_accepted() {
    let file = "context_sync_local.schema.json";
    let example = read_json(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/cli_input_v1/context_sync_local.json"),
    );
    schema::check(&directory(), file, &example).unwrap();
    for bad in [
        serde_json::json!({}),
        serde_json::json!({"observation": 7}),
        serde_json::json!({"observation": null}),
        serde_json::json!({"observation": "x", "kind": "ticket"}),
    ] {
        assert!(schema::check(&directory(), file, &bad).is_err(), "{bad}");
    }
    super::local_sync::the_example_input_is_accepted(&example);
}

#[test]
fn context_resolve_local_schema_is_closed_and_its_example_is_accepted() {
    let file = "context_resolve_local.schema.json";
    let example = read_json(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/cli_input_v1/context_resolve_local.json"),
    );
    schema::check(&directory(), file, &example).unwrap();
    for bad in [
        serde_json::json!({}),
        serde_json::json!({"observation": "x"}),
        serde_json::json!({"observation": "x", "resolutions": [{"token": "t"}]}),
        serde_json::json!({"observation": "x", "resolutions": [{"token": "t", "source": 1}]}),
        serde_json::json!({"observation": "x", "resolutions": [], "kind": "ticket"}),
    ] {
        assert!(schema::check(&directory(), file, &bad).is_err(), "{bad}");
    }
    super::local_sync::the_resolve_example_input_is_accepted(&example);
}

#[test]
fn protection_accepts_passphrase_and_none_only() {
    let f = super::keys::Fixture::new();
    for protection in ["passphrase", "none", "secret"] {
        let value = serde_json::json!({"label":"Generated key", "protection":protection});
        let allowed = protection != "secret";
        super::keys::call(
            &f.env,
            "generate",
            &["--input", "-", "--prepare"],
            &serde_json::to_vec(&value).unwrap(),
            2,
            if allowed {
                "not_confirmable"
            } else {
                "invalid_input"
            },
        );
        assert_eq!(
            schema::check(&directory(), "key_generate.schema.json", &value).is_ok(),
            allowed
        );
    }
    super::keys::no_generated_key_or_request(&f.env, super::mutation::REQUEST);
}

#[test]
fn keys_import_schema_matches_required_optional_and_closed_shapes() {
    let file = "key_import.schema.json";
    schema::validate(&directory(), file).unwrap();
    let definition = read_json(&directory().join(file));
    assert_closed(&definition);
    assert_eq!(
        definition["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert!(definition["title"].as_str().is_some_and(|s| !s.is_empty()));
    let example: Value = serde_json::from_slice(super::keys::IMPORT).unwrap();
    schema::check(&directory(), file, &example).unwrap();
    let mut optional = example.clone();
    optional.as_object_mut().unwrap().remove("public_key_path");
    schema::check(&directory(), file, &optional).unwrap();
    for field in ["label", "private_key_path", "public_key_path"] {
        for bad in [Value::Null, serde_json::json!(1)] {
            let mut value = example.clone();
            value[field] = bad;
            assert!(schema::check(&directory(), file, &value).is_err());
        }
        if field != "public_key_path" {
            let mut value = example.clone();
            value.as_object_mut().unwrap().remove(field);
            assert!(schema::check(&directory(), file, &value).is_err());
        }
    }
    let mut unknown = example;
    unknown["passphrase"] = serde_json::json!("not-an-input-field");
    assert!(schema::check(&directory(), file, &unknown).is_err());
}

#[test]
fn remote_names_are_required_and_only_selection_is_nullable() {
    for verb in ["remove", "select"] {
        let schema = format!("remote_{verb}.schema.json");
        assert!(
            schema::check(&directory(), &schema, &serde_json::json!({"name":"origin"})).is_ok()
        );
        assert_eq!(
            schema::check(&directory(), &schema, &serde_json::json!({"name":null})).is_ok(),
            verb == "select"
        );
        for invalid in [
            serde_json::json!({}),
            serde_json::json!({"name":1}),
            serde_json::json!({"name":"origin","url":"extra"}),
        ] {
            assert!(schema::check(&directory(), &schema, &invalid).is_err());
        }
    }
}

/// `remote select` takes a mode and a forge, and a forge may be null, which
/// removes it (design Part F); `remote remove` takes neither.
#[test]
fn remote_select_takes_integration_and_forge() {
    let select = "remote_select.schema.json";
    for valid in [
        serde_json::json!({"name": "origin", "integration": "pull_request"}),
        serde_json::json!({"name": "origin", "integration": "direct"}),
        serde_json::json!({"name": "origin", "forge": null}),
        serde_json::json!({"name": null, "forge": {"kind": "github", "web_url": "https://x.example.invalid"}}),
        serde_json::json!({"name": "origin", "forge": {}}),
    ] {
        assert!(
            schema::check(&directory(), select, &valid).is_ok(),
            "{valid}"
        );
    }
    for invalid in [
        serde_json::json!({"name": "origin", "integration": "merge"}),
        serde_json::json!({"name": "origin", "integration": null}),
        serde_json::json!({"name": "origin", "forge": 1}),
        serde_json::json!({"name": "origin", "forge": {"colour": "red"}}),
        serde_json::json!({"name": "origin", "forge": {"kind": 1}}),
        serde_json::json!({"integration": "direct"}),
    ] {
        assert!(
            schema::check(&directory(), select, &invalid).is_err(),
            "{invalid}"
        );
    }
    for field in [
        ("integration", serde_json::json!("direct")),
        ("forge", serde_json::json!({"kind": "github"})),
    ] {
        let mut input = serde_json::json!({"name": "origin"});
        input[field.0] = field.1;
        assert!(
            schema::check(&directory(), "remote_remove.schema.json", &input).is_err(),
            "remove refuses {}",
            field.0
        );
    }
}

/// `repo enable` takes a publication remote, a mode and a forge; `repo create`
/// does not (design R-8): its schema and its parser refuse each, and the
/// enable's refuse what is not of their shapes.
#[test]
fn repo_create_refuses_the_enable_only_fields() {
    let example = read_json(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cli_input_v1/repo_enable.json"),
    );
    let fields = [
        ("publication_remote", serde_json::json!("origin")),
        ("integration", serde_json::json!("pull_request")),
        (
            "forge",
            serde_json::json!({"kind": "github", "web_url": "https://github.example.invalid"}),
        ),
    ];
    for (name, value) in &fields {
        let mut input = example.clone();
        input[name] = value.clone();
        assert!(
            schema::check(&directory(), "repo_enable.schema.json", &input).is_ok(),
            "enable takes {name}"
        );
        assert!(
            schema::check(&directory(), "repo_create.schema.json", &input).is_err(),
            "create refuses {name}"
        );
    }
    let mut all = example.clone();
    for (name, value) in &fields {
        all[name] = value.clone();
    }
    assert!(schema::check(&directory(), "repo_enable.schema.json", &all).is_ok());
    for bad in [
        serde_json::json!({"publication_remote": 1}),
        serde_json::json!({"publication_remote": null}),
        serde_json::json!({"integration": "merge"}),
        serde_json::json!({"integration": null}),
        serde_json::json!({"forge": null}),
        serde_json::json!({"forge": {"kind": 1}}),
        serde_json::json!({"forge": {"colour": "red"}}),
    ] {
        let mut input = example.clone();
        for (name, value) in bad.as_object().unwrap() {
            input[name] = value.clone();
        }
        assert!(
            schema::check(&directory(), "repo_enable.schema.json", &input).is_err(),
            "{bad}"
        );
    }

    // The parsers: create refuses each field before it looks at the
    // repository, and makes nothing.
    for (name, value) in &fields {
        let mut f = super::repository::Fixture::new("create");
        let mut input = example.clone();
        input[name] = value.clone();
        f.set_input(&serde_json::to_vec(&input).unwrap());
        super::mutation::checked(
            &f.args("create", &["--json", "--prepare"]),
            &f.env,
            &[],
            2,
            "invalid_input",
            None,
        );
        assert!(!f.root.exists(), "{name}: nothing was made");
    }

    // Enable takes them to the library: a remote the repository does not
    // have is `invalid_remote`, and a forge that is no forge is
    // `invalid_input`.
    for (field, code) in [
        (
            serde_json::json!({"publication_remote": "origin"}),
            "invalid_remote",
        ),
        (
            serde_json::json!({"publication_remote": "origin", "forge": {"kind": "no-such-forge"}}),
            "invalid_input",
        ),
    ] {
        let mut f = super::repository::Fixture::new("enable");
        let mut input = example.clone();
        for (name, value) in field.as_object().unwrap() {
            input[name] = value.clone();
        }
        f.set_input(&serde_json::to_vec(&input).unwrap());
        super::mutation::checked(
            &f.args("enable", &["--json", "--prepare"]),
            &f.env,
            &[],
            2,
            code,
            None,
        );
    }
}

fn directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas/input/v1")
}

// Independent of the bin-private helper: help is this test's command inventory.
fn schema_name(resource: &str, verb: &str) -> String {
    format!("{resource}_{}.schema.json", verb.replace('-', "_"))
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn schema_files() -> BTreeSet<String> {
    let files: BTreeSet<_> = fs::read_dir(directory())
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            assert!(entry.file_type().unwrap().is_file());
            let name = entry.file_name().into_string().unwrap();
            assert!(
                name.ends_with(".schema.json"),
                "unexpected schema entry: {name}"
            );
            name
        })
        .collect();
    assert!(!files.is_empty());
    files
}

fn references(node: &Value, found: &mut Vec<String>) {
    if let Some(reference) = node.get("$ref") {
        found.push(reference.as_str().unwrap().to_owned());
    }
    if let Some(properties) = node.get("properties").and_then(Value::as_object) {
        for property in properties.values() {
            references(property, found);
        }
    }
    if let Some(items) = node.get("items") {
        references(items, found);
    }
}

#[test]
fn every_input_taking_command_has_exactly_one_schema_and_every_schema_is_reached() {
    let temp = tempfile::tempdir().unwrap();
    let env = CliEnv::new(temp.path().join("absent-data"));
    // Help returns before terminal/transport/service startup, even before E4.
    let output = run_cli(&[OsStr::new("--json"), OsStr::new("--help")], &env);
    assert_eq!(output.status, 0);
    let help = json(&output);
    schema::check_published("cli_help.schema.json", &help["data"]).unwrap();
    assert!(!env.data.exists());

    let files = schema_files();
    let mut roots = BTreeSet::new();
    for command in help["data"]["commands"].as_array().unwrap() {
        if command["available"] != true
            || !command["options"]
                .as_array()
                .unwrap()
                .iter()
                .any(|option| option["name"] == "--input")
        {
            continue;
        }
        let (resource, verb) = command["command"]
            .as_str()
            .unwrap()
            .split_once(' ')
            .unwrap();
        let file = schema_name(resource, verb);
        assert!(files.contains(&file), "missing input schema: {file}");
        assert!(roots.insert(file), "two commands map to one input schema");
    }

    let mut pending: Vec<_> = roots.into_iter().collect();
    let mut reached = BTreeSet::new();
    while let Some(file) = pending.pop() {
        assert!(
            files.contains(&file),
            "referenced schema is not published: {file}"
        );
        if reached.insert(file.clone()) {
            // Validate sibling references before following them; no outside paths.
            schema::validate(&directory(), &file).unwrap();
            references(&read_json(&directory().join(&file)), &mut pending);
        }
    }
    for file in PENDING_REFERENCES {
        assert!(
            files.contains(*file),
            "pending schema is not published: {file}"
        );
        assert!(
            reached.insert((*file).to_owned()),
            "remove the now-reached pending reference: {file}"
        );
    }
    assert_eq!(reached, files, "orphaned input schema");
}

fn assert_closed(node: &Value) {
    if node.get("$ref").is_some() {
        return;
    }
    let allows_object = match node.get("type") {
        Some(Value::Array(types)) => types.iter().any(|kind| kind == "object"),
        Some(kind) => kind == "object",
        None => true,
    };
    if allows_object {
        assert_eq!(
            node["additionalProperties"], false,
            "input objects must be closed"
        );
    }
    if let Some(properties) = node.get("properties").and_then(Value::as_object) {
        for property in properties.values() {
            assert_closed(property);
        }
    }
    if let Some(items) = node.get("items") {
        assert_closed(items);
    }
}

#[test]
fn every_input_schema_uses_only_the_checked_keywords_and_is_closed() {
    for file in schema_files() {
        schema::validate(&directory(), &file).unwrap();
        let value = read_json(&directory().join(&file));
        assert_eq!(
            value["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert!(
            value["title"]
                .as_str()
                .is_some_and(|title| !title.is_empty())
        );
        assert_closed(&value);
    }
}

#[test]
fn every_example_input_validates_against_its_schema() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cli_input_v1");
    for entry in fs::read_dir(examples).unwrap() {
        let entry = entry.unwrap();
        assert!(entry.file_type().unwrap().is_file());
        let name = entry.file_name().into_string().unwrap();
        if name == ".gitkeep" {
            continue;
        }
        let stem = name
            .strip_suffix(".json")
            .expect("input examples must be JSON files");
        schema::check(
            &directory(),
            &format!("{stem}.schema.json"),
            &read_json(&entry.path()),
        )
        .unwrap();
    }
}
