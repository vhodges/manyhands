//! Selected-source inspection and local generated-key validation. No SSH transport.
use super::super::RepositoryService;
use super::storage::{FileIdentity, KeyFileKind};
use super::*;
use rusqlite::{OptionalExtension, params};
use ssh_key::{Algorithm, Cipher, HashAlg, Kdf, PrivateKey};

impl RepositoryService {
    pub fn inspect_selected_key(
        &self,
        store: &KeyStore,
    ) -> Result<SelectedKeyInspection, KeyMaterialError> {
        let Some(registration) = self.selected_key_snapshot()? else {
            return Ok(SelectedKeyInspection::NoSelection);
        };
        let result = match registration.ownership {
            SharedKeyOwnership::Imported => KeySourceToken::observe(&registration.private_key_path)
                .map(|source| SelectedKeyInspection::ImportedReadable {
                    registration: registration.clone(),
                    source,
                }),
            SharedKeyOwnership::Generated => self
                .generated_snapshot(store, &registration, false)
                .map(|(source, _)| SelectedKeyInspection::Generated {
                    registration: registration.clone(),
                    source,
                }),
        };
        result.map_err(|mut e| {
            e.operation = KeyMaterialAction::Inspect;
            e.key_id = Some(registration.id);
            e
        })
    }

    pub fn unlock_generated_key<P: SessionCredentialProvider>(
        &self,
        store: &KeyStore,
        session: &mut SessionCredentials<P>,
    ) -> Result<GeneratedKeyUnlockOutcome, KeyMaterialError> {
        let mut key_id = None;
        let result = (|| {
            let Some(registration) = self.selected_key_snapshot()? else {
                session.clear();
                return Ok(GeneratedKeyUnlockOutcome::NoSelection);
            };
            key_id = Some(registration.id);
            if registration.ownership == SharedKeyOwnership::Imported {
                session.clear();
                return Ok(GeneratedKeyUnlockOutcome::ImportedValidationDeferred(
                    registration,
                ));
            }
            let (source, key) = self.generated_snapshot(store, &registration, true)?;
            let key = key.expect("reading snapshot provides a parsed key");
            validate_generated_profile(&key, &registration)?;
            if !key.is_encrypted() {
                session.clear();
                self.recheck_generated_source(store, &registration, &source)?;
                return Ok(GeneratedKeyUnlockOutcome::Ready(registration));
            }

            // The snapshot contains only owned data. All protected handles and
            // registry/store guards have dropped before the provider is called.
            let mut validation_error = None;
            let result = session.with_passphrase(
                UnlockRequest {
                    key_id: registration.id,
                    label: registration.label.clone(),
                    source: source.clone(),
                },
                |passphrase| {
                    let validated: Result<(), KeyMaterialError> = (|| {
                        self.recheck_generated_source(store, &registration, &source)?;
                        let decrypted = key
                            .decrypt(passphrase.as_bytes())
                            .map_err(|_| error(KeyMaterialErrorKind::UnlockFailed))?;
                        validate_generated_profile(&decrypted, &registration)?;
                        // Decrypted material is dropped here and never escapes.
                        Ok(())
                    })();
                    validated.map_err(|e| {
                        let failure = if e.kind == KeyMaterialErrorKind::UnlockFailed {
                            PassphraseUseFailure::Rejected
                        } else {
                            PassphraseUseFailure::Unavailable
                        };
                        validation_error = Some(e);
                        failure
                    })
                },
            );
            // Also recheck cancelled/unavailable responses and the interval spent
            // decrypting. A stale success must evict even a newly cached secret.
            self.recheck_generated_source(store, &registration, &source)?;
            if let Some(e) = validation_error {
                return Err(e);
            }
            match result {
                Ok(()) => Ok(GeneratedKeyUnlockOutcome::Ready(registration)),
                Err(SessionUnlockFailure::Cancelled) => Ok(GeneratedKeyUnlockOutcome::Cancelled),
                Err(SessionUnlockFailure::ProviderUnavailable) => {
                    Ok(GeneratedKeyUnlockOutcome::ProviderUnavailable)
                }
                Err(_) => Err(error(KeyMaterialErrorKind::UnlockFailed)),
            }
        })();
        result.map_err(|mut e| {
            session.clear();
            e.operation = KeyMaterialAction::Unlock;
            e.key_id = key_id;
            e
        })
    }

    fn selected_key_snapshot(&self) -> Result<Option<SharedKeyRegistration>, KeyMaterialError> {
        self.list_shared_keys()
            .map(|rows| rows.into_iter().find(|r| r.selected))
            .map_err(|_| error(KeyMaterialErrorKind::RegistryUnavailable))
    }

    fn generated_snapshot(
        &self,
        store: &KeyStore,
        registration: &SharedKeyRegistration,
        read_key: bool,
    ) -> Result<(KeySourceToken, Option<PrivateKey>), KeyMaterialError> {
        let (private_path, public_path) = store.key_paths(registration.id);
        if registration.private_key_path != private_path
            || registration.public_key_path.as_ref() != Some(&public_path)
        {
            return Err(error(KeyMaterialErrorKind::UnsafePath));
        }
        // Creation evidence proves provenance. Historical file freshness is
        // required by deletion/replay; unlock instead checks the current source
        // profile, recorded public identity, and freshness across this attempt.
        self.material_registry(|connection| {
            let identities = connection.query_row(
                "SELECT private_file_identity, public_file_identity FROM owned_generated_keys WHERE key_id=?1 AND private_key_path=?2 AND public_key_path=?3 AND public_key_fingerprint=?4",
                params![registration.id.to_string(), private_path.to_str(), public_path.to_str(), registration.public_key_fingerprint],
                |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Vec<u8>>(1)?)),
            ).optional().map_err(|_| error(KeyMaterialErrorKind::RegistryUnavailable))?
                .ok_or_else(|| error(KeyMaterialErrorKind::OwnershipUnverified))?;
            for identity in [identities.0, identities.1] {
                FileIdentity::decode(std::str::from_utf8(&identity)
                    .map_err(|_| error(KeyMaterialErrorKind::OwnershipUnverified))?)?;
            }
            Ok(())
        })?;
        let guard = store.lock()?;
        let mut private = guard
            .open_owned(registration.id, KeyFileKind::Private)?
            .ok_or_else(|| error(KeyMaterialErrorKind::SourceMissing))?;
        let _public = guard
            .open_owned(registration.id, KeyFileKind::Public)?
            .ok_or_else(|| error(KeyMaterialErrorKind::SourceMissing))?;
        let source = KeySourceToken::from_owned(private_path, private.identity());
        let key = if read_key {
            let bytes = private.read_secret()?;
            Some(
                PrivateKey::from_openssh(&*bytes)
                    .map_err(|_| error(KeyMaterialErrorKind::InvalidGeneratedKey))?,
            )
        } else {
            None
        };
        Ok((source, key))
    }

    fn recheck_generated_source(
        &self,
        store: &KeyStore,
        registration: &SharedKeyRegistration,
        source: &KeySourceToken,
    ) -> Result<(), KeyMaterialError> {
        if self.selected_key_snapshot()?.as_ref() != Some(registration) {
            return Err(error(KeyMaterialErrorKind::SelectionChanged));
        }
        let (current, _) = self
            .generated_snapshot(store, registration, false)
            .map_err(|_| error(KeyMaterialErrorKind::SourceChanged))?;
        if current != *source {
            return Err(error(KeyMaterialErrorKind::SourceChanged));
        }
        Ok(())
    }
}

fn validate_generated_profile(
    key: &PrivateKey,
    registration: &SharedKeyRegistration,
) -> Result<(), KeyMaterialError> {
    let profile_matches = if key.is_encrypted() {
        key.cipher() == Cipher::Aes256Ctr
            && matches!(key.kdf(), Kdf::Bcrypt { salt, rounds: 16 } if salt.len() == 16)
    } else {
        key.cipher() == Cipher::None && *key.kdf() == Kdf::None
    };
    if !profile_matches
        || key.algorithm() != Algorithm::Ed25519
        || Some(key.public_key().fingerprint(HashAlg::Sha256).to_string())
            != registration.public_key_fingerprint
    {
        return Err(error(KeyMaterialErrorKind::InvalidGeneratedKey));
    }
    Ok(())
}
fn error(kind: KeyMaterialErrorKind) -> KeyMaterialError {
    KeyMaterialError {
        operation: KeyMaterialAction::Inspect,
        key_id: None,
        operation_id: None,
        kind,
    }
}
