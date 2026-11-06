//! Ordinary independent retirement. Final evidence is never run as new, even
//! when its cleanup made no checkpoint or refused before an effect.
use super::super::{
    command::Command,
    dto::{
        ContextRetirementMutationDto, ContextRetirementMutationType, ContextRetirementState,
        MutationDataDto,
    },
    evidence::Position,
    identity::{FieldValue, IntentDigestBuilder},
};
use super::{
    Accepted, Answer, Binding, CallContext, Confirmed, EndedWithoutCheckpoint, Entered, Place,
    Prepared, Ran, Standing, item_scope, refused, resolve_root,
};
use crate::repository::{
    AuthoringKind, ContextRetirementOutcome, OperationFamily, RepositoryService,
    RetireContextRequest, RetirementDisposition, RetirementRemoval, authoring_kind_segment,
    recovery::JournalRow,
};
use crate::{
    canonical::ItemId,
    results::{DiscoveryEffect, Effects, ResultCode, Scope},
};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct ContextRetireInput {
    pub root: PathBuf,
    pub kind: AuthoringKind,
    pub id: ItemId,
}
pub(crate) struct ContextRetireBinding(ContextRetireInput);
impl From<ContextRetireInput> for ContextRetireBinding {
    fn from(input: ContextRetireInput) -> Self {
        Self(input)
    }
}

pub(crate) fn recorded_change(service: &RepositoryService, accepted: &Accepted) -> bool {
    service
        .retirement_mutation_state(&accepted.root(), accepted.operation_id)
        .ok()
        .flatten()
        .is_some_and(|s| {
            s.worktree != RetirementRemoval::None || s.branch != RetirementRemoval::None
        })
}

impl ContextRetireBinding {
    pub(crate) fn call(&self, call: &CallContext<'_>, accepted: &Accepted, reentry: bool) -> Ran {
        let request = RetireContextRequest {
            root: accepted.root(),
            kind: self.0.kind,
            item_id: self.0.id.clone(),
            operation_id: accepted.operation_id,
        };
        let result = call
            .session
            .retire_context(call.service, request, accepted, reentry);
        let Some(result) = result else {
            return Ran {
                answer: Answer::stopped(ResultCode::InternalError),
                standing: Standing::Owed,
            };
        };
        let snapshot = call
            .service
            .retirement_mutation_state(&accepted.root(), accepted.operation_id);
        let (code, state, standing, discovery, worktree, branch, materialization) = match result {
            Ok(ContextRetirementOutcome::Final(report)) => (
                report.code,
                match report.outcome {
                    RetirementDisposition::Retired => ContextRetirementState::Retired,
                    RetirementDisposition::AlreadyAbsent => ContextRetirementState::AlreadyAbsent,
                    RetirementDisposition::Refused => ContextRetirementState::Refused,
                    RetirementDisposition::Preserved => ContextRetirementState::Preserved,
                },
                Standing::Final,
                if report.materialization_settled
                    || report.outcome == RetirementDisposition::Preserved
                {
                    DiscoveryEffect::Current
                } else {
                    DiscoveryEffect::NotRequested
                },
                report.worktree_removal,
                report.branch_removal,
                report.materialization_settled,
            ),
            Ok(ContextRetirementOutcome::IndexPending) => {
                let s = snapshot.ok().flatten();
                (
                    ResultCode::DiscoveryPending,
                    ContextRetirementState::Pending,
                    Standing::Owed,
                    DiscoveryEffect::Pending,
                    s.as_ref().map_or(RetirementRemoval::None, |s| s.worktree),
                    s.as_ref().map_or(RetirementRemoval::None, |s| s.branch),
                    s.is_some_and(|s| s.materialization),
                )
            }
            Ok(ContextRetirementOutcome::CleanupPending {
                worktree_removal,
                branch_removal,
            }) => (
                if call.session.cancelled()
                    || call
                        .service
                        .cancellation_requested(accepted.request_id)
                        .unwrap_or(false)
                {
                    ResultCode::Cancelled
                } else {
                    ResultCode::RecoveryRequired
                },
                ContextRetirementState::Pending,
                Standing::Owed,
                DiscoveryEffect::NotRequested,
                worktree_removal,
                branch_removal,
                false,
            ),
            Err(error) => {
                let answer = refused(error);
                match snapshot {
                    Ok(Some(s)) => (
                        answer.code,
                        ContextRetirementState::Pending,
                        Standing::Owed,
                        if matches!(
                            s.phase.as_str(),
                            "preserving" | "local_absent" | "index_pending"
                        ) {
                            DiscoveryEffect::Pending
                        } else {
                            DiscoveryEffect::NotRequested
                        },
                        s.worktree,
                        s.branch,
                        s.materialization,
                    ),
                    Ok(None) => (
                        answer.code,
                        ContextRetirementState::Refused,
                        if matches!(answer.code, ResultCode::Busy | ResultCode::PollYielding) {
                            Standing::Owed
                        } else {
                            Standing::Final
                        },
                        DiscoveryEffect::NotRequested,
                        RetirementRemoval::None,
                        RetirementRemoval::None,
                        false,
                    ),
                    Err(_) => {
                        return Ran {
                            answer,
                            standing: Standing::Owed,
                        };
                    }
                }
            }
        };
        let data = MutationDataDto::ContextRetirement(ContextRetirementMutationDto {
            r#type: ContextRetirementMutationType::ContextRetirement,
            kind: match self.0.kind {
                AuthoringKind::Ticket => crate::repository::ItemDtoKind::Ticket,
                AuthoringKind::Document => crate::repository::ItemDtoKind::Document,
            },
            item_id: self.0.id.to_string(),
            operation_id: accepted.operation_id.to_string(),
            state,
            code,
            worktree_removal: worktree,
            branch_removal: branch,
            materialization_settled: materialization,
        });
        Ran {
            answer: Answer {
                effects: Effects {
                    discovery,
                    ..Effects::not_requested()
                },
                local_change: worktree != RetirementRemoval::None
                    || branch != RetirementRemoval::None,
                ..Answer::stopped(code)
            }
            .with_data(data),
            standing,
        }
    }
}
impl Binding for ContextRetireBinding {
    fn command(&self) -> Command {
        Command::ContextRetire
    }
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }
    fn place(&self) -> Place<'_> {
        Place::Repository(&self.0.root)
    }
    fn target(&self) -> String {
        self.0.id.to_string()
    }
    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        builder.field(
            "kind",
            FieldValue::Text(authoring_kind_segment(&self.0.kind)),
        )
    }
    fn scope(&self, repository: Option<String>, position: Option<&Position>) -> Scope {
        item_scope(&self.0.kind, &self.0.id.to_string(), repository, position)
    }
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        resolve_root(call.service, &self.0.root)?;
        Ok(Prepared {
            observation: None,
            family: Some(OperationFamily::Local),
            position: Position {
                base_ref: None,
                base_oid: None,
            },
            expected: None,
        })
    }
    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        self.call(call, accepted, false)
    }
    fn reenter(
        &mut self,
        _call: &CallContext<'_>,
        _accepted: &Accepted,
        _journal: JournalRow,
    ) -> Result<Entered, Ran> {
        Ok(Entered::Unfinished)
    }
    fn as_confirmed(&mut self) -> Option<&mut dyn Confirmed> {
        None
    }
    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, _entered: &Entered) -> Ran {
        self.call(call, accepted, true)
    }
}

/// A resume may spend a NEW current F2 for precisely this item's I-only debt.
/// It never obtains a destructive input, selects a context or observes a remote.
pub(crate) fn resume_handoff(
    call: &CallContext<'_>,
    root: &std::path::Path,
    operation: crate::repository::OperationId,
) -> Option<Ran> {
    use crate::repository::{
        ReadError, RepositoryOperation,
        context_retirement::{preserve, record},
    };
    let state = match call.service.retirement_mutation_state(root, operation) {
        Ok(None) => return None,
        Ok(Some(s)) => s,
        Err(error) => {
            return Some(Ran {
                answer: refused(error),
                standing: Standing::Owed,
            });
        }
    };
    let found=call.service.read_session(RepositoryOperation::Read,|db| {
        record::audit_connection(db).map_err(|_|ReadError::new(ResultCode::RecoveryRequired))?;
        let (kind,item,owner):(String,String,Option<String>)=db.query_row("SELECT kind,item_id,request_ulid FROM context_retirement_records WHERE operation_ulid=?1",[operation.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
        let owner=if state.phase=="preserving" {
            preserve::owner(db,operation).map_err(|_|ReadError::new(ResultCode::RecoveryRequired))?.map(|id|id.to_string())
        }else{owner};
        Ok((kind,item,owner))
    });
    let (kind, item, owner) = match found {
        Ok(v) => v,
        Err(e) => {
            return Some(Ran {
                answer: Answer::of_read(&e),
                standing: Standing::Owed,
            });
        }
    };
    let owner = owner?; // Ordinary unbound domain I remains generic.
    let run = || -> Result<Ran, Answer> {
        let id = super::super::identity::RequestId::parse(&owner)
            .map_err(|_| Answer::stopped(ResultCode::RecoveryRequired))?;
        let attempt = call
            .service
            .enter_request(id)
            .map_err(|e| Answer::of_read(&e))?
            .ok_or_else(|| Answer::stopped(ResultCode::RecoveryRequired))?;
        let row = call
            .service
            .request_record(id)
            .map_err(|e| Answer::of_read(&e))?
            .ok_or_else(|| Answer::stopped(ResultCode::RecoveryRequired))?;
        let accepted = Accepted::of_record(&row, attempt)
            .ok_or_else(|| Answer::stopped(ResultCode::RecoveryRequired))?;
        let ran = if state.phase == "preserving" {
            let result =
                preserve::refresh(call.service, root, operation, &accepted).map_err(refused)?;
            let complete = call
                .service
                .retirement_mutation_state(root, operation)
                .map_err(refused)?
                .is_some_and(|s| s.phase == "abandoned");
            let _ = result;
            Ran {
                answer: Answer {
                    effects: Effects {
                        discovery: if complete {
                            DiscoveryEffect::Current
                        } else {
                            DiscoveryEffect::Pending
                        },
                        ..Effects::not_requested()
                    },
                    ..Answer::stopped(if complete {
                        ResultCode::Ok
                    } else {
                        ResultCode::DiscoveryPending
                    })
                },
                standing: if complete {
                    Standing::Final
                } else {
                    Standing::Owed
                },
            }
        } else {
            if !matches!(state.phase.as_str(), "local_absent" | "index_pending") {
                return Err(Answer::stopped(ResultCode::RecoveryRequired));
            }
            let binding = ContextRetireBinding(ContextRetireInput {
                root: root.into(),
                kind: match kind.as_str() {
                    "ticket" => AuthoringKind::Ticket,
                    "document" => AuthoringKind::Document,
                    _ => return Err(Answer::stopped(ResultCode::RecoveryRequired)),
                },
                id: item
                    .parse()
                    .map_err(|_| Answer::stopped(ResultCode::RecoveryRequired))?,
            });
            binding.call(call, &accepted, true)
        };
        if matches!(ran.standing, Standing::Final) {
            let _ = call.service.finish(id, attempt, &ran.answer);
        }
        let mut answer = ran.answer;
        if matches!(ran.standing, Standing::Final)
            && matches!(answer.code, ResultCode::Ok | ResultCode::AlreadyApplied)
        {
            answer = Answer {
                effects: Effects {
                    discovery: DiscoveryEffect::Current,
                    ..Effects::not_requested()
                },
                ..Answer::stopped(ResultCode::Ok)
            };
        } else {
            answer.data = None;
        }
        Ok(Ran {
            answer,
            standing: ran.standing,
        })
    };
    Some(run().unwrap_or_else(|answer| Ran {
        answer,
        standing: Standing::Owed,
    }))
}
