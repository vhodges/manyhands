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
    /// `None` when the request names a repository whose root cannot be
    /// resolved to a canonical path: no record can be of this request
    /// then.
    pub(super) scope: Option<ScopeKey>,
    pub(super) digest: Option<IntentDigest>,
}

impl RequestIdentity {
    pub(super) fn of(request_id: RequestId, binding: &dyn Binding) -> Self {
        let scope = match binding.place() {
            Place::Repository(root) => ScopeKey::for_repository(root),
            Place::NewRepository(root) => ScopeKey::for_new_repository(root),
            Place::Application => Some(ScopeKey::application()),
        };
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
            local_change,
            offers_nothing,
        } = answer;
        let scope = self.scope(accepted.map(|accepted| &accepted.position));
        // A request with no operation names none.
        let operation = accepted.and_then(Accepted::operation);
        // A read suggests its action with the root it knows. One that
        // failed before it knew any, as a read of the request records
        // does, names none, and this result does: the action is then the
        // code's own, made with this result's root.
        let rootless = recovery
            .iter()
            .any(|action| action.arguments.is_empty() && !action.action.argument_keys().is_empty());
        let recovery = if offers_nothing {
            Vec::new()
        } else if recovery.is_empty() || rootless {
            outcome::recovery(
                code,
                self.binding.command(),
                scope.repository.as_deref(),
                Some(self.identity.request_id),
                operation.map(|operation| operation.operation_id),
                retry,
            )
        } else {
            recovery
        };
        // The host a result is about is named by its data, and the
        // action that approves it takes the host.
        let recovery = outcome::with_host_action(
            recovery,
            code,
            scope.repository.as_deref(),
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

    /// A request that stopped before it was accepted, having done nothing.
    pub(super) fn stopped(&self, code: ResultCode) -> Envelope<MutationDataDto> {
        self.envelope(Answer::stopped(code), None, false)
    }

    pub(super) fn read_failure(&self, error: &ReadError) -> Envelope<MutationDataDto> {
        self.envelope(Answer::of_read(error), None, false)
    }
}
