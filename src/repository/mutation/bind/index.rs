//! `index refresh` and `index rebuild`: the two commands that bring the
//! index up to what the repository holds. Neither writes canonical
//! content or Git, so each reports `discovery` and no other effect.
//!
//! Neither is an authoring write. The row of a refresh or a rebuild that
//! ran to its end has no checkpoint step, so such a row says nothing by
//! the authoring reading: the binding answers a request whose row has
//! completed from the row itself, and calls the domain again only for a
//! row that has not.
//!
//! A refresh or a rebuild that fails has made no durable effect. It is an
//! error with its code, and its row stays pending, which refuses every
//! other operation on the repository until the same request is sent
//! again; its record stays accepted for that.

use std::path::PathBuf;

use crate::results::{DiscoveryEffect, Effects, ResultCode, Scope};

use super::{
    super::{
        super::{
            OperationFamily, RebuildRepositoryRequest, RefreshRepositoryRequest, RepositoryService,
            recovery::{FinalKind, JournalRow},
        },
        command::Command,
        evidence::Position,
        identity::{IntentDigestBuilder, ScopeKey},
        outcome,
    },
    Accepted, Answer, Binding, CallContext, Confirmed, EndedWithoutCheckpoint, Entered, Place,
    Prepared, Ran, Standing, resolve_root,
};

/// The input of `index refresh`: the repository to observe again.
pub struct IndexRefreshInput {
    pub root: PathBuf,
}

/// The input of `index rebuild`: the repository to observe from nothing.
/// It need not be registered: a rebuild registers again a repository
/// that was enabled. One that never was is refused.
pub struct IndexRebuildInput {
    pub root: PathBuf,
}

#[derive(Clone, Copy)]
enum Verb {
    Refresh,
    Rebuild,
}

pub(crate) struct IndexBinding {
    verb: Verb,
    root: PathBuf,
}

impl From<IndexRefreshInput> for IndexBinding {
    fn from(input: IndexRefreshInput) -> Self {
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let IndexRefreshInput { root } = input;
        Self {
            verb: Verb::Refresh,
            root,
        }
    }
}

impl From<IndexRebuildInput> for IndexBinding {
    fn from(input: IndexRebuildInput) -> Self {
        let IndexRebuildInput { root } = input;
        Self {
            verb: Verb::Rebuild,
            root,
        }
    }
}

/// The index holds what the repository holds.
fn current() -> Answer {
    Answer {
        effects: Effects {
            discovery: DiscoveryEffect::Current,
            ..Effects::not_requested()
        },
        ..Answer::stopped(ResultCode::Ok)
    }
}

/// Whether a local operation's row says the operation ran to its end.
fn completed(journal: &JournalRow) -> bool {
    match journal {
        JournalRow::Final {
            kind: FinalKind::Completed,
            owes_work: false,
            checkpointed: _,
        } => true,
        JournalRow::Absent
        | JournalRow::Pending(_)
        | JournalRow::Final {
            kind: FinalKind::Completed | FinalKind::Cancelled | FinalKind::RetainedForInspection,
            owes_work: _,
            checkpointed: _,
        } => false,
    }
}

impl IndexBinding {
    /// The one domain call, under the recorded operation ID, and what it
    /// came to. A call that did not bring the index up to date has made
    /// no durable effect: it reports none, and what is left of it is its
    /// row's to say.
    fn call(&self, service: &RepositoryService, accepted: &Accepted) -> Ran {
        let root = accepted.root();
        let operation_id = accepted.operation_id;
        // Only the kind of an error is read: its text can hold a path or
        // a backend's message.
        //
        // A call that returned without an error has not always brought
        // the index up to date itself, and its row says whether the
        // operation has: only one that ran to its end has completed.
        // A rebuild also returns what the index holds when it found the
        // repository changing under it and stored nothing. A refresh
        // says the index is pending when it could not claim its row,
        // which is also what it says of a row that a call of the same
        // request beside this one has since completed.
        let done = |otherwise: ResultCode| match accepted.journal_row(service) {
            Some(journal) if completed(&journal) => ResultCode::Ok,
            Some(_) => otherwise,
            None => ResultCode::InternalError,
        };
        let code = match self.verb {
            Verb::Refresh => {
                match service.refresh_repository(RefreshRepositoryRequest { root, operation_id }) {
                    Ok(outcome) => match outcome::refresh_outcome_code(&outcome) {
                        ResultCode::Ok => ResultCode::Ok,
                        code => match done(code) {
                            // The answer of the refresh stands when its
                            // row cannot be read.
                            ResultCode::InternalError => code,
                            read => read,
                        },
                    },
                    Err(error) => outcome::repository_error_code(error.kind),
                }
            }
            Verb::Rebuild => {
                match service.rebuild_repository(RebuildRepositoryRequest { root, operation_id }) {
                    Ok(_) => done(ResultCode::Busy),
                    Err(error) => outcome::repository_error_code(error.kind),
                }
            }
        };
        if code == ResultCode::Ok {
            return Ran {
                answer: current(),
                standing: Standing::Final,
            };
        }
        Ran {
            answer: Answer::stopped(code),
            standing: Standing::Stopped {
                journal: accepted.journal_row(service),
            },
        }
    }
}

impl Binding for IndexBinding {
    fn command(&self) -> Command {
        match self.verb {
            Verb::Refresh => Command::IndexRefresh,
            Verb::Rebuild => Command::IndexRebuild,
        }
    }

    /// A refresh or a rebuild records no checkpoint step, whether it ran
    /// to its end or not: its completed row is its work done, and
    /// releasing the record for it would run that work a second time.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }

    fn place(&self) -> Place<'_> {
        Place::Repository(&self.root)
    }

    /// The repository as a whole: the command names nothing within it.
    fn target(&self) -> String {
        String::new()
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        // The command and the repository are what the digest is begun
        // with, and the binding holds nothing else.
        let Self { verb: _, root: _ } = self;
        builder
    }

    fn scope(&self, repository: Option<String>, _position: Option<&Position>) -> Scope {
        Scope {
            repository,
            ..Scope::default()
        }
    }

    /// A refresh needs the repository to be registered. A rebuild does
    /// not: it is what registers again a repository whose registration
    /// went with the index, so the root itself is asked for, and the
    /// domain operation refuses what is not a repository's root. A
    /// repository that was never enabled is refused by both.
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        let service = call.service;
        match self.verb {
            Verb::Refresh => {
                resolve_root(service, &self.root)?;
            }
            Verb::Rebuild => {
                if ScopeKey::for_repository(&self.root).is_none() {
                    return Err(Answer::stopped(ResultCode::InvalidPath));
                }
                // A rebuild brings back a registration that was lost. It
                // does not enable a repository: one that is not
                // registered and was never enabled is refused, before
                // anything is recorded for it. Whether it was is asked
                // of Git only for a repository that is not registered.
                if let Err(answer) = resolve_root(service, &self.root)
                    && answer.code == ResultCode::RepositoryNotEnabled
                {
                    return Err(answer);
                }
            }
        }
        Ok(Prepared {
            observation: None,
            family: Some(OperationFamily::Local),
            // Nothing is committed, so there is no branch to bound.
            position: Position {
                base_ref: None,
                base_oid: None,
            },
            expected: None,
        })
    }

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        self.call(call.service, accepted)
    }

    /// The evidence that the request's work is done is its operation's
    /// row: one that has completed brought the index up to date, and the
    /// request is finished with that result and no domain call. A row
    /// that has not is taken up again by the domain operation, which is
    /// the only thing that completes it.
    fn reenter(
        &mut self,
        _call: &CallContext<'_>,
        _accepted: &Accepted,
        journal: JournalRow,
    ) -> Result<Entered, Ran> {
        match journal {
            JournalRow::Final {
                kind: FinalKind::Completed,
                owes_work: false,
                checkpointed: _,
            } => Err(Ran {
                answer: current(),
                standing: Standing::Final,
            }),
            JournalRow::Absent | JournalRow::Pending(_) => Ok(Entered::Unfinished),
            // No local operation ends in another way or owes work once it
            // has completed. Nothing is run or settled on a row that
            // says so.
            JournalRow::Final {
                kind: FinalKind::Completed | FinalKind::Cancelled | FinalKind::RetainedForInspection,
                owes_work: _,
                checkpointed: _,
            } => Err(Ran {
                answer: Answer::stopped(ResultCode::InternalError),
                standing: Standing::NotRun { journal },
            }),
        }
    }

    fn as_confirmed(&mut self) -> Option<&mut dyn Confirmed> {
        None
    }

    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, _entered: &Entered) -> Ran {
        self.call(call.service, accepted)
    }
}
