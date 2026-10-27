//! Settling: what a call does to its request's record once it has an
//! answer. The record is finished with the result, deleted so that the
//! request ID is free again, or left accepted for a retry to continue.

use crate::results::{Envelope, Outcome, ResultCode};

use super::{
    super::{
        ReadError, RepositoryService,
        recovery::{FinalKind, JournalRow},
    },
    bind::{Accepted, Answer, Ran, Standing},
    dto::{MutationDataDto, RequestState},
    identity::RequestId,
    records::RequestResult,
    reply::{Kept, Reply},
};

/// What a call does to its record once its domain call has returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Settlement {
    /// The result is stored and the record is finished.
    Finish,
    /// The record is deleted and the confirmation it accepted released:
    /// the request ID is free again.
    Delete,
    /// The record stays accepted: a retry continues the request.
    Leave,
}

impl Settlement {
    /// Settling is decided from what the binding's call left and from the
    /// operation's journal row, never from the kind of error the domain
    /// returned.
    ///
    /// For a request that stopped, a result the domain has made final for
    /// the operation is asked for before whether anything is in flight: a
    /// cancelled synchronization that still owes its index hand-off is in
    /// flight and is final all the same, and asking in the other order
    /// would leave its record accepted for good. A row that could not be
    /// read leaves the record as it is.
    fn of(standing: &Standing) -> Self {
        match standing {
            Standing::Final => Self::Finish,
            Standing::Owed | Standing::Unconfirmed => Self::Leave,
            Standing::Stopped { journal: None } => Self::Leave,
            Standing::NotRun { journal } => match journal {
                JournalRow::Absent => Self::Delete,
                JournalRow::Pending(_) | JournalRow::Final { .. } => Self::Leave,
            },
            Standing::Stopped {
                journal: Some(journal),
            } => match journal {
                JournalRow::Final {
                    kind: FinalKind::Cancelled | FinalKind::RetainedForInspection,
                    owes_work: _,
                    checkpointed: _,
                } => Self::Finish,
                JournalRow::Final {
                    kind: FinalKind::Completed,
                    owes_work: _,
                    checkpointed: _,
                }
                | JournalRow::Absent
                | JournalRow::Pending(_) => {
                    if journal.in_flight() {
                        Self::Leave
                    } else {
                        Self::Delete
                    }
                }
            },
        }
    }
}

impl RepositoryService {
    /// The envelope of what a call found or did, with its record settled
    /// from it.
    pub(super) fn settled(
        &self,
        reply: &Reply<'_>,
        accepted: &Accepted,
        ran: Ran,
    ) -> Envelope<MutationDataDto> {
        if super::local_control::effects(self, accepted.request_id)
            .ok()
            .flatten()
            .is_some()
            && let Ok(Some(record)) = self.request_record(accepted.request_id)
            && record.state == RequestState::Finished
        {
            return self.replay(reply, record);
        }
        let Ran { answer, standing } = ran;
        let settlement = Settlement::of(&standing);
        let applied = self.settle(accepted, settlement, &answer);
        // A finish or a delete that did not apply: the write failed, or
        // the record was no longer this call's, because another call of
        // the request entered it or a resume took its operation up. The
        // envelope is still what this call found or did. It is not the
        // request's last word while a record of the request stands, and
        // says so by offering the same request: an accepted record is
        // finished by it, and one another call finished holds a result
        // that need not be this one, which it reads.
        let kept = match settlement {
            Settlement::Leave => Kept::Accepted,
            Settlement::Finish | Settlement::Delete => {
                if !applied && self.still_stands(accepted.request_id) {
                    Kept::Unsettled
                } else {
                    Kept::No
                }
            }
        };
        reply.envelope(answer, Some(accepted), kept)
    }

    /// Whether a record of the request stands now, after a settlement
    /// of it that did not apply: accepted still, or finished by another
    /// call. Where another call deleted it there is nothing left to send
    /// the request again for. A record that cannot be read is taken to
    /// stand, as it did when this call last knew.
    fn still_stands(&self, request_id: RequestId) -> bool {
        match self.request_record(request_id) {
            Ok(Some(record)) => match record.state {
                RequestState::Accepted | RequestState::Finished => true,
            },
            Ok(None) => false,
            Err(_) => true,
        }
    }

    /// Stores `answer` as the result of the request `request_id` and
    /// marks its record finished, if the record is still accepted with
    /// `attempt`. This is the one writer of a finished result: what a
    /// record stores of an answer, and the outcome it stores, are worked
    /// out here and nowhere else. The outcome is derived from the code
    /// and the effects exactly as an envelope's is.
    ///
    /// Returns whether the record was finished. `Ok(false)` for a record
    /// that another call has since entered, finished or deleted, and for
    /// an answer whose data cannot be stored: the record is then as it
    /// was, and the next call of the request re-enters it.
    pub(super) fn finish(
        &self,
        request_id: RequestId,
        attempt: u64,
        answer: &Answer,
    ) -> Result<bool, ReadError> {
        let data = match &answer.data {
            Some(data) => match serde_json::to_value(data) {
                Ok(data) => Some(data),
                Err(_) => return Ok(false),
            },
            None => None,
        };
        self.finish_request(
            request_id,
            attempt,
            &RequestResult {
                outcome: Outcome::of_mutation(
                    answer.code == ResultCode::Cancelled,
                    answer.code,
                    &answer.effects,
                    answer.local_change,
                ),
                code: answer.code,
                effects: answer.effects.clone(),
                data,
            },
        )
    }

    /// Changes the record as `settlement` says, and returns whether the
    /// change it asks for was made; a record that is left is as asked.
    /// Every change is conditional on the record still being accepted
    /// with this call's attempt, so a call never settles a record another
    /// call has since entered. A change that fails leaves the record as
    /// it is: the next call of the request re-enters it.
    fn settle(&self, accepted: &Accepted, settlement: Settlement, answer: &Answer) -> bool {
        match settlement {
            Settlement::Finish => self
                .finish(accepted.request_id, accepted.attempt, answer)
                .unwrap_or(false),
            // "Nothing in flight" is read from the journal row, and a call
            // of the request that was accepted and has not yet begun its
            // row shows none. A second call that enters in that window and
            // stops with no row deletes the record under the first, which
            // then does its work and finds no record to settle. That is a
            // known limit and it is safe: the first call's caller has its
            // result, no effect is repeated, and the next call of the
            // request has no record, so it is treated as new and answered
            // by the already-applied rule, which writes nothing and claims
            // no commit.
            Settlement::Delete => self
                .delete_request(accepted.request_id, accepted.attempt)
                .unwrap_or(false),
            Settlement::Leave => true,
        }
    }
}

#[cfg(test)]
#[path = "settle_tests.rs"]
mod tests;
