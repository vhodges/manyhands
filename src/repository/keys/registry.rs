#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    ffi::OsString,
    io::Read,
    path::{Component, Path, PathBuf},
};

use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};

use super::super::{
    IndexAvailability, ReadError, RepositoryError, RepositoryErrorKind, RepositoryOperation,
    RepositoryService, cache_read_guard, cache_write_guard, migrate_registry, open_registry,
    open_registry_read_only,
    recovery::{FinalKind, JournalRow, PendingOperation},
};
use super::{
    GeneratedKeyDeletionPreflight, KeyMaterialPhase, PrivateKeySourceState, PublicKeyMetadataState,
    RegisterSharedKeyOutcome, RegisterSharedKeyRequest, SharedKeyId, SharedKeyOwnership,
    SharedKeyRegistration, SharedKeySelectionOutcome, UnregisterSharedKeyOutcome,
};

const MAX_OPENSSH_PUBLIC_KEY_FILE_BYTES: u64 = 16 * 1024;
/// One byte more than can be read, so the buffer is never full and never
/// grows.
const PUBLIC_KEY_BUFFER_CAPACITY: usize = MAX_OPENSSH_PUBLIC_KEY_FILE_BYTES as usize + 1;

impl RepositoryService {
    pub fn register_shared_key(
        &self,
        request: RegisterSharedKeyRequest,
    ) -> Result<RegisterSharedKeyOutcome, RepositoryError> {
        let operation = RepositoryOperation::RegisterSharedKey;
        let data_directory = self.registry_data_directory();
        if request.label.trim().is_empty() {
            return Err(invalid_shared_key_metadata(operation, data_directory));
        }
        let (private_key_path, private_key_path_value) =
            normalize_shared_key_path(&request.private_key_path, operation, data_directory)?;
        let (public_key_path, public_key_path_value) = request
            .public_key_path
            .as_deref()
            .map(|path| normalize_shared_key_path(path, operation, data_directory))
            .transpose()?
            .map_or((None, None), |(path, value)| (Some(path), Some(value)));
        let private_source_state = private_key_source_state(&private_key_path);
        let (public_key_fingerprint, public_metadata_state) =
            public_key_metadata(public_key_path.as_deref());

        self.require_shared_key_registry(operation)?;
        let _cache_guard = cache_write_guard(&self.registry_path, data_directory, operation)?;
        let mut connection = open_registry(&self.registry_path, &mut |_| {})
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        migrate_registry(&mut connection)
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        let existing: Option<String> = transaction
            .query_row(
                "SELECT id FROM shared_ssh_keys WHERE private_key_path = ?1",
                [&private_key_path_value],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        if let Some(existing) = existing {
            let existing = SharedKeyId::parse(&existing)
                .map_err(|_| invalid_shared_key_metadata(operation, data_directory))?;
            transaction
                .commit()
                .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
            return Ok(RegisterSharedKeyOutcome::SourceAlreadyRegistered { existing });
        }

        let registration = SharedKeyRegistration {
            id: SharedKeyId::new(),
            label: request.label,
            ownership: request.ownership,
            private_key_path,
            public_key_path,
            public_key_fingerprint,
            private_source_state,
            public_metadata_state,
            selected: false,
        };
        transaction
            .execute(
                "INSERT INTO shared_ssh_keys (
                    id, label, ownership, private_key_path, public_key_path,
                    public_key_fingerprint, private_source_state, public_metadata_state, selected
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0)",
                params![
                    registration.id.to_string(),
                    registration.label,
                    shared_key_ownership_value(registration.ownership),
                    private_key_path_value,
                    public_key_path_value,
                    registration.public_key_fingerprint,
                    private_key_source_state_value(registration.private_source_state),
                    public_key_metadata_state_value(registration.public_metadata_state),
                ],
            )
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        transaction
            .commit()
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        Ok(RegisterSharedKeyOutcome::Registered(registration))
    }

    pub fn list_shared_keys(&self) -> Result<Vec<SharedKeyRegistration>, RepositoryError> {
        let operation = RepositoryOperation::ListSharedKeys;
        let data_directory = self.registry_data_directory();
        self.require_shared_key_registry(operation)?;
        let _cache_guard = cache_write_guard(&self.registry_path, data_directory, operation)?;
        let mut connection = open_registry(&self.registry_path, &mut |_| {})
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        migrate_registry(&mut connection)
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        let registrations = read_shared_key_registrations(&transaction, operation, data_directory)?;
        transaction
            .commit()
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        Ok(registrations)
    }

    pub fn preflight_generated_key_deletion(
        &self,
        id: SharedKeyId,
    ) -> Result<GeneratedKeyDeletionPreflight, RepositoryError> {
        let operation = RepositoryOperation::PreflightGeneratedKeyDeletion;
        let data_directory = self.registry_data_directory();
        self.require_shared_key_registry(operation)?;
        let _cache_guard = cache_read_guard(&self.registry_path, data_directory, operation)?;
        let connection = open_registry_read_only(&self.registry_path)
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        let registration = read_shared_key_registrations(&connection, operation, data_directory)?
            .into_iter()
            .find(|registration| registration.id == id);
        Ok(match registration {
            None => GeneratedKeyDeletionPreflight::NotRegistered,
            Some(registration) if registration.ownership == SharedKeyOwnership::Imported => {
                GeneratedKeyDeletionPreflight::ImportedKey
            }
            Some(registration) if registration.selected => {
                GeneratedKeyDeletionPreflight::SelectedKeyMustBeCleared
            }
            Some(registration) => GeneratedKeyDeletionPreflight::ConfirmationRequired(registration),
        })
    }

    pub fn select_shared_key(
        &self,
        id: SharedKeyId,
    ) -> Result<SharedKeySelectionOutcome, RepositoryError> {
        let operation = RepositoryOperation::SelectSharedKey;
        let data_directory = self.registry_data_directory();
        self.require_shared_key_registry(operation)?;
        let _cache_guard = cache_write_guard(&self.registry_path, data_directory, operation)?;
        let mut connection = open_registry(&self.registry_path, &mut |_| {})
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        migrate_registry(&mut connection)
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        require_no_pending_material(&transaction, id, operation, data_directory)?;
        let already_selected: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM shared_ssh_keys WHERE id=?1 AND selected=1)",
                [id.to_string()],
                |row| row.get(0),
            )
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        transaction
            .execute(
                "UPDATE shared_ssh_keys SET selected = 0 WHERE selected = 1",
                [],
            )
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        if transaction
            .execute(
                "UPDATE shared_ssh_keys SET selected = 1 WHERE id = ?1",
                [id.to_string()],
            )
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?
            == 0
        {
            transaction
                .rollback()
                .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
            return Err(invalid_shared_key_metadata(operation, data_directory));
        }
        let registration = read_shared_key_registrations(&transaction, operation, data_directory)?
            .into_iter()
            .find(|registration| registration.id == id)
            .ok_or_else(|| invalid_shared_key_metadata(operation, data_directory))?;
        if !already_selected {
            super::super::remote::state::invalidate_key_selection(&transaction)?;
        }
        transaction
            .commit()
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        Ok(SharedKeySelectionOutcome::Selected(registration))
    }

    pub fn clear_shared_key_selection(&self) -> Result<SharedKeySelectionOutcome, RepositoryError> {
        let operation = RepositoryOperation::ClearSharedKeySelection;
        let data_directory = self.registry_data_directory();
        self.require_shared_key_registry(operation)?;
        let _cache_guard = cache_write_guard(&self.registry_path, data_directory, operation)?;
        let mut connection = open_registry(&self.registry_path, &mut |_| {})
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        migrate_registry(&mut connection)
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        let cleared = transaction
            .execute(
                "UPDATE shared_ssh_keys SET selected = 0 WHERE selected = 1",
                [],
            )
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        if cleared != 0 {
            super::super::remote::state::invalidate_key_selection(&transaction)?;
        }
        transaction
            .commit()
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        Ok(if cleared == 0 {
            SharedKeySelectionOutcome::AlreadyCleared
        } else {
            SharedKeySelectionOutcome::Cleared
        })
    }

    pub fn unregister_shared_key(
        &self,
        id: SharedKeyId,
    ) -> Result<UnregisterSharedKeyOutcome, RepositoryError> {
        let operation = RepositoryOperation::UnregisterSharedKey;
        let data_directory = self.registry_data_directory();
        self.require_shared_key_registry(operation)?;
        let _cache_guard = cache_write_guard(&self.registry_path, data_directory, operation)?;
        let mut connection = open_registry(&self.registry_path, &mut |_| {})
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        migrate_registry(&mut connection)
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        require_no_pending_material(&transaction, id, operation, data_directory)?;
        let selected = transaction
            .query_row(
                "SELECT selected FROM shared_ssh_keys WHERE id = ?1",
                [id.to_string()],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        let outcome = match selected {
            None => UnregisterSharedKeyOutcome::NotRegistered,
            Some(1) => UnregisterSharedKeyOutcome::SelectedKeyMustBeCleared,
            Some(0) => {
                transaction
                    .execute(
                        "DELETE FROM shared_ssh_keys WHERE id = ?1",
                        [id.to_string()],
                    )
                    .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
                UnregisterSharedKeyOutcome::Unregistered
            }
            Some(_) => return Err(invalid_shared_key_metadata(operation, data_directory)),
        };
        transaction
            .commit()
            .map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
        Ok(outcome)
    }

    pub(super) fn registry_data_directory(&self) -> &Path {
        self.registry_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
    }

    pub(super) fn require_shared_key_registry(
        &self,
        operation: RepositoryOperation,
    ) -> Result<(), RepositoryError> {
        self.availability
            .lock()
            .map_err(|_| shared_key_registry_unavailable(operation, self.registry_data_directory()))
            .and_then(|availability| {
                matches!(*availability, IndexAvailability::Ready)
                    .then_some(())
                    .ok_or_else(|| {
                        shared_key_registry_unavailable(operation, self.registry_data_directory())
                    })
            })
    }
}

fn invalid_shared_key_metadata(
    operation: RepositoryOperation,
    data_directory: &Path,
) -> RepositoryError {
    RepositoryError::new(
        operation,
        Some(data_directory.to_owned()),
        RepositoryErrorKind::InvalidSharedKeyMetadata,
        "the shared key metadata is invalid",
    )
}

fn invalid_shared_key_source_path(
    operation: RepositoryOperation,
    data_directory: &Path,
) -> RepositoryError {
    RepositoryError::new(
        operation,
        Some(data_directory.to_owned()),
        RepositoryErrorKind::InvalidSharedKeySourcePath,
        "the shared key source path is invalid",
    )
}

fn shared_key_registry_unavailable(
    operation: RepositoryOperation,
    data_directory: &Path,
) -> RepositoryError {
    RepositoryError::new(
        operation,
        Some(data_directory.to_owned()),
        RepositoryErrorKind::SharedKeyRegistryUnavailable,
        "the shared key registry is unavailable",
    )
}

fn normalize_shared_key_path(
    path: &Path,
    operation: RepositoryOperation,
    data_directory: &Path,
) -> Result<(PathBuf, String), RepositoryError> {
    if !path.is_absolute() || path.to_str().is_none() {
        return Err(invalid_shared_key_source_path(operation, data_directory));
    }

    let mut normalized = PathBuf::new();
    let mut segments = Vec::<OsString>::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                segments.pop();
            }
            Component::Normal(segment) => segments.push(segment.to_owned()),
        }
    }
    for segment in segments {
        normalized.push(segment);
    }
    let value = normalized
        .to_str()
        .map(str::to_owned)
        .ok_or_else(|| invalid_shared_key_source_path(operation, data_directory))?;
    Ok((normalized, value))
}

fn private_key_source_state(path: &Path) -> PrivateKeySourceState {
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => PrivateKeySourceState::Available,
        Ok(_) => PrivateKeySourceState::Unavailable,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            PrivateKeySourceState::Missing
        }
        Err(_) => PrivateKeySourceState::Unavailable,
    }
}

fn public_key_metadata(path: Option<&Path>) -> (Option<String>, PublicKeyMetadataState) {
    let Some(path) = path else {
        return (None, PublicKeyMetadataState::NotProvided);
    };
    let Some(contents) = bounded_public_key_contents(path) else {
        return (None, PublicKeyMetadataState::Unavailable);
    };
    let Ok(contents) = std::str::from_utf8(&contents) else {
        return (None, PublicKeyMetadataState::Unavailable);
    };
    match openssh_public_key(contents) {
        Some(public_key) => (
            Some(public_key_fingerprint(&public_key)),
            PublicKeyMetadataState::FingerprintAvailable,
        ),
        None => (None, PublicKeyMetadataState::Unavailable),
    }
}

/// The key in OpenSSH public key text, or `None` when the text is not one.
pub(in super::super) fn openssh_public_key(contents: &str) -> Option<ssh_key::PublicKey> {
    ssh_key::PublicKey::from_openssh(contents).ok()
}

/// The fingerprint a registration stores for a public key.
pub(in super::super) fn public_key_fingerprint(public_key: &ssh_key::PublicKey) -> String {
    public_key.fingerprint(Default::default()).to_string()
}

/// The bytes of a regular file of at most 16 KiB, or `None`. Opening never
/// blocks, and nothing is written.
///
/// The buffer is erased when it is dropped, and is allocated once at a size
/// the read cannot outgrow, so no copy of the bytes is left behind by a
/// reallocation. A caller cannot know that the file holds a public key.
pub(in super::super) fn bounded_public_key_contents(
    path: &Path,
) -> Option<zeroize::Zeroizing<Vec<u8>>> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_NONBLOCK);
    let mut file = options.open(path).ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file() || metadata.len() > MAX_OPENSSH_PUBLIC_KEY_FILE_BYTES {
        return None;
    }

    let mut contents = zeroize::Zeroizing::new(Vec::with_capacity(PUBLIC_KEY_BUFFER_CAPACITY));
    std::io::Read::by_ref(&mut file)
        .take(MAX_OPENSSH_PUBLIC_KEY_FILE_BYTES)
        .read_to_end(&mut contents)
        .ok()?;
    let metadata = file.metadata().ok()?;
    (metadata.is_file() && metadata.len() <= MAX_OPENSSH_PUBLIC_KEY_FILE_BYTES).then_some(contents)
}

fn shared_key_ownership_value(ownership: SharedKeyOwnership) -> &'static str {
    match ownership {
        SharedKeyOwnership::Imported => "imported",
        SharedKeyOwnership::Generated => "generated",
    }
}

fn private_key_source_state_value(state: PrivateKeySourceState) -> &'static str {
    match state {
        PrivateKeySourceState::Available => "available",
        PrivateKeySourceState::Missing => "missing",
        PrivateKeySourceState::Unavailable => "unavailable",
    }
}

fn public_key_metadata_state_value(state: PublicKeyMetadataState) -> &'static str {
    match state {
        PublicKeyMetadataState::NotProvided => "not-provided",
        PublicKeyMetadataState::FingerprintAvailable => "available",
        PublicKeyMetadataState::Unavailable => "unavailable",
    }
}

pub(super) fn read_shared_key_registrations(
    connection: &rusqlite::Connection,
    operation: RepositoryOperation,
    data_directory: &Path,
) -> Result<Vec<SharedKeyRegistration>, RepositoryError> {
    stored_shared_key_registrations(connection, operation, data_directory).map_err(|error| {
        match error {
            StoredSharedKeysError::Sqlite(_) => {
                shared_key_registry_unavailable(operation, data_directory)
            }
            StoredSharedKeysError::InvalidMetadata(error) => error,
        }
    })
}

/// Why the stored registrations could not be returned.
pub(in super::super) enum StoredSharedKeysError {
    Sqlite(rusqlite::Error),
    /// A row holds a value no registration can have.
    InvalidMetadata(RepositoryError),
}

/// Every registration in the order it was registered, read through
/// `connection` and nothing else: no lock is taken, no key file is opened
/// and nothing is written. A failure to read is reported before an invalid
/// row is.
pub(in super::super) fn stored_shared_key_registrations(
    connection: &rusqlite::Connection,
    operation: RepositoryOperation,
    data_directory: &Path,
) -> Result<Vec<SharedKeyRegistration>, StoredSharedKeysError> {
    let rows = connection
        .prepare(
            "SELECT id, label, ownership, private_key_path, public_key_path,
                    public_key_fingerprint, private_source_state, public_metadata_state, selected
             FROM shared_ssh_keys ORDER BY rowid",
        )
        .and_then(|mut statement| {
            statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, i64>(8)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(StoredSharedKeysError::Sqlite)?;
    rows.into_iter()
        .map(|row| {
            shared_key_registration_from_row(row, operation, data_directory)
                .map_err(StoredSharedKeysError::InvalidMetadata)
        })
        .collect()
}

#[allow(clippy::type_complexity)]
fn shared_key_registration_from_row(
    row: (
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        String,
        String,
        i64,
    ),
    operation: RepositoryOperation,
    data_directory: &Path,
) -> Result<SharedKeyRegistration, RepositoryError> {
    let (
        id,
        label,
        ownership,
        private_key_path,
        public_key_path,
        public_key_fingerprint,
        private_source_state,
        public_metadata_state,
        selected,
    ) = row;
    if label.trim().is_empty() {
        return Err(invalid_shared_key_metadata(operation, data_directory));
    }
    let id = SharedKeyId::parse(&id)
        .map_err(|_| invalid_shared_key_metadata(operation, data_directory))?;
    let ownership = match ownership.as_str() {
        "imported" => SharedKeyOwnership::Imported,
        "generated" => SharedKeyOwnership::Generated,
        _ => return Err(invalid_shared_key_metadata(operation, data_directory)),
    };
    let private_key_path = shared_key_stored_path(&private_key_path, operation, data_directory)?;
    let public_key_path = public_key_path
        .as_deref()
        .map(|path| shared_key_stored_path(path, operation, data_directory))
        .transpose()?;
    let private_source_state = match private_source_state.as_str() {
        "available" => PrivateKeySourceState::Available,
        "missing" => PrivateKeySourceState::Missing,
        "unavailable" => PrivateKeySourceState::Unavailable,
        _ => return Err(invalid_shared_key_metadata(operation, data_directory)),
    };
    let public_metadata_state = match public_metadata_state.as_str() {
        "not-provided" if public_key_path.is_none() && public_key_fingerprint.is_none() => {
            PublicKeyMetadataState::NotProvided
        }
        "available"
            if public_key_path.is_some()
                && public_key_fingerprint
                    .as_deref()
                    .is_some_and(valid_public_key_fingerprint) =>
        {
            PublicKeyMetadataState::FingerprintAvailable
        }
        "unavailable" if public_key_path.is_some() && public_key_fingerprint.is_none() => {
            PublicKeyMetadataState::Unavailable
        }
        _ => return Err(invalid_shared_key_metadata(operation, data_directory)),
    };
    let selected = match selected {
        0 => false,
        1 => true,
        _ => return Err(invalid_shared_key_metadata(operation, data_directory)),
    };
    Ok(SharedKeyRegistration {
        id,
        label,
        ownership,
        private_key_path,
        public_key_path,
        public_key_fingerprint,
        private_source_state,
        public_metadata_state,
        selected,
    })
}

fn valid_public_key_fingerprint(value: &str) -> bool {
    value.starts_with("SHA256:")
        && value
            .parse::<ssh_key::Fingerprint>()
            .is_ok_and(|fingerprint| fingerprint.to_string() == value)
}

fn shared_key_stored_path(
    value: &str,
    operation: RepositoryOperation,
    data_directory: &Path,
) -> Result<PathBuf, RepositoryError> {
    let path = Path::new(value);
    let (path, _) = normalize_shared_key_path(path, operation, data_directory)
        .map_err(|_| invalid_shared_key_metadata(operation, data_directory))?;
    Ok(path)
}

pub(in super::super) fn migrate_material_schema(
    transaction: &Transaction<'_>,
) -> Result<(), RepositoryError> {
    transaction
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS owned_generated_keys (
                key_id TEXT PRIMARY KEY NOT NULL,
                private_key_path TEXT NOT NULL,
                public_key_path TEXT NOT NULL,
                private_file_identity BLOB NOT NULL,
                public_file_identity BLOB NOT NULL,
                public_key_fingerprint TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS key_material_operations (
                operation_id TEXT PRIMARY KEY NOT NULL,
                key_id TEXT NOT NULL,
                action TEXT NOT NULL CHECK (action IN ('generate', 'delete')),
                generation_label TEXT,
                private_key_path TEXT NOT NULL,
                public_key_path TEXT NOT NULL,
                private_file_identity BLOB,
                public_file_identity BLOB,
                public_key_fingerprint TEXT,
                phase TEXT NOT NULL,
                failure_code TEXT,
                CHECK (
                    (action = 'generate' AND phase IN (
                        'reserved', 'private-written', 'pair-written', 'completed',
                        'retained-for-inspection'
                    )) OR
                    (action = 'delete' AND phase IN (
                        'prepared', 'private-removed', 'files-removed', 'completed',
                        'retained-for-inspection'
                    ))
                ),
                CHECK (
                    (action = 'generate' AND generation_label IS NOT NULL
                        AND trim(generation_label) <> '') OR
                    (action = 'delete' AND generation_label IS NULL)
                )
            );
            CREATE UNIQUE INDEX IF NOT EXISTS key_material_operations_one_incomplete_per_key_idx
                ON key_material_operations(key_id) WHERE phase <> 'completed';",
        )
        .map_err(RepositoryError::sqlite)
}

/// Where the key-material operation `operation_id` stands, for the
/// mutation boundary's settling. Of the operation only its phase is read:
/// neither its paths nor its label.
///
/// A phase a generation or a deletion passes through is pending:
/// key-material recovery takes it up again. `completed` and
/// `retained-for-inspection` are final, whether or not the operation
/// failed, and neither owes work: nothing continues either.
#[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
pub(in super::super) fn lookup_material_operation(
    connection: &rusqlite::Connection,
    operation_id: super::OperationId,
) -> Result<JournalRow, ReadError> {
    let stored: Option<String> = connection
        .query_row(
            "SELECT phase FROM key_material_operations WHERE operation_id = ?1",
            [operation_id.to_string()],
            |row| row.get(0),
        )
        .optional()?;
    let Some(stored) = stored else {
        return Ok(JournalRow::Absent);
    };
    let phase = super::generation::parse_phase(&stored)?;
    Ok(match phase {
        KeyMaterialPhase::Reserved
        | KeyMaterialPhase::PrivateWritten
        | KeyMaterialPhase::PairWritten
        | KeyMaterialPhase::Prepared
        | KeyMaterialPhase::PrivateRemoved
        | KeyMaterialPhase::FilesRemoved => {
            JournalRow::Pending(PendingOperation::KeyMaterial { phase })
        }
        KeyMaterialPhase::Completed => JournalRow::Final {
            kind: FinalKind::Completed,
            owes_work: false,
            checkpointed: true,
        },
        KeyMaterialPhase::RetainedForInspection => JournalRow::Final {
            kind: FinalKind::RetainedForInspection,
            owes_work: false,
            checkpointed: true,
        },
    })
}

fn require_no_pending_material(
    connection: &rusqlite::Connection,
    id: SharedKeyId,
    operation: RepositoryOperation,
    data_directory: &Path,
) -> Result<(), RepositoryError> {
    let pending: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM key_material_operations WHERE key_id=?1 AND phase <> 'completed')",
        [id.to_string()], |row| row.get(0),
    ).map_err(|_| shared_key_registry_unavailable(operation, data_directory))?;
    if pending {
        Err(RepositoryError::new(
            operation,
            Some(data_directory.to_owned()),
            RepositoryErrorKind::SharedKeyMaterialPending,
            "key material recovery is pending; review the operation and confirm recovery before selecting or unregistering the key",
        ))
    } else {
        Ok(())
    }
}
