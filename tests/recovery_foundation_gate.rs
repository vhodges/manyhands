use manyhands::repository::{ExpectedPathObservation, IndexPending, OperationId};

mod support;

#[test]
fn operation_ids_round_trip_in_canonical_uppercase_only() {
    let source = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    let operation_id = OperationId::parse(source).unwrap();

    assert_eq!(operation_id.to_string(), source);
    assert!(OperationId::parse(&source.to_ascii_lowercase()).is_err());
}

#[test]
fn expected_path_observations_hash_exact_bytes() {
    assert_ne!(
        ExpectedPathObservation::from_bytes(b"before"),
        ExpectedPathObservation::from_bytes(b"after")
    );
}

#[test]
fn index_pending_retains_the_authoritative_result() {
    let pending = IndexPending::new("authoritative result");

    assert_eq!(pending.authoritative, "authoritative result");
}

#[test]
fn default_fixture_ids_are_unique_and_retry_ids_are_explicitly_shared() {
    let operation_id = support::operation_id();
    let root = std::path::Path::new("/repository");
    let first_default = support::enable_request(root);
    let second_default = support::enable_request(root);
    let initial = support::enable_request_with_operation_id(root, operation_id);
    let retry = support::enable_request_with_operation_id(root, operation_id);

    assert_ne!(first_default.operation_id, second_default.operation_id);
    assert_eq!(initial.operation_id, retry.operation_id);
}
