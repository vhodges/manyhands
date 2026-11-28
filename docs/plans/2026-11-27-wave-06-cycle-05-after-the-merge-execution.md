# Wave 06 Cycle 05 Execution Ledger

Ticket vh-ssepw (`01M876851FT3NPSH9KDXKXR1AJ`). Plan: `docs/plans/2026-11-27-wave-06-cycle-05-after-the-merge-implementation.md` (managed document `01M8G1CY7XE733CHN0XEF52J78`, approved 2026-11-28 at revision 4, `e2189ba6`). Design: managed document `01M8G1CXKNG56HZ4NFMKNAE3G7` (`1b659f7b`). Cycle document: managed document `01M8G1CX05SK9MY5D9YP67B0RP` (`d057897b`). The three documents live on their own branches until they are promoted at closure (design D-6); this ledger stays on the ticket branch (Cycle document decision 6). Execution: subagent-driven, implementers on Sonnet 5.5 at high effort, one fresh reviewer per task; Task 7's design checkpoint and the final review on the most capable model (Opus 5.5). One line per paragraph.

## Approval

- The owner approved the three documents at revision 4 without a further round, the named test edits E-1 to E-23, subagent-driven execution with Sonnet 5.5 at high effort and the most capable model for Task 7's checkpoint and the final review, the orchestrator approving Task 7's checkpoint and posting it on the ticket before any code, and implementation (Cycle document decision 15), on 2026-11-28. The documents were saved with status approved and published (Cycle document `d057897b`, design `1b659f7b`, plan `e2189ba6`).
- Planning checkpoint comment `01M8H438E8DD48TZHJH0A5ZME5` (request `01M8H4390B6NSB6NHDS4KMY0T5`), published at `4c9586d1`. Approval comment `01M8H5FWV0AD96G2B9YFCMABVP` (request `01M8H5FXDEPJF9T08SPC5Q3M6E`), published at `068a906b`.
- Notes left at approval, each published: vh-p7st2 (`01M8H5G8DK0KHH501GBDNHM08K`), vh-ew8pt (`01M8H5GNE4YZ0FS23ZW86B6JVN`), vh-tmghg (`01M8H5H0G1ZX0GN050Z8EWNX17`). Each ticket's worktree was clean and at its remote tip before the comment.

## Task 0

- Step 1: `git fetch origin`; `origin/main` is `15ea8eb5`, unchanged since planning. HEAD before and after `git merge origin/main`: `068a906b` (already up to date; main was merged at `0213d32d` during planning, never rebased). `git merge-base --is-ancestor origin/main HEAD` holds; the worktree is clean.
- Step 2: `git diff --stat 0213d32d HEAD -- src tests schemas skills .agents docs/RFC Cargo.toml Cargo.lock` is empty: since the documents' baseline only ticket and comment files changed, so every name and line of design Baseline Facts 1 to 27 stands as the three planning rounds verified it at `0213d32d` (the round-3 scoped re-review re-checked Facts 1, 4, 6, 7 and 11 to 27 and the plan's module paths). Spot-checked again here: `artifacts::fast_forward` 178, `poll_fast_forward_unlocked` 616, `completed_deleted_context` 279, `decode_frozen` 1227, `reconcile_pending_candidate` 3255, `integrate_divergence` 3668, `synchronize_remote_observed_inner` 6343, `apply_resolution_ref` 1948, `apply_for` 452, `activate` 504, `precheck_context` 189, `retire_context` 953, `delete_tracking_ref` 572, `fill_complete_read` 1535, `same_indexed_metadata` 1853, `ClosureDto` 617, `PollMutationDto` 2810, `migrate_poll_dispositions` 571, `reserve_remote_operation` 1518, `finish_remote_operation` 2250, `compare_local_proof` 927, `inspect_selected` 420, `migrate_pull_request_links` 2966, `tickets_made_ready` 1044, `made_ready` 881; `bridges/abandon.rs`, `remote_polling_state.known_remote_name` and `known_primary_branch`, and `lifecycle_record.rs` `supersede` (431) exist.
- Step 3, module paths (confirmed): `repository::discovery::tests` (inline, discovery.rs 3182); `repository::remote::poll::tests` (poll_tests.rs); `repository::remote::sync::tests` (sync_tests.rs); `repository::remote::state::persistence_tests` (state.rs 1928); `repository::setup::poll::tests` (setup/poll_tests.rs); `repository::closure::record::tests` and `repository::promotion::record::tests`; `repository::closure::run_tests` and `repository::promotion::run_tests`, with the lifecycle tests at `…::run_tests::pull_request::lifecycles`; `mutation/dto.rs` has named inline modules (`poll_retirement_recognition_tests` 3428, `primary_local_commits_tests` 3492, `pull_request_closure_tests` 3555), so its new tests sit in a named module of the same kind; the CLI's `--bin manyhands-cli` modules are `human::tests` and `polling::tests`; the SSH case is `--test mutation_remote c5_lifecycle::a_pull_request_close_publishes_only_the_ticket_branch`. The new `repository::item_merges::tests` is created in Task 1.
- Step 6: `.github/workflows/build.yml` lists the existing targets; this Cycle adds no test target, so no CI list changes.
- Steps 4 and 5, the premise sweep (a read-only research agent; verbatim in Appendix A): the only exhaustive matches without a wildcard in tests are the `MutationDataDto` helpers of `tests/support/mutation.rs` and `tests/support/repair.rs` (E-19); the `ClosureDto` and `PollMutationDto` literals are as design E-5 lists them, except that `mutation/dto.rs` holds no `PollMutationDto` literal; 8 test `ItemDto` literals name all 23 fields and need `merged_at: None`, 4 use `..` (U-7 settled). The poll goldens are written over SSH (`tests/cli_remote/polling.rs`, `tests/mutation_remote/c5_poll.rs`), except `mutation_v1/poll_retirement`, written by the fast-tier `tests/mutation_contract.rs` `an_opted_poll_matches_its_distinct_closed_schema_and_golden`; every item golden is written by a fast-tier test (`tests/read_contract.rs`, `tests/cli/content.rs`, `tests/cli/relationships.rs`). E-6: only the `unmatched` row of `retirement_recognition_committed_primary_and_independent_ticket_completion` changes, `Closed` to `Unheld`. E-7: no Wave 06 lifecycle row changes. E-21: no test pins a closed ticket's link (the closed section of `ticket_show_gives_the_link_and_the_publication` has no link row), so Part B's rule is a new assertion and no edit. `c5_polling_remote_context_of_a_closed_ticket` and its `poll_once_closed_ticket` golden pin decision 28's `current` for a target with no local context, expected unchanged. No test of `bridges/abandon.rs`, `bind/abandon.rs` or `reservation_tests.rs` pins a lifecycle record as not abandonable; the lifecycle listing pins are `d20_operations_list_a_branch_record_as_a_close_and_its_child_as_its_own_row`, `holds_cancellation_and_listing` and `c4_resume_cancelled_with_work_owed` (E-17's reach).
- Named edits found by the sweep, each additive, under the owner's test-edit approval, none reversing an intent:
  - E-24 (Task 5) `desktop/services/poll_convert_tests.rs` `check_now_attempt_reads_the_report` (about 401): its table gains the two dispositions, since it asserts `Dto::ALL.len()` rows.
  - E-25 (Task 5) `desktop/model/polling/entry_tests.rs` `dispositions_listed` and `desktop/model/polling_schedule_tests.rs` `attempt_summary_counts`: extended only where the desktop's mapping (R-14) gives the two values a row.
  - E-26 (Task 12) `src/results_tests.rs` `problem_codes_have_unique_snake_case_strings_and_fixed_guidance` (about 1053): its name list gains `primary_not_checked_out` and `primary_dirty` at their `ALL` positions.
  - E-27 (Task 7) `closure/record_tests.rs` `ALL_STEPS` (about 342) with `steps_permit_only_designed_transitions` and `closure_step_round_trips_through_parent_view`, and `promotion/record_tests.rs` `ALL_STEPS` (about 1559) with `pull_request_promotion_steps_and_checks`: gain `remote_deleting` and the cleanup action's graph.
  - E-28 (Task 7) the raw `INSERT`s into `ticket_closures` and `document_promotions` with explicit column lists (`closure/record_tests.rs`, `bind/lifecycle_tests.rs`, `setup/change_tests.rs`, `promotion/record_tests.rs`, `tests/read_status.rs`, `local_sync/record_tests.rs`; Appendix A lists the lines): unchanged when every new column has a default; Task 7 gives them defaults or edits each insert.
  - E-29 (Task 5) `closure/run_tests.rs` `a_poll_does_not_retire_a_context_a_direct_closure_holds` and `closure/run_pull_request_tests.rs` `a_poll_does_not_retire_a_context_a_branch_record_holds`: assert `closed` for a branch deleted with the closure merged; expected unchanged under the new proof, verified by running each.
  - E-30 (Task 8) `mutation/command.rs` `every_command_has_the_setup_gate_of_the_design` (about 1182) and its neighbours that loop `Command::ALL` (about 804, 860, 966), and `mutation/outcome_tests.rs`'s `Command::ALL` loops: gain `ContextCleanup`'s rows.
  - E-31 (Task 4) `remote/state_tests.rs` catalog tests beyond E-14: `task6_disposition_schema_upgrade_rollback_corruption_and_privacy`, `task6_disposition_schema_is_empty_atomic_metadata_only`, `task6_partial_disposition_schema_is_not_recreated`, `review_disposition_envelope_rejects_lost_column_pair_without_recreation`, `review_disposition_envelope_rejects_lost_object_pair_without_recreation`, `review_disposition_metadata_loss_is_not_hidden_by_updated_at_fallback` and `review_disposition_every_recognizable_partial_envelope_is_retained`, through `historical_disposition_schema` and `install_historical_dispositions`.
  - E-32 (Tasks 8 and 14) `tests/cli/recovery_tables.rs` (about 77, 130) and `tests/cli/skills.rs` (about 446) list `ticket withdraw-closure` beside the lifecycle verbs; covered by E-12 and E-13.

## Pre-flight scan (before Task 1)

| Rows | Produces / consumes | Found |
| --- | --- | --- |
| Task 1 against Task 2 | Task 1 produces `record_after_move` and the poll's call; Task 2 calls it at the other sites | Consistent |
| Task 1 against Tasks 3 and 6 | `merge_of` (Task 3) and `merges_by` (Task 6) are added with their first caller, not in Task 1 | Consistent with the plan's Intermediate commits rule |
| Task 3 against Task 6 | E-5's literals: Task 3 the `ClosureDto` and `ItemDto` ones, Task 6 the `PollMutationDto` ones | Consistent; `dto.rs` has no `PollMutationDto` literal (sweep) |
| Task 4 against Task 5 | Task 4 widens the CHECK catalog before Task 5 writes the values | Consistent: the catalog accepts values nothing writes yet |
| Task 5 against Task 6 | `poll_mutation.schema.json`: Task 5 adds the enum values, Task 6 the `merged` field | Consistent, sequential |
| Task 6 against Task 11 | Both add tests in the lifecycle test modules (`…::run_tests::pull_request::lifecycles`) | Consistent; Task 11 reuses Task 6's fixtures |
| Task 6 against Task 13 | Task 6 rewrites the existing poll goldens; Task 13 writes the new `poll_once_merged` | Consistent |
| Task 7 against Tasks 8 and 9 | Task 7 rebuilds the tables once; Tasks 8 and 9 read the new columns | Finding: a column's Rust field or accessor with no reader until Task 8 or 9 fails `clippy -D warnings` in `closure/record.rs`, which has no `dead_code` allowance. Ruling P-1 |
| Task 8 against Task 12 | CLI renderers: Task 8 the cleanup's, Task 6 and Task 12 the poll's lines in `polling.rs` | Consistent, different functions |
| Task 8 against Task 14 | Task 8 writes the taxonomy row and the `autonomy.md` cell with the verb; Task 14 the rest of D-1 and D-4 | Consistent |
| Task 9 against Task 10 | Task 9's stop mapping and fence; Task 10's abandon reads the record's step and the row's phase and epoch | Consistent; Task 10 depends on Task 9's steps |
| Each task's own text | Tests named against code named; files created against files touched later | Consistent, except E-21 (Ruling P-2) |

- Ruling P-1: Task 7 rebuilds each table once with every column the design names, and adds a column's Rust field or accessor in the task that first reads it (Task 7 for what its own validation and migration read, Tasks 8 and 9 for the rest); no lint allowance is added — the plan's Intermediate commits rule and `clippy -D warnings` — Task 8 or 9 then adds a field that maps an existing column.
- Ruling P-2: E-21 is a new assertion in Task 3's `a_closed_ticket_s_link_is_no_longer_offered`, and `ticket_show_gives_the_link_and_the_publication` is unchanged — no test pins a closed ticket's link today (sweep) — none.
- Ruling P-3: the desktop's `Disposition` maps `Merged` and `Unheld` as it maps `Current` (design R-14), so E-24's table gains two rows and E-25's lists change only where that mapping adds a row — Cycle 06 designs their display — none.
- Ruling P-4: E-24 to E-32 are adopted as named edits, each additive, under the owner's test-edit approval (decision 15) — none reverses an intent — none.

## Task 0 status

Task 0 complete: no name changed shape, no edit reverses an intent, no CI list changes. Execution starts with Task 1.

## Appendix A: Premise sweep, Task 0 Steps 4 and 5 (research agent, verbatim)

# Task 0 sweep (read-only)

Tree: ticket worktree 01M876851FT3NPSH9KDXKXR1AJ at 068a906b. "T" means a test file or an inline `#[cfg(test)]` module. Hit counts are total (production/test).

## 1. Sweep hits

| Term | Hits | Test files that need an edit |
| --- | --- | --- |
| `PollDisposition::` | 324 (118/206) | (a) none exhaustive without `_`. `tests/remote_polling.rs:725` `guidance()` has `_ => panic!`. Table pins in 3 files (see E-24, E-25). |
| `PollTargetDisposition::` | 42 (20/22) | (a) none exhaustive. `ALL`-derived: `tests/read_contract.rs` 1099/1615 (E-8), `bin/manyhands-cli/polling_tests.rs:153` `every_disposition_prints_the_domain_s_guidance` (needs only the new `guidance()`). |
| `ClosureDto {` | 32 raw; literals 3 prod + 11 T | (b) see below. |
| `ItemDto {` | 58 raw (mostly signatures); see section 3 | (b) see section 3. |
| `PollMutationDto {` | 8 raw; literals 1 prod + 5 T | (b) see below. |
| `MutationDataDto::` | 477 (116/361) | (a) exhaustive, no wildcard: `tests/support/mutation.rs` `ticket_data` 672, `document_data` 710, `remote_data` 852, `operation_data` 890, `key_data` 928, `identity_data` 966; `tests/support/repair.rs` `repair_data` 333. All src tests use `_`/`other`. |
| `PreviewDetailsDto::` | 108 (56/52) | (a) none exhaustive in T (all `let ... else`, `matches!` or `_ => unreachable!`). |
| `ProblemCode::` | 301 (68/233) | (a) none exhaustive. Pinned lists: see E-26. |
| `LifecycleBlocker::` | 197 (66/131) | (a) none exhaustive. Only `read_contract.rs` `contract_enumerations` (E-8) reads `ALL`. |
| `ClosureAction::` | 147 (61/86) | (a) none exhaustive. See E-27. |
| `PromotionStep::` | 143 (23/120) | (a) none exhaustive. Fixed arrays: E-27. |
| `Step::` | 1101 (505/596) | mostly unrelated local `Step` enums (`bind/poll_tests.rs`, `repo_tests.rs`, `sync_tests.rs`, `polling_schedule_tests.rs`, `terminal_tests.rs`). The closure/promotion ones are `ClosureStep::`, `PromotionStep::` and `ParentStep::` (E-16, E-27). |
| `RecordPath::` | 67 (41/26) | Section 5. |
| `Command::TicketWithdrawClosure` | 29 (16/13) | `bind/lifecycle_tests.rs:2243` `a_withdraw_is_a_lifecycle_parent_wherever_its_command_is_read` (its pattern, an extension); `mutation/replay_tests.rs:53`; `mutation/setup_gate_tests.rs:151`. |
| `abandonable` | 29 (17/12) | Section 6. |
| `LIFECYCLE_COMMANDS` | 2 (2/0) | production only. |
| `remote_poll_dispositions` | 79 (20/59) | `state_tests.rs` (45), `poll_tests.rs` (5), `tests/remote_polling.rs` (7: 692, 2711, 2717, 2822, 2826, 3000, 3388, probes and fault injections), `recovery_foundation_gate.rs:1141`, `repository_enablement.rs:631`. |
| `registry_migrations` | 194 (79/115) | The marker pins are E-14, E-15, E-16. `comment_publication_tests.rs:12-43` counts rows only relatively; unaffected. |
| `ticket_closures` / `document_promotions` | 140 (50/90) / 89 (35/54) | E-16, plus raw INSERTs (E-28). |
| `"closed"`/`"current"` beside `disposition` | 18 | E-6, E-7 and E-29 to E-31. |
| `pull_request` beside a closed ticket | none pins a closed ticket with a link | E-21 below. |

**`ClosureDto` literals (E-5).** All name every field, none uses `..`. Test literals: `closure/run_tests.rs:1572`; `drafts/compare_tests.rs:109,151`; `desktop/model/edit_tests.rs:92`; `desktop/model/ticket_view_tests/fixtures.rs:64`; `bin/manyhands-cli/human_tests.rs:90,318,345,381,400`; `tests/ticket_closure/journeys.rs:406`. This matches Baseline Fact 23. Production: `read/items.rs:1514,1520`, `desktop/model/drafts/create.rs:139`.

**`PollMutationDto` literals (E-5).** They name all 5 fields at `desktop/services/scheduler_attention_answer_tests.rs:322`, `scheduler_loop_tests.rs:789`, `poll_convert_tests.rs:447`, `bin/manyhands-cli/polling_tests.rs:85` and the production `bind/poll.rs:332`. `polling_tests.rs:362` uses `..poll`, so it needs no edit. `mutation/dto.rs` has no `PollMutationDto {` literal (its tests go through `from_stored`, 3080, 3484), and the design's mention of `dto.rs` for E-5 is empty.

**Candidate new E rows (E-24 onwards).**
- **E-24** `desktop/services/poll_convert_tests.rs:401` `check_now_attempt_reads_the_report`: its 18-pair table asserts `pairs.len() == Dto::ALL.len()` (line 426). It fails when `Merged` and `Unheld` are added. They need rows, or the desktop `Disposition` gains them (R-14).
- **E-25** `desktop/model/polling/entry_tests.rs:30` `dispositions_listed` and `desktop/model/polling_schedule_tests.rs:9` `attempt_summary_counts` list the desktop dispositions. Neither fails; extend them only if R-14 gives the two values a classification. `AttemptSummary::of` (`attempt.rs:217`) matches exhaustively in production. `polling_privacy_tests.rs:449`, `retire_tests.rs:100` and `polling_window.rs:117` use `Closed` and `Promoted` only.
- **E-26** `src/results_tests.rs:1053` `problem_codes_have_unique_snake_case_strings_and_fixed_guidance` pins the whole `ProblemCode` name list (28 names, ending `unknown_problem`). It needs `primary_not_checked_out` and `primary_dirty` inserted at their `ALL` positions. `stored_problem_codes_map_to_the_registry` (1015) fails only if the new codes have a stored form (they should not). `read_contract.rs:1886` and `3135`, and `read/tests.rs:184`, iterate `ALL` and pass once both problem schemas list the codes.
- **E-27** The step and action arrays in the lifecycle record tests: `closure/record_tests.rs:342` `ALL_STEPS` (15) feeds `steps_permit_only_designed_transitions` (144); `closure_step_round_trips_through_parent_view` (361) lists `ParentStep`; `promotion/record_tests.rs:1559` `ALL_STEPS` (13) feeds `pull_request_promotion_steps_and_checks` (1840, graph at about 2406). The new step `remote_deleting` and action `cleanup` appear in none; add them. None fails by itself.
- **E-28** Raw `INSERT INTO ticket_closures`/`document_promotions` with explicit column lists survive a rebuild only if the new columns have defaults (Task 7 checks): `closure/record_tests.rs` 623, 990, 1493, 1863, 1961, 1969, 2043, 2219, 2232; `bind/lifecycle_tests.rs` 2270, 2446, 2584; `setup/change_tests.rs:577`; `promotion/record_tests.rs:1749`; `tests/read_status.rs` 1405, 1798, 1821. `local_sync/record_tests.rs:352` `operation_id_collides_with_every_journal_and_child_namespace` inserts into both tables. `recovery.rs:4649` inserts positionally, into its own stub tables.
- **E-29** `closure/run_tests.rs:2245` `a_poll_does_not_retire_a_context_a_direct_closure_holds` (asserts `closed`, 2277) and `closure/run_pull_request_tests.rs:2954` `a_poll_does_not_retire_a_context_a_branch_record_holds` (asserts `closed`, 2977). Both observe the branch deleted with the local primary holding the closure. Expected unchanged (the proof passes, since the closure merged the context), but they run the new proof. Verify.
- **E-30** `command.rs:1182` `every_command_has_the_setup_gate_of_the_design` (ends `seen.len() == Command::ALL.len()`, 1260) fails until `ContextCleanup` has a row. Check also `command.rs` 804 `a_stored_name_is_read_back_as_its_command`, 860 `the_stopping_commands_and_their_stages_and_confirmation`, 966 `the_commands_whose_caller_must_first_be_shown_need_confirmation`, and `mutation/outcome_tests.rs` 1385-2299 (`Command::ALL` loops).
- **E-31** `remote/state_tests.rs` catalog tests beyond E-14's list: `task6_disposition_schema_upgrade_rollback_corruption_and_privacy` (1031), `task6_disposition_schema_is_empty_atomic_metadata_only` (1087), `task6_partial_disposition_schema_is_not_recreated` (1127), `review_disposition_envelope_rejects_lost_column_pair_without_recreation` (473), `..._lost_object_pair_...` (500), `review_disposition_metadata_loss_is_not_hidden_by_updated_at_fallback` (531), `review_disposition_every_recognizable_partial_envelope_is_retained` (959). They read the catalog through `historical_disposition_schema` (asserts profile in 15..=18, line 666) and `install_historical_dispositions`.
- **E-32** `tests/cli/recovery_tables.rs` 77, 130 and `tests/cli/skills.rs:446` list `ticket withdraw-closure`. E-12/E-13 already cover them.

## 2. Goldens

SSH = the writer's target file uses `tests/support/ssh_harness.rs`.

**Poll answer data** (E-3):
- `cli_v1/poll_once_fast_forward_and_materialize.json` (+ `poll_once_fast_forward_text.txt`): `cli_remote/polling.rs:338` `c5_polling_fast_forward_and_materialize_once`, SSH.
- `cli_v1/poll_once_dirty.json`: `polling.rs:422` `c5_polling_dirty_context_preserved`, SSH.
- `cli_v1/poll_once_diverged.json`: `polling.rs:453` `c5_polling_divergent_context_preserved`, SSH.
- `cli_v1/poll_once_malformed.json`: `polling.rs:475` `c5_polling_malformed_ref_names_nothing`, SSH.
- `cli_v1/poll_once_remotely_deleted.json`: `polling.rs:515` `c5_polling_deleted_context_preserved`, SSH.
- `cli_v1/poll_once_closed_ticket.json`: `polling.rs:710` `c5_polling_remote_context_of_a_closed_ticket` (golden at 724), SSH.
- `mutation_v1/poll_once.json`: `mutation_remote/c5_poll.rs:1363` `once_ok`, SSH.
- `poll_once_discovery_pending`: `c5_poll.rs:1448` `once_discovery_pending`, SSH.
- `poll_once_cancelled`: `c5_poll.rs:1537` `once_cancelled_by_the_token`, SSH.
- `poll_once_interrupted`: `c5_poll.rs:1677` `once_interrupted`, SSH.
- `poll_once_repository_inaccessible`: `c5_poll.rs:1884` `once_stored_repository_unavailable`, SSH.
- `poll_once_key_rejected_reentered`: `c5_poll.rs:1952` `once_key_rejected_reentered`, SSH.
- `mutation_v1/poll_retirement.json` (`poll_mutation` shape and `poll_retirement`): `tests/mutation_contract.rs:42` `an_opted_poll_matches_its_distinct_closed_schema_and_golden`, fast tier, not SSH.

The `poll_once_setup_activated` and `poll_once_setup_not_published` goldens carry setup data, not a poll answer, so they are unaffected. Text goldens (`poll_once_*_text.txt`) change only where an entry prints, and none does.

**Item DTO goldens** (E-4), all fast tier, none SSH. `read_v1/` is written in `tests/read_contract.rs`:
- `document_list` 2004 `document_list_matches_its_schema_and_golden`
- `document_show` 2053 `document_show_matches_its_schema_and_golden`
- `document_show_nonconforming` 2546 `a_nonconforming_resource_read_by_path_matches_its_schema_and_golden`
- `ticket_list` 2079 `ticket_list_matches_its_schema_and_golden`
- `ticket_show` 2102 `ticket_show_matches_its_schema_and_golden`
- `ticket_show_relationships` 2128 `a_ticket_with_relationships_matches_its_schema_and_golden`
- `ticket_list_relationship_problems` 2162 `ticket_relationship_problems_match_their_schema_and_golden`
- `ticket_ready` 2285 `ticket_ready_matches_its_schema_and_golden`
- `ticket_blocked` 2312 `ticket_blocked_matches_its_schema_and_golden`
- `ticket_children` 2381 `ticket_children_matches_its_schema_and_golden`
- `ticket_plan` 2428 `ticket_plan_matches_its_schema_and_golden`
- `ticket_critical_path` 2450 `ticket_critical_path_matches_its_schema_and_golden`
- `ticket_find` 2468 `ticket_find_matches_its_schema_and_golden`
- `ticket_list_nonconforming` 2493 `a_nonconforming_ticket_list_entry_matches_its_schema_and_golden`

The `cli_v1/cli_ticket_{show,list,ready,blocked,children,plan,critical_path,find}` goldens are written in `tests/cli/relationships.rs:56` `contract` (callers 107-211) and in `tests/cli/content.rs`: 100 `document_and_ticket_lists_match_f1_contracts_and_human_output` (list), 132 `show_by_id_and_both_paths_with_optional_context_preserves_f1_scope_and_body` (show), 229 `nonconforming_document_show_and_list_expose_path_and_fixed_problem_code`. The remaining item-carrying goldens are `cli_document_list`, `cli_document_show` and `cli_document_show_nonconforming` (all through `content.rs`). Preview goldens (`ticket_close_preview` and similar) carry a *planned* closure, not `ClosureDto`; `drafts/*.json` and `cli_input_v1` are not item DTOs. No mutation golden carries an item DTO.

## 3. U-7 (`ItemDto {` literals)

`ItemDto` has 23 fields. Naming all 23, no `..`, so each needs `merged_at: None` (8 test literals, 8 files):
- `drafts/compare_tests.rs:100`
- `desktop/model/editor_text_tests.rs:177`
- `desktop/model/documents_tests.rs:46`
- `desktop/model/edit_tests.rs:41`
- `desktop/model/ticket_view_tests/fixtures.rs:55`
- `desktop/model/drafts/save_tests.rs:61`
- `bin/manyhands-cli/human_tests.rs:44`
- `bin/manyhands-cli/fill_tests.rs:6`

Production naming all 23: `read/items.rs:1469`, `desktop/model/drafts/create.rs:130`.

Struct-update, so no edit: `human_tests.rs:72,87,565` (`..content_item()`) and `fill_tests.rs:126` (`..item()`), 4 literals in 2 files. Production struct-update: `read/items.rs:1763,1825,2610,2776` (`..bare_item`), which gain the field through `bare_item`. `tests/` has no `ItemDto {` literal.

## 4. E-6, E-7 and E-21 rows

**E-6**, `remote/poll_tests.rs:25` `retirement_recognition_committed_primary_and_independent_ticket_completion`. Today a ticket is `closed` by `proof::completion(primary)` alone and never reads `L` (`poll.rs` `completed_deleted_context`, about 290). The classification is called with the tracking state `RemotelyDeleted`, except `unknown` and `unpublished`.

| Case | Pins today | Under the design |
| --- | --- | --- |
| `closed` | ticket, primary holds `closed_at`, `L` is the primary's parent, so `closed` | held, proof passes: `closed` |
| `closed_missing_local_ref`, `closed_symbolic_local_ref` | `closed` | `L` unreadable: `closed` (K-14), unchanged |
| `dirty` | context worktree dirty, `closed` | proof reads `L`, not the worktree: `closed`, unchanged |
| `unmatched` | `L` carries an extra file `unmatched` the primary lacks, **`Closed`** | `ContentMismatch`: **`Unheld`**. This is the only row that changes (the `expected` match at about 217-224). |
| `open`, `malformed`, `tracking_only` | `remotely_deleted` (primary not held or unreadable) | not held: `remotely_deleted`, unchanged |
| `promoted` | document, proof passes: `promoted` | unchanged |
| `document_missing_local_ref`, `document_symbolic_local_ref`, `presence_only` | `remotely_deleted` (no local ref, or the proof fails) | unchanged (a document is never `unheld`) |
| `unknown`, `unpublished` | `history_unknown`, `unpublished` | unchanged |

Other E-6 candidates:
- `tests/ticket_closure/journeys.rs:311` `contained_closed` (called at 22 and 26; SSH). B's kept context `edited` is contained in A's merge, so the poll (assert at 355) stays `Closed`. No edit.
- `tests/cli_remote/retirement.rs:42` `retirement_cli_authenticated_live_pass` (SSH). B's two contexts are at their published tips and A's close merged them, so both stay `closed` (127). Line 121, `data.len() == 5`, becomes 6 (E-2).
- `tests/cli_remote/polling.rs:710` `c5_polling_remote_context_of_a_closed_ticket` (golden `poll_once_closed_ticket`, SSH). It pins `current` (723, 726) with no local branch, through decision 28 (`poll.rs` about 465, `primary_holds` on materialization). Part D's rows all need an `L`, so it stays `current`. Verify that detection does not run for a target without a local context. The golden gains only `merged: []`.
- `cli_remote/polling.rs:545` `completed_deleted` (three tests from 535): `closed` with a clean or dirty context, unchanged. With a dirty primary the context stays `remotely_deleted`, unchanged.

**E-7: confirmed, no edit.**
- `closure/run_pull_request_lifecycle_tests.rs:372` `lifecycle_ends_closed_and_retired_for_each_merge_style` (Commit, Squash, FastForward, branch deleted): `assert_retired_by` (316) expects `closed`. The retirement already runs the same proof, so it passes and stays `closed`. The helper is used at 389, 444, 504, 602, 669, 773, 857, 945, 1045 and 1050.
- 791 `merged_first_with_the_remote_branch_kept_closes_in_place`: the merge happens while the ticket is open, so the primary's blob has no `closed_at`, which is not held and gives no `merged`. Then it ends `closed`.
- 872 `merged_first_with_the_remote_branch_gone_closes_through_a_new_branch`: after a squash and the primary moving on, the poll asserts `remotely_deleted` (895). The primary's ticket is open, so it is not held, and it stays `remotely_deleted`.
- 1008 `another_clone_reads_closing_after_its_poll`: `assert_ne!(.., "closed")` (1018), unchanged.
- `promotion/run_pull_request_lifecycle_tests.rs:277` `pull_request_promote_lifecycles`: `current` (328) before the merge, with the primary lacking the document or holding a different blob, so not held; `promoted` (382, 406) after the branch is deleted. Unchanged.
- `tests/document_promotion/journeys.rs:435` `happy` (497, `Promoted`) is unchanged.

**E-21.** `tests/read_items.rs:6220` `ticket_show_gives_the_link_and_the_publication` gives a link row only in the world where ticket A is `closing` (6249). Its closed section (6318-6331, `closure_world(Primary::Closed, Own::None)`) has no link row and asserts only `publication == None`. It passes unchanged, so no existing test pins a closed ticket's link. The Part B behaviour needs a new assertion: call `give_link` in that world and assert `pull_request == None`. `document_show_gives_the_link_while_it_is_outstanding` (6339) is about documents. `closure/run_pull_request_lifecycle_tests.rs` asserts `shown.pull_request == None` after retirement (449, 1036); the link row is already gone there. `bin/manyhands-cli/human_tests.rs:342` renders a DTO and is unaffected.

## 5. RecordPath audit (E-23)

The enum (`promotion/lifecycle.rs:179`) has `Direct` and `Branch`. Exhaustive matches with no wildcard: `closure/record.rs:595-597` (`validate` dispatch); `closure/record.rs:1007-1011` (permitted transition); `promotion/lifecycle_record.rs:465-468` (`second_role`); `lifecycle_record.rs:477-485` (the `(RecordPath, ChildRole)` tuple match); `remote/reservation.rs:449` and `459` (`Some((holder, Branch))`, `Some((_, Direct))`, with `None`). Constructors: `closure/record.rs:256-260`, `1651-1653`; `promotion/lifecycle.rs:236-237`; `promotion/record.rs:173-176`.

`==` or `!=` comparisons:
- `closure/record.rs`: 750 (`supersedable`, `== Direct`), 1129, 1239, 1305, 1337 (`!= Branch`), 1616 (`deletion_reservation_required`, `== Direct`), 1685 (`branch_hold`, `== Branch`).
- `mutation/bind/lifecycle.rs`: 238, 244, 763, 1331, 1978, 2062 (all `== Branch`).
- `promotion/lifecycle.rs:1206` (`reruns_by_table`, `== Branch`).
- `promotion/record.rs`: 866, 909 (`!= Branch`).
- `promotion/run.rs`: 375, 525, 665, 2441, 2495, 2528 (`== Branch`).
- `promotion/lifecycle_record.rs:813` (`== Branch`).

Under `Cleanup`, every `== Direct` and `!= Branch` evaluates as it would for the other path (a `Cleanup` record would read as "not Direct" and as "not Branch"), so each is a real audit item.

`deletion_reservation_required`: defined `closure/record.rs:1615` (`path() == Direct && remote_name.is_some() && !(context_attempt == 0 && remote_context_oid.is_none())`); one caller, `ClosureRecord::remote_deletion_required` (`record.rs:1846`). `remote_deletion_required` is a trait method, declared in `promotion/lifecycle.rs:280` (default: `remote_name().is_some() && !(AlreadyContained && remote_context_oid().is_none())`, used by `PromotionRecord`). It is implemented for `ClosureRecord` at `closure/record.rs:1846`. Callers: `promotion/lifecycle_record.rs:991`, `1062`; `promotion/run.rs:1976`, `1990`.

Tests that pin these, none exhaustive:
- `closure/record_tests.rs`: 1723 `remote_deletion_required_unless_contained_without_tracking`; 1762 `closure_refusal_blockers_v2_rebuilds_an_old_bound_table_once` (path at 1795); 1924 `branch_records_rebuild_and_keep_direct_rows` (2021, 2065); 2246 `a_branch_record_begins_replays_and_is_refused_on_its_own_path` (2267, 2296, 2329).
- `promotion/record_tests.rs`: 1606 `promotion_modes_migration_rebuilds_a_v2_table_once` (1681); 1840 `pull_request_promotion_steps_and_checks` (2341, 2441); 2644 `pull_request_begin_and_finish_write_their_columns` (2679).
- `promotion/lifecycle_tests.rs:171` `promotion_remote_deletion_rule_is_unchanged` (189-195).
- `mutation/bind/lifecycle_tests.rs`: 264 `stopped_with`; 1003 `a_branch_record_s_answer_shows_what_its_context_child_published`; 1076 `a_stopped_branch_record_shows_what_it_wrote_and_its_publication_pushed`; 1205 `branch_stopped_with`; 1227 `a_branch_record_s_busy_stop_names_its_blocker`; 1399 `a_branch_record_s_answers_name_the_ticket_s_branch_and_a_direct_one_s_name_primary`; 1821 `a_branch_parent_view_is_never_integrated`.

`operation list`/`operation resume` lifecycle rows (E-23's last clause): `closure/run_pull_request_tests.rs:1790` `d20_operations_list_a_branch_record_as_a_close_and_its_child_as_its_own_row`; `promotion/run_pull_request_rerun_tests.rs:1909` `holds_cancellation_and_listing`; `mutation_remote/resume.rs:856` `c4_resume_cancelled_with_work_owed`; `bind/lifecycle_tests.rs:2243` `a_withdraw_is_a_lifecycle_parent_wherever_its_command_is_read`.

## 6. Abandon pins (E-17)

Neither `src/repository/bridges/abandon.rs` nor `src/repository/mutation/bind/abandon.rs` has tests of its own, and neither mentions closures or promotions. `src/repository/remote/reservation_tests.rs` has no `abandonable` call and no test that names it (its only abandon text is `UPDATE ticket_closures SET step='abandoned'` fixtures, 3964, 4194-4242). `reservation::abandonable` (`reservation.rs:909`) returns false for any synchronization whose id is in `document_promotions` or `ticket_closures` (`operation_id`, `context_sync_id` or `primary_sync_id`). The tests that pin a lifecycle record, or its child, as not abandonable:
- `closure/run_pull_request_tests.rs:1790` `d20_operations_list_a_branch_record_as_a_close_and_its_child_as_its_own_row`: asserts `!abandonable` for the child (1856-1861).
- `promotion/run_pull_request_rerun_tests.rs:1909` `holds_cancellation_and_listing`: `!abandonable` (2059-2064).
- `tests/mutation_remote/resume.rs:856` `c4_resume_cancelled_with_work_owed`: expects an `OperationAbandon` action for a sync child (891-896).
- `tests/mutation_confirmation/retirement.rs`, `abandon.rs` and `conformance.rs:121`: `operation abandon` previews of other operations; they do not name lifecycle records.
- `src/repository/local_sync/record_tests.rs:352` `operation_id_collides_with_every_journal_and_child_namespace` (rows for closure and promotion families): about the namespace, not abandon.
- `src/repository/closure/checkpoint_tests.rs:1590`: a comment about "an abandonable refusal", no assertion.

E-17's sentence "the tests of `bridges/abandon.rs` ... that pin lifecycle records as not abandonable" matches no inline test. The cleanup abandon needs new tests of its own (CC26), and these three tests stay as they are, since `abandonable` itself still answers false for a cleanup record's children.
