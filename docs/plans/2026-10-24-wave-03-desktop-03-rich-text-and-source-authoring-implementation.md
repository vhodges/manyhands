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
desktop with `zorite-editor`, through D2's drafts and F2's saves, with every
save state told truthfully.

**Architecture:**
- **Part 0** adds one F2 command, `context prepare`, over the domain's
  `prepare_context`.
- **The desktop** renders bodies with `zorite-markdown`'s `MarkdownView`
  and hosts one `zorite_editor::EditorState` per editing tab. It feeds D2's
  `DraftRegistry` and submits new `Intent`s through D1's worker.
- **`src/drafts/`** gains one target state, for a new item's draft.

**Tech stack:**
- Rust 2024, with `gpui-kit` 0.6.6 through `gpui_kit::*` only.
- `zorite-editor` `=0.10.0`, and `zorite-markdown` `=0.9.0` with feature
  `view`. Both are already locked; `markdown` 1.0.0 is already locked too.
- No new package in `Cargo.lock`.
- Devenv.

**Spec:** [Cycle](../Cycles/wave-03-desktop-03-rich-text-and-source-authoring.md)
and [design](2026-10-24-wave-03-desktop-03-rich-text-and-source-authoring-design.md).
Executors read both. "Fact n", "N-n", "R-n" and "E-n" refer to the
design's Baseline Facts, Recorded Load Normalization, Rulings and Named
Edits.

## Global Constraints

- **Where.** Work only in ticket `01M4CC0VN0MFTGKQ4HGTTC5H63`'s worktree
  and branch. Preserve unrelated edits in main and every other worktree.
- **Cargo.** Run every Rust command as `devenv shell -- cargo …` with
  `--locked`. The one exception is Task 2's single lockfile update.
- **Library changes are Part 0's only** (decision 2).
  - Task 1 may change `src/repository/mutation/` (`command.rs`, `mod.rs`,
    `dto.rs`, `bind/mod.rs` and the new `bind/context.rs`),
    `schemas/v1/`, `tests/mutation_bridges*`, `tests/support/` and
    `tests/fixtures/mutation_v1/`.
  - No other task changes `src/repository.rs`, `src/repository/`,
    `src/canonical.rs`, `src/results.rs`, `src/runtime.rs` or `src/lib.rs`.
  - A missing or wrong library capability found later stops the task.
    File a shared-library ticket with `file-ticket` and ask the owner.
- **`src/drafts/`** changes only in Task 11 (`TargetState::NotCreated`,
  ruling R-20). It imports nothing from `gpui_kit`.
- **Dependencies.** `Cargo.toml` changes only in Task 2. `Cargo.lock`
  changes only there, and gains no new `[[package]]`.
- **GPUI** only through `gpui_kit::*`, and no `use gpui_kit::*` glob.
  `zorite_editor` and `zorite_markdown` are used only under `src/desktop/`.
- **Privacy.** No type holding body, draft or source text derives `Debug`.
  Body text never goes into an operation entry, notification, dialog
  message, log or error.
- **Every test isolates itself.** Each one uses its own temporary
  repositories, data directory and Git home. No test reaches a network.
  No desktop test initializes the transport.
- **Editor tests:**
  - Focus the editor and draw a frame before typing.
  - Type with the window-level `simulate_input` / `simulate_keystrokes`,
    never gpui-kit's scoped helpers.
  - Never send Ctrl-Cmd-Space (design fact 33).
  - A view test that waits on the worker calls
    `cx.executor().allow_parking()`.
- **Test runs:**
  - `cargo test` takes one positional filter, and each Verify line uses
    one.
  - Within a task, run only the tests its Verify line names. The full
    suite runs at Part checkpoints and in Task 14 (owner's standing rule).
- **Desktop verification.** Each desktop task verifies with
  `--features desktop-test-support` tests, and with `--features desktop`
  check and clippy on their own.
- **Test-first.** Write each behavior's test before its code.
- **Existing tests** change only as the design's named edits E-1 to E-17.
  Any other change to a pre-existing test, even a new lint, stops for the
  owner.
- **Commits.** Commit per task with a conventional message ending
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- **Merging.** Merge `origin/main` at work boundaries and never rebase.
  Before each Part checkpoint, run `git fetch origin` and
  `git merge-tree --write-tree HEAD origin/main`.
- **Ticket comments.** After each task, add one as a canonical file under
  `.manyhands/comments/01M4CC0VN0MFTGKQ4HGTTC5H63/`. Take the ULID from
  `manyhands-cli id new` and `created_at` from the UTC clock. Record the
  commit, the tests run with their results, and any ruling. Part
  checkpoints add the review and its fixes.
- **The execution ledger**,
  `docs/plans/2026-10-24-wave-03-desktop-03-rich-text-and-source-authoring-execution.md`,
  is created in Task 0 and committed with every task. D1 and D2 never
  committed theirs.
- **Files.** Each new desktop source file stays below about 500 lines; test
  files are exempt.
- **Scripts.** Run them through Devenv:
  `devenv shell -- sh scripts/check-cli-no-gpui.sh`.
- **Implementers never dispatch subagents.**

## Review Focus

1. **Line endings and `$$` together.** A body with CRLF line endings and a
   words-mixed `$$` pair opens in the editor, is not dirty, and Done
   writes nothing; the repository stays byte-identical. Pinned in
   Task 2 (load) and Task 6 (no write).
2. **Typing during a save while a refresh lands.** Newer text stays in the
   editor and in the draft; only the saved revision retires. Pinned in
   Task 7.
3. **Edit on a hand-edited, uncommitted primary file.** Edit is refused
   with `worktree_not_clean`; no branch or worktree is made; the editor
   never opens. Pinned in Task 1 (binding) and Task 6 (tab).
4. **A very large body** (over 128 KiB) in the read view. It renders, plain
   first and then formatted, without blocking the window; Edit opens it.
   Pinned in Task 3.
5. **Closing the tab while Edit is preparing.** The request still completes
   and is listed in the operation area; no editor opens into a closed or
   different tab. Pinned in Task 6.

## File Map And Dependency Order

| Path | Task | Change |
| --- | --- | --- |
| `docs/plans/…-execution.md` | 0 | Execution ledger. |
| `src/repository/mutation/{command,mod,dto}.rs`, `bind/{mod,context}.rs` | 1 | `context prepare`. |
| `schemas/v1/context_mutation.schema.json`, `tests/fixtures/mutation_v1/context-prepare-*.json` | 1 | Schema and goldens. |
| `tests/mutation_bridges.rs`, `tests/mutation_bridges/context.rs` | 1 | Integration tests. |
| `tests/support/{conformance,mutation_golden}.rs`, `tests/mutation_contract.rs` | 1 | E-15, E-16, E-17. |
| `Cargo.toml`, `Cargo.lock` | 2 | Editor and viewer in `desktop`. |
| `src/desktop/model/editor_text.rs` and tests, `tests/fixtures/editor/*` | 2 | `load`, goldens. |
| `src/desktop/view/editor.rs` (feasibility test first) | 2, 4 | The editor host. |
| `src/desktop/view/item/body.rs`, `view/item.rs`, `view/item/source.rs` | 3 | Read view and Source toggle; E-1 to E-5. |
| `src/desktop/view/editor/{style,toolbar,keys}.rs`, `src/desktop/keys.rs`, `src/desktop/mod.rs` | 4 | Style, toolbar, editor key bindings. |
| `src/desktop/services/{requests,mod}.rs`, `requests/build.rs` | 5 | Seven intents, job and read kinds; E-11 to E-14. |
| `src/desktop/model/edit.rs`, `view/item/edit.rs`, `services/drafts.rs`, `services/drafts/save.rs` | 6 | Edit, provisioning, Save, states, Done. |
| `view/item/draft.rs`, `view/close_draft.rs`, `view/item/reads.rs` | 7 | Restored drafts, close-dialog Save, late results; E-8, E-9. |
| `view/item/review.rs` | 8 | Stale-edit review. |
| `view/item/metadata.rs`, `model/metadata.rs` | 9 | Metadata controls. |
| `view/item/relations_edit.rs`, `model/relations_edit.rs` | 10 | Pickers and cycle check. |
| `src/drafts/compare.rs`, `view/create.rs`, `services/drafts/create.rs`, `view/documents.rs`, `view/lists.rs` | 11 | Create. |
| `view/move_document.rs`, `view/new_folder.rs`, `view/documents.rs` | 12 | Move and New folder; E-10. |
| `view/item/repair.rs`, `model/item.rs`, `view/item/discussion.rs` | 13 | Repair; E-6, E-7. |
| `view/keyboard_tests/*`, privacy sweep, RFC, Wave, Cycle | 14 | Gate. |

Tasks run in order. Unit tests sit beside their module as `*_tests.rs`.

---

## Task 0: Baseline, References And Ledger

- [ ] **Step 1: Merge and check.**

```sh
git fetch origin
git merge origin/main          # never rebase
git merge-base --is-ancestor origin/main HEAD
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo check --locked --features desktop
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo run --locked --bin manyhands-cli
```

A failure is resolved or accepted by the owner before Task 1. The full test
suite is not run here; it runs at the Part checkpoints.

- [ ] **Step 2: Re-check the design's Baseline Facts.**
  - Run `git diff --stat 98f49b7 HEAD -- src/repository.rs src/repository src/results.rs src/drafts src/desktop Cargo.toml .github`.
  - Re-read each fact whose file changed, and update the design before
    coding if a cited signature moved.
  - Check that the bindings `secondary-shift-e`, `secondary-s`,
    `secondary-shift-m` and `secondary-n` are unused in
    `src/desktop/keys.rs`.
- [ ] **Step 3: Sweep for unlisted test edits.**

```sh
grep -rn "LATER\|EDITOR_LATER\|DRAFT_ACTIONS\|NEW_FOLDER_UNAVAILABLE\|item-source\|later-button\|draft-later\|close-draft-save" src/desktop --include=*_tests.rs
grep -rn "TargetState::" src --include=*_tests.rs
grep -rn "listed.len()\|HOMES\|DATA_ROOTS\|CASES" src/desktop tests --include=*.rs
```

  - Compare every hit with E-1 to E-17.
  - A pre-existing test that will break and is not listed stops for the
    owner.
  - Record the sweep, and its result, in the ledger.
- [ ] **Step 4: Ledger.** Create the execution ledger with:
  - the base, both heads and each check result;
  - the sweep;
  - a "Rulings during implementation" section, which starts empty.
- [ ] **Step 5: Commit.** Commit the ledger, and add the Task 0 ticket
  comment.

---

# Part 0: The Context Binding

## Task 1: `context prepare`

**Files:**
- Create: `src/repository/mutation/bind/context.rs`,
  `tests/mutation_bridges/context.rs`,
  `schemas/v1/context_mutation.schema.json`,
  `tests/fixtures/mutation_v1/context-prepare-*.json`
- Modify: `src/repository/mutation/{command,mod,dto}.rs`,
  `bind/mod.rs`, `tests/mutation_bridges.rs`,
  `tests/support/conformance.rs` (E-15),
  `tests/support/mutation_golden.rs` (E-16),
  `tests/mutation_contract.rs` (E-17)

**Interfaces:**
- Produces:

```rust
// src/repository/mutation/bind/context.rs, re-exported from mutation/mod.rs
pub struct ContextPrepareInput {
    pub root: PathBuf,
    pub kind: AuthoringKind,
    pub id: ItemId,
    pub intent: ContextPrepareIntent,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextPrepareIntent { Create, Edit }
// Mutation::ContextPrepare(ContextPrepareInput); command "context prepare"
// src/repository/mutation/dto.rs
pub struct ContextMutationDto { pub id: String, pub kind: String, pub branch: String, pub worktree: String, pub created: bool }
// MutationDataDto::Context(ContextMutationDto)
```

- Consumes: `RepositoryService::prepare_context` (fact 17),
  `save::primary_is_committed`, `item_scope`, `repository_error_code`.

- [ ] **Step 1: Write the required-outcomes tests first (ruling R-4).**
  - In `tests/mutation_bridges/context.rs`, one test per row of the
    design's re-entry table, over a real enabled repository.
  - Use `tests/support` `FailOnce` with `BeforeRequestDomainCall`,
    `BeforeWorktreeCreation` and `BeforeRequestSettlement`. Copy the
    `local_authoring.rs:7077` pattern for the in-domain point.
  - Mount the module in `tests/mutation_bridges.rs`.
  - Add the remaining behavior tests:

```text
edit_provisions_a_context_and_a_second_request_reuses_it
create_provisions_for_a_new_id_and_document_create_reuses_it
create_provisions_for_a_new_id_and_ticket_create_reuses_it
edit_of_a_closed_ticket_is_ticket_closed_and_makes_nothing
edit_of_an_uncommitted_primary_change_is_worktree_not_clean_and_makes_nothing
create_of_an_existing_id_is_occupied_path
edit_of_a_missing_item_is_item_not_found
a_request_record_holds_no_body
```

- [ ] **Step 2: Run them; every test fails to compile.**
  Run `devenv shell -- cargo test --locked --test mutation_bridges context`.
  Expected: compile errors naming `ContextPrepareInput`.
- [ ] **Step 3: Add the command.**
  - Add `ContextPrepare => "context prepare"` to `command.rs`.
  - Its rows: `has_operation` true, `stage` `Writing`, every other flag
    false.
  - Add `Mutation::ContextPrepare` with its `kind`, `confirmed` (`None`)
    and `into_binding` arms.
  - Add the `recorded_effects` and `from_stored` arms, and
    `ContextMutationDto`.
- [ ] **Step 4: Write the binding**, as the design's Part 0 says.
  - Follow `bind/folder.rs` for the shape and `bind/ticket.rs`
    (`TicketSlugAssign`) for reading an item.
  - **Fields** are `kind`, `id` and `intent`. **Scope** is `item_scope`.
  - **Prepare** does the R-2 checks before provisioning:
    - Edit reads with `show_item`, refuses a closed ticket, and calls
      `primary_is_committed` when there is no context;
    - Create requires `item_not_found` and answers `occupied_path`
      otherwise.
  - **Domain call:** `prepare_context`.
  - **Outcome:**
    - `Created` and `Reused` answer `ok` with data, and every effect is
      `not_requested` except discovery `current`;
    - `IndexPending` answers `discovery_pending`, with discovery
      `pending`.
  - **Re-entry:** whatever makes the Step 1 table pass. If a row cannot
    pass without changing code outside Task 1's files, stop for a ruling.
- [ ] **Step 5: Schema, goldens and named edits E-15 to E-17.**
  - Add the data schema.
  - Add goldens for `ok` and `discovery_pending`, and register them in
    `CASES`.
  - Home the command in `mutation_bridges` in `HOMES`, with its row.
  - Add the schema to `DATA_ROOTS`.
- [ ] **Step 6: Verify.**

```sh
devenv shell -- cargo test --locked --test mutation_bridges context
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

Expected: all pass, and the CLI's help is unchanged (R-5).

- [ ] **Step 7: Commit and ticket comment.**

## Part 0 Checkpoint

- [ ] **Step 1: Merge and run the full suite.** Run `git fetch origin` and
  `git merge-tree --write-tree HEAD origin/main`, merging main if it moved.
  Then run the full suite once:

```sh
devenv shell -- cargo test --all-features --locked
```

- [ ] **Step 2: Review.** A fresh reviewer reviews Part 0 as a library
  change of its own: scope, replay, the outcome table and the named edits.
- [ ] **Step 3: Record.** Write the ticket comment with the review and its
  fixes. The desktop Parts start only after this checkpoint passes
  (decision 2).

---

# Part A: Read View And Editor

## Task 2: Dependencies, Editor Feasibility And Load Normalization

**Files:**
- Modify: `Cargo.toml`, `Cargo.lock`
- Create: `src/desktop/model/editor_text.rs`, `editor_text_tests.rs`,
  `tests/fixtures/editor/{n1-crlf,n2-mixed-pair,n3-attached-fences,n4-inline-code,n4-tilde-fence,n4-indented,n4-html,unicode,tables,html,extensions,mixed,unknown-yaml-ticket}.md`
  and `.expected` files for the N rows
- Create: `src/desktop/view/editor.rs` (host skeleton),
  `src/desktop/view/editor_tests.rs`

**Interfaces:**

```rust
// src/desktop/model/editor_text.rs
pub(crate) fn crlf_to_lf(body: &str) -> String;   // N-1
pub(crate) fn load(body: &str) -> String;          // N-1 then normalize_math_fences
```

- [ ] **Step 1: Make the dependencies desktop-only.**
  - `desktop = ["dep:gpui-kit", "dep:futures", "dep:zorite-editor", "dep:zorite-markdown"]`.
  - `editor-probe = ["desktop"]`.
  - Add `zorite-markdown = { version = "=0.9.0", default-features = false, features = ["view"], optional = true }`.
  - Run `devenv shell -- cargo check --features desktop` once, without
    `--locked`.
  - Check `git diff Cargo.lock`. Only dependency lists may change: no new
    `[[package]]`, and no second `gpui-pre`.
  - Then run:

```sh
devenv shell -- cargo check --locked --features desktop
devenv shell -- cargo check --locked --features editor-probe --example editor_feasibility
devenv shell -- sh scripts/check-cli-no-gpui.sh
```

- [ ] **Step 2: Feasibility test (stop point).** In `view/editor_tests.rs`,
  the test `a_real_editor_takes_typing_and_undo_in_a_headless_window`:
  1. opens D1's harness window with an `EditorState` loaded with `"abc"`;
  2. focuses it and draws a frame;
  3. `simulate_input("d")`, then `text()` is `"abcd"` and one `Changed`
     was seen;
  4. `simulate_keystrokes("ctrl-z")`, then `text()` is `"abc"`.

  If it cannot be made to pass with the window-level helpers, stop and
  ask the owner before any later task.
- [ ] **Step 3: Write the fixtures and the load tests first.**
  - `editor_text_tests.rs`: `each_normalization_fixture_loads_to_its_expected_file`,
    `a_body_without_dollars_or_cr_is_unchanged`, `load_is_idempotent`.
  - `editor_tests.rs`: `the_editor_reads_back_exactly_what_load_predicts`,
    over every fixture.
  - Expected first run: compile errors, because `editor_text` doesn't
    exist yet.
- [ ] **Step 4: Implement** `editor_text`, and the host's
  `fn open(body: &str, window, cx) -> Entity<EditorState>`, which calls
  `with_text(crlf_to_lf(body))`.
- [ ] **Step 5: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands editor
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 6: Commit** `Cargo.toml`, `Cargo.lock`, the fixtures and the
  code, and add the ticket comment.

## Task 3: The Read View And The Source Toggle

**Files:**
- Create: `src/desktop/view/item/body.rs`, `body_tests.rs`
- Modify: `src/desktop/view/item.rs` (render order: the body replaces the
  source), `view/item/source.rs` (shown only under the toggle or for
  Inspect tabs), and `view/item_tests.rs` and `view/tabs_tests.rs`
  (E-1 to E-5)

**Interfaces:**

```rust
// view/item/body.rs
pub(crate) struct BodyView { show_source: bool }      // kept per ItemPane across reloads (R-6)
pub(crate) fn markdown_style(cx: &App) -> zorite_markdown::MarkdownStyle;
```

- [ ] **Step 1: Write the tests first.**
  - `a_document_body_renders_and_its_text_is_on_screen`
  - `a_ticket_body_renders`
  - `show_source_shows_the_exact_file_and_the_choice_survives_a_reload`
  - `an_inspect_tab_always_shows_the_source`
  - `a_non_utf8_item_says_so_in_both_views`
  - `an_image_is_a_label_and_nothing_is_loaded`: assert `opened_url()` is
    `None` after a click on the image label of a relative `src`.
  - `only_http_https_and_mailto_links_open`: click each; `opened_url()`
    matches only for the allowed schemes.
  - `a_large_body_renders_plain_then_formatted_without_blocking`: Review
    Focus 4, 200 KiB.
- [ ] **Step 2: Implement** the read view and toggle, as the design's "The
  read view" says. Warm the parse cache on the background executor when
  `needs_warm` (R-7).
- [ ] **Step 3: Apply E-1 to E-5.** Each test presses Show source where the
  design says. Nothing else changes.
- [ ] **Step 4: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::item
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::tabs
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
```

- [ ] **Step 5: Commit and ticket comment.**

## Task 4: The Editor Host

**Files:**
- Create: `src/desktop/view/editor/{style,toolbar,keys}.rs` with tests
- Modify: `src/desktop/view/editor.rs`, `src/desktop/keys.rs`,
  `src/desktop/mod.rs` (bind the editor keys after `gpui_kit::init`), and
  `view/harness.rs` (the same binding in the test shell)

**Interfaces:**

```rust
// view/editor.rs
pub(crate) struct EditorHost { /* editor, baseline: String, rich: bool, dirty: bool */ }
impl EditorHost {
    pub(crate) fn new(body: &str, window: &mut Window, cx: &mut Context<Self>) -> Self;
    pub(crate) fn text(&self, cx: &App) -> String;
    pub(crate) fn is_dirty(&self) -> bool;
    pub(crate) fn set_rich(&mut self, rich: bool, cx: &mut Context<Self>);
    pub(crate) fn rebase(&mut self, baseline: String, text: &str, cx: &mut Context<Self>); // review (R-18)
    pub(crate) fn reload_clean(&mut self, body: &str, cx: &mut Context<Self>);             // R-19
}
pub(crate) enum EditorHostEvent { DirtyChanged(bool), Edited }
// view/editor/keys.rs
pub(crate) fn bind(cx: &mut App); // the editor's actions in "Editor", without Underline (R-8)
// keys.rs: actions Edit, Save, ToggleRich, NewItem
```

- [ ] **Step 1: Write the tests first.**
  - `a_mode_switch_keeps_the_text_and_the_undo_history`
  - `undo_back_to_the_baseline_makes_the_editor_clean`
  - `a_normalized_body_is_not_dirty_after_load_and_mode_switches`
  - `bold_italic_code_and_strike_work_from_keys_and_from_the_toolbar`
  - `ctrl_u_does_not_underline`
  - `tab_indents_inside_the_editor_and_f6_leaves_it`
  - `table_buttons_show_in_a_table_and_insert_a_row`
  - `an_open_link_event_opens_only_safe_urls`
  - `a_focused_edit_changes_only_its_own_bytes`: `replace_range` at one
    offset in each round-trip fixture.
  - `each_round_trip_fixture_survives_both_mode_switches`
- [ ] **Step 2: Implement** the host, style, toolbar and keys, as the
  design's "The editor host" says.
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands editor
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands keys
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- sh scripts/check-cli-no-gpui.sh
```

- [ ] **Step 4: Commit and ticket comment.**

## Part A Checkpoint

- [ ] **Step 1: Merge and run the full suite.** Run the merge-tree check
  and merge main if it moved. Then run the full suite and the
  `--features desktop` check and clippy.
- [ ] **Step 2: Review.** A fresh reviewer reviews Part A against the
  design's Part A, the N table and contracts 2, 3, 5, 12 and 13.
- [ ] **Step 3: Record.** Write the ticket comment.

---

# Part B: Editing, Saving And Review

## Task 5: New Intents

**Files:**
- Modify: `src/desktop/services/requests.rs`, `requests/build.rs`,
  `services/mod.rs` (`JobKind`, `ReadKind::{NewId, ObservePath,
  RelationshipCheck}`), `services/requests_tests.rs` (E-11 to E-14)

**Interfaces:**

```rust
// Intent variants added (requests.rs)
ContextPrepare { root: PathBuf, kind: AuthoringKind, id: ItemId, intent: ContextPrepareIntent },
DocumentCreate { root: PathBuf, id: ItemId, path: PathBuf, title: String, body: String },
TicketCreate { root: PathBuf, id: ItemId, title: String, ticket_type: String, status: String,
               project: Option<String>, team: Option<String>, body: String,
               deps: Vec<String>, parent: Option<String> },
DocumentMove { root: PathBuf, id: ItemId, observation: Option<String>,
               destination_path: PathBuf, destination_observation: String },
FolderCreate { root: PathBuf, path: String },
DocumentRepair { root: PathBuf, path: PathBuf, observation: String, source: String },
TicketRepair { root: PathBuf, path: PathBuf, observation: String, source: String },
```

- `confirmed()` is true only for the two repairs.
- `changes_items()` is true for all seven.
- `label()` gives "Prepare to edit", "Create document", "Create ticket",
  "Move document", "Create folder", "Repair document" and "Repair ticket".
- `TicketCreate` builds with `initials: None` (R-21).

- [ ] **Step 1: Apply E-11 to E-14 first.** The count becomes 28, and each
  new intent gets its rows. Run them: they fail to compile.
- [ ] **Step 2: Implement** the variants, `build` arms, job kinds and read
  kinds.
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands services::requests
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
```

- [ ] **Step 4: Commit and ticket comment.**

## Task 6: Edit, Save And Done

**Files:**
- Create: `src/desktop/model/edit.rs`, `edit_tests.rs`,
  `src/desktop/view/item/edit.rs`, `view/item/edit_tests.rs`
- Modify: `view/item.rs`, `services/drafts.rs` and
  `services/drafts/save.rs` (remove the `dead_code` allowances on `begin`,
  `edit` and `save`), `view/item/details.rs` (the Edit control)

**Interfaces:**

```rust
// model/edit.rs
pub(crate) enum Phase { Viewing, Preparing(DesktopRequestId), Loading, Editing, Review }
pub(crate) enum SaveState { Unsaved, SavedLocally { publication_pending: bool },
                            DiscoveryPending, CheckpointFailed, Changed, Refused(ResultCode) }
pub(crate) fn save_state(envelope_code: ResultCode, effects: &Effects, has_publication_remote: bool) -> SaveState;
pub(crate) fn base_for_edit(item: &ItemDto) -> Option<DraftBase>; // body through editor_text::load (R-11)
```

- [ ] **Step 1: Write the tests first.**
  - `model/edit_tests.rs`: `save_state_maps_every_row_of_the_design_table`
  - `view/item/edit_tests.rs`:
    - `edit_prepares_the_context_with_progress_before_the_editor_opens`
    - `edit_of_a_closed_ticket_is_not_offered`
    - `edit_of_an_uncommitted_primary_file_is_refused_and_nothing_is_made`
      (Review Focus 3: `RepositorySnapshot` before and after)
    - `a_discovery_pending_context_waits_for_resume` (R-13)
    - `the_first_real_edit_begins_a_draft_and_later_edits_update_it`
    - `save_is_offered_only_when_dirty_and_not_while_saving`
    - `a_document_save_commits_once_and_reads_saved_locally`
    - `a_ticket_save_keeps_unknown_metadata_by_value`
    - `a_normalized_item_opened_and_done_writes_nothing`
      (Review Focus 1 and contract 3: the repository snapshot is
      byte-identical)
    - `an_edit_undone_to_the_baseline_offers_no_save`
    - `an_unavailable_store_is_said_before_editing`
    - `done_with_a_draft_offers_the_close_choices`
    - `closing_the_tab_while_preparing_opens_no_editor` (Review Focus 5)
    - `a_save_with_a_publication_remote_reads_publication_pending`
    - `secondary_s_saves_and_secondary_shift_m_switches_mode`
- [ ] **Step 2: Implement** the phases, Edit, Save, the state line and Done,
  as the design's Part B says.
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands edit
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands drafts
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
```

- [ ] **Step 4: Commit and ticket comment.**

## Task 7: Restored Drafts, Close-Dialog Save And Late Results

**Files:**
- Modify: `view/item/draft.rs`, `view/close_draft.rs`, `view/item/reads.rs`,
  `view/item/edit.rs`, `view/item/draft_tests.rs` (E-8),
  `view/tabs_tests.rs` (E-9)

- [ ] **Step 1: Apply E-8 and E-9, and write the new tests first.**
  - `continue_editing_loads_the_drafts_text_on_its_own_base_and_saves`
  - `save_in_the_close_dialog_saves_then_closes`
  - `a_refused_save_from_the_close_dialog_keeps_the_tab`
  - `keep_draft_is_still_the_first_tab_stop_of_the_close_dialog`
    (R-16; the D2 keyboard test also stays green)
  - `typing_during_a_save_while_a_refresh_lands_keeps_the_newer_text`
    (Review Focus 2: `DelayingBackend` holds the save, the test types,
    refreshes and releases; only the saved revision retires)
  - `a_late_read_does_not_replace_a_dirty_editor`
  - `a_clean_editor_reloads_and_keeps_its_cursor` (R-19)
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::item
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands view::tabs
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands keyboard_tests
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
```

- [ ] **Step 4: Commit and ticket comment.**

## Task 8: Stale-Edit Review

**Files:**
- Create: `view/item/review.rs`, `review_tests.rs`
- Modify: `view/item/edit.rs`

- [ ] **Step 1: Write the tests first.** Use D2's `second_actor` child
  process to save the item between the desktop's read and its save.
  - `a_stale_save_opens_the_review_and_leaves_the_draft_file_unchanged`
  - `one_sided_changes_default_and_both_changed_fields_must_be_chosen`
  - `taking_theirs_for_one_field_and_mine_for_the_body_saves_exactly_that`
  - `cancel_leaves_draft_and_base_unchanged`
  - `a_second_change_during_review_reopens_it_on_save`
  - `normalization_alone_is_never_shown_as_a_change` (R-11)
- [ ] **Step 2: Implement**, as the design's "Stale-edit review" says.
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands review
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
```

- [ ] **Step 4: Commit and ticket comment.**

## Part B Checkpoint

- [ ] **Step 1: Merge and run the full suite.** Run the merge-tree check,
  then the full suite and the desktop-only check and clippy.
- [ ] **Step 2: Review.** A fresh reviewer, with the adversarial brief on
  saves, late results and the review: contracts 4, 6, 7 and 8.
- [ ] **Step 3: Record.** Write the ticket comment.

---

# Part C: Metadata, Relationships And Creation

## Task 9: Metadata Controls

**Files:**
- Create: `view/item/metadata.rs`, `metadata_tests.rs`, `model/metadata.rs`
- Modify: `view/item/edit.rs`

- [ ] **Step 1: Write the tests first.**
  - `each_named_control_edits_its_field_and_marks_the_tab_dirty`
  - `id_short_code_closure_and_unknown_metadata_are_not_editable`
  - `a_status_of_closed_saves_as_ordinary_text_and_the_ticket_stays_open`
  - `an_emptied_project_is_saved_as_absent` (R-27)
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands metadata
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
```

- [ ] **Step 4: Commit and ticket comment.**

## Task 10: Dependencies, Parent And The Cycle Check

**Files:**
- Create: `view/item/relations_edit.rs`, `relations_edit_tests.rs`,
  `model/relations_edit.rs`, `model/relations_edit_tests.rs`

- [ ] **Step 1: Write the tests first,** over real tickets.
  - `a_dependency_is_chosen_from_the_list_and_by_short_code`
  - `a_parent_is_changed_and_cleared`
  - `a_proposed_dependency_cycle_is_explained_with_its_tickets_and_save_is_unavailable`
  - `a_proposed_parent_cycle_is_explained`
  - `an_unresolved_dependency_is_shown_and_saves`
  - `an_incomplete_index_adds_its_note`
  - `a_late_check_does_not_replace_a_newer_one`
  - `an_unchanged_relationship_is_sent_as_unchanged`
  - `a_library_relationship_cycle_answer_is_explained_the_same_way`
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands relations_edit
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
```

- [ ] **Step 4: Commit and ticket comment.**

## Task 11: New Document And New Ticket

**Files:**
- Create: `view/create.rs`, `create_tests.rs`, `services/drafts/create.rs`,
  `services/drafts/create_tests.rs`
- Modify: `src/drafts/compare.rs` (`TargetState::NotCreated`;
  `restore_policy` allows it), `src/drafts/compare_tests.rs` (new tests
  only), `view/draft_comparison.rs` (its label), `view/documents.rs`,
  `view/lists.rs` (the New buttons), `keys.rs` (`secondary-n`)

**Interfaces:**

```rust
// src/drafts/compare.rs
pub enum TargetState { /* … existing … */ NotCreated }
pub fn is_new_item_base(base: &DraftBase) -> bool; // observation and source both None (R-20)
// services/drafts/create.rs
impl DraftRegistry {
    pub(crate) fn create(&mut self, id: DraftId, cx: &mut Context<Self>) -> Task<Option<DesktopRequestId>>;
}
```

- [ ] **Step 1: Write the tests first.**
  - `src/drafts/compare_tests.rs`:
    - `a_new_item_draft_of_a_missing_item_is_not_created_and_restorable`
    - `a_new_item_draft_whose_item_exists_is_unverified`
    - `an_unverified_base_with_a_source_is_still_item_not_found_when_missing`
  - `view/create_tests.rs`:
    - `new_document_prepares_and_opens_an_unsaved_draft`
    - `the_path_must_be_under_docs_end_md_and_be_free`
    - `save_creates_the_document_once_and_the_tab_becomes_an_edit_session`
    - `new_ticket_creates_with_its_dependency`
    - `an_occupied_path_at_save_keeps_the_draft`
    - `initials_required_points_to_settings`
    - `a_create_retried_after_a_lost_answer_creates_one_item` (worker
      `requeue`)
    - `a_not_created_draft_restores_after_relaunch`
- [ ] **Step 2: Implement** as the design's "Create" says.
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --lib drafts::
devenv shell -- cargo test --locked --test drafts
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands create
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands drafts
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

- [ ] **Step 4: Commit and ticket comment.**

## Part C Checkpoint

- [ ] **Step 1: Merge and run the full suite.** Run the merge-tree check,
  then the full suite and the desktop-only check and clippy.
- [ ] **Step 2: Review.** A fresh reviewer reviews Part C: contracts 9, 10
  and 11 for create, and ruling R-20.
- [ ] **Step 3: Record.** Write the ticket comment.

---

# Part D: Moves, Folders, Repair And The Gate

## Task 12: Move And New Folder

**Files:**
- Create: `view/move_document.rs`, `move_document_tests.rs`,
  `view/new_folder.rs`, `new_folder_tests.rs`
- Modify: `view/documents.rs`, `view/documents_tests.rs` (E-10),
  `view/item/details.rs` (Move…)

- [ ] **Step 1: Apply E-10, and write the new tests first.**
  - `move_commits_both_paths_and_the_tab_follows`
  - `a_collision_is_refused_and_explained_and_the_dialog_stays`
  - `a_stale_source_is_refused`
  - `move_is_unavailable_with_an_unsaved_draft` (R-23)
  - `new_folder_opens_its_dialog` (E-10)
  - `new_folder_creates_and_explains_local_folders_and_lists_it`
  - `an_invalid_folder_name_is_refused`
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands move_document
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands folder
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
```

- [ ] **Step 4: Commit and ticket comment.**

## Task 13: Repair

**Files:**
- Create: `view/item/repair.rs`, `repair_tests.rs`
- Modify: `model/item.rs` (`LATER` becomes four),
  `view/item/discussion.rs`, `model/item_tests.rs` (E-6),
  `view/item_tests.rs` (E-7)

- [ ] **Step 1: Apply E-6 and E-7, and write the new tests first.**
  - `a_nonconforming_document_is_repaired_after_a_preview_naming_the_new_id`
  - `a_nonconforming_ticket_is_repaired`
  - `a_stale_preview_is_refused_and_prepare_again_is_offered`
  - `an_ambiguous_identity_blocks_repair_with_its_reason`
  - `a_non_utf8_file_says_it_cannot_be_repaired_here`
  - `the_repair_view_never_passes_through_the_markdown_editor`
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Verify.**

```sh
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands repair
devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands model::item
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
```

- [ ] **Step 4: Commit and ticket comment.**

## Task 14: Keyboard, Privacy, Amendments, Checklist And Gate

**Files:**
- Create: `view/keyboard_tests/editing.rs`, `view/editing_privacy_tests.rs`
- Modify: `docs/RFC/desktop-information-architecture-and-editor.md`,
  `docs/Waves/wave-03-dogfooding.md`, the ticket body, the Cycle's
  acceptance table, the design's checklist and the execution ledger

- [ ] **Step 1: Keyboard tests.** Edit, mode switch, formatting, Save, the
  review, metadata, both pickers, Create, Move, New folder and Repair, by
  keystroke alone, with Escape and focus return.
- [ ] **Step 2: Privacy sweep.** A sentinel body is in no operation entry,
  notification, dialog message, error or `Debug` output, across:
  - save, stale save and `invalid_input`;
  - create;
  - move;
  - repair.
- [ ] **Step 3: Document amendments.** Apply the design's Document
  Amendments, and record each on the ticket.
- [ ] **Step 4: Full gate.** Run it detached, with a two-hour budget, and
  record the counts in the ledger.

```sh
git fetch origin && git merge origin/main
git merge-tree --write-tree HEAD origin/main
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo check --locked --features desktop
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo test --all-features --locked
devenv shell -- sh scripts/check-cli-no-gpui.sh
devenv shell -- cargo run --locked --bin manyhands-cli
```

- [ ] **Step 5: Native checklist.** Ask the owner to run the design's
  Linux Wayland checklist, and the desktop smoke test
  `devenv shell -- cargo run --locked --features desktop --bin manyhands`.
  Record the results in the Cycle's table.
- [ ] **Step 6: Whole-branch reviews.** Run a spec review and a code
  quality review, then fix what they find.
- [ ] **Step 7: Review-ready.**
  - Fill the Cycle's acceptance table: tests per row, "Held (Linux)", and
    Windows and macOS left to G1.
  - Write the review-ready ticket comment, and push the ticket branch.
  - A PR, a merge, closing the ticket and native CI each need the owner's
    own go-ahead. The ticket is closed only after review approval, in the
    final push before merge.

## Plan Self-Review

| Acceptance row or ruling | Task |
| --- | --- |
| Binding; R-1 to R-5 | 1 |
| Load normalization; round trips; R-8, R-9, R-10, R-12, R-28 | 2, 4 |
| Read view; R-6, R-7 | 3 |
| Not dirty; shared undo | 4, 6 |
| Edit and save; states; R-11, R-13, R-14, R-15 | 6 |
| Late results; recovery; R-16, R-17, R-19 | 7 |
| Stale edit; R-18 | 8 |
| Metadata; R-27 | 9 |
| Relationships; R-26 | 10 |
| Create; R-20, R-21, R-22 | 11 |
| Move; folders; R-23, R-24 | 12 |
| Repair; R-25 | 13 |
| Privacy; keyboard; native checklist; CLI isolation; regression | 14 |
| E-1 to E-5 | 3 |
| E-6, E-7 | 13 |
| E-8, E-9 | 7 |
| E-10 | 12 |
| E-11 to E-14 | 5 |
| E-15 to E-17 | 1 |

Placeholders: none intended.
- Interfaces named in later tasks are defined where they are produced:
  `editor_text` in Task 2, `EditorHost` in Task 4, the intents in Task 5,
  `Phase` and `SaveState` in Task 6, and `NotCreated` and
  `DraftRegistry::create` in Task 11.
- Task 0's sweep is how "no other test edits" is proven.
