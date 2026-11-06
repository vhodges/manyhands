//! A thin ORIGINAL-LIVE loop over ordinary independent context-retire requests.
//! No durable cursor, batch journal, cleanup recovery or run-as-new exists here.
use super::super::{
    dto::{PollRetirementCompletion, PollRetirementMutationDto, RequestState},
    identity::FieldValue,
};
use super::poll::{PollOnceBinding, PollOnceInput};
use super::*;
use crate::repository::{
    self, PollPublicationRemoteRequest, RemotePollInvocation,
    context_retirement::{handoff, pass},
};

pub(crate) struct PollRetirementBinding {
    root: PathBuf,
}
impl From<PollOnceInput> for PollRetirementBinding {
    fn from(i: PollOnceInput) -> Self {
        Self { root: i.root }
    }
}

/// Report bytes are immutable; continuations are not. Offer only the current
/// ordinary child's route, never the finished pass as a cleanup/index owner.
/// This read is used for both the original answer and finished-parent replay.
pub(crate) fn offer_current_children(
    service: &RepositoryService,
    mut envelope: crate::results::Envelope<MutationDataDto>,
) -> crate::results::Envelope<MutationDataDto> {
    let Some(MutationDataDto::PollRetirement(report)) = envelope.data.as_ref() else {
        return envelope;
    };
    if report.completion != PollRetirementCompletion::Stopped {
        return envelope;
    }
    let Some(root) = envelope.scope.repository.as_deref() else {
        return envelope;
    };
    if service
        .read_session(repository::RepositoryOperation::Read, |db| {
            repository::context_retirement::record::audit_connection(db)
                .map_err(|_| repository::ReadError::new(ResultCode::RecoveryRequired))
        })
        .is_err()
    {
        return envelope;
    }
    for child in &report.retirements {
        let Some(operation) = child
            .operation_id
            .as_deref()
            .and_then(|id| repository::OperationId::parse(id).ok())
        else {
            continue;
        };
        let Ok(id) = super::super::RequestId::parse(&child.request_id) else {
            continue;
        };
        let Ok(Some(record)) = service.request_record(id) else {
            continue;
        };
        if record.state != RequestState::Accepted
            || record.command != "context retire"
            || record.scope.as_str() != root
            || record.target != child.item_id
            || record.operations.len() != 1
            || record.operations[0].family != repository::OperationFamily::Local
            || record.operations[0].operation_id != operation
        {
            continue;
        }
        let action = if super::resume::takes_up(service, root, operation) {
            crate::results::RecoveryAction::for_operation(
                crate::results::RecoveryActionKind::OperationResume,
                operation.to_string(),
                [],
            )
        } else {
            super::super::outcome::same_request(id)
        };
        if !envelope.recovery.contains(&action) {
            envelope.recovery.insert(0, action);
        }
    }
    envelope
}
fn answer(data: PollRetirementMutationDto) -> Ran {
    let code = data.stop_code.unwrap_or(ResultCode::Ok);
    let pending = data
        .retirements
        .iter()
        .any(|r| r.debt == Some(super::super::dto::PollRetirementDebt::Discovery));
    let changed = data
        .retirements
        .iter()
        .filter_map(|r| r.result.as_ref())
        .any(|r| {
            r.worktree_removal != repository::RetirementRemoval::None
                || r.branch_removal != repository::RetirementRemoval::None
        });
    let discovery = if pending || code == ResultCode::DiscoveryPending {
        crate::results::DiscoveryEffect::Pending
    } else if data.completion == PollRetirementCompletion::Completed
        || (data.poll.completion == super::super::dto::PollMutationCompletion::Completed
            && matches!(code, ResultCode::RecoveryRequired | ResultCode::Cancelled))
    {
        crate::results::DiscoveryEffect::Current
    } else {
        crate::results::DiscoveryEffect::NotRequested
    };
    Ran {
        answer: Answer {
            effects: Effects {
                discovery,
                ..Effects::not_requested()
            },
            local_change: changed,
            ..Answer::stopped(code)
        }
        .with_data(MutationDataDto::PollRetirement(data)),
        standing: Standing::Final,
    }
}
impl PollRetirementBinding {
    fn finish(
        &self,
        call: &CallContext<'_>,
        a: &Accepted,
        fallback: Option<super::super::dto::PollMutationDto>,
        forced: Option<ResultCode>,
    ) -> Ran {
        match pass::finish(call.service, a, fallback, forced) {
            Ok(data) => answer(data),
            Err(e) => Ran {
                answer: refused(e),
                standing: Standing::Owed,
            },
        }
    }
    fn run_live(&self, call: &CallContext<'_>, a: &Accepted, mut live: handoff::Live) -> Ran {
        #[cfg(test)]
        let mut ordinal = 0;
        while let Some(input) = live.next() {
            if call.session.cancelled()
                || call
                    .service
                    .cancellation_requested(a.request_id)
                    .unwrap_or(true)
            {
                return self.finish(call, a, None, Some(ResultCode::Cancelled));
            }
            if pass::has_blocking_debt(call.service, &a.root()).unwrap_or(true) {
                return self.finish(call, a, None, Some(ResultCode::RecoveryRequired));
            }
            let ri = match input.request_id() {
                Ok(id) => id,
                Err(e) => {
                    return Ran {
                        answer: refused(e),
                        standing: Standing::Owed,
                    };
                }
            };
            let binding = match retire::ContextRetireBinding::initial(input) {
                Ok(b) => b,
                Err(e) => {
                    return Ran {
                        answer: refused(e),
                        standing: Standing::Owed,
                    };
                }
            };
            let child = call
                .service
                .execute_binding(ri, None, Box::new(binding), call.session);
            match call.service.request_record(ri) {
                Ok(Some(row)) if row.state == RequestState::Finished => {}
                Ok(None) => {
                    if pass::has_blocking_debt(call.service, &a.root()).unwrap_or(true) {
                        return self.finish(call, a, None, Some(ResultCode::RecoveryRequired));
                    }
                    if pass::save_preaccept_refusal(call.service, a, ri, child.code).is_err() {
                        return self.finish(call, a, None, None);
                    }
                }
                Ok(Some(_)) => {
                    return self.finish(
                        call,
                        a,
                        None,
                        Some(
                            if call.session.cancelled()
                                || call
                                    .service
                                    .cancellation_requested(a.request_id)
                                    .unwrap_or(true)
                            {
                                ResultCode::Cancelled
                            } else {
                                ResultCode::RecoveryRequired
                            },
                        ),
                    );
                }
                Err(_) => return self.finish(call, a, None, Some(ResultCode::RecoveryRequired)),
            }
            #[cfg(test)]
            {
                handoff::child_checkpoint(ordinal);
                ordinal += 1;
            }
        }
        self.finish(call, a, None, None)
    }
    fn reported(&self, call: &CallContext<'_>, a: &Accepted, ran: Ran) -> Ran {
        let Some(MutationDataDto::Poll(poll)) = ran.answer.data.as_ref() else {
            return ran;
        };
        if matches!(ran.standing, Standing::Owed) {
            // Before ANY capture the ordinary P remains recoverable. This call
            // reports no retirement and never acquires an initial capability.
            return Ran {
                answer: Answer {
                    data: Some(MutationDataDto::PollRetirement(PollRetirementMutationDto {
                        r#type: super::super::dto::PollRetirementMutationType::PollRetirement,
                        poll: poll.clone(),
                        completion: PollRetirementCompletion::Stopped,
                        stop_code: Some(ran.answer.code),
                        retirements: Vec::new(),
                    })),
                    ..ran.answer
                },
                standing: ran.standing,
            };
        }
        self.finish(
            call,
            a,
            Some(poll.clone()),
            Some(ran.answer.code).filter(|c| *c != ResultCode::Ok),
        )
    }
}
impl Binding for PollRetirementBinding {
    fn command(&self) -> Command {
        Command::PollOnce
    }
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }
    fn place(&self) -> Place<'_> {
        Place::Repository(&self.root)
    }
    fn target(&self) -> String {
        String::new()
    }
    fn fields(&self, b: IntentDigestBuilder) -> IntentDigestBuilder {
        b.field("retire_completed", FieldValue::Flag(true))
    }
    fn scope(&self, repository: Option<String>, position: Option<&Position>) -> Scope {
        Scope {
            repository,
            branch: position.and_then(|p| p.branch().map(str::to_owned)),
            ..Scope::default()
        }
    }
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        PollOnceBinding::from(PollOnceInput {
            root: self.root.clone(),
        })
        .prepare(call)
    }
    fn run(&self, call: &CallContext<'_>, a: &Accepted) -> Ran {
        if let Err(e) = pass::begin(call.service, a) {
            return Ran {
                answer: refused(e),
                standing: Standing::Owed,
            };
        }
        let request = PollPublicationRemoteRequest {
            root: a.root(),
            operation_id: a.operation_id,
            invocation: RemotePollInvocation::Explicit,
            approval: None,
            restart: false,
        };
        match call.session.poll_once_retiring(call.service, request, a) {
            Some(Ok((_, Some(live)))) => self.run_live(call, a, live),
            Some(Ok((result, None))) => {
                self.reported(call, a, PollOnceBinding::reported(call, a, result))
            }
            Some(Err(e)) => {
                if pass::has_capture(call.service, &a.root(), a.operation_id).unwrap_or(false) {
                    return self.finish(call, a, None, None);
                }
                let stopped = PollOnceBinding::stopped(call, a, e);
                if matches!(stopped.standing, Standing::Final)
                    && let Ok(result) = call.service.poll_stored_result(&a.root(), a.operation_id)
                {
                    return self.reported(call, a, PollOnceBinding::reported(call, a, result));
                }
                stopped
            }
            None => Ran {
                answer: Answer::stopped(ResultCode::InternalError),
                standing: Standing::Owed,
            },
        }
    }
    fn reenter(
        &mut self,
        call: &CallContext<'_>,
        a: &Accepted,
        journal: JournalRow,
    ) -> Result<Entered, Ran> {
        match pass::has_capture(call.service, &a.root(), a.operation_id) {
            Ok(true) => Err(self.finish(call, a, None, None)),
            Ok(false) => {
                let mut ordinary = PollOnceBinding::from(PollOnceInput {
                    root: self.root.clone(),
                });
                match ordinary.reenter(call, a, journal) {
                    Ok(entered) => Ok(entered),
                    Err(stopped) => {
                        if let Err(e) = pass::close_unselected(call.service, a) {
                            return Err(Ran {
                                answer: refused(e),
                                standing: Standing::Owed,
                            });
                        }
                        let fallback = call
                            .service
                            .poll_stored_result(&a.root(), a.operation_id)
                            .ok()
                            .and_then(|result| {
                                let reported = PollOnceBinding::reported(call, a, result);
                                match reported.answer.data {
                                    Some(MutationDataDto::Poll(p)) => Some(p),
                                    _ => None,
                                }
                            });
                        Err(self.finish(call, a, fallback, Some(stopped.answer.code)))
                    }
                }
            }
            Err(e) => Err(Ran {
                answer: refused(e),
                standing: Standing::Owed,
            }),
        }
    }
    fn rerun(&self, call: &CallContext<'_>, a: &Accepted, _entered: &Entered) -> Ran {
        // No captured selection: finish ordinary P ONLY. No original source or
        // wrapper is attached on this route, even when repaired P returns Ok.
        let ordinary = PollOnceBinding::from(PollOnceInput {
            root: self.root.clone(),
        });
        let ran = ordinary.rerun(call, a, &Entered::Unfinished);
        self.reported(call, a, ran)
    }
    fn as_confirmed(&mut self) -> Option<&mut dyn Confirmed> {
        None
    }
}
