//! `poll configure`, `poll pause` and `poll resume`. Each writes one field of
//! the repository's polling policy in its request's own transaction, so it has
//! no safe point and takes no Ctrl-C window, and no verb here sets `enabled`.
//! A policy answer holds the policy as it left it, which is all that the note
//! about a recovery suspension reads.

use clap::ArgMatches;
use manyhands::{
    repository::{
        Mutation, MutationDataDto, PollConfigureInput, PollPauseInput, PollResumeInput,
        RepositoryService,
    },
    results::Envelope,
};
use std::path::PathBuf;

use crate::{
    admin,
    catalog::CommandSpec,
    mutate::{self, Driven, MutationFlags, Plan, Session},
    render::{self, Output, note_lines},
};

/// Decision 5: `poll resume` does not lift the suspension, and clearing it is
/// not a CLI verb.
const SUSPENDED_NOTE: &str = "automatic polls are suspended for recovery, and poll resume does not lift that; poll once still runs.";

pub fn configure(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let selected = || -> Result<_, Output> {
        let flags = admin::flags(session, spec, args)?;
        let object = admin::input_object(session, spec, args, flags.request_id)?;
        let interval_seconds = object
            .only(&["interval_seconds"])
            .and_then(|()| object.whole_number("interval_seconds"))
            .map_err(|error| admin::input_failure(session, spec, flags.request_id, error))?;
        let root = admin::mutation_root(session, spec, args, flags.request_id)?;
        Ok((flags, root, interval_seconds))
    };
    let (flags, root, interval_seconds) = match selected() {
        Ok(values) => values,
        Err(out) => return out,
    };
    policy(service, session, spec, flags, &mut |_| {
        Mutation::PollConfigure(PollConfigureInput {
            root: root.clone(),
            interval_seconds,
        })
    })
}

pub fn pause(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    flagged(service, session, spec, args, |root| {
        Mutation::PollPause(PollPauseInput { root })
    })
}

pub fn resume(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    flagged(service, session, spec, args, |root| {
        Mutation::PollResume(PollResumeInput { root })
    })
}

fn flagged(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
    build: fn(PathBuf) -> Mutation,
) -> Output {
    let selected = || -> Result<_, Output> {
        let flags = admin::flags(session, spec, args)?;
        let root = admin::mutation_root(session, spec, args, flags.request_id)?;
        Ok((flags, root))
    };
    let (flags, root) = match selected() {
        Ok(values) => values,
        Err(out) => return out,
    };
    policy(service, session, spec, flags, &mut |_| build(root.clone()))
}

fn policy(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    flags: MutationFlags,
    build: &mut dyn FnMut(&mutate::Extras) -> Mutation,
) -> Output {
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
        build,
        &mut std::thread::sleep,
    ) {
        Driven::Answered(output) => output,
        Driven::Prepared(envelope) => render::prepare_output(session.json, &envelope),
        Driven::Executed { envelope, supplied } => policy_output(session.json, &envelope, supplied),
    }
}

/// C2's output, and in human mode a note when the answer's data holds a
/// suspension.
fn policy_output(json: bool, envelope: &Envelope<MutationDataDto>, supplied: bool) -> Output {
    let mut output = render::mutation_output(json, envelope, supplied);
    if !json
        && let Some(MutationDataDto::PollPolicy(data)) = &envelope.data
        && data.recovery_suspended
    {
        output
            .stderr
            .extend(note_lines(&[SUSPENDED_NOTE.to_owned()]).into_bytes());
    }
    output
}

#[cfg(test)]
#[path = "polling_tests.rs"]
mod tests;
