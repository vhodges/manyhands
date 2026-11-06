//! Sealed original-live handoff and one-use inputs to ORDINARY local consumers.
//! Durable eligible history is deliberately insufficient to build any of these.
use super::{
    pass::{self, Candidate},
    record,
    source::OriginalPollSource,
};
use crate::repository::{
    self, OperationId, RemotePollReport, RepositoryError, RepositoryService,
    mutation::{Accepted, RequestId},
    remote::{OriginalWrapper, observation::ObservationConfiguration, state},
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use state::recovery_required;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[cfg(test)]
thread_local! {static BEFORE_SETTLE:std::cell::RefCell<Option<Box<dyn FnOnce()>>>=const{std::cell::RefCell::new(None)};}
#[cfg(test)]
type EligibleHook = Box<dyn FnOnce(&RepositoryService)>;
#[cfg(test)]
type ChildHook = Box<dyn FnMut(usize)>;
#[cfg(test)]
thread_local! {
    static AFTER_ELIGIBLE: std::cell::RefCell<Option<EligibleHook>> = const {std::cell::RefCell::new(None)};
    static AFTER_CHILD: std::cell::RefCell<Option<ChildHook>> = const {std::cell::RefCell::new(None)};
}
#[cfg(test)]
pub(super) fn observe_eligible(f: impl FnOnce(&RepositoryService) + 'static) {
    AFTER_ELIGIBLE.with(|s| assert!(s.borrow_mut().replace(Box::new(f)).is_none()));
}
#[cfg(test)]
pub(super) fn observe_children(f: impl FnMut(usize) + 'static) {
    AFTER_CHILD.with(|s| assert!(s.borrow_mut().replace(Box::new(f)).is_none()));
}
#[cfg(test)]
pub(in super::super) fn child_checkpoint(ordinal: usize) {
    let hook = AFTER_CHILD.with(|s| s.borrow_mut().take());
    if let Some(mut hook) = hook {
        hook(ordinal);
        AFTER_CHILD.with(|s| *s.borrow_mut() = Some(hook));
    }
}
#[cfg(test)]
pub(super) fn observe_next_settle_for_testing(f: impl FnOnce() + 'static) {
    BEFORE_SETTLE.with(|s| assert!(s.borrow_mut().replace(Box::new(f)).is_none()));
}

pub(in super::super) struct Capture {
    evidence: OriginalCapture,
    inputs: Vec<(Candidate, bool)>,
}
struct OriginalCapture {
    scope: Arc<()>,
    accepted: Accepted,
    repository: i64,
    source_epoch: i64,
    generation: i64,
    configuration: ObservationConfiguration,
    plan: repository::RemoteRefPlan,
    root: PathBuf,
    registry: PathBuf,
    live_nonce: String,
    capture_nonce: String,
    digest: Vec<u8>,
    report: String,
    provenance: i64,
    binding: i64,
}
pub(in super::super) struct Live {
    evidence: Arc<OriginalCapture>,
    inputs: std::iter::Enumerate<std::vec::IntoIter<(Candidate, bool)>>,
}
/// Neither Clone, serializable, public nor reconstructible from receipt IDs.
pub(in super::super) struct InitialInput {
    live: Arc<OriginalCapture>,
    pub(super) ordinal: usize,
    pub(super) candidate: Candidate,
    branch_lifetime: bool,
}
impl Live {
    pub(in super::super) fn next(&mut self) -> Option<InitialInput> {
        self.inputs
            .next()
            .map(|(ordinal, (candidate, branch_lifetime))| InitialInput {
                live: Arc::clone(&self.evidence),
                ordinal,
                candidate,
                branch_lifetime,
            })
    }
}
pub(in super::super) fn report_image(report: &RemotePollReport) -> Result<String, RepositoryError> {
    let targets:Vec<_>=report.targets().iter().map(|r|serde_json::json!({"target":r.target().map(|t|t.remote_ref()),"tracking":r.target().map(|t|t.tracking_ref()),"kind":r.kind().map(|k|format!("{k:?}")),"item_id":r.item_id().map(ToString::to_string),"disposition":format!("{:?}",r.disposition()),"before_oid":r.before_oid().map(|o|o.to_string()),"after_oid":r.after_oid().map(|o|o.to_string())})).collect();
    Ok(serde_json::json!({"version":1,"operation_id":report.operation_id().to_string(),"completion":format!("{:?}",report.completion()),"completed_at":report.completed_at().unix_timestamp(),"next_eligible_at":report.next_eligible_at().map(|t|t.unix_timestamp()),"outcome":report.outcome().map(|v|format!("{v:?}")),"local_changes":report.local_changes(),"targets":targets}).to_string())
}
fn digest(operation: &str, image: &str, report: &str) -> Vec<u8> {
    let mut h = blake3::Hasher::new();
    for field in [
        "manyhands-live-retirement-pass-v1",
        operation,
        image,
        report,
    ] {
        h.update(&(field.len() as u64).to_le_bytes());
        h.update(field.as_bytes());
    }
    h.finalize().as_bytes().to_vec()
}
pub(super) fn freeze(
    service: &RepositoryService,
    tx: &Transaction<'_>,
    s: &OriginalPollSource,
    report: &RemotePollReport,
) -> Result<Capture, RepositoryError> {
    if report.operation_id() != s.accepted.operation_id
        || report.completion() != repository::PollCompletion::Completed
        || report.outcome() != Some(repository::RemoteOutcomeCategory::Completed)
    {
        return Err(recovery_required());
    }
    let rows:Vec<(usize,String,bool)>=tx.prepare("SELECT poll_ordinal,candidate,verified FROM poll_retirement_candidates WHERE operation_ulid=?1 ORDER BY poll_ordinal").and_then(|mut q|q.query_map([s.accepted.operation_id.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?.collect()).map_err(|_|recovery_required())?;
    let mut inputs = Vec::new();
    for (ordinal, json, verified) in rows {
        let c = Candidate::decode(&json)?;
        let row = report
            .targets()
            .get(ordinal)
            .ok_or_else(recovery_required)?;
        if !verified
            || c.poll_ordinal != ordinal
            || row.kind() != Some(c.kind()?)
            || row.item_id().map(ToString::to_string).as_deref() != Some(c.item_id.as_str())
            || !matches!(
                row.disposition(),
                repository::PollDisposition::Closed | repository::PollDisposition::Promoted
            )
            || (c.refusal.is_none() && row.after_oid().map(|o| o.to_string()) != c.tip)
        {
            return Err(recovery_required());
        }
        let branch = c.resources == "branch_only" && c.refusal.is_none();
        if branch && !s.take_branch(&c) {
            return Err(recovery_required());
        }
        inputs.push((c, branch));
    }
    if inputs.len()
        != report
            .targets()
            .iter()
            .filter(|r| {
                matches!(
                    r.disposition(),
                    repository::PollDisposition::Closed | repository::PollDisposition::Promoted
                )
            })
            .count()
    {
        return Err(recovery_required());
    }
    let image = pass::encode(&inputs.iter().map(|(c, _)| c).collect::<Vec<_>>())?;
    let report = report_image(report)?;
    let operation = s.accepted.operation_id.to_string();
    let digest = digest(&operation, &image, &report);
    let capture_nonce = OperationId::new().to_string();
    let (provenance,batch):(i64,i64)=tx.query_row("SELECT p.id,p.observation_batch_id FROM retirement_advertisement_provenance p JOIN poll_retirement_passes pass ON pass.provenance_id=p.id JOIN retirement_observation_pins pin ON pin.provenance_id=p.id AND pin.pin_kind='source_attempt' JOIN remote_observation_batches b ON b.id=pin.observation_batch_id WHERE pass.operation_ulid=?1 AND b.is_current=1 AND b.configuration_generation=?2",params![operation,s.generation],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|_|recovery_required())?;
    tx.execute("INSERT INTO retirement_observation_receipts(provenance_id,repository_id,consumer_kind,consumer_operation_ulid,consumer_request_ulid,consumer_request_attempt,consumer_attempt_epoch,configuration_generation,frozen_worklist_digest) VALUES(?1,?2,'batch',?3,?4,?5,1,?6,?7)",params![provenance,s.repository,operation,s.accepted.request_id.to_string(),s.accepted.attempt,s.generation,digest]).map_err(|_|recovery_required())?;
    let binding = tx.last_insert_rowid();
    tx.execute("INSERT INTO retirement_observation_pins(provenance_id,pin_kind,holder_operation_ulid,holder_epoch,repository_id,observation_batch_id,binding_id) VALUES(?1,'consumer_attempt',?2,1,?3,?4,?5)",params![provenance,operation,s.repository,batch,binding]).map_err(|_|recovery_required())?;
    tx.execute("UPDATE poll_retirement_passes SET image=?2,original_report=?3,image_digest=?4,capture_nonce=?5,binding_id=?6 WHERE operation_ulid=?1 AND image IS NULL AND live=1",params![operation,image,report,digest,capture_nonce,binding]).map_err(|_|recovery_required())?;
    for (ordinal, (c, _)) in inputs.iter().enumerate() {
        tx.execute("INSERT INTO poll_retirement_targets(operation_ulid,ordinal,child_request_ulid,candidate,admission) VALUES(?1,?2,?3,?4,'reserved')",params![operation,ordinal,c.child_request,pass::encode(c)?]).map_err(|_|recovery_required())?;
    }
    Ok(Capture {
        evidence: OriginalCapture {
            scope: Arc::clone(&s.scope),
            accepted: Accepted {
                request_id: s.accepted.request_id,
                attempt: s.accepted.attempt,
                scope: s.accepted.scope.clone(),
                family: s.accepted.family,
                operation_id: s.accepted.operation_id,
                position: s.accepted.position.clone(),
                expected: s.accepted.expected.clone(),
            },
            repository: s.repository,
            source_epoch: s.epoch,
            generation: s.generation,
            configuration: s.configuration.clone(),
            plan: s.plan.clone(),
            root: s.root.clone(),
            registry: service.registry_path.clone(),
            live_nonce: s.nonce.clone(),
            capture_nonce,
            digest,
            report,
            provenance,
            binding,
        },
        inputs,
    })
}
impl OriginalCapture {
    fn validate(
        &self,
        service: &RepositoryService,
        tx: &Transaction<'_>,
        id: i64,
        eligible: bool,
    ) -> Result<(), RepositoryError> {
        if id != self.repository
            || !Arc::ptr_eq(&self.scope, &service.remote_reservation_scope)
            || service.registry_path != self.registry
            || pass::validate_parent(tx, &self.accepted, true)?
            || state::generation(tx, id)? != self.generation
            || !self
                .configuration
                .matches_owned(service, tx, &self.root, &self.plan)?
        {
            return Err(recovery_required());
        }
        let valid:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM poll_retirement_passes p JOIN retirement_advertisement_provenance source ON source.id=p.provenance_id JOIN retirement_observation_receipts binding ON binding.id=p.binding_id JOIN retirement_observation_pins pin ON pin.binding_id=binding.id AND pin.pin_kind='consumer_attempt' JOIN remote_observation_batches b ON b.id=pin.observation_batch_id JOIN remote_operation_records o ON o.repository_id=p.repository_id AND o.operation_ulid=p.operation_ulid WHERE p.operation_ulid=?1 AND p.live_nonce=?2 AND p.capture_nonce=?3 AND p.image_digest=?4 AND p.original_report=?5 AND p.provenance_id=?6 AND p.binding_id=?7 AND p.handoff=?8 AND b.is_current=1 AND b.configuration_generation=?9 AND o.owner_epoch=?10 AND o.phase='completed' AND o.outcome='completed' AND o.index_pending=0 AND o.reconciliation_required=0)",params![self.accepted.operation_id.to_string(),self.live_nonce,self.capture_nonce,self.digest,self.report,self.provenance,self.binding,if eligible{"eligible"}else{"unconfirmed"},self.generation,self.source_epoch],|r|r.get(0)).map_err(|_|recovery_required())?;
        if !valid {
            return Err(recovery_required());
        }
        Ok(())
    }
}
impl Capture {
    pub(in super::super) fn settle(
        self,
        service: &RepositoryService,
        root: &Path,
        wrapper: OriginalWrapper,
    ) -> Result<Option<Live>, RepositoryError> {
        if root != self.evidence.root || wrapper.report() != self.evidence.report {
            return Err(recovery_required());
        }
        #[cfg(test)]
        if let Some(hook) = BEFORE_SETTLE.with(|s| s.borrow_mut().take()) {
            hook();
        }
        let eligible = state::with_transaction(service, root, |tx, id| {
            let current = self.evidence.validate(service, tx, id, false);
            let eligible = current.is_ok() && wrapper.complete();
            // Losing the owned current wrapper/config/cancellation race is a
            // conservative stop, never a later repair's retirement authority.
            tx.execute("UPDATE poll_retirement_passes SET handoff=?2,live=CASE WHEN ?3 THEN live ELSE 0 END WHERE operation_ulid=?1 AND handoff='unconfirmed' AND summary IS NULL",params![self.evidence.accepted.operation_id.to_string(),if eligible{"eligible"}else{"ineligible"},eligible]).map_err(|_|recovery_required())?;
            record::release_bound_source(
                tx,
                self.evidence.provenance,
                self.evidence.binding,
                !eligible,
            )?;
            record::audit_connection(tx)?;
            Ok(eligible)
        })?;
        #[cfg(test)]
        if eligible && let Some(hook) = AFTER_ELIGIBLE.with(|s| s.borrow_mut().take()) {
            hook(service);
        }
        Ok(eligible.then(|| Live {
            evidence: Arc::new(self.evidence),
            inputs: self.inputs.into_iter().enumerate(),
        }))
    }
}
impl InitialInput {
    pub(in super::super) fn identity(
        &self,
    ) -> Result<(repository::AuthoringKind, crate::canonical::ItemId), RepositoryError> {
        Ok((
            self.candidate.kind()?,
            self.candidate
                .item_id
                .parse()
                .map_err(|_| recovery_required())?,
        ))
    }
    pub(in super::super) fn request_id(&self) -> Result<RequestId, RepositoryError> {
        RequestId::parse(&self.candidate.child_request).map_err(|_| recovery_required())
    }
    pub(in super::super) fn root(&self) -> &Path {
        &self.live.root
    }
    pub(super) fn plan(&self) -> &repository::RemoteRefPlan {
        &self.live.plan
    }
    pub(super) fn configuration(&self) -> &ObservationConfiguration {
        &self.live.configuration
    }
    pub(super) fn generation(&self) -> i64 {
        self.live.generation
    }
    pub(super) fn validate(
        &self,
        service: &RepositoryService,
        tx: &Transaction<'_>,
        id: i64,
    ) -> Result<(), RepositoryError> {
        self.live.validate(service, tx, id, true)?;
        if state::has_pending_conflict(tx, id)? {
            return Err(recovery_required());
        }
        if self.candidate.resources == "branch_only" && !self.branch_lifetime {
            return Err(recovery_required());
        }
        let matches:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM poll_retirement_targets WHERE operation_ulid=?1 AND ordinal=?2 AND child_request_ulid=?3 AND candidate=?4 AND admission IN ('reserved','accepted','selected'))",params![self.live.accepted.operation_id.to_string(),self.ordinal,self.candidate.child_request,pass::encode(&self.candidate)?],|r|r.get(0)).map_err(|_|recovery_required())?;
        if !matches {
            return Err(recovery_required());
        }
        Ok(())
    }
    pub(in super::super) fn admit(
        &self,
        service: &RepositoryService,
        tx: &Transaction<'_>,
        a: &Accepted,
    ) -> Result<(), RepositoryError> {
        self.validate(service, tx, self.live.repository)?;
        if pass::blocking_debt(tx, self.live.repository)? {
            return Err(recovery_required());
        }
        if a.request_id != self.request_id()?
            || a.family != Some(repository::OperationFamily::Local)
            || a.attempt != 1
        {
            return Err(recovery_required());
        }
        // Insertion and this association share the ordinary immediate admission
        // transaction, under the common Git lease. No fabricated acceptance.
        if tx.execute("UPDATE poll_retirement_targets SET admission='accepted',child_operation_ulid=?4 WHERE operation_ulid=?1 AND ordinal=?2 AND child_request_ulid=?3 AND admission='reserved'",params![self.live.accepted.operation_id.to_string(),self.ordinal,a.request_id.to_string(),a.operation_id.to_string()]).map_err(|_|recovery_required())?!=1{return Err(recovery_required());}
        Ok(())
    }
    pub(super) fn bind(
        &self,
        tx: &Transaction<'_>,
        request: &super::RetireContextRequest,
        scope: super::request_scope::Scope,
    ) -> Result<(), RepositoryError> {
        if scope.request_id().as_deref() != Some(self.candidate.child_request.as_str()) {
            return Err(recovery_required());
        }
        tx.execute("INSERT INTO retirement_observation_receipts(provenance_id,repository_id,consumer_kind,consumer_operation_ulid,consumer_request_ulid,consumer_request_attempt,consumer_attempt_epoch,configuration_generation) VALUES(?1,?2,'single',?3,?4,?5,1,?6)",params![self.live.provenance,self.live.repository,request.operation_id.to_string(),scope.request_id(),scope.attempt(),self.live.generation]).map_err(|_|recovery_required())?;
        let receipt = tx.last_insert_rowid();
        let batch: i64 = tx
            .query_row(
                "SELECT observation_batch_id FROM retirement_advertisement_provenance WHERE id=?1",
                [self.live.provenance],
                |r| r.get(0),
            )
            .map_err(|_| recovery_required())?;
        tx.execute("INSERT INTO retirement_observation_pins(provenance_id,pin_kind,holder_operation_ulid,holder_epoch,repository_id,observation_batch_id,binding_id) VALUES(?1,'consumer_attempt',?2,1,?3,?4,?5)",params![self.live.provenance,request.operation_id.to_string(),self.live.repository,batch,receipt]).map_err(|_|recovery_required())?;
        tx.execute("INSERT INTO poll_retirement_child_sources(repository_id,child_operation_ulid,child_request_ulid,parent_request_ulid,parent_operation_ulid,ordinal,image_digest,candidate,provenance_id,initial_receipt_id,local_epoch,request_attempt) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,1,?11)",params![self.live.repository,request.operation_id.to_string(),scope.request_id(),self.live.accepted.request_id.to_string(),self.live.accepted.operation_id.to_string(),self.ordinal,self.live.digest,pass::encode(&self.candidate)?,self.live.provenance,receipt,scope.attempt()]).map_err(|_|recovery_required())?;
        if tx.execute("UPDATE poll_retirement_targets SET admission='selected' WHERE operation_ulid=?1 AND ordinal=?2 AND child_operation_ulid=?3 AND admission='accepted'",params![self.live.accepted.operation_id.to_string(),self.ordinal,request.operation_id.to_string()]).map_err(|_|recovery_required())?!=1{return Err(recovery_required());}
        Ok(())
    }
    pub(super) fn receipt(
        &self,
        tx: &Transaction<'_>,
        operation: OperationId,
    ) -> Result<i64, RepositoryError> {
        tx.query_row("SELECT s.initial_receipt_id FROM poll_retirement_child_sources s JOIN retirement_observation_pins pin ON pin.binding_id=s.initial_receipt_id JOIN remote_observation_batches b ON b.id=pin.observation_batch_id WHERE s.child_operation_ulid=?1 AND s.child_request_ulid=?2 AND s.parent_operation_ulid=?3 AND s.ordinal=?4 AND s.image_digest=?5 AND s.provenance_id=?6 AND pin.holder_operation_ulid=?1 AND pin.holder_epoch=1 AND b.is_current=1 AND b.configuration_generation=?7",params![operation.to_string(),self.candidate.child_request,self.live.accepted.operation_id.to_string(),self.ordinal,self.live.digest,self.live.provenance,self.live.generation],|r|r.get(0)).map_err(|_|recovery_required())
    }
}

pub(super) fn reserved_child(db: &Connection, id: &str) -> Result<bool, RepositoryError> {
    if !repository::recovery::has_table(db, "poll_retirement_targets")
        .map_err(|_| recovery_required())?
    {
        return Ok(false);
    }
    db.query_row(
        "SELECT EXISTS(SELECT 1 FROM poll_retirement_targets WHERE child_operation_ulid=?1)",
        [id],
        |r| r.get(0),
    )
    .map_err(|_| recovery_required())
}
pub(super) fn require_release(
    db: &Connection,
    provenance: i64,
    binding: i64,
    terminal: bool,
) -> Result<(), RepositoryError> {
    let valid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM poll_retirement_passes p JOIN remote_operation_records o USING(operation_ulid) WHERE p.provenance_id=?1 AND p.binding_id=?2 AND p.handoff!='unconfirmed' AND (NOT ?3 OR p.live=0) AND o.phase='completed' AND o.owner_epoch=p.source_owner_epoch)",params![provenance,binding,terminal],|r|r.get(0)).map_err(|_|recovery_required())?;
    if valid {
        Ok(())
    } else {
        Err(recovery_required())
    }
}
pub(super) fn audit(db: &Connection) -> Result<(), RepositoryError> {
    if !repository::recovery::has_table(db, "poll_retirement_passes")
        .map_err(|_| recovery_required())?
    {
        return Ok(());
    }
    let mut q=db.prepare("SELECT operation_ulid,image,original_report,image_digest FROM poll_retirement_passes WHERE image IS NOT NULL").map_err(|_|recovery_required())?;
    for row in q
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Vec<u8>>(3)?,
            ))
        })
        .map_err(|_| recovery_required())?
    {
        let (op, image, report, stored) = row.map_err(|_| recovery_required())?;
        if stored != digest(&op, &image, &report) {
            return Err(recovery_required());
        }
        let (remote,primary):(String,String)=db.query_row("SELECT remote_name,primary_branch FROM poll_retirement_passes WHERE operation_ulid=?1",[&op],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|_|recovery_required())?;
        let plan = repository::RemoteRefPlan::from_configuration(&remote, &primary)
            .map_err(|_| recovery_required())?;
        let decoded = RemotePollReport::decode_frozen(&report, &plan)?;
        if decoded.operation_id().to_string() != op
            || decoded.completion() != repository::PollCompletion::Completed
            || decoded.outcome() != Some(repository::RemoteOutcomeCategory::Completed)
        {
            return Err(recovery_required());
        }
        let terminal:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM poll_retirement_passes p JOIN remote_operation_records o ON o.repository_id=p.repository_id AND o.operation_ulid=p.operation_ulid WHERE p.operation_ulid=?1 AND o.phase='completed' AND o.outcome='completed' AND o.owner_epoch=p.source_owner_epoch AND o.configuration_generation=p.configuration_generation AND o.remote_name=p.remote_name AND o.primary_branch=p.primary_branch AND o.poll_completed_at=?2)",params![op,decoded.completed_at().unix_timestamp()],|r|r.get(0)).map_err(|_|recovery_required())?;
        if !terminal {
            return Err(recovery_required());
        }
        let cs: Vec<Candidate> = serde_json::from_str(&image).map_err(|_| recovery_required())?;
        let rows:Vec<(String,String)>=db.prepare("SELECT child_request_ulid,candidate FROM poll_retirement_targets WHERE operation_ulid=?1 ORDER BY ordinal").and_then(|mut q|q.query_map([&op],|r|Ok((r.get(0)?,r.get(1)?)))?.collect()).map_err(|_|recovery_required())?;
        if rows.len() != cs.len() || pass::encode(&cs)? != image {
            return Err(recovery_required());
        }
        for (c, (ri, json)) in cs.iter().zip(rows) {
            if c.child_request != ri || pass::encode(c)? != json {
                return Err(recovery_required());
            }
            Candidate::decode(&json)?;
            let r = decoded
                .targets()
                .get(c.poll_ordinal)
                .ok_or_else(recovery_required)?;
            if r.kind() != Some(c.kind()?)
                || r.item_id().map(ToString::to_string).as_deref() != Some(c.item_id.as_str())
                || !matches!(
                    r.disposition(),
                    repository::PollDisposition::Closed | repository::PollDisposition::Promoted
                )
                || (c.refusal.is_none() && r.after_oid().map(|o| o.to_string()) != c.tip)
            {
                return Err(recovery_required());
            }
        }
        let (provenance,binding,source_epoch,live,handoff):(i64,i64,i64,bool,String)=db.query_row("SELECT provenance_id,binding_id,source_owner_epoch,live,handoff FROM poll_retirement_passes WHERE operation_ulid=?1",[&op],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).map_err(|_|recovery_required())?;
        audit_holder(
            db,
            provenance,
            "source_attempt",
            (&op, source_epoch, None),
            handoff == "unconfirmed",
            "handoff_decided",
        )?;
        audit_holder(
            db,
            provenance,
            "consumer_attempt",
            (&op, 1, Some(binding)),
            live,
            "consumer_terminal",
        )?;
    }
    // Exact pass holders only. K1's receipt/release must not alter this count.
    let bad:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM poll_retirement_passes pass JOIN retirement_advertisement_provenance p ON p.id=pass.provenance_id WHERE p.execution_kind!='poll_parent' OR p.repository_id!=pass.repository_id OR p.execution_request_ulid!=pass.request_ulid OR p.execution_request_attempt!=pass.request_attempt OR p.source_operation_ulid!=pass.operation_ulid OR p.source_owner_epoch!=pass.source_owner_epoch OR (pass.image IS NOT NULL AND NOT EXISTS(SELECT 1 FROM retirement_observation_receipts b WHERE b.id=pass.binding_id AND b.provenance_id=p.id AND b.consumer_kind='batch' AND b.consumer_operation_ulid=pass.operation_ulid AND b.consumer_attempt_epoch=1 AND b.frozen_worklist_digest=pass.image_digest)) OR (pass.image IS NOT NULL AND ((pass.handoff='unconfirmed' AND NOT EXISTS(SELECT 1 FROM retirement_observation_pins pin WHERE pin.provenance_id=p.id AND pin.pin_kind='source_attempt' AND pin.holder_operation_ulid=pass.operation_ulid AND pin.holder_epoch=p.source_owner_epoch)) OR (pass.live=1 AND NOT EXISTS(SELECT 1 FROM retirement_observation_pins pin WHERE pin.binding_id=pass.binding_id)) OR (pass.live=0 AND NOT EXISTS(SELECT 1 FROM retirement_observation_pin_releases r WHERE r.binding_id=pass.binding_id AND r.reason='consumer_terminal')))))",[],|r|r.get(0)).map_err(|_|recovery_required())?;
    if bad {
        return Err(recovery_required());
    }
    Ok(())
}
fn audit_holder(
    db: &Connection,
    provenance: i64,
    kind: &str,
    holder: (&str, i64, Option<i64>),
    wanted: bool,
    reason: &str,
) -> Result<(), RepositoryError> {
    let (operation, epoch, binding) = holder;
    let (pin,released):(bool,bool)=db.query_row("SELECT EXISTS(SELECT 1 FROM retirement_observation_pins WHERE provenance_id=?1 AND pin_kind=?2 AND holder_operation_ulid=?3 AND holder_epoch=?4 AND binding_id IS ?5),EXISTS(SELECT 1 FROM retirement_observation_pin_releases WHERE provenance_id=?1 AND pin_kind=?2 AND holder_operation_ulid=?3 AND holder_epoch=?4 AND binding_id IS ?5 AND reason=?6)",params![provenance,kind,operation,epoch,binding,reason],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|_|recovery_required())?;
    if pin != wanted || released == wanted {
        return Err(recovery_required());
    }
    Ok(())
}
pub(in super::super) fn recover_unconfirmed(
    service: &RepositoryService,
    root: &Path,
    op: OperationId,
) -> Result<(), RepositoryError> {
    state::with_transaction(service, root, |tx, _| {
        let row:Option<(i64,i64)>=tx.query_row("SELECT provenance_id,binding_id FROM poll_retirement_passes WHERE operation_ulid=?1 AND image IS NOT NULL AND handoff='unconfirmed'",[op.to_string()],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|_|recovery_required())?;
        if let Some((p, b)) = row {
            tx.execute("UPDATE poll_retirement_passes SET handoff='ineligible',live=0 WHERE operation_ulid=?1",[op.to_string()]).map_err(|_|recovery_required())?;
            record::release_bound_source(tx, p, b, true)?;
        }
        Ok(())
    })
}
pub(in super::super) fn replay_report(
    service: &RepositoryService,
    root: &Path,
    op: OperationId,
) -> Result<Option<(RemotePollReport, bool)>, RepositoryError> {
    state::with_transaction(service, root, |tx, id| {
        let row:Option<(String,String,String)>=tx.query_row("SELECT original_report,remote_name,primary_branch FROM poll_retirement_passes WHERE operation_ulid=?1 AND image IS NOT NULL",[op.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|_|recovery_required())?;
        let Some((json, remote, primary)) = row else {
            return Ok(None);
        };
        let plan = repository::RemoteRefPlan::from_configuration(&remote, &primary)
            .map_err(|_| recovery_required())?;
        let report = RemotePollReport::decode_frozen(&json, &plan)?;
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
