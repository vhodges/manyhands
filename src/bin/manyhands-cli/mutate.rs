use crate::{
    catalog::{CommandSpec, Kind},
    human_admin,
    progress::HumanProgress,
    prompt::{self, Asked, Consent, TerminalProvider},
    render::{self, Output},
    terminal::Terminal,
};
use manyhands::{
    repository::{keys::SessionCredentials, *},
    results::{Envelope, Outcome, RecoveryActionKind, ResultCode, Scope},
};
use std::{cell::Cell, io::Write, time::Duration};

#[derive(Debug, PartialEq, Eq)]
pub enum Flow {
    Prepare,
    Execute {
        confirmation: Option<ConfirmationId>,
    },
    Interactive,
}
pub fn flow(
    prepare: bool,
    confirm: Option<ConfirmationId>,
    confirmed: bool,
    interactive: bool,
    record_exists: bool,
) -> Flow {
    if prepare {
        Flow::Prepare
    } else if confirm.is_some() || !confirmed || !interactive || record_exists {
        Flow::Execute {
            confirmation: confirm,
        }
    } else {
        Flow::Interactive
    }
}
pub trait Boundary {
    fn prepare(&self, mutation: &Mutation) -> Envelope<PrepareDataDto>;
    fn execute(
        &self,
        call: MutationCall,
        credentials: &mut SessionCredentials<TerminalProvider<'_>>,
        progress: &mut dyn ProgressSink,
    ) -> Envelope<MutationDataDto>;
    fn request_exists(&self, id: RequestId) -> Result<bool, Envelope<RequestDto>>;
}
impl Boundary for RepositoryService {
    fn prepare(&self, mutation: &Mutation) -> Envelope<PrepareDataDto> {
        RepositoryService::prepare(self, mutation)
    }
    fn execute(
        &self,
        call: MutationCall,
        credentials: &mut SessionCredentials<TerminalProvider<'_>>,
        progress: &mut dyn ProgressSink,
    ) -> Envelope<MutationDataDto> {
        RepositoryService::execute(self, call, credentials, progress, &CancellationToken::new())
    }
    fn request_exists(&self, id: RequestId) -> Result<bool, Envelope<RequestDto>> {
        let envelope = self.show_request(id);
        if envelope.failure_class().is_none() {
            Ok(true)
        } else if envelope.code == ResultCode::RequestNotFound {
            Ok(false)
        } else {
            Err(envelope)
        }
    }
}
pub const RETRY_DELAYS_MS: [u64; 3] = [100, 200, 400];
pub fn should_retry(envelope: &Envelope<MutationDataDto>, request_id: RequestId) -> bool {
    (envelope.code == ResultCode::Busy || envelope.outcome == Outcome::Partial)
        && envelope.recovery.first().is_some_and(|action| {
            action.action == RecoveryActionKind::RequestRetry
                && action
                    .arguments
                    .get("request_id")
                    .and_then(serde_json::Value::as_str)
                    == Some(request_id.to_string().as_str())
        })
}
pub struct Session {
    pub json: bool,
    pub interactive: bool,
    pub terminal: Option<Box<dyn Terminal>>,
}
pub struct MutationFlags {
    pub request_id: Option<RequestId>,
    pub prepare: bool,
    pub confirm: Option<ConfirmationId>,
}
#[derive(Default)]
pub struct Extras {
    pub identity: Option<CommitIdentity>,
}
pub struct Plan {
    pub identity_prompt: bool,
    pub retry: bool,
}
pub fn note(line: &str) {
    let _ = writeln!(std::io::stderr().lock(), "{line}");
}

pub fn run(
    boundary: &dyn Boundary,
    session: &mut Session,
    spec: &'static CommandSpec,
    flags: MutationFlags,
    plan: Plan,
    build: &mut dyn FnMut(&Extras) -> Mutation,
    sleep: &mut dyn FnMut(Duration),
) -> Output {
    let command = format!("{} {}", spec.resource, spec.verb);
    let confirmed = matches!(
        spec.kind,
        Kind::Mutation {
            confirmed: true,
            ..
        }
    );
    let mut extras = Extras::default();
    if flags.prepare {
        return render::prepare_output(session.json, &boundary.prepare(&build(&extras)));
    }
    let request_id = match flags.request_id {
        Some(id) => id,
        None if session.interactive => {
            let id = RequestId::new();
            note(&format!("request: {id}"));
            id
        }
        None => return refusal(session, &command, None, ResultCode::UsageError, None),
    };
    let exists = if confirmed
        && session.interactive
        && flags.request_id.is_some()
        && flags.confirm.is_none()
    {
        match boundary.request_exists(request_id) {
            Ok(exists) => exists,
            Err(envelope) => {
                return render::envelope_output(session.json, &envelope, human_admin::request);
            }
        }
    } else {
        false
    };
    let confirmation = match flow(false, flags.confirm, confirmed, session.interactive, exists) {
        Flow::Prepare => unreachable!("prepare returned before allocating a request"),
        Flow::Execute { confirmation } => confirmation,
        Flow::Interactive => {
            let mut envelope = boundary.prepare(&build(&extras));
            if envelope.code == ResultCode::IdentityRequired && plan.identity_prompt {
                let asked = match session.terminal.as_deref_mut() {
                    Some(terminal) => prompt::identity(terminal),
                    None => Asked::Cancelled,
                };
                match asked {
                    Asked::Answer(identity) => extras.identity = Some(identity),
                    Asked::Cancelled => {
                        return refusal(
                            session,
                            &command,
                            Some(request_id),
                            ResultCode::Cancelled,
                            None,
                        );
                    }
                    Asked::Exhausted => {
                        return refusal(
                            session,
                            &command,
                            Some(request_id),
                            ResultCode::InvalidInput,
                            None,
                        );
                    }
                }
                note(&identity_note(request_id));
                envelope = boundary.prepare(&build(&extras));
            }
            let Some(preview) = envelope.data.as_ref().and_then(PrepareDataDto::preview) else {
                return render::prepare_output(session.json, &envelope);
            };
            let consent = session
                .terminal
                .as_deref_mut()
                .map(|t| prompt::consent(t, &human_admin::preview(preview)))
                .unwrap_or(Consent::No);
            if matches!(consent, Consent::No) {
                return refusal(
                    session,
                    &command,
                    Some(request_id),
                    ResultCode::Cancelled,
                    None,
                );
            }
            // A library preview always contains a canonical confirmation ID.
            match ConfirmationId::parse(&preview.confirmation_id) {
                Ok(id) => Some(id),
                Err(_) => {
                    return refusal(
                        session,
                        &command,
                        Some(request_id),
                        ResultCode::InternalError,
                        None,
                    );
                }
            }
        }
    };
    let supplied = Cell::new(false);
    let terminal = if session.interactive {
        session
            .terminal
            .as_deref_mut()
            .map(|t| t as &mut dyn Terminal)
    } else {
        None
    };
    let mut credentials = SessionCredentials::new(TerminalProvider::new(terminal, &supplied));
    let mut human_progress = HumanProgress::default();
    let mut no_progress = NoProgress;
    let progress: &mut dyn ProgressSink = if session.json {
        &mut no_progress
    } else {
        &mut human_progress
    };
    let mut attempts = 0;
    loop {
        let envelope = boundary.execute(
            MutationCall {
                request_id,
                confirmation,
                mutation: build(&extras),
            },
            &mut credentials,
            progress,
        );
        if !plan.retry || attempts == RETRY_DELAYS_MS.len() || !should_retry(&envelope, request_id)
        {
            return render::mutation_output(session.json, &envelope, supplied.get());
        }
        if !session.json {
            note("progress: retrying");
        }
        sleep(Duration::from_millis(RETRY_DELAYS_MS[attempts]));
        attempts += 1;
    }
}

fn identity_note(id: RequestId) -> String {
    format!(
        "note: request {id} now includes the identity you entered; every retry, including at a terminal, must give it as \"identity\" in --input"
    )
}

pub fn refusal(
    session: &Session,
    command: &str,
    request_id: Option<RequestId>,
    code: ResultCode,
    detail: Option<String>,
) -> Output {
    let mut envelope = Envelope::classified_mutation(
        command,
        Scope::default(),
        code == ResultCode::Cancelled,
        code,
        manyhands::results::Effects::not_requested(),
        false,
    );
    envelope.request_id = request_id.map(|id| id.to_string());
    let mut extra = String::new();
    if let Some(id) = &envelope.request_id {
        extra.push_str(&format!("request: {id}\n"));
    }
    if let Some(detail) = detail {
        extra.push_str(&detail);
        extra.push('\n');
    }
    render::failure_output(
        session.json,
        &envelope,
        (!extra.is_empty()).then_some(extra),
    )
}

#[cfg(test)]
#[path = "mutate_tests.rs"]
mod tests;
