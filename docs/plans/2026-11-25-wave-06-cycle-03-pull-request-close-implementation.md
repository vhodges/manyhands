---
title: "Wave 06 Cycle 03 Implementation Plan: Ticket Close In Pull-Request Mode, The Closing State And Withdraw"
date: 2026-11-25
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M899G2Y5Q4H3SQSSJVFTCR3N"
---

# Wave 06 Cycle 03 Implementation Plan: Ticket Close In Pull-Request Mode, The Closing State And Withdraw

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In a pull-request repository, close a ticket by recording its closure on the ticket's branch, publishing the branch and giving the pull request link; show such a closure as `closing` until the primary holds it; add `ticket withdraw-closure`; settle two clones' closures; close a merged-first ticket whose remote branch is gone, or a ticket with no context, through a new branch holding only the closure.

**Architecture:** Reads derive `closing` from the two copies the index already stores (`src/repository/read/items.rs`, `graph.rs`). The closed-ticket guard takes the mode, and closure immutability takes the primary side of a ticket context's pinned synchronization stages (`ticket_guard.rs`, `integration/validate.rs`). A ticket context's synchronization never publishes a head lacking the primary's closure, and a closure-only conflict settles itself (`integration/prepare.rs`, `remote/sync.rs`). The pull-request close, withdraw and new branch are branch records of the existing `ticket_closures` lifecycle (decision 13): the record gains its path, the runner (`promotion/run.rs`) branches on it to record on the branch and publish the branch instead of integrating and cleaning up, and the lifecycle's children, fences, cancellation, abandon, attribution and listing serve them. A registry table records given links. A new verb binds withdraw.

**Tech Stack:** Rust, `git2`, `rusqlite`, the existing `src/forge.rs`, the SSH test harness for one case.

**Spec:** [Cycle 03 design](2026-11-25-wave-06-cycle-03-pull-request-close-design.md) (revision 5), under the [Cycle document](../Cycles/wave-06-cycle-03-pull-request-close.md) and the [Pull-Request Mode RFC](../RFC/pull-request-mode.md).

Status: **approved** by the owner on 2026-11-25, at revision 5 (Cycle document decision 14). Execution: subagent-driven, with Sonnet 5.5 at high effort as the implementer.

## Global Constraints

**Precondition.** The owner has approved the three documents, approved the test edits, authorized implementation and chosen the execution method. Task 0 does not start before that.

**Roles.** The orchestrator runs the tasks, or dispatches one implementer per task and then a fresh reviewer, one at a time. It alone writes the ledger and ticket comments and publishes the ticket branch. Implementers never dispatch subagents and never edit ticket, comment or ledger files.

**Where.** Every task runs in `.manyhands/worktrees/01M87683QK8E34EYY8KS4VQG9Y` on `manyhands/ticket/01M87683QK8E34EYY8KS4VQG9Y`. No lane branches, no extra worktrees, no copies of `target/`.

**Cargo.** Every Rust command is `devenv shell -- cargo …` with `--locked`. One cargo command at a time on this machine (memory pressure ends the whole session). Desktop tests run with `--features desktop-test-support --bin manyhands`, on their own.

**Registry.** The registry migrations are one-way. Never run this branch's CLI or library against the owner's registry. Any run outside `cargo test` uses an isolated `HOME`, `MANYHANDS_DATA_DIR` and `GIT_CONFIG_GLOBAL` under a temporary root. Ticket operations run only from the up-to-date main checkout, as `devenv shell -- cargo run --locked --bin manyhands-cli -- …`.

**Tests (AGENTS.md Testing Strategy).**

- Between steps, run only the tests the step wrote or changed, each by exact name: `--lib <module path>::<test> -- --exact`, `--bin manyhands-cli <module path>::<test> -- --exact`, or `--test <target> <name> -- --exact`. Task 0 confirms each module path this plan names. Never run a whole module or target between steps.
- Every test command must report more than zero tests run. A run of zero is a failed step.
- Never run a whole SSH-harness target between steps. Stop and narrow any single test command that runs past about 15 minutes.
- Each new test sits at the cheapest layer that catches its regression. The plan adds one SSH case (Task 14), for the reason the design's Testing section gives (decision 11).
- The fast tier and the SSH tier run once, in Task 16; after a fix wave, only the targets whose code or tests changed run again.
- No real waits: the runner takes its clock as a parameter.

**Every new check fails first.** Each new test or assertion is run red once, before its code or against a deliberate stub, and the red run is recorded in the ledger. A row that pins unchanged behaviour (G2 to G6, the strict rows of P5, the direct-mode rows of P4 and P8) is exempt, and is recorded as such.

**Test edits.** The design's named edits E-2 to E-23 (E-16 approved by decision 11), and the edits Task 0's sweep finds, under the owner's test-edit approval: each logged in the ledger as E-24 onwards with its reason. An edit that removes, weakens or reverses an assertion's intent stops for the owner.

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
- No branch record moves the local primary or synchronizes the primary. Tasks 7 to 9 pin it with the publisher that refuses a primary target, and Task 14 with the server's primary.
- Every direct record behaves as before: the path branches read `direct`, `close` for every existing row. Task 7's direct rows and the `ticket_closure` and `document_promotion` targets pin it.
- A same-request retry after each crash point repeats no commit and keeps `closed_at`. Tasks 7 to 9 pin it.
- An unfinished branch record holds its ticket against every writer, the poll and every other synchronization, and lets its own publication child through. Task 7 pins it.
- A hand edit of `closed_at` in the worktree cannot be saved. Task 2 pins it.
- A closure the primary side holds cannot be dropped by any resolution, clean merge or fast-forward publication, and nothing changes for primary targets and local synchronizations. Tasks 3 and 6 pin it.
- No answer or record holds `closed_by`. Task 11 pins it (E-20).

---

## File Map And Dependency Order

| Task | Files | Depends on |
| --- | --- | --- |
| 0 | the ledger `docs/plans/2026-11-25-wave-06-cycle-03-pull-request-close-execution.md` | approval |
| 1 | `src/repository/read/dto.rs`, `items.rs`, `graph.rs`, the four schemas of Part A, `plan.schema.json`'s documentation, `src/bin/manyhands-cli/parse.rs`, `dispatch.rs`, `catalog.rs`, `human.rs`, the desktop arms; `tests/read_items.rs`, `tests/read_relationships.rs`, `items_tests.rs`, `graph_tests.rs`; E-2, E-3, E-16, E-17 | 0 |
| 2 | `src/repository/ticket_guard.rs` and its callers, the domain save; `ticket_guard_tests.rs`, `tests/mutation_bridges/context.rs`; E-14 (guard part) | 1 |
| 3 | `src/repository/integration/validate.rs`, its remote callers in `remote/sync.rs` and `remote/conflict_read.rs`; `integration/tests.rs`, `sync_tests.rs`; E-14 (validator part) | 0 |
| 4 | `src/repository/discovery.rs` (the links migration), `src/repository/pull_request_links.rs`, `cleanup.rs` `retire_context` and `context_retirement/record.rs` `finalize_index`, `ItemDto.pull_request`, `ClosureDto.publication`, `item.schema.json`, `human.rs`, and `pull_request.schema.json` and `PullRequestDto` if Cycle 02 has not landed; E-4, E-9 (Task 4 part), E-18 | 1 |
| 5 | `src/results.rs`, `src/results_tests.rs`, `mutation/outcome.rs`, the two schema code lists, `PreviewEffect` and its schema, `LifecycleBlocker` and its two schemas, `mutation/way_out.rs` (`tracking`), `human_admin.rs` `effect_phrase`, the desktop's `blocker_words`; E-5, E-6 | 0 |
| 6 | `src/repository/integration/prepare.rs` (R-30 and the settlement), `remote/sync.rs` (`integrate_divergence` and the push refusal), `integration/tests.rs`, `sync_tests.rs` | 3 |
| 7 | `src/repository/closure/record.rs` (the rebuild, columns, steps, CHECKs, `permits`, `validate`, the fence's range), `closure/observe.rs` (the branch observation and digest), `closure/checkpoint.rs` (the hook without integration), `closure/lifecycle.rs`, `closure/mod.rs`, `promotion/run.rs`, `promotion/lifecycle.rs`, `promotion/lifecycle_record.rs` (`ChildRole::Publication`, the second slot's readers), `remote/reservation.rs` (`closure_holds_context`'s exemption), `remote/poll.rs` (the poll's retirement check), the writers' hold (`repository.rs` `refuse_closed_ticket`, `bind/republish.rs`, `bind/resolve_history.rs`, `context retire`), `pull_request_links.rs` (`upsert_link`), `promotion/mod.rs` (`ParentView`), `bind/lifecycle.rs` (`Stages`, `held_by_its_child`), `bind/lifecycle_verb.rs` (`closure_reached`), `bind/mod.rs` (a lifecycle parent's recorded effects); `record_tests.rs`, `run_tests.rs` (a `pull_request` module), `promotion/run_tests.rs` (`FakeIo`); E-22, E-23 | 2, 4, 5, 6 |
| 8 | the same, `withdraw`, and its byte helper | 7 |
| 9 | the same, `new_branch`; `context_retirement/proof.rs` (the unheld entries); the new context-at-commit function in `src/repository.rs`; the retirement's reservation and its release | 7 |
| 10 | `mutation/dto.rs` (the data, the link data, the preview variant, `from_stored`, `authority`), `mutation/way_out.rs`, `bind/sync.rs` (`item sync`'s link), `bind/lifecycle.rs` (the branch shape of `reached_*`, `parent_integrated`, `parent_effects`, `made_ready`), `human_admin.rs` and `desktop/view/confirm.rs` (the preview's arms), `tests/support/mutation.rs`, `tests/support/repair.rs`; E-8 | 7, 8, 9 |
| 11 | `mutation/command.rs`, `mutation/mod.rs`, `bind/lifecycle_verb.rs` (the withdraw verb), `bind/resume.rs` (the rebuilt input by action), `mutation/progress.rs` (the verb in `asks_a_parent` and `cancel_parent`), `recovery.rs` (`LIFECYCLE_COMMANDS`), the CLI (`catalog.rs`, `parse.rs`, `dispatch.rs`, `lifecycle.rs`, `human_lifecycle.rs`, `human_collaboration.rs`, `mutate.rs`), the desktop's verb arms, `docs/RFC/cli-contract.md`'s taxonomy row, `skills/manyhands-overview/references/autonomy.md`, the preview schema; `tests/cli/lifecycle.rs`, `lifecycle_tests.rs`, `tests/mutation_confirmation`; E-9 (its part), E-10 to E-13, E-19, E-20, E-21 (its part) | 10 |
| 12 | `remote/state.rs` (`lifecycle_context_child`, `lifecycle_parent_of`: the second slot), `remote/conflict_read.rs`, `bind/conflict.rs`, `mutation/records.rs`; their tests | 11 |
| 13 | `src/repository/closure/run_tests.rs` (the lifecycles) | 6 to 9 |
| 14 | `tests/mutation_remote/c5_lifecycle.rs`, `tests/mutation_remote.rs` (the case table), `tests/support/mutation_golden.rs`, `tests/mutation_contract.rs`, the two data schemas, the goldens; E-7, E-9 (its part), E-11 (its part), E-21 (its part) | 12 |
| 15 | `docs/RFC/*`, `docs/research/wave-03-api-audit.md`, `skills/`, `.agents/skills/`; E-15 | 12, 14 |
| 16 | none new | all |

## Task 0: Baseline, References, Ledger

**Files:** create the ledger `docs/plans/2026-11-25-wave-06-cycle-03-pull-request-close-execution.md`.

- [ ] **Step 1:** In the worktree, `git fetch origin`, `git merge origin/main`, `git merge-base --is-ancestor origin/main HEAD`, `git status --short`. Record the base SHA and HEAD before and after in the ledger.
- [ ] **Step 2: Cycle 02.** Check whether `origin/main` holds `schemas/v1/pull_request.schema.json`, `PullRequestDto` and `setup_pending`. Record which. If the object exists in another shape than design Part C's, stop for the owner.
- [ ] **Step 3:** Re-check every name, path and line the design cites (Baseline Facts 1 to 23, Parts A to H) at the new HEAD. Mark each "existing" or "new" in a ledger table with its current file. Confirm the test module paths: `repository::closure::run_tests` and its new `pull_request` module, `repository::closure::record` tests, `repository::ticket_guard::tests`, `repository::integration::tests`, `repository::remote::sync::tests`, `repository::remote::reservation::tests`, `repository::context_retirement::tests`, `repository::cleanup::tests`, `repository::discovery::tests`, `repository::read::items::tests`, `repository::read::graph::tests`, `results::tests`, and the targets `read_items`, `read_relationships`, `read_contract`, `mutation_contract`, `mutation_bridges`, `mutation_confirmation`, `cli`. List E-16's exact lines.
- [ ] **Step 4: Premise sweep.** Grep for every site of the design's sweep list (Named Edits, last paragraph) and add each hit to the ledger as a named edit E-24 onwards. An edit that reverses an intent is listed for the owner before its task.
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

**Interfaces:** Produces `ClosureState::Closing` (`"closing"`), `DependencyState::Closing` (`"closing"`), `ReadinessReasonCode::ClosingDependency` (`"closing_dependency"`), `ClosureFilter::Closing`, and a private per-ID fact, the closure of the primary's copy, that every reader of Part A takes.

- [ ] **Step 1: Write the failing table test** `closure_state_follows_both_copies` in `tests/read_items.rs`, beside `presentation_across_contexts` and with its helpers: one row per line of design Part A's derivation table, asserting `closure.state` and the fields shown in `ticket list`, `ticket show` and `ticket show --path`, and, for a second ticket that depends on it and has it as `parent`, `deps[].state` and `parent.state`.
- [ ] **Step 2: Write the failing readiness test** `a_closing_dependency_blocks_until_the_primary_holds_it` in `tests/read_relationships.rs`, beside `closing_a_dependency_makes_its_dependent_ready_without_the_dependent_being_observed_again` and with its helpers: B depends on A; with A closing, B is `blocked` with one `closing_dependency` reason naming A; `ticket ready`, `ticket blocked`, `ticket list --ready`, `ticket list --blocked`, `ticket plan` and `ticket critical-path` do not list A, the plan's batches are numbered from 1, and `unplannable` does not hold A; `ticket deps` lines read `closing`; with A's closure on the primary's copy too, B is ready and A `closed`. Add `critical_path_gives_closing_tickets_no_length` to `graph_tests.rs`.
- [ ] **Step 3: Write the failing filter test** `closure_filter_closing_and_open` in `tests/read_items.rs`. Add the `closing` rows of E-3 to `parse_tests.rs` and `tests/cli/content.rs`.
- [ ] **Step 4: Run, expect FAIL**, each by exact name. Record the red runs.
- [ ] **Step 5: Implement** Part A. Apply E-2, E-3, E-16 and E-17; regenerate the three help goldens with `MANYHANDS_UPDATE_GOLDEN=1` for the tests that own them and read the diff.
- [ ] **Step 6: Run** by exact name: the new tests; the edited tests of E-2, E-3, E-16 and E-17; `presentation_across_contexts`; and `published_enumerations_list_exactly_the_registered_values` (`--test read_contract`). Then `devenv shell -- cargo check --locked --features desktop-test-support --tests`. Expected: PASS.
- [ ] **Step 7: Commit** "feat(read): the closing state".

## Task 2: The Guard And The Save Check (Design Part B)

**Files:** `src/repository/ticket_guard.rs` (`require_ticket_open` gains `integration: Integration`), its callers (`src/repository.rs` `refuse_closed_ticket`, `context_release.rs`, `bind/republish.rs`, `bind/resolve_history.rs`), the domain ticket save (`save_ticket_with_effective_config`). Tests: `src/repository/ticket_guard_tests.rs`, `tests/mutation_bridges/context.rs`. Named edits: E-14 (the guard part).

- [ ] **Step 1: Write the failing tests:**
  - `the_guard_reads_the_primary_only_in_pull_request_mode` in `ticket_guard_tests.rs`, a table over mode × (closure on the primary, on the branch, on both, on neither) × caller (save, slug, context prepare, comment checkpoint, republish, resolve-history).
  - `a_save_cannot_change_the_closure` in `ticket_guard_tests.rs`: in each mode, a worktree file with `closed_at` added by hand, changed, or removed against the head is refused with `invalid_input` and nothing is committed; a save of a closing ticket's title in a pull-request repository succeeds and keeps the head's closure.
  - `a_closing_ticket_takes_comments_and_saves` in `tests/mutation_bridges/context.rs`, beside `prepare_edit_of_a_closed_ticket_is_ticket_closed_and_makes_nothing`: in a pull-request repository (a version 2 configuration with a publication remote on an unreachable URL), `ticket save` of a closing ticket succeeds, and `comment add` checkpoints its comment and is not `ticket_closed`; with the closure on the primary both answer `ticket_closed`.
- [ ] **Step 2: Run, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Implement** the guard and the save check. Apply E-14's guard part.
- [ ] **Step 4: Run** the new tests and the guard tests E-14 edits, each by exact name. Expected: PASS.
- [ ] **Step 5: Commit** "feat(closure): the guard per mode and the save check".

## Task 3: Immutability In A Ticket Context (Design Part B)

**Files:** `src/repository/integration/validate.rs` (`validates_resolution_sides` and `validates_external_integration` gain the optional primary-side commit and the three-side closure check; `preserves_item_invariants`'s per-side closure check applies only without one), its remote callers in `remote/sync.rs` (a ticket context's Context stage: the pinned window's `primary_oid`; its Primary stage: the step's incoming commit; none for a primary target or window 0), `remote/conflict_read.rs` (the window's `primary_oid` from the `Recorded` step's window number), the local-sync callers (pass none). Tests: `src/repository/integration/tests.rs`, `sync_tests.rs`. Named edits: E-14 (the validator part).

- [ ] **Step 1: Write the failing tests:**
  - `provisional_closures_may_change_primary_ones_may_not` in `src/repository/integration/tests.rs`, a table over (base, local, incoming, primary, result) closures for both validators: the primary holding a closure requires exactly it; with the primary open, a provisional closure dropped, replaced by the local or incoming side's (accepted), replaced by the base's alone (refused); with no primary side, today's rule (strict rows, exempt from fail-first).
  - one row through each validator for a ticket context's Context stage with a pinned window, beside `sync_tests.rs`'s existing callers (about 4015 and 11480), and one strict row each for a primary target.
- [ ] **Step 2: Run, expect FAIL** (the changed rows), by exact name. Record the red runs.
- [ ] **Step 3: Implement.** Apply E-14's validator part.
- [ ] **Step 4: Run** the new tests and `all_recorded_ticket_sides_constrain_retained_closure_before_writes` and `retained_candidate_cannot_bypass_all_side_closure_validation` (unchanged), each by exact name. Expected: PASS.
- [ ] **Step 5: Commit** "feat(sync): a ticket context's merge keeps the primary's closure and may change a provisional one".

## Task 4: The Link Record And The Item's Fields (Design Part C)

**Files:**
- Modify: `src/repository/discovery.rs` (`migrate_registry` calls a new `migrate_pull_request_links` after `migrate_pull_request_mode`, creating `pull_request_links` by checking the schema, marker `wave_06_pull_request_links_v1`)
- Create: `src/repository/pull_request_links.rs` (declared in `src/repository.rs`) with `link_of` and `delete_link` (Task 7 adds `upsert_link`, with its first caller)
- Modify: `src/repository/cleanup.rs` `retire_context` and `src/repository/context_retirement/record.rs` `finalize_index`'s retiring branch (both call `delete_link`), `src/repository/read/dto.rs` (`ItemDto.pull_request`, `ClosureDto.publication`), the `ticket show` and `show --path` builders, `schemas/v1/item.schema.json`, `src/bin/manyhands-cli/human.rs` (`item_show`'s two lines)
- Create if Cycle 02 has not landed: `schemas/v1/pull_request.schema.json` and `PullRequestDto { source, target, forge, url }`
- Test: `src/repository/discovery.rs` tests, `tests/read_items.rs`, `src/repository/cleanup_tests.rs`, the `context_retirement` tests Task 0 confirms
- Named edits: E-4, E-9 (this task's part), E-18

- [ ] **Step 1: Write the failing tests:**
  - `pull_request_links_migration_creates_its_table_once` beside Cycle 01's `pull_request_mode_migration_adds_columns_once_and_refills`.
  - `ticket_show_gives_the_link_and_the_publication` in `tests/read_items.rs`: with a link row, `ticket show` gives it and `ticket list` gives null; a document gives null; in a pull-request repository a closing ticket whose tracking ref's ticket holds the same closure reads `publication` `published`, one whose tracking ref does not `pending`; in direct mode it reads null; lists read null.
  - `retire_context_deletes_the_link` in `src/repository/cleanup_tests.rs`, and `a_finalized_retirement_deletes_the_link` beside `finalize_index`'s tests.
- [ ] **Step 2: Run, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Implement** Part C's fields, the table and the deletion. Apply E-4, E-9's part and E-18: regenerate the item goldens for the tests that own them with `MANYHANDS_UPDATE_GOLDEN=1`, read the diff, and confirm that only `"pull_request": null` and `"publication": null` were added.
- [ ] **Step 4: Run** by exact name: the new tests, the golden tests of E-4, the registry-table test of E-18, and `published_enumerations_list_exactly_the_registered_values`; then `devenv shell -- cargo check --locked --features desktop-test-support --tests`. Expected: PASS.
- [ ] **Step 5: Commit** "feat(index): the link record and the item's pull request". The orchestrator adds the checkpoint comment.

## Task 5: The Code, The Effects And The Blocker (Design Part H)

**Files:** `src/results.rs` (`AlreadyClosing`, class `None`, message "The ticket's branch already holds a closure.", appended after `AlreadyApplied`; the documentation of class `None`), `src/results_tests.rs`, `src/repository/mutation/outcome.rs` (`recovery`'s shared arm), `schemas/v1/envelope.schema.json`, `schemas/v1/request.schema.json`, `src/repository/mutation/dto.rs` (`PreviewEffect` gains `ContextPublished`, `PullRequestLink`, `ClosureWithdrawn`, appended, and `ContextCreated`'s documentation widens; `LifecycleBlocker` gains `RemoteContextUnmaterialized`, appended), `src/repository/mutation/way_out.rs` (the new blocker in `blocked()`'s `tracking` set), `schemas/v1/preview_effect.schema.json`, `lifecycle_blocked.schema.json` and `lifecycle_mutation.schema.json`, `src/bin/manyhands-cli/human_admin.rs` (`effect_phrase`), `src/desktop/view/lifecycle/prepare.rs` (`blocker_words`), `tests/mutation_contract.rs`. Named edits: E-5, E-6.

- [ ] **Step 1: Apply E-5**, so `results::tests` fails. Run `devenv shell -- cargo test --all-features --locked --lib results::tests::result_codes_match_the_design_table_exactly -- --exact` and `… results::tests::result_code_messages_are_fixed -- --exact`. Expected: FAIL.
- [ ] **Step 2: Implement** the code, its recovery arm, the two schema lists, the three effects and their phrases, the blocker, its way out, its desktop words, and the schema descriptions.
- [ ] **Step 3: Apply E-6.** Run by exact name the two `results::tests` of Step 1, the `tests/read_contract.rs` tests that compare the code, effect and blocker enums with `::ALL`, a `way_out` test row for the new blocker, and both `every_result_code_has_a_*` tests of `tests/mutation_contract.rs`. Then `devenv shell -- cargo check --locked --features desktop-test-support --tests`. Expected: PASS.
- [ ] **Step 4: Commit** "feat(results): already_closing and the pull-request preview effects".

## Task 6: The Primary's Closure And Two Clones (Design Part G And R-30)

**Files:** `src/repository/integration/prepare.rs` (a function that, for a ticket context's stage given the stage's fetched primary, rewrites a clean result's closure to the primary's and settles a closure-only conflict, returning `MergePreparation::Clean` or nothing; the byte helper that replaces a closed ticket's closure fields canonically), `src/repository/remote/sync.rs` (`integrate_divergence` calls it after `prepare_clean_merge`; a ticket context's synchronization refuses its push with `ticket_closed`, ending its operation, when no merge was prepared and the head lacks the fetched primary's closure), `src/repository/integration/tests.rs`, `src/repository/remote/sync_tests.rs`.

- [ ] **Step 1: Write the failing tests:**
  - `closure_only_conflicts_settle_with_the_remote_closure` in `src/repository/integration/tests.rs`, a table over design G1 to G7 on the pure function (G4 to G6 as rows that leave the merge as it is, exempt from fail-first).
  - `a_context_stage_settles_a_closure_only_conflict` in `sync_tests.rs`, modelled on the existing `integrate_divergence` callers (about 614 and 5015): a clean stage commit, no installed conflict, theirs' closure.
  - `a_primary_stage_keeps_the_primary_closure` in `sync_tests.rs`: base with closure `A`, ours withdrawn, the fetched primary holding `A`: the committed result holds `A`.
  - `a_fast_forward_that_drops_the_primary_closure_is_not_published` in `sync_tests.rs` (G8): the local branch a withdraw on `C`, the fetched primary at `C`: the push decision answers `ticket_closed`, an `item sync` ends and a lifecycle child is kept; driven through the push decision's seam (Design Checkpoint Obligation 6), since `promotion_test_drive_local_bound` runs only `integrate_divergence`.
  - `a_proven_merge_merges_the_primary_in_cleanly` in `sync_tests.rs` (design Part D's last paragraph), exempt from fail-first.
- [ ] **Step 2: Run, expect FAIL** (G1, G7 and G8), by exact name. Record the red runs.
- [ ] **Step 3: Implement** Part G and R-30, with `SynchronizationError` and its exhaustive matches extended for the refusal, mapped to `ticket_closed`.
- [ ] **Step 4: Run** the tests of Step 1 by exact name. Expected: PASS.
- [ ] **Step 5: Commit** "feat(sync): keep the primary's closure, and settle a closure-only conflict with the published closure".

## Task 7: Branch Records, The Pull-Request Close (Design Part D; Test-First With A Design Checkpoint)

**Files:** `src/repository/closure/record.rs` (the rebuild under `wave_06_closure_modes_v1` with an explicit column list; `integration_mode`, `action`, `withdraw_oid`, `retired_oid`, `pending_ref`; the per-action CHECKs; `permits` and `validate` per path; `written_closure_unfinished`'s range for a branch record), `closure/observe.rs` (the branch observation, its blockers and situations, and its digest domain), `closure/checkpoint.rs` (the hook without an integration), `closure/lifecycle.rs` and `closure/mod.rs` (the branch path and its outcome), `promotion/run.rs` (`promote_inner` and `finish_steps` read the path: no integration, the publication child, no cleanup, the answer table; the clock as a parameter), `promotion/lifecycle.rs` (`ParentStep`, the optional integration and cleanup in the outcome), `promotion/lifecycle_record.rs` (`ChildRole::Publication`, `child_allowed`, `validate_request`, `remote_identity_of`, `children_contexts` and the other second-slot readers), `remote/reservation.rs` (`closure_holds_context` exempts the holding record's own validated publication child), `remote/poll.rs` (the poll's retirement consults the fence), the writers' hold for branch records (`repository.rs` `refuse_closed_ticket`, `bind/republish.rs`, `bind/resolve_history.rs`, `ticket repair`, `context retire`, `local_sync/record.rs` `admit` for the ticket's own local synchronization), `pull_request_links.rs` (`upsert_link`, called by the completion and by an `already_closing` abandon), `promotion/mod.rs` (`ParentView`), `bind/lifecycle.rs` (`Stages`, `held_by_its_child`), `bind/lifecycle_verb.rs` (`closure_reached`), `bind/mod.rs` (a lifecycle parent's recorded effects). Tests: `src/repository/closure/record_tests.rs`, a `pull_request` module of `closure/run_tests.rs`, `promotion/run_tests.rs` (`FakeIo` for the publication child), `reservation_tests.rs`. Named edits: E-22, E-23.

**Interfaces:** `ticket_closures` columns `integration_mode` (`IntegrationMode { Direct, PullRequest }`) and `action` (`ClosureAction { Close, Withdraw, NewBranch }`); `ChildRole::Publication`; `close_ticket_observed` takes the path from the observation and the clock from its caller. Exact names may follow the code; the design checkpoint records any change.

- [ ] **Step 1:** Merge `origin/main` if it moved.
- [ ] **Step 2: Write the failing tests first:**
  - `branch_records_rebuild_and_keep_direct_rows` in `record_tests.rs` (P7): an older table with direct rows rebuilds once, every row reads `direct`, `close`, and a second migrate changes nothing; the CHECKs and `permits` per action, with the direct rows unchanged (E-22).
  - one test per row of design D1 to D18, D20's library half and D21, and per row of the answer table, named `d<n>_<what>`, in the `pull_request` module of `closure/run_tests.rs`, built on the existing closure fixture with a pull-request configuration and `FakeIo` extended for the publication child; every test ends by asserting the local primary unchanged and that no child synchronized the primary. D12 and D13 use the existing `exit_at` failure points; D16 runs a save, a comment, an `item sync`, a slug, a context prepare, a republish, a resolve-history, a `context retire` and a poll against a record at `closing`, and the record's own publication child.
  - E-23's row: the C1 fence lets a branch record's own publication child through, and still refuses any other synchronization of the context.
  - The direct close's existing tests run unchanged (exempt from fail-first).
- [ ] **Step 3: Run each, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 4: Design checkpoint.** Before implementing, the implementer writes in its report how it meets each of the design's Design Checkpoint Obligations 1 to 4 and 6 to 11, and: the rebuilt DDL and its CHECKs; `permits` per path; each place in `promote_inner`, `finish_steps`, the hook and the outcome types that reads the path; each second-slot reader and how it reads the path; the fence's range and its exemption; every writer the hold reaches, per mode; the branch observation's situations, blockers and digest; and how the answer table maps to completion and `answer_code`. The orchestrator checks them against design Part D, D1 to D21, the answer table and the source, and records the result in the ledger, before Step 5.
- [ ] **Step 5: Implement** as checked. Apply E-22 and E-23.
- [ ] **Step 6: Run** the tests of Step 2, and the direct close's run tests and record tests that E-22 edits, by exact name. Expected: PASS.
- [ ] **Step 7: Commit** "feat(closure): the pull-request close as a mode of the closure lifecycle". The orchestrator adds the library checkpoint comment.

## Task 8: Withdraw (Design Part E)

**Files:** `src/repository/closure/` (the `withdraw` action's observation, step 3 and answer; the byte helper that removes a ticket's closure fields canonically, beside `closed_ticket_bytes`), `run_tests.rs`'s `pull_request` module.

- [ ] **Step 1: Write the failing tests** `w1_…` to `w14_…` for design W1 to W14, with the withdraw as its own `Lifecycle` implementation (Design Checkpoint Obligation 5); W1 runs in both modes; W6's conflicting variant asserts Task 3's rule accepts an external repair that keeps the withdraw; W9 has the forge merge the closure between the children, with `promotion_test_drive_local_bound` running the publication child's stages. Each ends with Task 7's primary assertions.
- [ ] **Step 2: Run, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Implement** the `withdraw` action.
- [ ] **Step 4: Run** the tests of Step 1 by exact name. Expected: PASS.
- [ ] **Step 5: Commit** "feat(closure): withdraw a provisional closure".

## Task 9: The New Branch (Design Part F; Test-First With A Design Checkpoint)

**Files:** `src/repository/closure/` (the `new_branch` action: step 1's commit and pending ref, step 2's retirement through the lifecycle's cleanup functions, step 3's branch), `src/repository/context_retirement/proof.rs` (`unheld_entries`), `src/repository.rs` (a function that creates a ticket's context branch at a given commit and adds its worktree, without `prepare_context_unlocked`'s root and local-primary checks), `remote/reservation.rs` (the release of the parent's `close` reservation after the retirement, U-1), `run_tests.rs`'s `pull_request` module.

**Interfaces:** `pub(in super::super) fn unheld_entries(repository: &git2::Repository, local: git2::Oid, primary: git2::Oid) -> Result<Vec<UnheldEntry>, ProofRefusal>` with `UnheldEntry { path: Vec<u8>, change: PathChange }`.

- [ ] **Step 1: Write the failing tests:**
  - `unheld_entries_against_the_primary` in `src/repository/context_retirement/tests.rs`, a table: a branch whose changes the primary holds exactly gives none; a squash after which the primary changed one of the paths gives that path `modified`; a path the branch added that the primary lacks gives `added`; a path the branch deleted that the primary keeps gives `deleted`; a rename gives a deletion and an addition; unrelated histories give `UnrelatedHistory` and two merge bases `MultipleMergeBases`.
  - `f1_…` to `f17_…` for design F1 to F17 in the `pull_request` module, each ending with Task 7's primary assertions; F1 and F3 also assert that the new branch's only commit over `P` changes only the ticket's file, and that nothing on the remote side was deleted; F10 has one test per crash point.
- [ ] **Step 2: Run each, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Design checkpoint.** Before implementing, the implementer writes: step 1's commit and pending ref, and the hold from step 1; the cleanup functions step 2 calls under the lease and their order, with each step update, a stop's category and its re-checks; the parent's `close` reservation, taken only for the remote part, and the transition that releases it and records `retired` (U-1; if no existing transition can be generalized, stop for the owner); how `retire_context`'s evidence is built with and without a context; the new branch function's preconditions; and the retry from each step. The orchestrator checks them against F1 to F15 and the source, and records the result.
- [ ] **Step 4: Implement** as checked.
- [ ] **Step 5: Run** the tests of Step 1 by exact name. Expected: PASS.
- [ ] **Step 6: Commit** "feat(closure): close through a new branch". The orchestrator adds the checkpoint comment.

## Task 10: The Data, The Preview And `item sync`'s Link (Design Part H)

**Files:** `src/repository/mutation/dto.rs` (`PullRequestClosureMutationDto` and its tag, `PullRequestLinkDataDto` and its tag, `PullRequestClosurePreviewDto` and its tag, `from_stored`'s guarded arms in design Part H's order, `authority`), `src/repository/mutation/way_out.rs` (a branch record's way out from its data), `src/repository/mutation/bind/lifecycle.rs` (the branch shape in `reached_effects`, `reached_code`, `reached_data`, `parent_integrated`, `parent_effects` and `made_ready`), `src/repository/mutation/bind/sync.rs` (`item sync`'s link answer and its `upsert_link`), `src/bin/manyhands-cli/human_admin.rs` and `src/desktop/view/confirm.rs` (the preview variant's arms), `tests/support/mutation.rs`, `tests/support/repair.rs`. Named edits: E-8. The three schemas land with their first goldens: the preview's in Task 11, the two data schemas in Task 14 (E-9, E-21).

- [ ] **Step 1: Write the failing tests:**
  - `pull_request_closure_data_reads_back` beside `dto.rs`'s `from_stored` tests: each action's stored data for `ticket close` and `ticket withdraw-closure`; a stored lifecycle answer for `ticket close` still reads as the lifecycle DTO; a stored synchronization conflict and host stop for both verbs (D19's library half).
  - `a_branch_record_answers_its_own_shape` beside `bind/lifecycle.rs`'s tests: a completed branch record's effects have no integration, its data is the new DTO, and `made_ready` is empty.
  - `item_sync_gives_the_link_of_a_closing_ticket` beside `bind/sync_tests.rs`'s binding tests.
  - a `way_out` row: a branch record's refusal with `remote_context_unmaterialized` gives `PollOnce`.
- [ ] **Step 2: Run, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Implement.** Apply E-8.
- [ ] **Step 4: Run** the tests of Step 1 by exact name; then `devenv shell -- cargo check --locked --features desktop-test-support --tests`. Expected: PASS.
- [ ] **Step 5: Commit** "feat(mutation): the pull-request closure data and item sync's link".

## Task 11: The Verb And The CLI (Design Parts D, E And H)

**Files:**
- Modify: `src/repository/mutation/command.rs` (`TicketWithdrawClosure` in every method), `mutation/mod.rs` (`Mutation::TicketWithdrawClosure`, `TicketWithdrawClosureInput { root, id }`), `bind/lifecycle_verb.rs` (a `Withdraw` verb type bound by `LifecycleBinding`), `bind/resume.rs` (the rebuilt lifecycle input reads the record's action), `mutation/progress.rs` (`asks_a_parent` and `cancel_parent` reach the new verb), `src/repository/recovery.rs` (`LIFECYCLE_COMMANDS` gains the verb)
- Modify: `src/bin/manyhands-cli/catalog.rs` (delivered `C5`), `parse.rs`, `dispatch.rs`, `lifecycle.rs`, `human_lifecycle.rs`, `human_collaboration.rs` (`item sync`'s link line), `mutate.rs` (the retry guard reads the branch record's blockers); the desktop's verb arms
- Modify: `docs/RFC/cli-contract.md` (the taxonomy row only), `skills/manyhands-overview/references/autonomy.md` (the verb in `ticket close`'s column)
- Create: `schemas/v1/pull_request_closure_preview.schema.json`, in `DATA_ROOTS` (E-21, 69 to 70), with its enums registered (E-9)
- Test: `tests/cli/lifecycle.rs`, `src/bin/manyhands-cli/lifecycle_tests.rs`, `tests/mutation_confirmation`
- Named edits: E-9 (its part), E-10 to E-13, E-19, E-20, E-21 (its part)

- [ ] **Step 1:** Merge `origin/main` if it moved. If Cycle 02 has landed, re-check `PullRequestDto` and `setup_pending`; if this Cycle lands second, add `setup_pending` to the close and withdraw prepares, after the ticket blockers, with a test row.
- [ ] **Step 2: Write the failing tests:**
  - In `tests/cli/lifecycle.rs`, with a committed version 2 pull-request configuration and an unreachable SSH remote (as `tests/cli/collaboration_exits.rs`'s `Fixture::with_remote` writes one; "observed deleted" by a `remote_context_states` row, as `closure/observe_tests.rs` writes one): `pull_request_close_prepare_previews_the_record`, which writes the preview golden; `…_previews_an_existing_closure` and `…_previews_the_new_branch`, asserting fields without goldens; `withdraw_of_a_closure_the_primary_holds_is_ticket_closed`; and `pull_request_close_stops_at_the_context_child`, with a selected key and a closed port as `collaboration_exits.rs` makes them, answering `transport_unavailable` with nothing written. Register each golden in `CLI_CASES`.
  - In `tests/mutation_confirmation`, the new verb's conformance row (E-12), and `withdraw_with_no_closure_is_already_applied`, which writes the `mutation_v1` golden `ticket_withdraw_closure_already_applied`, registered in `mutation_golden.rs` `CASES` (E-13).
  - `pull_request_closure_lines` in `lifecycle_tests.rs`: each closure line, the publication line, the link's line, and an unheld entry with a control character escaped.
  - E-20's pull-request-mode part; and the resume and cancel rows: `request cancel` of a stopped withdraw answers `cancelled` and the same request continues it; `operation resume` of an unfinished branch record answers `original_request_required` (D20's binding half).
- [ ] **Step 3: Run, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 4: Design checkpoint** for the binding: how `LifecycleBinding` carries the action, how resume rebuilds the input, and the order of the `from_stored` arms. The orchestrator checks them, and records the result.
- [ ] **Step 5: Implement.** Apply E-9's part, E-10 to E-13, E-19, E-20 and E-21's part.
- [ ] **Step 6: Run** by exact name Step 2's tests, the `command.rs` tests E-10 edits, the inventory, help, catalog, conformance, skills and contract tests E-11 to E-13 and E-19 name, `every_bound_command_has_a_golden`, `every_schema_is_reachable_from_a_published_root` and `every_data_root_describes_the_data_of_a_golden_and_every_data_has_one_root`; then `devenv shell -- cargo check --locked --features desktop-test-support --tests`. Expected: PASS.
- [ ] **Step 7: Commit** "feat(cli): close in pull-request mode and ticket withdraw-closure".

## Task 12: The Conflict Verbs For The Publication Child (Design Part D)

**Files:** `src/repository/remote/state.rs` (`lifecycle_context_child` and `lifecycle_parent_of` read the second slot of a branch record), `remote/conflict_read.rs`, `mutation/bind/conflict.rs`, `mutation/records.rs` (`operation_or_parent_holder`); their tests.

- [ ] **Step 1: Write the failing test** `a_branch_record_s_publication_conflict_is_found_by_its_id`: a branch record whose publication child holds a conflict: `conflict show` and `conflict resolve` by the record's ID reach the child, and the child's holder is the record (D10, D20).
- [ ] **Step 2: Run, expect FAIL**, by exact name. Record the red run.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the test by exact name, and the existing conflict-mapping tests of a direct record (unchanged). Expected: PASS.
- [ ] **Step 5: Commit** "feat(conflict): find a branch record's publication conflict". The orchestrator adds the checkpoint comment.

## Task 13: The Owner's Lifecycles In The Library (Design P13)

**Files:** `src/repository/closure/run_tests.rs` (the `pull_request` module).

- [ ] **Step 1: Write the tests**, with a local bare repository as the forge, driven by plain `git2` as `context_retirement/acceptance_tests.rs` drives one, `FakeIo` pushing the ticket's branch to it and `promotion_test_drive_local_bound` running the children's stages, and `poll::synthetic_fetch` with a `git2` fetch feeding the poll:
  - `lifecycle_ends_closed_and_retired_for_each_merge_style`: a table over merge commit, squash and fast-forward: record, publish, merge on the forge and delete the branch, poll; the poll answers the ticket closed, retirement succeeds, the link record is gone, and the ticket reads `closed` (A1, A2);
  - `the_recording_clone_reads_closing_until_it_fetches` (A3);
  - `declined_withdrawn_then_recorded_again` (A4);
  - `comments_and_saves_while_provisional` (A5);
  - `two_clones_record_and_the_later_keeps_the_published_closure` (A6);
  - `a_dependent_waits_for_the_primary` (A7);
  - `merged_first_with_the_remote_branch_kept_closes_in_place` (A9) and `merged_first_with_the_remote_branch_gone_closes_through_a_new_branch`, proven and unproven (A8);
  - `another_clone_reads_closing_after_its_poll`.
- [ ] **Step 2: Run each** by exact name. A test that passes first is recorded as such; each must then be shown able to fail once, by a deliberate stub in the runner (for example, one that integrates a branch record), and the red run recorded.
- [ ] **Step 3: Commit** "test(closure): the pull-request lifecycles".

## Task 14: The SSH Case And The Goldens (Design P14)

**Files:** `tests/mutation_remote/c5_lifecycle.rs` (the case, registered in `tests/mutation_remote.rs`'s case table beside the C5 lifecycle cases), `tests/support/mutation_golden.rs` (`CASES`), `tests/mutation_contract.rs`, `tests/cli/inventory.rs`, `schemas/v1/pull_request_closure_mutation.schema.json`, `schemas/v1/pull_request_link.schema.json`, the goldens under `tests/fixtures/mutation_v1/`. Named edits: E-7, E-9 (its part), E-11 (its part), E-21 (its part).

- [ ] **Step 1: Write the case** `a_pull_request_close_publishes_only_the_ticket_branch`, with this comment above it: no non-SSH publication remote can be selected, and the property is that only the ticket's branch reaches the server (decision 11). The case:
  1. commits a pull-request configuration with `[forge] pull_request_url = "https://forge.example/new?from={source}&to={target}"` on `w.peer` and pushes it, then runs a `repo sync` in the clone so its primary holds it (Cycle 01 K-2), and records the server's primary tip;
  2. creates and saves a ticket, then `ticket close` (prepare and confirm): `ok`, `pull_request.url` `https://forge.example/new?from=manyhands/ticket/<ULID>&to=main`, the server's ticket branch at the close commit, the server's primary unchanged;
  3. `ticket close` again: `already_closing`, nothing new on the server;
  4. `item sync` of the ticket: the tagged link data;
  5. `ticket withdraw-closure`: `ok`, the server's branch at the withdraw commit, the primary unchanged;
  6. `ticket close` again: `ok`, and the same request again: the identical envelope (the conformance rules E-12 exempts);
  7. writes the goldens `ticket_close_pull_request_ok`, `ticket_close_already_closing`, `ticket_withdraw_closure_ok` and `item_sync_pull_request_link`, with commit IDs replaced as existing goldens replace them.
- [ ] **Step 2: Run it red** against a stub that answers `ok` without publishing (step 2's server check then fails): `devenv shell -- cargo test --all-features --locked --test mutation_remote a_pull_request_close_publishes_only_the_ticket_branch -- --exact`. Record the red run.
- [ ] **Step 3:** Add the two data schemas with their enums registered (E-9) and in `DATA_ROOTS` (E-21, 70 to 72), and their names in `tests/cli/inventory.rs`'s `DATA` rows (E-11); register the goldens in `CASES`, generate them with `MANYHANDS_UPDATE_GOLDEN=1` on the same command, read them, and apply E-7.
- [ ] **Step 4: Run** the case again without the update variable, then by exact name both `every_result_code_has_a_*` tests, `every_bound_command_has_a_golden`, `every_schema_is_reachable_from_a_published_root`, `every_data_root_describes_the_data_of_a_golden_and_every_data_has_one_root`, the inventory's data-schema test, and `published_enumerations_list_exactly_the_registered_values`. Expected: PASS.
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
| A12 | 7, 8, 9, 11, 12 |
| A13 | 3, 6 |
| A14 | 4, 5, 10, 11, 14 |
| A15 | 15 |
| A16 | 16 |
| R-1, R-2, R-3, R-4, R-5, R-17, R-18, R-23, R-27, R-34, R-35, R-36 | 7 |
| R-6, R-7, R-8, R-9 | 1 |
| R-10, R-11 | 2 |
| R-12 | 3 |
| R-13, R-30 | 6 |
| R-14, R-15, R-26 | 4 |
| R-16 | 5 |
| R-19, R-20, R-21, R-31 | 9 |
| R-22 | 1, 5, 10, 11 |
| R-24, R-28 | 11 |
| R-25 | 7, 10, 14 |
| R-29 | 10 |
| R-32, R-33 | 8 |
| Review Focus lines | 1, 2, 3, 6, 7, 8, 9, 11, 13, 14 |

Checked: every design Part has a task (A: 1; B: 2 and 3; C: 4; D: 7, 10, 11, 12; E: 8; F: 9; G and R-30: 6; H: 5, 10, 11); the types named in Interfaces match across tasks (`IntegrationMode`, `ClosureAction`, `ChildRole::Publication`, `PullRequestDto`, `UnheldEntry`, `PullRequestClosureMutationDto`, `PullRequestLinkDataDto`, `PullRequestClosurePreviewDto`); no placeholder steps; the plan adds no test target, so no CI list changes.
