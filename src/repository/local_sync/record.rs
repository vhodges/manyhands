//! Same-database companion authority. Creation is indivisible with the local
//! envelope and is serialized under the common-Git lease. No fallback journal.
#[cfg(test)]
use crate::results::ResultCode;
use crate::{canonical::ItemId, repository::*};
#[cfg(test)]
use rusqlite::TransactionBehavior;
use rusqlite::{Connection, OptionalExtension, params};
use std::path::PathBuf;

const MARKER: &str = "local_context_sync_v1";
const CASCADE_MARKER: &str = "local_context_sync_child_cascade_v2";
const RECORD: &str = "CREATE TABLE local_context_sync_records (
 operation_ulid TEXT PRIMARY KEY NOT NULL,
 local_record_id INTEGER NOT NULL UNIQUE REFERENCES operation_records(id) ON DELETE CASCADE,
 repository_id INTEGER NOT NULL REFERENCES repositories(id),
 format_version INTEGER NOT NULL CHECK(format_version=1),
 kind TEXT NOT NULL CHECK(kind IN ('document','ticket')),
 item_id TEXT NOT NULL,
 context_worktree TEXT NOT NULL,
 context_ref TEXT NOT NULL,
 primary_ref TEXT NOT NULL,
 observation TEXT NOT NULL,
 context_parent TEXT NOT NULL CHECK(length(context_parent)=40 AND context_parent NOT GLOB '*[^0-9a-f]*'),
 primary_parent TEXT NOT NULL CHECK(length(primary_parent)=40 AND primary_parent NOT GLOB '*[^0-9a-f]*'),
 state TEXT NOT NULL CHECK(state IN ('prepared','applying','conflict_pending','resolution_prepared','commit_prepared','integrated','index_pending','complete','recovery_required','cancelled','abandoned')),
 owner_epoch INTEGER NOT NULL CHECK(owner_epoch>0),
 semantic_revision INTEGER NOT NULL CHECK(semantic_revision>0),
 generation INTEGER NOT NULL CHECK(generation>0),
 closure_operation TEXT,
 closure_checkpoint TEXT,
 candidate_oid TEXT,
 result_oid TEXT,
 active_resolution TEXT,
 cancel_issued INTEGER NOT NULL DEFAULT 0 CHECK(cancel_issued>=0),
 cancel_acknowledged INTEGER NOT NULL DEFAULT 0 CHECK(cancel_acknowledged>=0 AND cancel_acknowledged<=cancel_issued),
 integration_proven INTEGER NOT NULL DEFAULT 0 CHECK(integration_proven IN (0,1)),
 artifacts_released INTEGER NOT NULL DEFAULT 0 CHECK(artifacts_released IN (0,1)),
 index_complete INTEGER NOT NULL DEFAULT 0 CHECK(index_complete IN (0,1)),
 CHECK((closure_operation IS NULL)=(closure_checkpoint IS NULL)),
 CHECK(index_complete=0 OR (integration_proven=1 AND artifacts_released=1))
)";
const STEPS: &str = "CREATE TABLE local_context_sync_steps (
 operation_ulid TEXT NOT NULL REFERENCES local_context_sync_records(operation_ulid) ON DELETE CASCADE,
 ordinal INTEGER NOT NULL CHECK(ordinal>=0),
 generation INTEGER NOT NULL CHECK(generation>0),
 owner_epoch INTEGER NOT NULL CHECK(owner_epoch>0),
 kind TEXT NOT NULL CHECK(kind IN ('checkout','conflict_install','objects','path','index','reference','metadata_release')),
 phase TEXT NOT NULL CHECK(phase IN ('prepared','observed')),
 baseline_index_digest BLOB NOT NULL CHECK(typeof(baseline_index_digest)='blob' AND length(baseline_index_digest)=32),
 expected_index_digest BLOB CHECK(expected_index_digest IS NULL OR (typeof(expected_index_digest)='blob' AND length(expected_index_digest)=32)),
 baseline_metadata_digest BLOB NOT NULL CHECK(typeof(baseline_metadata_digest)='blob' AND length(baseline_metadata_digest)=32),
 expected_metadata_digest BLOB CHECK(expected_metadata_digest IS NULL OR (typeof(expected_metadata_digest)='blob' AND length(expected_metadata_digest)=32)),
 expected_old_oid TEXT,
 expected_new_oid TEXT,
 PRIMARY KEY(operation_ulid,ordinal)
)";
const RESOLUTIONS: &str = "CREATE TABLE local_context_sync_resolutions (
 attempt_ulid TEXT PRIMARY KEY NOT NULL,
 operation_ulid TEXT NOT NULL REFERENCES local_context_sync_records(operation_ulid) ON DELETE CASCADE,
 request_ulid TEXT NOT NULL UNIQUE,
 generation INTEGER NOT NULL CHECK(generation>0),
 owner_epoch INTEGER NOT NULL CHECK(owner_epoch>0),
 observation TEXT NOT NULL,
 input_digest BLOB NOT NULL CHECK(typeof(input_digest)='blob' AND length(input_digest)=32),
 candidate_oid TEXT,
 state TEXT NOT NULL CHECK(state IN ('prepared','applying','commit_prepared','integrated','complete','abandoned'))
)";

fn failure() -> RepositoryError {
    RepositoryError::new(
        RepositoryOperation::Read,
        None,
        RepositoryErrorKind::RecoveryRequired,
        "local synchronization authority is unavailable",
    )
}
#[cfg(test)]
fn recovery_error(_: impl std::fmt::Debug) -> ReadError {
    ReadError::new(ResultCode::RecoveryRequired)
}
#[cfg(test)]
fn sql_error(error: rusqlite::Error) -> ReadError {
    ReadError::new(match error.sqlite_error_code() {
        Some(rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked) => {
            ResultCode::Busy
        }
        _ => ResultCode::IndexUnavailable,
    })
}

fn schemas(cascade: bool) -> [(&'static str, String); 3] {
    let child = |sql: &str| {
        if cascade {
            sql.to_owned()
        } else {
            sql.replace(" ON DELETE CASCADE", "")
        }
    };
    [
        ("local_context_sync_records", RECORD.to_owned()),
        ("local_context_sync_steps", child(STEPS)),
        ("local_context_sync_resolutions", child(RESOLUTIONS)),
    ]
}

// Called within recovery::migrate_operation_records' transaction. Upgrading
// the two child FKs and recording the marker either commit together or roll
// back together; existing v1 authority is audited before any table is rebuilt.
pub(in super::super) fn migrate(db: &Connection) -> Result<(), RepositoryError> {
    let migrated: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [MARKER],
            |r| r.get(0),
        )
        .map_err(|_| failure())?;
    let cascaded: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [CASCADE_MARKER],
            |r| r.get(0),
        )
        .map_err(|_| failure())?;
    if cascaded && !migrated {
        return Err(failure());
    }
    if !migrated {
        let partial:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name IN ('local_context_sync_records','local_context_sync_steps','local_context_sync_resolutions'))",[],|r|r.get(0)).map_err(|_|failure())?;
        if partial {
            return Err(failure());
        }
    }
    for (name, sql) in schemas(cascaded || !migrated) {
        let actual: Option<String> = db
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name=?1",
                [name],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| failure())?;
        match actual {
            None if !migrated => {
                db.execute_batch(&sql).map_err(|_| failure())?;
            }
            Some(actual) if compact(&actual) == compact(&sql) => {}
            _ => return Err(failure()),
        }
    }
    if !migrated {
        db.execute("INSERT INTO registry_migrations(name) VALUES(?1)", [MARKER])
            .map_err(|_| failure())?;
    }
    if !cascaded {
        if migrated {
            audit_with_schema(db, false)?;
            for (name, sql) in schemas(true).into_iter().skip(1) {
                // Both audited layouts have identical columns/order; only the
                // parent FK delete action changes. No authority is synthesized.
                db.execute_batch(&format!("ALTER TABLE {name} RENAME TO {name}_before_cascade; {sql}; INSERT INTO {name} SELECT * FROM {name}_before_cascade; DROP TABLE {name}_before_cascade;")).map_err(|_|failure())?;
            }
        }
        db.execute(
            "INSERT INTO registry_migrations(name) VALUES(?1)",
            [CASCADE_MARKER],
        )
        .map_err(|_| failure())?;
    }
    audit(db)
}
fn compact(sql: &str) -> String {
    sql.split_whitespace().collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LocalSyncState {
    Prepared,
    Applying,
    ConflictPending,
    ResolutionPrepared,
    CommitPrepared,
    Integrated,
    IndexPending,
    Complete,
    RecoveryRequired,
    Cancelled,
    Abandoned,
}
impl LocalSyncState {
    fn parse(state: &str) -> Option<Self> {
        Some(match state {
            "prepared" => Self::Prepared,
            "applying" => Self::Applying,
            "conflict_pending" => Self::ConflictPending,
            "resolution_prepared" => Self::ResolutionPrepared,
            "commit_prepared" => Self::CommitPrepared,
            "integrated" => Self::Integrated,
            "index_pending" => Self::IndexPending,
            "complete" => Self::Complete,
            "recovery_required" => Self::RecoveryRequired,
            "cancelled" => Self::Cancelled,
            "abandoned" => Self::Abandoned,
            _ => return None,
        })
    }
    #[cfg(test)]
    fn ended(self) -> bool {
        matches!(self, Self::Complete | Self::Cancelled | Self::Abandoned)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct LocalContextSyncRecord {
    pub(crate) operation_id: OperationId,
    pub(crate) local_record_id: i64,
    pub(crate) repository_id: i64,
    pub(crate) kind: AuthoringKind,
    pub(crate) item_id: ItemId,
    pub(crate) worktree: PathBuf,
    pub(crate) context_ref: String,
    pub(crate) primary_ref: String,
    pub(crate) observation: String,
    pub(crate) context_parent: git2::Oid,
    pub(crate) primary_parent: git2::Oid,
    pub(crate) state: LocalSyncState,
    pub(crate) owner_epoch: i64,
    pub(crate) semantic_revision: i64,
    pub(crate) generation: i64,
    pub(crate) closure_operation: Option<OperationId>,
    pub(crate) closure_checkpoint: Option<git2::Oid>,
    pub(crate) candidate: Option<git2::Oid>,
    pub(crate) result: Option<git2::Oid>,
    pub(crate) active_resolution: Option<OperationId>,
    pub(crate) cancel_issued: i64,
    pub(crate) cancel_acknowledged: i64,
    pub(crate) integration_proven: bool,
    pub(crate) artifacts_released: bool,
    pub(crate) index_complete: bool,
}

fn strict_oid(text: &str) -> rusqlite::Result<git2::Oid> {
    let oid = git2::Oid::from_str(text).map_err(|_| rusqlite::Error::InvalidQuery)?;
    if oid.is_zero() || oid.to_string() != text {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(oid)
}

fn decode(r: &rusqlite::Row<'_>) -> rusqlite::Result<LocalContextSyncRecord> {
    let invalid = || rusqlite::Error::InvalidQuery;
    let operation_id =
        OperationId::parse(&r.get::<_, String>("operation_ulid")?).map_err(|_| invalid())?;
    let kind = match r.get::<_, String>("kind")?.as_str() {
        "document" => AuthoringKind::Document,
        "ticket" => AuthoringKind::Ticket,
        _ => return Err(invalid()),
    };
    let state = LocalSyncState::parse(&r.get::<_, String>("state")?).ok_or_else(invalid)?;
    if r.get::<_, i64>("format_version")? != 1 {
        return Err(invalid());
    }
    let oid = |column| -> rusqlite::Result<Option<git2::Oid>> {
        r.get::<_, Option<String>>(column)?
            .map(|s| strict_oid(&s))
            .transpose()
    };
    let flag = |column| -> rusqlite::Result<bool> {
        match r.get::<_, i64>(column)? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(invalid()),
        }
    };
    let result = LocalContextSyncRecord {
        candidate: oid("candidate_oid")?,
        result: oid("result_oid")?,
        active_resolution: r
            .get::<_, Option<String>>("active_resolution")?
            .map(|s| OperationId::parse(&s).map_err(|_| invalid()))
            .transpose()?,
        cancel_issued: r.get("cancel_issued")?,
        cancel_acknowledged: r.get("cancel_acknowledged")?,
        integration_proven: flag("integration_proven")?,
        artifacts_released: flag("artifacts_released")?,
        index_complete: flag("index_complete")?,
        operation_id,
        kind,
        state,
        local_record_id: r.get("local_record_id")?,
        repository_id: r.get("repository_id")?,
        item_id: r
            .get::<_, String>("item_id")?
            .parse()
            .map_err(|_| invalid())?,
        worktree: PathBuf::from(r.get::<_, String>("context_worktree")?),
        context_ref: r.get("context_ref")?,
        primary_ref: r.get("primary_ref")?,
        observation: r.get("observation")?,
        context_parent: strict_oid(&r.get::<_, String>("context_parent")?)?,
        primary_parent: strict_oid(&r.get::<_, String>("primary_parent")?)?,
        owner_epoch: r.get("owner_epoch")?,
        semantic_revision: r.get("semantic_revision")?,
        generation: r.get("generation")?,
        closure_operation: r
            .get::<_, Option<String>>("closure_operation")?
            .map(|s| OperationId::parse(&s).map_err(|_| invalid()))
            .transpose()?,
        closure_checkpoint: oid("closure_checkpoint")?,
    };
    if result.cancel_issued < 0
        || result.cancel_acknowledged < 0
        || result.cancel_acknowledged > result.cancel_issued
        || result.artifacts_released && !result.integration_proven
        || result.index_complete && !(result.integration_proven && result.artifacts_released)
        || matches!(
            result.state,
            LocalSyncState::Integrated | LocalSyncState::IndexPending | LocalSyncState::Complete
        ) && !result.integration_proven
        || result.state == LocalSyncState::Complete && !result.index_complete
    {
        return Err(invalid());
    }
    if result.owner_epoch <= 0
        || result.semantic_revision <= 0
        || result.generation <= 0
        || result.context_parent.is_zero()
        || result.primary_parent.is_zero()
        || result.closure_operation.is_some() != result.closure_checkpoint.is_some()
        || result.context_ref
            != format!(
                "refs/heads/manyhands/{}/{}",
                authoring_kind_segment(&result.kind),
                result.item_id
            )
        || !git2::Reference::is_valid_name(&result.primary_ref)
        || !result.worktree.is_absolute()
        || !valid_observation(&result.observation)
    {
        return Err(invalid());
    }
    Ok(result)
}
fn valid_observation(s: &str) -> bool {
    s.strip_prefix("local-sync-v1:").is_some_and(|s| {
        s.len() == 64
            && s.bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    })
}

// rusqlite's i64 decoder requires SQLite INTEGER storage, not merely numeric
// affinity. A REAL or TEXT that passes SQLite comparisons is not an epoch.
fn validate_child_authority(row: &rusqlite::Row<'_>) -> rusqlite::Result<()> {
    let generation: i64 = row.get("generation")?;
    let epoch: i64 = row.get("owner_epoch")?;
    let parent_generation: i64 = row.get("parent_generation")?;
    let parent_epoch: i64 = row.get("parent_epoch")?;
    if generation <= 0 || epoch <= 0 || generation != parent_generation || epoch > parent_epoch {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(())
}

pub(in super::super) fn audit(db: &Connection) -> Result<(), RepositoryError> {
    let cascaded: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [CASCADE_MARKER],
            |r| r.get(0),
        )
        .map_err(|_| failure())?;
    if !cascaded {
        return Err(failure());
    }
    audit_with_schema(db, true)
}

fn audit_with_schema(db: &Connection, cascade: bool) -> Result<(), RepositoryError> {
    let marked: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [MARKER],
            |r| r.get(0),
        )
        .map_err(|_| failure())?;
    if !marked {
        return Err(failure());
    }
    for (name, sql) in schemas(cascade) {
        let actual: String = db
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name=?1",
                [name],
                |r| r.get(0),
            )
            .map_err(|_| failure())?;
        if compact(&actual) != compact(&sql) {
            return Err(failure());
        }
        if db
            .prepare(&format!("PRAGMA foreign_key_check({name})"))
            .and_then(|mut s| s.exists([]))
            .map_err(|_| failure())?
        {
            return Err(failure());
        }
        // Re-evaluate the audited CHECK constraints too: a connection that
        // disabled them must not turn malformed stored bytes into authority.
        let checks = db
            .prepare(&format!("PRAGMA integrity_check({name})"))
            .and_then(|mut s| {
                s.query_map([], |r| r.get::<_, String>(0))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|_| failure())?;
        if checks != ["ok"] {
            return Err(failure());
        }
    }
    let broken:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM operation_records o LEFT JOIN local_context_sync_records s ON s.local_record_id=o.id WHERE o.action='context_sync_local' AND s.operation_ulid IS NULL) OR EXISTS(SELECT 1 FROM local_context_sync_records s LEFT JOIN operation_records o ON o.id=s.local_record_id LEFT JOIN repositories r ON r.id=s.repository_id WHERE o.id IS NULL OR r.id IS NULL OR o.operation_ulid IS NOT s.operation_ulid OR o.repository_id IS NOT s.repository_id OR o.action!='context_sync_local' OR o.root_path IS NOT r.root_path OR o.item_id IS NOT s.item_id OR o.context_path IS NOT s.context_worktree OR o.target IS NOT s.observation OR (o.state='completed')!=(s.state IN ('complete','cancelled','abandoned')) OR (s.active_resolution IS NOT NULL AND NOT EXISTS(SELECT 1 FROM local_context_sync_resolutions a WHERE a.attempt_ulid=s.active_resolution AND a.operation_ulid=s.operation_ulid AND a.generation=s.generation)))",[],|r|r.get(0)).map_err(|_|failure())?;
    if broken {
        return Err(failure());
    }
    let rows = db
        .prepare("SELECT * FROM local_context_sync_records")
        .and_then(|mut s| s.query_map([], decode)?.collect::<Result<Vec<_>, _>>())
        .map_err(|_| failure())?;
    for row in rows {
        if foreign_id_collision(db,&row.operation_id.to_string(),Some(row.local_record_id))?
            || db.query_row("SELECT EXISTS(SELECT 1 FROM local_context_sync_resolutions WHERE attempt_ulid=?1)",[row.operation_id.to_string()],|r|r.get::<_,bool>(0)).map_err(|_|failure())? {
            return Err(failure());
        }
    }
    let mut steps = db
        .prepare("SELECT s.kind,s.expected_old_oid,s.expected_new_oid,s.ordinal,s.generation,s.owner_epoch,p.generation AS parent_generation,p.owner_epoch AS parent_epoch FROM local_context_sync_steps s JOIN local_context_sync_records p ON p.operation_ulid=s.operation_ulid")
        .map_err(|_| failure())?;
    let mut rows = steps.query([]).map_err(|_| failure())?;
    while let Some(row) = rows.next().map_err(|_| failure())? {
        validate_child_authority(row).map_err(|_| failure())?;
        if row.get::<_, i64>("ordinal").map_err(|_| failure())? < 0 {
            return Err(failure());
        }
        let kind: String = row.get(0).map_err(|_| failure())?;
        let parse = |column| -> Result<Option<git2::Oid>, RepositoryError> {
            row.get::<_, Option<String>>(column)
                .map_err(|_| failure())?
                .map(|text| strict_oid(&text).map_err(|_| failure()))
                .transpose()
        };
        let (old, new) = (parse(1)?, parse(2)?);
        // A ref CAS intent needs both exact endpoints, even before its effect.
        // Other effect kinds may have no ref transition; any stored OID must
        // still decode canonically. This audit grants no execution authority.
        if kind == "reference" && (old.is_none() || new.is_none()) {
            return Err(failure());
        }
    }
    let mut attempts=db.prepare("SELECT a.attempt_ulid,a.operation_ulid,a.request_ulid,a.observation,a.candidate_oid,a.generation,a.owner_epoch,p.generation AS parent_generation,p.owner_epoch AS parent_epoch FROM local_context_sync_resolutions a JOIN local_context_sync_records p ON p.operation_ulid=a.operation_ulid").map_err(|_|failure())?;
    let mut rows = attempts.query([]).map_err(|_| failure())?;
    while let Some(row) = rows.next().map_err(|_| failure())? {
        validate_child_authority(row).map_err(|_| failure())?;
        for column in 0..3 {
            OperationId::parse(&row.get::<_, String>(column).map_err(|_| failure())?)
                .map_err(|_| failure())?;
        }
        if foreign_id_collision(db, &row.get::<_, String>(0).map_err(|_| failure())?, None)? {
            return Err(failure());
        }
        if !valid_observation(&row.get::<_, String>(3).map_err(|_| failure())?) {
            return Err(failure());
        }
        if let Some(oid) = row.get::<_, Option<String>>(4).map_err(|_| failure())? {
            strict_oid(&oid).map_err(|_| failure())?;
        }
    }
    Ok(())
}

#[cfg(test)]
thread_local! { pub(super) static WRITE_FAULT: std::cell::Cell<Option<std::io::ErrorKind>>=const {std::cell::Cell::new(None)}; }

#[cfg(test)]
fn open_for_write(service: &RepositoryService) -> Result<Connection, ReadError> {
    // Typed failures are injected at the actual registry-opening boundary,
    // before either identity row can be inserted (and before any Git effect).
    let access: std::io::Result<()> = {
        #[cfg(test)]
        if let Some(kind) = WRITE_FAULT.with(|f| f.take()) {
            return Err(journal_io(std::io::Error::from(kind)));
        }
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&service.registry_path)
            .map(|_| ())
    };
    access.map_err(journal_io)?;
    open_registry(&service.registry_path, &mut |_| {}).map_err(|error| match error.kind {
        RepositoryErrorKind::RepositoryBusy => ReadError::new(ResultCode::Busy),
        RepositoryErrorKind::RecoveryRequired => ReadError::new(ResultCode::RecoveryRequired),
        _ => ReadError::new(ResultCode::IndexUnavailable),
    })
}
#[cfg(test)]
fn journal_io(_: std::io::Error) -> ReadError {
    ReadError::new(ResultCode::IndexUnavailable)
}

// Task 4 exercises the real store internally. The approved Task 5 executor
// will make these seams production callers; no public claim/takeover API is
// published ahead of preparation and Task 7 request-attempt authority.
#[cfg(test)]
impl RepositoryService {
    /// Claim a retained journal without substituting its frozen observation or
    /// parents. This only transfers domain ownership; it performs no Git work.
    pub(in super::super) fn take_over_local_context_sync(
        &self,
        repo: &ResolvedRepository,
        expected: &LocalContextSyncRecord,
    ) -> Result<LocalContextSyncRecord, ReadError> {
        let git = git2::Repository::open(repo.root())
            .map_err(|_| ReadError::new(ResultCode::RepositoryInaccessible))?;
        let lease = repository_lease(&git, repo.root(), RepositoryOperation::ContextSyncLocal)?;
        let _cache = cache_write_guard(
            &self.registry_path,
            repo.root(),
            RepositoryOperation::ContextSyncLocal,
        )?;
        let mut db = open_for_write(self)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sql_error)?;
        audit(&tx).map_err(recovery_error)?;
        if !lease
            .still_names(&git)
            .map_err(|_| ReadError::new(ResultCode::RepositoryInaccessible))?
        {
            return Err(ReadError::new(ResultCode::ExternalChange));
        }
        let mut current=tx.query_row("SELECT s.* FROM local_context_sync_records s JOIN repositories r ON r.id=s.repository_id WHERE s.operation_ulid=?1 AND r.root_path=?2",params![expected.operation_id.to_string(),repo.root().to_str()],decode).map_err(recovery_error)?;
        if &current != expected || current.state.ended() {
            return Err(ReadError::new(ResultCode::ExternalChange));
        }
        require_remote_released(&tx, current.repository_id)?;
        current.owner_epoch = current
            .owner_epoch
            .checked_add(1)
            .ok_or_else(|| ReadError::new(ResultCode::RecoveryRequired))?;
        let changed=tx.execute("UPDATE local_context_sync_records SET owner_epoch=?2 WHERE operation_ulid=?1 AND owner_epoch=?3 AND generation=?4",params![current.operation_id.to_string(),current.owner_epoch,expected.owner_epoch,expected.generation]).map_err(sql_error)?;
        if changed != 1 {
            return Err(ReadError::new(ResultCode::RecoveryRequired));
        }
        tx.commit().map_err(sql_error)?;
        Ok(current)
    }
    /// The domain journal boundary: bind a frozen, observed pair without Git
    /// effects. Request acceptance/linkage is a separate mutation-boundary duty.
    /// This is not a synchronization executor.
    pub(in super::super) fn begin_local_context_sync(
        &self,
        repo: &ResolvedRepository,
        operation: OperationId,
        kind: AuthoringKind,
        item: &ItemId,
        observation: &str,
    ) -> Result<LocalContextSyncRecord, ReadError> {
        self.begin_local_context_sync_with(repo, operation, kind, item, observation, || {})
    }

    pub(super) fn begin_local_context_sync_with(
        &self,
        repo: &ResolvedRepository,
        operation: OperationId,
        kind: AuthoringKind,
        item: &ItemId,
        observation: &str,
        between: impl FnOnce(),
    ) -> Result<LocalContextSyncRecord, ReadError> {
        if !std::fs::symlink_metadata(&self.registry_path)
            .map_err(journal_io)?
            .is_file()
        {
            return Err(ReadError::new(ResultCode::IndexUnavailable));
        }
        if !valid_observation(observation) {
            return Err(ReadError::new(ResultCode::InvalidInput));
        }
        let git = git2::Repository::open(repo.root())
            .map_err(|_| ReadError::new(ResultCode::RepositoryInaccessible))?;
        let lease = repository_lease(&git, repo.root(), RepositoryOperation::ContextSyncLocal)?;
        if !lease
            .still_names(&git)
            .map_err(|_| ReadError::new(ResultCode::RepositoryInaccessible))?
        {
            return Err(ReadError::new(ResultCode::ExternalChange));
        }
        let existing = self.read_session(RepositoryOperation::Read, |db| {
            audit(db).map_err(recovery_error)?;
            db.query_row(
                "SELECT s.* FROM local_context_sync_records s JOIN repositories r ON r.id=s.repository_id WHERE s.operation_ulid=?1 AND r.root_path=?2",
                params![operation.to_string(),repo.root().to_str()],
                decode,
            )
            .optional()
            .map_err(recovery_error)
        })?;
        if let Some(existing) = existing {
            if existing.repository_id != repo.registration_id()
                || existing.kind != kind
                || existing.item_id != *item
                || existing.observation != observation
            {
                return Err(ReadError::new(ResultCode::RequestMismatch));
            }
            return Ok(existing);
        }
        let current = self.observe_local_context_sync(repo, kind, item)?;
        if current.observation != observation {
            return Err(ReadError::new(ResultCode::ExternalChange));
        }
        between();
        let _cache =
            cache_write_guard(&self.registry_path, repo.root(), RepositoryOperation::Read)?;
        // Open only after taking the cache guard: never keep a writable
        // connection to a database that a rebuild could have replaced.
        let mut db = open_for_write(self)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sql_error)?;
        let facts = super::observe::facts(&tx, repo, kind, item)?;
        if super::observe::from_facts(repo, &facts, kind, item)?.observation != observation {
            return Err(ReadError::new(ResultCode::ExternalChange));
        }
        let id = operation.to_string();
        if operation_collision(&tx, &id)? {
            return Err(ReadError::new(ResultCode::RequestMismatch));
        }
        let admitted = admit(&tx, facts.repository_id, kind, item, &git, &current)?;
        tx.execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,target,item_id,context_path,state,completed_step,observed_at) VALUES(?1,?2,?3,'context_sync_local',?4,?5,?6,'created','local_sync_prepared',?7)",params![facts.repository_id,repo.root().to_str(),id,observation,item.to_string(),current.context_worktree,time::OffsetDateTime::now_utc().unix_timestamp()]).map_err(sql_error)?;
        let envelope = tx.last_insert_rowid();
        tx.execute("INSERT INTO local_context_sync_records(operation_ulid,local_record_id,repository_id,format_version,kind,item_id,context_worktree,context_ref,primary_ref,observation,context_parent,primary_parent,state,owner_epoch,semantic_revision,generation,closure_operation,closure_checkpoint) VALUES(?1,?2,?3,1,?4,?5,?6,?7,?8,?9,?10,?11,'prepared',1,1,1,?12,?13)",params![id,envelope,facts.repository_id,authoring_kind_segment(&kind),item.to_string(),current.context_worktree,current.context_ref,current.primary_ref,observation,current.context_head,current.primary_head,admitted.as_ref().map(|(op,_)|op.to_string()),admitted.as_ref().map(|(_,oid)|oid.to_string())]).map_err(sql_error)?;
        let row = tx
            .query_row(
                "SELECT * FROM local_context_sync_records WHERE operation_ulid=?1",
                [id],
                decode,
            )
            .map_err(recovery_error)?;
        tx.commit().map_err(sql_error)?;
        Ok(row)
    }
    pub(in super::super) fn read_local_context_sync_record(
        &self,
        repo: &ResolvedRepository,
        id: OperationId,
    ) -> Result<LocalContextSyncRecord, ReadError> {
        self.read_session(RepositoryOperation::Read,|db| {
            audit(db).map_err(recovery_error)?;
            let row=db.query_row("SELECT s.* FROM local_context_sync_records s JOIN repositories r ON r.id=s.repository_id WHERE s.operation_ulid=?1 AND r.root_path=?2",params![id.to_string(),repo.root().to_str()],decode).optional().map_err(recovery_error)?;
            row.ok_or_else(||ReadError::new(ResultCode::RecoveryRequired))
        })
    }
}

#[cfg(test)]
fn operation_collision(db: &Connection, id: &str) -> Result<bool, ReadError> {
    Ok(foreign_id_collision(db, id, None).map_err(recovery_error)?
        || owned_identity(db, id).map_err(recovery_error)?)
}

// Only local-sync identities are excluded here. Old remote/local index handoffs
// and lifecycle child bindings retain their existing cross-journal rules.
const FOREIGN_IDENTITIES: &[(&str, &str)] = &[
    ("remote_operation_records", "operation_ulid"),
    ("key_material_operations", "operation_id"),
    ("key_registry_changes", "operation_id"),
    ("document_promotions", "operation_id"),
    ("document_promotions", "context_sync_id"),
    ("document_promotions", "primary_sync_id"),
    ("ticket_closures", "operation_id"),
    ("ticket_closures", "context_sync_id"),
    ("ticket_closures", "primary_sync_id"),
    ("comment_publication_bindings", "operation_ulid"),
    ("comment_publication_bindings", "synchronization_ulid"),
    ("remote_resolution_attempts", "attempt_ulid"),
    ("remote_identity_confirmations", "confirmation_ulid"),
];

fn foreign_id_collision(
    db: &Connection,
    id: &str,
    own_envelope: Option<i64>,
) -> Result<bool, RepositoryError> {
    let local:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM operation_records WHERE operation_ulid=?1 AND (?2 IS NULL OR id!=?2))",params![id,own_envelope],|r|r.get(0)).map_err(|_|failure())?;
    if local {
        return Ok(true);
    }
    for (table, column) in FOREIGN_IDENTITIES {
        if recovery::has_table(db, table).map_err(|_| failure())?
            && db
                .query_row(
                    &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE {column}=?1)"),
                    [id],
                    |r| r.get::<_, bool>(0),
                )
                .map_err(|_| failure())?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(in super::super) fn owned_identity(db: &Connection, id: &str) -> Result<bool, RepositoryError> {
    // The envelope alone still reserves its ID after companion loss.
    for (table, column, predicate) in [
        (
            "operation_records",
            "operation_ulid",
            " AND action='context_sync_local'",
        ),
        ("local_context_sync_records", "operation_ulid", ""),
        ("local_context_sync_resolutions", "attempt_ulid", ""),
    ] {
        if recovery::has_table(db, table).map_err(|_| failure())?
            && db
                .query_row(
                    &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE {column}=?1{predicate})"),
                    [id],
                    |r| r.get::<_, bool>(0),
                )
                .map_err(|_| failure())?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Call inside the foreign journal's insertion transaction, before effects.
/// Preflight/replay reads may also call this, but cannot replace that check.
pub(in super::super) fn require_foreign_id_available(
    db: &Connection,
    id: OperationId,
) -> Result<(), RepositoryError> {
    if owned_identity(db, &id.to_string())? {
        Err(RepositoryError::new(
            RepositoryOperation::Read,
            None,
            RepositoryErrorKind::OperationMismatch,
            "the operation ID belongs to local synchronization",
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
fn admit(
    db: &Connection,
    repository_id: i64,
    kind: AuthoringKind,
    item: &ItemId,
    git: &git2::Repository,
    observation: &LocalContextSyncObservationDto,
) -> Result<Option<(OperationId, git2::Oid)>, ReadError> {
    // Admission is intentionally one transaction with envelope creation.
    let pending:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM operation_records WHERE root_path=(SELECT root_path FROM repositories WHERE id=?1) AND (state!='completed' OR completed_step='comment_registration_pending')) OR EXISTS(SELECT 1 FROM document_promotions WHERE repository_id=?1 AND step NOT IN ('complete','abandoned','superseded'))",[repository_id],|r|r.get(0))?;
    if pending {
        return Err(ReadError::new(ResultCode::RecoveryRequired));
    }
    require_remote_released(db, repository_id)?;
    let ids=db.prepare("SELECT operation_id FROM ticket_closures WHERE repository_id=?1 AND step NOT IN ('complete','abandoned','superseded')")?.query_map([repository_id],|r|r.get::<_,String>(0))?.collect::<Result<Vec<_>,_>>()?;
    let mut admitted = None;
    for id in ids {
        let operation = OperationId::parse(&id).map_err(recovery_error)?;
        let r = closure::record::read_record(db, repository_id, operation)
            .map_err(recovery_error)?
            .ok_or_else(|| ReadError::new(ResultCode::RecoveryRequired))?;
        if kind != AuthoringKind::Ticket
            || r.item_id != *item
            || r.step != closure::record::ClosureStep::Closing
            || r.closure_kind != Some(closure::record::ClosureKind::Record)
            || admitted.is_some()
        {
            return Err(ReadError::new(ResultCode::RecoveryRequired));
        }
        let checkpoint = r
            .closure_oid
            .ok_or_else(|| ReadError::new(ResultCode::RecoveryRequired))?;
        closure::checkpoint::checkpoint_authority(git, item, checkpoint).map_err(recovery_error)?;
        let head = git2::Oid::from_str(&observation.context_head).map_err(recovery_error)?;
        if head != checkpoint && !git.graph_descendant_of(head, checkpoint).unwrap_or(false) {
            return Err(ReadError::new(ResultCode::RecoveryRequired));
        }
        let path = PathBuf::from(format!(".manyhands/tickets/{item}/ticket.md"));
        let closed = |oid| -> Result<crate::canonical::Ticket, ReadError> {
            let commit = git.find_commit(oid).map_err(recovery_error)?;
            let entry = commit
                .tree()
                .map_err(recovery_error)?
                .get_path(&path)
                .map_err(recovery_error)?;
            if entry.filemode() != 0o100644 {
                return Err(ReadError::new(ResultCode::RecoveryRequired));
            }
            let blob = git.find_blob(entry.id()).map_err(recovery_error)?;
            match crate::canonical::parse_item(
                &path,
                std::str::from_utf8(blob.content()).map_err(recovery_error)?,
            ) {
                Ok(crate::canonical::CanonicalItem::Ticket(ticket)) if ticket.id == *item => {
                    Ok(ticket)
                }
                _ => Err(ReadError::new(ResultCode::RecoveryRequired)),
            }
        };
        let original = closed(checkpoint)?;
        let current = closed(head)?;
        if original.closed_at != r.closed_at
            || original.closed_at.is_none()
            || original.closed_by.is_none()
            || original.closed_at != current.closed_at
            || original.closed_by != current.closed_by
        {
            return Err(ReadError::new(ResultCode::RecoveryRequired));
        }
        admitted = Some((operation, checkpoint));
    }
    Ok(admitted)
}

#[cfg(test)]
fn require_remote_released(db: &Connection, id: i64) -> Result<(), ReadError> {
    let retained:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records r WHERE r.repository_id=?1 AND (r.phase NOT IN ('completed','interrupted','cancelled','failed') OR r.index_pending!=0 OR r.reconciliation_required!=0 OR EXISTS(SELECT 1 FROM remote_integration_steps s WHERE s.operation_record_id=r.id AND s.phase!='applied') OR EXISTS(SELECT 1 FROM remote_resolution_attempts t WHERE t.operation_record_id=r.id AND (t.phase!='applied' OR NOT EXISTS(SELECT 1 FROM remote_resolution_index_artifacts a WHERE a.attempt_id=t.id AND a.phase='released'))) OR EXISTS(SELECT 1 FROM remote_integration_merge_metadata m JOIN remote_integration_steps s ON s.id=m.integration_step_id WHERE s.operation_record_id=r.id AND m.phase!='retired')))",[id],|r|r.get(0))?;
    if retained {
        Err(ReadError::new(ResultCode::RecoveryRequired))
    } else {
        Ok(())
    }
}

pub(in super::super) fn unfinished_context(
    db: &Connection,
    id: i64,
    item: &ItemId,
) -> Result<bool, RepositoryError> {
    audit(db)?;
    db.query_row("SELECT EXISTS(SELECT 1 FROM local_context_sync_records WHERE repository_id=?1 AND item_id=?2 AND state NOT IN ('complete','cancelled','abandoned'))",params![id,item.to_string()],|r|r.get(0)).map_err(|_|failure())
}

pub(in super::super) fn require_no_pending(
    db: &Connection,
    id: i64,
) -> Result<(), RepositoryError> {
    audit(db)?;
    let pending:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM local_context_sync_records WHERE repository_id=?1 AND state NOT IN ('complete','cancelled','abandoned'))",[id],|r|r.get(0)).map_err(|_|failure())?;
    if pending { Err(failure()) } else { Ok(()) }
}

pub(in super::super) fn refresh_allowed(
    db: &Connection,
    operation: &str,
    root: &std::path::Path,
) -> Result<bool, RepositoryError> {
    // Absence grants ordinary refresh permission only after proving there is
    // no local-sync envelope whose companion authority was lost.
    audit(db)?;
    let row:Option<(bool,bool,String)>=db.query_row("SELECT integration_proven,artifacts_released,state FROM local_context_sync_records WHERE operation_ulid=?1",[operation],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|_|failure())?;
    match row {
        None => Ok(!owned_identity(db, operation)?),
        Some((integrated, released, state)) => {
            let matches_root:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM local_context_sync_records s JOIN repositories r ON r.id=s.repository_id WHERE s.operation_ulid=?1 AND r.root_path=?2)",params![operation,root.to_str()],|r|r.get(0)).map_err(|_|failure())?;
            Ok(integrated
                && released
                && matches_root
                && matches!(state.as_str(), "integrated" | "index_pending" | "complete"))
        }
    }
}
