//! Internal conflict domain used by request-fenced execution and read-only views.
use super::{
    conflict_record::Image,
    recover::{self, Owner},
    run::{Fault, Reason},
};
use crate::{repository::*, results::ResultCode};
use rusqlite::{OptionalExtension, params};
use std::{collections::BTreeSet, path::Path};

pub(super) const MEMBERS: [&str; 3] = ["MERGE_HEAD", "MERGE_MSG", "MERGE_MODE"];
#[cfg(test)]
thread_local! {pub(super) static INSPECT_HOOK:std::cell::RefCell<Option<Box<dyn FnMut()>>>=const {std::cell::RefCell::new(None)};}
pub(super) fn backend_unlocked(owner: &Owner<'_>) -> Result<(), Fault> {
    backend_unlocked_in(&owner.git, &owner.row.context_ref)
}
pub(super) fn backend_unlocked_in(git: &git2::Repository, context_ref: &str) -> Result<(), Fault> {
    let metadata = integration::artifacts::MergeMetadataDirectory::open(git)?;
    for member in ["index.lock", "HEAD.lock"] {
        if metadata.digest(member)?.is_some() {
            return Err(Fault::ambiguous());
        }
    }
    for member in [
        format!("{context_ref}.lock"),
        format!("logs/{context_ref}.lock"),
        "packed-refs.lock".into(),
    ] {
        match std::fs::symlink_metadata(git.commondir().join(member)) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            _ => return Err(Fault::ambiguous()),
        }
    }
    Ok(())
}

pub(super) struct Entry {
    pub token: String,
    pub path: Vec<u8>,
    pub sides: [Option<git2::Oid>; 3],
    pub readable: bool,
}
pub(super) struct Inspection {
    pub observation: String,
    pub paths: Vec<Entry>,
    pub eligibility: integration::validate::ConflictEligibility,
    pub can_start_resolution: bool,
}
// Deliberately no Debug/Serialize: side source is ephemeral explicit content.
pub(super) struct Content {
    pub base: Option<String>,
    pub local: Option<String>,
    pub incoming: Option<String>,
    pub historical: bool,
}
pub(super) fn enumerate(
    repo: &ResolvedRepository,
    git: &git2::Repository,
    row: &super::record::LocalContextSyncRecord,
) -> Result<(Vec<Entry>, integration::validate::ConflictEligibility), Fault> {
    let index = prepared(git, row)?;
    let mut paths = Vec::new();
    let mut candidates = Vec::new();
    for conflict in index.conflicts()? {
        let conflict = conflict?;
        candidates.push(integration::validate::classify_conflict_entry(
            git, &conflict,
        ));
        let stages = [conflict.ancestor, conflict.our, conflict.their];
        let path = stages
            .iter()
            .flatten()
            .next()
            .ok_or_else(Fault::authority)?
            .path
            .clone();
        let mut hash = blake3::Hasher::new();
        hash.update(b"manyhands-local-conflict-token-v1");
        for field in [
            repo.root().as_os_str().as_encoded_bytes(),
            row.operation_id.to_string().as_bytes(),
            &row.generation.to_be_bytes(),
            &path,
        ] {
            super::observe::field(&mut hash, field);
        }
        for stage in &stages {
            if let Some(e) = stage {
                super::observe::field(&mut hash, &e.path);
                super::observe::field(&mut hash, &e.mode.to_be_bytes());
                super::observe::field(&mut hash, e.id.as_bytes());
            } else {
                super::observe::field(&mut hash, b"");
            }
        }
        paths.push(Entry {
            token: format!("local-conflict-v1:{}", hash.finalize().to_hex()),
            path,
            sides: stages.each_ref().map(|e| e.as_ref().map(|e| e.id)),
            readable: stages.iter().flatten().all(|e| {
                e.mode == 0o100644
                    && git
                        .find_blob(e.id)
                        .is_ok_and(|b| std::str::from_utf8(b.content()).is_ok())
            }),
        });
    }
    paths.sort_by(|a, b| a.path.cmp(&b.path));
    Ok((
        paths,
        integration::validate::conflict_eligibility(&candidates),
    ))
}
fn live_observation(
    row: &super::record::LocalContextSyncRecord,
    image: &Image,
) -> Result<String, Fault> {
    let mut hash = blake3::Hasher::new();
    hash.update(b"manyhands-local-resolution-observation-v1");
    super::observe::field(&mut hash, row.observation.as_bytes());
    super::observe::field(&mut hash, &row.semantic_revision.to_be_bytes());
    super::observe::field(
        &mut hash,
        &serde_json::to_vec(image).map_err(|_| Fault::authority())?,
    );
    Ok(format!("local-sync-v1:{}", hash.finalize().to_hex()))
}
pub(super) fn inspect(
    service: &RepositoryService,
    repo: &ResolvedRepository,
    id: OperationId,
) -> Result<Inspection, Fault> {
    let (row, evidence) = super::run::read_evidence(service, repo, id)?;
    let authority = read_authority(service, repo, &row)?;
    if recover::routing(repo)? != evidence.routing {
        return Err(Fault::foreign());
    }
    if matches!(
        row.state,
        super::record::LocalSyncState::Complete
            | super::record::LocalSyncState::Abandoned
            | super::record::LocalSyncState::Cancelled
    ) {
        return Err(Fault::foreign());
    }
    let git = git2::Repository::open(&row.worktree)?;
    let before = image(&git)?;
    let (paths, eligibility) = enumerate(repo, &git, &row)?;
    let head = git.refname_to_id(&row.context_ref)?;
    let stored: Option<Vec<u8>> = service.read_session(RepositoryOperation::Read, |db| {
        Ok(db.query_row(
            "SELECT manifest FROM local_context_sync_conflicts WHERE operation_ulid=?1",
            [id.to_string()],
            |r| r.get(0),
        )?)
    })?;
    let installed: Image = serde_json::from_slice(&stored.ok_or_else(Fault::authority)?)
        .map_err(|_| Fault::authority())?;
    validate_snapshot(&git, &row, &installed)?;
    #[cfg(test)]
    INSPECT_HOOK.with(|h| {
        if let Some(h) = h.borrow_mut().as_mut() {
            h()
        }
    });
    let (mut again, _) = super::run::read_evidence(service, repo, id)?;
    again.owner_epoch = row.owner_epoch;
    if row != again
        || authority != read_authority(service, repo, &again)?
        || before != image(&git)?
        || head != git.refname_to_id(&row.context_ref)?
        || recover::routing(repo)? != evidence.routing
    {
        return Err(Fault::foreign());
    }
    Ok(Inspection {
        observation: live_observation(&row, &before)?,
        paths,
        eligibility,
        can_start_resolution: row.active_resolution.is_none()
            && row.state == super::record::LocalSyncState::ConflictPending
            && before == installed
            && head == row.context_parent
            && eligibility == integration::validate::ConflictEligibility::EligibleCanonical,
    })
}
fn read_authority(
    service: &RepositoryService,
    repo: &ResolvedRepository,
    row: &super::record::LocalContextSyncRecord,
) -> Result<(super::observe::Facts, (i64, i64)), Fault> {
    Ok(service.read_session(RepositoryOperation::Read,|db| {
        let facts=super::observe::facts(db,repo,row.kind,&row.item_id)?;
        if facts.worktree!=row.worktree || facts.branch!=row.context_ref || format!("refs/heads/{}",facts.primary)!=row.primary_ref {return Err(ReadError::new(ResultCode::ExternalChange));}
        let cancellation=if let Some(attempt)=row.active_resolution {db.query_row("SELECT issued,acknowledged FROM local_context_sync_resolution_cancellation WHERE attempt_ulid=?1",[attempt.to_string()],|r|Ok((r.get(0)?,r.get(1)?)))?} else {(0,0)};
        Ok((facts,cancellation))
    })?)
}
fn validate_snapshot(
    git: &git2::Repository,
    row: &super::record::LocalContextSyncRecord,
    snapshot: &Image,
) -> Result<(), Fault> {
    if snapshot.index != entries(&prepared(git, row)?)
        || snapshot
            .paths
            .iter()
            .map(|p| p.0.clone())
            .collect::<BTreeSet<_>>()
            != snapshot.index.iter().map(|p| p.0.clone()).collect()
        || snapshot.metadata[0]
            != Some(*blake3::hash(format!("{}\n", row.primary_parent).as_bytes()).as_bytes())
    {
        return Err(Fault::authority());
    }
    Ok(())
}
pub(super) fn read(
    service: &RepositoryService,
    repo: &ResolvedRepository,
    id: OperationId,
    token: &str,
) -> Result<Content, Fault> {
    if !valid_token(token) {
        return Err(Fault {
            code: ResultCode::InvalidInput,
            reason: Reason::ForeignChange,
        });
    }
    let (row, _) = super::run::read_evidence(service, repo, id)?;
    let inspection = inspect(service, repo, id)?;
    let entry = inspection
        .paths
        .into_iter()
        .find(|e| e.token == token)
        .ok_or_else(Fault::foreign)?;
    if !entry.readable {
        return Err(Fault::foreign());
    }
    let git = git2::Repository::open(&row.worktree)?;
    let mut sides = Vec::new();
    for oid in entry.sides {
        sides.push(
            oid.map(|oid| {
                let access: std::io::Result<()> = if service
                    .should_inject(
                        FailurePoint::LocalSyncBeforeSideRead,
                        RepositoryOperation::Read,
                        repo.root(),
                    )
                    .unwrap_or(false)
                {
                    Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
                } else {
                    Ok(())
                };
                access.map_err(|error| Fault {
                    code: if error.kind() == std::io::ErrorKind::PermissionDenied {
                        ResultCode::RepositoryInaccessible
                    } else {
                        ResultCode::RecoveryRequired
                    },
                    reason: Reason::AuthorityMissing,
                })?;
                let blob = git.find_blob(oid)?;
                String::from_utf8(blob.content().to_vec()).map_err(|_| Fault::foreign())
            })
            .transpose()?,
        );
    }
    let mut again = super::run::read_evidence(service, repo, id)?.0;
    again.owner_epoch = row.owner_epoch;
    if row != again {
        return Err(Fault::foreign());
    }
    let mut sides = sides.into_iter();
    Ok(Content {
        base: sides.next().flatten(),
        local: sides.next().flatten(),
        incoming: sides.next().flatten(),
        historical: row.active_resolution.is_some(),
    })
}
pub(super) fn valid_token(token: &str) -> bool {
    token.strip_prefix("local-conflict-v1:").is_some_and(|s| {
        s.len() == 64
            && s.bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    })
}
pub(super) fn prepared(
    git: &git2::Repository,
    row: &super::record::LocalContextSyncRecord,
) -> Result<git2::Index, Fault> {
    Ok(integration::prepare::prepare_recorded_merge_index(
        git,
        row.context_parent,
        row.primary_parent,
    )?)
}
pub(super) fn has_conflicts(owner: &Owner<'_>) -> Result<bool, Fault> {
    Ok(prepared(&owner.git, &owner.row)?.has_conflicts())
}
pub(super) fn entries(index: &git2::Index) -> Vec<(Vec<u8>, u32, String, u16)> {
    index
        .iter()
        .map(|e| (e.path, e.mode, e.id.to_string(), (e.flags >> 12) & 3))
        .collect()
}
pub(super) fn image(git: &git2::Repository) -> Result<Image, Fault> {
    let mut index = git.index()?;
    index.read(true)?;
    let entries = entries(&index);
    let paths = entries.iter().map(|e| e.0.clone()).collect::<BTreeSet<_>>();
    let root = git.workdir().ok_or_else(Fault::foreign)?;
    let paths = paths
        .into_iter()
        .map(|path| {
            let text = std::str::from_utf8(&path).map_err(|_| Fault::foreign())?;
            let image = super::file_image::read(root, Path::new(text)).map_err(Fault::from)?;
            let (digest, mode) = image
                .map(|i| (Some(*blake3::hash(&i.bytes).as_bytes()), i.mode))
                .unwrap_or((None, None));
            Ok((path, digest, mode))
        })
        .collect::<Result<Vec<_>, Fault>>()?;
    let metadata = integration::artifacts::MergeMetadataDirectory::open(git)?;
    Ok(Image {
        index: entries,
        paths,
        metadata: [
            metadata.digest(MEMBERS[0])?,
            metadata.digest(MEMBERS[1])?,
            metadata.digest(MEMBERS[2])?,
        ],
    })
}
pub(super) fn stored(owner: &mut Owner<'_>) -> Result<Option<Option<Image>>, Fault> {
    let result = owner.transaction(|db, row| {
        let bytes: Option<Option<Vec<u8>>> = db
            .query_row(
                "SELECT manifest FROM local_context_sync_conflicts WHERE operation_ulid=?1",
                [row.operation_id.to_string()],
                |r| r.get(0),
            )
            .optional()?;
        bytes
            .map(|b| {
                b.map(|b| {
                    serde_json::from_slice(&b)
                        .map_err(|_| ReadError::new(ResultCode::RecoveryRequired))
                })
                .transpose()
            })
            .transpose()
    })?;
    if let Some(Some(image)) = &result {
        validate_snapshot(&owner.git, &owner.row, image)?;
    }
    Ok(result)
}
pub(super) fn install(owner: &mut Owner<'_>) -> Result<ResultCode, Fault> {
    owner.authorize()?;
    let index = prepared(&owner.git, &owner.row)?;
    let eligibility = integration::validate::conflict_eligibility(
        &index
            .conflicts()?
            .map(|c| c.map(|c| integration::validate::classify_conflict_entry(&owner.git, &c)))
            .collect::<Result<Vec<_>, _>>()?,
    );
    let evidence = owner.evidence()?;
    if let Some(Some(expected)) = stored(owner)? {
        if image(&owner.git)? != expected
            || owner.git.refname_to_id(&owner.row.context_ref)? != owner.row.context_parent
        {
            return Err(Fault::foreign());
        }
    } else {
        evidence
            .images
            .as_ref()
            .ok_or_else(Fault::authority)?
            .require_worktree_attributes(&owner.git)?;
        recover::clean_metadata(&owner.git, &owner.row.context_ref)?;
        if !recover::owned_image(owner, owner.row.context_parent)?
            || !recover::ref_proof(owner, &evidence, false)?
        {
            return Err(Fault::ambiguous());
        }
        owner.transaction(|db, row| {
            db.execute("INSERT INTO local_context_sync_conflicts(operation_ulid,generation) VALUES(?1,?2) ON CONFLICT DO NOTHING", params![row.operation_id.to_string(),row.generation])?;
            Ok(())
        })?;
        owner.step(10, "conflict_install", false, None)?;
        owner.phase("applying")?;
        owner.authorize()?;
        integration::artifacts::install_conflict(&owner.git, owner.row.primary_parent)?;
        owner.performed.wrote = true;
        super::resolve::boundary(super::resolve::Point::InstallReturned)?;
        let actual = image(&owner.git)?;
        if actual.index != entries(&index) {
            return Err(Fault::ambiguous());
        }
        let bytes = serde_json::to_vec(&actual).map_err(|_| Fault::authority())?;
        owner.transaction(|db,row| {
            db.execute("UPDATE local_context_sync_conflicts SET manifest=?2 WHERE operation_ulid=?1 AND manifest IS NULL", params![row.operation_id.to_string(),bytes])?;
            db.execute("UPDATE local_context_sync_records SET state='conflict_pending',semantic_revision=semantic_revision+1 WHERE operation_ulid=?1",[row.operation_id.to_string()])?;
            // A conflict install is distinct from the coarse clean checkout.
            Ok(())
        })?;
        owner.step(10, "conflict_install", true, None)?;
    }
    Err(Fault {
        code: ResultCode::RecoveryRequired,
        reason: if eligibility == integration::validate::ConflictEligibility::EligibleCanonical {
            Reason::CanonicalConflict
        } else {
            Reason::UnsupportedConflict
        },
    })
}
