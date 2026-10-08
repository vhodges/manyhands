use std::{
    collections::BTreeSet,
    path::{MAIN_SEPARATOR, Path, PathBuf},
};

use serde_json::{Value, json};
use time::{Duration, OffsetDateTime, UtcOffset};

use super::{
    CheckpointEffect, CleanupEffect, DiscoveryEffect, Effects, Envelope, FailureClass,
    IntegrationEffect, OperationFailureCode, Outcome, ProblemCode, PublicationEffect, REDACTED,
    RecoveryAction, ResultCode, SCHEMA_VERSION, Scope, WriteEffect, absolute_path_string,
    object_id_string, redact_url, relative_path_string, timestamp_string,
};

const ENVELOPE_FIELDS: [&str; 11] = [
    "schema_version",
    "command",
    "request_id",
    "operation_id",
    "outcome",
    "code",
    "message",
    "scope",
    "effects",
    "data",
    "recovery",
];

/// The design's result-code table, in its order.
const DESIGN_CODES: [(&str, Option<FailureClass>); 17] = [
    ("ok", None),
    ("invalid_path", Some(FailureClass::Input)),
    ("not_repository", Some(FailureClass::Input)),
    ("not_repository_root", Some(FailureClass::Input)),
    ("bare_repository", Some(FailureClass::Input)),
    ("repository_not_registered", Some(FailureClass::Blocked)),
    ("repository_inaccessible", Some(FailureClass::Blocked)),
    ("invalid_id", Some(FailureClass::Input)),
    ("item_not_found", Some(FailureClass::Input)),
    ("path_not_found", Some(FailureClass::Input)),
    ("key_not_found", Some(FailureClass::Input)),
    ("public_key_unavailable", Some(FailureClass::Blocked)),
    ("authority_not_found", Some(FailureClass::Input)),
    ("operation_not_found", Some(FailureClass::Input)),
    ("index_unavailable", Some(FailureClass::Blocked)),
    ("busy", Some(FailureClass::Transient)),
    ("internal_error", Some(FailureClass::Internal)),
];

fn keys(value: &Value) -> Vec<&str> {
    value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect()
}

fn sorted(mut names: Vec<&str>) -> Vec<&str> {
    names.sort_unstable();
    names
}

#[test]
fn result_codes_match_the_design_table_exactly() {
    let actual: Vec<_> = ResultCode::ALL
        .iter()
        .map(|code| (code.as_str(), code.failure_class()))
        .collect();
    assert_eq!(actual, DESIGN_CODES);
}

#[test]
fn result_code_messages_are_fixed() {
    let expected = [
        "The request completed.",
        "That path cannot be used as a target.",
        "No Git repository exists at that path.",
        "That path is inside a repository but is not its root or a linked worktree root.",
        "The repository has no working tree.",
        "The repository is not enabled in Manyhands.",
        "The repository cannot be read.",
        "That ID is not a canonical ULID.",
        "No item has that ID.",
        "No canonical resource exists at that path.",
        "No key registration has that ID.",
        "The key registration has no readable public key.",
        "No host pin exists for that authority.",
        "No operation has that ID.",
        "The index is degraded and must be rebuilt.",
        "The repository is busy; try again.",
        "An internal error occurred.",
    ];
    let actual: Vec<_> = ResultCode::ALL.iter().map(|code| code.message()).collect();
    assert_eq!(actual, expected);
}

#[test]
fn result_codes_have_unique_snake_case_strings_and_fixed_messages() {
    let mut strings = BTreeSet::new();
    let mut messages = BTreeSet::new();
    for code in ResultCode::ALL {
        let string = code.as_str();
        assert!(strings.insert(string), "{string} is repeated");
        assert!(!string.is_empty());
        assert!(
            string
                .split('_')
                .all(|word| !word.is_empty() && word.bytes().all(|byte| byte.is_ascii_lowercase())),
            "{string} is not lower_snake_case"
        );

        let message = code.message();
        assert!(!message.trim().is_empty(), "{string} has no message");
        assert!(messages.insert(message), "{string} shares a message");
        assert_eq!(serde_json::to_value(code).unwrap(), json!(string));
    }
}

#[test]
fn result_code_outcomes_follow_the_failure_class() {
    for code in ResultCode::ALL {
        let expected = match code.failure_class() {
            None => Outcome::Success,
            Some(FailureClass::Blocked) => Outcome::Blocked,
            Some(FailureClass::Input | FailureClass::Transient | FailureClass::Internal) => {
                Outcome::Error
            }
            Some(class) => panic!("{} has unexpected class {class:?}", code.as_str()),
        };
        assert_eq!(code.outcome(), expected, "{}", code.as_str());
    }
}

#[test]
fn outcomes_and_effects_serialize_as_the_rfc_strings() {
    let outcomes = [
        (Outcome::Success, "success"),
        (Outcome::Noop, "noop"),
        (Outcome::Partial, "partial"),
        (Outcome::Blocked, "blocked"),
        (Outcome::Cancelled, "cancelled"),
        (Outcome::Error, "error"),
    ];
    for (outcome, expected) in outcomes {
        assert_eq!(outcome.as_str(), expected);
        assert_eq!(serde_json::to_value(outcome).unwrap(), json!(expected));
    }

    macro_rules! assert_strings {
        ($($value:expr => $expected:literal),+ $(,)?) => {$(
            assert_eq!($value.as_str(), $expected);
            assert_eq!(serde_json::to_value($value).unwrap(), json!($expected));
        )+};
    }
    assert_strings! {
        WriteEffect::NotRequested => "not_requested",
        WriteEffect::Unchanged => "unchanged",
        WriteEffect::Written => "written",
        CheckpointEffect::NotRequested => "not_requested",
        CheckpointEffect::Unchanged => "unchanged",
        CheckpointEffect::Committed => "committed",
        CheckpointEffect::Pending => "pending",
        DiscoveryEffect::NotRequested => "not_requested",
        DiscoveryEffect::Current => "current",
        DiscoveryEffect::Pending => "pending",
        PublicationEffect::NotRequested => "not_requested",
        PublicationEffect::Published => "published",
        PublicationEffect::Current => "current",
        PublicationEffect::Pending => "pending",
        IntegrationEffect::NotRequested => "not_requested",
        IntegrationEffect::Complete => "complete",
        IntegrationEffect::Pending => "pending",
        CleanupEffect::NotRequested => "not_requested",
        CleanupEffect::Complete => "complete",
        CleanupEffect::Pending => "pending",
    }
}

#[test]
fn effects_not_requested_serializes_the_seven_rfc_fields() {
    assert_eq!(
        serde_json::to_value(Effects::not_requested()).unwrap(),
        json!({
            "write": "not_requested",
            "checkpoint": "not_requested",
            "discovery": "not_requested",
            "publication": "not_requested",
            "integration": "not_requested",
            "cleanup": "not_requested",
            "commit_oid": null,
        })
    );
}

#[test]
fn read_success_serializes_every_field_with_nulls_for_absent_values() {
    let envelope = Envelope::read_success("item list", Scope::default(), json!({"items": []}));
    let value = serde_json::to_value(&envelope).unwrap();

    assert_eq!(sorted(keys(&value)), sorted(ENVELOPE_FIELDS.to_vec()));
    assert_eq!(SCHEMA_VERSION, 1);
    assert_eq!(
        value,
        json!({
            "schema_version": 1,
            "command": "item list",
            "request_id": null,
            "operation_id": null,
            "outcome": "success",
            "code": "ok",
            "message": ResultCode::Ok.message(),
            "scope": {
                "repository": null,
                "item_id": null,
                "branch": null,
                "worktree": null,
                "remote": null,
            },
            "effects": serde_json::to_value(Effects::not_requested()).unwrap(),
            "data": {"items": []},
            "recovery": [],
        })
    );
}

#[test]
fn envelope_fields_serialize_in_the_rfc_order() {
    let envelope = Envelope::<Value>::failure(
        "item show",
        Scope::default(),
        ResultCode::InternalError,
        Vec::new(),
    );
    let text = serde_json::to_string(&envelope).unwrap();
    let positions: Vec<_> = ENVELOPE_FIELDS
        .iter()
        .map(|field| text.find(&format!("\"{field}\":")).unwrap())
        .collect();
    assert!(positions.is_sorted(), "{text}");
}

#[test]
fn failure_serializes_every_field_and_takes_its_outcome_from_the_code() {
    let scope = Scope {
        repository: Some("/projects/example".to_owned()),
        item_id: Some("01ARZ3NDEKTSV4RRFFQ69G5FAV".to_owned()),
        branch: Some("manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV".to_owned()),
        worktree: None,
        remote: Some("origin".to_owned()),
    };
    let mut arguments = serde_json::Map::new();
    arguments.insert("repository".to_owned(), json!("/projects/example"));
    let recovery = vec![
        RecoveryAction {
            action: "index.rebuild".to_owned(),
            operation_id: None,
            arguments,
        },
        RecoveryAction {
            action: "operation.resume".to_owned(),
            operation_id: Some("01ARZ3NDEKTSV4RRFFQ69G5FAW".to_owned()),
            arguments: serde_json::Map::new(),
        },
    ];

    let envelope =
        Envelope::<Value>::failure("item show", scope, ResultCode::IndexUnavailable, recovery);
    let value = serde_json::to_value(&envelope).unwrap();

    assert_eq!(sorted(keys(&value)), sorted(ENVELOPE_FIELDS.to_vec()));
    assert_eq!(
        value,
        json!({
            "schema_version": 1,
            "command": "item show",
            "request_id": null,
            "operation_id": null,
            "outcome": "blocked",
            "code": "index_unavailable",
            "message": ResultCode::IndexUnavailable.message(),
            "scope": {
                "repository": "/projects/example",
                "item_id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
                "branch": "manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV",
                "worktree": null,
                "remote": "origin",
            },
            "effects": serde_json::to_value(Effects::not_requested()).unwrap(),
            "data": null,
            "recovery": [
                {
                    "action": "index.rebuild",
                    "operation_id": null,
                    "arguments": {"repository": "/projects/example"},
                },
                {
                    "action": "operation.resume",
                    "operation_id": "01ARZ3NDEKTSV4RRFFQ69G5FAW",
                    "arguments": {},
                },
            ],
        })
    );

    for code in ResultCode::ALL {
        if code == ResultCode::Ok {
            continue;
        }
        let envelope = Envelope::<Value>::failure("item show", Scope::default(), code, Vec::new());
        assert_eq!(envelope.outcome, code.outcome());
        assert_eq!(envelope.message, code.message());
        let value = serde_json::to_value(&envelope).unwrap();
        assert_eq!(sorted(keys(&value)), sorted(ENVELOPE_FIELDS.to_vec()));
        assert_eq!(value["data"], Value::Null);
    }
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "a failure cannot carry the ok code")]
fn failure_rejects_the_ok_code() {
    let _ = Envelope::<Value>::failure("item show", Scope::default(), ResultCode::Ok, Vec::new());
}

/// Every code string the index stores, with the contract string it maps to.
const STORED_PROBLEM_CODES: [(&str, &str); 21] = [
    ("invalid-path", "invalid_path"),
    ("missing-front-matter", "missing_front_matter"),
    ("malformed-front-matter", "malformed_front_matter"),
    ("malformed-configuration", "malformed_configuration"),
    ("missing-field", "missing_field"),
    ("invalid-field", "invalid_field"),
    ("kind-path-mismatch", "kind_path_mismatch"),
    ("duplicate-id", "duplicate_id"),
    ("missing-comment-item", "missing_comment_item"),
    ("missing-parent", "missing_parent"),
    ("cross-item-parent", "cross_item_parent"),
    ("comment-cycle", "comment_cycle"),
    ("source", "source_unreadable"),
    ("context", "context_problem"),
    ("branch", "branch_problem"),
    ("retry-required", "retry_required"),
    ("relationship-wrong-type", "relationship_wrong_type"),
    ("relationship-invalid-id", "relationship_invalid_id"),
    ("relationship-self-reference", "relationship_self_reference"),
    ("duplicate-dependency", "duplicate_dependency"),
    ("invalid-slug", "invalid_slug"),
];

#[test]
fn stored_problem_codes_map_to_the_registry() {
    for (stored, expected) in STORED_PROBLEM_CODES {
        let code = ProblemCode::from_stored(stored);
        assert_eq!(code.as_str(), expected, "{stored}");
        assert_eq!(code.stored(), Some(stored));
    }
    let registered: BTreeSet<_> = ProblemCode::ALL
        .into_iter()
        .filter_map(ProblemCode::stored)
        .collect();
    assert_eq!(
        registered,
        STORED_PROBLEM_CODES
            .iter()
            .map(|(stored, _)| *stored)
            .collect()
    );
}

#[test]
fn unrecognized_stored_problem_codes_become_unknown_problem() {
    // The contract string of a code is not its stored string.
    for stored in [
        "",
        "future-code",
        "invalid_path",
        "unknown_problem",
        "Source",
    ] {
        assert_eq!(
            ProblemCode::from_stored(stored),
            ProblemCode::UnknownProblem,
            "{stored:?}"
        );
    }
}

#[test]
fn problem_codes_have_unique_snake_case_strings_and_fixed_guidance() {
    let names: Vec<_> = ProblemCode::ALL
        .into_iter()
        .map(ProblemCode::as_str)
        .collect();
    assert_eq!(
        names,
        [
            "invalid_path",
            "missing_front_matter",
            "malformed_front_matter",
            "malformed_configuration",
            "missing_field",
            "invalid_field",
            "kind_path_mismatch",
            "duplicate_id",
            "missing_comment_item",
            "missing_parent",
            "cross_item_parent",
            "comment_cycle",
            "source_unreadable",
            "context_problem",
            "branch_problem",
            "retry_required",
            "relationship_wrong_type",
            "relationship_invalid_id",
            "relationship_self_reference",
            "duplicate_dependency",
            "invalid_slug",
            "path_not_utf8",
            "metadata_not_representable",
            "relationship_not_a_ticket",
            "dependency_cycle",
            "parent_cycle",
            "unknown_problem",
        ]
    );
    assert_eq!(
        names.iter().collect::<BTreeSet<_>>().len(),
        ProblemCode::ALL.len()
    );
    for code in ProblemCode::ALL {
        let name = code.as_str();
        assert!(
            name.bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'),
            "{name}"
        );
        let guidance = code.guidance();
        assert!(guidance.ends_with('.') && guidance.is_ascii(), "{name}");
        assert_eq!(serde_json::to_value(code).unwrap(), json!(name));
    }
    assert_eq!(
        ProblemCode::RetryRequired.guidance(),
        "The repository changed while it was being observed; refresh the index again."
    );
}

/// Every failure code a key-material operation stores, with its contract
/// string.
const STORED_KEY_MATERIAL_FAILURES: [(&str, &str); 8] = [
    ("registry-unavailable", "registry_unavailable"),
    ("source-missing", "source_missing"),
    ("source-changed", "source_changed"),
    ("ownership-unverified", "ownership_unverified"),
    ("invalid-generated-key", "invalid_generated_key"),
    ("protection-unavailable", "protection_unavailable"),
    ("unsafe-path", "unsafe_path"),
    ("storage-unavailable", "storage_unavailable"),
];

#[test]
fn stored_key_material_failures_map_to_the_registry() {
    for (stored, expected) in STORED_KEY_MATERIAL_FAILURES {
        let code = OperationFailureCode::from_stored_key_material(stored);
        assert_eq!(code.as_str(), expected, "{stored}");
        assert_eq!(code.stored_key_material(), Some(stored));
    }
    let registered: BTreeSet<_> = OperationFailureCode::ALL
        .into_iter()
        .filter_map(OperationFailureCode::stored_key_material)
        .collect();
    assert_eq!(
        registered,
        STORED_KEY_MATERIAL_FAILURES
            .iter()
            .map(|(stored, _)| *stored)
            .collect()
    );
}

#[test]
fn unrecognized_stored_failures_become_unknown_failure() {
    // Neither a contract string nor a remote outcome is a stored
    // key-material code, and backend text is not a code at all.
    for stored in [
        "",
        "future-failure",
        "source_missing",
        "transport_unavailable",
        "unknown_failure",
        "No such file or directory (os error 2)",
    ] {
        assert_eq!(
            OperationFailureCode::from_stored_key_material(stored),
            OperationFailureCode::UnknownFailure,
            "{stored:?}"
        );
    }
}

#[test]
fn operation_failure_codes_have_unique_snake_case_strings() {
    let names: Vec<_> = OperationFailureCode::ALL
        .into_iter()
        .map(OperationFailureCode::as_str)
        .collect();
    assert_eq!(
        names,
        [
            "registry_unavailable",
            "source_missing",
            "source_changed",
            "ownership_unverified",
            "invalid_generated_key",
            "protection_unavailable",
            "unsafe_path",
            "storage_unavailable",
            "configuration_required",
            "selected_key_unavailable",
            "unlock_required",
            "host_approval_required",
            "transport_unavailable",
            "protocol_rejected",
            "cancelled",
            "repository_unavailable",
            "unknown_failure",
        ]
    );
    assert_eq!(
        names.iter().collect::<BTreeSet<_>>().len(),
        OperationFailureCode::ALL.len()
    );
    for code in OperationFailureCode::ALL {
        let name = code.as_str();
        assert!(
            name.bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte == b'_'),
            "{name}"
        );
        assert_eq!(serde_json::to_value(code).unwrap(), json!(name));
    }
}

#[test]
fn redact_url_removes_secrets_and_keeps_what_identifies_the_remote() {
    let cases = [
        // Password removed, SSH user kept.
        (
            "ssh://git:hunter2@example.com/team/repo.git",
            "ssh://git@example.com/team/repo.git",
        ),
        (
            "ssh://git@example.com:2222/team/repo.git",
            "ssh://git@example.com:2222/team/repo.git",
        ),
        (
            "SSH://git:hunter2@[::1]:2222/repo.git",
            "SSH://git@[::1]:2222/repo.git",
        ),
        (
            "ssh://example.com/team/repo.git",
            "ssh://example.com/team/repo.git",
        ),
        // HTTP user-info removed whole.
        (
            "https://alice:hunter2@example.com/team/repo.git",
            "https://example.com/team/repo.git",
        ),
        (
            "https://token@example.com/team/repo.git",
            "https://example.com/team/repo.git",
        ),
        (
            "http://alice:p@ss@example.com:8080/repo",
            "http://example.com:8080/repo",
        ),
        (
            "https://example.com/team/repo.git",
            "https://example.com/team/repo.git",
        ),
        ("https://alice:hunter2@example.com", "https://example.com"),
        // Query and fragment removed.
        (
            "https://example.com/repo.git?access_token=hunter2#hunter2",
            "https://example.com/repo.git",
        ),
        (
            "ssh://git@example.com/repo.git#hunter2?hunter2",
            "ssh://git@example.com/repo.git",
        ),
        ("https://example.com?token=hunter2", "https://example.com"),
        // A user name is kept only where it is known not to be a secret.
        (
            "ftp://alice:hunter2@example.com/repo",
            "ftp://example.com/repo",
        ),
        ("file:///srv/git/repo.git", "file:///srv/git/repo.git"),
        // A file URL is a local location, so its `#` and `?` are path text.
        ("file:///srv/repo#1", "file:///srv/repo#1"),
        ("file:///srv/a@b/repo?x", "file:///srv/a@b/repo?x"),
        ("ssh://git@host:22/p", "ssh://git@host:22/p"),
        (
            "ssh://git@example.com/team/repo@v2",
            "ssh://git@example.com/team/repo@v2",
        ),
        ("git@host:path", "git@host:path"),
        (
            "git@example.com:team/repo@v2",
            "git@example.com:team/repo@v2",
        ),
        (r"C:\path", r"C:\path"),
        ("C:/path", "C:/path"),
        ("//server/share/repo", "//server/share/repo"),
        ("relative/dir:with/colon@x", "relative/dir:with/colon@x"),
        // The scp-like form is kept as written.
        (
            "git@example.com:team/repo.git",
            "git@example.com:team/repo.git",
        ),
        ("example.com:team/repo.git", "example.com:team/repo.git"),
        // Local paths are unchanged.
        ("/srv/git/repo.git", "/srv/git/repo.git"),
        ("../sibling/repo.git", "../sibling/repo.git"),
        // A local path with a space cannot be told from a location with
        // arguments after it, and is replaced whole.
        ("repo with spaces", REDACTED),
        (
            r"C:\Users\alice@example\repo",
            r"C:\Users\alice@example\repo",
        ),
        ("C:/Users/alice@example/repo", "C:/Users/alice@example/repo"),
        ("/srv/a@b:c/repo?x#y", "/srv/a@b:c/repo?x#y"),
        // Unparseable input is replaced whole.
        ("https://alice:hunter2@/repo", REDACTED),
        ("https:///repo", REDACTED),
        ("://alice:hunter2@example.com/repo", REDACTED),
        ("1https://alice:hunter2@example.com/repo", REDACTED),
        ("https://alice:hunter2 @example.com/repo", REDACTED),
        ("https://alice:hunter2\n@example.com/repo", REDACTED),
        ("https://alice:hunter2/more@example.com/repo", REDACTED),
        ("https://example.com:hunter2/repo", REDACTED),
        ("https://[::1/repo", REDACTED),
        ("ssh://:hunter2@example.com/repo", REDACTED),
        ("alice:hunter2@example.com:team/repo.git", REDACTED),
        // No user-info was found, yet an `@` follows: the authority was cut
        // short by a character a credential should have escaped.
        ("https://hunter2/more@example.com/repo", REDACTED),
        ("https://alice:123/x@example.com/repo", REDACTED),
        ("https://alice:12345#abc@example.com/repo", REDACTED),
        ("https://alice:12345?abc@example.com/repo", REDACTED),
        ("https://exa$mple.com/repo", REDACTED),
        ("https://[not-an-address]/repo", REDACTED),
        // A file URL with an authority is treated as a network URL.
        (
            "file://alice:hunter2@example.com/repo",
            "file://example.com/repo",
        ),
        // Scheme-less forms that still carry a credential.
        ("a:hunter2@example.com:repo", REDACTED),
        ("c:hunter2@host/repo", REDACTED),
        ("https:/alice:hunter2@example.com/repo", REDACTED),
        ("//alice:hunter2@example.com/repo", REDACTED),
        ("alice@corp.com:hunter2@example.com:repo", REDACTED),
    ];
    for (input, expected) in cases {
        assert_eq!(redact_url(input), expected, "{input}");
    }
    assert_eq!(REDACTED, "[redacted]");
}

#[test]
fn redact_url_keeps_an_at_sign_after_the_authority_only_for_ssh() {
    let kept = "ssh://git@host/team/repo@v2";
    assert_eq!(redact_url(kept), kept);
    assert_eq!(
        redact_url("ssh://git:hunter2@host/team/repo@v2"),
        "ssh://git@host/team/repo@v2"
    );
    assert_eq!(redact_url("ssh://host/team/repo@v2"), REDACTED);

    for input in [
        "https://bob@x:123/hunter2@host/repo",
        "https://bob@host/team/repo@v2",
        "git+ssh://git@host/team/repo@v2",
    ] {
        assert_eq!(redact_url(input), REDACTED, "{input}");
    }
}

#[test]
fn redact_url_never_returns_the_secret() {
    let inputs = [
        "ssh://git:hunter2@example.com/team/repo.git",
        "https://alice:hunter2@example.com/team/repo.git",
        "https://hunter2@example.com/team/repo.git",
        "https://example.com/repo.git?token=hunter2",
        "https://example.com/repo.git#hunter2",
        "https://alice:hunter2/x@example.com/repo.git",
        "https://alice:hunter2@example.com:hunter2/repo.git",
        "weird+scheme://alice:hunter2@example.com/repo.git",
        "alice:hunter2@example.com:team/repo.git",
        "https://hunter2/more@example.com/repo",
        "https://alice:123/hunter2@example.com/repo",
        "https://alice:12345#hunter2@example.com/repo",
        "https://hunter2:12345#abc@example.com/repo",
        "https://alice:hunter2?abc@example.com/repo",
        "file://alice:hunter2@example.com/repo",
        "a:hunter2@example.com:repo",
        "c:hunter2@host/repo",
        "https:/alice:hunter2@example.com/repo",
        "//alice:hunter2@example.com/repo",
        "alice@corp.com:hunter2@example.com:repo",
        "https://bob@x:123/hunter2@host/repo",
        "https://bob@x/repo?hunter2@host",
    ];
    for input in inputs {
        let redacted = redact_url(input);
        assert!(!redacted.contains("hunter2"), "{input} became {redacted}");
        assert_eq!(redact_url(&redacted), redacted, "{input} is not stable");
    }
}

#[test]
fn timestamps_serialize_as_rfc3339_utc_to_the_second() {
    let instant = OffsetDateTime::from_unix_timestamp(1_780_000_000).unwrap();
    assert_eq!(
        timestamp_string(instant).as_deref(),
        Some("2026-05-28T20:26:40Z")
    );
    assert_eq!(
        timestamp_string(instant + Duration::nanoseconds(999_999_999)).as_deref(),
        Some("2026-05-28T20:26:40Z")
    );
    let offset = UtcOffset::from_hms(-7, 0, 0).unwrap();
    assert_eq!(
        timestamp_string(instant.to_offset(offset)).as_deref(),
        Some("2026-05-28T20:26:40Z")
    );
    assert_eq!(
        timestamp_string(OffsetDateTime::UNIX_EPOCH).as_deref(),
        Some("1970-01-01T00:00:00Z")
    );
    // RFC 3339 has no year before 0000.
    let before_year_zero = OffsetDateTime::from_unix_timestamp(-62_200_000_000).unwrap();
    assert_eq!(timestamp_string(before_year_zero), None);
}

#[test]
fn object_ids_serialize_as_full_lowercase_hex() {
    let hex = "0123456789abcdef0123456789abcdef01234567";
    let oid = git2::Oid::from_str(hex).unwrap();
    assert_eq!(object_id_string(oid), hex);
    assert_eq!(
        serde_json::to_value(object_id_string(oid)).unwrap(),
        json!(hex)
    );
}

#[test]
fn relative_paths_serialize_with_forward_slashes() {
    let platform = format!(".manyhands{MAIN_SEPARATOR}tickets{MAIN_SEPARATOR}one.md");
    assert_eq!(
        relative_path_string(Path::new(&platform)).as_deref(),
        Some(".manyhands/tickets/one.md")
    );
    let joined: PathBuf = ["docs", "plans", "a b.md"].iter().collect();
    assert_eq!(
        relative_path_string(&joined).as_deref(),
        Some("docs/plans/a b.md")
    );
    assert_eq!(
        relative_path_string(Path::new("./docs/./one.md")).as_deref(),
        Some("docs/one.md")
    );
    assert_eq!(
        relative_path_string(Path::new("one.md")).as_deref(),
        Some("one.md")
    );

    // An empty relative path names nothing.
    assert_eq!(relative_path_string(Path::new("")), None);
    assert_eq!(relative_path_string(Path::new(".")), None);
    assert_eq!(relative_path_string(Path::new("./")), None);

    // Not repository-relative: there is no honest string for these.
    assert_eq!(relative_path_string(Path::new("../outside.md")), None);
    assert_eq!(
        relative_path_string(Path::new("docs/../../outside.md")),
        None
    );
    let absolute = std::env::temp_dir();
    assert!(absolute.is_absolute());
    assert_eq!(relative_path_string(&absolute), None);
}

#[test]
fn absolute_paths_serialize_as_the_platform_reports_them() {
    let absolute = std::env::temp_dir().join("many hands").join("repo");
    assert_eq!(
        absolute_path_string(&absolute).as_deref(),
        Some(absolute.to_str().unwrap())
    );
}

#[cfg(unix)]
#[test]
fn non_utf8_paths_serialize_as_null() {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

    let name = OsStr::from_bytes(b"caf\xe9.md");
    let relative = Path::new("docs").join(name);
    let absolute = Path::new("/srv").join(name);

    assert_eq!(relative_path_string(&relative), None);
    assert_eq!(absolute_path_string(&absolute), None);
    assert_eq!(
        serde_json::to_value(relative_path_string(&relative)).unwrap(),
        Value::Null
    );
    assert_eq!(
        serde_json::to_value(absolute_path_string(&absolute)).unwrap(),
        Value::Null
    );
}

#[cfg(windows)]
#[test]
fn non_utf8_paths_serialize_as_null() {
    use std::{ffi::OsString, os::windows::ffi::OsStringExt};

    // An unpaired surrogate is a valid Windows file name and not valid UTF-8.
    let name = OsString::from_wide(&[0x0063, 0xD800, 0x002E, 0x006D, 0x0064]);
    let relative = Path::new("docs").join(&name);
    let absolute = Path::new(r"C:\srv").join(&name);

    assert_eq!(relative_path_string(&relative), None);
    assert_eq!(absolute_path_string(&absolute), None);
}

// Git runs a remote helper for `<transport>::<address>`, and the address is
// then whatever that helper takes: a command line, with its secrets.
#[test]
fn redact_url_replaces_a_remote_helper_location_whole() {
    for input in [
        "ext::git-remote-foo --token=SECRET %S",
        "ext::sh -c \"curl -H 'Authorization: Bearer X'\"",
        "ext::helper",
        "fd::7",
        "transport::address/with/a/path",
        "ext::ssh://git@example.com/repo.git",
    ] {
        assert_eq!(redact_url(input), REDACTED, "{input}");
    }
    // After a separator, `::` is path text.
    for kept in ["/srv/a::b/repo", "relative/a::b", "git@host:team/a::b"] {
        assert_eq!(redact_url(kept), kept);
    }
}

#[test]
fn redact_url_replaces_a_schemeless_location_with_whitespace_or_a_control_character() {
    for input in [
        "host:repo --upload-pack=SECRET",
        "git@example.com:team/repo.git\tSECRET",
        "/srv/git/repo\nSECRET",
        "repo\u{0}SECRET",
        "repo\u{7f}SECRET",
        "repo\u{a0}SECRET",
    ] {
        assert_eq!(redact_url(input), REDACTED, "{input:?}");
    }
}

#[test]
fn a_host_with_an_empty_port_is_not_a_host() {
    assert!(!super::is_host_port("host:"));
    assert!(!super::is_host_port("[::1]:"));
    assert!(super::is_host_port("host:22"));
    assert!(super::is_host_port("host"));
    assert!(super::is_host_port("[::1]"));
    assert_eq!(redact_url("https://example.com:/repo"), REDACTED);
    assert_eq!(redact_url("ssh://git@example.com:/repo"), REDACTED);
}
