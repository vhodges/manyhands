//! Golden JSON v1 envelopes under `tests/fixtures/read_v1`.
//!
//! An envelope is written with sorted keys, two-space indentation and a
//! final newline, after the values that differ from run to run are replaced
//! by fixed placeholders, and must equal its fixture byte for byte.
//! `MANYHANDS_UPDATE_GOLDEN=1` rewrites the fixtures instead; it is a
//! developer aid that no check sets.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;
use serde_json::{Map, Value};

use super::schema;

pub const UPDATE_VARIABLE: &str = "MANYHANDS_UPDATE_GOLDEN";
pub const ENVELOPE_SCHEMA: &str = "envelope.schema.json";

pub fn fixture_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/read_v1")
}

/// One envelope a read produced, and everything its contract test checks.
pub struct ContractCase<'a> {
    /// The fixture is `tests/fixtures/read_v1/<name>.json`.
    pub name: &'a str,
    /// The published schema file for the envelope's `data`.
    pub data_schema: &'a str,
    /// `(value, placeholder)` pairs for what differs from run to run:
    /// absolute paths, object IDs, generated IDs and unfixed timestamps.
    pub placeholders: &'a [(&'a str, &'a str)],
    /// Strings planted in the fixture that must not appear in the envelope.
    pub sentinels: &'a [&'a str],
}

/// Checks one envelope against the published contract: the redaction scan,
/// the envelope schema, the data schema, and the golden fixture.
pub fn assert_contract(case: &ContractCase<'_>, envelope: &impl Serialize) {
    let name = case.name;
    let value = serde_json::to_value(envelope).unwrap();
    let compact = serde_json::to_string(&value).unwrap();
    for sentinel in case.sentinels {
        assert!(!sentinel.is_empty(), "{name}: an empty sentinel");
        assert!(
            !compact.contains(sentinel),
            "{name}: the envelope contains {sentinel:?}"
        );
    }

    if let Err(problem) = schema::check_published(ENVELOPE_SCHEMA, &value) {
        panic!("{name}: the envelope does not match {ENVELOPE_SCHEMA}: {problem}");
    }
    let data = &value["data"];
    assert_eq!(
        data.is_null(),
        value["outcome"] != "success",
        "{name}: a read carries data exactly when it succeeds"
    );
    if !data.is_null()
        && let Err(problem) = schema::check_published(case.data_schema, data)
    {
        panic!(
            "{name}: data does not match {}: {problem}",
            case.data_schema
        );
    }

    assert_golden(name, &value, case.placeholders);
}

/// Compares `value` with its fixture, or rewrites the fixture when
/// `MANYHANDS_UPDATE_GOLDEN=1`.
pub fn assert_golden(name: &str, value: &Value, placeholders: &[(&str, &str)]) {
    let actual = render(value, placeholders);
    let path = fixture_directory().join(format!("{name}.json"));
    if std::env::var_os(UPDATE_VARIABLE).is_some_and(|value| value == "1") {
        fs::create_dir_all(fixture_directory()).unwrap();
        fs::write(&path, &actual).unwrap();
        return;
    }
    let expected = fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "{name}: cannot read {}: {error}; set {UPDATE_VARIABLE}=1 to write it",
            path.display()
        )
    });
    assert!(
        expected == actual.as_bytes(),
        "{name}: the envelope differs from {}\n--- expected\n{}\n--- actual\n{actual}",
        path.display(),
        String::from_utf8_lossy(&expected)
    );
}

/// The canonical text of a golden fixture: placeholders substituted, keys
/// sorted, two-space indentation, one final newline.
pub fn render(value: &Value, placeholders: &[(&str, &str)]) -> String {
    // The longest value first, so that a path is replaced before a path it
    // contains.
    let mut placeholders = placeholders.to_vec();
    placeholders.sort_by_key(|(value, _)| std::cmp::Reverse(value.len()));
    for (value, _) in &placeholders {
        assert!(
            !value.is_empty(),
            "an empty value cannot have a placeholder"
        );
    }
    let mut text = serde_json::to_string_pretty(&normalize(value, &placeholders)).unwrap();
    text.push('\n');
    text
}

/// Substitutes placeholders in string values, never in keys, and rebuilds
/// every object in key order whatever order `serde_json` keeps.
fn normalize(value: &Value, placeholders: &[(&str, &str)]) -> Value {
    match value {
        Value::String(text) => {
            let mut text = text.clone();
            for (value, placeholder) in placeholders {
                text = text.replace(value, placeholder);
            }
            Value::String(text)
        }
        Value::Array(elements) => Value::Array(
            elements
                .iter()
                .map(|element| normalize(element, placeholders))
                .collect(),
        ),
        Value::Object(object) => {
            let mut entries: Vec<_> = object.iter().collect();
            entries.sort_by(|left, right| left.0.cmp(right.0));
            Value::Object(
                entries
                    .into_iter()
                    .map(|(key, value)| (key.clone(), normalize(value, placeholders)))
                    .collect::<Map<_, _>>(),
            )
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => value.clone(),
    }
}
