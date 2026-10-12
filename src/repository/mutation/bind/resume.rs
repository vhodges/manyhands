//! `operation resume`: an operation that stopped is taken up for the
//! work it still owes that needs nothing from the caller. It needs no
//! confirmation: what it does was consented to when the operation began.
//!
//! Sending the request that began an operation again, under its own
//! request ID, is the general way to finish it. A resume is for the two
//! kinds of work that need none of that request's input:
//!
//! - the index hand-off of a local operation whose own work is recorded
//!   as done;
//! - a synchronization, which is taken up as its own request's next call
//!   would take it up: the remote is asked what it holds before anything
//!   is pushed, and nothing is merged.
//!
//! Anything else that is pending is left exactly as it was: a save that
//! stopped before or after its write, whose body no record holds, a key
//! generation, an enablement that has not registered its repository, a
//! synchronization that was fenced. While the request that began it has
//! an unfinished record, the answer is `original_request_required` and
//! names that request, which is what goes on. With no such record no
//! request can be sent again, and the answer says what the row is: a
//! fenced synchronization is `external_change`, as its request was
//! told; a local or key operation is `recovery_required`, with the
//! abandon of a local one offered; a poll, a promotion or a closure is
//! `invalid_input`.
//!
//! An operation that has ended owes nothing, and resuming it is a no-op.
//! One that was abandoned is refused as every repeat of its ID is.
//!
//! A resume answers to the request that owns the operation it takes up.
//! A cancellation asked of that request stops the resume, and the
//! owner's record is entered when the operation is taken over, so that
//! a call of the owner that was still running settles nothing.

use std::path::{Path, PathBuf};

use crate::results::{
    DiscoveryEffect, Effects, RecoveryAction, RecoveryActionKind, ResultCode, Scope,
};

use super::{
    super::{
        super::{
            OperationFamily, OperationId, ReadError, RefreshRepositoryRequest, RepositoryErrorKind,
            RepositoryOperation, RepositoryService, SynchronizationTarget, keys,
            recovery::{self, FinalKind, JournalRow, LocalStanding, PendingOperation},
            remote,
        },
        command::Command,
        evidence::Position,
        identity::{IntentDigestBuilder, RequestId, ScopeKey},
        outcome,
        progress::{ProgressStage, Subject},
        records,
    },
    Accepted, Answer, Binding, CallContext, EndedWithoutCheckpoint, Entered, Place, Prepared, Ran,
    Standing, sync,
};

/// The input of `operation resume`: the root of the repository, and the
/// operation to take up. A key operation belongs to the application and
/// is found whatever root is named.
#[derive(Clone)]
pub struct OperationResumeInput {
    pub root: PathBuf,
    pub operation_id: OperationId,
}

/// `operation resume`: the work one operation still owes that needs no
/// input is done under that operation's own ID, in its own journal.
///
/// **Its record holds no operation.** The operation it names is another
/// request's and is the request's target, as an abandon's is, so the
/// same request ID with another operation is `request_mismatch`. Nothing
/// is ever in flight under the resume itself: what is in flight is the
/// operation's, and the request that began it holds that. So a resume
/// that stops leaves no record, and its request ID is free. Its record
/// is finished when the operation's end state is reached or is final,
/// and is kept accepted only where nothing could be read of the
/// operation, for the same request to look again.
///
/// **What its result claims** is what its own call did or the operation
/// itself shows. A hand-off it ran is `discovery: current` and nothing
/// else: the write and the commit are the result of the request that
/// made them. A synchronization it took up reports what that operation
/// reports to its own request, from the same evidence.
///
/// **Cancellation.** The call's token is honored before the request is
/// accepted, and at the safe points of a synchronization it takes up,
/// where it ends that operation as it would for the operation's own
/// request. `request cancel` asks nothing of a resume.
pub(crate) struct OperationResumeBinding {
    root: PathBuf,
    operation_id: OperationId,
    /// This call entered the request again: an earlier call of it
    /// started and its end was not recorded.
    entered_again: bool,
}

impl From<OperationResumeInput> for OperationResumeBinding {
    fn from(input: OperationResumeInput) -> Self {
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let OperationResumeInput { root, operation_id } = input;
        Self {
            root,
            operation_id,
            entered_again: false,
        }
    }
}

/// The request's own journal row: it has no operation, so none.
const NO_ROW: JournalRow = JournalRow::Absent;

/// The request whose unfinished record holds the operation a resume
/// takes up.
#[derive(Clone, Copy)]
struct Owner {
    request: RequestId,
    /// The attempt this call raised the owner's record to when it took
    /// the operation over: the one its record is settled at. `None`
    /// for a call that took nothing over, or whose owner's record was
    /// settled meanwhile.
    attempt: Option<u64>,
}

/// How the operation a resume names stands, for someone who would take
/// it up.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Named {
    /// No journal holds an operation of the ID for the repository.
    NotFound,
    /// A local operation that was closed as abandoned. It is neither
    /// taken up nor taken to be done.
    Abandoned,
    /// The operation has ended and owes nothing: it completed, was
    /// rejected having left nothing, or was cancelled. `synchronization`
    /// is what it synchronized, when it is one.
    Ended {
        synchronization: Option<SynchronizationTarget>,
    },
    /// The operation has ended in a way nothing continues and has left
    /// work recorded as outstanding: a cancelled synchronization that
    /// Git may still hold something of. It is looked at, not resumed.
    /// `synchronization` is what it synchronized, when it is one.
    LeftToInspect {
        synchronization: Option<SynchronizationTarget>,
    },
    /// A local operation whose own work is recorded and which owes its
    /// index hand-off and nothing else.
    HandOff,
    /// A synchronization that has not ended, or has ended with its index
    /// hand-off owed, and was not fenced.
    Synchronization(SynchronizationTarget),
    /// The operation is pending and only the request that began it goes
    /// on with it: it needs that request's input, or was fenced and has
    /// a final answer for that request alone. `request` is that
    /// request, whose record holds the operation and is not finished:
    /// sent again, it is what goes on. Without such a record there is
    /// no request to name, and the operation stands as one of the three
    /// below.
    OriginalRequest { request: String },
    /// A synchronization that was fenced, with no unfinished record
    /// that holds it: its request has been given its final answer, or
    /// never was the boundary's. Nothing goes on with it.
    Fenced,
    /// A pending operation of the local journal or of the key journal
    /// that needs its request's input, and that no unfinished record
    /// holds: there is no request to send again. It is looked at, and a
    /// local one can be abandoned.
    Unowned,
    /// A poll, a promotion or a closure that has not ended. No request
    /// of the boundary begins one and nothing takes one up under its
    /// ID: it is no operation this command is for.
    NotResumable,
}

/// The standing of a pending operation that only its own request goes
/// on with: that request, when an unfinished record of `family` in
/// `scope` holds the operation, and `otherwise` when none does.
fn left_to_its_request(
    connection: &rusqlite::Connection,
    family: OperationFamily,
    scope: &str,
    operation_id: OperationId,
    otherwise: Named,
) -> rusqlite::Result<Named> {
    Ok(
        match records::operation_holder(connection, family, scope, operation_id)? {
            Some(request) => Named::OriginalRequest { request },
            None => otherwise,
        },
    )
}

/// A row that is not what any call of this build writes: nothing is
/// done on it.
fn unreadable() -> ReadError {
    ReadError::new(ResultCode::InternalError)
}

/// How the ID stands in the remote journal of the repository at `root`.
/// `None` when that journal holds no operation of the ID there.
#[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
fn found_remote(
    connection: &rusqlite::Connection,
    root: &str,
    operation_id: OperationId,
) -> Result<Option<Named>, ReadError> {
    let row = remote::state::lookup_operation(connection, root, operation_id)?;
    let target = remote::state::lookup_synchronization_target(connection, root, operation_id)?;
    Ok(Some(match (row, target) {
        (JournalRow::Absent, None) => return Ok(None),
        // The two are read of one row, in one transaction.
        (JournalRow::Absent, Some(_))
        | (
            JournalRow::Pending(_)
            | JournalRow::Final {
                kind: _,
                owes_work: _,
                checkpointed: _,
            },
            None,
        ) => return Err(unreadable()),
        (
            JournalRow::Final {
                kind: _,
                owes_work: false,
                checkpointed: _,
            },
            Some(synchronization),
        ) => Named::Ended { synchronization },
        // A synchronization that ran to its end and has not handed what
        // it published to the index: the domain, called again, does
        // that hand-off and nothing else.
        (
            JournalRow::Final {
                kind: FinalKind::Completed,
                owes_work: true,
                checkpointed: _,
            },
            Some(Some(target)),
        ) => Named::Synchronization(target),
        // Cancelled, which nothing continues, or an operation that is
        // no synchronization, with work recorded as outstanding.
        (
            JournalRow::Final {
                kind: FinalKind::Completed | FinalKind::Cancelled | FinalKind::RetainedForInspection,
                owes_work: true,
                checkpointed: _,
            },
            Some(synchronization),
        ) => Named::LeftToInspect { synchronization },
        (JournalRow::Pending(PendingOperation::Remote { phase: _ }), Some(Some(target))) => {
            // The domain takes a fenced operation up for nobody.
            if remote::state::lookup_fenced(connection, root, operation_id)? {
                left_to_its_request(
                    connection,
                    OperationFamily::Remote,
                    root,
                    operation_id,
                    Named::Fenced,
                )?
            } else {
                Named::Synchronization(target)
            }
        }
        // A poll, a promotion or a closure: no request of the boundary
        // begins one, and nothing takes one up under its ID.
        (JournalRow::Pending(PendingOperation::Remote { phase: _ }), Some(None)) => {
            Named::NotResumable
        }
        // The remote journal holds remote operations.
        (
            JournalRow::Pending(
                PendingOperation::Local { state: _, step: _ }
                | PendingOperation::KeyMaterial { phase: _ },
            ),
            Some(_),
        ) => return Err(unreadable()),
    }))
}

/// How the ID stands in the key journal, which is the application's.
/// `None` when that journal holds no operation of the ID.
#[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
fn found_key(
    connection: &rusqlite::Connection,
    operation_id: OperationId,
) -> Result<Option<Named>, ReadError> {
    Ok(Some(
        match keys::lookup_material_operation(connection, operation_id)? {
            JournalRow::Absent => return Ok(None),
            // A generation or a deletion is taken up by its own request,
            // which holds the passphrase or the consent it needs.
            JournalRow::Pending(_) => left_to_its_request(
                connection,
                OperationFamily::KeyMaterial,
                ScopeKey::application().as_str(),
                operation_id,
                Named::Unowned,
            )?,
            // Completed, or kept for inspection: nothing continues
            // either.
            JournalRow::Final {
                kind: _,
                owes_work: _,
                checkpointed: _,
            } => Named::Ended {
                synchronization: None,
            },
        },
    ))
}

/// How the ID stands in the local journal of the repository at `root`.
#[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
fn found_local(
    connection: &rusqlite::Connection,
    root: &str,
    operation_id: OperationId,
) -> Result<Named, ReadError> {
    let Some(row) = recovery::local_row(connection, root, operation_id)? else {
        return Ok(Named::NotFound);
    };
    if !row.is_well_formed() {
        return Err(unreadable());
    }
    Ok(match row.standing() {
        LocalStanding::Abandoned => Named::Abandoned,
        LocalStanding::Completed | LocalStanding::Rejected => Named::Ended {
            synchronization: None,
        },
        // The row's own step is what says the hand-off is all it owes.
        // The refresh a resume runs would complete any pending row it
        // was given, also one whose write never happened.
        LocalStanding::Pending if row.owes_only_its_hand_off() => Named::HandOff,
        LocalStanding::Pending => left_to_its_request(
            connection,
            OperationFamily::Local,
            root,
            operation_id,
            Named::Unowned,
        )?,
    })
}

/// How the operation `operation_id` stands for the repository at
/// `root`, the root as its journal rows name it. One read of the three
/// journals; see `OperationResumeBinding::observe`.
#[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
fn named(
    service: &RepositoryService,
    root: &str,
    operation_id: OperationId,
) -> Result<Named, ReadError> {
    service.read_session(RepositoryOperation::Read, |connection| {
        // An index that has never held an operation of a journal may
        // lack that journal's table.
        if recovery::has_table(connection, recovery::REMOTE_JOURNAL_TABLE)?
            && let Some(found) = found_remote(connection, root, operation_id)?
        {
            return Ok(found);
        }
        if recovery::has_table(connection, recovery::KEY_JOURNAL_TABLE)?
            && let Some(found) = found_key(connection, operation_id)?
        {
            return Ok(found);
        }
        found_local(connection, root, operation_id)
    })
}

/// Whether a resume of the operation would take it up now: the one
/// question that decides both what a resume does and whether a result
/// offers one. `false` for an operation it would refuse, and when the
/// journals cannot be read: a resume is not offered on a guess.
pub(crate) fn takes_up(service: &RepositoryService, root: &str, operation_id: OperationId) -> bool {
    match named(service, root, operation_id) {
        Ok(Named::HandOff | Named::Synchronization(_)) => true,
        Ok(
            Named::NotFound
            | Named::Abandoned
            | Named::Ended { synchronization: _ }
            | Named::LeftToInspect { synchronization: _ }
            | Named::OriginalRequest { request: _ }
            | Named::Fenced
            | Named::Unowned
            | Named::NotResumable,
        )
        | Err(_) => false,
    }
}

/// The index brought up to date, and nothing else: what a hand-off does.
fn current() -> Answer {
    Answer {
        effects: Effects {
            discovery: DiscoveryEffect::Current,
            ..Effects::not_requested()
        },
        ..Answer::stopped(ResultCode::Ok)
    }
}

impl OperationResumeBinding {
    /// The scope key of the request's root. `None` for a root that
    /// cannot be resolved.
    fn scope_key(&self) -> Option<ScopeKey> {
        match self.place() {
            Place::Repository(root) => ScopeKey::for_repository(root),
            Place::NewRepository(root) => ScopeKey::for_new_repository(root),
            Place::Application => None,
        }
    }

    /// Reads how the operation stands. Nothing is written and no lease
    /// is taken. The three journals are asked in one read: the remote
    /// one first, whose synchronizations also have a row of the local
    /// journal under their ID for their index hand-off, then the key
    /// journal, then the local one.
    fn observe(&self, service: &RepositoryService) -> Result<(ScopeKey, Named), Answer> {
        let scope = self
            .scope_key()
            .ok_or_else(|| Answer::stopped(ResultCode::InvalidPath))?;
        let found = named(service, scope.as_str(), self.operation_id)
            .map_err(|error| Answer::of_read(&error))?;
        Ok((scope, found))
    }

    /// What a resume of an operation that stands as `found` is answered
    /// with when there is nothing for it to take up. `None` for an
    /// operation it takes up.
    fn refusal(found: &Named) -> Option<Answer> {
        Some(match found {
            Named::HandOff | Named::Synchronization(_) => return None,
            Named::NotFound => Answer::stopped(ResultCode::OperationNotFound),
            // As the journal refuses every repeat of an abandoned
            // operation's ID.
            Named::Abandoned => Answer::stopped(outcome::repository_error_code(
                RepositoryErrorKind::OperationAbandoned,
            )),
            Named::Ended { synchronization: _ } => Answer::stopped(ResultCode::AlreadyApplied),
            Named::LeftToInspect { synchronization: _ } => {
                Answer::stopped(ResultCode::RecoveryRequired)
            }
            // The request to send is the one that began the operation,
            // and it is there to be sent: its record is not finished.
            Named::OriginalRequest { request } => Answer {
                recovery: vec![outcome::original_request(request.clone())],
                ..Answer::stopped(ResultCode::OriginalRequestRequired)
            },
            // What the request that began it was, or would be, told:
            // the configuration it was begun under is no longer the
            // repository's. Nothing is offered, since nothing goes on.
            Named::Fenced => Answer::stopped(ResultCode::ExternalChange),
            // No request can be sent again for it. The repository is
            // what is looked at; `execute` adds the abandon of a pending
            // row of the local journal alone that no call is at work
            // under, by the rule every result that says so follows.
            Named::Unowned => Answer::stopped(ResultCode::RecoveryRequired),
            Named::NotResumable => Answer::stopped(ResultCode::InvalidInput),
        })
    }

    /// The request whose unfinished record holds the operation, in the
    /// journal `family`: the one that owns it. `None` when no such
    /// record does.
    ///
    /// With `takes_over`, the owner's record is also entered, as that
    /// request's own next call would enter it: its attempt is raised.
    /// A call of the owner that is still running, and that loses the
    /// operation to this one, then settles nothing when it comes back:
    /// it would otherwise finish or delete the owner's record from a
    /// row this call has since changed. The record is left for the
    /// owner's next call, which answers from what the operation came
    /// to.
    ///
    /// `Err` when the records cannot be read or written: nothing is
    /// taken up on that.
    fn owner(
        &self,
        service: &RepositoryService,
        scope: &ScopeKey,
        family: OperationFamily,
        takes_over: bool,
    ) -> Result<Option<Owner>, Answer> {
        let operation_id = self.operation_id;
        let holder = service
            .read_session(RepositoryOperation::Read, |connection| {
                Ok(records::operation_holder(
                    connection,
                    family,
                    scope.as_str(),
                    operation_id,
                )?)
            })
            .map_err(|error| Answer::of_read(&error))?;
        let Some(holder) = holder else {
            return Ok(None);
        };
        let request =
            RequestId::parse(&holder).map_err(|_| Answer::stopped(ResultCode::InternalError))?;
        let attempt = if takes_over {
            // `None` when the record was finished or deleted meanwhile:
            // there is then nothing for a stale call to settle.
            service
                .enter_request(request)
                .map_err(|error| Answer::of_read(&error))?
        } else {
            None
        };
        Ok(Some(Owner { request, attempt }))
    }

    /// Finishes the owner's record with `answer`, the final answer of
    /// the synchronization this call took from it.
    ///
    /// The answer is the operation's, made by the synchronization's own
    /// binding from what this call saw: whether the push endpoint was
    /// asked before the operation was stopped is known to this call and
    /// to no later one. So it is the owner's answer as much as the
    /// resume's, and the owner's request, sent again, is given it.
    /// The record is changed only at the attempt this call raised it
    /// to. A write that fails leaves it accepted, and the owner's next
    /// call answers from the operation's row.
    fn settle_owner(service: &RepositoryService, owner: Owner, answer: &Answer) {
        let Owner {
            request,
            attempt: Some(attempt),
        } = owner
        else {
            return;
        };
        let _ = service.finish(request, attempt, answer);
    }

    /// The index hand-off of a local operation: a refresh under the
    /// operation's ID, which claims the row, observes the repository and
    /// completes the row. The caller has just read that the hand-off is
    /// all the row owes.
    fn hand_off(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        let service = call.service;
        if let Err(answer) = self.owner(service, &accepted.scope, OperationFamily::Local, true) {
            return Ran {
                answer,
                standing: Standing::Stopped { journal: None },
            };
        }
        call.session.report(
            &Subject::of(self, accepted).event(ProgressStage::Indexing, Effects::not_requested()),
        );
        let refreshed = service.refresh_repository(RefreshRepositoryRequest {
            root: accepted.root(),
            operation_id: self.operation_id,
        });
        // Nothing of the request is in flight, whatever its call came
        // to: its record goes unless it is finished.
        let stopped = |code| Ran {
            answer: Answer::stopped(code),
            standing: Standing::Stopped {
                journal: Some(NO_ROW),
            },
        };
        match refreshed {
            Ok(outcome) => match outcome::refresh_outcome_code(&outcome) {
                ResultCode::Ok => Ran {
                    answer: current(),
                    standing: Standing::Final,
                },
                // The refresh could not claim the row, which another
                // call is indexing under, or found the repository
                // changing while it observed it. It stored nothing. When
                // the row has ended all the same, another call ended
                // it: the end state holds and nothing is claimed.
                code => match self.observe(service) {
                    Ok((_, Named::Ended { synchronization: _ })) => Ran {
                        answer: Answer::stopped(ResultCode::AlreadyApplied),
                        standing: Standing::Final,
                    },
                    Ok(_) | Err(_) => stopped(code),
                },
            },
            // Only the error's kind is read: its text can hold a path
            // or a backend's message.
            Err(error) => stopped(outcome::repository_error_code(error.kind)),
        }
    }

    /// Takes up a synchronization, and reads what its own binding makes
    /// of the call as a result of this request.
    ///
    /// The operation's row decides what becomes of the synchronization's
    /// own record. For this request it decides one thing: an answer the
    /// operation has made final is stored, and anything else leaves no
    /// record of the resume, since the operation's own request is what
    /// holds whatever is still in flight.
    ///
    /// `takes_over` says the operation has not ended: this call takes
    /// it from whoever was running it.
    fn synchronize(
        &self,
        call: &CallContext<'_>,
        accepted: &Accepted,
        target: SynchronizationTarget,
        takes_over: bool,
    ) -> Ran {
        let owner = match self.owner(
            call.service,
            &accepted.scope,
            OperationFamily::Remote,
            takes_over,
        ) {
            Ok(owner) => owner,
            // Nothing is known of whose the operation is, or of whether
            // that request was asked to stop: nothing is called.
            Err(answer) => {
                return Ran {
                    answer,
                    standing: Standing::Stopped { journal: None },
                };
            }
        };
        // The request as the synchronization's binding is to see it:
        // with the operation it takes up in place of none.
        let operation = Accepted {
            request_id: accepted.request_id,
            attempt: accepted.attempt,
            scope: accepted.scope.clone(),
            family: Some(OperationFamily::Remote),
            operation_id: self.operation_id,
            position: accepted.position.clone(),
            expected: None,
        };
        let Ran { answer, standing } = sync::resumed(
            call,
            &operation,
            target,
            Subject::of(self, accepted),
            owner.map(|owner| owner.request),
        );
        let gone = Standing::Stopped {
            journal: Some(NO_ROW),
        };
        let ran = match standing {
            Standing::Final => Ran {
                answer,
                standing: Standing::Final,
            },
            // The synchronization has its outcome and its hand-off is
            // still owed, which a resume is what does.
            Standing::Owed => Ran {
                answer: Answer {
                    recovery: vec![RecoveryAction::for_operation(
                        RecoveryActionKind::OperationResume,
                        self.operation_id.to_string(),
                        [],
                    )],
                    ..answer
                },
                standing: gone,
            },
            Standing::Stopped { journal } => Ran {
                answer,
                standing: match journal {
                    // Nothing could be read of the operation: nothing is
                    // stored and nothing is deleted.
                    None => Standing::Stopped { journal: None },
                    // The operation was cancelled, by this call or
                    // before it: that is final for its ID.
                    Some(JournalRow::Final {
                        kind: FinalKind::Cancelled | FinalKind::RetainedForInspection,
                        owes_work: _,
                        checkpointed: _,
                    }) => Standing::Final,
                    Some(
                        JournalRow::Absent
                        | JournalRow::Pending(_)
                        | JournalRow::Final {
                            kind: FinalKind::Completed,
                            owes_work: _,
                            checkpointed: _,
                        },
                    ) => gone,
                },
            },
            // Neither is a synchronization's. Nothing is known then, and
            // nothing is stored or deleted on that.
            Standing::Unconfirmed
            | Standing::NotRun {
                journal:
                    JournalRow::Absent
                    | JournalRow::Pending(_)
                    | JournalRow::Final {
                        kind: _,
                        owes_work: _,
                        checkpointed: _,
                    },
            } => Ran {
                answer,
                standing: Standing::Stopped { journal: None },
            },
        };
        if let (Standing::Final, Some(owner)) = (&ran.standing, owner) {
            Self::settle_owner(call.service, owner, &ran.answer);
        }
        ran
    }

    /// The one call of the request: the operation is read again, and
    /// what it owes is done.
    fn call(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        let found = match self.observe(call.service) {
            Ok((_, found)) => found,
            // Nothing is known of the operation: the record is kept.
            Err(answer) => {
                return Ran {
                    answer,
                    standing: Standing::Stopped { journal: None },
                };
            }
        };
        match found {
            Named::HandOff => self.hand_off(call, accepted),
            Named::Synchronization(target) => self.synchronize(call, accepted, target, true),
            // A synchronization that has ended, met by a request whose
            // earlier call may be what ended it: the domain is asked
            // what the operation came to, which it answers from the row
            // and with no connection. The request is then not answered
            // as a no-op where its own call published, and is told of a
            // cancellation its own call made.
            Named::Ended {
                synchronization: Some(target),
            }
            | Named::LeftToInspect {
                synchronization: Some(target),
            } if self.entered_again => self.synchronize(call, accepted, target, false),
            Named::NotFound
            | Named::Abandoned
            | Named::Ended { synchronization: _ }
            | Named::LeftToInspect { synchronization: _ }
            | Named::OriginalRequest { request: _ }
            | Named::Fenced
            | Named::Unowned
            | Named::NotResumable => Ran {
                // It moved between this call's two reads of it.
                answer: Self::refusal(&found)
                    .unwrap_or_else(|| Answer::stopped(ResultCode::InternalError)),
                standing: Standing::Stopped {
                    journal: Some(NO_ROW),
                },
            },
        }
    }
}

impl Binding for OperationResumeBinding {
    fn command(&self) -> Command {
        Command::OperationResume
    }

    /// The request has no operation, so no row of its own is ever read.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }

    /// The repository whose journals hold the operation. A creation that
    /// was killed before it made its directory leaves a row for a root
    /// that is not there: such a root is named as the creation named it.
    fn place(&self) -> Place<'_> {
        if self.root.exists() {
            Place::Repository(&self.root)
        } else {
            Place::NewRepository(&self.root)
        }
    }

    /// The operation to take up.
    fn target(&self) -> String {
        self.operation_id.to_string()
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        let Self {
            // What the digest is begun with: the scope and the target.
            // The request holds nothing else.
            root: _,
            operation_id: _,
            // What a call found, and no part of the request.
            entered_again: _,
        } = self;
        builder
    }

    fn scope(&self, repository: Option<String>, _position: Option<&Position>) -> Scope {
        Scope {
            repository,
            ..Scope::default()
        }
    }

    /// Reads how the operation stands, and refuses a request that has
    /// nothing to take up before anything is recorded of it.
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        let (scope, found) = self.observe(call.service)?;
        if let Some(answer) = Self::refusal(&found) {
            return Err(answer);
        }
        // A synchronization's result can name the commit it published:
        // the request is placed on the branch that holds it.
        let position = match found {
            Named::Synchronization(target) => {
                sync::resumed_position(call.service, Path::new(scope.as_str()), target)?
            }
            Named::NotFound
            | Named::Abandoned
            | Named::Ended { synchronization: _ }
            | Named::LeftToInspect { synchronization: _ }
            | Named::HandOff
            | Named::OriginalRequest { request: _ }
            | Named::Fenced
            | Named::Unowned
            | Named::NotResumable => Position {
                base_ref: None,
                base_oid: None,
            },
        };
        Ok(Prepared {
            // The operation is another request's: this one has none.
            family: None,
            position,
            expected: None,
            observation: None,
        })
    }

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        self.call(call, accepted)
    }

    /// The operation is asked again how it stands:
    ///
    /// 1. It still owes what a resume does: the domain is called.
    /// 2. It is a synchronization that has ended, cancelled or not: the
    ///    domain is called all the same, and says what the operation
    ///    came to.
    /// 3. It is any other operation that has ended. The request's end
    ///    state holds, by its own earlier call or another's: the row
    ///    does not say which. The request is a no-op and is finished.
    /// 4. Anything else: there is nothing for this request to take up,
    ///    and nothing of it is in flight, so its record is deleted.
    fn reenter(
        &mut self,
        call: &CallContext<'_>,
        _accepted: &Accepted,
        journal: JournalRow,
    ) -> Result<Entered, Ran> {
        // No row is read for a request with no operation. Nothing is run
        // or settled on one that is.
        if journal != NO_ROW {
            return Err(Ran {
                answer: Answer::stopped(ResultCode::InternalError),
                standing: Standing::NotRun { journal },
            });
        }
        let found = match self.observe(call.service) {
            Ok((_, found)) => found,
            // Nothing is known of the operation: the record is kept.
            Err(answer) => {
                return Err(Ran {
                    answer,
                    standing: Standing::Stopped { journal: None },
                });
            }
        };
        self.entered_again = true;
        match &found {
            Named::HandOff
            | Named::Synchronization(_)
            | Named::Ended {
                synchronization: Some(_),
            }
            | Named::LeftToInspect {
                synchronization: Some(_),
            } => Ok(Entered::Unfinished),
            Named::Ended {
                synchronization: None,
            } => Err(Ran {
                answer: Answer::stopped(ResultCode::AlreadyApplied),
                standing: Standing::Final,
            }),
            Named::NotFound
            | Named::Abandoned
            | Named::LeftToInspect {
                synchronization: None,
            }
            | Named::OriginalRequest { request: _ }
            | Named::Fenced
            | Named::Unowned
            | Named::NotResumable => Err(Ran {
                answer: Self::refusal(&found)
                    .unwrap_or_else(|| Answer::stopped(ResultCode::InternalError)),
                standing: Standing::NotRun { journal: NO_ROW },
            }),
        }
    }

    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, _entered: &Entered) -> Ran {
        self.call(call, accepted)
    }
}
