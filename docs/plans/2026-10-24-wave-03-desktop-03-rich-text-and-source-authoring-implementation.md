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
desktop, in the owner's item-view design, with `zorite-editor`. Links and
local images work, saves go through D2's drafts and F2, every save state is
told truthfully, and unused work contexts are released.

**Architecture:**
- **Part 0** adds two F2 commands: `context prepare`, over the domain's
  `prepare_context`, and `context release`, over a new domain operation.
- **Part A** rebuilds the item tab as the design's frame: tab strip, fixed
  header, one scroll region with the page and the discussion, and a details
  sheet. Bodies and comments are shown in read-only
  `zorite_editor::EditorState`s, guarded by the app (decision 17).
- **Parts B–E** host a separate edit editor per editing tab with a toolbar,
  resolve links and images, feed D2's `DraftRegistry`, edit front matter in
  the sheet, and submit new `Intent`s through D1's worker.
- **`src/drafts/`** gains one target state.

**Tech stack:**
- Rust 2024.
- `gpui-kit` 0.6.6, through `gpui_kit::*` only.
- `zorite-editor` `=0.10.0`, `zorite-markdown` `=0.9.0` with no features,
  and `image` `=0.25.10`. All three are already locked; no new package.
- Devenv.

**Spec:** [Cycle](../Cycles/wave-03-desktop-03-rich-text-and-source-authoring.md)
and [design](2026-10-24-wave-03-desktop-03-rich-text-and-source-authoring-design.md).
Executors read both, and the design handoff `docs/design/item_view/` for
exact sizes and copy. "Fact n", "N-n", "R-n" and "E-n" refer to the
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

**Drafts module.** `src/drafts/` changes only in Task 22 (`NotCreated`,
R-20), and imports nothing from `gpui_kit`.

**Dependencies.** `Cargo.toml` changes only in Task 3. `Cargo.lock`
changes only there and gains no new `[[package]]`.

**GPUI.**
- Use it only through `gpui_kit::*`, and never as a `use gpui_kit::*` glob.
- `zorite_editor`, `zorite_markdown` and `image` are used only under
  `src/desktop/`.

**The design handoff.** Sizes, colours and copy come from
`docs/design/item_view/` and `docs/design/04-item-area.md`, except where
the design's Design Deviations table says otherwise. No task edits
`docs/design/`.

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
- The test shell (`view/harness.rs`) calls `zorite_editor::bind_keys` and
  D3's editor bindings from Task 3 on, as startup does.
- Never send Ctrl-Cmd-Space.
- A view test that waits on the worker calls
  `cx.executor().allow_parking()`.

**Test runs.**
- `cargo test` takes one positional filter, and each Verify line uses one.
- Every Verify step checks that the run reported more than zero tests.
- Within a task, run only the tests its Verify lines name.
- The full suite runs in Task 0 (entry-gate item 7) and once in Task 27.
  This follows the owner's standing rule: targeted tests between tasks,
  the full suite once at the end.
- Part checkpoints run the Part's targets in full: the library mutation
  targets for Part 0, and the desktop binary's tests and `--test drafts`
  for Parts A to E.

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

**Existing tests.** They change only as named edits E-1 to E-29. Any other
change to a pre-existing test, even a new lint, stops for the owner.

**Commits.** Commit per task with a conventional message ending
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Merging.**
- Merge `origin/main` at work boundaries; never rebase.
- Before each Part checkpoint, run `git fetch origin` and
  `git merge-tree --write-tree HEAD origin/main`.
- E-15 to E-17 count "main's at the merge, plus D3's".

**Ticket records** (`AGENTS.md`, after C3).
- After each task, add a ticket comment as a canonical file under
  `.manyhands/comments/01M4CC0VN0MFTGKQ4HGTTC5H63/`, until C4's
  `comment add` exists. Take the ULID from
  `devenv shell -- cargo run --locked --bin manyhands-cli -- id new` and
  `created_at` from the UTC clock.
- Each comment records the commit, the tests run with their counts and
  results, and any ruling.
- An edit to the ticket's own `ticket.md` is committed on the ticket branch
  first, then saved with `manyhands-cli ticket save` from an up-to-date main
  checkout. A new ticket (a `file-ticket` gap, or decision 14's follow-up)
  is made with `manyhands-cli ticket create`, with the owner's go-ahead.
- Closure stays a canonical ticket-file edit until C5, after review
  approval, in the final push before merge.
- Part checkpoints add the review and its fixes. The owner may split the
  Cycle at any checkpoint (decision 5): each checkpoint notifies the owner
  and continues unless the owner has asked to stop there. Part 0's review
  must pass before Task 3.

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
   words-mixed `$$` pair opens in the read view and in the editor, and is
   not dirty. Done changes no canonical byte and makes no commit. Pinned in
   Tasks 3 and 15.
2. **Typing during a save while a tab read lands.** Newer text stays in the
   editor and in the draft; only the saved revision retires. Pinned in
   Task 18.
3. **Edit on a hand-edited, uncommitted primary file.** Refused with
   `worktree_not_clean`; no branch or worktree is made; the editor never
   opens. Pinned in Tasks 1 and 14.
4. **A very large body**, 200 KiB. It opens in the read view, Edit opens
   it, and typing stays responsive. The tests are smoke tests; lag is
   checklist step 10. Pinned in Tasks 7 and 14.
5. **Every way the read view could be edited**: typing, Enter, Backspace,
   Delete, paste, middle-click paste, cut, the formatting keys, undo, a
   checkbox click, a right-click, an input-method commit or composition,
   and a table drag. The text shown never changes, no draft begins, and no
   mutation runs. Pinned in Tasks 3 and 7.

Crafted link and image paths (`..`, a symbolic link out of the repository,
`.git`, another item's worktree, SVG named `.png`, `file:` and
`javascript:`) are pinned by Tasks 11 and 12's own tests.

## File Map And Dependency Order

| Path | Task | Change |
| --- | --- | --- |
| `docs/plans/…-execution.md` | 0 | Execution ledger. |
| `src/repository/mutation/{command,mod,dto}.rs`, `bind/{mod,context}.rs`, `schemas/v1/context_mutation.schema.json`, `tests/mutation_bridges{.rs,/context.rs}`, `tests/fixtures/mutation_v1/context_*.json`, `tests/support/{conformance,mutation_golden,mutation,repair}.rs`, `tests/mutation_contract.rs` | 1 | `context prepare`; E-15 to E-18. |
| `src/repository.rs`, `src/repository/{context_release,cleanup,recovery}.rs`, `bridges/abandon.rs`, `read/{dto,status,status_tests}.rs`, three operation schemas, `tests/read_contract.rs` | 2 | `context release`; E-20 to E-22. |
| `Cargo.toml`, `Cargo.lock`, `src/desktop/model/editor_text.rs`, `src/desktop/view/editor.rs`, `view/editor/{read_only,keys}.rs`, `desktop/mod.rs`, `view/harness.rs`, `scripts/check-cli-no-gpui.sh`, `tests/fixtures/editor/*` | 3 | Dependencies, editor bindings, feasibility, the read-only guard, load normalization. |
| `src/desktop/assets.rs`, `desktop/mod.rs`, `view/item/tokens.rs`, `view/tabs.rs`, `view/tabs_tests.rs`, `view/settings/mod_tests.rs` | 4 | Icons, item colours, the tab strip; E-27, E-28, E-29. |
| `view/item.rs`, `view/item/page.rs`, `view/scroll.rs`, `view/scroll_tests.rs`, `view/workspace/{render,content}.rs` | 5a | The layout, the item's scroll region and Back to top; E-25. |
| `view/item/{header,chips,pill,banners}.rs`, `model/item.rs`, `model/item_tests.rs`, `view/item_tests.rs` | 5b | The header, chips, pill and banners; E-6 (labels), E-7 (first half). |
| `view/item/sheet.rs`, `view/item/sheet/{ticket,document,facts}.rs`, `view/relationships.rs`, `view/workspace/render.rs`, `view/item_tests.rs`, `view/relationships_tests.rs`, `view/keyboard_tests.rs`, `view/keyboard_tests/actions.rs` | 6 | The details sheet while viewing; E-23, E-26. |
| `view/item/page.rs`, `view/editor/style.rs`, `view/item/source.rs`, `view/item_tests.rs`, `view/tabs_tests.rs` | 7 | The read view; E-1 to E-5. |
| `view/item/discussion.rs`, `model/discussion.rs`, `keys.rs`, `view/item_tests.rs` | 8 | The discussion; E-24. |
| `src/desktop/view/editor/host.rs`, `keys.rs` | 9 | The editor host. |
| `model/markdown_edit.rs` | 10a | The toolbar's host edits. |
| `view/editor/toolbar.rs` | 10b | The toolbar. |
| `src/desktop/model/links.rs`, `services/links.rs`, `view/item/banners.rs` | 11 | Links. |
| `src/desktop/model/resources.rs`, `services/images.rs` | 12 | Images. |
| `src/desktop/services/{requests,mod}.rs`, `requests/build.rs`, `services/requests_tests.rs` | 13 | Eight intents; E-11 to E-14. |
| `src/desktop/model/edit.rs`, `view/item/edit.rs`, `view/item/pill.rs`, `services/created_contexts.rs`, `model/item.rs`, `model/item_tests.rs`, `view/editor/toolbar.rs`, desktop `classify` call sites | 14 | Edit, provisioning and drafts, `CreatedContexts`, normalized classification, the draft indicator; E-6, first half. |
| `view/item/edit.rs`, `view/item/banners.rs`, `view/editor/host.rs`, `services/drafts.rs`, `services/drafts/save.rs` | 15 | Save, `DraftSaved` and the session after a save, and states. |
| `model/edit.rs`, `view/item/edit.rs`, `view/item/banners.rs`, `view/drafts/row_actions.rs` | 16 | Done and the kept session. |
| `model/edit.rs`, `view/item/edit.rs`, `services/created_contexts.rs`, Drafts-page discard call sites | 17 | Release on leave. |
| `view/item/draft.rs`, `view/close_draft.rs`, `view/workspace/content.rs`, `view/item/reads.rs`, `services/drafts/restore.rs`, `view/drafts.rs`, `view/item/draft_tests.rs`, `view/tabs_tests.rs` | 18 | Restored drafts, close-dialog Save, late results; E-8, E-9. |
| `view/item/review.rs`, `test_support/drafts.rs` | 19 | Stale-edit review; E-19. |
| `view/item/sheet/fields.rs`, `model/metadata.rs` | 20 | Named controls in the sheet. |
| `view/item/relations_edit.rs`, `model/relations_edit.rs`, `view/item/sheet/ticket.rs`, `model/editor_text.rs` | 21 | Relationships. |
| `src/drafts/compare.rs`, `services/drafts/create.rs`, `services/mod.rs` (`for_drafts`), `view/item/draft.rs`, `view/item/reads.rs`, `view/draft_comparison.rs` | 22 | New-item drafts. |
| `view/create.rs`, `view/documents.rs`, `view/lists.rs`, `keys.rs` | 23 | Create dialogs. |
| `view/move_document.rs`, `view/new_folder.rs`, `view/documents.rs`, `view/documents_tests.rs`, `view/item/sheet/document.rs` | 24 | Move and New folder; E-10. |
| `view/item/repair.rs`, `view/confirm.rs`, `model/item.rs`, `view/item/banners.rs`, `model/item_tests.rs`, `view/item_tests.rs`, quit path | 25 | Repair; E-6, second half; E-7, second half. |
| `view/keyboard_tests/{frame,editing}.rs`, `view/editing_privacy_tests.rs` | 26 | Keyboard and privacy. |
| Cycle table, ledger, ticket | 27 | Gate and review-ready. |

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
  - Run `git diff --stat 6f1463c HEAD -- src/repository.rs src/repository src/results.rs src/drafts src/desktop tests/support tests/read_contract.rs schemas Cargo.toml .github`.
  - Re-read each fact whose file changed, and update the design before
    coding if a cited signature moved.
  - Confirm that the design's Document Amendments are in the RFCs and the
    Wave. They are applied in the approval commit.

- [ ] **Step 3: Sweep for unlisted test edits.** Search every `*.rs` under
  `src/` and `tests/`, not only `*_tests.rs`, for:
  - `LATER`, `EDITOR_LATER`, `DRAFT_ACTIONS`, `NEW_FOLDER_UNAVAILABLE`,
    `SHOWN`;
  - `item-source`, `later-button`, `later-action`, `draft-later`,
    `close-draft-save`, `tab-shown`;
  - `provenance`, `copy-id`, `copy-slug`, `rel-`, `item-problem`,
    `comment-`, `"discussion"`, `"content"`;
  - `MutationDataDto::`, `OperationAction::`, `RepositoryOperation::`,
    `LocalStep::`, `TargetState::`;
  - `listed.len()`, `HOMES`, `DATA_ROOTS`, `CASES`, `LOCAL_ACTIONS`;
  - `CloseWithDraft::`, `content_scroll`, `ContentScroll`;
  - `strip(`, `comment_author`, `created_at`, `local_time`.

  Compare each hit with E-1 to E-29. A pre-existing test that will break
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

# Part A: The Item Frame

## Task 3: Dependencies, Editor Bindings, Feasibility, The Read-Only Guard And Load Normalization

**Interfaces:**

```rust
// src/desktop/model/editor_text.rs
pub(crate) fn crlf_to_lf(body: &str) -> String;          // N-1
pub(crate) fn load(body: &str) -> String;                 // N-1, then normalize_math_fences
pub(crate) fn normalized(item: &ItemDto) -> ItemDto;      // body through load (R-11)
// src/desktop/view/editor.rs
pub(crate) fn open_editor(body: &str, window: &mut Window, cx: &mut App) -> Entity<EditorState>;
pub(crate) fn step_over_blocks(editor: &Entity<EditorState>, event: &EditorEvent, cx: &mut App) -> bool; // R-66
// src/desktop/view/editor/keys.rs (R-8, R-59)
pub(crate) fn bind(cx: &mut App); // zorite_editor::bind_keys; cmd-u/ctrl-u → NoAction and
                                  // Unbind("zorite_editor::Dismiss") for escape, in "Editor"
// src/desktop/view/editor/read_only.rs (R-61)
pub(crate) struct ReadOnly { /* editor, shown: String, cursor */ }
impl ReadOnly {
    pub(crate) fn new(body: &str, window: &mut Window, cx: &mut Context<Self>) -> Self;
    pub(crate) fn editor(&self) -> &Entity<EditorState>;
    pub(crate) fn reload(&mut self, body: &str, cx: &mut Context<Self>); // R-19
    pub(crate) fn cursor(&self, cx: &App) -> usize;                     // R-65
}
impl Render for ReadOnly { /* capture_action, capture_key_down, right and middle press capture, revert backstop */ }
```

- [ ] **Step 1: Dependencies and bindings.**
  - `desktop = ["dep:gpui-kit", "dep:futures", "dep:zorite-editor", "dep:zorite-markdown", "dep:image"]`.
  - `image = { version = "=0.25.10", default-features = false, features = ["png", "jpeg", "gif", "webp", "bmp"], optional = true }`.
  - `editor-probe = ["desktop"]`.
  - `zorite-markdown = { version = "=0.9.0", default-features = false, optional = true }`.
  - Run `devenv shell -- cargo check --features desktop` once, without
    `--locked`.
  - Then check `git diff Cargo.lock`: only dependency lists may change. No
    new `[[package]]` and no second `gpui-pre`.
  - Call `view::editor::keys::bind` after `gpui_kit::init` in
    `desktop/mod.rs`, and in `view/harness.rs`'s `Shell::start`.
  - Widen `scripts/check-cli-no-gpui.sh` to also refuse `zorite`, `image`
    and `markdown` crates in the CLI's dependency tree (fact 80).

```sh
devenv shell -- cargo check --locked --features desktop
devenv shell -- cargo check --locked --features editor-probe --example editor_feasibility
devenv shell -- sh scripts/check-cli-no-gpui.sh
```

- [ ] **Step 2: Feasibility test (stop point).**
  `view/editor_tests.rs::a_real_editor_takes_typing_and_undo_in_a_headless_window`:
  1. open a test window through the shell, which binds the keys;
  2. create an `EditorState` with `"abc"`, then `set_cursor(3)`;
  3. focus it and draw a frame;
  4. `simulate_input("d")`: `text()` is `"abcd"`, and one `Changed` was
     seen;
  5. `simulate_keystrokes("ctrl-z")`: `text()` is `"abc"`.

  If it cannot pass with the window-level helpers, stop and ask the owner
  before Step 3.

- [ ] **Step 3: The read-only guard's tests first (stop point; Review Focus
  5).** In `view/editor/read_only_tests.rs`, over a real, focused, painted
  editor in a `ReadOnly` holding a body with a paragraph, a checkbox, a
  link, a table and a `$$` block. Each asserts that the text is unchanged
  and that the `ReadOnly` emitted nothing:
  - `typing_enter_backspace_and_delete_change_nothing`
  - `paste_and_cut_change_nothing_and_copy_still_copies`
  - `the_formatting_keys_and_undo_change_nothing`
  - `an_input_method_commit_is_reverted` (`replace_text_in_range` on the
    entity, fact 76)
  - `an_input_method_composition_is_reverted_and_leaves_no_marked_text`
    (`replace_and_mark_text_in_range`)
  - `a_checkbox_click_is_reverted` (the box found from `bounds_for_offset`
    and the gutter; fragile, and checklist step 2 also covers it)
  - `a_right_or_middle_press_is_stopped` (focus did not move)
  - `select_all_copy_and_the_arrows_work`
  - `the_caret_steps_over_math_and_property_blocks` (R-66)
  - `tab_and_shift_tab_move_focus_out`
  - `a_link_click_still_emits_open_link`

  If any cannot pass with the guard of design R-61, stop and ask the owner.
  The viewer comes back only by the owner's ruling.
- [ ] **Step 4: Fixtures and load tests first.**
  - Add the design's golden fixtures, with `.expected` files.
  - `editor_text_tests.rs`:
    - `each_fixture_loads_to_its_expected_file`
    - `each_exemption_loads_unchanged`
    - `load_is_idempotent`
    - `normalized_changes_only_the_body`
  - `editor_tests.rs`: `the_editor_reads_back_exactly_what_load_predicts`,
    over every fixture.
- [ ] **Step 5: Implement.** Items that only later tasks call carry the
  dead-code allowance:
  - `ReadOnly` names Task 7;
  - `open_editor` and `load` name Task 14;
  - `normalized` names Task 14.
- [ ] **Step 6: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands editor
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands read_only
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands keyboard_tests
devenv shell -- sh scripts/check-cli-no-gpui.sh
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 7: Commit** `Cargo.toml`, `Cargo.lock`, the script, the
  fixtures and the code, and write the ticket comment.

## Task 4: Icons, Item Colours And The Tab Strip

**Interfaces:**

```rust
// src/desktop/assets.rs (R-52)
pub(crate) struct DesktopAssets;           // the item view's icons first, then the Kit's Assets
// src/desktop/view/item/tokens.rs (R-51)
pub(crate) struct ItemColors { /* header_block, header_border, header_title, page, page_text, … */ }
pub(crate) fn item_colors(cx: &App) -> ItemColors;   // by cx.theme().is_dark()
```

- [ ] **Step 1: Apply E-27, E-28 and E-29, and write the tests first.**
  - `view/item/tokens_tests.rs`:
    `header_colours_follow_dark_and_light_and_the_page_stays_white`
  - `assets_tests.rs`: `every_item_view_icon_loads`, calling
    `DesktopAssets::load` directly for each name in the design's list.
  - `view/tabs_tests.rs`:
    - `each_tab_shows_its_type_short_code_and_title`
    - `a_document_tab_has_no_short_code`
    - `a_tab_with_a_stored_draft_shows_its_dot_and_is_named_unsaved`
    - `the_tooltip_names_the_repository_and_the_context_when_there_is_one`
    - `the_strip_does_not_wrap`
    - `only_the_shown_tab_carries_the_active_line` (E-27)
    - `the_focused_strip_marks_the_active_tab`
- [ ] **Step 2: Implement.** Register `DesktopAssets` in `desktop/mod.rs`.
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::tabs
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::settings
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands tokens
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands assets
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands keyboard_tests
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 5a: The Layout And The Item's Scroll Region

**Interfaces:**

```rust
// view/item/page.rs
pub(crate) struct Page { /* D1's content for now; the read editor from Task 7 */ }
// view/scroll.rs: ContentScroll tracks the shown item's region; the region gets the Kit's vertical scrollbar
```

- [ ] **Step 1: Apply E-25, and write the tests first**
  (`view/item/frame_tests.rs`).
  - `the_tab_strip_and_header_stay_in_place_while_the_page_scrolls`
  - `page_keys_scroll_the_items_region_and_settings_keep_theirs`
  - `the_page_fills_the_region_and_the_discussion_starts_below_it`
  - `the_region_shows_a_scrollbar`
  - `back_to_top_returns_and_focuses_the_page`
  - `opening_an_item_writes_nothing` stays as it is.
- [ ] **Step 2: Implement.** The tab strip, a header placeholder holding
  D1's identity line, then the region with D1's content and the discussion,
  whose header row gains Back to top now (E-25).
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::item
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::scroll
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::tabs
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::settings
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands keyboard_tests
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 5b: The Header, Chips, Pill And Banners

**Interfaces:**

```rust
// view/item/pill.rs
pub(crate) enum PillState { ViewingShared, ViewingPrivate, ViewingDraftKept, Preparing, Saving, Review,
                            EditingUnsaved, EditingSaved { not_shared: bool }, Closed, NeedsAttention }
pub(crate) fn pill_words(state: PillState) -> &'static str;
// view/item/banners.rs
pub(crate) enum Banner { /* the design's Banners table, one variant per row */ }
// view/item/chips.rs
pub(crate) enum ChipField { Type, Status, Project, Team, Parent, WaitingOn, Author, Date, Updated, All }
pub(crate) struct OpenDetails(pub ChipField);   // emitted; Task 6 opens the sheet
```

- [ ] **Step 1: Apply E-6's label change and E-7's first half, and write
  the tests first** (`view/item/header_tests.rs`).
  - `the_breadcrumb_names_the_repository_section_and_file`
  - `ticket_chips_show_type_status_project_team_and_parent`
  - `waiting_on_shows_only_when_blocked`
  - `document_chips_show_status_author_and_date_only_when_strings`
  - `updated_is_relative_to_changed_at`
  - `every_chip_emits_open_details_for_its_field`
  - `the_pill_reads_each_viewing_state`
  - `share_and_the_primary_action_are_disabled_with_their_note`
  - `the_status_lines_are_banners_with_their_actions` (Refresh, recovery,
    item not found; D1's IDs are kept)
  - `a_nonconforming_item_shows_needs_fixing_with_repair_disabled`
  - `tab_order_runs_strip_header_chips_page`
- [ ] **Step 2: Implement.** Rehome D1's lines as the design's table says.
  Details, relationships and source stay in the page until Tasks 6 and 7.
  `OpenDetails` carries the dead-code allowance naming Task 6.
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::item
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands model::item
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands keyboard_tests
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 6: The Details Sheet While Viewing

**Interfaces:**

```rust
// view/item/sheet.rs
pub(crate) fn open_details(pane: &Entity<ItemPane>, field: ChipField, window: &mut Window, cx: &mut App);
// workspace/render.rs: unless_dialog also refuses while window.has_active_sheet(cx) (R-60)
```

- [ ] **Step 1: Apply E-23 and E-26, and write the tests first**
  (`view/item/sheet_tests.rs`).
  - `each_chip_and_all_details_open_the_sheet_and_focus_its_field`
  - `the_sheet_is_read_only_while_viewing`
  - `ticket_details_show_every_row_of_the_design`
  - `document_details_show_status_date_and_author_as_the_library_gives_them`
  - `other_fields_are_listed_in_the_librarys_key_order_and_are_not_editable`
  - `the_four_relationship_groups_are_in_the_ticket_sheet` (R-54)
  - `escape_done_and_close_return_focus_to_the_chip`
  - `no_workspace_key_acts_behind_the_sheet` (R-60)
  - `the_inspect_link_opens_the_items_inspect_tab`
  - `closing_the_tab_by_its_action_closes_the_sheet` (a programmatic
    close; the backdrop takes clicks)
  - `a_refresh_updates_the_open_sheet_in_place`
  - `the_sheet_writes_nothing`
- [ ] **Step 2: Implement.** Move D1's details, provenance, metadata,
  problems and relationships panel into the sheet.
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands sheet
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::item
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::relationships
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::scroll
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands keyboard_tests
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 7: The Read View

**Interfaces:**

```rust
// view/item/page.rs: Page holds read: Entity<ReadOnly>
// view/editor/style.rs
pub(crate) fn page_style(cx: &App) -> SyntaxStyle;   // all 25 fields from the page tokens
// view/editor.rs
pub(crate) fn reveal_caret(editor: &Entity<EditorState>, scroll: &ContentScroll, cx: &mut App); // R-67
```

- [ ] **Step 1: Apply E-1 to E-5, and write the tests first**
  (`view/item/page_tests.rs`).
  - `a_document_body_renders_in_a_read_editor`
  - `a_ticket_body_renders`
  - `select_all_and_copy_put_the_markdown_on_the_clipboard`
  - `typing_and_paste_in_the_tab_change_nothing_and_begin_no_draft`
  - `tab_moves_focus_out_of_the_read_editor`
  - `a_click_below_the_text_puts_the_caret_at_the_end`
  - `arrowing_past_the_fold_keeps_the_caret_in_view` (R-67)
  - `a_click_deep_in_a_long_body_does_not_jump_to_its_top` (R-67)
  - `tabbing_onto_a_read_editor_below_the_fold_reveals_its_caret` (R-67)
  - `a_clean_reload_keeps_the_cursor` (R-19)
  - `a_non_utf8_item_says_so`
  - `the_inspect_tab_still_shows_the_exact_file`
  - `a_large_body_opens` (200 KiB; a smoke test, Review Focus 4)
  - `viewing_changes_nothing_in_the_repository` (`RepositorySnapshot`
    before and after)
- [ ] **Step 2: Implement.** Remove the item tab's source textarea and its
  toggle (R-64). Inspect tabs keep `view/item/source.rs`.
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::item
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::tabs
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands read_only
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands keyboard_tests
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 8: The Discussion

**Interfaces:**

```rust
// model/discussion.rs
pub(crate) struct Thread { pub root: usize, pub replies: Vec<Reply> }   // indexes into CommentListDto.items
pub(crate) struct Reply { pub ix: usize, pub replying_to: Option<String> } // set when depth > 1 (R-53)
pub(crate) fn threads(list: &CommentListDto) -> Vec<Thread>;
pub(crate) fn display_name(author: Option<&str>) -> String;              // drops "<email>"
pub(crate) fn avatar_color(name: &str) -> Hsla;                          // stable; white text ≥ 4.5:1
pub(crate) fn comment_time(at: &str, now: OffsetDateTime, offset: UtcOffset) -> String; // "Oct 22, 10:14" | "Today, 15:36"
```

- [ ] **Step 1: Apply E-24, and write the tests first.**
  - `model/discussion_tests.rs`:
    - `threads_keep_the_librarys_order`
    - `a_deeper_reply_names_who_it_replies_to`
    - `display_names_drop_the_email`
    - `avatar_colours_are_stable_and_keep_contrast`
    - `times_read_today_or_the_date_at_the_given_offset`
  - `view/item/discussion_tests.rs`:
    - `a_malformed_entry_shows_its_path_and_reason`
    - `comment_bodies_are_read_editors_and_can_be_copied`
    - `the_comments_button_and_its_key_jump_and_focus_the_heading`
    - `the_floating_button_hides_once_the_discussion_shows`
    - `no_comments_says_so_and_hides_the_floating_button`
    - `comment_is_disabled_with_its_note`
- [ ] **Step 2: Implement.** One `ReadOnly` per comment body, created when
  the comment list lands and kept by comment ID across reloads. Bind
  `secondary-shift-c` to `JumpToDiscussion` in `keys.rs`.
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands discussion
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::item
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::scroll
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands keyboard_tests
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Part A Checkpoint

- [ ] **Step 1: Merge.** Run the merge-tree check, and merge main if it
  moved.
- [ ] **Step 2: Run the desktop binary's tests in full.**
  - `--features desktop-test-support --bin manyhands`;
  - `--test drafts`;
  - the desktop-only check and clippy;
  - the CLI isolation script.
- [ ] **Step 3: Review.** A fresh reviewer reviews Part A against the
  item-view handoff, the Design Deviations and Not Built tables, decisions
  12, 16 and 17, R-47 to R-54, R-56, R-60 to R-67, and contracts 1, 13 and
  16. Use the adversarial brief on the read-only guard.
- [ ] **Step 4: Ticket comment.**

---

# Part B: Editor, Toolbar, Links And Images

## Task 9: The Editor Host

**Interfaces:**

```rust
pub(crate) struct EditorHost { /* edit editor, baseline: String, rich: bool, dirty: bool, pressed: bool */ }
impl EditorHost {
    pub(crate) fn new(body: &str, cursor: Option<usize>, window: &mut Window, cx: &mut Context<Self>) -> Self; // R-65
    pub(crate) fn text(&self, cx: &App) -> String;
    pub(crate) fn is_dirty(&self) -> bool;
    pub(crate) fn set_rich(&mut self, rich: bool, cx: &mut Context<Self>);
    pub(crate) fn mark_saved(&mut self, saved: &str, cx: &mut Context<Self>);              // R-69, no set_text
    pub(crate) fn rebase(&mut self, baseline: String, text: &str, cx: &mut Context<Self>); // R-18
    pub(crate) fn reload_clean(&mut self, body: &str, cx: &mut Context<Self>);             // R-19
}
pub(crate) enum EditorHostEvent { DirtyChanged(bool), Edited, SelectionChanged }
// keys.rs: actions Edit, Save, ToggleRich, EditorEscape, InsertLink, NewItem;
//   escape and secondary-k bound in "EditHost > Editor" (R-59)
```

- [ ] **Step 1: Tests first.** They build an `EditorHost` directly in the
  test shell.
  - `a_mode_switch_keeps_the_text_and_the_undo_history`
  - `typing_in_rich_mode_then_switching_round_trips_each_fixture`
  - `undo_back_to_the_baseline_makes_the_editor_clean`
  - `mark_saved_makes_the_editor_clean_and_keeps_undo` (R-69)
  - `a_normalized_body_is_not_dirty_after_load_and_mode_switches`
  - `bold_italic_code_and_strike_work_from_their_keys`
  - `ctrl_u_does_not_underline`
  - `tab_indents_inside_the_editor_and_f6_leaves_it`
  - `escape_focuses_the_headers_done` (R-59)
  - `escape_after_a_pointer_press_dispatches_dismiss_and_a_second_focuses_done`
    (R-59; the dispatch is observed through a test hook, since no menu state
    is public)
  - `the_caret_steps_over_math_and_property_blocks` (R-66)
  - `markdown_mode_uses_the_monospace_font`
  - `plain_text_paste_works_in_both_modes`
  - `enter_backspace_and_arrows_edit_as_expected`
  - `the_caret_is_kept_visible_in_the_scroll_region` (R-67)
  - `the_cursor_given_is_where_editing_starts` (R-65)
- [ ] **Step 2: Implement.** `EditorHost` carries the dead-code allowance
  naming Task 14.
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands editor
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands keys
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands keyboard_tests
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- sh scripts/check-cli-no-gpui.sh
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 10a: The Toolbar's Host Edits

**Interfaces:**

```rust
// model/markdown_edit.rs (R-55)
pub(crate) struct Selection { pub start: usize, pub end: usize }          // byte offsets
pub(crate) fn selection_from_utf16(text: &str, utf16: Range<usize>) -> Selection;
pub(crate) struct HostEdit { pub range: Range<usize>, pub replacement: String, pub cursor: usize }
pub(crate) enum BlockStyle { Paragraph, Heading(u8) }
pub(crate) enum LinePrefix { Bullet, Numbered, Task, Quote }
pub(crate) fn set_block_style(text: &str, sel: Selection, style: BlockStyle) -> HostEdit;
pub(crate) fn toggle_prefix(text: &str, sel: Selection, prefix: LinePrefix) -> HostEdit;
pub(crate) fn code_block(text: &str, sel: Selection) -> HostEdit;
pub(crate) fn insert_table(text: &str, sel: Selection) -> HostEdit;
pub(crate) fn insert_link(text: &str, sel: Selection, address: &str) -> HostEdit;
pub(crate) fn block_style_at(text: &str, cursor: usize) -> BlockStyle;
pub(crate) fn table_at(text: &str, cursor: usize) -> bool;                // a pipe table around the caret
pub(crate) enum Mark { Bold, Italic, Strike, Code }
pub(crate) fn active_marks(text: &str, sel: Selection) -> Vec<Mark>;     // the editor's unwrap rule
```

- [ ] **Step 1: Tests first** (`model/markdown_edit_tests.rs`), over
  Markdown strings and selections:
  - `headings_are_set_replaced_and_cleared_on_the_caret_line`
  - `each_prefix_toggles_on_every_selected_line`
  - `a_numbered_list_counts_from_one`
  - `a_code_block_wraps_the_selected_lines_or_inserts_an_empty_fence`
  - `a_table_template_is_inserted_on_its_own_lines`
  - `a_link_wraps_the_selection_or_uses_the_address_as_its_text`
  - `utf16_selections_convert_at_non_bmp_and_combining_text`
  - `the_block_style_follows_the_caret_line`
  - `table_at_finds_the_caret_in_a_pipe_table_and_not_in_a_paragraph`
  - `active_marks_follow_the_editors_unwrap_rule`
- [ ] **Step 2: Implement.** The module carries the dead-code allowance
  naming Task 14.
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands markdown_edit
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 10b: The Toolbar

- [ ] **Step 1: Tests first** (`view/editor/toolbar_tests.rs`), through a
  real `EditorHost` in the test shell. Each inline-mark test selects text
  first, since those actions do nothing on an empty selection.
  - `every_control_edits_by_pointer_and_by_keyboard_in_both_modes`
  - `editor_actions_reach_the_editor_while_a_button_has_focus`
  - `a_host_edit_is_one_undo_step`
  - `left_right_home_and_end_move_along_the_toolbar`
  - `the_style_label_and_active_marks_follow_the_caret`
  - `table_controls_show_only_inside_a_table_and_insert_a_row`
  - `secondary_k_opens_the_link_popover_only_in_the_edit_host`
  - `markdown_mode_dims_the_formatting_controls_and_keeps_them_working`
- [ ] **Step 2: Implement.** The status item is a placeholder slot until
  Task 14 wires it. The toolbar carries the dead-code allowance naming
  Task 14.
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands toolbar
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands keyboard_tests
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 11: Links

**Interfaces:**

```rust
// model/links.rs
pub(crate) enum LinkTarget { External(String), Unsafe, Path { path: String, from_root: bool }, Title(String), Ignored }
pub(crate) fn parse_link(raw: &str) -> LinkTarget;
pub(crate) fn nearest<'a>(from_dir: &str, candidates: &'a [CandidatePath]) -> Option<&'a CandidatePath>;
// services/links.rs
pub(crate) fn resolve(target: LinkTarget, from: &ItemDto, …) -> Task<LinkResolution>; // Item(ItemLocator) | NotManaged | NotFound
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
    - `an_unresolved_link_says_not_found_in_a_banner`
    - `a_comments_link_resolves_from_its_items_folder`
  - **Handler tests.** Feed `EditorEvent::OpenLink` and `OpenWikiLink`
    straight to the read editor, an `EditorHost` built in the test and a
    comment editor. Try unsafe targets first, then safe ones, and check
    `opened_url()` after each.
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands links
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::item
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 12: Local Images

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

- [ ] **Step 1: Tests first.**
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
    - `the_read_editor_shows_the_image_once_loaded`
    - `an_edit_host_and_a_comment_share_the_tabs_cache` (an `EditorHost`
      built in the test, since Edit is wired in Task 14)
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands resources
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands images
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::item
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Part B Checkpoint

- [ ] **Step 1: Merge.** Run the merge-tree check, and merge main if it
  moved.
- [ ] **Step 2: Run the desktop binary's tests in full,** with
  `--test drafts`, the desktop-only check and clippy and the CLI isolation
  script.
- [ ] **Step 3: Review.** A fresh reviewer reviews Part B against the N
  table, decisions 1, 8, 9 and 10, R-8, R-12, R-55, R-59, R-66, R-67, R-69,
  and contracts 2, 3, 5, 12 and 13.
- [ ] **Step 4: Ticket comment.**

---

# Part C: Editing, Saving And Review

## Task 13: New Intents

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
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 14: Edit, Provisioning And Drafts

**Interfaces:**

```rust
pub(crate) enum Phase { Viewing, Preparing(DesktopRequestId), Loading, Editing, Saving, Review }
pub(crate) fn base_for_edit(item: &ItemDto) -> Option<DraftBase>; // DraftBase::of(normalized(item)) (R-11)
pub(crate) struct EditSession { saved: bool, kept: bool, /* host: Entity<EditorHost>, … */ }
pub(crate) struct CreatedContexts { /* (root, kind, id) seen created: true, not saved into (R-75) */ }
pub(crate) enum DraftIndicator { Kept { age_secs: u64 }, NotKept, RecoveryOff }   // from D2's state only
```

- [ ] **Step 1: Tests first.**
  - `edit_prepares_the_context_with_progress_in_the_pill_before_the_editor_opens`
  - `e_and_secondary_shift_e_start_edit_only_while_viewing`
  - `edit_of_a_closed_ticket_is_not_offered`
  - `edit_reads_continue_editing_while_a_draft_is_stored` (R-35)
  - `edit_of_an_uncommitted_primary_file_is_refused_and_nothing_is_made`
    (Review Focus 3, with a snapshot)
  - `a_discovery_pending_context_waits_for_resume`
  - `closing_the_tab_while_preparing_opens_no_editor`
  - `a_created_answer_enters_created_contexts_and_a_reused_one_does_not`
    (R-75)
  - `edit_carries_the_read_cursor_into_the_editor` (R-65)
  - `loading_reloads_the_read_editor_when_the_texts_differ` (R-70)
  - `the_toolbar_appears_and_focus_moves_into_the_editor`
  - `the_first_real_edit_begins_a_draft_and_later_edits_update_it`
  - `an_unavailable_store_is_said_before_editing`
  - `a_failed_flush_while_editing_stays_visible`: make the drafts folder
    read-only during editing; the failure shows, and typing continues.
  - `the_draft_indicator_reads_only_d2s_flush_state`
  - `undo_reaches_the_draft` (R-12)
  - `normalization_alone_reads_current_on_the_drafts_page` (R-11)
  - `unsorted_deps_classify_as_current` (R-72)
  - `a_large_body_opens_in_the_editor` (a smoke test; Review Focus 4)
  - Apply E-6's first half: Edit leaves `LATER`.
- [ ] **Step 2: Implement.**
  - `editor_text::normalized` at every desktop `classify` call, the Drafts
    page's included.
  - Remove D2's allowances on `begin` and `edit`, and the allowances that
    name Task 14 (`open_editor`, `load`, `normalized`, `EditorHost`,
    `markdown_edit` and the toolbar), which this task calls first.
  - Wire the toolbar's status item.
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands edit
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::item
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands model::item
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands toolbar
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands drafts
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands keyboard_tests
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 15: Save, The Session After A Save, And States

```rust
pub(crate) enum SaveState { Unsaved, SavedLocally { publication_pending: bool }, DiscoveryPending,
                            CheckpointFailed, Changed, ReviewBeforeSaving, Refused(ResultCode) }
pub(crate) fn save_state(code: ResultCode, effects: &Effects, has_publication_remote: bool) -> SaveState;
// services/drafts: the registry emits on each committed save, before it retires or rebases (R-69)
pub(crate) struct DraftSaved { pub draft: DraftId, pub submitted: DraftFields, pub base: DraftBase }
```

- [ ] **Step 1: Write the design's session-outcome rows for Task 15 first**
  (R-76). A row that answers differently stops the task for a ruling.
  - `edit_save_edit_save_commits_twice_keeps_undo_and_opens_no_review`
  - `a_second_actor_between_commit_and_read_makes_the_next_save_review`
  - `typing_during_the_save_stays_dirty_and_is_saved_next`
  - `typing_before_draft_saved_lands_is_drafted_on_the_post_save_base`
  - `a_clean_editor_re_read_with_an_equal_body_keeps_undo`
- [ ] **Step 2: The other tests first.**
  - `save_state_maps_every_row_of_the_design_table_to_its_words`
  - `save_is_offered_only_when_dirty_and_not_while_saving`
  - `a_document_save_commits_once_and_the_pill_reads_saved_on_this_computer`
  - `a_committed_save_removes_the_context_from_created_contexts` (R-75)
  - `a_committed_save_reads_the_tab_again_and_reloads_the_read_editor`
    (R-70)
  - `a_read_sent_before_a_save_never_lands_after_it` (R-70)
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
  - `discovery_pending_offers_resume_in_a_banner`
  - `already_applied_reads_saved_and_retires`
  - `secondary_s_saves_and_secondary_shift_m_switches_mode`
  - `secondary_s_on_a_clean_editor_says_no_changes_to_save`
  - `publication_pending_is_read_after_a_save_not_on_load`
- [ ] **Step 3: Implement.** Remove D2's allowance on `save`.
- [ ] **Step 4: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands edit
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands drafts
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::tabs
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 5: Commit, and write the ticket comment.**

## Task 16: Done And The Kept Session

- [ ] **Step 1: Write the design's session-outcome rows for Task 16 first**
  (R-76). A row that answers differently stops the task for a ruling.
  - `done_waits_for_a_committed_save_then_applies_its_rules`
  - `done_stops_when_the_waited_save_answers_anything_but_committed`
  - `done_with_unsaved_text_writes_the_draft_and_keeps_the_session`
  - `done_asks_when_the_draft_cannot_be_written`
  - `done_after_typing_and_undoing_discards_the_stored_draft`
  - `done_on_a_never_created_item_asks`
  - `a_drafts_page_discard_ends_the_kept_session_and_says_so` (R-71)
  - `the_drafts_pages_discard_is_disabled_while_that_draft_saves` (R-71)
- [ ] **Step 2: The other tests first** (decision 15; R-58, R-71, R-73).
  - `done_writes_a_revision_still_waiting_on_the_timer_first`
  - `the_kept_session_shows_the_saved_copy_in_page_header_sheet_and_title`
  - `continue_editing_edit_and_e_restore_text_cursor_and_undo`
  - `a_refresh_while_kept_shows_the_change_banner_and_opens_nothing` (R-68)
  - `a_drafts_page_discard_ends_a_live_session_and_says_so`
  - `closing_a_tab_with_a_kept_session_asks_first`
  - `a_metadata_change_alone_makes_closing_ask`
  - `closing_a_dirty_tab_with_no_stored_draft_asks_first` (R-41: an
    unavailable store, and a first write held)
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands edit
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::item
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::tabs
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands drafts
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 5: Commit, and write the ticket comment.**

## Task 17: Release On Leave

- [ ] **Step 1: Write the design's session-outcome rows for Task 17 first**
  (R-76, decision 18). A row that answers differently stops the task for a
  ruling.
  - `a_drafts_page_discard_of_a_kept_draft_releases_a_context_this_run_made`
  - `a_drafts_page_discard_of_a_held_never_created_draft_closes_the_tab_and_releases`
  - `keep_draft_then_a_later_drafts_page_discard_releases_the_context`
  - `discarding_a_draft_whose_context_this_run_did_not_make_releases_nothing`
- [ ] **Step 2: The other tests first** (R-37, R-40, R-75).
  - `done_after_an_unused_edit_releases_the_context`
  - `discard_after_an_unused_edit_releases_the_context`
  - `closing_a_clean_tab_after_an_unused_edit_releases_the_context`
  - `closing_the_tab_during_loading_releases_a_context_this_run_made`
  - `a_session_that_saved_keeps_its_context`
  - `done_after_an_edit_on_an_existing_context_releases_nothing`
  - `a_kept_draft_prevents_release`
  - `an_unavailable_draft_listing_prevents_release`
  - `discarding_a_restored_draft_releases_the_context_continue_editing_made`
  - `closing_the_tab_while_preparing_releases_the_context_it_made`
  - `discard_is_disabled_while_a_save_is_in_flight`
  - `a_stranded_context_can_be_released_from_the_tab` (R-40)
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands edit
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands drafts
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands keyboard_tests
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 5: Commit, and write the ticket comment.**

## Task 18: Restored Drafts, Close-Dialog Save And Late Results

- [ ] **Step 1: Apply E-8 and E-9, and write the new tests first.**
  - `continue_editing_from_the_banner_loads_the_drafts_text_dirty_on_its_own_base_and_saves`
  - `continue_editing_a_draft_based_on_primary_takes_the_contexts_token_when_the_source_matches`
    (R-42)
  - `continue_editing_opens_the_review_when_the_source_differs`
  - `save_from_a_restored_draft_prepares_the_context_first`
  - `save_in_the_close_dialog_saves_then_closes`
  - `a_refused_save_from_the_close_dialog_keeps_the_tab`
  - `a_pending_checkpoint_from_the_close_dialog_keeps_it_open_with_retry`
    (the design's session-outcome row; a different result stops for a
    ruling)
  - `keep_draft_is_still_the_first_tab_stop_of_the_close_dialog` (R-16)
  - `typing_during_a_save_while_a_tab_read_lands_keeps_the_newer_text`
    (Review Focus 2; `DelayingBackend` holds the save; the read is
    `ReadKind::Item`)
  - `a_late_read_does_not_replace_a_dirty_editor`
  - `a_refresh_with_a_kept_session_reloads_only_the_read_editor`
  - `a_clean_editor_reloads_only_on_a_newer_body_and_keeps_its_cursor`
    (R-19, R-70)
- [ ] **Step 2: Implement.** The new close-dialog tests live in
  `view/tabs_tests.rs`, beside D2's. The implementation includes
  `workspace/content.rs`'s close-choice match and R-38's placement.
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::item
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::tabs
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands drafts
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands keyboard_tests
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 19: Stale-Edit Review

- [ ] **Step 1: Apply E-19, and write the tests first.** They cover every row
  of the design's review outcome table (R-44), and the session-outcome row
  for Task 19. A row that answers differently stops the task for a ruling,
  and the table is not edited to fit. The tests:
  - `cancel_after_taking_theirs_on_the_body_leaves_the_draft_file_byte_identical`
  - `cancel_after_editing_the_body_restores_mine`
  - `switching_the_body_choice_after_an_edit_asks_first`
  - `an_edited_body_result_is_what_save_writes`
  - `a_stale_save_opens_the_review_and_leaves_the_draft_file_unchanged`
  - `a_refresh_shows_the_change_banner_and_compare_opens_the_review` (R-68)
  - `one_sided_changes_default_and_both_changed_fields_must_be_chosen`
  - `taking_theirs_for_the_title_and_mine_for_the_body_saves_exactly_that`
  - `a_ticket_change_by_the_second_actor_is_reviewed`
  - `cancel_leaves_draft_and_base_unchanged`
  - `a_second_change_during_review_reopens_it_on_save` (two different
    `SaveDocumentAs` bodies)
  - `a_body_changed_only_by_them_defaults_to_theirs`
  - `a_body_changed_by_both_must_be_chosen_before_use`
  - `an_unverified_base_opens_the_review_before_any_save` (R-36)
  - `done_and_save_are_disabled_during_the_review`
  - `escape_in_the_reviews_body_editor_focuses_done` (R-59)
  - `the_d2_privacy_sweep_still_holds_with_the_review_open` (R-38)
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands review
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands drafts::privacy_tests
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Part C Checkpoint

- [ ] **Step 1: Merge, and run the desktop binary's tests in full,** with
  `--test drafts`, the desktop-only check and clippy.
- [ ] **Step 2: Review.** A fresh reviewer, with the adversarial brief on
  saves, Done and the kept session, late results, release on leave and the
  review. The session-outcome table; contracts 4, 6, 7 and 8; decisions 15
  and 18; R-15, R-37, R-41, R-58, R-68 to R-71, R-73, R-75, R-76.
- [ ] **Step 3: Ticket comment.**

---

# Part D: Details Editing, Relationships And Creation

## Task 20: Named Controls In The Details Sheet

- [ ] **Step 1: Tests first** (`view/item/sheet/fields_tests.rs`).
  - `each_named_control_edits_its_field_and_marks_the_tab_dirty`
  - `a_chip_opens_the_sheet_focused_on_its_control`
  - `suggestions_come_from_the_loaded_tickets_values` (R-57)
  - `an_empty_required_field_shows_its_message_and_save_is_disabled`
  - `id_short_code_closure_and_every_other_key_are_not_editable`
  - `a_documents_status_date_and_author_are_not_editable` (decision 14)
  - `a_status_of_closed_saves_as_ordinary_text_and_the_ticket_stays_open`
  - `an_emptied_project_is_saved_as_absent` (R-27)
  - `secondary_s_inside_the_sheet_saves_the_tab` (R-74)
  - `the_controls_are_read_only_while_preparing_loading_and_in_review`
    (R-74)
  - `secondary_s_inside_the_sheet_does_nothing_during_the_review`
  - `the_sheets_body_is_a_draft_section_while_editing` (R-38)
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands sheet
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands metadata
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands drafts::privacy_tests
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 21: Dependencies, Parent And The Cycle Check

- [ ] **Step 1: Tests first,** over real tickets, in the sheet.
  - `a_dependency_is_chosen_from_the_list_and_by_short_code`
  - `a_parent_is_changed_and_cleared`
  - `a_proposed_dependency_cycle_is_explained_with_its_tickets_and_save_is_unavailable`
  - `a_proposed_parent_cycle_is_explained`
  - `an_unresolved_dependency_is_shown_and_saves`
  - `an_incomplete_index_adds_its_note`
  - `a_late_check_does_not_replace_a_newer_one`
  - `deps_are_shown_and_drafted_in_id_order` (R-72)
  - `adding_then_removing_a_dependency_returns_the_tab_to_clean_and_sends_unchanged`
  - `saving_a_ticket_whose_file_has_unsorted_deps_leaves_a_verified_base`
  - `a_library_relationship_cycle_answer_is_explained_the_same_way`
  - `dependents_and_children_stay_read_only`
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands relations_edit
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands sheet
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 22: New-Item Drafts

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
    - `a_titled_new_item_with_an_empty_body_can_be_saved` (R-73)
    - `an_occupied_path_for_this_drafts_own_item_becomes_a_review`
    - `a_marked_draft_whose_item_exists_routes_to_edit_and_review` (R-42)
    - `continue_editing_a_not_created_draft_whose_item_appeared_routes_to_review`
    - `create_stops_before_provisioning_when_the_store_is_unavailable`
      (R-46)
    - `a_create_retried_after_a_lost_answer_creates_one_item` (worker
      `retry`)
    - `discarding_a_new_item_draft_releases_the_context_this_session_made`
    - `discarding_a_restored_new_item_draft_keeps_its_context` (decision 18)
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

## Task 23: Create Dialogs

- [ ] **Step 1: Tests first** (`view/create_tests.rs`).
  - `new_document_prepares_and_opens_a_draft_already_in_the_store`
  - `the_path_must_be_under_docs_end_md_and_be_free`
  - `save_creates_the_document_once_and_the_tab_becomes_an_edit_session`
  - `new_ticket_creates_with_its_fields_and_dependency`
  - `the_new_tab_opens_editing_with_its_header_from_the_draft`
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
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Part D Checkpoint

- [ ] **Step 1: Merge, and run the desktop binary's tests in full,** with
  `--test drafts` and `--lib drafts::`.
- [ ] **Step 2: Review.** Contracts 9, 10 and 11 for create; decisions 14
  and 18; R-20, R-20b, R-27, R-38, R-57, R-72, R-73, R-74; release on
  discard.
- [ ] **Step 3: Ticket comment.**

---

# Part E: Moves, Folders, Repair And The Gate

## Task 24: Move And New Folder

- [ ] **Step 1: Apply E-10, and write the tests first.**
  - `move_from_the_sheet_commits_both_paths_and_the_tab_follows`
  - `a_collision_is_refused_and_explained_and_the_dialog_stays`
  - `a_stale_source_is_refused`
  - `move_is_offered_only_while_viewing_with_no_draft` (R-23)
  - `move_is_disabled_while_a_release_is_queued_or_running`
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
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands sheet
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 25: Repair

- [ ] **Step 1: Apply E-6's second half and E-7's second half, and write
  the tests first.**
  - `the_needs_fixing_banner_offers_repair`
  - `an_unmanaged_document_offers_adopt`
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
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 26: Keyboard And Privacy

- [ ] **Step 1: Keyboard tests,** driven by keystroke alone.
  - In `view/keyboard_tests/frame.rs`:
    - the tab strip, header actions and chips, in the design's tab order;
    - the details sheet, with Escape and focus return;
    - the discussion's navigation and its key;
    - the read view: select, copy, and Tab out.
  - In `view/keyboard_tests/editing.rs`:
    - Edit, mode switch, every toolbar control, Save and Done;
    - Continue editing after Done;
    - the review, the sheet's controls and both pickers;
    - Create, Move, New folder and Repair.

  Escape and focus return are checked wherever a dialog or the sheet
  closes, and Escape's move to the header's Done while editing.
- [ ] **Step 2: Privacy sweep.** In `view/editing_privacy_tests.rs`, a
  sentinel body, link target and image path must appear in no operation
  entry, notification, dialog message, error or `Debug` output. Cover:
  - save, stale save and `invalid_input`;
  - create, move, repair and release;
  - the details sheet and the comment editors.
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands keyboard_tests
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands privacy
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 4: Commit, and write the ticket comment.**

## Task 27: Reviews, Gate And Review-Ready

- [ ] **Step 1: Whole-branch reviews.** A spec review and a code-quality
  review. Fix what they find, re-run each fixed task's Verify lines, and
  commit.
- [ ] **Step 2: Native checklist.** Ask the owner to run the design's Linux
  Wayland checklist and the desktop smoke test
  `devenv shell -- cargo run --locked --features desktop --bin manyhands`.
  Record the results in the Cycle's table.
- [ ] **Step 3: Full gate, last.** Run it detached, with a two-hour budget,
  and record the counts in the ledger. If the merge of main brings desktop
  changes, ask the owner to re-run the checklist steps they touch.

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
    final push before merge, as a canonical ticket-file edit until C5
    (`AGENTS.md`).

## Plan Self-Review

| Acceptance row or ruling | Task |
| --- | --- |
| Context prepare; R-1 to R-5 | 1 |
| Context release; R-32, R-43 | 2 |
| E-21, E-22 | 2 |
| Read-only guard; R-8, R-59 (bindings), R-61, R-66 | 3, 7 |
| Load normalization | 3 |
| Tab strip; R-50, R-51, R-52 | 4 |
| Item frame; R-47 | 5a, 5b |
| Details sheet; R-48, R-54, R-60, R-64 | 6 |
| Read view; R-19 (read editor), R-49, R-62, R-63 (body), R-64, R-67 | 7 |
| Discussion; R-53, R-56, R-63 (comments) | 8 |
| Round trips; shared undo; not dirty (editor); R-9, R-12, R-59 (Escape), R-65, R-66, R-67, R-69 (`mark_saved`) | 9 |
| Toolbar; R-55 | 10a, 10b, 14 |
| Links | 11 |
| Images; R-33, R-34, R-39 | 12 |
| Edit; R-11, R-12, R-13, R-35, R-70 (Loading), R-72 (classification), R-75 (the set) | 14 |
| Save; states; untouched bytes; not dirty (repository); R-14, R-69, R-70; session outcomes for Task 15 | 15 |
| Done; R-15, R-41, R-58, R-68 (kept), R-71, R-73; session outcomes for Task 16 | 16 |
| Release on leave; R-37, R-40, R-75; decision 18; session outcomes for Task 17 | 17 |
| Late results; recovery; R-16, R-17, R-19, R-38, R-42 (Continue editing), R-70; the close-dialog outcome | 18 |
| Stale edit; R-18, R-36, R-44, R-68; the review's session outcome | 19 |
| Metadata; R-27, R-38 (sheet), R-57, R-74; decision 14 | 20 |
| Relationships; R-26, R-72 | 21 |
| Create (drafts); R-20, R-20b, R-42 (routing), R-46, R-73 (new item); decision 18 | 22 |
| Create (dialogs); R-21, R-22 | 23 |
| Move; folders; R-23, R-24 | 24 |
| Repair; R-25 | 25 |
| Keyboard; privacy | 26 |
| Native checklist; native run; CLI isolation; regression | 27 |
| R-76 (the session-outcome table) | 15, 16, 17, 18, 19 |
| E-1 to E-5 | 7 |
| E-6 | 5b (labels), 14, 25 |
| E-7 | 5b, 25 |
| E-8, E-9 | 18 |
| E-10 | 24 |
| E-11 to E-14 | 13 |
| E-15 to E-18 | 1, 2 |
| E-19 | 19 |
| E-20 | 2 |
| E-23, E-26 | 6 |
| E-24 | 8 |
| E-25 | 5a |
| E-27, E-28, E-29 | 4 |

**Interfaces** are defined where they are produced:

| Interface | Task |
| --- | --- |
| The context DTOs and inputs | 1, 2 |
| `editor_text`, `open_editor`, `step_over_blocks`, editor `keys::bind`, `ReadOnly` | 3 |
| `DesktopAssets`, `ItemColors` | 4 |
| `Page` (frame) | 5a |
| `PillState`, `Banner`, `ChipField`, `OpenDetails` | 5b |
| `open_details` | 6 |
| `page_style`, `reveal_caret` | 7 |
| `Thread`, `Reply` and the discussion helpers | 8 |
| `EditorHost` | 9 |
| `markdown_edit` | 10a |
| Link and image models | 11, 12 |
| Intents | 13 |
| `Phase`, `EditSession`, `CreatedContexts`, `DraftIndicator` | 14 |
| `SaveState`, `DraftSaved` | 15 |
| `NotCreated`, `begin_new`, `create` | 22 |

**Placeholders.** None intended. Task 0's sweep is how "no other test edits"
is proven.
