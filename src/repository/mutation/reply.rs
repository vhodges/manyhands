//! How a call identifies its request and builds its envelope.
//!
//! `RequestIdentity` is what a record must hold to be of the same request.
//! `Reply` builds every envelope a call returns but a replayed one: the
//! outcome is derived from the code and the effects, never chosen.

use crate::results::{Envelope, ResultCode, Scope, absolute_path_string};

use super::{
    super::ReadError,
    bind::{Accepted, Answer, Binding},
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
    /// `None` when the root cannot be resolved to a canonical path: no
    /// record can be of this request then.
    pub(super) scope: Option<ScopeKey>,
    pub(super) digest: Option<IntentDigest>,
}

impl RequestIdentity {
    pub(super) fn of(request_id: RequestId, binding: &dyn Binding) -> Self {
        let scope = ScopeKey::for_repository(binding.root());
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

    /// The repository, as a result's scope names it.
    fn repository(&self, binding: &dyn Binding) -> Option<String> {
        match &self.scope {
            Some(scope) => scope.repository().map(str::to_owned),
            None => absolute_path_string(binding.root()),
        }
    }

    pub(super) fn matches(&self, binding: &dyn Binding, record: &RequestRecord) -> bool {
        self.scope.as_ref() == Some(&record.scope)
            && record.command == binding.command().name()
            && record.target == binding.target()
            && self.digest == Some(record.intent_digest)
    }
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
    /// the effects; `retry` says the record was left accepted, so the same
    /// request finishes the work.
    pub(super) fn envelope(
        &self,
        answer: Answer,
        accepted: Option<&Accepted>,
        retry: bool,
    ) -> Envelope<MutationDataDto> {
        let Answer {
            code,
            effects,
            data,
            recovery,
        } = answer;
        let scope = self.scope(accepted.map(|accepted| &accepted.position));
        let recovery = if recovery.is_empty() {
            outcome::recovery(
                code,
                scope.repository.as_deref(),
                self.identity.request_id,
                accepted.map(|accepted| accepted.operation_id),
                retry,
            )
        } else {
            recovery
        };
        let mut envelope = Envelope::classified_mutation(
            self.binding.command().name(),
            scope,
            code == ResultCode::Cancelled,
            code,
            effects,
            false,
        )
        .with_request_id(self.identity.request_id.to_string())
        .with_recovery(recovery);
        if let Some(accepted) = accepted {
            envelope = envelope.with_operation_id(accepted.operation_id.to_string());
        }
        match data {
            Some(data) => envelope.with_data(data),
            None => envelope,
        }
    }

    /// A request that stopped before it was accepted, having done nothing.
    pub(super) fn stopped(&self, code: ResultCode) -> Envelope<MutationDataDto> {
        self.envelope(Answer::stopped(code), None, false)
    }

    pub(super) fn read_failure(&self, error: &ReadError) -> Envelope<MutationDataDto> {
        self.envelope(Answer::of_read(error), None, false)
    }
}
