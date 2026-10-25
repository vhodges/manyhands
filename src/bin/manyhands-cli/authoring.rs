use std::path::PathBuf;

use clap::ArgMatches;
use manyhands::repository::{
    FolderCreateInput, IndexRebuildInput, IndexRefreshInput, Mutation, RepositoryService,
};

use crate::{
    admin,
    catalog::CommandSpec,
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

#[cfg(test)]
#[path = "authoring_tests.rs"]
mod tests;
