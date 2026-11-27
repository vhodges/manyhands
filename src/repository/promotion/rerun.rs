//! A pull-request promotion's re-run table (Cycle 04 decision 14, the
//! design's table RR) and the checks at a new attempt's safe points.
//!
//! The table gives each call of the request, before its context child and
//! again when the child stops, one of four answers (`BranchRerun`). It tells
//! the child's own moves from a commit it did not make by a chain of heads
//! that starts at the accepted head, and it knows once that an attempt may
//! have pushed. It reads only the registry, Git and the configuration, and
//! never the network.
use super::{
    lifecycle::{BranchRerun, LifecycleKind},
    observe,
    record::{self, ChildRole, ChildState, PromotionRecord, PromotionStep},
};
use crate::repository::{
    ConfigurationInspection, RemoteOperationPhase as Phase, RemoteOperationSafePoint,
    RepositoryError, RepositoryOperation, RepositoryService, canonical_repository_root,
    read_configuration,
    remote::{
        observation::endpoint_digest_of,
        reservation,
        state::{self, StoredRemoteOperation},
    },
};
use git2::Oid;
use rusqlite::Connection;
use std::path::Path;

/// The design's table RR in its order: RR2, RR7, RR3, RR6, RR5, RR4, RR1.
pub(super) fn decide(
    service: &RepositoryService,
    root: &Path,
    r: &PromotionRecord,
) -> Result<BranchRerun, RepositoryError> {
    let (repository, _) = canonical_repository_root(root, RepositoryOperation::RepositorySnapshot)?;
    // Git and the configuration are read before the registry's write lock.
    let tip = observe::reference_oid(
        &repository,
        &LifecycleKind::DocumentPromotion.context_ref(&r.item_id),
        root,
    )?;
    let configured = match read_configuration(root) {
        Ok(ConfigurationInspection::Valid(config)) => Some(config.publication_remote),
        _ => None,
    };
    let endpoint = r
        .remote_name
        .as_deref()
        .map(|remote| endpoint_digest_of(&repository, remote));
    state::with_transaction(service, root, |tx, id| {
        let r =
            record::read_record(tx, id, r.operation_id)?.ok_or_else(state::recovery_required)?;
        let child = record::child_state(tx, &r, ChildRole::Context)?;
        // RR2: a completed child finishes the record, whatever else changed.
        if r.step == PromotionStep::ContextSynchronized || matches!(child, ChildState::Complete(_))
        {
            return Ok(BranchRerun::Finish);
        }
        if r.step != PromotionStep::Confirmed {
            return Err(state::recovery_required());
        }
        // RR7: the context branch is gone and the child did not complete.
        let Some(tip) = tip else {
            return Ok(BranchRerun::Gone);
        };
        let row = match &child {
            ChildState::Unowned(row) | ChildState::Retained(row) => Some(row),
            ChildState::Absent | ChildState::Complete(_) => None,
        };
        // RR3: an attempt may have pushed. The flag first: a row it was
        // noted for is never settled.
        if r.context_pushed {
            return Ok(BranchRerun::Run);
        }
        if let Some(row) = row
            && !reservation::settle_unsent_promotion_push(tx, row)?
            && reservation::abandon_evidence(tx, row)?.push_oid.is_some()
        {
            record::note_context_pushed(tx, id, r.operation_id)?;
            return Ok(BranchRerun::Run);
        }
        // RR6: a held child goes on to its own end, whatever else changed.
        if held(tx, &child)? {
            return Ok(BranchRerun::Run);
        }
        // RR5: the remote or its endpoint is not the one the record began with.
        let Some(configured) = configured else {
            return Err(state::recovery_required());
        };
        if configured.as_deref() != r.remote_name.as_deref() || endpoint != r.endpoint_digest {
            return Ok(BranchRerun::Refuse);
        }
        // RR4: the branch moved by something other than the record's child.
        let accepted = accepted(&r)?;
        if !own_heads(tx, accepted, row)?.contains(&tip) {
            return Ok(BranchRerun::Refuse);
        }
        // RR1: the child's own moves are accepted and the request goes on.
        if tip != accepted {
            record::accept_context_head(tx, id, r.operation_id, accepted, tip)?;
        }
        Ok(BranchRerun::Run)
    })
}

/// The head the record's next attempt may start from: the accepted one, or
/// the consented tip until the first move.
fn accepted(r: &PromotionRecord) -> Result<Oid, RepositoryError> {
    r.accepted_context_oid
        .or(r.consented_context_oid)
        .ok_or_else(state::recovery_required)
}

/// RR6: a child `child()` restarts rather than replaces.
fn held(db: &Connection, child: &ChildState) -> Result<bool, RepositoryError> {
    Ok(match child {
        ChildState::Retained(_) => true,
        ChildState::Unowned(row) => {
            !matches!(
                row.phase,
                Phase::Cancelled | Phase::Failed | Phase::Completed
            ) && reservation::promotion_child_has_resolution(db, row)?
        }
        ChildState::Absent | ChildState::Complete(_) => false,
    })
}

/// The heads only the record's child moved the branch to: the accepted head,
/// then each move of the current attempt, counted only when that attempt
/// began there and only from a head already counted.
fn own_heads(
    db: &Connection,
    accepted: Oid,
    row: Option<&StoredRemoteOperation>,
) -> Result<Vec<Oid>, RepositoryError> {
    let mut own = vec![accepted];
    if let Some(row) = row
        && row.sync_evidence.expected_oid == Some(accepted)
    {
        for (from, to) in reservation::promotion_child_moves(db, row)? {
            if own.contains(&from) && !own.contains(&to) {
                own.push(to);
            }
        }
    }
    Ok(own)
}

/// The checks at a new attempt's safe points. The observer asks only for an
/// attempt `child()` minted: a restarted one is a held child (RR6) and goes
/// on unchecked (decision 15).
pub(super) fn admits_point(
    service: &RepositoryService,
    root: &Path,
    r: &PromotionRecord,
    point: RemoteOperationSafePoint,
) -> Result<bool, RepositoryError> {
    // The branch tip as Git holds it now, read before the registry's lock.
    let (repository, _) = canonical_repository_root(root, RepositoryOperation::RepositorySnapshot)?;
    let tip = observe::reference_oid(
        &repository,
        &LifecycleKind::DocumentPromotion.context_ref(&r.item_id),
        root,
    )?;
    state::with_transaction(service, root, |tx, id| {
        let r =
            record::read_record(tx, id, r.operation_id)?.ok_or_else(state::recovery_required)?;
        if r.context_pushed {
            return Ok(true);
        }
        let accepted = accepted(&r)?;
        let row = match record::child_state(tx, &r, ChildRole::Context)? {
            ChildState::Unowned(row) | ChildState::Retained(row) => row,
            ChildState::Absent | ChildState::Complete(_) => return Ok(true),
        };
        Ok(match point {
            // The head the attempt began at, as it recorded it before this point.
            RemoteOperationSafePoint::BeforeFetch => row
                .sync_evidence
                .expected_oid
                .is_none_or(|start| start == accepted),
            // What it is about to push: a `Checkpoint` intent's candidate,
            // which must be in the chain and still be the branch tip. An
            // `Attempt` intent belongs to a restart and is admitted.
            RemoteOperationSafePoint::BeforePush => {
                match reservation::promotion_child_push_candidate(tx, &row)? {
                    Some(candidate) => {
                        tip == Some(candidate)
                            && own_heads(tx, accepted, Some(&row))?.contains(&candidate)
                    }
                    None => true,
                }
            }
            _ => true,
        })
    })
}
