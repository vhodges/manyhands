//! `remote add`: a remote of the repository's own Git configuration.
//!
//! A remote is no canonical content and no commit, so no effect shows it:
//! every effect is `not_requested`, `data` names the remote, and the
//! outcome says whether anything changed. The remote's location can hold
//! a credential. It is fed to the request's digest and given to the
//! domain operation, and is otherwise only ever written redacted.

use std::path::{Path, PathBuf};

use crate::results::{ResultCode, Scope, redact_url};

use super::{
    super::{
        super::{
            AddRemoteRequest, OperationFamily, RemoteChange, RemoteOutcome, RepositoryService,
            recovery::{FinalKind, JournalRow, PendingOperation},
        },
        command::Command,
        dto::{MutationDataDto, RemoteMutationDto},
        evidence::Position,
        identity::{FieldValue, IntentDigestBuilder},
        outcome,
    },
    Accepted, Answer, Binding, EndedWithoutCheckpoint, Entered, Prepared, Ran, Standing,
    resolve_root,
};

/// The input of `remote add`: the remote's name and its location, which
/// becomes its fetch and its push location.
pub struct RemoteAddInput {
    pub root: PathBuf,
    pub name: String,
    pub url: String,
}

/// The step `add_remote` records once the remote is in the repository's
/// configuration.
const REMOTE_CHANGED_STEP: &str = "remote_changed";

/// Whether the row of an `add_remote` says that the remote was added and
/// the operation has not ended: a change that no effect shows, with work
/// remaining.
pub(crate) fn added_and_unfinished(journal: Option<&JournalRow>) -> bool {
    match journal {
        Some(JournalRow::Pending(PendingOperation::Local { state: _, step })) => {
            step.as_deref() == Some(REMOTE_CHANGED_STEP)
        }
        Some(
            JournalRow::Absent
            | JournalRow::Pending(
                PendingOperation::Remote { phase: _ } | PendingOperation::KeyMaterial { phase: _ },
            )
            | JournalRow::Final {
                kind: FinalKind::Completed | FinalKind::Cancelled | FinalKind::RetainedForInspection,
                owes_work: _,
                checkpointed: _,
            },
        )
        | None => false,
    }
}

pub(crate) struct RemoteAddBinding {
    root: PathBuf,
    name: String,
    url: String,
}

impl From<RemoteAddInput> for RemoteAddBinding {
    fn from(input: RemoteAddInput) -> Self {
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let RemoteAddInput { root, name, url } = input;
        Self { root, name, url }
    }
}

impl RemoteAddBinding {
    /// What the command reports of its remote. The location is redacted:
    /// it is not a location to connect to.
    fn data(&self) -> MutationDataDto {
        MutationDataDto::Remote(RemoteMutationDto {
            name: Some(self.name.clone()),
            location: Some(redact_url(&self.url)),
        })
    }

    /// The input is one Git can write as a remote. The domain operation
    /// marks its row before it asks Git, so a name or a location Git
    /// refuses there would leave the row pending with nothing written;
    /// both are refused here, before the request is accepted.
    fn validate(&self) -> Result<(), Answer> {
        let Self { root: _, name, url } = self;
        let name_is_valid = !name.contains('\0') && git2::Remote::is_valid_name(name);
        let url_is_valid = !url.is_empty() && !url.chars().any(char::is_control);
        if name_is_valid && url_is_valid {
            Ok(())
        } else {
            Err(self.answer(ResultCode::InvalidRemote, false))
        }
    }

    fn answer(&self, code: ResultCode, local_change: bool) -> Answer {
        Answer {
            local_change,
            ..Answer::stopped(code).with_data(self.data())
        }
    }

    /// The one domain call, under the recorded operation ID, and what it
    /// came to.
    fn call(&self, service: &RepositoryService, accepted: &Accepted) -> Ran {
        let result = service.add_remote(AddRemoteRequest {
            root: accepted.root(),
            name: self.name.clone(),
            url: self.url.clone(),
            operation_id: accepted.operation_id,
        });
        match result {
            Ok(outcome) => {
                let code = outcome::remote_outcome_code(&outcome);
                let (added, standing) = match outcome {
                    RemoteOutcome::Changed => (true, Standing::Final),
                    RemoteOutcome::NoChange => (false, Standing::Final),
                    RemoteOutcome::IndexPending { authoritative } => (
                        match authoritative {
                            RemoteChange::Changed => true,
                            RemoteChange::NoChange => false,
                        },
                        Standing::Owed,
                    ),
                };
                Ran {
                    answer: self.answer(code, added),
                    standing,
                }
            }
            Err(error) => {
                // Only the kind is read: the error's text can hold the
                // location.
                let code = outcome::repository_error_code(error.kind);
                let journal = accepted.journal_row(service);
                Ran {
                    answer: self.answer(code, added_and_unfinished(journal.as_ref())),
                    standing: Standing::Stopped { journal },
                }
            }
        }
    }
}

impl Binding for RemoteAddBinding {
    fn command(&self) -> Command {
        Command::RemoteAdd
    }

    /// `add_remote` records no checkpoint step: its completed row is the
    /// remote in place, whichever call put it there.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn target(&self) -> String {
        self.name.clone()
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        let Self {
            // What the digest is begun with: the repository and the
            // target.
            root: _,
            name: _,
            url,
        } = self;
        // The location as the caller gave it, and not its redacted form:
        // two locations that differ only in a credential are two requests.
        builder.field("location", FieldValue::Text(url))
    }

    fn scope(&self, repository: Option<String>, _position: Option<&Position>) -> Scope {
        Scope {
            repository,
            remote: Some(self.name.clone()),
            ..Scope::default()
        }
    }

    fn prepare(&mut self, service: &RepositoryService) -> Result<Prepared, Answer> {
        self.validate()?;
        resolve_root(service, &self.root)?;
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
    /// row: one that has completed left the remote in place with the
    /// requested location, and the request is finished as a no-op with no
    /// domain call. Whether this request or an earlier one put the remote
    /// there the row does not say, and a no-op claims neither.
    ///
    /// A row that has not completed is taken up again by the domain
    /// operation. It reports no change when the remote is already there
    /// with the requested location, and completes the row and its
    /// hand-off when an earlier attempt added the remote.
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
                answer: self.answer(ResultCode::AlreadyApplied, false),
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
