//! A closes before Git is read; B is a genuinely new SQLite snapshot. Neither
//! session acquires Git ownership or persists an issued capability.

use crate::{canonical::ItemId, repository::*, results::ResultCode};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalContextSyncObservationDto {
    pub kind: String,
    pub item_id: String,
    pub context_worktree: String,
    pub context_ref: String,
    pub primary_ref: String,
    pub context_head: String,
    pub primary_head: String,
    pub observation: String,
}

#[derive(PartialEq, Eq)]
pub(super) struct Facts {
    pub repository_id: i64,
    pub context_id: i64,
    pub worktree: PathBuf,
    item_path: PathBuf,
    pub branch: String,
    pub primary: String,
    pub remote: Option<String>,
    registration: Vec<u8>,
    semantics: Vec<Vec<u8>>,
}

pub(super) fn field(hash: &mut blake3::Hasher, bytes: &[u8]) {
    hash.update(&(bytes.len() as u64).to_be_bytes());
    hash.update(bytes);
}

pub(super) fn facts(
    db: &Connection,
    repo: &ResolvedRepository,
    kind: AuthoringKind,
    item: &ItemId,
) -> Result<Facts, ReadError> {
    super::record::audit(db).map_err(|_| ReadError::new(ResultCode::RecoveryRequired))?;
    let registration = db
        .query_row(
            "SELECT id,enabled_at,config_blob_oid FROM repositories WHERE root_path=?1",
            [repo.root().to_str()],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .optional()?;
    let (repository_id, enabled_at, config) =
        registration.ok_or_else(|| ReadError::new(ResultCode::RepositoryNotRegistered))?;
    if repository_id != repo.registration_id() {
        return Err(ReadError::new(ResultCode::RepositoryNotRegistered));
    }
    let row = db.query_row("SELECT c.id,c.worktree_path,c.branch,i.canonical_path FROM contexts c JOIN discovered_items i ON i.context_id=c.id AND i.item_id=c.item_id WHERE c.repository_id=?1 AND c.kind='active' AND c.item_id=?2 AND i.kind=?3", params![repository_id,item.to_string(),authoring_kind_segment(&kind)], |r| Ok((r.get::<_, i64>(0)?,r.get::<_, String>(1)?,r.get::<_, String>(2)?,r.get::<_,String>(3)?))).optional()?;
    let (context_id, path, branch, item_path) =
        row.ok_or_else(|| ReadError::new(ResultCode::ItemNotFound))?;
    let (primary, remote) = db.query_row("SELECT primary_branch,publication_remote FROM configuration_observations WHERE repository_id=?1 AND state='valid'", [repository_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))).map_err(|_| ReadError::new(ResultCode::InvalidConfiguration))?;
    // Cancellation counters and association participate in stability, but an
    // executor-only takeover deliberately does not change these facts.
    let semantics = db.prepare("SELECT operation_ulid,semantic_revision,generation,state,active_resolution,cancel_issued,cancel_acknowledged FROM local_context_sync_records WHERE repository_id=?1 ORDER BY operation_ulid")?.query_map([repository_id], |r| {
        let values = (r.get::<_, String>(0)?,r.get::<_, i64>(1)?,r.get::<_, i64>(2)?,r.get::<_, String>(3)?,r.get::<_, Option<String>>(4)?,r.get::<_, i64>(5)?,r.get::<_, i64>(6)?);
        serde_json::to_vec(&values).map_err(|_| rusqlite::Error::InvalidQuery)
    })?.collect::<Result<Vec<_>, _>>()?;
    Ok(Facts {
        repository_id,
        context_id,
        worktree: path.into(),
        item_path: item_path.into(),
        branch: format!("refs/heads/{branch}"),
        primary,
        remote,
        registration: serde_json::to_vec(&(enabled_at, config))
            .map_err(|_| ReadError::new(ResultCode::InternalError))?,
        semantics,
    })
}

#[derive(PartialEq, Eq)]
struct GitImage {
    context: git2::Oid,
    primary: git2::Oid,
    index: [u8; 32],
    configuration: [u8; 32],
    common: PathBuf,
}

fn git_image(
    repo: &ResolvedRepository,
    facts: &Facts,
    kind: AuthoringKind,
    item: &ItemId,
) -> Result<GitImage, ReadError> {
    let inaccessible = || ReadError::new(ResultCode::RepositoryInaccessible);
    let changed = || ReadError::new(ResultCode::ExternalChange);
    let root = git2::Repository::open(repo.root()).map_err(|_| inaccessible())?;
    let worktree = git2::Repository::open_ext(
        &facts.worktree,
        git2::RepositoryOpenFlags::NO_SEARCH,
        &[] as &[&std::ffi::OsStr],
    )
    .map_err(|_| inaccessible())?;
    let common = fs::canonicalize(root.commondir()).map_err(|_| inaccessible())?;
    if fs::canonicalize(worktree.commondir()).map_err(|_| inaccessible())? != common
        || fs::canonicalize(worktree.workdir().ok_or_else(changed)?).map_err(|_| inaccessible())?
            != facts.worktree
        || facts.branch
            != format!(
                "refs/heads/manyhands/{}/{}",
                authoring_kind_segment(&kind),
                item
            )
        || worktree.head().map_err(|_| changed())?.name() != Some(facts.branch.as_str())
    {
        return Err(changed());
    }
    let registered = root
        .worktrees()
        .map_err(|_| inaccessible())?
        .iter()
        .flatten()
        .any(|name| {
            root.find_worktree(name)
                .ok()
                .is_some_and(|w| w.path() == facts.worktree)
        });
    if !registered {
        return Err(changed());
    }
    let index = worktree.index().map_err(|_| inaccessible())?;
    if index.has_conflicts() {
        return Err(ReadError::new(ResultCode::WorktreeConflicted));
    }
    let metadata = integration::artifacts::MergeMetadataDirectory::open(&worktree)
        .map_err(|_| ReadError::new(ResultCode::RecoveryRequired))?;
    for member in ["MERGE_HEAD", "MERGE_MSG", "MERGE_MODE", "index.lock"] {
        if metadata
            .digest(member)
            .map_err(|_| ReadError::new(ResultCode::RecoveryRequired))?
            .is_some()
        {
            return Err(ReadError::new(ResultCode::RecoveryRequired));
        }
    }
    let mut options = git2::StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .update_index(false);
    if !worktree
        .statuses(Some(&mut options))
        .map_err(|_| inaccessible())?
        .is_empty()
    {
        return Err(ReadError::new(ResultCode::WorktreeNotClean));
    }
    if worktree.state() != git2::RepositoryState::Clean {
        return Err(ReadError::new(ResultCode::RecoveryRequired));
    }
    let context = worktree
        .refname_to_id(&facts.branch)
        .map_err(|_| changed())?;
    let primary = root
        .refname_to_id(&format!("refs/heads/{}", facts.primary))
        .map_err(|_| changed())?;
    root.find_commit(primary).map_err(|_| inaccessible())?;
    worktree.find_commit(context).map_err(|_| inaccessible())?;
    let not_found = || ReadError::new(ResultCode::ItemNotFound);
    if !facts
        .item_path
        .to_str()
        .is_some_and(read::is_plain_relative)
    {
        return Err(not_found());
    }
    let tree = worktree
        .find_commit(context)
        .and_then(|c| c.tree())
        .map_err(|_| inaccessible())?;
    let entry = tree.get_path(&facts.item_path).map_err(|_| not_found())?;
    if !matches!(entry.filemode(), 0o100644 | 0o100755) {
        return Err(not_found());
    }
    let blob = worktree.find_blob(entry.id()).map_err(|_| inaccessible())?;
    let parsed = crate::canonical::parse_item(
        &facts.item_path,
        std::str::from_utf8(blob.content()).map_err(|_| not_found())?,
    )
    .map_err(|_| not_found())?;
    let matches = match parsed {
        crate::canonical::CanonicalItem::Document(document) => {
            kind == AuthoringKind::Document && document.id == *item
        }
        crate::canonical::CanonicalItem::Ticket(ticket) => {
            kind == AuthoringKind::Ticket && ticket.id == *item
        }
        _ => false,
    };
    if !matches {
        return Err(not_found());
    }
    // Only routing configuration enters the input. Effective user.name/email
    // are deliberately excluded, as are uncommitted primary authoring files.
    let configuration =
        fs::read(repo.root().join(".manyhands/config.toml")).map_err(|_| inaccessible())?;
    let routing = crate::canonical::parse_repository_config(
        std::str::from_utf8(&configuration)
            .map_err(|_| ReadError::new(ResultCode::InvalidConfiguration))?,
    )
    .map_err(|_| ReadError::new(ResultCode::InvalidConfiguration))?;
    if routing.primary_branch != facts.primary || routing.publication_remote != facts.remote {
        return Err(changed());
    }
    Ok(GitImage {
        context,
        primary,
        index: metadata
            .digest("index")
            .map_err(|_| inaccessible())?
            .ok_or_else(inaccessible)?,
        configuration: *blake3::hash(&configuration).as_bytes(),
        common,
    })
}

pub(super) fn observe_with(
    service: &RepositoryService,
    repo: &ResolvedRepository,
    kind: AuthoringKind,
    item: &ItemId,
    between: impl FnOnce(),
) -> Result<LocalContextSyncObservationDto, ReadError> {
    let result = (|| {
        let a =
            service.read_session(RepositoryOperation::Read, |db| facts(db, repo, kind, item))?;
        let git_a = git_image(repo, &a, kind, item)?;
        between();
        let b = service
            .read_session(RepositoryOperation::Read, |db| facts(db, repo, kind, item))
            .map_err(|_| ReadError::new(ResultCode::ExternalChange))?;
        if a != b {
            return Err(ReadError::new(ResultCode::ExternalChange));
        }
        let git_b = git_image(repo, &b, kind, item)
            .map_err(|_| ReadError::new(ResultCode::ExternalChange))?;
        if git_a != git_b {
            return Err(ReadError::new(ResultCode::ExternalChange));
        }
        seal(repo, &a, &git_a, kind, item)
    })();
    result.map_err(|mut error: ReadError| {
        error.scope = repo.scope();
        error.scope.item_id = Some(item.to_string());
        error
    })
}

#[cfg(test)]
pub(super) fn from_facts(
    repo: &ResolvedRepository,
    facts: &Facts,
    kind: AuthoringKind,
    item: &ItemId,
) -> Result<LocalContextSyncObservationDto, ReadError> {
    seal(
        repo,
        facts,
        &git_image(repo, facts, kind, item)?,
        kind,
        item,
    )
}

fn seal(
    repo: &ResolvedRepository,
    a: &Facts,
    git_a: &GitImage,
    kind: AuthoringKind,
    item: &ItemId,
) -> Result<LocalContextSyncObservationDto, ReadError> {
    let mut hash = blake3::Hasher::new();
    for value in [
        b"manyhands local context observation v1".as_slice(),
        repo.root().as_os_str().as_encoded_bytes(),
        a.worktree.as_os_str().as_encoded_bytes(),
        a.item_path.as_os_str().as_encoded_bytes(),
        git_a.common.as_os_str().as_encoded_bytes(),
        a.branch.as_bytes(),
        a.primary.as_bytes(),
        authoring_kind_segment(&kind).as_bytes(),
        item.to_string().as_bytes(),
        git_a.context.as_bytes(),
        git_a.primary.as_bytes(),
        &git_a.index,
        &git_a.configuration,
        &a.registration,
    ] {
        field(&mut hash, value);
    }
    field(&mut hash, &a.repository_id.to_be_bytes());
    field(&mut hash, &a.context_id.to_be_bytes());
    field(&mut hash, a.remote.as_deref().unwrap_or("").as_bytes());
    for semantic in &a.semantics {
        field(&mut hash, semantic);
    }
    Ok(LocalContextSyncObservationDto {
        kind: authoring_kind_segment(&kind).into(),
        item_id: item.to_string(),
        context_worktree: a
            .worktree
            .to_str()
            .ok_or_else(|| ReadError::new(ResultCode::InvalidPath))?
            .into(),
        context_ref: a.branch.clone(),
        primary_ref: format!("refs/heads/{}", a.primary),
        context_head: git_a.context.to_string(),
        primary_head: git_a.primary.to_string(),
        observation: format!("local-sync-v1:{}", hash.finalize().to_hex()),
    })
}

impl RepositoryService {
    /// Observe an existing context without accepting journal ownership.
    /// The journal record and mutating store seams are not public APIs.
    ///
    /// ```compile_fail
    /// use manyhands::repository::LocalContextSyncRecord;
    /// ```
    ///
    /// ```compile_fail
    /// use manyhands::{canonical::ItemId, repository::*};
    /// fn local_sync_begin_is_internal(s: &RepositoryService, r: &ResolvedRepository, id: &ItemId) {
    ///     let _ = s.begin_local_context_sync(r, OperationId::new(), AuthoringKind::Ticket, id, "token");
    /// }
    /// ```
    ///
    /// ```compile_fail
    /// use manyhands::repository::*;
    /// fn local_sync_read_is_internal(s: &RepositoryService, r: &ResolvedRepository) {
    ///     let _ = s.read_local_context_sync_record(r, OperationId::new());
    /// }
    /// ```
    ///
    /// ```compile_fail
    /// use manyhands::{canonical::ItemId, repository::*};
    /// fn local_sync_takeover_is_internal(s: &RepositoryService, r: &ResolvedRepository, id: &ItemId) {
    ///     let row = s.begin_local_context_sync(r, OperationId::new(), AuthoringKind::Ticket, id, "token").unwrap();
    ///     let _ = s.take_over_local_context_sync(r, &row);
    /// }
    /// ```
    pub fn observe_local_context_sync(
        &self,
        repo: &ResolvedRepository,
        kind: AuthoringKind,
        id: &ItemId,
    ) -> Result<LocalContextSyncObservationDto, ReadError> {
        observe_with(self, repo, kind, id, || {})
    }
}
