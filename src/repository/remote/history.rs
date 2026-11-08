//! The repair read: which contexts have lost their publication history, and
//! what the remote says about each after a fresh complete observation.
#![allow(clippy::result_large_err)]
use std::path::Path;

use git2::Oid;

use super::{
    PollCompletion, PollDisposition, PollPublicationRemoteRequest, PollTargetReport,
    RemoteOutcomeCategory, RemotePollError, RemotePollInvocation, RemotePollOutcome,
    RemotePollResult, RemotePublicationEvidence, RemoteSnapshot,
    poll::{PollRelation, classify_relation},
    state,
};
use crate::{
    canonical::ItemId,
    repository::{
        AuthoringKind, OperationId, RepositoryError, RepositoryService,
        keys::{SessionCredentialProvider, SessionCredentials},
        mutation::{CancellationToken, RequestId},
        promotion::lifecycle::CancellationProbe,
    },
};

/// How a context's local head relates to the branch the remote advertises.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteHistoryRelation {
    Same,
    LocalAhead,
    RemoteAhead,
    Diverged,
}

/// What the owner may do about an unresolved context. Both are offered only
/// when the remote does not have the branch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RemoteHistoryChoices {
    pub publish: bool,
    pub release: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteHistoryEntry {
    pub kind: AuthoringKind,
    pub item_id: ItemId,
    pub branch: String,
    pub local_head: Option<Oid>,
    pub tracking_ref_exists: bool,
    pub advertised: Option<Oid>,
    /// `None` when the remote lacks the branch, or when either commit isn't in
    /// the local object database to compare.
    pub relation: Option<RemoteHistoryRelation>,
    pub ahead: Option<usize>,
    pub behind: Option<usize>,
    /// The fetch recorded the context as published, so no choice is needed.
    pub resolved: bool,
    /// What the poll did with this context (a fast-forward shows here, not as
    /// `Same`). `None` when the poll reported no target for it.
    pub disposition: Option<PollDisposition>,
    pub choices: RemoteHistoryChoices,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteHistoryReport {
    pub entries: Vec<RemoteHistoryEntry>,
    /// The repository-wide cache-loss flag.
    pub cache_loss: bool,
    pub generation: i64,
}

/// Why the repair read has no report.
#[derive(Debug)]
pub enum RemoteHistoryError {
    /// The poll itself failed (transport, busy, conflict pending, repository).
    Poll(Box<RemotePollError>),
    /// The poll ended with a stopping category and refreshed nothing.
    Stopped(RemoteOutcomeCategory),
    Cancelled,
    Interrupted,
    /// An explicit poll is never asked for eligibility, and a finished one has
    /// a category: neither answer should occur.
    Internal,
}
impl std::fmt::Display for RemoteHistoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Poll(_) => "remote history poll failed",
            Self::Stopped(_) => "remote history poll stopped",
            Self::Cancelled => "remote history cancelled",
            Self::Interrupted => "remote history interrupted",
            Self::Internal => "remote history internal error",
        })
    }
}
impl std::error::Error for RemoteHistoryError {}
impl From<RemotePollError> for RemoteHistoryError {
    fn from(error: RemotePollError) -> Self {
        Self::Poll(Box::new(error))
    }
}
impl From<RepositoryError> for RemoteHistoryError {
    fn from(error: RepositoryError) -> Self {
        Self::Poll(Box::new(error.into()))
    }
}

/// Fresh evidence needs a finished poll whose category is `Completed`; the
/// other outcomes leave the stored evidence as it was. Mirrors the `poll once`
/// binding's reading of a report.
fn fresh(
    completion: PollCompletion,
    outcome: Option<RemoteOutcomeCategory>,
) -> Result<(), RemoteHistoryError> {
    match (completion, outcome) {
        (PollCompletion::Cancelled, _) => Err(RemoteHistoryError::Cancelled),
        (PollCompletion::Interrupted, _) => Err(RemoteHistoryError::Interrupted),
        (PollCompletion::Completed, None) => Err(RemoteHistoryError::Internal),
        (PollCompletion::Completed, Some(RemoteOutcomeCategory::Completed)) => Ok(()),
        (PollCompletion::Completed, Some(category)) => Err(RemoteHistoryError::Stopped(category)),
    }
}

/// The targets of a poll that refreshed the evidence.
fn fresh_targets(result: RemotePollResult) -> Result<Vec<PollTargetReport>, RemoteHistoryError> {
    let outcome = match result {
        RemotePollResult::Complete(outcome) => outcome,
        RemotePollResult::IndexPending(pending) => pending.authoritative,
    };
    let RemotePollOutcome::Polled(report) = outcome else {
        return Err(RemoteHistoryError::Internal);
    };
    fresh(report.completion(), report.outcome())?;
    Ok(report.targets().to_vec())
}

impl RepositoryService {
    /// Run the manual one-shot poll (what `poll once` runs: reserved, a real
    /// fetch of the publication remote, a complete batch persisted) and report
    /// every context whose publication history is unknown or that this poll
    /// just resolved. The poll's ordinary side effects apply: a clean,
    /// strictly-behind context is fast-forwarded and unmaterialized remote
    /// contexts are materialized; each entry carries the poll's disposition.
    ///
    /// `operation_id` names the poll's record, and `request` and `token` let
    /// `request cancel` stop it, as for `poll once`. A replayed terminal
    /// result performs no fetch, so `resolved` is then empty: use a fresh
    /// operation id unless resuming one.
    ///
    /// The resolved set is the difference between the fetch evidence in the
    /// snapshot read just before the poll and the one read after it. It isn't
    /// captured inside the persistence transaction; a concurrent poll that
    /// resolves a context in between still leaves it listed as resolved.
    pub fn remote_history<P: SessionCredentialProvider>(
        &self,
        root: &Path,
        operation_id: OperationId,
        request: RequestId,
        token: CancellationToken,
        session: &mut SessionCredentials<P>,
    ) -> Result<RemoteHistoryReport, RemoteHistoryError> {
        let before = self.remote_snapshot(root)?;
        let result = self.poll_publication_remote_observed(
            PollPublicationRemoteRequest {
                root: root.to_path_buf(),
                operation_id,
                invocation: RemotePollInvocation::Explicit,
                approval: None,
                restart: false,
            },
            session,
            CancellationProbe::new(token, request),
        )?;
        let targets = fresh_targets(result)?;
        Ok(self.history_report(root, &before, &targets)?)
    }

    /// The report from the snapshot before a poll and the registry after it.
    /// The after-snapshot and the generation are read in one transaction.
    pub(super) fn history_report(
        &self,
        root: &Path,
        before: &RemoteSnapshot,
        dispositions: &[PollTargetReport],
    ) -> Result<RemoteHistoryReport, RepositoryError> {
        let (after, generation) = state::with_transaction(self, root, |tx, id| {
            Ok((state::read_snapshot(tx, id)?, state::generation(tx, id)?))
        })?;
        let after = &after;
        let repository = git2::Repository::open(root).map_err(|_| state::recovery_required())?;
        let mut entries = Vec::new();
        for context in after.contexts() {
            let (Some(kind), Some(item_id)) = (context.kind(), context.item_id()) else {
                continue;
            };
            // Fetch evidence, not the push overlay: only a fetch resolves.
            let now = after.fetch_publication_evidence_for(kind, item_id);
            let resolved = match now {
                RemotePublicationEvidence::HistoryUnknown => false,
                RemotePublicationEvidence::ObservedPublished
                    if before.fetch_publication_evidence_for(kind, item_id)
                        == RemotePublicationEvidence::HistoryUnknown =>
                {
                    true
                }
                _ => continue,
            };
            let branch = format!(
                "manyhands/{}/{item_id}",
                crate::repository::authoring_kind_segment(&kind)
            );
            let local_head = repository
                .refname_to_id(&format!("refs/heads/{branch}"))
                .ok();
            let tracking_ref_exists = context
                .tracking_ref()
                .is_some_and(|name| repository.refname_to_id(name).is_ok());
            let advertised = context.advertised_oid();
            let counts = local_head
                .zip(advertised)
                .and_then(|(local, remote)| repository.graph_ahead_behind(local, remote).ok());
            let relation = local_head.zip(advertised).and_then(|(local, remote)| {
                classify_relation(local, remote, |a, b| repository.graph_descendant_of(b, a))
                    .ok()
                    .map(|relation| match relation {
                        PollRelation::Current => RemoteHistoryRelation::Same,
                        PollRelation::Behind => RemoteHistoryRelation::RemoteAhead,
                        PollRelation::LocalAhead => RemoteHistoryRelation::LocalAhead,
                        PollRelation::Diverged => RemoteHistoryRelation::Diverged,
                    })
            });
            let offered = !resolved && advertised.is_none();
            entries.push(RemoteHistoryEntry {
                kind,
                item_id: item_id.clone(),
                branch,
                local_head,
                tracking_ref_exists,
                advertised,
                relation,
                ahead: counts.map(|(ahead, _)| ahead),
                behind: counts.map(|(_, behind)| behind),
                resolved,
                disposition: dispositions
                    .iter()
                    .find(|target| target.kind() == Some(kind) && target.item_id() == Some(item_id))
                    .map(PollTargetReport::disposition),
                choices: RemoteHistoryChoices {
                    publish: offered,
                    release: offered,
                },
            });
        }
        entries.sort_by(|a, b| a.branch.cmp(&b.branch));
        Ok(RemoteHistoryReport {
            entries,
            cache_loss: after.history_unknown(),
            generation,
        })
    }
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
