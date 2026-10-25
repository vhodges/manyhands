use std::path::PathBuf;

use clap::ArgMatches;
use manyhands::canonical::ItemId;
use manyhands::repository::{
    DocumentCreateInput, DocumentDraft, DocumentMoveInput, DocumentSaveInput, FolderCreateInput,
    IndexRebuildInput, IndexRefreshInput, ItemDtoKind, Mutation, RepositoryService, RequestId,
};
use manyhands::results::ResultCode;

use crate::{
    admin,
    catalog::CommandSpec,
    fill::{self, DocumentFields},
    input::InputError,
    mutate::{self, Plan, Session},
    render::Output,
};

pub fn index_refresh(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    root_only(service, session, spec, args, |root| {
        Mutation::IndexRefresh(IndexRefreshInput { root })
    })
}

pub fn index_rebuild(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    root_only(service, session, spec, args, |root| {
        Mutation::IndexRebuild(IndexRebuildInput { root })
    })
}

fn root_only(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
    build: impl Fn(PathBuf) -> Mutation,
) -> Output {
    let (flags, root) = match (|| {
        let flags = admin::flags(session, spec, args)?;
        let root = admin::mutation_root(session, spec, args, flags.request_id)?;
        Ok((flags, root))
    })() {
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
        &mut |_| build(root.clone()),
        &mut std::thread::sleep,
    )
}

pub fn folder_create(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let (flags, path, root) = match (|| {
        let flags = admin::flags(session, spec, args)?;
        let object = admin::input_object(session, spec, args, flags.request_id)?;
        let path = (|| {
            object.only(&["path"])?;
            object.string("path")
        })()
        .map_err(|e| admin::input_failure(session, spec, flags.request_id, e))?;
        let root = admin::mutation_root(session, spec, args, flags.request_id)?;
        Ok((flags, path, root))
    })() {
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
            Mutation::FolderCreate(FolderCreateInput {
                root: root.clone(),
                path: path.clone(),
            })
        },
        &mut std::thread::sleep,
    )
}

pub fn document_create(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let (flags, root, id, path, draft) = match (|| {
        let flags = admin::flags(session, spec, args)?;
        let object = admin::input_object(session, spec, args, flags.request_id)?;
        let (id, path, draft) = (|| {
            object.only(&["item_id", "path", "title", "body"])?;
            let id = object.string("item_id")?.parse::<ItemId>().map_err(|_| {
                InputError::InvalidValue {
                    field: "item_id",
                    allowed: &["a canonical ULID"],
                }
            })?;
            Ok((
                id,
                PathBuf::from(object.string("path")?),
                DocumentDraft {
                    title: object.string("title")?,
                    body: object.string("body")?,
                },
            ))
        })()
        .map_err(|e| admin::input_failure(session, spec, flags.request_id, e))?;
        let root = admin::mutation_root(session, spec, args, flags.request_id)?;
        Ok((flags, root, id, path, draft))
    })() {
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
            Mutation::DocumentCreate(DocumentCreateInput {
                root: root.clone(),
                id: id.clone(),
                path: path.clone(),
                draft: draft.clone(),
            })
        },
        &mut std::thread::sleep,
    )
}

fn document_id(
    session: &Session,
    spec: &CommandSpec,
    args: &ArgMatches,
    request_id: Option<RequestId>,
) -> Result<ItemId, Output> {
    args.get_one::<String>("id")
        .expect("required item ID")
        .parse::<ItemId>()
        .map_err(|_| {
            mutate::refusal(
                session,
                &admin::name(spec),
                request_id,
                ResultCode::InvalidId,
                None,
            )
        })
}

pub fn document_save(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let (flags, root, id, observation, draft) = match (|| {
        let flags = admin::flags(session, spec, args)?;
        let object = admin::input_object(session, spec, args, flags.request_id)?;
        let (observation, fields) = (|| {
            object.only(&["observation", "title", "body"])?;
            Ok((
                object.string("observation")?,
                DocumentFields {
                    title: object.optional_string("title")?,
                    body: object.optional_string("body")?,
                },
            ))
        })()
        .map_err(|e| admin::input_failure(session, spec, flags.request_id, e))?;
        let root = admin::mutation_root(session, spec, args, flags.request_id)?;
        let id = document_id(session, spec, args, flags.request_id)?;
        // Fill once per invocation, before the driver. Replays retain the
        // caller's observation and bounded retries reuse this complete draft.
        let draft = if fill::needs_read_document(&fields) {
            let (repo, item) = fill::read_item(
                service,
                session,
                spec,
                &root,
                &id,
                ItemDtoKind::Document,
                flags.request_id,
            )?;
            fill::document_draft(fields, Some(&item))
                .map_err(|_| fill::item_not_found(session, spec, &repo, &id, flags.request_id))?
        } else {
            fill::document_draft(fields, None)
                .ok()
                .expect("both document fields are present")
        };
        Ok((flags, root, id, observation, draft))
    })() {
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
            Mutation::DocumentSave(DocumentSaveInput {
                root: root.clone(),
                id: id.clone(),
                observation: Some(observation.clone()),
                draft: draft.clone(),
            })
        },
        &mut std::thread::sleep,
    )
}

pub fn document_move(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let (flags, root, id, observation, destination_path, destination_observation) = match (|| {
        let flags = admin::flags(session, spec, args)?;
        let object = admin::input_object(session, spec, args, flags.request_id)?;
        let (observation, path, destination_observation) = (|| {
            object.only(&["observation", "destination_path", "destination_observation"])?;
            Ok((
                object.string("observation")?,
                PathBuf::from(object.string("destination_path")?),
                object.string("destination_observation")?,
            ))
        })()
        .map_err(|e| admin::input_failure(session, spec, flags.request_id, e))?;
        let root = admin::mutation_root(session, spec, args, flags.request_id)?;
        let id = document_id(session, spec, args, flags.request_id)?;
        Ok((flags, root, id, observation, path, destination_observation))
    })() {
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
            Mutation::DocumentMove(DocumentMoveInput {
                root: root.clone(),
                id: id.clone(),
                observation: Some(observation.clone()),
                destination_path: destination_path.clone(),
                destination_observation: destination_observation.clone(),
            })
        },
        &mut std::thread::sleep,
    )
}

#[cfg(test)]
#[path = "authoring_tests.rs"]
mod tests;
