//! A kept file this build cannot use as a set: one a newer build wrote,
//! and one that does not parse (Task 11a review fixes M-b and M-d).

use std::{fs, path::PathBuf};

use gpui_kit::TestAppContext;
use manyhands::repository::OperationId;

use super::{
    RESOLUTIONS, ResolutionStore, ResolutionStoreError,
    tests::{a_set, fields, kept},
};
use crate::desktop::{services::conflicts::ConflictSource, test_support::fixtures::Fixture};

/// The kept file of `request` in the fixture's data directory.
fn file_of(fixture: &Fixture, request: impl std::fmt::Display) -> PathBuf {
    fixture
        .data_dir()
        .join(RESOLUTIONS)
        .join(format!("{request}.json"))
}

/// Rewrites a kept file as a later format would: version 2.
fn made_newer(file: &PathBuf) {
    let written = fs::read_to_string(file).unwrap();
    assert!(written.contains("\"version\":1,"));
    fs::write(file, written.replace("\"version\":1,", "\"version\":2,")).unwrap();
}

/// M-b: a kept file of a newer format reads as `Newer`, not as a set and
/// not as damaged, and is kept by `settle` (and so by the sweep) even with
/// no record of its request: a newer build may still send it. One that
/// does not parse at all is `Damaged`, and goes.
#[gpui_kit::test]
fn a_newer_version_is_kept_and_a_damaged_one_goes(_cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let backend = fixture.delaying_backend();
    let store = fixture.resolution_store();
    let root = fixture.data_dir().join("a");

    let newer = a_set(&root, OperationId::new(), ConflictSource::Local, "Newer");
    store.keep(&newer).unwrap();
    made_newer(&file_of(&fixture, newer.request_id));
    let reopened = ResolutionStore::open_in(fixture.data_dir()).unwrap();
    assert!(matches!(
        reopened.kept(newer.request_id),
        Err(ResolutionStoreError::Newer)
    ));

    let damaged = a_set(&root, OperationId::new(), ConflictSource::Local, "Damaged");
    store.keep(&damaged).unwrap();
    fs::write(file_of(&fixture, damaged.request_id), "{\"version\":1,").unwrap();
    assert!(matches!(
        store.kept(damaged.request_id),
        Err(ResolutionStoreError::Damaged)
    ));

    // Neither request has a record.
    assert!(!store.settle(backend.as_ref(), newer.request_id));
    assert!(store.settle(backend.as_ref(), damaged.request_id));
    store.sweep(backend.as_ref());
    assert_eq!(store.requests().unwrap(), vec![newer.request_id]);
}

/// M-d: `keep` over a kept file it cannot read as a set says so, with the
/// file's own error, not `Differs`, and leaves the file as it was.
#[gpui_kit::test]
fn keep_over_an_unreadable_file_names_it(_cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let store = fixture.resolution_store();
    let root = fixture.data_dir().join("a");

    let damaged = a_set(&root, OperationId::new(), ConflictSource::Remote, "Damaged");
    fs::write(file_of(&fixture, damaged.request_id), "not a set").unwrap();
    assert!(matches!(
        store.keep(&damaged),
        Err(ResolutionStoreError::Damaged)
    ));
    assert_eq!(
        fs::read_to_string(file_of(&fixture, damaged.request_id)).unwrap(),
        "not a set"
    );

    let newer = a_set(&root, OperationId::new(), ConflictSource::Remote, "Newer");
    store.keep(&newer).unwrap();
    made_newer(&file_of(&fixture, newer.request_id));
    assert!(matches!(
        store.keep(&newer),
        Err(ResolutionStoreError::Newer)
    ));

    // A set that reads is still `Differs` against a different one.
    let mut kept_set = a_set(&root, OperationId::new(), ConflictSource::Remote, "Kept");
    store.keep(&kept_set).unwrap();
    let before = fields(&kept_set);
    kept_set.observation = "another".to_owned();
    assert!(matches!(
        store.keep(&kept_set),
        Err(ResolutionStoreError::Differs)
    ));
    assert_eq!(kept(&store, kept_set.request_id), Some(before));
}
