---
title: "Wave 03 F2: Request Replay, Confirmation And Shared Mutation Bridges"
date: 2026-10-08
status: approved
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M4ERD7MTN98DWEFYQPNFGNQT"
---

# Wave 03 F2: Request Replay, Confirmation And Shared Mutation Bridges

## Purpose And Authority

Give both front ends one headless way to change Manyhands state, so neither
the CLI nor the desktop decides for itself whether a retry is safe, whether
consent still holds, or what a half-finished operation left behind. The
Cycle adds a mutation boundary to the library: request IDs, input matching,
expected observations, two-phase confirmation, progress, cancellation and
one result envelope for every outcome. It also adds the narrow mutations the
baseline lacks. It delivers no CLI verb and no desktop screen.

This is Cycle F2 of [Wave 03](../Waves/wave-03-dogfooding.md#f2-request-replay-confirmation-and-shared-mutation-bridges),
tracked by ticket `01M4CC0VMR8HSPZXQ1WX41GWVK`. The
[CLI contract](../RFC/cli-contract.md),
[Git workflow](../RFC/git-workflow-and-conflict-recovery.md),
[runtime](../RFC/application-runtime-and-polling.md),
[authentication](../RFC/authentication-and-credential-handling.md),
[canonical schema](../RFC/canonical-content-and-comment-schema.md),
[ticket relationships and short codes](../RFC/ticket-relationships-and-short-codes.md),
[repository/index](../RFC/repository-index-persistence-and-refresh.md) and
[test strategy](../RFC/test-and-compatibility-strategy.md) RFCs are
authoritative. The
[design](../plans/2026-10-08-wave-03-foundation-02-replay-confirmation-and-bridges-design.md)
and [implementation plan](../plans/2026-10-08-wave-03-foundation-02-replay-confirmation-and-bridges-implementation.md)
accompany this document. The product owner approved all three on
2026-10-10. Implementation was a separate gate; it was authorized
afterwards and its outcome is recorded under
[Implementation Outcome](#implementation-outcome-2026-10-13).

## Entry Evidence And Dependency Boundary

- Reused the existing ticket worktree and branch; no replacement was created.
- `git fetch origin` on 2026-10-08 observed `origin/main` and local `main`
  both at `6cf5d7fcf0663f1d383a7f2f50980fa3ba8ab5e8`, the merge of F1. The
  ticket branch, three ticket checkpoints never pushed, was rebased onto it
  without conflicts, from `dc69406` to
  `ff1c8a48d94a91dfbcdc64bc417a3eefcbf0e4be`. The worktree was clean.
- **Wave 02 dependency.** The Wave requires Cycles 01 to 05 on main for F2.
  All five are closed and on main. Cycle 06 landed on 2026-10-09; the
  synchronization bindings refuse the divergence it merges, see decision
  12. Cycles 07 to 10 are
  not started.
- **Wave 03 dependency.** F1 merged to main as pull request 13 on
  2026-10-08.
- **Entry-gate item 3**, the API audit: the design's mutation-surface audit
  refreshes it at `6cf5d7f` by reading source. It ran no Rust command.
- **Entry-gate item 7**, the required Rust checks at the implementation
  baseline: the first step of the plan. Not yet run for this Cycle. The same
  checks passed on the F1 branch head, which main now contains, on
  2026-10-08.
- **The journal defect found during planning is fixed.** Many ordinary
  rejections left a journal row pending, and a pending row makes every
  other operation on that repository fail with `RecoveryRequired`. Defect
  ticket `01M4EWN2DK3MY6H4GBYDYXF6QH` reproduced it, fixed it and merged
  to main as pull request 15 on 2026-10-10. Part B is no longer blocked by
  it. The design's "The Journal Defect" says what the fix does and does not
  cover.
- **Refresh of 2026-10-10.** `origin/main` at `f87ce81` was merged into
  this branch without conflicts. Since `6cf5d7f` main gained Wave 02 Cycle
  06 (merge and conflict recovery), the index fix
  `01M4GD0KKXW684QBA49F6EX3WE` and the journal fix. The three documents
  were updated for them by their author; the updates are not reviewed, and
  nothing was run. Cycle 06 changes what a synchronization does; decision
  12 keeps F2's bindings clean-only.
- Seven defect and follow-up tickets raised from F1 are open. Two touch code
  F2 changes: `01M4EHGE4BXGPMCWWA1S1QR0JW` (index refresh atomicity) and
  `01M4EHGE9TMR99VYZC184J9XEC` (discovery entry caps). Neither blocks F2.

Before implementation, fetch and merge again, repeat the ancestry check and
re-check the audit against any library change that landed in between.

## Scope

- **Mutation boundary.** `prepare` and `execute` over a closed set of typed
  mutations; `show_request`, `cancel_request` and `resume_operation`.
- **Request identity and replay.** A caller-supplied request ULID bound to
  command, repository, target and semantic input. A retry returns the
  effects already completed and does only what remains. Reuse with different
  input is rejected. Request records hold a digest, never a body or a
  secret.
- **Expected observations.** F1's observation token checked on every write to
  an existing resource; a token for an absent path; observed repository
  state bound into confirmations.
- **Two-phase confirmation.** A persisted, ten-minute preview with an effect
  summary, for `repo create`, `repo enable`, `repo remove`,
  `repo identity-set`, `remote remove`, `key delete`, `host approve`,
  `host replace`, `document repair`, `ticket repair` and
  `operation abandon`. A preview creates nothing, including for a
  repository root that does not exist yet.
- **Bindings** for the twenty-eight baseline mutations the design lists
  (decision 9), and `operation abandon` (decision 13).
- **Result model for mutations.** Mutation envelopes, the result codes and
  recovery actions mutations need, one rule for outcome and failure class,
  and a complete mapping from every domain outcome and error.
- **Progress and cancellation.** A typed progress event and sink; a
  cancellation token and a cross-process cancellation request; cancellation
  honored at the approved safe points.
- **Bridges.**
  - Confirmed local identity: a standalone setter, and identity inside the
    create and enable previews.
  - Host approval and replacement through the existing scoped transport
    verification, and a read of what a host presents.
  - Folder creation under `docs/`, and its listing (decision 4).
  - Repair and adoption of nonconforming documents and tickets.
- **Ticket relationships.** Create and save accept `deps` and `parent`, write
  them in canonical form and reject a cycle before any write; the check is
  also a public read. Create generates the short code. Explicit short-code
  assignment. Repository-local initials. The optional repository prefix at
  create and enable.
- **Comment author.** `submit_comment` writes `created_by`.
- **Published contract.** Schemas and golden fixtures for everything above.

## Explicit Exclusions And Downstream Obligations

This Cycle parses no command, prints no human output, chooses no exit number
and prompts for nothing; the CLI Cycles own those. It builds no desktop
confirmation, timer or worker; the desktop Cycles do. It adds no thread and
no scheduling.

It adds no Git lifecycle algorithm. Synchronization, transport and
reservations are used as Wave 02 built them.

Not delivered, and who owns each:

- **The `comment add` binding**: the first of C4 or D4, as a shared-library
  change, with Wave 02 Cycle 07's checkpoint-then-publish operation.
- **Polling policy and `poll once` bindings**: the first of C5 or D6.
- **Promotion and closure bindings**: the first of C5 or D5, under track
  rule 6.
- **Conflict resolution and divergent synchronization**: C4 and D5. Wave 02
  Cycle 06 has since landed and `synchronize_remote` now merges a divergent
  remote itself. F2's two bindings ask it to refuse divergence instead
  (decision 12); C4 and D5 turn merging on when they bind
  `conflict resolve`.
- **Per-command input schemas**: the CLI Cycle that delivers each verb.
- **Native execution on Windows and macOS**: see decision 11.
- **Changing the short-code prefix or code length after enablement.** No RFC
  names a command for it. They are edited in the tracked configuration file.

Gaps found during planning that belong to later Cycles and are recorded here
so they are not lost:

- **Republishing a remotely deleted item branch.** The Wave gives C4 its
  confirmation, but no operation republishes one and no Wave 02 Cycle is
  named for it.
- **A CLI verb that returns an observation for an absent path.** F2 adds the
  read; `document move` needs it; the CLI RFC's taxonomy has no verb for
  it. C3's planning decides.
- **A nonconforming file on primary blocks comments** in every editing
  context cut from it, because `submit_comment` refuses a worktree that
  holds one. Existing behavior; C4 and D4 will meet it.

## Required Mutation Contract

1. **One request, one intent.** A request ID used again with the same
   command, repository, target and input continues that request. Used again
   with anything different, it is rejected and nothing runs.
2. **A retry never repeats an effect.** A finished request returns its stored
   result without touching the repository. An unfinished one continues only
   if nobody else has changed its paths since it was accepted, or they
   already hold its commit.
3. **A request that did nothing leaves nothing.** A local request that ends
   with nothing in flight leaves no request record and no pending journal
   row; the same or another request can follow at once. Whether something
   is in flight is read from the operation's journal row, never guessed
   from the error returned. A row the library closed as `rejected` counts
   as no row. Two existing exceptions are stated in the
   design: a failed index refresh or rebuild, and a synchronization that
   stopped after it reserved.
4. **Git and the file system outrank the records.** A stored result that
   names a commit no longer reachable is not returned. After the database is
   lost, a retry is decided from what is on disk.
5. **Writes to existing resources need a current observation.** A stale one
   is `external_change`; nothing is written. There is no override. A
   request's own earlier attempt does not make its observation stale, and a
   request for the state that already exists is a no-op, not a conflict.
6. **A preview changes nothing** beyond one record in the application
   database. It initializes no repository, creates no directory, writes no
   configuration and contacts no network.
7. **A confirmation binds what was previewed**: command, target, input and
   observed state. It is accepted once, by one request, within ten minutes,
   and only if a fresh observation still matches. The observation is
   compared again on any retry whose operation has no recorded step, unless
   the command's end state already holds.
8. **No lock is created.** The boundary takes no lease of its own and keeps
   none between calls. The repository lease, the reservation and the three
   journals remain the only authorities.
9. **Records hold no content.** No request record, confirmation record,
   progress event, envelope message or recovery action contains a Markdown
   body, a source, a passphrase, a private key, a credential or server text.
10. **Partial is not failure and not success.** Any durable effect followed
    by something unfinished is reported as `partial`, with what completed
    and a recovery action. Two partial results carry no action, by
    decision: a request ID reused with changed input over a request that
    made an effect, where the original request is what goes on, and a key
    generation whose files were retained for inspection, which is final.
11. **Cancellation is honored only at a safe point** and reports what had
    completed. Nothing is rolled back and no deadline is promised.
12. **Identity is never invented or made global.** Host trust is written only
    after the host presents the approved key on an authenticated connection.
13. **An item or comment ID is never generated on the caller's behalf.**
    Adoption uses the ID in the confirmed input. A short code is written
    once.

## Acceptance And Exit Evidence

The third and fourth columns were added on 2026-10-13, at the end of
implementation. Each test named was opened and its assertions read against
its row, at code head `765e43a`; nothing was run for this table.

| Contract | Required evidence | Held by | Status |
| --- | --- | --- | --- |
| Replay | For a ticket save and a clean synchronization against a real SSH remote: a retry after a lost result returns the same effects and makes no second commit or push, including when another request saved the item in between; a retry with changed input is rejected with nothing run. | `tests/mutation_replay/reenter.rs::a_retry_after_lost_output_reports_the_commit_and_makes_none`, `tests/mutation_replay/reenter.rs::a_retry_after_lost_output_and_another_save_reports_the_first_commit`; `tests/mutation_replay/execute.rs::a_finished_request_reused_with_changed_input_is_a_mismatch`; `tests/mutation_remote.rs::a_push_whose_result_was_lost_is_reconciled_by_the_same_request`, `tests/mutation_remote.rs::the_same_request_id_for_another_target_is_a_mismatch` | **Partly.** The save half is held whole. The synchronization half is held for the plain lost push and for changed input. "Another request saved the item in between" has no test on the synchronization side: when only the local branch moved after a lost push the built answer is `merge_required`, a deferred minor, and nothing pins it. The neighbouring case, a remote that has since been built on, is pinned by `tests/mutation_remote.rs::a_lost_push_the_remote_has_since_built_on_is_merge_required_and_leaves_nothing_of_a_merge`. |
| Interruption | With a failure injected at each existing fault point of a save, the retry completes the remainder and creates no duplicate commit, branch, worktree or item. | `tests/mutation_replay/reenter.rs::a_retry_completes_a_save_interrupted_at_each_fault_point`, `tests/mutation_replay/reenter.rs::a_retry_completes_a_save_whose_process_was_killed_at_each_fault_point`, `tests/mutation_replay/reenter.rs::two_processes_submitting_the_same_new_request_make_one_commit` | **Partly.** Four fault points are driven through the boundary, by ruling: `BeforeItemWrite`, `AfterOwnedWriteBeforeLifecyclePersistence`, `BeforeCheckpointCommit` and `BeforeIndexTransactionCommit`, each with and without an existing context, in-process and with the process killed. The two fault points of making the context are not injected by a boundary test. What is asserted is the commit count on the item's branch, its tip and the file; a second branch, worktree or item is not counted. |
| Required outcomes | Every row of the design's required-outcomes table has a test that produces the sequence and asserts the outcome. | Rows 1 and 2: `tests/mutation_replay/execute.rs::a_finished_request_is_answered_from_its_record`, `tests/mutation_replay/execute.rs::a_finished_request_whose_commit_is_unreachable_requires_recovery`, `tests/mutation_replay/execute.rs::a_finished_request_reused_with_changed_input_is_a_mismatch`. Rows 3 to 10: `tests/mutation_replay/reenter.rs::a_retry_after_lost_output_reports_the_commit_and_makes_none`, `tests/mutation_replay/reenter.rs::a_retry_after_lost_output_and_another_save_reports_the_first_commit`, `tests/mutation_replay/reenter.rs::a_retry_finishes_a_request_that_stopped_after_its_write`, `tests/mutation_replay/reenter.rs::a_retry_completes_a_save_interrupted_at_each_fault_point`, `tests/mutation_replay/reenter.rs::unrelated_commits_do_not_stop_a_retry`, `tests/mutation_replay/reenter.rs::a_foreign_commit_to_the_file_of_a_request_in_flight_is_an_external_change`, `tests/mutation_replay/reenter.rs::a_retry_after_the_lost_output_of_a_save_that_changed_nothing_is_a_no_op`, `tests/mutation_replay/reenter.rs::a_create_retried_after_the_initials_and_the_prefix_changed_keeps_its_short_code`. Rows 11 and 12: `tests/mutation_replay/execute.rs::a_rejected_request_leaves_nothing_and_its_id_can_be_used_again`, `tests/mutation_replay/execute.rs::an_error_after_the_write_is_partial_and_leaves_the_request_accepted`. Row 13: `tests/mutation_replay/reenter.rs::two_processes_submitting_the_same_new_request_make_one_commit`. Rows 14 and 15: `tests/mutation_replay/cache_loss.rs::a_completed_create_retried_after_the_cache_is_lost_is_already_applied`, `tests/mutation_replay/cache_loss.rs::a_completed_save_retried_after_the_cache_is_lost_is_already_applied`, `tests/mutation_replay/cache_loss.rs::a_write_that_was_not_committed_is_an_external_change_after_the_cache_is_lost`. Rows 16 to 18: `tests/mutation_confirmation.rs::a_retry_before_any_effect_observes_again`, `tests/mutation_confirmation.rs::a_repo_remove_whose_result_was_lost_is_a_no_op_and_not_external_change`, `tests/mutation_confirmation.rs::an_accepted_consent_is_honored_after_its_ten_minutes_for_repo_enable`, `tests/mutation_confirmation.rs::an_accepted_consent_is_honored_after_its_ten_minutes_for_repo_create`. Row 19: `tests/mutation_remote.rs::a_push_whose_result_was_lost_is_reconciled_by_the_same_request`. Row 20: `tests/mutation_replay/key_generation.rs::a_generation_interrupted_after_its_pair_was_written_is_registered_by_its_retry`. Row 21: `tests/mutation_confirmation.rs::a_deletion_interrupted_after_the_private_file_is_removed_is_partial_and_its_retry_completes` | Held. The paths on which a row is not met are the design's known limits, each named there with its row. |
| Nothing left behind | After each kind of rejection of a local request before an effect, no request record and no pending journal row exists, and a different request on the same repository succeeds. The two stated exceptions are each pinned by a test. | The refusal rule of the conformance table, for all thirty commands: `tests/support/conformance.rs`, run by `tests/mutation_replay/conformance.rs::every_command_of_this_target_keeps_the_rules_of_the_boundary` and its namesakes in `tests/mutation_confirmation/conformance.rs`, `tests/mutation_bridges/conformance.rs` and `tests/mutation_remote.rs`; `tests/mutation_replay/execute.rs::a_rejected_request_leaves_nothing_and_its_id_can_be_used_again`, `tests/mutation_replay/execute.rs::a_change_between_the_boundary_s_check_and_the_domain_s_is_an_external_change`. Exceptions: `tests/mutation_replay/index.rs::a_failed_refresh_or_rebuild_is_an_error_that_only_its_own_retry_clears`; `tests/mutation_remote.rs::a_divergent_remote_is_merge_required_and_holds_its_reservation` | Held. |
| Cache loss | With the database deleted between attempt and retry and the repository registered again: a create, a save and a synchronization each reach the intended state once; a save that died before its checkpoint is reported as an external change and a fresh save commits it; a confirmed command asks for a new preview; no result names a commit that is not found. | `tests/mutation_replay/cache_loss.rs::a_completed_create_retried_after_the_cache_is_lost_is_already_applied`, `tests/mutation_replay/cache_loss.rs::a_completed_save_retried_after_the_cache_is_lost_is_already_applied`, `tests/mutation_replay/cache_loss.rs::a_write_that_was_not_committed_is_an_external_change_after_the_cache_is_lost`, `tests/mutation_replay/cache_loss.rs::a_file_someone_else_changed_is_an_external_change_after_the_cache_is_lost`; `tests/mutation_confirmation.rs::a_confirmation_is_not_found_once_the_cache_is_lost`; `tests/mutation_remote.rs::a_synchronization_retried_after_the_cache_is_lost_is_blocked_until_each_piece_is_restored`, `tests/mutation_remote.rs::a_synchronization_retried_once_everything_is_restored_is_current_and_pushes_nothing`; the bridges in `tests/mutation_bridges/cache_loss.rs` (three tests) and `tests/mutation_bridges/repair.rs::a_repair_lost_with_the_index_is_made_again_under_the_same_id_in_the_same_context`; `tests/mutation_replay/execute.rs::a_finished_request_whose_commit_is_unreachable_requires_recovery` | Held. The last clause is held by a test that is not a cache-loss test: after a loss no stored result exists to name a commit. |
| Observations | A stale token on save, move, slug assignment and repair writes nothing. A document save or move without a source observation is rejected before any write. An uncommitted edit on primary stops a first save without creating a context. | `tests/mutation_replay/execute.rs::a_late_caller_asking_for_what_is_written_and_not_committed_is_an_external_change`, `tests/mutation_replay/execute.rs::a_change_between_the_boundary_s_check_and_the_domain_s_is_an_external_change`, `tests/mutation_replay/execute.rs::an_uncommitted_edit_on_primary_is_not_saved_over`; `tests/mutation_replay/documents.rs::a_move_checks_the_token_of_each_path`, `tests/mutation_replay/documents.rs::a_save_or_a_move_without_the_source_observation_is_invalid_input`, `tests/mutation_replay/documents.rs::an_uncommitted_edit_of_a_document_on_primary_is_not_saved_over`; `tests/mutation_replay/slug_assign.rs::a_stale_token_is_an_external_change_and_a_closed_ticket_is_refused`; `tests/mutation_bridges/repair.rs::a_stale_token_is_an_external_change_at_prepare` | Held. |
| Confirmation | A stale preview, an expired one, one used by another request, one presented with changed input and a missing one are each rejected with their own code and no effect. An accepted one is honored on retry after its expiry; a retry whose operation has no recorded step observes again; a retry of a completed command is a no-op. A preview of `repo create` leaves no directory, repository or registration. | `tests/mutation_confirmation.rs::a_stale_preview_of_repo_enable_is_external_change`, `tests/mutation_confirmation.rs::a_stale_preview_of_repo_create_is_external_change`, `tests/mutation_confirmation.rs::a_confirmation_lasts_ten_minutes_and_a_clock_set_back_does_not_extend_it`, `tests/mutation_confirmation.rs::a_second_request_presenting_an_accepted_confirmation_is_confirmation_used`, `tests/mutation_confirmation.rs::another_input_another_repository_and_another_command_are_confirmation_mismatch`, `tests/mutation_confirmation.rs::a_confirmation_that_no_record_holds_is_confirmation_not_found`, `tests/mutation_confirmation.rs::an_accepted_consent_is_honored_after_its_ten_minutes_for_repo_enable`, `tests/mutation_confirmation.rs::an_accepted_consent_is_honored_after_its_ten_minutes_for_repo_create`, `tests/mutation_confirmation.rs::a_retry_before_any_effect_observes_again`, `tests/mutation_confirmation.rs::a_repo_remove_whose_result_was_lost_is_a_no_op_and_not_external_change`, `tests/mutation_confirmation.rs::a_key_delete_whose_result_was_lost_is_a_no_op_and_not_external_change`, `tests/mutation_confirmation.rs::a_preview_of_repo_create_creates_nothing`; the stale-preview rule of the conformance table, for every confirmed command | Held. |
| No lock | With a preview outstanding, every other operation on the repository proceeds. | `tests/mutation_confirmation.rs::prepare_takes_no_lease_of_the_repository_it_previews`, `tests/mutation_confirmation.rs::a_save_and_a_refresh_succeed_on_a_repository_whose_preview_is_outstanding` | Held. |
| Result mapping | Every domain outcome and error variant has a code, enforced by exhaustive matches. A golden envelope exists for every bound command and every result code a fixture can produce; the rest are pinned by a table test. | The table tests of `src/repository/mutation/outcome_tests.rs`; `tests/mutation_contract.rs::every_bound_command_has_a_golden`, `tests/mutation_contract.rs::every_result_code_has_a_golden_or_a_reason_no_fixture_can_produce_it`, `tests/mutation_contract.rs::every_result_code_has_a_mutation_golden_or_a_reason_none_has_it`, `tests/mutation_contract.rs::the_fixture_directories_are_exactly_the_registered_cases`, `tests/mutation_contract.rs::every_data_root_describes_the_data_of_a_golden_and_every_data_has_one_root`, `tests/mutation_contract.rs::every_schema_is_reachable_from_a_published_root` | Held. Four codes have no mutation golden, each listed in the test with its reason: `remote_unavailable`, which no fixture can produce and whose mapping is pinned by the table test alone, and `invalid_id`, `public_key_unavailable` and `authority_not_found`, which only reads return and which have read goldens. `item sync`, `repo sync`, `host approve` and `host replace` have goldens of refusals and stops only; a request of theirs that did its work is checked against its schema in the SSH cases. |
| Partial results | For each binding family, a failure injected after the authoritative step yields `partial`, the completed effects and a recovery action. | `tests/mutation_replay/execute.rs::an_error_after_the_write_is_partial_and_leaves_the_request_accepted`, `tests/mutation_replay/execute.rs::an_error_after_the_commit_is_partial_with_that_commit`, `tests/mutation_replay/execute.rs::a_failure_after_the_checkpoint_is_partial_with_discovery_pending`; `tests/mutation_replay/remotes.rs::a_failure_after_the_remote_was_added_is_partial_and_says_its_hand_off_is_pending`, `tests/mutation_replay/remotes.rs::a_failure_after_the_selection_s_commit_is_partial_and_its_retry_reports_that_commit`; `tests/mutation_replay/key_generation.rs::a_generation_interrupted_after_its_pair_was_written_is_registered_by_its_retry`; `tests/mutation_confirmation.rs::a_create_interrupted_after_initialization_is_partial_and_its_retry_completes`, `tests/mutation_confirmation.rs::a_failed_hand_off_after_the_commit_is_partial_with_the_commit`, `tests/mutation_confirmation.rs::a_removal_interrupted_after_the_remote_is_deleted_is_partial_and_its_retry_completes`, `tests/mutation_confirmation.rs::a_deletion_interrupted_after_the_private_file_is_removed_is_partial_and_its_retry_completes`; `tests/mutation_remote.rs::a_published_commit_whose_index_hand_off_failed_is_partial_and_its_retry_pushes_nothing`; `tests/mutation_bridges/folder.rs::a_refresh_that_fails_after_the_folder_is_made_is_partial_and_any_refresh_finishes_it`; `tests/mutation_bridges/repair.rs::a_repair_stopped_after_its_write_is_completed_by_its_retry_with_the_same_id`; `tests/mutation_remote/host.rs::a_call_that_fails_after_the_pin_was_stored_says_so_and_its_retry_is_a_no_op` | Held for every family in which a failure can follow an effect. Two partial results that are not of this kind carry no recovery action: a `request_mismatch` against a request that made an effect, and a key generation retained for inspection; see contract item 10. |
| Progress and cancellation | Events are emitted in order with no content. Cancellation before acceptance has no effect and no record; cancellation at a held synchronization safe point, by token and by request from a second thread, stops there and reports completed effects; cancellation after the outcome is classified returns the completed result. | `tests/mutation_replay/progress.rs::a_local_command_reports_that_it_was_accepted_and_what_it_is_about_to_do`, `tests/mutation_replay/progress.rs::a_request_entered_again_reports_its_stage_and_not_that_it_was_accepted`; `tests/mutation_replay/execute.rs::a_request_cancelled_before_it_is_accepted_does_nothing`; `tests/mutation_remote.rs::the_events_of_a_synchronization_arrive_in_safe_point_order_and_hold_nothing_private`, `tests/mutation_remote.rs::a_token_cancelled_before_execute_does_nothing`, `tests/mutation_remote.rs::a_token_cancelled_from_the_sink_stops_at_the_first_safe_point`, `tests/mutation_remote.rs::a_cancellation_requested_while_the_transport_is_blocked_is_honored_when_it_returns`, `tests/mutation_remote.rs::a_cancellation_during_the_push_reports_the_push`, `tests/mutation_remote.rs::a_token_cancelled_once_the_push_has_returned_does_not_unsay_the_publication`, `tests/mutation_remote.rs::a_published_commit_whose_index_hand_off_failed_is_partial_and_its_retry_pushes_nothing` | Partly held. Everything up to the last clause is held. The last clause is not held as written: after the outcome is classified, the answer is `cancelled` with the publication reported, by token and by request alike, and not the completed result. The product owner read this row as not held on 2026-10-13 and accepted the built answer: cancelling is a choice the user is entitled to make, and the answer never understates a push. |
| Identity bridge | Set, no-op, an injected failure that leaves the configuration untouched, and identity inside the create and enable previews, including a root that does not exist. Global configuration is byte-identical afterwards. | `tests/mutation_bridges/identity.rs::an_identity_is_written_to_the_repository_and_to_no_other_configuration`, `tests/mutation_bridges/identity.rs::what_the_repository_already_holds_is_a_no_op_and_gets_no_confirmation`, `tests/mutation_bridges/identity.rs::a_failure_after_the_first_key_leaves_the_file_as_it_was_and_nothing_of_the_request`, `tests/mutation_bridges/identity.rs::a_write_that_fails_part_way_never_touched_the_configuration_file`; `tests/mutation_confirmation.rs::a_supplied_identity_is_previewed_and_written_to_the_repository_alone`, `tests/mutation_confirmation.rs::repo_enable_with_a_global_identity_names_its_source_and_writes_none`, `tests/mutation_confirmation.rs::repo_create_with_a_global_identity_says_it_will_be_written_and_writes_it`, `tests/mutation_confirmation.rs::a_preview_of_repo_create_creates_nothing` | Held. The row first asked for a failure "that restores the configuration". The write is now one rename under Git's lock, so nothing is ever restored; the row is reworded to what is proven. |
| Host approval | Observe, approve, replace, a host presenting a different key, and a locked key. Server-side push count and every local ref are unchanged. | `tests/mutation_remote/host.rs::observing_a_host_reports_what_it_presents_and_its_pin_and_pins_nothing`, `tests/mutation_remote/host.rs::approving_the_observed_fingerprint_stores_the_pin_and_publishes_nothing`, `tests/mutation_remote/host.rs::a_rotated_host_key_is_replaced_by_naming_the_old_and_the_new_fingerprint`, `tests/mutation_remote/host.rs::a_fingerprint_the_host_does_not_present_is_host_mismatch_and_pins_nothing`, `tests/mutation_remote/host.rs::an_approval_with_a_locked_key_and_no_passphrase_is_unlock_required_and_pins_nothing` | Held. |
| Folder creation | Create, no-op, an occupied path, a path past discovery's limits, and a symbolic link that would lead outside `docs/`. No commit and a clean worktree. The folder is listed after the refresh. | `tests/mutation_bridges/folder.rs::a_folder_is_made_with_the_directories_above_it_and_nothing_else`, `tests/mutation_bridges/folder.rs::a_directory_that_is_there_is_a_no_op_and_a_file_occupies_its_path`, `tests/mutation_bridges/folder.rs::a_folder_that_would_cross_a_limit_of_discovery_is_refused_and_nothing_is_made`, `tests/mutation_bridges/folder.rs::a_folder_seventeen_levels_deep_is_refused_and_sixteen_is_made`, `tests/mutation_bridges/folder.rs::nothing_is_made_through_a_symbolic_link_inside_or_outside_the_repository` | Held. The symbolic-link test runs on Unix only. |
| Repair and adoption | Adoption with a supplied ID; retry and cache loss keep that ID; an ID held by another item writes nothing and creates no context; closure fields cannot be added or removed; a closed ticket is repairable. | `tests/mutation_bridges/repair.rs::a_marker_only_document_is_adopted_with_the_id_its_source_carries`, `tests/mutation_bridges/repair.rs::a_repair_stopped_after_its_write_is_completed_by_its_retry_with_the_same_id`, `tests/mutation_bridges/repair.rs::a_repair_lost_with_the_index_is_made_again_under_the_same_id_in_the_same_context`, `tests/mutation_bridges/repair.rs::an_ambiguous_identity_is_refused_and_nothing_is_made`, `tests/mutation_bridges/repair.rs::a_closed_ticket_is_repaired_keeping_its_closure_and_no_repair_closes_or_reopens`, `tests/mutation_bridges/repair.rs::a_refresh_and_a_rebuild_never_adopt` | Held. |
| Relationships | `deps` written sorted, unique and one per line; unknown metadata preserved; a dependency cycle, a parent cycle and a self-reference rejected before any context or record exists; an unknown target accepted and reported. | `tests/mutation_relationships.rs::a_create_writes_the_set_relationships_in_the_canonical_form`, `tests/mutation_relationships.rs::repeated_entries_are_removed_and_the_rest_sorted`, `tests/mutation_relationships.rs::other_unknown_keys_keep_their_values_and_order_and_new_keys_follow_them`, `tests/mutation_relationships.rs::a_dependency_that_closes_a_cycle_is_a_relationship_cycle_naming_its_members`, `tests/mutation_relationships.rs::a_parent_that_closes_a_cycle_is_a_relationship_cycle_naming_its_members`, `tests/mutation_relationships.rs::a_ticket_naming_itself_in_either_field_is_a_cycle_of_one`, `tests/mutation_relationships.rs::an_unknown_target_is_accepted_and_listed_as_unresolved`; `tests/mutation_replay/execute.rs::a_relationship_cycle_is_rejected_before_anything_exists`, `tests/mutation_replay/execute.rs::an_unknown_relationship_target_is_accepted_and_listed` | Held. |
| Short codes | Golden vectors at each length, computed independently; initials for one-word, non-ASCII and overridden names; a short code unchanged by rename, identity change and prefix change; assignment rejected for a ticket that has one. | `tests/canonical_foundation.rs::short_codes_match_the_independently_computed_vectors`, `tests/canonical_foundation.rs::initials_follow_the_vectors`, `tests/canonical_foundation.rs::normalize_initials_rejects_everything_else`; `tests/mutation_replay/execute.rs::a_create_composes_the_short_code_from_the_configuration_and_the_initials`, `tests/mutation_replay/execute.rs::a_short_code_is_unchanged_by_an_identity_change_and_by_a_prefix_change`; `tests/mutation_relationships.rs::a_short_code_is_unchanged_by_a_later_save_that_changes_the_title`; `tests/mutation_replay/slug_assign.rs::a_ticket_that_has_another_short_code_is_slug_already_assigned` | Held. The vectors are in `tests/fixtures/slug_v1/vectors.json`, which records how they were computed. |
| Comment author | A new comment carries `created_by`; a retry keeps it and its `created_at`. | `tests/local_authoring.rs::comment_file_names_the_checkpoint_identity_as_created_by`, `tests/local_authoring.rs::recovery_comment_retry_keeps_created_at_and_created_by`, `tests/local_authoring.rs::recovery_comment_retry_under_another_identity_keeps_the_first_as_created_by`; `tests/canonical_foundation.rs::a_comments_author_is_written_as_created_by_and_read_back_into_the_field` | Held. |
| Privacy | Sentinel bodies, passphrases and remote credentials appear in no column of the request tables or journals and in no envelope, preview or event. | `tests/mutation_replay.rs::the_request_stores_hold_none_of_a_request_s_content`, `tests/mutation_replay.rs::the_privacy_scan_finds_a_sentinel_in_every_request_store`; `tests/mutation_replay/key_generation.rs::no_record_journal_row_or_result_of_a_generation_holds_its_passphrase`; `tests/mutation_confirmation.rs::no_preview_and_no_record_holds_a_remote_s_password_or_a_body`, `tests/mutation_confirmation.rs::nothing_a_key_deletion_reads_of_the_key_files_reaches_an_envelope_or_a_store`; `tests/mutation_remote.rs::the_output_privacy_scan_passes_what_the_boundary_emits_and_fails_what_is_private`, `tests/mutation_remote.rs::the_events_of_a_synchronization_arrive_in_safe_point_order_and_hold_nothing_private`; `tests/mutation_bridges/identity.rs::no_credential_of_any_configuration_reaches_an_envelope_or_a_record`; `tests/mutation_bridges/folder.rs::no_path_outside_the_repository_reaches_an_envelope_an_event_or_a_record`; `tests/mutation_bridges/repair.rs::nothing_a_repair_reads_reaches_what_it_gives_or_keeps`; `tests/mutation_remote/host.rs::the_output_privacy_scan_passes_what_a_host_approval_emits_and_fails_the_key_path` | Held. Every golden also passes the sentinel scan. Two things are published or hashed on purpose and are stated in the design: the two key-file paths in the preview of `key delete`, and the path of an imported key in a salted digest. |
| Front-end independence | The library builds and the new test targets pass without the `desktop` feature and without a display. | The six new targets: `mutation_replay`, `mutation_confirmation`, `mutation_contract`, `mutation_bridges`, `mutation_relationships`, `mutation_remote`. | Held. Evidence is the gate; see the gate results below. |
| Regression | The four required Devenv checks and the CLI smoke test pass. Existing tests pass unchanged, except the ones the design names under "Changes To Existing Code". | The full gate. | Held. Evidence is the gate; see the gate results below. Existing tests this Cycle changed are named in the design and in the fix report: one serializer order test, the two-process and killed-process save tests widened for a truthful `busy` or index-pending first answer, and the read goldens that hold the two reworded messages and the renamed pin fields. |

### Gate results

Local gate on Linux (x86_64) through Devenv, 2026-10-13, at code commit
`765e43a` (the commits after it change documents only):

| Command | Exit | Result |
| --- | --- | --- |
| `cargo check --all-features --locked` | 0 | |
| `cargo fmt --check` | 0 | |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | 0 | |
| `cargo test --all-features --locked --no-fail-fast` | 0 | 2112 passed, 0 failed in 30 targets; the six SSH-harness targets passed 88, 25, 15, 63, 31 and 105 cases |
| `cargo run --locked --bin manyhands-cli` | 0 | |
| `cargo test --locked --test mutation_replay --test mutation_confirmation --test mutation_contract --test mutation_bridges --test mutation_relationships --test mutation_remote` (no `desktop` feature) | 0 | 655 passed, 0 failed in 5 targets; `mutation_remote` passed 88 SSH cases |

Native CI (`build.yml`, five targets: x86_64 and aarch64 Linux, x86_64 and
aarch64 Windows, aarch64 macOS) passed on all five at `b5698fb` (run
38316228822), `0789ecb` (38330668350), `815f104` (38342297227), `46712e4`
(38358933493) and `c29ca0f` (38387645841). Earlier runs in this Cycle failed
on Windows and Linux for three test defects, each corrected in the tests: a
snapshot read of a lease file that another handle held, a two-process race
whose `busy` answers the test did not accept, and an assertion that two
commits made in the same second differ.

The final run, 38459717041 at `acb5f44`, passed on all five targets. The run
before it, 38432503577 at `7966744`, failed on both Windows targets and macOS
in one `mutation_remote` case, for a fourth test defect: a golden check added
by the contract-completeness work substituted the root placeholder with the
path the fixture was made at, which on those platforms is not the canonical
path an envelope reports. The test now uses the canonical path (`134017d`);
a run narrowed to the three failing targets (38444964170) passed before the
matrix was restored.

## Decisions For The Product Owner

All thirteen were decided by the product owner on 2026-10-10; decisions 12
and 13 were added that day. Each entry keeps the reasoning it was decided
on. One part of decision 2 remains: whether a pending merge conflict gets
an abandon path, which is ticket `01M4H33R34Z7C7EEKTY1ZCT950`.

1. **Size.** *Decided 2026-10-10: one Cycle in three parts.* F2 as the Wave scopes it is larger than F1: the boundary,
   twenty-eight bindings, three new domain operations and the relationship
   writers, in nineteen tasks. *Recommended:* keep one Cycle, built in three
   parts with a full gate and an independent review after each: (A)
   relationship writes, short codes and the comment author; (B) the boundary
   and the bindings for existing operations; (C) the bridges.
   *Alternative:* split at a part boundary into two Cycles, which needs a
   Wave amendment and lets C2 or C3 start on a partial foundation.
2. **The journal defect.** *Decided and done:* fixed on ticket
   `01M4EWN2DK3MY6H4GBYDYXF6QH` and merged to main on 2026-10-10. The
   ticket was closed with the rest of this decision still open.
   Three cases remain after the fix, all existing behavior that the
   boundary makes reachable by users; the design lists them. An operation
   killed after its journal row is written blocks the repository until the
   same request is repeated with the same input, and nothing clears it if
   the input is lost or someone else has since committed different content
   to the same file. A process killed between `enable` or
   `remote select` writing the configuration file and committing it leaves
   a dirty worktree that blocks authoring. And a synchronization that stops
   after reserving appears to hold its reservation, blocking local work
   until the request is retried or cancelled; that one is unconfirmed and
   was not re-examined after Cycle 06.
   *Recommended:* decide whether the library needs a way to abandon an
   operation, before C3 makes saves reachable by users. Ticket
   `01M4H33R34Z7C7EEKTY1ZCT950` asks the same for a pending merge
   conflict; the two belong together. The fix tracks what each call wrote
   but does not report it to the caller, so the boundary still infers it
   from the journal row. *Decided 2026-10-10:* an abandon path for local
   operations, as decision 13.
3. **Effects for changes that are not canonical content.** *Decided 2026-10-10: as recommended.* Identity, host
   trust, keys, remotes and registration removal fit none of the six
   effects, and the effect values are frozen. *Recommended:* report every
   effect as `not_requested`, put what changed in `data`, and let `outcome`
   say `success`, `noop` or `partial`. A half-finished key deletion is then
   visible only through `outcome` and `data`. *Alternative:* a new effect,
   which is a new contract version.
4. **Folder listing.** *Decided 2026-10-10: as recommended.* The Wave's F2 scope is amended to name
   the listing. F1 handed "folder creation and its listing" to F2; the
   Wave's F2 scope names only creation; and reads may not list a directory.
   A created empty folder is otherwise invisible. *Recommended:* the indexer
   records directories under `docs/` on primary, and F2 adds a read that
   lists them. This adds an index table and a refresh change, and the Wave
   text is amended. *Alternative:* create only; D1 raises listing as a
   shared-library change when its document tree needs it.
5. **Writes to a closed ticket.** *Decided 2026-10-10: as recommended.* The product owner expects to
   add short codes to this project's closed tickets by hand, as commits, at
   some later time; it is not urgent. Nothing stops a save to a lifecycle-closed
   ticket today, and F2 adds another way to write one. *Recommended:* the
   `ticket save` and `ticket slug-assign` bindings refuse a lifecycle-closed
   ticket. Repair stays allowed, as the CLI RFC expects. Loosening later is
   additive. This project's own closed tickets could then not be given a
   short code. *Alternatives:* allow slug assignment on closed tickets; or
   add no guard and leave it to Wave 02 Cycle 10.
6. **Host approval.** *Decided 2026-10-10: as recommended.* *Recommended:* the approvable host is one the selected
   publication remote names; the preview is local; the connection happens on
   confirmation, authenticates with the selected key, and the pin is written
   only then. This is stricter than the authentication RFC, which asks only
   that the host's key be observed again, and it means a non-interactive
   caller with a protected key cannot approve a host. It is what the
   existing verification does. *Alternative:* a host-key-only connection,
   which is a new transport path in Wave 02's code.
7. **Creating a repository when only a global identity exists.**
   *Decided 2026-10-10: as recommended.*
   `repo enable` uses the global identity and writes nothing. The existing
   create operation does not read global configuration. *Recommended:* for
   `repo create` the preview shows the global identity, and confirming it
   writes it to the new repository's local configuration. *Alternative:*
   change the create operation to use the global identity without writing
   it.
8. **Retention of request records.** *Decided 2026-10-10: as recommended.* No RFC says how long a retry may be
   answered. *Recommended:* no pruning in F2; rows are small, rejected
   requests leave none, and loss is safe. Add a rule when there is evidence
   it matters.
9. **Which commands F2 binds.** *Decided 2026-10-10: as recommended.* With `operation abandon` from
   decision 13. *Recommended:* the twenty-eight in the
   design: everything C2, C3 and D1 to D3 consume, plus the two
   synchronization commands the exit evidence needs. `comment add` and the
   polling commands are left to their first consumers, because their final
   shape depends on Wave 02 Cycles 07 and 08. *Alternative:* bind them now
   and accept a result shape that changes later.
10. **Registry names.** *Decided 2026-10-10: as recommended.* The result codes and recovery actions in the design
    are additions to a closed, published registry. Adding them is already
    non-breaking; the names are what is being approved. They can change
    until F2 merges and not after the CLI ships.
11. **Native evidence.** *Decided 2026-10-10: as recommended.* *Recommended:* as F1 decision 5: prove F2 on Linux,
    add its test targets to the native workflow's list, dispatch nothing,
    and carry Windows and macOS execution to G1.
12. **Synchronization after Cycle 06** (added 2026-10-10). *Decided
    2026-10-10: F2 stays clean-only.* The synchronization entry point F2
    adds takes a setting that refuses a divergent remote with
    `merge_required` before any merge is prepared, as the call behaved
    before Cycle 06, and the two bindings always set it. No conflict state
    is reachable through F2. C4 and D5 turn merging on when they bind
    `conflict resolve`, as a shared-library change. The existing
    `synchronize_remote` and its tests are unchanged. The three new error
    variants still get result codes, since the mapping is exhaustive. The
    product owner noted that nothing calls this code yet, so a state left
    behind during this work harms no one. The first choice, and why it was
    put again, are kept below for the record. The product owner first
    chose the recommendation below, before a review found its text
    incomplete. The drafts bind
    `item sync` and `repo sync` for clean synchronization and leave
    divergence to C4 and D5. Since Cycle 06, `synchronize_remote` merges a
    divergent remote itself: it makes merge commits, can ask for a
    confirmed identity, and can stop with a conflict that blocks every
    later synchronization of the repository until it is resolved. Three new
    error variants need result codes whatever is decided, because the
    mapping is exhaustive. *Recommended:* the two bindings pass divergence
    through as Cycle 06 built it and report it truthfully (the merge
    commit, `identity_required`, a conflict-pending code with the
    operation ID); F2 binds no `conflict resolve` and adds no conflict
    read, which stay with C4 and D5 as shared-library changes. Task 13
    grows by the divergent cases. *Alternatives:* bind
    `conflict resolve` and the conflict reads in F2, a larger Part B; or
    drop the two synchronization bindings from F2 and move their exit
    evidence to C4, which needs a Wave amendment.
    *What the review found, read from source:* a pending conflict also
    refuses every save of the conflicted item; other synchronizations get
    `busy` with nothing to act on; cancelling does not clear it; the only
    exits are `resolve_synchronization` and a merge commit made by hand.
    A stopped divergent synchronization can have made merge commits and
    written conflict markers into canonical files, which the effects as
    drafted cannot report, and the merge commit of a stopped
    synchronization has no read. So the recommendation as written ships a
    command that can enter a conflict and none that leaves it, and touches
    Tasks 5, 6, 9, 13 and 14, not two tasks. The fourth option, chosen,
    is the setting described at the head of this decision.
13. **Abandoning an operation** (added 2026-10-10). *Decided 2026-10-10:*
    F2 adds a confirmed `operation abandon`: it closes a pending local
    journal row, touches no file, and reports what the operation left
    behind. It is a task in Part B after confirmed administration, and the
    recovery action offered with `recovery_required`. It does not cover a
    pending merge conflict, which stays with ticket
    `01M4H33R34Z7C7EEKTY1ZCT950`. This is added scope; the Wave text does
    not name it.

Smaller rulings the design makes. None was overruled at approval:

- `prepare` takes no request ID. A confirmation is single-use and bound to
  the first request that accepts it. A clock set backwards expires it.
- An observation token is part of a request's input. After
  `external_change` the caller reads again and uses a new request ID.
- A request that ends with nothing in flight leaves no record, and releases
  the confirmation it had accepted.
- Losing a request record is safe: a request for the state that already
  exists is answered as a no-op.
- The replay rules are built test-first against the design's
  required-outcomes table, with a design checkpoint after Task 7.
- A retry examines only the request's own paths; unrelated commits on the
  same branch do not stop it.
- The adoption ID is supplied by the caller in the corrected source; the
  library never generates one. Ticket repair applies only when that ID
  equals the directory name. Repair needs the file to be committed.
- Until a repaired document's context merges, lists show both the
  nonconforming file on primary and the repaired item.
- Local operations have one safe point, before acceptance. Synchronization
  keeps the seven Wave 02 gave it.
- `resume_operation` and `show_request` are included. Resume is limited to
  the index hand-off and clean synchronization; everything else is resumed
  by repeating the request.
- The cycle check uses the index as it stands and does not scan. Its
  members are reported sorted by ID.
- Repeated `deps` entries are removed silently. A relationship to a known
  document or comment is rejected; to an unknown ID, accepted.
- Key order is not part of the canonical form of a ticket's front matter.
- Initials given on `ticket create` or `ticket slug-assign` apply to that
  call and are not stored. Names are not transliterated. The prefix and
  code length are read from primary's committed configuration.
- An invalid hand-written prefix or code length blocks short-code generation
  and nothing else.
- `repo identity-set` works on a repository that is not enabled.
- Folders are created on primary only, and never past discovery's limits.
- After the database is lost, an earlier generated key file is left for the
  user to import; nothing adopts it.
- No new dependency is added.

## Implementation Outcome (2026-10-13)

Status: **review-ready, awaiting the product owner.** The Cycle is built on
the ticket branch at code head `765e43a`. This section records what was
built, what is still owed and what was found. The execution ledger holds
the rulings behind it and the
[design](../plans/2026-10-08-wave-03-foundation-02-replay-confirmation-and-bridges-design.md)
carries the detail; the design describes the code as built.

All thirty commands are bound: the twenty-eight baseline commands,
`operation abandon` and `operation resume`. `resume_operation`, which the
scope names as a function, is the bound command `operation resume`, run
through `execute` with a request ID and a replay of its own. The three
parts were built in order, each with a full gate and an independent review:
relationship writes, short codes and the comment author; the boundary and
the bindings of existing operations; the bridges.

### Built beyond the approved scope, by product-owner decision

Each was decided during implementation, on the date given.

- **`repository_not_enabled`** (2026-10-12). A repository that was never
  enabled is refused with its own code and no action, and `index rebuild`
  does not register it. A repository whose registration was lost keeps
  `repository_not_registered` with `index.rebuild`.
- **`request.cancel` on a blocked synchronization** (2026-10-12).
  `merge_required` and `remote_branch_deleted` offer it while the request
  holds the reservation, since the same request cannot get past either.
- **Settings commands refuse a late retry** (2026-10-12). `remote select`,
  `key select`, `key clear` and `key import` answer `external_change`,
  final, when the setting is neither as the first call observed it nor as
  the request asks, so an older choice is not put over a later one.
- **`key_registry_changes`** (2026-10-12). A new index table records each
  key import, selection and clearing against its operation ID in the
  transaction of the change. A retry answers the first result from it.
- **The message of `repository_not_registered`** (2026-10-12) is "The
  repository is not registered with Manyhands."
- **`folder create` on a folder that is there still refreshes**
  (2026-10-13): a no-op with `discovery: current`, so the folder is listed
  after any success.
- **A second repair of a path under another ID is `not_repairable`**
  (2026-10-13).
- **The live-indexing guard** (2026-10-13). A second live call of a request
  leaves a journal row that another call is indexing under untouched and
  answers index-pending, in every operation that hands off to indexing:
  the saves, repair, remote add, remove and select, enablement and comment
  submission. Its consequence was accepted with it: a retry within about
  three seconds of a process that died while indexing is answered
  `discovery_pending`, and completes on a later retry.
- **Callers retry `busy` and `partial` automatically** (2026-10-12). Two
  callers that race can each be answered `busy` part-way. No product
  change; a front end takes `request.retry` without asking.
- **`host observe`** (2026-10-13). The read of what a host presents is
  published under its own command name; F1's `host inspect` keeps its
  name.
- **One vocabulary for a host's keys** (2026-10-13). `presented_*` is the
  key a host presented, `pinned_*` the stored pin, and `algorithm` with
  `fingerprint` the key a request names. F1's pin read was renamed to
  match: `host_pin` now gives `pinned_algorithm` and `pinned_fingerprint`.
  This and the reworded message of `not_repository_root` are the two
  changes this Cycle makes to what F1 published.
- **Native CI was run** (authorized 2026-10-12), where decision 11 had
  carried Windows and macOS execution to G1.

Also changed from the approved text without widening scope: the identity
write is one atomic write under Git's own configuration lock, and nothing
is restored; `host approve` and `host replace` take the key's algorithm
as well as its fingerprint; `operation show` does not find the operation
ID of a recorded key registry change (confirmed as a known limit,
2026-10-12). Where the built contract departs from an RFC sentence, the
design lists it once, under "Departures from the RFCs". The RFC texts are
not amended on this branch.

### Open obligations

- Product-owner acceptance of the Cycle, and each of publishing, merging
  and closing, which need their own authorization.
- The final gate and the native run on the final head; their results
  are recorded under the acceptance table.
- The RFC departures the design lists have no amendment to the RFCs. C-track
  planning that reads an RFC alone would expect the other answer.
- Two questions in the design are built one way and not confirmed: whether
  `merge_required` is the right code when the remote has only moved ahead
  after a lost push, and whether `operation resume` and
  `operation abandon` should require a root for a key operation.
- Two fixed messages are proposed for rewording and not changed:
  `host_replacement_required` and `host_mismatch`.
- The acceptance rows marked partly above: the synchronization half of
  lost output with a save in between has no test, and interruption is
  proven at four fault points by commit count.
- `remote_unavailable` is mapped and published and no test produces it end
  to end.

### Gaps for later Cycles

The three the document already lists stand:

- **Republishing a remotely deleted item branch.** Unchanged. A
  synchronization answers `remote_branch_deleted`, holds its reservation
  and offers `request.cancel`; nothing republishes.
- **A CLI verb that returns an observation for an absent path.** F2 built
  the read, `observe_path`, with the command name `path observe`. The CLI
  RFC's taxonomy still has no verb for it; C3's planning decides.
- **A nonconforming file on primary blocks comments.** Unchanged. A repair
  does not lift it until the repair's context merges, which needs promotion
  or closure from later Cycles; until then a repaired document is also
  listed twice.

Found in this Cycle and owned by a later one:

- `publication_pending`, which the CLI RFC's example for `comment add`
  uses, is not registered. The Cycle that binds `comment add` adds it.
- The evidence walk of a retry is not first-parent. C4 and D5 revisit it
  when they turn merging on.
- The ten-second "still stopping" display of a cancellation belongs to D6.
- The preview's field for the short-code prefix is `slug_prefix`, where
  the relationships RFC names the input `ticket_slug_prefix`.

### Defects found and not fixed

Each is in code that predates this Cycle, or outside its scope, and has a
ticket.

| Ticket | Title |
| --- | --- |
| `01M4PB7WZN9WNAQ3KXZ19J28KM` | Removing a remote can delete local branches, tags and another remote's settings |
| `01M4PB7YZMBFAFFM94AZY08HWS` | An interrupted save is closed as rejected although it wrote its file |
| `01M4PB80T7SVB23TQ6466RDHCB` | Repeating a document move that already committed is refused |
| `01M4PB82G07RZSQMNJZN672S3W` | A partly damaged application database is not recognized as corrupt and cannot be rebuilt |
| `01M4R19P943QZKHHKRXV2D2TP2` | A registration whose directory is gone cannot be removed |
| `01M4R19PC0T6ZPKM39X9DF26HX` | A blocked synchronization leaves its reservation held until its request is cancelled |
| `01M4R1K5VRSRXFM58VQCTQ9F3W` | Shorten the native CI feedback loop |
| `01M4SDB38H9JTG77VMC9102W2F` | Enabling a repository can erase another process's Git configuration change |
| `01M4TFGHPG98GCM6TX3B56HJAB` | A backslash in a file name under docs makes every refresh fail on Linux and macOS (low priority) |

The `remote remove` binding refuses the removals the first ticket
describes; the domain function is unchanged. Ticket
`01M4H33R34Z7C7EEKTY1ZCT950`, an abandon path for a pending merge
conflict, stays open from decision 2.

### Known limits accepted

They are listed once, in the design, under "What changed from the
proposal, and known limits", grouped by area, each with why it is
accepted. They are not repeated here.

### Deferred minors with no ticket

Recorded in the ledger as deferred or as recorded and not fixed. None
gives a wrong result that was found.

- **Records.** `cancel_requested` stays set on a finished record. A
  refusal on a retry that will never change holds its record, and the way
  out is a new request ID; `key remove` still deletes its record on a
  failed retry. An accepted record that cannot be read repeats
  `internal_error` with `request.retry`. A record of a key registry
  command from an earlier build has no row and no downgrade guard.
- **Synchronization and resume.** A `published` read from a row that only
  verified a push can name a commit this repository never pushed.
  `merge_required` when only the local branch moved after a lost push. A
  fenced `recovery_required` offers `repo.inspect` on its first answer and
  none on a replay. The owner's cancel flag is read at the first safe
  point and not before the call. A local hand-off raises the owner's
  attempt even when the refresh answers `busy`. `repo.inspect` on a key
  operation names the caller's root. `SafePointRequest::Stop` was added to
  a public enum that is not non-exhaustive, and is tested at one safe
  point.
- **Effects.** Two `not_requested` understatements of a remote command's
  hand-off, and a kept answer that says `discovery: pending` and is not
  stored.
- **Never enabled.** An unreadable branch tip counts as enabled; any entry
  at the configuration path counts, valid or not; remote-tracking
  branches are not read.
- **Identity.** The stale `config.lock` is not named in the `busy`
  message; a killed write can leave a private scratch directory;
  validation's private directory and the under-lock one-value check have
  no test of their own.
- **Host.** Four older SSH assertions count authenticated clients and not
  sockets. `presented_*` is null on a successful approval. An `execute`
  for a host the remote no longer names answers `already_applied` where
  `prepare` answers `invalid_input`. A `partial` may be attributed to a
  pin another actor stored.
- **Folders and repair.** The limits ruled as known and not fixed (short
  names, Unicode normalization, an over-long path, the non-Unix
  check-then-create window, the liveness check and its steps not being one
  transaction, a worktree removed by hand) are in the design's list.
- **Structure.** The authoring-only re-entry types are still in the shared
  binding trait. Two bindings choose `request.retry` themselves through
  `outcome::same_request`. The transport's own approval type keeps the
  names `expected` and `presented`.
- **Schemas.** The preview wrapper is restated in eight schema files.
  `command`, `state`, `step` and `algorithm` are closed in Rust and free
  strings in the schemas. `request.schema.json` repeats the outcome and
  code lists. `recovery_action.schema.json` does not say where
  `operation.resume` takes its root.
- **Tests.** Several pins were written after their fix and never seen
  failing; the scans of `key clear` and `key remove` cannot fail today;
  the conformance row of `key clear` runs over a no-op; two commits of
  the fix pass and two of Task 16 are not clean under clippy on their own.
- The remaining minors of each task's review are in the review files
  beside the ledger, and those of the whole-Cycle review of existing code
  (m2 to m5) in its report.

## Review Record

The three documents were reviewed on 2026-10-08 in three rounds. In the first,
one independent reviewer read them against the source and one against the
Wave, the RFCs and each other; their blocking findings are the first seven
rows. The documents were rewritten, and a third reviewer attacked the
mechanisms the rewrite introduced; its blocking findings are the last four
rows but four. A fourth reviewer then walked fifteen timelines through the
corrected rules and found four more blocking errors; those are the last
four rows. **Every round found blocking errors in how a retried request is
settled, including in the previous round's corrections, and the corrections
for the third round have not been reviewed.** The design therefore states
the required outcomes as the contract and treats its mechanism as
provisional; the plan builds it test-first and stops for a checkpoint.
Nothing here has been run.

| Source | Concern | Classification | Resolution |
| --- | --- | --- | --- |
| Source review; `recovery.rs:342-379`, `repository.rs:3727-3791`, `2583-2618` | Ordinary rejections leave a pending journal row that blocks the repository; the first draft called it possible and named three operations. | Decision | Stated as certain from source, with every path; decision 2. |
| Source review; `repository.rs:3507-3596`, `3286-3300` | Re-running `create_and_enable` after it completed fails, and `set_publication_remote` reports a change and a commit that are not the request's. | Settled | Per-operation replay table; `repo create` retries through `enable`; `remote select` compares before calling. |
| Source review; `read/items.rs:485-503`, `sync.rs:359` | The first draft's cache-loss table assumed a registered repository with keys and pins. Losing the database loses all three. | Settled | Table rewritten from an unregistered repository; "equal and committed" required for a no-op. |
| RFC review; CLI RFC 183 | A retry of an interrupted save would have failed its own observation check. | Settled | A retry skips the token check and re-enters the domain operation with the expectation recorded at acceptance. |
| RFC review; CLI RFC 325, 333-334 | A failure class fixed per code gives the wrong exit status when one code stops a request before and after an effect. | Settled | One rule for outcome and class from cancellation, effects and code. |
| RFC review; CLI RFC 118-119 | Refusing repair of a closed ticket contradicted the repair rules. | Settled | The guard covers save and slug assignment only. |
| Both reviews; CLI RFC 177 | A retry after acceptance never observed again, even before any effect. | Settled | The observation is compared again until the request has made an effect. |
| Source review; `repository.rs:2090-2095`, `recovery.rs:444-454` | The journal step does not show whether a commit was made, so an earlier attempt's commit could be misattributed or the file rewritten. | Settled | The position is recorded at acceptance; the commit is taken from commits after it that left the request's paths as intended. |
| Source review; `repository.rs:1440-1473`, `items.rs:1585-1587` | A token over primary's on-disk bytes does not match a context checked out from primary's head when primary has an uncommitted edit. | Settled | A first save requires primary's file to equal its head blob. |
| Source review; `canonical.rs:805-837` | The serializer emits unknown keys first and cannot quote selectively. | Settled | Key order is not canonical; quoting is left to the serializer and proven by round trip. |
| Source review; `repository.rs:7178-7186` | A folder past discovery's limits would stop all authoring. | Settled | `create_folder` refuses to cross them. |
| Source review; `discovery.rs:724-754`, `repository.rs:2315-2323` | A repaired ticket is not listed twice; a nonconforming primary file blocks comments. | Settled | Stated for documents only; the comment block recorded as a finding. |
| RFC review; Wave C2, D1 | Nothing let C2 or D1 learn the fingerprint a host presents. | Settled | `observe_host` added. |
| RFC review; Wave D3 | D3 must explain a cycle before a save and may not change the library. | Settled | The relationship check is a public read. |
| RFC review; CLI RFC 100, 264-297 | A `comment add` result published before its publication half exists would change later. | Decision | Decision 9. |
| RFC review | Tasks 6, 8, 9 and 10 of the first plan were too large to review, and Task 6 depended on Task 7. | Settled | Nineteen tasks; ticket bindings and token checking arrive together. |
| Both reviews | Several tests named fixtures or hooks that do not exist, or had no stated expectation. | Settled | Corrected in the plan; each rejection test names its code. |
| Second review; `coordination.rs:121-140`, `repository.rs:2648`, `2669-2726` | The rewrite deleted a request's record when a call "made no effect", but the domain returns the same error kinds before and after a write. | Settled | Settlement reads the operation's journal row; a record is deleted only when nothing is in flight. |
| Second review | A concurrent duplicate of a request could delete or finish the running one's record. | Settled | An attempt number on the record; every change is conditional on it; a retry never deletes. |
| Second review; `repository.rs:2368-2378` | Refusing a retry whenever the branch tip moved would strand requests after any unrelated commit, and the tip is undefined for a first save. | Settled | The check examines only the request's paths; an absent branch is recorded as primary's head. |
| Second review; `keys/deletion.rs:86-133`, `repository.rs:3192-3200` | "Has made an effect" had no source for confirmation retries, and the rule for commands without a journal was false. | Settled | A journal row for the operation ID is the test. Commands without one compare again, and a new preview completes a partial change. Whether an interrupted key deletion can be finished is established in Task 12. |
| Second review; `local_authoring.rs:5689-5712`, `6417-6495` | The journal fix cannot be decided from the error kind, and two existing tests require a pending row after a standalone context failure. | Decision | Stated in the design; settled on the defect ticket under decision 2. |
| Third review; `recovery.rs:474-479`, `repository.rs:2090-2095` | Skipping the save when its commit was found left a journal row that only the save can complete. | Settled | A retry always calls the domain operation; the evidence check supplies only the commit. |
| Third review; `keys/generation.rs:154-191` | An interrupted key generation is completed by its next call; treating its first result as final would lose the key. | Settled | Finished only on success or "retained for inspection". |
| Third review; `repository.rs:4295-4299` | A completed `repo remove`, identity change or host approval whose result was lost would be answered with `external_change`. | Settled | A retry first asks whether the end state already holds. |
| Third review; `repository.rs:1941-1949` | Content already in place from another request makes the domain report an external change, not a no-op. | Settled | The already-applied rule, stated once and used for late callers, lost records and cache loss. |
| Third review; `sync.rs:520`, `recovery.rs:280-285` | A synchronization that stops after reserving appears to hold its reservation and block local work. | Open | Stated as open case 3; reproduced in Task 13; raised as a ticket if it holds. |
| Refresh, 2026-10-10; main `f87ce81` | The journal fix keeps a closed row marked `rejected`, so "a journal row exists" no longer means the operation started. | Settled, unreviewed | A `rejected` row counts as no row in settlement, re-entry and the confirmation recheck. |
| Refresh, 2026-10-10; `sync.rs` at `f87ce81` | `synchronize_remote` now merges a divergent remote and has three new error variants; the drafts assume it refuses. | Decision | Decision 12. |

| Fourth review, of the refresh, 2026-10-10; `recovery.rs:386-401`, `repository.rs:3546-3559` | "Not rejected when the call returned" let `remote select` report another request's commit: a repeat resets the rejected row before the domain runs. | Settled, unreviewed | Rule 4 asks what the row was before the call, or whether the commit appeared during it. |
| Fourth review; `reservation.rs:582-584`, `repository.rs:2895-2909`, `sync.rs:3653-3916` | Decision 12 understated a pending conflict and its recommendation was not achievable as written. | Settled | Decision 12 reopened with the facts and decided: F2's bindings refuse divergence. |
| Fourth review | A record stranded when rule 1 met a rejected row; four wrong statements and two wrong test expectations about the journal fix. | Settled, unreviewed | Corrected in the design and plan. |

The fourth review read the refresh against the code and the replay rules
against the journal fix. It did not read the RFCs, the Wave, most of the
Cycle 06 tests, or the audit rows outside the journal, the lookup and
synchronization. Source line anchors in the rows above the refresh are at
`6cf5d7f` and no longer hold.

### Implementation reviews (2026-10-11 to 2026-10-13)

Every task was reviewed independently after it was built, and its findings
fixed or ruled before the next began; the ledger records each. The design
was reviewed against the code at the checkpoint after Task 7, and the
whole of Part A and of Part B at their checkpoints.

The whole Cycle was reviewed on 2026-10-13 through four lenses, each by an
independent reviewer reading the source. Counts are critical, important,
minor.

| Lens | Verdict | Findings |
| --- | --- | --- |
| Replay and confirmation safety | Ready with fixes | 0, 2, 5 |
| Privacy and redaction | Ready | 0, 0, 5 |
| The published contract against the RFCs and the design | Ready with fixes | 0, 3, 11 |
| Changes to existing code | Ready | 0, 0, 6 |

No lens found a request that applies twice, applies something other than
what was sent or consented to, or reports a false effect. The five
important findings were places where sibling commands answered one
situation differently, a recovery action that could not succeed, and an
exit-evidence phrase not yet held.

One fix pass followed, `e8c9592..1815366`, twenty commits for seventeen
items. Its review was **approved**, with no critical finding, one
important finding that was a missing test, and six minors. A touch-up,
`18a63d3..765e43a`, three commits, added that test's golden, gave
`repo identity-set` its siblings' recovery action, and renamed F1's pin
fields. Of the review's six minors, two were fixed there, one was a
correction to the fix report, and three stand as known limits in the
design: an accepted record that cannot be read repeats its answer, two
fixed messages do not fit, and `presented_*` is null on a successful
host approval.

The gate on `1815366`, before the touch-up, passed: the checks, formatting
and lints clean, 2,111 tests passed and none failed across 30 targets. The results of the final gate and of
the native run are recorded under the acceptance table.

Record planning, per-task progress, decisions, baseline and final
verification, review and review-ready status as ticket comments. Publishing,
merging, closing and worktree cleanup each need their own authorization.
