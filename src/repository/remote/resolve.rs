//! The repair write: `item resolve-history` records that one context whose
//! publication history is unknown was never published, so ordinary
//! synchronization may publish it (Decisions 4 and 7).
#![allow(clippy::result_large_err)]
use std::path::Path;

use git2::Oid;

use super::{RemotePublicationEvidence, RemoteRefPlan, state};
use crate::{
    canonical::ItemId,
    repository::{
        AuthoringKind, ConfigurationInspection, RepositoryError, RepositoryErrorKind,
        RepositoryOperation, RepositoryService, authoring_kind_segment, read_configuration,
        recovery,
    },
};

const OPERATION: RepositoryOperation = RepositoryOperation::RepositorySnapshot;

/// What the repair reads of one context: every input of the verb's
/// observation digest. Equal observations mean nothing the owner was shown
/// has moved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HistoryObservation {
    /// The context's branch, as its short name.
    pub(crate) branch: String,
    pub(crate) local_head: Option<Oid>,
    /// Whether the context's remote-tracking ref exists locally.
    pub(crate) tracking_ref_exists: bool,
    /// The OID the **current** batch advertises for the branch, or `None`
    /// when the batch lacks it.
    pub(crate) advertised: Option<Oid>,
    pub(crate) generation: i64,
    /// The fetch evidence, not the push overlay: only a fetch resolves.
    pub(crate) evidence: RemotePublicationEvidence,
    /// Published, then seen gone: resolving never stands in for a
    /// republish.
    pub(crate) remotely_deleted: bool,
    /// The repository-wide cache-loss flag.
    pub(crate) cache_loss: bool,
}

impl HistoryObservation {
    /// Whether the context may be resolved as never published: unknown, absent
    /// from the current batch, and not published-then-deleted.
    pub(crate) fn resolvable(&self) -> bool {
        self.evidence == RemotePublicationEvidence::HistoryUnknown
            && self.advertised.is_none()
            && !self.remotely_deleted
    }
}

/// How the marker file of a cache-loss repair ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MarkerRetirement {
    /// The flag was not cleared by this resolution.
    NotApplicable,
    Removed,
    /// The flag was cleared and no marker file was there.
    Absent,
    /// The flag was cleared, but the file could not be deleted. The commit
    /// stands: a migration ignores a marker when the flag is 0 and no rows say
    /// unknown.
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HistoryResolution {
    pub(crate) cache_loss_cleared: bool,
    pub(crate) marker: MarkerRetirement,
}

fn external_change(root: &Path) -> RepositoryError {
    RepositoryError::new(
        OPERATION,
        Some(root.to_owned()),
        RepositoryErrorKind::ExternalChange,
        "what the repair was shown has changed",
    )
}

fn plan_of(root: &Path) -> Result<RemoteRefPlan, RepositoryError> {
    let ConfigurationInspection::Valid(config) = read_configuration(root)? else {
        return Err(state::recovery_required());
    };
    let remote = config
        .publication_remote
        .as_ref()
        .ok_or_else(state::recovery_required)?;
    RemoteRefPlan::from_configuration(remote, &config.primary_branch)
        .map_err(|_| state::recovery_required())
}

fn observe_in(
    tx: &rusqlite::Transaction<'_>,
    repository_id: i64,
    root: &Path,
    plan: &RemoteRefPlan,
    kind: AuthoringKind,
    item_id: &ItemId,
) -> Result<HistoryObservation, RepositoryError> {
    let snapshot = state::read_snapshot(tx, repository_id)?;
    let generation = state::generation(tx, repository_id)?;
    let git = git2::Repository::open(root).map_err(|_| state::recovery_required())?;
    let branch = format!("manyhands/{}/{item_id}", authoring_kind_segment(&kind));
    let target = plan.context(kind, item_id);
    let advertised = snapshot
        .observations()
        .iter()
        .find(|observation| observation.remote_ref() == Some(target.remote_ref()))
        .map(|observation| observation.advertised_oid());
    Ok(HistoryObservation {
        local_head: git.refname_to_id(&format!("refs/heads/{branch}")).ok(),
        tracking_ref_exists: git.refname_to_id(target.tracking_ref()).is_ok(),
        branch,
        advertised,
        generation,
        evidence: snapshot.fetch_publication_evidence_for(kind, item_id),
        remotely_deleted: snapshot.context_deletion_observed(kind, item_id),
        cache_loss: snapshot.history_unknown(),
    })
}

impl RepositoryService {
    /// Reads what `item resolve-history` covers for one context. It takes no
    /// reservation and touches no remote.
    pub(crate) fn history_observation(
        &self,
        root: &Path,
        kind: AuthoringKind,
        item_id: &ItemId,
    ) -> Result<HistoryObservation, RepositoryError> {
        let plan = plan_of(root)?;
        state::with_transaction(self, root, |tx, id| {
            observe_in(tx, id, root, &plan, kind, item_id)
        })
    }

    /// Records the context as never published, in one registry transaction
    /// that re-reads first: if anything `expected` covers has moved the answer
    /// is `ExternalChange` and nothing is written. When the cache-loss flag is
    /// set and no context still reads unknown, the same transaction clears it,
    /// and after the commit the marker file is deleted. A file that cannot be
    /// deleted is reported and does not undo the commit.
    ///
    /// The registry transaction is atomic with every other registry write (a
    /// poll's batch included), which is what makes the re-read a fence; no
    /// remote reservation is taken, since nothing here reaches the remote.
    pub(crate) fn resolve_history(
        &self,
        root: &Path,
        kind: AuthoringKind,
        item_id: &ItemId,
        expected: &HistoryObservation,
    ) -> Result<HistoryResolution, RepositoryError> {
        let plan = plan_of(root)?;
        let cache_loss_cleared = state::with_transaction(self, root, |tx, id| {
            let found = observe_in(tx, id, root, &plan, kind, item_id)?;
            if found != *expected || !found.resolvable() {
                return Err(external_change(root));
            }
            state::record_context_resolved(tx, id, &plan, kind, item_id)?;
            state::clear_cache_loss_when_resolved(tx, id)
        })?;
        let marker = if !cache_loss_cleared {
            MarkerRetirement::NotApplicable
        } else {
            match self
                .registry_path
                .parent()
                .map(recovery::retire_remote_history_marker)
            {
                Some(Ok(true)) => MarkerRetirement::Removed,
                Some(Ok(false)) => MarkerRetirement::Absent,
                Some(Err(_)) | None => MarkerRetirement::Failed,
            }
        };
        Ok(HistoryResolution {
            cache_loss_cleared,
            marker,
        })
    }
}
