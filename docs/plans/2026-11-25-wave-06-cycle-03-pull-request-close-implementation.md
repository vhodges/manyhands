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

**Architecture:** Reads derive `closing` from the two copies the index already stores (`src/repository/read/items.rs`, `graph.rs`). The closed-ticket guard takes the mode, and closure immutability takes the primary side of a ticket context's pinned synchronization stages (`ticket_guard.rs`, `integration/validate.rs`). A clean merge on a ticket's branch keeps the primary's closure, and a closure-only conflict settles itself (`integration/prepare.rs`). A new runner, `src/repository/closure/pull_request.rs`, with its record `pull_request_closures`, follows the direct close's lifecycle rules (child attempts, holds, supersede before a commit) while it runs ordinary context synchronizations around a closure or withdraw commit, or retires an old context and makes a new branch. A registry table records given links. `ticket close`'s binding hands off by mode, a new verb binds withdraw, and the operation and cancellation layers attribute the runner's synchronizations to it.

**Tech Stack:** Rust, `git2`, `rusqlite`, the existing `src/forge.rs`, the SSH test harness for one case.

**Spec:** [Cycle 03 design](2026-11-25-wave-06-cycle-03-pull-request-close-design.md) (revision 3), under the [Cycle document](../Cycles/wave-06-cycle-03-pull-request-close.md) and the [Pull-Request Mode RFC](../RFC/pull-request-mode.md).

Status: **draft** (revision 3).

## Global Constraints

**Precondition.** The owner has approved the three documents, approved the test edits, authorized implementation and chosen the execution method. Task 0 does not start before that.

**Roles.** The orchestrator runs the tasks, or dispatches one implementer per task and then a fresh reviewer, one at a time. It alone writes the ledger and ticket comments and publishes the ticket branch. Implementers never dispatch subagents and never edit ticket, comment or ledger files.

**Where.** Every task runs in `.manyhands/worktrees/01M87683QK8E34EYY8KS4VQG9Y` on `manyhands/ticket/01M87683QK8E34EYY8KS4VQG9Y`. No lane branches, no extra worktrees, no copies of `target/`.

**Cargo.** Every Rust command is `devenv shell -- cargo …` with `--locked`. One cargo command at a time on this machine (memory pressure ends the whole session). Desktop tests run with `--features desktop-test-support --bin manyhands`, on their own.

**Registry.** The registry migration is one-way. Never run this branch's CLI or library against the owner's registry. Any run outside `cargo test` uses an isolated `HOME`, `MANYHANDS_DATA_DIR` and `GIT_CONFIG_GLOBAL` under a temporary root. Ticket operations run only from the up-to-date main checkout, as `devenv shell -- cargo run --locked --bin manyhands-cli -- …`.

**Tests (AGENTS.md Testing Strategy).**

- Between steps, run only the tests the step wrote or changed, each by exact name: `--lib <module path>::<test> -- --exact`, `--bin manyhands-cli <module path>::<test> -- --exact`, or `--test <target> <name> -- --exact`. Task 0 confirms each module path this plan names. Never run a whole module or target between steps.
- Every test command must report more than zero tests run. A run of zero is a failed step.
- Never run a whole SSH-harness target between steps. Stop and narrow any single test command that runs past about 15 minutes.
- Each new test sits at the cheapest layer that catches its regression. The plan adds one SSH case (Task 14), for the reason the design's Testing section gives (decision 11).
- The fast tier and the SSH tier run once, in Task 16; after a fix wave, only the targets whose code or tests changed run again.
- No real waits: the runner takes its clock as a parameter.

**Every new check fails first.** Each new test or assertion is run red once, before its code or against a deliberate stub, and the red run is recorded in the ledger. A row that pins unchanged behaviour (G2 to G6, the strict rows of P5, the direct-mode rows of P4) is exempt, and is recorded as such.

**Test edits.** The design's named edits E-2 to E-21 (E-16 approved by decision 11), and the edits Task 0's sweep finds, under the owner's test-edit approval: each logged in the ledger as E-22 onwards with its reason. An edit that removes, weakens or reverses an assertion's intent stops for the owner.

**Main.** Merge `origin/main` into the ticket branch in Task 0, before Tasks 7, 11 and 16, and whenever main moves. Never rebase. When Cycle 02 (vh-c97se) lands, merge it and re-check design Parts C and H against it (decision 5).

**Publish.** After each task's commit, the orchestrator publishes the branch with `item sync` from the main checkout and checks that `HEAD` equals `origin/manyhands/ticket/01M87683QK8E34EYY8KS4VQG9Y`.

**Commits.** Each ends with the trailer `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Every commit compiles and passes the static gates and the registration tests: a function with no production caller yet is added in the task that calls it, an enum value's exhaustive matches are given arms in the task that adds it, and a schema and its registrations land with its first golden.

**Static gates** (Task 16, and before any commit that touches Rust):

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

**Ticket checkpoints.** The orchestrator adds a comment to vh-gy8gz after Tasks 0, 4, 7, 9, 12 and 14, and at review-ready, each naming the tests that ran. Comments are added from the main checkout with `manyhands-cli comment add --repo /home/vhodges/work/src/manyhands --id 01M87683QK8E34EYY8KS4VQG9Y --input <file> --non-interactive --json --request-id <ULID>`.

## Review Focus

- A closing ticket never unblocks a dependent, in any read, and is never listed by the readiness queries. Tasks 1 and 13 pin it.
- No runner path moves the local primary or targets the primary in a synchronization. Tasks 7 to 9 pin it with the publisher that refuses a primary target, and Task 14 with the server's primary.
- A same-request retry after each crash point repeats no commit and keeps `closed_at`; a cancelled close continues with a new child attempt, or is superseded before its commit. Tasks 7 to 9 pin it.
- An unfinished close holds its ticket against every writer and the poll. Task 7 pins it.
- A hand edit of `closed_at` in the worktree cannot be saved. Task 2 pins it.
- A closure the primary side holds cannot be dropped by any resolution or clean merge, and nothing changes for primary targets and local synchronizations. Tasks 3 and 6 pin it.
- No answer or record holds `closed_by`. Task 11 pins it (E-20).

---

## File Map And Dependency Order

| Task | Files | Depends on |
| --- | --- | --- |
| 0 | the ledger `docs/plans/2026-11-25-wave-06-cycle-03-pull-request-close-execution.md` | approval |
| 1 | `src/repository/read/dto.rs`, `items.rs`, `graph.rs`, the four schemas of Part A, `plan.schema.json`'s documentation, `src/bin/manyhands-cli/parse.rs`, `dispatch.rs`, `catalog.rs`, `human.rs`, the desktop arms; `tests/read_items.rs`, `tests/read_relationships.rs`, `items_tests.rs`, `graph_tests.rs`; E-2, E-3, E-16, E-17 | 0 |
| 2 | `src/repository/ticket_guard.rs` and its callers, the domain save; `ticket_guard_tests.rs`, `tests/mutation_bridges/context.rs`; E-14 (guard part) | 1 |
| 3 | `src/repository/integration/validate.rs`, its remote callers in `remote/sync.rs` and `remote/conflict_read.rs`; `integration/tests.rs`, `sync_tests.rs`; E-14 (validator part) | 0 |
| 4 | `src/repository/discovery.rs` (the migration), `src/repository/closure/pull_request.rs` (the record's DDL), `src/repository/pull_request_links.rs`, `cleanup.rs` `retire_context` and `context_retirement/record.rs` `finalize_index`, `ItemDto.pull_request`, `ClosureDto.publication`, `item.schema.json`, `human.rs`, and `pull_request.schema.json` and `PullRequestDto` if Cycle 02 has not landed; E-4, E-9 (Task 4 part), E-18 | 1 |
| 5 | `src/results.rs`, `src/results_tests.rs`, `mutation/outcome.rs`, the two schema code lists, `PreviewEffect` and its schema, `LifecycleBlocker` and its two schemas, `mutation/way_out.rs` (`tracking`), `human_admin.rs` `effect_phrase`, the desktop's `blocker_words`; E-5, E-6 | 0 |
| 6 | `src/repository/integration/prepare.rs` (R-30 and the settlement), `remote/sync.rs` (`integrate_divergence`), `integration/tests.rs`, `sync_tests.rs` | 3 |
| 7 | `src/repository/closure/pull_request.rs` (the runner, `record`; the hold; supersede; attempts), `pull_request_tests.rs`, the direct close's prepare and confirm (the other path's record and the mode at confirm), the poll and the writers that consult the hold, `pull_request_links.rs` (`upsert_link`) | 2, 4, 5, 6 |
| 8 | the same, `withdraw` | 7 |
| 9 | the same, `new_branch`; `context_retirement/proof.rs` (the unheld entries); the new context-at-commit function in `src/repository.rs`; the retirement's reservation | 7 |
| 10 | `mutation/dto.rs` (the data, the link data, the preview variant, `from_stored`, `authority`), `mutation/way_out.rs`, `bind/sync.rs` (`item sync`'s link), `human_admin.rs` and `desktop/view/confirm.rs` (the preview's arms), `tests/support/mutation.rs`, `tests/support/repair.rs`; E-8 | 7, 8, 9 |
| 11 | `mutation/command.rs`, `mutation/mod.rs`, `bind/` (the close wrapper and the withdraw binding), `recovery.rs` (`LIFECYCLE_COMMANDS`), the CLI (`catalog.rs`, `parse.rs`, `dispatch.rs`, `lifecycle.rs`, `human_lifecycle.rs`, `human_collaboration.rs`, `mutate.rs`), the desktop's verb arms, `docs/RFC/cli-contract.md`'s taxonomy row, `skills/manyhands-overview/references/autonomy.md`; `tests/cli/lifecycle.rs`, `lifecycle_tests.rs`, `tests/mutation_confirmation`, the preview schema; E-9 (its part), E-10 to E-13, E-19 (skills part), E-20 | 10 |
| 12 | `mutation/progress.rs`, `read/status.rs`, `read/dto.rs` (`OperationAction`), `operation.schema.json`, `blocker.schema.json`, `recovery.rs` (`attribute_remote_row`), `remote/reservation.rs` (`abandonable`), `bind/resume.rs`, the desktop's `ACTION_WORDS`; their tests; E-19 (desktop part) | 11 |
| 13 | `src/repository/closure/pull_request_tests.rs` (the lifecycles) | 6 to 9 |
| 14 | `tests/mutation_remote/c5_lifecycle.rs`, `tests/mutation_remote.rs` (the case table), `tests/support/mutation_golden.rs`, `tests/mutation_contract.rs`, the two data schemas, the goldens; E-7, E-9 (its part), E-21 | 12 |
| 15 | `docs/RFC/*`, `docs/research/wave-03-api-audit.md`, `skills/`, `.agents/skills/`; E-15 | 12, 14 |
| 16 | none new | all |

## Task 0: Baseline, References, Ledger

**Files:** create the ledger `docs/plans/2026-11-25-wave-06-cycle-03-pull-request-close-execution.md`.

- [ ] **Step 1:** In the worktree, `git fetch origin`, `git merge origin/main`, `git merge-base --is-ancestor origin/main HEAD`, `git status --short`. Record the base SHA and HEAD before and after in the ledger.
- [ ] **Step 2: Cycle 02.** Check whether `origin/main` holds `schemas/v1/pull_request.schema.json`, `PullRequestDto` and `setup_pending`. Record which. If the object exists in another shape than design Part C's, stop for the owner.
- [ ] **Step 3:** Re-check every name, path and line the design cites (Baseline Facts 1 to 21, Parts A to H) at the new HEAD. Mark each "existing" or "new" in a ledger table with its current file. Confirm the test module paths: `repository::closure::pull_request::tests` (new), `repository::ticket_guard::tests`, `repository::integration::tests`, `repository::remote::sync::tests`, `repository::context_retirement::tests`, `repository::cleanup::tests`, `repository::discovery::tests`, `repository::read::items::tests`, `repository::read::graph::tests`, `results::tests`, and the targets `read_items`, `read_relationships`, `read_contract`, `mutation_contract`, `mutation_bridges`, `mutation_confirmation`, `cli`. List E-16's exact lines.
- [ ] **Step 4: Premise sweep.** Grep for every site of the design's sweep list (Named Edits, last paragraph) and add each hit to the ledger as a named edit E-22 onwards. An edit that reverses an intent is listed for the owner before its task.
- [ ] **Step 5:** Confirm that no CI test list changes: this Cycle adds no test target.
- [ ] **Step 6:** Commit the ledger. The orchestrator publishes the branch and adds the start checkpoint comment.

## Task 1: The Closing State (Design Part A)

**Files:**
- Modify: `src/repository/read/dto.rs` (`ClosureState`, `DependencyState`, `ReadinessReasonCode`, and the documentation Part A names), `src/repository/read/items.rs` (the per-ID primary-copy closure, `closure()` with the shown fields, `Targets::of`, `Targets::stored_node`, `Related::of`, `Target`, `ClosureFilter`, `TicketFilter::matches` including `--ready`/`--blocked`, `read_path`), `src/repository/read/graph.rs` (`TicketNode`, `TicketGraph`'s lookup, `blockers`, the tree and root lines, `ticket_readiness`, the plan's removal and renumbering, the critical path's zero length), `plan.schema.json`'s and `dto.rs`'s documentation of batch numbers
- Modify: `schemas/v1/item.schema.json`, `dependency.schema.json`, `dependency_tree_node.schema.json`, `readiness_reason.schema.json`
- Modify: `src/bin/manyhands-cli/parse.rs`, `dispatch.rs`, `catalog.rs`
- Modify: the desktop's exhaustive matches (design Baseline Fact 5), compile-level arms only
- Test: `tests/read_items.rs`, `tests/read_relationships.rs`, `src/repository/read/items_tests.rs`, `graph_tests.rs`, `tests/cli/content.rs`, `parse_tests.rs`
- Named edits: E-2, E-3, E-16, E-17

**Interfaces:**
- Produces: `ClosureState::Closing` (`"closing"`), `DependencyState::Closing` (`"closing"`), `ReadinessReasonCode::ClosingDependency` (`"closing_dependency"`), `ClosureFilter::Closing`; a private per-ID fact, the closure of the primary's copy (built from the stored rows at rank `PRIMARY_COPY` after the moved-context substitution), that every reader of Part A takes.

- [ ] **Step 1: Write the failing table test** `closure_state_follows_both_copies` in `tests/read_items.rs`, beside `presentation_across_contexts` and with its helpers: one row per line of design Part A's derivation table, asserting `closure.state` and the fields shown in `ticket list`, `ticket show` and `ticket show --path`, and, for a second ticket that depends on it and has it as `parent`, `deps[].state` and `parent.state`.
- [ ] **Step 2: Write the failing readiness test** `a_closing_dependency_blocks_until_the_primary_holds_it` in `tests/read_relationships.rs`, beside `closing_a_dependency_makes_its_dependent_ready_without_the_dependent_being_observed_again` and with its helpers: B depends on A; with A closing, B is `blocked` with one `closing_dependency` reason naming A; `ticket ready`, `ticket blocked`, `ticket list --ready`, `ticket list --blocked`, `ticket plan` and `ticket critical-path` do not list A, the plan's batches are numbered from 1, and `unplannable` does not hold A; `ticket deps` lines read `closing`; with A's closure on the primary's copy too, B is ready and A `closed`. Add `critical_path_gives_closing_tickets_no_length` to `graph_tests.rs`: a chain through a closing ticket that ties another on length keeps the longest chain of open tickets.
- [ ] **Step 3: Write the failing filter test** `closure_filter_closing_and_open` in `tests/read_items.rs`: over an open, a closing and a closed ticket, `Open` gives the first two, `Closing` the second, `Closed` the third, `All` three. Add the `closing` rows of E-3 to `parse_tests.rs` and `tests/cli/content.rs`.
- [ ] **Step 4: Run, expect FAIL**, each by exact name. Record the red runs.
- [ ] **Step 5: Implement** Part A. Apply E-2, E-3, E-16 and E-17; regenerate the three help goldens with `MANYHANDS_UPDATE_GOLDEN=1` for the tests that own them and read the diff.
- [ ] **Step 6: Run** by exact name: the new tests; the edited tests of E-2, E-3, E-16 and E-17; `presentation_across_contexts`; and `published_enumerations_list_exactly_the_registered_values` (`--test read_contract`). Then `devenv shell -- cargo check --locked --features desktop-test-support --tests`. Expected: PASS.
- [ ] **Step 7: Commit** "feat(read): the closing state".

## Task 2: The Guard And The Save Check (Design Part B)

**Files:**
- Modify: `src/repository/ticket_guard.rs` (`require_ticket_open` gains `integration: Integration`), its callers (`src/repository.rs` `refuse_closed_ticket`, `context_release.rs`, `bind/republish.rs`, `bind/resolve_history.rs`), the domain ticket save (`save_ticket_with_effective_config`)
- Test: `src/repository/ticket_guard_tests.rs`, `tests/mutation_bridges/context.rs`
- Named edits: E-14 (the guard part)

- [ ] **Step 1: Write the failing tests:**
  - `the_guard_reads_the_primary_only_in_pull_request_mode` in `ticket_guard_tests.rs`, a table over mode × (closure on the primary, on the branch, on both, on neither) × caller (save, slug, context prepare, comment checkpoint, republish, resolve-history): direct refuses when either holds it; pull-request mode only when the primary holds it.
  - `a_save_cannot_change_the_closure` in `ticket_guard_tests.rs`: in each mode, a worktree file with `closed_at` added by hand, changed, or removed against the head is refused with `invalid_input` and nothing is committed; a save of a closing ticket's title in a pull-request repository succeeds and keeps the head's closure.
  - `a_closing_ticket_takes_comments_and_saves` in `tests/mutation_bridges/context.rs`, beside `prepare_edit_of_a_closed_ticket_is_ticket_closed_and_makes_nothing`: in a pull-request repository (a version 2 configuration with a publication remote on an unreachable URL), `ticket save` of a closing ticket succeeds, and `comment add` checkpoints its comment and is not `ticket_closed` (its publication stops on the transport); with the closure on the primary both answer `ticket_closed`.
- [ ] **Step 2: Run, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Implement** the guard and the save check. Apply E-14's guard part.
- [ ] **Step 4: Run** the new tests and the guard tests E-14 edits, each by exact name. Expected: PASS.
- [ ] **Step 5: Commit** "feat(closure): the guard per mode and the save check".

## Task 3: Immutability In A Ticket Context (Design Part B)

**Files:** `src/repository/integration/validate.rs` (`preserves_item_invariants`, `validates_resolution_sides`, `validates_external_integration` gain the optional primary side), its remote callers in `remote/sync.rs` (a ticket context's Context stage: the pinned window's `primary_oid`; its Primary stage: the step's incoming commit; none for a primary target or window 0), `remote/conflict_read.rs` (the window's `primary_oid` from the `Recorded` step's window number), the local-sync callers (pass none). Tests: `src/repository/integration/tests.rs`, `sync_tests.rs`. Named edits: E-14 (the validator part).

- [ ] **Step 1: Write the failing tests:**
  - `provisional_closures_may_change_primary_ones_may_not` in `src/repository/integration/tests.rs`, a table of `preserves_item_invariants` over (old, new, primary side, the sides' closures): the primary side holding a closure requires exactly it; with the primary side open, a provisional closure dropped, replaced by the local or incoming side's (accepted), replaced by the base's alone (refused); with no primary side, today's rule (strict rows, exempt from fail-first).
  - one row through `validates_resolution_sides` and one through `validates_external_integration` for a ticket context's Context stage with a pinned window, beside `sync_tests.rs`'s existing callers (about 4015 and 11480), and one strict row each for a primary target.
- [ ] **Step 2: Run, expect FAIL** (the changed rows), by exact name. Record the red runs.
- [ ] **Step 3: Implement.** Apply E-14's validator part.
- [ ] **Step 4: Run** the new tests and `all_recorded_ticket_sides_constrain_retained_closure_before_writes` and `retained_candidate_cannot_bypass_all_side_closure_validation` (unchanged), each by exact name. Expected: PASS.
- [ ] **Step 5: Commit** "feat(sync): a ticket context's merge keeps the primary's closure and may change a provisional one".

## Task 4: The Link Record And The Item's Fields (Design Part C)

**Files:**
- Modify: `src/repository/discovery.rs` (`migrate_registry` calls a new `migrate_pull_request_close` after `migrate_pull_request_mode`, creating `pull_request_links` and `pull_request_closures` by checking the schema, marker `wave_06_pull_request_close_v1`)
- Create: `src/repository/closure/pull_request.rs` with the record's DDL only, and `src/repository/pull_request_links.rs` (declared in `src/repository.rs`) with `link_of` and `delete_link` (Task 7 adds `upsert_link`, with its first caller)
- Modify: `src/repository/cleanup.rs` `retire_context` and `src/repository/context_retirement/record.rs` `finalize_index`'s retiring branch (both call `delete_link`), `src/repository/read/dto.rs` (`ItemDto.pull_request`, `ClosureDto.publication`), the `ticket show` and `show --path` builders, `schemas/v1/item.schema.json`, `src/bin/manyhands-cli/human.rs` (`item_show`'s two lines)
- Create if Cycle 02 has not landed: `schemas/v1/pull_request.schema.json` and `PullRequestDto { source, target, forge, url }`
- Test: `src/repository/discovery.rs` tests, `tests/read_items.rs`, `src/repository/cleanup_tests.rs`, the `context_retirement` tests Task 0 confirms
- Named edits: E-4, E-9 (this task's part), E-18

**Interfaces:** Produces `PullRequestDto` (Cycle 02's or this Cycle's), `ItemDto.pull_request: Option<PullRequestDto>`, `ClosureDto.publication: Option<ClosurePublication>` (`pending` or `published`), `link_of(connection, repository_id, kind, item) -> Result<Option<PullRequestDto>, …>` and `delete_link(tx, repository_id, kind, item) -> Result<(), …>`.

- [ ] **Step 1: Write the failing tests:**
  - `pull_request_close_migration_creates_its_tables_once` beside Cycle 01's `pull_request_mode_migration_adds_columns_once_and_refills`: a registry without the tables or the marker migrates to both tables, the unique unfinished index and one marker; a second migrate changes nothing; a registry with the marker but without a table gets it back.
  - `ticket_show_gives_the_link_and_the_publication` in `tests/read_items.rs`: with a link row, `ticket show` gives it and `ticket list` gives null; a document gives null; a closing ticket whose tracking ref holds the closure's commit reads `publication` `published`, one whose tracking ref does not `pending`; without a publication remote it reads null; lists read null.
  - `retire_context_deletes_the_link` in `src/repository/cleanup_tests.rs`, and `a_finalized_retirement_deletes_the_link` beside `finalize_index`'s tests.
- [ ] **Step 2: Run, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Implement** Part C's fields, the tables and the deletion. Apply E-4, E-9's part and E-18: regenerate the item goldens for the tests that own them with `MANYHANDS_UPDATE_GOLDEN=1`, read the diff, and confirm that only `"pull_request": null` and `"publication": null` were added.
- [ ] **Step 4: Run** by exact name: the new tests, the golden tests of E-4, the registry-table test of E-18, and `published_enumerations_list_exactly_the_registered_values`; then `devenv shell -- cargo check --locked --features desktop-test-support --tests`. Expected: PASS.
- [ ] **Step 5: Commit** "feat(index): the link record and the item's pull request". The orchestrator adds the checkpoint comment.

## Task 5: The Code, The Effects And The Blocker (Design Part H)

**Files:** `src/results.rs` (`AlreadyClosing`, class `None`, message "The ticket's branch already holds a closure.", appended after `AlreadyApplied`; the documentation of class `None`), `src/results_tests.rs`, `src/repository/mutation/outcome.rs` (`recovery`'s shared arm), `schemas/v1/envelope.schema.json`, `schemas/v1/request.schema.json`, `src/repository/mutation/dto.rs` (`PreviewEffect` gains `ContextPublished`, `PullRequestLink`, `ClosureWithdrawn`, appended, and `ContextCreated`'s documentation widens; `LifecycleBlocker` gains `RemoteContextUnmaterialized`, appended), `src/repository/mutation/way_out.rs` (the new blocker in `blocked()`'s `tracking` set), `schemas/v1/preview_effect.schema.json` (values and description), `lifecycle_blocked.schema.json` and `lifecycle_mutation.schema.json` (the blocker), `src/bin/manyhands-cli/human_admin.rs` (`effect_phrase`), `src/desktop/view/lifecycle/prepare.rs` (`blocker_words`), `tests/mutation_contract.rs`. Named edits: E-5, E-6.

- [ ] **Step 1: Apply E-5**, so `results::tests` fails. Run `devenv shell -- cargo test --all-features --locked --lib results::tests::result_codes_match_the_design_table_exactly -- --exact` and `… results::tests::result_code_messages_are_fixed -- --exact`. Expected: FAIL.
- [ ] **Step 2: Implement** the code, its recovery arm, the two schema lists, the three effects and their phrases, the blocker, its way out, its desktop words, and the schema descriptions.
- [ ] **Step 3: Apply E-6.** Run by exact name the two `results::tests` of Step 1, the `tests/read_contract.rs` tests that compare the code, effect and blocker enums with `::ALL`, a `way_out` test row for the new blocker, and both `every_result_code_has_a_*` tests of `tests/mutation_contract.rs`. Then `devenv shell -- cargo check --locked --features desktop-test-support --tests`. Expected: PASS.
- [ ] **Step 4: Commit** "feat(results): already_closing and the pull-request preview effects".

## Task 6: The Primary's Closure And Two Clones (Design Part G And R-30)

**Files:** `src/repository/integration/prepare.rs` (a function that, for a ticket context's stage given the stage's fetched primary, rewrites a clean result's closure to the primary's (R-30) and settles a closure-only conflict (Part G), returning `MergePreparation::Clean` or nothing; and the byte helper that replaces a closed ticket's closure fields canonically), `src/repository/remote/sync.rs` (`integrate_divergence` calls it after `prepare_clean_merge`, for `Clean` and `Conflict`, before any conflict is installed), `src/repository/integration/tests.rs`, `src/repository/remote/sync_tests.rs`.

- [ ] **Step 1: Write the failing tests:**
  - `closure_only_conflicts_settle_with_the_remote_closure` in `src/repository/integration/tests.rs`, a table over design G1, G2, G3 and G7 on the pure function: G1 gives a clean merge whose ticket holds theirs' closure and ours' other changes; G2 and G3 give nothing; G7 gives the clean result with the primary's closure.
  - `a_context_stage_settles_a_closure_only_conflict` in `sync_tests.rs`, modelled on the existing `integrate_divergence` callers (about 614 and 5015), with a pinned window: a clean stage commit, no installed conflict, theirs' closure.
  - `a_primary_stage_keeps_the_primary_closure` in `sync_tests.rs`: base with closure `A`, ours withdrawn, the fetched primary holding `A`: the committed result holds `A`.
  - `a_proven_merge_merges_the_primary_in_cleanly` in `sync_tests.rs` (design Part D's last paragraph): a branch the primary holds by ancestry merges as a fast-forward; one whose every changed path equals the primary's tip merges cleanly to the primary's tree. G4 to G6 and this test pin existing behaviour and are exempt from fail-first; record that.
- [ ] **Step 2: Run, expect FAIL** (G1's and G7's tests), by exact name. Record the red runs.
- [ ] **Step 3: Implement** Part G and R-30.
- [ ] **Step 4: Run** the tests of Step 1 by exact name. Expected: PASS.
- [ ] **Step 5: Commit** "feat(sync): keep the primary's closure, and settle a closure-only conflict with the published closure".

## Task 7: The Runner, Record (Design Part D; Test-First With A Design Checkpoint)

**Files:** `src/repository/closure/pull_request.rs` (the runner, its record functions, the hold, supersede, the child attempts), `src/repository/closure/pull_request_tests.rs` (new, `#[path]` module), `src/repository/closure/mod.rs` (the public request and result types), `src/repository/pull_request_links.rs` (`upsert_link`), the direct close's prepare (the blocker for an unfinished runner row) and confirm (`external_change` in a pull-request repository; the insert's check of `pull_request_closures`), the poll's materialize, fast-forward and retire paths and the writers of Part D's hold (each consults the hold).

**Interfaces:**
- Produces, in `crate::repository`:
  - `pub struct PullRequestClosureRequest { pub root: PathBuf, pub operation_id: OperationId, pub item_id: ItemId, pub action: PullRequestClosureAction, pub expected: PullRequestClosureExpectation, pub approval: Option<HostApproval> }`, `pub enum PullRequestClosureAction { Record, Withdraw, NewBranch }`, and `PullRequestClosureExpectation`, whose `Debug` is redacted as `ClosureExpectation`'s;
  - `pub fn preview_pull_request_closure(&self, root: &Path, item_id: &ItemId, verb: PullRequestClosureVerb) -> Result<PullRequestClosurePreflight, RepositoryError>`, where `PullRequestClosureVerb` is `Close` (prepare chooses `Record` or `NewBranch`, design Part D) or `Withdraw`;
  - `pub fn close_by_pull_request<P: SessionCredentialProvider>(&self, request: PullRequestClosureRequest, session: &mut SessionCredentials<P>, observer: &mut dyn RunObserver, cancel: &dyn CancellationProbe) -> Result<PullRequestClosureResult, PullRequestClosureError>`, calling `fn close_by_pull_request_at(…, now: OffsetDateTime, io: &mut impl PullRequestIo)`;
  - `pub(in super::super) fn pull_request_closure_holds(connection, repository_id, item) -> Result<Option<RequestId>, …>`, the hold the poll and the writers consult;
  - `pub(in super::super) trait PullRequestIo { fn synchronize(&mut self, service: &RepositoryService, request: SynchronizeRemoteRequest, observer: &mut dyn SynchronizationObserver) -> Result<SynchronizationResult, SynchronizationError>; }`, with a production implementation calling `synchronize_remote_observed` with `SynchronizationSettings::default()`, and a test implementation in `pull_request_tests.rs` that refuses a primary target, pins a window and runs the stages against a local bare repository's branch when the test gives one (design Testing), completes the row as `FakeIo` does, and can be told to fail one call with a given error or to be cancelled.
- Exact field names may follow the code; the design checkpoint records any change.

- [ ] **Step 1:** Merge `origin/main` if it moved.
- [ ] **Step 2: Write the failing tests first**, one per row of design D1 to D20 and D23 (D21's binding half and D22 are Task 12's), named `d<n>_<what>`, in `pull_request_tests.rs`, with a fixture built like `closure/test_support.rs`'s (a pull-request configuration, an unreachable SSH remote, hand-written tracking refs) and the test publisher. Add one test per row of the answer table after step 4. Every test ends by asserting that the local primary's OID is the one the test set and that the publisher saw no primary target. D12 to D15 use failure points the runner exposes for tests, as `closure/run_tests.rs` `exit_at` does; D15 makes a commit with `git2` between the crash and the retry; D17 cancels a child and then runs the same request; D18 runs a save, a comment, an `item sync` and a poll against an unfinished row.
- [ ] **Step 3: Run each, expect FAIL**, by exact name under `repository::closure::pull_request::tests::`. Record the red runs.
- [ ] **Step 4: Design checkpoint.** Before implementing, the implementer writes in its report: the prepare's situation order and blockers; the digest's encoding; each step's row update and when it happens relative to the commit, the checkout and the ref move; the attempt rule's classification of each child state; supersede and its conditions; how the unique unfinished index answers a second request at prepare and at insert, and the cross-table check; the hold and every place that consults it; what a stop answers, its effects, and its rewritten recovery; how a finished row's answer is stored and rebuilt; and which existing functions it reuses. The orchestrator checks them against D1 to D23, the answer table and the source, and records the result in the ledger, before Step 5.
- [ ] **Step 5: Implement** as checked.
- [ ] **Step 6: Run** the tests of Step 2 by exact name. Expected: PASS.
- [ ] **Step 7: Commit** "feat(closure): the pull-request close". The orchestrator adds the library checkpoint comment.

## Task 8: The Runner, Withdraw (Design Part E)

**Files:** `src/repository/closure/pull_request.rs`, `pull_request_tests.rs`, and the byte helper that removes a ticket's closure fields canonically, beside `closed_ticket_bytes`.

- [ ] **Step 1: Write the failing tests** `w1_…` to `w11_…` for design W1 to W11; W1 runs in both modes; W6 drives a second clone's synchronization through the test publisher's pinned-window path on a base with a closure, ours withdrawn and theirs unchanged, and asserts a clean result without the closure, and its conflicting variant asserts Task 3's rule accepts an external repair that keeps the withdraw; W9 has the forge merge the closure between step 2 and step 4. Each ends with Task 7's primary assertions.
- [ ] **Step 2: Run, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Implement** the `withdraw` action: prepare's `already_applied`, the digest, step 3's order, the withdraw commit `Withdraw closure of ticket <ULID>`, and the check after step 4.
- [ ] **Step 4: Run** the tests of Step 1 by exact name. Expected: PASS.
- [ ] **Step 5: Commit** "feat(closure): withdraw a provisional closure".

## Task 9: The Runner, The New Branch (Design Part F; Test-First With A Design Checkpoint)

**Files:** `src/repository/closure/pull_request.rs`, `pull_request_tests.rs`, `src/repository/context_retirement/proof.rs` (`pub(in super::super) fn unheld_entries(repository: &git2::Repository, local: git2::Oid, primary: git2::Oid) -> Result<Vec<UnheldEntry>, ProofRefusal>` with `UnheldEntry { path: Vec<u8>, change: PathChange }`), `src/repository.rs` (a function that creates a ticket's context branch at a given commit and adds its worktree, beside `prepare_context_unlocked`, without its root and local-primary checks), the reservation of step 2 under `retire_ulid` (R-31, U-1).

- [ ] **Step 1: Write the failing tests:**
  - `unheld_entries_against_the_primary` in `src/repository/context_retirement/tests.rs`, a table: a branch whose changes the primary holds exactly gives none; a squash after which the primary changed one of the paths gives that path `modified`; a path the branch added that the primary lacks gives `added`; a path the branch deleted that the primary keeps gives `deleted`; a rename gives a deletion and an addition; unrelated histories give `UnrelatedHistory` and two merge bases `MultipleMergeBases`.
  - `f1_…` to `f14_…` for design F1 to F14 in `pull_request_tests.rs`, each ending with Task 7's primary assertions; F1 and F3 also assert that the new branch's only commit over `P` changes only the ticket's file, and that nothing on the test publisher's remote side was deleted; F10 has one test per crash point, inside step 2 at each of its steps and after it.
- [ ] **Step 2: Run each, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Design checkpoint.** Before implementing, the implementer writes: the cleanup functions it calls and their order, with each step update; the reservation under `retire_ulid`, how it is taken, held, completed and recovered after a crash, and how `operation list`, `operation abandon` and `operation resume` treat it (U-1; if it cannot be done with the existing transitions, stop for the owner); how `retire_context`'s evidence is built with and without a context; the new branch function's preconditions and what it registers; how the kept closure is re-read from `T`; and the retry from each step. The orchestrator checks them against F1 to F14 and the source, and records the result.
- [ ] **Step 4: Implement** as checked.
- [ ] **Step 5: Run** the tests of Step 1 by exact name. Expected: PASS.
- [ ] **Step 6: Commit** "feat(closure): close through a new branch". The orchestrator adds the checkpoint comment.

## Task 10: The Data, The Preview And `item sync`'s Link (Design Part H)

**Files:** `src/repository/mutation/dto.rs` (`PullRequestClosureMutationDto` and its tag, `PullRequestLinkDataDto` and its tag, `PullRequestClosurePreviewDto` and its tag, `from_stored`'s guarded arms in design Part H's order, `authority`), `src/repository/mutation/way_out.rs` (the runner's way out from its data), `src/repository/mutation/bind/sync.rs` (`item sync`'s link answer and its `upsert_link`), `src/bin/manyhands-cli/human_admin.rs` and `src/desktop/view/confirm.rs` (the preview variant's arms), `tests/support/mutation.rs`, `tests/support/repair.rs`. Named edits: E-8. The three schemas land with their first goldens: the preview's in Task 11, the two data schemas in Task 14 (E-9, E-21).

- [ ] **Step 1: Write the failing tests:**
  - `pull_request_closure_data_reads_back` beside `dto.rs`'s `from_stored` tests: each action's stored data through `from_stored` for `ticket close` and `ticket withdraw-closure`; a stored lifecycle answer for `ticket close` still reads as the lifecycle DTO; a stored synchronization conflict and host stop for both verbs (D21's library half).
  - `item_sync_gives_the_link_of_a_closing_ticket` beside `bind/sync_tests.rs`'s binding tests: a pull-request repository's `item sync` of a closing ticket's context, published or current, answers the tagged link data and upserts the record; of an open ticket, or in direct mode, no data.
  - a `way_out` row: a runner refusal with `remote_context_unmaterialized` gives `PollOnce`.
- [ ] **Step 2: Run, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Implement.** Apply E-8.
- [ ] **Step 4: Run** the tests of Step 1 by exact name; then `devenv shell -- cargo check --locked --features desktop-test-support --tests`. Expected: PASS.
- [ ] **Step 5: Commit** "feat(mutation): the pull-request closure data and item sync's link".

## Task 11: The Bindings, The Verb And The CLI (Design Parts D, E And H)

**Files:**
- Modify: `src/repository/mutation/command.rs` (`TicketWithdrawClosure` in every method), `mutation/mod.rs` (`Mutation::TicketWithdrawClosure`, `TicketWithdrawClosureInput { root, id }`), `bind/` (the `ticket close` wrapper that dispatches by mode at prepare and at confirm and by record at re-entry, and the withdraw binding), `src/repository/recovery.rs` (`LIFECYCLE_COMMANDS` gains the verb, for its author display)
- Modify: `src/bin/manyhands-cli/catalog.rs` (delivered `C5`), `parse.rs`, `dispatch.rs`, `lifecycle.rs`, `human_lifecycle.rs`, `human_collaboration.rs` (`item sync`'s link line), `mutate.rs` (the retry guard reads the runner's blockers); the desktop's verb arms
- Modify: `docs/RFC/cli-contract.md` (the taxonomy row only, for the inventory test), `skills/manyhands-overview/references/autonomy.md` (the verb in `ticket close`'s column)
- Test: `tests/cli/lifecycle.rs`, `src/bin/manyhands-cli/lifecycle_tests.rs`, `tests/mutation_confirmation`
- Named edits: E-10 to E-13, E-19 (skills part), E-20

- [ ] **Step 1:** Merge `origin/main` if it moved. If Cycle 02 has landed, re-check `PullRequestDto` and `setup_pending`; if this Cycle lands second, add `setup_pending` to the close and withdraw prepares, after the ticket blockers, with a test row.
- [ ] **Step 2: Write the failing tests:**
  - In `tests/cli/lifecycle.rs`, with a committed version 2 pull-request configuration and an unreachable SSH remote (written as `tests/cli/collaboration_exits.rs`'s `Fixture::with_remote` writes one; "observed deleted" by a `remote_context_states` row, as `closure/observe_tests.rs` writes one): `pull_request_close_prepare_previews_the_record`, which writes the preview golden; `…_previews_an_existing_closure` and `…_previews_the_new_branch` (a context whose remote branch was observed deleted, and one with no context), asserting fields without goldens; `withdraw_of_a_closure_the_primary_holds_is_ticket_closed`; and `pull_request_close_stops_at_the_first_synchronization`, with a selected key and a closed port as `collaboration_exits.rs` makes them, answering `transport_unavailable` with nothing written. Register each golden in `CLI_CASES`.
  - In `tests/mutation_confirmation`, the new verb's conformance row (E-12), and `withdraw_with_no_closure_is_already_applied`, which writes the `mutation_v1` golden `ticket_withdraw_closure_already_applied` so that `every_bound_command_has_a_golden` holds from this task.
  - `pull_request_closure_lines` in `lifecycle_tests.rs`: each closure line, the publication line, the link's line, and an unheld entry with a control character escaped.
  - E-20's pull-request-mode part.
- [ ] **Step 3: Run, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 4: Design checkpoint** for the binding: how the wrapper picks the path at prepare, at confirm and at re-entry, how the runner's child IDs relate to the request's operation, and the order of the `from_stored` arms. The orchestrator checks them against Part D's hand-off, and records the result.
- [ ] **Step 5: Implement.** Apply E-10 to E-13, E-19's skills part and E-20. Add `schemas/v1/pull_request_closure_preview.schema.json`, reached as `lifecycle_preview.schema.json` is, with its enums registered (E-9) and its golden.
- [ ] **Step 6: Run** by exact name Step 2's tests, the `command.rs` tests E-10 edits, the inventory, help, catalog, conformance, skills and contract tests E-11 to E-13 and E-19 name, `every_bound_command_has_a_golden` and `every_schema_is_reachable_from_a_published_root`; then `devenv shell -- cargo check --locked --features desktop-test-support --tests`. Expected: PASS.
- [ ] **Step 7: Commit** "feat(cli): close in pull-request mode and ticket withdraw-closure".

## Task 12: Operations, Cancellation And Resume (Design Part D And R-4)

**Files:** `src/repository/mutation/progress.rs` (`asks_a_parent` and `cancel_parent` reach a runner row by record, for both verbs, setting `cancel_requested`), `src/repository/read/status.rs` (the runner's rows listed with `pull_request_close`; its children hidden), `src/repository/read/dto.rs` (`OperationAction::PullRequestClose`), `schemas/v1/operation.schema.json`, `schemas/v1/blocker.schema.json`, `src/repository/recovery.rs` (`attribute_remote_row` attributes the runner's children), `src/repository/remote/reservation.rs` (`abandonable` excludes them), `src/repository/mutation/bind/resume.rs` (a runner row or its child: `original_request_required`), the desktop's `ACTION_WORDS` and its copy test (E-19's desktop part); their tests.

- [ ] **Step 1: Write the failing tests:**
  - `d17_cancel_leaves_the_close_for_its_request`: `request cancel` of a stopped close answers `cancelled`, the row stays unfinished with `cancel_requested`, and the same request continues it with a new child attempt (D17's binding half).
  - `d22_an_unfinished_close_is_listed_and_continued_by_its_request`: `operation list` shows the row with `pull_request_close` and the request, and no child on its own; `operation resume` of the row and of each child answers `original_request_required`; `operation abandon` of a child is refused (D22).
  - `d21_a_finished_close_replays`: the same request after `ok` answers the stored envelope (D21's binding half).
- [ ] **Step 2: Run, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Implement**, following the comment bindings' guards.
- [ ] **Step 4: Run** the tests of Step 1 by exact name, `published_enumerations_list_exactly_the_registered_values`, and the desktop's copy test; then `devenv shell -- cargo check --locked --features desktop-test-support --tests`. Expected: PASS.
- [ ] **Step 5: Commit** "feat(operations): attribute the pull-request close". The orchestrator adds the checkpoint comment.

## Task 13: The Owner's Lifecycles In The Library (Design P11)

**Files:** `src/repository/closure/pull_request_tests.rs`.

- [ ] **Step 1: Write the tests**, with a local bare repository as the forge, driven by plain `git2` as `context_retirement/acceptance_tests.rs` drives one, the test publisher pushing the ticket's branch to it and running the stages with a pinned window for merges, and `poll::synthetic_fetch` with a `git2` fetch feeding the poll:
  - `lifecycle_ends_closed_and_retired_for_each_merge_style`: a table over merge commit, squash (one new commit with the branch's tree changes) and fast-forward: record, publish, merge on the forge and delete the branch, poll; the poll answers the ticket closed, retirement succeeds, the link record is gone, and the ticket reads `closed` (A1, A2);
  - `the_recording_clone_reads_closing_until_it_fetches` (A3);
  - `declined_withdrawn_then_recorded_again` (A4);
  - `comments_and_saves_while_provisional` (A5);
  - `two_clones_record_and_the_later_keeps_the_published_closure`, two clones of the same forge (A6);
  - `a_dependent_waits_for_the_primary` (A7);
  - `merged_first_with_the_remote_branch_kept_closes_in_place` (A9) and `merged_first_with_the_remote_branch_gone_closes_through_a_new_branch`, proven and unproven (A8);
  - `another_clone_reads_closing_after_its_poll`: a second clone's poll materializes the branch and reads `closing`.
- [ ] **Step 2: Run each** by exact name. A test that passes first is recorded as such; each must then be shown able to fail once, by a deliberate stub in the runner (for example, one that records the closure on the primary), and the red run recorded.
- [ ] **Step 3: Commit** "test(closure): the pull-request lifecycles".

## Task 14: The SSH Case And The Goldens (Design P12)

**Files:** `tests/mutation_remote/c5_lifecycle.rs` (the case, registered in `tests/mutation_remote.rs`'s case table beside the C5 lifecycle cases), `tests/support/mutation_golden.rs` (`CASES`), `tests/mutation_contract.rs`, the goldens under `tests/fixtures/mutation_v1/`. Named edits: E-7, E-21.

- [ ] **Step 1: Write the case** `a_pull_request_close_publishes_only_the_ticket_branch`, with this comment above it: no non-SSH publication remote can be selected, and the property is that only the ticket's branch reaches the server (decision 11). The case:
  1. commits a pull-request configuration with `[forge] pull_request_url = "https://forge.example/new?from={source}&to={target}"` on `w.peer` and pushes it, then runs a `repo sync` in the clone so its primary holds it (Cycle 01 K-2), and records the server's primary tip;
  2. creates and saves a ticket, then `ticket close` (prepare and confirm): `ok`, `pull_request.url` `https://forge.example/new?from=manyhands/ticket/<ULID>&to=main`, the server's ticket branch at the close commit, the server's primary unchanged;
  3. `ticket close` again: `already_closing`, nothing new on the server;
  4. `item sync` of the ticket: the tagged link data;
  5. `ticket withdraw-closure`: `ok`, the server's branch at the withdraw commit, the primary unchanged;
  6. `ticket close` again: `ok`, and the same request again: the identical envelope (the conformance rules E-12 exempts);
  7. writes the goldens `ticket_close_pull_request_ok`, `ticket_close_already_closing`, `ticket_withdraw_closure_ok` and `item_sync_pull_request_link`, with commit IDs replaced as existing goldens replace them.
- [ ] **Step 2: Run it red** against a stub that answers `ok` without publishing (step 2's server check then fails): `devenv shell -- cargo test --all-features --locked --test mutation_remote a_pull_request_close_publishes_only_the_ticket_branch -- --exact`. Record the red run.
- [ ] **Step 3:** Register the goldens in `CASES`, add `schemas/v1/pull_request_closure_mutation.schema.json` and `pull_request_link.schema.json` with their enums registered (E-9) and in `DATA_ROOTS` (E-21), generate the goldens with `MANYHANDS_UPDATE_GOLDEN=1` on the same command, read them, and apply E-7.
- [ ] **Step 4: Run** the case again without the update variable, then by exact name both `every_result_code_has_a_*` tests, `every_bound_command_has_a_golden`, `every_schema_is_reachable_from_a_published_root`, `every_data_root_describes_the_data_of_a_golden`, and `published_enumerations_list_exactly_the_registered_values`. Expected: PASS.
- [ ] **Step 5: Commit** "test(closure): pull-request close end to end". The orchestrator adds the checkpoint comment.

## Task 15: Documents And Skills (Design D-1 To D-8)

- [ ] **Step 1:** Apply D-1 (the rest of the CLI contract, after Task 11's taxonomy row) to D-6 and D-8. Each amendment opens with "Amended 2026-11-25 by Wave 06 Cycle 03 (vh-gy8gz):". In Manyhands-managed text, one line per paragraph.
- [ ] **Step 2:** Apply D-7 to `skills/` and `.agents/skills/`, editing each copy where the two differ.
- [ ] **Step 3:** Apply E-15: the recovery-table constants and the withdraw rows' goldens `check_golden` needs (written from the tests that already produce them, or listed with their reason), and the audit's counts.
- [ ] **Step 4: Run** by exact name the tests in `tests/cli/skills.rs`, `inventory.rs`, `recovery_tables.rs` and `collaboration_exits.rs` that read the amended documents (Task 0's sweep names them). Expected: PASS, more than zero tests each.
- [ ] **Step 5: Commit** "docs: amend the contracts and skills for the pull-request close".

## Task 16: Gates, Review-Ready, Closure

- [ ] **Step 1:** Merge `origin/main` if it moved; if Cycle 02 landed meanwhile, apply Task 11 Step 1's `setup_pending` rule. Then `git merge-tree --write-tree HEAD origin/main`. Expected: no conflicts.
- [ ] **Step 2:** The static gates. Expected: clean.
- [ ] **Step 3: The fast tier**, AGENTS.md's command as written. Expected: PASS. Record the time.
- [ ] **Step 4: The SSH tier**: every SSH-harness target (design Testing), each once as `devenv shell -- cargo test --all-features --locked --test <target>`, and `--lib repository::local_sync::`. Expected: PASS. Record the times.
- [ ] **Step 5: Final review.** A fresh reviewer (the most capable model) reviews the whole branch against the design; a fix wave follows if needed, and after it only the targets whose code or tests changed run again.
- [ ] **Step 6:** Update the Cycle document's Acceptance statuses and the ledger. The orchestrator adds the review-ready comment (gates, tiers and times, the SSH case, the named edits applied) and publishes the branch.
- [ ] **Step 7: After review approval only:** commit any owned edit to the ticket's `ticket.md`, save the status as done with `ticket save`, publish with `item sync`, then run `ticket close --prepare` from the main checkout with the key loaded in ssh-agent, show the owner what will merge, publish and be removed, and `--confirm` after the owner's go-ahead. The close performs the merge. Then tick Cycle 03's boxes in the Wave 06 document through `manyhands-documents`, with the owner's go-ahead for its save and promotion. Native CI runs after the merge, on main, only when the owner dispatches it.

## Plan Self-Review

| Acceptance or ruling | Task |
| --- | --- |
| A1, A2, A3, A4 | 13 |
| A5 | 2, 13 |
| A6 | 6, 13 |
| A7 | 1, 13 |
| A8 | 9, 13 |
| A9 | 6, 7, 13 |
| A10 | 9 |
| A11 | 7, 8, 9, 14 |
| A12 | 7, 8, 9, 12 |
| A13 | 3, 6 |
| A14 | 4, 5, 10, 11, 12, 14 |
| A15 | 15 |
| A16 | 16 |
| R-1, R-3, R-5, R-17, R-23, R-27 | 7 |
| R-2 | 7, 11 |
| R-4, R-18 | 12 |
| R-6, R-7, R-8, R-9 | 1 |
| R-10, R-11 | 2 |
| R-12 | 3 |
| R-13, R-30 | 6 |
| R-14, R-15, R-26 | 4 |
| R-16 | 5 |
| R-19, R-20, R-21, R-31 | 9 |
| R-22 | 1, 5, 10, 11, 12 |
| R-24, R-25 | 11, 14 |
| R-28 | 11 |
| R-29 | 10 |
| R-32 | 8 |
| Review Focus lines | 1, 2, 3, 6, 7, 8, 9, 11, 13, 14 |

Checked: every design Part has a task (A: 1; B: 2 and 3; C: 4; D: 7, 11, 12; E: 8; F: 9; G and R-30: 6; H: 5, 10, 11, 12); the types named in Interfaces match across tasks (`PullRequestClosureRequest`, `PullRequestClosureAction`, `PullRequestClosureVerb`, `PullRequestClosureExpectation`, `PullRequestIo`, `pull_request_closure_holds`, `PullRequestDto`, `UnheldEntry`, `PullRequestClosureMutationDto`, `PullRequestLinkDataDto`, `PullRequestClosurePreviewDto`); no placeholder steps; the plan adds no test target, so no CI list changes.
