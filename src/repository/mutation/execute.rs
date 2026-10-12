//! `execute`: one call of a mutation, from its request ID to its envelope.
//!
//! This looks the request up and decides what the call is: a request no
//! record holds is validated, checked, accepted and run here; one whose
//! record is finished is replayed, one whose record is accepted is
//! re-entered, and one whose record is of another request is refused.
//! What is done to the record afterwards is settling's.

use std::path::Path;

use crate::results::{Effects, Envelope, RecoveryActionKind, ResultCode};

use super::{
    super::{
        FailurePoint, OperationId, RepositoryOperation, RepositoryService,
        bridges::abandon::stale_heartbeat,
        keys::{SessionCredentialProvider, SessionCredentials},
        leases_are_free, recovery,
    },
    CancellationToken, MutationCall, ProgressSink,
    bind::{Accepted, Answer, Binding, CallContext, Place, Ran, resume},
    confirm::Consent,
    dto::{MutationDataDto, RequestState},
    identity::ConfirmationId,
    outcome,
    progress::{CallSession, ProgressStage, Session, Subject},
    records::{self, ConfirmationRecord, InsertRequestOutcome, NewRequest},
    reply::{Reply, RequestIdentity},
};

impl RepositoryService {
    /// Runs one mutation under the caller's request ID and returns its
    /// result. Every handled outcome, success or not, is an envelope.
    ///
    /// The request ID decides what the call is. A request seen for the
    /// first time is validated, checked, accepted and run once. One whose
    /// record is finished gets the stored result again, whatever has
    /// happened to its target since, and nothing is written. The same ID
    /// with another command, target or input is `request_mismatch`, and
    /// nothing runs.
    ///
    /// The call is synchronous and runs on the caller's thread. It takes
    /// no lease and holds no lock between its steps: every Git or
    /// canonical effect is made by the domain operation the binding calls,
    /// under that operation's own lease and journal row.
    pub fn execute<P: SessionCredentialProvider>(
        &self,
        call: MutationCall,
        credentials: &mut SessionCredentials<P>,
        progress: &mut dyn ProgressSink,
        cancel: &CancellationToken,
    ) -> Envelope<MutationDataDto> {
        // Everything the call was given reaches a binding through its
        // session, by way of the call context.
        let session = CallSession::new(credentials, progress, cancel);
        let envelope = self.execute_call(call, &session);
        let envelope = self.offering_resume(envelope);
        self.offering_abandon(envelope)
    }

    /// Keeps `operation.resume` among what an envelope offers only
    /// where a resume of the operation it names would take the
    /// operation up, as things are once the call is over: it is asked
    /// with the question a resume itself asks; see `resume::takes_up`.
    ///
    /// An index hand-off can be owed by a row a resume leaves to its
    /// request: one that reached its hand-off without recording the
    /// step that says its own work is done, as an enablement or a
    /// selection does when it is taken up after its commit. A resume of
    /// it is `original_request_required`, so the action would be a dead
    /// end. The request's own result offers the same request in its
    /// place, which its record, left accepted, is there for. A resume's
    /// own result offers nothing in its place.
    fn offering_resume<T>(&self, mut envelope: Envelope<T>) -> Envelope<T> {
        let offers = envelope
            .recovery
            .iter()
            .any(|action| action.action == RecoveryActionKind::OperationResume);
        if !offers {
            return envelope;
        }
        let root = envelope.scope.repository.clone();
        let own = envelope.operation_id.clone();
        let request_id = envelope.request_id.clone();
        let mut offered = Vec::with_capacity(envelope.recovery.len());
        for action in std::mem::take(&mut envelope.recovery) {
            if action.action != RecoveryActionKind::OperationResume {
                offered.push(action);
                continue;
            }
            let taken_up = root
                .as_deref()
                .zip(action.operation_id.as_deref())
                .and_then(|(root, operation_id)| {
                    Some((root, OperationId::parse(operation_id).ok()?))
                })
                .is_some_and(|(root, operation_id)| resume::takes_up(self, root, operation_id));
            if taken_up {
                offered.push(action);
            } else if own.is_some()
                && action.operation_id == own
                && let Some(request_id) = request_id.clone()
            {
                offered.push(outcome::original_request(request_id));
            }
        }
        envelope.recovery = offered;
        envelope
    }

    /// Adds `operation.abandon` to an envelope of a request that
    /// stopped, where a pending local operation is what stands in the way;
    /// see `outcome::offering_abandon` for the two rules.
    ///
    /// The journal and the records are read only for an envelope either
    /// rule can apply to, after everything the call did, so the action
    /// names what is pending now. That is also so for a result given
    /// again from a finished record: its stored actions are none, and
    /// what it is offered is what is true when it is read.
    ///
    /// An operation is offered only when the lookup can see nothing
    /// against abandoning it: it is an operation of the local journal
    /// alone, no call is at work under it with a heartbeat, and no call
    /// holds the repository's lease or its bootstrap lease, which is
    /// looked at without waiting and let go at once. The look is taken
    /// last, and only when there is something to offer.
    ///
    /// An operation offered so that is another request's is asked, with
    /// the question a resume itself asks, whether a resume would take
    /// it up: where it would, the resume is offered before the abandon.
    ///
    /// A journal that cannot be read adds nothing: the envelope keeps
    /// what it has.
    pub(super) fn offering_abandon<T>(&self, mut envelope: Envelope<T>) -> Envelope<T> {
        if envelope.code.failure_class().is_none() {
            return envelope;
        }
        let Some(root) = envelope.scope.repository.clone() else {
            return envelope;
        };
        let inspects = outcome::abandon_may_be_offered(envelope.code, &envelope.recovery);
        // The request's own operation, when its envelope names one.
        let own = envelope
            .operation_id
            .as_deref()
            .and_then(|operation_id| OperationId::parse(operation_id).ok())
            .zip(envelope.request_id.clone());
        if !inspects && own.is_none() {
            return envelope;
        }
        let read = self.read_session(RepositoryOperation::Read, |connection| {
            let stale_before = stale_heartbeat();
            let mut pending = Vec::new();
            for (operation_id, row) in recovery::pending_rows(connection, &root)? {
                if !row.has_live_heartbeat(stale_before)
                    && !recovery::held_by_another_journal(connection, &operation_id)?
                {
                    pending.push(operation_id);
                }
            }
            // Whether the request's record was left accepted, holding
            // that operation as a local one.
            let own = match &own {
                Some((operation_id, request_id)) => {
                    let owner = records::operation_owner(connection, &root, *operation_id)?;
                    (owner.as_deref() == Some(request_id.as_str()))
                        .then(|| operation_id.to_string())
                }
                None => None,
            };
            Ok((pending, own))
        });
        let Ok((pending, own)) = read else {
            return envelope;
        };
        if pending.is_empty() {
            return envelope;
        }
        let data_directory = self
            .registry_path
            .parent()
            .unwrap_or_else(|| Path::new("."));
        if !leases_are_free(data_directory, Path::new(&root)) {
            return envelope;
        }
        // Asked only under the rule that offers another's operation.
        let resumable: Vec<String> = if inspects {
            pending
                .iter()
                .filter(|pending| envelope.operation_id.as_ref() != Some(*pending))
                .filter(|pending| {
                    OperationId::parse(pending)
                        .is_ok_and(|operation_id| resume::takes_up(self, &root, operation_id))
                })
                .cloned()
                .collect()
        } else {
            Vec::new()
        };
        envelope.recovery = outcome::offering_abandon(
            std::mem::take(&mut envelope.recovery),
            envelope.code,
            Some(&root),
            &pending,
            &resumable,
            own.as_deref(),
        );
        envelope
    }

    /// One call of `execute`, before the journal is asked what its
    /// envelope can offer.
    fn execute_call(&self, call: MutationCall, session: &dyn Session) -> Envelope<MutationDataDto> {
        let MutationCall {
            request_id,
            confirmation,
            mutation,
        } = call;
        let mut binding = mutation.into_binding();
        let identity = RequestIdentity::of(request_id, binding.as_ref());
        // Two processes that submit the same new request ID race on the
        // record's key. The one that loses finds a record on its second
        // look and is answered from it. A record released on re-entry, of
        // a request that ended before its checkpoint, takes one more look:
        // the request is then new.
        let mut restored = false;
        for _ in 0..3 {
            let record = match self.request_record(request_id) {
                Ok(record) => record,
                // The index is unavailable, and with it every record. A
                // rebuild is how it comes back, and a rebuild is recorded
                // before its operation begins like any request: a process
                // that died with the operation's row pending and no
                // record would leave a row nothing could take up again.
                // So the index is brought back first, as the rebuild
                // itself would bring it back, and the request looked up
                // again.
                //
                // A request that would be refused before it is accepted
                // is refused before anything is replaced for it.
                Err(error)
                    if error.code() == ResultCode::IndexUnavailable
                        && binding.command().restores_index() =>
                {
                    let refused = if confirmation.is_some() {
                        Some(ResultCode::NotConfirmable)
                    } else if session.cancelled() {
                        Some(ResultCode::Cancelled)
                    } else {
                        None
                    };
                    if let Some(code) = refused {
                        return Reply {
                            binding: binding.as_ref(),
                            identity: &identity,
                        }
                        .stopped(code);
                    }
                    // Only a repository's index is restored for it.
                    let Place::Repository(root) = binding.place() else {
                        return Reply {
                            binding: binding.as_ref(),
                            identity: &identity,
                        }
                        .read_failure(&error);
                    };
                    restored = true;
                    match self.restore_index_for_rebuild(root) {
                        Ok(()) => continue,
                        // Only the kind is read.
                        Err(error) => {
                            return Reply {
                                binding: binding.as_ref(),
                                identity: &identity,
                            }
                            .stopped(outcome::repository_error_code(error.kind));
                        }
                    }
                }
                Err(error) => {
                    return Reply {
                        binding: binding.as_ref(),
                        identity: &identity,
                    }
                    .read_failure(&error);
                }
            };
            match record {
                Some(record) => {
                    let reply = Reply {
                        binding: binding.as_ref(),
                        identity: &identity,
                    };
                    if !identity.matches(binding.as_ref(), &record) {
                        return self.mismatched(&reply, &record);
                    }
                    match record.state {
                        RequestState::Finished => return self.replay(&reply, record),
                        RequestState::Accepted => {
                            // `None` when another call settled the record
                            // meanwhile: the next look answers from what
                            // it left.
                            if let Some(envelope) =
                                self.reenter(binding.as_mut(), &identity, &record, session)
                            {
                                return envelope;
                            }
                        }
                    }
                }
                None => {
                    if let Some(envelope) =
                        self.first_call(binding.as_mut(), &identity, confirmation, session)
                    {
                        return envelope;
                    }
                }
            }
        }
        Reply {
            binding: binding.as_ref(),
            identity: &identity,
        }
        // The index was restored and its records still cannot be read:
        // that is not a state the same request sent again finds changed.
        .stopped(if restored {
            ResultCode::IndexUnavailable
        } else {
            ResultCode::Busy
        })
    }

    /// A request no record holds: validate, check, accept, run, settle.
    /// `None` when another call accepted the same request ID meanwhile.
    ///
    /// A command that needs confirmation is run only with a confirmation
    /// that is this request's to use, and only while the state its preview
    /// showed is still the state there is. Its confirmation is marked as
    /// accepted in the transaction that writes the request's record.
    fn first_call(
        &self,
        binding: &mut dyn Binding,
        identity: &RequestIdentity,
        confirmation: Option<ConfirmationId>,
        session: &dyn Session,
    ) -> Option<Envelope<MutationDataDto>> {
        let consent = match self.consent(&Reply { binding, identity }, confirmation) {
            Ok(consent) => consent,
            Err(envelope) => return Some(envelope),
        };
        let call = self.call_context(consent.given(), session);
        let prepared = match binding.prepare(&call) {
            Ok(prepared) => prepared,
            Err(answer) => return Some(Reply { binding, identity }.envelope(answer, None, false)),
        };
        // No confirmation was looked for, because the request's root has
        // no scope key. The binding's check of the root refuses such a
        // request. One that it let through has no consent, and is not
        // run on that.
        if let Consent::Unplaced = consent {
            return Some(Reply { binding, identity }.stopped(ResultCode::InternalError));
        }
        let consent = consent.given();
        let reply = Reply { binding, identity };
        // `prepare` resolved the repository, so the root has a scope key;
        // a request of the application always has one.
        let (Some(scope), Some(digest)) = (identity.scope.clone(), identity.digest) else {
            return Some(reply.stopped(ResultCode::InternalError));
        };
        // What the preview observed is compared with what is there now,
        // before anything is accepted. A command with no confirmation
        // observes nothing of the kind.
        if let Some(consent) = consent
            && prepared.observation != consent.observation_digest
        {
            return Some(reply.stopped(ResultCode::ExternalChange));
        }
        if session.cancelled() {
            return Some(reply.stopped(ResultCode::Cancelled));
        }
        let accepted = Accepted {
            request_id: identity.request_id,
            attempt: 1,
            scope: scope.clone(),
            family: prepared.family,
            // A new acceptance gets a new operation ID, so the journal's
            // own mismatch check never reaches a caller.
            operation_id: OperationId::new(),
            position: prepared.position,
            expected: prepared.expected,
        };
        self.run_acceptance_hook();
        let inserted = self.insert_request(&NewRequest {
            request_id: accepted.request_id,
            scope,
            command: binding.command().name().to_owned(),
            target: binding.target(),
            intent_digest: digest,
            confirmation: consent.map(|consent| consent.confirmation_id),
            base_ref: accepted.position.base_ref.clone(),
            base_oid: accepted.position.base_oid,
            expected_digest: accepted.expected.clone(),
            operations: accepted.operation().into_iter().collect(),
        });
        match inserted {
            Ok(InsertRequestOutcome::Inserted) => {}
            Ok(InsertRequestOutcome::RequestExists) => return None,
            // The confirmation was there to accept when it was looked
            // at, and is not now: another request accepted it meanwhile.
            // With no confirmation presented nothing is asked of one, and
            // this is not reached.
            Ok(InsertRequestOutcome::ConfirmationUnavailable) => {
                return Some(reply.stopped(if consent.is_some() {
                    ResultCode::ConfirmationUsed
                } else {
                    ResultCode::InternalError
                }));
            }
            Err(error) => return Some(reply.read_failure(&error)),
        }
        session.report(
            &Subject::of(reply.binding, &accepted)
                .event(ProgressStage::Accepted, Effects::not_requested()),
        );
        Some(self.called(&reply, &accepted, session, || {
            reply.binding.run(&call, &accepted)
        }))
    }

    /// What this call gives a binding to work with: the service, the
    /// consent the request runs with, when its command needs one, and
    /// what the call itself was given.
    pub(super) fn call_context<'a>(
        &'a self,
        confirmation: Option<&'a ConfirmationRecord>,
        session: &'a dyn Session,
    ) -> CallContext<'a> {
        CallContext {
            service: self,
            session,
            confirmation,
        }
    }

    /// Makes a domain call, `call`, for an accepted request and settles its
    /// record from what the call left.
    ///
    /// The two fault points stand in for a process that died: at either,
    /// the call ends with its record exactly as it was, and what it
    /// returns is not a result of the request.
    ///
    /// The stage the command reports is sent last before the call, so
    /// that a sink is told of what is about to be done and of nothing
    /// that is not.
    pub(super) fn called(
        &self,
        reply: &Reply<'_>,
        accepted: &Accepted,
        session: &dyn Session,
        call: impl FnOnce() -> Ran,
    ) -> Envelope<MutationDataDto> {
        let died = |point| {
            self.dies_at(point, accepted)
                .then(|| reply.envelope(Answer::stopped(ResultCode::InternalError), None, false))
        };
        if let Some(envelope) = died(FailurePoint::BeforeRequestDomainCall) {
            return envelope;
        }
        // The cache guard is not held here: the record functions release
        // it before they return, and the domain call takes it itself.
        self.run_request_hook();
        session.report(
            &Subject::of(reply.binding, accepted)
                .event(reply.binding.command().stage(), Effects::not_requested()),
        );
        let ran = call();
        if let Some(envelope) = died(FailurePoint::BeforeRequestSettlement) {
            return envelope;
        }
        self.settled(reply, accepted, ran)
    }

    /// Whether a test has this call end at `point`.
    fn dies_at(&self, point: FailurePoint, accepted: &Accepted) -> bool {
        self.should_inject(
            point,
            RepositoryOperation::Read,
            Path::new(accepted.scope.as_str()),
        )
        .unwrap_or(false)
    }
}
