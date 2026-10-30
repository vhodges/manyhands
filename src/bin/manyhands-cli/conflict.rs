//! `conflict show` and `conflict resolve`, C4's two verbs on a
//! synchronization's conflict.
//!
//! `conflict show` is a read of the library's `show_conflict` (R-12). It
//! takes no request ID and writes nothing, and its human output
//! (`human_collaboration::conflict_show`) prints no side: a side is content,
//! so only `--json` carries it (design, Human Output).
//!
//! `conflict resolve` takes the observation and a complete `{token,
//! source}` set as input (design, Input). It runs as the other collaboration
//! verbs do: through C2's driver with the bounded retry, inside the Ctrl-C
//! window (`mutate.rs`), and renders through `human_collaboration`. The
//! tokens, the observation and the sources belong to the library, which
//! judges their form and finds the conflict (R-9): the verb adds only the
//! reading of the input.

use clap::ArgMatches;
use manyhands::{
    repository::{
        ConflictDto, ConflictResolutionInput, ConflictResolveInput, Mutation, OperationId,
        ReadError, RepositoryService,
    },
    results::Envelope,
};

use crate::{
    admin,
    catalog::CommandSpec,
    collaboration, dispatch, human_collaboration,
    input::{InputError, InputObject},
    mutate::Session,
    render::Output,
};

pub fn show(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let name = admin::name(spec);
    dispatch::read_output(
        &name,
        session.json,
        conflict(service, &name, args),
        human_collaboration::conflict_show,
    )
}

fn conflict(
    service: &RepositoryService,
    name: &str,
    args: &ArgMatches,
) -> Result<Envelope<ConflictDto>, ReadError> {
    let repo = dispatch::resolve(service, args)?;
    let operation_id = OperationId::parse(
        args.get_one::<String>("operation-id")
            .expect("required operation selector"),
    )
    .map_err(|_| ReadError::invalid_id().with_scope(repo.scope()))?;
    Ok(Envelope::read_success(
        name,
        repo.scope(),
        service.show_conflict(&repo, operation_id)?,
    ))
}

/// The input of `conflict resolve`. It holds the sources, so it derives
/// nothing that could print them.
struct Fields {
    observation: String,
    resolutions: Vec<ConflictResolutionInput>,
}

/// Reads the observation and the resolutions. A token is whatever string
/// the input gives: the library judges its form, and whether it names a path
/// of the live conflict.
fn fields(object: &InputObject) -> Result<Fields, InputError> {
    object.only(&["observation", "resolutions"])?;
    let observation = object.string("observation")?;
    let resolutions = object
        .object_list("resolutions")?
        .iter()
        .map(|resolution| {
            resolution.only(&["token", "source"])?;
            Ok(ConflictResolutionInput {
                token: resolution.string("token")?,
                source: resolution.string("source")?,
            })
        })
        .collect::<Result<Vec<_>, InputError>>()?;
    Ok(Fields {
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
    let (flags, fields, root, operation_id) = match (|| {
        let flags = admin::flags(session, spec, args)?;
        let object = admin::input_object(session, spec, args, flags.request_id)?;
        let fields = fields(&object)
            .map_err(|e| admin::input_failure(session, spec, flags.request_id, e))?;
        let root = admin::mutation_root(session, spec, args, flags.request_id)?;
        let operation_id = collaboration::operation_id(session, spec, args, flags.request_id)?;
        Ok((flags, fields, root, operation_id))
    })() {
        Ok(values) => values,
        Err(out) => return out,
    };
    // The operation is named so that human output can read its conflict
    // when a result needs it (`collaboration::seen`).
    collaboration::run(
        service,
        session,
        spec,
        flags,
        Some(operation_id),
        &mut |_| {
            Mutation::ConflictResolve(ConflictResolveInput {
                root: root.clone(),
                operation_id,
                observation: fields.observation.clone(),
                resolutions: fields
                    .resolutions
                    .iter()
                    .map(|resolution| ConflictResolutionInput {
                        token: resolution.token.clone(),
                        source: resolution.source.clone(),
                    })
                    .collect(),
            })
        },
    )
}

#[cfg(test)]
#[path = "conflict_tests.rs"]
mod tests;
