use std::{error::Error, io, path::PathBuf};

use serde_json::Value;

use super::*;
use crate::{
    repository::{keys::KeyMaterialAction, validation_code_name},
    results::Outcome,
};

const SENTINEL: &str = "SENTINEL-7f3a";

fn serialized(error: &ReadError) -> String {
    serde_json::to_string(&error.to_envelope::<Value>("item show")).unwrap()
}

#[test]
fn repository_error_kinds_map_to_pinned_codes() {
    let cases = [
        (RepositoryErrorKind::InvalidPath, ResultCode::InvalidPath),
        (
            RepositoryErrorKind::InaccessibleRepository,
            ResultCode::RepositoryInaccessible,
        ),
        (
            RepositoryErrorKind::NotRepository,
            ResultCode::NotRepository,
        ),
        (
            RepositoryErrorKind::BareRepository,
            ResultCode::BareRepository,
        ),
        (
            RepositoryErrorKind::RepositoryNotRegistered,
            ResultCode::RepositoryNotRegistered,
        ),
        (
            RepositoryErrorKind::RepositoryNotEnabled,
            ResultCode::RepositoryNotRegistered,
        ),
        (
            RepositoryErrorKind::IndexUnavailable,
            ResultCode::IndexUnavailable,
        ),
        (RepositoryErrorKind::RepositoryBusy, ResultCode::Busy),
        (RepositoryErrorKind::Io, ResultCode::InternalError),
        (RepositoryErrorKind::Sqlite, ResultCode::InternalError),
        (RepositoryErrorKind::Git, ResultCode::InternalError),
        // Kinds no read produces are internal, never a guess.
        (
            RepositoryErrorKind::DirtyWorktree,
            ResultCode::InternalError,
        ),
        (
            RepositoryErrorKind::SharedKeyRegistryUnavailable,
            ResultCode::InternalError,
        ),
        (
            RepositoryErrorKind::RecoveryRequired,
            ResultCode::InternalError,
        ),
    ];
    for (kind, code) in cases {
        assert_eq!(repository_error_code(kind), code, "{kind:?}");
        let error = ReadError::from(RepositoryError::new(
            RepositoryOperation::Read,
            None,
            kind,
            "unused",
        ));
        assert_eq!(error.code, code, "{kind:?}");
    }
}

#[test]
fn key_material_error_kinds_map_to_pinned_codes() {
    let cases = [
        (KeyMaterialErrorKind::Busy, ResultCode::Busy),
        (KeyMaterialErrorKind::NotRegistered, ResultCode::KeyNotFound),
        (
            KeyMaterialErrorKind::RegistryUnavailable,
            ResultCode::InternalError,
        ),
        (
            KeyMaterialErrorKind::SourceUnreadable,
            ResultCode::InternalError,
        ),
        (
            KeyMaterialErrorKind::UnlockFailed,
            ResultCode::InternalError,
        ),
    ];
    for (kind, code) in cases {
        assert_eq!(key_material_error_code(kind), code, "{kind:?}");
        let error = ReadError::from(KeyMaterialError {
            operation: KeyMaterialAction::ListRecovery,
            key_id: None,
            operation_id: None,
            kind,
        });
        assert_eq!(error.code, code, "{kind:?}");
        assert!(error.source().is_some());
    }
}

#[test]
fn ssh_transport_error_kinds_are_internal() {
    for kind in [
        SshTransportErrorKind::ConfigurationInvalid,
        SshTransportErrorKind::NoSelectedKey,
        SshTransportErrorKind::KeyMissing,
        SshTransportErrorKind::HostTrustChanged,
        SshTransportErrorKind::RegistryUnavailable,
        SshTransportErrorKind::RuntimeUninitialized,
        SshTransportErrorKind::RemoteUnavailable,
    ] {
        assert_eq!(
            ssh_transport_error_code(&kind),
            ResultCode::InternalError,
            "{kind:?}"
        );
        let error = ReadError::from(kind);
        assert_eq!(error.code, ResultCode::InternalError);
        assert!(error.source().is_none());
    }
}

const VALIDATION_CODES: [canonical::ValidationCode; 12] = [
    canonical::ValidationCode::InvalidPath,
    canonical::ValidationCode::MissingFrontMatter,
    canonical::ValidationCode::MalformedFrontMatter,
    canonical::ValidationCode::MalformedConfiguration,
    canonical::ValidationCode::MissingField,
    canonical::ValidationCode::InvalidField,
    canonical::ValidationCode::KindPathMismatch,
    canonical::ValidationCode::DuplicateId,
    canonical::ValidationCode::MissingCommentItem,
    canonical::ValidationCode::MissingParent,
    canonical::ValidationCode::CrossItemParent,
    canonical::ValidationCode::CommentCycle,
];

#[test]
fn validation_problems_map_to_pinned_codes() {
    for code in VALIDATION_CODES {
        let expected = match code {
            canonical::ValidationCode::InvalidPath => ResultCode::InvalidPath,
            canonical::ValidationCode::InvalidField => ResultCode::InvalidId,
            _ => ResultCode::InternalError,
        };
        let error = ReadError::from(canonical::ValidationProblem {
            path: PathBuf::from(SENTINEL),
            code: code.clone(),
            message: SENTINEL.to_owned(),
        });
        assert_eq!(error.code, expected, "{code:?}");
        assert!(error.source().is_none());
        assert!(!serialized(&error).contains(SENTINEL));
    }

    let problem = "not a ulid".parse::<canonical::ItemId>().unwrap_err();
    assert_eq!(ReadError::from(problem).code, ResultCode::InvalidId);
}

#[test]
fn every_validation_code_is_stored_under_a_string_the_registry_maps_back() {
    for code in VALIDATION_CODES {
        let stored = validation_code_name(code.clone());
        let problem = ProblemCode::from(&code);
        assert_eq!(ProblemCode::from_stored(stored), problem, "{stored}");
        assert_eq!(problem.as_str(), stored.replace('-', "_"));
    }
}

#[test]
fn problems_serialize_guidance_from_the_registry() {
    for code in ProblemCode::ALL {
        let problem = ProblemDto {
            code,
            path: Some("docs/a.md".to_owned()),
        };
        assert_eq!(
            serde_json::to_value(&problem).unwrap(),
            serde_json::json!({
                "code": code.as_str(),
                "path": "docs/a.md",
                "guidance": code.guidance(),
            })
        );
    }
    let stored = ProblemDto {
        code: ProblemCode::from_stored("some-later-code"),
        path: None,
    };
    assert_eq!(
        serde_json::to_value(&stored).unwrap(),
        serde_json::json!({
            "code": "unknown_problem",
            "path": null,
            "guidance": ProblemCode::UnknownProblem.guidance(),
        })
    );
}

#[test]
fn a_read_error_never_shows_what_the_repository_error_carried() {
    let root = PathBuf::from(format!("/work/{SENTINEL}/repository"));
    let with_source = RepositoryError::with_source(
        RepositoryOperation::Read,
        Some(root.clone()),
        RepositoryErrorKind::Io,
        io::Error::other(format!("backend said {SENTINEL}")),
    );
    let with_message = RepositoryError::new(
        RepositoryOperation::Read,
        Some(root),
        RepositoryErrorKind::RepositoryNotRegistered,
        format!("message with {SENTINEL}"),
    );
    // The planted text is really there, so the scan below can fail.
    assert!(with_source.to_string().contains(SENTINEL));
    assert!(with_message.to_string().contains(SENTINEL));

    for (repository_error, code) in [
        (with_source, ResultCode::InternalError),
        (with_message, ResultCode::RepositoryNotRegistered),
    ] {
        let error = ReadError::from(repository_error);

        assert_eq!(error.code, code);
        assert_eq!(error.to_string(), code.message());
        assert!(!format!("{error:?}").contains(SENTINEL));
        assert!(!serialized(&error).contains(SENTINEL));
        // The source is kept for a front end that logs in process.
        let source = error.source().unwrap();
        assert!(source.to_string().contains(SENTINEL));
    }
}

#[test]
fn a_read_error_becomes_a_failure_envelope_with_its_scope_and_recovery() {
    let scope = Scope {
        repository: Some("/work/repository".to_owned()),
        ..Scope::default()
    };
    let error = ReadError::new(ResultCode::IndexUnavailable).with_scope(scope.clone());
    let envelope = error.to_envelope::<Value>("item list");

    assert_eq!(envelope.command, "item list");
    assert_eq!(envelope.outcome, Outcome::Blocked);
    assert_eq!(envelope.code, ResultCode::IndexUnavailable);
    assert_eq!(envelope.message, ResultCode::IndexUnavailable.message());
    assert_eq!(envelope.scope, scope);
    assert_eq!(envelope.data, None);
    assert_eq!(
        serde_json::to_value(&envelope.recovery).unwrap(),
        serde_json::json!([{"action": "index.rebuild", "operation_id": null, "arguments": {}}])
    );
    assert!(ReadError::new(ResultCode::Busy).recovery.is_empty());
}

#[test]
fn index_connection_failures_keep_their_sqlite_meaning() {
    let failure = |code| rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(code), None);
    for (sqlite, code) in [
        (rusqlite::ffi::SQLITE_BUSY, ResultCode::Busy),
        (rusqlite::ffi::SQLITE_LOCKED, ResultCode::Busy),
        (rusqlite::ffi::SQLITE_NOTADB, ResultCode::IndexUnavailable),
        (rusqlite::ffi::SQLITE_CORRUPT, ResultCode::IndexUnavailable),
        (rusqlite::ffi::SQLITE_READONLY, ResultCode::InternalError),
    ] {
        assert_eq!(ReadError::from(failure(sqlite)).code, code, "{sqlite}");
    }
    assert_eq!(
        ReadError::from(rusqlite::Error::QueryReturnedNoRows).code,
        ResultCode::InternalError
    );
}
