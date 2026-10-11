//! `execute`: one call of a mutation, from its request ID to its envelope.
//!
//! This looks the request up and decides what the call is: a request no
//! record holds is validated, checked, accepted and run here; one whose
//! record is finished is replayed, one whose record is accepted is
//! re-entered, and one whose record is of another request is refused.
//! What is done to the record afterwards is settling's.

use std::path::Path;

use crate::results::{Envelope, ResultCode};

use super::{
    super::{
        FailurePoint, OperationId, RepositoryOperation, RepositoryService,
        keys::{SessionCredentialProvider, SessionCredentials},
    },
    CancellationToken, MutationCall, ProgressSink,
    bind::{Accepted, Answer, Binding, Ran},
    dto::{MutationDataDto, RequestState},
    identity::ConfirmationId,
    records::{InsertRequestOutcome, NewRequest, RequestOperation},
    reply::{Reply, RequestIdentity},
    settle,
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
        // No bound command needs a credential, reports a stage or can be
        // stopped once it is accepted; the commands that do use these.
        _credentials: &mut SessionCredentials<P>,
        _progress: &mut dyn ProgressSink,
        cancel: &CancellationToken,
    ) -> Envelope<MutationDataDto> {
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
        for _ in 0..3 {
            let record = match self.request_record(request_id) {
                Ok(record) => record,
                // The index is unavailable, and with it every record. A
                // rebuild is how it comes back, so that one command runs
                // all the same.
                Err(error)
                    if error.code() == ResultCode::IndexUnavailable
                        && binding.command().restores_index() =>
                {
                    return self.unrecorded(binding.as_mut(), &identity, confirmation, cancel);
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
                                self.reenter(binding.as_mut(), &identity, &record)
                            {
                                return envelope;
                            }
                        }
                    }
                }
                None => {
                    if let Some(envelope) =
                        self.first_call(binding.as_mut(), &identity, confirmation, cancel)
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
        .stopped(ResultCode::Busy)
    }

    /// A request no record holds: validate, check, accept, run, settle.
    /// `None` when another call accepted the same request ID meanwhile.
    fn first_call(
        &self,
        binding: &mut dyn Binding,
        identity: &RequestIdentity,
        confirmation: Option<ConfirmationId>,
        cancel: &CancellationToken,
    ) -> Option<Envelope<MutationDataDto>> {
        let (accepted, request) = match self.checked(binding, identity, confirmation, cancel) {
            Ok(checked) => checked,
            Err(envelope) => return Some(envelope),
        };
        let reply = Reply { binding, identity };
        match self.insert_request(&request) {
            Ok(InsertRequestOutcome::Inserted) => {}
            Ok(InsertRequestOutcome::RequestExists) => return None,
            // No confirmation was presented.
            Ok(InsertRequestOutcome::ConfirmationUnavailable) => {
                return Some(reply.stopped(ResultCode::InternalError));
            }
            Err(error) => return Some(reply.read_failure(&error)),
        }
        Some(self.run_accepted(&reply, &accepted))
    }

    /// `index rebuild` with the index unavailable: the request is
    /// validated and checked as any first call is, and runs with no
    /// record, since there is nowhere to write one until it has run.
    ///
    /// Nothing is stored of a rebuild that ran to its end or left nothing:
    /// the same request ID sent again is a new request. One that stopped
    /// with its operation in flight refuses every other operation on the
    /// repository until it is taken up under its operation ID, which only
    /// a record holds. So its record is written then, if the index can
    /// now take one, and the same request finishes it.
    fn unrecorded(
        &self,
        binding: &mut dyn Binding,
        identity: &RequestIdentity,
        confirmation: Option<ConfirmationId>,
        cancel: &CancellationToken,
    ) -> Envelope<MutationDataDto> {
        let (accepted, request) = match self.checked(binding, identity, confirmation, cancel) {
            Ok(checked) => checked,
            Err(envelope) => return envelope,
        };
        let reply = Reply { binding, identity };
        let ran = match self.attempted(&reply, &accepted, || binding.run(self, &accepted)) {
            Ok(ran) => ran,
            Err(envelope) => return envelope,
        };
        let recorded = settle::leaves_accepted(&ran.standing)
            && matches!(
                self.insert_request(&request),
                Ok(InsertRequestOutcome::Inserted)
            );
        if recorded {
            return self.settled(&reply, &accepted, ran);
        }
        reply.envelope(ran.answer, Some(&accepted), false)
    }

    /// The checks of a first call, up to the point where its record is
    /// written: what the request is accepted as, and the record to write.
    /// An `Err` answers the request, which is not accepted.
    fn checked(
        &self,
        binding: &mut dyn Binding,
        identity: &RequestIdentity,
        confirmation: Option<ConfirmationId>,
        cancel: &CancellationToken,
    ) -> Result<(Accepted, NewRequest), Envelope<MutationDataDto>> {
        // No bound command takes a confirmation.
        if confirmation.is_some() {
            return Err(Reply { binding, identity }.stopped(ResultCode::NotConfirmable));
        }
        let prepared = match binding.prepare(self) {
            Ok(prepared) => prepared,
            Err(answer) => return Err(Reply { binding, identity }.envelope(answer, None, false)),
        };
        let reply = Reply { binding, identity };
        // `prepare` resolved the repository, so the root has a scope key.
        let (Some(scope), Some(digest)) = (identity.scope.clone(), identity.digest) else {
            return Err(reply.stopped(ResultCode::InternalError));
        };
        if cancel.is_cancelled() {
            return Err(reply.stopped(ResultCode::Cancelled));
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
        let request = NewRequest {
            request_id: accepted.request_id,
            scope,
            command: binding.command().name().to_owned(),
            target: binding.target(),
            intent_digest: digest,
            confirmation: None,
            base_ref: accepted.position.base_ref.clone(),
            base_oid: accepted.position.base_oid,
            expected_digest: accepted.expected.clone(),
            operations: vec![RequestOperation {
                family: accepted.family,
                operation_id: accepted.operation_id,
            }],
        };
        Ok((accepted, request))
    }

    /// Makes the binding's domain call for an accepted request and settles
    /// its record from what the call left.
    fn run_accepted(&self, reply: &Reply<'_>, accepted: &Accepted) -> Envelope<MutationDataDto> {
        self.called(reply, accepted, || reply.binding.run(self, accepted))
    }

    /// Makes a domain call, `call`, for an accepted request and settles its
    /// record from what the call left.
    ///
    /// The two fault points stand in for a process that died: at either,
    /// the call ends with its record exactly as it was, and what it
    /// returns is not a result of the request.
    pub(super) fn called(
        &self,
        reply: &Reply<'_>,
        accepted: &Accepted,
        call: impl FnOnce() -> Ran,
    ) -> Envelope<MutationDataDto> {
        match self.attempted(reply, accepted, call) {
            Ok(ran) => self.settled(reply, accepted, ran),
            Err(envelope) => envelope,
        }
    }

    /// Makes a domain call, `call`, for an accepted request. An `Err` is
    /// the envelope of a call that a test had die at a fault point.
    fn attempted(
        &self,
        reply: &Reply<'_>,
        accepted: &Accepted,
        call: impl FnOnce() -> Ran,
    ) -> Result<Ran, Envelope<MutationDataDto>> {
        let died = |point| {
            self.dies_at(point, accepted)
                .then(|| reply.envelope(Answer::stopped(ResultCode::InternalError), None, false))
        };
        if let Some(envelope) = died(FailurePoint::BeforeRequestDomainCall) {
            return Err(envelope);
        }
        // The cache guard is not held here: the record functions release
        // it before they return, and the domain call takes it itself.
        self.run_request_hook();
        let ran = call();
        if let Some(envelope) = died(FailurePoint::BeforeRequestSettlement) {
            return Err(envelope);
        }
        Ok(ran)
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
