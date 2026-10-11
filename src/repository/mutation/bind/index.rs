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

use std::path::{Path, PathBuf};

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
    Accepted, Answer, Binding, EndedWithoutCheckpoint, Entered, Prepared, Ran, Standing,
    resolve_root,
};

/// The input of `index refresh`: the repository to observe again.
pub struct IndexRefreshInput {
    pub root: PathBuf,
}

/// The input of `index rebuild`: the repository to observe from nothing.
/// It need not be registered: a rebuild registers it.
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
        let code = match self.verb {
            Verb::Refresh => {
                match service.refresh_repository(RefreshRepositoryRequest { root, operation_id }) {
                    Ok(outcome) => outcome::refresh_outcome_code(&outcome),
                    Err(error) => outcome::repository_error_code(error.kind),
                }
            }
            Verb::Rebuild => {
                match service.rebuild_repository(RebuildRepositoryRequest { root, operation_id }) {
                    // A rebuild also returns what the index holds when it
                    // found the repository changing under it and stored
                    // nothing. Its row says which: only one that ran to
                    // its end has completed.
                    Ok(_) => match accepted.journal_row(service) {
                        Some(journal) if completed(&journal) => ResultCode::Ok,
                        Some(_) => ResultCode::Busy,
                        None => ResultCode::InternalError,
                    },
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

    fn root(&self) -> &Path {
        &self.root
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
    /// not: it is what registers a repository whose registration went
    /// with the index, so only the root itself is asked for, and the
    /// domain operation refuses what is not a repository's root.
    fn prepare(&mut self, service: &RepositoryService) -> Result<Prepared, Answer> {
        match self.verb {
            Verb::Refresh => {
                resolve_root(service, &self.root)?;
            }
            Verb::Rebuild => {
                if ScopeKey::for_repository(&self.root).is_none() {
                    return Err(Answer::stopped(ResultCode::InvalidPath));
                }
            }
        }
        Ok(Prepared {
            family: OperationFamily::Local,
            // Nothing is committed, so there is no branch to bound.
            position: Position {
                base_ref: None,
                base_oid: None,
            },
            expected: None,
        })
    }

    fn run(&self, service: &RepositoryService, accepted: &Accepted) -> Ran {
        self.call(service, accepted)
    }

    /// The evidence that the request's work is done is its operation's
    /// row: one that has completed brought the index up to date, and the
    /// request is finished with that result and no domain call. A row
    /// that has not is taken up again by the domain operation, which is
    /// the only thing that completes it.
    fn reenter(
        &mut self,
        _service: &RepositoryService,
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

    fn rerun(&self, service: &RepositoryService, accepted: &Accepted, _entered: &Entered) -> Ran {
        self.call(service, accepted)
    }
}
