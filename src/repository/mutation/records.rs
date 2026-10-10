//! The three request tables: their definitions and every read and write
//! of them.
//!
//! A request record links a request ID to its command, target, input
//! digest, operations and, once finished, its result. A confirmation
//! record is a preview a later request may present as its consent. None of
//! them holds content: a digest, identifiers, states and the finished
//! envelope's data, never a Markdown body, a source, a passphrase, a key
//! or text a server sent.
//!
//! Each function here is one short transaction under the index lock, which
//! it releases before it returns: a domain call takes the same lock itself.
//! A failure is returned as a `ReadError`, whose code says whether the
//! index was busy, unavailable or held something unreadable.

// `ReadError` carries its whole scope by value, as the result contract has it.
#![allow(clippy::result_large_err)]

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};

use super::{
    dto::RequestState,
    identity::{ConfirmationId, IntentDigest, RequestId, ScopeKey},
};
use crate::{
    repository::{
        ExpectedPathObservation, OperationFamily, OperationId, ReadError, RepositoryError,
        RepositoryOperation, RepositoryService, cache_write_guard, migrate_registry, open_registry,
    },
    results::{
        CheckpointEffect, CleanupEffect, DiscoveryEffect, Effects, IntegrationEffect, Outcome,
        PublicationEffect, ResultCode, WriteEffect,
    },
};

/// The request tables, each by its name and the statement that creates it,
/// in an order in which they can be created.
///
/// `scope_key` is a path string and no reference to `repositories`: a
/// request to create a repository precedes its registration, and a request
/// to remove one outlives it. Times are seconds since the epoch, read from
/// the service's clock. A request record holds its result exactly when it
/// is `finished`.
const REQUEST_TABLES: [(&str, &str); 3] = [
    (
        "request_records",
        "CREATE TABLE request_records (
            request_ulid TEXT PRIMARY KEY NOT NULL,
            attempt INTEGER NOT NULL CHECK (attempt >= 1),
            scope_key TEXT NOT NULL,
            command TEXT NOT NULL,
            target TEXT NOT NULL,
            intent_digest TEXT NOT NULL CHECK (
                length(intent_digest) = 64 AND intent_digest NOT GLOB '*[^0-9a-f]*'
            ),
            confirmation_ulid TEXT,
            base_ref TEXT,
            base_oid TEXT CHECK (
                base_oid IS NULL
                OR (length(base_oid) = 40 AND base_oid NOT GLOB '*[^0-9a-f]*')
            ),
            expected_digest TEXT CHECK (
                expected_digest IS NULL
                OR expected_digest = 'missing'
                OR (length(expected_digest) = 64 AND expected_digest NOT GLOB '*[^0-9a-f]*')
            ),
            cancel_requested INTEGER NOT NULL DEFAULT 0 CHECK (cancel_requested IN (0, 1)),
            state TEXT NOT NULL CHECK (state IN ('accepted', 'finished')),
            outcome TEXT,
            code TEXT,
            effect_write TEXT,
            effect_checkpoint TEXT,
            effect_discovery TEXT,
            effect_publication TEXT,
            effect_integration TEXT,
            effect_cleanup TEXT,
            commit_oid TEXT CHECK (
                commit_oid IS NULL
                OR (length(commit_oid) = 40 AND commit_oid NOT GLOB '*[^0-9a-f]*')
            ),
            result_data TEXT CHECK (result_data IS NULL OR json_valid(result_data)),
            accepted_at INTEGER NOT NULL,
            finished_at INTEGER,
            CHECK (
                (state = 'finished'
                    AND outcome IS NOT NULL AND code IS NOT NULL
                    AND effect_write IS NOT NULL AND effect_checkpoint IS NOT NULL
                    AND effect_discovery IS NOT NULL AND effect_publication IS NOT NULL
                    AND effect_integration IS NOT NULL AND effect_cleanup IS NOT NULL
                    AND finished_at IS NOT NULL)
                OR (state = 'accepted'
                    AND outcome IS NULL AND code IS NULL
                    AND effect_write IS NULL AND effect_checkpoint IS NULL
                    AND effect_discovery IS NULL AND effect_publication IS NULL
                    AND effect_integration IS NULL AND effect_cleanup IS NULL
                    AND commit_oid IS NULL AND result_data IS NULL
                    AND finished_at IS NULL)
            )
        )",
    ),
    (
        "request_operations",
        "CREATE TABLE request_operations (
            request_ulid TEXT NOT NULL
                REFERENCES request_records(request_ulid) ON DELETE CASCADE,
            ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
            family TEXT NOT NULL CHECK (family IN ('local', 'remote', 'key_material')),
            operation_ulid TEXT NOT NULL,
            PRIMARY KEY (request_ulid, ordinal)
        )",
    ),
    (
        "confirmation_records",
        "CREATE TABLE confirmation_records (
            confirmation_ulid TEXT PRIMARY KEY NOT NULL,
            scope_key TEXT NOT NULL,
            command TEXT NOT NULL,
            target TEXT NOT NULL,
            intent_digest TEXT NOT NULL CHECK (
                length(intent_digest) = 64 AND intent_digest NOT GLOB '*[^0-9a-f]*'
            ),
            observation_digest TEXT CHECK (
                observation_digest IS NULL
                OR (length(observation_digest) = 64
                    AND observation_digest NOT GLOB '*[^0-9a-f]*')
            ),
            created_at INTEGER NOT NULL,
            expires_at INTEGER NOT NULL,
            accepted_by_request TEXT,
            accepted_at INTEGER,
            CHECK ((accepted_by_request IS NULL) = (accepted_at IS NULL))
        )",
    ),
];

fn missing_request_tables(connection: &Connection) -> Result<Vec<&'static str>, RepositoryError> {
    let mut missing = Vec::new();
    for (name, definition) in REQUEST_TABLES {
        let exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                [name],
                |row| row.get(0),
            )
            .map_err(RepositoryError::sqlite)?;
        if !exists {
            missing.push(definition);
        }
    }
    Ok(missing)
}

/// Adds the request tables to an index that lacks them, and touches
/// nothing else.
///
/// An index that has them all is only read, so opening it takes no write
/// lock. One that lacks any is migrated in a transaction that takes the
/// write lock before it looks again: of two processes that open an old
/// index at once, the second waits for the first and then adds nothing.
pub(in crate::repository) fn migrate(connection: &mut Connection) -> Result<(), RepositoryError> {
    if missing_request_tables(connection)?.is_empty() {
        return Ok(());
    }
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(RepositoryError::sqlite)?;
    for definition in missing_request_tables(&transaction)? {
        transaction
            .execute_batch(definition)
            .map_err(RepositoryError::sqlite)?;
    }
    transaction.commit().map_err(RepositoryError::sqlite)
}

/// A domain operation a request began, and the journal that holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestOperation {
    pub family: OperationFamily,
    pub operation_id: OperationId,
}

/// A request as it is accepted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewRequest {
    pub request_id: RequestId,
    pub scope: ScopeKey,
    pub command: String,
    pub target: String,
    pub intent_digest: IntentDigest,
    /// The confirmation the request presents, which accepting it marks as
    /// accepted.
    pub confirmation: Option<ConfirmationId>,
    /// The branch the request will commit to, and where it stood.
    pub base_ref: Option<String>,
    pub base_oid: Option<git2::Oid>,
    /// What the domain is told to expect at the request's path.
    pub expected_digest: Option<ExpectedPathObservation>,
    /// The request's operations, in the order it runs them.
    pub operations: Vec<RequestOperation>,
}

/// What a finished request returned: the parts of its envelope that are
/// not derived from the request itself. `data` is the envelope's `data`,
/// which no binding fills with a body or a source.
#[derive(Clone, Debug, PartialEq)]
pub struct RequestResult {
    pub outcome: Outcome,
    pub code: ResultCode,
    pub effects: Effects,
    pub data: Option<serde_json::Value>,
}

/// A request record as it is stored.
#[derive(Clone, Debug, PartialEq)]
pub struct RequestRecord {
    pub request_id: RequestId,
    /// Raised by each call that enters the request. A call changes the
    /// record only while this is still the number it entered with.
    pub attempt: u64,
    pub scope: ScopeKey,
    pub command: String,
    pub target: String,
    pub intent_digest: IntentDigest,
    pub confirmation: Option<ConfirmationId>,
    pub base_ref: Option<String>,
    pub base_oid: Option<git2::Oid>,
    pub expected_digest: Option<ExpectedPathObservation>,
    pub cancel_requested: bool,
    pub state: RequestState,
    /// Set exactly when the record is `finished`.
    pub result: Option<RequestResult>,
    pub accepted_at: i64,
    pub finished_at: Option<i64>,
    pub operations: Vec<RequestOperation>,
}

/// What became of an attempt to accept a request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InsertRequestOutcome {
    /// The record was written as `accepted` with attempt 1, and the
    /// confirmation it presents, if any, is marked as accepted by it.
    Inserted,
    /// A record already holds the request ID. Nothing was written.
    RequestExists,
    /// The confirmation the request presents is not recorded, or another
    /// request has accepted it. Nothing was written.
    ConfirmationUnavailable,
}

/// A preview as it is recorded. Its creation time is the clock's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewConfirmation {
    pub confirmation_id: ConfirmationId,
    pub scope: ScopeKey,
    pub command: String,
    pub target: String,
    pub intent_digest: IntentDigest,
    /// The digest of what the preview observed: 64 lowercase hexadecimal
    /// digits.
    pub observation_digest: Option<String>,
    pub expires_at: i64,
}

/// A confirmation record as it is stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfirmationRecord {
    pub confirmation_id: ConfirmationId,
    pub scope: ScopeKey,
    pub command: String,
    pub target: String,
    pub intent_digest: IntentDigest,
    pub observation_digest: Option<String>,
    pub created_at: i64,
    pub expires_at: i64,
    /// The request that accepted it, and when; both or neither.
    pub accepted_by: Option<RequestId>,
    pub accepted_at: Option<i64>,
}

/// What a request table holds that no function here wrote.
fn invalid_stored_data() -> ReadError {
    ReadError::new(ResultCode::InternalError)
}

/// The value of a contract enumeration that a record holds by its name.
fn stored<T: Copy, const N: usize>(
    all: [T; N],
    name: fn(T) -> &'static str,
    text: &str,
) -> Result<T, ReadError> {
    all.into_iter()
        .find(|value| name(*value) == text)
        .ok_or_else(invalid_stored_data)
}

fn expected_digest_text(expected: &ExpectedPathObservation) -> String {
    match expected {
        ExpectedPathObservation::Missing => "missing".to_owned(),
        ExpectedPathObservation::Blake3(bytes) => {
            blake3::Hash::from_bytes(*bytes).to_hex().to_string()
        }
    }
}

fn stored_expected_digest(text: &str) -> Result<ExpectedPathObservation, ReadError> {
    if text == "missing" {
        return Ok(ExpectedPathObservation::Missing);
    }
    // The digest's own form check is the one for any stored digest.
    IntentDigest::from_stored(text)
        .and_then(|_| blake3::Hash::from_hex(text).ok())
        .map(|hash| ExpectedPathObservation::Blake3(*hash.as_bytes()))
        .ok_or_else(invalid_stored_data)
}

fn attempt_number(attempt: u64) -> Result<i64, ReadError> {
    i64::try_from(attempt).map_err(|_| invalid_stored_data())
}

fn request_operations(
    connection: &Connection,
    request_id: &str,
) -> Result<Vec<RequestOperation>, ReadError> {
    let mut statement = connection.prepare(
        "SELECT family, operation_ulid FROM request_operations
          WHERE request_ulid = ?1 ORDER BY ordinal ASC",
    )?;
    let mut rows = statement.query([request_id])?;
    let mut operations = Vec::new();
    while let Some(row) = rows.next()? {
        let family: String = row.get(0)?;
        let operation_id: String = row.get(1)?;
        operations.push(RequestOperation {
            family: stored(OperationFamily::ALL, OperationFamily::as_str, &family)?,
            operation_id: OperationId::parse(&operation_id).map_err(|_| invalid_stored_data())?,
        });
    }
    Ok(operations)
}

/// The stored result of a finished record, from the columns that hold it.
struct StoredResult {
    outcome: String,
    code: String,
    effects: [String; 6],
    commit_oid: Option<String>,
    data: Option<String>,
}

impl StoredResult {
    fn read(self) -> Result<RequestResult, ReadError> {
        let [
            write,
            checkpoint,
            discovery,
            publication,
            integration,
            cleanup,
        ] = self.effects;
        Ok(RequestResult {
            outcome: stored(Outcome::ALL, Outcome::as_str, &self.outcome)?,
            code: stored(ResultCode::ALL, ResultCode::as_str, &self.code)?,
            effects: Effects {
                write: stored(WriteEffect::ALL, WriteEffect::as_str, &write)?,
                checkpoint: stored(CheckpointEffect::ALL, CheckpointEffect::as_str, &checkpoint)?,
                discovery: stored(DiscoveryEffect::ALL, DiscoveryEffect::as_str, &discovery)?,
                publication: stored(
                    PublicationEffect::ALL,
                    PublicationEffect::as_str,
                    &publication,
                )?,
                integration: stored(
                    IntegrationEffect::ALL,
                    IntegrationEffect::as_str,
                    &integration,
                )?,
                cleanup: stored(CleanupEffect::ALL, CleanupEffect::as_str, &cleanup)?,
                commit_oid: self.commit_oid,
            },
            data: self
                .data
                .map(|data| serde_json::from_str(&data).map_err(|_| invalid_stored_data()))
                .transpose()?,
        })
    }
}

fn read_request(
    connection: &Connection,
    request_id: RequestId,
) -> Result<Option<RequestRecord>, ReadError> {
    let id = request_id.to_string();
    let row = connection
        .query_row(
            "SELECT attempt, scope_key, command, target, intent_digest, confirmation_ulid,
                    base_ref, base_oid, expected_digest, cancel_requested, state, outcome,
                    code, effect_write, effect_checkpoint, effect_discovery,
                    effect_publication, effect_integration, effect_cleanup, commit_oid,
                    result_data, accepted_at, finished_at
               FROM request_records WHERE request_ulid = ?1",
            [&id],
            |row| {
                Ok((
                    (
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, Option<String>>(7)?,
                        row.get::<_, Option<String>>(8)?,
                        row.get::<_, bool>(9)?,
                        row.get::<_, String>(10)?,
                    ),
                    (
                        row.get::<_, Option<String>>(11)?,
                        row.get::<_, Option<String>>(12)?,
                        (13..19)
                            .map(|column| row.get::<_, Option<String>>(column))
                            .collect::<Result<Vec<_>, _>>()?,
                        row.get::<_, Option<String>>(19)?,
                        row.get::<_, Option<String>>(20)?,
                    ),
                    row.get::<_, i64>(21)?,
                    row.get::<_, Option<i64>>(22)?,
                ))
            },
        )
        .optional()?;
    let Some((accepted, result, accepted_at, finished_at)) = row else {
        return Ok(None);
    };
    let (
        attempt,
        scope_key,
        command,
        target,
        intent_digest,
        confirmation,
        base_ref,
        base_oid,
        expected_digest,
        cancel_requested,
        state,
    ) = accepted;
    let state = stored(RequestState::ALL, RequestState::as_str, &state)?;
    let (outcome, code, effects, commit_oid, data) = result;
    let result = match state {
        RequestState::Accepted => None,
        RequestState::Finished => {
            let effects: Option<Vec<String>> = effects.into_iter().collect();
            let effects: [String; 6] = effects
                .and_then(|effects| effects.try_into().ok())
                .ok_or_else(invalid_stored_data)?;
            Some(
                StoredResult {
                    outcome: outcome.ok_or_else(invalid_stored_data)?,
                    code: code.ok_or_else(invalid_stored_data)?,
                    effects,
                    commit_oid,
                    data,
                }
                .read()?,
            )
        }
    };
    Ok(Some(RequestRecord {
        request_id,
        attempt: u64::try_from(attempt).map_err(|_| invalid_stored_data())?,
        scope: ScopeKey::from_stored(&scope_key),
        command,
        target,
        intent_digest: IntentDigest::from_stored(&intent_digest).ok_or_else(invalid_stored_data)?,
        confirmation: confirmation
            .map(|id| ConfirmationId::parse(&id).map_err(|_| invalid_stored_data()))
            .transpose()?,
        base_ref,
        base_oid: base_oid
            .map(|oid| git2::Oid::from_str(&oid).map_err(|_| invalid_stored_data()))
            .transpose()?,
        expected_digest: expected_digest
            .as_deref()
            .map(stored_expected_digest)
            .transpose()?,
        cancel_requested,
        state,
        result,
        accepted_at,
        finished_at,
        operations: request_operations(connection, &id)?,
    }))
}

fn read_confirmation(
    connection: &Connection,
    confirmation_id: ConfirmationId,
) -> Result<Option<ConfirmationRecord>, ReadError> {
    let row = connection
        .query_row(
            "SELECT scope_key, command, target, intent_digest, observation_digest,
                    created_at, expires_at, accepted_by_request, accepted_at
               FROM confirmation_records WHERE confirmation_ulid = ?1",
            [confirmation_id.to_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<i64>>(8)?,
                ))
            },
        )
        .optional()?;
    let Some((
        scope_key,
        command,
        target,
        intent_digest,
        observation_digest,
        created_at,
        expires_at,
        accepted_by,
        accepted_at,
    )) = row
    else {
        return Ok(None);
    };
    Ok(Some(ConfirmationRecord {
        confirmation_id,
        scope: ScopeKey::from_stored(&scope_key),
        command,
        target,
        intent_digest: IntentDigest::from_stored(&intent_digest).ok_or_else(invalid_stored_data)?,
        observation_digest,
        created_at,
        expires_at,
        accepted_by: accepted_by
            .map(|id| RequestId::parse(&id).map_err(|_| invalid_stored_data()))
            .transpose()?,
        accepted_at,
    }))
}

fn insert_request(
    transaction: &Transaction<'_>,
    request: &NewRequest,
    now: i64,
) -> Result<InsertRequestOutcome, ReadError> {
    let id = request.request_id.to_string();
    let exists: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM request_records WHERE request_ulid = ?1)",
        [&id],
        |row| row.get(0),
    )?;
    if exists {
        return Ok(InsertRequestOutcome::RequestExists);
    }
    let confirmation = request.confirmation.map(|id| id.to_string());
    if let Some(confirmation) = &confirmation {
        // Conditional on its not having been accepted meanwhile.
        let accepted = transaction.execute(
            "UPDATE confirmation_records SET accepted_by_request = ?2, accepted_at = ?3
              WHERE confirmation_ulid = ?1 AND accepted_by_request IS NULL",
            params![confirmation, id, now],
        )?;
        if accepted == 0 {
            return Ok(InsertRequestOutcome::ConfirmationUnavailable);
        }
    }
    transaction.execute(
        "INSERT INTO request_records (
            request_ulid, attempt, scope_key, command, target, intent_digest,
            confirmation_ulid, base_ref, base_oid, expected_digest, state, accepted_at
         ) VALUES (?1, 1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'accepted', ?10)",
        params![
            id,
            request.scope.as_str(),
            request.command,
            request.target,
            request.intent_digest.to_string(),
            confirmation,
            request.base_ref,
            request.base_oid.map(|oid| oid.to_string()),
            request.expected_digest.as_ref().map(expected_digest_text),
            now
        ],
    )?;
    for (ordinal, operation) in request.operations.iter().enumerate() {
        transaction.execute(
            "INSERT INTO request_operations (request_ulid, ordinal, family, operation_ulid)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                id,
                i64::try_from(ordinal).map_err(|_| invalid_stored_data())?,
                operation.family.as_str(),
                operation.operation_id.to_string()
            ],
        )?;
    }
    Ok(InsertRequestOutcome::Inserted)
}

fn finish_request(
    transaction: &Transaction<'_>,
    request_id: RequestId,
    attempt: u64,
    result: &RequestResult,
    now: i64,
) -> Result<bool, ReadError> {
    let data = result
        .data
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|_| invalid_stored_data())?;
    let effects = &result.effects;
    let finished = transaction.execute(
        "UPDATE request_records
            SET state = 'finished', outcome = ?3, code = ?4, effect_write = ?5,
                effect_checkpoint = ?6, effect_discovery = ?7, effect_publication = ?8,
                effect_integration = ?9, effect_cleanup = ?10, commit_oid = ?11,
                result_data = ?12, finished_at = ?13
          WHERE request_ulid = ?1 AND state = 'accepted' AND attempt = ?2",
        params![
            request_id.to_string(),
            attempt_number(attempt)?,
            result.outcome.as_str(),
            result.code.as_str(),
            effects.write.as_str(),
            effects.checkpoint.as_str(),
            effects.discovery.as_str(),
            effects.publication.as_str(),
            effects.integration.as_str(),
            effects.cleanup.as_str(),
            effects.commit_oid,
            data,
            now
        ],
    )?;
    Ok(finished == 1)
}

fn delete_request(
    transaction: &Transaction<'_>,
    request_id: RequestId,
    attempt: u64,
) -> Result<bool, ReadError> {
    let id = request_id.to_string();
    let attempt = attempt_number(attempt)?;
    let current: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM request_records
          WHERE request_ulid = ?1 AND state = 'accepted' AND attempt = ?2)",
        params![id, attempt],
        |row| row.get(0),
    )?;
    if !current {
        return Ok(false);
    }
    // The confirmation is free to be accepted again. Its expiry is left
    // alone: it still runs from the confirmation's creation.
    transaction.execute(
        "UPDATE confirmation_records SET accepted_by_request = NULL, accepted_at = NULL
          WHERE accepted_by_request = ?1
            AND confirmation_ulid = (SELECT confirmation_ulid FROM request_records
                                      WHERE request_ulid = ?1)",
        [&id],
    )?;
    transaction.execute(
        "DELETE FROM request_operations WHERE request_ulid = ?1",
        [&id],
    )?;
    transaction.execute(
        "DELETE FROM request_records
          WHERE request_ulid = ?1 AND state = 'accepted' AND attempt = ?2",
        params![id, attempt],
    )?;
    Ok(true)
}

impl RepositoryService {
    fn record_time(&self) -> i64 {
        self.clock.now().unix_timestamp()
    }

    /// Runs `write` in one transaction on the index, under the exclusive
    /// index lock, and commits what it wrote unless it fails. The lock and
    /// the connection are gone when this returns.
    fn write_records<T>(
        &self,
        write: impl FnOnce(&Transaction<'_>) -> Result<T, ReadError>,
    ) -> Result<T, ReadError> {
        let operation = RepositoryOperation::OpenRegistry;
        let data_directory = self
            .registry_path
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let _cache_guard = cache_write_guard(&self.registry_path, data_directory, operation)?;
        self.require_index_available(operation, None)?;
        let mut connection = open_registry(&self.registry_path, &mut |_| {})?;
        migrate_registry(&mut connection)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let value = write(&transaction)?;
        transaction.commit()?;
        Ok(value)
    }

    /// Accepts a request: writes its record as `accepted` with attempt 1
    /// and, in the same transaction, marks the confirmation it presents as
    /// accepted by it. Refused, with nothing written, for a request ID a
    /// record already holds and for a confirmation that is not there to
    /// accept.
    #[doc(hidden)]
    pub fn insert_request(&self, request: &NewRequest) -> Result<InsertRequestOutcome, ReadError> {
        let now = self.record_time();
        self.write_records(|transaction| insert_request(transaction, request, now))
    }

    /// The record of a request, or `None` when no record holds its ID.
    #[doc(hidden)]
    pub fn request_record(
        &self,
        request_id: RequestId,
    ) -> Result<Option<RequestRecord>, ReadError> {
        self.read_session(RepositoryOperation::Read, |connection| {
            read_request(connection, request_id)
        })
    }

    /// Enters an `accepted` request again: raises its attempt number and
    /// returns the new one, which the caller settles the record with.
    /// `None`, with nothing changed, when no record holds the ID or the
    /// record is `finished`.
    #[doc(hidden)]
    pub fn enter_request(&self, request_id: RequestId) -> Result<Option<u64>, ReadError> {
        self.write_records(|transaction| {
            let attempt: Option<i64> = transaction
                .query_row(
                    "UPDATE request_records SET attempt = attempt + 1
                      WHERE request_ulid = ?1 AND state = 'accepted'
                  RETURNING attempt",
                    [request_id.to_string()],
                    |row| row.get(0),
                )
                .optional()?;
            attempt
                .map(|attempt| u64::try_from(attempt).map_err(|_| invalid_stored_data()))
                .transpose()
        })
    }

    /// Stores `result` and marks the record `finished`, if the record is
    /// still `accepted` with `attempt`. Returns whether it was; a call
    /// whose attempt another call has since raised changes nothing.
    #[doc(hidden)]
    pub fn finish_request(
        &self,
        request_id: RequestId,
        attempt: u64,
        result: &RequestResult,
    ) -> Result<bool, ReadError> {
        let now = self.record_time();
        self.write_records(|transaction| {
            finish_request(transaction, request_id, attempt, result, now)
        })
    }

    /// Deletes the record, if it is still `accepted` with `attempt`, and
    /// in the same transaction releases the confirmation it accepted.
    /// Returns whether it was; the request ID is then free again.
    #[doc(hidden)]
    pub fn delete_request(&self, request_id: RequestId, attempt: u64) -> Result<bool, ReadError> {
        self.write_records(|transaction| delete_request(transaction, request_id, attempt))
    }

    /// Records a preview. Returns `false`, with nothing written, when a
    /// record already holds the confirmation ID.
    #[doc(hidden)]
    pub fn insert_confirmation(&self, confirmation: &NewConfirmation) -> Result<bool, ReadError> {
        let now = self.record_time();
        self.write_records(|transaction| {
            let inserted = transaction.execute(
                "INSERT INTO confirmation_records (
                    confirmation_ulid, scope_key, command, target, intent_digest,
                    observation_digest, created_at, expires_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT (confirmation_ulid) DO NOTHING",
                params![
                    confirmation.confirmation_id.to_string(),
                    confirmation.scope.as_str(),
                    confirmation.command,
                    confirmation.target,
                    confirmation.intent_digest.to_string(),
                    confirmation.observation_digest,
                    now,
                    confirmation.expires_at
                ],
            )?;
            Ok(inserted == 1)
        })
    }

    /// The record of a confirmation, or `None` when no record holds its
    /// ID.
    #[doc(hidden)]
    pub fn confirmation_record(
        &self,
        confirmation_id: ConfirmationId,
    ) -> Result<Option<ConfirmationRecord>, ReadError> {
        self.read_session(RepositoryOperation::Read, |connection| {
            read_confirmation(connection, confirmation_id)
        })
    }
}
