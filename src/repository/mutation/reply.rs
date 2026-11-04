//! How a call identifies its request and builds its envelope.
//!
//! `RequestIdentity` is what a record must hold to be of the same request.
//! `Reply` builds every envelope a call returns but a replayed one: the
//! outcome is derived from the code and the effects, never chosen.

use crate::results::{Envelope, ResultCode, Scope, absolute_path_string};

use super::{
    super::ReadError,
    bind::{Accepted, Answer, Binding, Place},
    dto::MutationDataDto,
    evidence::Position,
    identity::{DigestSalt, IntentDigest, RequestId, ScopeKey},
    outcome,
    records::RequestRecord,
};

/// A request as `execute` identifies it: what a record of it must hold to
/// be the same request.
pub(super) struct RequestIdentity {
    pub(super) request_id: RequestId,
    /// `None` when the call cannot establish its scope. Removal can recover
    /// an exact persisted root without filesystem resolution; other repository
    /// commands still require the live canonical path.
    pub(super) scope: Option<ScopeKey>,
    pub(super) digest: Option<IntentDigest>,
}

impl RequestIdentity {
    pub(super) fn of(request_id: RequestId, binding: &dyn Binding) -> Self {
        Self::with_scope(
            request_id,
            binding,
            super::confirm::scope_key(binding.place()),
        )
    }

    /// The scope has already been resolved and frozen by the binding.
    pub(super) fn with_scope(
        request_id: RequestId,
        binding: &dyn Binding,
        scope: Option<ScopeKey>,
    ) -> Self {
        let digest = scope.as_ref().map(|scope| {
            binding
                .fields(IntentDigest::builder(
                    DigestSalt::Request(request_id),
                    binding.command().name(),
                    scope,
                    &binding.target(),
                ))
                .finish()
        });
        Self {
            request_id,
            scope,
            digest,
        }
    }

    /// The repository, as a result's scope names it: none for a request
    /// of the application.
    fn repository(&self, binding: &dyn Binding) -> Option<String> {
        match (&self.scope, binding.place()) {
            (Some(scope), _) => scope.repository().map(str::to_owned),
            (None, Place::Repository(root) | Place::NewRepository(root)) => {
                absolute_path_string(root)
            }
            (None, Place::Application) => None,
        }
    }

    pub(super) fn matches(&self, binding: &dyn Binding, record: &RequestRecord) -> bool {
        self.scope.as_ref() == Some(&record.scope)
            && record.command == binding.command().name()
            && record.target == binding.target()
            && self.digest == Some(record.intent_digest)
    }
}

/// What became of the request's record in the call an envelope is of,
/// for what the envelope offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kept {
    /// The call leaves no record that says anything else than its
    /// envelope: none was made, this call finished or deleted it, or
    /// another call deleted it.
    No,
    /// The record was left accepted, as the call's standing asks: the
    /// same request continues the work.
    Accepted,
    /// The call meant to finish or delete the record, that did not
    /// apply, and a record still stands: accepted, or finished by
    /// another call with a result that need not be this one. The result
    /// is what the call found or did, and is not the request's last
    /// word: the same request is to be sent again.
    Unsettled,
}

/// What the envelope of one call is built with.
pub(super) struct Reply<'a> {
    pub(super) binding: &'a dyn Binding,
    pub(super) identity: &'a RequestIdentity,
}

impl Reply<'_> {
    pub(super) fn scope(&self, position: Option<&Position>) -> Scope {
        self.binding
            .scope(self.identity.repository(self.binding), position)
    }

    /// The envelope of an answer. The outcome is derived from the code and
    /// the effects; `kept` says what became of the record, which decides
    /// whether the same request is offered. It changes neither the code
    /// nor the effects.
    pub(super) fn envelope(
        &self,
        answer: Answer,
        accepted: Option<&Accepted>,
        kept: Kept,
    ) -> Envelope<MutationDataDto> {
        let Answer {
            code,
            effects,
            data,
            recovery,
            local_change,
            offers_nothing,
            // Of a preview that is refused, and of no request.
            host_replacement: _,
            lifecycle_blocked: _,
        } = answer;
        let scope = self.scope(accepted.map(|accepted| &accepted.position));
        // A request with no operation names none.
        let operation = accepted.and_then(Accepted::operation);
        // The host a result is about is named by its data, and the
        // action that approves it takes the host.
        let recovery = outcome::offered(
            &outcome::Named {
                code,
                command: self.binding.command(),
                root: scope.repository.as_deref(),
                request_id: Some(self.identity.request_id),
                operation_id: operation.map(|operation| operation.operation_id),
                retry: kept == Kept::Accepted,
                unsettled: kept == Kept::Unsettled,
            },
            recovery,
            offers_nothing,
            data.as_ref()
                .and_then(MutationDataDto::authority)
                .as_deref(),
        );
        let mut envelope = Envelope::classified_mutation(
            self.binding.command().name(),
            scope,
            code == ResultCode::Cancelled,
            code,
            effects,
            local_change,
        )
        .with_request_id(self.identity.request_id.to_string())
        .with_recovery(recovery);
        if let Some(operation) = operation {
            envelope = envelope.with_operation_id(operation.operation_id.to_string());
        }
        match data {
            Some(data) => envelope.with_data(data),
            None => envelope,
        }
    }

    /// A call that found its request's record accepted and could not
    /// enter it: nothing was read of the operation and nothing was run.
    /// The record stands, so the same request is what goes on.
    pub(super) fn not_entered(&self, answer: Answer) -> Envelope<MutationDataDto> {
        self.envelope(answer, None, Kept::Accepted)
    }

    /// A call that meant to release its request's record, to run the
    /// request as new, and could not: the record stands.
    pub(super) fn not_released(&self, answer: Answer) -> Envelope<MutationDataDto> {
        self.envelope(answer, None, Kept::Unsettled)
    }

    /// A request that stopped before it was accepted, having done nothing.
    pub(super) fn stopped(&self, code: ResultCode) -> Envelope<MutationDataDto> {
        self.envelope(Answer::stopped(code), None, Kept::No)
    }

    pub(super) fn read_failure(&self, error: &ReadError) -> Envelope<MutationDataDto> {
        self.envelope(Answer::of_read(error), None, Kept::No)
    }
}
