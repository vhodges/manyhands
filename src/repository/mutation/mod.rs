//! The mutation boundary: how a front end's request to change Manyhands
//! state is identified, recorded and answered.
//!
//! This holds what identifies a request and the records that link a
//! request ID to its command, target, input digest, operations and result.
//! The records are not a journal: they hold no step, no lease and no
//! authority over Git. Losing them loses the ability to answer a retry
//! from memory, never the ability to work.
//!
//! This file is the boundary's public surface: the types a front end
//! calls it with, the reads of a request and of a journal row, and what
//! tests inject. The work of a call is in the modules below it:
//!
//! - `execute` looks a request up, decides what the call is, and accepts
//!   and runs a new one;
//! - `confirm` is two-phase confirmation: `prepare`, which gives a
//!   preview and a confirmation, and what `execute` asks of the
//!   confirmation a request presents;
//! - `replay` answers from a record without running anything;
//! - `reenter` takes up an accepted record again;
//! - `settle` finishes, deletes or leaves the record afterwards;
//! - `reply` identifies the request and builds its envelopes;
//! - `progress` is the sink, the token and the credentials a call is
//!   given and how they reach a binding, and `cancel_request`;
//! - `bind` holds what connects each command to all of that.

// `ReadError` carries its whole scope by value, as the result contract has it.
#![allow(clippy::result_large_err)]

use std::sync::Arc;

use time::OffsetDateTime;

use crate::results::{Envelope, ResultCode, Scope, timestamp_string};

use super::{
    OperationFamily, OperationId, ReadError, RepositoryOperation, RepositoryService, keys,
    recovery::{self, JournalRow},
    remote,
};

mod bind;
mod command;
mod confirm;
mod context;
mod dto;
mod evidence;
mod execute;
mod identity;
pub(super) mod local_control;
mod observe;
mod outcome;
mod progress;
mod records;
mod reenter;
mod replay;
mod reply;
mod settle;

/// The synchronization a comment's operation publishes it by, for the
/// conflict read (R-5).
pub(super) use bind::comment::comment_synchronization;
/// The attempt ID `conflict resolve` derives (R-14), for the
/// source-included test target that plants a row under it.
#[cfg(test)]
pub(crate) use bind::conflict::attempt_for as conflict_resolve_attempt;
use bind::{
    Binding, Confirmed,
    abandon::OperationAbandonBinding,
    comment::CommentAddBinding,
    conflict::ConflictResolveBinding,
    document::DocumentBinding,
    folder::FolderCreateBinding,
    host::HostBinding,
    index::IndexBinding,
    key::{KeyDeleteBinding, KeyGenerateBinding, KeyRegistryBinding},
    local_sync::LocalSyncBinding,
    remote::{RemoteAddBinding, RemoteRemoveBinding, RemoteSelectBinding},
    repair::RepairBinding,
    repo::RepoBinding,
    repo_identity::RepoIdentityBinding,
    repo_remove::RepoRemoveBinding,
    resume::OperationResumeBinding,
    sync::SyncBinding,
    ticket::{SlugAssignBinding, TicketBinding},
};
pub use bind::{
    abandon::OperationAbandonInput,
    comment::CommentAddInput,
    conflict::{ConflictResolutionInput, ConflictResolveInput},
    document::{DocumentCreateInput, DocumentMoveInput, DocumentSaveInput},
    folder::FolderCreateInput,
    host::{HostApproveInput, HostReplaceInput},
    index::{IndexRebuildInput, IndexRefreshInput},
    key::{
        KeyClearInput, KeyDeleteInput, KeyGenerateInput, KeyImportInput, KeyRemoveInput,
        KeySelectInput,
    },
    local_sync::{LocalConflictResolutionInput, LocalContextResolveInput, LocalContextSyncInput},
    remote::{RemoteAddInput, RemoteRemoveInput, RemoteSelectInput},
    repair::{DocumentRepairInput, TicketRepairInput},
    repo::{RepoCreateInput, RepoEnableInput},
    repo_identity::RepoIdentitySetInput,
    repo_remove::RepoRemoveInput,
    resume::OperationResumeInput,
    sync::{ItemSyncInput, RepoSyncInput},
    ticket::{TicketCreateInput, TicketSaveInput, TicketSlugAssignInput},
};
use command::Command;
pub use confirm::CONFIRMATION_LIFETIME_SECONDS;
pub use dto::{
    CancelDto, CommentMutationDto, CommentMutationType, CommentPendingReason, ConflictNextAction,
    ConflictNextDto, ConflictResolutionMutationDto, ConflictResolutionMutationType,
    DocumentMutationDto, HostMutationDto, HostObservationDto, HostPreviewDto, HostReplacementDto,
    IdentityMutationDto, IdentityPreviewDto, KeyDeletionPreviewDto, KeyDeletionState,
    KeyGenerationState, KeyMutationDto, LocalIdentityDto, MutationDataDto,
    OperationAbandonPreviewDto, OperationMutationDto, PrepareDataDto, PreviewDetailsDto,
    PreviewDto, PreviewEffect, PreviewEffectDto, PreviewIdentityDto, PreviewIdentitySource,
    RemoteMutationDto, RemoteRemovalPreviewDto, RepairMutationDto, RepairPreviewDto,
    RepositoryPreviewDto, RepositoryRemovalPreviewDto, RequestDto, RequestOperationDto,
    RequestResultDto, RequestState, SynchronizationConflictMutationDto,
    SynchronizationConflictType, SynchronizationMutationDto, TicketMutationDto,
};
use identity::ScopeKey;
pub use identity::{ConfirmationId, ConfirmationIdParseError, RequestId, RequestIdParseError};
pub(crate) use observe::absent_token;
/// For the closed-ticket guard's tests: the code a refused domain write
/// reports.
#[cfg(test)]
pub(super) use outcome::repository_error_code;
pub use progress::{CancellationToken, NoProgress, ProgressEvent, ProgressSink, ProgressStage};
use records::RequestRecord;
pub(super) use records::migrate as migrate_request_tables;
pub(super) use records::{operation_owner, release_operation_owner};

/// What the service reads the time from when it times a record. Tests
/// inject one to move time; everything else uses `SystemClock`.
pub trait Clock: Send + Sync {
    fn now(&self) -> OffsetDateTime;
}

/// The system's clock.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

impl RepositoryService {
    /// The same service, timing its records by `clock`.
    #[doc(hidden)]
    pub fn with_clock_for_testing(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Has `hook` run once in the mutation boundary, immediately before
    /// the next domain call an `execute` on this service makes: after the
    /// request is accepted, or, for a request entered again, after its
    /// journal row and evidence are read.
    #[doc(hidden)]
    pub fn set_request_hook_for_testing(&self, hook: impl FnOnce() + Send + 'static) {
        *self
            .request_hook
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Box::new(hook));
    }

    /// Has `hook` run once in the mutation boundary, immediately before
    /// the next request an `execute` on this service accepts is written:
    /// after its checks, the confirmation's among them, and before the
    /// transaction that records it.
    #[doc(hidden)]
    pub fn set_acceptance_hook_for_testing(&self, hook: impl FnOnce() + Send + 'static) {
        *self
            .acceptance_hook
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Box::new(hook));
    }

    /// Runs the test's hook, when one is set. This is on the path of
    /// every request that is accepted, so a lock that a panicking hook
    /// poisoned is taken all the same: no request fails for it.
    fn run_acceptance_hook(&self) {
        let hook = self
            .acceptance_hook
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(hook) = hook {
            hook();
        }
    }

    /// As `run_acceptance_hook`: this is on the path of every request,
    /// and a poisoned lock is taken all the same.
    fn run_request_hook(&self) {
        let hook = self
            .request_hook
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(hook) = hook {
            hook();
        }
    }

    /// Where the operation `operation_id` stands in the journal of its
    /// family: absent, pending in a state or phase, or final in some way
    /// and owing work or not. `JournalRow::in_flight` is what settling asks.
    ///
    /// A local or remote operation is looked up under the root `scope`
    /// names; a key-material operation belongs to the application,
    /// whatever `scope` is. The lookup reads the index only. It needs no
    /// registration and no repository on disk, so it answers for a root
    /// that is not registered and for one that no longer exists.
    ///
    /// This is what settling a request asks, and not the display read
    /// `show_operation`: a local row closed as `rejected` is absent here,
    /// and a remote operation that was interrupted or that failed is
    /// pending.
    #[doc(hidden)]
    pub fn journal_row(
        &self,
        family: OperationFamily,
        scope: &ScopeKey,
        operation_id: OperationId,
    ) -> Result<JournalRow, ReadError> {
        self.read_session(RepositoryOperation::Read, |connection| match family {
            OperationFamily::Local => {
                recovery::lookup_operation(connection, scope.as_str(), operation_id)
            }
            OperationFamily::Remote => {
                remote::state::lookup_operation(connection, scope.as_str(), operation_id)
            }
            OperationFamily::KeyMaterial => {
                keys::lookup_material_operation(connection, operation_id)
            }
        })
    }
}

impl RepositoryService {
    /// The step a local operation that has completed recorded of its own
    /// work; see `recovery::completed_step`.
    pub(crate) fn completed_local_step(
        &self,
        scope: &ScopeKey,
        operation_id: OperationId,
    ) -> Result<Option<recovery::RecordedStep>, ReadError> {
        self.read_session(RepositoryOperation::Read, |connection| {
            recovery::completed_step(connection, scope.as_str(), operation_id)
        })
    }
}

impl RepositoryService {
    /// The change to the key registry recorded against `operation_id`;
    /// see `keys::recorded_registry_change`.
    pub(crate) fn key_registry_change(
        &self,
        operation_id: OperationId,
    ) -> Result<Option<keys::KeyRegistryChange>, ReadError> {
        self.read_session(RepositoryOperation::Read, |connection| {
            keys::recorded_registry_change(connection, operation_id)
        })
    }
}

impl RepositoryService {
    /// What a synchronization recorded of its push; see
    /// `remote::state::lookup_push`.
    pub(crate) fn stored_push(
        &self,
        scope: &ScopeKey,
        operation_id: OperationId,
    ) -> Result<Option<remote::state::StoredPush>, ReadError> {
        self.read_session(RepositoryOperation::Read, |connection| {
            remote::state::lookup_push(connection, scope.as_str(), operation_id)
        })
    }
}

impl RepositoryService {
    /// Whether a remote operation was begun under a configuration the
    /// registration has since left; see `remote::state::lookup_fenced`.
    pub(crate) fn fenced(
        &self,
        scope: &ScopeKey,
        operation_id: OperationId,
    ) -> Result<bool, ReadError> {
        self.read_session(RepositoryOperation::Read, |connection| {
            remote::state::lookup_fenced(connection, scope.as_str(), operation_id)
        })
    }
}

impl RepositoryService {
    /// What the registration at `root` holds of polling; see
    /// `remote::state::select_polling_policy`. `None` for a root that is
    /// not registered.
    pub(crate) fn polling_policy(
        &self,
        root: &std::path::Path,
    ) -> Result<Option<remote::state::PollingPolicy>, ReadError> {
        self.read_session(RepositoryOperation::Read, |connection| {
            // A root that is not UTF-8 cannot have been stored.
            let Some(root) = root.to_str() else {
                return Ok(None);
            };
            Ok(remote::state::select_polling_policy(connection, root)?)
        })
    }
}

impl RepositoryService {
    /// The key a generation that has completed registered, and the
    /// fingerprint its row recorded; see `keys::completed_generation`.
    pub(crate) fn completed_generation(
        &self,
        operation_id: OperationId,
    ) -> Result<Option<(keys::SharedKeyId, Option<String>)>, ReadError> {
        self.read_session(RepositoryOperation::Read, |connection| {
            keys::completed_generation(connection, operation_id)
        })
    }
}

/// The command `show_request` answers.
const REQUEST_SHOW: &str = "request show";

/// A time a record holds, in seconds. One that cannot be written as a
/// timestamp was not written by any record function.
fn stored_time(seconds: i64) -> Result<String, ReadError> {
    OffsetDateTime::from_unix_timestamp(seconds)
        .ok()
        .and_then(timestamp_string)
        .ok_or_else(|| ReadError::new(ResultCode::InternalError))
}

/// What `request show` reports for a record, and the scope it is about.
fn request_dto(record: RequestRecord) -> Result<(Scope, RequestDto), ReadError> {
    let scope = Scope {
        repository: record.scope.repository().map(str::to_owned),
        ..Scope::default()
    };
    let dto = RequestDto {
        request_id: record.request_id.to_string(),
        state: record.state,
        command: record.command,
        accepted_at: stored_time(record.accepted_at)?,
        finished_at: record.finished_at.map(stored_time).transpose()?,
        operations: record
            .operations
            .into_iter()
            .map(|operation| RequestOperationDto {
                family: operation.family,
                operation_id: operation.operation_id.to_string(),
            })
            .collect(),
        result: record.result.map(|result| RequestResultDto {
            outcome: result.outcome,
            code: result.code,
            message: result.code.message().to_owned(),
            effects: result.effects,
            data: result.data,
        }),
    };
    Ok((scope, dto))
}

impl RepositoryService {
    /// `request show`: what the records hold for a request ID. Its state,
    /// its operation IDs and, once it is finished, its stored result.
    ///
    /// This is the read a client that lost its output uses to inspect
    /// recovery. It reads the records only: nothing is observed, resumed
    /// or changed, and a request whose record was deleted or never
    /// written is `request_not_found` whatever it did. A successful
    /// envelope reports the read; what the request came to is in
    /// `data.result`.
    pub fn show_request(&self, request_id: RequestId) -> Envelope<RequestDto> {
        let shown = self.request_record(request_id).and_then(|record| {
            record
                .map(request_dto)
                .transpose()?
                .ok_or_else(|| ReadError::new(ResultCode::RequestNotFound))
        });
        match shown {
            Ok((scope, dto)) => Envelope::read_success(REQUEST_SHOW, scope, dto),
            Err(error) => error.to_envelope(REQUEST_SHOW),
        }
    }
}

/// One request to change Manyhands state: the caller's request ID, the
/// confirmation it presents, if any, and the typed mutation.
///
/// The confirmation is the request's consent and no part of what
/// identifies it. A request seen for the first time is accepted with the
/// one it presents. From then on the request's record holds it, and a
/// later call of the same request is answered from the record whatever
/// it presents: the same confirmation, another, or none.
pub struct MutationCall {
    pub request_id: RequestId,
    pub confirmation: Option<ConfirmationId>,
    pub mutation: Mutation,
}

/// The mutations a front end can ask for: one variant for each bound
/// command, holding that command's typed input. The variant fixes the
/// envelope's `command`.
pub enum Mutation {
    LocalContextSync(LocalContextSyncInput),
    LocalContextResolve(LocalContextResolveInput),
    CommentAdd(CommentAddInput),
    ConflictResolve(ConflictResolveInput),
    DocumentCreate(DocumentCreateInput),
    DocumentSave(DocumentSaveInput),
    DocumentMove(DocumentMoveInput),
    TicketCreate(TicketCreateInput),
    TicketSave(TicketSaveInput),
    TicketSlugAssign(TicketSlugAssignInput),
    RemoteAdd(RemoteAddInput),
    RemoteSelect(RemoteSelectInput),
    IndexRefresh(IndexRefreshInput),
    IndexRebuild(IndexRebuildInput),
    KeyGenerate(KeyGenerateInput),
    KeyImport(KeyImportInput),
    KeySelect(KeySelectInput),
    KeyClear(KeyClearInput),
    KeyRemove(KeyRemoveInput),
    RepoCreate(RepoCreateInput),
    RepoEnable(RepoEnableInput),
    RemoteRemove(RemoteRemoveInput),
    RepoRemove(RepoRemoveInput),
    KeyDelete(KeyDeleteInput),
    OperationAbandon(OperationAbandonInput),
    ItemSync(ItemSyncInput),
    RepoSync(RepoSyncInput),
    OperationResume(OperationResumeInput),
    RepoIdentitySet(RepoIdentitySetInput),
    HostApprove(HostApproveInput),
    HostReplace(HostReplaceInput),
    FolderCreate(FolderCreateInput),
    DocumentRepair(DocumentRepairInput),
    TicketRepair(TicketRepairInput),
}

impl Mutation {
    /// The command, as the envelope names it.
    pub fn command(&self) -> &'static str {
        self.kind().name()
    }

    fn kind(&self) -> Command {
        match self {
            Self::LocalContextSync(_) => Command::LocalContextSync,
            Self::LocalContextResolve(_) => Command::LocalContextResolve,
            Self::CommentAdd(_) => Command::CommentAdd,
            Self::ConflictResolve(_) => Command::ConflictResolve,
            Self::DocumentCreate(_) => Command::DocumentCreate,
            Self::DocumentSave(_) => Command::DocumentSave,
            Self::DocumentMove(_) => Command::DocumentMove,
            Self::TicketCreate(_) => Command::TicketCreate,
            Self::TicketSave(_) => Command::TicketSave,
            Self::TicketSlugAssign(_) => Command::TicketSlugAssign,
            Self::RemoteAdd(_) => Command::RemoteAdd,
            Self::RemoteSelect(_) => Command::RemoteSelect,
            Self::IndexRefresh(_) => Command::IndexRefresh,
            Self::IndexRebuild(_) => Command::IndexRebuild,
            Self::KeyGenerate(_) => Command::KeyGenerate,
            Self::KeyImport(_) => Command::KeyImport,
            Self::KeySelect(_) => Command::KeySelect,
            Self::KeyClear(_) => Command::KeyClear,
            Self::KeyRemove(_) => Command::KeyRemove,
            Self::RepoCreate(_) => Command::RepoCreate,
            Self::RepoEnable(_) => Command::RepoEnable,
            Self::RemoteRemove(_) => Command::RemoteRemove,
            Self::RepoRemove(_) => Command::RepoRemove,
            Self::KeyDelete(_) => Command::KeyDelete,
            Self::OperationAbandon(_) => Command::OperationAbandon,
            Self::ItemSync(_) => Command::ItemSync,
            Self::RepoSync(_) => Command::RepoSync,
            Self::OperationResume(_) => Command::OperationResume,
            Self::RepoIdentitySet(_) => Command::RepoIdentitySet,
            Self::HostApprove(_) => Command::HostApprove,
            Self::HostReplace(_) => Command::HostReplace,
            Self::FolderCreate(_) => Command::FolderCreate,
            Self::DocumentRepair(_) => Command::DocumentRepair,
            Self::TicketRepair(_) => Command::TicketRepair,
        }
    }

    /// The binding `prepare` previews the mutation through, for a
    /// command that needs confirmation. `None` for every other command:
    /// there is nothing to preview, and nothing is read of its input.
    fn confirmed(&self) -> Option<Box<dyn Confirmed>> {
        match self {
            Self::LocalContextSync(_)
            | Self::LocalContextResolve(_)
            | Self::CommentAdd(_)
            | Self::ConflictResolve(_) => None,
            Self::RepoCreate(input) => Some(Box::new(RepoBinding::from(input.clone()))),
            Self::RepoEnable(input) => Some(Box::new(RepoBinding::from(input.clone()))),
            Self::RemoteRemove(input) => Some(Box::new(RemoteRemoveBinding::from(input.clone()))),
            Self::RepoRemove(input) => Some(Box::new(RepoRemoveBinding::from(input.clone()))),
            Self::KeyDelete(input) => Some(Box::new(KeyDeleteBinding::from(input.clone()))),
            Self::OperationAbandon(input) => {
                Some(Box::new(OperationAbandonBinding::from(input.clone())))
            }
            Self::RepoIdentitySet(input) => {
                Some(Box::new(RepoIdentityBinding::from(input.clone())))
            }
            Self::HostApprove(input) => Some(Box::new(HostBinding::from(input.clone()))),
            Self::HostReplace(input) => Some(Box::new(HostBinding::from(input.clone()))),
            Self::DocumentRepair(input) => Some(Box::new(RepairBinding::from(input.clone()))),
            Self::TicketRepair(input) => Some(Box::new(RepairBinding::from(input.clone()))),
            Self::DocumentCreate(_)
            | Self::DocumentSave(_)
            | Self::DocumentMove(_)
            | Self::TicketCreate(_)
            | Self::TicketSave(_)
            | Self::TicketSlugAssign(_)
            | Self::RemoteAdd(_)
            | Self::RemoteSelect(_)
            | Self::IndexRefresh(_)
            | Self::IndexRebuild(_)
            | Self::KeyGenerate(_)
            | Self::KeyImport(_)
            | Self::KeySelect(_)
            | Self::KeyClear(_)
            | Self::KeyRemove(_)
            | Self::ItemSync(_)
            | Self::RepoSync(_)
            | Self::OperationResume(_)
            | Self::FolderCreate(_) => None,
        }
    }

    fn into_binding(self) -> Box<dyn Binding> {
        match self {
            Self::LocalContextSync(input) => Box::new(LocalSyncBinding::Start(input)),
            Self::LocalContextResolve(input) => Box::new(LocalSyncBinding::Resolve(input)),
            Self::CommentAdd(input) => Box::new(CommentAddBinding::from(input)),
            Self::ConflictResolve(input) => Box::new(ConflictResolveBinding::from(input)),
            Self::DocumentCreate(input) => Box::new(DocumentBinding::from(input)),
            Self::DocumentSave(input) => Box::new(DocumentBinding::from(input)),
            Self::DocumentMove(input) => Box::new(DocumentBinding::from(input)),
            Self::TicketCreate(input) => Box::new(TicketBinding::from(input)),
            Self::TicketSave(input) => Box::new(TicketBinding::from(input)),
            Self::TicketSlugAssign(input) => Box::new(SlugAssignBinding::from(input)),
            Self::RemoteAdd(input) => Box::new(RemoteAddBinding::from(input)),
            Self::RemoteSelect(input) => Box::new(RemoteSelectBinding::from(input)),
            Self::IndexRefresh(input) => Box::new(IndexBinding::from(input)),
            Self::IndexRebuild(input) => Box::new(IndexBinding::from(input)),
            Self::KeyGenerate(input) => Box::new(KeyGenerateBinding::from(input)),
            Self::KeyImport(input) => Box::new(KeyRegistryBinding::from(input)),
            Self::KeySelect(input) => Box::new(KeyRegistryBinding::from(input)),
            Self::KeyClear(input) => Box::new(KeyRegistryBinding::from(input)),
            Self::KeyRemove(input) => Box::new(KeyRegistryBinding::from(input)),
            Self::RepoCreate(input) => Box::new(RepoBinding::from(input)),
            Self::RepoEnable(input) => Box::new(RepoBinding::from(input)),
            Self::RemoteRemove(input) => Box::new(RemoteRemoveBinding::from(input)),
            Self::RepoRemove(input) => Box::new(RepoRemoveBinding::from(input)),
            Self::KeyDelete(input) => Box::new(KeyDeleteBinding::from(input)),
            Self::OperationAbandon(input) => Box::new(OperationAbandonBinding::from(input)),
            Self::ItemSync(input) => Box::new(SyncBinding::from(input)),
            Self::RepoSync(input) => Box::new(SyncBinding::from(input)),
            Self::OperationResume(input) => Box::new(OperationResumeBinding::from(input)),
            Self::RepoIdentitySet(input) => Box::new(RepoIdentityBinding::from(input)),
            Self::HostApprove(input) => Box::new(HostBinding::from(input)),
            Self::HostReplace(input) => Box::new(HostBinding::from(input)),
            Self::FolderCreate(input) => Box::new(FolderCreateBinding::from(input)),
            Self::DocumentRepair(input) => Box::new(RepairBinding::from(input)),
            Self::TicketRepair(input) => Box::new(RepairBinding::from(input)),
        }
    }
}

/// What the boundary is built from: the intent digest, the request records
/// and the journal lookup. A front end uses the boundary itself; this is
/// public so that the integration tests can reach it.
#[doc(hidden)]
pub mod request_store {
    /// The published name of every command the boundary binds.
    pub fn bound_commands() -> Vec<&'static str> {
        super::Command::ALL
            .into_iter()
            .map(super::Command::name)
            .collect()
    }

    pub use super::{
        Clock, SystemClock,
        identity::{DigestSalt, FieldValue, IntentDigest, IntentDigestBuilder, ScopeKey},
        records::{
            ConfirmationRecord, InsertRequestOutcome, NewConfirmation, NewRequest,
            RequestOperation, RequestRecord, RequestResult,
        },
        recovery::{FinalKind, JournalRow, LocalStep, PendingOperation, RecordedStep},
    };
}
