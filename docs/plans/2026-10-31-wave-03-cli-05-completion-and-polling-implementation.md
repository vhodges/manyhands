---
title: "Wave 03 C5 CLI Completion And One-Shot Polling Implementation Plan"
date: 2026-10-31
status: "approved"
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
  - `src/repository/mutation/`, with `mutation/bind/conflict.rs` for decision 34's child mapping and holder (amended 2026-11-03 by C5, decision 34);
  - `src/repository/remote/{state,sync,poll}.rs`;
  - `src/repository/remote/conflict_read.rs`, for decision 34's parent-to-child lookup only (amended 2026-11-03 by C5);
  - `src/repository/remote/reservation.rs`, for `release_promotion_child` only (decision 18);
  - `src/repository/local_sync/run.rs`, for the identity check only (decision 26);
  - `src/repository/transport/remote.rs`, for telling a poll's held lease from a protocol failure only (decision 24);
  - `src/repository/promotion/` and `src/repository/closure/`, for the parent-record accessor, the cancellation probe and the identity path only;
  - `src/repository/read/{mod,dto,graph}.rs`, for exports, the corrected comment and the made-ready read;
  - `src/results.rs`, for the `RequestCancel` doc only;
  - `src/repository.rs`, for `set_publication_remote_recorded`, its already-selected branch and one `FailurePoint` variant (decisions 10 and 17) only;
  - `schemas/v1/`.

  No new `ResultCode` or `RecoveryActionKind`. No field on a public struct that tests build by literal: the probe is a crate-private entry point, and the public signatures stay. No column of an audited table changes, and no blocker enum's order. A gap beyond this stops the task for the owner.
- **Desktop.** No desktop file changes unless an exhaustive `match` on `Command`, `PreviewDetailsDto` or `MutationDataDto` requires one arm, as C4's `confirm.rs` did. Such a task also runs the two desktop commands in the checkpoint gate.
- **Bin.** New modules follow C1's pattern: `#[path = "manyhands-cli/<name>.rs"] mod <name>;`, with `#[cfg(test)] #[path = "<name>_tests.rs"] mod tests;`.
- **No dead code.** Add each helper in the task that first calls it. Clippy denies the rest.
- **Runtime.** The CLI spawns no thread and no child. Transport starts before the service opens and only for verbs that need it. JSON mode writes one envelope to stdout and nothing to stderr.
- **Content and secrets.** No type holding source or a token derives `Debug`, `Display` or `Serialize`, except the content read's DTO. `closed_by` appears only in the preview, canonical content, the index and Git, as Cycle 10 lists. Previews aren't persisted, and request records hold no `closed_by` or title. No input value, source, token or backend text reaches a diagnostic.
- **Test placement.** Library tables are modules of `tests/mutation_remote.rs`, on the new `tests/support/lifecycle_world.rs`. Cases in `harness = false` targets are `ssh_harness` cases named `c5_<module>_<case>`, registered in `CASES`, split per row under the 100-second watchdog. `#[test]` functions do not run there. `tests/cli_remote.rs` reaches public items only.
- **Test isolation.** Every CLI child goes through `tests/support/cli.rs`, `tests/support/pty.rs` or the SSH harness. Tests never touch developer keys, global Git configuration, the real terminal or real application data.
- **Pre-existing tests.** Only named edits E-1 to E-22. Any other semantic edit stops for owner review.
- **Outcome tables.** Each table's test is written first and seen to fail. It is committed together with the code that passes it. A row that answers otherwise stops for a ruling.
- **Test runs** (amended 2026-11-02, caused by main: `AGENTS.md` at `0dfd5a68`, merged as `0b04fd0c`).
  - Each task runs the targets and filters it names. Check that more than zero tests ran. Cargo takes one positional filter per command.
  - A task editing `src/repository/remote/` also runs `--lib remote`, `--lib local_sync` and `--lib integration`, each as its own command.
  - No task runs the full suite. The full `cargo test` and native CI run after merge. Use a 600-second tool timeout for gate commands, and run a long tier detached.
- **Checkpoint gate** (amended 2026-11-02, caused by main; desktop clippy amended 2026-11-03, caused by main: since D4's merge, `--all-targets --features desktop` fails on main, because D4's test modules need `desktop-test-support`, so the gate runs that feature, as whole-tree `--all-features` does; CI's `--features desktop --bin manyhands` also passes). The static gates across the whole tree, then `AGENTS.md`'s fast tier verbatim, then the SSH tier selected by changed path per `AGENTS.md`'s table:

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo check --locked --features desktop
  devenv shell -- cargo clippy --all-targets --locked --features desktop-test-support -- -D warnings
  devenv shell -- cargo check --locked --bin manyhands-cli
  devenv shell -- sh scripts/check-cli-no-gpui.sh
  devenv shell -- cargo test --all-features --locked --lib --bins \
    --test canonical_foundation --test cli --test discovery_rebuild \
    --test drafts --test journal_rejection --test key_material \
    --test key_storage --test local_authoring --test mutation_bridges \
    --test mutation_confirmation --test mutation_contract \
    --test mutation_relationships --test mutation_replay \
    --test read_boundary --test read_characterization --test read_comments \
    --test read_contract --test read_credentials --test read_items \
    --test read_relationships --test read_repository --test read_status \
    --test recovery_foundation_gate --test remote_reservation \
    --test repository_enablement --test session_credentials \
    --test shared_key_registry -- --skip repository::local_sync::
  # Then each SSH-tier target the changed paths select, per AGENTS.md's table,
  # as `devenv shell -- cargo test --all-features --locked --test <target>`.
  ```

- **Goldens.** Update with `MANYHANDS_UPDATE_GOLDEN=1`, never under `CI`. Read every regenerated golden's diff before committing. Register new cases in `tests/support/mutation_golden.rs`, `tests/support/golden.rs` or `tests/support/cli.rs`.
- **Commits.** Commit at the end of every task. Messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- **Execution ledger.** Task 0 creates `docs/plans/2026-10-31-wave-03-cli-05-completion-and-polling-execution.md`, and every task commits it.
- **Ticket comments.** One at Task 0, at each task's completion, at each Part checkpoint and at review-ready, under `.manyhands/comments/01M4CC0VMXTNXNH78C11DYXS8H/<ULID>.md`, since the comment switch is deferred. ULIDs come from `id new`, `created_at` from the UTC clock.
- **Authorization.** Pushing the ticket branch is the standing habit. Never push another branch, open a PR, merge, close the ticket, create a ticket, run `ticket save` or clean up without separate authorization. Native CI runs after merge with the owner's approval of that run.

- **Remaining tasks' Verify** (amended 2026-11-02, caused by main). Each keeps its targeted filters. At a checkpoint it adds the fast tier and the SSH-tier rows its changed paths select.

## Review Focus

Each has its test in the owning task.

1. A new-ID lifecycle request while cleanup is stopped on a supersedable reason: replaced, and the old request then answers `request_mismatch`. Task 2b.
2. `IndexPending`, then a retry: made-ready computed on the first complete read and stored when the request finishes. Task 4.
3. A ticket that depends on the closed ticket and on an unresolved ID: left out of made-ready. Task 4.
4. A Ctrl-C after the last safe point of a closure whose push is then refused. Task 13a.
5. A `show-local --token` with a tampered token: refused with nothing read. Task 10a.
6. `poll pause` against a stale whole-tuple caller: a control test shows `set_remote_polling` with stale values loses the newer interval, and `update_remote_polling` keeps it. Task 5a.
7. A malformed remote ref in `poll once`: nullable fields, and no raw name in output. Task 9.

## Task Order

Part A (library): Tasks 0, 2a–6. There is no Task 1: each command arrives in the task that binds it. Part B (CLI): Tasks 7–11. Part C (evidence): Tasks 12a–15.

## Task 0: Baseline, Re-Grounding, Audit Refresh And Ledger

- [ ] **Step 1:** Ask the owner how commits are identified. Merge `origin/main` if it moved.
- [ ] **Step 2:** Re-check every Baseline Fact by grep or test. A changed fact stops for a ruling.
- [ ] **Step 3:** Grep for test pins beyond E-1 to E-15: `NotResumable`, `"C5"`, `arrives with`, `Planned`, `command_unavailable`. `tests/read_relationships.rs`'s `"C5"` is data. An unlisted hit stops for the owner.
- [ ] **Step 4:** Read and record in the ledger:
  - `lifecycle_record.rs:345-354` and `:977`: that a same-ID retry clears `cancel_requested`;
  - `state::configure`: that it leaves `paused` alone;
  - that a registered repository always has its polling row, and that `request_records` and `remote_polling_state` share the registry connection (decision 15);
  - where recovery after a stop in `set_publication_remote_recorded` lives;
  - that a confirmation stores no `closed_by`, and what verb creates a missing context;
  - whether a `FailurePoint` yields an `ExternalChange`-kind error after a checkpoint. If none does, Task 2b's row becomes a unit test of the mapping.
- [ ] **Step 5:** Reproduce `01M669FJR2` through the binary, and confirm the later-publication order doesn't reach `01M66HFV44`. A surprise stops for the owner.
- [ ] **Step 6:** Refresh the API audit's C5 rows, with code lists.
- [ ] **Step 7:** Create the ledger. Commit `docs: start the C5 execution ledger and refresh the audit`. Ticket comment. Push.

## Task 2a: Lifecycle Binding: Accessor, Prepare And First Run

**Files:** Create `src/repository/mutation/bind/lifecycle.rs`, `tests/support/lifecycle_world.rs`, `tests/mutation_remote/c5_lifecycle.rs`. Modify `tests/mutation_remote.rs`, `command.rs`, `bind/mod.rs`, `mutation/mod.rs`, `progress.rs` (`Session` methods, both impls), `confirm.rs`, `dto.rs`, `outcome.rs`, `promotion/mod.rs`, `closure/mod.rs`, `schemas/v1/`, `tests/support/mutation_golden.rs`, `tests/support/conformance.rs`, `tests/mutation_contract.rs`, `src/bin/manyhands-cli/human_admin.rs`, `src/desktop/view/confirm.rs` if its match needs an arm, E-9, E-12, E-13.

- [ ] **Step 1:** Write the prepare and first-run rows as `c5_lifecycle_` cases for both verbs: ready; each blocker rule; dirty primary; unknown ID; wrong kind; `Complete` with each publication result; a local-only run. Build the world, and see each assertion fail against a binding that answers `internal_error`, so the failure isn't only a compile error.
- [ ] **Step 2:** Add `DocumentPromote` and `TicketClose`, with an arm in every predicate, `from_stored` and `recorded_effects`, their `HOMES` entries and conformance rows, and the `command.rs` tests. Implement `lifecycle_parent`, `Session` methods, `PreviewDetailsDto::Lifecycle`, `PrepareDataDto::LifecycleBlocked`, the data DTOs, schemas and goldens, and the binding's first run. Audit consumers of both DTOs; a desktop arm runs the two desktop gate commands.
- [ ] **Step 3: Verify.** `--test mutation_remote -- c5_lifecycle`, `--test mutation_contract`, `--test mutation_replay`, `--test mutation_confirmation`, `--test mutation_bridges`, `--lib mutation`.
- [ ] **Step 4:** Commit `feat: bind promotion and closure prepare and execution`. Ticket comment. Push.

## Task 2b: Lifecycle Binding: Re-Entry, Standing And The Answers Table

**Files:** Modify `bind/lifecycle.rs`, `bind/mod.rs` (`recorded_effects`), `confirm.rs`, `confirm_tests.rs` (E-15), `outcome.rs`, `tests/mutation_remote/c5_lifecycle.rs`.

- [ ] **Step 1:** Write the rows of the design's re-entry, standing and answers tables first, as `c5_lifecycle_` cases, split per row. They include: a crash before the record; a stop with a written checkpoint and no remote row, which keeps the record; a preview now `Blocked` on re-entry; a stale expectation at confirm; an expired confirmation, and a kept record past ten minutes; a deleted record's resend past ten minutes; a pending local sync and comment publication; `PrimaryAdvanced` continuing under the same request; refused push; refused deletion; a refused checkpoint merge and its `next:` lines; Review Focus 1, including the old request's stored `request_mismatch`; cache loss at each stage; each exit split by durable effect.
- [ ] **Step 2:** Implement `Row::Unfinished`, `row`, the standing and the parent-based `recorded_effects`. A row that answers otherwise stops for a ruling.
- [ ] **Step 3: Verify.** `--test mutation_remote -- c5_lifecycle`, `--test mutation_replay`, `--test mutation_confirmation`, `--lib mutation`.
- [ ] **Step 4:** Commit `feat: continue unfinished promotions and closures under their request`. Ticket comment. Push.

## Task 2c: Cancellation Reach

**Files:** Modify `promotion/run.rs`, `promotion/lifecycle.rs`, `promotion/mod.rs`, `closure/lifecycle.rs`, `closure/mod.rs`, `mutation/progress.rs`, `mutation/records.rs`, `mutation/outcome.rs`, `results.rs` (the `RequestCancel` doc), `bind/lifecycle.rs`, `tests/mutation_remote/c5_cancel.rs`, `tests/mutation_remote/c5_lifecycle.rs` (E-17), `tests/mutation_remote.rs`.

- [ ] **Step 1:** Write first, as `c5_cancel_` cases: `cancel_request` during each lifecycle stage, and with no parent yet; a token probe at a safe point and during a child; cancel by `request cancel`, then same request completing, then a new ID, before and after integration; a `request cancel` between automatic attempts stops attempt 2; a killed process's resend answers `cancelled` once, and the next continues; `Pending{Cancelled}` rows.
- [ ] **Step 2:** Implement the observed entry points and the `cancel_request` route. Public signatures stay.
- [ ] **Step 3: Verify.** `--test mutation_remote -- c5_cancel`, `--test document_promotion`, `--test ticket_closure`, and the `remote`, `local_sync` and `integration` lib filters.
- [ ] **Step 4:** Commit `feat: let cancellation reach promotion and closure safe points`. Ticket comment. Push.

## Task 3: Resume Routes

**Files:** Modify `bind/resume.rs`, `dto.rs` (`from_stored`'s tag), `tests/mutation_replay/resume.rs` (E-10), `tests/mutation_remote/c5_lifecycle.rs`.

- [ ] **Step 1:** Write the design's resume table first, with local-only and pre-integration rows, since those have no journal row. Include a pending child row's ID, and its made-ready.
- [ ] **Step 2:** Look in the lifecycle tables first, and correct the comments at `:36-37` and `:212-214`.
- [ ] **Step 3: Verify.** `--test mutation_remote -- c5_resume`, `--test mutation_replay`, and `--test mutation_remote -- resume`.
- [ ] **Step 4:** Commit `feat: resume unfinished promotions and closures by operation ID`. Ticket comment. Push.

## Task 4: Made-Ready Report

**Files:** Create `tests/mutation_remote/c5_made_ready.rs`. Modify `tests/mutation_remote.rs`, `bind/lifecycle.rs`, `read/graph.rs`, `read/mod.rs`, `dto.rs`, `schemas/v1/`.

- [ ] **Step 1:** Write first: one and many dependents; another open dependency; Review Focus 2 and 3; a dependent whose status text is `closed` with no closure; none; cleanup-only and supersede as `not_applicable`; stopped before integration, and a checkpoint only, as `not_computed`; domain `IndexPending` as `Owed`, computing on the retry; a complete closure with an incomplete index or a failed read as `Final` `index_unavailable_final` (decision 13), with its schema enum and golden, the failed read by a test hook named in the case; replay identical.
- [ ] **Step 2:** Implement `tickets_made_ready`, `made_ready` and `made_ready_state`.
- [ ] **Step 3: Verify.** `--test mutation_remote -- c5_made_ready`, `--lib read::graph`, `--test read_relationships`.
- [ ] **Step 4:** Commit `feat: report the tickets a closure made ready`. Ticket comment. Push.

## Task 5a: Policy Bindings

**Files:** Create `src/repository/mutation/bind/poll.rs`, `tests/mutation_remote/c5_poll.rs`. Modify `tests/mutation_remote.rs`, `mutation/mod.rs`, `command.rs`, `outcome_tests.rs`, `execute.rs`, `settle.rs` (comment), `records.rs`, `remote/state.rs`, `read/dto.rs`, `bind/mod.rs`, `dto.rs`, `schemas/v1/`, conformance and golden support (E-13).

- [ ] **Step 1:** Write the policy table first: success; already so, including not paused with a future deadline; out-of-range; unregistered; unenabled; unavailable index; `poll resume` clearing a deadline while leaving suspension and the backoff value, read through `polling_status`; pause surviving a remote's removal; Review Focus 6; a crash before the transaction leaves no record and no write; R1 committed with its reply lost, `poll pause` R2, R1 resent, replays and the pause stands; R1 stopped before commit, R2, R1 resent, is a new write and clears the pause; a replay.
- [ ] **Step 2:** Add the three commands with every arm and their conformance rows. Implement `update_remote_polling`, the boundary's transaction path and the three bindings. Correct `read/dto.rs:723`.
- [ ] **Step 3: Verify.** `--test mutation_remote -- c5_poll`, `--test read_status`, `--test mutation_contract`, `--test mutation_replay`, `--test mutation_confirmation`, `--test mutation_bridges`, `--lib mutation`, and the `remote`, `local_sync` and `integration` lib filters.
- [ ] **Step 4:** Commit `feat: bind polling policy controls`. Ticket comment. Push.

## Task 5b: `poll once` Binding

**Files:** Modify `mutation/mod.rs`, `command.rs`, `outcome.rs`, `outcome_tests.rs`, `bind/poll.rs`, `bind/resume.rs`, `progress.rs` (`Session`), `remote/poll.rs`, `dto.rs`, `tests/mutation_remote/c5_poll.rs`, `schemas/v1/`, `tests/support/conformance.rs`, `tests/support/mutation_golden.rs`, `tests/mutation_contract.rs`.

- [ ] **Step 1:** Write the design's `poll once` table first, split per row, including that no remote writes nothing, a pause doesn't refuse, each category's code, both `Err(Transport)` rows, a post-reservation stop followed by a new request that polls, a replay of a final transport stop offering nothing, and `operation resume` naming a held poll's request.
- [ ] **Step 2: Design checkpoint.** A row that fails stops here for a ruling; the mechanism isn't rewritten inside the task (three review rounds reached this table).
- [ ] **Step 3:** Add the command with every arm. Implement it, the outcome rule, the `Session` method and the probe.
- [ ] **Step 4: Verify.** `--test mutation_remote -- c5_poll`, `--test remote_polling`, `--test mutation_contract`, `--test mutation_replay`, `--test mutation_remote -- resume`, `--lib mutation`, and the three lib filters.
- [ ] **Step 5:** Commit `feat: bind explicit one-shot polling`. Ticket comment. Push.

## Task 6: Identity And Remote Selection Fixes, Part A Checkpoint

**Files:** Modify `remote/sync.rs`, `promotion/run.rs`, `promotion/lifecycle.rs`, `closure/lifecycle.rs`, `bind/lifecycle.rs`, `src/repository.rs`, `tests/remote_synchronization.rs`, `tests/document_promotion/`, `tests/ticket_closure/`, `tests/repository_enablement.rs`, `tests/repository_enablement/`, `tests/mutation_remote/c5_lifecycle.rs`. By decisions 22-24 and the owner's list: `src/repository/transport/remote.rs`, `remote/poll.rs`, `promotion/run_tests.rs` (E-19), `tests/remote_merge_recovery.rs`, `tests/mutation_remote/c5_poll.rs`, `tests/support/mutation_golden.rs` and a promotion's `identity_required` golden, added here rather than in Task 7.

- [ ] **Step 1:** Write `01M5MK57`'s regressions for all three paths, as registered cases: bracketed and trimmed configured identities, an unusable confirmed identity, a plain control. Each refusal asserts configuration and refs unchanged, and no unfinished row: journal unchanged for synchronization and promotion, one terminal refused row for closure (R-21). Add one pinning kept recovery records and replays. See them fail, and record the reproduction.
- [ ] **Step 2:** Apply `commits_unchanged` in `select_committing_identity`. Add the prepare check and the domain's check on the no-prior-record path (R-3), and the design's identity table as `c5_lifecycle_identity_` rows.
- [ ] **Step 3:** Write `01M669FJR2`'s regressions in `tests/repository_enablement`: status clean after selection; other index entries unchanged; retry and replay; a stop between commit and index update; a library test that the next synchronization isn't refused. See them fail, and fix `set_publication_remote_recorded`.
- [ ] **Step 4: Verify.**

  ```sh
  devenv shell -- cargo test --all-features --locked --test remote_synchronization -- identity
  devenv shell -- cargo test --all-features --locked --test document_promotion -- identity
  devenv shell -- cargo test --all-features --locked --test ticket_closure -- identity
  devenv shell -- cargo test --all-features --locked --test mutation_remote -- c5_lifecycle_identity
  devenv shell -- cargo test --all-features --locked --test repository_enablement
  devenv shell -- cargo test --all-features --locked --lib remote
  devenv shell -- cargo test --all-features --locked --lib local_sync
  devenv shell -- cargo test --all-features --locked --lib integration
  ```

- [ ] **Step 5:** Commit `fix: refuse unusable identities and keep the index current after remote selection`. It became two commits, identity then index, and a third for the rulings, accepted by the controller.
- [ ] **Step 6: Checkpoint.** Run the gate and the merge-tree check. Independent spec and quality review of Part A's range. Fix or rule every finding, and rerun the gate. Ticket comment. Push.

## Task 7: CLI Lifecycle Verbs

**Files:** Create `src/bin/manyhands-cli/lifecycle.rs`, `lifecycle_tests.rs`, `human_lifecycle.rs`, `tests/cli/lifecycle.rs`. Modify `src/bin/manyhands-cli.rs`, `catalog.rs`, `dispatch.rs`, `mutate.rs`, `collaboration.rs` (one arm, approved), `human_collaboration.rs` (one helper `pub(crate)`), `render.rs`, `human.rs`, `tests/cli.rs`, `tests/cli/help.rs`, `collaboration_tests.rs`, and the E-1 to E-7 and E-14 tests.

- [ ] **Step 1:** Catalog and grammar for both verbs. Move the planned-state tests one verb at a time. Gating `planned()` waits for Task 9 (R-8).
- [ ] **Step 2:** Implement through `mutate::run`, inside the Ctrl-C window, with unlock handling. The human preview shows every effect and the slug. The result prints `made ready:` lines with slugs.
- [ ] **Step 3:** The per-verb matrix's Task 7 cells, with goldens: preview, ok, publication pending, blocked, identity required, in JSON and text; input failure; no-op; mismatch. Local-only promotion and closure through the binary, and the application-data sentinel scan for `closed_by` and title.
- [ ] **Step 4: Verify.** `--bin manyhands-cli`, `--test cli -- lifecycle`, `--test cli -- help`, `--test cli -- boundaries`, `--test cli -- exits`.
- [ ] **Step 5:** Commit `feat: add document promote and ticket close to the CLI`. Ticket comment. Push.

## Task 8: CLI Policy Verbs

**Files:** Create `src/bin/manyhands-cli/polling.rs`, `polling_tests.rs`, `schemas/input/v1/poll_configure.schema.json`, its example under `tests/fixtures/cli_input_v1/`, `tests/cli/polling.rs`. Modify `src/bin/manyhands-cli.rs`, `catalog.rs`, `catalog_tests.rs`, `dispatch.rs`, `tests/cli.rs`.

- [ ] **Step 1:** `poll configure`, `poll pause`, `poll resume`, with goldens, the suspension note and the matrix's cells.
- [ ] **Step 2: Verify.** `--bin manyhands-cli`, `--test cli -- polling`, `--test cli -- input_schemas`.
- [ ] **Step 3:** Commit `feat: add polling policy verbs to the CLI`. Ticket comment. Push.

## Task 9: `poll once` Verb

**Files:** Modify `polling.rs`, `catalog.rs`, `catalog_tests.rs`, `parse_tests.rs`, `startup_tests.rs`, `mutate.rs`, `collaboration_tests.rs`, `tests/cli_remote.rs`. Create `tests/cli_remote/polling.rs`.

- [ ] **Step 1:** Implement with unlock handling and the Ctrl-C window. Human output per target (Review Focus 7), printing `PollTargetDisposition::guidance`. On host and unlock stops it says a **new request** follows, and on `busy` it hints `operation list` then `operation resume`. Gate `planned()` and `Availability::Planned` now (R-8).
- [ ] **Step 2:** `c5_polling_` cases, with human and JSON goldens: fast-forward and materialize once; dirty, divergent, malformed and deleted contexts preserved, with no push, checkpoint, merge or cleanup; a remote context of a closed ticket; host and unlock stops, each a final answer, then a new request after correction; no remote; usage failure; replay; mismatch; a second-process `busy`.
- [ ] **Step 3: Verify.** `--test cli_remote -- c5_polling`.
- [ ] **Step 4:** Commit `feat: add poll once to the CLI`. Ticket comment. Push.

## Task 10a: Context Reads And `context sync-local`

**Files:** Create `src/bin/manyhands-cli/local_sync.rs`, `local_sync_tests.rs`, `schemas/input/v1/context_sync_local.schema.json`, its example, `tests/cli/local_sync.rs`. Modify `src/bin/manyhands-cli.rs`, `catalog.rs`, `catalog_tests.rs`, `dispatch.rs`, `mutate.rs`, `collaboration_tests.rs`, `tests/cli.rs`.

- [ ] **Step 1:** Write the design's required outcomes for the reads and `context sync-local` first. Implement with goldens, with primary and remote refs unchanged and no transport start; replay and mismatch; Review Focus 5; human `--token` refused.
- [ ] **Step 2: Verify.** `--bin manyhands-cli`, `--test cli -- local_sync`, `--test cli -- input_schemas`.
- [ ] **Step 3:** Commit `feat: add local context observation, inspection and synchronization to the CLI`. Ticket comment. Push.

## Task 10b: `context resolve-local`

**Files:** Create `schemas/input/v1/context_resolve_local.schema.json`, its example. Modify `local_sync.rs`, `human_collaboration.rs`, `catalog.rs`, `dispatch.rs`, `tests/cli/local_sync.rs`.

- [ ] **Step 1:** Write the design's required outcomes for `context resolve-local` first. Implement with goldens, replay, mismatch and `operation abandon`'s guarded answer. A source sentinel is absent from application data and diagnostics.
- [ ] **Step 2:** Walk the route Task 7's human lines print, through the binary, a test first: a refused closure checkpoint merge, the four verbs, the same request, `ok`, and the close merges once (the resolution's own checkpoint merge is the repair). Task 7 prints it as a route "to try", and nothing has walked it. The walk must use these exact strings, pinned at `human_lifecycle.rs`'s `merge_refused` and by the `ticket_close_checkpoint_merge_refused_text` golden; a change to one changes the other:

  ```text
  manyhands-cli context observe-local --repo <root> --id <item>
  manyhands-cli context sync-local --repo <root> --id <item> --input - --request-id <new-request-id>
  manyhands-cli context show-local --repo <root> --operation-id <sync-operation-id>
  manyhands-cli context resolve-local --repo <root> --operation-id <sync-operation-id> --input - --request-id <new-request-id>
  ```

  `sync-local`'s input takes `observe-local`'s observation, and `sync-local` returns `operation_id` on a conflict stop, or `<sync-operation-id>` has no source. A failure stops for the owner. If the route holds, drop "to try" from `REPAIR` with a golden update.
- [ ] **Step 3: Verify.** `--bin manyhands-cli`, `--test cli -- local_sync`, `--test cli -- input_schemas`.
- [ ] **Step 4:** Commit `feat: add context resolve-local to the CLI`. Ticket comment. Push.

## Task 11: Real Content And Part B Checkpoint

- [ ] **Step 1:** Run the design's Dogfooding set on a disposable clone of this repository, with a fresh `MANYHANDS_DATA_DIR`, through the branch-built CLI. Record each answer in the ledger. A refusal or partial result stops for the owner.
- [ ] **Step 2:** Merge main if it moved, and run the gate.
- [ ] **Step 3:** Independent spec and quality review of Part B's range. Fix or rule every finding, and rerun the gate.
- [ ] **Step 4:** Ledger, ticket comment, push.

## Task 12a: Lifecycle Exit Evidence Over SSH

**Files:** Create `tests/cli_remote/lifecycle.rs`. Modify `tests/cli_remote.rs`.

- [ ] **Step 1:** `c5_lifecycle_` cases: dirty primary; stale consent; refused primary push (`reject_primary_updates`); refused deletion (`reject_context_deletion`); host and unlock stops; same-request completion and replay; `operation resume` of each verb through the binary, naming the request and, with no record, continuing; `PrimaryAdvanced`; counted single merge and checkpoint, allowing for bounded retries; `closed_at` and `closed_by` stable.
- [ ] **Step 2:** The refused checkpoint merge repaired through the context verbs, then `ticket close` again, with no manual Git step.
- [ ] **Step 3: Verify.** `--test cli_remote -- c5_lifecycle`.
- [ ] **Step 4:** Commit `test: prove lifecycle exit evidence over SSH`. Ticket comment. Push.

## Task 12b: The Six Journeys

**Files:** Create `tests/cli_remote/journeys_c5.rs`. Modify `tests/cli_remote.rs`.

- [ ] **Step 1:** The design's journey table as `c5_journey_` cases, each asserting Git objects and canonical content, including both later-publication variants and the close prerequisite failure.
- [ ] **Step 2: Verify.** `--test cli_remote -- c5_journey`.
- [ ] **Step 3:** Commit `test: prove the CLI counterparts of the six journeys`. Ticket comment. Push.

## Task 13a: Interruption And No Worker

**Files:** Modify `tests/cli_remote/interruption.rs`.

- [ ] **Step 1:** Ctrl-C at a safe point for both lifecycle verbs, `poll once` and both context mutations, and after the last one (Review Focus 4), `#[cfg(unix)]`. `request cancel` from a second invocation. Cancel, then same request, then new ID. A second press, a Ctrl-C during `operation resume` of a lifecycle, and a killed `poll once` resent under its ID. The row the library tables owe: a primary child that fails with cancellation asked (`publish`'s `Err` arm, `promotion/run.rs:1139-1145`), which releases that child. The interactive consent prompt for both lifecycle verbs, and a press at it.
- [ ] **Step 2:** Lost stdout, with a replay making no second effect.
- [ ] **Step 3:** `c5_interruption_no_resident_worker`: after each of `poll once`, `ticket close` and `document promote`, the process exited, both pipes reach EOF and, on Unix, no process remains in the child's session (`setsid`, `tests/support/cli.rs:71`). A positive control leaves a sleeping child and shows the check fails.
- [ ] **Step 4: Verify.** `--test cli_remote -- c5_interruption`.
- [ ] **Step 5:** Commit `test: prove C5 interruption and process exit`. Ticket comment. Push.

## Task 13b: Recovery Walk And Exits

**Files:** Modify `tests/cli/collaboration_exits.rs` (E-8), `tests/cli/exits.rs`, `tests/cli_remote/lifecycle.rs`.

- [ ] **Step 1:** Walk every `next:` line through the binary until the reservation is free, for each stop the lifecycle, poll and context verbs can print. Include decision 19's route: an `operation resume` with no request record stopped at `remote_branch_deleted`, then `item republish`, then `operation resume`, one merge. Also a released resolution whose branch is deleted: if no route frees it, stop for the owner. The same check for a retained conflict fenced by `key select` and by a poll's plan failure (`c5_cancel_key_select_fences_a_retained_conflict_close`, `c5_cancel_poll_plan_failure_fences_a_retained_conflict_close`): if no route frees them, this walk stops for the owner. Record each in the ledger.
- [ ] **Step 2:** Extend the exit table's set equality, with a reason for each unreachable code.
- [ ] **Step 3: Verify.** `--test cli_remote -- c5_lifecycle_walk`, `--test cli -- exits`.
- [ ] **Step 4:** Commit `test: walk C5 recovery paths and account for exit codes`. Ticket comment. Push.

## Task 14a: Inventory And Recovery Tables

**Files:** Create `tests/cli/inventory.rs`, `tests/cli/recovery_tables.rs`. Modify `tests/cli.rs`, `docs/RFC/cli-contract.md`.

- [ ] **Step 1:** Write the RFC taxonomy rows, grammar and recovery tables.
- [ ] **Step 2:** Write the tests. The generated help assertion covers `document promote` and `ticket close`, whose goldens (`help_document_promote`, `help_ticket_close`) Task 7 added. Fill gaps by added goldens. A gap in an earlier Cycle's verb gets a golden with no behaviour change. A behaviour gap stops for the owner.
- [ ] **Step 3: Verify.** `--test cli -- inventory`, `--test cli -- recovery_tables`.
- [ ] **Step 4:** Commit `test: account for every approved command and recovery path`. Ticket comment. Push.

## Task 14b: Document Amendments

- [ ] **Step 1:** Make the remaining amendments, in the dated form "Amended 2026-MM-DD by C5", citing the decision or ruling.
- [ ] **Step 2:** Grep for stale terms: `C5 remains cataloged`, `arrives with C5`, `C4/D5 consume`, `NotResumable`, `until C5`, `canonical until C5`.
- [ ] **Step 3:** Commit `docs: amend the RFCs, Wave and audit for C5`. Ticket comment. Push.

## Task 15: Full Verification, Review And Handoff

- [ ] **Step 1:** Merge `origin/main` and run, detached, the static gates (desktop clippy with `--all-targets --features desktop-test-support`, as the checkpoint gate says, amended 2026-11-03), the fast tier and every SSH-harness target: the branch's diff touches `src/repository/transport/`, and Task 15 follows `AGENTS.md`'s table across the branch's full diff (owner, 2026-11-02). No full suite (amended 2026-11-02, caused by main). Record per-target counts. A failure stops for the owner.
- [ ] **Step 2:** Independent whole-branch spec and quality reviews. Fix or rule every finding. Rerun the gate and the affected targets, and say what ran after the fixes.
- [ ] **Step 3:** Mark the Cycle and design `review-ready`, run `git merge-tree --write-tree HEAD origin/main`, add the review-ready comment and push. The ticket stays **open**.
- [ ] **Step 4: Closure,** only after the owner approves the review, in the final push before merge. Close this ticket by the canonical file edit (decision 9). Copy `01M5MK57` and `01M669FJR2`'s ticket files byte for byte from their branches, add the closure fields and a comment each, on the same go-ahead. Merging needs its own go-ahead, which also covers deleting those two remote branches, after confirming that they hold only their ticket files (decision 14).
- [ ] **Step 5:** Record the post-merge follow-up for the owner: a real `ticket close` and the AGENTS.md switch in a separate session, native CI with its own approval, and the follow-up candidates, which are one list in the CLI contract RFC's [C5 evidence and known limits](../RFC/cli-contract.md#c5-evidence-and-known-limits) (`effective_identity`'s cascade among them). None is opened without the owner's go-ahead.

## Plan Self-Review

**Acceptance rows to tasks:**

| Cycle acceptance row | Task |
| --- | --- |
| All commands accounted for | 14a |
| Dirty-primary blocking | 2a, 7, 12a |
| Stale consent | 2b, 12a |
| Push and cleanup failures | 2b, 12a |
| No duplicate close or merge | 2b, 3, 12a |
| Later primary publication | 6, 7, 12b |
| Safe explicit polling | 5b, 9 |
| Six journeys | 12b (plus C4's conflicts) |
| No resident worker | 13a |
| Made-ready | 4, 7, 12b |
| Local recovery | 10b, 12a |
| Identity | 6 |
| Cache loss | 2b |
| Remote selection | 0, 6 |
| Outcome tables | 2a, 2b, 3, 4, 5a, 5b, 10a, 10b |
| Ctrl-C | 2c, 13a |
| Recovery walk | 13b |
| Per-verb evidence | 7–10b, 12b, 13a |
| Privacy | 7 (`closed_by`, title), 10b (source) |
| Exit classes | 13b |
| Real repository | 11 |
| Closure | 15 |
| Unchanged behaviour | 11, 15 |

**Rulings to tasks:** R-1 Task 2a; R-2 Task 7; R-3 Task 6; R-4 Task 4; R-5, R-6 Task 6; R-7 Task 5a; R-8 Task 7; R-9, R-10 Task 14a; R-11 all; R-12 Task 11; R-15 Task 10a; R-16, R-17, R-18 Task 5b; R-19 Task 2b; R-20, R-29 Task 2c; R-21 Task 6. R-13 and R-14 are withdrawn.

**Decisions to tasks:** 1 all of Part A; 2 Task 6; 3 Task 10a; 4 Task 14a; 5 Tasks 5a, 8; 6 Tasks 2a, 6; 7 authorizes the documents, no task; 8 Tasks 2b, 2c, 13a; 9 Tasks 11, 15; 10 Tasks 0, 6; 11 Task 5a; 12 Task 2c; 13 Task 4; 14 Task 15; 15 Task 5a; 18 Task 2c.

**Decisions 21-24:** all Task 6. Decision 21 narrows the Step 1 row for synchronization only: closure stays R-21, and promotion's journal stays unchanged. `--lib promotion` found E-19, which Task 6's own list would have missed until the gate.

**Named edits:** E-1 to E-7, E-14 and E-16 in 7–10b; E-8 in 13b; E-9, E-12, E-13 in 2a, 5a, 5b; E-10 in 3; E-11 in 2a, 4, 5a, 5b; E-15 in 2b; E-17 in 2c; E-18 and E-19 in 6; E-20 and E-21 as the design records them; E-22 with decision 29.

**Stops built in:** Task 0: identity, a changed fact, an unlisted pin, a reproduction surprise. Outcome-table rows in 2a–5b. Task 11: a refusal on real content. Task 14a: a behaviour gap. Task 15: a suite failure, closure.

**Placeholders:** none intended. Facts marked for Task 0 are checks, not choices left to an implementer.
