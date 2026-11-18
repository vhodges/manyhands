//! Real owned integration-journal composition for local retirement exclusion
//! controls. Synthetic native-artifact evidence, never SSH or merge proof.
use super::{merge::IntegrationStage, state, *};
use crate::repository::{RepositoryError, RepositoryService};
use std::path::Path;
impl RepositoryService {
    pub(in super::super) fn retirement_test_conflict(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        window: u32,
    ) -> Result<(), RepositoryError> {
        let git = git2::Repository::open(root).map_err(|_| state::recovery_required())?;
        let commit = git
            .head()
            .and_then(|h| h.peel_to_commit())
            .map_err(|_| state::recovery_required())?;
        if window != 0 {
            let batch = state::with_transaction(self, root, |tx, id| {
                tx.query_row("SELECT id FROM remote_observation_batches WHERE repository_id=?1 AND is_current=1",[id],|r|r.get(0)).map_err(|_|state::recovery_required())
            })?;
            self.prepare_synchronization_window(
                root,
                owner,
                window,
                &state::IntegrationWindowIntent {
                    observation_batch_id: batch,
                    local_oid: commit.id(),
                    primary_oid: commit.id(),
                    context_oid: None,
                },
            )?;
        }
        self.prepare_synchronization_integration_in_window(
            root,
            owner,
            window,
            &state::IntegrationStepIntent {
                ordinal: 0,
                stage: IntegrationStage::Primary,
                local_oid: commit.id(),
                incoming_oid: commit.id(),
                baseline_tree_oid: commit.tree_id(),
                baseline_index_digest: [7; 32],
            },
        )?;
        self.begin_synchronization_integration_effect_in_window(root, owner, window, 0, None)?;
        self.release_synchronization_conflict_in_window(root, owner, window, 0, [31; 32])
    }
}
