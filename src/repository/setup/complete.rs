//! Whether a pending setup has merged: level-triggered, from what the local
//! primary holds now (design Part B, R-17).
use std::{path::Path, time::Duration};

use rusqlite::{OptionalExtension, TransactionBehavior};

use super::{
    data as setup_data,
    lock::{self, Begun, Held},
    publish::SetupError,
    record::{self, SetupKind, SetupRecord},
};
use crate::{
    canonical,
    repository::{
        OperationId, RepositoryError, RepositoryErrorKind, RepositoryOperation, RepositoryService,
        RequestState, cache_write_guard, coordination::SetupLock, mutation::finish_setup_request,
        recovery::advance_after_observation, remote::state,
    },
    results::ResultCode,
};

/// Whether the local primary already holds the setup `record`.
///
/// This reads `refs/heads/<primary_branch>`, never HEAD: HEAD may name the
/// setup branch, which holds the configuration before anyone merges it. The
/// setup is complete when that commit's tree holds a valid configuration,
/// read as the canonical reader reads it, that names the same primary
/// branch, and, for a change, whose blob is not the one the change began
/// from. A primary that does not exist, or holds no file, is not complete.
/// Git failing to answer is the error; the caller treats it as pending.
pub(crate) fn is_complete(
    repository: &git2::Repository,
    record: &SetupRecord,
) -> Result<bool, git2::Error> {
    let reference =
        match repository.find_reference(&format!("refs/heads/{}", record.primary_branch)) {
            Ok(reference) => reference,
            Err(error) if error.code() == git2::ErrorCode::NotFound => return Ok(false),
            Err(error) => return Err(error),
        };
    let tree = reference.peel_to_commit()?.tree()?;
    let entry = match tree.get_path(Path::new(canonical::CONFIG_PATH)) {
        Ok(entry) => entry,
        Err(error) if error.code() == git2::ErrorCode::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    let Ok(blob) = repository.find_blob(entry.id()) else {
        // A directory, or a submodule, where the file should be.
        return Ok(false);
    };
    let names_the_primary = std::str::from_utf8(blob.content())
        .ok()
        .and_then(|source| canonical::parse_repository_config(source).ok())
        .is_some_and(|configuration| configuration.primary_branch == record.primary_branch);
    Ok(names_the_primary && record.base_configuration_oid != Some(entry.id()))
}

/// Finishes the setup of the repository at `root` when the local primary
/// holds it, under the setup lock (design Part B). `Ok(false)` when there is
/// nothing to finish: no setup, one not complete, HEAD not on the primary,
/// the lock held, or an operation in the way (the next look finishes it).
pub(crate) fn finish(service: &RepositoryService, root: &Path) -> Result<bool, RepositoryError> {
    // The wait for a held lock is the service's lease wait, which a test
    // shortens (`with_lease_wait_for_testing`): no real time passes.
    finish_waiting(service, root, Some(service.lease_wait))
}

/// `finish`, waiting `wait` for a held lock instead of the usual timeout, so
/// that a test of the held lock spends no real time.
pub(super) fn finish_waiting(
    service: &RepositoryService,
    root: &Path,
    wait: Option<Duration>,
) -> Result<bool, RepositoryError> {
    match lock::begin_with(service, root, wait, || {}) {
        Ok(Begun::Held(held)) => {
            let Held {
                lock: _lock,
                repository,
                repository_id,
                record,
            } = *held;
            let root = lock::canonical_root(root).map_err(into_repository_error)?;
            finish_record(service, &root, &repository, repository_id, &record)
        }
        Ok(Begun::Gone) => Ok(false),
        Err(SetupError::Busy) => Ok(false),
        Err(error) => Err(into_repository_error(error)),
    }
}

/// `finish`, for a caller that already holds the setup lock `lock`.
pub(crate) fn finish_held(
    _lock: &SetupLock,
    service: &RepositoryService,
    root: &Path,
) -> Result<bool, RepositoryError> {
    let root = lock::canonical_root(root).map_err(into_repository_error)?;
    let Some((repository_id, record)) =
        lock::read_record(service, &root).map_err(into_repository_error)?
    else {
        return Ok(false);
    };
    let repository = git2::Repository::open(&root).map_err(|error| {
        RepositoryError::git(lock::operation_of(record.kind), Some(root.clone()), error)
    })?;
    finish_record(service, &root, &repository, repository_id, &record)
}

fn into_repository_error(error: SetupError) -> RepositoryError {
    match error {
        SetupError::Repository(error) => error,
        _ => state::recovery_required(),
    }
}

/// The finish proper, under the setup lock, for the record read under it.
fn finish_record(
    service: &RepositoryService,
    root: &Path,
    repository: &git2::Repository,
    repository_id: i64,
    record: &SetupRecord,
) -> Result<bool, RepositoryError> {
    let operation = lock::operation_of(record.kind);
    let git = |error| RepositoryError::git(operation, Some(root.to_owned()), error);
    if !is_complete(repository, record).map_err(git)? {
        return Ok(false);
    }
    // Only while HEAD names the primary: with the setup branch (or any other)
    // checked out, the working tree is not the primary's.
    let primary_ref = format!("refs/heads/{}", record.primary_branch);
    let head = repository.find_reference("HEAD").map_err(git)?;
    if head.symbolic_target() != Some(primary_ref.as_str()) {
        return Ok(false);
    }
    // What the primary holds is the merged configuration.
    let merged = repository
        .find_reference(&primary_ref)
        .and_then(|reference| reference.peel_to_commit())
        .and_then(|commit| commit.tree())
        .and_then(|tree| tree.get_path(Path::new(canonical::CONFIG_PATH)))
        .and_then(|entry| repository.find_blob(entry.id()))
        .map_err(git)?
        .content()
        .to_vec();

    // 1. The local setup branch, only at the commit the record names (it is
    // kept anywhere else, K-15), and the private base ref. Both are deleted
    // before the record, so a crash leaves a record the next look finishes.
    let branch = format!("refs/heads/manyhands/setup/{}", record.setup_id);
    match repository.find_reference(&branch) {
        Ok(mut reference) if reference.target() == record.commit_oid => {
            // `Reference::delete` does not look at other worktrees: a branch
            // checked out in any of them (or one whose checkouts cannot all
            // be told) is not deleted from under it. The finish stops, as
            // when HEAD is off the primary, and the next look finishes it.
            if crate::repository::cleanup::branch_is_checked_out(repository, &branch) != Ok(false) {
                return Ok(false);
            }
            reference.delete().map_err(git)?;
        }
        Ok(_) => {}
        Err(error) if error.code() == git2::ErrorCode::NotFound => {}
        Err(error) => return Err(git(error)),
    }
    let base = format!("refs/manyhands/setups/{}/base", record.setup_id);
    match repository.find_reference(&base) {
        Ok(mut reference) => reference.delete().map_err(git)?,
        Err(error) if error.code() == git2::ErrorCode::NotFound => {}
        Err(error) => return Err(git(error)),
    }

    // 2. A setup on another remote reconciles the registration and stores
    // the remote plan as remote selection does, under the lifecycle check
    // remote selection takes. In the way, it stops and the next look
    // finishes it.
    let lifecycle = if on_another_remote(repository, record) {
        let begin = |operation_id| {
            service.begin_lifecycle(
                repository,
                root,
                RepositoryOperation::SetPublicationRemote,
                operation_id,
                &record.remote_name,
            )
        };
        // A person may have abandoned the row a crash left under the stable
        // ID, which spends the ID. The abandoned row is closed and no longer
        // in the way, so the finish goes on under a fresh one.
        let begun = match begin(lifecycle_operation_id(record)) {
            Err(error) if error.kind == RepositoryErrorKind::OperationAbandoned => {
                begin(OperationId::new())
            }
            other => other,
        };
        match begun {
            Ok(begun) => Some(begun),
            Err(error)
                if matches!(
                    error.kind,
                    RepositoryErrorKind::RecoveryRequired | RepositoryErrorKind::RepositoryBusy
                ) =>
            {
                return Ok(false);
            }
            Err(error) => return Err(error),
        }
    } else {
        None
    };
    let ended = (|| -> Result<(), RepositoryError> {
        if lifecycle.is_some() {
            service.reconcile_registration(
                repository,
                root,
                RepositoryOperation::SetPublicationRemote,
            )?;
        }
        // 3. One registry transaction: the request, if still accepted, then
        // the record. `already_applied` when the merged file is the intended
        // one, `external_change` when it was edited in review.
        let code = if merged == record.configuration {
            ResultCode::AlreadyApplied
        } else {
            ResultCode::ExternalChange
        };
        let data = setup_data::of(service, root, None);
        let now = service.clock.now().unix_timestamp();
        let data_directory = service
            .registry_path
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let _guard = cache_write_guard(&service.registry_path, data_directory, operation)?;
        service.require_index_available(operation, Some(root))?;
        let mut connection = service.open_migrated_registry(operation)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(RepositoryError::sqlite)?;
        let finished = finish_setup_request(&tx, repository_id, record.request_id, code, data, now)
            .map_err(|_| state::recovery_required())?;
        if !finished {
            // Already finished is fine. Still accepted, the guard refused it:
            // ending the setup now would leave an accepted request with no
            // record (R-19), so nothing is written.
            let state: Option<String> = tx
                .query_row(
                    "SELECT state FROM request_records WHERE request_ulid = ?1",
                    [record.request_id.to_string()],
                    |row| row.get(0),
                )
                .optional()
                .map_err(RepositoryError::sqlite)?;
            if state.as_deref() == Some(RequestState::Accepted.as_str()) {
                return Err(state::recovery_required());
            }
        }
        record::delete(&tx, repository_id)?;
        // A first-time setup's own poll recorded attempts (a change's records
        // none, and the attempt of today's poll of its current remote is the
        // repository's own, which a direct selection keeps). They would hold
        // the first ordinary poll back until their next eligible time: they
        // are cleared, as a repository registered by a direct enable and a
        // selection has none, and the first poll is due at once.
        if lifecycle.is_some() && record.kind == SetupKind::Enable {
            tx.execute(
                "UPDATE remote_polling_state
                    SET latest_attempt_at = NULL, next_eligible_at = NULL
                  WHERE repository_id = ?1",
                [repository_id],
            )
            .map_err(RepositoryError::sqlite)?;
        }
        // The lifecycle row ends in the same transaction, so that no crash
        // after the commit leaves it pending with the record gone.
        if let Some((_, row)) = &lifecycle
            && row.id != 0
        {
            advance_after_observation(&tx, row.id, "completed", None, None)?;
        }
        let committed = tx.commit().map_err(RepositoryError::sqlite);
        #[cfg(test)]
        AFTER_COMMIT.with(|hook| {
            if let Some(hook) = hook.borrow_mut().take() {
                hook();
            }
        });
        committed
    })();
    if let Some((lease, lifecycle_record)) = lifecycle {
        match &ended {
            // Its row ended in the transaction.
            Ok(()) => {}
            // Nothing of this call's own is left: the row is closed, so the
            // next look begins afresh.
            Err(_) => service.discard_lifecycle(
                root,
                RepositoryOperation::SetPublicationRemote,
                lifecycle_record,
            )?,
        }
        drop(lease);
    }
    ended.map(|()| true)
}

#[cfg(test)]
thread_local! {
    /// Runs right after the record-deleting transaction commits, before
    /// anything else of the finish: a test reads what a crash there would
    /// leave.
    pub(super) static AFTER_COMMIT: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
}

/// The operation the finish's lifecycle row runs under: the setup's own ULID.
/// It is stable, so a finish that crashed between beginning its row and ending
/// it leaves a row that the next look takes up as its own, and not one that
/// every later look and every gated write finds in its way (`in_the_way`
/// matches a row by its operation ID).
pub(super) fn lifecycle_operation_id(record: &SetupRecord) -> OperationId {
    OperationId::parse(&record.setup_id.to_string())
        .expect("a setup's ID is a ULID, which an operation ID is too")
}

/// Whether the setup's remote is not the publication remote the primary's
/// configuration named before the setup: every first-time setup, and a change
/// that names a first or a different remote.
pub(super) fn on_another_remote(repository: &git2::Repository, record: &SetupRecord) -> bool {
    let Some(base) = record.base_configuration_oid else {
        return true;
    };
    let before = repository
        .find_blob(base)
        .ok()
        .and_then(|blob| std::str::from_utf8(blob.content()).ok().map(str::to_owned))
        .and_then(|text| canonical::parse_repository_config(&text).ok())
        .and_then(|configuration| configuration.publication_remote);
    before.as_deref() != Some(record.remote_name.as_str())
}

#[cfg(test)]
#[path = "complete_tests.rs"]
mod tests;
