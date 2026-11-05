//! A marker is identity, never deletion authority. Only Selection, constructed
//! during this fresh lease-held call, may install its durable intended nonce.
use super::{
    RetireContextRequest,
    execution::{
        absent_path, plain_registration_path, ref_file_absent, require_registration_identity,
    },
};
use crate::{
    repository::{
        OperationId, RemoteReservation, RepositoryError, RepositoryService, authoring_kind_segment,
        cleanup::{ContextCleanup, Precheck, WorktreeRelease},
        item_context,
        remote::state,
    },
    results::ResultCode,
};
use rusqlite::params;
use state::recovery_required;
#[cfg(unix)]
use std::{fs::File, io::Write};
use std::{io::Read, path::PathBuf};
pub(super) const MARKER: &str = "manyhands-retirement-instance";
const LIMIT: u64 = 192;
#[cfg(test)]
struct BootstrapHook {
    after_install: bool,
    action: Box<dyn FnOnce()>,
}
#[cfg(test)]
thread_local! {static BOOTSTRAP_HOOK:std::cell::RefCell<Option<BootstrapHook>>=const { std::cell::RefCell::new(None) };}
#[cfg(test)]
thread_local! {static PRUNE_ENTRY_HOOK:std::cell::RefCell<Option<Box<dyn FnOnce()>>>=std::cell::RefCell::new(None);}
#[cfg(test)]
impl RepositoryService {
    #[allow(dead_code)]
    pub(crate) fn set_retirement_prune_entry_hook_for_testing(hook: impl FnOnce() + 'static) {
        PRUNE_ENTRY_HOOK.with(|slot| {
            assert!(slot.borrow().is_none());
            *slot.borrow_mut() = Some(Box::new(hook));
        });
    }
    #[allow(dead_code)]
    pub(crate) fn set_retirement_bootstrap_hook_for_testing(
        after_install: bool,
        hook: impl FnOnce() + 'static,
    ) {
        BOOTSTRAP_HOOK.with(|slot| {
            assert!(slot.borrow().is_none());
            *slot.borrow_mut() = Some(BootstrapHook {
                after_install,
                action: Box::new(hook),
            });
        });
    }
}
#[cfg(test)]
fn bootstrap_hook(after_install: bool) {
    let hook = BOOTSTRAP_HOOK.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot
            .as_ref()
            .is_some_and(|hook| hook.after_install == after_install)
        {
            slot.take().map(|hook| hook.action)
        } else {
            None
        }
    });
    if let Some(hook) = hook {
        hook();
    }
}
#[cfg(test)]
#[path = "instance_tests.rs"]
mod tests;

fn finish_provisional(
    service: &RepositoryService,
    request: &RetireContextRequest,
    owner: &RemoteReservation,
    cleanup: &ContextCleanup,
) -> Result<Verified, RepositoryError> {
    state::with_transaction(service, &request.root, |tx, id| {
        owner
            .retirement_execution()?
            .validate_install(service, tx, id)?;
        if owner.retirement_execution()?.operation() != request.operation_id {
            return Err(recovery_required());
        }
        let (tip,nonce):(String,String)=tx.query_row("SELECT expected_tip,instance_nonce FROM context_retirement_records WHERE operation_ulid=?1 AND kind=?2 AND item_id=?3 AND initial_resources='present' AND phase='selected' AND instance_verified=0",params![request.operation_id.to_string(),authoring_kind_segment(&request.kind),request.item_id.to_string()],|r|Ok((r.get(0)?,r.get(1)?))).map_err(RepositoryError::sqlite)?;
        let identity = Identity {
            root: request.root.clone(),
            kind: request.kind,
            item: request.item_id.clone(),
            tip: git2::Oid::from_str(&tip).map_err(|_| recovery_required())?,
            nonce: OperationId::parse(&nonce).map_err(|_| recovery_required())?,
        };
        let git = git2::Repository::open(&request.root).map_err(|_| recovery_required())?;
        identity.verify_marker(&git, request)?;
        Anchor::open(&admin(&git, &request.item_id))
            .and_then(|a| a.sync())
            .map_err(|_| recovery_required())?;
        if cleanup.precheck_context(identity.kind, &identity.item, identity.tip) != Precheck::Ok {
            return Err(recovery_required());
        }
        if tx.execute("UPDATE context_retirement_records SET instance_verified=1 WHERE operation_ulid=?1 AND owner_epoch=?2 AND instance_verified=0 AND phase='selected'",params![request.operation_id.to_string(),owner.retirement_execution()?.epoch()]).map_err(RepositoryError::sqlite)?!=1{return Err(recovery_required());}
        super::record::audit_connection(tx)?;
        Ok(Verified(identity))
    })
}

struct Identity {
    root: PathBuf,
    kind: crate::repository::AuthoringKind,
    item: crate::canonical::ItemId,
    tip: git2::Oid,
    nonce: OperationId,
}
pub(super) struct Selection {
    identity: Identity,
    anchor: Anchor,
    create: bool,
}
pub(super) struct Verified(Identity);
/// Returned only by a successful guarded live prune; never read from SQL.
pub(super) struct Pruned {
    _private: (),
}
pub(super) enum Prune {
    Removed(Pruned),
    NotInvoked,
    Uncertain,
}

fn admin(git: &git2::Repository, item: &crate::canonical::ItemId) -> PathBuf {
    git.commondir().join("worktrees").join(item.to_string())
}
fn bytes(
    kind: crate::repository::AuthoringKind,
    item: &crate::canonical::ItemId,
    nonce: OperationId,
) -> Vec<u8> {
    format!(
        "manyhands-retirement-instance-v1\n{}\n{item}\n{nonce}\n",
        authoring_kind_segment(&kind)
    )
    .into_bytes()
}
fn parse(
    data: &[u8],
    kind: crate::repository::AuthoringKind,
    item: &crate::canonical::ItemId,
) -> Result<OperationId, RepositoryError> {
    let text = std::str::from_utf8(data).map_err(|_| recovery_required())?;
    let nonce = OperationId::parse(text.lines().nth(3).ok_or_else(recovery_required)?)
        .map_err(|_| recovery_required())?;
    if data != bytes(kind, item, nonce) {
        return Err(recovery_required());
    }
    Ok(nonce)
}
/// Deletion-only admission for the selected admin namespace. Git can leave
/// index/HEAD/config/pseudoref and per-worktree ref/log lock files here without
/// setting the worktree's separate `locked` marker. Preserve every *.lock slot
/// regardless of node type, and never follow a redirected namespace to inspect it.
/// Match ASCII case variants on every platform, including case-sensitive hosts,
/// so a case-preserving filesystem cannot hide an occupied Git lock slot.
/// Non-ASCII entry components (including directories) have unsupported folding/
/// normalization identity and must not make the lock inventory appear complete.
fn require_admin_locks_absent(path: &std::path::Path) -> Result<(), RepositoryError> {
    plain_registration_path(path, true).map_err(|_| recovery_required())?;
    for entry in std::fs::read_dir(path).map_err(|_| recovery_required())? {
        let entry = entry.map_err(|_| recovery_required())?;
        let path = entry.path();
        if entry.file_name().to_str().is_none_or(|name| {
            !name.is_ascii()
                || name
                    .rsplit_once('.')
                    .is_some_and(|(_, suffix)| suffix.eq_ignore_ascii_case("lock"))
        }) {
            return Err(recovery_required());
        }
        let metadata = std::fs::symlink_metadata(&path).map_err(|_| recovery_required())?;
        if metadata.is_dir() {
            require_admin_locks_absent(&path)?;
        } else {
            plain_registration_path(&path, false).map_err(|_| recovery_required())?;
        }
    }
    Ok(())
}
/// One retirement-only checkout authorization for the entire effect chain.
/// While W is present, the only exemption is the exact selected admin's direct
/// symbolic context binding at its fixed tip. With no exemption, W/A must be
/// absent and EVERY checkout must be unrelated, including target case variants.
/// Every HEAD must be parseable and refer directly to an existing commit.
/// Symbolic HEADs are supported only for ASCII shared refs/heads/* bindings;
/// non-ASCII spelling has unsupported filesystem folding/normalization identity,
/// and other namespaces/pseudorefs may resolve in a checkout's own admin.
/// Keep ordinary cleanup/release's scanner and behavior unchanged.
pub(super) fn require_checkout_inventory(
    git: &git2::Repository,
    request: &RetireContextRequest,
    selected_tip: Option<git2::Oid>,
) -> Result<(), RepositoryError> {
    let selected = admin(git, &request.item_id);
    let context = item_context(&request.root, request.kind, request.item_id.clone());
    let target = format!("refs/heads/{}", context.branch);
    require_registration_identity(git).map_err(|_| recovery_required())?;
    ref_file_absent(&request.root, &git.commondir().join("packed-refs"))?;
    if selected_tip.is_none()
        && (!absent_path(&request.root, &selected)?
            || !absent_path(&request.root, &context.worktree)?)
    {
        return Err(recovery_required());
    }
    let check = |directory: &std::path::Path, is_selected: bool| -> Result<(), RepositoryError> {
        plain_registration_path(&directory.join("HEAD"), false).map_err(|_| recovery_required())?;
        let bytes =
            std::fs::read_to_string(directory.join("HEAD")).map_err(|_| recovery_required())?;
        let head = bytes
            .strip_suffix("\r\n")
            .or_else(|| bytes.strip_suffix('\n'))
            .unwrap_or(&bytes);
        if head.is_empty() || head.contains(['\r', '\n', '\0']) {
            return Err(recovery_required());
        }
        if let Some(reference) = head.strip_prefix("ref: ") {
            // Do not resolve a per-worktree name through the primary refdb:
            // a common-dir direct ref could mask a linked admin's target alias.
            if !reference.is_ascii()
                || !reference.starts_with("refs/heads/")
                || !git2::Reference::is_valid_name(reference)
            {
                return Err(recovery_required());
            }
            // Refuse ASCII-equivalent target spellings even on case-sensitive
            // hosts. A case-preserving volume may map them to the same ref slot.
            if !is_selected && reference.eq_ignore_ascii_case(&target) {
                return Err(recovery_required());
            }
            ref_file_absent(&request.root, &git.commondir().join(reference))?;
            let actual = git
                .find_reference(reference)
                .map_err(|_| recovery_required())?;
            // Never resolve an indirect alias and mistake it for an unrelated
            // checkout, including aliases which ultimately name our context.
            if actual.symbolic_target().is_some() {
                return Err(recovery_required());
            }
            let oid = actual
                .target()
                .filter(|oid| !oid.is_zero())
                .ok_or_else(recovery_required)?;
            git.find_commit(oid).map_err(|_| recovery_required())?;
            if is_selected && (reference != target || Some(oid) != selected_tip) {
                return Err(recovery_required());
            }
        } else {
            let oid = git2::Oid::from_str(head).map_err(|_| recovery_required())?;
            if is_selected || oid.is_zero() || oid.to_string() != head {
                return Err(recovery_required());
            }
            git.find_commit(oid).map_err(|_| recovery_required())?;
        }
        Ok(())
    };
    check(git.path(), false)?;
    let worktrees = git.commondir().join("worktrees");
    if absent_path(&request.root, &worktrees)? {
        return if selected_tip.is_none() {
            Ok(())
        } else {
            Err(recovery_required())
        };
    }
    let mut found = false;
    for entry in std::fs::read_dir(worktrees).map_err(|_| recovery_required())? {
        let path = entry.map_err(|_| recovery_required())?.path();
        let is_selected = selected_tip.is_some() && path == selected;
        check(&path, is_selected)?;
        found |= is_selected;
    }
    if selected_tip.is_some() && !found {
        return Err(recovery_required());
    }
    Ok(())
}
pub(super) fn identity(
    git: &git2::Repository,
    request: &RetireContextRequest,
    tip: git2::Oid,
) -> Result<(), RepositoryError> {
    let context = item_context(&request.root, request.kind, request.item_id.clone());
    let reference = format!("refs/heads/{}", context.branch);
    require_registration_identity(git).map_err(|_| recovery_required())?;
    plain_registration_path(&git.path().join("HEAD"), false).map_err(|_| recovery_required())?;
    plain_registration_path(&context.worktree, true).map_err(|_| recovery_required())?;
    ref_file_absent(&request.root, &git.commondir().join(&reference))?;
    ref_file_absent(&request.root, &git.commondir().join("packed-refs"))?;
    for lock in [
        git.commondir().join(format!("{reference}.lock")),
        git.commondir().join("packed-refs.lock"),
    ] {
        if !absent_path(&request.root, &lock)? {
            return Err(recovery_required());
        }
    }
    require_admin_locks_absent(&admin(git, &request.item_id))?;
    let branch = git
        .find_reference(&reference)
        .map_err(|_| recovery_required())?;
    if branch.symbolic_target().is_some() || branch.target() != Some(tip) {
        return Err(recovery_required());
    }
    crate::repository::validate_context_worktree_identity(
        git,
        &context,
        crate::repository::RepositoryOperation::RetireContext,
    )?;
    let linked = git2::Repository::open(&context.worktree).map_err(|_| recovery_required())?;
    if linked.path().canonicalize().ok()
        != Some(
            admin(git, &request.item_id)
                .canonicalize()
                .map_err(|_| recovery_required())?,
        )
        || linked.commondir().canonicalize().ok() != git.commondir().canonicalize().ok()
    {
        return Err(recovery_required());
    }
    require_checkout_inventory(git, request, Some(tip))
}
impl Selection {
    pub(super) fn capture(
        git: &git2::Repository,
        request: &RetireContextRequest,
        cleanup: &ContextCleanup,
    ) -> Result<Self, ResultCode> {
        let context = item_context(&request.root, request.kind, request.item_id.clone());
        let tip = git
            .refname_to_id(&format!("refs/heads/{}", context.branch))
            .map_err(|_| ResultCode::RecoveryRequired)?;
        identity(git, request, tip).map_err(|_| ResultCode::RecoveryRequired)?;
        if cleanup.precheck_context(request.kind, &request.item_id, tip) != Precheck::Ok {
            return Err(ResultCode::WorktreeNotClean);
        }
        let anchor = Anchor::open(&admin(git, &request.item_id))
            .map_err(|_| ResultCode::RecoveryRequired)?;
        let marker = anchor.read().map_err(|_| ResultCode::RecoveryRequired)?;
        let (nonce, create) = match marker {
            Some(data) => (
                parse(&data, request.kind, &request.item_id)
                    .map_err(|_| ResultCode::RecoveryRequired)?,
                false,
            ),
            None => (OperationId::new(), true),
        };
        Ok(Self {
            identity: Identity {
                root: request.root.clone(),
                kind: request.kind,
                item: request.item_id.clone(),
                tip,
                nonce,
            },
            anchor,
            create,
        })
    }
    pub(super) fn tip(&self) -> git2::Oid {
        self.identity.tip
    }
    pub(super) fn nonce(&self) -> OperationId {
        self.identity.nonce
    }
    pub(super) fn install(
        self,
        service: &RepositoryService,
        request: &RetireContextRequest,
        owner: &RemoteReservation,
        cleanup: &ContextCleanup,
    ) -> Result<Verified, RepositoryError> {
        let expected = &self.identity;
        state::with_transaction(service, &expected.root, |tx, id| {
            owner
                .retirement_execution()?
                .validate_install(service, tx, id)?;
            let bound:bool=tx.query_row("SELECT initial_resources='present' AND phase='selected' AND instance_verified=0 AND instance_nonce=?2 AND expected_tip=?3 AND kind=?4 AND item_id=?5 FROM context_retirement_records WHERE operation_ulid=?1",params![request.operation_id.to_string(),expected.nonce.to_string(),expected.tip.to_string(),authoring_kind_segment(&expected.kind),expected.item.to_string()],|r|r.get(0)).map_err(RepositoryError::sqlite)?;
            if !bound {
                return Err(recovery_required());
            }
            let git = git2::Repository::open(&expected.root).map_err(|_| recovery_required())?;
            identity(&git, request, expected.tip)?;
            if cleanup.precheck_context(expected.kind, &expected.item, expected.tip) != Precheck::Ok
            {
                return Err(recovery_required());
            }
            self.anchor.validate().map_err(|_| recovery_required())?;
            if self.create {
                #[cfg(test)]
                bootstrap_hook(false);
                self.anchor
                    .create(&bytes(expected.kind, &expected.item, expected.nonce))
                    .map_err(|_| recovery_required())?;
            }
            expected.verify_marker(&git, request)?;
            self.anchor.validate().map_err(|_| recovery_required())?;
            super::record::audit_connection(tx)
        })?;
        #[cfg(test)]
        bootstrap_hook(true);
        // This phase has NO installation authority; it may only verify a fully
        // installed marker against the durable intent and finish the SQL CAS.
        self.anchor.validate().map_err(|_| recovery_required())?;
        finish_provisional(service, request, owner, cleanup)
    }
}
impl Identity {
    fn verify_marker(
        &self,
        git: &git2::Repository,
        request: &RetireContextRequest,
    ) -> Result<(), RepositoryError> {
        identity(git, request, self.tip)?;
        let data = Anchor::open(&admin(git, &self.item))
            .and_then(|a| a.read())
            .map_err(|_| recovery_required())?
            .ok_or_else(recovery_required)?;
        if parse(&data, self.kind, &self.item)? != self.nonce {
            return Err(recovery_required());
        }
        Ok(())
    }
}
impl Verified {
    pub(super) fn matches_stored(
        tx: &rusqlite::Transaction<'_>,
        git: &git2::Repository,
        request: &RetireContextRequest,
        cleanup: &ContextCleanup,
    ) -> Result<bool, RepositoryError> {
        let (tip,nonce):(String,String)=tx.query_row("SELECT expected_tip,instance_nonce FROM context_retirement_records WHERE operation_ulid=?1 AND initial_resources='present'",[request.operation_id.to_string()],|r|Ok((r.get(0)?,r.get(1)?))).map_err(RepositoryError::sqlite)?;
        let identity = Identity {
            root: request.root.clone(),
            kind: request.kind,
            item: request.item_id.clone(),
            tip: git2::Oid::from_str(&tip).map_err(|_| recovery_required())?,
            nonce: OperationId::parse(&nonce).map_err(|_| recovery_required())?,
        };
        Ok(identity.verify_marker(git, request).is_ok()
            && cleanup.precheck_context(request.kind, &request.item_id, identity.tip)
                == Precheck::Ok)
    }
    /// Reentry has verification-only authority. No Selection/create permit is
    /// reconstructed from the stored intended nonce, even when its marker lacks.
    pub(super) fn resume(
        service: &RepositoryService,
        request: &RetireContextRequest,
        owner: &RemoteReservation,
        cleanup: &ContextCleanup,
    ) -> Result<Self, RepositoryError> {
        let verified = state::with_transaction(service, &request.root, |tx, id| {
            owner
                .retirement_execution()?
                .validate_install(service, tx, id)?;
            let (tip,nonce,verified):(String,String,bool)=tx.query_row("SELECT expected_tip,instance_nonce,instance_verified FROM context_retirement_records c WHERE operation_ulid=?1 AND initial_resources='present' AND (phase='selected' OR (phase='worktree_removing' AND instance_verified=1 AND EXISTS(SELECT 1 FROM context_retirement_worktree_resumptions w WHERE w.operation_ulid=c.operation_ulid AND w.owner_epoch=c.owner_epoch)))",[request.operation_id.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(RepositoryError::sqlite)?;
            let identity = Identity {
                root: request.root.clone(),
                kind: request.kind,
                item: request.item_id.clone(),
                tip: git2::Oid::from_str(&tip).map_err(|_| recovery_required())?,
                nonce: OperationId::parse(&nonce).map_err(|_| recovery_required())?,
            };
            let git = git2::Repository::open(&request.root).map_err(|_| recovery_required())?;
            identity.verify_marker(&git, request)?;
            if cleanup.precheck_context(request.kind, &request.item_id, identity.tip)
                != Precheck::Ok
            {
                return Err(recovery_required());
            }
            Ok(verified.then_some(Self(identity)))
        })?;
        match verified {
            Some(v) => Ok(v),
            None => finish_provisional(service, request, owner, cleanup),
        }
    }
    pub(super) fn verify(
        &self,
        git: &git2::Repository,
        request: &RetireContextRequest,
        cleanup: &ContextCleanup,
    ) -> Result<(), RepositoryError> {
        self.0.verify_marker(git, request)?;
        if cleanup.precheck_context(self.0.kind, &self.0.item, self.0.tip) != Precheck::Ok {
            return Err(recovery_required());
        }
        Ok(())
    }
    pub(super) fn prune(
        self,
        git: &git2::Repository,
        request: &RetireContextRequest,
        cleanup: &ContextCleanup,
        authorize: impl FnOnce() -> bool,
    ) -> Prune {
        let result =
            cleanup.remove_retirement_worktree(self.0.kind, &self.0.item, self.0.tip, || {
                #[cfg(test)]
                {
                    let hook = PRUNE_ENTRY_HOOK.with(|slot| slot.borrow_mut().take());
                    if let Some(hook) = hook {
                        hook();
                    }
                }
                authorize() && self.0.verify_marker(git, request).is_ok()
            });
        match result {
            WorktreeRelease::Removed => Prune::Removed(Pruned { _private: () }),
            WorktreeRelease::Incomplete => Prune::Uncertain,
            WorktreeRelease::AlreadyAbsent
            | WorktreeRelease::NotClean
            | WorktreeRelease::RecoveryRequired => Prune::NotInvoked,
        }
    }
}

#[cfg(unix)]
struct Anchor {
    directory: File,
    path: PathBuf,
}
#[cfg(unix)]
impl Anchor {
    fn open(path: &std::path::Path) -> std::io::Result<Self> {
        Ok(Self {
            directory: crate::repository::native_resolution::open_directory(path)?,
            path: path.into(),
        })
    }
    fn file(&self, create: bool) -> std::io::Result<Option<File>> {
        use std::os::fd::{AsRawFd, FromRawFd};
        let flags = if create {
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL
        } else {
            libc::O_RDONLY
        };
        // SAFETY: live pinned directory, fixed NUL-terminated leaf, owned fd.
        let name = std::ffi::CString::new(MARKER).expect("fixed marker leaf");
        let fd = unsafe {
            libc::openat(
                self.directory.as_raw_fd(),
                name.as_ptr(),
                flags | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
                0o600 as libc::c_uint,
            )
        };
        if fd < 0 {
            let e = std::io::Error::last_os_error();
            if !create && e.kind() == std::io::ErrorKind::NotFound {
                return Ok(None);
            }
            return Err(e);
        }
        let file = unsafe { File::from_raw_fd(fd) };
        use std::os::unix::fs::MetadataExt;
        let m = file.metadata()?;
        if !m.is_file() || m.nlink() != 1 || m.uid() != unsafe { libc::geteuid() } {
            return Err(std::io::ErrorKind::InvalidData.into());
        }
        Ok(Some(file))
    }
    fn read(&self) -> std::io::Result<Option<Vec<u8>>> {
        self.validate()?;
        let Some(file) = self.file(false)? else {
            return Ok(None);
        };
        let mut bytes = Vec::new();
        file.take(LIMIT + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > LIMIT
            || !crate::repository::native_resolution::directory_matches(
                &self.path,
                &self.directory,
            )?
        {
            return Err(std::io::ErrorKind::InvalidData.into());
        }
        Ok(Some(bytes))
    }
    fn validate(&self) -> std::io::Result<()> {
        if crate::repository::native_resolution::directory_matches(&self.path, &self.directory)? {
            Ok(())
        } else {
            Err(std::io::ErrorKind::InvalidData.into())
        }
    }
    fn sync(&self) -> std::io::Result<()> {
        self.validate()?;
        self.file(false)?
            .ok_or(std::io::ErrorKind::NotFound)?
            .sync_all()?;
        self.directory.sync_all()?;
        self.validate()
    }
    fn create(&self, bytes: &[u8]) -> std::io::Result<()> {
        if !crate::repository::native_resolution::directory_matches(&self.path, &self.directory)? {
            return Err(std::io::ErrorKind::InvalidData.into());
        }
        let mut file = self.file(true)?.ok_or(std::io::ErrorKind::NotFound)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        self.directory.sync_all()?;
        if self.read()?.as_deref() != Some(bytes) {
            return Err(std::io::ErrorKind::InvalidData.into());
        }
        Ok(())
    }
}
#[cfg(windows)]
struct Anchor(crate::repository::native_resolution::Directory);
#[cfg(windows)]
impl Anchor {
    fn open(path: &std::path::Path) -> std::io::Result<Self> {
        crate::repository::native_resolution::Directory::open(path).map(Self)
    }
    fn read(&self) -> std::io::Result<Option<Vec<u8>>> {
        let Some(file) = self.0.file(MARKER)? else {
            return Ok(None);
        };
        {
            use std::os::windows::io::AsRawHandle;
            let mut information:windows_sys::Win32::Storage::FileSystem::BY_HANDLE_FILE_INFORMATION=unsafe{std::mem::zeroed()};
            if unsafe {
                windows_sys::Win32::Storage::FileSystem::GetFileInformationByHandle(
                    file.as_raw_handle(),
                    &mut information,
                )
            } == 0
            {
                return Err(std::io::Error::last_os_error());
            }
            if information.nNumberOfLinks != 1 {
                return Err(std::io::ErrorKind::InvalidData.into());
            }
        }
        let mut bytes = Vec::new();
        file.take(LIMIT + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > LIMIT {
            return Err(std::io::ErrorKind::InvalidData.into());
        }
        self.0.revalidate()?;
        Ok(Some(bytes))
    }
    fn create(&self, bytes: &[u8]) -> std::io::Result<()> {
        self.0.create_image(MARKER, bytes)?;
        self.0.revalidate()
    }
    fn validate(&self) -> std::io::Result<()> {
        self.0.revalidate()
    }
    fn sync(&self) -> std::io::Result<()> {
        self.0.flush(MARKER)?;
        self.0.revalidate()
    }
}
#[cfg(not(any(unix, windows)))]
struct Anchor;
#[cfg(not(any(unix, windows)))]
impl Anchor {
    fn open(_: &std::path::Path) -> std::io::Result<Self> {
        Err(std::io::ErrorKind::Unsupported.into())
    }
    fn read(&self) -> std::io::Result<Option<Vec<u8>>> {
        Err(std::io::ErrorKind::Unsupported.into())
    }
    fn create(&self, _: &[u8]) -> std::io::Result<()> {
        Err(std::io::ErrorKind::Unsupported.into())
    }
    fn validate(&self) -> std::io::Result<()> {
        Err(std::io::ErrorKind::Unsupported.into())
    }
    fn sync(&self) -> std::io::Result<()> {
        Err(std::io::ErrorKind::Unsupported.into())
    }
}
