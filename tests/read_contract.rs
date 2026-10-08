//! The published JSON v1 contract: schemas, golden envelopes and the
//! redaction scan. Each read added later gets one `ContractCase` here and
//! one entry in `golden::CASES`.

use std::{collections::BTreeSet, ffi::OsStr, fs, path::Path, str::FromStr};

use manyhands::{
    canonical::ItemId,
    repository::{
        Accessibility, ConfigurationState, IdentityAvailability, IdentitySource, IndexState,
        KeyOwnership, KeyPrivateSourceState, KeyPublicMetadataState, ProblemDto, RepositoryService,
        SharedKeyId, transport::SshAuthority,
    },
    results::{
        CheckpointEffect, CleanupEffect, DiscoveryEffect, Envelope, IntegrationEffect, Outcome,
        ProblemCode, PublicationEffect, ResultCode, Scope, WriteEffect,
    },
};
use serde_json::{Value, json};
use support::{
    credentials::{self, SECRET_SENTINELS},
    golden::{self, ContractCase},
    schema,
};

mod support;

/// No read test may initialize the Git transport; every test ends with this.
fn assert_git_transport_uninitialized() {
    assert!(!manyhands::runtime::git_transport_initialized());
}

fn write_schema(directory: &Path, file: &str, schema: Value) {
    fs::write(directory.join(file), schema.to_string()).unwrap();
}

/// A schema that uses every supported keyword, with a sibling it refers to.
fn sample_schemas() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    write_schema(
        directory.path(),
        "sample.schema.json",
        json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "sample.schema.json",
            "title": "Sample",
            "description": "Annotations are accepted and constrain nothing.",
            "type": "object",
            "additionalProperties": false,
            "required": ["id", "state", "note", "tags", "owner"],
            "properties": {
                "id": {"type": "string"},
                "state": {"type": "string", "enum": ["open", "closed"]},
                "note": {"type": ["string", "null"]},
                "tags": {"type": "array", "items": {"type": "string"}},
                "owner": {"$ref": "owner.schema.json", "description": "Who holds it."},
                "count": {"type": "integer"},
                "version": {"type": "integer", "enum": [1]},
            },
        }),
    );
    write_schema(
        directory.path(),
        "owner.schema.json",
        json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["name"],
            "properties": {"name": {"type": "string"}},
        }),
    );
    directory
}

fn sample_instance() -> Value {
    json!({
        "id": "a",
        "state": "open",
        "note": null,
        "tags": ["x", "y"],
        "owner": {"name": "n"},
    })
}

#[test]
fn schema_checker_accepts_a_conforming_instance() {
    let schemas = sample_schemas();
    let mut instance = sample_instance();

    schema::check(schemas.path(), "sample.schema.json", &instance).unwrap();
    instance["note"] = json!("text");
    instance["count"] = json!(3);
    instance["version"] = json!(1);
    schema::check(schemas.path(), "sample.schema.json", &instance).unwrap();
    // A number is the same value however it is written.
    instance["version"] = json!(1.0);
    schema::check(schemas.path(), "sample.schema.json", &instance).unwrap();
    assert_git_transport_uninitialized();
}

#[test]
fn schema_checker_rejects_each_kind_of_nonconforming_instance() {
    let schemas = sample_schemas();
    let reject = |change: &dyn Fn(&mut Value), expected: &str| {
        let mut instance = sample_instance();
        change(&mut instance);
        let problem = schema::check(schemas.path(), "sample.schema.json", &instance).unwrap_err();
        assert!(problem.contains(expected), "{problem}");
    };

    reject(
        &|instance| {
            instance.as_object_mut().unwrap().remove("note");
        },
        "missing required key \"note\"",
    );
    reject(
        &|instance| instance["extra"] = json!(1),
        "unexpected key \"extra\"",
    );
    reject(&|instance| instance["id"] = json!(7), "$.id: expected type");
    reject(
        &|instance| instance["count"] = json!(1.5),
        "$.count: expected type",
    );
    reject(
        &|instance| instance["id"] = Value::Null,
        "$.id: expected type",
    );
    reject(
        &|instance| instance["state"] = json!("reopened"),
        "$.state: \"reopened\" is not one of the allowed values",
    );
    reject(
        &|instance| instance["version"] = json!(2),
        "$.version: 2 is not one of the allowed values",
    );
    reject(
        &|instance| instance["tags"] = json!(["x", 2]),
        "$.tags[1]: expected type",
    );
    // Through the reference to the sibling file.
    reject(
        &|instance| instance["owner"] = json!({"name": "n", "email": "e"}),
        "$.owner: unexpected key \"email\"",
    );
    reject(
        &|instance| instance["owner"] = json!({}),
        "$.owner: missing required key \"name\"",
    );
    reject(&|instance| *instance = json!([]), "$: expected type");
    assert_git_transport_uninitialized();
}

#[test]
fn schema_checker_rejects_a_schema_outside_the_keyword_subset() {
    let cases = [
        // An unsupported keyword, even one a full validator would accept.
        (
            json!({"type": "string", "minLength": 1}),
            "unsupported keyword",
        ),
        (
            json!({"type": "object", "$comment": "x"}),
            "unsupported keyword",
        ),
        (
            json!({"type": "object", "$defs": {}}),
            "unsupported keyword",
        ),
        (
            json!({"oneOf": [{"type": "string"}]}),
            "unsupported keyword",
        ),
        // In a branch the instance never reaches.
        (
            json!({"type": ["object", "null"], "properties": {"a": {"pattern": "x"}}}),
            "/properties/a/pattern: unsupported keyword",
        ),
        (
            json!({"type": "array", "items": {"const": 1}}),
            "/items/const: unsupported keyword",
        ),
        // In a sibling the schema refers to.
        (
            json!({"type": ["object", "null"], "properties": {"a": {"$ref": "bad.schema.json"}}}),
            "bad.schema.json/format: unsupported keyword",
        ),
        // Supported keywords with values the subset does not allow.
        (json!({"type": "text"}), "unknown type"),
        (json!({"type": []}), "expected a type name or names"),
        (
            json!({"additionalProperties": {"type": "string"}}),
            "expected a boolean",
        ),
        (json!({"required": "id"}), "expected an array of names"),
        (json!({"enum": []}), "expected a non-empty array"),
        (json!({"title": 1}), "/title: expected a string"),
        (
            json!({"$ref": "bad.schema.json", "type": "object"}),
            "$ref must be the only keyword",
        ),
        (
            json!({"$ref": "../v2/bad.schema.json"}),
            "$ref must name a sibling file",
        ),
        (
            json!({"$ref": "bad.schema.json#/properties/a"}),
            "$ref must name a sibling file",
        ),
        (
            json!({"$ref": "https://example.test/bad.schema.json"}),
            "$ref must name a sibling file",
        ),
        (
            json!({"$ref": "absent.schema.json"}),
            "cannot read the schema",
        ),
        (json!(true), "a schema must be an object"),
        // A required name the schema does not describe.
        (
            json!({"type": "object", "required": ["a", "b"], "properties": {"a": {}}}),
            "/required: \"b\" is not a property",
        ),
        (
            json!({"type": "object", "required": ["a"]}),
            "/required: \"a\" is not a property",
        ),
        // A keyword under a type it can never apply to.
        (
            json!({"type": "string", "properties": {"a": {}}}),
            "/properties: the type cannot be an object",
        ),
        (
            json!({"type": ["string", "null"], "required": []}),
            "/required: the type cannot be an object",
        ),
        (
            json!({"type": "array", "additionalProperties": false}),
            "/additionalProperties: the type cannot be an object",
        ),
        (
            json!({"type": "object", "items": {"type": "string"}}),
            "/items: the type cannot be an array",
        ),
    ];
    for (schema, expected) in cases {
        let directory = tempfile::tempdir().unwrap();
        write_schema(
            directory.path(),
            "bad.schema.json",
            json!({"format": "uri"}),
        );
        write_schema(directory.path(), "case.schema.json", schema.clone());

        // The instance is never at fault here: the schema is refused first.
        let problem =
            schema::check(directory.path(), "case.schema.json", &Value::Null).unwrap_err();
        assert!(problem.contains(expected), "{schema}: {problem}");
        assert!(schema::validate(directory.path(), "case.schema.json").is_err());
    }
    assert_git_transport_uninitialized();
}

#[test]
fn schema_references_may_be_recursive() {
    let directory = tempfile::tempdir().unwrap();
    write_schema(
        directory.path(),
        "node.schema.json",
        json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["children"],
            "properties": {
                "children": {"type": "array", "items": {"$ref": "node.schema.json"}},
            },
        }),
    );

    let check = |instance| schema::check(directory.path(), "node.schema.json", &instance);
    check(json!({"children": [{"children": []}]})).unwrap();
    assert!(check(json!({"children": [{"children": [{}]}]})).is_err());
    assert_git_transport_uninitialized();
}

#[test]
fn golden_text_has_sorted_keys_two_space_indentation_and_a_final_newline() {
    let value = json!({
        "b": {"z": "/tmp/run-1/repository/docs", "a": [1, "/tmp/run-1/repository"]},
        "a": null,
        "/tmp/run-1/repository": "a key is never substituted",
    });

    assert_eq!(
        golden::render(
            &value,
            &[
                ("/tmp/run-1", "<temporary>"),
                ("/tmp/run-1/repository", "<repository>"),
            ],
        ),
        concat!(
            "{\n",
            "  \"/tmp/run-1/repository\": \"a key is never substituted\",\n",
            "  \"a\": null,\n",
            "  \"b\": {\n",
            "    \"a\": [\n",
            "      1,\n",
            "      \"<repository>\"\n",
            "    ],\n",
            "    \"z\": \"<repository>/docs\"\n",
            "  }\n",
            "}\n",
        )
    );
    assert_git_transport_uninitialized();
}

#[test]
fn golden_placeholders_replace_a_whole_value_or_a_path_under_it() {
    let placeholders = [
        ("/tmp/run-1", "<temporary>"),
        ("01ARZ3NDEKTSV4RRFFQ69G5FAV", "<id>"),
    ];
    let render = |text: &str| {
        let rendered = golden::render(&json!([text]), &placeholders);
        serde_json::from_str::<Value>(&rendered).unwrap()[0]
            .as_str()
            .unwrap()
            .to_owned()
    };

    assert_eq!(render("/tmp/run-1"), "<temporary>");
    assert_eq!(render("/tmp/run-1/docs/a.md"), "<temporary>/docs/a.md");
    assert_eq!(render("/tmp/run-1\\docs"), "<temporary>\\docs");
    assert_eq!(render("01ARZ3NDEKTSV4RRFFQ69G5FAV"), "<id>");
    // Not a path under the value, and not the value: left alone.
    for unrelated in [
        "/tmp/run-10",
        "/tmp/run-1.bak",
        "see /tmp/run-1",
        "ssh://host/tmp/run-1/docs",
        "branch/01ARZ3NDEKTSV4RRFFQ69G5FAV",
        "01ARZ3NDEKTSV4RRFFQ69G5FAVX",
    ] {
        assert_eq!(render(unrelated), unrelated);
    }
    assert_git_transport_uninitialized();
}

#[test]
#[should_panic(expected = "too short to be given a placeholder")]
fn golden_placeholders_refuse_a_short_value() {
    golden::render(&json!({}), &[("/tmp/ab", "<temporary>")]);
}

#[test]
fn sentinel_scan_reads_keys_and_string_values_not_json_text() {
    // Each of these is written differently in JSON text than in the string.
    let quoted = "pass\"word";
    let escaped = "C:\\secret\\key";
    let control = "line\nbreak";
    let value = json!({
        "a": [1, {"b": format!("before {quoted} after")}],
        "c": escaped,
        control: null,
    });
    let found = |sentinel: &'static str| golden::find_sentinel(&value, &[sentinel]);

    assert_eq!(found(quoted), Some((quoted, "$.a[1].b".to_owned())));
    assert_eq!(found(escaped), Some((escaped, "$.c".to_owned())));
    assert_eq!(
        found(control),
        Some((control, format!("$.{control} (key)")))
    );
    assert_eq!(found("absent"), None);
    // The compact text holds none of them, which is why it is not scanned.
    let compact = value.to_string();
    assert!(
        ![quoted, escaped, control]
            .iter()
            .any(|sentinel| compact.contains(sentinel))
    );
    assert_git_transport_uninitialized();
}

#[test]
fn golden_update_is_requested_only_by_the_exact_value() {
    let one = Some(OsStr::new("1"));

    assert!(golden::update_requested(one, None));
    assert!(!golden::update_requested(None, None));
    assert!(!golden::update_requested(Some(OsStr::new("0")), None));
    assert!(!golden::update_requested(Some(OsStr::new("true")), None));
    // Without a request, `CI` changes nothing.
    assert!(!golden::update_requested(None, Some(OsStr::new("true"))));
    assert_git_transport_uninitialized();
}

#[test]
#[should_panic(expected = "MANYHANDS_UPDATE_GOLDEN must not be set when CI is set")]
fn golden_update_is_refused_under_ci() {
    golden::update_requested(Some(OsStr::new("1")), Some(OsStr::new("true")));
}

fn published_schema_files() -> Vec<String> {
    let mut files: Vec<_> = fs::read_dir(schema::schema_directory())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    files.sort();
    files
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn every_published_schema_uses_only_the_checked_keywords() {
    let files = published_schema_files();

    assert!(files.contains(&golden::ENVELOPE_SCHEMA.to_owned()));
    for file in files {
        assert!(file.ends_with(".schema.json"), "{file}");
        schema::validate(&schema::schema_directory(), &file).unwrap();
        let schema = read_json(&schema::schema_directory().join(&file));
        assert_eq!(
            schema["$schema"], "https://json-schema.org/draft/2020-12/schema",
            "{file}"
        );
        assert!(schema["title"].is_string(), "{file}");
    }
    assert_git_transport_uninitialized();
}

/// The only objects the contract leaves open, by file and location: the
/// envelope's `data`, which each DTO schema describes, and a recovery
/// action's `arguments`, which each action defines.
const OPEN_OBJECTS: [(&str, &str); 2] = [
    ("envelope.schema.json", "/properties/data"),
    ("recovery_action.schema.json", "/properties/arguments"),
];

/// Every schema that can describe an object must close it: `properties`,
/// `additionalProperties: false`, and `required` naming every property, so
/// that no field is optional and none is unlisted. Returns what breaks that,
/// and records each open object it was told to allow.
fn closed_object_problems(
    file: &str,
    schema: &Value,
    location: &str,
    open: &[(&str, &str)],
    allowed: &mut BTreeSet<(String, String)>,
) -> Vec<String> {
    let mut problems = Vec::new();
    if schema.get("$ref").is_some() {
        return problems;
    }
    let properties = schema.get("properties").and_then(Value::as_object);
    let object_type = match schema.get("type") {
        Some(Value::Array(names)) => names.iter().any(|name| name == "object"),
        Some(name) => name == "object",
        None => false,
    };
    if open.contains(&(file, location)) {
        allowed.insert((file.to_owned(), location.to_owned()));
        if properties.is_some() {
            problems.push(format!("{file}{location}: an open object lists properties"));
        }
    } else if object_type || properties.is_some() {
        let names: Vec<&str> = properties
            .into_iter()
            .flat_map(|properties| properties.keys().map(String::as_str))
            .collect();
        let required: Vec<&str> = schema
            .get("required")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        if properties.is_none() {
            problems.push(format!("{file}{location}: an object without properties"));
        }
        if schema.get("additionalProperties") != Some(&Value::Bool(false)) {
            problems.push(format!(
                "{file}{location}: additionalProperties is not false"
            ));
        }
        if required.len() != names.len()
            || required.iter().collect::<BTreeSet<_>>() != names.iter().collect()
        {
            problems.push(format!(
                "{file}{location}: required is not exactly the property names"
            ));
        }
    }
    for (name, property) in properties.into_iter().flatten() {
        let location = format!("{location}/properties/{name}");
        problems.extend(closed_object_problems(
            file, property, &location, open, allowed,
        ));
    }
    if let Some(items) = schema.get("items") {
        let location = format!("{location}/items");
        problems.extend(closed_object_problems(
            file, items, &location, open, allowed,
        ));
    }
    problems
}

#[test]
fn every_published_object_is_closed_and_requires_all_its_properties() {
    let mut allowed = BTreeSet::new();
    for file in published_schema_files() {
        let schema = read_json(&schema::schema_directory().join(&file));
        let problems = closed_object_problems(&file, &schema, "", &OPEN_OBJECTS, &mut allowed);
        assert!(problems.is_empty(), "{problems:#?}");
    }
    // An entry that no longer names an open object is removed, not kept.
    assert_eq!(
        allowed,
        OPEN_OBJECTS
            .map(|(file, location)| (file.to_owned(), location.to_owned()))
            .into()
    );
    assert_git_transport_uninitialized();
}

#[test]
fn the_closed_object_lint_finds_each_way_an_object_is_left_open() {
    let lint = |schema: Value| {
        closed_object_problems(
            "x",
            &schema,
            "",
            &[("x", "/properties/open")],
            &mut BTreeSet::new(),
        )
    };
    let closed = json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["a", "open", "list"],
        "properties": {
            "a": {"type": ["string", "null"]},
            "open": {"type": "object"},
            "list": {"type": "array", "items": {"$ref": "other.schema.json"}},
        },
    });
    assert_eq!(lint(closed.clone()), Vec::<String>::new());

    let mut optional = closed.clone();
    optional["required"] = json!(["a", "open"]);
    let mut duplicated = closed.clone();
    duplicated["required"] = json!(["a", "a", "open"]);
    let mut unlisted = closed.clone();
    unlisted
        .as_object_mut()
        .unwrap()
        .remove("additionalProperties");
    let mut nested = closed.clone();
    nested["properties"]["a"] = json!({"type": ["object", "null"]});
    let mut in_items = closed.clone();
    in_items["properties"]["list"]["items"] =
        json!({"type": "object", "properties": {"b": {}}, "additionalProperties": false});
    let mut untyped = closed.clone();
    untyped["properties"]["a"] = json!({"properties": {}});
    for (schema, expected) in [
        (optional, "x: required is not exactly the property names"),
        (duplicated, "x: required is not exactly the property names"),
        (unlisted, "x: additionalProperties is not false"),
        (nested, "x/properties/a: an object without properties"),
        (
            in_items,
            "x/properties/list/items: required is not exactly the property names",
        ),
        (untyped, "x/properties/a: additionalProperties is not false"),
    ] {
        let problems = lint(schema);
        assert!(
            problems.iter().any(|problem| problem == expected),
            "{problems:?}"
        );
    }
    assert_git_transport_uninitialized();
}

#[test]
fn golden_fixtures_are_exactly_the_registered_cases() {
    let mut fixtures: Vec<_> = fs::read_dir(golden::fixture_directory())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    fixtures.sort();
    let mut cases: Vec<_> = golden::CASES
        .iter()
        .map(|case| format!("{case}.json"))
        .collect();
    cases.sort();

    assert_eq!(fixtures, cases);
    assert_git_transport_uninitialized();
}

#[test]
fn every_golden_fixture_is_canonical_text() {
    for case in golden::CASES {
        let path = golden::fixture_directory().join(format!("{case}.json"));
        let text = fs::read_to_string(&path).unwrap();
        let value: Value = serde_json::from_str(&text).unwrap();

        assert_eq!(golden::render(&value, &[]), text, "{path:?}");
        schema::check_published(golden::ENVELOPE_SCHEMA, &value).unwrap();
    }
    assert_git_transport_uninitialized();
}

fn schema_enum(file: &str, property: &str) -> Vec<String> {
    let schema = read_json(&schema::schema_directory().join(file));
    schema["properties"][property]["enum"]
        .as_array()
        .unwrap_or_else(|| panic!("{file}: {property} has no enum"))
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect()
}

/// Every `enum` in a published schema, by file and property, with the Rust
/// list it must equal.
fn contract_enumerations() -> Vec<(&'static str, &'static str, Vec<&'static str>)> {
    let envelope = golden::ENVELOPE_SCHEMA;
    let effects = "effects.schema.json";
    vec![
        (
            envelope,
            "outcome",
            Outcome::ALL.map(Outcome::as_str).to_vec(),
        ),
        (
            envelope,
            "code",
            ResultCode::ALL.map(ResultCode::as_str).to_vec(),
        ),
        (
            effects,
            "write",
            WriteEffect::ALL.map(WriteEffect::as_str).to_vec(),
        ),
        (
            effects,
            "checkpoint",
            CheckpointEffect::ALL.map(CheckpointEffect::as_str).to_vec(),
        ),
        (
            effects,
            "discovery",
            DiscoveryEffect::ALL.map(DiscoveryEffect::as_str).to_vec(),
        ),
        (
            effects,
            "publication",
            PublicationEffect::ALL
                .map(PublicationEffect::as_str)
                .to_vec(),
        ),
        (
            effects,
            "integration",
            IntegrationEffect::ALL
                .map(IntegrationEffect::as_str)
                .to_vec(),
        ),
        (
            effects,
            "cleanup",
            CleanupEffect::ALL.map(CleanupEffect::as_str).to_vec(),
        ),
        (
            "problem.schema.json",
            "code",
            ProblemCode::ALL.map(ProblemCode::as_str).to_vec(),
        ),
        (
            "repository_summary.schema.json",
            "accessibility",
            Accessibility::ALL.map(Accessibility::as_str).to_vec(),
        ),
        (
            "repository_configuration.schema.json",
            "state",
            ConfigurationState::ALL
                .map(ConfigurationState::as_str)
                .to_vec(),
        ),
        (
            "repository_index.schema.json",
            "state",
            IndexState::ALL.map(IndexState::as_str).to_vec(),
        ),
        (
            "repository_inspection.schema.json",
            "identity_state",
            IdentityAvailability::ALL
                .map(IdentityAvailability::as_str)
                .to_vec(),
        ),
        (
            "identity.schema.json",
            "source",
            IdentitySource::ALL.map(IdentitySource::as_str).to_vec(),
        ),
        (
            "key.schema.json",
            "ownership",
            KeyOwnership::ALL.map(KeyOwnership::as_str).to_vec(),
        ),
        (
            "key.schema.json",
            "private_source_state",
            KeyPrivateSourceState::ALL
                .map(KeyPrivateSourceState::as_str)
                .to_vec(),
        ),
        (
            "key.schema.json",
            "public_metadata_state",
            KeyPublicMetadataState::ALL
                .map(KeyPublicMetadataState::as_str)
                .to_vec(),
        ),
    ]
}

/// Counts the `enum` keywords in a schema, wherever they are.
fn count_enums(schema: &Value) -> usize {
    match schema {
        Value::Object(object) => object
            .iter()
            .map(|(key, value)| usize::from(key == "enum") + count_enums(value))
            .sum(),
        Value::Array(values) => values.iter().map(count_enums).sum(),
        _ => 0,
    }
}

#[test]
fn published_enumerations_list_exactly_the_registered_values() {
    let enumerations = contract_enumerations();
    for (file, property, expected) in &enumerations {
        assert_eq!(&schema_enum(file, property), expected, "{file}: {property}");
    }

    // No schema holds an enumeration this test does not tie to Rust. The
    // one exception is the schema version, an integer.
    assert_eq!(
        read_json(&schema::schema_directory().join(golden::ENVELOPE_SCHEMA))["properties"]["schema_version"]
            ["enum"],
        json!([manyhands::results::SCHEMA_VERSION])
    );
    let published: usize = published_schema_files()
        .iter()
        .map(|file| count_enums(&read_json(&schema::schema_directory().join(file))))
        .sum();
    assert_eq!(published, enumerations.len() + 1);
    assert_git_transport_uninitialized();
}

#[test]
fn every_problem_matches_the_problem_schema() {
    let mut guidance = BTreeSet::new();
    for code in ProblemCode::ALL {
        for path in [Some("docs/a.md".to_owned()), None] {
            let problem = serde_json::to_value(ProblemDto { code, path }).unwrap();
            schema::check_published("problem.schema.json", &problem).unwrap();
            assert_eq!(problem["guidance"], code.guidance());
        }
        guidance.insert(code.guidance());
    }
    assert_eq!(guidance.len(), ProblemCode::ALL.len());
    assert_git_transport_uninitialized();
}

#[test]
fn id_new_matches_its_schema_and_golden() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();

    let new_id = service.new_item_id();
    let id = new_id.id.clone();
    let envelope = Envelope::read_success("id new", Scope::default(), new_id);

    assert_eq!(ItemId::from_str(&id).unwrap().to_string(), id);
    assert_ne!(service.new_item_id().id, id);
    golden::assert_contract(
        &ContractCase {
            name: "id_new",
            data_schema: Some("new_id.schema.json"),
            placeholders: &[(&id, "<item-id>")],
            sentinels: &[],
        },
        &envelope,
    );
    assert_git_transport_uninitialized();
}

// `document list` is the command Task 5's read will answer; until then the
// envelope comes from the session itself, which is what fails.
#[test]
fn a_degraded_index_matches_the_failure_golden() {
    let data = tempfile::tempdir().unwrap();
    fs::write(data.path().join("manyhands.sqlite3"), b"not sqlite").unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();

    let error = service.read_session_for_testing(|_| Ok(())).unwrap_err();
    let directory = data.path().to_str().unwrap();

    assert_eq!(error.code(), ResultCode::IndexUnavailable);
    golden::assert_contract(
        &ContractCase {
            name: "failure_index_unavailable",
            data_schema: None,
            placeholders: &[],
            // The data directory is not the caller's business.
            sentinels: &[directory, "not sqlite", "manyhands.sqlite3"],
        },
        &error.to_envelope::<Value>("document list"),
    );
    assert_git_transport_uninitialized();
}

/// A credential planted in a remote URL; no envelope may carry it.
const CREDENTIAL: &str = "SENTINEL-9d2e";

fn repository_scope(root: &str) -> Scope {
    Scope {
        repository: Some(root.to_owned()),
        ..Scope::default()
    }
}

/// Two remotes: one with a credential in each location, and one that
/// publication can use and the configuration selects.
fn add_fixture_remotes(fixture: &support::TestRepository) {
    fixture
        .repository
        .remote(
            "origin",
            &format!("https://user:{CREDENTIAL}@example.invalid/team/repo.git"),
        )
        .unwrap();
    fixture
        .repository
        .remote_set_pushurl(
            "origin",
            Some(&format!(
                "https://example.invalid/team/repo.git?token={CREDENTIAL}"
            )),
        )
        .unwrap();
    fixture
        .repository
        .remote("publish", "ssh://git@example.invalid/team/repo.git")
        .unwrap();
    fs::write(
        fixture.root.join(".manyhands/config.toml"),
        "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"publish\"\n",
    )
    .unwrap();
}

#[test]
fn repo_list_matches_its_schema_and_golden() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);

    let list = enabled.service.list_repositories().unwrap();
    let root = list.items[0].root.clone();
    let enabled_at = list.items[0].enabled_at.clone().unwrap();
    let envelope = Envelope::read_success("repo list", Scope::default(), list);

    assert_eq!(Path::new(&root), fs::canonicalize(&fixture.root).unwrap());
    golden::assert_contract(
        &ContractCase {
            name: "repo_list",
            data_schema: Some("repository_list.schema.json"),
            placeholders: &[(&root, "<repository>"), (&enabled_at, "<timestamp>")],
            sentinels: &[enabled.data_directory.path().to_str().unwrap()],
        },
        &envelope,
    );
    assert_git_transport_uninitialized();
}

#[test]
fn repo_inspect_matches_its_schema_and_golden() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    add_fixture_remotes(&fixture);

    let inspection = enabled.service.inspect_repository(&fixture.root).unwrap();
    let root = inspection.root.clone();
    let envelope = Envelope::read_success("repo inspect", repository_scope(&root), inspection);

    golden::assert_contract(
        &ContractCase {
            name: "repo_inspect",
            data_schema: Some("repository_inspection.schema.json"),
            placeholders: &[(&root, "<repository>")],
            sentinels: &[CREDENTIAL],
        },
        &envelope,
    );
    assert_git_transport_uninitialized();
}

#[test]
fn repo_identity_matches_its_schema_and_golden() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();

    let identity = enabled.service.repository_identity(&repo).unwrap();
    let envelope = Envelope::read_success("repo identity", repo.scope(), identity);

    golden::assert_contract(
        &ContractCase {
            name: "repo_identity",
            data_schema: Some("identity.schema.json"),
            placeholders: &[(repo.root().to_str().unwrap(), "<repository>")],
            sentinels: &[],
        },
        &envelope,
    );
    assert_git_transport_uninitialized();
}

#[test]
fn remote_list_matches_its_schema_and_golden() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    add_fixture_remotes(&fixture);
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();

    let remotes = enabled.service.list_remotes_redacted(&repo).unwrap();
    let envelope = Envelope::read_success("remote list", repo.scope(), remotes);

    // The credential really is in the repository, so the scan can fail.
    assert!(
        fixture
            .repository
            .find_remote("origin")
            .unwrap()
            .url()
            .unwrap()
            .contains(CREDENTIAL)
    );
    assert!(
        !serde_json::to_string(&envelope)
            .unwrap()
            .contains(CREDENTIAL)
    );
    golden::assert_contract(
        &ContractCase {
            name: "remote_list",
            data_schema: Some("remote_list.schema.json"),
            placeholders: &[(repo.root().to_str().unwrap(), "<repository>")],
            sentinels: &[CREDENTIAL],
        },
        &envelope,
    );
    assert_git_transport_uninitialized();
}

#[test]
fn a_path_inside_a_repository_matches_the_failure_golden() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let subdirectory = fixture.root.join("nested");
    fs::create_dir_all(&subdirectory).unwrap();
    let root = fs::canonicalize(&fixture.root).unwrap();

    let error = enabled
        .service
        .resolve_repository(&subdirectory)
        .unwrap_err();

    assert_eq!(error.code(), ResultCode::NotRepositoryRoot);
    golden::assert_contract(
        &ContractCase {
            name: "failure_not_repository_root",
            data_schema: None,
            placeholders: &[(root.to_str().unwrap(), "<repository>")],
            sentinels: &["nested"],
        },
        &error.to_envelope::<Value>("remote list"),
    );
    assert_git_transport_uninitialized();
}

/// The placeholders for what differs between runs in a key envelope: the
/// generated IDs, the generated key's fingerprint, and where the files are.
fn key_placeholders(keys: &credentials::RegisteredKeys) -> Vec<(String, &'static str)> {
    let text = |path: &Path| path.to_str().unwrap().to_owned();
    vec![
        (keys.imported.id.to_string(), "<imported-key-id>"),
        (keys.generated.id.to_string(), "<generated-key-id>"),
        (keys.without_public.id.to_string(), "<third-key-id>"),
        (
            keys.generated.public_key_fingerprint.clone().unwrap(),
            "<generated-fingerprint>",
        ),
        (
            text(&keys.generated.private_key_path),
            "<generated-private-key>",
        ),
        (
            text(keys.generated.public_key_path.as_ref().unwrap()),
            "<generated-public-key>",
        ),
        (
            text(keys.imported.private_key_path.parent().unwrap()),
            "<key-files>",
        ),
    ]
}

fn assert_key_contract(
    name: &str,
    data_schema: Option<&str>,
    keys: &credentials::RegisteredKeys,
    extra: &[(&str, &str)],
    envelope: &impl serde::Serialize,
) {
    let placeholders = key_placeholders(keys);
    let mut placeholders: Vec<(&str, &str)> = placeholders
        .iter()
        .map(|(value, placeholder)| (value.as_str(), *placeholder))
        .collect();
    placeholders.extend_from_slice(extra);
    golden::assert_contract(
        &ContractCase {
            name,
            data_schema,
            placeholders: &placeholders,
            sentinels: &SECRET_SENTINELS,
        },
        envelope,
    );
}

/// A service with the three fixture registrations. The secrets the scan
/// looks for really are in the registered files.
fn service_with_keys() -> (
    tempfile::TempDir,
    RepositoryService,
    credentials::RegisteredKeys,
) {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let keys = credentials::register_keys(&service);
    let imported = fs::read_to_string(&keys.imported.private_key_path).unwrap();
    assert!(imported.contains(credentials::PRIVATE_KEY_SENTINEL));
    assert!(imported.contains(credentials::PRIVATE_FIXTURE_PASSPHRASE_SENTINEL));
    let generated = fs::read_to_string(&keys.generated.private_key_path).unwrap();
    assert!(generated.contains("PRIVATE KEY"));
    (data, service, keys)
}

#[test]
fn key_list_matches_its_schema_and_golden() {
    let (_data, service, keys) = service_with_keys();

    let list = service.list_keys().unwrap();
    let envelope = Envelope::read_success("key list", Scope::default(), list);

    assert_key_contract(
        "key_list",
        Some("key_list.schema.json"),
        &keys,
        &[],
        &envelope,
    );
    assert_git_transport_uninitialized();
}

#[test]
fn key_show_matches_its_schema_and_golden() {
    let (_data, service, keys) = service_with_keys();

    let key = service.show_key(keys.generated.id).unwrap();
    let envelope = Envelope::read_success("key show", Scope::default(), key);

    assert_key_contract("key_show", Some("key.schema.json"), &keys, &[], &envelope);
    assert_git_transport_uninitialized();
}

#[test]
fn key_public_matches_its_schema_and_golden() {
    let (_data, service, keys) = service_with_keys();

    let public_key = service.public_key_text(keys.imported.id).unwrap();
    let envelope = Envelope::read_success("key public", Scope::default(), public_key);

    assert_key_contract(
        "key_public",
        Some("public_key.schema.json"),
        &keys,
        &[],
        &envelope,
    );
    assert_git_transport_uninitialized();
}

#[test]
fn key_failures_match_their_goldens() {
    let (_data, service, keys) = service_with_keys();

    let not_found = service.show_key(SharedKeyId::new()).unwrap_err();
    let unavailable = service.public_key_text(keys.without_public.id).unwrap_err();

    assert_eq!(not_found.code(), ResultCode::KeyNotFound);
    assert_eq!(unavailable.code(), ResultCode::PublicKeyUnavailable);
    assert_key_contract(
        "failure_key_not_found",
        None,
        &keys,
        &[],
        &not_found.to_envelope::<Value>("key show"),
    );
    assert_key_contract(
        "failure_public_key_unavailable",
        None,
        &keys,
        &[],
        &unavailable.to_envelope::<Value>("key public"),
    );
    assert_git_transport_uninitialized();
}

/// Two pins of one host and one of another, stored out of order, under the
/// reapproval marker.
fn service_with_host_pins() -> (tempfile::TempDir, RepositoryService) {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    for (host, port, algorithm, sha256) in [
        (
            "git.example.invalid",
            2222,
            "ssh-rsa",
            credentials::OTHER_FINGERPRINT,
        ),
        (
            "git.example.invalid",
            22,
            "ssh-ed25519",
            credentials::PUBLIC_FIXTURE_FINGERPRINT,
        ),
        (
            "2001:db8::1",
            22,
            "ecdsa-sha2-nistp256",
            credentials::OTHER_FINGERPRINT,
        ),
    ] {
        credentials::pin_host(data.path(), host, port, algorithm, sha256);
    }
    credentials::require_host_reapproval(data.path());
    (data, service)
}

fn assert_host_contract(
    name: &str,
    data_schema: Option<&str>,
    data: &tempfile::TempDir,
    envelope: &impl serde::Serialize,
) {
    golden::assert_contract(
        &ContractCase {
            name,
            data_schema,
            placeholders: &[],
            // The data directory is not the caller's business.
            sentinels: &[data.path().to_str().unwrap()],
        },
        envelope,
    );
}

#[test]
fn host_list_matches_its_schema_and_golden() {
    let (data, service) = service_with_host_pins();

    let list = service.list_host_pins().unwrap();
    let envelope = Envelope::read_success("host list", Scope::default(), list);

    assert_host_contract(
        "host_list",
        Some("host_pin_list.schema.json"),
        &data,
        &envelope,
    );
    assert_git_transport_uninitialized();
}

#[test]
fn host_inspect_matches_its_schema_and_golden() {
    let (data, service) = service_with_host_pins();
    let authority = |host: &str, port| SshAuthority {
        host: host.to_owned(),
        port,
    };

    let pin = service
        .inspect_host(&authority("GIT.example.invalid", 2222))
        .unwrap();
    let envelope = Envelope::read_success("host inspect", Scope::default(), pin);
    let not_found = service
        .inspect_host(&authority("git.example.invalid", 2022))
        .unwrap_err();

    assert_host_contract(
        "host_inspect",
        Some("host_pin.schema.json"),
        &data,
        &envelope,
    );
    assert_eq!(not_found.code(), ResultCode::AuthorityNotFound);
    assert_host_contract(
        "failure_authority_not_found",
        None,
        &data,
        &not_found.to_envelope::<Value>("host inspect"),
    );
    assert_git_transport_uninitialized();
}
