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

## Task 1: The Arrival Record At The Poll's Move

- BASE `bbca7fb1`. Implementer (Sonnet 5.5, high): commit `f1fe3e22` "repository: arrivals over the poll's fast-forward". Files: `src/repository/item_merges.rs` and `item_merges_tests.rs` (new), `src/repository.rs`, `src/repository/discovery.rs` (`migrate_item_merges` after `migrate_pull_request_links`, marker `wave_06_item_merges_v1`, its unit test), `src/repository/remote/poll.rs` (the primary target's `FastForwarded` records with `owner.operation_id()`), `src/repository/remote/poll_tests.rs`, `tests/repository_enablement.rs` (E-1).
- Tests, each by exact name, red then green: `repository::discovery::tests::item_merges_migration_creates_its_table_once`, `repository::item_merges::tests::arrivals_follow_the_range` (20 rows; red against an empty stub on 16 rows, the four nothing-expected rows pinning unchanged behaviour; red against "diff every parent" on 5 rows and against "the oldest commit" on exactly the two-arrival and ticket-save rows), `repository::item_merges::tests::record_keeps_the_newer_arrival`, `repository::remote::poll::tests::the_poll_records_its_arrivals`; E-1 `--test repository_enablement registry_creates_repository_and_discovery_metadata_schema` red before the list gained `item_merges`, green after. Static gates clean.
- Implementer's concerns, ruled: a document deleted at one path while an unchanged path holds its ID is reported deleted — Ruling 1-1: accepted as the rule (a document ID at two paths is already an item problem), the doc comment to say so in the final fix wave — a row removed for a duplicated ID until that copy arrives again. A deleted ticket keeps its row — Ruling 1-2: as designed (R-2 names documents), tickets are not deleted from the primary — none. A commit time outside the table's range skips its row, and a year above 9999 fails the range — Ruling 1-3: within R-5 (a failure is dropped); the final fix wave may carry seconds per arrival — a range with such a commit records nothing. `mutation_replay` not run — the reviewer checked it as a named risk (its older database is migrated first), and the fast tier runs it in Task 15.
- Review (Sonnet 5.5): spec compliant, task quality approved; 0 Critical, 0 Important, 8 Minor, parked for the final review: (1) Ruling 1-1's doc comment; (2) a bad commit time fails the whole range; (3) a deleted ticket's row; (4) `arrivals` is long, extract a `candidates` helper; (5) `record_after_move`'s closure returning `Result<(), ()>`; (6) a comment on the walk's newest-first order; (7) `rows()` duplicated between `poll_tests.rs` and `item_merges_tests.rs`; (8) four nothing-expected rows green against the empty stub.
- Task 1: complete (commits bbca7fb1..f1fe3e22, review clean, 8 Minor parked).

## Task 2: Recording At Every Other Move

- BASE `2c48629d`. Design checkpoint (Step 0), written by the orchestrator from design Part A and the research and approved before code: the six sites, each with how it knows the ref is the primary, its `before`, `after` and operation ID (`sync.rs` `reconcile_pending_candidate`, both `integrate_divergence` stages, `synchronize_remote_observed_inner`; the runner after `L::apply` answered `Merged`, with `prepared.evidence().old_primary` read before `write_commit`; setup `activate` with a new `OperationId`). The implementer verified all six rows against the source.
- Implementer (Sonnet 5.5, high): commit `fcd98605` "repository: every move of the primary records its arrivals". The four sync sites share one gate, `record_primary_arrivals` (`sync.rs` about 7977), which records only for a `Primary` target. The runner records after `apply`, before the `AfterIntegrationRef` boundary; setup records after its lease block, before `finish_held`.
- Tests, each by exact name, red by removing its site's call, then green: `repository::remote::sync::tests::repo_sync_records_its_arrivals` (the restart and both `integrate_divergence` stages, and the gate with a `Context` target), `repository::closure::run_tests::a_direct_close_records_its_arrival`, `repository::promotion::run_tests::a_direct_promote_records_its_arrival`, `repository::setup::poll::tests::a_setup_activation_records_its_arrivals` (the activation's operation ID asserted to be a valid ULID, since it is minted inside). Static gates clean.
- Review 1 (Sonnet 5.5): spec ❌, 2 Important: the clean path's call (`sync.rs` about 7231) was pinned by no test; the new promote test displaced the F-1 doc comment in `promotion/run_tests.rs`. 4 Minor, deferred to the final review: (m1) the setup operation ID asserted only as a valid ULID; (m2) two readers of `item_merges` rows in tests (`rows_for_test` and Task 1's); (m3) the runner's `AlreadyContained` and `RecoveryRequired` non-recording branches untested; (m4) placement notes, judged sound.
- Ruling 2-1: the clean path is pinned by a row assertion added to an existing SSH case, run by exact name, with no new SSH case — the plan forbids a new SSH case and allows one by name between steps — an SSH target's fixture changes (named edit E-33), run again in Task 15's tier.
- Fix round 1 (the same implementer): commit `4ccfec7f`. E-33: `tests/remote_synchronization.rs` `primary_current_fast_forward_local_ahead`: the peer's advance also adds a managed document, and the case asserts the `item_merges` row at the commit that added it with the synchronization's operation ID; run alone by exact name, red with the clean-path call removed (0 passed, 1 failed), green with it restored ("1 SSH cases passed"). The promote test moved above the F-1 comment block. Static gates clean. Scoped re-review (Sonnet 5.5): both findings ADDRESSED, no new breakage.
- Task 2: fix round 1/5 (2 addressed, 0 open; commits fcd98605..4ccfec7f).
- Task 2: complete (commits 2c48629d..4ccfec7f, review clean, 4 Minor parked).

## Task 3: `merged_at` And A Closed Ticket's Link In Reads

- BASE `c942843f`. Implementer (Sonnet 5.5, high): commit `5db80123` "read: merged_at, and a closed ticket's link ends". `ClosureDto.merged_at` and a top-level `ItemDto.merged_at` (both `Option<String>`, required and nullable in `item.schema.json`), filled by `fill_complete_read` through the new `item_merges::merge_of` with the `timestamp()` formatter `closed_at` uses: a ticket's only while its closure reads `closed` and a row exists, a document's whenever a row exists, null in lists. A closed ticket's `pull_request` is null whatever its link row says (decision 10). Human `show` prints `Merged at:` after `Closed by:` for a ticket and after `Title:` for a document.
- Tests, red first, by exact name: `tests/read_items.rs` `ticket_show_gives_merged_at_on_its_closure`, `document_show_gives_merged_at`, `a_closed_ticket_s_link_is_no_longer_offered` (E-21 as a new assertion, Ruling P-2); `tests/discovery_rebuild.rs` `an_item_merge_survives_an_index_rebuild` (red only against a stub that deletes the rows: it pins unchanged behaviour); a `human::tests` test of `Merged at:`. E-4: the 25 golden writers, by exact name with `MANYHANDS_UPDATE_GOLDEN=1`, rewrote 24 fixtures with 97 added lines, each `"merged_at": null,`, and no line removed. E-5's literals. Static gates and the desktop compile check clean.
- E-34 (found by the implementer): `closure/run_tests.rs` `a_completed_closure_reads_closed_from_the_primary_only` now asserts the shown closure's `merged_at` is set (Task 2's direct close records an arrival) and compares the list entry with `merged_at` null; every other field still exact.
- Review 1 (Sonnet 5.5): spec ❌, 1 Important: `tests/ticket_closure/journeys.rs` `discoverable` (about 1188) compares a shown closure with its list entry and would fail. 3 Minor, deferred: (m1) a ragged reflow of `ItemDto.pull_request`'s doc comment in `read/dto.rs`; (m2) `merge_of` is queried for an open ticket only to be dropped; (m3) `ItemMerge` omits the commit.
- Fix round 1 (the same implementer): commit `8ac876e6`. E-35: `discoverable` asserts the shown `merged_at` is set and compares the rest exactly; `--test ticket_closure closed_ticket_discoverable_from_both_clones -- --exact` (one SSH case) red before ("observation: 1188"), green after. Scoped re-review (Sonnet 5.5): ADDRESSED, no new breakage.
- Task 3: fix round 1/5 (1 addressed, 0 open; commits 5db80123..8ac876e6).
- Task 3: complete (commits c942843f..8ac876e6, review clean, 3 Minor parked).

## Task 4: The Disposition Catalog

- BASE `781201d0`; `origin/main` `15ea8eb5`, unchanged (Step 0's merge a no-op). Design checkpoint (Step 0), written by the orchestrator from `migrate_poll_dispositions` and design Part D's truth table and approved before code: `POLL_DISPOSITION_SCHEMA_18` frozen from today's catalog, a current catalog with `merged` and `unheld`, two markers read and checked against the source's expected pair, refusals before any DDL, the marker inserts after the rebuild, inside `remote::state::migrate` in the first transaction. Its one slip: today's catalog ends `'unpublished','not_reached'`, not `'unpublished'`; the implementer copied the constant verbatim and appended `'merged','unheld'` after `'not_reached'`.
- Implementer (Sonnet 5.5, high): commit `6413f953` "registry: the poll disposition catalog gains merged and unheld". No Rust disposition enum changes (Task 5's).
- Tests: `repository::remote::state::persistence_tests::the_poll_disposition_catalog_upgrades_by_its_truth_table` (one row per truth-table line, every "other catalog with the merged marker" and contradiction combination, from catalogs holding a row of every value they allow; a refusal leaves the whole image unchanged), red before the change and by row against four stubs; the 13 edited or affected catalog tests and the new one, 14 passed; E-15 red without its edit, green with it. Static gates clean.
- E-14 as named: the helpers map profile 19 to the current catalog, give 16, 18 and 19 the `unpublished` marker and only 19 the `merged` marker; both markers deleted where a test simulates a registry from before the envelope; `profile != 19`; the dependency cross product and the exact-old-enum test widened to 19, so 18 runs the real upgrade. E-31's seven tests needed no edit: they test envelope loss on a current registry with both markers present and call neither helper (the sweep's stated reason, "through the two helpers", was wrong; the conclusion stands).
- E-36 (found at review, additive, logged here): `retirement_disposition_old_corrupt_and_partial_envelopes_roll_back` gains the rows (19, DELETE merged) and (18, INSERT merged), and `poll_dispositions_unpublished_migration_preserves_rows_and_is_idempotent` gains an assertion that the `merged` marker count is 1; no red run of these additions was recorded (they would go red against the first stub and the old code respectively).
- Review (Opus 5.5): spec compliant, task quality approved; 0 Critical, 0 Important, 4 Minor, deferred: (m1) E-36's unlisted additions without recorded red runs; (m2) the new test's `image` and `old_values` closures duplicate the corrupt test's and the exact-old-enum test's; (m3) the report's and the sweep's E-31 rationale (corrected above); (m4) a comment stronger than its probes (state_tests.rs about 1088) and `marked` beside `merged_marked` naming in `state.rs`. Named risks checked by the reviewer: a real registry at 18 with the `unpublished` marker upgrades with its rows; a refusal leaves no `remote_poll_dispositions_old` and no half marker (one transaction, refusals before DDL); `validate_poll_disposition_schema` and `audit_registry` compare against the new current catalog.
- Task 4: complete (commits 781201d0..6413f953, review clean, 4 Minor parked).

## Task 5: Detection Per Context

- BASE `f2f74177`; main unchanged (Step 0's merge a no-op). The controller's notes (`task-5-notes.md` in the workspace) resolved the brief's open points: every place of Baseline Fact 6, the desktop's mapping (Ruling P-3), the guidance strings, the classification rules and the named edits.
- Implementer (Sonnet 5.5, high): commit `7c66f6c6` "poll: merged and unheld contexts". Detection sits only in `classify_local_target`'s `PollRelation::Current` arm, after `require_clean_target`, and in `completed_deleted_context`; the pre-filter (a ticket's primary blob at its path with `closed_at`; a document's primary blob at `L`'s document path equal to `L`'s) before `proof::preservation`, which opens the repository at the root because libgit2 does not see the `shallow` file through a linked worktree's handle; only `ContentMismatch` gives `unheld`, for a ticket only. The two values at every listed place, the desktop mapping them as `Current`.
- Tests, by exact name: `repository::remote::poll::tests::detection_per_context` (Part D's rows, among them a later code commit, a shallow repository, an unreadable `L`, unrelated history, a document moved on the primary, `FastForward` and `LocalAhead` relations, and a whole poll with the root off the primary recording and detecting nothing) red against today's classification (6 new rows) and against a skipped proof (the `unheld` rows and 3 `current` rows); `an_old_frozen_report_still_decodes` red against a stub dropping the two values from `decode_frozen`. E-6 (only the `unmatched` row to `Unheld`), E-7 (none), E-18 (both guidance tables; `tests/remote_polling.rs`'s case alone by exact name), E-19, E-24, E-25 and E-29 (unchanged, run by exact name), each by exact name. Desktop compile check and static gates clean.
- E-37 and E-38 (additive desktop test edits, logged at review): `desktop/services/poll_convert_tests.rs` `poll_attempt_conversion` gains two rows (18 to 20); `desktop/model/activity_tests/states_copy.rs` `state_copy_table` gains the two values.
- Ruling 5-1: a ticket whose direct closure is still in progress (the primary holds the closure, the remote branch not yet deleted) reads `merged` in a poll during that window — the poll's closure fence sits only on the materialization and fast-forward paths, `merged` triggers no retirement and the desktop counts it up to date, and a cleanup's prepare is refused while the close's record holds the item (CC3) — the report shows `merged` for that window where it showed `current`.
- Review (Opus 5.5): spec compliant, task quality approved; 0 Critical, 0 Important, 7 Minor, deferred: (m1) MUST TRIAGE AT FINAL REVIEW — the document pre-filter reads and parses most of `docs/` on every poll for each clean, equal document context to find the document's path (`remote/poll.rs` about 389-430 `document_blob`), which sits uneasily with R-12; fix by the index's recorded path or a tree diff of `docs/`; (m2) E-37 and E-38 logged above; (m3) the two values sit in `state_copy_table`'s "not shared yet" loop; (m4) `done_on_the_primary` reopens the root repository per context, formats the ticket path twice, borrows a temporary; (m5) Ruling 5-1 verified; (m6) a document context with no edits since its promotion reads `merged`, as designed; (m7) no `MultipleMergeBases`, `MissingObject` or `InvalidPrimaryItem` rows (one arm covers them).
- Task 15's SSH tier must include the cases predicted unchanged that this task did not run: `ticket_closure` `contained_closed`, `cli_remote` `retirement_cli_authenticated_live_pass` and `c5_polling_remote_context_of_a_closed_ticket` with its golden, `remote_polling` `completed_deleted` (all in targets the tier already selects).
- Task 5: complete (commits f2f74177..7c66f6c6, review clean, 7 Minor parked).

## Task 6: The Poll's `merged` List

- BASE `3a107311`. Design checkpoint (Step 0), written by the orchestrator from `bind/poll.rs` `poll_data` and `reported` and `context_retirement/pass_report.rs` `finish` and approved before code: `poll_data` stays pure with `merged` empty; one builder reads the poll's own operation's rows (`item_merges::merges_by`) and each ticket's `tickets_made_ready` outside any registry write transaction; the ordinary poll fills it in `reported` after its refresh; a retiring poll's `finish` computes it before its transaction, only when no summary is frozen, and freezes it with the summary (R-38). The implementer verified every point; two refinements: the retiring poll's `refreshed` flag is the original operation's `!index_pending`, read in `finish`; `finish` computes the list only when no summary is frozen.
- Implementer (Sonnet 5.5, high): commit `9efb9acb` "poll: the merged list". `PollMergedDto` and the poll's own `PollMadeReadyState` (`current`, `index_pending`); `from_stored` takes five or six fields; the schema's closed `merged` objects; the human lines; `merges_by` filtered by `operation_ulid`.
- Tests, red first against named stubs (among them `refreshed` forced false, and `merges_by` ignoring the operation), by exact name: `repository::closure::run_tests::pull_request::lifecycles::a_poll_lists_its_arrivals_with_the_dependents_made_ready`, `…::a_retiring_poll_lists_its_arrivals` (with a request-level replay), `repository::promotion::run_tests::pull_request::lifecycles::a_poll_lists_a_document_s_arrival`; three `mutation/dto.rs` decoding tests (E-2's edit and two new, among them `an_old_poll_retirement_with_a_five_field_poll_still_decodes`); the CLI lines test; the desktop literal tests. E-3: the 13 poll goldens rewritten by their writers one case at a time (the SSH cases alone), each diff only `"merged": [],`; checked afterwards that every fixture holding `targets` holds `merged` (13 of 13). E-2's `tests/cli_remote/retirement.rs` `retirement_cli_authenticated_live_pass` alone (5 to 6 fields). E-5 and E-8. Static gates clean.
- Review (Sonnet 5.5): spec compliant, task quality approved; 0 Critical, 0 Important, 5 Minor, deferred: (m1) the retiring replay test proves the request-level replay, not that `finish` ignores changed rows once a summary is frozen; (m2) the schema does not tie `made_ready` and its state to the item's kind, which the decoder enforces; (m3) `all_from_stored` does not tie `kind` to the shape; (m4) `merged_of` drops a row whose time cannot be formatted with no comment saying why; (m5) `finish` makes up to two more registry reads. Named risks checked: the builder never fails the answer; the index reads in `finish` run before its write transaction, without nesting; a poll that did not move the primary lists none.
- `docs/RFC/cli-contract.md`'s "unchanged five fields" is Task 14's (D-1).
- Task 6: complete (commits 3a107311..9efb9acb, review clean, 5 Minor parked).

## Task 7: Design Checkpoint (Step 0)

- Written before any code of Part E, as the plan's Task 7 Step 0 requires: a designer on Opus 5.5 wrote the mechanism for every row CC1 to CC31 (revision 1); a separate checker on Opus 5.5 re-read the source for each claim (2 Blocking, 4 Important, 10 Minor); the designer revised it (revision 2), the checker re-checked the revision (1 Blocking, 0 Important, 6 Minor), and the designer applied that (revision 3). The orchestrator approved revision 3 on 2026-11-28. It is in Appendix B verbatim, the check and re-check in Appendix C.
- The checkpoint's STOP FOR OWNER item, confirmed and widened by the checker: after a cleanup's remote delete was observed, or once a cleanup with no publication remote began its local removal, `complete_record` and `cleanup::retire_context` require the registry's remote and primary to equal the record's plan, which a hand edit of the configuration followed by any poll, `remote select` or `remote remove` breaks, with abandon refused there (decision 13). The owner chose a known limit (Cycle document decision 16, K-22, 2026-11-28; saved and published at `038e86fb`): the same request answers `recovery_required`, naming the cause, until the configuration is restored.
- The checkpoint's main choices: both tables rebuilt to a v4 schema under `wave_06_closure_cleanup_v1` and `wave_06_promotion_cleanup_v1`, today's frozen as `SCHEMA_V3`, every new column nullable or defaulted (E-28's raw inserts unchanged); the third path as a sibling entry `run_cleanup_lifecycle` in a new `promotion/cleanup_run.rs`; the before-send hook passing `AfterAdvertisement` for every tip and saving `remote_deleting` only at `Some(R)`, through a wrapper in `remote/sync.rs` over `remote_safe_point_with`; `written_closure_holder`'s exclusion of cleanup records in Task 7 with the E-23 audit, and a `cleanup_holder` at the four retirement sites; `end_parent_row` (from `supersede`) to end a dead holder's row on an abandoned record and a fenced row at completion; `complete_record`'s owner rules relaxed for a fenced cleanup row only; all 22 `PromotionIo` implementers listed (E-40); the split across Tasks 7 to 10 so that each commit is clippy-clean.
- Ruling 7-1: `schemas/v1/operation_abandon_preview.schema.json`'s `action` values gain `close` and `promote`, so the abandon's preview can name a cleanup record's action — decision 11 has `operation abandon` reach a cleanup record, and the addition is additive — none.
- Ruling 7-2: `operation list` shows every unfinished cleanup record, with or without a reservation row (the checkpoint's M9) — better than K-18 states — K-18 is amended in the Cycle document at closure (D-6) and Task 14's contract says what the code does.
- Ruling 7-3: `remote/reservation.rs` changes in two places only: `finish_parent`'s join for a cleanup record (completed, and `cancelled`, a promotion arm new), the design's one permitted rule change; and a compile-only arm in `closure_holds_context`'s `RecordPath` match (about 449-459) that `RecordPath::Cleanup` forces and that never runs (cleanup records are excluded from `written_closure_holder`) — not a rule change — none.
- Ruling 7-4: `written_closure_holder`'s exclusion of cleanup records moves from the plan's Task 9 to Task 7, with E-23's audit of `RecordPath` — it is a `RecordPath` comparison the rebuild makes reachable — Task 9's file list loses it.

## Task 7: The Cleanup Records

- BASE `779ea705`; main unchanged (Step 0's merge a no-op). Built to the approved checkpoint (Appendix B) §1, §6's exclusion of cleanup records from `written_closure_holder` (Ruling 7-4) and §10's Task 7 row.
- Implementer (Sonnet 5.5, high): commit `98d83225` "lifecycle: cleanup records". Both tables rebuilt once from the frozen `SCHEMA_V3` with an explicit column list, under `wave_06_closure_cleanup_v1` and `wave_06_promotion_cleanup_v1`, inside `migrate_registry`'s first transaction; the action `cleanup` (promotion's a new `action` column, default `'promote'`), the step `remote_deleting`, the cleanup facts (`unheld_digest`, `primary_branch`, `endpoint_digest`, the stored `remote_branch` outcome), the cleanup CHECKs, graph, validation and immutables; `RecordPath::Cleanup` with every arm §1.5 lists; a cleanup record never superseded and never continued by the close, withdraw or promote verbs. `remote/reservation.rs` changes only by the compile-only arm at about 459.
- Ruling 7-5: `branch_preview` keeps its signature and takes the action by a guarded `.expect`, because its only caller, `Confirmed::preview`, refuses a cleanup action first; `branch_reached` answers `recovery_required` on `None` — it meets §1.5's intent with no test edit, and the same file has a precedent at about 1797 — a future caller without the guard would panic. The reviewer verified the guard on every path.
- Review 1 (Opus 5.5): spec ❌, 0 Critical, 2 Important, 6 Minor. Ruling 7-6: E-27 is applied in this task. Checkpoint §10's Task 7 row lists it, and the orchestrator's dispatch left it out — none. Ruling 7-7, on the plan-mandated Important: there is one definition of the cleanup graph, the `remote_branch` outcome rule and `validate_cleanup`'s shared tail, and each step type's functions delegate to it. §1.3 asks for the functions on both types, not two bodies, and Tasks 8 to 10 build on the graph — a modest refactor. Ruling 7-8: review Minor 3, no old closure row at `retired`, is a spec gap, because the brief asks for rows of every action and step — none.
- Fix round 1: commit `77544e62` "lifecycle: cleanup records, review fixes". E-27's arrays and the round-trip list gain `RemoteDeleting`, pinning that no direct or branch graph enters or leaves it. The graph, outcome rule and validation tail live once in `promotion/lifecycle_record.rs` (`parent::cleanup_permits`, `remote_branch_kept_or_written`, `CleanupFacts::broken`). The old rows hold every step, with `answer_code` and `consented_context_oid` carrying values. Scoped re-review (Sonnet 5.5): 3 of 3 addressed, the graph and tail shown equivalent for each step type, no new breakage; 1 Minor, a stray "`remote_removed`" in a doc comment near `lifecycle_record.rs` 435.
- Tests, red first against named stubs (about 25 in the first commit; 3 for E-27, 1 for the shared graph and 5 dropped columns in the fix), by exact name: `repository::closure::record::tests::cleanup_records_rebuild_once` and `repository::promotion::record::tests::cleanup_records_rebuild_once`, both table-driven; E-16's ten; E-27's `steps_permit_only_designed_transitions`, `closure_step_round_trips_through_parent_view` and `pull_request_promotion_steps_and_checks`; E-39's `promotion_steps_map_to_the_shared_step_view`; E-15's `tests/recovery_foundation_gate.rs` `failed_recovery_migration_leaves_no_partial_schema_and_retries_cleanly`; the five older rebuild tests on the final tree. E-41 `privacy_column_inventory_for_both_new_tables` and E-42 `no_closed_by_column_or_value` are logged additive edits: the column inventories gain the new columns only. Static gates clean.
- ⚠️ resolved: E-28's raw inserts outside the record test files are not a gap. Every changed CHECK reduces to its V3 form for a non-cleanup row, and the new columns are defaulted; Task 15's fast tier runs them. E-23's `operation list`/resume rows span tasks, because nothing writes a cleanup record before Task 8, so they go with Task 8.
- Carried: to Task 8, closure `save_record` refusing a step change while a local sync of the context is unfinished (`unfinished_context`), and E-23's operation rows. To Task 9, `pull_request_promotion_holder` also matching a cleanup record in pull-request mode, so `cleanup_holder` must not count it twice.
- Minor (deferred): (m1) the Rust-only `validate_cleanup` rules have no failing test: the cleanup category only at a stoppable step, and the primary branch valid for the remote. (m2) Promotion's `validate_direct` and `validate_pull_request` do not refuse `RemoteDeleting` or the cleanup columns; P2 and `permits` cover it. (m4) The promote `begin_record` test asserts only `is_err`, and promotion `refuse_record`'s `continues` check is untested. (m5) Digest decoding is duplicated. (m6) `try_row` duplicates `branch_row`. (m7) The doc-comment slip above.
- Task 7: complete (commits 779ea705..77544e62, review clean after 1 fix round, 6 Minor parked).

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


## Appendix B: Task 7 design checkpoint, revision 3, approved (verbatim)

# Task 7 Design Checkpoint: `context cleanup`'s Mechanism (U-2, U-5, U-6)

Read at `3a107311`, rechecked against the check at `42572592`; paths under `src/repository/`. "Code:" is what the source states, "Choice:" this design's decision, "Inference:" reasoning the code does not state.

## Revision 2

Against task-7-check.md and the owner's decision 16 (K-22).

| Finding | Resolution |
| --- | --- |
| B1 abandoned record with a live-phase row | §2.2 (a0): the same request ends a dead holder's row of an `abandoned` cleanup record before it answers. §7: the abandon reaches that row too. R3 is one transaction (§3). |
| B2 `AfterAdvertisement` not passed for every tip | §2.3: the cleanup's hook passes the safe point for every tip, and saves only at `Some(R)`. |
| I1 `complete_record:991` | §2.4: 991 and 1005 are relaxed together for a fenced cleanup row; 990 is kept. |
| I2 22 test implementers | §2.3 lists them and the rule for each; `advertise` fails closed by default; E-39 onwards; two files added to Task 9. |
| I3 clippy split | §10 rewritten: Task 7 takes compile-forced arms and plain comparisons only. Every trait method lands with its caller. `request_cancellation` is dropped. |
| I4 STOP scope | §11: decision 16's K-22 rule, applied to CC22, CC23, CC29, CC30 and the no-remote case (§4.1). |
| M1 | §1.5: promotion's `begin_record` and `refuse_record` check `continues`. |
| M2 | §8: `found_lifecycle`'s promotion branch is discriminated too. |
| M3 | §1.5: `parent_integrated` arm added. |
| M4 | §10: E-39 lists the four test edits. |
| M5 | §1.3: L and R are immutable from insert. |
| M6 | §2.3: the hook re-reads `cancel_requested`. |
| M7 | §2.2 (a): the takeover freezes the configuration and compares its digest. |
| M8 | §10 records the plan deviations. |
| M9 | Accepted: the K-18 filter is dropped (§8); every unfinished cleanup record is listed, which CC27 permits. |
| M10 | Applied: the terminal refusal in `cleanup_permits`, the digest byte `3`, the outcome built in cleanup_run.rs, and the citations. |
| Revision 3 (re-check R1-R6) | R1: K-22 compares with the record's plan, `(NULL, NULL)` with no remote, and a difference at `confirmed` is R1 (§4.1, §2.2 (g)). R2: `clear_cancellation` moved before (a), never on a terminal record. R3: `AmbiguousDeleteIo`'s retry takes `Send` and returns `Absent` itself. R4: `closure_action` returns `Option`, and `branch_effects` gives none for `Cleanup` (§1.5). R5: `operation list` shows B1's stranded row (§8). R6: the registry rewrite is attributed to the next `configure`; the extra `end_parent_row` callers and the K-18 amendment are in the ledger notes (§10). |

## 1. The Records (Task 7)

### 1.1 `ticket_closures`

Code: `SCHEMA` (closure/record.rs:365-423) limits `action` to `close`, `withdraw`, `new_branch` (392); `step` has no `remote_deleting` (371); CHECKs 414-416 tie integration, branch columns and cleanup steps to `(direct, close)` or `new_branch`. Columns added since V2 are nullable or defaulted (390-397), so E-28's raw inserts survive.

Choice, the new `SCHEMA` (v4):

- `step` gains `'remote_deleting'`; `action` becomes `IN ('close','withdraw','new_branch','cleanup')`, default `'close'`.
- Appended nullable columns, no default: `unheld_digest BLOB CHECK(unheld_digest IS NULL OR (typeof(unheld_digest)='blob' AND length(unheld_digest)=32))`, `primary_branch TEXT CHECK(primary_branch IS NULL OR (typeof(primary_branch)='text' AND length(primary_branch) BETWEEN 1 AND 255))`, `endpoint_digest BLOB` (as promotion/record.rs:264), `remote_branch TEXT CHECK(remote_branch IS NULL OR remote_branch IN ('deleted','absent','none'))`. L is `context_oid` and R is `remote_context_oid`, both written at insert.
- Changed: 404 → `closure_kind IS NOT NULL OR step='abandoned' OR action='cleanup'`; 414 → `(direct AND close) OR (integration_oid IS NULL AND integration_kind IS NULL AND step NOT IN ('integrating','integrated') AND (step!='remote_deleted' OR action='cleanup'))`; 416 gains `OR (action='cleanup' AND step!='retired')`; 417 → `action NOT IN ('new_branch','cleanup')`; 422 gains `OR action='cleanup'`.
- New CHECKs:
  - C1: `action!='cleanup' OR (closure_kind IS NULL AND closed_at IS NULL AND base_oid IS NULL AND integration_oid IS NULL AND integration_kind IS NULL AND publication_category IS NULL AND context_attempt=0 AND primary_attempt=0 AND adopted_context_oid IS NULL AND withdraw_oid IS NULL AND retired_oid IS NULL AND pending_ref IS NULL AND answer_code IS NULL AND consented_context_oid IS NULL AND context_oid IS NOT NULL AND unheld_digest IS NOT NULL AND primary_branch IS NOT NULL AND (endpoint_digest IS NULL)=(remote_name IS NULL) AND step NOT IN ('context_synchronized','closing','integrating','integrated','published','publication_pending','retired','superseded'))`.
  - C2: `action='cleanup' OR (unheld_digest IS NULL AND primary_branch IS NULL AND endpoint_digest IS NULL AND remote_branch IS NULL AND step!='remote_deleting')`.
  - C3: `action!='cleanup' OR (remote_branch IS NOT NULL)=(step IN ('remote_deleted','worktree_removing','worktree_removed','branch_deleted','complete'))`.
  - C4: `remote_branch IS NULL OR (remote_branch='none')=(remote_context_oid IS NULL)`.
  - C5: `step!='remote_deleting' OR remote_context_oid IS NOT NULL`.
  - C6: `action!='cleanup' OR refusal_category IS NULL OR refusal_category='external_change'`.
- `INDEX` (424) is unchanged.

### 1.2 `document_promotions`

Code: `SCHEMA` (promotion/record.rs:237-282) has no `action`. `primary_branch` and `endpoint_digest` exist and are tied to `consented_context_oid` (273). CHECKs 274-276 and 280 tie `pull_request` mode to the branch path.

Choice (v4): `step` gains `'remote_deleting'`; appended columns are `action TEXT NOT NULL DEFAULT 'promote' CHECK(action IN ('promote','cleanup'))`, `unheld_digest`, and `remote_branch`. The cleanup reuses `primary_branch` and `endpoint_digest`. 273 becomes `action='cleanup' OR (…)`, and 275, 276 and 280 gain `action='cleanup' OR`; 274 holds as it is. New CHECKs:

- P1, C1's analogue: `base_oid`, `integration_oid`, `integration_kind`, `publication_category`, `answer_code`, `consented_context_oid`, `situation` and `accepted_context_oid` are null; `context_pushed=0`; both attempts are 0; `context_oid`, `unheld_digest` and `primary_branch` are set; `(endpoint_digest IS NULL)=(remote_name IS NULL)`; the step is not one of `context_synchronized`, `integrating`, `integrated`, `published`, `publication_pending`, `superseded`.
- P2, C2's analogue.
- C3 to C6 verbatim.

### 1.3 Graph, validation, immutables, writers

Code: `save_record` picks `permits` or `branch_permits` by `old.path()` (closure/record.rs:1007-1012). Promotion's `permits` takes the mode (39-66, called at 785-787), and both refuse from a terminal step (closure 71).

Choice:
- **The graph.** `cleanup_permits(self, next, remote)` on both step types:
  - It refuses from a terminal step. It allows `self==next`.
  - With a remote: `confirmed → remote_deleting` and `confirmed → remote_deleted`. Without one: `confirmed → worktree_removing`.
  - `remote_deleting → remote_deleted → worktree_removing → worktree_removed → branch_deleted`.
  - `confirmed | remote_deleting → abandoned`.
  - Never `superseded`. `complete` is written only by `complete_record`'s own SQL (lifecycle_record.rs:1036).
  - Closure's dispatch gains `Cleanup`. Promotion's `save_record` moves to `match old.path()`.
- **`validate_cleanup`.** Closure dispatches at 595-598. Promotion's `validate` dispatches on the mode at 439-442 and moves to `path()`.
  - It checks C1 to C6 in Rust.
  - A cleanup category is allowed only at `confirmed`, `remote_deleting`, `remote_deleted`, `worktree_removing`, `worktree_removed` or `branch_deleted`.
  - It exempts a cleanup record from the `closure_kind` rule at closure/record.rs:587.
- **Immutables.** These are closure/record.rs:990-1027 and promotion/record.rs:774-799, which have no rule like R-T5-3 (M5).
  - Today's equalities stand.
  - For a cleanup record, `context_oid`, `remote_context_oid`, `unheld_digest`, `primary_branch` and `endpoint_digest` equal their insert values.
  - `remote_branch` goes from null to a value only on `confirmed|remote_deleting → remote_deleted`, or on `confirmed → worktree_removing` with no remote, and never changes after.
  - Both `UPDATE`s (closure 1039, promotion 803) gain `remote_branch`.
- **Writers.** Each lands with its caller.
  - `begin_cleanup_record` (Task 8) inserts at `confirmed`, after `parent::existing_record`. Any unfinished record of the item is `parent::busy`, so the begin supersedes nothing (R-33). No cleanup record is inserted refused.
  - `abandon_reserved_cleanup` (Task 9) is the `abandoned`/`external_change` update plus `finish_parent`, as in closure/record.rs:1354-1365.
- **Supersession (R-33).** Closure's `supersedable` is already `Direct`-only (749-753). Promotion's `ParentRecord::supersedable` (991-993), with the inline copies at 626 and 725, gains `path()==Direct`.

### 1.4 The rebuild

Code: the modes stage builds `{SCHEMA}` from `V2_COLUMNS` (closure/record.rs:478-494; promotion/record.rs:325-347), and `validate_schema` compares exactly (497-500; 349-352).

Choice:
- Today's `SCHEMA` becomes the frozen `SCHEMA_V3`, and the modes stage builds `{SCHEMA_V3}`.
- `V3_COLUMNS`: closure is `V2_COLUMNS` plus `integration_mode,action,withdraw_oid,retired_oid,pending_ref,answer_code,consented_context_oid` (32 columns). Promotion is `V2_COLUMNS` plus `integration_mode,answer_code,consented_context_oid,situation,primary_branch,endpoint_digest,accepted_context_oid,context_pushed` (29 columns).
- The markers are `wave_06_closure_cleanup_v1` and `wave_06_promotion_cleanup_v1`.
- A fourth stage follows the modes stage, in the same shape: drop the index, rename to `…_before_cleanup`, `{SCHEMA}`, `{INDEX}`, `INSERT INTO …({V3_COLUMNS}) SELECT {V3_COLUMNS} … ORDER BY rowid`, drop the old table, write the marker.
- Both run inside `migrate_registry`'s first transaction, with unchanged callers.

### 1.5 `RecordPath::Cleanup` and the forced arms (E-23)

`RecordPath` (promotion/lifecycle.rs:179) gains `Cleanup`. `ParentStep` (135) gains `RemoteDeleting`, with arms in:
- `resumable` (156-176);
- both `From` pairs (closure/record.rs:132-171, promotion/lifecycle.rs:710-748);
- `parent_integrated` (bind/lifecycle.rs:1985-1996, `false`, M3).

`ClosureAction` (closure/mod.rs:76-97) gains `Cleanup`, and `PromotionRecord` gains `action`.

| Site | For a cleanup record | Task |
| --- | --- | --- |
| closure/record.rs:256, promotion/record.rs:173 `path()` | `action==Cleanup` → `Cleanup`, first | 7 |
| closure/record.rs:284, promotion/lifecycle.rs:798 `merged_context` | unchanged: both give `context_oid` (L), since C1 nulls the rest | — |
| closure/record.rs:595, promotion/record.rs:439 validate; 1008, 785 permits | `validate_cleanup`, `cleanup_permits` | 7 |
| closure/record.rs:750 `supersedable`; 1129, 1239, 1305, 1337, promotion 866, 909 (`!= Branch`) | unchanged (false, refused) | — |
| closure/record.rs:1616 `deletion_reservation_required`; promotion's default (lifecycle.rs:280) | `Cleanup => remote_name.is_some()`; promotion's default already is | 7 |
| closure/record.rs:1644 `written_closure_holder` | `AND action!='cleanup'` (§6) | 7 |
| closure/record.rs:119, 697; closure/observe.rs:2177 | `Cleanup` arm, unreachable (callers are branch-only): `false` at 119 and 697, digest byte `3` at 2177 | 7 |
| bind/lifecycle.rs:334 `branch_effects`, 437-443 `closure_action` (callers 461, 1237) | `branch_effects` gives no effects for `Cleanup`; `closure_action` returns `Option`, and its two callers answer `recovery_required` on `None` | 7 |
| lifecycle_record.rs:465 `second_role`, 474 `child_allowed` | `Primary` (never minted); `(Cleanup, _) => false` first | 7 |
| lifecycle_record.rs:813 `remote_identity_of`, lifecycle.rs:1206 `reruns_by_table` | unchanged | — |
| remote/reservation.rs:449-459 `closure_holds_context` | `Some((_, Direct \| Cleanup)) => true`, compile-only (§11) | 7 |
| run.rs:375, 525, 665, 2456, 2510, 2543 | unchanged: `continues` keeps cleanup records out of `run_observed` (365-372) and `promote_inner` | — |
| bind/lifecycle.rs:238, 244, 763, 2062 | unchanged | — |
| bind/lifecycle.rs:1331 | `Cleanup` → `cleanup_stopped` (§3) | 8 |
| bind/lifecycle.rs:1978 | `Cleanup` → `false` | 7 |

`continues`:
- `TicketClosureLifecycle::continues` (closure/lifecycle.rs:308-310) becomes `!matches!(action, Withdraw | Cleanup)`.
- `DocumentPromotionLifecycle` overrides it with `action==Promote`.
- Its `begin_record` and `refuse_record` (promotion/lifecycle.rs:957, 995) check it, as closure's do (302-304), and answer `action_mismatch` (M1).

## 2. The Runner's Third Path (Tasks 8, 9)

### 2.1 Entry

Choice: the third path is `run_cleanup_lifecycle<C: CleanupLifecycle, P>`, a sibling of `run_lifecycle` (run.rs:95-109), with its body in a new `promotion/cleanup_run.rs`.
- It shares none of `promote_inner`'s steps. `continues` keeps the other lifecycles off its records.
- It reuses run.rs's `Direct`, `safe` (2342), `boundary` (241), `deletion_error` (2396), `index_exit` and the parts of `ContextCleanup`, each made `pub(super)`.
- `CleanupLifecycle` carries `KIND`, `Record`, `Error` and `Output`. Its methods land with their callers:
  - Task 8: `read_record`, `save_record`, `begin`, `observe(.., own)`, `clear_cancellation`, `complete_record`, `reconcile_completion`.
  - Task 9: `abandon_reserved`.
- There is no `request_cancellation`: `cancel_parent` goes through `cancel_ticket_closure` and `cancel_document_promotion`.
- The outcome is built in cleanup_run.rs, not through `make_outcome`, which is generic over `Lifecycle` (M10).
- `Session` (mutation/progress.rs:293, 521, 687) gains `cleanup_context`.

### 2.2 Order (Part E item 2)

**New request.** It observes, takes the lease and revalidates (as run.rs:557-595). A difference, a blocker or an invalid configuration answers `external_change`, with nothing recorded (CC9). Otherwise `begin_cleanup_record` runs in that lease (as run.rs:596-631); this revalidation is the inserting run's (b).

**Existing record.** `existing_record` (lifecycle_record.rs:348-393) answers `operation_mismatch` on another item or digest. `complete` replays from the record (CC27). An unfinished record then has `clear_cancellation` (run.rs:648), before (a); a terminal one never does, since it refuses one (lifecycle_record.rs:1100-1101).

**(a0) An abandoned record (B1).** It answers `external_change`. First, under the repository lease and in one Immediate transaction, any row of its operation in a reservation phase is ended with `end_parent_row` when `remote_holder_works` (bridges/abandon.rs:629, widened to `pub(in crate::repository)`) finds no live holder. A live holder answers `busy`, naming it. The usual way in is `operation resume`, since the abandon finished the original request.

**(a) Takeover.** It reads the row.
- In a reservation phase at the current generation: `restart_remote_lifecycle` (reservation.rs:1415-1482) with `RemoteOperationTarget::for_context(plan, KIND.action(), …)`, the plan coming from the record's `remote_name` and `primary_branch`.
- At another generation: §4.
- `cancelled` or `completed` with the record unfinished: `recovery_required`.
- After a takeover, `observation_configuration(root, plan)` is frozen and its digest compared with the record's `endpoint_digest` (M7). At `confirmed` a difference is R2. At `remote_deleting` it is §3's endpoint rule.

**(b) Consent re-check.** At `confirmed`, on a run that did not insert, `observe(.., own=Some(op))` runs under the lease. It refuses on another digest, any blocker, or an invalid configuration (R1, or R2 with a row). `own` drops the record from CC3's holder check. The cleanup's own `close`/`promote` row affects no blocker: blockers read the local journal, the comment bindings and the remote snapshot (closure/observe.rs:620-670).

**(c) Reserve.** At `confirmed` with a remote and no row:
- The frozen configuration's digest must equal the record's (else R1). Only then does `configure_endpoints` run (as run.rs:2010-2026), followed by `reserving` (2045).
- `Busy` or `PollYielding` is R4.
- The self-check (R-40) is one transaction: `reservation::owned`, a re-read of the record, and, if the record is terminal, `end_parent_row`, which is R3. Its answer is `external_change` (CC31). A process lost between the reservation and this check is (a0)'s case.

**(d) Checks before the delete.**
- At `confirmed`: `safe(BeforeTransport)`, `boundary(BeforeRemoteDeletion)`, `cancelled()`, then `precheck_head_and_status` and `full_check` against L (cleanup.rs:201, 369). A failure is R2 (CC12).
- At `remote_deleting`: `safe(BeforeTransport)` only (CC21).

**(e) The remote.**
- With R set, `io.delete(.., R, hook)` (§2.3):
  - `Absent` with `remote_deleting` not saved gives `absent` (CC14).
  - `Absent` once it was saved, or `Sent(Deleted)`, gives `deleted` (CC13, K-20).
  - `Moved` or `Sent(Changed)` is R2 (CC15).
  - `Sent(StillPresent)` is R6. `Held` is R8. An error is §3.
- With R unset: `io.advertise`, then `safe(AfterAdvertisement)`. Not advertised gives `none`; advertised is R2 (CC16).
- Then one write: `remote_deleted` with `remote_branch` (R-42).

**(f) A record at `remote_deleting`.** (a), then the second branch of (d), then (e).

**(g) Local retirement.** It runs from `remote_deleted`, or from `confirmed` with no remote, which writes `worktree_removing` with `none` (CC18).
- First comes K-22's check against the record's plan (§4.1): at `confirmed` it is R1, from then on R11.
- A restart re-passes `AfterAdvertisement` (as run.rs:2158-2165). `cancelled()` is checked between steps.
- Then the worktree removal (cleanup.rs:352), `delete_context_branch` at L (477), `require_local_cleanup_absent` (540) and `delete_tracking_ref(plan, …, expected R, op, L)` (572).
- In one transaction: `require_local_cleanup_absent` and `C::complete_record`, followed by `reconcile_completion` (as run.rs:2316-2334).
- Ownership is checked; the working tree's configuration is not. A stop is R7.
- A fence found here switches the same call to the fenced completion (§4).
- This tail is written apart from `finish_steps`, so direct mode is untouched.

### 2.3 `PromotionIo`

Code:
- `delete` (run.rs:32-41) returns `DeleteObservation`.
- `Direct::delete` passes `AfterAdvertisement` after `fresh_advertisement` (155-165).
- `delete_exact` answers `Absent` when the ref is not advertised (transport/remote.rs:318-332).
- `fresh_advertisement` returns the ref list (204-215).

Choice:
- **The new signature.**
  - `delete` gains `before_send: &mut dyn FnMut(Option<Oid>) -> Result<BeforeSend, RepositoryError>`, called with the reference's tip from the connection's advertisement. The reference is `plan.context(kind, item).remote_ref()`.
  - It returns `RemoteDeletion { Absent, Moved(Oid), Held, Sent(DeleteObservation) }`; `BeforeSend` is `{ Send, Absent, Moved(Oid), Held }`.
  - `Direct::delete` on `Send` calls `delete_exact` as today; its own `Absent` maps to `RemoteDeletion::Absent`. `Moved` and `Sent(Changed)` keep the preserving fetch (184-201).
- **The direct hook** is `|_|` the plain `remote_safe_point(AfterAdvertisement)`: `Continue` is `Send`, anything else `Held`. The direct runner maps `Absent` to `Deleted`, `Held` to `Clean::Cancelled` and `Moved` to `Changed`. It keeps run.rs:2141's safe point. Calls and order are unchanged.
- **The cleanup's hook (B2, M6)** always calls `service.remote_safe_point_then(root, owner, AfterAdvertisement, capture)`.
  - In the capture, only for tip `Some(R)` with `Continue`, it re-reads the record's `cancel_requested` and saves `remote_deleting` with `C::save_record`.
  - A decision other than `Continue` is `Held`, whatever the tip. Otherwise `None` is `Absent`, another tip is `Moved`, and `Some(R)` is `Send`.
- **`remote_safe_point_then`** is a new `RepositoryService` method in remote/sync.rs. It is a pass-through to `remote_safe_point_with` (reservation.rs:2031, `pub(super)`, which remote/poll.rs:2554 already calls). reservation.rs is not edited.
- **`advertise`** (new) returns `Result<Option<Oid>, SynchronizationError>`. Its default body is `Err(SynchronizationError::RecoveryRequired)`, which fails closed. `Direct` overrides it with `with_scoped_action_remote` (`SshDirection::Delete`) and the same owned boundary as `delete`.
- **`FakeIo`** (run_tests.rs:363-466) gains a `tip` script (default `Some(expected)`), an `advertisement` script and counters. Its `delete` calls `before_send(tip)` and returns `Sent(deletion)` only on `Send`.
- **The other test implementers (I2, E-40).** Each changes its signature and return type.
  - These 18 pass `before_send` through to their inner implementer:
    - closure/run_tests.rs `CancelAfterPrimary` 1504 and `GuardIo` 1642.
    - closure/run_pull_request_tests.rs `PullRequestIo` 651, `DriveThenStop` 1462, `CancelAtPublication` 1629, `GoneAfterChild` 1868, `PublishesNothing` 1931 and `DeletedAfterChild` 4587.
    - promotion/run_tests.rs `MutatingChild` 747, `MatrixIo` 922, `ConflictIo` 1707, `ConsentIo` 2572, `CancelReleasedIo` 3073, `Quality1BarrierIo` 4036, `Quality2LostReturnCancelIo` 4516, `Quality3LostExecutorIo` 4841, `HandingOffChild` 5610 and `HeldAtReservation` 7179.
  - Each one that returns its own result calls `before_send(Some(expected))` first, as production passes the safe point before sending, and returns `Held` when told to:
    - `DeleteErrorIo` 2353 (then its error).
    - `AmbiguousDeleteIo` 3002 (then its effect and error). Its retry also calls `before_send(Some(expected))`; the direct hook answers `Send` on `Continue` whatever the tip, and the retry then returns `RemoteDeletion::Absent` itself, as `delete_exact` does for a ref that is not advertised (transport/remote.rs:326-332), which the direct runner reads as `Deleted`, as today.
  - Two keep their bodies and call no hook: `Unreadable` 5450 (`unreachable!`) and promotion/run_pull_request_tests.rs `PullRequestIo` 365 (a trap).
  - Inference: the direct-record FakeIo tests now pass `AfterAdvertisement` once more inside `delete`, as production does. Task 9 runs each affected test by name.

### 2.4 `complete_record` for a cleanup record

Code (lifecycle_record.rs:974-1041):
- 990 refuses an owner with no remote. 991 refuses `remote_deletion_required() && owner.is_none()`.
- 995-1004 require the registry's remote to be the record's. 1005-1007 require a row exactly when an owner is given.
- It ends with `finish_parent` (1038) and marks `lifecycle_contexts` (repository.rs:13503).

Choice (I1), for `path()==Cleanup`:
- 991 and 1005 together allow no owner exactly when the row is fenced: `generation != state::generation`, phase `interrupted`. 990 and 995-1004 stay.
- A fenced row is ended by `end_parent_row(tx, id, op)`. That function is extracted from `supersede` (lifecycle_record.rs:436-443: identity check, epoch raised, `cancelled`), and `supersede` calls it.
- The completion marks the item's context only.
- `reconcile_completion` (1044-1068) also accepts the `cancelled` row.

## 3. Stop Mapping (Part E item 3)

"Ended" is one transaction, the capture of `remote_safe_point_then(AfterAdvertisement)` with no connection, running `abandon_reserved_cleanup`: the record goes to `abandoned`, and `finish_parent` gives `cancelled`.
- `acknowledge` writes `completed_step` in that same transaction for both outcomes (reservation.rs:748-762), and `owned()` sees `reserved` or `advertising`.
- So no state exists with the record ended and its row live.

| # | Stop | Code | Record | Reservation |
| --- | --- | --- | --- | --- |
| R0 | revalidation differs (CC9) | `external_change` | none | none |
| R1 | (b)/(c) difference, no row (CC10) | `external_change` | `abandoned` | none |
| R2 | difference with a row; head/full check; `Moved`/`Changed`; advertised with R unset (CC10, 12, 15, 16) | `external_change` | `abandoned` | ended |
| R3 | terminal after reserving (CC31) | `external_change` | already `abandoned` | `end_parent_row`, one transaction |
| R4 | `busy` or `poll_yielding` (CC11) | that code | `confirmed` | none |
| R5 | transport, key, host or configuration error (CC20) | as `item sync` | step kept; category from `deletion_error` | kept |
| R6 | `StillPresent` (CC19) | `push_rejected` | `remote_deleting` | kept |
| R7 | local removal stopped (CC23) | `recovery_required`; way out `request.retry` | step kept | kept |
| R8 | cancellation before `remote_deleting` is saved (CC25) | `cancelled` (130) | `confirmed` | kept if taken |
| R9 | cancellation after it is saved (the boundary at run.rs:133-139) | `cancelled` | `remote_deleting` (CC21) | kept |
| R10 | fence before `remote_deleted` (CC30) | `recovery_required`; `operation.abandon` | unchanged | fenced |
| R11 | K-22 (§4.1) | `recovery_required`; `repo.inspect`, `request.retry` | step kept | as it is |

Errors:
- A `delete` or `advertise` error re-reads the row first: a generation change is R10, else R5.
- `Repository(RecoveryRequired)` with `cancelled()` true is R8 or R9 (run.rs:2130-2139).

Endpoint rule: a live endpoint digest that differs at `remote_deleting` calls `configure_endpoints`. That fences every held row (state.rs:3071-3085), so the answer is R10.

`Stop` (lifecycle_verb.rs:141-157) gains `Fenced(OperationId)` and `ConfigurationChanged { remote, primary }`.

## 4. The Configuration Fence (CC30)

Code:
- `configure_known` interrupts every row in a reservation phase and raises the generation (state.rs:2933-2954).
- `owned_record` (reservation.rs:368) and the restart (1455) refuse another generation.
- `in_the_way` reads `RESERVATION_PHASES` (recovery.rs:2370, 2867), which `interrupted` is not, so the holds lapse (K-21).

Choice:
- (a), or an error, finds `row.generation != state::generation`.
- At `confirmed` or `remote_deleting` the answer is R10.
- At `remote_deleted` or later, (g) runs with no connection and no owner. `complete_record` ends the row with `end_parent_row`, with no owner and no generation check (R-39), subject to §4.1.
- CC3 and CC28 still hold: they read the record (§6).

### 4.1 K-22 (decision 16)

Code:
- `complete_record` (lifecycle_record.rs:995-1004) needs the registry's remote to be the record's. `retire_context` (cleanup.rs:975-986) needs the registry's (remote, primary) to be the evidence plan's, and is (NULL, NULL) with no plan (982-983).
- `remote_polling_state` keeps the two columns null together (remote/state.rs:64) and inserts them null (67).
- The registry's plan is rewritten only by a `configure`: a poll's (remote/poll.rs:1775, 1806-1822), or a lifecycle's or synchronization's `configure_endpoints` (run.rs:2014, remote/sync.rs), at state.rs:3013-3014. `remote select` and `remote remove` write only the configuration file, so the next `configure` brings the change.
- Inference: a plan change always fences any active row first (state.rs:2942-2954), so whenever the plan differs, the row is fenced or absent.

Choice, at the start of (g) and again inside the completion transaction:
- **The check (Revision 3).** It compares the registry's `(remote_name, primary_branch)` with the record's plan, as `retire_context` does:
  - With `remote_name` set, the plan is `(remote_name, primary_branch)`.
  - With none, it is `(NULL, NULL)`, whatever the record's `primary_branch`. That column is the consented primary, which (b)'s digest binds.
  - On a difference nothing local is touched.
- **At `confirmed`** (only a no-remote cleanup reaches (g) there, before it writes `worktree_removing`), a difference is a consent difference: R1 (CC10), with nothing removed and the abandon not needed. R11 applies from `worktree_removing` with no remote, or from `remote_deleted` with one.
- **The answer** is R11, `recovery_required`.
  - Its blocker names the cleanup's operation.
  - Its cause, in the human line and the record's category `remote_deletion_configuration`, reads: "the publication remote or primary branch changed since this cleanup began with `<remote>`/`<primary>` (or with no publication remote); restore it, then send the same request".
  - Its way out is `repo.inspect` (root), then `request.retry` (request ID).
- **After a restore**, the same request finishes.
- **The abandon** refuses (decision 13, §7 `Done`).
- **The repository's hold** is as K-12 says: an active row holds it. Under K-22 the row is fenced or absent, so nothing is held (K-21).

The rule applies to each row:

| Row | What happens |
| --- | --- |
| CC17, CC18 (no remote) | The registry's plan is `(NULL, NULL)`, which equals the record's plan, so the cleanup finishes. |
| CC22 | A re-run at `remote_deleted` or later finishes, unless the registry's plan differs from the record's, which is R11. |
| CC23 | After a local stop, a plan change gives R11 until it is restored. |
| CC29 | At `confirmed` or `remote_deleting`, a change gives CC10 or CC30; with no remote at `confirmed` it is R1 (CC10). From `remote_deleted`, or once a no-remote cleanup wrote `worktree_removing`, it gives R11. |
| CC30 | A fence with an unchanged plan (key selection, key clearing, or an endpoint change of the same remote, state.rs:3044-3051, 3075-3080) finishes. A plan rewritten by the next `configure` after a hand edit, `remote select` or `remote remove` gives R11. |
| No remote (CC18 with CC23) | Once a poll registers a remote, the registry's plan is no longer `(NULL, NULL)`, so R11 until the remote is removed and a poll fences with no plan (poll.rs:1775). |

## 5. `finish_parent`'s Join (the only rule change in remote/reservation.rs)

Code: reservation.rs:1355-1368. The completed joins already match L (1357, 1360), because C1 nulls `adopted_context_oid` and `closure_oid`.

Choice:
- Closure's `CASE` gains `WHEN EXISTS(SELECT 1 FROM ticket_closures c WHERE c.repository_id=?1 AND c.operation_id=?2 AND c.action='cleanup' AND c.step='abandoned') THEN 'cancelled'`.
- Promotion's query becomes `CASE WHEN <today's EXISTS> THEN 'completed' WHEN EXISTS(… p.action='cleanup' AND p.step='abandoned') THEN 'cancelled' END`.

## 6. Holds (Part E item 6)

Code: `written_closure_holder` (closure/record.rs:1635-1658) would read a cleanup row as a branch hold.

Choice:
- **Exclusion (Task 7).** Its second disjunct gains `AND action!='cleanup'`. These then behave as today (R-41): `branch_hold`, `closure_holds_context` (reservation.rs:447), the poll's skips (poll.rs:506, 785), local_sync/record.rs:1161, bind/republish.rs:215 and bridges/repair.rs:473 and 508.
- **The new predicate (Task 9).** `lifecycle_record::cleanup_holder(db, id, kind, item)`: the item's unfinished `action='cleanup'` row, at any step, in either mode.
- **CC3.**
  - The holders of `Close`, `Withdraw` and `Promote` (lifecycle_verb.rs:604-611, 692-698; `unfinished_holder` at promotion/mod.rs:575-585) find a cleanup record once it is never supersedable, and `held` (bind/lifecycle.rs:803-839) names it.
  - The domain refuses it as `busy` (closure/record.rs:874; promotion/record.rs:626, 669).
  - The cleanup verbs' own holders use `unfinished_record` with no supersedable filter.
- **CC28.** `cleanup_holder` is added beside today's holders at authority.rs:326-340 and 442-450, source.rs:205-210 and execution.rs:333-342, in `context_retirement/`.

## 7. Abandon (Part E item 7, CC26, Task 10)

Code:
- `observe` reads the local journal, then the remote journal (bind/abandon.rs:134-156).
- A lifecycle ID reads `NotFound` or `Refused` (reservation.rs:909-947), which answer 789 and 796.
- `abandon_remote_operation` (bridges/abandon.rs:776-852) is the pattern to follow.

Choice:
- **`abandonable_cleanup(root, op)`** returns `NotACleanup`, `Ended`, `Done` or `Pending(CleanupRemains)`.
  - `Pending` means the record is at `confirmed` or `remote_deleting`, or `abandoned` with a row of its operation in a reservation phase (B1).
  - `Done` means `remote_deleted` or later.
  - The remains are the kind, item, step, the row's `(phase, owner_epoch, generation, host, pid)` or none, and the holder `operation_holder(.., Remote, .., Some("context cleanup"))` (records.rs:971).
- **`Found::Cleanup`** is asked first in `observe`.
- **The preview.**
  - `Done` answers `recovery_required` with `same_request`. `Ended` answers `already_applied`. A live holder (`remote_holder_works`) answers `busy`, naming it.
  - Otherwise it gives `OperationAbandonPreviewDto` with `action` `close` or `promote` and `state` the step: `confirmed` (not deleted), `remote_deleting` (may have been deleted) or `abandoned` (the row only). It also gives `step` (the row's), `request_id` and `evidence: null`.
  - The digest binds the operation, kind, step and the row's phase, epoch and generation.
- **`abandon_cleanup(root, op, expected)`** takes `repository_lease` and `cache_write_guard`, as `abandon_operation` does, then runs one Immediate transaction.
  - It re-reads, then answers `external_change` for a changed standing, `Done`, or `busy` for a live holder.
  - Otherwise it moves an unfinished record to `abandoned`/`external_change`.
  - It runs `end_parent_row` on a row in any phase but `completed` or `cancelled`.
  - It finishes the holding request with the stored `external_change` answer, through a new `records::finish_cleanup_request` (on `finish_own_cancel`'s pattern, 794-834; `finish_request` 695). It does not release the request (R-36).
  - With no row the record is marked alone. (c)'s self-check or (a0) ends any later row.
- **`reservation::abandonable`** is unchanged (E-17).

## 8. The Binding (Task 8)

- **`Command::ContextCleanup`** gets every classifier arm (command.rs:76-78, 172, 224, 279, 391, 457, 534, 589, 660, 745) and `setup_gate` `Refused` (324-330, CC4).
- **`Mutation::ContextCleanup`** (mod.rs:697, 757, 801, 879) binds `ContextCleanupBinding`.
  - Its target is the bare ID, with no fields, as for `context retire` (retire.rs:272-280).
  - At prepare and confirm it resolves the kind from an unfinished record of the item, else the index's item, else the local context ref. None gives `item_not_found` (CC1).
  - It then delegates to `LifecycleBinding<CleanupTicket>` or `<CleanupDocument>`.
- **The verbs** (lifecycle_verb.rs:160-197): `COMMAND = ContextCleanup`; `REPORTS_MADE_READY` is true for the ticket, for its slug, as `Withdraw` sets it (614-618).
  - `observe` gives `Observed::Cleanup(Box<CleanupReady>)` or `Blocked`. `call` goes to `session.cleanup_context`.
  - `held` comes before the blocked answer (bind/lifecycle.rs:1776-1791). `names_no_item` (2220) gives CC1.
  - CC5's blockers follow `LifecycleBlocker::ALL`, with `NotDoneOnPrimary` appended (dto.rs:424-450).
  - A proof error other than `ContentMismatch` or `TicketOpen` gives CC6.
- **The observation**, shared by both kinds:
  - closure/observe.rs's context, journal and comment checks (620-670, 1000-1045), and `primary_presence`.
  - `context_behind_remote` and `history_unknown`, skipped when the branch was observed deleted (R-43).
  - `not_done_on_primary` comes from `proof::completion` for a ticket, and from the document's presence by ID for a document.
  - The proof is `proof::preservation(L, local primary)`. `unheld` is empty on a pass; otherwise it is `unheld_entries(L)`, joined by path with R's entries when the branch was observed deleted at R≠L.
  - The digest (CC8) is blake3 under `b"manyhands context cleanup v1"` over kind, item, L, R, the unheld entries, remote, endpoint digest and primary.
- **The command-keyed lists:**
  - recovery.rs:1040 `LIFECYCLE_COMMANDS`.
  - progress.rs:804-809 `asks_a_parent`, and `cancel_parent` (813-831), which resolves the table from the request's remote operation.
  - setup/change.rs:499-519 (`?9`).
  - bind/mod.rs:1080-1096 `recorded_effects`, which resolves the verb from the record, and 1207.
  - bind/lifecycle.rs:2208-2212 `held_by_its_child`, `false`.
  - replay.rs:200-209, `None`. way_out.rs:122-145, its recovery actions only.
  - bind/resume.rs `found_lifecycle`: the promotion branch (307-317, M2) and the closure branch (318-336) name `ContextCleanup` and the kind for an `action=cleanup` record; `lifecycle()` (1336-1371) builds that binding.
  - dto.rs:3406, 3445, 3453 and 3497 `from_stored`.
- **The listing (M9).** read/status.rs lists every unfinished cleanup record as today's code lists lifecycle records (606-631, 661-686), and its attribution names `context cleanup`. It also lists an `abandoned` cleanup record while its row is in a reservation phase (B1's stranded row): the terminal filters at 608 and 663 gain that one exception (`listed` true), and remote_operations' parent skip (722-731) keeps the row from being listed a second time. K-18's amended text belongs to the Cycle document, at D-6.

## 9. CC1 to CC31

| CC | Path | Test (P7) |
| --- | --- | --- |
| 1, 2, 4-9 | §8; R0 | `context_cleanup_prepare_outcomes` (T8) |
| 3 | §6 | prepare_outcomes (own); `context_cleanup_holds_its_item_and_the_repository` (T9) |
| 10-16, 19, 31 | §2.2 (b)-(e); R1-R4, R6 | `context_cleanup_confirm_outcomes` (T9) |
| 17, 18 | (g), §2.4 | prepare_outcomes (no remote), confirm_outcomes |
| 20-23, 25 | R5, (f), (a)+(g), R7, R8/R9, (a0) | `context_cleanup_stops_and_crashes` (T9), with a lost process after reserving |
| 22, 23, 29, 30 (K-22), no-remote | §4.1, R11 | stops_and_crashes: a hand edit plus a poll, `remote select` after a fence, and a no-remote cleanup with `remote select` |
| 24, 28, 29 | the active row; `cleanup_holder`; setup/change.rs | holds_its_item_and_the_repository |
| 26 | §7, including an abandoned record with a dead holder's row | `context_cleanup_abandon` (T10) |
| 27 | replay, resume, listing | confirm_outcomes; abandon |
| 30 | §4 | stops_and_crashes: key selection, key clearing and an endpoint change, before and after `remote_deleted` |

## 10. The Split

Each field or function lands with its first reader (P-1). Only promotion/record.rs has `#![allow(dead_code)]` (line 5).

**Task 7.**
- §1.1 to §1.4.
- `ClosureAction::Cleanup`, `PromotionRecord.action`, `ParentStep::RemoteDeleting` and `RecordPath::Cleanup`, with the Task 7 rows of §1.5.
- `continues`, with M1's checks.
- Promotion's `supersedable`.
- §6's exclusion.
- The fields that `decode`, `validate` and `save_record` read. No writer.
- Edits: E-15's two markers, E-16, E-23, E-27, E-28 (unchanged), and E-39 (M4): promotion/lifecycle_tests.rs `shared_view` (66-85), `promotion_steps_map_to_the_shared_step_view` (88-142), and the literals at closure/record_tests.rs:1885 and promotion/record_tests.rs:1786.
- Test: `cleanup_records_rebuild_once`, which also checks that `written_closure_holder` ignores a cleanup row.

**Task 8.**
- §8.
- `CleanupLifecycle` with its Task 8 methods, both lifecycles, and `begin_cleanup_record`.
- `run_cleanup_lifecycle`: the new request, replay, (b), (g) with no remote, and K-22 for no remote.
- §2.4 without the fenced branch.
- The outcome, `cleanup_stopped`, and `Stop::ConfigurationChanged`.
- With a remote, the run answers `recovery_required` for now, as the plan has it.
- Edits: E-8 to E-12, E-30.

**Task 9.**
- (a0), (a), (c) to (f), and K-22 with a remote.
- §2.3, including `remote_safe_point_then`, `advertise`, `FakeIo` and E-40's 22 implementers. Files added: closure/run_pull_request_tests.rs and promotion/run_pull_request_tests.rs.
- §3, §4, `end_parent_row`, `abandon_reserved`, §5, `cleanup_holder` with the CC28 sites, `reconcile_completion`'s `cancelled`, `Stop::Fenced`, and `remote_holder_works`'s widened visibility.
- It removes Task 8's interim answer.

**Task 10.**
- §7, `finish_cleanup_request`, and the `action` values of `operation_abandon_preview.schema.json`.
- E-17.

**Plan deviations (M8), for the ledger:**
- remote/sync.rs `remote_safe_point_then`.
- promotion/cleanup_run.rs, in place of run.rs.
- mutation/records.rs.
- bridges/abandon.rs's visibility change.
- Two callers of the supersede-style row end beyond the two that Part E item 8 names: (a0) and R3's self-check. R3 checks `reservation::owned` first; neither changes remote/reservation.rs.
- closure/observe.rs:2177.
- The abandon preview schema.
- §6's exclusion moved from Task 9 to Task 7.
- The K-18 filter dropped; K-18's text is amended in the Cycle document at D-6.

## 11. STOP FOR OWNER

None remains. The previous item was resolved by decision 16 (§4.1).

Item 8's triggers:
1. **A change to remote/reservation.rs's rules beyond `finish_parent`'s join: none.** reservation.rs gets §5's join and the compile-only arm at 449-459, which `RecordPath::Cleanup` forces. That arm never runs, because `written_closure_holder` excludes cleanup records; E-23 and the ledger's §5 list it. B1 and R3 end rows through `end_parent_row` in lifecycle_record.rs, which is `supersede`'s body. The hook uses `remote_safe_point_with` through remote/sync.rs. `owned_for_acknowledgement`, the point order and `acknowledge` are untouched.
2. **A new remote action: none.** The cleanup reserves `close` or `promote`.
3. **A transport change: none.** `fresh_advertisement`, `delete_exact` and `observe_delete` are called as today.
4. **The conflict verbs and the key verbs: untouched.**
5. **A row that cannot be met: none.** CC30's gap is K-22 now.

Contract additions to confirm (not triggers):
- `operation_abandon_preview`'s `action` enum gains `close` and `promote`.
- R11's cause is carried in the human line and the record's existing category, so no new field.


## Appendix C: Task 7 checkpoint's independent check and re-check (verbatim)

# Task 7 Design Checkpoint: Independent Check

I read the source at HEAD `42572592`. The checkpoint says it was read at `3a107311`. Since then only Task 6's files have changed (`mutation/dto.rs`, `mutation/mod.rs`, `bind/poll.rs`, `pass_report.rs`, the poll goldens). Every line the checkpoint cites in the files it relies on still matches, apart from the small citation errors in M10. Paths are under `src/repository/`.

Counts: 2 Blocking, 4 Important, 10 Minor. The STOP item holds, and it reaches more rows than it says (section "The STOP item").

## Blocking

**B1. CC31 and R-40: if the process dies right after reserving, the cleanup's record is abandoned while its row still holds the repository, and nothing can end the row.**
Evidence:
- `reserve_remote_operation_kind` never reads the parent's step (`remote/reservation.rs:1599-1808`).
- `restart_remote_lifecycle` refuses a parent that has ended (`reservation.rs:1442-1451`).
- In §2.2 an `abandoned` record answers `external_change`. In §7 `Ended` answers `already_applied`.
- R3 is two transactions, because `remote_safe_point` opens its own (`reservation.rs:2042`) and `finish_parent` runs after it.

Sequence:
1. A run at `confirmed` with no row passes (b).
2. `operation abandon` lands. There is no row, so it marks the record alone.
3. The run's reservation commits.
4. The process is lost before the self-check, or between R3's safe point and `finish_parent`.

The record is now `abandoned`, and its row is `reserved` or `advertising` at the current generation with a dead holder. Every way out fails:
- The same request answers `external_change`.
- `operation abandon` answers `already_applied`.
- `operation resume` finds nothing to resume.
- `in_the_way` (`recovery.rs:2867`) refuses every local write, and every reservation answers `busy`, until some unrelated configuration fence lands.

So CC31's "no cleanup record is terminal while a row of its operation is in a reservation phase" fails, and so does CC24's "until it ... is abandoned".

Correction, needing no change to `remote/reservation.rs`:
- In §7, `abandonable_cleanup` and `abandon_cleanup` treat an `abandoned` cleanup record whose row is in a reservation phase, with no live holder (`remote_holder_works`), as abandonable. The abandon ends the row with `end_parent_row` in its one Immediate transaction, and its preview binds the row's phase, epoch and generation.
- In §2.2, the same request does the same under the repository lease before it answers `external_change`.
- R3 becomes one transaction: re-read the record and, if it is terminal, call `end_parent_row` (or run `finish_parent` inside `remote_safe_point_with`'s capture).

**B2. CC14 fails on a first run: the run never passes `AfterAdvertisement`, so `finish_parent` refuses.**
Evidence:
- `finish_parent` needs `completed_step == Some(AfterAdvertisement)` (`reservation.rs:1351`).
- The cleanup's hook (§2.3) returns `Absent` for tip `None`, and `Moved` for another tip, without calling the safe point.
- §2.2(e) writes `remote_deleted` with no safe point.
- §2.2(g) passes `AfterAdvertisement` again only "on a restart".
- Today's tail passes it after `io.delete` for every outcome (`promotion/run.rs:2141-2146`).

Sequence:
1. The run reserves (`completed_step` is NULL).
2. It passes `BeforeTransport`.
3. The hook sees tip `None` and returns `Absent`.
4. The run writes `remote_deleted`. Step (g) is not a restart, so no safe point is passed.
5. `complete_record` calls `finish_parent`, which answers `recovery_required` and rolls the whole transaction back.

The first answer is `recovery_required` instead of `ok` with `absent`. Only a second run completes, through the restart.

Correction: the hook calls `remote_safe_point_saving(AfterAdvertisement)` for every tip. It saves `remote_deleting` only when the tip is `Some(R)` and the decision is `Continue`, and any decision other than `Continue` maps to `Held` whatever the tip. Alternatively, keep run.rs:2141's safe point after `io.delete`.

## Important

**I1. The fenced completion is blocked by `complete_record`'s owner rule.**
`lifecycle_record.rs:991` refuses when `remote_deletion_required() && owner.is_none()`. The cleanup override in §1.5 makes `remote_deletion_required()` true for every cleanup with a remote. §2.4 relaxes only the rule at 1005 (a row exists exactly when an owner is given). As written, CC30 at `remote_deleted` or later can never complete.
Correction: for `path()==Cleanup`, allow no owner exactly when the operation's row is fenced (`row.generation != state::generation`, phase `interrupted`). Relax 991 and 1005 together, and keep 990.

**I2. Changing `PromotionIo::delete`'s signature touches 22 test implementers, not just `FakeIo`.**
The implementers are listed by `grep "PromotionIo for"`:
- `closure/run_tests.rs`: 1504, 1642.
- `closure/run_pull_request_tests.rs`: 651, 1462, 1629, 1868, 1931, 4587.
- `promotion/run_tests.rs`: 747, 922, 1707, 2353, 2572, 3002, 3073, 4036, 4516, 4841, 5450, 5610, 7179.
- `promotion/run_pull_request_tests.rs`: 365.

Two of those files are outside Task 9's file list. Several of these implementers drive cancellation, ambiguous deletes or lost returns around the delete, for example `AmbiguousDeleteIo`, `Quality2LostReturnCancelIo` and `DeleteErrorIo`. Whether each one calls `before_send` decides whether those tests still test what they test.
Correction:
- Give `advertise` a default implementation that fails closed.
- State the rule each implementer follows: call `before_send(Some(expected))` first, as `FakeIo` will.
- Log the edits as an E-row, run each affected test by name, and add the two files to Task 9's files.

**I3. The split is not clippy-clean as written.**
- Two §1.5 rows can't land in Task 7. The `bind/lifecycle.rs:1331` row maps to `cleanup_stopped`, from §3 and Task 8/9. The `run.rs:2543` row is "the cleanup's own outcome", which needs `C::outcome` from Task 8. Task 7 should take only the compile-forced arms and the comparisons whose value needs no new code, and move those two rows to Tasks 8 and 9. (`run.rs:2456`'s contexts `[Item]` is a plain comparison and can stay in Task 7.)
- `CleanupLifecycle` declares methods whose first caller comes later: `abandon_reserved` (Task 9), `reconcile_completion` (§10 puts it in Task 9), and `request_cancellation`, which nothing calls because `cancel_parent` goes through `cancel_ticket_closure` and `cancel_document_promotion`. Those raise `dead_code` warnings under `-D warnings`. Only `promotion/record.rs` has `#![allow(dead_code)]` (line 5). Add each method with its caller, and drop `request_cancellation` from the trait.

**I4. The STOP item's list of triggers is incomplete, and other rows share the problem.**
Details are in "The STOP item" section below.

## Minor

**M1. The promote half of §1.5's `continues` claim is false.**
Only closure's `begin_record` calls `continues` (`closure/lifecycle.rs:302-304`). Promotion's (`promotion/lifecycle.rs:957-970`) returns any existing record from `existing_record`. A promote request bound to a cleanup record's operation ID and expectation would continue down the direct path at `run.rs:665`. The digest mismatch makes this unreachable in practice. Fix: check `continues` there too, failing closed.

**M2. `operation resume` needs the discriminator in both tables.**
`bind/resume.rs` `found_lifecycle` names `DocumentPromote` for every promotion record (307-317). The `action=cleanup` discriminator is needed there as well as in the closure branch the checkpoint cites (318-336).

**M3. One exhaustive `ParentStep` match is missing from §1.5.**
`mutation/bind/lifecycle.rs:1985-1996` (`parent_integrated`) needs a `RemoteDeleting` arm. §1.5 lists only `resumable` and the two `From` pairs.

**M4. Test edits not listed in E-16, E-27 or the checkpoint.**
- `promotion/lifecycle_tests.rs` `shared_view` (66-85, an exhaustive match).
- `promotion/lifecycle_tests.rs` `promotion_steps_map_to_the_shared_step_view` (88-142: a struct literal and a step list without `remote_deleting`).
- The struct literals `closure/record_tests.rs:1885` and `promotion/record_tests.rs:1786`.

**M5. Promotion's save does not freeze `remote_context_oid`.**
Promotion has no rule like R-T5-3, so its `save_record` lets `remote_context_oid` go from None to Some (`promotion/record.rs:797`). For a cleanup record, make `context_oid` and `remote_context_oid` fully immutable from insert, not just "once set".

**M6. The hook's save must preserve the record's cancellation flag.**
`save_record` refuses a changed `cancel_requested` (`promotion/record.rs:792`, `closure/record.rs:1014`). The hook should re-read the flag in the transaction, as `save` does (`run.rs:233-237`).

**M7. The takeover path doesn't say where step (e)'s `ObservationConfiguration` comes from.**
After (a) takes the row over at `confirmed`, step (c) is skipped. Compute `observation_configuration(root, plan)` there and require its digest to equal the record's, else R2. No `configure_endpoints` call is needed.

**M8. Changes outside the plan's Tasks 7 to 10 file lists.** None of them changes a rule.
- The `remote/sync.rs` wrapper `remote_safe_point_saving`. It is built on `remote_safe_point_with`, which is `pub(super)` (`reservation.rs:2031`) and already called from `remote/poll.rs:2554`. It leaves `owned_for_acknowledgement`, the point ordering and `acknowledge` unchanged.
- The new `promotion/cleanup_run.rs` (the plan says `promotion/run.rs`).
- `mutation/records.rs` (`finish_cleanup_request`).
- `read/status.rs` (the K-18 filter).
- `closure/observe.rs:2177` (a compile-forced arm).
- `schemas/v1/operation_abandon_preview.schema.json`.
- §6's exclusion also moves from Task 9, where the plan puts it, to Task 7.

Record all of these as plan deviations in the ledger.

**M9. The K-18 filter takes away a listing today's code would give.**
Today `read/status.rs:606-631` and `661-686` list every unfinished lifecycle record, whether or not it has a row. CC27 requires only the rows that have one. The filter is optional; drop it, or give the reason for it.

**M10. Small fixes and citation errors.**
- `cleanup_permits` should refuse when the current step is terminal, as `permits` does (`closure/record.rs:71`).
- Say what the "fail closed" arm in the `closure/observe.rs:2177` digest returns (a distinct byte).
- `make_outcome` is generic over `L: Lifecycle`, so `C::outcome` can't be called from it. Build the outcome in `cleanup_run.rs`.
- `restart_remote_lifecycle` is at lines 1415-1482, not 1415-1517.
- Promotion's `endpoint_digest` column is at line 264, not 263.

## The STOP item (CC30 at `remote_deleted` or later)

The item holds.
- `complete_record` refuses unless `remote_polling_state.remote_name` equals the record's (`lifecycle_record.rs:995-1004`).
- `retire_context` refuses unless that row's remote and primary equal the evidence plan's (`cleanup.rs:975-986`).
- The poll's unreadable-configuration fence calls `configure(None, Unavailable)` (`remote/poll.rs:1775`), which sets both to NULL (`remote/state.rs:3013-3014`).
- Key selection and key clearing fence with the current plan (`state.rs:3044-3051`).
- An endpoint change with the same remote and primary is a `Credentials` change, so it keeps the plan (`state.rs:3075-3080`). An endpoint change that comes with a changed plan is a `Plan` change, and it rewrites the remote and primary.

Option A, a new known limit, is consistent with the guard. The K-22 text should be wider than the item says:

1. **A poll fences and rewrites the plan before it reserves** (`poll.rs:1806-1822`, `configure_poll_endpoints`). A hand edit of `.manyhands/config.toml`, or a checkout at the root of a branch with another remote or primary, followed by any poll (the desktop's automatic polls included), fences an active cleanup row that was never fenced and rewrites the registry's plan in one step. CC29's refusal of `remote select` and `remote remove` does not stop that.
2. **CC22 and CC23 at `remote_deleted` or later are the same case.** So is a live step (g) fenced in the middle of its run (I1's path).
3. **A cleanup with no publication remote that stopped during its local removal (CC18 with CC23) is stuck with no fence at all.**
   - It holds no row, so a direct `remote select` proceeds, as CC29 says.
   - The next poll registers the new remote (`state.rs:3013`).
   - From then on `complete_record:1002` and `retire_context:982` refuse every re-run, and `operation abandon` refuses too (`Done`).
   - CC29's "the next run ends refused (CC10) or meets a fence (CC30)" is not met.
   - The way out is to run `remote remove` and then a poll, which fences with no plan (`poll.rs:1775`).

No other row has the problem: the R2 and R3 ends and the abandon read no plan.

## Verified as correct

- **The rebuilt `ticket_closures` table:**
  - CHECK 404 is the `closure_kind IS NOT NULL OR step='abandoned'` rule.
  - CHECKs 414 to 417 and 422 are rewritten as stated.
  - C1 to C6 do what the checkpoint says. C1, together with the existing `(closed_at IS NULL)=(closure_oid IS NULL)` CHECK, nulls `closure_oid`.
  - The new columns are nullable with no default, so E-28's raw inserts survive.
  - The completed join of `finish_parent` already matches L, because C1 nulls `adopted_context_oid` and `closure_oid`.
- **The rebuilt `document_promotions` table:** 273, 275, 276 and 280 need the `action='cleanup' OR` prefix; 274 and 281 hold as they are. `action` defaults to `promote`.
- **The migration and its markers:**
  - `V3_COLUMNS` are 32 and 29 columns.
  - The stage order means a fresh table skips every rebuild, and a table at an older version rebuilds through `SCHEMA_V3`.
  - A missing modes marker with the cleanup marker present fails `validate_schema` closed.
  - E-15's marker list sorts as expected.
  - E-16's tests are the ones that delete markers or compare `SCHEMA`: `closure/record_tests.rs:93,103,1782,1927,2011` and `promotion/record_tests.rs:1520,1664,1717`.
- **Paths and supersession:**
  - The `RecordPath` mentions match the ledger's audit.
  - `written_closure_holder` builds its path from SQL, so the `Cleanup` arm in `closure_holds_context` is compile-only.
  - Closure's `supersedable` already requires `Direct`.
  - The `ClosureAction` match sites listed are complete; every other match has a wildcard.
  - `child_allowed` and `second_role` behave as stated.
  - `remote_identity_of` expects `KIND.action()` for a cleanup.
  - `deletion_reservation_required` needs the override; promotion's default already gives `remote_name.is_some()`.
- **R-34 (CC10):** the takeover happens before the record ends, so `restart_remote_lifecycle`'s ended-parent check passes. `AfterAdvertisement` passed with no connection leaves the row active, with `completed_step` set, whether it continues or is cancelled (`reservation.rs:748-762, 805`). `abandon_reserved_new_branch`'s pattern (`closure/record.rs:1354-1365`) then works in one transaction.
- **R-35:** the hook's save can run inside the `AfterAdvertisement` safe point's own transaction, and it is skipped when the stop is cancelled.
- **Fences, the abandon and the holds:**
  - Fence detection by generation is right (`state.rs:2954`, `reservation.rs:368-371`, `1455`).
  - `supersede`'s ending of a row (`lifecycle_record.rs:436-443`) is the right body for `end_parent_row`.
  - The abandon's pattern matches the code it cites: `abandon_remote_operation`, `remote_holder_works`, `operation_holder` with the Remote family, `finish_own_cancel` and `finish_request`.
  - CC3 works through `unfinished_holder` and `held`.
  - The CC28 sites are the four cited in `context_retirement/`.
  - CC29: `setup/change.rs:481-497` also refuses for any unfinished record.
- **No change to the transport, the conflict verbs or the key verbs, and no new remote action.** `fresh_advertisement` returns the ref list (`transport/remote.rs:204-215`), so the tip can be read in `run.rs` without touching the transport.

## Re-check of revision 2

Scoped to revision 2 of task-7-design.md and decision 16, at HEAD `42572592`. Paths are under `src/repository/`.

Counts: 1 Blocking, 0 Important, 6 Minor. Once R1 (the K-22 comparison) is corrected, the checkpoint can be approved; the Minor items can be fixed in the text or at implementation.

### Findings

**R1 (Blocking). The K-22 check (§4.1) refuses every cleanup with no publication remote.**
- `remote_polling_state` has `CHECK ((remote_name IS NULL) = (primary_branch IS NULL))` (`remote/state.rs:64`), and its row is inserted with both NULL (`state.rs:67`). So in a repository with no remote, the registry's primary is NULL.
- C1 requires a cleanup record's `primary_branch IS NOT NULL`.
- §4.1 compares the registry's remote and primary with "the record's `remote_name` and `primary_branch`". For a no-remote cleanup that comparison always differs, so every such cleanup answers R11. CC17 with no remote and CC18 fail.

Correction: compare with the record's plan, as `retire_context` does (`cleanup.rs:982-983`). The registry's (remote, primary) must equal (`remote_name`, `primary_branch` only when `remote_name` is set). With no remote, that means both registry columns are NULL.

Also state the case at `confirmed` with no remote. There (g)'s check gives R11 with the record still at `confirmed`, where abandon is available, while §4.1's CC29 row says CC10. Either refuse as CC10 or accept R11 there, and say which.

**R2 (Minor). "Then `clear_cancellation`" sits inside (a0), the abandoned-record path.**
`clear_cancellation` refuses a terminal record (`lifecycle_record.rs:1100-1101`), so implemented as placed, an abandoned record would answer `recovery_required` instead of `external_change`. Move the sentence to the unfinished-record flow, before (a).

**R3 (Minor). `AmbiguousDeleteIo`'s retry (§2.3) does not get `Absent` from the hook.**
The direct hook ignores the tip and answers `Send` on `Continue`, so `before_send(None)` does not give `Absent`. The retry should take `Send` and then return `RemoteDeletion::Absent` itself, as `delete_exact` does for a ref that is not advertised (`transport/remote.rs:326-332`).

**R4 (Minor). `closure_action` cannot take a `Cleanup` arm as worded.**
`closure_action` (`mutation/bind/lifecycle.rs:437-443`) is infallible, so "the DTO maps answer `recovery_required`" has no place to happen. Give it a concrete shape: for example, `closure_action` returns `Option`, its callers at 461 and 1237 answer `recovery_required` on `None`, and `branch_effects` (334) gives no effects for `Cleanup`.

**R5 (Minor). The B1 state is invisible to `operation list`.**
- An `abandoned` record is filtered out as terminal (`read/status.rs:608, 663`).
- Its row is skipped because it belongs to a parent (`read/status.rs:722-731`).

So the stranded row CC27 speaks of ("lists a cleanup that has a reservation row") is listed nowhere; only `--id` and `in_the_way`'s blocker reach it. Correction: list an `abandoned` cleanup record while its row is in a reservation phase.

**R6 (Minor). Two wording and ledger points.**
- (a0) and R3 add callers of the supersede-style row end beyond the two that Part E item 8 names (the abandon and `complete_record`). Neither changes `remote/reservation.rs`, and R3 checks `reservation::owned` first, but list both under M8's deviations.
- §4.1's "`remote select` and `remote remove` rewrite them (state.rs:3013-3014)" is inaccurate. Those verbs write only the configuration file. The registry is rewritten by the next `configure` (`poll.rs:1775, 1822`, `run.rs:2014`, `remote/sync.rs`), which leads to the same outcome.
- Amending K-18 is a change to the Cycle document's text, not to Task 14's contract and skills files; record it for D-6.

### Verified

- **B1 is resolved through the abandon.**
  - `abandonable_cleanup`'s `Pending` covers an `abandoned` record whose row is in a reservation phase.
  - `abandon_cleanup` ends that row with `end_parent_row` in its one Immediate transaction. `end_parent_row` is `supersede`'s body (`lifecycle_record.rs:436-443`): identity check, epoch raised, `cancelled`, on any phase.
  - A live holder answers `busy` (`remote_holder_works`, `bridges/abandon.rs:629-651`). The reserving run's row carries this process's host and pid (`reservation.rs:1626, 1802`; the restart at 1424 and 1477).
  - R3 is now one transaction: `owned`, re-read, `end_parent_row`.
  - R2's capture of `remote_safe_point_then` runs `abandon_reserved_cleanup` and `finish_parent` after `acknowledge`, in the same transaction (`reservation.rs:2042-2116`). `finish_parent`'s `owned()` sees `reserved` or `advertising` (748-762, 805).
  - (a0) by the same request is reached mainly through `operation resume`: the abandon finishes the original `context cleanup` request, so sending it again replays its stored answer and never reaches (a0).
- **B2 is resolved.** The cleanup's hook passes `AfterAdvertisement` for every tip and saves only at `Some(R)` with `Continue`, so `finish_parent`'s `completed_step` check (`reservation.rs:1351`) holds on a first run that finds the branch absent. M6 (re-reading the cancellation flag) is applied.
- **I1 is resolved and scoped.** For `path()==Cleanup` only, the rules at 991 and 1005 allow no owner exactly when the row is fenced. 990 and 995-1004 stay. With no owner, no row and a remote, the record is still refused. No other record is loosened.
- **I2 is resolved.** The 22 implementers match each `delete` body:
  - 18 delegate to `inner.delete`.
  - `DeleteErrorIo` and `AmbiguousDeleteIo` return their own result.
  - `Unreadable` is `unreachable!`, and promotion's `PullRequestIo` (`run_pull_request_tests.rs:617-629`) returns `RecoveryRequired`.
  - `advertise`'s default fails closed.
- **I3 is resolved.**
  - Task 7 takes only arms and comparisons that need no new function.
  - Each `CleanupLifecycle` method has its caller in its task: `observe(.., own=Some)` in Task 8's (b), `abandon_reserved` in Task 9's R2.
  - `end_parent_row` is used from Task 9 through `supersede`.
  - `Stop::ConfigurationChanged` lands with Task 8's no-remote K-22, `Stop::Fenced` with Task 9.
  - With only `promotion/record.rs` allowing dead code, no `dead_code` warning remains.
- **Decision 16 is written into every row it governs.** §4.1 applies it to CC22, CC23, CC29, CC30 and the no-remote case. The answer is `recovery_required` with the cause named in the human line and the existing `remote_deletion_configuration` category. Its way-out actions `repo.inspect` and `request.retry` exist (`results.rs:408, 415`). The same request finishes after a restore. Abandon stays refused (`Done`). `retire_context` is not relaxed. The inference that any plan change fences every active row first holds (`state.rs:2942-2954`).
- **No new error code or field, no change to the transport, the conflict verbs or the key verbs.** The only changes to `remote/reservation.rs` are §5's join and the compile-only arm. `remote_safe_point_then` passes through to `remote_safe_point_with`.
- **Dropping the K-18 filter is sound.**
  - Every unfinished cleanup record is listed as other lifecycle records are (`read/status.rs:606-631, 661-686`), which CC27 permits.
  - With a row, the attribution reads the row's command from its linked request (`recovery.rs:1168`), which is `context cleanup`.
  - `author_for` drops the author once `LIFECYCLE_COMMANDS` holds the command (`recovery.rs:1031`).
