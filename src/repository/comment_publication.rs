//! Local-first comment publication. Git owns content; this module owns correlation.
use super::*;
use keys::{SessionCredentialProvider, SessionCredentials};
use transport::HostApproval;

#[derive(Clone)]
pub struct PublishCommentRequest {
    pub comment: SubmitCommentRequest,
    pub approval: Option<HostApproval>,
    pub confirmed_identity: Option<ConfirmedCommitIdentity>,
}
impl From<SubmitCommentRequest> for PublishCommentRequest {
    fn from(comment: SubmitCommentRequest) -> Self {
        Self {
            comment,
            approval: None,
            confirmed_identity: None,
        }
    }
}
impl fmt::Debug for PublishCommentRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PublishCommentRequest(<redacted>)")
    }
}

#[derive(Clone)]
pub struct RetryCommentPublicationRequest {
    pub root: PathBuf,
    pub operation_id: OperationId,
    pub approval: Option<HostApproval>,
    pub confirmed_identity: Option<ConfirmedCommitIdentity>,
    pub restart: bool,
}
impl fmt::Debug for RetryCommentPublicationRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RetryCommentPublicationRequest")
            .field("operation_id", &self.operation_id)
            .field("restart", &self.restart)
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub struct CommentReceipt {
    pub root: PathBuf,
    pub kind: AuthoringKind,
    pub item_id: canonical::ItemId,
    pub comment_id: canonical::ItemId,
    pub parent_id: Option<canonical::ItemId>,
    pub operation_id: OperationId,
    pub synchronization_id: OperationId,
    pub comment_path: PathBuf,
    pub context_branch: String,
    pub checkpoint_oid: git2::Oid,
}
impl fmt::Debug for CommentReceipt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommentReceipt")
            .field("operation_id", &self.operation_id)
            .field("synchronization_id", &self.synchronization_id)
            .field("comment_id", &self.comment_id)
            .field("checkpoint_oid", &self.checkpoint_oid)
            .finish_non_exhaustive()
    }
}

pub enum CommentPublicationPendingReason {
    NoPublicationRemote,
    LocalRecoveryRequired,
    Synchronization(Box<SynchronizationError>),
}
impl fmt::Debug for CommentPublicationPendingReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoPublicationRemote => f.write_str("NoPublicationRemote"),
            Self::LocalRecoveryRequired => f.write_str("LocalRecoveryRequired"),
            Self::Synchronization(error) => f
                .debug_tuple("Synchronization")
                .field(&error.to_string())
                .finish(),
        }
    }
}
#[derive(Debug)]
pub enum CommentPublicationState {
    Published {
        oid: git2::Oid,
    },
    AlreadyCurrent {
        oid: git2::Oid,
    },
    Pending {
        reason: CommentPublicationPendingReason,
    },
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CommentIndexingState {
    pub local_pending: bool,
    pub remote_pending: bool,
}
pub enum CommentSubmissionOutcome {
    IdentityRequired {
        context: ItemContext,
    },
    Saved {
        receipt: Box<CommentReceipt>,
        publication: CommentPublicationState,
        indexing: CommentIndexingState,
        /// Local context and this call's checkpoint effect, not publication authority.
        context: ItemContext,
        checkpoint: LocalCheckpoint,
    },
}

/// Preserves the pre-checkpoint category while redacting backend diagnostics.
pub struct CommentSubmissionError {
    pub kind: RepositoryErrorKind,
    pub operation_id: Option<OperationId>,
    error: Box<RepositoryError>,
}
impl CommentSubmissionError {
    pub fn external_change(&self) -> Option<&ExternalChangeDiagnostic> {
        self.error.external_change()
    }
}
impl From<RepositoryError> for CommentSubmissionError {
    fn from(error: RepositoryError) -> Self {
        Self {
            kind: error.kind,
            operation_id: None,
            error: Box::new(error),
        }
    }
}
impl fmt::Debug for CommentSubmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommentSubmissionError")
            .field("kind", &self.kind)
            .field("operation_id", &self.operation_id)
            .finish()
    }
}
impl fmt::Display for CommentSubmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "comment submission requires recovery ({:?})", self.kind)
    }
}
impl std::error::Error for CommentSubmissionError {}

pub(super) fn recovery_error() -> RepositoryError {
    RepositoryError::new(
        RepositoryOperation::SubmitComment,
        None,
        RepositoryErrorKind::RecoveryRequired,
        "original comment checkpoint requires reconciliation",
    )
}

pub(super) fn pending_synchronization(error: SynchronizationError) -> CommentPublicationState {
    let error = match error {
        SynchronizationError::Repository(error) => {
            SynchronizationError::Repository(RepositoryError::new(
                RepositoryOperation::SubmitComment,
                None,
                error.kind,
                "context synchronization requires recovery",
            ))
        }
        error => error,
    };
    CommentPublicationState::Pending {
        reason: CommentPublicationPendingReason::Synchronization(Box::new(error)),
    }
}

impl RepositoryService {
    pub fn submit_comment<P: SessionCredentialProvider>(
        &self,
        request: PublishCommentRequest,
        session: &mut SessionCredentials<P>,
    ) -> Result<CommentSubmissionOutcome, CommentSubmissionError> {
        self.publish_comment_with_config(request, None, session)
    }

    #[doc(hidden)]
    pub fn submit_comment_with_identity_config_for_testing<P: SessionCredentialProvider>(
        &self,
        request: PublishCommentRequest,
        effective_config: &Config,
        session: &mut SessionCredentials<P>,
    ) -> Result<CommentSubmissionOutcome, CommentSubmissionError> {
        self.publish_comment_with_config(request, Some(effective_config), session)
    }

    fn publish_comment_with_config<P: SessionCredentialProvider>(
        &self,
        request: PublishCommentRequest,
        config: Option<&Config>,
        session: &mut SessionCredentials<P>,
    ) -> Result<CommentSubmissionOutcome, CommentSubmissionError> {
        let id = request.comment.target.operation_id;
        let result = (|| {
            let (_, root) = canonical_repository_root(
                &request.comment.target.root,
                RepositoryOperation::SubmitComment,
            )?;
            if let Some(binding) = self.read_comment_binding(&root, id)? {
                binding.matches_request(&request.comment)?;
                if let Ok((receipt, time)) =
                    self.prove_comment_receipt(&root, &binding, Some(&request.comment.body))
                {
                    let persistence_pending = self
                        .store_comment_receipt(&root, &binding, receipt.checkpoint_oid, &time)
                        .is_err();
                    return self.continue_comment_publication(
                        receipt,
                        LocalCheckpoint::NoChange,
                        request.approval,
                        request.confirmed_identity,
                        false,
                        persistence_pending,
                        session,
                    );
                }
                // Recorded receipts are immutable; failed proof cannot authorize a
                // return to the file-writing half of submission.
                if binding.checkpoint_oid.is_some() {
                    return Err(recovery_error());
                }
            }
            let local = self.checkpoint_comment_locally(request.comment.clone(), config)?;
            let (context, checkpoint, pending, mut binding) = match local {
                LocalCommentCheckpointOutcome::IdentityRequired { context } => {
                    return Ok(CommentSubmissionOutcome::IdentityRequired { context });
                }
                LocalCommentCheckpointOutcome::Saved {
                    context,
                    checkpoint,
                    binding,
                } => (context, checkpoint, false, binding),
                LocalCommentCheckpointOutcome::IndexPending {
                    context,
                    checkpoint,
                    binding,
                } => (context, checkpoint, true, binding),
            };
            if let LocalCheckpoint::Checkpointed { commit_oid }
            | LocalCheckpoint::RefreshPending { commit_oid } = &checkpoint
            {
                binding.checkpoint_oid = Some(*commit_oid);
            }
            let (receipt, time) =
                match self.prove_comment_receipt(&root, &binding, Some(&request.comment.body)) {
                    Ok(value) => value,
                    Err(error) => {
                        // A scoped commit returned by the local writer remains
                        // authoritative even if later correlation proof is refused.
                        // Never turn it into an unsaved result or attempt transport.
                        if let LocalCheckpoint::Checkpointed { commit_oid }
                        | LocalCheckpoint::RefreshPending { commit_oid } = &checkpoint
                        {
                            return Ok(CommentSubmissionOutcome::Saved {
                                receipt: Box::new(binding_receipt(&root, &binding, *commit_oid)),
                                context,
                                checkpoint,
                                publication: CommentPublicationState::Pending {
                                    reason: CommentPublicationPendingReason::LocalRecoveryRequired,
                                },
                                indexing: CommentIndexingState {
                                    local_pending: true,
                                    remote_pending: false,
                                },
                            });
                        }
                        return Err(error);
                    }
                };
            let persisted = self
                .store_comment_receipt(&root, &binding, receipt.checkpoint_oid, &time)
                .is_ok();
            // The local OID is already proved even if receipt/cache writes now fail.
            if !persisted || pending {
                return Ok(CommentSubmissionOutcome::Saved {
                    receipt: Box::new(receipt),
                    context,
                    checkpoint,
                    publication: CommentPublicationState::Pending {
                        reason: CommentPublicationPendingReason::LocalRecoveryRequired,
                    },
                    indexing: CommentIndexingState {
                        local_pending: true,
                        remote_pending: false,
                    },
                });
            }
            if self
                .check_failure(
                    FailurePoint::CommentAfterLocalHandoff,
                    RepositoryOperation::SubmitComment,
                    &root,
                )
                .is_err()
            {
                return Ok(CommentSubmissionOutcome::Saved {
                    receipt: Box::new(receipt),
                    context,
                    checkpoint,
                    publication: pending_synchronization(SynchronizationError::Interrupted),
                    indexing: CommentIndexingState::default(),
                });
            }
            self.continue_comment_publication(
                receipt,
                checkpoint,
                request.approval,
                request.confirmed_identity,
                false,
                false,
                session,
            )
        })();
        result.map_err(|error: RepositoryError| {
            let mut error = CommentSubmissionError::from(error);
            error.operation_id = Some(id);
            error
        })
    }

    pub fn retry_comment_publication<P: SessionCredentialProvider>(
        &self,
        request: RetryCommentPublicationRequest,
        session: &mut SessionCredentials<P>,
    ) -> Result<CommentSubmissionOutcome, CommentSubmissionError> {
        let result = (|| {
            let (_, root) =
                canonical_repository_root(&request.root, RepositoryOperation::SubmitComment)?;
            let binding = self
                .read_comment_binding(&root, request.operation_id)?
                .ok_or_else(recovery_error)?;
            // Reconciliation may recover a commit-before-receipt interruption, but
            // never writes a comment or guesses a creation time.
            let (receipt, time) = self.prove_comment_receipt(&root, &binding, None)?;
            let persistence_pending = self
                .store_comment_receipt(&root, &binding, receipt.checkpoint_oid, &time)
                .is_err();
            self.continue_comment_publication(
                receipt,
                LocalCheckpoint::NoChange,
                request.approval,
                request.confirmed_identity,
                request.restart,
                persistence_pending,
                session,
            )
        })();
        result.map_err(|error: RepositoryError| {
            let mut error = CommentSubmissionError::from(error);
            error.operation_id = Some(request.operation_id);
            error
        })
    }

    /// This lookup grants no ownership: the existing child cancellation API decides.
    pub fn cancel_comment_publication(
        &self,
        root: &Path,
        operation_id: OperationId,
    ) -> Result<bool, CommentSubmissionError> {
        let (_, root) = canonical_repository_root(root, RepositoryOperation::SubmitComment)?;
        let binding = self
            .read_comment_binding(&root, operation_id)?
            .ok_or_else(recovery_error)?;
        let active = self
            .active_remote_operation(&root)?
            .is_some_and(|child| child.operation_id() == binding.synchronization_id);
        if active {
            self.cancel_remote_operation(&root, binding.synchronization_id)?;
        }
        Ok(active)
    }

    #[allow(clippy::too_many_arguments)]
    fn continue_comment_publication<P: SessionCredentialProvider>(
        &self,
        receipt: CommentReceipt,
        checkpoint: LocalCheckpoint,
        approval: Option<HostApproval>,
        confirmed_identity: Option<ConfirmedCommitIdentity>,
        restart: bool,
        persistence_pending: bool,
        session: &mut SessionCredentials<P>,
    ) -> Result<CommentSubmissionOutcome, RepositoryError> {
        let context = receipt.context();
        let mut indexing = CommentIndexingState::default();
        let pending = self
            .comment_local_pending(&receipt.root, receipt.operation_id)
            .unwrap_or(true);
        if pending && !persistence_pending {
            let _ = self.repair_comment_handoff(&receipt);
        }
        indexing.local_pending = persistence_pending
            || self
                .comment_local_pending(&receipt.root, receipt.operation_id)
                .unwrap_or(true);
        let mut publication = CommentPublicationState::Pending {
            reason: CommentPublicationPendingReason::LocalRecoveryRequired,
        };
        if !indexing.local_pending {
            publication = self.synchronize_comment_receipt(
                &receipt,
                approval,
                confirmed_identity,
                restart,
                session,
                &mut indexing,
            );
        }
        Ok(CommentSubmissionOutcome::Saved {
            receipt: Box::new(receipt),
            publication,
            indexing,
            context,
            checkpoint,
        })
    }

    fn repair_comment_handoff(&self, receipt: &CommentReceipt) -> Result<(), RepositoryError> {
        let (repository, root) =
            canonical_repository_root(&receipt.root, RepositoryOperation::SubmitComment)?;
        // Git proof has already established the authoritative checkpoint. Only
        // its unfinished pre-index observation may advance; a live index owner
        // retains its state and epoch for the existing refresh claim protocol.
        {
            let _lease = repository_lease(&repository, &root, RepositoryOperation::SubmitComment)?;
            if let Err(error) = registered_repository_id(
                &self.registry_path,
                &root,
                RepositoryOperation::SubmitComment,
            ) {
                if error.kind == RepositoryErrorKind::RepositoryNotRegistered {
                    self.park_comment_registration_handoff(&root, receipt.operation_id)?;
                }
                return Err(error);
            }
            let _guard = cache_write_guard(
                &self.registry_path,
                &root,
                RepositoryOperation::SubmitComment,
            )?;
            let connection = open_registry(&self.registry_path, &mut |_| {})?;
            connection.execute("UPDATE operation_records SET state='authoring_checkpoint_observed',completed_step='authoring_checkpoint_observed'
                WHERE operation_ulid=?1 AND root_path=?2 AND action='submit_comment' AND (state IN ('created','worktree_observed','comment_checkpoint_intent','authoring_destination_observed') OR (state='completed' AND completed_step='comment_registration_pending'))",
                params![receipt.operation_id.to_string(),root.to_str()]).map_err(|_| recovery_error())?;
        }
        self.refresh_repository(RefreshRepositoryRequest {
            root,
            operation_id: receipt.operation_id,
        })?;
        Ok(())
    }

    fn synchronize_comment_receipt<P: SessionCredentialProvider>(
        &self,
        receipt: &CommentReceipt,
        approval: Option<HostApproval>,
        confirmed_identity: Option<ConfirmedCommitIdentity>,
        restart: bool,
        session: &mut SessionCredentials<P>,
        indexing: &mut CommentIndexingState,
    ) -> CommentPublicationState {
        let pending = pending_synchronization;
        let child = self.comment_child_authority(&receipt.root, receipt.synchronization_id);
        let child = match child {
            Ok(child) => child,
            Err(_) => return pending(SynchronizationError::RecoveryRequired),
        };
        let terminal = child == Some(true);
        // This check permits unrelated dirty work but refuses a moved context or
        // missing/replaced canonical blob, including local-only pending replay.
        if !terminal && !receipt.contained_at_context() {
            return pending(SynchronizationError::RecoveryRequired);
        }
        if child.is_none() {
            match read_configuration_for(&receipt.root, RepositoryOperation::SubmitComment) {
                Ok(ConfigurationInspection::Valid(config))
                    if config.publication_remote.is_none() =>
                {
                    return CommentPublicationState::Pending {
                        reason: CommentPublicationPendingReason::NoPublicationRemote,
                    };
                }
                Ok(ConfigurationInspection::Valid(_)) => {}
                _ => return pending(SynchronizationError::RecoveryRequired),
            }
        }
        // Historical authority is independent of today's branch/endpoint. A
        // nonterminal child, however, may not publish a tree missing this comment.
        let result = self.synchronize_remote(
            SynchronizeRemoteRequest {
                root: receipt.root.clone(),
                operation_id: receipt.synchronization_id,
                target: SynchronizationTarget::Context {
                    kind: receipt.kind,
                    item_id: receipt.item_id.clone(),
                },
                approval,
                confirmed_identity,
                restart,
            },
            session,
        );
        let outcome = match result {
            Ok(SynchronizationResult::Complete(outcome)) => outcome,
            Ok(SynchronizationResult::IndexPending(value)) => {
                indexing.remote_pending = true;
                value.authoritative
            }
            Err(error) => return pending(error),
        };
        match outcome {
            SynchronizationOutcome::Published { oid, .. } if receipt.contained_at(oid) => {
                CommentPublicationState::Published { oid }
            }
            SynchronizationOutcome::AlreadyCurrent { oid, .. } if receipt.contained_at(oid) => {
                CommentPublicationState::AlreadyCurrent { oid }
            }
            _ => pending(SynchronizationError::RecoveryRequired),
        }
    }

    pub(super) fn prove_comment_receipt(
        &self,
        root: &Path,
        binding: &recovery::CommentBinding,
        body: Option<&str>,
    ) -> Result<(CommentReceipt, String), RepositoryError> {
        let repository = Repository::open(root).map_err(|_| recovery_error())?;
        let branch = format!(
            "manyhands/{}/{}",
            authoring_kind_segment(&binding.kind),
            binding.item_id
        );
        let path = PathBuf::from(format!(
            ".manyhands/comments/{}/{}.md",
            binding.item_id, binding.comment_id
        ));
        let oid = if let Some(oid) = binding.checkpoint_oid {
            oid
        } else {
            let head = repository
                .refname_to_id(&format!("refs/heads/{branch}"))
                .map_err(|_| recovery_error())?;
            let candidate =
                find_comment_checkpoint(&repository, binding, &path)?.ok_or_else(recovery_error)?;
            if head != candidate
                && !repository
                    .graph_descendant_of(head, candidate)
                    .map_err(|_| recovery_error())?
            {
                return Err(recovery_error());
            }
            candidate
        };
        let comment = checkpoint_comment(&repository, oid, binding, &path)?;
        if body.is_some_and(|body| body != comment.body) {
            return Err(RepositoryError::new(
                RepositoryOperation::SubmitComment,
                None,
                RepositoryErrorKind::OccupiedItemPath,
                "submitted body differs from the original checkpoint",
            ));
        }
        let time = comment.created_at.unix_timestamp_nanos().to_string();
        if binding
            .created_at
            .as_ref()
            .is_some_and(|recorded| recorded != &time)
        {
            return Err(recovery_error());
        }
        Ok((binding_receipt(root, binding, oid), time))
    }

    pub(super) fn comment_checkpoint_candidate_exists(
        &self,
        root: &Path,
        binding: &recovery::CommentBinding,
    ) -> Result<bool, RepositoryError> {
        let repository = Repository::open(root).map_err(|_| recovery_error())?;
        let path = PathBuf::from(format!(
            ".manyhands/comments/{}/{}.md",
            binding.item_id, binding.comment_id
        ));
        Ok(find_comment_checkpoint(&repository, binding, &path)?.is_some())
    }
}

/// A receiptless retry is rare and must also notice an unreachable original
/// checkpoint. Reachable-history-only search would authorize a replacement
/// after an external reset. No object is written and no body digest is retained.
fn find_comment_checkpoint(
    repository: &Repository,
    binding: &recovery::CommentBinding,
    path: &Path,
) -> Result<Option<git2::Oid>, RepositoryError> {
    let odb = repository.odb().map_err(|_| recovery_error())?;
    let mut found = None;
    let mut ambiguous = false;
    let subject = format!("Checkpoint comment {}", binding.comment_id);
    odb.foreach(|oid| {
        if let Ok(commit) = repository.find_commit(*oid)
            && commit.parent_count() == 1
            && commit.parent_id(0).ok() == Some(binding.pre_checkpoint_oid)
            && commit.message() == Some(subject.as_str())
            && (checkpoint_comment(repository, *oid, binding, path).is_err()
                || found.replace(*oid).is_some())
        {
            ambiguous = true;
            return false;
        }
        true
    })
    .map_err(|_| recovery_error())?;
    if ambiguous {
        return Err(recovery_error());
    }
    Ok(found)
}

fn binding_receipt(
    root: &Path,
    binding: &recovery::CommentBinding,
    oid: git2::Oid,
) -> CommentReceipt {
    CommentReceipt {
        root: root.to_owned(),
        kind: binding.kind,
        item_id: binding.item_id.clone(),
        comment_id: binding.comment_id.clone(),
        parent_id: binding.parent_id.clone(),
        operation_id: binding.operation_id,
        synchronization_id: binding.synchronization_id,
        comment_path: PathBuf::from(format!(
            ".manyhands/comments/{}/{}.md",
            binding.item_id, binding.comment_id
        )),
        context_branch: format!(
            "manyhands/{}/{}",
            authoring_kind_segment(&binding.kind),
            binding.item_id
        ),
        checkpoint_oid: oid,
    }
}

fn checkpoint_comment(
    repository: &Repository,
    oid: git2::Oid,
    binding: &recovery::CommentBinding,
    path: &Path,
) -> Result<canonical::Comment, RepositoryError> {
    let commit = repository.find_commit(oid).map_err(|_| recovery_error())?;
    if commit.parent_count() != 1
        || commit.parent_id(0).ok() != Some(binding.pre_checkpoint_oid)
        || commit.message() != Some(format!("Checkpoint comment {}", binding.comment_id).as_str())
    {
        return Err(recovery_error());
    }
    let tree = commit.tree().map_err(|_| recovery_error())?;
    let parent = commit
        .parent(0)
        .and_then(|parent| parent.tree())
        .map_err(|_| recovery_error())?;
    let diff = repository
        .diff_tree_to_tree(Some(&parent), Some(&tree), None)
        .map_err(|_| recovery_error())?;
    if diff.deltas().len() != 1 {
        return Err(recovery_error());
    }
    let delta = diff.deltas().next().ok_or_else(recovery_error)?;
    if delta.status() != git2::Delta::Added || delta.new_file().path() != Some(path) {
        return Err(recovery_error());
    }
    let entry = tree.get_path(path).map_err(|_| recovery_error())?;
    if entry.filemode() != 0o100644 {
        return Err(recovery_error());
    }
    let blob = repository
        .find_blob(entry.id())
        .map_err(|_| recovery_error())?;
    let source = std::str::from_utf8(blob.content()).map_err(|_| recovery_error())?;
    match canonical::parse_item(path, source).map_err(|_| recovery_error())? {
        canonical::CanonicalItem::Comment(comment)
            if comment.id == binding.comment_id
                && comment.item_id == binding.item_id
                && comment.parent_id == binding.parent_id
                && comment.unknown.is_empty() =>
        {
            Ok(comment)
        }
        _ => Err(recovery_error()),
    }
}

impl CommentReceipt {
    fn context(&self) -> ItemContext {
        ItemContext {
            root: self.root.clone(),
            kind: self.kind,
            item_id: self.item_id.clone(),
            branch: self.context_branch.clone(),
            worktree: self
                .root
                .join(".manyhands/worktrees")
                .join(self.item_id.to_string()),
        }
    }
    fn contained_at_context(&self) -> bool {
        let context = self.context();
        let Ok(repository) = Repository::open(&self.root) else {
            return false;
        };
        if validate_context_worktree(
            &repository,
            &context,
            false,
            RepositoryOperation::SubmitComment,
        )
        .is_err()
        {
            return false;
        }
        let Ok(linked) = Repository::open(&context.worktree) else {
            return false;
        };
        let (Ok(root_common), Ok(linked_common)) = (
            std::fs::canonicalize(repository.commondir()),
            std::fs::canonicalize(linked.commondir()),
        ) else {
            return false;
        };
        if root_common != linked_common {
            return false;
        }
        repository
            .refname_to_id(&format!("refs/heads/{}", self.context_branch))
            .is_ok_and(|oid| self.contained_at(oid))
    }
    fn contained_at(&self, oid: git2::Oid) -> bool {
        let check = || -> Result<bool, git2::Error> {
            let repository = Repository::open(&self.root)?;
            if oid != self.checkpoint_oid
                && !repository.graph_descendant_of(oid, self.checkpoint_oid)?
            {
                return Ok(false);
            }
            let original = repository
                .find_commit(self.checkpoint_oid)?
                .tree()?
                .get_path(&self.comment_path)?;
            let current = repository
                .find_commit(oid)?
                .tree()?
                .get_path(&self.comment_path)?;
            Ok(original.id() == current.id() && original.filemode() == current.filemode())
        };
        check().unwrap_or(false)
    }
}
