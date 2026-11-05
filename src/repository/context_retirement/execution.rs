//! Metadata-only initial-absence execution. No destructive producer is exposed
//! here: present/branch-only execution is connected separately after its full
//! observation, instance and effect authority is implemented.
use std::{path::Path, path::PathBuf};

use rusqlite::{OptionalExtension, TransactionBehavior, params};

use super::super::{
    AuthoringKind, OperationId, RefreshOutcome, RefreshRepositoryRequest, RepositoryError,
    RepositoryErrorKind, RepositoryOperation, RepositoryService, authoring_kind_segment,
    cache_write_guard, canonical_repository_root,
    discovery::{migrate_registry, open_registry},
    item_context, local_sync, recovery,
    remote::state::recovery_required,
    repository_lease,
};
use super::{ContextRetirementReport, record};
use crate::{canonical::ItemId, results::ResultCode};

#[derive(Clone, Debug)]
pub struct RetireContextRequest {
    pub root: PathBuf,
    pub kind: AuthoringKind,
    pub item_id: ItemId,
    pub operation_id: OperationId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContextRetirementOutcome {
    Final(ContextRetirementReport),
    IndexPending,
}

const OP: RepositoryOperation = RepositoryOperation::RetireContext;

/// Inspect every deterministic ancestor without following redirected paths.
/// Even a dangling symlink is presence, never evidence of absence.
fn absent_path(root: &Path, path: &Path) -> Result<bool, RepositoryError> {
    let relative = path.strip_prefix(root).map_err(|_| recovery_required())?;
    let mut at = root.to_owned();
    for component in relative.components() {
        at.push(component);
        match std::fs::symlink_metadata(&at) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || (at != path && !metadata.is_dir()) {
                    return Err(recovery_required());
                }
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    // A junction or other reparse point need not report as a
                    // symlink. Match cleanup's existing no-follow policy.
                    if metadata.file_attributes() & 0x400 != 0 {
                        return Err(recovery_required());
                    }
                }
                if at == path {
                    return Ok(false);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(true),
            Err(_) => return Err(recovery_required()),
        }
    }
    Ok(false)
}

fn ref_file_absent(root: &Path, path: &Path) -> Result<bool, RepositoryError> {
    let absent = absent_path(root, path)?;
    if !absent && !std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_file()) {
        return Err(recovery_required());
    }
    Ok(absent)
}

fn metadata_line(text: &str) -> Result<&str, ()> {
    let line = text
        .strip_suffix("\r\n")
        .or_else(|| text.strip_suffix('\n'))
        .unwrap_or(text);
    if line.is_empty() || line.contains(['\r', '\n', '\0']) {
        return Err(());
    }
    Ok(line)
}

fn plain_registration_path(path: &Path, directory: bool) -> Result<(), ()> {
    let anchor = path
        .ancestors()
        .last()
        .filter(|p| p.is_absolute())
        .ok_or(())?;
    if absent_path(anchor, path).map_err(|_| ())? {
        return Err(());
    }
    let metadata = std::fs::symlink_metadata(path).map_err(|_| ())?;
    if (directory && !metadata.is_dir()) || (!directory && !metadata.is_file()) {
        return Err(());
    }
    Ok(())
}

/// Retirement-only identity-chain proof before cleanup's HEAD inventory reads
/// any registration links. Canonical equality alone follows redirected targets;
/// require every traversed component plain before resolving either direction.
fn require_registration_identity(git: &git2::Repository) -> Result<(), ()> {
    plain_registration_path(git.commondir(), true)?;
    let common = git.commondir().canonicalize().map_err(|_| ())?;
    let inventory = common.join("worktrees");
    if absent_path(&common, &inventory).map_err(|_| ())? {
        return Ok(());
    }
    plain_registration_path(&inventory, true)?;
    for entry in std::fs::read_dir(&inventory).map_err(|_| ())? {
        let admin = entry.map_err(|_| ())?.path();
        plain_registration_path(&admin, true)?;
        for member in ["commondir", "gitdir", "HEAD"] {
            plain_registration_path(&admin.join(member), false)?;
        }
        let link = std::fs::read_to_string(admin.join("commondir")).map_err(|_| ())?;
        let linked_common = admin.join(metadata_line(&link)?);
        plain_registration_path(&linked_common, true)?;
        if linked_common.canonicalize().map_err(|_| ())? != common {
            return Err(());
        }
        let link = std::fs::read_to_string(admin.join("gitdir")).map_err(|_| ())?;
        // Absolute links remain absolute; Git resolves this admin field at the
        // admin, but relative gitfile targets at the worktree, not the admin.
        let gitfile = admin.join(metadata_line(&link)?);
        plain_registration_path(&gitfile, false)?;
        let contents = std::fs::read_to_string(&gitfile).map_err(|_| ())?;
        let target = metadata_line(&contents)?
            .strip_prefix("gitdir: ")
            .ok_or(())?;
        if target.is_empty() {
            return Err(());
        }
        let linked_admin = gitfile.parent().ok_or(())?.join(target);
        plain_registration_path(&linked_admin, true)?;
        if linked_admin.canonicalize().map_err(|_| ())? != admin.canonicalize().map_err(|_| ())? {
            return Err(());
        }
        // Both directions bind this exact admin, whose commondir target was
        // checked above. No foreign repository is opened or lease acquired.
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Inventory {
    Absent,
    Invalid,
    PresentOrBranchOnly,
}

fn initial_inventory(
    git: &git2::Repository,
    root: &Path,
    request: &RetireContextRequest,
) -> Inventory {
    let context = item_context(root, request.kind, request.item_id.clone());
    // Check the file stores BEFORE asking libgit2 to read/follow a ref slot.
    let loose_absent = match ref_file_absent(
        root,
        &git.commondir()
            .join(format!("refs/heads/{}", context.branch)),
    ) {
        Ok(absent) => absent,
        Err(_) => return Inventory::Invalid,
    };
    if ref_file_absent(root, &git.commondir().join("packed-refs")).is_err() {
        return Inventory::Invalid;
    }
    let branch_absent = match git.find_reference(&format!("refs/heads/{}", context.branch)) {
        Ok(reference)
            if reference.symbolic_target().is_none()
                && reference.target().is_some_and(|oid| !oid.is_zero()) =>
        {
            false
        }
        Ok(_) => return Inventory::Invalid,
        Err(error) if error.code() == git2::ErrorCode::NotFound => true,
        Err(_) => return Inventory::Invalid,
    };
    // Git's NotFound alone may have followed a dangling loose-ref symlink.
    // Inspect both authoritative file stores without following redirected slots
    // or ancestors, and require libgit2 to read the complete ref inventory.
    if branch_absent
        && (!loose_absent
            || !git
                .references()
                .is_ok_and(|mut refs| refs.all(|reference| reference.is_ok())))
    {
        return Inventory::Invalid;
    }
    let parts = (
        absent_path(root, &context.worktree),
        absent_path(
            root,
            &git.commondir()
                .join("worktrees")
                .join(request.item_id.to_string()),
        ),
        absent_path(
            root,
            &git.commondir()
                .join(format!("refs/heads/{}.lock", context.branch)),
        ),
    );
    match parts {
        (Ok(true), Ok(true), Ok(true)) if branch_absent => {
            // Deterministic path absence alone misses renamed/external
            // registrations and incomplete entries filtered out by libgit2.
            if matches!(absent_path(root, &git.path().join("HEAD")), Ok(false))
                && require_registration_identity(git).is_ok()
                && matches!(
                    super::super::cleanup::branch_is_checked_out(
                        git,
                        &format!("refs/heads/{}", context.branch)
                    ),
                    Ok(false)
                )
            {
                Inventory::Absent
            } else {
                Inventory::Invalid
            }
        }
        (Ok(w), Ok(a), Ok(true)) if !branch_absent && w == a => Inventory::PresentOrBranchOnly,
        _ => Inventory::Invalid,
    }
}

impl RepositoryService {
    /// Retire an initially absent context's stale discovery bookkeeping. The
    /// present-context cleanup executor is not yet exposed by this increment.
    pub fn retire_context(
        &self,
        request: RetireContextRequest,
    ) -> Result<ContextRetirementOutcome, RepositoryError> {
        self.require_index_available(OP, Some(&request.root))?;
        let (git, root) = canonical_repository_root(&request.root, OP)?;
        if git.is_worktree() {
            return Err(recovery_required());
        }
        let lease = repository_lease(&git, &root, OP)?;
        let target = format!(
            "authoring-context-v1/{}/{}/Retire",
            authoring_kind_segment(&request.kind),
            request.item_id
        );
        {
            let _guard = cache_write_guard(&self.registry_path, &root, OP)?;
            let mut db = open_registry(&self.registry_path, &mut |_| {})?;
            migrate_registry(&mut db)?;
            let tx = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(RepositoryError::sqlite)?;
            record::audit_connection(&tx)?;
            let repository_id: i64 = tx
                .query_row(
                    "SELECT id FROM repositories WHERE root_path=?1",
                    [root.to_str()],
                    |r| r.get(0),
                )
                .optional()
                .map_err(RepositoryError::sqlite)?
                .ok_or_else(|| {
                    RepositoryError::new(
                        OP,
                        Some(root.clone()),
                        RepositoryErrorKind::RepositoryNotRegistered,
                        "retirement requires a registered repository",
                    )
                })?;
            let id = request.operation_id.to_string();
            let existing: Option<(String,String,String,Option<String>)> = tx.query_row("SELECT o.root_path,o.target,c.phase,c.final_result FROM operation_records o JOIN context_retirement_records c ON c.local_record_id=o.id WHERE o.operation_ulid=?1", [&id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(RepositoryError::sqlite)?;
            if let Some((bound_root, bound_target, phase, result)) = existing {
                if Some(bound_root.as_str()) != root.to_str() || bound_target != target {
                    return Err(RepositoryError::new(
                        OP,
                        Some(root.clone()),
                        RepositoryErrorKind::OperationMismatch,
                        "retirement inputs do not match the original operation",
                    ));
                }
                let domain_only: bool = tx.query_row("SELECT request_ulid IS NULL AND batch_operation_ulid IS NULL FROM context_retirement_records WHERE operation_ulid=?1", [&id], |r| r.get(0)).map_err(RepositoryError::sqlite)?;
                if !domain_only {
                    return Err(recovery_required());
                }
                if let Some(result) = result {
                    return Ok(ContextRetirementOutcome::Final(
                        ContextRetirementReport::decode(&result)?,
                    ));
                }
                if let Some(result) = record::finish_configuration_refusal(&tx, &id, &root)? {
                    tx.commit().map_err(RepositoryError::sqlite)?;
                    return Ok(ContextRetirementOutcome::Final(result));
                }
                if !matches!(phase.as_str(), "local_absent" | "index_pending")
                    || !record::refresh_allowed(&tx, &id, &root)?
                {
                    return Err(recovery_required());
                }
                // An accepted authoritative handoff NEVER reinspects B/W/A or
                // consumes another materialization, including after recreation.
            } else {
                if local_sync::record::foreign_id_collision(&tx, &id, None)?
                    || local_sync::record::owned_identity(&tx, &id)?
                    || record::owned_identity(&tx, &id)?
                {
                    return Err(recovery_required());
                }
                recovery::require_no_pending_local(&tx, repository_id)?;
                // A terminal remote row may still own discovery/reconciliation
                // before a local handoff row exists. Its released reservation
                // is not permission for a new retirement to settle that debt.
                let blocked: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE repository_id=?1 AND (phase IN ('reserved','advertising','persisting','fetch_prepared','fetch_observed','local_prepared','local_fast_forwarded','push_prepared','push_returned','push_verified','reconciling') OR index_pending!=0 OR reconciliation_required!=0)) OR EXISTS(SELECT 1 FROM request_operations WHERE operation_ulid=?2)", params![repository_id,id], |r| r.get(0)).map_err(RepositoryError::sqlite)?;
                if blocked {
                    return Err(recovery_required());
                }
                // This increment grants only M (metadata-only absence), not O/C/F.
                let inventory = initial_inventory(&git, &root, &request);
                if inventory == Inventory::PresentOrBranchOnly {
                    return Err(recovery_required());
                }
                let materialization: Option<(String,String,String)> = tx.query_row("SELECT operation_ulid,target_oid,step FROM remote_poll_materializations WHERE repository_id=?1 AND kind=?2 AND item_id=?3", params![repository_id,authoring_kind_segment(&request.kind),request.item_id.to_string()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(RepositoryError::sqlite)?;
                let count: i64 = tx.query_row("SELECT count(*) FROM remote_poll_materializations WHERE repository_id=?1 AND item_id=?2", params![repository_id,request.item_id.to_string()], |r| r.get(0)).map_err(RepositoryError::sqlite)?;
                let valid_owner = if let Some((owner, oid, _)) = &materialization {
                    OperationId::parse(owner).is_ok_and(|id| id.to_string()==*owner)
                        && git2::Oid::from_str(oid).is_ok_and(|id| !id.is_zero() && id.to_string()==*oid)
                        && tx.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE repository_id=?1 AND operation_ulid=?2 AND action='poll' AND poll_kind='update')", params![repository_id,owner], |r| r.get::<_, bool>(0)).map_err(RepositoryError::sqlite)?
                } else {
                    true
                };
                let complete = inventory == Inventory::Absent
                    && valid_owner
                    && count == i64::from(materialization.is_some())
                    && materialization
                        .as_ref()
                        .is_none_or(|(_, _, step)| step == "complete");
                tx.execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,target,state,observed_at) VALUES(?1,?2,?3,'retire_context',?4,'created',?5)", params![repository_id,root.to_str(),id,target,self.clock.now().unix_timestamp()]).map_err(RepositoryError::sqlite)?;
                let local_id = tx.last_insert_rowid();
                let captured = materialization.as_ref().filter(|_| complete);
                tx.execute("INSERT INTO context_retirement_records(operation_ulid,local_record_id,repository_id,format_version,kind,item_id,initial_resources,selection_epoch,instance_verified,materialization_capture,materialization_operation_ulid,materialization_oid,materialization_step,materialization_settled,owner_epoch,phase,worktree_removal,branch_removal,absence_basis,final_code,final_result)
                  VALUES(?1,?2,?3,1,?4,?5,?13,1,0,?6,?7,?8,?9,0,1,?10,'none','none','none',?11,?12)",
                  params![id,local_id,repository_id,authoring_kind_segment(&request.kind),request.item_id.to_string(),if !complete { "unverified" } else if captured.is_some() { "complete" } else { "absent" },captured.map(|v| &v.0),captured.map(|v| &v.1),captured.map(|v| &v.2),if complete { "selected" } else { "refused" },(!complete).then_some(ResultCode::RecoveryRequired.as_str()),if complete { None } else { Some(ContextRetirementReport::refused(ResultCode::RecoveryRequired).encode()?) },if inventory==Inventory::Absent { "absent" } else { "invalid" }]).map_err(RepositoryError::sqlite)?;
                tx.execute("INSERT INTO context_retirement_attempts(operation_ulid,owner_epoch,repository_id,phase) VALUES(?1,1,?2,?3)", params![id,repository_id,if complete { "active" } else { "ended" }]).map_err(RepositoryError::sqlite)?;
                if !complete {
                    tx.execute(
                        "UPDATE operation_records SET state='completed' WHERE id=?1",
                        [local_id],
                    )
                    .map_err(RepositoryError::sqlite)?;
                    tx.commit().map_err(RepositoryError::sqlite)?;
                    return Ok(ContextRetirementOutcome::Final(
                        ContextRetirementReport::refused(ResultCode::RecoveryRequired),
                    ));
                }
                if initial_inventory(&git, &root, &request) != Inventory::Absent {
                    return Err(recovery_required());
                }
                if let Some((owner, oid, step)) = materialization
                    && tx.execute("DELETE FROM remote_poll_materializations WHERE repository_id=?1 AND kind=?2 AND item_id=?3 AND operation_ulid=?4 AND target_oid=?5 AND step=?6 AND step='complete'", params![repository_id,authoring_kind_segment(&request.kind),request.item_id.to_string(),owner,oid,step]).map_err(RepositoryError::sqlite)? != 1 {
                    return Err(recovery_required());
                }
                let replaced: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM remote_poll_materializations WHERE repository_id=?1 AND item_id=?2)", params![repository_id,request.item_id.to_string()], |r| r.get(0)).map_err(RepositoryError::sqlite)?;
                if replaced {
                    return Err(recovery_required());
                }
                tx.execute("UPDATE context_retirement_records SET phase='index_pending',absence_basis='initial',materialization_settled=1 WHERE operation_ulid=?1", [&id]).map_err(RepositoryError::sqlite)?;
                tx.execute("UPDATE operation_records SET state='authoritative_observed',completed_step='context_retirement_absent' WHERE id=?1", [local_id]).map_err(RepositoryError::sqlite)?;
                tx.execute(
                    "UPDATE repositories SET refresh_required=1 WHERE id=?1",
                    [repository_id],
                )
                .map_err(RepositoryError::sqlite)?;
            }
            tx.commit().map_err(RepositoryError::sqlite)?;
        }
        drop(lease);
        // Full discovery is local and outside the lease, under this SAME local
        // operation's separate index epoch. It cannot invoke cleanup/transport.
        let refreshed = self.refresh_repository(RefreshRepositoryRequest {
            root: root.clone(),
            operation_id: request.operation_id,
        });
        match refreshed {
            Ok(RefreshOutcome::Refreshed { .. }) => {
                let _guard = cache_write_guard(&self.registry_path, &root, OP)?;
                let db = open_registry(&self.registry_path, &mut |_| {})?;
                record::audit_connection(&db)?;
                let json: String = db.query_row("SELECT final_result FROM context_retirement_records WHERE operation_ulid=?1", [request.operation_id.to_string()], |r| r.get(0)).map_err(RepositoryError::sqlite)?;
                Ok(ContextRetirementOutcome::Final(
                    ContextRetirementReport::decode(&json)?,
                ))
            }
            Ok(_) => Ok(ContextRetirementOutcome::IndexPending),
            Err(error)
                if matches!(
                    error.kind,
                    RepositoryErrorKind::Sqlite
                        | RepositoryErrorKind::InjectedFailure
                        | RepositoryErrorKind::IndexUnavailable
                ) =>
            {
                Ok(ContextRetirementOutcome::IndexPending)
            }
            Err(error) => Err(error),
        }
    }
}
