//! Original-live pass metadata, not a journal or restartable work queue. Each
//! admitted child is an ordinary request. A durable selection or eligible flag
//! alone can never construct the process capability in `handoff`.
use super::{
    ContextRetirementReport, RetireContextRequest, RetirementDisposition, record, request_scope,
};
use crate::repository::{
    self, OperationId, RemotePollReport, RepositoryError, RepositoryService, mutation,
    remote::state,
};
use mutation::{
    Accepted, RequestId,
    request_store::{DigestSalt, FieldValue, IntentDigest, RequestRecord, ScopeKey},
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use state::recovery_required;
use std::path::Path;
#[path = "pass_report.rs"]
mod report;
pub(in super::super) use report::{finish, lost_initial, save_preaccept_refusal};

pub(super) fn objects() -> Vec<(&'static str, &'static str, String)> {
    let mut result=vec![
        ("table","poll_retirement_passes", "CREATE TABLE poll_retirement_passes (
 operation_ulid TEXT PRIMARY KEY NOT NULL,
 repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
 request_ulid TEXT NOT NULL UNIQUE REFERENCES request_records(request_ulid) ON DELETE NO ACTION,
 request_attempt INTEGER NOT NULL CHECK(typeof(request_attempt)='integer' AND request_attempt>0),
 live_nonce TEXT NOT NULL,
 live INTEGER NOT NULL CHECK(live IN (0,1)),
 source_owner_epoch INTEGER CHECK(source_owner_epoch IS NULL OR source_owner_epoch>=0),
 configuration_generation INTEGER CHECK(configuration_generation IS NULL OR configuration_generation>=0),
 remote_name TEXT, primary_branch TEXT, endpoint_digest BLOB,
 provenance_id INTEGER UNIQUE REFERENCES retirement_advertisement_provenance(id) ON DELETE NO ACTION,
 binding_id INTEGER UNIQUE REFERENCES retirement_observation_receipts(id) ON DELETE NO ACTION,
 image TEXT CHECK(image IS NULL OR json_valid(image)),
 original_report TEXT CHECK(original_report IS NULL OR json_valid(original_report)),
 image_digest BLOB CHECK(image_digest IS NULL OR length(image_digest)=32),
 capture_nonce TEXT,
 handoff TEXT NOT NULL CHECK(handoff IN ('unconfirmed','eligible','ineligible')),
 summary TEXT CHECK(summary IS NULL OR json_valid(summary)),
 CHECK((image IS NULL)=(original_report IS NULL) AND (image IS NULL)=(image_digest IS NULL) AND (image IS NULL)=(capture_nonce IS NULL) AND (image IS NULL)=(binding_id IS NULL)),
 CHECK((remote_name IS NULL)=(primary_branch IS NULL) AND (remote_name IS NULL)=(configuration_generation IS NULL) AND (remote_name IS NULL)=(source_owner_epoch IS NULL) AND (remote_name IS NULL)=(endpoint_digest IS NULL)),
 CHECK(endpoint_digest IS NULL OR length(endpoint_digest)=32),
 CHECK(summary IS NULL OR live=0)
 )".into()),
        ("table","poll_retirement_candidates","CREATE TABLE poll_retirement_candidates (
 operation_ulid TEXT NOT NULL REFERENCES poll_retirement_passes(operation_ulid) ON DELETE CASCADE,
 poll_ordinal INTEGER NOT NULL CHECK(poll_ordinal>=0),
 candidate TEXT NOT NULL CHECK(json_valid(candidate)),
 verified INTEGER NOT NULL CHECK(verified IN (0,1)),
 PRIMARY KEY(operation_ulid,poll_ordinal)
 )".into()),
        ("table","poll_retirement_targets","CREATE TABLE poll_retirement_targets (
 operation_ulid TEXT NOT NULL REFERENCES poll_retirement_passes(operation_ulid) ON DELETE CASCADE,
 ordinal INTEGER NOT NULL CHECK(ordinal>=0),
 child_request_ulid TEXT NOT NULL UNIQUE,
 candidate TEXT NOT NULL CHECK(json_valid(candidate)),
 child_operation_ulid TEXT UNIQUE,
 admission TEXT NOT NULL CHECK(admission IN ('reserved','accepted','selected','no_effect','refused')),
 refusal_code TEXT,
 PRIMARY KEY(operation_ulid,ordinal),
 CHECK((admission IN ('accepted','selected','no_effect'))=(child_operation_ulid IS NOT NULL)),
 CHECK((admission='refused')=(refusal_code IS NOT NULL))
 )".into()),
        ("table","poll_retirement_child_sources","CREATE TABLE poll_retirement_child_sources (
 repository_id INTEGER NOT NULL,
 child_operation_ulid TEXT NOT NULL,
 child_request_ulid TEXT NOT NULL UNIQUE REFERENCES request_records(request_ulid) ON DELETE NO ACTION,
 parent_request_ulid TEXT NOT NULL REFERENCES request_records(request_ulid) ON DELETE NO ACTION,
 parent_operation_ulid TEXT NOT NULL REFERENCES poll_retirement_passes(operation_ulid) ON DELETE CASCADE,
 ordinal INTEGER NOT NULL CHECK(ordinal>=0),
 image_digest BLOB NOT NULL CHECK(length(image_digest)=32),
 candidate TEXT NOT NULL CHECK(json_valid(candidate)),
 provenance_id INTEGER NOT NULL REFERENCES retirement_advertisement_provenance(id) ON DELETE CASCADE,
 initial_receipt_id INTEGER NOT NULL UNIQUE REFERENCES retirement_observation_receipts(id) ON DELETE CASCADE,
 local_epoch INTEGER NOT NULL CHECK(local_epoch=1),
 request_attempt INTEGER NOT NULL CHECK(request_attempt>0),
 PRIMARY KEY(repository_id,child_operation_ulid),
 FOREIGN KEY(repository_id,child_operation_ulid) REFERENCES context_retirement_records(repository_id,operation_ulid) ON DELETE CASCADE
 )".into()),
        ("trigger","poll_retirement_identity","CREATE TRIGGER poll_retirement_identity BEFORE UPDATE OF operation_ulid,repository_id,request_ulid,request_attempt,live_nonce ON poll_retirement_passes BEGIN SELECT RAISE(ABORT,'immutable pass identity'); END".into()),
        ("trigger","poll_retirement_source_identity","CREATE TRIGGER poll_retirement_source_identity BEFORE UPDATE OF source_owner_epoch,configuration_generation,remote_name,primary_branch,endpoint_digest ON poll_retirement_passes WHEN OLD.source_owner_epoch IS NOT NULL BEGIN SELECT RAISE(ABORT,'immutable pass source'); END".into()),
        ("trigger","poll_retirement_capture_identity","CREATE TRIGGER poll_retirement_capture_identity BEFORE UPDATE OF image,original_report,image_digest,capture_nonce,binding_id ON poll_retirement_passes WHEN OLD.image IS NOT NULL BEGIN SELECT RAISE(ABORT,'immutable pass image'); END".into()),
        ("trigger","poll_retirement_provenance_identity","CREATE TRIGGER poll_retirement_provenance_identity BEFORE UPDATE OF provenance_id ON poll_retirement_passes WHEN OLD.provenance_id IS NOT NULL BEGIN SELECT RAISE(ABORT,'immutable pass advertisement'); END".into()),
        ("trigger","poll_retirement_live_forward","CREATE TRIGGER poll_retirement_live_forward BEFORE UPDATE OF live ON poll_retirement_passes WHEN NEW.live>OLD.live BEGIN SELECT RAISE(ABORT,'stopped pass'); END".into()),
        ("trigger","poll_retirement_handoff_forward","CREATE TRIGGER poll_retirement_handoff_forward BEFORE UPDATE OF handoff ON poll_retirement_passes WHEN OLD.handoff!='unconfirmed' BEGIN SELECT RAISE(ABORT,'decided handoff'); END".into()),
        ("trigger","poll_retirement_summary_immutable","CREATE TRIGGER poll_retirement_summary_immutable BEFORE UPDATE ON poll_retirement_passes WHEN OLD.summary IS NOT NULL BEGIN SELECT RAISE(ABORT,'immutable pass result'); END".into()),
        ("trigger","poll_retirement_target_identity","CREATE TRIGGER poll_retirement_target_identity BEFORE UPDATE OF operation_ulid,ordinal,child_request_ulid,candidate ON poll_retirement_targets BEGIN SELECT RAISE(ABORT,'immutable selected target'); END".into()),
        ("trigger","poll_retirement_child_identity","CREATE TRIGGER poll_retirement_child_identity BEFORE UPDATE OF child_operation_ulid ON poll_retirement_targets WHEN OLD.child_operation_ulid IS NOT NULL BEGIN SELECT RAISE(ABORT,'immutable child mapping'); END".into()),
        ("trigger","poll_retirement_target_sealed","CREATE TRIGGER poll_retirement_target_sealed BEFORE UPDATE ON poll_retirement_targets WHEN EXISTS(SELECT 1 FROM poll_retirement_passes p WHERE p.operation_ulid=OLD.operation_ulid AND p.summary IS NOT NULL) BEGIN SELECT RAISE(ABORT,'ended pass target'); END".into()),
        ("trigger","poll_retirement_membership_immutable","CREATE TRIGGER poll_retirement_membership_immutable BEFORE UPDATE ON poll_retirement_child_sources BEGIN SELECT RAISE(ABORT,'immutable initial consumer'); END".into()),
        ("trigger","poll_retirement_membership_retained","CREATE TRIGGER poll_retirement_membership_retained BEFORE DELETE ON poll_retirement_child_sources WHEN EXISTS(SELECT 1 FROM context_retirement_records c WHERE c.operation_ulid=OLD.child_operation_ulid AND c.phase NOT IN ('complete','refused','abandoned')) BEGIN SELECT RAISE(ABORT,'pending child source'); END".into()),
    ];
    for (name, table) in [
        (
            "poll_retirement_candidate_frozen",
            "poll_retirement_candidates",
        ),
        ("poll_retirement_target_frozen", "poll_retirement_targets"),
    ] {
        result.push(("trigger",name,format!("CREATE TRIGGER {name} BEFORE INSERT ON {table} WHEN EXISTS(SELECT 1 FROM poll_retirement_passes p WHERE p.operation_ulid=NEW.operation_ulid AND p.summary IS NOT NULL) BEGIN SELECT RAISE(ABORT,'ended pass'); END")));
    }
    result
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Candidate {
    pub poll_ordinal: usize,
    pub kind: String,
    pub item_id: String,
    pub resources: String,
    pub tip: Option<String>,
    pub nonce: Option<String>,
    pub materialization: Option<(String, String, String)>,
    pub refusal: Option<String>,
    pub child_request: String,
}
pub(super) fn encode<T: serde::Serialize>(v: &T) -> Result<String, RepositoryError> {
    serde_json::to_value(v)
        .map(|v| v.to_string())
        .map_err(|_| recovery_required())
}
impl Candidate {
    pub(super) fn decode(s: &str) -> Result<Self, RepositoryError> {
        let c: Self = serde_json::from_str(s).map_err(|_| recovery_required())?;
        if encode(&c)? != s
            || !matches!(
                c.resources.as_str(),
                "present" | "branch_only" | "absent" | "invalid"
            )
            || repository::authoring_kind_segment(&c.kind()?) != c.kind
            || !c
                .item_id
                .parse::<crate::canonical::ItemId>()
                .is_ok_and(|i| i.to_string() == c.item_id)
            || !RequestId::parse(&c.child_request).is_ok_and(|i| i.to_string() == c.child_request)
            || c.tip.as_ref().is_some_and(|s| {
                !git2::Oid::from_str(s).is_ok_and(|o| !o.is_zero() && o.to_string() == *s)
            })
            || c.nonce
                .as_ref()
                .is_some_and(|s| !OperationId::parse(s).is_ok_and(|i| i.to_string() == *s))
            || (c.refusal.is_none()
                && ((c.resources == "present") != c.nonce.is_some()
                    || matches!(c.resources.as_str(), "present" | "branch_only")
                        != c.tip.is_some()))
        {
            return Err(recovery_required());
        }
        if let Some((op, oid, step)) = &c.materialization
            && (!OperationId::parse(op).is_ok_and(|i| i.to_string() == *op)
                || !git2::Oid::from_str(oid).is_ok_and(|o| !o.is_zero() && o.to_string() == *oid)
                || step != "complete")
        {
            return Err(recovery_required());
        }
        if c.refusal.as_ref().is_some_and(|s| {
            !crate::results::ResultCode::ALL
                .into_iter()
                .any(|code| code.as_str() == s)
        }) {
            return Err(recovery_required());
        }
        Ok(c)
    }
    pub(super) fn kind(&self) -> Result<repository::AuthoringKind, RepositoryError> {
        match self.kind.as_str() {
            "ticket" => Ok(repository::AuthoringKind::Ticket),
            "document" => Ok(repository::AuthoringKind::Document),
            _ => Err(recovery_required()),
        }
    }
}

pub(in super::super) fn begin(
    tx: &Transaction<'_>,
    accepted: &Accepted,
) -> Result<(), RepositoryError> {
    // Ordinary request insertion and initial metadata are one atomic admission.
    // Reentry never calls this initializer or recreates a missing companion.
    require_unreserved(tx, accepted.request_id)?;
    validate_parent(tx, accepted, false)?;
    let id: i64 = tx
        .query_row(
            "SELECT id FROM repositories WHERE root_path=?1",
            [accepted.scope.as_str()],
            |r| r.get(0),
        )
        .map_err(|_| recovery_required())?;
    tx.execute("INSERT INTO poll_retirement_passes(operation_ulid,repository_id,request_ulid,request_attempt,live_nonce,live,handoff) VALUES(?1,?2,?3,?4,?5,1,'unconfirmed')",params![accepted.operation_id.to_string(),id,accepted.request_id.to_string(),accepted.attempt,OperationId::new().to_string()]).map_err(|_|recovery_required())?;
    Ok(())
}
pub(super) fn validate_parent(
    db: &Connection,
    a: &Accepted,
    original: bool,
) -> Result<bool, RepositoryError> {
    let digest = poll_intent(a.request_id, &a.scope, true);
    let cancel:Option<bool>=db.query_row("SELECT r.cancel_requested FROM request_records r WHERE r.request_ulid=?1 AND r.attempt=?2 AND r.state='accepted' AND r.scope_key=?3 AND r.command='poll once' AND r.target='' AND r.intent_digest=?4 AND r.confirmation_ulid IS NULL AND r.expected_digest IS NULL AND r.base_ref IS ?5 AND r.base_oid IS ?6 AND (SELECT count(*) FROM request_operations m WHERE m.request_ulid=r.request_ulid)=1 AND EXISTS(SELECT 1 FROM request_operations m WHERE m.request_ulid=r.request_ulid AND m.ordinal=0 AND m.family='remote' AND m.operation_ulid=?7)",params![a.request_id.to_string(),a.attempt,a.scope.as_str(),digest.to_string(),a.position.base_ref,a.position.base_oid.map(|o|o.to_string()),a.operation_id.to_string()],|r|r.get(0)).optional().map_err(|_|recovery_required())?;
    if original {
        let same:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM poll_retirement_passes WHERE operation_ulid=?1 AND request_ulid=?2 AND request_attempt=?3 AND live=1 AND summary IS NULL)",params![a.operation_id.to_string(),a.request_id.to_string(),a.attempt],|r|r.get(0)).map_err(|_|recovery_required())?;
        if !same {
            return Err(recovery_required());
        }
    }
    cancel.ok_or_else(recovery_required)
}

fn poll_intent(request: RequestId, scope: &ScopeKey, retiring: bool) -> IntentDigest {
    let builder = IntentDigest::builder(DigestSalt::Request(request), "poll once", scope, "");
    if retiring {
        builder.field("retire_completed", FieldValue::Flag(true))
    } else {
        builder
    }
    .finish()
}

/// Mode is immutable request identity, not the presence of recoverable pass
/// metadata. Unknown digests must never acquire the ordinary poll fallback.
pub(in super::super) fn is_opted_request(record: &RequestRecord) -> Result<bool, RepositoryError> {
    if record.command != "poll once"
        || !record.target.is_empty()
        || record.scope.repository().is_none()
        || record.confirmation.is_some()
        || record.expected_digest.is_some()
        || record.base_ref.is_none()
        || record.base_oid.is_none()
        || record.operations.len() != 1
        || record.operations[0].family != repository::OperationFamily::Remote
    {
        return Err(recovery_required());
    }
    if record.intent_digest == poll_intent(record.request_id, &record.scope, true) {
        Ok(true)
    } else if record.intent_digest == poll_intent(record.request_id, &record.scope, false) {
        Ok(false)
    } else {
        Err(recovery_required())
    }
}
pub(super) fn materialization(
    db: &Connection,
    id: i64,
    c: &Candidate,
) -> Result<Option<(String, String, String)>, RepositoryError> {
    let rows:Vec<(String,String,String,String)>=db.prepare("SELECT kind,operation_ulid,target_oid,step FROM remote_poll_materializations WHERE repository_id=?1 AND item_id=?2").and_then(|mut q|q.query_map(params![id,c.item_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?.collect()).map_err(|_|recovery_required())?;
    match rows.as_slice() {
        [] => Ok(None),
        [(kind, op, oid, step)] if *kind == c.kind && step == "complete" => {
            let owned:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE repository_id=?1 AND operation_ulid=?2 AND action='poll' AND poll_kind='update')",params![id,op],|r|r.get(0)).map_err(|_|recovery_required())?;
            if !owned
                || !OperationId::parse(op).is_ok_and(|i| i.to_string() == *op)
                || !git2::Oid::from_str(oid).is_ok_and(|o| !o.is_zero() && o.to_string() == *oid)
            {
                return Err(recovery_required());
            }
            Ok(Some((op.clone(), oid.clone(), step.clone())))
        }
        _ => Err(recovery_required()),
    }
}
pub(super) fn unclaimed(
    db: &Connection,
    operation: &str,
    scope: request_scope::Scope,
) -> Result<bool, RepositoryError> {
    if !repository::recovery::has_table(db, "poll_retirement_targets")
        .map_err(|_| recovery_required())?
    {
        return Ok(false);
    }
    let yes:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM poll_retirement_targets t JOIN request_operations m ON m.request_ulid=t.child_request_ulid AND m.operation_ulid=t.child_operation_ulid WHERE t.child_operation_ulid=?1 AND t.child_request_ulid IS ?2 AND t.admission='accepted' AND NOT EXISTS(SELECT 1 FROM operation_records o WHERE o.operation_ulid=?1) AND NOT EXISTS(SELECT 1 FROM context_retirement_records c WHERE c.operation_ulid=?1) AND NOT EXISTS(SELECT 1 FROM poll_retirement_child_sources s WHERE s.child_operation_ulid=?1) AND NOT EXISTS(SELECT 1 FROM retirement_observation_receipts b WHERE b.consumer_operation_ulid=?1))",params![operation,scope.request_id()],|r|r.get(0)).map_err(|_|recovery_required())?;
    Ok(yes)
}
pub(in super::super) fn require_unreserved(
    db: &Connection,
    request: RequestId,
) -> Result<(), RepositoryError> {
    if !repository::recovery::has_table(db, "poll_retirement_targets")
        .map_err(|_| recovery_required())?
    {
        return Ok(());
    }
    let reserved: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM poll_retirement_targets WHERE child_request_ulid=?1)",
            [request.to_string()],
            |r| r.get(0),
        )
        .map_err(|_| recovery_required())?;
    if reserved {
        Err(recovery_required())
    } else {
        Ok(())
    }
}
pub(in super::super) fn ordinary_poll_recovery(
    db: &Connection,
    repository: i64,
    operation: &str,
    priority: repository::RemoteOperationPriority,
) -> Result<bool, RepositoryError> {
    if priority != repository::RemoteOperationPriority::Manual {
        return Ok(false);
    }
    if !repository::recovery::has_table(db, "poll_retirement_passes")
        .map_err(|_| recovery_required())?
    {
        return Ok(false);
    }
    let yes = db.query_row("SELECT EXISTS(SELECT 1 FROM poll_retirement_passes p JOIN repositories repo ON repo.id=p.repository_id JOIN request_records r ON r.request_ulid=p.request_ulid AND r.scope_key=repo.root_path JOIN request_operations m ON m.request_ulid=p.request_ulid AND m.operation_ulid=p.operation_ulid WHERE p.operation_ulid=?1 AND p.repository_id=?2 AND p.image IS NULL AND m.family='remote')",params![operation,repository],|r|r.get(0)).map_err(|_|recovery_required())?;
    if yes {
        audit_parents(db)?;
    }
    Ok(yes)
}
pub(in super::super) fn has_capture(
    service: &RepositoryService,
    root: &Path,
    op: OperationId,
) -> Result<bool, RepositoryError> {
    state::with_transaction(service, root, |tx, id| {
        record::audit_connection(tx)?;
        // Initial metadata is atomic with acceptance. Missing is corruption,
        // not evidence that this accepted invocation never began capture.
        tx.query_row("SELECT image IS NOT NULL FROM poll_retirement_passes WHERE operation_ulid=?1 AND repository_id=?2",params![op.to_string(),id],|r|r.get(0)).map_err(|_|recovery_required())
    })
}

pub(in super::super) fn has_cut(
    service: &RepositoryService,
    root: &Path,
    op: OperationId,
) -> Result<bool, RepositoryError> {
    state::with_transaction(service, root, |tx, id| {
        record::audit_connection(tx)?;
        tx.query_row("SELECT summary IS NOT NULL FROM poll_retirement_passes WHERE operation_ulid=?1 AND repository_id=?2",params![op.to_string(),id],|r|r.get(0)).map_err(|_|recovery_required())
    })
}

/// Wire completion also describes terminal failures. Before deriving Current
/// from an immutable cut, read the exact original UPDATE-P's durable outcome;
/// this read cannot refresh, restart P, reconstruct selection or change the cut.
pub(in super::super) fn original_poll_completed(
    service: &RepositoryService,
    accepted: &Accepted,
) -> Result<bool, RepositoryError> {
    state::with_transaction(service, &accepted.root(), |tx, id| {
        record::audit_connection(tx)?;
        validate_parent(tx, accepted, false)?;
        let original =
            state::read_operation(tx, id, accepted.operation_id)?.ok_or_else(recovery_required)?;
        if original.target.action() != repository::RemoteOperationAction::Poll
            || original.poll_kind != Some(state::PollKind::Update)
            || original.priority != repository::RemoteOperationPriority::Manual
        {
            return Err(recovery_required());
        }
        Ok(
            original.phase == repository::RemoteOperationPhase::Completed
                && original.outcome == Some(repository::RemoteOutcomeCategory::Completed),
        )
    })
}

pub(in super::super) fn is_parent(db: &Connection, ri: RequestId) -> Result<bool, RepositoryError> {
    db.query_row(
        "SELECT EXISTS(SELECT 1 FROM poll_retirement_passes WHERE request_ulid=?1)",
        [ri.to_string()],
        |r| r.get(0),
    )
    .map_err(|_| recovery_required())
}
pub(super) fn blocking_debt(db: &Connection, id: i64) -> Result<bool, RepositoryError> {
    let pending:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM operation_records WHERE repository_id=?1 AND (state!='completed' OR completed_step='comment_registration_pending')) OR EXISTS(SELECT 1 FROM remote_operation_records WHERE repository_id=?1 AND (phase IN ('reserved','advertising','persisting','fetch_prepared','fetch_observed','local_prepared','local_fast_forwarded','push_prepared','push_returned','push_verified','reconciling') OR index_pending!=0 OR reconciliation_required!=0))",[id],|r|r.get(0)).map_err(|_|recovery_required())?;
    Ok(pending || state::has_pending_conflict(db, id)?)
}
pub(in super::super) fn has_blocking_debt(
    service: &RepositoryService,
    root: &Path,
) -> Result<bool, RepositoryError> {
    state::with_transaction(service, root, |tx, id| blocking_debt(tx, id))
}
pub(in super::super) fn close_unselected(
    service: &RepositoryService,
    a: &Accepted,
) -> Result<(), RepositoryError> {
    state::with_transaction(service, &a.root(), |tx, id| {
        let r = state::read_operation(tx, id, a.operation_id)?.ok_or_else(recovery_required)?;
        if !matches!(
            r.phase,
            repository::RemoteOperationPhase::Interrupted
                | repository::RemoteOperationPhase::Failed
                | repository::RemoteOperationPhase::Cancelled
        ) {
            return Err(recovery_required());
        }
        super::source::close_stopped(tx, &r)
    })
}

pub(super) fn audit(db: &Connection) -> Result<(), RepositoryError> {
    // Historical catalogs precede the consumer/pass envelope.
    if !repository::recovery::has_table(db, "poll_retirement_passes")
        .map_err(|_| recovery_required())?
    {
        return Ok(());
    }
    audit_parents(db)?;
    let bad:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM poll_retirement_targets t LEFT JOIN poll_retirement_passes p USING(operation_ulid) LEFT JOIN request_records r ON r.request_ulid=t.child_request_ulid WHERE p.image IS NULL OR (t.admission IN ('accepted','selected','no_effect') AND (r.request_ulid IS NULL OR NOT EXISTS(SELECT 1 FROM request_operations m WHERE m.request_ulid=t.child_request_ulid AND m.family='local' AND m.operation_ulid=t.child_operation_ulid))) OR (t.admission IN ('reserved','refused') AND r.request_ulid IS NOT NULL)) OR EXISTS(SELECT 1 FROM poll_retirement_child_sources s LEFT JOIN poll_retirement_passes p ON p.operation_ulid=s.parent_operation_ulid LEFT JOIN poll_retirement_targets t ON t.operation_ulid=p.operation_ulid AND t.ordinal=s.ordinal LEFT JOIN context_retirement_records c ON c.operation_ulid=s.child_operation_ulid LEFT JOIN context_retirement_attempts a ON a.operation_ulid=c.operation_ulid AND a.owner_epoch=s.local_epoch LEFT JOIN retirement_observation_receipts b ON b.id=s.initial_receipt_id WHERE c.operation_ulid IS NULL OR c.repository_id!=s.repository_id OR c.request_ulid IS NOT s.child_request_ulid OR c.batch_operation_ulid IS NOT NULL OR a.batch_attempt_epoch IS NOT NULL OR a.observer_operation_ulid IS NOT NULL OR a.request_attempt IS NOT s.request_attempt OR s.local_epoch!=c.selection_epoch OR t.child_request_ulid IS NOT s.child_request_ulid OR t.child_operation_ulid IS NOT s.child_operation_ulid OR t.admission!='selected' OR t.candidate!=s.candidate OR p.image_digest!=s.image_digest OR p.request_ulid!=s.parent_request_ulid OR p.provenance_id!=s.provenance_id OR b.provenance_id!=s.provenance_id OR b.repository_id!=s.repository_id OR b.consumer_kind!='single' OR b.consumer_operation_ulid!=s.child_operation_ulid OR b.consumer_request_ulid!=s.child_request_ulid OR b.consumer_request_attempt!=s.request_attempt OR b.consumer_attempt_epoch!=s.local_epoch)",[],|r|r.get(0)).map_err(|_|recovery_required())?;
    if bad {
        return Err(recovery_required());
    }
    let mismatched:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM poll_retirement_child_sources s JOIN context_retirement_records c ON c.operation_ulid=s.child_operation_ulid JOIN poll_retirement_passes p ON p.operation_ulid=s.parent_operation_ulid WHERE c.configuration_generation IS NOT p.configuration_generation OR c.remote_name IS NOT p.remote_name OR c.primary_branch IS NOT p.primary_branch OR c.endpoint_digest IS NOT p.endpoint_digest OR c.initial_resources IS NOT json_extract(s.candidate,'$.resources')) OR EXISTS(SELECT 1 FROM poll_retirement_targets t WHERE t.admission='selected' AND NOT EXISTS(SELECT 1 FROM poll_retirement_child_sources s WHERE s.child_operation_ulid=t.child_operation_ulid))",[],|r|r.get(0)).map_err(|_|recovery_required())?;
    if mismatched {
        return Err(recovery_required());
    }
    let mut q=db.prepare("SELECT s.candidate,c.kind,c.item_id,c.expected_tip,c.instance_nonce,c.materialization_operation_ulid,c.materialization_oid,c.materialization_step FROM poll_retirement_child_sources s JOIN context_retirement_records c ON c.operation_ulid=s.child_operation_ulid").map_err(|_|recovery_required())?;
    for row in q
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, Option<String>>(7)?,
            ))
        })
        .map_err(|_| recovery_required())?
    {
        let (candidate, kind, item, tip, nonce, mop, moid, mstep) =
            row.map_err(|_| recovery_required())?;
        let c = Candidate::decode(&candidate)?;
        if c.kind != kind
            || c.item_id != item
            || c.tip != tip
            || c.nonce != nonce
            || c.materialization != mop.zip(moid).zip(mstep).map(|((a, b), c)| (a, b, c))
        {
            return Err(recovery_required());
        }
    }
    let missing:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM retirement_observation_receipts b JOIN retirement_advertisement_provenance p ON p.id=b.provenance_id WHERE b.consumer_kind='single' AND p.execution_kind='poll_parent' AND NOT EXISTS(SELECT 1 FROM poll_retirement_child_sources s WHERE s.initial_receipt_id=b.id)) OR EXISTS(SELECT 1 FROM poll_retirement_targets t JOIN context_retirement_records c ON c.operation_ulid=t.child_operation_ulid WHERE c.initial_resources IN ('present','branch_only') AND NOT EXISTS(SELECT 1 FROM poll_retirement_child_sources s WHERE s.child_operation_ulid=c.operation_ulid))",[],|r|r.get(0)).map_err(|_|recovery_required())?;
    if missing {
        return Err(recovery_required());
    }
    audit_summaries(db)?;
    Ok(())
}

fn audit_summaries(db: &Connection) -> Result<(), RepositoryError> {
    let missing:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM poll_retirement_passes p JOIN request_records r ON r.request_ulid=p.request_ulid WHERE p.image IS NOT NULL AND r.state='finished' AND p.summary IS NULL)",[],|r|r.get(0)).map_err(|_|recovery_required())?;
    if missing {
        return Err(recovery_required());
    }
    let mut q=db.prepare("SELECT p.operation_ulid,p.summary,r.result_data,r.code,p.image IS NULL AND EXISTS(SELECT 1 FROM remote_operation_records o WHERE o.repository_id=p.repository_id AND o.operation_ulid=p.operation_ulid AND o.phase='failed' AND o.outcome='host_approval_required') FROM poll_retirement_passes p JOIN request_records r ON r.request_ulid=p.request_ulid WHERE p.summary IS NOT NULL").map_err(|_|recovery_required())?;
    for row in q
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, bool>(4)?,
            ))
        })
        .map_err(|_| recovery_required())?
    {
        let (op, text, copy, code, host_failure) = row.map_err(|_| recovery_required())?;
        let value: serde_json::Value =
            serde_json::from_str(&text).map_err(|_| recovery_required())?;
        let report = mutation::PollRetirementMutationDto::from_stored(&value)
            .ok_or_else(recovery_required)?;
        if report.poll.operation_id != op
            || copy.as_ref().is_some_and(|s| {
                let Ok(copy) = serde_json::from_str::<serde_json::Value>(s) else {
                    return true;
                };
                copy != value && !host_stop_copy(&report, &copy, code.as_deref(), host_failure)
            })
        {
            return Err(recovery_required());
        }
        let targets:Vec<(String,Option<String>)>=db.prepare("SELECT candidate,child_operation_ulid FROM poll_retirement_targets WHERE operation_ulid=?1 ORDER BY ordinal").and_then(|mut q|q.query_map([&op],|r|Ok((r.get(0)?,r.get(1)?)))?.collect()).map_err(|_|recovery_required())?;
        if targets.len() != report.retirements.len() {
            return Err(recovery_required());
        }
        for ((json, ki), result) in targets.iter().zip(&report.retirements) {
            let c = Candidate::decode(json)?;
            if result.request_id != c.child_request
                || result.item_id != c.item_id
                || result.operation_id != *ki
                || result.kind.as_str() != c.kind
            {
                return Err(recovery_required());
            }
        }
    }
    Ok(())
}

/// Poll once already reports a live host stop using its closed synchronization
/// host root. Only an empty preselection failed pass may keep that public answer
/// alongside its internal stopped cut; no captured/child result gets an exception.
fn host_stop_copy(
    report: &mutation::PollRetirementMutationDto,
    copy: &serde_json::Value,
    code: Option<&str>,
    host_failure: bool,
) -> bool {
    let Some(host) = mutation::SynchronizationMutationDto::from_stored(copy) else {
        return false;
    };
    let replacement = report.stop_code == Some(crate::results::ResultCode::HostReplacementRequired);
    host_failure
        && report.completion == mutation::PollRetirementCompletion::Stopped
        && report.poll.completion == mutation::PollMutationCompletion::Completed
        && report.retirements.is_empty()
        && report.poll.targets.is_empty()
        && matches!(
            report.stop_code,
            Some(
                crate::results::ResultCode::HostApprovalRequired
                    | crate::results::ResultCode::HostReplacementRequired
            )
        )
        && code == report.stop_code.map(|c| c.as_str())
        && host.pinned_algorithm.is_some() == replacement
        && host.pinned_fingerprint.is_some() == replacement
        && serde_json::to_value(host).ok().as_ref() == Some(copy)
}

fn audit_parents(db: &Connection) -> Result<(), RepositoryError> {
    let mut q=db.prepare("SELECT p.operation_ulid,p.request_ulid,repo.root_path,r.intent_digest FROM poll_retirement_passes p JOIN repositories repo ON repo.id=p.repository_id JOIN request_records r ON r.request_ulid=p.request_ulid").map_err(|_|recovery_required())?;
    for row in q
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })
        .map_err(|_| recovery_required())?
    {
        let (op, ri, root, stored) = row.map_err(|_| recovery_required())?;
        let id = RequestId::parse(&ri).map_err(|_| recovery_required())?;
        let expected = poll_intent(id, &ScopeKey::from_stored(&root), true);
        if expected.to_string() != stored
            || id.to_string() != ri
            || !OperationId::parse(&op).is_ok_and(|id| id.to_string() == op)
        {
            return Err(recovery_required());
        }
        let valid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM poll_retirement_passes p JOIN request_records r ON r.request_ulid=p.request_ulid WHERE p.operation_ulid=?1 AND r.command='poll once' AND r.target='' AND r.scope_key=?2 AND r.attempt>=p.request_attempt AND r.confirmation_ulid IS NULL AND r.expected_digest IS NULL AND r.base_ref IS NOT NULL AND r.base_oid IS NOT NULL AND (p.primary_branch IS NULL OR r.base_ref='refs/heads/'||p.primary_branch) AND (SELECT count(*) FROM request_operations m WHERE m.request_ulid=r.request_ulid)=1 AND EXISTS(SELECT 1 FROM request_operations m WHERE m.request_ulid=r.request_ulid AND m.ordinal=0 AND m.family='remote' AND m.operation_ulid=p.operation_ulid) AND (SELECT count(*) FROM request_operations m WHERE m.operation_ulid=p.operation_ulid)=1)",params![op,root],|r|r.get(0)).map_err(|_|recovery_required())?;
        if !valid {
            return Err(recovery_required());
        }
    }
    Ok(())
}

pub(super) fn audit_initial_lineage(db: &Connection) -> Result<(), RepositoryError> {
    let rows:Vec<(String,i64)>=db.prepare("SELECT old.operation_ulid,old.owner_epoch FROM context_retirement_reentries rr JOIN context_retirement_attempts old ON old.operation_ulid=rr.operation_ulid AND old.owner_epoch=rr.previous_epoch WHERE old.observer_operation_ulid IS NULL AND NOT EXISTS(SELECT 1 FROM context_retirement_reentries previous WHERE previous.operation_ulid=old.operation_ulid AND previous.owner_epoch=old.owner_epoch AND previous.purpose='preserve')").and_then(|mut q|q.query_map([],|r|Ok((r.get(0)?,r.get(1)?)))?.collect()).map_err(|_|recovery_required())?;
    if rows.is_empty() {
        return Ok(());
    }
    if !repository::recovery::has_table(db, "poll_retirement_child_sources")
        .map_err(|_| recovery_required())?
    {
        return Err(recovery_required());
    }
    for (op, epoch) in rows {
        let valid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM poll_retirement_child_sources s JOIN retirement_observation_receipts b ON b.id=s.initial_receipt_id JOIN retirement_advertisement_provenance p ON p.id=b.provenance_id JOIN context_retirement_records c ON c.operation_ulid=s.child_operation_ulid WHERE s.child_operation_ulid=?1 AND s.local_epoch=?2 AND c.selection_epoch=?2 AND b.consumer_kind='single' AND b.consumer_operation_ulid=?1 AND b.consumer_attempt_epoch=?2 AND b.consumer_request_ulid=s.child_request_ulid AND b.consumer_request_attempt=s.request_attempt AND p.execution_kind='poll_parent')",params![op,epoch],|r|r.get(0)).map_err(|_|recovery_required())?;
        if !valid {
            return Err(recovery_required());
        }
    }
    Ok(())
}
