use std::{fmt, path::PathBuf};

use super::OperationId;

mod deletion;
pub use deletion::GeneratedKeyDeletionReview;
mod generation;
mod inspection;
mod registry;
pub mod session;
pub use session::{
    InvalidPassphrase, KeySourceToken, PassphraseResponse, PassphraseUseFailure, SecretPassphrase,
    SessionCredentialProvider, SessionCredentials, SessionUnlockFailure, UnlockReason,
    UnlockRequest,
};
pub(crate) mod storage;
pub use storage::KeyStore;

#[cfg(test)]
pub(super) use generation::failure_code as stored_failure_code;
pub(super) use registry::{
    StoredSharedKeysError, bounded_public_key_contents, lookup_material_operation,
    migrate_material_schema, openssh_public_key, public_key_fingerprint,
    stored_shared_key_registrations,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SharedKeyId(ulid::Ulid);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SharedKeyIdParseError;

impl SharedKeyId {
    pub fn new() -> Self {
        Self(ulid::Ulid::new())
    }

    pub fn parse(value: &str) -> Result<Self, SharedKeyIdParseError> {
        let id: ulid::Ulid = value.parse().map_err(|_| SharedKeyIdParseError)?;
        (id.to_string() == value)
            .then_some(Self(id))
            .ok_or(SharedKeyIdParseError)
    }
}

impl Default for SharedKeyId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SharedKeyId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl fmt::Display for SharedKeyIdParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("shared key ID must be a canonical uppercase ULID")
    }
}

impl std::error::Error for SharedKeyIdParseError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharedKeyOwnership {
    Imported,
    Generated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrivateKeySourceState {
    Available,
    Missing,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicKeyMetadataState {
    NotProvided,
    FingerprintAvailable,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedKeyRegistration {
    pub id: SharedKeyId,
    pub label: String,
    pub ownership: SharedKeyOwnership,
    pub private_key_path: PathBuf,
    pub public_key_path: Option<PathBuf>,
    pub public_key_fingerprint: Option<String>,
    pub private_source_state: PrivateKeySourceState,
    pub public_metadata_state: PublicKeyMetadataState,
    pub selected: bool,
}

#[derive(Clone, Debug)]
pub struct RegisterSharedKeyRequest {
    pub label: String,
    pub ownership: SharedKeyOwnership,
    pub private_key_path: PathBuf,
    pub public_key_path: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegisterSharedKeyOutcome {
    Registered(SharedKeyRegistration),
    SourceAlreadyRegistered { existing: SharedKeyId },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SharedKeySelectionOutcome {
    Selected(SharedKeyRegistration),
    Cleared,
    AlreadyCleared,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnregisterSharedKeyOutcome {
    Unregistered,
    NotRegistered,
    SelectedKeyMustBeCleared,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GeneratedKeyDeletionPreflight {
    ConfirmationRequired(SharedKeyRegistration),
    NotRegistered,
    ImportedKey,
    SelectedKeyMustBeCleared,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyMaterialAction {
    Generate,
    Inspect,
    Unlock,
    ReviewDeletion,
    Delete,
    ListRecovery,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyMaterialErrorKind {
    InvalidLabel,
    InvalidPassphrase,
    HomeUnavailable,
    RegistryUnavailable,
    Busy,
    NotRegistered,
    SelectedKeyMustBeCleared,
    ImportedKey,
    OwnershipUnverified,
    UnsafePath,
    ProtectionUnavailable,
    SourceMissing,
    SourceUnreadable,
    NotRegularFile,
    InvalidGeneratedKey,
    UnlockFailed,
    SourceChanged,
    SelectionChanged,
    OperationMismatch,
    ConfirmationRequired,
    RandomnessUnavailable,
    GenerationFailed,
    StorageUnavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyMaterialError {
    pub operation: KeyMaterialAction,
    pub key_id: Option<SharedKeyId>,
    pub operation_id: Option<OperationId>,
    pub kind: KeyMaterialErrorKind,
}

impl KeyMaterialError {
    pub fn guidance(&self) -> &'static str {
        match self.kind {
            KeyMaterialErrorKind::InvalidLabel => "enter a non-empty key label",
            KeyMaterialErrorKind::InvalidPassphrase => {
                "use a non-empty passphrase that meets the key protection requirements"
            }
            KeyMaterialErrorKind::HomeUnavailable => {
                "choose an absolute user home before accessing generated keys"
            }
            KeyMaterialErrorKind::RegistryUnavailable => {
                "the key registry is unavailable; close other operations and try again"
            }
            KeyMaterialErrorKind::Busy => {
                "another key material operation is active; wait and try again"
            }
            KeyMaterialErrorKind::NotRegistered => "the shared key is not registered",
            KeyMaterialErrorKind::SelectedKeyMustBeCleared => {
                "clear the selected shared key before continuing"
            }
            KeyMaterialErrorKind::ImportedKey => {
                "this operation is available only for generated keys"
            }
            KeyMaterialErrorKind::OwnershipUnverified => {
                "ownership evidence is unavailable; inspect the retained files manually"
            }
            KeyMaterialErrorKind::UnsafePath => {
                "the generated key path is outside the protected key store"
            }
            KeyMaterialErrorKind::ProtectionUnavailable => {
                "secure storage protection is unavailable on this system"
            }
            KeyMaterialErrorKind::SourceMissing => "the selected key source is missing",
            KeyMaterialErrorKind::SourceUnreadable => "the selected key source is not readable",
            KeyMaterialErrorKind::NotRegularFile => "the selected key source is not a regular file",
            KeyMaterialErrorKind::InvalidGeneratedKey => {
                "the generated key material is invalid; inspect the retained files"
            }
            KeyMaterialErrorKind::UnlockFailed => {
                "the generated key could not be unlocked; verify the passphrase and try again"
            }
            KeyMaterialErrorKind::SourceChanged => {
                "the key source changed during the operation; inspect it and try again"
            }
            KeyMaterialErrorKind::SelectionChanged => {
                "the selected key changed during the operation; inspect the current selection"
            }
            KeyMaterialErrorKind::OperationMismatch => {
                "the operation ID belongs to different key material work; use a fresh operation ID"
            }
            KeyMaterialErrorKind::ConfirmationRequired => {
                "review the generated key deletion again before confirming"
            }
            KeyMaterialErrorKind::RandomnessUnavailable => {
                "secure randomness is unavailable; retry on a healthy system"
            }
            KeyMaterialErrorKind::GenerationFailed => {
                "key generation failed; retry with a fresh operation ID"
            }
            KeyMaterialErrorKind::StorageUnavailable => {
                "the protected key store is unavailable; inspect its permissions and try again"
            }
        }
    }
}

impl fmt::Display for KeyMaterialError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.guidance())
    }
}

impl std::error::Error for KeyMaterialError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyMaterialPhase {
    Reserved,
    PrivateWritten,
    PairWritten,
    Prepared,
    PrivateRemoved,
    FilesRemoved,
    Completed,
    RetainedForInspection,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyMaterialFailureCode(String);

impl KeyMaterialFailureCode {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryAction {
    RetryGeneration,
    ReviewDeletionAgain,
    InspectRetainedFiles,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyMaterialRecovery {
    pub operation_id: OperationId,
    pub key_id: SharedKeyId,
    pub action: KeyMaterialAction,
    pub phase: KeyMaterialPhase,
    pub failure_code: Option<KeyMaterialFailureCode>,
    pub recovery_action: RecoveryAction,
}

/// Protection values can own a secret and cannot be serialized.
///
/// ```compile_fail
/// # use manyhands::repository::keys::KeyProtection;
/// # fn assert_serialize<T: serde::Serialize>() {}
/// assert_serialize::<KeyProtection>();
/// ```
#[derive(Debug)]
pub enum KeyProtection {
    Unencrypted,
    Passphrase(SecretPassphrase),
}

/// Generation requests can own protected credentials and cannot be serialized.
///
/// ```compile_fail
/// # use manyhands::repository::keys::GenerateSharedKeyRequest;
/// # fn assert_serialize<T: serde::Serialize>() {}
/// assert_serialize::<GenerateSharedKeyRequest>();
/// ```
#[derive(Debug)]
pub struct GenerateSharedKeyRequest {
    pub operation_id: OperationId,
    pub label: String,
    pub protection: KeyProtection,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GenerateSharedKeyOutcome {
    Created(SharedKeyRegistration),
    AlreadyCreated(SharedKeyRegistration),
    RecoveryRequired(KeyMaterialRecovery),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectedKeyInspection {
    NoSelection,
    ImportedReadable {
        registration: SharedKeyRegistration,
        source: KeySourceToken,
    },
    Generated {
        registration: SharedKeyRegistration,
        source: KeySourceToken,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GeneratedKeyUnlockOutcome {
    Ready(SharedKeyRegistration),
    NoSelection,
    ImportedValidationDeferred(SharedKeyRegistration),
    Cancelled,
    ProviderUnavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeleteGeneratedKeyOutcome {
    Deleted,
    AlreadyDeleted,
    Cancelled,
    RecoveryRequired(KeyMaterialRecovery),
}
