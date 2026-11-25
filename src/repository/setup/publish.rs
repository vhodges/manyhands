//! Publishing a setup: from a recorded setup, fetch the remote's primary,
//! make the setup commit, create the local setup branch and push it, under
//! the setup lock, so that a crash at any step is finished by the same
//! request with no second push (design Part D, "Publishing"); and the
//! ending of a refused setup (R-7).

use std::{path::Path, time::Duration};

use git2::{ErrorCode, Oid, Repository};
use rusqlite::{Transaction, TransactionBehavior};

use super::{
    channel::SetupChannel,
    commit,
    lock::{self, Begun, Held, operation_of},
    record::{self, SetupKind, SetupRecord},
};
use crate::{
    canonical::{self, CONFIG_PATH, ItemId},
    forge::{self, PullRequestLink},
    repository::{
        CommitIdentity, ReadError, RepositoryError, RepositoryErrorKind, RepositoryOperation,
        RepositoryService, RequestState, cache_write_guard,
        coordination::{repository_lease, repository_lease_waiting},
        mutation::{
            RequestId, finish_setup_request, repository_error_code, ssh_transport_error_code,
        },
        remote::sync::local_only_commits,
        request_store::RequestResult,
        transport::{SetupScope, SshTransportError, SshTransportErrorKind},
    },
    results::ResultCode,
};

/// A setup the remote holds.
#[derive(Clone, Debug)]
pub(crate) struct Published {
    pub(crate) setup_id: ItemId,
    pub(crate) commit_oid: Oid,
    pub(crate) link: PullRequestLink,
}

/// Why publishing stopped. The four refusals of the fetched base end the
/// setup and its request; every other stop keeps the record and leaves the
/// request accepted, to be continued.
#[derive(Debug)]
pub(crate) enum SetupError {
    /// The remote advertises no primary. The setup ended.
    RemotePrimaryMissing,
    /// The local primary has commits the base lacks, newest first, at most
    /// twenty, and whether that is all of them. The setup ended.
    PrimaryHasLocalCommits {
        commits: Vec<(Oid, String)>,
        complete: bool,
    },
    /// A first-time setup, and the base already holds a configuration. The
    /// setup ended.
    RemoteAlreadySetUp,
    /// A change, and the base's configuration is not the one the change
    /// began from. The setup ended.
    ExternalChange,
    /// There is no setup: it ended elsewhere, and this is the stored result
    /// of the request it finished.
    Ended(RequestResult),
    /// Another setup operation, or a writer of the repository, holds a lock.
    Busy,
    /// A remote ref or a local one is where no step of ours put it, or the
    /// setup cannot be read. The record is kept.
    RecoveryRequired,
    /// No commit identity is configured.
    IdentityRequired,
    /// The push returned and the branch is not on the remote.
    PushNotSeen,
    /// The transport stopped.
    Transport(SshTransportError),
    Repository(RepositoryError),
    Registry(ReadError),
}

impl SetupError {
    /// The code the setup answers with. The transport's `EndpointChanged`
    /// and `PublicationRemoteMissing` are `recovery_required` here (design
    /// S11, ruling R-R): the setup's remote changed or went under a record
    /// that is kept. The shared mapping is not changed for them.
    pub(crate) fn code(&self) -> ResultCode {
        match self {
            Self::RemotePrimaryMissing => ResultCode::RemotePrimaryMissing,
            Self::PrimaryHasLocalCommits { .. } => ResultCode::PrimaryHasLocalCommits,
            Self::RemoteAlreadySetUp => ResultCode::RemoteAlreadySetUp,
            Self::ExternalChange => ResultCode::ExternalChange,
            Self::Ended(stored) => stored.code,
            Self::Busy => ResultCode::Busy,
            Self::RecoveryRequired => ResultCode::RecoveryRequired,
            Self::IdentityRequired => ResultCode::IdentityRequired,
            Self::PushNotSeen => ResultCode::TransportUnavailable,
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

impl From<RepositoryError> for SetupError {
    /// A held lock, of the repository or of the setup, is `busy`.
    fn from(error: RepositoryError) -> Self {
        if error.kind == RepositoryErrorKind::RepositoryBusy {
            Self::Busy
        } else {
            Self::Repository(error)
        }
    }
}

impl From<SshTransportError> for SetupError {
    fn from(error: SshTransportError) -> Self {
        Self::Transport(error)
    }
}

/// Publishes the setup of the repository at `root`, or finishes publishing
/// it, for the request `request_id` that recorded it (design Part D).
///
/// Under the setup lock, from the record read again under it: gone, the
/// setup ended elsewhere and `Ended` has the stored result of its request.
/// Every `Err` but the four refusals keeps the record and leaves the request
/// accepted.
pub(crate) fn continue_setup(
    service: &RepositoryService,
    root: &Path,
    request_id: RequestId,
    channel: &mut dyn SetupChannel,
) -> Result<Published, SetupError> {
    continue_setup_waiting(service, root, request_id, channel, None)
}

/// `continue_setup`, waiting `wait` for a held lock instead of the usual
/// timeout, so that a test of `busy` spends no real time.
pub(super) fn continue_setup_waiting(
    service: &RepositoryService,
    root: &Path,
    request_id: RequestId,
    channel: &mut dyn SetupChannel,
    wait: Option<Duration>,
) -> Result<Published, SetupError> {
    let held = match lock::begin_with(service, root, wait, || {})? {
        Begun::Held(held) if held.record.request_id == request_id => held,
        // Gone, or another request's: this request's setup is over.
        Begun::Held(_) | Begun::Gone => return Err(ended(service, request_id)),
    };
    let Held {
        lock: _lock,
        repository,
        repository_id,
        record,
    } = *held;
    let root = lock::canonical_root(root)?;
    Publishing {
        service,
        root,
        operation: operation_of(record.kind),
        scope: SetupScope::of(&record),
        repository,
        repository_id,
        record,
        wait,
    }
    .run(channel)
}

/// What the request's stored result says, for a setup that is gone.
fn ended(service: &RepositoryService, request_id: RequestId) -> SetupError {
    match service.request_record(request_id) {
        Ok(Some(record)) if record.state == RequestState::Finished => match record.result {
            Some(result) => SetupError::Ended(result),
            None => SetupError::RecoveryRequired,
        },
        Ok(_) => SetupError::RecoveryRequired,
        Err(error) => SetupError::Registry(error),
    }
}

/// Whether a failed push may have landed, so that it is observed before it
/// is answered.
fn observed_after(kind: &SshTransportErrorKind) -> bool {
    matches!(
        kind,
        SshTransportErrorKind::PushRejected
            | SshTransportErrorKind::TransportUnavailable
            | SshTransportErrorKind::RemoteUnavailable
    )
}

struct Publishing<'a> {
    service: &'a RepositoryService,
    root: std::path::PathBuf,
    operation: RepositoryOperation,
    scope: SetupScope,
    repository: Repository,
    repository_id: i64,
    record: SetupRecord,
    wait: Option<Duration>,
}

impl Publishing<'_> {
    fn git(&self, error: git2::Error) -> SetupError {
        SetupError::Repository(RepositoryError::git(
            self.operation,
            Some(self.root.clone()),
            error,
        ))
    }

    fn run(self, channel: &mut dyn SetupChannel) -> Result<Published, SetupError> {
        // Published: the request came back after the record was made.
        if let Some(published) = self.record.published_oid {
            return self.published(published);
        }
        // What this call has already seen of the remote's branch.
        let mut seen: Option<Option<Oid>> = None;
        let commit = match self.record.commit_oid {
            None => {
                let base = self.fetch_and_decide(channel)?;
                self.make(base, None)?
            }
            Some(commit) => match self.repository.find_commit(commit) {
                Ok(_) => {
                    let lease = self.lease()?;
                    self.ensure_branch(commit, None)?;
                    drop(lease);
                    commit
                }
                Err(error) if error.code() == ErrorCode::NotFound => {
                    // A lost commit object: made again only while the
                    // remote has no branch.
                    if channel.observe_branch(&self.scope)?.is_some() {
                        return Err(SetupError::RecoveryRequired);
                    }
                    seen = Some(None);
                    let base = self.record.base_oid.ok_or(SetupError::RecoveryRequired)?;
                    self.make(base, Some(commit))?
                }
                Err(error) => return Err(self.git(error)),
            },
        };

        let at = match seen {
            Some(at) => at,
            None => channel.observe_branch(&self.scope)?,
        };
        match at {
            Some(oid) if oid == commit => {}
            Some(_) => return Err(SetupError::RecoveryRequired),
            None => {
                // Only these kinds of failure may have followed a push that
                // landed; any other stop is the answer as it is.
                let pushed = match channel.push_branch(&self.scope, commit) {
                    Err(error) if !observed_after(&error.kind) => return Err(error.into()),
                    pushed => pushed,
                };
                match channel.observe_branch(&self.scope) {
                    Ok(Some(oid)) if oid == commit => {}
                    Ok(Some(_)) => return Err(SetupError::RecoveryRequired),
                    Ok(None) => {
                        return Err(match pushed {
                            Err(error) => error.into(),
                            Ok(_) => SetupError::PushNotSeen,
                        });
                    }
                    Err(error) => {
                        return Err(match pushed {
                            Err(error) => error.into(),
                            Ok(_) => error.into(),
                        });
                    }
                }
            }
        }
        self.write(|tx| {
            record::record_published(tx, self.repository_id, commit).map_err(SetupError::from)
        })?;
        self.published(commit)
    }

    fn published(&self, commit_oid: Oid) -> Result<Published, SetupError> {
        Ok(Published {
            setup_id: self.record.setup_id.clone(),
            commit_oid,
            link: self.link()?,
        })
    }

    /// The pull request link, built as `repo inspect` builds it: from the
    /// record's configuration and the setup remote's fetch URL.
    fn link(&self) -> Result<PullRequestLink, SetupError> {
        let configuration = std::str::from_utf8(&self.record.configuration)
            .ok()
            .and_then(|text| canonical::parse_repository_config(text).ok())
            .ok_or(SetupError::RecoveryRequired)?;
        let url = self
            .repository
            .find_remote(&self.record.remote_name)
            .ok()
            .and_then(|remote| remote.url().map(str::to_owned));
        let forge = forge::recognise(url.as_deref(), configuration.forge().as_ref());
        Ok(forge::pull_request_link(
            forge.as_ref(),
            &format!("manyhands/setup/{}", self.record.setup_id),
            &self.record.primary_branch,
        ))
    }

    fn lease(&self) -> Result<crate::repository::coordination::RepositoryLease, SetupError> {
        Ok(match self.wait {
            None => repository_lease(&self.repository, &self.root, self.operation)?,
            Some(wait) => {
                repository_lease_waiting(&self.repository, &self.root, self.operation, wait)?
            }
        })
    }

    /// One registry transaction, under the registry's write guard.
    fn write<T>(
        &self,
        write: impl FnOnce(&Transaction<'_>) -> Result<T, SetupError>,
    ) -> Result<T, SetupError> {
        let service = self.service;
        let data_directory = service
            .registry_path
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let _guard = cache_write_guard(&service.registry_path, data_directory, self.operation)?;
        service.require_index_available(self.operation, Some(&self.root))?;
        let mut connection = service.open_migrated_registry(self.operation)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(RepositoryError::sqlite)?;
        let value = write(&tx)?;
        tx.commit().map_err(RepositoryError::sqlite)?;
        Ok(value)
    }

    /// Step 1: fetches the base through the channel and decides on it. The
    /// four refusals end the setup. The base is the commit of the private
    /// base ref the fetch wrote.
    fn fetch_and_decide(&self, channel: &mut dyn SetupChannel) -> Result<Oid, SetupError> {
        let advertised = channel.fetch_base(&self.scope)?;
        if advertised.primary.is_none() {
            return Err(self.end_refused(SetupError::RemotePrimaryMissing));
        }
        let base = self
            .repository
            .find_reference(&self.scope.base_ref())
            .and_then(|reference| reference.peel_to_commit())
            .map_err(|_| SetupError::RecoveryRequired)?;
        let base_oid = base.id();

        let local = match self
            .repository
            .find_reference(&format!("refs/heads/{}", self.record.primary_branch))
        {
            Ok(reference) => reference.target(),
            Err(error) if error.code() == ErrorCode::NotFound => None,
            Err(error) => return Err(self.git(error)),
        };
        if let Some(local) = local
            && local != base_oid
            && !self
                .repository
                .graph_descendant_of(base_oid, local)
                .map_err(|error| self.git(error))?
        {
            let refusal = match local_only_commits(&self.repository, local, base_oid) {
                Ok((commits, complete)) => SetupError::PrimaryHasLocalCommits { commits, complete },
                Err(_) => return Err(SetupError::RecoveryRequired),
            };
            return Err(self.end_refused(refusal));
        }

        let configuration = base
            .tree()
            .map_err(|error| self.git(error))?
            .get_path(Path::new(CONFIG_PATH))
            .ok()
            .map(|entry| entry.id());
        match self.record.kind {
            SetupKind::Enable if configuration.is_some() => {
                return Err(self.end_refused(SetupError::RemoteAlreadySetUp));
            }
            SetupKind::Change if configuration != self.record.base_configuration_oid => {
                return Err(self.end_refused(SetupError::ExternalChange));
            }
            _ => {}
        }
        Ok(base_oid)
    }

    /// Step 2, under the repository lease: makes the setup commit on
    /// `base` and records it (in place of `replacing`, a lost one), then
    /// creates the local branch.
    fn make(&self, base: Oid, replacing: Option<Oid>) -> Result<Oid, SetupError> {
        let identity = self.identity()?.ok_or(SetupError::IdentityRequired)?;
        let message = match self.record.kind {
            SetupKind::Enable => commit::ENABLE_MESSAGE,
            SetupKind::Change => commit::CHANGE_MESSAGE,
        };
        let lease = self.lease()?;
        let made = commit::build(
            &self.repository,
            base,
            &self.record.configuration,
            message,
            &identity,
        )
        .map_err(|error| self.git(error))?;
        self.write(|tx| {
            match replacing {
                None => record::record_commit(tx, self.repository_id, base, made),
                Some(lost) => record::replace_commit(tx, self.repository_id, lost, made),
            }
            .map_err(SetupError::from)
        })?;
        self.ensure_branch(made, replacing)?;
        drop(lease);
        Ok(made)
    }

    /// The configured commit identity: both parts set and not empty.
    fn identity(&self) -> Result<Option<CommitIdentity>, SetupError> {
        let config = self.repository.config().map_err(|error| self.git(error))?;
        let name = config.get_string("user.name").ok();
        let email = config.get_string("user.email").ok();
        Ok(match (name, email) {
            (Some(name), Some(email)) if !name.is_empty() && !email.is_empty() => {
                Some(CommitIdentity { name, email })
            }
            _ => None,
        })
    }

    /// The local setup branch at `commit`: made when absent, and moved from
    /// `replacing`, the lost commit it may still name. Any other OID is a
    /// branch no step of ours put there.
    fn ensure_branch(&self, commit: Oid, replacing: Option<Oid>) -> Result<(), SetupError> {
        let name = self.scope.branch_ref();
        let message = "manyhands: set up";
        match self.repository.find_reference(&name) {
            Ok(reference) => match reference.target() {
                Some(target) if target == commit => Ok(()),
                Some(target) if Some(target) == replacing => self
                    .repository
                    .reference_matching(&name, commit, true, target, message)
                    .map(drop)
                    .map_err(|error| self.git(error)),
                _ => Err(SetupError::RecoveryRequired),
            },
            Err(error) if error.code() == ErrorCode::NotFound => self
                .repository
                .reference(&name, commit, false, message)
                .map(drop)
                .map_err(|error| self.git(error)),
            Err(error) => Err(self.git(error)),
        }
    }

    /// Ends a refused setup (R-7): the private base ref is deleted, then in
    /// one registry transaction the setup request is finished with the
    /// refusal, the record is deleted, and for a first-time setup the
    /// registration too. The request's own operation row is not touched. The
    /// refusal is the answer, unless the ending itself failed: then that
    /// failure is, the record and the accepted request stay, and the next
    /// call reaches the same refusal again.
    fn end_refused(&self, refusal: SetupError) -> SetupError {
        match self.end(refusal.code()) {
            Ok(()) => refusal,
            Err(error) => error,
        }
    }

    fn end(&self, code: ResultCode) -> Result<(), SetupError> {
        match self.repository.find_reference(&self.scope.base_ref()) {
            Ok(mut reference) => reference.delete().map_err(|error| self.git(error))?,
            Err(error) if error.code() == ErrorCode::NotFound => {}
            Err(error) => return Err(self.git(error)),
        }
        let now = self.service.clock.now().unix_timestamp();
        self.write(|tx| {
            // The request first: its scope is the registration's root.
            finish_setup_request(tx, self.repository_id, self.record.request_id, code, now)
                .map_err(SetupError::Registry)?;
            record::delete(tx, self.repository_id)?;
            if self.record.kind == SetupKind::Enable {
                tx.execute(
                    "DELETE FROM repositories WHERE id = ?1",
                    [self.repository_id],
                )
                .map_err(RepositoryError::sqlite)?;
            }
            Ok(())
        })
    }
}

#[cfg(test)]
#[path = "publish_tests.rs"]
mod tests;
