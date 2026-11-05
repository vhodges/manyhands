//! Durable retirement authority. The entire additive envelope is installed only
//! after the remote, local and request journals exist; it is never reconstructed
//! from today's Git state. No canonical content is stored here.
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use super::super::{RepositoryError, remote::state::recovery_required};

const MARKER: &str = "context_retirement_v1";
const REENTRY_MARKER: &str = "context_retirement_v2";
#[cfg(test)]
thread_local! {static MIGRATION_FAILURE:std::cell::Cell<Option<u8>>=const {std::cell::Cell::new(None)};}
#[cfg(test)]
thread_local! {static RELEASE_FAILURE:std::cell::Cell<bool>=const {std::cell::Cell::new(false)};}
#[cfg(test)]
pub(super) fn set_release_failure(fail: bool) -> bool {
    RELEASE_FAILURE.with(|slot| slot.replace(fail))
}
#[cfg(test)]
pub(super) fn fail_migration_at(point: u8) {
    MIGRATION_FAILURE.with(|slot| slot.set(Some(point)));
}
#[cfg(test)]
fn migration_checkpoint(point: u8) -> Result<(), RepositoryError> {
    if MIGRATION_FAILURE.with(|slot| {
        if slot.get() == Some(point) {
            slot.set(None);
            true
        } else {
            false
        }
    }) {
        return Err(recovery_required());
    }
    Ok(())
}

#[path = "authority.rs"]
mod authority;
pub(in super::super) use authority::ObserverExecution;
pub(super) use authority::{
    BranchClaim, bind_reentry_observer, claim_branch, claim_present, preserve_worktree_resumption,
    refuse_before_branch_effect, refuse_before_worktree_effect, refuse_execution, refuse_fresh,
    settle_branch_after_prune,
};

/// Retirement identities are spent even after their executor ends. Include the
/// envelope independently of its companion, so companion loss never frees an ID.
pub(in super::super) fn owned_identity(db: &Connection, id: &str) -> Result<bool, RepositoryError> {
    for (table, column, predicate) in [
        (
            "operation_records",
            "operation_ulid",
            " AND action='retire_context'",
        ),
        ("context_retirement_records", "operation_ulid", ""),
        ("context_retirement_attempts", "observer_operation_ulid", ""),
        (
            "retirement_advertisement_provenance",
            "source_operation_ulid",
            "",
        ),
        (
            "retirement_advertisement_provenance",
            "execution_parent_ulid",
            "",
        ),
        (
            "retirement_advertisement_provenance",
            "execution_batch_operation_ulid",
            "",
        ),
        (
            "retirement_observation_receipts",
            "consumer_operation_ulid",
            "",
        ),
    ] {
        if super::super::recovery::has_table(db, table).map_err(|_| recovery_required())?
            && db
                .query_row(
                    &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE {column}=?1{predicate})"),
                    [id],
                    |r| r.get::<_, bool>(0),
                )
                .map_err(|_| recovery_required())?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(in super::super) fn require_foreign_id_available(
    db: &Connection,
    id: super::super::OperationId,
) -> Result<(), RepositoryError> {
    if owned_identity(db, &id.to_string())? {
        return Err(recovery_required());
    }
    Ok(())
}

/// A generic refresh may borrow only this exact authoritative, still-owed
/// handoff. Terminal replay belongs to the retirement result reader, not a new
/// discovery pass. Auditing first also refuses a lost companion/attempt.
pub(in super::super) fn refresh_allowed(
    db: &Connection,
    operation: &str,
    root: &std::path::Path,
) -> Result<bool, RepositoryError> {
    if !super::super::recovery::has_table(db, "context_retirement_records")
        .map_err(|_| recovery_required())?
    {
        // A pre-retirement journal may be queried before the late additive
        // migration. This is NOT recovery of a marked/partial retirement store.
        let partial: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name LIKE 'retirement_%' OR name LIKE 'context_retirement_%')", [], |r| r.get(0)).map_err(|_| recovery_required())?;
        let marked = super::super::recovery::has_table(db, "registry_migrations")
            .map_err(|_| recovery_required())?
            && db
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
                    [MARKER],
                    |r| r.get::<_, bool>(0),
                )
                .map_err(|_| recovery_required())?;
        let envelope: bool = db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM operation_records WHERE action='retire_context')",
                [],
                |r| r.get(0),
            )
            .map_err(|_| recovery_required())?;
        return if partial || marked || envelope {
            Err(recovery_required())
        } else {
            Ok(true)
        };
    }
    audit(db)?;
    if !owned_identity(db, operation)? {
        return Ok(true);
    }
    db.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_records c JOIN operation_records o ON o.id=c.local_record_id
      WHERE c.operation_ulid=?1 AND o.root_path=?2 AND o.state IN ('authoritative_observed','indexing','failed','observed','persisted','retry','index_invalidated')
        AND c.phase IN ('local_absent','index_pending') AND c.materialization_settled=1
        AND ((c.absence_basis='initial' AND o.completed_step='context_retirement_absent')
          OR (c.absence_basis='owned_intent' AND o.completed_step='context_branch_released')))",
        params![operation, root.to_str()], |r| r.get(0)).map_err(|_| recovery_required())
}

/// The envelope reserves identity even after companion loss. Generic abandon
/// must never erase a retirement request or reinterpret a terminal/partial row.
pub(in super::super) fn requires_guarded_abandon(
    db: &Connection,
    operation: super::super::OperationId,
) -> Result<bool, RepositoryError> {
    db.query_row("SELECT EXISTS(SELECT 1 FROM operation_records WHERE operation_ulid=?1 AND action='retire_context') OR EXISTS(SELECT 1 FROM context_retirement_records WHERE operation_ulid=?1)", [operation.to_string()], |r| r.get(0)).map_err(|_| recovery_required())
}

const RECORDS: &str = "CREATE TABLE context_retirement_records (
 operation_ulid TEXT PRIMARY KEY NOT NULL,
 local_record_id INTEGER NOT NULL UNIQUE REFERENCES operation_records(id) ON DELETE CASCADE,
 repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
 format_version INTEGER NOT NULL CHECK(format_version=1),
 kind TEXT NOT NULL CHECK(kind IN ('ticket','document')),
 item_id TEXT NOT NULL,
 initial_resources TEXT NOT NULL CHECK(initial_resources IN ('present','branch_only','absent','invalid')),
 expected_tip TEXT,
 selection_epoch INTEGER NOT NULL CHECK(typeof(selection_epoch)='integer' AND selection_epoch>0),
 instance_nonce TEXT,
 instance_verified INTEGER NOT NULL CHECK(instance_verified IN (0,1)),
 remote_name TEXT,
 primary_branch TEXT,
 configuration_generation INTEGER CHECK(configuration_generation IS NULL OR (typeof(configuration_generation)='integer' AND configuration_generation>=0)),
 endpoint_digest BLOB CHECK(endpoint_digest IS NULL OR (typeof(endpoint_digest)='blob' AND length(endpoint_digest)=32)),
 initial_primary_oid TEXT,
 request_ulid TEXT REFERENCES request_records(request_ulid) ON DELETE NO ACTION,
 batch_operation_ulid TEXT,
 batch_ordinal INTEGER CHECK(batch_ordinal IS NULL OR (typeof(batch_ordinal)='integer' AND batch_ordinal>=0)),
 materialization_capture TEXT NOT NULL CHECK(materialization_capture IN ('unverified','absent','complete')),
 materialization_operation_ulid TEXT,
 materialization_oid TEXT,
 materialization_step TEXT,
 materialization_settled INTEGER NOT NULL CHECK(materialization_settled IN (0,1)),
 owner_epoch INTEGER NOT NULL CHECK(typeof(owner_epoch)='integer' AND owner_epoch>=selection_epoch),
 phase TEXT NOT NULL CHECK(phase IN ('selected','worktree_removing','worktree_removed','branch_deleting','local_absent','index_pending','complete','refused','preserving','abandoned')),
 worktree_removal TEXT NOT NULL CHECK(worktree_removal IN ('none','possible','observed')),
 branch_removal TEXT NOT NULL CHECK(branch_removal IN ('none','possible','observed')),
 absence_basis TEXT NOT NULL CHECK(absence_basis IN ('none','initial','owned_intent')),
 final_code TEXT,
 final_result TEXT CHECK(final_result IS NULL OR json_valid(final_result)),
 UNIQUE(repository_id,operation_ulid),
 CHECK((initial_resources IN ('present','branch_only'))=(expected_tip IS NOT NULL)),
 CHECK((initial_resources='present')=(instance_nonce IS NOT NULL)),
 CHECK(instance_verified=0 OR initial_resources='present'),
 CHECK((remote_name IS NULL)=(primary_branch IS NULL) AND (remote_name IS NULL)=(configuration_generation IS NULL) AND (remote_name IS NULL)=(endpoint_digest IS NULL)),
 CHECK((batch_operation_ulid IS NULL)=(batch_ordinal IS NULL)),
 CHECK(batch_operation_ulid IS NULL OR request_ulid IS NOT NULL),
 CHECK((materialization_capture='complete')=(materialization_operation_ulid IS NOT NULL) AND (materialization_capture='complete')=(materialization_oid IS NOT NULL) AND (materialization_capture='complete')=(materialization_step IS NOT NULL)),
 CHECK(materialization_step IS NULL OR materialization_step='complete'),
 CHECK(materialization_capture!='unverified' OR (phase='refused' AND worktree_removal='none' AND branch_removal='none')),
 CHECK((phase IN ('complete','refused','abandoned'))=(final_code IS NOT NULL) AND (final_code IS NULL)=(final_result IS NULL)),
 CHECK(absence_basis!='initial' OR (initial_resources='absent' AND worktree_removal='none' AND branch_removal='none')),
 CHECK(materialization_settled=0 OR absence_basis!='none')
)";

const ATTEMPTS: &str = "CREATE TABLE context_retirement_attempts (
 operation_ulid TEXT NOT NULL,
 owner_epoch INTEGER NOT NULL CHECK(typeof(owner_epoch)='integer' AND owner_epoch>0),
 repository_id INTEGER NOT NULL,
 request_attempt INTEGER CHECK(request_attempt IS NULL OR (typeof(request_attempt)='integer' AND request_attempt>0)),
 batch_attempt_epoch INTEGER CHECK(batch_attempt_epoch IS NULL OR (typeof(batch_attempt_epoch)='integer' AND batch_attempt_epoch>0)),
 observer_operation_ulid TEXT,
 observer_owner_epoch INTEGER CHECK(observer_owner_epoch IS NULL OR (typeof(observer_owner_epoch)='integer' AND observer_owner_epoch>=0)),
 phase TEXT NOT NULL CHECK(phase IN ('active','ended','fenced')),
 PRIMARY KEY(operation_ulid,owner_epoch),
 UNIQUE(observer_operation_ulid),
 FOREIGN KEY(repository_id,operation_ulid) REFERENCES context_retirement_records(repository_id,operation_ulid) ON DELETE CASCADE,
 FOREIGN KEY(repository_id,observer_operation_ulid) REFERENCES remote_operation_records(repository_id,operation_ulid) ON DELETE NO ACTION DEFERRABLE INITIALLY DEFERRED,
 CHECK((observer_operation_ulid IS NULL)=(observer_owner_epoch IS NULL))
)";

const PROVENANCE: &str = "CREATE TABLE retirement_advertisement_provenance (
 id INTEGER PRIMARY KEY,
 repository_id INTEGER NOT NULL,
 source_operation_ulid TEXT NOT NULL,
 source_owner_epoch INTEGER NOT NULL CHECK(typeof(source_owner_epoch)='integer' AND source_owner_epoch>=0),
 observation_batch_id INTEGER NOT NULL CHECK(typeof(observation_batch_id)='integer' AND observation_batch_id>0),
 observed_at INTEGER NOT NULL CHECK(typeof(observed_at)='integer' AND observed_at>=0),
 configuration_generation INTEGER NOT NULL CHECK(typeof(configuration_generation)='integer' AND configuration_generation>=0),
 remote_name TEXT NOT NULL,
 primary_branch TEXT NOT NULL,
 endpoint_digest BLOB NOT NULL CHECK(typeof(endpoint_digest)='blob' AND length(endpoint_digest)=32),
 execution_kind TEXT NOT NULL CHECK(execution_kind IN ('local_parent','batch_gap','poll_parent')),
 execution_parent_ulid TEXT,
 execution_parent_epoch INTEGER CHECK(execution_parent_epoch IS NULL OR (typeof(execution_parent_epoch)='integer' AND execution_parent_epoch>=0)),
 execution_request_ulid TEXT REFERENCES request_records(request_ulid) ON DELETE NO ACTION,
 execution_request_attempt INTEGER CHECK(execution_request_attempt IS NULL OR (typeof(execution_request_attempt)='integer' AND execution_request_attempt>0)),
 execution_batch_operation_ulid TEXT,
 execution_batch_attempt_epoch INTEGER CHECK(execution_batch_attempt_epoch IS NULL OR (typeof(execution_batch_attempt_epoch)='integer' AND execution_batch_attempt_epoch>0)),
 UNIQUE(repository_id,source_operation_ulid,source_owner_epoch),
 FOREIGN KEY(repository_id,source_operation_ulid) REFERENCES remote_operation_records(repository_id,operation_ulid) ON DELETE CASCADE,
 CHECK((execution_parent_ulid IS NULL)=(execution_parent_epoch IS NULL)),
 CHECK((execution_kind='batch_gap')=(execution_parent_ulid IS NULL)),
 CHECK((execution_request_ulid IS NULL)=(execution_request_attempt IS NULL)),
 CHECK((execution_batch_operation_ulid IS NULL)=(execution_batch_attempt_epoch IS NULL)),
 CHECK(execution_kind='local_parent' OR (execution_request_ulid IS NOT NULL AND execution_batch_operation_ulid IS NOT NULL))
)";

const REENTRIES: &str = "CREATE TABLE context_retirement_reentries (
 operation_ulid TEXT NOT NULL,
 owner_epoch INTEGER NOT NULL CHECK(typeof(owner_epoch)='integer' AND owner_epoch>1),
 previous_epoch INTEGER NOT NULL CHECK(typeof(previous_epoch)='integer' AND previous_epoch>0 AND owner_epoch=previous_epoch+1),
 purpose TEXT NOT NULL CHECK(purpose IN ('observe','resume_worktree','metadata','preserve','no_effect')),
 PRIMARY KEY(operation_ulid,owner_epoch),
 FOREIGN KEY(operation_ulid,owner_epoch) REFERENCES context_retirement_attempts(operation_ulid,owner_epoch) ON DELETE CASCADE,
 FOREIGN KEY(operation_ulid,previous_epoch) REFERENCES context_retirement_attempts(operation_ulid,owner_epoch) ON DELETE NO ACTION
)";

const RECEIPTS: &str = "CREATE TABLE retirement_observation_receipts (
 id INTEGER PRIMARY KEY,
 provenance_id INTEGER NOT NULL UNIQUE REFERENCES retirement_advertisement_provenance(id) ON DELETE CASCADE,
 repository_id INTEGER NOT NULL,
 consumer_kind TEXT NOT NULL CHECK(consumer_kind IN ('single','batch')),
 consumer_operation_ulid TEXT NOT NULL,
 consumer_request_ulid TEXT REFERENCES request_records(request_ulid) ON DELETE NO ACTION,
 consumer_request_attempt INTEGER CHECK(consumer_request_attempt IS NULL OR (typeof(consumer_request_attempt)='integer' AND consumer_request_attempt>0)),
 consumer_attempt_epoch INTEGER NOT NULL CHECK(typeof(consumer_attempt_epoch)='integer' AND consumer_attempt_epoch>0),
 configuration_generation INTEGER NOT NULL CHECK(typeof(configuration_generation)='integer' AND configuration_generation>=0),
 frozen_worklist_digest BLOB CHECK(frozen_worklist_digest IS NULL OR (typeof(frozen_worklist_digest)='blob' AND length(frozen_worklist_digest)=32)),
 CHECK((consumer_request_ulid IS NULL)=(consumer_request_attempt IS NULL)),
 CHECK((consumer_kind='batch')=(frozen_worklist_digest IS NOT NULL)),
 CHECK(consumer_kind!='batch' OR consumer_request_ulid IS NOT NULL)
)";

/// M is an owned all-resource-absence receipt, not a branch-deletion receipt.
/// The exact earlier intent remains immutable, including any uncertainty.
const ABSENCE_RECEIPTS: &str = "CREATE TABLE context_retirement_absence_receipts (
 operation_ulid TEXT NOT NULL,
 owner_epoch INTEGER NOT NULL,
 intent_owner_epoch INTEGER NOT NULL,
 intent_resource TEXT NOT NULL CHECK(intent_resource IN ('worktree','branch')),
 intent_receipt_id INTEGER NOT NULL REFERENCES retirement_observation_receipts(id) ON DELETE NO ACTION,
 inventory TEXT NOT NULL CHECK(inventory='all_absent'),
 worktree_removal TEXT NOT NULL CHECK(worktree_removal IN ('none','possible','observed')),
 branch_removal TEXT NOT NULL CHECK(branch_removal IN ('none','possible','observed')),
 PRIMARY KEY(operation_ulid,owner_epoch),
 UNIQUE(operation_ulid),
 FOREIGN KEY(operation_ulid,owner_epoch) REFERENCES context_retirement_reentries(operation_ulid,owner_epoch) ON DELETE CASCADE,
 FOREIGN KEY(operation_ulid,intent_owner_epoch,intent_resource) REFERENCES context_retirement_effect_intents(operation_ulid,owner_epoch,resource) ON DELETE NO ACTION,
 CHECK(owner_epoch>intent_owner_epoch),
 CHECK(worktree_removal!='none' OR branch_removal!='none')
)";

const WORKTREE_RESUMPTIONS: &str = "CREATE TABLE context_retirement_worktree_resumptions (
 operation_ulid TEXT NOT NULL,
 owner_epoch INTEGER NOT NULL,
 intent_owner_epoch INTEGER NOT NULL,
 intent_resource TEXT NOT NULL CHECK(intent_resource='worktree'),
 intent_receipt_id INTEGER NOT NULL REFERENCES retirement_observation_receipts(id) ON DELETE NO ACTION,
 PRIMARY KEY(operation_ulid,owner_epoch),
 FOREIGN KEY(operation_ulid,owner_epoch) REFERENCES context_retirement_reentries(operation_ulid,owner_epoch) ON DELETE CASCADE,
 FOREIGN KEY(operation_ulid,intent_owner_epoch,intent_resource) REFERENCES context_retirement_effect_intents(operation_ulid,owner_epoch,resource) ON DELETE NO ACTION,
 CHECK(owner_epoch>intent_owner_epoch)
)";

const INTENTS: &str = "CREATE TABLE context_retirement_effect_intents (
 operation_ulid TEXT NOT NULL,
 owner_epoch INTEGER NOT NULL,
 resource TEXT NOT NULL CHECK(resource IN ('worktree','branch')),
 receipt_id INTEGER NOT NULL REFERENCES retirement_observation_receipts(id) ON DELETE NO ACTION DEFERRABLE INITIALLY DEFERRED,
 primary_oid TEXT NOT NULL,
 proof_kind TEXT NOT NULL CHECK(proof_kind IN ('ancestor','folded')),
 observation TEXT CHECK(observation IS NULL OR observation IN ('removed','already_absent','not_clean','changed','incomplete')),
 PRIMARY KEY(operation_ulid,owner_epoch,resource),
 FOREIGN KEY(operation_ulid,owner_epoch) REFERENCES context_retirement_attempts(operation_ulid,owner_epoch) ON DELETE CASCADE
)";

const PINS: &str = "CREATE TABLE retirement_observation_pins (
 provenance_id INTEGER NOT NULL REFERENCES retirement_advertisement_provenance(id) ON DELETE NO ACTION DEFERRABLE INITIALLY DEFERRED,
 pin_kind TEXT NOT NULL CHECK(pin_kind IN ('source_attempt','consumer_attempt')),
 holder_operation_ulid TEXT NOT NULL,
 holder_epoch INTEGER NOT NULL CHECK(typeof(holder_epoch)='integer' AND holder_epoch>=0),
 repository_id INTEGER NOT NULL,
 observation_batch_id INTEGER NOT NULL,
 binding_id INTEGER UNIQUE REFERENCES retirement_observation_receipts(id) ON DELETE NO ACTION DEFERRABLE INITIALLY DEFERRED,
 PRIMARY KEY(provenance_id,pin_kind,holder_operation_ulid,holder_epoch),
 FOREIGN KEY(repository_id,observation_batch_id) REFERENCES remote_observation_batches(repository_id,id) ON DELETE NO ACTION DEFERRABLE INITIALLY DEFERRED,
 CHECK((pin_kind='consumer_attempt')=(binding_id IS NOT NULL))
)";

const RELEASES: &str = "CREATE TABLE \"retirement_observation_pin_releases\" (
 provenance_id INTEGER NOT NULL REFERENCES retirement_advertisement_provenance(id) ON DELETE CASCADE,
 pin_kind TEXT NOT NULL CHECK(pin_kind IN ('source_attempt','consumer_attempt')),
 holder_operation_ulid TEXT NOT NULL,
 holder_epoch INTEGER NOT NULL CHECK(typeof(holder_epoch)='integer' AND holder_epoch>=0),
 repository_id INTEGER NOT NULL,
 observation_batch_id INTEGER NOT NULL CHECK(typeof(observation_batch_id)='integer' AND observation_batch_id>0),
 binding_id INTEGER REFERENCES retirement_observation_receipts(id) ON DELETE NO ACTION DEFERRABLE INITIALLY DEFERRED,
 reason TEXT NOT NULL CHECK(reason IN ('source_terminal','source_fenced','handoff_decided','consumer_terminal','consumer_fenced')),
 PRIMARY KEY(provenance_id,pin_kind,holder_operation_ulid,holder_epoch),
 CHECK((pin_kind='consumer_attempt')=(binding_id IS NOT NULL))
)";

/// SQL fragments are entirely static. Keeping each object separate lets startup
/// compare every table/index/trigger without a permissive CREATE IF NOT EXISTS.
fn objects() -> Vec<(&'static str, &'static str, String)> {
    let mut objects = vec![
        ("table", "context_retirement_records", RECORDS.to_owned()),
        ("table", "context_retirement_attempts", ATTEMPTS.to_owned()),
        ("table", "context_retirement_reentries", REENTRIES.to_owned()),
        ("table", "context_retirement_absence_receipts", ABSENCE_RECEIPTS.to_owned()),
        ("table", "context_retirement_worktree_resumptions", WORKTREE_RESUMPTIONS.to_owned()),
        ("table", "retirement_advertisement_provenance", PROVENANCE.to_owned()),
        ("table", "retirement_observation_receipts", RECEIPTS.to_owned()),
        ("table", "context_retirement_effect_intents", INTENTS.to_owned()),
        ("table", "retirement_observation_pins", PINS.to_owned()),
        ("table", "retirement_observation_pin_releases", RELEASES.to_owned()),
        ("index", "retirement_active_executor", "CREATE UNIQUE INDEX retirement_active_executor ON context_retirement_attempts(operation_ulid) WHERE phase='active'".to_owned()),
        ("index", "retirement_pinned_batch", "CREATE INDEX retirement_pinned_batch ON retirement_observation_pins(repository_id,observation_batch_id)".to_owned()),
    ];
    for (name, table, columns, condition) in [
        (
            "retirement_record_identity",
            "context_retirement_records",
            "operation_ulid,local_record_id,repository_id,format_version,kind,item_id,initial_resources,expected_tip,selection_epoch,instance_nonce,remote_name,primary_branch,configuration_generation,endpoint_digest,request_ulid,batch_operation_ulid,batch_ordinal,materialization_capture,materialization_operation_ulid,materialization_oid,materialization_step",
            "",
        ),
        (
            "retirement_initial_primary",
            "context_retirement_records",
            "initial_primary_oid",
            " WHEN OLD.initial_primary_oid IS NOT NULL",
        ),
        (
            "retirement_terminal_result",
            "context_retirement_records",
            "",
            " WHEN OLD.phase IN ('complete','refused','abandoned')",
        ),
        (
            "retirement_attempt_identity",
            "context_retirement_attempts",
            "operation_ulid,owner_epoch,repository_id,request_attempt,batch_attempt_epoch",
            "",
        ),
        (
            "retirement_observer_binding",
            "context_retirement_attempts",
            "observer_operation_ulid,observer_owner_epoch",
            " WHEN OLD.observer_operation_ulid IS NOT NULL",
        ),
        (
            "retirement_effect_identity",
            "context_retirement_effect_intents",
            "operation_ulid,owner_epoch,resource,receipt_id,primary_oid,proof_kind",
            "",
        ),
        (
            "retirement_reentry_immutable",
            "context_retirement_reentries",
            "",
            "",
        ),
        (
            "retirement_absence_receipt_immutable",
            "context_retirement_absence_receipts",
            "",
            "",
        ),
        (
            "retirement_worktree_resumption_immutable",
            "context_retirement_worktree_resumptions",
            "",
            "",
        ),
        (
            "retirement_effect_observation",
            "context_retirement_effect_intents",
            "observation",
            " WHEN OLD.observation IS NOT NULL",
        ),
        (
            "retirement_provenance_immutable",
            "retirement_advertisement_provenance",
            "",
            "",
        ),
        (
            "retirement_binding_immutable",
            "retirement_observation_receipts",
            "",
            "",
        ),
        (
            "retirement_pin_immutable",
            "retirement_observation_pins",
            "",
            "",
        ),
        (
            "retirement_release_immutable",
            "retirement_observation_pin_releases",
            "",
            "",
        ),
    ] {
        let of = if columns.is_empty() {
            String::new()
        } else {
            format!(" OF {columns}")
        };
        objects.push(("trigger", name, format!("CREATE TRIGGER {name} BEFORE UPDATE{of} ON {table}{condition} BEGIN SELECT RAISE(ABORT,'immutable retirement evidence'); END")));
    }
    objects.extend([
        ("trigger", "retirement_record_delete", "CREATE TRIGGER retirement_record_delete BEFORE DELETE ON context_retirement_records WHEN OLD.phase NOT IN ('complete','refused','abandoned') BEGIN SELECT RAISE(ABORT,'pending retirement debt'); END".to_owned()),
        ("trigger", "retirement_envelope_delete", "CREATE TRIGGER retirement_envelope_delete BEFORE DELETE ON operation_records WHEN OLD.action='retire_context' AND (OLD.state!='completed' OR NOT EXISTS(SELECT 1 FROM context_retirement_records c WHERE c.local_record_id=OLD.id AND c.phase IN ('complete','refused','abandoned'))) BEGIN SELECT RAISE(ABORT,'pending retirement envelope'); END".to_owned()),
        ("trigger", "retirement_provenance_delete", "CREATE TRIGGER retirement_provenance_delete BEFORE DELETE ON retirement_advertisement_provenance WHEN EXISTS(SELECT 1 FROM retirement_observation_pins p WHERE p.provenance_id=OLD.id) BEGIN SELECT RAISE(ABORT,'pinned retirement provenance'); END".to_owned()),
        ("trigger", "retirement_binding_delete", "CREATE TRIGGER retirement_binding_delete BEFORE DELETE ON retirement_observation_receipts WHEN EXISTS(SELECT 1 FROM retirement_observation_pins p WHERE p.binding_id=OLD.id) BEGIN SELECT RAISE(ABORT,'pinned retirement binding'); END".to_owned()),
        ("trigger", "retirement_no_repin", "CREATE TRIGGER retirement_no_repin BEFORE INSERT ON retirement_observation_pins WHEN EXISTS(SELECT 1 FROM retirement_observation_pin_releases r WHERE r.provenance_id=NEW.provenance_id AND r.pin_kind=NEW.pin_kind AND r.holder_operation_ulid=NEW.holder_operation_ulid AND r.holder_epoch=NEW.holder_epoch) BEGIN SELECT RAISE(ABORT,'released retirement pin'); END".to_owned()),
        ("trigger", "retirement_pin_release_required", "CREATE TRIGGER retirement_pin_release_required BEFORE DELETE ON retirement_observation_pins WHEN NOT EXISTS(SELECT 1 FROM retirement_observation_pin_releases r WHERE r.provenance_id=OLD.provenance_id AND r.pin_kind=OLD.pin_kind AND r.holder_operation_ulid=OLD.holder_operation_ulid AND r.holder_epoch=OLD.holder_epoch AND r.repository_id=OLD.repository_id AND r.observation_batch_id=OLD.observation_batch_id AND r.binding_id IS OLD.binding_id) BEGIN SELECT RAISE(ABORT,'retirement pin release receipt required'); END".to_owned()),
        ("trigger", "retirement_materialization_forward", "CREATE TRIGGER retirement_materialization_forward BEFORE UPDATE OF materialization_settled ON context_retirement_records WHEN NEW.materialization_settled<OLD.materialization_settled BEGIN SELECT RAISE(ABORT,'settled retirement materialization'); END".to_owned()),
        ("trigger", "retirement_instance_verified_forward", "CREATE TRIGGER retirement_instance_verified_forward BEFORE UPDATE OF instance_verified ON context_retirement_records WHEN NEW.instance_verified<OLD.instance_verified BEGIN SELECT RAISE(ABORT,'verified retirement instance history'); END".to_owned()),
    ]);
    for (kind, _, sql) in &mut objects {
        if *kind == "table" {
            *sql = scalar_constraints(sql);
        }
    }
    objects
}

pub(super) fn published_v1_objects()
-> Result<Vec<(&'static str, &'static str, String)>, RepositoryError> {
    let mut objects = objects();
    objects.retain(|(_, name, _)| {
        !matches!(
            *name,
            "context_retirement_reentries"
                | "context_retirement_absence_receipts"
                | "context_retirement_worktree_resumptions"
                | "retirement_reentry_immutable"
                | "retirement_absence_receipt_immutable"
                | "retirement_worktree_resumption_immutable"
        )
    });
    for (_, name, sql) in &mut objects {
        if *name == "retirement_observation_pin_releases" {
            *sql = sql.replace(",'consumer_fenced'", "").replace(
                "CREATE TABLE \"retirement_observation_pin_releases\"",
                "CREATE TABLE retirement_observation_pin_releases",
            );
        }
    }
    let mut bytes = Vec::new();
    for (kind, name, sql) in &objects {
        for field in [*kind, *name, &normalized(sql)] {
            bytes.extend_from_slice(field.as_bytes());
            bytes.push(0);
        }
    }
    // Independently extracted from 7b153deb9ed829c948092f6a855acbf2819612a0:
    // kind NUL name NUL normalized-SQL NUL, in published object order. This
    // prevents a later edit of the current envelope silently redefining v1.
    if git2::Oid::hash_object(git2::ObjectType::Blob, &bytes)
        .map_err(|_| recovery_required())?
        .to_string()
        != "b8ac7ae7201e6182a91d1a8546e63ad518dc9701"
    {
        return Err(recovery_required());
    }
    Ok(objects)
}

/// Expand the same canonical scalar constraints at each statically declared
/// column. The leading space anchors the complete column name: operation_ulid
/// must not accidentally rewrite consumer_operation_ulid's CHECK expression.
fn scalar_constraints(sql: &str) -> String {
    let mut sql = sql.to_owned();
    for column in [
        "operation_ulid",
        "item_id",
        "instance_nonce",
        "request_ulid",
        "batch_operation_ulid",
        "materialization_operation_ulid",
        "observer_operation_ulid",
        "source_operation_ulid",
        "execution_parent_ulid",
        "execution_request_ulid",
        "execution_batch_operation_ulid",
        "consumer_operation_ulid",
        "consumer_request_ulid",
        "holder_operation_ulid",
    ] {
        let declaration = format!(" {column} TEXT");
        sql = sql.replace(&declaration, &format!("{declaration} CHECK({column} IS NULL OR (typeof({column})='text' AND length({column})=26 AND substr({column},1,1) BETWEEN '0' AND '7' AND {column} NOT GLOB '*[^0123456789ABCDEFGHJKMNPQRSTVWXYZ]*'))"));
    }
    for column in [
        "expected_tip",
        "initial_primary_oid",
        "materialization_oid",
        "primary_oid",
    ] {
        let declaration = format!(" {column} TEXT");
        sql = sql.replace(&declaration, &format!("{declaration} CHECK({column} IS NULL OR (typeof({column})='text' AND length({column})=40 AND {column} NOT GLOB '*[^0-9a-f]*' AND {column}!='0000000000000000000000000000000000000000'))"));
    }
    sql
}

// SQL's quoted literals are case/whitespace-sensitive evidence. Do not use the
// old general schema normalizer that folds literals as well as SQL keywords.
fn normalized(sql: &str) -> String {
    let mut quoted = false;
    sql.trim_end_matches(';')
        .chars()
        .filter(|c| {
            if *c == '\'' {
                quoted = !quoted;
            }
            quoted || !c.is_whitespace()
        })
        .collect()
}

pub(in super::super) fn migrate(db: &mut Connection) -> Result<(), RepositoryError> {
    require_known_markers(db)?;
    let marked: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [MARKER],
            |r| r.get(0),
        )
        .map_err(|_| recovery_required())?;
    let current: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [REENTRY_MARKER],
            |r| r.get(0),
        )
        .map_err(|_| recovery_required())?;
    if current && !marked {
        return Err(recovery_required());
    }
    if marked && current {
        return validate_schema(db);
    }
    let tx = db
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| recovery_required())?;
    let marked: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [MARKER],
            |r| r.get(0),
        )
        .map_err(|_| recovery_required())?;
    let definitions = objects();
    require_known_markers(&tx)?;
    let current: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [REENTRY_MARKER],
            |r| r.get(0),
        )
        .map_err(|_| recovery_required())?;
    if current && !marked {
        return Err(recovery_required());
    }
    if marked && !current {
        let prior = published_v1_objects()?;
        validate_schema_objects(&tx, &prior)?;
        for (kind, name, sql) in &definitions {
            if !prior.iter().any(|(_, old, _)| old == name) {
                let present: bool = tx
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name=?1)",
                        [name],
                        |r| r.get(0),
                    )
                    .map_err(|_| recovery_required())?;
                if present {
                    return Err(recovery_required());
                }
                if !matches!(*kind, "table" | "trigger") {
                    return Err(recovery_required());
                }
                tx.execute_batch(sql).map_err(|_| recovery_required())?;
            }
        }
        #[cfg(test)]
        migration_checkpoint(1)?;
        // Empty additive companions cannot excuse any legacy row. Audit all
        // old CHECKs, FKs and history BEFORE widening the old release CHECK.
        let mut transition = definitions.clone();
        let prior_release = &prior
            .iter()
            .find(|(_, name, _)| *name == "retirement_observation_pin_releases")
            .ok_or_else(recovery_required)?
            .2;
        transition
            .iter_mut()
            .find(|(_, name, _)| *name == "retirement_observation_pin_releases")
            .ok_or_else(recovery_required)?
            .2 = prior_release.clone();
        audit_objects(&tx, &transition)?;
        #[cfg(test)]
        migration_checkpoint(2)?;
        replace_release_table(&tx, &definitions)?;
    }
    if !marked {
        for (_, name, _) in &definitions {
            if tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name=?1)",
                    [name],
                    |r| r.get::<_, bool>(0),
                )
                .map_err(|_| recovery_required())?
            {
                return Err(recovery_required());
            }
        }
        for (_, _, sql) in &definitions {
            tx.execute_batch(sql).map_err(|_| recovery_required())?;
        }
        tx.execute("INSERT INTO registry_migrations(name) VALUES(?1)", [MARKER])
            .map_err(|_| recovery_required())?;
    }
    if !current {
        tx.execute(
            "INSERT INTO registry_migrations(name) VALUES(?1)",
            [REENTRY_MARKER],
        )
        .map_err(|_| recovery_required())?;
    }
    #[cfg(test)]
    migration_checkpoint(7)?;
    audit(&tx)?;
    tx.commit().map_err(|_| recovery_required())
}

fn replace_release_table(
    tx: &rusqlite::Transaction<'_>,
    definitions: &[(&str, &str, String)],
) -> Result<(), RepositoryError> {
    let sql = &definitions
        .iter()
        .find(|(_, name, _)| *name == "retirement_observation_pin_releases")
        .ok_or_else(recovery_required)?
        .2;
    tx.execute_batch(&sql.replace(
        "CREATE TABLE \"retirement_observation_pin_releases\"",
        "CREATE TABLE retirement_observation_pin_releases_upgrade",
    ))
    .map_err(|_| recovery_required())?;
    tx.execute_batch("INSERT INTO retirement_observation_pin_releases_upgrade(rowid,provenance_id,pin_kind,holder_operation_ulid,holder_epoch,repository_id,observation_batch_id,binding_id,reason) SELECT rowid,provenance_id,pin_kind,holder_operation_ulid,holder_epoch,repository_id,observation_batch_id,binding_id,reason FROM retirement_observation_pin_releases;").map_err(|_|recovery_required())?;
    #[cfg(test)]
    migration_checkpoint(3)?;
    // validate_schema_objects checked the affected attachment/FK closure before
    // any upgrade DDL. Build/copy the replacement first; never rename the old
    // parent and accidentally retarget immutable FK evidence to a shadow table.
    for name in [
        "retirement_release_immutable",
        "retirement_no_repin",
        "retirement_pin_release_required",
    ] {
        tx.execute_batch(&format!("DROP TRIGGER {name}"))
            .map_err(|_| recovery_required())?;
    }
    #[cfg(test)]
    migration_checkpoint(4)?;
    tx.execute_batch("DROP TABLE retirement_observation_pin_releases")
        .map_err(|_| recovery_required())?;
    #[cfg(test)]
    migration_checkpoint(5)?;
    tx.execute_batch("ALTER TABLE retirement_observation_pin_releases_upgrade RENAME TO retirement_observation_pin_releases").map_err(|_|recovery_required())?;
    for name in [
        "retirement_release_immutable",
        "retirement_no_repin",
        "retirement_pin_release_required",
    ] {
        let sql = &definitions
            .iter()
            .find(|(_, candidate, _)| *candidate == name)
            .ok_or_else(recovery_required)?
            .2;
        tx.execute_batch(sql).map_err(|_| recovery_required())?;
    }
    #[cfg(test)]
    migration_checkpoint(6)?;
    Ok(())
}

/// Full retained history is audited in a read transaction at service startup,
/// not under every ordinary mutation's immediate write transaction.
pub(in super::super) fn audit_registry(db: &mut Connection) -> Result<(), RepositoryError> {
    let tx = db
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .map_err(|_| recovery_required())?;
    audit(&tx)?;
    tx.rollback().map_err(|_| recovery_required())
}

fn validate_schema(db: &Connection) -> Result<(), RepositoryError> {
    validate_schema_objects(db, &objects())
}

fn validate_schema_objects(
    db: &Connection,
    definitions: &[(&str, &str, String)],
) -> Result<(), RepositoryError> {
    for (kind, name, sql) in definitions {
        let actual: Option<String> = db
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type=?1 AND name=?2",
                params![kind, name],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| recovery_required())?;
        if actual.as_deref().map(normalized) != Some(normalized(sql)) {
            return Err(recovery_required());
        }
    }
    validate_release_closure(db, definitions)?;
    Ok(())
}

/// Required-object equality alone does not prove DROP TABLE is lossless:
/// SQLite drops attached objects and may cascade into unknown incoming FKs.
/// Only this affected closure is closed; unrelated repository schema is allowed.
fn validate_release_closure(
    db: &Connection,
    definitions: &[(&str, &str, String)],
) -> Result<(), RepositoryError> {
    const RELEASE: &str = "retirement_observation_pin_releases";
    let attached = db.prepare("SELECT type,name,sql FROM sqlite_master WHERE tbl_name=?1 COLLATE NOCASE AND type IN ('index','trigger')")
        .and_then(|mut q| q.query_map([RELEASE], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?)))?.collect::<Result<Vec<_>,_>>())
        .map_err(|_| recovery_required())?;
    for (kind, name, sql) in attached {
        // The pinned table DDL has exactly this single implicit primary index.
        let primary = kind == "index"
            && sql.is_none()
            && name.eq_ignore_ascii_case("sqlite_autoindex_retirement_observation_pin_releases_1");
        let declared = definitions.iter().any(|(expected_kind, expected_name, _)| {
            kind == *expected_kind && name.eq_ignore_ascii_case(expected_name)
        });
        if !primary && !declared {
            return Err(recovery_required());
        }
    }
    let tables = db
        .prepare("SELECT name FROM sqlite_master WHERE type='table'")
        .and_then(|mut q| {
            q.query_map([], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| recovery_required())?;
    for table in tables {
        let parents = db
            .prepare("SELECT \"table\" FROM pragma_foreign_key_list(?1)")
            .and_then(|mut q| {
                q.query_map([&table], |r| r.get::<_, String>(0))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|_| recovery_required())?;
        // Managed tables' exact DDL, including every FK, was checked above.
        // Any unknown child is outside that canonical dependency closure,
        // regardless of action, row count, or SQLite identifier spelling.
        let declared = definitions
            .iter()
            .any(|(kind, name, _)| *kind == "table" && table.eq_ignore_ascii_case(name));
        if !declared
            && parents
                .iter()
                .any(|parent| parent.eq_ignore_ascii_case(RELEASE))
        {
            return Err(recovery_required());
        }
    }
    Ok(())
}

fn audit(db: &Connection) -> Result<(), RepositoryError> {
    require_known_markers(db)?;
    let marked: bool = db
        .query_row(
            "SELECT count(*)=2 FROM registry_migrations WHERE name IN (?1,?2)",
            params![MARKER, REENTRY_MARKER],
            |r| r.get(0),
        )
        .map_err(|_| recovery_required())?;
    if !marked {
        return Err(recovery_required());
    }
    audit_objects(db, &objects())
}

fn require_known_markers(db: &Connection) -> Result<(), RepositoryError> {
    let unknown:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name LIKE 'context_retirement_%' AND name NOT IN (?1,?2))",params![MARKER,REENTRY_MARKER],|r|r.get(0)).map_err(|_|recovery_required())?;
    if unknown {
        return Err(recovery_required());
    }
    Ok(())
}

fn audit_objects(
    db: &Connection,
    definitions: &[(&str, &str, String)],
) -> Result<(), RepositoryError> {
    validate_schema_objects(db, definitions)?;
    for (kind, name, _) in definitions {
        if *kind == "table" {
            let mut q = db
                .prepare(&format!("PRAGMA integrity_check('{name}')"))
                .map_err(|_| recovery_required())?;
            let results = q
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(|_| recovery_required())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| recovery_required())?;
            if results != ["ok"]
                || db
                    .prepare(&format!("PRAGMA foreign_key_check('{name}')"))
                    .and_then(|mut q| q.exists([]))
                    .map_err(|_| recovery_required())?
            {
                return Err(recovery_required());
            }
        }
    }
    let orphan: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM operation_records o LEFT JOIN context_retirement_records c ON c.local_record_id=o.id WHERE o.action='retire_context' AND c.operation_ulid IS NULL)
     OR EXISTS(SELECT 1 FROM context_retirement_records c LEFT JOIN operation_records o ON o.id=c.local_record_id LEFT JOIN repositories r ON r.id=c.repository_id WHERE o.id IS NULL OR o.action!='retire_context' OR o.operation_ulid IS NOT c.operation_ulid OR o.repository_id IS NOT c.repository_id OR o.root_path IS NOT r.root_path OR o.target IS NOT ('authoring-context-v1/'||c.kind||'/'||c.item_id||'/Retire'))", [], |r| r.get(0)).map_err(|_| recovery_required())?;
    if orphan {
        return Err(recovery_required());
    }
    for (table, ids, oids) in [
        (
            "context_retirement_records",
            &[
                "operation_ulid",
                "item_id",
                "instance_nonce",
                "request_ulid",
                "batch_operation_ulid",
                "materialization_operation_ulid",
            ][..],
            &["expected_tip", "initial_primary_oid", "materialization_oid"][..],
        ),
        (
            "context_retirement_attempts",
            &["operation_ulid", "observer_operation_ulid"][..],
            &[][..],
        ),
        (
            "context_retirement_effect_intents",
            &["operation_ulid"][..],
            &["primary_oid"][..],
        ),
        (
            "retirement_advertisement_provenance",
            &[
                "source_operation_ulid",
                "execution_parent_ulid",
                "execution_request_ulid",
                "execution_batch_operation_ulid",
            ][..],
            &[][..],
        ),
        (
            "retirement_observation_receipts",
            &["consumer_operation_ulid", "consumer_request_ulid"][..],
            &[][..],
        ),
        (
            "retirement_observation_pins",
            &["holder_operation_ulid"][..],
            &[][..],
        ),
        (
            "retirement_observation_pin_releases",
            &["holder_operation_ulid"][..],
            &[][..],
        ),
    ] {
        for (columns, object_id) in [(ids, false), (oids, true)] {
            for column in columns {
                let mut query = db
                    .prepare(&format!("SELECT {column} FROM {table}"))
                    .map_err(|_| recovery_required())?;
                let values = query
                    .query_map([], |r| r.get::<_, Option<String>>(0))
                    .map_err(|_| recovery_required())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| recovery_required())?;
                for value in values.into_iter().flatten() {
                    let valid = if object_id {
                        git2::Oid::from_str(&value)
                            .is_ok_and(|oid| !oid.is_zero() && oid.to_string() == value)
                    } else {
                        super::super::OperationId::parse(&value)
                            .is_ok_and(|id| id.to_string() == value)
                    };
                    if !valid {
                        return Err(recovery_required());
                    }
                }
            }
        }
    }
    // Retirement owns a closed set of noncheckpoint local steps. A phase and
    // its effect/settlement evidence must justify the step in both directions;
    // a foreign lifecycle step is not harmless display metadata.
    let inconsistent: bool = db.query_row("SELECT EXISTS(
      SELECT 1 FROM context_retirement_records c JOIN operation_records o ON o.id=c.local_record_id
      WHERE NOT EXISTS(SELECT 1 FROM context_retirement_attempts a WHERE a.operation_ulid=c.operation_ulid AND a.owner_epoch=c.selection_epoch)
       OR NOT EXISTS(SELECT 1 FROM context_retirement_attempts a WHERE a.operation_ulid=c.operation_ulid AND a.repository_id=c.repository_id AND a.owner_epoch=c.owner_epoch AND ((c.phase IN ('complete','refused','abandoned') AND a.phase IN ('ended','fenced')) OR (c.phase NOT IN ('complete','refused','abandoned') AND a.phase='active')))
       OR (c.phase IN ('complete','refused','abandoned')) != (o.state='completed')
       OR (c.absence_basis='owned_intent' AND NOT EXISTS(SELECT 1 FROM context_retirement_effect_intents e WHERE e.operation_ulid=c.operation_ulid))
       OR (c.worktree_removal!='none' AND (c.initial_resources!='present' OR NOT EXISTS(SELECT 1 FROM context_retirement_effect_intents e WHERE e.operation_ulid=c.operation_ulid AND e.resource='worktree')))
       OR (c.branch_removal!='none' AND NOT EXISTS(SELECT 1 FROM context_retirement_effect_intents e WHERE e.operation_ulid=c.operation_ulid AND e.resource='branch'))
       OR (c.phase IN ('local_absent','index_pending','complete') AND (c.materialization_settled!=1 OR c.absence_basis='none'
           OR o.completed_step IS NOT CASE WHEN c.absence_basis='initial' THEN 'context_retirement_absent' ELSE 'context_branch_released' END))
       OR (c.phase IN ('local_absent','index_pending','complete') AND c.absence_basis='owned_intent' AND
           ((c.branch_removal='none' OR NOT EXISTS(SELECT 1 FROM context_retirement_effect_intents e WHERE e.operation_ulid=c.operation_ulid AND e.resource='branch')) AND NOT EXISTS(SELECT 1 FROM context_retirement_absence_receipts m WHERE m.operation_ulid=c.operation_ulid AND m.owner_epoch=c.owner_epoch)))
       OR (c.request_ulid IS NULL AND (EXISTS(SELECT 1 FROM request_operations ro WHERE ro.operation_ulid=c.operation_ulid)
         OR EXISTS(SELECT 1 FROM context_retirement_attempts a WHERE a.operation_ulid=c.operation_ulid AND a.request_attempt IS NOT NULL)))
       OR (c.request_ulid IS NOT NULL AND (NOT EXISTS(SELECT 1 FROM request_records r JOIN request_operations ro ON ro.request_ulid=r.request_ulid
         JOIN context_retirement_attempts a ON a.operation_ulid=c.operation_ulid AND a.owner_epoch=c.owner_epoch
         WHERE r.request_ulid=c.request_ulid AND r.scope_key=o.root_path AND r.command='context retire' AND r.target=c.item_id
           AND ro.family='local' AND ro.operation_ulid=c.operation_ulid AND a.request_attempt IS NOT NULL AND a.request_attempt<=r.attempt
           AND (c.phase IN ('complete','refused','abandoned') OR (r.state='accepted' AND a.request_attempt=r.attempt)))
         OR (SELECT count(*) FROM request_operations ro WHERE ro.operation_ulid=c.operation_ulid)!=1))
         OR (o.completed_step='context_retirement_absent' AND (c.absence_basis!='initial' OR c.materialization_settled!=1 OR c.phase NOT IN ('local_absent','index_pending','complete')))
       OR (o.completed_step='context_branch_released' AND (c.absence_basis!='owned_intent' OR c.materialization_settled!=1 OR c.phase NOT IN ('local_absent','index_pending','complete')))
        OR (o.completed_step='context_worktree_released' AND (c.worktree_removal='none' OR c.phase NOT IN ('worktree_removed','branch_deleting','preserving')))
        OR (o.completed_step='context_retirement_preserved' AND c.phase NOT IN ('preserving','index_pending','abandoned'))
        OR (o.completed_step IS NOT NULL AND o.completed_step NOT IN ('context_retirement_absent','context_worktree_released','context_branch_released'))
        OR (c.phase='selected' AND (o.completed_step IS NOT NULL OR o.state NOT IN ('created','failed')
          OR c.worktree_removal!='none' OR c.branch_removal!='none' OR c.absence_basis!='none' OR c.materialization_settled!=0))
        OR (c.phase='refused' AND o.completed_step IS NOT NULL)
        OR (c.phase='worktree_removing' AND (o.completed_step IS NOT NULL OR o.state NOT IN ('created','failed')
          OR c.worktree_removal!='possible' OR c.branch_removal!='none' OR c.absence_basis!='none' OR c.materialization_settled!=0))
        OR (c.phase='worktree_removed' AND (o.completed_step IS NOT 'context_worktree_released'
          OR c.branch_removal!='none' OR c.absence_basis!='none' OR c.materialization_settled!=0))
        OR (o.completed_step='context_worktree_released' AND (c.worktree_removal!='observed'
          OR o.state NOT IN ('created','context_worktree_released','failed')
          OR NOT EXISTS(SELECT 1 FROM context_retirement_effect_intents e WHERE e.operation_ulid=c.operation_ulid AND e.resource='worktree' AND e.observation='removed')))
        OR (c.phase='branch_deleting' AND (c.branch_removal!='possible' OR c.absence_basis!='none' OR c.materialization_settled!=0
          OR (c.initial_resources='present' AND o.completed_step IS NOT 'context_worktree_released')
          OR (c.initial_resources='branch_only' AND (o.completed_step IS NOT NULL OR c.worktree_removal!='none' OR o.state NOT IN ('created','failed')))))
        OR (c.phase IN ('local_absent','index_pending') AND o.state NOT IN ('authoritative_observed','indexing','failed','observed','persisted','retry','index_invalidated'))
    )", [], |r| r.get(0)).map_err(|_| recovery_required())?;
    if inconsistent {
        return Err(recovery_required());
    }
    let invalid_reentry:bool=db.query_row("SELECT EXISTS(
      SELECT 1 FROM context_retirement_reentries rr
      JOIN context_retirement_records c ON c.operation_ulid=rr.operation_ulid
      JOIN context_retirement_attempts a ON a.operation_ulid=rr.operation_ulid AND a.owner_epoch=rr.owner_epoch
      JOIN context_retirement_attempts old ON old.operation_ulid=rr.operation_ulid AND old.owner_epoch=rr.previous_epoch
      WHERE old.phase!='fenced' OR rr.owner_epoch>c.owner_epoch OR a.repository_id!=c.repository_id
        OR c.request_ulid IS NOT NULL OR c.batch_operation_ulid IS NOT NULL
        OR c.owner_epoch-c.selection_epoch!=(SELECT count(*) FROM context_retirement_reentries chain WHERE chain.operation_ulid=c.operation_ulid)
        OR (rr.purpose IN ('observe','resume_worktree'))!=(a.observer_operation_ulid IS NOT NULL)
        OR (old.observer_operation_ulid IS NOT NULL AND NOT EXISTS(
          SELECT 1 FROM remote_operation_records o WHERE o.repository_id=c.repository_id AND o.operation_ulid=old.observer_operation_ulid
            AND o.owner_epoch=old.observer_owner_epoch+1 AND o.phase IN ('interrupted','completed','cancelled','failed')
            AND o.action='poll' AND o.poll_kind='observe' AND o.priority='manual'
            AND o.configuration_generation=c.configuration_generation AND o.remote_name=c.remote_name AND o.primary_branch=c.primary_branch))
        OR (old.observer_operation_ulid IS NULL AND NOT EXISTS(SELECT 1 FROM context_retirement_reentries previous WHERE previous.operation_ulid=rr.operation_ulid AND previous.owner_epoch=rr.previous_epoch AND previous.purpose='preserve'))
        OR (rr.purpose='observe' AND (c.initial_resources!='present' OR EXISTS(SELECT 1 FROM context_retirement_effect_intents e WHERE e.operation_ulid=c.operation_ulid AND e.owner_epoch<rr.owner_epoch)))
        OR (rr.purpose NOT IN ('observe','resume_worktree') AND (EXISTS(SELECT 1 FROM context_retirement_effect_intents e WHERE e.operation_ulid=c.operation_ulid AND e.owner_epoch=rr.owner_epoch)
          OR EXISTS(SELECT 1 FROM retirement_observation_receipts b WHERE b.consumer_operation_ulid=c.operation_ulid AND b.consumer_attempt_epoch=rr.owner_epoch)))
        OR (rr.purpose IN ('metadata','preserve') AND NOT EXISTS(SELECT 1 FROM context_retirement_effect_intents e WHERE e.operation_ulid=c.operation_ulid AND e.owner_epoch<rr.owner_epoch AND (e.observation IS NULL OR e.observation IN ('removed','already_absent','incomplete'))))
        OR (rr.purpose='no_effect' AND EXISTS(SELECT 1 FROM context_retirement_effect_intents e WHERE e.operation_ulid=c.operation_ulid))
        OR (rr.purpose='resume_worktree' AND NOT EXISTS(SELECT 1 FROM context_retirement_worktree_resumptions w WHERE w.operation_ulid=rr.operation_ulid AND w.owner_epoch=rr.owner_epoch))
        OR (rr.owner_epoch=c.owner_epoch AND rr.purpose='no_effect' AND (c.phase!='refused' OR c.final_code!='external_change' OR c.worktree_removal!='none' OR c.branch_removal!='none'))
        OR (rr.owner_epoch=c.owner_epoch AND rr.purpose='preserve' AND (c.phase NOT IN ('worktree_removing','worktree_removed','branch_deleting') OR (c.worktree_removal='none' AND c.branch_removal='none')))
        OR (rr.owner_epoch=c.owner_epoch AND rr.purpose='metadata' AND (c.phase NOT IN ('index_pending','local_absent','complete') OR c.absence_basis!='owned_intent' OR c.materialization_settled!=1 OR NOT EXISTS(SELECT 1 FROM context_retirement_absence_receipts m WHERE m.operation_ulid=c.operation_ulid AND m.owner_epoch=c.owner_epoch))))",[],|r|r.get(0)).map_err(|_|recovery_required())?;
    if invalid_reentry {
        return Err(recovery_required());
    }
    let invalid_resumption:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_worktree_resumptions w
      JOIN context_retirement_reentries rr ON rr.operation_ulid=w.operation_ulid AND rr.owner_epoch=w.owner_epoch
      JOIN context_retirement_records c ON c.operation_ulid=w.operation_ulid
      JOIN context_retirement_effect_intents e ON e.operation_ulid=w.operation_ulid AND e.owner_epoch=w.intent_owner_epoch AND e.resource=w.intent_resource
      JOIN context_retirement_attempts old ON old.operation_ulid=e.operation_ulid AND old.owner_epoch=e.owner_epoch
      WHERE rr.purpose!='resume_worktree' OR e.receipt_id!=w.intent_receipt_id OR old.phase!='fenced'
        OR c.initial_resources!='present' OR c.instance_verified!=1 OR c.initial_primary_oid IS NULL
        OR (e.observation IS NOT NULL AND e.observation!='incomplete')
        OR EXISTS(SELECT 1 FROM context_retirement_effect_intents prior WHERE prior.operation_ulid=c.operation_ulid AND prior.owner_epoch<w.owner_epoch AND (prior.resource='branch' OR prior.observation='removed')))",[],|r|r.get(0)).map_err(|_|recovery_required())?;
    if invalid_resumption {
        return Err(recovery_required());
    }
    let invalid_absence:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_absence_receipts m
      JOIN context_retirement_records c ON c.operation_ulid=m.operation_ulid
      JOIN context_retirement_reentries rr ON rr.operation_ulid=m.operation_ulid AND rr.owner_epoch=m.owner_epoch
      JOIN context_retirement_effect_intents e ON e.operation_ulid=m.operation_ulid AND e.owner_epoch=m.intent_owner_epoch AND e.resource=m.intent_resource
      JOIN context_retirement_attempts old ON old.operation_ulid=e.operation_ulid AND old.owner_epoch=e.owner_epoch
      JOIN retirement_observation_receipts b ON b.id=m.intent_receipt_id
      WHERE c.owner_epoch!=m.owner_epoch OR rr.purpose!='metadata' OR old.phase!='fenced'
        OR e.receipt_id!=m.intent_receipt_id OR b.consumer_operation_ulid!=c.operation_ulid OR b.consumer_attempt_epoch!=e.owner_epoch OR b.repository_id!=c.repository_id
        OR (e.observation IS NOT NULL AND e.observation NOT IN ('removed','already_absent','incomplete'))
        OR c.absence_basis!='owned_intent' OR c.materialization_settled!=1 OR c.phase NOT IN ('index_pending','local_absent','complete')
        OR m.worktree_removal!=c.worktree_removal OR m.branch_removal!=c.branch_removal
        OR EXISTS(SELECT 1 FROM context_retirement_effect_intents current WHERE current.operation_ulid=c.operation_ulid AND current.owner_epoch=m.owner_epoch))",[],|r|r.get(0)).map_err(|_|recovery_required())?;
    if invalid_absence {
        return Err(recovery_required());
    }
    // No batch/source or guarded-preservation producer/correlation is installed
    // by this bounded increment. Preserve the approved future semantics, but do
    // not accept placeholder steps/phases without their controlling authority.
    let unsupported_authority: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_records WHERE batch_operation_ulid IS NOT NULL OR phase IN ('preserving','abandoned'))
      OR EXISTS(SELECT 1 FROM retirement_advertisement_provenance WHERE execution_kind!='local_parent' OR execution_batch_operation_ulid IS NOT NULL)
      OR EXISTS(SELECT 1 FROM retirement_observation_receipts WHERE consumer_kind!='single')", [], |r| r.get(0)).map_err(|_| recovery_required())?;
    if unsupported_authority {
        return Err(recovery_required());
    }
    let identity_collision: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_attempts a WHERE a.observer_operation_ulid=a.operation_ulid
      OR EXISTS(SELECT 1 FROM operation_records o WHERE o.operation_ulid=a.observer_operation_ulid)
      OR EXISTS(SELECT 1 FROM local_context_sync_records s WHERE s.operation_ulid=a.observer_operation_ulid)
      OR EXISTS(SELECT 1 FROM key_material_operations k WHERE k.operation_id=a.observer_operation_ulid)
      OR EXISTS(SELECT 1 FROM key_registry_changes k WHERE k.operation_id=a.observer_operation_ulid))", [], |r| r.get(0)).map_err(|_| recovery_required())?;
    if identity_collision {
        return Err(recovery_required());
    }
    // An intent alone authorizes no authoritative handoff or terminal removal
    // claim. Settled removal uses this closed mapping; incomplete/owned-absence
    // evidence remains Possible, never Observed. A live pre-Git refusal instead
    // retains the branch intent as changed with no claimed effect.
    // The direct resumed-W -> M exception below correlates the changed attempt's
    // own immutable resumption with M's exact older uncertain intent/receipt.
    // It preserves Possible W debt; changed+Possible or purpose alone is not proof.
    let invalid_effect: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_effect_intents e
      JOIN context_retirement_records c ON c.operation_ulid=e.operation_ulid
      JOIN retirement_observation_receipts b ON b.id=e.receipt_id
      WHERE b.consumer_kind!='single' OR b.consumer_operation_ulid!=e.operation_ulid OR b.consumer_attempt_epoch!=e.owner_epoch
        OR b.repository_id!=c.repository_id OR c.initial_resources NOT IN ('present','branch_only')
        OR (e.resource='worktree' AND (c.initial_resources!='present' OR c.instance_verified!=1 OR (c.worktree_removal='none' AND NOT coalesce(c.phase='refused' AND e.observation='changed',0))))
        OR (e.resource='branch' AND c.branch_removal='none' AND NOT coalesce((c.phase='refused' OR (c.phase='worktree_removed' AND c.worktree_removal='observed') OR EXISTS(SELECT 1 FROM context_retirement_absence_receipts m WHERE m.operation_ulid=c.operation_ulid AND m.owner_epoch=c.owner_epoch)) AND e.observation='changed',0))
        OR (c.phase IN ('local_absent','index_pending','complete') AND c.absence_basis='owned_intent' AND NOT coalesce(
          (e.observation='removed' AND CASE e.resource WHEN 'branch' THEN c.branch_removal ELSE c.worktree_removal END='observed')
           OR (e.observation IN ('already_absent','incomplete') AND CASE e.resource WHEN 'branch' THEN c.branch_removal ELSE c.worktree_removal END='possible')
            OR (EXISTS(SELECT 1 FROM context_retirement_absence_receipts m WHERE m.operation_ulid=c.operation_ulid AND m.owner_epoch=c.owner_epoch) AND ((e.observation IS NULL AND CASE e.resource WHEN 'branch' THEN c.branch_removal ELSE c.worktree_removal END='possible') OR (e.observation='changed' AND CASE e.resource WHEN 'branch' THEN c.branch_removal ELSE c.worktree_removal END='none')))
            OR (e.resource='worktree' AND e.observation='changed' AND c.worktree_removal='possible' AND c.branch_removal='none' AND c.materialization_settled=1 AND EXISTS(
              SELECT 1 FROM context_retirement_absence_receipts m
              JOIN context_retirement_reentries mr ON mr.operation_ulid=m.operation_ulid AND mr.owner_epoch=m.owner_epoch
              JOIN context_retirement_worktree_resumptions w ON w.operation_ulid=e.operation_ulid AND w.owner_epoch=e.owner_epoch
              JOIN context_retirement_effect_intents prior ON prior.operation_ulid=w.operation_ulid AND prior.owner_epoch=w.intent_owner_epoch AND prior.resource=w.intent_resource AND prior.receipt_id=w.intent_receipt_id
              JOIN context_retirement_attempts resumed ON resumed.operation_ulid=e.operation_ulid AND resumed.owner_epoch=e.owner_epoch
              WHERE m.operation_ulid=c.operation_ulid AND m.owner_epoch=c.owner_epoch AND mr.purpose='metadata' AND mr.previous_epoch=e.owner_epoch
                AND m.intent_resource='worktree' AND m.intent_owner_epoch=w.intent_owner_epoch AND m.intent_receipt_id=w.intent_receipt_id
                AND (prior.observation IS NULL OR prior.observation='incomplete') AND resumed.phase='fenced'
                AND m.inventory='all_absent' AND m.worktree_removal='possible' AND m.branch_removal='none'))
           OR (e.resource='worktree' AND e.owner_epoch<c.owner_epoch AND EXISTS(SELECT 1 FROM context_retirement_worktree_resumptions w JOIN context_retirement_effect_intents current ON current.operation_ulid=w.operation_ulid AND current.owner_epoch=w.owner_epoch AND current.resource='worktree' JOIN context_retirement_attempts old ON old.operation_ulid=e.operation_ulid AND old.owner_epoch=e.owner_epoch WHERE w.operation_ulid=c.operation_ulid AND w.owner_epoch>e.owner_epoch AND w.owner_epoch<=c.owner_epoch AND old.phase='fenced' AND ((current.observation='removed' AND c.worktree_removal='observed') OR EXISTS(SELECT 1 FROM context_retirement_absence_receipts m WHERE m.operation_ulid=c.operation_ulid AND m.owner_epoch=c.owner_epoch AND m.intent_resource='worktree' AND m.intent_owner_epoch=current.owner_epoch AND m.intent_receipt_id=current.receipt_id AND m.worktree_removal='possible' AND c.worktree_removal='possible')))),0)))", [], |r| r.get(0)).map_err(|_| recovery_required())?;
    if invalid_effect {
        return Err(recovery_required());
    }
    let bad_observation: bool = db.query_row("SELECT EXISTS(
      SELECT 1 FROM retirement_advertisement_provenance p
      JOIN remote_operation_records o ON o.repository_id=p.repository_id AND o.operation_ulid=p.source_operation_ulid
      WHERE o.action!='poll' OR o.configuration_generation!=p.configuration_generation
       OR o.remote_name!=p.remote_name OR o.primary_branch!=p.primary_branch
       OR o.owner_epoch<p.source_owner_epoch
       OR (p.execution_kind='local_parent' AND o.owner_epoch>p.source_owner_epoch AND EXISTS(SELECT 1 FROM context_retirement_attempts a
         WHERE a.operation_ulid=p.execution_parent_ulid AND a.owner_epoch=p.execution_parent_epoch AND a.phase='active'))
       OR (p.execution_kind='local_parent' AND (o.poll_kind!='observe' OR NOT EXISTS(
         SELECT 1 FROM context_retirement_attempts a JOIN context_retirement_records c ON c.operation_ulid=a.operation_ulid
         WHERE a.operation_ulid=p.execution_parent_ulid AND a.owner_epoch=p.execution_parent_epoch
           AND a.repository_id=p.repository_id AND a.observer_operation_ulid=p.source_operation_ulid
           AND a.observer_owner_epoch=p.source_owner_epoch AND a.request_attempt IS p.execution_request_attempt
           AND c.request_ulid IS p.execution_request_ulid AND c.batch_operation_ulid IS p.execution_batch_operation_ulid
           AND c.initial_resources IN ('present','branch_only')
           AND (c.initial_resources!='present' OR c.instance_verified=1)
           AND c.configuration_generation=p.configuration_generation AND c.remote_name=p.remote_name
           AND c.primary_branch=p.primary_branch AND c.endpoint_digest=p.endpoint_digest)))
       OR (p.execution_kind='batch_gap' AND o.poll_kind!='observe')
       OR (p.execution_kind='poll_parent' AND (o.poll_kind!='update' OR p.execution_parent_ulid!=p.source_operation_ulid OR p.execution_parent_epoch!=p.source_owner_epoch))
    ) OR EXISTS(
      SELECT 1 FROM retirement_observation_receipts b JOIN retirement_advertisement_provenance p ON p.id=b.provenance_id
      WHERE b.repository_id!=p.repository_id OR b.configuration_generation!=p.configuration_generation
       OR (b.consumer_kind='single' AND EXISTS(SELECT 1 FROM context_retirement_records c
         WHERE c.operation_ulid=b.consumer_operation_ulid AND c.initial_resources='present' AND c.instance_verified!=1))
       OR b.consumer_request_ulid IS NOT p.execution_request_ulid OR b.consumer_request_attempt IS NOT p.execution_request_attempt
       OR (b.consumer_kind='single' AND (p.execution_kind!='local_parent' OR p.execution_batch_operation_ulid IS NOT NULL
         OR b.consumer_operation_ulid!=p.execution_parent_ulid OR b.consumer_attempt_epoch!=p.execution_parent_epoch))
       OR (b.consumer_kind='batch' AND (b.consumer_operation_ulid IS NOT p.execution_batch_operation_ulid OR b.consumer_attempt_epoch IS NOT p.execution_batch_attempt_epoch))
    ) OR EXISTS(
      SELECT 1 FROM retirement_observation_pins pin JOIN retirement_advertisement_provenance p ON p.id=pin.provenance_id
      JOIN remote_observation_batches batch ON batch.id=pin.observation_batch_id
      LEFT JOIN retirement_observation_receipts b ON b.id=pin.binding_id
      WHERE pin.repository_id!=p.repository_id OR pin.observation_batch_id!=p.observation_batch_id
       OR batch.repository_id!=p.repository_id OR batch.configuration_generation!=p.configuration_generation
       OR batch.remote_name!=p.remote_name OR batch.primary_branch!=p.primary_branch OR batch.observed_at!=p.observed_at
       OR (pin.pin_kind='source_attempt' AND (p.execution_kind!='poll_parent' OR pin.holder_operation_ulid!=p.source_operation_ulid OR pin.holder_epoch!=p.source_owner_epoch))
       OR (pin.pin_kind='consumer_attempt' AND (b.provenance_id IS NOT p.id OR b.consumer_operation_ulid IS NOT pin.holder_operation_ulid OR b.consumer_attempt_epoch IS NOT pin.holder_epoch))
    ) OR EXISTS(
      SELECT 1 FROM retirement_observation_receipts b
      WHERE NOT EXISTS(SELECT 1 FROM retirement_observation_pins pin WHERE pin.binding_id=b.id AND pin.pin_kind='consumer_attempt')
        AND NOT EXISTS(SELECT 1 FROM retirement_observation_pin_releases r WHERE r.binding_id=b.id AND r.provenance_id=b.provenance_id AND r.pin_kind='consumer_attempt' AND r.holder_operation_ulid=b.consumer_operation_ulid AND r.holder_epoch=b.consumer_attempt_epoch)
    ) OR EXISTS(
      SELECT 1 FROM retirement_advertisement_provenance p
      WHERE NOT EXISTS(SELECT 1 FROM retirement_observation_receipts b WHERE b.provenance_id=p.id)
        AND (p.execution_kind!='poll_parent' OR (
          NOT EXISTS(SELECT 1 FROM retirement_observation_pins pin WHERE pin.provenance_id=p.id AND pin.pin_kind='source_attempt')
          AND NOT EXISTS(SELECT 1 FROM retirement_observation_pin_releases r WHERE r.provenance_id=p.id AND r.pin_kind='source_attempt')))
    )", [], |r| r.get(0)).map_err(|_| recovery_required())?;
    if bad_observation {
        return Err(recovery_required());
    }
    let bad_release: bool = db.query_row("SELECT EXISTS(
      SELECT 1 FROM retirement_observation_pin_releases r
      JOIN retirement_advertisement_provenance p ON p.id=r.provenance_id
      LEFT JOIN retirement_observation_receipts b ON b.id=r.binding_id
      WHERE r.repository_id!=p.repository_id OR r.observation_batch_id!=p.observation_batch_id
        OR EXISTS(SELECT 1 FROM retirement_observation_pins pin WHERE pin.provenance_id=r.provenance_id AND pin.pin_kind=r.pin_kind AND pin.holder_operation_ulid=r.holder_operation_ulid AND pin.holder_epoch=r.holder_epoch)
         OR (r.pin_kind='consumer_attempt' AND (r.reason NOT IN ('consumer_terminal','consumer_fenced') OR b.provenance_id IS NOT p.id
          OR b.consumer_operation_ulid IS NOT r.holder_operation_ulid OR b.consumer_attempt_epoch IS NOT r.holder_epoch
          OR (b.consumer_kind='single' AND NOT EXISTS(SELECT 1 FROM context_retirement_records c JOIN context_retirement_attempts a ON a.operation_ulid=c.operation_ulid
            WHERE c.operation_ulid=b.consumer_operation_ulid AND a.owner_epoch=b.consumer_attempt_epoch AND ((r.reason='consumer_terminal' AND a.phase IN ('ended','fenced') AND c.phase IN ('complete','refused','abandoned')) OR (r.reason='consumer_fenced' AND a.phase='fenced' AND EXISTS(SELECT 1 FROM context_retirement_reentries rr WHERE rr.operation_ulid=c.operation_ulid AND rr.previous_epoch=a.owner_epoch AND rr.owner_epoch<=c.owner_epoch)))))))
        OR (r.pin_kind='source_attempt' AND (p.execution_kind!='poll_parent' OR r.holder_operation_ulid!=p.source_operation_ulid OR r.holder_epoch!=p.source_owner_epoch
          OR r.reason NOT IN ('source_terminal','source_fenced','handoff_decided')
          OR NOT EXISTS(SELECT 1 FROM remote_operation_records o WHERE o.repository_id=p.repository_id AND o.operation_ulid=p.source_operation_ulid
            AND (o.owner_epoch>p.source_owner_epoch OR o.phase IN ('completed','cancelled','interrupted')))))
    )", [], |r| r.get(0)).map_err(|_| recovery_required())?;
    if bad_release {
        return Err(recovery_required());
    }
    let mut results = db.prepare("SELECT c.phase,c.final_code,c.final_result,c.worktree_removal,c.branch_removal,c.materialization_settled,c.absence_basis,o.completed_step FROM context_retirement_records c JOIN operation_records o ON o.id=c.local_record_id WHERE c.final_result IS NOT NULL").map_err(|_| recovery_required())?;
    let rows = results
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, bool>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, Option<String>>(7)?,
            ))
        })
        .map_err(|_| recovery_required())?;
    for row in rows {
        let (phase, code, json, worktree, branch, settled, basis, step) =
            row.map_err(|_| recovery_required())?;
        let report = super::ContextRetirementReport::decode(&json)?;
        use super::{RetirementDisposition as D, RetirementRemoval as R};
        let removal = |value| match value {
            R::None => "none",
            R::Possible => "possible",
            R::Observed => "observed",
        };
        let valid = match report.outcome {
            D::AlreadyAbsent => {
                phase == "complete"
                    && report.code == crate::results::ResultCode::AlreadyApplied
                    && basis == "initial"
                    && settled
                    && worktree == "none"
                    && branch == "none"
                    && step.as_deref() == Some("context_retirement_absent")
            }
            D::Retired => {
                phase == "complete"
                    && report.code == crate::results::ResultCode::Ok
                    && basis == "owned_intent"
                    && settled
                    && step.as_deref() == Some("context_branch_released")
            }
            D::Refused => {
                phase == "refused"
                    && !matches!(
                        report.code,
                        crate::results::ResultCode::Ok
                            | crate::results::ResultCode::AlreadyApplied
                            | crate::results::ResultCode::DiscoveryPending
                    )
                    && worktree == "none"
                    && branch == "none"
                    && basis == "none"
                    && !settled
                    && step.is_none()
            }
            D::Preserved => {
                phase == "abandoned"
                    && report.code == crate::results::ResultCode::ExternalChange
                    && step.as_deref() == Some("context_retirement_preserved")
            }
        };
        if !valid
            || report.code.as_str() != code
            || removal(report.worktree_removal) != worktree
            || removal(report.branch_removal) != branch
            || report.materialization_settled != settled
        {
            return Err(recovery_required());
        }
    }
    Ok(())
}

pub(super) fn audit_connection(db: &Connection) -> Result<(), RepositoryError> {
    audit(db)
}

/// Metadata-only Final for a selected, no-effect domain attempt whose original
/// generation has been fenced. It grants no remote/Git mutation authority.
pub(super) fn finish_configuration_refusal(
    tx: &rusqlite::Transaction<'_>,
    operation: &str,
    root: &std::path::Path,
) -> Result<Option<super::ContextRetirementReport>, RepositoryError> {
    struct Selected {
        local_id: i64,
        repository_id: i64,
        epoch: i64,
        observer: Option<String>,
        observer_epoch: Option<i64>,
    }
    let row = tx.query_row("SELECT c.local_record_id,c.repository_id,c.owner_epoch,a.observer_operation_ulid,a.observer_owner_epoch
      FROM context_retirement_records c JOIN operation_records o ON o.id=c.local_record_id
      JOIN context_retirement_attempts a ON a.operation_ulid=c.operation_ulid AND a.owner_epoch=c.owner_epoch
      JOIN remote_polling_state p ON p.repository_id=c.repository_id
      WHERE c.operation_ulid=?1 AND o.root_path=?2 AND c.phase='selected' AND o.state!='completed' AND a.phase='active'
        AND c.configuration_generation IS NOT NULL AND c.configuration_generation!=p.configuration_generation
        AND c.request_ulid IS NULL AND c.batch_operation_ulid IS NULL AND a.request_attempt IS NULL AND a.batch_attempt_epoch IS NULL
        AND c.worktree_removal='none' AND c.branch_removal='none' AND c.absence_basis='none'
        AND NOT EXISTS(SELECT 1 FROM context_retirement_effect_intents e WHERE e.operation_ulid=c.operation_ulid)",
        params![operation,root.to_str()], |r| Ok(Selected { local_id: r.get(0)?, repository_id: r.get(1)?, epoch: r.get(2)?, observer: r.get(3)?, observer_epoch: r.get(4)? })).optional().map_err(RepositoryError::sqlite)?;
    let Some(Selected {
        local_id,
        repository_id,
        epoch,
        observer,
        observer_epoch,
    }) = row
    else {
        return Ok(None);
    };
    let blocked: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM operation_records WHERE root_path=?1 AND id!=?2 AND (state!='completed' OR completed_step='comment_registration_pending'))
      OR EXISTS(SELECT 1 FROM remote_operation_records WHERE repository_id=?3 AND phase IN ('reserved','advertising','persisting','fetch_prepared','fetch_observed','local_prepared','local_fast_forwarded','push_prepared','push_returned','push_verified','reconciling') AND operation_ulid IS NOT ?4)",
      params![root.to_str(),local_id,repository_id,observer], |r| r.get(0)).map_err(RepositoryError::sqlite)?;
    if blocked {
        return Err(recovery_required());
    }
    if let Some(observer) = observer {
        let valid: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records o JOIN context_retirement_records c ON c.repository_id=o.repository_id
          WHERE c.operation_ulid=?1 AND o.operation_ulid=?2 AND o.owner_epoch=?3 AND o.action='poll' AND o.poll_kind='observe' AND o.priority='manual'
            AND o.configuration_generation=c.configuration_generation AND o.remote_name=c.remote_name AND o.primary_branch=c.primary_branch)",
            params![operation,observer,observer_epoch], |r| r.get(0)).map_err(RepositoryError::sqlite)?;
        if !valid {
            return Err(recovery_required());
        }
        tx.execute("UPDATE remote_operation_records SET owner_epoch=owner_epoch+1,phase='interrupted',outcome='configuration_required' WHERE repository_id=?1 AND operation_ulid=?2 AND phase NOT IN ('completed','cancelled','interrupted','failed')", params![repository_id,observer]).map_err(RepositoryError::sqlite)?;
    }
    let report =
        super::ContextRetirementReport::refused(crate::results::ResultCode::ExternalChange);
    tx.execute("UPDATE context_retirement_records SET phase='refused',final_code=?2,final_result=?3 WHERE operation_ulid=?1", params![operation,report.code.as_str(),report.encode()?]).map_err(RepositoryError::sqlite)?;
    if tx.execute("UPDATE context_retirement_attempts SET phase='ended' WHERE operation_ulid=?1 AND owner_epoch=?2 AND phase='active'", params![operation,epoch]).map_err(RepositoryError::sqlite)?!=1 { return Err(recovery_required()); }
    release_single_consumer(tx, operation, epoch)?;
    if tx
        .execute(
            "UPDATE operation_records SET state='completed' WHERE id=?1 AND state!='completed'",
            [local_id],
        )
        .map_err(RepositoryError::sqlite)?
        != 1
    {
        return Err(recovery_required());
    }
    audit(tx)?;
    Ok(Some(report))
}

/// Z for an exact single consumer, inside its holder's settlement transaction.
/// Batch/source scopes are deliberately not admitted by this capability.
fn release_single_consumer(
    tx: &rusqlite::Transaction<'_>,
    operation: &str,
    epoch: i64,
) -> Result<(), RepositoryError> {
    release_consumer(tx, operation, epoch, false)
}
pub(super) fn release_replaced_consumer(
    tx: &rusqlite::Transaction<'_>,
    operation: &str,
    epoch: i64,
) -> Result<(), RepositoryError> {
    release_consumer(tx, operation, epoch, true)
}
fn release_consumer(
    tx: &rusqlite::Transaction<'_>,
    operation: &str,
    epoch: i64,
    replaced: bool,
) -> Result<(), RepositoryError> {
    let terminal: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_records c JOIN context_retirement_attempts a ON a.operation_ulid=c.operation_ulid
      WHERE c.operation_ulid=?1 AND a.owner_epoch=?2 AND a.phase IN ('ended','fenced') AND c.phase IN ('complete','refused','abandoned') AND c.request_ulid IS NULL AND c.batch_operation_ulid IS NULL)", params![operation,epoch], |r| r.get(0)).map_err(RepositoryError::sqlite)?;
    let fenced:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_reentries rr JOIN context_retirement_attempts a ON a.operation_ulid=rr.operation_ulid AND a.owner_epoch=rr.previous_epoch JOIN context_retirement_records c ON c.operation_ulid=rr.operation_ulid WHERE rr.operation_ulid=?1 AND rr.previous_epoch=?2 AND a.phase='fenced' AND rr.owner_epoch=c.owner_epoch AND c.request_ulid IS NULL AND c.batch_operation_ulid IS NULL)",params![operation,epoch],|r|r.get(0)).map_err(RepositoryError::sqlite)?;
    if (!replaced && !terminal) || (replaced && !fenced) {
        return Err(recovery_required());
    }
    let reason = if replaced {
        "consumer_fenced"
    } else {
        "consumer_terminal"
    };
    let binding: Option<(i64,i64,i64,i64)> = tx.query_row("SELECT b.id,p.id,p.repository_id,p.observation_batch_id FROM retirement_observation_receipts b JOIN retirement_advertisement_provenance p ON p.id=b.provenance_id
      WHERE b.consumer_kind='single' AND b.consumer_operation_ulid=?1 AND b.consumer_attempt_epoch=?2", params![operation,epoch], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(RepositoryError::sqlite)?;
    let Some((binding, provenance, repository, batch)) = binding else {
        return Ok(());
    };
    let released: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM retirement_observation_pin_releases WHERE provenance_id=?1 AND pin_kind='consumer_attempt' AND holder_operation_ulid=?2 AND holder_epoch=?3 AND repository_id=?4 AND observation_batch_id=?5 AND binding_id=?6 AND reason=?7)", params![provenance,operation,epoch,repository,batch,binding,reason], |r| r.get(0)).map_err(RepositoryError::sqlite)?;
    let live: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM retirement_observation_pins WHERE provenance_id=?1 AND pin_kind='consumer_attempt' AND holder_operation_ulid=?2 AND holder_epoch=?3 AND repository_id=?4 AND observation_batch_id=?5 AND binding_id=?6)", params![provenance,operation,epoch,repository,batch,binding], |r| r.get(0)).map_err(RepositoryError::sqlite)?;
    if released {
        // Never resolve/collect a historical scalar ID: it may have been reused.
        return if live {
            Err(recovery_required())
        } else {
            Ok(())
        };
    }
    if !live {
        return Err(recovery_required());
    }
    let exact: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM remote_observation_batches batch JOIN retirement_advertisement_provenance p ON p.observation_batch_id=batch.id
      WHERE p.id=?1 AND batch.repository_id=p.repository_id AND batch.remote_name=p.remote_name AND batch.primary_branch=p.primary_branch AND batch.configuration_generation=p.configuration_generation AND batch.observed_at=p.observed_at)", [provenance], |r| r.get(0)).map_err(RepositoryError::sqlite)?;
    if !exact {
        return Err(recovery_required());
    }
    // Preserve the existing receipt-boundary rollback control without installing
    // an unknown release-table attachment (now correctly rejected by R1).
    #[cfg(test)]
    if set_release_failure(false) {
        return Err(recovery_required());
    }
    tx.execute("INSERT INTO retirement_observation_pin_releases(provenance_id,pin_kind,holder_operation_ulid,holder_epoch,repository_id,observation_batch_id,binding_id,reason) VALUES(?1,'consumer_attempt',?2,?3,?4,?5,?6,?7)", params![provenance,operation,epoch,repository,batch,binding,reason]).map_err(RepositoryError::sqlite)?;
    if tx.execute("DELETE FROM retirement_observation_pins WHERE provenance_id=?1 AND pin_kind='consumer_attempt' AND holder_operation_ulid=?2 AND holder_epoch=?3 AND repository_id=?4 AND observation_batch_id=?5 AND binding_id=?6", params![provenance,operation,epoch,repository,batch,binding]).map_err(RepositoryError::sqlite)?!=1 { return Err(recovery_required()); }
    tx.execute("DELETE FROM remote_observation_batches WHERE repository_id=?1 AND id=?2 AND is_current=0
      AND NOT EXISTS(SELECT 1 FROM remote_integration_windows w WHERE w.observation_batch_id=remote_observation_batches.id)
      AND NOT EXISTS(SELECT 1 FROM retirement_observation_pins pin WHERE pin.repository_id=remote_observation_batches.repository_id AND pin.observation_batch_id=remote_observation_batches.id)", params![repository,batch]).map_err(RepositoryError::sqlite)?;
    Ok(())
}

/// Called only inside the claimed refresh's completion transaction, before its
/// local blocker is released. This grants no discovery or Git authority itself.
pub(in super::super) fn finalize_index(
    tx: &rusqlite::Transaction<'_>,
    owner: super::super::recovery::IndexOwner,
    repository_id: i64,
    root: &std::path::Path,
) -> Result<(), RepositoryError> {
    let operation: Option<(Option<String>,String)> = tx.query_row("SELECT operation_ulid,action FROM operation_records WHERE id=?1 AND root_path=?2 AND state='indexing' AND index_owner_epoch=?3",
        params![owner.record_id,root.to_str(),owner.epoch], |r| Ok((r.get(0)?,r.get(1)?))).optional().map_err(|_| recovery_required())?;
    let Some((operation, action)) = operation else {
        return Err(recovery_required());
    };
    if action != "retire_context" {
        if operation
            .as_ref()
            .is_some_and(|operation| owned_identity(tx, operation).unwrap_or(true))
        {
            return Err(recovery_required());
        }
        return Ok(());
    }
    let operation = operation.ok_or_else(recovery_required)?;
    if !refresh_allowed(tx, &operation, root)? {
        return Err(recovery_required());
    }
    // Only implemented standalone handoffs. Preserving/source/batch authority
    // remains closed; refresh never manufactures an executor or repeats effects.
    let handoff: Option<(String,String,String)> = tx.query_row("SELECT absence_basis,branch_removal,worktree_removal FROM context_retirement_records c WHERE operation_ulid=?1 AND repository_id=?2 AND request_ulid IS NULL AND batch_operation_ulid IS NULL AND ((initial_resources='absent' AND absence_basis='initial' AND worktree_removal='none') OR (initial_resources='branch_only' AND absence_basis='owned_intent' AND worktree_removal='none') OR (initial_resources='present' AND instance_verified=1 AND absence_basis='owned_intent' AND worktree_removal='observed') OR (absence_basis='owned_intent' AND EXISTS(SELECT 1 FROM context_retirement_absence_receipts m WHERE m.operation_ulid=c.operation_ulid AND m.owner_epoch=c.owner_epoch)))", params![operation,repository_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|_| recovery_required())?;
    let Some((basis, branch, worktree)) = handoff else {
        return Err(recovery_required());
    };
    let result = if basis == "initial" {
        super::ContextRetirementReport::absent()
    } else {
        let branch = match branch.as_str() {
            "observed" => super::RetirementRemoval::Observed,
            "possible" => super::RetirementRemoval::Possible,
            "none" => super::RetirementRemoval::None,
            _ => return Err(recovery_required()),
        };
        super::ContextRetirementReport::retired(
            if worktree == "observed" {
                super::RetirementRemoval::Observed
            } else if worktree == "possible" {
                super::RetirementRemoval::Possible
            } else {
                super::RetirementRemoval::None
            },
            branch,
        )
    };
    if tx.execute("UPDATE context_retirement_records SET phase='complete',final_code=?2,final_result=?3 WHERE operation_ulid=?1 AND phase IN ('local_absent','index_pending')", params![operation,result.code.as_str(),result.encode()?]).map_err(RepositoryError::sqlite)? != 1 { return Err(recovery_required()); }
    if tx.execute("UPDATE context_retirement_attempts SET phase='ended' WHERE operation_ulid=?1 AND owner_epoch=(SELECT owner_epoch FROM context_retirement_records WHERE operation_ulid=?1) AND phase='active'", [&operation]).map_err(RepositoryError::sqlite)? != 1 { return Err(recovery_required()); }
    let epoch = tx
        .query_row(
            "SELECT owner_epoch FROM context_retirement_records WHERE operation_ulid=?1",
            [&operation],
            |r| r.get(0),
        )
        .map_err(RepositoryError::sqlite)?;
    release_single_consumer(tx, &operation, epoch)?;
    Ok(())
}
