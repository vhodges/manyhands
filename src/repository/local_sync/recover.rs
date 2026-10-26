//! Exact pre/post-state reconciliation for the local executor.
use super::{
    record::{self, LocalContextSyncRecord},
    run::{Fault, Reason},
};
use crate::{repository::*, results::ResultCode};
use rusqlite::{Connection, Transaction, TransactionBehavior, params};
use std::{fs, path::Path};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Disposition {
    Noop,
    FastForward,
    Merge,
}
impl Disposition {
    fn name(self) -> &'static str {
        match self {
            Self::Noop => "noop",
            Self::FastForward => "fast_forward",
            Self::Merge => "merge",
        }
    }
}

/// Only hashes, OIDs and effect flags are durable. No source or candidate bytes.
#[derive(Clone)]
pub(super) struct Execution {
    pub disposition: Disposition,
    pub routing: [u8; 32],
    pub index: [u8; 32],
    pub log: [u8; 32],
    pub head_log: [u8; 32],
    pub ref_time: i64,
    pub commit_identity: Option<(String, String, i64, i32)>,
    pub commit_observed: bool,
    pub checkout_started: bool,
    pub checkout_observed: bool,
    pub ref_started: bool,
    pub ref_observed: bool,
}
impl Execution {
    pub fn prepare(
        repo: &ResolvedRepository,
        git: &git2::Repository,
        branch: &str,
        disposition: Disposition,
    ) -> Result<Self, ReadError> {
        Ok(Self {
            disposition,
            routing: routing(repo)?,
            index: index_digest(git)?,
            log: log_digest(git, branch)?,
            head_log: head_log_digest(git)?,
            ref_time: time::OffsetDateTime::now_utc().unix_timestamp(),
            commit_identity: None,
            commit_observed: false,
            checkout_started: false,
            checkout_observed: false,
            ref_started: false,
            ref_observed: false,
        })
    }
    pub fn insert(&self, tx: &Connection, id: OperationId) -> Result<(), ReadError> {
        tx.execute("INSERT INTO local_context_sync_execution(operation_ulid,format_version,disposition,routing_digest,baseline_index,baseline_log,baseline_head_log,ref_time) VALUES(?1,1,?2,?3,?4,?5,?6,?7)",params![id.to_string(),self.disposition.name(),self.routing.as_slice(),self.index.as_slice(),self.log.as_slice(),self.head_log.as_slice(),self.ref_time])?;
        Ok(())
    }
    pub fn read(db: &Connection, id: OperationId) -> Result<Self, ReadError> {
        db.query_row("SELECT e.*,EXISTS(SELECT 1 FROM local_context_sync_steps s WHERE s.operation_ulid=e.operation_ulid AND s.kind='objects' AND s.phase='observed') AS commit_observed FROM local_context_sync_execution e WHERE operation_ulid=?1",[id.to_string()],|r| {
            let hash=|name| -> rusqlite::Result<[u8;32]> {r.get::<_,Vec<u8>>(name)?.try_into().map_err(|_|rusqlite::Error::InvalidQuery)};
            let disposition=match r.get::<_,String>("disposition")?.as_str() {"noop"=>Disposition::Noop,"fast_forward"=>Disposition::FastForward,"merge"=>Disposition::Merge,_=>return Err(rusqlite::Error::InvalidQuery)};
            let commit_identity=r.get::<_,Option<String>>("commit_name")?.map(|name| -> rusqlite::Result<_> {Ok((name,r.get("commit_email")?,r.get("commit_time")?,r.get("commit_offset")?))}).transpose()?;
            Ok(Self {disposition,routing:hash("routing_digest")?,index:hash("baseline_index")?,log:hash("baseline_log")?,head_log:hash("baseline_head_log")?,ref_time:r.get("ref_time")?,commit_identity,commit_observed:r.get("commit_observed")?,checkout_started:r.get("checkout_started")?,checkout_observed:r.get("checkout_observed")?,ref_started:r.get("ref_started")?,ref_observed:r.get("ref_observed")?})
        }).map_err(|_|ReadError::new(ResultCode::RecoveryRequired))
    }
}

pub(super) fn routing(repo: &ResolvedRepository) -> Result<[u8; 32], ReadError> {
    let source = fs::read(repo.root().join(".manyhands/config.toml"))
        .map_err(|_| ReadError::new(ResultCode::RepositoryInaccessible))?;
    Ok(*blake3::hash(&source).as_bytes())
}
pub(super) fn index_digest(git: &git2::Repository) -> Result<[u8; 32], ReadError> {
    integration::artifacts::MergeMetadataDirectory::open(git)
        .and_then(|m| m.digest("index"))
        .map_err(|_| ReadError::new(ResultCode::RecoveryRequired))?
        .ok_or_else(|| ReadError::new(ResultCode::RecoveryRequired))
}
fn bytes(root: &Path, path: &Path) -> Result<Vec<u8>, ReadError> {
    owned_resolution_file_bytes(root, path, RepositoryOperation::ContextSyncLocal, root)
        .map(|b| b.unwrap_or_default())
        .map_err(|_| ReadError::new(ResultCode::RecoveryRequired))
}
fn log_bytes(git: &git2::Repository, branch: &str) -> Result<Vec<u8>, ReadError> {
    bytes(git.commondir(), &Path::new("logs").join(branch))
}
fn log_digest(git: &git2::Repository, branch: &str) -> Result<[u8; 32], ReadError> {
    Ok(*blake3::hash(&log_bytes(git, branch)?).as_bytes())
}
fn head_log_digest(git: &git2::Repository) -> Result<[u8; 32], ReadError> {
    Ok(*blake3::hash(&bytes(git.path(), Path::new("logs/HEAD"))?).as_bytes())
}

pub(super) fn clean_metadata(git: &git2::Repository, branch: &str) -> Result<(), Fault> {
    let metadata = integration::artifacts::MergeMetadataDirectory::open(git)?;
    for member in [
        "MERGE_HEAD",
        "MERGE_MSG",
        "MERGE_MODE",
        "index.lock",
        "HEAD.lock",
    ] {
        if metadata.digest(member)?.is_some() {
            return Err(Fault::ambiguous());
        }
    }
    for relative in [
        format!("{branch}.lock"),
        format!("logs/{branch}.lock"),
        "packed-refs.lock".into(),
    ] {
        match fs::symlink_metadata(git.commondir().join(relative)) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            _ => return Err(Fault::ambiguous()),
        }
    }
    if git.state() != git2::RepositoryState::Clean {
        return Err(Fault::ambiguous());
    }
    Ok(())
}

fn entries(index: &git2::Index) -> Vec<(Vec<u8>, u32, git2::Oid, u16)> {
    index
        .iter()
        .map(|e| (e.path, e.mode, e.id, (e.flags >> 12) & 3))
        .collect()
}
/// Compare against the frozen tree, not HEAD (which our own checkout may precede).
/// Git's status alone is insufficient: it can trust cached stat information.
pub(super) fn exact_image(git: &git2::Repository, oid: git2::Oid) -> Result<bool, Fault> {
    let tree = git.find_commit(oid)?.tree()?;
    let mut expected = git2::Index::new()?;
    expected.read_tree(&tree)?;
    // Another handle performs the bounded backend call. Never compare its
    // durable image with libgit2's cached pre-checkout in-memory index.
    let mut actual = git.index()?;
    actual.read(true)?;
    if entries(&expected) != entries(&actual) {
        return Ok(false);
    }
    let root = git.workdir().ok_or_else(Fault::foreign)?;
    for entry in expected.iter() {
        if !matches!(entry.mode, 0o100644 | 0o100755) {
            return Err(Fault::foreign());
        }
        let path = std::str::from_utf8(&entry.path).map_err(|_| Fault::foreign())?;
        let content = owned_file_bytes(
            root,
            Path::new(path),
            RepositoryOperation::ContextSyncLocal,
            root,
        )
        .map_err(|_| Fault::foreign())?;
        if content.as_deref() != Some(git.find_blob(entry.id)?.content()) {
            return Ok(false);
        }
    }
    let mut options = git2::StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .update_index(false);
    if git.statuses(Some(&mut options))?.iter().any(|e| {
        e.status().intersects(
            git2::Status::WT_NEW
                | git2::Status::WT_MODIFIED
                | git2::Status::WT_DELETED
                | git2::Status::WT_TYPECHANGE
                | git2::Status::WT_RENAMED
                | git2::Status::CONFLICTED,
        )
    }) {
        return Ok(false);
    }
    Ok(true)
}

pub(super) struct Owner<'a> {
    pub service: &'a RepositoryService,
    pub repo: &'a ResolvedRepository,
    pub git: git2::Repository,
    pub lease: coordination::RepositoryLease,
    pub row: LocalContextSyncRecord,
}
impl Owner<'_> {
    pub fn transaction<T>(
        &mut self,
        change: impl FnOnce(&Transaction<'_>, &LocalContextSyncRecord) -> Result<T, ReadError>,
    ) -> Result<T, Fault> {
        if !self
            .lease
            .still_names(&self.git)
            .map_err(|_| Fault::foreign())?
        {
            return Err(Fault::foreign());
        }
        let _cache = cache_write_guard(
            &self.service.registry_path,
            self.repo.root(),
            RepositoryOperation::ContextSyncLocal,
        )?;
        let mut db = record::open_for_write(self.service)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(ReadError::from)?;
        record::audit(&tx).map_err(|_| Fault::authority())?;
        let current=tx.query_row("SELECT s.* FROM local_context_sync_records s JOIN repositories r ON r.id=s.repository_id WHERE s.operation_ulid=?1 AND r.id=?2 AND r.root_path=?3",params![self.row.operation_id.to_string(),self.repo.registration_id(),self.repo.root().to_str()],record::decode).map_err(|_|Fault::authority())?;
        if current != self.row {
            return Err(Fault::foreign());
        }
        let result = change(&tx, &current)?;
        let row = tx
            .query_row(
                "SELECT * FROM local_context_sync_records WHERE operation_ulid=?1",
                [current.operation_id.to_string()],
                record::decode,
            )
            .map_err(|_| Fault::authority())?;
        record::audit(&tx).map_err(|_| Fault::authority())?;
        tx.commit().map_err(ReadError::from)?;
        self.row = row;
        Ok(result)
    }
    pub fn evidence(&mut self) -> Result<Execution, Fault> {
        self.transaction(|tx, row| Execution::read(tx, row.operation_id))
    }
    pub fn authorize(&mut self) -> Result<(), Fault> {
        let facts = self
            .transaction(|tx, row| super::observe::facts(tx, self.repo, row.kind, &row.item_id))?;
        if facts.worktree != self.row.worktree
            || facts.branch != self.row.context_ref
            || format!("refs/heads/{}", facts.primary) != self.row.primary_ref
        {
            return Err(Fault::foreign());
        }
        let root = git2::Repository::open(self.repo.root())?;
        if fs::canonicalize(root.commondir()).map_err(|_| Fault::foreign())?
            != fs::canonicalize(self.git.commondir()).map_err(|_| Fault::foreign())?
            || self.git.workdir() != Some(self.row.worktree.as_path())
            || self.git.head()?.name() != Some(self.row.context_ref.as_str())
        {
            return Err(Fault::foreign());
        }
        let registered = root.worktrees()?.iter().flatten().any(|name| {
            root.find_worktree(name)
                .ok()
                .is_some_and(|w| w.path() == self.row.worktree)
        });
        if !registered || routing(self.repo)? != self.evidence()?.routing {
            return Err(Fault::foreign());
        }
        Ok(())
    }
    pub fn step(
        &mut self,
        ordinal: i64,
        kind: &str,
        observed: bool,
        new: Option<git2::Oid>,
    ) -> Result<(), Fault> {
        let index = index_digest(&self.git)?;
        self.transaction(|tx,row| {
            let empty=blake3::hash(b"local-sync-no-merge-metadata-v1");
            if observed {
                if tx.execute("UPDATE local_context_sync_steps SET phase='observed',owner_epoch=?3,expected_index_digest=?4 WHERE operation_ulid=?1 AND ordinal=?2 AND generation=?5",params![row.operation_id.to_string(),ordinal,row.owner_epoch,index.as_slice(),row.generation])?!=1 {return Err(ReadError::new(ResultCode::RecoveryRequired));}
            } else {
                tx.execute("INSERT INTO local_context_sync_steps(operation_ulid,ordinal,generation,owner_epoch,kind,phase,baseline_index_digest,baseline_metadata_digest,expected_old_oid,expected_new_oid) VALUES(?1,?2,?3,?4,?5,'prepared',?6,?7,?8,?9) ON CONFLICT(operation_ulid,ordinal) DO NOTHING",params![row.operation_id.to_string(),ordinal,row.generation,row.owner_epoch,kind,index.as_slice(),empty.as_bytes().as_slice(),row.context_parent.to_string(),new.map(|o|o.to_string())])?;
            }
            let matching:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM local_context_sync_steps WHERE operation_ulid=?1 AND ordinal=?2 AND generation=?3 AND kind=?4 AND expected_old_oid=?5 AND expected_new_oid IS ?6)",params![row.operation_id.to_string(),ordinal,row.generation,kind,row.context_parent.to_string(),new.map(|o|o.to_string())],|r|r.get(0))?;
            if !matching {return Err(ReadError::new(ResultCode::RecoveryRequired));}
            if !observed && tx.execute("UPDATE local_context_sync_steps SET owner_epoch=?3 WHERE operation_ulid=?1 AND ordinal=?2",params![row.operation_id.to_string(),ordinal,row.owner_epoch])?!=1 {return Err(ReadError::new(ResultCode::RecoveryRequired));}
            Ok(())
        })
    }
    pub fn flag(&mut self, column: &str) -> Result<(), Fault> {
        // Column names are fixed private call-site constants, never wire input.
        self.transaction(|tx, row| {
            if tx.execute(
                &format!(
                    "UPDATE local_context_sync_execution SET {column}=1 WHERE operation_ulid=?1"
                ),
                [row.operation_id.to_string()],
            )? != 1
            {
                return Err(ReadError::new(ResultCode::RecoveryRequired));
            }
            Ok(())
        })
    }
    pub fn phase(&mut self, phase: &str) -> Result<(), Fault> {
        self.transaction(|tx,row| {
            if tx.execute("UPDATE local_context_sync_records SET state=?2,semantic_revision=semantic_revision+1 WHERE operation_ulid=?1",params![row.operation_id.to_string(),phase])?!=1 {return Err(ReadError::new(ResultCode::RecoveryRequired));}
            Ok(())
        })
    }
}

/// A fully proven result requires the one expected log append, not just the ref.
pub(super) fn ref_proof(owner: &Owner<'_>, e: &Execution, result: bool) -> Result<bool, Fault> {
    let bytes = log_bytes(&owner.git, &owner.row.context_ref)?;
    if !result {
        return Ok(
            *blake3::hash(&bytes).as_bytes() == e.log && head_log_digest(&owner.git)? == e.head_log
        );
    }
    let new = owner.row.candidate.ok_or_else(Fault::authority)?;
    let expected = format!(
        "{} {} Manyhands <local-sync@manyhands.invalid> {} +0000\tmanyhands local sync {}\n",
        owner.row.context_parent, new, e.ref_time, owner.row.operation_id
    );
    let head = bytes_for_head(owner)?;
    Ok(bytes
        .strip_suffix(expected.as_bytes())
        .is_some_and(|prefix| *blake3::hash(prefix).as_bytes() == e.log)
        && head
            .strip_suffix(expected.as_bytes())
            .is_some_and(|prefix| *blake3::hash(prefix).as_bytes() == e.head_log))
}
fn bytes_for_head(owner: &Owner<'_>) -> Result<Vec<u8>, ReadError> {
    bytes(owner.git.path(), Path::new("logs/HEAD"))
}

pub(super) fn mismatch_reason(e: &Execution) -> Fault {
    if e.checkout_started || e.ref_started {
        Fault::ambiguous()
    } else {
        Fault {
            code: ResultCode::ExternalChange,
            reason: Reason::ForeignChange,
        }
    }
}

/// Coarse checkout has no per-path resume authority. Matching changed paths can
/// prove writes for effect reporting, but NEVER authorize partial reconciliation.
/// Only exact_image of the complete baseline/result authorizes another effect.
pub(super) fn checkout_progress(owner: &Owner<'_>) -> Result<bool, Fault> {
    let Some(candidate) = owner.row.candidate else {
        return Ok(false);
    };
    let old = owner.git.find_commit(owner.row.context_parent)?.tree()?;
    let new = owner.git.find_commit(candidate)?.tree()?;
    let root = owner.git.workdir().ok_or_else(Fault::foreign)?;
    for delta in owner
        .git
        .diff_tree_to_tree(Some(&old), Some(&new), None)?
        .deltas()
    {
        let Some(path) = delta.new_file().path().or_else(|| delta.old_file().path()) else {
            continue;
        };
        let Ok(actual) = owned_file_bytes(root, path, RepositoryOperation::ContextSyncLocal, root)
        else {
            continue;
        };
        if delta.new_file().id().is_zero() {
            if actual.is_none() {
                return Ok(true);
            }
        } else if let Ok(blob) = owner.git.find_blob(delta.new_file().id())
            && actual.as_deref() == Some(blob.content())
        {
            return Ok(true);
        }
    }
    Ok(false)
}
