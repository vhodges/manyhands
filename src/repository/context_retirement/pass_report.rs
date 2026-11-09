//! Metadata-only, immutable pass cuts. No observation, selection, acceptance,
//! index work or cleanup is reachable from this reconciliation path.
use super::*;
use crate::results::ResultCode;

type ChildSnapshotRow = (String, String, Option<String>, String, Option<String>);
type PassSnapshotRow = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    Option<i64>,
    Option<i64>,
);
use mutation::{
    ContextRetirementMutationDto, ContextRetirementMutationType, ContextRetirementState,
    PollMutationDto, PollRetirementCompletion, PollRetirementDebt as D, PollRetirementItemDto,
    PollRetirementItemState as S, PollRetirementMutationDto, PollRetirementMutationType,
};

pub(in super::super::super) fn child_data(
    c: &Candidate,
    op: &str,
    r: super::ContextRetirementReport,
) -> Result<ContextRetirementMutationDto, RepositoryError> {
    Ok(ContextRetirementMutationDto {
        r#type: ContextRetirementMutationType::ContextRetirement,
        kind: if c.kind == "ticket" {
            repository::ItemDtoKind::Ticket
        } else {
            repository::ItemDtoKind::Document
        },
        item_id: c.item_id.clone(),
        operation_id: op.into(),
        state: match r.outcome {
            super::RetirementDisposition::Retired => ContextRetirementState::Retired,
            super::RetirementDisposition::AlreadyAbsent => ContextRetirementState::AlreadyAbsent,
            super::RetirementDisposition::Refused => ContextRetirementState::Refused,
            super::RetirementDisposition::Preserved => ContextRetirementState::Preserved,
        },
        code: r.code,
        worktree_removal: r.worktree_removal,
        branch_removal: r.branch_removal,
        materialization_settled: r.materialization_settled,
    })
}
pub(in super::super::super) fn lost_initial(
    service: &RepositoryService,
    request: &RetireContextRequest,
    a: &Accepted,
) -> Result<Option<ContextRetirementMutationDto>, RepositoryError> {
    let scope = request_scope::Scope::capture(service, request, a)?;
    state::with_transaction(service, &request.root, |tx, id| {
        record::audit_connection(tx)?;
        let row:Option<(String,String,i64)>=tx.query_row("SELECT t.candidate,t.admission,p.repository_id FROM poll_retirement_targets t JOIN poll_retirement_passes p USING(operation_ulid) WHERE t.child_request_ulid=?1 AND t.child_operation_ulid=?2",params![a.request_id.to_string(),a.operation_id.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|_|recovery_required())?;
        let Some((json, status, repository)) = row else {
            return Ok(None);
        };
        let candidate = Candidate::decode(&json)?;
        if repository != id
            || candidate.child_request != a.request_id.to_string()
            || candidate.kind()? != request.kind
            || candidate.item_id != request.item_id.to_string()
        {
            return Err(recovery_required());
        }
        scope.validate(tx, &request.operation_id.to_string(), &request.root)?;
        if !matches!(status.as_str(), "accepted" | "no_effect") {
            return Ok(None);
        }
        if status == "accepted" {
            // A preselection refusal may be durable even when its ordinary
            // boundary copy was lost. Leave it to the audited ORDINARY terminal
            // reader; it is not never-claimed and needs no invented membership.
            let terminal:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_records c JOIN operation_records o ON o.id=c.local_record_id WHERE c.operation_ulid=?1 AND c.request_ulid=?2 AND c.repository_id=?3 AND c.kind=?4 AND c.item_id=?5 AND o.root_path=?6 AND o.state='completed' AND c.initial_resources='invalid' AND c.materialization_capture='unverified' AND c.phase='refused' AND c.final_result IS NOT NULL)",params![request.operation_id.to_string(),a.request_id.to_string(),id,candidate.kind,candidate.item_id,request.root.to_str()],|r|r.get(0)).map_err(|_|recovery_required())?;
            if terminal {
                return Ok(None);
            }
            if !unclaimed(tx, &request.operation_id.to_string(), scope)? {
                return Err(recovery_required());
            }
        }
        tx.execute("UPDATE poll_retirement_targets SET admission='no_effect' WHERE child_request_ulid=?1 AND admission='accepted'",[a.request_id.to_string()]).map_err(|_|recovery_required())?;
        child_data(
            &candidate,
            &request.operation_id.to_string(),
            super::ContextRetirementReport::refused(ResultCode::ExternalChange),
        )
        .map(Some)
    })
}
fn snapshots(
    tx: &Transaction<'_>,
    op: &str,
) -> Result<Vec<PollRetirementItemDto>, RepositoryError> {
    let rows:Vec<ChildSnapshotRow>=tx.prepare("SELECT candidate,child_request_ulid,child_operation_ulid,admission,refusal_code FROM poll_retirement_targets WHERE operation_ulid=?1 ORDER BY ordinal").and_then(|mut q|q.query_map([op],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?.collect()).map_err(|_|recovery_required())?;
    let mut result = Vec::new();
    for (json, ri, ki, admission, refusal) in rows {
        let c = Candidate::decode(&json)?;
        let mut row = PollRetirementItemDto {
            kind: if c.kind == "ticket" {
                repository::ItemDtoKind::Ticket
            } else {
                repository::ItemDtoKind::Document
            },
            item_id: c.item_id.clone(),
            request_id: ri.clone(),
            operation_id: ki.clone(),
            state: S::NotReached,
            code: None,
            debt: None,
            result: None,
        };
        if admission == "refused" {
            row.state = S::Refused;
            row.code = Some(
                ResultCode::ALL
                    .into_iter()
                    .find(|code| Some(code.as_str()) == refusal.as_deref())
                    .ok_or_else(recovery_required)?,
            );
        } else if let Some(ki) = ki {
            let ordinary: Option<Option<String>> = tx
                .query_row(
                    "SELECT result_data FROM request_records WHERE request_ulid=?1",
                    [&ri],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|_| recovery_required())?;
            let ordinary = ordinary.ok_or_else(recovery_required)?;
            let data = if let Some(json) = ordinary {
                let v = serde_json::from_str(&json).map_err(|_| recovery_required())?;
                Some(ContextRetirementMutationDto::from_stored(&v).ok_or_else(recovery_required)?)
            } else {
                let local:Option<(String,Option<String>)>=tx.query_row("SELECT phase,final_result FROM context_retirement_records WHERE operation_ulid=?1",[&ki],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|_|recovery_required())?;
                match local {
                    Some((_, Some(json))) => Some(child_data(
                        &c,
                        &ki,
                        super::ContextRetirementReport::decode(&json)?,
                    )?),
                    Some((phase, None)) => {
                        row.state = S::Pending;
                        row.code = Some(
                            if matches!(
                                phase.as_str(),
                                "local_absent" | "index_pending" | "preserving"
                            ) {
                                ResultCode::DiscoveryPending
                            } else {
                                ResultCode::RecoveryRequired
                            },
                        );
                        row.debt = Some(if row.code == Some(ResultCode::DiscoveryPending) {
                            D::Discovery
                        } else {
                            D::Cleanup
                        });
                        let (w,b,m):(String,String,bool)=tx.query_row("SELECT worktree_removal,branch_removal,materialization_settled FROM context_retirement_records WHERE operation_ulid=?1",[&ki],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|_|recovery_required())?;
                        let removal = |s: &str| match s {
                            "none" => Ok(repository::RetirementRemoval::None),
                            "observed" => Ok(repository::RetirementRemoval::Observed),
                            "possible" => Ok(repository::RetirementRemoval::Possible),
                            _ => Err(recovery_required()),
                        };
                        Some(ContextRetirementMutationDto {
                            r#type: ContextRetirementMutationType::ContextRetirement,
                            kind: row.kind,
                            item_id: c.item_id.clone(),
                            operation_id: ki.clone(),
                            state: ContextRetirementState::Pending,
                            code: row.code.ok_or_else(recovery_required)?,
                            worktree_removal: removal(&w)?,
                            branch_removal: removal(&b)?,
                            materialization_settled: m,
                        })
                    }
                    None if matches!(admission.as_str(), "accepted" | "no_effect") => {
                        let has:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM operation_records WHERE operation_ulid=?1) OR EXISTS(SELECT 1 FROM retirement_observation_receipts WHERE consumer_operation_ulid=?1) OR EXISTS(SELECT 1 FROM poll_retirement_child_sources WHERE child_operation_ulid=?1)",[&ki],|r|r.get(0)).map_err(|_|recovery_required())?;
                        if has {
                            return Err(recovery_required());
                        }
                        tx.execute("UPDATE poll_retirement_targets SET admission='no_effect' WHERE child_request_ulid=?1 AND admission='accepted'",[&ri]).map_err(|_|recovery_required())?;
                        Some(child_data(
                            &c,
                            &ki,
                            super::ContextRetirementReport::refused(ResultCode::ExternalChange),
                        )?)
                    }
                    None => return Err(recovery_required()),
                }
            };
            if let Some(data) = data {
                if data.operation_id != ki || data.item_id != c.item_id || data.kind != row.kind {
                    return Err(recovery_required());
                }
                row.state = match data.state {
                    ContextRetirementState::Retired => S::Retired,
                    ContextRetirementState::AlreadyAbsent => S::AlreadyAbsent,
                    ContextRetirementState::Refused => S::Refused,
                    ContextRetirementState::Preserved => S::Preserved,
                    ContextRetirementState::Pending => S::Pending,
                };
                row.code = Some(data.code);
                if data.state == ContextRetirementState::Pending {
                    row.debt = Some(if data.code == ResultCode::DiscoveryPending {
                        D::Discovery
                    } else {
                        D::Cleanup
                    });
                } else {
                    mutation::finish_recorded_retirement(tx, &ri, &data)?;
                }
                row.result = Some(data);
            }
        }
        result.push(row);
    }
    Ok(result)
}
/// One lease + one immediate TX: revoke starts/effects, snapshot exact children,
/// release ONLY pass holds and write the one immutable cut before parent Final.
pub(in super::super::super) fn finish(
    service: &RepositoryService,
    a: &Accepted,
    fallback: Option<PollMutationDto>,
    forced: Option<ResultCode>,
) -> Result<PollRetirementMutationDto, RepositoryError> {
    let (git, root) = repository::canonical_repository_root(
        &a.root(),
        repository::RepositoryOperation::RetireContext,
    )?;
    let _lease =
        repository::repository_lease(&git, &root, repository::RepositoryOperation::RetireContext)?;
    state::with_transaction(service, &root, |tx, id| {
        let cancelled = validate_parent(tx, a, false)?;
        let (summary,report,remote,primary,handoff,provenance,binding):PassSnapshotRow=tx.query_row("SELECT summary,original_report,remote_name,primary_branch,handoff,provenance_id,binding_id FROM poll_retirement_passes WHERE operation_ulid=?1 AND request_ulid=?2",params![a.operation_id.to_string(),a.request_id.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).map_err(|_|recovery_required())?;
        if let Some(summary) = summary {
            return PollRetirementMutationDto::from_stored(
                &serde_json::from_str(&summary).map_err(|_| recovery_required())?,
            )
            .ok_or_else(recovery_required);
        }
        // Decide cancellation in the same lease/TX as the FIRST immutable cut.
        // An existing cut above is historical, never rewritten by later flags.
        let forced = if cancelled {
            Some(ResultCode::Cancelled)
        } else {
            forced
        };
        record::audit_connection(tx)?;
        let poll = if let Some(report) = report {
            let plan = repository::RemoteRefPlan::from_configuration(
                &remote.ok_or_else(recovery_required)?,
                &primary.ok_or_else(recovery_required)?,
            )
            .map_err(|_| recovery_required())?;
            mutation::poll_report_data(&RemotePollReport::decode_frozen(&report, &plan)?)
                .ok_or_else(recovery_required)?
        } else {
            fallback.ok_or_else(recovery_required)?
        };
        tx.execute(
            "UPDATE poll_retirement_passes SET live=0 WHERE operation_ulid=?1 AND live=1",
            [a.operation_id.to_string()],
        )
        .map_err(|_| recovery_required())?;
        let retirements = snapshots(tx, &a.operation_id.to_string())?;
        let never_claimed:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM poll_retirement_targets WHERE operation_ulid=?1 AND admission='no_effect')",[a.operation_id.to_string()],|r|r.get(0)).map_err(|_|recovery_required())?;
        let complete = handoff == "eligible"
            && forced.is_none()
            && !never_claimed
            && retirements
                .iter()
                .all(|r| !matches!(r.state, S::Pending | S::NotReached));
        let stop = if complete {
            None
        } else {
            Some(forced.unwrap_or(if handoff != "eligible" {
                ResultCode::DiscoveryPending
            } else {
                ResultCode::RecoveryRequired
            }))
        };
        if handoff == "unconfirmed" {
            tx.execute(
                "UPDATE poll_retirement_passes SET handoff='ineligible' WHERE operation_ulid=?1",
                [a.operation_id.to_string()],
            )
            .map_err(|_| recovery_required())?;
        }
        if let Some(binding) = binding {
            record::release_bound_source(
                tx,
                provenance.ok_or_else(recovery_required)?,
                binding,
                true,
            )?;
        }
        let data = PollRetirementMutationDto {
            r#type: PollRetirementMutationType::PollRetirement,
            poll,
            completion: if complete {
                PollRetirementCompletion::Completed
            } else {
                PollRetirementCompletion::Stopped
            },
            stop_code: stop,
            retirements,
        };
        tx.execute("UPDATE poll_retirement_passes SET summary=?2 WHERE operation_ulid=?1 AND summary IS NULL",params![a.operation_id.to_string(),encode(&data)?]).map_err(|_|recovery_required())?;
        // Parent cancellation reaches the child's OWN ordinary safe points.
        if stop == Some(ResultCode::Cancelled) {
            tx.execute("UPDATE request_records SET cancel_requested=1 WHERE state='accepted' AND request_ulid IN (SELECT child_request_ulid FROM poll_retirement_targets WHERE operation_ulid=?1)",[a.operation_id.to_string()]).map_err(|_|recovery_required())?;
        }
        let _ = id;
        record::audit_connection(tx)?;
        Ok(data)
    })
}
pub(in super::super::super) fn save_preaccept_refusal(
    service: &RepositoryService,
    a: &Accepted,
    ri: RequestId,
    code: ResultCode,
) -> Result<(), RepositoryError> {
    let (git, root) = repository::canonical_repository_root(
        &a.root(),
        repository::RepositoryOperation::RetireContext,
    )?;
    let _lease =
        repository::repository_lease(&git, &root, repository::RepositoryOperation::RetireContext)?;
    state::with_transaction(service, &root, |tx, _| {
        if validate_parent(tx, a, true)? {
            return Err(recovery_required());
        }
        if tx.execute("UPDATE poll_retirement_targets SET admission='refused',refusal_code=?3 WHERE operation_ulid=?1 AND child_request_ulid=?2 AND admission='reserved' AND NOT EXISTS(SELECT 1 FROM request_records WHERE request_ulid=?2)",params![a.operation_id.to_string(),ri.to_string(),code.as_str()]).map_err(|_|recovery_required())?!=1{return Err(recovery_required());}
        Ok(())
    })
}
