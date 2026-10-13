//! Answering a call from a record without running anything: a finished
//! record of the request gives its stored result again, and a record of
//! another request under the same ID refuses the call.

use std::path::Path;

use crate::results::{Effects, Envelope, ResultCode};

use super::{
    super::RepositoryService,
    bind::{self, Accepted, Answer},
    command::Command,
    dto::{MutationDataDto, RequestState},
    evidence, outcome,
    records::RequestRecord,
    reply::{Kept, Reply},
};

impl RepositoryService {
    /// A finished record of this request: its stored result, unchanged.
    /// No domain call is made and nothing is written.
    ///
    /// The envelope is rebuilt from the record: the outcome, code, effects,
    /// commit and data it stored and the operation it names. The record
    /// matches this call's command, repository and target, and its
    /// recorded position gives the scope its branch. It stores no recovery
    /// action, and a finished ticket create or save has none.
    pub(super) fn replay(
        &self,
        reply: &Reply<'_>,
        record: RequestRecord,
    ) -> Envelope<MutationDataDto> {
        let (Some(accepted), Some(result)) =
            (Accepted::of_record(&record, record.attempt), record.result)
        else {
            return reply.stopped(ResultCode::InternalError);
        };
        // A result that names a commit is given again only while that
        // commit can be reached from the branch the request committed to,
        // or from primary once that branch is gone.
        if let Some(commit) = result.effects.commit_oid.as_deref() {
            let reachable = git2::Oid::from_str(commit).is_ok_and(|commit| {
                record.scope.repository().is_some_and(|root| {
                    evidence::still_reachable(
                        Path::new(root),
                        accepted.position.base_ref.as_deref(),
                        commit,
                    )
                })
            });
            if !reachable {
                return reply.stopped(ResultCode::RecoveryRequired);
            }
        }
        let data = match result.data.as_ref() {
            // Stored data of a command this build does not bind cannot be
            // given its shape.
            Some(stored) => {
                match Command::parse(&record.command)
                    .and_then(|command| MutationDataDto::from_stored(command, stored))
                {
                    Some(data) => Some(data),
                    None => return reply.stopped(ResultCode::InternalError),
                }
            }
            None => None,
        };
        // The one place an outcome is not derived: it is the stored one.
        let envelope = Envelope::mutation(
            record.command,
            reply.scope(Some(&accepted.position)),
            result.outcome,
            result.code,
        )
        .with_effects(result.effects)
        .with_request_id(record.request_id.to_string());
        let envelope = match accepted.operation() {
            Some(operation) => envelope.with_operation_id(operation.operation_id.to_string()),
            None => envelope,
        };
        // A finished result of a synchronization can be one that tells
        // its caller to look at the repository: an operation that was
        // stopped or fenced with its push not confirmed. That is as
        // true of the result given again as of the result first given,
        // so the action is made again from the code. So it is for the
        // result of a resume that took a synchronization up. No other
        // finished result of either has an action.
        //
        // A folder creation whose refresh failed is finished too, with
        // nothing pending that the journal could offer a resume of: its
        // result says the index is behind and offers a refresh, and
        // says so again here. No other command's finished result is
        // given an action.
        let envelope = match Command::parse(&envelope.command) {
            Some(command)
                if command.finishes_as_a_synchronization()
                    || command.hands_off_to_any_refresh() =>
            {
                let recovery = outcome::offered(
                    &outcome::Named {
                        code: envelope.code,
                        command,
                        root: envelope.scope.repository.as_deref(),
                        request_id: Some(accepted.request_id),
                        operation_id: accepted.operation().map(|operation| operation.operation_id),
                        retry: false,
                        unsettled: false,
                    },
                    Vec::new(),
                    false,
                    None,
                );
                envelope.with_recovery(recovery)
            }
            Some(_) | None => envelope,
        };
        match data {
            Some(data) => envelope.with_data(data),
            None => envelope,
        }
    }

    /// A record exists and is not of this request: the same request ID
    /// with another repository, command, target or input. Nothing runs and
    /// the record is not changed.
    ///
    /// When the recorded request is unfinished and an attempt of it ran,
    /// the result reports what the repository shows of it, a commit to its
    /// path among it, and the outcome is `partial` when that is a durable
    /// effect. Otherwise it is an input error: a finished request's
    /// effects are its own result's, which `request show` gives, and are
    /// not reported as this call's.
    pub(super) fn mismatched(
        &self,
        reply: &Reply<'_>,
        record: &RequestRecord,
    ) -> Envelope<MutationDataDto> {
        let (effects, local_change) = match record.state {
            RequestState::Accepted => {
                let recorded = bind::recorded_effects(self, record);
                (recorded.effects, recorded.local_change)
            }
            RequestState::Finished => (Effects::not_requested(), false),
        };
        reply.envelope(
            Answer {
                effects,
                local_change,
                ..Answer::stopped(ResultCode::RequestMismatch)
            },
            None,
            Kept::No,
        )
    }
}
