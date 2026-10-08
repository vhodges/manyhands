//! The published JSON v1 contract: schemas, golden envelopes and the
//! redaction scan. Each read added later gets one `ContractCase` here.

use std::{collections::BTreeSet, fs, path::Path, str::FromStr};

use manyhands::{
    canonical::ItemId,
    repository::{ProblemDto, RepositoryService},
    results::{Envelope, Outcome, ProblemCode, ResultCode, Scope},
};
use serde_json::{Value, json};
use support::{
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
            "type": "object",
            "additionalProperties": false,
            "required": ["id", "state", "note", "tags", "owner"],
            "properties": {
                "id": {"type": "string"},
                "state": {"type": "string", "enum": ["open", "closed"]},
                "note": {"type": ["string", "null"]},
                "tags": {"type": "array", "items": {"type": "string"}},
                "owner": {"$ref": "owner.schema.json"},
                "count": {"type": "integer"},
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
            json!({"type": "object", "description": "x"}),
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
    ];
    for (schema, expected) in cases {
        let directory = tempfile::tempdir().unwrap();
        write_schema(
            directory.path(),
            "bad.schema.json",
            json!({"format": "uri"}),
        );
        write_schema(directory.path(), "case.schema.json", schema.clone());

        // `null` conforms to nothing these schemas say, and to everything
        // they leave unsaid: only the schema itself can be at fault.
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

fn published_schema_files() -> Vec<String> {
    let mut files: Vec<_> = fs::read_dir(schema::schema_directory())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    files.sort();
    files
}

#[test]
fn every_published_schema_uses_only_the_checked_keywords() {
    let files = published_schema_files();

    assert!(files.contains(&golden::ENVELOPE_SCHEMA.to_owned()));
    for file in files {
        assert!(file.ends_with(".schema.json"), "{file}");
        schema::validate(&schema::schema_directory(), &file).unwrap();
    }
    assert_git_transport_uninitialized();
}

#[test]
fn every_golden_fixture_is_canonical_text() {
    for entry in fs::read_dir(golden::fixture_directory()).unwrap() {
        let path = entry.unwrap().path();
        let text = fs::read_to_string(&path).unwrap();
        let value: Value = serde_json::from_str(&text).unwrap();

        assert_eq!(path.extension().unwrap(), "json", "{path:?}");
        assert_eq!(golden::render(&value, &[]), text, "{path:?}");
        schema::check_published(golden::ENVELOPE_SCHEMA, &value).unwrap();
    }
    assert_git_transport_uninitialized();
}

fn schema_enum(file: &str, property: &str) -> Vec<String> {
    let text = fs::read_to_string(schema::schema_directory().join(file)).unwrap();
    let schema: Value = serde_json::from_str(&text).unwrap();
    schema["properties"][property]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn published_enumerations_list_exactly_the_registered_codes() {
    assert_eq!(
        schema_enum(golden::ENVELOPE_SCHEMA, "code"),
        ResultCode::ALL.map(ResultCode::as_str)
    );
    assert_eq!(
        schema_enum("problem.schema.json", "code"),
        ProblemCode::ALL.map(ProblemCode::as_str)
    );
    assert_eq!(
        schema_enum(golden::ENVELOPE_SCHEMA, "outcome"),
        [
            Outcome::Success,
            Outcome::Noop,
            Outcome::Partial,
            Outcome::Blocked,
            Outcome::Cancelled,
            Outcome::Error,
        ]
        .map(Outcome::as_str)
    );
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
            data_schema: "new_id.schema.json",
            placeholders: &[(&id, "<item-id>")],
            sentinels: &[],
        },
        &envelope,
    );
    assert_git_transport_uninitialized();
}

#[test]
fn a_degraded_index_matches_the_failure_golden() {
    let data = tempfile::tempdir().unwrap();
    fs::write(data.path().join("manyhands.sqlite3"), b"not sqlite").unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();

    let error = service.read_session_for_testing(|_| ()).unwrap_err();
    let directory = data.path().to_str().unwrap();

    assert_eq!(error.code, ResultCode::IndexUnavailable);
    golden::assert_contract(
        &ContractCase {
            name: "failure_index_unavailable",
            data_schema: "new_id.schema.json",
            placeholders: &[],
            // The data directory is not the caller's business.
            sentinels: &[directory, "not sqlite", "manyhands.sqlite3"],
        },
        &error.to_envelope::<Value>("item list"),
    );
    assert_git_transport_uninitialized();
}
