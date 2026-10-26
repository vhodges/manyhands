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

**Draft, 2026-10-26.** Not approved. Implementation is not authorized, and does not start until the gate ticket `01M57SKVF6TQ3QM6AXJ35JGGSY` is merged to main (Cycle decision 2).

**Goal:** Add C4's library part and its eight CLI verbs, so that an agent or a person can comment, synchronize, resolve or repair conflicts, republish a deleted branch, and resume or abandon operations through `manyhands-cli`. Then switch this project's comment dogfooding to the CLI.

**Architecture:**

- **Library (Part A).** Three new mutation commands, `CommentAdd`, `ConflictResolve` and `ItemRepublish`, are bound in `src/repository/mutation/bind/`. A conflict read lives in `src/repository/remote/conflict_read.rs`. The two synchronization bindings merge, and resume gains routes. All of it goes through F2's boundary: request records, replay, confirmation, progress and cancellation.
- **CLI (Part B).** C1's catalog, parse, startup, dispatch and render, and C2's `mutate::run`, with new bin-private modules beside them. A scoped Ctrl-C window passes a real cancellation token to `execute`.
- **Evidence (Part C).** Two-clone SSH journeys through the binary, plus interruption evidence.

**Tech Stack:**

- Rust 2024, clap's builder API, `serde_json` and `git2`;
- `libc` and `windows-sys`, both existing dependencies, for the Ctrl-C window;
- F2's mutation boundary, result model, schemas and test support;
- C1–C3's CLI harness, goldens, pseudo-terminal helper and SSH harness.

**Spec:** the [Design](2026-10-26-wave-03-cli-04-discussion-and-sync-design.md), under the [Cycle](../Cycles/wave-03-cli-04-discussion-and-sync.md). Read both before any task. The Cycle governs the design, and the design governs this plan. "Fact N" means the design's Baseline Fact N, "R-N" its ruling and "E-N" its named edit. The five tables are the design's required-outcome tables, by name.

## Global Constraints

- **Devenv.** Run every Rust command through Devenv with `--locked`: `devenv shell -- cargo <command> --locked`.
  - No new crate.
  - No change to `Cargo.toml` or `Cargo.lock`.
  - No new `[[test]]` target (R-29). New tests are modules of existing targets.
- **Branch.** Work only in the ticket worktree `.manyhands/worktrees/01M4CC0VMWEQQ9G4W0DAG7ADPY`, on `manyhands/ticket/01M4CC0VMWEQQ9G4W0DAG7ADPY`.
  - If `origin/main` moves, `git merge origin/main`. Never rebase.
  - Before each checkpoint, run `git fetch origin` and then `git merge-tree --write-tree HEAD origin/main`.
- **The gate first (decision 2).** Task 0 does not start until `git log --oneline origin/main --grep 01M57SKVF6TQ3QM6AXJ35JGGSY` shows the gate's merge.
- **Library scope (decision 1).** Library changes are Part A's only, and only in these files:
  - under `src/repository/mutation/`;
  - `src/repository/comment_publication.rs` and `src/repository/recovery.rs`;
  - `src/repository/remote/{sync,refs,reservation,state,merge}.rs` and the new `remote/conflict_read.rs`;
  - `src/repository/read/mod.rs` and `read/dto.rs`, for exports only;
  - `src/repository.rs`, for re-exports only;
  - `schemas/v1/`.

  Also:
  - No new `ResultCode` or `RecoveryActionKind` (decision 8).
  - No field is added to a public struct that tests build by literal (R-4).
  - A gap beyond these files stops the task for the owner.
- **Desktop.** Only Task 5's one arm in `src/desktop/view/confirm.rs` (decision 10). That task also runs `cargo check --locked --features desktop` and `cargo clippy --all-targets --locked --features desktop -- -D warnings`, with the desktop feature alone.
- **Bin.** CLI code lives in `src/bin/manyhands-cli.rs` and `src/bin/manyhands-cli/`. New modules follow C1's pattern: `#[path = "manyhands-cli/<name>.rs"] mod <name>;`, with `#[cfg(test)] #[path = "<name>_tests.rs"] mod tests;`. A module that `tests/cli_remote.rs` mounts must still resolve there.
- **No dead code.** `cargo clippy -- -D warnings` fails on an unused function, so add each helper in the task that first calls it.
- **Runtime.**
  - The CLI spawns no thread and no child process. On Windows, the console-control routine runs on a thread the system creates (R-21).
  - The transport starts before the service opens. Help, version, usage errors and unavailable commands touch neither.
  - All errors before dispatch have a null `request_id`. JSON mode writes one envelope to stdout and nothing to stderr.
- **Content and secrets.**
  - No type that holds a body, source or token derives or implements `Debug`, `Display` or `Serialize`, except the conflict read's DTO, which is a requested read.
  - No input value, body, source, token, backend text or SQL reaches a diagnostic.
  - JSON `message` is always `ResultCode::message()`.
- **Test isolation.**
  - Every CLI child goes through `tests/support/cli.rs`, `tests/support/pty.rs` or the SSH harness.
  - Fixtures write `core.autocrlf = false` and no identity into the test home.
  - The identity cases call `require_no_system_identity()` (`tests/cli/repository.rs`). An SSH target, which cannot reach it, uses a copy of it in its new module.
  - The SSH targets compile `src/lib.rs` as a module (`tests/comment_publication.rs:4`), so their tests may call the crate-private functions this plan adds.
  - Tests never touch developer keys, global Git configuration, the real terminal or real application data.
- **Pre-existing tests.** Only named edits E-1 to E-11, in the tasks the design names. Any other semantic edit, including a new lint on an old test, stops for owner review.
- **Outcome tables.** Each table's test is written first, in its task's first step, and run to see it fail before any implementation. The test and the code that makes it pass are committed together, so no commit is left red or uncompiled. If the code answers a row differently, stop for a ruling. Never edit a table to fit.
- **Test runs (owner, 2026-10-17, 2026-10-19 and 2026-10-24).**
  - Each task runs only the targets and filters its steps name. Check that more than zero tests ran.
  - Cargo takes **one** positional test filter per command.
  - SSH targets (`harness = false`) take a substring filter after `--`.
  - Clippy in a task step is `cargo clippy --all-targets --all-features --locked -- -D warnings`.
  - The Part checkpoints (Tasks 6 and 11) run the checkpoint gate. The full suite runs once, in Task 15, detached, with a two-hour budget. The full suite took 1 h 44 min in C3; use a 600-second tool timeout for gate commands.
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
  devenv shell -- cargo test --all-features --locked --bin manyhands-cli --test cli --test read_contract --test mutation_contract --test mutation_replay --test mutation_confirmation
  devenv shell -- cargo test --all-features --locked --test mutation_remote --test comment_publication --test remote_merge_recovery --test cli_remote
  ```

  After the gate:
  1. `git fetch origin` and `git merge-tree --write-tree HEAD origin/main`;
  2. an independent review of the Part's commit range against the Cycle, the design and this plan;
  3. fix or rule every finding before the next Part;
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
  - Never push another branch, open a pull request, merge, close the ticket, create a ticket, run `ticket save`, or clean up without separate owner authorization. Task 15's dogfooding needs its own go-ahead at that step.
  - Native CI runs after merge, on main, only with the owner's approval of that run.

## Review Focus

These are inputs the spec implies but no table lists. Each has its test in the owning task.

1. **A comment body with CRLF line endings, non-BMP characters, a line `---`, and no final newline.** `comment add` writes it byte for byte into the body, and `comment list` returns it unchanged. Task 2, Step 6.
2. **A conflict whose path holds a terminal escape sequence or a newline.** `conflict show`'s human output and every diagnostic escape it. Its token still resolves, and `conflict resolve` writes only that path. Task 3, Step 5 and Task 9, Step 4.
3. **`conflict resolve` with the same token twice, or with a token from another operation's conflict.** Both are `invalid_input` before any write, and no source text appears in the message. Task 4, Step 5.
4. **A Ctrl-C at a confirmed verb's consent prompt, before execution.** It is the prompt's own cancellation, exit 130. The handler is not installed, and nothing is reserved. `operation abandon` in Task 7, Step 6; `item republish` over SSH in Task 12, Step 2.
5. **`comment add` whose `parent_id` is the comment's own `comment_id`.** It is refused before any write, and no binding row stays. Task 2, Step 6.

---

## File Map And Task Order

| Task | Part | Creates | Modifies |
| --- | --- | --- | --- |
| 0 | — | `docs/plans/2026-10-26-wave-03-cli-04-discussion-and-sync-execution.md` | `docs/research/wave-03-api-audit.md` |
| 1 | A | `tests/mutation_remote/merging.rs`, `schemas/v1/synchronization_conflict_mutation.schema.json`, `conflict_next.schema.json`, golden `repo_sync_conflict_pending.json` | `bind/sync.rs`, `outcome.rs`, `dto.rs`, `mod.rs`, `tests/mutation_remote.rs` (E-3, module), E-4, E-5, E-7, E-8, E-10 |
| 2 | A | `bind/comment.rs`, `tests/comment_publication/binding.rs`, `schemas/v1/comment_mutation.schema.json`, goldens `comment_add_*` | `comment_publication.rs`, `recovery.rs`, `command.rs`, `mod.rs`, `dto.rs`, `outcome.rs`, `bind/mod.rs`, `bind/resume.rs`, `tests/comment_publication.rs` (module), E-5, E-6, E-7, E-8, E-9, E-10, E-11 |
| 3 | A | `remote/conflict_read.rs`, `tests/remote_merge_recovery/conflict_read.rs`, `schemas/v1/conflict.schema.json`, `conflict_path.schema.json`, read goldens `conflict_*` | `remote/mod.rs`, `read/mod.rs`, `read/dto.rs`, `src/repository.rs`, `tests/remote_merge_recovery.rs` (module), `tests/support/golden.rs`, E-8 |
| 4 | A | `bind/conflict.rs`, `tests/remote_merge_recovery/resolution.rs`, `schemas/v1/conflict_resolution_mutation.schema.json` | `remote/sync.rs`, `remote/state.rs`, `remote/merge.rs`, `command.rs`, `mod.rs`, `dto.rs`, `outcome.rs`, `bind/mod.rs`, `bind/resume.rs`, E-6, E-7, E-8, E-9, E-10, E-11 |
| 5 | A | `bind/republish.rs`, `tests/mutation_remote/republish.rs`, `schemas/v1/republish_preview.schema.json`, goldens `item_republish_*` | `remote/{sync,refs,reservation,state}.rs`, `command.rs`, `mod.rs`, `dto.rs`, `confirm.rs` (if needed), `src/desktop/view/confirm.rs` (one arm), `src/bin/manyhands-cli/human_admin.rs` (preview arms), `schemas/v1/preview_effect.schema.json`, `preview.schema.json`, E-6, E-7, E-8, E-9 |
| 6 | A | — | ledger, ticket comment |
| 7 | B | `interrupt.rs`, `interrupt/unix.rs`, `interrupt/windows.rs`, `collaboration.rs`, `human_collaboration.rs` and their tests, `tests/cli/collaboration.rs` | `src/bin/manyhands-cli.rs`, `catalog.rs`, `mutate.rs`, `dispatch.rs`, `tests/cli.rs`, `tests/support/cli.rs`, E-1 |
| 8 | B | `discussion.rs`, `discussion_tests.rs`, `tests/cli/discussion.rs`, `schemas/input/v1/comment_add.schema.json`, `tests/fixtures/cli_input_v1/comment_add.json` | `catalog.rs`, `dispatch.rs`, `human_collaboration.rs`, `tests/cli.rs`, `tests/support/cli.rs`, E-1 |
| 9 | B | `conflict.rs`, `conflict_tests.rs`, `tests/cli/conflicts.rs`, `schemas/input/v1/conflict_resolve.schema.json`, `tests/fixtures/cli_input_v1/conflict_resolve.json` | `input.rs`, `input_tests.rs`, `catalog.rs`, `dispatch.rs`, `human_collaboration.rs`, `tests/cli.rs`, `tests/support/cli.rs`, E-1 |
| 10 | B | `tests/cli/republish.rs` | `collaboration.rs`, `catalog.rs`, `catalog_tests.rs` (E-2), `dispatch.rs`, `tests/cli.rs`, `tests/support/cli.rs`, E-1 |
| 11 | B | — | ledger, ticket comment |
| 12 | C | `tests/support/cli_pair.rs`, `tests/cli_remote/collaboration.rs` | `tests/cli_remote.rs`, `tests/support/mod.rs` (if it lists modules) |
| 13 | C | `tests/cli_remote/interruption.rs`, `tests/cli/collaboration_exits.rs` | `tests/cli_remote.rs`, `tests/cli.rs` |
| 14 | — | — | `docs/RFC/cli-contract.md`, `docs/RFC/git-workflow-and-conflict-recovery.md`, `docs/RFC/canonical-content-and-comment-schema.md`, `docs/Waves/wave-03-dogfooding.md`, `docs/research/wave-03-api-audit.md` |
| 15 | — | — | the Cycle, the design, the ledger, the ticket and comments, `AGENTS.md`, the skills of fact 48 |

Tasks run in order. Part A's tasks extend the same closed inventories (fact 49), so none can run in parallel. Part B's tasks each flip verbs in `catalog.rs`. A verb becomes available only in the task that dispatches it, gives it its schema and example, and adds its goldens.

---

## Task 0: Baseline, Re-Grounding After The Gate, Audit Refresh And Ledger

**Files:**
- Create: `docs/plans/2026-10-26-wave-03-cli-04-discussion-and-sync-execution.md`
- Modify: `docs/research/wave-03-api-audit.md`

- [ ] **Step 1: Confirm the base.**
  - `git fetch origin`. Confirm the gate's merge with `git log --oneline origin/main --grep 01M57SKVF6TQ3QM6AXJ35JGGSY`. If it is absent, stop: implementation is not authorized yet (decision 2).
  - `git merge origin/main`, then `git merge-tree --write-tree HEAD origin/main`.
  - Read `git show origin/main:AGENTS.md`.
  - For each known-limit ticket in the Cycle's Entry Evidence, run `git log --oneline origin/main --grep <ULID>`. A merged one changes a known limit, and the controller rules before Task 1.
  - Check whether D3 (`01M4CC0VN0MFTGKQ4HGTTC5H63`) has merged, and record it. It decides who resolves the `confirm.rs` merge (decision 10).
  - Record HEAD and `origin/main`.
- [ ] **Step 2: Re-check every Baseline Fact** (1–53) against the merged source, and record each as confirmed or changed, with its new line numbers. In particular:
  - the gate's delivered changes to every file Part A touches (fact 53): new commands, `local_control` hooks, resume routing, inventory sizes;
  - fact 28: whether `repo identity-set` succeeds while a merge's `IdentityRequired` holds the reservation. Read `set_local_identity` and record the rule Task 1 must meet;
  - fact 41: whether a token cancelled during `operation resume` stops it at a safe point;
  - the code `submit_comment` returns for a `parent_id` that names no comment of the item, and for a self-parent. Task 2's table needs it.

  A changed fact that alters a task stops here for a controller ruling.
- [ ] **Step 3: Prove E-1 to E-11 are the only named edits.**

  ```sh
  grep -rn -E 'refuse_divergence|REFUSING|merge_required|MergeRequired|is_comment_synchronization|LeftToInspect' tests src/repository/mutation
  grep -rln -E 'comment add|item sync|repo sync|conflict (show|resolve)|operation (resume|abandon)|"C4"' src/bin/manyhands-cli tests/cli tests/cli_remote tests/cli.rs tests/support/cli.rs tests/fixtures/cli_v1
  grep -rn -E 'Command::ALL|bound_commands\(\)|HOMES|DATA_ROOTS|UNBOUND_COMMANDS|CODES_NO_MUTATION_GOLDEN_HAS|contract_enumerations|PreviewDetailsDto::|MutationDataDto::' tests src --include=*.rs
  ```

  Classify every hit as one of:
  - the production code a task changes;
  - an E-1 to E-11 site;
  - a loop over an inventory that takes the new entries unchanged.

  For E-3, record each listed test as binding-level (edited) or domain-level (unchanged), from whether it reaches the binding's `SETTINGS`. Any other hit stops for an owner ruling.
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

  Record per-target counts. A failure outside C4's files stops for the owner. It is not fixed here.
- [ ] **Step 5: Refresh the API audit.**
  - Add a section, "Wave 03 CLI Cycle C4 refresh (2026-10-26)".
  - Update the C4 rows in place: `comment add`, `item sync`, `repo sync`, `conflict show`, `conflict resolve`, `operation resume` and `operation abandon`. Add `item republish`. For each, record the planned binding and signature, whether it is confirmed, its input type, and every result code it can return as designed. Read the existing codes from `outcome.rs`, `execute.rs`, `confirm.rs` and the binding.
  - Correct the stale "missing" and "pending Wave 02" claims. Do not add a second row for any verb.
  - Under `#### C4 exit-coverage union`, give the union of those codes in one fenced `text` block, space-separated.
- [ ] **Step 6: Create the execution ledger.** Give it front matter like the design's and a new document ULID from `id new`. Its sections are:
  - baseline;
  - confirmed facts;
  - audit refresh;
  - Cycle decisions 1–10, rulings R-1 to R-30 and named edits E-1 to E-11;
  - the planning review record;
  - an empty task log.
- [ ] **Step 7: Commit** `docs: record C4 baseline after the gate and refresh the API audit`. Add a ticket comment and push the ticket branch.

## Task 1: Merging Synchronization

**Files:**
- Create: `tests/mutation_remote/merging.rs`, `schemas/v1/synchronization_conflict_mutation.schema.json`, `schemas/v1/conflict_next.schema.json`, `tests/fixtures/mutation_v1/repo_sync_conflict_pending.json`
- Modify: `src/repository/mutation/bind/sync.rs`, `outcome.rs`, `dto.rs`, `mod.rs`, `tests/mutation_remote.rs` (module line, E-3), `tests/support/mutation_golden.rs` (E-4), `outcome_tests.rs` (E-5), `tests/mutation_contract.rs` (E-7), `tests/read_contract.rs` (E-8), `tests/support/mutation.rs` (E-10)

**Interfaces:**
- Consumes: `RepositoryService::synchronize_remote_observed` (fact 24); `inspect_synchronization_recovery` (fact 29); `World` (`tests/support/remote_world.rs:139`) and its peer helpers.
- Produces:
  - `pub struct ConflictNextDto { pub operation_id: String, pub next_action: ConflictNextAction }` and `contract_enum! ConflictNextAction { Resolve => "resolve", ExternalRepair => "external_repair" }`, in `mutation/dto.rs`;
  - `pub struct SynchronizationConflictMutationDto { pub r#type: SynchronizationConflictType, pub conflict: ConflictNextDto }`, where the single-value enum is `"synchronization_conflict"`;
  - `MutationDataDto::SynchronizationConflict(SynchronizationConflictMutationDto)`;
  - `pub(crate) fn conflict_next(service: &RepositoryService, root: &Path, operation_id: OperationId) -> ConflictNextDto` in `bind/sync.rs`, which later tasks call.

- [ ] **Step 1: Write the divergent-synchronization table as a test** in `tests/mutation_remote/merging.rs`, one case per row, for both `item sync` and `repo sync`, through `RepositoryService::execute`.
  - Use `World` and its raw peer to make the remote diverge: `commit` plus `push_peer`.
  - Each case asserts code, outcome, effects, data and recovery exactly as the row says. For merge rows it also asserts parent order and subject, and that the remote keeps its earlier tip as an ancestor.
  - The identity rows call `require_no_system_identity()`, run `repo identity-set` through the boundary, then re-execute the same `MutationCall`.

  Register the module in `tests/mutation_remote.rs`.
- [ ] **Step 2: Run it to see it fail.** `devenv shell -- cargo test --all-features --locked --test mutation_remote -- merging`. Expected: the merge and conflict rows fail with `merge_required`.
- [ ] **Step 3: Implement.**
  - Set `refuse_divergence: false` in `SETTINGS`, and update the module doc and the comments of fact 6 (R-16).
  - On `ConflictPending`, return `recovery_required` with `integration: pending` and the data from `conflict_next` (R-8). `next_action` is `resolve` when the inspection's eligibility is `EligibleCanonical`, else `external_repair`.
  - Meet the identity rows (R-17). If `set_local_identity` refuses while the reservation is held, either release the reservation on `IdentityRequired`, or check identity before reserving when history diverges. Record the choice in the ledger.
  - If no mechanism meets the rows, **stop** at a design checkpoint.
  - Add the two schemas, and register the DTO in `DATA_ROOTS` and `contract_enumerations`.
- [ ] **Step 4: Apply E-3, E-4, E-5 (the conflict rows and the `merge_required` tests), E-7, E-8 and E-10,** exactly as the design's table says, and only the tests Task 0 classified binding-level.
- [ ] **Step 5: Verify.**

  ```sh
  devenv shell -- cargo test --all-features --locked --test mutation_remote
  devenv shell -- cargo test --all-features --locked --lib mutation::outcome
  devenv shell -- cargo test --all-features --locked --test mutation_contract
  devenv shell -- cargo test --all-features --locked --test read_contract
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  ```

- [ ] **Step 6: Commit** `feat: merge divergent history in item and repo sync`. Add a ticket comment and push.

## Task 2: Comment Submission Binding

**Files:**
- Create: `src/repository/mutation/bind/comment.rs`, `tests/comment_publication/binding.rs`, `schemas/v1/comment_mutation.schema.json`, goldens `comment_add_ok`, `comment_add_ticket_closed`, `comment_add_publication_pending`
- Modify: `src/repository/comment_publication.rs`, `src/repository/recovery.rs` (if the binding lookup needs a reader), `command.rs`, `mod.rs`, `dto.rs`, `outcome.rs`, `bind/mod.rs`, `bind/resume.rs`, `tests/comment_publication.rs` (module line), E-5, E-6, E-7, E-8, E-9, E-10, E-11

**Interfaces:**
- Consumes: fact 17's types; `ConflictNextDto` and `conflict_next` (Task 1); the binding table (fact 21).
- Produces:
  - `pub struct CommentAddInput { pub root: PathBuf, pub kind: AuthoringKind, pub id: ItemId, pub comment_id: ItemId, pub parent_id: Option<ItemId>, pub body: String, pub created_by: Option<String> }`;
  - `Mutation::CommentAdd(CommentAddInput)`, with `Command::CommentAdd => "comment add"`;
  - `pub struct CommentMutationDto { r#type, item_id, comment_id, parent_id: Option<String>, created_by: Option<String>, synchronization_id: String, publication: PublicationEffect, pending_reason: Option<CommentPendingReason>, conflict: Option<ConflictNextDto> }`;
  - crate-private `submit_comment_observed(&self, request: PublishCommentRequest, author: Option<CommentAuthor>, session, settings: SynchronizationSettings, observer: &mut dyn SynchronizationObserver)` and `retry_comment_publication_observed(...)` (R-3, R-4);
  - `pub(crate) fn comment_synchronization(connection, root, operation_id) -> Option<(OperationId /*comment*/, OperationId /*synchronization*/)>`, which resolves either ID (R-5).

- [ ] **Step 1: Write the comment-submission table as a test** in `tests/comment_publication/binding.rs`, one case per row, through `execute`.
  - Copy the private `Pair` setup from `tests/comment_publication.rs:357` into the new module. Do not edit the original.
  - The server-refusal row uses the fixture's refusal hook. The other-item-conflict row first leaves a conflict on another item.

  Register the module.
- [ ] **Step 2: Run it to see it fail.** `devenv shell -- cargo test --all-features --locked --test comment_publication -- binding`. Expected: it does not compile, because `Mutation::CommentAdd` does not exist yet. That is the failing state; nothing is committed until Step 8.
- [ ] **Step 3: Implement the observed, authored submission** in `comment_publication.rs` (R-3, R-4, R-7).
  - `CommentAuthor::parse(&str) -> Result<CommentAuthor, ...>` enforces R-7.
  - The checkpoint writes the author's text in place of fact 20's format when given.
  - Publication passes the observer and the settings to `synchronize_remote_observed`.
  - The public `submit_comment` calls the observed form with `Unobserved` and the default settings, so its behavior and tests are unchanged.
- [ ] **Step 4: Implement the binding** in `bind/comment.rs`.
  - It follows `bind/sync.rs`'s shape: `prepare`, `run`, `rerun`, `reenter`, the `Watch` observer and effects from the journal.
  - Map outcomes to the table (R-6).
  - Add the command to every closed inventory of fact 49 (E-6, E-7, E-8, E-9, E-10).
- [ ] **Step 5: Route resume (R-20).** In `bind/resume.rs`, a comment's synchronization or the comment's own operation goes to `retry_comment_publication_observed` with `restart: true`, in place of `LeftToInspect`. Apply E-11 to the tests Task 0 found.
- [ ] **Step 6: Review Focus 1 and 5.**
  - A body with CRLF, a non-BMP character, a `---` line and no final newline round-trips through `comment list`, byte for byte.
  - A self-parent is refused with Task 0's recorded code, and no binding row stays.
- [ ] **Step 7: Verify.**

  ```sh
  devenv shell -- cargo test --all-features --locked --test comment_publication
  devenv shell -- cargo test --all-features --locked --test mutation_replay
  devenv shell -- cargo test --all-features --locked --test mutation_contract
  devenv shell -- cargo test --all-features --locked --test read_contract
  devenv shell -- cargo test --all-features --locked --lib mutation
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  ```

- [ ] **Step 8: Commit** `feat: bind comment submission with authorship and safe points`. Add a ticket comment and push.

## Task 3: Conflict Inspection Read

**Files:**
- Create: `src/repository/remote/conflict_read.rs`, `tests/remote_merge_recovery/conflict_read.rs`, `schemas/v1/conflict.schema.json`, `schemas/v1/conflict_path.schema.json`, read goldens `conflict_show_pending`, `conflict_show_external`, `conflict_show_repaired`
- Modify: `src/repository/remote/mod.rs`, `read/mod.rs`, `read/dto.rs` (re-export only), `src/repository.rs`, `tests/remote_merge_recovery.rs` (module line), `tests/support/golden.rs`, E-8

**Interfaces:**
- Consumes: facts 29–32 and 35; `comment_synchronization` (Task 2).
- Produces:
  - `pub fn show_conflict(&self, repo: &ResolvedRepository, operation_id: OperationId) -> Result<ConflictDto, ReadError>`;
  - `ConflictDto` and `ConflictPathDto`, as the design's tables;
  - `contract_enum!`s `ConflictState` and `ConflictReadNextAction` (`resolve`, `retry_resolution`, `external_repair`, `resume`, `none`);
  - `pub(crate) fn path_tokens(inspection: &SynchronizationConflictInspection) -> Vec<(String, ConflictPathToken)>` and `pub(crate) fn observation_text(observation: &ConflictObservation) -> String` (R-9), which Task 4 uses.

- [ ] **Step 1: Write the conflict-states table as a test** in `tests/remote_merge_recovery/conflict_read.rs`.
  - Copy the private `Pair` helpers (`tests/remote_merge_recovery.rs:595-1262`) that it needs into the module. Do not edit the originals.
  - One case per row. The repaired and refused rows use `install_repair` (`:2335`) and `refused_false_repairs`' Git steps.

  Nothing is committed until Step 7.
- [ ] **Step 2: Run it to see it fail.** `devenv shell -- cargo test --all-features --locked --test remote_merge_recovery -- conflict_read`.
- [ ] **Step 3: Implement** `conflict_read.rs`.
  - Read the step's phase with the internal state functions, then inspect when a conflict is live (fact 29).
  - Derive the tokens and the observation by R-9.
  - Read the sides for readable paths (fact 32, R-10), and escape display paths (R-11).
  - Report each state by the table (R-13).
  - Accept a comment's operation ID (R-5).
  - Prove it writes nothing (R-12): a test compares the database file's bytes before and after.
- [ ] **Step 4: Schemas and goldens.** Closed schemas, each registered as a data root, and the enums in `contract_enumerations` (E-8).
- [ ] **Step 5: Review Focus 2.** A conflict on a path with an escape sequence and a newline: the token resolves, and no raw control byte appears in any `Debug` or diagnostic. Extend the coverage of `hostile_conflict_paths_are_redacted` (`tests/remote_merge_recovery.rs:3181`) by a new case in the new module, not by editing it.
- [ ] **Step 6: Verify.**

  ```sh
  devenv shell -- cargo test --all-features --locked --test remote_merge_recovery
  devenv shell -- cargo test --all-features --locked --test read_contract
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  ```

- [ ] **Step 7: Commit** `feat: read synchronization conflicts with string tokens`. Add a ticket comment and push.

## Task 4: Conflict Resolution Binding And Resume Routes

**Files:**
- Create: `src/repository/mutation/bind/conflict.rs`, `tests/remote_merge_recovery/resolution.rs`, `schemas/v1/conflict_resolution_mutation.schema.json`, goldens `conflict_resolve_ok`, `conflict_resolve_external_change`
- Modify: `src/repository/remote/sync.rs`, `state.rs`, `merge.rs` (resumption, R-14), `command.rs`, `mod.rs`, `dto.rs`, `outcome.rs`, `bind/mod.rs`, `bind/resume.rs`, E-6, E-7, E-8, E-9, E-10, E-11

**Interfaces:**
- Consumes: `path_tokens`, `observation_text` and `show_conflict` (Task 3); `conflict_next` (Task 1); `retry_comment_publication_observed` (Task 2); fact 33.
- Produces:
  - `pub struct ConflictResolveInput { pub root: PathBuf, pub operation_id: OperationId, pub observation: String, pub resolutions: Vec<ConflictResolutionInput> }` and `pub struct ConflictResolutionInput { pub token: String, pub source: String }`;
  - `Mutation::ConflictResolve`, with `Command::ConflictResolve => "conflict resolve"`;
  - `ConflictResolutionMutationDto { r#type, operation_id, commit_oid: Option<String>, publication: PublicationEffect, conflict: Option<ConflictNextDto> }`.

- [ ] **Step 1: Write the interrupted-resolution table and the resume table as tests** in `tests/remote_merge_recovery/resolution.rs`.
  - Interruptions use the library's existing `FailurePoint`s around resolution and publication. Find them with `grep -n 'FailurePoint::' src/repository/remote/sync.rs` and record them in the ledger.
  - Each retry runs in a **fresh** `RepositoryService` opened on the same data directory, so no in-memory state carries over (R-14).

  Nothing is committed until Step 7.
- [ ] **Step 2: Run them to see them fail.** `devenv shell -- cargo test --all-features --locked --test remote_merge_recovery -- resolution`.
- [ ] **Step 3: Implement cross-process resumption (R-14).**
  - Derive the attempt ID from the request.
  - Make the library able to finish an attempt at `ResolutionPrepared`, `CommitPrepared` or `Applied` from its durable rows plus the presented input. Use `remote_resolution_attempts` and `remote_resolution_paths` (`state.rs:818-851`), without storing a body.
  - If any row cannot be met within the Global Constraints' library files, **stop** at a design checkpoint, and record what each row answers.
- [ ] **Step 4: Implement the binding** in `bind/conflict.rs`.
  - Steps 1–5 of the design's Conflict Resolution section, with continuation (R-15) and outcome mapping.
  - Resume's second and third routes (R-20).
  - Add the command to every closed inventory (E-6, E-7, E-8, E-9, E-10), and apply E-11's remainder.
- [ ] **Step 5: Review Focus 3.** A duplicate token and a token from another operation's conflict are both `invalid_input` before any write, and no source text appears in the envelope or stores.
- [ ] **Step 6: Verify.**

  ```sh
  devenv shell -- cargo test --all-features --locked --test remote_merge_recovery
  devenv shell -- cargo test --all-features --locked --test comment_publication
  devenv shell -- cargo test --all-features --locked --test mutation_replay
  devenv shell -- cargo test --all-features --locked --test mutation_contract
  devenv shell -- cargo test --all-features --locked --test read_contract
  devenv shell -- cargo test --all-features --locked --lib mutation
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  ```

- [ ] **Step 7: Commit** `feat: bind conflict resolution and resume resolved synchronizations`. Add a ticket comment and push.

## Task 5: Republish

**Files:**
- Create: `src/repository/mutation/bind/republish.rs`, `tests/mutation_remote/republish.rs`, `schemas/v1/republish_preview.schema.json`, goldens `item_republish_preview`, `item_republish_ok`, `item_republish_external_change`, `item_republish_preview_noop`
- Modify: `src/repository/remote/{sync,refs,reservation,state}.rs`, `command.rs`, `mod.rs`, `dto.rs`, `schemas/v1/preview_effect.schema.json`, `schemas/v1/preview.schema.json`, `src/desktop/view/confirm.rs` (one arm), `src/bin/manyhands-cli/human_admin.rs` (the two exhaustive preview matches), E-6, E-7, E-8, E-9

**Interfaces:**
- Consumes: facts 12, 13 and 36; `bind/sync.rs`'s request builder and observer.
- Produces:
  - `pub struct ItemRepublishInput { pub root: PathBuf, pub kind: AuthoringKind, pub id: ItemId }`, which derives `Clone`;
  - `Mutation::ItemRepublish`, with `Command::ItemRepublish => "item republish"`, `needs_confirmation` and `stops_at_safe_points` both true;
  - `RepublishPreviewDto { root, kind, item_id, branch, last_published_oid, local_oid }`;
  - `PreviewDetailsDto::Republish(RepublishPreviewDto)` and `PreviewEffect::BranchRepublished => "branch_republished"`.

- [ ] **Step 1: Write the republish table as a test** in `tests/mutation_remote/republish.rs`.
  - Delete the published context on the bare remote with the peer.
  - Run `prepare` then `execute` with the confirmation, and cover every row, including the moved-local and reappeared-remote rows.

  Nothing is committed until Step 6.
- [ ] **Step 2: Run it to see it fail.** `devenv shell -- cargo test --all-features --locked --test mutation_remote -- republish`.
- [ ] **Step 3: Implement the consent (R-18).**
  - A crate-private `RepublishConsent { branch, last_published, local }` reaches each of fact 36's five refusal points through new crate-private parameters (R-4). Each point lets it through only when it equals the live observation.
  - Ordinary `item sync` passes none.
- [ ] **Step 4: Implement the binding and the preview (R-19).**
  - Use `operation abandon`'s confirmed shape (`bind/abandon.rs`) for `preview`, `row` and `observe_again`.
  - Add the desktop arm with three fields, "Branch", "Last published" and "Local commit", and the two CLI arms.
  - Add the command to every closed inventory (E-6, E-7, E-8, E-9).
- [ ] **Step 5: Verify.**

  ```sh
  devenv shell -- cargo test --all-features --locked --test mutation_remote
  devenv shell -- cargo test --all-features --locked --test mutation_confirmation
  devenv shell -- cargo test --all-features --locked --test mutation_contract
  devenv shell -- cargo test --all-features --locked --test read_contract
  devenv shell -- cargo check --locked --features desktop
  devenv shell -- cargo clippy --all-targets --locked --features desktop -- -D warnings
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  ```

- [ ] **Step 6: Commit** `feat: republish a remotely deleted item branch with consent`. Add a ticket comment and push.

## Task 6: Part A Checkpoint

- [ ] **Step 1:** Merge `origin/main` if it moved, and run the checkpoint gate.
- [ ] **Step 2:** Prove that Part A's library diff stays inside the Global Constraints' files: `git diff --stat <Task 0 HEAD>..HEAD -- src/repository src/repository.rs src/lib.rs src/results.rs src/runtime.rs`. Also prove `src/results.rs` is unchanged.
- [ ] **Step 3:** An independent review of Part A's range: spec, then quality. Use the adversarial brief on R-14, R-17 and R-18. Fix or rule every finding.
- [ ] **Step 4:** Record counts and reviews in the ledger. Add a ticket comment and push.

## Task 7: The Ctrl-C Window And The Sync, Resume And Abandon Verbs

**Files:**
- Create: `src/bin/manyhands-cli/interrupt.rs`, `interrupt/unix.rs`, `interrupt/windows.rs`, `interrupt_tests.rs`, `collaboration.rs`, `collaboration_tests.rs`, `human_collaboration.rs`, `human_collaboration_tests.rs`, `tests/cli/collaboration.rs`
- Modify: `src/bin/manyhands-cli.rs`, `catalog.rs`, `mutate.rs`, `dispatch.rs`, `tests/cli.rs`, `tests/support/cli.rs`, help goldens (E-1)

**Interfaces:**
- Consumes: `Mutation::{ItemSync, RepoSync, OperationResume, OperationAbandon}` (fact 2); `show_item` (fact 37); `mutate::run` (fact 40).
- Produces:
  - `pub(crate) fn during<T>(token: &CancellationToken, work: impl FnOnce() -> T) -> T` in `interrupt.rs`;
  - a `Plan.stoppable: bool` field on `mutate::Plan`, so that `execute` gets the window's token;
  - `pub const OPERATION_ID: OptionSpec` in `catalog.rs`.

- [ ] **Step 1: Write the window's unit tests first** in `interrupt_tests.rs`, Unix only:
  - after `during` returns, `sigaction` reports the disposition it had before;
  - a `raise(SIGINT)` inside `during` cancels the token, and the process continues;
  - `during` nested in a test returns its closure's value.

  The Ctrl-C through the pseudo-terminal needs a synchronization held at a safe point, which needs the SSH fixture. It is Task 13's.
- [ ] **Step 2: Implement `interrupt.rs`** (R-21):
  - Unix: `libc::sigaction` with a static `OnceLock`/atomic slot for the token. The first signal calls `cancel()`; the second restores `SIG_DFL` and `raise`s.
  - Windows: `SetConsoleCtrlHandler`. The first `CTRL_C_EVENT` cancels and returns `TRUE`; the second returns `FALSE`.
  - Always restore on return.
- [ ] **Step 3: Wire `mutate::run`.** With `plan.stoppable`, wrap `execute` in `during` with a token made for the call. Otherwise behavior is unchanged. The retry loop still never follows `cancelled` (R-23).
- [ ] **Step 4: Dispatch four verbs** in `collaboration.rs`: `item sync`, `repo sync`, `operation resume` and `operation abandon`.
  - Kind lookup by `show_item` (R-2).
  - `operation abandon` uses C2's confirmed flow and its consent prompt.
  - Flip them to `Available("C4")`, add `OPERATION_ID`, and regenerate the help goldens (E-1).
- [ ] **Step 5: Human output** in `human_collaboration.rs`: conflict and next lines, the `remote_branch_deleted` next line, and progress.
- [ ] **Step 6: Goldens and tests** in `tests/cli/collaboration.rs`, without SSH:
  - `repo_sync_publication_remote_required`, `item_sync_item_not_found`, `operation_resume_not_found` and `operation_abandon_preview`;
  - text goldens for each verb;
  - abandon's prepare, confirm and interactive cases;
  - C3's stuck save closed by abandon, with its file unchanged (R-30);
  - **Review Focus 4:** a Ctrl-C (0x03 through `PtyCli`) at `operation abandon`'s consent prompt is exit 130, with the operation still pending and no accepted request record.
- [ ] **Step 7: Verify.**

  ```sh
  devenv shell -- cargo test --all-features --locked --bin manyhands-cli
  devenv shell -- cargo test --all-features --locked --test cli
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo check --locked --bin manyhands-cli
  ```

- [ ] **Step 8: Commit** `feat: add sync, resume and abandon verbs with a Ctrl-C window`. Add a ticket comment and push.

## Task 8: `comment add`

**Files:**
- Create: `src/bin/manyhands-cli/discussion.rs`, `discussion_tests.rs`, `tests/cli/discussion.rs`, `schemas/input/v1/comment_add.schema.json`, `tests/fixtures/cli_input_v1/comment_add.json`
- Modify: `catalog.rs`, `dispatch.rs`, `human_collaboration.rs`, `tests/cli.rs`, `tests/support/cli.rs`, help goldens (E-1)

**Interfaces:**
- Consumes: `Mutation::CommentAdd` and `CommentMutationDto` (Task 2); `interrupt::during` and `Plan.stoppable` (Task 7).
- Produces: `discussion::comment_add(args, session, service) -> Output`.

- [ ] **Step 1: Write tests first** in `tests/cli/discussion.rs`, all without a remote:
  - the input schema's fields;
  - `comment_add_ok` with publication pending, exit 0;
  - `comment_add_ticket_closed`;
  - `comment_add_invalid_input`, for a malformed `created_by`;
  - a reply's parent and depth through `comment list`;
  - human text with the publication note;
  - a lost-output replay that writes no second file.
- [ ] **Step 2: Implement.**
  - `only` with the four keys.
  - `comment_id` and `parent_id` parsed as ULIDs, as `invalid_id`.
  - Kind by `show_item` (R-2).
  - `Plan { retry: true, stoppable: true, .. }`.
- [ ] **Step 3:** Flip `comment add` to available, and regenerate the help goldens (E-1).
- [ ] **Step 4: Verify.** As Task 7, Step 7, plus `devenv shell -- cargo test --all-features --locked --test cli -- input_schemas`.
- [ ] **Step 5: Commit** `feat: add comment add`. Add a ticket comment and push.

## Task 9: `conflict show` And `conflict resolve`

**Files:**
- Create: `src/bin/manyhands-cli/conflict.rs`, `conflict_tests.rs`, `tests/cli/conflicts.rs`, `schemas/input/v1/conflict_resolve.schema.json`, `tests/fixtures/cli_input_v1/conflict_resolve.json`
- Modify: `input.rs`, `input_tests.rs`, `catalog.rs`, `dispatch.rs`, `human_collaboration.rs`, `tests/cli.rs`, `tests/support/cli.rs`, help goldens (E-1)

**Interfaces:**
- Consumes: `show_conflict` (Task 3); `Mutation::ConflictResolve` (Task 4).
- Produces: `input::InputObject::object_list(&self, key) -> Result<Vec<InputObject>, InputError>`; `conflict::show` and `conflict::resolve`.

- [ ] **Step 1: Tests first.**
  - `object_list` unit tests: a non-array, an array with a non-object, and an empty array, each refused.
  - In `tests/cli/conflicts.rs`, a local conflict built with the library actor:
    1. `conflict show` JSON and text;
    2. its tokens fed to `conflict resolve`;
    3. a stale observation, `external_change`, exit 3;
    4. a missing token, `invalid_input`, exit 2;
    5. `conflict_show_operation_not_found`.
- [ ] **Step 2: Implement** `object_list`, both verbs and their human output. `conflict show` prints sides only in JSON (design, Human Output).
- [ ] **Step 3:** Flip both verbs and regenerate the help goldens (E-1). The example `conflict_resolve.json` validates by schema (design, Input).
- [ ] **Step 4: Review Focus 2 through the binary.** A hostile path's human output is escaped.
- [ ] **Step 5: Verify.** As Task 8, Step 4.
- [ ] **Step 6: Commit** `feat: add conflict show and conflict resolve`. Add a ticket comment and push.

## Task 10: `item republish`

**Files:**
- Create: `tests/cli/republish.rs`
- Modify: `collaboration.rs`, `catalog.rs`, `catalog_tests.rs` (E-2), `dispatch.rs`, `tests/cli.rs`, `tests/support/cli.rs`, help goldens (E-1)

- [ ] **Step 1: Tests first**, without SSH:
  - `item_republish_publication_remote_required`: `--prepare` on a repository with no publication remote;
  - `--confirm` with an unknown confirmation is `confirmation_not_found`;
  - `--prepare` together with `--request-id` is a usage error, as C2's mode rules give.

  A deleted branch needs a recorded publication, which needs the SSH fixture. The preview golden `item_republish_preview`, confirm and interactive consent are Task 12's.
- [ ] **Step 2: Implement.** Add the catalog entry `Available("C4")` "Republish a deleted item branch.", and use C2's confirmed flow with `Plan.stoppable`. Apply E-2.
- [ ] **Step 3:** Regenerate the help goldens (E-1), and read the diff: one added row.
- [ ] **Step 4: Verify.** As Task 8, Step 4, plus `devenv shell -- cargo test --all-features --locked --bin manyhands-cli -- catalog`.
- [ ] **Step 5: Commit** `feat: add item republish`. Add a ticket comment and push.

## Task 11: Part B Checkpoint

- [ ] **Step 1:** Merge `origin/main` if it moved, and run the checkpoint gate.
- [ ] **Step 2:** Prove that C5's verbs still answer `command_unavailable`, and help and version still start no transport: the existing tests pass unchanged.
- [ ] **Step 3:** An independent review of Part B's range: spec, then quality. Fix or rule every finding.
- [ ] **Step 4:** Ledger, ticket comment, push.

## Task 12: Two-Clone SSH Journeys

**Files:**
- Create: `tests/support/cli_pair.rs`, `tests/cli_remote/collaboration.rs`
- Modify: `tests/cli_remote.rs`

**Interfaces:**
- Produces: `pub struct CliPair { pub server: SshRemoteFixture, pub a: CliClone, pub b: CliClone }`, with `CliClone { env: CliEnv, root: PathBuf }` and `CliPair::new() -> Result<CliPair, FixtureError>`. Each clone is enabled, with a publication remote, its own selected key allowed by the server, and an approved host, all set up through the CLI.

- [ ] **Step 1:** Build `CliPair`. Each setup step goes through the binary with `checked`, and every run goes through `scan_output`.
- [ ] **Step 2: Write the Wave's two-clone rows as `cli_remote` cases,** each under the 100-second watchdog (R-24):
  - comment publish success, and failure by server refusal;
  - ordered replies from both clones;
  - one saved comment after a retry;
  - already-current, fast-forward and clean divergence for `item sync` and `repo sync`;
  - a canonical conflict shown and resolved;
  - a stale resolution;
  - an outside repair of a code-file conflict, then resume, plus the three refused repairs;
  - republish after the peer deletes the branch: `item sync`'s refusal, the `item_republish_preview` golden, confirm, a moved local branch refused at confirm, and a Ctrl-C at the interactive consent prompt (Review Focus 4).
- [ ] **Step 3: Verify.** `devenv shell -- cargo test --all-features --locked --test cli_remote -- collaboration`. Then run the whole `cli_remote` target once.
- [ ] **Step 4: Commit** `test: prove C4 journeys across two clones over SSH`. Add a ticket comment and push.

## Task 13: Interruption, Lost Output And Exit Evidence

**Files:**
- Create: `tests/cli_remote/interruption.rs`, `tests/cli/collaboration_exits.rs`
- Modify: `tests/cli_remote.rs`, `tests/cli.rs`

- [ ] **Step 1: Ctrl-C through the PTY over SSH.**
  - `item sync` held by `stall_at` is cancelled to exit 130, with effects; a new request completes it.
  - `comment add` cancelled during publication keeps the checkpoint; `operation resume` publishes.
  - `operation resume` cancelled at a safe point (fact 41).
- [ ] **Step 2: Lost stdout.** `item sync` and `comment add` run with stdout closed (`boundaries.rs:247`'s pattern) exit 1. Then `request show` returns the stored result, and a replay makes no second push, file or checkpoint (R-25).
- [ ] **Step 3: The exit table (R-27).** Map every code in Task 0's union to its test, or to a reason no fixture reaches it, in `collaboration_exits.rs` and the ledger. Assert set equality with the union.
- [ ] **Step 4: Verify.**

  ```sh
  devenv shell -- cargo test --all-features --locked --test cli_remote -- interruption
  devenv shell -- cargo test --all-features --locked --test cli -- collaboration_exits
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  ```

- [ ] **Step 5: Commit** `test: prove C4 interruption, lost output and exit coverage`. Add a ticket comment and push.

## Task 14: Document Amendments

- [ ] **Step 1:** Make every amendment in the design's Document Amendments, except AGENTS.md, the skills and the ticket. Use the dated form "Amended 2026-10-26 by C4" (or the implementation date), and cite the decision or ruling.
- [ ] **Step 2:** Reconcile the API audit's C4 rows with what was delivered.
- [ ] **Step 3:** Grep the three planning documents and the amended RFCs for stale terms: `publication_pending`, `conflict.show`, `{path, observation, source}`, "never merges", "refuse divergence".
- [ ] **Step 4: Commit** `docs: amend RFCs, Wave and audit for C4`. Add a ticket comment and push.

## Task 15: Full Verification, Review, Dogfooding, Switch And Handoff

- [ ] **Step 1:** Merge `origin/main`, then run the checkpoint gate and the full suite once, detached, with a two-hour budget. Record per-target counts in the ledger. A failure stops for the owner.
- [ ] **Step 2:** Independent whole-branch reviews, spec and quality, of `<Task 0 HEAD>..HEAD`. Fix or rule every finding, and re-review the fixes.
- [ ] **Step 3: Dogfooding, only with the owner's go-ahead at this step (R-26).** Follow the design's Dogfooding section exactly:
  - preflight;
  - the request ID recorded before sending;
  - one `comment add` of the review-ready comment, with `created_by` `Claude Opus 5.5 <noreply@anthropic.com>`;
  - the value check with `comment list`.

  A refusal or partial result stops for the owner.
- [ ] **Step 4: The switch.** Edit the files of fact 48 as the design's Dogfooding step 5 says. Commit `docs: switch comment dogfooding to the CLI`, and give that range its own independent spec and quality review.
- [ ] **Step 5: Review-ready.**
  - Tick the ticket's planning boxes.
  - Mark the Cycle and design `review-ready`.
  - Run `git merge-tree --write-tree HEAD origin/main`.
  - Push the ticket branch.

  The ticket stays **open**.
- [ ] **Step 6: Closure**, only after the owner approves the review, and in the final push before merge. Close the ticket by the canonical ticket-file edit, until C5's `ticket close`. Merging needs its own authorization.

## Plan Self-Review

**Acceptance rows to tasks:**

| Cycle acceptance row | Task |
| --- | --- |
| Comment publish success and failure | 2 (library), 8 (binary), 12 (two clones) |
| Ordered replies | 12 |
| One saved comment after retry | 2, 12 |
| Clean and divergent sync | 1, 12 |
| Stale resolutions | 4, 9, 12 |
| Outside repair of a noncanonical conflict | 3, 4, 12 |
| No secret or raw-server output | 12, 13 (every `cli_remote` case scans) |
| Ctrl-C | 7, 13 |
| Lost stdout | 13 |
| Dogfooding switch | 15 |
| Republish | 5, 10, 12 |
| Resume and abandon | 2, 4, 7 |
| Identity | 1 |
| Outcome tables | 1, 2, 3, 4, 5 |
| Exit classes | 13 |
| Schemas, goldens and examples | 1–5, 7–10 |
| Mutating-Cycle evidence | 7–10, 12, 13 |
| Unchanged behavior | 11 |
| Documents | 14, 15 |

**Rulings to tasks:**

| Task | Rulings |
| --- | --- |
| 1 | R-8, R-16, R-17 |
| 2 | R-3, R-4, R-5, R-6, R-7, R-20 |
| 3 | R-9, R-10, R-11, R-12, R-13 |
| 4 | R-14, R-15, R-20 |
| 5 | R-18, R-19 |
| 7 | R-1, R-2, R-21, R-22, R-23, R-30 |
| 8–10 | R-1, R-2 |
| 12 | R-24 |
| 13 | R-25, R-27, R-28 |
| Every task | R-29 |
| 15 | R-26 |

**Named edits:** E-1 in 7–10; E-2 in 10; E-3 to E-5 in 1, 2 and 4; E-6 to E-10 in 1–5; E-11 in 2 and 4.

**Stops built in:**
- Task 0: the gate merged; a changed fact; an unlisted test hit.
- Tasks 1–5: each outcome-table row.
- Task 1: R-17's mechanism.
- Task 4: R-14's mechanism.
- Task 15: dogfooding.

**Placeholders:** none intended. The parent refusal's code, the failure points and the E-3 classification are Task 0 records, not choices left to an implementer.
