use super::lifecycle::LifecycleKind;
use super::*;
use crate::{
    canonical,
    repository::{
        remote::{state, sync},
        *,
    },
};
use std::collections::BTreeSet;

/// Ephemeral observation shared by preview and the later post-sync/leased checks.
/// It carries no authority to mutate Git or the registry.
pub(super) struct PromotionObservation {
    blockers: BTreeSet<PromotionBlocker>,
    preview: Option<DocumentPromotionPreview>,
}

impl PromotionObservation {
    pub(super) fn into_preflight(self) -> DocumentPromotionPreflight {
        if self.blockers.is_empty() {
            DocumentPromotionPreflight::Ready(
                self.preview.expect("unblocked observation has a preview"),
            )
        } else {
            DocumentPromotionPreflight::Blocked {
                blockers: self.blockers.into_iter().collect(),
            }
        }
    }
}

pub(in super::super) fn unavailable(root: &Path) -> RepositoryError {
    RepositoryError::new(
        RepositoryOperation::Read,
        Some(root.into()),
        RepositoryErrorKind::Git,
        "promotion Git observation unavailable",
    )
}

pub(super) fn preview(
    service: &RepositoryService,
    request: PreviewDocumentPromotionRequest,
) -> Result<DocumentPromotionPreflight, RepositoryError> {
    Ok(prepare(service, request)?.preflight)
}

/// Filesystem discovery facts are gathered only outside the repository lease.
/// Their process-only guard binds the selected checkout/namespace and actual Git
/// inputs; revalidation never calls filesystem discovery or enumerates worktrees.
pub(in super::super) struct PreparedObservation {
    pub(super) preflight: DocumentPromotionPreflight,
    pub(super) configuration: canonical::RepositoryConfig,
    pub(super) repository_id: i64,
    seal: ObservationSeal,
    facts: DiscoveryFacts,
}
struct DiscoveryFacts {
    kind: LifecycleKind,
    not_document: bool,
    document_path: Option<PathBuf>,
}
impl PreparedObservation {
    pub(super) fn revalidate(
        &self,
        service: &RepositoryService,
        repository: &git2::Repository,
    ) -> Result<DocumentPromotionPreflight, RepositoryError> {
        let fresh = self.seal.reopen(repository, &self.configuration)?;
        let (id, snapshot, pending) = read_view(service, &self.seal.root)?;
        if id != self.repository_id {
            return Err(changed());
        }
        Ok(observe_git(
            &fresh,
            &self.seal.root,
            &self.configuration,
            &self.seal.item,
            &snapshot,
            pending,
            &self.facts,
        )?
        .into_preflight())
    }
}
pub(in super::super) fn changed() -> RepositoryError {
    RepositoryError::new(
        RepositoryOperation::Read,
        None,
        RepositoryErrorKind::ExternalChange,
        "promotion observation changed",
    )
}

/// The process-only binding between unleased discovery and a later
/// revalidation: the lifecycle kind, the canonical root, the item and the
/// observation guard taken before discovery (shared by both lifecycles).
pub(in super::super) struct ObservationSeal {
    kind: LifecycleKind,
    root: PathBuf,
    item: ItemId,
    guard: [u8; 32],
}
impl ObservationSeal {
    /// Runs `discover` (filesystem discovery, outside any lease) between two
    /// equal observation guards.
    pub(in super::super) fn discover<F>(
        kind: LifecycleKind,
        repository: &git2::Repository,
        root: &Path,
        config: &canonical::RepositoryConfig,
        item: &ItemId,
        discover: impl FnOnce() -> F,
    ) -> Result<(F, Self), RepositoryError> {
        let before = observation_guard(kind, repository, root, config, item)?;
        let facts = discover();
        if before != observation_guard(kind, repository, root, config, item)? {
            return Err(changed());
        }
        Ok((
            facts,
            Self {
                kind,
                root: root.to_owned(),
                item: item.clone(),
                guard: before,
            },
        ))
    }
    #[allow(dead_code)] // Read by the closure revalidation (Task 6 caller).
    pub(in super::super) fn root(&self) -> &Path {
        &self.root
    }
    #[allow(dead_code)] // Read by the closure revalidation (Task 6 caller).
    pub(in super::super) fn item(&self) -> &ItemId {
        &self.item
    }
    /// A fresh handle on the same checkout, refused unless its common
    /// directory and the observation guard are unchanged.
    pub(in super::super) fn reopen(
        &self,
        repository: &git2::Repository,
        configuration: &canonical::RepositoryConfig,
    ) -> Result<git2::Repository, RepositoryError> {
        let fresh = git2::Repository::open_ext(
            &self.root,
            git2::RepositoryOpenFlags::NO_SEARCH,
            &[] as &[&std::ffi::OsStr],
        )
        .map_err(|_| changed())?;
        if fresh.commondir().canonicalize().ok() != repository.commondir().canonicalize().ok()
            || observation_guard(self.kind, &fresh, &self.root, configuration, &self.item)?
                != self.guard
        {
            return Err(changed());
        }
        Ok(fresh)
    }
}

/// The canonical repository and its valid configuration, opened for an
/// unleased observation.
pub(in super::super) fn open_observation(
    kind: LifecycleKind,
    service: &RepositoryService,
    root: &Path,
) -> Result<(git2::Repository, PathBuf, canonical::RepositoryConfig), RepositoryError> {
    let operation = RepositoryOperation::Read;
    service.require_index_available(operation, Some(root))?;
    let (repository, root) = canonical_repository_root(root, operation)?;
    let config = match read_configuration_for(&root, operation)? {
        ConfigurationInspection::Valid(config) => config,
        _ => {
            return Err(RepositoryError::new(
                operation,
                Some(root),
                RepositoryErrorKind::InvalidConfiguration,
                match kind {
                    LifecycleKind::DocumentPromotion => {
                        "promotion requires a valid repository configuration"
                    }
                    LifecycleKind::TicketClosure => {
                        "closure requires a valid repository configuration"
                    }
                },
            ));
        }
    };
    Ok((repository, root, config))
}

pub(super) fn prepare(
    service: &RepositoryService,
    request: PreviewDocumentPromotionRequest,
) -> Result<PreparedObservation, RepositoryError> {
    prepare_for(LifecycleKind::DocumentPromotion, service, request)
}
pub(super) fn prepare_for(
    kind: LifecycleKind,
    service: &RepositoryService,
    request: PreviewDocumentPromotionRequest,
) -> Result<PreparedObservation, RepositoryError> {
    if kind != LifecycleKind::DocumentPromotion {
        // The discovery facts and result checks below are document-only; the
        // closure observation has its own (`closure::observe`).
        return Err(state::recovery_required());
    }
    let (repository, root, config) = open_observation(kind, service, &request.root)?;
    // A read-only SQLite transaction provides one coherent history/conflict view.
    // No repository lease, reservation, refresh, migration or transport is used.
    let (repository_id, snapshot, pending) = read_view(service, &root)?;
    let (facts, seal) =
        ObservationSeal::discover(kind, &repository, &root, &config, &request.item_id, || {
            let discovered = discovery::observe_root(&repository, &root);
            let active = discovered
                .active_contexts
                .iter()
                .find(|a| a.context.item_id.as_ref() == Some(&request.item_id));
            DiscoveryFacts {
                kind,
                not_document: active.is_some_and(|a| {
                    a.items
                        .iter()
                        .any(|i| i.id == request.item_id && i.kind == AuthoringKind::Ticket)
                }) || (active.is_none()
                    && discovered
                        .items
                        .iter()
                        .any(|i| i.id == request.item_id && i.kind == AuthoringKind::Ticket)),
                document_path: active
                    .filter(|a| {
                        a.context.branch.as_deref()
                            == Some(kind.context_branch(&request.item_id).as_str())
                    })
                    .and_then(|a| {
                        a.items
                            .iter()
                            .find(|i| i.id == request.item_id && i.kind == kind.authoring())
                    })
                    .map(|i| i.path.clone()),
            }
        })?;
    let preflight = observe_git(
        &repository,
        &root,
        &config,
        &request.item_id,
        &snapshot,
        pending,
        &facts,
    )?
    .into_preflight();
    let prepared = PreparedObservation {
        preflight,
        configuration: config,
        repository_id,
        seal,
        facts,
    };
    #[cfg(test)]
    super::run_tests::observation_prepared(service, &root);
    Ok(prepared)
}
fn read_view(
    service: &RepositoryService,
    root: &Path,
) -> Result<(i64, RemoteSnapshot, bool), RepositoryError> {
    read_view_with(service, root, |_, _| Ok(()))
        .map(|(id, snapshot, pending, ())| (id, snapshot, pending))
}
/// One read-only SQLite transaction: the registration, the remote snapshot,
/// the pending-conflict flag and whatever else `extra` reads in the same view.
pub(in super::super) fn read_view_with<T>(
    service: &RepositoryService,
    root: &Path,
    extra: impl FnOnce(&rusqlite::Transaction<'_>, i64) -> Result<T, RepositoryError>,
) -> Result<(i64, RemoteSnapshot, bool, T), RepositoryError> {
    let operation = RepositoryOperation::Read;
    let (id, snapshot, pending, extra) = {
        let _guard = cache_read_guard(&service.registry_path, root, operation)?;
        let mut connection = open_registry_read_only(&service.registry_path)?;
        let transaction = connection.transaction().map_err(RepositoryError::sqlite)?;
        let id = transaction
            .query_row(
                "SELECT id FROM repositories WHERE root_path=?1",
                [registry_root_key(root, operation)?],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(RepositoryError::sqlite)?
            .ok_or_else(|| {
                RepositoryError::new(
                    operation,
                    Some(root.to_owned()),
                    RepositoryErrorKind::RepositoryNotRegistered,
                    "promotion requires a registered repository",
                )
            })?;
        let snapshot = state::read_snapshot(&transaction, id)?;
        let pending = state::has_pending_conflict(&transaction, id)?;
        let extra = extra(&transaction, id)?;
        (id, snapshot, pending, extra)
    };
    Ok((id, snapshot, pending, extra))
}
fn guard_field(hash: &mut blake3::Hasher, bytes: &[u8]) {
    hash.update(&(bytes.len() as u64).to_be_bytes());
    hash.update(bytes);
}
/// Fixed locator metadata and known canonical/index paths, not a discovery walk.
/// Git status is re-read (including ignored canonical paths) so new namespaces
/// cannot hide behind cached discovery. Metadata of known canonical files also
/// catches edits masked by index flags without reading/scanning their contents.
fn observation_guard(
    kind: LifecycleKind,
    repository: &git2::Repository,
    root: &Path,
    config: &canonical::RepositoryConfig,
    item: &ItemId,
) -> Result<[u8; 32], RepositoryError> {
    fn metadata(
        hash: &mut blake3::Hasher,
        path: &Path,
        contents: bool,
    ) -> Result<(), RepositoryError> {
        guard_field(hash, path.as_os_str().as_encoded_bytes());
        match std::fs::symlink_metadata(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                hash.update(&[0]);
            }
            Err(_) => return Err(changed()),
            Ok(m) => {
                hash.update(&[
                    1,
                    u8::from(m.is_dir()),
                    u8::from(m.is_file()),
                    u8::from(m.file_type().is_symlink()),
                ]);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    hash.update(&m.dev().to_be_bytes());
                    hash.update(&m.ino().to_be_bytes());
                }
                #[cfg(windows)]
                if !m.file_type().is_symlink() {
                    // Reuse the existing Win32 namespace/identity backend;
                    // metadata timestamps alone do not identify directories.
                    let identity = if m.is_dir() {
                        native_resolution::Directory::open(path).and_then(|d| d.identity())
                    } else {
                        let parent = path.parent().ok_or_else(changed)?;
                        let name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .ok_or_else(changed)?;
                        let directory =
                            native_resolution::Directory::open(parent).map_err(|_| changed())?;
                        let file = directory
                            .file(name)
                            .map_err(|_| changed())?
                            .ok_or_else(changed)?;
                        native_resolution::identity(&file)
                    }
                    .map_err(|_| changed())?;
                    for part in identity {
                        hash.update(&part.to_be_bytes());
                    }
                }
                if !m.is_dir() {
                    hash.update(&m.len().to_be_bytes());
                    for time in [m.modified().ok(), m.created().ok()] {
                        match time.and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()) {
                            Some(t) => {
                                hash.update(&[1]);
                                hash.update(&t.as_nanos().to_be_bytes());
                            }
                            None => {
                                hash.update(&[0]);
                            }
                        }
                    }
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::MetadataExt;
                        hash.update(&m.ctime().to_be_bytes());
                        hash.update(&m.ctime_nsec().to_be_bytes());
                        hash.update(&m.mode().to_be_bytes());
                    }
                    if contents && m.is_file() && !m.file_type().is_symlink() {
                        guard_field(hash, &std::fs::read(path).map_err(|_| changed())?);
                    }
                }
            }
        }
        Ok(())
    }
    fn ref_stamp(
        hash: &mut blake3::Hasher,
        repo: &git2::Repository,
        name: &str,
    ) -> Result<(), RepositoryError> {
        guard_field(hash, name.as_bytes());
        match repo.find_reference(name) {
            Ok(r) => {
                hash.update(&[1]);
                guard_field(hash, r.symbolic_target_bytes().unwrap_or_default());
                if let Some(oid) = r.target() {
                    guard_field(hash, oid.as_bytes());
                }
            }
            Err(e) if e.code() == git2::ErrorCode::NotFound => {
                hash.update(&[0]);
            }
            Err(_) => return Err(changed()),
        }
        Ok(())
    }
    fn canonical_path(bytes: &[u8]) -> Option<&Path> {
        let path = Path::new(std::str::from_utf8(bytes).ok()?);
        (path
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
            && (path.starts_with("docs")
                || path.starts_with(".manyhands/tickets")
                || path.starts_with(".manyhands/comments")))
        .then_some(path)
    }
    let mut hash = blake3::Hasher::new();
    // Match the configuration reader's bytes even if an existing configuration
    // uses an indirection: locator metadata alone is not configuration consent.
    guard_field(
        &mut hash,
        &std::fs::read(root.join(canonical::CONFIG_PATH)).map_err(|_| changed())?,
    );
    let context = root.join(".manyhands/worktrees").join(item.to_string());
    let admin = repository
        .commondir()
        .join("worktrees")
        .join(item.to_string());
    for path in [
        root.to_owned(),
        repository.path().into(),
        repository.commondir().into(),
        root.join(".git"),
        root.join(".manyhands"),
        root.join(canonical::CONFIG_PATH),
        context.clone(),
        context.join(".git"),
        admin.clone(),
        admin.join("HEAD"),
        admin.join("gitdir"),
        admin.join("commondir"),
        root.join(kind.guarded_directory()),
        context.join(kind.guarded_directory()),
    ] {
        metadata(&mut hash, &path, true)?;
    }
    ref_stamp(
        &mut hash,
        repository,
        &format!("refs/heads/{}", config.primary_branch),
    )?;
    ref_stamp(&mut hash, repository, &kind.context_ref(item))?;
    for path in [root, &context] {
        let linked = git2::Repository::open_ext(
            path,
            git2::RepositoryOpenFlags::NO_SEARCH,
            &[] as &[&std::ffi::OsStr],
        );
        let Ok(linked) = linked else {
            hash.update(&[0]);
            continue;
        };
        hash.update(&[1]);
        guard_field(&mut hash, linked.path().as_os_str().as_encoded_bytes());
        guard_field(&mut hash, linked.commondir().as_os_str().as_encoded_bytes());
        if let Some(workdir) = linked.workdir() {
            guard_field(&mut hash, workdir.as_os_str().as_encoded_bytes());
        }
        ref_stamp(&mut hash, &linked, "HEAD")?;
        for name in [
            "config",
            "index",
            "MERGE_HEAD",
            "MERGE_MSG",
            "MERGE_MODE",
            "REBASE_HEAD",
            "CHERRY_PICK_HEAD",
            "rebase-apply",
            "rebase-merge",
        ] {
            metadata(&mut hash, &linked.path().join(name), true)?;
        }
        let index = linked.index().map_err(|_| changed())?;
        let mut count = 0;
        for entry in index.iter() {
            guard_field(&mut hash, &entry.path);
            hash.update(entry.id.as_bytes());
            hash.update(&entry.mode.to_be_bytes());
            hash.update(&entry.flags.to_be_bytes());
            hash.update(&entry.flags_extended.to_be_bytes());
            if let Some(relative) = canonical_path(&entry.path) {
                count += 1;
                if count > MAX_DOCUMENT_DIRECTORY_ENTRIES + MAX_MANAGED_DIRECTORY_ENTRIES {
                    return Err(changed());
                }
                metadata(&mut hash, &path.join(relative), false)?;
            }
        }
        for canonical_only in [false, true] {
            let mut options = git2::StatusOptions::new();
            options.include_untracked(true).recurse_untracked_dirs(true);
            if canonical_only {
                options
                    .include_ignored(true)
                    .recurse_ignored_dirs(true)
                    .pathspec("docs")
                    .pathspec(".manyhands/tickets")
                    .pathspec(".manyhands/comments");
            }
            let statuses = linked.statuses(Some(&mut options)).map_err(|_| changed())?;
            hash.update(&(statuses.len() as u64).to_be_bytes());
            for entry in statuses.iter() {
                guard_field(&mut hash, entry.path_bytes());
                hash.update(&entry.status().bits().to_be_bytes());
                if canonical_only {
                    count += 1;
                    if count > MAX_DOCUMENT_DIRECTORY_ENTRIES + MAX_MANAGED_DIRECTORY_ENTRIES {
                        return Err(changed());
                    }
                    if let Some(relative) = canonical_path(entry.path_bytes()) {
                        metadata(&mut hash, &path.join(relative), false)?;
                    }
                }
            }
        }
    }
    Ok(*hash.finalize().as_bytes())
}

pub(in super::super) fn ancestor(
    repository: &git2::Repository,
    older: Oid,
    newer: Oid,
    root: &Path,
) -> Result<bool, RepositoryError> {
    if older == newer {
        return Ok(true);
    }
    repository
        .graph_descendant_of(newer, older)
        .map_err(|_| unavailable(root))
}

pub(in super::super) fn reference_oid(
    repository: &git2::Repository,
    name: &str,
    root: &Path,
) -> Result<Option<Oid>, RepositoryError> {
    match repository.find_reference(name) {
        Ok(reference) => reference
            .peel_to_commit()
            .map(|c| Some(c.id()))
            .map_err(|_| unavailable(root)),
        Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(_) => Err(unavailable(root)),
    }
}

/// A materialized target's worktree state, as `require_clean_target` reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum Cleanliness {
    Clean,
    Dirty,
    Conflicted,
}
pub(in super::super) fn target_cleanliness(
    linked: &git2::Repository,
    target: &SynchronizationTarget,
    root: &Path,
) -> Result<Cleanliness, RepositoryError> {
    // require_clean_target verifies the index tree. Keep even its tree cache
    // writes in memory, including when the observed index is not canonical.
    let odb = linked.odb().map_err(|_| unavailable(root))?;
    let _memory = odb
        .add_new_mempack_backend(1000)
        .map_err(|_| unavailable(root))?;
    match sync::require_clean_target(linked, target) {
        Ok(()) => Ok(Cleanliness::Clean),
        Err(SynchronizationError::WorktreeNotClean { .. }) => Ok(Cleanliness::Dirty),
        Err(SynchronizationError::WorktreeConflicted { .. }) => Ok(Cleanliness::Conflicted),
        Err(_) => Err(unavailable(root)),
    }
}

/// The item's authoring context as Git sees it, without discovery.
pub(in super::super) enum ContextPresence {
    /// No worktree registered at the canonical path.
    Missing,
    /// A worktree exists but is not the materialized context.
    Nonconforming,
    /// The materialized context; `oid` is absent when its branch vanished
    /// between materialization and the read, and `cleanliness` is only read
    /// when `oid` is present.
    Materialized {
        oid: Option<Oid>,
        cleanliness: Cleanliness,
    },
}
pub(in super::super) fn context_presence(
    kind: LifecycleKind,
    repository: &git2::Repository,
    root: &Path,
    config: &canonical::RepositoryConfig,
    item: &ItemId,
) -> Result<ContextPresence, RepositoryError> {
    let target = SynchronizationTarget::Context {
        kind: kind.authoring(),
        item_id: item.clone(),
    };
    match sync::materialized_target(root, &config.primary_branch, &target) {
        Ok(linked) => {
            let oid = reference_oid(repository, &kind.context_ref(item), root)?;
            let cleanliness = match oid {
                Some(_) => target_cleanliness(&linked, &target, root)?,
                None => Cleanliness::Clean,
            };
            Ok(ContextPresence::Materialized { oid, cleanliness })
        }
        Err(_) => Ok(
            if !root
                .join(".manyhands/worktrees")
                .join(item.to_string())
                .exists()
                || repository.find_worktree(&item.to_string()).is_err()
            {
                ContextPresence::Missing
            } else {
                ContextPresence::Nonconforming
            },
        ),
    }
}

/// The local primary branch tip and its checkout.
pub(in super::super) enum PrimaryPresence {
    Missing,
    NotCheckedOut(Oid),
    CheckedOut(Oid, Cleanliness),
}
impl PrimaryPresence {
    pub(in super::super) fn oid(&self) -> Option<Oid> {
        match self {
            Self::Missing => None,
            Self::NotCheckedOut(oid) | Self::CheckedOut(oid, _) => Some(*oid),
        }
    }
}
pub(in super::super) fn primary_presence(
    repository: &git2::Repository,
    root: &Path,
    config: &canonical::RepositoryConfig,
) -> Result<PrimaryPresence, RepositoryError> {
    let Some(oid) = reference_oid(
        repository,
        &format!("refs/heads/{}", config.primary_branch),
        root,
    )?
    else {
        return Ok(PrimaryPresence::Missing);
    };
    Ok(
        match sync::materialized_target(
            root,
            &config.primary_branch,
            &SynchronizationTarget::Primary,
        ) {
            Ok(linked) => PrimaryPresence::CheckedOut(
                oid,
                target_cleanliness(&linked, &SynchronizationTarget::Primary, root)?,
            ),
            Err(_) => PrimaryPresence::NotCheckedOut(oid),
        },
    )
}

/// D2: the remote-tracking facts both lifecycles bind.
pub(in super::super) struct RemoteFacts {
    pub(in super::super) tracking_primary: Option<Oid>,
    pub(in super::super) tracking_context: Option<Oid>,
    /// The local primary has commits the tracking primary lacks.
    pub(in super::super) primary_ahead: bool,
    /// The context lacks commits the tracking context has.
    pub(in super::super) context_behind: bool,
}
/// `None` without a publication remote.
pub(in super::super) fn remote_facts(
    kind: LifecycleKind,
    repository: &git2::Repository,
    root: &Path,
    config: &canonical::RepositoryConfig,
    item: &ItemId,
    primary_oid: Option<Oid>,
    context_oid: Option<Oid>,
) -> Result<Option<RemoteFacts>, RepositoryError> {
    let Some(remote) = &config.publication_remote else {
        return Ok(None);
    };
    let plan = RemoteRefPlan::from_configuration(remote, &config.primary_branch)
        .map_err(|_| unavailable(root))?;
    let tracking_primary = reference_oid(repository, plan.primary().tracking_ref(), root)?;
    let tracking_context = reference_oid(
        repository,
        plan.context(kind.authoring(), item).tracking_ref(),
        root,
    )?;
    let primary_ahead = match (primary_oid, tracking_primary) {
        (Some(primary), Some(tracking)) => !ancestor(repository, primary, tracking, root)?,
        _ => false,
    };
    let context_behind = match (context_oid, tracking_context) {
        (Some(context), Some(tracking)) => !ancestor(repository, tracking, context, root)?,
        _ => false,
    };
    Ok(Some(RemoteFacts {
        tracking_primary,
        tracking_context,
        primary_ahead,
        context_behind,
    }))
}

/// D5: whether an absent tracking context was once published.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum RemoteContextHistory {
    NeverPublished,
    Deleted,
    Unknown,
}
pub(in super::super) fn remote_context_history(
    snapshot: &RemoteSnapshot,
    kind: LifecycleKind,
    item: &ItemId,
) -> RemoteContextHistory {
    match snapshot.fetch_publication_evidence_for(kind.authoring(), item) {
        RemotePublicationEvidence::ObservedPublished => RemoteContextHistory::Deleted,
        RemotePublicationEvidence::HistoryUnknown => RemoteContextHistory::Unknown,
        RemotePublicationEvidence::NeverPublished => RemoteContextHistory::NeverPublished,
    }
}

/// The integration result tree, built in `worker`'s memory-backed ODB.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum PlannedResult {
    /// The context is contained in the base: the base tree.
    Contained(Oid),
    /// A merge commit is needed; its tree.
    Merge(Oid),
    /// The in-memory merge conflicts.
    Conflict,
}
pub(in super::super) fn planned_result(
    repository: &git2::Repository,
    worker: &git2::Repository,
    root: &Path,
    base: Oid,
    context: Oid,
) -> Result<PlannedResult, RepositoryError> {
    let tree = |oid: Oid| {
        worker
            .find_commit(oid)
            .map(|c| c.tree_id())
            .map_err(|_| unavailable(root))
    };
    if ancestor(repository, context, base, root)? {
        return Ok(PlannedResult::Contained(tree(base)?));
    }
    if ancestor(repository, base, context, root)? {
        return Ok(PlannedResult::Merge(tree(context)?));
    }
    match sync::prepare_clean_merge(root, base, context).map_err(|_| unavailable(root))? {
        sync::MergePreparation::Clean(prepared) => Ok(PlannedResult::Merge(
            sync::import_prepared_tree(worker, &prepared).map_err(|_| unavailable(root))?,
        )),
        sync::MergePreparation::Conflict => Ok(PlannedResult::Conflict),
    }
}

/// Observe actual Git afresh. The caller supplies a coherently read remote
/// snapshot and pending-conflict flag; the same function can run after sync or
/// while the caller already holds the integration lease.
fn observe_git(
    repository: &git2::Repository,
    root: &Path,
    config: &canonical::RepositoryConfig,
    item: &ItemId,
    snapshot: &RemoteSnapshot,
    pending: bool,
    facts: &DiscoveryFacts,
) -> Result<PromotionObservation, RepositoryError> {
    let mut observation = PromotionObservation {
        blockers: BTreeSet::new(),
        preview: None,
    };
    let kind = facts.kind;
    let blockers = &mut observation.blockers;
    let context_branch = kind.context_branch(item);
    let not_document = facts.not_document;
    let mut context_oid = None;
    let mut context_document_path = None;
    if not_document {
        blockers.insert(PromotionBlocker::NotADocument);
    } else {
        match context_presence(kind, repository, root, config, item)? {
            ContextPresence::Materialized { oid, cleanliness } => {
                context_oid = oid;
                if context_oid.is_none() {
                    // The branch can disappear between materialization and this
                    // unleased read; an incomplete observation is never ready.
                    blockers.insert(PromotionBlocker::ContextMissing);
                } else {
                    match cleanliness {
                        Cleanliness::Clean => {}
                        Cleanliness::Dirty => {
                            blockers.insert(PromotionBlocker::ContextDirty);
                        }
                        Cleanliness::Conflicted => {
                            blockers.insert(PromotionBlocker::ContextConflicted);
                        }
                    }
                }
                context_document_path = facts.document_path.clone();
                if context_document_path.is_none() {
                    blockers.insert(PromotionBlocker::ContextNonconforming);
                }
            }
            ContextPresence::Missing => {
                blockers.insert(PromotionBlocker::ContextMissing);
            }
            ContextPresence::Nonconforming => {
                blockers.insert(PromotionBlocker::ContextNonconforming);
            }
        }
    }
    if pending {
        blockers.insert(PromotionBlocker::ConflictPending);
    }
    let primary = primary_presence(repository, root, config)?;
    let primary_oid = primary.oid();
    match primary {
        PrimaryPresence::Missing => {
            blockers.insert(PromotionBlocker::PrimaryMissing);
        }
        PrimaryPresence::NotCheckedOut(_) => {
            blockers.insert(PromotionBlocker::PrimaryNotCheckedOut);
        }
        PrimaryPresence::CheckedOut(_, Cleanliness::Clean) => {}
        PrimaryPresence::CheckedOut(_, Cleanliness::Dirty) => {
            blockers.insert(PromotionBlocker::PrimaryDirty);
        }
        PrimaryPresence::CheckedOut(_, Cleanliness::Conflicted) => {
            blockers.insert(PromotionBlocker::PrimaryConflicted);
        }
    }
    let mut tracking_primary = None;
    let mut tracking_context = None;
    if let Some(remote) = remote_facts(
        kind,
        repository,
        root,
        config,
        item,
        primary_oid,
        context_oid,
    )? {
        tracking_primary = remote.tracking_primary;
        tracking_context = remote.tracking_context;
        if tracking_primary.is_none() {
            blockers.insert(PromotionBlocker::PrimaryMissing);
        }
        if remote.primary_ahead {
            blockers.insert(PromotionBlocker::PrimaryAheadOfRemote);
        }
        if remote.context_behind {
            blockers.insert(PromotionBlocker::ContextBehindRemote);
        }
    }
    let base = if config.publication_remote.is_some() {
        tracking_primary
    } else {
        primary_oid
    };
    let mut integration = None;
    let mut result = None;
    // A dedicated memory-backed handle allows the existing prepared-tree importer
    // and canonical tree reader to be reused without writing a single ODB object.
    let worker = git2::Repository::open(root).map_err(|_| unavailable(root))?;
    let odb = worker.odb().map_err(|_| unavailable(root))?;
    let _memory = odb
        .add_new_mempack_backend(1000)
        .map_err(|_| unavailable(root))?;
    if let (Some(base), Some(context), Some(_)) = (base, context_oid, &context_document_path) {
        match planned_result(repository, &worker, root, base, context)? {
            PlannedResult::Contained(tree) => {
                integration = Some(PlannedIntegration::AlreadyContained);
                result = Some(tree);
            }
            PlannedResult::Merge(tree) => {
                integration = Some(PlannedIntegration::MergeCommit);
                result = Some(tree);
            }
            PlannedResult::Conflict => {
                integration = Some(PlannedIntegration::MergeCommit);
                blockers.insert(PromotionBlocker::IntegrationConflict);
            }
        }
    }
    if config.publication_remote.is_some()
        && tracking_context.is_none()
        && integration != Some(PlannedIntegration::AlreadyContained)
        && !not_document
    {
        match remote_context_history(snapshot, kind, item) {
            RemoteContextHistory::Deleted => {
                blockers.insert(PromotionBlocker::RemoteContextDeleted);
            }
            RemoteContextHistory::Unknown => {
                blockers.insert(PromotionBlocker::HistoryUnknown);
            }
            RemoteContextHistory::NeverPublished => {}
        }
    }
    if let (Some(base), Some(result_oid)) = (base, result) {
        let result = worker
            .find_tree(result_oid)
            .map_err(|_| unavailable(root))?;
        let result_document_path =
            result_document_path(&worker, &result, item).map_err(|_| unavailable(root))?;
        if result_document_path.is_none() {
            blockers.insert(PromotionBlocker::DocumentInvalidAfterIntegration);
        }
        if blockers.is_empty() {
            let base_tree = worker
                .find_commit(base)
                .and_then(|c| c.tree())
                .map_err(|_| unavailable(root))?;
            let (changes, new_entries) = changes(&worker, &base_tree, &result, root)?;
            let integration = integration.expect("result has an integration kind");
            let cleanup = PlannedCleanup {
                worktree: true,
                local_branch: true,
                remote_branch: config.publication_remote.is_some()
                    && !(integration == PlannedIntegration::AlreadyContained
                        && tracking_context.is_none()),
            };
            let mut preview = DocumentPromotionPreview {
                item_id: item.clone(),
                document_path: context_document_path.expect("conforming context document"),
                context_branch,
                context_oid: context_oid.expect("materialized context"),
                primary_branch: config.primary_branch.clone(),
                primary_oid: primary_oid.expect("primary exists"),
                publication_remote: config.publication_remote.clone(),
                tracking_primary,
                tracking_context,
                changes,
                integration,
                publication: if config.publication_remote.is_some() {
                    PlannedPublication::Publish
                } else {
                    PlannedPublication::PendingNoRemote
                },
                cleanup,
                expectation: PromotionExpectation::from_bytes([0; 32]),
            };
            preview.expectation = expectation(&preview, &new_entries);
            observation.preview = Some(preview);
        }
    }
    Ok(observation)
}

fn result_document_path(
    repository: &git2::Repository,
    tree: &git2::Tree<'_>,
    item: &ItemId,
) -> Result<Option<PathBuf>, ()> {
    let mut forbidden = false;
    tree.walk(git2::TreeWalkMode::PreOrder, |parent, entry| {
        let mut path = parent.as_bytes().to_vec();
        path.extend_from_slice(entry.name_bytes());
        if path == b".manyhands/worktrees" || path.starts_with(b".manyhands/worktrees/") {
            forbidden = true;
        }
        git2::TreeWalkResult::Ok
    })
    .map_err(|_| ())?;
    if forbidden {
        return Ok(None);
    }
    let snapshot = RepositoryService::canonical_tree_sources(repository, tree).map_err(|_| ())?;
    let sources = snapshot
        .sources
        .iter()
        .filter_map(|(path, bytes)| {
            Some((
                PathBuf::from(std::str::from_utf8(path).ok()?),
                std::str::from_utf8(bytes).ok()?.to_owned(),
            ))
        })
        .collect::<Vec<_>>();
    let validation = canonical::validate_context(sources.clone());
    if !validation
        .items
        .iter()
        .any(|found| matches!(found, canonical::CanonicalItem::Document(doc) if doc.id == *item))
    {
        return Ok(None);
    }
    let paths = sources
        .iter()
        .filter_map(|(path, source)| match canonical::parse_item(path, source) {
            Ok(canonical::CanonicalItem::Document(doc)) if doc.id == *item => Some(path),
            _ => None,
        })
        .collect::<Vec<_>>();
    let [path] = paths.as_slice() else {
        return Ok(None);
    };
    if !matches!(
        snapshot.entries.get(path.as_os_str().as_encoded_bytes()),
        Some((_, 0o100644 | 0o100755, Some(git2::ObjectType::Blob)))
    ) {
        return Ok(None);
    }
    Ok(Some((*path).clone()))
}

/// Result-tree evidence retained only for consent encoding, not the public DTO.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) struct PromotionChangeEntry {
    pub(in super::super) oid: Option<Oid>,
    /// Git entry mode including type bits; zero means no resulting entry.
    pub(in super::super) mode: u32,
}

/// One change-set entry with the result-tree evidence the digest binds
/// (T2-1), shared by both lifecycles.
#[derive(Clone, PartialEq, Eq)]
pub(in super::super) struct ChangeEvidence {
    pub(in super::super) change: PromotionPathChange,
    pub(in super::super) entry: PromotionChangeEntry,
}

/// The change set from `base` to `result`, sorted by path, with evidence.
pub(in super::super) fn change_evidence(
    repository: &git2::Repository,
    base: &git2::Tree<'_>,
    result: &git2::Tree<'_>,
    root: &Path,
) -> Result<Vec<ChangeEvidence>, RepositoryError> {
    let (changes, entries) = changes(repository, base, result, root)?;
    Ok(changes
        .into_iter()
        .zip(entries)
        .map(|(change, entry)| ChangeEvidence { change, entry })
        .collect())
}

fn changes(
    repository: &git2::Repository,
    base: &git2::Tree<'_>,
    result: &git2::Tree<'_>,
    root: &Path,
) -> Result<(Vec<PromotionPathChange>, Vec<PromotionChangeEntry>), RepositoryError> {
    let mut options = git2::DiffOptions::new();
    options.include_typechange(true);
    // No find_similar call: moves are deliberately delete + add.
    let diff = repository
        .diff_tree_to_tree(Some(base), Some(result), Some(&mut options))
        .map_err(|_| unavailable(root))?;
    let mut entries = Vec::new();
    for delta in diff.deltas() {
        let (file, change) = match delta.status() {
            git2::Delta::Added => (delta.new_file(), PathChange::Added),
            git2::Delta::Deleted => (delta.old_file(), PathChange::Deleted),
            git2::Delta::Modified | git2::Delta::Typechange => {
                (delta.new_file(), PathChange::Modified)
            }
            _ => return Err(unavailable(root)),
        };
        // Read the result tree itself, including in-memory prepared merges.
        // Local filesystem permissions/types do not supply consent evidence.
        let result_entry = if change == PathChange::Deleted {
            None
        } else {
            Some(
                result
                    .get_path(file.path().ok_or_else(|| unavailable(root))?)
                    .map_err(|_| unavailable(root))?,
            )
        };
        entries.push((
            file.path_bytes().ok_or_else(|| unavailable(root))?.to_vec(),
            change,
            PromotionChangeEntry {
                oid: result_entry.as_ref().map(|entry| entry.id()),
                mode: result_entry
                    .as_ref()
                    .map_or(0, |entry| entry.filemode() as u32),
            },
        ));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let mut changes = Vec::new();
    let mut new_entries = Vec::new();
    for (path, change, entry) in entries {
        #[cfg(unix)]
        let path = {
            use std::os::unix::ffi::OsStringExt;
            PathBuf::from(std::ffi::OsString::from_vec(path))
        };
        #[cfg(not(unix))]
        let path = PathBuf::from(String::from_utf8(path).map_err(|_| unavailable(root))?);
        changes.push(PromotionPathChange { path, change });
        new_entries.push(entry);
    }
    Ok((changes, new_entries))
}

/// The digest encodings both lifecycles share: u64 big-endian lengths and
/// counts, single-byte tags, and fixed-width OIDs and modes.
pub(in super::super) mod digest {
    use super::*;

    pub(in super::super::super) fn field(hash: &mut blake3::Hasher, bytes: &[u8]) {
        hash.update(&(bytes.len() as u64).to_be_bytes());
        hash.update(bytes);
    }
    pub(in super::super::super) fn remote(hash: &mut blake3::Hasher, remote: Option<&str>) {
        match remote {
            Some(remote) => {
                hash.update(&[1]);
                field(hash, remote.as_bytes());
            }
            None => {
                hash.update(&[0]);
            }
        }
    }
    pub(in super::super::super) fn integration(
        hash: &mut blake3::Hasher,
        integration: PlannedIntegration,
    ) {
        hash.update(&[match integration {
            PlannedIntegration::MergeCommit => 0,
            PlannedIntegration::AlreadyContained => 1,
        }]);
    }
    pub(in super::super::super) fn changes(
        hash: &mut blake3::Hasher,
        changes: &[PromotionPathChange],
        new_entries: &[PromotionChangeEntry],
    ) {
        hash.update(&(changes.len() as u64).to_be_bytes());
        assert_eq!(changes.len(), new_entries.len());
        for (change, entry) in changes.iter().zip(new_entries) {
            field(hash, change.path.as_os_str().as_encoded_bytes());
            hash.update(&[match change.change {
                PathChange::Added => 0,
                PathChange::Modified => 1,
                PathChange::Deleted => 2,
            }]);
            match entry.oid {
                Some(oid) => {
                    hash.update(&[1]);
                    hash.update(oid.as_bytes());
                }
                None => {
                    hash.update(&[0]);
                }
            }
            // T2-1: fixed-width big-endian resulting Git mode, or zero for Deleted.
            hash.update(&entry.mode.to_be_bytes());
        }
    }
    pub(in super::super::super) fn cleanup(hash: &mut blake3::Hasher, cleanup: PlannedCleanup) {
        hash.update(&[
            u8::from(cleanup.remote_branch),
            u8::from(cleanup.worktree),
            u8::from(cleanup.local_branch),
        ]);
    }
    /// D6: moving OIDs are bound only without a publication remote.
    pub(in super::super::super) fn local_oids(
        hash: &mut blake3::Hasher,
        remote: Option<&str>,
        primary: Oid,
        context: Oid,
    ) {
        if remote.is_none() {
            hash.update(primary.as_bytes());
            hash.update(context.as_bytes());
        }
    }
}

fn expectation(
    preview: &DocumentPromotionPreview,
    new_entries: &[PromotionChangeEntry],
) -> PromotionExpectation {
    let mut hash = blake3::Hasher::new();
    hash.update(b"manyhands promotion v1");
    digest::field(&mut hash, preview.item_id.to_string().as_bytes());
    digest::field(
        &mut hash,
        preview.document_path.as_os_str().as_encoded_bytes(),
    );
    digest::field(&mut hash, preview.context_branch.as_bytes());
    digest::field(&mut hash, preview.primary_branch.as_bytes());
    digest::remote(&mut hash, preview.publication_remote.as_deref());
    digest::integration(&mut hash, preview.integration);
    digest::changes(&mut hash, &preview.changes, new_entries);
    digest::cleanup(&mut hash, preview.cleanup);
    digest::local_oids(
        &mut hash,
        preview.publication_remote.as_deref(),
        preview.primary_oid,
        preview.context_oid,
    );
    PromotionExpectation::from_bytes(*hash.finalize().as_bytes())
}

#[cfg(test)]
#[path = "observe_tests.rs"]
mod tests;
