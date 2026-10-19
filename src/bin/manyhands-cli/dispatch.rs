use super::{
    catalog::CommandSpec,
    human, parse,
    render::{self, Output},
};
use clap::ArgMatches;
use manyhands::canonical::ItemId;
use manyhands::repository::{
    HostPinDto, HostPinListDto, IdentityDto, IndexStatusDto, KeyDto, KeyListDto, OperationDto,
    OperationId, OperationListDto, PollingStatusDto, PublicKeyDto, ReadError, RemoteListDto,
    RepositoryInspectionDto, RepositoryListDto, RepositoryService, ResolvedRepository, SharedKeyId,
    transport::SshAuthority,
};
use manyhands::results::{Envelope, ResultCode, Scope};
use std::path::{Path, PathBuf};

#[allow(dead_code)] // Used by content/relationship dispatch in Tasks 5–6.
pub fn item_id(args: &ArgMatches, repo: &ResolvedRepository) -> Result<ItemId, ReadError> {
    value(args, "id")?
        .parse()
        .map_err(|_| ReadError::invalid_id().with_scope(repo.scope()))
}

fn value<'a>(args: &'a ArgMatches, name: &str) -> Result<&'a str, ReadError> {
    args.get_one::<String>(name)
        .map(String::as_str)
        .ok_or_else(|| ReadError::new(ResultCode::InternalError))
}

fn repository_path(args: &ArgMatches) -> Result<PathBuf, ReadError> {
    let path = Path::new(value(args, "repo")?);
    if path.is_absolute() {
        Ok(path.to_owned())
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .map_err(|_| ReadError::new(ResultCode::InternalError))
    }
}

pub fn resolve(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<ResolvedRepository, ReadError> {
    service.resolve_repository(&repository_path(args)?)
}

fn read_output<T: serde::Serialize>(
    name: &str,
    json: bool,
    result: Result<Envelope<T>, ReadError>,
    human: impl FnOnce(&T) -> render::Human,
) -> Output {
    match result {
        Ok(envelope) => render::envelope_output(json, &envelope, human),
        Err(error) => render::failure_output(json, &error.to_envelope(name), None),
    }
}

fn repo_list(service: &RepositoryService) -> Result<Envelope<RepositoryListDto>, ReadError> {
    Ok(Envelope::read_success(
        "repo list",
        Scope::default(),
        service.list_repositories()?,
    ))
}

fn repo_inspect(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<RepositoryInspectionDto>, ReadError> {
    // Inspection deliberately works without a registration or resolver.
    let data = service.inspect_repository(&repository_path(args)?)?;
    let scope = Scope {
        repository: Some(data.root.clone()),
        ..Scope::default()
    };
    Ok(Envelope::read_success("repo inspect", scope, data))
}

fn repo_identity(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<IdentityDto>, ReadError> {
    let repo = resolve(service, args)?;
    Ok(Envelope::read_success(
        "repo identity",
        repo.scope(),
        service.repository_identity(&repo)?,
    ))
}

fn remote_list(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<RemoteListDto>, ReadError> {
    let repo = resolve(service, args)?;
    Ok(Envelope::read_success(
        "remote list",
        repo.scope(),
        service.list_remotes_redacted(&repo)?,
    ))
}

fn key_list(service: &RepositoryService) -> Result<Envelope<KeyListDto>, ReadError> {
    Ok(Envelope::read_success(
        "key list",
        Scope::default(),
        service.list_keys()?,
    ))
}

fn key_show(service: &RepositoryService, args: &ArgMatches) -> Result<Envelope<KeyDto>, ReadError> {
    let id = SharedKeyId::parse(value(args, "key-id")?).map_err(|_| ReadError::invalid_id())?;
    Ok(Envelope::read_success(
        "key show",
        Scope::default(),
        service.show_key(id)?,
    ))
}

fn key_public(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<PublicKeyDto>, ReadError> {
    let id = SharedKeyId::parse(value(args, "key-id")?).map_err(|_| ReadError::invalid_id())?;
    Ok(Envelope::read_success(
        "key public",
        Scope::default(),
        service.public_key_text(id)?,
    ))
}

fn host_list(service: &RepositoryService) -> Result<Envelope<HostPinListDto>, ReadError> {
    Ok(Envelope::read_success(
        "host list",
        Scope::default(),
        service.list_host_pins()?,
    ))
}

fn host_inspect(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<HostPinDto>, ReadError> {
    // The parser has already validated this; use the same bracket-stripping helper.
    let (host, port) = parse::authority(value(args, "authority")?)
        .map_err(|_| ReadError::new(ResultCode::UsageError))?;
    Ok(Envelope::read_success(
        "host inspect",
        Scope::default(),
        service.inspect_host(&SshAuthority { host, port })?,
    ))
}

fn index_status(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<IndexStatusDto>, ReadError> {
    let repo = resolve(service, args)?;
    Ok(Envelope::read_success(
        "index status",
        repo.scope(),
        service.index_status(&repo)?,
    ))
}

fn poll_status(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<PollingStatusDto>, ReadError> {
    let repo = resolve(service, args)?;
    Ok(Envelope::read_success(
        "poll status",
        repo.scope(),
        service.polling_status(&repo)?,
    ))
}

fn operation_list(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<OperationListDto>, ReadError> {
    let repo = resolve(service, args)?;
    Ok(Envelope::read_success(
        "operation list",
        repo.scope(),
        service.list_operations(&repo)?,
    ))
}

fn operation_show(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<OperationDto>, ReadError> {
    let repo = resolve(service, args)?;
    let id = OperationId::parse(value(args, "operation-id")?)
        .map_err(|_| ReadError::invalid_id().with_scope(repo.scope()))?;
    Ok(Envelope::read_success(
        "operation show",
        repo.scope(),
        service.show_operation(&repo, id)?,
    ))
}

pub fn dispatch(
    command: &'static CommandSpec,
    args: &ArgMatches,
    json: bool,
    service: &RepositoryService,
) -> Output {
    let name = format!("{} {}", command.resource, command.verb);
    match name.as_str() {
        "repo list" => return read_output(&name, json, repo_list(service), human::repository_list),
        "repo inspect" => {
            return read_output(
                &name,
                json,
                repo_inspect(service, args),
                human::repository_inspection,
            );
        }
        "repo identity" => {
            return read_output(&name, json, repo_identity(service, args), human::identity);
        }
        "remote list" => {
            return read_output(&name, json, remote_list(service, args), human::remote_list);
        }
        "key list" => return read_output(&name, json, key_list(service), human::key_list),
        "key show" => return read_output(&name, json, key_show(service, args), human::key_show),
        "key public" => {
            return read_output(&name, json, key_public(service, args), human::public_key);
        }
        "host list" => return read_output(&name, json, host_list(service), human::host_list),
        "host inspect" => {
            return read_output(
                &name,
                json,
                host_inspect(service, args),
                human::host_inspect,
            );
        }
        "index status" => {
            return read_output(
                &name,
                json,
                index_status(service, args),
                human::index_status,
            );
        }
        "poll status" => {
            return read_output(
                &name,
                json,
                poll_status(service, args),
                human::polling_status,
            );
        }
        "operation list" => {
            return read_output(
                &name,
                json,
                operation_list(service, args),
                human::operation_list,
            );
        }
        "operation show" => {
            return read_output(
                &name,
                json,
                operation_show(service, args),
                human::operation_show,
            );
        }
        _ => {}
    }
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
    ]
}

#[cfg(test)]
#[path = "dispatch_tests.rs"]
mod tests;
