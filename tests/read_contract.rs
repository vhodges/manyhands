//! The published JSON v1 contract: schemas, golden envelopes and the
//! redaction scan. Each read added later gets one `ContractCase` here and
//! one entry in `golden::CASES`.

use std::{collections::BTreeSet, ffi::OsStr, fs, path::Path, str::FromStr};

use manyhands::{
    canonical::ItemId,
    repository::{
        Accessibility, ChangeSource, ClosureState, ConfigurationState, CycleKind,
        DependencyDirection, DependencyState, IdentityAvailability, IdentitySource,
        IndexProblemDto, IndexState, IndexStatusState, ItemContextKind, ItemDto, ItemDtoKind,
        KeyOwnership, KeyPrivateSourceState, KeyPublicMetadataState, OperationAction,
        OperationFamily, OperationNextAction, OperationScope, PollingInterval, PollingOutcome,
        ProblemDto, ReadinessFilter, ReadinessReasonCode, ReadinessState, RepositoryService,
        ResolvedRepository, SharedKeyId, TicketFilter, UnplannableReasonCode,
        transport::SshAuthority,
    },
    results::{
        CheckpointEffect, CleanupEffect, DiscoveryEffect, Envelope, IntegrationEffect,
        OperationFailureCode, Outcome, ProblemCode, PublicationEffect, ResultCode, Scope,
        WriteEffect,
    },
};
use serde_json::{Value, json};
use support::{
    credentials::{self, SECRET_SENTINELS},
    golden::{self, ContractCase},
    items,
    operations::{self, OPERATION_A, OPERATION_B, OPERATION_C},
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
/// envelope's `data`, which each DTO schema describes; an item's and a
/// comment's `unknown_metadata`, whose keys are the file's own; and a
/// recovery action's `arguments`, which each action defines.
const OPEN_OBJECTS: [(&str, &str); 4] = [
    ("comment.schema.json", "/properties/unknown_metadata"),
    ("envelope.schema.json", "/properties/data"),
    ("item.schema.json", "/properties/unknown_metadata"),
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

/// The strings of the `enum` of `property`, which is a property name or a
/// dotted path through nested objects. A nullable enumeration also lists
/// `null`, which its `type` already allows and which is not a string.
fn schema_enum(file: &str, property: &str) -> Vec<String> {
    let schema = read_json(&schema::schema_directory().join(file));
    let mut node = &schema;
    for name in property.split('.') {
        node = &node["properties"][name];
    }
    node["enum"]
        .as_array()
        .unwrap_or_else(|| panic!("{file}: {property} has no enum"))
        .iter()
        .filter(|value| !value.is_null())
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
            "item.schema.json",
            "kind",
            ItemDtoKind::ALL.map(ItemDtoKind::as_str).to_vec(),
        ),
        (
            "item.schema.json",
            "closure.state",
            ClosureState::ALL.map(ClosureState::as_str).to_vec(),
        ),
        (
            "item.schema.json",
            "parent.state",
            DependencyState::ALL.map(DependencyState::as_str).to_vec(),
        ),
        (
            "item.schema.json",
            "readiness.state",
            ReadinessState::ALL.map(ReadinessState::as_str).to_vec(),
        ),
        (
            "item.schema.json",
            "change_source",
            ChangeSource::ALL.map(ChangeSource::as_str).to_vec(),
        ),
        (
            "item_context.schema.json",
            "kind",
            ItemContextKind::ALL.map(ItemContextKind::as_str).to_vec(),
        ),
        (
            "dependency.schema.json",
            "state",
            DependencyState::ALL.map(DependencyState::as_str).to_vec(),
        ),
        (
            "readiness_reason.schema.json",
            "code",
            ReadinessReasonCode::ALL
                .map(ReadinessReasonCode::as_str)
                .to_vec(),
        ),
        (
            "unplannable_reason.schema.json",
            "code",
            UnplannableReasonCode::ALL
                .map(UnplannableReasonCode::as_str)
                .to_vec(),
        ),
        (
            "cycle.schema.json",
            "kind",
            CycleKind::ALL.map(CycleKind::as_str).to_vec(),
        ),
        (
            "dependency_tree.schema.json",
            "direction",
            DependencyDirection::ALL
                .map(DependencyDirection::as_str)
                .to_vec(),
        ),
        (
            "dependency_tree_node.schema.json",
            "state",
            DependencyState::ALL.map(DependencyState::as_str).to_vec(),
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
        (
            "index_status.schema.json",
            "state",
            IndexStatusState::ALL.map(IndexStatusState::as_str).to_vec(),
        ),
        (
            "index_problem.schema.json",
            "code",
            ProblemCode::ALL.map(ProblemCode::as_str).to_vec(),
        ),
        (
            "polling_status.schema.json",
            "latest_outcome",
            PollingOutcome::ALL.map(PollingOutcome::as_str).to_vec(),
        ),
        (
            "operation.schema.json",
            "family",
            OperationFamily::ALL.map(OperationFamily::as_str).to_vec(),
        ),
        (
            "operation.schema.json",
            "scope",
            OperationScope::ALL.map(OperationScope::as_str).to_vec(),
        ),
        (
            "operation.schema.json",
            "action",
            OperationAction::ALL.map(OperationAction::as_str).to_vec(),
        ),
        (
            "operation.schema.json",
            "next_action",
            OperationNextAction::ALL
                .map(OperationNextAction::as_str)
                .to_vec(),
        ),
        (
            "operation.schema.json",
            "failure_code",
            OperationFailureCode::ALL
                .map(OperationFailureCode::as_str)
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
        for (path, target_id) in [
            (Some("docs/a.md".to_owned()), None),
            (None, Some(items::TICKET_A.to_owned())),
        ] {
            let problem = serde_json::to_value(ProblemDto {
                code,
                path,
                target_id,
            })
            .unwrap();
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

/// The values of an item envelope that differ from run to run: where the
/// repository is, when it was refreshed, and the commit its root is at.
fn item_placeholders(root: &str, items: &[&ItemDto]) -> Vec<(String, &'static str)> {
    let mut placeholders = vec![(root.to_owned(), "<repository>")];
    for item in items {
        if let Some(refreshed_at) = &item.index.refreshed_at {
            placeholders.push((refreshed_at.clone(), "<refreshed-at>"));
        }
        if let Some(head_oid) = &item.context.head_oid {
            placeholders.push((head_oid.clone(), "<head-oid>"));
        }
    }
    placeholders.sort();
    placeholders.dedup();
    placeholders
}

fn assert_item_contract(
    name: &str,
    data_schema: Option<&str>,
    root: &str,
    items: &[&ItemDto],
    sentinels: &[&str],
    envelope: &impl serde::Serialize,
) {
    let placeholders = item_placeholders(root, items);
    let placeholders: Vec<(&str, &str)> = placeholders
        .iter()
        .map(|(value, placeholder)| (value.as_str(), *placeholder))
        .collect();
    golden::assert_contract(
        &ContractCase {
            name,
            data_schema,
            placeholders: &placeholders,
            sentinels,
        },
        envelope,
    );
}

fn item_scope(repo: &ResolvedRepository, id: &str) -> Scope {
    Scope {
        item_id: Some(id.to_owned()),
        ..repo.scope()
    }
}

#[test]
fn document_list_matches_its_schema_and_golden() {
    let (fixture, enabled) = items::contract_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();

    let list = enabled.service.list_documents(&repo).unwrap();
    let envelope = Envelope::read_success("document list", repo.scope(), list.clone());

    assert_eq!(list.items.len(), 2);
    assert_item_contract(
        "document_list",
        Some("item_list.schema.json"),
        repo.root().to_str().unwrap(),
        &list.items.iter().collect::<Vec<_>>(),
        &[enabled.data_directory.path().to_str().unwrap()],
        &envelope,
    );
    assert_git_transport_uninitialized();
}

#[test]
fn document_show_matches_its_schema_and_golden() {
    let (fixture, enabled) = items::contract_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();

    let item = enabled
        .service
        .show_item(&repo, &items::item_id(items::DOCUMENT_A))
        .unwrap();
    let envelope = Envelope::read_success(
        "document show",
        item_scope(&repo, items::DOCUMENT_A),
        item.clone(),
    );

    assert_item_contract(
        "document_show",
        Some("item.schema.json"),
        repo.root().to_str().unwrap(),
        &[&item],
        &[enabled.data_directory.path().to_str().unwrap()],
        &envelope,
    );
    assert_git_transport_uninitialized();
}

#[test]
fn ticket_list_matches_its_schema_and_golden() {
    let (fixture, enabled) = items::contract_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();

    let list = enabled
        .service
        .list_tickets(&repo, &TicketFilter::default())
        .unwrap();
    let envelope = Envelope::read_success("ticket list", repo.scope(), list.clone());

    assert_eq!(list.items.len(), 2);
    assert_item_contract(
        "ticket_list",
        Some("item_list.schema.json"),
        repo.root().to_str().unwrap(),
        &list.items.iter().collect::<Vec<_>>(),
        &[enabled.data_directory.path().to_str().unwrap()],
        &envelope,
    );
    assert_git_transport_uninitialized();
}

#[test]
fn ticket_show_matches_its_schema_and_golden() {
    let (fixture, enabled) = items::contract_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();

    let item = enabled
        .service
        .show_item(&repo, &items::item_id(items::TICKET_B))
        .unwrap();
    let envelope = Envelope::read_success(
        "ticket show",
        item_scope(&repo, items::TICKET_B),
        item.clone(),
    );

    assert_item_contract(
        "ticket_show",
        Some("item.schema.json"),
        repo.root().to_str().unwrap(),
        &[&item],
        &[enabled.data_directory.path().to_str().unwrap()],
        &envelope,
    );
    assert_git_transport_uninitialized();
}

#[test]
fn a_ticket_with_relationships_matches_its_schema_and_golden() {
    let (fixture, enabled) = items::contract_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();

    let item = enabled
        .service
        .show_item(&repo, &items::item_id(items::TICKET_A))
        .unwrap();
    let envelope = Envelope::read_success(
        "ticket show",
        item_scope(&repo, items::TICKET_A),
        item.clone(),
    );

    assert_eq!(item.slug.as_deref(), Some("mh-vh-k9x2b"));
    assert_eq!(item.parent.as_ref().unwrap().id, items::TICKET_B);
    assert_eq!(item.deps.len(), 2);
    assert_item_contract(
        "ticket_show_relationships",
        Some("item.schema.json"),
        repo.root().to_str().unwrap(),
        &[&item],
        &[enabled.data_directory.path().to_str().unwrap()],
        &envelope,
    );
    assert_git_transport_uninitialized();
}

/// Front matter text of relationship values that are ignored. None of it
/// may be published with the problems that report them.
const IGNORED_SLUG: &str = "Not-A-Slug-7d1e";
const IGNORED_ID: &str = "not-an-id-7d1e";

#[test]
fn ticket_relationship_problems_match_their_schema_and_golden() {
    let (fixture, enabled) = items::contract_repository();
    let root = &fixture.root;
    let path = items::ticket_path(items::TICKET_C);
    // A parent and a dependency that are documents, the ticket itself, a
    // dependency twice, and values that are not a short code or an ID.
    items::write(
        root,
        &path,
        &items::ticket_source_with(
            items::TICKET_C,
            "Related badly",
            "chore",
            "open",
            &format!(
                "slug: {IGNORED_SLUG}\nparent: {}\ndeps:\n  - {}\n  - {}\n  - {}\n  - {}\n  - {IGNORED_ID}\n",
                items::DOCUMENT_A,
                items::TICKET_C,
                items::TICKET_A,
                items::TICKET_A,
                items::DOCUMENT_B,
            ),
        ),
    );
    items::commit(&fixture, &[&path], items::COMMITTED_AT + 300);
    items::refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();

    let list = enabled
        .service
        .list_tickets(
            &repo,
            &TicketFilter {
                ticket_type: Some("chore".to_owned()),
                ..Default::default()
            },
        )
        .unwrap();
    let envelope = Envelope::read_success("ticket list", repo.scope(), list.clone());

    // The ticket is listed as a ticket, with its one valid dependency.
    assert_eq!(list.items.len(), 1);
    assert_eq!(list.items[0].id.as_deref(), Some(items::TICKET_C));
    assert_eq!(list.items[0].deps.len(), 1);
    assert_eq!(
        list.items[0]
            .problems
            .iter()
            .map(|problem| (problem.code, problem.target_id.as_deref()))
            .collect::<Vec<_>>(),
        [
            (ProblemCode::InvalidSlug, None),
            (
                ProblemCode::RelationshipSelfReference,
                Some(items::TICKET_C)
            ),
            (ProblemCode::DuplicateDependency, Some(items::TICKET_A)),
            (ProblemCode::RelationshipInvalidId, None),
            (ProblemCode::RelationshipNotATicket, Some(items::DOCUMENT_A)),
            (ProblemCode::RelationshipNotATicket, Some(items::DOCUMENT_B)),
        ]
    );
    assert_item_contract(
        "ticket_list_relationship_problems",
        Some("item_list.schema.json"),
        repo.root().to_str().unwrap(),
        &list.items.iter().collect::<Vec<_>>(),
        &[
            IGNORED_SLUG,
            IGNORED_ID,
            enabled.data_directory.path().to_str().unwrap(),
        ],
        &envelope,
    );
    assert_git_transport_uninitialized();
}

/// The relationship fixture, resolved, with every ticket of it as the list
/// has it: the placeholders of any read of it come from those.
fn relationship_contract() -> (
    support::TestRepository,
    support::EnabledRepository,
    ResolvedRepository,
    Vec<ItemDto>,
) {
    let (fixture, enabled) = items::relationship_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let tickets = enabled
        .service
        .list_tickets(&repo, &TicketFilter::default())
        .unwrap()
        .items;
    assert_eq!(tickets.len(), 7);
    (fixture, enabled, repo, tickets)
}

fn assert_relationship_contract(
    name: &str,
    data_schema: &str,
    enabled: &support::EnabledRepository,
    repo: &ResolvedRepository,
    tickets: &[ItemDto],
    envelope: &impl serde::Serialize,
) {
    assert_item_contract(
        name,
        Some(data_schema),
        repo.root().to_str().unwrap(),
        &tickets.iter().collect::<Vec<_>>(),
        &[enabled.data_directory.path().to_str().unwrap()],
        envelope,
    );
    assert_git_transport_uninitialized();
}

fn listed_ids(list: &manyhands::repository::ItemListDto) -> Vec<&str> {
    list.items
        .iter()
        .map(|item| item.id.as_deref().unwrap())
        .collect()
}

#[test]
fn ticket_ready_matches_its_schema_and_golden() {
    let (_fixture, enabled, repo, tickets) = relationship_contract();

    let list = enabled
        .service
        .ticket_readiness(
            &repo,
            &TicketFilter {
                readiness: Some(ReadinessFilter::Ready),
                ..Default::default()
            },
        )
        .unwrap();

    assert_eq!(listed_ids(&list), [items::RELATED_B]);
    let envelope = Envelope::read_success("ticket ready", repo.scope(), list);
    assert_relationship_contract(
        "ticket_ready",
        "item_list.schema.json",
        &enabled,
        &repo,
        &tickets,
        &envelope,
    );
}

#[test]
fn ticket_blocked_matches_its_schema_and_golden() {
    let (_fixture, enabled, repo, tickets) = relationship_contract();

    let list = enabled
        .service
        .ticket_readiness(
            &repo,
            &TicketFilter {
                readiness: Some(ReadinessFilter::Blocked),
                ..Default::default()
            },
        )
        .unwrap();

    // Latest change first.
    assert_eq!(
        listed_ids(&list),
        [
            items::RELATED_G,
            items::RELATED_F,
            items::RELATED_E,
            items::RELATED_D,
            items::RELATED_C,
        ]
    );
    let envelope = Envelope::read_success("ticket blocked", repo.scope(), list);
    assert_relationship_contract(
        "ticket_blocked",
        "item_list.schema.json",
        &enabled,
        &repo,
        &tickets,
        &envelope,
    );
}

#[test]
fn ticket_deps_matches_its_schema_and_golden() {
    let (_fixture, enabled, repo, tickets) = relationship_contract();

    let tree = enabled
        .service
        .ticket_dependencies(
            &repo,
            &items::item_id(items::RELATED_E),
            DependencyDirection::Both,
            Some(2),
        )
        .unwrap();

    // E and F wait for each other, and G waits for E: both trees have
    // lines, and each comes back to E on a repeated line.
    assert_eq!(tree.dependencies.len(), 2);
    assert_eq!(tree.dependents.len(), 3);
    for lines in [&tree.dependencies, &tree.dependents] {
        assert_eq!(lines.iter().filter(|line| line.repeated).count(), 1);
    }
    let envelope = Envelope::read_success("ticket deps", item_scope(&repo, items::RELATED_E), tree);
    assert_relationship_contract(
        "ticket_deps",
        "dependency_tree.schema.json",
        &enabled,
        &repo,
        &tickets,
        &envelope,
    );
}

#[test]
fn ticket_children_matches_its_schema_and_golden() {
    let (_fixture, enabled, repo, tickets) = relationship_contract();

    let list = enabled
        .service
        .ticket_children(&repo, &items::item_id(items::RELATED_B))
        .unwrap();

    assert_eq!(listed_ids(&list), [items::RELATED_G, items::RELATED_C]);
    let envelope =
        Envelope::read_success("ticket children", item_scope(&repo, items::RELATED_B), list);
    assert_relationship_contract(
        "ticket_children",
        "item_list.schema.json",
        &enabled,
        &repo,
        &tickets,
        &envelope,
    );
}

#[test]
fn ticket_cycles_matches_its_schema_and_golden() {
    let (_fixture, enabled, repo, tickets) = relationship_contract();

    let cycles = enabled.service.ticket_cycles(&repo).unwrap();

    assert_eq!(
        cycles
            .items
            .iter()
            .map(|cycle| cycle.kind)
            .collect::<Vec<_>>(),
        [CycleKind::Deps, CycleKind::Parent]
    );
    let envelope = Envelope::read_success("ticket cycles", repo.scope(), cycles);
    assert_relationship_contract(
        "ticket_cycles",
        "cycle_list.schema.json",
        &enabled,
        &repo,
        &tickets,
        &envelope,
    );
}

#[test]
fn ticket_plan_matches_its_schema_and_golden() {
    let (_fixture, enabled, repo, tickets) = relationship_contract();

    let plan = enabled
        .service
        .ticket_plan(&repo, &TicketFilter::default())
        .unwrap();

    assert_eq!(plan.batches.len(), 2);
    assert_eq!(plan.unplannable.len(), 4);
    let envelope = Envelope::read_success("ticket plan", repo.scope(), plan);
    assert_relationship_contract(
        "ticket_plan",
        "plan.schema.json",
        &enabled,
        &repo,
        &tickets,
        &envelope,
    );
}

#[test]
fn ticket_critical_path_matches_its_schema_and_golden() {
    let (_fixture, enabled, repo, tickets) = relationship_contract();

    let path = enabled.service.ticket_critical_path(&repo).unwrap();

    assert_eq!(listed_ids(&path), [items::RELATED_B, items::RELATED_C]);
    let envelope = Envelope::read_success("ticket critical-path", repo.scope(), path);
    assert_relationship_contract(
        "ticket_critical_path",
        "item_list.schema.json",
        &enabled,
        &repo,
        &tickets,
        &envelope,
    );
}

#[test]
fn ticket_find_matches_its_schema_and_golden() {
    let (_fixture, enabled, repo, tickets) = relationship_contract();

    let found = enabled
        .service
        .find_tickets_by_slug(&repo, &items::SHARED_SLUG.to_uppercase())
        .unwrap();

    assert_eq!(listed_ids(&found), [items::RELATED_C, items::RELATED_B]);
    let envelope = Envelope::read_success("ticket find", repo.scope(), found);
    assert_relationship_contract(
        "ticket_find",
        "item_list.schema.json",
        &enabled,
        &repo,
        &tickets,
        &envelope,
    );
}

/// Planted in front matter that fails to parse, where the parser's message
/// repeats it and the index stores that message.
const PARSER_SENTINEL: &str = "SENTINEL-c40a";

#[test]
fn a_nonconforming_ticket_list_entry_matches_its_schema_and_golden() {
    let (fixture, enabled) = items::contract_repository();
    let root = &fixture.root;
    items::write(
        root,
        &items::ticket_path(items::TICKET_C),
        &format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: ticket\n\
             {PARSER_SENTINEL}: 1\n{PARSER_SENTINEL}: 2\n---\n"
        ),
    );
    items::refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    // The sentinel really is in the index, so the scan can fail.
    let stored: String = items::index(enabled.data_directory.path())
        .query_row(
            "SELECT guidance FROM problems WHERE code = 'malformed-front-matter'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(stored.contains(PARSER_SENTINEL), "{stored}");

    // The filter matches no ticket; the entry is there all the same.
    let list = enabled
        .service
        .list_tickets(
            &repo,
            &TicketFilter {
                status: Some("no such status".to_owned()),
                ..Default::default()
            },
        )
        .unwrap();
    let envelope = Envelope::read_success("ticket list", repo.scope(), list.clone());

    assert_eq!(list.items.len(), 1);
    assert_eq!(list.items[0].id, None);
    assert_item_contract(
        "ticket_list_nonconforming",
        Some("item_list.schema.json"),
        repo.root().to_str().unwrap(),
        &list.items.iter().collect::<Vec<_>>(),
        &[
            PARSER_SENTINEL,
            enabled.data_directory.path().to_str().unwrap(),
        ],
        &envelope,
    );
    assert_git_transport_uninitialized();
}

#[test]
fn a_nonconforming_resource_read_by_path_matches_its_schema_and_golden() {
    let (fixture, enabled) = items::contract_repository();
    let root = &fixture.root;
    let path = "docs/broken.md";
    let source = "no front matter\n";
    items::write(root, path, source);
    items::refresh_completely(&enabled.service, root);
    // What the index stores beside the problem's code is not what is read.
    let stored = items::index(enabled.data_directory.path())
        .execute("UPDATE problems SET guidance = ?1", [GUIDANCE_SENTINEL])
        .unwrap();
    assert_eq!(stored, 1);
    let repo = enabled.service.resolve_repository(root).unwrap();

    let item = enabled
        .service
        .show_path(&repo, None, Path::new(path))
        .unwrap();
    let envelope = Envelope::read_success("document show", repo.scope(), item.clone());

    assert_eq!(item.id, None);
    assert_eq!(item.path, path);
    assert_eq!(item.source.as_deref(), Some(source));
    assert_eq!(
        item.problems
            .iter()
            .map(|problem| problem.code)
            .collect::<Vec<_>>(),
        [ProblemCode::MissingFrontMatter]
    );
    assert_item_contract(
        "document_show_nonconforming",
        Some("item.schema.json"),
        repo.root().to_str().unwrap(),
        &[&item],
        &[
            GUIDANCE_SENTINEL,
            enabled.data_directory.path().to_str().unwrap(),
        ],
        &envelope,
    );
    assert_git_transport_uninitialized();
}

#[test]
fn a_degraded_index_matches_the_failure_golden() {
    let (fixture, enabled) = items::contract_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let (data, service) = items::degraded_service(enabled);

    let error = service.list_documents(&repo).unwrap_err();

    assert_eq!(error.code(), ResultCode::IndexUnavailable);
    golden::assert_contract(
        &ContractCase {
            name: "failure_index_unavailable",
            data_schema: None,
            placeholders: &[(repo.root().to_str().unwrap(), "<repository>")],
            // The data directory is not the caller's business.
            sentinels: &[
                data.path().to_str().unwrap(),
                "not sqlite",
                "manyhands.sqlite3",
            ],
        },
        &error.to_envelope::<Value>("document list"),
    );
    assert_git_transport_uninitialized();
}

#[test]
fn an_item_whose_file_is_gone_matches_the_failure_golden() {
    let (fixture, enabled) = items::contract_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    fs::remove_file(fixture.root.join("docs/guide.md")).unwrap();

    let error = enabled
        .service
        .show_item(&repo, &items::item_id(items::DOCUMENT_A))
        .unwrap_err();

    assert_eq!(error.code(), ResultCode::ItemNotFound);
    golden::assert_contract(
        &ContractCase {
            name: "failure_item_not_found",
            data_schema: None,
            placeholders: &[(repo.root().to_str().unwrap(), "<repository>")],
            sentinels: &[enabled.data_directory.path().to_str().unwrap()],
        },
        &error.to_envelope::<Value>("document show"),
    );
    assert_git_transport_uninitialized();
}

#[test]
fn comment_list_matches_its_schema_and_golden() {
    let (fixture, enabled) = items::contract_repository();
    let root = &fixture.root;
    items::write_comment(
        root,
        items::TICKET_A,
        items::COMMENT_A,
        None,
        "2026-09-30T12:35:00Z",
        "created_by: Ada Lovelace <ada@example.invalid>\nreviewed: true\n",
    );
    items::write_comment(
        root,
        items::TICKET_A,
        items::COMMENT_B,
        Some(items::COMMENT_A),
        "2026-09-30T12:36:00Z",
        "",
    );
    items::write_comment(
        root,
        items::TICKET_A,
        items::COMMENT_C,
        None,
        "2026-09-30T12:37:00Z",
        "",
    );
    // Not a comment: the parser's message repeats the sentinel, and the
    // index stores that message.
    items::write(
        root,
        &items::comment_path(items::TICKET_A, items::COMMENT_D),
        &format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: comment\n\
             {PARSER_SENTINEL}: 1\n{PARSER_SENTINEL}: 2\n---\n{PARSER_SENTINEL}\n"
        ),
    );
    items::refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();

    let list = enabled
        .service
        .list_comments(&repo, &items::item_id(items::TICKET_A))
        .unwrap();
    let envelope = Envelope::read_success(
        "comment list",
        item_scope(&repo, items::TICKET_A),
        list.clone(),
    );

    assert_eq!(list.items.len(), 3);
    assert_eq!(list.items[0].replies.len(), 1);
    assert_eq!(list.items[2].id, None);
    let mut placeholders = vec![(repo.root().to_str().unwrap(), "<repository>")];
    if let Some(refreshed_at) = &list.index.refreshed_at {
        placeholders.push((refreshed_at, "<refreshed-at>"));
    }
    if let Some(head_oid) = &list.context.head_oid {
        placeholders.push((head_oid, "<head-oid>"));
    }
    golden::assert_contract(
        &ContractCase {
            name: "comment_list",
            data_schema: Some("comment_list.schema.json"),
            placeholders: &placeholders,
            sentinels: &[
                PARSER_SENTINEL,
                "created_by",
                enabled.data_directory.path().to_str().unwrap(),
            ],
        },
        &envelope,
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
    // Enabling refreshes the index, within a second or so of registering it:
    // the two times may or may not be the same text, so they share a name.
    let refreshed_at = list.items[0].index.refreshed_at.clone().unwrap();
    assert_eq!(list.items[0].index.state, IndexState::Current);
    let envelope = Envelope::read_success("repo list", Scope::default(), list);

    assert_eq!(Path::new(&root), fs::canonicalize(&fixture.root).unwrap());
    golden::assert_contract(
        &ContractCase {
            name: "repo_list",
            data_schema: Some("repository_list.schema.json"),
            placeholders: &[
                (&root, "<repository>"),
                (&enabled_at, "<timestamp>"),
                (&refreshed_at, "<timestamp>"),
            ],
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
        credentials::pin_host(&service, host, port, algorithm, sha256);
    }
    credentials::require_host_reapproval(&service);
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

/// Stored beside a problem's code, where no read may take it from.
const GUIDANCE_SENTINEL: &str = "SENTINEL-2f6b";

#[test]
fn every_index_problem_matches_its_schema() {
    for code in ProblemCode::ALL {
        for (path, worktree) in [
            (Some("docs/a.md".to_owned()), Some("/repository".to_owned())),
            (None, None),
        ] {
            let problem = serde_json::to_value(IndexProblemDto {
                code,
                path,
                worktree,
            })
            .unwrap();
            schema::check_published("index_problem.schema.json", &problem).unwrap();
            assert_eq!(problem["guidance"], code.guidance());
        }
    }
    assert_git_transport_uninitialized();
}

#[test]
fn index_status_matches_its_schema_and_golden() {
    let (fixture, enabled) = items::contract_repository();
    let data = enabled.data_directory.path();
    items::write(
        &fixture.root,
        "docs/marker.md",
        "---\nmanyhands_managed: true\n---\n",
    );
    items::refresh_completely(&enabled.service, &fixture.root);
    let index = items::index(data);
    index
        .execute(
            "INSERT INTO problems (repository_id, code, guidance, observed_at)
             SELECT id, 'retry-required', ?1, 0 FROM repositories",
            [GUIDANCE_SENTINEL],
        )
        .unwrap();
    index
        .execute("UPDATE problems SET guidance = ?1", [GUIDANCE_SENTINEL])
        .unwrap();
    drop(index);
    operations::insert_local(data, Some(OPERATION_A), "refresh", "failed", None);
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();

    let status = enabled.service.index_status(&repo).unwrap();
    let refreshed_at = status.refreshed_at.clone().unwrap();
    let envelope = Envelope::read_success("index status", repo.scope(), status.clone());

    assert_eq!(status.state, IndexStatusState::Current);
    assert_eq!(status.problems.len(), 2);
    assert_eq!(status.pending_operations.len(), 1);
    golden::assert_contract(
        &ContractCase {
            name: "index_status",
            data_schema: Some("index_status.schema.json"),
            placeholders: &[
                (repo.root().to_str().unwrap(), "<repository>"),
                (&refreshed_at, "<refreshed-at>"),
            ],
            sentinels: &[GUIDANCE_SENTINEL, data.to_str().unwrap()],
        },
        &envelope,
    );
    assert_git_transport_uninitialized();
}

#[test]
fn an_unavailable_index_status_matches_its_schema_and_golden() {
    let (fixture, enabled) = items::contract_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let (data, service) = items::degraded_service(enabled);

    let status = service.index_status(&repo).unwrap();
    let envelope = Envelope::read_success("index status", repo.scope(), status);

    golden::assert_contract(
        &ContractCase {
            name: "index_status_unavailable",
            data_schema: Some("index_status.schema.json"),
            placeholders: &[(repo.root().to_str().unwrap(), "<repository>")],
            sentinels: &[
                data.path().to_str().unwrap(),
                "not sqlite",
                "manyhands.sqlite3",
            ],
        },
        &envelope,
    );
    assert_git_transport_uninitialized();
}

#[test]
fn poll_status_matches_its_schema_and_golden() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let data = enabled.data_directory.path();
    enabled
        .service
        .set_remote_polling(
            &fixture.root,
            true,
            false,
            PollingInterval::from_seconds(600).unwrap(),
        )
        .unwrap();
    operations::configure_remote(data);
    items::index(data)
        .execute_batch(
            "UPDATE remote_polling_state
                SET latest_outcome = 'host_approval_required',
                    automatic_backoff_seconds = 120",
        )
        .unwrap();
    operations::insert_current_observation(data);
    operations::insert_remote_poll(
        data,
        OPERATION_B,
        "advertising",
        Some("before_transport"),
        None,
    );
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();

    let status = enabled.service.polling_status(&repo).unwrap();
    let envelope = Envelope::read_success("poll status", repo.scope(), status.clone());

    assert_eq!(status.next_eligible_at, None);
    assert_eq!(
        status.latest_outcome,
        Some(PollingOutcome::HostApprovalRequired)
    );
    golden::assert_contract(
        &ContractCase {
            name: "poll_status",
            data_schema: Some("polling_status.schema.json"),
            placeholders: &[(repo.root().to_str().unwrap(), "<repository>")],
            sentinels: &[data.to_str().unwrap()],
        },
        &envelope,
    );
    assert_git_transport_uninitialized();
}

/// A repository with an operation in each store: a key generation kept for
/// inspection, the poll that holds the reservation, a document save part of
/// the way through, an interrupted synchronization of a ticket, and a
/// refresh from before operations had IDs.
fn repository_with_operations() -> (
    support::TestRepository,
    support::EnabledRepository,
    ResolvedRepository,
) {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let data = enabled.data_directory.path();
    operations::configure_remote(data);
    operations::insert_local(data, None, "refresh", "failed", None);
    operations::insert_local(
        data,
        Some(OPERATION_C),
        "save_document",
        "authoring_destination_observed",
        Some("authoring_destination_observed"),
    );
    items::index(data)
        .execute(
            "UPDATE operation_records SET item_id = ?1, context_path = root_path
              WHERE operation_ulid = ?2",
            [items::DOCUMENT_A, OPERATION_C],
        )
        .unwrap();
    operations::insert_remote_poll(data, OPERATION_B, "reserved", None, None);
    operations::insert_remote_synchronization(
        data,
        "01ARZ3NDEKTSV4RRFFQ69G5FA4",
        Some(("ticket", items::TICKET_A)),
        "interrupted",
        Some("before_fetch"),
        None,
    );
    operations::insert_key_material(
        data,
        OPERATION_A,
        "generate",
        "retained-for-inspection",
        Some("source-missing"),
    );
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    (fixture, enabled, repo)
}

#[test]
fn operation_list_matches_its_schema_and_golden() {
    let (_fixture, enabled, repo) = repository_with_operations();

    let list = enabled.service.list_operations(&repo).unwrap();
    let envelope = Envelope::read_success("operation list", repo.scope(), list.clone());

    let families: Vec<_> = list.items.iter().map(|item| item.family).collect();
    assert_eq!(
        families,
        [
            OperationFamily::KeyMaterial,
            OperationFamily::Remote,
            OperationFamily::Local,
            OperationFamily::Remote,
            OperationFamily::Local,
        ]
    );
    // The key is a new one each run.
    let key_id = list.items[0].key_id.as_deref().unwrap();
    assert!(SharedKeyId::parse(key_id).is_ok());
    golden::assert_contract(
        &ContractCase {
            name: "operation_list",
            data_schema: Some("operation_list.schema.json"),
            placeholders: &[
                (repo.root().to_str().unwrap(), "<repository>"),
                (key_id, "<key-id>"),
            ],
            sentinels: &[
                operations::KEY_MATERIAL_SENTINEL,
                enabled.data_directory.path().to_str().unwrap(),
            ],
        },
        &envelope,
    );
    assert_git_transport_uninitialized();
}

#[test]
fn operation_show_matches_its_schema_and_golden() {
    let (_fixture, enabled, repo) = repository_with_operations();
    let data = enabled.data_directory.path();
    operations::insert_remote_poll(
        data,
        "01ARZ3NDEKTSV4RRFFQ69G5FA5",
        "failed",
        Some("after_advertisement"),
        Some("protocol_rejected"),
    );

    let operation = enabled
        .service
        .show_operation(
            &repo,
            operations::operation_id("01ARZ3NDEKTSV4RRFFQ69G5FA5"),
        )
        .unwrap();
    let envelope = Envelope::read_success("operation show", repo.scope(), operation.clone());
    let not_found = enabled
        .service
        .show_operation(
            &repo,
            operations::operation_id(operations::OPERATION_ABSENT),
        )
        .unwrap_err();

    assert_eq!(
        operation.failure_code,
        Some(OperationFailureCode::ProtocolRejected)
    );
    let placeholders = [(repo.root().to_str().unwrap(), "<repository>")];
    let sentinels = [operations::KEY_MATERIAL_SENTINEL, data.to_str().unwrap()];
    golden::assert_contract(
        &ContractCase {
            name: "operation_show",
            data_schema: Some("operation.schema.json"),
            placeholders: &placeholders,
            sentinels: &sentinels,
        },
        &envelope,
    );
    assert_eq!(not_found.code(), ResultCode::OperationNotFound);
    golden::assert_contract(
        &ContractCase {
            name: "failure_operation_not_found",
            data_schema: None,
            placeholders: &placeholders,
            sentinels: &sentinels,
        },
        &not_found.to_envelope::<Value>("operation show"),
    );
    assert_git_transport_uninitialized();
}

#[test]
fn every_stored_operation_shape_matches_the_operation_schema() {
    let (_fixture, enabled, repo) = repository_with_operations();
    let data = enabled.data_directory.path();
    operations::insert_key_material(
        data,
        "01ARZ3NDEKTSV4RRFFQ69G5FA6",
        "delete",
        "completed",
        None,
    );
    operations::insert_local(
        data,
        Some("01ARZ3NDEKTSV4RRFFQ69G5FA7"),
        "rebuild",
        "completed",
        Some("completed"),
    );

    let mut shapes = enabled.service.list_operations(&repo).unwrap().items;
    for id in ["01ARZ3NDEKTSV4RRFFQ69G5FA6", "01ARZ3NDEKTSV4RRFFQ69G5FA7"] {
        shapes.push(
            enabled
                .service
                .show_operation(&repo, operations::operation_id(id))
                .unwrap(),
        );
    }

    assert_eq!(shapes.len(), 7);
    for operation in shapes {
        let value = serde_json::to_value(&operation).unwrap();
        schema::check_published("operation.schema.json", &value).unwrap();
    }
    assert_git_transport_uninitialized();
}
