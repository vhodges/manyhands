//! Private A2/A3 projection adapter. No Task 4 schema/producer, target selection,
//! enrollment, child begin or batch consumption is installed by this module.
use super::{
    record,
    source::{self, Projection},
};
use crate::repository::{
    OperationId, PollCompletion, RemotePollReport, RepositoryError, RepositoryService,
    remote::{OriginalWrapper, observation::ObservationConfiguration, state},
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use state::recovery_required;
use std::{path::Path, sync::Arc};

const CONTRACT: &str = "poll_retirement_handoff_contract_v1";

#[path = "batch.rs"]
pub(in super::super) mod batch;

// Read-only test observation at the actual A3 entry, after original refresh.
// No runtime API, authority replacement or guard bypass is provided.
#[cfg(test)]
thread_local! {static BEFORE_SETTLE:std::cell::RefCell<Option<Box<dyn FnOnce()>>>=const {std::cell::RefCell::new(None)};}
#[cfg(test)]
pub(super) fn observe_next_settle_for_testing(observe: impl FnOnce() + 'static) {
    BEFORE_SETTLE.with(|slot| {
        assert!(slot.borrow_mut().replace(Box::new(observe)).is_none());
    });
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Candidate {
    ordinal: usize,
    kind: String,
    item_id: String,
    initial_resources: String,
    expected_tip: String,
    selection_epoch: i64,
    instance_nonce: Option<String>,
    instance_verified: bool,
    child_operation_id: String,
    materialization_capture: String,
    materialization_operation_id: Option<String>,
    materialization_oid: Option<String>,
    materialization_step: Option<String>,
}
fn decode_image(bytes: &str, count: i64) -> Result<Vec<Candidate>, RepositoryError> {
    let entries: Vec<Candidate> = serde_json::from_str(bytes).map_err(|_| recovery_required())?;
    // Compare canonical bytes, NOT Value's permissive string-value equality.
    let canonical = serde_json::to_value(&entries)
        .map_err(|_| recovery_required())?
        .to_string();
    if i64::try_from(entries.len()).ok() != Some(count) || canonical.as_str() != bytes {
        return Err(recovery_required());
    }
    let mut keys = std::collections::BTreeSet::new();
    let mut children = std::collections::BTreeSet::new();
    for (ordinal, k) in entries.iter().enumerate() {
        if k.ordinal != ordinal
            || !matches!(k.kind.as_str(), "ticket" | "document")
            || !k
                .item_id
                .parse::<crate::canonical::ItemId>()
                .is_ok_and(|id| id.to_string() == k.item_id)
            || k.selection_epoch != 1
            || !git2::Oid::from_str(&k.expected_tip)
                .is_ok_and(|oid| !oid.is_zero() && oid.to_string() == k.expected_tip)
            || !OperationId::parse(&k.child_operation_id)
                .is_ok_and(|id| id.to_string() == k.child_operation_id)
            || !keys.insert((&k.kind, &k.item_id))
            || !children.insert(&k.child_operation_id)
            || match k.initial_resources.as_str() {
                "present" => {
                    !k.instance_verified
                        || !k.instance_nonce.as_ref().is_some_and(|nonce| {
                            OperationId::parse(nonce).is_ok_and(|id| id.to_string() == *nonce)
                        })
                }
                "branch_only" => k.instance_verified || k.instance_nonce.is_some(),
                _ => true,
            }
            || match k.materialization_capture.as_str() {
                "absent" => {
                    k.materialization_operation_id.is_some()
                        || k.materialization_oid.is_some()
                        || k.materialization_step.is_some()
                }
                "complete" => {
                    !k.materialization_operation_id
                        .as_ref()
                        .is_some_and(|id| OperationId::parse(id).is_ok())
                        || !k.materialization_oid.as_ref().is_some_and(|value| {
                            git2::Oid::from_str(value)
                                .is_ok_and(|oid| !oid.is_zero() && oid.to_string() == *value)
                        })
                        || k.materialization_step.as_deref() != Some("complete")
                }
                _ => true,
            }
        {
            return Err(recovery_required());
        }
    }
    Ok(entries)
}

fn no_child_collision(db: &Connection, child: &str) -> Result<(), RepositoryError> {
    for (table, column) in [
        ("operation_records", "operation_ulid"),
        ("remote_operation_records", "operation_ulid"),
        ("local_context_sync_records", "operation_ulid"),
        ("key_material_operations", "operation_id"),
        ("key_registry_changes", "operation_id"),
        ("request_operations", "operation_ulid"),
    ] {
        if crate::repository::recovery::has_table(db, table).map_err(|_| recovery_required())?
            && db
                .query_row(
                    &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE {column}=?1)"),
                    [child],
                    |r| r.get::<_, bool>(0),
                )
                .map_err(|_| recovery_required())?
            && (table != "operation_records" || !batch::admitted(db, child)?)
        {
            return Err(recovery_required());
        }
    }
    Ok(())
}

pub(super) fn reserved_child(db: &Connection, child: &str) -> Result<bool, RepositoryError> {
    if !crate::repository::recovery::has_table(db, "poll_retirement_freezes")
        .map_err(|_| recovery_required())?
    {
        return Ok(false);
    }
    contract(db)?;
    let mut query = db
        .prepare("SELECT image,candidate_count FROM poll_retirement_freezes")
        .map_err(|_| recovery_required())?;
    for row in query
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
        .map_err(|_| recovery_required())?
    {
        let (bytes, count) = row.map_err(|_| recovery_required())?;
        if decode_image(&bytes, count)?
            .iter()
            .any(|k| k.child_operation_id == child)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn verify_candidates(
    db: &Connection,
    p: &Projection,
    entries: &[Candidate],
) -> Result<(), RepositoryError> {
    let git = git2::Repository::open(&p.root).map_err(|_| recovery_required())?;
    for k in entries {
        no_child_collision(db, &k.child_operation_id)?;
        let kind = if k.kind == "ticket" {
            crate::repository::AuthoringKind::Ticket
        } else {
            crate::repository::AuthoringKind::Document
        };
        let request = super::RetireContextRequest {
            root: (&p.root).into(),
            kind,
            item_id: k.item_id.parse().map_err(|_| recovery_required())?,
            operation_id: OperationId::parse(&k.child_operation_id)
                .map_err(|_| recovery_required())?,
        };
        let tip = git2::Oid::from_str(&k.expected_tip).map_err(|_| recovery_required())?;
        super::instance::verify_frozen(&git, &request, tip, k.instance_nonce.as_deref())?;
        let rows:Vec<(String,String,String,String)>=db.prepare("SELECT kind,operation_ulid,target_oid,step FROM remote_poll_materializations WHERE repository_id=?1 AND item_id=?2").map_err(|_|recovery_required())?.query_map(params![p.repository,k.item_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|_|recovery_required())?.collect::<Result<_,_>>().map_err(|_|recovery_required())?;
        if match k.materialization_capture.as_str() {
            "absent" => !rows.is_empty(),
            "complete" => {
                rows != vec![(
                    k.kind.clone(),
                    k.materialization_operation_id
                        .clone()
                        .ok_or_else(recovery_required)?,
                    k.materialization_oid
                        .clone()
                        .ok_or_else(recovery_required)?,
                    "complete".into(),
                )]
            }
            _ => true,
        } {
            return Err(recovery_required());
        }
    }
    Ok(())
}

/// C's no-effect comparison is NOT the inverse of strict A2/A3 admission.
/// Both the full Git inspection and every required SQL read/decode/correlation
/// must succeed before an observed mismatch may authorize a stable Final.
fn compare_candidate(
    db: &Connection,
    p: &Projection,
    k: &Candidate,
) -> Result<super::instance::FrozenComparison, RepositoryError> {
    use super::instance::{self, FrozenComparison};
    no_child_collision(db, &k.child_operation_id)?;
    let git = git2::Repository::open(&p.root).map_err(|_| recovery_required())?;
    let request = super::RetireContextRequest {
        root: (&p.root).into(),
        kind: match k.kind.as_str() {
            "ticket" => crate::repository::AuthoringKind::Ticket,
            "document" => crate::repository::AuthoringKind::Document,
            _ => return Err(recovery_required()),
        },
        item_id: k.item_id.parse().map_err(|_| recovery_required())?,
        operation_id: OperationId::parse(&k.child_operation_id).map_err(|_| recovery_required())?,
    };
    let tip = git2::Oid::from_str(&k.expected_tip).map_err(|_| recovery_required())?;
    let identity = instance::compare_frozen(&git, &request, tip, k.instance_nonce.as_deref())?;
    // Do not short-circuit this evidence after an observed tip/nonce change.
    let rows:Vec<(String,String,String,String)>=db.prepare("SELECT kind,operation_ulid,target_oid,step FROM remote_poll_materializations WHERE repository_id=?1 AND item_id=?2").map_err(|_|recovery_required())?.query_map(params![p.repository,k.item_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|_|recovery_required())?.collect::<Result<_,_>>().map_err(|_|recovery_required())?;
    if rows.len() > 1 {
        return Err(recovery_required());
    }
    for (kind, owner, oid, step) in &rows {
        if kind != &k.kind
            || step != "complete"
            || !OperationId::parse(owner).is_ok_and(|id| id.to_string() == *owner)
            || !git2::Oid::from_str(oid).is_ok_and(|id| !id.is_zero() && id.to_string() == *oid)
        {
            return Err(recovery_required());
        }
        let correlated: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE repository_id=?1 AND operation_ulid=?2 AND action='poll' AND poll_kind='update')", params![p.repository,owner], |r|r.get(0)).map_err(|_|recovery_required())?;
        if !correlated {
            return Err(recovery_required());
        }
    }
    let expected = match k.materialization_capture.as_str() {
        "absent" => Vec::new(),
        "complete" => vec![(
            k.kind.clone(),
            k.materialization_operation_id
                .clone()
                .ok_or_else(recovery_required)?,
            k.materialization_oid
                .clone()
                .ok_or_else(recovery_required)?,
            "complete".into(),
        )],
        _ => return Err(recovery_required()),
    };
    Ok(
        if identity == FrozenComparison::Changed || rows != expected {
            FrozenComparison::Changed
        } else {
            FrozenComparison::Unchanged
        },
    )
}

fn contract(db: &Connection) -> Result<(), RepositoryError> {
    for table in ["poll_retirement_freeze_inputs", "poll_retirement_freezes"] {
        if !crate::repository::recovery::has_table(db, table).map_err(|_| recovery_required())? {
            return Err(recovery_required());
        }
    }
    if !db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [CONTRACT],
            |r| r.get::<_, bool>(0),
        )
        .map_err(|_| recovery_required())?
    {
        return Err(recovery_required());
    }
    Ok(())
}

pub(in super::super) fn report_image(report: &RemotePollReport) -> Result<String, RepositoryError> {
    let targets:Vec<_>=report.targets().iter().map(|r|serde_json::json!({
        "target":r.target().map(|t|t.remote_ref()),"tracking":r.target().map(|t|t.tracking_ref()),"kind":r.kind().map(|k|format!("{k:?}")),
        "item_id":r.item_id().map(ToString::to_string),"disposition":format!("{:?}",r.disposition()),
        "before_oid":r.before_oid().map(|oid|oid.to_string()),"after_oid":r.after_oid().map(|oid|oid.to_string())
    })).collect();
    Ok(serde_json::json!({"version":1,"operation_id":report.operation_id().to_string(),
        "completion":format!("{:?}",report.completion()),"completed_at":report.completed_at().unix_timestamp(),
        "next_eligible_at":report.next_eligible_at().map(|at|at.unix_timestamp()),
        "outcome":report.outcome().map(|v|format!("{v:?}")),"local_changes":report.local_changes(),"targets":targets}).to_string())
}

fn digest(p: &Projection, image: &str, report: &str) -> Vec<u8> {
    let mut hash = blake3::Hasher::new();
    hash.update(&(p.endpoint.len() as u64).to_le_bytes());
    hash.update(&p.endpoint);
    for part in [
        "manyhands-poll-retirement-image-v1",
        &p.operation,
        &p.repository.to_string(),
        &p.root,
        &p.remote,
        &p.primary,
        &p.request,
        &p.request_attempt.to_string(),
        &p.epoch.to_string(),
        &p.source_epoch.to_string(),
        &p.generation.to_string(),
        &p.base,
        image,
        report,
    ] {
        hash.update(&(part.len() as u64).to_le_bytes());
        hash.update(part.as_bytes());
    }
    hash.finalize().as_bytes().to_vec()
}

// Non-Clone and one-use. It escapes only after the owned transaction commits.
// Its constructor is the preterminal freeze, never a result/replay callback.
pub(in super::super) struct Capture {
    scope: Arc<()>,
    p: Projection,
    // Exact sealed source configuration, including selected key/source identity.
    // Process-only equality evidence: never encoded, digested or formatted.
    configuration: ObservationConfiguration,
    nonce: String,
    digest: Vec<u8>,
    report: String,
}

pub(super) fn freeze(
    service: &RepositoryService,
    tx: &Transaction<'_>,
    p: &Projection,
    configuration: ObservationConfiguration,
    report: &RemotePollReport,
) -> Result<Capture, RepositoryError> {
    contract(tx)?;
    if report.operation_id().to_string() != p.operation
        || report.completion() != PollCompletion::Completed
        || report.outcome() != Some(crate::repository::RemoteOutcomeCategory::Completed)
    {
        return Err(recovery_required());
    }
    let (count,image):(i64,String)=tx.query_row("SELECT candidate_count,image FROM poll_retirement_freeze_inputs WHERE operation_ulid=?1 AND owner_epoch=?2 AND request_attempt=?3 AND source_owner_epoch=?4 AND configuration_generation=?5",params![p.operation,p.epoch,p.request_attempt,p.source_epoch,p.generation],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|_|recovery_required())?;
    let entries = decode_image(&image, count)?;
    verify_candidates(tx, p, &entries)?;
    for k in &entries {
        if reserved_child(tx, &k.child_operation_id)?
            || k.child_operation_id == p.operation
            || !report.targets().iter().any(|row| {
                row.item_id().is_some_and(|id| id.to_string() == k.item_id)
                    && row.kind().is_some_and(|kind| {
                        crate::repository::authoring_kind_segment(&kind) == k.kind
                    })
                    && row
                        .after_oid()
                        .is_some_and(|oid| oid.to_string() == k.expected_tip)
                    && matches!(
                        row.disposition(),
                        crate::repository::PollDisposition::Closed
                            | crate::repository::PollDisposition::Promoted
                    )
            })
        {
            return Err(recovery_required());
        }
    }
    let (provenance,batch):(i64,i64)=tx.query_row("SELECT p.id,p.observation_batch_id FROM poll_retirement_attempts a JOIN retirement_advertisement_provenance p ON p.id=a.active_provenance_id JOIN retirement_observation_pins pin ON pin.provenance_id=p.id AND pin.pin_kind='source_attempt' JOIN remote_observation_batches b ON b.id=pin.observation_batch_id WHERE a.operation_ulid=?1 AND a.owner_epoch=?2 AND b.is_current=1 AND b.configuration_generation=?3 AND p.source_owner_epoch=?4",params![p.operation,p.epoch,p.generation,p.source_epoch],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|_|recovery_required())?;
    let report = report_image(report)?;
    let digest = digest(p, &image, &report);
    let nonce = OperationId::new().to_string();
    tx.execute("INSERT INTO retirement_observation_receipts(provenance_id,repository_id,consumer_kind,consumer_operation_ulid,consumer_request_ulid,consumer_request_attempt,consumer_attempt_epoch,configuration_generation,frozen_worklist_digest) VALUES(?1,?2,'batch',?3,?4,?5,?6,?7,?8)",params![provenance,p.repository,p.operation,p.request,p.request_attempt,p.epoch,p.generation,digest]).map_err(|_|recovery_required())?;
    let binding = tx.last_insert_rowid();
    tx.execute("INSERT INTO retirement_observation_pins(provenance_id,pin_kind,holder_operation_ulid,holder_epoch,repository_id,observation_batch_id,binding_id) VALUES(?1,'consumer_attempt',?2,?3,?4,?5,?6)",params![provenance,p.operation,p.epoch,p.repository,batch,binding]).map_err(|_|recovery_required())?;
    tx.execute("INSERT INTO poll_retirement_freezes(operation_ulid,owner_epoch,provenance_id,binding_id,capture_nonce,candidate_count,image,original_report,digest,eligibility,final_code) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,'unconfirmed',NULL)",params![p.operation,p.epoch,provenance,binding,nonce,count,image,report,digest]).map_err(|_|recovery_required())?;
    if tx.execute("UPDATE poll_retirement_batches SET stage='frozen' WHERE operation_ulid=?1 AND owner_epoch=?2 AND stage='unplanned'",params![p.operation,p.epoch]).map_err(|_|recovery_required())?!=1 {return Err(recovery_required());}
    Ok(Capture {
        scope: Arc::clone(&service.remote_reservation_scope),
        p: p.clone(),
        configuration,
        nonce,
        digest,
        report,
    })
}

struct Frozen {
    provenance: i64,
    binding: i64,
    nonce: String,
    count: i64,
    image: String,
    report: String,
    digest: Vec<u8>,
    eligibility: String,
    code: Option<String>,
}
fn read(db: &Connection, p: &Projection) -> Result<Frozen, RepositoryError> {
    contract(db)?;
    db.query_row("SELECT provenance_id,binding_id,capture_nonce,candidate_count,image,original_report,digest,eligibility,final_code FROM poll_retirement_freezes WHERE operation_ulid=?1 AND owner_epoch=?2",params![p.operation,p.epoch],|r|Ok(Frozen{provenance:r.get(0)?,binding:r.get(1)?,nonce:r.get(2)?,count:r.get(3)?,image:r.get(4)?,report:r.get(5)?,digest:r.get(6)?,eligibility:r.get(7)?,code:r.get(8)?})).map_err(|_|recovery_required())
}

pub(super) fn audit_correlations(db: &Connection) -> Result<(), RepositoryError> {
    if !crate::repository::recovery::has_table(db, "poll_retirement_freezes")
        .map_err(|_| recovery_required())?
    {
        return Ok(());
    }
    let invalid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM poll_retirement_freezes f LEFT JOIN poll_retirement_batches b USING(operation_ulid) LEFT JOIN poll_retirement_attempts a USING(operation_ulid,owner_epoch) LEFT JOIN retirement_advertisement_provenance p ON p.id=f.provenance_id LEFT JOIN retirement_observation_receipts binding ON binding.id=f.binding_id WHERE b.operation_ulid IS NULL OR b.stage!='frozen' OR a.operation_ulid IS NULL OR a.active_provenance_id IS NOT p.id OR binding.provenance_id IS NOT p.id OR p.execution_batch_operation_ulid IS NOT f.operation_ulid OR p.execution_batch_attempt_epoch IS NOT f.owner_epoch OR (SELECT count(*) FROM poll_retirement_freezes other WHERE other.operation_ulid=f.operation_ulid)!=1)",[],|r|r.get(0)).map_err(|_|recovery_required())?;
    if invalid {
        return Err(recovery_required());
    }
    Ok(())
}

pub(super) fn audit_source(
    db: &Connection,
    p: &Projection,
    provenance: i64,
) -> Result<(), RepositoryError> {
    let f = read(db, p)?;
    let entries = decode_image(&f.image, f.count)?;
    if f.provenance != provenance
        || f.digest != digest(p, &f.image, &f.report)
        || !OperationId::parse(&f.nonce).is_ok_and(|id| id.to_string() == f.nonce)
    {
        return Err(recovery_required());
    }
    for k in &entries {
        no_child_collision(db, &k.child_operation_id)?;
    }
    let report: serde_json::Value =
        serde_json::from_str(&f.report).map_err(|_| recovery_required())?;
    if report["version"] != 1
        || report["operation_id"] != p.operation
        || report["completion"] != "Completed"
        || report["outcome"] != "Completed"
    {
        return Err(recovery_required());
    }
    let plan = crate::repository::RemoteRefPlan::from_configuration(&p.remote, &p.primary)
        .map_err(|_| recovery_required())?;
    let decoded = RemotePollReport::decode_frozen(&f.report, &plan)?;
    for k in &entries {
        if !decoded.targets().iter().any(|row| {
            row.item_id().is_some_and(|id| id.to_string() == k.item_id)
                && row
                    .kind()
                    .is_some_and(|kind| crate::repository::authoring_kind_segment(&kind) == k.kind)
                && row
                    .after_oid()
                    .is_some_and(|oid| oid.to_string() == k.expected_tip)
                && matches!(
                    row.disposition(),
                    crate::repository::PollDisposition::Closed
                        | crate::repository::PollDisposition::Promoted
                )
        }) {
            return Err(recovery_required());
        }
    }
    let valid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM retirement_advertisement_provenance p JOIN retirement_observation_receipts b ON b.provenance_id=p.id JOIN remote_operation_records o ON o.repository_id=p.repository_id AND o.operation_ulid=p.source_operation_ulid JOIN poll_retirement_attempts a ON a.operation_ulid=p.execution_batch_operation_ulid AND a.owner_epoch=p.execution_batch_attempt_epoch WHERE p.id=?1 AND b.id=?2 AND a.active_provenance_id=p.id AND p.repository_id=?3 AND p.source_operation_ulid=?4 AND p.source_owner_epoch=?5 AND p.execution_parent_ulid=?4 AND p.execution_parent_epoch=?5 AND p.execution_batch_operation_ulid=?4 AND p.execution_batch_attempt_epoch=?6 AND p.execution_request_ulid=?7 AND p.execution_request_attempt=?8 AND p.configuration_generation=?9 AND p.remote_name=?10 AND p.primary_branch=?11 AND p.endpoint_digest=?12 AND b.consumer_kind='batch' AND b.consumer_operation_ulid=?4 AND b.consumer_request_ulid=?7 AND b.consumer_request_attempt=?8 AND b.consumer_attempt_epoch=?6 AND b.frozen_worklist_digest=?13 AND o.owner_epoch=?5 AND o.phase='completed' AND o.outcome='completed' AND o.action='poll' AND o.poll_kind='update' AND o.priority='manual' AND o.configuration_generation=?9 AND o.remote_name=?10 AND o.primary_branch=?11 AND o.poll_completed_at=?14)",params![provenance,f.binding,p.repository,p.operation,p.source_epoch,p.epoch,p.request,p.request_attempt,p.generation,p.remote,p.primary,p.endpoint,f.digest,report["completed_at"].as_i64().ok_or_else(recovery_required)?],|r|r.get(0)).map_err(|_|recovery_required())?;
    if !valid {
        return Err(recovery_required());
    }
    let bad_reason:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM retirement_observation_pin_releases WHERE provenance_id=?1 AND reason!=CASE pin_kind WHEN 'source_attempt' THEN 'handoff_decided' ELSE 'consumer_terminal' END)",[provenance],|r|r.get(0)).map_err(|_|recovery_required())?;
    if bad_reason {
        return Err(recovery_required());
    }
    let (source,consumer,releases):(i64,i64,i64)=db.query_row("SELECT (SELECT count(*) FROM retirement_observation_pins WHERE provenance_id=?1 AND pin_kind='source_attempt'),(SELECT count(*) FROM retirement_observation_pins WHERE provenance_id=?1 AND pin_kind='consumer_attempt'),(SELECT count(*) FROM retirement_observation_pin_releases WHERE provenance_id=?1)",[provenance],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|_|recovery_required())?;
    let valid = match f.eligibility.as_str() {
        "unconfirmed" => {
            p.phase == "active" && f.code.is_none() && (source, consumer, releases) == (1, 1, 0)
        }
        "eligible" => {
            if f.count == 0 {
                p.phase == "ended"
                    && f.code.as_deref() == Some("ok")
                    && (source, consumer, releases) == (0, 0, 2)
            } else {
                (p.phase == "active"
                    && f.code.is_none()
                    && (source, consumer, releases) == (0, 1, 1))
                    || (p.phase == "ended"
                        && f.code.as_deref() == Some("ok")
                        && (source, consumer, releases) == (0, 0, 2)
                        && batch::completed(db, p, &f)?)
            }
        }
        "ineligible" => {
            p.phase == "ended"
                && matches!(f.code.as_deref(), Some("discovery_pending" | "cancelled"))
                && (source, consumer, releases) == (0, 0, 2)
        }
        _ => false,
    };
    if !valid {
        return Err(recovery_required());
    }
    Ok(())
}

impl Capture {
    pub(in super::super) fn settle(
        self,
        service: &RepositoryService,
        root: &Path,
        wrapper: OriginalWrapper,
    ) -> Result<Option<batch::Live>, RepositoryError> {
        if !Arc::ptr_eq(&self.scope, &service.remote_reservation_scope)
            || root.to_str() != Some(self.p.root.as_str())
            || wrapper.report() != self.report
        {
            return Err(recovery_required());
        }
        #[cfg(test)]
        if let Some(observe) = BEFORE_SETTLE.with(|slot| slot.borrow_mut().take()) {
            observe();
        }
        let live = state::with_transaction(service, root, |tx, id| {
            record::audit_connection(tx)?;
            let mut current = source::projection(tx, &self.p.operation, self.p.epoch)?;
            current.stage = "unplanned".into();
            if current != self.p
                || id != self.p.repository
                || state::generation(tx, id)? != self.p.generation
            {
                return Err(recovery_required());
            }
            let f = read(tx, &self.p)?;
            if f.nonce != self.nonce
                || f.digest != self.digest
                || f.report != self.report
                || f.eligibility != "unconfirmed"
            {
                return Err(recovery_required());
            }
            let cancelled:bool=tx.query_row("SELECT cancel_requested FROM request_records WHERE request_ulid=?1 AND attempt=?2 AND state='accepted' AND EXISTS(SELECT 1 FROM poll_retirement_batches WHERE operation_ulid=?3 AND owner_epoch=?4)",params![self.p.request,self.p.request_attempt,self.p.operation,self.p.epoch],|r|r.get(0)).optional().map_err(|_|recovery_required())?.ok_or_else(recovery_required)?;
            let current_batch:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM retirement_observation_pins pin JOIN remote_observation_batches b ON b.id=pin.observation_batch_id WHERE pin.provenance_id=?1 AND pin.pin_kind='source_attempt' AND b.is_current=1 AND b.configuration_generation=?2)",params![f.provenance,self.p.generation],|r|r.get(0)).map_err(|_|recovery_required())?;
            if !current_batch {
                return Err(recovery_required());
            }
            let plan = crate::repository::remote::observation::observation_plan(root)
                .map_err(|_| recovery_required())?;
            if plan.remote_name() != self.p.remote
                || plan.primary_branch() != self.p.primary
                || self.configuration.endpoint_digest().as_slice() != self.p.endpoint
                || !self.configuration.matches_owned(service, tx, root, &plan)?
            {
                return Err(recovery_required());
            }
            if wrapper.complete() && !cancelled {
                crate::repository::recovery::require_no_pending_local(tx, id)?;
            }
            if !cancelled && wrapper.complete() {
                verify_candidates(tx, &self.p, &decode_image(&f.image, f.count)?)?;
            }
            decide(
                tx,
                &self.p,
                &f,
                if cancelled {
                    "ineligible"
                } else if wrapper.complete() {
                    "eligible"
                } else {
                    "ineligible"
                },
                if cancelled {
                    "cancelled"
                } else if wrapper.complete() {
                    "ok"
                } else {
                    "discovery_pending"
                },
            )?;
            Ok(!cancelled && wrapper.complete() && f.count > 0)
        })?;
        Ok(live.then(|| batch::Live::from_original(self)))
    }
}

fn decide(
    tx: &Transaction<'_>,
    p: &Projection,
    f: &Frozen,
    eligibility: &str,
    code: &str,
) -> Result<(), RepositoryError> {
    let terminal = eligibility == "ineligible" || f.count == 0;
    if tx.execute("UPDATE poll_retirement_freezes SET eligibility=?3,final_code=?4 WHERE operation_ulid=?1 AND owner_epoch=?2 AND eligibility='unconfirmed' AND capture_nonce=?5",params![p.operation,p.epoch,eligibility,terminal.then_some(code),f.nonce]).map_err(|_|recovery_required())?!=1 {return Err(recovery_required());}
    if terminal && tx.execute("UPDATE poll_retirement_attempts SET phase='ended' WHERE operation_ulid=?1 AND owner_epoch=?2 AND phase='active'",params![p.operation,p.epoch]).map_err(|_|recovery_required())?!=1 {return Err(recovery_required());}
    if eligibility == "eligible" && f.count == 0 {
        batch::settle_empty(tx, p)?;
    }
    record::release_bound_source(tx, f.provenance, f.binding, terminal)?;
    record::audit_connection(tx)
}

/// Recovery settles only undecided completed-P metadata. It never returns a
/// capture token, reopens P, upgrades eligibility, or resolves released batch IDs.
pub(in super::super) fn recover_unconfirmed(
    service: &RepositoryService,
    root: &Path,
    operation: OperationId,
) -> Result<(), RepositoryError> {
    state::with_transaction(service, root, |tx, id| {
        recover_in_transaction(tx, id, root, operation)
    })
}
fn recover_in_transaction(
    tx: &Transaction<'_>,
    id: i64,
    root: &Path,
    operation: OperationId,
) -> Result<(), RepositoryError> {
    if !crate::repository::recovery::has_table(tx, "poll_retirement_freezes")
        .map_err(|_| recovery_required())?
    {
        return Ok(());
    }
    let epoch:Option<i64>=tx.query_row("SELECT owner_epoch FROM poll_retirement_freezes WHERE operation_ulid=?1 AND eligibility='unconfirmed'",[operation.to_string()],|r|r.get(0)).optional().map_err(|_|recovery_required())?;
    let Some(epoch) = epoch else {
        return Ok(());
    };
    record::audit_connection(tx)?;
    let p = source::projection(tx, &operation.to_string(), epoch)?;
    if p.repository != id || p.root != root.to_str().ok_or_else(recovery_required)? {
        return Err(recovery_required());
    }
    let f = read(tx, &p)?;
    decide(tx, &p, &f, "ineligible", "discovery_pending")
}

pub(super) fn require_release(
    db: &Connection,
    provenance: i64,
    binding: i64,
    consumer_terminal: bool,
) -> Result<(), RepositoryError> {
    let valid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM poll_retirement_freezes f JOIN poll_retirement_attempts a USING(operation_ulid,owner_epoch) JOIN remote_operation_records o USING(operation_ulid) WHERE f.provenance_id=?1 AND f.binding_id=?2 AND ((?3 AND a.phase='ended' AND f.final_code IS NOT NULL AND (f.eligibility='ineligible' OR (f.eligibility='eligible' AND f.candidate_count=0))) OR (NOT ?3 AND a.phase='active' AND f.final_code IS NULL AND f.eligibility='eligible' AND f.candidate_count>0)) AND o.phase='completed' AND o.owner_epoch=a.source_owner_epoch)",params![provenance,binding,consumer_terminal],|r|r.get(0)).map_err(|_|recovery_required())?;
    if !valid {
        if !consumer_terminal {
            return Err(recovery_required());
        }
        batch::require_completed_release(db, provenance, binding)?;
    }
    Ok(())
}

/// Read-only public P replay after conservative recovery. No live Capture or
/// execution authority is reconstructed, even when the public wrapper is Complete.
pub(in super::super) fn replay_report(
    service: &RepositoryService,
    root: &Path,
    operation: OperationId,
) -> Result<Option<(RemotePollReport, bool)>, RepositoryError> {
    state::with_transaction(service, root, |tx, id| {
        recover_in_transaction(tx, id, root, operation)?;
        if !crate::repository::recovery::has_table(tx, "poll_retirement_freezes")
            .map_err(|_| recovery_required())?
        {
            return Ok(None);
        }
        let epoch: Option<i64> = tx
            .query_row(
                "SELECT owner_epoch FROM poll_retirement_freezes WHERE operation_ulid=?1",
                [operation.to_string()],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| recovery_required())?;
        let Some(epoch) = epoch else {
            return Ok(None);
        };
        record::audit_connection(tx)?;
        let p = source::projection(tx, &operation.to_string(), epoch)?;
        if p.repository != id || p.root != root.to_str().ok_or_else(recovery_required)? {
            return Err(recovery_required());
        }
        let f = read(tx, &p)?;
        let plan = crate::repository::RemoteRefPlan::from_configuration(&p.remote, &p.primary)
            .map_err(|_| recovery_required())?;
        let report = RemotePollReport::decode_frozen(&f.report, &plan)?;
        let pending = tx
            .query_row(
                "SELECT refresh_required FROM repositories WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .map_err(|_| recovery_required())?;
        Ok(Some((report, pending)))
    })
}
