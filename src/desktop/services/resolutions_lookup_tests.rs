//! `ResolutionStore::kept_for` (Task 11b review M-5): a set found by its
//! conflict, the newest first, passing by the requests excepted and the
//! sets of another root or operation.

use std::path::PathBuf;

use gpui_kit::TestAppContext;
use manyhands::repository::{OperationId, RequestId};

use super::tests::{a_set, fields};
use crate::desktop::{services::conflicts::ConflictSource, test_support::fixtures::Fixture};

#[gpui_kit::test]
fn kept_for_finds_the_newest_set_of_its_conflict(_cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let store = fixture.resolution_store();
    let root = PathBuf::from("/nowhere/a");
    let operation = OperationId::new();
    // Request IDs in a known order: two made in one millisecond need not be.
    let mut older = a_set(&root, operation, ConflictSource::Remote, "Older");
    older.request_id = RequestId::parse("01ARZ3NDEKTSV4RRFFQ69G5FA0").unwrap();
    let mut newer = a_set(&root, operation, ConflictSource::Remote, "Newer");
    newer.request_id = RequestId::parse("01ARZ3NDEKTSV4RRFFQ69G5FB0").unwrap();
    let elsewhere = a_set(
        &PathBuf::from("/nowhere/b"),
        operation,
        ConflictSource::Remote,
        "B",
    );
    let other = a_set(&root, OperationId::new(), ConflictSource::Remote, "Other");
    for set in [&older, &newer, &elsewhere, &other] {
        store.keep(set).unwrap();
    }
    let found = |except: &[_]| {
        store
            .kept_for(&root, operation, except)
            .unwrap()
            .as_ref()
            .map(fields)
    };

    assert_eq!(found(&[]), Some(fields(&newer)), "the newest first");
    assert_eq!(found(&[newer.request_id]), Some(fields(&older)), "except");
    assert_eq!(found(&[newer.request_id, older.request_id]), None);
    assert_eq!(
        store
            .kept_for(&PathBuf::from("/nowhere/c"), operation, &[])
            .unwrap()
            .map(|set| set.request_id),
        None,
        "by root"
    );
    assert_eq!(
        store
            .kept_for(&root, other.operation_id, &[])
            .unwrap()
            .map(|set| set.request_id),
        Some(other.request_id),
        "by operation"
    );
}
