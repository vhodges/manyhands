//! The polling verbs.
//!
//! `poll configure`, `poll pause` and `poll resume` each write one field of the
//! repository's polling policy in the request's own transaction, so they have no
//! safe point and take no Ctrl-C window, and no verb here sets `enabled`. A
//! policy answer holds the policy as it left it, which is all that the note about
//! a recovery suspension reads.
//!
//! `poll once` is one explicit pass, in the Ctrl-C window (`mutate.rs`) and with
//! C2's unlock prompt at a terminal. A poll's stop after its reservation is final
//! (R-18), so its human lines say a new request follows and never that this one
//! continues. What a target is named by comes from the data, whose fields are all
//! null for a malformed ref; each is escaped when printed.

use clap::ArgMatches;
use manyhands::{
    repository::{
        Mutation, MutationDataDto, PollConfigureInput, PollMutationDto, PollOnceInput,
        PollPauseInput, PollResumeInput, RepositoryService,
    },
    results::{Envelope, RecoveryActionKind, ResultCode},
};
use std::path::PathBuf;

use crate::{
    admin,
    catalog::CommandSpec,
    human_admin,
    mutate::{self, MutationFlags, Session},
    render::{self, Output, field, quoted, refusal_of},
};

/// Decision 5: `poll resume` does not lift the suspension, and clearing it is
/// not a CLI verb.
const SUSPENDED_NOTE: &str = "automatic polls are suspended for recovery, and poll resume does not lift that; poll once still runs.";

/// What follows a stop no resend gets past. Each says "new request" because
/// sending the same one gets this answer back, or is no help.
const NEW_REQUEST_NOTE: &str = "polling again takes a new request.";
const HOST_NOTE: &str = "once the host is approved or replaced, polling again takes a new request.";
const UNLOCK_NOTE: &str =
    "polling again takes a new request, made at a terminal, where it can ask for the passphrase.";
const KEY_NOTE: &str = "once a usable key is selected, polling again takes a new request.";
const REMOTE_NOTE: &str =
    "once a publication remote is selected, polling again takes a new request.";
const PASSPHRASE_HINT: &str = "hint: if the passphrase was wrong, poll again with a new request\n";
const BUSY_NOTE: &str = "something else holds this repository, and it may be no operation: operation list names the unfinished operations, and operation resume, with an operation's ID for the <operation-id> placeholder, names the request that holds it. If none is another request's, wait and send this request again (add --request-id unless you run it at a terminal).";
/// The library refuses an interval out of range with no detail of its own.
const RANGE_DETAIL: &str = "Input field interval_seconds must be from 60 to 3600.\n";

/// The flags and root every polling verb starts from.
fn selected(
    session: &Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Result<(MutationFlags, PathBuf), Output> {
    let flags = admin::flags(session, spec, args)?;
    let root = admin::mutation_root(session, spec, args, flags.request_id)?;
    Ok((flags, root))
}

pub fn once(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let (flags, root) = match selected(session, spec, args) {
        Ok(values) => values,
        Err(out) => return out,
    };
    // Every polling verb runs C2's bounded retry and asks for no identity.
    let driven = mutate::drive(
        service,
        session,
        spec,
        flags,
        mutate::RETRYING,
        &mut |_| Mutation::PollOnce(PollOnceInput { root: root.clone() }),
        &mut std::thread::sleep,
    );
    mutate::finish(
        session.json,
        driven,
        |envelope| render::prepare_output(false, envelope),
        |envelope, supplied| once_output(envelope, supplied, root.to_string_lossy().as_ref()),
    )
}

/// A value of a target's line. A malformed ref's fields are all null.
fn shown(value: Option<&str>) -> String {
    value.map_or_else(|| "none".to_owned(), field)
}

/// The report's fields, then a line per target with the domain's guidance. A
/// slug isn't added: a poll names refs and items as the domain reports them.
fn report(data: &PollMutationDto) -> String {
    let mut out = human_admin::fields_without(data, &["targets"]);
    if data.targets.is_empty() {
        out.push_str("targets: none\n");
    }
    for target in &data.targets {
        out.push_str(&format!(
            "target: {} kind={} item={} disposition={} before={} after={}\n  {}\n",
            shown(target.target.as_deref()),
            shown(target.kind.map(|kind| kind.as_str())),
            shown(target.item_id.as_deref()),
            target.disposition.as_str(),
            shown(target.before_oid.as_deref()),
            shown(target.after_oid.as_deref()),
            target.disposition.guidance(),
        ));
    }
    out
}

/// The note of a stop that takes a new request, by the design's table. A code
/// offering the same request again has none: that request goes on.
fn final_note(envelope: &Envelope<MutationDataDto>) -> Option<&'static str> {
    if envelope
        .recovery
        .iter()
        .any(|action| action.action == RecoveryActionKind::RequestRetry)
    {
        return None;
    }
    // A conflict or a pending local operation answers `recovery_required`
    // with `repo.inspect`: what it waits for isn't this request's.
    let inspect = envelope
        .recovery
        .iter()
        .any(|action| action.action == RecoveryActionKind::RepoInspect);
    match envelope.code {
        ResultCode::HostApprovalRequired | ResultCode::HostReplacementRequired => Some(HOST_NOTE),
        ResultCode::UnlockRequired | ResultCode::UnlockFailed => Some(UNLOCK_NOTE),
        ResultCode::NoSelectedKey | ResultCode::KeyRejected | ResultCode::KeyUnavailable => {
            Some(KEY_NOTE)
        }
        ResultCode::PublicationRemoteRequired => Some(REMOTE_NOTE),
        ResultCode::TransportUnavailable
        | ResultCode::RemoteUnavailable
        | ResultCode::RepositoryInaccessible
        | ResultCode::InvalidRemote
        | ResultCode::ExternalChange
        | ResultCode::Cancelled => Some(NEW_REQUEST_NOTE),
        // Final when nothing above offers a way on: a host that couldn't be
        // verified, an index that can't be read, an internal stop.
        ResultCode::RecoveryRequired | ResultCode::IndexUnavailable | ResultCode::InternalError
            if !inspect =>
        {
            Some(NEW_REQUEST_NOTE)
        }
        _ => None,
    }
}

/// How to find what a `busy` waits for, after C2's line for the same request.
fn busy_route(root: &str) -> String {
    let root = quoted(root);
    format!(
        "next: manyhands-cli operation list --repo {root}\nnext: manyhands-cli operation resume --repo {root} --operation-id <operation-id>\n"
    )
}

/// The human output of a `poll once` answer. `root` is the one the command
/// named, for an answer whose scope holds none.
fn once_output(envelope: &Envelope<MutationDataDto>, supplied: bool, root: &str) -> Output {
    let mut plain = envelope.clone();
    let body = match &envelope.data {
        Some(MutationDataDto::Poll(data)) => {
            plain.data = None;
            report(data)
        }
        _ => String::new(),
    };
    // C2's hint says to run the command again, which a request ID only replays.
    let after = if supplied && envelope.code == ResultCode::TransportUnavailable {
        PASSPHRASE_HINT
    } else {
        ""
    };
    let mut output = render::compact(&plain, false, &body, after, envelope.recovery.clone(), "");
    if envelope.code == ResultCode::Busy {
        let root = envelope.scope.repository.as_deref().unwrap_or(root);
        output.stderr.extend(busy_route(root).into_bytes());
        output
            .stderr
            .extend(render::note_line(BUSY_NOTE).into_bytes());
    }
    if let Some(note) = final_note(envelope) {
        output.stderr.extend(render::note_line(note).into_bytes());
    }
    output
}

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
    policy_toggle(service, session, spec, args, |root| {
        Mutation::PollPause(PollPauseInput { root })
    })
}

pub fn resume(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    policy_toggle(service, session, spec, args, |root| {
        Mutation::PollResume(PollResumeInput { root })
    })
}

/// `poll pause` or `poll resume`: one flag of the policy, set or cleared.
fn policy_toggle(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
    build: fn(PathBuf) -> Mutation,
) -> Output {
    let (flags, root) = match selected(session, spec, args) {
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
    let driven = mutate::drive(
        service,
        session,
        spec,
        flags,
        mutate::RETRYING,
        build,
        &mut std::thread::sleep,
    );
    mutate::finish(
        session.json,
        driven,
        |envelope| render::prepare_output(false, envelope),
        policy_output,
    )
}

/// C2's output, and in human mode a note when the answer's data holds a
/// suspension, or the range an interval is refused for.
fn policy_output(envelope: &Envelope<MutationDataDto>, supplied: bool) -> Output {
    // The CLI checks the interval's type, so an `invalid_input` of `poll
    // configure` is its range.
    if envelope.command == "poll configure" && envelope.code == ResultCode::InvalidInput {
        let mut extra = human_admin::failure_extra(envelope, supplied);
        extra.push_str(RANGE_DETAIL);
        let refusal = refusal_of(envelope, envelope.recovery.clone());
        return render::failure_output(false, &refusal, Some(extra));
    }
    let mut output = render::mutation_output(false, envelope, supplied);
    if let Some(MutationDataDto::PollPolicy(data)) = &envelope.data
        && data.recovery_suspended
    {
        output
            .stderr
            .extend(render::note_line(SUSPENDED_NOTE).into_bytes());
    }
    output
}

#[cfg(test)]
#[path = "polling_tests.rs"]
mod tests;
