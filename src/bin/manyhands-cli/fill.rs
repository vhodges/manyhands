use std::path::Path;

use manyhands::{
    canonical::ItemId,
    repository::{
        DocumentDraft, ItemDto, ItemDtoKind, ReadError, RepositoryService, RequestId,
        ResolvedRepository,
    },
    results::{Effects, ResultCode},
};

use crate::{
    admin,
    catalog::CommandSpec,
    mutate::Session,
    render::{self, Output},
};

pub struct DocumentFields {
    pub title: Option<String>,
    pub body: Option<String>,
}

pub enum FillError {
    Unfillable,
}

pub fn needs_read_document(fields: &DocumentFields) -> bool {
    fields.title.is_none() || fields.body.is_none()
}

pub fn document_draft(
    fields: DocumentFields,
    read: Option<&ItemDto>,
) -> Result<DocumentDraft, FillError> {
    if read.is_some_and(|item| item.kind != ItemDtoKind::Document) {
        return Err(FillError::Unfillable);
    }
    Ok(DocumentDraft {
        title: fields
            .title
            .or_else(|| read.and_then(|item| item.title.clone()))
            .ok_or(FillError::Unfillable)?,
        body: fields
            .body
            .or_else(|| read.and_then(|item| item.body.clone()))
            .ok_or(FillError::Unfillable)?,
    })
}

pub fn read_item(
    service: &RepositoryService,
    session: &Session,
    spec: &CommandSpec,
    root: &Path,
    id: &ItemId,
    kind: ItemDtoKind,
    request_id: Option<RequestId>,
) -> Result<(ResolvedRepository, ItemDto), Output> {
    let failure = |error: ReadError| read_failure(session, spec, request_id, error);
    let repo = service.resolve_repository(root).map_err(failure)?;
    let item = service.show_item(&repo, id).map_err(failure)?;
    if item.kind != kind {
        return Err(item_not_found(session, spec, &repo, id, request_id));
    }
    Ok((repo, item))
}

pub fn item_not_found(
    session: &Session,
    spec: &CommandSpec,
    repo: &ResolvedRepository,
    id: &ItemId,
    request_id: Option<RequestId>,
) -> Output {
    let mut scope = repo.scope();
    scope.item_id = Some(id.to_string());
    read_failure(
        session,
        spec,
        request_id,
        ReadError::new(ResultCode::ItemNotFound).with_scope(scope),
    )
}

fn read_failure(
    session: &Session,
    spec: &CommandSpec,
    request_id: Option<RequestId>,
    error: ReadError,
) -> Output {
    let mut envelope = error.to_envelope(admin::name(spec));
    envelope.request_id = request_id.map(|id| id.to_string());
    envelope.effects = Effects::not_requested();
    render::failure_output(session.json, &envelope, None)
}

#[cfg(test)]
#[path = "fill_tests.rs"]
mod tests;
