//! Bounded composition of the real owned journal APIs for retirement exclusion
//! tests. Synthetic observation/native-artifact evidence, NOT SSH/merge/filesystem
//! resolution proof. No production API, direct SQL writes or ownership bypass.
use super::{merge::IntegrationStage, state, *};
use crate::repository::{OperationId, RepositoryError, RepositoryService};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum ResolutionStage {
    Intent,
    Paths,
    Candidate,
    Applied,
    Releasing,
    Released,
}

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
            .and_then(|head| head.peel_to_commit())
            .map_err(|_| state::recovery_required())?;
        if window != 0 {
            let batch = state::with_transaction(self, root, |tx, id| {
                tx.query_row("SELECT id FROM remote_observation_batches WHERE repository_id=?1 AND is_current=1", [id], |r| r.get(0)).map_err(|_| state::recovery_required())
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

    pub(in super::super) fn retirement_test_resolution(
        &self,
        root: &Path,
        operation: OperationId,
        plan: &RemoteRefPlan,
        window: u32,
        stage: ResolutionStage,
    ) -> Result<(), RepositoryError> {
        let RemoteReservationOutcome::Reserved(owner) = self
            .reacquire_synchronization_conflict_in_window(
                root,
                operation,
                &RemoteOperationTarget::for_primary_synchronization(plan),
                window,
                0,
                [31; 32],
            )?
        else {
            return Err(state::recovery_required());
        };
        let git = git2::Repository::open(root).map_err(|_| state::recovery_required())?;
        let commit = git
            .head()
            .and_then(|head| head.peel_to_commit())
            .map_err(|_| state::recovery_required())?;
        let attempt = OperationId::new();
        self.prepare_synchronization_resolution_attempt_in_window(
            root,
            &owner,
            window,
            &state::ResolutionAttemptIntent {
                attempt_id: attempt,
                step_ordinal: 0,
                observation_digest: [31; 32],
                input_digest: [32; 32],
                preflight_digest: [33; 32],
                identity_confirmation_id: None,
                commit_time: (123, 0),
            },
            &[state::ResolutionPathIntent {
                ordinal: 0,
                path_digest: [34; 32],
                expected_digest: [35; 32],
                result_digest: [36; 32],
                prewrite_digest: [37; 32],
                base_blob_oid: None,
                local_blob_oid: None,
                incoming_blob_oid: None,
                mode: 0o100644,
            }],
        )?;
        self.begin_synchronization_resolution_path_effects(root, &owner, attempt)?;
        self.prepare_synchronization_index_artifact(
            root,
            &owner,
            attempt,
            &state::ResolutionIndexArtifact {
                device: 0,
                inode: 1,
                sentinel_digest: [39; 32],
                baseline_digest: [40; 32],
                baseline_identity: (0, 1),
                metadata: [None; 3],
                output: None,
                ref_phase: "not_started".into(),
                phase: "intent".into(),
            },
        )?;
        if stage == ResolutionStage::Intent {
            return Ok(());
        }
        self.advance_synchronization_index_artifact(root, &owner, attempt, "published")?;
        self.prepare_synchronization_ref_log_artifact(
            root,
            &owner,
            attempt,
            "baseline",
            &state::ResolutionRefLogArtifact {
                device: 0,
                inode: 1,
                digest: [38; 32],
            },
        )?;
        if stage == ResolutionStage::Paths {
            return Ok(());
        }
        self.observe_synchronization_resolution_path_effect(root, &owner, attempt, 0)?;
        self.prepare_synchronization_resolution_candidate(root, &owner, attempt, commit.id())?;
        if stage == ResolutionStage::Candidate {
            return Ok(());
        }
        self.prepare_synchronization_index_output(root, &owner, attempt, (0, 2, [41; 32]))?;
        self.prepare_synchronization_ref_log_artifact(
            root,
            &owner,
            attempt,
            "transition",
            &state::ResolutionRefLogArtifact {
                device: 0,
                inode: 2,
                digest: [42; 32],
            },
        )?;
        self.advance_synchronization_resolution_ref_effect(root, &owner, attempt, "observed")?;
        self.observe_synchronization_resolution_checkpoint(
            root,
            &owner,
            attempt,
            commit.id(),
            commit.tree_id(),
        )?;
        if stage == ResolutionStage::Applied {
            return Ok(());
        }
        self.advance_synchronization_index_artifact(root, &owner, attempt, "release_intent")?;
        if stage == ResolutionStage::Releasing {
            return Ok(());
        }
        self.advance_synchronization_index_artifact(root, &owner, attempt, "released")
    }
}
