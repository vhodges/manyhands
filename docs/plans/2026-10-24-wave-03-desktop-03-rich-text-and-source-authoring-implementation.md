---
title: "Wave 03 D3 Rich-Text And Source Authoring Implementation Plan"
date: 2026-10-24
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5RE7RXNQ4PNZR6NZWEP9Q75"
---

# Rich-Text And Source Authoring Implementation Plan

> **For agentic workers:** Draft for product-owner review. Implementation is
> not authorized until the owner approves the Cycle, design and this plan.
> Once authorized, use `implementing-a-cycle` and the selected execution
> method task by task, re-read the design, repeat the ticket preflight, and
> record every checkpoint on ticket `01M4CC0VN0MFTGKQ4HGTTC5H63`. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Read, edit, create, move and repair documents and tickets in the
desktop with `zorite-editor`. Links and local images work, saves go through
D2's drafts and F2, every save state is told truthfully, and unused work
contexts are released.

**Architecture:**
- **Part 0** adds two F2 commands: `context prepare`, over the domain's
  `prepare_context`, and `context release`, over a new domain operation.
- **The desktop** renders bodies with `zorite-markdown`'s `MarkdownView`,
  hosts one `zorite_editor::EditorState` per editing tab, resolves links
  and images itself, feeds D2's `DraftRegistry`, and submits new `Intent`s
  through D1's worker.
- **`src/drafts/`** gains one target state.

**Tech stack:**
- Rust 2024.
- `gpui-kit` 0.6.6, through `gpui_kit::*` only.
- `zorite-editor` `=0.10.0`, `zorite-markdown` `=0.9.0` with feature `view`,
  `markdown` `=1.0.0` and `image` `=0.25.10`. All four are already locked; no
  new package.
- Devenv.

**Spec:** [Cycle](../Cycles/wave-03-desktop-03-rich-text-and-source-authoring.md)
and [design](2026-10-24-wave-03-desktop-03-rich-text-and-source-authoring-design.md).
Executors read both. "Fact n", "N-n", "R-n" and "E-n" refer to the
design's Baseline Facts, Recorded Load Normalization, Rulings and Named
Edits.

## Global Constraints

**Where and how to run.**
- Work only in ticket `01M4CC0VN0MFTGKQ4HGTTC5H63`'s worktree and branch.
  Preserve unrelated edits in main and every other worktree.
- Run every Rust command as `devenv shell -- cargo …` with `--locked`. The
  one exception is Task 3's single lockfile update.

**Library changes are Part 0's only** (decisions 2 and 11).
- **Task 1** may change:
  - `src/repository/mutation/`;
  - `schemas/v1/context_mutation.schema.json`;
  - `tests/mutation_bridges*`, `tests/support/` and
    `tests/fixtures/mutation_v1/`.
- **Task 2** may also change:
  - `src/repository.rs`, `src/repository/context_release.rs` and its tests;
  - `src/repository/cleanup.rs`, `recovery.rs` and `bridges/abandon.rs`;
  - `src/repository/read/{dto,status}.rs`;
  - the three operation schemas;
  - `tests/read_contract.rs` (E-20);
  - `src/repository/read/status_tests.rs` (E-21);
  - the test module inside `src/repository/recovery.rs` (E-22).
- **No other task** changes `src/repository.rs`, `src/repository/`,
  `src/canonical.rs`, `src/results.rs`, `src/runtime.rs` or `src/lib.rs`.
- **A gap found later** stops the task: file a shared-library ticket with
  `file-ticket` and ask the owner.
- **No new `ResultCode`.**

**Drafts module.** `src/drafts/` changes only in Task 15 (`NotCreated`,
R-20), and imports nothing from `gpui_kit`.

**Dependencies.** `Cargo.toml` changes only in Task 3. `Cargo.lock`
changes only there and gains no new `[[package]]`.

**GPUI.**
- Use it only through `gpui_kit::*`, and never as a `use gpui_kit::*` glob.
- `zorite_editor`, `zorite_markdown`, `markdown` and `image` are used only
  under `src/desktop/`.

**Privacy.**
- No type holding body, draft or source text derives `Debug`.
- Body text and link or image targets never enter an operation entry,
  notification, dialog message, log or error.

**Test isolation.**
- Every test uses its own temporary repositories, data directory and Git
  home.
- No test reaches a network.
- No desktop test initializes the transport.

**Editor tests.**
- Set the cursor, focus the editor and draw a frame before typing.
- Type with the window-level `simulate_input` / `simulate_keystrokes`,
  never gpui-kit's scoped helpers.
- Call `zorite_editor::bind_keys` in the test shell.
- Never send Ctrl-Cmd-Space.
- A view test that waits on the worker calls
  `cx.executor().allow_parking()`.

**Test runs.**
- `cargo test` takes one positional filter, and each Verify line uses one.
- Every Verify step checks that the run reported more than zero tests.
- Within a task, run only the tests its Verify lines name.
- The full suite runs in Task 0 (entry-gate item 7) and once in Task 20.
  This follows the owner's standing rule: targeted tests between tasks,
  the full suite once at the end.
- Part checkpoints run the Part's targets in full: the library mutation
  targets for Part 0, and the desktop binary's tests and `--test drafts`
  for Parts A to D.

**Desktop verification.** Each desktop task verifies with:
- `--features desktop-test-support` tests;
- `--features desktop` check and clippy on their own;
- as its last Verify line,
  `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`.

**Dead code.** An item that only a later task calls carries
`#[cfg_attr(not(test), allow(dead_code))] // Called from Task N.` Task N is
the task that first calls it in production code. That task removes the
allowance. This is the repository's convention (`services/requests.rs:337`).

**Test-first.** Write each behavior's test before its code.

**Existing tests.** They change only as named edits E-1 to E-22. Any other
change to a pre-existing test, even a new lint, stops for the owner.

**Commits.** Commit per task with a conventional message ending
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Merging.**
- Merge `origin/main` at work boundaries; never rebase.
- Before each Part checkpoint, run `git fetch origin` and
  `git merge-tree --write-tree HEAD origin/main`.
- E-15 to E-17 count "main's at the merge, plus D3's".

**Ticket records.**
- After each task, add a ticket comment as a canonical file under
  `.manyhands/comments/01M4CC0VN0MFTGKQ4HGTTC5H63/`. Take the ULID from
  `manyhands-cli id new` and `created_at` from the UTC clock.
- Each comment records the commit, the tests run with their counts and
  results, and any ruling.
- Once C3 has merged and the CLI supports ticket or comment writes, use
  the CLI for them (`AGENTS.md`).
- Part checkpoints add the review and its fixes.
- They notify the owner but do not wait, unless the owner asks for a stop.
  Part 0's review must pass before Task 3.

**Execution ledger.**
`docs/plans/2026-10-24-wave-03-desktop-03-rich-text-and-source-authoring-execution.md`
is created in Task 0 and committed with every task. D1 and D2 never
committed theirs.

**File size.** Each new desktop source file stays below about 500 lines;
test files are exempt.

**Scripts** run through Devenv too:
`devenv shell -- sh scripts/check-cli-no-gpui.sh`.

**Implementers never dispatch subagents.**

## Review Focus

1. **Line endings and `$$` together.** A body with CRLF line endings and a
   words-mixed `$$` pair opens in the editor and is not dirty. Done changes
   no canonical byte and makes no commit. Pinned in Tasks 3 and 10.
2. **Typing during a save while a tab read lands.** Newer text stays in the
   editor and in the draft; only the saved revision retires. Pinned in
   Task 11.
3. **Edit on a hand-edited, uncommitted primary file.** Refused with
   `worktree_not_clean`; no branch or worktree is made; the editor never
   opens. Pinned in Tasks 1 and 9.
4. **A very large body**, over 128 KiB, in the read view. It renders plain
   first and then formatted, without blocking. Edit opens it, and typing
   stays responsive (checklist step 9). Pinned in Tasks 4 and 9.
5. **A crafted image or link path**, using `..`, a symbolic link out of the
   repository, `.git`, another item's worktree, SVG content named `.png`,
   or `file:` and `javascript:` schemes. Nothing is read, decoded or
   opened. Pinned in Tasks 6 and 7.

## File Map And Dependency Order

| Path | Task | Change |
| --- | --- | --- |
| `docs/plans/…-execution.md` | 0 | Execution ledger. |
| `src/repository/mutation/{command,mod,dto}.rs`, `bind/{mod,context}.rs`, `schemas/v1/context_mutation.schema.json`, `tests/mutation_bridges{.rs,/context.rs}`, `tests/fixtures/mutation_v1/context_*.json`, `tests/support/{conformance,mutation_golden,mutation,repair}.rs`, `tests/mutation_contract.rs` | 1 | `context prepare`; E-15 to E-18. |
| `src/repository.rs`, `src/repository/{context_release,cleanup,recovery}.rs`, `bridges/abandon.rs`, `read/{dto,status,status_tests}.rs`, three operation schemas, `tests/read_contract.rs` | 2 | `context release`; E-20 to E-22. |
| `Cargo.toml`, `Cargo.lock`, `src/desktop/model/editor_text.rs`, `src/desktop/view/editor.rs`, `tests/fixtures/editor/*` | 3 | Dependencies, feasibility, load normalization. |
| `src/desktop/view/item/body.rs`, `view/item.rs`, `view/item/source.rs`, `view/item_tests.rs`, `view/tabs_tests.rs` | 4 | Read view; E-1 to E-5. |
| `src/desktop/view/editor/{style,toolbar,keys}.rs`, `keys.rs`, `mod.rs`, `view/harness.rs` | 5 | Editor host. |
| `src/desktop/model/links.rs`, `services/links.rs`, `model/link_rewrite.rs`, `services/render_prep.rs`, `view/item/body.rs` | 6 | Links; background preparation. |
| `src/desktop/model/resources.rs`, `services/images.rs`, `Cargo.toml` already holds `image` | 7 | Images. |
| `src/desktop/services/{requests,mod}.rs`, `requests/build.rs`, `services/requests_tests.rs` | 8 | Eight intents; E-11 to E-14. |
| `src/desktop/model/edit.rs`, `view/item/edit.rs`, `view/item/details.rs`, `model/item.rs`, `model/item_tests.rs` | 9 | Edit, provisioning, release on leave; E-6, first half. |
| `view/item/edit.rs`, `services/drafts.rs`, `services/drafts/save.rs` | 10 | Save, states, Done. |
| `view/item/draft.rs`, `view/close_draft.rs`, `view/workspace/content.rs`, `view/item/reads.rs`, `services/drafts/restore.rs`, `view/drafts.rs`, `view/item/draft_tests.rs`, `view/tabs_tests.rs` | 11 | Restored drafts, close-dialog Save, late results, normalized classification; E-8, E-9. |
| `view/item/review.rs`, `test_support/drafts.rs` | 12 | Stale-edit review; E-19. |
| `view/item/metadata.rs`, `model/metadata.rs` | 13 | Metadata controls. |
| `view/item/relations_edit.rs`, `model/relations_edit.rs` | 14 | Relationships. |
| `src/drafts/compare.rs`, `services/drafts/create.rs`, `services/mod.rs` (`for_drafts`), `view/item/draft.rs`, `view/item/reads.rs`, `view/draft_comparison.rs` | 15 | New-item drafts. |
| `view/create.rs`, `view/documents.rs`, `view/lists.rs`, `keys.rs` | 16 | Create dialogs. |
| `view/move_document.rs`, `view/new_folder.rs`, `view/documents.rs`, `view/documents_tests.rs`, `view/item/details.rs` | 17 | Move and New folder; E-10. |
| `view/item/repair.rs`, `view/confirm.rs`, `model/item.rs`, `view/item/discussion.rs`, `model/item_tests.rs`, `view/item_tests.rs`, quit path | 18 | Repair; E-6, second half; E-7. |
| `view/keyboard_tests/editing.rs`, `view/editing_privacy_tests.rs` | 19 | Keyboard and privacy. |
| Cycle table, ledger, ticket | 20 | Gate and review-ready. |

Tasks run in order. Unit tests sit beside their module as `*_tests.rs`.

---

## Task 0: Baseline, References And Ledger

- [ ] **Step 1: Merge and run the baseline (entry-gate item 7).**

```sh
git fetch origin
git merge origin/main          # never rebase
git merge-base --is-ancestor origin/main HEAD
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo check --locked --features desktop
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo test --all-features --locked
devenv shell -- cargo run --locked --bin manyhands-cli
```

  Run the test line detached, with a two-hour budget. A failure is resolved
  or accepted by the owner before Task 1.

  Save the output of `devenv shell -- cargo run --locked --bin manyhands-cli -- help`
  in the ledger. Task 1 compares its own run against this copy.

- [ ] **Step 2: Re-check the Baseline Facts.**
  - Run `git diff --stat 98f49b7 HEAD -- src/repository.rs src/repository src/results.rs src/drafts src/desktop tests/support tests/read_contract.rs schemas Cargo.toml .github`.
  - Re-read each fact whose file changed, and update the design before
    coding if a cited signature moved.
  - Confirm that the design's Document Amendments are in the RFCs and the
    Wave. They are applied in the approval commit.

- [ ] **Step 3: Sweep for unlisted test edits.** Search every `*.rs` under
  `src/` and `tests/`, not only `*_tests.rs`, for:
  - `LATER`, `EDITOR_LATER`, `DRAFT_ACTIONS`, `NEW_FOLDER_UNAVAILABLE`;
  - `item-source`, `later-button`, `draft-later`, `close-draft-save`;
  - `MutationDataDto::`, `OperationAction::`, `RepositoryOperation::`,
    `LocalStep::`, `TargetState::`;
  - `listed.len()`, `HOMES`, `DATA_ROOTS`, `CASES`, `LOCAL_ACTIONS`;
  - `CloseWithDraft::`.

  Compare each hit with E-1 to E-22. A pre-existing test that will break
  and is not listed stops for the owner. Record the sweep in the ledger.

- [ ] **Step 4: Ledger.** Create the ledger with:
  - the base, both heads and each check result with its counts;
  - the sweep;
  - a "Rulings during implementation" section.

- [ ] **Step 5: Commit, and write the ticket comment.**

---

# Part 0: Work Contexts

## Task 1: `context prepare`

**Files:** as the File Map's Task 1 row.

**Interfaces:**
- Produces, as in the design's Part 0:
  - `ContextPrepareInput`, `ContextPrepareIntent`;
  - `Mutation::ContextPrepare`;
  - `ContextMutationDto`, `MutationDataDto::Context`.
- Consumes:
  - `prepare_context` (fact 17);
  - `super::save::primary_is_committed`;
  - `item_scope`, `repository_error_code`;
  - `authoring_kind_segment`, for the digest.

- [ ] **Step 1: Write the outcome tests first (R-4).**
  - In `tests/mutation_bridges/context.rs`, write one test per row of the
    design's `context prepare` table, over a real enabled repository.
  - Use `FailOnce` with `BeforeRequestDomainCall`, `BeforeWorktreeCreation`
    and `BeforeRequestSettlement`, following `local_authoring.rs:7077` and
    `mutation_replay/index.rs:141`.
  - Add these behavior tests:

```text
prepare_edit_provisions_and_a_second_request_reuses_it
prepare_create_provisions_for_a_new_id_and_document_create_reuses_it
prepare_create_provisions_for_a_new_id_and_ticket_create_reuses_it
prepare_edit_of_a_closed_ticket_is_ticket_closed_and_makes_nothing
prepare_edit_of_an_uncommitted_primary_change_is_worktree_not_clean_and_makes_nothing
prepare_create_of_an_existing_id_is_occupied_path
prepare_edit_of_a_missing_item_is_item_not_found
prepare_records_hold_no_body
```

  - Mount the module, then run
    `devenv shell -- cargo test --locked --test mutation_bridges prepare_`.
    Expected: compile errors.

- [ ] **Step 2: Add the command and its plumbing.**
  - The `command.rs` entry, with the design's row values.
  - The `Mutation` variant and its arms.
  - `recorded_effects`, `from_stored`, `ContextMutationDto` and the
    `authority()` arm.
  - **Settlement:**
    - `ended_without_checkpoint` is `DidNothing`;
    - `ok` settles final;
    - `discovery_pending` settles owed.

- [ ] **Step 3: Write the binding** as the design says. If a row of the
  table cannot pass without changing files outside Task 1's list, stop for
  a ruling.

- [ ] **Step 4: Registries and named edits E-15 to E-18.**
  - Schema `context_mutation.schema.json`.
  - Goldens `context_prepare.json` and
    `context_prepare_discovery_pending.json`, registered in `CASES`.
  - The `HOMES` row in `mutation_bridges`'s conformance table.
  - `DATA_ROOTS`.
  - The `Context(_)` arms in `tests/support/mutation.rs` and
    `tests/support/repair.rs`.

- [ ] **Step 5: Verify.**

```sh
devenv shell -- cargo test --locked --test mutation_bridges prepare_
devenv shell -- cargo test --locked --test mutation_bridges keeps_the_rules_of_the_boundary
devenv shell -- cargo test --locked --test mutation_replay keeps_the_rules_of_the_boundary
devenv shell -- cargo test --locked --test mutation_confirmation keeps_the_rules_of_the_boundary
devenv shell -- cargo test --locked --test mutation_remote keeps_the_rules_of_the_boundary
devenv shell -- cargo test --locked --test mutation_contract every_
devenv shell -- cargo test --locked --test mutation_contract the_fixture_directories_are_exactly_the_registered_cases
devenv shell -- cargo test --locked --lib mutation::
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo run --locked --bin manyhands-cli -- help
```

  Expected: all pass, and the help output equals the copy Task 0 saved in the
  ledger (R-5).

- [ ] **Step 6: Commit, and write the ticket comment.**

## Task 2: `context release`

**Files:** as the File Map's Task 2 row.

**Interfaces:**
- Produces:
  - `ReleaseContextRequest`, `ContextReleaseOutcome` and
    `RepositoryService::release_context`;
  - `RepositoryOperation::ReleaseContext`, action `release_context`;
  - `LocalStep::{ContextWorktreeReleased, ContextBranchReleased}`;
  - `FailurePoint::{BeforeContextBranchRelease, AfterContextBranchRelease}`;
  - `OperationAction::ReleaseContext`;
  - `ContextReleaseInput`, `Mutation::ContextRelease`.
- Consumes:
  - the cleanup primitives, through lease-held variants and a precheck
    that does not require the item (R-32);
  - `reject_pending_synchronization_target`;
  - `handoff_post_authoritative`, with the hand-off owed to
    `operation resume`;
  - `RemoteSnapshot::publication_evidence_for`;
  - `lifecycle_record::unfinished_record`.

- [ ] **Step 1: Write the outcome tests first.**
  - **Domain tests,** in `src/repository/context_release_tests.rs`, one per
    check (a) to (d), plus one for ownership:
    - each refusal leaves the worktree, branch and `contexts` row unchanged;
    - a Create context with no item is released;
    - a branch left without its worktree is released;
    - after a release, a `context prepare` Edit answers `created: true`;
    - each of these refuses: an unfinished closure or promotion, a proven
      publication, and a non-primary branch checked out at the root;
    - an unknown publication history with no tracking ref is released
      (R-43).

  These are the design's required-outcome table. A row that answers
  differently stops the task for a ruling. The table is not edited to fit.
  - **Binding tests,** in `tests/mutation_bridges/context.rs`, one per row
    of the design's `context release` table, prefixed `release_`.

- [ ] **Step 2: Implement** the domain operation, its steps, failure point,
  action name, abandon `writes` arm, `OperationAction`, schemas and the
  binding.
  - The lease-held removal uses the item-free precheck.
  - The hand-off is owed to `operation resume`, and
    `hands_off_to_any_refresh` is false (R-32).
  - Apply E-20, E-21 and E-22.
  - Add the second `HOMES` and `ROWS` rows (E-15) and the golden
    `context_release.json` (E-16).

- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --lib context_release
devenv shell -- cargo test --locked --lib cleanup
devenv shell -- cargo test --locked --lib recovery
devenv shell -- cargo test --locked --lib read::status
devenv shell -- cargo test --locked --lib mutation::
devenv shell -- cargo test --locked --test mutation_bridges release_
devenv shell -- cargo test --locked --test mutation_bridges keeps_the_rules_of_the_boundary
devenv shell -- cargo test --locked --test read_contract published_enumerations_list_exactly_the_registered_values
devenv shell -- cargo test --locked --test read_contract every_published_
devenv shell -- cargo test --locked --test mutation_contract every_
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Part 0 Checkpoint

- [ ] **Step 1: Merge.** Run the merge-tree check, and merge main if it
  moved.
- [ ] **Step 2: Run Part 0's targets in full.**
  - The targets: `--test mutation_bridges`, `mutation_replay`,
    `mutation_confirmation`, `mutation_remote`, `mutation_contract`,
    `read_contract` and `local_authoring`.
  - The library unit tests: `--lib`.
- [ ] **Step 3: Review.** A fresh reviewer reviews Part 0 as a library
  change of its own: scope, both outcome tables, R-1 to R-5, R-32, R-43
  and E-15 to E-22. Use the adversarial brief on release and replay.
- [ ] **Step 4: Ticket comment.** Desktop tasks start only after this
  review passes (decision 2).

---

# Part A: Read View, Editor, Links And Images

## Task 3: Dependencies, Editor Feasibility And Load Normalization

**Interfaces:**

```rust
// src/desktop/model/editor_text.rs
pub(crate) fn crlf_to_lf(body: &str) -> String;          // N-1
pub(crate) fn load(body: &str) -> String;                 // N-1, then normalize_math_fences
pub(crate) fn normalized(item: &ItemDto) -> ItemDto;      // body through load (R-11)
// src/desktop/view/editor.rs
pub(crate) fn open_editor(body: &str, window: &mut Window, cx: &mut App) -> Entity<EditorState>;
```

- [ ] **Step 1: Dependencies.**
  - `desktop = ["dep:gpui-kit", "dep:futures", "dep:zorite-editor", "dep:zorite-markdown", "dep:markdown", "dep:image"]`.
  - `image = { version = "=0.25.10", default-features = false, features = ["png", "jpeg", "gif", "webp", "bmp"], optional = true }`.
  - `editor-probe = ["desktop"]`.
  - `zorite-markdown = { version = "=0.9.0", default-features = false, features = ["view"], optional = true }`.
  - `markdown = { version = "=1.0.0", optional = true }`.
  - Run `devenv shell -- cargo check --features desktop` once, without
    `--locked`.
  - Then check `git diff Cargo.lock`: only dependency lists may change. No
    new `[[package]]` and no second `gpui-pre`.

```sh
devenv shell -- cargo check --locked --features desktop
devenv shell -- cargo check --locked --features editor-probe --example editor_feasibility
devenv shell -- sh scripts/check-cli-no-gpui.sh
```

- [ ] **Step 2: Feasibility test (stop point).**
  `view/editor_tests.rs::a_real_editor_takes_typing_and_undo_in_a_headless_window`:
  1. open a test window;
  2. call `zorite_editor::bind_keys`;
  3. create an `EditorState` with `"abc"`, then `set_cursor(3)`;
  4. focus it and draw a frame;
  5. `simulate_input("d")`: `text()` is `"abcd"`, and one `Changed` was
     seen;
  6. `simulate_keystrokes("ctrl-z")`: `text()` is `"abc"`.

  If it cannot pass with the window-level helpers, stop and ask the owner
  before Task 4.

- [ ] **Step 3: Fixtures and load tests first.**
  - Add the design's golden fixtures, with `.expected` files.
  - `editor_text_tests.rs`:
    - `each_fixture_loads_to_its_expected_file`
    - `each_exemption_loads_unchanged`
    - `load_is_idempotent`
    - `normalized_changes_only_the_body`
  - `editor_tests.rs`: `the_editor_reads_back_exactly_what_load_predicts`,
    over every fixture.

- [ ] **Step 4: Implement.** Items that only later tasks call carry the
  dead-code allowance:
  - `open_editor` names Task 5;
  - `load` names Task 5;
  - `normalized` names Task 11.

- [ ] **Step 5: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands editor
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 6: Commit** `Cargo.toml`, `Cargo.lock`, the fixtures and the
  code, and write the ticket comment.

## Task 4: The Read View And The Source Toggle

- [ ] **Step 1: Tests first** (`view/item/body_tests.rs`).
  - `a_document_body_renders_and_its_text_is_on_screen`
  - `a_ticket_body_renders`
  - `show_source_shows_the_exact_file_and_the_choice_survives_a_reload`
  - `an_inspect_tab_always_shows_the_source`
  - `a_non_utf8_item_says_so_in_both_views`
  - `a_large_body_renders_plain_then_formatted_without_blocking`
    (200 KiB; Review Focus 4)
  - `viewing_and_toggling_change_nothing_in_the_repository`
    (`RepositorySnapshot` before and after)
- [ ] **Step 2: Implement.**
  - Create `services/render_prep.rs`, with background `warm_parse` per body
    version (R-7). Task 6 adds the link rewrite and image sources to it.
  - Apply E-1 to E-5.
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::item
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::tabs
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands keyboard_tests
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 5: The Editor Host

**Interfaces:**

```rust
pub(crate) struct EditorHost { /* editor, baseline: String, rich: bool, dirty: bool */ }
impl EditorHost {
    pub(crate) fn new(body: &str, window: &mut Window, cx: &mut Context<Self>) -> Self;
    pub(crate) fn text(&self, cx: &App) -> String;
    pub(crate) fn is_dirty(&self) -> bool;
    pub(crate) fn set_rich(&mut self, rich: bool, cx: &mut Context<Self>);
    pub(crate) fn rebase(&mut self, baseline: String, text: &str, cx: &mut Context<Self>); // R-18
    pub(crate) fn reload_clean(&mut self, body: &str, cx: &mut Context<Self>);             // R-19
}
pub(crate) enum EditorHostEvent { DirtyChanged(bool), Edited }
// view/editor/keys.rs: bind(cx): zorite_editor::bind_keys, then cmd-u/ctrl-u → NoAction in "Editor" (R-8)
// keys.rs: actions Edit, Save, ToggleRich, NewItem
```

- [ ] **Step 1: Tests first.**
  - `a_mode_switch_keeps_the_text_and_the_undo_history`
  - `typing_in_rich_mode_then_switching_round_trips_each_fixture`
  - `undo_back_to_the_baseline_makes_the_editor_clean`
  - `a_normalized_body_is_not_dirty_after_load_and_mode_switches`
  - `bold_italic_code_and_strike_work_from_keys_and_from_the_toolbar`
  - `ctrl_u_does_not_underline`
  - `tab_indents_inside_the_editor_and_f6_leaves_it`
  - `table_buttons_show_in_a_table_and_insert_a_row`
  - `plain_text_paste_works_in_both_modes`
  - `enter_backspace_and_arrows_edit_as_expected`
- [ ] **Step 2: Implement.** The startup binding goes in `desktop/mod.rs`,
  after `gpui_kit::init`, and the same binding in `view/harness.rs`.
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands editor
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands keys
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands keyboard_tests
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- sh scripts/check-cli-no-gpui.sh
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 6: Links

**Interfaces:**

```rust
// model/links.rs
pub(crate) enum LinkTarget { External(String), Unsafe, Path { path: String, from_root: bool }, Title(String), Ignored }
pub(crate) fn parse_link(raw: &str) -> LinkTarget;
pub(crate) fn nearest<'a>(from_dir: &str, candidates: &'a [CandidatePath]) -> Option<&'a CandidatePath>;
// services/links.rs
pub(crate) fn resolve(target: LinkTarget, from: &ItemDto, …) -> Task<LinkResolution>; // Item(ItemLocator) | NotManaged | NotFound
// model/link_rewrite.rs
pub(crate) fn rewrite(body: &str) -> String;   // qualifying links → [[mh-link:<target>|<text>]] (R-31)
// services/render_prep.rs
pub(crate) struct Prepared { pub text: String, pub image_srcs: Vec<String> }
pub(crate) fn prepare(body: String, cx: &App) -> Task<Prepared>; // rewrite, image sources, warm_parse (R-7)
```

- [ ] **Step 1: Tests first.**
  - `model/links_tests.rs`:
    - `titles_brackets_fragments_and_percent_escapes_are_stripped`
    - `the_fragment_and_scheme_are_split_before_decoding` (`a%23b.md`,
      `a%3Ab.md`)
    - `schemes_other_than_http_https_mailto_are_unsafe`
    - `mailto_keeps_only_subject_and_body`
    - `backslashes_drive_letters_and_double_slashes_are_refused`
    - `dot_dot_past_the_root_is_not_found_with_no_fallback`
    - `nearest_counts_folder_steps_and_breaks_ties_by_path`
    - `caret_and_refs_wiki_targets_are_ignored`
  - `services/links_tests.rs`, over real repositories:
    - `an_exact_path_in_the_items_context_wins_over_primary`
    - `an_exact_path_on_primary_opens_its_tab`
    - `a_missing_path_falls_back_to_the_nearest_same_name_and_says_so`
    - `a_title_link_resolves_closest_first`
    - `a_link_to_a_non_item_path_is_not_found`
    - `an_unresolved_link_says_not_found`
  - `model/link_rewrite_tests.rs`:
    - `relative_and_mailto_links_become_mh_links_and_http_links_stay`
    - `every_mailto_destination_is_reduced_even_when_not_rewritten`
      (formatted text, reference definitions, images)
    - `links_with_formatted_text_pipes_brackets_or_references_stay`
    - `a_users_own_mh_link_is_a_path_link`
    - `the_rewrite_never_reaches_a_save`
  - `services/render_prep_tests.rs`:
    `a_large_body_is_prepared_off_the_ui_thread_and_warmed_as_rewritten`
  - **Handler tests.** Feed `EditorEvent::OpenLink` and `OpenWikiLink`
    straight to the editor host, and call the read view's `on_wiki_link`
    handler directly. Try unsafe targets first, then safe ones, and check
    `opened_url()` after each.
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands links
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands link_rewrite
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands render_prep
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::item
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 7: Local Images

**Interfaces:**

```rust
// model/resources.rs
pub(crate) enum ImageKind { Png, Jpeg, Gif, Webp, Bmp }
pub(crate) fn sniff(head: &[u8]) -> Option<ImageKind>;                   // magic bytes; SVG → None
pub(crate) fn contained(root: &Path, candidate: &Path) -> Option<PathBuf>; // canonical, inside, not .git or another worktree
pub(crate) const IMAGE_SIZE_CAP: u64 = 20 * 1024 * 1024;
// services/images.rs
pub(crate) struct ImageCache { /* src → Pending | Loaded(Arc<RenderImage>, f32, f32) | Failed */ }
```

- [ ] **Step 1: Tests first** (Review Focus 5).
  - `model/resources_tests.rs`, over temporary trees:
    - `dot_dot_symlink_escape_git_and_other_worktrees_are_refused`
    - `a_case_variant_of_git_is_refused`
    - `an_image_inside_a_context_worktree_is_allowed` (the check is relative
      to the tree root)
    - `a_fifo_or_directory_is_never_opened`
    - `a_handle_that_is_not_the_checked_file_is_refused`
    - `svg_named_png_is_not_an_image`
    - `oversized_files_are_not_read`
    - `each_supported_kind_is_sniffed`
    - `a_many_frame_gif_decodes_only_its_first_frame`
    - `an_image_over_4096_pixels_is_refused`
    - `the_per_tab_budget_stops_further_decodes`
  - `services/images_tests.rs`:
    - `a_local_png_loads_off_the_ui_thread_and_notifies`
    - `an_https_src_is_never_loaded`
    - `nearest_name_finds_an_image_in_docs`
    - `the_cache_clears_on_reload`
  - View tests:
    - `the_editor_provider_returns_the_image_once_loaded`
    - `the_read_view_inline_image_answers_from_the_cache`
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands resources
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands images
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::item
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Part A Checkpoint

- [ ] **Step 1: Merge.** Run the merge-tree check, and merge main if it
  moved.
- [ ] **Step 2: Run the desktop binary's tests in full.**
  - `--features desktop-test-support --bin manyhands`;
  - the desktop-only check and clippy;
  - the CLI isolation script.
- [ ] **Step 3: Review.** A fresh reviewer reviews Part A against the N
  table, decisions 1, 6, 8, 9 and 10, and contracts 2, 3, 5, 12 and 13.
- [ ] **Step 4: Ticket comment.**

---

# Part B: Editing, Saving And Review

## Task 8: New Intents

**Interfaces:** eight `Intent` variants.
- `ContextPrepare { root, kind, id, intent }`
- `ContextRelease { root, kind, id }`
- `DocumentCreate { root, id, path, title, body }`
- `TicketCreate { root, id, title, ticket_type, status, project, team, body, deps, parent }`
- `DocumentMove { root, id, observation, destination_path, destination_observation }`
- `FolderCreate { root, path }`
- `DocumentRepair { root, path, observation, source }`
- `TicketRepair { root, path, observation, source }`

Their properties:
- `confirmed()` is true only for the repairs.
- `changes_items()` is true for all eight.
- Labels: "Prepare to edit", "Release work context", "Create document",
  "Create ticket", "Move document", "Create folder", "Repair document" and
  "Repair ticket".
- `TicketCreate` builds with `initials: None`.
- Each variant carries the dead-code allowance, naming its task.
- `ReadKind` gains `NewId`, `ObservePath` and `RelationshipCheck`.
  `Documents` already exists.

- [ ] **Step 1: Apply E-11 to E-14 first.** The count becomes 29. Run them:
  they fail to compile.
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands services::requests
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 9: Edit, Provisioning And Release On Leave

**Interfaces:**

```rust
pub(crate) enum Phase { Viewing, Preparing(DesktopRequestId), Loading, Editing, Saving, Review }
pub(crate) fn base_for_edit(item: &ItemDto) -> Option<DraftBase>; // body through editor_text::load (R-11)
pub(crate) struct EditSession { provisioned: bool, saved: bool, /* … */ }
```

- [ ] **Step 1: Tests first.**
  - `edit_prepares_the_context_with_progress_before_the_editor_opens`
  - `edit_of_a_closed_ticket_is_not_offered`
  - `edit_is_unavailable_while_a_stored_draft_exists` (R-35)
  - `edit_of_an_uncommitted_primary_file_is_refused_and_nothing_is_made`
    (Review Focus 3, with a snapshot)
  - `a_discovery_pending_context_waits_for_resume`
  - `closing_the_tab_while_preparing_opens_no_editor`
  - `the_first_real_edit_begins_a_draft_and_later_edits_update_it`
  - `an_unavailable_store_is_said_before_editing`
  - `a_failed_flush_while_editing_stays_visible`: make the drafts folder
    read-only during editing; the failure shows, and typing continues.
  - `done_after_an_unused_edit_releases_the_context`
  - `discard_after_an_unused_edit_releases_the_context`
  - `closing_a_clean_tab_after_an_unused_edit_releases_the_context`
  - `a_session_that_saved_keeps_its_context`
  - `done_after_an_edit_on_an_existing_context_releases_nothing`
  - `another_draft_of_the_item_prevents_release` (R-37)
  - `an_unavailable_draft_listing_prevents_release`
  - `discarding_a_restored_edit_draft_releases_its_context` (R-45)
  - `a_metadata_change_alone_makes_leaving_ask`
  - `closing_the_tab_while_preparing_releases_the_context_it_made`
  - `leaving_a_dirty_editor_with_no_stored_draft_asks_first` (R-41: an
    unavailable store, and a first write held)
  - `discard_is_disabled_while_a_save_is_in_flight`
  - `a_stranded_context_can_be_released_from_the_tab` (R-40)
  - `undo_reaches_the_draft` (R-12)
  - `a_large_body_opens_in_the_editor` (Review Focus 4)
  - Apply E-6's first half: Edit leaves `LATER`.
- [ ] **Step 2: Implement.** Remove D2's allowances on `begin` and `edit`,
  which this task calls first.
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands edit
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::item
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands model::item
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands keyboard_tests
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 10: Save, States And Done

```rust
pub(crate) enum SaveState { Unsaved, SavedLocally { publication_pending: bool }, DiscoveryPending,
                            CheckpointFailed, Changed, ReviewBeforeSaving, Refused(ResultCode) }
pub(crate) fn save_state(code: ResultCode, effects: &Effects, has_publication_remote: bool) -> SaveState;
```

- [ ] **Step 1: Tests first.**
  - `save_state_maps_every_row_of_the_design_table`
  - `save_is_offered_only_when_dirty_and_not_while_saving`
  - `a_document_save_commits_once_and_reads_saved_locally`
  - `the_saved_file_keeps_every_untouched_body_byte`: a typed edit and a
    toolbar Bold over a real repository; the body is compared byte for
    byte outside the edit and the N set; front matter by value.
  - `a_ticket_save_keeps_unknown_metadata_by_value`
  - `a_normalized_item_opened_and_done_writes_nothing` (Review Focus 1):
    the canonical bytes of every branch are unchanged, and there is no new
    commit.
  - `an_edit_undone_to_the_baseline_offers_no_save_anywhere`
  - `a_pending_checkpoint_offers_retry_and_one_commit_follows` (a
    `FailOnce` checkpoint point)
  - `discovery_pending_offers_resume`
  - `already_applied_reads_saved_locally_and_retires`
  - `done_with_a_draft_offers_the_close_choices`
  - `secondary_s_saves_and_secondary_shift_m_switches_mode`
  - `publication_pending_is_read_after_a_save_not_on_load`
- [ ] **Step 2: Implement.** Remove D2's allowance on `save`.
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands edit
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands drafts
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::tabs
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 11: Restored Drafts, Close-Dialog Save And Late Results

- [ ] **Step 1: Apply E-8 and E-9, and write the new tests first.**
  - `continue_editing_loads_the_drafts_text_dirty_on_its_own_base_and_saves`
  - `continue_editing_a_draft_based_on_primary_takes_the_contexts_token_when_the_source_matches`
    (R-42)
  - `continue_editing_opens_the_review_when_the_source_differs`
  - `save_from_a_restored_draft_prepares_the_context_first`
  - `save_in_the_close_dialog_saves_then_closes`
  - `a_refused_save_from_the_close_dialog_keeps_the_tab`
  - `keep_draft_is_still_the_first_tab_stop_of_the_close_dialog` (R-16)
  - `typing_during_a_save_while_a_tab_read_lands_keeps_the_newer_text`
    (Review Focus 2; `DelayingBackend` holds the save; the read is
    `ReadKind::Item`)
  - `a_late_read_does_not_replace_a_dirty_editor`
  - `a_clean_editor_reloads_and_keeps_its_cursor` (R-19)
  - `normalization_alone_reads_current_on_the_drafts_page` (R-11)
- [ ] **Step 2: Implement.** The new close-dialog tests live in
  `view/tabs_tests.rs`, beside D2's. The implementation includes:
  - `workspace/content.rs`'s close-choice match;
  - `editor_text::normalized` at every desktop `classify` call;
  - R-38's placement.
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::item
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::tabs
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands drafts
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands keyboard_tests
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 12: Stale-Edit Review

- [ ] **Step 1: Apply E-19, and write the tests first.** They cover every row
  of the design's review outcome table (R-44). A row that answers
  differently stops the task for a ruling, and the table is not edited to
  fit. The tests:
  - `cancel_after_taking_theirs_on_the_body_leaves_the_draft_file_byte_identical`
  - `cancel_after_editing_the_body_restores_mine`
  - `switching_the_body_choice_after_an_edit_asks_first`
  - `an_edited_body_result_is_what_save_writes`
  - `a_stale_save_opens_the_review_and_leaves_the_draft_file_unchanged`
  - `one_sided_changes_default_and_both_changed_fields_must_be_chosen`
  - `taking_theirs_for_the_title_and_mine_for_the_body_saves_exactly_that`
  - `a_ticket_change_by_the_second_actor_is_reviewed`
  - `cancel_leaves_draft_and_base_unchanged`
  - `a_second_change_during_review_reopens_it_on_save` (two different
    `SaveDocumentAs` bodies)
  - `a_body_changed_only_by_them_defaults_to_theirs`
  - `a_body_changed_by_both_must_be_chosen_before_use`
  - `an_unverified_base_opens_the_review_before_any_save` (R-36)
  - `the_d2_privacy_sweep_still_holds_with_the_review_open` (R-38)
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands review
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands drafts::privacy_tests
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Part B Checkpoint

- [ ] **Step 1: Merge, and run the desktop binary's tests in full,** with
  `--test drafts`, the desktop-only check and clippy.
- [ ] **Step 2: Review.** A fresh reviewer, with the adversarial brief on
  saves, late results, release on leave and the review. Contracts 4, 6, 7
  and 8.
- [ ] **Step 3: Ticket comment.**

---

# Part C: Metadata, Relationships And Creation

## Task 13: Metadata Controls

- [ ] **Step 1: Tests first.**
  - `each_named_control_edits_its_field_and_marks_the_tab_dirty`
  - `id_short_code_closure_and_unknown_metadata_are_not_editable`
  - `a_status_of_closed_saves_as_ordinary_text_and_the_ticket_stays_open`
  - `an_emptied_project_is_saved_as_absent` (R-27)
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands metadata
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::item
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 14: Dependencies, Parent And The Cycle Check

- [ ] **Step 1: Tests first,** over real tickets.
  - `a_dependency_is_chosen_from_the_list_and_by_short_code`
  - `a_parent_is_changed_and_cleared`
  - `a_proposed_dependency_cycle_is_explained_with_its_tickets_and_save_is_unavailable`
  - `a_proposed_parent_cycle_is_explained`
  - `an_unresolved_dependency_is_shown_and_saves`
  - `an_incomplete_index_adds_its_note`
  - `a_late_check_does_not_replace_a_newer_one`
  - `deps_are_kept_in_id_order_and_an_unchanged_relationship_is_sent_as_unchanged`
  - `a_library_relationship_cycle_answer_is_explained_the_same_way`
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands relations_edit
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::item
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 15: New-Item Drafts

**Interfaces:**

```rust
// src/drafts/compare.rs
pub enum TargetState { /* … existing … */ NotCreated }
pub fn is_new_item_base(base: &DraftBase) -> bool;   // observation and source both None (R-20)
// services/drafts/create.rs
impl DraftRegistry {
    pub(crate) fn begin_new(&mut self, target: DraftTarget, base: DraftBase, current: DraftFields, cx: &mut Context<Self>) -> Task<Result<DraftId, DraftErrorKind>>;
    pub(crate) fn create(&mut self, id: DraftId, cx: &mut Context<Self>) -> Task<Option<DesktopRequestId>>;
}
```

- [ ] **Step 1: Tests first.**
  - In `src/drafts/compare_tests.rs`, new tests only:
    - `a_new_item_draft_of_a_missing_item_is_not_created_and_restorable`
    - `a_new_item_draft_whose_item_exists_with_other_fields_is_unverified`
    - `a_new_item_draft_whose_item_exists_with_equal_fields_is_already_saved`
    - `an_unverified_base_with_a_source_is_still_item_not_found_when_missing`
  - In `services/drafts/create_tests.rs`:
    - `create_completion_clears_the_marker_when_the_read_matches`
    - `create_completion_clears_the_marker_when_the_read_fails` (R-20b)
    - `a_retitle_between_create_and_read_reviews_with_theirs_as_default`
      (R-20b)
    - `the_new_item_draft_is_stored_before_context_prepare` (R-20)
    - `an_occupied_path_for_this_drafts_own_item_becomes_a_review`
    - `a_marked_draft_whose_item_exists_routes_to_edit_and_review` (R-42)
    - `continue_editing_a_not_created_draft_whose_item_appeared_routes_to_review`
    - `create_stops_before_provisioning_when_the_store_is_unavailable`
      (R-46)
    - `a_create_retried_after_a_lost_answer_creates_one_item` (worker
      `retry`)
    - `discarding_a_new_item_draft_releases_its_context`
    - `a_not_created_draft_restores_into_a_tab_with_no_item_read`
    - `continue_editing_a_not_created_draft_prepares_with_create`
- [ ] **Step 2: Implement.**
  - `for_drafts` routes the create intents' answers.
  - The banner and restore handle `NotCreated`.
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --lib drafts::
devenv shell -- cargo test --locked --test drafts
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands drafts
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands draft_comparison
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::item
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 16: Create Dialogs

- [ ] **Step 1: Tests first** (`view/create_tests.rs`).
  - `new_document_prepares_and_opens_a_draft_already_in_the_store`
  - `the_path_must_be_under_docs_end_md_and_be_free`
  - `save_creates_the_document_once_and_the_tab_becomes_an_edit_session`
  - `new_ticket_creates_with_its_fields_and_dependency`
  - `an_occupied_path_at_save_keeps_the_draft`
  - `initials_required_points_to_settings`
  - `secondary_n_opens_the_shown_lists_dialog`
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands create
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::documents
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::tickets
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands keys
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands keyboard_tests
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Part C Checkpoint

- [ ] **Step 1: Merge, and run the desktop binary's tests in full,** with
  `--test drafts` and `--lib drafts::`.
- [ ] **Step 2: Review.** Contracts 9, 10 and 11 for create; R-20 and
  R-20b; release on discard.
- [ ] **Step 3: Ticket comment.**

---

# Part D: Moves, Folders, Repair And The Gate

## Task 17: Move And New Folder

- [ ] **Step 1: Apply E-10, and write the tests first.**
  - `move_commits_both_paths_and_the_tab_follows`
  - `a_collision_is_refused_and_explained_and_the_dialog_stays`
  - `a_stale_source_is_refused`
  - `move_is_unavailable_with_an_unsaved_draft`
  - `a_move_retried_after_a_lost_answer_shows_the_moved_document` (known
    defect `01M4PB80T7`)
  - `new_folder_opens_its_dialog` (E-10)
  - `new_folder_creates_and_explains_local_folders_and_lists_it`
  - `a_folder_retry_reads_folder_exists`
  - `an_invalid_folder_name_is_refused`
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands move_document
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands new_folder
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::documents
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::item
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 18: Repair

- [ ] **Step 1: Apply E-6's second half and E-7, and write the tests
  first.**
  - `a_nonconforming_document_is_repaired_after_a_preview_naming_the_new_id`
  - `a_nonconforming_ticket_is_repaired`
  - `a_stale_preview_is_refused_and_prepare_again_is_offered`
  - `an_ambiguous_identity_blocks_repair_with_its_reason`
  - `a_repair_retried_after_a_lost_answer_answers_once`
  - `closing_or_quitting_with_edited_repair_text_asks_first`
  - `a_non_utf8_file_says_it_cannot_be_repaired_here`
  - `the_repair_view_never_passes_through_the_markdown_editor`
- [ ] **Step 2: Implement.** This includes `view/confirm.rs`'s adoption
  line.
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands repair
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands model::item
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::item
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands confirm
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 19: Keyboard And Privacy

- [ ] **Step 1: Keyboard tests.** In `view/keyboard_tests/editing.rs`,
  drive these by keystroke alone:
  - Edit, mode switch, formatting and Save;
  - the review, metadata and both pickers;
  - Create, Move, New folder and Repair.

  Escape and focus return are checked wherever the editor does not hold
  focus.
- [ ] **Step 2: Privacy sweep.** In `view/editing_privacy_tests.rs`, a
  sentinel body, link target and image path must appear in no operation
  entry, notification, dialog message, error or `Debug` output. Cover:
  - save, stale save and `invalid_input`;
  - create, move, repair and release.
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands keyboard_tests
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands privacy
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 20: Reviews, Gate And Review-Ready

- [ ] **Step 1: Whole-branch reviews.** A spec review and a code-quality
  review. Fix what they find, re-run each fixed task's Verify lines, and
  commit.
- [ ] **Step 2: Native checklist.** Ask the owner to run the design's Linux
  Wayland checklist and the desktop smoke test
  `devenv shell -- cargo run --locked --features desktop --bin manyhands`.
  Record the results in the Cycle's table.
- [ ] **Step 3: Full gate, last.** Run it detached, with a two-hour budget,
  and record the counts in the ledger.

```sh
git fetch origin && git merge origin/main
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo check --locked --features desktop
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo test --all-features --locked
devenv shell -- sh scripts/check-cli-no-gpui.sh
devenv shell -- cargo run --locked --bin manyhands-cli
git merge-tree --write-tree HEAD origin/main
```

- [ ] **Step 4: Review-ready.**
  - Fill the Cycle's acceptance table: the tests per row, "Held (Linux)",
    and the native run left to the post-merge workflow.
  - Write the review-ready ticket comment, and push the ticket branch.
  - A PR, a merge, closing the ticket and native CI each need the owner's
    own go-ahead. The ticket is closed only after review approval, in the
    final push before merge.

## Plan Self-Review

| Acceptance row or ruling | Task |
| --- | --- |
| Context prepare; R-1 to R-5 | 1 |
| Context release; R-32, R-43 | 2 |
| E-21, E-22 | 2 |
| Load normalization; round trips | 3, 5 |
| Read view; R-6, R-7 (warm) | 4 |
| Shared undo; not dirty (editor); R-8, R-9, R-12 | 5 |
| Links; R-31; background preparation (R-7) | 6 |
| Images; R-33, R-34, R-39 | 7 |
| Edit; release on leave; R-12, R-13, R-35, R-37, R-40, R-41, R-45 | 9 |
| Save; states; untouched bytes; not dirty (repository); R-11, R-14, R-15 | 10 |
| Late results; recovery; R-16, R-17, R-19, R-38, R-42 (Continue editing) | 11 |
| Stale edit; R-18, R-36, R-44 | 12 |
| Metadata; R-27 | 13 |
| Relationships; R-26 | 14 |
| Create (drafts); R-20, R-20b, R-42 (routing), R-46 | 15 |
| Create (dialogs); R-21, R-22 | 16 |
| Move; folders; R-23, R-24 | 17 |
| Repair; R-25 | 18 |
| Keyboard; privacy | 19 |
| Native checklist; native run; CLI isolation; regression | 20 |
| E-1 to E-5 | 4 |
| E-6 | 9, 18 |
| E-7 | 18 |
| E-8, E-9 | 11 |
| E-10 | 17 |
| E-11 to E-14 | 8 |
| E-15 to E-18 | 1, 2 |
| E-19 | 12 |
| E-20 | 2 |

**Interfaces** are defined where they are produced:

| Interface | Task |
| --- | --- |
| The context DTOs and inputs | 1, 2 |
| `editor_text` | 3 |
| `EditorHost` | 5 |
| Link and image models | 6, 7 |
| Intents | 8 |
| `Phase`, `EditSession` | 9 |
| `SaveState` | 10 |
| `NotCreated`, `begin_new`, `create` | 15 |

**Placeholders.** None intended. Task 0's sweep is how "no other test edits"
is proven.
