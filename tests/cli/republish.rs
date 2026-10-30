//! `item republish` through the binary, without SSH. A deleted branch needs
//! a recorded publication, which needs the SSH fixture, so the preview
//! golden, the confirmed push and the interactive consent are Task 12b's.
//!
//! The library's own table (`tests/mutation_remote/republish.rs`) pins what
//! the library answers. These cases pin what the verb adds: C2's mode rules,
//! the item lookup, and the refusals that need no remote.

use serde_json::{Value, json};

use super::{index::Fixture, mutation};
use crate::support::{self, items, mutation::REQUEST_1};

/// A confirmation ID no preview made.
const UNKNOWN_CONFIRMATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC1";
const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM0";
const OBSERVATION: &str =
    "conflict-v1:0000000000000000000000000000000000000000000000000000000000000000";
const TOKEN: &str =
    "conflict-path-v1:1111111111111111111111111111111111111111111111111111111111111111";

impl Fixture {
    fn republish(&self, extra: &[&str], status: i32, code: &str) -> Value {
        let mut flags = vec!["--id", items::TICKET_A];
        flags.extend_from_slice(extra);
        let value = self.checked("item", "republish", &flags, &[], status, code);
        assert_eq!(value["command"], "item republish");
        value
    }
}

#[test]
fn item_republish_publication_remote_required() {
    let f = Fixture::new();
    let before = support::repository_and_worktree_snapshot(&f.repo);
    // `--prepare` reads stored evidence and needs a publication remote
    // before it answers anything else.
    let value = f.republish(&["--prepare"], 3, "publication_remote_required");
    assert!(value["request_id"].is_null(), "{value}");
    assert!(value["data"].is_null(), "{value}");
    assert_eq!(value["scope"]["item_id"], items::TICKET_A);
    mutation::golden(
        "item_republish_publication_remote_required",
        &value,
        &f.repo.root,
    );
    // The document of the fixture is as good an item: the kind is read, not
    // given.
    let document = f.checked(
        "item",
        "republish",
        &["--id", items::DOCUMENT_A, "--prepare"],
        &[],
        3,
        "publication_remote_required",
    );
    assert_eq!(document["scope"]["item_id"], items::DOCUMENT_A);
    assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
}

#[test]
fn an_unknown_item_is_item_not_found_before_anything_is_prepared() {
    let f = Fixture::new();
    let before = support::repository_and_worktree_snapshot(&f.repo);
    for extra in [vec!["--prepare"], vec!["--request-id", REQUEST_1]] {
        let mut flags = vec!["--id", items::TICKET_ABSENT];
        flags.extend(extra.iter().copied());
        let value = f.checked("item", "republish", &flags, &[], 2, "item_not_found");
        assert_eq!(value["scope"]["item_id"], items::TICKET_ABSENT);
    }
    let malformed = f.checked(
        "item",
        "republish",
        &["--id", "not-a-ulid", "--prepare"],
        &[],
        2,
        "invalid_id",
    );
    assert!(malformed["request_id"].is_null());
    assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
}

#[test]
fn an_unknown_confirmation_is_confirmation_not_found() {
    let f = Fixture::new();
    let before = support::repository_and_worktree_snapshot(&f.repo);
    let value = f.republish(
        &["--request-id", REQUEST_1, "--confirm", UNKNOWN_CONFIRMATION],
        3,
        "confirmation_not_found",
    );
    assert!(value["data"].is_null(), "{value}");
    assert_eq!(value["effects"]["publication"], "not_requested");
    assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
}

#[test]
fn a_malformed_confirmation_is_invalid_id() {
    let f = Fixture::new();
    f.republish(
        &["--request-id", REQUEST_1, "--confirm", "not-a-ulid"],
        2,
        "invalid_id",
    );
}

#[test]
fn prepare_refuses_a_request_id_and_a_confirmation_as_usage_errors() {
    let f = Fixture::new();
    let before = support::repository_and_worktree_snapshot(&f.repo);
    for extra in [
        vec!["--prepare", "--request-id", REQUEST_1],
        vec!["--prepare", "--confirm", UNKNOWN_CONFIRMATION],
        vec![
            "--prepare",
            "--request-id",
            REQUEST_1,
            "--confirm",
            UNKNOWN_CONFIRMATION,
        ],
    ] {
        let value = f.republish(&extra, 2, "usage_error");
        assert!(value["request_id"].is_null(), "{extra:?}");
    }
    assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
}

#[test]
fn item_republish_refuses_input_and_needs_its_item() {
    let f = Fixture::new();
    let value = f.checked(
        "item",
        "republish",
        &[
            "--id",
            items::TICKET_A,
            "--request-id",
            REQUEST_1,
            "--input",
            "-",
        ],
        b"{}",
        2,
        "usage_error",
    );
    assert!(value["request_id"].is_null());
    let value = f.checked(
        "item",
        "republish",
        &["--request-id", REQUEST_1],
        &[],
        2,
        "usage_error",
    );
    assert!(value["request_id"].is_null());
}

#[test]
fn a_json_run_needs_a_request_id_to_execute() {
    let f = Fixture::new();
    let value = f.republish(&[], 2, "usage_error");
    assert!(value["request_id"].is_null());
}

/// Only `item republish` is confirmed among the C4 verbs that start work
/// (`operation abandon` is the other confirmed one, and it does not start a
/// synchronization). The other five are not, so `--prepare` is refused for
/// each, with nothing recorded.
#[test]
fn prepare_of_each_unconfirmed_mutation_is_not_confirmable() {
    let f = Fixture::new();
    let before = support::repository_and_worktree_snapshot(&f.repo);
    let operation = manyhands::repository::OperationId::new().to_string();
    let comment = serde_json::to_vec(&json!({"comment_id": COMMENT, "body": "b"})).unwrap();
    let resolve = serde_json::to_vec(&json!({
        "observation": OBSERVATION,
        "resolutions": [{"token": TOKEN, "source": "s"}]
    }))
    .unwrap();
    for (resource, verb, flags, input) in [
        (
            "comment",
            "add",
            vec!["--id", items::TICKET_A, "--input", "-", "--prepare"],
            comment,
        ),
        (
            "item",
            "sync",
            vec!["--id", items::TICKET_A, "--prepare"],
            Vec::new(),
        ),
        ("repo", "sync", vec!["--prepare"], Vec::new()),
        (
            "conflict",
            "resolve",
            vec!["--operation-id", &operation, "--input", "-", "--prepare"],
            resolve,
        ),
        (
            "operation",
            "resume",
            vec!["--operation-id", &operation, "--prepare"],
            Vec::new(),
        ),
    ] {
        let value = f.checked(resource, verb, &flags, &input, 2, "not_confirmable");
        assert_eq!(value["command"], format!("{resource} {verb}"));
        assert!(value["request_id"].is_null(), "{resource} {verb}");
        assert!(value["data"].is_null(), "{resource} {verb}");
    }
    assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
}
