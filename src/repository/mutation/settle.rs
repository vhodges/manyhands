//! Settling: what a call does to its request's record once it has an
//! answer. The record is finished with the result, deleted so that the
//! request ID is free again, or left accepted for a retry to continue.

use crate::results::Envelope;

use super::{
    super::{
        RepositoryService,
        recovery::{FinalKind, JournalRow},
    },
    bind::{Accepted, Ran, Standing},
    dto::MutationDataDto,
    records::RequestResult,
    reply::Reply,
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

/// Whether settling leaves the record of a call that ended with
/// `standing` accepted, for a retry to continue.
pub(super) fn leaves_accepted(standing: &Standing) -> bool {
    Settlement::of(standing) == Settlement::Leave
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
        let Ran { answer, standing } = ran;
        let settlement = Settlement::of(&standing);
        let envelope = reply.envelope(answer, Some(accepted), settlement == Settlement::Leave);
        self.settle(accepted, settlement, &envelope);
        envelope
    }

    /// Changes the record as `settlement` says. Every change is
    /// conditional on the record still being accepted with this call's
    /// attempt, so a call never settles a record another call has since
    /// entered. A change that fails leaves the record as it is: the next
    /// call of the request re-enters it.
    fn settle(
        &self,
        accepted: &Accepted,
        settlement: Settlement,
        envelope: &Envelope<MutationDataDto>,
    ) {
        match settlement {
            Settlement::Finish => {
                let data = match &envelope.data {
                    Some(data) => match serde_json::to_value(data) {
                        Ok(data) => Some(data),
                        // The record stays accepted and the request is
                        // answered again by re-entering it.
                        Err(_) => return,
                    },
                    None => None,
                };
                let _ = self.finish_request(
                    accepted.request_id,
                    accepted.attempt,
                    &RequestResult {
                        outcome: envelope.outcome,
                        code: envelope.code,
                        effects: envelope.effects.clone(),
                        data,
                    },
                );
            }
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
            Settlement::Delete => {
                let _ = self.delete_request(accepted.request_id, accepted.attempt);
            }
            Settlement::Leave => {}
        }
    }
}

#[cfg(test)]
#[path = "settle_tests.rs"]
mod tests;
