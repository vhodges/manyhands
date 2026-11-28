//! The setup poll: whether a pending setup has merged into its remote's
//! primary, and, when it has, activating the repository (design Part G).
// The transport's typed error is large by design, as in `transport` and `remote`.
#![allow(clippy::result_large_err)]

use std::{path::Path, time::Duration};

use super::{
    channel::{ServiceChannel, SetupChannel},
    complete,
    lock::{self, Begun, Held},
    publish::SetupError,
    record::{self, SetupKind, SetupRecord},
};
use crate::{
    canonical::{self, Integration, ItemId},
    forge::{self, PullRequestLink},
    repository::{
        OperationId, ReadError, RefreshRepositoryRequest, RemotePollInvocation, RepositoryError,
        RepositoryErrorKind, RepositoryOperation, RepositoryService,
        coordination::{repository_lease, repository_lease_waiting},
        integration::{self, artifacts},
        keys::{SessionCredentialProvider, SessionCredentials},
        mutation::{repository_error_code, ssh_transport_error_code},
        promotion::lifecycle::CancellationProbe,
        remote::state,
        transport::{SetupScope, SshTransportError, SshTransportErrorKind},
    },
    results::ResultCode,
};
use rusqlite::OptionalExtension;

/// What a setup poll saw of the pending setup, in the order of design
/// Part G's checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupPollOutcome {
    NotPublished,
    ConfigurationInvalid,
    PrimaryDiffers,
    BranchDeleted,
    WaitingForMerge,
    WaitingForCheckout,
    Diverged,
    Activated,
}

impl SetupPollOutcome {
    pub const ALL: [Self; 8] = [
        Self::NotPublished,
        Self::ConfigurationInvalid,
        Self::PrimaryDiffers,
        Self::BranchDeleted,
        Self::WaitingForMerge,
        Self::WaitingForCheckout,
        Self::Diverged,
        Self::Activated,
    ];

    /// The name the contract publishes.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotPublished => "not_published",
            Self::ConfigurationInvalid => "configuration_invalid",
            Self::PrimaryDiffers => "primary_differs",
            Self::BranchDeleted => "branch_deleted",
            Self::WaitingForMerge => "waiting_for_merge",
            Self::WaitingForCheckout => "waiting_for_checkout",
            Self::Diverged => "diverged",
            Self::Activated => "activated",
        }
    }
}

/// What a setup poll reports: the setup, what it saw, the merged mode on
/// activation (`None` otherwise), whether the merged file is not the one the
/// setup intended, and the link that opens the pull request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupPollReport {
    pub setup_id: ItemId,
    pub outcome: SetupPollOutcome,
    pub integration: Option<Integration>,
    pub differs: bool,
    pub link: PullRequestLink,
}

/// Why a setup poll stopped. Every stop is final: a new request polls again.
#[derive(Debug)]
pub enum SetupPollError {
    /// The poll was asked to stop.
    Cancelled,
    /// Another setup operation, or a writer of the repository, holds a lock.
    Busy,
    /// The setup cannot be read, or its remote is not where it was.
    RecoveryRequired,
    /// The setup ended between the choice of the poll and the lock, so there
    /// is nothing to poll: the caller takes the ordinary path.
    Gone,
    Transport(SshTransportError),
    Repository(RepositoryError),
    Registry(ReadError),
}

impl SetupPollError {
    /// The code the stop answers with.
    pub fn code(&self) -> ResultCode {
        match self {
            Self::Cancelled => ResultCode::Cancelled,
            Self::Busy => ResultCode::Busy,
            Self::RecoveryRequired | Self::Gone => ResultCode::RecoveryRequired,
            // As publishing answers them (ruling R-R): the setup's remote
            // changed or went under a record that is kept.
            Self::Transport(error) => match &error.kind {
                SshTransportErrorKind::EndpointChanged
                | SshTransportErrorKind::PublicationRemoteMissing => ResultCode::RecoveryRequired,
                kind => ssh_transport_error_code(kind),
            },
            Self::Repository(error) => repository_error_code(error.kind),
            Self::Registry(error) => error.code(),
        }
    }
}

impl From<SetupError> for SetupPollError {
    fn from(error: SetupError) -> Self {
        match error {
            SetupError::Busy => Self::Busy,
            SetupError::Transport(error) => Self::Transport(error),
            SetupError::Repository(error) => Self::Repository(error),
            SetupError::Registry(error) => Self::Registry(error),
            // The rest are publishing's own stops: none is reached by a poll.
            _ => Self::RecoveryRequired,
        }
    }
}

impl From<RepositoryError> for SetupPollError {
    /// A held lock, of the repository or of the setup, is `busy`.
    fn from(error: RepositoryError) -> Self {
        if error.kind == RepositoryErrorKind::RepositoryBusy {
            Self::Busy
        } else {
            Self::Repository(error)
        }
    }
}

impl From<SshTransportError> for SetupPollError {
    fn from(error: SshTransportError) -> Self {
        Self::Transport(error)
    }
}

/// Which poll a request of `poll once` takes for a repository with a setup
/// on another remote (design Part G).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SetupPollPath {
    /// A first-time setup: the setup poll instead of today's poll.
    Replace,
    /// A change on another remote: today's poll of the current remote, then
    /// the setup poll.
    After,
}

/// The poll the repository at `root` takes: none for a repository with no
/// setup, a complete one, or a change on the same remote.
pub(crate) fn path_of(
    service: &RepositoryService,
    root: &Path,
) -> Result<Option<SetupPollPath>, SetupPollError> {
    Ok(path_and_primary(service, root)?.map(|(path, _)| path))
}

/// `path_of`, with the setup's primary branch, for a request that is placed
/// on it when the repository has no configuration file to name it.
pub(crate) fn path_and_primary(
    service: &RepositoryService,
    root: &Path,
) -> Result<Option<(SetupPollPath, String)>, SetupPollError> {
    let root = lock::canonical_root(root)?;
    let Some((_, record)) = lock::read_record(service, &root)? else {
        return Ok(None);
    };
    let operation = lock::operation_of(record.kind);
    let repository = git2::Repository::open(&root)
        .map_err(|error| RepositoryError::git(operation, Some(root.clone()), error))?;
    Ok(path_of_record(&repository, &record, operation, &root)?
        .map(|path| (path, record.primary_branch)))
}

/// `path_of` inside a registry transaction, for the acceptance of a request
/// that must not read the registry on a connection of its own: whether the
/// repository's setup is a first-time one the local primary does not hold.
pub(crate) fn replaces_in(connection: &rusqlite::Connection, root: &Path) -> bool {
    let Ok(root) = std::fs::canonicalize(root) else {
        return false;
    };
    let Some(key) = root.to_str() else {
        return false;
    };
    let Ok(Some(id)) = connection
        .query_row(
            "SELECT id FROM repositories WHERE root_path = ?1",
            [key],
            |row| row.get::<_, i64>(0),
        )
        .optional()
    else {
        return false;
    };
    let Ok(Some(record)) = record::read(connection, id) else {
        return false;
    };
    let Ok(repository) = git2::Repository::open(&root) else {
        return false;
    };
    matches!(
        path_of_record(&repository, &record, lock::operation_of(record.kind), &root),
        Ok(Some(SetupPollPath::Replace))
    )
}

fn path_of_record(
    repository: &git2::Repository,
    record: &SetupRecord,
    operation: RepositoryOperation,
    root: &Path,
) -> Result<Option<SetupPollPath>, RepositoryError> {
    // A setup the local primary already holds is finished, not polled.
    if complete::is_complete(repository, record)
        .map_err(|error| RepositoryError::git(operation, Some(root.to_owned()), error))?
    {
        return Ok(None);
    }
    Ok(match record.kind {
        SetupKind::Enable => Some(SetupPollPath::Replace),
        SetupKind::Change if complete::on_another_remote(repository, record) => {
            Some(SetupPollPath::After)
        }
        SetupKind::Change => None,
    })
}

/// The pull request link, built as `repo inspect` builds it: from the
/// record's configuration and the setup remote's fetch URL.
fn link(
    repository: &git2::Repository,
    record: &SetupRecord,
) -> Result<PullRequestLink, SetupPollError> {
    let configuration = std::str::from_utf8(&record.configuration)
        .ok()
        .and_then(|text| canonical::parse_repository_config(text).ok())
        .ok_or(SetupPollError::RecoveryRequired)?;
    let url = repository
        .find_remote(&record.remote_name)
        .ok()
        .and_then(|remote| remote.url().map(str::to_owned));
    let forge = forge::recognise(url.as_deref(), configuration.forge().as_ref());
    Ok(forge::pull_request_link(
        forge.as_ref(),
        &format!("manyhands/setup/{}", record.setup_id),
        &record.primary_branch,
    ))
}

impl RepositoryService {
    /// Polls the pending setup of the repository at `root` with the session's
    /// credentials; `automatic` never prompts (design Part G).
    pub fn poll_setup<P: SessionCredentialProvider>(
        &self,
        root: &Path,
        automatic: bool,
        session: &mut SessionCredentials<P>,
    ) -> Result<SetupPollReport, SetupPollError> {
        self.poll_setup_observed(root, automatic, session, None)
    }

    /// `poll_setup`, stopped by `probe` as well.
    pub(in super::super) fn poll_setup_observed<P: SessionCredentialProvider>(
        &self,
        root: &Path,
        automatic: bool,
        session: &mut SessionCredentials<P>,
        probe: Option<&CancellationProbe>,
    ) -> Result<SetupPollReport, SetupPollError> {
        let mut channel = ServiceChannel::new(self, root.to_owned(), session).automatic(automatic);
        self.poll_setup_with(root, automatic, &mut channel, probe, None)
    }

    /// The setup poll over any channel, under the setup lock. `wait` is how
    /// long a held lock is waited for: `None` is the usual timeout.
    ///
    /// It decides in the order of design Part G (G1 to G7) on the base it
    /// fetched, and activates the repository on G7.
    pub(in super::super) fn poll_setup_with(
        &self,
        root: &Path,
        automatic: bool,
        channel: &mut dyn SetupChannel,
        probe: Option<&CancellationProbe>,
        wait: Option<Duration>,
    ) -> Result<SetupPollReport, SetupPollError> {
        let held = match lock::begin_with(self, root, wait, || {})? {
            Begun::Held(held) => *held,
            // The setup ended elsewhere, and its request was finished in the
            // transaction that ended it.
            Begun::Gone => return Err(SetupPollError::Gone),
        };
        let root = lock::canonical_root(root)?;
        let operation = lock::operation_of(held.record.kind);
        let polling = Polling {
            service: self,
            git: |error| RepositoryError::git(operation, Some(root.clone()), error),
            root: &root,
            operation,
            invocation: if automatic {
                RemotePollInvocation::Automatic
            } else {
                RemotePollInvocation::Explicit
            },
            held: &held,
            link: link(&held.repository, &held.record)?,
            wait,
        };
        polling.run(channel, probe)
    }
}

struct Polling<'a, G> {
    service: &'a RepositoryService,
    git: G,
    root: &'a Path,
    operation: RepositoryOperation,
    invocation: RemotePollInvocation,
    held: &'a Held,
    link: PullRequestLink,
    wait: Option<Duration>,
}

impl<G: Fn(git2::Error) -> RepositoryError> Polling<'_, G> {
    fn record(&self) -> &SetupRecord {
        &self.held.record
    }

    fn report(&self, outcome: SetupPollOutcome) -> SetupPollReport {
        SetupPollReport {
            setup_id: self.record().setup_id.clone(),
            outcome,
            integration: None,
            differs: false,
            link: self.link.clone(),
        }
    }

    fn cancelled(&self, probe: Option<&CancellationProbe>) -> Result<(), SetupPollError> {
        match probe {
            Some(probe) if probe.asked(self.service)? => Err(SetupPollError::Cancelled),
            _ => Ok(()),
        }
    }

    /// Records the attempt of a first-time setup that tried a connection, so
    /// that the desktop schedules the setup poll by the repository's polling
    /// policy. A change on another remote records nothing: today's poll's
    /// record schedules both. Best effort: the answer does not depend on it.
    ///
    /// `Activated` records nothing either, by decision: activation is the
    /// registration of a direct enable plus remote selection, and the design's
    /// own equality test (`remote_polling_state` after activation equals the
    /// direct path's) wins over "records its attempt". The first ordinary
    /// poll is then immediately due.
    fn attempt(&self, stopped_by: Option<&SshTransportErrorKind>) {
        if self.record().kind != SetupKind::Enable {
            return;
        }
        let now = self.service.clock.now();
        let _ = state::with_transaction(self.service, self.root, |tx, id| {
            state::record_setup_poll_attempt(tx, id, stopped_by, self.invocation, now)
        });
    }

    fn answered(&self, outcome: SetupPollOutcome) -> SetupPollReport {
        self.attempt(None);
        self.report(outcome)
    }

    fn run(
        &self,
        channel: &mut dyn SetupChannel,
        probe: Option<&CancellationProbe>,
    ) -> Result<SetupPollReport, SetupPollError> {
        let record = self.record();
        let repository = &self.held.repository;
        // G1: nothing is on the remote yet, and nothing is connected to.
        if record.published_oid.is_none() {
            return Ok(self.report(SetupPollOutcome::NotPublished));
        }
        self.cancelled(probe)?;
        let scope = SetupScope::of(record);
        let advertised = match channel.fetch_base(&scope) {
            Ok(advertised) => advertised,
            Err(error) => {
                self.attempt(Some(&error.kind));
                return Err(error.into());
            }
        };
        // Nothing after the fetch is a safe point of a write: it is asked
        // before the first one.
        self.cancelled(probe)?;
        // A remote with no primary holds nothing the setup could be in.
        let base = if advertised.primary.is_some() {
            Some(
                repository
                    .find_reference(&scope.base_ref())
                    .and_then(|reference| reference.peel_to_commit())
                    .map_err(|_| SetupPollError::RecoveryRequired)?,
            )
        } else {
            None
        };
        let blob = match &base {
            Some(base) => base
                .tree()
                .map_err(&self.git)?
                .get_path(Path::new(canonical::CONFIG_PATH))
                .ok()
                .map(|entry| entry.id()),
            None => None,
        };
        // The base holds the setup when it holds a configuration that is not
        // the one the change began from (a first-time setup began from none).
        let holds = blob.is_some() && blob != record.base_configuration_oid;
        if holds {
            let configuration = repository
                .find_blob(blob.expect("a blob"))
                .ok()
                .and_then(|blob| std::str::from_utf8(blob.content()).ok().map(str::to_owned))
                .and_then(|text| canonical::parse_repository_config(&text).ok());
            // G2
            match configuration {
                None => return Ok(self.answered(SetupPollOutcome::ConfigurationInvalid)),
                Some(configuration) if configuration.primary_branch != record.primary_branch => {
                    return Ok(self.answered(SetupPollOutcome::PrimaryDiffers));
                }
                Some(_) => {}
            }
        } else if advertised.setup.is_none() {
            // G3: the setup's branch is gone, and the base does not hold it.
            return Ok(self.answered(SetupPollOutcome::BranchDeleted));
        } else {
            // G4
            return Ok(self.answered(SetupPollOutcome::WaitingForMerge));
        }
        let base = base.expect("a base that holds a configuration");

        // G5: the root is on the primary, and clean.
        let primary_ref = format!("refs/heads/{}", record.primary_branch);
        let on_primary = repository
            .find_reference("HEAD")
            .ok()
            .and_then(|head| head.symbolic_target().map(str::to_owned))
            .as_deref()
            == Some(primary_ref.as_str());
        let mut options = git2::StatusOptions::new();
        options
            .include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(false);
        let clean = repository.state() == git2::RepositoryState::Clean
            && repository
                .statuses(Some(&mut options))
                .map_err(&self.git)?
                .is_empty();
        let local = match repository.find_reference(&primary_ref) {
            Ok(reference) => reference.target(),
            Err(error) if error.code() == git2::ErrorCode::NotFound => None,
            Err(error) => return Err((self.git)(error).into()),
        };
        // The setup's branch checked out in any worktree: the finish would
        // keep it and stop, so the poll waits before it moves the primary
        // (the person switches away from the branch).
        let setup_branch = format!("refs/heads/manyhands/setup/{}", record.setup_id);
        let on_setup_branch =
            crate::repository::cleanup::branch_is_checked_out(repository, &setup_branch)
                == Ok(true);
        let Some(local) = local.filter(|_| on_primary && clean && !on_setup_branch) else {
            return Ok(self.answered(SetupPollOutcome::WaitingForCheckout));
        };
        // G6: the local primary has commits the base lacks.
        if local != base.id()
            && !repository
                .graph_descendant_of(base.id(), local)
                .map_err(&self.git)?
        {
            return Ok(self.answered(SetupPollOutcome::Diverged));
        }
        self.activate(&base, local)
    }

    /// G7, under the setup lock. In this order, so that a crash leaves a
    /// record the next poll or gate finishes: the repository's lease, the
    /// local primary's fast-forward to the base (a no-op when it is there),
    /// the remote-tracking ref of the setup's remote at the base, then the
    /// finish, then a refresh.
    fn activate(
        &self,
        base: &git2::Commit<'_>,
        local: git2::Oid,
    ) -> Result<SetupPollReport, SetupPollError> {
        let record = self.record();
        let repository = &self.held.repository;
        // What the merged file says, before the finish deletes the record.
        let merged = repository
            .find_blob(
                base.tree()
                    .map_err(&self.git)?
                    .get_path(Path::new(canonical::CONFIG_PATH))
                    .map_err(&self.git)?
                    .id(),
            )
            .map_err(&self.git)?;
        let integration = std::str::from_utf8(merged.content())
            .ok()
            .and_then(|text| canonical::parse_repository_config(text).ok())
            .map_or(Integration::Direct, |configuration| {
                configuration.integration()
            });
        let differs = merged.content() != record.configuration.as_slice();
        {
            let _lease = match self.wait {
                None => repository_lease(repository, self.root, self.operation)?,
                Some(wait) => {
                    repository_lease_waiting(repository, self.root, self.operation, wait)?
                }
            };
            if local != base.id() {
                artifacts::fast_forward::<integration::Error>(
                    repository,
                    &format!("refs/heads/{}", record.primary_branch),
                    local,
                    base.id(),
                    || Ok(()),
                    || {},
                )
                .map_err(|_| SetupPollError::RecoveryRequired)?;
            }
            repository
                .reference(
                    &format!(
                        "refs/remotes/{}/{}",
                        record.remote_name, record.primary_branch
                    ),
                    base.id(),
                    true,
                    "manyhands: set up",
                )
                .map_err(&self.git)?;
        }
        // The primary's move, made under the lease and now released, is a
        // Manyhands fast-forward that records its arrivals; the activation has
        // no operation of its own, so this one is minted for it.
        if local != base.id() {
            crate::repository::item_merges::record_after_move(
                self.service,
                self.root,
                &OperationId::new(),
                local,
                base.id(),
            );
        }
        if !complete::finish_held(&self.held.lock, self.service, self.root)? {
            // In the way: the registration is not done, and the next poll or
            // gate finishes it.
            return Err(SetupPollError::RecoveryRequired);
        }
        // `refresh_required` is set; a refresh that fails is left to the next.
        let _ = self.service.refresh_repository(RefreshRepositoryRequest {
            root: self.root.to_owned(),
            operation_id: OperationId::new(),
        });
        Ok(SetupPollReport {
            setup_id: record.setup_id.clone(),
            outcome: SetupPollOutcome::Activated,
            integration: Some(integration),
            differs,
            link: self.link.clone(),
        })
    }
}

#[cfg(test)]
#[path = "poll_tests.rs"]
mod tests;
