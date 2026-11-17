//! `context observe-local`, `context show-local`, `context sync-local` and
//! `context resolve-local`.
//!
//! All four are local: none touches a remote, primary or a remote ref. The item's
//! kind comes from the item, and the bin holds no merge logic: what it adds to
//! the library is the reading of what an ID or token names, in the design's
//! required outcomes:
//!
//! - `show-local` and `resolve-local` tell an unknown operation
//!   (`operation_not_found`) and another kind of operation (`invalid_input`) from
//!   a local synchronization, through `show_operation`, before the inspection,
//!   which would answer both `recovery_required`;
//! - `show-local --token` refuses a token the inspection's list shows `read`
//!   would refuse too, as `invalid_input`, with no side read
//!   (`refused_unread`); any other token is the library's to judge;
//! - `resolve-local` refuses up front, from the inspection, an external-only
//!   conflict (`recovery_required`) and a token the live conflict doesn't list
//!   (`unlisted`, `invalid_input`), each with nothing accepted or written.
//!
//! Human output is `human_local_sync.rs`'s.
//!
//! A conflict's sides are content: only `--json` carries them (R-15). A
//! resolution's sources are read from `--input` and handed to the library,
//! which validates the whole set before any write. Nothing here derives
//! `Debug` or prints them.

use clap::ArgMatches;
use manyhands::{
    canonical::ItemId,
    repository::{
        LocalConflictResolutionInput, LocalContextResolveInput, LocalContextSyncEligibility,
        LocalContextSyncInput, LocalContextSyncInspectionDto, LocalContextSyncReason,
        LocalContextSyncState, Mutation, OperationAction, OperationId, ReadError,
        RepositoryService, ResolvedRepository,
    },
    results::{Envelope, ResultCode},
};

use crate::{
    admin, authoring,
    catalog::CommandSpec,
    collaboration, dispatch, fill,
    human_local_sync::{
        data_of, external_then, inspected, local_output, no_context, observation, resolve_way_on,
        show_line, sides, way_on,
    },
    input::{InputError, InputObject},
    mutate::{self, Session},
    render::{self, Output, quoted},
};

const NO_CONTEXT_MESSAGE: &str = "This item has no editing context.";
const TOKEN_JSON_DETAIL: &str =
    "--token reads a conflict's sides, and only --json prints them: add --json.";

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
            Some(TOKEN_JSON_DETAIL.to_owned()),
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
                Some(render::note_line(&note).trim_end().to_owned()),
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
                resolutions: input.resolutions.clone(),
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
