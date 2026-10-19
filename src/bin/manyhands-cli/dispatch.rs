use super::{
    catalog::CommandSpec,
    human,
    render::{self, Output},
};
use clap::ArgMatches;
use manyhands::repository::RepositoryService;
use manyhands::results::{Envelope, ResultCode, Scope};

pub fn dispatch(
    command: &'static CommandSpec,
    _args: &ArgMatches,
    json: bool,
    service: &RepositoryService,
) -> Output {
    let name = format!("{} {}", command.resource, command.verb);
    if name == "id new" {
        let envelope = Envelope::read_success(name, Scope::default(), service.new_item_id());
        return render::envelope_output(json, &envelope, human::new_id);
    }
    let code = if placeholder_verbs().contains(&name.as_str()) {
        render::UNAVAILABLE_CODE
    } else {
        // A Run not covered by dispatch is an internal routing error.
        ResultCode::InternalError
    };
    render::failure_output(
        json,
        &Envelope::failure(name, Scope::default(), code, Vec::new()),
        None,
    )
}

pub fn placeholder_verbs() -> Vec<&'static str> {
    vec![
        "repo list",
        "repo inspect",
        "repo identity",
        "remote list",
        "key list",
        "key show",
        "key public",
        "host list",
        "host inspect",
        "document list",
        "document show",
        "folder list",
        "ticket list",
        "ticket show",
        "ticket ready",
        "ticket blocked",
        "ticket deps",
        "ticket children",
        "ticket cycles",
        "ticket plan",
        "ticket critical-path",
        "ticket find",
        "comment list",
        "index status",
        "poll status",
        "operation list",
        "operation show",
    ]
}

#[cfg(test)]
#[path = "dispatch_tests.rs"]
mod tests;
