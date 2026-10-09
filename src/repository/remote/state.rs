use std::{fmt, time::Duration};

use git2::Oid;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use std::path::Path;

use super::{
    RemoteRefClassification, RemoteRefPlan, RemoteRefTarget,
    merge::{ConfirmedCommitIdentity, IntegrationStage},
};
use crate::repository::{RepositoryError, RepositoryErrorKind, RepositoryOperation};
use crate::{canonical::ItemId, repository::AuthoringKind};

pub(in super::super) fn recovery_required() -> RepositoryError {
    RepositoryError::new(
        RepositoryOperation::RepositorySnapshot,
        None,
        RepositoryErrorKind::RecoveryRequired,
        "remote state requires recovery",
    )
}

pub(in super::super) fn migrate(transaction: &Transaction<'_>) -> Result<(), RepositoryError> {
    let table_count: i64 = transaction.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('remote_polling_state','remote_observation_batches','remote_ref_observations','remote_context_states','remote_operation_records')",[],|row|row.get(0)).map_err(|_| recovery_required())?;
    if table_count != 0 && table_count != 5 {
        return Err(recovery_required());
    }
    if table_count == 5 {
        let missing_policy: bool = transaction.query_row("SELECT EXISTS(SELECT 1 FROM repositories LEFT JOIN remote_polling_state ON repositories.id=remote_polling_state.repository_id WHERE remote_polling_state.repository_id IS NULL)",[],|row|row.get(0)).map_err(|_| recovery_required())?;
        if missing_policy {
            return Err(recovery_required());
        }
    }
    let history_unknown = transaction
        .path()
        .filter(|path| !path.is_empty())
        .and_then(|path| Path::new(path).parent())
        .map(crate::repository::recovery::remote_history_lost)
        .transpose()
        .map_err(|_| recovery_required())?
        .unwrap_or(false);
    transaction.execute_batch(&format!("
        CREATE TABLE IF NOT EXISTS remote_polling_state (
            repository_id INTEGER PRIMARY KEY REFERENCES repositories(id) ON DELETE CASCADE,
            enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0,1)),
            paused INTEGER NOT NULL DEFAULT 0 CHECK(paused IN (0,1)),
            interval_seconds INTEGER NOT NULL DEFAULT 300 CHECK(interval_seconds BETWEEN 60 AND 3600),
            automatic_backoff_seconds INTEGER CHECK(automatic_backoff_seconds BETWEEN 60 AND 900),
            recovery_suspended INTEGER NOT NULL DEFAULT {history} CHECK(recovery_suspended IN (0,1)),
            history_unknown INTEGER NOT NULL DEFAULT {history} CHECK(history_unknown IN (0,1)),
            configuration_generation INTEGER NOT NULL DEFAULT 0 CHECK(configuration_generation >= 0),
            endpoint_digest BLOB CHECK(endpoint_digest IS NULL OR (typeof(endpoint_digest)='blob' AND length(endpoint_digest)=32)),
            remote_name TEXT, primary_branch TEXT,
            latest_outcome TEXT CHECK(latest_outcome IN ('completed','configuration_required','selected_key_unavailable','unlock_required','host_approval_required','transport_unavailable','protocol_rejected','cancelled','repository_unavailable')),
            CHECK ((remote_name IS NULL) = (primary_branch IS NULL))
        );
        CREATE TRIGGER IF NOT EXISTS remote_policy_on_registration AFTER INSERT ON repositories BEGIN
            INSERT INTO remote_polling_state(repository_id) VALUES (NEW.id);
        END;
        CREATE TABLE IF NOT EXISTS remote_observation_batches (
            id INTEGER PRIMARY KEY,
            repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
            remote_name TEXT NOT NULL, primary_branch TEXT NOT NULL,
            configuration_generation INTEGER NOT NULL CHECK(configuration_generation >= 0),
            observed_at INTEGER NOT NULL CHECK(observed_at >= 0),
            is_current INTEGER NOT NULL CHECK(is_current IN (0,1)),
            UNIQUE(repository_id,id)
        );
        CREATE UNIQUE INDEX IF NOT EXISTS remote_current_batch ON remote_observation_batches(repository_id) WHERE is_current=1;
        CREATE INDEX IF NOT EXISTS remote_batches_repository ON remote_observation_batches(repository_id,observed_at);
        CREATE TABLE IF NOT EXISTS remote_ref_observations (
            batch_id INTEGER NOT NULL REFERENCES remote_observation_batches(id) ON DELETE CASCADE,
            ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
            remote_ref TEXT, tracking_ref TEXT,
            classification TEXT NOT NULL CHECK(classification IN ('primary','context','malformed')),
            advertised_oid TEXT NOT NULL CHECK(length(advertised_oid)=40 AND advertised_oid NOT GLOB '*[^0-9a-f]*'),
            tracking_oid TEXT CHECK(length(tracking_oid)=40 AND tracking_oid NOT GLOB '*[^0-9a-f]*'),
            PRIMARY KEY(batch_id,ordinal), UNIQUE(batch_id,remote_ref),
            CHECK ((remote_ref IS NULL) = (tracking_ref IS NULL)),
            CHECK (classification='malformed' OR remote_ref IS NOT NULL)
        );
        CREATE TABLE IF NOT EXISTS remote_context_states (
            repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
            remote_ref TEXT NOT NULL, tracking_ref TEXT NOT NULL,
            kind TEXT NOT NULL CHECK(kind IN ('document','ticket')), item_id TEXT NOT NULL,
            last_advertised_oid TEXT CHECK(length(last_advertised_oid)=40 AND last_advertised_oid NOT GLOB '*[^0-9a-f]*'),
            tracking_oid TEXT CHECK(length(tracking_oid)=40 AND tracking_oid NOT GLOB '*[^0-9a-f]*'),
            publication_evidence TEXT NOT NULL CHECK(publication_evidence IN ('never_published','observed_published','history_unknown')),
            state TEXT NOT NULL CHECK(state IN ('observed','unmaterialized','malformed','remotely_deleted','history_unknown')),
            observed_at INTEGER NOT NULL CHECK(observed_at >= 0),
            PRIMARY KEY(repository_id,remote_ref)
        );
        CREATE INDEX IF NOT EXISTS remote_context_item ON remote_context_states(repository_id,kind,item_id);
    ", history=i32::from(history_unknown))).map_err(|_| recovery_required())?;
    transaction
        .execute_batch(REMOTE_OPERATION_SCHEMA)
        .map_err(|_| recovery_required())?;
    if table_count == 0 {
        transaction
            .execute(
                "INSERT INTO remote_polling_state(repository_id) SELECT id FROM repositories",
                [],
            )
            .map_err(|_| recovery_required())?;
    }
    // Additive upgrade from the immutable Task 2 audit envelope.
    let columns = transaction
        .prepare("PRAGMA table_info(remote_operation_records)")
        .and_then(|mut query| {
            query
                .query_map([], |row| row.get::<_, String>(1))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| recovery_required())?;
    let reservation_columns = ["yield_requested", "cancel_requested", "owner_epoch"];
    let existing_reservation_columns = reservation_columns
        .iter()
        .filter(|name| columns.iter().any(|column| column == **name))
        .count();
    if existing_reservation_columns != 0
        && existing_reservation_columns != reservation_columns.len()
    {
        return Err(recovery_required());
    }
    for (name, definition) in [
        (
            "yield_requested",
            "INTEGER NOT NULL DEFAULT 0 CHECK(yield_requested IN (0,1))",
        ),
        (
            "cancel_requested",
            "INTEGER NOT NULL DEFAULT 0 CHECK(cancel_requested IN (0,1))",
        ),
        (
            "owner_epoch",
            "INTEGER NOT NULL DEFAULT 0 CHECK(owner_epoch >= 0)",
        ),
    ] {
        if !columns.iter().any(|column| column == name) {
            transaction
                .execute_batch(&format!(
                    "ALTER TABLE remote_operation_records ADD COLUMN {name} {definition}"
                ))
                .map_err(|_| recovery_required())?;
        }
    }
    migrate_synchronization(transaction)?;
    migrate_merge_evidence(transaction)?;
    transaction
        .execute_batch(REMOTE_OPERATION_INDEXES)
        .map_err(|_| recovery_required())?;
    let has_digest = table_count == 0
        || transaction
            .prepare("PRAGMA table_info(remote_polling_state)")
            .and_then(|mut query| {
                query
                    .query_map([], |row| row.get::<_, String>(1))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|_| recovery_required())?
            .iter()
            .any(|name| name == "endpoint_digest");
    if !has_digest {
        transaction.execute_batch("ALTER TABLE remote_polling_state ADD COLUMN endpoint_digest BLOB CHECK(endpoint_digest IS NULL OR (typeof(endpoint_digest)='blob' AND length(endpoint_digest)=32))")
            .map_err(|_| recovery_required())?;
    }
    Ok(())
}

const REMOTE_OPERATION_SCHEMA: &str = r#"        CREATE TABLE IF NOT EXISTS remote_operation_records (
            id INTEGER PRIMARY KEY,
            repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
            operation_ulid TEXT NOT NULL, configuration_generation INTEGER NOT NULL CHECK(configuration_generation >= 0),
            remote_name TEXT NOT NULL, primary_branch TEXT NOT NULL,
            primary_ref TEXT NOT NULL, primary_tracking_ref TEXT NOT NULL,
            context_ref TEXT, context_tracking_ref TEXT, kind TEXT CHECK(kind IN ('document','ticket')),
            item_id TEXT, local_branch TEXT,
            action TEXT NOT NULL CHECK(action IN ('poll','synchronize_context','synchronize_primary','promote','close')),
            priority TEXT NOT NULL CHECK(priority IN ('poll','manual')),
            phase TEXT NOT NULL CHECK(phase IN ('reserved','advertising','persisting','fetch_prepared','fetch_observed','local_prepared','local_fast_forwarded','push_prepared','push_returned','push_verified','reconciling','completed','interrupted','cancelled','failed')),
            completed_step TEXT CHECK(completed_step IN ('before_transport','after_advertisement','between_observations','before_batch_commit','after_batch_commit','before_local_mutation','before_fetch','after_fetch','before_local_update','after_local_update','before_push','after_push_return','after_push_verification','before_discovery')),
            local_oid TEXT CHECK(length(local_oid)=40 AND local_oid NOT GLOB '*[^0-9a-f]*'),
            tracking_oid TEXT CHECK(length(tracking_oid)=40 AND tracking_oid NOT GLOB '*[^0-9a-f]*'),
            advertised_oid TEXT CHECK(length(advertised_oid)=40 AND advertised_oid NOT GLOB '*[^0-9a-f]*'),
            created_at INTEGER NOT NULL CHECK(created_at >= 0), updated_at INTEGER NOT NULL CHECK(updated_at >= created_at),
            outcome TEXT CHECK(outcome IN ('completed','configuration_required','selected_key_unavailable','unlock_required','host_approval_required','transport_unavailable','protocol_rejected','cancelled','repository_unavailable')),
            sync_checkpoint TEXT CHECK(sync_checkpoint IN ('fetch_prepared','fetch_observed','local_prepared','local_fast_forwarded','push_prepared','push_returned','push_verified','discovery_pending')),
            expected_oid TEXT CHECK(length(expected_oid)=40 AND expected_oid NOT GLOB '*[^0-9a-f]*'),
            primary_tracking_oid TEXT CHECK(length(primary_tracking_oid)=40 AND primary_tracking_oid NOT GLOB '*[^0-9a-f]*'),
            push_oid TEXT CHECK(length(push_oid)=40 AND push_oid NOT GLOB '*[^0-9a-f]*'),
            push_advertised_oid TEXT CHECK(length(push_advertised_oid)=40 AND push_advertised_oid NOT GLOB '*[^0-9a-f]*'),
            authoritative_kind TEXT CHECK(authoritative_kind IN ('published','already_current')),
            authoritative_oid TEXT CHECK(length(authoritative_oid)=40 AND authoritative_oid NOT GLOB '*[^0-9a-f]*'),
            index_pending INTEGER NOT NULL DEFAULT 0 CHECK(index_pending IN (0,1)),
            reconciliation_required INTEGER NOT NULL DEFAULT 0 CHECK(reconciliation_required IN (0,1)),
            CHECK((authoritative_kind IS NULL)=(authoritative_oid IS NULL)),
            CHECK(index_pending=0 OR authoritative_kind IS NOT NULL),
            UNIQUE(repository_id,operation_ulid),
            CHECK((context_ref IS NULL)=(context_tracking_ref IS NULL)),
            CHECK((context_ref IS NULL)=(kind IS NULL)), CHECK((kind IS NULL)=(item_id IS NULL)),
            CHECK((action IN ('poll','synchronize_primary'))=(context_ref IS NULL)),
            CHECK((action='poll')=(local_branch IS NULL))
        );
"#;
const REMOTE_OPERATION_INDEXES: &str = r#"        CREATE INDEX IF NOT EXISTS remote_operations_repository ON remote_operation_records(repository_id,phase,created_at);
        CREATE TRIGGER IF NOT EXISTS remote_operation_target_immutable BEFORE UPDATE OF
            repository_id,operation_ulid,configuration_generation,remote_name,primary_branch,primary_ref,primary_tracking_ref,context_ref,context_tracking_ref,kind,item_id,local_branch,action,priority,created_at
            ON remote_operation_records BEGIN SELECT RAISE(ABORT,'immutable remote operation target'); END;
        CREATE UNIQUE INDEX IF NOT EXISTS remote_active_reservation ON remote_operation_records(repository_id) WHERE phase IN ('reserved','advertising','persisting','fetch_prepared','fetch_observed','local_prepared','local_fast_forwarded','push_prepared','push_returned','push_verified','reconciling');
"#;

// A pinned pass cannot be deleted independently of its window. The deferred
// batch FK still permits the complete explicit repository-registration cascade.
const MERGE_EVIDENCE_SCHEMA: &str = r#"
        CREATE TABLE remote_integration_windows (
            operation_record_id INTEGER NOT NULL REFERENCES remote_operation_records(id) ON DELETE CASCADE,
            number INTEGER NOT NULL CHECK(number BETWEEN 0 AND 4294967295),
            configuration_generation INTEGER NOT NULL CHECK(configuration_generation >= 0),
            owner_epoch INTEGER NOT NULL CHECK(owner_epoch >= 0),
            kind TEXT NOT NULL CHECK(kind IN ('legacy','pinned')),
            observation_batch_id INTEGER REFERENCES remote_observation_batches(id) ON DELETE NO ACTION DEFERRABLE INITIALLY DEFERRED,
            local_oid TEXT CHECK(length(local_oid)=40 AND local_oid NOT GLOB '*[^0-9a-f]*'),
            primary_oid TEXT CHECK(length(primary_oid)=40 AND primary_oid NOT GLOB '*[^0-9a-f]*'),
            context_oid TEXT CHECK(length(context_oid)=40 AND context_oid NOT GLOB '*[^0-9a-f]*'),
            PRIMARY KEY(operation_record_id,number),
            CHECK((number=0 AND kind='legacy' AND observation_batch_id IS NULL AND local_oid IS NULL AND primary_oid IS NULL AND context_oid IS NULL) OR (number>0 AND kind='pinned' AND observation_batch_id IS NOT NULL AND local_oid IS NOT NULL AND primary_oid IS NOT NULL))
        );
        CREATE INDEX remote_integration_windows_batch ON remote_integration_windows(observation_batch_id);
        CREATE TABLE remote_integration_steps (
            id INTEGER PRIMARY KEY,
            operation_record_id INTEGER NOT NULL REFERENCES remote_operation_records(id) ON DELETE CASCADE,
            configuration_generation INTEGER NOT NULL CHECK(configuration_generation >= 0),
            owner_epoch INTEGER NOT NULL CHECK(owner_epoch >= 0),
            ordinal INTEGER NOT NULL CHECK(ordinal BETWEEN 0 AND 1),
            stage TEXT NOT NULL CHECK(stage IN ('context','primary')),
            local_oid TEXT NOT NULL CHECK(length(local_oid)=40 AND local_oid NOT GLOB '*[^0-9a-f]*'),
            incoming_oid TEXT NOT NULL CHECK(length(incoming_oid)=40 AND incoming_oid NOT GLOB '*[^0-9a-f]*'),
            baseline_tree_oid TEXT NOT NULL CHECK(length(baseline_tree_oid)=40 AND baseline_tree_oid NOT GLOB '*[^0-9a-f]*'),
            baseline_index_digest BLOB NOT NULL CHECK(typeof(baseline_index_digest)='blob' AND length(baseline_index_digest)=32),
            candidate_oid TEXT CHECK(length(candidate_oid)=40 AND candidate_oid NOT GLOB '*[^0-9a-f]*'),
            result_oid TEXT CHECK(length(result_oid)=40 AND result_oid NOT GLOB '*[^0-9a-f]*'),
            observed_tree_oid TEXT CHECK(length(observed_tree_oid)=40 AND observed_tree_oid NOT GLOB '*[^0-9a-f]*'),
            conflict_digest BLOB CHECK(conflict_digest IS NULL OR (typeof(conflict_digest)='blob' AND length(conflict_digest)=32)),
            phase TEXT NOT NULL CHECK(phase IN ('prepared','applying','conflict_pending','resolution_prepared','commit_prepared','applied','recovery_required')),
            window_number INTEGER NOT NULL DEFAULT 0 CHECK(window_number BETWEEN 0 AND 4294967295),
            FOREIGN KEY(operation_record_id,window_number) REFERENCES remote_integration_windows(operation_record_id,number) ON DELETE CASCADE,
            UNIQUE(operation_record_id,window_number,ordinal), UNIQUE(operation_record_id,window_number,stage),
            CHECK(phase!='conflict_pending' OR conflict_digest IS NOT NULL),
            CHECK(phase!='commit_prepared' OR candidate_oid IS NOT NULL),
            CHECK(phase!='applied' OR (result_oid IS NOT NULL AND observed_tree_oid IS NOT NULL))
        );
        CREATE INDEX remote_integration_steps_operation ON remote_integration_steps(operation_record_id,window_number,ordinal);
        CREATE INDEX remote_integration_steps_pending ON remote_integration_steps(operation_record_id) WHERE phase='conflict_pending';
        CREATE TABLE remote_identity_confirmations (
            id INTEGER PRIMARY KEY,
            confirmation_ulid TEXT NOT NULL UNIQUE,
            operation_record_id INTEGER NOT NULL REFERENCES remote_operation_records(id) ON DELETE CASCADE,
            configuration_generation INTEGER NOT NULL CHECK(configuration_generation >= 0),
            owner_epoch INTEGER NOT NULL CHECK(owner_epoch >= 0),
            input_digest BLOB NOT NULL CHECK(typeof(input_digest)='blob' AND length(input_digest)=32),
            configuration_digest BLOB NOT NULL CHECK(typeof(configuration_digest)='blob' AND length(configuration_digest)=32),
            phase TEXT NOT NULL CHECK(phase IN ('prepared','applying','applied','recovery_required')),
            applied_configuration_digest BLOB CHECK(applied_configuration_digest IS NULL OR (typeof(applied_configuration_digest)='blob' AND length(applied_configuration_digest)=32)),
            CHECK(phase!='applied' OR applied_configuration_digest IS NOT NULL)
        );
        CREATE INDEX remote_identity_confirmations_operation ON remote_identity_confirmations(operation_record_id);
        CREATE TABLE remote_resolution_attempts (
            id INTEGER PRIMARY KEY,
            attempt_ulid TEXT NOT NULL UNIQUE,
            operation_record_id INTEGER NOT NULL REFERENCES remote_operation_records(id) ON DELETE CASCADE,
            integration_step_id INTEGER NOT NULL REFERENCES remote_integration_steps(id) ON DELETE CASCADE,
            configuration_generation INTEGER NOT NULL CHECK(configuration_generation >= 0),
            owner_epoch INTEGER NOT NULL CHECK(owner_epoch >= 0),
            observation_digest BLOB NOT NULL CHECK(typeof(observation_digest)='blob' AND length(observation_digest)=32),
            input_digest BLOB NOT NULL CHECK(typeof(input_digest)='blob' AND length(input_digest)=32),
            preflight_digest BLOB NOT NULL CHECK(typeof(preflight_digest)='blob' AND length(preflight_digest)=32),
            identity_confirmation_id INTEGER REFERENCES remote_identity_confirmations(id) ON DELETE RESTRICT,
            candidate_oid TEXT CHECK(length(candidate_oid)=40 AND candidate_oid NOT GLOB '*[^0-9a-f]*'),
            checkpoint_oid TEXT CHECK(length(checkpoint_oid)=40 AND checkpoint_oid NOT GLOB '*[^0-9a-f]*'),
            phase TEXT NOT NULL CHECK(phase IN ('prepared','paths_applying','candidate_prepared','applied','recovery_required')),
            CHECK(phase NOT IN ('candidate_prepared','applied') OR candidate_oid IS NOT NULL),
            CHECK(phase!='applied' OR checkpoint_oid IS NOT NULL)
        );
        CREATE INDEX remote_resolution_attempts_operation ON remote_resolution_attempts(operation_record_id,integration_step_id);
        CREATE TABLE remote_resolution_paths (
            attempt_id INTEGER NOT NULL REFERENCES remote_resolution_attempts(id) ON DELETE CASCADE,
            ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
            path_digest BLOB NOT NULL CHECK(typeof(path_digest)='blob' AND length(path_digest)=32),
            expected_digest BLOB NOT NULL CHECK(typeof(expected_digest)='blob' AND length(expected_digest)=32),
            result_digest BLOB NOT NULL CHECK(typeof(result_digest)='blob' AND length(result_digest)=32),
            prewrite_digest BLOB NOT NULL CHECK(typeof(prewrite_digest)='blob' AND length(prewrite_digest)=32),
            base_blob_oid TEXT CHECK(length(base_blob_oid)=40 AND base_blob_oid NOT GLOB '*[^0-9a-f]*'),
            local_blob_oid TEXT CHECK(length(local_blob_oid)=40 AND local_blob_oid NOT GLOB '*[^0-9a-f]*'),
            incoming_blob_oid TEXT CHECK(length(incoming_blob_oid)=40 AND incoming_blob_oid NOT GLOB '*[^0-9a-f]*'),
            mode INTEGER NOT NULL CHECK(mode IN (33188,33261)),
            applied INTEGER NOT NULL DEFAULT 0 CHECK(applied IN (0,1)),
            PRIMARY KEY(attempt_id,ordinal), UNIQUE(attempt_id,path_digest)
        );
        CREATE TABLE remote_resolution_index_artifacts (
            attempt_id INTEGER PRIMARY KEY REFERENCES remote_resolution_attempts(id) ON DELETE CASCADE,
            device INTEGER NOT NULL CHECK(device >= 0),
            inode INTEGER NOT NULL CHECK(inode > 0),
            sentinel_digest BLOB NOT NULL CHECK(typeof(sentinel_digest)='blob' AND length(sentinel_digest)=32),
            baseline_digest BLOB NOT NULL CHECK(typeof(baseline_digest)='blob' AND length(baseline_digest)=32),
            baseline_device INTEGER NOT NULL CHECK(baseline_device >= 0),
            baseline_inode INTEGER NOT NULL CHECK(baseline_inode > 0),
            merge_head_digest BLOB CHECK(merge_head_digest IS NULL OR (typeof(merge_head_digest)='blob' AND length(merge_head_digest)=32)),
            merge_msg_digest BLOB CHECK(merge_msg_digest IS NULL OR (typeof(merge_msg_digest)='blob' AND length(merge_msg_digest)=32)),
            merge_mode_digest BLOB CHECK(merge_mode_digest IS NULL OR (typeof(merge_mode_digest)='blob' AND length(merge_mode_digest)=32)),
            ref_phase TEXT NOT NULL DEFAULT 'not_started' CHECK(ref_phase IN ('not_started','intent','observed')),
            output_device INTEGER CHECK(output_device >= 0),
            output_inode INTEGER CHECK(output_inode > 0),
            output_digest BLOB CHECK(output_digest IS NULL OR (typeof(output_digest)='blob' AND length(output_digest)=32)),
            phase TEXT NOT NULL CHECK(phase IN ('intent','published','release_intent','released')),
            CHECK((output_device IS NULL AND output_inode IS NULL AND output_digest IS NULL) OR (output_device IS NOT NULL AND output_inode IS NOT NULL AND output_digest IS NOT NULL)),
            CHECK(phase!='intent' OR (output_digest IS NULL AND ref_phase='not_started')),
            CHECK(ref_phase='not_started' OR output_digest IS NOT NULL),
            CHECK(phase NOT IN ('release_intent','released') OR (output_digest IS NOT NULL AND ref_phase='observed'))
        );
        CREATE TABLE remote_resolution_ref_log_artifacts (
            attempt_id INTEGER NOT NULL REFERENCES remote_resolution_index_artifacts(attempt_id) ON DELETE CASCADE,
            role TEXT NOT NULL CHECK(role IN ('baseline','transition')),
            device INTEGER NOT NULL CHECK(device >= 0),
            inode INTEGER NOT NULL CHECK(inode > 0),
            digest BLOB NOT NULL CHECK(typeof(digest)='blob' AND length(digest)=32),
            PRIMARY KEY(attempt_id,role)
        );
        CREATE TRIGGER remote_resolution_ref_log_artifact_immutable BEFORE UPDATE ON remote_resolution_ref_log_artifacts BEGIN SELECT RAISE(ABORT,'immutable ref log evidence'); END;
        CREATE TRIGGER remote_resolution_index_output_immutable BEFORE UPDATE OF output_device,output_inode,output_digest ON remote_resolution_index_artifacts WHEN OLD.output_digest IS NOT NULL BEGIN SELECT RAISE(ABORT,'immutable index output'); END;
        CREATE TRIGGER remote_resolution_index_artifact_immutable BEFORE UPDATE OF attempt_id,device,inode,sentinel_digest,baseline_digest,baseline_device,baseline_inode,merge_head_digest,merge_msg_digest,merge_mode_digest ON remote_resolution_index_artifacts BEGIN SELECT RAISE(ABORT,'immutable index ownership'); END;
        CREATE TRIGGER remote_integration_window_immutable BEFORE UPDATE ON remote_integration_windows BEGIN SELECT RAISE(ABORT,'immutable integration window'); END;
        CREATE TRIGGER remote_integration_step_immutable BEFORE UPDATE OF operation_record_id,configuration_generation,owner_epoch,window_number,ordinal,stage,local_oid,incoming_oid,baseline_tree_oid,baseline_index_digest ON remote_integration_steps BEGIN SELECT RAISE(ABORT,'immutable integration evidence'); END;
        CREATE TRIGGER remote_identity_confirmation_immutable BEFORE UPDATE OF confirmation_ulid,operation_record_id,configuration_generation,owner_epoch,input_digest,configuration_digest ON remote_identity_confirmations BEGIN SELECT RAISE(ABORT,'immutable identity evidence'); END;
        CREATE TRIGGER remote_resolution_attempt_immutable BEFORE UPDATE OF attempt_ulid,operation_record_id,integration_step_id,configuration_generation,owner_epoch,observation_digest,input_digest,preflight_digest,identity_confirmation_id ON remote_resolution_attempts BEGIN SELECT RAISE(ABORT,'immutable resolution evidence'); END;
        CREATE TRIGGER remote_resolution_path_immutable BEFORE UPDATE OF attempt_id,ordinal,path_digest,expected_digest,result_digest,prewrite_digest,base_blob_oid,local_blob_oid,incoming_blob_oid,mode ON remote_resolution_paths BEGIN SELECT RAISE(ABORT,'immutable resolution path evidence'); END;
"#;

fn migrate_merge_evidence(tx: &Transaction<'_>) -> Result<(), RepositoryError> {
    let tables = [
        "remote_integration_steps",
        "remote_identity_confirmations",
        "remote_resolution_attempts",
        "remote_resolution_paths",
    ];
    let present = tables
        .iter()
        .map(|table| {
            tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                [*table],
                |row| row.get::<_, bool>(0),
            )
            .map_err(|_| recovery_required())
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|present| *present)
        .count();
    if present != 0 && present != tables.len() {
        return Err(recovery_required());
    }
    if present == 0 {
        tx.execute_batch(MERGE_EVIDENCE_SCHEMA)
            .map_err(|_| recovery_required())?;
        tx.execute_batch("INSERT INTO remote_integration_windows(operation_record_id,number,configuration_generation,owner_epoch,kind) SELECT id,0,configuration_generation,0,'legacy' FROM remote_operation_records WHERE action IN ('synchronize_primary','synchronize_context');")
            .map_err(|_| recovery_required())?;
    } else {
        // The preflight digest was added after the initial Task 4 envelope.
        // An old in-flight attempt cannot honestly gain evidence that was never
        // observed, so fail closed rather than inventing a digest. Empty legacy
        // attempt tables are safely upgraded in place.
        let attempt_columns = tx
            .prepare("PRAGMA table_info(remote_resolution_attempts)")
            .and_then(|mut statement| {
                statement
                    .query_map([], |row| row.get::<_, String>(1))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|_| recovery_required())?;
        if !attempt_columns
            .iter()
            .any(|column| column == "preflight_digest")
        {
            let has_attempts: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM remote_resolution_attempts)",
                    [],
                    |row| row.get(0),
                )
                .map_err(|_| recovery_required())?;
            if has_attempts {
                return Err(recovery_required());
            }
            tx.execute_batch(
                "DROP TABLE remote_resolution_paths; DROP TABLE remote_resolution_attempts;",
            )
            .map_err(|_| recovery_required())?;
            for (kind, name) in [
                ("TABLE", "remote_resolution_attempts"),
                ("INDEX", "remote_resolution_attempts_operation"),
                ("TABLE", "remote_resolution_paths"),
                ("TRIGGER", "remote_resolution_attempt_immutable"),
                ("TRIGGER", "remote_resolution_path_immutable"),
            ] {
                let sql = merge_schema_object_sql(kind, name).ok_or_else(recovery_required)?;
                tx.execute_batch(sql).map_err(|_| recovery_required())?;
            }
        }
        // Per-path pre-write evidence is required to resume a partially
        // applied attempt without overwriting an external replacement.
        let path_columns = tx
            .prepare("PRAGMA table_info(remote_resolution_paths)")
            .and_then(|mut statement| {
                statement
                    .query_map([], |row| row.get::<_, String>(1))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|_| recovery_required())?;
        if !path_columns
            .iter()
            .any(|column| column == "prewrite_digest")
        {
            let has_attempts: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM remote_resolution_attempts)",
                    [],
                    |row| row.get(0),
                )
                .map_err(|_| recovery_required())?;
            if has_attempts {
                return Err(recovery_required());
            }
            tx.execute_batch("DROP TABLE remote_resolution_paths;")
                .map_err(|_| recovery_required())?;
            for (kind, name) in [
                ("TABLE", "remote_resolution_paths"),
                ("TRIGGER", "remote_resolution_path_immutable"),
            ] {
                let sql = merge_schema_object_sql(kind, name).ok_or_else(recovery_required)?;
                tx.execute_batch(sql).map_err(|_| recovery_required())?;
            }
        }
    }
    let artifact_present: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='remote_resolution_index_artifacts')", [], |row| row.get(0)).map_err(|_| recovery_required())?;
    if !artifact_present {
        // Legacy attempts retain their evidence but gain no inferred live-lock ownership.
        for (kind, name) in [
            ("TABLE", "remote_resolution_index_artifacts"),
            ("TRIGGER", "remote_resolution_index_artifact_immutable"),
            ("TRIGGER", "remote_resolution_index_output_immutable"),
        ] {
            tx.execute_batch(merge_schema_object_sql(kind, name).ok_or_else(recovery_required)?)
                .map_err(|_| recovery_required())?;
        }
    }
    let ref_log_present: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='remote_resolution_ref_log_artifacts')", [], |row| row.get(0)).map_err(|_| recovery_required())?;
    if !ref_log_present {
        // Additive: legacy candidates acquire no inferred log proof.
        for (kind, name) in [
            ("TABLE", "remote_resolution_ref_log_artifacts"),
            ("TRIGGER", "remote_resolution_ref_log_artifact_immutable"),
        ] {
            tx.execute_batch(merge_schema_object_sql(kind, name).ok_or_else(recovery_required)?)
                .map_err(|_| recovery_required())?;
        }
    }
    migrate_integration_windows(tx)?;
    validate_merge_evidence_schema(tx)
}

/// Validate the complete old envelope before rewriting its UNIQUE constraints.
/// Child tables are copied in dependency order with their original integer IDs;
/// foreign keys stay enabled throughout. No candidate, lock or effect proof is
/// synthesized. The temporary copies contain only the existing redacted evidence.
fn migrate_integration_windows(tx: &Transaction<'_>) -> Result<(), RepositoryError> {
    let columns = tx
        .prepare("PRAGMA table_info(remote_integration_steps)")
        .and_then(|mut query| {
            query
                .query_map([], |row| row.get::<_, String>(1))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| recovery_required())?;
    if columns.iter().any(|column| column == "window_number") {
        // A missing window table in the new schema is evidence loss, not an
        // invitation to recreate legacy rows on a later startup.
        return Ok(());
    }
    let window_present: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='remote_integration_windows')", [], |row| row.get(0)).map_err(|_| recovery_required())?;
    if window_present {
        return Err(recovery_required());
    }
    for &(kind, name) in MERGE_SCHEMA_OBJECTS {
        if let Some(expected) = legacy_merge_schema_object_sql(kind, name) {
            validate_schema_object(tx, kind, name, &expected)?;
        }
    }
    let copied = [
        "remote_integration_steps",
        "remote_resolution_attempts",
        "remote_resolution_paths",
        "remote_resolution_index_artifacts",
        "remote_resolution_ref_log_artifacts",
    ];
    for table in copied {
        if tx
            .prepare(&format!("PRAGMA foreign_key_check({table})"))
            .and_then(|mut query| query.exists([]))
            .map_err(|_| recovery_required())?
        {
            return Err(recovery_required());
        }
        tx.execute_batch(&format!(
            "CREATE TEMP TABLE {table}_window_upgrade AS SELECT * FROM {table};"
        ))
        .map_err(|_| recovery_required())?;
    }
    for table in copied.into_iter().rev() {
        tx.execute_batch(&format!("DROP TABLE {table};"))
            .map_err(|_| recovery_required())?;
    }
    for &(kind, name) in MERGE_SCHEMA_OBJECTS {
        if (kind == "TABLE" && name == "remote_integration_windows")
            || copied.contains(&name)
            || matches!(
                name,
                "remote_integration_steps_operation"
                    | "remote_integration_steps_pending"
                    | "remote_resolution_attempts_operation"
                    | "remote_integration_windows_batch"
                    | "remote_integration_window_immutable"
                    | "remote_integration_step_immutable"
                    | "remote_resolution_attempt_immutable"
                    | "remote_resolution_path_immutable"
                    | "remote_resolution_index_artifact_immutable"
                    | "remote_resolution_index_output_immutable"
                    | "remote_resolution_ref_log_artifact_immutable"
            )
        {
            tx.execute_batch(merge_schema_object_sql(kind, name).ok_or_else(recovery_required)?)
                .map_err(|_| recovery_required())?;
        }
    }
    tx.execute_batch("INSERT INTO remote_integration_windows(operation_record_id,number,configuration_generation,owner_epoch,kind) SELECT id,0,configuration_generation,0,'legacy' FROM remote_operation_records WHERE action IN ('synchronize_primary','synchronize_context');")
        .map_err(|_| recovery_required())?;
    for table in copied {
        let columns = tx
            .prepare(&format!("PRAGMA temp.table_info({table}_window_upgrade)"))
            .and_then(|mut query| {
                query
                    .query_map([], |row| row.get::<_, String>(1))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|_| recovery_required())?
            .join(",");
        tx.execute_batch(&format!(
            "INSERT INTO {table}({columns}) SELECT {columns} FROM temp.{table}_window_upgrade; DROP TABLE temp.{table}_window_upgrade;"
        ))
        .map_err(|_| recovery_required())?;
    }
    Ok(())
}

fn legacy_merge_schema_object_sql(kind: &str, name: &str) -> Option<String> {
    if matches!(
        name,
        "remote_integration_windows"
            | "remote_integration_windows_batch"
            | "remote_integration_window_immutable"
    ) {
        return None;
    }
    Some(
        merge_schema_object_sql(kind, name)?
            .replace("            window_number INTEGER NOT NULL DEFAULT 0 CHECK(window_number BETWEEN 0 AND 4294967295),\n", "")
            .replace("            FOREIGN KEY(operation_record_id,window_number) REFERENCES remote_integration_windows(operation_record_id,number) ON DELETE CASCADE,\n", "")
            .replace("UNIQUE(operation_record_id,window_number,ordinal), UNIQUE(operation_record_id,window_number,stage)", "UNIQUE(operation_record_id,ordinal), UNIQUE(operation_record_id,stage)")
            .replace("remote_integration_steps(operation_record_id,window_number,ordinal)", "remote_integration_steps(operation_record_id,ordinal)")
            .replace("owner_epoch,window_number,ordinal", "owner_epoch,ordinal"),
    )
}

fn merge_schema_object_sql(kind: &str, name: &str) -> Option<&'static str> {
    let prefix = format!("CREATE {kind} {name}");
    let start = MERGE_EVIDENCE_SCHEMA.find(&prefix)?;
    let statement = &MERGE_EVIDENCE_SCHEMA[start..];
    let end = if kind == "TRIGGER" {
        statement.find(" END;")? + " END;".len()
    } else {
        statement.find(';')? + 1
    };
    Some(&statement[..end])
}

fn normalized_schema_sql(sql: &str) -> String {
    sql.chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn validate_schema_object(
    connection: &Connection,
    kind: &str,
    name: &str,
    expected: &str,
) -> Result<(), RepositoryError> {
    let actual = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type=?1 AND name=?2",
            params![kind.to_ascii_lowercase(), name],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|_| recovery_required())?
        .flatten()
        .map(|sql| normalized_schema_sql(&sql));
    if actual != Some(normalized_schema_sql(expected.trim_end_matches(';'))) {
        return Err(recovery_required());
    }
    Ok(())
}

const MERGE_SCHEMA_OBJECTS: &[(&str, &str)] = &[
    ("TABLE", "remote_integration_windows"),
    ("TABLE", "remote_integration_steps"),
    ("TABLE", "remote_identity_confirmations"),
    ("TABLE", "remote_resolution_attempts"),
    ("TABLE", "remote_resolution_paths"),
    ("TABLE", "remote_resolution_index_artifacts"),
    ("TABLE", "remote_resolution_ref_log_artifacts"),
    ("INDEX", "remote_integration_windows_batch"),
    ("INDEX", "remote_integration_steps_operation"),
    ("INDEX", "remote_integration_steps_pending"),
    ("INDEX", "remote_identity_confirmations_operation"),
    ("INDEX", "remote_resolution_attempts_operation"),
    ("TRIGGER", "remote_integration_window_immutable"),
    ("TRIGGER", "remote_integration_step_immutable"),
    ("TRIGGER", "remote_identity_confirmation_immutable"),
    ("TRIGGER", "remote_resolution_attempt_immutable"),
    ("TRIGGER", "remote_resolution_path_immutable"),
    ("TRIGGER", "remote_resolution_index_artifact_immutable"),
    ("TRIGGER", "remote_resolution_index_output_immutable"),
    ("TRIGGER", "remote_resolution_ref_log_artifact_immutable"),
];

fn validate_merge_evidence_schema(connection: &Connection) -> Result<(), RepositoryError> {
    for (table, required) in [
        (
            "remote_integration_steps",
            &[
                "id",
                "operation_record_id",
                "configuration_generation",
                "owner_epoch",
                "ordinal",
                "stage",
                "local_oid",
                "incoming_oid",
                "baseline_tree_oid",
                "baseline_index_digest",
                "candidate_oid",
                "result_oid",
                "observed_tree_oid",
                "conflict_digest",
                "phase",
                "window_number",
            ] as &[_],
        ),
        (
            "remote_identity_confirmations",
            &[
                "id",
                "confirmation_ulid",
                "operation_record_id",
                "configuration_generation",
                "owner_epoch",
                "input_digest",
                "configuration_digest",
                "phase",
                "applied_configuration_digest",
            ],
        ),
        (
            "remote_resolution_attempts",
            &[
                "id",
                "attempt_ulid",
                "operation_record_id",
                "integration_step_id",
                "configuration_generation",
                "owner_epoch",
                "observation_digest",
                "input_digest",
                "preflight_digest",
                "identity_confirmation_id",
                "candidate_oid",
                "checkpoint_oid",
                "phase",
            ],
        ),
        (
            "remote_resolution_paths",
            &[
                "attempt_id",
                "ordinal",
                "path_digest",
                "expected_digest",
                "result_digest",
                "prewrite_digest",
                "base_blob_oid",
                "local_blob_oid",
                "incoming_blob_oid",
                "mode",
                "applied",
            ],
        ),
    ] {
        let columns = connection
            .prepare(&format!("PRAGMA table_info({table})"))
            .and_then(|mut statement| {
                statement
                    .query_map([], |row| row.get::<_, String>(1))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|_| recovery_required())?;
        if columns != required {
            return Err(recovery_required());
        }
    }
    for &(kind, name) in MERGE_SCHEMA_OBJECTS {
        validate_schema_object(
            connection,
            kind,
            name,
            merge_schema_object_sql(kind, name).ok_or_else(recovery_required)?,
        )?;
    }
    Ok(())
}

fn migrate_synchronization(tx: &Transaction<'_>) -> Result<(), RepositoryError> {
    let columns = tx
        .prepare("PRAGMA table_info(remote_operation_records)")
        .and_then(|mut q| {
            q.query_map([], |row| row.get::<_, String>(1))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| recovery_required())?;
    let sync_columns = [
        "sync_checkpoint",
        "expected_oid",
        "primary_tracking_oid",
        "push_oid",
        "push_advertised_oid",
        "authoritative_kind",
        "authoritative_oid",
        "index_pending",
        "reconciliation_required",
    ];
    let count = sync_columns
        .iter()
        .filter(|name| columns.iter().any(|column| column == **name))
        .count();
    if count == sync_columns.len() {
        return Ok(());
    }
    if count != 0 {
        return Err(recovery_required());
    }
    // Rebuild CHECK constraints and the active index in this same migration transaction.
    tx.execute_batch("DROP TRIGGER IF EXISTS remote_operation_target_immutable; DROP INDEX IF EXISTS remote_active_reservation; DROP INDEX IF EXISTS remote_operations_repository; ALTER TABLE remote_operation_records RENAME TO remote_operation_records_cycle04;")
        .map_err(|_| recovery_required())?;
    tx.execute_batch(REMOTE_OPERATION_SCHEMA)
        .map_err(|_| recovery_required())?;
    for (name, definition) in [
        (
            "yield_requested",
            "INTEGER NOT NULL DEFAULT 0 CHECK(yield_requested IN (0,1))",
        ),
        (
            "cancel_requested",
            "INTEGER NOT NULL DEFAULT 0 CHECK(cancel_requested IN (0,1))",
        ),
        (
            "owner_epoch",
            "INTEGER NOT NULL DEFAULT 0 CHECK(owner_epoch >= 0)",
        ),
    ] {
        tx.execute_batch(&format!(
            "ALTER TABLE remote_operation_records ADD COLUMN {name} {definition}"
        ))
        .map_err(|_| recovery_required())?;
    }
    let list = columns.join(",");
    tx.execute_batch(&format!("INSERT INTO remote_operation_records({list}) SELECT {list} FROM remote_operation_records_cycle04; DROP TABLE remote_operation_records_cycle04;"))
        .map_err(|_| recovery_required())?;
    Ok(())
}

const DEFAULT_POLL_INTERVAL_SECONDS: u64 = 300;
const MINIMUM_POLL_INTERVAL_SECONDS: u64 = 60;
const MAXIMUM_POLL_INTERVAL_SECONDS: u64 = 3_600;
const MINIMUM_AUTOMATIC_BACKOFF_SECONDS: u64 = 60;
const MAXIMUM_AUTOMATIC_BACKOFF_SECONDS: u64 = 900;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PollingInterval(Duration);

impl PollingInterval {
    pub fn from_seconds(seconds: u64) -> Result<Self, RemotePollingValueError> {
        if !(MINIMUM_POLL_INTERVAL_SECONDS..=MAXIMUM_POLL_INTERVAL_SECONDS).contains(&seconds) {
            return Err(RemotePollingValueError::IntervalOutOfRange);
        }
        Ok(Self(Duration::from_secs(seconds)))
    }

    pub fn duration(self) -> Duration {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AutomaticBackoff(Duration);

impl AutomaticBackoff {
    pub fn from_seconds(seconds: u64) -> Result<Self, RemotePollingValueError> {
        if !(MINIMUM_AUTOMATIC_BACKOFF_SECONDS..=MAXIMUM_AUTOMATIC_BACKOFF_SECONDS)
            .contains(&seconds)
        {
            return Err(RemotePollingValueError::BackoffOutOfRange);
        }
        Ok(Self(Duration::from_secs(seconds)))
    }

    pub fn duration(self) -> Duration {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemotePollingValueError {
    IntervalOutOfRange,
    BackoffOutOfRange,
}

impl fmt::Display for RemotePollingValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::IntervalOutOfRange => "poll interval must be between 60 and 3600 seconds",
            Self::BackoffOutOfRange => "automatic backoff must be between 60 and 900 seconds",
        })
    }
}

impl std::error::Error for RemotePollingValueError {}

#[cfg(test)]
#[path = "state_tests.rs"]
mod persistence_tests;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemotePollingConfiguration {
    enabled: bool,
    paused: bool,
    interval: PollingInterval,
    automatic_backoff: Option<AutomaticBackoff>,
    recovery_suspended: bool,
}

impl RemotePollingConfiguration {
    pub fn new(
        enabled: bool,
        paused: bool,
        interval: PollingInterval,
        automatic_backoff: Option<AutomaticBackoff>,
        recovery_suspended: bool,
    ) -> Self {
        Self {
            enabled,
            paused,
            interval,
            automatic_backoff,
            recovery_suspended,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn interval(&self) -> Duration {
        self.interval.duration()
    }

    pub fn automatic_backoff(&self) -> Option<Duration> {
        self.automatic_backoff.map(AutomaticBackoff::duration)
    }

    pub fn recovery_suspended(&self) -> bool {
        self.recovery_suspended
    }

    pub fn with_automatic_backoff(mut self, backoff: AutomaticBackoff) -> Self {
        self.automatic_backoff = Some(backoff);
        self
    }

    pub fn delay_for(&self, invocation: RemotePollInvocation) -> Option<Duration> {
        match invocation {
            RemotePollInvocation::Automatic => self.automatic_backoff(),
            RemotePollInvocation::Explicit => None,
        }
    }
}

impl Default for RemotePollingConfiguration {
    fn default() -> Self {
        Self {
            enabled: true,
            paused: false,
            interval: PollingInterval(Duration::from_secs(DEFAULT_POLL_INTERVAL_SECONDS)),
            automatic_backoff: None,
            recovery_suspended: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemotePollInvocation {
    Automatic,
    Explicit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteOperationAction {
    Poll,
    SynchronizeContext,
    SynchronizePrimary,
    Promote,
    Close,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteOperationPriority {
    Poll,
    Manual,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteOperationPhase {
    Reserved,
    Advertising,
    Persisting,
    Completed,
    Interrupted,
    Cancelled,
    Failed,
    FetchPrepared,
    FetchObserved,
    LocalPrepared,
    LocalFastForwarded,
    PushPrepared,
    PushReturned,
    PushVerified,
    Reconciling,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteOperationSafePoint {
    BeforeTransport,
    AfterAdvertisement,
    BetweenObservations,
    BeforeBatchCommit,
    AfterBatchCommit,
    BeforeLocalMutation,
    BeforeFetch,
    AfterFetch,
    BeforeLocalUpdate,
    AfterLocalUpdate,
    BeforePush,
    AfterPushReturn,
    AfterPushVerification,
    BeforeDiscovery,
}

/// Action checkpoint survives interruption; terminal phase never erases effect evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SynchronizationCheckpoint {
    FetchPrepared,
    FetchObserved,
    LocalPrepared,
    LocalFastForwarded,
    PushPrepared,
    PushReturned,
    PushVerified,
    DiscoveryPending,
}
impl SynchronizationCheckpoint {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::FetchPrepared => "fetch_prepared",
            Self::FetchObserved => "fetch_observed",
            Self::LocalPrepared => "local_prepared",
            Self::LocalFastForwarded => "local_fast_forwarded",
            Self::PushPrepared => "push_prepared",
            Self::PushReturned => "push_returned",
            Self::PushVerified => "push_verified",
            Self::DiscoveryPending => "discovery_pending",
        }
    }
    fn parse(value: &str) -> Result<Self, RepositoryError> {
        Ok(match value {
            "fetch_prepared" => Self::FetchPrepared,
            "fetch_observed" => Self::FetchObserved,
            "local_prepared" => Self::LocalPrepared,
            "local_fast_forwarded" => Self::LocalFastForwarded,
            "push_prepared" => Self::PushPrepared,
            "push_returned" => Self::PushReturned,
            "push_verified" => Self::PushVerified,
            "discovery_pending" => Self::DiscoveryPending,
            _ => return Err(recovery_required()),
        })
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SynchronizationEvidence {
    pub expected_oid: Option<Oid>,
    pub local_oid: Option<Oid>,
    pub tracking_oid: Option<Oid>,
    pub primary_tracking_oid: Option<Oid>,
    pub push_oid: Option<Oid>,
    pub push_advertised_oid: Option<Oid>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SynchronizationAuthority {
    Published(Oid),
    AlreadyCurrent(Oid),
}
impl SynchronizationAuthority {
    pub(super) fn oid(self) -> Oid {
        match self {
            Self::Published(oid) | Self::AlreadyCurrent(oid) => oid,
        }
    }
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Published(_) => "published",
            Self::AlreadyCurrent(_) => "already_current",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SynchronizationTarget {
    Primary,
    Context {
        kind: AuthoringKind,
        item_id: ItemId,
    },
}

impl SynchronizationTarget {
    pub fn operation_target(&self, plan: &RemoteRefPlan) -> RemoteOperationTarget {
        match self {
            Self::Primary => RemoteOperationTarget::for_primary_synchronization(plan),
            Self::Context { kind, item_id } => RemoteOperationTarget::for_context(
                plan,
                RemoteOperationAction::SynchronizeContext,
                *kind,
                item_id.clone(),
            )
            .expect("synchronization contexts always have a context action"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SynchronizeRemoteRequest {
    pub root: std::path::PathBuf,
    pub operation_id: crate::repository::OperationId,
    pub target: SynchronizationTarget,
    pub approval: Option<crate::repository::transport::HostApproval>,
    /// Optional confirmation for the missing-effective-identity commit boundary.
    /// It has no effect on no-op or fast-forward synchronization.
    pub confirmed_identity: Option<ConfirmedCommitIdentity>,
    /// Explicitly resume the same interrupted or publication-ambiguous action;
    /// ordinary duplicate calls never steal ownership.
    pub restart: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteOperationTarget {
    action: RemoteOperationAction,
    remote_name: String,
    primary_ref: RemoteRefTarget,
    context_ref: Option<RemoteRefTarget>,
    item: Option<(AuthoringKind, ItemId)>,
    local_branch: Option<String>,
}

impl RemoteOperationTarget {
    pub fn for_poll(plan: &RemoteRefPlan) -> Self {
        Self {
            action: RemoteOperationAction::Poll,
            remote_name: plan.remote_name().to_owned(),
            primary_ref: plan.primary().clone(),
            context_ref: None,
            item: None,
            local_branch: None,
        }
    }

    pub fn for_primary_synchronization(plan: &RemoteRefPlan) -> Self {
        Self {
            action: RemoteOperationAction::SynchronizePrimary,
            remote_name: plan.remote_name().to_owned(),
            primary_ref: plan.primary().clone(),
            context_ref: None,
            item: None,
            local_branch: Some(plan.primary_branch().to_owned()),
        }
    }

    pub fn for_context(
        plan: &RemoteRefPlan,
        action: RemoteOperationAction,
        kind: AuthoringKind,
        item_id: ItemId,
    ) -> Result<Self, RemoteOperationTargetError> {
        if !matches!(
            action,
            RemoteOperationAction::SynchronizeContext
                | RemoteOperationAction::Promote
                | RemoteOperationAction::Close
        ) {
            return Err(RemoteOperationTargetError::ContextNotApplicable);
        }
        let context_ref = plan.context(kind, &item_id);
        let local_branch = context_ref
            .remote_ref()
            .strip_prefix("refs/heads/")
            .expect("derived context refs are always local heads")
            .to_owned();
        Ok(Self {
            action,
            remote_name: plan.remote_name().to_owned(),
            primary_ref: plan.primary().clone(),
            context_ref: Some(context_ref),
            item: Some((kind, item_id)),
            local_branch: Some(local_branch),
        })
    }

    pub fn action(&self) -> RemoteOperationAction {
        self.action
    }

    pub fn remote_name(&self) -> &str {
        &self.remote_name
    }

    pub fn primary_ref(&self) -> &RemoteRefTarget {
        &self.primary_ref
    }

    pub fn context_ref(&self) -> Option<&RemoteRefTarget> {
        self.context_ref.as_ref()
    }

    pub fn item(&self) -> Option<(AuthoringKind, &ItemId)> {
        self.item.as_ref().map(|(kind, item_id)| (*kind, item_id))
    }

    pub fn local_branch(&self) -> Option<&str> {
        self.local_branch.as_deref()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteOperationTargetError {
    ContextNotApplicable,
}

impl fmt::Display for RemoteOperationTargetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("remote operation action does not accept a context target")
    }
}

impl std::error::Error for RemoteOperationTargetError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteOutcomeCategory {
    Completed,
    ConfigurationRequired,
    SelectedKeyUnavailable,
    UnlockRequired,
    HostApprovalRequired,
    TransportUnavailable,
    ProtocolRejected,
    Cancelled,
    RepositoryUnavailable,
}

impl fmt::Display for RemoteOutcomeCategory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Completed => "completed",
            Self::ConfigurationRequired => "configuration required",
            Self::SelectedKeyUnavailable => "selected key unavailable",
            Self::UnlockRequired => "unlock required",
            Self::HostApprovalRequired => "host approval required",
            Self::TransportUnavailable => "transport unavailable",
            Self::ProtocolRejected => "protocol rejected",
            Self::Cancelled => "cancelled",
            Self::RepositoryUnavailable => "repository unavailable",
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteRefObservation {
    target: Option<RemoteRefTarget>,
    advertised_oid: Oid,
    tracking_oid: Option<Oid>,
    classification: RemoteRefClassification,
}

impl RemoteRefObservation {
    #[allow(dead_code)]
    pub(crate) fn from_advertisement(
        plan: &RemoteRefPlan,
        remote_ref: &str,
        advertised_oid: Oid,
        tracking_oid: Option<Oid>,
    ) -> Option<Self> {
        let classification = plan.classify_advertised_ref(remote_ref)?;
        let target = plan.target_for_advertised_ref(remote_ref);
        Some(Self {
            target,
            advertised_oid,
            tracking_oid,
            classification,
        })
    }

    pub fn target(&self) -> Option<&RemoteRefTarget> {
        self.target.as_ref()
    }

    pub fn remote_ref(&self) -> Option<&str> {
        self.target.as_ref().map(RemoteRefTarget::remote_ref)
    }

    pub fn tracking_ref(&self) -> Option<&str> {
        self.target.as_ref().map(RemoteRefTarget::tracking_ref)
    }

    pub fn advertised_oid(&self) -> Oid {
        self.advertised_oid
    }

    pub fn tracking_oid(&self) -> Option<Oid> {
        self.tracking_oid
    }

    pub fn classification(&self) -> &RemoteRefClassification {
        &self.classification
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemotePublicationEvidence {
    NeverPublished,
    ObservedPublished,
    HistoryUnknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteContextState {
    Observed,
    Unmaterialized,
    Malformed,
    RemotelyDeleted,
    HistoryUnknown,
}

impl fmt::Display for RemoteContextState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Observed => "observed",
            Self::Unmaterialized => "unmaterialized",
            Self::Malformed => "malformed",
            Self::RemotelyDeleted => "remotely deleted",
            Self::HistoryUnknown => "history unknown",
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteContextSnapshot {
    target: Option<RemoteRefTarget>,
    kind: Option<AuthoringKind>,
    item_id: Option<ItemId>,
    advertised_oid: Option<Oid>,
    tracking_oid: Option<Oid>,
    publication_evidence: RemotePublicationEvidence,
    state: RemoteContextState,
}

impl RemoteContextSnapshot {
    #[allow(dead_code)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        target: Option<RemoteRefTarget>,
        kind: Option<AuthoringKind>,
        item_id: Option<ItemId>,
        advertised_oid: Option<Oid>,
        tracking_oid: Option<Oid>,
        publication_evidence: RemotePublicationEvidence,
        state: RemoteContextState,
    ) -> Self {
        Self {
            target,
            kind,
            item_id,
            advertised_oid,
            tracking_oid,
            publication_evidence,
            state,
        }
    }

    pub fn target(&self) -> Option<&RemoteRefTarget> {
        self.target.as_ref()
    }

    pub fn remote_ref(&self) -> Option<&str> {
        self.target.as_ref().map(RemoteRefTarget::remote_ref)
    }

    pub fn tracking_ref(&self) -> Option<&str> {
        self.target.as_ref().map(RemoteRefTarget::tracking_ref)
    }

    pub fn kind(&self) -> Option<AuthoringKind> {
        self.kind
    }

    pub fn item_id(&self) -> Option<&ItemId> {
        self.item_id.as_ref()
    }

    pub fn advertised_oid(&self) -> Option<Oid> {
        self.advertised_oid
    }

    pub fn tracking_oid(&self) -> Option<Oid> {
        self.tracking_oid
    }

    pub fn publication_evidence(&self) -> RemotePublicationEvidence {
        self.publication_evidence
    }

    pub fn state(&self) -> RemoteContextState {
        self.state
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RemoteSnapshot {
    polling: RemotePollingConfiguration,
    latest_outcome: Option<RemoteOutcomeCategory>,
    observations: Vec<RemoteRefObservation>,
    contexts: Vec<RemoteContextSnapshot>,
    history_unknown: bool,
}

impl RemoteSnapshot {
    #[allow(dead_code)]
    pub(crate) fn new(
        polling: RemotePollingConfiguration,
        latest_outcome: Option<RemoteOutcomeCategory>,
        observations: Vec<RemoteRefObservation>,
        contexts: Vec<RemoteContextSnapshot>,
    ) -> Self {
        Self {
            polling,
            latest_outcome,
            observations,
            contexts,
            history_unknown: false,
        }
    }

    pub fn polling(&self) -> &RemotePollingConfiguration {
        &self.polling
    }

    pub fn latest_outcome(&self) -> Option<RemoteOutcomeCategory> {
        self.latest_outcome
    }

    pub fn observations(&self) -> &[RemoteRefObservation] {
        &self.observations
    }

    pub fn contexts(&self) -> &[RemoteContextSnapshot] {
        &self.contexts
    }

    /// Missing rows never prove a deletion. After cache loss, they cannot prove
    /// first-publication eligibility either, even after recovery is resumed.
    pub fn publication_evidence_for(
        &self,
        kind: AuthoringKind,
        id: &ItemId,
    ) -> RemotePublicationEvidence {
        self.contexts
            .iter()
            .find(|context| context.kind == Some(kind) && context.item_id.as_ref() == Some(id))
            .map(|context| context.publication_evidence)
            .unwrap_or(if self.history_unknown {
                RemotePublicationEvidence::HistoryUnknown
            } else {
                RemotePublicationEvidence::NeverPublished
            })
    }
}

fn flag(value: i64) -> Result<bool, RepositoryError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(recovery_required()),
    }
}

fn outcome(value: &str) -> Result<RemoteOutcomeCategory, RepositoryError> {
    Ok(match value {
        "completed" => RemoteOutcomeCategory::Completed,
        "configuration_required" => RemoteOutcomeCategory::ConfigurationRequired,
        "selected_key_unavailable" => RemoteOutcomeCategory::SelectedKeyUnavailable,
        "unlock_required" => RemoteOutcomeCategory::UnlockRequired,
        "host_approval_required" => RemoteOutcomeCategory::HostApprovalRequired,
        "transport_unavailable" => RemoteOutcomeCategory::TransportUnavailable,
        "protocol_rejected" => RemoteOutcomeCategory::ProtocolRejected,
        "cancelled" => RemoteOutcomeCategory::Cancelled,
        "repository_unavailable" => RemoteOutcomeCategory::RepositoryUnavailable,
        _ => return Err(recovery_required()),
    })
}

pub(super) fn outcome_name(value: RemoteOutcomeCategory) -> &'static str {
    match value {
        RemoteOutcomeCategory::Completed => "completed",
        RemoteOutcomeCategory::ConfigurationRequired => "configuration_required",
        RemoteOutcomeCategory::SelectedKeyUnavailable => "selected_key_unavailable",
        RemoteOutcomeCategory::UnlockRequired => "unlock_required",
        RemoteOutcomeCategory::HostApprovalRequired => "host_approval_required",
        RemoteOutcomeCategory::TransportUnavailable => "transport_unavailable",
        RemoteOutcomeCategory::ProtocolRejected => "protocol_rejected",
        RemoteOutcomeCategory::Cancelled => "cancelled",
        RemoteOutcomeCategory::RepositoryUnavailable => "repository_unavailable",
    }
}

/// Configuration preflight supplies only a validated plan and whether any
/// endpoint/key selection changed. No endpoint, key ID, path, or secret-derived
/// fingerprint is stored. Task 3 owns when to invoke this transition.
pub(in super::super) fn configure(
    transaction: &Transaction<'_>,
    repository_id: i64,
    plan: Option<&RemoteRefPlan>,
    configuration_changed: bool,
) -> Result<i64, RepositoryError> {
    read_snapshot(transaction, repository_id)?;
    let policy = read_policy(transaction, repository_id)?;
    if !configuration_changed && policy.plan.as_ref() == plan {
        return Ok(policy.generation);
    }
    let generation = policy
        .generation
        .checked_add(1)
        .ok_or_else(recovery_required)?;
    transaction.execute("UPDATE remote_operation_records SET phase='interrupted',outcome='configuration_required' WHERE repository_id=?1 AND phase IN ('reserved','advertising','persisting','fetch_prepared','fetch_observed','local_prepared','local_fast_forwarded','push_prepared','push_returned','push_verified','reconciling')",[repository_id]).map_err(|_| recovery_required())?;
    // The mutable current snapshot is invalidated, but a pass referenced by an
    // immutable window is retained as historical intent. It is never made
    // current again, nor used to authorize effects at the new generation.
    transaction.execute("UPDATE remote_observation_batches SET is_current=0 WHERE repository_id=?1 AND is_current=1", [repository_id]).map_err(|_| recovery_required())?;
    transaction.execute("DELETE FROM remote_ref_observations WHERE batch_id IN (SELECT batch.id FROM remote_observation_batches batch WHERE batch.repository_id=?1 AND NOT EXISTS(SELECT 1 FROM remote_integration_windows window_pass WHERE window_pass.observation_batch_id=batch.id))",[repository_id]).map_err(|_| recovery_required())?;
    transaction
        .execute(
            "DELETE FROM remote_observation_batches WHERE repository_id=?1 AND NOT EXISTS(SELECT 1 FROM remote_integration_windows window_pass WHERE window_pass.observation_batch_id=remote_observation_batches.id)",
            [repository_id],
        )
        .map_err(|_| recovery_required())?;
    transaction
        .execute(
            "DELETE FROM remote_context_states WHERE repository_id=?1",
            [repository_id],
        )
        .map_err(|_| recovery_required())?;
    transaction.execute("UPDATE remote_polling_state SET history_unknown=CASE WHEN remote_name IS NOT NULL THEN 1 ELSE history_unknown END,configuration_generation=?2,remote_name=?3,primary_branch=?4,automatic_backoff_seconds=NULL,latest_outcome=NULL WHERE repository_id=?1",
        params![repository_id,generation,plan.map(RemoteRefPlan::remote_name),plan.map(RemoteRefPlan::primary_branch)]).map_err(|_| recovery_required())?;
    Ok(generation)
}

/// Selection is global to this registry. Fence every configured repository in
/// the same transaction as the selection edit; a failed fence rolls back both.
pub(in super::super) fn invalidate_key_selection(
    transaction: &Transaction<'_>,
) -> Result<(), RepositoryError> {
    let ids = transaction
        .prepare("SELECT repository_id FROM remote_polling_state WHERE remote_name IS NOT NULL")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, i64>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| recovery_required())?;
    for id in ids {
        let policy = read_policy(transaction, id)?;
        configure(transaction, id, policy.plan.as_ref(), true)?;
    }
    Ok(())
}

/// Bind the generation to both validated SSH directions across service/process
/// restarts without persisting locator text or any key/credential identity.
pub(in super::super) fn configure_endpoints(
    tx: &Transaction<'_>,
    id: i64,
    plan: &RemoteRefPlan,
    digest: &[u8; 32],
) -> Result<i64, RepositoryError> {
    let previous = read_endpoint_digest(tx, id)?;
    let has_history: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM remote_observation_batches WHERE repository_id=?1 AND is_current=1) OR EXISTS(SELECT 1 FROM remote_operation_records WHERE repository_id=?1 AND phase IN ('reserved','advertising','persisting','fetch_prepared','fetch_observed','local_prepared','local_fast_forwarded','push_prepared','push_returned','push_verified','reconciling'))", [id], |row| row.get(0))
        .map_err(|_| recovery_required())?;
    let changed = previous
        .as_ref()
        .map_or(has_history, |previous| previous != digest);
    let generation = configure(tx, id, Some(plan), changed)?;
    tx.execute(
        "UPDATE remote_polling_state SET endpoint_digest=?2 WHERE repository_id=?1",
        params![id, digest.as_slice()],
    )
    .map_err(|_| recovery_required())?;
    Ok(generation)
}

fn read_endpoint_digest(
    connection: &Connection,
    id: i64,
) -> Result<Option<[u8; 32]>, RepositoryError> {
    let value: Option<Vec<u8>> = connection
        .query_row(
            "SELECT endpoint_digest FROM remote_polling_state WHERE repository_id=?1",
            [id],
            |row| row.get(0),
        )
        .map_err(|_| recovery_required())?;
    value
        .map(|value| value.try_into().map_err(|_| recovery_required()))
        .transpose()
}

#[allow(dead_code)]
pub(in super::super) fn record_outcome(
    transaction: &Transaction<'_>,
    repository_id: i64,
    category: RemoteOutcomeCategory,
    backoff: Option<AutomaticBackoff>,
) -> Result<(), RepositoryError> {
    read_snapshot(transaction, repository_id)?;
    transaction.execute("UPDATE remote_polling_state SET latest_outcome=?2,automatic_backoff_seconds=?3 WHERE repository_id=?1",
        params![repository_id,outcome_name(category),backoff.map(|value| value.duration().as_secs())]).map_err(|_| recovery_required())?;
    Ok(())
}

/// Invoke only with the complete result of a successful advertisement. Failed,
/// partial, or cancelled calls use record_outcome and leave the current batch.
/// The caller wraps this in with_transaction; a stale configuration is rejected
/// before writes. No guard or transaction spans transport work.
#[allow(dead_code)]
pub(in super::super) fn complete_batch(
    transaction: &Transaction<'_>,
    repository_id: i64,
    plan: &RemoteRefPlan,
    generation: i64,
    observations: &[RemoteRefObservation],
    observed_at: i64,
) -> Result<i64, RepositoryError> {
    let batch = persist_complete_advertisement(
        transaction,
        repository_id,
        plan,
        generation,
        observations,
        observed_at,
    )?;
    record_outcome(
        transaction,
        repository_id,
        RemoteOutcomeCategory::Completed,
        None,
    )?;
    Ok(batch)
}

/// Shared complete-advertisement model, independent of polling retry policy.
pub(in super::super) fn persist_complete_advertisement(
    transaction: &Transaction<'_>,
    repository_id: i64,
    plan: &RemoteRefPlan,
    generation: i64,
    observations: &[RemoteRefObservation],
    observed_at: i64,
) -> Result<i64, RepositoryError> {
    read_snapshot(transaction, repository_id)?;
    let policy = read_policy(transaction, repository_id)?;
    if observed_at < 0 || policy.generation != generation || policy.plan.as_ref() != Some(plan) {
        return Err(recovery_required());
    }
    let mut seen = std::collections::HashSet::new();
    for observation in observations {
        if observation.classification == RemoteRefClassification::MalformedContext {
            continue;
        }
        let target = observation.target.as_ref().ok_or_else(recovery_required)?;
        if plan.target_for_advertised_ref(target.remote_ref()).as_ref() != Some(target)
            || plan.classify_advertised_ref(target.remote_ref()).as_ref()
                != Some(&observation.classification)
            || !seen.insert(target.remote_ref())
        {
            return Err(recovery_required());
        }
    }
    transaction.execute("UPDATE remote_observation_batches SET is_current=0 WHERE repository_id=?1 AND is_current=1",[repository_id]).map_err(|_| recovery_required())?;
    transaction.execute("INSERT INTO remote_observation_batches(repository_id,remote_name,primary_branch,configuration_generation,observed_at,is_current) VALUES(?1,?2,?3,?4,?5,1)",
        params![repository_id,plan.remote_name(),plan.primary_branch(),generation,observed_at]).map_err(|_| recovery_required())?;
    let batch = transaction.last_insert_rowid();
    // Last successful OIDs survive an absent advertisement. Unknown history
    // cannot turn into a deletion merely because an empty batch completed.
    transaction.execute("UPDATE remote_context_states SET state='remotely_deleted',observed_at=?2 WHERE repository_id=?1 AND publication_evidence='observed_published'",params![repository_id,observed_at]).map_err(|_| recovery_required())?;
    for (ordinal, observation) in observations.iter().enumerate() {
        let (target, classification) = match observation.classification {
            RemoteRefClassification::Primary => (observation.target.as_ref(), "primary"),
            RemoteRefClassification::RecognizedContext { .. } => {
                (observation.target.as_ref(), "context")
            }
            RemoteRefClassification::MalformedContext => (None, "malformed"),
        };
        transaction.execute("INSERT INTO remote_ref_observations(batch_id,ordinal,remote_ref,tracking_ref,classification,advertised_oid,tracking_oid) VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![batch,ordinal as i64,target.map(RemoteRefTarget::remote_ref),target.map(RemoteRefTarget::tracking_ref),classification,observation.advertised_oid.to_string(),observation.tracking_oid.map(|value|value.to_string())]).map_err(|_| recovery_required())?;
        if let RemoteRefClassification::RecognizedContext { kind, ref item_id } =
            observation.classification
        {
            let target = plan.context(kind, item_id);
            let kind = match kind {
                AuthoringKind::Document => "document",
                AuthoringKind::Ticket => "ticket",
            };
            transaction.execute("INSERT INTO remote_context_states(repository_id,remote_ref,tracking_ref,kind,item_id,last_advertised_oid,tracking_oid,publication_evidence,state,observed_at) VALUES(?1,?2,?3,?4,?5,?6,?7,'observed_published','unmaterialized',?8) ON CONFLICT(repository_id,remote_ref) DO UPDATE SET last_advertised_oid=excluded.last_advertised_oid,tracking_oid=excluded.tracking_oid,publication_evidence='observed_published',state='unmaterialized',observed_at=excluded.observed_at",
                params![repository_id,target.remote_ref(),target.tracking_ref(),kind,item_id.to_string(),observation.advertised_oid.to_string(),observation.tracking_oid.map(|value|value.to_string()),observed_at]).map_err(|_| recovery_required())?;
        }
    }
    Ok(batch)
}

fn oid(value: Option<String>) -> Result<Option<Oid>, RepositoryError> {
    value
        .map(|value| {
            let parsed = Oid::from_str(&value).map_err(|_| recovery_required())?;
            if parsed.to_string() != value {
                return Err(recovery_required());
            }
            Ok(parsed)
        })
        .transpose()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct IntegrationWindowIntent {
    pub observation_batch_id: i64,
    pub local_oid: Oid,
    pub primary_oid: Oid,
    /// None records an absent context slot; primary remains ordinal one.
    pub context_oid: Option<Oid>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct IntegrationWindowEvidence {
    pub number: u32,
    /// Window zero is the legacy envelope and has no invented pass proof.
    pub intent: Option<IntegrationWindowIntent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct IntegrationStepIntent {
    pub ordinal: u8,
    pub stage: IntegrationStage,
    pub local_oid: Oid,
    pub incoming_oid: Oid,
    pub baseline_tree_oid: Oid,
    pub baseline_index_digest: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)] // Task 3 supplies caller-confirmed identity before candidates.
pub(super) struct IdentityConfirmationIntent {
    pub confirmation_id: crate::repository::OperationId,
    pub input_digest: [u8; 32],
    pub configuration_digest: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)] // Task 4 supplies actual conflict observations and paths.
pub(super) struct ResolutionAttemptIntent {
    pub attempt_id: crate::repository::OperationId,
    pub step_ordinal: u8,
    pub observation_digest: [u8; 32],
    pub input_digest: [u8; 32],
    /// Immutable observation of every mutable local input before resolution.
    pub preflight_digest: [u8; 32],
    pub identity_confirmation_id: Option<crate::repository::OperationId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)] // Task 4 supplies actual conflict observations and paths.
pub(super) struct ResolutionPathIntent {
    pub ordinal: u32,
    pub path_digest: [u8; 32],
    pub expected_digest: [u8; 32],
    pub result_digest: [u8; 32],
    /// Exact descriptor-read worktree state before any owned replacement.
    pub prewrite_digest: [u8; 32],
    pub base_blob_oid: Option<Oid>,
    pub local_blob_oid: Option<Oid>,
    pub incoming_blob_oid: Option<Oid>,
    pub mode: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum IntegrationStepPhase {
    Prepared,
    Applying,
    ConflictPending,
    ResolutionPrepared,
    CommitPrepared,
    Applied,
    RecoveryRequired,
}

impl IntegrationStepPhase {
    fn parse(value: &str) -> Result<Self, RepositoryError> {
        Ok(match value {
            "prepared" => Self::Prepared,
            "applying" => Self::Applying,
            "conflict_pending" => Self::ConflictPending,
            "resolution_prepared" => Self::ResolutionPrepared,
            "commit_prepared" => Self::CommitPrepared,
            "applied" => Self::Applied,
            "recovery_required" => Self::RecoveryRequired,
            _ => return Err(recovery_required()),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct IntegrationStepEvidence {
    pub window_number: u32,
    pub intent: IntegrationStepIntent,
    pub phase: IntegrationStepPhase,
    pub candidate_oid: Option<Oid>,
    pub result_oid: Option<Oid>,
    pub observed_tree_oid: Option<Oid>,
    pub conflict_digest: Option<[u8; 32]>,
}

#[allow(dead_code)] // Consumed by the reservation controller in Task 3.
#[derive(Clone, Debug)]
pub(in super::super) struct StoredRemoteOperation {
    pub id: i64,
    pub operation_id: crate::repository::OperationId,
    pub generation: i64,
    pub target: RemoteOperationTarget,
    pub priority: RemoteOperationPriority,
    pub phase: RemoteOperationPhase,
    pub completed_step: Option<RemoteOperationSafePoint>,
    pub local_oid: Option<Oid>,
    pub tracking_oid: Option<Oid>,
    pub advertised_oid: Option<Oid>,
    pub created_at: i64,
    pub updated_at: i64,
    pub outcome: Option<RemoteOutcomeCategory>,
    pub yield_requested: bool,
    pub cancel_requested: bool,
    pub owner_epoch: i64,
    pub sync_checkpoint: Option<SynchronizationCheckpoint>,
    pub sync_evidence: SynchronizationEvidence,
    pub authority: Option<SynchronizationAuthority>,
    pub index_pending: bool,
    pub reconciliation_required: bool,
}

fn operation_from_row(row: &rusqlite::Row<'_>) -> Result<StoredRemoteOperation, RepositoryError> {
    let text = |name| row.get::<_, String>(name).map_err(|_| recovery_required());
    let optional = |name| {
        row.get::<_, Option<String>>(name)
            .map_err(|_| recovery_required())
    };
    let number = |name| row.get::<_, i64>(name).map_err(|_| recovery_required());
    let plan = RemoteRefPlan::from_configuration(&text("remote_name")?, &text("primary_branch")?)
        .map_err(|_| recovery_required())?;
    let target = match text("action")?.as_str() {
        "poll" => RemoteOperationTarget::for_poll(&plan),
        "synchronize_primary" => RemoteOperationTarget::for_primary_synchronization(&plan),
        action @ ("synchronize_context" | "promote" | "close") => {
            let action = match action {
                "synchronize_context" => RemoteOperationAction::SynchronizeContext,
                "promote" => RemoteOperationAction::Promote,
                _ => RemoteOperationAction::Close,
            };
            let kind = match optional("kind")?.as_deref() {
                Some("document") => AuthoringKind::Document,
                Some("ticket") => AuthoringKind::Ticket,
                _ => return Err(recovery_required()),
            };
            let id = optional("item_id")?
                .ok_or_else(recovery_required)?
                .parse()
                .map_err(|_| recovery_required())?;
            RemoteOperationTarget::for_context(&plan, action, kind, id)
                .map_err(|_| recovery_required())?
        }
        _ => return Err(recovery_required()),
    };
    if target.primary_ref.remote_ref() != text("primary_ref")?
        || target.primary_ref.tracking_ref() != text("primary_tracking_ref")?
        || target.context_ref().map(RemoteRefTarget::remote_ref)
            != optional("context_ref")?.as_deref()
        || target.context_ref().map(RemoteRefTarget::tracking_ref)
            != optional("context_tracking_ref")?.as_deref()
        || target.local_branch() != optional("local_branch")?.as_deref()
        || (target.item().is_none()
            && (optional("kind")?.is_some() || optional("item_id")?.is_some()))
    {
        return Err(recovery_required());
    }
    let phase = match text("phase")?.as_str() {
        "reserved" => RemoteOperationPhase::Reserved,
        "advertising" => RemoteOperationPhase::Advertising,
        "persisting" => RemoteOperationPhase::Persisting,
        "completed" => RemoteOperationPhase::Completed,
        "interrupted" => RemoteOperationPhase::Interrupted,
        "cancelled" => RemoteOperationPhase::Cancelled,
        "failed" => RemoteOperationPhase::Failed,
        "fetch_prepared" => RemoteOperationPhase::FetchPrepared,
        "fetch_observed" => RemoteOperationPhase::FetchObserved,
        "local_prepared" => RemoteOperationPhase::LocalPrepared,
        "local_fast_forwarded" => RemoteOperationPhase::LocalFastForwarded,
        "push_prepared" => RemoteOperationPhase::PushPrepared,
        "push_returned" => RemoteOperationPhase::PushReturned,
        "push_verified" => RemoteOperationPhase::PushVerified,
        "reconciling" => RemoteOperationPhase::Reconciling,
        _ => return Err(recovery_required()),
    };
    let priority = match text("priority")?.as_str() {
        "poll" => RemoteOperationPriority::Poll,
        "manual" => RemoteOperationPriority::Manual,
        _ => return Err(recovery_required()),
    };
    let completed_step = optional("completed_step")?
        .map(|step| {
            Ok(match step.as_str() {
                "before_transport" => RemoteOperationSafePoint::BeforeTransport,
                "after_advertisement" => RemoteOperationSafePoint::AfterAdvertisement,
                "between_observations" => RemoteOperationSafePoint::BetweenObservations,
                "before_batch_commit" => RemoteOperationSafePoint::BeforeBatchCommit,
                "after_batch_commit" => RemoteOperationSafePoint::AfterBatchCommit,
                "before_local_mutation" => RemoteOperationSafePoint::BeforeLocalMutation,
                "before_fetch" => RemoteOperationSafePoint::BeforeFetch,
                "after_fetch" => RemoteOperationSafePoint::AfterFetch,
                "before_local_update" => RemoteOperationSafePoint::BeforeLocalUpdate,
                "after_local_update" => RemoteOperationSafePoint::AfterLocalUpdate,
                "before_push" => RemoteOperationSafePoint::BeforePush,
                "after_push_return" => RemoteOperationSafePoint::AfterPushReturn,
                "after_push_verification" => RemoteOperationSafePoint::AfterPushVerification,
                "before_discovery" => RemoteOperationSafePoint::BeforeDiscovery,
                _ => return Err(recovery_required()),
            })
        })
        .transpose()?;
    let created_at = number("created_at")?;
    let updated_at = number("updated_at")?;
    let generation = number("configuration_generation")?;
    let local_oid = oid(optional("local_oid")?)?;
    let owner_epoch = number("owner_epoch")?;
    if created_at < 0
        || updated_at < created_at
        || generation < 0
        || owner_epoch < 0
        || (priority == RemoteOperationPriority::Poll
            && target.action() != RemoteOperationAction::Poll)
        || (target.action() == RemoteOperationAction::Poll && local_oid.is_some())
    {
        return Err(recovery_required());
    }
    let sync_checkpoint = optional("sync_checkpoint")?
        .as_deref()
        .map(SynchronizationCheckpoint::parse)
        .transpose()?;
    let sync_evidence = SynchronizationEvidence {
        expected_oid: oid(optional("expected_oid")?)?,
        local_oid,
        tracking_oid: oid(optional("tracking_oid")?)?,
        primary_tracking_oid: oid(optional("primary_tracking_oid")?)?,
        push_oid: oid(optional("push_oid")?)?,
        push_advertised_oid: oid(optional("push_advertised_oid")?)?,
    };
    let authority = match (
        optional("authoritative_kind")?.as_deref(),
        oid(optional("authoritative_oid")?)?,
    ) {
        (None, None) => None,
        (Some("published"), Some(oid)) => Some(SynchronizationAuthority::Published(oid)),
        (Some("already_current"), Some(oid)) => Some(SynchronizationAuthority::AlreadyCurrent(oid)),
        _ => return Err(recovery_required()),
    };
    let index_pending = flag(number("index_pending")?)?;
    let reconciliation_required = flag(number("reconciliation_required")?)?;
    let is_sync = matches!(
        target.action(),
        RemoteOperationAction::SynchronizePrimary | RemoteOperationAction::SynchronizeContext
    );
    let sync_phase = matches!(
        phase,
        RemoteOperationPhase::FetchPrepared
            | RemoteOperationPhase::FetchObserved
            | RemoteOperationPhase::LocalPrepared
            | RemoteOperationPhase::LocalFastForwarded
            | RemoteOperationPhase::PushPrepared
            | RemoteOperationPhase::PushReturned
            | RemoteOperationPhase::PushVerified
            | RemoteOperationPhase::Reconciling
    );
    let checkpoint_phase = sync_checkpoint.map(|checkpoint| match checkpoint {
        SynchronizationCheckpoint::FetchPrepared => RemoteOperationPhase::FetchPrepared,
        SynchronizationCheckpoint::FetchObserved => RemoteOperationPhase::FetchObserved,
        SynchronizationCheckpoint::LocalPrepared => RemoteOperationPhase::LocalPrepared,
        SynchronizationCheckpoint::LocalFastForwarded => RemoteOperationPhase::LocalFastForwarded,
        SynchronizationCheckpoint::PushPrepared => RemoteOperationPhase::PushPrepared,
        SynchronizationCheckpoint::PushReturned => RemoteOperationPhase::PushReturned,
        SynchronizationCheckpoint::PushVerified => RemoteOperationPhase::PushVerified,
        SynchronizationCheckpoint::DiscoveryPending => RemoteOperationPhase::Completed,
    });
    let effect_prepared = matches!(
        sync_checkpoint,
        Some(
            SynchronizationCheckpoint::LocalPrepared
                | SynchronizationCheckpoint::LocalFastForwarded
                | SynchronizationCheckpoint::PushPrepared
                | SynchronizationCheckpoint::PushReturned
                | SynchronizationCheckpoint::PushVerified
                | SynchronizationCheckpoint::DiscoveryPending
        )
    );
    let push_prepared = matches!(
        sync_checkpoint,
        Some(
            SynchronizationCheckpoint::PushPrepared
                | SynchronizationCheckpoint::PushReturned
                | SynchronizationCheckpoint::PushVerified
                | SynchronizationCheckpoint::DiscoveryPending
        )
    );
    if (sync_phase && phase != RemoteOperationPhase::Reconciling && checkpoint_phase != Some(phase))
        || (phase == RemoteOperationPhase::Reconciling && !reconciliation_required)
        || (effect_prepared
            && (sync_evidence.expected_oid.is_none()
                || sync_evidence.local_oid.is_none()
                || sync_evidence.primary_tracking_oid.is_none()))
        || (push_prepared
            && (sync_evidence.push_oid.is_none()
                || sync_evidence.local_oid != sync_evidence.push_oid))
        || (sync_checkpoint == Some(SynchronizationCheckpoint::PushVerified)
            && sync_evidence.push_advertised_oid != sync_evidence.push_oid)
        || (sync_checkpoint == Some(SynchronizationCheckpoint::DiscoveryPending)
            && authority.is_none())
        || (!is_sync
            && (sync_phase
                || sync_checkpoint.is_some()
                || authority.is_some()
                || index_pending
                || reconciliation_required
                || sync_evidence.expected_oid.is_some()
                || sync_evidence.push_oid.is_some()
                || sync_evidence.primary_tracking_oid.is_some()
                || sync_evidence.push_advertised_oid.is_some()))
        || (index_pending && authority.is_none())
        || authority.is_some_and(|value| {
            sync_checkpoint != Some(SynchronizationCheckpoint::DiscoveryPending)
                || phase != RemoteOperationPhase::Completed
                || reconciliation_required
                || sync_evidence.local_oid != Some(value.oid())
                || sync_evidence.push_oid != Some(value.oid())
                || sync_evidence.push_advertised_oid != Some(value.oid())
        })
    {
        return Err(recovery_required());
    }
    Ok(StoredRemoteOperation {
        id: number("id")?,
        operation_id: crate::repository::OperationId::parse(&text("operation_ulid")?)
            .map_err(|_| recovery_required())?,
        generation,
        target,
        priority,
        phase,
        completed_step,
        local_oid,
        tracking_oid: oid(optional("tracking_oid")?)?,
        advertised_oid: oid(optional("advertised_oid")?)?,
        created_at,
        updated_at,
        outcome: optional("outcome")?.as_deref().map(outcome).transpose()?,
        yield_requested: flag(number("yield_requested")?)?,
        cancel_requested: flag(number("cancel_requested")?)?,
        owner_epoch,
        sync_checkpoint,
        sync_evidence,
        authority,
        index_pending,
        reconciliation_required,
    })
}

pub(in super::super) fn read_operations(
    connection: &Connection,
    repository_id: i64,
) -> Result<Vec<StoredRemoteOperation>, RepositoryError> {
    let records = read_operation_rows(connection, repository_id, false)?;
    for record in &records {
        audit_merge_evidence(connection, record)?;
    }
    Ok(records)
}

pub(super) fn read_operation_rows(
    connection: &Connection,
    repository_id: i64,
    active_only: bool,
) -> Result<Vec<StoredRemoteOperation>, RepositoryError> {
    let query = if active_only {
        "SELECT * FROM remote_operation_records WHERE repository_id=?1 AND phase IN ('reserved','advertising','persisting','fetch_prepared','fetch_observed','local_prepared','local_fast_forwarded','push_prepared','push_returned','push_verified','reconciling') ORDER BY id"
    } else {
        "SELECT * FROM remote_operation_records WHERE repository_id=?1 ORDER BY id"
    };
    let records = connection
        .prepare(query)
        .and_then(|mut statement| {
            statement
                .query_map([repository_id], |row| Ok(operation_from_row(row)))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| recovery_required())?
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    for record in &records {
        audit_merge_evidence(connection, record)?;
    }
    Ok(records)
}

pub(super) fn read_operation(
    connection: &Connection,
    repository_id: i64,
    operation_id: crate::repository::OperationId,
) -> Result<Option<StoredRemoteOperation>, RepositoryError> {
    let record = connection
        .query_row(
            "SELECT * FROM remote_operation_records WHERE repository_id=?1 AND operation_ulid=?2",
            params![repository_id, operation_id.to_string()],
            |row| Ok(operation_from_row(row)),
        )
        .optional()
        .map_err(|_| recovery_required())?
        .transpose()?;
    if let Some(record) = &record {
        audit_merge_evidence(connection, record)?;
    }
    Ok(record)
}

/// The remote operations the status reads look at: the one with this ID,
/// or every one that is not known to be finished. Those are the rows in a
/// phase that holds the reservation, the synchronizations that stopped
/// short of an authoritative outcome, and the rows with index or
/// reconciliation work recorded as outstanding; a poll that ended is never
/// among them, so the result does not grow with the number of polls.
///
/// Nothing is written, and a SQLite failure is returned as it is, so that
/// a read can tell a busy or unreadable index from a row that is there and
/// invalid. Each row is checked as `read_operation` checks it.
pub(in super::super) fn select_status_operations(
    connection: &Connection,
    repository_id: i64,
    operation_id: Option<&str>,
) -> rusqlite::Result<Vec<Result<StoredRemoteOperation, RepositoryError>>> {
    let mut statement = connection.prepare(
        "SELECT * FROM remote_operation_records
          WHERE repository_id=?1
            AND (operation_ulid=?2
                 OR (?2 IS NULL
                     AND (phase NOT IN ('completed','interrupted','cancelled','failed')
                          OR (action IN ('synchronize_context','synchronize_primary')
                              AND phase IN ('interrupted','failed'))
                          OR index_pending=1
                          OR reconciliation_required=1)))
          ORDER BY id",
    )?;
    statement
        .query_map(params![repository_id, operation_id], |row| {
            Ok(operation_from_row(row))
        })?
        .collect()
}

/// The stored polling policy and the outcome of the latest attempt, for
/// the status read: `None` when the registration has no policy row.
///
/// Only these columns are read and checked. A SQLite failure is returned
/// as it is; the inner error is a value that is there and invalid.
#[allow(clippy::type_complexity)]
pub(in super::super) fn select_polling_status(
    connection: &Connection,
    repository_id: i64,
) -> rusqlite::Result<
    Option<Result<(RemotePollingConfiguration, Option<RemoteOutcomeCategory>), RepositoryError>>,
> {
    let values = connection
        .query_row(
            "SELECT enabled,paused,interval_seconds,automatic_backoff_seconds,
                    recovery_suspended,latest_outcome
               FROM remote_polling_state WHERE repository_id=?1",
            [repository_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            },
        )
        .optional()?;
    Ok(
        values.map(|(enabled, paused, interval, backoff, suspended, latest)| {
            let seconds = |value: i64| u64::try_from(value).map_err(|_| recovery_required());
            Ok((
                RemotePollingConfiguration::new(
                    flag(enabled)?,
                    flag(paused)?,
                    PollingInterval::from_seconds(seconds(interval)?)
                        .map_err(|_| recovery_required())?,
                    backoff
                        .map(|backoff| {
                            AutomaticBackoff::from_seconds(seconds(backoff)?)
                                .map_err(|_| recovery_required())
                        })
                        .transpose()?,
                    flag(suspended)?,
                ),
                latest.as_deref().map(outcome).transpose()?,
            ))
        }),
    )
}

pub(super) fn generation(
    connection: &Connection,
    repository_id: i64,
) -> Result<i64, RepositoryError> {
    Ok(read_policy(connection, repository_id)?.generation)
}

fn stage_name(stage: IntegrationStage) -> &'static str {
    match stage {
        IntegrationStage::Context => "context",
        IntegrationStage::Primary => "primary",
    }
}

fn digest(value: Option<Vec<u8>>) -> Result<Option<[u8; 32]>, RepositoryError> {
    value
        .map(|value| value.try_into().map_err(|_| recovery_required()))
        .transpose()
}

fn synchronization_record(record: &StoredRemoteOperation) -> Result<(), RepositoryError> {
    if !matches!(
        record.target.action(),
        RemoteOperationAction::SynchronizePrimary | RemoteOperationAction::SynchronizeContext
    ) {
        return Err(recovery_required());
    }
    Ok(())
}

pub(super) fn integration_window(
    connection: &Connection,
    record: &StoredRemoteOperation,
    number: u32,
) -> Result<Option<IntegrationWindowEvidence>, RepositoryError> {
    let row = connection.query_row(
        "SELECT configuration_generation,owner_epoch,kind,observation_batch_id,local_oid,primary_oid,context_oid FROM remote_integration_windows WHERE operation_record_id=?1 AND number=?2",
        params![record.id, i64::from(number)],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?, row.get::<_, Option<i64>>(3)?, row.get::<_, Option<String>>(4)?, row.get::<_, Option<String>>(5)?, row.get::<_, Option<String>>(6)?)),
    ).optional().map_err(|_| recovery_required())?;
    let Some((generation, epoch, kind, batch, local, primary, context)) = row else {
        return Ok(None);
    };
    synchronization_record(record)?;
    if generation != record.generation || epoch < 0 || epoch > record.owner_epoch {
        return Err(recovery_required());
    }
    let intent = match (
        number,
        kind.as_str(),
        batch,
        oid(local)?,
        oid(primary)?,
        oid(context)?,
    ) {
        (0, "legacy", None, None, None, None) => None,
        (
            1..,
            "pinned",
            Some(observation_batch_id),
            Some(local_oid),
            Some(primary_oid),
            context_oid,
        ) => {
            let intent = IntegrationWindowIntent {
                observation_batch_id,
                local_oid,
                primary_oid,
                context_oid,
            };
            validate_window_observation(connection, record, &intent, false)?;
            Some(intent)
        }
        _ => return Err(recovery_required()),
    };
    Ok(Some(IntegrationWindowEvidence { number, intent }))
}

fn validate_window_observation(
    connection: &Connection,
    record: &StoredRemoteOperation,
    intent: &IntegrationWindowIntent,
    require_current: bool,
) -> Result<(), RepositoryError> {
    let valid_batch: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM remote_observation_batches batch JOIN remote_operation_records operation ON operation.repository_id=batch.repository_id WHERE batch.id=?1 AND operation.id=?2 AND batch.configuration_generation=?3 AND batch.remote_name=?4 AND batch.primary_branch=?5 AND (?6=0 OR batch.is_current=1))",
        params![intent.observation_batch_id, record.id, record.generation, record.target.remote_name(), record.target.primary_ref().remote_ref().strip_prefix("refs/heads/").ok_or_else(recovery_required)?, require_current],
        |row| row.get(0),
    ).map_err(|_| recovery_required())?;
    if !valid_batch {
        return Err(recovery_required());
    }
    let observed = |target: &RemoteRefTarget| -> Result<Option<Oid>, RepositoryError> {
        let values = connection.query_row(
            "SELECT advertised_oid,tracking_oid,tracking_ref,classification FROM remote_ref_observations WHERE batch_id=?1 AND remote_ref=?2",
            params![intent.observation_batch_id, target.remote_ref()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, Option<String>>(2)?, row.get::<_, String>(3)?)),
        ).optional().map_err(|_| recovery_required())?;
        let Some((advertised, tracking, tracking_ref, classification)) = values else {
            return Ok(None);
        };
        if tracking.as_deref() != Some(advertised.as_str())
            || tracking_ref.as_deref() != Some(target.tracking_ref())
            || classification
                != if target == record.target.primary_ref() {
                    "primary"
                } else {
                    "context"
                }
        {
            return Err(recovery_required());
        }
        oid(Some(advertised))
    };
    let context = record
        .target
        .context_ref()
        .map(observed)
        .transpose()?
        .flatten();
    if observed(record.target.primary_ref())? != Some(intent.primary_oid)
        || context != intent.context_oid
    {
        return Err(recovery_required());
    }
    Ok(())
}

/// Append a frozen pass only after all earlier local stages are durable. This
/// journal API does not prove Git effects: the controller supplies freshly
/// observed head/refs outside its lease. An unresolved legacy push is deliberately
/// blocked until publication composition supplies its separate window binding.
pub(super) fn prepare_integration_window(
    tx: &Transaction<'_>,
    record: &StoredRemoteOperation,
    number: u32,
    intent: &IntegrationWindowIntent,
) -> Result<IntegrationWindowEvidence, RepositoryError> {
    synchronization_record(record)?;
    if let Some(existing) = integration_window(tx, record, number)? {
        return if existing.intent.as_ref() == Some(intent) {
            Ok(existing)
        } else {
            Err(recovery_required())
        };
    }
    if number == 0 || record.authority.is_some() || record.sync_evidence.push_oid.is_some() {
        return Err(recovery_required());
    }
    let previous = number.checked_sub(1).ok_or_else(recovery_required)?;
    let window = integration_window(tx, record, previous)?.ok_or_else(recovery_required)?;
    let latest: i64 = tx.query_row("SELECT max(number) FROM remote_integration_windows WHERE operation_record_id=?1", [record.id], |row| row.get(0)).map_err(|_| recovery_required())?;
    let unfinished: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM remote_integration_steps WHERE operation_record_id=?1 AND phase!='applied')", [record.id], |row| row.get(0)).map_err(|_| recovery_required())?;
    let unfinished_resolution: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM remote_resolution_attempts attempt LEFT JOIN remote_resolution_index_artifacts artifact ON artifact.attempt_id=attempt.id WHERE attempt.operation_record_id=?1 AND (attempt.phase!='applied' OR artifact.phase IS NULL OR artifact.phase!='released' OR artifact.ref_phase!='observed'))", [record.id], |row| row.get(0)).map_err(|_| recovery_required())?;
    if latest != i64::from(window.number)
        || unfinished
        || unfinished_resolution
        || window.intent.as_ref().is_some_and(|old| {
            old.local_oid == intent.local_oid
                && old.primary_oid == intent.primary_oid
                && old.context_oid == intent.context_oid
        })
    {
        return Err(recovery_required());
    }
    let has_steps: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM remote_integration_steps WHERE operation_record_id=?1 AND window_number=?2)", params![record.id, i64::from(previous)], |row| row.get(0)).map_err(|_| recovery_required())?;
    if window.intent.is_some() || has_steps {
        // An empty or context-only previous pass is unfinished, even though it
        // has no unapplied row. Absent context is represented by its window.
        let primary_ordinal = if record.target.context_ref().is_some() {
            1
        } else {
            0
        };
        if integration_step_in_window(tx, record.id, previous, primary_ordinal)?.is_none() {
            return Err(recovery_required());
        }
    }
    validate_window_observation(tx, record, intent, true)?;
    tx.execute("INSERT INTO remote_integration_windows(operation_record_id,number,configuration_generation,owner_epoch,kind,observation_batch_id,local_oid,primary_oid,context_oid) VALUES(?1,?2,?3,?4,'pinned',?5,?6,?7,?8)", params![record.id, i64::from(number), record.generation, record.owner_epoch, intent.observation_batch_id, intent.local_oid.to_string(), intent.primary_oid.to_string(), intent.context_oid.map(|oid| oid.to_string())]).map_err(|_| recovery_required())?;
    integration_window(tx, record, number)?.ok_or_else(recovery_required)
}

pub(super) fn prepare_integration_step(
    tx: &Transaction<'_>,
    record: &StoredRemoteOperation,
    intent: &IntegrationStepIntent,
) -> Result<IntegrationStepEvidence, RepositoryError> {
    prepare_integration_step_in_window(tx, record, 0, intent)
}

pub(super) fn prepare_integration_step_in_window(
    tx: &Transaction<'_>,
    record: &StoredRemoteOperation,
    window_number: u32,
    intent: &IntegrationStepIntent,
) -> Result<IntegrationStepEvidence, RepositoryError> {
    synchronization_record(record)?;
    let window = integration_window(tx, record, window_number)?.ok_or_else(recovery_required)?;
    if intent.ordinal > 1
        || (record.target.action() == RemoteOperationAction::SynchronizePrimary
            && (intent.ordinal != 0 || intent.stage != IntegrationStage::Primary))
        || (record.target.action() == RemoteOperationAction::SynchronizeContext
            && ((intent.ordinal == 0 && intent.stage != IntegrationStage::Context)
                || (intent.ordinal == 1 && intent.stage != IntegrationStage::Primary)))
    {
        return Err(recovery_required());
    }
    if let Some(existing) = integration_step_in_window(tx, record.id, window_number, intent.ordinal)?
    {
        if existing.intent != *intent || existing.phase != IntegrationStepPhase::Prepared {
            return Err(recovery_required());
        }
        return Ok(existing);
    }
    let latest: i64 = tx.query_row("SELECT max(number) FROM remote_integration_windows WHERE operation_record_id=?1", [record.id], |row| row.get(0)).map_err(|_| recovery_required())?;
    if latest != i64::from(window_number) {
        return Err(recovery_required());
    }
    let expected_ordinal: i64 = tx
        .query_row(
            "SELECT count(*) FROM remote_integration_steps WHERE operation_record_id=?1 AND window_number=?2",
            params![record.id, i64::from(window_number)],
            |row| row.get(0),
        )
        .map_err(|_| recovery_required())?;
    let absent_context = record.target.context_ref().is_some()
        && window
            .intent
            .as_ref()
            .is_some_and(|pass| pass.context_oid.is_none());
    let first_ordinal = i64::from(absent_context);
    if expected_ordinal + first_ordinal != i64::from(intent.ordinal) {
        return Err(recovery_required());
    }
    if let Some(pass) = &window.intent {
        let incoming = match intent.stage {
            IntegrationStage::Context => pass.context_oid,
            IntegrationStage::Primary => Some(pass.primary_oid),
        };
        let expected_local = if expected_ordinal == 0 {
            pass.local_oid
        } else {
            integration_step_in_window(tx, record.id, window_number, 0)?
                .filter(|step| step.phase == IntegrationStepPhase::Applied)
                .and_then(|step| step.result_oid)
                .ok_or_else(recovery_required)?
        };
        if incoming != Some(intent.incoming_oid) || expected_local != intent.local_oid {
            return Err(recovery_required());
        }
    }
    tx.execute(
        "INSERT INTO remote_integration_steps(operation_record_id,configuration_generation,owner_epoch,ordinal,stage,local_oid,incoming_oid,baseline_tree_oid,baseline_index_digest,phase,window_number) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,'prepared',?10)",
        params![record.id,record.generation,record.owner_epoch,i64::from(intent.ordinal),stage_name(intent.stage),intent.local_oid.to_string(),intent.incoming_oid.to_string(),intent.baseline_tree_oid.to_string(),intent.baseline_index_digest.as_slice(),i64::from(window_number)],
    ).map_err(|_| recovery_required())?;
    integration_step_in_window(tx, record.id, window_number, intent.ordinal)?
        .ok_or_else(recovery_required)
}

pub(super) fn begin_integration_effect(
    tx: &Transaction<'_>,
    record: &StoredRemoteOperation,
    ordinal: u8,
    candidate_oid: Option<Oid>,
) -> Result<(), RepositoryError> {
    begin_integration_effect_in_window(tx, record, 0, ordinal, candidate_oid)
}

pub(super) fn begin_integration_effect_in_window(
    tx: &Transaction<'_>,
    record: &StoredRemoteOperation,
    window_number: u32,
    ordinal: u8,
    candidate_oid: Option<Oid>,
) -> Result<(), RepositoryError> {
    let step = integration_step_in_window(tx, record.id, window_number, ordinal)?
        .ok_or_else(recovery_required)?;
    if step.phase != IntegrationStepPhase::Prepared {
        return Err(recovery_required());
    }
    tx.execute(
        "UPDATE remote_integration_steps SET phase='applying',candidate_oid=?2 WHERE operation_record_id=?1 AND ordinal=?3 AND window_number=?4",
        params![record.id,candidate_oid.map(|oid| oid.to_string()),i64::from(ordinal),i64::from(window_number)],
    ).map_err(|_| recovery_required())?;
    Ok(())
}

pub(super) fn observe_integration_effect(
    tx: &Transaction<'_>,
    record: &StoredRemoteOperation,
    ordinal: u8,
    result_oid: Oid,
    observed_tree_oid: Oid,
) -> Result<(), RepositoryError> {
    observe_integration_effect_in_window(tx, record, 0, ordinal, result_oid, observed_tree_oid)
}

pub(super) fn observe_integration_effect_in_window(
    tx: &Transaction<'_>,
    record: &StoredRemoteOperation,
    window_number: u32,
    ordinal: u8,
    result_oid: Oid,
    observed_tree_oid: Oid,
) -> Result<(), RepositoryError> {
    let step = integration_step_in_window(tx, record.id, window_number, ordinal)?
        .ok_or_else(recovery_required)?;
    if step.phase != IntegrationStepPhase::Applying {
        return Err(recovery_required());
    }
    tx.execute(
        "UPDATE remote_integration_steps SET phase='applied',result_oid=?2,observed_tree_oid=?3 WHERE operation_record_id=?1 AND ordinal=?4 AND window_number=?5",
        params![record.id,result_oid.to_string(),observed_tree_oid.to_string(),i64::from(ordinal),i64::from(window_number)],
    ).map_err(|_| recovery_required())?;
    Ok(())
}

pub(super) fn record_integration_conflict(
    tx: &Transaction<'_>,
    record: &StoredRemoteOperation,
    ordinal: u8,
    conflict_digest: [u8; 32],
) -> Result<(), RepositoryError> {
    record_integration_conflict_in_window(tx, record, 0, ordinal, conflict_digest)
}

pub(super) fn record_integration_conflict_in_window(
    tx: &Transaction<'_>,
    record: &StoredRemoteOperation,
    window_number: u32,
    ordinal: u8,
    conflict_digest: [u8; 32],
) -> Result<(), RepositoryError> {
    let step = integration_step_in_window(tx, record.id, window_number, ordinal)?
        .ok_or_else(recovery_required)?;
    if step.phase != IntegrationStepPhase::Applying {
        return Err(recovery_required());
    }
    tx.execute(
        "UPDATE remote_integration_steps SET phase='conflict_pending',conflict_digest=?2 WHERE operation_record_id=?1 AND ordinal=?3 AND window_number=?4",
        params![record.id,conflict_digest.as_slice(),i64::from(ordinal),i64::from(window_number)],
    ).map_err(|_| recovery_required())?;
    Ok(())
}

pub(super) fn integration_step(
    connection: &Connection,
    operation_record_id: i64,
    ordinal: u8,
) -> Result<Option<IntegrationStepEvidence>, RepositoryError> {
    // Compatibility lookup is deliberately window zero, never "latest". Old
    // opaque observations and replay calls must retain their original binding.
    integration_step_in_window(connection, operation_record_id, 0, ordinal)
}

pub(super) fn integration_step_in_window(
    connection: &Connection,
    operation_record_id: i64,
    window_number: u32,
    ordinal: u8,
) -> Result<Option<IntegrationStepEvidence>, RepositoryError> {
    connection.query_row(
        "SELECT ordinal,stage,local_oid,incoming_oid,baseline_tree_oid,baseline_index_digest,phase,candidate_oid,result_oid,observed_tree_oid,conflict_digest FROM remote_integration_steps WHERE operation_record_id=?1 AND ordinal=?2 AND window_number=?3",
        params![operation_record_id,i64::from(ordinal),i64::from(window_number)],
        |row| {
            let stage = match row.get::<_, String>(1)?.as_str() {
                "context" => IntegrationStage::Context,
                "primary" => IntegrationStage::Primary,
                _ => return Err(rusqlite::Error::InvalidQuery),
            };
            let phase = row.get::<_, String>(6)?;
            Ok((
                IntegrationStepIntent {
                    ordinal: row.get::<_, i64>(0)?.try_into().map_err(|_| rusqlite::Error::InvalidQuery)?,
                    stage,
                    local_oid: Oid::from_str(&row.get::<_, String>(2)?).map_err(|_| rusqlite::Error::InvalidQuery)?,
                    incoming_oid: Oid::from_str(&row.get::<_, String>(3)?).map_err(|_| rusqlite::Error::InvalidQuery)?,
                    baseline_tree_oid: Oid::from_str(&row.get::<_, String>(4)?).map_err(|_| rusqlite::Error::InvalidQuery)?,
                    baseline_index_digest: row.get::<_, Vec<u8>>(5)?.try_into().map_err(|_| rusqlite::Error::InvalidQuery)?,
                },
                phase,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<Vec<u8>>>(10)?,
            ))
        },
    ).optional().map_err(|_| recovery_required())?.map(|(intent, phase, candidate, result, tree, conflict)| {
        let phase = IntegrationStepPhase::parse(&phase)?;
        let candidate_oid = oid(candidate)?;
        let result_oid = oid(result)?;
        let observed_tree_oid = oid(tree)?;
        let conflict_digest = digest(conflict)?;
        if (phase == IntegrationStepPhase::ConflictPending && conflict_digest.is_none())
            || (phase == IntegrationStepPhase::CommitPrepared && candidate_oid.is_none())
            || (phase == IntegrationStepPhase::Applied
                && (result_oid.is_none() || observed_tree_oid.is_none()))
        {
            return Err(recovery_required());
        }
        Ok(IntegrationStepEvidence { window_number, intent, phase, candidate_oid, result_oid, observed_tree_oid, conflict_digest })
    }).transpose()
}

fn audit_merge_evidence(
    connection: &Connection,
    record: &StoredRemoteOperation,
) -> Result<(), RepositoryError> {
    let windows = connection.prepare("SELECT number FROM remote_integration_windows WHERE operation_record_id=?1 ORDER BY number")
        .and_then(|mut statement| statement.query_map([record.id], |row| row.get::<_, i64>(0))?.collect::<Result<Vec<_>, _>>())
        .map_err(|_| recovery_required())?;
    // Every synchronization acquires its legacy marker at insertion/migration.
    // Its absence may mean DELETE cascaded away all child effect evidence; an
    // empty, FK-valid envelope must never be accepted or reconstructed as fresh.
    if matches!(
        record.target.action(),
        RemoteOperationAction::SynchronizePrimary | RemoteOperationAction::SynchronizeContext
    ) && windows.first() != Some(&0)
    {
        return Err(recovery_required());
    }
    for (expected, number) in windows.into_iter().enumerate() {
        if number != expected as i64 {
            return Err(recovery_required());
        }
        let number = number.try_into().map_err(|_| recovery_required())?;
        let window = integration_window(connection, record, number)?.ok_or_else(recovery_required)?;
        let first = i64::from(
            record.target.context_ref().is_some()
                && window
                    .intent
                    .as_ref()
                    .is_some_and(|pass| pass.context_oid.is_none()),
        );
        let steps = connection.prepare("SELECT ordinal,configuration_generation,owner_epoch FROM remote_integration_steps WHERE operation_record_id=?1 AND window_number=?2 ORDER BY ordinal")
            .and_then(|mut statement| statement.query_map(params![record.id, i64::from(number)], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?)))?.collect::<Result<Vec<_>, _>>())
            .map_err(|_| recovery_required())?;
        for (expected, (ordinal, generation, epoch)) in steps.into_iter().enumerate() {
            if ordinal != expected as i64 + first
                || !(0..=1).contains(&ordinal)
                || generation != record.generation
                || epoch < 0
                || epoch > record.owner_epoch
            {
                return Err(recovery_required());
            }
            let step = integration_step_in_window(connection, record.id, number, ordinal as u8)?
                .ok_or_else(recovery_required)?;
            if (record.target.action() == RemoteOperationAction::SynchronizePrimary
                && (ordinal != 0 || step.intent.stage != IntegrationStage::Primary))
                || (record.target.action() == RemoteOperationAction::SynchronizeContext
                    && ((ordinal == 0 && step.intent.stage != IntegrationStage::Context)
                        || (ordinal == 1 && step.intent.stage != IntegrationStage::Primary)))
            {
                return Err(recovery_required());
            }
            if let Some(pass) = &window.intent {
                let incoming = match step.intent.stage {
                    IntegrationStage::Context => pass.context_oid,
                    IntegrationStage::Primary => Some(pass.primary_oid),
                };
                let local = if expected == 0 {
                    Some(pass.local_oid)
                } else {
                    integration_step_in_window(connection, record.id, number, 0)?
                        .filter(|step| step.phase == IntegrationStepPhase::Applied)
                        .and_then(|step| step.result_oid)
                };
                if incoming != Some(step.intent.incoming_oid) || local != Some(step.intent.local_oid)
                {
                    return Err(recovery_required());
                }
            }
        }
    }
    let orphan_step: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM remote_integration_steps step LEFT JOIN remote_integration_windows pass ON pass.operation_record_id=step.operation_record_id AND pass.number=step.window_number WHERE step.operation_record_id=?1 AND pass.number IS NULL)", [record.id], |row| row.get(0)).map_err(|_| recovery_required())?;
    if orphan_step {
        return Err(recovery_required());
    }
    let invalid_attempt: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM remote_resolution_attempts attempt LEFT JOIN remote_integration_steps step ON step.id=attempt.integration_step_id WHERE attempt.operation_record_id=?1 AND (step.operation_record_id!=attempt.operation_record_id OR step.configuration_generation!=attempt.configuration_generation OR attempt.configuration_generation!=?2 OR attempt.owner_epoch<0 OR length(attempt.observation_digest)!=32 OR length(attempt.input_digest)!=32 OR length(attempt.preflight_digest)!=32))",
        params![record.id,record.generation],
        |row| row.get(0),
    ).map_err(|_| recovery_required())?;
    let invalid_confirmation: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM remote_identity_confirmations WHERE operation_record_id=?1 AND (configuration_generation!=?2 OR owner_epoch<0 OR length(input_digest)!=32 OR length(configuration_digest)!=32 OR (phase='applied' AND applied_configuration_digest IS NULL)))",
        params![record.id,record.generation],
        |row| row.get(0),
    ).map_err(|_| recovery_required())?;
    let invalid_paths: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM remote_resolution_paths path JOIN remote_resolution_attempts attempt ON attempt.id=path.attempt_id WHERE attempt.operation_record_id=?1 AND (length(path.path_digest)!=32 OR length(path.expected_digest)!=32 OR length(path.result_digest)!=32 OR length(path.prewrite_digest)!=32 OR path.mode NOT IN (33188,33261)))",
        [record.id],
        |row| row.get(0),
    ).map_err(|_| recovery_required())?;
    let artifact_attempts = connection.prepare("SELECT attempt.attempt_ulid FROM remote_resolution_attempts attempt JOIN remote_resolution_index_artifacts artifact ON artifact.attempt_id=attempt.id WHERE attempt.operation_record_id=?1")
        .and_then(|mut statement| statement.query_map([record.id], |row| row.get::<_,String>(0))?.collect::<Result<Vec<_>,_>>()).map_err(|_| recovery_required())?;
    for attempt in artifact_attempts {
        let attempt =
            crate::repository::OperationId::parse(&attempt).map_err(|_| recovery_required())?;
        resolution_index_artifact(connection, record, attempt)?.ok_or_else(recovery_required)?;
        for role in ["baseline", "transition"] {
            resolution_ref_log_artifact(connection, record, attempt, role)?;
        }
    }
    if invalid_attempt || invalid_confirmation || invalid_paths {
        return Err(recovery_required());
    }
    Ok(())
}

pub(super) fn has_pending_conflict(
    connection: &Connection,
    repository_id: i64,
) -> Result<bool, RepositoryError> {
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM remote_integration_steps step JOIN remote_operation_records operation ON operation.id=step.operation_record_id WHERE operation.repository_id=?1 AND step.phase='conflict_pending')",
        [repository_id],
        |row| row.get(0),
    ).map_err(|_| recovery_required())
}

/// Normal authoring remains available in unrelated contexts, but it must not
/// manufacture a one-parent checkpoint from the worktree that holds this
/// synchronization's unresolved merge.
pub(in super::super) fn has_pending_context_conflict(
    connection: &Connection,
    repository_id: i64,
    kind: &str,
    item_id: &str,
) -> Result<bool, RepositoryError> {
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM remote_integration_steps step JOIN remote_operation_records operation ON operation.id=step.operation_record_id WHERE operation.repository_id=?1 AND operation.action='synchronize_context' AND operation.kind=?2 AND operation.item_id=?3 AND step.phase IN ('conflict_pending','resolution_prepared','commit_prepared'))",
        rusqlite::params![repository_id, kind, item_id],
        |row| row.get(0),
    ).map_err(|_| recovery_required())
}

fn validate_child_id(
    tx: &Transaction<'_>,
    operation_id: crate::repository::OperationId,
) -> Result<(), RepositoryError> {
    let id = operation_id.to_string();
    let collision: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM operation_records WHERE operation_ulid=?1) OR EXISTS(SELECT 1 FROM remote_operation_records WHERE operation_ulid=?1) OR EXISTS(SELECT 1 FROM remote_identity_confirmations WHERE confirmation_ulid=?1) OR EXISTS(SELECT 1 FROM remote_resolution_attempts WHERE attempt_ulid=?1)",
        [&id],
        |row| row.get(0),
    ).map_err(|_| recovery_required())?;
    if collision {
        Err(recovery_required())
    } else {
        Ok(())
    }
}

pub(super) fn prepare_identity_confirmation(
    tx: &Transaction<'_>,
    record: &StoredRemoteOperation,
    intent: &IdentityConfirmationIntent,
) -> Result<(), RepositoryError> {
    synchronization_record(record)?;
    let id = intent.confirmation_id.to_string();
    let existing = tx.query_row(
        "SELECT operation_record_id,configuration_generation,owner_epoch,input_digest,configuration_digest FROM remote_identity_confirmations WHERE confirmation_ulid=?1",
        [&id],
        |row| Ok((row.get::<_,i64>(0)?,row.get::<_,i64>(1)?,row.get::<_,i64>(2)?,row.get::<_,Vec<u8>>(3)?,row.get::<_,Vec<u8>>(4)?)),
    ).optional().map_err(|_| recovery_required())?;
    if let Some((parent, generation, epoch, input, configuration)) = existing {
        if parent == record.id
            && generation == record.generation
            && epoch >= 0
            && input.as_slice() == intent.input_digest
            && configuration.as_slice() == intent.configuration_digest
        {
            return Ok(());
        }
        return Err(recovery_required());
    }
    validate_child_id(tx, intent.confirmation_id)?;
    tx.execute("INSERT INTO remote_identity_confirmations(confirmation_ulid,operation_record_id,configuration_generation,owner_epoch,input_digest,configuration_digest,phase) VALUES(?1,?2,?3,?4,?5,?6,'prepared')",params![id,record.id,record.generation,record.owner_epoch,intent.input_digest.as_slice(),intent.configuration_digest.as_slice()]).map_err(|_| recovery_required())?;
    Ok(())
}

pub(super) fn prepare_resolution_attempt(
    tx: &Transaction<'_>,
    record: &StoredRemoteOperation,
    intent: &ResolutionAttemptIntent,
    paths: &[ResolutionPathIntent],
) -> Result<(), RepositoryError> {
    prepare_resolution_attempt_in_window(tx, record, 0, intent, paths)
}

pub(super) fn prepare_resolution_attempt_in_window(
    tx: &Transaction<'_>,
    record: &StoredRemoteOperation,
    window_number: u32,
    intent: &ResolutionAttemptIntent,
    paths: &[ResolutionPathIntent],
) -> Result<(), RepositoryError> {
    synchronization_record(record)?;
    if paths.is_empty()
        || paths.iter().enumerate().any(|(index, path)| {
            path.ordinal as usize != index || !matches!(path.mode, 33188 | 33261)
        })
    {
        return Err(recovery_required());
    }
    let step =
        integration_step_in_window(tx, record.id, window_number, intent.step_ordinal)?
            .ok_or_else(recovery_required)?;
    if !matches!(
        step.phase,
        IntegrationStepPhase::ConflictPending
            | IntegrationStepPhase::ResolutionPrepared
            | IntegrationStepPhase::CommitPrepared
            | IntegrationStepPhase::Applied
    ) || step.conflict_digest != Some(intent.observation_digest)
    {
        return Err(recovery_required());
    }
    let identity_id = intent
        .identity_confirmation_id
        .map(|confirmation| {
            tx.query_row(
                "SELECT id FROM remote_identity_confirmations WHERE confirmation_ulid=?1 AND operation_record_id=?2",
                params![confirmation.to_string(), record.id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|_| recovery_required())
        })
        .transpose()?;
    let attempt = intent.attempt_id.to_string();
    let existing = tx.query_row("SELECT operation_record_id,integration_step_id,configuration_generation,owner_epoch,observation_digest,input_digest,preflight_digest,identity_confirmation_id FROM remote_resolution_attempts WHERE attempt_ulid=?1",[&attempt],|row| Ok((row.get::<_,i64>(0)?,row.get::<_,i64>(1)?,row.get::<_,i64>(2)?,row.get::<_,i64>(3)?,row.get::<_,Vec<u8>>(4)?,row.get::<_,Vec<u8>>(5)?,row.get::<_,Vec<u8>>(6)?,row.get::<_,Option<i64>>(7)?))).optional().map_err(|_| recovery_required())?;
    if let Some((
        parent,
        step_id,
        generation,
        epoch,
        observation,
        input,
        preflight,
        confirmation_id,
    )) = existing
    {
        let expected_step: i64 = tx.query_row("SELECT id FROM remote_integration_steps WHERE operation_record_id=?1 AND ordinal=?2 AND window_number=?3",params![record.id,i64::from(intent.step_ordinal),i64::from(window_number)],|row| row.get(0)).map_err(|_| recovery_required())?;
        if parent == record.id
            && step_id == expected_step
            && generation == record.generation
            // The original owner epoch is immutable attempt evidence. A later
            // fenced reacquisition may resume only this identical attempt.
            && epoch >= 0
            && observation.as_slice() == intent.observation_digest
            && input.as_slice() == intent.input_digest
            // A partially-applied owned attempt necessarily changes its own
            // preflight image. The original digest remains immutable audit
            // evidence; result/path input still has to match exactly.
            && preflight.len() == 32
            && confirmation_id == identity_id
        {
            return Ok(());
        }
        return Err(recovery_required());
    }
    validate_child_id(tx, intent.attempt_id)?;
    let step_id: i64 = tx
        .query_row(
            "SELECT id FROM remote_integration_steps WHERE operation_record_id=?1 AND ordinal=?2 AND window_number=?3",
            params![record.id, i64::from(intent.step_ordinal), i64::from(window_number)],
            |row| row.get(0),
        )
        .map_err(|_| recovery_required())?;
    tx.execute("INSERT INTO remote_resolution_attempts(attempt_ulid,operation_record_id,integration_step_id,configuration_generation,owner_epoch,observation_digest,input_digest,preflight_digest,identity_confirmation_id,phase) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,'prepared')",params![attempt,record.id,step_id,record.generation,record.owner_epoch,intent.observation_digest.as_slice(),intent.input_digest.as_slice(),intent.preflight_digest.as_slice(),identity_id]).map_err(|_| recovery_required())?;
    let attempt_id = tx.last_insert_rowid();
    for path in paths {
        tx.execute("INSERT INTO remote_resolution_paths(attempt_id,ordinal,path_digest,expected_digest,result_digest,prewrite_digest,base_blob_oid,local_blob_oid,incoming_blob_oid,mode) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![attempt_id,i64::from(path.ordinal),path.path_digest.as_slice(),path.expected_digest.as_slice(),path.result_digest.as_slice(),path.prewrite_digest.as_slice(),path.base_blob_oid.map(|oid|oid.to_string()),path.local_blob_oid.map(|oid|oid.to_string()),path.incoming_blob_oid.map(|oid|oid.to_string()),i64::from(path.mode)]).map_err(|_| recovery_required())?;
    }
    Ok(())
}

type ResolutionCandidateAttempt = (
    IntegrationStepEvidence,
    Oid,
    String,
    [u8; 32],
    bool,
    Vec<(u32, [u8; 32], [u8; 32], [u8; 32])>,
);

pub(super) fn resolution_candidate_for_attempt(
    connection: &Connection,
    record: &StoredRemoteOperation,
    attempt: crate::repository::OperationId,
) -> Result<Option<ResolutionCandidateAttempt>, RepositoryError> {
    // The attempt's foreign key is authoritative; never infer a resolution
    // step from ordinal zero when a context synchronization has two stages.
    let candidate = connection.query_row(
        "SELECT attempt.candidate_oid,attempt.phase,step.ordinal,attempt.input_digest,attempt.identity_confirmation_id,step.window_number FROM remote_resolution_attempts attempt JOIN remote_integration_steps step ON step.id=attempt.integration_step_id WHERE attempt.attempt_ulid=?1 AND attempt.operation_record_id=?2 AND attempt.configuration_generation=?3 AND step.operation_record_id=attempt.operation_record_id",
        params![attempt.to_string(), record.id, record.generation],
        |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?, row.get::<_, Vec<u8>>(3)?, row.get::<_, Option<i64>>(4)?, row.get::<_, i64>(5)?)),
    ).optional().map_err(|_| recovery_required())?;
    let Some((candidate, phase, ordinal, input_digest, identity_confirmation_id, window_number)) =
        candidate
    else {
        return Ok(None);
    };
    let step = integration_step_in_window(
        connection,
        record.id,
        window_number.try_into().map_err(|_| recovery_required())?,
        ordinal.try_into().map_err(|_| recovery_required())?,
    )?
    .ok_or_else(recovery_required)?;
    if !matches!(
        step.phase,
        IntegrationStepPhase::CommitPrepared | IntegrationStepPhase::Applied
    ) {
        return Ok(None);
    }
    let candidate = candidate.ok_or_else(recovery_required)?;
    let candidate = Oid::from_str(&candidate).map_err(|_| recovery_required())?;
    if step.candidate_oid != Some(candidate)
        || !matches!(phase.as_str(), "candidate_prepared" | "applied")
    {
        return Err(recovery_required());
    }
    let paths = connection
        .prepare("SELECT ordinal,path_digest,expected_digest,result_digest FROM remote_resolution_paths WHERE attempt_id=(SELECT id FROM remote_resolution_attempts WHERE attempt_ulid=?1) ORDER BY ordinal")
        .and_then(|mut statement| {
            statement
                .query_map([attempt.to_string()], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, Vec<u8>>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                        row.get::<_, Vec<u8>>(3)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| recovery_required())?
        .into_iter()
        .map(|(ordinal, path, expected, result)| {
            Ok((
                ordinal.try_into().map_err(|_| recovery_required())?,
                path.try_into().map_err(|_| recovery_required())?,
                expected.try_into().map_err(|_| recovery_required())?,
                result.try_into().map_err(|_| recovery_required())?,
            ))
        })
        .collect::<Result<Vec<_>, RepositoryError>>()?;
    if paths.is_empty() {
        return Err(recovery_required());
    }
    Ok(Some((
        step,
        candidate,
        phase,
        input_digest.try_into().map_err(|_| recovery_required())?,
        identity_confirmation_id.is_some(),
        paths,
    )))
}

#[derive(Clone, Debug)]
pub(super) struct ResolutionPathEvidence {
    pub ordinal: u32,
    pub applied: bool,
    pub prewrite_digest: [u8; 32],
    pub result_digest: [u8; 32],
}

pub(super) fn resolution_paths_for_attempt(
    connection: &Connection,
    record: &StoredRemoteOperation,
    attempt: crate::repository::OperationId,
) -> Result<Vec<ResolutionPathEvidence>, RepositoryError> {
    let paths = connection
        .prepare("SELECT path.ordinal,path.applied,path.prewrite_digest,path.result_digest FROM remote_resolution_paths path JOIN remote_resolution_attempts attempt ON attempt.id=path.attempt_id WHERE attempt.attempt_ulid=?1 AND attempt.operation_record_id=?2 AND attempt.configuration_generation=?3 ORDER BY path.ordinal")
        .and_then(|mut statement| {
            statement
                .query_map(params![attempt.to_string(), record.id, record.generation], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, bool>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                        row.get::<_, Vec<u8>>(3)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| recovery_required())?
        .into_iter()
        .map(|(ordinal, applied, prewrite, result)| {
            Ok(ResolutionPathEvidence {
                ordinal: ordinal.try_into().map_err(|_| recovery_required())?,
                applied,
                prewrite_digest: prewrite.try_into().map_err(|_| recovery_required())?,
                result_digest: result.try_into().map_err(|_| recovery_required())?,
            })
        })
        .collect::<Result<Vec<_>, RepositoryError>>()?;
    if paths.is_empty()
        || paths
            .iter()
            .enumerate()
            .any(|(index, path)| path.ordinal as usize != index)
    {
        return Err(recovery_required());
    }
    Ok(paths)
}

fn resolution_attempt_id(
    connection: &Connection,
    record: &StoredRemoteOperation,
    attempt: crate::repository::OperationId,
) -> Result<(i64, String), RepositoryError> {
    connection.query_row(
        "SELECT id,phase FROM remote_resolution_attempts WHERE attempt_ulid=?1 AND operation_record_id=?2 AND configuration_generation=?3",
        params![attempt.to_string(),record.id,record.generation],
        |row| Ok((row.get(0)?,row.get(1)?)),
    ).optional().map_err(|_| recovery_required())?.ok_or_else(recovery_required)
}

/// Fixed-role manifest provenance. Images and signer authority are bound by the
/// private anchored manifest; signer bytes are derived from immutable candidate ODB.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ResolutionRefLogArtifact {
    pub device: u64,
    pub inode: u64,
    pub digest: [u8; 32],
}

pub(super) fn resolution_ref_log_artifact(
    connection: &Connection,
    record: &StoredRemoteOperation,
    attempt: crate::repository::OperationId,
    role: &str,
) -> Result<Option<ResolutionRefLogArtifact>, RepositoryError> {
    if !matches!(role, "baseline" | "transition") {
        return Err(recovery_required());
    }
    let (id, phase) = resolution_attempt_id(connection, record, attempt)?;
    let row = connection.query_row("SELECT device,inode,digest FROM remote_resolution_ref_log_artifacts WHERE attempt_id=?1 AND role=?2", params![id, role], |row| Ok((row.get::<_,i64>(0)?,row.get::<_,i64>(1)?,row.get::<_,Vec<u8>>(2)?))).optional().map_err(|_| recovery_required())?;
    row.map(|(device, inode, digest)| {
        if inode <= 0
            || !matches!(
                phase.as_str(),
                "paths_applying" | "candidate_prepared" | "applied"
            )
        {
            return Err(recovery_required());
        }
        let index = resolution_index_artifact(connection, record, attempt)?
            .ok_or_else(recovery_required)?;
        if role == "transition"
            && (resolution_candidate_for_attempt(connection, record, attempt)?.is_none()
                || resolution_ref_log_artifact(connection, record, attempt, "baseline")?.is_none()
                || index.phase == "intent")
        {
            return Err(recovery_required());
        }
        Ok(ResolutionRefLogArtifact {
            device: device.try_into().map_err(|_| recovery_required())?,
            inode: inode.try_into().map_err(|_| recovery_required())?,
            digest: digest.try_into().map_err(|_| recovery_required())?,
        })
    })
    .transpose()
}

pub(super) fn prepare_resolution_ref_log_artifact(
    tx: &Transaction<'_>,
    record: &StoredRemoteOperation,
    attempt: crate::repository::OperationId,
    role: &str,
    artifact: &ResolutionRefLogArtifact,
) -> Result<(), RepositoryError> {
    if let Some(existing) = resolution_ref_log_artifact(tx, record, attempt, role)? {
        return (existing == *artifact)
            .then_some(())
            .ok_or_else(recovery_required);
    }
    let (id, phase) = resolution_attempt_id(tx, record, attempt)?;
    let index = resolution_index_artifact(tx, record, attempt)?.ok_or_else(recovery_required)?;
    if index.ref_phase != "not_started"
        || index.phase != "published"
        || (role == "baseline" && phase != "paths_applying")
        || (role == "transition"
            && (phase != "candidate_prepared"
                || resolution_ref_log_artifact(tx, record, attempt, "baseline")?.is_none()))
        || !matches!(role, "baseline" | "transition")
    {
        return Err(recovery_required());
    }
    tx.execute("INSERT INTO remote_resolution_ref_log_artifacts(attempt_id,role,device,inode,digest) VALUES(?1,?2,?3,?4,?5)", params![id,role,i64::try_from(artifact.device).map_err(|_| recovery_required())?,i64::try_from(artifact.inode).map_err(|_| recovery_required())?,artifact.digest.as_slice()]).map_err(|_| recovery_required())?;
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ResolutionIndexArtifact {
    pub device: u64,
    pub inode: u64,
    pub sentinel_digest: [u8; 32],
    pub baseline_digest: [u8; 32],
    pub baseline_identity: (u64, u64),
    pub metadata: [Option<[u8; 32]>; 3],
    pub output: Option<(u64, u64, [u8; 32])>,
    pub ref_phase: String,
    pub phase: String,
}

pub(super) fn resolution_index_artifact(
    connection: &Connection,
    record: &StoredRemoteOperation,
    attempt: crate::repository::OperationId,
) -> Result<Option<ResolutionIndexArtifact>, RepositoryError> {
    let (id, attempt_phase) = resolution_attempt_id(connection, record, attempt)?;
    let row = connection.query_row("SELECT device,inode,sentinel_digest,baseline_digest,merge_head_digest,merge_msg_digest,merge_mode_digest,phase,output_device,output_inode,output_digest,baseline_device,baseline_inode,ref_phase FROM remote_resolution_index_artifacts WHERE attempt_id=?1", [id], |row| Ok((row.get::<_,i64>(0)?,row.get::<_,i64>(1)?,row.get::<_,Vec<u8>>(2)?,row.get::<_,Vec<u8>>(3)?,row.get::<_,Option<Vec<u8>>>(4)?,row.get::<_,Option<Vec<u8>>>(5)?,row.get::<_,Option<Vec<u8>>>(6)?,row.get::<_,String>(7)?,row.get::<_,Option<i64>>(8)?,row.get::<_,Option<i64>>(9)?,row.get::<_,Option<Vec<u8>>>(10)?,row.get::<_,i64>(11)?,row.get::<_,i64>(12)?,row.get::<_,String>(13)?))).optional().map_err(|_| recovery_required())?;
    row.map(
        |(
            device,
            inode,
            sentinel,
            baseline,
            head,
            msg,
            mode,
            phase,
            output_device,
            output_inode,
            output_digest,
            baseline_device,
            baseline_inode,
            ref_phase,
        )| {
            if !matches!(ref_phase.as_str(), "not_started" | "intent" | "observed") {
                return Err(recovery_required());
            }
            if (attempt_phase == "applied" && ref_phase != "observed")
                || (matches!(phase.as_str(), "release_intent" | "released")
                    && (attempt_phase != "applied"
                        || output_digest.is_none()
                        || ref_phase != "observed"))
            {
                return Err(recovery_required());
            }
            if inode <= 0
                || baseline_inode <= 0
                || !matches!(
                    phase.as_str(),
                    "intent" | "published" | "release_intent" | "released"
                )
            {
                return Err(recovery_required());
            }
            Ok(ResolutionIndexArtifact {
                device: device.try_into().map_err(|_| recovery_required())?,
                inode: inode.try_into().map_err(|_| recovery_required())?,
                sentinel_digest: sentinel.try_into().map_err(|_| recovery_required())?,
                baseline_digest: baseline.try_into().map_err(|_| recovery_required())?,
                baseline_identity: (
                    baseline_device
                        .try_into()
                        .map_err(|_| recovery_required())?,
                    baseline_inode.try_into().map_err(|_| recovery_required())?,
                ),
                metadata: [head, msg, mode]
                    .into_iter()
                    .map(|digest| {
                        digest
                            .map(|digest| digest.try_into().map_err(|_| recovery_required()))
                            .transpose()
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .try_into()
                    .map_err(|_| recovery_required())?,
                output: match (output_device, output_inode, output_digest) {
                    (None, None, None) => None,
                    (Some(device), Some(inode), Some(digest)) if inode > 0 => Some((
                        device.try_into().map_err(|_| recovery_required())?,
                        inode.try_into().map_err(|_| recovery_required())?,
                        digest.try_into().map_err(|_| recovery_required())?,
                    )),
                    _ => return Err(recovery_required()),
                },
                ref_phase,
                phase,
            })
        },
    )
    .transpose()
}

pub(super) fn prepare_resolution_index_artifact(
    tx: &Transaction<'_>,
    record: &StoredRemoteOperation,
    attempt: crate::repository::OperationId,
    artifact: &ResolutionIndexArtifact,
) -> Result<(), RepositoryError> {
    let (id, phase) = resolution_attempt_id(tx, record, attempt)?;
    if let Some(existing) = resolution_index_artifact(tx, record, attempt)? {
        let mut intended = artifact.clone();
        intended.phase = existing.phase.clone();
        intended.output = existing.output;
        intended.ref_phase = existing.ref_phase.clone();
        return (existing == intended)
            .then_some(())
            .ok_or_else(recovery_required);
    }
    if phase != "paths_applying"
        || artifact.phase != "intent"
        || artifact.ref_phase != "not_started"
    {
        return Err(recovery_required());
    }
    tx.execute("INSERT INTO remote_resolution_index_artifacts(attempt_id,device,inode,sentinel_digest,baseline_digest,baseline_device,baseline_inode,merge_head_digest,merge_msg_digest,merge_mode_digest,phase) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,'intent')", params![id,i64::try_from(artifact.device).map_err(|_| recovery_required())?,i64::try_from(artifact.inode).map_err(|_| recovery_required())?,artifact.sentinel_digest.as_slice(),artifact.baseline_digest.as_slice(),i64::try_from(artifact.baseline_identity.0).map_err(|_| recovery_required())?,i64::try_from(artifact.baseline_identity.1).map_err(|_| recovery_required())?,artifact.metadata[0].as_ref().map(|digest|digest.as_slice()),artifact.metadata[1].as_ref().map(|digest|digest.as_slice()),artifact.metadata[2].as_ref().map(|digest|digest.as_slice())]).map_err(|_| recovery_required())?;
    Ok(())
}

pub(super) fn prepare_resolution_index_output(
    tx: &Transaction<'_>,
    record: &StoredRemoteOperation,
    attempt: crate::repository::OperationId,
    output: (u64, u64, [u8; 32]),
) -> Result<(), RepositoryError> {
    let (id, phase) = resolution_attempt_id(tx, record, attempt)?;
    let artifact = resolution_index_artifact(tx, record, attempt)?.ok_or_else(recovery_required)?;
    if let Some(existing) = artifact.output {
        return (existing == output)
            .then_some(())
            .ok_or_else(recovery_required);
    }
    if phase != "candidate_prepared" || artifact.phase != "published" {
        return Err(recovery_required());
    }
    tx.execute("UPDATE remote_resolution_index_artifacts SET output_device=?2,output_inode=?3,output_digest=?4 WHERE attempt_id=?1", params![id,i64::try_from(output.0).map_err(|_| recovery_required())?,i64::try_from(output.1).map_err(|_| recovery_required())?,output.2.as_slice()]).map_err(|_| recovery_required())?;
    Ok(())
}

pub(super) fn advance_resolution_ref_effect(
    tx: &Transaction<'_>,
    record: &StoredRemoteOperation,
    attempt: crate::repository::OperationId,
    next: &str,
) -> Result<(), RepositoryError> {
    let (id, phase) = resolution_attempt_id(tx, record, attempt)?;
    let artifact = resolution_index_artifact(tx, record, attempt)?.ok_or_else(recovery_required)?;
    resolution_ref_log_artifact(tx, record, attempt, "transition")?
        .ok_or_else(recovery_required)?;
    if artifact.ref_phase == "observed" && next == "observed" {
        return Ok(());
    }
    if phase != "candidate_prepared"
        || artifact.phase != "published"
        || artifact.output.is_none()
        || !matches!(
            (artifact.ref_phase.as_str(), next),
            ("not_started", "intent")
                | ("intent", "intent")
                | ("intent", "observed")
                | ("not_started", "observed")
        )
    {
        return Err(recovery_required());
    }
    tx.execute(
        "UPDATE remote_resolution_index_artifacts SET ref_phase=?2 WHERE attempt_id=?1",
        params![id, next],
    )
    .map_err(|_| recovery_required())?;
    Ok(())
}

pub(super) fn advance_resolution_index_artifact(
    tx: &Transaction<'_>,
    record: &StoredRemoteOperation,
    attempt: crate::repository::OperationId,
    next: &str,
) -> Result<(), RepositoryError> {
    let (id, phase) = resolution_attempt_id(tx, record, attempt)?;
    let artifact = resolution_index_artifact(tx, record, attempt)?.ok_or_else(recovery_required)?;
    if matches!(next, "release_intent" | "released")
        && (phase != "applied" || artifact.output.is_none() || artifact.ref_phase != "observed")
    {
        return Err(recovery_required());
    }
    if artifact.phase == next {
        return Ok(());
    }
    if !matches!(
        (artifact.phase.as_str(), next),
        ("intent", "published") | ("published", "release_intent") | ("release_intent", "released")
    ) {
        return Err(recovery_required());
    }
    tx.execute(
        "UPDATE remote_resolution_index_artifacts SET phase=?2 WHERE attempt_id=?1",
        params![id, next],
    )
    .map_err(|_| recovery_required())?;
    Ok(())
}

pub(super) fn begin_resolution_path_effects(
    tx: &Transaction<'_>,
    record: &StoredRemoteOperation,
    attempt: crate::repository::OperationId,
) -> Result<(), RepositoryError> {
    let (attempt_id, phase) = resolution_attempt_id(tx, record, attempt)?;
    if phase == "prepared" {
        tx.execute(
            "UPDATE remote_resolution_attempts SET phase='paths_applying' WHERE id=?1",
            [attempt_id],
        )
        .map_err(|_| recovery_required())?;
        tx.execute("UPDATE remote_integration_steps SET phase='resolution_prepared' WHERE id=(SELECT integration_step_id FROM remote_resolution_attempts WHERE id=?1) AND phase='conflict_pending'", [attempt_id]).map_err(|_| recovery_required())?;
    } else if !matches!(
        phase.as_str(),
        "paths_applying" | "candidate_prepared" | "applied"
    ) {
        return Err(recovery_required());
    }
    Ok(())
}

pub(super) fn observe_resolution_path_effect(
    tx: &Transaction<'_>,
    record: &StoredRemoteOperation,
    attempt: crate::repository::OperationId,
    ordinal: u32,
) -> Result<(), RepositoryError> {
    let (attempt_id, phase) = resolution_attempt_id(tx, record, attempt)?;
    if matches!(phase.as_str(), "candidate_prepared" | "applied") {
        let applied: bool = tx
            .query_row(
                "SELECT applied FROM remote_resolution_paths WHERE attempt_id=?1 AND ordinal=?2",
                params![attempt_id, i64::from(ordinal)],
                |row| row.get(0),
            )
            .map_err(|_| recovery_required())?;
        return applied.then_some(()).ok_or_else(recovery_required);
    }
    if phase != "paths_applying" {
        return Err(recovery_required());
    }
    let changed = tx.execute("UPDATE remote_resolution_paths SET applied=1 WHERE attempt_id=?1 AND ordinal=?2 AND applied=0",params![attempt_id,i64::from(ordinal)]).map_err(|_| recovery_required())?;
    if changed == 0 {
        let applied: bool = tx
            .query_row(
                "SELECT applied FROM remote_resolution_paths WHERE attempt_id=?1 AND ordinal=?2",
                params![attempt_id, i64::from(ordinal)],
                |row| row.get(0),
            )
            .map_err(|_| recovery_required())?;
        if !applied {
            return Err(recovery_required());
        }
    }
    Ok(())
}

pub(super) fn prepare_resolution_candidate(
    tx: &Transaction<'_>,
    record: &StoredRemoteOperation,
    attempt: crate::repository::OperationId,
    candidate_oid: Oid,
) -> Result<(), RepositoryError> {
    let (attempt_id, phase) = resolution_attempt_id(tx, record, attempt)?;
    if phase == "candidate_prepared" {
        let existing: String = tx
            .query_row(
                "SELECT candidate_oid FROM remote_resolution_attempts WHERE id=?1",
                [attempt_id],
                |row| row.get(0),
            )
            .map_err(|_| recovery_required())?;
        return (Oid::from_str(&existing).map_err(|_| recovery_required())? == candidate_oid)
            .then_some(())
            .ok_or_else(recovery_required);
    }
    if phase != "paths_applying" {
        return Err(recovery_required());
    }
    let incomplete: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM remote_resolution_paths WHERE attempt_id=?1 AND applied=0)",[attempt_id],|row|row.get(0)).map_err(|_| recovery_required())?;
    if incomplete {
        return Err(recovery_required());
    }
    tx.execute("UPDATE remote_resolution_attempts SET phase='candidate_prepared',candidate_oid=?2 WHERE id=?1",params![attempt_id,candidate_oid.to_string()]).map_err(|_| recovery_required())?;
    tx.execute("UPDATE remote_integration_steps SET phase='commit_prepared',candidate_oid=?2 WHERE id=(SELECT integration_step_id FROM remote_resolution_attempts WHERE id=?1) AND phase='resolution_prepared'",params![attempt_id,candidate_oid.to_string()]).map_err(|_| recovery_required())?;
    Ok(())
}

pub(super) fn observe_resolution_checkpoint(
    tx: &Transaction<'_>,
    record: &StoredRemoteOperation,
    attempt: crate::repository::OperationId,
    checkpoint_oid: Oid,
    observed_tree_oid: Oid,
) -> Result<(), RepositoryError> {
    let (attempt_id, phase) = resolution_attempt_id(tx, record, attempt)?;
    if phase == "applied" {
        let checkpoint: String = tx
            .query_row(
                "SELECT checkpoint_oid FROM remote_resolution_attempts WHERE id=?1",
                [attempt_id],
                |row| row.get(0),
            )
            .map_err(|_| recovery_required())?;
        return (Oid::from_str(&checkpoint).map_err(|_| recovery_required())? == checkpoint_oid)
            .then_some(())
            .ok_or_else(recovery_required);
    }
    if phase != "candidate_prepared" {
        return Err(recovery_required());
    }
    let candidate: String = tx
        .query_row(
            "SELECT candidate_oid FROM remote_resolution_attempts WHERE id=?1",
            [attempt_id],
            |row| row.get(0),
        )
        .map_err(|_| recovery_required())?;
    if Oid::from_str(&candidate).map_err(|_| recovery_required())? != checkpoint_oid {
        return Err(recovery_required());
    }
    tx.execute(
        "UPDATE remote_resolution_attempts SET phase='applied',checkpoint_oid=?2 WHERE id=?1",
        params![attempt_id, checkpoint_oid.to_string()],
    )
    .map_err(|_| recovery_required())?;
    tx.execute("UPDATE remote_integration_steps SET phase='applied',result_oid=?2,observed_tree_oid=?3 WHERE id=(SELECT integration_step_id FROM remote_resolution_attempts WHERE id=?1) AND phase='commit_prepared'",params![attempt_id,checkpoint_oid.to_string(),observed_tree_oid.to_string()]).map_err(|_| recovery_required())?;
    tx.execute(
        "UPDATE repositories SET refresh_required=1 WHERE id=(SELECT repository_id FROM remote_operation_records WHERE id=?1)",
        [record.id],
    )
    .map_err(|_| recovery_required())?;
    Ok(())
}

pub(super) fn begin_identity_confirmation_effect(
    tx: &Transaction<'_>,
    record: &StoredRemoteOperation,
    confirmation: crate::repository::OperationId,
) -> Result<(), RepositoryError> {
    let changed = tx.execute("UPDATE remote_identity_confirmations SET phase='applying' WHERE confirmation_ulid=?1 AND operation_record_id=?2 AND configuration_generation=?3 AND phase='prepared'",params![confirmation.to_string(),record.id,record.generation]).map_err(|_| recovery_required())?;
    if changed == 1 {
        return Ok(());
    }
    let phase: String = tx.query_row("SELECT phase FROM remote_identity_confirmations WHERE confirmation_ulid=?1 AND operation_record_id=?2",params![confirmation.to_string(),record.id],|row| row.get(0)).map_err(|_| recovery_required())?;
    if matches!(phase.as_str(), "applying" | "applied") {
        Ok(())
    } else {
        Err(recovery_required())
    }
}

pub(super) fn observe_identity_confirmation_effect(
    tx: &Transaction<'_>,
    record: &StoredRemoteOperation,
    confirmation: crate::repository::OperationId,
    applied_configuration_digest: [u8; 32],
) -> Result<(), RepositoryError> {
    let changed = tx.execute("UPDATE remote_identity_confirmations SET phase='applied',applied_configuration_digest=?4 WHERE confirmation_ulid=?1 AND operation_record_id=?2 AND configuration_generation=?3 AND phase='applying'",params![confirmation.to_string(),record.id,record.generation,applied_configuration_digest.as_slice()]).map_err(|_| recovery_required())?;
    if changed == 1 {
        return Ok(());
    }
    let phase: String = tx.query_row("SELECT phase FROM remote_identity_confirmations WHERE confirmation_ulid=?1 AND operation_record_id=?2",params![confirmation.to_string(),record.id],|row| row.get(0)).map_err(|_| recovery_required())?;
    if phase == "applied" {
        Ok(())
    } else {
        Err(recovery_required())
    }
}

/// Inserts a validated immutable audit envelope. Arbitration and transitions
/// belong to the controller; this helper does not acquire a Git mutation lock.
#[allow(dead_code)]
pub(in super::super) fn insert_operation(
    transaction: &Transaction<'_>,
    repository_id: i64,
    operation_id: crate::repository::OperationId,
    target: &RemoteOperationTarget,
    priority: RemoteOperationPriority,
    created_at: i64,
) -> Result<i64, RepositoryError> {
    read_snapshot(transaction, repository_id)?;
    let policy = read_policy(transaction, repository_id)?;
    let plan = policy.plan.as_ref().ok_or_else(recovery_required)?;
    if created_at < 0
        || target.remote_name() != plan.remote_name()
        || target.primary_ref() != plan.primary()
    {
        return Err(recovery_required());
    }
    let action = match target.action() {
        RemoteOperationAction::Poll => "poll",
        RemoteOperationAction::SynchronizeContext => "synchronize_context",
        RemoteOperationAction::SynchronizePrimary => "synchronize_primary",
        RemoteOperationAction::Promote => "promote",
        RemoteOperationAction::Close => "close",
    };
    let priority = match priority {
        RemoteOperationPriority::Poll => "poll",
        RemoteOperationPriority::Manual => "manual",
    };
    transaction.execute("INSERT INTO remote_operation_records(repository_id,operation_ulid,configuration_generation,remote_name,primary_branch,primary_ref,primary_tracking_ref,context_ref,context_tracking_ref,kind,item_id,local_branch,action,priority,phase,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,'reserved',?15,?15)",params![repository_id,operation_id.to_string(),policy.generation,target.remote_name(),plan.primary_branch(),target.primary_ref().remote_ref(),target.primary_ref().tracking_ref(),target.context_ref().map(RemoteRefTarget::remote_ref),target.context_ref().map(RemoteRefTarget::tracking_ref),target.item().map(|(kind,_)|match kind {AuthoringKind::Document=>"document",AuthoringKind::Ticket=>"ticket"}),target.item().map(|(_,id)|id.to_string()),target.local_branch(),action,priority,created_at]).map_err(|_| recovery_required())?;
    let record_id = transaction.last_insert_rowid();
    if matches!(
        target.action(),
        RemoteOperationAction::SynchronizePrimary | RemoteOperationAction::SynchronizeContext
    ) {
        transaction.execute("INSERT INTO remote_integration_windows(operation_record_id,number,configuration_generation,owner_epoch,kind) VALUES(?1,0,?2,0,'legacy')", params![record_id, policy.generation]).map_err(|_| recovery_required())?;
    }
    Ok(record_id)
}

struct StoredPolicy {
    polling: RemotePollingConfiguration,
    history_unknown: bool,
    generation: i64,
    plan: Option<RemoteRefPlan>,
    latest_outcome: Option<RemoteOutcomeCategory>,
}

fn read_policy(
    connection: &Connection,
    repository_id: i64,
) -> Result<StoredPolicy, RepositoryError> {
    let values = connection.query_row("SELECT enabled,paused,interval_seconds,automatic_backoff_seconds,recovery_suspended,history_unknown,configuration_generation,remote_name,primary_branch,latest_outcome,endpoint_digest FROM remote_polling_state WHERE repository_id=?1", [repository_id], |row| {
        Ok((row.get::<_,i64>(0)?,row.get::<_,i64>(1)?,row.get::<_,i64>(2)?,row.get::<_,Option<i64>>(3)?,row.get::<_,i64>(4)?,row.get::<_,i64>(5)?,row.get::<_,i64>(6)?,row.get::<_,Option<String>>(7)?,row.get::<_,Option<String>>(8)?,row.get::<_,Option<String>>(9)?,row.get::<_,Option<Vec<u8>>>(10)?))
    }).map_err(|_| recovery_required())?;
    let (
        enabled,
        paused,
        interval,
        backoff,
        suspended,
        unknown,
        generation,
        remote,
        primary,
        latest,
        endpoint_digest,
    ) = values;
    if generation < 0
        || endpoint_digest
            .as_ref()
            .is_some_and(|digest| digest.len() != 32)
    {
        return Err(recovery_required());
    }
    let plan = match (remote, primary) {
        (None, None) => None,
        (Some(remote), Some(primary)) => Some(
            RemoteRefPlan::from_configuration(&remote, &primary)
                .map_err(|_| recovery_required())?,
        ),
        _ => return Err(recovery_required()),
    };
    Ok(StoredPolicy {
        polling: RemotePollingConfiguration::new(
            flag(enabled)?,
            flag(paused)?,
            PollingInterval::from_seconds(interval.try_into().map_err(|_| recovery_required())?)
                .map_err(|_| recovery_required())?,
            backoff
                .map(|seconds| {
                    AutomaticBackoff::from_seconds(
                        seconds.try_into().map_err(|_| recovery_required())?,
                    )
                    .map_err(|_| recovery_required())
                })
                .transpose()?,
            flag(suspended)?,
        ),
        history_unknown: flag(unknown)?,
        generation,
        plan,
        latest_outcome: latest.as_deref().map(outcome).transpose()?,
    })
}

/// Exhaustive integrity audit at service initialization, after migration has
/// committed. Retained history is checked in a read transaction, never in an
/// ordinary state transition's immediate write transaction.
pub(in super::super) fn audit_registry(connection: &mut Connection) -> Result<(), RepositoryError> {
    let transaction = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)
        .map_err(|_| recovery_required())?;
    validate_merge_evidence_schema(&transaction)?;
    for table in [
        "remote_resolution_ref_log_artifacts",
        "remote_resolution_index_artifacts",
        "remote_polling_state",
        "remote_observation_batches",
        "remote_ref_observations",
        "remote_context_states",
        "remote_operation_records",
        "remote_integration_steps",
        "remote_integration_windows",
        "remote_identity_confirmations",
        "remote_resolution_attempts",
        "remote_resolution_paths",
    ] {
        if transaction
            .prepare(&format!("PRAGMA foreign_key_check({table})"))
            .and_then(|mut statement| statement.exists([]))
            .map_err(|_| recovery_required())?
        {
            return Err(recovery_required());
        }
    }
    let repository_ids = transaction
        .prepare("SELECT id FROM repositories ORDER BY id")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, i64>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| recovery_required())?;
    for repository_id in repository_ids {
        read_snapshot_rows(&transaction, repository_id, true)?;
    }
    transaction.rollback().map_err(|_| recovery_required())
}

/// Routine reconstruction is bounded by current refs, context evidence and
/// active operations, independent of the number of completed poll attempts.
pub(in super::super) fn read_snapshot(
    connection: &Connection,
    repository_id: i64,
) -> Result<RemoteSnapshot, RepositoryError> {
    read_snapshot_rows(connection, repository_id, false)
}

fn read_snapshot_rows(
    connection: &Connection,
    repository_id: i64,
    audit_history: bool,
) -> Result<RemoteSnapshot, RepositoryError> {
    let policy = read_policy(connection, repository_id)?;
    if audit_history {
        read_operations(connection, repository_id)?;
    } else {
        read_operation_rows(connection, repository_id, true)?;
    }
    let mut snapshot = RemoteSnapshot::new(
        policy.polling,
        policy.latest_outcome,
        Vec::new(),
        Vec::new(),
    );
    snapshot.history_unknown = policy.history_unknown;
    let query = if audit_history {
        "SELECT id,remote_name,primary_branch,configuration_generation,observed_at,is_current FROM remote_observation_batches WHERE repository_id=?1 ORDER BY id"
    } else {
        "SELECT id,remote_name,primary_branch,configuration_generation,observed_at,is_current FROM remote_observation_batches WHERE repository_id=?1 AND is_current=1 ORDER BY id"
    };
    let batches = connection
        .prepare(query)
        .and_then(|mut statement| {
            statement
                .query_map([repository_id], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, i64>(5)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| recovery_required())?;
    let mut current_count = 0;
    for (batch_id, remote, primary, generation, observed_at, current) in batches {
        let plan = RemoteRefPlan::from_configuration(&remote, &primary)
            .map_err(|_| recovery_required())?;
        if observed_at < 0 {
            return Err(recovery_required());
        }
        let current = flag(current)?;
        if generation != policy.generation || policy.plan.as_ref() != Some(&plan) {
            // Only an exact window-bound historical pass may outlive the
            // configuration that selected it. The ordinary snapshot never
            // selects these rows; startup still audits their complete contents.
            let historical_pass: bool = if audit_history && !current {
                connection.query_row(
                    "SELECT EXISTS(SELECT 1 FROM remote_integration_windows window_pass JOIN remote_operation_records operation ON operation.id=window_pass.operation_record_id WHERE window_pass.observation_batch_id=?1 AND window_pass.kind='pinned' AND window_pass.configuration_generation=?2 AND operation.configuration_generation=?2 AND operation.repository_id=?3 AND operation.remote_name=?4 AND operation.primary_branch=?5)",
                    params![batch_id, generation, repository_id, plan.remote_name(), plan.primary_branch()],
                    |row| row.get(0),
                ).map_err(|_| recovery_required())?
            } else {
                false
            };
            if !historical_pass {
                return Err(recovery_required());
            }
        }
        current_count += usize::from(current);
        if current_count > 1 {
            return Err(recovery_required());
        }
        let rows = connection.prepare("SELECT ordinal,remote_ref,tracking_ref,classification,advertised_oid,tracking_oid FROM remote_ref_observations WHERE batch_id=?1 ORDER BY ordinal")
            .and_then(|mut statement| statement.query_map([batch_id],|row| Ok((row.get::<_,i64>(0)?,row.get::<_,Option<String>>(1)?,row.get::<_,Option<String>>(2)?,row.get::<_,String>(3)?,row.get::<_,String>(4)?,row.get::<_,Option<String>>(5)?)))?.collect::<Result<Vec<_>,_>>()).map_err(|_| recovery_required())?;
        for (ordinal, remote_ref, tracking_ref, classification, advertised, tracking) in rows {
            if ordinal < 0 {
                return Err(recovery_required());
            }
            let advertised_oid = oid(Some(advertised))?.ok_or_else(recovery_required)?;
            let tracking_oid = oid(tracking)?;
            let observation = match (remote_ref, tracking_ref, classification.as_str()) {
                (None, None, "malformed") => RemoteRefObservation {
                    target: None,
                    advertised_oid,
                    tracking_oid,
                    classification: RemoteRefClassification::MalformedContext,
                },
                (Some(remote_ref), Some(tracking_ref), classification) => {
                    let observation = RemoteRefObservation::from_advertisement(
                        &plan,
                        &remote_ref,
                        advertised_oid,
                        tracking_oid,
                    )
                    .ok_or_else(recovery_required)?;
                    let expected_class = match observation.classification {
                        RemoteRefClassification::Primary => "primary",
                        RemoteRefClassification::RecognizedContext { .. } => "context",
                        // Malformed names are never persisted: only their fixed classification and OIDs.
                        RemoteRefClassification::MalformedContext => {
                            return Err(recovery_required());
                        }
                    };
                    if expected_class != classification
                        || observation.tracking_ref() != Some(tracking_ref.as_str())
                    {
                        return Err(recovery_required());
                    }
                    observation
                }
                _ => return Err(recovery_required()),
            };
            if current {
                if observation.classification == RemoteRefClassification::MalformedContext {
                    snapshot.contexts.push(RemoteContextSnapshot::new(
                        None,
                        None,
                        None,
                        Some(advertised_oid),
                        tracking_oid,
                        RemotePublicationEvidence::HistoryUnknown,
                        RemoteContextState::Malformed,
                    ));
                }
                snapshot.observations.push(observation);
            }
        }
    }
    let mut current_contexts: std::collections::HashMap<_, _> = snapshot
        .observations
        .iter()
        .filter(|observation| {
            matches!(
                observation.classification,
                RemoteRefClassification::RecognizedContext { .. }
            )
        })
        .map(|observation| {
            (
                observation.remote_ref().expect("validated context target"),
                observation,
            )
        })
        .collect();
    let contexts = connection.prepare("SELECT remote_ref,tracking_ref,kind,item_id,last_advertised_oid,tracking_oid,publication_evidence,state,observed_at FROM remote_context_states WHERE repository_id=?1 ORDER BY remote_ref")
        .and_then(|mut statement| statement.query_map([repository_id],|row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,Option<String>>(4)?,row.get::<_,Option<String>>(5)?,row.get::<_,String>(6)?,row.get::<_,String>(7)?,row.get::<_,i64>(8)?)))?.collect::<Result<Vec<_>,_>>()).map_err(|_| recovery_required())?;
    for (remote_ref, tracking_ref, kind, id, advertised, tracking, evidence, state, observed_at) in
        contexts
    {
        let kind = match kind.as_str() {
            "document" => AuthoringKind::Document,
            "ticket" => AuthoringKind::Ticket,
            _ => return Err(recovery_required()),
        };
        let id: ItemId = id.parse().map_err(|_| recovery_required())?;
        let target = policy
            .plan
            .as_ref()
            .ok_or_else(recovery_required)?
            .context(kind, &id);
        if target.remote_ref() != remote_ref
            || target.tracking_ref() != tracking_ref
            || observed_at < 0
        {
            return Err(recovery_required());
        }
        let evidence = match evidence.as_str() {
            "never_published" => RemotePublicationEvidence::NeverPublished,
            "observed_published" => RemotePublicationEvidence::ObservedPublished,
            "history_unknown" => RemotePublicationEvidence::HistoryUnknown,
            _ => return Err(recovery_required()),
        };
        let state = match state.as_str() {
            "observed" => RemoteContextState::Observed,
            "unmaterialized" => RemoteContextState::Unmaterialized,
            "malformed" => RemoteContextState::Malformed,
            "remotely_deleted" => RemoteContextState::RemotelyDeleted,
            "history_unknown" => RemoteContextState::HistoryUnknown,
            _ => return Err(recovery_required()),
        };
        let advertised = oid(advertised)?;
        if (state != RemoteContextState::HistoryUnknown)
            != (evidence == RemotePublicationEvidence::ObservedPublished)
            || (evidence == RemotePublicationEvidence::ObservedPublished) != advertised.is_some()
            || (snapshot.history_unknown && evidence == RemotePublicationEvidence::NeverPublished)
        {
            return Err(recovery_required());
        }
        let tracking = oid(tracking)?;
        match current_contexts.remove(remote_ref.as_str()) {
            Some(observation)
                if matches!(
                    state,
                    RemoteContextState::Observed
                        | RemoteContextState::Unmaterialized
                        | RemoteContextState::Malformed
                ) && evidence == RemotePublicationEvidence::ObservedPublished
                    && advertised == Some(observation.advertised_oid)
                    && tracking == observation.tracking_oid => {}
            None if matches!(
                state,
                RemoteContextState::RemotelyDeleted | RemoteContextState::HistoryUnknown
            ) => {}
            _ => return Err(recovery_required()),
        }
        snapshot.contexts.push(RemoteContextSnapshot::new(
            Some(target),
            Some(kind),
            Some(id),
            advertised,
            tracking,
            evidence,
            state,
        ));
    }
    if !current_contexts.is_empty() {
        return Err(recovery_required());
    }
    // Local discovery has its own validation and recovery contract. Startup's
    // remote audit must not reinterpret unrelated local-cache corruption.
    if audit_history {
        return Ok(snapshot);
    }
    let local_contexts = connection.prepare("SELECT branch,item_id FROM contexts WHERE repository_id=?1 AND kind='active' ORDER BY branch")
        .and_then(|mut statement| statement.query_map([repository_id],|row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?)))?.collect::<Result<Vec<_>,_>>()).map_err(|_| recovery_required())?;
    for (branch, id) in local_contexts {
        let (kind, item_id) =
            crate::repository::stored_authoring_branch(&branch).ok_or_else(recovery_required)?;
        if item_id.to_string() != id {
            return Err(recovery_required());
        }
        if let Some(context) = snapshot.contexts.iter_mut().find(|context| {
            context.kind == Some(kind) && context.item_id.as_ref() == Some(&item_id)
        }) {
            if context.state == RemoteContextState::Unmaterialized {
                context.state = RemoteContextState::Observed;
            }
            continue;
        }
        let target = policy
            .plan
            .as_ref()
            .map(|plan| plan.context(kind, &item_id));
        let evidence = snapshot.publication_evidence_for(kind, &item_id);
        snapshot.contexts.push(RemoteContextSnapshot::new(
            target,
            Some(kind),
            Some(item_id),
            None,
            None,
            evidence,
            RemoteContextState::HistoryUnknown,
        ));
    }
    Ok(snapshot)
}

impl crate::repository::RepositoryService {
    /// Explicitly release cache-recovery suspension. Durable pause, polling
    /// policy and unknown publication history remain independent.
    pub fn resume_remote_polling_after_recovery(&self, root: &Path) -> Result<(), RepositoryError> {
        with_transaction(self, root, |transaction, id| {
            read_snapshot(transaction, id)?;
            transaction
                .execute(
                    "UPDATE remote_polling_state SET recovery_suspended=0 WHERE repository_id=?1",
                    [id],
                )
                .map_err(|_| recovery_required())?;
            Ok(())
        })
    }

    pub fn remote_snapshot(&self, root: &Path) -> Result<RemoteSnapshot, RepositoryError> {
        self.repository_snapshot(root)
            .map(|snapshot| snapshot.remote)
    }

    pub fn set_remote_polling(
        &self,
        root: &Path,
        enabled: bool,
        paused: bool,
        interval: PollingInterval,
    ) -> Result<(), RepositoryError> {
        with_transaction(self, root, |transaction, id| {
            read_snapshot(transaction, id)?;
            transaction.execute("UPDATE remote_polling_state SET enabled=?2,paused=?3,interval_seconds=?4 WHERE repository_id=?1",
                params![id,enabled,paused,interval.duration().as_secs()]).map_err(|_| recovery_required())?;
            Ok(())
        })
    }
}

/// A short cache-recovery-guarded transaction, never held across Git or network
/// work. The registry is an observation cache, not a repository mutation lock.
pub(in super::super) fn with_transaction<T>(
    service: &crate::repository::RepositoryService,
    root: &Path,
    update: impl FnOnce(&Transaction<'_>, i64) -> Result<T, RepositoryError>,
) -> Result<T, RepositoryError> {
    use crate::repository::{cache_read_guard, open_registry, registry_root_key};
    let operation = RepositoryOperation::RepositorySnapshot;
    service.require_index_available(operation, Some(root))?;
    let root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_owned());
    let _guard = cache_read_guard(&service.registry_path, &root, operation)?;
    let mut connection =
        open_registry(&service.registry_path, &mut |_| {}).map_err(|_| recovery_required())?;
    let transaction = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|_| recovery_required())?;
    let id = transaction
        .query_row(
            "SELECT id FROM repositories WHERE root_path=?1",
            [registry_root_key(&root, operation)?],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| recovery_required())?
        .ok_or_else(|| {
            RepositoryError::new(
                operation,
                None,
                RepositoryErrorKind::RepositoryNotRegistered,
                "the repository is not registered",
            )
        })?;
    let result = update(&transaction, id)?;
    transaction.commit().map_err(|_| recovery_required())?;
    Ok(result)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteObservationOutcome {
    category: RemoteOutcomeCategory,
    snapshot: RemoteSnapshot,
}

impl RemoteObservationOutcome {
    #[allow(dead_code)]
    pub(crate) fn new(category: RemoteOutcomeCategory, snapshot: RemoteSnapshot) -> Self {
        Self { category, snapshot }
    }

    pub fn category(&self) -> RemoteOutcomeCategory {
        self.category
    }

    pub fn snapshot(&self) -> &RemoteSnapshot {
        &self.snapshot
    }
}
