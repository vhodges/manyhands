//! Explicit cleanup through the ordinary mutation driver. No input payload,
//! implicit host approval, identity prompt or separate recovery engine.
use crate::{
    admin, authoring,
    catalog::CommandSpec,
    mutate::{self, Session},
    render::{self, Output, field},
};
use clap::ArgMatches;
use manyhands::{
    repository::{
        AuthoringKind, ContextRetireInput, ContextRetirementMutationDto, Mutation, MutationDataDto,
        RepositoryService,
    },
    results::{Envelope, RecoveryActionKind},
};

pub fn retire(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let selected = || {
        let flags = admin::flags(session, spec, args)?;
        let root = admin::mutation_root(session, spec, args, flags.request_id)?;
        let id = authoring::item_id(session, spec, args, flags.request_id)?;
        let kind = match args.get_one::<String>("kind").map(String::as_str) {
            Some("ticket") => AuthoringKind::Ticket,
            Some("document") => AuthoringKind::Document,
            _ => unreachable!("the catalog parser validates kind"),
        };
        Ok::<_, Output>((flags, root, kind, id))
    };
    let (flags, root, kind, id) = match selected() {
        Ok(v) => v,
        Err(out) => return out,
    };
    let driven = mutate::drive(
        service,
        session,
        spec,
        flags,
        mutate::RETRYING,
        &mut |_| {
            Mutation::ContextRetire(ContextRetireInput {
                root: root.clone(),
                kind,
                id: id.clone(),
            })
        },
        &mut std::thread::sleep,
    );
    mutate::finish(
        session.json,
        driven,
        |e| render::prepare_output(false, e),
        output,
    )
}

pub(crate) fn removal(data: &ContextRetirementMutationDto) -> String {
    format!(
        "worktree_removal={} branch_removal={} materialization_settled={}",
        data.worktree_removal.as_str(),
        data.branch_removal.as_str(),
        data.materialization_settled
    )
}

fn output(envelope: &Envelope<MutationDataDto>, supplied: bool) -> Output {
    let mut plain = envelope.clone();
    let body = if let Some(MutationDataDto::ContextRetirement(data)) = &envelope.data {
        plain.data = None;
        format!(
            "retirement: kind={} item={} state={} code={} {}\n",
            data.kind.as_str(),
            field(&data.item_id),
            data.state.as_str(),
            data.code.as_str(),
            removal(data)
        )
    } else {
        String::new()
    };
    compact(&plain, supplied, &body, "")
}

/// Only the new retirement human paths use this formatter. Current recovery
/// can name an operation absent from the historical report; neither the report
/// nor the envelope's own operation ID supplies a continuation target.
pub(crate) fn compact(
    envelope: &Envelope<MutationDataDto>,
    supplied: bool,
    body: &str,
    after: &str,
) -> Output {
    let mut next = after.to_owned();
    let mut remaining = Vec::new();
    for action in &envelope.recovery {
        let verb = match action.action {
            RecoveryActionKind::OperationResume => "resume",
            RecoveryActionKind::OperationAbandon => "abandon",
            _ => {
                remaining.push(action.clone());
                continue;
            }
        };
        let Some(operation) = action.operation_id.as_deref() else {
            remaining.push(action.clone());
            continue;
        };
        let root = action
            .arguments
            .get("root")
            .and_then(serde_json::Value::as_str)
            .or(envelope.scope.repository.as_deref());
        if let Some(root) = root {
            let prepare = if action.action == RecoveryActionKind::OperationAbandon {
                " --prepare"
            } else {
                ""
            };
            next.push_str(&format!(
                "next: manyhands-cli operation {verb} --repo {} --operation-id {}{prepare}\n",
                render::quoted(root),
                render::quoted(operation)
            ));
        } else {
            // No established root: retain the exact action-owned ID without
            // inventing a repository from an invocation or a frozen target.
            next.push_str(&format!(
                "next: {} operation_id={}\n",
                action.action.as_str(),
                field(operation)
            ));
        }
    }
    render::compact(envelope, supplied, body, &next, remaining, "")
}

#[cfg(test)]
#[path = "retirement_tests.rs"]
mod tests;
