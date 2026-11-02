//! `context observe-local`, `context show-local` and `context sync-local`.
//!
//! All three are local: none touches a remote, primary or a remote ref. The item's
//! kind comes from the item, and the bin holds no merge logic: what it adds to
//! the library is the reading of what an ID or token names, in the design's
//! required outcomes:
//!
//! - the library's inspection answers `recovery_required` for an ID with no local
//!   synchronization, as an execution must. `show_operation` tells an unknown ID
//!   (`operation_not_found`) and another operation's (`invalid_input`) from it;
//! - the library answers a well-formed token that names no path of the live
//!   conflict `external_change`. Where the inspection's list shows `read` would
//!   refuse it too, such a token is `invalid_input` with no side read; any other
//!   token is the library's to judge (`refused_unread`).
//!
//! A conflict's sides are content: only `--json` carries them (R-15).

use clap::ArgMatches;
use manyhands::{
    canonical::ItemId,
    repository::{
        ClosureState, ItemContextKind, ItemDto, LocalContextConflictContentDto,
        LocalContextSyncInput, LocalContextSyncInspectionDto, LocalContextSyncMutationDto,
        LocalContextSyncNextAction, LocalContextSyncObservationDto, LocalContextSyncReason,
        LocalContextSyncState, Mutation, MutationDataDto, OperationAction, OperationId, ReadError,
        RepositoryService, ResolvedRepository,
    },
    results::{Envelope, ResultCode},
};

use crate::{
    admin, authoring,
    catalog::CommandSpec,
    collaboration, dispatch, fill, human_admin, human_lifecycle,
    mutate::{self, Driven, Plan, Session},
    render::{self, Human, Output, field, note_lines, quoted},
};

const TOKEN_NEEDS_JSON: &str =
    "--token reads a conflict's sides, and only --json prints them: add --json.";
const INPUT_NOTE: &str = "context sync-local's --input takes this as {\"observation\": \"...\"}.";
const SIDES_NOTE: &str = "context show-local --token <token> --json reads a path's sides.";
const DIRTY_NOTE: &str = "commit or save what the context holds, then observe it again.";
const OBSERVE_NOTE: &str =
    "sending this observation again gets the same answer: observe again, and use a new request.";
const RESOLVE_NOTE: &str = "<new-request-id> is a placeholder to fill in, and --input - takes the observation and a source for each token that context show-local prints.";
const IDENTITY_NOTE: &str =
    "once an identity is set, send this same request again: a merge commit needs one.";
const EXTERNAL_NOTE: &str = "Manyhands can't resolve this: it takes a repair outside it, with Git.";

fn observe_line(root: &str, item: &str) -> String {
    format!(
        "next: manyhands-cli context observe-local --repo {} --id {}\n",
        quoted(root),
        field(item)
    )
}

/// What an observation's refusal adds: a save, when the item's effective copy
/// is primary's and a save of it is allowed, as lifecycle's `ContextMissing`
/// way out. A context that has lost the item, or a closed ticket, gets none.
fn no_context(item: &ItemDto, root: &str, id: &str) -> Option<String> {
    let closed = item
        .closure
        .as_ref()
        .is_some_and(|closure| closure.state == ClosureState::Closed);
    (item.context.kind == ItemContextKind::Primary && !closed).then(|| {
        format!(
            "next: {}\n{}",
            human_lifecycle::save_line(item.kind, root, id),
            note_lines(&[human_lifecycle::NO_CONTEXT_NOTE.to_owned()])
        )
    })
}

pub fn observe(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let name = admin::name(spec);
    let failed = |error: ReadError, extra: Option<String>| {
        render::failure_output(session.json, &error.to_envelope(&name), extra)
    };
    let (repo, id) = match resolved(service, args) {
        Ok(values) => values,
        Err(error) => return failed(error, None),
    };
    // Not found here is an unknown item, with nothing to offer.
    let item = match service.show_item(&repo, &id) {
        Ok(item) => item,
        Err(error) => return failed(error, None),
    };
    let mut scope = repo.scope();
    scope.item_id = Some(id.to_string());
    match service.observe_local_context_sync(&repo, collaboration::authoring_kind(item.kind), &id) {
        Ok(data) => render::envelope_output(
            session.json,
            &Envelope::read_success(&name, scope, data),
            observation,
        ),
        Err(error) if error.code() == ResultCode::ItemNotFound => {
            let root = repo.root().to_string_lossy();
            let extra = no_context(&item, &root, &id.to_string());
            failed(error, extra)
        }
        Err(error) => failed(error, None),
    }
}

fn resolved(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<(ResolvedRepository, ItemId), ReadError> {
    let repo = dispatch::resolve(service, args)?;
    let id = dispatch::item_id(args, &repo)?;
    Ok((repo, id))
}

fn observation(data: &LocalContextSyncObservationDto) -> Human {
    Human {
        stdout: human_admin::fields(data),
        notes: vec![INPUT_NOTE.to_owned()],
    }
}

pub fn show(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let name = admin::name(spec);
    let token = args.get_one::<String>("token");
    if token.is_some() && !session.json {
        let error = ReadError::new(ResultCode::UsageError);
        return render::failure_output(
            false,
            &error.to_envelope(&name),
            Some(TOKEN_NEEDS_JSON.to_owned()),
        );
    }
    let failed = |error: ReadError, extra: Option<String>| {
        render::failure_output(session.json, &error.to_envelope(&name), extra)
    };
    let repo = match dispatch::resolve(service, args) {
        Ok(repo) => repo,
        Err(error) => return failed(error, None),
    };
    let id = match dispatch::operation_id(args, &repo) {
        Ok(id) => id,
        Err(error) => return failed(error, None),
    };
    if let Err(refused) = is_a_local_sync(service, &repo, id) {
        return failed(refused.error, refused.extra);
    }
    let invalid = || ReadError::new(ResultCode::InvalidInput).with_scope(repo.scope());
    let inspection = service
        .inspect_local_context_sync(&repo, id)
        .map_err(|error| error.with_scope(repo.scope()));
    let Some(token) = token else {
        return dispatch::read_output(
            &name,
            session.json,
            inspection.map(|data| Envelope::read_success(&name, repo.scope(), data)),
            inspected,
        );
    };
    let content = inspection.and_then(|view| {
        if refused_unread(&view, token) {
            return Err(invalid());
        }
        service
            .read_local_context_conflict(&repo, id, token)
            .map_err(|error| error.with_scope(repo.scope()))
    });
    dispatch::read_output(
        &name,
        true,
        content.map(|data| Envelope::read_success(&name, repo.scope(), data)),
        sides,
    )
}

/// Whether a token is one `conflict::read` refuses too, so that no side is
/// read for it: the inspection lists the live conflict's paths and not this
/// token, or the operation has finished and lists none. A listed token is the
/// library's, readable or not, and so is any state that lists no paths while
/// it is going on, since `read` judges those itself.
fn refused_unread(view: &LocalContextSyncInspectionDto, token: &str) -> bool {
    if view.paths.iter().any(|path| path.token == token) {
        return false;
    }
    !view.paths.is_empty()
        || matches!(
            view.state,
            LocalContextSyncState::Complete
                | LocalContextSyncState::Cancelled
                | LocalContextSyncState::Abandoned
        )
}

/// A token's sides print with `--json` only, which `show` holds to; in human
/// mode this renderer is never reached, and would print no side.
fn sides(_: &LocalContextConflictContentDto) -> Human {
    Human {
        stdout: String::new(),
        notes: Vec::new(),
    }
}

struct Refused {
    error: ReadError,
    extra: Option<String>,
}

/// An unknown ID is `operation_not_found`, and another operation's is
/// `invalid_input`; a synchronization's conflict is `conflict show`'s.
fn is_a_local_sync(
    service: &RepositoryService,
    repo: &ResolvedRepository,
    id: OperationId,
) -> Result<(), Refused> {
    let operation = service
        .show_operation(repo, id)
        .map_err(|error| Refused { error, extra: None })?;
    if operation.action == OperationAction::ContextSyncLocal {
        return Ok(());
    }
    // Only a synchronization's conflict is `conflict show`'s.
    let synchronization = matches!(
        operation.action,
        OperationAction::SynchronizeContext | OperationAction::SynchronizePrimary
    );
    let extra = synchronization.then(|| {
        format!(
            "next: manyhands-cli conflict show --repo {} --operation-id {id}\n",
            quoted(&repo.root().to_string_lossy())
        )
    });
    Err(Refused {
        error: ReadError::new(ResultCode::InvalidInput).with_scope(repo.scope()),
        extra,
    })
}

/// The inspection's fields, then a line per conflicted path. No side.
fn inspected(data: &LocalContextSyncInspectionDto) -> Human {
    let mut stdout = human_admin::fields_without(data, &["paths"]);
    for path in &data.paths {
        stdout.push_str(&format!(
            "path: {} token={} content_available={}\n",
            field(&path.display_path),
            field(&path.token),
            path.content_available
        ));
    }
    Human {
        stdout,
        notes: if data.paths.is_empty() {
            Vec::new()
        } else {
            vec![SIDES_NOTE.to_owned()]
        },
    }
}

pub fn sync(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let selected = || -> Result<_, Output> {
        let flags = admin::flags(session, spec, args)?;
        let object = admin::input_object(session, spec, args, flags.request_id)?;
        let observation = object
            .only(&["observation"])
            .and_then(|()| object.string("observation"))
            .map_err(|error| admin::input_failure(session, spec, flags.request_id, error))?;
        let root = admin::mutation_root(session, spec, args, flags.request_id)?;
        let id = authoring::item_id(session, spec, args, flags.request_id)?;
        let (_, item) = fill::read_any_item(service, session, spec, &root, &id, flags.request_id)?;
        Ok((
            flags,
            observation,
            root,
            id,
            collaboration::authoring_kind(item.kind),
        ))
    };
    let (flags, observation, root, id, kind) = match selected() {
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
        &mut |_| {
            Mutation::LocalContextSync(LocalContextSyncInput {
                root: root.clone(),
                kind,
                id: id.clone(),
                observation: observation.clone(),
            })
        },
        &mut std::thread::sleep,
    ) {
        Driven::Answered(output) => output,
        Driven::Prepared(envelope) => render::prepare_output(session.json, &envelope),
        Driven::Executed { envelope, supplied } if session.json => {
            render::mutation_output(true, &envelope, supplied)
        }
        Driven::Executed { envelope, supplied } => {
            let mut output = render::mutation_output(false, &envelope, supplied);
            output
                .stderr
                .extend(way_on(&envelope, &root.to_string_lossy(), &id.to_string()).into_bytes());
            output
        }
    }
}

fn data_of(envelope: &Envelope<MutationDataDto>) -> Option<&LocalContextSyncMutationDto> {
    match &envelope.data {
        Some(MutationDataDto::LocalContextSync(data)) => Some(data),
        _ => None,
    }
}

/// What a stop's human lines add to C2's, by the required outcomes' last column.
fn way_on(envelope: &Envelope<MutationDataDto>, root: &str, item: &str) -> String {
    let notes = |note: &str| note_lines(&[note.to_owned()]);
    match envelope.code {
        ResultCode::InvalidInput => return observe_line(root, item),
        ResultCode::ExternalChange => {
            return format!("{}{}", observe_line(root, item), notes(OBSERVE_NOTE));
        }
        ResultCode::WorktreeNotClean => {
            return format!("{}{}", observe_line(root, item), notes(DIRTY_NOTE));
        }
        ResultCode::IdentityRequired => return notes(IDENTITY_NOTE),
        _ => {}
    }
    let Some(data) = data_of(envelope) else {
        return String::new();
    };
    match data.next_action {
        LocalContextSyncNextAction::Resolve => match data.operation_id.as_deref() {
            Some(operation) => {
                let (root, operation) = (quoted(root), field(operation));
                format!(
                    "next: manyhands-cli context show-local --repo {root} --operation-id {operation}\nnext: manyhands-cli context resolve-local --repo {root} --operation-id {operation} --input - --request-id <new-request-id>\n{}",
                    notes(RESOLVE_NOTE)
                )
            }
            None => String::new(),
        },
        // The refusals the gate leaves to a repair outside (gate 10): the
        // library offers nothing for two of them, and `external_repair` for
        // the third. A closure's or a lost authority's `external_repair` has
        // its code's way on, so it gets no such note.
        _ if matches!(
            data.reason,
            LocalContextSyncReason::UnrelatedHistories
                | LocalContextSyncReason::ImmutableClosureDisagreement
                | LocalContextSyncReason::UnsupportedConflict
        ) =>
        {
            notes(EXTERNAL_NOTE)
        }
        _ => String::new(),
    }
}

#[cfg(test)]
#[path = "local_sync_tests.rs"]
mod tests;
