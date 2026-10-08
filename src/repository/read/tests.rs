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
        assert_eq!(error.code(), code, "{kind:?}");
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
        assert_eq!(error.code(), code, "{kind:?}");
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
        assert_eq!(error.code(), ResultCode::InternalError);
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
fn caller_input_errors_are_named_constructors_not_a_conversion() {
    assert_eq!(ReadError::invalid_id().code(), ResultCode::InvalidId);
    assert_eq!(ReadError::invalid_path().code(), ResultCode::InvalidPath);
    for error in [ReadError::invalid_id(), ReadError::invalid_path()] {
        assert!(error.source().is_none());
        assert!(error.recovery.is_empty());
        assert_eq!(error.scope, Scope::default());
    }
}

#[test]
fn a_read_error_cannot_carry_the_ok_code() {
    assert_eq!(failure_code(ResultCode::Ok), ResultCode::InternalError);
    for code in ResultCode::ALL {
        if code != ResultCode::Ok {
            assert_eq!(failure_code(code), code);
            assert_eq!(ReadError::new(code).code(), code);
        }
    }
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "a read error cannot carry the ok code")]
fn constructing_a_read_error_with_the_ok_code_is_caught_in_debug_builds() {
    let _ = ReadError::new(ResultCode::Ok);
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

        assert_eq!(error.code(), code);
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
fn a_sqlite_failure_maps_the_same_way_raw_or_wrapped() {
    let failure = |code| rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(code), None);
    for (sqlite, code) in [
        (rusqlite::ffi::SQLITE_BUSY, ResultCode::Busy),
        (rusqlite::ffi::SQLITE_LOCKED, ResultCode::Busy),
        (rusqlite::ffi::SQLITE_NOTADB, ResultCode::IndexUnavailable),
        (rusqlite::ffi::SQLITE_CORRUPT, ResultCode::IndexUnavailable),
        (rusqlite::ffi::SQLITE_CANTOPEN, ResultCode::IndexUnavailable),
        (rusqlite::ffi::SQLITE_READONLY, ResultCode::InternalError),
    ] {
        let raw = ReadError::from(failure(sqlite));
        let wrapped = ReadError::from(RepositoryError::sqlite(failure(sqlite)));

        assert_eq!(raw.code(), code, "{sqlite}");
        assert_eq!(wrapped.code(), code, "{sqlite}");
        assert_eq!(raw.recovery, wrapped.recovery, "{sqlite}");
        assert_eq!(
            raw.recovery.len(),
            usize::from(code == ResultCode::IndexUnavailable)
        );
    }
    // Only the `Sqlite` kind is read through its source.
    let other_kind = RepositoryError::with_source(
        RepositoryOperation::Read,
        None,
        RepositoryErrorKind::Io,
        failure(rusqlite::ffi::SQLITE_NOTADB),
    );
    assert_eq!(
        ReadError::from(other_kind).code(),
        ResultCode::InternalError
    );
    assert_eq!(
        ReadError::from(rusqlite::Error::QueryReturnedNoRows).code(),
        ResultCode::InternalError
    );
}

#[test]
fn every_identity_level_has_its_contract_name() {
    use git2::ConfigLevel;

    let cases = [
        (ConfigLevel::Local, IdentitySource::Repository, "repository"),
        (ConfigLevel::Global, IdentitySource::Global, "global"),
        (ConfigLevel::XDG, IdentitySource::Xdg, "xdg"),
        (ConfigLevel::System, IdentitySource::System, "system"),
        (
            ConfigLevel::ProgramData,
            IdentitySource::ProgramData,
            "program_data",
        ),
        (ConfigLevel::App, IdentitySource::Application, "application"),
    ];
    for (level, source, name) in cases {
        assert_eq!(admin::identity_source(level), Some(source), "{level:?}");
        assert_eq!(source.as_str(), name);
    }
    // Levels identity resolution does not read are never named, and `none`
    // is kept for an identity that was not found.
    assert_eq!(admin::identity_source(ConfigLevel::Worktree), None);
    assert_eq!(admin::identity_source(ConfigLevel::Highest), None);
    assert_eq!(IdentitySource::ALL.len(), cases.len() + 1);
    assert_eq!(IdentitySource::None.as_str(), "none");
}

#[test]
fn a_failure_at_a_root_names_it_in_scope_and_in_the_rebuild_only() {
    let root = Path::new("/work/repository");
    let rebuild = serde_json::json!([{
        "action": "index.rebuild",
        "operation_id": null,
        "arguments": {"root": "/work/repository"},
    }]);

    let unavailable = resolve::at_root(ReadError::new(ResultCode::IndexUnavailable), root);
    assert_eq!(
        unavailable.scope.repository.as_deref(),
        Some("/work/repository")
    );
    assert_eq!(
        serde_json::to_value(&unavailable.recovery).unwrap(),
        rebuild
    );

    for code in ResultCode::ALL {
        if matches!(code, ResultCode::Ok | ResultCode::IndexUnavailable) {
            continue;
        }
        let error = resolve::at_root(ReadError::new(code), root);
        assert_eq!(error.code(), code);
        assert_eq!(error.scope.repository.as_deref(), Some("/work/repository"));
        assert!(error.recovery.is_empty(), "{code:?}");
    }

    // A scope the failure already names is the more exact one.
    let named = ReadError::new(ResultCode::Busy).with_scope(Scope {
        repository: Some("/work/other".to_owned()),
        item_id: Some("01ARZ3NDEKTSV4RRFFQ69G5FAV".to_owned()),
        ..Scope::default()
    });
    let named = resolve::at_root(named, root);
    assert_eq!(named.scope.repository.as_deref(), Some("/work/other"));
    assert_eq!(
        named.scope.item_id.as_deref(),
        Some("01ARZ3NDEKTSV4RRFFQ69G5FAV")
    );
}

#[cfg(unix)]
#[test]
fn a_root_that_is_not_utf8_is_left_out_instead_of_being_converted() {
    use std::os::unix::ffi::OsStrExt;

    let root = Path::new(std::ffi::OsStr::from_bytes(b"/work/\xff"));
    let error = resolve::at_root(ReadError::new(ResultCode::IndexUnavailable), root);

    assert_eq!(error.scope.repository, None);
    assert_eq!(
        serde_json::to_value(&error.recovery).unwrap(),
        serde_json::json!([{"action": "index.rebuild", "operation_id": null, "arguments": {}}])
    );
}

#[test]
fn a_stored_configuration_this_build_cannot_read_is_invalid_not_valid() {
    let unknown = |configuration: ConfigurationDto| {
        assert_eq!(configuration.state, ConfigurationState::Invalid);
        assert_eq!(configuration.primary_branch, None);
        assert_eq!(configuration.publication_remote, None);
        assert_eq!(
            configuration.problems,
            [ProblemDto {
                code: ProblemCode::UnknownProblem,
                path: Some(".manyhands/config.toml".to_owned()),
            }]
        );
    };

    unknown(admin::stored_configuration(
        Some("some-later-state"),
        Some("main".to_owned()),
        Some("origin".to_owned()),
        None,
    ));
    // Valid without a primary branch is not a configuration.
    unknown(admin::stored_configuration(
        Some("valid"),
        None,
        Some("origin".to_owned()),
        None,
    ));
    unknown(admin::stored_configuration(
        Some("invalid"),
        None,
        None,
        Some("some-later-code"),
    ));
    unknown(admin::stored_configuration(
        Some("invalid"),
        None,
        None,
        None,
    ));
}
