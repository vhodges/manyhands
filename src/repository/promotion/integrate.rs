use super::*;
use crate::repository::*;
use crate::repository::{bridges::identity::LocalConfigEdit, remote::sync};

#[derive(Clone)]
pub(super) struct IntegrationContext {
    pub(super) item_id: ItemId,
    pub(super) primary_branch: String,
    pub(super) oid: Oid,
}

#[derive(Clone)]
pub(super) struct IntegrationRequest {
    pub(super) expected: PromotionExpectation,
    pub(super) context: IntegrationContext,
    pub(super) primary_oid: Oid,
    pub(super) publication_remote: Option<String>,
}
impl IntegrationRequest {
    pub(super) fn from_preview(preview: &DocumentPromotionPreview) -> Self {
        Self {
            expected: preview.expectation(),
            context: IntegrationContext {
                item_id: preview.item_id().clone(),
                primary_branch: preview.primary_branch().into(),
                oid: preview.context_oid(),
            },
            primary_oid: preview.primary_oid(),
            publication_remote: preview.publication_remote().map(str::to_owned),
        }
    }
}

pub(super) struct IntegrationEvidence {
    pub(super) base: Oid,
    pub(super) context: IntegrationContext,
    pub(super) old_primary: Oid,
    pub(super) kind: PlannedIntegration,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Integrated {
    Merged { oid: Oid },
    AlreadyContained { oid: Oid },
    RecoveryRequired,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum IntegrationState {
    Integrated { oid: Oid },
    NotIntegrated,
    RecoveryRequired,
}
enum IntegrationTree {
    Contained,
    Context(Oid),
    Prepared(sync::PreparedMerge),
}

/// Owns the single integration lease. The runner persists `evidence()` as
/// integrating BEFORE consuming this value with `write_commit`. No registry
/// guard/transaction escapes observation or is held across a caller checkpoint.
pub(super) struct PreparedIntegration {
    repository: git2::Repository,
    root: PathBuf,
    evidence: IntegrationEvidence,
    tree: IntegrationTree,
    signature: Option<git2::Signature<'static>>,
    _lease: coordination::RepositoryLease,
}

/// F3 boundary: a candidate object exists, but primary is still at old_primary.
/// Consuming this applies exactly one guarded old-local-primary -> merge update.
pub(super) struct IntegrationCandidate {
    prepared: PreparedIntegration,
    oid: Oid,
}

pub(super) fn integrate(
    service: &RepositoryService,
    root: &Path,
    record: &IntegrationRequest,
    identity: Option<&ConfirmedCommitIdentity>,
) -> Result<Integrated, DocumentPromotionError> {
    prepare_integration(service, root, record, identity)?
        .write_commit()?
        .apply()
}
pub(super) fn prepare_integration(
    service: &RepositoryService,
    root: &Path,
    record: &IntegrationRequest,
    identity: Option<&ConfirmedCommitIdentity>,
) -> Result<PreparedIntegration, DocumentPromotionError> {
    let (repository, root) = canonical_repository_root(root, RepositoryOperation::Read)?;
    let lease = repository_lease(&repository, &root, RepositoryOperation::Read)?;
    let primary_ref = format!("refs/heads/{}", record.context.primary_branch);
    let context_ref = format!("refs/heads/manyhands/document/{}", record.context.item_id);
    if repository.refname_to_id(&primary_ref).ok() != Some(record.primary_oid)
        || repository.refname_to_id(&context_ref).ok() != Some(record.context.oid)
    {
        return Err(DocumentPromotionError::ExternalChange);
    }
    // Reuses Task 2's coherent read-only state/actual-Git observation. It takes
    // no repository lease, and drops its cache guard before returning.
    let preview = match observe::preview(
        service,
        PreviewDocumentPromotionRequest {
            root: root.clone(),
            item_id: record.context.item_id.clone(),
        },
    )? {
        DocumentPromotionPreflight::Ready(preview) => preview,
        DocumentPromotionPreflight::Blocked { blockers } => {
            return Err(
                if blockers.contains(&PromotionBlocker::IntegrationConflict) {
                    DocumentPromotionError::ReconfirmationRequired
                } else {
                    DocumentPromotionError::Blocked(blockers)
                },
            );
        }
    };
    if preview.primary_branch() != record.context.primary_branch
        || preview.publication_remote() != record.publication_remote.as_deref()
        || preview.primary_oid() != record.primary_oid
        || preview.context_oid() != record.context.oid
    {
        return Err(DocumentPromotionError::ExternalChange);
    }
    if preview.expectation() != record.expected {
        return Err(DocumentPromotionError::ReconfirmationRequired);
    }
    let base = preview
        .tracking_primary()
        .filter(|_| record.publication_remote.is_some())
        .unwrap_or(preview.primary_oid());
    let evidence = IntegrationEvidence {
        base,
        context: record.context.clone(),
        old_primary: preview.primary_oid(),
        kind: preview.integration(),
    };
    let tree = if preview.integration() == PlannedIntegration::AlreadyContained {
        IntegrationTree::Contained
    } else {
        let base_in_context = ancestor(&repository, base, record.context.oid)?;
        if record.publication_remote.is_some() && !base_in_context {
            return Err(DocumentPromotionError::PrimaryAdvanced);
        }
        if base_in_context {
            IntegrationTree::Context(
                repository
                    .find_commit(record.context.oid)
                    .map_err(|_| unavailable())?
                    .tree_id(),
            )
        } else {
            match sync::prepare_clean_merge(&root, base, record.context.oid).map_err(sync_error)? {
                sync::MergePreparation::Clean(prepared) => IntegrationTree::Prepared(prepared),
                sync::MergePreparation::Conflict => {
                    return Err(DocumentPromotionError::ReconfirmationRequired);
                }
            }
        }
    };
    let signature = if matches!(tree, IntegrationTree::Contained) {
        None
    } else {
        // Check ignored-path collisions through a memory ODB before creating
        // any object. The guarded safe checkout remains the final authority.
        checkout_preflight(&root, &tree)?;
        let identity = promotion_identity(service, &repository, &root, identity)?;
        Some(git2::Signature::now(&identity.name, &identity.email).map_err(|_| unavailable())?)
    };
    Ok(PreparedIntegration {
        repository,
        root,
        evidence,
        tree,
        signature,
        _lease: lease,
    })
}
impl PreparedIntegration {
    pub(super) fn evidence(&self) -> &IntegrationEvidence {
        &self.evidence
    }
    pub(super) fn write_commit(self) -> Result<IntegrationCandidate, DocumentPromotionError> {
        let oid = match &self.tree {
            IntegrationTree::Contained => self.evidence.base,
            tree => {
                let tree = self
                    .repository
                    .find_tree(import_tree(&self.repository, tree)?)
                    .map_err(|_| unavailable())?;
                let base = self
                    .repository
                    .find_commit(self.evidence.base)
                    .map_err(|_| unavailable())?;
                let context = self
                    .repository
                    .find_commit(self.evidence.context.oid)
                    .map_err(|_| unavailable())?;
                let signature = self
                    .signature
                    .as_ref()
                    .expect("merge identity was resolved");
                self.repository
                    .commit(
                        None,
                        signature,
                        signature,
                        &format!("Promote document {}", self.evidence.context.item_id),
                        &tree,
                        &[&base, &context],
                    )
                    .map_err(|_| unavailable())?
            }
        };
        Ok(IntegrationCandidate {
            prepared: self,
            oid,
        })
    }
}
impl IntegrationCandidate {
    pub(super) fn apply(self) -> Result<Integrated, DocumentPromotionError> {
        let prepared = self.prepared;
        if matches!(prepared.tree, IntegrationTree::Contained) {
            return Ok(Integrated::AlreadyContained {
                oid: prepared.evidence.base,
            });
        }
        let branch = format!("refs/heads/{}", prepared.evidence.context.primary_branch);
        if let Err(error) = sync::fast_forward(
            &prepared.repository,
            &branch,
            prepared.evidence.old_primary,
            self.oid,
        ) {
            // Even an error from the ref transaction can follow an effect. Git,
            // not the operation error, determines the integration result.
            return match reconcile_integration(
                &prepared.repository,
                prepared.evidence.base,
                &prepared.evidence.context,
            ) {
                IntegrationState::Integrated { oid } => Ok(Integrated::Merged { oid }),
                IntegrationState::RecoveryRequired => Ok(Integrated::RecoveryRequired),
                IntegrationState::NotIntegrated => {
                    // A failed transaction may have checked out the candidate
                    // while leaving the ref old. Do not disguise that as a
                    // pre-integration refusal or automatically roll it back.
                    let clean = primary_clean(&prepared.root);
                    if prepared.repository.refname_to_id(&branch).ok()
                        == Some(prepared.evidence.old_primary)
                        && clean
                    {
                        Err(sync_error(error))
                    } else {
                        Ok(Integrated::RecoveryRequired)
                    }
                }
            };
        }
        Ok(Integrated::Merged { oid: self.oid })
    }
}
pub(super) fn reconcile_integration(
    repository: &git2::Repository,
    base: Oid,
    context: &IntegrationContext,
) -> IntegrationState {
    let result = || -> Option<IntegrationState> {
        let tip = repository
            .refname_to_id(&format!("refs/heads/{}", context.primary_branch))
            .ok()?;
        let subject = format!("Promote document {}", context.item_id);
        let mut current = tip;
        while current != base {
            let commit = repository.find_commit(current).ok()?;
            if commit.summary() == Some(subject.as_str())
                && commit.parent_count() == 2
                && commit.parent_id(0).ok()? == base
                && commit.parent_id(1).ok()? == context.oid
            {
                return Some(IntegrationState::Integrated { oid: current });
            }
            if commit.parent_count() == 0 {
                break;
            }
            current = commit.parent_id(0).ok()?;
        }
        Some(
            if tip == base || repository.graph_descendant_of(base, tip).ok()? {
                IntegrationState::NotIntegrated
            } else {
                IntegrationState::RecoveryRequired
            },
        )
    };
    result().unwrap_or(IntegrationState::RecoveryRequired)
}

fn unavailable() -> DocumentPromotionError {
    DocumentPromotionError::Repository(RepositoryError::new(
        RepositoryOperation::RepositorySnapshot,
        None,
        RepositoryErrorKind::RecoveryRequired,
        "promotion integration unavailable",
    ))
}
fn sync_error(error: SynchronizationError) -> DocumentPromotionError {
    match error {
        SynchronizationError::Repository(error) => repository_error(error),
        SynchronizationError::ExternalChange => DocumentPromotionError::ExternalChange,
        SynchronizationError::IdentityRequired {
            expected_configuration,
            ..
        } => DocumentPromotionError::IdentityRequired {
            expected_configuration,
        },
        _ => unavailable(),
    }
}
fn repository_error(error: RepositoryError) -> DocumentPromotionError {
    if error.kind == RepositoryErrorKind::ExternalChange {
        DocumentPromotionError::ExternalChange
    } else {
        error.into()
    }
}
fn ancestor(
    repository: &git2::Repository,
    older: Oid,
    newer: Oid,
) -> Result<bool, DocumentPromotionError> {
    if older == newer {
        return Ok(true);
    }
    repository
        .graph_descendant_of(newer, older)
        .map_err(|_| unavailable())
}
fn import_tree(
    repository: &git2::Repository,
    tree: &IntegrationTree,
) -> Result<Oid, DocumentPromotionError> {
    match tree {
        IntegrationTree::Context(oid) => Ok(*oid),
        IntegrationTree::Prepared(prepared) => {
            sync::import_prepared_tree(repository, prepared).map_err(sync_error)
        }
        IntegrationTree::Contained => unreachable!("contained needs no import"),
    }
}
fn checkout_preflight(root: &Path, tree: &IntegrationTree) -> Result<(), DocumentPromotionError> {
    let worker = git2::Repository::open(root).map_err(|_| unavailable())?;
    let odb = worker.odb().map_err(|_| unavailable())?;
    let _memory = odb
        .add_new_mempack_backend(1000)
        .map_err(|_| unavailable())?;
    let tree = worker
        .find_tree(import_tree(&worker, tree)?)
        .map_err(|_| unavailable())?;
    // git2 0.20's dry_run selects CHECKOUT_NONE, not libgit2's DRY_RUN:
    // NONE skips the conflict action for ignored files. Inspect ignored paths
    // explicitly instead. Primary cleanliness already excludes other collisions.
    let mut options = git2::StatusOptions::new();
    options
        .include_ignored(true)
        .include_untracked(false)
        .recurse_ignored_dirs(true);
    let statuses = worker
        .statuses(Some(&mut options))
        .map_err(|_| unavailable())?;
    for entry in statuses
        .iter()
        .filter(|entry| entry.status().contains(git2::Status::IGNORED))
    {
        let path = entry.path().ok_or_else(unavailable)?;
        // A new file can also replace an ancestor of an ignored file. This
        // catches directory/file and symlink collisions without following them.
        for path in Path::new(path.trim_end_matches('/')).ancestors() {
            if let Ok(entry) = tree.get_path(path)
                && entry.kind() != Some(git2::ObjectType::Tree)
            {
                return Err(unavailable());
            }
        }
    }
    Ok(())
}
fn primary_clean(root: &Path) -> bool {
    let check = || -> Result<(), SynchronizationError> {
        let repository =
            git2::Repository::open(root).map_err(|_| SynchronizationError::RecoveryRequired)?;
        let odb = repository
            .odb()
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let _memory = odb
            .add_new_mempack_backend(1000)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        sync::require_clean_target(&repository, &SynchronizationTarget::Primary)
    };
    check().is_ok()
}
fn promotion_identity(
    service: &RepositoryService,
    repository: &git2::Repository,
    root: &Path,
    confirmation: Option<&ConfirmedCommitIdentity>,
) -> Result<CommitIdentity, DocumentPromotionError> {
    let target = SynchronizationTarget::Primary;
    if let sync::CommittingIdentity::Effective(identity) =
        sync::select_committing_identity(repository, confirmation, &target).map_err(sync_error)?
    {
        return Ok(identity);
    }
    let mut edit = LocalConfigEdit::begin(repository, root).map_err(repository_error)?;
    let select = || {
        let fresh = git2::Repository::open(root).map_err(|_| unavailable())?;
        sync::select_committing_identity(&fresh, confirmation, &target).map_err(sync_error)
    };
    let identity = match select()? {
        sync::CommittingIdentity::Effective(identity) => return Ok(identity),
        sync::CommittingIdentity::Confirmed(identity) => identity,
    };
    edit.config()
        .set_str("user.name", &identity.name)
        .map_err(|_| unavailable())?;
    service
        .identity_name_checkpoint(root)
        .map_err(repository_error)?;
    edit.config()
        .set_str("user.email", &identity.email)
        .map_err(|_| unavailable())?;
    // Recheck effective identity and exact consent while holding both locks,
    // immediately before installing. No bridge-specific identity policy is used.
    if let sync::CommittingIdentity::Effective(identity) = select()? {
        return Ok(identity);
    }
    let fresh = git2::Repository::open(root).map_err(|_| unavailable())?;
    if !edit
        .would_persist_identity(&fresh, &identity.name, &identity.email)
        .map_err(repository_error)?
    {
        return Err(unavailable());
    }
    edit.install().map_err(repository_error)?;
    Ok(identity)
}

#[cfg(test)]
#[path = "integrate_tests.rs"]
mod tests;
