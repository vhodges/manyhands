//! `remote add`, `remote select` and `remote remove`.
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
            AddRemoteRequest, ConfigurationInspection, ExpectedPathObservation, OperationFamily,
            RemoteChange, RemoteOutcome, RemoveRemoteRequest, RepositoryError, RepositoryErrorKind,
            RepositoryOperation, RepositoryService, SetPublicationRemoteRequest, owned_file_bytes,
            read_configuration_for,
            recovery::{FinalKind, JournalRow, PendingOperation},
            remote::state::PollingPolicy,
        },
        command::Command,
        dto::{
            MutationDataDto, PreviewDetailsDto, PreviewEffect, RemoteMutationDto,
            RemoteRemovalPreviewDto,
        },
        evidence::Position,
        identity::{FieldValue, IntentDigestBuilder},
        observe::{Observation, ObservationDigest},
        outcome::{self, SaveReport},
        records::RequestRecord,
    },
    Accepted, Answer, Binding, CallContext, Confirmed, EndedWithoutCheckpoint, Entered, Place,
    Prepared, Previewed, Ran, Standing, lost_refusal, refused, resolve_root, save,
};

/// The input of `remote add`: the remote's name and its location, which
/// becomes its fetch and its push location.
pub struct RemoteAddInput {
    pub root: PathBuf,
    pub name: String,
    pub url: String,
}

/// The step `add_remote` and `remove_remote` record once the
/// repository's configuration has the remote, or no longer has it.
const REMOTE_CHANGED_STEP: &str = "remote_changed";

/// Whether the row of an `add_remote` or a `remove_remote` says that
/// the remote was added or removed and the operation has not ended: a
/// change that no effect shows, with work remaining.
pub(crate) fn changed_and_unfinished(journal: Option<&JournalRow>) -> bool {
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
                    answer: self.answer(code, changed_and_unfinished(journal.as_ref())),
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
            observation: None,
            family: Some(OperationFamily::Local),
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

/// The input of `remote remove`: the remote to take out of the
/// repository's own Git configuration.
#[derive(Clone)]
pub struct RemoteRemoveInput {
    pub root: PathBuf,
    pub name: String,
}

/// `remote remove`. It needs confirmation.
///
/// **What its preview covers.** Whether the repository has the remote,
/// and where it fetches from and pushes to; whether it is the publication
/// remote; the remote's refspecs, and whether the repository holds
/// references they name and branches that follow the remote, which Git
/// removes with it; and the repository's polling policy, with the remote
/// polling observes. The command reads no identity.
///
/// **What Git removes with a remote.** Not the references under
/// `refs/remotes/<name>/` as such: every reference of the repository
/// that matches the destination of any of the remote's refspecs, fetch
/// or push. For a remote configured in the usual way those are its
/// tracking references. For a mirror, `+refs/*:refs/*`, or a remote
/// with a push refspec onto `refs/heads/*`, they are the repository's
/// own branches and tags. A preview cannot stand for that, so such a
/// remote is refused, at `prepare` and before every domain call, and
/// is never removed through this command.
///
/// Two more things are not the remote's and go with it. A remote named
/// `<name>/<more>` keeps its tracking references under this remote's
/// own prefix, where this remote's refspec matches them. And Git removes
/// the remote's configuration by a pattern that is not anchored, which
/// takes every entry of a remote named `<name>.<more>`. Either is
/// refused in the same way.
///
/// **A row that ended without a checkpoint step** says nothing here, as
/// for `remote add`: `remove_remote` records `remote_changed` when it
/// removed the remote, and completes a row with no step both when it
/// found no such remote and when it refused to remove the publication
/// remote.
pub(crate) struct RemoteRemoveBinding {
    root: PathBuf,
    name: String,
}

impl From<RemoteRemoveInput> for RemoteRemoveBinding {
    fn from(input: RemoteRemoveInput) -> Self {
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let RemoteRemoveInput { root, name } = input;
        Self { root, name }
    }
}

/// A remote as the repository's configuration has it.
struct Configured {
    /// The locations as Git holds them, credentials and all. They are fed
    /// to the observation's digest and go nowhere else.
    fetch: Option<String>,
    push: Option<String>,
    /// The locations as the remote list gives them: redacted. `None` for
    /// a remote that list leaves out.
    listed: Option<(String, String)>,
}

/// What a preview of `remote remove` covers, at `root`.
struct Removal {
    root: PathBuf,
    /// `None` when the repository has no remote of the name.
    remote: Option<Configured>,
    /// Whether the remote is the publication remote.
    selected: bool,
    /// The remote's refspecs as its configuration holds them, each with
    /// its direction, in the order Git reads them.
    refspecs: Vec<String>,
    /// Whether Git would remove any reference with the remote: one that
    /// matches the destination of one of its refspecs.
    tracking_refs: bool,
    /// Whether any of those is outside `refs/remotes/<name>/`, or is
    /// under it and is another remote's as well: under the prefix of a
    /// remote named `<name>/...`, or matched by the destination of a
    /// fetch refspec of any other remote. Such a reference is not this
    /// remote's own.
    foreign_refs: bool,
    /// The other remotes whose names begin with this one's and a `.` or
    /// a `/`, in name order: the ones Git could take a part of.
    siblings: Vec<String>,
    /// The fetch refspecs of every other remote of the repository, each
    /// with its remote's name, in name order: what says whether a
    /// reference is another remote's as well.
    other_refspecs: Vec<String>,
    /// Whether Git would remove with the remote's section a
    /// configuration entry that is not the remote's own, or could not
    /// remove one that is: an entry of the remote that is not in the
    /// repository's own configuration file.
    foreign_config: bool,
    /// Whether any local branch has the remote as its upstream's.
    upstreams: bool,
    polling: Option<PollingPolicy>,
}

impl Removal {
    /// Whether polling observes the remote.
    fn polling_affected(&self, name: &str) -> bool {
        self.polling
            .as_ref()
            .is_some_and(|polling| polling.remote_name.as_deref() == Some(name))
    }
}

/// A Git error met while observing. It is dropped: its text can hold a
/// location.
fn unreadable(_: git2::Error) -> Answer {
    Answer::stopped(ResultCode::RepositoryInaccessible)
}

impl RemoteRemoveBinding {
    /// What the command reports of its remote: its name. A removal gives
    /// no location.
    fn data(&self) -> MutationDataDto {
        MutationDataDto::Remote(RemoteMutationDto {
            name: Some(self.name.clone()),
            location: None,
        })
    }

    fn answer(&self, code: ResultCode, local_change: bool) -> Answer {
        Answer {
            local_change,
            ..Answer::stopped(code).with_data(self.data())
        }
    }

    /// A name that cannot be a remote's is refused before anything is
    /// read: the domain operation would report it as a Git failure.
    fn validate(&self) -> Result<(), Answer> {
        let Self { root: _, name } = self;
        if is_remote_name(name) {
            Ok(())
        } else {
            Err(self.answer(ResultCode::InvalidRemote, false))
        }
    }

    /// Whether the repository at `root` has a remote of the name. `Err`
    /// when Git could not be read.
    fn in_place(&self, root: &Path) -> Result<bool, ()> {
        let repository = git2::Repository::open(root).map_err(|_| ())?;
        match repository.find_remote(&self.name) {
            Ok(_) => Ok(true),
            Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(false),
            Err(_) => Err(()),
        }
    }

    /// Reads what a preview covers. Nothing is written and no lease is
    /// taken. An `Err` is a root the command cannot work at, or a read
    /// that failed.
    fn observe(&self, service: &RepositoryService) -> Result<Removal, Answer> {
        let resolved = resolve_root(service, &self.root)?;
        let root = resolved.root().to_owned();
        let repository = git2::Repository::open(&root).map_err(unreadable)?;
        let own = format!("refs/remotes/{}/", self.name);
        let mut siblings: Vec<String> = repository
            .remotes()
            .map_err(unreadable)?
            .iter()
            .flatten()
            .filter(|other| {
                other
                    .strip_prefix(self.name.as_str())
                    .is_some_and(|rest| rest.starts_with(['.', '/']))
            })
            .map(str::to_owned)
            .collect();
        siblings.sort_unstable();
        // The references of a remote named `<name>/...` are under this
        // remote's own prefix, and are not this remote's.
        let of_a_sibling: Vec<String> = siblings
            .iter()
            .filter(|other| other[self.name.len()..].starts_with('/'))
            .map(|other| format!("refs/remotes/{other}/"))
            .collect();
        // Every other remote, to ask of each whether it fetches into a
        // reference this remote's removal would delete.
        let mut names: Vec<String> = repository
            .remotes()
            .map_err(unreadable)?
            .iter()
            .flatten()
            .filter(|other| *other != self.name)
            .map(str::to_owned)
            .collect();
        names.sort_unstable();
        let mut others = Vec::new();
        let mut other_refspecs = Vec::new();
        for other in &names {
            let remote = repository.find_remote(other).map_err(unreadable)?;
            for refspec in remote.refspecs() {
                if refspec.direction() == git2::Direction::Fetch {
                    other_refspecs.push(format!(
                        "{other} {}",
                        String::from_utf8_lossy(refspec.bytes())
                    ));
                }
            }
            others.push(remote);
        }
        let also_another_s = |name: &str| {
            others.iter().any(|other| {
                other.refspecs().any(|refspec| {
                    refspec.direction() == git2::Direction::Fetch && refspec.dst_matches(name)
                })
            })
        };
        let mut refspecs = Vec::new();
        let (mut tracking_refs, mut foreign_refs) = (false, false);
        let remote = match repository.find_remote(&self.name) {
            Ok(remote) => {
                // As `git_remote_delete` does it: every reference of the
                // repository is asked of every refspec of the remote,
                // fetch and push alike, whether it matches the refspec's
                // destination. The match is libgit2's own.
                for refspec in remote.refspecs() {
                    refspecs.push(format!(
                        "{} {}",
                        match refspec.direction() {
                            git2::Direction::Fetch => "fetch",
                            git2::Direction::Push => "push",
                        },
                        String::from_utf8_lossy(refspec.bytes())
                    ));
                }
                let references = repository.references().map_err(unreadable)?;
                for reference in references {
                    let reference = reference.map_err(unreadable)?;
                    // A name that is not text is one Git would still
                    // match: it is taken to be removed, and not the
                    // remote's own.
                    let name = reference.name();
                    let removed = match name {
                        Some(name) => remote.refspecs().any(|refspec| refspec.dst_matches(name)),
                        None => true,
                    };
                    if removed {
                        tracking_refs = true;
                        let own = name.is_some_and(|name| {
                            name.starts_with(&own)
                                && !of_a_sibling.iter().any(|other| name.starts_with(other))
                                && !also_another_s(name)
                        });
                        if !own {
                            foreign_refs = true;
                        }
                    }
                }
                let listed = service
                    .list_remotes_redacted(&resolved)
                    .map_err(|error| Answer::of_read(&error))?
                    .items
                    .into_iter()
                    .find(|listed| listed.name == self.name)
                    .map(|listed| (listed.fetch_location, listed.push_location));
                Some(Configured {
                    fetch: remote.url().map(str::to_owned),
                    push: remote.pushurl().or(remote.url()).map(str::to_owned),
                    listed,
                })
            }
            Err(error) if error.code() == git2::ErrorCode::NotFound => None,
            Err(error) => return Err(unreadable(error)),
        };
        // As the domain operation reads it: the configuration file in
        // the repository's own worktree.
        let selected = match read_configuration_for(&root, RepositoryOperation::RemoveRemote)
            .map_err(refused)?
        {
            ConfigurationInspection::Valid(configuration) => {
                configuration.publication_remote.as_deref() == Some(self.name.as_str())
            }
            ConfigurationInspection::Missing | ConfigurationInspection::Invalid(_) => false,
        };
        let config = repository
            .config()
            .and_then(|mut config| config.snapshot())
            .map_err(unreadable)?;
        // Git removes the remote's section by deleting every entry whose
        // name matches `remote\.<name>\..+` anywhere in it: the pattern
        // is not anchored. Besides the remote's own `remote.<name>.<key>`
        // that takes every entry of a remote named `<name>.<more>`, and
        // of anything else whose name holds that text.
        let mut foreign_config = false;
        if remote.is_some() {
            let section = format!("remote.{}.", self.name);
            let mut entries = config.entries(None).map_err(unreadable)?;
            while let Some(entry) = entries.next() {
                let entry = entry.map_err(unreadable)?;
                // A name that is not text is taken to be matched.
                let Some(key) = entry.name() else {
                    foreign_config = true;
                    continue;
                };
                let matched = key
                    .find(&section)
                    .is_some_and(|at| key.len() > at + section.len());
                let own = key
                    .strip_prefix(&section)
                    .is_some_and(|rest| !rest.is_empty() && !rest.contains('.'));
                // Git deletes what it matches from the repository's own
                // configuration file, and fails on an entry that is not
                // there, with the remote's references already removed.
                let removable = own && entry.level() == git2::ConfigLevel::Local;
                if matched && !removable {
                    foreign_config = true;
                }
            }
        }
        let mut upstreams = false;
        let mut entries = config
            // The entries `git_remote_delete` looks through.
            .entries(Some("branch\\..+\\.remote"))
            .map_err(unreadable)?;
        while let Some(entry) = entries.next() {
            if entry.map_err(unreadable)?.value() == Some(self.name.as_str()) {
                upstreams = true;
            }
        }
        let polling = service
            .polling_policy(&root)
            .map_err(|error| Answer::of_read(&error))?;
        Ok(Removal {
            root,
            remote,
            selected,
            refspecs,
            tracking_refs,
            foreign_refs,
            siblings,
            other_refspecs,
            foreign_config,
            upstreams,
            polling,
        })
    }

    /// The digest of what was observed: what a confirmation stores, and
    /// what a later observation is compared with.
    fn digest(&self, removal: &Removal) -> ObservationDigest {
        let Removal {
            // The request's scope.
            root: _,
            remote,
            selected,
            refspecs,
            tracking_refs,
            foreign_refs,
            siblings,
            other_refspecs,
            foreign_config,
            upstreams,
            polling,
        } = removal;
        let optional = |observation: Observation, name: &str, value: Option<&str>| match value {
            Some(value) => observation.text(name, value),
            None => observation.absent(name),
        };
        let flag = |observation: Observation, name: &str, value: bool| {
            observation.text(name, if value { "yes" } else { "no" })
        };
        let command = Command::RemoteRemove.name();
        let observation = Observation::of(command);
        let observation = match remote {
            Some(Configured {
                fetch,
                push,
                // The redacted form of the same two.
                listed: _,
            }) => {
                let observation = observation.text("remote", "configured");
                let observation = optional(observation, "fetch", fetch.as_deref());
                optional(observation, "push", push.as_deref())
            }
            None => observation.absent("remote").absent("fetch").absent("push"),
        };
        let observation = flag(observation, "selected", *selected);
        let mut observation = observation.text("refspecs", &refspecs.len().to_string());
        for refspec in refspecs {
            observation = observation.text("refspec", refspec);
        }
        let observation = flag(observation, "tracking_refs", *tracking_refs);
        let observation = flag(observation, "foreign_refs", *foreign_refs);
        let mut observation = observation.text("siblings", &siblings.len().to_string());
        for sibling in siblings {
            observation = observation.text("sibling", sibling);
        }
        let mut observation = observation.text("other_refspecs", &other_refspecs.len().to_string());
        for refspec in other_refspecs {
            observation = observation.text("other_refspec", refspec);
        }
        let observation = flag(observation, "foreign_config", *foreign_config);
        let observation = flag(observation, "upstreams", *upstreams);
        let observation = match polling {
            Some(PollingPolicy {
                enabled,
                paused,
                interval_seconds,
                remote_name,
            }) => {
                let observation = flag(observation, "polling_enabled", *enabled);
                let observation = flag(observation, "polling_paused", *paused)
                    .text("polling_interval", &interval_seconds.to_string());
                optional(observation, "polling_remote", remote_name.as_deref())
            }
            None => observation
                .absent("polling_enabled")
                .absent("polling_paused")
                .absent("polling_interval")
                .absent("polling_remote"),
        };
        ObservationDigest::of_state(command, observation.finish())
    }

    /// The checks the domain operation would end the request for, in the
    /// order it makes them, on what was observed. An `Err` is a
    /// rejection, or a no-op when the repository has no such remote.
    fn check(&self, removal: &Removal) -> Result<(), Answer> {
        if removal.selected {
            return Err(self.answer(ResultCode::PublicationRemoteInUse, false));
        }
        if removal.remote.is_none() {
            return Err(self.answer(ResultCode::AlreadyApplied, false));
        }
        self.removable(removal)
    }

    /// Refuses a remote whose removal would take with it something that
    /// is not its own: a reference that is not its tracking reference (a
    /// branch, a tag, another remote's, or one another remote fetches
    /// into as well), or a configuration entry of another remote. And
    /// one whose removal Git would leave half done: a remote with a
    /// setting outside the repository's own configuration file. No preview is given of it and it is not removed;
    /// it is not a remote this command can remove.
    fn removable(&self, removal: &Removal) -> Result<(), Answer> {
        if removal.foreign_refs || removal.foreign_config {
            Err(self.answer(ResultCode::InvalidRemote, false))
        } else {
            Ok(())
        }
    }

    /// Whether the request's operation removed the remote and has not
    /// ended: its row is pending, and holds the step that says so or the
    /// remote is gone. A call that removed the remote and could not
    /// record the step leaves the second.
    fn removed_and_unfinished(&self, root: &Path, journal: Option<&JournalRow>) -> bool {
        changed_and_unfinished(journal)
            || (matches!(
                journal,
                Some(JournalRow::Pending(PendingOperation::Local {
                    state: _,
                    step: None
                }))
            ) && self.in_place(root) == Ok(false))
    }

    /// The one domain call, under the recorded operation ID, and what it
    /// came to.
    fn call(&self, service: &RepositoryService, accepted: &Accepted) -> Ran {
        let root = accepted.root();
        let result = service.remove_remote(RemoveRemoteRequest {
            root: root.clone(),
            name: self.name.clone(),
            operation_id: accepted.operation_id,
        });
        match result {
            Ok(outcome) => {
                let code = outcome::remote_outcome_code(&outcome);
                let (removed, standing) = match outcome {
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
                    answer: self.answer(code, removed),
                    standing,
                }
            }
            Err(error) => {
                // Only the kind is read: the error's text can hold a
                // location.
                let code = outcome::repository_error_code(error.kind);
                let journal = accepted.journal_row(service);
                Ran {
                    answer: self.answer(code, self.removed_and_unfinished(&root, journal.as_ref())),
                    standing: Standing::Stopped { journal },
                }
            }
        }
    }

    /// A row that has completed: what the request is answered with, and
    /// what becomes of its record. No domain call is made.
    ///
    /// That the row has completed does not show the remote to be gone:
    /// `remove_remote` also completes its row when it refuses to remove
    /// the publication remote.
    ///
    /// - The row recorded `remote_changed`: this operation removed the
    ///   remote. The request is finished with `ok`, whatever has been
    ///   done since, a remote of the same name added again among it.
    /// - Otherwise Git is asked. No such remote is there: the request's
    ///   end state holds and the change was not its own, a no-op. One is
    ///   there: the operation ended having done nothing, and that answer
    ///   was lost; see `lost_refusal`.
    fn ended(&self, call: &CallContext<'_>, accepted: &Accepted, journal: JournalRow) -> Ran {
        // What cannot be read answers the call and leaves the record as
        // it is.
        let unread = |answer: Answer, journal| Ran {
            answer: answer.with_data(self.data()),
            standing: Standing::NotRun { journal },
        };
        let step = match call
            .service
            .completed_local_step(&accepted.scope, accepted.operation_id)
        {
            Ok(step) => step,
            Err(error) => return unread(Answer::of_read(&error), journal),
        };
        if step.as_deref() == Some(REMOTE_CHANGED_STEP) {
            return Ran {
                answer: self.answer(ResultCode::Ok, true),
                standing: Standing::Final,
            };
        }
        match self.in_place(&accepted.root()) {
            Ok(false) => Ran {
                answer: self.answer(ResultCode::AlreadyApplied, false),
                standing: Standing::Final,
            },
            Ok(true) => Ran {
                answer: lost_refusal(
                    self.observe(call.service).and_then(|removal| {
                        self.check(&removal)?;
                        Ok(self.digest(&removal))
                    }),
                    call.previewed().as_ref(),
                )
                .with_data(self.data()),
                standing: Standing::Stopped {
                    journal: Some(journal),
                },
            },
            Err(()) => unread(Answer::stopped(ResultCode::RepositoryInaccessible), journal),
        }
    }
}

impl Binding for RemoteRemoveBinding {
    fn command(&self) -> Command {
        Command::RemoteRemove
    }

    /// `remove_remote` records no checkpoint step, and completes a row
    /// with no step of its work both for a remote it did not find and for
    /// one it refused to remove. The record is kept, and `reenter`
    /// answers from the step the row holds and from what Git has.
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
            // target. The request holds nothing else.
            root: _,
            name: _,
        } = self;
        builder
    }

    fn scope(&self, repository: Option<String>, _position: Option<&Position>) -> Scope {
        Scope {
            repository,
            remote: Some(self.name.clone()),
            ..Scope::default()
        }
    }

    /// Validates the input and observes. No check is made of what the
    /// preview covers: the boundary compares the digest with the
    /// confirmation's, and refuses a request whose preview is stale as
    /// `external_change`, whatever it is that has changed. A remote that
    /// was selected for publication since the preview is one such change.
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        self.validate()?;
        let removal = self
            .observe(call.service)
            .map_err(|answer| answer.with_data(self.data()))?;
        // The one check made here. It is of no state a preview can have
        // covered: no confirmation is ever given of such a remote.
        self.removable(&removal)?;
        Ok(Prepared {
            family: Some(OperationFamily::Local),
            // Nothing is committed, so there is no branch to bound.
            position: Position {
                base_ref: None,
                base_oid: None,
            },
            expected: None,
            observation: Some(self.digest(&removal).to_stored()),
        })
    }

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        self.call(call.service, accepted)
    }

    /// In order:
    ///
    /// 1. A row that has completed is answered from the step it holds
    ///    and from what Git has, with no domain call; see `ended`.
    /// 2. A row that holds a step removed the remote. The request's own
    ///    work has changed what would be observed, so nothing is
    ///    compared, and the confirmation is the request's however old it
    ///    is: the domain call completes the row and its hand-off. Not so
    ///    while a remote of the name is there again: the domain call
    ///    would remove it, so the answer is `external_change` and the
    ///    record stays as it is.
    /// 3. With no step recorded, nothing of the request's is known to be
    ///    done. When the repository has no such remote, the request's
    ///    end state holds: with no row it is a no-op and is finished,
    ///    and a row that is pending is the domain's to end. Otherwise
    ///    what was previewed is observed again and compared: a
    ///    difference is `external_change`, and nothing is removed.
    fn reenter(
        &mut self,
        call: &CallContext<'_>,
        accepted: &Accepted,
        journal: JournalRow,
    ) -> Result<Entered, Ran> {
        let not_run = |answer: Answer, journal: JournalRow| Ran {
            answer: answer.with_data(self.data()),
            standing: Standing::NotRun { journal },
        };
        let step = match &journal {
            JournalRow::Absent => None,
            JournalRow::Pending(PendingOperation::Local { state: _, step }) => step.clone(),
            JournalRow::Final {
                kind: FinalKind::Completed,
                owes_work: false,
                checkpointed: _,
            } => return Err(self.ended(call, accepted, journal)),
            // No local operation is in another journal, ends in another
            // way or owes work once it has completed. Nothing is run or
            // settled on a row that says so.
            JournalRow::Pending(
                PendingOperation::Remote { phase: _ } | PendingOperation::KeyMaterial { phase: _ },
            )
            | JournalRow::Final {
                kind: FinalKind::Completed | FinalKind::Cancelled | FinalKind::RetainedForInspection,
                owes_work: _,
                checkpointed: _,
            } => {
                return Err(not_run(Answer::stopped(ResultCode::InternalError), journal));
            }
        };
        if step.is_some() {
            // The domain operation removes whatever remote has the name
            // when it is called, also under a row that has removed one.
            // A remote that is there now was added since, and nobody was
            // shown it: the operation is not called while it stands.
            return match self.in_place(&accepted.root()) {
                Ok(false) => Ok(Entered::Unfinished),
                Ok(true) => Err(not_run(
                    self.answer(ResultCode::ExternalChange, true),
                    journal,
                )),
                Err(()) => Err(not_run(
                    Answer::stopped(ResultCode::RepositoryInaccessible),
                    journal,
                )),
            };
        }
        match self.in_place(&accepted.root()) {
            Ok(true) => {}
            Ok(false) if journal == JournalRow::Absent => {
                return Err(Ran {
                    answer: self.answer(ResultCode::AlreadyApplied, false),
                    standing: Standing::Final,
                });
            }
            Ok(false) => return Ok(Entered::Unfinished),
            Err(()) => {
                return Err(not_run(
                    Answer::stopped(ResultCode::RepositoryInaccessible),
                    journal,
                ));
            }
        }
        let removal = match self.observe(call.service) {
            Ok(removal) => removal,
            Err(answer) => return Err(not_run(answer, journal)),
        };
        if let Err(answer) = self.removable(&removal) {
            return Err(not_run(answer, journal));
        }
        // A request whose record holds no consent that can be read has
        // none that anything could be compared with: nothing is run.
        if call.previewed() != Some(self.digest(&removal)) {
            return Err(not_run(
                Answer::stopped(ResultCode::ExternalChange),
                journal,
            ));
        }
        Ok(Entered::Unfinished)
    }

    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, _entered: &Entered) -> Ran {
        self.call(call.service, accepted)
    }
}

impl Confirmed for RemoteRemoveBinding {
    fn preview(&mut self, call: &CallContext<'_>) -> Result<Previewed, Answer> {
        self.validate()?;
        let removal = self
            .observe(call.service)
            .map_err(|answer| answer.with_data(self.data()))?;
        self.check(&removal)?;
        let mut effects = vec![PreviewEffect::RemoteRemoved];
        if removal.tracking_refs {
            effects.push(PreviewEffect::RemoteTrackingRefsRemoved);
        }
        if removal.upstreams {
            effects.push(PreviewEffect::BranchUpstreamsRemoved);
        }
        let listed = removal
            .remote
            .as_ref()
            .and_then(|remote| remote.listed.clone());
        Ok(Previewed {
            observation: self.digest(&removal).to_stored(),
            effects,
            details: PreviewDetailsDto::RemoteRemoval(RemoteRemovalPreviewDto {
                root: removal.root.to_string_lossy().into_owned(),
                name: self.name.clone(),
                fetch_location: listed.as_ref().map(|(fetch, _)| fetch.clone()),
                push_location: listed.map(|(_, push)| push),
                // `check` refused a remote that is selected.
                publication_affected: removal.selected,
                polling_affected: removal.polling_affected(&self.name),
            }),
        })
    }
}
