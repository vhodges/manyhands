use std::{
    fs,
    path::{Path, PathBuf},
};

use manyhands::repository::keys::{KeyMaterialAction, KeyMaterialError, KeyMaterialErrorKind};
use manyhands::repository::{
    GeneratedKeyDeletionPreflight, LeaseKind, OperationId, PrivateKeySourceState,
    PublicKeyMetadataState, REGISTRY_FILE, RegisterSharedKeyOutcome, RegisterSharedKeyRequest,
    RepositoryErrorKind, RepositoryOperation, RepositoryService, SharedKeyId, SharedKeyOwnership,
    SharedKeySelectionOutcome, UnregisterSharedKeyOutcome,
};

mod support;

#[cfg(unix)]
struct KillableChild {
    child: std::process::Child,
}

#[cfg(unix)]
impl Drop for KillableChild {
    fn drop(&mut self) {
        if self.child.try_wait().unwrap().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[test]
fn shared_key_registry_cache_lease_child() {
    let Ok(root) = std::env::var("MANYHANDS_LEASE_ROOT") else {
        return;
    };
    let data_directory = PathBuf::from(std::env::var("MANYHANDS_LEASE_DATA_DIRECTORY").unwrap());
    let kind = LeaseKind::parse(&std::env::var("MANYHANDS_LEASE_KIND").unwrap()).unwrap();
    let ready = PathBuf::from(std::env::var("MANYHANDS_LEASE_READY").unwrap());
    let release = PathBuf::from(std::env::var("MANYHANDS_LEASE_RELEASE").unwrap());
    let _holder = RepositoryService::hold_lease_for_testing(
        std::path::Path::new(&root),
        &data_directory,
        kind,
    )
    .unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(ready)
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !release.exists() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[cfg(unix)]
#[test]
fn shared_key_registry_fifo_child() {
    let (Ok(data_directory), Ok(private_key_path), Ok(public_key_path), Ok(ready)) = (
        std::env::var("MANYHANDS_FIFO_DATA_DIRECTORY"),
        std::env::var("MANYHANDS_FIFO_PRIVATE_KEY_PATH"),
        std::env::var("MANYHANDS_FIFO_PUBLIC_KEY_PATH"),
        std::env::var("MANYHANDS_FIFO_READY"),
    ) else {
        return;
    };
    let service = RepositoryService::open_at(std::path::Path::new(&data_directory)).unwrap();
    let outcome = service
        .register_shared_key(RegisterSharedKeyRequest {
            label: "FIFO companion".to_owned(),
            ownership: SharedKeyOwnership::Imported,
            private_key_path: PathBuf::from(private_key_path),
            public_key_path: Some(PathBuf::from(public_key_path)),
        })
        .unwrap();
    assert!(matches!(
        outcome,
        RegisterSharedKeyOutcome::Registered(registration)
            if registration.public_key_fingerprint.is_none()
                && registration.public_metadata_state == PublicKeyMetadataState::Unavailable
    ));
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(ready)
        .unwrap();
}

fn private_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/shared_key_registry_private_fixture")
}

const PUBLIC_KEY_COMMENT: &str = "manyhands-cycle01-distinctive-public-comment";
const PRIVATE_KEY_SENTINEL: &[u8] = b"CYCLE01_PRIVATE_SENTINEL_DO_NOT_PERSIST";
const PASSPHRASE_SENTINEL: &[u8] = b"CYCLE01_PASSPHRASE_SENTINEL_DO_NOT_PERSIST";
const PUBLIC_KEY_FINGERPRINT: &str = "SHA256:kmYcvdi2GkPeWxB6XLjrZB8JHsy2Hm8luHMFp9GMvqk";

fn public_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/shared_key_registry_public_fixture.pub")
}

fn assert_registry_storage_excludes(data_directory: &Path, forbidden: &[&[u8]]) {
    let database = fs::read(data_directory.join(REGISTRY_FILE)).unwrap();
    let wal = fs::read(data_directory.join(format!("{REGISTRY_FILE}-wal"))).unwrap();

    for contents in [&database, &wal] {
        for value in forbidden {
            assert!(
                !contents
                    .windows(value.len())
                    .any(|candidate| candidate == *value),
                "registry storage must not retain protected key material"
            );
        }
    }
}

fn fixture_derived_sentinel<'a>(fixture: &'a [u8], sentinel: &[u8]) -> &'a [u8] {
    fixture
        .windows(sentinel.len())
        .find(|candidate| *candidate == sentinel)
        .unwrap_or_else(|| panic!("private fixture must include its protected marker"))
}

fn hold_registry_read_transaction(data_directory: &Path) -> rusqlite::Connection {
    let connection = rusqlite::Connection::open(data_directory.join(REGISTRY_FILE)).unwrap();
    connection.execute_batch("BEGIN DEFERRED").unwrap();
    connection
        .query_row("SELECT COUNT(*) FROM shared_ssh_keys", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap();
    connection
}

fn registration_request(
    label: &str,
    ownership: SharedKeyOwnership,
    private_key_path: PathBuf,
) -> RegisterSharedKeyRequest {
    RegisterSharedKeyRequest {
        label: label.to_owned(),
        ownership,
        private_key_path,
        public_key_path: None,
    }
}

fn registry_table_columns(connection: &rusqlite::Connection, table: &str) -> Vec<String> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT name FROM pragma_table_info('{table}') ORDER BY cid"
        ))
        .unwrap();
    statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

#[test]
fn material_errors_expose_only_fixed_secret_free_guidance() {
    let key_id: manyhands::repository::keys::SharedKeyId = SharedKeyId::new();
    let error = KeyMaterialError {
        operation: KeyMaterialAction::Unlock,
        key_id: Some(key_id),
        operation_id: Some(OperationId::new()),
        kind: KeyMaterialErrorKind::UnlockFailed,
    };

    assert_eq!(
        error.guidance(),
        "the generated key could not be unlocked; verify the passphrase and try again"
    );
    assert_eq!(error.to_string(), error.guidance());
}

#[test]
fn material_schema_preserves_cycle01_rows_and_selection() {
    let data = tempfile::tempdir().unwrap();
    let registry_path = data.path().join(REGISTRY_FILE);
    let imported_id = SharedKeyId::new();
    let generated_id = SharedKeyId::new();
    let imported_path = data.path().join("imported-key");
    let generated_path = data.path().join("generated-key");
    let connection = rusqlite::Connection::open(&registry_path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE shared_ssh_keys (
                id TEXT PRIMARY KEY NOT NULL,
                label TEXT NOT NULL,
                ownership TEXT NOT NULL CHECK (ownership IN ('imported', 'generated')),
                private_key_path TEXT NOT NULL UNIQUE,
                public_key_path TEXT,
                public_key_fingerprint TEXT,
                private_source_state TEXT NOT NULL CHECK (
                    private_source_state IN ('available', 'missing', 'unavailable')
                ),
                public_metadata_state TEXT NOT NULL CHECK (
                    public_metadata_state IN ('not-provided', 'available', 'unavailable')
                ),
                selected INTEGER NOT NULL DEFAULT 0 CHECK (selected IN (0, 1))
            );
            CREATE UNIQUE INDEX shared_ssh_keys_one_selected_idx
                ON shared_ssh_keys(selected) WHERE selected = 1;",
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO shared_ssh_keys (
                id, label, ownership, private_key_path, public_key_path,
                public_key_fingerprint, private_source_state, public_metadata_state, selected
            ) VALUES (?1, 'Imported key', 'imported', ?2, NULL, NULL, 'missing', 'not-provided', 1)",
            rusqlite::params![imported_id.to_string(), imported_path.to_str().unwrap()],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO shared_ssh_keys (
                id, label, ownership, private_key_path, public_key_path,
                public_key_fingerprint, private_source_state, public_metadata_state, selected
            ) VALUES (?1, 'Generated key', 'generated', ?2, NULL, NULL, 'missing', 'not-provided', 0)",
            rusqlite::params![generated_id.to_string(), generated_path.to_str().unwrap()],
        )
        .unwrap();
    drop(connection);

    for _ in 0..2 {
        let service = RepositoryService::open_at(data.path()).unwrap();
        let registrations = service.list_shared_keys().unwrap();
        assert_eq!(registrations.len(), 2);
        assert_eq!(registrations[0].id, imported_id);
        assert_eq!(registrations[0].ownership, SharedKeyOwnership::Imported);
        assert!(registrations[0].selected);
        assert_eq!(registrations[1].id, generated_id);
        assert_eq!(registrations[1].ownership, SharedKeyOwnership::Generated);
        assert!(!registrations[1].selected);
    }

    let connection = rusqlite::Connection::open(registry_path).unwrap();
    for table in ["owned_generated_keys", "key_material_operations"] {
        assert!(
            connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                    [table],
                    |row| row.get::<_, bool>(0),
                )
                .unwrap(),
            "material migration must create {table}"
        );
    }
    assert_eq!(
        registry_table_columns(&connection, "owned_generated_keys"),
        [
            "key_id",
            "private_key_path",
            "public_key_path",
            "private_file_identity",
            "public_file_identity",
            "public_key_fingerprint",
        ]
    );
    assert_eq!(
        registry_table_columns(&connection, "key_material_operations"),
        [
            "operation_id",
            "key_id",
            "action",
            "generation_label",
            "private_key_path",
            "public_key_path",
            "private_file_identity",
            "public_file_identity",
            "public_key_fingerprint",
            "phase",
            "failure_code",
        ]
    );
    for table in ["owned_generated_keys", "key_material_operations"] {
        for column in registry_table_columns(&connection, table) {
            assert!(
                !column.contains("secret")
                    && !column.contains("passphrase")
                    && !column.contains("private_key_hash")
                    && !column.contains("confirmation"),
                "material schema must not persist protected data: {table}.{column}"
            );
        }
    }
}

#[test]
fn material_schema_is_idempotent_and_rejects_invalid_phases() {
    let data = tempfile::tempdir().unwrap();
    for _ in 0..2 {
        drop(RepositoryService::open_at(data.path()).unwrap());
    }
    let connection = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let key_id = SharedKeyId::new().to_string();
    let private_path = data.path().join("private-key");
    let public_path = data.path().join("private-key.pub");
    let insert = "INSERT INTO key_material_operations (
        operation_id, key_id, action, generation_label, private_key_path, public_key_path, phase
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)";

    for (action, phase) in [
        ("generate", "prepared"),
        ("delete", "reserved"),
        ("rotate", "reserved"),
        ("generate", "unknown"),
    ] {
        let result = connection.execute(
            insert,
            rusqlite::params![
                OperationId::new().to_string(),
                key_id,
                action,
                "Material key",
                private_path.to_str().unwrap(),
                public_path.to_str().unwrap(),
                phase,
            ],
        );
        assert!(result.is_err(), "{action}/{phase} must be rejected");
    }

    connection
        .execute(
            insert,
            rusqlite::params![
                OperationId::new().to_string(),
                key_id,
                "generate",
                "Material key",
                private_path.to_str().unwrap(),
                public_path.to_str().unwrap(),
                "reserved",
            ],
        )
        .unwrap();
    let duplicate_incomplete = connection.execute(
        insert,
        rusqlite::params![
            OperationId::new().to_string(),
            key_id,
            "delete",
            Option::<String>::None,
            private_path.to_str().unwrap(),
            public_path.to_str().unwrap(),
            "prepared",
        ],
    );
    assert!(
        duplicate_incomplete.is_err(),
        "only one incomplete material operation may exist for a key"
    );
}

#[test]
fn registration_generates_an_opaque_id_and_retains_private_metadata_without_mutating_the_fixture() {
    let data = tempfile::tempdir().unwrap();
    let fixture = private_fixture_path();
    let before = fs::read(&fixture).unwrap();
    let private_key_path = fixture
        .parent()
        .unwrap()
        .join("./shared_key_registry_private_fixture");
    let service = RepositoryService::open_at(data.path()).unwrap();

    let outcome = service
        .register_shared_key(registration_request(
            "Personal deployment key",
            SharedKeyOwnership::Generated,
            private_key_path,
        ))
        .unwrap();

    let RegisterSharedKeyOutcome::Registered(registration) = outcome else {
        panic!("the first registration must succeed");
    };
    let id = registration.id.to_string();
    assert_eq!(id.len(), 26);
    assert!(
        id.bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
    );
    assert_eq!(SharedKeyId::parse(&id).unwrap(), registration.id);
    assert_eq!(registration.label, "Personal deployment key");
    assert_eq!(registration.ownership, SharedKeyOwnership::Generated);
    assert_eq!(registration.private_key_path, fixture);
    assert_eq!(
        registration.private_source_state,
        PrivateKeySourceState::Available
    );
    assert_eq!(registration.public_key_path, None);
    assert_eq!(registration.public_key_fingerprint, None);
    assert_eq!(
        registration.public_metadata_state,
        PublicKeyMetadataState::NotProvided
    );
    assert!(!registration.selected);
    assert_eq!(fs::read(&fixture).unwrap(), before);
}

#[test]
fn registration_collides_for_dot_and_dot_dot_forms_of_the_same_private_source() {
    let data = tempfile::tempdir().unwrap();
    let fixture = private_fixture_path();
    let first_path = fixture
        .parent()
        .unwrap()
        .join("fixtures/../shared_key_registry_private_fixture");
    let second_path = fixture
        .parent()
        .unwrap()
        .join("./shared_key_registry_private_fixture");
    let service = RepositoryService::open_at(data.path()).unwrap();

    let RegisterSharedKeyOutcome::Registered(first) = service
        .register_shared_key(registration_request(
            "First label",
            SharedKeyOwnership::Imported,
            first_path,
        ))
        .unwrap()
    else {
        panic!("the first registration must succeed");
    };
    let second = service
        .register_shared_key(registration_request(
            "Second label",
            SharedKeyOwnership::Generated,
            second_path,
        ))
        .unwrap();

    assert_eq!(
        second,
        RegisterSharedKeyOutcome::SourceAlreadyRegistered { existing: first.id }
    );
    assert_eq!(service.list_shared_keys().unwrap(), vec![first]);
}

#[test]
fn registration_rejects_relative_private_paths_with_a_typed_source_path_error() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();

    let error = service
        .register_shared_key(registration_request(
            "Relative source",
            SharedKeyOwnership::Imported,
            PathBuf::from("relative/private-key"),
        ))
        .unwrap_err();

    assert_eq!(error.operation, RepositoryOperation::RegisterSharedKey);
    assert_eq!(error.kind, RepositoryErrorKind::InvalidSharedKeySourcePath);
    assert_eq!(error.root.as_deref(), Some(data.path()));
    assert!(!error.to_string().contains("relative/private-key"));
    assert!(service.list_shared_keys().unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn registration_rejects_non_unicode_private_path_components_before_lexical_normalization() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};

    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let private_key_path = PathBuf::from("/tmp")
        .join(OsString::from_vec(vec![0xff]))
        .join("..");

    let error = service
        .register_shared_key(registration_request(
            "Non-Unicode source",
            SharedKeyOwnership::Imported,
            private_key_path,
        ))
        .unwrap_err();

    assert_eq!(error.operation, RepositoryOperation::RegisterSharedKey);
    assert_eq!(error.kind, RepositoryErrorKind::InvalidSharedKeySourcePath);
    assert_eq!(error.root.as_deref(), Some(data.path()));
    assert!(service.list_shared_keys().unwrap().is_empty());
}

#[test]
fn registration_rejects_blank_labels_with_a_typed_metadata_error() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();

    let error = service
        .register_shared_key(registration_request(
            " \t\n",
            SharedKeyOwnership::Imported,
            private_fixture_path(),
        ))
        .unwrap_err();

    assert_eq!(error.operation, RepositoryOperation::RegisterSharedKey);
    assert_eq!(error.kind, RepositoryErrorKind::InvalidSharedKeyMetadata);
    assert_eq!(error.root.as_deref(), Some(data.path()));
}

#[test]
fn registration_rejects_relative_optional_public_paths_without_persisting() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();

    let error = service
        .register_shared_key(RegisterSharedKeyRequest {
            label: "Relative public source".to_owned(),
            ownership: SharedKeyOwnership::Imported,
            private_key_path: data.path().join("missing-private-key"),
            public_key_path: Some(PathBuf::from("relative-public-key.pub")),
        })
        .unwrap_err();

    assert_eq!(error.operation, RepositoryOperation::RegisterSharedKey);
    assert_eq!(error.kind, RepositoryErrorKind::InvalidSharedKeySourcePath);
    assert_eq!(error.root.as_deref(), Some(data.path()));
    assert!(!error.to_string().contains("relative-public-key.pub"));
    assert!(service.list_shared_keys().unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn registration_rejects_non_unicode_optional_public_paths_without_persisting() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};

    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let public_key_path = PathBuf::from("/tmp")
        .join(OsString::from_vec(vec![0xff]))
        .join("..");

    let error = service
        .register_shared_key(RegisterSharedKeyRequest {
            label: "Non-Unicode public source".to_owned(),
            ownership: SharedKeyOwnership::Imported,
            private_key_path: data.path().join("missing-private-key"),
            public_key_path: Some(public_key_path),
        })
        .unwrap_err();

    assert_eq!(error.operation, RepositoryOperation::RegisterSharedKey);
    assert_eq!(error.kind, RepositoryErrorKind::InvalidSharedKeySourcePath);
    assert_eq!(error.root.as_deref(), Some(data.path()));
    assert!(service.list_shared_keys().unwrap().is_empty());
}

#[test]
fn registration_allows_duplicate_labels_for_distinct_private_paths() {
    let data = tempfile::tempdir().unwrap();
    let sources = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();

    let RegisterSharedKeyOutcome::Registered(first) = service
        .register_shared_key(registration_request(
            "Shared label",
            SharedKeyOwnership::Imported,
            sources.path().join("missing-one"),
        ))
        .unwrap()
    else {
        panic!("the first registration must succeed");
    };
    let RegisterSharedKeyOutcome::Registered(second) = service
        .register_shared_key(registration_request(
            "Shared label",
            SharedKeyOwnership::Imported,
            sources.path().join("missing-two"),
        ))
        .unwrap()
    else {
        panic!("a distinct private source must register");
    };

    assert_ne!(first.id, second.id);
    assert_eq!(first.label, second.label);
    assert_eq!(first.private_source_state, PrivateKeySourceState::Missing);
    assert_eq!(second.private_source_state, PrivateKeySourceState::Missing);
    assert_eq!(service.list_shared_keys().unwrap(), vec![first, second]);
}

#[test]
fn recovery_missing_private_source_remains_registered_without_private_format_errors() {
    let data = tempfile::tempdir().unwrap();
    let sources = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();

    let RegisterSharedKeyOutcome::Registered(registration) = service
        .register_shared_key(registration_request(
            "Missing source",
            SharedKeyOwnership::Imported,
            sources.path().join("missing-private-key"),
        ))
        .unwrap()
    else {
        panic!("a missing private source must remain registerable");
    };

    assert_eq!(
        registration.private_source_state,
        PrivateKeySourceState::Missing
    );
    assert_eq!(service.list_shared_keys().unwrap(), vec![registration]);
}

#[test]
fn recovery_nonregular_private_source_remains_registered_without_private_format_errors() {
    let data = tempfile::tempdir().unwrap();
    let sources = tempfile::tempdir().unwrap();
    let private_key_path = sources.path().join("private-key-directory");
    fs::create_dir(&private_key_path).unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();

    let RegisterSharedKeyOutcome::Registered(registration) = service
        .register_shared_key(registration_request(
            "Nonregular source",
            SharedKeyOwnership::Imported,
            private_key_path,
        ))
        .unwrap()
    else {
        panic!("a nonregular private source must remain registerable");
    };

    assert_eq!(
        registration.private_source_state,
        PrivateKeySourceState::Unavailable
    );
    assert_eq!(service.list_shared_keys().unwrap(), vec![registration]);
}

#[cfg(unix)]
#[test]
fn registration_retains_an_inaccessible_private_source_as_unavailable_without_raw_errors() {
    use std::os::unix::fs::PermissionsExt;

    struct PermissionRestore {
        path: PathBuf,
        permissions: fs::Permissions,
    }

    impl Drop for PermissionRestore {
        fn drop(&mut self) {
            let _ = fs::set_permissions(&self.path, self.permissions.clone());
        }
    }

    let data = tempfile::tempdir().unwrap();
    let sources = tempfile::tempdir().unwrap();
    let blocked = sources.path().join("blocked");
    fs::create_dir(&blocked).unwrap();
    let _restore = PermissionRestore {
        path: blocked.clone(),
        permissions: fs::metadata(&blocked).unwrap().permissions(),
    };
    fs::set_permissions(&blocked, fs::Permissions::from_mode(0o000)).unwrap();
    let private_key_path = blocked.join("private-key");
    let metadata_error = match fs::metadata(&private_key_path) {
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => error,
        _ => {
            // Privileged processes can traverse a mode-000 directory, so this platform case skips.
            eprintln!("skipping inaccessible-source assertion because metadata remains accessible");
            return;
        }
    };
    let service = RepositoryService::open_at(data.path()).unwrap();

    let RegisterSharedKeyOutcome::Registered(registration) = service
        .register_shared_key(registration_request(
            "Inaccessible source",
            SharedKeyOwnership::Imported,
            private_key_path,
        ))
        .unwrap()
    else {
        panic!("an inaccessible private source must remain registerable");
    };

    assert_eq!(
        registration.private_source_state,
        PrivateKeySourceState::Unavailable
    );
    let listed = service.list_shared_keys().unwrap();
    assert_eq!(listed, vec![registration]);
    assert!(!format!("{listed:?}").contains(&metadata_error.to_string()));
}

#[test]
fn registration_and_listing_require_no_git_repository_at_the_application_data_directory() {
    let data = tempfile::tempdir().unwrap();
    assert!(!data.path().join(".git").exists());
    let service = RepositoryService::open_at(data.path()).unwrap();

    let RegisterSharedKeyOutcome::Registered(registration) = service
        .register_shared_key(registration_request(
            "Application-local registry",
            SharedKeyOwnership::Imported,
            private_fixture_path(),
        ))
        .unwrap()
    else {
        panic!("application-local registration must not require Git");
    };

    assert_eq!(service.list_shared_keys().unwrap(), vec![registration]);
    assert!(!data.path().join(".git").exists());
}

#[test]
fn registration_maps_an_unavailable_registry_to_a_typed_error() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let registry_path = data.path().join(REGISTRY_FILE);
    fs::remove_file(&registry_path).unwrap();
    fs::create_dir(&registry_path).unwrap();

    let error = service
        .register_shared_key(registration_request(
            "Unavailable registry",
            SharedKeyOwnership::Imported,
            data.path().join("missing-private-key"),
        ))
        .unwrap_err();

    assert_eq!(error.operation, RepositoryOperation::RegisterSharedKey);
    assert_eq!(
        error.kind,
        RepositoryErrorKind::SharedKeyRegistryUnavailable
    );
    assert_eq!(error.root.as_deref(), Some(data.path()));
    assert!(!error.to_string().contains("missing-private-key"));
}

#[test]
fn registration_preserves_a_busy_cache_write_error() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let holder = support::hold_lease_in_child_for_test(
        data.path(),
        data.path(),
        LeaseKind::CacheWrite,
        "shared_key_registry_cache_lease_child",
    );

    let error = service
        .register_shared_key(registration_request(
            "Busy registry",
            SharedKeyOwnership::Imported,
            data.path().join("missing-private-key"),
        ))
        .unwrap_err();

    assert_eq!(error.operation, RepositoryOperation::RegisterSharedKey);
    assert_eq!(error.kind, RepositoryErrorKind::RepositoryBusy);
    assert_eq!(error.root.as_deref(), Some(data.path()));
    holder.release();
}

#[test]
fn listing_maps_an_unavailable_registry_to_a_typed_error() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let registry_path = data.path().join(REGISTRY_FILE);
    fs::remove_file(&registry_path).unwrap();
    fs::create_dir(&registry_path).unwrap();

    let error = service.list_shared_keys().unwrap_err();

    assert_eq!(error.operation, RepositoryOperation::ListSharedKeys);
    assert_eq!(
        error.kind,
        RepositoryErrorKind::SharedKeyRegistryUnavailable
    );
    assert_eq!(error.root.as_deref(), Some(data.path()));
}

#[test]
fn listing_preserves_a_busy_cache_write_error() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let holder = support::hold_lease_in_child_for_test(
        data.path(),
        data.path(),
        LeaseKind::CacheWrite,
        "shared_key_registry_cache_lease_child",
    );

    let error = service.list_shared_keys().unwrap_err();

    assert_eq!(error.operation, RepositoryOperation::ListSharedKeys);
    assert_eq!(error.kind, RepositoryErrorKind::RepositoryBusy);
    assert_eq!(error.root.as_deref(), Some(data.path()));
    holder.release();
}

#[test]
fn registration_survives_reopen_and_remains_unselected() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let RegisterSharedKeyOutcome::Registered(registration) = service
        .register_shared_key(registration_request(
            "Durable key",
            SharedKeyOwnership::Imported,
            private_fixture_path(),
        ))
        .unwrap()
    else {
        panic!("the first registration must succeed");
    };
    drop(service);

    let reopened = RepositoryService::open_at(data.path()).unwrap();
    assert_eq!(reopened.list_shared_keys().unwrap(), vec![registration]);
    assert!(!reopened.list_shared_keys().unwrap()[0].selected);
}

#[test]
fn public_metadata_valid_companion_stores_only_a_fingerprint_without_secret_persistence() {
    let data = tempfile::tempdir().unwrap();
    let private_key_path = private_fixture_path();
    let private_fixture = fs::read(&private_key_path).unwrap();
    let private_key_sentinel = fixture_derived_sentinel(&private_fixture, PRIVATE_KEY_SENTINEL);
    let passphrase_sentinel = fixture_derived_sentinel(&private_fixture, PASSPHRASE_SENTINEL);
    let service = RepositoryService::open_at(data.path()).unwrap();
    let _wal_reader = hold_registry_read_transaction(data.path());
    let public_key_path = public_fixture_path();
    let public_key = fs::read_to_string(&public_key_path).unwrap();
    let public_key_body = public_key.split_whitespace().nth(1).unwrap();
    assert!(public_key.contains(PUBLIC_KEY_COMMENT));

    let RegisterSharedKeyOutcome::Registered(registration) = service
        .register_shared_key(RegisterSharedKeyRequest {
            label: "Public metadata deferred".to_owned(),
            ownership: SharedKeyOwnership::Imported,
            private_key_path,
            public_key_path: Some(public_key_path.clone()),
        })
        .unwrap()
    else {
        panic!("the first registration must succeed");
    };

    assert_eq!(registration.public_key_path, Some(public_key_path.clone()));
    assert_eq!(
        registration.public_key_fingerprint.as_deref(),
        Some(PUBLIC_KEY_FINGERPRINT)
    );
    assert_eq!(
        registration.public_metadata_state,
        PublicKeyMetadataState::FingerprintAvailable
    );
    let listed = service.list_shared_keys().unwrap();
    assert_eq!(listed, vec![registration]);
    assert!(!format!("{listed:?}").contains(public_key_body));
    assert!(!format!("{listed:?}").contains(PUBLIC_KEY_COMMENT));
    let stored = service
        .with_registry_connection_for_testing(|connection| {
            connection.query_row(
                "SELECT public_key_path, public_key_fingerprint FROM shared_ssh_keys",
                [],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                    ))
                },
            )
        })
        .unwrap()
        .unwrap();
    assert_eq!(stored.0.as_deref(), public_key_path.to_str());
    assert_eq!(stored.1.as_deref(), Some(PUBLIC_KEY_FINGERPRINT));
    assert_registry_storage_excludes(
        data.path(),
        &[
            private_key_sentinel,
            passphrase_sentinel,
            public_key_body.as_bytes(),
            PUBLIC_KEY_COMMENT.as_bytes(),
        ],
    );
}

#[test]
fn public_metadata_missing_and_malformed_companions_are_unavailable_without_parser_errors() {
    let data = tempfile::tempdir().unwrap();
    let sources = tempfile::tempdir().unwrap();
    let missing_public_key_path = sources.path().join("missing-public-key.pub");
    let malformed_public_key_path = sources.path().join("malformed-public-key.pub");
    let malformed_public_key = "CYCLE01_MALFORMED_PUBLIC_KEY_PARSE_TEXT";
    fs::write(&malformed_public_key_path, malformed_public_key).unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let _wal_reader = hold_registry_read_transaction(data.path());

    let RegisterSharedKeyOutcome::Registered(missing) = service
        .register_shared_key(RegisterSharedKeyRequest {
            label: "Missing companion".to_owned(),
            ownership: SharedKeyOwnership::Imported,
            private_key_path: sources.path().join("missing-private-key"),
            public_key_path: Some(missing_public_key_path.clone()),
        })
        .unwrap()
    else {
        panic!("a missing companion must remain registerable");
    };
    let RegisterSharedKeyOutcome::Registered(malformed) = service
        .register_shared_key(RegisterSharedKeyRequest {
            label: "Malformed companion".to_owned(),
            ownership: SharedKeyOwnership::Imported,
            private_key_path: sources.path().join("other-missing-private-key"),
            public_key_path: Some(malformed_public_key_path.clone()),
        })
        .unwrap()
    else {
        panic!("a malformed companion must remain registerable");
    };

    for (registration, public_key_path) in [
        (&missing, &missing_public_key_path),
        (&malformed, &malformed_public_key_path),
    ] {
        assert_eq!(registration.public_key_path, Some(public_key_path.clone()));
        assert_eq!(registration.public_key_fingerprint, None);
        assert_eq!(
            registration.public_metadata_state,
            PublicKeyMetadataState::Unavailable
        );
    }
    let listed = service.list_shared_keys().unwrap();
    assert_eq!(listed, vec![missing, malformed]);
    assert!(!format!("{listed:?}").contains(malformed_public_key));
    assert_registry_storage_excludes(data.path(), &[malformed_public_key.as_bytes()]);
}

#[test]
fn public_metadata_oversized_regular_companion_is_unavailable_without_parsing() {
    const OVERSIZED_PUBLIC_COMPANION_BYTES: usize = 64 * 1024;

    let data = tempfile::tempdir().unwrap();
    let sources = tempfile::tempdir().unwrap();
    let oversized_public_key_path = sources.path().join("oversized-public-key.pub");
    let public_key = fs::read_to_string(public_fixture_path()).unwrap();
    let mut fields = public_key.split_whitespace();
    let algorithm = fields.next().unwrap();
    let body = fields.next().unwrap();
    fs::write(
        &oversized_public_key_path,
        format!(
            "{algorithm} {body} {}\n",
            "x".repeat(OVERSIZED_PUBLIC_COMPANION_BYTES)
        ),
    )
    .unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();

    let RegisterSharedKeyOutcome::Registered(registration) = service
        .register_shared_key(RegisterSharedKeyRequest {
            label: "Oversized companion".to_owned(),
            ownership: SharedKeyOwnership::Imported,
            private_key_path: sources.path().join("missing-private-key"),
            public_key_path: Some(oversized_public_key_path.clone()),
        })
        .unwrap()
    else {
        panic!("an oversized companion must remain registerable");
    };

    assert_eq!(
        registration.public_key_path,
        Some(oversized_public_key_path)
    );
    assert_eq!(registration.public_key_fingerprint, None);
    assert_eq!(
        registration.public_metadata_state,
        PublicKeyMetadataState::Unavailable
    );
    assert_eq!(service.list_shared_keys().unwrap(), vec![registration]);
}

#[cfg(unix)]
#[test]
fn public_metadata_nonregular_companion_is_unavailable_without_parsing() {
    let data = tempfile::tempdir().unwrap();
    let sources = tempfile::tempdir().unwrap();
    let public_key_path = sources.path().join("public-key-directory");
    fs::create_dir(&public_key_path).unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();

    let RegisterSharedKeyOutcome::Registered(registration) = service
        .register_shared_key(RegisterSharedKeyRequest {
            label: "Nonregular companion".to_owned(),
            ownership: SharedKeyOwnership::Imported,
            private_key_path: sources.path().join("missing-private-key"),
            public_key_path: Some(public_key_path.clone()),
        })
        .unwrap()
    else {
        panic!("a nonregular companion must remain registerable");
    };

    assert_eq!(registration.public_key_path, Some(public_key_path));
    assert_eq!(registration.public_key_fingerprint, None);
    assert_eq!(
        registration.public_metadata_state,
        PublicKeyMetadataState::Unavailable
    );
    assert_eq!(service.list_shared_keys().unwrap(), vec![registration]);
}

#[cfg(unix)]
#[test]
fn public_metadata_fifo_companion_is_unavailable_without_hanging() {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};

    let data = tempfile::tempdir().unwrap();
    let sources = tempfile::tempdir().unwrap();
    let public_key_path = sources.path().join("public-key-fifo");
    let public_key_path_c = CString::new(public_key_path.as_os_str().as_bytes()).unwrap();
    // The path remains valid for the duration of the POSIX FIFO creation call.
    assert_eq!(
        unsafe { libc::mkfifo(public_key_path_c.as_ptr(), 0o600) },
        0
    );
    let ready = sources.path().join("ready");
    let started = std::time::Instant::now();
    let mut child = KillableChild {
        child: std::process::Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("shared_key_registry_fifo_child")
            .arg("--nocapture")
            .env("MANYHANDS_FIFO_DATA_DIRECTORY", data.path())
            .env(
                "MANYHANDS_FIFO_PRIVATE_KEY_PATH",
                sources.path().join("missing-private-key"),
            )
            .env("MANYHANDS_FIFO_PUBLIC_KEY_PATH", &public_key_path)
            .env("MANYHANDS_FIFO_READY", &ready)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap(),
    };
    let deadline = started + std::time::Duration::from_secs(2);
    while !ready.exists() {
        if child.child.try_wait().unwrap().is_some() {
            panic!("FIFO companion child exited before reporting completion");
        }
        assert!(
            std::time::Instant::now() < deadline,
            "FIFO companion child did not complete before its timeout"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(child.child.wait().unwrap().success());
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
}

#[test]
fn listing_rejects_an_available_public_fingerprint_without_a_companion_path() {
    let data = tempfile::tempdir().unwrap();
    let sources = tempfile::tempdir().unwrap();
    let private_key_path = sources.path().join("missing-private-key");
    let invalid_fingerprint = "SHA256:corrupted-public-fingerprint";
    let service = RepositoryService::open_at(data.path()).unwrap();

    service
        .with_registry_connection_for_testing(|connection| {
            connection.execute(
                "INSERT INTO shared_ssh_keys (
                    id, label, ownership, private_key_path, public_key_path,
                    public_key_fingerprint, private_source_state, public_metadata_state, selected
                ) VALUES (?1, ?2, 'imported', ?3, NULL, ?4, 'missing', 'available', 0)",
                rusqlite::params![
                    SharedKeyId::new().to_string(),
                    "Corrupt public metadata",
                    private_key_path.to_str().unwrap(),
                    invalid_fingerprint,
                ],
            )
        })
        .unwrap()
        .unwrap();

    let error = service.list_shared_keys().unwrap_err();
    assert_eq!(error.operation, RepositoryOperation::ListSharedKeys);
    assert_eq!(error.kind, RepositoryErrorKind::InvalidSharedKeyMetadata);
    assert_eq!(error.root.as_deref(), Some(data.path()));
    assert!(!error.to_string().contains(invalid_fingerprint));
    assert!(
        !error
            .to_string()
            .contains(private_key_path.to_str().unwrap())
    );
}

fn assert_available_public_row_is_rejected(label: &str, fingerprint: &str) {
    let data = tempfile::tempdir().unwrap();
    let sources = tempfile::tempdir().unwrap();
    let private_key_path = sources.path().join("missing-private-key");
    let public_key_path = sources.path().join("companion-public-key");
    let service = RepositoryService::open_at(data.path()).unwrap();

    service
        .with_registry_connection_for_testing(|connection| {
            connection.execute(
                "INSERT INTO shared_ssh_keys (
                    id, label, ownership, private_key_path, public_key_path,
                    public_key_fingerprint, private_source_state, public_metadata_state, selected
                ) VALUES (?1, ?2, 'imported', ?3, ?4, ?5, 'missing', 'available', 0)",
                rusqlite::params![
                    SharedKeyId::new().to_string(),
                    label,
                    private_key_path.to_str().unwrap(),
                    public_key_path.to_str().unwrap(),
                    fingerprint,
                ],
            )
        })
        .unwrap()
        .unwrap();

    let Err(error) = service.list_shared_keys() else {
        panic!("corrupt public metadata must be rejected");
    };
    assert_eq!(error.operation, RepositoryOperation::ListSharedKeys);
    assert_eq!(error.kind, RepositoryErrorKind::InvalidSharedKeyMetadata);
    assert_eq!(error.root.as_deref(), Some(data.path()));
}

#[test]
fn listing_rejects_an_available_public_fingerprint_with_a_blank_label() {
    assert_available_public_row_is_rejected(" \t\n", PUBLIC_KEY_FINGERPRINT);
}

#[test]
fn listing_rejects_an_available_public_fingerprint_that_is_empty() {
    assert_available_public_row_is_rejected("Corrupt fingerprint", "");
}

#[test]
fn listing_rejects_an_available_public_fingerprint_that_is_malformed() {
    assert_available_public_row_is_rejected("Corrupt fingerprint", "SHA256:not-a-fingerprint");
}

#[test]
fn generated_key_deletion_preflight_is_read_only_for_registered_keys() {
    let data = tempfile::tempdir().unwrap();
    let sources = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let RegisterSharedKeyOutcome::Registered(generated) = service
        .register_shared_key(registration_request(
            "Generated key",
            SharedKeyOwnership::Generated,
            sources.path().join("generated-private-key"),
        ))
        .unwrap()
    else {
        panic!("a generated key must register");
    };
    let RegisterSharedKeyOutcome::Registered(imported) = service
        .register_shared_key(registration_request(
            "Imported key",
            SharedKeyOwnership::Imported,
            sources.path().join("imported-private-key"),
        ))
        .unwrap()
    else {
        panic!("an imported key must register");
    };

    let operation_record_count = || {
        service
            .with_registry_connection_for_testing(|connection| {
                connection.query_row("SELECT COUNT(*) FROM operation_records", [], |row| {
                    row.get::<_, i64>(0)
                })
            })
            .unwrap()
            .unwrap()
    };
    let before = service.list_shared_keys().unwrap();
    let operation_records_before = operation_record_count();

    assert_eq!(
        service
            .preflight_generated_key_deletion(generated.id)
            .unwrap(),
        GeneratedKeyDeletionPreflight::ConfirmationRequired(generated.clone())
    );
    assert_eq!(service.list_shared_keys().unwrap(), before);
    assert_eq!(operation_record_count(), operation_records_before);
    assert_eq!(
        service
            .preflight_generated_key_deletion(imported.id)
            .unwrap(),
        GeneratedKeyDeletionPreflight::ImportedKey
    );
    assert_eq!(service.list_shared_keys().unwrap(), before);
    assert_eq!(operation_record_count(), operation_records_before);
    assert_eq!(
        service
            .preflight_generated_key_deletion(SharedKeyId::new())
            .unwrap(),
        GeneratedKeyDeletionPreflight::NotRegistered
    );
    assert_eq!(service.list_shared_keys().unwrap(), before);
    assert_eq!(operation_record_count(), operation_records_before);

    service.select_shared_key(generated.id).unwrap();
    let selected = service.list_shared_keys().unwrap();
    assert_eq!(
        service
            .preflight_generated_key_deletion(generated.id)
            .unwrap(),
        GeneratedKeyDeletionPreflight::SelectedKeyMustBeCleared
    );
    assert_eq!(service.list_shared_keys().unwrap(), selected);
    assert_eq!(operation_record_count(), operation_records_before);
}

#[test]
fn selection_replaces_the_selected_key_and_survives_reopen() {
    let data = tempfile::tempdir().unwrap();
    let sources = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let RegisterSharedKeyOutcome::Registered(first) = service
        .register_shared_key(registration_request(
            "First key",
            SharedKeyOwnership::Imported,
            private_fixture_path(),
        ))
        .unwrap()
    else {
        panic!("the first registration must succeed");
    };
    let RegisterSharedKeyOutcome::Registered(second) = service
        .register_shared_key(registration_request(
            "Second key",
            SharedKeyOwnership::Generated,
            sources.path().join("second-private-key"),
        ))
        .unwrap()
    else {
        panic!("the second registration must succeed");
    };

    let mut selected_first = first.clone();
    selected_first.selected = true;
    assert_eq!(
        service.select_shared_key(first.id).unwrap(),
        SharedKeySelectionOutcome::Selected(selected_first)
    );
    let mut selected_second = second.clone();
    selected_second.selected = true;
    assert_eq!(
        service.select_shared_key(second.id).unwrap(),
        SharedKeySelectionOutcome::Selected(selected_second)
    );
    drop(service);

    let reopened = RepositoryService::open_at(data.path()).unwrap();
    let selected = reopened
        .list_shared_keys()
        .unwrap()
        .into_iter()
        .filter(|registration| registration.selected)
        .map(|registration| registration.id)
        .collect::<Vec<_>>();
    assert_eq!(selected, vec![second.id]);
}

#[test]
fn selection_of_an_unknown_key_rolls_back_the_existing_selection() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let RegisterSharedKeyOutcome::Registered(first) = service
        .register_shared_key(registration_request(
            "First key",
            SharedKeyOwnership::Imported,
            private_fixture_path(),
        ))
        .unwrap()
    else {
        panic!("the registration must succeed");
    };
    service.select_shared_key(first.id).unwrap();

    let error = service.select_shared_key(SharedKeyId::new()).unwrap_err();

    assert_eq!(error.operation, RepositoryOperation::SelectSharedKey);
    assert_eq!(error.kind, RepositoryErrorKind::InvalidSharedKeyMetadata);
    assert_eq!(error.root.as_deref(), Some(data.path()));
    drop(service);

    let reopened = RepositoryService::open_at(data.path()).unwrap();
    let selected = reopened
        .list_shared_keys()
        .unwrap()
        .into_iter()
        .filter(|registration| registration.selected)
        .collect::<Vec<_>>();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].id, first.id);
}

#[test]
fn selection_rejects_a_second_selected_row_through_the_partial_unique_index() {
    let data = tempfile::tempdir().unwrap();
    let sources = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let RegisterSharedKeyOutcome::Registered(first) = service
        .register_shared_key(registration_request(
            "First key",
            SharedKeyOwnership::Imported,
            private_fixture_path(),
        ))
        .unwrap()
    else {
        panic!("the first registration must succeed");
    };
    let RegisterSharedKeyOutcome::Registered(second) = service
        .register_shared_key(registration_request(
            "Second key",
            SharedKeyOwnership::Generated,
            sources.path().join("second-private-key"),
        ))
        .unwrap()
    else {
        panic!("the second registration must succeed");
    };
    service.select_shared_key(first.id).unwrap();

    let raw_private_key_path = sources.path().join("raw-private-key");
    let insert = service
        .with_registry_connection_for_testing(|connection| {
            connection.execute(
                "INSERT INTO shared_ssh_keys (
                    id, label, ownership, private_key_path, public_key_path,
                    public_key_fingerprint, private_source_state, public_metadata_state, selected
                ) VALUES (?1, ?2, 'imported', ?3, NULL, NULL, 'missing', 'not-provided', 1)",
                rusqlite::params![
                    SharedKeyId::new().to_string(),
                    "Raw selected key",
                    raw_private_key_path.to_str().unwrap(),
                ],
            )
        })
        .unwrap();

    assert!(insert.is_err());
    let selected = service
        .list_shared_keys()
        .unwrap()
        .into_iter()
        .filter(|registration| registration.selected)
        .map(|registration| registration.id)
        .collect::<Vec<_>>();
    assert_eq!(selected, vec![first.id]);
    assert!(
        service
            .list_shared_keys()
            .unwrap()
            .into_iter()
            .any(|registration| registration.id == second.id)
    );
}

#[test]
fn clearing_selection_reports_whether_a_key_was_selected() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let RegisterSharedKeyOutcome::Registered(registration) = service
        .register_shared_key(registration_request(
            "Selected key",
            SharedKeyOwnership::Imported,
            private_fixture_path(),
        ))
        .unwrap()
    else {
        panic!("the registration must succeed");
    };
    service.select_shared_key(registration.id).unwrap();

    assert_eq!(
        service.clear_shared_key_selection().unwrap(),
        SharedKeySelectionOutcome::Cleared
    );
    assert!(
        !service.list_shared_keys().unwrap()[0].selected,
        "clearing must leave no selected key"
    );
    assert_eq!(
        service.clear_shared_key_selection().unwrap(),
        SharedKeySelectionOutcome::AlreadyCleared
    );
}

#[test]
fn unregistering_requires_clearing_the_selected_key_and_preserves_key_fixtures() {
    let data = tempfile::tempdir().unwrap();
    let sources = tempfile::tempdir().unwrap();
    let private_fixture = private_fixture_path();
    let public_fixture = public_fixture_path();
    let private_before = fs::read(&private_fixture).unwrap();
    let public_before = fs::read(&public_fixture).unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let RegisterSharedKeyOutcome::Registered(first) = service
        .register_shared_key(RegisterSharedKeyRequest {
            label: "Fixture key".to_owned(),
            ownership: SharedKeyOwnership::Imported,
            private_key_path: private_fixture.clone(),
            public_key_path: Some(public_fixture.clone()),
        })
        .unwrap()
    else {
        panic!("the first registration must succeed");
    };
    let RegisterSharedKeyOutcome::Registered(second) = service
        .register_shared_key(registration_request(
            "Second key",
            SharedKeyOwnership::Generated,
            sources.path().join("second-private-key"),
        ))
        .unwrap()
    else {
        panic!("the second registration must succeed");
    };
    service.select_shared_key(first.id).unwrap();

    assert_eq!(
        service.unregister_shared_key(first.id).unwrap(),
        UnregisterSharedKeyOutcome::SelectedKeyMustBeCleared
    );
    assert!(
        service
            .list_shared_keys()
            .unwrap()
            .into_iter()
            .any(|registration| registration.id == first.id)
    );
    assert_eq!(
        service.clear_shared_key_selection().unwrap(),
        SharedKeySelectionOutcome::Cleared
    );
    assert_eq!(
        service.unregister_shared_key(first.id).unwrap(),
        UnregisterSharedKeyOutcome::Unregistered
    );
    assert_eq!(service.list_shared_keys().unwrap(), vec![second]);
    assert_eq!(fs::read(&private_fixture).unwrap(), private_before);
    assert_eq!(fs::read(&public_fixture).unwrap(), public_before);
}

#[test]
fn unregistering_an_absent_key_reports_not_registered() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();

    assert_eq!(
        service.unregister_shared_key(SharedKeyId::new()).unwrap(),
        UnregisterSharedKeyOutcome::NotRegistered
    );
}
