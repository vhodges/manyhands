//! Cancelling a pending setup (design Part H): the remote setup branch is
//! deleted only at its recorded commit, the local branch, the private base
//! ref and the record are removed, a first-time setup is unregistered, and
//! the setup's own request and the cancel's are finished, all under the setup
//! lock.
// The transport's typed error is large by design, as in `transport` and `remote`.
#![allow(clippy::result_large_err)]

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use git2::{ErrorCode, Oid, Repository};
use rusqlite::TransactionBehavior;

use super::{
    channel::{ServiceChannel, SetupChannel},
    data as setup_data,
    lock::{self, Begun, Held},
    publish::SetupError,
    record::{self, SetupKind, SetupRecord},
};
use crate::{
    canonical::{CONFIG_PATH, ItemId},
    repository::{
        Blocker, OperationId, ReadError, RepositoryError, RepositoryErrorKind, RepositoryOperation,
        RepositoryService, RequestState, cache_write_guard,
        coordination::{RepositoryLease, repository_lease, repository_lease_waiting},
        keys::{SessionCredentialProvider, SessionCredentials},
        mutation::{
            RequestId, finish_cancel_request, finish_setup_request, repository_error_code,
            ssh_transport_error_code,
        },
        recovery::{self, LocalStep},
        request_store::RequestResult,
        transport::{DeleteAttempt, SshTransportError, SshTransportErrorKind},
    },
    results::ResultCode,
};

/// What `repo setup-cancel` asks of the domain: the repository, and the
/// operation and request it runs under.
#[derive(Clone, Debug)]
pub struct CancelSetupRequest {
    pub root: PathBuf,
    pub operation_id: OperationId,
    pub request_id: RequestId,
}

/// What became of the remote setup branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteBranch {
    /// Deleted by this cancel, at its recorded commit.
    Deleted,
    /// Not on the remote, or gone from it.
    Absent,
    /// Left: it is at another commit than the recorded one.
    Kept,
    /// The setup was never published: no commit, no connection.
    NotPublished,
}

impl RemoteBranch {
    pub const ALL: [Self; 4] = [Self::Deleted, Self::Absent, Self::Kept, Self::NotPublished];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Deleted => "deleted",
            Self::Absent => "absent",
            Self::Kept => "kept",
            Self::NotPublished => "not_published",
        }
    }
}

/// What became of the local setup branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalBranch {
    Deleted,
    Absent,
    /// Left: it is at another commit than the recorded one.
    Kept,
}

impl LocalBranch {
    pub const ALL: [Self; 3] = [Self::Deleted, Self::Absent, Self::Kept];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Deleted => "deleted",
            Self::Absent => "absent",
            Self::Kept => "kept",
        }
    }
}

/// What a cancel did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupCancelReport {
    pub setup_id: ItemId,
    pub(crate) kind: SetupKind,
    pub remote_branch: RemoteBranch,
    pub local_branch: LocalBranch,
    /// A first-time setup's registration was removed.
    pub unregistered: bool,
}

/// Why a cancel stopped. The record, the branches and the setup's request
/// are as they were, except what a step before the stop had already done
/// (the cancel's own row, a deleted remote branch).
#[derive(Debug)]
pub enum SetupCancelError {
    /// There is no setup: the end state holds.
    NoSetup,
    /// The cancel's own request was finished already, in the transaction
    /// that ended the setup: this is its stored result.
    Ended(RequestResult),
    /// The setup has merged: the remote's primary holds it (design R-14).
    Merged,
    /// Another operation of the repository has not finished, and the row or
    /// the reservation, when one names it.
    InTheWay(Option<Blocker>),
    /// The remote still has the branch after the delete: it was refused or
    /// lost.
    DeleteNotSeen,
    /// Another setup operation, or a writer of the repository, holds a lock.
    Busy,
    /// The setup cannot be read, or something is where no step of ours put
    /// it.
    RecoveryRequired,
    Transport(SshTransportError),
    Repository(RepositoryError),
    Registry(ReadError),
}

impl SetupCancelError {
    /// The code the stop answers with.
    pub fn code(&self) -> ResultCode {
        match self {
            Self::NoSetup => ResultCode::AlreadyApplied,
            Self::Ended(stored) => stored.code,
            Self::Merged | Self::InTheWay(_) | Self::RecoveryRequired => {
                ResultCode::RecoveryRequired
            }
            Self::DeleteNotSeen => ResultCode::PushRejected,
            Self::Busy => ResultCode::Busy,
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

/// What a preview of `repo setup-cancel` binds of the setup: its ID, its
/// kind, its remote and how far it has come, so that a publish between
/// `prepare` and the confirm is `external_change`. `None` is no setup. The
/// binding digests it.
pub(crate) fn state_key(record: Option<&SetupRecord>) -> String {
    match record {
        None => "none".to_owned(),
        Some(record) => format!(
            "{}|{}|{}|{}|{}",
            record.setup_id,
            record.kind.as_str(),
            record.remote_name,
            record.state().as_str(),
            record
                .commit_oid
                .map(|oid| oid.to_string())
                .unwrap_or_default()
        ),
    }
}

/// The steps of a cancel a test can stop after, to read what a crash there
/// leaves.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Step {
    /// The cancel's own row is recorded and its lease released.
    Recorded,
    /// The remote branch is gone, or was kept.
    Remote,
    /// The local branch and the private ref are deleted.
    Local,
    /// The registry transaction committed.
    Transaction,
}

#[cfg(test)]
thread_local! {
    /// The step a cancel stops after, once: the call answers
    /// `recovery_required` there, as a crash would end it.
    pub(super) static STOP_AFTER: std::cell::Cell<Option<Step>> =
        const { std::cell::Cell::new(None) };
    /// Runs right after the remote step and before the transaction: a test
    /// changes the repository as another process would.
    pub(super) static BEFORE_TRANSACTION: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
}

impl From<SshTransportError> for SetupCancelError {
    fn from(error: SshTransportError) -> Self {
        Self::Transport(error)
    }
}

impl From<ReadError> for SetupCancelError {
    fn from(error: ReadError) -> Self {
        Self::Registry(error)
    }
}

impl From<RepositoryError> for SetupCancelError {
    /// A held lock, of the repository or of the setup, is `busy`; another row
    /// in the way is `recovery_required` naming it; a row of the same
    /// operation ID that names another setup says that the setup this request
    /// began for is gone.
    fn from(error: RepositoryError) -> Self {
        match error.kind {
            RepositoryErrorKind::RepositoryBusy => Self::Busy,
            RepositoryErrorKind::RecoveryRequired => Self::InTheWay(error.blocker().cloned()),
            RepositoryErrorKind::OperationMismatch => Self::NoSetup,
            _ => Self::Repository(error),
        }
    }
}

/// The setup of the repository registered at `root`: none for a root that is
/// not registered or has no setup.
pub(crate) fn current_setup(
    service: &RepositoryService,
    root: &Path,
) -> Result<Option<SetupRecord>, SetupCancelError> {
    Ok(lock::read_record(service, root)?.map(|(_, record)| record))
}

impl From<SetupError> for SetupCancelError {
    fn from(error: SetupError) -> Self {
        match error {
            SetupError::Busy => Self::Busy,
            SetupError::Transport(error) => Self::Transport(error),
            SetupError::Repository(error) => error.into(),
            SetupError::Registry(error) => Self::Registry(error),
            _ => Self::RecoveryRequired,
        }
    }
}

/// Whether a failed delete may have landed, so that it is observed before it
/// is answered.
fn observed_after(kind: &SshTransportErrorKind) -> bool {
    matches!(
        kind,
        SshTransportErrorKind::PushRejected
            | SshTransportErrorKind::TransportUnavailable
            | SshTransportErrorKind::RemoteUnavailable
    )
}

/// Stops a cancel after `step`, once, when a test asks it to.
#[cfg(test)]
fn reached(step: Step) -> Result<(), SetupCancelError> {
    if STOP_AFTER.with(|stop| stop.get()) == Some(step) {
        STOP_AFTER.with(|stop| stop.set(None));
        return Err(SetupCancelError::RecoveryRequired);
    }
    Ok(())
}

impl RepositoryService {
    /// Cancels the pending setup of `request.root` with the session's
    /// credentials (design Part H).
    pub fn cancel_setup<P: SessionCredentialProvider>(
        &self,
        request: &CancelSetupRequest,
        session: &mut SessionCredentials<P>,
    ) -> Result<SetupCancelReport, SetupCancelError> {
        let mut channel = ServiceChannel::new(self, request.root.clone(), session);
        self.cancel_setup_with(request, &mut channel, None)
    }

    /// The cancel over any channel, under the setup lock (design Part H).
    /// `wait` is how long a held lock is waited for: `None` is the usual
    /// timeout.
    ///
    /// In order: the lock and the record; the cancel's own row, recorded at
    /// `setup_cancel_recorded` with the lease released before any
    /// connection; the remote branch (one fetch, and a guarded delete and its
    /// observation only for a branch at the recorded commit); the local
    /// branch and the private ref; one registry transaction. Every step is
    /// idempotent, so the same request finishes a cancel that stopped after
    /// any of them.
    pub(in super::super) fn cancel_setup_with(
        &self,
        request: &CancelSetupRequest,
        channel: &mut dyn SetupChannel,
        wait: Option<Duration>,
    ) -> Result<SetupCancelReport, SetupCancelError> {
        let held = match lock::begin_with(self, &request.root, wait, || {})? {
            Begun::Held(held) => *held,
            Begun::Gone => return Err(self.nothing_to_cancel(request)),
        };
        let Held {
            lock: _lock,
            repository,
            repository_id,
            record,
        } = held;
        let root = lock::canonical_root(&request.root)?;
        let cancelling = Cancelling {
            service: self,
            operation: RepositoryOperation::CancelSetup,
            request,
            root,
            repository,
            repository_id,
            wait,
        };
        cancelling.run(record, channel)
    }

    /// What a cancel finds when there is no setup: its own request's stored
    /// result when the transaction that ended the setup finished it, and the
    /// end state otherwise.
    fn nothing_to_cancel(&self, request: &CancelSetupRequest) -> SetupCancelError {
        match self.request_record(request.request_id) {
            Ok(Some(record)) if record.state == RequestState::Finished => match record.result {
                Some(result) => SetupCancelError::Ended(result),
                None => SetupCancelError::RecoveryRequired,
            },
            Ok(_) => SetupCancelError::NoSetup,
            Err(error) => SetupCancelError::Registry(error),
        }
    }
}

struct Cancelling<'a> {
    service: &'a RepositoryService,
    operation: RepositoryOperation,
    request: &'a CancelSetupRequest,
    root: PathBuf,
    repository: Repository,
    repository_id: i64,
    wait: Option<Duration>,
}

impl Cancelling<'_> {
    fn git(&self, error: git2::Error) -> SetupCancelError {
        SetupCancelError::Repository(RepositoryError::git(
            self.operation,
            Some(self.root.clone()),
            error,
        ))
    }

    fn lease(&self) -> Result<RepositoryLease, SetupCancelError> {
        Ok(match self.wait {
            None => repository_lease(&self.repository, &self.root, self.operation)?,
            Some(wait) => {
                repository_lease_waiting(&self.repository, &self.root, self.operation, wait)?
            }
        })
    }

    fn run(
        &self,
        record: SetupRecord,
        channel: &mut dyn SetupChannel,
    ) -> Result<SetupCancelReport, SetupCancelError> {
        let scope = crate::repository::transport::SetupScope::of(&record);
        // 1. The cancel's own row, completed at `setup_cancel_recorded`, and
        // the lease let go before any connection. Another row in the way
        // stops it here, with nothing sent.
        let own_row = self.record_own_row(&record)?;
        #[cfg(test)]
        reached(Step::Recorded)?;

        // 2. The remote branch.
        let remote_branch = match record.commit_oid {
            None => RemoteBranch::NotPublished,
            Some(commit) => self.remote_step(&record, &scope, commit, channel)?,
        };
        #[cfg(test)]
        reached(Step::Remote)?;

        // 3. The local branch and the private ref, under the lease.
        let local_branch = {
            let _lease = self.lease()?;
            self.local_step(&record, &scope)?
        };
        #[cfg(test)]
        reached(Step::Local)?;

        let report = SetupCancelReport {
            setup_id: record.setup_id.clone(),
            kind: record.kind,
            remote_branch,
            local_branch,
            unregistered: record.kind == SetupKind::Enable,
        };
        #[cfg(test)]
        if let Some(hook) = BEFORE_TRANSACTION.with(|hook| hook.borrow_mut().take()) {
            hook();
        }
        // 4. One registry transaction.
        let _lease = self.lease()?;
        self.end(&record, own_row, &report)?;
        #[cfg(test)]
        reached(Step::Transaction)?;
        Ok(report)
    }

    /// The cancel's own row: begun under the lease, so that another row in
    /// the way refuses it, completed at `setup_cancel_recorded`, and the
    /// lease released. Its target is the setup's ID, so a request that is
    /// entered again for a setup that was replaced by another is refused as
    /// `NoSetup`. The row's ID in the journal, `0` when the registry is
    /// unavailable.
    fn record_own_row(&self, record: &SetupRecord) -> Result<i64, SetupCancelError> {
        let (lease, row) = self.service.begin_lifecycle(
            &self.repository,
            &self.root,
            self.operation,
            self.request.operation_id,
            &record.setup_id.to_string(),
        )?;
        if row.id != 0 {
            self.write(|tx| {
                recovery::complete_with_step(tx, row.id, LocalStep::SetupCancelRecorded)
                    .map_err(SetupCancelError::from)
            })?;
        }
        drop(lease);
        Ok(row.id)
    }

    /// One registry transaction, under the registry's write guard.
    fn write<T>(
        &self,
        write: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, SetupCancelError>,
    ) -> Result<T, SetupCancelError> {
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

    /// Step 2: the base is fetched and the setup's branch observed in the
    /// same advertisement. A setup the base already holds is refused. The
    /// branch is deleted only at the recorded commit, and what a delete left
    /// is read by an advertisement of its own.
    fn remote_step(
        &self,
        record: &SetupRecord,
        scope: &crate::repository::transport::SetupScope,
        commit: Oid,
        channel: &mut dyn SetupChannel,
    ) -> Result<RemoteBranch, SetupCancelError> {
        let advertised = channel.fetch_base(scope)?;
        if advertised.primary.is_some() && self.base_holds_the_setup(record, scope)? {
            return Err(SetupCancelError::Merged);
        }
        match advertised.setup {
            None => Ok(RemoteBranch::Absent),
            Some(at) if at != commit => Ok(RemoteBranch::Kept),
            Some(_) => {
                // The delete's own answer is no evidence of what it left.
                let attempt = match channel.delete_branch(scope, commit) {
                    Ok(attempt) => Some(attempt),
                    Err(error) if observed_after(&error.kind) => None,
                    Err(error) => return Err(error.into()),
                };
                match channel.observe_branch(scope)? {
                    None if attempt == Some(DeleteAttempt::Absent) => Ok(RemoteBranch::Absent),
                    None => Ok(RemoteBranch::Deleted),
                    Some(at) if at == commit => Err(SetupCancelError::DeleteNotSeen),
                    Some(_) => Ok(RemoteBranch::Kept),
                }
            }
        }
    }

    /// Whether the fetched base holds the setup: a configuration that is not
    /// the one a change began from (a first-time setup began from none), as
    /// the setup poll decides it, without the checkout checks.
    fn base_holds_the_setup(
        &self,
        record: &SetupRecord,
        scope: &crate::repository::transport::SetupScope,
    ) -> Result<bool, SetupCancelError> {
        let base = self
            .repository
            .find_reference(&scope.base_ref())
            .and_then(|reference| reference.peel_to_commit())
            .map_err(|_| SetupCancelError::RecoveryRequired)?;
        let blob = base
            .tree()
            .map_err(|error| self.git(error))?
            .get_path(Path::new(CONFIG_PATH))
            .ok()
            .map(|entry| entry.id());
        Ok(blob.is_some() && blob != record.base_configuration_oid)
    }

    /// Step 3: the local setup branch, only at the recorded commit and while
    /// no worktree's HEAD names it (kept otherwise), and the private base ref. Both
    /// are idempotent.
    fn local_step(
        &self,
        record: &SetupRecord,
        scope: &crate::repository::transport::SetupScope,
    ) -> Result<LocalBranch, SetupCancelError> {
        let branch = match self.repository.find_reference(&scope.branch_ref()) {
            Ok(mut reference) => {
                // A branch that any worktree's HEAD names is kept as well:
                // deleting it would leave that HEAD on a branch that is not
                // there (K-15), and `Reference::delete` does not look. A
                // checkout that cannot be told is a checkout.
                if reference.target().is_some()
                    && reference.target() == record.commit_oid
                    && crate::repository::cleanup::branch_is_checked_out(
                        &self.repository,
                        &scope.branch_ref(),
                    ) == Ok(false)
                {
                    reference.delete().map_err(|error| self.git(error))?;
                    LocalBranch::Deleted
                } else {
                    LocalBranch::Kept
                }
            }
            Err(error) if error.code() == ErrorCode::NotFound => LocalBranch::Absent,
            Err(error) => return Err(self.git(error)),
        };
        match self.repository.find_reference(&scope.base_ref()) {
            Ok(mut reference) => reference.delete().map_err(|error| self.git(error))?,
            Err(error) if error.code() == ErrorCode::NotFound => {}
            Err(error) => return Err(self.git(error)),
        }
        Ok(branch)
    }

    /// Step 4, in one transaction: nothing else of the repository is in the
    /// way; the setup's request is finished as `cancelled`; the record is
    /// deleted, and for a first-time setup the root's other operation rows and
    /// the registration, as `repo remove` deletes them, keeping the cancel's
    /// own row; and the cancel's own request is finished with its answer.
    fn end(
        &self,
        record: &SetupRecord,
        own_row: i64,
        report: &SetupCancelReport,
    ) -> Result<(), SetupCancelError> {
        // Read before the transaction: the record, as the setup's request
        // is finished with it.
        let data = setup_data::of(self.service, &self.root, None);
        let now = self.service.clock.now().unix_timestamp();
        let root_key = self
            .root
            .to_str()
            .ok_or(SetupCancelError::RecoveryRequired)?
            .to_owned();
        self.write(|tx| {
            // The setup is the one this cancel began for.
            match record::read(tx, self.repository_id)? {
                Some(current) if current.setup_id == record.setup_id => {}
                _ => return Err(SetupCancelError::RecoveryRequired),
            }
            recovery::require_not_in_the_way(
                tx,
                &self.root,
                self.operation,
                self.request.operation_id,
                &record.setup_id.to_string(),
            )?;
            // The setup's request first: its guard reads its own row.
            let finished = finish_setup_request(
                tx,
                self.repository_id,
                record.request_id,
                ResultCode::Cancelled,
                data,
                now,
            )?;
            self.refuse_if_still_accepted(tx, record.request_id, finished)?;
            record::delete(tx, self.repository_id)?;
            if record.kind == SetupKind::Enable {
                tx.execute(
                    "DELETE FROM operation_records WHERE root_path = ?1 AND id != ?2",
                    rusqlite::params![root_key, own_row],
                )
                .map_err(RepositoryError::sqlite)?;
                tx.execute(
                    "DELETE FROM repositories WHERE id = ?1",
                    [self.repository_id],
                )
                .map_err(RepositoryError::sqlite)?;
            }
            let own = finish_cancel_request(tx, self.request.request_id, report, now)?;
            self.refuse_if_still_accepted(tx, self.request.request_id, own)?;
            Ok(())
        })
    }

    /// A request already finished is fine. One that is still accepted that
    /// the guard refused would be left with its setup gone (R-19), so
    /// nothing is written.
    fn refuse_if_still_accepted(
        &self,
        tx: &rusqlite::Transaction<'_>,
        request_id: RequestId,
        finished: bool,
    ) -> Result<(), SetupCancelError> {
        if finished {
            return Ok(());
        }
        let state: Option<String> = rusqlite::OptionalExtension::optional(tx.query_row(
            "SELECT state FROM request_records WHERE request_ulid = ?1",
            [request_id.to_string()],
            |row| row.get(0),
        ))
        .map_err(RepositoryError::sqlite)?;
        if state.as_deref() == Some(RequestState::Accepted.as_str()) {
            return Err(SetupCancelError::RecoveryRequired);
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "cancel_tests.rs"]
mod tests;
