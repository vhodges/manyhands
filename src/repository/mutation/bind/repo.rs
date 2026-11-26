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
//! worktree is clean; whether a configuration file is there; and whether
//! this data directory's registry holds the root. A repository enabled
//! as requested that the registry does not hold, enabled through another
//! data directory say, is registered and indexed, and nothing is
//! written to it or committed: the result is `ok` with no commit. For
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

use crate::repository::mutation::confirm::{self, Row, Seen};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use git2::{ConfigLevel, Repository};

use crate::{
    canonical::{self, ForgeSetting, Integration},
    results::{Effects, PublicationEffect, ResultCode, Scope, WriteEffect, redact_url},
};

use super::{
    super::{
        super::{
            CommitIdentity, ConfigurationInspection, CreateRepositoryRequest,
            EnableRepositoryOutcome, EnableRepositoryRequest, INITIALIZATION_MESSAGE,
            OperationFamily, RepositoryErrorKind, RepositoryOperation, RepositoryService,
            WorktreeState, canonical_configuration_with, canonical_repository_root,
            checked_out_branch, publication_remote_eligible, read_configuration_for,
            recovery::{FinalKind, JournalRow, LocalStep, PendingOperation},
            resolve_identity,
            setup::{
                data as setup_data,
                enable::{SetupOutcome, SetupRequest},
                intent::{self, Intended, IntentError},
            },
            validate_identity, worktree_state,
        },
        command::Command,
        confirm::scope_key,
        dto::{
            PreviewDetailsDto, PreviewEffect, PreviewIdentityDto, PreviewIdentitySource,
            RepositoryPreviewDto, SetupForgeDto, SetupPreviewDto,
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
    /// The remote the configuration names as the publication remote, and
    /// that a setup is published to.
    pub publication_remote: Option<String>,
    /// The mode: `direct` commits the configuration to the primary, `pull
    /// request` puts it on a setup branch. With none, the forge's default
    /// for the remote's address, or direct for no remote.
    pub integration: Option<Integration>,
    /// The `[forge]` setting the configuration is to hold.
    pub forge: Option<ForgeSetting>,
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
    publication_remote: Option<String>,
    integration: Option<Integration>,
    forge: Option<ForgeSetting>,
    /// The identity the domain call is given, which it writes to the
    /// repository's own configuration: the one supplied, and for a create
    /// that supplies none, the one found outside any repository. Set
    /// before each domain call.
    acting: Option<CommitIdentity>,
    /// What the domain call does, worked out from the input and what was
    /// observed: set with `acting`, before each call. An enable that has none
    /// is never run as a direct one: only a row that holds a step of its own
    /// says what the request was.
    route: Option<Route>,
}

/// What a request that finds no configuration does.
#[derive(Clone)]
enum Route {
    /// Commits the configuration to the primary, as an enable always has.
    Direct,
    /// Puts the configuration on a setup branch for a pull request.
    Setup(Box<Intended>),
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
            publication_remote: None,
            integration: None,
            forge: None,
            acting: None,
            route: Some(Route::Direct),
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
            publication_remote,
            integration,
            forge,
        } = input;
        Self {
            verb: Verb::Enable,
            root,
            primary_branch,
            identity,
            slug_prefix,
            publication_remote,
            integration,
            forge,
            acting: None,
            route: None,
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
        /// Whether this data directory's registry holds the root.
        registered: bool,
        /// Whether the publication remote a valid configuration names
        /// can be published to from this clone. True when it names none,
        /// or there is no valid configuration.
        remote_eligible: bool,
    },
}

/// What a request found, at `root`.
struct Observed {
    root: PathBuf,
    state: State,
    /// The identity Git would commit as and the level it is configured
    /// at, read only for a request that supplies none.
    effective: Option<(CommitIdentity, PreviewIdentitySource)>,
    /// What the request's remote, mode and forge come to, for a request that
    /// supplies any of them and finds no configuration.
    intent: Option<SetupObservation>,
}

/// What a request's publication remote, mode and forge come to at the
/// repository, read from Git and contacting nothing.
struct SetupObservation {
    fetch_url: Option<String>,
    push_url: Option<String>,
    /// Whether the remote can be published to from this clone: true when the
    /// request names none.
    eligible: bool,
    intended: Result<Intended, IntentError>,
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

/// The result of an enablement that registered a repository it found
/// enabled, and committed nothing; see `outcome::registration_result`.
fn registered(discovered: bool) -> Answer {
    let (code, effects) = outcome::registration_result(discovered);
    Answer {
        effects,
        local_change: true,
        ..Answer::stopped(code)
    }
}

/// Whether the registry of `service`'s data directory holds `root`.
fn is_registered(service: &RepositoryService, root: &Path) -> Result<bool, Answer> {
    service
        .registration(root)
        .map(|registration| registration.is_some())
        .map_err(|error| Answer::of_read(&error))
}

/// Whether the request's operation registered the repository and handed
/// off to the index on finding it enabled already: its row's last step
/// is the hand-off's, and no step of a configuration it wrote comes
/// before it. Only an operation that registered a root this data
/// directory did not hold, or that took up its own row, hands off there;
/// one that wrote the configuration keeps that step as its last. `Err`
/// when the journal could not be read.
///
/// One case is not told apart. A request whose own pending row has no
/// step recorded, because it stopped before it registered, takes that
/// row up on its retry and hands off. If another actor registered the
/// root in this data directory meanwhile, the request answers `ok` with
/// a local change where `already_applied` would be right. Nothing in the
/// journal separates the two. The answer is harmless: the registration
/// is made once whichever answer is given.
fn registered_by_request(service: &RepositoryService, accepted: &Accepted) -> Result<bool, ()> {
    Ok(accepted
        .recorded_local_step(service)?
        .is_some_and(|step| step.is(LocalStep::AuthoritativeObserved)))
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
        canonical_configuration_with(
            &self.primary_branch,
            self.slug_prefix.as_deref(),
            self.publication_remote.as_deref(),
            self.forge.as_ref(),
            &self.root,
            self.operation(),
        )
        .ok()
    }

    /// Whether the request supplies a publication remote, a mode or a forge.
    /// None of them is for a create, and a request that supplies none has
    /// the digests and the answers it always had.
    fn supplies_setting(&self) -> bool {
        self.publication_remote.is_some() || self.integration.is_some() || self.forge.is_some()
    }

    /// What the supplied remote, mode and forge come to at `repository`:
    /// the remote's addresses, whether it can be published to, and the
    /// intent. Nothing is contacted.
    fn observe_intent(&self, repository: &Repository, root: &Path) -> SetupObservation {
        let name = self.publication_remote.as_deref();
        let (fetch_url, push_url) = match name.map(|name| repository.find_remote(name)) {
            Some(Ok(remote)) => (
                remote.url().map(str::to_owned),
                remote.pushurl().or(remote.url()).map(str::to_owned),
            ),
            Some(Err(_)) | None => (None, None),
        };
        let eligible = name.is_none_or(|name| {
            publication_remote_eligible(repository, root, name).unwrap_or(false)
        });
        let intended = intent::for_enable(
            &self.primary_branch,
            name,
            self.integration,
            self.forge.as_ref(),
            fetch_url.as_deref(),
        );
        SetupObservation {
            fetch_url,
            push_url,
            eligible,
            intended,
        }
    }

    /// The route the observation gives: a pull-request setup when the
    /// repository has no configuration and the intent is pull request.
    fn route_of(&self, observed: &Observed) -> Route {
        match (&observed.state, &observed.intent) {
            (
                State::Enable {
                    configuration: ConfigurationInspection::Missing,
                    head: _,
                    worktree: _,
                    registered: _,
                    remote_eligible: _,
                },
                Some(SetupObservation {
                    intended: Ok(intended),
                    eligible: true,
                    fetch_url: _,
                    push_url: _,
                }),
            ) if intended.integration == Integration::PullRequest => {
                Route::Setup(Box::new(intended.clone()))
            }
            _ => Route::Direct,
        }
    }

    /// The checks of the input alone.
    fn validate(&self) -> Result<(), Answer> {
        let invalid = |_| Answer::stopped(ResultCode::InvalidInput);
        // The branch and the prefix are valid exactly when the
        // configuration that holds them can be written.
        canonical_configuration_with(
            &self.primary_branch,
            self.slug_prefix.as_deref(),
            self.publication_remote.as_deref(),
            self.forge.as_ref(),
            &self.root,
            self.operation(),
        )
        .map_err(invalid)?;
        // A pull-request mode that is named needs a remote, whatever the
        // repository holds.
        if self.integration == Some(Integration::PullRequest) && self.publication_remote.is_none() {
            return Err(Answer::stopped(ResultCode::InvalidInput));
        }
        if let Some(identity) = &self.identity {
            validate_identity(identity, self.operation()).map_err(invalid)?;
        }
        Ok(())
    }

    /// Reads what a preview covers. Nothing is written and no lease is
    /// taken. An `Err` is a root that is not one the command can work at.
    fn observe(&self, service: &RepositoryService) -> Result<Observed, Answer> {
        match self.verb {
            Verb::Create => self.observe_target(),
            Verb::Enable => self.observe_repository(service),
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
            intent: None,
        })
    }

    fn observe_repository(&self, service: &RepositoryService) -> Result<Observed, Answer> {
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
        let registered = is_registered(service, &root)?;
        let remote_eligible = match &configuration {
            ConfigurationInspection::Valid(config) => match &config.publication_remote {
                Some(remote) => {
                    publication_remote_eligible(&repository, &root, remote).map_err(refused)?
                }
                None => true,
            },
            ConfigurationInspection::Missing | ConfigurationInspection::Invalid(_) => true,
        };
        let effective = match &self.identity {
            Some(_) => None,
            None => {
                let config = repository.config().map_err(inaccessible)?;
                self.effective_identity(&config, &root)?
            }
        };
        let intent = (self.supplies_setting()
            && matches!(configuration, ConfigurationInspection::Missing))
        .then(|| self.observe_intent(&repository, &root));
        Ok(Observed {
            root,
            state: State::Enable {
                head,
                worktree,
                configuration,
                registered,
                remote_eligible,
            },
            effective,
            intent,
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
            state: self.state_digest(&observed.state, observed.intent.as_ref()),
            unchanging: self.identity_digest(observed.effective.as_ref()),
        }
    }

    /// The digest of the state the command changes.
    fn state_digest(&self, state: &State, intent: Option<&SetupObservation>) -> String {
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
                registered,
                remote_eligible,
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
                    // A registration made by another between the preview
                    // and the request changes what the request would do.
                    .text("registered", if *registered { "yes" } else { "no" })
                    .text(
                        "publication_remote",
                        if *remote_eligible {
                            "eligible"
                        } else {
                            "ineligible"
                        },
                    )
            }
        };
        // What a request that supplies a remote, a mode or a forge adds to
        // what it covers: the mode they come to, the forge, and the remote's
        // fetch and push addresses, so that a change of any of them between
        // the preview and the request is `external_change`. A request that
        // supplies none covers what it always did.
        let observation = match intent {
            None => observation,
            Some(intent) => {
                let observation = observation
                    .text(
                        "mode",
                        match &intent.intended {
                            Ok(intended) => intended.integration.as_str(),
                            Err(error) => match error {
                                IntentError::InvalidInput => "invalid_input",
                                IntentError::InvalidRemote => "invalid_remote",
                            },
                        },
                    )
                    .text(
                        "remote_name",
                        self.publication_remote.as_deref().unwrap_or(""),
                    )
                    .text("remote_usable", if intent.eligible { "yes" } else { "no" });
                let forge = self.forge.as_ref();
                let observation = observation
                    .text(
                        "forge_kind",
                        forge.and_then(|f| f.kind).map_or("", |k| k.as_str()),
                    )
                    .text(
                        "forge_web_url",
                        forge.and_then(|f| f.web_url.as_deref()).unwrap_or(""),
                    )
                    .text(
                        "forge_pull_request_url",
                        forge
                            .and_then(|f| f.pull_request_url.as_deref())
                            .unwrap_or(""),
                    );
                match &intent.fetch_url {
                    Some(url) => observation.text("fetch_url", url),
                    None => observation.absent("fetch_url"),
                }
                .text(
                    "push_url_present",
                    if intent.push_url.is_some() {
                        "yes"
                    } else {
                        "no"
                    },
                )
                .text("push_url", intent.push_url.as_deref().unwrap_or(""))
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
                    // A supplied remote, mode or forge that is not the
                    // configuration's would be lost: the shared
                    // configuration decides (design R-10).
                    && self
                        .publication_remote
                        .as_deref()
                        .is_none_or(|remote| config.publication_remote.as_deref() == Some(remote))
                    && self
                        .integration
                        .is_none_or(|integration| config.integration() == integration)
                    && self
                        .forge
                        .as_ref()
                        .is_none_or(|forge| config.forge().as_ref() == Some(forge))
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
    /// whose configuration is the one the request asks for, which this
    /// data directory's registry holds. A registry that cannot be read
    /// is not known to hold it.
    ///
    /// The registration is required for a create as for an enable. So a
    /// create whose repository is there but not registered here does not
    /// hold its end state in two places: `observe_again` with no journal
    /// row compares what it observes instead of finishing as a no-op, and
    /// `ended`, for a lost answer with no commit of the request's, does
    /// not answer `already_applied`.
    fn end_state_holds(&self, service: &RepositoryService, root: &Path) -> bool {
        is_repository_root(root)
            && read_configuration_for(root, self.operation())
                .is_ok_and(|configuration| self.is_as_requested(&configuration))
            && is_registered(service, root).unwrap_or(false)
    }

    /// Whether what was observed is a repository enabled as requested
    /// that this data directory does not hold: the request then only
    /// registers it and indexes it, and commits nothing.
    fn registers_only(&self, state: &State) -> bool {
        match state {
            State::Enable {
                head: _,
                worktree: _,
                configuration,
                registered,
                remote_eligible: _,
            } => !registered && self.is_as_requested(configuration),
            State::Create { target: _ } => false,
        }
    }

    /// The checks the domain operation would refuse the request for, in
    /// the order it makes them, on what was observed. An `Err` is a
    /// rejection, or a no-op when the repository is enabled as requested
    /// and registered here.
    ///
    /// For a valid configuration the order is the domain's: the primary
    /// branch it holds, then the prefix asked for, then the publication
    /// remote it names, and only then whether the root is registered.
    ///
    /// A repository enabled as requested that is not registered here is
    /// registered by the request, which commits nothing: the domain asks
    /// for no identity then, and neither does this.
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
                registered,
                remote_eligible,
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
                // A pull request is made on a commit of the remote's primary:
                // an unborn HEAD has no branch to activate (R-11).
                if matches!(
                    head,
                    Head::Unborn {
                        name: _,
                        primary_exists: _
                    }
                ) && matches!(self.route_of(observed), Route::Setup(_))
                {
                    return stopped(ResultCode::WrongBranch);
                }
                match worktree {
                    WorktreeState::Clean => {}
                    WorktreeState::Dirty => return stopped(ResultCode::WorktreeNotClean),
                    WorktreeState::Conflicted => return stopped(ResultCode::WorktreeConflicted),
                }
                match configuration {
                    ConfigurationInspection::Missing => {
                        // The remote is one this clone can publish to, and
                        // the intent is one that can be written.
                        if let Some(intent) = &observed.intent {
                            if !intent.eligible {
                                return stopped(ResultCode::InvalidRemote);
                            }
                            if let Err(error) = &intent.intended {
                                return stopped(error.code());
                            }
                        }
                    }
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
                        // The domain refuses a publication remote this
                        // clone cannot publish to before anything else it
                        // does for an enabled repository, registered here
                        // or not.
                        if !remote_eligible {
                            return stopped(ResultCode::InvalidRemote);
                        }
                        if *registered {
                            return Err(enabled(None));
                        }
                        return Ok(());
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
                    registered: _,
                    remote_eligible: _,
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
                    registered: _,
                    remote_eligible: _,
                }
                | State::Create { target: _ } => None,
            },
        }
    }

    /// The one domain call, under the recorded operation ID. A create is
    /// made through `enable` once its repository exists: only the first
    /// part of a create is its own, and `enable` takes up a row a create
    /// began.
    fn call(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        let service = call.service;
        let root = accepted.root();
        if self.verb == Verb::Enable {
            match &self.route {
                Some(Route::Setup(intended)) => {
                    return self.call_setup(call, accepted, Some(intended), false);
                }
                Some(Route::Direct) => {}
                // No observation gave a route. A row that holds a step of an
                // enable's own says it is a direct one; the setup's step goes
                // on from its record; with none, nothing says what the request
                // is, and it is never run as a direct enable: it is owed.
                None => match accepted.recorded_local_step(service) {
                    Ok(Some(step)) if step.is(LocalStep::SetupRecorded) => {
                        return self.call_setup(call, accepted, None, true);
                    }
                    Ok(Some(_)) => {}
                    Ok(None) | Err(()) => {
                        return Ran {
                            answer: Answer::stopped(ResultCode::RecoveryRequired),
                            standing: Standing::Owed,
                        };
                    }
                },
            }
        }
        let through_enable = match self.verb {
            Verb::Enable => true,
            Verb::Create => is_repository_root(&root),
        };
        let prefix = self.slug_prefix.as_deref();
        let result = if through_enable {
            service.enable_direct_with(
                EnableRepositoryRequest {
                    root: root.clone(),
                    primary_branch: self.primary_branch.clone(),
                    identity: self.acting.clone(),
                    operation_id: accepted.operation_id,
                },
                prefix,
                self.publication_remote.as_deref(),
                self.forge.as_ref(),
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
                // The request's own commit, when it made one; otherwise
                // the registration, when the request's operation made it;
                // otherwise nothing is the request's.
                let answer = match own(None) {
                    Ok(Some(commit)) => Ok(enabled(Some(commit))),
                    Ok(None) => registered_by_request(service, accepted).map(|registered_here| {
                        if registered_here {
                            registered(true)
                        } else {
                            enabled(None)
                        }
                    }),
                    Err(()) => Err(()),
                };
                return match answer {
                    Ok(answer) => Ran {
                        answer,
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
            Err(error) => {
                let blocker = error.blocker().cloned();
                let mut ran = self.stopped(service, accepted, &root, error.kind);
                ran.answer = ran.answer.naming(service, &root, blocker.as_ref());
                return ran;
            }
        };
        // The commit is made and the hand-off or the registration is
        // owed: the same request completes it. A hand-off owed with no
        // commit of the request's follows the registration of a
        // repository it found enabled, which is the request's own: only
        // its own row, or a root it registered, is handed off there.
        match own(None) {
            Ok(commit) => {
                let answer = match (commit, report) {
                    (
                        None,
                        SaveReport::Saved {
                            claimed: _,
                            discovered: false,
                        },
                    ) => registered(false),
                    _ => {
                        let (code, effects) = report.result(commit);
                        Answer {
                            effects,
                            ..Answer::stopped(code)
                        }
                    }
                };
                Ran {
                    answer,
                    standing: Standing::Owed,
                }
            }
            Err(()) => unconfirmed(),
        }
    }

    /// A pull-request enable: records the setup and publishes it, or, for a
    /// request whose row completed at `setup_recorded`, goes on from its
    /// record (design Part K). `intended` is what a first call records.
    fn call_setup(
        &self,
        call: &CallContext<'_>,
        accepted: &Accepted,
        intended: Option<&Intended>,
        recorded: bool,
    ) -> Ran {
        let service = call.service;
        let root = accepted.root();
        let request = EnableRepositoryRequest {
            root: root.clone(),
            primary_branch: self.primary_branch.clone(),
            identity: self.identity.clone(),
            operation_id: accepted.operation_id,
        };
        let setup = SetupRequest {
            request_id: accepted.request_id,
            prefix: self.slug_prefix.clone(),
            remote_name: self.publication_remote.clone().unwrap_or_default(),
            intended: intended.cloned(),
            recorded,
        };
        let outcome = service.enable_with_setup(request, setup, &|root, request_id| {
            call.session.publish_setup(service, root, request_id)
        });
        match outcome {
            Ok(outcome) => setup_ran(service, accepted, &root, outcome),
            Err(error) => {
                let blocker = error.blocker().cloned();
                // A refusal before the recording is an enable's, settled as
                // one. The row decides, as it stands when the call answers:
                // another call of the request may have recorded the setup
                // meanwhile (`stopped` reads the journal after the domain call
                // returned), and a request that is recorded, or whose row
                // cannot be read, is owed whatever this call was: settled as a
                // completed row is, it would be deleted, and its ID freed for a
                // second setup.
                if !recorded && !recorded_now(service, accepted) {
                    let mut ran = self.stopped(service, accepted, &root, error.kind);
                    if !recorded_now(service, accepted) {
                        ran.answer = ran.answer.naming(service, &root, blocker.as_ref());
                        return ran;
                    }
                }
                let answer = Answer {
                    data: setup_data::of(service, &root, None),
                    // The setup is recorded: a durable change no effect
                    // shows, so the request is partial, never a refusal.
                    local_change: true,
                    ..Answer::stopped(outcome::repository_error_code(error.kind))
                }
                .naming(service, &root, blocker.as_ref());
                Ran {
                    answer,
                    standing: Standing::Owed,
                }
            }
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
            // The operation found the repository enabled as requested
            // and registered it here, and the repository is so: the
            // registration is the request's result, which was lost.
            //
            // Otherwise the operation found the repository enabled as
            // requested and registered, or its own commit is no longer
            // on the branch and the repository is enabled all the same:
            // nothing is the request's to report, and nothing is left to
            // do.
            Ok(None) if self.end_state_holds(service, &accepted.root()) => {
                match registered_by_request(service, accepted) {
                    Ok(registered_here) => Ran {
                        answer: if registered_here {
                            registered(true)
                        } else {
                            enabled(None)
                        },
                        standing: Standing::Final,
                    },
                    Err(()) => Ran {
                        answer: Answer::stopped(ResultCode::RepositoryInaccessible),
                        standing: Standing::Unconfirmed,
                    },
                }
            }
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
                    self.observe(service).and_then(|observed| {
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
            publication_remote,
            integration,
            forge,
            // Worked out from the input and what is observed.
            acting: _,
            route: _,
        } = self;
        fn optional(value: Option<&String>) -> FieldValue<'_> {
            match value {
                Some(value) => FieldValue::Text(value),
                None => FieldValue::Absent,
            }
        }
        let mut builder = builder
            .field("primary_branch", FieldValue::Text(primary_branch))
            .field(
                "name",
                optional(identity.as_ref().map(|identity| &identity.name)),
            )
            .field(
                "email",
                optional(identity.as_ref().map(|identity| &identity.email)),
            )
            .field("slug_prefix", optional(slug_prefix.as_ref()));
        // Each is a field of the request only when it is supplied, so that
        // the digest of a request that supplies none is the one it was.
        if let Some(remote) = publication_remote {
            builder = builder.field("publication_remote", FieldValue::Text(remote));
        }
        if let Some(mode) = integration {
            builder = builder.field("integration", FieldValue::Text(mode.as_str()));
        }
        if let Some(forge) = forge {
            builder = builder
                .field(
                    "forge_kind",
                    forge
                        .kind
                        .map_or(FieldValue::Absent, |kind| FieldValue::Text(kind.as_str())),
                )
                .field("forge_web_url", optional(forge.web_url.as_ref()))
                .field(
                    "forge_pull_request_url",
                    optional(forge.pull_request_url.as_ref()),
                );
        }
        builder
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
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        self.validate()?;
        let observed = self.observe(call.service)?;
        // A root that cannot be written as text has no scope key, and no
        // record could be of a request on it.
        if scope_key(self.place()).is_none() {
            return Err(Answer::stopped(ResultCode::InvalidPath));
        }
        self.acting = self.acting_identity(&observed);
        self.route = Some(self.route_of(&observed));
        Ok(Prepared {
            family: Some(OperationFamily::Local),
            position: self.position(&observed),
            expected: None,
            observation: Some(self.digest(&observed).to_stored()),
        })
    }

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        self.call(call, accepted)
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
        confirm::reenter(self, call, accepted, journal)
    }

    fn as_confirmed(&mut self) -> Option<&mut dyn Confirmed> {
        Some(self)
    }

    /// A request whose row completed at `setup_recorded` goes on from its
    /// record: it writes what a crash may have left unwritten and publishes,
    /// and records nothing again. Any other goes through `call`, by the
    /// route its observation gave.
    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, _entered: &Entered) -> Ran {
        match accepted.recorded_local_step(call.service) {
            Ok(Some(step)) if step.is(LocalStep::SetupRecorded) => {
                self.call_setup(call, accepted, None, true)
            }
            Ok(_) => self.call(call, accepted),
            // Git or the journal could not say what the request has done.
            Err(()) => Ran {
                answer: Answer::stopped(ResultCode::RepositoryInaccessible),
                standing: Standing::Unconfirmed,
            },
        }
    }
}

impl Confirmed for RepoBinding {
    fn row(&mut self, call: &CallContext<'_>, accepted: &Accepted, journal: &JournalRow) -> Row {
        let service = call.service;
        let not_run = |answer: Answer| Ran {
            answer,
            standing: Standing::NotRun {
                journal: journal.clone(),
            },
        };
        // What the request's consent was to. A request whose record
        // holds none, or one that cannot be read as a digest, has none
        // that anything could be compared with: nothing is run on that.
        let previewed = call.previewed();
        let step = match journal {
            JournalRow::Absent => None,
            JournalRow::Pending(PendingOperation::Local { state: _, step }) => *step,
            JournalRow::Final {
                kind: FinalKind::Completed,
                owes_work: false,
                checkpointed: _,
            } => {
                // A request that recorded a setup is entered through its
                // record, which the domain reads: nothing is compared, no
                // identity is, and a record that cannot be read is owed,
                // never the direct path.
                return match accepted.recorded_local_step(service) {
                    Ok(Some(step)) if step.is(LocalStep::SetupRecorded) => Row::Unfinished,
                    Err(()) => Row::answers(Ran {
                        answer: Answer::stopped(ResultCode::RepositoryInaccessible),
                        standing: Standing::Unconfirmed,
                    }),
                    Ok(_) => Row::answers(self.ended(
                        service,
                        accepted,
                        journal.clone(),
                        previewed.as_ref(),
                    )),
                };
            }
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
            } => return Row::Impossible,
        };
        let root = accepted.root();
        let Some(previewed) = previewed else {
            return Row::answers(not_run(Answer::stopped(ResultCode::ExternalChange)));
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
            return Row::Recorded;
        }
        if step.is_some() {
            let effective = match self.effective_identity_at(&root) {
                Ok(effective) => effective,
                Err(answer) => return Row::answers(not_run(answer)),
            };
            if previewed.unchanging != self.identity_digest(effective.as_ref()) {
                return Row::answers(not_run(Answer::stopped(ResultCode::ExternalChange)));
            }
            self.acting = self.acting_with(effective.as_ref());
            return Row::Recorded;
        }
        Row::Unrecorded
    }

    fn observe_again(
        &mut self,
        call: &CallContext<'_>,
        accepted: &Accepted,
        journal: &JournalRow,
    ) -> Result<Seen, Ran> {
        let holds = self.end_state_holds(call.service, &accepted.root());
        if holds && *journal == JournalRow::Absent {
            return Ok(Seen::EndState(enabled(None)));
        }
        let observed = match self.observe(call.service) {
            Ok(observed) => observed,
            Err(answer) => {
                return Err(Ran {
                    answer,
                    standing: Standing::NotRun {
                        journal: journal.clone(),
                    },
                });
            }
        };
        self.acting = self.acting_identity(&observed);
        self.route = Some(self.route_of(&observed));
        // A row that is pending is the domain's to end, also when the
        // repository was enabled as requested under it: nothing is
        // written then, and nothing is compared.
        if holds {
            return Ok(Seen::EndState(enabled(None)));
        }
        let found = self.digest(&observed);
        // One difference is the request's own: a create whose row is
        // pending made its directory before it recorded anything, so a
        // preview of an absent target accepts an empty directory under
        // that row.
        let own_directory = *journal != JournalRow::Absent
            && matches!(
                observed.state,
                State::Create {
                    target: Target::EmptyDirectory
                }
            );
        let own = own_directory.then(|| ObservationDigest {
            state: self.state_digest(
                &State::Create {
                    target: Target::Absent,
                },
                None,
            ),
            unchanging: found.unchanging.clone(),
        });
        Ok(Seen::State { found, own })
    }

    fn preview(&mut self, call: &CallContext<'_>) -> Result<Previewed, Answer> {
        self.validate()?;
        let observed = self.observe(call.service)?;
        self.check(&observed)?;
        let (identity, source) = match (&self.identity, &observed.effective) {
            (Some(identity), _) => (identity.clone(), PreviewIdentitySource::Supplied),
            (None, Some((identity, source))) => (identity.clone(), *source),
            // `check` refused a request with no identity. A request that
            // only registers needs none, but a preview's details always
            // name one: with none to name, it is refused all the same.
            (None, None) => return Err(Answer::stopped(ResultCode::IdentityRequired)),
        };
        // Registration alone: the exclusion rule is written when it is
        // not there, which is outside the worktree, and the root is
        // registered and indexed. No identity, configuration or commit
        // is written.
        if self.registers_only(&observed.state) {
            return Ok(Previewed {
                observation: self.digest(&observed).to_stored(),
                effects: vec![
                    PreviewEffect::WorktreeExclusionWritten,
                    PreviewEffect::RepositoryRegistered,
                ],
                details: PreviewDetailsDto::Repository(RepositoryPreviewDto {
                    root: observed.root.to_string_lossy().into_owned(),
                    primary_branch: self.primary_branch.clone(),
                    identity: PreviewIdentityDto {
                        name: identity.name,
                        email: identity.email,
                        source,
                        written_locally: false,
                    },
                    slug_prefix: self.slug_prefix.clone(),
                }),
            });
        }
        let written_locally = match self.verb {
            Verb::Create => true,
            Verb::Enable => self.identity.is_some(),
        };
        // A pull-request enable: the repository is registered with no
        // configuration, and the setup is recorded and published.
        if let (Route::Setup(intended), Some(intent)) = (self.route_of(&observed), &observed.intent)
        {
            let mut effects = Vec::new();
            if written_locally {
                effects.push(PreviewEffect::IdentityWritten);
            }
            effects.extend([
                PreviewEffect::WorktreeExclusionWritten,
                PreviewEffect::RepositoryRegistered,
                PreviewEffect::SetupRecorded,
                PreviewEffect::SetupBranchCommitted,
                PreviewEffect::SetupBranchPublished,
            ]);
            return Ok(Previewed {
                observation: self.digest(&observed).to_stored(),
                effects,
                details: PreviewDetailsDto::Setup(SetupPreviewDto {
                    root: observed.root.to_string_lossy().into_owned(),
                    primary_branch: self.primary_branch.clone(),
                    identity: PreviewIdentityDto {
                        name: identity.name,
                        email: identity.email,
                        source,
                        written_locally,
                    },
                    slug_prefix: self.slug_prefix.clone(),
                    integration: intended.integration.as_str().to_owned(),
                    remote_name: self.publication_remote.clone().unwrap_or_default(),
                    remote_location: intent.fetch_url.as_deref().map(redact_url),
                    forge: self.forge.as_ref().map(|forge| SetupForgeDto {
                        kind: forge.kind.map(|kind| kind.as_str().to_owned()),
                        web_url: forge.web_url.clone(),
                        pull_request_url: forge.pull_request_url.clone(),
                    }),
                }),
            });
        }
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
            registered: _,
            remote_eligible: _,
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

/// Whether the request's own row is at `setup_recorded`, or cannot be
/// read: the request is then owed.
pub(super) fn recorded_now(service: &RepositoryService, accepted: &Accepted) -> bool {
    match accepted.recorded_local_step(service) {
        Ok(step) => step.is_some_and(|step| step.is(LocalStep::SetupRecorded)),
        Err(()) => true,
    }
}

/// What a pull-request setup that got as far as its recording answers with,
/// and what is left of its request (design K2). It is shared: an enable's and
/// a selection's setups end alike.
pub(super) fn setup_ran(
    service: &RepositoryService,
    accepted: &Accepted,
    root: &Path,
    outcome: SetupOutcome,
) -> Ran {
    match outcome {
        SetupOutcome::Published(published) => {
            let Some(data) = setup_data::of(service, root, Some(&published)) else {
                // Published, and its record cannot be read: the same
                // request answers it once it can. The setup is recorded and
                // pushed, a durable change no effect shows: partial, as the
                // other owed answers after the recording are.
                return Ran {
                    answer: Answer {
                        local_change: true,
                        ..Answer::stopped(ResultCode::RecoveryRequired)
                    },
                    standing: Standing::Owed,
                };
            };
            Ran {
                answer: Answer {
                    // No commit: the setup commit is never on the
                    // primary, so a replay checks nothing against it.
                    effects: Effects {
                        write: WriteEffect::Written,
                        publication: PublicationEffect::Published,
                        ..Effects::not_requested()
                    },
                    data: Some(data),
                    ..Answer::stopped(ResultCode::Ok)
                },
                standing: Standing::Final,
            }
        }
        SetupOutcome::Recorded { stop, authority } => {
            // The host a transport stop is about, for the approval the
            // stop is answered with: the setup's data names none.
            let recovery = match (&authority, root.to_str()) {
                (Some(authority), Some(root)) => {
                    outcome::host_action(stop, root, authority, accepted.request_id)
                }
                _ => Vec::new(),
            };
            Ran {
                answer: Answer {
                    data: setup_data::of(service, root, None),
                    recovery,
                    // The registry rows and the record are durable
                    // changes no effect shows.
                    local_change: true,
                    ..Answer::stopped(stop)
                },
                standing: Standing::Owed,
            }
        }
        // The domain finished the request in the transaction that ended
        // the setup: `settled` answers the stored envelope.
        SetupOutcome::Ended(error) => Ran {
            answer: Answer::stopped(error.code()),
            standing: Standing::Final,
        },
        SetupOutcome::AnotherSetup => Ran {
            answer: Answer::stopped(ResultCode::SetupPending),
            standing: Standing::Final,
        },
        // A change's refusal: only a `remote select` meets it.
        SetupOutcome::InTheWay { blocker } => Ran {
            answer: Answer::stopped(ResultCode::RecoveryRequired).naming(
                service,
                root,
                blocker.as_ref(),
            ),
            standing: Standing::Final,
        },
        SetupOutcome::IdentityRequired => {
            // Nothing was recorded by this call, but another call of the
            // request may have recorded the setup since (R-20): a request
            // recorded, or whose row cannot be read, is owed with its setup,
            // as the `Err` arm of the call has it. Settled as a refusal it
            // would be deleted, leaving a record that names a request that
            // has none.
            if recorded_now(service, accepted) {
                return Ran {
                    answer: Answer {
                        data: setup_data::of(service, root, None),
                        local_change: true,
                        ..Answer::stopped(ResultCode::IdentityRequired)
                    },
                    standing: Standing::Owed,
                };
            }
            Ran {
                answer: Answer::stopped(ResultCode::IdentityRequired),
                standing: Standing::Stopped {
                    journal: accepted.journal_row(service),
                },
            }
        }
    }
}

#[cfg(test)]
#[path = "repo_tests.rs"]
pub(super) mod tests;
