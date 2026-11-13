//! What is mechanical about the two verbs' domain calls: their inputs, what each
//! preview and call gives, and the same in the binding's own types.

use std::path::{Path, PathBuf};

use crate::canonical::ItemId;

use super::super::super::{
    super::{
        AuthoringKind, CloseTicketRequest, ClosureBlocker, ClosureExpectation, ClosureRecording,
        DocumentPromotionError, DocumentPromotionOutcome, DocumentPromotionPreflight,
        DocumentPromotionPreview, DocumentPromotionResult, ItemDtoKind, OperationId, PathChange,
        PlannedCleanup, PlannedClosure, PlannedIntegration, PlannedPublication,
        PreviewDocumentPromotionRequest, PreviewTicketClosureRequest, PromoteDocumentRequest,
        PromotionBlocker, PromotionCleanup, PromotionCleanupPending, PromotionExpectation,
        PromotionIntegration, PromotionPathChange, PromotionPublication,
        PromotionPublicationPending, PromotionRemoteDeletionCategory,
        PromotionSynchronizationCategory, ReadError, RepositoryError, RepositoryOperation,
        RepositoryService, SynchronizationError, SynchronizationTarget, TicketClosureError,
        TicketClosureOutcome, TicketClosurePreflight, TicketClosurePreview, TicketClosureResult,
        closure, promotion,
        promotion::{ParentView, lifecycle::LifecyclePreview, run::RunObserver},
        remote::{state, sync},
        transport::SshTransportError,
    },
    command::Command,
    dto::{
        LifecycleBlocker, LifecycleCleanupPendingReason, LifecyclePathChangeDto,
        LifecyclePathChangeKind, LifecyclePublicationPendingReason,
    },
    identity::RequestId,
    progress::Session,
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
pub(in super::super::super) struct Ready {
    pub(super) expected: [u8; 32],
    pub(super) branch: String,
    pub(super) primary_branch: String,
    pub(super) remote: Option<String>,
    pub(super) context_oid: git2::Oid,
    pub(super) primary_oid: git2::Oid,
    /// A context child runs before integration, by the verb's rule.
    pub(super) context_child: bool,
    pub(super) changes: Vec<PromotionPathChange>,
    pub(super) integration: PlannedIntegration,
    pub(super) publication: PlannedPublication,
    pub(super) cleanup: PlannedCleanup,
    pub(super) closure: Option<PlannedClosure>,
}

pub(in super::super::super) enum Observed {
    Ready(Box<Ready>),
    Blocked(Vec<LifecycleBlocker>),
}

/// How far an operation that reached primary got, of either verb.
pub(in super::super::super) struct Reached {
    pub(super) recording: Option<ClosureRecording>,
    pub(super) integration: PromotionIntegration,
    pub(super) publication: PromotionPublication,
    pub(super) cleanup: PromotionCleanup,
    pub(super) index_pending: bool,
}

/// What stopped an operation before it could report that, of either verb.
pub(in super::super::super) enum Stop {
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
    /// `Interrupted`, by a cancellation.
    Cancelled,
    OperationMismatch,
    RecoveryRequired,
}

/// What is the verb's own: its command, its preview, its one domain call and
/// its parent table.
pub(in super::super::super) trait LifecycleVerb {
    const COMMAND: Command;
    const KIND: AuthoringKind;
    const DTO_KIND: ItemDtoKind;
    /// Whether its answers hold a made-ready report: a closure's do. A ticket is
    /// also what has a slug, so `preview` reads this to look one up.
    const REPORTS_MADE_READY: bool;

    fn observe(
        service: &RepositoryService,
        root: &Path,
        id: &ItemId,
    ) -> Result<Observed, RepositoryError>;

    /// `None` when the session has no credentials to call it with. `progress`
    /// is told of each step the run takes (D5 decision 11).
    #[allow(clippy::too_many_arguments)]
    fn call(
        session: &dyn Session,
        service: &RepositoryService,
        root: PathBuf,
        operation_id: OperationId,
        item_id: ItemId,
        expected: [u8; 32],
        request_id: RequestId,
        progress: &mut dyn RunObserver,
    ) -> Option<Result<Reached, Stop>>;

    fn parent(
        service: &RepositoryService,
        root: &Path,
        operation: OperationId,
    ) -> Result<Option<ParentView<LifecycleBlocker>>, RepositoryError>;

    /// The item's unfinished parent that a new request can't supersede, in
    /// the repository `repository_id`: a read in the caller's read session.
    fn holder(
        db: &rusqlite::Connection,
        repository_id: i64,
        id: &ItemId,
    ) -> Result<Option<OperationId>, ReadError>;
}

pub(in super::super::super) struct Promote;
pub(in super::super::super) struct Close;

/// Whether a commit made now would have no usable identity. It only reads.
fn identity_missing(root: &Path) -> Result<bool, RepositoryError> {
    let operation = RepositoryOperation::RepositorySnapshot;
    let repository = git2::Repository::open(root)
        .map_err(|error| RepositoryError::git(operation, Some(root.to_owned()), error))?;
    match sync::select_committing_identity(&repository, None, &SynchronizationTarget::Primary) {
        Ok(_) => Ok(false),
        Err(SynchronizationError::IdentityRequired { .. }) => Ok(true),
        Err(SynchronizationError::Repository(error)) => Err(error),
        Err(_) => Err(state::recovery_required()),
    }
}

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
        PromotionBlocker::HistoryUnresolved => LifecycleBlocker::HistoryUnresolved,
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
        ClosureBlocker::HistoryUnresolved => LifecycleBlocker::HistoryUnresolved,
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
    const REPORTS_MADE_READY: bool = false;

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
                DocumentPromotionPreflight::Ready(preview)
                    if preview.integration() == PlannedIntegration::MergeCommit
                        && identity_missing(root)? =>
                {
                    Observed::Blocked(vec![LifecycleBlocker::IdentityRequired])
                }
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
        request_id: RequestId,
        progress: &mut dyn RunObserver,
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
            request_id,
            progress,
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
        Ok(
            promotion::lifecycle_parent::<promotion::record::PromotionRecord, _>(
                service, root, operation,
            )?
            .map(|parent| parent.of_blockers(promotion_blocker)),
        )
    }

    fn holder(
        db: &rusqlite::Connection,
        repository_id: i64,
        id: &ItemId,
    ) -> Result<Option<OperationId>, ReadError> {
        promotion::unfinished_holder::<promotion::record::PromotionRecord>(db, repository_id, id)
    }
}

impl LifecycleVerb for Close {
    const COMMAND: Command = Command::TicketClose;
    const KIND: AuthoringKind = AuthoringKind::Ticket;
    const DTO_KIND: ItemDtoKind = ItemDtoKind::Ticket;
    const REPORTS_MADE_READY: bool = true;

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
        request_id: RequestId,
        progress: &mut dyn RunObserver,
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
            request_id,
            progress,
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
        Ok(
            promotion::lifecycle_parent::<closure::record::ClosureRecord, _>(
                service, root, operation,
            )?
            .map(|parent| parent.of_blockers(closure_blocker)),
        )
    }

    fn holder(
        db: &rusqlite::Connection,
        repository_id: i64,
        id: &ItemId,
    ) -> Result<Option<OperationId>, ReadError> {
        promotion::unfinished_holder::<closure::record::ClosureRecord>(db, repository_id, id)
    }
}

/// What a preview of the item shows of its changes, relative to the repository.
pub(super) fn path_change(change: &PromotionPathChange) -> LifecyclePathChangeDto {
    LifecyclePathChangeDto {
        path: change.path().to_string_lossy().replace('\\', "/"),
        change: match change.change() {
            PathChange::Added => LifecyclePathChangeKind::Added,
            PathChange::Modified => LifecyclePathChangeKind::Modified,
            PathChange::Deleted => LifecyclePathChangeKind::Deleted,
        },
    }
}

pub(super) fn publication_pending(
    reason: PromotionPublicationPending,
) -> LifecyclePublicationPendingReason {
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

pub(super) fn cleanup_pending(reason: PromotionCleanupPending) -> LifecycleCleanupPendingReason {
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
