//! `folder create`: one directory under `docs/` in the primary worktree,
//! and the refresh that puts it in the index. It needs no confirmation.
//!
//! It writes no canonical content and makes no commit, so it reports
//! `discovery` and no other effect. A directory that was made is a
//! durable change that no effect shows.
//!
//! **Its operation is the refresh's.** Making the directory records
//! nothing. The request's one operation is the refresh that follows,
//! and its row is the one row. What that row says of a request that is
//! entered again:
//!
//! - it has completed: the index was brought up to date after the
//!   directory was there, by a call whose answer was lost. The request
//!   is finished as a no-op, and nothing is looked at or made. Whether
//!   that call made the directory or found it is not recorded, and
//!   either way this one changes nothing;
//! - it is pending, or there is none: the directory may or may not be
//!   there, and the domain is called again. It makes what is missing,
//!   finds what is there, and takes the refresh up under the same ID.
//!
//! So a retry never answers `occupied_path` for a directory an earlier
//! call of the request made: a directory is never what occupies a path.
//!
//! **A refresh that fails** after the directory is there is `partial`,
//! `discovery_pending`. The domain has closed the refresh's row and left
//! the registration stale, so the request is finished and what is
//! offered is `index.refresh`, which any request can make.

use std::path::PathBuf;

use crate::results::{DiscoveryEffect, Effects, ResultCode, Scope};

use super::{
    super::{
        super::{
            CreateFolderError, CreateFolderOutcome, CreateFolderRequest, DocumentFolderPath,
            FolderDiscovery, OperationFamily, RepositoryService,
            recovery::{FinalKind, JournalRow},
        },
        command::Command,
        evidence::Position,
        identity::{FieldValue, IntentDigestBuilder},
    },
    Accepted, Answer, Binding, CallContext, Confirmed, EndedWithoutCheckpoint, Entered, Place,
    Prepared, Ran, Standing, refused, resolve_root,
};

/// The input of `folder create`: the root of a registered repository and
/// the folder's path below it, which begins with `docs/` and is written
/// with forward slashes on every platform.
pub struct FolderCreateInput {
    pub root: PathBuf,
    pub path: String,
}

pub(crate) struct FolderCreateBinding {
    root: PathBuf,
    path: String,
}

impl From<FolderCreateInput> for FolderCreateBinding {
    fn from(input: FolderCreateInput) -> Self {
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let FolderCreateInput { root, path } = input;
        Self { root, path }
    }
}

/// The answer for a directory that is there, made or found, by what
/// became of the refresh.
fn there(code: ResultCode, discovery: FolderDiscovery, made: bool) -> (Answer, bool) {
    let (code, effect, settled) = match discovery {
        FolderDiscovery::Current => (code, DiscoveryEffect::Current, true),
        FolderDiscovery::Pending => (ResultCode::DiscoveryPending, DiscoveryEffect::Pending, true),
        FolderDiscovery::PendingUnderOperation => (
            ResultCode::DiscoveryPending,
            DiscoveryEffect::Pending,
            false,
        ),
    };
    let answer = Answer {
        effects: Effects {
            discovery: effect,
            ..Effects::not_requested()
        },
        local_change: made,
        ..Answer::stopped(code)
    };
    (answer, settled)
}

impl FolderCreateBinding {
    /// The one domain call, under the recorded operation ID, and what
    /// it came to.
    fn call(&self, service: &RepositoryService, accepted: &Accepted) -> Ran {
        // The path was read in `prepare`, and a record is of a request
        // that passed it. One that does not parse now is refused all the
        // same, and nothing is made.
        let Ok(path) = DocumentFolderPath::parse(&self.path) else {
            return Ran {
                answer: Answer::stopped(ResultCode::InvalidInput),
                standing: Standing::Stopped {
                    journal: accepted.journal_row(service),
                },
            };
        };
        let result = service.create_folder(CreateFolderRequest {
            root: accepted.root(),
            path,
            operation_id: accepted.operation_id,
        });
        // What is left of a request that made nothing, or that was
        // refused: its row says, and a first call has none.
        let stopped = |answer| Ran {
            answer,
            standing: Standing::Stopped {
                journal: accepted.journal_row(service),
            },
        };
        match result {
            Ok(CreateFolderOutcome::Created(discovery)) => {
                Self::settled(there(ResultCode::Ok, discovery, true), stopped)
            }
            Ok(CreateFolderOutcome::AlreadyPresent(discovery)) => {
                Self::settled(there(ResultCode::AlreadyApplied, discovery, false), stopped)
            }
            Ok(CreateFolderOutcome::Occupied) => stopped(Answer::stopped(ResultCode::OccupiedPath)),
            Ok(CreateFolderOutcome::LimitExceeded) => {
                stopped(Answer::stopped(ResultCode::InvalidInput))
            }
            // Only the kind of an error is read: its text can hold a
            // path.
            Err(CreateFolderError {
                error,
                left_directories: false,
            }) => stopped(refused(error)),
            // A directory was made and the call failed before the index
            // was told: the same request goes on from what is there.
            Err(CreateFolderError {
                error,
                left_directories: true,
            }) => Ran {
                answer: Answer {
                    local_change: true,
                    ..refused(error)
                },
                standing: Standing::Owed,
            },
        }
    }

    /// A request whose directory is there is finished, unless the row
    /// of its refresh could not be closed: then the row says what is
    /// left of it.
    fn settled((answer, settled): (Answer, bool), stopped: impl FnOnce(Answer) -> Ran) -> Ran {
        if settled {
            Ran {
                answer,
                standing: Standing::Final,
            }
        } else {
            stopped(answer)
        }
    }
}

impl Binding for FolderCreateBinding {
    fn command(&self) -> Command {
        Command::FolderCreate
    }

    /// The request's row is a refresh's, which records no checkpoint
    /// step whatever it did: the binding reads it for itself.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }

    fn place(&self) -> Place<'_> {
        Place::Repository(&self.root)
    }

    /// The folder's path, as the request gives it.
    fn target(&self) -> String {
        self.path.clone()
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        let Self {
            // What the digest is begun with.
            root: _,
            path,
        } = self;
        builder.field("path", FieldValue::Text(path))
    }

    fn scope(&self, repository: Option<String>, _position: Option<&Position>) -> Scope {
        Scope {
            repository,
            ..Scope::default()
        }
    }

    /// Reads the path, which needs no file system, and resolves the
    /// repository. What is at the path is the domain's to find, under
    /// the repository lease.
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        DocumentFolderPath::parse(&self.path)
            .map_err(|_| Answer::stopped(ResultCode::InvalidInput))?;
        resolve_root(call.service, &self.root)?;
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

    /// See the module's description for what each row says.
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
                answer: there(ResultCode::AlreadyApplied, FolderDiscovery::Current, false).0,
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
