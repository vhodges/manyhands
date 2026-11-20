# Fast writes execution ledger

Ticket `01M7RDVNZDCCVSMWHDY9503852` (vh-f3wts). Plan: `docs/plans/2026-11-19-fast-writes-implementation.md`. Base SHA: `ad3527dd` (main `790a71ed` merged, not rebased).

## Reference drift (Task 0)

Checked by grep against the tree at `ad3527dd`. No file under `src/`, `tests/`, `scripts/`, `skills/` or `docs/RFC/` changed between the plan's first draft (`97e4dc6c`) and the base, so the drift below is in the plan's own numbers, not from main moving.

| Cited in plan | Plan says | Current location | Note |
| --- | --- | --- | --- |
| Task 7 Files: `persist_rebuild_observation` | `src/repository.rs` ~6202 | `src/repository.rs:6153` (called at ~1740; the function runs to ~6227, so ~6202 is inside it) | Drift of 49 lines. Use 6153. |
| Task 10 Files: `FailurePoint` and the new `FailurePoint::AfterCheckpointBeforeIndexStep` | listed under `src/repository/recovery.rs` | `pub enum FailurePoint` is at `src/repository.rs:931`; `BeforeIndexTransactionCommit` is one of its variants (used ~1739) | Wrong file in the plan, not in `recovery.rs`. The new variant goes in `src/repository.rs`. `src/drafts/store.rs:33` has an unrelated `FailurePoint`. |
| Task 5 Interfaces: `ContextObservation` ("holds today's per-context fields") | read as an existing type | missing: today's types are `RootContextObservation` (`src/repository/discovery.rs:38`), `ActiveContextObservation` (96) and `RootObservation` (54) | `ContextObservation` is a new struct Task 5 adds; `observe_root` keeps returning today's three types. Use today's names when reading the code. |
| Task 2 Interfaces: `pub(super) enum ContextKey` in `discovery.rs` | a new name | no `ContextKey` under `src/repository/`; a different `ContextKey` struct exists at `src/desktop/services/created_contexts.rs:30` | Different module, so no clash unless both are imported together. Check imports. |

Largest of the rest: 22 lines (the remote delete at `src/repository/promotion/run.rs:1362` against "~1340", where the cleanup check `precheck_context` is at exactly 1340; `persist_rebuild_retry` 6228 against ~6250; `reconcile_registration` 11326 against ~11347; `fetch_and_observe` 7905 against ~7925). Every other cited line is within 10 lines. Four citations name a different thing than a first grep suggests, and all four are correct as cited: the cleanup checks at `promotion/run.rs` ~1261, ~1340, ~1432 are `precheck_context` calls (the full `require_cleanup_clean` is at `cleanup.rs:253`/`536`, reached through them); `OperationAction` is the enum opened at `read/dto.rs:766` (cited ~773); the sync-local row prefix cited at `sync.rs` ~7053 is written by `local_refresh_identity` (called at 7053, defined 8217-8222); `recovery.rs` ~2098 is the `SELECT ... FROM operation_records` query that Task 14 extends to read `completed_step`. The `set_observation_hook_for_testing` users listed in the plan match the tree exactly (`tests/mutation_replay/local_sync*.rs` is four files: `local_sync`, `local_sync_inspection_spec`, `local_sync_live_prerequisites`, `local_sync_quality`). Every test and helper name in E-1 to E-11 and in the Re-run list exists. Names the plan introduces (`PrimaryActivityMap`, `Basis`, `IndexOutcome`, `CallScope`, `PendingRow`, `PreviewFacts`, `set_context_scan_hook`, `set_preview_facts_hook`, `Checkpoint::TransferStarted`, and the new tests) are absent, as they should be.

All other cited references checked within ~30 lines: 357.

## Tests that pin the checks the plan keeps (Task 0)

Tags: `[read]` means I read the test and it asserts the check; `[name]` means chosen by name and call site only, so unsure. Library tests are `cargo test --lib <module path>::<name>`; the `tests/remote_synchronization.rs` names are the case names registered in its `CASES` table (each case runs in its own child process over SSH).

1. **Push-absence before a merge.** Code: `push_absence` (`src/repository/remote/sync.rs:7161`), `synchronization_push_absence_boundary` (6926), used at ~6280 (before any local update) and ~6701; republish evidence at ~7396.
   - `released_checkpoint_descendant_after_old_push_intent_opens_a_continuation_attempt`, `src/repository/remote/sync_tests.rs:5253` `[read]`: the boundary is `Ambiguous` while the newest publication attempt is unverified and `Deleted` once it is verified.
   - `an_unreturned_first_push_with_no_publication_evidence_is_retryable_not_ambiguous`, `src/repository/remote/sync_tests.rs:5792` `[read]`: table over row kinds, `Ambiguous` or none.
   - `a_confirmed_abandon_answers_the_push_so_a_new_synchronization_is_not_ambiguous`, `src/repository/remote/sync_tests.rs:5931` `[read]`: ambiguous before the abandon, not after.
   - `a_synchronization_refused_for_another_s_unanswered_push_names_it`, `src/repository/mutation/bind/sync_tests.rs:133` `[read]`: the sync is refused `recovery_required` naming the unanswered push.
   - `a_comment_whose_publication_meets_another_s_unanswered_push_names_it`, `src/repository/mutation/bind/comment_tests.rs:26` `[name]`: same refusal for a comment's publication.
   - `sync_ambiguous_push_restart_is_reconciliation_not_a_fresh_push`, `src/repository/remote/reservation_tests.rs:2747` `[name]`: the reservation side of a restart after an ambiguous push; unsure it reaches `push_absence`.
   - SSH, indirect, `[name]`: `merge_candidate_ambiguous_acceptance_exact_restart` (`tests/remote_synchronization.rs:1862`), `merge_candidate_continuation_ambiguous_acceptance_verifies_same_attempt` (1438).
2. **Head and status check before a local update and before a push.** Code: `inspect_synchronization_local` (`sync.rs:4121`): `inspected_target` (worktree identity plus `require_clean_target`) against `local_target`'s OID under the repository lease, `ExternalChange` if they differ. Call sites before local updates: ~6013, ~6187, ~6284, ~6306, ~6476, ~6518, ~6692, ~7544, ~7694, ~7713; before the push: ~6871, ~6906, ~6976, ~8051, ~8099; in the fetch window (Task 18 drops these): ~7934, ~7944, ~7954. No test calls `inspect_synchronization_local` directly.
   - `dirty_preflight_preserves_bytes_index_and_refs`, `src/repository/remote/sync_tests.rs:177` `[read]`: a dirty primary answers `WorktreeNotClean` and nothing changes.
   - `wrong_branch_and_unmaterialized_context_are_nonmutating`, `src/repository/remote/sync_tests.rs:195` `[read]`: a wrong branch or missing context answers `TargetNotMaterialized`.
   - `foreign_merge_rebase_and_cherry_pick_metadata_are_refused_without_adoption`, `src/repository/remote/sync_tests.rs:219` `[read]`: `local_target` answers `WorktreeConflicted`.
   - `one_expected_old_safe_fast_forward_updates_worktree_and_ref`, `src/repository/remote/sync_tests.rs:759` `[read]`: a fast-forward from a stale expected head answers `ExternalChange`.
   - `primary_dirty_conflicted_preservation` and `context_dirty_wrong_detached_preservation`, `tests/remote_synchronization.rs:604` and `:650` `[read]` (SSH): dirty, conflicted, wrong-branch and detached targets are refused before any connection authenticates, with the files unchanged.
   - `divergence_recheck_refuses_configuration_and_tracking_changes_before_mutation`, `src/repository/remote/sync_tests.rs:525` `[name]`: a recheck before mutation, but for configuration and tracking; unsure it covers head or status.
   - `receiver_expected_old_divergent_race`, `tests/remote_synchronization.rs:2276` `[name]` (SSH): a push-time expected-old race; unsure.
   - Gap (unsure): I found no test that moves the head or status between the inspection and the update or push, which is the `current != observed` branch at `sync.rs:4131`. Task 18 removes inspections, so the task that does it should add one row if it relies on that branch.
3. **Re-confirmation at `src/repository/promotion/integrate.rs` ~183.** Code: `if preview.expectation() != record.expected { return Err(L::Error::reconfirmation_required()) }` at 183-184, with the integration-conflict refusal at ~170 and the merge conflict at ~244, in `prepare_integration_for` (110).
   - `digest_is_recomputed_under_lease_before_any_write_including_identity_config`, `src/repository/promotion/integrate_tests.rs:271` `[read]`: new context content after consent answers `ReconfirmationRequired` with the tree untouched.
   - `stale_confirmation_is_refused_without_writes`, `src/repository/promotion/integrate_tests.rs:578` `[name]`.
   - `post_child_new_path_reconfirms_but_primary_only_merge_preserves_consent`, `src/repository/promotion/run_tests.rs:621` `[name]`.
   - `collaborator_change_after_closing_recorded_abandons_with_nothing_written`, `src/repository/closure/run_tests.rs:782` `[read]`: a collaborator's change after the record exists answers `ReconfirmationRequired` and abandons.
   - `pre_checkpoint_refusals_abandon_with_nothing_written_replay_and_a_new_id_works`, `src/repository/closure/run_tests.rs:498` `[read]`: the primary moves after confirmation; unsure it is line 183 and not the earlier consent refusal in `promote_inner`.
   - `a_changed_identity_after_the_record_exists_abandons_as_reconfirmation_required`, `src/repository/closure/run_tests.rs:590` `[read]`: unsure for the same reason (the runner maps pre-integration consent refusals to this code).
   - `remote_primary_advance_from_closing_is_reconfirmation_required`, `src/repository/closure/checkpoint_tests.rs:1417` `[name]`.
   - `a_reset_that_leaves_the_context_contained_without_tracking_reconfirms`, `src/repository/closure/run_tests.rs:1703` `[name]`.
4. **A resumed lifecycle's re-check.** Code: `promote_inner` re-previews a stored record (`preview::<L>` at ~659, ~679, ~710) and `prepare_integration_for` re-observes and revalidates under the lease (`L::prepare` ~119, `L::revalidate` ~159); `resume_written` (~129) skips consent but keeps the primary's safety checks.
   - `collaborator_change_after_closing_recorded_abandons_with_nothing_written`, `src/repository/closure/run_tests.rs:782` `[read]`: stopped at `ClosureAfterClosingRecorded`, then re-run on a reopened service; the re-check refuses and the record is `Abandoned`.
   - `integrating_resume_refuses_a_primary_moved_since_the_record`, `src/repository/closure/checkpoint_tests.rs:1379` `[read]`: a resume at the recorded base succeeds; with the primary moved it answers `RecoveryRequired` and keeps the record.
   - `remote_primary_advance_from_closing_is_reconfirmation_required`, `src/repository/closure/checkpoint_tests.rs:1417` `[name]`.
   - `f6_rechecks_a_dirty_context_under_lease_before_deletion`, `src/repository/promotion/run_tests.rs:2239` `[name]`: the cleanup-side re-check.
   - `cleanup_precheck_preserves_dirty_untracked_and_moved_before_remote_delete`, `src/repository/promotion/run_tests.rs:5071` `[name]`: P-1's cheap check before the remote delete; Task 22 re-runs it.
   - `local_sync_configured_remote_closure_requires_explicit_second_sync_and_resume`, `src/repository/closure/run_tests.rs:1027` `[name]`.
   - Gap: nothing pins "a resumed attempt rechecks under the lease without a consent preview" yet. That is Task 21's new test `a_resumed_attempt_rechecks_under_the_lease_without_a_consent_preview`.

## Task log

Task 0: started.
