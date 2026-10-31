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
//! **A first run** calls the domain under the request's operation ID, with no
//! host approval and no confirmed identity (decision 6). What is left of the
//! request is read from the parent record, not the journal, because a parent
//! usually has no journal row (fact 29): no parent, the request is deleted; a
//! parent a consent refusal ended, the refusal is stored; an unfinished one,
//! the request stays accepted, and never `Stopped{Absent}`.
//!
//! **Left for later tasks.** Task 2b continues an unfinished parent under its
//! request, with `Row::Unfinished` in place of the safe answer `row` gives
//! here, and reads `recorded_effects` from the parent. Task 2c connects
//! cancellation, Task 3 `operation resume`, Task 4 the made-ready report, and
//! Task 6 the promotion identity row.

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
            promotion::{ParentView, lifecycle::ParentStep},
            read_configuration_for,
            recovery::JournalRow,
            transport::SshTransportError,
        },
        command::Command,
        confirm::{Row, Seen},
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
    tracking_context: Option<git2::Oid>,
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

    /// Whether a context child runs before integration, by the verb's rule.
    fn runs_context_child(ready: &Ready) -> bool;

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
    ) -> Result<Option<ParentView>, RepositoryError>;
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
        tracking_context: preview.tracking_context(),
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
        tracking_context: preview.tracking_context(),
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

    /// Cycle 09's rule, as `LifecyclePreview::runs_context_child` has it.
    fn runs_context_child(ready: &Ready) -> bool {
        ready.remote.is_some()
            && !(ready.integration == PlannedIntegration::AlreadyContained
                && ready.tracking_context.is_none())
    }

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
    ) -> Result<Option<ParentView>, RepositoryError> {
        promotion::lifecycle_parent(service, root, operation)
    }
}

impl LifecycleVerb for Close {
    const COMMAND: Command = Command::TicketClose;
    const KIND: AuthoringKind = AuthoringKind::Ticket;
    const DTO_KIND: ItemDtoKind = ItemDtoKind::Ticket;

    /// A contained context is never synchronized, and the checkpoint is never
    /// pushed (`ClosureObservation::needs_context_child`).
    fn runs_context_child(ready: &Ready) -> bool {
        ready.remote.is_some() && ready.integration != PlannedIntegration::AlreadyContained
    }

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
    ) -> Result<Option<ParentView>, RepositoryError> {
        closure::lifecycle_parent(service, root, operation)
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

    /// The effects a parent shows of a request that stopped, read from its
    /// step: a commit named by the record is not yet an effect (fact 41).
    fn parent_effects(root: &Path, id: &ItemId, parent: &ParentView) -> Effects {
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
        );
        // A checkpoint is written once the context's branch holds it.
        let branch = context_names(root, &V::KIND, &id.to_string()).branch_ref;
        let checkpointed = parent
            .checkpoint
            .is_some_and(|commit| evidence::still_reachable(root, Some(&branch), commit));
        Effects {
            checkpoint: if checkpointed {
                CheckpointEffect::Committed
            } else {
                CheckpointEffect::NotRequested
            },
            integration: if integrated {
                IntegrationEffect::Complete
            } else {
                IntegrationEffect::NotRequested
            },
            commit_oid: if integrated {
                parent.integration.map(|oid| oid.to_string())
            } else {
                None
            },
            ..Effects::not_requested()
        }
    }

    /// The answer to an operation that stopped. What it left is the parent's
    /// to say: it is read after the call.
    fn stopped(&self, service: &RepositoryService, accepted: &Accepted, stop: Stop) -> Ran {
        let root = accepted.root();
        let parent = V::parent(service, &root, accepted.operation_id);
        let (code, blockers, offers_nothing) = match &stop {
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
            Stop::Interrupted | Stop::RecoveryRequired => {
                (ResultCode::RecoveryRequired, None, false)
            }
            Stop::OperationMismatch => (ResultCode::RequestMismatch, None, true),
        };
        let data = blockers.map(|blockers| {
            MutationDataDto::Lifecycle(Box::new(LifecycleMutationDto {
                blockers,
                ..self.data()
            }))
        });
        let (effects, standing) = match &parent {
            // Nothing can be said of what it left: the record is left.
            Err(_) => (
                Effects::not_requested(),
                Standing::Stopped { journal: None },
            ),
            Ok(None) => (
                Effects::not_requested(),
                Standing::Stopped {
                    journal: Some(JournalRow::Absent),
                },
            ),
            Ok(Some(view)) => (
                Self::parent_effects(&root, &self.id, view),
                if view.refused
                    || matches!(
                        view.step,
                        ParentStep::Abandoned | ParentStep::Superseded | ParentStep::Complete
                    )
                {
                    Standing::Final
                } else {
                    Standing::Owed
                },
            ),
        };
        Ran {
            answer: Answer {
                effects,
                data,
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

    /// Task 2b's until then: a request entered again is not run, and stays
    /// as it is for the same request to take up once it can.
    fn not_entered() -> Ran {
        Ran {
            answer: Answer::stopped(ResultCode::RecoveryRequired),
            standing: Standing::Unconfirmed,
        }
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
    /// confirmation's: no check is made here of what the preview covers.
    /// The request is placed on primary, whose reachability a replay checks.
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        let (root, observed) = self.observe(call.service)?;
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
        _call: &CallContext<'_>,
        _accepted: &Accepted,
        _journal: JournalRow,
    ) -> Result<Entered, Ran> {
        // A confirmed command is entered again by `confirm::reenter` and by
        // nothing else (fact 36): this is not reached.
        Err(Self::not_entered())
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
                // An ID no item of this kind holds, as `item republish`
                // answers it. An item with no context reads as a missing
                // context to the domain, and the index tells the two apart.
                let unknown = blockers.contains(&LifecycleBlocker::ContextMissing)
                    && resolve_root(call.service, &root).is_ok_and(|repository| {
                        call.service
                            .show_item(&repository, &self.id)
                            .is_err_and(|error| error.code() == ResultCode::ItemNotFound)
                    });
                if unknown
                    || blockers.iter().any(|blocker| {
                        matches!(
                            blocker,
                            LifecycleBlocker::NotADocument | LifecycleBlocker::NotATicket
                        )
                    })
                {
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
        if V::runs_context_child(&ready) {
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

    /// Task 2b reads the parent here, and gives `Row::Unfinished`. Until
    /// then a request entered again is answered without a domain call, and
    /// left accepted, whatever its parent holds: nothing is run on a consent
    /// this task has not compared, and nothing it did is forgotten.
    fn row(&mut self, _call: &CallContext<'_>, _accepted: &Accepted, _journal: &JournalRow) -> Row {
        Row::answers(Self::not_entered())
    }

    /// Asked only of a row that reads `Row::Unrecorded`, which Task 2b is the
    /// first to answer.
    fn observe_again(
        &mut self,
        _call: &CallContext<'_>,
        _accepted: &Accepted,
        _journal: &JournalRow,
    ) -> Result<Seen, Ran> {
        Err(Self::not_entered())
    }
}
