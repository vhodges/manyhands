//! What is mechanical about the two verbs' domain calls: their inputs, what each
//! preview and call gives, and the same in the binding's own types.

use std::path::{Path, PathBuf};

use crate::{
    canonical::{Integration, ItemId},
    results::ResultCode,
};

use super::super::super::{
    super::{
        AuthoringKind, BranchAnswer, BranchClosureOutcome, CloseTicketRequest, ClosureAction,
        ClosureBlocker, ClosureExpectation, ClosureRecording, ClosureWithdrawalPreflight,
        ConfigurationInspection, DocumentPromotionError, DocumentPromotionOutcome,
        DocumentPromotionPreflight, DocumentPromotionPreview, DocumentPromotionResult, ItemDtoKind,
        OperationId, PathChange, PlannedCleanup, PlannedClosure, PlannedIntegration,
        PlannedPublication, PreviewDocumentPromotionRequest, PreviewTicketClosureRequest,
        PromoteDocumentRequest, PromotionBlocker, PromotionCleanup, PromotionCleanupPending,
        PromotionExpectation, PromotionIntegration, PromotionPathChange, PromotionPublication,
        PromotionPublicationPending, PromotionRemoteDeletionCategory,
        PromotionSynchronizationCategory, ReadError, RepositoryError, RepositoryOperation,
        RepositoryService, SynchronizationError, SynchronizationTarget, TicketClosureError,
        TicketClosureOutcome, TicketClosurePreflight, TicketClosurePreview, TicketClosureResult,
        closure, promotion,
        promotion::{ParentView, lifecycle::LifecyclePreview, run::RunObserver},
        read_configuration_for,
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

/// The input of `ticket withdraw-closure`: the ticket.
#[derive(Clone)]
pub struct TicketWithdrawClosureInput {
    pub root: PathBuf,
    pub id: ItemId,
}

/// What a branch record's preview shows that a direct one does not: the
/// action, the commits it starts from, and a new branch's retirement and
/// unheld entries (Part H).
pub(in super::super::super) struct BranchReady {
    pub(super) action: ClosureAction,
    /// The context's head: a new branch's old tip, none for a new branch
    /// with no context.
    pub(super) context_oid: Option<git2::Oid>,
    pub(super) tracking_primary: Option<git2::Oid>,
    /// What a new branch retires of the old context.
    pub(super) retired_oid: Option<git2::Oid>,
    /// Each path the old context holds that the primary does not.
    pub(super) unheld: Vec<LifecyclePathChangeDto>,
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
    /// Set for a branch record's preview, which is shown as its own.
    pub(super) record: Option<BranchReady>,
    /// Set for a pull-request promotion's preview: the situation, the link
    /// and `R` it consents to, which integrates nothing.
    pub(super) pull_request: Option<promotion::PullRequestPromotion>,
}

pub(in super::super::super) enum Observed {
    Ready(Box<Ready>),
    Blocked(Vec<LifecycleBlocker>),
    /// A withdraw of a ticket whose branch holds no closure: `already_applied`
    /// with nothing to confirm (decision 11).
    NothingToWithdraw,
    /// A promote in a pull-request repository whose document the primary
    /// already holds: `already_applied` with nothing to confirm (R-4).
    PrimaryHoldsDocument,
    /// A withdraw in a repository with no publication remote: its children
    /// publish, so there is nowhere to (R-33).
    PublicationRemoteRequired,
}

/// How far an operation that reached primary got, of either verb.
pub(in super::super::super) struct Reached {
    pub(super) recording: Option<ClosureRecording>,
    pub(super) integration: PromotionIntegration,
    pub(super) publication: PromotionPublication,
    pub(super) cleanup: PromotionCleanup,
    pub(super) index_pending: bool,
}

/// What a call that did not stop answers: how far an operation got that
/// reached primary, or how a pull-request close answered.
pub(in super::super::super) enum Called {
    Reached(Reached),
    /// A branch record's answer, whatever it was. `index_pending` is the
    /// domain's own: its refresh is yet to run.
    Branch {
        outcome: BranchClosureOutcome,
        index_pending: bool,
    },
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
    ) -> Option<Result<Called, Stop>>;

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
/// `ticket withdraw-closure`: the close's twin over the same record, whose
/// action is `withdraw`.
pub(in super::super::super) struct Withdraw;

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
        ClosureBlocker::RemoteContextUnmaterialized => {
            LifecycleBlocker::RemoteContextUnmaterialized
        }
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
        record: None,
        pull_request: preview.pull_request().cloned(),
    }
}

fn ready_of_closure(preview: &TicketClosurePreview) -> Ready {
    // Every path but a direct close's is a branch record's, whose preview
    // shows what it starts from and, for a new branch, what it retires.
    let record = (preview.path() != (Integration::Direct, ClosureAction::Close)).then(|| {
        let facts = preview.new_branch();
        BranchReady {
            action: preview.path().1,
            context_oid: match facts {
                Some(facts) => facts.old_tip,
                None => Some(preview.context_oid()),
            },
            tracking_primary: preview.tracking_primary(),
            retired_oid: facts.and_then(|facts| facts.retired_oid),
            unheld: facts
                .map(|facts| {
                    facts
                        .unheld
                        .iter()
                        .map(|entry| LifecyclePathChangeDto {
                            path: String::from_utf8_lossy(&entry.path).replace('\\', "/"),
                            change: match entry.change {
                                PathChange::Added => LifecyclePathChangeKind::Added,
                                PathChange::Modified => LifecyclePathChangeKind::Modified,
                                PathChange::Deleted => LifecyclePathChangeKind::Deleted,
                            },
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    });
    Ready {
        expected: preview.expectation().to_bytes(),
        branch: preview.context_branch().to_owned(),
        primary_branch: preview.primary_branch().to_owned(),
        remote: preview.publication_remote().map(str::to_owned),
        context_oid: preview.context_oid(),
        primary_oid: preview.primary_oid(),
        // As `ClosureObservation::needs_context_child`: a contained context
        // is never synchronized, and the checkpoint is never pushed. A new
        // branch has no context to synchronize, and nothing else of a branch
        // record is contained: it integrates nothing.
        context_child: preview.publication_remote().is_some()
            && match &record {
                Some(record) => record.action != ClosureAction::NewBranch,
                None => preview.integration() != PlannedIntegration::AlreadyContained,
            },
        changes: preview.changes().to_vec(),
        integration: preview.integration(),
        publication: preview.publication(),
        cleanup: preview.cleanup(),
        closure: Some(preview.closure().clone()),
        record,
        pull_request: None,
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

/// A closure verb's answer as the binding reads it: how far it got, how a
/// branch record answered, or what stopped it.
fn called_of(result: Result<TicketClosureResult, TicketClosureError>) -> Result<Called, Stop> {
    match result {
        Ok(TicketClosureResult::Complete(outcome)) => {
            Ok(Called::Reached(closure_reached(&outcome, false)))
        }
        Ok(TicketClosureResult::IndexPending(pending)) => Ok(Called::Reached(closure_reached(
            &pending.authoritative,
            true,
        ))),
        Ok(TicketClosureResult::Branch(outcome)) => Ok(Called::Branch {
            outcome,
            index_pending: false,
        }),
        Ok(TicketClosureResult::BranchIndexPending(pending)) => Ok(Called::Branch {
            outcome: pending.authoritative,
            index_pending: true,
        }),
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
                    if preview.pull_request().is_none()
                        && preview.integration() == PlannedIntegration::MergeCommit
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
                DocumentPromotionPreflight::PrimaryHolds => Observed::PrimaryHoldsDocument,
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
    ) -> Option<Result<Called, Stop>> {
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
                Ok(Called::Reached(promotion_reached(&outcome, false)))
            }
            Ok(DocumentPromotionResult::IndexPending(pending)) => Ok(Called::Reached(
                promotion_reached(&pending.authoritative, true),
            )),
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
    ) -> Option<Result<Called, Stop>> {
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
        Some(called_of(result))
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

impl LifecycleVerb for Withdraw {
    const COMMAND: Command = Command::TicketWithdrawClosure;
    const KIND: AuthoringKind = AuthoringKind::Ticket;
    const DTO_KIND: ItemDtoKind = ItemDtoKind::Ticket;
    // As the close's: it makes the preview look up the ticket's slug. A branch
    // record's answers never read a made-ready report.
    const REPORTS_MADE_READY: bool = true;

    fn observe(
        service: &RepositoryService,
        root: &Path,
        id: &ItemId,
    ) -> Result<Observed, RepositoryError> {
        // The configuration first, before the domain looks at anything: a
        // withdraw's children publish, so with no publication remote there is
        // nothing to offer, and that answer comes ahead of the hold (R-33,
        // decision 33).
        if let ConfigurationInspection::Valid(configuration) =
            read_configuration_for(root, RepositoryOperation::RepositorySnapshot)?
            && configuration.publication_remote.is_none()
        {
            return Ok(Observed::PublicationRemoteRequired);
        }
        Ok(
            match service.preview_closure_withdrawal(PreviewTicketClosureRequest {
                root: root.to_owned(),
                item_id: id.clone(),
                confirmed_identity: None,
            })? {
                ClosureWithdrawalPreflight::Ready(preview) => {
                    Observed::Ready(Box::new(ready_of_closure(&preview)))
                }
                ClosureWithdrawalPreflight::Blocked {
                    blockers,
                    identity_configuration: _,
                } => Observed::Blocked(blockers.into_iter().map(closure_blocker).collect()),
                ClosureWithdrawalPreflight::NothingToWithdraw => Observed::NothingToWithdraw,
                ClosureWithdrawalPreflight::PublicationRemoteRequired => {
                    Observed::PublicationRemoteRequired
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
    ) -> Option<Result<Called, Stop>> {
        let result = session.withdraw_closure(
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
        Some(called_of(result))
    }

    fn parent(
        service: &RepositoryService,
        root: &Path,
        operation: OperationId,
    ) -> Result<Option<ParentView<LifecycleBlocker>>, RepositoryError> {
        Close::parent(service, root, operation)
    }

    fn holder(
        db: &rusqlite::Connection,
        repository_id: i64,
        id: &ItemId,
    ) -> Result<Option<OperationId>, ReadError> {
        Close::holder(db, repository_id, id)
    }
}

/// The code a branch record's answer gives (section 9.3). A reconfirmation
/// the consent no longer holds for answers as an external change does.
pub(super) fn branch_code(answer: BranchAnswer) -> ResultCode {
    match answer {
        BranchAnswer::Merged
        | BranchAnswer::Published
        | BranchAnswer::WithdrawnSince
        | BranchAnswer::Withdrawn => ResultCode::Ok,
        BranchAnswer::Existing | BranchAnswer::KeptPublished | BranchAnswer::AlreadyClosing => {
            ResultCode::AlreadyClosing
        }
        BranchAnswer::ClosedElsewhere
        | BranchAnswer::TicketClosed
        | BranchAnswer::AlreadyClosed
        | BranchAnswer::ContextTicketClosed
        | BranchAnswer::WithdrawClosed
        | BranchAnswer::ClosedSince => ResultCode::TicketClosed,
        BranchAnswer::AlreadyApplied => ResultCode::AlreadyApplied,
        BranchAnswer::RemoteBranchDeleted | BranchAnswer::ContextRemoteBranchDeleted => {
            ResultCode::RemoteBranchDeleted
        }
        // The code a direct close's `context_changed` cleanup stop answers.
        BranchAnswer::RemoteChanged
        | BranchAnswer::ContextHistoryUnresolved
        | BranchAnswer::ContextChanged => ResultCode::RecoveryRequired,
        BranchAnswer::ExternalChange | BranchAnswer::ReconfirmationRequired => {
            ResultCode::ExternalChange
        }
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
