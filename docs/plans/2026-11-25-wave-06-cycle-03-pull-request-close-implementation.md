---
title: "Wave 06 Cycle 03 Implementation Plan: Ticket Close In Pull-Request Mode, The Closing State And Withdraw"
date: 2026-11-25
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M899G2Y5Q4H3SQSSJVFTCR3N"
---

# Wave 06 Cycle 03 Implementation Plan: Ticket Close In Pull-Request Mode, The Closing State And Withdraw

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In a pull-request repository, close a ticket by recording its closure on the ticket's branch, publishing the branch and giving the pull request link; show such a closure as `closing` until the primary holds it; add `ticket withdraw-closure`; settle two clones' closures; close a merged-first ticket whose remote branch is gone, or a ticket with no context, through a new branch holding only the closure.

**Architecture:** Reads derive `closing` from the two copies the index already stores (`src/repository/read/items.rs`, `graph.rs`). The closed-ticket guard and closure immutability take the mode and the merge's primary side (`ticket_guard.rs`, `integration/validate.rs`). A new runner, `src/repository/closure/pull_request.rs`, with its record `pull_request_closures`, runs ordinary context synchronizations around a closure or withdraw commit, or retires an old context and makes a new branch. A closure-only conflict settles itself in the remote merge (`remote/sync.rs`). A registry table records given links. `ticket close`'s binding hands off by mode, and a new verb binds withdraw.

**Tech Stack:** Rust, `git2`, `rusqlite`, the existing `src/forge.rs`, the SSH test harness for one case.

**Spec:** [Cycle 03 design](2026-11-25-wave-06-cycle-03-pull-request-close-design.md) (revision 1), under the [Cycle document](../Cycles/wave-06-cycle-03-pull-request-close.md) and the [Pull-Request Mode RFC](../RFC/pull-request-mode.md).

Status: **draft** (revision 1).

## Global Constraints

**Precondition.** The owner has approved the three documents, approved the test edits, authorized implementation and chosen the execution method. Task 0 does not start before that.

**Roles.** The orchestrator runs the tasks, or dispatches one implementer per task and then a fresh reviewer, one at a time. It alone writes the ledger and ticket comments and publishes the ticket branch. Implementers never dispatch subagents and never edit ticket, comment or ledger files.

**Where.** Every task runs in `.manyhands/worktrees/01M87683QK8E34EYY8KS4VQG9Y` on `manyhands/ticket/01M87683QK8E34EYY8KS4VQG9Y`. No lane branches, no extra worktrees, no copies of `target/`.

**Cargo.** Every Rust command is `devenv shell -- cargo …` with `--locked`. One cargo command at a time on this machine (memory pressure ends the whole session). Desktop tests run with `--features desktop-test-support --bin manyhands`, on their own.

**Registry.** The registry migration is one-way. Never run this branch's CLI or library against the owner's registry. Any run outside `cargo test` uses an isolated `HOME`, `MANYHANDS_DATA_DIR` and `GIT_CONFIG_GLOBAL` under a temporary root. Ticket operations run only from the up-to-date main checkout, as `devenv shell -- cargo run --locked --bin manyhands-cli -- …`.

**Tests (AGENTS.md Testing Strategy).**

- Between steps, run only the tests the step wrote or changed, by exact name: `--lib <module path>::<test> -- --exact`, `--bin manyhands-cli <module path>::<test> -- --exact`, or `--test <target> <name> -- --exact`. Task 0 confirms each module path this plan names.
- Every test command must report more than zero tests run. A run of zero is a failed step.
- Never run a whole SSH-harness target between steps. Stop and narrow any single test command that runs past about 15 minutes.
- Each new test sits at the cheapest layer that catches its regression. The plan adds one SSH case (Task 11), for the reason the design's Testing section gives.
- The fast tier and the SSH tier run once, in Task 13.
- No real waits: the runner takes its clock as a parameter.

**Every new check fails first.** Each new test or assertion is run red once, before its code or against a deliberate stub, and the red run is recorded in the ledger. A row that pins unchanged behaviour (G3 to G6, the direct-mode rows of P4 and P5) is exempt.

**Test edits.** The design's named edits E-1 to E-16, and the edits Task 0's sweep finds, under the owner's test-edit approval: each logged in the ledger as E-17 onwards with its reason. An edit that removes or weakens an assertion's intent stops for the owner.

**Main.** Merge `origin/main` into the ticket branch in Task 0, before Tasks 5, 9 and 13, and whenever main moves. Never rebase. When Cycle 02 (vh-c97se) lands, merge it and re-check design Part C and Part H against it (decision 5).

**Publish.** After each task's commit, the orchestrator publishes the branch with `item sync` from the main checkout and checks that `HEAD` equals `origin/manyhands/ticket/01M87683QK8E34EYY8KS4VQG9Y`.

**Commits.** Each ends with the trailer `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Static gates** (Task 13, and before any commit that touches Rust):

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

**Ticket checkpoints.** The orchestrator adds a comment to vh-gy8gz after Tasks 0, 3, 5, 7, 9 and 11, and at review-ready, each naming the tests that ran. Comments are added from the main checkout with `manyhands-cli comment add --repo /home/vhodges/work/src/manyhands --id 01M87683QK8E34EYY8KS4VQG9Y --input <file> --non-interactive --json --request-id <ULID>`.

## Review Focus

- A closing ticket never unblocks a dependent, in any read. Tasks 1 and 10 pin it.
- No runner path moves the local primary or targets the primary in a synchronization. Tasks 5 to 7 pin it with the publisher that refuses a primary target, and Task 11 with the server's primary.
- A same-request retry after each crash point repeats no commit and keeps `closed_at`. Tasks 5 and 7 pin it.
- A hand edit of `closed_at` in the worktree cannot be saved. Task 2 pins it.
- A closure the primary holds cannot be dropped by any resolution. Task 2 pins it.

---

## File Map And Dependency Order

| Task | Files | Depends on |
| --- | --- | --- |
| 0 | the ledger `docs/plans/2026-11-25-wave-06-cycle-03-pull-request-close-execution.md` | approval |
| 1 | `src/repository/read/dto.rs`, `items.rs`, `graph.rs`, the five schemas of Part A, `src/bin/manyhands-cli/parse.rs`, `dispatch.rs`, `catalog.rs`, the desktop arms; `tests/read_items.rs`, `tests/read_relationships.rs`; E-1 to E-3, E-16 | 0 |
| 2 | `src/repository/ticket_guard.rs` and its callers, the domain save, `src/repository/integration/validate.rs` and its callers; their tests; E-14 | 0 |
| 3 | `src/repository/discovery.rs` (the migration), `src/repository/closure/pull_request.rs` (the record module, tables only), the link record module, `cleanup.rs` `retire_context`, `ItemDto.pull_request`, `item.schema.json`, `pull_request.schema.json` and `PullRequestDto` if Cycle 02 has not landed; E-4 | 1 |
| 4 | `src/results.rs`, `src/results_tests.rs`, `src/repository/mutation/outcome.rs`, the two schema code lists, `PreviewEffect` and its schema, `human_admin.rs` `effect_phrase`; E-5, E-6, part of E-9 | 0 |
| 5 | `src/repository/closure/pull_request.rs` (the runner, `record`), `pull_request_tests.rs` | 2, 3, 4 |
| 6 | the same, `withdraw` | 5 |
| 7 | the same, `new_branch`; `context_retirement/proof.rs` (the unheld entries); the new context-at-commit function in `src/repository.rs` | 5 |
| 8 | `src/repository/integration/validate.rs` (the pure settlement), `src/repository/remote/sync.rs` (`integrate_divergence`), `sync_tests.rs` | 2 |
| 9 | the mutation layer (`command.rs`, `mod.rs`, `bind/`, `dto.rs`), `recovery.rs`, `read/status.rs`, `bind/resume.rs`, the two new schemas, the CLI (`catalog.rs`, `parse.rs`, `dispatch.rs`, `lifecycle.rs`, `human_lifecycle.rs`, `human_admin.rs`), the desktop arms, `tests/cli/lifecycle.rs`; E-8, E-9 to E-13 | 5, 6, 7 |
| 10 | `src/repository/closure/pull_request_tests.rs` (the lifecycles) | 5 to 8 |
| 11 | `tests/mutation_remote.rs` or its `c5_lifecycle.rs` module, `tests/support/mutation_golden.rs`, `tests/mutation_contract.rs`, the goldens; E-7 | 9 |
| 12 | `docs/RFC/*`, `docs/research/wave-03-api-audit.md`, `skills/`, `.agents/skills/`; E-15 | 9, 11 |
| 13 | none new | all |

## Task 0: Baseline, References, Ledger

**Files:** create the ledger `docs/plans/2026-11-25-wave-06-cycle-03-pull-request-close-execution.md`.

- [ ] **Step 1:** In the worktree, `git fetch origin`, `git merge origin/main`, `git merge-base --is-ancestor origin/main HEAD`, `git status --short`. Record the base SHA and HEAD before and after in the ledger.
- [ ] **Step 2: Cycle 02.** Check whether `origin/main` holds `schemas/v1/pull_request.schema.json` and `PullRequestDto`. Record which. If they exist in another shape than design Part C's, stop for the owner.
- [ ] **Step 3:** Re-check every name, path and line the design cites (Baseline Facts 1 to 21, Parts A to H) at the new HEAD. Mark each "existing" or "new" in a ledger table with its current file. Confirm the test module paths: `repository::closure::pull_request::tests` (new), `repository::ticket_guard::tests`, `repository::integration::tests`, `repository::remote::sync::tests::closure_conflict_classification`, `repository::context_retirement::tests`, `repository::cleanup::tests`, `repository::discovery::tests`, `results::tests`, and the targets `read_items`, `read_relationships`, `read_contract`, `mutation_contract`, `mutation_bridges`, `cli`.
- [ ] **Step 4: U-1 and U-2.** Read `src/repository/mutation/bind/sync.rs` and `src/repository/remote/sync.rs`: whether a context synchronization stopped by a transport error before a push keeps its reservation, and whether a context synchronization refuses when the root checkout is off the primary or not clean. Record both with file and line; if U-2 holds, Part D's preview gains that blocker (record the ruling).
- [ ] **Step 5: Premise sweep.** Grep for every site of the design's sweep list (Named Edits, last paragraph) and add each hit to the ledger as a named edit E-17 onwards.
- [ ] **Step 6:** Confirm that no CI test list changes: this Cycle adds no test target.
- [ ] **Step 7:** Commit the ledger. The orchestrator publishes the branch and adds the start checkpoint comment.

## Task 1: The Closing State (Design Part A)

**Files:**
- Modify: `src/repository/read/dto.rs` (`ClosureState`, `DependencyState`, `ReadinessReasonCode`, `ClosureDto`'s documentation), `src/repository/read/items.rs` (the derivation helper, `Target`, `ClosureFilter`, `TicketFilter::matches`), `src/repository/read/graph.rs` (`TicketNode`, `blockers`, the tree and root lines, `ticket_readiness`)
- Modify: `schemas/v1/item.schema.json`, `dependency.schema.json`, `dependency_tree_node.schema.json`, `readiness_reason.schema.json`
- Modify: `src/bin/manyhands-cli/parse.rs`, `dispatch.rs`, `catalog.rs` (the `--closure` value and help)
- Modify: the desktop's exhaustive matches (design Baseline Fact 5), compile-level arms only
- Test: `tests/read_items.rs`, `tests/read_relationships.rs`
- Named edits: E-1, E-2, E-3, E-16

**Interfaces:**
- Produces: `ClosureState::Closing` (`"closing"`), `DependencyState::Closing` (`"closing"`), `ReadinessReasonCode::ClosingDependency` (`"closing_dependency"`), `ClosureFilter::Closing`; a private `fn closure_state(effective: &StoredItem, primary: Option<&StoredItem>) -> ClosureState` in `items.rs` (the exact parameter types follow the rows the read holds).

- [ ] **Step 1: Write the failing table test** `closure_state_follows_both_copies` in `tests/read_items.rs`, beside `presentation_across_contexts` and with its helpers: for each row of design Part A's derivation table (a ticket with no closure; a closure on the primary only; on both; on the active context only; on the active context with no primary copy), assert `closure.state` in `ticket list` and `ticket show`, and for a second ticket that depends on it and has it as `parent`, `deps[].state` and `parent.state`.
- [ ] **Step 2: Write the failing readiness test** `a_closing_dependency_blocks_until_the_primary_holds_it` in `tests/read_relationships.rs`, beside `closing_a_dependency_makes_its_dependent_ready_without_the_dependent_being_observed_again` and with its helpers: ticket B depends on A; A closing gives B `blocked` with one `closing_dependency` reason naming A, `ticket ready` without B and with A (its own dependencies closed), `ticket blocked` with B, `ticket deps` lines `closing`; then A's closure on the primary's copy too gives B ready, and A `closed`.
- [ ] **Step 3: Write the failing filter test** `closure_filter_closing_and_open` in `tests/read_items.rs`: over an open, a closing and a closed ticket, `Open` gives the first two, `Closing` the second, `Closed` the third, `All` three; and extend `parse_tests.rs`'s valid list with `closing` (E-3).
- [ ] **Step 4: Run, expect FAIL**, each by exact name. Record the red runs.
- [ ] **Step 5: Implement** Part A: the enum values in order, the derivation helper used by lists and `show`, `TicketNode.closing`, the reason, the tree and root lines, `ticket_readiness`'s keep rule, the filter, the parser, dispatch (an explicit `closing` arm) and the help text, the schema enums, the desktop arms. Apply E-1 to E-3 and E-16; regenerate the three help goldens with `MANYHANDS_UPDATE_GOLDEN=1` for the tests that own them and read the diff.
- [ ] **Step 6: Run** the new tests, `published_enumerations_list_exactly_the_registered_values` (`--test read_contract`), the edited tests and `presentation_across_contexts` by exact name. Then `devenv shell -- cargo check --locked --features desktop-test-support --tests`. Expected: PASS.
- [ ] **Step 7: Commit** "feat(read): the closing state".

## Task 2: Guards, The Save Check And Immutability (Design Part B)

**Files:**
- Modify: `src/repository/ticket_guard.rs` (`require_ticket_open` gains `integration: Integration`), its callers (`src/repository.rs` `refuse_closed_ticket`, `context_release.rs`, `bind/republish.rs`, `bind/resolve_history.rs`)
- Modify: the domain ticket save (`src/repository.rs` `save_ticket_with_effective_config`): the save check
- Modify: `src/repository/integration/validate.rs` (`preserves_item_invariants`, `validates_resolution_sides`, `validates_external_integration` gain the primary side) and every caller (design Baseline Fact 7)
- Test: `src/repository/ticket_guard_tests.rs`, `src/repository/integration/tests.rs`, `tests/mutation_bridges/context.rs`
- Named edits: E-14

- [ ] **Step 1: Write the failing tests:**
  - `the_guard_reads_the_primary_only_in_pull_request_mode` in `ticket_guard_tests.rs`, a table over mode × (closure on primary, on branch, on both, on neither): direct refuses when either holds it; pull-request mode only when the primary holds it.
  - `a_save_cannot_change_the_closure` in `ticket_guard_tests.rs`: in each mode, a worktree file with `closed_at` added by hand, changed, or removed against the head is refused with `invalid_input` and nothing is committed; a save of a closing ticket's title in a pull-request repository succeeds and keeps the head's closure.
  - `a_closing_ticket_takes_comments_and_saves` in `tests/mutation_bridges/context.rs`, beside `prepare_edit_of_a_closed_ticket_is_ticket_closed_and_makes_nothing`: in a pull-request repository, `comment add` (local) and `ticket save` of a closing ticket succeed; with the closure on the primary they answer `ticket_closed`.
  - `provisional_closures_may_change_primary_ones_may_not` in `src/repository/integration/tests.rs`, a table of `preserves_item_invariants` over (old, new, primary): open to any; the same closure; a provisional closure dropped, replaced by none or by the primary's; a closure the primary holds dropped or changed (refused); and one row each through `validates_resolution_sides` and `validates_external_integration`.
- [ ] **Step 2: Run, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Implement** Part B. Each validator caller passes its merge's primary input: the integration window's `primary_oid` in `remote/sync.rs` and `conflict_read.rs`, the local primary in `local_sync/`. Apply E-14.
- [ ] **Step 4: Run** the new tests, the 13 guard tests, the `closure_conflict_classification` module and the validators' existing tests by exact name. Expected: PASS.
- [ ] **Step 5: Commit** "feat(closure): the guard per mode, the save check and provisional closures".

## Task 3: The Link Record And The Item's Pull Request (Design Part C)

**Files:**
- Modify: `src/repository/discovery.rs` (`migrate_registry` calls a new `migrate_pull_request_close` after `migrate_pull_request_mode`, creating `pull_request_links` and `pull_request_closures` by checking the schema, marker `wave_06_pull_request_close_v1`)
- Create: `src/repository/closure/pull_request.rs` with the record's DDL and row functions only (the runner comes in Task 5), and `src/repository/pull_request_links.rs` (new, declared in `src/repository.rs`) with the link record's functions
- Modify: `src/repository/cleanup.rs` `retire_context` (delete the item's link row), `src/repository/read/dto.rs` (`ItemDto.pull_request`), the `ticket show` builder, `schemas/v1/item.schema.json`
- Create if Cycle 02 has not landed: `schemas/v1/pull_request.schema.json` and `PullRequestDto { source, target, forge, url }`, with `forge`'s enum registered
- Test: `src/repository/discovery.rs` tests, `tests/read_items.rs`, `src/repository/cleanup_tests.rs`
- Named edits: E-4

**Interfaces:** Produces `PullRequestDto` (Cycle 02's or this Cycle's), `ItemDto.pull_request: Option<PullRequestDto>`, and the link record's `upsert_link(tx, repository_id, kind, item, &PullRequestLink, given_at)`, `link_of(connection, repository_id, kind, item) -> Option<PullRequestDto>` and `delete_link(tx, repository_id, kind, item)`.

- [ ] **Step 1: Write the failing tests:**
  - `pull_request_close_migration_creates_its_tables_once` beside Cycle 01's `pull_request_mode_migration_adds_columns_once_and_refills`: a registry without the tables or the marker migrates to both tables, the unique unfinished index and one marker; a second migrate changes nothing; a registry with the marker but without a table gets it back.
  - `ticket_show_gives_the_recorded_link` in `tests/read_items.rs`: with a link row, `ticket show` gives it and `ticket list` gives null; a document gives null.
  - `retirement_deletes_the_link` in `src/repository/cleanup_tests.rs`: a retired ticket's link row is gone.
- [ ] **Step 2: Run, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Implement** Part C and the tables. Apply E-4: regenerate the item goldens for the tests that own them with `MANYHANDS_UPDATE_GOLDEN=1`, read the diff, and confirm that only `"pull_request": null` was added.
- [ ] **Step 4: Run** the new tests, the golden tests of E-4 and `published_enumerations_list_exactly_the_registered_values` by exact name. Expected: PASS.
- [ ] **Step 5: Commit** "feat(index): the link record and the item's pull request". The orchestrator adds the checkpoint comment.

## Task 4: The Code And The Preview Effects (Design Part H)

**Files:** `src/results.rs` (`AlreadyClosing`, class `None`, message "The ticket's branch already holds a closure.", appended after `AlreadyApplied`; the documentation of class `None`), `src/results_tests.rs`, `src/repository/mutation/outcome.rs` (`recovery`'s shared arm), `schemas/v1/envelope.schema.json`, `schemas/v1/request.schema.json`, `src/repository/mutation/dto.rs` (`PreviewEffect` gains `ContextPublished`, `PullRequestLink`, `ClosureWithdrawn`, `ContextCreated`, appended), `schemas/v1/preview_effect.schema.json` (values and description), `src/bin/manyhands-cli/human_admin.rs` (`effect_phrase`), `tests/mutation_contract.rs`, `tests/read_contract.rs`. Named edits: E-5, E-6, E-9 (the effects).

- [ ] **Step 1: Apply E-5**, so `results::tests` fails. Run `devenv shell -- cargo test --all-features --locked --lib results::tests`. Expected: FAIL.
- [ ] **Step 2: Implement** the code, its recovery arm, the two schema lists, the four effects, their phrases and the schema description.
- [ ] **Step 3: Apply E-6** and the effects' part of E-9. Run `results::tests`, the `tests/read_contract.rs` tests that compare the code and effect enums, and both `every_result_code_has_a_*` tests of `tests/mutation_contract.rs`, each by exact name. Expected: PASS.
- [ ] **Step 4: Commit** "feat(results): already_closing and the pull-request preview effects".

## Task 5: The Runner, Record (Design Part D; Test-First With A Design Checkpoint)

**Files:** `src/repository/closure/pull_request.rs` (the runner), `src/repository/closure/pull_request_tests.rs` (new, `#[path]` module), `src/repository/closure/mod.rs` (the public request and result types), the observation shared with the direct close where reused.

**Interfaces:**
- Produces, in `crate::repository`:
  - `pub struct PullRequestClosureRequest { pub root: PathBuf, pub operation_id: OperationId, pub item_id: ItemId, pub action: PullRequestClosureAction, pub expected: [u8; 32], pub approval: Option<HostApproval> }` and `pub enum PullRequestClosureAction { Record, Withdraw, NewBranch }`;
  - `pub fn preview_pull_request_closure(&self, root: &Path, item_id: &ItemId, action: Option<PullRequestClosureAction>) -> Result<PullRequestClosurePreflight, RepositoryError>` (`None` lets prepare choose `Record` or `NewBranch`, design Part D's table);
  - `pub fn close_by_pull_request<P: SessionCredentialProvider>(&self, request: PullRequestClosureRequest, session: &mut SessionCredentials<P>, observer: &mut dyn SynchronizationObserver) -> Result<PullRequestClosureResult, PullRequestClosureError>`, calling `fn close_by_pull_request_at(…, now: OffsetDateTime, io: &mut impl PullRequestIo)`;
  - `pub(in super::super) trait PullRequestIo { fn synchronize(&mut self, service: &RepositoryService, request: SynchronizeRemoteRequest, observer: &mut dyn SynchronizationObserver) -> Result<SynchronizationResult, SynchronizationError>; }` with a production implementation calling `synchronize_remote_observed` with `MERGING`, and a test implementation in `pull_request_tests.rs` that refuses a primary target, completes a context target by writing its row, its tracking ref and, when given, a local bare repository's branch, and can be told to fail one call with a given error.
- Exact field names may follow the code; the design checkpoint records any change.

- [ ] **Step 1:** Merge `origin/main` if it moved.
- [ ] **Step 2: Write the failing tests first**, one per row of design D1 to D16 that the library can reach (D16 is Task 9's), named `d<n>_<what>`, in `pull_request_tests.rs`, with a fixture built like `closure/test_support.rs`'s (a pull-request configuration, an unreachable SSH remote, hand-written tracking refs) and the test publisher. Every test ends by asserting that the local primary's and the tracking primary's OIDs are those the test set, and that the publisher saw no primary target. D9 includes the CLI's retry being a second call of the same request. D10 and D11 use the failure points the runner exposes for tests, as `closure/run_tests.rs` `exit_at` does.
- [ ] **Step 3: Run each, expect FAIL**, by exact name under `repository::closure::pull_request::tests::`. Record the red runs.
- [ ] **Step 4: Design checkpoint.** Before implementing, the implementer writes in its report: where prepare chooses the action and which blockers it reads; the digest's encoding; each step's row update and when it happens relative to the commit and the ref move; how a retry finds its step; how the unique unfinished index is met by a second request; how `request cancel` ends the row (D13); and which existing functions it reuses for the commit, the move and the identity check. The orchestrator checks them against D1 to D16 and the source, and records the result in the ledger, before Step 5.
- [ ] **Step 5: Implement** as checked.
- [ ] **Step 6: Run** the tests of Step 2 by exact name. Expected: PASS.
- [ ] **Step 7: Commit** "feat(closure): the pull-request close". The orchestrator adds the library checkpoint comment.

## Task 6: The Runner, Withdraw (Design Part E)

**Files:** `src/repository/closure/pull_request.rs`, `pull_request_tests.rs`.

- [ ] **Step 1: Write the failing tests** `w1_…` to `w7_…` for design W1 to W7. W6 runs the remote merge's stage function on a base with a closure, ours withdrawn and theirs unchanged, and asserts a clean result without the closure; its conflicting variant uses Task 2's validator. Each ends with Task 5's primary assertions.
- [ ] **Step 2: Run, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Implement** the `withdraw` action: prepare's `already_applied`, the digest, step 3's checks and the withdraw commit `Withdraw closure of ticket <ULID>`.
- [ ] **Step 4: Run** the tests of Step 1 and Task 5's tests by exact name. Expected: PASS.
- [ ] **Step 5: Commit** "feat(closure): withdraw a provisional closure".

## Task 7: The Runner, The New Branch (Design Part F; Test-First With A Design Checkpoint)

**Files:** `src/repository/closure/pull_request.rs`, `pull_request_tests.rs`, `src/repository/context_retirement/proof.rs` (`pub(in super::super) fn unheld_entries(repository: &git2::Repository, local: git2::Oid, primary: git2::Oid) -> Result<Vec<UnheldEntry>, ProofRefusal>` with `UnheldEntry { path: Vec<u8>, change: PathChange }`), `src/repository.rs` (a function that creates a ticket's context branch at a given commit and adds its worktree, beside `prepare_context_unlocked`).

- [ ] **Step 1: Write the failing tests:**
  - `unheld_entries_against_the_primary` in `src/repository/context_retirement/tests.rs`, a table: a branch whose changes the primary holds exactly gives none; a squash after which the primary changed one of the paths gives that path `modified`; a path the branch added that the primary lacks gives `added`; a path the branch deleted that the primary keeps gives `deleted`; unrelated histories give `UnrelatedHistory`.
  - `f1_…` to `f9_…` for design F1 to F9 in `pull_request_tests.rs`, each ending with Task 5's primary assertions; F1 and F3 also assert that the new branch's only commit over `P` changes only the ticket's file, and that nothing on the test publisher's remote side was deleted.
- [ ] **Step 2: Run each, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Design checkpoint.** Before implementing, the implementer writes: the cleanup functions it calls and their order; how `retire_context`'s evidence is built when the tracking ref is already absent; the new branch function's preconditions and what it registers; the step updates around the retirement, the commit, the branch and the worktree; and the retry from each step. The orchestrator checks them against F1 to F9 and the source, and records the result.
- [ ] **Step 4: Implement** as checked.
- [ ] **Step 5: Run** the tests of Step 1 and Task 5's and Task 6's tests by exact name. Expected: PASS.
- [ ] **Step 6: Commit** "feat(closure): close through a new branch". The orchestrator adds the checkpoint comment.

## Task 8: Two Clones (Design Part G)

**Files:** `src/repository/integration/validate.rs` (the pure `settle_closure_only_conflict(repository, base, ours, theirs, primary, item) -> Result<Option<git2::Oid>, …>`, returning the merged tree), `src/repository/remote/sync.rs` (`integrate_divergence`'s Context stage calls it before installing a conflict), `src/repository/remote/sync_tests.rs` (the `closure_conflict_classification` module).

- [ ] **Step 1: Write the failing tests:**
  - `closure_only_conflicts_settle_with_the_remote_closure`, a table over design G1 to G6 on the pure function: G1 gives a tree whose ticket holds theirs' closure and ours' other changes; G2 and G3 give none; G4 to G6 are clean merges that never reach it (asserted through `prepare_clean_merge`).
  - `a_context_stage_settles_a_closure_only_conflict`: one synchronization-level row, with hand-written tracking refs as the module's existing tests do, asserting a clean stage commit and no installed conflict.
  - `a_proven_merge_merges_the_primary_in_cleanly` (design Part D's last paragraph): a branch the primary holds by ancestry and one it holds by the Folded proof each merge with the primary cleanly to the primary's tree.
- [ ] **Step 2: Run, expect FAIL**, by exact name (the proven-merge row is exempt if it passes first: it pins existing behaviour; record that). Record the red runs.
- [ ] **Step 3: Implement** Part G.
- [ ] **Step 4: Run** the new tests and the whole `closure_conflict_classification` module by its path. Expected: PASS.
- [ ] **Step 5: Commit** "feat(sync): settle a closure-only conflict with the published closure".

## Task 9: The Bindings, The Verb And The CLI (Design Part H)

**Files:**
- Modify: `src/repository/mutation/command.rs` (`TicketWithdrawClosure` in every method), `mutation/mod.rs` (`Mutation::TicketWithdrawClosure`, `TicketWithdrawClosureInput { root, id }`), `bind/` (a binding for the runner, and `ticket close`'s hand-off by mode at prepare and by record at re-entry), `dto.rs` (`PullRequestClosureMutationDto` and its tag, the preview variant `PullRequestClosurePreviewDto` and its tag, `from_stored`'s tag-guarded arms, `authority`), `src/repository/recovery.rs` (`LIFECYCLE_COMMANDS`), `read/status.rs` and `read/dto.rs` (`OperationAction::PullRequestClose`), `bind/resume.rs` (`original_request_required` for it)
- Create: `schemas/v1/pull_request_closure_mutation.schema.json`, `schemas/v1/pull_request_closure_preview.schema.json`; modify `operation.schema.json`
- Modify: `src/bin/manyhands-cli/catalog.rs`, `parse.rs`, `dispatch.rs`, `lifecycle.rs`, `human_lifecycle.rs`, `human_admin.rs`; the desktop's arms for the new preview, data and verb
- Test: `tests/cli/lifecycle.rs`, `src/repository/mutation/` replay tests, `human_lifecycle` tests
- Named edits: E-8, E-9 (the remaining enums), E-10 to E-13

**Interfaces:** Consumes Tasks 5 to 7's library API. Produces the data and preview DTOs of design Part H.

- [ ] **Step 1:** Merge `origin/main` if it moved; if Cycle 02 has landed, re-check `PullRequestDto` and `setup_pending`, and if this Cycle lands second, add `setup_pending` to the close and withdraw prepares with a test row.
- [ ] **Step 2: Write the failing tests:**
  - In `tests/cli/lifecycle.rs`, with the committed pull-request configuration and an unreachable SSH remote (the bypass `tests/cli/collaboration_exits.rs` `with_remote` uses): `pull_request_close_prepare_previews_the_record`, `…_previews_an_existing_closure`, `…_previews_the_new_branch` (a context whose remote branch was observed deleted, and one with no context), `withdraw_with_no_closure_is_already_applied`, `withdraw_of_a_closure_the_primary_holds_is_ticket_closed`, and a confirm that stops at the first synchronization with `transport_unavailable` and writes nothing. Each writes its CLI golden.
  - `pull_request_closure_data_reads_back`: the stored data of each action through `from_stored` for `ticket close`, `ticket withdraw-closure` and `operation resume` (D16).
  - `pull_request_closure_lines` in the human tests: each closure line, the publication line, the link's line, and an unheld entry with a control character escaped.
  - `an_unfinished_pull_request_close_is_listed_and_resumed_by_its_request`: `operation list` shows it with `pull_request_close`; `operation resume` answers `original_request_required`.
- [ ] **Step 3: Run, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 4: Design checkpoint** for the binding: how `ticket close` picks the path at prepare and at re-entry, how the runner's two synchronization IDs relate to the request's operation, and the `from_stored` arm order (U-3). The orchestrator checks them against Part D's hand-off and D12 to D16, and records the result.
- [ ] **Step 5: Implement.** Apply E-8, E-9 to E-13. Register the schemas in `DATA_ROOTS` and their enums in `contract_enumerations` with the goldens of Step 2.
- [ ] **Step 6: Run** Step 2's tests, the `command.rs` tests, the inventory, help, conformance and contract tests named by E-9 to E-13, and `every_bound_command_has_a_golden`, each by exact name; then `devenv shell -- cargo check --locked --features desktop-test-support --tests`. Expected: PASS.
- [ ] **Step 7: Commit** "feat(cli): close in pull-request mode and ticket withdraw-closure". The orchestrator adds the checkpoint comment.

## Task 10: The Owner's Lifecycles In The Library (Design P11)

**Files:** `src/repository/closure/pull_request_tests.rs`.

- [ ] **Step 1: Write the tests**, with a local bare repository as the forge, driven by plain `git2` as `context_retirement/acceptance_tests.rs` drives one, the test publisher pushing the ticket's branch to it, and `poll::synthetic_fetch` with a `git2` fetch feeding the poll:
  - `lifecycle_ends_closed_and_retired_for_each_merge_style`: a table over merge commit, squash (one new commit with the branch's tree changes) and fast-forward: record, publish, merge on the forge and delete the branch, poll; the poll answers the ticket closed, retirement succeeds, the link record is gone, and the ticket reads `closed` (A1, A2);
  - `the_recording_clone_reads_closing_until_it_fetches` (A3);
  - `declined_withdrawn_then_recorded_again` (A4);
  - `comments_and_saves_while_provisional` (A5);
  - `two_clones_record_and_the_later_keeps_the_published_closure`, two clones of the same forge (A6);
  - `a_dependent_waits_for_the_primary` (A7);
  - `merged_first_with_the_remote_branch_kept_closes_in_place` (A9) and `merged_first_with_the_remote_branch_gone_closes_through_a_new_branch`, proven and unproven (A8).
- [ ] **Step 2: Run each** by exact name. A test that passes first is recorded as such; each must then be shown able to fail once, by a deliberate stub in the runner (for example, a stub that records the closure on the primary), and the red run recorded.
- [ ] **Step 3: Commit** "test(closure): the pull-request lifecycles".

## Task 11: The SSH Case And The Goldens (Design P12)

**Files:** `tests/mutation_remote.rs` (the case's row in its case table) or its `c5_lifecycle.rs` module, `tests/support/mutation_golden.rs` (`CASES`), `tests/mutation_contract.rs`, the goldens under `tests/fixtures/mutation_v1/`. Named edit: E-7.

- [ ] **Step 1: Write the case** `a_pull_request_close_publishes_only_the_ticket_branch`, with this comment above it: no non-SSH publication remote can be selected, and the property is that only the ticket's branch reaches the server. The case:
  1. commits a pull-request configuration on `w.peer` and pushes it, then runs a `repo sync` in the clone so its primary holds it (Cycle 01 K-2), and records the server's primary tip;
  2. creates and saves a ticket, then `ticket close` (prepare and confirm): `ok`, `pull_request` with the Bitbucket Cloud URL for the fixture's remote or null, the server's ticket branch at the close commit, the server's primary unchanged;
  3. `ticket close` again: `already_closing`, nothing new on the server;
  4. `ticket withdraw-closure`: `ok`, the server's branch at the withdraw commit, the primary unchanged;
  5. `ticket close` again: `ok`, the primary unchanged;
  6. writes the goldens `ticket_close_pull_request_ok`, `ticket_close_already_closing` and `ticket_withdraw_closure_ok`, with commit IDs replaced as existing goldens replace them.
- [ ] **Step 2: Run it red** against a stub that answers `ok` without publishing (step 2's server check then fails): `devenv shell -- cargo test --all-features --locked --test mutation_remote a_pull_request_close_publishes_only_the_ticket_branch -- --exact`. Record the red run.
- [ ] **Step 3:** Register the goldens in `CASES`, generate them with `MANYHANDS_UPDATE_GOLDEN=1` on the same command, read them, and apply E-7.
- [ ] **Step 4: Run** the case again without the update variable, then by exact name both `every_result_code_has_a_*` tests, `every_bound_command_has_a_golden`, the data-root golden test of `tests/mutation_contract.rs`, and `published_enumerations_list_exactly_the_registered_values`. Expected: PASS.
- [ ] **Step 5: Commit** "test(closure): pull-request close end to end". The orchestrator adds the checkpoint comment.

## Task 12: Documents And Skills (Design D-1 To D-8)

- [ ] **Step 1:** Apply D-1 to D-6 and D-8. Each amendment opens with "Amended 2026-11-25 by Wave 06 Cycle 03 (vh-gy8gz):". In Manyhands-managed text, one line per paragraph.
- [ ] **Step 2:** Apply D-7 to `skills/` and `.agents/skills/`, editing each copy where the two differ.
- [ ] **Step 3:** Apply E-15 if D-6 changes the audit's counts.
- [ ] **Step 4: Run** `devenv shell -- cargo test --all-features --locked --test cli skills::`, `… --test cli inventory::`, `… --test cli recovery_tables` and `… --test cli collaboration_exits`. Expected: PASS, more than zero tests each.
- [ ] **Step 5: Commit** "docs: amend the contracts and skills for the pull-request close".

## Task 13: Gates, Review-Ready, Closure

- [ ] **Step 1:** Merge `origin/main` if it moved, then `git merge-tree --write-tree HEAD origin/main`. Expected: no conflicts.
- [ ] **Step 2:** The static gates. Expected: clean.
- [ ] **Step 3: The fast tier**, AGENTS.md's command as written. Expected: PASS. Record the time.
- [ ] **Step 4: The SSH tier**: every SSH-harness target (design Testing), each once as `devenv shell -- cargo test --all-features --locked --test <target>`, and `--lib repository::local_sync::`. Expected: PASS. Record the times.
- [ ] **Step 5:** Update the Cycle document's Acceptance statuses and the ledger. The orchestrator adds the review-ready comment (gates, tiers and times, the SSH case, the named edits applied) and publishes the branch.
- [ ] **Step 6: After review approval only:** commit any owned edit to the ticket's `ticket.md`, save the status as done with `ticket save`, publish with `item sync`, then run `ticket close --prepare` from the main checkout with the key loaded in ssh-agent, show the owner what will merge, publish and be removed, and `--confirm` after the owner's go-ahead. The close performs the merge. Native CI runs after the merge, on main, only when the owner dispatches it.

## Plan Self-Review

| Acceptance or ruling | Task |
| --- | --- |
| A1, A2, A3, A4 | 10 |
| A5 | 2, 10 |
| A6 | 8, 10 |
| A7 | 1, 10 |
| A8 | 7, 10 |
| A9 | 5, 8 (the proven merge), 10 |
| A10 | 7 |
| A11 | 5, 6, 7, 11 |
| A12 | 5, 6, 7 |
| A13 | 2 |
| A14 | 3, 4, 9, 11 |
| A15 | 12 |
| A16 | 13 |
| R-1, R-2, R-3, R-4, R-5, R-21 | 5 |
| R-6, R-7, R-8, R-20 | 1 |
| R-9, R-10, R-11 | 2 |
| R-12 | 8 |
| R-13, R-14, R-23, R-24 | 3 |
| R-15 | 4 |
| R-16 | 6 |
| R-17 | 5, 9 |
| R-18, R-19 | 7 |
| R-22 | 11 |
| R-25 | 6 (W3) |
| Review Focus lines | 1, 2, 5, 7, 10, 11 |

Checked: every design Part has a task (A: 1; B: 2; C: 3; D: 5; E: 6; F: 7; G: 8; H: 4 and 9); the types named in Interfaces match across tasks (`PullRequestClosureRequest`, `PullRequestClosureAction`, `PullRequestIo`, `PullRequestDto`, `UnheldEntry`, `PullRequestClosureMutationDto`, `PullRequestClosurePreviewDto`); no placeholder steps; the plan adds no test target, so no CI list changes.
