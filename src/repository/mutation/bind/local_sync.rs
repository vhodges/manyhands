//! Local-primary synchronization and source-carrying resolution bindings.
use super::super::{identity::FieldValue, local_control};
use super::*;
use crate::{
    canonical::ItemId,
    repository::{local_sync as domain, *},
    results::*,
};

pub struct LocalContextSyncInput {
    pub root: PathBuf,
    pub kind: AuthoringKind,
    pub id: ItemId,
    pub observation: String,
}
pub struct LocalConflictResolutionInput {
    pub token: String,
    pub source: String,
}
pub struct LocalContextResolveInput {
    pub root: PathBuf,
    pub operation_id: OperationId,
    pub observation: String,
    pub resolutions: Vec<LocalConflictResolutionInput>,
}
pub(crate) enum LocalSyncBinding {
    Start(LocalContextSyncInput),
    Resolve(LocalContextResolveInput),
}
impl LocalSyncBinding {
    fn call(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        let repo = match resolve_root(call.service, &accepted.root()) {
            Ok(r) => r,
            Err(answer) => {
                return Ran {
                    answer,
                    standing: Standing::Unconfirmed,
                };
            }
        };
        let cancelled = || call.session.cancelled();
        let subject = super::super::progress::Subject::of(self, accepted);
        let progress = |stage, effects| call.session.report(&subject.event(stage, effects));
        let fence = domain::Fence {
            id: accepted.request_id,
            attempt: accepted.attempt,
            cancelled: &cancelled,
            progress: &progress,
        };
        let (id, kind, item, report) = match self {
            Self::Start(input) => {
                let id = accepted.operation_id;
                (
                    id,
                    input.kind,
                    input.id.clone(),
                    domain::execute_start(
                        call.service,
                        &repo,
                        input.kind,
                        &input.id,
                        id,
                        &input.observation,
                        fence,
                    ),
                )
            }
            Self::Resolve(input) => {
                let row = match call
                    .service
                    .read_local_context_sync_record(&repo, input.operation_id)
                {
                    Ok(r) => r,
                    Err(e) => {
                        return Ran {
                            answer: Answer::of_read(&e),
                            standing: Standing::Unconfirmed,
                        };
                    }
                };
                if let Some(ran) = terminal_unlinked(call, accepted, &repo, &row) {
                    return ran;
                }
                (
                    input.operation_id,
                    row.kind,
                    row.item_id.clone(),
                    domain::execute_resolution(
                        call.service,
                        &repo,
                        input.operation_id,
                        &input.observation,
                        &input.resolutions,
                        fence,
                    ),
                )
            }
        };
        if matches!(self, Self::Resolve(_))
            && let Ok(current) = call.service.read_local_context_sync_record(&repo, id)
            && let Some(ran) = terminal_unlinked(call, accepted, &repo, &current)
        {
            return ran;
        }
        let unlinked_resolution = matches!(self, Self::Resolve(_))
            && matches!(
                local_control::linked(call.service, accepted.request_id),
                Ok(false)
            );
        let resolution = (matches!(self, Self::Resolve(_)) && !unlinked_resolution)
            .then_some(accepted.request_id);
        let start = local_control::starting_request(call.service, &accepted.scope, id)
            .ok()
            .flatten();
        let mut dto = domain::data(
            &report,
            kind,
            &item,
            report.state.map(|_| id),
            resolution,
            if unlinked_resolution || report.state.is_none() {
                None
            } else {
                start.or_else(|| matches!(self, Self::Start(_)).then_some(accepted.request_id))
            },
        );
        cancellation_view(call, &repo, id, report.code, &mut dto);
        let standing = if unlinked_resolution {
            Standing::Stopped {
                journal: Some(JournalRow::Absent),
            }
        } else {
            match dto.state {
                LocalContextSyncState::Complete
                | LocalContextSyncState::Cancelled
                | LocalContextSyncState::Abandoned => Standing::Final,
                LocalContextSyncState::Refused => Standing::Stopped {
                    journal: Some(JournalRow::Absent),
                },
                _ => Standing::Owed,
            }
        };
        let mut answer = Answer {
            effects: if unlinked_resolution {
                Effects::not_requested()
            } else {
                report.effects
            },
            ..Answer::stopped(report.code)
        };
        answer.recovery = local_actions(&dto, repo.root());
        answer.offers_nothing = matches!(dto.next_action, LocalContextSyncNextAction::None);
        answer.data = Some(MutationDataDto::LocalContextSync(dto));
        Ran { answer, standing }
    }
}
fn terminal_unlinked(
    call: &CallContext<'_>,
    accepted: &Accepted,
    repo: &ResolvedRepository,
    row: &domain::record::LocalContextSyncRecord,
) -> Option<Ran> {
    let linked = local_control::linked(call.service, accepted.request_id).ok()?;
    if linked {
        return None;
    }
    let state = match row.state {
        domain::record::LocalSyncState::Complete => LocalContextSyncState::Complete,
        domain::record::LocalSyncState::Abandoned => LocalContextSyncState::Abandoned,
        domain::record::LocalSyncState::Cancelled => LocalContextSyncState::Cancelled,
        _ => return None,
    };
    let _ = repo;
    let cancelled = state != LocalContextSyncState::Complete;
    let data = LocalContextSyncMutationDto {
        data_type: LocalContextSyncDataType::LocalContextSync,
        item_id: row.item_id.to_string(),
        kind: authoring_kind_segment(&row.kind).into(),
        operation_id: Some(row.operation_id.to_string()),
        resolution_request_id: Some(accepted.request_id.to_string()),
        required_request_id: None,
        result_oid: None,
        state,
        reason: if cancelled {
            LocalContextSyncReason::Abandoned
        } else {
            LocalContextSyncReason::ForeignChange
        },
        next_action: LocalContextSyncNextAction::None,
        eligibility: None,
        cancel_acknowledged: cancelled,
    };
    Some(Ran {
        answer: Answer {
            offers_nothing: true,
            ..Answer::stopped(if cancelled {
                ResultCode::Cancelled
            } else {
                ResultCode::ExternalChange
            })
            .with_data(MutationDataDto::LocalContextSync(data))
        },
        standing: if cancelled {
            Standing::Final
        } else {
            Standing::Stopped {
                journal: Some(JournalRow::Absent),
            }
        },
    })
}
impl Binding for LocalSyncBinding {
    fn command(&self) -> Command {
        match self {
            Self::Start(_) => Command::LocalContextSync,
            Self::Resolve(_) => Command::LocalContextResolve,
        }
    }
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }
    fn place(&self) -> Place<'_> {
        Place::Repository(match self {
            Self::Start(i) => &i.root,
            Self::Resolve(i) => &i.root,
        })
    }
    fn target(&self) -> String {
        match self {
            Self::Start(i) => format!("{}/{}", authoring_kind_segment(&i.kind), i.id),
            Self::Resolve(i) => i.operation_id.to_string(),
        }
    }
    fn fields(&self, mut builder: IntentDigestBuilder) -> IntentDigestBuilder {
        let observation = match self {
            Self::Start(i) => &i.observation,
            Self::Resolve(i) => &i.observation,
        };
        builder = builder.field("observation", FieldValue::Text(observation));
        if let Self::Resolve(i) = self {
            let mut pairs: Vec<_> = i.resolutions.iter().collect();
            pairs.sort_by(|a, b| a.token.cmp(&b.token));
            for p in pairs {
                builder = builder
                    .field("token", FieldValue::Text(&p.token))
                    .field("source", FieldValue::Bytes(p.source.as_bytes()));
            }
        }
        builder
    }
    fn scope(&self, repository: Option<String>, position: Option<&Position>) -> Scope {
        match self {
            Self::Start(i) => item_scope(&i.kind, &i.id.to_string(), repository, position),
            Self::Resolve(_) => Scope {
                repository,
                ..Scope::default()
            },
        }
    }
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        let root = match self {
            Self::Start(i) => &i.root,
            Self::Resolve(i) => &i.root,
        };
        let repo = resolve_root(call.service, root)?;
        let observation = match self {
            Self::Start(i) => &i.observation,
            Self::Resolve(i) => &i.observation,
        };
        if !domain::valid_observation(observation) {
            return Err(Answer::stopped(ResultCode::InvalidInput));
        }
        let (family, branch) = match self {
            Self::Start(i) => {
                let observed = call
                    .service
                    .observe_local_context_sync(&repo, i.kind, &i.id)
                    .map_err(|e| Answer::of_read(&e))?;
                (Some(OperationFamily::Local), observed.context_ref)
            }
            Self::Resolve(i) => {
                let mut tokens = std::collections::BTreeSet::new();
                if i.resolutions.is_empty()
                    || i.resolutions
                        .iter()
                        .any(|s| !domain::valid_token(&s.token) || !tokens.insert(&s.token))
                {
                    return Err(Answer::stopped(ResultCode::InvalidInput));
                }
                let row = call
                    .service
                    .read_local_context_sync_record(&repo, i.operation_id)
                    .map_err(|e| Answer::of_read(&e))?;
                (None, row.context_ref)
            }
        };
        Ok(Prepared {
            family,
            position: Position::of(repo.root(), &branch)
                .map_err(|_| Answer::stopped(ResultCode::RepositoryInaccessible))?,
            expected: None,
            observation: None,
        })
    }
    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        self.call(call, accepted)
    }
    fn reenter(
        &mut self,
        _call: &CallContext<'_>,
        _accepted: &Accepted,
        _journal: JournalRow,
    ) -> Result<Entered, Ran> {
        Ok(Entered::Unfinished)
    }
    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, _entered: &Entered) -> Ran {
        self.call(call, accepted)
    }
    fn as_confirmed(&mut self) -> Option<&mut dyn Confirmed> {
        None
    }
}

pub(crate) fn local_target(
    service: &RepositoryService,
    root: &std::path::Path,
    id: OperationId,
) -> Result<Option<domain::record::LocalContextSyncRecord>, Answer> {
    use rusqlite::{OptionalExtension, params};
    // Other resume families also support missing/unregistered roots. Preserve
    // their routing; a registered local target is always queried by its resolved
    // canonical scope, never by the caller's path spelling.
    let Ok(repo) = resolve_root(service, root) else {
        return Ok(None);
    };
    service.read_session(RepositoryOperation::Read,|db|{
        Ok(db.query_row("SELECT s.* FROM local_context_sync_records s JOIN repositories r ON r.id=s.repository_id WHERE s.operation_ulid=?1 AND r.root_path=?2",params![id.to_string(),repo.root().to_str()],domain::record::decode).optional()?)
    }).map_err(|e|Answer::of_read(&e))
}
pub(crate) fn prepare_resume(
    call: &CallContext<'_>,
    root: &std::path::Path,
    id: OperationId,
) -> Result<Option<Prepared>, Answer> {
    let Some(row) = local_target(call.service, root, id)? else {
        return Ok(None);
    };
    let repo = resolve_root(call.service, root)?;
    if needs_original(&row) {
        let view = call
            .service
            .inspect_local_context_sync(&repo, id)
            .map_err(|e| Answer::of_read(&e))?;
        let mut answer = Answer::stopped(ResultCode::OriginalRequestRequired);
        if let Some(required) = view.active_resolution_request_id.as_ref() {
            answer
                .recovery
                .push(outcome::original_request(required.clone()));
        }
        answer.data = Some(MutationDataDto::LocalContextSync(
            LocalContextSyncMutationDto {
                data_type: LocalContextSyncDataType::LocalContextSync,
                item_id: view.item_id,
                kind: view.kind,
                operation_id: Some(id.to_string()),
                resolution_request_id: view.active_resolution_request_id.clone(),
                required_request_id: view.active_resolution_request_id,
                result_oid: None,
                state: view.state,
                reason: LocalContextSyncReason::OriginalResolutionRequired,
                next_action: LocalContextSyncNextAction::RetryResolution,
                eligibility: view.eligibility,
                cancel_acknowledged: view.cancel_acknowledged,
            },
        ));
        return Err(answer);
    }
    Ok(Some(Prepared {
        family: None,
        position: Position::of(repo.root(), &row.context_ref)
            .map_err(|_| Answer::stopped(ResultCode::RepositoryInaccessible))?,
        expected: None,
        observation: None,
    }))
}
pub(crate) fn resume(call: &CallContext<'_>, accepted: &Accepted, id: OperationId) -> Option<Ran> {
    let row = match local_target(call.service, &accepted.root(), id) {
        Ok(Some(r)) => r,
        Ok(None) => return None,
        Err(answer) => {
            return Some(Ran {
                answer,
                standing: Standing::Unconfirmed,
            });
        }
    };
    let repo = match resolve_root(call.service, &accepted.root()) {
        Ok(r) => r,
        Err(answer) => {
            return Some(Ran {
                answer,
                standing: Standing::Unconfirmed,
            });
        }
    };
    // The scoped read is complete; linkage/takeover has not begun and no lease
    // or SQL transaction is held. Report this public observation boundary.
    call.session.report(&ProgressEvent {
        request_id: accepted.request_id.to_string(),
        operation_id: None,
        command: Command::OperationResume.name().into(),
        scope: Scope {
            repository: Some(accepted.scope.as_str().into()),
            ..Scope::default()
        },
        stage: ProgressStage::Observing,
        effects: Effects::not_requested(),
    });
    if let Some(ran) = terminal_unlinked(call, accepted, &repo, &row) {
        return Some(ran);
    }
    if needs_original(&row)
        && let Err(answer) = prepare_resume(call, &accepted.root(), id)
    {
        return Some(Ran {
            answer,
            standing: Standing::Stopped {
                journal: Some(JournalRow::Absent),
            },
        });
    }
    let cancelled = || call.session.cancelled();
    let progress = |stage, effects| {
        call.session.report(&ProgressEvent {
            request_id: accepted.request_id.to_string(),
            operation_id: None,
            command: Command::OperationResume.name().into(),
            scope: Scope {
                repository: Some(accepted.scope.as_str().into()),
                ..Scope::default()
            },
            stage,
            effects,
        })
    };
    let report = domain::execute_start(
        call.service,
        &repo,
        row.kind,
        &row.item_id,
        id,
        &row.observation,
        domain::Fence {
            id: accepted.request_id,
            attempt: accepted.attempt,
            cancelled: &cancelled,
            progress: &progress,
        },
    );
    if let Ok(current) = call.service.read_local_context_sync_record(&repo, id)
        && let Some(ran) = terminal_unlinked(call, accepted, &repo, &current)
    {
        return Some(ran);
    }
    let start = local_control::starting_request(call.service, &accepted.scope, id)
        .ok()
        .flatten();
    let resolution = call
        .service
        .inspect_local_context_sync(&repo, id)
        .ok()
        .and_then(|i| i.active_resolution_request_id)
        .and_then(|r| RequestId::parse(&r).ok());
    let mut dto = domain::data(&report, row.kind, &row.item_id, Some(id), resolution, start);
    cancellation_view(call, &repo, id, report.code, &mut dto);
    let unlinked_original = report.code == ResultCode::OriginalRequestRequired
        && matches!(
            local_control::linked(call.service, accepted.request_id),
            Ok(false)
        );
    let standing = if unlinked_original {
        Standing::Stopped {
            journal: Some(JournalRow::Absent),
        }
    } else if matches!(
        dto.state,
        LocalContextSyncState::Complete
            | LocalContextSyncState::Cancelled
            | LocalContextSyncState::Abandoned
    ) {
        Standing::Final
    } else {
        Standing::Owed
    };
    let mut answer = Answer {
        effects: if unlinked_original {
            Effects::not_requested()
        } else {
            report.effects
        },
        ..Answer::stopped(report.code)
    };
    answer.recovery = local_actions(&dto, repo.root());
    answer.offers_nothing = dto.next_action == LocalContextSyncNextAction::None;
    answer.data = Some(MutationDataDto::LocalContextSync(dto));
    Some(Ran { answer, standing })
}

fn local_actions(dto: &LocalContextSyncMutationDto, root: &std::path::Path) -> Vec<RecoveryAction> {
    use LocalContextSyncNextAction as A;
    let arguments = || [("root", root.to_string_lossy().into_owned().into())];
    match dto.next_action {
        A::None => Vec::new(),
        A::Resolve | A::ExternalRepair => vec![RecoveryAction::new(
            RecoveryActionKind::RepoInspect,
            arguments(),
        )],
        A::Resume => dto
            .operation_id
            .as_ref()
            .map(|id| {
                vec![RecoveryAction::for_operation(
                    RecoveryActionKind::OperationResume,
                    id.clone(),
                    [],
                )]
            })
            .unwrap_or_default(),
        A::RetryResolution => dto
            .required_request_id
            .as_ref()
            .map(|id| vec![outcome::original_request(id.clone())])
            .unwrap_or_default(),
        A::SetIdentity => {
            let mut actions = vec![RecoveryAction::new(
                RecoveryActionKind::RepoIdentitySet,
                arguments(),
            )];
            if let Some(id) = dto.required_request_id.as_ref() {
                actions.push(outcome::original_request(id.clone()));
            }
            actions
        }
    }
}
fn needs_original(row: &domain::record::LocalContextSyncRecord) -> bool {
    row.active_resolution.is_some()
        && !row.integration_proven
        && row.candidate.is_none_or(|oid| {
            git2::Repository::open(&row.worktree)
                .and_then(|git| git.find_commit(oid).map(|_| ()))
                .is_err()
        })
}

fn cancellation_view(
    call: &CallContext<'_>,
    repo: &ResolvedRepository,
    id: OperationId,
    code: ResultCode,
    dto: &mut LocalContextSyncMutationDto,
) {
    if code != ResultCode::Cancelled
        || matches!(
            dto.state,
            LocalContextSyncState::Cancelled
                | LocalContextSyncState::Abandoned
                | LocalContextSyncState::Complete
        )
    {
        return;
    }
    if dto.state == LocalContextSyncState::ConflictPending {
        dto.next_action = LocalContextSyncNextAction::ExternalRepair;
    }
    if let Ok(view) = call.service.inspect_local_context_sync(repo, id) {
        dto.next_action = view.next_action;
        dto.eligibility = view.eligibility;
        dto.required_request_id = view.required_request_id;
    }
}
