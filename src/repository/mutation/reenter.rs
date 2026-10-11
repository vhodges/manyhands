//! Re-entry: a call that finds an accepted record of its own request. An
//! earlier call of the request started and its end was not recorded.

use crate::results::{Envelope, ResultCode};

use super::{
    super::{
        RepositoryService,
        recovery::{FinalKind, JournalRow},
    },
    bind::{Accepted, Answer, Binding, EndedWithoutCheckpoint, Ran, Standing},
    dto::MutationDataDto,
    records::RequestRecord,
    reply::{Reply, RequestIdentity},
};

/// How a call that re-enters an accepted record takes it, decided from the
/// operation's journal row as it was before the call and from what the
/// binding says of a row that ended without a checkpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reentry {
    /// The record is released and the call is run as a new request.
    AsNew,
    /// The record is kept: the binding reads its evidence, and the request
    /// is continued or answered from what its operation did.
    AsRecorded,
}

impl Reentry {
    fn of(journal: &JournalRow, rule: EndedWithoutCheckpoint) -> Self {
        match journal {
            JournalRow::Final {
                kind: FinalKind::Completed,
                owes_work: false,
                checkpointed: false,
            } => match rule {
                EndedWithoutCheckpoint::DidNothing => Self::AsNew,
                // Handled as a row that has its checkpoint step is handled:
                // the record is kept and the binding re-enters it.
                EndedWithoutCheckpoint::SaysNothing => Self::AsRecorded,
            },
            JournalRow::Absent
            | JournalRow::Pending(_)
            | JournalRow::Final {
                kind: _,
                owes_work: _,
                checkpointed: _,
            } => Self::AsRecorded,
        }
    }
}

impl RepositoryService {
    /// An accepted record matches this call: an earlier call of the
    /// request started and its end was not recorded. `None` when the
    /// record was settled between the look that found it and this.
    ///
    /// The call raises the record's attempt, so that it is the one call
    /// that may settle it: a call that entered before it and is still
    /// running reports what it did and changes nothing. The caller's
    /// token is not checked again. Then, in order:
    ///
    /// 1. The operation's journal row is read here, as it is before this
    ///    call. A row that cannot be read answers the call and leaves the
    ///    record as it is.
    /// 2. When the row is final, completed, owes no work and never
    ///    reached a checkpoint, and the binding says such a row did
    ///    nothing, the record is released and `None` is returned: the
    ///    next look finds no record and runs the call as a new request.
    ///    A binding that says the row tells it nothing goes on as for any
    ///    other final row.
    /// 3. The binding sets up its domain call and reads what the
    ///    repository shows of the request since the recorded position.
    ///    What cannot be read answers the call and leaves the record as
    ///    it is. The binding may answer here with no domain call: the
    ///    ticket binding finishes a request whose operation has completed
    ///    and whose own commit is in range, and answers
    ///    `external_change` when someone else has committed other
    ///    content to the path since.
    /// 4. Otherwise the domain operation is called under the recorded
    ///    operation ID and expectation. This is the only way the work is
    ///    continued: only the domain completes its row and its hand-off.
    /// 5. What is reported comes from the evidence read before and after
    ///    that call, and the record is settled as a first call's is.
    pub(super) fn reenter(
        &self,
        binding: &mut dyn Binding,
        identity: &RequestIdentity,
        record: &RequestRecord,
    ) -> Option<Envelope<MutationDataDto>> {
        let attempt = match self.enter_request(record.request_id) {
            Ok(Some(attempt)) => attempt,
            Ok(None) => return None,
            Err(error) => return Some(Reply { binding, identity }.read_failure(&error)),
        };
        let Some(accepted) = Accepted::of_record(record, attempt) else {
            return Some(Reply { binding, identity }.stopped(ResultCode::InternalError));
        };
        // The row as it is before this call: its state afterwards proves
        // nothing about an earlier attempt.
        let journal =
            match self.journal_row(accepted.family, &accepted.scope, accepted.operation_id) {
                Ok(journal) => journal,
                Err(error) => {
                    let ran = Ran {
                        answer: Answer::of_read(&error),
                        standing: Standing::Stopped { journal: None },
                    };
                    return Some(self.settled(&Reply { binding, identity }, &accepted, ran));
                }
            };
        match Reentry::of(&journal, binding.ended_without_checkpoint()) {
            // The operation ended before its checkpoint: it stopped or was
            // refused with nothing in flight. Had that answer not been
            // lost, settling would have deleted the record. It is deleted
            // now, with this call's attempt, and the request is taken as
            // new on the next look: a new operation ID, the token check
            // and the already-applied rule. The old row is final and is
            // left alone; calling the domain on it again would record a
            // checkpoint on a row that never had one, and a later retry
            // could then take another request's commit for this one's.
            Reentry::AsNew => {
                return match self.delete_request(accepted.request_id, accepted.attempt) {
                    Ok(_) => None,
                    Err(error) => Some(Reply { binding, identity }.read_failure(&error)),
                };
            }
            Reentry::AsRecorded => {}
        }
        let before = binding.reenter(self, &accepted, journal);
        let reply = Reply { binding, identity };
        Some(match before {
            Ok(before) => self.called(&reply, &accepted, || {
                binding.rerun(self, &accepted, &before)
            }),
            Err(ran) => self.settled(&reply, &accepted, ran),
        })
    }
}

#[cfg(test)]
#[path = "reenter_tests.rs"]
mod tests;
