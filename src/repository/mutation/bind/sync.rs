//! `item sync` and `repo sync`: the two commands that synchronize a
//! branch with the publication remote. `repo sync` synchronizes the
//! primary branch, `item sync` the editing branch of one item.
//!
//! Neither merges. Both call the sibling of `synchronize_remote` with
//! its refuse-divergence setting, so a branch whose local and remote
//! history have diverged is `merge_required` and nothing of a merge is
//! begun.
//!
//! Neither is an authoring write, and both answer `SaysNothing`. What
//! says a synchronization is done is its row in the remote journal and
//! what the domain's own reconciliation finds when it is called again
//! under the same operation ID: a row that ended with an outcome is
//! replayed, and one that did not is taken up with `restart`, which
//! asks the remote what it holds before it pushes anything. So every
//! re-entry makes the domain call, and none answers from the row alone.
//!
//! A synchronization holds the repository's reservation from its first
//! safe point until it ends with an outcome or is cancelled. One that
//! stops with an error in between keeps it, and its record stays
//! accepted: the same request takes it up again, and `request cancel`
//! followed by the same request ends it.
//!
//! This binding is also where a call's progress and cancellation meet a
//! running operation: at each safe point it reports a stage, and asks
//! the synchronization to stop when the call's token is cancelled or the
//! request's record has been asked to.

use std::path::PathBuf;

use crate::{
    canonical::ItemId,
    results::{DiscoveryEffect, Effects, PublicationEffect, ResultCode, Scope},
};

use super::{
    super::{
        super::{
            AuthoringKind, ConfigurationInspection, OperationFamily, RemoteOperationSafePoint,
            RepositoryOperation, RepositoryService, SafePointRequest, SynchronizationError,
            SynchronizationObserver, SynchronizationOutcome, SynchronizationResult,
            SynchronizationSettings, SynchronizationTarget, SynchronizeRemoteRequest,
            authoring_kind_segment, read_configuration_for,
            recovery::{FinalKind, JournalRow},
            transport::SshTransportErrorKind,
        },
        command::Command,
        context::context_names,
        dto::{MutationDataDto, SynchronizationMutationDto},
        evidence::Position,
        identity::IntentDigestBuilder,
        outcome,
        progress::{ProgressStage, Subject},
    },
    Accepted, Answer, Binding, CallContext, EndedWithoutCheckpoint, Entered, Place, Prepared, Ran,
    Standing, item_scope, refused, resolve_root,
};

/// The input of `item sync`: the item whose editing branch is
/// synchronized.
pub struct ItemSyncInput {
    pub root: PathBuf,
    pub kind: AuthoringKind,
    pub id: ItemId,
}

/// The input of `repo sync`: the repository whose primary branch is
/// synchronized.
pub struct RepoSyncInput {
    pub root: PathBuf,
}

pub(crate) struct SyncBinding {
    root: PathBuf,
    target: SynchronizationTarget,
}

impl From<ItemSyncInput> for SyncBinding {
    fn from(input: ItemSyncInput) -> Self {
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let ItemSyncInput { root, kind, id } = input;
        Self {
            root,
            target: SynchronizationTarget::Context { kind, item_id: id },
        }
    }
}

impl From<RepoSyncInput> for SyncBinding {
    fn from(input: RepoSyncInput) -> Self {
        let RepoSyncInput { root } = input;
        Self {
            root,
            target: SynchronizationTarget::Primary,
        }
    }
}

/// The bindings never merge.
const SETTINGS: SynchronizationSettings = SynchronizationSettings {
    refuse_divergence: true,
};

/// The stage a synchronization is at when it reaches `point`: what it is
/// about to do. `None` for a point that is a poll's and not a
/// synchronization's, which no synchronization reaches.
fn stage_at(point: RemoteOperationSafePoint) -> Option<ProgressStage> {
    match point {
        RemoteOperationSafePoint::BeforeFetch => Some(ProgressStage::Fetching),
        // What was fetched is compared with what is here.
        RemoteOperationSafePoint::AfterFetch => Some(ProgressStage::Observing),
        RemoteOperationSafePoint::BeforeLocalUpdate => Some(ProgressStage::UpdatingLocal),
        // What the remote would be pushed is asked of it.
        RemoteOperationSafePoint::AfterLocalUpdate => Some(ProgressStage::Observing),
        RemoteOperationSafePoint::BeforePush => Some(ProgressStage::Pushing),
        RemoteOperationSafePoint::AfterPushReturn => Some(ProgressStage::Verifying),
        RemoteOperationSafePoint::AfterPushVerification => Some(ProgressStage::Indexing),
        RemoteOperationSafePoint::BeforeTransport
        | RemoteOperationSafePoint::AfterAdvertisement
        | RemoteOperationSafePoint::BetweenObservations
        | RemoteOperationSafePoint::BeforeBatchCommit
        | RemoteOperationSafePoint::AfterBatchCommit
        | RemoteOperationSafePoint::BeforeLocalMutation
        | RemoteOperationSafePoint::BeforeDiscovery => None,
    }
}

/// What a synchronization has published so far, as an event or a result
/// that stopped gives it. `pushed` is the commit a push of which the
/// remote is known to have accepted.
fn effects_so_far(pushed: Option<git2::Oid>) -> Effects {
    match pushed {
        Some(commit) => Effects {
            publication: PublicationEffect::Published,
            discovery: DiscoveryEffect::Pending,
            commit_oid: Some(commit.to_string()),
            ..Effects::not_requested()
        },
        None => Effects {
            publication: PublicationEffect::Pending,
            ..Effects::not_requested()
        },
    }
}

/// The binding's observer of one domain call: it reports the stage of
/// each safe point, and asks the synchronization to stop when the call
/// or the request has been asked to.
struct Watch<'a> {
    call: &'a CallContext<'a>,
    accepted: &'a Accepted,
    subject: Subject,
    /// Whether this call saw its own push return: the remote accepted
    /// it. The row that then reaches `push_verified` does not say so by
    /// itself.
    pushed: bool,
}

impl Watch<'_> {
    /// The commit the remote is known to have accepted a push of: what
    /// the row says, or what this call saw. `None` also when the journal
    /// cannot be read: nothing is claimed on that.
    fn accepted_push(&self) -> Option<git2::Oid> {
        let stored = self
            .call
            .service
            .stored_push(&self.accepted.scope, self.accepted.operation_id)
            .ok()
            .flatten()?;
        (stored.accepted || self.pushed).then_some(stored.commit)
    }
}

impl SynchronizationObserver for Watch<'_> {
    fn safe_point(&mut self, point: RemoteOperationSafePoint) -> SafePointRequest {
        if point == RemoteOperationSafePoint::AfterPushReturn {
            self.pushed = true;
        }
        let session = self.call.session;
        // An event says what is published only once it is: before the
        // push has returned nothing is, and no journal is read.
        let effects = || effects_so_far(self.pushed.then(|| self.accepted_push()).flatten());
        if let Some(stage) = stage_at(point) {
            session.report(&self.subject.event(stage, effects()));
        }
        // The call's own token, and what `request cancel` asked of the
        // request from elsewhere, before or after the reservation
        // existed. The flag is cleared as it is passed on. A record that
        // cannot be read asks nothing.
        let asked = session.cancelled()
            || self
                .call
                .service
                .take_cancellation(self.accepted.request_id)
                .unwrap_or(false);
        if asked {
            session.report(&self.subject.event(ProgressStage::Stopping, effects()));
            SafePointRequest::Cancel
        } else {
            SafePointRequest::Continue
        }
    }
}

/// What a host that is not trusted presented, as the result's data.
fn host_data(error: &SynchronizationError) -> Option<MutationDataDto> {
    let SynchronizationError::Transport(error) = error else {
        return None;
    };
    let authority = error.authority.as_ref()?;
    let (presented, expected) = match &error.kind {
        SshTransportErrorKind::HostApprovalRequired { presented } => (presented, None),
        SshTransportErrorKind::HostReplacementRequired {
            expected,
            presented,
        } => (presented, Some(expected)),
        SshTransportErrorKind::ConfigurationInvalid
        | SshTransportErrorKind::PublicationRemoteMissing
        | SshTransportErrorKind::UsernameRequired
        | SshTransportErrorKind::NoSelectedKey
        | SshTransportErrorKind::KeyMissing
        | SshTransportErrorKind::KeyUnreadable
        | SshTransportErrorKind::KeySourceChanged
        | SshTransportErrorKind::SelectionChanged
        | SshTransportErrorKind::EndpointChanged
        | SshTransportErrorKind::KeyInvalidOrUnsupported
        | SshTransportErrorKind::KeyRejected
        | SshTransportErrorKind::UnlockCancelled
        | SshTransportErrorKind::ProviderUnavailable
        | SshTransportErrorKind::UnlockFailed
        | SshTransportErrorKind::HostTrustChanged
        | SshTransportErrorKind::HostVerificationUnavailable
        | SshTransportErrorKind::RegistryUnavailable
        | SshTransportErrorKind::RuntimeUninitialized
        | SshTransportErrorKind::TransportUnavailable
        | SshTransportErrorKind::RemoteUnavailable
        | SshTransportErrorKind::PushRejected
        | SshTransportErrorKind::ProtocolFailure => return None,
    };
    Some(MutationDataDto::Synchronization(
        SynchronizationMutationDto {
            host: authority.host.clone(),
            port: authority.port,
            algorithm: presented.algorithm.clone(),
            presented_fingerprint: presented.sha256.clone(),
            expected_fingerprint: expected.map(|expected| expected.sha256.clone()),
        },
    ))
}

impl SyncBinding {
    /// The branch the request synchronizes, as a full reference name:
    /// the item's editing branch, or `primary`.
    fn branch_ref(&self, root: &std::path::Path, primary: &str) -> String {
        match &self.target {
            SynchronizationTarget::Primary => format!("refs/heads/{primary}"),
            SynchronizationTarget::Context { kind, item_id } => {
                context_names(root, kind, &item_id.to_string()).branch_ref
            }
        }
    }

    /// The one domain call, under the recorded operation ID, and what it
    /// came to. `restart` is set by a call that enters the request
    /// again: it takes up an operation that stopped, where an ordinary
    /// repeat is refused, and changes nothing for one that has no row
    /// or has ended.
    fn call(&self, call: &CallContext<'_>, accepted: &Accepted, restart: bool) -> Ran {
        let service = call.service;
        let mut watch = Watch {
            call,
            accepted,
            subject: Subject::of(self, accepted),
            pushed: false,
        };
        let request = SynchronizeRemoteRequest {
            root: accepted.root(),
            operation_id: accepted.operation_id,
            target: self.target.clone(),
            // A host is approved by its own command, never in passing.
            approval: None,
            // Only a merge commits, and nothing here merges.
            confirmed_identity: None,
            restart,
        };
        let Some(result) = call
            .session
            .synchronize(service, request, SETTINGS, &mut watch)
        else {
            // No domain call was made. The record is the journal row's
            // to settle: with none, nothing is in flight.
            return Ran {
                answer: Answer::stopped(ResultCode::InternalError),
                standing: Standing::Stopped {
                    journal: accepted.journal_row(service),
                },
            };
        };
        match result {
            Ok(result) => self.finished(&result),
            Err(error) => self.stopped(service, accepted, &watch, &error),
        }
    }

    /// The domain returned a result: the synchronization has its
    /// outcome, and all that can remain is its index hand-off.
    fn finished(&self, result: &SynchronizationResult) -> Ran {
        let (outcome, discovery, standing) = match result {
            SynchronizationResult::Complete(outcome) => {
                (outcome, DiscoveryEffect::Current, Standing::Final)
            }
            SynchronizationResult::IndexPending(pending) => (
                &pending.authoritative,
                DiscoveryEffect::Pending,
                Standing::Owed,
            ),
        };
        let effects = match outcome {
            SynchronizationOutcome::Published { target: _, oid } => Effects {
                publication: PublicationEffect::Published,
                discovery,
                commit_oid: Some(oid.to_string()),
                ..Effects::not_requested()
            },
            // The remote held the commit already: nothing was published,
            // and no commit is this request's.
            SynchronizationOutcome::AlreadyCurrent { target: _, oid: _ } => Effects {
                publication: PublicationEffect::Current,
                discovery,
                ..Effects::not_requested()
            },
            // The publication remote was cleared between this request's
            // checks and its call. Nothing was published, and the row
            // the domain refreshed the index under is a local one: the
            // request has no row in the remote journal, and its record
            // goes.
            SynchronizationOutcome::PublishPending {
                target: _,
                local_oid: _,
                reason: _,
            } => {
                return Ran {
                    answer: Answer::stopped(outcome::synchronization_outcome_code(outcome)),
                    standing: Standing::Stopped {
                        journal: Some(JournalRow::Absent),
                    },
                };
            }
        };
        Ran {
            answer: Answer {
                effects,
                ..Answer::stopped(outcome::synchronization_result_code(result))
            },
            standing,
        }
    }

    /// The domain returned an error. What the request is answered with
    /// is the error's code, unless the operation's row says it was
    /// cancelled: that is final for the operation, whichever error told
    /// of it, and nothing else is `cancelled`. What becomes of the
    /// record is the row's to say.
    fn stopped(
        &self,
        service: &RepositoryService,
        accepted: &Accepted,
        watch: &Watch<'_>,
        error: &SynchronizationError,
    ) -> Ran {
        let journal = accepted.journal_row(service);
        let cancelled = match &journal {
            Some(JournalRow::Final {
                kind: FinalKind::Cancelled,
                owes_work: _,
                checkpointed: _,
            }) => true,
            Some(
                JournalRow::Absent
                | JournalRow::Pending(_)
                | JournalRow::Final {
                    kind: FinalKind::Completed | FinalKind::RetainedForInspection,
                    owes_work: _,
                    checkpointed: _,
                },
            )
            | None => false,
        };
        let code = if cancelled {
            ResultCode::Cancelled
        } else {
            outcome::synchronization_error_code(error)
        };
        // With no row the operation never reserved: nothing was begun.
        let effects = match &journal {
            Some(JournalRow::Absent) => Effects::not_requested(),
            Some(
                JournalRow::Pending(_)
                | JournalRow::Final {
                    kind: _,
                    owes_work: _,
                    checkpointed: _,
                },
            )
            | None => effects_so_far(watch.accepted_push()),
        };
        Ran {
            answer: Answer {
                effects,
                data: host_data(error),
                ..Answer::stopped(code)
            },
            standing: Standing::Stopped { journal },
        }
    }
}

/// What the repository shows of an unfinished synchronization, for a
/// call that is refused under its request ID: the push its row says the
/// remote accepted.
pub(crate) fn recorded_effects(service: &RepositoryService, accepted: &Accepted) -> Effects {
    let pushed = service
        .stored_push(&accepted.scope, accepted.operation_id)
        .ok()
        .flatten()
        .filter(|stored| stored.accepted)
        .map(|stored| stored.commit);
    effects_so_far(pushed)
}

impl Binding for SyncBinding {
    fn command(&self) -> Command {
        match &self.target {
            SynchronizationTarget::Primary => Command::RepoSync,
            SynchronizationTarget::Context {
                kind: _,
                item_id: _,
            } => Command::ItemSync,
        }
    }

    /// The row of a synchronization is in the remote journal, which
    /// records phases and no checkpoint step: a row that ended is its
    /// work done, or cancelled, and is never released as having done
    /// nothing.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }

    fn place(&self) -> Place<'_> {
        Place::Repository(&self.root)
    }

    /// The item, with its kind, or the repository as a whole.
    fn target(&self) -> String {
        match &self.target {
            SynchronizationTarget::Primary => String::new(),
            SynchronizationTarget::Context { kind, item_id } => {
                format!("{}/{item_id}", authoring_kind_segment(kind))
            }
        }
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        // The command, the repository and the target are what the digest
        // is begun with, and the binding holds nothing else.
        let Self { root: _, target: _ } = self;
        builder
    }

    fn scope(&self, repository: Option<String>, position: Option<&Position>) -> Scope {
        match &self.target {
            SynchronizationTarget::Primary => Scope {
                branch: position.and_then(|position| position.branch().map(str::to_owned)),
                repository,
                ..Scope::default()
            },
            SynchronizationTarget::Context { kind, item_id } => {
                item_scope(kind, &item_id.to_string(), repository, position)
            }
        }
    }

    /// A repository with no publication remote has nowhere to publish
    /// to, and is refused here: the domain would instead bind the
    /// request to the local branch and report it as pending, which is
    /// no request of the boundary's. Everything else is the domain's to
    /// check, before it reserves.
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        let repository = resolve_root(call.service, &self.root)?;
        let root = repository.root();
        let configuration =
            match read_configuration_for(root, RepositoryOperation::RepositorySnapshot)
                .map_err(refused)?
            {
                ConfigurationInspection::Valid(configuration) => configuration,
                ConfigurationInspection::Missing | ConfigurationInspection::Invalid(_) => {
                    return Err(Answer::stopped(ResultCode::InvalidConfiguration));
                }
            };
        if configuration.publication_remote.is_none() {
            return Err(Answer::stopped(ResultCode::PublicationRemoteRequired));
        }
        let position = Position::of(root, &self.branch_ref(root, &configuration.primary_branch))
            .map_err(|_| Answer::stopped(ResultCode::RepositoryInaccessible))?;
        Ok(Prepared {
            family: Some(OperationFamily::Remote),
            position,
            expected: None,
            observation: None,
        })
    }

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        self.call(call, accepted, false)
    }

    /// Nothing is read here and nothing is answered: whatever the row
    /// holds, the domain is called under the recorded operation ID. It
    /// replays a row that ended with an outcome, doing the index
    /// hand-off that row still owes, says of a cancelled row that it
    /// was, and takes up any other by asking the remote what it holds.
    fn reenter(
        &mut self,
        _call: &CallContext<'_>,
        _accepted: &Accepted,
        _journal: JournalRow,
    ) -> Result<Entered, Ran> {
        Ok(Entered::Unfinished)
    }

    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, _entered: &Entered) -> Ran {
        self.call(call, accepted, true)
    }
}
