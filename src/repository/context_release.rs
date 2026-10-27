//! `release_context`: the work context of one item, its branch and its
//! worktree, is removed when it holds nothing of anyone's: no change in
//! the worktree, no commit of its own on the branch, and nothing
//! published. It commits nothing and touches no remote state (R-32).
//!
//! One lease is held from the row's beginning to its hand-off, and the
//! cleanup primitives are used under it:
//!
//! 1. The row is begun.
//! 2. A new row whose context has neither its worktree nor its branch
//!    answers `AlreadyAbsent`: nothing is touched and the row is
//!    discarded.
//! 3. An unfinished closure or promotion of the item owns the context,
//!    and is `RecoveryRequired`. A ticket that is closed is
//!    `TicketClosed`.
//! 4. Before anything is touched: (a) the worktree, when there, is clean
//!    (`DirtyWorktree`) and is the context's as it is registered
//!    (`RecoveryRequired`); (b) primary is checked out at the root
//!    (`RecoveryRequired`) and the branch's tip is primary's tip or
//!    behind it (`ExternalChange`); (c) no remote ref names the context
//!    and nothing proves that it was published (`ExternalChange`), an
//!    unknown history being no proof (R-43); (d) no synchronization of
//!    the item is pending (`RecoveryRequired`). A refusal of a new row
//!    discards it.
//! 5. The worktree is removed, and `ContextWorktreeReleased` recorded.
//! 6. (b) and (c) are read again of the branch's tip as it is now, the
//!    branch is deleted at that tip, and `ContextBranchReleased`
//!    recorded. A refusal here keeps the row pending.
//! 7. The index is told, as a save's hand-off tells it: `Released`, or
//!    `IndexPending` with the row owed to `operation resume`.
//!
//! **Re-entry.** A pending row with no step runs from step 3 again; one
//! with `ContextWorktreeReleased` goes to step 6, and one with
//! `ContextBranchReleased` to step 7. A row that has completed is not
//! looked at again.

use std::path::{Path, PathBuf};

use git2::{Oid, Repository};

use super::{
    AuthoringKind, CallEffects, ConfigurationInspection, FailurePoint, ItemContext, OperationId,
    RemotePublicationEvidence, RepositoryError, RepositoryErrorKind, RepositoryOperation,
    RepositoryService, authoring_error, authoring_kind_segment, canonical,
    canonical_repository_root, checked_out_branch,
    cleanup::{BranchDeletion, ContextCleanup, ReleaseCheck, WorktreeRelease},
    closure::record::ClosureRecord,
    item_context,
    promotion::{lifecycle_record::unfinished_record, record::PromotionRecord},
    read_configuration_for,
    recovery::{LocalStep, RecoveryRecord},
    remote::state,
    ticket_guard,
};

/// A request to release the work context of one item.
#[derive(Clone, Debug)]
pub struct ReleaseContextRequest {
    pub root: PathBuf,
    pub kind: AuthoringKind,
    pub item_id: canonical::ItemId,
    pub operation_id: OperationId,
}

/// What a release came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextReleaseOutcome {
    /// The worktree and the branch are gone, and the index was told.
    Released,
    /// Neither was there: nothing was touched.
    AlreadyAbsent,
    /// The worktree and the branch are gone and the index was not told:
    /// the row is pending, and `operation resume` finishes it.
    IndexPending,
}

const OPERATION: RepositoryOperation = RepositoryOperation::ReleaseContext;

/// What a release's row is bound to: the context, as an authoring
/// operation's target names it, so that a synchronization of the same
/// context in flight stands in its way, as it does in a save's.
fn release_target(kind: AuthoringKind, item_id: &canonical::ItemId) -> String {
    format!(
        "authoring-context-v1/{}/{item_id}/Release",
        authoring_kind_segment(&kind)
    )
}

fn recovery_required(root: &Path, message: &str) -> RepositoryError {
    authoring_error(
        OPERATION,
        root,
        RepositoryErrorKind::RecoveryRequired,
        message,
    )
}

fn external_change(root: &Path, message: &str) -> RepositoryError {
    authoring_error(
        OPERATION,
        root,
        RepositoryErrorKind::ExternalChange,
        message,
    )
}

fn git(root: &Path, error: git2::Error) -> RepositoryError {
    RepositoryError::git(OPERATION, Some(root.to_owned()), error)
}

/// Whether something is at `path`, a link included.
fn present(root: &Path, path: &Path) -> Result<bool, RepositoryError> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(RepositoryError::io(OPERATION, Some(root.to_owned()), error)),
    }
}

/// One release, as its call holds it.
struct Release<'a> {
    repository: Repository,
    root: &'a Path,
    primary_branch: &'a str,
    context: ItemContext,
    record: RecoveryRecord,
}

impl Release<'_> {
    /// Whether the context's worktree is there: its directory, or its
    /// registration in the repository.
    fn worktree_present(&self) -> Result<bool, RepositoryError> {
        let admin = self
            .repository
            .commondir()
            .join("worktrees")
            .join(self.context.item_id.to_string());
        Ok(present(self.root, &self.context.worktree)? || present(self.root, &admin)?)
    }

    /// The tip of the context's branch, when it exists.
    fn branch_tip(&self) -> Result<Option<Oid>, RepositoryError> {
        let name = format!("refs/heads/{}", self.context.branch);
        match self.repository.find_reference(&name) {
            Ok(reference) => match (reference.symbolic_target(), reference.target()) {
                (None, Some(tip)) => Ok(Some(tip)),
                _ => Err(recovery_required(
                    self.root,
                    "the context branch is not a branch at a commit",
                )),
            },
            Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(None),
            Err(error) => Err(git(self.root, error)),
        }
    }

    /// (b), first half: primary is checked out at the root. Its tip.
    fn primary_checked_out(&self) -> Result<Oid, RepositoryError> {
        let refused = || {
            recovery_required(
                self.root,
                "the configured primary branch must be checked out at the repository root",
            )
        };
        match checked_out_branch(&self.repository, self.root, OPERATION) {
            Ok(branch) if branch == self.primary_branch => {}
            Ok(_) | Err(_) => return Err(refused()),
        }
        self.repository
            .refname_to_id(&format!("refs/heads/{}", self.primary_branch))
            .map_err(|_| refused())
    }

    /// (b), second half: the branch holds no commit of its own.
    fn contained(&self, primary: Oid, tip: Oid) -> Result<(), RepositoryError> {
        if tip == primary
            || self
                .repository
                .graph_descendant_of(primary, tip)
                .map_err(|error| git(self.root, error))?
        {
            Ok(())
        } else {
            Err(external_change(
                self.root,
                "the context branch holds a commit of its own",
            ))
        }
    }

    /// (c): no remote ref names the context, and nothing proves that it
    /// was published. A history that is not known is no proof (R-43):
    /// with no such ref and no commit of its own, nothing of the
    /// branch's can be on a remote.
    fn unpublished(&self, service: &RepositoryService) -> Result<(), RepositoryError> {
        let suffix = format!(
            "/manyhands/{}/{}",
            authoring_kind_segment(&self.context.kind),
            self.context.item_id
        );
        let references = self
            .repository
            .references()
            .map_err(|error| git(self.root, error))?;
        for reference in references {
            let reference = reference.map_err(|error| git(self.root, error))?;
            let name = reference.name_bytes();
            if name.starts_with(b"refs/remotes/") && name.ends_with(suffix.as_bytes()) {
                return Err(external_change(self.root, "a remote ref names the context"));
            }
        }
        let evidence = state::with_transaction(service, self.root, |tx, id| {
            Ok(state::read_snapshot(tx, id)?
                .publication_evidence_for(self.context.kind, &self.context.item_id))
        })?;
        match evidence {
            RemotePublicationEvidence::NeverPublished
            | RemotePublicationEvidence::HistoryUnknown => Ok(()),
            RemotePublicationEvidence::ObservedPublished => {
                Err(external_change(self.root, "the context was published"))
            }
        }
    }

    /// 3: a closure or a promotion that has not ended owns the context,
    /// and a closed ticket's context is its closure's to retire.
    fn unowned(&self, service: &RepositoryService) -> Result<(), RepositoryError> {
        let item = &self.context.item_id;
        let owned = state::with_transaction(service, self.root, |tx, id| {
            Ok(unfinished_record::<ClosureRecord>(tx, id, item)?.is_some()
                || unfinished_record::<PromotionRecord>(tx, id, item)?.is_some())
        })?;
        if owned {
            return Err(recovery_required(
                self.root,
                "an unfinished closure or promotion owns the context",
            ));
        }
        if matches!(self.context.kind, AuthoringKind::Ticket) {
            ticket_guard::require_ticket_open(&self.repository, self.primary_branch, item)
                .map_err(|error| error.for_operation(OPERATION, self.root))?;
        }
        Ok(())
    }
}

impl RepositoryService {
    /// Removes the work context of one item that holds no work of its
    /// own; see the module's description.
    pub fn release_context(
        &self,
        request: ReleaseContextRequest,
    ) -> Result<ContextReleaseOutcome, RepositoryError> {
        let ReleaseContextRequest {
            root,
            kind,
            item_id,
            operation_id,
        } = request;
        self.require_index_available(OPERATION, Some(&root))?;
        let (repository, root) = canonical_repository_root(&root, OPERATION)?;
        let primary_branch = match read_configuration_for(&root, OPERATION)? {
            ConfigurationInspection::Valid(configuration) => configuration.primary_branch,
            ConfigurationInspection::Missing => {
                return Err(authoring_error(
                    OPERATION,
                    &root,
                    RepositoryErrorKind::RepositoryNotEnabled,
                    "a Manyhands configuration is required before authoring",
                ));
            }
            ConfigurationInspection::Invalid(problem) => {
                return Err(authoring_error(
                    OPERATION,
                    &root,
                    RepositoryErrorKind::InvalidConfiguration,
                    problem.message,
                ));
            }
        };
        let (lease, record) = self.begin_lifecycle(
            &repository,
            &root,
            OPERATION,
            operation_id,
            &release_target(kind, &item_id),
        )?;
        // A row that completed: the release ran to its end, and a context
        // there now is another operation's.
        if !record.is_new && !record.is_pending {
            return if record.completed_step == Some(LocalStep::ContextBranchReleased) {
                Ok(ContextReleaseOutcome::Released)
            } else {
                Err(recovery_required(
                    &root,
                    "the release's row completed without releasing its context",
                ))
            };
        }
        // The row is another call's to finish while that call is indexing
        // under it: the release's own work is done, or it would not have
        // handed off.
        if record.is_pending && self.is_being_indexed(&root, OPERATION, operation_id)? {
            return Ok(ContextReleaseOutcome::IndexPending);
        }
        let effects = CallEffects::default();
        let cleanup = match ContextCleanup::holding(&root, lease) {
            Ok(cleanup) => cleanup,
            Err(error) => {
                return self.settle_rejected_lifecycle(
                    &root,
                    OPERATION,
                    record,
                    &effects,
                    Err(error),
                );
            }
        };
        let release = Release {
            repository,
            root: &root,
            primary_branch: &primary_branch,
            context: item_context(&root, kind, item_id),
            record,
        };
        match self.release_under_lease(&release, &cleanup, &effects) {
            Ok(true) => {}
            // The lease is held until the row is closed.
            Ok(false) => {
                self.discard_lifecycle(&root, OPERATION, record)?;
                return Ok(ContextReleaseOutcome::AlreadyAbsent);
            }
            Err(error) => {
                return self.settle_rejected_lifecycle(
                    &root,
                    OPERATION,
                    record,
                    &effects,
                    Err(error),
                );
            }
        }
        Ok(
            if self.handoff_post_authoritative(
                &root,
                OPERATION,
                record,
                operation_id,
                cleanup.into_lease(),
            )? {
                ContextReleaseOutcome::Released
            } else {
                ContextReleaseOutcome::IndexPending
            },
        )
    }

    /// Steps 2 to 6, under the lease `cleanup` holds. `Ok(false)` for a
    /// new row with nothing to release.
    fn release_under_lease(
        &self,
        release: &Release<'_>,
        cleanup: &ContextCleanup,
        effects: &CallEffects,
    ) -> Result<bool, RepositoryError> {
        let Release {
            root,
            context,
            record,
            ..
        } = release;
        let (root, kind, item) = (*root, context.kind, &context.item_id);
        let step = record.completed_step;
        if step == Some(LocalStep::ContextBranchReleased) {
            return Ok(true);
        }
        if step != Some(LocalStep::ContextWorktreeReleased) {
            let worktree = release.worktree_present()?;
            let tip = release.branch_tip()?;
            if record.is_new && !worktree && tip.is_none() {
                return Ok(false);
            }
            release.unowned(self)?;
            // (a)
            let tip = match (worktree, tip) {
                (true, Some(tip)) => {
                    match cleanup.precheck_release(kind, item, tip) {
                        ReleaseCheck::Clean => {}
                        ReleaseCheck::NotClean => return Err(not_clean(root)),
                        ReleaseCheck::RecoveryRequired => {
                            return Err(not_the_context(root));
                        }
                    }
                    Some(tip)
                }
                (true, None) => return Err(not_the_context(root)),
                (false, tip) => tip,
            };
            // (b)
            let primary = release.primary_checked_out()?;
            if let Some(tip) = tip {
                release.contained(primary, tip)?;
            }
            // (c)
            release.unpublished(self)?;
            // (d)
            self.reject_pending_synchronization_target(root, &kind, item, OPERATION)?;
            if let Some(tip) = tip.filter(|_| worktree) {
                effects.begin();
                match cleanup.release_worktree(kind, item, tip) {
                    WorktreeRelease::Removed => {
                        self.advance_lifecycle(
                            root,
                            OPERATION,
                            *record,
                            LocalStep::ContextWorktreeReleased,
                        )?;
                    }
                    WorktreeRelease::AlreadyAbsent => effects.undone(),
                    WorktreeRelease::NotClean => {
                        effects.undone();
                        return Err(not_clean(root));
                    }
                    WorktreeRelease::RecoveryRequired => {
                        effects.undone();
                        return Err(not_the_context(root));
                    }
                    WorktreeRelease::Incomplete => {
                        return Err(recovery_required(
                            root,
                            "the context worktree was only partly removed",
                        ));
                    }
                }
            }
            self.check_failure(FailurePoint::BeforeContextBranchRelease, OPERATION, root)?;
        }
        // 6. The branch as it is now: no commit was made on it, and
        // nothing published of it, since it was checked.
        if let Some(tip) = release.branch_tip()? {
            let primary = release.primary_checked_out()?;
            release.contained(primary, tip)?;
            release.unpublished(self)?;
            effects.begin();
            match cleanup.delete_context_branch(kind, item, tip) {
                BranchDeletion::Deleted | BranchDeletion::AlreadyAbsent => {}
                BranchDeletion::Changed => {
                    return Err(external_change(
                        root,
                        "the context branch changed while it was released",
                    ));
                }
            }
        }
        self.advance_lifecycle(root, OPERATION, *record, LocalStep::ContextBranchReleased)?;
        self.check_failure(FailurePoint::AfterContextBranchRelease, OPERATION, root)?;
        Ok(true)
    }
}

fn not_clean(root: &Path) -> RepositoryError {
    authoring_error(
        OPERATION,
        root,
        RepositoryErrorKind::DirtyWorktree,
        "the context worktree holds a change, an untracked file or an ignored file",
    )
}

fn not_the_context(root: &Path) -> RepositoryError {
    recovery_required(
        root,
        "the context worktree is not the context's as it is registered",
    )
}

#[cfg(test)]
#[path = "context_release_tests.rs"]
mod tests;
