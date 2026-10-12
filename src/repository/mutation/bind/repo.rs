//! `repo create` and `repo enable`: the two commands that make a
//! repository a Manyhands repository. Each needs confirmation.
//!
//! `repo remove`, which takes a repository out of the repository list
//! again, is in `repo_remove`.
//!
//! Enablement writes the configuration file and commits it, so each
//! reports `write`, `checkpoint` and `discovery`, and carries no data.
//!
//! **What its preview covers.** For `repo enable`: the checked-out
//! branch and its commit, or that the branch is unborn; whether the
//! worktree is clean; and whether a configuration file is there. For
//! `repo create`: whether the target is absent, an empty directory or
//! occupied. For both, when the request supplies no identity: the
//! identity Git would commit as, and the level of configuration it comes
//! from.
//!
//! **Which commit is the request's.** An initialization commit is taken
//! as the request's own on the evidence of the request's operation, never
//! on what Git's history shows alone: the commit the domain names when it
//! makes one, and otherwise, on a retry or after a lost result, the commit
//! that put the configuration file on the branch, when the operation's row
//! holds one of the two steps only an operation that wrote that file
//! records. A row with neither step wrote no configuration, and no commit
//! is its request's whatever the branch holds.
//!
//! **A row that ended without a checkpoint step** says nothing for these
//! commands; see `ended_without_checkpoint`.

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use git2::{ConfigLevel, Repository};

use crate::{
    canonical,
    results::{ResultCode, Scope},
};

use super::{
    super::{
        super::{
            CommitIdentity, ConfigurationInspection, CreateRepositoryRequest,
            EnableRepositoryOutcome, EnableRepositoryRequest, INITIALIZATION_MESSAGE,
            OperationFamily, RepositoryErrorKind, RepositoryOperation, RepositoryService,
            WorktreeState, canonical_configuration, canonical_repository_root, checked_out_branch,
            read_configuration_for,
            recovery::{FinalKind, JournalRow, LocalStep, PendingOperation},
            resolve_identity, validate_identity, worktree_state,
        },
        command::Command,
        confirm::scope_key,
        dto::{
            PreviewDetailsDto, PreviewEffect, PreviewIdentityDto, PreviewIdentitySource,
            RepositoryPreviewDto,
        },
        evidence::Position,
        identity::{FieldValue, IntentDigestBuilder},
        observe::{Observation, ObservationDigest},
        outcome::{self, SaveReport},
    },
    Accepted, Answer, Binding, CallContext, Confirmed, EndedWithoutCheckpoint, Entered, Place,
    Prepared, Previewed, Ran, Recorded, Standing, lost_refusal, refused,
};

/// The input of `repo create`: the directory to make a repository in,
/// which is absent or empty, in a parent that exists.
#[derive(Clone)]
pub struct RepoCreateInput {
    pub root: PathBuf,
    pub primary_branch: String,
    /// The identity the initialization commit is made as, which is
    /// written to the new repository's own Git configuration. With none,
    /// the identity Git finds outside any repository is used, and is
    /// written there as well.
    pub identity: Option<CommitIdentity>,
    /// The short code prefix the configuration is to hold.
    pub slug_prefix: Option<String>,
}

/// The input of `repo enable`: the root of a Git repository that is not
/// yet a Manyhands repository.
#[derive(Clone)]
pub struct RepoEnableInput {
    pub root: PathBuf,
    /// The branch that is to be primary. It must be the one checked out.
    pub primary_branch: String,
    /// The identity the initialization commit is made as, which is
    /// written to the repository's own Git configuration. With none, the
    /// identity Git already finds is used, and nothing is written.
    pub identity: Option<CommitIdentity>,
    /// The short code prefix the configuration is to hold.
    pub slug_prefix: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Verb {
    Create,
    Enable,
}

pub(crate) struct RepoBinding {
    verb: Verb,
    root: PathBuf,
    primary_branch: String,
    identity: Option<CommitIdentity>,
    slug_prefix: Option<String>,
    /// The identity the domain call is given, which it writes to the
    /// repository's own configuration: the one supplied, and for a create
    /// that supplies none, the one found outside any repository. Set
    /// before each domain call.
    acting: Option<CommitIdentity>,
}

impl From<RepoCreateInput> for RepoBinding {
    fn from(input: RepoCreateInput) -> Self {
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let RepoCreateInput {
            root,
            primary_branch,
            identity,
            slug_prefix,
        } = input;
        Self {
            verb: Verb::Create,
            root,
            primary_branch,
            identity,
            slug_prefix,
            acting: None,
        }
    }
}

impl From<RepoEnableInput> for RepoBinding {
    fn from(input: RepoEnableInput) -> Self {
        let RepoEnableInput {
            root,
            primary_branch,
            identity,
            slug_prefix,
        } = input;
        Self {
            verb: Verb::Enable,
            root,
            primary_branch,
            identity,
            slug_prefix,
            acting: None,
        }
    }
}

/// The two steps a local row holds only when its operation wrote the
/// configuration file: enablement records the first once the file is
/// written and the second once it is committed, and records neither on a
/// repository it found enabled.
const CONFIGURATION_WRITTEN_STEP: LocalStep = LocalStep::ConfigurationWritten;
const INITIALIZATION_COMMITTED_STEP: LocalStep = LocalStep::InitializationCommitted;

/// Where HEAD is, as far as enablement asks.
enum Head {
    /// On a branch that has no commit yet. `primary_exists` says the
    /// branch the request names exists all the same, which enablement
    /// refuses: it would have to move HEAD onto a branch it did not make.
    Unborn {
        name: String,
        primary_exists: bool,
    },
    Branch {
        name: String,
        commit: git2::Oid,
    },
    Detached,
}

/// What stands at the root a create is to make.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Target {
    Absent,
    EmptyDirectory,
    /// A file, a link, or a directory that holds something.
    Occupied,
}

/// The state a preview covers.
enum State {
    Create {
        target: Target,
    },
    Enable {
        head: Head,
        worktree: WorktreeState,
        configuration: ConfigurationInspection,
    },
}

/// What a request found, at `root`.
struct Observed {
    root: PathBuf,
    state: State,
    /// The identity Git would commit as and the level it is configured
    /// at, read only for a request that supplies none.
    effective: Option<(CommitIdentity, PreviewIdentitySource)>,
}

fn identity_source(level: ConfigLevel) -> Option<PreviewIdentitySource> {
    match level {
        ConfigLevel::Local => Some(PreviewIdentitySource::Repository),
        ConfigLevel::Global => Some(PreviewIdentitySource::Global),
        ConfigLevel::XDG => Some(PreviewIdentitySource::Xdg),
        ConfigLevel::System => Some(PreviewIdentitySource::System),
        ConfigLevel::ProgramData => Some(PreviewIdentitySource::ProgramData),
        ConfigLevel::App => Some(PreviewIdentitySource::Application),
        // No identity is looked for at these.
        ConfigLevel::Worktree | ConfigLevel::Highest => None,
    }
}

fn inaccessible(_: git2::Error) -> Answer {
    Answer::stopped(ResultCode::RepositoryInaccessible)
}

/// Whether the directory at `root` is the root of a Git repository's
/// worktree: exactly this directory, never a repository above it.
fn is_repository_root(root: &Path) -> bool {
    Repository::open_ext(root, git2::RepositoryOpenFlags::NO_SEARCH, &[] as &[&OsStr])
        .is_ok_and(|repository| !repository.is_bare())
}

/// The commit that put the configuration file on the branch `position`
/// names: the newest commit on the branch's first-parent line that holds
/// the file and whose parent does not. `None` when the branch does not
/// hold the file, or held it already at the recorded commit.
///
/// The commit is named only when it is one enablement makes: its message
/// is enablement's, and the configuration it holds is `written`, the
/// file the request's operation wrote, when the caller has the request's
/// input to say what that is. A commit of the file that someone made by
/// hand, or that another repository put at the path holds, is none of
/// the request's.
fn initialization_commit(
    root: &Path,
    position: &Position,
    written: Option<&str>,
) -> Result<Option<git2::Oid>, git2::Error> {
    let repository = Repository::open(root)?;
    let absent = |error: &git2::Error| {
        matches!(
            error.code(),
            git2::ErrorCode::NotFound | git2::ErrorCode::UnbornBranch
        )
    };
    let tip = match position
        .base_ref
        .as_deref()
        .map(|name| repository.refname_to_id(name))
    {
        Some(Ok(tip)) => tip,
        Some(Err(error)) if !absent(&error) => return Err(error),
        Some(Err(_)) | None => match repository.head().and_then(|head| head.peel_to_commit()) {
            Ok(commit) => commit.id(),
            Err(error) if absent(&error) => return Ok(None),
            Err(error) => return Err(error),
        },
    };
    let holds = |commit: &git2::Commit<'_>| -> Result<bool, git2::Error> {
        match commit.tree()?.get_path(Path::new(canonical::CONFIG_PATH)) {
            Ok(_) => Ok(true),
            Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    };
    let mut commit = repository.find_commit(tip)?;
    let found = loop {
        if Some(commit.id()) == position.base_oid || !holds(&commit)? {
            return Ok(None);
        }
        if commit.parent_count() == 0 {
            break commit;
        }
        let parent = commit.parent(0)?;
        if !holds(&parent)? {
            break commit;
        }
        commit = parent;
    };
    if found.message_bytes() != INITIALIZATION_MESSAGE.as_bytes() {
        return Ok(None);
    }
    if let Some(written) = written {
        let entry = found.tree()?.get_path(Path::new(canonical::CONFIG_PATH))?;
        let holds_what_was_written = match repository.find_blob(entry.id()) {
            Ok(blob) => blob.content() == written.as_bytes(),
            // Something other than a file is committed there.
            Err(error) if error.code() == git2::ErrorCode::NotFound => false,
            Err(error) => return Err(error),
        };
        if !holds_what_was_written {
            return Ok(None);
        }
    }
    Ok(Some(found.id()))
}

/// The initialization commit that is the request's own: the commit that
/// put the configuration file on the branch, when the request's operation
/// recorded that it wrote that file. `Ok(None)` when the operation wrote
/// no configuration, or no commit of enablement's holds what it wrote.
/// `Err` when the journal or Git could not be read. `written` is the
/// configuration the request writes, when its input is at hand; see
/// `initialization_commit`.
fn own_commit(
    service: &RepositoryService,
    accepted: &Accepted,
    written: Option<&str>,
) -> Result<Option<git2::Oid>, ()> {
    let wrote = accepted.recorded_local_step(service)?.is_some_and(|step| {
        step.is(CONFIGURATION_WRITTEN_STEP) || step.is(INITIALIZATION_COMMITTED_STEP)
    });
    if !wrote {
        return Ok(None);
    }
    initialization_commit(&accepted.root(), &accepted.position, written).map_err(|_| ())
}

/// Whether the row says the operation has done something and has not
/// ended: a repository initialized, or a configuration written.
fn begun_and_unfinished(journal: Option<&JournalRow>) -> bool {
    match journal {
        Some(JournalRow::Pending(PendingOperation::Local { state: _, step })) => step.is_some(),
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

/// What the repository shows of an unfinished create or enable, without
/// its input: its own initialization commit, when it made one, and
/// whether it has begun and not ended.
pub(crate) fn recorded_effects(
    service: &RepositoryService,
    accepted: &Accepted,
    journal: Option<&JournalRow>,
) -> Recorded {
    recorded(service, accepted, journal, None)
}

/// `recorded_effects`, with the configuration the request writes when
/// its input is at hand.
fn recorded(
    service: &RepositoryService,
    accepted: &Accepted,
    journal: Option<&JournalRow>,
    written: Option<&str>,
) -> Recorded {
    Recorded {
        effects: outcome::stopped_save_effects(
            own_commit(service, accepted, written).ok().flatten(),
            false,
        ),
        local_change: begun_and_unfinished(journal),
    }
}

/// The result of an enablement that needs nothing more: its commit, or a
/// no-op when the request made none.
fn enabled(commit: Option<git2::Oid>) -> Answer {
    let (code, effects) = SaveReport::Saved {
        claimed: commit,
        discovered: true,
    }
    .result(commit);
    Answer {
        effects,
        ..Answer::stopped(code)
    }
}

impl RepoBinding {
    fn operation(&self) -> RepositoryOperation {
        match self.verb {
            Verb::Create => RepositoryOperation::CreateAndEnable,
            Verb::Enable => RepositoryOperation::Enable,
        }
    }

    /// The configuration file the request writes. `None` for an input
    /// that could not be written, which no request is accepted with.
    fn written(&self) -> Option<String> {
        canonical_configuration(
            &self.primary_branch,
            self.slug_prefix.as_deref(),
            &self.root,
            self.operation(),
        )
        .ok()
    }

    /// The checks of the input alone.
    fn validate(&self) -> Result<(), Answer> {
        let invalid = |_| Answer::stopped(ResultCode::InvalidInput);
        // The branch and the prefix are valid exactly when the
        // configuration that holds them can be written.
        canonical_configuration(
            &self.primary_branch,
            self.slug_prefix.as_deref(),
            &self.root,
            self.operation(),
        )
        .map_err(invalid)?;
        if let Some(identity) = &self.identity {
            validate_identity(identity, self.operation()).map_err(invalid)?;
        }
        Ok(())
    }

    /// Reads what a preview covers. Nothing is written and no lease is
    /// taken. An `Err` is a root that is not one the command can work at.
    fn observe(&self) -> Result<Observed, Answer> {
        match self.verb {
            Verb::Create => self.observe_target(),
            Verb::Enable => self.observe_repository(),
        }
    }

    fn observe_target(&self) -> Result<Observed, Answer> {
        let invalid = || Answer::stopped(ResultCode::InvalidPath);
        let name = self.root.file_name().ok_or_else(invalid)?;
        let parent = self
            .root
            .parent()
            .and_then(|parent| std::fs::canonicalize(parent).ok())
            .filter(|parent| parent.is_dir())
            .ok_or_else(invalid)?;
        let root = parent.join(name);
        let unreadable = |_| Answer::stopped(ResultCode::RepositoryInaccessible);
        let target = match std::fs::symlink_metadata(&root) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Target::Absent,
            Err(error) => return Err(unreadable(error)),
            Ok(metadata) if !metadata.is_dir() => Target::Occupied,
            Ok(_) => {
                let mut entries = std::fs::read_dir(&root).map_err(unreadable)?;
                match entries.next().transpose().map_err(unreadable)? {
                    Some(_) => Target::Occupied,
                    None => Target::EmptyDirectory,
                }
            }
        };
        let effective = match &self.identity {
            Some(_) => None,
            // There is no repository yet: what Git finds outside one.
            None => {
                let config = git2::Config::open_default().map_err(inaccessible)?;
                self.effective_identity(&config, &root)?
            }
        };
        Ok(Observed {
            root,
            state: State::Create { target },
            effective,
        })
    }

    fn observe_repository(&self) -> Result<Observed, Answer> {
        let operation = RepositoryOperation::Enable;
        let (repository, root) =
            canonical_repository_root(&self.root, operation).map_err(refused)?;
        let head = match repository.head() {
            Ok(head) => match checked_out_branch(&repository, &root, operation) {
                Ok(name) => Head::Branch {
                    name,
                    commit: head.peel_to_commit().map_err(inaccessible)?.id(),
                },
                Err(error) if error.kind == RepositoryErrorKind::DetachedHead => Head::Detached,
                Err(error) => return Err(refused(error)),
            },
            Err(error) if error.code() == git2::ErrorCode::UnbornBranch => {
                let primary = format!("refs/heads/{}", self.primary_branch);
                let primary_exists = match repository.find_reference(&primary) {
                    Ok(_) => true,
                    Err(error) if error.code() == git2::ErrorCode::NotFound => false,
                    Err(error) => return Err(inaccessible(error)),
                };
                // A HEAD that names no local branch is detached to
                // enablement, born or not.
                match checked_out_branch(&repository, &root, operation) {
                    Ok(name) => Head::Unborn {
                        name,
                        primary_exists,
                    },
                    Err(error) if error.kind == RepositoryErrorKind::DetachedHead => Head::Detached,
                    Err(error) => return Err(refused(error)),
                }
            }
            Err(error) => return Err(inaccessible(error)),
        };
        let worktree = worktree_state(&repository, &root).map_err(refused)?;
        let configuration = read_configuration_for(&root, operation).map_err(refused)?;
        let effective = match &self.identity {
            Some(_) => None,
            None => {
                let config = repository.config().map_err(inaccessible)?;
                self.effective_identity(&config, &root)?
            }
        };
        Ok(Observed {
            root,
            state: State::Enable {
                head,
                worktree,
                configuration,
            },
            effective,
        })
    }

    /// The identity `config` gives, the repository's own first, and where
    /// it comes from.
    fn effective_identity(
        &self,
        config: &git2::Config,
        root: &Path,
    ) -> Result<Option<(CommitIdentity, PreviewIdentitySource)>, Answer> {
        Ok(resolve_identity(config, config, root, self.operation())
            .map_err(refused)?
            .and_then(|(identity, level)| identity_source(level).map(|source| (identity, source))))
    }

    /// The digest of what was observed: what a confirmation stores, and
    /// what a later observation is compared with.
    fn digest(&self, observed: &Observed) -> ObservationDigest {
        ObservationDigest {
            state: self.state_digest(&observed.state),
            unchanging: self.identity_digest(observed.effective.as_ref()),
        }
    }

    /// The digest of the state the command changes.
    fn state_digest(&self, state: &State) -> String {
        let observation = Observation::of(self.command().name());
        let observation = match state {
            State::Create { target } => observation.text(
                "target",
                match target {
                    Target::Absent => "absent",
                    Target::EmptyDirectory => "empty_directory",
                    Target::Occupied => "occupied",
                },
            ),
            State::Enable {
                head,
                worktree,
                configuration,
            } => {
                let observation = match head {
                    Head::Unborn {
                        name,
                        primary_exists,
                    } => observation
                        .text("head", "unborn")
                        .text("branch", name)
                        .absent("commit")
                        .text("primary", if *primary_exists { "exists" } else { "absent" }),
                    Head::Branch { name, commit } => observation
                        .text("head", "branch")
                        .text("branch", name)
                        .text("commit", &commit.to_string())
                        .absent("primary"),
                    Head::Detached => observation
                        .text("head", "detached")
                        .absent("branch")
                        .absent("commit")
                        .absent("primary"),
                };
                observation
                    .text(
                        "worktree",
                        match worktree {
                            WorktreeState::Clean => "clean",
                            WorktreeState::Dirty => "dirty",
                            WorktreeState::Conflicted => "conflicted",
                        },
                    )
                    .text(
                        "configuration",
                        match configuration {
                            ConfigurationInspection::Missing => "missing",
                            ConfigurationInspection::Valid(_) => "valid",
                            ConfigurationInspection::Invalid(_) => "invalid",
                        },
                    )
            }
        };
        observation.finish()
    }

    /// The digest of the identity the request will commit as, which no
    /// call of the request changes outside the repository it is made in.
    /// It is covered only when the request supplies none: one that is
    /// supplied is in the request's own digest.
    fn identity_digest(
        &self,
        effective: Option<&(CommitIdentity, PreviewIdentitySource)>,
    ) -> String {
        let observation = Observation::of(self.command().name());
        match (&self.identity, effective) {
            (Some(_), _) => Observation::no_identity(self.command().name()),
            (None, None) => observation.text("identity", "none").finish(),
            (None, Some((identity, source))) => observation
                .text("identity", source.as_str())
                .text("name", &identity.name)
                .text("email", &identity.email)
                .finish(),
        }
    }

    /// Whether the configuration `inspection` found is the one the
    /// request asks for: the repository is enabled as requested.
    fn is_as_requested(&self, configuration: &ConfigurationInspection) -> bool {
        match configuration {
            ConfigurationInspection::Valid(config) => {
                config.primary_branch == self.primary_branch
                    && self.slug_prefix.as_ref().is_none_or(|requested| {
                        canonical::slug_settings(config)
                            .ok()
                            .and_then(|(stored, _)| stored)
                            .as_ref()
                            == Some(requested)
                    })
            }
            ConfigurationInspection::Missing | ConfigurationInspection::Invalid(_) => false,
        }
    }

    /// Whether the command's end state holds at `root`: a repository
    /// whose configuration is the one the request asks for.
    fn end_state_holds(&self, root: &Path) -> bool {
        is_repository_root(root)
            && read_configuration_for(root, self.operation())
                .is_ok_and(|configuration| self.is_as_requested(&configuration))
    }

    /// The checks the domain operation would refuse the request for, in
    /// the order it makes them, on what was observed. An `Err` is a
    /// rejection, or a no-op when the repository is enabled as requested.
    fn check(&self, observed: &Observed) -> Result<(), Answer> {
        let stopped = |code| Err(Answer::stopped(code));
        match &observed.state {
            State::Create { target } => match target {
                Target::Absent | Target::EmptyDirectory => {}
                Target::Occupied => return stopped(ResultCode::OccupiedPath),
            },
            State::Enable {
                head,
                worktree,
                configuration,
            } => {
                let on_primary = match head {
                    Head::Unborn {
                        name: _,
                        primary_exists,
                    } => !primary_exists,
                    Head::Branch { name, commit: _ } => *name == self.primary_branch,
                    Head::Detached => false,
                };
                if !on_primary {
                    return stopped(ResultCode::WrongBranch);
                }
                match worktree {
                    WorktreeState::Clean => {}
                    WorktreeState::Dirty => return stopped(ResultCode::WorktreeNotClean),
                    WorktreeState::Conflicted => return stopped(ResultCode::WorktreeConflicted),
                }
                match configuration {
                    ConfigurationInspection::Missing => {}
                    ConfigurationInspection::Invalid(_) => {
                        return stopped(ResultCode::InvalidConfiguration);
                    }
                    ConfigurationInspection::Valid(config) => {
                        if config.primary_branch != self.primary_branch {
                            return stopped(ResultCode::InvalidConfiguration);
                        }
                        // Nothing would write a prefix to a repository
                        // that is enabled already.
                        if !self.is_as_requested(configuration) {
                            return stopped(ResultCode::InvalidInput);
                        }
                        return Err(enabled(None));
                    }
                }
            }
        }
        if self.identity.is_none() && observed.effective.is_none() {
            return stopped(ResultCode::IdentityRequired);
        }
        Ok(())
    }

    /// The identity Git would commit as at `root`, and where it comes
    /// from, read as `observe` reads it and without the rest of the
    /// observation: for a request that supplies none. For a create that
    /// is what Git finds outside any repository, also once the
    /// repository is there, since that is what its preview showed.
    fn effective_identity_at(
        &self,
        root: &Path,
    ) -> Result<Option<(CommitIdentity, PreviewIdentitySource)>, Answer> {
        if self.identity.is_some() {
            return Ok(None);
        }
        let config = match self.verb {
            Verb::Create => git2::Config::open_default(),
            Verb::Enable => Repository::open(root).and_then(|repository| repository.config()),
        }
        .map_err(inaccessible)?;
        self.effective_identity(&config, root)
    }

    /// The identity the domain call is given, from what was observed.
    fn acting_identity(&self, observed: &Observed) -> Option<CommitIdentity> {
        self.acting_with(observed.effective.as_ref())
    }

    /// The identity the domain call is given, when `effective` is the
    /// identity Git would commit as.
    fn acting_with(
        &self,
        effective: Option<&(CommitIdentity, PreviewIdentitySource)>,
    ) -> Option<CommitIdentity> {
        match (&self.identity, self.verb) {
            (Some(identity), _) => Some(identity.clone()),
            // Enablement finds the identity itself and writes nothing.
            (None, Verb::Enable) => None,
            // Creation reads no configuration outside the repository it
            // makes, so it is given what was found there, and writes it
            // to the new repository's own configuration.
            (None, Verb::Create) => effective.map(|(identity, _)| identity.clone()),
        }
    }

    /// Where the request commits: the primary branch, and its commit when
    /// it has one.
    fn position(&self, observed: &Observed) -> Position {
        Position {
            base_ref: Some(format!("refs/heads/{}", self.primary_branch)),
            base_oid: match &observed.state {
                State::Enable {
                    head: Head::Branch { name: _, commit },
                    worktree: _,
                    configuration: _,
                } => Some(*commit),
                State::Enable {
                    head:
                        Head::Unborn {
                            name: _,
                            primary_exists: _,
                        }
                        | Head::Detached,
                    worktree: _,
                    configuration: _,
                }
                | State::Create { target: _ } => None,
            },
        }
    }

    /// The one domain call, under the recorded operation ID. A create is
    /// made through `enable` once its repository exists: only the first
    /// part of a create is its own, and `enable` takes up a row a create
    /// began.
    fn call(&self, service: &RepositoryService, accepted: &Accepted) -> Ran {
        let root = accepted.root();
        let through_enable = match self.verb {
            Verb::Enable => true,
            Verb::Create => is_repository_root(&root),
        };
        let prefix = self.slug_prefix.as_deref();
        let result = if through_enable {
            service.enable_with_slug_prefix(
                EnableRepositoryRequest {
                    root: root.clone(),
                    primary_branch: self.primary_branch.clone(),
                    identity: self.acting.clone(),
                    operation_id: accepted.operation_id,
                },
                prefix,
            )
        } else {
            service.create_and_enable_with_slug_prefix(
                CreateRepositoryRequest {
                    root: root.clone(),
                    primary_branch: self.primary_branch.clone(),
                    identity: self.acting.clone(),
                    operation_id: accepted.operation_id,
                },
                prefix,
            )
        };
        // The commit the domain names is taken only from a call that
        // made it. Every other result names the head it found, or none.
        let own = |named: Option<git2::Oid>| match named {
            Some(commit) => Ok(Some(commit)),
            None => own_commit(service, accepted, self.written().as_deref()),
        };
        let unconfirmed = || Ran {
            answer: Answer::stopped(ResultCode::RepositoryInaccessible),
            standing: Standing::Unconfirmed,
        };
        let report = match result {
            Ok(EnableRepositoryOutcome::Enabled { commit_oid }) => {
                return Ran {
                    answer: enabled(Some(commit_oid)),
                    standing: Standing::Final,
                };
            }
            Ok(EnableRepositoryOutcome::AlreadyEnabled) => {
                return match own(None) {
                    Ok(commit) => Ran {
                        answer: enabled(commit),
                        standing: Standing::Final,
                    },
                    Err(()) => unconfirmed(),
                };
            }
            Ok(EnableRepositoryOutcome::IdentityRequired) => {
                return Ran {
                    answer: Answer::stopped(ResultCode::IdentityRequired),
                    standing: Standing::Stopped {
                        journal: accepted.journal_row(service),
                    },
                };
            }
            Ok(EnableRepositoryOutcome::IndexPending(_)) => SaveReport::Saved {
                claimed: None,
                discovered: false,
            },
            Ok(EnableRepositoryOutcome::RegistrationPending { commit_oid: _ }) => {
                SaveReport::RegistrationPending { claimed: None }
            }
            Err(error) => return self.stopped(service, accepted, &root, error.kind),
        };
        // The commit is made and the hand-off or the registration is
        // owed: the same request completes it.
        match own(None) {
            Ok(commit) => {
                let (code, effects) = report.result(commit);
                Ran {
                    answer: Answer {
                        effects,
                        ..Answer::stopped(code)
                    },
                    standing: Standing::Owed,
                }
            }
            Err(()) => unconfirmed(),
        }
    }

    /// A domain call that returned an error: its code, and what the
    /// repository shows the request has done.
    fn stopped(
        &self,
        service: &RepositoryService,
        accepted: &Accepted,
        root: &Path,
        kind: RepositoryErrorKind,
    ) -> Ran {
        let occupied = self.verb == Verb::Create
            && kind == RepositoryErrorKind::InvalidPath
            && std::fs::symlink_metadata(root).is_ok()
            && !is_repository_root(root);
        let code = if occupied {
            // The target is a file, or holds something that is not the
            // request's own repository.
            ResultCode::OccupiedPath
        } else {
            outcome::repository_error_code(kind)
        };
        let journal = accepted.journal_row(service);
        let recorded = recorded(
            service,
            accepted,
            journal.as_ref(),
            self.written().as_deref(),
        );
        Ran {
            answer: Answer {
                effects: recorded.effects,
                local_change: recorded.local_change,
                ..Answer::stopped(code)
            },
            standing: Standing::Stopped { journal },
        }
    }

    /// A row that has completed: what the request is answered with, and
    /// what becomes of its record. No domain call is made.
    fn ended(
        &self,
        service: &RepositoryService,
        accepted: &Accepted,
        journal: JournalRow,
        previewed: Option<&ObservationDigest>,
    ) -> Ran {
        match own_commit(service, accepted, self.written().as_deref()) {
            Err(()) => Ran {
                answer: Answer::stopped(ResultCode::RepositoryInaccessible),
                standing: Standing::Unconfirmed,
            },
            // The operation wrote the configuration and its commit is on
            // the branch: the request's result, which was lost.
            Ok(Some(commit)) => Ran {
                answer: enabled(Some(commit)),
                standing: Standing::Final,
            },
            // The operation found the repository enabled as requested,
            // or its own commit is no longer on the branch and the
            // repository is enabled all the same: nothing is the
            // request's to report, and nothing is left to do.
            Ok(None) if self.end_state_holds(&accepted.root()) => Ran {
                answer: enabled(None),
                standing: Standing::Final,
            },
            // The operation ended having done nothing: for want of an
            // identity, say. Had that answer not been lost, settling
            // would have deleted the record; it is deleted now, which
            // releases the confirmation. The answer says what is true
            // as things are:
            //
            // - what stands in the request's way, when something does;
            // - that what was previewed is not what is there, when it is
            //   not;
            // - otherwise that the request did not run and can be sent
            //   again. Nothing has changed and nothing is in its way, so
            //   to call it a change would be untrue; `busy` is the code
            //   whose one meaning to a caller is "send this again".
            Ok(None) => Ran {
                answer: lost_refusal(
                    self.observe().and_then(|observed| {
                        self.check(&observed)?;
                        Ok(self.digest(&observed))
                    }),
                    previewed,
                ),
                standing: Standing::Stopped {
                    journal: Some(journal),
                },
            },
        }
    }
}

impl Binding for RepoBinding {
    fn command(&self) -> Command {
        match self.verb {
            Verb::Create => Command::RepoCreate,
            Verb::Enable => Command::RepoEnable,
        }
    }

    /// A completed row of an enablement with no checkpoint step does not
    /// show that nothing was committed. Enablement records
    /// `configuration_written` before its commit and
    /// `initialization_committed` after it; a call that is cut off
    /// between the two leaves a pending row with the first step and the
    /// commit made, and the call that takes the row up finds the
    /// repository enabled and completes the row with that step still on
    /// it, which is no checkpoint step. Releasing the record for such a
    /// row would report an initialization that committed as one that did
    /// not, and would send a command that completed back through a
    /// confirmation it has used.
    ///
    /// So the record is kept and `reenter` answers from evidence of its
    /// own: the step the row holds says whether the operation wrote the
    /// configuration, and Git gives the commit that put it on the branch.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }

    fn place(&self) -> Place<'_> {
        match self.verb {
            Verb::Create => Place::NewRepository(&self.root),
            Verb::Enable => Place::Repository(&self.root),
        }
    }

    /// The repository as a whole: the command names nothing within it.
    fn target(&self) -> String {
        String::new()
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        let Self {
            verb: _,
            root: _,
            primary_branch,
            identity,
            slug_prefix,
            // Worked out from the input and what is observed.
            acting: _,
        } = self;
        fn optional(value: Option<&String>) -> FieldValue<'_> {
            match value {
                Some(value) => FieldValue::Text(value),
                None => FieldValue::Absent,
            }
        }
        builder
            .field("primary_branch", FieldValue::Text(primary_branch))
            .field(
                "name",
                optional(identity.as_ref().map(|identity| &identity.name)),
            )
            .field(
                "email",
                optional(identity.as_ref().map(|identity| &identity.email)),
            )
            .field("slug_prefix", optional(slug_prefix.as_ref()))
    }

    fn scope(&self, repository: Option<String>, position: Option<&Position>) -> Scope {
        Scope {
            branch: position.and_then(|position| position.branch().map(str::to_owned)),
            repository,
            ..Scope::default()
        }
    }

    /// Validates the input and observes. No check is made of what the
    /// preview covers: the boundary compares the digest with the
    /// confirmation's, and refuses a request whose preview is stale as
    /// `external_change`, whatever it is that has changed.
    fn prepare(&mut self, _call: &CallContext<'_>) -> Result<Prepared, Answer> {
        self.validate()?;
        let observed = self.observe()?;
        // A root that cannot be written as text has no scope key, and no
        // record could be of a request on it.
        if scope_key(self.place()).is_none() {
            return Err(Answer::stopped(ResultCode::InvalidPath));
        }
        self.acting = self.acting_identity(&observed);
        Ok(Prepared {
            family: Some(OperationFamily::Local),
            position: self.position(&observed),
            expected: None,
            observation: Some(self.digest(&observed).to_stored()),
        })
    }

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        self.call(call.service, accepted)
    }

    /// In order:
    ///
    /// 1. A row that has completed is answered from what its operation
    ///    recorded, with no domain call; see `ended`.
    /// 2. A row that holds a step is taken up by the domain call. The
    ///    request's own work has changed the state that would be
    ///    observed, so that is not compared, and the confirmation is the
    ///    request's however old it is. The identity the preview showed
    ///    is compared all the same, until the initialization commit is
    ///    recorded: the request's work never changes where it comes
    ///    from, and a request that went on with another would commit
    ///    as, and write, an identity nobody was shown. A difference is
    ///    `external_change`, and nothing more is written. Once the
    ///    commit is recorded nothing left to do reads the identity, and
    ///    it is not compared.
    /// 3. With no step recorded, nothing of the request's is known to be
    ///    there. When no row stands and the end state holds, the request
    ///    is a no-op and is finished. Otherwise what was previewed is
    ///    observed again and compared, state and identity: a difference
    ///    is `external_change`. One difference is the request's own: a
    ///    create whose row is pending made its directory before it
    ///    recorded anything, so a preview of an absent target accepts an
    ///    empty directory under that row.
    fn reenter(
        &mut self,
        call: &CallContext<'_>,
        accepted: &Accepted,
        journal: JournalRow,
    ) -> Result<Entered, Ran> {
        let service = call.service;
        let not_run = |answer: Answer, journal: JournalRow| Ran {
            answer,
            standing: Standing::NotRun { journal },
        };
        // What the request's consent was to. A request whose record
        // holds none, or one that cannot be read as a digest, has none
        // that anything could be compared with: nothing is run on that.
        let previewed = call.previewed();
        let step = match &journal {
            JournalRow::Absent => None,
            JournalRow::Pending(PendingOperation::Local { state: _, step }) => *step,
            JournalRow::Final {
                kind: FinalKind::Completed,
                owes_work: false,
                checkpointed: _,
            } => return Err(self.ended(service, accepted, journal, previewed.as_ref())),
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
        let root = accepted.root();
        let changed =
            |journal: JournalRow| not_run(Answer::stopped(ResultCode::ExternalChange), journal);
        let Some(previewed) = previewed else {
            return Err(changed(journal));
        };
        // Everything that depends on the identity is done once the
        // commit is recorded: enablement writes the identity to the
        // repository's own configuration before it writes the
        // configuration file, and authors the commit before it records
        // this step, for a create as for an enable. From here the
        // request only registers and indexes, and reads no identity, so
        // none is compared: a request told to send itself again is not
        // refused for an identity that has changed since. A row that
        // holds the step before it may or may not have committed, and is
        // still compared.
        if step.is_some_and(|step| step.is(INITIALIZATION_COMMITTED_STEP)) {
            // Not used by what is left to do; given as before when it
            // can be read.
            let effective = self.effective_identity_at(&root).ok().flatten();
            self.acting = self.acting_with(effective.as_ref());
            return Ok(Entered::Unfinished);
        }
        if step.is_some() {
            let effective = match self.effective_identity_at(&root) {
                Ok(effective) => effective,
                Err(answer) => return Err(not_run(answer, journal)),
            };
            if previewed.unchanging != self.identity_digest(effective.as_ref()) {
                return Err(changed(journal));
            }
            self.acting = self.acting_with(effective.as_ref());
            return Ok(Entered::Unfinished);
        }
        let holds = self.end_state_holds(&root);
        if holds && journal == JournalRow::Absent {
            return Err(Ran {
                answer: enabled(None),
                standing: Standing::Final,
            });
        }
        let observed = match self.observe() {
            Ok(observed) => observed,
            Err(answer) => return Err(not_run(answer, journal)),
        };
        // A row that is pending is the domain's to end, also when the
        // repository was enabled as requested under it: nothing is
        // written then, and nothing is compared.
        if !holds {
            let found = self.digest(&observed);
            let own_directory = journal != JournalRow::Absent
                && matches!(
                    observed.state,
                    State::Create {
                        target: Target::EmptyDirectory
                    }
                )
                && previewed.state
                    == self.state_digest(&State::Create {
                        target: Target::Absent,
                    });
            if previewed.unchanging != found.unchanging
                || (previewed.state != found.state && !own_directory)
            {
                return Err(changed(journal));
            }
        }
        self.acting = self.acting_identity(&observed);
        Ok(Entered::Unfinished)
    }

    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, _entered: &Entered) -> Ran {
        self.call(call.service, accepted)
    }
}

impl Confirmed for RepoBinding {
    fn preview(&mut self, _call: &CallContext<'_>) -> Result<Previewed, Answer> {
        self.validate()?;
        let observed = self.observe()?;
        self.check(&observed)?;
        let (identity, source) = match (&self.identity, &observed.effective) {
            (Some(identity), _) => (identity.clone(), PreviewIdentitySource::Supplied),
            (None, Some((identity, source))) => (identity.clone(), *source),
            // `check` refused a request with no identity.
            (None, None) => return Err(Answer::stopped(ResultCode::IdentityRequired)),
        };
        let written_locally = match self.verb {
            Verb::Create => true,
            Verb::Enable => self.identity.is_some(),
        };
        let mut effects = Vec::new();
        if let State::Create { target } = &observed.state {
            if *target == Target::Absent {
                effects.push(PreviewEffect::DirectoryCreated);
            }
            effects.push(PreviewEffect::RepositoryInitialized);
        }
        if written_locally {
            effects.push(PreviewEffect::IdentityWritten);
        }
        // Enablement moves an unborn HEAD onto the primary branch when it
        // names another.
        if let State::Enable {
            head:
                Head::Unborn {
                    name,
                    primary_exists: _,
                },
            worktree: _,
            configuration: _,
        } = &observed.state
            && *name != self.primary_branch
        {
            effects.push(PreviewEffect::HeadMoved);
        }
        effects.extend([
            PreviewEffect::WorktreeExclusionWritten,
            PreviewEffect::ConfigurationWritten,
            PreviewEffect::InitializationCommitted,
            PreviewEffect::RepositoryRegistered,
        ]);
        Ok(Previewed {
            observation: self.digest(&observed).to_stored(),
            effects,
            details: PreviewDetailsDto::Repository(RepositoryPreviewDto {
                root: observed.root.to_string_lossy().into_owned(),
                primary_branch: self.primary_branch.clone(),
                identity: PreviewIdentityDto {
                    name: identity.name,
                    email: identity.email,
                    source,
                    written_locally,
                },
                slug_prefix: self.slug_prefix.clone(),
            }),
        })
    }
}
