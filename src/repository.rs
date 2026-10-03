use std::{
    collections::{BTreeSet, HashMap},
    fmt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock, Weak},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use git2::{
    BranchType, Config, ConfigLevel, Index, IndexEntry, Repository, RepositoryInitOptions,
    Signature, Status, StatusOptions, WorktreeAddOptions,
};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use time::OffsetDateTime;

use crate::canonical;

mod coordination;
mod discovery;
mod recovery;

use coordination::{
    BootstrapLease, CacheReadGuard, CacheWriteGuard, RepositoryLease, bootstrap_lease,
    cache_read_guard, cache_write_guard, repository_lease,
};
use discovery::{
    ObservedCommentThread, ObservedItem, RootConfiguration, RootObservation,
    RootObservationProblem, migrate_registry, observe_root, open_registry, open_registry_read_only,
};
use recovery::{
    RecoveryRecord, advance_after_observation, begin_or_reconcile_operation, pending_for_root,
    record_persisted_context as record_recovery_context,
};

pub const REGISTRY_FILE: &str = "manyhands.sqlite3";
const REGISTRY_BUSY_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_DOCUMENT_DIRECTORY_DEPTH: usize = 16;
const MAX_DOCUMENT_DIRECTORY_ENTRIES: usize = 1024;
const MAX_MANAGED_DIRECTORY_DEPTH: usize = 1;
const MAX_MANAGED_DIRECTORY_ENTRIES: usize = 1024;

type RepositoryOperationLock = Arc<Mutex<()>>;
type RepositoryOperationLockMap = HashMap<(PathBuf, PathBuf), Weak<Mutex<()>>>;

fn process_repository_operation_lock(registry_path: &Path, root: &Path) -> RepositoryOperationLock {
    static OPERATION_LOCKS: OnceLock<Mutex<RepositoryOperationLockMap>> = OnceLock::new();
    let locks = OPERATION_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut locks = locks.lock().unwrap_or_else(|error| error.into_inner());
    locks.retain(|_, lock| lock.strong_count() != 0);
    let key = (registry_path.to_owned(), root.to_owned());
    if let Some(lock) = locks.get(&key).and_then(Weak::upgrade) {
        return lock;
    }
    let lock = Arc::new(Mutex::new(()));
    locks.insert(key, Arc::downgrade(&lock));
    lock
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OperationId(ulid::Ulid);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationIdParseError;

impl OperationId {
    pub fn new() -> Self {
        Self(ulid::Ulid::new())
    }

    pub fn parse(value: &str) -> Result<Self, OperationIdParseError> {
        let id: ulid::Ulid = value.parse().map_err(|_| OperationIdParseError)?;
        (id.to_string() == value)
            .then_some(Self(id))
            .ok_or(OperationIdParseError)
    }
}

impl Default for OperationId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for OperationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl fmt::Display for OperationIdParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("operation ID must be a canonical uppercase ULID")
    }
}

impl std::error::Error for OperationIdParseError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExpectedPathObservation {
    Missing,
    Blake3([u8; 32]),
}

impl ExpectedPathObservation {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self::Blake3(*blake3::hash(bytes).as_bytes())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexPending<T> {
    pub authoritative: T,
}

impl<T> IndexPending<T> {
    pub fn new(authoritative: T) -> Self {
        Self { authoritative }
    }
}

pub struct RepositoryService {
    registry_path: PathBuf,
    availability: Mutex<IndexAvailability>,
    failure_point: Mutex<Option<FailurePoint>>,
    observation_hook: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    registration_git_hook: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    corrupt_cache_decision_hook: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    corrupt_cache_critical_hook: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    rebuild_error_hook: Mutex<Option<Box<dyn FnOnce() + Send>>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IndexAvailability {
    Ready,
    Degraded,
}

#[derive(Clone)]
pub struct CommitIdentity {
    pub name: String,
    pub email: String,
}

#[derive(Clone)]
pub struct CreateRepositoryRequest {
    pub root: PathBuf,
    pub primary_branch: String,
    pub identity: Option<CommitIdentity>,
    pub operation_id: OperationId,
}

#[derive(Clone)]
pub struct EnableRepositoryRequest {
    pub root: PathBuf,
    pub primary_branch: String,
    pub identity: Option<CommitIdentity>,
    pub operation_id: OperationId,
}

#[derive(Clone)]
pub struct AddRemoteRequest {
    pub root: PathBuf,
    pub name: String,
    pub url: String,
    pub operation_id: OperationId,
}

#[derive(Clone)]
pub struct SetPublicationRemoteRequest {
    pub root: PathBuf,
    pub name: Option<String>,
    pub operation_id: OperationId,
}

#[derive(Clone)]
pub struct RemoveRemoteRequest {
    pub root: PathBuf,
    pub name: String,
    pub operation_id: OperationId,
}

#[derive(Clone)]
pub struct RemoveRegistrationRequest {
    pub root: PathBuf,
    pub operation_id: OperationId,
}

#[derive(Clone)]
pub struct RefreshRepositoryRequest {
    pub root: PathBuf,
    pub operation_id: OperationId,
}

#[derive(Clone)]
pub struct RebuildRepositoryRequest {
    pub root: PathBuf,
    pub operation_id: OperationId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecoveryInspection {
    Pending {
        operation_id: OperationId,
        operation: RepositoryOperation,
        root: PathBuf,
        item_id: Option<canonical::ItemId>,
        context: Option<PathBuf>,
        completed_step: Option<String>,
        next_action: RepositoryOperation,
    },
    LegacyIndexOperation {
        root: PathBuf,
        operation: RepositoryOperation,
        next_action: RepositoryOperation,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthoringKind {
    Document,
    Ticket,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextIntent {
    Create,
    Edit,
}

#[derive(Clone)]
pub struct AuthoringTarget {
    pub root: PathBuf,
    pub kind: AuthoringKind,
    pub item_id: canonical::ItemId,
    pub intent: ContextIntent,
    pub operation_id: OperationId,
}

pub struct ItemContext {
    pub root: PathBuf,
    pub kind: AuthoringKind,
    pub item_id: canonical::ItemId,
    pub branch: String,
    pub worktree: PathBuf,
}

pub enum ContextProvisionOutcome {
    Created(ItemContext),
    Reused(ItemContext),
}

#[derive(Clone)]
pub struct DocumentDraft {
    pub title: String,
    pub body: String,
}

#[derive(Clone)]
pub struct TicketDraft {
    pub title: String,
    pub ticket_type: String,
    pub status: String,
    pub project: Option<String>,
    pub team: Option<String>,
    pub body: String,
}

#[derive(Clone)]
pub struct SaveDocumentRequest {
    pub target: AuthoringTarget,
    pub source_path: Option<PathBuf>,
    pub destination_path: PathBuf,
    pub draft: DocumentDraft,
    pub expected_source: Option<ExpectedPathObservation>,
    pub expected_destination: ExpectedPathObservation,
}

#[derive(Clone)]
pub struct SaveTicketRequest {
    pub target: AuthoringTarget,
    pub draft: TicketDraft,
    pub expected_path: ExpectedPathObservation,
}

#[derive(Clone)]
pub struct SubmitCommentRequest {
    pub target: AuthoringTarget,
    pub comment_id: canonical::ItemId,
    pub parent_id: Option<canonical::ItemId>,
    pub body: String,
    pub expected_destination: ExpectedPathObservation,
}

pub enum LocalCheckpoint {
    Checkpointed { commit_oid: git2::Oid },
    NoChange,
    RefreshPending { commit_oid: git2::Oid },
}

pub enum SaveOutcome {
    IdentityRequired {
        context: ItemContext,
    },
    Saved {
        context: ItemContext,
        checkpoint: LocalCheckpoint,
    },
}

pub enum CommentPublicationState {
    PublishPending,
    SyncDeferred,
}

pub enum CommentSubmissionOutcome {
    IdentityRequired {
        context: ItemContext,
    },
    Saved {
        context: ItemContext,
        checkpoint: LocalCheckpoint,
        publication: CommentPublicationState,
    },
}

#[derive(Debug, PartialEq)]
pub struct RepositoryInspection {
    pub root: PathBuf,
    pub head_branch: Option<String>,
    pub local_branches: Vec<String>,
    pub configuration: ConfigurationInspection,
    pub identity: IdentityInspection,
    pub remotes: Vec<RemoteInfo>,
}

#[derive(Debug, PartialEq)]
pub enum ConfigurationInspection {
    Missing,
    Valid(canonical::RepositoryConfig),
    Invalid(canonical::ValidationProblem),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityInspection {
    Available,
    Required,
}

#[derive(Debug, PartialEq, Eq)]
pub struct RemoteInfo {
    pub name: String,
    pub fetch_url: String,
    pub push_url: String,
    pub publication_eligible: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum EnableRepositoryOutcome {
    Enabled { commit_oid: git2::Oid },
    AlreadyEnabled,
    IdentityRequired,
    RegistrationPending { commit_oid: git2::Oid },
}

#[derive(Debug, PartialEq, Eq)]
pub enum RemoteOutcome {
    Changed,
    NoChange,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PublicationRemoteOutcome {
    Changed { commit_oid: git2::Oid },
    NoChange,
    RegistrationPending { commit_oid: git2::Oid },
}

#[derive(Debug, PartialEq, Eq)]
pub enum RemoveRegistrationOutcome {
    Removed,
    NotRegistered,
}

#[derive(Debug, PartialEq, Eq)]
pub struct RepositorySnapshot {
    pub root: PathBuf,
    pub configuration: SnapshotConfiguration,
    pub refresh_required: bool,
    pub contexts: Vec<DiscoveredContext>,
    pub items: Vec<DiscoveredItem>,
    pub problems: Vec<DiscoveryProblem>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SnapshotConfiguration {
    Valid {
        primary_branch: String,
        publication_remote: Option<String>,
    },
    Invalid {
        code: canonical::ValidationCode,
        guidance: String,
    },
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryContextKind {
    Primary,
    Unverified,
    Active,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryActivitySource {
    GitCommit,
    UncommittedFilesystem,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DiscoveredContext {
    pub kind: DiscoveryContextKind,
    pub branch: Option<String>,
    pub worktree: PathBuf,
    pub item_id: Option<canonical::ItemId>,
    pub head_oid: Option<git2::Oid>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DiscoveredItem {
    pub context: PathBuf,
    pub id: canonical::ItemId,
    pub kind: AuthoringKind,
    pub path: PathBuf,
    pub title: String,
    pub ticket_type: Option<String>,
    pub status: Option<String>,
    pub project: Option<String>,
    pub team: Option<String>,
    pub closed_at: Option<OffsetDateTime>,
    pub activity_at: OffsetDateTime,
    pub activity_source: DiscoveryActivitySource,
    pub comments: Vec<DiscoveredCommentThread>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DiscoveredCommentThread {
    pub id: canonical::ItemId,
    pub path: PathBuf,
    pub created_at: OffsetDateTime,
    pub replies: Vec<DiscoveredCommentThread>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DiscoveryProblem {
    pub context: Option<PathBuf>,
    pub path: Option<PathBuf>,
    pub code: String,
    pub guidance: String,
    pub observed_at: OffsetDateTime,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RefreshOutcome {
    Refreshed {
        snapshot: RepositorySnapshot,
    },
    RetryRequired {
        root: PathBuf,
        context: Option<PathBuf>,
    },
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryConnectionPhase {
    BeforeWal,
    AfterWal,
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailurePoint {
    BeforeRepositoryInitialization,
    AfterRepositoryInitialization,
    BeforeConfigurationWrite,
    BeforeInitializationCommit,
    BeforePublicationConfigurationCommit,
    BeforeContextBranchCreation,
    BeforeWorktreeCreation,
    BeforeItemWrite,
    BeforeCheckpointCommit,
    BeforeRegistryWrite,
    BeforeRegistrationRemovalTransaction,
    AfterRemoteMutation,
    AfterContextObservation,
    BeforeIndexTransactionCommit,
    BeforeCorruptCacheReplacement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RepositoryOperation {
    OpenRegistry,
    Inspect,
    CreateAndEnable,
    Enable,
    RemoveRegistration,
    ListRemotes,
    AddRemote,
    RemoveRemote,
    SetPublicationRemote,
    PrepareContext,
    SaveDocument,
    SaveTicket,
    SubmitComment,
    RefreshRepository,
    RebuildRepository,
    RepositorySnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RepositoryErrorKind {
    InvalidPath,
    InaccessibleRepository,
    NotRepository,
    BareRepository,
    DetachedHead,
    WrongCheckedOutBranch,
    DirtyWorktree,
    ConflictedWorktree,
    InvalidConfiguration,
    InvalidPublicationRemote,
    UnavailablePublicationRemote,
    SelectedRemoteRemoval,
    RemoteNameConflict,
    RegistryRefreshPending,
    RepositoryNotRegistered,
    IndexUnavailable,
    RepositoryBusy,
    OperationMismatch,
    RecoveryRequired,
    ExternalChange,
    DirtyConfigurationPath,
    InvalidIdentity,
    RepositoryNotEnabled,
    MissingAuthoringTarget,
    OccupiedItemPath,
    MismatchedAuthoringContext,
    RollbackIncomplete,
    Io,
    Sqlite,
    Git,
    InjectedFailure,
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeaseKind {
    Repository,
    Bootstrap,
    CacheRead,
    CacheWrite,
}

impl LeaseKind {
    #[doc(hidden)]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "repository" => Some(Self::Repository),
            "bootstrap" => Some(Self::Bootstrap),
            "cache-read" => Some(Self::CacheRead),
            "cache-write" => Some(Self::CacheWrite),
            _ => None,
        }
    }

    #[doc(hidden)]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Repository => "repository",
            Self::Bootstrap => "bootstrap",
            Self::CacheRead => "cache-read",
            Self::CacheWrite => "cache-write",
        }
    }
}

#[doc(hidden)]
pub struct LeaseHolderForTesting(LeaseGuardForTesting);

#[allow(dead_code)]
enum LeaseGuardForTesting {
    Repository(RepositoryLease),
    Bootstrap(BootstrapLease),
    CacheRead(CacheReadGuard),
    CacheWrite(CacheWriteGuard),
}

impl Drop for LeaseHolderForTesting {
    fn drop(&mut self) {
        match &self.0 {
            LeaseGuardForTesting::Repository(_)
            | LeaseGuardForTesting::Bootstrap(_)
            | LeaseGuardForTesting::CacheRead(_)
            | LeaseGuardForTesting::CacheWrite(_) => {}
        }
    }
}

#[derive(Debug)]
pub struct RepositoryError {
    pub root: Option<PathBuf>,
    pub operation: RepositoryOperation,
    pub kind: RepositoryErrorKind,
    message: String,
    source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}

impl RepositoryError {
    fn io(operation: RepositoryOperation, root: Option<PathBuf>, error: std::io::Error) -> Self {
        Self::with_source(operation, root, RepositoryErrorKind::Io, error)
    }

    fn sqlite(error: rusqlite::Error) -> Self {
        Self::with_source(
            RepositoryOperation::OpenRegistry,
            None,
            RepositoryErrorKind::Sqlite,
            error,
        )
    }

    fn git(operation: RepositoryOperation, root: Option<PathBuf>, error: git2::Error) -> Self {
        Self::with_source(operation, root, RepositoryErrorKind::Git, error)
    }

    fn new(
        operation: RepositoryOperation,
        root: Option<PathBuf>,
        kind: RepositoryErrorKind,
        error: impl fmt::Display,
    ) -> Self {
        Self {
            root,
            operation,
            kind,
            message: error.to_string(),
            source: None,
        }
    }

    fn with_source(
        operation: RepositoryOperation,
        root: Option<PathBuf>,
        kind: RepositoryErrorKind,
        error: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            root,
            operation,
            kind,
            message: error.to_string(),
            source: Some(Box::new(error)),
        }
    }

    fn for_operation(mut self, operation: RepositoryOperation, root: &Path) -> Self {
        self.operation = operation;
        self.root = Some(root.to_owned());
        self
    }
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "repository operation {:?} failed ({:?}): {}",
            self.operation, self.kind, self.message
        )
    }
}

impl std::error::Error for RepositoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn std::error::Error + 'static))
    }
}

struct CheckpointSpec<'a> {
    operation: RepositoryOperation,
    subject: &'a str,
    added_paths: &'a [&'a Path],
    removed_source: Option<&'a Path>,
}

impl RepositoryService {
    pub fn open_default() -> Result<Self, RepositoryError> {
        Self::open_at(&default_data_directory()?)
    }

    pub fn open_at(data_directory: &Path) -> Result<Self, RepositoryError> {
        Self::open_at_with_registry_observer(data_directory, |_| {})
    }

    #[doc(hidden)]
    pub fn hold_lease_for_testing(
        root: &Path,
        data_directory: &Path,
        kind: LeaseKind,
    ) -> Result<LeaseHolderForTesting, RepositoryError> {
        let operation = RepositoryOperation::Inspect;
        let root = if kind == LeaseKind::Bootstrap {
            let parent = root.parent().ok_or_else(|| {
                RepositoryError::new(
                    operation,
                    Some(root.to_owned()),
                    RepositoryErrorKind::InvalidPath,
                    "the bootstrap target has no parent directory",
                )
            })?;
            let name = root.file_name().ok_or_else(|| {
                RepositoryError::new(
                    operation,
                    Some(root.to_owned()),
                    RepositoryErrorKind::InvalidPath,
                    "the bootstrap target must name a directory",
                )
            })?;
            std::fs::canonicalize(parent)
                .map_err(|error| RepositoryError::io(operation, Some(root.to_owned()), error))?
                .join(name)
        } else {
            std::fs::canonicalize(root)
                .map_err(|error| RepositoryError::io(operation, Some(root.to_owned()), error))?
        };
        let guard = match kind {
            LeaseKind::Repository => {
                let repository = Repository::discover(&root)
                    .map_err(|error| RepositoryError::git(operation, Some(root.clone()), error))?;
                LeaseGuardForTesting::Repository(repository_lease(&repository, &root, operation)?)
            }
            LeaseKind::Bootstrap => {
                LeaseGuardForTesting::Bootstrap(bootstrap_lease(data_directory, &root, operation)?)
            }
            LeaseKind::CacheRead => LeaseGuardForTesting::CacheRead(cache_read_guard(
                &data_directory.join(REGISTRY_FILE),
                &root,
                operation,
            )?),
            LeaseKind::CacheWrite => LeaseGuardForTesting::CacheWrite(cache_write_guard(
                &data_directory.join(REGISTRY_FILE),
                &root,
                operation,
            )?),
        };
        Ok(LeaseHolderForTesting(guard))
    }

    pub fn refresh_repository(
        &self,
        request: RefreshRepositoryRequest,
    ) -> Result<RefreshOutcome, RepositoryError> {
        let root = &request.root;
        let operation = RepositoryOperation::RefreshRepository;
        self.require_index_available(operation, Some(root))?;
        let (repository, root) = canonical_repository_root(root, operation)?;
        // Only establish durable recovery state while holding the common Git lease.
        // Filesystem and Git observation intentionally happens after this scope.
        let (repository_id, record) = {
            let _repository_lease = repository_lease(&repository, &root, operation)?;
            (
                registered_repository_id(&self.registry_path, &root, operation)?,
                begin_operation(&self.registry_path, &root, operation, request.operation_id)?,
            )
        };
        let operation_id = record.id;
        let before = observe_root(&repository, &root);
        if let Err(error) =
            self.check_failure(FailurePoint::AfterContextObservation, operation, &root)
        {
            set_refresh_operation(&self.registry_path, operation_id, "failed", None, &root)?;
            return Err(error);
        }
        if let Some(hook) = self
            .observation_hook
            .lock()
            .map_err(|_| {
                RepositoryError::new(
                    operation,
                    Some(root.clone()),
                    RepositoryErrorKind::InjectedFailure,
                    "the test observation hook is unavailable",
                )
            })?
            .take()
        {
            hook();
        }
        // Reacquire only to close the observation window and publish its stable result.
        let _repository_lease = repository_lease(&repository, &root, operation)?;
        let after = observe_root(&repository, &root);
        set_refresh_operation(&self.registry_path, operation_id, "observed", None, &root)?;
        if let Err(error) =
            self.check_failure(FailurePoint::BeforeIndexTransactionCommit, operation, &root)
        {
            set_refresh_operation(&self.registry_path, operation_id, "failed", None, &root)?;
            return Err(error);
        }
        let mut changed = changed_contexts(&before, &after, &root);
        let active_ids = before
            .active_contexts
            .iter()
            .flat_map(|context| context.items.iter().map(|item| item.id.clone()))
            .collect::<BTreeSet<_>>();
        if !changed.contains(&root) {
            persist_context_refresh(
                &self.registry_path,
                repository_id,
                operation_id,
                &before,
                &before.context,
                &before.head,
                &before.items,
                &before.problems,
                &before.validation.problems,
                &active_ids,
                &root,
            )?;
        }
        for active in &before.active_contexts {
            if !changed.contains(&active.context.worktree) {
                persist_context_refresh(
                    &self.registry_path,
                    repository_id,
                    operation_id,
                    &before,
                    &active.context,
                    &active.head,
                    &active.items,
                    &active.problems,
                    &active.validation.problems,
                    &BTreeSet::new(),
                    &root,
                )?;
            }
        }
        if let Some(context) = changed.pop_first() {
            persist_retry_problem(
                &self.registry_path,
                repository_id,
                &context,
                operation_id,
                &root,
            )?;
            return Ok(RefreshOutcome::RetryRequired {
                root,
                context: Some(context),
            });
        }
        reconcile_disappeared_contexts(
            &self.registry_path,
            repository_id,
            &before,
            operation_id,
            &root,
        )?;
        let snapshot = self.repository_snapshot(&root)?;
        Ok(RefreshOutcome::Refreshed { snapshot })
    }

    pub fn rebuild_repository(
        &self,
        request: RebuildRepositoryRequest,
    ) -> Result<RepositorySnapshot, RepositoryError> {
        let root = &request.root;
        let operation = RepositoryOperation::RebuildRepository;
        let (repository, root) = canonical_repository_root(root, operation)?;
        if self.requires_corrupt_cache_replacement()? {
            self.run_corrupt_cache_hook(&self.corrupt_cache_decision_hook, operation, &root)?;
            self.reconcile_corrupt_cache_replacement(operation, &root)?;
        }
        let operation_id = {
            let _repository_lease = repository_lease(&repository, &root, operation)?;
            begin_operation(&self.registry_path, &root, operation, request.operation_id)?.id
        };
        let result = (|| {
            let observation = observe_root(&repository, &root);
            if let Some(hook) = self
                .observation_hook
                .lock()
                .map_err(|_| {
                    RepositoryError::new(
                        operation,
                        Some(root.clone()),
                        RepositoryErrorKind::InjectedFailure,
                        "the test observation hook is unavailable",
                    )
                })?
                .take()
            {
                hook();
            }
            let _repository_lease = repository_lease(&repository, &root, operation)?;
            let after = observe_root(&repository, &root);
            set_rebuild_operation(&self.registry_path, operation_id, "observed", &root)?;
            if !changed_contexts(&observation, &after, &root).is_empty() {
                persist_rebuild_retry(&self.registry_path, operation_id, &root)?;
                return read_repository_snapshot_from_registry(&self.registry_path, &root);
            }
            self.check_failure(FailurePoint::BeforeIndexTransactionCommit, operation, &root)?;
            persist_rebuild_observation(&self.registry_path, operation_id, &root, &observation)?;
            set_rebuild_operation(&self.registry_path, operation_id, "completed", &root)?;
            read_repository_snapshot_from_registry(&self.registry_path, &root)
        })();
        match result {
            Ok(snapshot) => Ok(snapshot),
            Err(error) => {
                self.run_corrupt_cache_hook(&self.rebuild_error_hook, operation, &root)?;
                let _ = set_rebuild_operation(&self.registry_path, operation_id, "error", &root);
                Err(error)
            }
        }
    }

    pub fn repository_snapshot(&self, root: &Path) -> Result<RepositorySnapshot, RepositoryError> {
        self.require_index_available(RepositoryOperation::RepositorySnapshot, Some(root))?;
        let root = match std::fs::canonicalize(root) {
            Ok(root) => root,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => root.to_owned(),
            Err(error) => {
                return Err(RepositoryError::io(
                    RepositoryOperation::RepositorySnapshot,
                    Some(root.to_owned()),
                    error,
                ));
            }
        };
        let _cache_guard = cache_read_guard(
            &self.registry_path,
            &root,
            RepositoryOperation::RepositorySnapshot,
        )?;
        let root_path = root.to_str().ok_or_else(|| {
            RepositoryError::new(
                RepositoryOperation::RepositorySnapshot,
                Some(root.clone()),
                RepositoryErrorKind::RepositoryNotRegistered,
                "the repository is not registered",
            )
        })?;
        let mut connection = open_registry_read_only(&self.registry_path)
            .map_err(|error| snapshot_index_error_source(&root, error))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|error| snapshot_index_error_source(&root, error))?;
        let snapshot = read_repository_snapshot(&transaction, &root, root_path);
        transaction
            .rollback()
            .map_err(|error| snapshot_index_error_source(&root, error))?;
        snapshot
    }

    pub fn recovery_inspection(
        &self,
        root: &Path,
    ) -> Result<Vec<RecoveryInspection>, RepositoryError> {
        let operation = RepositoryOperation::Inspect;
        let root = std::fs::canonicalize(root)
            .map_err(|error| RepositoryError::io(operation, Some(root.to_owned()), error))?;
        let _cache_guard = cache_read_guard(&self.registry_path, &root, operation)?;
        let connection = open_registry_read_only(&self.registry_path)
            .map_err(|error| error.for_operation(operation, &root))?;
        pending_for_root(&connection, &root).map_err(|error| error.for_operation(operation, &root))
    }

    pub fn prepare_context(
        &self,
        target: AuthoringTarget,
    ) -> Result<ContextProvisionOutcome, RepositoryError> {
        let operation = RepositoryOperation::PrepareContext;
        self.require_index_available(operation, Some(&target.root))?;
        let (_, root) = canonical_repository_root(&target.root, operation)?;
        let operation_lock = process_repository_operation_lock(&self.registry_path, &root);
        let _operation_lock = operation_lock.lock().map_err(|_| {
            RepositoryError::new(
                operation,
                Some(root),
                RepositoryErrorKind::InjectedFailure,
                "the repository operation synchronization state is unavailable",
            )
        })?;
        self.prepare_context_unlocked(target)
    }

    fn prepare_context_unlocked(
        &self,
        target: AuthoringTarget,
    ) -> Result<ContextProvisionOutcome, RepositoryError> {
        let operation = RepositoryOperation::PrepareContext;
        self.require_index_available(operation, Some(&target.root))?;
        let (repository, root) = canonical_repository_root(&target.root, operation)?;
        let configuration = match read_configuration_for(&root, operation)? {
            ConfigurationInspection::Valid(configuration) => configuration,
            ConfigurationInspection::Missing => {
                return Err(RepositoryError::new(
                    operation,
                    Some(root),
                    RepositoryErrorKind::RepositoryNotEnabled,
                    "a Manyhands configuration is required before authoring",
                ));
            }
            ConfigurationInspection::Invalid(problem) => {
                return Err(RepositoryError::new(
                    operation,
                    Some(root),
                    RepositoryErrorKind::InvalidConfiguration,
                    problem.message,
                ));
            }
        };
        if checked_out_branch(&repository, &root, operation)? != configuration.primary_branch {
            return Err(RepositoryError::new(
                operation,
                Some(root),
                RepositoryErrorKind::WrongCheckedOutBranch,
                "the configured primary branch must be checked out at the repository root",
            ));
        }
        ensure_configuration_path_clean(&repository, &root, operation)?;

        let item_id = target.item_id.to_string();
        let branch = format!(
            "manyhands/{}/{item_id}",
            authoring_kind_segment(&target.kind)
        );
        if configuration.primary_branch == branch {
            return Err(RepositoryError::new(
                operation,
                Some(root),
                RepositoryErrorKind::MismatchedAuthoringContext,
                "the configured primary branch cannot be an authoring branch",
            ));
        }
        let worktree = root.join(".manyhands/worktrees").join(&item_id);
        let context = ItemContext {
            root: root.clone(),
            kind: target.kind,
            item_id: target.item_id,
            branch: branch.clone(),
            worktree: worktree.clone(),
        };
        let primary_context = canonical_context_at(&root, operation)?;
        let primary_has_target = primary_context
            .items
            .iter()
            .any(|item| authoring_item_matches(item, &context));

        if matches!(target.intent, ContextIntent::Create)
            && canonical_context_has_item_id(&primary_context, &context.item_id)
        {
            return Err(RepositoryError::new(
                operation,
                Some(root),
                RepositoryErrorKind::OccupiedItemPath,
                "the item ID already exists in the primary repository",
            ));
        }

        validate_deterministic_worktree_path_chain(&worktree, &root, operation)?;

        if branch_checked_out_in_another_worktree(
            &repository,
            &worktree,
            &branch,
            operation,
            &root,
        )? {
            return Err(RepositoryError::new(
                operation,
                Some(root),
                RepositoryErrorKind::MismatchedAuthoringContext,
                "the deterministic authoring branch is checked out in another worktree",
            ));
        }
        if worktree.exists() {
            validate_context_worktree(
                &repository,
                &context,
                matches!(target.intent, ContextIntent::Edit),
                operation,
            )?;
            return Ok(ContextProvisionOutcome::Reused(context));
        }
        if matches!(target.intent, ContextIntent::Edit) && !primary_has_target {
            return Err(RepositoryError::new(
                operation,
                Some(root),
                RepositoryErrorKind::MissingAuthoringTarget,
                "the requested item is not a valid canonical item in the primary repository",
            ));
        }
        ensure_authoring_worktree_base(&root, operation)?;

        let reference = match repository.find_branch(&branch, BranchType::Local) {
            Ok(branch) => {
                if matches!(target.intent, ContextIntent::Edit) {
                    let primary_head = repository
                        .head()
                        .and_then(|head| head.peel_to_commit())
                        .map_err(|error| {
                            RepositoryError::git(operation, Some(root.clone()), error)
                        })?;
                    if !primary_has_target || branch.get().target() != Some(primary_head.id()) {
                        return Err(RepositoryError::new(
                            operation,
                            Some(root),
                            RepositoryErrorKind::MismatchedAuthoringContext,
                            "the deterministic authoring branch exists without its worktree",
                        ));
                    }
                }
                branch.into_reference()
            }
            Err(error) if error.code() == git2::ErrorCode::NotFound => {
                let head = repository
                    .head()
                    .and_then(|head| head.peel_to_commit())
                    .map_err(|error| RepositoryError::git(operation, Some(root.clone()), error))?;
                self.check_failure(FailurePoint::BeforeContextBranchCreation, operation, &root)?;
                repository
                    .branch(&branch, &head, false)
                    .map_err(|error| RepositoryError::git(operation, Some(root.clone()), error))?
                    .into_reference()
            }
            Err(error) => return Err(RepositoryError::git(operation, Some(root), error)),
        };
        let mut options = WorktreeAddOptions::new();
        options.reference(Some(&reference));
        self.check_failure(FailurePoint::BeforeWorktreeCreation, operation, &root)?;
        repository
            .worktree(&item_id, &worktree, Some(&options))
            .map_err(|error| RepositoryError::git(operation, Some(root), error))?;
        Ok(ContextProvisionOutcome::Created(context))
    }

    pub fn save_document(
        &self,
        request: SaveDocumentRequest,
    ) -> Result<SaveOutcome, RepositoryError> {
        self.save_document_with_effective_config(request, None)
    }

    #[doc(hidden)]
    pub fn save_document_with_identity_config_for_testing(
        &self,
        request: SaveDocumentRequest,
        effective_config: &Config,
    ) -> Result<SaveOutcome, RepositoryError> {
        self.save_document_with_effective_config(request, Some(effective_config))
    }

    fn save_document_with_effective_config(
        &self,
        request: SaveDocumentRequest,
        effective_config: Option<&Config>,
    ) -> Result<SaveOutcome, RepositoryError> {
        let operation = RepositoryOperation::SaveDocument;
        self.require_index_available(operation, Some(&request.target.root))?;
        let (_, root) = canonical_repository_root(&request.target.root, operation)?;
        let operation_lock = process_repository_operation_lock(&self.registry_path, &root);
        let _operation_lock = operation_lock.lock().map_err(|_| {
            RepositoryError::new(
                operation,
                Some(root),
                RepositoryErrorKind::InjectedFailure,
                "the repository operation synchronization state is unavailable",
            )
        })?;
        let intent = request.target.intent;
        let context = match self.prepare_context_unlocked(AuthoringTarget {
            root: request.target.root,
            kind: request.target.kind,
            item_id: request.target.item_id,
            intent: request.target.intent,
            operation_id: request.target.operation_id,
        })? {
            ContextProvisionOutcome::Created(context)
            | ContextProvisionOutcome::Reused(context) => context,
        };
        if !matches!(context.kind, AuthoringKind::Document) {
            return Err(authoring_error(
                operation,
                &context.root,
                RepositoryErrorKind::MissingAuthoringTarget,
                "document saves require a document target",
            ));
        }
        let repository = Repository::open(&context.worktree)
            .map_err(|error| RepositoryError::git(operation, Some(context.root.clone()), error))?;
        let config = repository
            .config()
            .map_err(|error| RepositoryError::git(operation, Some(context.root.clone()), error))?;
        let effective_config = effective_config.unwrap_or(&config);
        let Some(identity) =
            resolve_identity(&config, effective_config, &context.worktree, operation)?
        else {
            return Ok(SaveOutcome::IdentityRequired { context });
        };

        let destination = owned_document_path(&request.destination_path, operation, &context)?;
        let source = request
            .source_path
            .as_deref()
            .map(|path| owned_document_path(path, operation, &context))
            .transpose()?;
        if matches!(intent, ContextIntent::Create)
            && let Some(existing) =
                document_path_for_id(&context.worktree, &context.item_id, operation)?
            && existing != destination
        {
            return Err(authoring_error(
                operation,
                &context.root,
                RepositoryErrorKind::OccupiedItemPath,
                "the requested document ID already exists at a different path",
            ));
        }
        validate_safe_owned_parent(&context.worktree, &destination, operation, &context.root)?;
        if let Some(source) = &source {
            validate_safe_owned_parent(&context.worktree, source, operation, &context.root)?;
        }
        let document = match (intent, source.as_deref()) {
            (ContextIntent::Create, Some(_)) => {
                return Err(authoring_error(
                    operation,
                    &context.root,
                    RepositoryErrorKind::InvalidPath,
                    "new documents cannot have a source path",
                ));
            }
            (ContextIntent::Create, None) => canonical::Document {
                id: context.item_id.clone(),
                title: request.draft.title,
                body: request.draft.body,
                unknown: serde_yaml::Mapping::new(),
            },
            (ContextIntent::Edit, None) => {
                return Err(authoring_error(
                    operation,
                    &context.root,
                    RepositoryErrorKind::MissingAuthoringTarget,
                    "editing a document requires its source path",
                ));
            }
            (ContextIntent::Edit, Some(source)) => {
                let existing_path =
                    if source_exists(&context.worktree, source, operation, &context.root)? {
                        source
                    } else {
                        &destination
                    };
                let canonical::CanonicalItem::Document(mut document) =
                    read_owned_item(&context.worktree, existing_path, operation, &context.root)?
                else {
                    return Err(authoring_error(
                        operation,
                        &context.root,
                        RepositoryErrorKind::MissingAuthoringTarget,
                        "the source path is not a canonical document",
                    ));
                };
                if document.id != context.item_id {
                    return Err(authoring_error(
                        operation,
                        &context.root,
                        RepositoryErrorKind::MissingAuthoringTarget,
                        "the source document ID does not match the selected target",
                    ));
                }
                document.title = request.draft.title;
                document.body = request.draft.body;
                document
            }
        };
        let serialized = canonical::serialize_item(&canonical::CanonicalItem::Document(document))
            .map_err(|problem| {
            authoring_error(
                operation,
                &context.root,
                RepositoryErrorKind::InvalidPath,
                problem.message,
            )
        })?;
        let canonical::CanonicalItem::Document(destination_document) =
            canonical::parse_item(&destination, &serialized).map_err(|problem| {
                authoring_error(
                    operation,
                    &context.root,
                    RepositoryErrorKind::InvalidPath,
                    problem.message,
                )
            })?
        else {
            unreachable!("serialized document parses as a document")
        };
        if destination_document.id != context.item_id {
            return Err(authoring_error(
                operation,
                &context.root,
                RepositoryErrorKind::InvalidPath,
                "destination document ID does not match the selected target",
            ));
        }

        let moving = source.as_ref().is_some_and(|source| source != &destination);
        let destination_exists =
            owned_file_exists(&context.worktree, &destination, operation, &context.root)?;
        if destination_exists && source.as_deref() != Some(destination.as_path()) {
            let existing =
                read_owned_item(&context.worktree, &destination, operation, &context.root)?;
            if existing != canonical::CanonicalItem::Document(destination_document.clone()) {
                return Err(authoring_error(
                    operation,
                    &context.root,
                    RepositoryErrorKind::OccupiedItemPath,
                    "the destination path is occupied by different content",
                ));
            }
        }
        let source_present = source
            .as_ref()
            .map(|source| source_exists(&context.worktree, source, operation, &context.root))
            .transpose()?
            .unwrap_or(false);
        if moving && !source_present && !destination_exists {
            return Err(authoring_error(
                operation,
                &context.root,
                RepositoryErrorKind::MissingAuthoringTarget,
                "neither move path contains the selected document",
            ));
        }
        let source_in_head = if moving && !source_present {
            validate_head_document_source(
                &repository,
                source.as_ref().expect("move has a source"),
                &context,
                operation,
            )?
        } else {
            false
        };
        if !destination_exists || source.as_deref() == Some(destination.as_path()) {
            ensure_safe_owned_parent(&context.worktree, &destination, operation, &context.root)?;
            self.check_failure(FailurePoint::BeforeItemWrite, operation, &context.root)?;
            write_owned_document(
                &context.worktree,
                &destination,
                serialized.as_bytes(),
                operation,
                &context.root,
            )?;
        }
        if moving {
            let source = source.as_ref().expect("move has a source");
            if source_present {
                self.check_failure(FailurePoint::BeforeItemWrite, operation, &context.root)?;
                remove_owned_file(&context.worktree, source, operation, &context.root)?;
            }
        }
        let checkpoint = self.checkpoint_owned_paths(
            &repository,
            &context,
            &identity,
            CheckpointSpec {
                operation: RepositoryOperation::SaveDocument,
                subject: &format!("Checkpoint document {}", context.item_id),
                added_paths: &[destination.as_path()],
                removed_source: (source_present || source_in_head)
                    .then(|| source.as_deref().expect("move has a source")),
            },
        )?;
        Ok(SaveOutcome::Saved {
            context,
            checkpoint,
        })
    }

    pub fn save_ticket(&self, request: SaveTicketRequest) -> Result<SaveOutcome, RepositoryError> {
        self.save_ticket_with_effective_config(request, None)
    }

    #[doc(hidden)]
    pub fn save_ticket_with_identity_config_for_testing(
        &self,
        request: SaveTicketRequest,
        effective_config: &Config,
    ) -> Result<SaveOutcome, RepositoryError> {
        self.save_ticket_with_effective_config(request, Some(effective_config))
    }

    fn save_ticket_with_effective_config(
        &self,
        request: SaveTicketRequest,
        effective_config: Option<&Config>,
    ) -> Result<SaveOutcome, RepositoryError> {
        let operation = RepositoryOperation::SaveTicket;
        self.require_index_available(operation, Some(&request.target.root))?;
        let (_, root) = canonical_repository_root(&request.target.root, operation)?;
        let operation_lock = process_repository_operation_lock(&self.registry_path, &root);
        let _operation_lock = operation_lock.lock().map_err(|_| {
            RepositoryError::new(
                operation,
                Some(root),
                RepositoryErrorKind::InjectedFailure,
                "the repository operation synchronization state is unavailable",
            )
        })?;
        let intent = request.target.intent;
        let context = match self.prepare_context_unlocked(AuthoringTarget {
            root: request.target.root,
            kind: request.target.kind,
            item_id: request.target.item_id,
            intent,
            operation_id: request.target.operation_id,
        })? {
            ContextProvisionOutcome::Created(context)
            | ContextProvisionOutcome::Reused(context) => context,
        };
        if !matches!(context.kind, AuthoringKind::Ticket) {
            return Err(authoring_error(
                operation,
                &context.root,
                RepositoryErrorKind::MissingAuthoringTarget,
                "ticket saves require a ticket target",
            ));
        }
        let repository = Repository::open(&context.worktree)
            .map_err(|error| RepositoryError::git(operation, Some(context.root.clone()), error))?;
        let config = repository
            .config()
            .map_err(|error| RepositoryError::git(operation, Some(context.root.clone()), error))?;
        let effective_config = effective_config.unwrap_or(&config);
        let Some(identity) =
            resolve_identity(&config, effective_config, &context.worktree, operation)?
        else {
            return Ok(SaveOutcome::IdentityRequired { context });
        };
        let path = PathBuf::from(format!(".manyhands/tickets/{}/ticket.md", context.item_id));
        validate_safe_owned_parent(&context.worktree, &path, operation, &context.root)?;
        if ticket_id_exists_at_a_different_path(
            &context.worktree,
            &context.item_id,
            &path,
            operation,
            &context.root,
        )? {
            return Err(authoring_error(
                operation,
                &context.root,
                RepositoryErrorKind::OccupiedItemPath,
                "the ticket ID already exists at a different ticket path",
            ));
        }
        let exists = owned_file_exists(&context.worktree, &path, operation, &context.root)?;
        let ticket = match intent {
            ContextIntent::Create => canonical::Ticket {
                id: context.item_id.clone(),
                title: request.draft.title,
                ticket_type: request.draft.ticket_type,
                status: request.draft.status,
                project: request.draft.project,
                team: request.draft.team,
                closed_at: None,
                closed_by: None,
                body: request.draft.body,
                unknown: serde_yaml::Mapping::new(),
            },
            ContextIntent::Edit => {
                let canonical::CanonicalItem::Ticket(mut ticket) =
                    read_owned_item(&context.worktree, &path, operation, &context.root)?
                else {
                    return Err(authoring_error(
                        operation,
                        &context.root,
                        RepositoryErrorKind::MissingAuthoringTarget,
                        "the ticket path does not contain a canonical ticket",
                    ));
                };
                if ticket.id != context.item_id {
                    return Err(authoring_error(
                        operation,
                        &context.root,
                        RepositoryErrorKind::MissingAuthoringTarget,
                        "the ticket ID does not match the selected target",
                    ));
                }
                ticket.title = request.draft.title;
                ticket.ticket_type = request.draft.ticket_type;
                ticket.status = request.draft.status;
                ticket.project = request.draft.project;
                ticket.team = request.draft.team;
                ticket.body = request.draft.body;
                ticket
            }
        };
        let serialized = canonical::serialize_item(&canonical::CanonicalItem::Ticket(
            ticket.clone(),
        ))
        .map_err(|problem| {
            authoring_error(
                operation,
                &context.root,
                RepositoryErrorKind::InvalidPath,
                problem.message,
            )
        })?;
        let canonical::CanonicalItem::Ticket(parsed) = canonical::parse_item(&path, &serialized)
            .map_err(|problem| {
                authoring_error(
                    operation,
                    &context.root,
                    RepositoryErrorKind::InvalidPath,
                    problem.message,
                )
            })?
        else {
            unreachable!("serialized ticket parses as a ticket")
        };
        if parsed.id != context.item_id {
            return Err(authoring_error(
                operation,
                &context.root,
                RepositoryErrorKind::InvalidPath,
                "ticket ID does not match the selected target",
            ));
        }
        let write = if exists {
            let existing = read_owned_item(&context.worktree, &path, operation, &context.root)?;
            if matches!(intent, ContextIntent::Create)
                && existing != canonical::CanonicalItem::Ticket(ticket.clone())
            {
                return Err(authoring_error(
                    operation,
                    &context.root,
                    RepositoryErrorKind::OccupiedItemPath,
                    "the ticket path is occupied by different content",
                ));
            }
            existing != canonical::CanonicalItem::Ticket(ticket)
        } else {
            true
        };
        if write {
            ensure_safe_owned_parent(&context.worktree, &path, operation, &context.root)?;
            self.check_failure(FailurePoint::BeforeItemWrite, operation, &context.root)?;
            write_owned_document(
                &context.worktree,
                &path,
                serialized.as_bytes(),
                operation,
                &context.root,
            )?;
        }
        let checkpoint = self.checkpoint_owned_paths(
            &repository,
            &context,
            &identity,
            CheckpointSpec {
                operation: RepositoryOperation::SaveTicket,
                subject: &format!("Checkpoint ticket {}", context.item_id),
                added_paths: &[path.as_path()],
                removed_source: None,
            },
        )?;
        Ok(SaveOutcome::Saved {
            context,
            checkpoint,
        })
    }

    pub fn submit_comment(
        &self,
        request: SubmitCommentRequest,
    ) -> Result<CommentSubmissionOutcome, RepositoryError> {
        self.submit_comment_with_effective_config(request, None)
    }

    #[doc(hidden)]
    pub fn submit_comment_with_identity_config_for_testing(
        &self,
        request: SubmitCommentRequest,
        effective_config: &Config,
    ) -> Result<CommentSubmissionOutcome, RepositoryError> {
        self.submit_comment_with_effective_config(request, Some(effective_config))
    }

    fn submit_comment_with_effective_config(
        &self,
        request: SubmitCommentRequest,
        effective_config: Option<&Config>,
    ) -> Result<CommentSubmissionOutcome, RepositoryError> {
        let operation = RepositoryOperation::SubmitComment;
        self.require_index_available(operation, Some(&request.target.root))?;
        let (_, root) = canonical_repository_root(&request.target.root, operation)?;
        let operation_lock = process_repository_operation_lock(&self.registry_path, &root);
        let _operation_lock = operation_lock.lock().map_err(|_| {
            RepositoryError::new(
                operation,
                Some(root),
                RepositoryErrorKind::InjectedFailure,
                "the repository operation synchronization state is unavailable",
            )
        })?;
        if !matches!(request.target.intent, ContextIntent::Edit) {
            return Err(authoring_error(
                operation,
                &request.target.root,
                RepositoryErrorKind::MissingAuthoringTarget,
                "comment submission requires an edit target",
            ));
        }
        let context = match self.prepare_context_unlocked(AuthoringTarget {
            root: request.target.root,
            kind: request.target.kind,
            item_id: request.target.item_id,
            intent: ContextIntent::Edit,
            operation_id: request.target.operation_id,
        })? {
            ContextProvisionOutcome::Created(context)
            | ContextProvisionOutcome::Reused(context) => context,
        };
        let publication = comment_publication_state(
            read_configuration_for(&context.root, operation)?,
            operation,
            &context.root,
        )?;
        let context_content = canonical_context_at(&context.worktree, operation)?;
        if !context_content
            .items
            .iter()
            .any(|item| authoring_item_matches(item, &context))
        {
            return Err(authoring_error(
                operation,
                &context.root,
                RepositoryErrorKind::MissingAuthoringTarget,
                "the selected context does not contain the requested canonical item",
            ));
        }
        if let Some(parent_id) = &request.parent_id
            && !context_content.items.iter().any(|item| {
                matches!(item, canonical::CanonicalItem::Comment(comment)
                    if &comment.id == parent_id && comment.item_id == context.item_id)
            })
        {
            return Err(authoring_error(
                operation,
                &context.root,
                RepositoryErrorKind::MissingAuthoringTarget,
                "the requested parent is not a conforming comment for the selected item",
            ));
        }
        let path = PathBuf::from(format!(
            ".manyhands/comments/{}/{}.md",
            context.item_id, request.comment_id
        ));
        let existing_comment_paths =
            canonical_comment_paths_for_id(&context.worktree, &request.comment_id, operation)?;
        if canonical_context_has_item_id(&context_content, &request.comment_id)
            && (existing_comment_paths.len() != 1 || existing_comment_paths[0] != path)
        {
            return Err(authoring_error(
                operation,
                &context.root,
                RepositoryErrorKind::OccupiedItemPath,
                "the comment ID already exists at a different path",
            ));
        }

        validate_safe_owned_parent(&context.worktree, &path, operation, &context.root)?;
        let exists = owned_file_exists(&context.worktree, &path, operation, &context.root)?;
        let comment = if exists {
            let canonical::CanonicalItem::Comment(comment) =
                read_owned_item(&context.worktree, &path, operation, &context.root)?
            else {
                return Err(authoring_error(
                    operation,
                    &context.root,
                    RepositoryErrorKind::OccupiedItemPath,
                    "the comment path is occupied by a different canonical item",
                ));
            };
            if comment.id != request.comment_id
                || comment.item_id != context.item_id
                || comment.parent_id != request.parent_id
                || comment.body != request.body
                || !comment.unknown.is_empty()
            {
                return Err(authoring_error(
                    operation,
                    &context.root,
                    RepositoryErrorKind::OccupiedItemPath,
                    "the comment path is occupied by different content",
                ));
            }
            comment
        } else {
            canonical::Comment {
                id: request.comment_id.clone(),
                item_id: context.item_id.clone(),
                parent_id: request.parent_id.clone(),
                created_at: OffsetDateTime::now_utc(),
                body: request.body.clone(),
                unknown: serde_yaml::Mapping::new(),
            }
        };
        let serialized = canonical::serialize_item(&canonical::CanonicalItem::Comment(comment))
            .map_err(|problem| {
                authoring_error(
                    operation,
                    &context.root,
                    RepositoryErrorKind::InvalidPath,
                    problem.message,
                )
            })?;
        let mut context_sources = Vec::new();
        collect_canonical_sources(&context.worktree, &mut context_sources, operation)?;
        if !exists {
            context_sources.push((path.clone(), serialized.clone()));
        }
        let validated = canonical::validate_context(context_sources);
        if !validated.problems.is_empty() {
            return Err(authoring_error(
                operation,
                &context.root,
                RepositoryErrorKind::MissingAuthoringTarget,
                "the selected context contains nonconforming canonical content",
            ));
        }
        if !validated.items.iter().any(|item| {
            matches!(item, canonical::CanonicalItem::Comment(comment)
                if comment.id == request.comment_id
                    && comment.item_id == context.item_id
                    && comment.parent_id == request.parent_id
                    && comment.body == request.body)
        }) {
            return Err(authoring_error(
                operation,
                &context.root,
                RepositoryErrorKind::MissingAuthoringTarget,
                "the submitted comment is not conforming in the selected context",
            ));
        }

        let repository = Repository::open(&context.worktree)
            .map_err(|error| RepositoryError::git(operation, Some(context.root.clone()), error))?;
        let config = repository
            .config()
            .map_err(|error| RepositoryError::git(operation, Some(context.root.clone()), error))?;
        let effective_config = effective_config.unwrap_or(&config);
        let Some(identity) =
            resolve_identity(&config, effective_config, &context.worktree, operation)?
        else {
            return Ok(CommentSubmissionOutcome::IdentityRequired { context });
        };
        if !exists {
            self.check_failure(FailurePoint::BeforeItemWrite, operation, &context.root)?;
            ensure_safe_owned_parent(&context.worktree, &path, operation, &context.root)?;
            write_owned_document(
                &context.worktree,
                &path,
                serialized.as_bytes(),
                operation,
                &context.root,
            )?;
        }
        let checkpoint = self.checkpoint_owned_paths(
            &repository,
            &context,
            &identity,
            CheckpointSpec {
                operation,
                subject: &format!("Checkpoint comment {}", request.comment_id),
                added_paths: &[path.as_path()],
                removed_source: None,
            },
        )?;
        Ok(CommentSubmissionOutcome::Saved {
            context,
            checkpoint,
            publication,
        })
    }

    fn check_failure(
        &self,
        point: FailurePoint,
        operation: RepositoryOperation,
        root: &Path,
    ) -> Result<(), RepositoryError> {
        if self.should_inject(point, operation, root)? {
            return Err(RepositoryError::new(
                operation,
                Some(root.to_owned()),
                RepositoryErrorKind::InjectedFailure,
                format!("{point:?} was interrupted by a test failure hook"),
            ));
        }
        Ok(())
    }

    fn should_inject(
        &self,
        point: FailurePoint,
        operation: RepositoryOperation,
        root: &Path,
    ) -> Result<bool, RepositoryError> {
        let mut failure_point = self.failure_point.lock().map_err(|_| {
            RepositoryError::new(
                operation,
                Some(root.to_owned()),
                RepositoryErrorKind::InjectedFailure,
                "the fixed test failure state is unavailable",
            )
        })?;
        if *failure_point == Some(point) {
            *failure_point = None;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn reconcile_registration(
        &self,
        repository: &Repository,
        root: &Path,
        operation: RepositoryOperation,
    ) -> Result<(), RepositoryError> {
        self.check_failure(FailurePoint::BeforeRegistryWrite, operation, root)?;
        if let Some(hook) = self
            .registration_git_hook
            .lock()
            .map_err(|_| {
                RepositoryError::new(
                    operation,
                    Some(root.to_owned()),
                    RepositoryErrorKind::InjectedFailure,
                    "the test registration Git hook is unavailable",
                )
            })?
            .take()
        {
            hook();
        }
        reconcile_registration(&self.registry_path, repository, root)
    }

    fn begin_lifecycle(
        &self,
        repository: &Repository,
        root: &Path,
        operation: RepositoryOperation,
        operation_id: OperationId,
        target: &str,
    ) -> Result<(RepositoryLease, RecoveryRecord), RepositoryError> {
        let lease = repository_lease(repository, root, operation)?;
        let record = self.begin_lifecycle_record(root, operation, operation_id, target)?;
        Ok((lease, record))
    }

    fn begin_lifecycle_record(
        &self,
        root: &Path,
        operation: RepositoryOperation,
        operation_id: OperationId,
        target: &str,
    ) -> Result<RecoveryRecord, RepositoryError> {
        // Preserve authoritative local-Git behavior when the optional cache is unavailable.
        if self.registry_path.is_dir() {
            return Ok(RecoveryRecord { id: 0 });
        }
        let _cache_guard = cache_write_guard(&self.registry_path, root, operation)?;
        let mut connection = open_registry(&self.registry_path, &mut |_| {})
            .map_err(|error| error.for_operation(operation, root))?;
        migrate_registry(&mut connection).map_err(|error| error.for_operation(operation, root))?;
        let record =
            begin_or_reconcile_operation(&mut connection, root, operation, operation_id, target)
                .map_err(|error| error.for_operation(operation, root))?;
        Ok(record)
    }

    fn complete_lifecycle(
        &self,
        root: &Path,
        operation: RepositoryOperation,
        record: RecoveryRecord,
    ) -> Result<(), RepositoryError> {
        if record.id == 0 {
            return Ok(());
        }
        let _cache_guard = cache_write_guard(&self.registry_path, root, operation)?;
        let mut connection = open_registry(&self.registry_path, &mut |_| {})
            .map_err(|error| error.for_operation(operation, root))?;
        migrate_registry(&mut connection).map_err(|error| error.for_operation(operation, root))?;
        advance_after_observation(&connection, record.id, "completed", None, None)
            .map_err(|error| error.for_operation(operation, root))
    }

    fn advance_lifecycle(
        &self,
        root: &Path,
        operation: RepositoryOperation,
        record: RecoveryRecord,
        step: &str,
    ) -> Result<(), RepositoryError> {
        if record.id == 0 {
            return Ok(());
        }
        let _cache_guard = cache_write_guard(&self.registry_path, root, operation)?;
        let mut connection = open_registry(&self.registry_path, &mut |_| {})
            .map_err(|error| error.for_operation(operation, root))?;
        migrate_registry(&mut connection).map_err(|error| error.for_operation(operation, root))?;
        advance_after_observation(&connection, record.id, step, None, None)
            .map_err(|error| error.for_operation(operation, root))
    }

    fn checkpoint_owned_paths(
        &self,
        repository: &Repository,
        context: &ItemContext,
        identity: &CommitIdentity,
        spec: CheckpointSpec<'_>,
    ) -> Result<LocalCheckpoint, RepositoryError> {
        let CheckpointSpec {
            operation,
            subject,
            added_paths,
            removed_source,
        } = spec;
        let head = repository
            .head()
            .and_then(|head| head.peel_to_commit())
            .map_err(|error| RepositoryError::git(operation, Some(context.root.clone()), error))?;
        let head_tree = head
            .tree()
            .map_err(|error| RepositoryError::git(operation, Some(context.root.clone()), error))?;
        let mut index = Index::new()
            .map_err(|error| RepositoryError::git(operation, Some(context.root.clone()), error))?;
        index
            .read_tree(&head_tree)
            .map_err(|error| RepositoryError::git(operation, Some(context.root.clone()), error))?;
        if let Some(source) = removed_source {
            index.remove_path(source).map_err(|error| {
                RepositoryError::git(operation, Some(context.root.clone()), error)
            })?;
        }
        for path in added_paths {
            add_owned_blob(
                &mut index,
                repository,
                &context.worktree,
                path,
                operation,
                &context.root,
            )?;
        }
        let tree_oid = index
            .write_tree_to(repository)
            .map_err(|error| RepositoryError::git(operation, Some(context.root.clone()), error))?;
        if tree_oid == head_tree.id() {
            return match self.mark_checkpoint_refresh(&context.root, operation) {
                Ok(()) => Ok(LocalCheckpoint::NoChange),
                Err(()) => Ok(LocalCheckpoint::RefreshPending {
                    commit_oid: head.id(),
                }),
            };
        }
        let tree = repository
            .find_tree(tree_oid)
            .map_err(|error| RepositoryError::git(operation, Some(context.root.clone()), error))?;
        let signature = Signature::now(&identity.name, &identity.email)
            .map_err(|error| RepositoryError::git(operation, Some(context.root.clone()), error))?;
        self.check_failure(
            FailurePoint::BeforeCheckpointCommit,
            operation,
            &context.root,
        )?;
        let commit_oid = repository
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                subject,
                &tree,
                &[&head],
            )
            .map_err(|error| RepositoryError::git(operation, Some(context.root.clone()), error))?;
        Ok(
            match self.mark_checkpoint_refresh(&context.root, operation) {
                Ok(()) => LocalCheckpoint::Checkpointed { commit_oid },
                Err(()) => LocalCheckpoint::RefreshPending { commit_oid },
            },
        )
    }

    fn mark_checkpoint_refresh(
        &self,
        root: &Path,
        operation: RepositoryOperation,
    ) -> Result<(), ()> {
        self.check_failure(FailurePoint::BeforeRegistryWrite, operation, root)
            .and_then(|()| mark_document_refresh_required(&self.registry_path, root, operation))
            .map_err(|_| ())
    }

    fn require_index_available(
        &self,
        operation: RepositoryOperation,
        root: Option<&Path>,
    ) -> Result<(), RepositoryError> {
        if self.index_is_unavailable()? {
            return Err(RepositoryError::new(
                operation,
                root.map(Path::to_owned),
                RepositoryErrorKind::IndexUnavailable,
                "the local repository index is unavailable until an explicit rebuild succeeds",
            ));
        }
        Ok(())
    }

    fn index_is_unavailable(&self) -> Result<bool, RepositoryError> {
        self.availability
            .lock()
            .map(|availability| matches!(*availability, IndexAvailability::Degraded))
            .map_err(|_| {
                RepositoryError::new(
                    RepositoryOperation::OpenRegistry,
                    None,
                    RepositoryErrorKind::InjectedFailure,
                    "the index availability state is unavailable",
                )
            })
    }

    fn requires_corrupt_cache_replacement(&self) -> Result<bool, RepositoryError> {
        self.availability
            .lock()
            .map(|availability| matches!(*availability, IndexAvailability::Degraded))
            .map_err(|_| {
                RepositoryError::new(
                    RepositoryOperation::OpenRegistry,
                    None,
                    RepositoryErrorKind::InjectedFailure,
                    "the index availability state is unavailable",
                )
            })
    }

    fn set_index_availability(
        &self,
        availability: IndexAvailability,
        operation: RepositoryOperation,
        root: &Path,
    ) -> Result<(), RepositoryError> {
        *self.availability.lock().map_err(|_| {
            RepositoryError::new(
                operation,
                Some(root.to_owned()),
                RepositoryErrorKind::InjectedFailure,
                "the index availability state is unavailable",
            )
        })? = availability;
        Ok(())
    }

    fn reconcile_corrupt_cache_replacement(
        &self,
        operation: RepositoryOperation,
        root: &Path,
    ) -> Result<(), RepositoryError> {
        let _cache_guard = cache_write_guard(&self.registry_path, root, operation)?;
        let availability = match open_registry(&self.registry_path, &mut |_| {})
            .and_then(|mut connection| migrate_registry(&mut connection))
        {
            Ok(()) => IndexAvailability::Ready,
            Err(error) if is_structural_sqlite_corruption(&error) => {
                self.run_corrupt_cache_hook(&self.corrupt_cache_critical_hook, operation, root)?;
                self.check_failure(FailurePoint::BeforeCorruptCacheReplacement, operation, root)?;
                replace_corrupt_registry(&self.registry_path, root)?;
                let mut connection = open_registry(&self.registry_path, &mut |_| {})
                    .map_err(|error| error.for_operation(operation, root))?;
                migrate_registry(&mut connection)
                    .map_err(|error| error.for_operation(operation, root))?;
                IndexAvailability::Ready
            }
            Err(error) => return Err(error.for_operation(operation, root)),
        };
        self.set_index_availability(availability, operation, root)
    }

    fn run_corrupt_cache_hook(
        &self,
        hook: &Mutex<Option<Box<dyn FnOnce() + Send>>>,
        operation: RepositoryOperation,
        root: &Path,
    ) -> Result<(), RepositoryError> {
        if let Some(hook) = hook
            .lock()
            .map_err(|_| {
                RepositoryError::new(
                    operation,
                    Some(root.to_owned()),
                    RepositoryErrorKind::InjectedFailure,
                    "the test corrupt-cache hook is unavailable",
                )
            })?
            .take()
        {
            hook();
        }
        Ok(())
    }

    #[doc(hidden)]
    pub fn open_at_with_registry_phase_observer(
        data_directory: &Path,
        observer: impl FnMut(RegistryConnectionPhase),
    ) -> Result<Self, RepositoryError> {
        Self::open_at_with_registry_observer(data_directory, observer)
    }

    fn open_at_with_registry_observer(
        data_directory: &Path,
        mut observer: impl FnMut(RegistryConnectionPhase),
    ) -> Result<Self, RepositoryError> {
        std::fs::create_dir_all(data_directory)
            .map_err(|error| RepositoryError::io(RepositoryOperation::OpenRegistry, None, error))?;
        let registry_path = data_directory.join(REGISTRY_FILE);
        let _cache_guard = cache_read_guard(
            &registry_path,
            data_directory,
            RepositoryOperation::OpenRegistry,
        )?;
        let availability = match open_registry(&registry_path, &mut observer)
            .and_then(|mut connection| migrate_registry(&mut connection))
        {
            Ok(()) => IndexAvailability::Ready,
            Err(error) if is_structural_sqlite_corruption(&error) => IndexAvailability::Degraded,
            Err(error) => return Err(error),
        };

        Ok(Self {
            registry_path,
            availability: Mutex::new(availability),
            failure_point: Mutex::new(None),
            observation_hook: Mutex::new(None),
            registration_git_hook: Mutex::new(None),
            corrupt_cache_decision_hook: Mutex::new(None),
            corrupt_cache_critical_hook: Mutex::new(None),
            rebuild_error_hook: Mutex::new(None),
        })
    }

    pub fn inspect(&self, root: &Path) -> Result<RepositoryInspection, RepositoryError> {
        self.require_index_available(RepositoryOperation::Inspect, Some(root))?;
        self.inspect_with_identity_provider(root, &mut RepositoryIdentityConfig)
    }

    pub fn enable(
        &self,
        request: EnableRepositoryRequest,
    ) -> Result<EnableRepositoryOutcome, RepositoryError> {
        self.require_index_available(RepositoryOperation::Enable, Some(&request.root))?;
        self.enable_with_identity_provider(request, &mut RepositoryIdentityConfig)
    }

    pub fn list_remotes(&self, root: &Path) -> Result<Vec<RemoteInfo>, RepositoryError> {
        self.require_index_available(RepositoryOperation::ListRemotes, Some(root))?;
        let (repository, root) = canonical_repository_root(root, RepositoryOperation::ListRemotes)?;
        remote_info_for(&repository, &root, RepositoryOperation::ListRemotes)
    }

    pub fn add_remote(&self, request: AddRemoteRequest) -> Result<RemoteOutcome, RepositoryError> {
        self.require_index_available(RepositoryOperation::AddRemote, Some(&request.root))?;
        let (repository, root) =
            canonical_repository_root(&request.root, RepositoryOperation::AddRemote)?;
        registry_root_key(&root, RepositoryOperation::AddRemote)?;
        let (_lease, record) = self.begin_lifecycle(
            &repository,
            &root,
            RepositoryOperation::AddRemote,
            request.operation_id,
            &format!("{}\u{1f}{}", request.name, request.url),
        )?;
        match repository.find_remote(&request.name) {
            Ok(remote) if remote.url() == Some(request.url.as_str()) => {
                mark_registered_refresh_required(
                    &self.registry_path,
                    &root,
                    RepositoryOperation::AddRemote,
                )?;
                self.complete_lifecycle(&root, RepositoryOperation::AddRemote, record)?;
                return Ok(RemoteOutcome::NoChange);
            }
            Ok(_) => {
                self.complete_lifecycle(&root, RepositoryOperation::AddRemote, record)?;
                return Err(RepositoryError::new(
                    RepositoryOperation::AddRemote,
                    Some(root),
                    RepositoryErrorKind::RemoteNameConflict,
                    "a remote with this name already has a different fetch URL",
                ));
            }
            Err(error) if error.code() == git2::ErrorCode::NotFound => {}
            Err(error) => {
                return Err(RepositoryError::git(
                    RepositoryOperation::AddRemote,
                    Some(root),
                    error,
                ));
            }
        }
        repository
            .remote(&request.name, &request.url)
            .map_err(|error| {
                RepositoryError::git(RepositoryOperation::AddRemote, Some(root.clone()), error)
            })?;
        self.advance_lifecycle(
            &root,
            RepositoryOperation::AddRemote,
            record,
            "remote_changed",
        )?;
        self.check_failure(
            FailurePoint::AfterRemoteMutation,
            RepositoryOperation::AddRemote,
            &root,
        )?;
        mark_registered_refresh_required(
            &self.registry_path,
            &root,
            RepositoryOperation::AddRemote,
        )?;
        self.complete_lifecycle(&root, RepositoryOperation::AddRemote, record)?;
        Ok(RemoteOutcome::Changed)
    }

    pub fn remove_remote(
        &self,
        request: RemoveRemoteRequest,
    ) -> Result<RemoteOutcome, RepositoryError> {
        let selected = &request.root;
        let name = &request.name;
        self.require_index_available(RepositoryOperation::RemoveRemote, Some(selected))?;
        let (repository, root) =
            canonical_repository_root(selected, RepositoryOperation::RemoveRemote)?;
        registry_root_key(&root, RepositoryOperation::RemoveRemote)?;
        let configuration = read_configuration_for(&root, RepositoryOperation::RemoveRemote)?;
        if matches!(configuration, ConfigurationInspection::Valid(ref config) if config.publication_remote.as_deref() == Some(name))
        {
            return Err(RepositoryError::new(
                RepositoryOperation::RemoveRemote,
                Some(root),
                RepositoryErrorKind::SelectedRemoteRemoval,
                "clear the publication remote before removing it",
            ));
        }
        let (_lease, record) = self.begin_lifecycle(
            &repository,
            &root,
            RepositoryOperation::RemoveRemote,
            request.operation_id,
            name,
        )?;
        match repository.find_remote(name) {
            Ok(_) => {}
            Err(error) if error.code() == git2::ErrorCode::NotFound => {
                mark_registered_refresh_required(
                    &self.registry_path,
                    &root,
                    RepositoryOperation::RemoveRemote,
                )?;
                self.complete_lifecycle(&root, RepositoryOperation::RemoveRemote, record)?;
                return Ok(RemoteOutcome::NoChange);
            }
            Err(error) => {
                return Err(RepositoryError::git(
                    RepositoryOperation::RemoveRemote,
                    Some(root),
                    error,
                ));
            }
        }
        repository.remote_delete(name).map_err(|error| {
            RepositoryError::git(RepositoryOperation::RemoveRemote, Some(root.clone()), error)
        })?;
        self.advance_lifecycle(
            &root,
            RepositoryOperation::RemoveRemote,
            record,
            "remote_changed",
        )?;
        self.check_failure(
            FailurePoint::AfterRemoteMutation,
            RepositoryOperation::RemoveRemote,
            &root,
        )?;
        mark_registered_refresh_required(
            &self.registry_path,
            &root,
            RepositoryOperation::RemoveRemote,
        )?;
        self.complete_lifecycle(&root, RepositoryOperation::RemoveRemote, record)?;
        Ok(RemoteOutcome::Changed)
    }

    pub fn set_publication_remote(
        &self,
        request: SetPublicationRemoteRequest,
    ) -> Result<PublicationRemoteOutcome, RepositoryError> {
        let operation = RepositoryOperation::SetPublicationRemote;
        self.require_index_available(operation, Some(&request.root))?;
        let (repository, root) = canonical_repository_root(&request.root, operation)?;
        let ConfigurationInspection::Valid(mut config) = read_configuration_for(&root, operation)?
        else {
            return Err(RepositoryError::new(
                operation,
                Some(root),
                RepositoryErrorKind::InvalidConfiguration,
                "a valid Manyhands configuration is required",
            ));
        };
        if checked_out_branch(&repository, &root, operation)? != config.primary_branch {
            return Err(RepositoryError::new(
                operation,
                Some(root),
                RepositoryErrorKind::WrongCheckedOutBranch,
                "the configured primary branch must be checked out",
            ));
        }
        ensure_configuration_path_clean(&repository, &root, operation)?;
        if let Some(name) = request.name.as_deref() {
            let remote = match repository.find_remote(name) {
                Ok(remote) => remote,
                Err(error) if error.code() == git2::ErrorCode::NotFound => {
                    return Err(RepositoryError::new(
                        operation,
                        Some(root),
                        RepositoryErrorKind::UnavailablePublicationRemote,
                        "the selected publication remote does not exist",
                    ));
                }
                Err(error) => {
                    return Err(RepositoryError::git(operation, Some(root), error));
                }
            };
            if !remote.url().is_some_and(|fetch| {
                ssh_compatible(fetch) && ssh_compatible(remote.pushurl().unwrap_or(fetch))
            }) {
                return Err(RepositoryError::new(
                    operation,
                    Some(root),
                    RepositoryErrorKind::InvalidPublicationRemote,
                    "the selected remote must have SSH-compatible fetch and push URLs",
                ));
            }
        }
        let (_lease, record) = self.begin_lifecycle(
            &repository,
            &root,
            operation,
            request.operation_id,
            request.name.as_deref().unwrap_or(""),
        )?;
        if config.publication_remote == request.name {
            self.reconcile_registration(&repository, &root, operation)
                .map_err(|error| registry_refresh_pending(operation, &root, error))?;
            self.complete_lifecycle(&root, operation, record)?;
            return Ok(PublicationRemoteOutcome::NoChange);
        }
        let local_config = repository
            .config()
            .map_err(|error| RepositoryError::git(operation, Some(root.clone()), error))?;
        let Some(identity) = resolve_identity(&local_config, &local_config, &root, operation)?
        else {
            return Err(RepositoryError::new(
                operation,
                Some(root),
                RepositoryErrorKind::InvalidIdentity,
                "configure user.name and user.email before changing the publication remote",
            ));
        };
        config.publication_remote = request.name;
        let source = canonical::serialize_repository_config(&config).map_err(|problem| {
            RepositoryError::new(
                operation,
                Some(root.clone()),
                RepositoryErrorKind::InvalidConfiguration,
                problem.message,
            )
        })?;
        let path = root.join(canonical::CONFIG_PATH);
        let before = std::fs::read(&path)
            .map_err(|error| RepositoryError::io(operation, Some(root.clone()), error))?;
        replace_bytes_atomically(&path, source.as_bytes(), &root)
            .map_err(|error| error.for_operation(operation, &root))?;
        self.advance_lifecycle(
            &root,
            operation,
            record,
            "publication_configuration_written",
        )?;
        let result = (|| {
            let tree_id = configuration_tree(&repository, source.as_bytes(), &root, operation)?;
            let tree = repository
                .find_tree(tree_id)
                .map_err(|error| RepositoryError::git(operation, Some(root.clone()), error))?;
            let parent = repository
                .head()
                .and_then(|head| head.peel_to_commit())
                .map_err(|error| RepositoryError::git(operation, Some(root.clone()), error))?;
            let signature = Signature::now(&identity.name, &identity.email)
                .map_err(|error| RepositoryError::git(operation, Some(root.clone()), error))?;
            self.check_failure(
                FailurePoint::BeforePublicationConfigurationCommit,
                operation,
                &root,
            )?;
            repository
                .commit(
                    Some("HEAD"),
                    &signature,
                    &signature,
                    "Configure Manyhands publication remote",
                    &tree,
                    &[&parent],
                )
                .map_err(|error| RepositoryError::git(operation, Some(root.clone()), error))
        })();
        let commit_oid = match result {
            Ok(oid) => oid,
            Err(error) => {
                if let Err(restore) = replace_bytes_atomically(&path, &before, &root) {
                    return Err(rollback_incomplete_for(
                        operation,
                        &root,
                        error,
                        RestoreFailures {
                            config: Some(RestoreFileError::Repository(
                                restore.for_operation(operation, &root),
                            )),
                            exclude: None,
                            identity: None,
                            head: None,
                            branch: None,
                            directory: None,
                        },
                    ));
                }
                return Err(error);
            }
        };
        self.advance_lifecycle(&root, operation, record, "publication_committed")?;
        match self.reconcile_registration(&repository, &root, operation) {
            Ok(()) => {
                self.complete_lifecycle(&root, operation, record)?;
                Ok(PublicationRemoteOutcome::Changed { commit_oid })
            }
            Err(_) => Ok(PublicationRemoteOutcome::RegistrationPending { commit_oid }),
        }
    }

    #[doc(hidden)]
    pub fn open_at_with_failure_point_for_testing(
        data_directory: &Path,
        failure_point: FailurePoint,
    ) -> Result<Self, RepositoryError> {
        let service = Self::open_at(data_directory)?;
        *service.failure_point.lock().map_err(|_| {
            RepositoryError::new(
                RepositoryOperation::OpenRegistry,
                None,
                RepositoryErrorKind::InjectedFailure,
                "the fixed test failure state is unavailable",
            )
        })? = Some(failure_point);
        Ok(service)
    }

    /// Creates and enables a new repository at `request.root`.
    ///
    /// The caller must provide a trusted, non-shared parent directory. This
    /// path-based API cannot prevent another party from replacing the target
    /// between filesystem operations.
    pub fn create_and_enable(
        &self,
        request: CreateRepositoryRequest,
    ) -> Result<EnableRepositoryOutcome, RepositoryError> {
        self.require_index_available(RepositoryOperation::CreateAndEnable, Some(&request.root))?;
        canonical_configuration(
            &request.primary_branch,
            &request.root,
            RepositoryOperation::CreateAndEnable,
        )?;
        let identity = match request.identity {
            Some(identity) => {
                validate_identity(&identity, RepositoryOperation::CreateAndEnable)?;
                identity
            }
            None => return Ok(EnableRepositoryOutcome::IdentityRequired),
        };
        let parent = request.root.parent().ok_or_else(|| {
            RepositoryError::new(
                RepositoryOperation::CreateAndEnable,
                None,
                RepositoryErrorKind::InvalidPath,
                "the selected target has no parent directory",
            )
        })?;
        let name = request.root.file_name().ok_or_else(|| {
            RepositoryError::new(
                RepositoryOperation::CreateAndEnable,
                None,
                RepositoryErrorKind::InvalidPath,
                "the selected target must name a directory",
            )
        })?;
        let parent = std::fs::canonicalize(parent)
            .map_err(|error| canonicalization_error(RepositoryOperation::CreateAndEnable, error))?;
        if !parent.is_dir() {
            return Err(RepositoryError::new(
                RepositoryOperation::CreateAndEnable,
                Some(parent),
                RepositoryErrorKind::InvalidPath,
                "the selected target parent is not a directory",
            ));
        }
        let root = parent.join(name);
        registry_root_key(&root, RepositoryOperation::CreateAndEnable)?;
        let bootstrap = bootstrap_lease(
            self.registry_path
                .parent()
                .unwrap_or_else(|| Path::new(".")),
            &root,
            RepositoryOperation::CreateAndEnable,
        )?;
        let resumes_create = self
            .recovery_inspection(&root)
            .unwrap_or_default()
            .iter()
            .any(|inspection| {
                matches!(
                    inspection,
                    RecoveryInspection::Pending {
                        operation: RepositoryOperation::CreateAndEnable,
                        operation_id: pending_id,
                        completed_step: Some(step),
                        ..
                    } if *pending_id == request.operation_id
                        && matches!(
                            step.as_str(),
                            "repository_initialized"
                                | "configuration_written"
                                | "initialization_committed"
                        )
                )
            });
        self.begin_lifecycle_record(
            &root,
            RepositoryOperation::CreateAndEnable,
            request.operation_id,
            &request.primary_branch,
        )?;
        if resumes_create && Repository::open(&root).is_ok() {
            let repository = Repository::open(&root).map_err(|error| {
                RepositoryError::git(
                    RepositoryOperation::CreateAndEnable,
                    Some(root.clone()),
                    error,
                )
            })?;
            let lease = repository_lease(&repository, &root, RepositoryOperation::CreateAndEnable)?;
            self.begin_lifecycle_record(
                &root,
                RepositoryOperation::CreateAndEnable,
                request.operation_id,
                &request.primary_branch,
            )?;
            drop(bootstrap);
            drop(lease);
            return self
                .enable(EnableRepositoryRequest {
                    root,
                    primary_branch: request.primary_branch,
                    identity: Some(identity),
                    operation_id: request.operation_id,
                })
                .map_err(|error| {
                    error.for_operation(RepositoryOperation::CreateAndEnable, &request.root)
                });
        }
        let created_target = match std::fs::symlink_metadata(&root) {
            Ok(metadata) if !metadata.is_dir() => {
                return Err(RepositoryError::new(
                    RepositoryOperation::CreateAndEnable,
                    Some(root),
                    RepositoryErrorKind::InvalidPath,
                    "the selected target is not a directory",
                ));
            }
            Ok(_) => {
                if std::fs::read_dir(&root)
                    .map_err(|error| {
                        RepositoryError::io(
                            RepositoryOperation::CreateAndEnable,
                            Some(root.clone()),
                            error,
                        )
                    })?
                    .next()
                    .transpose()
                    .map_err(|error| {
                        RepositoryError::io(
                            RepositoryOperation::CreateAndEnable,
                            Some(root.clone()),
                            error,
                        )
                    })?
                    .is_some()
                {
                    return Err(RepositoryError::new(
                        RepositoryOperation::CreateAndEnable,
                        Some(root),
                        RepositoryErrorKind::InvalidPath,
                        "the selected target directory is not empty",
                    ));
                }
                false
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir(&root).map_err(|error| {
                    RepositoryError::io(
                        RepositoryOperation::CreateAndEnable,
                        Some(root.clone()),
                        error,
                    )
                })?;
                true
            }
            Err(error) => {
                return Err(RepositoryError::io(
                    RepositoryOperation::CreateAndEnable,
                    Some(root),
                    error,
                ));
            }
        };
        validate_creation_target(&root)?;
        let owned_target = created_target
            .then(|| OwnedTarget::new(&root))
            .transpose()?;
        let result = (|| {
            let injected = self.should_inject(
                FailurePoint::BeforeRepositoryInitialization,
                RepositoryOperation::CreateAndEnable,
                &root,
            )?;
            validate_creation_target(&root)?;
            if injected {
                return Err(RepositoryError::new(
                    RepositoryOperation::CreateAndEnable,
                    Some(root.clone()),
                    RepositoryErrorKind::InjectedFailure,
                    "BeforeRepositoryInitialization was interrupted by a test failure hook",
                ));
            }
            let mut options = RepositoryInitOptions::new();
            options.initial_head(&request.primary_branch);
            Repository::init_opts(&root, &options).map_err(|error| {
                RepositoryError::git(
                    RepositoryOperation::CreateAndEnable,
                    Some(root.clone()),
                    error,
                )
            })?;
            let initialized = Repository::open(&root).map_err(|error| {
                RepositoryError::git(
                    RepositoryOperation::CreateAndEnable,
                    Some(root.clone()),
                    error,
                )
            })?;
            self.advance_lifecycle(
                &root,
                RepositoryOperation::CreateAndEnable,
                self.begin_lifecycle_record(
                    &root,
                    RepositoryOperation::CreateAndEnable,
                    request.operation_id,
                    &request.primary_branch,
                )?,
                "repository_initialized",
            )?;
            self.check_failure(
                FailurePoint::AfterRepositoryInitialization,
                RepositoryOperation::CreateAndEnable,
                &root,
            )?;
            // Serialize the handoff from path bootstrap to the common Git lease.
            let repository = initialized;
            let lease = repository_lease(&repository, &root, RepositoryOperation::CreateAndEnable)?;
            self.begin_lifecycle_record(
                &root,
                RepositoryOperation::CreateAndEnable,
                request.operation_id,
                &request.primary_branch,
            )?;
            drop(bootstrap);
            drop(lease);
            self.enable(EnableRepositoryRequest {
                root: root.clone(),
                primary_branch: request.primary_branch,
                identity: Some(identity),
                operation_id: request.operation_id,
            })
            .map_err(|error| error.for_operation(RepositoryOperation::CreateAndEnable, &root))
        })();
        if let Err(original) = result {
            if let Some(owned_target) = owned_target
                && let Err(cleanup) = owned_target.remove_if_empty()
            {
                return Err(creation_cleanup_incomplete(&root, original, cleanup));
            }
            return Err(original);
        }
        result
    }

    #[doc(hidden)]
    pub fn enable_with_identity_config_for_testing(
        &self,
        request: EnableRepositoryRequest,
        effective_config: &mut Config,
    ) -> Result<EnableRepositoryOutcome, RepositoryError> {
        self.enable_with_identity_provider(
            request,
            &mut SuppliedIdentityConfig { effective_config },
        )
    }

    fn enable_with_identity_provider(
        &self,
        request: EnableRepositoryRequest,
        identity_config: &mut impl IdentityConfigProvider,
    ) -> Result<EnableRepositoryOutcome, RepositoryError> {
        self.require_index_available(RepositoryOperation::Enable, Some(&request.root))?;
        let (repository, root) =
            canonical_repository_root(&request.root, RepositoryOperation::Enable)?;
        registry_root_key(&root, RepositoryOperation::Enable)?;
        let (_lease, record) = self.begin_lifecycle(
            &repository,
            &root,
            RepositoryOperation::Enable,
            request.operation_id,
            &request.primary_branch,
        )?;
        let requested_config =
            canonical_configuration(&request.primary_branch, &root, RepositoryOperation::Enable)?;
        let unborn = repository.is_empty().map_err(|error| {
            RepositoryError::git(RepositoryOperation::Enable, Some(root.clone()), error)
        })?;
        if !unborn {
            let branch = checked_out_branch(&repository, &root, RepositoryOperation::Enable)?;
            if branch != request.primary_branch {
                return Err(RepositoryError::new(
                    RepositoryOperation::Enable,
                    Some(root),
                    RepositoryErrorKind::WrongCheckedOutBranch,
                    "the requested primary branch is not checked out at the repository root",
                ));
            }
        }
        match worktree_state(&repository, &root)? {
            WorktreeState::Clean => {}
            WorktreeState::Dirty => {
                return Err(RepositoryError::new(
                    RepositoryOperation::Enable,
                    Some(root),
                    RepositoryErrorKind::DirtyWorktree,
                    "initialization requires a clean worktree",
                ));
            }
            WorktreeState::Conflicted => {
                return Err(RepositoryError::new(
                    RepositoryOperation::Enable,
                    Some(root),
                    RepositoryErrorKind::ConflictedWorktree,
                    "initialization requires a non-conflicted worktree",
                ));
            }
        }

        let configuration = read_configuration_for(&root, RepositoryOperation::Enable)?;
        match configuration {
            ConfigurationInspection::Invalid(problem) => {
                return Err(RepositoryError::new(
                    RepositoryOperation::Enable,
                    Some(root),
                    RepositoryErrorKind::InvalidConfiguration,
                    problem.message,
                ));
            }
            ConfigurationInspection::Valid(config) => {
                if config.primary_branch != request.primary_branch {
                    return Err(RepositoryError::new(
                        RepositoryOperation::Enable,
                        Some(root),
                        RepositoryErrorKind::InvalidConfiguration,
                        "the configured primary branch differs from the requested branch",
                    ));
                }
                if let Some(remote) = config.publication_remote
                    && !publication_remote_eligible(&repository, &root, &remote)?
                {
                    return Err(RepositoryError::new(
                        RepositoryOperation::Enable,
                        Some(root),
                        RepositoryErrorKind::UnavailablePublicationRemote,
                        "the configured publication remote is unavailable or ineligible",
                    ));
                }
                ensure_worktree_exclusion(&repository, &root)?;
                self.reconcile_registration(&repository, &root, RepositoryOperation::Enable)?;
                self.complete_lifecycle(&root, RepositoryOperation::Enable, record)?;
                return Ok(EnableRepositoryOutcome::AlreadyEnabled);
            }
            ConfigurationInspection::Missing => {}
        }

        let (identity, write_identity) = match request.identity {
            Some(identity) => {
                validate_identity(&identity, RepositoryOperation::Enable)?;
                (identity, true)
            }
            None => {
                let local_config = repository.config().map_err(|error| {
                    RepositoryError::git(RepositoryOperation::Enable, Some(root.clone()), error)
                })?;
                let effective_config = identity_config.effective_config(
                    &repository,
                    &root,
                    RepositoryOperation::Enable,
                )?;
                let Some(identity) = resolve_identity(
                    &local_config,
                    &effective_config,
                    &root,
                    RepositoryOperation::Enable,
                )?
                else {
                    self.complete_lifecycle(&root, RepositoryOperation::Enable, record)?;
                    return Ok(EnableRepositoryOutcome::IdentityRequired);
                };
                (identity, false)
            }
        };

        let config_path = root.join(canonical::CONFIG_PATH);
        let config_before = read_bytes_if_exists(&config_path, RepositoryOperation::Enable, &root)?;
        let exclude_path = repository.commondir().join("info/exclude");
        let exclude_before =
            read_bytes_if_exists(&exclude_path, RepositoryOperation::Enable, &root)?;
        let original_head_target = unborn
            .then(|| snapshot_unborn_head(&repository, &root))
            .transpose()?;
        let local_config_path = repository.commondir().join("config");
        let local_config_before =
            read_bytes_if_exists(&local_config_path, RepositoryOperation::Enable, &root)?;
        let mut created_manyhands_directory = false;
        let result = (|| {
            if write_identity {
                write_local_identity(&repository, &root, &identity)?;
            }
            if unborn {
                let reference = format!("refs/heads/{}", request.primary_branch);
                repository.set_head(&reference).map_err(|error| {
                    RepositoryError::git(RepositoryOperation::Enable, Some(root.clone()), error)
                })?;
            }
            ensure_worktree_exclusion(&repository, &root)?;
            created_manyhands_directory = prepare_configuration_parent(&root).map_err(|error| {
                RepositoryError::io(RepositoryOperation::Enable, Some(root.clone()), error)
            })?;
            self.check_failure(
                FailurePoint::BeforeConfigurationWrite,
                RepositoryOperation::Enable,
                &root,
            )?;
            write_configuration_atomically(&config_path, &requested_config, &root)?;
            self.advance_lifecycle(
                &root,
                RepositoryOperation::Enable,
                record,
                "configuration_written",
            )?;
            let tree_id = initialization_tree(&repository, &config_path, unborn, &root)?;
            let tree = repository.find_tree(tree_id).map_err(|error| {
                RepositoryError::git(RepositoryOperation::Enable, Some(root.clone()), error)
            })?;
            let signature = Signature::now(&identity.name, &identity.email).map_err(|error| {
                RepositoryError::git(RepositoryOperation::Enable, Some(root.clone()), error)
            })?;
            let parents = if unborn {
                Vec::new()
            } else {
                vec![
                    repository
                        .head()
                        .and_then(|head| head.peel_to_commit())
                        .map_err(|error| {
                            RepositoryError::git(
                                RepositoryOperation::Enable,
                                Some(root.clone()),
                                error,
                            )
                        })?,
                ]
            };
            self.check_failure(
                FailurePoint::BeforeInitializationCommit,
                RepositoryOperation::Enable,
                &root,
            )?;
            repository
                .commit(
                    Some("HEAD"),
                    &signature,
                    &signature,
                    "Initialize Manyhands",
                    &tree,
                    &parents.iter().collect::<Vec<_>>(),
                )
                .map(|commit_oid| EnableRepositoryOutcome::Enabled { commit_oid })
                .map_err(|error| {
                    RepositoryError::git(RepositoryOperation::Enable, Some(root.clone()), error)
                })
        })();
        let commit_oid = match result {
            Ok(EnableRepositoryOutcome::Enabled { commit_oid }) => commit_oid,
            Ok(_) => unreachable!("initialization only returns an enabled commit"),
            Err(original) => {
                let rollback = restore_enablement(
                    &config_path,
                    config_before.as_deref(),
                    &exclude_path,
                    exclude_before.as_deref(),
                    write_identity.then_some((&local_config_path, local_config_before.as_deref())),
                    created_manyhands_directory.then_some(config_path.parent().unwrap()),
                    unborn.then_some((
                        &repository,
                        original_head_target.as_deref(),
                        &request.primary_branch,
                    )),
                );
                return match rollback {
                    Ok(()) => Err(original),
                    Err(failures) => Err(rollback_incomplete(&root, original, *failures)),
                };
            }
        };
        self.advance_lifecycle(
            &root,
            RepositoryOperation::Enable,
            record,
            "initialization_committed",
        )?;
        match self.reconcile_registration(&repository, &root, RepositoryOperation::Enable) {
            Ok(()) => {
                self.complete_lifecycle(&root, RepositoryOperation::Enable, record)?;
                Ok(EnableRepositoryOutcome::Enabled { commit_oid })
            }
            Err(_) => Ok(EnableRepositoryOutcome::RegistrationPending { commit_oid }),
        }
    }

    #[doc(hidden)]
    pub fn inspect_with_identity_config_for_testing(
        &self,
        root: &Path,
        effective_config: &mut Config,
    ) -> Result<RepositoryInspection, RepositoryError> {
        self.inspect_with_identity_provider(root, &mut SuppliedIdentityConfig { effective_config })
    }

    fn inspect_with_identity_provider(
        &self,
        selected: &Path,
        identity_config: &mut impl IdentityConfigProvider,
    ) -> Result<RepositoryInspection, RepositoryError> {
        self.require_index_available(RepositoryOperation::Inspect, Some(selected))?;
        let (repository, root) = canonical_repository_root(selected, RepositoryOperation::Inspect)?;
        let head_branch = checked_out_branch(&repository, &root, RepositoryOperation::Inspect)?;
        let local_branches = local_branches(&repository, &root)?;
        let configuration = read_configuration(&root)?;
        let local_config = repository.config().map_err(|error| {
            RepositoryError::git(RepositoryOperation::Inspect, Some(root.clone()), error)
        })?;
        let effective_config =
            identity_config.effective_config(&repository, &root, RepositoryOperation::Inspect)?;
        let identity = if resolve_identity(
            &local_config,
            &effective_config,
            &root,
            RepositoryOperation::Inspect,
        )?
        .is_some()
        {
            IdentityInspection::Available
        } else {
            IdentityInspection::Required
        };
        let remotes = remote_info(&repository, &root)?;

        Ok(RepositoryInspection {
            root,
            head_branch: Some(head_branch),
            local_branches,
            configuration,
            identity,
            remotes,
        })
    }

    #[doc(hidden)]
    pub fn with_registry_connection_for_testing<T>(
        &self,
        inspect: impl FnOnce(&rusqlite::Connection) -> T,
    ) -> Result<T, RepositoryError> {
        self.require_index_available(RepositoryOperation::OpenRegistry, None)?;
        let _cache_guard = cache_read_guard(
            &self.registry_path,
            self.registry_path
                .parent()
                .unwrap_or_else(|| Path::new(".")),
            RepositoryOperation::OpenRegistry,
        )?;
        let connection = open_registry(&self.registry_path, &mut |_| {})?;
        Ok(inspect(&connection))
    }

    #[doc(hidden)]
    pub fn set_observation_hook_for_testing(&self, hook: impl FnOnce() + Send + 'static) {
        *self
            .observation_hook
            .lock()
            .expect("test observation hook lock") = Some(Box::new(hook));
    }

    #[doc(hidden)]
    pub fn set_registration_git_hook_for_testing(&self, hook: impl FnOnce() + Send + 'static) {
        *self
            .registration_git_hook
            .lock()
            .expect("test registration Git hook lock") = Some(Box::new(hook));
    }

    #[doc(hidden)]
    pub fn set_corrupt_cache_decision_hook_for_testing(
        &self,
        hook: impl FnOnce() + Send + 'static,
    ) {
        *self
            .corrupt_cache_decision_hook
            .lock()
            .expect("test corrupt-cache decision hook lock") = Some(Box::new(hook));
    }

    #[doc(hidden)]
    pub fn set_corrupt_cache_critical_hook_for_testing(
        &self,
        hook: impl FnOnce() + Send + 'static,
    ) {
        *self
            .corrupt_cache_critical_hook
            .lock()
            .expect("test corrupt-cache critical hook lock") = Some(Box::new(hook));
    }

    #[doc(hidden)]
    pub fn set_rebuild_error_hook_for_testing(&self, hook: impl FnOnce() + Send + 'static) {
        *self
            .rebuild_error_hook
            .lock()
            .expect("test rebuild error hook lock") = Some(Box::new(hook));
    }

    pub fn remove_registration(
        &self,
        request: RemoveRegistrationRequest,
    ) -> Result<RemoveRegistrationOutcome, RepositoryError> {
        let root = &request.root;
        self.require_index_available(RepositoryOperation::RemoveRegistration, Some(root))?;
        let root = std::fs::canonicalize(root).map_err(|error| {
            canonicalization_error(RepositoryOperation::RemoveRegistration, error)
        })?;
        if !root.is_dir() {
            return Err(RepositoryError::new(
                RepositoryOperation::RemoveRegistration,
                None,
                RepositoryErrorKind::InvalidPath,
                "the selected path is not a directory",
            ));
        }
        let root_path = registry_root_key(&root, RepositoryOperation::RemoveRegistration)?;
        let repository = Repository::open(&root).map_err(|error| {
            RepositoryError::git(
                RepositoryOperation::RemoveRegistration,
                Some(root.clone()),
                error,
            )
        })?;
        let _lease = repository_lease(&repository, &root, RepositoryOperation::RemoveRegistration)?;
        let _cache_guard = cache_write_guard(
            &self.registry_path,
            &root,
            RepositoryOperation::RemoveRegistration,
        )?;
        let mut connection = open_registry(&self.registry_path, &mut |_| {})
            .map_err(|error| error.for_operation(RepositoryOperation::RemoveRegistration, &root))?;
        migrate_registry(&mut connection)
            .map_err(|error| error.for_operation(RepositoryOperation::RemoveRegistration, &root))?;
        let _record = begin_or_reconcile_operation(
            &mut connection,
            &root,
            RepositoryOperation::RemoveRegistration,
            request.operation_id,
            "",
        )
        .map_err(|error| error.for_operation(RepositoryOperation::RemoveRegistration, &root))?;
        self.check_failure(
            FailurePoint::BeforeRegistrationRemovalTransaction,
            RepositoryOperation::RemoveRegistration,
            &root,
        )?;
        let transaction = connection.transaction().map_err(|error| {
            RepositoryError::sqlite(error)
                .for_operation(RepositoryOperation::RemoveRegistration, &root)
        })?;
        transaction
            .execute(
                "DELETE FROM operation_records WHERE root_path = ?1",
                [root_path],
            )
            .map_err(|error| {
                RepositoryError::sqlite(error)
                    .for_operation(RepositoryOperation::RemoveRegistration, &root)
            })?;
        let deleted = transaction
            .execute("DELETE FROM repositories WHERE root_path = ?1", [root_path])
            .map_err(|error| {
                RepositoryError::sqlite(error)
                    .for_operation(RepositoryOperation::RemoveRegistration, &root)
            })?;
        transaction.commit().map_err(|error| {
            RepositoryError::sqlite(error)
                .for_operation(RepositoryOperation::RemoveRegistration, &root)
        })?;
        Ok(if deleted == 0 {
            RemoveRegistrationOutcome::NotRegistered
        } else {
            RemoveRegistrationOutcome::Removed
        })
    }
}

fn is_structural_sqlite_corruption(error: &RepositoryError) -> bool {
    error
        .source
        .as_deref()
        .and_then(|source| source.downcast_ref::<rusqlite::Error>())
        .is_some_and(|error| {
            matches!(
                error,
                rusqlite::Error::SqliteFailure(code, _)
                    if matches!(
                        code.code,
                        rusqlite::ErrorCode::DatabaseCorrupt | rusqlite::ErrorCode::NotADatabase
                    )
            )
        })
}

fn replace_corrupt_registry(registry_path: &Path, root: &Path) -> Result<(), RepositoryError> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            RepositoryError::new(
                RepositoryOperation::RebuildRepository,
                Some(root.to_owned()),
                RepositoryErrorKind::Io,
                error,
            )
        })?
        .as_nanos();
    let diagnostic = registry_path.with_file_name(format!("{REGISTRY_FILE}.corrupt-{timestamp}"));
    std::fs::rename(registry_path, &diagnostic).map_err(|error| {
        RepositoryError::io(
            RepositoryOperation::RebuildRepository,
            Some(root.to_owned()),
            error,
        )
    })?;
    for suffix in ["-wal", "-shm"] {
        let sidecar = registry_path.with_file_name(format!("{REGISTRY_FILE}{suffix}"));
        let diagnostic_sidecar = diagnostic.with_file_name(format!(
            "{}{}",
            diagnostic
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default(),
            suffix
        ));
        if sidecar.exists() {
            let _ = std::fs::rename(sidecar, diagnostic_sidecar);
        }
    }
    Ok(())
}

fn begin_operation(
    registry_path: &Path,
    root: &Path,
    operation: RepositoryOperation,
    operation_id: OperationId,
) -> Result<recovery::RecoveryRecord, RepositoryError> {
    let _cache_guard = cache_read_guard(registry_path, root, operation)?;
    let mut connection = open_registry(registry_path, &mut |_| {})
        .map_err(|error| error.for_operation(operation, root))?;
    begin_or_reconcile_operation(&mut connection, root, operation, operation_id, "")
        .map_err(|error| error.for_operation(operation, root))
}

fn registered_repository_id(
    registry_path: &Path,
    root: &Path,
    operation: RepositoryOperation,
) -> Result<i64, RepositoryError> {
    let _cache_guard = cache_read_guard(registry_path, root, operation)?;
    let root_path = registry_root_key(root, operation)?;
    let connection = open_registry(registry_path, &mut |_| {})
        .map_err(|error| error.for_operation(operation, root))?;
    connection
        .query_row(
            "SELECT id FROM repositories WHERE root_path = ?1",
            [root_path],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| RepositoryError::sqlite(error).for_operation(operation, root))?
        .ok_or_else(|| {
            RepositoryError::new(
                operation,
                Some(root.to_owned()),
                RepositoryErrorKind::RepositoryNotRegistered,
                "the repository is not registered",
            )
        })
}

fn set_rebuild_operation(
    registry_path: &Path,
    operation_id: i64,
    state: &str,
    root: &Path,
) -> Result<(), RepositoryError> {
    let _cache_guard =
        cache_read_guard(registry_path, root, RepositoryOperation::RebuildRepository)?;
    let connection = open_registry(registry_path, &mut |_| {})
        .map_err(|error| error.for_operation(RepositoryOperation::RebuildRepository, root))?;
    advance_after_observation(&connection, operation_id, state, None, None)
        .map_err(|error| error.for_operation(RepositoryOperation::RebuildRepository, root))?;
    Ok(())
}

fn persist_rebuild_observation(
    registry_path: &Path,
    operation_id: i64,
    root: &Path,
    observation: &RootObservation,
) -> Result<(), RepositoryError> {
    let operation = RepositoryOperation::RebuildRepository;
    let _cache_guard = cache_read_guard(registry_path, root, operation)?;
    let root_path = registry_root_key(root, operation)?;
    let persisted_context_count =
        i64::try_from(1 + observation.active_contexts.len()).map_err(|_| {
            RepositoryError::new(
                operation,
                Some(root.to_owned()),
                RepositoryErrorKind::Sqlite,
                "too many observed contexts to persist",
            )
        })?;
    let mut connection = open_registry(registry_path, &mut |_| {})
        .map_err(|error| error.for_operation(operation, root))?;
    let transaction = connection
        .transaction()
        .map_err(|error| RepositoryError::sqlite(error).for_operation(operation, root))?;
    transaction
        .execute(
            "INSERT INTO repositories (root_path, enabled_at, accessibility, config_blob_oid, refresh_required)
             VALUES (?1, ?2, 'accessible', NULL, 0)
             ON CONFLICT(root_path) DO UPDATE SET accessibility = 'accessible', refresh_required = 0",
            params![root_path, OffsetDateTime::now_utc().unix_timestamp()],
        )
        .map_err(|error| RepositoryError::sqlite(error).for_operation(operation, root))?;
    let repository_id: i64 = transaction
        .query_row(
            "SELECT id FROM repositories WHERE root_path = ?1",
            [root_path],
            |row| row.get(0),
        )
        .map_err(|error| RepositoryError::sqlite(error).for_operation(operation, root))?;
    transaction
        .execute(
            "UPDATE operation_records SET repository_id = ?2 WHERE id = ?1 AND root_path = ?3",
            params![operation_id, repository_id, root_path],
        )
        .map_err(|error| RepositoryError::sqlite(error).for_operation(operation, root))?;
    persist_observation(&transaction, repository_id, observation, root)
        .map_err(|error| error.for_operation(operation, root))?;
    transaction
        .execute(
            "UPDATE repositories SET accessibility = 'accessible', refresh_required = 0 WHERE id = ?1",
            [repository_id],
        )
        .map_err(|error| RepositoryError::sqlite(error).for_operation(operation, root))?;
    transaction
        .commit()
        .map_err(|error| RepositoryError::sqlite(error).for_operation(operation, root))?;
    let connection = open_registry(registry_path, &mut |_| {})
        .map_err(|error| error.for_operation(operation, root))?;
    advance_after_observation(
        &connection,
        operation_id,
        "persisted",
        None,
        Some(persisted_context_count),
    )
    .map_err(|error| error.for_operation(operation, root))
}

fn persist_rebuild_retry(
    registry_path: &Path,
    operation_id: i64,
    root: &Path,
) -> Result<(), RepositoryError> {
    let operation = RepositoryOperation::RebuildRepository;
    let _cache_guard = cache_read_guard(registry_path, root, operation)?;
    let root_path = registry_root_key(root, operation)?;
    let mut connection = open_registry(registry_path, &mut |_| {})
        .map_err(|error| error.for_operation(operation, root))?;
    let transaction = connection
        .transaction()
        .map_err(|error| RepositoryError::sqlite(error).for_operation(operation, root))?;
    let repository_id: i64 = transaction
        .query_row(
            "SELECT id FROM repositories WHERE root_path = ?1",
            [root_path],
            |row| row.get(0),
        )
        .map_err(|error| RepositoryError::sqlite(error).for_operation(operation, root))?;
    transaction
        .execute(
            "UPDATE repositories SET refresh_required = 1 WHERE id = ?1",
            [repository_id],
        )
        .map_err(|error| RepositoryError::sqlite(error).for_operation(operation, root))?;
    transaction
        .execute(
            "INSERT INTO problems (repository_id, code, guidance, observed_at)
             VALUES (?1, 'retry-required', 'the repository changed while it was being observed; rebuild again', ?2)",
            params![repository_id, OffsetDateTime::now_utc().unix_timestamp()],
        )
        .map_err(|error| RepositoryError::sqlite(error).for_operation(operation, root))?;
    transaction
        .commit()
        .map_err(|error| RepositoryError::sqlite(error).for_operation(operation, root))?;
    let connection = open_registry(registry_path, &mut |_| {})
        .map_err(|error| error.for_operation(operation, root))?;
    advance_after_observation(&connection, operation_id, "retry", None, None)
        .map_err(|error| error.for_operation(operation, root))
}

fn read_repository_snapshot_from_registry(
    registry_path: &Path,
    root: &Path,
) -> Result<RepositorySnapshot, RepositoryError> {
    let _cache_guard =
        cache_read_guard(registry_path, root, RepositoryOperation::RebuildRepository)?;
    let root_path = registry_root_key(root, RepositoryOperation::RebuildRepository)?;
    let mut connection = open_registry_read_only(registry_path)
        .map_err(|error| error.for_operation(RepositoryOperation::RebuildRepository, root))?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .map_err(|error| {
            RepositoryError::sqlite(error)
                .for_operation(RepositoryOperation::RebuildRepository, root)
        })?;
    let snapshot = read_repository_snapshot(&transaction, root, root_path)
        .map_err(|error| error.for_operation(RepositoryOperation::RebuildRepository, root));
    transaction.rollback().map_err(|error| {
        RepositoryError::sqlite(error).for_operation(RepositoryOperation::RebuildRepository, root)
    })?;
    snapshot
}

fn set_refresh_operation(
    registry_path: &Path,
    operation_id: i64,
    state: &str,
    context: Option<&Path>,
    root: &Path,
) -> Result<(), RepositoryError> {
    let _cache_guard =
        cache_read_guard(registry_path, root, RepositoryOperation::RefreshRepository)?;
    let connection = open_registry(registry_path, &mut |_| {})
        .map_err(|error| error.for_operation(RepositoryOperation::RefreshRepository, root))?;
    advance_after_observation(&connection, operation_id, state, context, None)
        .map_err(|error| error.for_operation(RepositoryOperation::RefreshRepository, root))?;
    Ok(())
}

#[allow(dead_code)]
fn record_persisted_context(
    registry_path: &Path,
    operation_id: i64,
    context: &Path,
    root: &Path,
) -> Result<(), RepositoryError> {
    let _cache_guard =
        cache_read_guard(registry_path, root, RepositoryOperation::RefreshRepository)?;
    let connection = open_registry(registry_path, &mut |_| {})
        .map_err(|error| error.for_operation(RepositoryOperation::RefreshRepository, root))?;
    record_recovery_context(&connection, operation_id, context)
        .map_err(|error| error.for_operation(RepositoryOperation::RefreshRepository, root))?;
    Ok(())
}

fn changed_contexts(
    before: &RootObservation,
    after: &RootObservation,
    root: &Path,
) -> BTreeSet<PathBuf> {
    let mut changed = BTreeSet::new();
    if before.head.oid != after.head.oid
        || before.head.branch != after.head.branch
        || before.configuration_source != after.configuration_source
        || before.sources != after.sources
        || format!(
            "{:?}{:?}{:?}{:?}",
            before.context, before.items, before.problems, before.validation
        ) != format!(
            "{:?}{:?}{:?}{:?}",
            after.context, after.items, after.problems, after.validation
        )
    {
        changed.insert(root.to_owned());
    }
    for context in &before.active_contexts {
        let stable = after
            .active_contexts
            .iter()
            .find(|candidate| candidate.context.worktree == context.context.worktree)
            .is_some_and(|candidate| {
                candidate.head.oid == context.head.oid
                    && candidate.head.branch == context.head.branch
                    && candidate.sources == context.sources
                    && format!(
                        "{:?}{:?}{:?}{:?}",
                        candidate.context,
                        candidate.items,
                        candidate.problems,
                        candidate.validation
                    ) == format!(
                        "{:?}{:?}{:?}{:?}",
                        context.context, context.items, context.problems, context.validation
                    )
            });
        if !stable {
            changed.insert(context.context.worktree.clone());
        }
    }
    for context in &after.active_contexts {
        if !before
            .active_contexts
            .iter()
            .any(|candidate| candidate.context.worktree == context.context.worktree)
        {
            changed.insert(context.context.worktree.clone());
        }
    }
    changed
}

#[allow(clippy::too_many_arguments)]
fn persist_context_refresh(
    registry_path: &Path,
    repository_id: i64,
    operation_id: i64,
    root_observation: &RootObservation,
    context: &discovery::RootContextObservation,
    head: &discovery::RootHeadObservation,
    items: &[ObservedItem],
    problems: &[RootObservationProblem],
    validation: &[canonical::ValidationProblem],
    excluded: &BTreeSet<canonical::ItemId>,
    root: &Path,
) -> Result<(), RepositoryError> {
    let _cache_guard =
        cache_read_guard(registry_path, root, RepositoryOperation::RefreshRepository)?;
    let mut connection = open_registry(registry_path, &mut |_| {})
        .map_err(|error| error.for_operation(RepositoryOperation::RefreshRepository, root))?;
    let transaction = connection.transaction().map_err(|error| {
        RepositoryError::sqlite(error).for_operation(RepositoryOperation::RefreshRepository, root)
    })?;
    transaction
        .execute(
            "DELETE FROM contexts WHERE repository_id = ?1 AND worktree_path = ?2",
            params![repository_id, context.worktree.to_str()],
        )
        .map_err(|error| {
            RepositoryError::sqlite(error)
                .for_operation(RepositoryOperation::RefreshRepository, root)
        })?;
    if context.worktree == root {
        transaction
            .execute(
                "DELETE FROM configuration_observations WHERE repository_id = ?1",
                [repository_id],
            )
            .map_err(|error| {
                RepositoryError::sqlite(error)
                    .for_operation(RepositoryOperation::RefreshRepository, root)
            })?;
        transaction
            .execute(
                "UPDATE repositories SET config_blob_oid = ?2 WHERE id = ?1",
                params![
                    repository_id,
                    matches!(root_observation.configuration, RootConfiguration::Valid(_))
                        .then(|| root_observation
                            .configuration_blob_oid
                            .map(|oid| oid.to_string()))
                        .flatten()
                ],
            )
            .map_err(|error| {
                RepositoryError::sqlite(error)
                    .for_operation(RepositoryOperation::RefreshRepository, root)
            })?;
        match &root_observation.configuration {
            RootConfiguration::Valid(configuration) => {
                transaction.execute("INSERT INTO configuration_observations (repository_id, state, primary_branch, publication_remote) VALUES (?1, 'valid', ?2, ?3)", params![repository_id, configuration.primary_branch, configuration.publication_remote]).map_err(|error| RepositoryError::sqlite(error).for_operation(RepositoryOperation::RefreshRepository, root))?;
            }
            RootConfiguration::Missing => {
                transaction.execute("INSERT INTO configuration_observations (repository_id, state) VALUES (?1, 'missing')", [repository_id]).map_err(|error| RepositoryError::sqlite(error).for_operation(RepositoryOperation::RefreshRepository, root))?;
            }
            RootConfiguration::Invalid(problem) => {
                transaction.execute("INSERT INTO configuration_observations (repository_id, state, invalid_code, guidance) VALUES (?1, 'invalid', ?2, ?3)", params![repository_id, validation_code_name(problem.code.clone()), problem.message]).map_err(|error| RepositoryError::sqlite(error).for_operation(RepositoryOperation::RefreshRepository, root))?;
            }
        }
    }
    persist_context(
        &transaction,
        repository_id,
        PersistedContext {
            context,
            head,
            items,
            problems,
            validation_problems: validation,
            excluded_item_ids: excluded,
        },
        root,
    )?;
    transaction.commit().map_err(|error| {
        RepositoryError::sqlite(error).for_operation(RepositoryOperation::RefreshRepository, root)
    })?;
    let connection = open_registry(registry_path, &mut |_| {})
        .map_err(|error| error.for_operation(RepositoryOperation::RefreshRepository, root))?;
    record_recovery_context(&connection, operation_id, &context.worktree)
        .map_err(|error| error.for_operation(RepositoryOperation::RefreshRepository, root))
}

fn persist_retry_problem(
    registry_path: &Path,
    repository_id: i64,
    context: &Path,
    operation_id: i64,
    root: &Path,
) -> Result<(), RepositoryError> {
    let _cache_guard =
        cache_read_guard(registry_path, root, RepositoryOperation::RefreshRepository)?;
    let connection = open_registry(registry_path, &mut |_| {})
        .map_err(|error| error.for_operation(RepositoryOperation::RefreshRepository, root))?;
    let context_id: Option<i64> = connection
        .query_row(
            "SELECT id FROM contexts WHERE repository_id = ?1 AND worktree_path = ?2",
            params![repository_id, context.to_str()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| {
            RepositoryError::sqlite(error)
                .for_operation(RepositoryOperation::RefreshRepository, root)
        })?;
    connection
        .execute(
            "UPDATE repositories SET refresh_required = 1 WHERE id = ?1",
            [repository_id],
        )
        .map_err(|error| {
            RepositoryError::sqlite(error)
                .for_operation(RepositoryOperation::RefreshRepository, root)
        })?;
    connection.execute("INSERT INTO problems (repository_id, context_id, code, guidance, observed_at) VALUES (?1, ?2, 'retry-required', 'the context changed while it was being observed; refresh again', ?3)", params![repository_id, context_id, OffsetDateTime::now_utc().unix_timestamp()]).map_err(|error| RepositoryError::sqlite(error).for_operation(RepositoryOperation::RefreshRepository, root))?;
    set_refresh_operation(registry_path, operation_id, "retry", Some(context), root)
}

fn reconcile_disappeared_contexts(
    registry_path: &Path,
    repository_id: i64,
    observation: &RootObservation,
    operation_id: i64,
    root: &Path,
) -> Result<(), RepositoryError> {
    let _cache_guard =
        cache_read_guard(registry_path, root, RepositoryOperation::RefreshRepository)?;
    let mut connection = open_registry(registry_path, &mut |_| {})
        .map_err(|error| error.for_operation(RepositoryOperation::RefreshRepository, root))?;
    let transaction = connection.transaction().map_err(|error| {
        RepositoryError::sqlite(error).for_operation(RepositoryOperation::RefreshRepository, root)
    })?;
    let paths = std::iter::once(observation.context.worktree.clone())
        .chain(
            observation
                .active_contexts
                .iter()
                .map(|context| context.context.worktree.clone()),
        )
        .collect::<BTreeSet<_>>();
    let stored = transaction
        .prepare("SELECT worktree_path FROM contexts WHERE repository_id = ?1")
        .and_then(|mut statement| {
            statement
                .query_map([repository_id], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|error| {
            RepositoryError::sqlite(error)
                .for_operation(RepositoryOperation::RefreshRepository, root)
        })?;
    for path in stored
        .into_iter()
        .filter(|path| !paths.contains(&PathBuf::from(path)))
    {
        transaction
            .execute(
                "DELETE FROM contexts WHERE repository_id = ?1 AND worktree_path = ?2",
                params![repository_id, path],
            )
            .map_err(|error| {
                RepositoryError::sqlite(error)
                    .for_operation(RepositoryOperation::RefreshRepository, root)
            })?;
    }
    transaction
        .execute(
            "DELETE FROM problems WHERE repository_id = ?1 AND code = 'retry-required'",
            [repository_id],
        )
        .map_err(|error| {
            RepositoryError::sqlite(error)
                .for_operation(RepositoryOperation::RefreshRepository, root)
        })?;
    transaction.execute("UPDATE repositories SET accessibility = 'accessible', refresh_required = 0 WHERE id = ?1", [repository_id]).map_err(|error| RepositoryError::sqlite(error).for_operation(RepositoryOperation::RefreshRepository, root))?;
    transaction.commit().map_err(|error| {
        RepositoryError::sqlite(error).for_operation(RepositoryOperation::RefreshRepository, root)
    })?;
    let connection = open_registry(registry_path, &mut |_| {})
        .map_err(|error| error.for_operation(RepositoryOperation::RefreshRepository, root))?;
    advance_after_observation(&connection, operation_id, "completed", None, None)
        .map_err(|error| error.for_operation(RepositoryOperation::RefreshRepository, root))
}

#[allow(dead_code)]
fn observations_match(before: &RootObservation, after: &RootObservation) -> bool {
    before.head.oid == after.head.oid
        && before.head.branch == after.head.branch
        && before.configuration_source == after.configuration_source
        && before.sources == after.sources
        && before.active_contexts.len() == after.active_contexts.len()
        && before
            .active_contexts
            .iter()
            .zip(&after.active_contexts)
            .all(|(left, right)| {
                left.context.worktree == right.context.worktree
                    && left.head.oid == right.head.oid
                    && left.head.branch == right.head.branch
                    && left.sources == right.sources
            })
}

#[allow(dead_code)]
fn persist_observation(
    transaction: &rusqlite::Transaction<'_>,
    repository_id: i64,
    observation: &RootObservation,
    root: &Path,
) -> Result<(), RepositoryError> {
    let operation = RepositoryOperation::RefreshRepository;
    transaction
        .execute(
            "DELETE FROM contexts WHERE repository_id = ?1",
            [repository_id],
        )
        .map_err(|error| RepositoryError::sqlite(error).for_operation(operation, root))?;
    transaction
        .execute(
            "DELETE FROM problems WHERE repository_id = ?1",
            [repository_id],
        )
        .map_err(|error| RepositoryError::sqlite(error).for_operation(operation, root))?;
    transaction
        .execute(
            "DELETE FROM configuration_observations WHERE repository_id = ?1",
            [repository_id],
        )
        .map_err(|error| RepositoryError::sqlite(error).for_operation(operation, root))?;
    transaction
        .execute(
            "UPDATE repositories SET config_blob_oid = ?2 WHERE id = ?1",
            params![
                repository_id,
                matches!(observation.configuration, RootConfiguration::Valid(_))
                    .then(|| observation
                        .configuration_blob_oid
                        .map(|oid| oid.to_string()))
                    .flatten()
            ],
        )
        .map_err(|error| RepositoryError::sqlite(error).for_operation(operation, root))?;
    match &observation.configuration {
        RootConfiguration::Valid(configuration) => {
            transaction.execute(
                "INSERT INTO configuration_observations (repository_id, state, primary_branch, publication_remote) VALUES (?1, 'valid', ?2, ?3)",
                params![repository_id, configuration.primary_branch, configuration.publication_remote],
            )
        }
        RootConfiguration::Missing => transaction.execute(
            "INSERT INTO configuration_observations (repository_id, state) VALUES (?1, 'missing')", [repository_id],
        ),
        RootConfiguration::Invalid(problem) => transaction.execute(
            "INSERT INTO configuration_observations (repository_id, state, invalid_code, guidance) VALUES (?1, 'invalid', ?2, ?3)",
            params![repository_id, validation_code_name(problem.code.clone()), problem.message],
        ),
    }.map_err(|error| RepositoryError::sqlite(error).for_operation(operation, root))?;
    let active_item_ids = observation
        .active_contexts
        .iter()
        .flat_map(|context| context.items.iter().map(|item| item.id.clone()))
        .collect::<BTreeSet<_>>();
    persist_context(
        transaction,
        repository_id,
        PersistedContext {
            context: &observation.context,
            head: &observation.head,
            items: &observation.items,
            problems: &observation.problems,
            validation_problems: &observation.validation.problems,
            excluded_item_ids: &active_item_ids,
        },
        root,
    )?;
    for active in &observation.active_contexts {
        persist_context(
            transaction,
            repository_id,
            PersistedContext {
                context: &active.context,
                head: &active.head,
                items: &active.items,
                problems: &active.problems,
                validation_problems: &active.validation.problems,
                excluded_item_ids: &BTreeSet::new(),
            },
            root,
        )?;
    }
    Ok(())
}

struct PersistedContext<'a> {
    context: &'a discovery::RootContextObservation,
    head: &'a discovery::RootHeadObservation,
    items: &'a [ObservedItem],
    problems: &'a [RootObservationProblem],
    validation_problems: &'a [canonical::ValidationProblem],
    excluded_item_ids: &'a BTreeSet<canonical::ItemId>,
}

fn persist_context(
    transaction: &rusqlite::Transaction<'_>,
    repository_id: i64,
    observed: PersistedContext<'_>,
    root: &Path,
) -> Result<(), RepositoryError> {
    let operation = RepositoryOperation::RefreshRepository;
    transaction.execute(
        "INSERT INTO contexts (repository_id, kind, branch, worktree_path, item_id, head_oid) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![repository_id, context_kind_name(observed.context.kind), observed.context.branch, observed.context.worktree.to_str(), observed.context.item_id.as_ref().map(ToString::to_string), observed.head.oid.map(|oid| oid.to_string())],
    ).map_err(|error| RepositoryError::sqlite(error).for_operation(operation, root))?;
    let context_id = transaction.last_insert_rowid();
    for item in observed.items {
        if observed.excluded_item_ids.contains(&item.id) {
            continue;
        }
        transaction.execute(
            "INSERT INTO discovered_items (context_id, item_id, kind, canonical_path, title, ticket_type, status, project, team, closed_at, activity_at, activity_source) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![context_id, item.id.to_string(), authoring_kind_name(item.kind), item.path.to_str(), item.title, item.ticket_type, item.status, item.project, item.team, item.closed_at.map(OffsetDateTime::unix_timestamp), item.activity_at.unix_timestamp(), activity_source_name(item.activity_source)],
        ).map_err(|error| RepositoryError::sqlite(error).for_operation(operation, root))?;
        persist_comments(
            transaction,
            transaction.last_insert_rowid(),
            &item.comments,
            None,
            root,
        )?;
    }
    let observed_at = OffsetDateTime::now_utc().unix_timestamp();
    for problem in observed.problems {
        let (path, code, guidance) = match problem {
            RootObservationProblem::Configuration(problem) => (
                Some(problem.path.as_path()),
                validation_code_name(problem.code.clone()),
                problem.message.as_str(),
            ),
            RootObservationProblem::Branch { message } => (None, "branch", message.as_str()),
            RootObservationProblem::Source { path, message } => {
                (Some(path.as_path()), "source", message.as_str())
            }
            RootObservationProblem::Context { path, message } => {
                (Some(path.as_path()), "context", message.as_str())
            }
        };
        let stored_path = path.and_then(|path| {
            path.strip_prefix(&observed.context.worktree)
                .ok()
                .unwrap_or(path)
                .to_str()
        });
        transaction.execute("INSERT INTO problems (repository_id, context_id, path, code, guidance, observed_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![repository_id, context_id, stored_path, code, guidance, observed_at])
            .map_err(|error| RepositoryError::sqlite(error).for_operation(operation, root))?;
    }
    for problem in observed.validation_problems {
        transaction.execute("INSERT INTO problems (repository_id, context_id, path, code, guidance, observed_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![repository_id, context_id, problem.path.to_str(), validation_code_name(problem.code.clone()), problem.message, observed_at])
            .map_err(|error| RepositoryError::sqlite(error).for_operation(operation, root))?;
    }
    Ok(())
}

fn persist_comments(
    transaction: &rusqlite::Transaction<'_>,
    item_id: i64,
    comments: &[ObservedCommentThread],
    parent: Option<&canonical::ItemId>,
    root: &Path,
) -> Result<(), RepositoryError> {
    for comment in comments {
        transaction.execute("INSERT INTO discovered_comments (item_id, comment_id, parent_comment_id, canonical_path, created_at) VALUES (?1, ?2, ?3, ?4, ?5)", params![item_id, comment.id.to_string(), parent.map(ToString::to_string), comment.path.to_str(), comment.created_at.unix_timestamp()])
            .map_err(|error| RepositoryError::sqlite(error).for_operation(RepositoryOperation::RefreshRepository, root))?;
        persist_comments(
            transaction,
            item_id,
            &comment.replies,
            Some(&comment.id),
            root,
        )?;
    }
    Ok(())
}

fn context_kind_name(kind: DiscoveryContextKind) -> &'static str {
    match kind {
        DiscoveryContextKind::Primary => "primary",
        DiscoveryContextKind::Unverified => "unverified",
        DiscoveryContextKind::Active => "active",
    }
}
fn authoring_kind_name(kind: AuthoringKind) -> &'static str {
    match kind {
        AuthoringKind::Document => "document",
        AuthoringKind::Ticket => "ticket",
    }
}
fn activity_source_name(source: DiscoveryActivitySource) -> &'static str {
    match source {
        DiscoveryActivitySource::GitCommit => "git",
        DiscoveryActivitySource::UncommittedFilesystem => "filesystem",
    }
}
fn validation_code_name(code: canonical::ValidationCode) -> &'static str {
    match code {
        canonical::ValidationCode::InvalidPath => "invalid-path",
        canonical::ValidationCode::MissingFrontMatter => "missing-front-matter",
        canonical::ValidationCode::MalformedFrontMatter => "malformed-front-matter",
        canonical::ValidationCode::MalformedConfiguration => "malformed-configuration",
        canonical::ValidationCode::MissingField => "missing-field",
        canonical::ValidationCode::InvalidField => "invalid-field",
        canonical::ValidationCode::KindPathMismatch => "kind-path-mismatch",
        canonical::ValidationCode::DuplicateId => "duplicate-id",
        canonical::ValidationCode::MissingCommentItem => "missing-comment-item",
        canonical::ValidationCode::MissingParent => "missing-parent",
        canonical::ValidationCode::CrossItemParent => "cross-item-parent",
        canonical::ValidationCode::CommentCycle => "comment-cycle",
    }
}

struct StoredComment {
    id: canonical::ItemId,
    parent_id: Option<canonical::ItemId>,
    path: PathBuf,
    created_at: OffsetDateTime,
}

fn read_repository_snapshot(
    connection: &rusqlite::Connection,
    root: &Path,
    root_path: &str,
) -> Result<RepositorySnapshot, RepositoryError> {
    let operation = RepositoryOperation::RepositorySnapshot;
    let registrations = connection
        .prepare(
            "SELECT id, config_blob_oid, refresh_required FROM repositories WHERE root_path = ?1",
        )
        .and_then(|mut statement| {
            statement
                .query_map([root_path], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|error| snapshot_index_error_source(root, error))?;
    let [(repository_id, config_blob_oid, refresh_required)] = registrations.as_slice() else {
        return if registrations.is_empty() {
            Err(RepositoryError::new(
                operation,
                Some(root.to_owned()),
                RepositoryErrorKind::RepositoryNotRegistered,
                "the repository is not registered",
            ))
        } else {
            Err(snapshot_index_error(
                root,
                "repository registration is not unique",
            ))
        };
    };
    if let Some(config_blob_oid) = config_blob_oid {
        git2::Oid::from_str(config_blob_oid)
            .map_err(|_| snapshot_index_error(root, "configuration blob OID is invalid"))?;
    }
    let refresh_required = match refresh_required {
        0 => false,
        1 => true,
        _ => {
            return Err(snapshot_index_error(
                root,
                "refresh-required flag is invalid",
            ));
        }
    };
    let configuration = connection
        .query_row(
            "SELECT state, primary_branch, publication_remote, invalid_code, guidance
             FROM configuration_observations WHERE repository_id = ?1",
            [*repository_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            },
        )
        .optional()
        .map_err(|error| snapshot_index_error_source(root, error))?
        .map(
            |(state, primary_branch, publication_remote, invalid_code, guidance)| {
                stored_configuration(
                    &state,
                    primary_branch,
                    publication_remote,
                    invalid_code,
                    guidance,
                    root,
                )
            },
        )
        .transpose()?
        .unwrap_or(SnapshotConfiguration::Missing);

    let contexts = connection
        .prepare(
            "SELECT id, kind, branch, worktree_path, item_id, head_oid
             FROM contexts WHERE repository_id = ?1
             ORDER BY worktree_path ASC, id ASC",
        )
        .and_then(|mut statement| {
            statement
                .query_map([*repository_id], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|error| snapshot_index_error_source(root, error))?;
    let contexts = contexts
        .into_iter()
        .map(|(id, kind, branch, worktree, item_id, head_oid)| {
            let worktree = stored_absolute_path(&worktree, root)?;
            let kind = stored_context_kind(&kind, root)?;
            let item_id = item_id
                .as_deref()
                .map(|value| stored_item_id(value, root))
                .transpose()?;
            stored_context_fields(kind, branch.as_deref(), item_id.as_ref(), &worktree, root)?;
            Ok((
                id,
                DiscoveredContext {
                    kind,
                    branch,
                    worktree,
                    item_id,
                    head_oid: head_oid
                        .as_deref()
                        .map(|value| {
                            git2::Oid::from_str(value)
                                .map_err(|_| snapshot_index_error(root, "head OID is invalid"))
                        })
                        .transpose()?,
                },
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let items = connection
        .prepare(
            "SELECT discovered_items.id, contexts.worktree_path, discovered_items.item_id,
                    discovered_items.kind, discovered_items.canonical_path, discovered_items.title,
                    discovered_items.ticket_type, discovered_items.status, discovered_items.project,
                    discovered_items.team, discovered_items.closed_at, discovered_items.activity_at,
                    discovered_items.activity_source
             FROM discovered_items JOIN contexts ON contexts.id = discovered_items.context_id
             WHERE contexts.repository_id = ?1
             ORDER BY contexts.worktree_path ASC, discovered_items.canonical_path ASC,
                      discovered_items.item_id ASC, discovered_items.id ASC",
        )
        .and_then(|mut statement| {
            statement
                .query_map([*repository_id], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, Option<String>>(7)?,
                        row.get::<_, Option<String>>(8)?,
                        row.get::<_, Option<String>>(9)?,
                        row.get::<_, Option<i64>>(10)?,
                        row.get::<_, i64>(11)?,
                        row.get::<_, String>(12)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|error| snapshot_index_error_source(root, error))?;
    let items = items
        .into_iter()
        .map(
            |(
                row_id,
                context,
                id,
                kind,
                path,
                title,
                ticket_type,
                status,
                project,
                team,
                closed_at,
                activity_at,
                activity_source,
            )| {
                let comments = read_stored_comments(connection, root, row_id)?;
                let kind = stored_authoring_kind(&kind, root)?;
                stored_item_metadata(
                    kind,
                    &title,
                    (&ticket_type, &status, &project, &team),
                    closed_at,
                    root,
                )?;
                Ok(DiscoveredItem {
                    context: stored_absolute_path(&context, root)?,
                    id: stored_item_id(&id, root)?,
                    kind,
                    path: stored_relative_path(&path, root)?,
                    title,
                    ticket_type,
                    status,
                    project,
                    team,
                    closed_at: closed_at
                        .map(|value| stored_timestamp(value, root))
                        .transpose()?,
                    activity_at: stored_timestamp(activity_at, root)?,
                    activity_source: stored_activity_source(&activity_source, root)?,
                    comments,
                })
            },
        )
        .collect::<Result<Vec<_>, _>>()?;
    let mut item_ids = BTreeSet::new();
    for (_, context) in &contexts {
        if context.kind == DiscoveryContextKind::Active {
            let (branch_kind, branch_item_id) = stored_authoring_branch(
                context
                    .branch
                    .as_deref()
                    .expect("validated active context branch"),
            )
            .expect("validated active context branch");
            if !items.iter().any(|item| {
                item.context == context.worktree
                    && item.id == branch_item_id
                    && item.kind == branch_kind
            }) {
                return Err(snapshot_index_error(
                    root,
                    "stored active context does not contain its branch-identified item",
                ));
            }
        }
    }
    for item in &items {
        if !item_ids.insert(item.id.clone()) || !insert_comment_ids(&item.comments, &mut item_ids) {
            return Err(snapshot_index_error(
                root,
                "stored item ID is not globally unique",
            ));
        }
    }

    let problems = connection
        .prepare(
            "SELECT contexts.worktree_path, problems.path, problems.code, problems.guidance, problems.observed_at
             FROM problems LEFT JOIN contexts ON contexts.id = problems.context_id
             WHERE problems.repository_id = ?1
             ORDER BY problems.observed_at ASC, contexts.worktree_path ASC, problems.path ASC,
                      problems.code ASC, problems.guidance ASC, problems.id ASC",
        )
        .and_then(|mut statement| {
            statement.query_map([*repository_id], |row| {
                Ok((row.get::<_, Option<String>>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, i64>(4)?))
            })?.collect::<Result<Vec<_>, _>>()
        })
        .map_err(|error| snapshot_index_error_source(root, error))?
        .into_iter()
        .map(|(context, path, code, guidance, observed_at)| Ok(DiscoveryProblem {
            context: context.as_deref().map(|value| stored_absolute_path(value, root)).transpose()?,
            path: path.as_deref().map(|value| stored_relative_path(value, root)).transpose()?,
            code,
            guidance,
            observed_at: stored_timestamp(observed_at, root)?,
        }))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(RepositorySnapshot {
        root: root.to_owned(),
        configuration,
        refresh_required,
        contexts: contexts.into_iter().map(|(_, context)| context).collect(),
        items,
        problems,
    })
}

fn read_stored_comments(
    connection: &rusqlite::Connection,
    root: &Path,
    item_row_id: i64,
) -> Result<Vec<DiscoveredCommentThread>, RepositoryError> {
    let comments = connection
        .prepare(
            "SELECT comment_id, parent_comment_id, canonical_path, created_at
         FROM discovered_comments WHERE item_id = ?1
         ORDER BY created_at ASC, comment_id ASC, id ASC",
        )
        .and_then(|mut statement| {
            statement
                .query_map([item_row_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|error| snapshot_index_error_source(root, error))?
        .into_iter()
        .map(|(id, parent_id, path, created_at)| {
            Ok(StoredComment {
                id: stored_item_id(&id, root)?,
                parent_id: parent_id
                    .as_deref()
                    .map(|value| stored_item_id(value, root))
                    .transpose()?,
                path: stored_relative_path(&path, root)?,
                created_at: stored_timestamp(created_at, root)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let ids = comments
        .iter()
        .map(|comment| comment.id.clone())
        .collect::<BTreeSet<_>>();
    if comments.iter().any(|comment| {
        comment
            .parent_id
            .as_ref()
            .is_some_and(|parent| !ids.contains(parent))
    }) {
        return Err(snapshot_index_error(root, "comment parent is missing"));
    }
    let mut ancestors = BTreeSet::new();
    let threads = stored_comment_children(&comments, None, &mut ancestors, root)?;
    if threads.iter().map(comment_thread_len).sum::<usize>() != comments.len() {
        return Err(snapshot_index_error(root, "comment tree is cyclic"));
    }
    Ok(threads)
}

fn stored_comment_children(
    comments: &[StoredComment],
    parent: Option<&canonical::ItemId>,
    ancestors: &mut BTreeSet<canonical::ItemId>,
    root: &Path,
) -> Result<Vec<DiscoveredCommentThread>, RepositoryError> {
    comments
        .iter()
        .filter(|comment| comment.parent_id.as_ref() == parent)
        .map(|comment| {
            if !ancestors.insert(comment.id.clone()) {
                return Err(snapshot_index_error(root, "comment tree is cyclic"));
            }
            let replies = stored_comment_children(comments, Some(&comment.id), ancestors, root)?;
            ancestors.remove(&comment.id);
            Ok(DiscoveredCommentThread {
                id: comment.id.clone(),
                path: comment.path.clone(),
                created_at: comment.created_at,
                replies,
            })
        })
        .collect()
}

fn comment_thread_len(thread: &DiscoveredCommentThread) -> usize {
    1 + thread.replies.iter().map(comment_thread_len).sum::<usize>()
}
fn insert_comment_ids(
    comments: &[DiscoveredCommentThread],
    ids: &mut BTreeSet<canonical::ItemId>,
) -> bool {
    comments
        .iter()
        .all(|comment| ids.insert(comment.id.clone()) && insert_comment_ids(&comment.replies, ids))
}

fn stored_item_id(value: &str, root: &Path) -> Result<canonical::ItemId, RepositoryError> {
    value
        .parse()
        .map_err(|_| snapshot_index_error(root, "stored item ID is invalid"))
}
fn stored_configuration(
    state: &str,
    primary_branch: Option<String>,
    publication_remote: Option<String>,
    invalid_code: Option<String>,
    guidance: Option<String>,
    root: &Path,
) -> Result<SnapshotConfiguration, RepositoryError> {
    match state {
        "missing"
            if primary_branch.is_none()
                && publication_remote.is_none()
                && invalid_code.is_none()
                && guidance.is_none() =>
        {
            Ok(SnapshotConfiguration::Missing)
        }
        "valid" if invalid_code.is_none() && guidance.is_none() => primary_branch
            .filter(|branch| !branch.is_empty())
            .map(|primary_branch| {
                let mut source = format!(
                    "format_version = 1\nprimary_branch = {}\n",
                    toml::Value::String(primary_branch)
                );
                if let Some(remote) = publication_remote {
                    source.push_str(&format!(
                        "publication_remote = {}\n",
                        toml::Value::String(remote)
                    ));
                }
                let configuration = canonical::parse_repository_config(&source)
                    .map_err(|_| snapshot_index_error(root, "valid configuration is invalid"))?;
                Ok(SnapshotConfiguration::Valid {
                    primary_branch: configuration.primary_branch,
                    publication_remote: configuration.publication_remote,
                })
            })
            .unwrap_or_else(|| {
                Err(snapshot_index_error(
                    root,
                    "valid configuration is incomplete",
                ))
            }),
        "invalid" if primary_branch.is_none() && publication_remote.is_none() => {
            let code = invalid_code
                .as_deref()
                .and_then(stored_validation_code)
                .ok_or_else(|| {
                    snapshot_index_error(root, "invalid configuration code is invalid")
                })?;
            let guidance = guidance
                .filter(|guidance| !guidance.is_empty())
                .ok_or_else(|| {
                    snapshot_index_error(root, "invalid configuration guidance is missing")
                })?;
            Ok(SnapshotConfiguration::Invalid { code, guidance })
        }
        _ => Err(snapshot_index_error(
            root,
            "configuration observation is invalid",
        )),
    }
}
fn stored_validation_code(value: &str) -> Option<canonical::ValidationCode> {
    match value {
        "invalid-path" => Some(canonical::ValidationCode::InvalidPath),
        "missing-front-matter" => Some(canonical::ValidationCode::MissingFrontMatter),
        "malformed-front-matter" => Some(canonical::ValidationCode::MalformedFrontMatter),
        "malformed-configuration" => Some(canonical::ValidationCode::MalformedConfiguration),
        "missing-field" => Some(canonical::ValidationCode::MissingField),
        "invalid-field" => Some(canonical::ValidationCode::InvalidField),
        "kind-path-mismatch" => Some(canonical::ValidationCode::KindPathMismatch),
        "duplicate-id" => Some(canonical::ValidationCode::DuplicateId),
        "missing-comment-item" => Some(canonical::ValidationCode::MissingCommentItem),
        "missing-parent" => Some(canonical::ValidationCode::MissingParent),
        "cross-item-parent" => Some(canonical::ValidationCode::CrossItemParent),
        "comment-cycle" => Some(canonical::ValidationCode::CommentCycle),
        _ => None,
    }
}
fn stored_timestamp(value: i64, root: &Path) -> Result<OffsetDateTime, RepositoryError> {
    OffsetDateTime::from_unix_timestamp(value)
        .map_err(|_| snapshot_index_error(root, "stored timestamp is invalid"))
}
fn stored_absolute_path(value: &str, root: &Path) -> Result<PathBuf, RepositoryError> {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        Ok(path)
    } else {
        Err(snapshot_index_error(
            root,
            "stored absolute path is invalid",
        ))
    }
}
fn stored_relative_path(value: &str, root: &Path) -> Result<PathBuf, RepositoryError> {
    let path = PathBuf::from(value);
    if !path.as_os_str().is_empty()
        && !path.is_absolute()
        && !value.contains('\\')
        && !value
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
        && !path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        Ok(path)
    } else {
        Err(snapshot_index_error(
            root,
            "stored relative path is invalid",
        ))
    }
}
fn stored_item_metadata(
    kind: AuthoringKind,
    title: &str,
    (ticket_type, status, project, team): (
        &Option<String>,
        &Option<String>,
        &Option<String>,
        &Option<String>,
    ),
    closed_at: Option<i64>,
    root: &Path,
) -> Result<(), RepositoryError> {
    stored_text(title, root)?;
    match kind {
        AuthoringKind::Document
            if ticket_type.is_none()
                && status.is_none()
                && project.is_none()
                && team.is_none()
                && closed_at.is_none() =>
        {
            Ok(())
        }
        AuthoringKind::Ticket => {
            for value in [ticket_type, status, project, team].into_iter().flatten() {
                stored_text(value, root)?;
            }
            if ticket_type.is_none() || status.is_none() {
                return Err(snapshot_index_error(root, "ticket metadata is incomplete"));
            }
            Ok(())
        }
        AuthoringKind::Document => Err(snapshot_index_error(root, "document has ticket metadata")),
    }
}
fn stored_text(value: &str, root: &Path) -> Result<(), RepositoryError> {
    if value.is_empty() || value.contains('\0') {
        Err(snapshot_index_error(root, "stored text is invalid"))
    } else {
        Ok(())
    }
}
fn stored_context_kind(value: &str, root: &Path) -> Result<DiscoveryContextKind, RepositoryError> {
    match value {
        "primary" => Ok(DiscoveryContextKind::Primary),
        "unverified" => Ok(DiscoveryContextKind::Unverified),
        "active" => Ok(DiscoveryContextKind::Active),
        _ => Err(snapshot_index_error(root, "stored context kind is invalid")),
    }
}
fn stored_context_fields(
    kind: DiscoveryContextKind,
    branch: Option<&str>,
    item_id: Option<&canonical::ItemId>,
    worktree: &Path,
    root: &Path,
) -> Result<(), RepositoryError> {
    if let Some(branch) = branch {
        let source = format!(
            "format_version = 1\nprimary_branch = {}\n",
            toml::Value::String(branch.to_owned())
        );
        canonical::parse_repository_config(&source)
            .map_err(|_| snapshot_index_error(root, "stored context branch is invalid"))?;
    }
    match kind {
        DiscoveryContextKind::Primary
            if branch.is_some() && item_id.is_none() && worktree == root =>
        {
            Ok(())
        }
        DiscoveryContextKind::Unverified if item_id.is_none() && worktree == root => Ok(()),
        DiscoveryContextKind::Active if branch.is_some() && item_id.is_some() => {
            let (branch, item_id) = match (branch, item_id) {
                (Some(branch), Some(item_id)) => (branch, item_id),
                _ => unreachable!(),
            };
            let expected_worktree = root.join(".manyhands/worktrees").join(item_id.to_string());
            let Some((_, branch_id)) = stored_authoring_branch(branch) else {
                return Err(snapshot_index_error(
                    root,
                    "stored active context branch is invalid",
                ));
            };
            if branch_id == *item_id && worktree == expected_worktree {
                Ok(())
            } else {
                Err(snapshot_index_error(
                    root,
                    "stored active context path or branch does not match its item",
                ))
            }
        }
        _ => Err(snapshot_index_error(
            root,
            "stored context fields are inconsistent",
        )),
    }
}

fn stored_authoring_branch(branch: &str) -> Option<(AuthoringKind, canonical::ItemId)> {
    let mut segments = branch.strip_prefix("manyhands/")?.split('/');
    let kind = match segments.next()? {
        "document" => AuthoringKind::Document,
        "ticket" => AuthoringKind::Ticket,
        _ => return None,
    };
    let item_id = segments.next()?.parse().ok()?;
    (segments.next().is_none()).then_some((kind, item_id))
}
fn stored_authoring_kind(value: &str, root: &Path) -> Result<AuthoringKind, RepositoryError> {
    match value {
        "document" => Ok(AuthoringKind::Document),
        "ticket" => Ok(AuthoringKind::Ticket),
        _ => Err(snapshot_index_error(root, "stored item kind is invalid")),
    }
}
fn stored_activity_source(
    value: &str,
    root: &Path,
) -> Result<DiscoveryActivitySource, RepositoryError> {
    match value {
        "git" => Ok(DiscoveryActivitySource::GitCommit),
        "filesystem" => Ok(DiscoveryActivitySource::UncommittedFilesystem),
        _ => Err(snapshot_index_error(
            root,
            "stored activity source is invalid",
        )),
    }
}
fn snapshot_index_error(root: &Path, message: impl fmt::Display) -> RepositoryError {
    RepositoryError::new(
        RepositoryOperation::RepositorySnapshot,
        Some(root.to_owned()),
        RepositoryErrorKind::IndexUnavailable,
        message,
    )
}
fn snapshot_index_error_source(
    root: &Path,
    error: impl std::error::Error + Send + Sync + 'static,
) -> RepositoryError {
    RepositoryError::with_source(
        RepositoryOperation::RepositorySnapshot,
        Some(root.to_owned()),
        RepositoryErrorKind::IndexUnavailable,
        error,
    )
}

fn default_data_directory() -> Result<PathBuf, RepositoryError> {
    resolve_default_data_directory(
        directories::ProjectDirs::from("com", "manyhands", "Manyhands")
            .map(|directories| directories.data_local_dir().to_owned()),
    )
}

fn authoring_kind_segment(kind: &AuthoringKind) -> &'static str {
    match kind {
        AuthoringKind::Document => "document",
        AuthoringKind::Ticket => "ticket",
    }
}

fn authoring_error(
    operation: RepositoryOperation,
    root: &Path,
    kind: RepositoryErrorKind,
    message: impl fmt::Display,
) -> RepositoryError {
    RepositoryError::new(operation, Some(root.to_owned()), kind, message)
}

fn owned_document_path(
    path: &Path,
    operation: RepositoryOperation,
    context: &ItemContext,
) -> Result<PathBuf, RepositoryError> {
    canonical::parse_item(path, &canonical_document_probe(&context.item_id)).map_err(
        |problem| {
            authoring_error(
                operation,
                &context.root,
                RepositoryErrorKind::InvalidPath,
                problem.message,
            )
        },
    )?;
    if !path.starts_with("docs") {
        return Err(authoring_error(
            operation,
            &context.root,
            RepositoryErrorKind::InvalidPath,
            "document paths must be below docs",
        ));
    }
    Ok(path.to_owned())
}

fn canonical_document_probe(item_id: &canonical::ItemId) -> String {
    canonical::serialize_item(&canonical::CanonicalItem::Document(canonical::Document {
        id: item_id.clone(),
        title: "probe".to_owned(),
        body: String::new(),
        unknown: serde_yaml::Mapping::new(),
    }))
    .expect("fixed probe document is valid")
}

fn owned_file_exists(
    root: &Path,
    relative: &Path,
    operation: RepositoryOperation,
    repository_root: &Path,
) -> Result<bool, RepositoryError> {
    match std::fs::symlink_metadata(root.join(relative)) {
        Ok(metadata) if metadata.file_type().is_file() && !metadata.file_type().is_symlink() => {
            Ok(true)
        }
        Ok(_) => Err(authoring_error(
            operation,
            repository_root,
            RepositoryErrorKind::InvalidPath,
            "an owned path must be a regular non-symlink file",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(RepositoryError::io(
            operation,
            Some(repository_root.to_owned()),
            error,
        )),
    }
}

fn source_exists(
    root: &Path,
    relative: &Path,
    operation: RepositoryOperation,
    repository_root: &Path,
) -> Result<bool, RepositoryError> {
    owned_file_exists(root, relative, operation, repository_root)
}

fn read_owned_item(
    root: &Path,
    relative: &Path,
    operation: RepositoryOperation,
    repository_root: &Path,
) -> Result<canonical::CanonicalItem, RepositoryError> {
    if !owned_file_exists(root, relative, operation, repository_root)? {
        return Err(authoring_error(
            operation,
            repository_root,
            RepositoryErrorKind::MissingAuthoringTarget,
            "the owned file is missing",
        ));
    }
    let source = std::fs::read_to_string(root.join(relative))
        .map_err(|error| RepositoryError::io(operation, Some(repository_root.to_owned()), error))?;
    canonical::parse_item(relative, &source).map_err(|problem| {
        authoring_error(
            operation,
            repository_root,
            RepositoryErrorKind::MissingAuthoringTarget,
            problem.message,
        )
    })
}

fn write_owned_document(
    root: &Path,
    relative: &Path,
    bytes: &[u8],
    operation: RepositoryOperation,
    repository_root: &Path,
) -> Result<(), RepositoryError> {
    ensure_safe_owned_parent(root, relative, operation, repository_root)?;
    let path = root.join(relative);
    let _ = owned_file_exists(root, relative, operation, repository_root)?;
    replace_bytes_atomically(&path, bytes, repository_root)
        .map_err(|error| error.for_operation(operation, repository_root))
}

fn ensure_safe_owned_parent(
    root: &Path,
    relative: &Path,
    operation: RepositoryOperation,
    repository_root: &Path,
) -> Result<(), RepositoryError> {
    let parent = relative.parent().ok_or_else(|| {
        authoring_error(
            operation,
            repository_root,
            RepositoryErrorKind::InvalidPath,
            "an owned path needs a parent directory",
        )
    })?;
    let mut current = root.to_owned();
    for component in parent.components() {
        let std::path::Component::Normal(component) = component else {
            return Err(authoring_error(
                operation,
                repository_root,
                RepositoryErrorKind::InvalidPath,
                "owned paths must be relative",
            ));
        };
        current.push(component);
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => {
                return Err(authoring_error(
                    operation,
                    repository_root,
                    RepositoryErrorKind::InvalidPath,
                    "an owned parent must be a real directory",
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir(&current).map_err(|error| {
                    RepositoryError::io(operation, Some(repository_root.to_owned()), error)
                })?
            }
            Err(error) => {
                return Err(RepositoryError::io(
                    operation,
                    Some(repository_root.to_owned()),
                    error,
                ));
            }
        }
    }
    Ok(())
}

fn validate_safe_owned_parent(
    root: &Path,
    relative: &Path,
    operation: RepositoryOperation,
    repository_root: &Path,
) -> Result<(), RepositoryError> {
    let parent = relative.parent().ok_or_else(|| {
        authoring_error(
            operation,
            repository_root,
            RepositoryErrorKind::InvalidPath,
            "an owned path needs a parent directory",
        )
    })?;
    let mut current = root.to_owned();
    for component in parent.components() {
        let std::path::Component::Normal(component) = component else {
            return Err(authoring_error(
                operation,
                repository_root,
                RepositoryErrorKind::InvalidPath,
                "owned paths must be relative",
            ));
        };
        current.push(component);
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => {
                return Err(authoring_error(
                    operation,
                    repository_root,
                    RepositoryErrorKind::InvalidPath,
                    "an owned parent must be a real directory",
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => {
                return Err(RepositoryError::io(
                    operation,
                    Some(repository_root.to_owned()),
                    error,
                ));
            }
        }
    }
    Ok(())
}

fn remove_owned_file(
    root: &Path,
    relative: &Path,
    operation: RepositoryOperation,
    repository_root: &Path,
) -> Result<(), RepositoryError> {
    if !owned_file_exists(root, relative, operation, repository_root)? {
        return Ok(());
    }
    std::fs::remove_file(root.join(relative))
        .map_err(|error| RepositoryError::io(operation, Some(repository_root.to_owned()), error))
}

fn add_owned_blob(
    index: &mut Index,
    repository: &Repository,
    root: &Path,
    relative: &Path,
    operation: RepositoryOperation,
    repository_root: &Path,
) -> Result<(), RepositoryError> {
    if !owned_file_exists(root, relative, operation, repository_root)? {
        return Err(authoring_error(
            operation,
            repository_root,
            RepositoryErrorKind::MissingAuthoringTarget,
            "the checkpoint destination is missing",
        ));
    }
    let bytes = std::fs::read(root.join(relative))
        .map_err(|error| RepositoryError::io(operation, Some(repository_root.to_owned()), error))?;
    let blob = repository.blob(&bytes).map_err(|error| {
        RepositoryError::git(operation, Some(repository_root.to_owned()), error)
    })?;
    index
        .add(&IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            file_size: bytes.len() as u32,
            id: blob,
            flags: 0,
            flags_extended: 0,
            path: relative.as_os_str().as_encoded_bytes().to_vec(),
        })
        .map_err(|error| RepositoryError::git(operation, Some(repository_root.to_owned()), error))
}

fn validate_head_document_source(
    repository: &Repository,
    source_path: &Path,
    context: &ItemContext,
    operation: RepositoryOperation,
) -> Result<bool, RepositoryError> {
    let tree = repository
        .head()
        .and_then(|head| head.peel_to_tree())
        .map_err(|error| RepositoryError::git(operation, Some(context.root.clone()), error))?;
    let entry = match tree.get_path(source_path) {
        Ok(entry) => entry,
        Err(error) if error.code() == git2::ErrorCode::NotFound => return Ok(false),
        Err(error) => {
            return Err(RepositoryError::git(
                operation,
                Some(context.root.clone()),
                error,
            ));
        }
    };
    let blob = repository
        .find_blob(entry.id())
        .map_err(|error| RepositoryError::git(operation, Some(context.root.clone()), error))?;
    let source = std::str::from_utf8(blob.content()).map_err(|_| {
        authoring_error(
            operation,
            &context.root,
            RepositoryErrorKind::MissingAuthoringTarget,
            "the move source is not canonical text",
        )
    })?;
    match canonical::parse_item(source_path, source) {
        Ok(canonical::CanonicalItem::Document(document)) if document.id == context.item_id => {
            Ok(true)
        }
        _ => Err(authoring_error(
            operation,
            &context.root,
            RepositoryErrorKind::MissingAuthoringTarget,
            "the move source in context HEAD is not the selected document",
        )),
    }
}

fn authoring_item_matches(item: &canonical::CanonicalItem, context: &ItemContext) -> bool {
    match (item, &context.kind) {
        (canonical::CanonicalItem::Document(document), AuthoringKind::Document) => {
            document.id == context.item_id
        }
        (canonical::CanonicalItem::Ticket(ticket), AuthoringKind::Ticket) => {
            ticket.id == context.item_id
        }
        _ => false,
    }
}

fn canonical_item_has_id(item: &canonical::CanonicalItem, item_id: &canonical::ItemId) -> bool {
    match item {
        canonical::CanonicalItem::Document(document) => &document.id == item_id,
        canonical::CanonicalItem::Ticket(ticket) => &ticket.id == item_id,
        canonical::CanonicalItem::Comment(comment) => &comment.id == item_id,
    }
}

struct CanonicalContext {
    items: Vec<canonical::CanonicalItem>,
    candidates: Vec<canonical::CanonicalItem>,
}

fn canonical_context_at(
    root: &Path,
    operation: RepositoryOperation,
) -> Result<CanonicalContext, RepositoryError> {
    let mut sources = Vec::new();
    collect_canonical_sources(root, &mut sources, operation)?;
    let candidates = sources
        .iter()
        .filter_map(|(path, source)| canonical::parse_item(path, source).ok())
        .collect();
    let items = canonical::validate_context(sources).items;
    Ok(CanonicalContext { items, candidates })
}

fn document_path_for_id(
    root: &Path,
    item_id: &canonical::ItemId,
    operation: RepositoryOperation,
) -> Result<Option<PathBuf>, RepositoryError> {
    let mut sources = Vec::new();
    collect_canonical_sources(root, &mut sources, operation)?;
    Ok(sources.into_iter().find_map(|(path, source)| {
        matches!(canonical::parse_item(&path, &source), Ok(canonical::CanonicalItem::Document(document)) if document.id == *item_id)
            .then_some(path)
    }))
}

fn ticket_id_exists_at_a_different_path(
    root: &Path,
    item_id: &canonical::ItemId,
    expected: &Path,
    operation: RepositoryOperation,
    repository_root: &Path,
) -> Result<bool, RepositoryError> {
    let tickets = root.join(".manyhands/tickets");
    for entry in read_directory_if_exists(&tickets, repository_root, operation)? {
        let file_type = entry.file_type().map_err(|error| {
            RepositoryError::io(operation, Some(repository_root.to_owned()), error)
        })?;
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }
        let path = entry.path().join("ticket.md");
        let relative = path.strip_prefix(root).map_err(|_| {
            authoring_error(
                operation,
                repository_root,
                RepositoryErrorKind::InvalidPath,
                "ticket path must remain inside the selected context",
            )
        })?;
        if relative == expected {
            continue;
        }
        match std::fs::symlink_metadata(&path) {
            Ok(metadata)
                if metadata.file_type().is_file() && !metadata.file_type().is_symlink() => {}
            Ok(_) => continue,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(RepositoryError::io(
                    operation,
                    Some(repository_root.to_owned()),
                    error,
                ));
            }
        };
        let source = std::fs::read_to_string(&path).map_err(|error| {
            RepositoryError::io(operation, Some(repository_root.to_owned()), error)
        })?;
        if matches!(canonical::parse_item(expected, &source), Ok(canonical::CanonicalItem::Ticket(ticket)) if ticket.id == *item_id)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn canonical_context_has_item_id(context: &CanonicalContext, item_id: &canonical::ItemId) -> bool {
    context
        .candidates
        .iter()
        .any(|item| canonical_item_has_id(item, item_id))
}

fn canonical_context_has_duplicate_item_id(
    context: &CanonicalContext,
    item_id: &canonical::ItemId,
) -> bool {
    context
        .candidates
        .iter()
        .filter(|item| canonical_item_has_id(item, item_id))
        .nth(1)
        .is_some()
}

fn canonical_comment_paths_for_id(
    root: &Path,
    item_id: &canonical::ItemId,
    operation: RepositoryOperation,
) -> Result<Vec<PathBuf>, RepositoryError> {
    let mut sources = Vec::new();
    collect_canonical_sources(root, &mut sources, operation)?;
    Ok(sources
        .into_iter()
        .filter_map(|(path, source)| {
            matches!(canonical::parse_item(&path, &source), Ok(canonical::CanonicalItem::Comment(comment)) if comment.id == *item_id)
                .then_some(path)
        })
        .collect())
}

fn comment_publication_state(
    configuration: ConfigurationInspection,
    operation: RepositoryOperation,
    root: &Path,
) -> Result<CommentPublicationState, RepositoryError> {
    match configuration {
        ConfigurationInspection::Valid(configuration)
            if configuration.publication_remote.is_some() =>
        {
            Ok(CommentPublicationState::SyncDeferred)
        }
        ConfigurationInspection::Valid(_) => Ok(CommentPublicationState::PublishPending),
        ConfigurationInspection::Missing | ConfigurationInspection::Invalid(_) => {
            Err(authoring_error(
                operation,
                root,
                RepositoryErrorKind::InvalidConfiguration,
                "the repository configuration is not valid",
            ))
        }
    }
}

fn ensure_authoring_worktree_base(
    root: &Path,
    operation: RepositoryOperation,
) -> Result<(), RepositoryError> {
    let parent = root.join(".manyhands");
    let parent_metadata = std::fs::symlink_metadata(&parent)
        .map_err(|error| RepositoryError::io(operation, Some(root.to_owned()), error))?;
    if parent_metadata.file_type().is_symlink() || !parent_metadata.is_dir() {
        return Err(RepositoryError::new(
            operation,
            Some(root.to_owned()),
            RepositoryErrorKind::MismatchedAuthoringContext,
            "the Manyhands directory must be a real directory",
        ));
    }
    let base = parent.join("worktrees");
    match std::fs::symlink_metadata(&base) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(RepositoryError::new(
                operation,
                Some(root.to_owned()),
                RepositoryErrorKind::MismatchedAuthoringContext,
                "the Manyhands worktree base must be a real directory",
            ));
        }
        Ok(_) => return Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(RepositoryError::io(operation, Some(root.to_owned()), error)),
    }
    match std::fs::create_dir(&base) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(RepositoryError::io(operation, Some(root.to_owned()), error)),
    }
    let metadata = std::fs::symlink_metadata(&base)
        .map_err(|error| RepositoryError::io(operation, Some(root.to_owned()), error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(RepositoryError::new(
            operation,
            Some(root.to_owned()),
            RepositoryErrorKind::MismatchedAuthoringContext,
            "the Manyhands worktree base must be a real directory",
        ));
    }
    Ok(())
}

fn collect_canonical_sources(
    root: &Path,
    sources: &mut Vec<(PathBuf, String)>,
    operation: RepositoryOperation,
) -> Result<(), RepositoryError> {
    collect_document_sources(root, &root.join("docs"), sources, operation)?;
    collect_ticket_sources(root, &root.join(".manyhands/tickets"), sources, operation)?;
    collect_comment_sources(root, &root.join(".manyhands/comments"), sources, operation)?;
    Ok(())
}

fn collect_ticket_sources(
    root: &Path,
    directory: &Path,
    sources: &mut Vec<(PathBuf, String)>,
    operation: RepositoryOperation,
) -> Result<(), RepositoryError> {
    collect_managed_sources(root, directory, sources, operation, |path, depth, _| {
        depth == 1 && path.file_name().is_some_and(|name| name == "ticket.md")
    })
}

fn collect_comment_sources(
    root: &Path,
    directory: &Path,
    sources: &mut Vec<(PathBuf, String)>,
    operation: RepositoryOperation,
) -> Result<(), RepositoryError> {
    collect_managed_sources(
        root,
        directory,
        sources,
        operation,
        |path, depth, is_file| {
            is_file && depth == 1 && path.extension().is_some_and(|extension| extension == "md")
        },
    )
}

fn collect_managed_sources(
    root: &Path,
    directory: &Path,
    sources: &mut Vec<(PathBuf, String)>,
    operation: RepositoryOperation,
    is_canonical_source: impl Fn(&Path, usize, bool) -> bool,
) -> Result<(), RepositoryError> {
    match std::fs::symlink_metadata(directory) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => return Ok(()),
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(RepositoryError::io(operation, Some(root.to_owned()), error)),
    }
    let mut directories = vec![(directory.to_owned(), 0)];
    let mut entries = 0;
    while let Some((directory, depth)) = directories.pop() {
        for entry in read_directory_if_exists(&directory, root, operation)? {
            entries += 1;
            if entries > MAX_MANAGED_DIRECTORY_ENTRIES {
                return Err(managed_traversal_limit_error(root, operation));
            }
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|error| RepositoryError::io(operation, Some(root.to_owned()), error))?;
            if file_type.is_dir() {
                if is_canonical_source(&path, depth, false) {
                    collect_canonical_source(root, &path, sources, operation)?;
                    continue;
                }
                if depth >= MAX_MANAGED_DIRECTORY_DEPTH {
                    return Err(managed_traversal_limit_error(root, operation));
                }
                directories.push((path, depth + 1));
            } else if file_type.is_file() && is_canonical_source(&path, depth, true) {
                collect_canonical_source(root, &path, sources, operation)?;
            }
        }
    }
    Ok(())
}

fn managed_traversal_limit_error(root: &Path, operation: RepositoryOperation) -> RepositoryError {
    RepositoryError::new(
        operation,
        Some(root.to_owned()),
        RepositoryErrorKind::InvalidPath,
        "the managed item directories exceed the bounded canonical source traversal limit",
    )
}

fn collect_document_sources(
    root: &Path,
    directory: &Path,
    sources: &mut Vec<(PathBuf, String)>,
    operation: RepositoryOperation,
) -> Result<(), RepositoryError> {
    match std::fs::symlink_metadata(directory) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => return Ok(()),
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(RepositoryError::io(operation, Some(root.to_owned()), error)),
    }
    let mut directories = vec![(directory.to_owned(), 0)];
    let mut entries = 0;
    while let Some((directory, depth)) = directories.pop() {
        for entry in read_directory_if_exists(&directory, root, operation)? {
            entries += 1;
            if entries > MAX_DOCUMENT_DIRECTORY_ENTRIES {
                return Err(document_traversal_limit_error(root, operation));
            }
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|error| RepositoryError::io(operation, Some(root.to_owned()), error))?;
            if file_type.is_dir() {
                if depth >= MAX_DOCUMENT_DIRECTORY_DEPTH {
                    return Err(document_traversal_limit_error(root, operation));
                }
                directories.push((path, depth + 1));
            } else if file_type.is_file()
                && path.extension().is_some_and(|extension| extension == "md")
            {
                collect_canonical_source(root, &path, sources, operation)?;
            }
        }
    }
    Ok(())
}

fn document_traversal_limit_error(root: &Path, operation: RepositoryOperation) -> RepositoryError {
    RepositoryError::new(
        operation,
        Some(root.to_owned()),
        RepositoryErrorKind::InvalidPath,
        "the docs directory exceeds the bounded canonical source traversal limit",
    )
}

fn read_directory_if_exists(
    path: &Path,
    root: &Path,
    operation: RepositoryOperation,
) -> Result<Vec<std::fs::DirEntry>, RepositoryError> {
    match std::fs::read_dir(path) {
        Ok(entries) => entries
            .map(|entry| {
                entry.map_err(|error| RepositoryError::io(operation, Some(root.to_owned()), error))
            })
            .collect(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(RepositoryError::io(operation, Some(root.to_owned()), error)),
    }
}

fn collect_canonical_source(
    root: &Path,
    path: &Path,
    sources: &mut Vec<(PathBuf, String)>,
    operation: RepositoryOperation,
) -> Result<(), RepositoryError> {
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::InvalidData => return Ok(()),
        Err(error) => return Err(RepositoryError::io(operation, Some(root.to_owned()), error)),
    };
    let relative = path
        .strip_prefix(root)
        .expect("source is below its root")
        .to_owned();
    sources.push((relative, source));
    Ok(())
}

fn validate_context_worktree(
    repository: &Repository,
    context: &ItemContext,
    require_target: bool,
    operation: RepositoryOperation,
) -> Result<(), RepositoryError> {
    validate_existing_authoring_worktree_path(context, operation)?;
    let worktree = repository
        .find_worktree(&context.item_id.to_string())
        .map_err(|error| {
            RepositoryError::new(
                operation,
                Some(context.root.clone()),
                RepositoryErrorKind::MismatchedAuthoringContext,
                error,
            )
        })?;
    let actual = std::fs::canonicalize(worktree.path())
        .map_err(|error| RepositoryError::io(operation, Some(context.root.clone()), error))?;
    let expected = std::fs::canonicalize(&context.worktree)
        .map_err(|error| RepositoryError::io(operation, Some(context.root.clone()), error))?;
    if actual != expected {
        return Err(RepositoryError::new(
            operation,
            Some(context.root.clone()),
            RepositoryErrorKind::MismatchedAuthoringContext,
            "the deterministic worktree name is registered at a different path",
        ));
    }
    let linked = Repository::open(&context.worktree)
        .map_err(|error| RepositoryError::git(operation, Some(context.root.clone()), error))?;
    if checked_out_branch(&linked, &context.worktree, operation)? != context.branch {
        return Err(RepositoryError::new(
            operation,
            Some(context.root.clone()),
            RepositoryErrorKind::MismatchedAuthoringContext,
            "the deterministic worktree has a different checked-out branch",
        ));
    }
    let worktree_context = canonical_context_at(&context.worktree, operation)?;
    let has_target = worktree_context
        .items
        .iter()
        .any(|item| authoring_item_matches(item, context));
    let target_candidates = worktree_context
        .candidates
        .iter()
        .filter(|item| canonical_item_has_id(item, &context.item_id))
        .collect::<Vec<_>>();
    let only_matching_target_kind = !target_candidates.is_empty()
        && target_candidates
            .iter()
            .all(|item| authoring_item_matches(item, context));
    if (require_target && (!has_target && !only_matching_target_kind))
        || (!require_target
            && (canonical_context_has_duplicate_item_id(&worktree_context, &context.item_id)
                || worktree_context.candidates.iter().any(|item| {
                    canonical_item_has_id(item, &context.item_id)
                        && !authoring_item_matches(item, context)
                })))
    {
        return Err(RepositoryError::new(
            operation,
            Some(context.root.clone()),
            RepositoryErrorKind::MismatchedAuthoringContext,
            "the deterministic worktree does not contain the requested canonical item",
        ));
    }
    Ok(())
}

fn validate_existing_authoring_worktree_path(
    context: &ItemContext,
    operation: RepositoryOperation,
) -> Result<(), RepositoryError> {
    for path in [
        context.root.as_path(),
        &context.root.join(".manyhands"),
        &context.root.join(".manyhands/worktrees"),
        context.worktree.as_path(),
    ] {
        let metadata = std::fs::symlink_metadata(path).map_err(|error| {
            RepositoryError::new(
                operation,
                Some(context.root.clone()),
                RepositoryErrorKind::MismatchedAuthoringContext,
                error,
            )
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(RepositoryError::new(
                operation,
                Some(context.root.clone()),
                RepositoryErrorKind::MismatchedAuthoringContext,
                "the deterministic worktree path must contain only real directories",
            ));
        }
    }
    Ok(())
}

fn validate_deterministic_worktree_path_chain(
    worktree: &Path,
    root: &Path,
    operation: RepositoryOperation,
) -> Result<(), RepositoryError> {
    for path in [
        root,
        &root.join(".manyhands"),
        &root.join(".manyhands/worktrees"),
        worktree,
    ] {
        match std::fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(RepositoryError::new(
                    operation,
                    Some(root.to_owned()),
                    RepositoryErrorKind::MismatchedAuthoringContext,
                    "the deterministic worktree path must contain only real directories",
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(RepositoryError::io(operation, Some(root.to_owned()), error)),
        }
    }
    Ok(())
}

fn branch_checked_out_in_another_worktree(
    repository: &Repository,
    expected_path: &Path,
    branch: &str,
    operation: RepositoryOperation,
    root: &Path,
) -> Result<bool, RepositoryError> {
    let expected = expected_path.canonicalize().ok();
    let names = repository
        .worktrees()
        .map_err(|error| RepositoryError::git(operation, Some(root.to_owned()), error))?;
    for name in names.iter_bytes() {
        let name = std::str::from_utf8(name).map_err(|error| {
            RepositoryError::new(
                operation,
                Some(root.to_owned()),
                RepositoryErrorKind::MismatchedAuthoringContext,
                error,
            )
        })?;
        let worktree = repository
            .find_worktree(name)
            .map_err(|error| RepositoryError::git(operation, Some(root.to_owned()), error))?;
        let path = worktree.path();
        if expected.as_deref() == Some(path) {
            continue;
        }
        let linked = Repository::open(path).map_err(|error| {
            RepositoryError::new(
                operation,
                Some(root.to_owned()),
                RepositoryErrorKind::MismatchedAuthoringContext,
                error,
            )
        })?;
        let checked_out = checked_out_branch(&linked, path, operation).map_err(|error| {
            RepositoryError::new(
                operation,
                Some(root.to_owned()),
                RepositoryErrorKind::MismatchedAuthoringContext,
                error,
            )
        })?;
        if checked_out == branch {
            return Ok(true);
        }
    }
    Ok(false)
}

fn resolve_default_data_directory(directory: Option<PathBuf>) -> Result<PathBuf, RepositoryError> {
    directory.ok_or_else(|| {
        RepositoryError::new(
            RepositoryOperation::OpenRegistry,
            None,
            RepositoryErrorKind::Io,
            "the platform did not provide an application data directory",
        )
    })
}

trait IdentityConfigProvider {
    fn effective_config(
        &mut self,
        repository: &Repository,
        root: &Path,
        operation: RepositoryOperation,
    ) -> Result<Config, RepositoryError>;
}

struct RepositoryIdentityConfig;

impl IdentityConfigProvider for RepositoryIdentityConfig {
    fn effective_config(
        &mut self,
        repository: &Repository,
        root: &Path,
        operation: RepositoryOperation,
    ) -> Result<Config, RepositoryError> {
        repository
            .config()
            .map_err(|error| RepositoryError::git(operation, Some(root.to_owned()), error))
    }
}

struct SuppliedIdentityConfig<'a> {
    effective_config: &'a mut Config,
}

impl IdentityConfigProvider for SuppliedIdentityConfig<'_> {
    fn effective_config(
        &mut self,
        _repository: &Repository,
        root: &Path,
        operation: RepositoryOperation,
    ) -> Result<Config, RepositoryError> {
        self.effective_config
            .snapshot()
            .map_err(|error| RepositoryError::git(operation, Some(root.to_owned()), error))
    }
}

fn canonical_repository_root(
    selected: &Path,
    operation: RepositoryOperation,
) -> Result<(Repository, PathBuf), RepositoryError> {
    let selected = std::fs::canonicalize(selected)
        .map_err(|error| canonicalization_error(operation, error))?;
    if !selected.is_dir() {
        return Err(RepositoryError::new(
            operation,
            None,
            RepositoryErrorKind::InvalidPath,
            "the selected path is not a directory",
        ));
    }
    let repository = Repository::discover(&selected).map_err(|error| {
        let kind = match error.code() {
            git2::ErrorCode::NotFound => RepositoryErrorKind::NotRepository,
            _ => RepositoryErrorKind::InaccessibleRepository,
        };
        RepositoryError::with_source(operation, None, kind, error)
    })?;

    if repository.is_bare() {
        return Err(RepositoryError::new(
            operation,
            Some(selected),
            RepositoryErrorKind::BareRepository,
            "bare repositories have no working directory",
        ));
    }

    let root = repository.workdir().ok_or_else(|| {
        RepositoryError::new(
            operation,
            Some(selected.clone()),
            RepositoryErrorKind::BareRepository,
            "repository has no working directory",
        )
    })?;
    let root = std::fs::canonicalize(root)
        .map_err(|error| RepositoryError::io(operation, Some(selected.clone()), error))?;
    if selected != root {
        return Err(RepositoryError::new(
            operation,
            Some(root),
            RepositoryErrorKind::InvalidPath,
            "the selected path is not the repository working-directory root",
        ));
    }

    Ok((repository, root))
}

fn canonicalization_error(
    operation: RepositoryOperation,
    error: std::io::Error,
) -> RepositoryError {
    let kind = match error.kind() {
        std::io::ErrorKind::NotFound
        | std::io::ErrorKind::NotADirectory
        | std::io::ErrorKind::InvalidInput => RepositoryErrorKind::InvalidPath,
        std::io::ErrorKind::PermissionDenied => RepositoryErrorKind::InaccessibleRepository,
        _ => RepositoryErrorKind::Io,
    };
    RepositoryError::with_source(operation, None, kind, error)
}

fn registry_root_key(root: &Path, operation: RepositoryOperation) -> Result<&str, RepositoryError> {
    root.to_str().ok_or_else(|| {
        RepositoryError::new(
            operation,
            Some(root.to_owned()),
            RepositoryErrorKind::InvalidPath,
            "the canonical repository root is not valid UTF-8",
        )
    })
}

fn checked_out_branch(
    repository: &Repository,
    root: &Path,
    operation: RepositoryOperation,
) -> Result<String, RepositoryError> {
    let head = repository
        .find_reference("HEAD")
        .map_err(|error| RepositoryError::git(operation, Some(root.to_owned()), error))?;
    let Some(name) = head.symbolic_target() else {
        return Err(RepositoryError::new(
            operation,
            Some(root.to_owned()),
            RepositoryErrorKind::DetachedHead,
            "HEAD is detached",
        ));
    };
    name.strip_prefix("refs/heads/")
        .map(str::to_owned)
        .ok_or_else(|| {
            RepositoryError::new(
                operation,
                Some(root.to_owned()),
                RepositoryErrorKind::DetachedHead,
                "HEAD does not name a local branch",
            )
        })
}

fn local_branches(repository: &Repository, root: &Path) -> Result<Vec<String>, RepositoryError> {
    let mut branches = repository
        .branches(Some(BranchType::Local))
        .map_err(|error| {
            RepositoryError::git(RepositoryOperation::Inspect, Some(root.to_owned()), error)
        })?
        .map(|branch| {
            let (branch, _) = branch.map_err(|error| {
                RepositoryError::git(RepositoryOperation::Inspect, Some(root.to_owned()), error)
            })?;
            branch
                .name()
                .map_err(|error| {
                    RepositoryError::git(RepositoryOperation::Inspect, Some(root.to_owned()), error)
                })
                .map(|name| name.map(str::to_owned))
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    branches.sort();
    Ok(branches)
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorktreeState {
    Clean,
    Dirty,
    Conflicted,
}

#[allow(dead_code)]
fn worktree_state(repository: &Repository, root: &Path) -> Result<WorktreeState, RepositoryError> {
    let mut options = StatusOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true);
    let statuses = repository.statuses(Some(&mut options)).map_err(|error| {
        RepositoryError::git(RepositoryOperation::Enable, Some(root.to_owned()), error)
    })?;
    if statuses
        .iter()
        .any(|entry| entry.status().contains(Status::CONFLICTED))
    {
        Ok(WorktreeState::Conflicted)
    } else if statuses.iter().all(|entry| {
        entry.path() == Some(canonical::CONFIG_PATH)
            && committed_configuration_matches_worktree(repository, root).unwrap_or(false)
    }) {
        Ok(WorktreeState::Clean)
    } else {
        Ok(WorktreeState::Dirty)
    }
}

fn committed_configuration_matches_worktree(
    repository: &Repository,
    root: &Path,
) -> Result<bool, ()> {
    let tree = repository
        .head()
        .and_then(|head| head.peel_to_tree())
        .map_err(|_| ())?;
    let entry = tree
        .get_path(Path::new(canonical::CONFIG_PATH))
        .map_err(|_| ())?;
    let blob = repository.find_blob(entry.id()).map_err(|_| ())?;
    let source = std::fs::read(root.join(canonical::CONFIG_PATH)).map_err(|_| ())?;
    Ok(blob.content() == source)
}

fn read_configuration(root: &Path) -> Result<ConfigurationInspection, RepositoryError> {
    read_configuration_for(root, RepositoryOperation::Inspect)
}

fn read_configuration_for(
    root: &Path,
    operation: RepositoryOperation,
) -> Result<ConfigurationInspection, RepositoryError> {
    let path = root.join(canonical::CONFIG_PATH);
    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ConfigurationInspection::Missing);
        }
        Err(error) => {
            return Err(RepositoryError::io(operation, Some(root.to_owned()), error));
        }
    };
    Ok(match canonical::parse_repository_config(&source) {
        Ok(config) => ConfigurationInspection::Valid(config),
        Err(problem) => ConfigurationInspection::Invalid(problem),
    })
}

fn canonical_configuration(
    primary_branch: &str,
    root: &Path,
    operation: RepositoryOperation,
) -> Result<String, RepositoryError> {
    canonical::serialize_repository_config(&canonical::RepositoryConfig {
        primary_branch: primary_branch.to_owned(),
        publication_remote: None,
        unknown: toml::Table::new(),
    })
    .map_err(|problem| {
        RepositoryError::new(
            operation,
            Some(root.to_owned()),
            RepositoryErrorKind::InvalidConfiguration,
            problem.message,
        )
    })
}

fn validate_creation_target(root: &Path) -> Result<(), RepositoryError> {
    let metadata = std::fs::symlink_metadata(root).map_err(|error| {
        RepositoryError::io(
            RepositoryOperation::CreateAndEnable,
            Some(root.to_owned()),
            error,
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(RepositoryError::new(
            RepositoryOperation::CreateAndEnable,
            Some(root.to_owned()),
            RepositoryErrorKind::InvalidPath,
            "the selected target is no longer a real directory",
        ));
    }
    let canonical = std::fs::canonicalize(root)
        .map_err(|error| canonicalization_error(RepositoryOperation::CreateAndEnable, error))?;
    if canonical != root {
        return Err(RepositoryError::new(
            RepositoryOperation::CreateAndEnable,
            Some(root.to_owned()),
            RepositoryErrorKind::InvalidPath,
            "the selected target no longer resolves to its original path",
        ));
    }
    Ok(())
}

struct OwnedTarget {
    root: PathBuf,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

impl OwnedTarget {
    fn new(root: &Path) -> Result<Self, RepositoryError> {
        validate_creation_target(root)?;
        let metadata = std::fs::symlink_metadata(root).map_err(|error| {
            RepositoryError::io(
                RepositoryOperation::CreateAndEnable,
                Some(root.to_owned()),
                error,
            )
        })?;
        Ok(Self {
            root: root.to_owned(),
            #[cfg(unix)]
            device: {
                use std::os::unix::fs::MetadataExt;
                metadata.dev()
            },
            #[cfg(unix)]
            inode: {
                use std::os::unix::fs::MetadataExt;
                metadata.ino()
            },
        })
    }

    fn remove_if_empty(&self) -> Result<(), std::io::Error> {
        let metadata = std::fs::symlink_metadata(&self.root)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(std::io::Error::other(
                "the operation-created target was replaced before cleanup",
            ));
        }
        if std::fs::canonicalize(&self.root)? != self.root {
            return Err(std::io::Error::other(
                "the operation-created target no longer resolves to its original path",
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if metadata.dev() != self.device || metadata.ino() != self.inode {
                return Err(std::io::Error::other(
                    "the operation-created target was replaced before cleanup",
                ));
            }
        }
        if std::fs::read_dir(&self.root)?.next().is_some() {
            return Err(std::io::Error::other(
                "the operation-created target is no longer empty",
            ));
        }
        std::fs::remove_dir(&self.root)
    }
}

fn read_bytes_if_exists(
    path: &Path,
    operation: RepositoryOperation,
    root: &Path,
) -> Result<Option<Vec<u8>>, RepositoryError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(RepositoryError::io(operation, Some(root.to_owned()), error)),
    }
}

fn ensure_worktree_exclusion(repository: &Repository, root: &Path) -> Result<(), RepositoryError> {
    let path = repository.commondir().join("info/exclude");
    let before = read_bytes_if_exists(&path, RepositoryOperation::Enable, root)?;
    let Some(source) = before else {
        return write_exclude(&path, b".manyhands/worktrees/\n", root);
    };
    let mut normalized = Vec::with_capacity(source.len());
    let mut matched = 0;
    let mut delimiter = None;
    let mut start = 0;
    while start < source.len() {
        let end = source[start..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|offset| start + offset + 1)
            .unwrap_or(source.len());
        let line = &source[start..end];
        let content = line
            .strip_suffix(b"\n")
            .unwrap_or(line)
            .strip_suffix(b"\r")
            .unwrap_or(line.strip_suffix(b"\n").unwrap_or(line));
        if content == b".manyhands/worktrees/" {
            matched += 1;
            if delimiter.is_none() {
                delimiter = line
                    .ends_with(b"\r\n")
                    .then_some(b"\r\n".as_slice())
                    .or_else(|| line.ends_with(b"\n").then_some(b"\n".as_slice()));
            }
        } else {
            if delimiter.is_none() {
                delimiter = line
                    .ends_with(b"\r\n")
                    .then_some(b"\r\n".as_slice())
                    .or_else(|| line.ends_with(b"\n").then_some(b"\n".as_slice()));
            }
            normalized.extend_from_slice(line);
        }
        start = end;
    }
    if matched == 1 && source.ends_with(b"\n") {
        return Ok(());
    }
    let delimiter = delimiter.unwrap_or(b"\n");
    if !normalized.is_empty() && !normalized.ends_with(b"\n") {
        normalized.extend_from_slice(delimiter);
    }
    normalized.extend_from_slice(b".manyhands/worktrees/");
    normalized.extend_from_slice(delimiter);
    write_exclude(&path, &normalized, root)
}

fn write_exclude(path: &Path, source: &[u8], root: &Path) -> Result<(), RepositoryError> {
    replace_bytes_atomically(path, source, root)
}

fn write_configuration_atomically(
    path: &Path,
    source: &str,
    root: &Path,
) -> Result<(), RepositoryError> {
    replace_bytes_atomically(path, source.as_bytes(), root)?;
    Ok(())
}

fn prepare_configuration_parent(root: &Path) -> std::io::Result<bool> {
    match std::fs::create_dir(root.join(".manyhands")) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(error) => Err(error),
    }
}

fn snapshot_unborn_head(repository: &Repository, root: &Path) -> Result<String, RepositoryError> {
    let head = repository.find_reference("HEAD").map_err(|error| {
        RepositoryError::git(RepositoryOperation::Enable, Some(root.to_owned()), error)
    })?;
    head.symbolic_target().map(str::to_owned).ok_or_else(|| {
        RepositoryError::new(
            RepositoryOperation::Enable,
            Some(root.to_owned()),
            RepositoryErrorKind::Git,
            "unborn repository HEAD is not symbolic",
        )
    })
}

fn replace_bytes_atomically(
    path: &Path,
    source: &[u8],
    root: &Path,
) -> Result<(), RepositoryError> {
    let parent = path.parent().expect("restored file has a parent");
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|error| {
        RepositoryError::io(RepositoryOperation::Enable, Some(root.to_owned()), error)
    })?;
    use std::io::Write;
    temporary
        .write_all(source)
        .and_then(|()| temporary.flush())
        .map_err(|error| {
            RepositoryError::io(RepositoryOperation::Enable, Some(root.to_owned()), error)
        })?;
    temporary.persist(path).map_err(|error| {
        RepositoryError::io(
            RepositoryOperation::Enable,
            Some(root.to_owned()),
            error.error,
        )
    })?;
    Ok(())
}

fn initialization_tree(
    repository: &Repository,
    config_path: &Path,
    unborn: bool,
    root: &Path,
) -> Result<git2::Oid, RepositoryError> {
    let mut index = Index::new().map_err(|error| {
        RepositoryError::git(RepositoryOperation::Enable, Some(root.to_owned()), error)
    })?;
    if !unborn {
        let tree = repository
            .head()
            .and_then(|head| head.peel_to_tree())
            .map_err(|error| {
                RepositoryError::git(RepositoryOperation::Enable, Some(root.to_owned()), error)
            })?;
        index.read_tree(&tree).map_err(|error| {
            RepositoryError::git(RepositoryOperation::Enable, Some(root.to_owned()), error)
        })?;
    }
    let source = std::fs::read(config_path).map_err(|error| {
        RepositoryError::io(RepositoryOperation::Enable, Some(root.to_owned()), error)
    })?;
    let blob = repository.blob(&source).map_err(|error| {
        RepositoryError::git(RepositoryOperation::Enable, Some(root.to_owned()), error)
    })?;
    index
        .add(&IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            file_size: source.len() as u32,
            id: blob,
            flags: 0,
            flags_extended: 0,
            path: canonical::CONFIG_PATH.as_bytes().to_vec(),
        })
        .map_err(|error| {
            RepositoryError::git(RepositoryOperation::Enable, Some(root.to_owned()), error)
        })?;
    index.write_tree_to(repository).map_err(|error| {
        RepositoryError::git(RepositoryOperation::Enable, Some(root.to_owned()), error)
    })
}

fn restore_enablement(
    config_path: &Path,
    config_before: Option<&[u8]>,
    exclude_path: &Path,
    exclude_before: Option<&[u8]>,
    identity: Option<(&Path, Option<&[u8]>)>,
    created_manyhands_directory: Option<&Path>,
    unborn: Option<(&Repository, Option<&str>, &str)>,
) -> Result<(), Box<RestoreFailures>> {
    let config = restore_file(config_path, config_before).err();
    let exclude = restore_file(exclude_path, exclude_before).err();
    let identity = identity.and_then(|(path, before)| restore_file(path, before).err());
    let mut failures = RestoreFailures {
        config,
        exclude,
        identity,
        head: None,
        branch: None,
        directory: None,
    };
    if let Some((repository, prior, branch)) = unborn {
        restore_unborn_head(repository, prior, branch, &mut failures);
    }
    if let Some(directory) = created_manyhands_directory {
        failures.directory = std::fs::remove_dir(directory).err();
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(Box::new(failures))
    }
}

fn restore_file(path: &Path, prior: Option<&[u8]>) -> Result<(), RestoreFileError> {
    match prior {
        Some(bytes) => {
            replace_bytes_atomically(path, bytes, path).map_err(RestoreFileError::Repository)
        }
        None => match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(RestoreFileError::Io(error)),
        },
    }
}

#[derive(Debug)]
enum RestoreFileError {
    Io(std::io::Error),
    Repository(RepositoryError),
}

impl fmt::Display for RestoreFileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => error.fmt(formatter),
            Self::Repository(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for RestoreFileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Repository(error) => Some(error),
        }
    }
}

#[derive(Debug)]
struct RestoreFailures {
    config: Option<RestoreFileError>,
    exclude: Option<RestoreFileError>,
    identity: Option<RestoreFileError>,
    head: Option<git2::Error>,
    branch: Option<git2::Error>,
    directory: Option<std::io::Error>,
}

impl RestoreFailures {
    fn is_empty(&self) -> bool {
        self.config.is_none()
            && self.exclude.is_none()
            && self.identity.is_none()
            && self.head.is_none()
            && self.branch.is_none()
            && self.directory.is_none()
    }
}

impl fmt::Display for RestoreFailures {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "canonical rollback did not complete")?;
        if let Some(error) = &self.config {
            write!(formatter, "; configuration restoration failed: {error}")?;
        }
        if let Some(error) = &self.exclude {
            write!(formatter, "; exclude restoration failed: {error}")?;
        }
        if let Some(error) = &self.identity {
            write!(formatter, "; local identity restoration failed: {error}")?;
        }
        if let Some(error) = &self.head {
            write!(formatter, "; symbolic HEAD restoration failed: {error}")?;
        }
        if let Some(error) = &self.branch {
            write!(formatter, "; unborn branch cleanup failed: {error}")?;
        }
        if let Some(error) = &self.directory {
            write!(formatter, "; created .manyhands cleanup failed: {error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for RestoreFailures {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.config
            .as_ref()
            .map(|error| error as &(dyn std::error::Error + 'static))
            .or_else(|| {
                self.exclude
                    .as_ref()
                    .map(|error| error as &(dyn std::error::Error + 'static))
            })
            .or_else(|| {
                self.identity
                    .as_ref()
                    .map(|error| error as &(dyn std::error::Error + 'static))
            })
            .or_else(|| {
                self.head
                    .as_ref()
                    .map(|error| error as &(dyn std::error::Error + 'static))
            })
            .or_else(|| {
                self.branch
                    .as_ref()
                    .map(|error| error as &(dyn std::error::Error + 'static))
            })
            .or_else(|| {
                self.directory
                    .as_ref()
                    .map(|error| error as &(dyn std::error::Error + 'static))
            })
    }
}

#[derive(Debug)]
struct RollbackFailure {
    original: RepositoryError,
    restoration: RestoreFailures,
}

impl fmt::Display for RollbackFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} after original failure: {}",
            self.restoration, self.original
        )
    }
}

impl std::error::Error for RollbackFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.original)
    }
}

#[derive(Debug)]
struct CreationCleanupFailure {
    original: RepositoryError,
    cleanup: std::io::Error,
}

impl fmt::Display for CreationCleanupFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "operation-created target cleanup failed: {} after original failure: {}",
            self.cleanup, self.original
        )
    }
}

impl std::error::Error for CreationCleanupFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.original)
    }
}

fn creation_cleanup_incomplete(
    root: &Path,
    original: RepositoryError,
    cleanup: std::io::Error,
) -> RepositoryError {
    RepositoryError::with_source(
        RepositoryOperation::CreateAndEnable,
        Some(root.to_owned()),
        RepositoryErrorKind::RollbackIncomplete,
        CreationCleanupFailure { original, cleanup },
    )
}

fn rollback_incomplete(
    root: &Path,
    original: RepositoryError,
    restoration: RestoreFailures,
) -> RepositoryError {
    rollback_incomplete_for(RepositoryOperation::Enable, root, original, restoration)
}

fn rollback_incomplete_for(
    operation: RepositoryOperation,
    root: &Path,
    original: RepositoryError,
    restoration: RestoreFailures,
) -> RepositoryError {
    RepositoryError::with_source(
        operation,
        Some(root.to_owned()),
        RepositoryErrorKind::RollbackIncomplete,
        RollbackFailure {
            original,
            restoration,
        },
    )
}

fn restore_unborn_head(
    repository: &Repository,
    prior: Option<&str>,
    primary_branch: &str,
    failures: &mut RestoreFailures,
) {
    if let Some(prior) = prior {
        failures.head = repository.set_head(prior).err();
    }
    match repository.find_reference(&format!("refs/heads/{primary_branch}")) {
        Ok(mut reference) => failures.branch = reference.delete().err(),
        Err(error) if error.code() == git2::ErrorCode::NotFound => {}
        Err(error) => failures.branch = Some(error),
    }
}

fn write_local_identity(
    repository: &Repository,
    root: &Path,
    identity: &CommitIdentity,
) -> Result<(), RepositoryError> {
    let mut config = repository.config().map_err(|error| {
        RepositoryError::git(RepositoryOperation::Enable, Some(root.to_owned()), error)
    })?;
    config
        .set_str("user.name", &identity.name)
        .map_err(|error| {
            RepositoryError::git(RepositoryOperation::Enable, Some(root.to_owned()), error)
        })?;
    config
        .set_str("user.email", &identity.email)
        .map_err(|error| {
            RepositoryError::git(RepositoryOperation::Enable, Some(root.to_owned()), error)
        })
}

fn publication_remote_eligible(
    repository: &Repository,
    root: &Path,
    name: &str,
) -> Result<bool, RepositoryError> {
    publication_remote_eligible_for(repository, root, name, RepositoryOperation::Enable)
}

fn publication_remote_eligible_for(
    repository: &Repository,
    root: &Path,
    name: &str,
    operation: RepositoryOperation,
) -> Result<bool, RepositoryError> {
    let remote = match repository.find_remote(name) {
        Ok(remote) => remote,
        Err(error) if error.code() == git2::ErrorCode::NotFound => return Ok(false),
        Err(error) => {
            return Err(RepositoryError::git(
                operation,
                Some(root.to_owned()),
                error,
            ));
        }
    };
    let Some(fetch) = remote.url() else {
        return Ok(false);
    };
    Ok(ssh_compatible(fetch) && ssh_compatible(remote.pushurl().unwrap_or(fetch)))
}

fn ssh_compatible(url: &str) -> bool {
    if let Some(rest) = url.strip_prefix("ssh://") {
        let Some((authority, path)) = rest.split_once('/') else {
            return false;
        };
        return !authority.is_empty()
            && !path.is_empty()
            && authority.matches('@').count() <= 1
            && !authority.contains(['/', '\\'])
            && !authority.contains(char::is_whitespace)
            && ssh_authority_compatible(authority);
    }
    if url.contains("://") || url.contains('\\') || url.contains(char::is_whitespace) {
        return false;
    }
    let Some((host, path)) = url.split_once(':') else {
        return false;
    };
    let (user, hostname) = host
        .split_once('@')
        .map_or((None, host), |(user, hostname)| (Some(user), hostname));
    !host.is_empty()
        && !path.is_empty()
        && !hostname.is_empty()
        && host.matches('@').count() <= 1
        && user.is_none_or(valid_scp_user)
        && valid_scp_host(hostname)
        && !(host.len() == 1 && host.as_bytes()[0].is_ascii_alphabetic())
}

fn valid_scp_user(user: &str) -> bool {
    !user.is_empty()
        && user
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

fn valid_scp_host(host: &str) -> bool {
    !matches!(host, "file" | "http" | "https")
        && !host.starts_with(['.', '/'])
        && host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
        && host
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && host
            .bytes()
            .last()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
}

fn ssh_authority_compatible(authority: &str) -> bool {
    let host_port = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host_port)| host_port);
    if let Some(rest) = host_port.strip_prefix('[') {
        let Some((host, port)) = rest.split_once(']') else {
            return false;
        };
        return !host.is_empty()
            && match port {
                "" => true,
                port if port.starts_with(':') => valid_ssh_port(&port[1..]),
                _ => false,
            };
    }
    match host_port.split_once(':') {
        Some((host, port)) => !host.is_empty() && valid_ssh_port(port),
        None => !host_port.is_empty(),
    }
}

fn valid_ssh_port(port: &str) -> bool {
    port.parse::<u16>().is_ok()
}

fn resolve_identity(
    local_config: &Config,
    effective_config: &Config,
    root: &Path,
    operation: RepositoryOperation,
) -> Result<Option<CommitIdentity>, RepositoryError> {
    if let Some(identity) =
        complete_identity_at_level(local_config, ConfigLevel::Local, root, operation)?
    {
        return Ok(Some(identity));
    }
    for level in [
        ConfigLevel::Global,
        ConfigLevel::XDG,
        ConfigLevel::System,
        ConfigLevel::ProgramData,
        ConfigLevel::App,
    ] {
        if let Some(identity) =
            complete_identity_at_level(effective_config, level, root, operation)?
        {
            return Ok(Some(identity));
        }
    }
    Ok(None)
}

fn complete_identity_at_level(
    config: &Config,
    level: ConfigLevel,
    root: &Path,
    operation: RepositoryOperation,
) -> Result<Option<CommitIdentity>, RepositoryError> {
    let mut name = None;
    let mut email = None;
    let mut entries = config
        .entries(Some("user.*"))
        .map_err(|error| RepositoryError::git(operation, Some(root.to_owned()), error))?;
    while let Some(entry) = entries.next() {
        let entry =
            entry.map_err(|error| RepositoryError::git(operation, Some(root.to_owned()), error))?;
        if entry.level() != level {
            continue;
        }
        match entry.name() {
            Some("user.name") => name = entry.value().map(str::to_owned),
            Some("user.email") => email = entry.value().map(str::to_owned),
            _ => {}
        }
    }
    Ok(match (name, email) {
        (Some(name), Some(email)) if !name.is_empty() && !email.is_empty() => {
            Some(CommitIdentity { name, email })
        }
        _ => None,
    })
}

#[allow(dead_code)] // Task 4 calls this before its first local config write.
fn validate_identity(
    identity: &CommitIdentity,
    operation: RepositoryOperation,
) -> Result<(), RepositoryError> {
    if identity.name.is_empty()
        || identity.email.is_empty()
        || identity.name.contains('\0')
        || identity.email.contains('\0')
    {
        return Err(RepositoryError::new(
            operation,
            None,
            RepositoryErrorKind::InvalidIdentity,
            "commit identity name and email must be nonempty and NUL-free",
        ));
    }
    Ok(())
}

fn remote_info(repository: &Repository, root: &Path) -> Result<Vec<RemoteInfo>, RepositoryError> {
    remote_info_for(repository, root, RepositoryOperation::Inspect)
}

fn remote_info_for(
    repository: &Repository,
    root: &Path,
    operation: RepositoryOperation,
) -> Result<Vec<RemoteInfo>, RepositoryError> {
    let mut remotes = Vec::new();
    for name in repository
        .remotes()
        .map_err(|error| RepositoryError::git(operation, Some(root.to_owned()), error))?
        .iter()
        .flatten()
    {
        let remote = repository
            .find_remote(name)
            .map_err(|error| RepositoryError::git(operation, Some(root.to_owned()), error))?;
        let Some(fetch_url) = remote.url() else {
            continue;
        };
        remotes.push(RemoteInfo {
            name: name.to_owned(),
            fetch_url: fetch_url.to_owned(),
            push_url: remote.pushurl().unwrap_or(fetch_url).to_owned(),
            publication_eligible: ssh_compatible(fetch_url)
                && ssh_compatible(remote.pushurl().unwrap_or(fetch_url)),
        });
    }
    remotes.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(remotes)
}

fn ensure_configuration_path_clean(
    repository: &Repository,
    root: &Path,
    operation: RepositoryOperation,
) -> Result<(), RepositoryError> {
    let head = repository
        .head()
        .and_then(|head| head.peel_to_commit())
        .map_err(|error| RepositoryError::git(operation, Some(root.to_owned()), error))?;
    let head_entry = configuration_entry_from_tree(
        &head
            .tree()
            .map_err(|error| RepositoryError::git(operation, Some(root.to_owned()), error))?,
    );
    let worktree_matches_head = head_entry.is_some_and(|(oid, _)| {
        std::fs::symlink_metadata(root.join(canonical::CONFIG_PATH))
            .ok()
            .filter(regular_nonexecutable_file)
            .and_then(|_| repository.find_blob(oid).ok())
            .and_then(|blob| {
                std::fs::read(root.join(canonical::CONFIG_PATH))
                    .ok()
                    .map(|bytes| bytes == blob.content())
            })
            .unwrap_or(false)
    });
    let index = repository
        .index()
        .map_err(|error| RepositoryError::git(operation, Some(root.to_owned()), error))?;
    let index_entry = index
        .get_path(Path::new(canonical::CONFIG_PATH), 0)
        .map(|entry| (entry.id, entry.mode));
    let index_matches_ancestor = match index_entry {
        Some(index_entry) => {
            first_parent_configuration_entries(repository, &head, operation, root)?
                .into_iter()
                .any(|ancestor| index_entry == ancestor)
        }
        None => false,
    };
    if !worktree_matches_head || !index_matches_ancestor {
        return Err(RepositoryError::new(
            operation,
            Some(root.to_owned()),
            RepositoryErrorKind::DirtyConfigurationPath,
            "the Manyhands configuration path must match HEAD; stage or reset it before publication",
        ));
    }
    Ok(())
}

fn regular_nonexecutable_file(metadata: &std::fs::Metadata) -> bool {
    if !metadata.file_type().is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 != 0 {
            return false;
        }
    }
    true
}

fn first_parent_configuration_entries(
    repository: &Repository,
    commit: &git2::Commit<'_>,
    operation: RepositoryOperation,
    root: &Path,
) -> Result<Vec<(git2::Oid, u32)>, RepositoryError> {
    let mut commit = commit.clone();
    let mut entries = Vec::new();
    loop {
        let tree = commit
            .tree()
            .map_err(|error| RepositoryError::git(operation, Some(root.to_owned()), error))?;
        if let Some(entry) = configuration_entry_from_tree(&tree)
            && repository.find_blob(entry.0).ok().is_some_and(|blob| {
                std::str::from_utf8(blob.content()).is_ok_and(|source| {
                    canonical::parse_repository_config(source).is_ok_and(|config| {
                        canonical::serialize_repository_config(&config)
                            .is_ok_and(|canonical| canonical.as_bytes() == blob.content())
                    })
                })
            })
        {
            entries.push(entry);
        }
        if commit.parent_count() == 0 {
            break;
        }
        commit = commit
            .parent(0)
            .map_err(|error| RepositoryError::git(operation, Some(root.to_owned()), error))?;
    }
    Ok(entries)
}

fn configuration_entry_from_tree(tree: &git2::Tree<'_>) -> Option<(git2::Oid, u32)> {
    tree.get_path(Path::new(canonical::CONFIG_PATH))
        .ok()
        .filter(|entry| entry.filemode() == 0o100644)
        .map(|entry| (entry.id(), 0o100644))
}

fn configuration_tree(
    repository: &Repository,
    source: &[u8],
    root: &Path,
    operation: RepositoryOperation,
) -> Result<git2::Oid, RepositoryError> {
    let head_tree = repository
        .head()
        .and_then(|head| head.peel_to_tree())
        .map_err(|error| RepositoryError::git(operation, Some(root.to_owned()), error))?;
    let mut index = Index::new()
        .map_err(|error| RepositoryError::git(operation, Some(root.to_owned()), error))?;
    index
        .read_tree(&head_tree)
        .map_err(|error| RepositoryError::git(operation, Some(root.to_owned()), error))?;
    let blob = repository
        .blob(source)
        .map_err(|error| RepositoryError::git(operation, Some(root.to_owned()), error))?;
    index
        .add(&IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            file_size: source.len() as u32,
            id: blob,
            flags: 0,
            flags_extended: 0,
            path: canonical::CONFIG_PATH.as_bytes().to_vec(),
        })
        .map_err(|error| RepositoryError::git(operation, Some(root.to_owned()), error))?;
    index
        .write_tree_to(repository)
        .map_err(|error| RepositoryError::git(operation, Some(root.to_owned()), error))
}

fn mark_registered_refresh_required(
    registry_path: &Path,
    root: &Path,
    operation: RepositoryOperation,
) -> Result<(), RepositoryError> {
    let _cache_guard = cache_read_guard(registry_path, root, operation)?;
    let root_path = registry_root_key(root, operation)?;
    let connection = open_registry(registry_path, &mut |_| {})
        .map_err(|error| registry_refresh_pending(operation, root, error))?;
    connection
        .execute(
            "UPDATE repositories SET refresh_required = 1 WHERE root_path = ?1",
            [root_path],
        )
        .map_err(|error| registry_refresh_pending(operation, root, error))?;
    Ok(())
}

fn mark_document_refresh_required(
    registry_path: &Path,
    root: &Path,
    operation: RepositoryOperation,
) -> Result<(), RepositoryError> {
    let _cache_guard = cache_read_guard(registry_path, root, operation)?;
    let root_path = registry_root_key(root, operation)?;
    let connection = open_registry(registry_path, &mut |_| {})
        .map_err(|error| registry_refresh_pending(operation, root, error))?;
    let updated = connection
        .execute(
            "UPDATE repositories SET refresh_required = 1 WHERE root_path = ?1",
            [root_path],
        )
        .map_err(|error| registry_refresh_pending(operation, root, error))?;
    if updated != 1 {
        return Err(registry_refresh_pending(
            operation,
            root,
            std::io::Error::other("the repository registration is missing or duplicated"),
        ));
    }
    Ok(())
}

fn registry_refresh_pending(
    operation: RepositoryOperation,
    root: &Path,
    error: impl std::error::Error + Send + Sync + 'static,
) -> RepositoryError {
    RepositoryError {
        root: Some(root.to_owned()),
        operation,
        kind: RepositoryErrorKind::RegistryRefreshPending,
        message: "Git remote configuration is authoritative; local registry refresh is pending"
            .to_owned(),
        source: Some(Box::new(error)),
    }
}

fn reconcile_registration(
    registry_path: &Path,
    repository: &Repository,
    root: &Path,
) -> Result<(), RepositoryError> {
    let config_blob_oid = committed_configuration_blob_oid(repository, root)?;
    let enabled_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            RepositoryError::new(
                RepositoryOperation::Enable,
                Some(root.to_owned()),
                RepositoryErrorKind::Io,
                error,
            )
        })?
        .as_secs() as i64;
    let root_path = registry_root_key(root, RepositoryOperation::Enable)?;
    let _cache_guard = cache_read_guard(registry_path, root, RepositoryOperation::Enable)?;
    let mut connection = open_registry(registry_path, &mut |_| {})
        .map_err(|error| error.for_operation(RepositoryOperation::Enable, root))?;
    migrate_registry(&mut connection)
        .map_err(|error| error.for_operation(RepositoryOperation::Enable, root))?;
    let transaction = connection.transaction().map_err(|error| {
        RepositoryError::sqlite(error).for_operation(RepositoryOperation::Enable, root)
    })?;
    let inserted = transaction
        .execute(
            "INSERT INTO repositories (
                root_path, enabled_at, accessibility, config_blob_oid, refresh_required
            ) VALUES (?1, ?2, 'accessible', ?3, 1)
            ON CONFLICT(root_path) DO NOTHING",
            (&root_path, enabled_at, config_blob_oid.to_string()),
        )
        .map_err(|error| {
            RepositoryError::sqlite(error).for_operation(RepositoryOperation::Enable, root)
        })?;
    if inserted == 0 {
        transaction
            .query_row(
                "SELECT id FROM repositories WHERE root_path = ?1",
                [&root_path],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|error| {
                RepositoryError::sqlite(error).for_operation(RepositoryOperation::Enable, root)
            })?;
        transaction
            .execute(
                "UPDATE repositories
                 SET accessibility = 'accessible', config_blob_oid = ?2, refresh_required = 1
                 WHERE root_path = ?1",
                (root_path, config_blob_oid.to_string()),
            )
            .map_err(|error| {
                RepositoryError::sqlite(error).for_operation(RepositoryOperation::Enable, root)
            })?;
    }
    transaction.commit().map_err(|error| {
        RepositoryError::sqlite(error).for_operation(RepositoryOperation::Enable, root)
    })
}

fn committed_configuration_blob_oid(
    repository: &Repository,
    root: &Path,
) -> Result<git2::Oid, RepositoryError> {
    let config_oid = repository
        .head()
        .and_then(|head| head.peel_to_tree())
        .and_then(|tree| tree.get_path(Path::new(canonical::CONFIG_PATH)))
        .map(|entry| entry.id())
        .map_err(|error| {
            RepositoryError::git(RepositoryOperation::Enable, Some(root.to_owned()), error)
        })?;
    repository
        .find_blob(config_oid)
        .map(|_| config_oid)
        .map_err(|error| {
            RepositoryError::git(RepositoryOperation::Enable, Some(root.to_owned()), error)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_service_is_sync() {
        fn assert_sync<T: Sync>() {}

        assert_sync::<RepositoryService>();
    }

    #[test]
    fn concurrent_cross_root_operation_ids_yield_one_mismatch() {
        let first_directory = tempfile::tempdir().unwrap();
        let second_directory = tempfile::tempdir().unwrap();
        let first_root = create_born_repository(first_directory.path());
        let second_root = create_born_repository(second_directory.path());
        let data = tempfile::tempdir().unwrap();
        let first_service = RepositoryService::open_at(data.path()).unwrap();
        let second_service = RepositoryService::open_at(data.path()).unwrap();
        let operation_id = OperationId::new();
        let _barrier = recovery::pause_before_begin_for_testing(
            operation_id,
            Arc::new(std::sync::Barrier::new(2)),
        );

        let results = std::thread::scope(|scope| {
            let first_call = scope.spawn(|| {
                first_service.rebuild_repository(RebuildRepositoryRequest {
                    root: first_root,
                    operation_id,
                })
            });
            let second_call = scope.spawn(|| {
                second_service.rebuild_repository(RebuildRepositoryRequest {
                    root: second_root,
                    operation_id,
                })
            });
            [first_call.join().unwrap(), second_call.join().unwrap()]
        });

        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter_map(|result| result.as_ref().err())
                .map(|error| error.kind)
                .collect::<Vec<_>>(),
            vec![RepositoryErrorKind::OperationMismatch]
        );
    }

    fn create_born_repository(root: &Path) -> PathBuf {
        let mut options = RepositoryInitOptions::new();
        options.initial_head("main");
        let repository = Repository::init_opts(root, &options).unwrap();
        let mut config = Config::open(&repository.path().join("config")).unwrap();
        config.set_str("user.name", "Manyhands Test").unwrap();
        config
            .set_str("user.email", "manyhands-test@example.invalid")
            .unwrap();
        std::fs::write(root.join("fixture.txt"), "fixture\n").unwrap();
        let mut index = repository.index().unwrap();
        index.add_path(Path::new("fixture.txt")).unwrap();
        let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
        let signature = Signature::new(
            "Manyhands Test",
            "manyhands-test@example.invalid",
            &git2::Time::new(0, 0),
        )
        .unwrap();
        repository
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                "Initial fixture commit",
                &tree,
                &[],
            )
            .unwrap();
        root.to_owned()
    }

    #[test]
    fn permission_denied_error_mappers_preserve_kind_operation_and_root() {
        let root = Path::new("/repository");

        let canonicalization = canonicalization_error(
            RepositoryOperation::Inspect,
            std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        );
        assert_eq!(
            canonicalization.kind,
            RepositoryErrorKind::InaccessibleRepository
        );
        assert_eq!(canonicalization.operation, RepositoryOperation::Inspect);
        assert_eq!(canonicalization.root, None);

        let write = RepositoryError::io(
            RepositoryOperation::Enable,
            Some(root.to_owned()),
            std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        );
        assert_eq!(write.kind, RepositoryErrorKind::Io);
        assert_eq!(write.operation, RepositoryOperation::Enable);
        assert_eq!(write.root.as_deref(), Some(root));
    }

    #[test]
    fn default_data_directory_reports_an_io_error_when_platform_data_is_absent() {
        let error = resolve_default_data_directory(None).unwrap_err();

        assert_eq!(error.operation, RepositoryOperation::OpenRegistry);
        assert_eq!(error.kind, RepositoryErrorKind::Io);
    }

    #[test]
    fn default_data_directory_preserves_a_platform_data_directory() {
        let supplied = PathBuf::from("/application/data");

        assert_eq!(
            resolve_default_data_directory(Some(supplied.clone())).unwrap(),
            supplied
        );
    }

    #[test]
    fn publication_rollback_incomplete_preserves_commit_failure_source() {
        let original = RepositoryError::new(
            RepositoryOperation::SetPublicationRemote,
            None,
            RepositoryErrorKind::Git,
            "commit failed",
        );
        let error = rollback_incomplete_for(
            RepositoryOperation::SetPublicationRemote,
            Path::new("/repository"),
            original,
            RestoreFailures {
                config: Some(RestoreFileError::Io(std::io::Error::other(
                    "restore failed",
                ))),
                exclude: None,
                identity: None,
                head: None,
                branch: None,
                directory: None,
            },
        );

        assert_eq!(error.kind, RepositoryErrorKind::RollbackIncomplete);
        assert_eq!(error.operation, RepositoryOperation::SetPublicationRemote);
        assert!(
            std::error::Error::source(&error)
                .unwrap()
                .to_string()
                .contains("commit failed")
        );
        assert!(error.to_string().contains("restore failed"));
    }

    #[test]
    fn nonblob_configuration_tree_entry_is_not_a_stale_index_candidate() {
        let directory = tempfile::tempdir().unwrap();
        let repository = Repository::init(directory.path()).unwrap();
        let empty = repository.treebuilder(None).unwrap().write().unwrap();
        let mut manyhands = repository.treebuilder(None).unwrap();
        manyhands.insert("config.toml", empty, 0o040000).unwrap();
        let manyhands = manyhands.write().unwrap();
        let mut root = repository.treebuilder(None).unwrap();
        root.insert(".manyhands", manyhands, 0o040000).unwrap();
        let tree = repository.find_tree(root.write().unwrap()).unwrap();
        let signature =
            Signature::new("Test", "test@example.invalid", &git2::Time::new(0, 0)).unwrap();
        let commit_oid = repository
            .commit(Some("HEAD"), &signature, &signature, "nonblob", &tree, &[])
            .unwrap();
        let commit = repository.find_commit(commit_oid).unwrap();
        let blob = repository
            .blob(b"format_version = 1\nprimary_branch = \"main\"\n")
            .unwrap();
        let mut index = Index::new().unwrap();
        index
            .add(&IndexEntry {
                ctime: git2::IndexTime::new(0, 0),
                mtime: git2::IndexTime::new(0, 0),
                dev: 0,
                ino: 0,
                mode: 0o100644,
                uid: 0,
                gid: 0,
                file_size: 0,
                id: blob,
                flags: 0,
                flags_extended: 0,
                path: canonical::CONFIG_PATH.as_bytes().to_vec(),
            })
            .unwrap();

        assert!(
            first_parent_configuration_entries(
                &repository,
                &commit,
                RepositoryOperation::SetPublicationRemote,
                directory.path(),
            )
            .unwrap()
            .is_empty()
        );
        assert!(
            index
                .get_path(Path::new(canonical::CONFIG_PATH), 0)
                .is_some()
        );
    }

    #[cfg(unix)]
    #[test]
    fn creation_target_validation_rejects_a_substituted_symlink() {
        use std::os::unix::fs::symlink;

        let parent = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let root = parent.path().join("project");
        std::fs::create_dir(&root).unwrap();
        std::fs::remove_dir(&root).unwrap();
        symlink(outside.path(), &root).unwrap();

        let error = validate_creation_target(&root).unwrap_err();

        assert_eq!(error.operation, RepositoryOperation::CreateAndEnable);
        assert_eq!(error.kind, RepositoryErrorKind::InvalidPath);
        assert!(
            std::fs::symlink_metadata(&root)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(std::fs::read_dir(outside.path()).unwrap().next().is_none());
    }

    #[test]
    fn owned_target_cleanup_refuses_a_nonempty_target() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("project");
        std::fs::create_dir(&root).unwrap();
        let owned = OwnedTarget::new(&root).unwrap();
        std::fs::write(root.join("preserve.txt"), "preserve\n").unwrap();

        let error = owned.remove_if_empty().unwrap_err();

        assert!(error.to_string().contains("target is no longer empty"));
        assert!(root.join("preserve.txt").is_file());
    }

    fn isolated_config(level: ConfigLevel, source: &str) -> (tempfile::TempDir, Config) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config");
        std::fs::write(&path, source).unwrap();
        let mut config = Config::new().unwrap();
        config.add_file(&path, level, false).unwrap();
        (directory, config)
    }

    #[test]
    fn identity_resolver_uses_only_complete_values_from_one_configuration_level() {
        let (_local_directory, local) =
            isolated_config(ConfigLevel::Local, "[user]\nname = Local Name\n");
        let (_effective_directory, mut effective) = isolated_config(
            ConfigLevel::Global,
            "[user]\nemail = global@example.invalid\n",
        );

        assert!(
            resolve_identity(
                &local,
                &effective,
                Path::new("/"),
                RepositoryOperation::Inspect,
            )
            .unwrap()
            .is_none()
        );

        effective.set_str("user.name", "Global Name").unwrap();
        assert_eq!(
            resolve_identity(
                &local,
                &effective,
                Path::new("/"),
                RepositoryOperation::Inspect,
            )
            .unwrap()
            .map(|identity| (identity.name, identity.email)),
            Some((
                "Global Name".to_owned(),
                "global@example.invalid".to_owned()
            ))
        );
    }

    #[test]
    fn identity_validation_rejects_empty_or_nul_values() {
        for identity in [
            CommitIdentity {
                name: String::new(),
                email: "person@example.invalid".to_owned(),
            },
            CommitIdentity {
                name: "Person".to_owned(),
                email: String::new(),
            },
            CommitIdentity {
                name: "Person\0Name".to_owned(),
                email: "person@example.invalid".to_owned(),
            },
            CommitIdentity {
                name: "Person".to_owned(),
                email: "person\0@example.invalid".to_owned(),
            },
        ] {
            let error = validate_identity(&identity, RepositoryOperation::Enable).unwrap_err();
            assert_eq!(error.kind, RepositoryErrorKind::InvalidIdentity);
        }

        assert!(
            validate_identity(
                &CommitIdentity {
                    name: "Person".to_owned(),
                    email: "person@example.invalid".to_owned(),
                },
                RepositoryOperation::Enable
            )
            .is_ok()
        );
    }

    #[test]
    fn committed_configuration_oid_rejects_a_tree_entry() {
        let directory = tempfile::tempdir().unwrap();
        let repository = Repository::init(directory.path()).unwrap();
        let empty_tree = repository
            .find_tree(repository.treebuilder(None).unwrap().write().unwrap())
            .unwrap();
        let mut configuration_directory = repository.treebuilder(None).unwrap();
        configuration_directory
            .insert("config.toml", empty_tree.id(), 0o040000)
            .unwrap();
        let configuration_directory = repository
            .find_tree(configuration_directory.write().unwrap())
            .unwrap();
        let mut root = repository.treebuilder(None).unwrap();
        root.insert(".manyhands", configuration_directory.id(), 0o040000)
            .unwrap();
        let root = repository.find_tree(root.write().unwrap()).unwrap();
        let signature = Signature::now("Manyhands Test", "manyhands-test@example.invalid").unwrap();
        repository
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                "Tree configuration",
                &root,
                &[],
            )
            .unwrap();

        let error = committed_configuration_blob_oid(&repository, directory.path()).unwrap_err();

        assert_eq!(error.operation, RepositoryOperation::Enable);
        assert_eq!(error.kind, RepositoryErrorKind::Git);
        assert_eq!(error.root, Some(directory.path().to_owned()));
    }

    #[test]
    fn rollback_failure_from_real_files_preserves_both_restore_errors_and_original_context() {
        let directory = tempfile::tempdir().unwrap();
        let blocker = directory.path().join("not-a-directory");
        std::fs::write(&blocker, "blocker").unwrap();
        let failures = restore_enablement(
            &blocker.join("config"),
            Some(b"config"),
            &blocker.join("exclude"),
            Some(b"exclude"),
            None,
            None,
            None,
        )
        .unwrap_err();
        assert!(failures.config.is_some());
        assert!(failures.exclude.is_some());
        let original = RepositoryError::new(
            RepositoryOperation::Enable,
            Some(PathBuf::from("/repository")),
            RepositoryErrorKind::Git,
            "initialization commit failed",
        );
        let error = rollback_incomplete(Path::new("/repository"), original, *failures);

        assert_eq!(error.kind, RepositoryErrorKind::RollbackIncomplete);
        assert!(error.to_string().contains("initialization commit failed"));
        assert!(
            error
                .to_string()
                .contains("configuration restoration failed")
        );
        assert!(error.to_string().contains("exclude restoration failed"));
        assert!(std::error::Error::source(&error).is_some());
    }

    #[test]
    fn configuration_directory_creation_records_only_this_attempts_success() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();

        assert!(prepare_configuration_parent(root).unwrap());
        assert!(!prepare_configuration_parent(root).unwrap());
    }

    #[test]
    fn unborn_head_snapshot_reports_a_missing_head_as_enable_git_error() {
        let directory = tempfile::tempdir().unwrap();
        let repository = Repository::init(directory.path()).unwrap();
        std::fs::remove_file(repository.path().join("HEAD")).unwrap();

        let error = snapshot_unborn_head(&repository, directory.path()).unwrap_err();

        assert_eq!(error.operation, RepositoryOperation::Enable);
        assert_eq!(error.kind, RepositoryErrorKind::Git);
    }

    #[test]
    fn failed_replacement_after_prepared_parent_removes_owned_empty_directory() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let config = root.join(canonical::CONFIG_PATH);
        let exclude = root.join("exclude");

        assert!(prepare_configuration_parent(root).unwrap());
        std::fs::create_dir(&config).unwrap();
        assert!(replace_bytes_atomically(&config, b"config", root).is_err());
        std::fs::remove_dir(&config).unwrap();

        restore_enablement(
            &config,
            None,
            &exclude,
            None,
            None,
            Some(config.parent().unwrap()),
            None,
        )
        .unwrap();

        assert!(!root.join(".manyhands").exists());
    }
}
