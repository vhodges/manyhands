---
title: "Wave 03 C5 CLI Completion And One-Shot Polling Implementation Plan"
date: 2026-10-31
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M69C6SHD04G8PT6D1Y36CDN7"
---

# CLI Completion And One-Shot Polling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the promotion, closure and polling bindings, expose the remaining verbs and the local-sync verbs through `manyhands-cli`, fix the identity selector, and prove the complete CLI contract.

**Architecture:** A shared lifecycle binding and a polling binding in `src/repository/mutation/bind/` (Part A). Bin modules on C2's `mutate::run` driver and C1's catalog (Part B). Two-clone SSH journeys, the inventory and recovery-table tests (Part C).

**Tech Stack:** Rust 2024, clap's builder API, `serde_json`, `git2`, F2's mutation boundary, C1–C4's CLI harness, `CliPair` and the SSH harness.

**Spec:** the [Design](2026-10-31-wave-03-cli-05-completion-and-polling-design.md), under the [Cycle](../Cycles/wave-03-cli-05-completion-and-polling.md). The Cycle governs the design, and the design governs this plan.

## Global Constraints

- **Devenv.** `devenv shell -- cargo <command> --locked`. No new crate, no `Cargo.toml` or `Cargo.lock` change, no new `[[test]]` target (R-11).
- **Branch.** Work only in `.manyhands/worktrees/01M4CC0VMXTNXNH78C11DYXS8H`, on `manyhands/ticket/01M4CC0VMXTNXNH78C11DYXS8H`. If `origin/main` moves, `git merge origin/main`. Never rebase. Before each checkpoint, `git fetch origin` and `git merge-tree --write-tree HEAD origin/main`.
- **Git identity.** This machine has no configured Git identity. Task 0 stops and asks the owner how commits are identified, rather than writing configuration.
- **Library scope (decision 1).** Library changes are Part A's only, and only in:
  - `src/repository/mutation/`;
  - `src/repository/remote/{state,sync,poll}.rs`;
  - `src/repository/promotion/` and `src/repository/closure/`, for accessors and the identity path only;
  - `src/repository/read/{mod,dto,graph}.rs`, for exports, the corrected comment and the made-ready read;
  - `schemas/v1/`.

  No new `ResultCode` or `RecoveryActionKind`. No field on a public struct that tests build by literal. No column of an audited table changes, and no blocker enum's order. A gap beyond this stops the task for the owner.
- **Desktop.** No desktop file changes unless an exhaustive `match` on `Command`, `PreviewDetailsDto` or `MutationDataDto` requires one arm, as C4's `confirm.rs` did. Such a task also runs the two desktop commands in the checkpoint gate.
- **Bin.** New modules follow C1's pattern: `#[path = "manyhands-cli/<name>.rs"] mod <name>;`, with `#[cfg(test)] #[path = "<name>_tests.rs"] mod tests;`.
- **No dead code.** Add each helper in the task that first calls it. Clippy denies the rest.
- **Runtime.** The CLI spawns no thread and no child. Transport starts before the service opens and only for verbs that need it. JSON mode writes one envelope to stdout and nothing to stderr.
- **Content and secrets.** No type holding source or a token derives `Debug`, `Display` or `Serialize`, except the content read's DTO. `closed_by` appears only in the preview and canonical content. No input value, source, token or backend text reaches a diagnostic.
- **Test placement.** Library tables are modules of `tests/mutation_remote.rs`. Cases in `harness = false` targets are `ssh_harness` cases named `c5_<module>_<case>`, registered in `CASES`, split per row under the 100-second watchdog. `#[test]` functions do not run there. `tests/cli_remote.rs` reaches public items only.
- **Test isolation.** Every CLI child goes through `tests/support/cli.rs`, `tests/support/pty.rs` or the SSH harness. Tests never touch developer keys, global Git configuration, the real terminal or real application data.
- **Pre-existing tests.** Only named edits E-1 to E-12. Any other semantic edit stops for owner review.
- **Outcome tables.** Each table's test is written first and seen to fail. It is committed together with the code that passes it. A row that answers otherwise stops for a ruling.
- **Test runs.**
  - Each task runs only the targets and filters it names. Check that more than zero tests ran. Cargo takes one positional filter per command.
  - A task editing `src/repository/remote/` also runs `--lib remote`, `--lib local_sync` and `--lib integration`, each as its own command.
  - The full suite runs once, in Task 15, detached. C4's took 3 h 34 min on a loaded machine. Use a 600-second tool timeout for gate commands.
- **Checkpoint gate.**

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo check --locked --features desktop
  devenv shell -- cargo clippy --all-targets --locked --features desktop -- -D warnings
  devenv shell -- cargo check --locked --bin manyhands-cli
  devenv shell -- sh scripts/check-cli-no-gpui.sh
  devenv shell -- cargo test --all-features --locked --lib
  devenv shell -- cargo test --all-features --locked --bin manyhands-cli --test cli --test read_contract --test mutation_contract --test mutation_replay --test mutation_confirmation --test mutation_bridges
  devenv shell -- cargo test --all-features --locked --test mutation_remote --test document_promotion --test ticket_closure --test remote_polling --test cli_remote
  ```

- **Goldens.** Update with `MANYHANDS_UPDATE_GOLDEN=1`, never under `CI`. Read every regenerated golden's diff before committing. Register new cases in `tests/support/mutation_golden.rs`, `tests/support/golden.rs` or `tests/support/cli.rs`.
- **Commits.** Commit at the end of every task. Messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- **Execution ledger.** Task 0 creates `docs/plans/2026-10-31-wave-03-cli-05-completion-and-polling-execution.md`, and every task commits it.
- **Ticket comments.** One at Task 0, at each task's completion, at each Part checkpoint and at review-ready, under `.manyhands/comments/01M4CC0VMXTNXNH78C11DYXS8H/<ULID>.md`, since the comment switch is deferred. ULIDs come from `id new`, `created_at` from the UTC clock.
- **Authorization.** Pushing the ticket branch is the standing habit. Never push another branch, open a PR, merge, close the ticket, create a ticket, run `ticket save` or clean up without separate authorization. Native CI runs after merge with the owner's approval of that run.

## Review Focus

Each has its test in the owning task.

1. A new-ID lifecycle request while cleanup is stopped on a supersedable reason: replaced, not `busy`. Task 2.
2. `IndexPending` then a retry: made-ready computed once and stored. Task 4.
3. A ticket that depends on the closed ticket and on an unresolved ID: left out of made-ready. Task 4.
4. A Ctrl-C after the last safe point of a closure whose push is then refused. Task 13.
5. A `show-local --token` with a tampered token: refused with nothing read. Task 10.
6. A second actor changing `interval_seconds` during `poll pause`: both survive. Task 5.
7. `poll once` with a path holding a terminal escape: human output escapes it. Task 9.

## File Map And Task Order

Part A (library): Tasks 0–6. Part B (CLI): Tasks 7–11. Part C (evidence): Tasks 12–15.

## Task 0: Baseline, Re-Grounding, Audit Refresh And Ledger

- [ ] **Step 1:** Confirm the Git identity with the owner. Merge `origin/main` if it moved.
- [ ] **Step 2:** Re-check every Baseline Fact by grep or test. A changed fact stops for a ruling.
- [ ] **Step 3:** Grep for test pins beyond E-1 to E-12: `NotResumable`, `"C5"`, `arrives with`, `Planned`, `command_unavailable`. An unlisted hit stops for the owner.
- [ ] **Step 4:** Read `RemotePollError`, `PollDisposition`, `lifecycle_record.rs:345-354` and `closure/record.rs:527,627`. Record the answers in the ledger. Confirm by an existing test that an explicit poll skips eligibility.
- [ ] **Step 5:** Refresh the API audit's C5 rows, with code lists.
- [ ] **Step 6:** Record what a `--prepare` writes in application data, and how to run the real-repository reads without touching the owner's data. Stop for the owner if it can't be isolated (R-12).
- [ ] **Step 7:** Create the ledger. Add a ticket comment. Push.

## Task 1: Commands And Predicates

**Files:** Modify `src/repository/mutation/command.rs`, `outcome_tests.rs` (E-9, E-12)

- [ ] **Step 1:** Add `DocumentPromote`, `TicketClose`, `PollOnce`, `PollConfigure`, `PollPause`, `PollResume` with an arm in every predicate, per the design's table. Add them only in the task that first binds them if clippy or the exhaustive `match` in `bind/mod.rs` requires it: Task 2 and Task 5 may each carry their own.
- [ ] **Step 2:** Extend the names test, and add predicate tests.
- [ ] **Step 3:** `devenv shell -- cargo test --all-features --locked --lib mutation::command`.
- [ ] **Step 4:** Commit `feat: add C5 commands to the mutation boundary`. Ticket comment.

## Task 2: Lifecycle Binding

**Files:** Create `src/repository/mutation/bind/lifecycle.rs`, `tests/mutation_remote/c5_lifecycle.rs`. Modify `bind/mod.rs`, `mutation/mod.rs`, `dto.rs`, `outcome.rs`, `schemas/v1/`.

- [ ] **Step 1: Design checkpoint.** Show the mechanism that rebuilds `expected` on re-entry (design "Observation"), in the ledger, before writing the binding. Copy `bind/republish.rs`'s structure.
- [ ] **Step 2:** Write the design's answers table as `c5_lifecycle_` cases for both verbs, on the existing `tests/document_promotion` and `tests/ticket_closure` fixtures' setup. Run and see them fail.
- [ ] **Step 3:** Implement the binding, `PreviewDetailsDto::Lifecycle`, data DTOs, schemas and goldens.
- [ ] **Step 4:** Add Review Focus 1, and these rows: a dirty primary, a stale expectation at confirm, an expired confirmation, `Interrupted` then the same request, a pending local sync, a pending comment publication, an `ExternalChange`-kind repository error after a checkpoint (fact 11).
- [ ] **Step 5: Verify.** `devenv shell -- cargo test --all-features --locked --test mutation_remote -- c5_lifecycle`, then `--test mutation_contract`, then `--lib mutation`, then the `remote`, `local_sync` and `integration` lib filters if `remote/` was touched.
- [ ] **Step 6:** Commit `feat: bind document promotion and ticket closure at the mutation boundary`. Ticket comment. Push.

## Task 3: Resume Routes

**Files:** Modify `bind/resume.rs`, `tests/mutation_remote/` (E-10).

- [ ] **Step 1:** Write the table first: an unfinished promotion and closure with no live request continue; one with a live request answers `left_to_its_request`; a poll stays `invalid_input`; a finished one is `already_applied`, as F2; a cleanup-pending record continues cleanup only.
- [ ] **Step 2:** Split the route, and correct the comments at `:36-37` and `:212-214`.
- [ ] **Step 3: Verify.** `--test mutation_remote -- c5_resume`, `--lib mutation::bind::resume`.
- [ ] **Step 4:** Commit `feat: resume unfinished promotions and closures by operation ID`. Ticket comment. Push.

## Task 4: Made-Ready Report

**Files:** Modify `bind/lifecycle.rs`, `read/graph.rs`, `dto.rs`, `schemas/v1/`.

- [ ] **Step 1:** Write the rows first: one and many dependents, a dependent with another open dependency, an unresolved dependency, a dependent whose status text is `closed` but which has no closure, none, `IndexPending`, retry, replay, a failed read after closure.
- [ ] **Step 2:** Implement `made_ready` and `made_ready_state`.
- [ ] **Step 3: Verify.** `--test mutation_remote -- c5_made_ready`, `--lib read::graph`, `--test read_relationships`.
- [ ] **Step 4:** Commit `feat: report the tickets a closure made ready`. Ticket comment. Push.

## Task 5: Polling Bindings

**Files:** Create `src/repository/mutation/bind/poll.rs`, `tests/mutation_remote/c5_poll.rs`. Modify `remote/state.rs`, `read/dto.rs`, `bind/mod.rs`, `schemas/v1/`.

- [ ] **Step 1:** Write the tables first. `poll once`: complete, `IndexPending`, transport stop, busy, conflict pending, replay, restart after interruption, under a pause. Policy verbs: success, already in that state, out-of-range interval, Review Focus 6.
- [ ] **Step 2:** Implement `update_remote_polling` and the bindings. Correct `read/dto.rs:723`.
- [ ] **Step 3: Verify.** `--test mutation_remote -- c5_poll`, `--test remote_polling`, `--test read_status`, and the `remote`, `local_sync` and `integration` lib filters.
- [ ] **Step 4:** Commit `feat: bind explicit polling and its policy controls`. Ticket comment. Push.

## Task 6: Identity Fix And Part A Checkpoint

**Files:** Modify `remote/sync.rs`; tests in `tests/remote_synchronization.rs`, `tests/document_promotion/`, `tests/ticket_closure/`.

- [ ] **Step 1:** Write `01M5MK57`'s regressions for all three paths: bracketed and trimmed configured identities, an unusable confirmed identity, a plain control. Each refusal asserts configuration, refs and journal unchanged. See them fail, and record the reproduction in the ledger.
- [ ] **Step 2:** Apply `commits_unchanged` in `select_committing_identity`. Add the promotion prepare check (R-3).
- [ ] **Step 3: Verify.** The three targets with `identity` filters, then the three lib filters.
- [ ] **Step 4:** Commit `fix: refuse unusable Git identities before sync, promotion and closure write`.
- [ ] **Step 5: Checkpoint.** Run the gate, the merge-tree check, an independent spec and quality review of Part A's range. Fix or rule every finding, and rerun the gate. Ticket comment. Push.

## Task 7: CLI Lifecycle Verbs

**Files:** Create `src/bin/manyhands-cli/lifecycle.rs`, `lifecycle_tests.rs`, `human_lifecycle.rs`. Modify `catalog.rs`, `dispatch.rs`, `src/bin/manyhands-cli.rs`, `tests/cli/help.rs` (E-1 to E-7), `tests/cli/` new `lifecycle.rs`.

- [ ] **Step 1:** Catalog and grammar for both verbs. Move the planned-state tests (E-2 to E-6).
- [ ] **Step 2:** Implement through `mutate::run`, inside the Ctrl-C window. Human preview shows every effect, and the result prints `made ready:` lines.
- [ ] **Step 3:** Goldens: preview, ok, publication pending, blocked, identity required, each in JSON and text. Local-only closure and promotion through the binary.
- [ ] **Step 4: Verify.** `--bin manyhands-cli`, `--test cli -- lifecycle`, `--test cli -- help`.
- [ ] **Step 5:** Commit `feat: add document promote and ticket close to the CLI`. Ticket comment. Push.

## Task 8: CLI Policy Verbs

**Files:** Create `src/bin/manyhands-cli/polling.rs`, `polling_tests.rs`, `schemas/input/v1/poll_configure.schema.json`, `tests/fixtures/cli_input_v1/` example, `tests/cli/polling.rs`. Modify `catalog.rs`, `dispatch.rs`.

- [ ] **Step 1:** `poll configure`, `poll pause`, `poll resume`, with goldens and human output, including the suspension note.
- [ ] **Step 2: Verify.** `--bin manyhands-cli`, `--test cli -- polling`, `--test cli -- input_schemas`.
- [ ] **Step 3:** Commit `feat: add polling policy verbs to the CLI`. Ticket comment.

## Task 9: `poll once` Verb

**Files:** Modify `polling.rs`, `catalog.rs`. Create `tests/cli_remote/polling.rs`. Modify `tests/cli_remote.rs`.

- [ ] **Step 1:** Implement with unlock handling and the Ctrl-C window. Human output per target, escaping paths (Review Focus 7).
- [ ] **Step 2:** `c5_polling_` cases: fast-forward and materialize once; dirty, divergent, malformed and deleted contexts preserved; transport stop; replay.
- [ ] **Step 3: Verify.** `--test cli_remote -- c5_polling`.
- [ ] **Step 4:** Commit `feat: add poll once to the CLI`. Ticket comment. Push.

## Task 10: Context Verbs

**Files:** Create `src/bin/manyhands-cli/local_sync.rs`, `local_sync_tests.rs`, `schemas/input/v1/context_resolve_local.schema.json`, a fixture example, `tests/cli/local_sync.rs`. Modify `catalog.rs`, `dispatch.rs`, `human_collaboration.rs`.

- [ ] **Step 1:** The four verbs, with goldens. A no-op, fast-forward, clean merge and conflict, with primary and remote refs unchanged and no transport start.
- [ ] **Step 2:** Show, resolve, stale observation, incomplete set, tampered token (Review Focus 5), and `operation abandon`'s guarded answer.
- [ ] **Step 3: Verify.** `--bin manyhands-cli`, `--test cli -- local_sync`, `--test cli -- input_schemas`.
- [ ] **Step 4:** Commit `feat: add local context synchronization verbs to the CLI`. Ticket comment. Push.

## Task 11: Part B Checkpoint

- [ ] **Step 1:** Merge main if it moved, run the gate.
- [ ] **Step 2:** Independent spec and quality review of Part B's range. Fix or rule every finding, rerun the gate.
- [ ] **Step 3:** Ledger, ticket comment, push.

## Task 12: Lifecycle Journeys Over SSH

**Files:** Create `tests/cli_remote/lifecycle.rs`, `tests/cli_remote/journeys_c5.rs`. Modify `tests/cli_remote.rs`.

- [ ] **Step 1:** The design's journey table as `c5_journey_` cases, asserting Git objects and canonical content.
- [ ] **Step 2:** `c5_lifecycle_` exit evidence: dirty primary, stale consent, refused primary push (`reject_primary_updates`), refused deletion (`reject_context_deletion`), same-request completion, counted single merge and checkpoint, `closed_at` and `closed_by` stable.
- [ ] **Step 3:** Later primary publication. If `01M669FJR2` blocks it, stop for the owner (R-14).
- [ ] **Step 4:** The refused checkpoint merge repaired through the context verbs, then `ticket close` again.
- [ ] **Step 5: Verify.** `--test cli_remote -- c5_journey`, then `-- c5_lifecycle`.
- [ ] **Step 6:** Commit `test: prove the CLI's six journeys and lifecycle exit evidence`. Ticket comment. Push.

## Task 13: Interruption, No Worker, Recovery Walk And Exits

**Files:** Modify `tests/cli_remote/interruption.rs`, `tests/cli/collaboration_exits.rs` (E-8), `tests/cli/exits.rs`.

- [ ] **Step 1:** Ctrl-C at a safe point for both lifecycle verbs and `poll once`, and after the last one (Review Focus 4), `#[cfg(unix)]`.
- [ ] **Step 2:** `request cancel` from a second invocation; lost stdout, with a replay making no second effect.
- [ ] **Step 3:** `c5_interruption_no_resident_worker`: after each of the three verbs, the process exited, both pipes reach EOF and, on Unix, no process remains in the child's process group.
- [ ] **Step 4:** Walk every `next:` line through the binary until the reservation is free, for each stop the lifecycle and poll verbs can print. Record each in the ledger.
- [ ] **Step 5:** Extend the exit table's set equality, and give each unreachable code its reason.
- [ ] **Step 6: Verify.** `--test cli_remote -- c5_interruption`, `--test cli -- exits`.
- [ ] **Step 7:** Commit `test: prove C5 interruption, process exit, recovery and exit coverage`. Ticket comment. Push.

## Task 14: Inventory, Recovery Tables And Document Amendments

**Files:** Create `tests/cli/inventory.rs`, `tests/cli/recovery_tables.rs`. Modify `tests/cli.rs` and every document in the design's Document Amendments, except the ones the owner gates.

- [ ] **Step 1:** Write the RFC recovery tables, the taxonomy rows and grammar.
- [ ] **Step 2:** Write the two tests. Run and fix gaps through added goldens. A gap in an earlier Cycle's verb is added as a golden, with no behaviour change. A behaviour gap stops for the owner.
- [ ] **Step 3:** Make the remaining amendments, in the dated form "Amended 2026-MM-DD by C5", citing the decision or ruling. Grep for stale terms: `C5 remains cataloged`, `arrives with C5`, `C4/D5 consume`, `NotResumable`, `until C5`.
- [ ] **Step 4: Verify.** `--test cli -- inventory`, `--test cli -- recovery_tables`.
- [ ] **Step 5:** Commit `docs: complete the CLI contract and amend the RFCs for C5`. Ticket comment. Push.

## Task 15: Full Verification, Review, Dogfooding And Handoff

- [ ] **Step 1:** Merge `origin/main`, run the gate and the full suite once, detached. Record per-target counts. A failure stops for the owner.
- [ ] **Step 2:** Independent whole-branch spec and quality reviews. Fix or rule every finding. Rerun the gate and the affected targets, and say what ran after the fixes.
- [ ] **Step 3: Dogfooding.** With the owner's go-ahead at this step, on a disposable clone of this repository: execute `document promote` on a throwaway document and `ticket close` on a throwaway ticket through the branch-built CLI, then `poll once`. A refusal or partial result stops for the owner.
- [ ] **Step 4:** Mark the Cycle and design `review-ready`, run `git merge-tree --write-tree HEAD origin/main`, add the review-ready comment and push. The ticket stays **open**.
- [ ] **Step 5: Closure,** only after the owner approves the review. With the owner's go-ahead, close this ticket through the branch-built `ticket close`, at a terminal if the key is protected. If that isn't approved or can't run, use the canonical file edit. Close `01M5MK57` on the same go-ahead. Merging needs its own go-ahead.

## Plan Self-Review

**Acceptance rows to tasks:**

| Cycle acceptance row | Task |
| --- | --- |
| All commands accounted for | 14 |
| Dirty-primary blocking | 2, 7, 12 |
| Stale consent | 2, 12 |
| Push and cleanup failures | 2, 12 |
| No duplicate close or merge | 2, 3, 12 |
| Later primary publication | 7, 12 |
| Safe explicit polling | 5, 9 |
| Six journeys | 12 (plus C4's conflicts) |
| No resident worker | 13 |
| Made-ready | 4, 7, 12 |
| Local recovery | 10, 12 |
| Identity | 6 |
| Outcome tables | 2, 3, 4, 5 |
| Ctrl-C | 13 |
| Recovery walk | 13 |
| Exit classes | 13 |
| Real repository | 0, 15 |
| Unchanged behaviour | 11, 15 |

**Rulings to tasks:** R-1 Task 2; R-2 Task 7; R-3 Task 6; R-4 Task 4; R-5, R-6 Task 6; R-7 Task 5; R-8 Task 7; R-9, R-10 Task 14; R-11 all; R-12 Tasks 0, 15; R-13 Task 5; R-14 Task 12; R-15 Task 10.

**Named edits:** E-1 to E-7 in 7–10; E-8 in 13; E-9, E-12 in 1; E-10 in 3; E-11 in 2, 4, 5.

**Stops built in:** Task 0: identity, a changed fact, an unlisted pin, real-repository isolation. Task 2: the expectation checkpoint. Outcome-table rows in 2–5. Task 12: `01M669FJR2`. Task 14: a behaviour gap. Task 15: dogfooding, closure.

**Placeholders:** none intended. Facts marked for Task 0 are checks, not choices left to an implementer.
