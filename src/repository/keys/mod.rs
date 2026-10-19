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
pub(crate) use registry::observed_setting;
pub(super) use registry::{
    StoredSharedKeysError, bounded_public_key_contents, completed_generation,
    lookup_material_operation, migrate_material_schema, openssh_public_key, public_key_fingerprint,
    recorded_registry_change, stored_shared_key_registrations,
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

/// What a recorded change to the key registry was asked to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyRegistryAction {
    Import,
    Select,
    Clear,
}

/// A setting of the key registry that a change is made to, and that a
/// request observes before it is accepted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyRegistrySetting {
    /// Which key is selected, or that none is.
    Selection,
    /// Which key is registered for a private key file, or that none is.
    Source,
}

/// What a change to the key registry is made under, for a caller that
/// may make it again: the mutation boundary, on a retry of a request.
pub(crate) struct KeyRegistryMark {
    /// The operation the change is recorded against, in the transaction
    /// that makes it. A change already recorded against it is not made
    /// again: the call is answered with what the record holds.
    pub(crate) operation_id: OperationId,
    /// The setting as the request observed it before it was accepted;
    /// see `observed_setting`. When it is given, it is compared with the
    /// setting in the transaction that would change it, and a setting
    /// that is neither this nor already as asked is not changed. `None`
    /// makes the change whatever the setting is.
    pub(crate) expected: Option<super::ExpectedPathObservation>,
}

/// A change to the key registry as it was recorded against its
/// operation, in the transaction that made it: what a retry is answered
/// with, whatever has been done to the key or to the selection since.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KeyRegistryChange {
    pub(crate) action: KeyRegistryAction,
    /// The key that was registered or selected. `None` for a clearing.
    pub(crate) key_id: Option<SharedKeyId>,
    /// The label and the public key fingerprint of that key's
    /// registration as they were then: what the result gave of it.
    pub(crate) label: Option<String>,
    pub(crate) public_key_fingerprint: Option<String>,
    /// Whether anything changed. `false` for a change that found the
    /// registry already as it was asked to be.
    pub(crate) changed: bool,
}

/// What a change made under a `KeyRegistryMark` came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MarkedRegistryChange {
    /// The change as it is recorded against the operation: made by this
    /// call, or by an earlier one under the same operation.
    Recorded(KeyRegistryChange),
    /// The setting is neither as the mark expects it nor as the change
    /// asks: someone changed it since. Nothing was changed or recorded.
    Moved,
    /// The key a selection names is not registered. Nothing was changed
    /// or recorded.
    NotRegistered,
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
