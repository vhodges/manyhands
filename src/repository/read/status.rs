//! Status reads: one registration's index, its polling policy, and the
//! operations recorded for it.
//!
//! Each reports what is stored. None observes the repository or resumes
//! anything. The operation reads ask the system only whether the process
//! an operation names still exists, and only to say so in `live`; it
//! changes nothing they report of the operation's state.

use std::str::FromStr;

use rusqlite::{Connection, OptionalExtension, params};
use time::OffsetDateTime;

use super::{
    IndexProblemDto, IndexState, IndexStatusDto, IndexStatusState, OperationAction,
    OperationAttributionDto, OperationDto, OperationFamily, OperationListDto, OperationNextAction,
    OperationOwner, PollingOutcome, PollingStatusDto, ReadError, ResolvedRepository,
    items::{behind, effective_rows, invalid_stored_data, stored_index_state, stored_items},
};
use crate::{
    canonical::ItemId,
    repository::{
        OperationId, RemoteOperationAction, RemoteOperationPhase, RemoteOperationSafePoint,
        RemoteOutcomeCategory, RepositoryOperation, RepositoryService, SharedKeyId,
        liveness::{AttributionRow, Liveness, ProcessProbe, Worked, holder_liveness, this_host},
        promotion::lifecycle::resumable,
        recovery,
        remote::state::{self, StoredRemoteOperation},
    },
    results::{OperationFailureCode, ProblemCode, ResultCode, timestamp_string},
};

/// The longest state or step name a local operation is taken to have.
const LONGEST_STORED_NAME: usize = 64;

/// How many times the read of a blocker is tried before it is given up.
const BLOCKER_READS: usize = 3;

/// The state a local operation has once nothing remains to be done.
const LOCAL_COMPLETED: &str = "completed";

/// An operation as its store holds it, and whether the operation list
/// includes it.
struct StoredOperation {
    listed: bool,
    dto: OperationDto,
}

/// The attribution columns of an operation row, as stored.
#[derive(Default)]
struct StoredAttribution {
    request_id: Option<String>,
    command: Option<String>,
    author: Option<String>,
    host: Option<String>,
    pid: Option<i64>,
    started_at: Option<i64>,
    comment_id: Option<String>,
}

/// How a read judges whether an operation's holder still works: by the
/// host this process runs on, the process table the service was given, the
/// service's clock, which decides whether an indexing heartbeat is fresh,
/// and, for a local row of another process, whether a call holds the
/// repository's lease, looked at once per read and without waiting. One is
/// made for each read, so every row of it is judged alike.
struct Holders<'a> {
    host: String,
    probe: &'a dyn ProcessProbe,
    /// The time before which an indexing heartbeat is too old to be a
    /// call's; the rule every other reader of the heartbeat uses.
    stale_before: i64,
    data_directory: std::path::PathBuf,
    lease_held: std::cell::OnceCell<bool>,
}

impl<'a> Holders<'a> {
    fn of(service: &'a RepositoryService) -> Self {
        Self {
            host: this_host(),
            probe: &*service.process_probe,
            stale_before: service.clock.now().unix_timestamp()
                - crate::repository::INDEX_OWNER_STALE_AFTER,
            data_directory: service
                .registry_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .to_owned(),
            lease_held: std::cell::OnceCell::new(),
        }
    }

    /// The attribution a row names. `heartbeat` is whether the row shows a
    /// call at work under it by the heartbeat; `local` is the root of a
    /// local row, whose call works under that repository's lease, and
    /// `None` for a remote row, whose call works only while it holds the
    /// reservation, `holds_reservation`. A row that names neither a host
    /// nor a process is not judged.
    fn attribution(
        &self,
        stored: StoredAttribution,
        heartbeat: bool,
        local: Option<&std::path::Path>,
        holds_reservation: bool,
    ) -> Result<OperationAttributionDto, ReadError> {
        let live = (stored.host.is_some() || stored.pid.is_some()).then(|| {
            let row = AttributionRow {
                host: stored.host.clone(),
                pid: stored.pid,
                index_heartbeat_fresh: heartbeat,
            };
            let worked = match local {
                Some(root) => Worked::UnderLease {
                    lease_held: *self.lease_held.get_or_init(|| {
                        !crate::repository::coordination::leases_are_free(
                            &self.data_directory,
                            root,
                        )
                    }),
                },
                None => Worked::ByProcess { holds_reservation },
            };
            holder_liveness(&row, &self.host, std::process::id(), self.probe, worked)
                == Liveness::Live
        });
        Ok(OperationAttributionDto {
            request_id: stored.request_id,
            command: stored.command,
            author: stored.author,
            host: stored.host,
            pid: stored.pid,
            started_at: stored.started_at.map(stored_time).transpose()?,
            comment_id: stored.comment_id,
            live,
        })
    }

    /// The attribution of a remote row, which no heartbeat keeps alive, or
    /// none for an operation that has no such row. A promotion's or a
    /// closure's parent has none of its own: its reservation is the remote
    /// row of the same ID, and it is the one that names the holder. A call
    /// is at work under it only while it holds the reservation and, for a
    /// parent, only while the parent is `open`, not ended.
    fn remote(
        &self,
        connection: &Connection,
        repo: &ResolvedRepository,
        operation: &str,
        open: bool,
    ) -> Result<OperationAttributionDto, ReadError> {
        let (stored, holds_reservation) = connection
            .query_row(
                &format!(
                    "SELECT request_ulid, command, author, host, pid, started_at,
                            phase IN {}
                       FROM remote_operation_records
                      WHERE repository_id = ?1 AND operation_ulid = ?2
                      ORDER BY id DESC LIMIT 1",
                    recovery::RESERVATION_PHASES
                ),
                params![repo.registration_id(), operation],
                |row| {
                    Ok((
                        StoredAttribution {
                            request_id: row.get(0)?,
                            command: row.get(1)?,
                            author: row.get(2)?,
                            host: row.get(3)?,
                            pid: row.get(4)?,
                            started_at: row.get(5)?,
                            comment_id: None,
                        },
                        row.get::<_, bool>(6)?,
                    ))
                },
            )
            .optional()?
            .unwrap_or_default();
        self.attribution(stored, false, None, open && holds_reservation)
    }
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
pub(crate) fn local_action(stored: &str) -> Option<OperationAction> {
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
        "release_context" => Some(OperationAction::ReleaseContext),
        "retire_context" => Some(OperationAction::RetireContext),
        "save_document" => Some(OperationAction::SaveDocument),
        "save_ticket" => Some(OperationAction::SaveTicket),
        "submit_comment" => Some(OperationAction::SubmitComment),
        "repair_item" => Some(OperationAction::RepairItem),
        "context_sync_local" => Some(OperationAction::ContextSyncLocal),
        _ => None,
    }
}

/// What the target of a refresh begins with when the refresh is the local
/// half of a synchronization that had no remote to publish to.
const LOCAL_SYNCHRONIZATION_TARGET: &str = "synchronization-local-v1/";

/// Whether `text` is a full object ID as Git writes one.
fn is_object_id(text: &str) -> bool {
    text.len() == 40
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// What a local synchronization's target says was synchronized: the
/// primary branch, or one item's branch. The target is
/// `synchronization-local-v1/primary/<oid>` or
/// `synchronization-local-v1/<document|ticket>/<item>/<oid>`, as
/// synchronization writes it; `rest` is what follows the prefix. Anything
/// else under that prefix was not written by it.
fn local_synchronization(rest: &str) -> Result<(OperationAction, Option<String>), ReadError> {
    let parts: Vec<&str> = rest.split('/').collect();
    match parts[..] {
        ["primary", oid] if is_object_id(oid) => Ok((OperationAction::SynchronizePrimary, None)),
        ["document" | "ticket", item, oid]
            if is_object_id(oid) && ItemId::from_str(item).is_ok() =>
        {
            Ok((OperationAction::SynchronizeContext, Some(item.to_owned())))
        }
        _ => Err(invalid_stored_data()),
    }
}

/// The local operations recorded for the repository's root: the one with
/// this ID, whatever its state, or every one that has not completed, in the
/// order they were stored.
///
/// A record is matched by root, as recovery matches it, because one made
/// while the repository was being enabled can precede its registration.
/// Its target is read only to recognize a local synchronization, which the
/// store records as a refresh, and is never passed on.
fn local_operations(
    connection: &Connection,
    repo: &ResolvedRepository,
    holders: &Holders<'_>,
    id: Option<&str>,
) -> Result<Vec<StoredOperation>, ReadError> {
    crate::repository::local_sync::record::audit(connection)
        .map_err(|_| ReadError::new(ResultCode::RecoveryRequired))?;
    let mut statement = connection.prepare(
        "SELECT operation_ulid, action, item_id, context_path, state, completed_step,
                observed_at, target, id, request_ulid, command, author, host, pid,
                started_at, comment_id
           FROM operation_records
          WHERE root_path = ?1
            AND ((?2 IS NULL
                  AND (state != 'completed'
                       OR completed_step IS 'comment_registration_pending'))
                 OR operation_ulid = ?2)
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
        let target: Option<String> = row.get(7)?;
        if operation_id
            .as_deref()
            .is_some_and(|id| OperationId::parse(id).is_err())
            || item_id
                .as_deref()
                .is_some_and(|id| ItemId::from_str(id).is_err())
        {
            return Err(invalid_stored_data());
        }
        let heartbeat = recovery::LocalRow {
            id: row.get(8)?,
            action: action.clone(),
            target: target.clone(),
            state: state.clone(),
            step: completed_step.clone(),
            observed_at: row.get(6)?,
        }
        .has_live_heartbeat(holders.stale_before);
        let attribution = holders.attribution(
            StoredAttribution {
                request_id: row.get(9)?,
                command: row.get(10)?,
                author: row.get(11)?,
                host: row.get(12)?,
                pid: row.get(13)?,
                started_at: row.get(14)?,
                comment_id: row.get(15)?,
            },
            heartbeat,
            Some(repo.root()),
            false,
        )?;
        let action = local_action(&action).ok_or_else(invalid_stored_data)?;
        let synchronized = target
            .as_deref()
            .and_then(|target| target.strip_prefix(LOCAL_SYNCHRONIZATION_TARGET));
        let (action, item_id) = match synchronized {
            // Only a refresh carries such a target.
            Some(rest) if action == OperationAction::Refresh => local_synchronization(rest)?,
            Some(_) => return Err(invalid_stored_data()),
            None => (action, item_id),
        };
        // A comment committed with no registration is completed and
        // still owes its index hand-off.
        let listed = state != LOCAL_COMPLETED
            || recovery::waits_for_registration(&state, completed_step.as_deref());
        operations.push(StoredOperation {
            listed,
            dto: OperationDto {
                operation_id,
                family: OperationFamily::Local,
                owner: OperationOwner::Repository,
                action,
                state: stored_name(&state)?,
                completed_step: completed_step.as_deref().map(stored_name).transpose()?,
                // Recovery resumes a local operation by asking for the same
                // action again under the same ID.
                next_action: listed.then_some(OperationNextAction::Resume),
                item_id,
                key_id: None,
                worktree: row.get(3)?,
                updated_at: Some(stored_time(row.get(6)?)?),
                failure_code: None,
                attribution,
            },
        });
    }
    Ok(operations)
}

pub(crate) fn remote_action(action: RemoteOperationAction) -> OperationAction {
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
pub(crate) fn remote_phase(phase: RemoteOperationPhase) -> (&'static str, bool) {
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

pub(crate) fn remote_step(step: RemoteOperationSafePoint) -> &'static str {
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

/// Whether the operation list includes a remote operation, and what the
/// operation waits for.
///
/// - One in a phase that holds the reservation is listed. Its store does
///   not say whether anything is still running it, so nothing is offered.
/// - A synchronization that was interrupted or that failed is listed and
///   can be resumed: asked for again under its ID with a restart, it is
///   given the reservation back. A cancelled one is never given it. A
///   promotion's or closure's reservation offers nothing: its parent's own
///   row, listed beside it, offers the resume.
/// - A synchronization that completed and whose index hand-off is still
///   pending is listed and can be resumed: asked for again, it does only
///   that hand-off.
/// - One with reconciliation recorded as required is listed whatever its
///   phase, because Git may hold what it started.
///
/// A poll that ended is not listed, whatever became of it. A new poll
/// takes its place, and listing each would grow without bound.
///
/// `of_a_comment` says the synchronization is the one a comment's binding
/// keeps. It is listed as any other is and no resume is offered for it:
/// it is the comment submission's to take up, and `operation resume`
/// refuses it.
fn remote_standing(
    record: &StoredRemoteOperation,
    of_a_comment: bool,
) -> (bool, Option<OperationNextAction>) {
    let (_, holds_reservation) = remote_phase(record.phase);
    let is_synchronization = matches!(
        record.target.action(),
        RemoteOperationAction::SynchronizeContext | RemoteOperationAction::SynchronizePrimary
    );
    let stopped = matches!(
        record.phase,
        RemoteOperationPhase::Interrupted | RemoteOperationPhase::Failed
    );
    if holds_reservation {
        (true, None)
    } else if is_synchronization && (stopped || record.index_pending) {
        (true, (!of_a_comment).then_some(OperationNextAction::Resume))
    } else {
        (record.index_pending || record.reconciliation_required, None)
    }
}

fn remote_operation(
    connection: &Connection,
    repo: &ResolvedRepository,
    holders: &Holders<'_>,
    record: &StoredRemoteOperation,
) -> Result<StoredOperation, ReadError> {
    let (state, _) = remote_phase(record.phase);
    let of_a_comment =
        recovery::is_comment_synchronization(connection, &record.operation_id.to_string())?;
    let (listed, next_action) = remote_standing(record, of_a_comment);
    Ok(StoredOperation {
        listed,
        dto: OperationDto {
            operation_id: Some(record.operation_id.to_string()),
            family: OperationFamily::Remote,
            owner: OperationOwner::Repository,
            action: remote_action(record.target.action()),
            state: state.to_owned(),
            completed_step: record
                .completed_step
                .map(|step| remote_step(step).to_owned()),
            next_action,
            item_id: record.target.item().map(|(_, id)| id.to_string()),
            key_id: None,
            worktree: None,
            updated_at: Some(stored_time(record.updated_at)?),
            failure_code: record.outcome.and_then(remote_failure),
            attribution: holders.remote(
                connection,
                repo,
                &record.operation_id.to_string(),
                true,
            )?,
        },
    })
}

/// Document promotion parents: the stored step is both state and completed
/// step. Resume is offered for every parent `operation resume` would take
/// up, by the predicate it uses: one an accepted request holds too, whose
/// resume names that request (D5 decision 10). A complete parent, or one
/// that ended or a consent refusal ended, is offered nothing.
fn promotion_operations(
    connection: &Connection,
    repo: &ResolvedRepository,
    holders: &Holders<'_>,
    id: Option<&str>,
) -> Result<Vec<StoredOperation>, ReadError> {
    if !recovery::has_table(connection, "document_promotions")? {
        return Ok(Vec::new());
    }
    let records = match id {
        Some(id) => crate::repository::promotion::record::read_record(
            connection,
            repo.registration_id(),
            OperationId::parse(id).map_err(|_| invalid_stored_data())?,
        )
        .map_err(|_| invalid_stored_data())?
        .into_iter()
        .collect(),
        None => crate::repository::promotion::record::records(connection, repo.registration_id())
            .map_err(|_| invalid_stored_data())?,
    };
    records
        .into_iter()
        .filter(|r| id.is_some() || !r.step.terminal())
        .map(|r| {
            Ok(StoredOperation {
                listed: !r.step.terminal(),
                dto: OperationDto {
                    operation_id: Some(r.operation_id.to_string()),
                    family: OperationFamily::Remote,
                    owner: OperationOwner::Repository,
                    action: OperationAction::Promote,
                    state: r.step.name().to_owned(),
                    completed_step: Some(r.step.name().to_owned()),
                    next_action: resumable(&r).then_some(OperationNextAction::Resume),
                    item_id: Some(r.item_id.to_string()),
                    key_id: None,
                    worktree: None,
                    updated_at: Some(stored_time(r.updated_at)?),
                    failure_code: None,
                    attribution: holders.remote(
                        connection,
                        repo,
                        &r.operation_id.to_string(),
                        !r.step.terminal(),
                    )?,
                },
            })
        })
        .collect()
}

/// Ticket closure parents, as promotion parents: the stored step, including
/// `closing`, is both state and completed step, and resume is offered for
/// every parent left to continue, held or not.
fn closure_operations(
    connection: &Connection,
    repo: &ResolvedRepository,
    holders: &Holders<'_>,
    id: Option<&str>,
) -> Result<Vec<StoredOperation>, ReadError> {
    if !recovery::has_table(connection, "ticket_closures")? {
        return Ok(Vec::new());
    }
    let records = match id {
        Some(id) => crate::repository::closure::record::read_record(
            connection,
            repo.registration_id(),
            OperationId::parse(id).map_err(|_| invalid_stored_data())?,
        )
        .map_err(|_| invalid_stored_data())?
        .into_iter()
        .collect(),
        None => crate::repository::closure::record::records(connection, repo.registration_id())
            .map_err(|_| invalid_stored_data())?,
    };
    records
        .into_iter()
        .filter(|r| id.is_some() || !r.step.terminal())
        .map(|r| {
            Ok(StoredOperation {
                listed: !r.step.terminal(),
                dto: OperationDto {
                    operation_id: Some(r.operation_id.to_string()),
                    family: OperationFamily::Remote,
                    owner: OperationOwner::Repository,
                    action: OperationAction::Close,
                    state: r.step.name().to_owned(),
                    completed_step: Some(r.step.name().to_owned()),
                    next_action: resumable(&r).then_some(OperationNextAction::Resume),
                    item_id: Some(r.item_id.to_string()),
                    key_id: None,
                    worktree: None,
                    updated_at: Some(stored_time(r.updated_at)?),
                    failure_code: None,
                    attribution: holders.remote(
                        connection,
                        repo,
                        &r.operation_id.to_string(),
                        !r.step.terminal(),
                    )?,
                },
            })
        })
        .collect()
}

/// The registration's remote operations: the one with this ID, whatever
/// its phase, or the ones the list includes. A SQLite failure keeps its
/// meaning; a row the remote store's own checks reject is invalid.
fn remote_records(
    connection: &Connection,
    repo: &ResolvedRepository,
    id: Option<&str>,
) -> Result<Vec<StoredRemoteOperation>, ReadError> {
    state::select_status_operations(connection, repo.registration_id(), id)?
        .into_iter()
        .map(|record| record.map_err(|_| invalid_stored_data()))
        .collect()
}

fn remote_operations(
    connection: &Connection,
    repo: &ResolvedRepository,
    holders: &Holders<'_>,
    id: Option<&str>,
) -> Result<Vec<StoredOperation>, ReadError> {
    let has_promotions = recovery::has_table(connection, "document_promotions")?;
    let has_closures = recovery::has_table(connection, "ticket_closures")?;
    let mut operations = Vec::new();
    for record in remote_records(connection, repo, id)? {
        // The parent's stored step is authoritative for promotion and
        // closure status; do not show the held reservation as a duplicate
        // operation.
        let parent = |exists: bool, sql: &str| -> Result<bool, ReadError> {
            Ok(exists
                && connection.query_row(sql, [record.operation_id.to_string()], |r| {
                    r.get::<_, bool>(0)
                })?)
        };
        if parent(
            has_promotions,
            "SELECT EXISTS(SELECT 1 FROM document_promotions WHERE operation_id=?1)",
        )? || parent(
            has_closures,
            "SELECT EXISTS(SELECT 1 FROM ticket_closures WHERE operation_id=?1)",
        )? {
            continue;
        }
        let operation = remote_operation(connection, repo, holders, &record)?;
        if id.is_some() || operation.listed {
            operations.push(operation);
        }
    }
    Ok(operations)
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
/// failed, in the order they were stored. Of the key only its ID is read:
/// neither its paths nor its label.
fn key_material_operations(
    connection: &Connection,
    id: Option<&str>,
) -> Result<Vec<StoredOperation>, ReadError> {
    let mut statement = connection.prepare(
        "SELECT operation_id, action, phase, failure_code, key_id
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
        let key_id: String = row.get(4)?;
        if OperationId::parse(&operation_id).is_err() || SharedKeyId::parse(&key_id).is_err() {
            return Err(invalid_stored_data());
        }
        operations.push(StoredOperation {
            listed: state != "completed" || failure.is_some(),
            dto: OperationDto {
                operation_id: Some(operation_id),
                family: OperationFamily::KeyMaterial,
                owner: OperationOwner::Application,
                action,
                state: state.to_owned(),
                completed_step: None,
                next_action: key_material_next_action(action, state, failure.is_some()),
                item_id: None,
                key_id: Some(key_id),
                worktree: None,
                updated_at: None,
                failure_code: failure
                    .as_deref()
                    .map(OperationFailureCode::from_stored_key_material),
                attribution: OperationAttributionDto::default(),
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

/// The unfinished operations of one repository: its local rows, then its
/// remote synchronizations and polls, promotions and closures, in the order
/// each store gives them. `registered` is whether the repository has a
/// registration; without one only the local rows, which recovery matches by
/// root, exist.
fn repository_operations(
    connection: &Connection,
    repo: &ResolvedRepository,
    holders: &Holders<'_>,
    registered: bool,
) -> Result<Vec<StoredOperation>, ReadError> {
    let mut operations = local_operations(connection, repo, holders, None)?;
    if registered {
        operations.extend(remote_operations(connection, repo, holders, None)?);
        operations.extend(promotion_operations(connection, repo, holders, None)?);
        operations.extend(closure_operations(connection, repo, holders, None)?);
    }
    Ok(operations)
}

/// Orders operations as the operation list does: by operation ID, which is
/// the order they were started in. The sort is stable, so operations with
/// no ID keep the order they were stored in.
fn sort_operations(items: &mut [OperationDto]) {
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
            // A problem about the working tree itself is stored with a path
            // that is empty once made relative to it.
            path: row
                .get::<_, Option<String>>(1)?
                .filter(|path| !path.is_empty()),
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
            let pending_operations = local_operations(connection, repo, &Holders::of(self), None)?
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
    /// Nothing is contacted and no process is looked for. The stored next
    /// eligibility deadline is returned without clamping or otherwise changing it.
    pub fn polling_status(&self, repo: &ResolvedRepository) -> Result<PollingStatusDto, ReadError> {
        self.read_session(RepositoryOperation::Read, |connection| {
            require_registration(connection, repo)?;
            let registration = repo.registration_id();
            // Only the policy row and the current batch's time are read, so
            // an observation this read does not report cannot fail it.
            let (polling, latest_outcome, next_eligible_at) =
                state::select_polling_status(connection, registration)?
                    .ok_or_else(invalid_stored_data)?
                    .map_err(|_| invalid_stored_data())?;
            let observed_at: Option<i64> = connection
                .query_row(
                    "SELECT observed_at FROM remote_observation_batches
                      WHERE repository_id = ?1 AND is_current = 1",
                    [registration],
                    |row| row.get(0),
                )
                .optional()?;
            let active = remote_records(connection, repo, None)?
                .into_iter()
                .find(|record| remote_phase(record.phase).1);
            Ok(PollingStatusDto {
                enabled: polling.enabled(),
                paused: polling.paused(),
                interval_seconds: polling.interval().as_secs(),
                backoff_seconds: polling.automatic_backoff().map(|delay| delay.as_secs()),
                recovery_suspended: polling.recovery_suspended(),
                latest_outcome: latest_outcome.map(polling_outcome),
                latest_observed_at: observed_at.map(stored_time).transpose()?,
                active_operation_id: active.map(|operation| operation.operation_id.to_string()),
                next_eligible_at: next_eligible_at
                    .map(|time| stored_time(time.unix_timestamp()))
                    .transpose()?,
            })
        })
        .map_err(|error| repo.failure(error))
    }

    /// The operations with work outstanding: the repository's local
    /// operations that have not completed; its remote operation that holds
    /// the reservation, its synchronizations that can be resumed and any
    /// remote operation with index or reconciliation work recorded; and
    /// the application's key-material operations that have not completed
    /// or that failed. A poll that ended is never listed, so the list does
    /// not grow with the number of polls.
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
            let holders = Holders::of(self);
            let mut items: Vec<OperationDto> =
                repository_operations(connection, repo, &holders, true)?
                    .into_iter()
                    .chain(key_material_operations(connection, None)?)
                    .map(|operation| operation.dto)
                    .collect();
            sort_operations(&mut items);
            Ok(OperationListDto {
                items,
                complete: true,
            })
        })
        .map_err(|error| repo.failure(error))
    }

    /// The unfinished operations of the repository at `root`, for
    /// `inspect_repository`: those `list_operations` lists for it, in its
    /// order and with its attribution, judged as it judges them, less the
    /// application's key-material operations, which are no repository's.
    ///
    /// `registration` is the root's registration, `None` for a root that
    /// has none: it has local rows only, matched by root as recovery
    /// matches them. It reads as any read does, on a read-only connection,
    /// so it leaves no write-ahead file behind.
    pub(super) fn inspected_operations(
        &self,
        root: &std::path::Path,
        registration: Option<i64>,
    ) -> Result<Vec<OperationDto>, ReadError> {
        self.read_session(RepositoryOperation::Read, |connection| {
            let repo =
                ResolvedRepository::registered(registration.unwrap_or_default(), root.to_owned());
            let holders = Holders::of(self);
            let mut items: Vec<OperationDto> =
                repository_operations(connection, &repo, &holders, registration.is_some())?
                    .into_iter()
                    .map(|operation| operation.dto)
                    .collect();
            sort_operations(&mut items);
            Ok(items)
        })
    }

    /// The first unfinished operation of the repository at `root` whose
    /// holder still works, as `list_operations` judges it, for a writer
    /// that could not take a lease or the index's lock in time: that is
    /// what such a refusal is for while one is at work. `Nothing` when the
    /// repository has none, `Unreadable` when it cannot be read in
    /// `BLOCKER_READS` tries. The repository is the one registered at
    /// `root`; a root with no registration has local rows only.
    pub(crate) fn live_holder(&self, root: &std::path::Path) -> crate::repository::LiveHolder {
        use crate::repository::LiveHolder;
        (0..BLOCKER_READS)
            .find_map(|_| self.read_live_holder(root).ok())
            .map_or(LiveHolder::Unreadable, |found| {
                found.map_or(LiveHolder::Nothing, LiveHolder::Live)
            })
    }

    /// One try of `live_holder`.
    fn read_live_holder(
        &self,
        root: &std::path::Path,
    ) -> Result<Option<crate::repository::Blocker>, ReadError> {
        let Some(root_text) = root.to_str() else {
            return Ok(None);
        };
        self.writer_read_session(RepositoryOperation::Read, |connection| {
            let registration: Option<i64> = connection
                .query_row(
                    "SELECT id FROM repositories WHERE root_path = ?1",
                    [root_text],
                    |row| row.get(0),
                )
                .optional()?;
            let repo =
                ResolvedRepository::registered(registration.unwrap_or_default(), root.to_owned());
            let holders = Holders::of(self);
            let mut items: Vec<OperationDto> =
                repository_operations(connection, &repo, &holders, registration.is_some())?
                    .into_iter()
                    .filter(|operation| operation.listed)
                    .map(|operation| operation.dto)
                    .collect();
            sort_operations(&mut items);
            Ok(items.into_iter().find_map(|operation| {
                if operation.attribution.live != Some(true) {
                    return None;
                }
                let operation_id = OperationId::parse(operation.operation_id.as_deref()?).ok()?;
                Some(crate::repository::Blocker {
                    family: operation.family,
                    operation_id,
                    live: Some(true),
                    seen: Some(Box::new(operation)),
                })
            }))
        })
    }

    /// The operation `blocker` names, in its own journal, as `show_operation`
    /// reads it, and whether the list includes it: whether it is still
    /// unfinished. `Unreadable` when it cannot be read in `BLOCKER_READS`
    /// tries: the read waits a moment at most for the index's guard, and the
    /// writer the blocker names may hold it then. The repository is the one
    /// registered at `root`, the root its rows name; a root with no
    /// registration has local rows only.
    pub(crate) fn blocking_operation(
        &self,
        root: &std::path::Path,
        blocker: &crate::repository::Blocker,
    ) -> crate::repository::BlockerRow {
        use crate::repository::BlockerRow;
        (0..BLOCKER_READS)
            .find_map(|_| self.read_blocking_operation(root, blocker).ok())
            .map_or(BlockerRow::Unreadable, |found| match found {
                Some((operation, unfinished)) => BlockerRow::Found {
                    operation: Box::new(operation),
                    unfinished,
                },
                None => BlockerRow::Gone,
            })
    }

    /// One try of `blocking_operation`.
    fn read_blocking_operation(
        &self,
        root: &std::path::Path,
        blocker: &crate::repository::Blocker,
    ) -> Result<Option<(OperationDto, bool)>, ReadError> {
        let Some(root_text) = root.to_str() else {
            return Ok(None);
        };
        self.writer_read_session(RepositoryOperation::Read, |connection| {
            let registration: Option<i64> = connection
                .query_row(
                    "SELECT id FROM repositories WHERE root_path = ?1",
                    [root_text],
                    |row| row.get(0),
                )
                .optional()?;
            let text = blocker.operation_id.to_string();
            let holders = Holders::of(self);
            let found = match (blocker.family, registration) {
                (OperationFamily::Local, registration) => {
                    let repo = ResolvedRepository::registered(
                        registration.unwrap_or_default(),
                        root.to_owned(),
                    );
                    local_operations(connection, &repo, &holders, Some(&text))?
                }
                (OperationFamily::Remote, Some(registration)) => {
                    let repo = ResolvedRepository::registered(registration, root.to_owned());
                    let mut found = remote_operations(connection, &repo, &holders, Some(&text))?;
                    found.extend(promotion_operations(
                        connection,
                        &repo,
                        &holders,
                        Some(&text),
                    )?);
                    found.extend(closure_operations(
                        connection,
                        &repo,
                        &holders,
                        Some(&text),
                    )?);
                    found
                }
                (OperationFamily::Remote, None) | (OperationFamily::KeyMaterial, _) => Vec::new(),
            };
            Ok(found
                .into_iter()
                .next()
                .map(|operation| (operation.dto, operation.listed)))
        })
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
            let holders = Holders::of(self);
            local_operations(connection, repo, &holders, Some(&text))?
                .into_iter()
                .chain(remote_operations(connection, repo, &holders, Some(&text))?)
                .chain(promotion_operations(
                    connection,
                    repo,
                    &holders,
                    Some(&text),
                )?)
                .chain(closure_operations(connection, repo, &holders, Some(&text))?)
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
