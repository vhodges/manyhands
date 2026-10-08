//! Golden JSON v1 envelopes under `tests/fixtures/read_v1`.
//!
//! An envelope is written with sorted keys, two-space indentation and a
//! final newline, after the values that differ from run to run are replaced
//! by fixed placeholders, and must equal its fixture byte for byte.
//! `MANYHANDS_UPDATE_GOLDEN=1` rewrites the fixtures instead; it is a
//! developer aid that no check sets, and it is refused when `CI` is set.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;
use serde_json::{Map, Value};

use super::schema;

pub const UPDATE_VARIABLE: &str = "MANYHANDS_UPDATE_GOLDEN";
pub const ENVELOPE_SCHEMA: &str = "envelope.schema.json";

/// Every contract case, by fixture name. A case that is not listed here is
/// refused, and a fixture file with no entry fails `tests/read_contract.rs`,
/// so the directory holds exactly the envelopes some test still produces.
pub const CASES: &[&str] = &[
    "document_list",
    "document_show",
    "failure_authority_not_found",
    "failure_index_unavailable",
    "failure_item_not_found",
    "failure_key_not_found",
    "failure_not_repository_root",
    "failure_public_key_unavailable",
    "host_inspect",
    "host_list",
    "id_new",
    "key_list",
    "key_public",
    "key_show",
    "remote_list",
    "repo_identity",
    "repo_inspect",
    "repo_list",
    "ticket_list",
    "ticket_list_nonconforming",
    "ticket_show",
];

/// A shorter value is too likely to occur inside unrelated data.
const SHORTEST_PLACEHOLDER_TARGET: usize = 8;

pub fn fixture_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/read_v1")
}

/// One envelope a read produced, and everything its contract test checks.
pub struct ContractCase<'a> {
    /// The fixture is `tests/fixtures/read_v1/<name>.json`; `name` must be
    /// listed in `CASES`.
    pub name: &'a str,
    /// The published schema file for the envelope's `data`, or `None` for a
    /// failure, which carries no data.
    pub data_schema: Option<&'a str>,
    /// `(value, placeholder)` pairs for what differs from run to run:
    /// absolute paths, object IDs, generated IDs and unfixed timestamps. A
    /// string is replaced when it is the value, or a path under it.
    pub placeholders: &'a [(&'a str, &'a str)],
    /// Strings planted in the fixture that must not appear in the envelope.
    pub sentinels: &'a [&'a str],
}

/// Checks one envelope against the published contract: the redaction scan,
/// the envelope schema, the data schema, and the golden fixture.
pub fn assert_contract(case: &ContractCase<'_>, envelope: &impl Serialize) {
    let name = case.name;
    assert!(CASES.contains(&name), "{name}: not listed in golden::CASES");
    let value = serde_json::to_value(envelope).unwrap();
    if let Some((sentinel, location)) = find_sentinel(&value, case.sentinels) {
        panic!("{name}: the envelope contains {sentinel:?} at {location}");
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
    match case.data_schema {
        Some(data_schema) => {
            assert!(!data.is_null(), "{name}: a failure names no data schema");
            if let Err(problem) = schema::check_published(data_schema, data) {
                panic!("{name}: data does not match {data_schema}: {problem}");
            }
        }
        None => assert!(data.is_null(), "{name}: data needs a data schema"),
    }

    assert_golden(name, &value, case.placeholders);
}

/// The first sentinel found in a key or a string value, with where it is.
/// The scan reads the strings themselves, not their JSON text, so a sentinel
/// holding a quote or a backslash is still found.
pub fn find_sentinel<'a>(value: &Value, sentinels: &[&'a str]) -> Option<(&'a str, String)> {
    for sentinel in sentinels {
        assert!(!sentinel.is_empty(), "an empty sentinel matches everything");
    }
    find_sentinel_at(value, sentinels, "$")
}

fn find_sentinel_at<'a>(
    value: &Value,
    sentinels: &[&'a str],
    location: &str,
) -> Option<(&'a str, String)> {
    let within = |text: &str| {
        sentinels
            .iter()
            .copied()
            .find(|sentinel| text.contains(sentinel))
    };
    match value {
        Value::String(text) => within(text).map(|sentinel| (sentinel, location.to_owned())),
        Value::Array(elements) => elements.iter().enumerate().find_map(|(index, element)| {
            find_sentinel_at(element, sentinels, &format!("{location}[{index}]"))
        }),
        Value::Object(object) => object.iter().find_map(|(key, value)| {
            let location = format!("{location}.{key}");
            within(key)
                .map(|sentinel| (sentinel, format!("{location} (key)")))
                .or_else(|| find_sentinel_at(value, sentinels, &location))
        }),
        Value::Null | Value::Bool(_) | Value::Number(_) => None,
    }
}

/// Compares `value` with its fixture, or rewrites the fixture when
/// `MANYHANDS_UPDATE_GOLDEN=1`.
pub fn assert_golden(name: &str, value: &Value, placeholders: &[(&str, &str)]) {
    let actual = render(value, placeholders);
    let path = fixture_directory().join(format!("{name}.json"));
    if update_requested(
        std::env::var_os(UPDATE_VARIABLE).as_deref(),
        std::env::var_os("CI").as_deref(),
    ) {
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

/// Whether to rewrite fixtures, given the values of `MANYHANDS_UPDATE_GOLDEN`
/// and `CI`. A check must compare, so asking for an update under `CI` panics.
pub fn update_requested(update: Option<&std::ffi::OsStr>, ci: Option<&std::ffi::OsStr>) -> bool {
    let requested = update.is_some_and(|value| value == "1");
    assert!(
        !(requested && ci.is_some()),
        "{UPDATE_VARIABLE} must not be set when CI is set"
    );
    requested
}

/// The canonical text of a golden fixture: placeholders substituted, keys
/// sorted, two-space indentation, one final newline.
pub fn render(value: &Value, placeholders: &[(&str, &str)]) -> String {
    // The longest value first, so that a path is replaced before a path it
    // lies under.
    let mut placeholders = placeholders.to_vec();
    placeholders.sort_by_key(|(value, _)| std::cmp::Reverse(value.len()));
    for (value, _) in &placeholders {
        assert!(
            value.len() >= SHORTEST_PLACEHOLDER_TARGET,
            "{value:?} is too short to be given a placeholder"
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
        Value::String(text) => Value::String(substitute(text, placeholders)),
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

/// Replaces `text` when it is a placeholder's value, or a path under it:
/// the value followed by a path separator. Nothing else is touched, so a
/// value that merely occurs inside other data stays as it is.
fn substitute(text: &str, placeholders: &[(&str, &str)]) -> String {
    for (value, placeholder) in placeholders {
        match text.strip_prefix(value) {
            Some("") => return (*placeholder).to_owned(),
            Some(rest) if rest.starts_with(['/', '\\']) => return format!("{placeholder}{rest}"),
            _ => {}
        }
    }
    text.to_owned()
}
