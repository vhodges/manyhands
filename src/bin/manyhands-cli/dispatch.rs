use super::{
    catalog::CommandSpec,
    human, human_authoring, parse,
    render::{self, Output},
};
use clap::ArgMatches;
use manyhands::canonical::ItemId;
use manyhands::repository::{
    ClosureFilter, CommentListDto, CycleListDto, DependencyDirection, DependencyTreeDto,
    DocumentFolderListDto, HostPinDto, HostPinListDto, IdentityDto, IndexStatusDto, ItemDto,
    ItemDtoKind, ItemListDto, KeyDto, KeyListDto, OperationDto, OperationId, OperationListDto,
    PathObservationDto, PlanDto, PollingStatusDto, ProposedRelationships, PublicKeyDto, ReadError,
    ReadinessFilter, RelationshipCheckDto, RemoteListDto, RepositoryInspectionDto,
    RepositoryListDto, RepositoryService, ResolvedRepository, SharedKeyId, TicketFilter,
    transport::SshAuthority,
};
use manyhands::results::{Envelope, ResultCode, Scope};
use std::path::{Path, PathBuf};

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

// Relationship verbs declare only a subset of these options. `try_get_one`
// also handles an undeclared option, unlike `get_one`, which would panic.
fn ticket_filter(args: &ArgMatches) -> TicketFilter {
    let text = |name| args.try_get_one::<String>(name).ok().flatten().cloned();
    let flag = |name| {
        args.try_get_one::<bool>(name)
            .ok()
            .flatten()
            .copied()
            .unwrap_or(false)
    };
    TicketFilter {
        status: text("status"),
        ticket_type: text("type"),
        project: text("project"),
        slug: text("slug"),
        closure: match text("closure").as_deref() {
            Some("open") => ClosureFilter::Open,
            Some("closed") => ClosureFilter::Closed,
            _ => ClosureFilter::All,
        },
        readiness: if flag("ready") {
            Some(ReadinessFilter::Ready)
        } else if flag("blocked") {
            Some(ReadinessFilter::Blocked)
        } else {
            None
        },
    }
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

pub fn host_observe(
    service: &RepositoryService,
    session: &mut crate::mutate::Session,
    args: &ArgMatches,
) -> Output {
    use crate::{admin, catalog, mutate, prompt::TerminalProvider, terminal::Terminal};
    use manyhands::repository::keys::SessionCredentials;
    use std::{cell::Cell, io::Write};
    let spec = catalog::find("host", "observe").expect("host observe catalog entry");
    let root = match admin::mutation_root(session, spec, args, None) {
        Ok(root) => root,
        Err(out) => return out,
    };
    let authority = match args
        .get_one::<String>("authority")
        .map(|text| parse::authority(text))
        .transpose()
    {
        Ok(value) => value.map(|(host, port)| SshAuthority { host, port }),
        Err(_) => {
            return mutate::refusal(session, "host observe", None, ResultCode::UsageError, None);
        }
    };
    if !session.json {
        let _ = writeln!(std::io::stderr().lock(), "progress: connecting");
    }
    let supplied = Cell::new(false);
    let terminal = if session.interactive {
        session
            .terminal
            .as_deref_mut()
            .map(|t| t as &mut dyn Terminal)
    } else {
        None
    };
    let mut credentials = SessionCredentials::new(TerminalProvider::new(terminal, &supplied));
    let envelope = service.observe_host(&root, authority.as_ref(), &mut credentials);
    render::observation_output(session.json, &envelope, supplied.get())
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

fn document_list(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<ItemListDto>, ReadError> {
    let repo = resolve(service, args)?;
    Ok(Envelope::read_success(
        "document list",
        repo.scope(),
        service.list_documents(&repo)?,
    ))
}

fn folder_list(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<DocumentFolderListDto>, ReadError> {
    let repo = resolve(service, args)?;
    Ok(Envelope::read_success(
        "folder list",
        repo.scope(),
        service.list_document_folders(&repo)?,
    ))
}

fn ticket_list(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<ItemListDto>, ReadError> {
    let repo = resolve(service, args)?;
    Ok(Envelope::read_success(
        "ticket list",
        repo.scope(),
        service.list_tickets(&repo, &ticket_filter(args))?,
    ))
}

fn show(
    service: &RepositoryService,
    args: &ArgMatches,
    name: &str,
    kind: ItemDtoKind,
) -> Result<Envelope<ItemDto>, ReadError> {
    let repo = resolve(service, args)?;
    let mut scope = repo.scope();
    let data = if let Some(path) = args.get_one::<String>("path") {
        let context = args.get_one::<String>("context").map(PathBuf::from);
        service.show_path(&repo, context.as_deref(), Path::new(path))?
    } else {
        let id = item_id(args, &repo)?;
        scope.item_id = Some(id.to_string());
        service.show_item(&repo, &id)?
    };
    if data.kind != kind {
        return Err(ReadError::new(ResultCode::ItemNotFound).with_scope(scope));
    }
    Ok(Envelope::read_success(name, scope, data))
}

fn document_show(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<ItemDto>, ReadError> {
    show(service, args, "document show", ItemDtoKind::Document)
}

fn ticket_show(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<ItemDto>, ReadError> {
    show(service, args, "ticket show", ItemDtoKind::Ticket)
}

fn comment_list(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<CommentListDto>, ReadError> {
    let repo = resolve(service, args)?;
    let id = item_id(args, &repo)?;
    let scope = Scope {
        item_id: Some(id.to_string()),
        ..repo.scope()
    };
    Ok(Envelope::read_success(
        "comment list",
        scope,
        service.list_comments(&repo, &id)?,
    ))
}

fn ticket_ready(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<ItemListDto>, ReadError> {
    let repo = resolve(service, args)?;
    let filter = TicketFilter {
        readiness: Some(ReadinessFilter::Ready),
        ..ticket_filter(args)
    };
    Ok(Envelope::read_success(
        "ticket ready",
        repo.scope(),
        service.ticket_readiness(&repo, &filter)?,
    ))
}

fn ticket_blocked(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<ItemListDto>, ReadError> {
    let repo = resolve(service, args)?;
    let filter = TicketFilter {
        readiness: Some(ReadinessFilter::Blocked),
        ..ticket_filter(args)
    };
    Ok(Envelope::read_success(
        "ticket blocked",
        repo.scope(),
        service.ticket_readiness(&repo, &filter)?,
    ))
}

fn ticket_deps(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<DependencyTreeDto>, ReadError> {
    let repo = resolve(service, args)?;
    let id = item_id(args, &repo)?;
    let direction = match value(args, "direction")? {
        "up" => DependencyDirection::Up,
        "down" => DependencyDirection::Down,
        "both" => DependencyDirection::Both,
        // The parser's value list guarantees these are the only directions.
        _ => return Err(ReadError::new(ResultCode::InternalError).with_scope(repo.scope())),
    };
    let depth = args.get_one::<u32>("depth").copied();
    let scope = Scope {
        item_id: Some(id.to_string()),
        ..repo.scope()
    };
    Ok(Envelope::read_success(
        "ticket deps",
        scope,
        service.ticket_dependencies(&repo, &id, direction, depth)?,
    ))
}

fn ticket_children(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<ItemListDto>, ReadError> {
    let repo = resolve(service, args)?;
    let id = item_id(args, &repo)?;
    let scope = Scope {
        item_id: Some(id.to_string()),
        ..repo.scope()
    };
    Ok(Envelope::read_success(
        "ticket children",
        scope,
        service.ticket_children(&repo, &id)?,
    ))
}

fn ticket_cycles(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<CycleListDto>, ReadError> {
    let repo = resolve(service, args)?;
    Ok(Envelope::read_success(
        "ticket cycles",
        repo.scope(),
        service.ticket_cycles(&repo)?,
    ))
}

fn ticket_plan(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<PlanDto>, ReadError> {
    let repo = resolve(service, args)?;
    Ok(Envelope::read_success(
        "ticket plan",
        repo.scope(),
        service.ticket_plan(&repo, &ticket_filter(args))?,
    ))
}

fn ticket_critical_path(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<ItemListDto>, ReadError> {
    let repo = resolve(service, args)?;
    Ok(Envelope::read_success(
        "ticket critical-path",
        repo.scope(),
        service.ticket_critical_path(&repo)?,
    ))
}

fn ticket_find(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<ItemListDto>, ReadError> {
    let repo = resolve(service, args)?;
    Ok(Envelope::read_success(
        "ticket find",
        repo.scope(),
        service.find_tickets_by_slug(&repo, value(args, "slug")?)?,
    ))
}

fn path_observe(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<PathObservationDto>, ReadError> {
    let repo = resolve(service, args)?;
    let context = args.get_one::<String>("context").map(PathBuf::from);
    let data = service.observe_path(&repo, context.as_deref(), Path::new(value(args, "path")?))?;
    Ok(Envelope::read_success("path observe", repo.scope(), data))
}

fn ticket_check(
    service: &RepositoryService,
    args: &ArgMatches,
) -> Result<Envelope<RelationshipCheckDto>, ReadError> {
    let repo = resolve(service, args)?;
    let id = item_id(args, &repo)?;
    let parse_id = |text: &str| {
        text.parse::<ItemId>()
            .map_err(|_| ReadError::invalid_id().with_scope(repo.scope()))
    };
    let deps = args
        .get_one::<String>("deps")
        .map(|text| text.split(',').map(parse_id).collect::<Result<Vec<_>, _>>())
        .transpose()?
        .unwrap_or_default();
    let parent = args
        .get_one::<String>("parent")
        .map(|text| parse_id(text))
        .transpose()?;
    let data =
        service.check_ticket_relationships(&repo, &id, &ProposedRelationships { deps, parent })?;
    let mut scope = repo.scope();
    scope.item_id = Some(id.to_string());
    Ok(Envelope::read_success("ticket check", scope, data))
}

pub fn dispatch(
    command: &'static CommandSpec,
    args: &ArgMatches,
    session: &mut crate::mutate::Session,
    service: &RepositoryService,
) -> Output {
    let json = session.json;
    let name = format!("{} {}", command.resource, command.verb);
    match name.as_str() {
        "path observe" => {
            return read_output(
                &name,
                json,
                path_observe(service, args),
                human_authoring::path_observation,
            );
        }
        "ticket check" => {
            return read_output(
                &name,
                json,
                ticket_check(service, args),
                human_authoring::relationship_check,
            );
        }
        "repo create" => return crate::admin::repo_create(service, session, command, args),
        "index refresh" => return crate::authoring::index_refresh(service, session, command, args),
        "index rebuild" => return crate::authoring::index_rebuild(service, session, command, args),
        "folder create" => return crate::authoring::folder_create(service, session, command, args),
        "document create" => {
            return crate::authoring::document_create(service, session, command, args);
        }
        "document save" => return crate::authoring::document_save(service, session, command, args),
        "document move" => return crate::authoring::document_move(service, session, command, args),
        "document repair" => {
            return crate::authoring::document_repair(service, session, command, args);
        }
        "ticket repair" => return crate::authoring::ticket_repair(service, session, command, args),
        "ticket create" => return crate::authoring::ticket_create(service, session, command, args),
        "ticket save" => return crate::authoring::ticket_save(service, session, command, args),
        "ticket slug-assign" => {
            return crate::authoring::ticket_slug_assign(service, session, command, args);
        }
        "item sync" => return crate::collaboration::item_sync(service, session, command, args),
        "repo sync" => return crate::collaboration::repo_sync(service, session, command, args),
        "operation resume" => {
            return crate::collaboration::operation_resume(service, session, command, args);
        }
        "operation abandon" => {
            return crate::collaboration::operation_abandon(service, session, command, args);
        }
        "repo enable" => return crate::admin::repo_enable(service, session, command, args),
        "repo remove" => return crate::admin::repo_remove(service, session, command, args),
        "repo identity-set" => {
            return crate::admin::repo_identity_set(service, session, command, args);
        }
        "remote add" => return crate::admin::remote_add(service, session, command, args),
        "remote remove" => return crate::admin::remote_remove(service, session, command, args),
        "remote select" => return crate::admin::remote_select(service, session, command, args),
        "key import" => return crate::admin::key_import(service, session, command, args),
        "key generate" => return crate::admin::key_generate(service, session, command, args),
        "key select" => return crate::admin::key_select(service, session, command, args),
        "key clear" => return crate::admin::key_clear(service, session, command, args),
        "key remove" => return crate::admin::key_remove(service, session, command, args),
        "key delete" => return crate::admin::key_delete(service, session, command, args),
        "request show" | "request cancel" => {
            let id = match crate::admin::request_id(session, command, args) {
                Ok(Some(id)) => id,
                Err(output) => return output,
                Ok(None) => {
                    return crate::mutate::refusal(
                        session,
                        &name,
                        None,
                        ResultCode::InvalidId,
                        None,
                    );
                }
            };
            return if command.verb == "show" {
                render::envelope_output(
                    json,
                    &service.show_request(id),
                    crate::human_admin::request,
                )
            } else {
                render::envelope_output(
                    json,
                    &service.cancel_request(id),
                    crate::human_admin::cancel,
                )
            };
        }
        "ticket ready" => {
            return read_output(&name, json, ticket_ready(service, args), human::item_list);
        }
        "ticket blocked" => {
            return read_output(
                &name,
                json,
                ticket_blocked(service, args),
                human::ticket_blocked,
            );
        }
        "ticket deps" => {
            return read_output(&name, json, ticket_deps(service, args), human::ticket_deps);
        }
        "ticket children" => {
            return read_output(
                &name,
                json,
                ticket_children(service, args),
                human::item_list,
            );
        }
        "ticket cycles" => {
            return read_output(
                &name,
                json,
                ticket_cycles(service, args),
                human::ticket_cycles,
            );
        }
        "ticket plan" => {
            return read_output(&name, json, ticket_plan(service, args), human::ticket_plan);
        }
        "ticket critical-path" => {
            return read_output(
                &name,
                json,
                ticket_critical_path(service, args),
                human::item_list,
            );
        }
        "ticket find" => {
            return read_output(&name, json, ticket_find(service, args), human::item_list);
        }
        "document list" => {
            return read_output(&name, json, document_list(service, args), human::item_list);
        }
        "document show" => {
            return read_output(&name, json, document_show(service, args), human::item_show);
        }
        "folder list" => {
            return read_output(&name, json, folder_list(service, args), human::folder_list);
        }
        "ticket list" => {
            return read_output(&name, json, ticket_list(service, args), human::item_list);
        }
        "ticket show" => {
            return read_output(&name, json, ticket_show(service, args), human::item_show);
        }
        "comment list" => {
            return read_output(
                &name,
                json,
                comment_list(service, args),
                human::comment_list,
            );
        }
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
        "host observe" => return host_observe(service, session, args),
        "host approve" => return crate::admin::host_approve(service, session, command, args),
        "host replace" => return crate::admin::host_replace(service, session, command, args),
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
    vec![]
}

#[cfg(test)]
#[path = "dispatch_tests.rs"]
mod tests;
