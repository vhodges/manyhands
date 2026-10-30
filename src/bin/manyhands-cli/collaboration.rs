//! The collaboration verbs that take no input: `item sync`, `repo sync`,
//! `operation resume` and `operation abandon`. Each is C2's driver around
//! one library mutation. The stoppable ones run inside the Ctrl-C window
//! (`mutate.rs`), and their results are rendered by
//! `human_collaboration`.

use std::path::{Path, PathBuf};

use clap::ArgMatches;
use manyhands::{
    canonical::ItemId,
    repository::{
        AuthoringKind, ItemDtoKind, ItemSyncInput, Mutation, MutationDataDto,
        OperationAbandonInput, OperationId, OperationResumeInput, ReadError, RepoSyncInput,
        RepositoryService, RequestId,
    },
    results::{Effects, Envelope, RecoveryActionKind, ResultCode},
};

use crate::{
    admin,
    catalog::CommandSpec,
    human_collaboration::{self, Seen},
    mutate::{self, Driven, Extras, MutationFlags, Plan, Session},
    render::{self, Output},
};

pub fn item_sync(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let (flags, root, id) = match (|| {
        let flags = admin::flags(session, spec, args)?;
        let root = admin::mutation_root(session, spec, args, flags.request_id)?;
        let id = item_id(session, spec, args, flags.request_id)?;
        Ok((flags, root, id))
    })() {
        Ok(values) => values,
        Err(out) => return out,
    };
    // R-2: one read gives the item's kind before the request is built. Its
    // refusal is the verb's, as C3's fill read's is.
    let kind = match item_kind(service, session, spec, &root, &id, flags.request_id) {
        Ok(kind) => kind,
        Err(out) => return out,
    };
    run(service, session, spec, flags, None, &mut |_| {
        Mutation::ItemSync(ItemSyncInput {
            root: root.clone(),
            kind,
            id: id.clone(),
        })
    })
}

pub fn repo_sync(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let (flags, root) = match (|| {
        let flags = admin::flags(session, spec, args)?;
        let root = admin::mutation_root(session, spec, args, flags.request_id)?;
        Ok((flags, root))
    })() {
        Ok(values) => values,
        Err(out) => return out,
    };
    run(service, session, spec, flags, None, &mut |_| {
        Mutation::RepoSync(RepoSyncInput { root: root.clone() })
    })
}

pub fn operation_resume(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let (flags, root, operation_id) = match operation(session, spec, args) {
        Ok(values) => values,
        Err(out) => return out,
    };
    run(
        service,
        session,
        spec,
        flags,
        Some(operation_id),
        &mut |_| {
            Mutation::OperationResume(OperationResumeInput {
                root: root.clone(),
                operation_id,
            })
        },
    )
}

/// C2's confirmed flow: `--prepare`, `--confirm`, or consent at a terminal.
pub fn operation_abandon(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let (flags, root, operation_id) = match operation(session, spec, args) {
        Ok(values) => values,
        Err(out) => return out,
    };
    run(service, session, spec, flags, None, &mut |_| {
        Mutation::OperationAbandon(OperationAbandonInput {
            root: root.clone(),
            operation_id,
        })
    })
}

/// The flags, the root and the operation of a verb that names one.
fn operation(
    session: &Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Result<(MutationFlags, PathBuf, OperationId), Output> {
    let flags = admin::flags(session, spec, args)?;
    let root = admin::mutation_root(session, spec, args, flags.request_id)?;
    let operation_id = OperationId::parse(
        args.get_one::<String>("operation-id")
            .expect("required operation selector"),
    )
    .map_err(|_| invalid_id(session, spec, flags.request_id))?;
    Ok((flags, root, operation_id))
}

/// Runs the mutation through C2's driver, with its bounded retry (R-23),
/// and renders the result. `resumed` is the operation a resume names.
fn run(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    flags: MutationFlags,
    resumed: Option<OperationId>,
    build: &mut dyn FnMut(&Extras) -> Mutation,
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
        Driven::Executed { envelope, supplied } if session.json => {
            render::mutation_output(true, &envelope, supplied)
        }
        Driven::Executed { envelope, supplied } => {
            let seen = seen(service, &envelope, resumed);
            human_collaboration::output(&envelope, supplied, &seen)
        }
    }
}

/// What human output needs that the envelope does not say: whether the
/// request an `operation resume` was refused for owns a remote resolution
/// whose paths are written and not committed (design, Resume). The
/// conflict read says so; when it cannot be read, nothing is added.
fn seen(
    service: &RepositoryService,
    envelope: &Envelope<MutationDataDto>,
    resumed: Option<OperationId>,
) -> Seen {
    let (Some(operation), Some(request), Some(root)) = (
        resumed,
        original_request(envelope),
        envelope.scope.repository.as_deref(),
    ) else {
        return Seen::default();
    };
    let conflict = service
        .resolve_repository(Path::new(root))
        .and_then(|repo| service.show_conflict(&repo, operation));
    Seen {
        supersedable: conflict.is_ok_and(|conflict| {
            supersedable(conflict.historical, conflict.request_id.as_deref(), request)
        }),
    }
}

/// The other request an `original_request_required` names.
fn original_request(envelope: &Envelope<MutationDataDto>) -> Option<&str> {
    if envelope.code != ResultCode::OriginalRequestRequired {
        return None;
    }
    envelope
        .recovery
        .iter()
        .filter(|action| action.action == RecoveryActionKind::RequestRetry)
        .filter_map(|action| action.arguments.get("request_id")?.as_str())
        .find(|request| envelope.request_id.as_deref() != Some(*request))
}

/// Whether a conflict whose unfinished attempt is `attempt`'s, and has
/// written paths when `historical` (R-39), is the resolution of `request`.
fn supersedable(historical: bool, attempt: Option<&str>, request: &str) -> bool {
    historical && attempt == Some(request)
}

fn item_id(
    session: &Session,
    spec: &CommandSpec,
    args: &ArgMatches,
    request_id: Option<RequestId>,
) -> Result<ItemId, Output> {
    args.get_one::<String>("id")
        .expect("required item ID")
        .parse::<ItemId>()
        .map_err(|_| invalid_id(session, spec, request_id))
}

fn invalid_id(session: &Session, spec: &CommandSpec, request_id: Option<RequestId>) -> Output {
    mutate::refusal(
        session,
        &admin::name(spec),
        request_id,
        ResultCode::InvalidId,
        None,
    )
}

fn authoring_kind(kind: ItemDtoKind) -> AuthoringKind {
    match kind {
        ItemDtoKind::Document => AuthoringKind::Document,
        ItemDtoKind::Ticket => AuthoringKind::Ticket,
    }
}

/// The kind of the item `id`, read with `show_item` (R-2, fact 37). A
/// refusal of the read is the verb's, with its request ID and no effect.
fn item_kind(
    service: &RepositoryService,
    session: &Session,
    spec: &CommandSpec,
    root: &Path,
    id: &ItemId,
    request_id: Option<RequestId>,
) -> Result<AuthoringKind, Output> {
    let failure = |error: ReadError| {
        let mut envelope = error.to_envelope(admin::name(spec));
        envelope.request_id = request_id.map(|id| id.to_string());
        envelope.effects = Effects::not_requested();
        render::failure_output(session.json, &envelope, None)
    };
    let repo = service.resolve_repository(root).map_err(failure)?;
    let item = service.show_item(&repo, id).map_err(failure)?;
    Ok(authoring_kind(item.kind))
}

#[cfg(test)]
#[path = "collaboration_tests.rs"]
mod tests;
