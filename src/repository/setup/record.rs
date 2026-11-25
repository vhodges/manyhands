//! The setup record: the one pending setup a repository holds, in
//! `repository_setups`. Storage only: no transport, Git effect, prompt or
//! lease. The record holds IDs, names, OIDs, a timestamp, an endpoint digest
//! and the intended configuration bytes; never a remote URL, a credential or
//! a commit identity. The endpoints are a digest, and the configuration
//! holds what the configuration holds (a remote's name, a forge's address).
//!
//! The table is keyed by the repository, so a second setup in one clone is
//! impossible, and it goes with the registration by cascade. Its state is
//! `recorded` until `published_oid` is set, then `published`.
use super::super::{RepositoryError, mutation::RequestId, remote::state};
use crate::canonical::ItemId;
use git2::Oid;
use rusqlite::{Connection, Row, Transaction, params};

/// What a setup does to the repository's configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SetupKind {
    /// Onboards a repository that has no configuration.
    Enable,
    /// Changes a setting of a repository that has one.
    Change,
}

impl SetupKind {
    /// The name as the table stores it.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Enable => "enable",
            Self::Change => "change",
        }
    }

    fn parse(stored: &str) -> Option<Self> {
        match stored {
            "enable" => Some(Self::Enable),
            "change" => Some(Self::Change),
            _ => None,
        }
    }
}

/// How far a setup has come: `recorded` until the remote is seen to hold
/// its commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SetupState {
    Recorded,
    Published,
}

impl SetupState {
    /// The name the contract publishes.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Recorded => "recorded",
            Self::Published => "published",
        }
    }
}

/// One repository's pending setup. The branch is
/// `manyhands/setup/<setup_id>`.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct SetupRecord {
    pub(crate) setup_id: ItemId,
    pub(crate) kind: SetupKind,
    pub(crate) remote_name: String,
    pub(crate) primary_branch: String,
    /// The setup remote's fetch and push endpoints, as
    /// `remote_polling_state` digests them.
    pub(crate) endpoint_digest: [u8; 32],
    /// The intended configuration file, the serializer's output.
    pub(crate) configuration: Vec<u8>,
    /// A change only: the primary's configuration blob when it began.
    pub(crate) base_configuration_oid: Option<Oid>,
    /// The base and the setup commit, once made.
    pub(crate) base_oid: Option<Oid>,
    pub(crate) commit_oid: Option<Oid>,
    /// The setup commit, once the remote is seen to hold it.
    pub(crate) published_oid: Option<Oid>,
    pub(crate) request_id: RequestId,
    pub(crate) created_at: i64,
}

impl SetupRecord {
    pub(crate) fn state(&self) -> SetupState {
        if self.published_oid.is_some() {
            SetupState::Published
        } else {
            SetupState::Recorded
        }
    }
}

impl std::fmt::Debug for SetupRecord {
    /// Leaves out the configuration bytes and the digest.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SetupRecord")
            .field("setup_id", &self.setup_id)
            .field("kind", &self.kind)
            .field("remote_name", &self.remote_name)
            .field("primary_branch", &self.primary_branch)
            .field("base_configuration_oid", &self.base_configuration_oid)
            .field("base_oid", &self.base_oid)
            .field("commit_oid", &self.commit_oid)
            .field("published_oid", &self.published_oid)
            .field("request_id", &self.request_id)
            .field("created_at", &self.created_at)
            .finish_non_exhaustive()
    }
}

/// The table setups are kept in.
pub(in super::super) const TABLE: &str = "repository_setups";

/// The marker of the migration that creates the table.
pub(in super::super) const MARKER: &str = "wave_06_repository_setup_v1";

/// No object's name contains "request": the replay test asserts it.
///
/// `published_oid IS commit_oid`, where the design wrote `=`: with no commit
/// recorded, `=` is null and a CHECK lets it pass, which would admit a
/// published commit that was never made.
pub(in super::super) const SCHEMA: &str = "CREATE TABLE repository_setups (
    repository_id INTEGER PRIMARY KEY REFERENCES repositories(id) ON DELETE CASCADE,
    setup_id TEXT NOT NULL UNIQUE CHECK(length(setup_id)=26),
    kind TEXT NOT NULL CHECK(kind IN ('enable','change')),
    remote_name TEXT NOT NULL,
    primary_branch TEXT NOT NULL,
    endpoint_digest BLOB NOT NULL CHECK(typeof(endpoint_digest)='blob' AND length(endpoint_digest)=32),
    configuration BLOB NOT NULL,
    base_configuration_oid TEXT CHECK(length(base_configuration_oid)=40 AND base_configuration_oid NOT GLOB '*[^0-9a-f]*' AND base_configuration_oid!='0000000000000000000000000000000000000000'),
    base_oid TEXT CHECK(length(base_oid)=40 AND base_oid NOT GLOB '*[^0-9a-f]*' AND base_oid!='0000000000000000000000000000000000000000'),
    commit_oid TEXT CHECK(length(commit_oid)=40 AND commit_oid NOT GLOB '*[^0-9a-f]*' AND commit_oid!='0000000000000000000000000000000000000000'),
    published_oid TEXT CHECK(length(published_oid)=40 AND published_oid NOT GLOB '*[^0-9a-f]*' AND published_oid!='0000000000000000000000000000000000000000'),
    request_id TEXT NOT NULL CHECK(length(request_id)=26),
    created_at INTEGER NOT NULL,
    CHECK((kind='change')=(base_configuration_oid IS NOT NULL)),
    CHECK((base_oid IS NULL)=(commit_oid IS NULL)),
    CHECK(published_oid IS NULL OR published_oid IS commit_oid)
)";

/// Whether the registry has the table.
pub(in super::super) fn exists(db: &Connection) -> Result<bool, RepositoryError> {
    db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        [TABLE],
        |row| row.get(0),
    )
    .map_err(RepositoryError::sqlite)
}

/// Fails with `recovery_required` when the table is not exactly the schema
/// this build creates.
pub(in super::super) fn validate_schema(db: &Connection) -> Result<(), RepositoryError> {
    state::validate_schema_object(db, "TABLE", TABLE, SCHEMA)
}

/// The setup of `repository_id`, if it has one.
///
/// Whoever might take the setup lock reads this first, so a repository with
/// no record never creates the lock file.
pub(crate) fn read(
    db: &Connection,
    repository_id: i64,
) -> Result<Option<SetupRecord>, RepositoryError> {
    let mut statement = db
        .prepare(
            "SELECT setup_id, kind, remote_name, primary_branch, endpoint_digest, configuration,
                    base_configuration_oid, base_oid, commit_oid, published_oid, request_id,
                    created_at
               FROM repository_setups WHERE repository_id = ?1",
        )
        .map_err(|_| state::recovery_required())?;
    let mut rows = statement
        .query([repository_id])
        .map_err(|_| state::recovery_required())?;
    rows.next()
        .map_err(|_| state::recovery_required())?
        .map(decode)
        .transpose()
}

fn corrupt<E>(_: E) -> RepositoryError {
    state::recovery_required()
}

/// A stored row as a record; a row that is none is `recovery_required`.
fn decode(row: &Row<'_>) -> Result<SetupRecord, RepositoryError> {
    let oid = |name: &str| -> Result<Option<Oid>, RepositoryError> {
        row.get::<_, Option<String>>(name)
            .map_err(corrupt)?
            .map(|text| Oid::from_str(&text).map_err(corrupt))
            .transpose()
    };
    let record = SetupRecord {
        setup_id: row
            .get::<_, String>("setup_id")
            .map_err(corrupt)?
            .parse()
            .map_err(corrupt)?,
        kind: SetupKind::parse(&row.get::<_, String>("kind").map_err(corrupt)?)
            .ok_or_else(state::recovery_required)?,
        remote_name: row.get("remote_name").map_err(corrupt)?,
        primary_branch: row.get("primary_branch").map_err(corrupt)?,
        endpoint_digest: row
            .get::<_, Vec<u8>>("endpoint_digest")
            .map_err(corrupt)?
            .try_into()
            .map_err(corrupt)?,
        configuration: row.get("configuration").map_err(corrupt)?,
        base_configuration_oid: oid("base_configuration_oid")?,
        base_oid: oid("base_oid")?,
        commit_oid: oid("commit_oid")?,
        published_oid: oid("published_oid")?,
        request_id: RequestId::parse(&row.get::<_, String>("request_id").map_err(corrupt)?)
            .map_err(corrupt)?,
        created_at: row.get("created_at").map_err(corrupt)?,
    };
    Ok(record)
}

/// Records `record` as the setup of `repository_id`. Fails when the
/// repository already has one, or when the record breaks a rule of the
/// table.
pub(crate) fn insert(
    tx: &Transaction<'_>,
    repository_id: i64,
    record: &SetupRecord,
) -> Result<(), RepositoryError> {
    tx.execute(
        "INSERT INTO repository_setups (
            repository_id, setup_id, kind, remote_name, primary_branch, endpoint_digest,
            configuration, base_configuration_oid, base_oid, commit_oid, published_oid,
            request_id, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            repository_id,
            record.setup_id.to_string(),
            record.kind.as_str(),
            record.remote_name,
            record.primary_branch,
            record.endpoint_digest.as_slice(),
            record.configuration,
            record.base_configuration_oid.map(|oid| oid.to_string()),
            record.base_oid.map(|oid| oid.to_string()),
            record.commit_oid.map(|oid| oid.to_string()),
            record.published_oid.map(|oid| oid.to_string()),
            record.request_id.to_string(),
            record.created_at,
        ],
    )
    .map(drop)
    .map_err(RepositoryError::sqlite)
}

/// Records the base and the setup commit made on it. Repeating the same
/// pair is a no-op; a different pair, or no record, fails.
pub(crate) fn record_commit(
    tx: &Transaction<'_>,
    repository_id: i64,
    base_oid: Oid,
    commit_oid: Oid,
) -> Result<(), RepositoryError> {
    let changed = tx
        .execute(
            "UPDATE repository_setups SET base_oid = ?2, commit_oid = ?3
              WHERE repository_id = ?1
                AND (base_oid IS NULL OR (base_oid = ?2 AND commit_oid = ?3))",
            params![repository_id, base_oid.to_string(), commit_oid.to_string()],
        )
        .map_err(RepositoryError::sqlite)?;
    (changed == 1)
        .then_some(())
        .ok_or_else(state::recovery_required)
}

/// Records that the remote holds the setup commit. The commit must be the
/// recorded one; repeating it is a no-op.
pub(crate) fn record_published(
    tx: &Transaction<'_>,
    repository_id: i64,
    published_oid: Oid,
) -> Result<(), RepositoryError> {
    let changed = tx
        .execute(
            "UPDATE repository_setups SET published_oid = ?2
              WHERE repository_id = ?1 AND commit_oid = ?2
                AND (published_oid IS NULL OR published_oid = ?2)",
            params![repository_id, published_oid.to_string()],
        )
        .map_err(RepositoryError::sqlite)?;
    (changed == 1)
        .then_some(())
        .ok_or_else(state::recovery_required)
}

/// Removes the setup of `repository_id`. Whether there was one.
pub(crate) fn delete(tx: &Transaction<'_>, repository_id: i64) -> Result<bool, RepositoryError> {
    tx.execute(
        "DELETE FROM repository_setups WHERE repository_id = ?1",
        [repository_id],
    )
    .map(|deleted| deleted != 0)
    .map_err(RepositoryError::sqlite)
}

#[cfg(test)]
#[path = "record_tests.rs"]
mod tests;
