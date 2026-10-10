//! Shared support for the mutation boundary tests: a clock the test sets,
//! the privacy scan of the request stores, and the golden envelopes under
//! `tests/fixtures/mutation_v1`.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
};

use manyhands::repository::{RepositoryService, request_store::Clock};
use rusqlite::types::ValueRef;
use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;

use super::{golden, items, schema};

/// A clock that reads what the test last set, in seconds.
pub struct TestClock(AtomicI64);

impl TestClock {
    pub fn at(seconds: i64) -> Arc<Self> {
        Arc::new(Self(AtomicI64::new(seconds)))
    }

    pub fn set(&self, seconds: i64) {
        self.0.store(seconds, Ordering::SeqCst);
    }
}

impl Clock for TestClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(self.0.load(Ordering::SeqCst)).unwrap()
    }
}

/// A service over `data_directory` whose records are timed by `clock`.
pub fn service_with_clock(data_directory: &Path, clock: &Arc<TestClock>) -> RepositoryService {
    RepositoryService::open_at(data_directory)
        .unwrap()
        .with_clock_for_testing(clock.clone())
}

/// The three request tables and the three journals: every store a request
/// leaves a row in.
pub const REQUEST_STORES: [&str; 6] = [
    "request_records",
    "request_operations",
    "confirmation_records",
    "operation_records",
    "remote_operation_records",
    "key_material_operations",
];

/// Where a planted sentinel was found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredSentinel {
    pub sentinel: String,
    pub table: &'static str,
    pub column: String,
}

fn bytes_contain(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// The first of `sentinels` found in any column of any row of the request
/// stores, with where it is.
///
/// A scenario plants its sentinels in what no store may keep: a Markdown
/// body or source, a passphrase, key bytes, text a server sent. Every
/// column is read, whatever its type, as the bytes SQLite holds; a text
/// value that is JSON is also read as its strings, so a sentinel holding a
/// quote or a backslash is still found. A store that does not exist fails
/// the scan: it would otherwise pass by reading nothing.
pub fn find_stored_sentinel(data_directory: &Path, sentinels: &[&str]) -> Option<StoredSentinel> {
    for sentinel in sentinels {
        assert!(!sentinel.is_empty(), "an empty sentinel matches everything");
    }
    let connection = items::index(data_directory);
    for table in REQUEST_STORES {
        let columns: Vec<String> = connection
            .prepare("SELECT name FROM pragma_table_info(?1) ORDER BY cid")
            .unwrap()
            .query_map([table], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(!columns.is_empty(), "{table}: the store does not exist");
        let mut statement = connection
            .prepare(&format!("SELECT * FROM {table}"))
            .unwrap();
        let mut rows = statement.query([]).unwrap();
        while let Some(row) = rows.next().unwrap() {
            for (index, column) in columns.iter().enumerate() {
                let found = |sentinel: &str| StoredSentinel {
                    sentinel: sentinel.to_owned(),
                    table,
                    column: column.clone(),
                };
                let bytes = match row.get_ref(index).unwrap() {
                    ValueRef::Null => continue,
                    ValueRef::Integer(value) => value.to_string().into_bytes(),
                    ValueRef::Real(value) => value.to_string().into_bytes(),
                    ValueRef::Text(text) => text.to_vec(),
                    ValueRef::Blob(blob) => blob.to_vec(),
                };
                if let Some(sentinel) = sentinels
                    .iter()
                    .find(|sentinel| bytes_contain(&bytes, sentinel.as_bytes()))
                {
                    return Some(found(sentinel));
                }
                if let Ok(json) = serde_json::from_slice::<Value>(&bytes)
                    && let Some((sentinel, _)) = golden::find_sentinel(&json, sentinels)
                {
                    return Some(found(sentinel));
                }
            }
        }
    }
    None
}

/// Fails when any request store holds one of `sentinels`.
pub fn assert_request_stores_exclude(data_directory: &Path, sentinels: &[&str]) {
    if let Some(found) = find_stored_sentinel(data_directory, sentinels) {
        panic!(
            "{}.{} holds the sentinel {:?}",
            found.table, found.column, found.sentinel
        );
    }
}

/// Every golden envelope of the mutation boundary, by fixture name. A
/// fixture with no entry, or an entry with no fixture, fails
/// `tests/mutation_replay.rs`.
pub const CASES: &[&str] = &[
    "failure_request_not_found",
    "request_show_accepted",
    "request_show_finished",
];

pub fn fixture_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mutation_v1")
}

/// Checks one envelope of the mutation boundary against the published
/// contract, as `golden::assert_mutation_contract` checks one under
/// `read_v1`: the redaction scan, the envelope schema, the registered
/// recovery actions, the data schema when the case names one, and the
/// golden fixture `tests/fixtures/mutation_v1/<name>.json`.
pub fn assert_contract(case: &golden::ContractCase<'_>, envelope: &impl Serialize) {
    let name = case.name;
    let value = serde_json::to_value(envelope).unwrap();
    assert!(
        CASES.contains(&name),
        "{name}: not listed in mutation::CASES"
    );
    if let Some((sentinel, location)) = golden::find_sentinel(&value, case.sentinels) {
        panic!("{name}: the envelope contains {sentinel:?} at {location}");
    }
    if let Err(problem) = schema::check_published(golden::ENVELOPE_SCHEMA, &value) {
        panic!(
            "{name}: the envelope does not match {}: {problem}",
            golden::ENVELOPE_SCHEMA
        );
    }
    golden::assert_registered_recovery(name, &value);
    let data = &value["data"];
    match case.data_schema {
        Some(data_schema) => {
            assert!(!data.is_null(), "{name}: a data schema needs data");
            if let Err(problem) = schema::check_published(data_schema, data) {
                panic!("{name}: data does not match {data_schema}: {problem}");
            }
        }
        None => assert!(data.is_null(), "{name}: data needs a data schema"),
    }

    let actual = golden::render(&value, case.placeholders);
    let path = fixture_directory().join(format!("{name}.json"));
    if golden::update_requested(
        std::env::var_os(golden::UPDATE_VARIABLE).as_deref(),
        std::env::var_os("CI").as_deref(),
    ) {
        fs::create_dir_all(fixture_directory()).unwrap();
        fs::write(&path, &actual).unwrap();
        return;
    }
    let expected = fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "{name}: cannot read {}: {error}; set {}=1 to write it",
            path.display(),
            golden::UPDATE_VARIABLE
        )
    });
    assert!(
        expected == actual.as_bytes(),
        "{name}: the envelope differs from {}\n--- expected\n{}\n--- actual\n{actual}",
        path.display(),
        String::from_utf8_lossy(&expected)
    );
}
