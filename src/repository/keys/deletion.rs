//! Confirmed deletion uses historical creation evidence, never caller labels.
use super::super::{FailurePoint, RepositoryOperation, RepositoryService};
use super::generation::{
    MaterialOperation, decode_identity, failure_code, read_operation, save_progress,
};
use super::storage::{FileIdentity, KeyFileKind, OwnedKeyFile, OwnedStoreGuard};
use super::*;
use rusqlite::{Connection, OptionalExtension, params};

/// A one-use metadata snapshot. Obtain another review after interruption or change.
///
/// ```compile_fail
/// # use manyhands::repository::keys::GeneratedKeyDeletionReview;
/// # fn requires_clone<T: Clone>() {}
/// requires_clone::<GeneratedKeyDeletionReview>();
/// ```
/// ```compile_fail
/// # use manyhands::repository::keys::GeneratedKeyDeletionReview;
/// # fn requires_serialize<T: serde::Serialize>() {}
/// requires_serialize::<GeneratedKeyDeletionReview>();
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct GeneratedKeyDeletionReview {
    registration: SharedKeyRegistration,
    evidence: CreationEvidence,
    private: Option<FileIdentity>,
    public: Option<FileIdentity>,
}
impl GeneratedKeyDeletionReview {
    pub fn registration(&self) -> &SharedKeyRegistration {
        &self.registration
    }
}
#[derive(Debug, PartialEq, Eq)]
struct CreationEvidence {
    private: FileIdentity,
    public: FileIdentity,
}

impl RepositoryService {
    pub fn review_generated_key_deletion(
        &self,
        store: &KeyStore,
        id: SharedKeyId,
    ) -> Result<GeneratedKeyDeletionReview, KeyMaterialError> {
        let result = (|| {
            // Reject unrelated/unproven registrations before opening or creating a store.
            self.material_registry(|c| eligible(c, store, id).map(|_| ()))?;
            let guard = store.lock()?;
            self.material_registry(|c| snapshot(c, store, &guard, id).map(|(review, _, _)| review))
        })();
        result.map_err(|mut e| {
            e.operation = KeyMaterialAction::ReviewDeletion;
            e.key_id = Some(id);
            e
        })
    }

    pub fn delete_generated_key(
        &self,
        store: &KeyStore,
        operation_id: OperationId,
        review: Option<GeneratedKeyDeletionReview>,
        confirmed: bool,
    ) -> Result<DeleteGeneratedKeyOutcome, KeyMaterialError> {
        // Cancellation is deliberately prior to all registry and filesystem access.
        if !confirmed {
            return Ok(DeleteGeneratedKeyOutcome::Cancelled);
        }
        let key_id = review.as_ref().map(|r| r.registration.id);
        let result = (|| {
            // Completion replay must not even open replacement entries in the store.
            let completed =
                self.material_registry(|c| replay_status(c, store, operation_id, review.as_ref()))?;
            if completed {
                return Ok(DeleteGeneratedKeyOutcome::AlreadyDeleted);
            }
            let review = review.ok_or_else(|| error(KeyMaterialErrorKind::ConfirmationRequired))?;
            let guard = store.lock()?;
            self.material_registry(|c| {
                if replay_status(c, store, operation_id, Some(&review))? {
                    return Ok(DeleteGeneratedKeyOutcome::AlreadyDeleted);
                }
                let (current, private, public) = snapshot(c, store, &guard, review.registration.id)?;
                if current != review { return Err(error(KeyMaterialErrorKind::SourceChanged)); }
                let pending: Option<String> = c.query_row(
                    "SELECT operation_id FROM key_material_operations WHERE key_id=?1 AND phase <> 'completed'",
                    [review.registration.id.to_string()], |r| r.get(0),
                ).optional().map_err(registry_error)?;
                if pending.as_deref().is_some_and(|id| id != operation_id.to_string()) {
                    return Err(error(KeyMaterialErrorKind::Busy));
                }
                if read_operation(c, operation_id)?.is_none() {
                    c.execute("INSERT INTO key_material_operations (operation_id,key_id,action,private_key_path,public_key_path,private_file_identity,public_file_identity,public_key_fingerprint,phase) VALUES (?1,?2,'delete',?3,?4,?5,?6,?7,'prepared')",
                        params![operation_id.to_string(), review.registration.id.to_string(), review.registration.private_key_path.to_str(), review.registration.public_key_path.as_ref().and_then(|p| p.to_str()), review.evidence.private.encode().as_bytes(), review.evidence.public.encode().as_bytes(), review.registration.public_key_fingerprint]).map_err(registry_error)?;
                }
                let mut op = read_operation(c, operation_id)?.ok_or_else(|| error(KeyMaterialErrorKind::RegistryUnavailable))?;
                let work = (|| {
                    self.deletion_checkpoint(FailurePoint::DeletionAfterIntent)?;
                    if let Some(file) = private { file.remove()?; }
                    guard.sync_directory()?;
                    self.deletion_checkpoint(FailurePoint::DeletionAfterPrivateUnlink)?;
                    op.phase = KeyMaterialPhase::PrivateRemoved;
                    op.failure = None;
                    save_progress(c, &op)?;
                    self.deletion_checkpoint(FailurePoint::DeletionAfterPrivatePhase)?;
                    if let Some(file) = public { file.remove()?; }
                    guard.sync_directory()?;
                    self.deletion_checkpoint(FailurePoint::DeletionAfterPublicUnlink)?;
                    op.phase = KeyMaterialPhase::FilesRemoved;
                    save_progress(c, &op)?;
                    self.deletion_checkpoint(FailurePoint::DeletionAfterFilesPhase)?;
                    // Check absence afresh; progress alone never authorizes finalization.
                    if guard.open_owned(op.key_id, KeyFileKind::Private)?.is_some()
                        || guard.open_owned(op.key_id, KeyFileKind::Public)?.is_some() {
                        return Err(error(KeyMaterialErrorKind::SourceChanged));
                    }
                    self.deletion_checkpoint(FailurePoint::DeletionBeforeFinalTransaction)?;
                    let tx = c.transaction().map_err(registry_error)?;
                    tx.execute("DELETE FROM shared_ssh_keys WHERE id=?1", [op.key_id.to_string()]).map_err(registry_error)?;
                    tx.execute("DELETE FROM owned_generated_keys WHERE key_id=?1", [op.key_id.to_string()]).map_err(registry_error)?;
                    tx.execute("UPDATE key_material_operations SET phase='completed',failure_code=NULL WHERE operation_id=?1", [operation_id.to_string()]).map_err(registry_error)?;
                    tx.commit().map_err(registry_error)?;
                    op.phase = KeyMaterialPhase::Completed;
                    self.deletion_checkpoint(FailurePoint::DeletionAfterFinalTransaction)?;
                    Ok(DeleteGeneratedKeyOutcome::Deleted)
                })();
                match work {
                    Ok(outcome) => Ok(outcome),
                    Err(e) => {
                        op.failure = Some(failure_code(e.kind).into());
                        let _ = save_progress(c, &op);
                        Ok(DeleteGeneratedKeyOutcome::RecoveryRequired(op.recovery()))
                    }
                }
            })
        })();
        result.map_err(|mut e| {
            e.operation = KeyMaterialAction::Delete;
            e.operation_id = Some(operation_id);
            e.key_id = key_id;
            e
        })
    }

    fn deletion_checkpoint(&self, point: FailurePoint) -> Result<(), KeyMaterialError> {
        if self
            .should_inject(
                point,
                RepositoryOperation::UnregisterSharedKey,
                self.registry_data_directory(),
            )
            .map_err(registry_error)?
        {
            let kind = match point {
                FailurePoint::DeletionBeforeFinalTransaction
                | FailurePoint::DeletionAfterFinalTransaction => {
                    KeyMaterialErrorKind::RegistryUnavailable
                }
                _ => KeyMaterialErrorKind::StorageUnavailable,
            };
            Err(error(kind))
        } else {
            Ok(())
        }
    }
}

fn eligible(
    c: &Connection,
    store: &KeyStore,
    id: SharedKeyId,
) -> Result<(SharedKeyRegistration, CreationEvidence), KeyMaterialError> {
    let r = super::registry::read_shared_key_registrations(
        c,
        RepositoryOperation::PreflightGeneratedKeyDeletion,
        std::path::Path::new("."),
    )
    .map_err(registry_error)?
    .into_iter()
    .find(|r| r.id == id)
    .ok_or_else(|| error(KeyMaterialErrorKind::NotRegistered))?;
    if r.ownership != SharedKeyOwnership::Generated {
        return Err(error(KeyMaterialErrorKind::ImportedKey));
    }
    if r.selected {
        return Err(error(KeyMaterialErrorKind::SelectedKeyMustBeCleared));
    }
    let evidence = c.query_row("SELECT private_key_path,public_key_path,private_file_identity,public_file_identity,public_key_fingerprint FROM owned_generated_keys WHERE key_id=?1",
        [id.to_string()], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,Vec<u8>>(2)?,row.get::<_,Vec<u8>>(3)?,row.get::<_,String>(4)?)))
        .optional().map_err(registry_error)?.ok_or_else(|| error(KeyMaterialErrorKind::OwnershipUnverified))?;
    let paths = store.key_paths(id);
    if r.private_key_path != paths.0
        || r.public_key_path.as_ref() != Some(&paths.1)
        || std::path::Path::new(&evidence.0) != paths.0
        || std::path::Path::new(&evidence.1) != paths.1
        || r.public_key_fingerprint.as_ref() != Some(&evidence.4)
    {
        return Err(error(KeyMaterialErrorKind::OwnershipUnverified));
    }
    let pending_generation: bool = c.query_row("SELECT EXISTS(SELECT 1 FROM key_material_operations WHERE key_id=?1 AND action <> 'delete' AND phase <> 'completed')", [id.to_string()], |r| r.get(0)).map_err(registry_error)?;
    if pending_generation {
        return Err(error(KeyMaterialErrorKind::Busy));
    }
    Ok((
        r,
        CreationEvidence {
            private: decode_identity(Some(&evidence.2))?,
            public: decode_identity(Some(&evidence.3))?,
        },
    ))
}

type Snapshot = (
    GeneratedKeyDeletionReview,
    Option<OwnedKeyFile>,
    Option<OwnedKeyFile>,
);
fn snapshot(
    c: &Connection,
    store: &KeyStore,
    guard: &OwnedStoreGuard,
    id: SharedKeyId,
) -> Result<Snapshot, KeyMaterialError> {
    let (registration, evidence) = eligible(c, store, id)?;
    let private = guard.open_owned(id, KeyFileKind::Private)?;
    let public = guard.open_owned(id, KeyFileKind::Public)?;
    let private_identity = private.as_ref().map(OwnedKeyFile::identity);
    let public_identity = public.as_ref().map(OwnedKeyFile::identity);
    if private_identity
        .as_ref()
        .is_some_and(|i| i != &evidence.private)
        || public_identity
            .as_ref()
            .is_some_and(|i| i != &evidence.public)
    {
        return Err(error(KeyMaterialErrorKind::SourceChanged));
    }
    Ok((
        GeneratedKeyDeletionReview {
            registration,
            evidence,
            private: private_identity,
            public: public_identity,
        },
        private,
        public,
    ))
}

fn replay_status(
    c: &Connection,
    store: &KeyStore,
    id: OperationId,
    review: Option<&GeneratedKeyDeletionReview>,
) -> Result<bool, KeyMaterialError> {
    let Some(op) = read_operation(c, id)? else {
        return Ok(false);
    };
    matches_operation(&op, store, review)?;
    Ok(op.phase == KeyMaterialPhase::Completed)
}
fn matches_operation(
    op: &MaterialOperation,
    store: &KeyStore,
    review: Option<&GeneratedKeyDeletionReview>,
) -> Result<(), KeyMaterialError> {
    let paths = store.key_paths(op.key_id);
    if op.action != KeyMaterialAction::Delete
        || paths.0 != std::path::Path::new(&op.private_path)
        || paths.1 != std::path::Path::new(&op.public_path)
    {
        return Err(error(KeyMaterialErrorKind::OperationMismatch));
    }
    if let Some(r) = review
        && (r.registration.id != op.key_id
            || r.registration.private_key_path != paths.0
            || r.registration.public_key_path.as_ref() != Some(&paths.1)
            || r.registration.public_key_fingerprint != op.fingerprint
            || decode_identity(op.private_identity.as_deref())? != r.evidence.private
            || decode_identity(op.public_identity.as_deref())? != r.evidence.public)
    {
        return Err(error(KeyMaterialErrorKind::OperationMismatch));
    }
    Ok(())
}
fn error(kind: KeyMaterialErrorKind) -> KeyMaterialError {
    KeyMaterialError {
        operation: KeyMaterialAction::Delete,
        key_id: None,
        operation_id: None,
        kind,
    }
}
fn registry_error(_: impl std::fmt::Debug) -> KeyMaterialError {
    error(KeyMaterialErrorKind::RegistryUnavailable)
}
