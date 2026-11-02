//! `context observe-local`, `context show-local`, `context sync-local` and
//! `context resolve-local`.
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
//! A conflict's sides are content: only `--json` carries them (R-15). A
//! resolution's sources are read from `--input` and handed to the library,
//! which validates the whole set before any write. Nothing here derives
//! `Debug` or prints them.

use clap::ArgMatches;
use manyhands::{
    canonical::ItemId,
    repository::{
        ClosureState, ItemContextKind, ItemDto, LocalConflictResolutionInput,
        LocalContextConflictContentDto, LocalContextResolveInput, LocalContextSyncEligibility,
        LocalContextSyncInput, LocalContextSyncInspectionDto, LocalContextSyncMutationDto,
        LocalContextSyncNextAction, LocalContextSyncObservationDto, LocalContextSyncReason,
        LocalContextSyncState, Mutation, MutationDataDto, OperationAction, OperationId, ReadError,
        RepositoryService, ResolvedRepository,
    },
    results::{Envelope, RecoveryActionKind, ResultCode},
};

use crate::{
    admin, authoring,
    catalog::CommandSpec,
    collaboration, dispatch, fill, human_admin, human_lifecycle,
    input::{InputError, InputObject},
    mutate::{self, Session},
    render::{self, Human, Output, field, note_lines, quoted},
};

const NO_CONTEXT_MESSAGE: &str = "This item has no editing context.";
const TOKEN_NEEDS_JSON: &str =
    "--token reads a conflict's sides, and only --json prints them: add --json.";
const INPUT_NOTE: &str = "context sync-local's --input takes this as {\"observation\": \"...\"}.";
const SIDES_NOTE: &str = "context show-local --token <token> --json reads a path's sides.";
const DIRTY_NOTE: &str =
    "commit or save what the context holds, then observe again, and use a new request.";
const MALFORMED_NOTE: &str =
    "this observation can't be used: observe again, and use a new request.";
const OBSERVE_NOTE: &str =
    "sending this observation again gets the same answer: observe again, and use a new request.";
const RESOLVE_NOTE: &str = "<new-request-id> is a placeholder to fill in, and --input - takes the observation and a source for each token that context show-local prints.";
const IDENTITY_NOTE: &str =
    "once an identity is set, send this same request again: a merge commit needs one.";
const ORIGINAL_NOTE: &str = "this resolution is the original request's to finish: send that request again, with the same input.";
const EXTERNAL_NOTE: &str = "Manyhands can't resolve this: it takes a repair outside it, with Git.";
/// What follows the repair of a refusal that kept nothing to go on with.
const EXTERNAL_REFUSED_NOTE: &str = "Manyhands can't resolve this: it takes a repair outside it, with Git. After the repair, observe again, and use a new request.";

/// What follows the repair of a conflict a request holds: that request again.
fn external_then(holder: &str, request: &str) -> String {
    format!(
        "{EXTERNAL_NOTE} After the repair, send {holder} again ({}).",
        field(request)
    )
}

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
            let mut envelope = error.to_envelope(&name);
            // The item exists: the read found no editing context of it.
            if !session.json {
                envelope.message = NO_CONTEXT_MESSAGE.to_owned();
            }
            render::failure_output(session.json, &envelope, extra)
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

/// Whether the live conflict's paths are listed and this token isn't among
/// them: `resolve-local`'s refusal, which leaves a finished operation, with no
/// paths listed, to the library.
fn unlisted(view: &LocalContextSyncInspectionDto, token: &str) -> bool {
    !view.paths.is_empty() && !view.paths.iter().any(|path| path.token == token)
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
    let driven = mutate::drive(
        service,
        session,
        spec,
        flags,
        mutate::RETRYING,
        &mut |_| {
            Mutation::LocalContextSync(LocalContextSyncInput {
                root: root.clone(),
                kind,
                id: id.clone(),
                observation: observation.clone(),
            })
        },
        &mut std::thread::sleep,
    );
    mutate::finish(
        session.json,
        driven,
        |envelope| render::prepare_output(false, envelope),
        |envelope, supplied| {
            let way = way_on(envelope, &root.to_string_lossy(), &id.to_string());
            let mut output = local_output(envelope, supplied, way.replaces_next_action);
            output.stderr.extend(way.lines.into_bytes());
            output
        },
    )
}

/// The input of `context resolve-local`. It holds the sources, so it derives
/// nothing that could print them.
struct Resolution {
    observation: String,
    resolutions: Vec<LocalConflictResolutionInput>,
}

/// Reads the observation and the `{token, source}` pairs. The library judges
/// each token's form, the set's completeness and each source.
fn resolution(object: &InputObject) -> Result<Resolution, InputError> {
    object.only(&["observation", "resolutions"])?;
    let observation = object.string("observation")?;
    let resolutions = object
        .object_list("resolutions")?
        .iter()
        .map(|pair| {
            pair.only(&["token", "source"])?;
            Ok(LocalConflictResolutionInput {
                token: pair.string("token")?,
                source: pair.string("source")?,
            })
        })
        .collect::<Result<Vec<_>, InputError>>()?;
    Ok(Resolution {
        observation,
        resolutions,
    })
}

pub fn resolve(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let selected = || -> Result<_, Output> {
        let flags = admin::flags(session, spec, args)?;
        let object = admin::input_object(session, spec, args, flags.request_id)?;
        let input = resolution(&object)
            .map_err(|error| admin::input_failure(session, spec, flags.request_id, error))?;
        let root = admin::mutation_root(session, spec, args, flags.request_id)?;
        let operation = collaboration::operation_id(session, spec, args, flags.request_id)?;
        Ok((flags, input, root, operation))
    };
    let (flags, input, root, operation) = match selected() {
        Ok(values) => values,
        Err(out) => return out,
    };
    let repo = service.resolve_repository(&root).ok();
    let inspect = || {
        repo.as_ref()
            .and_then(|repo| service.inspect_local_context_sync(repo, operation).ok())
    };
    // Two refusals before anything is accepted or written, read from the
    // inspection. A prepare is the library's (`not_confirmable`), and so is a
    // request it may hold a record of, to replay or mismatch; without an
    // inspection the library judges the whole input.
    let recorded = flags
        .request_id
        .is_some_and(|id| service.show_request(id).code != ResultCode::RequestNotFound);
    if !flags.prepare
        && !recorded
        && let Some(view) = inspect()
    {
        let name = admin::name(spec);
        // An external-only conflict: no resolution here can pass, so none is
        // offered, and no `show-local` line loops back to it.
        if view.eligibility == Some(LocalContextSyncEligibility::ExternalResolutionRequired) {
            let note = external_then("the synchronization's request", &view.starting_request_id);
            return mutate::refusal(
                session,
                &name,
                flags.request_id,
                ResultCode::RecoveryRequired,
                Some(note_lines(&[note]).trim_end().to_owned()),
            );
        }
        // A token that names no path of the live conflict, which
        // `conflict::read` refuses too. A finished operation lists none, and
        // the library answers it (`terminal_unlinked`).
        if input
            .resolutions
            .iter()
            .any(|pair| unlisted(&view, &pair.token))
        {
            let root = root.to_string_lossy();
            return mutate::refusal(
                session,
                &name,
                flags.request_id,
                ResultCode::InvalidInput,
                Some(
                    show_line(&root, &operation.to_string())
                        .trim_end()
                        .to_owned(),
                ),
            );
        }
    }
    let driven = mutate::drive(
        service,
        session,
        spec,
        flags,
        mutate::RETRYING,
        &mut |_| {
            Mutation::LocalContextResolve(LocalContextResolveInput {
                root: root.clone(),
                operation_id: operation,
                observation: input.observation.clone(),
                resolutions: input
                    .resolutions
                    .iter()
                    .map(|pair| LocalConflictResolutionInput {
                        token: pair.token.clone(),
                        source: pair.source.clone(),
                    })
                    .collect(),
            })
        },
        &mut std::thread::sleep,
    );
    mutate::finish(
        session.json,
        driven,
        |envelope| render::prepare_output(false, envelope),
        |envelope, supplied| {
            // The original request, as the data names it, or the inspection
            // does after a stop that names none (the gate: it is resent).
            let original = data_of(envelope)
                .and_then(|data| data.required_request_id.clone())
                .or_else(|| {
                    matches!(
                        envelope.code,
                        ResultCode::ExternalChange | ResultCode::OriginalRequestRequired
                    )
                    .then(inspect)
                    .flatten()
                    .filter(|view| {
                        view.reason == LocalContextSyncReason::OriginalResolutionRequired
                    })
                    .and_then(|view| view.required_request_id)
                });
            let way = resolve_way_on(
                envelope,
                &root.to_string_lossy(),
                &operation.to_string(),
                original.as_deref(),
            );
            let mut output = local_output(envelope, supplied, way.replaces_next_action);
            output.stderr.extend(way.lines.into_bytes());
            output
        },
    )
}

fn show_line(root: &str, operation: &str) -> String {
    format!(
        "next: manyhands-cli context show-local --repo {} --operation-id {}\n",
        quoted(root),
        field(operation)
    )
}

/// The lines a verb adds to a result's, and whether one of them, a line to
/// look again, replaces the data's `next action`: there, looking again is the
/// way on, not a repair outside.
struct WayOn {
    lines: String,
    replaces_next_action: bool,
}

impl WayOn {
    fn adds(lines: String) -> Self {
        Self {
            lines,
            replaces_next_action: false,
        }
    }

    fn looks_again(lines: String) -> Self {
        Self {
            lines,
            replaces_next_action: true,
        }
    }
}

/// What a resolution's stop adds, by the required outcomes' last column. With
/// an original request to finish the resolution, that request is the way on,
/// as `request.retry`, unless C2's lines already name it.
fn resolve_way_on(
    envelope: &Envelope<MutationDataDto>,
    root: &str,
    operation: &str,
    original: Option<&str>,
) -> WayOn {
    if let Some(original) = original.filter(|id| envelope.request_id.as_deref() != Some(*id)) {
        let named = envelope.recovery.iter().any(|action| {
            action.action == RecoveryActionKind::RequestRetry
                && action.arguments.get("request_id").and_then(|v| v.as_str()) == Some(original)
        });
        let retry = if named {
            String::new()
        } else {
            format!("next: request.retry request_id={}\n", field(original))
        };
        return WayOn::adds(format!(
            "{retry}{}",
            note_lines(&[ORIGINAL_NOTE.to_owned()])
        ));
    }
    match envelope.code {
        ResultCode::InvalidInput | ResultCode::ExternalChange => {
            WayOn::looks_again(show_line(root, operation))
        }
        ResultCode::OriginalRequestRequired => WayOn::adds(note_lines(&[ORIGINAL_NOTE.to_owned()])),
        _ => WayOn::adds(String::new()),
    }
}

/// A local synchronization's result in human mode: C2's lines, with the
/// data's non-empty fields only, as `human_collaboration`'s compact lines do,
/// and no `operation:` line when the data names no operation (a refusal with
/// no owner). `reobserve` drops a `next action` that the verb's own line
/// replaces (`WayOn`).
fn local_output(envelope: &Envelope<MutationDataDto>, supplied: bool, reobserve: bool) -> Output {
    let mut plain = envelope.clone();
    plain.data = None;
    let mut body = String::new();
    if let Some(data) = data_of(envelope) {
        if data.operation_id.is_none() {
            plain.operation_id = None;
        }
        if let Ok(serde_json::Value::Object(mut fields)) = serde_json::to_value(data) {
            fields.remove("type");
            if reobserve {
                fields.remove("next_action");
            }
            fields.retain(|_, value| {
                !matches!(
                    value,
                    serde_json::Value::Null | serde_json::Value::Bool(false)
                ) && value.as_str() != Some("none")
            });
            body = human_admin::fields(&serde_json::Value::Object(fields));
        }
    }
    render::compact(&plain, supplied, &body, "", plain.recovery.clone(), "")
}

fn data_of(envelope: &Envelope<MutationDataDto>) -> Option<&LocalContextSyncMutationDto> {
    match &envelope.data {
        Some(MutationDataDto::LocalContextSync(data)) => Some(data),
        _ => None,
    }
}

/// What a stop's human lines add to C2's, by the required outcomes' last column.
fn way_on(envelope: &Envelope<MutationDataDto>, root: &str, item: &str) -> WayOn {
    let notes = |note: &str| note_lines(&[note.to_owned()]);
    let again =
        |note: &str| WayOn::looks_again(format!("{}{}", observe_line(root, item), notes(note)));
    match envelope.code {
        ResultCode::InvalidInput => return again(MALFORMED_NOTE),
        ResultCode::ExternalChange => return again(OBSERVE_NOTE),
        ResultCode::WorktreeNotClean => return again(DIRTY_NOTE),
        ResultCode::IdentityRequired => return WayOn::adds(notes(IDENTITY_NOTE)),
        _ => {}
    }
    let Some(data) = data_of(envelope) else {
        return WayOn::adds(String::new());
    };
    WayOn::adds(match data.next_action {
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
            match (&data.operation_id, envelope.request_id.as_deref()) {
                (Some(_), Some(request)) => notes(&external_then("this same request", request)),
                _ => notes(EXTERNAL_REFUSED_NOTE),
            }
        }
        _ => String::new(),
    })
}

#[cfg(test)]
#[path = "local_sync_tests.rs"]
mod tests;
