use crate::{
    catalog::CommandSpec,
    input::{self, InputError, InputObject, Tristate},
    mutate::{self, MutationFlags, Plan, Session},
    render::Output,
};
use clap::ArgMatches;
use manyhands::{
    repository::{
        CommitIdentity, ConfirmationId, InitialsChange, Mutation, RemoteAddInput,
        RemoteRemoveInput, RemoteSelectInput, RepoCreateInput, RepoEnableInput,
        RepoIdentitySetInput, RepoRemoveInput, RepositoryService, RequestId,
    },
    results::ResultCode,
};
use std::path::PathBuf;

fn name(spec: &CommandSpec) -> String {
    format!("{} {}", spec.resource, spec.verb)
}

// Session/spec context is needed to render the first failure in the caller's
// mode, retaining a valid request ID if a later selector is malformed.
pub fn flags(
    session: &Session,
    spec: &CommandSpec,
    args: &ArgMatches,
) -> Result<MutationFlags, Output> {
    let request_id = request_id(session, spec, args)?;
    let confirm = args
        .try_get_one::<String>("confirm")
        .ok()
        .flatten()
        .map(|text| ConfirmationId::parse(text))
        .transpose()
        .map_err(|_| {
            mutate::refusal(
                session,
                &name(spec),
                request_id,
                ResultCode::InvalidId,
                None,
            )
        })?;
    Ok(MutationFlags {
        request_id,
        confirm,
        prepare: args
            .try_get_one::<bool>("prepare")
            .ok()
            .flatten()
            .copied()
            .unwrap_or(false),
    })
}
pub fn request_id(
    session: &Session,
    spec: &CommandSpec,
    args: &ArgMatches,
) -> Result<Option<RequestId>, Output> {
    args.get_one::<String>("request-id")
        .map(|text| RequestId::parse(text))
        .transpose()
        .map_err(|_| mutate::refusal(session, &name(spec), None, ResultCode::InvalidId, None))
}
pub fn input_object(
    session: &Session,
    spec: &CommandSpec,
    args: &ArgMatches,
    request_id: Option<RequestId>,
) -> Result<InputObject, Output> {
    let cwd = std::env::current_dir()
        .map_err(|_| input_failure(session, spec, request_id, InputError::Unreadable))?;
    input::read(
        args.get_one::<String>("input").expect("required input"),
        &cwd,
        &mut std::io::stdin().lock(),
    )
    .map_err(|e| input_failure(session, spec, request_id, e))
}
fn input_failure(
    session: &Session,
    spec: &CommandSpec,
    request_id: Option<RequestId>,
    error: InputError,
) -> Output {
    mutate::refusal(
        session,
        &name(spec),
        request_id,
        ResultCode::InvalidInput,
        Some(error.detail()),
    )
}
pub fn mutation_root(
    session: &Session,
    spec: &CommandSpec,
    args: &ArgMatches,
    request_id: Option<RequestId>,
) -> Result<PathBuf, Output> {
    let root = PathBuf::from(args.get_one::<String>("repo").expect("required repository"));
    if root.is_absolute() {
        Ok(root)
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(root))
            .map_err(|_| {
                mutate::refusal(
                    session,
                    &name(spec),
                    request_id,
                    ResultCode::InvalidPath,
                    None,
                )
            })
    }
}
struct IdentityInput {
    name: String,
    email: String,
    initials: InitialsChange,
}
fn identity_input(input: &InputObject) -> Result<IdentityInput, InputError> {
    input.only(&["name", "email", "initials"])?;
    Ok(IdentityInput {
        name: input.string("name")?,
        email: input.string("email")?,
        initials: match input.tristate("initials")? {
            Tristate::Missing => InitialsChange::Unchanged,
            Tristate::Null => InitialsChange::Clear,
            Tristate::Value(s) => InitialsChange::Set(s),
        },
    })
}
pub fn repo_identity_set(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let result = (|| {
        let flags = flags(session, spec, args)?;
        let object = input_object(session, spec, args, flags.request_id)?;
        let input = identity_input(&object)
            .map_err(|e| input_failure(session, spec, flags.request_id, e))?;
        let root = mutation_root(session, spec, args, flags.request_id)?;
        Ok((flags, input, root))
    })();
    let (flags, input, root) = match result {
        Ok(values) => values,
        Err(out) => return out,
    };
    mutate::run(
        service,
        session,
        spec,
        flags,
        Plan {
            identity_prompt: false,
            retry: true,
        },
        &mut |_| {
            Mutation::RepoIdentitySet(RepoIdentitySetInput {
                root: root.clone(),
                name: input.name.clone(),
                email: input.email.clone(),
                initials: input.initials.clone(),
            })
        },
        &mut std::thread::sleep,
    )
}
struct RepositoryInput {
    primary_branch: String,
    identity: Option<CommitIdentity>,
    slug_prefix: Option<String>,
}

fn repository_input(input: &InputObject) -> Result<RepositoryInput, InputError> {
    input.only(&["primary_branch", "identity", "ticket_slug_prefix"])?;
    let primary_branch = input.string("primary_branch")?;
    let identity = input
        .optional_object("identity")?
        .map(|identity| {
            identity.only(&["name", "email"])?;
            Ok(CommitIdentity {
                name: identity.string("name")?,
                email: identity.string("email")?,
            })
        })
        .transpose()?;
    Ok(RepositoryInput {
        primary_branch,
        identity,
        slug_prefix: input.optional_string("ticket_slug_prefix")?,
    })
}

pub fn repo_create(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    repo_onboard(service, session, spec, args, |root, input, identity| {
        Mutation::RepoCreate(RepoCreateInput {
            root,
            primary_branch: input.primary_branch.clone(),
            identity,
            slug_prefix: input.slug_prefix.clone(),
        })
    })
}

pub fn repo_enable(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    repo_onboard(service, session, spec, args, |root, input, identity| {
        Mutation::RepoEnable(RepoEnableInput {
            root,
            primary_branch: input.primary_branch.clone(),
            identity,
            slug_prefix: input.slug_prefix.clone(),
        })
    })
}

fn repo_onboard(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
    build: impl Fn(PathBuf, &RepositoryInput, Option<CommitIdentity>) -> Mutation,
) -> Output {
    let result = (|| {
        let flags = flags(session, spec, args)?;
        let object = input_object(session, spec, args, flags.request_id)?;
        let input = repository_input(&object)
            .map_err(|e| input_failure(session, spec, flags.request_id, e))?;
        let root = mutation_root(session, spec, args, flags.request_id)?;
        Ok((flags, input, root))
    })();
    let (flags, input, root) = match result {
        Ok(values) => values,
        Err(out) => return out,
    };
    mutate::run(
        service,
        session,
        spec,
        flags,
        Plan {
            identity_prompt: input.identity.is_none(),
            retry: true,
        },
        &mut |extras| {
            build(
                root.clone(),
                &input,
                extras
                    .identity
                    .as_ref()
                    .or(input.identity.as_ref())
                    .cloned(),
            )
        },
        &mut std::thread::sleep,
    )
}

pub fn repo_remove(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let result = (|| {
        let flags = flags(session, spec, args)?;
        let root = mutation_root(session, spec, args, flags.request_id)?;
        Ok((flags, root))
    })();
    let (flags, root) = match result {
        Ok(values) => values,
        Err(out) => return out,
    };
    mutate::run(
        service,
        session,
        spec,
        flags,
        Plan {
            identity_prompt: false,
            retry: true,
        },
        &mut |_| Mutation::RepoRemove(RepoRemoveInput { root: root.clone() }),
        &mut std::thread::sleep,
    )
}

pub fn remote_add(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let result = (|| {
        let flags = flags(session, spec, args)?;
        let object = input_object(session, spec, args, flags.request_id)?;
        let fields = (|| {
            object.only(&["name", "url"])?;
            Ok((object.string("name")?, object.string("url")?))
        })();
        let (name, url) = fields.map_err(|e| input_failure(session, spec, flags.request_id, e))?;
        let root = mutation_root(session, spec, args, flags.request_id)?;
        Ok((flags, name, url, root))
    })();
    let (flags, name, url, root) = match result {
        Ok(values) => values,
        Err(out) => return out,
    };
    mutate::run(
        service,
        session,
        spec,
        flags,
        Plan {
            identity_prompt: false,
            retry: true,
        },
        &mut |_| {
            Mutation::RemoteAdd(RemoteAddInput {
                root: root.clone(),
                name: name.clone(),
                url: url.clone(),
            })
        },
        &mut std::thread::sleep,
    )
}

pub fn remote_remove(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    remote_name(
        service,
        session,
        spec,
        args,
        |object| Ok(Some(object.string("name")?)),
        |root, name| {
            Mutation::RemoteRemove(RemoteRemoveInput {
                root,
                name: name.expect("required remote name"),
            })
        },
    )
}

pub fn remote_select(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    remote_name(
        service,
        session,
        spec,
        args,
        |object| object.nullable_string("name"),
        |root, name| Mutation::RemoteSelect(RemoteSelectInput { root, name }),
    )
}

fn remote_name(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
    read_name: impl FnOnce(&InputObject) -> Result<Option<String>, InputError>,
    build: impl Fn(PathBuf, Option<String>) -> Mutation,
) -> Output {
    let result = (|| {
        let flags = flags(session, spec, args)?;
        let object = input_object(session, spec, args, flags.request_id)?;
        let name = object
            .only(&["name"])
            .and_then(|()| read_name(&object))
            .map_err(|e| input_failure(session, spec, flags.request_id, e))?;
        let root = mutation_root(session, spec, args, flags.request_id)?;
        Ok((flags, name, root))
    })();
    let (flags, name, root) = match result {
        Ok(values) => values,
        Err(out) => return out,
    };
    mutate::run(
        service,
        session,
        spec,
        flags,
        Plan {
            identity_prompt: false,
            retry: true,
        },
        &mut |_| build(root.clone(), name.clone()),
        &mut std::thread::sleep,
    )
}

#[cfg(test)]
#[path = "admin_tests.rs"]
mod tests;
