# Concurrent writers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Attribute every operation, make a second writer wait for a live peer and then answer `busy` naming the holder, name the blocker in every refusal, remove the two states nothing can clear, and scope the hold per item.

**Architecture:** Additive nullable attribution columns on both operation tables (one named migration) are filled where a row is created and where a request is linked to it. A pure liveness function with an injected process probe feeds three consumers: the operation DTOs, `in_the_way` (wait, then `busy` or `recovery_required`, both carrying a blocker), and `repo inspect`. Two sync repairs and a confirmed remote abandon remove the unclearable states. The per-item scope comes last.

**Tech Stack:** Rust, rusqlite (registry), libc and windows-sys (already dependencies), JSON schemas under `schemas/v1/`.

**Spec:** `docs/plans/2026-11-18-concurrent-writers-design.md` (approved 2026-11-18). Ticket `01M7NMJKJG35F3C51JF7EVDSRP` (vh-bhepg); reproduction results are in its first comment.

## Global Constraints

- Run Rust only as `devenv shell -- cargo <cmd>` with `CARGO_TARGET_DIR=/home/vhodges/work/src/manyhands/target`.
- Between steps run only the tests you wrote or changed, by exact name (`--lib <module>::<test> -- --exact` or `--test <target> <name> -- --exact`). Never a whole SSH target between steps. The static gates, then the fast tier, then the SSH tier by changed path, run once at review-ready (AGENTS.md "Development" and "Testing Strategy").
- SSH tier targets this work touches: `src/repository/remote/` -> `ssh_transport`, `remote_observation`, `remote_synchronization`, `remote_merge_recovery`, `remote_polling`, `cli_remote`; `src/repository/mutation/` -> `mutation_remote`.
- No real waits in tests: inject the clock and the sleeper. At most one SSH case (Task 9).
- JSON contract: schemas have `additionalProperties:false`; every new field is an optional/nullable addition to schema version 1, and every new or changed field is added to its schema in the same task. Keep the schema-vs-DTO drift tests green.
- The registry migration is one-way: never run a build from this branch against the real registry (`~/.local/share` or the owner's data dir). Scratch registries only (isolated `HOME`, `MANYHANDS_DATA_DIR`, `GIT_CONFIG_GLOBAL`).
- Merge `origin/main` into the ticket worktree (never rebase) at the start of each task if main moved; commit and push the ticket branch as you go.
- Manyhands content (tickets, comments, documents) is one line per paragraph, no hard wrapping.
- Commit trailer: `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.

## Review Focus

- A reused or unreadable pid must never make a stale row look live and must never lead to a takeover; unknown host or pid is stale (Task 4).
- A registry created by an older build, with rows and no attribution columns, must migrate and list with nulls, and an older row must still resume (Task 3).
- A `busy` answer must leave no operation, and retrying the same request id after the holder finishes must complete (Task 6).
- `operation resume` and `operation abandon` on a row a live process owns must be refused with the blocker, not executed (Tasks 6 and 8).
- A confirmed abandon must show the remote evidence (ref present or absent, local and remote oids) before the person confirms (Task 8).

---

### Task 1: Retry an unreturned first push

**Files:**
- Modify: `src/repository/remote/sync.rs` (`push_absence`, ~7079; boundary at ~6877; consumers ~6242-6263, ~6651)
- Test: `src/repository/remote/sync_tests.rs` (existing `an_unreturned_first_push_with_no_publication_evidence_is_retryable_not_ambiguous`, aec7f69c; existing ~5555 test)

**Interfaces:**
- Produces: `push_absence` ignores an own intent whose checkpoint is `PushPrepared`, with no publication attempt or authority, when `fetch_publication_evidence_for(kind, item) == NeverPublished`.

- [ ] **Step 1: Run the failing test.** `--lib repository::remote::sync::tests::an_unreturned_first_push_with_no_publication_evidence_is_retryable_not_ambiguous -- --exact`. Expected: FAIL on row (1).
- [ ] **Step 2: Implement** the narrowed exemption in `push_absence`, reusing the evidence check the `retired` filter already uses. `PushReturned`, an intent with evidence, and any non-own row stay ambiguous.
- [ ] **Step 3: Run** the test above and the existing test at sync_tests.rs ~5555 (find by `ambiguous` in its name). Expected: PASS both.
- [ ] **Step 4: Commit** "Retry an unreturned first push on a never-published context".

### Task 2: Backfill `expected_oid` after a restart

**Files:**
- Modify: `src/repository/remote/sync.rs` (main path after the reconcile, ~6372-6379; model on `republish_reserved` ~7626)
- Test: `src/repository/remote/sync_tests.rs`

**Interfaces:**
- Produces: a small function `backfill_expected_oid(evidence: &mut SynchronizationEvidence, local: Oid)` (use the real evidence type's name) that sets `expected_oid` with `get_or_insert(local)`, called after the reconcile on the main path.

- [ ] **Step 1: Write the failing test** `a_restarted_row_without_expected_oid_can_checkpoint_push_prepared_after_backfill`: seed a context row at `fetch_observed` with `expected_oid` NULL and the local head one commit ahead of tracking; assert `checkpoint_synchronization(PushPrepared)` is refused before the backfill (`RecoveryRequired`) and succeeds after `backfill_expected_oid`; assert an existing `expected_oid` is not overwritten.
- [ ] **Step 2: Run** it by exact name. Expected: FAIL (function missing).
- [ ] **Step 3: Implement** the function and the call; decide whether to also write the skipped `FetchPrepared` on restart, and record the decision in the report.
- [ ] **Step 4: Run** the test. Expected: PASS. Commit.

### Task 3: Attribution storage and migration

**Files:**
- Modify: `src/repository/recovery.rs` (`operation_records` creation ~597, `begin_or_reconcile` ~764, migration pattern ~67), `src/repository/remote/reservation.rs` (remote row creation), `src/repository/mutation/records.rs` (`request_operations` link, ~103), `src/repository/mutation/execute.rs` (command and author known here)
- Test: `src/repository/recovery.rs` tests (use `tests/support/operations.rs::insert_local` only for seeding) and `src/repository/mutation` tests

**Interfaces:**
- Produces, on both tables, nullable columns `request_ulid TEXT`, `command TEXT`, `author TEXT`, `host TEXT`, `pid INTEGER`, `started_at INTEGER` (unix seconds); on `operation_records` also `comment_id TEXT`; `item_id` is now written for `save_ticket` and `submit_comment` rows. Migration name `cycle_operation_attribution` (builder may adjust) in `registry_migrations`.
- Host, pid and `started_at` are written at row creation (`started_at` from the injected `Clock`). Request id, command and author are written where `request_operations` links the request to the row, so no new plumbing is needed through the domain calls. The later `synchronize_context` row is linked to the same request when the comment's publication reserves it.

- [ ] **Step 1: Write failing tests**: `a_legacy_registry_migrates_attribution_columns_and_old_rows_keep_null` (build a registry with the old tables and one pending row, open with the new code, assert the columns exist, the row is untouched, and it still resumes); `a_new_authoring_row_records_request_host_pid_item_and_comment` (save a ticket and add a comment through the mutation API with a test clock; assert every column, and that the sync row for the comment carries the same request id).
- [ ] **Step 2: Run** them by exact name. Expected: FAIL.
- [ ] **Step 3: Implement** the named additive migration (idempotent, same pattern as `cycle_07_comment_publication_bindings`), the writes, and the link-time fill.
- [ ] **Step 4: Run** the tests. Expected: PASS. Commit.

### Task 4: Liveness

**Files:**
- Create: `src/repository/liveness.rs` (register in `src/repository.rs`)
- Test: same file, `#[cfg(test)]`

**Interfaces:**
- Produces: `pub enum Liveness { Live, Stale }` and `pub fn row_liveness(row: &AttributionRow, this_host: &str, probe: &dyn ProcessProbe) -> Liveness`, with `pub trait ProcessProbe { fn exists(&self, pid: u32) -> bool }`, a real `SystemProbe` (libc `kill(pid, 0)` treating EPERM as existing on Unix, `OpenProcess` on Windows), and `pub fn this_host() -> String`. Live only when host equals `this_host`, pid is known and the probe says it exists; anything unknown is `Stale`. The existing indexing heartbeat rule still makes an indexing row live.

- [ ] **Step 1: Write failing table test** `liveness_is_live_only_for_a_known_host_and_existing_pid`: rows (same host, existing pid) Live; (same host, missing pid) Stale; (other host) Stale; (host null) Stale; (pid null) Stale; (fresh indexing heartbeat, missing pid) Live. Plus `the_system_probe_sees_this_process_and_not_a_free_pid`.
- [ ] **Step 2: Run** by exact name. Expected: FAIL (module missing). **Step 3: Implement. Step 4: Run.** Expected: PASS. Commit.

### Task 5: Show attribution in operations and requests

**Files:**
- Modify: `src/repository/read/dto.rs` (`OperationDto` ~833), `src/repository/read/status.rs` (`local_operations` ~123, `list_operations` ~767, `show_operation` ~811, request show), `schemas/v1/operation.schema.json`, `operation_list.schema.json`, `operation_mutation.schema.json`, `request_operation.schema.json`
- Test: an existing CLI contract target (extend, do not add a target): the `cli` target module that holds the operation golden, and `tests/read_status.rs`

**Interfaces:**
- Consumes: Task 3 columns, Task 4 `row_liveness`.
- Produces: nullable `request_id`, `command`, `author`, `host`, `pid`, `started_at`, `comment_id`, and `live` (bool, null when the row is not provably attributed) on every operation DTO; `request show` lists every operation the request started, in start order.

- [ ] **Step 1: Write failing tests**: extend the `operation list`/`show` golden with the new fields for a seeded attributed row and a legacy null row; `request_show_lists_the_sync_the_comment_started`.
- [ ] **Step 2: Run** by exact name; Expected: FAIL. **Step 3: Implement** DTO, queries and schemas. **Step 4: Run** the tests and the schema drift test for these schemas. Expected: PASS. Commit.

### Task 6: Wait for a live peer, then `busy` with the blocker

**Files:**
- Modify: `src/repository/recovery.rs` (`in_the_way` ~1471, `recovery_required` ~2336), `src/repository/mutation/outcome.rs` (busy and recovery_required mapping, `offering_abandon` ~1450-1530, `RecoveryActionKind` ~826), `src/repository/mutation/bind/resume.rs` (refuse a live-owned row), `src/results.rs` (data on these answers), `schemas/v1/envelope.schema.json`, `recovery_action.schema.json`; Create: `schemas/v1/blocker.schema.json`
- Test: `tests/mutation_replay/reenter.rs` (model on `a_concurrent_duplicate_is_busy_and_a_third_call_finishes_the_record`, ~982), `src/repository/recovery.rs` tests

**Interfaces:**
- Produces: `pub struct WaitPolicy { pub budget: Duration, pub poll: Duration }` (default 5 s and 50 ms) with an injectable sleeper and clock on the service; `BlockerDto { operation_id, family, action, item_id, request_id, author, host, pid, started_at, live }` carried as `data.blocker` on `busy` and `recovery_required`; `recovery` gains an `operation.show` action naming `operation_id`.
- A live blocking row: `in_the_way` polls up to `budget`, then returns `busy` with the blocker and `request.retry`. A stale row: `recovery_required` with the blocker and the existing resume/abandon actions. Neither creates a row.

- [ ] **Step 1: Write failing tests**: `a_second_writer_waits_for_a_live_peer_then_answers_busy_naming_it` (writer A parked in its operation; B with another request id and an injected zero-cost sleeper; assert `busy`, `data.blocker.operation_id/request_id/pid/live=true`, `request.retry`, no new operation row); `a_busy_request_completes_when_retried_after_the_holder_finishes` (same request id); `a_stale_row_answers_recovery_required_naming_the_operation` (seed with a dead pid); `resume_of_a_live_owned_row_is_refused_with_the_blocker`; `a_peer_that_finishes_inside_the_wait_lets_the_second_writer_complete` (sleeper advances and completes A).
- [ ] **Step 2: Run** each by exact name; Expected: FAIL. **Step 3: Implement. Step 4: Run** the tests and the `mutation_replay` contract tests that assert the old canned `recovery_required` shape (update them where the blocker is now present). Expected: PASS. Commit.

### Task 7: `repo inspect` lists unfinished operations

**Files:**
- Modify: `src/repository/read/admin.rs` (`inspect_repository` ~273), `src/repository/read/dto.rs` (`RepositoryInspectionDto` ~153), `schemas/v1/repository_inspection.schema.json`; reuse `pending_for_root` (`recovery.rs` ~2195) and the Task 5 operation DTO
- Test: `tests/read_repository.rs`

**Interfaces:**
- Produces: `operations: [OperationDto]` (unfinished local and remote rows of the root, with attribution and `live`) on the inspection data; the `repo.inspect` recovery hint stays.

- [ ] **Step 1: Write failing test** `repo_inspect_lists_unfinished_operations_with_attribution` (seed one live and one stale row). **Step 2: Run** by name; FAIL. **Step 3: Implement. Step 4: Run;** PASS. Commit.

### Task 8: Confirmed abandon of a remote operation

**Files:**
- Modify: `src/repository/mutation/bridges/abandon.rs` (abandon preview and confirm; remote refusal), `src/repository/remote/reservation.rs` (`acknowledge_refused_push` pattern, ~1834), `schemas/v1/operation_abandon_preview.schema.json`
- Test: `tests/mutation_replay/resume.rs` or `tests/mutation_bridges/` (an existing module), `src/repository/remote/sync_tests.rs`

**Interfaces:**
- Consumes: Task 5 attribution and `live`.
- Produces: `operation abandon --prepare` on a remote operation returns a preview with `evidence { remote_ref_present, local_oid, remote_oid, push_oid }`; `--confirm` marks the row's push attempt `protocol_rejected` and closes the operation. A row with `live = true` is refused with the blocker.

- [ ] **Step 1: Write failing tests**: `abandon_of_a_remote_push_prepared_row_previews_the_evidence_then_clears_it` (assert the new sync is not ambiguous afterwards); `abandon_of_a_live_owned_remote_row_is_refused`; `abandon_preview_reports_the_remote_ref_when_the_push_landed`. **Step 2: Run;** FAIL. **Step 3: Implement. Step 4: Run;** PASS. Commit. Also update `skills`-facing `recovery`/autonomy only in Task 11.

### Task 9: One SSH case: an interrupted first push resumes to published

**Files:**
- Modify/Test: `tests/cli_remote/interruption.rs` (existing SSH interruption target; extend it, do not add a target)

- [ ] **Step 1: Write failing test** `a_publication_killed_after_push_prepared_resumes_to_published`: use the target's existing kill-at-a-step support to stop a `comment add` publication at `push_prepared` for a never-published ticket branch, then `operation resume`; assert `published`, the branch on the remote, `operation list` empty. This needs SSH because the behaviour is a real first push to a remote that lacks the branch. **Step 2: Run** by exact name (single case); on a tree without Task 1 it fails with `recovery_required`. **Step 3:** no new code expected; fix only what the case exposes. **Step 4: Run;** PASS. Commit.

### Task 10: Per-item scope of the hold

**Files:**
- Modify: `src/repository/recovery.rs` (`in_the_way` second check, `require_refresh_can_begin` ~1543, `require_no_pending_local` ~729), `src/repository/remote/reservation.rs` (~1283-1310 only if the analysis calls for it)
- Test: `src/repository/recovery.rs` tests, `tests/mutation_bridges/context.rs` (`release_with_another_pending_row_on_the_root_is_recovery_required` ~1205 stays valid where repository scope is real), `tests/mutation_replay/reenter.rs`

**Interfaces:**
- Consumes: Task 3 `item_id` on authoring rows. Produces: `Pending::Another` applies only to rows of the same item and to rows with a recorded repository-wide reason.

- [ ] **Step 1: Analyse first** (no code): read why the hold is per repository and list every reason (shared index refresh, discovery index, primary-branch checkout, refresh rows with an empty target) in the task report. If a reason needs a redesign, stop and report; the owner decides whether to split it out.
- [ ] **Step 2: Write failing table test** `an_unfinished_operation_blocks_its_own_item_and_repository_wide_work_but_not_other_items`: rows (same item blocked; other item allowed; whole-repository refresh blocked; whole-repository operation blocked by an item's row where the analysis says so). Add `writers_on_two_tickets_both_complete_while_one_is_unfinished` in `reenter.rs`.
- [ ] **Step 3: Run;** FAIL. **Step 4: Implement. Step 5: Run** the new tests and the existing `context.rs` and `journal_rejection` cases named above. Expected: PASS. Commit.

### Task 11: Skills, incident re-run, ticket notes

**Files:**
- Modify: `skills/manyhands-recovery/SKILL.md` ("Whose operation is it?"), `skills/manyhands-overview/SKILL.md` and `references/cli-conventions.md` and `references/autonomy.md` (prose bullet only; the table stays byte-identical to the design's "Agent autonomy" table), `skills/manyhands-comments/SKILL.md`, `skills/manyhands-sharing-and-sync/SKILL.md`, `docs/plans/2026-11-17-user-skills-design.md` (amended note)
- Test: `tests/cli/skills.rs` drift tests

- [ ] **Step 1:** Update the skills as the ticket's second comment lists (prove ownership from the new fields; drop the baseline-list workaround; describe the `busy` answer and `data.blocker`; remove the "no two writers" caution; one place says which behaviour needs which CLI build; minimum CLI version stays 0.2.0 with the old guidance kept for older builds). Verify every changed claim by running it on this branch's build in scratch repositories.
- [ ] **Step 2: Run** `devenv shell -- cargo test --all-features --locked --test cli skills::`. Expected: PASS.
- [ ] **Step 3: Re-run the incident reproducer** (two concurrent `comment add` on different tickets, 30+ pairs, local and with the throwaway SSH remote; the kill-after-`push_prepared` case) on this build in scratch repositories and record the counts in a ticket comment: no operation left behind, `busy` names the holder, the killed publication resumes.
- [ ] **Step 4: Commit.** Then review-ready: static gates, fast tier, SSH tier by changed path, once; merge main; comment the results on the ticket; the merge note says the registry migrates one-way.
