---
title: "Wave 03 C4 CLI Discussion, Synchronization And Conflict Recovery Implementation Plan"
date: 2026-10-26
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5X4YWRNZ1SFGXH5J8E9QD2J"
---

# CLI Discussion, Synchronization And Conflict Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Draft, 2026-10-26.** Revised after the third review round, and re-grounded on 2026-10-27 after the gate ticket `01M57SKVF6TQ3QM6AXJ35JGGSY` merged at `a372edb` (Cycle decision 2). Not approved, and implementation is not authorized.

**Goal:** Add C4's library part and its eight CLI verbs, so that an agent or a person can comment, synchronize, resolve or repair conflicts, republish a deleted branch, and resume or abandon operations through `manyhands-cli`. Then switch this project's comment dogfooding to the CLI.

**Architecture:**

- **Library (Part A).** Three new mutation commands, `CommentAdd`, `ConflictResolve` and `ItemRepublish`, are bound in `src/repository/mutation/bind/`. A conflict read lives in `src/repository/remote/conflict_read.rs`. The two synchronization bindings merge, and resume gains routes. All of it goes through F2's boundary: request records, replay, confirmation, progress and cancellation.
- **CLI (Part B).** C1's catalog, parse, startup, dispatch and render, and C2's `mutate::run`, with new bin-private modules beside them. The `Boundary` impl puts a scoped Ctrl-C window around `execute`.
- **Evidence (Part C).** Two-clone SSH journeys through the binary, plus interruption evidence.

**Tech Stack:**

- Rust 2024, clap's builder API, `serde_json` and `git2`;
- `libc` and `windows-sys`, both existing dependencies, for the Ctrl-C window;
- F2's mutation boundary, result model, schemas and test support;
- C1–C3's CLI harness, goldens, pseudo-terminal helper and SSH harness.

**Spec:** the [Design](2026-10-26-wave-03-cli-04-discussion-and-sync-design.md), under the [Cycle](../Cycles/wave-03-cli-04-discussion-and-sync.md). Read both before any task. The Cycle governs the design, and the design governs this plan. "Fact N" means the design's Baseline Fact N, "R-N" its ruling and "E-N" its named edit. The tables are the design's required-outcome tables, by name.

## Global Constraints

- **Devenv.** Run every Rust command through Devenv with `--locked`: `devenv shell -- cargo <command> --locked`.
  - No new crate.
  - No change to `Cargo.toml` or `Cargo.lock`.
  - No new `[[test]]` target (R-29). New tests are modules of existing targets.
- **Branch.** Work only in the ticket worktree `.manyhands/worktrees/01M4CC0VMWEQQ9G4W0DAG7ADPY`, on `manyhands/ticket/01M4CC0VMWEQQ9G4W0DAG7ADPY`.
  - If `origin/main` moves, `git merge origin/main`. Never rebase.
  - Before each checkpoint, run `git fetch origin` and then `git merge-tree --write-tree HEAD origin/main`.
- **The gate first (decision 2).** Task 0 does not start until `git show origin/main:.manyhands/tickets/01M57SKVF6TQ3QM6AXJ35JGGSY/ticket.md` exists and has `closed_at`.
- **Library scope (decision 1).** Library changes are Part A's only, and only in these files:
  - `src/repository/mutation/`;
  - `src/repository/comment_publication.rs` and `src/repository/recovery.rs`;
  - `src/repository.rs`, for the new crate-private `checkpoint_comment_locally_with`, `checkpoint_comment_locally`'s delegation to it, and the re-exports only;
  - `src/repository/remote/{mod,sync,refs,reservation,state,merge}.rs`, and the new `remote/conflict_read.rs`;
  - nothing in `src/repository/integration/` or `src/repository/local_sync/`. The gate moved only helpers there (fact 25), and its local executor shares them. `conflict_read.rs` calls `integration::validate::classify_conflict_entry` read-only (fact 30);
  - `src/repository/read/mod.rs` and `read/dto.rs`, for exports only;
  - `schemas/v1/`.

  Also:
  - No new `ResultCode` or `RecoveryActionKind` (decision 8).
  - No field is added to a public struct that tests build by literal (R-4).
  - No column, trigger or listed object of an audited table changes (fact 75), the gate's eight `local_context_sync_*` tables included. New durable state goes in new tables, so main's binaries still open a registry the branch has migrated.
  - A gap beyond these files stops the task for the owner.
- **Desktop.** Only Task 5's one arm in `src/desktop/view/confirm.rs` (decision 10). That task also runs `cargo check --locked --features desktop` and `cargo clippy --all-targets --locked --features desktop -- -D warnings`, with the desktop feature alone.
- **Bin.** CLI code lives in `src/bin/manyhands-cli.rs` and `src/bin/manyhands-cli/`. New modules follow C1's pattern: `#[path = "manyhands-cli/<name>.rs"] mod <name>;`, with `#[cfg(test)] #[path = "<name>_tests.rs"] mod tests;`. A module that `tests/cli_remote.rs` mounts must still resolve there. `Boundary`'s signature and `Plan`'s fields do not change (fact 61).
- **No dead code.** `cargo clippy -- -D warnings` fails on an unused function, so add each helper in the task that first calls it.
- **Runtime.**
  - The CLI spawns no thread and no child process. On Windows, the console-control routine runs on a thread the system creates (R-21).
  - The transport starts before the service opens. Help, version, usage errors and unavailable commands touch neither.
  - All errors before dispatch have a null `request_id`. JSON mode writes one envelope to stdout and nothing to stderr.
- **Content and secrets.**
  - No type that holds a body, source or token derives or implements `Debug`, `Display` or `Serialize`, except the conflict read's DTO, which is a requested read.
  - No input value, body, source, token, backend text or SQL reaches a diagnostic.
  - JSON `message` is always `ResultCode::message()`.
- **Test placement.**
  - Part A's tables are modules of `tests/mutation_remote.rs`, which mounts `src/lib.rs` as a module along with the golden, schema and conformance support (fact 62). Its tests may call the crate-private functions this plan adds.
  - Each new case in a `harness = false` target is an `ssh_harness` case named `c4_<module>_<case>`, registered in the target's `CASES` and run with `-- c4_<module>` (R-31). `#[test]` functions do not run there (fact 58). Each verification step checks that the expected number of cases ran.
  - `tests/cli_remote.rs` reaches public items only (fact 62).
  - `tests/support/cli_pair.rs` is mounted with `#[path]` from `tests/cli_remote.rs` only, never from `tests/support/mod.rs` (fact 59).
- **Test isolation.**
  - Every CLI child goes through `tests/support/cli.rs`, `tests/support/pty.rs` or the SSH harness.
  - The SSH harness keeps system identity out (fact 60). The `cli`-target identity cases call `require_no_system_identity()` (`tests/cli/repository.rs`).
  - An identity case built on `World` removes the local identity `World` sets (fact 44) before it starts.
  - Tests never touch developer keys, global Git configuration, the real terminal or real application data.
  - PTY cases are `#[cfg(unix)]` (fact 43).
- **Pre-existing tests.** Only named edits E-1 to E-12, in the tasks the design names. `tests/support/pty.rs` gains one additive helper. Any other semantic edit, including a new lint on an old test, stops for owner review.
- **Outcome tables.** Each table's test is written first, in its task's first step, and run to see it fail before any implementation. The test and the code that makes it pass are committed together, so no commit is left red or uncompiled. If the code answers a row differently, stop for a ruling. Never edit a table to fit.
- **Test runs (owner, 2026-10-17, 2026-10-19 and 2026-10-24).**
  - Each task runs only the targets and filters its steps name. Check that more than zero tests ran.
  - A task that edits `src/repository/remote/` also runs `cargo test --all-features --locked --lib remote`, `--lib local_sync` and `--lib integration`, each as its own command. The gate's local executor calls into those modules.
  - Cargo takes **one** positional test filter per command.
  - Clippy in a task step is `cargo clippy --all-targets --all-features --locked -- -D warnings`.
  - The Part checkpoints (Tasks 6 and 11) run the checkpoint gate. The full suite runs once, in Task 15, detached, with a two-hour budget. It took 1 h 44 min in C3. Use a 600-second tool timeout for gate commands.
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
  devenv shell -- cargo test --all-features --locked --test mutation_remote --test comment_publication --test remote_merge_recovery --test cli_remote
  ```

  After the gate:
  1. `git fetch origin` and `git merge-tree --write-tree HEAD origin/main`;
  2. an independent review of the Part's commit range against the Cycle, the design and this plan;
  3. fix or rule every finding, and rerun the gate after the fixes;
  4. add a ticket comment.
- **Goldens.**
  - Update with `MANYHANDS_UPDATE_GOLDEN=1`, never under `CI`.
  - Read every regenerated golden's diff before committing.
  - Register new cases in `tests/support/mutation_golden.rs` (`CASES`), `tests/support/golden.rs` (`CASES`) or `tests/support/cli.rs` (`CLI_CASES`, `CLI_TEXT_CASES`).
  - Names follow R-28.
- **No subagents.** Implementers never dispatch subagents.
- **Commits.** Commit at the end of every task. Messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- **Execution ledger.** Task 0 creates `docs/plans/2026-10-26-wave-03-cli-04-discussion-and-sync-execution.md`, and every task commits it.
- **Ticket comments.** Add one at Task 0, at each task's completion, at each Part checkpoint and at review-ready.
  - Until Task 15's switch, write each under `.manyhands/comments/01M4CC0VMWEQQ9G4W0DAG7ADPY/<ULID>.md` in the canonical schema.
  - Take the ULID from `id new`, and `created_at` from the UTC clock.
- **Authorization.**
  - Pushing the ticket branch is the standing habit.
  - Never push another branch, open a pull request, merge, close the ticket, create a ticket, run `ticket save`, or clean up without separate owner authorization.
  - Task 15's dogfooding needs its own go-ahead at that step.
  - Native CI runs after merge, on main, only with the owner's approval of that run.

## Review Focus

These are inputs the spec implies but no table lists. Each has its test in the owning task.

1. **A comment body with CRLF line endings, non-BMP characters, a line `---`, and no final newline.** `comment add` writes it byte for byte into the body, and `comment list` returns it unchanged. Task 2b, Step 4.
2. **A conflict whose path holds a terminal escape sequence or a newline.**
   - `conflict show`'s human output and every diagnostic escape it, without escaping the library's display path twice. Task 3, Step 4 and Task 9, Step 4.
   - Its token still resolves, and `conflict resolve` writes only that path. Task 4b, Step 3.
3. **`conflict resolve` with the same token twice, or with a token from another operation's conflict.** The duplicate is `invalid_input` and the foreign token is `external_change` (R-33). Neither writes, and no source text appears in the envelope or the stores. Task 4a, Step 4.
4. **A Ctrl-C at a confirmed verb's consent prompt, before execution.** It is the prompt's own cancellation, exit 130. The handler is not installed, and nothing is reserved. `operation abandon` in Task 7, Step 7; `item republish` over SSH in Task 12b, Step 2.
5. **`comment add` whose `parent_id` is the comment's own `comment_id`.** It is `item_not_found` before any write, with no binding row left (design, Comment Submission). Task 2b, Step 4.

---

## File Map And Task Order

| Task | Part | Creates | Modifies |
| --- | --- | --- | --- |
| 0 | — | `docs/plans/2026-10-26-wave-03-cli-04-discussion-and-sync-execution.md` | `docs/research/wave-03-api-audit.md` |
| 1 | A | `tests/mutation_remote/merging.rs`, `schemas/v1/synchronization_conflict_mutation.schema.json`, `conflict_next.schema.json`, golden `repo_sync_recovery_required.json` | `bind/sync.rs`, `outcome.rs`, `dto.rs`, `mod.rs`, `tests/mutation_remote.rs` (module and `CASES`, E-3), E-4, E-5, E-7, E-8, E-10 |
| 2a | A | `tests/mutation_remote/comment_domain.rs` | `comment_publication.rs`, `src/repository.rs` (`checkpoint_comment_locally_with`), `recovery.rs`, `bind/sync.rs` (`Watch` constructor), `tests/mutation_remote.rs` |
| 2b | A | `bind/comment.rs`, `tests/mutation_remote/comment.rs`, `schemas/v1/comment_mutation.schema.json`, mutation goldens `comment_add*` | `command.rs`, `mod.rs`, `dto.rs`, `outcome.rs`, `bind/mod.rs`, `tests/mutation_remote.rs`, E-6, E-7, E-8, E-9, E-10 |
| 3 | A | `remote/conflict_read.rs`, `tests/mutation_remote/conflict_read.rs`, `schemas/v1/conflict.schema.json`, `conflict_path.schema.json`, read goldens `conflict_show_*` | `remote/mod.rs`, `read/mod.rs`, `read/dto.rs` (exports), `src/repository.rs` (re-export), `tests/mutation_remote.rs`, `tests/support/golden.rs`, E-8 |
| 4a | A | `bind/conflict.rs`, `tests/mutation_remote/resolution.rs`, `schemas/v1/conflict_resolution_mutation.schema.json`, mutation goldens `conflict_resolve*` | `remote/{sync,state,merge}.rs` (R-14, R-35, R-41), `command.rs`, `mod.rs`, `dto.rs`, `outcome.rs`, `bind/mod.rs`, `tests/mutation_remote.rs`, `tests/repository_enablement.rs` (E-12), E-6, E-7, E-8, E-9, E-10 |
| 4b | A | `tests/mutation_remote/resume.rs` | `bind/resume.rs`, `dto.rs` (`from_stored`), `tests/mutation_remote.rs`, E-11 |
| 5 | A | `bind/republish.rs`, `tests/mutation_remote/republish.rs`, `schemas/v1/item_republish_preview.schema.json`, mutation goldens `item_republish*` | `remote/{sync,state}.rs` (the push-only path), `mutation/progress.rs` (`Session::republish`), `command.rs`, `mod.rs`, `dto.rs`, `schemas/v1/preview_effect.schema.json`, `src/desktop/view/confirm.rs` (one arm), `src/bin/manyhands-cli/human_admin.rs` (the two preview matches), E-5, E-6, E-7, E-8, E-9 |
| 6 | A | — | ledger, ticket comment |
| 7 | B | `interrupt.rs`, `interrupt/unix.rs`, `interrupt/windows.rs`, `collaboration.rs`, `human_collaboration.rs` and their tests, `tests/cli/collaboration.rs`, `tests/cli_remote/interruption.rs` (the spike case) | `src/bin/manyhands-cli.rs`, `catalog.rs`, `mutate.rs` (the `Boundary` impl), `dispatch.rs`, `tests/cli.rs`, `tests/cli_remote.rs`, `tests/support/cli.rs`, E-1 |
| 8 | B | `discussion.rs`, `discussion_tests.rs`, `tests/cli/discussion.rs`, `schemas/input/v1/comment_add.schema.json`, `tests/fixtures/cli_input_v1/comment_add.json` | `catalog.rs`, `dispatch.rs`, `human_collaboration.rs`, `tests/cli.rs`, `tests/support/cli.rs`, E-1 |
| 9 | B | `conflict.rs`, `conflict_tests.rs`, `tests/cli/conflicts.rs`, `schemas/input/v1/conflict_resolve.schema.json`, `tests/fixtures/cli_input_v1/conflict_resolve.json` | `input.rs`, `input_tests.rs`, `catalog.rs`, `dispatch.rs`, `human_collaboration.rs`, `tests/cli.rs`, `tests/support/cli.rs`, E-1 |
| 10 | B | `tests/cli/republish.rs` | `collaboration.rs`, `catalog.rs`, `catalog_tests.rs` (E-2), `dispatch.rs`, `tests/cli.rs`, `tests/support/cli.rs`, E-1 |
| 11 | B | — | ledger, ticket comment |
| 12a | C | `tests/support/cli_pair.rs`, `tests/cli_remote/collaboration.rs` | `tests/cli_remote.rs` |
| 12b | C | `tests/cli_remote/conflicts.rs` | `tests/cli_remote.rs` |
| 13 | C | `tests/cli/collaboration_exits.rs` | `tests/cli_remote/interruption.rs`, `tests/cli_remote.rs`, `tests/cli.rs`, `tests/support/pty.rs` (one additive helper) |
| 14 | — | — | `docs/RFC/cli-contract.md`, `docs/RFC/git-workflow-and-conflict-recovery.md`, `docs/RFC/canonical-content-and-comment-schema.md`, the F2 design, `docs/Waves/wave-03-dogfooding.md`, `docs/research/wave-03-api-audit.md` |
| 15 | — | — | the Cycle, the design, the ledger, comments, `AGENTS.md`, the skills of fact 48 |

Tasks run in order. Part A's tasks extend the same closed inventories (fact 49), so none can run in parallel. Part B's tasks each flip verbs in `catalog.rs`. A verb becomes available only in the task that dispatches it, gives it its schema and example, and adds its goldens.

---

## Task 0: Baseline, Re-Grounding After The Gate, Audit Refresh And Ledger

**Files:**
- Create: `docs/plans/2026-10-26-wave-03-cli-04-discussion-and-sync-execution.md`
- Modify: `docs/research/wave-03-api-audit.md`

- [ ] **Step 1: Confirm the base.**
  - `git fetch origin`. Confirm that `git show origin/main:.manyhands/tickets/01M57SKVF6TQ3QM6AXJ35JGGSY/ticket.md` has `closed_at`. It did at `a372edb`, on 2026-10-27.
  - `git merge origin/main`, then `git merge-tree --write-tree HEAD origin/main`. Record whether main moved past `a372edb`. If it did not, the facts stand as the design's 2026-10-27 re-grounding recorded them, and Step 2 re-checks only fact 81's red test. If it did, Step 2 re-checks every fact.
  - Read `git show origin/main:AGENTS.md`.
  - For each known-limit ticket in the Cycle's Entry Evidence, check its `closed_at` on `origin/main` the same way. A closed one changes a known limit, and the controller rules before Task 1.
  - Check whether D3 (`01M4CC0VN0MFTGKQ4HGTTC5H63`) is closed on main, and record it. It decides who resolves the `confirm.rs` merge (decision 10).
  - Record HEAD and `origin/main`.
- [ ] **Step 2: Re-check the Baseline Facts** (1–86) against the merged source, as Step 1 says, and record each as confirmed or changed, with its new paths and line numbers. In particular:
  - any change since `a372edb` to the files Part A touches, or to the gate's `local_sync/`, `integration/`, `local_control.rs` and `bind/local_sync.rs` (facts 25, 53 and 80);
  - fact 28: `set_local_identity` is still not refused by a pending operation;
  - what `operation abandon` answers for a comment operation whose publication is pending (fact 10). If it would close the comment's binding while its checkpoint stays unpublished, stop for a ruling;
  - fact 81: whether `registry_creates_repository_and_discovery_metadata_schema` still fails on main for the gate's tables. Run `devenv shell -- cargo test --all-features --locked --test repository_enablement registry_creates_repository_and_discovery_metadata_schema`. If it still fails, record it, and stop for the owner before Task 4a (E-12).

  A changed fact that alters a task stops here for a controller ruling.
- [ ] **Step 3: Prove E-1 to E-12 are the only named edits.**

  ```sh
  grep -rn -E 'refuse_divergence|REFUSING|merge_required|MergeRequired|is_comment_synchronization|LeftToInspect' tests src/repository/mutation
  grep -rln -E 'comment add|item sync|repo sync|item republish|conflict (show|resolve)|operation (resume|abandon)|"C4"' src/bin/manyhands-cli tests/cli tests/cli_remote tests/cli.rs tests/support/cli.rs tests/fixtures/cli_v1
  grep -rn -E 'Command::ALL|bound_commands\(\)|HOMES|DATA_ROOTS|CODES_NO_MUTATION_GOLDEN_HAS|CODES_NO_FIXTURE_CAN_PRODUCE|contract_enumerations|PreviewDetailsDto::|PreviewEffect::|MutationDataDto::' tests src --include=*.rs
  ```

  Classify every hit as one of:
  - the production code a task changes;
  - an E-1 to E-12 site;
  - a loop over an inventory that takes the new entries unchanged;
  - a single-variant match with a fallback, such as `let Some(MutationDataDto::LocalContextSync(..)) = … else { panic!(…) }` in `tests/mutation_replay/local_sync*.rs`, or the wildcard matches at `tests/mutation_remote.rs:1441`, `tests/mutation_remote/host.rs:332` and `tests/mutation_confirmation/abandon.rs:825`. A new variant leaves these unchanged.

  The 2026-10-27 re-grounding ran these greps on `a372edb`: grep 3 had 311 hits, and greps 1 and 2 had no gate-added hits.

  For E-3, record each listed test as binding-level (edited) or domain-level (unchanged), from whether it reaches the binding's `SETTINGS` or its own `REFUSING`. Any other hit stops for an owner ruling.
- [ ] **Step 4: Baseline checks.**

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo check --locked --features desktop
  devenv shell -- cargo clippy --all-targets --locked --features desktop -- -D warnings
  ```

  Then run the full suite once, detached, with a two-hour budget:

  ```sh
  devenv shell -- cargo test --all-features --locked
  ```

  Record per-target counts. A failure outside C4's files stops for the owner. It is not fixed here. Fact 81's test is expected to fail unless main has fixed it, and its failure is recorded with that reference.
- [ ] **Step 5: Refresh the API audit.**
  - Add a section, "Wave 03 CLI Cycle C4 refresh", dated, citing decision 16.
  - Update the C4 rows in place: `comment add`, `item sync`, `repo sync`, `conflict show`, `conflict resolve`, `operation resume` and `operation abandon`. Add `item republish`. For each, record the planned binding and signature, whether it is confirmed, its input type, and every result code it can return as designed. Read the existing codes from `outcome.rs`, `execute.rs`, `confirm.rs` and the binding.
  - Correct the stale "missing" and "pending Wave 02" claims. Do not add a second row for any verb.
  - Under `#### C4 exit-coverage union`, give the union of those codes in one fenced `text` block, space-separated.
- [ ] **Step 6: Create the execution ledger.** Give it front matter like the design's and a new document ULID from `id new`. Its sections are:
  - baseline;
  - confirmed facts;
  - audit refresh;
  - Cycle decisions 1–20, rulings R-1 to R-41 and named edits E-1 to E-12;
  - owed native checks, starting with the Windows Ctrl-C check;
  - the planning review record;
  - an empty task log.
- [ ] **Step 7: Commit** `docs: record C4 baseline after the gate and refresh the API audit`. Add a ticket comment and push the ticket branch.

## Task 1: Merging Synchronization

**Files:**
- Create: `tests/mutation_remote/merging.rs`, `schemas/v1/synchronization_conflict_mutation.schema.json`, `schemas/v1/conflict_next.schema.json`, `tests/fixtures/mutation_v1/repo_sync_recovery_required.json`
- Modify: `src/repository/mutation/bind/sync.rs`, `outcome.rs`, `dto.rs`, `mod.rs`, `tests/mutation_remote.rs` (module, `CASES`, E-3), `tests/support/mutation_golden.rs` (E-4), `outcome_tests.rs` (E-5), `tests/mutation_contract.rs` (E-7), `tests/read_contract.rs` (E-8), `tests/support/mutation.rs` (E-10)

**Interfaces:**
- Consumes: `Session::synchronize` (fact 54); `inspect_synchronization_recovery` (fact 29); `World` (`tests/support/remote_world.rs:139`) and its peer helpers.
- Produces:
  - `pub struct ConflictNextDto { pub operation_id: String, pub next_action: ConflictNextAction }` and `contract_enum! ConflictNextAction { Resolve => "resolve", ExternalRepair => "external_repair" }`, in `mutation/dto.rs`;
  - `pub struct SynchronizationConflictMutationDto { pub r#type: SynchronizationConflictType, pub conflict: ConflictNextDto }`, where the single-value enum is `"synchronization_conflict"`;
  - `MutationDataDto::SynchronizationConflict(SynchronizationConflictMutationDto)`;
  - `pub(super) const MERGING: SynchronizationSettings` and `pub(crate) fn conflict_next(service: &RepositoryService, root: &Path, operation_id: OperationId) -> ConflictNextDto` in `bind/sync.rs`, which later tasks call;
  - a test helper `fn leave_local_sync_conflict(world: &World) -> OperationId` in `tests/mutation_remote.rs`, which Tasks 2b, 4a and 5 reuse (R-38).

- [ ] **Step 1: Write the divergent-synchronization table as cases** in `tests/mutation_remote/merging.rs`, one `c4_merging_<row>` case per row, for both `item sync` and `repo sync`, through `RepositoryService::execute`.
  - Use `World` and its raw peer to make the remote diverge, with `commit` plus `push_peer`.
  - Each case asserts code, outcome, effects, data and recovery exactly as the row says. For merge rows it also asserts parent order and subject, and that the remote keeps its earlier tip as an ancestor.
  - The identity rows remove `World`'s local identity first. They run `repo identity-set` through `prepare` and `execute` with its confirmation (it is confirmed), then re-execute the first `MutationCall`.
  - The unfinished-local-sync row (R-38) first leaves a `context sync-local` stopped in a conflict, through `Mutation::LocalContextSync`, as `ConflictWorld::with_incoming` does (`tests/mutation_replay/local_sync.rs:164-212`). Later tasks reuse the same helper for their rows.

  Register the module and its cases in `tests/mutation_remote.rs`.
- [ ] **Step 2: Run it to see it fail.** `devenv shell -- cargo test --all-features --locked --test mutation_remote -- c4_merging`. Expected: it does not compile, because the new DTOs do not exist yet. That is the failing state; nothing is committed until Step 6.
- [ ] **Step 3: Implement.**
  - Replace `SETTINGS` with `MERGING`, `refuse_divergence: false`, and update the module doc, the comments of fact 6, and `bind/sync.rs:100` and `:590` (fact 84), which say nothing merges (R-16).
  - On `ConflictPending`, return `recovery_required` with `integration: pending` and the data from `conflict_next` (R-8). `next_action` is `resolve` when the inspection's eligibility is `EligibleCanonical`, else `external_repair`.
  - The identity rows need no workaround (R-17).
  - Add the two schemas, and register the DTO in `DATA_ROOTS` and `contract_enumerations`.
- [ ] **Step 4: Apply E-3, E-4, E-5 (the conflict rows and the `merge_required` tests), E-7, E-8 and E-10,** exactly as the design's table says, and only to the tests Task 0 classified binding-level.
- [ ] **Step 5: Verify.**

  ```sh
  devenv shell -- cargo test --all-features --locked --test mutation_remote
  devenv shell -- cargo test --all-features --locked --lib mutation::outcome
  devenv shell -- cargo test --all-features --locked --test mutation_contract
  devenv shell -- cargo test --all-features --locked --test read_contract
  devenv shell -- cargo test --all-features --locked --test mutation_bridges
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  ```

- [ ] **Step 6: Commit** `feat: merge divergent history in item and repo sync`. Add a ticket comment and push.

## Task 2a: The Library's Comment Submission

**Files:**
- Create: `tests/mutation_remote/comment_domain.rs`
- Modify: `src/repository/comment_publication.rs`, `src/repository.rs` (`checkpoint_comment_locally_with`), `src/repository/recovery.rs` (releasing a binding), `src/repository/mutation/bind/sync.rs` (a `pub(super)` `Watch` constructor), `tests/mutation_remote.rs`

**Interfaces:**
- Consumes: fact 17's types; `Session::synchronize` and `Watch` (fact 54); fact 57's identity rules.
- Produces:
  - `pub(crate) type Synchronizer<'a> = dyn FnMut(SynchronizeRemoteRequest) -> Option<Result<SynchronizationResult, SynchronizationError>> + 'a`;
  - crate-private `submit_comment_with(&self, request: PublishCommentRequest, author: Option<CommentAuthor>, restart: bool, release_unwritten: bool, synchronize: &mut Synchronizer)` and `retry_comment_publication_with(&self, request: RetryCommentPublicationRequest, synchronize: &mut Synchronizer)` (R-3, R-4);
  - `pub(crate) struct CommentAuthor` with `parse(&str) -> Result<CommentAuthor, CommentAuthorError>` (R-7);
  - crate-private `checkpoint_comment_locally_with(request, config, author: Option<CommentAuthor>, release_unwritten: bool)`, which the unchanged `checkpoint_comment_locally` delegates to with `None` and `false`, so its behavior is unchanged (fact 73);
  - `pub(super) fn Watch::new(accepted: Accepted, …) -> Watch` in `bind/sync.rs`.

- [ ] **Step 1: Write the domain cases first** in `tests/mutation_remote/comment_domain.rs`, as `c4_comment_domain_` cases, calling the crate-private functions directly:
  - a given author is written verbatim, and a malformed one is refused before any write;
  - a parent that is not a comment of the item is refused with no binding row left;
  - on `IdentityRequired` with nothing written and `release_unwritten: true`, the binding is deleted and the row marked `rejected` in one transaction, and the same operation then binds afresh (R-34), including after another checkpoint lands in the context;
  - on `IdentityRequired` after an earlier attempt wrote the file, the binding and row are kept;
  - with `release_unwritten: false`, the public path's behavior is unchanged;
  - a synchronizer that records its requests sees the child `synchronization_id`, `MERGING` and `restart` as given.
- [ ] **Step 2: Run them to see them fail.** `devenv shell -- cargo test --all-features --locked --test mutation_remote -- c4_comment_domain`. Expected: it does not compile.
- [ ] **Step 3: Implement.**
  - `submit_comment_with` and `retry_comment_publication_with`. The public `submit_comment` and `retry_comment_publication` call them with a synchronizer over `synchronize_remote` (unobserved, default settings), so their behavior and tests are unchanged.
  - `checkpoint_comment_locally_with`: the author's text replaces fact 20's format when given; the parent check moves before the binding row's insert (fact 19); and, when asked, a binding that wrote nothing is released on `IdentityRequired` (R-34).
  - The `Watch` constructor.
- [ ] **Step 4: Verify.**

  ```sh
  devenv shell -- cargo test --all-features --locked --test mutation_remote -- c4_comment_domain
  devenv shell -- cargo test --all-features --locked --test comment_publication
  devenv shell -- cargo test --all-features --locked --test remote_merge_recovery
  devenv shell -- cargo test --all-features --locked --lib repository
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  ```

- [ ] **Step 5: Commit** `feat: let comment submission take a synchronizer and an author`. Add a ticket comment and push.

## Task 2b: The Comment Submission Binding

**Files:**
- Create: `src/repository/mutation/bind/comment.rs`, `tests/mutation_remote/comment.rs`, `schemas/v1/comment_mutation.schema.json`, mutation goldens `comment_add`, `comment_add_push_rejected`, `comment_add_ticket_closed`, `comment_add_item_not_found`
- Modify: `command.rs`, `mod.rs`, `dto.rs`, `outcome.rs`, `bind/mod.rs`, `tests/mutation_remote.rs`, E-6, E-7, E-8, E-9, E-10

**Interfaces:**
- Consumes: Task 2a's functions; `MERGING`, `ConflictNextDto` and `conflict_next` (Task 1); the binding table (fact 21).
- Produces:
  - `pub struct CommentAddInput { pub root: PathBuf, pub kind: AuthoringKind, pub id: ItemId, pub comment_id: ItemId, pub parent_id: Option<ItemId>, pub body: String, pub created_by: Option<String> }`;
  - `Mutation::CommentAdd(CommentAddInput)`, with `Command::CommentAdd => "comment add"`, and the predicates of the design's table (R-32);
  - `pub struct CommentMutationDto { r#type, item_id, comment_id, parent_id: Option<String>, created_by: Option<String>, synchronization_id: String, publication: PublicationEffect, pending_reason: Option<CommentPendingReason>, conflict: Option<ConflictNextDto> }`;
  - `pub(crate) fn comment_synchronization(connection, root, operation_id) -> Option<(OperationId /*comment*/, OperationId /*synchronization*/)>`, which resolves either ID (R-5).

- [ ] **Step 1: Write the comment-submission table as cases** in `tests/mutation_remote/comment.rs`, one `c4_comment_<row>` case per row, through `execute`.
  - Use `World` and `use super::*` for the target's private helpers, as the existing cases do.
  - The server-refusal row uses the fixture's refusal hook. The conflict rows have the peer push a conflicting edit to the item's context branch. The other-item row first leaves a conflict on another item. The unfinished-local-sync row uses Task 1's helper (R-38).

  Register the module and its cases.
- [ ] **Step 2: Run it to see it fail.** `devenv shell -- cargo test --all-features --locked --test mutation_remote -- c4_comment`. Expected: it does not compile, because `Mutation::CommentAdd` does not exist yet. Nothing is committed until Step 6.
- [ ] **Step 3: Implement the binding** in `bind/comment.rs`.
  - It follows `bind/sync.rs`'s shape: `prepare`, `run`, `rerun`, `reenter`, effects from the journal.
  - Its synchronizer closes over `call.session.synchronize(service, request, MERGING, &mut watch)`. `watch` comes from Task 2a's constructor, with an `Accepted` carrying the child `synchronization_id`, as `resume.rs:647-655` builds one. Its owner is the `comment add` request.
  - Standing (R-34): the binding calls `submit_comment_with` with `release_unwritten: true`, and keeps the record accepted on `IdentityRequired`. A `Saved` result with publication pending is `Standing::Owed`, except a final child or a code never finished by the same request, which is `Standing::Final`. `ended_without_checkpoint` answers `SaysNothing`.
  - Re-entry calls `submit_comment_with` with the full input and `restart: true` (fact 65).
  - Map outcomes to the table (R-6).
  - Add the command to every closed inventory of fact 49 (E-6, E-7, E-8, E-9, E-10).
- [ ] **Step 4: Review Focus 1 and 5,** as `c4_comment_` cases.
  - A body with CRLF, a non-BMP character, a `---` line and no final newline round-trips through `list_comments`, byte for byte.
  - A self-parent is `item_not_found`, and no binding row stays.
  - `request_mismatch`: the same request ID sent again with another body. It is exit 4 when the first request checkpointed, else 2 (`replay.rs:153-163`).
- [ ] **Step 5: Verify.**

  ```sh
  devenv shell -- cargo test --all-features --locked --test mutation_remote
  devenv shell -- cargo test --all-features --locked --test comment_publication
  devenv shell -- cargo test --all-features --locked --test mutation_replay
  devenv shell -- cargo test --all-features --locked --test mutation_bridges
  devenv shell -- cargo test --all-features --locked --test mutation_contract
  devenv shell -- cargo test --all-features --locked --test read_contract
  devenv shell -- cargo test --all-features --locked --lib mutation
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  ```

- [ ] **Step 6: Commit** `feat: bind comment submission with authorship and safe points`. Add a ticket comment and push.

## Task 3: Conflict Inspection Read

**Files:**
- Create: `src/repository/remote/conflict_read.rs`, `tests/mutation_remote/conflict_read.rs`, `schemas/v1/conflict.schema.json`, `schemas/v1/conflict_path.schema.json`, read goldens `conflict_show_conflict_pending`, `conflict_show_repaired`, `conflict_show_complete`
- Modify: `src/repository/remote/mod.rs`, `read/mod.rs` and `read/dto.rs` (exports), `src/repository.rs` (re-export), `tests/mutation_remote.rs`, `tests/support/golden.rs`, E-8

**Interfaces:**
- Consumes: facts 29–32 and 35; `comment_synchronization` (Task 2b).
- Produces:
  - `pub fn show_conflict(&self, repo: &ResolvedRepository, operation_id: OperationId) -> Result<ConflictDto, ReadError>`;
  - `ConflictDto` and `ConflictPathDto`, as the design's tables, with `request_id`, and `contract_enum!`s `ConflictState`, `ConflictReadNextAction` (`resolve`, `retry_resolution`, `external_repair`, `resume`, `synchronize`, `none`), `ConflictPathKind` and `ConflictPathStructure`;
  - `pub(crate) fn path_tokens(inspection: &SynchronizationConflictInspection) -> Vec<(String, ConflictPathToken)>` and `pub(crate) fn observation_text(observation: &ConflictObservation) -> String` (R-9), which Task 4a uses.

- [ ] **Step 1: Write the conflict-states table as cases** in `tests/mutation_remote/conflict_read.rs`, one `c4_conflict_read_<row>` case per row.
  - `World`'s peer makes the conflicts.
  - The repaired and refused rows follow `install_repair`'s Git steps (`tests/remote_merge_recovery.rs:2335`) and `refused_false_repairs`'.
  - The `busy` row holds the repository lease from a library-level actor.
- [ ] **Step 2: Run it to see it fail.** `devenv shell -- cargo test --all-features --locked --test mutation_remote -- c4_conflict_read`. Expected: it does not compile.
- [ ] **Step 3: Implement** `conflict_read.rs`.
  - Look the operation ID up in the gate's local-sync records first, and answer `invalid_input` for a local sync (decision 19, R-37).
  - Read the step's phase with the internal state functions, then inspect when a conflict is live (fact 29). At `ResolutionPrepared` the inspection still issues an observation (fact 86). If it does not, stop at a design checkpoint.
  - Derive the tokens and the observation by R-9.
  - Report each path's kind and structure by calling `crate::repository::integration::validate::classify_conflict_entry` on each index conflict, because the inspection carries only token and eligibility (fact 30, decision 12). Map all eight structures, `incompatible_closures` included (R-40).
  - Read the sides for readable paths (fact 32, R-10), set `historical` (R-39), and escape display paths by R-11's rule.
  - Report the repaired and refused states with `validates_external_integration`, read-only.
  - Accept a comment's operation ID (R-5).
  - Prove it writes nothing (R-12): a case compares the database file's bytes before and after.
- [ ] **Step 4: Schemas, goldens and Review Focus 2.**
  - Closed schemas, each registered as a data root, and the enums in `contract_enumerations` (E-8).
  - A `c4_conflict_read_hostile_path` case: a path with an escape sequence and a newline. Its token resolves, and no raw control byte appears in a diagnostic or a `Debug` rendering.
  - A `c4_conflict_read_display_path` case pins R-11 byte for byte: a control byte, a backslash, a non-ASCII UTF-8 character and an invalid byte.
  - A `c4_conflict_read_incompatible_closures` case: two clones close the same ticket with different `closed_at`, and the read reports that path's structure and `external_repair` (R-40).
- [ ] **Step 5: Verify.**

  ```sh
  devenv shell -- cargo test --all-features --locked --test mutation_remote -- c4_conflict_read
  devenv shell -- cargo test --all-features --locked --test read_contract
  devenv shell -- cargo test --all-features --locked --test mutation_contract
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  ```

- [ ] **Step 6: Commit** `feat: read synchronization conflicts with string tokens`. Add a ticket comment and push.

## Task 4a: Conflict Resolution Binding

**Files:**
- Create: `src/repository/mutation/bind/conflict.rs`, `tests/mutation_remote/resolution.rs`, `schemas/v1/conflict_resolution_mutation.schema.json`, mutation goldens `conflict_resolve`, `conflict_resolve_external_change`, `conflict_resolve_invalid_input`, `conflict_resolve_original_request_required`
- Modify: `src/repository/remote/{sync,state,merge}.rs` (R-14, R-35, R-41), `command.rs`, `mod.rs`, `dto.rs`, `outcome.rs`, `bind/mod.rs`, `tests/mutation_remote.rs`, `tests/repository_enablement.rs` (E-12), E-6, E-7, E-8, E-9, E-10

**Interfaces:**
- Consumes: `path_tokens`, `observation_text` and `show_conflict` (Task 3); `MERGING` and `conflict_next` (Task 1); `retry_comment_publication_with` (Task 2a); fact 33.
- Produces:
  - `pub struct ConflictResolveInput { pub root: PathBuf, pub operation_id: OperationId, pub observation: String, pub resolutions: Vec<ConflictResolutionInput> }` and `pub struct ConflictResolutionInput { pub token: String, pub source: String }`;
  - `Mutation::ConflictResolve`, with `Command::ConflictResolve => "conflict resolve"`, and the predicates of the design's table (R-32);
  - `ConflictResolutionMutationDto { r#type, operation_id, commit_oid: Option<String>, publication: PublicationEffect, conflict: Option<ConflictNextDto> }`.

- [ ] **Step 1: Write the interrupted-resolution table as cases** in `tests/mutation_remote/resolution.rs`, one `c4_resolution_<row>` case per row.
  - Interruptions use the library's `FailurePoint`s around resolution and publication. Find them with `grep -n 'FailurePoint::' src/repository/remote/sync.rs src/repository/remote/state.rs`, and record them in the ledger. They are in `sync.rs:4625-5253` and `state.rs`; `integration/` has none, and the `LocalSync*` points are the gate's local executor's (fact 64).
  - The unfinished-local-sync row uses Task 1's helper (R-38).
  - Row 1 has no failure point (fact 64). Its case installs an SQL trigger that refuses the first resolution-path insert.
  - Each retry runs in a fresh `RepositoryService` opened on the same data directory (decision 13).
- [ ] **Step 2: Run it to see it fail.** `devenv shell -- cargo test --all-features --locked --test mutation_remote -- c4_resolution`. Expected: it does not compile.
- [ ] **Step 3: Implement the durable state (R-14, R-35).**
  - Derive the attempt ID from a domain-separated BLAKE3 digest of the request ID, the synchronization's ID, the integration window and the stage ordinal (fact 72).
  - Record the association in a new table, `remote_resolution_owners`, with the attempt ID, the request ID and the presented input's digest, as values with no foreign key, created by its own migration step (R-41). No audited table changes (fact 75). Apply E-12.
  - Make the library finish an attempt without storing a body:
    - at `ResolutionPrepared`, with the presented input and the live inspection;
    - at `CommitPrepared` or `Applied`, from `remote_resolution_attempts` and `remote_resolution_paths` alone (fact 34). That also serves `operation resume` (decision 18).
  - Rebuild the library's tokens only for `reconcile_resolution_candidate`. Find each path's bytes by hashing every path of the candidate tree and matching `path_digest` (fact 70).
  - Refuse a second attempt beside an unfinished one (fact 71), except decision 18's supersede of a `ResolutionPrepared` attempt whose written paths still hold exactly what it wrote.
  - Fill `ConflictDto.request_id` in `conflict_read.rs` from the new table. Until this task, Task 3 reports it as null. Extend Task 3's `c4_conflict_read_` case for `resolution_prepared` to assert it.
  - If any row cannot be met within the Global Constraints' library files, **stop** at a design checkpoint, and record what each row answers.
- [ ] **Step 4: Implement the binding** in `bind/conflict.rs`.
  - It follows steps 1–7 of the design's Conflict Resolution section: syntax, replay first, association, validation in step 4's order, resolve, continuation (R-15) and outcome mapping.
  - The continuation's `Watch` owner is the resolve request (R-35).
  - Add the command to every closed inventory (E-6, E-7, E-8, E-9, E-10).
  - **Review Focus 3** as `c4_resolution_` cases: a duplicate token is `invalid_input`, and a token from another operation's conflict is `external_change`. Neither writes, and no source text appears in the envelope or the request stores.
- [ ] **Step 5: Verify.**

  ```sh
  devenv shell -- cargo test --all-features --locked --test mutation_remote
  devenv shell -- cargo test --all-features --locked --test mutation_replay
  devenv shell -- cargo test --all-features --locked --test mutation_bridges
  devenv shell -- cargo test --all-features --locked --test mutation_contract
  devenv shell -- cargo test --all-features --locked --test read_contract
  devenv shell -- cargo test --all-features --locked --test repository_enablement
  devenv shell -- cargo test --all-features --locked --test remote_merge_recovery
  devenv shell -- cargo test --all-features --locked --lib mutation
  devenv shell -- cargo test --all-features --locked --lib remote
  devenv shell -- cargo test --all-features --locked --lib local_sync
  devenv shell -- cargo test --all-features --locked --lib integration
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  ```

  `--test mutation_replay` proves that `tests/mutation_replay.rs:400` still passes, and `--lib remote` proves `state_tests.rs`' drop tests still pass (R-41). If `repository_enablement` fails only for the gate's tables (fact 81), stop for the owner.

- [ ] **Step 6: Commit** `feat: bind conflict resolution that the same request can finish`. Add a ticket comment and push.

## Task 4b: Resume Routes

**Files:**
- Create: `tests/mutation_remote/resume.rs`
- Modify: `src/repository/mutation/bind/resume.rs`, `dto.rs` (`from_stored`'s `OperationResume` arm), `tests/mutation_remote.rs`, E-11

**Interfaces:**
- Consumes: `retry_comment_publication_with` (Task 2a) and `comment_synchronization` (Task 2b); `MERGING` and `conflict_next` (Task 1).
- Produces: resume's new routes (R-20).

- [ ] **Step 1: Write the resume table as cases** in `tests/mutation_remote/resume.rs`, one `c4_resume_<row>` case per row. Add a case showing that `list_operations` lists the comment and resolved-synchronization operations that resume can now act on. The local-context-sync row asserts that the gate's routing answers exactly as before C4, using Task 1's helper.
- [ ] **Step 2: Run it to see it fail.** `devenv shell -- cargo test --all-features --locked --test mutation_remote -- c4_resume`.
- [ ] **Step 3: Implement the routes** in `bind/resume.rs`, in place of `LeftToInspect` for those cases:
  - finishing a prepared resolution candidate from durable state (decision 18);
  - for an attempt with only written paths, `original_request_required` naming the attempt's request from `remote_resolution_owners` (R-20, decision 20).

  Leave the gate's local routing untouched (fact 9). Extend `from_stored`, keeping the gate's arm guarded on `type: local_context_sync` first, and matching C4's shapes by their own `type` after it (fact 49).
  - Apply E-11 to the tests Task 0 found.
  - **Review Focus 2:** a `c4_resume_hostile_path` case resolves a conflict on a hostile path through `execute`, and only that path changes in the resolution commit.
- [ ] **Step 4: Verify.**

  ```sh
  devenv shell -- cargo test --all-features --locked --test mutation_remote
  devenv shell -- cargo test --all-features --locked --test mutation_replay
  devenv shell -- cargo test --all-features --locked --test mutation_contract
  devenv shell -- cargo test --all-features --locked --lib mutation
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  ```

- [ ] **Step 5: Commit** `feat: resume comment publication and resolved synchronizations`. Add a ticket comment and push.

## Task 5: Republish

**Files:**
- Create: `src/repository/mutation/bind/republish.rs`, `tests/mutation_remote/republish.rs`, `schemas/v1/item_republish_preview.schema.json`, mutation goldens `item_republish_preview`, `item_republish_preview_noop`, `item_republish`, `item_republish_external_change`, `item_republish_ticket_closed`
- Modify: `src/repository/remote/{sync,state}.rs` (the push-only path), `src/repository/mutation/progress.rs` (`Session::republish`), `command.rs`, `mod.rs`, `dto.rs`, `schemas/v1/preview_effect.schema.json`, `src/desktop/view/confirm.rs` (one arm), `src/bin/manyhands-cli/human_admin.rs` (the two preview matches), `tests/support/confirmation.rs`, E-5, E-6, E-7, E-8, E-9

**Interfaces:**
- Consumes: facts 12, 13, 36, 55 and 69; `bind/sync.rs`'s `Watch` constructor (Task 2a).
- Produces:
  - `pub struct ItemRepublishInput { pub root: PathBuf, pub kind: AuthoringKind, pub id: ItemId }`, which derives `Clone`;
  - `Mutation::ItemRepublish`, with `Command::ItemRepublish => "item republish"`, and the predicates of the design's table (R-32);
  - `ItemRepublishPreviewDto { root, kind, item_id, branch, remote, last_published_oid: Option<String>, local_oid }`;
  - `PreviewDetailsDto::Republish(ItemRepublishPreviewDto)` and `PreviewEffect::BranchRepublished => "branch_republished"`;
  - crate-private `RepublishConsent { branch, last_published, local }`, `Session::republish(service, request, consent, observer)`, and the domain entry `republish_context` it calls.

- [ ] **Step 1: Write the republish table as cases** in `tests/mutation_remote/republish.rs`, one `c4_republish_<row>` case per row.
  - First publish the item, then have the peer delete its branch on the bare remote, then run `item sync` so the deletion is recorded. The push-absence row instead records the deletion through a push.
  - The primary-moved and ancestor rows have the peer advance remote primary first.
  - The busy row leaves the refused `item sync` request holding its reservation.
  - The unfinished-local-sync row uses Task 1's helper (R-38).
  - Run `prepare`, then `execute` with the confirmation, covering every row.
- [ ] **Step 2: Run it to see it fail.** `devenv shell -- cargo test --all-features --locked --test mutation_remote -- c4_republish`.
- [ ] **Step 3: Implement the push-only path (decision 11, R-18).**
  - A reservation-free read classifies the deletion evidence from the same records as `synchronization_push_absence_boundary`, for the preview. Only `Deleted` makes a preview.
  - `republish_context` fetches and proves the absence. It refuses an advertised context or a moved local tip as `external_change`, before any push.
  - It pushes exactly `local_oid`, leaving out the push-absence boundary (`sync.rs:6209-6213`) and the per-record refusal (`reservation.rs:2742-2759`), and keeping the reservation's other checks (fact 80). It records the publication and integrates nothing. It does not use the clean plan (fact 69).
  - A lifecycle-closed ticket is `ticket_closed` (decision 17).
  - `operation resume` on an operation an `item republish` request holds answers `original_request_required` (fact 78). Add that route to `bind/resume.rs`, with its case.
  - Ordinary `item sync` is unchanged.
- [ ] **Step 4: Implement the binding and the preview (R-19).**
  - Use `operation abandon`'s confirmed shape (`bind/abandon.rs`) for `preview`, `row` and `observe_again`. The preview reads the stored evidence (fact 55), and its no-op says to run `item sync` first.
  - An `external_change` finishes the record (`[request.prepare]`); a transport stop leaves it accepted.
  - Add the desktop arm with three fields, "Branch", "Last published" and "Local commit", and the two CLI arms.
  - Add the command to every closed inventory, `preview_schema` included (E-5, E-6, E-7, E-8, E-9).
- [ ] **Step 5: Verify.**

  ```sh
  devenv shell -- cargo test --all-features --locked --test mutation_remote
  devenv shell -- cargo test --all-features --locked --test mutation_confirmation
  devenv shell -- cargo test --all-features --locked --test mutation_bridges
  devenv shell -- cargo test --all-features --locked --test mutation_contract
  devenv shell -- cargo test --all-features --locked --test read_contract
  devenv shell -- cargo test --all-features --locked --lib mutation
  devenv shell -- cargo test --all-features --locked --lib remote
  devenv shell -- cargo test --all-features --locked --lib local_sync
  devenv shell -- cargo test --all-features --locked --lib integration
  devenv shell -- cargo check --locked --features desktop
  devenv shell -- cargo clippy --all-targets --locked --features desktop -- -D warnings
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  ```

- [ ] **Step 6: Commit** `feat: republish a remotely deleted item branch with consent`. Add a ticket comment and push.

## Task 6: Part A Checkpoint

- [ ] **Step 1:** Merge `origin/main` if it moved, and run the checkpoint gate.
- [ ] **Step 2: Prove the library scope.**

  ```sh
  git fetch origin
  git diff --quiet origin/main...HEAD -- src/results.rs src/lib.rs src/runtime.rs src/canonical.rs src/drafts
  git diff --name-only origin/main...HEAD -- src | grep -v -E '^src/repository/(mutation/|comment_publication\.rs$|recovery\.rs$|remote/(mod|sync|refs|reservation|state|merge|conflict_read)\.rs$|read/(mod|dto)\.rs$)|^src/repository\.rs$|^src/desktop/view/confirm\.rs$|^src/bin/manyhands-cli/human_admin\.rs$'
  ```

  The three-dot diff compares from the merge base, so it shows C4's own changes and not main's. The first must exit 0, and the second must print nothing. Then read `src/repository.rs`'s diff: it must touch only `checkpoint_comment_locally_with`, `checkpoint_comment_locally`'s delegation and the re-exports. To prove the filter can fail, pipe `printf 'src/canonical.rs\nsrc/desktop/model/item.rs\nsrc/repository/integration/validate.rs\nsrc/repository/local_sync/run.rs\n'` through the same `grep -v -E` and see all four lines printed.
- [ ] **Step 3:** An independent review of Part A's range: spec, then quality. Use the adversarial brief on decision 13's mechanism and on R-18. Fix or rule every finding, and rerun the gate after the fixes.
- [ ] **Step 4:** Record counts and reviews in the ledger. Add a ticket comment and push.

## Task 7: The Ctrl-C Window And The Sync, Resume And Abandon Verbs

**Files:**
- Create: `src/bin/manyhands-cli/interrupt.rs`, `interrupt/unix.rs`, `interrupt/windows.rs`, `interrupt_tests.rs`, `collaboration.rs`, `collaboration_tests.rs`, `human_collaboration.rs`, `human_collaboration_tests.rs`, `tests/cli/collaboration.rs`, `tests/cli_remote/interruption.rs`
- Modify: `src/bin/manyhands-cli.rs`, `catalog.rs`, `mutate.rs` (the `Boundary` impl, and the retry loop's latch check), `dispatch.rs`, `tests/cli.rs`, `tests/cli_remote.rs`, `tests/support/cli.rs`, help goldens (E-1)

**Interfaces:**
- Consumes: `Mutation::{ItemSync, RepoSync, OperationResume, OperationAbandon}` (fact 2); `show_item` (fact 37); `mutate::run` (fact 40).
- Produces:
  - `pub(crate) fn during<T>(work: impl FnOnce(&CancellationToken) -> T) -> T` in `interrupt.rs`;
  - `pub const OPERATION_ID: OptionSpec` in `catalog.rs`.

- [ ] **Step 1: Write the window's unit tests first** in `interrupt_tests.rs`, Unix only. `sigaction` and the latch are process-wide, so every case that raises a signal or sets the latch re-runs the test binary as a child process, filtered to that one case, and asserts on the child's exit. No in-process test sees a set latch:
  - after `during` returns, `sigaction` reports the disposition it had before;
  - a `raise(SIGINT)` inside `during` cancels the token, and the process continues;
  - the run's latch (R-36) is set after that press, and stays set in a second window of the same run;
  - a second `raise(SIGINT)` in one window restores the default disposition. That case runs in a child process, which dies by SIGINT.
- [ ] **Step 2: Implement `interrupt.rs`** (R-21, R-36).
  - Unix: `libc::sigaction` with `SA_RESTART`, and the token in an `AtomicPtr` slot. The first signal calls `cancel()` and sets the run's latch; the second restores `SIG_DFL` and `raise`s.
  - Windows: `SetConsoleCtrlHandler`. The first `CTRL_C_EVENT` cancels, sets the latch and returns `TRUE`; the second returns `FALSE`.
  - Teardown restores the disposition, clears the slot, then drops the token. The handler checks for null, and a drop guard covers an unwind.
- [ ] **Step 3: Wire the window.**
  - In the `Boundary` impl (`mutate.rs:50-62`): when the call's `Mutation` is `ItemSync`, `RepoSync`, `ItemRepublish`, `CommentAdd`, `ConflictResolve` or `OperationResume`, call `RepositoryService::execute` inside `during` with its token. Otherwise pass a fresh token, as now.
  - In `mutate::run`: reset the run's latch on entry, and stop the retry loop once it is set (R-36). Because every test that raises a signal or sets the latch runs in a child process (Step 1), `mutate_tests` needs no change.
  - `Boundary`'s signature, `Plan` and `FakeBoundary` are unchanged (fact 61). The retry loop still never follows `cancelled` (R-23).
- [ ] **Step 4: Dispatch four verbs** in `collaboration.rs`: `item sync`, `repo sync`, `operation resume` and `operation abandon`.
  - Kind lookup by `show_item` (R-2).
  - The sync verbs and `operation resume` run with `Plan { retry: true, .. }`, so C2's bounded retry applies (R-23).
  - `operation abandon` uses C2's confirmed flow and its consent prompt.
  - Flip them to `Available("C4")`, add `OPERATION_ID`, and regenerate the help goldens (E-1).
- [ ] **Step 5: The spike (decision 14).** Write `c4_interruption_held_transfer` in `tests/cli_remote/interruption.rs`, `#[cfg(unix)]`. Register `interruption::CASES` in `tests/cli_remote.rs`.
  1. Build `World` with `CliEnv`, as `tests/cli_remote/unlock.rs`'s `world()` does, and approve its host.
  2. Hold the transfer with `hold_transfer_after`, and run `item sync` through `PtyCli` with `--non-interactive --request-id`.
  3. `send(&[3])` after the progress line, then release the hold.
  4. Record the exit, the code and the effects in the ledger.

  `cancelled` (130) or `transport_unavailable` (5) with effects intact is accepted. Assert whichever it is, and record it as Known Limits says. Anything that misreports effects stops for the owner.
- [ ] **Step 6: Human output** in `human_collaboration.rs`:
  - the conflict and next lines, with the root quoted and escaped;
  - the `remote_branch_deleted` next line;
  - `original_request_required`'s next line naming the request. For a remote resolution with only written paths, it adds the supersede note (Resume);
  - the gate's `LocalContextSyncMutationDto` from `operation resume` and `operation abandon`: its `state`, `reason`, `next_action`, and a `next:` line naming `required_request_id` (R-37). Test it in `human_collaboration_tests.rs` by rendering the gate's goldens `tests/fixtures/mutation_v1/local_sync_*.json`;
  - progress.
- [ ] **Step 7: Goldens and tests** in `tests/cli/collaboration.rs`, without SSH:
  - `repo_sync_publication_remote_required`, `item_sync_item_not_found`, `operation_resume_operation_not_found` and `operation_abandon_preview`;
  - text goldens for each verb;
  - abandon's prepare, confirm and interactive cases;
  - C3's stuck save closed by abandon, with its file unchanged (R-30);
  - abandoning a remote operation, which is refused;
  - abandon's stale write: a confirmation presented after the operation's row moved is `external_change`;
  - `request_mismatch` for `operation resume` and `operation abandon`: the same request ID sent again with another operation ID. The sync verbs' need a recorded request, which needs a remote, so they are Task 12a's;
  - a replay of a finished `operation resume` and `operation abandon` returns its stored result;
  - progress: a human-mode `operation resume` of a local hand-off prints its `progress:` lines on stderr, and JSON stderr is empty;
  - **Review Focus 4:** a Ctrl-C (0x03 through `PtyCli`) at `operation abandon`'s consent prompt is exit 130, with the operation still pending and no accepted request record.
- [ ] **Step 8: Verify.**

  ```sh
  devenv shell -- cargo test --all-features --locked --bin manyhands-cli
  devenv shell -- cargo test --all-features --locked --test cli
  devenv shell -- cargo test --all-features --locked --test cli_remote -- c4_interruption
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo check --locked --bin manyhands-cli
  ```

- [ ] **Step 9: Commit** `feat: add sync, resume and abandon verbs with a Ctrl-C window`. Add a ticket comment and push.

## Task 8: `comment add`

**Files:**
- Create: `src/bin/manyhands-cli/discussion.rs`, `discussion_tests.rs`, `tests/cli/discussion.rs`, `schemas/input/v1/comment_add.schema.json`, `tests/fixtures/cli_input_v1/comment_add.json`
- Modify: `catalog.rs`, `dispatch.rs`, `human_collaboration.rs`, `tests/cli.rs`, `tests/support/cli.rs`, help goldens (E-1)

**Interfaces:**
- Consumes: `Mutation::CommentAdd` and `CommentMutationDto` (Task 2b); the window (Task 7).
- Produces: `discussion::comment_add(args, session, service) -> Output`.

- [ ] **Step 1: Write tests first** in `tests/cli/discussion.rs`, all without a remote:
  - the input schema's fields;
  - `comment_add_ok` with publication pending, exit 0;
  - `comment_add_ticket_closed`;
  - `comment_add_invalid_input`, for a malformed `created_by` and for a malformed `comment_id`;
  - a reply's parent and depth through `comment list`;
  - human text with the publication note;
  - a lost-output replay that writes no second file;
  - the example input `comment_add.json` run through the binary, as C3's `every_c3_example_input_runs` does.
- [ ] **Step 2: Implement.**
  - `only` with the four keys.
  - `comment_id` and `parent_id` parsed as ULIDs; a malformed one is `invalid_input`.
  - Kind by `show_item` (R-2).
  - `Plan { retry: true, .. }` as C3's saves.
- [ ] **Step 3:** Flip `comment add` to available, and regenerate the help goldens (E-1).
- [ ] **Step 4: Verify.** As Task 7, Step 8, without the `cli_remote` line, plus `devenv shell -- cargo test --all-features --locked --test cli -- input_schemas`.
- [ ] **Step 5: Commit** `feat: add comment add`. Add a ticket comment and push.

## Task 9: `conflict show` And `conflict resolve`

**Files:**
- Create: `src/bin/manyhands-cli/conflict.rs`, `conflict_tests.rs`, `tests/cli/conflicts.rs`, `schemas/input/v1/conflict_resolve.schema.json`, `tests/fixtures/cli_input_v1/conflict_resolve.json`
- Modify: `input.rs`, `input_tests.rs`, `catalog.rs`, `dispatch.rs`, `human_collaboration.rs`, `tests/cli.rs`, `tests/support/cli.rs`, help goldens (E-1)

**Interfaces:**
- Consumes: `show_conflict` (Task 3); `Mutation::ConflictResolve` (Task 4a).
- Produces: `input::InputObject::object_list(&self, key) -> Result<Vec<InputObject>, InputError>`; `conflict::show` and `conflict::resolve`.

- [ ] **Step 1: Tests first.**
  - `object_list` unit tests: a non-array, an array with a non-object, and an empty array, each refused.
  - In `tests/cli/conflicts.rs`, without SSH:
    - `conflict_show_operation_not_found`, exit 2;
    - `conflict show` on a local operation that is no synchronization, `invalid_input`;
    - `conflict show` and `conflict resolve` on a local-context sync's operation ID, `invalid_input` (decision 19). The sync is made with the gate's library mutation in the test's setup, because no verb starts one;
    - `conflict resolve` with a malformed token, `invalid_input`, exit 2;
    - `conflict show --prepare`, a usage error;
    - the `conflict_resolve.json` example, validated by schema.

  A live conflict needs a remote, so the show-and-resolve journey, its text golden and a stale observation are Task 12b's.
- [ ] **Step 2: Implement** `object_list`, both verbs and their human output. `conflict show` prints sides only in JSON (design, Human Output).
- [ ] **Step 3:** Flip both verbs and regenerate the help goldens (E-1).
- [ ] **Step 4: Review Focus 2's escaping** in `human_collaboration_tests.rs`: a `ConflictDto` whose display path holds an escape sequence and a newline renders as text with no raw control byte, and with the library's escaping not doubled. A `ConflictDto` with an `incompatible_closures` path prints the design's note.
- [ ] **Step 5: Verify.** As Task 8, Step 4.
- [ ] **Step 6: Commit** `feat: add conflict show and conflict resolve`. Add a ticket comment and push.

## Task 10: `item republish`

**Files:**
- Create: `tests/cli/republish.rs`
- Modify: `collaboration.rs`, `catalog.rs`, `catalog_tests.rs` (E-2), `dispatch.rs`, `tests/cli.rs`, `tests/support/cli.rs`, help goldens (E-1)

- [ ] **Step 1: Tests first**, without SSH:
  - `item_republish_publication_remote_required`: `--prepare` on a repository with no publication remote;
  - `--confirm` with an unknown confirmation is `confirmation_not_found`;
  - `--prepare` together with `--request-id` is a usage error, as C2's mode rules give;
  - `not_confirmable` for `--prepare` on each of the five unconfirmed mutations: `comment add`, `item sync`, `repo sync`, `conflict resolve` and `operation resume`.

  A deleted branch needs a recorded publication, which needs the SSH fixture. The preview golden, confirm and interactive consent are Task 12b's.
- [ ] **Step 2: Implement.** Add the catalog entry `Available("C4")` "Republish a deleted item branch.", and use C2's confirmed flow. Apply E-2.
- [ ] **Step 3:** Regenerate the help goldens (E-1), and read the diff: one added row.
- [ ] **Step 4: Verify.** As Task 8, Step 4, plus `devenv shell -- cargo test --all-features --locked --bin manyhands-cli -- catalog`.
- [ ] **Step 5: Commit** `feat: add item republish`. Add a ticket comment and push.

## Task 11: Part B Checkpoint

- [ ] **Step 1:** Merge `origin/main` if it moved, and run the checkpoint gate.
- [ ] **Step 2:** Prove that C5's verbs still answer `command_unavailable`, and help and version still start no transport: the existing tests pass unchanged.
- [ ] **Step 3:** An independent review of Part B's range: spec, then quality. Fix or rule every finding, and rerun the gate after the fixes.
- [ ] **Step 4:** Ledger, ticket comment, push.

## Task 12a: Two Clones: Comments And Synchronization

**Files:**
- Create: `tests/support/cli_pair.rs`, `tests/cli_remote/collaboration.rs`
- Modify: `tests/cli_remote.rs`

**Interfaces:**
- Produces: `pub struct CliPair { pub server: SshRemoteFixture, pub a: CliClone, pub b: CliClone }`, with `CliClone { env: CliEnv, root: PathBuf }` and `CliPair::new() -> Result<CliPair, FixtureError>`.
  - It seeds the item on the bare remote's primary before both clones are made (R-33).
  - Each clone is enabled, with a publication remote, its own selected key allowed by the server, and an approved host, all set up through the binary.

- [ ] **Step 1:** Build `CliPair`, mounted with `#[path]` from `tests/cli_remote.rs` only. Each setup step goes through the binary with `checked`, and every run goes through `scan_output`.
- [ ] **Step 2: Write these rows as `c4_collaboration_` cases,** each under the watchdog (R-24). Clone B makes its own context with a first write before it synchronizes the item.
  - comment publish success, and failure by server refusal;
  - ordered replies from both clones;
  - one saved comment after a retry;
  - already-current, fast-forward and clean divergence for `item sync` and `repo sync`;
  - `operation resume` publishing a comment whose publication stopped;
  - the `_ok` goldens of `item sync`, `repo sync` and `operation resume`;
  - for `comment add`, `item sync`, `repo sync` and `operation resume`: human-mode `progress:` lines on stderr with empty JSON stderr, a replay of the finished request with no second effect, and `request_mismatch` for the same request ID sent again with another target.
- [ ] **Step 3: Verify.** `devenv shell -- cargo test --all-features --locked --test cli_remote -- c4_collaboration`.
- [ ] **Step 4: Commit** `test: prove C4 comment and sync journeys across two clones`. Add a ticket comment and push.

## Task 12b: Two Clones: Conflicts And Republish

**Files:**
- Create: `tests/cli_remote/conflicts.rs`
- Modify: `tests/cli_remote.rs`

- [ ] **Step 1: Write these rows as `c4_conflicts_` cases,** on `CliPair`:
  - a canonical conflict shown (JSON and the text golden) and resolved with the shown tokens;
  - a stale resolution, `external_change`;
  - an outside repair of a code-file conflict, then `operation resume`, and the three refused repairs;
  - the `_ok` golden of `conflict resolve`.
- [ ] **Step 2: Write the republish rows** as `c4_conflicts_republish_` cases. After the peer deletes the branch:
  - `item sync`'s refusal;
  - the `item_republish_preview` golden;
  - confirm, and the `item_republish_ok` golden;
  - a moved local branch, refused at confirm;
  - a transport stop after acceptance, then the same request completing it;
  - progress lines, a replay of the finished request, and `request_mismatch` for the same request ID with another `--id`, for both `item republish` and `conflict resolve`;
  - **Review Focus 4:** a Ctrl-C at the interactive consent prompt, `#[cfg(unix)]`.
- [ ] **Step 3: Verify.** `devenv shell -- cargo test --all-features --locked --test cli_remote -- c4_conflicts`. Then run the whole `cli_remote` target once.
- [ ] **Step 4: Commit** `test: prove C4 conflict and republish journeys across two clones`. Add a ticket comment and push.

## Task 13: Interruption, Lost Output And Exit Evidence

**Files:**
- Create: `tests/cli/collaboration_exits.rs`
- Modify: `tests/cli_remote/interruption.rs`, `tests/cli_remote.rs`, `tests/cli.rs`, `tests/support/pty.rs` (one additive helper)

- [ ] **Step 1: The PTY helper.** Add `PtyCli::finish_any(self) -> (Option<i32>, Option<i32>, String)` (exit code, signal, stderr) beside `finish`. Change nothing else in the file.
- [ ] **Step 2: Ctrl-C over SSH,** as `c4_interruption_` cases, `#[cfg(unix)]`, each recording its answer under decision 14:
  - `item sync`, `comment add`, `conflict resolve`, `item republish` and `operation resume` cancelled at a safe point, each exiting 130 with effects. A new request completes the work as the design's Cancellation section says;
  - a second Ctrl-C ends the process by SIGINT (`finish_any`), and `request show` and a new request recover;
  - a Ctrl-C at the unlock prompt of a protected key is still `unlock_required`, exit 3, with the window installed.
- [ ] **Step 3: `request cancel`** for each verb that stops at safe points (`item sync`, `repo sync`, `item republish`, `comment add`). Each is held, then cancelled from a second `CliEnv` invocation, and stops at its next safe point. `conflict resolve` and `operation resume` hold no operation of their own and are N/A (R-35, fact 11).
- [ ] **Step 4: The latch (R-36).** A Ctrl-C that lands after the last safe point of an `item sync` whose push is then refused stops C2's bounded retry: no second execute runs.
- [ ] **Step 5: Lost stdout,** over SSH in `tests/cli_remote/interruption.rs`. `item sync` and `comment add` run with stdout closed, using the closed-pipe technique of `tests/cli/boundaries.rs:247`, exit 1. Then `request show` returns the stored result, and a replay makes no second push, file or checkpoint (R-25).
- [ ] **Step 6: The exit table (R-27).** Map every code in Task 0's union to its test, or to a reason no fixture reaches it, in `collaboration_exits.rs` and the ledger. Assert set equality with the union.
- [ ] **Step 7: Verify.**

  ```sh
  devenv shell -- cargo test --all-features --locked --test cli_remote -- c4_interruption
  devenv shell -- cargo test --all-features --locked --test cli -- collaboration_exits
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  ```

- [ ] **Step 8: Commit** `test: prove C4 interruption, lost output and exit coverage`. Add a ticket comment and push.

## Task 14: Document Amendments

- [ ] **Step 1:** Make every amendment in the design's Document Amendments, except AGENTS.md and the skills. Use the dated form "Amended <date> by C4", and cite the decision or ruling.
- [ ] **Step 2:** Reconcile the API audit's C4 rows with what was delivered.
- [ ] **Step 3:** Grep the amended RFCs, the Wave and the audit for stale terms: `publication_pending`, `conflict.show`, `{path, observation, source}`, "never merges", "refuse divergence", "`operation show` re-observes", "future CLI adapter contract", "C4/D5 must adapt". The only expected hits are in amendment text that names what it replaces. Check that each of the gate's sentences fact 85 lists is scoped to the local operation (decision 20), and that the gate's capability row keeps its local half. The planning documents name these terms on purpose, in their decisions, rulings and amendment lists, and are not grepped.
- [ ] **Step 4: Commit** `docs: amend RFCs, Wave, F2 design and audit for C4`. Add a ticket comment and push.

## Task 15: Full Verification, Review, Dogfooding, Switch And Handoff

- [ ] **Step 1:** Merge `origin/main`, then run the checkpoint gate and the full suite once, detached, with a two-hour budget. Record per-target counts in the ledger. A failure stops for the owner.
- [ ] **Step 2:** Independent whole-branch reviews, spec and quality, of `<Task 0 HEAD>..HEAD`. Fix or rule every finding. After the fixes, rerun the checkpoint gate and the affected targets, and record them, then re-review the fixes.
- [ ] **Step 3: Dogfooding, only with the owner's go-ahead at this step (R-26).** Follow the design's Dogfooding steps 1–3 exactly:
  - the preflight, with its stops, including that every edit in this worktree is committed, and that main's CLI still opens the shared registry;
  - the request ID recorded before sending;
  - one verification comment through `comment add`, with `created_by` `Claude Opus 5.5 <noreply@anthropic.com>`;
  - the value check with `comment list`.

  A refusal or partial result stops for the owner.
- [ ] **Step 4: The switch.** Edit the files of fact 48 as the design's Dogfooding step 4 says, including AGENTS.md's explicit extended exception (decision 15). Commit `docs: switch comment dogfooding to the CLI`, and give that range its own independent spec and quality review.
- [ ] **Step 5: Review-ready.**
  1. Mark the Cycle and design `review-ready`, and commit.
  2. Run `git merge-tree --write-tree HEAD origin/main`.
  3. Add the review-ready comment through `comment add`, under the same go-ahead, as the Dogfooding section's step 5. It checkpoints into this worktree.
  4. Push the ticket branch.

  The ticket stays **open**.
- [ ] **Step 6: Closure**, only after the owner approves the review, and in the final push before merge. Close the ticket by the canonical ticket-file edit, until C5's `ticket close`. Merging needs its own authorization.

## Plan Self-Review

**Acceptance rows to tasks:**

| Cycle acceptance row | Task |
| --- | --- |
| Comment publish success and failure | 2b (library), 8 (binary), 12a (two clones) |
| Ordered replies | 12a |
| One saved comment after retry | 2a, 2b, 12a |
| Clean and divergent sync | 1, 12a |
| Stale resolutions | 4a, 12b |
| Outside repair of a noncanonical conflict | 3, 4b, 12b |
| No secret or raw-server output | 12a, 12b, 13 (every `cli_remote` case scans) |
| Ctrl-C | 7 (spike), 13 |
| Lost stdout | 13 |
| Dogfooding switch | 15 |
| Republish | 5, 10, 12b |
| Resume and abandon | 4b, 7, 12a |
| Identity | 1, 2a, 2b |
| Outcome tables | 1, 2b, 3, 4a, 4b, 5 |
| Exit classes | 13 |
| Schemas, goldens and examples | 1–5, 7–10, 12a, 12b |
| Mutating-Cycle evidence | the design's matrix: 1, 2b, 4a, 5, 7–10, 12a, 12b, 13 |
| Unchanged behavior | 11 |
| Documents | 14, 15 |

**Rulings to tasks:**

| Task | Rulings |
| --- | --- |
| 1 | R-8, R-16, R-17, R-38 |
| 2a | R-3, R-4, R-7, R-34 |
| 2b | R-5, R-6, R-32, R-34, R-38 |
| 3 | R-9, R-10, R-11, R-12, R-13, R-37, R-39, R-40 |
| 4a | R-14, R-15, R-32, R-33, R-35, R-38, R-40, R-41 |
| 4b | R-20, R-37 |
| 5 | R-18, R-19, R-32, R-38 |
| 7 | R-1, R-2, R-21, R-22, R-23, R-30, R-36, R-37 |
| 8–10 | R-1, R-2; 9 also R-37 and R-40 |
| 12a, 12b | R-24, R-33 |
| 13 | R-25, R-27 |
| Every task | R-28, R-29, R-31 |
| 15 | R-26 |

**Named edits:** E-1 in 7–10; E-2 in 10; E-3 and E-4 in 1; E-5 in 1 and 5; E-6 in 2b, 4a and 5; E-7 and E-8 in 1, 2b, 3, 4a and 5; E-9 in 2b, 4a and 5; E-10 in 1, 2b and 4a; E-11 in 4b; E-12 in 4a.

**Stops built in:**
- Task 0: the gate not closed; a changed fact; an unlisted test hit; abandon closing a pending comment; fact 81's test still red on main.
- Tasks 1, 2a, 2b, 3, 4a, 4b, 5: each outcome-table row.
- Task 3: no observation at `ResolutionPrepared` (fact 86).
- Task 4a: decision 13's mechanism; `repository_enablement` red only for the gate's tables.
- Task 7: the spike misreporting effects.
- Task 15: dogfooding.

**Placeholders:** none intended. The failure points and the E-3 classification are Task 0 records, not choices left to an implementer.
