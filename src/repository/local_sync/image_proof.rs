//! Immutable accepted image evidence, not a per-path checkout/resume journal.
//! Only paths, OIDs, modes and digests are stored; source stays ephemeral.
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

const MARKER: &str = "local_context_sync_frozen_images_v6";
const SCHEMA:&str="CREATE TABLE local_context_sync_images (
 operation_ulid TEXT PRIMARY KEY NOT NULL REFERENCES local_context_sync_records(operation_ulid) ON DELETE CASCADE,
 generation INTEGER NOT NULL CHECK(typeof(generation)='integer' AND generation>0),
 owner_epoch INTEGER NOT NULL CHECK(typeof(owner_epoch)='integer' AND owner_epoch>0),
 manifest BLOB NOT NULL CHECK(typeof(manifest)='blob' AND length(manifest)>0),
 baseline_v2 BLOB CHECK(baseline_v2 IS NULL OR (typeof(baseline_v2)='blob' AND length(baseline_v2)=32))
)";

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Proof {
    #[serde(skip)]
    pub baseline_v2: Option<[u8; 32]>,
    version: u32,
    pub coarse_version: u32,
    context: String,
    primary: String,
    tree: String,
    paths: Vec<PathProof>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PathProof {
    path: String,
    before: Option<EntryProof>,
    after: Option<EntryProof>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryProof {
    oid: String,
    mode: u32,
    digests: Vec<[u8; 32]>,
}

fn valid_oid(text: &str) -> bool {
    git2::Oid::from_str(text).is_ok_and(|id| !id.is_zero() && id.to_string() == text)
}
fn decode(bytes: &[u8], context: &str, primary: &str) -> Result<Proof, ()> {
    let proof: Proof = serde_json::from_slice(bytes).map_err(|_| ())?;
    if proof.version != 1
        || !matches!(proof.coarse_version, 1 | 2)
        || proof.context != context
        || proof.primary != primary
        || !valid_oid(&proof.context)
        || !valid_oid(&proof.primary)
        || !valid_oid(&proof.tree)
        || proof.paths.is_empty()
    {
        return Err(());
    }
    let mut last = None;
    for path in &proof.paths {
        if !super::super::read::is_plain_relative(&path.path)
            || last.is_some_and(|p: &str| p >= path.path.as_str())
            || path.before.is_none() && path.after.is_none()
        {
            return Err(());
        }
        last = Some(path.path.as_str());
        for entry in [path.before.as_ref(), path.after.as_ref()]
            .into_iter()
            .flatten()
        {
            if !valid_oid(&entry.oid)
                || !matches!(entry.mode, 0o100644 | 0o100755)
                || entry.digests.is_empty()
                || entry.digests.len() > 2
            {
                return Err(());
            }
        }
        if path.before.as_ref().is_some_and(|e| e.digests.len() != 1) {
            return Err(());
        }
    }
    Ok(proof)
}
fn failure() -> super::super::RepositoryError {
    use super::super::*;
    RepositoryError::new(
        RepositoryOperation::Read,
        None,
        RepositoryErrorKind::RecoveryRequired,
        "local image authority is unavailable",
    )
}
pub(super) fn migrate(db: &Connection) -> Result<(), super::super::RepositoryError> {
    let marked: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [MARKER],
            |r| r.get(0),
        )
        .map_err(|_| failure())?;
    if !marked {
        let exists: bool = db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='local_context_sync_images')",
                [],
                |r| r.get(0),
            )
            .map_err(|_| failure())?;
        if exists {
            return Err(failure());
        }
        db.execute_batch(SCHEMA).map_err(|_| failure())?;
        db.execute("INSERT INTO registry_migrations(name) VALUES(?1)", [MARKER])
            .map_err(|_| failure())?;
    }
    audit(db)
}
pub(super) fn audit(db: &Connection) -> Result<(), super::super::RepositoryError> {
    let marked: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [MARKER],
            |r| r.get(0),
        )
        .map_err(|_| failure())?;
    if !marked {
        return Err(failure());
    }
    let actual: String = db
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name='local_context_sync_images'",
            [],
            |r| r.get(0),
        )
        .map_err(|_| failure())?;
    if actual.split_whitespace().collect::<String>()
        != SCHEMA.split_whitespace().collect::<String>()
    {
        return Err(failure());
    }
    if db
        .prepare("PRAGMA foreign_key_check(local_context_sync_images)")
        .and_then(|mut s| s.exists([]))
        .map_err(|_| failure())?
    {
        return Err(failure());
    }
    let orphan:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM local_context_sync_images i LEFT JOIN local_context_sync_execution e ON e.operation_ulid=i.operation_ulid WHERE e.operation_ulid IS NULL)",[],|r|r.get(0)).map_err(|_|failure())?;
    if orphan {
        return Err(failure());
    }
    let mut stmt=db.prepare("SELECT i.manifest,i.generation,i.owner_epoch,p.generation,p.owner_epoch,p.context_parent,p.primary_parent,i.baseline_v2,e.checkout_started FROM local_context_sync_images i JOIN local_context_sync_records p ON p.operation_ulid=i.operation_ulid JOIN local_context_sync_execution e ON e.operation_ulid=i.operation_ulid").map_err(|_|failure())?;
    let mut rows = stmt.query([]).map_err(|_| failure())?;
    while let Some(r) = rows.next().map_err(|_| failure())? {
        let g: i64 = r.get(1).map_err(|_| failure())?;
        let epoch: i64 = r.get(2).map_err(|_| failure())?;
        if g <= 0
            || epoch <= 0
            || g != r.get::<_, i64>(3).map_err(|_| failure())?
            || epoch > r.get::<_, i64>(4).map_err(|_| failure())?
        {
            return Err(failure());
        }
        let mut proof = decode(
            &r.get::<_, Vec<u8>>(0).map_err(|_| failure())?,
            &r.get::<_, String>(5).map_err(|_| failure())?,
            &r.get::<_, String>(6).map_err(|_| failure())?,
        )
        .map_err(|_| failure())?;
        let baseline: Option<Vec<u8>> = r.get(7).map_err(|_| failure())?;
        proof.baseline_v2 = baseline
            .map(|b| b.try_into().map_err(|_| failure()))
            .transpose()?;
        if (proof.coarse_version == 1 && proof.baseline_v2.is_some())
            || (proof.coarse_version == 2
                && r.get::<_, bool>(8).map_err(|_| failure())?
                && proof.baseline_v2.is_none())
        {
            return Err(failure());
        }
    }
    Ok(())
}

#[cfg(test)]
mod execute {
    use super::super::{recover::Disposition, run::Fault};
    use super::*;
    use crate::repository::*;
    use crate::results::ResultCode;
    type StoredProof = (Vec<u8>, String, String, Option<Vec<u8>>);
    use std::{collections::BTreeMap, path::Path};

    fn index(git: &git2::Repository, oid: git2::Oid) -> Result<git2::Index, Fault> {
        let mut index = git2::Index::new()?;
        index.read_tree(&git.find_commit(oid)?.tree()?)?;
        Ok(index)
    }
    fn entries(index: &git2::Index) -> Vec<(Vec<u8>, u32, git2::Oid, u16)> {
        index
            .iter()
            .map(|e| (e.path, e.mode, e.id, (e.flags >> 12) & 3))
            .collect()
    }
    fn expected(
        git: &git2::Repository,
        candidate: git2::Oid,
        attrs: git2::Oid,
    ) -> Result<BTreeMap<String, EntryProof>, Fault> {
        // A private handle, empty temporary workdir, and memory-only index bind
        // attribute lookup to the selected tree. No file source is materialized,
        // no checkout runs here, and set_workdir(false) writes no configuration.
        let empty = tempfile::tempdir().map_err(|_| Fault::authority())?;
        let worker = git2::Repository::open(git.path())?;
        worker.set_odb(&git.odb()?)?;
        let mut attrs_index = index(git, attrs)?;
        worker.set_index(&mut attrs_index)?;
        worker.set_workdir(empty.path(), false)?;
        index(git, candidate)?
            .iter()
            .map(|entry| {
                let path = std::str::from_utf8(&entry.path)
                    .map_err(|_| Fault::authority())?
                    .to_owned();
                let bytes =
                    integration::filtered::checkout_bytes(&worker, candidate, entry.id, &path)?;
                Ok((
                    path,
                    EntryProof {
                        oid: entry.id.to_string(),
                        mode: entry.mode,
                        digests: vec![*blake3::hash(&bytes).as_bytes()],
                    },
                ))
            })
            .collect()
    }
    impl Proof {
        pub(in super::super) fn build(
            git: &git2::Repository,
            local: git2::Oid,
            primary: git2::Oid,
            disposition: Disposition,
        ) -> Result<Self, Fault> {
            let worker = git2::Repository::open(git.path())?;
            let odb = worker.odb()?;
            let _memory = odb.add_new_mempack_backend(1000)?;
            let candidate = match disposition {
                Disposition::Noop => local,
                Disposition::FastForward => primary,
                Disposition::Merge => {
                    let merge = integration::prepare::prepare_clean_merge_in(
                        &worker,
                        git.path(),
                        local,
                        primary,
                    )?;
                    let integration::prepare::MergePreparation::Clean(merge) = merge else {
                        return Err(Fault::authority());
                    };
                    let tree = integration::prepare::import_prepared_tree(&worker, &merge)?;
                    let sig = git2::Signature::new(
                        "Manyhands",
                        "preview@manyhands.invalid",
                        &git2::Time::new(0, 0),
                    )?;
                    worker.commit(
                        None,
                        &sig,
                        &sig,
                        "Image preview",
                        &worker.find_tree(tree)?,
                        &[&worker.find_commit(local)?, &worker.find_commit(primary)?],
                    )?
                }
            };
            let mut after = expected(&worker, candidate, candidate)?;
            // Libgit2 may have cached baseline attributes during its safe-checkout
            // preflight. Both enumerated images are from the frozen pair, never
            // from arbitrary live attributes after an interruption.
            for (path, old_attrs) in expected(&worker, candidate, local)? {
                let entry = after.get_mut(&path).ok_or_else(Fault::authority)?;
                for digest in old_attrs.digests {
                    if !entry.digests.contains(&digest) {
                        entry.digests.push(digest);
                    }
                }
            }
            let mut paths: BTreeMap<String, PathProof> = after
                .into_iter()
                .map(|(path, after)| {
                    (
                        path.clone(),
                        PathProof {
                            path,
                            before: None,
                            after: Some(after),
                        },
                    )
                })
                .collect();
            let root = git.workdir().ok_or_else(Fault::foreign)?;
            for entry in index(git, local)?.iter() {
                let path = std::str::from_utf8(&entry.path)
                    .map_err(|_| Fault::authority())?
                    .to_owned();
                let image = super::super::file_image::read(root, Path::new(&path))
                    .map_err(|_| Fault::foreign())?
                    .ok_or_else(Fault::foreign)?;
                if image.mode.is_some_and(|mode| mode != entry.mode) {
                    return Err(Fault::foreign());
                }
                paths
                    .entry(path.clone())
                    .or_insert(PathProof {
                        path,
                        before: None,
                        after: None,
                    })
                    .before = Some(EntryProof {
                    oid: entry.id.to_string(),
                    mode: entry.mode,
                    digests: vec![*blake3::hash(&image.bytes).as_bytes()],
                });
            }
            Ok(Self {
                baseline_v2: None,
                version: 1,
                coarse_version: 2,
                context: local.to_string(),
                primary: primary.to_string(),
                tree: worker.find_commit(candidate)?.tree_id().to_string(),
                paths: paths.into_values().collect(),
            })
        }
        pub(in super::super) fn insert(
            &self,
            db: &Connection,
            id: OperationId,
        ) -> Result<(), ReadError> {
            let bytes = serde_json::to_vec(self)
                .map_err(|_| ReadError::new(ResultCode::RecoveryRequired))?;
            if db.execute("INSERT INTO local_context_sync_images(operation_ulid,generation,owner_epoch,manifest) SELECT operation_ulid,generation,owner_epoch,?2 FROM local_context_sync_records WHERE operation_ulid=?1",rusqlite::params![id.to_string(),bytes])?!=1 {return Err(ReadError::new(ResultCode::RecoveryRequired));}
            Ok(())
        }
        pub(in super::super) fn read(
            db: &Connection,
            id: OperationId,
        ) -> Result<Option<Self>, ReadError> {
            let row:Option<StoredProof>=db.query_row("SELECT i.manifest,p.context_parent,p.primary_parent,i.baseline_v2 FROM local_context_sync_images i JOIN local_context_sync_records p ON p.operation_ulid=i.operation_ulid WHERE i.operation_ulid=?1",[id.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?;
            row.map(|(bytes, context, primary, baseline)| {
                let mut proof = decode(&bytes, &context, &primary)
                    .map_err(|_| ReadError::new(ResultCode::RecoveryRequired))?;
                proof.baseline_v2 = baseline
                    .map(|b| {
                        b.try_into()
                            .map_err(|_| ReadError::new(ResultCode::RecoveryRequired))
                    })
                    .transpose()?;
                Ok(proof)
            })
            .transpose()
        }
        pub(in super::super) fn matches(
            &self,
            git: &git2::Repository,
            oid: git2::Oid,
        ) -> Result<bool, Fault> {
            let baseline = oid.to_string() == self.context;
            if !baseline && git.find_commit(oid)?.tree_id().to_string() != self.tree {
                return Err(Fault::authority());
            }
            let expected = index(git, oid)?;
            let mut actual = git.index()?;
            actual.read(true)?;
            if entries(&expected) != entries(&actual) {
                return Ok(false);
            }
            let bound = self
                .paths
                .iter()
                .filter_map(|p| {
                    if baseline {
                        p.before.as_ref()
                    } else {
                        p.after.as_ref()
                    }
                    .map(|e| {
                        (
                            p.path.as_bytes().to_vec(),
                            e.mode,
                            git2::Oid::from_str(&e.oid).expect("audited OID"),
                            0,
                        )
                    })
                })
                .collect::<Vec<_>>();
            if bound != entries(&expected) {
                return Err(Fault::authority());
            }
            let root = git.workdir().ok_or_else(Fault::foreign)?;
            for path in &self.paths {
                let Some(entry) = (if baseline {
                    path.before.as_ref()
                } else {
                    path.after.as_ref()
                }) else {
                    continue;
                };
                let Some(image) = super::super::file_image::read(root, Path::new(&path.path))
                    .map_err(|_| Fault::foreign())?
                else {
                    return Ok(false);
                };
                if image.mode.is_some_and(|mode| mode != entry.mode) {
                    return Ok(false);
                }
                let digest = *blake3::hash(&image.bytes).as_bytes();
                let retained = !baseline
                    && path.before.as_ref().is_some_and(|old| {
                        old.oid == entry.oid
                            && old.mode == entry.mode
                            && old.digests.contains(&digest)
                    });
                if !entry.digests.contains(&digest) && !retained {
                    return Ok(false);
                }
            }
            let mut options = git2::StatusOptions::new();
            options
                .include_untracked(true)
                .recurse_untracked_dirs(true)
                .update_index(false);
            // Complete explicit tracked-path proof above supersedes tracked stat/
            // filter status, which may reinterpret an unchanged retained image.
            Ok(!git
                .statuses(Some(&mut options))?
                .iter()
                .any(|e| e.status().contains(git2::Status::WT_NEW)))
        }
        pub(in super::super) fn filters_match(
            &self,
            git: &git2::Repository,
            candidate: git2::Oid,
        ) -> Result<bool, Fault> {
            if git.find_commit(candidate)?.tree_id().to_string() != self.tree {
                return Err(Fault::authority());
            }
            let mut current = expected(git, candidate, candidate)?;
            for (path, old) in expected(git, candidate, git2::Oid::from_str(&self.context)?)? {
                let item = current.get_mut(&path).ok_or_else(Fault::authority)?;
                for digest in old.digests {
                    if !item.digests.contains(&digest) {
                        item.digests.push(digest);
                    }
                }
            }
            let bound = self
                .paths
                .iter()
                .filter_map(|p| p.after.as_ref().map(|e| (&p.path, e)))
                .collect::<Vec<_>>();
            if bound.len() != current.len()
                || bound.iter().any(|(path, e)| {
                    current
                        .get(*path)
                        .is_none_or(|actual| actual.oid != e.oid || actual.mode != e.mode)
                })
            {
                return Err(Fault::authority());
            }
            Ok(bound.into_iter().all(|(path, e)| {
                current
                    .get(path)
                    .is_some_and(|actual| actual.digests == e.digests)
            }))
        }
    }
}
