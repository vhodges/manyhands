//! Domain safe points only. Task 7 must bind the authoritative S/R/U F2 counters
//! and atomically settle all intended-parent associations, including unlinked U/R.
use super::{
    record,
    recover::{self, Owner},
    resolve,
    run::{Fault, Reason},
};
use crate::{repository::*, results::ResultCode};
use rusqlite::{TransactionBehavior, params};
thread_local! { pub(super) static ACK_HOOK: std::cell::RefCell<Option<Box<dyn FnMut()>>> = const {std::cell::RefCell::new(None)}; }

pub(super) fn issue(
    service: &RepositoryService,
    repo: &ResolvedRepository,
    id: OperationId,
    request: Option<OperationId>,
) -> Result<(), Fault> {
    let _cache = cache_write_guard(
        &service.registry_path,
        repo.root(),
        RepositoryOperation::ContextSyncLocal,
    )?;
    let mut db = record::open_for_write(service)?;
    let tx = db
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(ReadError::from)?;
    record::audit(&tx)?;
    let row=tx.query_row("SELECT s.* FROM local_context_sync_records s JOIN repositories r ON r.id=s.repository_id WHERE s.operation_ulid=?1 AND r.id=?2 AND r.root_path=?3",params![id.to_string(),repo.registration_id(),repo.root().to_str()],record::decode).map_err(ReadError::from)?;
    if matches!(
        row.state,
        record::LocalSyncState::Complete
            | record::LocalSyncState::Cancelled
            | record::LocalSyncState::Abandoned
    ) {
        return Err(Fault::foreign());
    }
    let changed=if let Some(request)=request {
        tx.execute("UPDATE local_context_sync_resolution_cancellation SET issued=issued+1 WHERE attempt_ulid=?1 AND attempt_ulid IN (SELECT attempt_ulid FROM local_context_sync_resolutions WHERE request_ulid=?2)",params![row.active_resolution.map(|id|id.to_string()),request.to_string()])
    } else {tx.execute("UPDATE local_context_sync_records SET cancel_issued=cancel_issued+1 WHERE operation_ulid=?1",[id.to_string()])}.map_err(ReadError::from)?;
    if changed != 1 {
        return Err(Fault::authority());
    }
    record::audit(&tx)?;
    tx.commit().map_err(ReadError::from)?;
    Ok(())
}
pub(super) fn safe_point(owner: &mut Owner<'_>) -> Result<(), Fault> {
    let (issued,ack,r_issued,r_ack)=owner.transaction(|db,row| {
        let (r_issued,r_ack)=if let Some(attempt)=row.active_resolution {db.query_row("SELECT issued,acknowledged FROM local_context_sync_resolution_cancellation WHERE attempt_ulid=?1",[attempt.to_string()],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?)))?} else {(0,0)};
        Ok((row.cancel_issued,row.cancel_acknowledged,r_issued,r_ack))
    })?;
    if issued == ack && r_issued == r_ack {
        return Ok(());
    }
    let evidence = owner.evidence()?;
    let terminal = owner.row.state == record::LocalSyncState::Prepared
        && owner.row.active_resolution.is_none()
        && !evidence.checkout_started
        && !evidence.commit_observed
        && !evidence.ref_started
        && !evidence.conflict_observed;
    ACK_HOOK.with(|h| {
        if let Some(h) = h.borrow_mut().as_mut() {
            h()
        }
    });
    owner.transaction(|db,row| {
        db.execute("UPDATE local_context_sync_records SET cancel_acknowledged=max(cancel_acknowledged,?2) WHERE operation_ulid=?1 AND cancel_issued>=?2",params![row.operation_id.to_string(),issued])?;
        if let Some(attempt)=row.active_resolution {db.execute("UPDATE local_context_sync_resolution_cancellation SET acknowledged=max(acknowledged,?2) WHERE attempt_ulid=?1 AND issued>=?2",params![attempt.to_string(),r_issued])?;}
        if terminal {
            db.execute("UPDATE local_context_sync_records SET state='cancelled',owner_epoch=owner_epoch+1,semantic_revision=semantic_revision+1 WHERE operation_ulid=?1",[row.operation_id.to_string()])?;
            db.execute("UPDATE operation_records SET state='completed',completed_step='completed' WHERE id=?1",[row.local_record_id])?;
        }
        Ok(())
    })?;
    Err(Fault {
        code: ResultCode::Cancelled,
        reason: Reason::Cancelled,
    })
}

/// Final domain completion and cancellation issuance serialize on this database
/// transaction. Observed counters are acknowledged without settling F2 requests.
pub(super) fn acknowledge_pending(
    db: &rusqlite::Transaction<'_>,
    row: &record::LocalContextSyncRecord,
) -> Result<bool, ReadError> {
    let (issued, acknowledged) = if let Some(attempt) = row.active_resolution {
        db.query_row("SELECT issued,acknowledged FROM local_context_sync_resolution_cancellation WHERE attempt_ulid=?1",[attempt.to_string()],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?)))?
    } else {
        (0, 0)
    };
    if issued == acknowledged && row.cancel_issued == row.cancel_acknowledged {
        return Ok(false);
    }
    db.execute(
        "UPDATE local_context_sync_records SET cancel_acknowledged=?2 WHERE operation_ulid=?1",
        params![row.operation_id.to_string(), row.cancel_issued],
    )?;
    if let Some(attempt) = row.active_resolution {
        db.execute("UPDATE local_context_sync_resolution_cancellation SET acknowledged=?2 WHERE attempt_ulid=?1",params![attempt.to_string(),issued])?;
    }
    Ok(true)
}
pub(super) fn abandon(
    service: &RepositoryService,
    repo: &ResolvedRepository,
    id: OperationId,
) -> Result<(), Fault> {
    let mut owner = resolve::owner(service, repo, id)?;
    let evidence = owner.evidence()?;
    let prepared = owner.row.state == record::LocalSyncState::Prepared
        && owner.row.active_resolution.is_none()
        && owner.row.candidate.is_none()
        && !evidence.checkout_started
        && !evidence.ref_started
        && !evidence.conflict_observed;
    let integrated = owner.row.state == record::LocalSyncState::IndexPending
        && owner.row.integration_proven
        && owner.row.artifacts_released;
    if !prepared && !integrated {
        return Err(Fault::authority());
    }
    recover::clean_metadata(&owner.git, &owner.row.context_ref)?;
    if prepared && !recover::owned_image(&owner, owner.row.context_parent)? {
        return Err(Fault::foreign());
    }
    if integrated {
        recover::verify_released_integration(&mut owner)?;
    }
    owner.transaction(|db,row| {
        // Until Task 7's all-associated atomic disposition exists, never delete
        // or bypass a real public request's authority using this internal seam.
        let associated:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM request_operations WHERE operation_ulid=?1)",[row.operation_id.to_string()],|r|r.get(0))?;
        if associated {return Err(ReadError::new(ResultCode::RecoveryRequired));}
        db.execute("UPDATE local_context_sync_resolutions SET state='abandoned' WHERE operation_ulid=?1",[row.operation_id.to_string()])?;
        db.execute("UPDATE local_context_sync_records SET state='abandoned',owner_epoch=owner_epoch+1,semantic_revision=semantic_revision+1 WHERE operation_ulid=?1",[row.operation_id.to_string()])?;
        db.execute("UPDATE operation_records SET state='completed',completed_step='abandoned' WHERE id=?1",[row.local_record_id])?;
        Ok(())
    })
}
