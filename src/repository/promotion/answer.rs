//! A pull-request promotion's answer tables (design Part C): the first table,
//! after a context child that completed, and the second, after one that
//! ended. Each judges whether the primary already holds the document (R-3) on
//! what the child left, with no network, and finishes the record in one
//! transaction. Nothing here fetches, merges, pushes or moves the primary.
use super::{
    DocumentPromotionError, PromotionExpectation,
    lifecycle::{BranchEnd, LifecycleKind, LifecycleRequest},
    observe,
    record::{
        self, BranchFinish, ChildRole, ChildState, LinkWrite, PromotionAnswer, PromotionRecord,
        PromotionSituation, PublicationCategory,
    },
};
use crate::repository::{
    ConfigurationInspection, RemoteRefPlan, RepositoryError, RepositoryOperation,
    RepositoryService, SynchronizationError, canonical_repository_root, read_configuration,
    remote::state,
};
use git2::Oid;
use std::path::Path;

/// What a begun record has of its own: its remote, the primary branch and
/// the situation it began with, never the live configuration's.
fn begun(r: &PromotionRecord) -> Result<(&str, &str, PromotionSituation, Oid), RepositoryError> {
    match (
        r.remote_name.as_deref(),
        r.primary_branch.as_deref(),
        r.situation,
        r.consented_context_oid,
    ) {
        (Some(remote), Some(primary), Some(situation), Some(consented)) => {
            Ok((remote, primary, situation, consented))
        }
        _ => Err(state::recovery_required()),
    }
}

/// The tracking primary as it is now, which the child's fetch moved.
fn tracking_primary(
    repository: &git2::Repository,
    root: &Path,
    remote: &str,
    primary: &str,
) -> Result<Option<Oid>, RepositoryError> {
    let plan = RemoteRefPlan::from_configuration(remote, primary)
        .map_err(|_| observe::unavailable(root))?;
    observe::reference_oid(repository, plan.primary().tracking_ref(), root)
}

/// The first table, after a context child that completed: R-3 judged on the
/// child's authority OID, then `already_promoted` for a promote again, else
/// `promoted` with the link built now. The record finishes `complete` with
/// the child's category.
pub(super) fn finish(
    service: &RepositoryService,
    request: &LifecycleRequest<PromotionExpectation>,
    r: &PromotionRecord,
) -> Result<(), DocumentPromotionError> {
    let root = &request.root;
    let (remote, primary, situation, _) = begun(r)?;
    let stored = match state::with_transaction(service, root, |tx, _| {
        record::child_state(tx, r, ChildRole::Context)
    })? {
        ChildState::Complete(stored) => stored,
        _ => return Err(DocumentPromotionError::RecoveryRequired),
    };
    let (authority, category) = match stored.authority {
        Some(state::SynchronizationAuthority::Published(oid)) => {
            (oid, PublicationCategory::Published)
        }
        Some(state::SynchronizationAuthority::AlreadyCurrent(oid)) => {
            (oid, PublicationCategory::AlreadyCurrent)
        }
        None => return Err(DocumentPromotionError::RecoveryRequired),
    };
    let (repository, _) = canonical_repository_root(root, RepositoryOperation::RepositorySnapshot)?;
    let tracking = tracking_primary(&repository, root, remote, primary)?;
    let given_at = (request.clock)().unix_timestamp();
    let finish = if observe::primary_holds_document(
        &repository,
        authority,
        tracking,
        primary,
        remote,
        &r.item_id,
        false,
    ) {
        BranchFinish {
            answer: PromotionAnswer::PrimaryHolds,
            category,
            link: LinkWrite::Untouched,
        }
    } else if situation == PromotionSituation::Again {
        BranchFinish {
            answer: PromotionAnswer::AlreadyPromoted,
            category,
            link: LinkWrite::Renew { given_at },
        }
    } else {
        // The link as the remote's address and the configuration read now
        // give it: the record holds no URL (R-30), and a configuration or an
        // address that cannot be read never fails the promote.
        let forge = match read_configuration(root) {
            Ok(ConfigurationInspection::Valid(config)) => config.forge(),
            _ => None,
        };
        BranchFinish {
            answer: PromotionAnswer::Promoted,
            category,
            link: LinkWrite::Give {
                link: crate::repository::closure::checkpoint::context_branch_link(
                    &repository,
                    primary,
                    forge.as_ref(),
                    remote,
                    &LifecycleKind::DocumentPromotion.context_branch(&r.item_id),
                ),
                given_at,
            },
        }
    };
    state::with_transaction(service, root, |tx, id| {
        record::complete_branch_record(tx, id, r.operation_id, &finish)
    })?;
    Ok(())
}

/// The second table, after a context child that ended (or a branch gone after
/// its child): R-3 judged on the consented tip, then the end's own answer.
/// The record is abandoned with the refusal `reconfirmation_required` and its
/// answer; a deleted remote branch also ends the document's link.
pub(super) fn end(
    service: &RepositoryService,
    root: &Path,
    r: &PromotionRecord,
    end: BranchEnd<'_>,
) -> Result<(), RepositoryError> {
    let (remote, primary, _, consented) = begun(r)?;
    let (answer, remote_gone) = match end {
        BranchEnd::ContextGone => (PromotionAnswer::ContextGone, false),
        BranchEnd::ContextChild(SynchronizationError::RemoteContextDeleted) => {
            (PromotionAnswer::RemoteBranchDeleted, true)
        }
        BranchEnd::ContextChild(SynchronizationError::HistoryUnresolved) => {
            (PromotionAnswer::HistoryUnresolved, false)
        }
        // The runner ends a record only over the stops above, and only a
        // ticket's context answers anything else it names.
        BranchEnd::ContextChild(_) => return Err(state::recovery_required()),
    };
    let (repository, _) = canonical_repository_root(root, RepositoryOperation::RepositorySnapshot)?;
    let tracking = tracking_primary(&repository, root, remote, primary)?;
    let answer = if observe::primary_holds_document(
        &repository,
        consented,
        tracking,
        primary,
        remote,
        &r.item_id,
        remote_gone,
    ) {
        PromotionAnswer::PrimaryHolds
    } else {
        answer
    };
    state::with_transaction(service, root, |tx, id| {
        record::abandon_branch_record(tx, id, r.operation_id, answer)
    })
}
