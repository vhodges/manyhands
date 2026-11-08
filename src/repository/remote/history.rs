//! The repair read: which contexts have lost their publication history, and
//! what the remote says about each after a fresh complete observation.
#![allow(clippy::result_large_err)]
use std::path::Path;

use git2::Oid;

use super::{
    ObservePublicationRemoteRequest, RemoteObservationError, RemoteOutcomeCategory,
    RemotePollInvocation, RemotePublicationEvidence, RemoteSnapshot,
    poll::{PollRelation, classify_relation},
    state,
};
use crate::{
    canonical::ItemId,
    repository::{
        AuthoringKind, OperationId, RepositoryError, RepositoryService,
        keys::{SessionCredentialProvider, SessionCredentials},
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
    pub choices: RemoteHistoryChoices,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteHistoryReport {
    pub entries: Vec<RemoteHistoryEntry>,
    /// The repository-wide cache-loss flag.
    pub cache_loss: bool,
    pub generation: i64,
}

impl RepositoryService {
    /// Observe the publication remote once (reserved, a complete advertisement,
    /// persisted as the current batch) and report every context whose
    /// publication history is unknown or that this observation just resolved.
    ///
    /// This is `observe_publication_remote`, the narrowest existing call that
    /// reserves, completes and persists a batch. It is not the full poll: that
    /// would also fast-forward and materialize local contexts, which a read
    /// must not do. The unknown set is read just before the observation, and
    /// the resolved set is the difference with the snapshot the observation
    /// returns.
    pub fn remote_history<P: SessionCredentialProvider>(
        &self,
        root: &Path,
        session: &mut SessionCredentials<P>,
    ) -> Result<RemoteHistoryReport, RemoteObservationError> {
        let before = self.remote_snapshot(root)?;
        let outcome = self.observe_publication_remote(
            ObservePublicationRemoteRequest {
                root: root.to_path_buf(),
                operation_id: OperationId::new(),
                invocation: RemotePollInvocation::Explicit,
                approval: None,
                restart: false,
            },
            session,
        )?;
        if outcome.category() != RemoteOutcomeCategory::Completed {
            return Err(RemoteObservationError::Interrupted);
        }
        Ok(self.history_report(root, &before, outcome.snapshot())?)
    }

    /// The report from the snapshots either side of an observation.
    pub(super) fn history_report(
        &self,
        root: &Path,
        before: &RemoteSnapshot,
        after: &RemoteSnapshot,
    ) -> Result<RemoteHistoryReport, RepositoryError> {
        let generation = state::with_transaction(self, root, |tx, id| state::generation(tx, id))?;
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
