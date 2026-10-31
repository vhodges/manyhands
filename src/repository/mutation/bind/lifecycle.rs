//! `document promote` and `ticket close`: one binding over the domain's one
//! lifecycle runner, generic over a small verb (R-1).
//!
//! **The preview** is the domain's: it contacts no remote and takes no lease.
//! `Ready` gives a confirmation whose observation digest binds the domain's
//! expectation, which binds everything the preview shows. `Blocked` gives no
//! confirmation and answers with every blocker. `Binding::prepare` observes
//! the same way and checks nothing, so a request whose state has moved since
//! its preview is `external_change` with no record.
//!
//! **A run** calls the domain under the request's operation ID, with no host
//! approval and no confirmed identity (decision 6). What is left of the
//! request is read from the parent record, not the journal, because a parent
//! usually has no journal row (fact 29): no parent, the request is deleted and
//! a new prepare follows; a parent a consent refusal ended or a complete or
//! superseded one, the result is stored; an unfinished one, the request stays
//! accepted, and never `Stopped{Absent}`.
//!
//! **Entered again,** `Confirmed::row` reads the same record (R-19). An
//! unfinished or complete parent is called again with the record's own
//! expectation and nothing compared, since the request's own work moved the
//! state. A parent that ended answers from what it holds. With no parent, what
//! is there is compared with what was previewed.
//!
//! **Left for later tasks.** Task 2c connects cancellation, Task 3 `operation
//! resume`, Task 4 the made-ready report, and Task 6 the promotion identity
//! row.

use std::{marker::PhantomData, path::Path, path::PathBuf};

use crate::{
    canonical::ItemId,
    results::{
        CheckpointEffect, CleanupEffect, DiscoveryEffect, Effects, IntegrationEffect,
        PublicationEffect, RecoveryAction, ResultCode, Scope, WriteEffect,
    },
};

use super::{
    super::{
        super::{
            AuthoringKind, CloseTicketRequest, ClosureBlocker, ClosureExpectation,
            ClosureRecording, ConfigurationInspection, DocumentPromotionError,
            DocumentPromotionOutcome, DocumentPromotionPreflight, DocumentPromotionPreview,
            DocumentPromotionResult, ItemDtoKind, OperationFamily, OperationId, PathChange,
            PlannedCleanup, PlannedClosure, PlannedIntegration, PlannedPublication,
            PreviewDocumentPromotionRequest, PreviewTicketClosureRequest, PromoteDocumentRequest,
            PromotionBlocker, PromotionCleanup, PromotionCleanupPending, PromotionExpectation,
            PromotionIntegration, PromotionPathChange, PromotionPublication,
            PromotionPublicationPending, PromotionRemoteDeletionCategory,
            PromotionSynchronizationCategory, RepositoryError, RepositoryOperation,
            RepositoryService, SynchronizationError, TicketClosureError, TicketClosureOutcome,
            TicketClosurePreflight, TicketClosurePreview, TicketClosureResult, closure, promotion,
            promotion::{
                ParentView,
                lifecycle::{LifecyclePreview, ParentStep},
                lifecycle_record::Refusal,
                record::PublicationCategory,
            },
            read_configuration_for,
            recovery::JournalRow,
            transport::SshTransportError,
        },
        command::Command,
        confirm::{self, Row, Seen},
        context::context_names,
        dto::{
            LifecycleBlockedDto, LifecycleBlocker, LifecycleCleanupPendingReason,
            LifecycleCleanupState, LifecycleIntegrationKind, LifecycleMutationDto,
            LifecycleMutationType, LifecyclePathChangeDto, LifecyclePathChangeKind,
            LifecyclePlannedCleanupDto, LifecyclePlannedClosureDto, LifecyclePlannedIntegration,
            LifecyclePlannedPublication, LifecyclePreviewDto, LifecyclePublicationPendingReason,
            LifecyclePublicationState, LifecycleRecording, MutationDataDto, PreviewDetailsDto,
            PreviewEffect,
        },
        evidence::{self, Position},
        identity::{IntentDigestBuilder, ScopeKey},
        observe::{Observation, ObservationDigest},
        outcome,
        progress::Session,
    },
    Accepted, Answer, Binding, CallContext, Confirmed, EndedWithoutCheckpoint, Entered, Place,
    Prepared, Previewed, Ran, Standing, refused, resolve_root,
};

/// The input of `document promote`: the document.
#[derive(Clone)]
pub struct DocumentPromoteInput {
    pub root: PathBuf,
    pub id: ItemId,
}

/// The input of `ticket close`: the ticket.
#[derive(Clone)]
pub struct TicketCloseInput {
    pub root: PathBuf,
    pub id: ItemId,
}

/// What the preview of either verb shows and consents to.
pub(crate) struct Ready {
    expected: [u8; 32],
    branch: String,
    primary_branch: String,
    remote: Option<String>,
    context_oid: git2::Oid,
    primary_oid: git2::Oid,
    /// A context child runs before integration, by the verb's rule.
    context_child: bool,
    changes: Vec<PromotionPathChange>,
    integration: PlannedIntegration,
    publication: PlannedPublication,
    cleanup: PlannedCleanup,
    closure: Option<PlannedClosure>,
}

pub(crate) enum Observed {
    Ready(Box<Ready>),
    Blocked(Vec<LifecycleBlocker>),
}

/// How far an operation that reached primary got, of either verb.
pub(crate) struct Reached {
    recording: Option<ClosureRecording>,
    integration: PromotionIntegration,
    publication: PromotionPublication,
    cleanup: PromotionCleanup,
    index_pending: bool,
}

/// What stopped an operation before it could report that, of either verb.
pub(crate) enum Stop {
    Repository(RepositoryError),
    Transport(Box<SshTransportError>),
    Blocked(Vec<LifecycleBlocker>),
    ExternalChange,
    ReconfirmationRequired,
    PrimaryAdvanced,
    IdentityRequired,
    ContextSynchronization(Box<SynchronizationError>),
    Busy,
    PollYielding,
    Interrupted,
    OperationMismatch,
    RecoveryRequired,
}

/// What is the verb's own: its command, its preview, its one domain call and
/// its parent table.
pub(crate) trait LifecycleVerb {
    const COMMAND: Command;
    const KIND: AuthoringKind;
    const DTO_KIND: ItemDtoKind;

    fn observe(
        service: &RepositoryService,
        root: &Path,
        id: &ItemId,
    ) -> Result<Observed, RepositoryError>;

    /// `None` when the session has no credentials to call it with.
    fn call(
        session: &dyn Session,
        service: &RepositoryService,
        root: PathBuf,
        operation_id: OperationId,
        item_id: ItemId,
        expected: [u8; 32],
    ) -> Option<Result<Reached, Stop>>;

    fn parent(
        service: &RepositoryService,
        root: &Path,
        operation: OperationId,
    ) -> Result<Option<ParentView<LifecycleBlocker>>, RepositoryError>;
}

pub(crate) struct Promote;
pub(crate) struct Close;

fn promotion_blocker(blocker: PromotionBlocker) -> LifecycleBlocker {
    match blocker {
        PromotionBlocker::NotADocument => LifecycleBlocker::NotADocument,
        PromotionBlocker::ContextMissing => LifecycleBlocker::ContextMissing,
        PromotionBlocker::ContextNonconforming => LifecycleBlocker::ContextNonconforming,
        PromotionBlocker::ContextDirty => LifecycleBlocker::ContextDirty,
        PromotionBlocker::ContextConflicted => LifecycleBlocker::ContextConflicted,
        PromotionBlocker::ConflictPending => LifecycleBlocker::ConflictPending,
        PromotionBlocker::PrimaryMissing => LifecycleBlocker::PrimaryMissing,
        PromotionBlocker::PrimaryNotCheckedOut => LifecycleBlocker::PrimaryNotCheckedOut,
        PromotionBlocker::PrimaryDirty => LifecycleBlocker::PrimaryDirty,
        PromotionBlocker::PrimaryConflicted => LifecycleBlocker::PrimaryConflicted,
        PromotionBlocker::PrimaryAheadOfRemote => LifecycleBlocker::PrimaryAheadOfRemote,
        PromotionBlocker::ContextBehindRemote => LifecycleBlocker::ContextBehindRemote,
        PromotionBlocker::IntegrationConflict => LifecycleBlocker::IntegrationConflict,
        PromotionBlocker::RemoteContextDeleted => LifecycleBlocker::RemoteContextDeleted,
        PromotionBlocker::HistoryUnknown => LifecycleBlocker::HistoryUnknown,
        PromotionBlocker::DocumentInvalidAfterIntegration => {
            LifecycleBlocker::DocumentInvalidAfterIntegration
        }
    }
}

fn closure_blocker(blocker: ClosureBlocker) -> LifecycleBlocker {
    match blocker {
        ClosureBlocker::NotATicket => LifecycleBlocker::NotATicket,
        ClosureBlocker::ContextMissing => LifecycleBlocker::ContextMissing,
        ClosureBlocker::ContextNonconforming => LifecycleBlocker::ContextNonconforming,
        ClosureBlocker::ContextDirty => LifecycleBlocker::ContextDirty,
        ClosureBlocker::ContextConflicted => LifecycleBlocker::ContextConflicted,
        ClosureBlocker::ConflictPending => LifecycleBlocker::ConflictPending,
        ClosureBlocker::LocalOperationPending => LifecycleBlocker::LocalOperationPending,
        ClosureBlocker::CommentPublicationPending => LifecycleBlocker::CommentPublicationPending,
        ClosureBlocker::PrimaryMissing => LifecycleBlocker::PrimaryMissing,
        ClosureBlocker::PrimaryNotCheckedOut => LifecycleBlocker::PrimaryNotCheckedOut,
        ClosureBlocker::PrimaryDirty => LifecycleBlocker::PrimaryDirty,
        ClosureBlocker::PrimaryConflicted => LifecycleBlocker::PrimaryConflicted,
        ClosureBlocker::PrimaryAheadOfRemote => LifecycleBlocker::PrimaryAheadOfRemote,
        ClosureBlocker::ContextBehindRemote => LifecycleBlocker::ContextBehindRemote,
        ClosureBlocker::IntegrationConflict => LifecycleBlocker::IntegrationConflict,
        ClosureBlocker::RemoteContextDeleted => LifecycleBlocker::RemoteContextDeleted,
        ClosureBlocker::HistoryUnknown => LifecycleBlocker::HistoryUnknown,
        ClosureBlocker::AlreadyClosed => LifecycleBlocker::AlreadyClosed,
        ClosureBlocker::PrimaryReopened => LifecycleBlocker::PrimaryReopened,
        ClosureBlocker::TicketInvalidAfterClosure => LifecycleBlocker::TicketInvalidAfterClosure,
        ClosureBlocker::IdentityRequired => LifecycleBlocker::IdentityRequired,
    }
}

fn ready_of_promotion(preview: &DocumentPromotionPreview) -> Ready {
    Ready {
        expected: preview.expectation().to_bytes(),
        branch: preview.context_branch().to_owned(),
        primary_branch: preview.primary_branch().to_owned(),
        remote: preview.publication_remote().map(str::to_owned),
        context_oid: preview.context_oid(),
        primary_oid: preview.primary_oid(),
        // The runner's own rule, with the remote it asks for beside it.
        context_child: preview.publication_remote().is_some()
            && LifecyclePreview::runs_context_child(preview),
        changes: preview.changes().to_vec(),
        integration: preview.integration(),
        publication: preview.publication(),
        cleanup: preview.cleanup(),
        closure: None,
    }
}

fn ready_of_closure(preview: &TicketClosurePreview) -> Ready {
    Ready {
        expected: preview.expectation().to_bytes(),
        branch: preview.context_branch().to_owned(),
        primary_branch: preview.primary_branch().to_owned(),
        remote: preview.publication_remote().map(str::to_owned),
        context_oid: preview.context_oid(),
        primary_oid: preview.primary_oid(),
        // As `ClosureObservation::needs_context_child`: a contained context
        // is never synchronized, and the checkpoint is never pushed.
        context_child: preview.publication_remote().is_some()
            && preview.integration() != PlannedIntegration::AlreadyContained,
        changes: preview.changes().to_vec(),
        integration: preview.integration(),
        publication: preview.publication(),
        cleanup: preview.cleanup(),
        closure: Some(preview.closure().clone()),
    }
}

fn promotion_reached(outcome: &DocumentPromotionOutcome, index_pending: bool) -> Reached {
    Reached {
        recording: None,
        integration: outcome.integration(),
        publication: outcome.publication(),
        cleanup: outcome.cleanup(),
        index_pending,
    }
}

fn closure_reached(outcome: &TicketClosureOutcome, index_pending: bool) -> Reached {
    Reached {
        recording: Some(outcome.closure()),
        integration: outcome.integration(),
        publication: outcome.publication(),
        cleanup: outcome.cleanup(),
        index_pending,
    }
}

impl LifecycleVerb for Promote {
    const COMMAND: Command = Command::DocumentPromote;
    const KIND: AuthoringKind = AuthoringKind::Document;
    const DTO_KIND: ItemDtoKind = ItemDtoKind::Document;

    fn observe(
        service: &RepositoryService,
        root: &Path,
        id: &ItemId,
    ) -> Result<Observed, RepositoryError> {
        Ok(
            match service.preview_document_promotion(PreviewDocumentPromotionRequest {
                root: root.to_owned(),
                item_id: id.clone(),
            })? {
                DocumentPromotionPreflight::Ready(preview) => {
                    Observed::Ready(Box::new(ready_of_promotion(&preview)))
                }
                DocumentPromotionPreflight::Blocked { blockers } => {
                    Observed::Blocked(blockers.into_iter().map(promotion_blocker).collect())
                }
            },
        )
    }

    fn call(
        session: &dyn Session,
        service: &RepositoryService,
        root: PathBuf,
        operation_id: OperationId,
        item_id: ItemId,
        expected: [u8; 32],
    ) -> Option<Result<Reached, Stop>> {
        let result = session.promote_document(
            service,
            PromoteDocumentRequest {
                root,
                operation_id,
                item_id,
                expected: PromotionExpectation::from_bytes(expected),
                approval: None,
                confirmed_identity: None,
            },
        )?;
        Some(match result {
            Ok(DocumentPromotionResult::Complete(outcome)) => {
                Ok(promotion_reached(&outcome, false))
            }
            Ok(DocumentPromotionResult::IndexPending(pending)) => {
                Ok(promotion_reached(&pending.authoritative, true))
            }
            Err(error) => Err(match error {
                DocumentPromotionError::Repository(error) => Stop::Repository(error),
                DocumentPromotionError::Transport(error) => Stop::Transport(error),
                DocumentPromotionError::Blocked(blockers) => {
                    Stop::Blocked(blockers.into_iter().map(promotion_blocker).collect())
                }
                DocumentPromotionError::ExternalChange => Stop::ExternalChange,
                DocumentPromotionError::ReconfirmationRequired => Stop::ReconfirmationRequired,
                DocumentPromotionError::PrimaryAdvanced => Stop::PrimaryAdvanced,
                DocumentPromotionError::IdentityRequired {
                    expected_configuration: _,
                } => Stop::IdentityRequired,
                DocumentPromotionError::ContextSynchronization(error) => {
                    Stop::ContextSynchronization(error)
                }
                DocumentPromotionError::Busy => Stop::Busy,
                DocumentPromotionError::PollYielding => Stop::PollYielding,
                DocumentPromotionError::Interrupted => Stop::Interrupted,
                DocumentPromotionError::OperationMismatch => Stop::OperationMismatch,
                DocumentPromotionError::RecoveryRequired => Stop::RecoveryRequired,
            }),
        })
    }

    fn parent(
        service: &RepositoryService,
        root: &Path,
        operation: OperationId,
    ) -> Result<Option<ParentView<LifecycleBlocker>>, RepositoryError> {
        Ok(promotion::lifecycle_parent(service, root, operation)?
            .map(|parent| parent.of_blockers(promotion_blocker)))
    }
}

impl LifecycleVerb for Close {
    const COMMAND: Command = Command::TicketClose;
    const KIND: AuthoringKind = AuthoringKind::Ticket;
    const DTO_KIND: ItemDtoKind = ItemDtoKind::Ticket;

    fn observe(
        service: &RepositoryService,
        root: &Path,
        id: &ItemId,
    ) -> Result<Observed, RepositoryError> {
        Ok(
            match service.preview_ticket_closure(PreviewTicketClosureRequest {
                root: root.to_owned(),
                item_id: id.clone(),
                confirmed_identity: None,
            })? {
                TicketClosurePreflight::Ready(preview) => {
                    Observed::Ready(Box::new(ready_of_closure(&preview)))
                }
                TicketClosurePreflight::Blocked {
                    blockers,
                    identity_configuration: _,
                } => Observed::Blocked(blockers.into_iter().map(closure_blocker).collect()),
            },
        )
    }

    fn call(
        session: &dyn Session,
        service: &RepositoryService,
        root: PathBuf,
        operation_id: OperationId,
        item_id: ItemId,
        expected: [u8; 32],
    ) -> Option<Result<Reached, Stop>> {
        let result = session.close_ticket(
            service,
            CloseTicketRequest {
                root,
                operation_id,
                item_id,
                expected: ClosureExpectation::from_bytes(expected),
                approval: None,
                confirmed_identity: None,
            },
        )?;
        Some(match result {
            Ok(TicketClosureResult::Complete(outcome)) => Ok(closure_reached(&outcome, false)),
            Ok(TicketClosureResult::IndexPending(pending)) => {
                Ok(closure_reached(&pending.authoritative, true))
            }
            Err(error) => Err(match error {
                TicketClosureError::Repository(error) => Stop::Repository(error),
                TicketClosureError::Transport(error) => Stop::Transport(error),
                TicketClosureError::Blocked(blockers) => {
                    Stop::Blocked(blockers.into_iter().map(closure_blocker).collect())
                }
                TicketClosureError::ExternalChange => Stop::ExternalChange,
                TicketClosureError::ReconfirmationRequired => Stop::ReconfirmationRequired,
                TicketClosureError::PrimaryAdvanced => Stop::PrimaryAdvanced,
                TicketClosureError::IdentityRequired {
                    expected_configuration: _,
                } => Stop::IdentityRequired,
                TicketClosureError::ContextSynchronization(error) => {
                    Stop::ContextSynchronization(error)
                }
                TicketClosureError::Busy => Stop::Busy,
                TicketClosureError::PollYielding => Stop::PollYielding,
                TicketClosureError::Interrupted => Stop::Interrupted,
                TicketClosureError::OperationMismatch => Stop::OperationMismatch,
                TicketClosureError::RecoveryRequired => Stop::RecoveryRequired,
            }),
        })
    }

    fn parent(
        service: &RepositoryService,
        root: &Path,
        operation: OperationId,
    ) -> Result<Option<ParentView<LifecycleBlocker>>, RepositoryError> {
        Ok(closure::lifecycle_parent(service, root, operation)?
            .map(|parent| parent.of_blockers(closure_blocker)))
    }
}

pub(crate) struct LifecycleBinding<V> {
    root: PathBuf,
    id: ItemId,
    /// The expectation the request consents to, as `Binding::prepare`
    /// observed it. `None` for a state that is blocked now, which the
    /// boundary refuses before anything runs.
    expected: Option<[u8; 32]>,
    verb: PhantomData<V>,
}

impl From<DocumentPromoteInput> for LifecycleBinding<Promote> {
    fn from(input: DocumentPromoteInput) -> Self {
        let DocumentPromoteInput { root, id } = input;
        Self {
            root,
            id,
            expected: None,
            verb: PhantomData,
        }
    }
}

impl From<TicketCloseInput> for LifecycleBinding<Close> {
    fn from(input: TicketCloseInput) -> Self {
        let TicketCloseInput { root, id } = input;
        Self {
            root,
            id,
            expected: None,
            verb: PhantomData,
        }
    }
}

/// The digest a confirmation stores. The domain's expectation binds all the
/// preview shows, `closed_by` among it, so this holds a hash of a hash.
fn digest_of(command: Command, observed: &Observed) -> ObservationDigest {
    let name = command.name();
    let state = match observed {
        Observed::Ready(ready) => {
            let expectation: String = ready
                .expected
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            Observation::of(name).text("expectation", &expectation)
        }
        Observed::Blocked(blockers) => {
            let names: Vec<&str> = blockers.iter().map(|blocker| blocker.as_str()).collect();
            Observation::of(name).text("blockers", &names.join(","))
        }
    };
    ObservationDigest::of_state(name, state.finish())
}

/// The answer to what stands in the way of a verb, at a prepare or at a run.
/// The blockers are the caller's to read, whatever the code.
fn blocked_code(blockers: &[LifecycleBlocker]) -> ResultCode {
    match blockers {
        [LifecycleBlocker::IdentityRequired] => ResultCode::IdentityRequired,
        [LifecycleBlocker::AlreadyClosed] => ResultCode::TicketClosed,
        _ => ResultCode::RecoveryRequired,
    }
}

/// What a preview of the item shows of its changes, relative to the repository.
fn path_change(change: &PromotionPathChange) -> LifecyclePathChangeDto {
    LifecyclePathChangeDto {
        path: change.path().to_string_lossy().replace('\\', "/"),
        change: match change.change() {
            PathChange::Added => LifecyclePathChangeKind::Added,
            PathChange::Modified => LifecyclePathChangeKind::Modified,
            PathChange::Deleted => LifecyclePathChangeKind::Deleted,
        },
    }
}

fn publication_pending(reason: PromotionPublicationPending) -> LifecyclePublicationPendingReason {
    match reason {
        PromotionPublicationPending::NoPublicationRemote => {
            LifecyclePublicationPendingReason::NoPublicationRemote
        }
        PromotionPublicationPending::Cancelled => LifecyclePublicationPendingReason::Cancelled,
        PromotionPublicationPending::Synchronization(category) => match category {
            PromotionSynchronizationCategory::Conflict => {
                LifecyclePublicationPendingReason::Conflict
            }
            PromotionSynchronizationCategory::Rejected => {
                LifecyclePublicationPendingReason::Rejected
            }
            PromotionSynchronizationCategory::Transport => {
                LifecyclePublicationPendingReason::Transport
            }
            PromotionSynchronizationCategory::Identity => {
                LifecyclePublicationPendingReason::Identity
            }
            PromotionSynchronizationCategory::Busy => LifecyclePublicationPendingReason::Busy,
            PromotionSynchronizationCategory::Yielding => {
                LifecyclePublicationPendingReason::Yielding
            }
            PromotionSynchronizationCategory::RecoveryRequired => {
                LifecyclePublicationPendingReason::RecoveryRequired
            }
        },
    }
}

fn cleanup_pending(reason: PromotionCleanupPending) -> LifecycleCleanupPendingReason {
    match reason {
        PromotionCleanupPending::AwaitingPublication => {
            LifecycleCleanupPendingReason::AwaitingPublication
        }
        PromotionCleanupPending::ContextChanged => LifecycleCleanupPendingReason::ContextChanged,
        PromotionCleanupPending::IgnoredFiles => LifecycleCleanupPendingReason::IgnoredFiles,
        PromotionCleanupPending::RemoteContextChanged => {
            LifecycleCleanupPendingReason::RemoteContextChanged
        }
        PromotionCleanupPending::RemoteDeletionFailed(category) => match category {
            PromotionRemoteDeletionCategory::Transport => {
                LifecycleCleanupPendingReason::RemoteDeletionTransport
            }
            PromotionRemoteDeletionCategory::Rejected => {
                LifecycleCleanupPendingReason::RemoteDeletionRejected
            }
            PromotionRemoteDeletionCategory::Configuration => {
                LifecycleCleanupPendingReason::RemoteDeletionConfiguration
            }
            PromotionRemoteDeletionCategory::Key => {
                LifecycleCleanupPendingReason::RemoteDeletionKey
            }
            PromotionRemoteDeletionCategory::Unlock => {
                LifecycleCleanupPendingReason::RemoteDeletionUnlock
            }
            PromotionRemoteDeletionCategory::Host => {
                LifecycleCleanupPendingReason::RemoteDeletionHost
            }
        },
        PromotionCleanupPending::RecoveryRequired => {
            LifecycleCleanupPendingReason::RecoveryRequired
        }
        PromotionCleanupPending::Cancelled => LifecycleCleanupPendingReason::Cancelled,
    }
}

/// The design's categories, for a publication that stopped.
fn publication_code(category: PromotionSynchronizationCategory) -> ResultCode {
    match category {
        PromotionSynchronizationCategory::Conflict
        | PromotionSynchronizationCategory::RecoveryRequired => ResultCode::RecoveryRequired,
        PromotionSynchronizationCategory::Rejected => ResultCode::PushRejected,
        PromotionSynchronizationCategory::Transport => ResultCode::TransportUnavailable,
        PromotionSynchronizationCategory::Identity => ResultCode::IdentityRequired,
        PromotionSynchronizationCategory::Busy => ResultCode::Busy,
        PromotionSynchronizationCategory::Yielding => ResultCode::PollYielding,
    }
}

/// The design's categories, for a remote branch that could not be deleted.
fn deletion_code(category: PromotionRemoteDeletionCategory) -> ResultCode {
    match category {
        PromotionRemoteDeletionCategory::Transport => ResultCode::TransportUnavailable,
        PromotionRemoteDeletionCategory::Rejected => ResultCode::PushRejected,
        PromotionRemoteDeletionCategory::Configuration => ResultCode::RecoveryRequired,
        PromotionRemoteDeletionCategory::Key => ResultCode::NoSelectedKey,
        PromotionRemoteDeletionCategory::Unlock => ResultCode::UnlockRequired,
        PromotionRemoteDeletionCategory::Host => ResultCode::HostApprovalRequired,
    }
}

/// The design's answers table, for an operation that reached primary. The
/// publication's stop comes first, since a cleanup awaiting it is its result.
fn reached_code(reached: &Reached) -> ResultCode {
    match (reached.publication, reached.cleanup) {
        (
            PromotionPublication::Pending {
                reason: PromotionPublicationPending::Cancelled,
            },
            _,
        )
        | (
            _,
            PromotionCleanup::Pending {
                reason: PromotionCleanupPending::Cancelled,
            },
        ) => ResultCode::Cancelled,
        (
            PromotionPublication::Pending {
                reason: PromotionPublicationPending::Synchronization(category),
            },
            _,
        ) => publication_code(category),
        (
            _,
            PromotionCleanup::Pending {
                reason: PromotionCleanupPending::RemoteDeletionFailed(category),
            },
        ) => deletion_code(category),
        (
            _,
            PromotionCleanup::Pending {
                reason:
                    PromotionCleanupPending::AwaitingPublication
                    | PromotionCleanupPending::ContextChanged
                    | PromotionCleanupPending::IgnoredFiles
                    | PromotionCleanupPending::RemoteContextChanged
                    | PromotionCleanupPending::RecoveryRequired,
            },
        ) => ResultCode::RecoveryRequired,
        (
            PromotionPublication::Published { oid: _ }
            | PromotionPublication::AlreadyCurrent { oid: _ }
            | PromotionPublication::Pending {
                reason: PromotionPublicationPending::NoPublicationRemote,
            },
            PromotionCleanup::Complete,
        ) => {
            if reached.index_pending {
                ResultCode::DiscoveryPending
            } else {
                ResultCode::Ok
            }
        }
    }
}

fn reached_effects(reached: &Reached) -> Effects {
    let integrated = match reached.integration {
        PromotionIntegration::Merged { oid } | PromotionIntegration::AlreadyContained { oid } => {
            oid
        }
    };
    Effects {
        write: WriteEffect::NotRequested,
        checkpoint: match reached.recording {
            Some(ClosureRecording::Recorded { checkpoint_oid: _ }) => CheckpointEffect::Committed,
            Some(ClosureRecording::Existing) | None => CheckpointEffect::NotRequested,
        },
        discovery: if reached.index_pending {
            DiscoveryEffect::Pending
        } else {
            DiscoveryEffect::Current
        },
        publication: match reached.publication {
            PromotionPublication::Published { oid: _ } => PublicationEffect::Published,
            PromotionPublication::AlreadyCurrent { oid: _ } => PublicationEffect::Current,
            PromotionPublication::Pending { reason: _ } => PublicationEffect::Pending,
        },
        integration: IntegrationEffect::Complete,
        cleanup: match reached.cleanup {
            PromotionCleanup::Complete => CleanupEffect::Complete,
            PromotionCleanup::Pending { reason: _ } => CleanupEffect::Pending,
        },
        commit_oid: Some(integrated.to_string()),
    }
}

impl<V: LifecycleVerb> LifecycleBinding<V> {
    /// Reads the repository and what the domain's preview says of the item.
    fn observe(&self, service: &RepositoryService) -> Result<(PathBuf, Observed), Answer> {
        let repository = resolve_root(service, &self.root)?;
        let root = repository.root().to_owned();
        let observed = V::observe(service, &root, &self.id).map_err(refused)?;
        Ok((root, observed))
    }

    fn data(&self) -> LifecycleMutationDto {
        LifecycleMutationDto {
            r#type: LifecycleMutationType::Lifecycle,
            kind: V::DTO_KIND,
            item_id: self.id.to_string(),
            recording: None,
            checkpoint_oid: None,
            integration: None,
            integration_oid: None,
            publication: None,
            publication_oid: None,
            publication_pending: None,
            cleanup: None,
            cleanup_pending: None,
            blockers: Vec::new(),
        }
    }

    fn reached_data(&self, reached: &Reached) -> LifecycleMutationDto {
        let (recording, checkpoint_oid) = match reached.recording {
            Some(ClosureRecording::Recorded { checkpoint_oid }) => {
                (Some(LifecycleRecording::Record), Some(checkpoint_oid))
            }
            Some(ClosureRecording::Existing) => (Some(LifecycleRecording::Existing), None),
            None => (None, None),
        };
        let (integration, integration_oid) = match reached.integration {
            PromotionIntegration::Merged { oid } => (LifecycleIntegrationKind::Merged, oid),
            PromotionIntegration::AlreadyContained { oid } => {
                (LifecycleIntegrationKind::AlreadyContained, oid)
            }
        };
        let (publication, publication_oid, pending) = match reached.publication {
            PromotionPublication::Published { oid } => {
                (LifecyclePublicationState::Published, Some(oid), None)
            }
            PromotionPublication::AlreadyCurrent { oid } => {
                (LifecyclePublicationState::AlreadyCurrent, Some(oid), None)
            }
            PromotionPublication::Pending { reason } => (
                LifecyclePublicationState::Pending,
                None,
                Some(publication_pending(reason)),
            ),
        };
        let (cleanup, cleanup_reason) = match reached.cleanup {
            PromotionCleanup::Complete => (LifecycleCleanupState::Complete, None),
            PromotionCleanup::Pending { reason } => (
                LifecycleCleanupState::Pending,
                Some(cleanup_pending(reason)),
            ),
        };
        LifecycleMutationDto {
            recording,
            checkpoint_oid: checkpoint_oid.map(|oid| oid.to_string()),
            integration: Some(integration),
            integration_oid: Some(integration_oid.to_string()),
            publication: Some(publication),
            publication_oid: publication_oid.map(|oid| oid.to_string()),
            publication_pending: pending,
            cleanup: Some(cleanup),
            cleanup_pending: cleanup_reason,
            ..self.data()
        }
    }

    /// The answer to an operation that reached primary. What it left to do
    /// is the same request's to continue, whatever stopped it (decision 8).
    fn reached(&self, accepted: &Accepted, reached: &Reached) -> Ran {
        let code = reached_code(reached);
        // A code's own action for each of these would be wrong: a hand-off
        // is continued by the request that recomputes what it reports, and
        // a host category carries no authority to approve.
        let recovery: Vec<RecoveryAction> = match code {
            ResultCode::DiscoveryPending => vec![outcome::same_request(accepted.request_id)],
            ResultCode::HostApprovalRequired => {
                let mut actions = vec![outcome::same_request(accepted.request_id)];
                actions.extend(outcome::inspect_the_root_it_lies_in(&accepted.root()));
                actions
            }
            _ => Vec::new(),
        };
        Ran {
            answer: Answer {
                effects: reached_effects(reached),
                recovery,
                ..Answer::stopped(code).with_data(MutationDataDto::Lifecycle(Box::new(
                    self.reached_data(reached),
                )))
            },
            standing: if code == ResultCode::Ok {
                Standing::Final
            } else {
                Standing::Owed
            },
        }
    }

    /// The answer to an operation that stopped. What it left is the parent's
    /// to say: it is read after the call.
    fn stopped(&self, service: &RepositoryService, accepted: &Accepted, stop: Stop) -> Ran {
        let root = accepted.root();
        let parent = V::parent(service, &root, accepted.operation_id);
        let (code, blockers, offers_nothing) = stop_answer(&stop);
        let stands = matches!(&parent, Ok(Some(view)) if !ended(view));
        // A refused merge, a child's conflict: the state is the repository's
        // to name, and a preview reads it with nothing changed.
        let names_its_state = stands
            && match &stop {
                Stop::RecoveryRequired => true,
                Stop::ContextSynchronization(error) => {
                    outcome::synchronization_error_code(error) == ResultCode::RecoveryRequired
                }
                _ => false,
            };
        let blockers = match (blockers, names_its_state) {
            (Some(blockers), _) => Some(blockers),
            (None, true) => match V::observe(service, &root, &self.id) {
                Ok(Observed::Blocked(blockers)) => Some(blockers),
                Ok(Observed::Ready(_)) | Err(_) => None,
            },
            (None, false) => None,
        };
        let (effects, recovery, standing) = match &parent {
            // Nothing can be said of what it left: the record is left.
            Err(_) => (
                Effects::not_requested(),
                Vec::new(),
                Standing::Stopped { journal: None },
            ),
            Ok(None) => (
                Effects::not_requested(),
                no_parent_recovery(code),
                Standing::Stopped {
                    journal: Some(JournalRow::Absent),
                },
            ),
            Ok(Some(view)) => (
                parent_effects::<V>(service, &root, view),
                if stands {
                    standing_recovery(code, accepted)
                } else {
                    Vec::new()
                },
                if stands {
                    Standing::Owed
                } else {
                    Standing::Final
                },
            ),
        };
        let data = blockers.map(|blockers| {
            let checkpoint = parent
                .as_ref()
                .ok()
                .and_then(Option::as_ref)
                .filter(|_| effects.checkpoint == CheckpointEffect::Committed)
                .and_then(|view| view.checkpoint);
            MutationDataDto::Lifecycle(Box::new(LifecycleMutationDto {
                blockers,
                recording: checkpoint.map(|_| LifecycleRecording::Record),
                checkpoint_oid: checkpoint.map(|oid| oid.to_string()),
                ..self.data()
            }))
        });
        Ran {
            answer: Answer {
                effects,
                data,
                recovery,
                offers_nothing,
                ..Answer::stopped(code)
            },
            standing,
        }
    }

    /// The one domain call, under the request's operation ID.
    fn call(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        let called = self.expected.and_then(|expected| {
            V::call(
                call.session,
                call.service,
                accepted.root(),
                accepted.operation_id,
                self.id.clone(),
                expected,
            )
        });
        match called {
            Some(Ok(reached)) => self.reached(accepted, &reached),
            Some(Err(stop)) => self.stopped(call.service, accepted, stop),
            // No consent was observed, or the session has no credentials:
            // nothing was called, and the record is left as it is.
            None => Ran {
                answer: Answer::stopped(ResultCode::InternalError),
                standing: Standing::Stopped { journal: None },
            },
        }
    }

    /// The answer to what stands in the way of a request entered again with
    /// no parent, as a prepare gives it. Its data holds what a prepare's
    /// refusal would, since `execute` has no other place for it.
    fn blocked_again(
        &self,
        service: &RepositoryService,
        root: &Path,
        blockers: Vec<LifecycleBlocker>,
    ) -> Answer {
        if names_no_item(service, root, &self.id, &blockers) {
            return Answer::stopped(ResultCode::ItemNotFound);
        }
        Answer::stopped(blocked_code(&blockers)).with_data(MutationDataDto::Lifecycle(Box::new(
            LifecycleMutationDto {
                blockers,
                ..self.data()
            },
        )))
    }

    /// A parent that has ended answers its request from what it holds.
    fn ended_answer(
        &self,
        service: &RepositoryService,
        root: &Path,
        parent: &ParentView<LifecycleBlocker>,
    ) -> Option<Answer> {
        let effects = parent_effects::<V>(service, root, parent);
        let answer = match &parent.refusal {
            Some(Refusal::Blocked(blockers)) => Answer::stopped(blocked_code(blockers)).with_data(
                MutationDataDto::Lifecycle(Box::new(LifecycleMutationDto {
                    blockers: blockers.clone(),
                    ..self.data()
                })),
            ),
            Some(Refusal::ExternalChange | Refusal::ReconfirmationRequired) => {
                Answer::stopped(ResultCode::ExternalChange)
            }
            None if parent.step == ParentStep::Superseded => Answer {
                offers_nothing: true,
                ..Answer::stopped(ResultCode::RequestMismatch)
            },
            None => return None,
        };
        Some(Answer { effects, ..answer })
    }
}

impl<V: LifecycleVerb> Binding for LifecycleBinding<V> {
    fn command(&self) -> Command {
        V::COMMAND
    }

    /// A parent's row is in the remote journal when it has one, and records
    /// phases and no checkpoint step (fact 15).
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }

    fn place(&self) -> Place<'_> {
        Place::Repository(&self.root)
    }

    fn target(&self) -> String {
        format!(
            "{}/{}",
            super::super::super::authoring_kind_segment(&V::KIND),
            self.id
        )
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        let Self {
            // What the digest is begun with: the scope and the target. The
            // request holds nothing else.
            root: _,
            id: _,
            // What a call observed, and no part of the request.
            expected: _,
            verb: _,
        } = self;
        builder
    }

    /// The request's position is on primary, so its result names primary:
    /// the context's worktree does not outlive a complete run.
    fn scope(&self, repository: Option<String>, position: Option<&Position>) -> Scope {
        Scope {
            repository,
            item_id: Some(self.id.to_string()),
            branch: position.and_then(|position| position.branch().map(str::to_owned)),
            worktree: None,
            remote: None,
        }
    }

    /// Observes, and gives the digest the boundary compares with the
    /// confirmation's. What the preview covers is not checked here, with one
    /// exception (R-22): an identity removed since the preview, alone, is
    /// answered as what it is, since a new preview would only say the same.
    /// The request is placed on primary, whose reachability a replay checks.
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        let (root, observed) = self.observe(call.service)?;
        if let Observed::Blocked(blockers) = &observed
            && matches!(blockers.as_slice(), [LifecycleBlocker::IdentityRequired])
        {
            return Err(Answer::stopped(ResultCode::IdentityRequired).with_data(
                MutationDataDto::Lifecycle(Box::new(LifecycleMutationDto {
                    blockers: blockers.clone(),
                    ..self.data()
                })),
            ));
        }
        let configuration =
            match read_configuration_for(&root, RepositoryOperation::RepositorySnapshot)
                .map_err(refused)?
            {
                ConfigurationInspection::Valid(configuration) => configuration,
                ConfigurationInspection::Missing | ConfigurationInspection::Invalid(_) => {
                    return Err(Answer::stopped(ResultCode::InvalidConfiguration));
                }
            };
        let position = Position::of(
            &root,
            &format!("refs/heads/{}", configuration.primary_branch),
        )
        .map_err(|_| Answer::stopped(ResultCode::RepositoryInaccessible))?;
        self.expected = match &observed {
            Observed::Ready(ready) => Some(ready.expected),
            Observed::Blocked(_) => None,
        };
        Ok(Prepared {
            family: Some(OperationFamily::Remote),
            position,
            expected: None,
            observation: Some(digest_of(V::COMMAND, &observed).to_stored()),
        })
    }

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        self.call(call, accepted)
    }

    fn reenter(
        &mut self,
        call: &CallContext<'_>,
        accepted: &Accepted,
        journal: JournalRow,
    ) -> Result<Entered, Ran> {
        confirm::reenter(self, call, accepted, journal)
    }

    fn as_confirmed(&mut self) -> Option<&mut dyn Confirmed> {
        Some(self)
    }

    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, _entered: &Entered) -> Ran {
        self.call(call, accepted)
    }
}

impl<V: LifecycleVerb> Confirmed for LifecycleBinding<V> {
    fn preview(&mut self, call: &CallContext<'_>) -> Result<Previewed, Answer> {
        let (root, observed) = self.observe(call.service)?;
        let digest = digest_of(V::COMMAND, &observed);
        let ready = match observed {
            Observed::Ready(ready) => ready,
            Observed::Blocked(blockers) => {
                if names_no_item(call.service, &root, &self.id, &blockers) {
                    return Err(Answer::stopped(ResultCode::ItemNotFound));
                }
                return Err(Answer {
                    lifecycle_blocked: Some(LifecycleBlockedDto {
                        blockers: blockers.clone(),
                    }),
                    ..Answer::stopped(blocked_code(&blockers))
                });
            }
        };
        let key = ScopeKey::for_repository(&self.root)
            .ok_or_else(|| Answer::stopped(ResultCode::InvalidPath))?;
        let mut effects = Vec::new();
        if ready.context_child {
            effects.push(PreviewEffect::ContextSynchronized);
        }
        let closure = ready.closure.as_ref().map(|closure| match closure {
            PlannedClosure::Record { closed_by } => {
                effects.push(PreviewEffect::ClosureRecorded);
                LifecyclePlannedClosureDto {
                    recording: LifecycleRecording::Record,
                    closed_by: closed_by.clone(),
                }
            }
            PlannedClosure::Existing {
                closed_at: _,
                closed_by,
            } => LifecyclePlannedClosureDto {
                recording: LifecycleRecording::Existing,
                closed_by: closed_by.clone(),
            },
        });
        if ready.integration == PlannedIntegration::MergeCommit {
            effects.push(PreviewEffect::PrimaryIntegrated);
        }
        if ready.publication == PlannedPublication::Publish {
            effects.push(PreviewEffect::PrimaryPublished);
        }
        if ready.cleanup.remote_branch {
            effects.push(PreviewEffect::RemoteBranchDeleted);
        }
        if ready.cleanup.worktree {
            effects.push(PreviewEffect::WorktreeRemoved);
        }
        if ready.cleanup.local_branch {
            effects.push(PreviewEffect::BranchDeleted);
        }
        // A label from the index: one it does not hold is shown as none, and
        // is no part of what is consented to.
        let slug = if V::KIND == AuthoringKind::Ticket {
            resolve_root(call.service, &root)
                .ok()
                .and_then(|repository| call.service.show_item(&repository, &self.id).ok())
                .and_then(|item| item.slug)
        } else {
            None
        };
        Ok(Previewed {
            observation: digest.to_stored(),
            effects,
            details: PreviewDetailsDto::Lifecycle(Box::new(LifecyclePreviewDto {
                root: key.as_str().to_owned(),
                kind: V::DTO_KIND,
                item_id: self.id.to_string(),
                slug,
                branch: ready
                    .branch
                    .strip_prefix("refs/heads/")
                    .unwrap_or(&ready.branch)
                    .to_owned(),
                primary_branch: ready.primary_branch.clone(),
                remote: ready.remote.clone(),
                context_oid: ready.context_oid.to_string(),
                primary_oid: ready.primary_oid.to_string(),
                changes: ready.changes.iter().map(path_change).collect(),
                integration: match ready.integration {
                    PlannedIntegration::MergeCommit => LifecyclePlannedIntegration::MergeCommit,
                    PlannedIntegration::AlreadyContained => {
                        LifecyclePlannedIntegration::AlreadyContained
                    }
                },
                publication: match ready.publication {
                    PlannedPublication::Publish => LifecyclePlannedPublication::Publish,
                    PlannedPublication::PendingNoRemote => {
                        LifecyclePlannedPublication::PendingNoRemote
                    }
                },
                cleanup: LifecyclePlannedCleanupDto {
                    remote_branch: ready.cleanup.remote_branch,
                    worktree: ready.cleanup.worktree,
                    local_branch: ready.cleanup.local_branch,
                },
                closure,
            })),
        })
    }

    /// The parent record says where the request stands; see the table in
    /// the module's comment. A parent that goes on is called with the
    /// expectation it was begun with, not one read now.
    fn row(&mut self, call: &CallContext<'_>, accepted: &Accepted, _journal: &JournalRow) -> Row {
        let root = accepted.root();
        let parent = match V::parent(call.service, &root, accepted.operation_id) {
            Ok(Some(parent)) => parent,
            Ok(None) => return Row::Unrecorded,
            Err(error) => {
                return Row::answers(Ran {
                    answer: refused(error),
                    standing: Standing::Stopped { journal: None },
                });
            }
        };
        if let Some(answer) = self.ended_answer(call.service, &root, &parent) {
            return Row::answers(Ran {
                answer,
                standing: Standing::Final,
            });
        }
        self.expected = Some(parent.expectation);
        Row::Unfinished
    }

    /// No parent: the request stopped before its record began. What is there
    /// is the consent's to judge, in `confirm::reenter`. A state now blocked
    /// answers as a prepare does, and the record is deleted.
    fn observe_again(
        &mut self,
        call: &CallContext<'_>,
        _accepted: &Accepted,
        _journal: &JournalRow,
    ) -> Result<Seen, Ran> {
        let (root, observed) = self.observe(call.service).map_err(|answer| Ran {
            answer,
            standing: Standing::Stopped { journal: None },
        })?;
        let found = digest_of(V::COMMAND, &observed);
        match observed {
            Observed::Ready(ready) => {
                self.expected = Some(ready.expected);
                Ok(Seen::State { found, own: None })
            }
            Observed::Blocked(blockers) => Err(Ran {
                answer: self.blocked_again(call.service, &root, blockers),
                standing: Standing::Stopped {
                    journal: Some(JournalRow::Absent),
                },
            }),
        }
    }
}

/// What a request that has not settled shows of its effects, for a call that
/// finds its record accepted and does not run it.
pub(super) fn recorded_effects<V: LifecycleVerb>(
    service: &RepositoryService,
    accepted: &Accepted,
) -> Effects {
    let root = accepted.root();
    match V::parent(service, &root, accepted.operation_id) {
        Ok(Some(parent)) => parent_effects::<V>(service, &root, &parent),
        Ok(None) | Err(_) => Effects::not_requested(),
    }
}

/// Whether a parent has ended, so that its request's answer is final: refused,
/// superseded or complete.
fn ended(parent: &ParentView<LifecycleBlocker>) -> bool {
    parent.refusal.is_some()
        || matches!(
            parent.step,
            ParentStep::Abandoned | ParentStep::Superseded | ParentStep::Complete
        )
}

/// The effects a parent shows, read from its step: a commit named by the
/// record is not yet an effect (fact 41). A superseded parent had reached
/// cleanup, and only that is ever superseded.
fn parent_effects<V: LifecycleVerb>(
    service: &RepositoryService,
    root: &Path,
    parent: &ParentView<LifecycleBlocker>,
) -> Effects {
    let integrated = matches!(
        parent.step,
        ParentStep::Integrated
            | ParentStep::Published
            | ParentStep::PublicationPending
            | ParentStep::RemoteDeleted
            | ParentStep::WorktreeRemoving
            | ParentStep::WorktreeRemoved
            | ParentStep::BranchDeleted
            | ParentStep::Complete
            | ParentStep::Superseded
    );
    // A checkpoint is written once the context's branch holds it, or primary
    // once that branch is gone.
    let branch = context_names(root, &V::KIND, &parent.item_id.to_string()).branch_ref;
    let checkpointed = parent
        .checkpoint
        .is_some_and(|commit| evidence::still_reachable(root, Some(&branch), commit));
    if !integrated {
        return Effects {
            checkpoint: if checkpointed {
                CheckpointEffect::Committed
            } else {
                CheckpointEffect::NotRequested
            },
            ..Effects::not_requested()
        };
    }
    Effects {
        write: WriteEffect::NotRequested,
        checkpoint: if checkpointed {
            CheckpointEffect::Committed
        } else {
            CheckpointEffect::NotRequested
        },
        discovery: if service
            .repository_snapshot(root)
            .is_ok_and(|snapshot| !snapshot.refresh_required)
        {
            DiscoveryEffect::Current
        } else {
            DiscoveryEffect::Pending
        },
        publication: match parent.publication {
            Some(PublicationCategory::Published) => PublicationEffect::Published,
            Some(PublicationCategory::AlreadyCurrent) => PublicationEffect::Current,
            _ => PublicationEffect::Pending,
        },
        integration: IntegrationEffect::Complete,
        cleanup: if parent.step == ParentStep::Complete {
            CleanupEffect::Complete
        } else {
            CleanupEffect::Pending
        },
        commit_oid: parent.integration.map(|oid| oid.to_string()),
    }
}

/// The code a stop answers with, the blockers it names, and whether the code's
/// actions are not to be offered. A repository error of any kind stays one: it
/// may follow a written checkpoint, so it never reads as a consent refusal
/// (fact 11).
fn stop_answer(stop: &Stop) -> (ResultCode, Option<Vec<LifecycleBlocker>>, bool) {
    match stop {
        Stop::Repository(error) => (outcome::repository_error_code(error.kind), None, false),
        Stop::Transport(error) => (outcome::ssh_transport_error_code(&error.kind), None, false),
        Stop::Blocked(blockers) => (blocked_code(blockers), Some(blockers.clone()), false),
        Stop::ExternalChange | Stop::ReconfirmationRequired | Stop::PrimaryAdvanced => {
            (ResultCode::ExternalChange, None, false)
        }
        Stop::IdentityRequired => (ResultCode::IdentityRequired, None, false),
        Stop::ContextSynchronization(error) => {
            (outcome::synchronization_error_code(error), None, false)
        }
        Stop::Busy => (ResultCode::Busy, None, false),
        Stop::PollYielding => (ResultCode::PollYielding, None, false),
        // Task 2c answers `cancelled` for one that was asked to stop.
        Stop::Interrupted | Stop::RecoveryRequired => (ResultCode::RecoveryRequired, None, false),
        Stop::OperationMismatch => (ResultCode::RequestMismatch, None, true),
    }
}

/// With no parent the request is deleted, so a code whose own action is to
/// send it again offers a new preview: the design's first row of the standing
/// table, "whatever its code". Any other code's action stands.
fn no_parent_recovery(code: ResultCode) -> Vec<RecoveryAction> {
    match code {
        ResultCode::Busy
        | ResultCode::PollYielding
        | ResultCode::RemoteUnavailable
        | ResultCode::TransportUnavailable => vec![outcome::prepare_again()],
        _ => Vec::new(),
    }
}

/// What a code would offer wrongly while a parent stands: a host stop carries
/// no authority to approve, and a child's branch deleted at the remote is the
/// child's reservation and not the request's to cancel, so the request is what
/// is offered. Every other code's own actions follow the same request.
fn standing_recovery(code: ResultCode, accepted: &Accepted) -> Vec<RecoveryAction> {
    match code {
        ResultCode::HostApprovalRequired | ResultCode::HostReplacementRequired => {
            let mut actions = vec![outcome::same_request(accepted.request_id)];
            actions.extend(outcome::inspect_the_root_it_lies_in(&accepted.root()));
            actions
        }
        _ if outcome::never_gets_past(code) => vec![outcome::same_request(accepted.request_id)],
        _ => Vec::new(),
    }
}

/// An ID no item of this kind holds, as `item republish` answers it. An item
/// with no context reads as a missing context to the domain, and the index
/// tells the two apart, unless it is marked behind: then it cannot say.
fn names_no_item(
    service: &RepositoryService,
    root: &Path,
    id: &ItemId,
    blockers: &[LifecycleBlocker],
) -> bool {
    let unknown = blockers.contains(&LifecycleBlocker::ContextMissing)
        && service
            .repository_snapshot(root)
            .is_ok_and(|snapshot| !snapshot.refresh_required)
        && resolve_root(service, root).is_ok_and(|repository| {
            service
                .show_item(&repository, id)
                .is_err_and(|error| error.code() == ResultCode::ItemNotFound)
        });
    unknown
        || blockers.iter().any(|blocker| {
            matches!(
                blocker,
                LifecycleBlocker::NotADocument | LifecycleBlocker::NotATicket
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::{RepositoryErrorKind, RepositoryOperation};

    fn reached(publication: PromotionPublication, cleanup: PromotionCleanup) -> Reached {
        Reached {
            recording: None,
            integration: PromotionIntegration::Merged {
                oid: git2::Oid::zero(),
            },
            publication,
            cleanup,
            index_pending: false,
        }
    }

    fn pending_publication(reason: PromotionPublicationPending) -> PromotionPublication {
        PromotionPublication::Pending { reason }
    }

    fn pending_cleanup(reason: PromotionCleanupPending) -> PromotionCleanup {
        PromotionCleanup::Pending { reason }
    }

    const AWAITING: PromotionCleanup = PromotionCleanup::Pending {
        reason: PromotionCleanupPending::AwaitingPublication,
    };

    #[test]
    fn a_publication_that_stopped_answers_with_its_category() {
        use PromotionSynchronizationCategory as Category;
        for (category, code) in [
            (Category::Conflict, ResultCode::RecoveryRequired),
            (Category::Rejected, ResultCode::PushRejected),
            (Category::Transport, ResultCode::TransportUnavailable),
            (Category::Identity, ResultCode::IdentityRequired),
            (Category::Busy, ResultCode::Busy),
            (Category::Yielding, ResultCode::PollYielding),
            (Category::RecoveryRequired, ResultCode::RecoveryRequired),
        ] {
            let stopped = reached(
                pending_publication(PromotionPublicationPending::Synchronization(category)),
                AWAITING,
            );
            assert_eq!(reached_code(&stopped), code, "{category:?}");
        }
    }

    #[test]
    fn a_deletion_that_stopped_answers_with_its_category() {
        use PromotionRemoteDeletionCategory as Category;
        for (category, code) in [
            (Category::Transport, ResultCode::TransportUnavailable),
            (Category::Rejected, ResultCode::PushRejected),
            (Category::Configuration, ResultCode::RecoveryRequired),
            (Category::Key, ResultCode::NoSelectedKey),
            (Category::Unlock, ResultCode::UnlockRequired),
            (Category::Host, ResultCode::HostApprovalRequired),
        ] {
            let stopped = reached(
                PromotionPublication::Published {
                    oid: git2::Oid::zero(),
                },
                pending_cleanup(PromotionCleanupPending::RemoteDeletionFailed(category)),
            );
            assert_eq!(reached_code(&stopped), code, "{category:?}");
        }
    }

    #[test]
    fn cleanup_stops_that_name_no_category_are_recovery() {
        for reason in [
            PromotionCleanupPending::ContextChanged,
            PromotionCleanupPending::IgnoredFiles,
            PromotionCleanupPending::RemoteContextChanged,
            PromotionCleanupPending::RecoveryRequired,
        ] {
            let stopped = reached(
                PromotionPublication::Published {
                    oid: git2::Oid::zero(),
                },
                pending_cleanup(reason),
            );
            assert_eq!(reached_code(&stopped), ResultCode::RecoveryRequired);
        }
    }

    #[test]
    fn cancelled_publication_or_cleanup_is_cancelled() {
        assert_eq!(
            reached_code(&reached(
                pending_publication(PromotionPublicationPending::Cancelled),
                AWAITING
            )),
            ResultCode::Cancelled
        );
        assert_eq!(
            reached_code(&reached(
                PromotionPublication::Published {
                    oid: git2::Oid::zero()
                },
                pending_cleanup(PromotionCleanupPending::Cancelled)
            )),
            ResultCode::Cancelled
        );
    }

    /// Task 0 found no failure point that yields this after a checkpoint. A
    /// repository error of kind `ExternalChange` is `external_change` by its
    /// own mapping, and is not a consent refusal: a standing parent decides
    /// what is left, and the domain's half is `closure/run_tests.rs`.
    #[test]
    fn a_repository_external_change_stays_a_repository_error() {
        let stop = Stop::Repository(RepositoryError::new(
            RepositoryOperation::RepositorySnapshot,
            None,
            RepositoryErrorKind::ExternalChange,
            "moved",
        ));
        let (code, blockers, offers_nothing) = stop_answer(&stop);
        assert_eq!(code, ResultCode::ExternalChange);
        assert_eq!(blockers, None);
        assert!(!offers_nothing);
    }

    #[test]
    fn a_stop_answers_with_the_tables_code() {
        for (stop, code) in [
            (Stop::Busy, ResultCode::Busy),
            (Stop::PollYielding, ResultCode::PollYielding),
            (Stop::Interrupted, ResultCode::RecoveryRequired),
            (Stop::RecoveryRequired, ResultCode::RecoveryRequired),
            (Stop::IdentityRequired, ResultCode::IdentityRequired),
            (Stop::PrimaryAdvanced, ResultCode::ExternalChange),
            (Stop::ExternalChange, ResultCode::ExternalChange),
            (Stop::ReconfirmationRequired, ResultCode::ExternalChange),
        ] {
            assert_eq!(stop_answer(&stop).0, code);
        }
        let (code, _, offers_nothing) = stop_answer(&Stop::OperationMismatch);
        assert_eq!(code, ResultCode::RequestMismatch);
        assert!(offers_nothing);
    }

    #[test]
    fn with_no_parent_a_code_that_would_resend_prepares_again() {
        for code in [
            ResultCode::Busy,
            ResultCode::PollYielding,
            ResultCode::TransportUnavailable,
            ResultCode::RemoteUnavailable,
        ] {
            let offered = no_parent_recovery(code);
            assert_eq!(offered.len(), 1);
            assert_eq!(
                offered[0].action,
                crate::results::RecoveryActionKind::RequestPrepare
            );
        }
        assert!(no_parent_recovery(ResultCode::RecoveryRequired).is_empty());
    }
}
