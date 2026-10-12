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
            AuthoringKind, ConfigurationInspection, OperationFamily, ReadError,
            RemoteOperationSafePoint, RepositoryOperation, RepositoryService, SafePointRequest,
            SynchronizationError, SynchronizationObserver, SynchronizationOutcome,
            SynchronizationResult, SynchronizationSettings, SynchronizationTarget,
            SynchronizeRemoteRequest, authoring_kind_segment, read_configuration_for,
            recovery::{FinalKind, JournalRow},
            transport::SshTransportErrorKind,
        },
        command::Command,
        context::context_names,
        dto::{MutationDataDto, SynchronizationMutationDto},
        evidence::Position,
        identity::{IntentDigestBuilder, RequestId},
        outcome,
        progress::{ProgressStage, Subject},
    },
    Accepted, Answer, Binding, CallContext, Confirmed, EndedWithoutCheckpoint, Entered, Place,
    Prepared, Ran, Standing, item_scope, refused, resolve_root,
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

/// What is known of the commit a synchronization set out to publish.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Publication {
    /// Nothing is known to be on the remote.
    Unknown,
    /// The remote holds the commit, and a push of this operation put it
    /// there or is taken to have.
    Published(git2::Oid),
    /// The remote held the commit before this operation pushed anything.
    Current,
}

/// What a synchronization has published so far, as an event or a result
/// that stopped gives it.
fn effects_so_far(publication: Publication) -> Effects {
    match publication {
        Publication::Published(commit) => Effects {
            publication: PublicationEffect::Published,
            discovery: DiscoveryEffect::Pending,
            commit_oid: Some(commit.to_string()),
            ..Effects::not_requested()
        },
        Publication::Current => Effects {
            publication: PublicationEffect::Current,
            discovery: DiscoveryEffect::Pending,
            ..Effects::not_requested()
        },
        Publication::Unknown => Effects {
            publication: PublicationEffect::Pending,
            ..Effects::not_requested()
        },
    }
}

/// What the journal alone shows of a synchronization's publication: the
/// push its row says the remote holds. A claim about a request's push
/// comes from that request's own evidence, which is this, what its call
/// saw, and what the push endpoint advertised to it; never from what a
/// fetch shows, which with a push URL of its own is another place.
/// `None` when the journal cannot be read: nothing is claimed, and
/// nothing is denied, on that.
fn recorded_publication(service: &RepositoryService, accepted: &Accepted) -> Option<Publication> {
    Some(
        match service
            .stored_push(&accepted.scope, accepted.operation_id)
            .ok()?
        {
            Some(stored) if stored.accepted => Publication::Published(stored.commit),
            Some(_) | None => Publication::Unknown,
        },
    )
}

/// Whether nothing after `point` can be stopped before the push is
/// made: every safe point up to and including the one before the push.
fn precedes_the_push(point: RemoteOperationSafePoint) -> bool {
    match point {
        RemoteOperationSafePoint::BeforeFetch
        | RemoteOperationSafePoint::AfterFetch
        | RemoteOperationSafePoint::BeforeLocalUpdate
        | RemoteOperationSafePoint::AfterLocalUpdate
        | RemoteOperationSafePoint::BeforePush
        | RemoteOperationSafePoint::BeforeTransport
        | RemoteOperationSafePoint::AfterAdvertisement
        | RemoteOperationSafePoint::BetweenObservations
        | RemoteOperationSafePoint::BeforeBatchCommit
        | RemoteOperationSafePoint::AfterBatchCommit
        | RemoteOperationSafePoint::BeforeLocalMutation
        | RemoteOperationSafePoint::BeforeDiscovery => true,
        RemoteOperationSafePoint::AfterPushReturn
        | RemoteOperationSafePoint::AfterPushVerification => false,
    }
}

/// Whether the request's operation has recorded its intent to push and
/// not what became of the push. Such an operation is not ended as
/// having published nothing until the push endpoint has been asked. A
/// journal that cannot be read is taken to hold such an intent.
fn push_is_unsettled(service: &RepositoryService, accepted: &Accepted) -> bool {
    service
        .stored_push(&accepted.scope, accepted.operation_id)
        .map_or(true, |stored| stored.is_some_and(|stored| !stored.accepted))
}

/// The answer for an operation that ended, cancelled or fenced, with a
/// push intended and nothing known of it: the publication is not
/// confirmed, which is neither published nor not published. A look at
/// the repository is what is offered, and a new synchronization is what
/// tells.
fn not_confirmed() -> Answer {
    Answer {
        effects: effects_so_far(Publication::Unknown),
        ..Answer::stopped(ResultCode::RecoveryRequired)
    }
}

/// The final answer of a request whose operation was fenced: begun
/// under a configuration the registration has since left, by a change
/// of the remote's endpoints or of the selected key. The domain has let
/// the reservation go and takes the operation up for nobody, so the
/// same request cannot go on and is not offered. `None` for an
/// operation that is not fenced, or whose row has ended.
///
/// What the answer says depends on how far the operation had got:
///
/// - with no push intended, nothing was published: `external_change`;
/// - with the remote known to hold the commit, that is what is said,
///   with the index hand-off owed: `external_change`, as a partial
///   result;
/// - with a push intended and nothing known of it, `recovery_required`
///   with the publication pending: it is not confirmed, and is not said
///   to have failed.
pub(crate) fn fenced_answer(
    service: &RepositoryService,
    accepted: &Accepted,
    journal: Option<&JournalRow>,
) -> Option<Answer> {
    match journal? {
        JournalRow::Pending(_) => {}
        JournalRow::Absent
        | JournalRow::Final {
            kind: _,
            owes_work: _,
            checkpointed: _,
        } => return None,
    }
    if !service
        .fenced(&accepted.scope, accepted.operation_id)
        .unwrap_or(false)
    {
        return None;
    }
    // A journal that cannot be read says nothing of a push, and nothing
    // final is said on that: the record is left as it is.
    let intended = service
        .stored_push(&accepted.scope, accepted.operation_id)
        .ok()?
        .is_some();
    let publication = recorded_publication(service, accepted)?;
    Some(match (intended, publication) {
        (false, _) => Answer::stopped(ResultCode::ExternalChange),
        (true, Publication::Published(_) | Publication::Current) => Answer {
            effects: effects_so_far(publication),
            ..Answer::stopped(ResultCode::ExternalChange)
        },
        (true, Publication::Unknown) => not_confirmed(),
    })
}

/// The binding's observer of one domain call: it reports the stage of
/// each safe point, and asks the synchronization to stop when the call
/// or the request has been asked to.
///
/// A cancellation is passed on at the safe point it is seen at, with
/// one exception. An operation that began this call with a push
/// intended and nothing known of it is not stopped before the call has
/// asked the push endpoint what it holds: its cancellation is passed on
/// at the safe point before the push is made again, or a later one.
///
/// The request's flag is read and never cleared here. It is cleared
/// once the operation's row says it was cancelled, so that a call that
/// dies, or cannot record the cancellation on the operation, or has
/// lost the operation to another call, leaves the flag for whoever
/// comes next. A flag that cannot be read is not guessed at: before the
/// push the call stops where it is, with the operation as it was.
///
/// The flag is that of the request that owns the operation, which is
/// the request the call is of, except for a resume: `request cancel`
/// is asked of the request that began the operation, and a resume that
/// takes the operation up answers to it exactly as that request's own
/// next call would.
struct Watch<'a> {
    call: &'a CallContext<'a>,
    accepted: &'a Accepted,
    subject: Subject,
    /// The request whose record holds the operation, and so the flag
    /// `request cancel` sets. `None` when no unfinished record holds
    /// it: there is then no flag, and only the call's token stops it.
    owner: Option<RequestId>,
    /// The operation began this call with a push intended and nothing
    /// known of it.
    unsettled: bool,
    /// The safe point this call asked for a cancellation at. Asking is
    /// not stopping: the operation stopped there only if the call then
    /// ended interrupted with its row cancelled.
    asked_at: Option<RemoteOperationSafePoint>,
    /// This call saw its own push return: the remote took it.
    pushed: bool,
    /// What the domain said of the commit, from what the push endpoint
    /// advertised.
    verified: Option<Publication>,
}

impl Watch<'_> {
    /// What is known of the publication: what the push endpoint showed
    /// this call, what the call saw of its own push, or what the
    /// operation's row says. `None` when the journal has to be read and
    /// cannot be: nothing is claimed, and nothing is denied, on that.
    fn publication(&self) -> Option<Publication> {
        if let Some(verified) = self.verified {
            return Some(verified);
        }
        if self.pushed {
            let stored = self
                .call
                .service
                .stored_push(&self.accepted.scope, self.accepted.operation_id)
                .ok()?;
            if let Some(stored) = stored {
                return Some(Publication::Published(stored.commit));
            }
        }
        recorded_publication(self.call.service, self.accepted)
    }

    /// Whether a cancellation is asked for and has not been passed on:
    /// the call's token, or the request's flag, which is left as it is.
    /// A flag that cannot be read is not taken to be set.
    fn cancellation_is_pending(&self) -> bool {
        self.call.session.cancelled() || self.asked_of_the_owner().unwrap_or(false)
    }

    /// Whether the request that owns the operation has been asked to
    /// stop. The flag is read and left set.
    fn asked_of_the_owner(&self) -> Result<bool, ReadError> {
        match self.owner {
            Some(owner) => self.call.service.cancellation_requested(owner),
            None => Ok(false),
        }
    }
}

impl SynchronizationObserver for Watch<'_> {
    fn safe_point(&mut self, point: RemoteOperationSafePoint) -> SafePointRequest {
        if point == RemoteOperationSafePoint::AfterPushReturn {
            self.pushed = true;
        }
        let session = self.call.session;
        // An event says what is published only once it is: before the
        // push has returned or the remote has been asked nothing is,
        // and nothing is read.
        let effects = |watch: &Self| {
            // An event is no answer: one that cannot read what was
            // published says that nothing is known to be.
            effects_so_far(if watch.pushed || watch.verified.is_some() {
                watch.publication().unwrap_or(Publication::Unknown)
            } else {
                Publication::Unknown
            })
        };
        if let Some(stage) = stage_at(point) {
            session.report(&self.subject.event(stage, effects(self)));
        }
        let asks_the_push_endpoint_first = self.unsettled
            && match point {
                RemoteOperationSafePoint::BeforePush
                | RemoteOperationSafePoint::AfterPushReturn
                | RemoteOperationSafePoint::AfterPushVerification => false,
                RemoteOperationSafePoint::BeforeFetch
                | RemoteOperationSafePoint::AfterFetch
                | RemoteOperationSafePoint::BeforeLocalUpdate
                | RemoteOperationSafePoint::AfterLocalUpdate
                | RemoteOperationSafePoint::BeforeTransport
                | RemoteOperationSafePoint::AfterAdvertisement
                | RemoteOperationSafePoint::BetweenObservations
                | RemoteOperationSafePoint::BeforeBatchCommit
                | RemoteOperationSafePoint::AfterBatchCommit
                | RemoteOperationSafePoint::BeforeLocalMutation
                | RemoteOperationSafePoint::BeforeDiscovery => true,
            };
        if asks_the_push_endpoint_first {
            return SafePointRequest::Continue;
        }
        // The call's own token, and what `request cancel` asked of the
        // request from elsewhere, before or after the reservation
        // existed. The flag is read and left set.
        let asked = if session.cancelled() {
            Ok(true)
        } else {
            self.asked_of_the_owner()
        };
        match asked {
            Ok(true) => {
                self.asked_at = Some(point);
                session.report(&self.subject.event(ProgressStage::Stopping, effects(self)));
                SafePointRequest::Cancel
            }
            Ok(false) => SafePointRequest::Continue,
            // Whether the request was asked to stop cannot be told.
            // Before the push that is not taken for "no": the call
            // stops, and the operation is left for a call that can
            // tell. Once the push has returned the answer says
            // published either way, and the call goes on.
            Err(_) if precedes_the_push(point) => SafePointRequest::Stop,
            Err(_) => SafePointRequest::Continue,
        }
    }

    fn verified(&mut self, commit: git2::Oid, published: bool) {
        self.verified = Some(if published {
            Publication::Published(commit)
        } else {
            Publication::Current
        });
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

/// What a synchronization reads of its repository before it is
/// accepted.
struct Configured {
    /// The repository's root, as it is registered.
    root: PathBuf,
    primary_branch: String,
    /// Whether the repository has a publication remote.
    publishes: bool,
}

/// Takes up the synchronization of `target` that `accepted` names, for
/// a request that did not begin it: `operation resume`. `accepted` is
/// that request with the operation's ID and journal in place of its
/// own, and `subject` the request as its events name it.
///
/// It is the call a re-entry of the synchronization's own request
/// makes, with the same settings: the operation is taken up with
/// `restart`, which asks the remote what it holds before it pushes
/// anything, and nothing is merged. What the result says of a push is
/// that operation's own evidence, as it is for its own request.
///
/// `owner` is the request whose unfinished record holds the operation,
/// when one does. A cancellation asked of it stops this call as it
/// would stop that request's own, and is cleared on its record once
/// the operation's row says it was cancelled.
pub(crate) fn resumed(
    call: &CallContext<'_>,
    accepted: &Accepted,
    target: SynchronizationTarget,
    subject: Subject,
    owner: Option<RequestId>,
) -> Ran {
    SyncBinding {
        root: accepted.root(),
        target,
    }
    .call(call, accepted, subject, owner, true)
}

/// The position on its branch of a synchronization of `target` in the
/// repository at `root`: what a request that takes it up is accepted
/// with, so that a result of it that names a commit is given again only
/// while that branch still holds the commit.
pub(crate) fn resumed_position(
    service: &RepositoryService,
    root: &std::path::Path,
    target: SynchronizationTarget,
) -> Result<Position, Answer> {
    let binding = SyncBinding {
        root: root.to_owned(),
        target,
    };
    let configured = binding.configured(service)?;
    binding.position(&configured)
}

impl SyncBinding {
    /// Resolves the request's root and reads the repository's
    /// configuration.
    fn configured(&self, service: &RepositoryService) -> Result<Configured, Answer> {
        let repository = resolve_root(service, &self.root)?;
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
        Ok(Configured {
            root: root.to_owned(),
            publishes: configuration.publication_remote.is_some(),
            primary_branch: configuration.primary_branch,
        })
    }

    /// The request's position on the branch it synchronizes.
    fn position(&self, configured: &Configured) -> Result<Position, Answer> {
        let Configured {
            root,
            primary_branch,
            publishes: _,
        } = configured;
        Position::of(root, &self.branch_ref(root, primary_branch))
            .map_err(|_| Answer::stopped(ResultCode::RepositoryInaccessible))
    }

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

    fn request(&self, accepted: &Accepted, restart: bool) -> SynchronizeRemoteRequest {
        SynchronizeRemoteRequest {
            root: accepted.root(),
            operation_id: accepted.operation_id,
            target: self.target.clone(),
            // A host is approved by its own command, never in passing.
            approval: None,
            // Only a merge commits, and nothing here merges.
            confirmed_identity: None,
            restart,
        }
    }

    /// The one domain call, under the recorded operation ID, and what it
    /// came to. `restart` is set by a call that enters the request
    /// again: it takes up an operation that stopped, where an ordinary
    /// repeat is refused, and changes nothing for one that has no row
    /// or has ended.
    ///
    /// A cancellation asked of an operation whose push is intended and
    /// not accounted for is not passed on until the call has asked the
    /// push endpoint; see `Watch`. When the call stops with an error
    /// before it got that far, the push endpoint could not be asked, or
    /// the domain refused to go on. The cancellation is honored all the
    /// same, so that it always frees the repository, with no network if
    /// need be: the operation is asked to stop, one more call under its
    /// ID acknowledges that, which reserves nothing and opens no
    /// connection. What the request is then answered with is
    /// `stopped`'s to say, which also clears the request's flag once
    /// the row is cancelled. Should asking fail,
    /// nothing is acknowledged, the flag stands for the next call, and
    /// the error is what is reported.
    ///
    /// `subject` is the request the call's events are of: the
    /// synchronization's own, or the resume that takes it up. `owner`
    /// is the request whose record holds the operation, whose flag a
    /// cancellation is read from.
    fn call(
        &self,
        call: &CallContext<'_>,
        accepted: &Accepted,
        subject: Subject,
        owner: Option<RequestId>,
        restart: bool,
    ) -> Ran {
        let service = call.service;
        let root = accepted.root();
        let mut watch = Watch {
            call,
            accepted,
            subject,
            owner,
            unsettled: push_is_unsettled(service, accepted),
            asked_at: None,
            pushed: false,
            verified: None,
        };
        let Some(result) = call.session.synchronize(
            service,
            self.request(accepted, restart),
            SETTINGS,
            &mut watch,
        ) else {
            // No domain call was made. The record is the journal row's
            // to settle: with none, nothing is in flight.
            return Ran {
                answer: Answer::stopped(ResultCode::InternalError),
                standing: Standing::Stopped {
                    journal: accepted.journal_row(service),
                },
            };
        };
        let error = match result {
            Ok(result) => return self.finished(&result),
            Err(error) => error,
        };
        if !(watch.unsettled && watch.cancellation_is_pending()) {
            return self.stopped(service, accepted, &watch, &error);
        }
        // Only an operation that holds the reservation is asked, and the
        // second call is made only once the row says it was: a call
        // under the ID of a row that was not asked would synchronize.
        let asked = service
            .cancel_remote_operation(&root, accepted.operation_id)
            .is_ok()
            && service
                .active_remote_operation(&root)
                .ok()
                .flatten()
                .is_some_and(|active| {
                    active.operation_id() == accepted.operation_id && active.cancel_requested()
                });
        if asked
            && let Some(Err(acknowledged)) = call.session.synchronize(
                service,
                self.request(accepted, true),
                SETTINGS,
                &mut watch,
            )
        {
            return self.stopped(service, accepted, &watch, &acknowledged);
        }
        self.stopped(service, accepted, &watch, &error)
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
    /// record is the row's to say, but for an operation that was
    /// fenced, whose answer is final; see `fenced_answer`.
    fn stopped(
        &self,
        service: &RepositoryService,
        accepted: &Accepted,
        watch: &Watch<'_>,
        error: &SynchronizationError,
    ) -> Ran {
        let journal = accepted.journal_row(service);
        if let Some(answer) = fenced_answer(service, accepted, journal.as_ref()) {
            return Ran {
                answer,
                standing: Standing::Final,
            };
        }
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
        if cancelled {
            // The cancellation is acknowledged: the request's flag has
            // done its work, and only now is it cleared. A failure to
            // clear it leaves a flag on a record that is finished below,
            // where nothing reads it.
            if let Some(owner) = watch.owner {
                let _ = service.take_cancellation(owner);
            }
        }
        // What the remote holds has to be read to be said. A journal
        // that cannot be read gives no answer to store: the record is
        // left as it is, and the next call of the request, which finds
        // the row as it stands, answers.
        let Some(publication) = watch.publication() else {
            return Ran {
                answer: Answer {
                    effects: effects_so_far(Publication::Unknown),
                    ..Answer::stopped(outcome::synchronization_error_code(error))
                },
                standing: Standing::Stopped { journal: None },
            };
        };
        // The operation stopped at the safe point before the push, which
        // follows the push endpoint's own word that it does not hold the
        // commit: this call asked there, and ended interrupted.
        let stopped_before_the_push = cancelled
            && watch.asked_at == Some(RemoteOperationSafePoint::BeforePush)
            && matches!(error, SynchronizationError::Interrupted);
        // Cancelled with a push intended, nothing known of it, and the
        // push endpoint not asked by the call that stopped it: the
        // operation is over and the repository free, and the answer
        // says of the push only that it is not confirmed.
        if cancelled
            && watch.unsettled
            && !stopped_before_the_push
            && publication == Publication::Unknown
        {
            return Ran {
                answer: not_confirmed(),
                standing: Standing::Final,
            };
        }
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
            | None => effects_so_far(publication),
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

/// What a call of the synchronization's own request leaves of its
/// record, given what the call came to.
///
/// A call can stop with an error under a row that has completed: the
/// row completed beside it, or before it, and the call failed before
/// the domain read the row, on a repository or an index it could not
/// read for a moment. The error is no result of the synchronization,
/// which has one. Nothing is stored and nothing is deleted: were the
/// record released, the request sent again would be a new one, find the
/// remote current and be finished as a no-op, for a request that
/// published. The same request is answered from the row.
fn kept_for_its_result(ran: Ran) -> Ran {
    let Ran {
        mut answer,
        standing,
    } = ran;
    let standing = match standing {
        Standing::Stopped {
            journal:
                Some(JournalRow::Final {
                    kind: FinalKind::Completed,
                    owes_work,
                    checkpointed: _,
                }),
        } => {
            // What the call saw of the publication says the hand-off is
            // pending, which is all a call that stopped can know. The
            // row knows more: completed and owing no work, its hand-off
            // ran.
            if !owes_work && answer.effects.discovery == DiscoveryEffect::Pending {
                answer.effects.discovery = DiscoveryEffect::Current;
            }
            Standing::Unconfirmed
        }
        Standing::Stopped {
            journal:
                Some(
                    JournalRow::Absent
                    | JournalRow::Pending(_)
                    | JournalRow::Final {
                        kind: FinalKind::Cancelled | FinalKind::RetainedForInspection,
                        owes_work: _,
                        checkpointed: _,
                    },
                )
                | None,
        }
        | Standing::Final
        | Standing::Owed
        | Standing::Unconfirmed
        | Standing::NotRun { journal: _ } => standing,
    };
    Ran { answer, standing }
}

/// What the repository shows of an unfinished synchronization, for a
/// call that is refused under its request ID: the push the remote is
/// known to hold.
///
/// This is the answer to another request, and stores nothing. With a
/// journal that cannot be read it says the publication is pending,
/// which is what "not confirmed" is written as: neither published nor
/// not.
pub(crate) fn recorded_effects(service: &RepositoryService, accepted: &Accepted) -> Effects {
    effects_so_far(match recorded_publication(service, accepted) {
        Some(publication) => publication,
        None => Publication::Unknown,
    })
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
        let configured = self.configured(call.service)?;
        if !configured.publishes {
            return Err(Answer::stopped(ResultCode::PublicationRemoteRequired));
        }
        let position = self.position(&configured)?;
        Ok(Prepared {
            family: Some(OperationFamily::Remote),
            position,
            expected: None,
            observation: None,
        })
    }

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        kept_for_its_result(self.call(
            call,
            accepted,
            Subject::of(self, accepted),
            Some(accepted.request_id),
            false,
        ))
    }

    /// Nothing is read here and nothing is answered: whatever the row
    /// holds, the domain is called under the recorded operation ID. It
    /// replays a row that ended with an outcome, doing the index
    /// hand-off that row still owes, says of a cancelled row that it
    /// was, and takes up any other by asking the remote what it holds.
    ///
    /// The one row nothing is called for is a fenced one, which the
    /// domain takes up for nobody: its answer is final.
    fn reenter(
        &mut self,
        call: &CallContext<'_>,
        accepted: &Accepted,
        journal: JournalRow,
    ) -> Result<Entered, Ran> {
        match fenced_answer(call.service, accepted, Some(&journal)) {
            Some(answer) => Err(Ran {
                answer,
                standing: Standing::Final,
            }),
            None => Ok(Entered::Unfinished),
        }
    }

    fn as_confirmed(&mut self) -> Option<&mut dyn Confirmed> {
        None
    }

    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, _entered: &Entered) -> Ran {
        kept_for_its_result(self.call(
            call,
            accepted,
            Subject::of(self, accepted),
            Some(accepted.request_id),
            true,
        ))
    }
}
