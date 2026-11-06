//! Private current-F2 seam shared by ordinary retirement and its binding.
use super::*;
use crate::repository::{
    AuthoringKind, OperationFamily, OperationId, RepositoryError, RepositoryService,
    authoring_kind_segment, keys::*, mutation, remote::state, transport::*,
};
use mutation::{
    RequestId,
    request_store::{DigestSalt, FieldValue, IntentDigest, ScopeKey},
};
use rusqlite::{Connection, OptionalExtension, params};
use state::recovery_required;

/// Sealed current-request snapshot, never constructed from caller IDs alone.
/// Copying it inside a live executor/index owner does not extend its validity:
/// every owning transaction rechecks the actual current accepted request.
#[derive(Clone, Copy, Debug)]
pub(in super::super) struct RequestFence {
    request: RequestId,
    attempt: i64,
    operation: OperationId,
    kind: AuthoringKind,
    item: Option<[u8; 26]>,
    digest: IntentDigest,
    preserving: bool,
}
#[derive(Clone, Copy, Debug)]
pub(in super::super) enum Scope {
    Domain,
    Request(RequestFence),
}

fn digest(
    id: RequestId,
    root: &std::path::Path,
    kind: AuthoringKind,
    item: &str,
) -> Result<IntentDigest, RepositoryError> {
    let scope = ScopeKey::from_stored(root.to_str().ok_or_else(recovery_required)?);
    Ok(
        IntentDigest::builder(DigestSalt::Request(id), "context retire", &scope, item)
            .field("kind", FieldValue::Text(authoring_kind_segment(&kind)))
            .finish(),
    )
}
impl RequestFence {
    pub(super) fn preserving(
        accepted: &mutation::Accepted,
        operation: OperationId,
    ) -> Result<Self, RepositoryError> {
        if accepted.family.is_some()
            || accepted.position.base_ref.is_some()
            || accepted.position.base_oid.is_some()
            || accepted.expected.is_some()
        {
            return Err(recovery_required());
        }
        Ok(Self {
            request: accepted.request_id,
            attempt: i64::try_from(accepted.attempt).map_err(|_| recovery_required())?,
            operation,
            kind: AuthoringKind::Ticket,
            item: None,
            digest: IntentDigest::builder(
                DigestSalt::Request(accepted.request_id),
                "operation abandon",
                &accepted.scope,
                &operation.to_string(),
            )
            .finish(),
            preserving: true,
        })
    }
    pub(super) fn is_preserving(self) -> bool {
        self.preserving
    }
    pub(super) fn is_preserving_for(self, operation: &str) -> bool {
        self.preserving && self.operation.to_string() == operation
    }
    pub(super) fn request(self) -> RequestId {
        self.request
    }
    fn item(&self) -> &str {
        std::str::from_utf8(self.item.as_ref().expect("ordinary retirement fence"))
            .expect("canonical ItemId bytes")
    }
    pub(super) fn id(&self) -> String {
        self.request.to_string()
    }
    pub(super) fn attempt(&self) -> i64 {
        self.attempt
    }
    pub(in super::super) fn validate(
        &self,
        db: &Connection,
        operation: &str,
        root: &std::path::Path,
    ) -> Result<bool, RepositoryError> {
        if self.preserving
            || operation != self.operation.to_string()
            || digest(self.request, root, self.kind, self.item())? != self.digest
        {
            return Err(recovery_required());
        }
        let cancelled:Option<bool>=db.query_row("SELECT r.cancel_requested FROM request_records r WHERE r.request_ulid=?1 AND r.attempt=?2 AND r.state='accepted' AND r.scope_key=?3 AND r.command='context retire' AND r.target=?4 AND r.intent_digest=?5 AND r.confirmation_ulid IS NULL AND r.base_ref IS NULL AND r.base_oid IS NULL AND r.expected_digest IS NULL AND EXISTS(SELECT 1 FROM request_operations m WHERE m.request_ulid=r.request_ulid AND m.ordinal=0 AND m.family='local' AND m.operation_ulid=?6) AND (SELECT count(*) FROM request_operations m WHERE m.request_ulid=r.request_ulid)=1 AND (SELECT count(*) FROM request_operations m WHERE m.operation_ulid=?6)=1", params![self.id(),self.attempt,root.to_str(),self.item(),self.digest.to_string(),operation], |r|r.get(0)).optional().map_err(|_|recovery_required())?;
        cancelled.ok_or_else(recovery_required)
    }
}
impl Scope {
    fn expected(
        request: &RetireContextRequest,
        accepted: &mutation::Accepted,
    ) -> Result<RequestFence, RepositoryError> {
        if accepted.attempt == 0
            || accepted.operation_id != request.operation_id
            || accepted.family != Some(OperationFamily::Local)
            || accepted.scope.repository() != request.root.to_str()
            || accepted.expected.is_some()
            || accepted.position.base_ref.is_some()
            || accepted.position.base_oid.is_some()
        {
            return Err(recovery_required());
        }
        let fence = RequestFence {
            request: accepted.request_id,
            attempt: i64::try_from(accepted.attempt).map_err(|_| recovery_required())?,
            operation: request.operation_id,
            kind: request.kind,
            item: Some(
                request
                    .item_id
                    .to_string()
                    .as_bytes()
                    .try_into()
                    .map_err(|_| recovery_required())?,
            ),
            digest: digest(
                accepted.request_id,
                &request.root,
                request.kind,
                &request.item_id.to_string(),
            )?,
            preserving: false,
        };
        Ok(fence)
    }
    pub(super) fn capture(
        service: &RepositoryService,
        request: &RetireContextRequest,
        accepted: &mutation::Accepted,
    ) -> Result<Self, RepositoryError> {
        let fence = Self::expected(request, accepted)?;
        state::with_transaction(service, &request.root, |tx, _| {
            fence.validate(tx, &request.operation_id.to_string(), &request.root)?;
            Ok(())
        })?;
        Ok(Self::Request(fence))
    }
    fn finished(
        service: &RepositoryService,
        request: &RetireContextRequest,
        accepted: &mutation::Accepted,
    ) -> Result<Option<ContextRetirementOutcome>, RepositoryError> {
        let f = Self::expected(request, accepted)?;
        state::with_transaction(service, &request.root, |tx, _| {
            // A separate read-only result path, not a permissive ownership flag.
            let result:Option<String>=tx.query_row("SELECT c.final_result FROM request_records r JOIN context_retirement_records c ON c.request_ulid=r.request_ulid JOIN operation_records o ON o.id=c.local_record_id WHERE r.request_ulid=?1 AND r.state='finished' AND ?2<=r.attempt AND r.scope_key=?3 AND r.command='context retire' AND r.target=?4 AND r.intent_digest=?5 AND r.confirmation_ulid IS NULL AND r.base_ref IS NULL AND r.base_oid IS NULL AND r.expected_digest IS NULL AND c.operation_ulid=?6 AND c.kind=?7 AND c.item_id=?4 AND o.root_path=?3 AND c.phase IN ('complete','refused') AND c.final_result IS NOT NULL AND EXISTS(SELECT 1 FROM request_operations m WHERE m.request_ulid=r.request_ulid AND m.ordinal=0 AND m.family='local' AND m.operation_ulid=c.operation_ulid) AND (SELECT count(*) FROM request_operations m WHERE m.request_ulid=r.request_ulid)=1 AND (SELECT count(*) FROM request_operations m WHERE m.operation_ulid=c.operation_ulid)=1",params![f.id(),f.attempt,request.root.to_str(),f.item(),f.digest.to_string(),f.operation.to_string(),authoring_kind_segment(&f.kind)],|r|r.get(0)).optional().map_err(|_|recovery_required())?;
            if let Some(result) = result {
                record::audit_connection(tx)?;
                Ok(Some(ContextRetirementOutcome::Final(
                    ContextRetirementReport::decode(&result)?,
                )))
            } else {
                Ok(None)
            }
        })
    }
    pub(super) fn fence(self) -> Option<RequestFence> {
        match self {
            Self::Domain => None,
            Self::Request(f) => Some(f),
        }
    }
    pub(super) fn request_id(self) -> Option<String> {
        self.fence().map(|f| f.id())
    }
    pub(super) fn attempt(self) -> Option<i64> {
        self.fence().map(|f| f.attempt)
    }
    pub(super) fn validate(
        self,
        db: &Connection,
        operation: &str,
        root: &std::path::Path,
    ) -> Result<bool, RepositoryError> {
        let bound:Option<(Option<String>,Option<String>)>=db.query_row("SELECT request_ulid,batch_operation_ulid FROM context_retirement_records WHERE operation_ulid=?1",[operation],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|_|recovery_required())?;
        match self {
            Self::Domain => {
                let mapped: bool = db
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM request_operations WHERE operation_ulid=?1)",
                        [operation],
                        |r| r.get(0),
                    )
                    .map_err(|_| recovery_required())?;
                if mapped || bound.is_some_and(|(r, b)| r.is_some() || b.is_some()) {
                    return Err(recovery_required());
                }
                Ok(false)
            }
            Self::Request(f) => {
                if bound.is_some_and(|(r, b)| r.as_deref() != Some(f.id().as_str()) || b.is_some())
                {
                    return Err(recovery_required());
                }
                f.validate(db, operation, root)
            }
        }
    }
    pub(super) fn effect(
        self,
        db: &Connection,
        operation: &str,
        root: &std::path::Path,
    ) -> Result<(), RepositoryError> {
        if self.validate(db, operation, root)? {
            return Err(recovery_required());
        }
        Ok(())
    }
}

/// Historical attempts may be older than F2, including after a request finishes.
/// This validates retained correlation only; it mints no current execution scope.
/// Pass R maps remote P, not child Ki; `pass` audits that separate history.
pub(super) fn audit(db: &Connection) -> Result<(), RepositoryError> {
    let mut q=db.prepare("SELECT c.operation_ulid,o.root_path,c.kind,c.item_id,c.request_ulid,r.attempt,r.scope_key,r.command,r.target,r.intent_digest FROM context_retirement_records c JOIN operation_records o ON o.id=c.local_record_id JOIN request_records r ON r.request_ulid=c.request_ulid WHERE c.request_ulid IS NOT NULL AND c.batch_operation_ulid IS NULL").map_err(|_|recovery_required())?;
    let rows = q
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, String>(7)?,
                r.get::<_, String>(8)?,
                r.get::<_, String>(9)?,
            ))
        })
        .map_err(|_| recovery_required())?;
    for row in rows {
        let (op, root, kind, item, id, attempt, scope, command, target, stored) =
            row.map_err(|_| recovery_required())?;
        let request = RequestId::parse(&id).map_err(|_| recovery_required())?;
        let kind = match kind.as_str() {
            "ticket" => AuthoringKind::Ticket,
            "document" => AuthoringKind::Document,
            _ => return Err(recovery_required()),
        };
        if scope != root
            || command != "context retire"
            || target != item
            || digest(request, std::path::Path::new(&root), kind, &item)?.to_string() != stored
        {
            return Err(recovery_required());
        }
        let valid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM request_records WHERE request_ulid=?1 AND confirmation_ulid IS NULL AND base_ref IS NULL AND base_oid IS NULL AND expected_digest IS NULL) AND EXISTS(SELECT 1 FROM request_operations WHERE request_ulid=?1 AND ordinal=0 AND family='local' AND operation_ulid=?2) AND (SELECT count(*) FROM request_operations WHERE request_ulid=?1)=1 AND (SELECT count(*) FROM request_operations WHERE operation_ulid=?2)=1 AND NOT EXISTS(SELECT 1 FROM context_retirement_attempts WHERE operation_ulid=?2 AND (request_attempt IS NULL OR request_attempt>?3))",params![id,op,attempt],|r|r.get(0)).map_err(|_|recovery_required())?;
        if !valid {
            return Err(recovery_required());
        }
    }
    Ok(())
}

/// The index claim carries process authority, never adopts a stored old attempt.
pub(in super::super) fn validate_index(
    db: &Connection,
    record: i64,
    fence: Option<RequestFence>,
    epoch: Option<i64>,
) -> Result<(), RepositoryError> {
    if let Some(f) = fence.filter(|f| f.is_preserving()) {
        return super::preserve::validate_index(db, record, f, epoch);
    }
    if !crate::repository::recovery::has_table(db, "context_retirement_records")
        .map_err(|_| recovery_required())?
    {
        let orphan:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM operation_records WHERE id=?1 AND action='retire_context')",[record],|r|r.get(0)).map_err(|_|recovery_required())?;
        return if fence.is_none() && !orphan {
            Ok(())
        } else {
            Err(recovery_required())
        };
    }
    let row:Option<(Option<String>,String,bool)>=db.query_row("SELECT c.operation_ulid,o.root_path,(o.action='retire_context' OR coalesce(o.target GLOB 'authoring-context-v1/*/*/Retire',0) OR c.operation_ulid IS NOT NULL OR EXISTS(SELECT 1 FROM context_retirement_index_claims i WHERE i.operation_ulid=o.operation_ulid) OR EXISTS(SELECT 1 FROM retirement_observation_receipts b WHERE b.consumer_kind='single' AND b.consumer_operation_ulid=o.operation_ulid) OR EXISTS(SELECT 1 FROM request_operations m JOIN request_records r USING(request_ulid) WHERE m.operation_ulid=o.operation_ulid AND r.command='context retire')) FROM operation_records o LEFT JOIN context_retirement_records c ON c.local_record_id=o.id WHERE o.id=?1",[record],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|_|recovery_required())?;
    if let Some((operation, root, true)) = row {
        let operation = operation.ok_or_else(recovery_required)?;
        // Validate the entire retirement envelope/history BEFORE a heartbeat,
        // error or cache mutation, not after persist or only at finalization.
        record::audit_connection(db)?;
        let scope = fence.map_or(Scope::Domain, Scope::Request);
        scope.validate(db, &operation, std::path::Path::new(&root))?;
        let handoff:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_records c JOIN operation_records o ON o.id=c.local_record_id JOIN repositories r ON r.id=c.repository_id WHERE o.id=?1 AND o.repository_id=c.repository_id AND o.root_path=r.root_path AND o.action='retire_context' AND o.target='authoring-context-v1/'||c.kind||'/'||c.item_id||'/Retire' AND c.phase IN ('local_absent','index_pending','complete') AND c.materialization_settled=1 AND o.completed_step=CASE WHEN c.absence_basis='initial' THEN 'context_retirement_absent' ELSE 'context_branch_released' END AND (?2 IS NULL OR (o.state='indexing' AND o.index_owner_epoch=?2)) AND (?3 IS NULL OR EXISTS(SELECT 1 FROM context_retirement_index_claims i WHERE i.operation_ulid=c.operation_ulid AND i.request_ulid=c.request_ulid AND i.repository_id=c.repository_id AND i.request_attempt=?3 AND i.index_owner_epoch=?2)))",params![record,epoch,fence.map(|f|f.attempt())],|r|r.get(0)).map_err(|_|recovery_required())?;
        if !handoff {
            return Err(recovery_required());
        }
        Ok(())
    } else if fence.is_none() {
        Ok(())
    } else {
        Err(recovery_required())
    }
}

impl RepositoryService {
    pub(super) fn refresh_retirement_request(
        &self,
        request: &RetireContextRequest,
        fence: RequestFence,
    ) -> Result<crate::repository::RefreshOutcome, RepositoryError> {
        let (git, root) = crate::repository::canonical_repository_root(
            &request.root,
            crate::repository::RepositoryOperation::RetireContext,
        )?;
        let (id, owner) = state::with_transaction(self, &root, |tx, id| {
            let scope = Scope::Request(fence);
            scope.validate(tx, &request.operation_id.to_string(), &root)?;
            if !record::refresh_allowed_scoped(tx, &request.operation_id.to_string(), &root, scope)?
            {
                return Err(recovery_required());
            }
            let local:i64=tx.query_row("SELECT local_record_id FROM context_retirement_records WHERE operation_ulid=?1",[request.operation_id.to_string()],|r|r.get(0)).map_err(RepositoryError::sqlite)?;
            // F2 authorizes the claim; the independent index counter identifies
            // its executor. Durable append-only claims spend an F2 attempt once,
            // even across services, invalidation, failures and process restart.
            // The full handoff audit above first proves reverse spend coverage:
            // absence here cannot launder a previously claimed index epoch.
            let used:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_index_claims WHERE operation_ulid=?1 AND request_attempt>=?2)",params![request.operation_id.to_string(),fence.attempt()],|r|r.get(0)).map_err(RepositoryError::sqlite)?;
            if used {
                return Ok((id, None));
            }
            let old: i64 = tx
                .query_row(
                    "SELECT index_owner_epoch FROM operation_records WHERE id=?1",
                    [local],
                    |r| r.get(0),
                )
                .map_err(RepositoryError::sqlite)?;
            let next = old
                .checked_add(1)
                .filter(|_| old >= 0)
                .ok_or_else(recovery_required)?;
            let changed=tx.execute("UPDATE operation_records SET state='indexing',index_owner_epoch=?2,observed_at=?3 WHERE id=?1 AND index_owner_epoch=?4 AND state IN ('authoritative_observed','indexing','failed','observed','persisted','retry','index_invalidated')",params![local,next,self.clock.now().unix_timestamp(),old]).map_err(RepositoryError::sqlite)?;
            if changed == 1 {
                tx.execute("INSERT INTO context_retirement_index_claims(operation_ulid,repository_id,request_ulid,request_attempt,index_owner_epoch) VALUES(?1,?2,?3,?4,?5)",params![request.operation_id.to_string(),id,fence.id(),fence.attempt(),next]).map_err(RepositoryError::sqlite)?;
            }
            let owner = (changed == 1).then_some(crate::repository::recovery::IndexOwner {
                record_id: local,
                epoch: next,
                retirement_request: Some(fence),
            });
            Ok((id, owner))
        })?;
        let Some(owner) = owner else {
            return Ok(crate::repository::RefreshOutcome::IndexPending { root });
        };
        let heartbeat = crate::repository::IndexOwnerHeartbeat::start(
            self.registry_path.clone(),
            owner.clone(),
            root.clone(),
            self.take_index_owner_heartbeat_hook(),
        );
        self.run_refresh_claim_hook();
        let result = self.refresh_repository_claimed(&git, root, id, owner);
        drop(heartbeat);
        result
    }
}

impl RepositoryService {
    #[allow(dead_code)] // Direct fence regression seam; binding supplies its token below.
    pub(crate) fn retire_context_for_request<P: SessionCredentialProvider>(
        &self,
        request: RetireContextRequest,
        accepted: &mutation::Accepted,
        approval: Option<HostApproval>,
        session: &mut SessionCredentials<P>,
    ) -> Result<ContextRetirementOutcome, RepositoryError> {
        self.retire_context_for_request_cancellable(
            request,
            accepted,
            approval,
            session,
            mutation::CancellationToken::new(),
        )
    }
    fn retire_context_for_request_cancellable<P: SessionCredentialProvider>(
        &self,
        request: RetireContextRequest,
        accepted: &mutation::Accepted,
        approval: Option<HostApproval>,
        session: &mut SessionCredentials<P>,
        cancel: mutation::CancellationToken,
    ) -> Result<ContextRetirementOutcome, RepositoryError> {
        if let Some(result) = Scope::finished(self, &request, accepted)? {
            return Ok(result);
        }
        let scope = Scope::capture(self, &request, accepted)?;
        self.retire_context_authenticated_cancellable(request, approval, session, scope, cancel)
    }
    #[allow(dead_code)] // Source-included direct fence tests retain this default-token entry.
    pub(crate) fn reenter_context_for_request<P: SessionCredentialProvider>(
        &self,
        request: ReenterContextRetirementRequest,
        accepted: &mutation::Accepted,
        approval: Option<HostApproval>,
        session: &mut SessionCredentials<P>,
    ) -> Result<ContextRetirementOutcome, RepositoryError> {
        self.reenter_context_for_request_cancellable(
            request,
            accepted,
            approval,
            session,
            mutation::CancellationToken::new(),
        )
    }
    fn reenter_context_for_request_cancellable<P: SessionCredentialProvider>(
        &self,
        request: ReenterContextRetirementRequest,
        accepted: &mutation::Accepted,
        approval: Option<HostApproval>,
        session: &mut SessionCredentials<P>,
        cancel: mutation::CancellationToken,
    ) -> Result<ContextRetirementOutcome, RepositoryError> {
        if let Some(result) = Scope::finished(self, &request.retirement, accepted)? {
            return Ok(result);
        }
        let scope = Scope::capture(self, &request.retirement, accepted)?;
        self.reenter_context_retirement_cancellable(request, approval, session, scope, cancel)
    }
}

pub(crate) struct MutationState {
    pub epoch: i64,
    pub phase: String,
    pub worktree: RetirementRemoval,
    pub branch: RetirementRemoval,
    pub materialization: bool,
}
impl RepositoryService {
    pub(crate) fn retirement_mutation_state(
        &self,
        root: &std::path::Path,
        operation: OperationId,
    ) -> Result<Option<MutationState>, RepositoryError> {
        state::with_transaction(self, root, |tx, _| {
            record::audit_connection(tx)?;
            tx.query_row("SELECT owner_epoch,phase,worktree_removal,branch_removal,materialization_settled FROM context_retirement_records c JOIN operation_records o ON o.id=c.local_record_id WHERE c.operation_ulid=?1 AND o.root_path=?2", params![operation.to_string(),root.to_str()], |r| {
                let removal = |text: String| match text.as_str() { "observed" => RetirementRemoval::Observed, "possible" => RetirementRemoval::Possible, _ => RetirementRemoval::None };
                Ok(MutationState { epoch:r.get(0)?,phase:r.get(1)?,worktree:removal(r.get(2)?),branch:removal(r.get(3)?),materialization:r.get(4)? })
            }).optional().map_err(|_| recovery_required())
        })
    }
    pub(crate) fn execute_retirement_request<P: SessionCredentialProvider>(
        &self,
        request: RetireContextRequest,
        accepted: &mutation::Accepted,
        reentry: bool,
        session: &mut SessionCredentials<P>,
        cancel: crate::repository::CancellationToken,
    ) -> Result<ContextRetirementOutcome, RepositoryError> {
        let existing = self.retirement_mutation_state(&request.root, request.operation_id)?;
        if let Some(s) = existing {
            if s.phase == "preserving" {
                return Ok(ContextRetirementOutcome::IndexPending);
            }
            self.reenter_context_for_request_cancellable(
                ReenterContextRetirementRequest {
                    retirement: request,
                    expected_owner_epoch: s.epoch,
                },
                accepted,
                None,
                session,
                cancel,
            )
        } else if reentry {
            // A current accepted request with no local envelope is provably
            // never claimed (the audit rejects missing companions). Fence the
            // old F2 and settle metadata only, never select today's context.
            let scope = Scope::capture(self, &request, accepted)?;
            record::refuse_fresh_scoped(
                self,
                &request,
                if self
                    .cancellation_requested(accepted.request_id)
                    .unwrap_or(false)
                {
                    crate::results::ResultCode::Cancelled
                } else {
                    crate::results::ResultCode::ExternalChange
                },
                scope,
            )
            .map(ContextRetirementOutcome::Final)
        } else {
            self.retire_context_for_request_cancellable(request, accepted, None, session, cancel)
        }
    }
}
