//! `remote add` and `remote select`.
//!
//! `remote add` changes the repository's own Git configuration. A remote
//! is no canonical content and no commit, so no effect shows it: every
//! effect is `not_requested`, `data` names the remote, and the outcome
//! says whether anything changed. The remote's location can hold a
//! credential. It is fed to the request's digest and given to the domain
//! operation, and is otherwise only ever written redacted.
//!
//! `remote select` writes the publication remote into the Manyhands
//! configuration file and commits it on primary. That is an authoring
//! write of one file, and it is bound as one: the shared save code reads
//! its commit from Git at the configuration file's path.

use std::path::{Path, PathBuf};

use crate::{
    canonical,
    results::{Effects, ResultCode, Scope, redact_url},
};

use super::{
    super::{
        super::{
            AddRemoteRequest, ExpectedPathObservation, OperationFamily, RemoteChange,
            RemoteOutcome, RepositoryError, RepositoryErrorKind, RepositoryOperation,
            RepositoryService, SetPublicationRemoteRequest, owned_file_bytes,
            recovery::{FinalKind, JournalRow, PendingOperation},
        },
        command::Command,
        dto::{MutationDataDto, RemoteMutationDto},
        evidence::Position,
        identity::{FieldValue, IntentDigestBuilder},
        outcome::{self, SaveReport},
        records::RequestRecord,
    },
    Accepted, Answer, Binding, CallContext, EndedWithoutCheckpoint, Entered, Place, Prepared, Ran,
    Standing, resolve_root, save,
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

/// Whether Git takes `name` as the name of a remote.
fn is_remote_name(name: &str) -> bool {
    !name.contains('\0') && git2::Remote::is_valid_name(name)
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
        let name_is_valid = is_remote_name(name);
        let url_is_valid = !url.is_empty() && !url.chars().any(char::is_control);
        if name_is_valid && url_is_valid {
            Ok(())
        } else {
            Err(self.answer(ResultCode::InvalidRemote, false))
        }
    }

    /// Whether the repository at `root` has the remote with the requested
    /// location: `Some(false)` when it has the name with another, `None`
    /// when it has no remote of the name, and `Err` when Git could not
    /// be read. The Git error is dropped: its text can hold a location.
    fn in_place(&self, root: &Path) -> Result<Option<bool>, ()> {
        let repository = git2::Repository::open(root).map_err(|_| ())?;
        match repository.find_remote(&self.name) {
            Ok(remote) => Ok(Some(remote.url() == Some(self.url.as_str()))),
            Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(None),
            Err(_) => Err(()),
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

    fn place(&self) -> Place<'_> {
        Place::Repository(&self.root)
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

    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        let service = call.service;
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

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        self.call(call.service, accepted)
    }

    /// A row that has completed is answered here, with no domain call,
    /// from what the row recorded and what Git holds. That the row has
    /// completed does not show the remote to be in place: `add_remote`
    /// also completes a row an earlier attempt began when it refuses the
    /// request for a name conflict.
    ///
    /// - The row recorded `remote_changed`: this operation added the
    ///   remote. The request is finished with `ok`, whatever has been
    ///   done to the remote since.
    /// - Otherwise Git is asked. The remote is there with the requested
    ///   location: the request's end state holds and the change was not
    ///   its own, a no-op. It is there with another location: the name is
    ///   taken, and the request stopped with nothing in flight. It is not
    ///   there: the request is not done, and the domain operation is
    ///   called, as for a row that has not completed.
    ///
    /// A row that has not completed is taken up again by the domain
    /// operation. It reports no change when the remote is already there
    /// with the requested location, and completes the row and its
    /// hand-off when an earlier attempt added the remote.
    fn reenter(
        &mut self,
        call: &CallContext<'_>,
        accepted: &Accepted,
        journal: JournalRow,
    ) -> Result<Entered, Ran> {
        match journal {
            JournalRow::Final {
                kind: FinalKind::Completed,
                owes_work: false,
                checkpointed: _,
            } => {
                // What cannot be read answers the call and leaves the
                // record as it is.
                let unread = |answer, journal| Ran {
                    answer,
                    standing: Standing::NotRun { journal },
                };
                let step = match call
                    .service
                    .completed_local_step(&accepted.scope, accepted.operation_id)
                {
                    Ok(step) => step,
                    Err(error) => return Err(unread(Answer::of_read(&error), journal)),
                };
                if step.as_deref() == Some(REMOTE_CHANGED_STEP) {
                    return Err(Ran {
                        answer: self.answer(ResultCode::Ok, true),
                        standing: Standing::Final,
                    });
                }
                match self.in_place(&accepted.root()) {
                    Ok(Some(true)) => Err(Ran {
                        answer: self.answer(ResultCode::AlreadyApplied, false),
                        standing: Standing::Final,
                    }),
                    Ok(Some(false)) => Err(Ran {
                        answer: self.answer(ResultCode::RemoteNameConflict, false),
                        standing: Standing::Stopped {
                            journal: Some(journal),
                        },
                    }),
                    Ok(None) => Ok(Entered::Unfinished),
                    Err(()) => Err(unread(
                        Answer::stopped(ResultCode::RepositoryInaccessible),
                        journal,
                    )),
                }
            }
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

    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, _entered: &Entered) -> Ran {
        self.call(call.service, accepted)
    }
}

/// The input of `remote select`: the remote to publish to, or none, which
/// clears the selection.
pub struct RemoteSelectInput {
    pub root: PathBuf,
    pub name: Option<String>,
}

pub(crate) struct RemoteSelectBinding {
    root: PathBuf,
    name: Option<String>,
}

impl From<RemoteSelectInput> for RemoteSelectBinding {
    fn from(input: RemoteSelectInput) -> Self {
        let RemoteSelectInput { root, name } = input;
        Self { root, name }
    }
}

/// The configuration file as the worktree at `root` holds it.
fn configuration_file(root: &Path) -> Option<Vec<u8>> {
    owned_file_bytes(
        root,
        Path::new(canonical::CONFIG_PATH),
        RepositoryOperation::Read,
        root,
    )
    .ok()
    .flatten()
}

/// What the repository shows of an unfinished `remote select` whose
/// operation ran, without the request's input: a commit to the
/// configuration file since the request was accepted, and a
/// configuration file that is no longer the one it was accepted against.
pub(crate) fn recorded_effects(record: &RequestRecord, accepted: &Accepted) -> Effects {
    let Some(root) = record.scope.repository() else {
        return Effects::not_requested();
    };
    let root = Path::new(root);
    save::recorded_effects(
        root,
        &accepted.position,
        canonical::CONFIG_PATH,
        configuration_file(root).as_deref(),
        record.expected_digest.as_ref(),
    )
}

impl RemoteSelectBinding {
    fn data(&self) -> MutationDataDto {
        MutationDataDto::Remote(RemoteMutationDto {
            name: self.name.clone(),
            // The request names a remote and gives no location.
            location: None,
        })
    }

    /// A name that cannot be a remote's is refused before the request is
    /// accepted: the domain operation would report it as a Git failure.
    fn validate(&self) -> Result<(), Answer> {
        let Self { root: _, name } = self;
        match name {
            Some(name) if !is_remote_name(name) => {
                Err(Answer::stopped(ResultCode::InvalidRemote).with_data(self.data()))
            }
            Some(_) | None => Ok(()),
        }
    }
}

impl save::Save for RemoteSelectBinding {
    fn path(&self) -> String {
        canonical::CONFIG_PATH.to_owned()
    }

    /// The configuration is committed on primary, in the repository's own
    /// worktree.
    fn worktree(&self, root: &Path) -> PathBuf {
        root.to_owned()
    }

    fn context_file(&self, root: &Path) -> Option<Vec<u8>> {
        configuration_file(root)
    }

    /// The configuration is as the request intends when it names the
    /// requested publication remote, or none for a request that clears
    /// the selection. Nothing else in it is the request's.
    fn holds_bytes(&self, bytes: &[u8]) -> bool {
        // Every field is named: one added to the input does not compile
        // until it is part of what the request intends or left out here.
        let Self { root: _, name } = self;
        std::str::from_utf8(bytes)
            .ok()
            .and_then(|source| canonical::parse_repository_config(source).ok())
            .is_some_and(|config| config.publication_remote == *name)
    }

    /// The one domain call. It takes no expectation of the file: it
    /// refuses a configuration file that is not as committed.
    ///
    /// With no identity to commit as it returns an identity error, which
    /// is reported as a save's "identity required": the identity is the
    /// repository's, and no part of the request's input.
    ///
    /// The commit it names is the head it found whenever the
    /// configuration equals the request, whoever committed it. Its row
    /// says whether this operation committed: it records
    /// `publication_committed` when it did, and a row that has ended
    /// without that step committed nothing. The commit named by such a
    /// call is dropped here, so that a selection someone else committed
    /// after this request was accepted, which is in the request's range
    /// and changed its path, is not reported as its own.
    fn call_domain(
        &self,
        service: &RepositoryService,
        accepted: &Accepted,
    ) -> Result<SaveReport, RepositoryError> {
        let result = service.set_publication_remote(SetPublicationRemoteRequest {
            root: accepted.root(),
            name: self.name.clone(),
            operation_id: accepted.operation_id,
        });
        match result {
            Ok(outcome) => {
                let report = SaveReport::of_publication_remote(outcome);
                Ok(match accepted.journal_row(service) {
                    Some(JournalRow::Final {
                        kind: _,
                        owes_work: _,
                        checkpointed: false,
                    }) => report.naming_no_commit(),
                    // A row that passed its commit, one still pending,
                    // and one that could not be read: the evidence check
                    // decides.
                    Some(
                        JournalRow::Absent
                        | JournalRow::Pending(_)
                        | JournalRow::Final {
                            kind: _,
                            owes_work: _,
                            checkpointed: true,
                        },
                    )
                    | None => report,
                })
            }
            Err(error) if error.kind == RepositoryErrorKind::InvalidIdentity => {
                Ok(SaveReport::IdentityRequired)
            }
            Err(error) => Err(error),
        }
    }

    fn answer(&self, code: ResultCode, effects: Effects, _root: &Path) -> Answer {
        Answer {
            effects,
            ..Answer::stopped(code).with_data(self.data())
        }
    }

    /// The domain call needs nothing but the input.
    fn resume(&mut self, _service: &RepositoryService, _accepted: &Accepted) -> Result<(), Answer> {
        Ok(())
    }
}

impl Binding for RemoteSelectBinding {
    fn command(&self) -> Command {
        Command::RemoteSelect
    }

    /// `set_publication_remote` records `publication_committed` on every
    /// row that made its commit, as an authoring save records its
    /// checkpoint. A completed row without that step committed nothing:
    /// it found the configuration as requested, or stopped before its
    /// write. The record is released and the request runs as new, which
    /// finds the same and reports a no-op with no commit.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::DidNothing
    }

    fn place(&self) -> Place<'_> {
        Place::Repository(&self.root)
    }

    /// The remote, or nothing for a request that clears the selection.
    /// The digest tells that apart from a remote with an empty name.
    fn target(&self) -> String {
        self.name.clone().unwrap_or_default()
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        let Self { root: _, name } = self;
        builder.field(
            "name",
            match name {
                Some(name) => FieldValue::Text(name),
                None => FieldValue::Null,
            },
        )
    }

    fn scope(&self, repository: Option<String>, position: Option<&Position>) -> Scope {
        Scope {
            branch: position.and_then(|position| position.branch().map(str::to_owned)),
            remote: self.name.clone(),
            repository,
            ..Scope::default()
        }
    }

    /// The request is accepted against the configuration file as it is,
    /// on the branch that is checked out: the domain operation commits
    /// there, and refuses a branch that is not primary.
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        let service = call.service;
        self.validate()?;
        let repository = resolve_root(service, &self.root)?;
        let root = repository.root();
        let inaccessible = |_| Answer::stopped(ResultCode::RepositoryInaccessible);
        let head = git2::Repository::open(root)
            .and_then(|repository| {
                repository
                    .head()
                    .map(|head| String::from_utf8_lossy(head.name_bytes()).into_owned())
            })
            .map_err(inaccessible)?;
        Ok(Prepared {
            family: OperationFamily::Local,
            position: Position::of(root, &head).map_err(inaccessible)?,
            expected: Some(match configuration_file(root) {
                Some(bytes) => ExpectedPathObservation::from_bytes(&bytes),
                None => ExpectedPathObservation::Missing,
            }),
        })
    }

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        save::run(self, call.service, accepted)
    }

    fn reenter(
        &mut self,
        call: &CallContext<'_>,
        accepted: &Accepted,
        journal: JournalRow,
    ) -> Result<Entered, Ran> {
        save::reenter(self, call.service, accepted, journal).map(Entered::Authoring)
    }

    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, entered: &Entered) -> Ran {
        match entered.authoring() {
            Ok(before) => save::rerun(self, call.service, accepted, before),
            Err(ran) => ran,
        }
    }
}
