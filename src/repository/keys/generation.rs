//! Durable generation: journal intent before exclusive writes, then commit evidence and registration.
use super::super::{
    FailurePoint, RepositoryOperation, RepositoryService, cache_write_guard, open_registry,
};
use super::storage::{FileIdentity, KeyFileKind, OwnedStoreGuard};
use super::*;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use ssh_key::rand_core::{OsRng, RngCore};
use ssh_key::{Cipher, HashAlg, Kdf, LineEnding, PrivateKey, PublicKey};
use zeroize::Zeroizing;

#[derive(Clone)]
struct MaterialOperation {
    operation_id: OperationId,
    key_id: SharedKeyId,
    action: KeyMaterialAction,
    label: Option<String>,
    private_path: String,
    public_path: String,
    private_identity: Option<Vec<u8>>,
    public_identity: Option<Vec<u8>>,
    fingerprint: Option<String>,
    phase: KeyMaterialPhase,
    failure: Option<String>,
}

fn error(kind: KeyMaterialErrorKind) -> KeyMaterialError {
    KeyMaterialError {
        operation: KeyMaterialAction::Generate,
        key_id: None,
        operation_id: None,
        kind,
    }
}
fn registry_error(_: impl std::fmt::Debug) -> KeyMaterialError {
    error(KeyMaterialErrorKind::RegistryUnavailable)
}
fn entropy(bytes: &mut [u8]) -> Result<(), KeyMaterialError> {
    OsRng
        .try_fill_bytes(bytes)
        .map_err(|_| error(KeyMaterialErrorKind::RandomnessUnavailable))
}

impl RepositoryService {
    pub fn generate_shared_key(
        &self,
        store: &KeyStore,
        request: GenerateSharedKeyRequest,
    ) -> Result<GenerateSharedKeyOutcome, KeyMaterialError> {
        self.generate_key(store, &request).map_err(|mut e| {
            e.operation = KeyMaterialAction::Generate;
            e.operation_id = Some(request.operation_id);
            e
        })
    }

    fn generate_key(
        &self,
        store: &KeyStore,
        request: &GenerateSharedKeyRequest,
    ) -> Result<GenerateSharedKeyOutcome, KeyMaterialError> {
        if request.label.trim().is_empty() {
            return Err(error(KeyMaterialErrorKind::InvalidLabel));
        }
        // Validate replay identity before any cryptography or store mutation.
        let existing = self.material_registry(|c| read_operation(c, request.operation_id))?;
        if let Some(op) = &existing {
            op.matches(store, request)?;
        }
        let encoded = if existing.is_none() {
            if self.generation_injected(FailurePoint::GenerationEntropyUnavailable)? {
                return Err(error(KeyMaterialErrorKind::RandomnessUnavailable));
            }
            Some(encode_pair(&request.protection)?)
        } else {
            None
        };
        let guard = store.lock()?;
        self.material_registry(|connection| {
            if let Some(op) = read_operation(connection, request.operation_id)? {
                op.matches(store, request)?;
                return self.replay_generation(connection, &guard, op);
            }
            // An operation disappearing between the two observations is not authorization to recreate it.
            let Some((private, public)) = encoded else {
                return Err(error(KeyMaterialErrorKind::OperationMismatch));
            };
            let transaction = connection.transaction().map_err(registry_error)?;
            let key_id =
                reserve_generation(&transaction, request.operation_id, &request.label, store)?;
            transaction.commit().map_err(registry_error)?;
            let mut op = read_operation(connection, request.operation_id)?
                .ok_or_else(|| error(KeyMaterialErrorKind::RegistryUnavailable))?;
            if self.generation_injected(FailurePoint::GenerationAfterReservation)? {
                return Ok(interrupted(
                    connection,
                    &mut op,
                    KeyMaterialErrorKind::StorageUnavailable,
                ));
            }
            let result = (|| {
                let mut file = guard.create_private(key_id)?;
                if self.generation_injected(FailurePoint::GenerationAfterExclusiveCreate)? {
                    return Err(error(KeyMaterialErrorKind::StorageUnavailable));
                }
                file.write_all_and_sync(private.as_bytes())?;
                op.private_identity = Some(file.identity().encode().into_bytes());
                op.phase = KeyMaterialPhase::PrivateWritten;
                save_progress(connection, &op)?;
                if self.generation_injected(FailurePoint::GenerationAfterPrivateWrite)? {
                    return Err(error(KeyMaterialErrorKind::StorageUnavailable));
                }
                let mut file = guard.create_public(key_id)?;
                file.write_all_and_sync(public.as_bytes())?;
                op.public_identity = Some(file.identity().encode().into_bytes());
                op.fingerprint = Some(
                    PublicKey::from_openssh(&public)
                        .map_err(|_| error(KeyMaterialErrorKind::InvalidGeneratedKey))?
                        .fingerprint(HashAlg::Sha256)
                        .to_string(),
                );
                guard.sync_directory()?;
                op.phase = KeyMaterialPhase::PairWritten;
                save_progress(connection, &op)?;
                if self.generation_injected(FailurePoint::GenerationAfterPublicWrite)? {
                    return Err(error(KeyMaterialErrorKind::StorageUnavailable));
                }
                verify_pair(&guard, &op)?;
                if self.generation_injected(FailurePoint::GenerationBeforeFinalTransaction)? {
                    return Err(error(KeyMaterialErrorKind::RegistryUnavailable));
                }
                finalize(connection, &op)?;
                op.phase = KeyMaterialPhase::Completed;
                if self.generation_injected(FailurePoint::GenerationAfterFinalTransaction)? {
                    return Err(error(KeyMaterialErrorKind::RegistryUnavailable));
                }
                Ok(GenerateSharedKeyOutcome::Created(registration(
                    connection, &op,
                )?))
            })();
            match result {
                Ok(outcome) => Ok(outcome),
                Err(e) => Ok(interrupted(connection, &mut op, e.kind)),
            }
        })
    }

    fn replay_generation(
        &self,
        connection: &mut Connection,
        guard: &OwnedStoreGuard,
        mut op: MaterialOperation,
    ) -> Result<GenerateSharedKeyOutcome, KeyMaterialError> {
        if op.phase == KeyMaterialPhase::Completed {
            let registration = registration(connection, &op)?;
            let verified = verify_pair(guard, &op).and_then(|()| verify_evidence(connection, &op));
            if let Err(e) = verified {
                op.failure = Some(failure_code(e.kind).into());
                // Keep completion evidence intact; missing/changed sources never recreate material.
                let _ = save_progress(connection, &op);
                return Ok(GenerateSharedKeyOutcome::RecoveryRequired(op.recovery()));
            }
            if op.failure.take().is_some() {
                save_progress(connection, &op)?;
            }
            return Ok(GenerateSharedKeyOutcome::AlreadyCreated(registration));
        }
        if op.phase == KeyMaterialPhase::PairWritten {
            match verify_pair(guard, &op) {
                Ok(()) => {
                    guard.sync_directory()?;
                    match finalize(connection, &op) {
                        Ok(()) => {
                            return Ok(GenerateSharedKeyOutcome::Created(registration(
                                connection, &op,
                            )?));
                        }
                        Err(e) => return Ok(interrupted(connection, &mut op, e.kind)),
                    }
                }
                Err(e) => {
                    op.failure = Some(failure_code(e.kind).into());
                }
            }
        } else if op.failure.is_none() {
            op.failure = Some("ownership-unverified".into());
        }
        op.phase = KeyMaterialPhase::RetainedForInspection;
        save_progress(connection, &op)?;
        Ok(GenerateSharedKeyOutcome::RecoveryRequired(op.recovery()))
    }

    pub fn list_key_material_recovery(&self) -> Result<Vec<KeyMaterialRecovery>, KeyMaterialError> {
        self.material_registry(|connection| {
            let mut statement = connection.prepare("SELECT operation_id FROM key_material_operations WHERE phase <> 'completed' OR failure_code IS NOT NULL ORDER BY rowid").map_err(registry_error)?;
            let ids = statement.query_map([], |row| row.get::<_,String>(0)).map_err(registry_error)?.collect::<Result<Vec<_>,_>>().map_err(registry_error)?;
            ids.into_iter().map(|id| {
                let id = OperationId::parse(&id).map_err(registry_error)?;
                read_operation(connection, id)?.map(|op| op.recovery()).ok_or_else(|| error(KeyMaterialErrorKind::RegistryUnavailable))
            }).collect()
        }).map_err(|mut e| { e.operation = KeyMaterialAction::ListRecovery; e })
    }

    fn material_registry<T>(
        &self,
        work: impl FnOnce(&mut Connection) -> Result<T, KeyMaterialError>,
    ) -> Result<T, KeyMaterialError> {
        let operation = RepositoryOperation::RegisterSharedKey;
        self.require_shared_key_registry(operation)
            .map_err(registry_error)?;
        let _cache = cache_write_guard(
            &self.registry_path,
            self.registry_data_directory(),
            operation,
        )
        .map_err(|e| {
            if e.kind == super::super::RepositoryErrorKind::RepositoryBusy {
                error(KeyMaterialErrorKind::Busy)
            } else {
                registry_error(e)
            }
        })?;
        let mut connection =
            open_registry(&self.registry_path, &mut |_| {}).map_err(registry_error)?;
        work(&mut connection)
    }

    fn generation_injected(&self, point: FailurePoint) -> Result<bool, KeyMaterialError> {
        self.should_inject(
            point,
            RepositoryOperation::RegisterSharedKey,
            self.registry_data_directory(),
        )
        .map_err(registry_error)
    }
}

fn encode_pair(
    protection: &KeyProtection,
) -> Result<(Zeroizing<String>, String), KeyMaterialError> {
    // Upstream random/encrypt convenience methods use infallible RNG calls. Acquire
    // OS randomness fallibly before constructing any material instead.
    let mut seed = Zeroizing::new([0u8; 32]);
    entropy(seed.as_mut())?;
    let pair = ssh_key::private::Ed25519Keypair::from_seed(&seed);
    let private = PrivateKey::new(pair.into(), "")
        .map_err(|_| error(KeyMaterialErrorKind::GenerationFailed))?;
    let public = private
        .public_key()
        .to_openssh()
        .map_err(|_| error(KeyMaterialErrorKind::GenerationFailed))?;
    let private = match protection {
        KeyProtection::Unencrypted => private,
        KeyProtection::Passphrase(passphrase) => {
            let mut random = Zeroizing::new([0u8; 20]);
            entropy(random.as_mut())?;
            let checkint = u32::from_le_bytes([random[0], random[1], random[2], random[3]]);
            private
                .encrypt_with(
                    Cipher::Aes256Ctr,
                    Kdf::Bcrypt {
                        salt: random[4..].to_vec(),
                        rounds: 16,
                    },
                    checkint,
                    passphrase.expose().as_bytes(),
                )
                .map_err(|_| error(KeyMaterialErrorKind::GenerationFailed))?
        }
    };
    Ok((
        private
            .to_openssh(LineEnding::LF)
            .map_err(|_| error(KeyMaterialErrorKind::GenerationFailed))?,
        public,
    ))
}

fn reserve_generation(
    tx: &Transaction<'_>,
    operation_id: OperationId,
    label: &str,
    store: &KeyStore,
) -> Result<SharedKeyId, KeyMaterialError> {
    let mut bytes = Zeroizing::new([0u8; 16]);
    entropy(bytes.as_mut())?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| error(KeyMaterialErrorKind::GenerationFailed))?
        .as_millis() as u64;
    let key_id = SharedKeyId(ulid::Ulid::from_parts(
        timestamp,
        u128::from_le_bytes(*bytes),
    ));
    let (private, public) = store.key_paths(key_id);
    let private = private
        .to_str()
        .ok_or_else(|| error(KeyMaterialErrorKind::UnsafePath))?;
    let public = public
        .to_str()
        .ok_or_else(|| error(KeyMaterialErrorKind::UnsafePath))?;
    tx.execute("INSERT INTO key_material_operations (operation_id,key_id,action,generation_label,private_key_path,public_key_path,phase) VALUES (?1,?2,'generate',?3,?4,?5,'reserved')", params![operation_id.to_string(),key_id.to_string(),label,private,public]).map_err(registry_error)?;
    Ok(key_id)
}

fn verify_pair(guard: &OwnedStoreGuard, op: &MaterialOperation) -> Result<(), KeyMaterialError> {
    let mut private = guard
        .open_owned(op.key_id, KeyFileKind::Private)?
        .ok_or_else(|| error(KeyMaterialErrorKind::SourceMissing))?;
    let mut public = guard
        .open_owned(op.key_id, KeyFileKind::Public)?
        .ok_or_else(|| error(KeyMaterialErrorKind::SourceMissing))?;
    if decode_identity(op.private_identity.as_deref())? != private.identity()
        || decode_identity(op.public_identity.as_deref())? != public.identity()
    {
        return Err(error(KeyMaterialErrorKind::SourceChanged));
    }
    let private_bytes = private.read_secret()?;
    let public_bytes = public.read_secret()?;
    let key = PrivateKey::from_openssh(&*private_bytes)
        .map_err(|_| error(KeyMaterialErrorKind::InvalidGeneratedKey))?;
    let public_str = std::str::from_utf8(&public_bytes)
        .map_err(|_| error(KeyMaterialErrorKind::InvalidGeneratedKey))?;
    let public_key = PublicKey::from_openssh(public_str)
        .map_err(|_| error(KeyMaterialErrorKind::InvalidGeneratedKey))?;
    if key.algorithm() != ssh_key::Algorithm::Ed25519
        || key.public_key() != &public_key
        || !key.comment().is_empty()
        || !public_key.comment().is_empty()
        || Some(public_key.fingerprint(HashAlg::Sha256).to_string()) != op.fingerprint
    {
        return Err(error(KeyMaterialErrorKind::InvalidGeneratedKey));
    }
    Ok(())
}
fn decode_identity(value: Option<&[u8]>) -> Result<FileIdentity, KeyMaterialError> {
    let value = value
        .and_then(|v| std::str::from_utf8(v).ok())
        .ok_or_else(|| error(KeyMaterialErrorKind::OwnershipUnverified))?;
    FileIdentity::decode(value)
}

fn finalize(connection: &mut Connection, op: &MaterialOperation) -> Result<(), KeyMaterialError> {
    let tx = connection.transaction().map_err(registry_error)?;
    tx.execute("INSERT INTO owned_generated_keys (key_id,private_key_path,public_key_path,private_file_identity,public_file_identity,public_key_fingerprint) VALUES (?1,?2,?3,?4,?5,?6)", params![op.key_id.to_string(),op.private_path,op.public_path,op.private_identity,op.public_identity,op.fingerprint]).map_err(registry_error)?;
    tx.execute("INSERT INTO shared_ssh_keys (id,label,ownership,private_key_path,public_key_path,public_key_fingerprint,private_source_state,public_metadata_state,selected) VALUES (?1,?2,'generated',?3,?4,?5,'available','available',0)", params![op.key_id.to_string(),op.label,op.private_path,op.public_path,op.fingerprint]).map_err(registry_error)?;
    tx.execute("UPDATE key_material_operations SET phase='completed',failure_code=NULL WHERE operation_id=?1", [op.operation_id.to_string()]).map_err(registry_error)?;
    tx.commit().map_err(registry_error)
}
fn verify_evidence(c: &Connection, op: &MaterialOperation) -> Result<(), KeyMaterialError> {
    let valid: bool = c.query_row("SELECT EXISTS(SELECT 1 FROM owned_generated_keys WHERE key_id=?1 AND private_key_path=?2 AND public_key_path=?3 AND private_file_identity=?4 AND public_file_identity=?5 AND public_key_fingerprint=?6)", params![op.key_id.to_string(),op.private_path,op.public_path,op.private_identity,op.public_identity,op.fingerprint], |r| r.get(0)).map_err(registry_error)?;
    if valid {
        Ok(())
    } else {
        Err(error(KeyMaterialErrorKind::OwnershipUnverified))
    }
}
fn registration(
    c: &Connection,
    op: &MaterialOperation,
) -> Result<SharedKeyRegistration, KeyMaterialError> {
    let rows = super::registry::read_shared_key_registrations(
        c,
        RepositoryOperation::ListSharedKeys,
        std::path::Path::new("."),
    )
    .map_err(registry_error)?;
    let r = rows
        .into_iter()
        .find(|r| r.id == op.key_id)
        .ok_or_else(|| error(KeyMaterialErrorKind::NotRegistered))?;
    if r.ownership != SharedKeyOwnership::Generated
        || r.private_key_path != std::path::Path::new(&op.private_path)
        || r.public_key_path.as_deref() != Some(std::path::Path::new(&op.public_path))
        || r.public_key_fingerprint != op.fingerprint
        || Some(&r.label) != op.label.as_ref()
    {
        return Err(error(KeyMaterialErrorKind::OwnershipUnverified));
    }
    Ok(r)
}
fn save_progress(c: &Connection, op: &MaterialOperation) -> Result<(), KeyMaterialError> {
    c.execute("UPDATE key_material_operations SET phase=?2,private_file_identity=?3,public_file_identity=?4,public_key_fingerprint=?5,failure_code=?6 WHERE operation_id=?1", params![op.operation_id.to_string(), phase_value(op.phase),op.private_identity,op.public_identity,op.fingerprint,op.failure]).map_err(registry_error)?;
    Ok(())
}
fn interrupted(
    c: &Connection,
    op: &mut MaterialOperation,
    kind: KeyMaterialErrorKind,
) -> GenerateSharedKeyOutcome {
    op.failure = Some(failure_code(kind).into());
    let _ = save_progress(c, op);
    GenerateSharedKeyOutcome::RecoveryRequired(op.recovery())
}

impl MaterialOperation {
    fn matches(
        &self,
        store: &KeyStore,
        request: &GenerateSharedKeyRequest,
    ) -> Result<(), KeyMaterialError> {
        let paths = store.key_paths(self.key_id);
        if self.action != KeyMaterialAction::Generate
            || self.label.as_deref() != Some(&request.label)
            || paths.0 != std::path::Path::new(&self.private_path)
            || paths.1 != std::path::Path::new(&self.public_path)
        {
            return Err(error(KeyMaterialErrorKind::OperationMismatch));
        }
        Ok(())
    }
    fn recovery(&self) -> KeyMaterialRecovery {
        let recovery_action = match (self.action, self.phase) {
            (_, KeyMaterialPhase::RetainedForInspection | KeyMaterialPhase::Completed) => {
                RecoveryAction::InspectRetainedFiles
            }
            (KeyMaterialAction::Generate, _) => RecoveryAction::RetryGeneration,
            _ => RecoveryAction::ReviewDeletionAgain,
        };
        KeyMaterialRecovery {
            operation_id: self.operation_id,
            key_id: self.key_id,
            action: self.action,
            phase: self.phase,
            failure_code: self
                .failure
                .as_ref()
                .map(|v| KeyMaterialFailureCode(sanitize_code(v).into())),
            recovery_action,
        }
    }
}
fn read_operation(
    c: &Connection,
    id: OperationId,
) -> Result<Option<MaterialOperation>, KeyMaterialError> {
    let raw = c.query_row("SELECT key_id,action,generation_label,private_key_path,public_key_path,private_file_identity,public_file_identity,public_key_fingerprint,phase,failure_code FROM key_material_operations WHERE operation_id=?1", [id.to_string()], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,Option<Vec<u8>>>(5)?,r.get::<_,Option<Vec<u8>>>(6)?,r.get::<_,Option<String>>(7)?,r.get::<_,String>(8)?,r.get::<_,Option<String>>(9)?))).optional().map_err(registry_error)?;
    raw.map(|r| {
        Ok(MaterialOperation {
            operation_id: id,
            key_id: SharedKeyId::parse(&r.0).map_err(registry_error)?,
            action: match r.1.as_str() {
                "generate" => KeyMaterialAction::Generate,
                "delete" => KeyMaterialAction::Delete,
                _ => return Err(error(KeyMaterialErrorKind::RegistryUnavailable)),
            },
            label: r.2,
            private_path: r.3,
            public_path: r.4,
            private_identity: r.5,
            public_identity: r.6,
            fingerprint: r.7,
            phase: parse_phase(&r.8)?,
            failure: r.9,
        })
    })
    .transpose()
}
fn phase_value(phase: KeyMaterialPhase) -> &'static str {
    match phase {
        KeyMaterialPhase::Reserved => "reserved",
        KeyMaterialPhase::PrivateWritten => "private-written",
        KeyMaterialPhase::PairWritten => "pair-written",
        KeyMaterialPhase::Prepared => "prepared",
        KeyMaterialPhase::PrivateRemoved => "private-removed",
        KeyMaterialPhase::FilesRemoved => "files-removed",
        KeyMaterialPhase::Completed => "completed",
        KeyMaterialPhase::RetainedForInspection => "retained-for-inspection",
    }
}
fn parse_phase(value: &str) -> Result<KeyMaterialPhase, KeyMaterialError> {
    match value {
        "reserved" => Ok(KeyMaterialPhase::Reserved),
        "private-written" => Ok(KeyMaterialPhase::PrivateWritten),
        "pair-written" => Ok(KeyMaterialPhase::PairWritten),
        "prepared" => Ok(KeyMaterialPhase::Prepared),
        "private-removed" => Ok(KeyMaterialPhase::PrivateRemoved),
        "files-removed" => Ok(KeyMaterialPhase::FilesRemoved),
        "completed" => Ok(KeyMaterialPhase::Completed),
        "retained-for-inspection" => Ok(KeyMaterialPhase::RetainedForInspection),
        _ => Err(error(KeyMaterialErrorKind::RegistryUnavailable)),
    }
}
fn failure_code(kind: KeyMaterialErrorKind) -> &'static str {
    match kind {
        KeyMaterialErrorKind::RegistryUnavailable => "registry-unavailable",
        KeyMaterialErrorKind::SourceMissing => "source-missing",
        KeyMaterialErrorKind::SourceChanged => "source-changed",
        KeyMaterialErrorKind::OwnershipUnverified => "ownership-unverified",
        KeyMaterialErrorKind::InvalidGeneratedKey => "invalid-generated-key",
        KeyMaterialErrorKind::ProtectionUnavailable => "protection-unavailable",
        KeyMaterialErrorKind::UnsafePath => "unsafe-path",
        _ => "storage-unavailable",
    }
}
fn sanitize_code(value: &str) -> &'static str {
    match value {
        "registry-unavailable" => "registry-unavailable",
        "source-missing" => "source-missing",
        "source-changed" => "source-changed",
        "ownership-unverified" => "ownership-unverified",
        "invalid-generated-key" => "invalid-generated-key",
        "protection-unavailable" => "protection-unavailable",
        "unsafe-path" => "unsafe-path",
        _ => "storage-unavailable",
    }
}
