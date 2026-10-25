#![allow(dead_code)] // Composed by the later promotion runner and Cycle 10.
use super::{
    AuthoringKind, OperationId, RemoteRefPlan, RepositoryError, RepositoryLease,
    RepositoryOperation, authoring_kind_segment, item_context,
    remote::{state, sync},
    repository_lease, validate_context_worktree_identity,
};
use crate::canonical::ItemId;
use git2::Oid;
use rusqlite::OptionalExtension;
use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Precheck {
    Ok,
    Changed,
    IgnoredFiles,
    Locked,
}
#[derive(Debug, PartialEq, Eq)]
pub(super) enum WorktreeRemoval {
    Removed,
    AlreadyAbsent,
    Changed,
    RecoveryRequired,
}
#[derive(Debug, PartialEq, Eq)]
pub(super) enum BranchDeletion {
    Deleted,
    AlreadyAbsent,
    Changed,
}
#[derive(Debug, PartialEq, Eq)]
pub(super) enum TrackingDeletion {
    Changed,
    RecoveryRequired,
}
/// Local cleanup capability: keeps the repository lease through every check and
/// effect. The lifecycle caller additionally validates durable branch_deleted
/// intent, remote ownership and frozen configuration before tracking deletion.
pub(super) struct ContextCleanup {
    repository: git2::Repository,
    root: PathBuf,
    _lease: RepositoryLease,
}
/// Not constructible outside this module, not Clone, and cannot outlive the
/// cleanup lease. This proves the *local* tracking effect, not publication or
/// lifecycle completion. A fresh retry obtains fresh evidence after revalidation.
pub(super) struct RetirementEvidence<'a> {
    cleanup: &'a ContextCleanup,
    plan: Option<RemoteRefPlan>,
    kind: AuthoringKind,
    item_id: ItemId,
    operation_id: OperationId,
    retired_oid: Oid,
    _tracking_lock: Option<git2::Transaction<'a>>,
}
impl RetirementEvidence<'_> {
    /// Bind the private Git proof to the parent being completed, without giving
    /// callers a way to manufacture or detach that proof from its lease/lock.
    /// The kind is checked too: a document proof never retires a ticket, nor
    /// the reverse.
    pub(super) fn matches(
        &self,
        kind: AuthoringKind,
        operation: OperationId,
        item: &ItemId,
        oid: Oid,
    ) -> bool {
        self.kind == kind
            && self.operation_id == operation
            && &self.item_id == item
            && self.retired_oid == oid
    }
}
impl ContextCleanup {
    pub(super) fn acquire(root: &Path) -> Result<Self, RepositoryError> {
        let (repository, root) =
            super::canonical_repository_root(root, RepositoryOperation::RepositorySnapshot)?;
        if repository.is_worktree() {
            return Err(state::recovery_required());
        }
        let lease = repository_lease(&repository, &root, RepositoryOperation::RepositorySnapshot)?;
        Ok(Self {
            repository,
            root,
            _lease: lease,
        })
    }
    pub(super) fn precheck_context(&self, kind: AuthoringKind, id: &ItemId, oid: Oid) -> Precheck {
        let target = super::SynchronizationTarget::Context {
            kind,
            item_id: id.clone(),
        };
        let context = item_context(&self.root, kind, id.clone());
        if oid.is_zero()
            || validate_context_worktree_identity(
                &self.repository,
                &context,
                RepositoryOperation::RepositorySnapshot,
            )
            .is_err()
        {
            return Precheck::Changed;
        }
        let Ok(linked) = sync::materialized_target(&self.root, "", &target) else {
            return Precheck::Changed;
        };
        let expected_admin = self
            .repository
            .commondir()
            .join("worktrees")
            .join(id.to_string());
        if !linked.is_worktree()
            || linked.path().canonicalize().ok() != expected_admin.canonicalize().ok()
            || linked.commondir().canonicalize().ok()
                != self.repository.commondir().canonicalize().ok()
        {
            return Precheck::Changed;
        }
        if linked.head().ok().and_then(|h| h.target()) != Some(oid)
            || self
                .repository
                .find_reference(&format!("refs/heads/{}", context.branch))
                .ok()
                .and_then(|r| r.target())
                != Some(oid)
        {
            return Precheck::Changed;
        }
        // The recorded clean OID is canonical authority. Do not invoke the
        // filesystem canonical-context scanner while this capability owns the
        // common lease; materialization/status/ignored checks remain below.
        if super::validate_context_tree(&linked, &context, RepositoryOperation::RepositorySnapshot)
            .is_err()
        {
            return Precheck::Changed;
        }
        // This stronger proof is deletion-only. Do not change synchronization's
        // status policy or clear index flags to make hidden data appear clean.
        let [Ok(worktree_root), Ok(admin_root)] =
            [context.worktree, expected_admin].map(|path| path.canonicalize())
        else {
            return Precheck::Changed;
        };
        let removal_roots = [worktree_root, admin_root];
        if let Err(reason) =
            require_cleanup_clean(&linked, &target, &removal_roots, &mut BTreeSet::new(), 0)
        {
            return reason;
        }
        match self
            .repository
            .find_worktree(&id.to_string())
            .and_then(|w| w.is_locked())
        {
            Ok(git2::WorktreeLockStatus::Unlocked) => Precheck::Ok,
            Ok(git2::WorktreeLockStatus::Locked(_)) => Precheck::Locked,
            Err(_) => Precheck::Changed,
        }
    }
    pub(super) fn remove_context_worktree(
        &self,
        kind: AuthoringKind,
        id: &ItemId,
        oid: Oid,
    ) -> WorktreeRemoval {
        let context = item_context(&self.root, kind, id.clone());
        let admin = self
            .repository
            .commondir()
            .join("worktrees")
            .join(id.to_string());
        match (present(&context.worktree), present(&admin)) {
            (Ok(false), Ok(false)) => return WorktreeRemoval::AlreadyAbsent,
            (Ok(true), Ok(true)) => (),
            _ => return WorktreeRemoval::RecoveryRequired,
        }
        if self.precheck_context(kind, id, oid) != Precheck::Ok {
            // Partial prune must never be guessed finished. Missing .git or any
            // tracked path requires explicit repair, even if libgit2 can prune it.
            let partial = git2::Repository::open(&context.worktree)
                .ok()
                .and_then(|r| r.index().ok())
                .map(|index| {
                    index.iter().any(|entry| {
                        std::str::from_utf8(&entry.path)
                            .ok()
                            .is_none_or(|path| present(&context.worktree.join(path)) != Ok(true))
                    })
                })
                .unwrap_or(true);
            return if partial {
                WorktreeRemoval::RecoveryRequired
            } else {
                WorktreeRemoval::Changed
            };
        }
        let Ok(worktree) = self.repository.find_worktree(&id.to_string()) else {
            return WorktreeRemoval::RecoveryRequired;
        };
        let mut options = git2::WorktreePruneOptions::new();
        options.valid(true).working_tree(true).locked(false);
        if worktree.prune(Some(&mut options)).is_err() {
            return WorktreeRemoval::RecoveryRequired;
        }
        if present(&context.worktree) == Ok(false) && present(&admin) == Ok(false) {
            WorktreeRemoval::Removed
        } else {
            WorktreeRemoval::RecoveryRequired
        }
    }
    pub(super) fn delete_context_branch(
        &self,
        kind: AuthoringKind,
        id: &ItemId,
        oid: Oid,
    ) -> BranchDeletion {
        let reference = format!(
            "refs/heads/manyhands/{}/{id}",
            authoring_kind_segment(&kind)
        );
        let result = (|| {
            if oid.is_zero() {
                return Err(());
            }
            // A verified absent branch needs no lock/effect. Locking a missing
            // slot can recreate empty ref directories even on an idempotent call.
            if direct_oid(&self.repository, &reference)
                .map_err(|_| ())?
                .is_none()
            {
                return Ok(BranchDeletion::AlreadyAbsent);
            }
            let mut transaction = self.repository.transaction().map_err(|_| ())?;
            transaction.lock_ref(&reference).map_err(|_| ())?;
            let actual = direct_oid(&self.repository, &reference).map_err(|_| ())?;
            if actual.is_none() {
                return Ok(BranchDeletion::AlreadyAbsent);
            }
            if actual != Some(oid) || self.branch_is_checked_out(&reference)? {
                return Err(());
            }
            transaction.remove(&reference).map_err(|_| ())?;
            transaction.commit().map_err(|_| ())?;
            Ok(BranchDeletion::Deleted)
        })();
        result.unwrap_or(BranchDeletion::Changed)
    }
    /// Revalidate a previously completed local cleanup without repeating its
    /// destructive helpers. The capability keeps the repository lease through
    /// this proof, the following tracking effect and final SQL transaction.
    pub(super) fn require_local_cleanup_absent(
        &self,
        kind: AuthoringKind,
        id: &ItemId,
    ) -> Result<(), TrackingDeletion> {
        let context = item_context(&self.root, kind, id.clone());
        let branch = format!("refs/heads/{}", context.branch);
        if direct_oid(&self.repository, &branch)?.is_some() {
            return Err(TrackingDeletion::Changed);
        }
        let admin = self
            .repository
            .commondir()
            .join("worktrees")
            .join(id.to_string());
        match (present(&context.worktree), present(&admin)) {
            (Ok(false), Ok(false)) => (),
            (Ok(true), Ok(true)) => return Err(TrackingDeletion::Changed),
            _ => return Err(TrackingDeletion::RecoveryRequired),
        }
        match self.branch_is_checked_out(&branch) {
            Ok(false) => Ok(()),
            Ok(true) => Err(TrackingDeletion::Changed),
            Err(()) => Err(TrackingDeletion::RecoveryRequired),
        }
    }
    fn branch_is_checked_out(&self, reference: &str) -> Result<bool, ()> {
        let at_branch = |gitdir: &Path| -> Result<bool, ()> {
            let head = std::fs::read_to_string(gitdir.join("HEAD")).map_err(|_| ())?;
            let head = head.trim_end_matches('\n');
            if let Some(branch) = head.strip_prefix("ref: ") {
                if !git2::Reference::is_valid_name(branch) {
                    return Err(());
                }
                if branch == reference {
                    return Ok(true);
                }
                // An indirect alias is not evidence of an unrelated checkout.
                direct_oid(&self.repository, branch)
                    .map_err(|_| ())?
                    .ok_or(())?;
                Ok(false)
            } else {
                let detached = Oid::from_str(head).map_err(|_| ())?;
                if detached.is_zero() {
                    return Err(());
                }
                Ok(false)
            }
        };
        if at_branch(self.repository.path())? {
            return Ok(true);
        }
        // libgit2's worktrees() silently filters incomplete registrations. Only
        // a complete filesystem inventory can prove no checkout exists; missing
        // or invalid evidence is a refusal, never permission to delete a ref.
        let inventory = self.repository.commondir().join("worktrees");
        match std::fs::symlink_metadata(&inventory) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(_) => return Err(()),
            Ok(_) => plain_metadata(&inventory, true)?,
        }
        let common = self.repository.commondir().canonicalize().map_err(|_| ())?;
        for entry in std::fs::read_dir(&inventory).map_err(|_| ())? {
            let entry = entry.map_err(|_| ())?;
            let admin = entry.path();
            plain_metadata(&admin, true)?;
            for member in ["commondir", "gitdir", "HEAD"] {
                plain_metadata(&admin.join(member), false)?;
            }
            let read_link = |member: &str| -> Result<PathBuf, ()> {
                let text = std::fs::read_to_string(admin.join(member)).map_err(|_| ())?;
                let text = text.trim_end_matches('\n');
                if text.is_empty() {
                    return Err(());
                }
                // Absolute links remain absolute; relative metadata links are
                // interpreted at their registration, as Git's commondir is.
                admin.join(text).canonicalize().map_err(|_| ())
            };
            if read_link("commondir")? != common {
                return Err(());
            }
            let name = entry.file_name();
            let worktree = self
                .repository
                .find_worktree(name.to_str().ok_or(())?)
                .map_err(|_| ())?;
            if read_link("gitdir")?
                != worktree
                    .path()
                    .join(".git")
                    .canonicalize()
                    .map_err(|_| ())?
            {
                return Err(());
            }
            // Inspect this actual registration's HEAD, never a redirected .git
            // link in its working directory.
            if at_branch(&admin)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
    /// Git-first completion seam. Caller must first persist branch_deleted and
    /// validate the real reservation/configuration. This helper never invents
    /// that authority or resets history. SQL finalization is separate below.
    pub(super) fn delete_tracking_ref(
        &self,
        plan: Option<&RemoteRefPlan>,
        kind: AuthoringKind,
        id: &ItemId,
        expected: Option<Oid>,
        operation: OperationId,
        oid: Oid,
    ) -> Result<RetirementEvidence<'_>, TrackingDeletion> {
        if oid.is_zero()
            || expected.is_some_and(|o| o.is_zero())
            || (plan.is_none() && expected.is_some())
        {
            return Err(TrackingDeletion::Changed);
        }
        let tracking_lock = if let Some(plan) = plan {
            let target = plan.context(kind, id);
            let mut transaction = self
                .repository
                .transaction()
                .map_err(|_| TrackingDeletion::RecoveryRequired)?;
            transaction
                .lock_ref(target.tracking_ref())
                .map_err(|_| TrackingDeletion::RecoveryRequired)?;
            match direct_oid(&self.repository, target.tracking_ref())? {
                None => (),
                Some(actual) if Some(actual) == expected => {
                    transaction
                        .remove(target.tracking_ref())
                        .map_err(|_| TrackingDeletion::RecoveryRequired)?;
                    transaction
                        .commit()
                        .map_err(|_| TrackingDeletion::RecoveryRequired)?;
                    // Git deletion is already durable. A failure to reacquire
                    // the now-absent slot is recovery, never SQL retirement.
                    transaction = self
                        .repository
                        .transaction()
                        .map_err(|_| TrackingDeletion::RecoveryRequired)?;
                    transaction
                        .lock_ref(target.tracking_ref())
                        .map_err(|_| TrackingDeletion::RecoveryRequired)?;
                }
                _ => return Err(TrackingDeletion::Changed),
            }
            if direct_oid(&self.repository, target.tracking_ref())?.is_some() {
                return Err(TrackingDeletion::Changed);
            }
            Some(transaction)
        } else {
            None
        };
        Ok(RetirementEvidence {
            cleanup: self,
            plan: plan.cloned(),
            kind,
            item_id: id.clone(),
            operation_id: operation,
            retired_oid: oid,
            _tracking_lock: tracking_lock,
        })
    }
}

/// Called only while ContextCleanup owns the superproject lease. Recursion never
/// acquires another lease/reservation, opens a transport, changes configuration
/// or writes an index. Each populated gitlink gets its own full proof regardless
/// of its parent's submodule ignore configuration.
fn require_cleanup_clean(
    repository: &git2::Repository,
    target: &super::SynchronizationTarget,
    removal_roots: &[PathBuf; 2],
    visited: &mut BTreeSet<PathBuf>,
    depth: usize,
) -> Result<(), Precheck> {
    const MAX_REPOSITORIES: usize = 64;
    // Git's on-disk CE_VALID (assume-unchanged) and extended SKIP_WORKTREE bits.
    const ASSUME_UNCHANGED: u16 = 0x8000;
    const SKIP_WORKTREE: u16 = 0x4000;
    if depth >= MAX_REPOSITORIES || visited.len() >= MAX_REPOSITORIES {
        return Err(Precheck::Changed);
    }
    let gitdir = repository
        .path()
        .canonicalize()
        .map_err(|_| Precheck::Changed)?;
    if !visited.insert(gitdir.clone()) {
        return Err(Precheck::Changed);
    }
    let index = repository.index().map_err(|_| Precheck::Changed)?;
    if index.iter().any(|entry| {
        entry.flags & ASSUME_UNCHANGED != 0 || entry.flags_extended & SKIP_WORKTREE != 0
    }) {
        return Err(Precheck::Changed);
    }
    let gitlinks = index
        .iter()
        .filter(|entry| entry.mode == 0o160000)
        .collect::<Vec<_>>();
    drop(index);
    sync::require_clean_target(repository, target).map_err(|_| Precheck::Changed)?;
    let mut options = git2::StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(true)
        .recurse_ignored_dirs(true);
    let statuses = repository
        .statuses(Some(&mut options))
        .map_err(|_| Precheck::Changed)?;
    if !statuses.is_empty() {
        return Err(
            if statuses
                .iter()
                .any(|entry| entry.status().contains(git2::Status::IGNORED))
            {
                Precheck::IgnoredFiles
            } else {
                Precheck::Changed
            },
        );
    }
    drop(statuses);
    let workdir = repository.workdir().ok_or(Precheck::Changed)?;
    for entry in gitlinks {
        let path = std::str::from_utf8(&entry.path).map_err(|_| Precheck::Changed)?;
        let relative = Path::new(path);
        if relative.as_os_str().is_empty()
            || !relative
                .components()
                .all(|part| matches!(part, Component::Normal(_)))
        {
            return Err(Precheck::Changed);
        }
        let mut child_path = workdir.to_owned();
        for component in relative.components() {
            child_path.push(component);
            plain_metadata(&child_path, true).map_err(|_| Precheck::Changed)?;
        }
        let git_link =
            std::fs::symlink_metadata(child_path.join(".git")).map_err(|_| Precheck::Changed)?;
        if git_link.is_dir() {
            plain_metadata(&child_path.join(".git"), true).map_err(|_| Precheck::Changed)?;
        } else {
            plain_metadata(&child_path.join(".git"), false).map_err(|_| Precheck::Changed)?;
        }
        let child = repository
            .find_submodule(path)
            .and_then(|module| module.open())
            .map_err(|_| Precheck::Changed)?;
        if child.workdir().and_then(|p| p.canonicalize().ok())
            != Some(child_path.canonicalize().map_err(|_| Precheck::Changed)?)
            || child.head().ok().and_then(|head| head.target()) != Some(entry.id)
            || child.is_worktree()
        {
            // A foreign linked-worktree registration is not ours to remove.
            return Err(Precheck::Changed);
        }
        require_cleanup_clean(&child, target, removal_roots, visited, depth + 1)?;
        let common = child
            .commondir()
            .canonicalize()
            .map_err(|_| Precheck::Changed)?;
        let child_gitdir = child.path().canonicalize().map_err(|_| Precheck::Changed)?;
        if removal_roots
            .iter()
            .any(|root| child_gitdir.starts_with(root) || common.starts_with(root))
        {
            // Even a clean worktree does not prove all locally stored submodule
            // history is disposable. Its Git metadata must survive this prune.
            return Err(Precheck::Changed);
        }
    }
    Ok(())
}

fn plain_metadata(path: &Path, directory: bool) -> Result<(), ()> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| ())?;
    if (directory && !metadata.is_dir())
        || (!directory && !metadata.is_file())
        || metadata.file_type().is_symlink()
    {
        return Err(());
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(());
        }
    }
    Ok(())
}
fn present(path: &Path) -> Result<bool, ()> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(()),
    }
}
fn direct_oid(
    repository: &git2::Repository,
    reference: &str,
) -> Result<Option<Oid>, TrackingDeletion> {
    match repository.find_reference(reference) {
        Ok(r) if r.symbolic_target().is_none() => r
            .target()
            .map(Some)
            .ok_or(TrackingDeletion::RecoveryRequired),
        Ok(_) => Err(TrackingDeletion::Changed),
        Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(_) => Err(TrackingDeletion::RecoveryRequired),
    }
}

/// SQL-only retirement pieces. The lifecycle composes completion and applicable
/// reservation finish into this same caller-owned transaction, retaining the
/// evidence (and its absent-slot ref lock) until commit/reconciliation. A commit error
/// is not proof of rollback: reopen/reconcile the real record before replay.
pub(super) fn retire_context(
    tx: &rusqlite::Transaction<'_>,
    repository_id: i64,
    evidence: &RetirementEvidence<'_>,
) -> Result<(), RepositoryError> {
    let registered: String = tx
        .query_row(
            "SELECT root_path FROM repositories WHERE id=?1",
            [repository_id],
            |r| r.get(0),
        )
        .map_err(|_| state::recovery_required())?;
    if registered
        != super::registry_root_key(
            &evidence.cleanup.root,
            RepositoryOperation::RepositorySnapshot,
        )?
    {
        return Err(state::recovery_required());
    }
    // Revalidate frozen remote configuration and the actual ref immediately
    // before any SQL write. Evidence alone cannot hide a newly recreated ref.
    let (remote, primary): (Option<String>, Option<String>) = tx
        .query_row(
            "SELECT remote_name,primary_branch FROM remote_polling_state WHERE repository_id=?1",
            [repository_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| state::recovery_required())?;
    if remote.as_deref() != evidence.plan.as_ref().map(RemoteRefPlan::remote_name)
        || primary.as_deref() != evidence.plan.as_ref().map(RemoteRefPlan::primary_branch)
    {
        return Err(state::recovery_required());
    }
    if let Some(plan) = &evidence.plan {
        let target = plan.context(evidence.kind, &evidence.item_id);
        if direct_oid(&evidence.cleanup.repository, target.tracking_ref())
            .map_err(|_| state::recovery_required())?
            .is_some()
        {
            return Err(state::recovery_required());
        }
        // A state row and its current observation must retire together. Never
        // edit a frozen pass in-place: completed synchronization windows may pin
        // that exact batch. Retain it as historical, with an otherwise identical
        // current projection excluding only this item, in the same SQL TX.
        let previous:Option<i64>=tx.query_row("SELECT id FROM remote_observation_batches WHERE repository_id=?1 AND is_current=1 AND EXISTS(SELECT 1 FROM remote_ref_observations WHERE batch_id=remote_observation_batches.id AND remote_ref=?2)",rusqlite::params![repository_id,target.remote_ref()],|r|r.get(0)).optional().map_err(|_|state::recovery_required())?;
        if let Some(previous) = previous {
            tx.execute(
                "UPDATE remote_observation_batches SET is_current=0 WHERE id=?1",
                [previous],
            )
            .map_err(|_| state::recovery_required())?;
            tx.execute("INSERT INTO remote_observation_batches(repository_id,remote_name,primary_branch,configuration_generation,observed_at,is_current) SELECT repository_id,remote_name,primary_branch,configuration_generation,observed_at,1 FROM remote_observation_batches WHERE id=?1",[previous]).map_err(|_|state::recovery_required())?;
            let current = tx.last_insert_rowid();
            tx.execute("INSERT INTO remote_ref_observations(batch_id,ordinal,remote_ref,tracking_ref,classification,advertised_oid,tracking_oid) SELECT ?1,ordinal,remote_ref,tracking_ref,classification,advertised_oid,tracking_oid FROM remote_ref_observations WHERE batch_id=?2 AND (remote_ref IS NULL OR remote_ref!=?3)",rusqlite::params![current,previous,target.remote_ref()]).map_err(|_|state::recovery_required())?;
        }
    }
    let kind = authoring_kind_segment(&evidence.kind);
    tx.execute(
        "DELETE FROM remote_context_states WHERE repository_id=?1 AND kind=?2 AND item_id=?3",
        rusqlite::params![repository_id, kind, evidence.item_id.to_string()],
    )
    .map_err(|_| state::recovery_required())?;
    tx.execute("INSERT INTO retired_contexts(repository_id,kind,item_id,operation_id,retired_oid) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(repository_id,kind,item_id) DO UPDATE SET operation_id=excluded.operation_id,retired_oid=excluded.retired_oid",rusqlite::params![repository_id,kind,evidence.item_id.to_string(),evidence.operation_id.to_string(),evidence.retired_oid.to_string()]).map_err(|_|state::recovery_required())?;
    Ok(())
}

#[cfg(test)]
#[path = "cleanup_tests.rs"]
mod tests;
