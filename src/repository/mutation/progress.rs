//! Progress and cancellation: what a call of `execute` is given besides
//! its request, and how that reaches a binding.
//!
//! A caller hands `execute` a sink for the call's progress, a token that
//! stops it and the session's credentials. All three are the call's and
//! no part of the request: the same request sent again brings its own.
//!
//! Cancellation is cooperative. It is honored before a request is
//! accepted, by every command, and after that only where the request's
//! operation has a safe point, which is a synchronization: at each one
//! the binding looks at the token and at the flag `cancel_request` sets
//! on the request's record, and asks the synchronization to stop there.
//! A local operation has no safe point once it is accepted.

use std::{
    cell::RefCell,
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use serde::{Serialize, Serializer};

use crate::results::{Effects, Envelope, ResultCode, Scope, contract_enum};

use super::{
    super::{
        OperationFamily, ReadError, RepositoryService, SynchronizationError,
        SynchronizationObserver, SynchronizationResult, SynchronizationSettings,
        SynchronizeRemoteRequest,
        keys::{SessionCredentialProvider, SessionCredentials},
        recovery::{FinalKind, JournalRow, PendingOperation},
        transport::{SshTransportError, SshTransportVerified, VerifySshTransportRequest},
    },
    bind::{Accepted, Binding, sync},
    command::Command,
    dto::{CancelDto, RequestState},
    identity::RequestId,
    records::RequestRecord,
};

/// How a caller asks a running request to stop. A request that is
/// cancelled before it is accepted does nothing. One that is accepted
/// stops at the next safe point its operation reaches, when it has one.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

contract_enum!(
    /// What a request is about to do, as its progress names it.
    ///
    /// `accepted` is the request recorded, before anything is done for
    /// it. `writing` and `indexing` are what a local command then does.
    /// The rest are a synchronization's: `connecting` before it opens a
    /// connection, `fetching` before it fetches, `observing` while it
    /// compares what it has with what the remote holds, `updating_local`
    /// before it moves a local branch, `pushing` before it pushes,
    /// `verifying` once the push has returned and before the remote is
    /// asked what it now holds, and `indexing` once that is known.
    /// `stopping` is a cancellation the request has seen and is about to
    /// honor.
    ///
    /// A resume reports `observing` before it looks at the operation it
    /// names, then `indexing` before a hand-off it runs, or the stages
    /// of a synchronization it takes up.
    ProgressStage {
        Accepted => "accepted",
        Observing => "observing",
        Writing => "writing",
        Connecting => "connecting",
        Fetching => "fetching",
        UpdatingLocal => "updating_local",
        Pushing => "pushing",
        Verifying => "verifying",
        Indexing => "indexing",
        Stopping => "stopping",
    }
);

/// One step of a request's progress.
///
/// It names the request and where it works, and holds nothing of what
/// the request carries or meets: no body or source, no commit message,
/// no remote location, no credential and no text a server sent.
/// `effects` is what the request has completed so far; the request's
/// result is what names a commit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProgressEvent {
    pub request_id: String,
    /// Null for a request that has no operation.
    pub operation_id: Option<String>,
    pub command: String,
    pub scope: Scope,
    pub stage: ProgressStage,
    pub effects: Effects,
}

/// Where a request reports its progress.
///
/// `event` is called on the thread that runs `execute`, and `execute`
/// goes on only when it returns: a sink that waits holds the request
/// where it is. Every event of a call has been sent before `execute`
/// returns the call's result.
///
/// A sink must not call back into the library. Two of a
/// synchronization's stages, `updating_local` and the `observing` that
/// follows it, are reported while the repository's lease is held.
///
/// A sink that panics is sent nothing more by that call. The panic goes
/// no further: the request runs on as if the sink had returned.
pub trait ProgressSink {
    fn event(&mut self, event: &ProgressEvent);
}

/// A sink for a caller that wants no progress.
pub struct NoProgress;

impl ProgressSink for NoProgress {
    fn event(&mut self, _event: &ProgressEvent) {}
}

/// The request a call's events are of: everything of an event but its
/// stage and effects.
pub(crate) struct Subject {
    request_id: String,
    operation_id: Option<String>,
    command: &'static str,
    scope: Scope,
}

impl Subject {
    /// The accepted request `accepted`, of `binding`.
    pub(crate) fn of(binding: &dyn Binding, accepted: &Accepted) -> Self {
        Self {
            request_id: accepted.request_id.to_string(),
            operation_id: accepted
                .operation()
                .map(|operation| operation.operation_id.to_string()),
            command: binding.command().name(),
            scope: binding.scope(
                accepted.scope.repository().map(str::to_owned),
                Some(&accepted.position),
            ),
        }
    }

    pub(crate) fn event(&self, stage: ProgressStage, effects: Effects) -> ProgressEvent {
        ProgressEvent {
            request_id: self.request_id.clone(),
            operation_id: self.operation_id.clone(),
            command: self.command.to_owned(),
            scope: self.scope.clone(),
            stage,
            effects,
        }
    }
}

/// What one call was given besides its request, as a binding reaches
/// it: where its progress goes, what stops it and what it authenticates
/// with.
///
/// This is a trait and its one use is as a trait object. `execute` is
/// generic over what supplies a passphrase, and a binding is not: what
/// a call was given is behind this, so that no binding, and nothing
/// between `execute` and a binding, is generic with it.
pub(crate) trait Session {
    /// Whether the call's token has been cancelled.
    fn cancelled(&self) -> bool;

    /// Sends `event` to the call's sink.
    fn report(&self, event: &ProgressEvent);

    /// Synchronizes with the session's credentials; see
    /// `RepositoryService::synchronize_remote_observed`. `None` when the
    /// session has no credentials, or they are in use: nothing was
    /// called.
    fn synchronize(
        &self,
        service: &RepositoryService,
        request: SynchronizeRemoteRequest,
        settings: SynchronizationSettings,
        observer: &mut dyn SynchronizationObserver,
    ) -> Option<Result<SynchronizationResult, SynchronizationError>>;

    /// Connects to the host of the publication remote with the session's
    /// credentials and sees the key it presents; see
    /// `RepositoryService::verify_ssh_transport`. `None` when the
    /// session has no credentials, or they are in use: nothing was
    /// called.
    fn verify_transport(
        &self,
        service: &RepositoryService,
        request: VerifySshTransportRequest,
    ) -> Option<Result<SshTransportVerified, SshTransportError>>;
}

/// A call's sink, and whether it has panicked.
struct Reporter<'s> {
    sink: &'s mut dyn ProgressSink,
    panicked: bool,
}

/// What one call of `execute` was given.
///
/// A binding is handed its call's context by shared reference, and both
/// a sink and the credentials are written to: each is behind a
/// `RefCell`, which the one thread a call runs on borrows for the length
/// of one use. The two are apart, because a synchronization reports
/// while it holds the credentials.
pub(crate) struct CallSession<'s, P> {
    progress: RefCell<Reporter<'s>>,
    cancel: &'s CancellationToken,
    credentials: RefCell<&'s mut SessionCredentials<P>>,
}

impl<'s, P> CallSession<'s, P> {
    pub(crate) fn new(
        credentials: &'s mut SessionCredentials<P>,
        progress: &'s mut dyn ProgressSink,
        cancel: &'s CancellationToken,
    ) -> Self {
        Self {
            progress: RefCell::new(Reporter {
                sink: progress,
                panicked: false,
            }),
            cancel,
            credentials: RefCell::new(credentials),
        }
    }
}

impl<P: SessionCredentialProvider> Session for CallSession<'_, P> {
    fn cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    /// A sink that panics is the caller's fault and not the request's:
    /// the panic is caught here, the sink is sent nothing more, and the
    /// request goes on. A sink that is already being written to, which
    /// one thread cannot bring about, is passed over.
    fn report(&self, event: &ProgressEvent) {
        let Ok(mut reporter) = self.progress.try_borrow_mut() else {
            return;
        };
        if reporter.panicked {
            return;
        }
        let sink = &mut *reporter.sink;
        if catch_unwind(AssertUnwindSafe(|| sink.event(event))).is_err() {
            reporter.panicked = true;
        }
    }

    fn synchronize(
        &self,
        service: &RepositoryService,
        request: SynchronizeRemoteRequest,
        settings: SynchronizationSettings,
        observer: &mut dyn SynchronizationObserver,
    ) -> Option<Result<SynchronizationResult, SynchronizationError>> {
        let mut credentials = self.credentials.try_borrow_mut().ok()?;
        Some(service.synchronize_remote_observed(request, &mut credentials, settings, observer))
    }

    fn verify_transport(
        &self,
        service: &RepositoryService,
        request: VerifySshTransportRequest,
    ) -> Option<Result<SshTransportVerified, SshTransportError>> {
        let mut credentials = self.credentials.try_borrow_mut().ok()?;
        Some(service.verify_ssh_transport(request, &mut credentials))
    }
}

/// What `prepare` gives a binding in place of a call's session: a
/// preview reports no progress and is not stopped, since it changes
/// nothing and is over at once, and it authenticates with nothing.
pub(crate) struct NoSession;

impl Session for NoSession {
    fn cancelled(&self) -> bool {
        false
    }

    fn report(&self, _event: &ProgressEvent) {}

    fn synchronize(
        &self,
        _service: &RepositoryService,
        _request: SynchronizeRemoteRequest,
        _settings: SynchronizationSettings,
        _observer: &mut dyn SynchronizationObserver,
    ) -> Option<Result<SynchronizationResult, SynchronizationError>> {
        None
    }

    fn verify_transport(
        &self,
        _service: &RepositoryService,
        _request: VerifySshTransportRequest,
    ) -> Option<Result<SshTransportVerified, SshTransportError>> {
        None
    }
}

impl RepositoryService {
    /// Finishes the record of a request whose operation was fenced,
    /// with the answer `bind::sync::fenced_answer` gives it. `None` for
    /// a request that is not fenced, which is left exactly as it is.
    /// The record is changed only if it is still accepted at the
    /// attempt it was read at, as every settlement is.
    fn finish_fenced(&self, record: &RequestRecord) -> Option<Result<(), ReadError>> {
        let accepted = Accepted::of_record(record, record.attempt)?;
        let journal = accepted.journal_row(self);
        let answer = sync::fenced_answer(self, &accepted, journal.as_ref())?;
        Some(
            self.finish(record.request_id, record.attempt, &answer)
                .map(|_| ()),
        )
    }
}

/// The command `cancel_request` answers.
const REQUEST_CANCEL: &str = "request cancel";

impl RepositoryService {
    /// `request cancel`: asks the request `request_id` to stop, from
    /// another thread or process than the one running it.
    ///
    /// It sets a flag on the request's record, which the request's
    /// synchronization reads at each safe point it reaches, and, when
    /// the synchronization already holds the repository's reservation,
    /// asks that operation to stop as `cancel_remote_operation` does.
    /// Either way the request stops at its next safe point and no
    /// sooner: while a transport call has not returned, the request is
    /// `requested` and not `acknowledged`.
    ///
    /// Only a request that is accepted and whose command has a safe
    /// point is asked anything. A request of any other command is left
    /// exactly as it is: a local operation cannot be stopped once it is
    /// accepted, and its record stays open for the retry that finishes
    /// it. Nothing here finishes or deletes a record, whatever its
    /// operation's row holds; a request that stops is finished by the
    /// call that was running it, or by its next one.
    ///
    /// A successful envelope reports the call. A request ID no record
    /// holds is `request_not_found`.
    pub fn cancel_request(&self, request_id: RequestId) -> Envelope<CancelDto> {
        let record = match self.request_record(request_id) {
            Ok(Some(record)) => record,
            Ok(None) => {
                return Envelope::failure(
                    REQUEST_CANCEL,
                    Scope::default(),
                    ResultCode::RequestNotFound,
                    Vec::new(),
                );
            }
            Err(error) => return error.to_envelope(REQUEST_CANCEL),
        };
        let scope = Scope {
            repository: record.scope.repository().map(str::to_owned),
            ..Scope::default()
        };
        let stops = Command::parse(&record.command).is_some_and(Command::stops_at_safe_points);
        let dto = |requested, acknowledged| CancelDto {
            request_id: request_id.to_string(),
            state: record.state,
            requested,
            acknowledged,
        };
        let data = match (record.state, stops) {
            // What the request came to is its stored result.
            (RequestState::Finished, _) => dto(
                false,
                record
                    .result
                    .as_ref()
                    .is_some_and(|result| result.code == ResultCode::Cancelled),
            ),
            (RequestState::Accepted, false) => dto(false, false),
            (RequestState::Accepted, true) => {
                // A request whose operation was fenced can reach no
                // safe point: nothing takes the operation up again. It
                // is not asked something nobody will answer. Its record
                // is finished here with the answer its own next call
                // would give, and that call then returns it.
                if let Some(finished) = self.finish_fenced(&record) {
                    return match finished {
                        Ok(()) => Envelope::read_success(
                            REQUEST_CANCEL,
                            scope,
                            CancelDto {
                                request_id: request_id.to_string(),
                                state: RequestState::Finished,
                                requested: false,
                                acknowledged: false,
                            },
                        ),
                        Err(error) => error.to_envelope(REQUEST_CANCEL),
                    };
                }
                match self.request_cancellation(request_id) {
                    // Finished or deleted since it was read: nothing is
                    // left to ask.
                    Ok(false) => dto(false, false),
                    Ok(true) => {
                        let mut acknowledged = false;
                        if let Ok(Some((issued, ack))) = self
                            .read_session(crate::repository::RepositoryOperation::Read, |db| {
                                super::local_control::counters(db, request_id)
                            })
                        {
                            acknowledged = issued > 0 && ack == issued;
                        }
                        for operation in &record.operations {
                            if operation.family != OperationFamily::Remote {
                                continue;
                            }
                            let row = self.journal_row(
                                operation.family,
                                &record.scope,
                                operation.operation_id,
                            );
                            match row {
                                // The reservation exists: the operation
                                // is asked directly as well, unless it
                                // has recorded its intent to push. From
                                // then on only the request's own call
                                // passes a cancellation on, at a safe
                                // point where what became of the push is
                                // known: asked directly, the operation
                                // could be ended by a restart before the
                                // remote was asked.
                                Ok(JournalRow::Pending(PendingOperation::Remote { phase: _ })) => {
                                    let intends_a_push = self
                                        .stored_push(&record.scope, operation.operation_id)
                                        .map_or(true, |stored| stored.is_some());
                                    if !intends_a_push {
                                        // A failure here loses nothing:
                                        // the record's flag stands, and
                                        // the next safe point reads it.
                                        let _ = self.cancel_remote_operation(
                                            Path::new(record.scope.as_str()),
                                            operation.operation_id,
                                        );
                                    }
                                }
                                Ok(JournalRow::Final {
                                    kind: FinalKind::Cancelled,
                                    owes_work: _,
                                    checkpointed: _,
                                }) => acknowledged = true,
                                Ok(
                                    JournalRow::Absent
                                    | JournalRow::Pending(
                                        PendingOperation::Local { state: _, step: _ }
                                        | PendingOperation::KeyMaterial { phase: _ },
                                    )
                                    | JournalRow::Final {
                                        kind:
                                            FinalKind::Completed | FinalKind::RetainedForInspection,
                                        owes_work: _,
                                        checkpointed: _,
                                    },
                                )
                                | Err(_) => {}
                            }
                        }
                        dto(true, acknowledged)
                    }
                    Err(error) => return error.to_envelope(REQUEST_CANCEL),
                }
            }
        };
        Envelope::read_success(REQUEST_CANCEL, scope, data)
    }
}
