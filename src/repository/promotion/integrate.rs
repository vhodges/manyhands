use super::lifecycle::{
    DocumentPromotionLifecycle, LeasedIntegration, Lifecycle, LifecycleError, LifecycleKind,
    LifecyclePreview, LifecycleRecord, LifecycleRefusal, PreIntegration,
};
use super::*;
use crate::repository::*;
use crate::repository::{bridges::identity::LocalConfigEdit, remote::sync};

#[derive(Clone)]
pub(in super::super) struct IntegrationContext {
    pub(in super::super) item_id: ItemId,
    pub(in super::super) primary_branch: String,
    pub(in super::super) oid: Oid,
}

/// What integration is bound to: the recorded consent, the exact
/// synchronized context, and the primary and remote it was consented on.
#[derive(Clone)]
pub(in super::super) struct IntegrationRequest<E = PromotionExpectation> {
    pub(in super::super) expected: E,
    pub(in super::super) context: IntegrationContext,
    pub(in super::super) primary_oid: Oid,
    pub(in super::super) publication_remote: Option<String>,
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
pub(in super::super) enum Integrated {
    Merged { oid: Oid },
    AlreadyContained { oid: Oid },
    RecoveryRequired,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum IntegrationState {
    Integrated { oid: Oid },
    NotIntegrated,
    RecoveryRequired,
}
enum IntegrationTree {
    Contained,
    /// A tree already in the repository ODB: the context's own tree, or the
    /// result a lifecycle hook prepared.
    Context(Oid),
    Prepared(sync::PreparedMerge),
}

/// Owns the single integration lease. The runner persists `evidence()` as
/// integrating BEFORE consuming this value with `write_commit`. No registry
/// guard/transaction escapes observation or is held across a caller checkpoint.
pub(in super::super) struct PreparedIntegration {
    kind: LifecycleKind,
    repository: git2::Repository,
    root: PathBuf,
    evidence: IntegrationEvidence,
    tree: IntegrationTree,
    signature: Option<git2::Signature<'static>>,
    /// The lifecycle hook wrote under this lease (a closure checkpoint): the
    /// runner's record snapshot is stale.
    checkpointed: bool,
    _lease: coordination::RepositoryLease,
}

/// F3 boundary: a candidate object exists, but primary is still at old_primary.
/// Consuming this applies exactly one guarded old-local-primary -> merge update.
pub(in super::super) struct IntegrationCandidate {
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
    DocumentPromotionLifecycle.prepare_integration(service, root, record, identity, None)
}
/// The one integration path for every lifecycle (C3): observation prepared
/// outside the lease, then, under it, a written effect resumed (C9) or the
/// observation revalidated, the hook, and the result tree and signer.
pub(in super::super) fn prepare_integration_for<L: Lifecycle>(
    lifecycle: &L,
    service: &RepositoryService,
    root: &Path,
    record: &IntegrationRequest<L::Expectation>,
    identity: Option<&ConfirmedCommitIdentity>,
    parent: Option<&L::Record>,
) -> Result<PreparedIntegration, L::Error> {
    let (repository, root) = canonical_repository_root(root, RepositoryOperation::Read)?;
    let observation = L::prepare(
        service,
        L::preview_request(root.clone(), record.context.item_id.clone(), identity),
    )?;
    let lease = repository_lease(&repository, &root, RepositoryOperation::Read)?;
    let primary_ref = format!("refs/heads/{}", record.context.primary_branch);
    let context_ref = L::KIND.context_ref(&record.context.item_id);
    if let Some(parent) = parent
        && let Some(old_primary) =
            written_primary::<L::Error>(&repository, &primary_ref, parent.effect_written())?
        && let Some(checkpoint) = lifecycle.resume_written(
            service,
            &mut LeasedIntegration {
                repository: &repository,
                root: &root,
                context: &record.context,
                base: old_primary,
                operation_id: Some(parent.operation_id()),
                record: Some(parent),
            },
        )?
    {
        // Consent is not re-checked: the written checkpoint is the consented
        // effect (C9). Only the primary's own safety checks remain.
        return prepare_checkpoint_integration(
            L::KIND,
            repository,
            root,
            lease,
            record.context.clone(),
            old_primary,
            checkpoint,
        );
    }
    if repository.refname_to_id(&primary_ref).ok() != Some(record.primary_oid)
        || repository.refname_to_id(&context_ref).ok() != Some(record.context.oid)
    {
        return Err(L::Error::external_change());
    }
    // Revalidate prepared discovery facts against current bounded Git/checkout/
    // configuration checks. Canonical result validation reads Git trees only.
    let preflight = L::revalidate(&observation, service, &repository).map_err(|e| {
        if e.kind == RepositoryErrorKind::ExternalChange {
            L::Error::external_change()
        } else {
            e.into()
        }
    })?;
    let preview = match L::ready(preflight) {
        Ok(preview) => preview,
        Err(refusal) => {
            return Err(if refusal.integration_conflict() {
                L::Error::reconfirmation_required()
            } else {
                L::refusal_error(&refusal)
            });
        }
    };
    if preview.primary_branch() != record.context.primary_branch
        || preview.publication_remote() != record.publication_remote.as_deref()
        || preview.primary_oid() != record.primary_oid
        || preview.context_oid() != record.context.oid
    {
        return Err(L::Error::external_change());
    }
    if preview.expectation() != record.expected {
        return Err(L::Error::reconfirmation_required());
    }
    let base = preview
        .tracking_primary()
        .filter(|_| record.publication_remote.is_some())
        .unwrap_or(preview.primary_oid());
    match lifecycle.pre_integration_with(
        service,
        &preview,
        identity,
        &mut LeasedIntegration {
            repository: &repository,
            root: &root,
            context: &record.context,
            base,
            operation_id: parent.map(LifecycleRecord::operation_id),
            record: parent,
        },
    )? {
        PreIntegration::None => (),
        checkpoint @ PreIntegration::Checkpoint { base: written, .. } => {
            if written != base {
                return Err(unavailable());
            }
            return prepare_checkpoint_integration(
                L::KIND,
                repository,
                root,
                lease,
                record.context.clone(),
                preview.primary_oid(),
                checkpoint,
            );
        }
    }
    let evidence = IntegrationEvidence {
        base,
        context: record.context.clone(),
        old_primary: preview.primary_oid(),
        kind: preview.integration(),
    };
    let tree = if preview.integration() == PlannedIntegration::AlreadyContained {
        IntegrationTree::Contained
    } else {
        let base_in_context = ancestor::<L::Error>(&repository, base, record.context.oid)?;
        if record.publication_remote.is_some() && !base_in_context {
            return Err(L::Error::primary_advanced());
        }
        if base_in_context {
            IntegrationTree::Context(
                repository
                    .find_commit(record.context.oid)
                    .map_err(|_| unavailable::<L::Error>())?
                    .tree_id(),
            )
        } else {
            match sync::prepare_clean_merge(&root, base, record.context.oid)
                .map_err(sync_error::<L::Error>)?
            {
                sync::MergePreparation::Clean(prepared) => IntegrationTree::Prepared(prepared),
                sync::MergePreparation::Conflict => {
                    return Err(L::Error::reconfirmation_required());
                }
            }
        }
    };
    let signature = if matches!(tree, IntegrationTree::Contained) {
        None
    } else {
        // Check ignored-path collisions through a memory ODB before creating
        // any object. The guarded safe checkout remains the final authority.
        checkout_preflight::<L::Error>(&root, &tree)?;
        let identity = promotion_identity::<L::Error>(service, &repository, &root, identity)?;
        Some(
            git2::Signature::now(&identity.name, &identity.email)
                .map_err(|_| unavailable::<L::Error>())?,
        )
    };
    Ok(PreparedIntegration {
        kind: L::KIND,
        repository,
        root,
        evidence,
        tree,
        signature,
        checkpointed: false,
        _lease: lease,
    })
}
/// The local primary under the lease, for `resume_written`. Unreadable, it is
/// `None` for a record with nothing written (revalidation reports the change),
/// but `RecoveryRequired` once an effect may be written: a written checkpoint
/// must never fall through to an abandonable consent refusal.
pub(in super::super) fn written_primary<E: LifecycleError>(
    repository: &git2::Repository,
    primary_ref: &str,
    written: bool,
) -> Result<Option<Oid>, E> {
    match repository.refname_to_id(primary_ref) {
        Ok(oid) => Ok(Some(oid)),
        Err(_) if written => Err(E::recovery_required()),
        Err(_) => Ok(None),
    }
}
/// Integration of a lifecycle hook's checkpoint under the lease already
/// held: the merge of `[base, merged context]` with the hook's result tree
/// and signer. `old_primary` is the local primary the guarded update moves.
pub(in super::super) fn prepare_checkpoint_integration<E: LifecycleError>(
    kind: LifecycleKind,
    repository: git2::Repository,
    root: PathBuf,
    lease: coordination::RepositoryLease,
    context: IntegrationContext,
    old_primary: Oid,
    checkpoint: PreIntegration,
) -> Result<PreparedIntegration, E> {
    let PreIntegration::Checkpoint {
        base,
        merged_context,
        result_tree,
        signer,
    } = checkpoint
    else {
        return Err(unavailable::<E>());
    };
    let tree = IntegrationTree::Context(result_tree);
    checkout_preflight::<E>(&root, &tree)?;
    let signature =
        git2::Signature::now(&signer.name, &signer.email).map_err(|_| unavailable::<E>())?;
    Ok(PreparedIntegration {
        kind,
        repository,
        root,
        evidence: IntegrationEvidence {
            base,
            context: IntegrationContext {
                oid: merged_context,
                ..context
            },
            old_primary,
            kind: PlannedIntegration::MergeCommit,
        },
        tree,
        signature: Some(signature),
        checkpointed: true,
        _lease: lease,
    })
}
impl PreparedIntegration {
    pub(super) fn evidence(&self) -> &IntegrationEvidence {
        &self.evidence
    }
    /// The hook wrote under this lease: reload the record before saving.
    pub(super) fn checkpointed(&self) -> bool {
        self.checkpointed
    }
    pub(super) fn write_commit(self) -> Result<IntegrationCandidate, DocumentPromotionError> {
        self.write_commit_for()
    }
    pub(in super::super) fn write_commit_for<E: LifecycleError>(
        self,
    ) -> Result<IntegrationCandidate, E> {
        let oid = match &self.tree {
            IntegrationTree::Contained => self.evidence.base,
            tree => {
                let tree = self
                    .repository
                    .find_tree(import_tree::<E>(&self.repository, tree)?)
                    .map_err(|_| unavailable::<E>())?;
                let base = self
                    .repository
                    .find_commit(self.evidence.base)
                    .map_err(|_| unavailable::<E>())?;
                let context = self
                    .repository
                    .find_commit(self.evidence.context.oid)
                    .map_err(|_| unavailable::<E>())?;
                let signature = self
                    .signature
                    .as_ref()
                    .expect("merge identity was resolved");
                self.repository
                    .commit(
                        None,
                        signature,
                        signature,
                        &self.kind.merge_subject(&self.evidence.context.item_id),
                        &tree,
                        &[&base, &context],
                    )
                    .map_err(|_| unavailable::<E>())?
            }
        };
        Ok(IntegrationCandidate {
            prepared: self,
            oid,
        })
    }
    /// Reuse the recorded candidate after an F3 interruption, rather than mint
    /// another promotion commit with a different timestamp/signature.
    pub(super) fn resume_candidate(
        self,
        oid: Oid,
    ) -> Result<IntegrationCandidate, DocumentPromotionError> {
        self.resume_candidate_for(oid)
    }
    pub(in super::super) fn resume_candidate_for<E: LifecycleError>(
        self,
        oid: Oid,
    ) -> Result<IntegrationCandidate, E> {
        let valid = match &self.tree {
            IntegrationTree::Contained => oid == self.evidence.base,
            tree => {
                let commit = self
                    .repository
                    .find_commit(oid)
                    .map_err(|_| unavailable::<E>())?;
                commit.parent_count() == 2
                    && commit.parent_id(0).ok() == Some(self.evidence.base)
                    && commit.parent_id(1).ok() == Some(self.evidence.context.oid)
                    && commit.summary()
                        == Some(
                            self.kind
                                .merge_subject(&self.evidence.context.item_id)
                                .as_str(),
                        )
                    && commit.tree_id() == import_tree::<E>(&self.repository, tree)?
            }
        };
        if !valid {
            return Err(unavailable::<E>());
        }
        Ok(IntegrationCandidate {
            prepared: self,
            oid,
        })
    }
}
impl IntegrationCandidate {
    pub(super) fn oid(&self) -> Oid {
        self.oid
    }
    pub(super) fn apply(self) -> Result<Integrated, DocumentPromotionError> {
        self.apply_for()
    }
    pub(in super::super) fn apply_for<E: LifecycleError>(self) -> Result<Integrated, E> {
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
            return match reconcile_integration_for(
                prepared.kind,
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
    DocumentPromotionLifecycle::reconcile_integration(repository, base, context)
}
/// First-parent search for the lifecycle's merge subject with parents
/// `[base, context]`; the subject is the reconciliation key.
pub(super) fn reconcile_integration_for(
    kind: LifecycleKind,
    repository: &git2::Repository,
    base: Oid,
    context: &IntegrationContext,
) -> IntegrationState {
    let result = || -> Option<IntegrationState> {
        let tip = repository
            .refname_to_id(&format!("refs/heads/{}", context.primary_branch))
            .ok()?;
        let subject = kind.merge_subject(&context.item_id);
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

fn unavailable<E: LifecycleError>() -> E {
    E::from(RepositoryError::new(
        RepositoryOperation::RepositorySnapshot,
        None,
        RepositoryErrorKind::RecoveryRequired,
        "promotion integration unavailable",
    ))
}
fn sync_error<E: LifecycleError>(error: SynchronizationError) -> E {
    match error {
        SynchronizationError::Repository(error) => repository_error(error),
        SynchronizationError::ExternalChange => E::external_change(),
        SynchronizationError::IdentityRequired {
            expected_configuration,
            ..
        } => E::identity_required(expected_configuration),
        _ => unavailable(),
    }
}
fn repository_error<E: LifecycleError>(error: RepositoryError) -> E {
    if error.kind == RepositoryErrorKind::ExternalChange {
        E::external_change()
    } else {
        error.into()
    }
}
fn ancestor<E: LifecycleError>(
    repository: &git2::Repository,
    older: Oid,
    newer: Oid,
) -> Result<bool, E> {
    if older == newer {
        return Ok(true);
    }
    repository
        .graph_descendant_of(newer, older)
        .map_err(|_| unavailable::<E>())
}
fn import_tree<E: LifecycleError>(
    repository: &git2::Repository,
    tree: &IntegrationTree,
) -> Result<Oid, E> {
    match tree {
        IntegrationTree::Context(oid) => Ok(*oid),
        IntegrationTree::Prepared(prepared) => {
            sync::import_prepared_tree(repository, prepared).map_err(sync_error::<E>)
        }
        IntegrationTree::Contained => unreachable!("contained needs no import"),
    }
}
fn checkout_preflight<E: LifecycleError>(root: &Path, tree: &IntegrationTree) -> Result<(), E> {
    let worker = git2::Repository::open(root).map_err(|_| unavailable::<E>())?;
    let odb = worker.odb().map_err(|_| unavailable::<E>())?;
    let _memory = odb
        .add_new_mempack_backend(1000)
        .map_err(|_| unavailable::<E>())?;
    let tree = worker
        .find_tree(import_tree::<E>(&worker, tree)?)
        .map_err(|_| unavailable::<E>())?;
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
        .map_err(|_| unavailable::<E>())?;
    for entry in statuses
        .iter()
        .filter(|entry| entry.status().contains(git2::Status::IGNORED))
    {
        let path = entry.path().ok_or_else(unavailable::<E>)?;
        // A new file can also replace an ancestor of an ignored file. This
        // catches directory/file and symlink collisions without following them.
        for path in Path::new(path.trim_end_matches('/')).ancestors() {
            if let Ok(entry) = tree.get_path(path)
                && entry.kind() != Some(git2::ObjectType::Tree)
            {
                return Err(unavailable::<E>());
            }
        }
    }
    Ok(())
}
pub(super) fn primary_clean(root: &Path) -> bool {
    let check = || -> Result<(), ()> {
        let repository = git2::Repository::open(root).map_err(|_| ())?;
        let odb = repository.odb().map_err(|_| ())?;
        let _memory = odb.add_new_mempack_backend(1000).map_err(|_| ())?;
        sync::require_clean_target(&repository, &SynchronizationTarget::Primary).map_err(|_| ())
    };
    check().is_ok()
}
/// The committing identity under the lease: effective wins; else the
/// configuration-bound confirmation, persisted to repository-local config
/// before any commit (T3-1), include-aware (Q3-1). Closure reuses it.
pub(in super::super) fn promotion_identity<E: LifecycleError>(
    service: &RepositoryService,
    repository: &git2::Repository,
    root: &Path,
    confirmation: Option<&ConfirmedCommitIdentity>,
) -> Result<CommitIdentity, E> {
    let target = SynchronizationTarget::Primary;
    if let sync::CommittingIdentity::Effective(identity) =
        sync::select_committing_identity(repository, confirmation, &target)
            .map_err(sync_error::<E>)?
    {
        return Ok(identity);
    }
    let mut edit = LocalConfigEdit::begin(repository, root).map_err(repository_error::<E>)?;
    let select = || {
        let fresh = git2::Repository::open(root).map_err(|_| unavailable::<E>())?;
        sync::select_committing_identity(&fresh, confirmation, &target).map_err(sync_error::<E>)
    };
    let identity = match select()? {
        sync::CommittingIdentity::Effective(identity) => return Ok(identity),
        sync::CommittingIdentity::Confirmed(identity) => identity,
    };
    edit.config()
        .set_str("user.name", &identity.name)
        .map_err(|_| unavailable::<E>())?;
    service
        .identity_name_checkpoint(root)
        .map_err(repository_error::<E>)?;
    edit.config()
        .set_str("user.email", &identity.email)
        .map_err(|_| unavailable::<E>())?;
    // Recheck effective identity and exact consent while holding both locks,
    // immediately before installing. No bridge-specific identity policy is used.
    if let sync::CommittingIdentity::Effective(identity) = select()? {
        return Ok(identity);
    }
    let fresh = git2::Repository::open(root).map_err(|_| unavailable::<E>())?;
    if !edit
        .would_persist_identity(&fresh, &identity.name, &identity.email)
        .map_err(repository_error::<E>)?
    {
        return Err(unavailable::<E>());
    }
    edit.install().map_err(repository_error::<E>)?;
    Ok(identity)
}

#[cfg(test)]
#[path = "integrate_tests.rs"]
mod tests;
