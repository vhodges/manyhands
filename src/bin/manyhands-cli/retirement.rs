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
    results::Envelope,
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
    let Some(MutationDataDto::ContextRetirement(data)) = &envelope.data else {
        return render::mutation_output(false, envelope, supplied);
    };
    let mut plain = envelope.clone();
    plain.data = None;
    let body = format!(
        "retirement: kind={} item={} state={} code={} {}\n",
        data.kind.as_str(),
        field(&data.item_id),
        data.state.as_str(),
        data.code.as_str(),
        removal(data)
    );
    render::compact(&plain, supplied, &body, "", envelope.recovery.clone(), "")
}

#[cfg(test)]
#[path = "retirement_tests.rs"]
mod tests;
