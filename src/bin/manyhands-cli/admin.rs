use crate::{
    catalog::CommandSpec,
    input::{self, InputError, InputObject, Tristate},
    mutate::{self, MutationFlags, Plan, Session},
    prompt::{self, Asked},
    render::Output,
};
use clap::ArgMatches;
use manyhands::{
    repository::{
        CommitIdentity, ConfirmationId, HostApproveInput, HostReplaceInput, InitialsChange,
        KeyClearInput, KeyDeleteInput, KeyGenerateInput, KeyImportInput, KeyRemoveInput,
        KeySelectInput, Mutation, RemoteAddInput, RemoteRemoveInput, RemoteSelectInput,
        RepoCreateInput, RepoEnableInput, RepoIdentitySetInput, RepoRemoveInput, RepositoryService,
        RequestId, SharedKeyId,
        keys::{KeyProtection, KeyStore},
        transport::SshAuthority,
    },
    results::ResultCode,
};
use std::path::PathBuf;

pub(crate) fn name(spec: &CommandSpec) -> String {
    format!("{} {}", spec.resource, spec.verb)
}

// Session/spec context is needed to render the first failure in the caller's
// mode, retaining a valid request ID if a later selector is malformed.
pub fn flags(
    session: &Session,
    spec: &CommandSpec,
    args: &ArgMatches,
) -> Result<MutationFlags, Output> {
    let mut flags = MutationFlags {
        request_id: request_id(session, spec, args)?,
        allocated: false,
        prepare: args
            .try_get_one::<bool>("prepare")
            .ok()
            .flatten()
            .copied()
            .unwrap_or(false),
        confirm: None,
    };
    flags.allocate(session);
    let request_id = flags.request_id;
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
    flags.confirm = confirm;
    Ok(flags)
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
pub(crate) fn input_failure(
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

pub fn key_generate(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let result = (|| {
        let flags = flags(session, spec, args)?;
        let object = input_object(session, spec, args, flags.request_id)?;
        let fields = (|| {
            object.only(&["label", "protection"])?;
            Ok((
                object.string("label")?,
                object.one_of("protection", &["passphrase", "none"])? == "passphrase",
            ))
        })();
        let (label, protected) =
            fields.map_err(|e| input_failure(session, spec, flags.request_id, e))?;
        let store = KeyStore::for_current_user().map_err(|_| {
            mutate::refusal(
                session,
                &name(spec),
                flags.request_id,
                ResultCode::InternalError,
                None,
            )
        })?;
        Ok((flags, label, protected, store))
    })();
    let (flags, label, protected, store) = match result {
        Ok(values) => values,
        Err(out) => return out,
    };
    let mut secret = if protected && !flags.prepare {
        if !session.interactive {
            return mutate::refusal(
                session,
                &name(spec),
                flags.request_id,
                ResultCode::UnlockRequired,
                Some("A passphrase can be entered only at a terminal".into()),
            );
        }
        let asked = session
            .terminal
            .as_deref_mut()
            .map(|terminal| prompt::new_passphrase(terminal, &label))
            .unwrap_or(Asked::Cancelled);
        match asked {
            Asked::Answer(secret) => Some(secret),
            Asked::Cancelled => {
                return mutate::refusal(
                    session,
                    &name(spec),
                    flags.request_id,
                    ResultCode::Cancelled,
                    None,
                );
            }
            Asked::Exhausted => {
                return mutate::refusal(
                    session,
                    &name(spec),
                    flags.request_id,
                    ResultCode::InvalidInput,
                    None,
                );
            }
        }
    } else {
        // F2 prepare rejects this unconfirmed command as not_confirmable before
        // reading its input. Unencrypted is sufficient; never ask for a secret.
        None
    };
    mutate::run(
        service,
        session,
        spec,
        flags,
        Plan {
            identity_prompt: false,
            retry: !protected,
        },
        &mut |_| {
            Mutation::KeyGenerate(KeyGenerateInput {
                store: store.clone(),
                label: label.clone(),
                // An unconfirmed protected execute builds exactly once (retry is
                // disabled). Move its secret, never clone it or panic on a retry.
                protection: secret
                    .take()
                    .map_or(KeyProtection::Unencrypted, KeyProtection::Passphrase),
            })
        },
        &mut std::thread::sleep,
    )
}

pub fn key_import(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let result = (|| {
        let flags = flags(session, spec, args)?;
        let object = input_object(session, spec, args, flags.request_id)?;
        let fields = (|| {
            object.only(&["label", "private_key_path", "public_key_path"])?;
            Ok((
                object.string("label")?,
                object.string("private_key_path")?,
                object.optional_string("public_key_path")?,
            ))
        })();
        let (label, private, public) =
            fields.map_err(|e| input_failure(session, spec, flags.request_id, e))?;
        let cwd = std::env::current_dir().map_err(|_| {
            mutate::refusal(
                session,
                &name(spec),
                flags.request_id,
                ResultCode::InvalidPath,
                None,
            )
        })?;
        Ok((
            flags,
            label,
            cwd.join(private),
            public.map(|path| cwd.join(path)),
        ))
    })();
    let (flags, label, private_key_path, public_key_path) = match result {
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
            Mutation::KeyImport(KeyImportInput {
                label: label.clone(),
                private_key_path: private_key_path.clone(),
                public_key_path: public_key_path.clone(),
            })
        },
        &mut std::thread::sleep,
    )
}

fn key_id(
    session: &Session,
    spec: &CommandSpec,
    args: &ArgMatches,
    request_id: Option<RequestId>,
) -> Result<SharedKeyId, Output> {
    SharedKeyId::parse(
        args.get_one::<String>("key-id")
            .expect("required key selector"),
    )
    .map_err(|_| {
        mutate::refusal(
            session,
            &name(spec),
            request_id,
            ResultCode::InvalidId,
            None,
        )
    })
}

pub fn key_select(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    key_selected(service, session, spec, args, |id| {
        Mutation::KeySelect(KeySelectInput { id })
    })
}

pub fn key_remove(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    key_selected(service, session, spec, args, |id| {
        Mutation::KeyRemove(KeyRemoveInput { id })
    })
}

fn key_selected(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
    build: impl Fn(SharedKeyId) -> Mutation,
) -> Output {
    let result = (|| {
        let flags = flags(session, spec, args)?;
        let id = key_id(session, spec, args, flags.request_id)?;
        Ok((flags, id))
    })();
    let (flags, id) = match result {
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
        &mut |_| build(id),
        &mut std::thread::sleep,
    )
}

pub fn key_clear(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let flags = match flags(session, spec, args) {
        Ok(flags) => flags,
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
        &mut |_| Mutation::KeyClear(KeyClearInput {}),
        &mut std::thread::sleep,
    )
}

pub fn key_delete(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let result = (|| {
        let flags = flags(session, spec, args)?;
        let id = key_id(session, spec, args, flags.request_id)?;
        let store = KeyStore::for_current_user().map_err(|_| {
            mutate::refusal(
                session,
                &name(spec),
                flags.request_id,
                ResultCode::InternalError,
                None,
            )
        })?;
        Ok((flags, id, store))
    })();
    let (flags, id, store) = match result {
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
            Mutation::KeyDelete(KeyDeleteInput {
                store: store.clone(),
                id,
            })
        },
        &mut std::thread::sleep,
    )
}

struct HostInput {
    authority: SshAuthority,
    algorithm: String,
    fingerprint: String,
    old_fingerprint: Option<String>,
}

pub fn host_approve(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    host_mutation(service, session, spec, args, false)
}

pub fn host_replace(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    host_mutation(service, session, spec, args, true)
}

fn host_mutation(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
    replace: bool,
) -> Output {
    let result = (|| {
        let flags = flags(session, spec, args)?;
        let object = input_object(session, spec, args, flags.request_id)?;
        let fields = (|| {
            object.only(if replace {
                &["authority", "algorithm", "fingerprint", "old_fingerprint"]
            } else {
                &["authority", "algorithm", "fingerprint"]
            })?;
            Ok((
                object.string("authority")?,
                object.string("algorithm")?,
                object.string("fingerprint")?,
                if replace {
                    Some(object.string("old_fingerprint")?)
                } else {
                    None
                },
            ))
        })();
        let (authority, algorithm, fingerprint, old_fingerprint) =
            fields.map_err(|e| input_failure(session, spec, flags.request_id, e))?;
        let (host, port) = crate::parse::authority(&authority).map_err(|_| {
            mutate::refusal(
                session,
                &name(spec),
                flags.request_id,
                ResultCode::InvalidInput,
                Some("Input field authority must be host[:port] or [IPv6][:port], with a port from 1 to 65535.".into()),
            )
        })?;
        let root = mutation_root(session, spec, args, flags.request_id)?;
        Ok((
            flags,
            root,
            HostInput {
                authority: SshAuthority { host, port },
                algorithm,
                fingerprint,
                old_fingerprint,
            },
        ))
    })();
    let (flags, root, input) = match result {
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
            if let Some(old_fingerprint) = &input.old_fingerprint {
                Mutation::HostReplace(HostReplaceInput {
                    root: root.clone(),
                    authority: input.authority.clone(),
                    algorithm: input.algorithm.clone(),
                    fingerprint: input.fingerprint.clone(),
                    old_fingerprint: old_fingerprint.clone(),
                })
            } else {
                Mutation::HostApprove(HostApproveInput {
                    root: root.clone(),
                    authority: input.authority.clone(),
                    algorithm: input.algorithm.clone(),
                    fingerprint: input.fingerprint.clone(),
                })
            }
        },
        &mut std::thread::sleep,
    )
}

#[cfg(test)]
#[path = "admin_tests.rs"]
mod tests;
