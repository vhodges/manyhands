//! Status reads: one registration's index, its polling policy, and the
//! operations recorded for it.
//!
//! Each reports what is stored. None observes the repository, asks whether
//! a process is running, or resumes anything.

use std::str::FromStr;

use rusqlite::{Connection, OptionalExtension, params};
use time::OffsetDateTime;

use super::{
    IndexProblemDto, IndexState, IndexStatusDto, IndexStatusState, OperationAction, OperationDto,
    OperationFamily, OperationListDto, OperationNextAction, OperationScope, PollingOutcome,
    PollingStatusDto, ReadError, ResolvedRepository,
    items::{behind, effective_rows, invalid_stored_data, stored_index_state, stored_items},
};
use crate::{
    canonical::ItemId,
    repository::{
        OperationId, RemoteOperationAction, RemoteOperationPhase, RemoteOperationSafePoint,
        RemoteOutcomeCategory, RepositoryOperation, RepositoryService,
        remote::state::{self, StoredRemoteOperation},
    },
    results::{OperationFailureCode, ProblemCode, ResultCode, timestamp_string},
};

/// The longest state or step name a local operation is taken to have.
const LONGEST_STORED_NAME: usize = 64;

/// The state a local operation has once nothing remains to be done.
const LOCAL_COMPLETED: &str = "completed";

/// An operation as its store holds it, and whether the operation list
/// includes it.
struct StoredOperation {
    listed: bool,
    dto: OperationDto,
}

/// A time a store recorded, in seconds. One that cannot be written as a
/// timestamp was not written by any store.
fn stored_time(seconds: i64) -> Result<String, ReadError> {
    OffsetDateTime::from_unix_timestamp(seconds)
        .ok()
        .and_then(timestamp_string)
        .ok_or_else(invalid_stored_data)
}

/// A state or step name of a local operation. The store keeps these as
/// free text, so only what has the form of a name is passed on: a
/// lowercase letter followed by lowercase letters, digits and underscores.
fn stored_name(stored: &str) -> Result<String, ReadError> {
    let is_name = stored.len() <= LONGEST_STORED_NAME
        && stored.starts_with(|first: char| first.is_ascii_lowercase())
        && stored
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
    if is_name {
        Ok(stored.to_owned())
    } else {
        Err(invalid_stored_data())
    }
}

/// The action a local operation record names. The names are the ones the
/// recovery store writes; the contract strings happen to equal them.
fn local_action(stored: &str) -> Option<OperationAction> {
    match stored {
        "create_and_enable" => Some(OperationAction::CreateAndEnable),
        "enable" => Some(OperationAction::Enable),
        "remove_registration" => Some(OperationAction::RemoveRegistration),
        "add_remote" => Some(OperationAction::AddRemote),
        "remove_remote" => Some(OperationAction::RemoveRemote),
        "set_publication_remote" => Some(OperationAction::SetPublicationRemote),
        "refresh" => Some(OperationAction::Refresh),
        "rebuild" => Some(OperationAction::Rebuild),
        "prepare_context" => Some(OperationAction::PrepareContext),
        "save_document" => Some(OperationAction::SaveDocument),
        "save_ticket" => Some(OperationAction::SaveTicket),
        "submit_comment" => Some(OperationAction::SubmitComment),
        _ => None,
    }
}

/// The local operations recorded for the repository's root: the one with
/// this ID, whatever its state, or every one that has not completed, in the
/// order they were stored.
///
/// A record is matched by root, as recovery matches it, because one made
/// while the repository was being enabled can precede its registration.
fn local_operations(
    connection: &Connection,
    repo: &ResolvedRepository,
    id: Option<&str>,
) -> Result<Vec<StoredOperation>, ReadError> {
    let mut statement = connection.prepare(
        "SELECT operation_ulid, action, item_id, context_path, state, completed_step,
                observed_at
           FROM operation_records
          WHERE root_path = ?1
            AND ((?2 IS NULL AND state != 'completed') OR operation_ulid = ?2)
          ORDER BY id ASC",
    )?;
    let mut rows = statement.query(params![repo.root().to_str(), id])?;
    let mut operations = Vec::new();
    while let Some(row) = rows.next()? {
        let operation_id: Option<String> = row.get(0)?;
        let action: String = row.get(1)?;
        let item_id: Option<String> = row.get(2)?;
        let state: String = row.get(4)?;
        let completed_step: Option<String> = row.get(5)?;
        if operation_id
            .as_deref()
            .is_some_and(|id| OperationId::parse(id).is_err())
            || item_id
                .as_deref()
                .is_some_and(|id| ItemId::from_str(id).is_err())
        {
            return Err(invalid_stored_data());
        }
        let listed = state != LOCAL_COMPLETED;
        operations.push(StoredOperation {
            listed,
            dto: OperationDto {
                operation_id,
                family: OperationFamily::Local,
                scope: OperationScope::Repository,
                action: local_action(&action).ok_or_else(invalid_stored_data)?,
                state: stored_name(&state)?,
                completed_step: completed_step.as_deref().map(stored_name).transpose()?,
                // Recovery resumes a local operation by asking for the same
                // action again under the same ID.
                next_action: listed.then_some(OperationNextAction::Resume),
                item_id,
                context: row.get(3)?,
                updated_at: Some(stored_time(row.get(6)?)?),
                failure_code: None,
            },
        });
    }
    Ok(operations)
}

fn remote_action(action: RemoteOperationAction) -> OperationAction {
    match action {
        RemoteOperationAction::Poll => OperationAction::Poll,
        RemoteOperationAction::SynchronizeContext => OperationAction::SynchronizeContext,
        RemoteOperationAction::SynchronizePrimary => OperationAction::SynchronizePrimary,
        RemoteOperationAction::Promote => OperationAction::Promote,
        RemoteOperationAction::Close => OperationAction::Close,
    }
}

/// The contract name of a remote operation's phase, and whether an
/// operation in that phase holds the registration's reservation.
fn remote_phase(phase: RemoteOperationPhase) -> (&'static str, bool) {
    match phase {
        RemoteOperationPhase::Reserved => ("reserved", true),
        RemoteOperationPhase::Advertising => ("advertising", true),
        RemoteOperationPhase::Persisting => ("persisting", true),
        RemoteOperationPhase::FetchPrepared => ("fetch_prepared", true),
        RemoteOperationPhase::FetchObserved => ("fetch_observed", true),
        RemoteOperationPhase::LocalPrepared => ("local_prepared", true),
        RemoteOperationPhase::LocalFastForwarded => ("local_fast_forwarded", true),
        RemoteOperationPhase::PushPrepared => ("push_prepared", true),
        RemoteOperationPhase::PushReturned => ("push_returned", true),
        RemoteOperationPhase::PushVerified => ("push_verified", true),
        RemoteOperationPhase::Reconciling => ("reconciling", true),
        RemoteOperationPhase::Completed => ("completed", false),
        RemoteOperationPhase::Interrupted => ("interrupted", false),
        RemoteOperationPhase::Cancelled => ("cancelled", false),
        RemoteOperationPhase::Failed => ("failed", false),
    }
}

fn remote_step(step: RemoteOperationSafePoint) -> &'static str {
    match step {
        RemoteOperationSafePoint::BeforeTransport => "before_transport",
        RemoteOperationSafePoint::AfterAdvertisement => "after_advertisement",
        RemoteOperationSafePoint::BetweenObservations => "between_observations",
        RemoteOperationSafePoint::BeforeBatchCommit => "before_batch_commit",
        RemoteOperationSafePoint::AfterBatchCommit => "after_batch_commit",
        RemoteOperationSafePoint::BeforeLocalMutation => "before_local_mutation",
        RemoteOperationSafePoint::BeforeFetch => "before_fetch",
        RemoteOperationSafePoint::AfterFetch => "after_fetch",
        RemoteOperationSafePoint::BeforeLocalUpdate => "before_local_update",
        RemoteOperationSafePoint::AfterLocalUpdate => "after_local_update",
        RemoteOperationSafePoint::BeforePush => "before_push",
        RemoteOperationSafePoint::AfterPushReturn => "after_push_return",
        RemoteOperationSafePoint::AfterPushVerification => "after_push_verification",
        RemoteOperationSafePoint::BeforeDiscovery => "before_discovery",
    }
}

fn polling_outcome(outcome: RemoteOutcomeCategory) -> PollingOutcome {
    match outcome {
        RemoteOutcomeCategory::Completed => PollingOutcome::Completed,
        RemoteOutcomeCategory::ConfigurationRequired => PollingOutcome::ConfigurationRequired,
        RemoteOutcomeCategory::SelectedKeyUnavailable => PollingOutcome::SelectedKeyUnavailable,
        RemoteOutcomeCategory::UnlockRequired => PollingOutcome::UnlockRequired,
        RemoteOutcomeCategory::HostApprovalRequired => PollingOutcome::HostApprovalRequired,
        RemoteOutcomeCategory::TransportUnavailable => PollingOutcome::TransportUnavailable,
        RemoteOutcomeCategory::ProtocolRejected => PollingOutcome::ProtocolRejected,
        RemoteOutcomeCategory::Cancelled => PollingOutcome::Cancelled,
        RemoteOutcomeCategory::RepositoryUnavailable => PollingOutcome::RepositoryUnavailable,
    }
}

/// Why a remote operation did not complete, or `None` when it did.
fn remote_failure(outcome: RemoteOutcomeCategory) -> Option<OperationFailureCode> {
    match outcome {
        RemoteOutcomeCategory::Completed => None,
        RemoteOutcomeCategory::ConfigurationRequired => {
            Some(OperationFailureCode::ConfigurationRequired)
        }
        RemoteOutcomeCategory::SelectedKeyUnavailable => {
            Some(OperationFailureCode::SelectedKeyUnavailable)
        }
        RemoteOutcomeCategory::UnlockRequired => Some(OperationFailureCode::UnlockRequired),
        RemoteOutcomeCategory::HostApprovalRequired => {
            Some(OperationFailureCode::HostApprovalRequired)
        }
        RemoteOutcomeCategory::TransportUnavailable => {
            Some(OperationFailureCode::TransportUnavailable)
        }
        RemoteOutcomeCategory::ProtocolRejected => Some(OperationFailureCode::ProtocolRejected),
        RemoteOutcomeCategory::Cancelled => Some(OperationFailureCode::Cancelled),
        RemoteOutcomeCategory::RepositoryUnavailable => {
            Some(OperationFailureCode::RepositoryUnavailable)
        }
    }
}

fn remote_operation(record: &StoredRemoteOperation) -> Result<StoredOperation, ReadError> {
    let (state, listed) = remote_phase(record.phase);
    Ok(StoredOperation {
        listed,
        dto: OperationDto {
            operation_id: Some(record.operation_id.to_string()),
            family: OperationFamily::Remote,
            scope: OperationScope::Repository,
            action: remote_action(record.target.action()),
            state: state.to_owned(),
            completed_step: record
                .completed_step
                .map(|step| remote_step(step).to_owned()),
            next_action: None,
            item_id: record.target.item().map(|(_, id)| id.to_string()),
            context: None,
            updated_at: Some(stored_time(record.updated_at)?),
            failure_code: record.outcome.and_then(remote_failure),
        },
    })
}

/// The registration's remote operations: the one with this ID, whatever
/// its phase, or the one that holds the reservation. The remote store
/// checks every value of a row it returns.
fn remote_operations(
    connection: &Connection,
    repo: &ResolvedRepository,
    id: Option<OperationId>,
) -> Result<Vec<StoredOperation>, ReadError> {
    let records = match id {
        Some(id) => state::read_operation(connection, repo.registration_id(), id)?
            .into_iter()
            .collect(),
        None => state::read_operation_rows(connection, repo.registration_id(), true)?,
    };
    records.iter().map(remote_operation).collect()
}

/// A key-material phase under its contract name, or `None` for a phase
/// the action cannot be in.
fn key_material_state(action: OperationAction, phase: &str) -> Option<&'static str> {
    match (action, phase) {
        (_, "completed") => Some("completed"),
        (_, "retained-for-inspection") => Some("retained_for_inspection"),
        (OperationAction::GenerateKey, "reserved") => Some("reserved"),
        (OperationAction::GenerateKey, "private-written") => Some("private_written"),
        (OperationAction::GenerateKey, "pair-written") => Some("pair_written"),
        (OperationAction::DeleteKey, "prepared") => Some("prepared"),
        (OperationAction::DeleteKey, "private-removed") => Some("private_removed"),
        (OperationAction::DeleteKey, "files-removed") => Some("files_removed"),
        _ => None,
    }
}

/// What key-material recovery offers for an operation, as
/// `list_key_material_recovery` decides it; nothing for an operation that
/// completed without a failure, which that list leaves out.
fn key_material_next_action(
    action: OperationAction,
    state: &str,
    failed: bool,
) -> Option<OperationNextAction> {
    match (action, state) {
        (_, "completed") if !failed => None,
        (_, "completed" | "retained_for_inspection") => {
            Some(OperationNextAction::InspectRetainedFiles)
        }
        (OperationAction::GenerateKey, _) => Some(OperationNextAction::RetryGeneration),
        _ => Some(OperationNextAction::ReviewDeletionAgain),
    }
}

/// The application's key-material operations: the one with this ID,
/// whatever its phase, or every one that has not completed or that
/// failed, in the order they were stored. Neither the key's paths nor its
/// label is read.
fn key_material_operations(
    connection: &Connection,
    id: Option<&str>,
) -> Result<Vec<StoredOperation>, ReadError> {
    let mut statement = connection.prepare(
        "SELECT operation_id, action, phase, failure_code
           FROM key_material_operations
          WHERE (?1 IS NULL AND (phase <> 'completed' OR failure_code IS NOT NULL))
             OR operation_id = ?1
          ORDER BY rowid ASC",
    )?;
    let mut rows = statement.query([id])?;
    let mut operations = Vec::new();
    while let Some(row) = rows.next()? {
        let operation_id: String = row.get(0)?;
        let action: String = row.get(1)?;
        let phase: String = row.get(2)?;
        let failure: Option<String> = row.get(3)?;
        let action = match action.as_str() {
            "generate" => OperationAction::GenerateKey,
            "delete" => OperationAction::DeleteKey,
            _ => return Err(invalid_stored_data()),
        };
        let state = key_material_state(action, &phase).ok_or_else(invalid_stored_data)?;
        if OperationId::parse(&operation_id).is_err() {
            return Err(invalid_stored_data());
        }
        operations.push(StoredOperation {
            listed: state != "completed" || failure.is_some(),
            dto: OperationDto {
                operation_id: Some(operation_id),
                family: OperationFamily::KeyMaterial,
                scope: OperationScope::Application,
                action,
                state: state.to_owned(),
                completed_step: None,
                next_action: key_material_next_action(action, state, failure.is_some()),
                item_id: None,
                context: None,
                updated_at: None,
                failure_code: failure
                    .as_deref()
                    .map(OperationFailureCode::from_stored_key_material),
            },
        });
    }
    Ok(operations)
}

/// The order of the three stores where one operation ID is in more than
/// one of them: a synchronization and the refresh that follows it share
/// theirs.
fn family_rank(family: OperationFamily) -> u8 {
    match family {
        OperationFamily::Local => 0,
        OperationFamily::Remote => 1,
        OperationFamily::KeyMaterial => 2,
    }
}

/// Every problem the index stores for the registration, with the working
/// tree it was found in. The stored guidance is not read: it can hold a
/// parser's or the operating system's own words.
fn index_problems(
    connection: &Connection,
    repo: &ResolvedRepository,
) -> Result<Vec<IndexProblemDto>, ReadError> {
    let mut statement = connection.prepare(
        "SELECT problems.code, problems.path, contexts.worktree_path
           FROM problems
           LEFT JOIN contexts ON contexts.id = problems.context_id
          WHERE problems.repository_id = ?1
          ORDER BY contexts.worktree_path ASC, problems.path ASC, problems.code ASC,
                   problems.id ASC",
    )?;
    let mut rows = statement.query([repo.registration_id()])?;
    let mut problems = Vec::new();
    while let Some(row) = rows.next()? {
        let code: String = row.get(0)?;
        problems.push(IndexProblemDto {
            code: ProblemCode::from_stored(&code),
            path: row.get(1)?,
            worktree: row.get(2)?,
        });
    }
    Ok(problems)
}

/// Fails with `repository_not_registered` when the registration was removed
/// after the repository was resolved, so that its absence is not read as
/// having no operations and the default polling policy.
fn require_registration(
    connection: &Connection,
    repo: &ResolvedRepository,
) -> Result<(), ReadError> {
    stored_index_state(connection, repo).map(drop)
}

fn count(value: usize) -> Option<u64> {
    u64::try_from(value).ok()
}

impl RepositoryService {
    /// How far the registration's index is behind, how much it holds, the
    /// problems it stored and the local operations that have not yet
    /// brought it up to date.
    ///
    /// Unlike every other read, this one succeeds when the index cannot be
    /// read: `state` is then `unavailable`. That can only be asked of a
    /// repository resolved while the index could still be read.
    pub fn index_status(&self, repo: &ResolvedRepository) -> Result<IndexStatusDto, ReadError> {
        let status = self.read_session(RepositoryOperation::Read, |connection| {
            let (index, _) = stored_index_state(connection, repo)?;
            let stored = stored_items(connection, repo)?;
            let (items, is_behind) = effective_rows(repo, &stored);
            let index = if is_behind { behind(&index) } else { index };
            let contexts: i64 = connection.query_row(
                "SELECT COUNT(*) FROM contexts WHERE repository_id = ?1",
                [repo.registration_id()],
                |row| row.get(0),
            )?;
            let problems = index_problems(connection, repo)?;
            let pending_operations = local_operations(connection, repo, None)?
                .into_iter()
                .map(|operation| operation.dto)
                .collect();
            Ok(IndexStatusDto {
                state: match index.state {
                    IndexState::Current => IndexStatusState::Current,
                    IndexState::Stale => IndexStatusState::Stale,
                    IndexState::NeverRefreshed => IndexStatusState::NeverRefreshed,
                },
                refreshed_at: index.refreshed_at,
                context_count: u64::try_from(contexts).ok(),
                item_count: count(items.len()),
                problem_count: count(problems.len()),
                problems,
                pending_operations,
            })
        });
        match status {
            Err(error) if error.code() == ResultCode::IndexUnavailable => Ok(IndexStatusDto {
                state: IndexStatusState::Unavailable,
                refreshed_at: None,
                context_count: None,
                item_count: None,
                problem_count: None,
                problems: Vec::new(),
                pending_operations: Vec::new(),
            }),
            status => status.map_err(|error| repo.failure(error)),
        }
    }

    /// The registration's stored polling policy, how the latest attempt to
    /// observe its remote ended, and the remote operation under way.
    ///
    /// Nothing is contacted and no process is looked for. When the next
    /// attempt is due is not stored, so `next_eligible_at` is null.
    pub fn polling_status(&self, repo: &ResolvedRepository) -> Result<PollingStatusDto, ReadError> {
        self.read_session(RepositoryOperation::Read, |connection| {
            require_registration(connection, repo)?;
            let registration = repo.registration_id();
            let snapshot = state::read_snapshot(connection, registration)?;
            let polling = snapshot.polling();
            let observed_at: Option<i64> = connection
                .query_row(
                    "SELECT observed_at FROM remote_observation_batches
                      WHERE repository_id = ?1 AND is_current = 1",
                    [registration],
                    |row| row.get(0),
                )
                .optional()?;
            let active = state::read_operation_rows(connection, registration, true)?;
            Ok(PollingStatusDto {
                enabled: polling.enabled(),
                paused: polling.paused(),
                interval_seconds: polling.interval().as_secs(),
                backoff_seconds: polling.automatic_backoff().map(|delay| delay.as_secs()),
                recovery_suspended: polling.recovery_suspended(),
                latest_outcome: snapshot.latest_outcome().map(polling_outcome),
                latest_observed_at: observed_at.map(stored_time).transpose()?,
                active_operation_id: active
                    .first()
                    .map(|operation| operation.operation_id.to_string()),
                next_eligible_at: None,
            })
        })
        .map_err(|error| repo.failure(error))
    }

    /// The operations that have not finished: the repository's local
    /// operations that have not completed, the remote operation that holds
    /// its reservation, and the application's key-material operations that
    /// have not completed or that failed.
    ///
    /// They are ordered by operation ID, which is the order they were
    /// started in. A local operation recorded before operations had IDs
    /// follows the rest, in the order it was stored.
    pub fn list_operations(
        &self,
        repo: &ResolvedRepository,
    ) -> Result<OperationListDto, ReadError> {
        self.read_session(RepositoryOperation::Read, |connection| {
            require_registration(connection, repo)?;
            let mut items: Vec<OperationDto> = local_operations(connection, repo, None)?
                .into_iter()
                .chain(remote_operations(connection, repo, None)?)
                .chain(key_material_operations(connection, None)?)
                .map(|operation| operation.dto)
                .collect();
            // The sort is stable, so operations with no ID keep the order
            // they were stored in.
            items.sort_by(|left, right| {
                (
                    left.operation_id.is_none(),
                    &left.operation_id,
                    family_rank(left.family),
                )
                    .cmp(&(
                        right.operation_id.is_none(),
                        &right.operation_id,
                        family_rank(right.family),
                    ))
            });
            Ok(OperationListDto {
                items,
                complete: true,
            })
        })
        .map_err(|error| repo.failure(error))
    }

    /// The operation with this ID, whether or not it has finished: one of
    /// the repository's local or remote operations, or one of the
    /// application's key-material operations. No other ID is
    /// `operation_not_found`.
    ///
    /// Where more than one store holds the ID, an operation the list
    /// includes is returned before one it does not, and then a local
    /// operation before a remote one.
    pub fn show_operation(
        &self,
        repo: &ResolvedRepository,
        id: OperationId,
    ) -> Result<OperationDto, ReadError> {
        self.read_session(RepositoryOperation::Read, |connection| {
            require_registration(connection, repo)?;
            let text = id.to_string();
            local_operations(connection, repo, Some(&text))?
                .into_iter()
                .chain(remote_operations(connection, repo, Some(id))?)
                .chain(key_material_operations(connection, Some(&text))?)
                .min_by_key(|operation| (!operation.listed, family_rank(operation.dto.family)))
                .map(|operation| operation.dto)
                .ok_or_else(|| ReadError::new(ResultCode::OperationNotFound))
        })
        .map_err(|error| repo.failure(error))
    }
}

#[cfg(test)]
#[path = "status_tests.rs"]
mod tests;
