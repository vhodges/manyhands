---
title: "Wave 02 Cycle 08 One-Shot Polling And Materialization Execution Ledger"
date: 2026-10-11
status: approved
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M4NM26RWRM6ATDHN16HV1M9S"
---

# One-Shot Polling And Materialization Execution Ledger

## Task 8 Permission-Denied Evidence Follow-Up — 2026-10-14T13:11:49Z

**IN PROGRESS / CONTROLLER REVIEWS PENDING.** Owner explicitly required closing
row 6 before final reviews. Clean increment base
`1bbc9e250a4095e4b5a43721126297e27cf2b4ee`; complete Task 8 base remains
`b0f005531879c5ee2806ed13052bdd157df627d7`; whole-branch base remains
`d1ca88772c5768d55aadb2ca4dcac1c69dab0d17`. Same assigned worktree/branch,
inline, no subagents or repeated fetch/rebase. Prior approvals/minor resolutions
remain trusted. **Row 6 now fully LOCAL-proved**, with exact new passing case:
`poll_unix_permission_denied_worktree_preserves_local_state`.

### Real Permission Denial And Preservation

New cfg(unix) case in tests/remote_polling.rs uses the public poll API and the
existing real SSH two-clone fixture. A clean checked-out ticket context is
strictly behind a collaborator's new commit. A fixture-local restoration guard
is installed before changing the worktree directory to **000**. Both actual
directory enumeration and reading its `.git` file must return
`std::io::ErrorKind::PermissionDenied` before polling. **No root/ineffective-mode
skip or success branch**: ineffective permissions fail the assertions, with the
guard restoring permissions on unwind/early error. Local UID was **1000**.

Poll returns completed report with **Inaccessible and exact fixed guidance**.
After it, `.git` remains genuinely PermissionDenied and directory mode remains
000. Explicitly checked restoration precedes all poll-result and preservation
assertions; guard also restores on failure. Full before/after existing structural,
permission-aware snapshot matches: local refs, status, worktree content/attributes,
HEAD/index/merge metadata, gitfiles, registration/admin artifacts and shared config.
Complete direct/symbolic ref inventory allows **only the fetched context tracking
ref** to be created/advanced to the advertised incoming commit. Local context
branch stays at its original commit and incoming file is absent. Existing
no-receive-pack counter baselines and recovery/raw-output privacy scans apply.

This is **Unix permission semantics proved on local Linux only**. Non-Unix builds
do not register this case: no ignored/fake passing fallback. Windows ACL
permission-denied preservation coverage is **PENDING native evidence** and this
case does not prove it. macOS/Linux aarch64 execution likewise pending. No new
helper holds, fake transport, production hook, credential/config/session logging,
schema, dependency or production behavior change.

### Trials And Final Gate

- First exact focused command
  `devenv shell -- cargo test --locked --test remote_polling poll_unix_permission_denied_worktree_preserves_local_state`
  compiled and failed before denial: the existing clean_checkout fixture helper
  requires context tracking at the local OID, but a new unpublished local context
  has no tracking ref before the first poll. Its `refname_to_id` returns an error.
  Replace that inappropriate helper with actual baseline HEAD/index/clean-status
  checks; after polling, check local branch and absence of incoming content while
  independently expecting the tracking ref at incoming. Next focused run **1
  SSH case passed** against unchanged production. **No production RED claimed.**
- First sequential full-gate attempt: check/fmt PASS, Clippy rejected decimal
  `Permissions::from_mode(0)` under non_octal_unix_permissions. Test-only literal
  corrected to `0o000`; tests and CLI were **not reached** in that aborted chain.
- Final changed Rust tree then passed the complete sequential chain below within
  its 3,600,000 ms bound. **No Rust edits after that gate.** Its new code justified
  repeating the previous checkpoint's gate. No other failures or timeout here.

Full raw output log:
`/home/vhodges/.local/share/opencode/tool-output/tool_13a6c7eeb001Z64MpsevEGUX77`.

| Exact command | Final result |
| --- | --- |
| `devenv shell -- cargo check --all-features --locked` | PASS |
| `devenv shell -- cargo fmt --check` | PASS |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | PASS |
| `devenv shell -- cargo test --all-features --locked` | **1,766 passed, 0 failed, 1 existing ignored** |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | PASS, exit 0, no window |
| `git diff --check` | PASS, repeated after bookkeeping |
| `sha256sum Cargo.lock devenv.lock` | Entry/final hashes identical |

Runtime totals: **615 library + 779 ordinary integration + 363 harness=false SSH
cases + 9 doctests**. SSH targets actually report **41 comment publication,
25 merge recovery, 15 observation, 74 polling, 63 synchronization, 31 fixture,
114 transport**. Private negative-control child cases and the existing ignored
thousand-item characterization are not counted. New case is explicitly `... ok`
in the full log at line 1514. All 12 rows in the living acceptance map below are
rechecked against this output: **122 distinct named tests, 0 missing PASS names**.
Row 6's prior literal gap is closed locally, not relabeled as missing/redirected.
Row 12's native dimension remains unproved. Accepted-title raw canonical-index
exception and every metadata/output privacy assertion are unchanged and passing.

```text
8a20e8ba089f63bddfa9b87a08d5d4308404b4c2440c050453654e422a9549f8  Cargo.lock
d4a78b8259525e7dcf66814477b0f96282e6570c9a21ebc84eda7d7fb8950d6e  devenv.lock
```

GUI startup not redundantly rerun for this test-only increment; prior bounded
actual display/startup evidence below remains historical process-only evidence.
Canonical checkpoint comment `01M4X8MRBTPKGT67BFP85CZBXN`; scripts/show-ticket
validation **PASS**, rendering **43 comments**, new comment, open ticket and no
malformed diagnostics. Raw-output map/count verification **PASS**, with 12 rows,
122 exact names, zero missing PASS lines and 1,766 total passing tests.
Increment changes only polling test, living map/ledger, Cycle evidence,
ticket and that comment: **5 files**. Complete Task 8 range through returned HEAD
contains **9 files** (prior seven plus polling test and third comment); complete whole-branch range
contains **81 files**, including 43 canonical comments. Exact return SHA follows
the authorized new local commit; no amend.

Permission-test checkpoint: `48c612149a21037046559a1efb52c297a51b2341`.
Post-commit name inventory corrected the cumulative Task 8 file count from 8 to 9;
this ledger-only correction is a separate local commit, with no Rust change or
gate rerun. Increment and whole-branch counts remain 5 and 81 respectively.

Controller SPEC, whole-branch quality and independent whole-branch reviews remain
**PENDING / unrun by implementer**. Task 8 unchecked, ticket open. All five native
targets and Windows ACL evidence **PENDING AUTHORIZATION / native verification**;
workflow stays manual and undispatched. No push/PR/merge/closure/worktree removal.
D1–D4, accepted⇒active-only amendment and sole Task 5 final-validator canonical
collection-under-single-lease exception unchanged; network, prompts and discovery/
index refresh remain outside. New local proof closes the explicit acceptance gap
without changing the approved contract or claiming native/final review completion.

## Task 8 Local Verification Checkpoint — 2026-10-14

Historical first checkpoint: `1bbc9e250a4095e4b5a43721126297e27cf2b4ee`.
Commands/counts/trials below retain that checkpoint's evidence; its living
acceptance map is refreshed by the permission-denied follow-up above.

**Status: IN PROGRESS / CONTROLLER REVIEW PENDING.** Fresh inline implementer
at clean exact base `b0f005531879c5ee2806ed13052bdd157df627d7`, assigned ticket
worktree/branch, no subagents. Controller reports Task 7 SPEC
`ses_ec64fd841ffe4JHjZ3XJ7I1MOp` and QUALITY
`ses_ec608e461ffeB8pQQSieLJnzhE` **APPROVE** through this base after all canonical
path, first-poll sentinel and snapshot fixes. Prior Tasks 0–7 approvals and
current-main no-op resume preflight trusted; no fetch/rebase or review replay.
Historical pending Task 7 statements below are superseded by these approvals.
Entry checkpoint `01M4X3ER7B21PNX7DZNQ0RMZ55`.

### Deferred Minors Resolved Locally

1. Task 2 QUALITY `ses_ec9e9909cffeS4kt0Z0ENmyjss`: added two direct-SQL durable
   invariant tests in state_tests.rs. Explicit observe↔update, update→NULL, and
   legacy NULL→either explicit kind are denied, including same-kind SQL updates
   under the approved strict immutability semantic. Compare **every stored row
   value**, not merely poll_kind, after each denial and two reopens; decode NULL
   as Observe. A genuine pre-kind/pre-intent/pre-disposition registry migrates to
   explicit Observe once, retaining all original row values, then denies kind
   changes on both reopens. Production trigger unchanged; already-correct
   hardening, **no behavioral RED claimed**.
2. Task 6 QUALITY `ses_ec6bed51effeNWgnoaPfQ9pdlV`: extracted
   `attach_context_dispositions` in state.rs. Audit calls it before its early
   return; normal reads call it only after local-only contexts are projected.
   Exactly one pass on either path, identical target/malformed predicates. No
   audit/envelope/history/local-cache validation removed or relaxed. Existing
   meaningful state and ordered snapshot tests cover local-only HistoryUnknown,
   persisted and malformed contexts, reopen, historical validation, partial
   envelopes and privacy; no helper-call-count mirror test added.

### Actual Trial And Failure History

- First `devenv shell -- cargo test --locked --lib remote::state::persistence_tests::poll_kind_`
  compiled but **0 passed/2 failed/613 filtered**: new fixture attempted invalid
  outcome `interrupted`, which is a phase only. Existing SQL CHECK correctly
  refused fixture setup before any tested kind mutation. Remove the invented
  outcome, retain interrupted phase and NULL outcome. Next run **2 passed**;
  no production bug or RED-to-fix claim. One patch attempt after formatting did
  not match its pre-format context; reread and applied correct test-only edit.
- State suite **61 passed/554 filtered** before cleanup and again after cleanup.
  Ordered persisted/new/malformed-context snapshot test **1 passed/614 filtered**
  after cleanup. Full gate below passed first time; no compile/lint/test timeout
  or subsequent Rust edit. Two SQL cases increase library count 613→615.
- Post-gate count aggregation first used unavailable `python3` (shell exit 127,
  not a Rust/test failure); Node aggregation of actual summary lines confirms
  **1,403 ordinary (library + integration + doctests) + 362 SSH = 1,765**.
- Initial wrapper around successful `scripts/show-ticket` output falsely matched
  ordinary body text containing "malformed"; restricting the diagnostic check to
  line-anchored `show-ticket:` confirms **42 comments, both new comments rendered,
  open ticket, no malformed diagnostics**. This was a wrapper false positive,
  not schema corruption or a script failure.
- Historical Tasks 0–7 meaningful REDs, fixture invalidity, compilation trials
  and outer-chain timeout remain in their dated entries below; not erased or
  rerun. No new transport, config, secret logging or dependency.

### Sequential Final-Tree Verification

Final Rust edits preceded this single required gate. Commands joined with `&&`,
3,600,000 ms outer bound; entire chain completed successfully. Actual full log:
`/home/vhodges/.local/share/opencode/tool-output/tool_13a3db7a5001ZVwqt6D5YeQIkQ`.
All Rust commands, including focused tests and optional startup, used Devenv.

| Exact command | Observed result |
| --- | --- |
| `devenv shell -- cargo check --all-features --locked` | PASS |
| `devenv shell -- cargo fmt --check` | PASS |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | PASS |
| `devenv shell -- cargo test --all-features --locked` | **1,765 passed, 0 failed, 1 existing ignored** |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | PASS, exit 0, no window |
| `git diff --check` | PASS; repeat after bookkeeping before commit |
| `sha256sum Cargo.lock devenv.lock` | Entry/final hashes identical (below) |
| `scripts/show-ticket 01K7F6H9J2N4Q6S8V0X2Z4B6DG` | PASS, open ticket, 42 comments, both new Node-ULID comments rendered, no malformed diagnostics |

Post-bookkeeping evidence verifier extracted the 12 acceptance-table rows and
matched **121 distinct exact test names** against actual passing full-gate lines:
**0 missing passing names**. It checks runtime output, not declarations or globs.
Preservation diff against Task 8 base for dependencies/lockfiles/toolchain,
workflow, CLI/desktop, schemas, tests and approved design/plan is empty.

Runtime summary breakdown: **615 library + 779 ordinary integration + 362 SSH
target cases + 9 doctests**. Binary test targets each 0. Existing ignored case:
`reads_of_a_thousand_item_repository_are_characterized`; not run or counted.
Harness=false runtime totals, not globs or private negative-control child cases:

| SSH target | Actual passing cases |
| --- | ---: |
| comment_publication | 41 |
| remote_merge_recovery | 25 |
| remote_observation | 15 |
| remote_polling | 73 |
| remote_synchronization | 63 |
| ssh_fixture | 31 |
| ssh_transport | 114 |

Both disconnect cases report **131,029 PACK payload bytes, 0 new indexed objects,
incoming commit not indexed** in this final run. The byte-qualified real fault
and object inventory checks pass; zero additional indexed objects is an actual
observation, not a portable assumption. Task 7 normal/alias **73+73** evidence
remains trusted; alias target was not redundantly rerun for this state-only cleanup.

Lockfiles (SHA-256), unchanged at entry and after full gate:

```text
8a20e8ba089f63bddfa9b87a08d5d4308404b4c2440c050453654e422a9549f8  Cargo.lock
d4a78b8259525e7dcf66814477b0f96282e6570c9a21ebc84eda7d7fb8950d6e  devenv.lock
```

Display inspection found `DISPLAY=:1`, `WAYLAND_DISPLAY=wayland-1`,
`XDG_RUNTIME_DIR=/run/user/1000` and the Wayland socket. Optional bounded command:
`timeout --signal=TERM --kill-after=10s 90s devenv shell -- cargo run --locked --features desktop --bin manyhands`.
Build finished in 16.05 s, then the actual `target/debug/manyhands` ran and emitted
`Hello from Manyhands!` twice, without startup error, until the intentional bound.
This is **process startup evidence only**; no visual inspection/interaction claimed.
No Rust edits followed the full gate or the startup check.

### All 12 Acceptance Rows: Exact Named Passing Tests And Dimensions

Names below are exact test identifiers observed as `... ok` in the latest full
gate (permission follow-up log above), not inferred from source declarations. Namespace key: **P** =
`repository::remote::poll::tests` (poll_tests.rs), **S** =
`repository::remote::state::persistence_tests` (state_tests.rs), **R** =
`repository::remote::reservation::tests` (reservation_tests.rs), **T** =
`tests/ssh_transport.rs` harness case, **SSH** = `tests/remote_polling.rs` harness
case. Prefixes identify the source/target; backticked suffixes are exact names.
Rows 1–11 now have local proving tests for every listed property, including
literal Unix permission-denied unreadability in row 6. Row 12's local
gate passes but its native runtime dimension is pending, **not native-proved**.

| Cycle row | Property dimensions → exact passing tests | Local assessment |
| --- | --- | --- |
| 1. Eligibility / explicit bypass | Disabled, paused, recovery-suspended, not-due, zero connections/prompts/new reservation and unchanged outcome/policy → SSH `poll_policy_disabled`, `poll_policy_paused`, `poll_policy_recovery_suspended`, `poll_policy_not_due`; blocked-session no automatic prompt → SSH `poll_locked_blocked_session_no_automatic_prompt`; precedence and no writes → P `eligibility_policy_precedence_wins_over_due_and_future_deadlines`, `ineligible_policy_writes_nothing_even_when_deadline_needs_clamping`, `task6_automatic_gates_do_not_reserve_or_change_outcome`; active/interrupted automatic restart gates → P `review_restart_auto_gate_disabled`, `review_restart_auto_gate_paused`, `review_restart_auto_gate_recovery_suspended`, `review_restart_auto_gate_not_due`; explicit restart bypass/manual identity → P `review_explicit_restart_preserves_manual_identity_and_bypasses_policy_gates` | Proved locally |
| 2. Durable time / retry policy | Attempt/deadline migration, rollback and reopen → S `attempt_migration_preserves_existing_rows_and_is_a_no_op_on_second_start`, `finishing_attempt_timestamps_commit_and_roll_back_with_the_outcome`; all nine outcome categories × automatic/explicit atomic finish → R `finish_updates_attempt_deadline_and_phase_atomically_from_pre_finish_policy`; category/invocation pure table → P `next_eligibility_result_table_covers_every_category_and_invocation`; 60→120→240→480→900→900 for transport/protocol → P `network_retry_deadlines_start_at_sixty_double_and_cap_at_nine_hundred`, SSH `poll_unreachable_backoff_cap_and_explicit_failure`; rejected key/host attention → SSH `poll_rejected_key_attention_without_backoff`, `poll_unapproved_host_without_backoff`; larger interval/max-backoff clamp, one durable write and countdown with no reservation → P `rollback_clamp_uses_larger_of_interval_and_maximum_backoff`, `stored_clamp_is_written_once_and_counts_down_without_reserving`; malformed UTC values fail closed → S `attempt_columns_reject_non_integer_negative_and_unrenderable_timestamps`, integration `corrupt_poll_times_fail_closed_at_snapshot_reopen_gate_and_status` (remote_reservation target) | Proved locally |
| 3. Scoped fetch / guard / no push / complete batch | Exact primary/document/ticket objects and tracking, excluded malformed/outside objects, no tags/FETCH_HEAD/prune/unrelated refs, rewind tracking only → T `poll_fetch_set`; absent primary/empty set → T `poll_absent_primary_and_empty_set`; old-value race/noncommit/partial guarded write → T `poll_tracking_race`, `poll_noncommit_tip`, `poll_partial_guarded_write`; fetch advertisement consistency → T `poll_download_advertisement`; complete batch replacement/deletion authority → S `only_complete_batches_replace_current_and_deletion_retains_last_successful_oid`; batch + initial NotReached atomic/foreign-owner rollback → P `task6_atomic_batch_extension_rolls_back_and_rejects_foreign_owner`, `task6_disposition_insert_fault_rolls_back_advertisement_and_old_report`; no poll receive-pack advertisement/command/update → every one of 74 SSH cases, concretely `poll_all_three_fast_forward`, `poll_materialize_document_once`, `poll_materialize_ticket_once` | Proved locally |
| 4. Manual/local priority / safe stop / repeat safety | Advertisement/in-transfer automatic yield and successful manual retry → SSH `poll_automatic_advertisement_yields_to_manual`, `poll_automatic_in_transfer_yields_to_manual`; explicit operation Busy → SSH `poll_explicit_advertisement_makes_manual_busy`; D4 local-save retry/no new journal row → SSH `poll_local_save_requests_automatic_yield`, R `update_local_prepared_journal_guard_yields_without_creating_local_rows`; cancel/yield every update boundary, release/fence/unchanged policy → R `update_requests_at_every_boundary_release_slot_preserve_policy_and_fence_owner`; safe-point ordering/active one-slot/repeated local mutation → R `update_local_stage_is_ordered_repeatable_active_and_completion_requires_discovery`; partial cursor/report/unchanged completed local target on restart → SSH `poll_partial_batch_yield_report_and_restart`, P `task6_yield_between_targets_keeps_changed_primary_and_restart_is_current`; D3 real retained divergence blocks both modes with zero connections → SSH `poll_retained_real_divergence_conflict_blocks_both_invocations` | Proved locally |
| 5. Strict clean FF once | Primary/document/ticket/all-three branch/index/files/tracking plus second/current full refs and no new objects → SSH `poll_primary_fast_forward`, `poll_document_fast_forward`, `poll_ticket_fast_forward`, `poll_all_three_fast_forward`, `poll_already_current`; first-poll direct/symbolic sentinels retained in those five cases; pure equal/behind/ahead/diverged and ancestry errors → P `poll_relation_equal_behind_ahead_and_diverged_are_pure`, `poll_relation_surfaces_both_ancestry_errors_without_guessing`; local FF then exact current snapshot → P `poll_fast_forward_primary_and_context_advance_files_index_branch_then_are_current`; immediate local/tracking/config/endpoint reobservation → P `poll_fast_forward_reobserves_ref_tracking_configuration_and_endpoint_before_mutation`, SSH `poll_tracking_changed_before_target_fences` | Proved locally |
| 6. Exceptional local preservation / disposition | Tracked dirty/staged/untracked/conflicted/ahead/diverged → SSH `poll_tracked_dirty`, `poll_staged_dirty`, `poll_untracked_dirty`, `poll_git_conflicted_without_pending_record`, `poll_local_ahead`, `poll_diverged`; seven dirty/merge/index variants with full snapshots → P `poll_classification_dirty_and_conflicted_states_preserve_full_snapshot`; wrong context/primary branch and missing directory → SSH `poll_context_wrong_branch`, `poll_primary_wrong_branch`, `poll_missing_worktree_directory`; detached/unregistered/primary elsewhere/symlink → P `poll_wrong_or_detached_head_and_primary_not_at_root_are_inaccessible`, `poll_missing_and_unregistered_contexts_are_inaccessible_and_unchanged`, `poll_symlink_deterministic_path_is_inaccessible_without_following_it`; ignored destructive collisions → SSH `poll_ignored_directory_collision_preserved`, P `poll_retiring_directory_ignored_descendants_are_dirty_before_any_checkout`; foreign/admin/backlink ownership → SSH `poll_foreign_worktree_ownership_refused`, P `poll_context_identity_requires_exact_registered_admin_and_gitfile_backlink`; fixed guidance → P `poll_dispositions_have_only_fixed_guidance`; literal Unix permission-denied directory and .git access, Inaccessible, no local effects, full snapshot after guarded restoration, denied mode unchanged during poll and complete direct/symbolic refs with only fetched tracking advancement → SSH `poll_unix_permission_denied_worktree_preserves_local_state` | **Fully LOCAL-proved; Unix permission case executed on Linux, Windows ACL native proof pending** |
| 7. Once-only conforming materialization / comments / index | New document/ticket discovered once with canonical titles/comments, single branch/worktree/intent, no upstream; second/reopen/second service current → SSH `poll_materialize_document_once`, `poll_materialize_ticket_once`; concurrent Busy/reservation uniqueness then current retry → SSH `poll_concurrent_materialization_once`; lease race → P `poll_materialization_two_services_race_under_root_lease`; accepted tree ⇒ active discovery, invalid comments allowed, duplicate/wrong kind/id/mode/UTF-8/worktree subtree rejected → P `poll_tree_conformity_matrix_rejects_without_creation_and_all_accepted_are_discoverable`, `poll_conformity_cannot_hide_executable_duplicate_or_discovery_excluded_item`; ordered persisted/new/malformed snapshot after refresh → P `task6_ordered_existing_new_malformed_targets_materialize_and_snapshot_after_refresh` | Proved locally; accepted⇒active amendment only |
| 8. Exceptional remote state / no creation/deletion / scoped visibility | Nonconforming/malformed/outside → SSH `poll_nonconforming_tree`, `poll_malformed_family`, `poll_outside_family`; old deleted/new renamed nonconforming with unchanged local work → SSH `poll_remote_deleted_preserves_local_work`, `poll_remote_renamed_old_deleted_new_nonconforming`; pre-existing branch/path obstruction and lost-history preservation → SSH `poll_preexisting_branch_blocks_materialization`, `poll_preexisting_path_blocks_materialization`, `poll_history_unknown_after_registry_loss`; exhaustive local obstruction matrix → P `poll_materialization_local_obstruction_matrix_is_byte_exact_unchanged`; visibility of local-only HistoryUnknown disposition and persisted context → S `task6_local_history_unknown_context_also_exposes_its_latest_disposition`, `local_context_with_lost_history_is_projected_as_unknown_without_a_deletion`; dot-document selection → SSH `poll_dot_document_nonconforming`, P `poll_document_selection_matches_discovery_extensions_including_dotfiles` | Proved locally; out-of-family excluded from reports as contracted |
| 9. Interruption/reopen / Git-first unfinished-only reconciliation | Real partly downloaded PACK disconnect, prior complete batch/ref/ODB preservation, explicit unchanged retry deadline and automatic doubling, terminal replay zero connections then next poll completes → SSH `poll_disconnect_download_next_poll_finishes`, `poll_disconnect_partial_download_automatic_backoff`; tracking-before-batch/batch rollback/FF-before-row → SSH `poll_failure_after_tracking_writes_restart`, `poll_batch_commit_failure_atomic_restart`, `poll_fast_forward_before_disposition_failure_restart`; intent/branch/worktree/branch-record ambiguity/D2 partial checkout → SSH `poll_materialization_intent_failure_restart`, `poll_materialization_branch_failure_restart`, `poll_materialization_worktree_failure_restart`, `poll_materialization_branch_record_failure_retains_ambiguity`, `poll_failed_checkout_retains_partial_d2`; completion/index boundaries → SSH `poll_completion_transaction_failure_restart`, `poll_after_completion_failure_index_only_recovery`, `poll_refresh_failure_index_only_recovery`; abandoned/interrupted epoch fencing → R `update_restart_from_every_abandoned_or_interrupted_stage_reobserves_with_new_epoch`; completed ID config-independent replay → P `review_terminal_replay_precedes_removed_or_changed_live_configuration`, `task6_batch_completion_once_and_older_replay_keeps_its_times_but_no_targets`; original handoff targets/index-pending survive later poll → P `review_original_report_survives_second_poll_before_refresh_return`, `review_original_stop_report_survives_later_poll_and_index_repair`, `review_original_fenced_stop_keeps_pending_after_later_poll_repairs_index`; kind durable identity → S `poll_kind_direct_sql_changes_are_denied_and_preserve_the_entire_row_after_reopen`, `poll_kind_legacy_migration_freezes_observe_identity_and_preserves_rows_on_reopen` | Proved locally; actual failed-checkout fault Linux-only |
| 10. Index separation / no second fetch | Failure/crash retains authoritative poll result and durable stale flag, repair by ordinary fresh refresh ID, completed replay without transport → SSH `poll_after_completion_failure_index_only_recovery`, `poll_refresh_failure_index_only_recovery`; P `task6_index_failure_and_crash_leave_repairable_flag_without_poll_id_reuse`; original reports remain independently owned when next poll replaces stored targets → P `review_original_report_survives_second_poll_before_refresh_return` | Proved locally |
| 11. Privacy / rows / sidecars / diagnostics / output | Accepted document/ticket title absent from recovery rows, formatted result/snapshot and isolated raw output; legitimate canonical index title present in live DB and backup (not falsely forbidden in raw scans); body/endpoint/key/passphrase/rejected-title/malformed-name absent from rows/DB/nonempty WAL/VACUUM backup → SSH `poll_recovery_database_wal_backup_privacy`; output stdout, beyond-prefix stderr on failing child, clean failure, missing inventory, both accepted-title negative controls → SSH `poll_output_capture_negative_controls`; metadata schema/rollback/corruption/privacy and malformed name redaction → S `task6_disposition_schema_upgrade_rollback_corruption_and_privacy`, `malformed_response_names_never_enter_database_wal_backups_or_public_formatting`; all-case final privacy scans and exact fixed guidance → P `poll_dispositions_have_only_fixed_guidance`; type-only/mode-only/name-content-boundary snapshot negative controls → SSH `poll_snapshot_detects_type_only_changes`, `poll_snapshot_detects_permission_only_changes`, `poll_snapshot_detects_name_content_boundary_changes` | Proved locally with canonical index-content title exception |
| 12. Required gate / honest native evidence | Exact commands and runtime totals above passed on final changed Rust tree; two new SQL tests plus all named evidence tests pass in that gate; CLI exits 0; hashes unchanged; bounded desktop process startup above. Each native target below explicitly pending authorization; manual wiring proves no native execution | **Local gate PASS; native execution not proved** |

Cancellation supplements row 4/9 at each orchestration boundary: SSH
`poll_cancel_before_transport`, `poll_cancel_after_advertisement`,
`poll_cancel_between_observations`, `poll_cancel_before_batch`,
`poll_cancel_after_batch`, `poll_cancel_before_local_mutation`,
`poll_cancel_before_discovery`, `poll_public_cancel_held_advertisement`.
Frozen publication/endpoint/key changes: SSH
`poll_held_publication_remote_rename_fences`, `poll_held_endpoint_change_fences`,
`poll_held_selected_key_change_fences`. These also pass in the final gate and
prove no further local changes after a generation mismatch.

### Review Ranges, Scope And Pending Work

Complete whole-branch review base (origin/main at trusted preflight):
`d1ca88772c5768d55aadb2ca4dcac1c69dab0d17`. Review **base..returned checkpoint HEAD**,
not HEAD~1. Exact checkpoint SHA is returned after local commit; this heading is
its durable evidence anchor. Task 8 increment starts exact b0f0055 above.

| Scope | Exact complete range through prior approvals |
| --- | --- |
| Ticket/planning | `d1ca88772c5768d55aadb2ca4dcac1c69dab0d17..094e5734d1d4480ee29e6f675aaba4a8883b9060` |
| Task 0 + reviews | `094e5734d1d4480ee29e6f675aaba4a8883b9060..4c1590d73b9dbab17030670989bd8825328f1f55` |
| Task 1 | `4c1590d73b9dbab17030670989bd8825328f1f55..5ff9f4eee31cd88067a406ba8acbc317c0ee2045` |
| Task 2 | `5ff9f4eee31cd88067a406ba8acbc317c0ee2045..6cb69f4b6dd4f9798e509f9cdddc1acc7cd4f39a` |
| Task 3 + correction | `6cb69f4b6dd4f9798e509f9cdddc1acc7cd4f39a..34b97c178a48b39b6ae32e30de52ff5722d1dd06` |
| Task 4 + blockers/amendment/corrections | `34b97c178a48b39b6ae32e30de52ff5722d1dd06..8cb5d777758287b200e09255dc1fa7f254fc4da3` |
| Task 5 + blockers/exception/correction | `8cb5d777758287b200e09255dc1fa7f254fc4da3..ff43d2c3873e4c9cbd9f9f579c5cdd0ef4c513a7` (implementation review base `ab07d0d3096a0e6129add1ba32a2f977a131b80a`) |
| Task 6 + interface checkpoint/corrections | `ff43d2c3873e4c9cbd9f9f579c5cdd0ef4c513a7..4d89295070715d6a15ced2c08d72aff0dd0ee46f` |
| Task 7 + evidence/quality corrections | `4d89295070715d6a15ced2c08d72aff0dd0ee46f..b0f005531879c5ee2806ed13052bdd157df627d7` |
| Task 8 verification checkpoint | `b0f005531879c5ee2806ed13052bdd157df627d7..returned local checkpoint HEAD` |

Task 8 changes only state.rs, state_tests.rs, this ledger, Cycle evidence, open
ticket and canonical comments. Complete pre-checkpoint branch inventory is **78
files**; Task 8's two new comments make **80** at handoff. Includes 42 canonical
comments, one ticket, four planning/Cycle documents, API audit, one manifest,
one manual workflow, 20 src Rust files and 10 integration/support Rust files.
The final `git diff --name-only base..HEAD` output is the authoritative full-range
file list returned with the checkpoint; counts are verified before handoff.
No Task 8 dependencies/lockfiles, CLI/desktop, schemas, GPUI/runtime libraries,
transport/support or approved design/plan edits. Cycle change is evidence only.
D1–D4, accepted⇒active-only amendment and **sole Task 5 final-validator canonical
collection under the same lease** exception remain unchanged; network, prompts
and discovery/index-refresh scans stay outside.

Controller will dispatch Task 8 SPEC, whole-branch quality and independent
whole-branch review after this return. **Unrun here; no review approval or Task 8
completion claimed.** The literal permission-denied gap recorded at this first
checkpoint is closed by the owner-authorized follow-up above; native proof remains pending.
Only subsequently authorized review fixes with actual new Rust code require a
fresh final-tree full gate. Final approved-review recording, final review-ready
comment and Task 8 tick await controller review resolution/resume.

| Native target | Status |
| --- | --- |
| Linux x86_64 | PENDING AUTHORIZATION |
| Linux aarch64 | PENDING AUTHORIZATION |
| Windows x86_64 | PENDING AUTHORIZATION |
| Windows aarch64 | PENDING AUTHORIZATION |
| macOS aarch64 | PENDING AUTHORIZATION |

Build workflow is manual-only and wired for remote_polling, **undispatched**.
No native proof claimed from YAML/local Linux. No push, PR, merge, ticket closure,
worktree removal or review spawning. Ticket open, Task 8 unchecked. Local
review-pending checkpoint comment `01M4X5PT42E2TRWCPFM9Q8KAKM`.

## Task 7 Quality Corrections: Verified Re-Review Handoff — 2026-10-14T11:28:19Z

QUALITY `ses_ec608e461ffeB8pQQSieLJnzhE`'s **Important and both minors are
technically corrected**, awaiting **SPEC increment re-review first**, then the
same quality worker's re-review. Earlier Task 7 SPEC approval through
`650c827bef529c306bca9b377bb8cfed03efb490` is historical, not approval of this
increment. That exact SHA is the correction base; complete Task 7 base remains
`4d89295070715d6a15ced2c08d72aff0dd0ee46f`. Exact local HEAD returned after
authorized commit. Ticket open, Task 8 **not started**.

### Important: Portable Context Identity

Before edits, the owner's Linux alias reproducer really failed:
`devenv shell -- env TMPDIR=/proc/self/root/tmp cargo test --locked --test remote_polling poll_materialize_ticket_once`
— **exit 1, 0 passed/1 failed**, found.len() assertion at original line 967.
The fixture used raw temporary spelling while discovery stores canonical paths.

discovered_once now canonicalizes the expected worktree and each observed item
context, following `remote_merge_recovery.rs:1096–1102`. Item ID, exact one-item
count, title/comment IDs, branch/index/HEAD/tracking, registration and intent
assertions remain. Audited every new-fixture path comparison: the only other
file-identity equality is the VACUUM backup inventory, now canonical on both
sides too. WAL detection compares only the filename suffix. Snapshot registration
path hashing is deliberately exact before/after artifact observation, not a
raw-versus-canonical identity comparison; no service result is compared to a raw
path string or normalized by suffix matching.

Normal **and aliased** entire polling targets each pass **73 cases**, not only
the originally failing ticket test. Coverage includes document/ticket successful
materialization, concurrent/second-service/reopen/current paths, intent/branch/
worktree/branch-record/partial-checkout recovery, and both accepted-title privacy
indexing/backup/output paths. These are Linux alias-identity regressions, **not
macOS or Windows runtime proof**. Native verification remains pending.

### Minor 1: First-Poll Sentinel Preservation

stale_tracking now returns the actual before-poll sentinel inventory, checked
against the explicit expected states: `refs/remotes/origin/stale` is direct at
the initial base OID with no symbolic target; `refs/remotes/unused/HEAD` has no
direct OID and points symbolically to that stale ref. Each of the four clean
update cases and all-three current case captures these states **before the
first poll** and asserts exact retention **after that first poll**. Legitimate
main/document/ticket tracking creation/advancement remains allowed and is checked
against fetched tips. Existing second/current complete all_refs checks remain.
fetch.prune=true remains an adversarial fixture setting. This is stronger evidence
for already-correct no-prune behavior; **no artificial first-poll failure claimed**.

### Minor 2: Structural And Permission-Aware Snapshots

files() now hashes explicit directory/file/link/other tags, Unix permission bits
under cfg(unix), or readonly attributes under cfg(not(unix)). Directory entry
counts frame recursion; names, file data and link targets carry length prefixes.
Symlinks use symlink_metadata/read_link without traversal; special files are
never opened. Root directory attributes, Git config, HEAD/index/merge metadata,
linked-worktree gitfiles/admin artifacts and parked original checkout files
use the same structural observation. Missing metadata has an explicit marker.
Existing canonical/Git-metadata traversal exclusions remain unchanged.

Three new controls use actual ignored/unregistered fixture entries, so Git status
cannot supply the evidence missing from the filesystem hash:

| Exact test name | Actual change | RED → GREEN |
| --- | --- | --- |
| `poll_snapshot_detects_type_only_changes` | Empty regular file → empty directory, equal Unix permissions 700 / equal non-Unix permission attributes | **0/1 → pass** |
| `poll_snapshot_detects_permission_only_changes` | Unix 644→600 (no execute/content change), then restoration; non-Unix readonly toggle | **0/1 → pass** |
| `poll_snapshot_detects_name_content_boundary_changes` | Name/data partition `a`+`bc` → `ab`+`c` | **0/1 → pass** |

Each first run compiled and failed on equal before/after hashes **before helper
edits**; exact commands/RED checkpoint below. Controls assert behavioral detection,
not a particular digest or encoding. Permission restoration reestablishes the
original digest. Non-Unix readonly branch is present for native runs but was not
executed on Linux. All existing preservation cases still pass with stronger hashes.

### Exact Verification And Trial Accounting

| Exact command | Result |
| --- | --- |
| `devenv shell -- cargo test --locked --test remote_polling poll_snapshot_detects` | **3 passed** after three individual meaningful REDs |
| `devenv shell -- env TMPDIR=/proc/self/root/tmp cargo test --locked --test remote_polling poll_materialize` | **2 passed**, document + ticket |
| `devenv shell -- env TMPDIR=/proc/self/root/tmp cargo test --locked --test remote_polling poll_recovery_database` | **1 passed**, accepted document/ticket indexing/backup/privacy |
| `devenv shell -- cargo test --locked --test remote_polling` | **73 passed**, 0 failed, normal temporary roots |
| `devenv shell -- env TMPDIR=/proc/self/root/tmp cargo test --locked --test remote_polling` | **73 passed**, 0 failed, aliased temporary roots |
| `devenv shell -- cargo check --all-features --locked` | PASS |
| `devenv shell -- cargo fmt --check` | PASS |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | PASS |
| `devenv shell -- cargo test --all-features --locked` | **1,763 passed**, 0 failed, 1 existing ignored |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | PASS, exit 0, no window |
| `scripts/show-ticket 01K7F6H9J2N4Q6S8V0X2Z4B6DG` | PASS, open ticket and all three new correction comments rendered, no malformed diagnostics |
| `git diff --check` | PASS |

Post-gate CLI/canonical/preservation log `tool_13a2ee9a6001R6NmSrOVC0qi7S`;
production/support/manifest/lockfile/workflow/approved-contract preservation checks
also exit 0. No Rust edit after the gate or during this canonical bookkeeping.

Final required check/fmt/Clippy/full-test gate ran sequentially after all Rust edits,
full log `tool_13a0d5ef50016datImfI6Kva9B`: **613 library + 779 ordinary integration
+ 362 SSH-target cases + 9 doctests**. Includes polling **73**, observation **15**,
synchronization **63**, merge recovery **25**, comment publication **41**, fixture
**31**, transport **114**. No new production/library tests or dependency changes.
The three new cases are fixture evidence controls. Full normal and alias targets
also prove existing real-SSH privacy and zero poll receive-pack assertions remain.

Only failures in this correction are the **four intentional, meaningful REDs**:
original aliased ticket assertion and three snapshot negative controls. No compile,
setup, accidental trial, Clippy failure or timeout after RED; all GREEN runs passed
first time. No fake transport, sleeps, developer keys, global configuration edits
or production/test-hook changes. Environment overrides are inside Devenv. No Rust
edit after final gate; only canonical bookkeeping follows.

### Review Scope And Lifecycle

Only tests/remote_polling.rs, ledger, open ticket and three Node-ULID comments
change from the correction base. Production, tests/support, other SSH case bodies,
manifest/lockfiles, workflow and approved Cycle/design/plan have empty diffs.
Prior Task 0/preflight and Task 6 approvals remain trusted, D1–D4/sole Task 5
validator lease exception unchanged. Carry **Task 2 SQL-kind immutability** and
**Task 6 duplicate disposition attachment** minors explicitly to final Cycle
review, separate from the two Task 7 minors now corrected. SPEC increment re-review
precedes quality re-review by `ses_ec608e461ffeB8pQQSieLJnzhE`; no Task 8 until
reviews. All five native executions pending; no dispatch/push/PR/merge/closure/
cleanup or subagents. Re-review-ready comment `01M4X2Q740QMGYS5E2FCG3BQMF`.

## Task 7 Quality Corrections: Snapshot Controls RED — 2026-10-14T10:42:29Z

Added three actual-filesystem negative controls **before changing files()/snapshot**.
They operate under an ignored, unregistered fixture subtree so Git status does
not independently detect the change and mask a weak filesystem hash. Exact runs,
each **exit 1, 0 passed/1 failed** at assert_ne on equal before/after snapshots:

| Exact command | Missing evidence |
| --- | --- |
| `devenv shell -- cargo test --locked --test remote_polling poll_snapshot_detects_type_only_changes` | Empty regular file replaced by empty directory, same Unix permission bits (700) / non-Unix attributes |
| `devenv shell -- cargo test --locked --test remote_polling poll_snapshot_detects_permission_only_changes` | Unix mode 644→600, no execute/content change, then restored; non-Unix control toggles readonly |
| `devenv shell -- cargo test --locked --test remote_polling poll_snapshot_detects_name_content_boundary_changes` | `a` containing `bc` vs `ab` containing `c`, same concatenated bytes |

All three new controls compile and fail meaningfully against the original helper,
not fixture setup. Existing Important alias-path RED is already recorded above.
No production source edit or helper/portability correction preceded these REDs.
Proceed with length-delimited typed/permission-aware snapshot encoding, canonical
path comparison and initial sentinel checks. Minor 1 strengthens already-delivered
no-prune behavior, so no artificial failure claimed for it. Canonical RED checkpoint
`01M4X03A3WCAP5F16EGZK6AQNR`; ticket open, Task 8 unstarted.

## Task 7 Quality Corrections: Reproduced Important RED — 2026-10-14T10:39:59Z

Controller/user reports Task 7 SPEC approval through
`650c827bef529c306bca9b377bb8cfed03efb490` (historical approval). QUALITY
`ses_ec608e461ffeB8pQQSieLJnzhE` **BLOCKED one Important and two minors**.
Fix all three inline, no subagents, at this verified clean exact correction base
in the assigned worktree/branch. Complete Task 7 base stays
`4d89295070715d6a15ced2c08d72aff0dd0ee46f`. Corrected increment needs SPEC
re-review before the same quality reviewer's re-review; Task 8 unstarted.

Important verified before any test/helper edit: discovered_once compares the
service's canonical i.context with a raw temporary Side.root-derived path. The
reference merge-recovery fixture canonicalizes both paths instead. Meaningful
reproduction command, **exit 1, 0 passed/1 failed**, assertion at found.len():

`devenv shell -- env TMPDIR=/proc/self/root/tmp cargo test --locked --test remote_polling poll_materialize_ticket_once`

This is an actual test portability bug, not a production defect or fake RED.
Will audit other new-fixture raw/canonical comparisons and verify document,
ticket, concurrency/materialization recovery and privacy indexing under normal
and aliased temporary roots. Environment override is inside the Devenv shell.

Minor 1 verified: sentinels are installed before initial fetch but the full ref
baseline is only taken afterward, so first-poll deletion/mutation escapes. Capture
exact direct/symbolic sentinel states before first poll and check after it while
allowing legitimate fetched tracking tips. Minor 2 verified: files() concatenates
names/content without type, permission or length/structure encoding. Add actual
type-only/permission-only/field-boundary negative controls before hardening the
snapshot helper, then record their meaningful RED. No production/dependency/design
change. Ticket open, native pending, prior Task 0/preflight/Task 6 approvals trusted;
both earlier deferred minors retained. Entry comment `01M4WZYQSB8Z088S64MG3Z9RJ4`.

## Task 7 SPEC Evidence Corrections: Verified Re-Review Handoff — 2026-10-14T10:22:41Z

All three P2 evidence findings from `ses_ec64fd841ffe4JHjZ3XJ7I1MOp` are
**technically corrected, awaiting SPEC re-review; quality has not started**.
Incremental correction base `6132090ba3d8930ad42ca86e2a99e139603dfd4b`; complete
Task 7 review base `4d89295070715d6a15ced2c08d72aff0dd0ee46f`. Exact new local
HEAD returned after authorized commit. No production behavior change or design
amendment. Historical Task 7 evidence/limitations below remain recorded and are
superseded only where this correction strengthens the named acceptance checks.

### 1. Complete Ref Inventory On Current/Second Polls

`all_refs` now records **every ref name, direct OID and symbolic target**. The
four clean-update cases compare that complete inventory before/after the second
poll, along with object inventory/local snapshots and renewed branch/index/files/
tracking assertions. Current acceptance now includes primary, document and ticket
together; only the initial real poll may introduce context tracking refs, then
two more polls must preserve the complete inventory. Once-only materialization
second/reopen/second-service current polls also compare complete inventories.
Each clean/current fixture plants an unadvertised `refs/remotes/origin/stale` and
unrelated symbolic `refs/remotes/unused/HEAD`, with fetch.prune=true, so deletion
or symbolic-target changes cannot hide behind the local-only snapshot. The
snapshot still intentionally permits legitimate tracking writes during initial
fetch/exceptional-state processing; no preservation assertion was weakened.

Named checks: `poll_primary_fast_forward`, `poll_document_fast_forward`,
`poll_ticket_fast_forward`, `poll_all_three_fast_forward`, `poll_already_current`,
`poll_materialize_document_once`, `poll_materialize_ticket_once`.
Initial strengthened clean/current runs were already GREEN, not fabricated RED.

### 2. Proven Partial PACK Download Before Disconnect

The real upload relay gains an **opt-in byte-qualified disconnect fault**,
installed only while the existing 131,072-response-byte barrier is held. The
controller first asserts at least 65,536 actual **sideband-1 PACK payload bytes**
were forwarded; ACK/control packets, framing and progress text do not count.
Only after this proof does it install the fault and release the barrier. A
separate counter proves exactly one byte-qualified disconnect executed. Transport
still uses real russh/Git helpers, with no sleeps, fabricated transfer, production
hook or change to existing fixture fault behavior when the new option is absent.

`poll_upload_pack_counter_fragmentation` verifies byte observation across widths
1/2/3/7/16,384, progress/ACK exclusion, incomplete packets counted only on complete
forwarding, and malformed input fail-closed/sticky bounded observation. It tests
the fixture parser only, not a synthetic transport. Unrelated relay output is
never modified and unsupported packet output simply stops counting.

Both `poll_disconnect_download_next_poll_finishes` (explicit) and new
`poll_disconnect_partial_download_automatic_backoff` retain a real prior completed
batch. After partial download they assert all refs, local Git/files/config and
the complete prior batch/ref-observation/disposition rows unchanged. Every old
ODB object survives, every retained object reads valid, and additional valid
partial-download objects are permitted and counted, not mistaken for ref-update
authority. Actual Linux observations in focused and final runs were **131,029
PACK payload bytes, 0 new indexed objects, incoming commit not indexed** for each
invocation. Those object observations are reported, not imposed as a universal
backend requirement.

Each failed record is `failed`/`transport_unavailable`. Automatic backoff doubles
120→240 with completion-based deadline; explicit failure retains backoff=120 and
deadline=12345. Reopened service replays the failed ID with zero new connections
and unchanged prior batch, then a fresh explicit poll completes the unfinished
download/FF and verifies branch/index/worktree/tracking. Poll-phase receive-pack
advertisement/command/update counters remain at their baseline throughout.
All three initial byte-fault/parser checks were already GREEN.

### 3. Accepted-Title Privacy With Explicit Canonical-Index Scope

Valid collaborator document and ticket contexts now use distinct accepted-title
sentinels and real body-sentinel comments. **Metadata probes** include these titles
for every recovery/operation row, formatted results/reports/RemoteSnapshot and
isolated raw output. **Whole-store probes** still include the rejected TITLE,
BODY, endpoint, passphrase/key and malformed-name probes, but permit accepted
titles because discovered_items legitimately indexes canonical titles. No body,
credential, endpoint or rejected-title probe was removed from any raw scan.

The privacy journey materializes both accepted contexts successfully, verifies
their comments/discovery and diagnostic RemoteSnapshots, and proves the accepted
titles are really indexed in the live database and VACUUM INTO backup. Recovery
rows in both live registries and the backup are scanned with the stronger metadata
set. Negative controls inject each accepted title into a fixture-only backup
recovery table: the same row scanner must reject it, then the fixture table is
removed and the backup recovery scan passes again. Raw generated DB/WAL/backup
scans continue to reject every non-index-content sentinel. **No claim of raw DB/
WAL/backup absence of legitimate accepted indexed titles is made.**

Existing materialization/replay results are scanned with accepted-title probes.
`poll_output_capture_negative_controls` now also proves exact OutputPrivacy for
accepted document title on stdout and accepted ticket title beyond the prefix on
stderr of a failing child, alongside all four original controls (six total).
Those private child controls are not counted as ordinary passing target cases.

### First Trials And Fixture Failure Accounting

No production defect found/fixed; **no production behavioral RED claimed**.
All Rust commands below used Devenv, sequentially:

| Exact command | First result / correction evidence |
| --- | --- |
| `devenv shell -- cargo test --locked --test remote_polling fast_forward` | **5 passed**, including four strengthened clean cases and existing disposition-failure case |
| `devenv shell -- cargo test --locked --test remote_polling poll_already_current` | **1 passed** |
| `devenv shell -- cargo test --locked --test remote_polling poll_upload_pack_counter_fragmentation` | **1 passed** |
| `devenv shell -- cargo test --locked --test remote_polling poll_disconnect` | **2 passed**, measured 131,029 payload bytes/0 new indexed objects each |
| `devenv shell -- cargo test --locked --test remote_polling poll_materialize` | **2 passed** with accepted-title result/row/output probes |
| `devenv shell -- cargo test --locked --test remote_polling poll_recovery_database` | First **0 passed/1 failed**, then five diagnostic **0/1** runs; after fixture correction **1 passed** |
| `devenv shell -- cargo test --locked --test remote_polling poll_output_capture` | **1 passed**, all six negative controls |

The sole failure was **fixture invalidity**, not title leakage or a production
bug: the new combined successful document/ticket journey reused comment ID C for
two different items. Global item/comment ID uniqueness correctly made the cache
unreadable and refresh IndexPending. The first numerical diagnostic pinned failure
to the first combined materialization; a query for an unfinished refresh row found
none, then retry returned fixed IndexUnavailable before journaling. Source
`repository.rs:6010–6014` explicitly rejects globally reused item/comment IDs.
Use a distinct canonical document-comment ID, matching its filename and discovery
assertion; the combined valid fixture then succeeds. Five temporary numerical-only
diagnostic trials are retained honestly; all diagnostic/retry plumbing was removed.
No compile failure, timeout, Clippy failure or production patch in this correction.

### Final Submission Verification And Handoff

After all Rust/test edits, final sequential commands:

| Exact command | Result |
| --- | --- |
| `devenv shell -- cargo test --locked --test remote_polling` | **70 passed**, 0 failed: original 68 cases plus automatic partial-download and fixture-counter cases |
| `devenv shell -- cargo check --all-features --locked` | PASS |
| `devenv shell -- cargo fmt --check` | PASS |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | PASS |
| `devenv shell -- cargo test --all-features --locked` | **1,760 passed**, 0 failed, 1 existing ignored |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | PASS, exit 0, no window |
| `scripts/show-ticket 01K7F6H9J2N4Q6S8V0X2Z4B6DG` | PASS, open ticket and both correction comments rendered, no malformed diagnostics |
| `git diff --check` | PASS |

CLI/canonical-validation log `tool_139f21b19001QDfY3UKlfyuSCt`. Production,
manifest/lockfile, workflow and all three approved-contract preservation diff
checks also exit 0. These post-gate operations do not change the Rust tree.

Full gate log `tool_139d2679f0017Gt3lDgqTITLXk`: **613 library + 779 ordinary
integration + 359 SSH-target cases + 9 doctests**. It includes **70 polling,
15 observation, 63 synchronization, 25 merge recovery, 41 comment publication,
31 fixture and 114 transport**. Existing case bodies in other targets are
unchanged, all passing with the additive fixture observer/fault. Their prior
standalone Task 7 checks remain recorded below; not falsely claimed rerun as
standalone commands here. Full gate completed within its 3,600,000 ms bound;
no outer timeout in this correction. No Rust edit after this gate.

Only remote_polling.rs, ssh_remote.rs, ssh_server.rs, ledger/open ticket and two
new canonical Node-ULID comments change from the correction base. Production,
dependencies/lockfiles, workflow, approved Cycle/design/plan and existing SSH test
bodies remain unchanged. Return complete Task 7 range for SPEC re-review by
`ses_ec64fd841ffe4JHjZ3XJ7I1MOp` before quality. Native execution remains pending
for all five targets; workflow still manual-only and undispatched. Task 8 unstarted,
ticket open, no subagents/push/PR/merge/closure/cleanup. Task 2 SQL-kind immutability
and Task 6 duplicate disposition attachment minors remain explicitly deferred to
final Cycle review. Re-review comment `01M4WYZ203H6WF06FST9VFHZH0`.

## Task 7 SPEC Evidence Corrections: Entry — 2026-10-14T09:21:28Z

Reviewer `ses_ec64fd841ffe4JHjZ3XJ7I1MOp` reports **three P2 evidence gaps**;
quality has not started. Verified clean fix base
`6132090ba3d8930ad42ca86e2a99e139603dfd4b`, same assigned branch/worktree.
Complete Task 7 review base remains `4d89295070715d6a15ced2c08d72aff0dd0ee46f`.
Task 0/preflight and Task 6 approvals trusted; no rebase or repeated characterization.

Source confirms all three: local preservation snapshots intentionally exclude
remote refs, but current/second-poll cases have no full inventory comparison;
disconnect_at(Transfer) runs before the first transfer response is forwarded;
accepted materialization titles are ordinary unprobed content and only a rejected
title is a privacy sentinel. These are test-evidence gaps, not established
production defects. Harden complete ref inventories, disconnect only beyond a
real byte barrier, and separate accepted-title recovery/diagnostic/output probes
from whole-store probes that must permit canonical indexed titles. Record actual
first-run results; no artificial behavioral RED. Existing cases/production
semantics remain unchanged. Fix inline, no subagents; spec re-review before quality.
Ticket open, Task 8 unstarted. Entry comment `01M4WVEZ4CAG09BJ2NT1G3XYE1`.

## Task 7 Delivered Locally For Independent Review — 2026-10-14T09:07:32Z

**Task 7 review base:** `4d89295070715d6a15ced2c08d72aff0dd0ee46f`.
Exact local delivery HEAD returned after the authorized commit. All 68 new
real-SSH cases pass on Linux. Task 7 implemented; independent spec then quality
review pending. Task 8 has **not** started. Ticket remains **open**.

### Implementation And Boundaries

New `tests/remote_polling.rs` links the public library normally (no source
inclusion or cfg(test) access), uses the existing russh/real Git helper fixture,
and registers as `harness=false`. Two independent authenticated clones have
separate roots/services/registries. Peer Git setup/publishing/deletion/rename is
outside each poll's counter scope. Every poll checks a settled baseline for zero
new receive-pack advertisements, commands and ref updates; every case finishes
with privacy scanning. Held-worker tests check the same baseline before manual
retry, since deliberate synchronization may publish legitimately.

Necessary **test-fixture-only** additions in support files: real accepted-TCP
connection counter; opt-in transfer hold after 131,072 bytes forwarded by real
upload-pack; and opt-in single-character isolation-directory names (including
nested privacy controls). Existing targets keep their original path mode and
all holds default absent. Pacing plus the byte barrier proves a negotiated,
partly transmitted download, not just an advertisement stop. Controllers use
bounded channels/holds and helper-audit barriers; no acceptance-test sleep,
developer keys, global Git config mutation or synthetic transport. These are
fixture plumbing, not changes to shared production helpers.

Registry triggers request stops/fail atomic transitions; existing public
FailurePoint injects materialization/index failures. The existing public
PollBeforeTarget boundary callback installs an actual external tracking-ref
change outside all guards; its report, batch, branch, fetched commit and
tracking OID are checked. Cancellation also exercises public cancel while the
advertisement is held. No production file, dependency/lockfile, versioned DTO
schema, CLI/desktop, API audit or approved Cycle/design/plan changed.

Preservation snapshots hash worktree bytes, HEAD/index/merge metadata, statuses,
local refs, shared Git config, registration/admin artifacts and deterministic
obstructions. Missing/foreign fixtures include their parked original checkout.
Tracking refs may advance only as fetched; explicit all-ref assertions cover
stops before tracking writes. Materialization checks exact branch/HEAD/index,
single deterministic registration, comments/discovery, no upstream config and
immutable original intent owner/target across second/reopen/second-service calls.
Concurrent Busy proves only one reservation record before retry.

D2 actual failed checkout is Linux-specific: a regular `a` is written, overlong
symlink `z` fails, branch/gitfile/admin/registration survive and index is absent.
Reopen/restart and the next poll retain byte-identical partial artifacts and
MaterializationIncomplete. This case is not native Windows/macOS proof. D3 uses
an actual divergent canonical ticket synchronization with retained conflict;
both invocation modes add zero connections/operations or Git effects.
D4 local save requests yield without a new journal row and succeeds on retry.

Privacy scans cover every remote/operation recovery table (including dispositions
and intents), generated stores, a live nonempty WAL, VACUUM INTO backup, formatted
results/snapshots and isolated raw child output. BODY/passphrase/endpoint/private
key/malformed-name canaries are never cached. Ordinary canonical titles are
intentionally discovery content: the title sentinel is used in an executable
**rejected** context tree, so a whole-database title scan tests metadata privacy
without confusing legitimate indexed titles with recovery state. No recovery
scan omits title probes. Nested controls prove stdout leak, beyond-prefix stderr
leak on failure, clean failure and missing inventory all fail with exact categories.

### Exact Passing Task 7 Case Inventory (68)

All names below passed in `devenv shell -- cargo test --locked --test remote_polling`
and again in the required all-features submission gate:

| Group | Exact test names | Passed |
| --- | --- | ---: |
| Clean + current | `poll_primary_fast_forward`, `poll_document_fast_forward`, `poll_ticket_fast_forward`, `poll_all_three_fast_forward`, `poll_already_current` | 5 |
| Once-only materialization | `poll_materialize_document_once`, `poll_materialize_ticket_once`, `poll_concurrent_materialization_once` | 3 |
| Local exceptional state | `poll_tracked_dirty`, `poll_staged_dirty`, `poll_untracked_dirty`, `poll_git_conflicted_without_pending_record`, `poll_local_ahead`, `poll_diverged`, `poll_context_wrong_branch`, `poll_missing_worktree_directory`, `poll_primary_wrong_branch` | 9 |
| Remote exceptional state | `poll_remote_deleted_preserves_local_work`, `poll_remote_renamed_old_deleted_new_nonconforming`, `poll_nonconforming_tree`, `poll_malformed_family`, `poll_outside_family`, `poll_preexisting_branch_blocks_materialization`, `poll_preexisting_path_blocks_materialization`, `poll_history_unknown_after_registry_loss` | 8 |
| Prior hardening acceptance | `poll_ignored_directory_collision_preserved`, `poll_foreign_worktree_ownership_refused`, `poll_dot_document_nonconforming` | 3 |
| Policy/attention | `poll_policy_disabled`, `poll_policy_paused`, `poll_policy_recovery_suspended`, `poll_policy_not_due`, `poll_unreachable_backoff_cap_and_explicit_failure`, `poll_rejected_key_attention_without_backoff`, `poll_unapproved_host_without_backoff`, `poll_locked_blocked_session_no_automatic_prompt` | 8 |
| Arbitration/D3/D4 | `poll_automatic_advertisement_yields_to_manual`, `poll_automatic_in_transfer_yields_to_manual`, `poll_explicit_advertisement_makes_manual_busy`, `poll_local_save_requests_automatic_yield`, `poll_retained_real_divergence_conflict_blocks_both_invocations` | 5 |
| Cancellation | `poll_cancel_before_transport`, `poll_cancel_after_advertisement`, `poll_cancel_between_observations`, `poll_cancel_before_batch`, `poll_cancel_after_batch`, `poll_cancel_before_local_mutation`, `poll_cancel_before_discovery`, `poll_public_cancel_held_advertisement` | 8 |
| Partial cursor/restart | `poll_partial_batch_yield_report_and_restart` | 1 |
| Interruption/reopen | `poll_disconnect_download_next_poll_finishes`, `poll_failure_after_tracking_writes_restart`, `poll_batch_commit_failure_atomic_restart`, `poll_fast_forward_before_disposition_failure_restart`, `poll_materialization_intent_failure_restart`, `poll_materialization_branch_failure_restart`, `poll_materialization_worktree_failure_restart`, `poll_materialization_branch_record_failure_retains_ambiguity`, `poll_failed_checkout_retains_partial_d2`, `poll_completion_transaction_failure_restart`, `poll_after_completion_failure_index_only_recovery`, `poll_refresh_failure_index_only_recovery` | 12 |
| Frozen inputs | `poll_held_publication_remote_rename_fences`, `poll_held_endpoint_change_fences`, `poll_held_selected_key_change_fences`, `poll_tracking_changed_before_target_fences` | 4 |
| Privacy | `poll_recovery_database_wal_backup_privacy`, `poll_output_capture_negative_controls` | 2 |

### Final Sequential Verification

| Exact command | Result |
| --- | --- |
| `devenv shell -- cargo test --locked --test remote_polling` | **68 passed**, 0 failed |
| `devenv shell -- cargo test --locked --test remote_observation` | **15 passed**, 0 failed |
| `devenv shell -- cargo test --locked --test remote_synchronization` | **63 passed**, 0 failed, standalone rerun after outer chain timeout |
| `devenv shell -- cargo test --locked --test remote_merge_recovery` | **25 passed**, 0 failed |
| `devenv shell -- cargo test --locked --test comment_publication` | **41 passed**, 0 failed |
| `devenv shell -- cargo check --all-features --locked` | PASS |
| `devenv shell -- cargo fmt --check` | PASS |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | PASS |
| `devenv shell -- cargo test --all-features --locked` | **1,758 passed**, 0 failed, 1 existing ignored |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | PASS, exit 0, no window |
| `scripts/show-ticket 01K7F6H9J2N4Q6S8V0X2Z4B6DG` | PASS, open ticket and both new canonical Node-ULID comments rendered; no malformed diagnostics |
| `git diff --check` | PASS |
| `git diff --exit-code -- src Cargo.lock devenv.lock docs/Cycles/wave-02-cycle-08-one-shot-polling-and-materialization.md docs/plans/2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-design.md docs/plans/2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-implementation.md` | PASS, production/lockfiles/approved contracts unchanged |

Final required check/fmt/Clippy/full-test order is sequential, log
`tool_1398d3266001W2O7VUoM1Aliyb`: **613 library + 779 ordinary integration +
357 real-SSH (68 poll, 15 observation, 63 synchronization, 25 merge, 41 comment,
31 fixture, 114 transport) + 9 doctests**. Initial check/fmt/Clippy also passed
before the last three acceptance-assertion tightenings. Those edits and fixture
trial fixes justified the final gate. No Rust change after the final gate.

### Honest Trial/RED Accounting

**No production bug found or production behavior changed; no behavioral RED
claimed.** Acceptance tests exercise already-delivered behavior. Fixture/test
failures below are retained as trial evidence, not mislabeled bug regressions:

- Initial `devenv shell -- cargo test --locked --test remote_polling` did not
  compile: one Rust string escape and one temporary-ODB lifetime. Fixed tests.
- Successive same-command trials stopped at **3 passed/1 failed** (all-three
  clean assertion included IGNORED worktree directory), **5/1** (intent query
  used `oid` instead of `target_oid`), **12/1** (ahead fixture created a sibling
  commit rather than publishing the base), **16/1** (primary-wrong fixture also
  advanced a clean context), **18/1** (renamed tree happened to contain the seed
  document's valid ID). Corrected setup/assertions; next trial **36 passed**.
  The all-three focused diagnostic trial was **0 passed/1 failed**, confirming
  status bit 16384 (IGNORED), not a dirty checkout or production failure.
- Expanded target compile trial: one mismatched match-arm return type in the
  selected-key fixture. Corrected; `devenv shell -- cargo test --locked --test remote_polling poll_automatic`
  **2 passed**. Expanded full trial **40/1**: retained-conflict check consulted
  a cached libgit2 Index; reopening the actual checkout proves installed stages.
  `devenv shell -- cargo test --locked --test remote_polling poll_retained` then **1 passed**.
- `devenv shell -- cargo test --locked --test remote_polling poll_cancel` initial **0/1**: setup's initial
  refresh-required flag caused IndexPending. Explicit initial public refresh
  fixes baseline; next **7 passed**. Later final-tightening full trial **41/1**:
  seeded backoff was legitimately reset by the first endpoint-generation setup.
  Establish endpoint generation with a real successful poll first, then seed
  metadata. Final 7 boundary cases preserve backoff=120/deadline=12345.
- `devenv shell -- cargo test --locked --test remote_polling failure` initial **9/1**: post-completion repair
  selected an old completed refresh ID. Select only unfinished ordinary refresh
  records (otherwise new ID); next **11 passed**. No production recovery change.
- `devenv shell -- cargo test --locked --test remote_polling poll_held` two **0/1** trials: Git fixture remote
  rename itself moves tracking refs. Numeric diagnosis confirmed Transport
  stop; compare all refs after the fixture's rename, before releasing the hold.
  Next **3 passed**, with no poll ref write or batch.
- `devenv shell -- cargo test --locked --test remote_polling poll_recovery_database` initial **0/1**: no live
  nonempty WAL when the last connection closed. Hold a WAL connection across
  actual polls; next **1 passed**, with backup/nonempty WAL inventory assertions.
- Short-path/full trial **35/1**: blocked-session test additionally demanded no
  TCP handshake, although host verification precedes blocked authentication.
  Required no prompt/no Git helper still holds; removed only that extra TCP
  assertion. Eligibility and D3 zero-connection tests use the new real counter.
  Next full target **68 passed**; final strengthened-tree full target **68 passed**.
- Additional focused commands, each **1 passed**:
  `devenv shell -- cargo test --locked --test remote_polling poll_partial`;
  `devenv shell -- cargo test --locked --test remote_polling poll_failed_checkout`;
  `devenv shell -- cargo test --locked --test remote_polling poll_output_capture`;
  `devenv shell -- cargo test --locked --test remote_polling poll_tracking_changed`;
  `devenv shell -- cargo test --locked --test remote_polling poll_disconnect`.
- Final dedicated five-target command chain (each exact command above joined
  sequentially) exceeded its **600,000 ms outer tool timeout** during synchronization,
  after poll **68**, observation **15**, and 45 synchronization success lines.
  This is **not** a completed synchronization pass. No test failure printed.
  The standalone synchronization rerun used 1,200,000 ms and passed all **63**;
  merge/comment subsequently passed **25/41**. Final required four-command gate
  used 2,400,000 ms and completed with the full results above. No concurrent
  Devenv commands or repeated unchanged-tree success run beyond mandated checks.

### Review And Native Handoff

Manual-only `.github/workflows/build.yml` appends `--test remote_polling` to the
shared native command for all five matrix targets. **No workflow dispatched;
all five native executions pending.** Linux-only D2 checkout fault is reported
as such. Task 6 approvals recorded at entry remain trusted. Carry **Task 2
direct-SQL kind-immutability regression minor** and **Task 6 duplicate disposition
attachment minor** explicitly into final Cycle review (first audit return still
needs attachment; normal path currently does two passes). No cleanup for those
minors in Task 7. No Task 8, subagents, design change, push/PR/merge/closure or
worktree removal. Canonical review-ready comment `01M4WTNE2QC4091RCJ9KM6YYNM`.

## Task 7 Entry And Task 6 Approvals — 2026-10-14T07:26:16Z

Fresh Task 7-only implementer, inline, no subagents. Clean exact base/HEAD
`4d89295070715d6a15ced2c08d72aff0dd0ee46f`, specified ticket branch/worktree
verified. Prior controller preflight and Task 0 trusted; no repeated fetch/rebase
or characterizations. Inspected approved plan, current APIs and real SSH fixtures.

Controller/user reports Task 6 SPEC `ses_ec74e7b1cffe52NiF8TdyiLNkR` **APPROVE**,
then quality `ses_ec6bed51effeNWgnoaPfQ9pdlV` **APPROVE** through this base.
Historical pending-review statements below are superseded. Carry quality's one
nonblocking minor explicitly to final Cycle review: state.rs 5208–5219/5256–5267
duplicates disposition attachment on the normal return; the first audit return
still needs its pass. Also retain Task 2 direct-SQL kind-immutability regression
minor. Neither is required cleanup for Task 7.

Task 7 acceptance tests exercise already-delivered public behavior. No artificial
acceptance-target RED is required or claimed; any meaningful discovered bug must
have recorded behavioral RED before a narrow production correction. Public API,
real in-process russh/helper transport, counter baselines, barriers/registry
triggers and public FailurePoint only. D1–D4 and sole Task 5 final-validator
lease exception unchanged. Native workflow wiring only; native execution pending.
Task 8 unstarted, ticket open. Entry comment `01M4WMW0XX9QSW6XWAXBKBQDDK`.

## Task 6 Spec Corrections: Final GREEN And Re-Review Handoff — 2026-10-14T06:48:06Z

All four findings from `ses_ec74e7b1cffe52NiF8TdyiLNkR` are **technically corrected,
awaiting SPEC re-review**. Quality has **not started**. Incremental fix base
`72a6260853fe5f9f49b0342a602f999f990a01bc`; complete Task 6 base
`ff43d2c3873e4c9cbd9f9f579c5cdd0ef4c513a7`, including a70a2ab and initial feature.
Return exact local correction HEAD after commit; review the complete range, not
only the new commit. No amendment to the approved Cycle/design/implementation plan.

### Corrected Findings

1. **P1 eligibility:** stored terminal replay alone bypasses policy. Active or
   interrupted automatic restart checks disabled/paused/recovery suspension/not-due
   before any current configuration work or owner re-acquisition. Public regressions
   prove all eight phase/policy cases with zero operation/policy writes and unchanged
   epoch/outcome. Manual-ID explicit restart still bypasses automatic policy.
2. **P2 replay identity:** registered root/op-ID lookup and immutable stored action/
   kind/invocation validation precede live plan lookup. Removing selection/config
   or changing remote/primary leaves terminal original report untouched, for both
   restart values, with zero attempt/outcome writes. Invocation/observe-kind mismatch
   still refuses before consulting missing live configuration.
3. **P2 owned result:** safe-point, batch and finish callbacks capture the original
   report inside the existing ownership-checked transaction before release. Capture
   clones only metadata/target shadow; existing observe/sync entry points use identity
   callbacks with unchanged checks/acknowledgment/policy writes. Shadow rows and the
   committed-local-change flag advance only after durable row writes. Success/index
   failure return that same captured outcome; cancel/yield preserve first FF and
   NotReached remainder even when a later explicit poll replaces the set and scans.
   A foreign generation fence also retains the original handoff's IndexPending
   status after later repair, without a new persistent field or original scan.
   Only later older-ID replay receives empty targets. Fenced readback verifies
   scope/repository/operation/generation/epoch; it cannot adopt a restarted owner.
4. **P2 envelope/data loss:** table, owner trigger and both replay columns are one
   all-present/all-absent envelope. Every one of 14 recognizable partial shapes
   refuses, retaining schema/rows with no recreation. Genuine legacy migration
   preserves original operation fields and freezes only its pre-envelope updated_at
   time fallback once, with unknown original deadline left NULL. Modern NULL
   completion is RecoveryRequired at read/startup, not inferred from updated_at.
   Update-kind generation fencing records actual completion/deadline in its own
   existing transaction. Observe/sync metadata semantics are unchanged.

### Test-First Evidence And Trial Accounting

Meaningful RED before each behavior correction, with exact commands below:

| Exact command | RED |
| --- | --- |
| `devenv shell -- cargo test --locked --lib remote::poll::tests::review_restart_auto_gate_` | **0 passed/4 failed/604 filtered**, transport error instead of each gate result |
| `devenv shell -- cargo test --locked --lib remote::poll::tests::review_terminal_replay_` | **0 passed/1 failed/607 filtered**, removed configuration breaks replay |
| `devenv shell -- cargo test --locked --lib remote::poll::tests::review_original_report_` | **0 passed/1 failed/607 filtered**, actual second poll makes original targets empty after existing public boundary dispatch wiring |
| `devenv shell -- cargo test --locked --lib remote::state::persistence_tests::review_disposition_` | **1 passed/3 failed/604 filtered**, silent pair recreation twice and modern-time fallback; genuine-old control passes |
| `devenv shell -- cargo test --locked --lib remote::poll::tests::review_original_fenced_stop_` | Additional R3 **0 passed/1 failed/612 filtered**, original stop incorrectly Complete after foreign fence/later index repair |

Initial R3 missing-callback-dispatch trial is recorded separately below, not claimed
as report-loss RED. One R4 post-fix assertion trial needed direct reads of preserved
time columns instead of a helper querying the correctly missing table; fixed test
only. First broad correction gate passed 1,689 tests, then the additional actual
fenced-stop RED justified the final corrected-tree gate. No timeout, Clippy failure,
production-fix trial failure or redundant unchanged-tree broad rerun in this fix.

**15 new library tests** (10 poll, 5 state) in this correction. Actual local Git,
two services, test-installed tracking and real cache transitions; no fake transport,
sleeps/global config/developer keys. Existing public, non-cfg(test) FailurePoint
boundary callback dispatch is outside lease/registry/transport scopes. No new
dependency, DTO/schema field, phase, safe point, transport adapter or frontend code.

### Final Sequential Devenv Evidence

| Exact command | Result |
| --- | --- |
| `devenv shell -- cargo test --locked --lib remote::poll::tests::review_` | **10 passed**, 603 filtered |
| `devenv shell -- cargo test --locked --lib remote::state::persistence_tests::review_disposition_` | **5 passed**, 607 filtered (state unchanged after this focused run; all 5 pass in final library/full gate) |
| `devenv shell -- cargo test --locked --lib remote::poll` | **76 passed**, 537 filtered |
| `devenv shell -- cargo test --locked --lib remote::state` | **59 passed**, 553 filtered (state unchanged after focused run) |
| `devenv shell -- cargo test --locked --lib remote::reservation` | **40 passed**, 572 filtered (reservation unchanged after focused run) |
| `devenv shell -- cargo check --all-features --locked` | PASS |
| `devenv shell -- cargo fmt --check` | PASS |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | PASS |
| `devenv shell -- cargo test --all-features --locked` | **1,690 passed, 0 failed, 1 existing ignored** |
| `devenv shell -- cargo test --locked --lib` | Final standalone **613 passed**, 0 failed |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | PASS, exit 0, no window |

Final check/fmt/Clippy/full gate ran in the mandated order; log
`tool_139039d7b001Ok7TtKnL72SBZD`: **613 library**, **779 ordinary integration**,
**289 unchanged real-SSH** (41 comment, 25 merge, 15 observation, 63 synchronization,
31 fixture, 114 transport), **9 doctests**. Final standalone/CLI/preservation log
`tool_1392a241c001mYnB47hZGCxz5J`. Existing thousand-item characterization ignored.
No Rust edit after the final gate; only canonical bookkeeping follows.

### Local Delivery And Review Status

Changed implementation: poll.rs, reservation.rs, state.rs and two associated poll/
state test files. Existing reservation tests, observe_publication_remote body,
shared sync/refs/discovery/read/frontend helpers, dependencies/lockfiles, integration/
SSH tests, API audit and approved contracts have empty diffs against fix base.
Only those five Rust files, ledger, open ticket and two new Node-ULID comments are
staged for the authorized local correction commit. Status/source diff/recent log
inspected; git diff --check, preservation diff and scripts/show-ticket schema/render
checks precede commit. No subagents, Task 7, native dispatch, push, PR, merge,
closure or worktree removal. Canonical re-review comment
`01M4WJP4VF6W2FPCZ7ZHM9229A` records this checkpoint.

Return to spec reviewer `ses_ec74e7b1cffe52NiF8TdyiLNkR`; do not start quality until
spec approval. Ticket remains **open**. Task 2 direct-SQL poll-kind immutability
minor remains **deferred to final Cycle review**; whole-poll SSH/native exit
evidence remains pending, not claimed by these local-stage tests. Prior Task 5
approvals, D1–D4 and owner exceptions remain unchanged.

## Task 6 Spec Corrections: Investigation And RED — 2026-10-14T04:48:30Z

Spec reviewer `ses_ec74e7b1cffe52NiF8TdyiLNkR` **BLOCKED four findings** before
quality review. Verified clean fix base `72a6260853fe5f9f49b0342a602f999f990a01bc`
in the assigned worktree/branch; complete Task 6 review base remains
`ff43d2c3873e4c9cbd9f9f579c5cdd0ef4c513a7`, including a70a2ab. Fix only, inline,
no subagents; prior preflight/Task 0 trusted. Controller confirms necessary minimal
reservation transaction helper edits remain authorized implementation detail.
No approved contract change, Task 7 or outward/lifecycle action.

Source verifies all four: automatic gate is conditional on no replay row; current
plan lookup precedes operation lookup; original report is read only after released
reservation/refresh; disposition objects and time-column pair migrate separately.
Tests added before behavior corrections. Sequential exact RED evidence:

| Finding / exact command | RED result |
| --- | --- |
| R1 `devenv shell -- cargo test --locked --lib remote::poll::tests::review_restart_auto_gate_` | **0 passed/4 failed/604 filtered**, transport error instead of each NotEligible; active cases fail first, interrupted cases execute at GREEN |
| R2 `devenv shell -- cargo test --locked --lib remote::poll::tests::review_terminal_replay_` | **0 passed/1 failed/607 filtered**, removed publication selection produces transport error instead of original report; remaining configuration variants execute at GREEN |
| R4 `devenv shell -- cargo test --locked --lib remote::state::persistence_tests::review_disposition_` | **1 passed/3 failed/604 filtered**, lost pair silently recreated twice; missing new-row completion hidden by updated_at fallback; genuine all-absent/no-op control already passes |
| R3 `devenv shell -- cargo test --locked --lib remote::poll::tests::review_original_report_` | **0 passed/1 failed/607 filtered**, initial fixture boundary not reached; not yet a reproduced report-loss RED |

R1 asserts zero operation/policy writes, unchanged epoch/phase/outcome/snapshot for
all four policies on active and interrupted automatic restart. R2 covers removed
selection/config file and changed remote/primary, restart false/true, exact original
report, no attempt/outcome writes and invocation mismatch. R4 checks no schema
version change/recreation, retained data, new-row NULL metadata refusal and genuine
old rows unchanged with only allowed legacy-time reconstruction, unknown deadline.
No fake transport: real local repositories, installed tracking refs, panic provider.

R3 uses the existing public FailurePoint::PollAfterCompletion and public local
boundary callback to run a second real local post-fetch poll after release but
before first refresh/result, covering success and injected refresh failure. That
point currently calls check_failure directly, so only test plumbing must first
route it through the already-existing boundary dispatcher. This leaves the racy
report logic unchanged; record actual lost-report RED before its correction.
Canonical RED/review checkpoint `01M4WBV4WAGBZ4GEWTE583GF8X` precedes fixes.
Quality not started. Ticket remains open; Task 2 SQL-kind minor deferred final.

R3 actual race RED after only existing boundary dispatch wiring:
`devenv shell -- cargo test --locked --lib remote::poll::tests::review_original_report_`
exits **101, 0 passed/1 failed/607 filtered**. Second local poll really reserves,
commits a new batch/disposition set and refreshes after first completion. First
original response incorrectly has **0 targets**, expected its FastForwarded row.
This is the meaningful report-loss RED before transaction/report capture changes.

### Focused Corrections And GREEN

R1 gates every nonterminal automatic restart before re-acquisition; terminal replay
alone bypasses policy. Public cases prove active/interrupted × disabled, paused,
recovery-suspended and not-due, zero policy/operation writes and unchanged owner.
Explicit restart of a manual-priority ID still bypasses automatic policy and retains
backoff/deadline on its actual preflight transport failure.

R2 resolves the registered root/operation record first, validates stored action/
kind/invocation identity with its immutable target, and returns terminal reports
before any live config lookup. New attempts keep their prior missing-config path;
eligible restart resolves/rechecks its live target before configuration writes.
Removed publication/config file and changed remote/primary all replay identically
for restart false/true; different invocation or observe kind remains OperationMismatch.

R3 adds generic capture callbacks to the same shared safe-point, batch and finish
transaction implementations. Existing public observe/sync callers use identity
callbacks; no ownership/phase/policy algorithm duplicated or rewritten. The original
executor clones its report (metadata, times, disposition shadow, pending state)
before the transaction releases ownership. The shadow is updated only after each
durable row write, keeping failed targets NotReached. Success/failure refresh
wrappers reuse that exact captured outcome; only an older replay reads empty targets.
Stop reports capture before acknowledgment commits too, and stay IndexPending even
when a later poll repairs the index before the first response. Configuration-fenced
readback verifies original scope/repository/generation/epoch rather than adopting
a restarted executor. AfterCompletion callback runs only outside all guards.

R4 treats table+owner trigger+two replay columns as one all-present/all-absent
envelope; every one of **14 recognizable partial bitmasks** refuses without schema
or row rewriting. A genuine pre-envelope migration freezes only the existing
legacy updated_at completion fallback once, preserving original row fields; the
historical per-ID deadline is unknown and remains NULL. Modern completion reads
have no coalesce fallback; startup rejects lost completed/cancelled/failed metadata
and interrupted latest-report metadata. Update-kind configuration fencing writes
its actual completion/deadline in the same fence transaction, so this stricter
audit does not require inferred modern times. Observe/sync metadata stays unchanged.

One R4 fixture trial after the correction was **3 passed/1 failed** because its
post-refusal data-preservation assertion called a helper that queries the correctly
still-missing table. Corrected assertion to read the retained time columns directly;
no production relaxation. All other correction trials compiled and executed without
fixture failures. Original race's second poll is manual priority, respecting due
policy after the first completes; its post-fetch data comes from actual local Git.

Sequential focused evidence on the corrected tree, before final broad gates:

| Exact command | GREEN |
| --- | --- |
| `devenv shell -- cargo test --locked --lib remote::poll::tests::review_` | **9 passed**, 603 filtered |
| `devenv shell -- cargo test --locked --lib remote::state::persistence_tests::review_disposition_` | **5 passed**, 607 filtered |
| `devenv shell -- cargo test --locked --lib remote::poll` | **75 passed**, 537 filtered |
| `devenv shell -- cargo test --locked --lib remote::state` | **59 passed**, 553 filtered |
| `devenv shell -- cargo test --locked --lib remote::reservation` | **40 passed**, 572 filtered |

Four findings technically corrected, **not independently approved**. Spec re-review
required before quality starts. No scope amendment, subagents or Task 7 work.

### Additional R3 Fenced-Stop RED Before Final Handoff

The first corrective broad gate passed all four required checks and **1,689 tests**
(log `tool_138d3489c001No35S3ChQQAHpQ`), with **612** in the standalone library run.
No failure/timeout in that gate. Before committing, self-review tests a foreign
configuration-generation fence followed by a later explicit poll/index repair
before the first caller observes its stop. The original targets are preserved,
but reading the now-cleared root flag incorrectly returns Complete for that
original interrupted handoff. This is part of R3's stop-release race, not new scope.

Meaningful RED before the change:
`devenv shell -- cargo test --locked --lib remote::poll::tests::review_original_fenced_stop_`
exits **101, 0 passed/1 failed/612 filtered**, at "original fenced stop must retain
its own index handoff state". Real two-service post-fetch polls, actual cache
configuration fence, primary FF retained and context NotReached, only later poll
scans. Retain the original executor's committed-local-change boolean beside its
in-memory target shadow, so stopped results remain IndexPending despite later
repair. No new durable field/schema or scan; successful refresh/replay semantics
unchanged. A new final gate is justified by this actual regression correction.

## Task 6 Delivered Locally For Review — 2026-10-14T04:31:06Z

**Complete Task 6 review base:** `ff43d2c3873e4c9cbd9f9f579c5cdd0ef4c513a7`.
Include historical docs checkpoint `a70a2abcebb4c84694629bb7bd79e048ea1c83c1`
and the feature/bookkeeping commit whose exact HEAD is returned after commit.
The controller clarification resolved the entry interface issue within existing
Task 6 authorization; **no owner amendment or approved-contract change**.
Prior Task 5 spec/quality approvals and Task 0/preflight evidence remain trusted.

### Delivered Interfaces And Ordered Orchestration

- Public `poll_publication_remote`, request/result/outcome/report/error types,
  PollCompletion/PollDisposition/PollTargetReport and public re-exports. Request
  has only root/op ID/invocation/approval/restart and no Debug. Reports use
  accessors and validated IDs/refs/OIDs/times/fixed categories; errors render fixed
  text without inner Debug. The outcome accessor retains the existing fixed
  operation category for before-batch cancellation and older terminal replay.
- Terminal replay precedes eligibility/endpoints/authentication; interrupted
  replay requires explicit restart, observe/update identity stays distinct.
  New automatic attempts gate before endpoint configuration/reservation; explicit
  bypass preserves policy flags. Missing remote records configuration-required
  without transport. D3 and pending-local gates precede transport; automatic Busy
  and manual PollYielding use the delivered arbitration and D4 remains unchanged.
- Real fetch uses Task 3's combined frozen/automatic authenticated scope and exact
  parsed fetch set. Caller progress probes read flags/generation/epoch from one
  bounded read-only query, throttled to 250 ms; no lease, migration or write.
  Captured stop reason is sticky and wins over backend GenericError classification.
  Everything after successful fetch is one crate-private `poll_after_fetch` entry.
- Shared batch transaction body extracted once into `commit_observation_batch_with`.
  Existing observation/synchronization helper/signature forwards a no-op callback;
  their transaction and acknowledgment algorithm is unchanged. Poll's initial
  ordered NotReached rows commit after the advertisement and before the same
  AfterBatchCommit acknowledgment. Callback faults roll back both and checkpoint.
- Targets process primary, existing local contexts by ref, new remote contexts by
  ref, then redacted malformed entries. Remote-only history does not promote a
  new remote context into the local group. Each target takes BeforeLocalMutation,
  a single short common lease, frozen plan/config/endpoint/key/ref re-observation,
  classification/guarded FF or Task 5 materialization, durable row/refresh flag,
  release. Registered inaccessible contexts classify normally; owned incomplete
  intents still use Task 5 reconciliation. Task 4/5 wrappers retain their behavior
  with poll-local unlocked bodies enabling that same lease acquisition.
- Stop/fence after batch returns Interrupted/Cancelled reports with processed rows
  retained and remaining NotReached; changed targets durably require refresh and
  no discovery runs on the stop path. Final configuration fence also runs before
  discovery. Success sets refresh-required before finish, writes policy/times once,
  releases reservation, then ordinary index refresh outside lease under a fresh
  internal ID. Index failure preserves the report/flag; ordinary local refresh
  recovery resumes that separate journal ID without transport. Poll record never
  gains synchronization index_pending authority.

### Durable Metadata, Audit And Privacy

One latest `remote_poll_dispositions` set per repository, with exact-SQL checked
table/owner trigger installed all-or-nothing, repository and operation FKs, startup
row/identity/OID/time/schema audits and exact sorted inventory entry. Only the
approved target metadata columns are stored. Initial NotReached rows are replaced
per processed target; a later batch replaces the set. Latest-ID replay uses rows;
older terminal replay returns an empty target list and original completion/outcome.

Two nullable **metadata-only** operation columns, poll_completed_at and
poll_next_eligible_at, retain each attempt's completion/deadline. They are necessary
because latest policy timestamps/interval can change after an older ID completes;
deriving replay from current policy would rewrite its outcome. They migrate as a
pair in the same transaction; partial migration rolls back the new table too.
Their stop/finish writes are update-kind-only and atomic with existing phase writes;
old observe/sync rows gain NULLs, not inferred evidence. Startup validates times
and rejects non-update ownership of those fields. No new phase or safe point.

RemoteSnapshot exposes poll_dispositions/primary_disposition plus existing times;
RemoteContextSnapshot exposes latest_disposition, including projected local-only
HistoryUnknown contexts. State enum/versioned DTO schemas stay unchanged.
Dispositions use the existing boxed snapshot policy allocation to avoid public
result-size growth. DB/live WAL/VACUUM backup and report checks exclude canonical
body, malformed raw ref and endpoint privacy probes. Fixed guidance only.

Observation.rs changes are **three visibility widenings only**; the public
observe_publication_remote body is byte-for-byte unchanged. Sync/refs/discovery/
read/frontend/helper bodies and dependencies/lockfiles/approved documents have
empty diffs against review base. API audit poll-once/R09/R13/R16 now names the
operation/results and separates downstream adapters, drafts and scheduling.

### Verification, Counts And Honest Trial Record

**27 new library tests**: 22 poll, 4 state, 1 reservation. Real local Git/ODB
fixtures install tracking refs and test the post-fetch library stage, not a fake
transport. Public non-cfg(test) FailurePoint boundaries plus the hidden public
poll-boundary callback support deterministic actual external local/tracking/ref/
endpoint changes outside lease/registry/transport scopes and harness=false reuse.
No sleeps, global Git mutation or developer keys. Existing SSH suites unchanged.

Original recorded RED: poll **44 passed/5 failed/529 filtered**, state **50 passed/
2 failed/526 filtered** before behavior. Incremental RED and fixture trials are
retained below, including final endpoint fence, local-only snapshot projection,
wrong-category/target-order corrections and actual refresh-journal resumption.
First full gate reached inventory **77 passed/1 failed**, not a full pass. One
subsequent gate stopped at Clippy's collapsible local-group predicate before full
tests; collapsed it without behavior change. No timeout or fake passing gate.

Final sequential Devenv evidence:

| Exact command | Final result |
| --- | --- |
| `devenv shell -- cargo test --locked --lib remote::poll` | **66 passed**, 532 filtered |
| `devenv shell -- cargo test --locked --lib remote::state` | **54 passed**, 544 filtered |
| `devenv shell -- cargo test --locked --lib remote::reservation` | **40 passed**, 556 filtered (before final poll-only fixes; final full gate includes all 40) |
| `devenv shell -- cargo test --locked --lib` | **598 passed**, 0 failed |
| `devenv shell -- cargo check --all-features --locked` | PASS |
| `devenv shell -- cargo fmt --check` | PASS |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | PASS |
| `devenv shell -- cargo test --all-features --locked` | **1,675 passed, 0 failed, 1 existing ignored** |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | PASS, exit 0, no window |

Focused/library log `tool_138890967001rT9hVw0cMRVbWb`; final full sequential gate
log `tool_1388f80350017D54xCWWMkT8Kp`: **598 library**, **779 ordinary integration**,
**289 unchanged real-SSH** (41 comment, 25 merge, 15 observation, 63 synchronization,
31 fixture, 114 transport), **9 compile-fail doctests**. Enablement inventory **78
passed** in full gate. Existing thousand-item characterization remains ignored.
No Rust change after that final gate; only canonical bookkeeping follows.

### Review Handoff And Remaining Gates

Task 6 implementation is ticked and ready for controller's **independent spec,
then quality review** of the full range. No subagent dispatched. **Task 2 minor
direct-SQL poll-kind immutability regression remains explicitly deferred to final
Cycle review** (`ses_ec9e9909cffeS4kt0Z0ENmyjss`). Whole-poll real-SSH/native exit
evidence is still pending; Task 6 library results do not claim it. No Task 7 work,
native dispatch, desktop interaction, push, PR, merge, closure or worktree removal.
Ticket remains **open**. D1–D4, accepted⇒active-only and the sole Task 5 final
canonical validator same-lease exception remain unchanged.

Canonical GREEN/review-ready comment `01M4WAV9ECDNZ8NFSP7CGSXK85`. Status/diff/log
and source/interface/contract diffs inspected before staging only intended files.
Whitespace and scripts/show-ticket canonical schema/rendering checked before the
authorized local `feat: add one-shot remote poll` commit. Full range includes the
earlier docs-only issue/clarification checkpoint; exact head returned after commit.

## Task 6 Clarification, Resume And RED — 2026-10-14T02:40:36Z

### Incremental Validation And Snapshot Regression RED

First sequential required gate passed check/fmt/Clippy, then full tests stopped
at enablement inventory **77 passed/1 failed** because the new table was appended
out of alphabetic order (actual SQL inventory is sorted). Library **596** and
earlier untouched SSH suites had passed; this is not a complete full-suite pass.
Log `tool_1386d29760019wg6haq5fQMBL3`. Corrected expected inventory order only.
Self-review then found two orchestration category/order bugs. Before production
correction, `devenv shell -- cargo test --locked --lib remote::poll::tests::task6_`
is **20 passed/2 failed/576 filtered**: a registered worktree on a wrong branch
was BlockedByLocalState instead of Inaccessible; remote-only advertisement history
was put before an existing local context. Initial wrong-head fixture attempted to
reuse main (backend refuses because root checks it out); corrected to a real
unoccupied foreign branch, then both failures are behavioral RED. Only remote
contexts actually locally present/cached active enter the existing group; without
an owned materialization intent, a registered local context classifies normally.
Owned incomplete materialization still uses Task 5's conservative reconciliation.

Additional deterministic local-race tests use public, non-cfg(test) FailurePoint
boundaries and a hidden public callback seam usable by harness=false SSH tests.
Callbacks run outside lease/registry/transport scopes, then production reobserves.
Before wiring the boundary, `devenv shell -- cargo test --locked --lib
remote::poll::tests::task6_` is **18 passed/2 failed/576 filtered**: target boundary
not reached, final endpoint case completes instead of stopping. Wiring will then
exercise actual external local-ref/tracking/endpoint changes, not fabricated
transport. The boundary does not permit a caller-supplied fetch result.
With the public boundaries wired, the identical command is **19 passed/1 failed/
576 filtered** before the final fence correction: actual local/tracking/endpoint
changes before context processing now prove Interrupted with primary retained;
an actual endpoint change after the last target still incorrectly completes and
scans. Added final configuration re-observation only after this behavioral RED.

Initial five new poll cases GREEN **49 passed**. Expanded stop/order/replay/index
matrix trial **53 passed/1 failed**: later refresh incorrectly used a new ID after
an injected refresh had created a failed local journal row. Existing index recovery
requires resuming that ordinary refresh ID; corrected fixture to retrieve/resume
it, proving it differs from poll ID. No production recovery-policy change.
Further gate matrix trial **58 passed/1 failed**: assertion used nonexistent local
`phase` column instead of `state='created'`; corrected fixture only.
State trial **51 passed/1 failed**: Cycle 04 migration fixture replaced operation
records but retained the new empty disposition table with its FK. Drop that table
with the other modern evidence tables to represent genuine legacy schema.
Clippy trial found seven needless borrows after poll-local lease extraction, one
cloned fixture slice, and snapshot growth affecting RefreshOutcome. Corrected
borrows/slice and grouped dispositions into the existing private boxed snapshot
policy allocation, retaining public shapes; no suppression of those diagnostics.

Self-review finds local-only HistoryUnknown contexts are appended after existing
dispositions are attached. Meaningful regression before correction:
`devenv shell -- cargo test --locked --lib remote::state::persistence_tests::task6_local_history_unknown_context_also_exposes_its_latest_disposition`
exits **101, 0 passed/1 failed/589 filtered**, expected Some(HistoryUnknown), got
None. Initial trial used an intentionally synthetic cached path through the full
local-cache validator (IndexUnavailable), then a function-pointer coercion failed
to compile; corrected to the existing state test's direct reconstruction closure.
Only that final assertion run is behavioral RED. Correction follows this record.

Controller clarifies the file list is a planning map, not a prohibition on
necessary shared-seam edits. Atomic batch plus initial rows was already approved:
a minimal reservation.rs/reservation_tests.rs transaction extraction is within
Task 6 authority. The entry issue below is **resolved as an implementation detail,
not an owner amendment**. Approved contracts unchanged. Preserve the existing
observation and synchronization paths exactly using one internal transaction
implementation, no duplicated ownership/acknowledgment logic. Review base remains
`ff43d2c3873e4c9cbd9f9f579c5cdd0ef4c513a7`, including docs checkpoint
`a70a2abcebb4c84694629bb7bd79e048ea1c83c1`. Clean resume HEAD verified.

Tests first, behavior-free signatures/todo only before RED. Initial poll compile
trial had nine wrong module qualification errors for existing transport/key types;
corrected imports, then executed meaningful runtime RED before behavior:

| Exact command | Result |
| --- | --- |
| `devenv shell -- cargo test --locked --lib remote::poll` | **44 passed, 5 failed, 529 filtered**, exit 101; missing atomic callback/public gates/post-fetch behavior |
| `devenv shell -- cargo test --locked --lib remote::state` | **50 passed, 2 failed, 526 filtered**, exit 101; disposition columns absent; dependent partial-loss fixture cannot yet drop absent table |

Atomic test writes in the callback after verifying the advertisement exists,
returns an error, and requires rollback of batch, callback write and checkpoint;
also rejects another service's ownership token before invoking its callback.
Other tests require primary ref/index/files advancement, exact terminal replay
without credentials, four automatic gates with no reservation/outcome writes,
configuration fencing, and atomic-batch cancellation returning NotReached without
discovery. State tests require exact metadata-only columns and partial-loss refusal.
Library fixtures install real local tracking refs and drive one post-fetch seam;
no fake transport. Public types carry no endpoint/key/passphrase. One temporary
private-interface visibility warning will be resolved with visibility-only widening.
Canonical RED/resume comment `01M4W4GZ1NQT9JYSJP1YM2ZEJQ` precedes production.
Task 6 in progress, ticket open; prior Task 5 approvals and Task 2 deferred minor
retained; no subagents or Task 7 work.

## Task 6 Entry And Atomic Batch Interface Blocker — 2026-10-14T02:33:54Z

Fresh implementer assigned **Task 6 only**, inline, no subagents, in the exact
ticket worktree/branch. Verified clean HEAD and task base
`ff43d2c3873e4c9cbd9f9f579c5cdd0ef4c513a7`, registered worktree identity and
ancestry of the recorded main `d1ca88772c5768d55aadb2ca4dcac1c69dab0d17`.
Task 0 and prior controller preflight trusted; no fetch/rebase or characterization
replay. Inspected current AGENTS, approved Task 6/design, ticket/ledger and the
delivered observation, reservation, polling, state and poll-fetch interfaces.

### Prior Task 5 Reviews Now Approved

Controller/user conveys completed independent reviews after the valid-owned
unfinished-context dirty/ahead reconciliation correction:

- Spec `ses_ec7f49995ffeVTGZBqRN4rzEoX`: **passed** corrected Task 5.
- Quality `ses_ec7c4bbc3ffensuQh7z1f1Gqul`: **APPROVE, no findings**.
- The approvals cover Task 5 through this Task 6 base, cumulative range from
  `ab07d0d3096a0e6129add1ba32a2f977a131b80a`; historical P2 and RED/GREEN evidence
  above are retained, with pending re-review status superseded by this report.
- Accepted tree ⇒ active context only remains approved; rejected trees may still
  be valid locally. Only Task 5 final unchanged canonical validator may collect
  sources under its same lease. D1–D4 remain unchanged.
- **Task 2 minor direct-SQL poll-kind immutability regression remains deferred
  to final Cycle review**, from `ses_ec9e9909cffeS4kt0Z0ENmyjss`.

### Interface/Ownership Conflict Before Production

Task 6 requires the caller's ordered initial `not_reached` disposition set and
the complete advertisement to commit in **one transaction**, while consuming
the delivered `commit_observation_batch` seam. Its explicit file list does not
include reservation.rs (the inventory-test exception is unrelated).

Source evidence on the exact task base:

- Approved implementation plan Task 6, lines 411–416/431–435/461–463: consume
  `commit_observation_batch`, commit batch plus initial disposition rows together,
  and process primary, existing contexts, then new contexts in ref order.
- Approved design lines 366–372/389–391: one latest set initialized at the batch
  boundary, before per-target processing; replay reports that stored set.
- `src/repository/remote/reservation.rs:471–528`: the delivered helper accepts
  only service/root/owner/plan/observations/time. It opens its own
  `state::with_transaction`, verifies ownership and stop requests, persists the
  advertisement, and acknowledges `AfterBatchCommit`. There is no caller-supplied
  disposition payload or in-transaction extension hook.
- `src/repository/remote/state.rs:5002–5035`: `with_transaction` opens a separate
  connection and commits before returning. Wrapping the helper in a second
  `with_transaction` cannot join its transaction. Writing disposition rows after
  it returns leaves a crash-visible batch without its required initial rows;
  writing them before it returns publishes rows without the batch.
- `state.rs:1968–2024`: the shared advertisement writer receives no owning poll
  ID or caller-observed ordered local target set. Inferring those from unrelated
  cache rows or installing an implicit trigger side channel would replace the
  explicit orchestration boundary rather than consume the delivered interface.
- Production observation and synchronization callers use the same helper
  (`observation.rs:268`, `sync.rs:6283`). Their behavior is frozen; the observation
  operation must remain exactly unchanged.

**Stopped before Rust/test edits** rather than silently splitting the transaction,
duplicating the ownership/acknowledgment algorithm or extending a file outside the
Task 6 allowlist. This is an interface/ownership blocker, not evidence that atomic
polling is infeasible.

### Narrow Owner Ruling Requested — Not Yet Approved

Authorize reservation.rs (and focused reservation tests if needed) for a narrow
transaction-composition extension: retain the existing helper/signature and its
observation/synchronization behavior, and provide a full-poll variant or internal
transaction seam that also writes the caller's ordered initial disposition rows
before the same transaction acknowledges/commits `AfterBatchCommit`. Preserve
ownership, cancellation, yield and generation checks. No new phase, safe point,
transport seam, table beyond the approved disposition table, or scan permission.

This proposal is **not an accepted amendment**. Approved Cycle/design/plan, shared
helper bodies and production code remain unchanged. Task 6 is blocked/unticked,
not review-ready; ticket open. No Task 7 work or outward/lifecycle action.

### Verification And Handoff

No Rust command executed; **0 tests run**, no compile RED, behavioral RED, GREEN
or Rust gate claimed. Planned RED commands remain unrun:

- `devenv shell -- cargo test --locked --lib remote::poll`
- `devenv shell -- cargo test --locked --lib remote::state`

This is a docs-only blocker checkpoint. Canonical Node-generated comment
`01M4W44NN8N2BRMZBC1H4AG4HN` records entry, prior approvals and the requested
ruling. Status/full diff/recent log inspected. Exact checks, each exit **0**:

- `git diff --check`: PASS.
- `git diff --exit-code -- src tests Cargo.toml Cargo.lock devenv.lock`: PASS,
  source/test/dependency tree unchanged.
- `scripts/show-ticket 01K7F6H9J2N4Q6S8V0X2Z4B6DG`: PASS, canonical ticket and
  new comment rendered with open status and correct Node ULID/timestamp.

Only ledger, open ticket and new comment are staged for the authorized local
bookkeeping commit. Checkpoint range starts at the exact Task 6 base above;
new commit head returned after commit. Rust gates and implementation RED/GREEN
remain required after the ruling authorizes implementation.

## Task 5 Spec P2 Reconciliation Correction — 2026-10-14T01:40:07Z

### Correction And Final GREEN — 2026-10-14T02:19:44Z

Removed only the clean/original-tip requirement from the fully materialized
owned-context row and its now-unused clean observation. A direct context branch,
deterministic path, exact Task 4 ownership and regular index still prove the
materialization. Unfinished intent then enters the **existing step 5** unchanged
canonical validator under the same single lease; only successful validation
marks complete, retaining original owner and creation OID. Present delegates
normal cleanliness/ancestry classification to the existing classifier. No Git
mutation is performed for this reconciliation row.

The conservative **branch-only** row is byte-for-byte unchanged: branch_created,
exact original direct OID, no path/admin/registration. Partial, wrong identity,
symbolic branch and missing-index states still refuse. No approved-document or
schema change; no exception broadened. Parent/shared helper bodies, Task 4's
ownership wrapper and classification are unchanged.

Four new regression tests exercise **8 positive cases** (document/ticket ×
Current, untracked Dirty, valid canonical-edit Dirty, actual descendant-commit
LocalAhead) and **3 refusal controls** (malformed canonical source, foreign
standalone replacement, missing index). Every positive proves full root and
target snapshots/config unchanged, original owner/OID retained with complete,
one registration, normal classification and repeat Present with no effects.
Refusal controls prove root plus actual replacement-target snapshots and
unfinished intent remain unchanged across repeated calls. The malformed source
now reaches final canonical validation rather than being refused solely for
dirty status. Earlier failed-checkout, ambiguous-intent and strict branch-only
tests remain green. No fake transport, sleep, developer key/global config change
or new testing seam. No setup or production trial failure after recorded RED.

Final sequential Devenv evidence on the unchanged-after-verification Rust tree:

| Exact command | Result |
| --- | --- |
| `devenv shell -- cargo test --locked --lib remote::poll::tests::poll_materialization_owned_unfinished_` | RED **1 passed/3 failed/567 filtered**, then GREEN **4 passed/567 filtered** |
| `devenv shell -- cargo test --locked --lib remote::poll` | **44 passed**, 527 filtered |
| `devenv shell -- cargo check --all-features --locked` | PASS |
| `devenv shell -- cargo fmt --check` | PASS |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | PASS |
| `devenv shell -- cargo test --all-features --locked` | **1,648 passed, 0 failed, 1 existing ignored** |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | PASS, exit 0 |

Full log `tool_138173849001kHezuxNuakarWT`: **571 library**, **779 ordinary
integration**, **289 unchanged real-SSH** (41 comment, 25 merge, 15 observation,
63 synchronization, 31 fixture, 114 transport), **9 doctests**. One full gate
for this fix, started with the longer bound justified by prior Task 5 timing;
no timeout/repeated broad gate here. The existing thousand-item characterization
remains ignored. Native matrix not run or claimed.

P2 **technically corrected, awaiting SPEC re-review** by
`ses_ec7f49995ffeVTGZBqRN4rzEoX`, before any quality review. Incremental fix base
`4dc35de84fab433a299c4de8c640ad5647ceb15c`; cumulative Task 5 review base
`ab07d0d3096a0e6129add1ba32a2f977a131b80a`; exact local head returned after
the new fix/bookkeeping commit. Canonical GREEN/re-review comment
`01M4W3AQQGAF0YJ28908ABW64Q`. Only poll.rs, poll_tests.rs, ledger, open ticket
and two new comments change. Status/diff/recent log inspected, git diff --check
and scripts/show-ticket canonical schema/rendering validated before staging only
intended files. Task 2 SQL kind-immutability minor deferred final, ticket open,
Task 6 unstarted; no subagents, push/PR/native dispatch/merge/closure/removal.

Controller conveys spec review `ses_ec7f49995ffeVTGZBqRN4rzEoX`: **BLOCKED P2**.
`poll.rs:739–743` demands clean worktree and original target OID for an unfinished
intent even after the worktree was fully created and is valid/owned. This narrows
the design's valid materialized row: retained untracked work, a valid canonical
edit or a new local commit should finish final validation/intent completion and
then classify normally (Dirty/LocalAhead/Current), with Git left unchanged.

Verified clean fix base `4dc35de84fab433a299c4de8c640ad5647ceb15c` in the assigned
worktree/branch. Complete Task 5 review base remains
`ab07d0d3096a0e6129add1ba32a2f977a131b80a`. Prior controller preflight and Task 0
gate trusted; no fetch/rebase. Correction only, no amendment, subagents or Task 6.
The owner exception already permits only final step 5's unchanged canonical
validator under the same lease, including completion of the interrupted step;
no other scan permission. D1–D4 and accepted⇒active-only policy unchanged.
Task 2 SQL kind-immutability minor remains deferred final. Quality not started;
spec re-review must precede it.

### Meaningful Regression RED Before Predicate Change

Added four real-Git test groups. Each first interrupts at public
PollAfterMaterializationWorktree, proves InjectedFailure at that boundary,
branch_created intent/original owner/OID, exact owned worktree with index, and
clean initial checkout. Positive groups then add an untracked file, append a
valid canonical body edit, or make an actual local descendant commit. The test
requires Present, full root/target Git/config snapshot preservation, immutable
original intent owner/OID with complete, normal Dirty/LocalAhead/Current
classification and an idempotent next call. Both document and ticket variants
will be exercised at GREEN; RED loops stop at their first failing document case.
Negative controls retain malformed canonical source, foreign standalone
substitution and missing index with unchanged artifacts/intent.

Before any production change:
`devenv shell -- cargo test --locked --lib remote::poll::tests::poll_materialization_owned_unfinished_`
exits **101: 1 passed, 3 failed, 567 filtered**. All three positive regressions
fail at Refused(MaterializationIncomplete), expected Present. Negative controls
pass against the old predicate; the clean Current control passes before the
untracked RED. No compile/setup failure in this regression run.
`git diff --exit-code` confirms production/shared helpers and approved documents
unchanged at RED. Only poll_tests.rs changed before this record. Canonical
Node-generated RED comment `01M4W126TJWXJJCPW4FMSJD99K` precedes correction.

## Task 5 Implementation And Review Handoff — 2026-10-14T01:30:16Z

Task 5 delivered locally, review base
**`ab07d0d3096a0e6129add1ba32a2f977a131b80a`**. Review the complete range from
that base to the local feature checkpoint HEAD returned after commit, including
the owner-approved exception reconciliation. Earlier blocked checkpoints are
historical, not the implementation range. Controller preflight and Task 0 gate
trusted, no duplicate fetch/rebase/characterization or subagents. Prior Task 4
spec/quality approvals and accepted⇒active-only amendment retained. D1–D4 unchanged.

### Delivered Behavior

- Additive `remote_poll_materializations` has only repository/kind/item ID,
  immutable owning poll ID/target OID and fixed step. Unique per repository/
  context, repository and owning-operation FKs. Four-object envelope (table,
  intent immutability, step transition and owner immutability triggers) is
  installed atomically and exact-SQL validated. The owner trigger survives loss
  of the table to distinguish partial loss from a genuine old registry.
  Startup audits IDs/OIDs/steps, update-poll ownership and FKs. Existing registries
  gain no inferred row; exact table inventory updated.
- `materialize_context` acquires the root/common lease **once**. It checks the
  path chain before configuration reads; reobserves frozen plan/endpoints/key
  and the fetched tracking OID; observes branch/path/admin state before loading
  intent ownership. It validates conformity before effects, durably records
  intended in a short cache transaction, creates the branch **without force**,
  records branch_created, uses exactly authoring's WorktreeAddOptions::reference,
  validates actual worktree ownership and unchanged canonical validator in step
  5 only, and records complete. No branch configuration, cleanup or transport.
- Reconciliation returns typed `Present` for the caller to classify normally;
  it does not claim Current against a newer incoming OID. Unfinished valid
  worktree recovery finishes only step 5 with matching retained owner/target.
  Missing everything starts step 1 regardless of cached step, resetting only
  step to intended after Git/filesystem absence is observed; immutable context,
  owner and target are retained. Only recorded branch_created at the recorded
  **direct** branch OID with no path/admin/registration resumes worktree creation.
  An intended row plus branch alone is incomplete, including a real failed
  branch-step transaction. No matching intent means blocked; other partial
  evidence means incomplete and nothing removed. Later calls retain original
  intent ownership rather than adopting the caller's new operation ID.
- Task 4's stronger poll-local ownership proof is consumed unchanged: linked
  worktree, canonical owner commondir, exact admin/name/path and regular exact
  gitfile backlink. Poll-local reconciliation also rejects a symbolic branch
  replacement and absent/nonregular index. Shared repository, discovery,
  synchronization and refs helper bodies are untouched. repository.rs adds only
  four **public, non-cfg(test)** FailurePoint variants, used by the existing
  open_at_with_failure_point_for_testing seam.

### Test Coverage And Honest Trial History

Eleven new poll tests and four new state tests add **15 library tests**. Actual
Git success proves both document and ticket branch/HEAD at fetched commit,
one registration at deterministic path, clean index/worktree, no upstream,
complete intent, shared validation success and active discovery. Second/reopen/
later-ID calls are snapshot-identical and preserve original intent owner.
Two services share one data directory and start at a barrier: exactly one
materializes; the other is Present or the unchanged lease's retryable Busy.
After both calls return, a retry is Present with one worktree/intent.

Ten initial obstruction cases assert full root/target/index/HEAD/status/ref/admin
snapshot and config preservation: same/different/elsewhere-checked-out branch,
file/empty/nonempty/symlink deterministic path, registration elsewhere, redirected
Manyhands/base. Three successful-creation interruption boundaries prove actual
Git state and restart without repetition. The fourth uses Task 0's **real Linux
checkout failure**: regular `a` written, overlong symlink `z` fails; branch,
worktree .git, admin HEAD/gitdir/commondir and registration remain, index absent.
Reopen and repeated calls report MaterializationIncomplete with byte-identical
retained partial artifacts. No fault fabrication or cleanup in the service.

Seven ambiguity cases include intended+branch, moved branch, partial file/dir,
admin-only, another kind's intent and foreign standalone replacement; repeat
calls preserve every artifact and row. Tracking/config/endpoint changes are
ExternalChange before effects. Added all-absent cached-step RED and symbolic
replacement RED are recorded below before their poll-local corrections. Extra
hardening covers a real SQL branch-step failure, nonconforming tree, unrelated
operation, exact schema/privacy columns, SQL immutability, redacted DB/WAL/
VACUUM backup, corrupt startup rows/nonpoll ownership and registration cascade.

Trials, not successful gates:

- New fixture compile/runtime errors and corrected initial behavioral RED are
  recorded below. First production compile needed OperationId::parse (not a
  nonexistent FromStr implementation); no requirement change.
- Initial poll GREEN trial **34 passed/2 failed**: race test assumed queued
  lease rather than documented retryable Busy; wrong-kind fixture lacked its
  incoming tracking ref. Corrected assertions/setup, no production relaxation.
- Initial state trial **47 passed/1 failed**: old Cycle 04 migration fixture
  retained new materialization objects while replacing remote operations.
  Removed the new empty table in that legacy fixture to represent a genuine
  old registry; all other historical migration assertions retained.
- Corrupt-row fixture initially left SQLite FK checks on while altering the
  owning ID; explicitly disabled them for external corruption simulation.
  Corrected state focused run **50 passed**. No audit production weakening.
- First full command exceeded the tool's **600,000 ms** limit after all **567
  library** tests passed and during unchanged comment-publication SSH cases.
  Log `tool_137e533e7001J3UvUufSYQk3lq` is incomplete, not a passing full gate.
  Checked own command was gone without touching other worktrees' processes;
  reran the identical full command with a 3,600,000 ms bound. No Rust change
  between these two runs; the longer bound was justified by the timeout.

### Final Sequential Devenv Evidence

| Exact command | Result |
| --- | --- |
| `devenv shell -- cargo test --locked --lib remote::poll` | **40 passed**, 527 filtered; final symbolic-ref correction included |
| `devenv shell -- cargo test --locked --lib remote::state` | **50 passed**, 516 filtered; state unchanged afterwards except formatting |
| `devenv shell -- cargo test --locked --test repository_enablement` | **78 passed** |
| `devenv shell -- cargo check --all-features --locked` | PASS |
| `devenv shell -- cargo fmt --check` | PASS |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | PASS |
| `devenv shell -- cargo test --all-features --locked` | PASS, **1,644 passed, 0 failed, 1 existing ignored** |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | PASS, exit 0 |

Passing full log `tool_137eb5d63001IPUFMOPVHfgzIN`: **567 library**, **779
ordinary integration**, **289 real-SSH** (41 comment, 25 merge, 15 observation,
63 synchronization, 31 fixture, 114 transport), **9 compile-fail doctests**.
The existing thousand-item characterization remains ignored. No new transport
claim comes from library fixtures; existing SSH targets run unchanged. No native
matrix or desktop interaction claim. Cargo/devenv lockfiles and dependencies,
frontends and versioned schema files unchanged. No sleeps/developer keys/global
Git mutation, fake transport or cfg(test) production fault hook added.

### Review-Ready Checkpoint

Self-reviewed the actual source and approved-exception diff against Task 5.
Task 5 implementation ticked; independent **spec, then quality** review pending.
**Task 2 direct-SQL poll-kind immutability minor remains deferred to final Cycle
review**. Native platform evidence remains pending; Linux failed-checkout shape
does not prove Windows/macOS behavior. No other owner decision or amendment.

Canonical GREEN comment `01M4W0G54ZF0M98PXS90NJFEJH`. Intended files are six
Rust/test files, four approved/ledger documents, open ticket and three new
comments. Shared helper diffs are empty (no visibility widening needed because
existing parent-private helpers are accessible to descendants); repository.rs
diff is FailurePoint variants only. Status/full diff/recent log inspected;
git diff --check and scripts/show-ticket schema/rendering checked before staging
only these intended files. No Rust edit after final verification. Ticket remains
open, Task 6 not started, no push/PR/native dispatch/merge/closure/removal.

## Task 5 Owner Exception And Resume — 2026-10-14T00:26:14Z

### Meaningful RED Before Production — 2026-10-14T00:32:06Z

Additional Git-first regression RED before correction:
`devenv shell -- cargo test --locked --lib remote::poll::tests::poll_materialization_git_absence`
exits 101, **0 passed, 1 failed, 563 filtered**. No Git branch/path/registration
with cached branch_created incorrectly returned MaterializationIncomplete
instead of starting step 1. The design's all-absent row applies to every cached
step; the reset must retain immutable context/owner/target and is authorized
only after observing complete Git/filesystem absence under the lease.

Self-review ownership RED before correction:
`devenv shell -- cargo test --locked --lib remote::poll::tests::poll_materialization_completed_intent`
exits 101, **0 passed, 1 failed, 566 filtered**. An external replacement of the
completed intent's direct branch with a symbolic reference to another local
branch at the same OID incorrectly returned Present. Materialization must
observe the actual direct reference, not adopt its resolved destination.
The regression asserts full snapshot preservation and MaterializationIncomplete;
poll-local correction only, no Task 4/shared helper-body change.

Added seven real-local-Git materialization test groups, two schema groups and
the exact inventory entry. New API and public FailurePoint variants initially
compile-RED (27 errors, including two corrected discovery fixture signature
errors); added behavior-free types/signature with `todo!` only. First runtime
trial failed in fixture reservation because policy plan had not been configured.
Compared the working reservation fixture and configured the synthetic plan;
corrected runtime now reaches the missing materialization placeholder in all
seven groups. No production behavior or helper body changed before this record.

Sequential exact commands and results:

| Command | RED result |
| --- | --- |
| `devenv shell -- cargo test --locked --lib remote::poll` | **29 passed, 7 failed, 525 filtered**, exit 101, all seven at missing materialization behavior |
| `devenv shell -- cargo test --locked --lib remote::state` | **46 passed, 2 failed, 513 filtered**, exit 101; table columns absent; dependent loss-test trigger absent (not yet an exercised partial-loss regression) |
| `devenv shell -- cargo test --locked --test repository_enablement` | **77 passed, 1 failed**, exit 101; exact table inventory missing remote_poll_materializations |

Matrix loops stop at their first RED; GREEN must exercise remaining cases.
Tests cover document/ticket success, second/reopen/later-call ownership, two
services with a barrier, branch/path/registration/redirect preservation, all
four interruption boundaries including Task 0's actual Linux failed checkout,
ambiguous intended-branch/partial/admin/foreign-identity state, and configuration/
tracking/endpoint reobservation. Real repositories and ODB trees, no transport
claim/fake, sleep, developer key or global Git config change. Public failure
variants use the existing non-cfg(test) service seam. Canonical RED comment
`01M4VX5NM6TMJ35B0048FWBBN7` records this checkpoint before production.

Controller conveys owner's **"yes"** approving only Task 5 step 5's unchanged
`validate_context_worktree` canonical collection under the **same single lease**
before `complete`. Discovery/index-refresh scans, network calls and prompts remain
outside. Reconciled plan global constraint, design step 5 and Cycle clarity
before production. No other design change; accepted⇒active-only conformity
amendment and D1–D4 unchanged. Earlier blocker and independent confirmation
`ses_ec8580fb8ffeelwyx0FtUPwU2p` remain separate historical evidence.

Controller reports fresh fetch/rebase clean no-op: main
`d1ca88772c5768d55aadb2ca4dcac1c69dab0d17`, before/after ticket HEAD and new
**Task 5 review base** `ab07d0d3096a0e6129add1ba32a2f977a131b80a`.
Implementer confirmed exact HEAD, clean assigned branch/worktree and ancestry;
did not repeat network/rebase. Task 0 gate trusted. Prior Task 4 spec
`ses_ec8bd9b3effeS3OD0g7achFQNy` and quality `ses_ec892692effePXdmvv7I1Ys7ER`
approvals retained. Task 2 direct-SQL kind-immutability minor remains deferred
to final review.

Task 5 only resumes inline, no subagents. Meaningful RED precedes production;
all Rust commands use sequential Devenv. Review range includes the approved
exception plus implementation. Canonical Node-generated resume comment
`01M4VWTXC6JGWH7CXZQ5XPRHF8`. Ticket open, Task 5 unticked until delivery,
Task 6 unstarted; local commit and independent spec then quality handoff only.

## Task 5 Independent Blocker Confirmation — 2026-10-13T23:46:53Z

Controller conveys independent read-only review
`ses_ec8580fb8ffeelwyx0FtUPwU2p`: **CONFIRMED written Task 5 inconsistency**.
This is confirmation of the blocker, not approval of an exception. Verified
clean assigned worktree/branch at checkpoint base
`fe454477d16a86a338d75e8e9939205e56ff9612`; original Task 5 base remains
`8cb5d777758287b200e09255dc1fa7f254fc4da3`. No fetch/rebase or Rust command.

### Independent Source Evidence

- Approved implementation plan
  `2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-implementation.md`
  (document ID `01M4NM26R0Q7ZXR0TDSPQB6H59`), line 49: **any scan** excluded
  under the lease. Task 5 lines 352–355/392–395 require the existing validators,
  five ordered steps under one lease acquisition, and Git-first reconciliation.
- Approved design
  `2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-design.md`
  (document ID `01M4NM26Q2CZH0CGBT31QVZA0W`), lines 332–340: step 5 calls
  unchanged `validate_context_worktree` under that lease before `complete`.
- Approved Cycle
  `wave-02-cycle-08-one-shot-polling-and-materialization.md`
  (document ID `01M4NM26P51XW21SPJDF6RPYQ9`), line 182: **discovery scan**
  wording; lines 168–171 put index-only refresh outside the lease.
- RFC `docs/RFC/repository-index-persistence-and-refresh.md:146–153,162–174`:
  **full read-only/full scans** outside the lease, with refs, worktrees and
  canonical paths re-observed under it before local mutation/cache replacement.
  The narrower Cycle/RFC wording suggests a narrower intention but does not
  explicitly exempt Task 5 canonical validation from the plan's any-scan rule.
- Source `src/repository.rs:8354,7976–7987,8136–8144,8175–8215,8226–8263,8290–8312`:
  the unchanged validator unconditionally collects docs/tickets/comments,
  recursively enumerates directories and reads contents. `require_target = false`
  does not avoid collection. Traversal bounds do not create a policy exception.

### Narrow Owner Proposal — UNAPPROVED

Permit **only Task 5 materialization step 5** to perform the unchanged
`validate_context_worktree` canonical collection under the **same single lease
acquisition**, before recording `complete`. Discovery scans, index refresh,
network calls and prompts remain outside the lease. This proposal authorizes
no helper-body change, replacement validator or extra lease acquisition.

This is a proposal for the owner, **not an accepted exception**. Approved
Cycle/design/implementation plan are not amended. Accepted⇒active-only agreement,
stricter non-executable poll conformity, unchanged discovery and D1–D4 remain.
Task 2 direct-SQL poll-kind immutability regression remains explicitly deferred
to final Cycle review. Prior Task 4 approvals remain recorded and trusted.

Task 5 **blocked and unticked**, not review-ready; Task 6 not started; ticket
open. This authorized local docs-only checkpoint changes the ledger and canonical
Node-generated comment `01M4VTJW60QVE0460ZRYBWJQS2`. Whitespace, unchanged
Rust/tests/dependencies/approved documents and scripts/show-ticket rendering are
checked before staging intended files and committing. Exact checkpoint range
starts `fe454477d16a86a338d75e8e9939205e56ff9612`; head returned after commit.
No test execution or RED/GREEN claim. No subagents or outward/lifecycle action.

## Task 5 Pre-Implementation Scope Blocker — 2026-10-13T23:43:36Z

Fresh implementer assigned **Task 5 only**, inline with no subagents. Verified
clean assigned worktree and branch at the exact task base
`8cb5d777758287b200e09255dc1fa7f254fc4da3`. This assignment explicitly forbids
network and rebase, so the prior controller preflight remains trusted; no new
fetch/rebase was performed. Current AGENTS, approved Task 5/design, ledger,
ticket, relevant poll/state fixtures and existing repository helpers inspected.

### Prior Task 4 Reviews Now Approved

The controller reports approval after the directory-collapse ignored-collision,
foreign-worktree identity and docs/.md-selection corrections, through this base:

- Spec `ses_ec8bd9b3effeS3OD0g7achFQNy`: **approved** corrected Task 4.
- Quality `ses_ec892692effePXdmvv7I1Ys7ER`: **approved** corrected Task 4.
- These supersede the pending re-review status above; historical findings and
  RED/GREEN evidence remain intact. Ticket review checkboxes reflect the report.
- Owner amendment remains accepted tree ⇒ active context **only**; rejected
  trees may still be active in discovery. Non-executable poll conformity and
  unchanged discovery are retained. D1–D4 unchanged.
- **Deferred Task 2 minor remains for final Cycle review:** direct-SQL
  poll-kind immutability regression from `ses_ec9e9909cffeS4kt0Z0ENmyjss`.

### Contradiction Requiring Owner Ruling Before Implementation

The assignment says no network, prompt **or scan under the lease**, requires
five ordered materialization steps under **one lease acquisition**, and freezes
existing helper bodies to visibility-only changes. Approved design step 5
(`...-design.md:332–340`) explicitly requires `validate_context_worktree` before
recording `complete`, under that lease. The unchanged helper is not identity-only:

- `src/repository.rs:8315–8384`: `validate_context_worktree` unconditionally
  calls `canonical_context_at` at line 8354 (even with `require_target = false`).
- `src/repository.rs:7976–7987`: `canonical_context_at` calls
  `collect_canonical_sources`, parses candidates and validates the context.
- `src/repository.rs:8136–8144`: that collector traverses docs, tickets and
  comments. `collect_managed_sources:8175–8215` and
  `collect_document_sources:8226–8263` recursively enumerate directories;
  `collect_canonical_source:8290–8312` reads UTF-8 file bodies.

Thus calling the required unchanged validator in step 5 performs a canonical
filesystem scan while holding the lease. Moving it outside the lease, replacing
it with the stronger poll-local identity proof alone, or changing its body would
silently alter an explicit requirement. No such alteration is authorized.

**Stopped before production or test edits**, as instructed for scope
contradictions. Owner ruling requested: does the no-scan constraint permit the
unchanged canonical validation inside `validate_context_worktree` at step 5,
while discovery/index refresh scans remain outside the lease? This question is
not an accepted exception or amendment; approved Cycle/design/plan unchanged.

### Evidence And Handoff

No Rust command was run, no behavioral RED or GREEN is claimed, and no failing
test was added. The three planned RED commands remain unrun:

- `devenv shell -- cargo test --locked --lib remote::poll`
- `devenv shell -- cargo test --locked --lib remote::state`
- `devenv shell -- cargo test --locked --test repository_enablement`

This is a bookkeeping-only checkpoint, not a Rust submission or feature commit.
Only ledger, open ticket and canonical Node-generated comment
`01M4VTCVFT1CJHJ0YQ3WC77KTR` are changed. Exact checkpoint range begins
`8cb5d777758287b200e09255dc1fa7f254fc4da3`; head returned after local commit.
`git diff --exit-code -- src tests Cargo.toml Cargo.lock devenv.lock` verifies
the source/test/dependency tree is unchanged. `git diff --check` and
`scripts/show-ticket 01K7F6H9J2N4Q6S8V0X2Z4B6DG` validate whitespace and
canonical ticket/comment rendering before staging only these intended files.

Task 5 remains unticked and blocked for an owner ruling, not review-ready.
Task 6 not started; ticket open. No subagents, outward publication, native run,
merge, closure, removal or design change. Required Rust gates will run after
authorized Rust implementation; prior Task 4 verification remains historical.

## Task 4 Quality Identity And Selection Corrections — 2026-10-13T22:52:37Z

Controller reports prior spec `ses_ec8bd9b3effeS3OD0g7achFQNy` passed cumulative
Task 4 through `6daaaec520aeb37e777185cc28b8576e51b43383`. Independent quality
`ses_ec892692effePXdmvv7I1Ys7ER` then **BLOCKED with two Important findings**:

1. Matching workdir, branch spelling and OIDs do not prove context ownership.
   Root registration can remain while its deterministic directory holds a
   standalone or foreign linked repository; poll can fast-forward that foreign
   namespace while holding only the original root's lease. Add poll-local linked
   identity: expected canonical commondir, exact registered gitdir and backlink.
2. `docs/.md` (including nested forms) has no `Path::extension`, yet the tree
   reader/canonical parser accept its suffix. Discovery skips it. Poll document
   file selection must use discovery's extension boundary; `.hidden.md` remains
   selected, and ignored `.md` copies must not break accepted⇒active.

Fix base verified clean in the assigned worktree/branch:
`6daaaec520aeb37e777185cc28b8576e51b43383`. Approved contract/amendment and D1–D4
unchanged. Task 0/preflight trusted; no fetch/rebase. Only Task 4 poll-local fixes
authorized, shared helpers frozen, no subagents/later tasks. New spec re-review
must precede quality re-review. Task 2 SQL kind-immutability minor stays deferred.

### Investigation And RED Before Either Production Correction

`materialized_target` and `validate_context_worktree` check location/branch but
not opened-repository ownership. Locked `git2 0.20.4` Worktree API exposes path,
name and validate; libgit2 `worktree.c:253–285` validate checks presence/shape,
not the required exact identity. `git_worktree__read_link:75–99` resolves relative
metadata links against the admin directory. Poll must verify ownership separately.
Discovery `collect_documents` uses extension `md`; the tree reader records the
broader suffix-based sources unchanged, so poll must apply selection itself.

Before production, added real repository and ODB regressions, then ran sequential
Devenv commands:

| Command | Behavioral RED |
| --- | --- |
| `devenv shell -- cargo test --locked --lib remote::poll::tests::poll_context_identity_` | **0 passed, 3 failed, 549 filtered**, exit 101. Standalone and foreign-linked replacements kept root registration, exact expected branch/OIDs and incoming commit objects; both returned FastForwarded and changed target bytes. Same-common/wrong-admin substitution also changed state. Full root/target/foreign-owner snapshots assert preservation and Inaccessible. |
| `devenv shell -- cargo test --locked --lib remote::poll::tests::poll_document_selection_matches_discovery_extensions_including_dotfiles` | **0 passed, 1 failed, 551 filtered**, exit 101: docs/.md accepted, expected rejected. Matrix includes nested .md, top/nested .hidden.md controls, and visible document plus ignored .md duplicate; all accepted trees must be active in real discovery. |

Initial auxiliary same-common fixture had an old backlink and thus opened with
the other workdir; corrected its backlink to the deterministic path and reran
the identity filter before production. All three failures then prove behavior,
not setup. Backlink-malformation variant follows the same-common case at GREEN.
Fixture identities are pinned locally to synthetic name/email for reflogs. Object
graphs are copied with public ODB reads/writes, no clone/fake/SSH transport,
developer keys, sleeps or global Git configuration changes. RED recorded before
either production correction in comment `01M4VQFGNCG4TE2MMA6WD27C3Z`.

### Poll-Local Corrections And Final GREEN — 2026-10-13T23:34:43Z

Added a poll-local materialized-target wrapper around the unchanged sync helper.
It retains workdir/branch checks and additionally requires a context to be a
linked worktree whose canonical commondir is the owner's, whose opened gitdir
is exactly the owner's `worktrees/<ULID>` admin directory, and whose regular
non-symlink admin `gitdir` backlink resolves to the exact regular worktree `.git`
file. Registration name/path must match too; admin directories are real,
non-symlink directories. Relative backlinks resolve against the admin directory,
matching the locked backend. Invalid/unreadable identity yields Inaccessible
before cleanliness or ancestry work. Classification and update preparation use
the wrapper; a freshly opened, revalidated handle is used immediately before
the existing OID checks and shared safe updater. No foreign namespace is updated
under the original root's lease.

Conformity now selects documents under docs only when `Path::extension == md`,
matching existing discovery before parsing/ID selection. Top/nested `.md`-only
trees are rejected and create nothing; `.hidden.md` controls are accepted and
active. A visible document plus an ignored `.md` carrying the same ID remains
accepted/active because discovery excludes that copy too. The reader still
captures the broader sources unchanged; executable/mode/encoding/duplicate and
comment rules otherwise retain the approved behavior.

GREEN exercises four ownership cases (standalone, foreign-linked, same-common
wrong admin, malformed exact-file backlink) and verifies classification plus
fast-forward both return Inaccessible with full root/target/foreign-owner
snapshot equality. Six document-selection cases prove rejection/preservation
and accepted⇒active in real worktrees. Existing healthy context fast-forward
and all previous preservation/conformity tests remain green. All new test state
is local fixture Git/ODB state; no fake transport or production test seam.

Final sequential Devenv evidence on the unchanged-after-verification Rust tree:

| Command | Result |
| --- | --- |
| `devenv shell -- cargo test --locked --lib remote::poll` | **29 passed**, 523 filtered |
| `devenv shell -- cargo test --locked --lib remote::` | **326 passed**, 226 filtered |
| `devenv shell -- cargo check --all-features --locked` | PASS |
| `devenv shell -- cargo fmt --check` | PASS |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | PASS |
| `devenv shell -- cargo test --all-features --locked` | **1,629 passed, 0 failed, 1 existing ignored** |

Full log `tool_1377f574d001T58TNCzyUcVnSN`: **552 library**, **779 ordinary
integration**, **289 real-SSH** (41 comment, 25 merge, 15 observation, 63
synchronization, 31 fixture, 114 transport), **9 compile-fail doctests**.
Four new library tests added; the ignored test is the existing thousand-item
characterization. One full gate ran for these corrections, with no production
trial failure or duplicate broad repeat. Shared helpers, acceptance tests,
dependencies, lockfiles and approved Cycle/design/plan have empty diffs against
the fix base. No native run or Task 0 characterization replay.

Self-reviewed source/fixtures against both quality findings and the frozen
contracts. Only `poll.rs`, `poll_tests.rs`, ledger, open ticket and two new
canonical comments belong to the new local correction commit. Whitespace and
scripts/show-ticket schema/rendering checks precede staging intended files;
status/diff/recent log inspected. Canonical GREEN comment
`01M4VSWK7DJCKF0FWEA82VZC6Q` records re-review readiness.

Both Important findings are technically corrected, **not independently approved**.
Return for **spec re-review, then quality re-review**. Incremental fix range starts
`6daaaec520aeb37e777185cc28b8576e51b43383`; cumulative Task 4 range starts
`961b3c21db11f797e5bd1cdac0bf85d748cd1dec`; exact new head returned after commit.
Prior spec approval through 6daaaec5 remains historical controller-reported
evidence. Ticket open; no Task 5/subagents/outward publication. Deferred Task 2
direct-SQL kind-immutability minor remains explicit for final Cycle review.

## Task 4 Spec P1 Correction — 2026-10-13T22:01:48Z

Controller conveys independent spec review `ses_ec8bd9b3effeS3OD0g7achFQNy`:
**BLOCKED, P1**, before quality review. The incoming-path preflight skips any
path present in the old tree, including a tracked directory replaced by an
incoming file/symlink. Ignored descendants can then obstruct checkout after
unrelated tracked paths were already changed. Authorized fix is poll-only;
shared helper bodies and the approved contract/amendment remain unchanged.

Verified clean assigned worktree/branch at fix base
`e08d42d8cfdbc1aa5798c30a94d554d4b7f175e9`. Controller preflight/Task 0 evidence
remain trusted; no fetch/rebase. Cumulative spec re-review base remains
`961b3c21db11f797e5bd1cdac0bf85d748cd1dec`. Quality has not started; ticket open,
no later task or subagents. Deferred Task 2 direct-SQL kind-immutability minor
still goes to final Cycle review.

### Meaningful Regression RED Before Production

Added real local repository regression
`poll_retiring_directory_ignored_descendants_are_dirty_before_any_checkout`:
tracked `dir/child`, tracked `afile`, status clean with ignored
`dir/local.ignored`; incoming descendant replaces `dir` with a file and changes
`afile`. It asserts the complete before/after snapshot first, then Dirty.
Snapshot includes worktree bytes/directories/symlinks, index bytes, status,
HEAD, refs and registrations. Additional matrix variants cover nested retiring
directories, incoming symlinks, ignored nested files/empty directories/symlinks;
clean controls include noncolliding ignored state. Matrix stops at first RED.

Initial compile trial needed an explicit drop of a shadowed fixture TreeBuilder;
corrected without production changes. Correct behavioral command:
`devenv shell -- cargo test --locked --lib remote::poll::tests::poll_retiring_directory_ignored_descendants`
exits 101: **0 passed, 1 failed, 547 filtered**. The snapshot proves `afile`
changed and `dir/child` was deleted, while HEAD/index/refs stayed old and the
ignored sentinel survived; result was `Err(RecoveryRequired)` instead of Dirty.
This reproduces the P1 independently, not merely its category. No transport,
fake, sleeps, developer key or global Git configuration. Recorded before any
production correction in canonical comment `01M4VMJE8B5XZAF6KRT7YYRSBY`.

### Poll-Only Correction And GREEN — 2026-10-13T22:38:26Z

Incoming paths whose old entry is a directory now distinguish directory-to-
file/symlink replacement from ordinary tracked updates. Before the shared
updater, the poll validates tracked directory ancestors are actual non-symlink
directories, then examines only the retiring subtree's entry names/types against
the old Git tree. An extra entry (including an ignored empty directory), foreign
type or opaque nested state returns Dirty. Unreadable filesystem/tree evidence
returns Inaccessible, never a clean guess. Tracked symlinks are treated as leaves;
no file body is read and no local symlink is traversed. Normal tracked-file-to-
directory transitions keep the existing path. No discovery refresh, transport
or prompt occurs in this targeted local-state preflight.

The shared `sync::fast_forward` body and all other synchronization/ref/discovery
helpers remain byte-for-byte the fix base; no rollback, forced ref, deletion or
new testing hook is introduced. Approved Cycle/design/plan and one-way agreement
are unchanged. This repairs implementation of the existing preservation contract.

Two new tests execute **20 obstructed cases** on Linux: root/nested retiring
directory × incoming regular file/symlink × ignored file, ignored nested file,
ignored descendant under deeper tracked directories, ignored empty directory,
ignored symlink. Every case proves complete snapshot equality, Dirty and an
unchanged outside symlink-target sentinel. **Four clean controls** still advance
HEAD/index/worktree while preserving a noncolliding ignored file. Windows omits
symlink variants rather than changing permissions; no native run is claimed.
Initial GREEN passed all 25 poll tests; deeper tracked-descendant hardening then
passed in the final focused and full runs. No failed production fix trial or
duplicate broad run for this correction.

All final Rust commands ran sequentially through Devenv in the assigned worktree:

| Command | Result |
| --- | --- |
| `devenv shell -- cargo test --locked --lib remote::poll` | **25 passed**, 523 filtered; obstruction and clean-control matrices green |
| `devenv shell -- cargo test --locked --lib remote::` | **322 passed**, 226 filtered |
| `devenv shell -- cargo check --all-features --locked` | PASS |
| `devenv shell -- cargo fmt --check` | PASS |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | PASS |
| `devenv shell -- cargo test --all-features --locked` | **1,625 passed, 0 failed, 1 existing ignored** |

Full log `tool_1374f3fe8001N5kMGAMMp9qtK8`: **548 library**, **779 ordinary
integration**, **289 real-SSH** (41 comment, 25 merge, 15 observation, 63
synchronization, 31 fixture, 114 transport), **9 compile-fail doctests**. Existing
SSH/shared suites and tests are unchanged; no transport claim comes from the
new local tests. Task 0 trusted evidence not replayed; no native dispatch.

Only `poll.rs`, `poll_tests.rs`, ledger, open ticket and the two correction
comments belong to this new local fix commit. Git diff confirms shared helpers,
repository.rs, acceptance tests, dependencies and lockfiles are unchanged. Source
and recent log inspected; canonical comments/schema and whitespace validated
with scripts/show-ticket and git diff --check before staging intended files.

Technically corrected P1 awaits **spec re-review** by
`ses_ec8bd9b3effeS3OD0g7achFQNy`; no approval is inferred, **quality not started**.
Incremental fix base `e08d42d8cfdbc1aa5798c30a94d554d4b7f175e9`; cumulative Task 4
spec range starts `961b3c21db11f797e5bd1cdac0bf85d748cd1dec` and ends at the new
commit head returned after commit. Canonical GREEN/re-review comment
`01M4VPNGXTADT2ASHG97ZQH66Q`. Ticket remains open, Task 5 unstarted, no subagents.
Deferred Task 2 SQL kind-immutability minor remains for final review.

## Task 4 Owner Amendment And Resume — 2026-10-13T20:09:05Z

The controller conveyed the owner's **"Yes"** approving the focused amendment:
retain executable rejection and unchanged existing discovery; agreement is
**accepted tree ⇒ active context only**. Remove rejected ⇒ inactive; rejected
trees must still be rejected by polling with no automatic creation.
Materialization validation may be stricter than local discovery. D1–D4 unchanged.
Updated the approved Task 4 agreement, relevant design text and Cycle contract
for consistency before production behavior. The prior blocker RED is historical,
separate evidence, not a test for the amended agreement.

Controller's fresh fetch/rebase succeeded as a clean no-op: main
`d1ca88772c5768d55aadb2ca4dcac1c69dab0d17`, before/after ticket HEAD and exact
Task 4 review base `961b3c21db11f797e5bd1cdac0bf85d748cd1dec`. Implementer
confirmed clean assigned worktree/branch at that SHA. No repeated preflight.
Task 4 only resumes inline, no subagents, local commit/bookkeeping then return
for independent spec and quality reviews. Task 3 cumulative approvals remain
controller-reported and trusted; Task 2 minor direct-SQL kind-immutability
regression remains explicitly deferred to final review. No later task authority.

Canonical Node-generated comment `01M4VE4298RCK58VXV4YT3M6P8` records owner
amendment/resume. Task 4 is unticked until delivered; ticket remains open.

### New Task 4 RED Before Behavior — 2026-10-13T20:17:29Z

`devenv shell -- cargo test --locked --lib remote::poll` first exited 101 with
58 missing-new-API compile errors. Added only behavior-free signatures/enums and
`todo!` bodies; corrected runtime RED exits 101: **8 passed, 12 failed, 523
filtered**. All 12 new tests fail at the missing guidance/relation/classification/
fast-forward/conformity placeholders. No helper visibility or behavior changed
yet. Matrix loops stop at their first placeholder; subsequent GREEN will exercise
every case. This is distinct from the historical pre-amendment blocker RED.

Fixture trials, not behavior RED: symlink creation initially used libgit2's
trailing-slash workdir and failed ENOENT; corrected to the exact deterministic
path. An auxiliary clean-status/different-index-tree probe using disabled
filemode, ignorecase and a corrupt TREE cache did not establish that prerequisite:
mode/case changes are reported staged, and libgit2 recomputes the corrupt cache.
Removed that unsupported auxiliary probe (including its temporary OpenSSL use).
The real different-index-tree case remains in the preservation matrix and expects
Dirty because staged status precedes the defensive tree-equality guard; no
production classification-order adaptation. Initial placeholder warning is
unused update fields, which implementation will consume.

Tests use real local repositories, ODB-built trees, full worktree/index/status/
HEAD/ref snapshots, no transport/fake/sleep/global Git mutation. Owner-amended
agreement validates all accepted trees through real worktree discovery and checks
rejected trees cause no branch/path/registration creation. Production starts only
after this recorded RED checkpoint and canonical comment
`01M4VEKEC2JMK9AD1DH3NFFHCW`.

Additional conformity regression RED before its correction: the same command
exits 101, **21 passed, 1 failed, 523 filtered**, for a valid ordinary document
plus an executable duplicate carrying the same ID. Filtering modes before
identity parsing hid the duplicate that discovery would include. Added a
discovery-excluded `docs/.git/` source case to the same test. The one-way agreement
requires retaining duplicate identity checks before mode filtering, and excluding
sources discovery cannot observe. Actual local ahead/diverged/missing-ancestry
repository hardening is already green (not claimed as new behavior RED).

After an initial passing full gate (1,622 tests), self-review found the incoming
path precheck could misclassify a clean tracked-file-to-directory change as
Inaccessible: checking the child first produces ENOTDIR for the tracked parent
that safe checkout is authorized to replace. Added regression before correcting
that precheck: `devenv shell -- cargo test --locked --lib
remote::poll::tests::poll_fast_forward_allows_clean_tracked_file_to_directory_transition`
exits 101, **0 passed, 1 failed, 545 filtered**, expected FastForwarded, got
Inaccessible. This is a poll-only precheck bug, not a shared helper rewrite or
another product decision. Re-run required gates on the corrected Rust tree.

### Task 4 Implementation And Final Verification — 2026-10-13T21:50:29Z

Delivered Task 4 only under the owner's narrowly approved agreement amendment:

- `PollDisposition` has all **15** contract variants and fixed static guidance.
  No free-text reason or recovery content storage. Pure `classify_relation`
  consumes unchanged `clean_descendant`; either ancestry-query error propagates,
  never guessed as divergence. The Git-backed classifier surfaces ancestry
  failure as the existing typed RecoveryRequired error.
- `classify_local_target` skips deleted/unknown remote contexts before any local
  classification, preserving that batch category. Otherwise it checks materialized
  identity/exact workdir, then uses unchanged cleanliness checks: conflict/state
  metadata before any dirty status (including untracked directories); defensive
  index-tree mismatch/read failure is Inaccessible. Equal/current, strict behind,
  ahead and divergence are separate typed outcomes; context compares only its own
  incoming OID. No primary integration or publication.
- `poll_fast_forward` is a remote-module local-stage seam, acquiring the common
  repository lease itself. Under it, it reobserves canonical configuration/ref
  plan, frozen endpoints/key evidence, own tracking OID, branch OID and local
  cleanliness, checks incoming path collisions, then rechecks OIDs immediately
  before/after calling unchanged locked safe checkout/ref update. An ignored
  incoming collision is Dirty before effects. No transport, prompt or discovery
  refresh runs under this lease. Target-order orchestration and stored report
  rows belong to Task 6, not this seam.
- `conforming_context_tree` uses the existing ODB reader's sources/modes and the
  widened active-item predicate plus `canonical::validate_context`. It checks
  exact ticket path or unique document ID/kind, regular non-executable UTF-8
  item sources, rejects any gitlink and any `.manyhands/worktrees` entry, and
  preserves malformed-comment conformity with ordinary discovery diagnostics.
  Document traversal bounds/excluded `.git` sources preserve accepted⇒active.
  Matching executable duplicates are not hidden before identity validation.
  Rejected validation creates no branch, directory or registration; accepted
  matrix trees are checked out and accepted by existing discovery. No rejected⇒
  inactive assertion remains.
- Shared `sync.rs`, `refs.rs` and `discovery.rs` diffs are **visibility only**
  (including the tree snapshot fields). No helper body/test hook rewrite, no
  synchronization contract change. `repository.rs` needs no edit: its existing
  parent-private configuration/lease helpers are already accessible to children.
  Five Rust files changed, no frontend/schema/dependency/lockfile changes.

Preservation snapshots cover worktree file/directory/symlink bytes, target index,
Git status, HEAD, sorted refs, and worktree registrations (including context
HEAD/index files). The staged different-index-tree matrix case is Dirty by the
approved status-first order; the shared defensive tree check remains unchanged.
Tests have real local Git state, no fake transport, sleeps, developer keys,
global Git configuration or cfg(test) production checkpoint hook. Existing
harness-false transport suites run unchanged over real SSH.

#### Honest Trial And Investigation History

- First implementation: **19 passed, 1 failed** for ignored collision returning
  RecoveryRequired rather than Dirty. A diagnostic proved git2 `dry_run` returns
  success here. Locked source `git2-0.20.4/src/build.rs:350–353` selects checkout
  NONE; `libgit2/checkout.c` short-circuits that strategy before ignored conflict
  selection. Replaced only the new poll preflight with incoming-path inspection,
  retaining the shared safe updater. Diagnostic removed; focused **20 passed**.
- The executable-duplicate RED above led to identity checking before mode
  filtering. The second `.git` case initially failed fixture setup because
  TreeBuilder protects the name; the test writes that hostile tree directly to
  the ODB, without changing global Git options. Final regression and matrix pass.
- Initial required check/fmt passed; Clippy failed on three large shared-error
  results and one test type-complexity. Followed sync/observation's existing
  result_large_err module convention to preserve the typed seam and added a
  test alias. No public shared-error or helper behavior change.
- Initial full gate passed **1,622 tests**. The additional tracked-file-to-
  directory RED above justified a second full gate after correcting ancestor
  inspection. First focused retry reached FastForwarded, correct HEAD and file
  bytes, but asserted against the fixture's cached old index; corrected the
  verification to `index.read(true)` before tree comparison. No production
  change for that stale-handle assertion. Final focused and broad gates pass.

#### Final Evidence (Sequential Devenv Commands)

| Command | Result |
| --- | --- |
| `devenv shell -- cargo test --locked --lib remote::poll` | Final GREEN: **23 passed**, 523 filtered; 15 new tests over the 8 existing |
| `devenv shell -- cargo test --locked --lib remote::` | **319 passed**, 226 filtered before the final shape regression; final full gate includes the resulting 320 matches |
| `devenv shell -- cargo test --locked --test remote_synchronization` | **63 real-SSH cases**, unchanged; final full gate again passes all 63 |
| `devenv shell -- cargo test --locked --test remote_merge_recovery` | **25 real-SSH cases**, unchanged; final full gate again passes all 25 |
| `devenv shell -- cargo check --all-features --locked` | PASS on final Rust tree |
| `devenv shell -- cargo fmt --check` | PASS on final Rust tree |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | PASS on final Rust tree |
| `devenv shell -- cargo test --all-features --locked` | PASS on final Rust tree: **1,623 passed, 0 failed, 1 existing ignored** |

Final full-gate log: `tool_137257cf9001DtpHhLQfzDVEtz`. Breakdown: **546 library**,
**779 ordinary integration**, **289 real-SSH** (comment publication 41, merge
recovery 25, observation 15, synchronization 63, SSH fixture 31, SSH transport
114), **9 compile-fail doctests**. Both binary targets have zero tests. The ignored
test remains the existing thousand-item read characterization. Native matrix
was not run. Task 0 characterization/baseline was not repeated.

Unchanged SHA-256: Cargo.lock
`8a20e8ba089f63bddfa9b87a08d5d4308404b4c2440c050453654e422a9549f8`;
devenv.lock `d4a78b8259525e7dcf66814477b0f96282e6570c9a21ebc84eda7d7fb8950d6e`.

#### Independent Review Handoff

Self-reviewed actual source and shared-helper diffs against amended Task 4.
Task 4 delivered and ticked; independent spec and quality gates remain unticked.
Exact review base is **`961b3c21db11f797e5bd1cdac0bf85d748cd1dec`**; head is
returned after the single local implementation/bookkeeping commit. That range
includes the owner-approved amendment. Prior blocker tests and independent
confirmation remain historical evidence in earlier commits/ledger sections.
No second material decision or unresolved test failure. Ticket open, no Task 5.

**Deferred Task 2 minor remains for final Cycle review:** direct-SQL poll-kind
immutability regression suggested by `ses_ec9e9909cffeS4kt0Z0ENmyjss`.
Task 3 cumulative approvals remain controller-reported, trusted and ticked.
No implementer subagents; return this range to the controller for both reviews.
Canonical review-ready comment `01M4VKXQKGZT8153HENHVW8QVB` records final evidence.
Only intended Task 4 Rust/docs/ticket/comments are staged after status/full diff/
recent-log inspection, whitespace and scripts/show-ticket validation.

`scripts/show-ticket 01K7F6H9J2N4Q6S8V0X2Z4B6DG` validated/rendered the open
ticket and all canonical comments, including the three new Node-generated ULIDs.
`git diff --check` passed. Source/helper and amendment/bookkeeping diffs and recent
log were inspected; frontend, repository.rs, acceptance tests and dependency/
lockfile diffs are empty. Only documentation wording/indentation changed after
the passing final Rust gate, so no duplicate Rust verification is needed.

## Task 4 Independent Blocker Confirmation — 2026-10-13T19:58:22Z

Controller conveyed independent read-only blocker review
`ses_ec92a184effe7ndQKcvXtxKxkg`: **CONFIRMED contract contradiction**. This
records reviewer confirmation, not owner approval of an amendment. Earlier Task 3
review approvals were likewise reported by the controller, not directly by the
owner. Documentation-only checkpoint base:
`b3f989c0f963c3b933d8ab4479e037271e70e775`; assigned worktree/branch verified clean.

The independent review grounds the contradiction in these unchanged sources:

- Design lines 303–304 require regular non-executable canonical sources.
- Implementation plan lines 310–318 require negative executable-mode tests and
  two-way agreement with existing discovery; lines 325–327 restrict shared
  helper/rule changes to visibility widening without body changes.
- `src/repository/discovery.rs:389–479` accepts a valid registration, exact
  authoring branch and valid matching item. Lines 1077–1078 and 1304–1336 collect
  regular UTF-8 files without an executable-permission exclusion.
- `src/canonical.rs:354–359` accepts the reproduced document's valid fields;
  canonical document parsing has no executable-mode input.
- **Tree-reader clarification:**
  `src/repository/remote/sync.rs:4487–4533` captures entry modes in the snapshot;
  it does not itself reject executable entries. Future poll conformity validation
  must enforce any retained non-executable rule using that captured information.

### Focused Proposal For Owner — UNAPPROVED, Awaiting Decision

Recommended amendment: retain the non-executable poll conformity rule and leave
discovery behavior unchanged, but require only **accepted tree ⇒ active context**
after checkout. Poll validation may be stricter than discovery. Remove the
**rejected tree ⇒ inactive context** implication from the agreement requirement.
This proposal is recorded for decision only; it does not change the approved
Cycle contract, design or implementation plan.

Other options require an explicit owner decision: accepting executable poll
items changes the approved policy; authorizing discovery behavior changes
broadens Task 4's scope beyond visibility-only changes. No option is approved
by this review or this bookkeeping authorization.

**Task 4 remains blocked awaiting OWNER amendment**, unticked, ticket open.
No implementation or test was added or run for this checkpoint, and no later
task was started. The deferred Task 2 direct-SQL kind-immutability minor remains
explicit for final Cycle review. Canonical Node-generated comment
`01M4VDGDZ2RRWK1V2RTTN6JHMJ` records the independent confirmation and unapproved
proposal. Only ledger, ticket and comment are intended for the authorized local
documentation commit; schema/rendering and whitespace checks precede commit.

## Task 4 Blocked Before Production — 2026-10-13T19:53:29Z

Fresh implementer assigned **Task 4 only**, inline, no subagents, in the assigned
ticket worktree and branch. Verified clean base
`34b97c178a48b39b6ae32e30de52ff5722d1dd06`. Controller's resume preflight and
approved Task 0 gate are trusted; no repeated fetch/rebase or characterization.
Current AGENTS, approved design/implementation plan, ledger, ticket, poll tests,
synchronization helpers and discovery rule were inspected before production work.

### Prior Task 3 Reviews Accepted

Controller reports cumulative Task 3 approval after the atomic-absence guard fix,
through this Task 4 base, for range
`6cb69f4b6dd4f9798e509f9cdddc1acc7cd4f39a..34b97c178a48b39b6ae32e30de52ff5722d1dd06`:

- Spec `ses_ec9b35ee6ffeBdb0HkkfCCp9R4`: approved cumulative corrected work.
- Quality `ses_ec9b0a1d9ffeWP162PBxjUK6l4`: approved cumulative corrected work;
  previous Important atomic-absence blocker resolved.
- Ticket's prior Task 3 review checkboxes now reflect that controller report.
- **Deferred Task 2 minor remains explicit for final Cycle review:** direct-SQL
  poll-kind immutability regression suggested by quality review
  `ses_ec9e9909cffeS4kt0Z0ENmyjss`. It was not addressed in this blocked task.

### Meaningful RED And Confirmed Contract Contradiction

Before any production code or visibility change, added temporary test
`repository::remote::poll::tests::rejected_executable_item_tree_is_not_active_in_existing_discovery`
to `src/repository/remote/poll_tests.rs`. It tests the required two-way discovery
agreement against existing discovery, without needing a new poll placeholder:

1. Initialize a real local repository with initial branch `main` and explicit
   fixture signature (`Test`, `test@example.invalid`, timestamp 0).
2. Build the document blob and nested Git trees in memory. Source is
   `---\nmanyhands_managed: true\nmanyhands_kind: document\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAV\ntitle: Fixture document\n---\n`.
   Insert it at `docs/document.md` with mode **100755**, assert that stored mode,
   and create the fixture commit on HEAD.
3. Create branch `manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV` at that commit,
   then a registered worktree named `01ARZ3NDEKTSV4RRFFQ69G5FAV` at the exact
   `.manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FAV` path with
   `WorktreeAddOptions::reference`. Verify checked-out document bytes.
4. Write root `.manyhands/config.toml` with
   `format_version = 1\nprimary_branch = "main"\n`.
5. Call existing `discovery::observe_root`; assert `active_contexts.is_empty()`
   because the approved poll rule rejects executable item entries.

`devenv shell -- cargo test --locked --lib remote::poll` exited **101**:
**8 passed, 1 failed, 523 filtered, 0 ignored**. Failure was the final assertion,
not setup or compilation: **"executable item rejected by the approved poll rule
was accepted as an active context"**. The real Git tree/worktree was created
successfully. No transport, fake, sleep, developer key or global Git mutation.

Source explains the mismatch: `discovery.rs::collect_source` accepts regular
files without excluding executable permissions; `collect_authoring_item_sources`
and `authoring_item_matches` then accept this valid document. The item-only
predicate has no file-mode input. In contrast, design Local Stage/Conformity
requires regular **non-executable** sources, and Task 4 explicitly requires every
rejected tree checked out as a worktree to be **not active** in existing discovery.
Task 4 also limits shared helpers/rules to visibility widening with unchanged
bodies and synchronization contracts. These requirements cannot all hold for
the reproduced executable tree. This is not an amendment to D1–D4 or Task 0.

### Stop And Handoff

Stopped for owner amendment as explicitly instructed. No Task 4 production API,
classification, fast-forward, helper visibility or later-task work was implemented.
The temporary probe was removed with a patch after recording RED;
`git diff --exit-code -- src tests Cargo.toml Cargo.lock devenv.lock` confirms
the Rust/tests/dependency tree remains byte-for-byte at the reviewed Task 4 base.
The probe is reproducible from the steps above; it is not committed as a failing
permanent test or weakened to green. No Task 4 GREEN or full Rust gate is claimed;
this is a bookkeeping-only blocker checkpoint, not a Rust submission.

Task 4 remains unticked and blocked, ticket open. Required owner amendment must
reconcile executable-tree rejection and two-way discovery agreement with the
visibility-only restriction before implementation resumes. Native evidence and
the deferred Task 2 minor remain open. Canonical Node-generated comment
`01M4VD7G36JQ3T0D3VXEX7YDTQ` records prior approvals and the blocker.

Only this ledger, the existing open ticket and that comment belong to the local
blocker checkpoint. Its exact head is returned after committing; the prescribed
feature commit is not appropriate because no Task 4 feature was delivered.

## Planning Authorization And Preflight — 2026-10-11

The owner asked for the Wave 02 Cycle 08 Cycle document, design and detailed
implementation plan, a review of them, and presentation for review and
approval, on existing ticket `01K7F6H9J2N4Q6S8V0X2Z4B6DG`. Authorization is
planning only. No implementation, commit, push, CI dispatch, merge, closure or
cleanup was authorized or performed.

- Repository: `/home/vhodges/work/src/manyhands`.
- Verified registered ticket worktree:
  `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01K7F6H9J2N4Q6S8V0X2Z4B6DG`.
- Branch: `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DG`.
- Ticket worktree was clean before the rebase. The main checkout had unrelated
  untracked `.superpowers/`; left untouched. Other worktrees untouched.
- Fresh `git fetch origin` succeeded; local main and `origin/main` agreed at
  `d1ca88772c5768d55aadb2ca4dcac1c69dab0d17` (Cycle 07 merge, pull request 17).
- Before ticket HEAD: `5539c60eeb4022c71b1dc13716aeb2d2545a7375`.
- Rebase onto the fetched base succeeded without conflicts (386 commits behind,
  one ticket checkpoint replayed).
- After ticket HEAD: `27d5ef54c67a116a590602d86b7086f8602c0c62`.
- `merge-base --is-ancestor` passed; post-rebase status was clean. Main was
  never rebased or switched.
- The published ticket ref still holds the original checkpoint, so the branch
  reads ahead 387 and behind 1. A later push must reconcile that ref; a force
  push is not authorized.
- `manyhands-cli` is startup-only with no ticket or comment operation, so
  ticket and comment updates use the canonical filesystem paths.

## Grounding

Read `starting-a-cycle` with its preflight reference, `review-cycle-docs` with
its matrix, `AGENTS.md` and the ticket. Read Wave 02 in full; the polling,
remote-ref, coordination and reservation sections of the Git workflow, index,
MVP, runtime and test RFCs; the CLI contract's poll rows; the Wave 03 API
audit's polling gaps; and the Cycle 04 and Cycle 07 documents, design and
ledger for contract and format. Inspected `remote/{observation,reservation,
state,sync,refs}.rs`, `transport/{remote,operation,callbacks}.rs`,
`repository.rs` context creation, `discovery.rs`, `recovery.rs`,
`read/{status,dto}.rs`, `results.rs`, the SSH fixture and its test targets,
`Cargo.toml` and the native workflow.

Source inspection used three delegated read-only passes plus direct reads of
the reservation finish logic, the journal-begin guard, the operation-table
schema, the snapshot reader and the failure-point seam. No Rust command was
run. Findings that shaped the design are listed under Current Source Boundary
in the design.

## Artifacts

- [Cycle](../Cycles/wave-02-cycle-08-one-shot-polling-and-materialization.md)
- [Design](2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-design.md)
- [Detailed implementation plan](2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-implementation.md)

## Skill-Based Self-Review

Applied `review-cycle-docs` and every row of its matrix to the three documents
against the authorities and the inspected source.

| Source | Concern / promised behavior | Classification | Resolution / ruling | Proof or follow-up |
| --- | --- | --- | --- | --- |
| Git RFC, Remote Refs | "Before a poll prunes stale tracking refs" implies pruning; a tracking ref can be the only local reference to a collaborator's commits | **Decision D1** | Recommended: never delete a tracking ref in this Cycle; cost is accumulating stale refs until Cycles 09/10 | Owner answer; Task 3 no-deletion tests |
| Wave 02 "materializes exactly once"; Git RFC "never clean up" | An interrupted worktree creation can leave a partial directory; finishing or removing it are both recovery policy | **Decision D2** | Recommended: resume only branch-created-with-no-path; report anything else and remove nothing; cost is a rare manual repair | Owner answer; Task 0 characterization; Task 5 interruption tests |
| Cycle 06/07 conflict rule; reservation code | A pending conflict makes every synchronization `Busy` but does not stop a poll, and a poll moves tracking refs | **Decision D3** | Recommended: refuse before transport with a typed non-failure result; cost is no background discovery until the conflict is resolved, with no abandon path yet | Owner answer; Task 0 characterization; Tasks 2, 6, 7 |
| MVP and Git RFC manual priority; `recovery.rs` guard | A local save or comment begun during a poll is refused with `RecoveryRequired`; the full poll widens that window from an advertisement to a fetch and checkout | **Decision D4** | Recommended: against a poll-priority record, request yield and return `RepositoryBusy`; cost is a narrow change to a Wave 01 guard | Owner answer; Task 2 and Task 7 tests |
| Wave 02 gate "obeys stored policy"; existing observe ignores policy | Whether the domain or the caller enforces eligibility | Ruling | Ruling: the poll gates automatic invocations itself through one pure function also exposed read-only — the Wave gate requires it and Wave 03 needs the same answer — cost if wrong: Wave 03 double-checks harmlessly | Tasks 1, 6, 7 |
| Runtime RFC "Explicit Poll now bypasses automatic pause/backoff" | The RFC names pause and backoff; `enabled = 0` and recovery suspension are not named | Ruling | Ruling: explicit bypasses all four because each gates automatic polling, and clears none — matches the existing explicit observation — cost if wrong: one extra deliberate fetch; revisit if the owner wants `enabled = 0` to forbid explicit polls | Task 1 and Task 7 policy cases |
| Runtime RFC clock-rollback clamp | A stateless clamp never becomes due | Ruling | Ruling: persist the clamped deadline once — only a stored bound counts down — cost if wrong: one extra write in an otherwise read-only check | Task 1 clamp tests |
| Runtime RFC "host failure never becomes ordinary network backoff" | Next eligible time after key, unlock and host failures | Ruling | Ruling: store the category, keep backoff, use the interval; session suspension stays Wave 03 state — cost if wrong: a background attempt per interval that returns without prompting | Tasks 1, 2, 7 |
| Git RFC poll refspecs | Wildcards would download malformed family names and give them tracking refs | Ruling | Ruling: explicit mappings for names that parse, equivalent to the RFC set restricted to recognized contexts — nothing may use a malformed ref — cost if wrong: a malformed ref's objects are not local | Task 3 |
| Cycle 04 observation model | Reuse the advertisement-only operation or add one | Ruling | Ruling: new operation plus stored poll kind; observe unchanged — its callers rely on no local change — cost if wrong: one column and a rule fork | Task 2 frozen-behaviour tests |
| Operation table `CHECK` lists | New phase or safe-point names force a table rebuild | Ruling | Ruling: reuse `local_prepared`, `before_local_mutation`, `before_discovery`; verified the schema has no action-to-phase constraint — cost if wrong: status text shared with synchronization | Task 2 |
| Restart contract | Cycle 04 refuses restart at `BeforeLocalMutation` | Ruling | Ruling: `update` polls may restart from any step because each local step acts only on state re-observed under the lease — cost if wrong: an extra fetch | Tasks 2, 6, 7 reopen cases |
| Poll ID versus local ID rule | A poll cannot share its ID with the refresh | Ruling | Ruling: durable refresh-required flag before completion, then a refresh under a fresh internal ID; no `index_pending` on the poll record — cost if wrong: a stale index repaired by any refresh | Task 6 index tests |
| Index RFC "appear as recovery problems" | Poll exceptions written to `problems` would be erased by the refresh | Ruling | Ruling: store fixed dispositions with fixed guidance in remote state, exposed on the remote snapshot — the refresh owns `problems` — cost if wrong: Wave 03 maps dispositions into its problem view | Task 6 snapshot tests; Wave 03 read DTO obligation |
| MVP RFC "local tracking branch" | Whether to write upstream branch configuration | Ruling | Ruling: none; no existing context has any and tracking derives from the ref plan — cost if wrong: an additive later write | Task 5 |
| MVP RFC primary fast-forward | A primary branch not checked out could be moved safely | Ruling | Ruling: leave it, as synchronization does, and report `inaccessible` — cost if wrong: one unchanged branch | Tasks 4, 7 |
| Git RFC item synchronization step 2 | Integrating primary into a context | Settled | A poll compares a context with its own tracking ref only; anything else is a merge | Task 4 |
| Remote rename | No rename signal exists in Git | Settled | Old name `remotely_deleted`, new name `nonconforming` when the item ID no longer matches; local context kept | Task 7 rename case |
| Authentication RFC; Cycle 03/04 session model | Prompts and trust during an automatic poll | Settled | Existing automatic-scope behaviour: no prompt when the session is blocked; host approval typed and never implied | Tasks 3, 7 |
| Privacy contract | New tables and columns | Settled | IDs, ref names, OIDs, timestamps and fixed enumerations only; scans extended | Tasks 1, 6, 7 |
| Runtime RFC transport limits | No total poll deadline | Settled | Per-call limits stand; in-transfer stop is added through the progress callback as the Git RFC requires | Task 0, Task 3, native evidence |
| Test strategy RFC | Fixture fidelity | Settled | Real in-process SSH with Git helpers for every transport claim; no fake transport; library tests cover only post-fetch logic on real local repositories | Tasks 3, 6, 7 |
| Native platforms | Progress-callback stop, worktree checkout failure and path length differ by platform | Feasibility gap | Not provable by inspection; Task 0 characterizes on Linux, native runs need separate dispatch authorization | Task 0; Task 8 native record |
| Lifecycle | Closure and publication | Settled | Ticket stays open; closure in the final pre-merge push after approval; every outward step separately authorized | Task 8 |

At review time four product and recovery questions were open for the owner
(D1–D4); all four were decided as recommended, recorded below. Task 0
characterizations and native evidence are open **gates**, not decisions.

### Corrections Made During Self-Review

- The first plan draft relied on a "fixture transport seam" for library tests.
  None exists. Task 6 now isolates the post-fetch stage as one function that
  library tests drive on a real local repository, and leaves every transport
  claim to real SSH.
- The first plan draft said "the existing failure-point seam" covers
  materialization. The seam is the public `FailurePoint` enumeration, which
  needs new poll variants. Task 5 now says so.
- Checked the operation table for a constraint tying `local_prepared` to
  synchronization actions before relying on it; there is none, and the active
  reservation index and the local journal guard already include that phase.
- The design first compared the clamp statelessly. It now persists the bound.
- Confirmed the three documents agree on dispositions, the result table, the
  decision markers and the task that proves each acceptance row.

## Verification And Approval Checkpoints

Planning checks only: internal links resolve, document IDs are unique canonical
ULIDs, and the new files have no whitespace errors. No Rust or native test was
run; all verification in the plan is future work.

Approval needed before implementation: the Cycle, design and plan; answers to
D1–D4; and separate implementation authorization. Proposed lifecycle
checkpoints are one ticket comment per task, an independent review per task,
the full local gate, a whole-branch review, native runs when authorized, and
closure in the final pre-merge push after approval.

## Document Approval And Commit Authorization — 2026-10-11T14:39:22Z

The owner approved the Cycle, design and implementation plan, approved the
recommended answer to each open decision, and authorized committing this
planning checkpoint.

| Decision | Answer |
| --- | --- |
| D1 stale tracking refs | Never delete a tracking ref in this Cycle |
| D2 half-created worktree | Resume only branch-created-with-no-path; report anything else and remove nothing |
| D3 polling during a pending conflict | Refuse before transport with a typed non-failure result |
| D4 local action during an automatic poll | Request yield and return retryable `RepositoryBusy` |

Document status changed from proposed to approved in all four artifacts. The
approval does not authorize implementation, push, CI dispatch, merge, closure
or cleanup. The planning commit is local; the published ticket ref still holds
the pre-rebase checkpoint and was not force-pushed.

## Task 0 Execution — 2026-10-11

### Authority, Scope And Base

The owner explicitly authorized rebase, Tasks 0–8 implementation, local commits
and canonical ticket/comment/ledger updates. This implementer was assigned
**Task 0 only**, inline with no subagents, followed by independent spec and
quality reviews. The 2026-10-11 document approval and D1–D4 answers at
`2026-10-11T14:39:22Z` were re-read, not reopened:

- D1: never delete tracking refs.
- D2: resume only an intent's branch at its recorded OID with no path and no
  registration; retain and report all half-created worktrees.
- D3: a pending synchronization conflict blocks the new full poll before
  transport, with a typed non-failure result.
- D4: a local action during an automatic poll requests yield and receives
  retryable `RepositoryBusy`.

All shell commands used the assigned workdir
`/home/vhodges/work/src/manyhands/.manyhands/worktrees/01K7F6H9J2N4Q6S8V0X2Z4B6DG`
and branch `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DG`. Read current
`AGENTS.md`, the implementing skill's preflight/verification references, all
three approved documents, this ledger and the ticket's existing comments.
The CLI remains startup-only, so canonical files were used for bookkeeping.

Preflight, before mutation:

- `git worktree list --porcelain` confirmed the assigned registration and branch.
- Ticket status was clean. Main status was read for preflight only: unrelated
  untracked `.superpowers/` was preserved; other worktrees were untouched.
- `git rev-list --left-right --count main...origin/main` returned `0 0`.
- Fresh `git fetch origin main` succeeded. Rebase onto `origin/main` was a
  clean no-op, with no conflicts and no `AGENTS.md` change.
- Base: `d1ca88772c5768d55aadb2ca4dcac1c69dab0d17`.
- Before and after rebase / **Task 0 base HEAD**:
  `094e5734d1d4480ee29e6f675aaba4a8883b9060`.
- `git merge-base --is-ancestor origin/main HEAD` returned success;
  post-rebase `git status --short --branch` showed a clean ticket branch.

### Caller Re-Scan And Wave 03 Compatibility

Re-scanned `src/` and `tests/` for `observe_publication_remote`,
`reserve_remote_operation*`, `remote_safe_point`, `finish_remote_operation`
and `RemoteSnapshot`; also re-read the Wave 03 API audit's C2/C4, E-observe,
E-policy, E-reserve and `poll once` rows. `git diff origin/main -- src tests
docs/research/wave-03-api-audit.md` was empty before characterization: no new
caller or changed production assumption since the approved planning base.

Production consumers remain observation orchestration in `remote/observation.rs`,
reservation/safe-point consumers in `remote/sync.rs`, and snapshot construction,
read and re-exports in `remote/state.rs`, `remote/mod.rs` and `repository.rs`.
The other direct callers are existing library and acceptance tests (including
comment publication and read-status tests). Neither frontend calls the remote
operations, and no implemented Wave 03 consumer was found.

Carry these compatibility items into the later tasks:

1. Wave 03 W3-02/W3-12 maps service-owned reservation tokens and durable
   cancel/yield acknowledgement into bounded-worker progress; a request is not
   an immediate cancellation of a blocking SSH call.
2. E-observe remains advertisement-only, with its existing replay, policy and
   safe-point behavior unchanged. The new full poll must be a separate API.
3. E-policy/E-reserve and W3-06/W3-12 `poll once` need the new eligibility,
   disposition and attempt-time accessors later; existing `RemoteSnapshot`
   publication/history fields and stored-row validation must retain their meaning.
4. The desktop scheduler/session attention state and unsaved-draft protection
   remain downstream obligations. This Linux characterization is not native
   matrix proof.

### Five Executed Throwaway Characterizations

Temporary assertions were added only to existing test targets. No production
behavior, dependency, target or workflow was changed. Real transport claims
used the in-process `russh` fixture and Git upload-pack, isolated homes and
pre-thread initialization. No fake transport, sleeps in test/controller code,
developer keys or global Git configuration were used; transfer pacing is the
existing fixture facility. No `cfg(test)` hook was used by a harness-false target.

**1. Stored tracking OID and snapshot equality — PASS.**
`task0_tracking_batch_equality` in `remote/reservation_tests.rs` created two
real local commits, committed an observation with tracking OID A, then moved
the actual context tracking ref to B before another batch. The ordinary
snapshot and a freshly reopened service still returned the unchanged A
snapshot. Committing the next batch with tracking OID B put B in both its
observation and context snapshot. A negative control changed only the stored
`remote_context_states.tracking_oid` back to A: snapshot returned
`RecoveryRequired`. Thus tracking OID is the value captured when the observation
is constructed, not a live ref or an expected pre-fetch value. The equality
check in `state.rs:4567–4582` compares the two **stored** rows, not Git refs.
Writing tracking refs before constructing/committing the complete batch is
compatible, and a crash between those stages does not itself corrupt a snapshot.

**2. Interrupted and failed synchronization after tracking movement — PASS.**
`task0_restart_moved_tracking` in `tests/remote_synchronization.rs` ran two
real-SSH two-clone scenarios. A fixture SQL trigger requested yield on
`AfterFetch`; the service returned `Interrupted` after fetching A, with local
main still at the base. The interrupted variant retained that state. The
failed variant explicitly reacquired the reservation, reached `BeforeFetch`
and used the public `finish_remote_operation(ProtocolRejected)` transition;
SQL confirmed literal `failed` rather than merely a failed call. This is a
synthetic durable failure boundary, not a claim about a network failure.
In both variants the fixture rewound the tracking ref to the base and the
peer published a newer B whose commit was absent locally. A reopened service
with the same operation ID and `restart=true` opened **13 fresh Git helpers**,
downloaded B, advanced local main and tracking to B, and returned
`Complete(AlreadyCurrent { oid: B })`. No receiver update occurred during the
restart. No stale result was replayed, and no deletion was inferred.
Runner evidence: `0 13` (interrupted) and `1 13` (failed).

**3. Parked conflict resolution and tracking refs (D3) — PASS.**
`task0_conflict_tracking_movement` in `tests/remote_merge_recovery.rs` parked
a real canonical ticket conflict with recorded local/incoming parents L/R.
It captured the inspection token, then moved both primary and context tracking
refs to L. Inspection still returned the same observation token; the incoming
side still matched the blob at R. `resolve_all` made a local checkpoint with
ordered parents `[L, R]`, kept both tracking refs at L, and made zero new
authenticated connections and zero receiver updates. Resolution uses the
recorded integration parents and conflict evidence, not current tracking tips.
Source grounding: `resolve_synchronization`/recovery inspection in `sync.rs`
and the fixed HEAD/branch/packed-ref storage checks in `RefLogSnapshot`.
This finding does **not** repeal D3: the approved full poll still refuses before
transport during a pending conflict. Runner evidence: `1 1 1` for unchanged
token, frozen-parent resolution and no transport.

**4. Part-way git2 worktree checkout failure (D2) — PASS.**
`task0_partial_worktree_checkout` used real `git2::Repository::worktree` with
the same default `WorktreeAddOptions::reference` as local authoring. The tree
contained ordinary file `a` and symlink `z` with a 5,000-byte target, causing a
real Linux filesystem failure after `a` was written. `worktree` returned
`GenericError`; the path, `a`, worktree `.git` file, admin directory and worktree
registration all remained. `z` was not created. The admin directory contained
exactly `gitdir`, `HEAD`, `commondir` (no index); the branch remained at its
created OID. Observed output:
`path=true a=true z=false gitfile=true admin=true registered=true`.
A targeted recheck replaced the initial directory predicate for `z` with
`symlink_metadata(...).kind() == NotFound`, asserted the exact sorted admin
entry set, and opened the registered worktree to assert its original symbolic
branch and OID. All those explicit assertions passed.
This is a retained half-created worktree, not the branch-only resumable case.
No cleanup/adoption was attempted. This proves the Linux failure shape only,
not Windows path-length behavior or every possible checkout failure.

**5. Transfer-progress false return during paced SSH download — PASS.**
`task0_progress_abort` in `tests/ssh_fixture.rs` put a deterministic
incompressible 320,000-byte blob on remote main and paced outgoing fixture
chunks at 50 ms. It called real `Remote::download` with the exact forced
primary mapping, FETCH_HEAD disabled, tags off, prune off, a selected fixture
key and an exact host-key callback. `transfer_progress` returned false once
received bytes reached at least 16,384. The final measured run reached the
callback once at **65,931 bytes**, returned in **61 µs after false** and
**362 ms from download start**, with `GenericError` (enum ordinal 0). Assertions
proved: incomplete download; zero local refs; no FETCH_HEAD; advertised commit
absent locally; exactly one helper; zero receiver updates. No guarded tracking
write stage was entered. Runner tuple:
`1 65931 61 362 0 1 0` = callbacks, bytes, stop-to-return µs, elapsed ms, refs,
helpers, error-code ordinal. This is callback-to-return latency on this fixture,
not a universal 250-ms wall-clock cancellation bound. Existing 10-s TCP/30-s
blocking SSH limits remain; no total deadline was added.

### Characterization Commands And Honest Trial History

All used `devenv shell -- cargo`:

| Command suffix | Final result |
| --- | --- |
| `test --all-features --locked --lib task0_ -- --nocapture` | 2 passed, 504 filtered; 0.11 s |
| `test --all-features --locked --lib task0_partial_worktree_checkout -- --nocapture` | 1 passed, 504 filtered; 0.01 s; explicit absence/admin/HEAD recheck |
| `test --all-features --locked --test remote_synchronization task0_` | 1 SSH case passed; both interrupted/failed scenarios |
| `test --all-features --locked --test remote_merge_recovery task0_` | 1 SSH case passed |
| `test --all-features --locked --test ssh_fixture task0_` | 1 SSH case passed |

Five named tests executed, covering all five gate questions. Initial throwaway
compilation failed on a missing qualified `AuthoringKind` and an assumed
`WorktreeAddOptions::checkout_options` method that git2 0.20.4 does not expose.
The test was corrected to use the real filesystem checkout failure above.
The first three SSH cases passed but arbitrary stdout was filtered by the
runner, so numeric observations were added and the cases rerun. An extra
speculative assertion that callback abort returns `ErrorCode::User` failed at
line 143; investigation measured `GenericError` instead. That unsupported
assertion was removed; the actual abort/no-ref/latency assertions passed in
both the original and final run. The approved design does not require a
libgit2 `User` code, so this is not a design contradiction; later transport
code must not identify cancellation solely by that code.

One parallel Devenv invocation printed an enter-shell `load-exports` chmod
race although Cargo and its case passed. All three SSH commands were then run
sequentially; their final passing evidence had successful shell setup. No
production fix or design amendment was made for a test assumption or shell race.

All temporary tests and registrations, including the explicit worktree
recheck, were removed using patches. The post-removal
`git diff --exit-code -- src tests Cargo.toml Cargo.lock devenv.lock`
passed: the original Rust/tests/lockfiles are byte-for-byte
the Task 0 base. The final commit contains no characterization test.

### Baseline On The Original Rust Tree

| Required command | Result |
| --- | --- |
| `devenv shell -- cargo check --all-features --locked` | PASS; successful retry finished in 1m 35s |
| `devenv shell -- cargo fmt --check` | PASS |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | PASS; 1m 31s |
| `devenv shell -- cargo test --all-features --locked` | PASS; 1,568 passed, 0 failed, 1 existing ignored characterization |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | PASS; exit 0, no window; build 1m 35s |

Test breakdown: 504 library; 775 ordinary integration; 280 real-SSH cases
(comment publication 41, merge recovery 25, observation 15, synchronization 63,
SSH fixture 31, SSH transport 105); 9 compile-fail doctests. Both binary test
targets had zero tests. The existing ignored test is the thousand-item read
characterization. Existing sync/merge/comment suites passed without edits.

The initial check reached the 120-s tool timeout during compilation, then
passed with a longer tool limit. The initial full test command compiled in
6m 23s, passed library and preceding suites, then hit the 1,200-s **tool**
timeout part-way through merge recovery; it did not report a test failure.
The complete rerun used a 3,600-s tool limit and exited 0 through doctests.
These tool-limit changes do not change runtime transport limits.

Lockfile SHA-256 (unchanged):

- `Cargo.lock`: `8a20e8ba089f63bddfa9b87a08d5d4308404b4c2440c050453654e422a9549f8`.
- `devenv.lock`: `d4a78b8259525e7dcf66814477b0f96282e6570c9a21ebc84eda7d7fb8950d6e`.

### Checkpoint And Review Handoff

No gate contradiction or unresolved blocker was found. Task 0 is complete and
ready for independent spec review, then quality review; Tasks 1–8 were not
started. The ticket stays open and native evidence remains pending separate
authorization. The local checkpoint changes only this ledger, the ticket and
its Task 0 comment `01M4NV86AZMAVANZ2G8RJQM6FZ`. Its exact head SHA is reported
by the final handoff (`git rev-parse HEAD` after commit), avoiding a self-referential
commit identifier in its own contents.

`scripts/show-ticket 01K7F6H9J2N4Q6S8V0X2Z4B6DG` validated/rendered the open
ticket and all four canonical comments successfully. `git diff --check` passed;
status, full diff and `git log --oneline -10` were inspected before staging
only these three evidence files for the local checkpoint.

## Task 0 Independent Reviews And Owner Pause — 2026-10-11T20:01:51Z

The owner reported both independent reviews passed and instructed:
**"Pause after task 0 and before task 1"**. This checkpoint records review
bookkeeping only; Task 1 has not started and execution is paused until owner
direction to resume.

Both reviews inspected the full Task 0 range:

- Base: `094e5734d1d4480ee29e6f675aaba4a8883b9060`.
- Head: `11f7dbd6b99ee7f0a838c0faa972cb0c794c763f`.

| Review | Session ID | Scope and result reported by owner |
| --- | --- | --- |
| Independent spec | `ses_ed4470e17ffeiE6Bsc6av876RR` | Full range plus current source/ref/schema inspection; spec compliant, no blockers or findings requiring action |
| Independent quality | `ses_ed443ab4effebJy3h12aBe4kYO` | Full range plus relevant source and locked backend; hashes, counts and schema checked; **APPROVED**, zero Critical, Important or Minor issues |

Neither reviewer replayed the removed throwaway assertions or reproduced the
historical baseline. Both explicitly accepted the recorded evidence limitations.
The characterizations and baseline remain the implementer's historical Linux
evidence; review approval is not a new test run or native matrix proof. Native
evidence remains pending separate authorization.

Both Task 0 review checkboxes are ticked on the open ticket, which now states
Task 0 is completed, independently reviewed and paused before Task 1. Canonical
comment `01M4P8XCDN3ZBFEXBJ3W865M7N` records the review results and pause.
The bookkeeping base is `11f7dbd6b99ee7f0a838c0faa972cb0c794c763f`; only the
ticket, this ledger and that comment are included in the local checkpoint.
No Rust tree change or broad baseline repeat is needed for this bookkeeping.

## Task 1 Execution Resumed — 2026-10-13

The controller conveyed the owner's authorization to resume remaining execution
and assigned this fresh implementer **Task 1 only**, inline, no subagents, with
one local task commit and canonical bookkeeping, then return for independent spec
and quality review. The controller (not the owner) reported the approved Task 0
reviews and performed the resume fetch/rebase. Task 0's five characterizations
and historical baseline are trusted and were not replayed.

- All operations use the assigned ticket worktree and branch.
- Controller's fresh fetch of origin main and ticket rebase were a clean no-op:
  main `d1ca88772c5768d55aadb2ca4dcac1c69dab0d17`; before/after HEAD
  `4c1590d73b9dbab17030670989bd8825328f1f55` (also Task 1 base).
- Implementer confirmed that exact HEAD and clean branch status. No further
  fetch/rebase is required per task. D1–D4 and the approved design remain settled.
- Current AGENTS, Task 1 plan/design, ledger, ticket and latest comments read.
  The CLI has no ticket/comment commands; canonical files remain the supported
  bookkeeping path. No publication, native dispatch or lifecycle closure authority.

### Test-First Red Evidence (Before Production Behavior)

All Rust commands use Devenv, sequentially to avoid the previously recorded
shell-setup race:

| Command | Observed red result |
| --- | --- |
| `devenv shell -- cargo test --locked --lib remote::state` | exit 101; 42 passed, 3 failed, 463 filtered. Missing `latest_attempt_at` on migration/range queries; startup wrongly accepted a half-migrated table. |
| `devenv shell -- cargo test --locked --lib read::status` | exit 101; 9 passed, 1 failed, 498 filtered. Stored deadline expected `2026-01-01T00:00:00Z`, got `None`. |
| `devenv shell -- cargo test --locked --lib remote::poll` | exit 101; compile-red for missing eligibility types/functions/service method and snapshot accessors. Initial trial also omitted imports of existing policy types; corrected those test imports and reran: 45 missing-new-API errors, including the additional transaction-writer test. No runtime poll-test pass is claimed. |

The new tests precede production behavior. Poll module initially contains only
its test registration. Pure table tests cross all nine existing outcome
categories with automatic/explicit, including cancellation and capped retries;
transaction tests count writes, verify persisted countdown and policy precedence,
and exercise timestamp/outcome rollback. Busy/yield/interrupted/conflict/not-eligible
have no stored outcome category and must bypass the finishing writer in Task 2.

After recording compile-red, added API signatures with `todo!` bodies (no policy
behavior) so poll assertions could execute: the same `remote::poll` command
reported **0 passed, 8 failed, 509 filtered**, exit 101, at the eligibility,
next-deadline and persisted-clamp placeholders. This runtime red is recorded
before replacing those bodies with behavior. The export warning identifies the
necessary public wiring: `repository::remote` is private, so Task 1 also needs
two type re-exports in `src/repository.rs`, with no other change to that file.

### Implementation And Integration Boundary

- Fresh schemas and additive migration install both nullable integer UTC-second
  columns with checks for `0..=253402300799` (RFC 3339 through year 9999). Missing
  both upgrades to null; missing just one fails closed, transactionally. Snapshot
  and status readers validate stored times; startup audit continues to pass.
- Snapshot exposes both times as `Option<OffsetDateTime>`. The existing versioned
  status DTO's deadline field renders RFC 3339 UTC; no schema/other DTO change.
- Pure eligibility enforces Disabled → Paused → RecoverySuspended before due
  time. The service checks only the policy row inside `with_transaction`, before
  any endpoint/reservation work; its sole write is the bounded rollback clamp.
  Counted SQL writes prove one clamp, an unchanged second check, and eventual due.
- Pure `next_eligible` consumes **pre-finish** policy and returns a replacement
  deadline, or `None` to preserve it. Automatic network/protocol deadlines use
  60-second doubling capped at 900; attention failures use the interval; explicit
  failures and cancellation retain the stored deadline. Reservation finish still
  owns backoff writes. The transaction-only `record_poll_attempt` writer updates
  attempt time and optionally deadline alongside existing outcome/phase writes;
  atomicity/preservation is tested. Task 2 will wire it into finish and cancellation.
- No request/session debugging, endpoint, content, credential or path column is
  introduced. The existing schema/privacy scan now covers the polling columns.
- Two public type re-exports in `src/repository.rs` are necessary API wiring.
  No other behavior there was changed. All CLI/desktop/schema files, dependencies,
  lockfiles and synchronization/merge/comment suites are unchanged.

### Green Evidence And Lint Investigation

| Command | Result |
| --- | --- |
| `devenv shell -- cargo fmt` | PASS; formatted intended Rust files |
| `devenv shell -- cargo test --locked --lib remote::poll` | PASS; 8 passed, 0 failed, 509 filtered |
| `devenv shell -- cargo test --locked --lib remote::state` | PASS; 46 passed, 0 failed, 471 filtered |
| `devenv shell -- cargo test --locked --lib read::status` | PASS; 10 passed, 0 failed, 507 filtered |
| `devenv shell -- cargo check --all-features --locked` | PASS on final Rust tree |
| `devenv shell -- cargo fmt --check` | PASS on final Rust tree |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | PASS on final Rust tree |
| `devenv shell -- cargo test --all-features --locked` | PASS on final Rust tree; 1,581 passed, 0 failed, 1 existing ignored |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | PASS; exit 0, no window |
| `timeout --signal=TERM 90s devenv shell -- cargo run --locked --features desktop --bin manyhands` | Built in 1m 18s, reached `Running target/debug/manyhands`, no startup diagnostic; stopped by the smoke-test timeout. No visual window interaction claimed. |

The first check/fmt passed, then Clippy failed on a collapsible clamp `if` and
`large_enum_variant` in existing `RefreshOutcome`: adding the inline timestamps
grew its snapshot payload to 280 bytes versus 48 for the next variant. Traced the
growth to `RemoteSnapshot`; boxed only its private polling-policy storage, keeping
the accessor and public result shape intact, and collapsed the condition. Re-ran
check/fmt/Clippy successfully, then ran the full suite on this final Rust tree.
No lint suppression or public result-enum change was used.

Full-test breakdown: **517 library** (13 new Task 1 tests), **775 ordinary
integration**, **280 real-SSH** (comment publication 41, merge recovery 25,
observation 15, synchronization 63, SSH fixture 31, SSH transport 105), **9
compile-fail doctests**. Both binary test targets have zero tests; the one ignored
test remains the thousand-item read characterization. The untouched sync/merge/
comment suites passed. This is verification of the changed Task 1 tree, not a
replay of Task 0's throwaway characterizations or its original-tree baseline.
Display inspection found `DISPLAY=:1` and `WAYLAND_DISPLAY=wayland-1`, so the
bounded desktop launch was attempted. Native matrix remains unexecuted.

### Task 1 Review Handoff — 2026-10-13T14:51:10Z

Reviewed the production and test diffs against Task 1 and the settled result
table; no Task 1 blocker found. No later task implementation was started. Skill
review handoff is to the controller's independent reviewers, with no subagent
dispatch by this implementer. The ticket remains open, Task 1 is ticked, both
Task 1 review gates are pending. Canonical comment `01M4TVXXYRWN0MSPHSM83A0F8E`
records the resume, red/green evidence and review readiness.

Task 1 base: `4c1590d73b9dbab17030670989bd8825328f1f55`; the local commit head
and exact base-to-head range are reported after commit in the implementer handoff,
avoiding a self-referential SHA in committed content. Final status/diff/recent-log
inspection, `git diff --check` and `scripts/show-ticket` validation precede staging
only Task 1 source and canonical bookkeeping. No push, PR, native dispatch, merge,
closure or worktree cleanup is performed.

## Task 2 Execution — 2026-10-13

Assigned Task 2 only, inline with no subagents; approved design/plan and D1–D4
remain settled. The controller already refreshed/rebased at resume (no-op main
`d1ca88772c5768d55aadb2ca4dcac1c69dab0d17`); no repeated fetch/rebase.
Verified clean assigned worktree/branch and Task 2 base
`5ff9f4eee31cd88067a406ba8acbc317c0ee2045`. All Rust uses Devenv.

### Prior Task 1 Independent Review Evidence

Controller reports full Task 1 range
`4c1590d73b9dbab17030670989bd8825328f1f55..5ff9f4eee31cd88067a406ba8acbc317c0ee2045`:

- Spec `ses_eca3c077effeAOiqWMn8w5xeR5`: compliant.
- Quality `ses_eca397d08ffeZI2d2YhlbUJ99h`: approved; minor missing direct
  corrupt-timestamp read coverage carried into this task.

Focused hardening `devenv shell -- cargo test --locked --test remote_reservation
corrupt_poll_times`: 1 passed, 9 filtered. Both columns independently tested
with CHECK enforcement bypassed: negative, year-10000, text, real values;
snapshot and reopen return RecoveryRequired; deadline gate returns
RecoveryRequired and status returns the existing InternalError mapping.
This is already-green hardening, not new-behavior RED. Initial trial incorrectly
expected the deadline-only gate/status projection to read attempt time; it
returned Eligible. Source confirms the projection consumes only next_eligible_at;
corrected the test scope, with no production change.

### New-Behavior RED Before Implementation

- `devenv shell -- cargo test --locked --lib remote::reservation`: compile-red,
  10 missing new seam/variant errors. Refined ConflictPending into an internal
  full-poll reservation wrapper, keeping public observation/sync enums unchanged.
  Added behavior-free seam delegates to existing observe paths so assertions run.
  Rerun: exit 101, **30 passed, 7 failed, 487 filtered**. Missing poll_kind migration,
  D3 gate, kind-specific safe-point ordering/finish/restart and timestamps.
- `devenv shell -- cargo test --locked --test remote_reservation`: exit 101,
  **10 passed, 1 failed**: finish did not record latest_attempt_at.
- `devenv shell -- cargo test --locked --test local_authoring`: initial test typo
  used nonexistent comment_id helper; corrected to existing root_comment_id.
  Rerun exit 101, **131 passed, 1 failed**: automatic poll guard returned
  RecoveryRequired instead of RepositoryBusy. No production guard change yet.

New tests use real local Git repositories; no transport claim, fake transport,
sleep, global Git configuration, developer key, network or prompt is introduced.
Implementation begins only after this recorded RED checkpoint.

### Additional Policy-Boundary RED

After the initial local-stage implementation, strengthened the policy tests to
seed backoff/deadline **before** the advertisement batch rather than afterwards.
`devenv shell -- cargo test --locked --lib remote::reservation` then exited 101:
**35 passed, 2 failed, 487 filtered**. Existing observe `complete_batch` clears
backoff and writes Completed immediately; an update batch must retain policy
until the local stage finishes. This exposes a missing update-specific batch
fork, not a reason to change observation or synchronization behavior. The
following fix uses the existing policy-independent advertisement writer for
update batches, retaining existing observe complete_batch and sync handling.

### Implemented Boundary

- Probe-and-add nullable `poll_kind` with only observe/update allowed on poll
  rows and NULL (empty) on non-poll rows; migrate old poll rows to observe.
  Legacy NULL poll rows decode as observe. A separate immutable-kind trigger
  avoids rebuilding either CHECK list or changing the existing target trigger.
- Internal `reserve_remote_poll` and `RemotePollReservationOutcome` distinguish
  D3 ConflictPending from ordinary reservation outcomes. Existing public
  observation/synchronization reservation enums remain intact. Same-ID kind
  mismatch is OperationMismatch, including terminal replay and restart.
- Update-only safe-point fork permits repeatable BeforeLocalMutation after the
  committed batch, once-only BeforeDiscovery, both under local_prepared; the
  existing active index and journal guard already include that phase. Completed
  requires BeforeDiscovery. Observe remains AfterBatchCommit-complete and rejects
  BeforeDiscovery; its old BeforeLocalMutation/restart restrictions remain.
- Internal restart_remote_poll resets interrupted/abandoned update records at
  every stage to reserved, clears step/yield, increments owner epoch and fences
  the original service's old token. Terminal completed/cancelled records replay.
- Update batches persist the complete advertisement without writing a poll
  outcome/backoff. Finish consumes the pre-finish policy, then atomically writes
  outcome, backoff, attempt time, deadline and phase. Automatic network/protocol
  failures double the previous backoff; attention uses interval; explicit failure
  and cancellation preserve policy; Completed clears backoff and uses interval.
  Cancel acknowledgement writes attempt time with no deadline/backoff change.
- D3 rejects new update reservations before insertion, with no policy write;
  observe remains permitted. D4 uses the existing journal guard's sole active
  phase predicate: automatic poll requests yield, commits that request and
  returns RepositoryBusy before journal insertion; manual remains RecoveryRequired.
  Both kinds and every existing synchronization branch retain their contracts.

Additional already-green coverage checks the inclusive UTC endpoints and NULL,
the recovery journal guard directly at update local_prepared (save/context/comment
× both priorities), cancellation-writer rollback, actual AfterBatchCommit request
acknowledgement as well as the pre-batch transaction recheck. The guard tests
leave zero new local rows and retry successfully after acknowledgement. This
hardening does not claim a second new-behavior RED cycle.

### GREEN, Gate Failures And Final Verification — 2026-10-13T16:19:29Z

All commands below ran in the assigned worktree through Devenv, sequentially:

| Command | Result |
| --- | --- |
| `devenv shell -- cargo test --locked --lib remote::reservation` | Final focused PASS: 39 passed, 487 filtered; final full gate also passes all 39 |
| `devenv shell -- cargo test --locked --test remote_reservation` | PASS: 12 passed |
| `devenv shell -- cargo test --locked --test local_authoring` | PASS: 132 passed |
| `devenv shell -- cargo test --locked --lib remote::` | PASS: 299 passed, 225 filtered before the final batch-policy fork/hardening; final full gate covers the resulting 301 matching tests |
| `devenv shell -- cargo test --locked --test remote_observation` | PASS: 15 real-SSH cases, suite untouched |
| `devenv shell -- cargo test --locked --test read_status` | PASS: 23 passed after the deadline expectation correction |
| `devenv shell -- cargo check --all-features --locked` | PASS on final Rust tree |
| `devenv shell -- cargo fmt --check` | PASS on final Rust tree |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | PASS on final Rust tree |
| `devenv shell -- cargo test --all-features --locked` | PASS: 1,594 passed, 0 failed, 1 existing ignored |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | PASS: exit 0, no window |
| `timeout --signal=TERM 90s devenv shell -- cargo run --locked --features desktop --bin manyhands` | Built in 14.66 s, reached Running; bounded stop after 90 s, no startup diagnostic; no visual interaction claimed |

Full gate breakdown: **526 library** (+9 Task 2 tests), **779 ordinary
integration** (+3 reservation and +1 authoring), **280 real-SSH** (comment
publication 41, merge recovery 25, observation 15, synchronization 63, fixture
31, transport 105), **9 compile-fail doctests**. Both binaries have zero tests;
the existing thousand-item read characterization remains ignored. The final
full log is `tool_135f2723a001mSAMD0YzfOXEpI` in the harness output directory.

Honest trial history after first implementation:

- Reservation tests initially had 35 pass/2 failures because observation-restart
  equality assertions assumed no timestamps. Updated those assertions to prove
  all observation/context/policy/publication evidence stays identical while
  finish adds exactly attempt+interval. A trial used a nonexistent history_unknown
  accessor; corrected to the existing absent-item publication evidence accessor.
- The first integration run had 9 pass/2 failures: the old lifecycle guard test
  expected RecoveryRequired instead of approved D4 RepositoryBusy; updated its
  expected result and asserted durable yield. The generic restart refactor had
  also inadvertently changed non-poll restart errors to OperationMismatch;
  restored the original RecoveryRequired rule before the passing rerun.
- Direct journal test initially used unwrap_err on opaque RecoveryRecord, which
  has no Debug; used err/expect instead, without adding Debug to production.
- The first required full gate passed library/preceding targets (including 41
  comment SSH cases), then failed one tests/read_status assertion expecting None
  after an automatic network failure (22 pass/1 fail). The explicitly required
  finish timestamp now gives a deadline. **Minimal extra test-only file** beyond
  the Task 2 list: tests/read_status.rs now asserts attempt+60 in the existing DTO
  and that inserting an observation retains it. No status production/schema
  change. Focused status passed; final check/fmt/Clippy and full gate passed.
  This concrete failure justified the second full gate; no third broad repeat.

### Local Review Handoff

Task 2 implementation is ready for the controller's independent spec then quality
review. Self-reviewed the base-to-working-tree production/test diff against Task 2
and the settled result table. The Task 1 minor coverage finding is resolved.
No material design contradiction, unresolved local failure or later-task work.
Native matrix evidence and Task 2 independent approval remain open gates.

Task 2 base is `5ff9f4eee31cd88067a406ba8acbc317c0ee2045`; its exact head and
base-to-head range are returned after the single local task commit (a commit
cannot contain its own SHA). This entry and comments bind that range to the
`feat: give poll reservations a local stage+bookkeeping` checkpoint. Canonical
RED comment `01M4TX8PRJBBYH50CQ3GT5VYTP` and review-ready comment
`01M4V0ZN2MHG18CD1BFC2CYDWX` retain prior review and current verification evidence.
Ticket remains open; Task 2 is ticked with its two independent review gates open.
Final status/full diff/recent log, diff whitespace check and scripts/show-ticket
validation precede staging only intended Task 2 source/tests/bookkeeping.

`scripts/show-ticket 01K7F6H9J2N4Q6S8V0X2Z4B6DG` successfully validated/rendered
the open ticket and all eight canonical comments, including both Node-generated
Task 2 ULIDs. `git diff --check` passed; status, production/test/bookkeeping
diffs and recent `git log --oneline -10` were inspected. Only the seven intended
Rust/test files, this ledger, the open ticket and its two comments are staged.

## Task 3 Execution — 2026-10-13

Fresh implementer assigned Task 3 only, inline, no subagents, on the assigned
ticket worktree/branch. Confirmed clean HEAD at Task 3 base
`6cb69f4b6dd4f9798e509f9cdddc1acc7cd4f39a`. Controller's resume refresh/rebase
remains authoritative; no new fetch/rebase. Approved design and D1–D4 retained.

### Prior Task 2 Reviews

Controller reports range
`5ff9f4eee31cd88067a406ba8acbc317c0ee2045..6cb69f4b6dd4f9798e509f9cdddc1acc7cd4f39a`:

- Spec `ses_ec9ebcd99ffekVz7ijwoL63NJx`: compliant.
- Quality `ses_ec9e9909cffeS4kt0Z0ENmyjss`: approved; minor suggested direct-SQL
  poll-kind immutability regression remains explicitly deferred to final review.
- Task 1 minor direct corrupt timestamp read coverage was resolved by Task 2.
  Existing green evidence is not a new-behavior RED claim.

### Test-First RED — 2026-10-13T16:37:08Z

Before production behavior, sequential Devenv commands:

- `devenv shell -- cargo test --locked --lib transport`: compile-red for missing
  callback API and private combined scope; behavior-free signatures/placeholders
  then gave runtime RED: **28 passed, 2 failed, 498 filtered** at mapping/stop
  placeholders.
- `devenv shell -- cargo test --locked --test ssh_transport poll_`: compile-red
  for missing fetch API/private scope (also corrected missing existing
  PassphraseResponse import). With behavior-free fetch placeholder, runtime RED
  exits 101 on the first real-SSH case, `poll_fetch_set`. Harness stops at first
  failing child; individual filters `poll_absent_primary_and_empty_set`,
  `poll_tracking_race`, `poll_noncommit_tip` each exit 101 at the same missing
  fetch behavior. `poll_progress_stop` also exited 101, but later investigation
  identified a pre-transport setup failure; corrected behavioral RED is below.
  No transport assertions passed yet.

Tests register seven real in-process SSH cases, including policy compatibility,
and two unit tests. No new cfg(test) checkpoint hook or fake transport. The
source-inclusion policy driver constructs a frozen expectation from real public
registry/configuration state; races use the caller callback and public lease
probe. New behavior begins only after this checkpoint.

### Stop Fixture Correction And Behavioral RED

First GREEN trial: library 30 passed; SSH filter passed fetch-set, absent/empty,
tracking race and noncommit, then stop case failed before transport. Investigation
found its fixture helper tried a parentless `commit(Some(main))` over existing
main; libgit2 refuses that inconsistent parent update. Corrected the test helper
to create the object without an update, then set the fixture ref. The earlier
stop RED was a setup failure and is **not** stop-behavior evidence. Removed the
progress registration before continuing; corrected
`devenv shell -- cargo test --locked --test ssh_transport poll_progress_stop`
exited 101 at poll.rs line 307 (`unwrap_err` after a successful full download).
This is behavioral RED for the missing registration; registration is implemented
only after this corrected checkpoint. Unit stop-intent RED remains valid.

Additional exact-set scope test preceded the extracted predicate:
`devenv shell -- cargo test --locked --lib transport` exited 101 with
**30 passed, 1 failed, 498 filtered**, at its behavior-free scope placeholder.
It requires the complete advertised parsed set and rejects duplicates, omitted
members, wildcards, malformed/unadvertised sources, local destinations and Push.
Two added SSH hardening cases (download-connection advertisement, partial guarded
write failure) and stronger excluded graph/key-ID assertions were already green;
they are not presented as new-behavior RED.

### Implemented Task 3 Boundary

- `fetch_poll_set` uses the shared `TransferAction::FetchTracking` path. Its
  renewed selected-key connection lists the advertisement, derives explicit
  forced mappings for the advertised primary and parsed document/ticket names,
  and captures the old destinations before download. Malformed names remain
  in the returned advertisement but are not downloaded; other families are
  ignored. The original download-connection advertisement is returned.
- Empty parsed sets return before `download`; no configured/default refspec
  fallback. FETCH_HEAD updates, tag following and pruning are disabled. D1
  retains previously fetched tracking refs even when sources disappear.
- Under the existing short repository lease, the complete parsed mapping set,
  configuration, endpoint, selected key and host policy are rechecked; every
  fetched tip must be a commit and every old destination must still match
  before the first write. Existing guarded `reference_matching`/nonforcing
  creation performs the writes. A later write failure returns failure while
  retaining the valid tracking observation already written, never completion.
- Progress registration belongs only to the poll action. Its explicit sticky
  closure-stop flag prevents writes after false independently of libgit2's
  GenericError/User code; result stays the existing redacted TransportUnavailable
  category so the later orchestrator must consume its caller-owned stop flag.
  Transport neither knows reservations nor throttles queries. Task 6 must supply
  the short read-only flags query, <=250-ms caller throttle and acknowledgement.
- The existing combined automatic+frozen-expectation scope is visible to the
  repository's remote module; its policy behavior is unchanged. Automatic blocked
  unlock and endpoint/key/source mismatch cases are compatibility evidence,
  not a new policy-behavior RED claim.
- Source-inclusion fixture driver adds no checkpoint hook. Callback races use
  real local refs or fixture ref locks; stop uses paced real SSH and the public
  repository lease probe. Privacy scans run after transport/leases have ended.
  No frontend, dependency, schema or later-task implementation changed.

### Acceptance Evidence

| Task 3 requirement | Passing real-SSH case |
| --- | --- |
| Primary + document + ticket exact tips; malformed/outside commit/tree/blob objects absent; tags, FETCH_HEAD, unrelated/local refs preserved; tracking-only ancestry rewind | `poll_fetch_set` |
| Absent primary leaves its tracking unchanged; empty parsed set makes no download callback, downloads no outside graph, retains absent context tracking (D1) | `poll_absent_primary_and_empty_set` |
| Existing or initially absent destination changed after capture: fails, preserves changed destination and primary still old; all checked before first write | `poll_tracking_race` |
| Downloaded blob tip is present but rejected before any tracking write | `poll_noncommit_tip` |
| Paced stop exits without refs/FETCH_HEAD/commit, precisely two helpers (scope+download), no further connection; callback can acquire public lease | `poll_progress_stop` |
| Frozen endpoint, missing/replaced key selection, changed key source: zero helpers/prompts before refusal | `poll_frozen_policy` |
| Blocked automatic unlock: no second prompt, zero Git helpers | `poll_automatic_blocked_unlock` |
| New context added after earlier scope advertisement: returned advertisement and tracking belong to download connection | `poll_download_advertisement` |
| Second tracking ref lock fails guarded write; first valid tracking tip retained, context/local ref preserved, no completed claim | `poll_partial_guarded_write` |

Every case (and every loop iteration) waits for real helper audits, asserts zero
receive-pack commands/advertisements and zero receiver updates, then scans the
registry for password/endpoint probes and registers them with the child-output
privacy harness. All nine cases use the real in-process SSH fixture. Three new
library tests prove parsed mapping selection, complete exact-set validation and
sticky stop intent; library tests make no SSH transfer claim.

### GREEN And Full Verification — 2026-10-13T17:21:49Z

All Rust commands ran sequentially through Devenv in the assigned worktree:

| Command | Result |
| --- | --- |
| `devenv shell -- cargo fmt` | PASS |
| `devenv shell -- cargo test --locked --lib transport` | Final focused PASS: 31 passed, 498 filtered |
| `devenv shell -- cargo test --locked --test ssh_transport poll_` | PASS: 9 real-SSH cases after fixture correction/hardening; final entire target/full gate also cover all nine after exact-set predicate wiring |
| `devenv shell -- cargo test --locked --test ssh_transport` | PASS on final Rust tree: 114 real-SSH cases, including all 105 untouched existing cases |
| `devenv shell -- cargo check --all-features --locked` | PASS |
| `devenv shell -- cargo fmt --check` | PASS |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | PASS |
| `devenv shell -- cargo test --all-features --locked` | PASS: 1,606 passed, 0 failed, 1 existing ignored |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | PASS: exit 0, no window |
| `timeout --signal=TERM 90s devenv shell -- cargo run --locked --features desktop --bin manyhands` | Built in 8.03 s, reached Running, no startup diagnostic; bounded timeout stop, no visual interaction claimed |

Full gate breakdown: **529 library** (+3 Task 3), **779 ordinary integration**,
**289 real-SSH** (comment publication 41, merge recovery 25, observation 15,
synchronization 63, fixture 31, transport 114), **9 compile-fail doctests**.
Both binary test targets have zero tests; the existing thousand-item read
characterization remains ignored. Full log: `tool_1362dc32a001ZRHM1I40DZfKCY`.
Existing synchronization, observation, merge and comment cases passed unchanged.
No broad gate failure or duplicate broad rerun. Initial compile trials, corrected
fixture failure and corrected behavioral RED are recorded above, not concealed.

Unchanged lock hashes:

- Cargo.lock: `8a20e8ba089f63bddfa9b87a08d5d4308404b4c2440c050453654e422a9549f8`.
- devenv.lock: `d4a78b8259525e7dcf66814477b0f96282e6570c9a21ebc84eda7d7fb8950d6e`.

### Review Handoff And Deferred Finding

Self-reviewed the complete Task 3 base-to-working-tree diff against the approved
Task 3 plan and transport constraints. Task 3 is ready for the controller's
independent spec then quality review, with no implementer subagent dispatch.
Base: `6cb69f4b6dd4f9798e509f9cdddc1acc7cd4f39a`; exact local commit head/range
will be returned after committing `feat: fetch the poll ref set with guarded
tracking refs+bookkeeping`, avoiding a self-referential SHA in committed content.

**Deferred minor still open:** Task 2 quality review
`ses_ec9e9909cffeS4kt0Z0ENmyjss` suggested a direct-SQL kind-immutability
regression. Carry explicitly into the final Cycle review ledger; Task 3 did not
add it or change poll-kind production behavior. Task 1 corrupt-time minor remains
resolved. Native matrix and independent Task 3 review are remaining gates.
Linux paced stop evidence is not a universal callback-latency guarantee; existing
10-s TCP/30-s blocking-SSH limits remain, without a total deadline. D2 Linux
partial-worktree characterization remains authoritative and untouched.

Canonical comments `01M4V1ZZM9SAQHCJFT8DFA7SHJ` (RED/prior reviews) and
`01M4V4HS2G64D9TR1P71N6CQB8` (review-ready/corrected RED/GREEN) record checkpoints.
Ticket remains open; Task 4 was not started. Final status/full diff/recent-log,
`git diff --check` and `scripts/show-ticket` validation precede staging only the
seven owned Rust/test files, this ledger, the ticket and its two new comments.

`scripts/show-ticket 01K7F6H9J2N4Q6S8V0X2Z4B6DG` successfully validated/rendered
the open ticket and all ten canonical comments, including both new Node ULIDs.
Final diff/recent log were inspected and `git diff --check` passed.

## Task 3 Independent Reviews And Atomic-Absence Correction — 2026-10-13

Controller reports review of Task 3 range
`6cb69f4b6dd4f9798e509f9cdddc1acc7cd4f39a..dc96f0b71fad8bba911f349bbf53140864b81d1b`:

- Spec `ses_ec9b35ee6ffeBdb0HkkfCCp9R4`: passed.
- Quality `ses_ec9b0a1d9ffeWP162PBxjUK6l4`: **blocked, Important**. Initially
  absent tracking destinations use `reference(..., false, ...)`, which has a
  check-to-lock race against external Git. Fix before Task 4; new local commit,
  no amendment. No guard-contract or design amendment is needed.

Resumed inside the same assigned worktree/branch, clean fix base
`dc96f0b71fad8bba911f349bbf53140864b81d1b`; no fresh fetch/rebase. Loaded receiving
code review and applied the existing systematic-debugging/TDD instructions.

### Locked Backend Investigation

Verified the actual Cargo registry source, `git2 0.20.4` and
`libgit2-sys 0.18.8+1.9.7`, rather than assuming force=false is atomic:

- git2 `repo.rs:1509–1517` calls `git_reference_create`; libgit2 `refs.c:487–495`
  delegates with NULL `old_id`. `refdb_fs.c:1112–1125` checks existence before
  `refdb_fs_backend__write` takes `loose_lock` at 1613. `cmp_old_ref:1521–1523`
  skips comparison for NULL. A Git creator after that check can be replaced.
- `reference_matching` passes a non-NULL expected OID through
  `git_reference_create_matching` unchanged (`repo.rs:1564–1571`,
  `refs.c:480–481`). The filesystem backend takes the ref lock before
  `write_tail:1634` calls `cmp_old_ref`. That comparator accepts lookup NotFound
  only when the supplied expected OID is zero (`1525–1527`); existing direct
  values or symbolic types differ and return Modified without committing.
- The force bit must be true to bypass the non-atomic preliminary existence
  check, **not** to bypass the expected-value comparison under the ref lock.
  The existing Some(old) path already uses this bit with a nonzero expectation.
  A zero expectation therefore represents atomic absence for valid Git refs.

### Focused RED Before Guard Behavior — 2026-10-13T17:37:20Z

Extracted the existing match unchanged into `write_tracking_tip`; caller still
validates every commit and destination before any write. Two real local-ref
primitive tests use the public git2 Transaction lock and external Git with system
and global configuration excluded, no sleeps, test hooks or simulated transport.

`devenv shell -- cargo test --locked --lib transport_absent_tracking_guard`
exited 101: **0 passed, 2 failed, 529 filtered**. The lock test captures absence,
creates a competing ref using external `git update-ref`, holds its public ref
lock, then invokes the unchanged primitive. Expected Locked, got Exists. The
other test expected under-lock Modified after competing creation, got Exists.
These errors distinguish the best-effort pre-lock check from the required locked
CAS path. The test does not claim to pause inside C between instructions; the
locked source analysis plus deterministic lock/compare-path assertions prove the
chosen atomic primitive. Competing OID/reflog preservation, release/retry, real
absence success and direct/same-OID/packed/symbolic variants are also asserted.
The correction starts only after this recorded RED.

### Atomic-Absence Fix And GREEN — 2026-10-13T18:13:46Z

The shared local writer now always uses `reference_matching`, retaining the
captured OID for an existing destination and supplying `Oid::zero()` for captured
absence. `force=true` skips only the preliminary best-effort existence check;
the locked backend still compares the supplied expectation under its ref lock.
No unguarded write is introduced. The all-tips commit/old-value validation loop
stays before the first write; later guarded failures still retain valid earlier
tracking observations and return ProtocolFailure. Existing Some(old) behavior
and all mapping/observation/stop code are unchanged.

Sequential Devenv evidence:

| Command | Result |
| --- | --- |
| `devenv shell -- cargo fmt` | PASS |
| `devenv shell -- cargo test --locked --lib transport_absent_tracking_guard` | GREEN: 2 passed, 529 filtered (after RED 0 passed/2 failed) |
| `devenv shell -- cargo test --locked --lib transport` | PASS: 33 passed, 498 filtered |
| `devenv shell -- cargo test --locked --test ssh_transport poll_` | PASS: 9 real-SSH cases, including initial-absence race, validate-all-before-first-write and permitted partial failure |
| `devenv shell -- cargo test --locked --test ssh_transport exact_` | PASS: 7 real-SSH cases, shared-writer compatibility |
| `devenv shell -- cargo check --all-features --locked` | PASS |
| `devenv shell -- cargo fmt --check` | PASS |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | PASS after test-only range-pattern correction |
| `devenv shell -- cargo test --all-features --locked` | PASS: 1,608 passed, 0 failed, 1 existing ignored |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | PASS: exit 0, no window |
| `timeout --signal=TERM 90s devenv shell -- cargo run --locked --features desktop --bin manyhands` | Built in 6.48 s, reached Running, no startup diagnostic, bounded timeout stop; no visual interaction claim |

The first Clippy gate flagged the new test's `0 | 1 | 2` pattern; changed only
that pattern to `0..=2`, then check/fmt/Clippy passed and the full gate ran once.
No production trial failure after the recorded RED. No duplicate broad test run.
Full log: `tool_1365d606d001URFdZV4VOOZeUU`. Counts: **531 library** (+2 fix
regressions), **779 ordinary integration**, **289 real-SSH** (41 comment, 25
merge, 15 observation, 63 synchronization, 31 fixture, 114 transport), **9
compile-fail doctests**. Both binary targets have zero tests; the existing
thousand-item characterization remains ignored. All existing SSH cases passed
without edits. Real transport assertions remain in real-SSH targets; new unit
tests claim only local reference primitive/lock behavior and backend semantics.

Lock hashes still match the previous checkpoint: Cargo.lock
`8a20e8ba089f63bddfa9b87a08d5d4308404b4c2440c050453654e422a9549f8`, devenv.lock
`d4a78b8259525e7dcf66814477b0f96282e6570c9a21ebc84eda7d7fb8950d6e`.

Self-reviewed the base-to-fix diff against the Important finding and source
semantics. The blocker is technically corrected with RED/GREEN evidence;
independent quality re-review remains pending and is not implied by this entry.
New local commit `fix: guard absent tracking refs atomically` from fix base
`dc96f0b71fad8bba911f349bbf53140864b81d1b`; its exact head/range is returned after
commit. Full Task 3 re-review starts at `6cb69f4b6dd4f9798e509f9cdddc1acc7cd4f39a`.
Only remote.rs and canonical ledger/ticket/two fix comments change.
Comments `01M4V5E6NFNSBJQM0FDDG8DVRK` and `01M4V7GXPAC24SSPC0WXW6BAEF` record
review, investigation, RED and GREEN. Task 2's direct-SQL kind-immutability minor
remains explicitly deferred; Task 1's corrupt-time minor remains resolved.
Native matrix remains pending. Ticket stays open and Task 4 remains unstarted.

Final `git diff --check` passed; status/full diff/recent log inspected before
staging only those five intended files. `scripts/show-ticket
01K7F6H9J2N4Q6S8V0X2Z4B6DG` successfully validated/rendered the open ticket and
all twelve canonical comments, including both Node-generated fix ULIDs.
