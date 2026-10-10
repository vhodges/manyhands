//! Read-only services for the Manyhands front ends.
//!
//! Every read that touches the index goes through `read_session`: the shared
//! index lock, a read-only connection and a transaction that is rolled back.
//! A read returns a DTO or a `ReadError`; neither carries backend text.
//!
//! Four reads touch no index data and use no session: `repository_identity`,
//! `list_remotes_redacted`, the Git part of `inspect_repository` and
//! `new_item_id`. They are never `busy` while the index is locked
//! exclusively. The first two take a resolved repository and, having no
//! session, do not check that its registration still exists either: they
//! answer for a repository removed from Manyhands since it was resolved,
//! where every other read of it is `repository_not_registered`.

// `ReadError` carries its whole scope by value, as the result contract has it.
#![allow(clippy::result_large_err)]

use std::{error::Error, fmt, path::Path};

use rusqlite::{Connection, Transaction, TransactionBehavior};
use time::OffsetDateTime;

use super::{
    RepositoryError, RepositoryErrorKind, RepositoryOperation, RepositoryService, cache_read_guard,
    keys::{KeyMaterialError, KeyMaterialErrorKind},
    open_registry_read_only,
    transport::SshTransportErrorKind,
};
use crate::{
    canonical,
    results::{
        Envelope, ProblemCode, RecoveryAction, RecoveryActionKind, ResultCode, Scope,
        timestamp_string,
    },
};

mod admin;
mod comments;
mod credentials;
mod dto;
mod graph;
mod items;
mod resolve;
mod status;

pub use dto::{
    Accessibility, ChangeSource, ClosureDto, ClosureState, CommentDto, CommentListDto,
    ConfigurationDto, ConfigurationState, CycleDto, CycleKind, CycleListDto, DependencyDirection,
    DependencyDto, DependencyState, DependencyTreeDto, DependencyTreeNodeDto, HostPinDto,
    HostPinListDto, IdentityAvailability, IdentityDto, IdentitySource, IndexProblemDto, IndexState,
    IndexStateDto, IndexStatusDto, IndexStatusState, ItemContextDto, ItemContextKind, ItemDto,
    ItemDtoKind, ItemListDto, KeyDto, KeyListDto, KeyOwnership, KeyPrivateSourceState,
    KeyPublicMetadataState, NewIdDto, OperationAction, OperationDto, OperationFamily,
    OperationListDto, OperationNextAction, OperationOwner, PlanBatchDto, PlanDto, PollingOutcome,
    PollingStatusDto, ProblemDto, PublicKeyDto, ReadinessDto, ReadinessReasonCode,
    ReadinessReasonDto, ReadinessState, RelationshipCheckDto, RelationshipRejectionDto, RemoteDto,
    RemoteListDto, RepositoryInspectionDto, RepositoryListDto, RepositorySummaryDto,
    UnplannableReasonCode, UnplannableReasonDto, UnplannableTicketDto,
};
pub use items::{ClosureFilter, ProposedRelationships, ReadinessFilter, TicketFilter};
pub use resolve::ResolvedRepository;

/// The argument of a recovery action that names a repository root.
const ROOT_ARGUMENT: &str = "root";

/// A recovery action that takes a repository root and nothing else, with
/// the root when one is known. Every action the reads suggest is one, and
/// each is made here, from the registry.
fn root_action(action: RecoveryActionKind, root: Option<&str>) -> RecoveryAction {
    debug_assert_eq!(action.argument_keys(), [ROOT_ARGUMENT]);
    RecoveryAction::new(action, root.map(|root| (ROOT_ARGUMENT, root.into())))
}

/// How far a registration's index is behind its repository.
///
/// `never_refreshed` is a registration with no refresh time and no observed
/// context: nothing has ever been stored for it. An index that holds
/// contexts and no refresh time was written before that time was recorded;
/// it has rows worth listing and is `stale`, with a null `refreshed_at`.
fn index_state(
    refresh_required: bool,
    refreshed_at: Option<i64>,
    has_contexts: bool,
) -> IndexStateDto {
    IndexStateDto {
        state: match refreshed_at {
            None if !has_contexts => IndexState::NeverRefreshed,
            None => IndexState::Stale,
            Some(_) if refresh_required => IndexState::Stale,
            Some(_) => IndexState::Current,
        },
        refreshed_at: refreshed_at
            .and_then(|seconds| OffsetDateTime::from_unix_timestamp(seconds).ok())
            .and_then(timestamp_string),
    }
}

/// Why a read returned no data.
///
/// `Display` prints only the code's fixed message. The source is kept for
/// in-process logging by a front end and is never serialized.
pub struct ReadError {
    code: ResultCode,
    pub scope: Scope,
    pub recovery: Vec<RecoveryAction>,
    source: Option<Box<dyn Error + Send + Sync>>,
}

impl ReadError {
    /// `code` must not be `ResultCode::Ok`; outside a debug build it is
    /// replaced by `internal_error`, so a failure can never report success.
    pub fn new(code: ResultCode) -> Self {
        debug_assert!(
            code != ResultCode::Ok,
            "a read error cannot carry the ok code"
        );
        let code = failure_code(code);
        let recovery = match code {
            ResultCode::IndexUnavailable => {
                vec![root_action(RecoveryActionKind::IndexRebuild, None)]
            }
            _ => Vec::new(),
        };
        Self {
            code,
            scope: Scope::default(),
            recovery,
            source: None,
        }
    }

    /// The caller supplied a string that is not a canonical ULID. Only for
    /// input: an ID that fails to parse in stored content is a `ProblemDto`.
    pub fn invalid_id() -> Self {
        Self::new(ResultCode::InvalidId)
    }

    /// The caller supplied a path that cannot name a target. Only for
    /// input: a bad path found in stored content is a `ProblemDto`.
    pub fn invalid_path() -> Self {
        Self::new(ResultCode::InvalidPath)
    }

    pub fn code(&self) -> ResultCode {
        self.code
    }

    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_recovery(mut self, recovery: Vec<RecoveryAction>) -> Self {
        self.recovery = recovery;
        self
    }

    fn with_source(mut self, source: impl Error + Send + Sync + 'static) -> Self {
        self.source = Some(Box::new(source));
        self
    }

    /// The failure envelope for this error. The caller names the command,
    /// as it does for `Envelope::read_success`.
    pub fn to_envelope<T>(&self, command: impl Into<String>) -> Envelope<T> {
        Envelope::failure(
            command,
            self.scope.clone(),
            self.code,
            self.recovery.clone(),
        )
    }
}

fn failure_code(code: ResultCode) -> ResultCode {
    match code {
        ResultCode::Ok => ResultCode::InternalError,
        code => code,
    }
}

impl fmt::Display for ReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code.message())
    }
}

// Written out so that formatting a read error can never print its source.
impl fmt::Debug for ReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReadError")
            .field("code", &self.code)
            .field("scope", &self.scope)
            .field("recovery", &self.recovery)
            .field("has_source", &self.source.is_some())
            .finish()
    }
}

impl Error for ReadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn Error + 'static))
    }
}

// Each conversion below maps by kind through a `match` with no wildcard arm,
// so a kind added later does not compile until it is mapped. None of them
// formats the error it converts.
//
// There is deliberately no conversion from `canonical::ValidationProblem`:
// the same validation codes are raised for caller input and for stored
// content, and only the call site knows which it parsed. Input uses
// `ReadError::invalid_id` and `ReadError::invalid_path`.

impl From<RepositoryError> for ReadError {
    fn from(error: RepositoryError) -> Self {
        // A SQLite failure means the same thing whether or not it was
        // wrapped on its way here.
        let sqlite = error
            .source
            .as_deref()
            .and_then(|source| source.downcast_ref::<rusqlite::Error>())
            .filter(|_| error.kind == RepositoryErrorKind::Sqlite);
        let code = match sqlite {
            Some(sqlite) => sqlite_error_code(sqlite),
            None => repository_error_code(error.kind),
        };
        Self::new(code).with_source(error)
    }
}

impl From<KeyMaterialError> for ReadError {
    fn from(error: KeyMaterialError) -> Self {
        Self::new(key_material_error_code(error.kind)).with_source(error)
    }
}

impl From<SshTransportErrorKind> for ReadError {
    fn from(kind: SshTransportErrorKind) -> Self {
        Self::new(ssh_transport_error_code(&kind))
    }
}

impl From<rusqlite::Error> for ReadError {
    fn from(error: rusqlite::Error) -> Self {
        Self::new(sqlite_error_code(&error)).with_source(error)
    }
}

/// A failure of the read-only index connection. SQLite's own busy result
/// keeps its meaning; an index file that is missing, corrupt or not a
/// database is unavailable until it is rebuilt; everything else is internal.
fn sqlite_error_code(error: &rusqlite::Error) -> ResultCode {
    match error.sqlite_error_code() {
        Some(rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked) => {
            ResultCode::Busy
        }
        Some(
            rusqlite::ErrorCode::DatabaseCorrupt
            | rusqlite::ErrorCode::NotADatabase
            | rusqlite::ErrorCode::CannotOpen,
        ) => ResultCode::IndexUnavailable,
        _ => ResultCode::InternalError,
    }
}

fn repository_error_code(kind: RepositoryErrorKind) -> ResultCode {
    match kind {
        RepositoryErrorKind::InvalidPath => ResultCode::InvalidPath,
        RepositoryErrorKind::InaccessibleRepository => ResultCode::RepositoryInaccessible,
        RepositoryErrorKind::NotRepository => ResultCode::NotRepository,
        RepositoryErrorKind::BareRepository => ResultCode::BareRepository,
        RepositoryErrorKind::RepositoryNotRegistered
        | RepositoryErrorKind::RepositoryNotEnabled => ResultCode::RepositoryNotRegistered,
        RepositoryErrorKind::IndexUnavailable => ResultCode::IndexUnavailable,
        RepositoryErrorKind::RepositoryBusy => ResultCode::Busy,
        // A backend failure. Its text stays in the source.
        RepositoryErrorKind::Io | RepositoryErrorKind::Sqlite | RepositoryErrorKind::Git => {
            ResultCode::InternalError
        }
        // Shared key registration and selection. Key reads use the session.
        RepositoryErrorKind::InvalidSharedKeyMetadata
        | RepositoryErrorKind::InvalidSharedKeySourcePath
        | RepositoryErrorKind::SharedKeyRegistryUnavailable
        | RepositoryErrorKind::SharedKeyMaterialPending => ResultCode::InternalError,
        // Preconditions of authoring and of branch and worktree changes.
        RepositoryErrorKind::DetachedHead
        | RepositoryErrorKind::WrongCheckedOutBranch
        | RepositoryErrorKind::DirtyWorktree
        | RepositoryErrorKind::ConflictedWorktree
        | RepositoryErrorKind::DirtyConfigurationPath
        | RepositoryErrorKind::InvalidIdentity
        | RepositoryErrorKind::MissingAuthoringTarget
        | RepositoryErrorKind::OccupiedItemPath
        | RepositoryErrorKind::MismatchedAuthoringContext => ResultCode::InternalError,
        // Configuration and remote changes.
        RepositoryErrorKind::InvalidConfiguration
        | RepositoryErrorKind::InvalidPublicationRemote
        | RepositoryErrorKind::UnavailablePublicationRemote
        | RepositoryErrorKind::SelectedRemoteRemoval
        | RepositoryErrorKind::RemoteNameConflict => ResultCode::InternalError,
        // Recoverable operations, which only a mutation begins or resumes.
        RepositoryErrorKind::RegistryRefreshPending
        | RepositoryErrorKind::OperationMismatch
        | RepositoryErrorKind::RecoveryRequired
        | RepositoryErrorKind::ExternalChange
        | RepositoryErrorKind::RollbackIncomplete
        | RepositoryErrorKind::InjectedFailure => ResultCode::InternalError,
    }
}

fn key_material_error_code(kind: KeyMaterialErrorKind) -> ResultCode {
    match kind {
        KeyMaterialErrorKind::Busy => ResultCode::Busy,
        KeyMaterialErrorKind::NotRegistered => ResultCode::KeyNotFound,
        // Reported for any registry failure, not only a degraded index, so
        // it cannot promise that a rebuild is the recovery.
        KeyMaterialErrorKind::RegistryUnavailable => ResultCode::InternalError,
        // Input to generation, import and unlock.
        KeyMaterialErrorKind::InvalidLabel
        | KeyMaterialErrorKind::InvalidPassphrase
        | KeyMaterialErrorKind::ConfirmationRequired => ResultCode::InternalError,
        // Private key files, which no read opens.
        KeyMaterialErrorKind::HomeUnavailable
        | KeyMaterialErrorKind::UnsafePath
        | KeyMaterialErrorKind::ProtectionUnavailable
        | KeyMaterialErrorKind::SourceMissing
        | KeyMaterialErrorKind::SourceUnreadable
        | KeyMaterialErrorKind::NotRegularFile
        | KeyMaterialErrorKind::InvalidGeneratedKey
        | KeyMaterialErrorKind::UnlockFailed
        | KeyMaterialErrorKind::SourceChanged
        | KeyMaterialErrorKind::RandomnessUnavailable
        | KeyMaterialErrorKind::GenerationFailed
        | KeyMaterialErrorKind::StorageUnavailable => ResultCode::InternalError,
        // Deletion and selection.
        KeyMaterialErrorKind::SelectedKeyMustBeCleared
        | KeyMaterialErrorKind::ImportedKey
        | KeyMaterialErrorKind::OwnershipUnverified
        | KeyMaterialErrorKind::SelectionChanged
        | KeyMaterialErrorKind::OperationMismatch => ResultCode::InternalError,
    }
}

/// No read reaches the SSH transport, so every kind is internal. The match
/// is still written out: a kind added later has to be placed here.
fn ssh_transport_error_code(kind: &SshTransportErrorKind) -> ResultCode {
    match kind {
        SshTransportErrorKind::ConfigurationInvalid
        | SshTransportErrorKind::PublicationRemoteMissing
        | SshTransportErrorKind::UsernameRequired
        | SshTransportErrorKind::EndpointChanged => ResultCode::InternalError,
        SshTransportErrorKind::NoSelectedKey
        | SshTransportErrorKind::KeyMissing
        | SshTransportErrorKind::KeyUnreadable
        | SshTransportErrorKind::KeySourceChanged
        | SshTransportErrorKind::SelectionChanged
        | SshTransportErrorKind::KeyInvalidOrUnsupported
        | SshTransportErrorKind::KeyRejected
        | SshTransportErrorKind::UnlockCancelled
        | SshTransportErrorKind::ProviderUnavailable
        | SshTransportErrorKind::UnlockFailed => ResultCode::InternalError,
        SshTransportErrorKind::HostApprovalRequired { .. }
        | SshTransportErrorKind::HostReplacementRequired { .. }
        | SshTransportErrorKind::HostTrustChanged
        | SshTransportErrorKind::HostVerificationUnavailable => ResultCode::InternalError,
        SshTransportErrorKind::RegistryUnavailable
        | SshTransportErrorKind::RuntimeUninitialized
        | SshTransportErrorKind::TransportUnavailable
        | SshTransportErrorKind::RemoteUnavailable
        | SshTransportErrorKind::PushRejected
        | SshTransportErrorKind::ProtocolFailure => ResultCode::InternalError,
    }
}

impl From<&canonical::ValidationCode> for ProblemCode {
    fn from(code: &canonical::ValidationCode) -> Self {
        match code {
            canonical::ValidationCode::InvalidPath => Self::InvalidPath,
            canonical::ValidationCode::MissingFrontMatter => Self::MissingFrontMatter,
            canonical::ValidationCode::MalformedFrontMatter => Self::MalformedFrontMatter,
            canonical::ValidationCode::MalformedConfiguration => Self::MalformedConfiguration,
            canonical::ValidationCode::MissingField => Self::MissingField,
            canonical::ValidationCode::InvalidField => Self::InvalidField,
            canonical::ValidationCode::KindPathMismatch => Self::KindPathMismatch,
            canonical::ValidationCode::DuplicateId => Self::DuplicateId,
            canonical::ValidationCode::MissingCommentItem => Self::MissingCommentItem,
            canonical::ValidationCode::MissingParent => Self::MissingParent,
            canonical::ValidationCode::CrossItemParent => Self::CrossItemParent,
            canonical::ValidationCode::CommentCycle => Self::CommentCycle,
        }
    }
}

impl From<canonical::RelationshipProblemCode> for ProblemCode {
    fn from(code: canonical::RelationshipProblemCode) -> Self {
        match code {
            canonical::RelationshipProblemCode::WrongType => Self::RelationshipWrongType,
            canonical::RelationshipProblemCode::InvalidId => Self::RelationshipInvalidId,
            canonical::RelationshipProblemCode::SelfReference => Self::RelationshipSelfReference,
            canonical::RelationshipProblemCode::DuplicateDependency => Self::DuplicateDependency,
            canonical::RelationshipProblemCode::InvalidSlug => Self::InvalidSlug,
        }
    }
}

impl RepositoryService {
    /// Runs `read` against the index without being able to change it.
    ///
    /// The session holds the shared index lock, bounded as every lease is,
    /// and gives `read` a read-only connection inside a deferred transaction
    /// that is rolled back whatever `read` returns. A lock not obtained in
    /// time is `busy`; a degraded index is `index_unavailable`.
    pub(in crate::repository) fn read_session<T>(
        &self,
        operation: RepositoryOperation,
        read: impl FnOnce(&Connection) -> Result<T, ReadError>,
    ) -> Result<T, ReadError> {
        let data_directory = self
            .registry_path
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let _cache_guard = cache_read_guard(&self.registry_path, data_directory, operation)?;
        self.require_index_available(operation, None)?;
        let connection = open_registry_read_only(&self.registry_path)?;
        // The read-only flag covers the index file only. These two refuse
        // the writes it still allows: temporary tables, and a second
        // database file, which `VACUUM INTO` would otherwise create.
        connection.pragma_update(None, "query_only", true)?;
        forbid_attached_databases(&connection);
        let transaction = Transaction::new_unchecked(&connection, TransactionBehavior::Deferred)?;
        let result = read(&transaction);
        // Dropping the transaction rolls it back as well; this states it.
        let _ = transaction.rollback();
        result
    }

    #[doc(hidden)]
    pub fn read_session_for_testing<T>(
        &self,
        read: impl FnOnce(&Connection) -> Result<T, rusqlite::Error>,
    ) -> Result<T, ReadError> {
        self.read_session(RepositoryOperation::Read, |connection| {
            Ok(read(connection)?)
        })
    }

    /// A new item ID. Nothing is reserved or written; the ID becomes an item
    /// only when a later save uses it.
    pub fn new_item_id(&self) -> NewIdDto {
        NewIdDto {
            id: canonical::ItemId::generate().to_string(),
        }
    }
}

/// Lowers the connection's limit on attached databases to none, so neither
/// `ATTACH` nor `VACUUM INTO` can open a file beside the index.
fn forbid_attached_databases(connection: &Connection) {
    // The return value is the previous limit, which is of no use here.
    let _ = connection.set_limit(rusqlite::limits::Limit::SQLITE_LIMIT_ATTACHED, 0);
}

#[cfg(test)]
mod tests;
