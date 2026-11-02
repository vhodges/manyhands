//! `document promote` and `ticket close`: C2's confirmed flow around one
//! library mutation, in the Ctrl-C window (`mutate.rs`), with C2's bounded
//! retry. Neither takes input, and neither reads the item first: the library
//! answers an unknown ID or a wrong kind. Results are rendered by
//! `human_lifecycle`.

use std::path::PathBuf;

use clap::ArgMatches;
use manyhands::{
    canonical::ItemId,
    repository::{DocumentPromoteInput, Mutation, RepositoryService, TicketCloseInput},
};

use crate::{
    admin, authoring,
    catalog::CommandSpec,
    human_lifecycle,
    mutate::{self, Driven, Plan, Session},
    render::{self, Output},
};

pub fn document_promote(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    run(service, session, spec, args, |root, id| {
        Mutation::DocumentPromote(DocumentPromoteInput { root, id })
    })
}

pub fn ticket_close(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    run(service, session, spec, args, |root, id| {
        Mutation::TicketClose(TicketCloseInput { root, id })
    })
}

fn run(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
    build: fn(PathBuf, ItemId) -> Mutation,
) -> Output {
    let (flags, root, id) = match (|| {
        let flags = admin::flags(session, spec, args)?;
        let root = admin::mutation_root(session, spec, args, flags.request_id)?;
        let id = authoring::item_id(session, spec, args, flags.request_id)?;
        Ok((flags, root, id))
    })() {
        Ok(values) => values,
        Err(out) => return out,
    };
    let plan = Plan {
        identity_prompt: false,
        retry: true,
    };
    match mutate::drive(
        service,
        session,
        spec,
        flags,
        plan,
        &mut |_| build(root.clone(), id.clone()),
        &mut std::thread::sleep,
    ) {
        Driven::Answered(output) => output,
        Driven::Executed { envelope, supplied } if session.json => {
            render::mutation_output(true, &envelope, supplied)
        }
        Driven::Executed { envelope, supplied } => human_lifecycle::output(&envelope, supplied),
    }
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
