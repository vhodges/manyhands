//! `document promote`, `ticket close` and `ticket withdraw-closure`: C2's confirmed flow around one
//! library mutation, in the Ctrl-C window (`mutate.rs`), with C2's bounded
//! retry. None takes input, and none reads the item first: the library
//! answers an unknown ID or a wrong kind. Results are rendered by
//! `human_lifecycle`.

use std::path::PathBuf;

use clap::ArgMatches;
use manyhands::{
    canonical::ItemId,
    repository::{
        DocumentPromoteInput, ItemDtoKind, Mutation, RepositoryService, RequestId,
        TicketCloseInput, TicketWithdrawClosureInput,
    },
};

use crate::{
    admin, authoring,
    catalog::CommandSpec,
    human_lifecycle,
    mutate::{self, Session},
    render::Output,
};

pub fn document_promote(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    run(
        service,
        session,
        spec,
        args,
        ItemDtoKind::Document,
        |root, id| Mutation::DocumentPromote(DocumentPromoteInput { root, id }),
    )
}

pub fn ticket_close(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    run(
        service,
        session,
        spec,
        args,
        ItemDtoKind::Ticket,
        |root, id| Mutation::TicketClose(TicketCloseInput { root, id }),
    )
}

pub fn ticket_withdraw_closure(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    run(
        service,
        session,
        spec,
        args,
        ItemDtoKind::Ticket,
        |root, id| Mutation::TicketWithdrawClosure(TicketWithdrawClosureInput { root, id }),
    )
}

fn run(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
    kind: ItemDtoKind,
    build: fn(PathBuf, ItemId) -> Mutation,
) -> Output {
    let selected = || -> Result<_, Output> {
        let flags = admin::flags(session, spec, args)?;
        let root = admin::mutation_root(session, spec, args, flags.request_id)?;
        let id = authoring::item_id(session, spec, args, flags.request_id)?;
        Ok((flags, root, id))
    };
    let (flags, root, id) = match selected() {
        Ok(values) => values,
        Err(out) => return out,
    };
    let driven = mutate::drive(
        service,
        session,
        spec,
        flags,
        mutate::RETRYING,
        &mut |_| build(root.clone(), id.clone()),
        &mut std::thread::sleep,
    );
    mutate::finish(
        session.json,
        driven,
        |envelope| {
            // The command of the request that holds the item, which a
            // request ID is only ever sent again under: a read of its record.
            let holder = |request: &str| {
                let request = RequestId::parse(request).ok()?;
                service
                    .request_record(request)
                    .ok()
                    .flatten()
                    .map(|record| record.command)
            };
            human_lifecycle::prepare(kind, envelope, &holder)
        },
        human_lifecycle::output,
    )
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
