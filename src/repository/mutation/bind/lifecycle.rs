//! `document promote` and `ticket close`: one binding over the domain's one
//! lifecycle runner, generic over a small verb.
//!
//! **The preview** is the domain's: it contacts no remote and takes no lease.
//! `Ready` gives a confirmation whose observation digest binds the domain's
//! expectation, which binds everything the preview shows. `Blocked` gives no
//! confirmation and answers with every blocker. `Binding::prepare` observes
//! the same way, so a request whose state has moved since its preview is
//! `external_change` with no record. The one thing it answers itself is an
//! identity removed since the preview, alone, since a new preview would say the
//! same.
//!
//! **A run** calls the domain under the request's operation ID, with no host
//! approval and no confirmed identity. What is left of the
//! request is read from the parent record, not the journal, because a parent
//! usually has no journal row: no parent, the request is deleted and
//! a new prepare follows, unless a call entered again met a failure that
//! passes by itself; a parent a consent refusal ended or a superseded one
//! answers from what it holds, and that is stored; a complete one, or one
//! unfinished, leaves the request accepted, and never `Stopped{Absent}`.
//!
//! **Entered again,** `Confirmed::row` reads the same record. An
//! unfinished or complete parent is called again with the record's own
//! expectation and nothing compared, since the request's own work moved the
//! state. A parent that ended answers from what it holds. With no parent, what
//! is there is compared with what was previewed.
//!
//! **Cancellation.** The domain's safe points read the call's token and the
//! request's flag. A run that stops for either answers `cancelled` and its
//! request stays accepted, with its flag taken, so that the same request
//! continues.
//!
//! **`operation resume`** enters a parent no request record holds through
//! `Confirmed::row` and `Binding::rerun`, and decides its own record.
//!
//! **The made-ready report** is a closure's, read when an answer is made, so
//! each retry reads again and a finished request's stored envelope keeps the
//! last. A promotion reports none, by null, and reads nothing.
//!
//! **Identity.** A promotion's preview has no blocker for it, so a ready
//! preview that plans a merge commit is answered as blocked here, last, with
//! nothing else in the way. A closure's preview names it itself.

use std::{marker::PhantomData, path::Path, path::PathBuf};

use crate::{
    canonical::ItemId,
    results::{
        CheckpointEffect, CleanupEffect, DiscoveryEffect, Effects, IntegrationEffect,
        PublicationEffect, RecoveryAction, RecoveryActionKind, ResultCode, Scope, WriteEffect,
    },
};

use super::{
    super::{
        super::{
            AuthoringKind, ClosureRecording, OperationFamily, OperationId, PlannedClosure,
            PlannedIntegration, PlannedPublication, PromotionCleanup, PromotionCleanupPending,
            PromotionIntegration, PromotionPublication, PromotionPublicationPending,
            PromotionRemoteDeletionCategory, PromotionSynchronizationCategory,
            RemoteOperationSafePoint, RepositoryOperation, RepositoryService,
            promotion::{
                ParentView,
                lifecycle::ParentStep,
                lifecycle_record::{ChildRole, Refusal},
                record::{IntegrationKind, PublicationCategory},
                run::{RunEvent, RunObserver},
            },
            recovery::JournalRow,
        },
        command::Command,
        confirm::{self, Row, Seen},
        context::context_names,
        dto::{
            LifecycleBlockedDto, LifecycleBlocker, LifecycleCleanupState, LifecycleIntegrationKind,
            LifecycleMadeReadyDto, LifecycleMadeReadyState, LifecycleMutationDto,
            LifecycleMutationType, LifecyclePlannedCleanupDto, LifecyclePlannedClosureDto,
            LifecyclePlannedIntegration, LifecyclePlannedPublication, LifecyclePreviewDto,
            LifecyclePublicationState, LifecycleRecording, MutationDataDto, PreviewDetailsDto,
            PreviewEffect,
        },
        evidence::{self, Position},
        identity::{IntentDigestBuilder, RequestId, ScopeKey},
        observe::{Observation, ObservationDigest},
        outcome,
        progress::{ProgressStage, Session, Subject},
        records,
    },
    Accepted, Answer, Binding, CallContext, Confirmed, EndedWithoutCheckpoint, Entered, Place,
    Prepared, Previewed, Ran, Standing, primary_position, refused, resolve_root,
};

#[path = "lifecycle_verb.rs"]
mod verb;
pub(in super::super) use verb::{Close, LifecycleVerb, Promote};
pub use verb::{DocumentPromoteInput, TicketCloseInput};
use verb::{Observed, Reached, Stop, cleanup_pending, path_change, publication_pending};

pub(in super::super) struct LifecycleBinding<V> {
    root: PathBuf,
    id: ItemId,
    /// The expectation the request consents to, as `Binding::prepare`
    /// observed it. `None` for a state that is blocked now, which the
    /// boundary refuses before anything runs.
    expected: Option<[u8; 32]>,
    /// Whether what this call observed, and consented to, is a closure the
    /// ticket already holds: its run records nothing, and reports no
    /// `recording` (R-15). `false` when this call observed nothing.
    closure_exists: bool,
    verb: PhantomData<V>,
}

impl From<DocumentPromoteInput> for LifecycleBinding<Promote> {
    fn from(input: DocumentPromoteInput) -> Self {
        let DocumentPromoteInput { root, id } = input;
        Self {
            root,
            id,
            expected: None,
            closure_exists: false,
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
            closure_exists: false,
            verb: PhantomData,
        }
    }
}

/// Whether `observed` is ready with a closure the ticket already holds (R-15).
fn closure_exists(observed: &Observed) -> bool {
    matches!(
        observed,
        Observed::Ready(ready) if matches!(ready.closure, Some(PlannedClosure::Existing { .. }))
    )
}

/// A run's steps as the request's progress stages (D5 decision 11). The
/// context's synchronization reports `connecting` as it starts, and its
/// fetch, local update and push as a share's do (U-4); the primary's reports
/// `publishing` once, and none of its safe points.
struct Stages<'a> {
    session: &'a dyn Session,
    subject: Subject,
    /// A closure the ticket already holds is not recorded again (R-15).
    closure_exists: bool,
}

impl RunObserver for Stages<'_> {
    fn event(&mut self, event: RunEvent) {
        let stage = match event {
            RunEvent::ChildStarting(ChildRole::Context) => ProgressStage::Connecting,
            RunEvent::ChildStarting(ChildRole::Primary) => ProgressStage::Publishing,
            RunEvent::ChildSafePoint(ChildRole::Context, point) => match point {
                RemoteOperationSafePoint::BeforeFetch => ProgressStage::Fetching,
                RemoteOperationSafePoint::BeforeLocalUpdate => ProgressStage::UpdatingLocal,
                RemoteOperationSafePoint::BeforePush => ProgressStage::Pushing,
                RemoteOperationSafePoint::BeforeTransport
                | RemoteOperationSafePoint::AfterAdvertisement
                | RemoteOperationSafePoint::BetweenObservations
                | RemoteOperationSafePoint::BeforeBatchCommit
                | RemoteOperationSafePoint::AfterBatchCommit
                | RemoteOperationSafePoint::BeforeLocalMutation
                | RemoteOperationSafePoint::AfterFetch
                | RemoteOperationSafePoint::AfterLocalUpdate
                | RemoteOperationSafePoint::AfterPushReturn
                | RemoteOperationSafePoint::AfterPushVerification
                | RemoteOperationSafePoint::BeforeDiscovery => return,
            },
            RunEvent::ChildSafePoint(ChildRole::Primary, _) => return,
            RunEvent::Recording if self.closure_exists => return,
            RunEvent::Recording => ProgressStage::Recording,
            RunEvent::Integrating => ProgressStage::Integrating,
            RunEvent::CleaningUp => ProgressStage::CleaningUp,
        };
        self.session
            .report(&self.subject.event(stage, Effects::not_requested()));
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

/// Where a made-ready report comes from, which says what it can answer.
enum MadeReadyFrom {
    /// Not yet on primary.
    BeforeIntegration,
    /// A cleanup only: this operation put nothing on primary.
    CleanupOnly,
    /// The domain's own `IndexPending`: its refresh is yet to run.
    IndexPending,
    /// The domain's outcome. `finished` is an answer that is the request's
    /// last, so a read that fails will not be made again.
    Outcome { finished: bool },
    /// A stop past integration, with no outcome from the domain. The request
    /// is owed, and an index that reads the ticket as open says nothing yet.
    Stop,
}

/// `made_ready` and `made_ready_state`, both null for a promotion.
type MadeReadyReport = (
    Option<Vec<LifecycleMadeReadyDto>>,
    Option<LifecycleMadeReadyState>,
);

thread_local! {
    /// What a test stages for the next report's read. A boundary runs on the
    /// thread its caller called it on. It is no field of the service, as the
    /// request hook is, so that the service holds no second test-only field for
    /// one read, and what it stages needn't be `Send`.
    static MADE_READY_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
}

impl RepositoryService {
    /// Has `hook` run immediately before the next made-ready report's read on
    /// this thread, whichever service makes it: after the domain has
    /// answered, so that a test can fail a read which that answer did not
    /// depend on. The hook is per thread, not per service, and the report
    /// that runs it consumes it: it runs once.
    #[doc(hidden)]
    pub fn set_made_ready_hook_for_testing(hook: impl FnOnce() + 'static) {
        MADE_READY_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
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
    /// Reads the repository and what the domain's preview says of the item:
    /// the repository's root, its registration's ID, and the preview.
    fn observe(&self, service: &RepositoryService) -> Result<(PathBuf, i64, Observed), Answer> {
        let repository = resolve_root(service, &self.root)?;
        let root = repository.root().to_owned();
        let observed = V::observe(service, &root, &self.id).map_err(refused)?;
        Ok((root, repository.registration_id(), observed))
    }

    /// Decision 33: while an unfinished parent that a new request can't
    /// supersede holds the item, a prepare is refused, and the way on is that
    /// parent's: the request whose accepted record holds its operation, or a
    /// resume of the operation when no record does.
    fn held(&self, service: &RepositoryService, repository_id: i64) -> Result<(), Answer> {
        let key = ScopeKey::for_repository(&self.root);
        // The parent and the request holding it, in one read.
        let held = service
            .read_session(RepositoryOperation::Read, |db| {
                let Some(operation) = V::holder(db, repository_id, &self.id)? else {
                    return Ok(None);
                };
                let Some(key) = &key else {
                    return Ok(Some((operation, None)));
                };
                let holder = records::operation_or_parent_holder(
                    db,
                    key.as_str(),
                    repository_id,
                    operation,
                )?;
                Ok(Some((operation, Some(holder))))
            })
            .map_err(|error| Answer::of_read(&error))?;
        let Some((operation, holder)) = held else {
            return Ok(());
        };
        let holder = holder.ok_or_else(|| Answer::stopped(ResultCode::InvalidPath))?;
        let action = match holder.and_then(|holder| RequestId::parse(&holder).ok()) {
            Some(request) => outcome::same_request(request),
            None => RecoveryAction::for_operation(
                RecoveryActionKind::OperationResume,
                operation.to_string(),
                [],
            ),
        };
        Err(Answer {
            recovery: vec![action],
            ..Answer::stopped(ResultCode::RecoveryRequired)
        })
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
            made_ready: V::REPORTS_MADE_READY.then(Vec::new),
            made_ready_state: V::REPORTS_MADE_READY.then_some(LifecycleMadeReadyState::NotComputed),
        }
    }

    /// Data that names the item and nothing of how far it got.
    pub(super) fn naming_its_item(&self) -> MutationDataDto {
        MutationDataDto::Lifecycle(Box::new(self.data()))
    }

    /// The made-ready report, read now. A read that fails or is incomplete is
    /// `final` only for an answer that is: an operation still owed is read
    /// again by its next call, so it says that instead.
    fn made_ready(
        &self,
        service: &RepositoryService,
        root: &Path,
        from: MadeReadyFrom,
    ) -> MadeReadyReport {
        if !V::REPORTS_MADE_READY {
            return (None, None);
        }
        let none = |state| (Some(Vec::new()), Some(state));
        match from {
            MadeReadyFrom::BeforeIntegration => {
                return none(LifecycleMadeReadyState::NotComputed);
            }
            MadeReadyFrom::CleanupOnly => return none(LifecycleMadeReadyState::NotApplicable),
            MadeReadyFrom::IndexPending => return none(LifecycleMadeReadyState::IndexPending),
            MadeReadyFrom::Outcome { finished: _ } | MadeReadyFrom::Stop => {}
        }
        if let Some(hook) = MADE_READY_HOOK.with(std::cell::RefCell::take) {
            hook();
        }
        let read = resolve_root(service, root)
            .ok()
            .and_then(|repository| service.tickets_made_ready(&repository, &self.id).ok());
        let stop = matches!(from, MadeReadyFrom::Stop);
        match read {
            Some(read) if read.complete && (read.target_closed || !stop) => (
                Some(
                    read.tickets
                        .into_iter()
                        .map(|ticket| LifecycleMadeReadyDto {
                            id: ticket.id,
                            slug: ticket.slug,
                        })
                        .collect(),
                ),
                Some(LifecycleMadeReadyState::Current),
            ),
            _ if matches!(from, MadeReadyFrom::Outcome { finished: true }) => {
                none(LifecycleMadeReadyState::IndexUnavailableFinal)
            }
            _ => none(LifecycleMadeReadyState::IndexPending),
        }
    }

    fn reached_data(
        &self,
        service: &RepositoryService,
        root: &Path,
        reached: &Reached,
    ) -> LifecycleMutationDto {
        let cleanup_only = reached.recording == Some(ClosureRecording::Existing)
            && matches!(
                reached.integration,
                PromotionIntegration::AlreadyContained { oid: _ }
            );
        let (made_ready, made_ready_state) = self.made_ready(
            service,
            root,
            if cleanup_only {
                MadeReadyFrom::CleanupOnly
            } else if reached.index_pending {
                MadeReadyFrom::IndexPending
            } else {
                MadeReadyFrom::Outcome {
                    finished: reached_code(reached) == ResultCode::Ok,
                }
            },
        );
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
            made_ready,
            made_ready_state,
            ..self.data()
        }
    }

    /// The answer to an operation that reached primary. What it left to do
    /// is the same request's to continue, whatever stopped it.
    fn reached(&self, service: &RepositoryService, accepted: &Accepted, reached: &Reached) -> Ran {
        let code = reached_code(reached);
        // A code's own action for each of these would be wrong: a hand-off
        // is continued by the request that recomputes what it reports, and
        // a host category carries no authority to approve.
        let recovery: Vec<RecoveryAction> = match code {
            ResultCode::DiscoveryPending => vec![outcome::same_request(accepted.request_id)],
            ResultCode::HostApprovalRequired => {
                then_inspect(outcome::same_request(accepted.request_id), accepted)
            }
            _ => Vec::new(),
        };
        Ran {
            answer: Answer {
                effects: reached_effects(reached),
                recovery,
                ..Answer::stopped(code).with_data(MutationDataDto::Lifecycle(Box::new(
                    self.reached_data(service, &accepted.root(), reached),
                )))
            },
            standing: if code == ResultCode::Ok {
                Standing::Final
            } else {
                Standing::Owed
            },
        }
    }

    /// The data of an answer that names what stands in the way.
    fn blocked_data(&self, blockers: Vec<LifecycleBlocker>) -> MutationDataDto {
        MutationDataDto::Lifecycle(Box::new(LifecycleMutationDto {
            blockers,
            ..self.data()
        }))
    }

    /// The answer to an operation that stopped. What it left is the parent's
    /// to say: it is read after the call, and another call of the request may
    /// have moved it since this one's stop.
    fn stopped(
        &self,
        service: &RepositoryService,
        accepted: &Accepted,
        stop: Stop,
        entered_again: bool,
    ) -> Ran {
        let root = accepted.root();
        let primary = accepted.position.base_ref.as_deref();
        let (code, blockers, offers_nothing) = stop_answer(&stop);
        let parent = match V::parent(service, &root, accepted.operation_id) {
            // Nothing can be said of what it left: the record is left.
            Err(_) => {
                return Ran {
                    answer: Answer::stopped(code),
                    standing: Standing::Stopped { journal: None },
                };
            }
            Ok(parent) => parent,
        };
        let Some(view) = parent else {
            // A call entered again may be meeting the call it came after,
            // which has yet to begin its record: the record is that call's
            // too, and is not deleted under it.
            let (recovery, standing) = if entered_again && outcome::passes_by_itself(code) {
                (Vec::new(), Standing::Owed)
            } else {
                (
                    no_parent_recovery(code),
                    Standing::Stopped {
                        journal: Some(JournalRow::Absent),
                    },
                )
            };
            return Ran {
                answer: Answer {
                    recovery,
                    data: blockers.map(|blockers| self.blocked_data(blockers)),
                    offers_nothing,
                    ..Answer::stopped(code)
                },
                standing,
            };
        };
        if let Some(answer) = self.ended_answer(service, &root, primary, &view) {
            return Ran {
                answer,
                standing: Standing::Final,
            };
        }
        let effects = parent_effects(&V::KIND, service, &root, primary, &view);
        let report = self.made_ready(
            service,
            &root,
            if !parent_integrated(&root, primary, &view) {
                MadeReadyFrom::BeforeIntegration
            } else if view.checkpoint.is_none()
                && view.integration_kind == Some(IntegrationKind::AlreadyContained)
            {
                MadeReadyFrom::CleanupOnly
            } else {
                MadeReadyFrom::Stop
            },
        );
        // This call's stop is not the request's last word: the next call gets
        // the outcome the parent stored.
        if view.step == ParentStep::Complete {
            return Ran {
                answer: Answer {
                    effects,
                    data: self.stop_data(
                        None,
                        false,
                        &view,
                        CheckpointEffect::NotRequested,
                        report,
                    ),
                    recovery: vec![outcome::same_request(accepted.request_id)],
                    ..Answer::stopped(code)
                },
                standing: Standing::Owed,
            };
        }
        // A child's stop names the item for both verbs, as its route out is
        // the item's: the human answer prints it from this data.
        let child = matches!(stop, Stop::ContextSynchronization(_));
        let blockers = self.named_blockers(service, &root, &stop, blockers);
        Ran {
            answer: Answer {
                data: self.stop_data(blockers, child, &view, effects.checkpoint, report),
                effects,
                recovery: standing_recovery(code, accepted),
                offers_nothing,
                ..Answer::stopped(code)
            },
            standing: Standing::Owed,
        }
    }

    /// What a stop names as standing in the way. A refused merge or a child's
    /// conflict is the repository's to name, and a preview reads it with
    /// nothing changed.
    fn named_blockers(
        &self,
        service: &RepositoryService,
        root: &Path,
        stop: &Stop,
        blockers: Option<Vec<LifecycleBlocker>>,
    ) -> Option<Vec<LifecycleBlocker>> {
        let names_its_state = match stop {
            Stop::RecoveryRequired => true,
            Stop::ContextSynchronization(error) => {
                outcome::synchronization_error_code(error) == ResultCode::RecoveryRequired
            }
            _ => false,
        };
        match (blockers, names_its_state) {
            (Some(blockers), _) => Some(blockers),
            (None, true) => match V::observe(service, root, &self.id) {
                Ok(Observed::Blocked(blockers)) => Some(blockers),
                Ok(Observed::Ready(_)) | Err(_) => None,
            },
            (None, false) => None,
        }
    }

    /// The data of a stop: its blockers, with the checkpoint it wrote, and its
    /// report. A closure's stop reports; a promotion's has its blockers to
    /// say, and names its item when its child stopped (`child`).
    fn stop_data(
        &self,
        blockers: Option<Vec<LifecycleBlocker>>,
        child: bool,
        view: &ParentView<LifecycleBlocker>,
        checkpoint: CheckpointEffect,
        (made_ready, made_ready_state): MadeReadyReport,
    ) -> Option<MutationDataDto> {
        let data = match blockers {
            Some(blockers) => {
                let written = view
                    .checkpoint
                    .filter(|_| checkpoint == CheckpointEffect::Committed);
                LifecycleMutationDto {
                    blockers,
                    recording: written.map(|_| LifecycleRecording::Record),
                    checkpoint_oid: written.map(|oid| oid.to_string()),
                    ..self.data()
                }
            }
            None if V::REPORTS_MADE_READY || child => self.data(),
            None => return None,
        };
        Some(MutationDataDto::Lifecycle(Box::new(LifecycleMutationDto {
            made_ready,
            made_ready_state,
            ..data
        })))
    }

    /// The one domain call, under the request's operation ID, whose steps
    /// are reported to the call's sink.
    fn call(&self, call: &CallContext<'_>, accepted: &Accepted, entered_again: bool) -> Ran {
        let mut stages = Stages {
            session: call.session,
            subject: Subject::of(self, accepted),
            closure_exists: self.closure_exists,
        };
        let called = self.expected.and_then(|expected| {
            V::call(
                call.session,
                call.service,
                accepted.root(),
                accepted.operation_id,
                self.id.clone(),
                expected,
                accepted.request_id,
                &mut stages,
            )
        });
        let ran = match called {
            Some(Ok(reached)) => self.reached(call.service, accepted, &reached),
            Some(Err(stop)) => {
                let stop = self.interrupted_as_asked(call, accepted, stop);
                self.stopped(call.service, accepted, stop, entered_again)
            }
            // No consent was observed, or the session has no credentials:
            // nothing was called, and the record is left as it is.
            None => Ran {
                answer: Answer::stopped(ResultCode::InternalError),
                standing: Standing::Stopped { journal: None },
            },
        };
        // The cancellation is answered, so the next call starts clean. A
        // call that dies before this, or a take that fails, leaves the flag,
        // and the next call answers `cancelled` once more.
        if ran.answer.code == ResultCode::Cancelled {
            let _ = call.service.take_cancellation(accepted.request_id);
        }
        ran
    }

    /// `Interrupted` is `cancelled` once the call, the request or the parent
    /// was asked to stop. Anything else that interrupts it is no cancellation.
    fn interrupted_as_asked(
        &self,
        call: &CallContext<'_>,
        accepted: &Accepted,
        stop: Stop,
    ) -> Stop {
        if !matches!(stop, Stop::Interrupted) {
            return stop;
        }
        let asked = call.session.cancelled()
            || call
                .service
                .cancellation_requested(accepted.request_id)
                .unwrap_or(false)
            || V::parent(call.service, &accepted.root(), accepted.operation_id)
                .is_ok_and(|parent| parent.is_some_and(|parent| parent.cancel_requested));
        if asked { Stop::Cancelled } else { stop }
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
        Answer::stopped(blocked_code(&blockers)).with_data(self.blocked_data(blockers))
    }

    /// A parent that has ended answers its request from what it holds.
    fn ended_answer(
        &self,
        service: &RepositoryService,
        root: &Path,
        primary: Option<&str>,
        parent: &ParentView<LifecycleBlocker>,
    ) -> Option<Answer> {
        let effects = parent_effects(&V::KIND, service, root, primary, parent);
        let answer = match &parent.refusal {
            Some(Refusal::Blocked(blockers)) => Answer::stopped(blocked_code(blockers))
                .with_data(self.blocked_data(blockers.clone())),
            Some(Refusal::ExternalChange | Refusal::ReconfirmationRequired) => {
                Answer::stopped(ResultCode::ExternalChange)
            }
            // The record's own check makes this unreachable: a parent that
            // says it was abandoned is not run, whatever it holds.
            None if parent.step == ParentStep::Abandoned => {
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
    /// phases and no checkpoint step.
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
            closure_exists: _,
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
    /// exception: an identity removed since the preview, alone, is
    /// answered as what it is, since a new preview would only say the same.
    /// The request is placed on primary, whose reachability a replay checks.
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        let (root, _, observed) = self.observe(call.service)?;
        if let Observed::Blocked(blockers) = &observed
            && matches!(blockers.as_slice(), [LifecycleBlocker::IdentityRequired])
        {
            return Err(Answer::stopped(ResultCode::IdentityRequired)
                .with_data(self.blocked_data(blockers.clone())));
        }
        let position = primary_position(&root)?;
        self.expected = match &observed {
            Observed::Ready(ready) => Some(ready.expected),
            Observed::Blocked(_) => None,
        };
        self.closure_exists = closure_exists(&observed);
        Ok(Prepared {
            family: Some(OperationFamily::Remote),
            position,
            expected: None,
            observation: Some(digest_of(V::COMMAND, &observed).to_stored()),
        })
    }

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        self.call(call, accepted, false)
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
        self.call(call, accepted, true)
    }
}

impl<V: LifecycleVerb> Confirmed for LifecycleBinding<V> {
    fn preview(&mut self, call: &CallContext<'_>) -> Result<Previewed, Answer> {
        let (root, repository_id, observed) = self.observe(call.service)?;
        self.held(call.service, repository_id)?;
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
        let slug = if V::REPORTS_MADE_READY {
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

    /// The parent record says where the request stands, as the module's
    /// comment says. A parent that goes on is called with the expectation it
    /// was begun with, not one read now.
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
        let primary = accepted.position.base_ref.as_deref();
        if let Some(answer) = self.ended_answer(call.service, &root, primary, &parent) {
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
        let (root, _, observed) = self.observe(call.service).map_err(|answer| Ran {
            answer,
            standing: Standing::Stopped { journal: None },
        })?;
        let found = digest_of(V::COMMAND, &observed);
        self.closure_exists = closure_exists(&observed);
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
        Ok(Some(parent)) => parent_effects(
            &V::KIND,
            service,
            &root,
            accepted.position.base_ref.as_deref(),
            &parent,
        ),
        Ok(None) | Err(_) => Effects::not_requested(),
    }
}

/// Whether primary holds what the parent integrates, read from its step: a
/// commit named by the record is not yet there, until primary holds
/// it, which a kill between the ref and the record leaves at `integrating`. A
/// superseded parent had reached cleanup, and only that is ever superseded.
fn parent_integrated(
    root: &Path,
    primary: Option<&str>,
    parent: &ParentView<LifecycleBlocker>,
) -> bool {
    match parent.step {
        ParentStep::Integrated
        | ParentStep::Published
        | ParentStep::PublicationPending
        | ParentStep::RemoteDeleted
        | ParentStep::WorktreeRemoving
        | ParentStep::WorktreeRemoved
        | ParentStep::BranchDeleted
        | ParentStep::Complete
        | ParentStep::Superseded => true,
        ParentStep::Integrating => parent
            .integration
            .is_some_and(|commit| evidence::still_reachable(root, primary, commit)),
        ParentStep::Confirmed | ParentStep::ContextSynchronized | ParentStep::Abandoned => false,
    }
}

/// The commit a child's push is known to have put on the remote.
fn landed_push(
    service: &RepositoryService,
    scope: &ScopeKey,
    child: OperationId,
) -> Option<git2::Oid> {
    service
        .stored_push(scope, child)
        .ok()
        .flatten()
        .filter(|stored| stored.accepted)
        .map(|stored| stored.commit)
}

/// The effects a parent shows: its own, by its step, and what its children made
/// that stays whatever stopped it. Before integration that is the
/// context child's merge on the context's branch and its landed push. After it,
/// the primary child's landed push, which the parent may not have recorded yet.
/// Nothing is claimed of a row that can't be read.
fn parent_effects(
    kind: &AuthoringKind,
    service: &RepositoryService,
    root: &Path,
    primary: Option<&str>,
    parent: &ParentView<LifecycleBlocker>,
) -> Effects {
    let integrated = parent_integrated(root, primary, parent);
    let scope = ScopeKey::for_repository(root);
    // A checkpoint is written once the context's branch holds it, or primary
    // once that branch is gone: never whatever is checked out.
    let branch = context_names(root, kind, &parent.item_id.to_string()).branch_ref;
    let branch_stands = git2::Repository::open(root)
        .is_ok_and(|repository| repository.refname_to_id(&branch).is_ok());
    let holder = if branch_stands {
        Some(branch.as_str())
    } else {
        primary
    };
    let checkpoint = if parent
        .checkpoint
        .is_some_and(|commit| evidence::still_reachable(root, holder, commit))
    {
        CheckpointEffect::Committed
    } else {
        CheckpointEffect::NotRequested
    };
    let discovery = || {
        if service
            .repository_snapshot(root)
            .is_ok_and(|snapshot| !snapshot.refresh_required)
        {
            DiscoveryEffect::Current
        } else {
            DiscoveryEffect::Pending
        }
    };
    let [context_child, primary_child] = parent.children;
    if !integrated {
        let on_context = context_child.zip(scope.as_ref());
        let pushed = on_context.and_then(|(child, scope)| landed_push(service, scope, child));
        let merged = on_context
            .and_then(|(child, scope)| super::sync::merge_made_on(service, scope, child, &branch));
        if pushed.is_none() && merged.is_none() {
            return Effects {
                checkpoint,
                ..Effects::not_requested()
            };
        }
        return Effects {
            checkpoint,
            discovery: discovery(),
            publication: if pushed.is_some() {
                PublicationEffect::Published
            } else {
                PublicationEffect::NotRequested
            },
            integration: if merged.is_some() {
                IntegrationEffect::Complete
            } else {
                IntegrationEffect::NotRequested
            },
            commit_oid: pushed.or(merged).map(|oid| oid.to_string()),
            ..Effects::not_requested()
        };
    }
    Effects {
        write: WriteEffect::NotRequested,
        checkpoint,
        discovery: discovery(),
        publication: match parent.publication {
            Some(PublicationCategory::Published) => PublicationEffect::Published,
            Some(PublicationCategory::AlreadyCurrent) => PublicationEffect::Current,
            _ if primary_child
                .zip(scope.as_ref())
                .and_then(|(child, scope)| landed_push(service, scope, child))
                .is_some() =>
            {
                PublicationEffect::Published
            }
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
/// may follow a written checkpoint, so it never reads as a consent refusal.
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
        Stop::Cancelled => (ResultCode::Cancelled, None, false),
        Stop::Interrupted | Stop::RecoveryRequired => (ResultCode::RecoveryRequired, None, false),
        Stop::OperationMismatch => (ResultCode::RequestMismatch, None, true),
    }
}

/// With no parent the request is deleted, so a code whose own action is to
/// send it again offers a new preview. Any other code's action stands, and
/// `internal_error` has none.
fn no_parent_recovery(code: ResultCode) -> Vec<RecoveryAction> {
    if outcome::passes_by_itself(code) {
        vec![outcome::prepare_again()]
    } else {
        Vec::new()
    }
}

/// `first`, then a look at the repository.
fn then_inspect(first: RecoveryAction, accepted: &Accepted) -> Vec<RecoveryAction> {
    let mut actions = vec![first];
    actions.extend(outcome::inspect_the_root_it_lies_in(&accepted.root()));
    actions
}

/// What a code would offer wrongly while a parent stands. A host stop carries
/// no authority to approve. A child's `remote_branch_deleted` or
/// `merge_required` is one the same request cannot get past, so it isn't
/// offered: its cancellation lets the child go, and the repository shows what
/// happened to the branch. Every other code's own actions follow
/// the same request.
fn standing_recovery(code: ResultCode, accepted: &Accepted) -> Vec<RecoveryAction> {
    match code {
        ResultCode::HostApprovalRequired | ResultCode::HostReplacementRequired => {
            then_inspect(outcome::same_request(accepted.request_id), accepted)
        }
        _ if outcome::never_gets_past(code) => {
            then_inspect(outcome::request_cancel(accepted.request_id), accepted)
        }
        _ => Vec::new(),
    }
}

/// Whether `active`, the repository's reservation, is held by a child of the
/// parent `operation` of a promotion's or a closure's request. For any other
/// command there is no such parent.
pub(in super::super) fn held_by_its_child(
    service: &RepositoryService,
    command: Option<Command>,
    root: &Path,
    operation: OperationId,
    active: OperationId,
) -> bool {
    let parent = match command {
        Some(Command::DocumentPromote) => Promote::parent(service, root, operation),
        Some(Command::TicketClose) => Close::parent(service, root, operation),
        _ => return false,
    };
    matches!(parent, Ok(Some(parent)) if parent.children.contains(&Some(active)))
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
#[path = "lifecycle_tests.rs"]
mod tests;
