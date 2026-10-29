---
title: "Wave 03 D3 Rich-Text And Source Authoring Execution Ledger"
date: 2026-10-26
status: "in-progress"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5X2F5H329J2Z8P1481XFK2J"
---

# Wave 03 D3 Rich-Text And Source Authoring Execution Ledger

## Base and heads

- Session start HEAD: `1f0aed9`.
- `origin/main`: `0464c42`.
- `git merge origin/main` was a no-op ("Already up to date").
- `git merge-base --is-ancestor origin/main HEAD` passed.
- `git diff --stat 6f1463c HEAD` over the library, desktop, test-support,
  schema, Cargo and CI paths is empty: no Baseline Fact changed since the
  design's refresh.
- **Merge 1, after Task 5b:** `origin/main` moved to `a372edb`, which merged
  ticket `01M57SKVF6TQ3QM6AXJ35JGGSY` (closure recovery gaps, local context
  synchronization). Merged as `da60fcf`.
  - Main changed no desktop file and closed none of the known-defect
    tickets.
  - Four registries conflicted: `command.rs`, `mutation_contract.rs`,
    `support/conformance.rs` and `support/mutation_golden.rs`. Each was
    resolved as the union of both sides.
  - Two merged counts were wrong and are now corrected:
    `OperationAction::ALL[..15]` in `read/status_tests.rs`, and
    `LOCAL_ACTIONS` (15) in `read_contract.rs`. The `read/dto.rs` comment
    now says fifteen.
  - **Interplay with main's local sync.** Main's local sync keeps its
    journal row pending until it ends, so `context prepare` and
    `context release` answer `recovery_required` at `begin_lifecycle`
    (`recovery.rs:791`). Local sync refuses to start while any row on the
    root is pending (`local_sync/record.rs:999`), a half-done release
    included. No outcome-table answer changed.
  - **Tests after the merge** (the owner's trimmed list):
    - clippy across all targets and all features: clean;
    - `mutation_contract`: 44;
    - `read_contract`: 72;
    - `mutation_bridges` `context`: 46;
    - `keeps_the_rules_of_the_boundary`: 1 each in bridges, replay and
      confirmation;
    - `--lib`: `context_release` 26, `recovery` 59, `mutation::` 108,
      `repository::read::status` 10;
    - the desktop binary: 574;
    - desktop clippy, fmt and the CLI isolation script: clean.

    Before the trim, these also passed: the full `--lib`, which caught the
    two counts above, plus `mutation_bridges` 166, `mutation_replay` 383,
    `mutation_confirmation` 183, `read_items` 82 and `local_authoring`
    140.
  - `merge-tree` against `a372edb` is clean.

## Baseline checks

All through `devenv shell --`, at `1f0aed9`.

| Check | Result | Count |
| --- | --- | --- |
| `cargo check --all-features --locked` | exit 0 | 155 s |
| `cargo fmt --check` | exit 0 | 14 s |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | exit 0, no warnings | 113 s |
| `cargo check --locked --features desktop` | exit 0 | 46 s |
| `cargo clippy --locked --features desktop --bin manyhands -- -D warnings` | exit 0, no warnings | 14 s |
| `cargo run --locked --bin manyhands-cli` (smoke) | exit 0, no window | 73 s |
| `cargo run --locked --bin manyhands-cli -- help` | exit 2, `usage_error`: `help` is not a subcommand (see CLI help copy) | 0 s |
| `scripts/check-cli-no-gpui.sh` | exit 0 | 1 s |
| `cargo test --all-features --locked` | stopped at the two-hour budget (exit 124) inside `ticket_closure`, the last test target; 0 failed. 31 libtest targets: 3,866 passed, 0 failed, 1 ignored (`read_characterization`, ignored before D3). 10 SSH targets: 579 cases passed. | 7,200 s (build 12 min 3 s) |
| `cargo test --all-features --locked --test ticket_closure` | exit 0, 73 SSH cases passed (run separately to complete the suite) | 823 s |
| `cargo test --all-features --locked --doc` | exit 0, 11 passed | 2 s |

Baseline total: **4,529 passed, 0 failed, 1 ignored**, across every test
target and the doc tests.

## CLI help copy

`manyhands-cli help` is not a subcommand. It exits 2:

```text
error: The command line is not valid; see --help.
code: usage_error
error: unrecognized subcommand 'help'

Usage: manyhands-cli [OPTIONS] [COMMAND]
```

`manyhands-cli --help` exits 0 with this text. Tasks 1 and 2 compare both
outputs against these copies (R-5): neither adds a CLI command.

```text
Usage: manyhands-cli [OPTIONS] [COMMAND]

Commands:
  repo       
  remote     
  key        
  host       
  document   
  folder     
  ticket     
  comment    
  index      
  poll       
  operation  
  id         
  request    
  path       
  item       
  conflict   

Options:
      --json             Write a JSON result.
      --non-interactive  Run without terminal prompts.
      --help             Show help.
      --version          Show the version.

Command catalog:
  repo list  List registered repositories. Available now.
  repo inspect  Inspect a repository, including an unregistered one. Available now.
  repo identity  Show repository identity. Available now.
  remote list  List redacted repository remotes. Available now.
  key list  List registered keys. Available now.
  key show  Show key metadata. Available now.
  key public  Print public key text. Available now.
  host list  List stored host pins. Available now.
  host inspect  Inspect a stored host pin. Available now.
  document list  List documents. Available now.
  document show  Show a document by ID or path. Available now.
  folder list  List document folders. Available now.
  ticket list  List tickets, open and closed unless filtered. Available now.
  ticket show  Show a ticket by ID or path. Available now.
  ticket ready  List ready tickets. Available now.
  ticket blocked  List blocked tickets and their reasons. Available now.
  ticket deps  Show ticket dependencies. Available now.
  ticket children  List child tickets. Available now.
  ticket cycles  List ticket relationship cycles. Available now.
  ticket plan  Show ticket execution batches. Available now.
  ticket critical-path  Show the ticket critical path. Available now.
  ticket find  Find tickets by short code. Available now.
  comment list  List comments for an item. Available now.
  index status  Show index status. Available now.
  poll status  Show polling status. Available now.
  operation list  List repository operations. Available now.
  operation show  Show a repository operation. Available now.
  id new  Generate a new item ID. Available now.
  repo create  Create a repository. Available now.
  repo enable  Enable Manyhands in a repository. Available now.
  repo remove  Remove a repository registration. Available now.
  repo identity-set  Set repository identity. Available now.
  remote add  Add a repository remote. Available now.
  remote remove  Remove a repository remote. Available now.
  remote select  Select a repository remote. Available now.
  key generate  Generate a key. Available now.
  key import  Import a key. Available now.
  key select  Select a key for a repository. Available now.
  key clear  Clear a repository's selected key. Available now.
  key remove  Remove a key registration. Available now.
  key delete  Delete a key. Available now.
  host approve  Approve a host pin. Available now.
  host replace  Replace a host pin. Available now.
  host observe  Observe a host key. Available now.
  request show  Show a request. Available now.
  request cancel  Cancel a request. Available now.
  folder create  Create a document folder. Available now.
  document create  Create a document. Available now.
  document save  Save a document. Available now.
  document move  Move a document. Available now.
  document repair  Repair a document. Available now.
  ticket create  Create a ticket. Available now.
  ticket save  Save a ticket. Available now.
  ticket repair  Repair a ticket. Available now.
  ticket slug-assign  Assign a ticket short code. Available now.
  index refresh  Refresh the index. Available now.
  index rebuild  Rebuild the index. Available now.
  path observe  Observe a path for authoring or repair. Available now.
  ticket check  Check ticket relationships. Available now.
  comment add  Add a comment. Arrives with C4.
  item sync  Synchronize an item. Arrives with C4.
  repo sync  Synchronize a repository. Arrives with C4.
  conflict show  Show a conflict. Arrives with C4.
  conflict resolve  Resolve a conflict. Arrives with C4.
  operation resume  Resume an operation. Arrives with C4.
  operation abandon  Abandon an operation. Arrives with C4.
  document promote  Promote a document. Arrives with C5.
  ticket close  Close a ticket. Arrives with C5.
  poll once  Poll a repository once. Arrives with C5.
  poll configure  Configure polling. Arrives with C5.
  poll pause  Pause polling. Arrives with C5.
  poll resume  Resume polling. Arrives with C5.

MANYHANDS_DATA_DIR: advanced/testing data-directory override; relative paths use the invocation directory.
```

## Document Amendments

All present at this HEAD.

| Amendment | Where |
| --- | --- |
| Desktop RFC, load normalization (N-1 to N-4, N-2b, acceptance, line endings) | desktop RFC lines 139-150 |
| Canonical RFC, "recorded load normalization (desktop RFC)" | canonical RFC lines 48-51 |
| Desktop RFC, unsupported constructs | desktop RFC lines 153-157 |
| Desktop RFC, resources and links | desktop RFC lines 158-167 |
| Desktop RFC, reload (R-19) | desktop RFC lines 168-169 |
| Desktop RFC, keyboard (R-59) | desktop RFC lines 170-173 |
| Wave, D3 section amendment | Wave lines 716-741 |
| Wave, D4 section amendment | Wave lines 773-778 |
| Ticket body, "merged into this branch" | ticket.md lines 39-40; no "rebased" remains |
| Follow-up ticket for other front-matter keys (decision 14) | `01M5X0YK5JQSMEZH59K38X8458` (`vh-j39te`), on its own branch `manyhands/ticket/01M5X0YK5JQSMEZH59K38X8458`, pushed; depends on this ticket |

## Sweep

Every `*.rs` under `src/` and `tests/` was searched for the Task 0 Step 3
strings. Production-only hits are not listed. Counts: 49 pre-existing tests
covered by E-1 to E-29 (a), 20 groups that will not break (b), 8 that will
or may break and are not listed (c). The full table is in the SDD
workspace's Task 0 report.

No exhaustive `match` in a test breaks on the new `TargetState`,
`LocalStep`, `OperationAction`, `RepositoryOperation` or `MutationDataDto`
variants beyond E-18, E-21 and E-22.

Unlisted tests (c). The owner ruled on 2026-10-26 (see Rulings):

| Where | Test | Reads | Verdict | Owner ruling |
| --- | --- | --- | --- | --- |
| `view/find_tests.rs:117` | a_ulid_opens_its_ticket_closed_or_not | `item-lifecycle` | will break (sheet, Task 6) | E-30 |
| `view/keyboard_tests.rs:429` | find_by_short_code_and_ulid | `item-lifecycle` | will break (Task 6) | E-30 |
| `view/tickets_tests.rs:211-212, 234` | arrows_move_in_the_list_and_enter_opens | `item-kind`, `item-id`, `item-path` | `item-id` will break; others may | E-30 |
| `view/documents_tests.rs:165, 180-181` | arrows_move_in_the_tree_and_enter_opens | `item-path`, `item-kind`, `item-id` | `item-id` will break; others may | E-30 |
| `view/find_tests.rs:169` | a_shared_short_code_lists_both_and_choosing_one_opens_it_by_id | `item-id` | will break (Task 6) | E-30 |
| `view/keyboard_tests/actions.rs:241` | a_nonconforming_entry_opens_by_keyboard | `item-path` | may break (Task 5b) | constraint, no edit |
| `view/item_tests.rs:370-379, 397-399` | inside the E-7 test | `item-nonconforming`, `item-path`, `item-problem*` | may break (Tasks 5b, 6) | constraint, no edit |
| `model/item_tests.rs:144` | a_comment_without_an_author_is_by_an_unknown_author | `comment_author` | may break (Task 8) | constraint, no edit |

Implementation constraints from the ruling: Inspect tabs keep `item-path`,
`item-nonconforming`, `item-problem*` and `item-source` visible; the header
keeps `item-title`; the pane root keeps `item-view`; `comment_author` and
`UNKNOWN_AUTHOR` stay. If any cannot hold, the task stops for the owner.

From the Task 0 review, the same rule keeps D1's banner and state IDs that
`tabs_tests.rs`, `item_tests.rs` and `draft_tests.rs` read: `item-failure`,
`item-failure-code`, `item-retry`, `item-retry-button`,
`item-not-refreshed*` and `item-recovery`. No D3 element, the state pill
included, uses the ID `item-state`, which `tabs_tests.rs:628` asserts is
absent.

Notes for later tasks: Task 22 leaves `targets()` in
`model/drafts/rows_tests.rs` as it is. No named edit covers it, and
`NotCreated` breaks nothing there (see Rulings). Tasks 1 and 2 keep the
`Command::ALL` invariants in `command.rs`.

## Task log

| Task | Commits | Tests run | Result | Notes |
| --- | --- | --- | --- | --- |
| 0 | this ledger's first commit | baseline above: 4,529 passed, 0 failed, 1 ignored | complete | E-30 added by the owner; see Rulings |
| 1 | `feat: add context prepare` (this commit) | mutation_bridges `prepare_` → 17, `keeps_the_rules_of_the_boundary` → 1; mutation_replay / mutation_confirmation `keeps_the_rules_of_the_boundary` → 1 each; mutation_remote same filter → 2 SSH cases; mutation_contract `every_` → 5, `the_fixture_directories_are_exactly_the_registered_cases` → 1; lib `mutation::` → 107; clippy all-targets/all-features clean; fmt clean; CLI `help` (exit 2) and `--help` (exit 0) equal the Task 0 copies | complete (review fixes in `e94d5c2`) | `discovery_pending` of `context prepare` offers `request.retry`, not `operation.resume`: the row keeps step `worktree_observed`, which `operation resume` leaves to the original request. The owner kept the library as it is (R-77). Its schema's `kind` enum broke `read_contract published_enumerations_list_exactly_the_registered_values`, which its Verify lines did not run; fixed in Task 2 by E-31 |
| 2 | `feat: add context release` (`e32a313`); `fix: address the Task 2 review of context release` (this commit) | lib `context_release` → 24, `cleanup` → 47, `recovery` → 54, `read::status` → 10, `mutation::` → 108; mutation_bridges `release_` → 17, `keeps_the_rules_of_the_boundary` → 1, `prepare_` → 17; read_contract `published_enumerations_list_exactly_the_registered_values` → 1, `every_published_` → 2; mutation_contract `every_` → 5, `the_fixture_directories_are_exactly_the_registered_cases` → 1; clippy all-targets/all-features clean; fmt clean; CLI `help` (exit 2) and `--help` (exit 0) equal the Task 0 copies | complete (review fixes in this commit) | Every row of the release table passes, the resume row through a real `operation resume`. E-31 (owner) cleared the block: `published_enumerations_list_exactly_the_registered_values` had failed since Task 1's `context_mutation.schema.json` `kind` enum. The review fixes add `release_whose_refresh_fails_is_finished_by_the_same_request_too`, which answers `ok`, and correct the `BeforeContextBranchRelease` doc. The worktree presence check is now one `ContextCleanup` helper. The domain tests name their cases in their messages, and the promotion refusal asserts its discarded row |
| Part 0 checkpoint | `docs: record R-78 and the Part 0 review rulings` (`16a5ac1`); `fix: discard a refused branch-only release row` (`d818519`) | at `d818519`, in full: mutation_bridges 166, mutation_replay 308, mutation_confirmation 183, mutation_remote 88 SSH cases, mutation_contract 44, read_contract 71, local_authoring 140, `--lib` 1,297; all passed, none ignored. main `0464c42` unchanged; merge-tree clean | complete | Review approved (0 Critical, 0 Important, 4 Minor). Minor 1 became owner decision R-78 (Known Limit and two Task 17 tests), Minor 4 a step-6 correction, Minor 2 fixed, Minor 3 parked; see Rulings. A first run at `65e4d57` had 7 `--lib` failures, all child spawns through `current_exe()`, because a concurrent build replaced the test binary mid-run; the run was repeated with nothing else building |
| 3 | `feat: add the editor crates, their keys, the read-only guard and load normalization` (`0889477`); `docs: record Task 3 in the ledger and on the ticket` (`fce9859`); `fix: address the Task 3 review` (this commit) | desktop-test-support `--bin manyhands`: `editor` → 26, `read_only` → 15, `keyboard_tests` → 23; `check --features desktop` and `clippy --features desktop --bin manyhands` clean; fmt clean; clippy all-targets/all-features clean; `check-cli-no-gpui.sh` exit 0 | complete (review fixes in this commit) | Both stop points passed: the feasibility test passed on the first run, and all 11 guard tests pass with the guard of R-61. Lockfile: two dependency-list lines (`image`, `zorite-markdown`), no new `[[package]]`, one `gpui-pre`. Ruling P-1 applied (`normalized_sorts_a_tickets_deps_by_id`). Added beyond the brief's list: `underline_has_no_key_and_escape_reaches_the_bindings_around_the_editor` (keys::bind) and `a_reload_shows_the_new_body_and_keeps_the_cursor` (R-19). `open_editor` installs an interim Kit-theme `SyntaxStyle`, which Task 7's `page_style` replaces; read editors need rich text for checkboxes, links and R-66. The ReadOnly tracks pointer presses for R-66 and hands `step_over_blocks` a click as `at_end: false`. Review fixes (R-79, owner): the guard reverts exactly with a whole-text `replace_range`; under the old `set_text` revert, `a_change_on_a_body_the_pin_loads_differently_twice_is_reverted_exactly` spun at full CPU until a 90 s bound killed it, and now passes in 0.12 s. Added `set_exact` (Task 18), the fact-81 fixtures `f81_inner_pair` and `f81_inner_fence`, `a_second_load_changes_both_shapes_of_fact_81` and `load_is_idempotent_over_the_fixtures` (renamed). Minors 1 to 4 and 6 fixed: the round trips load unchanged, `beyond` checks its range, inline math and word moves step over blocks, the cursor is set before typing, and the undo test says what it proves |
| 4 | `feat: add the item icons, the item colours and the app's own tab strip` (`c97899c`) | desktop-test-support `--bin manyhands`: `view::tabs` → 29, `view::settings` → 93, `tokens` → 1, `assets` → 2, `keyboard_tests` → 23; `check --features desktop`, `clippy --features desktop --bin manyhands` and clippy all-targets/all-features clean; fmt clean | complete (review fixes in this commit) | E-27, E-28 and E-29 applied as named (E-29 also in `discard_deletes_only_that_draft`, whose Two has a stored draft). The 36 icons come from the Kit's own `gpui-kit-assets` catalog through `icon_assets!`; no SVG is vendored. `ItemColors` also carries the workspace handoff's `accent` and `faint` for the tab lines. `content.rs` passes whether the content region has focus to `TabContent::render`; `ItemPane::drafts` is a new accessor. The tooltip is a `Tooltip` on the tab, tested through `tab_info`. Review fixes: the close button is 20 × 20 with the Kit's 12 px `close` icon; the `ItemColors` allowances are per field and name Task 5b, 7, 8, 9 or 10b; tab width is asserted (at most 260 px, not collapsed, title on one line); the active line states `amber` or `faint` in its accessible value, which the test reads under the real focus. The tooltip keeps the repository's directory name, as the sidebar's `RepositoryEntry::name` does |
| 5a | `feat: lay out the item tab over its own scroll region` (`1bcc366`); `docs: record Task 5a in the ledger and on the ticket` (`9fe4c0b`); `fix: address the Task 5a review` (this commit) | desktop-test-support `--bin manyhands`, after the review fixes: `view::item` → 36, `view::scroll` → 2, `view::tabs` → 29, `view::settings` → 93, `view::workspace` → 25, `view::drafts` → 41, `keyboard_tests` → 23; `check --features desktop`, `clippy --features desktop --bin manyhands` and clippy all-targets/all-features clean; fmt clean. Outside the gate: `view::relationships` → 8 | complete (review fixes in this commit) | E-25 applied to both scroll tests: they scroll the item's region, and Back to top under a 200-line body is the control below the fold (names kept). `opening_an_item_writes_nothing` unchanged and passing; every element ID of Task 0's sweep stays. Each item pane has its own `ContentScroll` (`region()`), sharing the reveal state with the column's, so each tab keeps its own offset; `Workspace::scroll_target` gives the page keys the shown item's region. The page is white (`page`, `page_text`), its column 760 px and centred, at least the region's last-frame height, with a canvas that asks for one more frame when that height changes. The scrollbar is the Kit's `Scrollbar` laid over the region, thumb `page_border_strong`; those three `ItemColors` allowances are removed. Outside the File Map row: `view/item/discussion.rs` (Back to top in its header row) and `view/item/tokens.rs` (the allowances). Review fixes: D1's details of the selected repository now end the item's own scroll region, after the discussion (owner, 2026-10-27, revised); the workspace hands each opened pane a `RegionEnd` that builds them as the pane draws, revealed by the pane's region, and the item tab is alone in the content column. In the 1440 × 700 window the region is 270.5 px (422 less the 34 px strip and the 117.5 px header), not about 58 px. The item tab and pane drop their zero minimum, so a short window scrolls the content column instead of overlapping the header. The thumb keeps `page_border_strong` when hovered and dragged. Not tested (owner, 2026-10-27: layout needs no tests): the details' place, the region's height, `follow_height`'s extra frame, the short window, and the Minor 3 coverage items; a throwaway probe checked the first, second and fourth and was removed |
| 5b | `feat: give the item tab its header, chips, pill and banners` (`4b2c600`); `docs: record Task 5b in the ledger and on the ticket` (`63566e7`); `fix: address the Task 5b review` (this commit) | desktop-test-support `--bin manyhands`, after the review fixes: `view::item` → 48 (12 in `header_tests`), `keyboard_tests` → 23, `model::item` → 9 (before them: 47, 23 and 9); `clippy --features desktop --bin manyhands` and clippy all-targets/all-features clean; fmt clean. Outside the Verify list, for the stop rule: the whole desktop binary once before the review, 573 passed | complete (review fixes in this commit) | E-6's label change applied: renamed `the_unavailable_actions_are_the_designs`, Sync is Share, ellipses on Approve and merge… and Close ticket… (Edit and Repair stay in `LATER`). E-7's first half applied (lines 386–399 only): the Inspect tab's `later-action` rows are the header's Edit, Share and Close ticket… (the item's kind is a ticket) and Repair in the "Needs fixing" banner, without Comment. The header: breadcrumb, title (`item-title`), chips emitting `OpenDetails` (allowance names Task 6), the pill (`item-pill`, never `item-state`), Edit/Share/primary disabled with the note as tooltip and as a `later-note` element, and the banner slot, most severe first. D1's status, not-refreshed and recovery lines are banners with D1's IDs; an Inspect tab's problems are in "Needs fixing"; Comment ends the discussion. Details, relationships, problems of item tabs and the source stay on the page (Tasks 6 and 7); D2's draft banner stays on the page (Task 18). The three `header_*` `ItemColors` allowances are removed. Outside the File Map row: `details.rs` (title and recovery moved), `status.rs` (not-refreshed drops its own border), `discussion.rs` (Comment) Review fixes: "Needs fixing" shows D1's reason for every nonconforming file (owner, 2026-10-27; no code change). The slot keeps the table's order within a tone (`Banner` derives `Ord` in the table's row order). "Needs fixing", and an Inspect tab's problems leaving the page, key on the file being nonconforming (no ID), not on the tab being an Inspect tab; a conforming item read by path shows neither (new test). The labels are named constants in `model/item.rs` and `LATER` is built from them; `later_action` takes `Weight` and `NoteAt` and asserts its label is in `LATER`. Each later action's `later-action` group has `Role::Group` and the note as its `aria_description`, since the Kit button takes no description; the `later-note` rows stay. `Banner::Changed`'s allowance names Task 16 |
| 6 | `feat: open the details sheet from the header's chips` (`05e38a4`); `docs: record Task 6 in the ledger and on the ticket` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `sheet` → 14, `view::item` → 60 passed and 2 failed, `view::relationships` → 8, `view::scroll` → 2, `keyboard_tests` → 23, `view::find` → 8, `view::tickets` → 8 passed and 1 failed, `view::documents` → 4; `clippy --features desktop --bin manyhands` and clippy all-targets/all-features clean; fmt clean. Outside the Verify list, for the stop rule: the whole desktop binary once, 585 passed and 3 failed | blocked: three pre-existing tests need a ruling, and none is edited | The three: `view/item/header_tests.rs::every_chip_emits_open_details_for_its_field` (Task 5b's): the first chip's click opens the sheet, and the Kit's backdrop takes the next chip's click (fact 61). `::tab_order_runs_strip_header_chips_page` (Task 5b's): `copy-id` is no longer the page's first control, because E-23 moves it into the sheet. `view/tickets_tests.rs::arrows_move_in_the_list_and_enter_opens`, with E-30 applied as written: Escape gives focus back to All details, so the arrows that follow no longer reach the list. Proposed edits, each tried on a scratch copy (12 and 9 passed) and then reverted: an Escape after each chip's click; "the page holds focus" (`item-page`) in place of `copy-id`; and `secondary-2` after E-30's Escape. Applied as named: E-23 (seven `item_tests`, all eight `relationships_tests`, `copy_by_keyboard`, `each_relationship_group_is_followed_by_keyboard`), E-26 and E-30 (all five). The sheet: the Kit `Sheet`, 440 px, opened by the workspace from each pane's `OpenDetails` at the next frame. The chip is focused first, then the field. The rows are the design's (ticket, document), each field read only in its 32 px box at 75%. D1's IDs are kept, and the Facts keep D1's kind, ID and lifecycle. The four groups are in the sheet. `relationships` and `rel-*` are kept, and each group's visible heading gives way to its row's label. The Inspect link opens an Inspect tab of the item's file. Root's sheet layer is drawn inside the content region, deferred, so focus in the sheet is the region's and E-23's assertions stand unchanged. `unless_dialog` refuses while a sheet is open (R-60); Quit still acts, as behind a dialog. The workspace closes a sheet whose tab is not the one shown; `open_item` always does. A focused control that a reload removes leaves focus in the sheet (`focus.rs`, new test). An Inspect tab keeps D1's details on its page, and names a conforming file by its kind and ID (carried from Task 5b, new test). `OpenDetails`'s allowance is removed. Outside the File Map row: `workspace.rs` (the sheet's owner and request), `workspace/content.rs` (`ask_details`, `sync_sheet`, a test accessor), `workspace/focus.rs` and `tabs.rs` (the subscription) |
| 6 fix 1 | `fix: close the details sheet when quitting waits` (`1556fb8`); `docs: record Task 6 fix round 1 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `view::item` → 63, `view::tickets` → 9, `sheet` → 15, `view::workspace` → 25 (the quit path's own tests); clippy all-targets/all-features clean; fmt clean | complete | The owner's three test decisions applied exactly: `every_chip_emits_open_details_for_its_field` presses Escape after each chip's click, in both loops; `tab_order_runs_strip_header_chips_page` asserts `item-page` holds focus in place of the two `copy-id` lines; `arrows_move_in_the_list_and_enter_opens` adds `secondary-2` after E-30's Escape. Review Important 1: `wait_to_quit` (`workspace/lifecycle.rs`), the one path into the Finishing and SavingDrafts waits, now drops the workspace's sheet owner and any pending request, closes an open sheet, and only then focuses Quit now, since closing the sheet gives focus back to its chip. New test `quit_behind_the_sheet_closes_it_and_focuses_quit_now`: RED "the sheet closes", then GREEN. Review Minor 4: `closing_the_tab_by_its_action_closes_the_sheet` asserts focus is on the remaining tab, which it already was. Minors 1–3 parked by the controller |
| 7 | `feat: show the item's body in a read editor on the page` (`d839db7`); `docs: record Task 7 in the ledger and on the ticket` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `view::item` → 79, `view::tabs` → 29, `read_only` → 16, `keyboard_tests` → 23; then the whole desktop binary once, for the stop rule: 605 passed, 0 failed (589 before plus 16 new); `clippy --features desktop --bin manyhands` and clippy all-targets/all-features clean; fmt clean | complete | The item tab's body is in a read editor (`ReadOnly`) on the page; D1's source textarea stays for Inspect tabs only (R-64). `page_style` sets all 25 fields from the page tokens; `reveal_caret` (R-67) runs after each caret move while the read editor has focus and when it takes focus. The `ReadOnly` wrapper is now the read view's tab stop (zorite makes the editor's focus with GPUI's default, which Tab passes by) and hands focus to the editor after the frame; Shift-Tab leaves from the wrapper's place. Focus hooks are frame-time checks with `defer_in`, since GPUI sends focus events only to an active window, which the test window is not. A press on the page below the text puts the caret at the end; no press focuses the page itself (the Task 5a ring). A clean reload of a changed body is a load of the body read, once (R-79), keeping the cursor clamped; an unchanged body leaves the editor alone, so `set_exact` is not used here. E-1 to E-5 applied; E-1 and E-2 reach the sheet's Inspect link by Tab and Enter, since it sits below the sheet's fold. 16 tests in `view/item/page_tests.rs`: the brief's 14, plus `a_table_drag_is_reverted` (Review Focus 5, carried from Task 3) and `a_control_click_on_math_opens_no_math_menu` (carried from Task 3's review). No D3 test from Tasks 3–6 changed |
| 7 fix 1 | `fix: focus the read view from any press on the page` (`7140ea8`); `docs: record Task 7 fix round 1 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once at the end: `view::item` → 82; `clippy --features desktop --bin manyhands` (a `cfg(test)` gate changed) and clippy all-targets/all-features clean; fmt clean | complete | Review Important 1: a press on the page that no control took focuses the read editor, its caret to the end only below the text; with no read view the press goes on, so focus goes into the content region as in D1. The page's ring shows only after Back to top (a flag), never after a press. New tests `a_press_beside_the_text_focuses_the_read_editor` (RED "the read editor has focus") and `a_press_on_an_inspect_page_focuses_the_content_region` (RED "the content region"), then GREEN; the ring now has the ID `item-page-ring`. Minor 1: `page_border_soft` relabelled Task 8, `page_fill` Task 10b. Minor 2: `shift_tab_from_back_to_top_enters_the_read_view`, green at once (it pins the existing hand-off). Concern 5 (no `set_exact` for a reload) confirmed by the controller |
| 8 | `feat: show the discussion as threads with a read editor per comment` (`53824f5`); `docs: record Task 8 in the ledger and on the ticket` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `discussion` → 16, `view::item` → 88 passed and 2 failed, `view::scroll` → 2, `keyboard_tests` → 23; `clippy --features desktop --bin manyhands` and clippy all-targets/all-features clean; fmt clean. Outside the Verify list, for the stop rule: the whole desktop binary once, 620 passed and 2 failed (608 before plus 14 new) | blocked: two D3 tests from earlier tasks need a ruling, and neither is edited | The two: `view/item/header_tests.rs::tab_order_runs_strip_header_chips_page` (Task 5b's): the header's new Comments {n} is an available action, so a tab stop between the strip and the first chip (handoff tab order: strip, header actions, chips), and "Tab stays in the strip until the header" fails (`header_tests.rs:882`). `view/item/frame_tests.rs::back_to_top_returns_and_focuses_the_page` (Task 5a's): at 700 px the region is 224.5 px tall and the designed discussion 295.5 px (padding 28/48/48, cards), so End leaves Back to top above the fold and its click fails "not visible". Proposed edits, tried on scratch copies (each passed) and then reverted byte for byte: in the first, Tab from the strip reaches `item-comments`, then one Tab reaches the first chip, and on the way back one more Shift-Tab passes `item-comments`; in the second, its first `shell.keys("end", cx)` becomes `shell.keys("secondary-shift-c", cx)`. Built: `model/discussion.rs` (`threads`, `display_name`, `avatar_color`, `comment_time`, `initials`); the discussion on the `discussion` ground in a 664 px column, one card per root, every reply at the one indent (36 px further in, to 50), "Reply to {name}" only past depth 1 (R-53), avatars from the whole `created_by` with white at 4.5:1 or better (R-56), times at the offset found as `confirm.rs` finds it, per comment (fact 80), a malformed entry's path and guidance. One `ReadOnly` per comment body, made when the list lands, kept by ID, reloaded only when its body changed (R-63, R-79), with R-67's caret reveal and per-frame focus hook. Comments {n} after the pill (a tab stop, with a count badge), `secondary-shift-c` and the floating button jump with `scroll_to_top_of_item` and focus the "Discussion" heading (no tab stop); the floating button hides with no comments and once the header is 60 px inside the region (`bounds_for_item` plus `offset()`), and a prepaint check redraws when the jump moved the region after render. Back to top is `page-teal`; Comment's note is `page-muted` (`NoteAt::BelowOnPage`). E-24 applied. Task 8's allowances on `page_border_soft` and `discussion` removed. 14 new tests: the brief's 11, plus `initials_take_the_first_and_last_words`, `a_reload_keeps_each_comments_read_editor` and `tab_runs_through_each_comment_and_reveals_its_caret`. Outside the File Map row: `view/confirm.rs` (`local_offset` visible in `view`), `view/workspace/render.rs` (the key's handler, so it acts from the strip too, refused behind a dialog or sheet), `view/item/page.rs` (the discussion's child index and what lies over the region), and the split `view/item/discussion/{rows,jump}.rs` |
| 8 fix 1 | `fix: apply the Task 8 test rulings and the review's minors` (`fcaddbc`); `docs: record Task 8 fix round 1 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `discussion` → 18, `view::item` → 91 (the two tests Task 8 was blocked on now pass); clippy all-targets/all-features clean; fmt clean | complete | The controller's ruling applied exactly as proposed (rustfmt wrapped the two added asserts): `header_tests::tab_order_runs_strip_header_chips_page` waits for `item-comments`, then one Tab to the first chip, and one more Shift-Tab passes `item-comments` on the way back; `frame_tests::back_to_top_returns_and_focuses_the_page`'s first `end` is `secondary-shift-c`. Review minors: 2, the discussion's status line takes `page-muted` (`status_in`; every other caller unchanged); 3, a time with no local offset says " UTC" (`shown_time`, new test `a_time_with_no_local_offset_is_marked_utc`, RED "Oct 20, 10:14" then GREEN; E-24's time line now calls `shown_time` with the offset it passes, so it holds on non-Unix targets too); 5, `the_comments_button_and_its_key_jump_and_focus_the_heading` asserts one Tab from the heading reaches Back to top and `secondary-shift-c` does nothing behind the open sheet (both passed at once: they pin what the review read); 6, the body sync builds a `HashSet` of listed IDs once a frame; 7, `avatar_initials` gives "?" for a blank author too (new test `a_blank_or_missing_author_has_no_initials`, RED "UA" then GREEN); 8, `page::DISCUSSION_CHILD`, no alias. Minors 1, 4 and 8's helper half parked by the controller. No desktop-only clippy: no `cfg(test)` support or feature gating changed. Outside the File Map: `view/item/status.rs` (`status_in`) |
| Part A fix 1 | `fix: bring the shown tab into view and apply Part A review minors` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `view::tabs` → 30, `sheet` → 16, `view::item` → 95, `discussion` → 19, `read_only` → 17, `keyboard_tests` → 23, `queue` → 6 (the three new request-queue tests), plus `scroll` → 8 and `view::workspace` → 27 for the layout change; clippy all-targets/all-features clean; fmt clean | complete | Important 1: the strip had no width limit (the Content region is `flex_1` with an automatic minimum, so it grew to the strip's 1300 px and nothing scrolled), so `render.rs` gives the region `min_w_0` and the strip `w_full().min_w(0)`. Then `tab-strip` tracks a `ScrollHandle`; a per-frame check (`TabContent::reveal`) calls `scroll_to_item` when the shown tab changes and when a tab's focus (or its close button's) is newly taken. New test `the_strip_scrolls_the_shown_tab_into_view` (six tabs of ~30 characters, window 1440 x 1400 as the harness sets; the tab must lie inside the strip): RED "the newest tab is in view" with only the width fix, GREEN after. Minor 1: the sheet's description is `VIEWING_PRIVATE` for an item read from an Active context; new test `the_sheet_names_the_private_copy_of_an_item_with_a_context` (RED by mutation). Minor 5: new tests `typing_and_paste_in_the_tab_queue_no_request`, `viewing_queues_no_request`, `comment_bodies_queue_no_request`. Minor 6: new test `undo_and_redo_with_a_history_never_reach_the_editor`: fails with the `Undo` stop removed (4 notifications); the `Redo` stop is unobservable behind the guard (only a leaked undo fills the redo stack and the revert clears it), so removing it leaves the test green. No desktop-only clippy: no `cfg(test)` support or feature gating changed |
| Part A fix 1b | `fix: show a kept draft in the closed and needs-attention pill` (this commit) | desktop-test-support `--bin manyhands`, once: `view::item` → 96; clippy all-targets/all-features clean; fmt clean | complete | Review Minor 2, owner decision 2026-10-27: `PillState` gains `ClosedDraftKept` ("Closed · unsaved changes kept", archive) and `NeedsAttentionDraftKept` ("Needs attention · unsaved changes kept", circle-alert); order unchanged. New test `the_pill_adds_a_kept_draft_to_closed_and_needs_attention`. Controller ruling: the two `draft_kept = true` assertions and the comment in `the_pill_reads_each_viewing_state` now expect the combined states; no other line of it changed |
| 9 | `feat: add the edit editor's host` (`a8e3cd8`); `docs: record Task 9 in the ledger and on the ticket` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `editor` → 52, `keys` → 28, `keyboard_tests` → 23; `clippy --features desktop --bin manyhands`, `check-cli-no-gpui.sh` and clippy all-targets/all-features clean; then `view::item` → 96 and the whole desktop binary once, for the stop rule: 650 passed, 0 failed (631 before plus 19 new); fmt clean | complete | `view/editor/host.rs`: `EditorHost` (the brief's interface, plus `place(done, scroll)`, `editor()` and `is_rich()`), `EditorHostEvent`, all with the allowance naming Task 14. The edit editor is its own entity (R-62), loaded through `open_editor`; the baseline is its `text()`; the cursor given is clamped (R-65). Rich text and Markdown are the same editor (`set_markdown_style` / `clear_markdown_style`); Markdown sets the theme's monospace family at 13.5 px in `page-text`. R-12: every `Changed` and notify compares the text with the last seen; a difference is `Edited`, and `DirtyChanged` on a flip, so undo and redo count. `mark_saved` moves the baseline only (R-69); `rebase` goes through `set_exact`, then the cursor, clamped (R-18, R-79), and emits no `Edited`; `reload_clean` loads a changed body once with `set_text(crlf_to_lf)` and the cursor clamped, and leaves an unchanged body or a dirty editor alone (R-19). The host is a tab stop that hands focus to the editor, as `ReadOnly`'s wrapper does; Tab and Shift-Tab indent and outdent, F6, Shift-F6 and the region keys leave. One `pressed` flag serves R-66 and R-59: set by a capture-phase press of any button; cleared by the arrow and word moves (capture), by `Changed` and `SelectionChanged` other than the press's own (those arrive before the next frame, which clears `settling`), and by `EditorEscape`; taken by a block event. Escape with it set dispatches `Dismiss` through the editor's focus handle, else focuses Done. `step_over_blocks` reused; `entered_by_pointer` moved from `read_only.rs` to `view/editor.rs` and shared. Keys: actions `Edit`, `Save`, `ToggleRich`, `EditorEscape`, `InsertLink`, `NewItem`; `escape` and `secondary-k` bound in `"EditHost > Editor"`, `secondary-shift-m` (`ToggleRich`, handled by the host) in `"EditHost"`; `Edit`, `Save`, `InsertLink` and `NewItem` carry allowances naming Tasks 14, 15, 10b and 23. One shared caret-reveal helper, `view::editor::CaretInView`, replaces the copies in `view/item/page.rs` and `view/item/discussion.rs` with no change of behaviour (their tests unedited). 19 new tests in `view/editor/host_tests.rs`: the brief's 16, plus `rebase_puts_the_result_in_exactly_and_restarts_undo`, `a_clean_reload_shows_the_new_body_and_keeps_the_cursor` and `the_edit_host_keys_act_only_in_the_edit_host`. RED: 17 of 19 failed against an interface stub; the 2 guards (`ctrl_u_does_not_underline`, `a_normalized_body_is_not_dirty_after_load_and_mode_switches`) and five more checks were proven by mutation. No pre-existing test and no D3 test from Tasks 3–8 changed |
| 9 fix 1 | `fix: leave a clean editor alone when the re-read is its own text` (`a5152e8`); `docs: record Task 9 fix round 1 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `editor` → 53, `keys` → 28; clippy all-targets/all-features clean; fmt clean | complete | Review Important 1: `reload_clean` now leaves a clean editor alone when the body is the text last saved or loaded (`crlf_to_lf(body) == baseline`) or the body first loaded (`load(body) == baseline`), so a fact-81 text the editor holds never re-enters through `set_text` (R-79) and undo survives a post-save re-read (R-69). New test `a_clean_reload_of_the_text_the_editor_holds_leaves_it_alone`, over both second-pass fixtures (the edit is a Markdown-mode paste, which does not rewrite): RED "f81_inner_pair: the text is unchanged"; with the loop reversed, "f81_inner_fence: the text is unchanged"; then GREEN. Minor 1: the four action allowances say "Called from Task N." Minor 3: the capture-phase press listener notifies, so a frame follows every press and `settling` always ends; `escape_after_a_pointer_press_…` still covers the flag. R-59's `settling` reading and the one `pressed` flag accepted by the controller. Minors 2 and 4 parked by the controller. No desktop-only clippy: no `cfg(test)` support or feature gating changed |
| 10a | `feat: add the toolbar's host edits` (`d5ef576`); `docs: record Task 10a in the ledger and on the ticket` (this commit) | desktop-test-support `--bin manyhands`, once: `markdown_edit` → 10; clippy desktop-only, clippy all-targets/all-features and fmt clean | complete | `model/markdown_edit.rs` (461 lines, allowance naming Task 14) and `markdown_edit_tests.rs`: the brief's interfaces. `active_marks` follows zorite-editor 0.10.0 `toggle_wrap_pair` (`lib.rs:2393-2420`): empty selection none; marked when the selection starts and ends with the marker or the marker sits just outside both sides; so a selected `**x**` reads Bold and Italic, as the toggles do. List toggle: on when any non-blank line lacks the prefix, off when all have it; blank lines skipped unless all are blank; other list markers replaced; numbers renumber from 1. Cursor: a caret keeps its place in the text; a range lands at the end of the edited lines; a code fence or link lands after it, an empty fence inside it, a table in its first header cell. UTF-16 start rounds down and end rounds up to a char boundary. The 2 x 2 table is a header row and one body row. Tests were written with the code; RED shown by one mutation (empty-selection guard) failing `active_marks_follow_the_editors_unwrap_rule`. No pre-existing test changed |
| 10a fix 1 | `fix: harden table_at, collapsed UTF-16 carets and link addresses` (`c741c64`); `docs: record Task 10a fix round 1 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once: `markdown_edit` → 12; clippy all-targets/all-features and fmt clean | complete | Review minors: `table_at` is false inside a backtick or tilde fence (three or more, unclosed runs on); an empty UTF-16 range stays empty (rounded down); `insert_link` takes `<address>` for a space, parenthesis or angle bracket, encoding `<` and `>` as %3C/%3E. The test at the old `2..2` line now expects the collapsed result. Module is 513 lines |
| 10b | `feat: add the edit editor's toolbar` (`febb778`); `docs: record Task 10b in the ledger and on the ticket` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `toolbar` → 8, `keyboard_tests` → 23; clippy desktop-only and clippy all-targets/all-features clean; then `editor` → 62 and the whole desktop binary once, for the stop rule: 671 passed, 0 failed (663 before plus 8 new); gate (both clippies, fmt) clean | complete | `view/editor/toolbar.rs` (behaviour) with `toolbar/controls.rs` (the control table), `toolbar/render.rs` (drawing), `toolbar/popovers.rs` (Style menu, link popover) and `toolbar/table.rs` (table controls); the module carries the allowance naming Task 14. Editor actions (Undo, Redo, Bold, Italic, Strikethrough, Inline code) go through the editor's `FocusHandle::dispatch_action` (fact 80); host edits read the selection once through `selected_text_range` and `selection_from_utf16`, then one `replace_range` and `set_cursor`, so one undo step that the host's R-12 compare sees as an edit (dirty); an edit that changes nothing is skipped. Table controls are drawn only while `table_at` holds and call the editor's table methods; in Markdown, where the editor draws no table and `set_caret_table_align` reads drawn rows, alignment is a host edit writing the separator as the editor does (the tests compare it with the editor's own rich-text result). Active state (style name, marks, in-table) is read on the host's `Edited` and `SelectionChanged`, never per frame. Markdown draws formatting controls at 35%; mode switch, Undo and Redo stay at full strength. Keys: actions `ToolbarNext`, `ToolbarPrevious`, `ToolbarFirst`, `ToolbarLast`, `ClosePopover`; Left/Right/Home/End and `secondary-shift-m` (`ToggleRich`, forwarded to the host) in `"EditToolbar"`, Up/Down/Escape in `"EditToolbarMenu"`, Escape in `"EditToolbarLink"`. Only the first control is a tab stop; a click prevents focus, so the editor keeps it. The host handles `InsertLink` (`secondary-k`, `"EditHost > Editor"`) by emitting the new `EditorHostEvent::LinkRequested`, which opens the toolbar's link popover; `InsertLink`'s allowance is removed, and so are the allowances on `page_fill`, `toolbar_hover` (Task 10b) and `segmented` (Task 9) in `view/item/tokens.rs`, which the toolbar uses. Table tooltip says "Table" (Design Deviations). Status item: an empty slot for Task 14. 8 new tests in `view/editor/toolbar_tests.rs`, names as the brief. RED: 8 of 8 failed against a stub; 10 mutations each caught. No pre-existing test and no D3 test from Tasks 3–10a changed |
| 10b fix 1 | `fix: align the last column past a row's end, and rove the toolbar's tab stop` (`370387e`); `docs: record Task 10b fix round 1 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `toolbar` → 8, `markdown_edit` → 15, `keyboard_tests` → 23, `editor` → 64; clippy all-targets/all-features and clippy desktop-only (a `cfg(test)` module was added) clean; fmt clean | complete | Review Important 1: in Markdown, Align past a row's last cell (before its closing pipe, or at its end) now takes the last column, as the editor's `table_cell_at` clamps; new Markdown cases on the header and a body row are checked against the editor's rich-text result. RED "AlignLeft at 15, past the last cell of \"| A \| B \|\"", then GREEN. Minor 3: `align_column` and its table helpers moved to `model/markdown_edit/table.rs` (as `ColumnAlign` and `align_column`), with `table_at`, `in_code_fence` and the row checks, shared by both; `markdown_edit.rs` is 439 lines, and re-exports them, so Task 10a's 12 tests pass unedited. Inside a code fence it now aligns nothing, as the editor draws no table there. 3 pure tests in `markdown_edit/table_tests.rs` (separator rewrite and caret shift; past the last cell, a row with no closing pipe; adjacent tables, an escaped pipe, the separator row, a paragraph, a fence, no separator). RED against the moved code: the past-the-end and fence cases. Minor 1: the toolbar's tab stop roves (the focused drawn control, else Rich text); Shift-Tab from a control leaves in one press, RED against `d651fc3`'s toolbar "Shift-Tab leaves the toolbar". Minor 2: the Style menu has `MenuNext`, `MenuPrevious`, `MenuFirst`, `MenuLast` (Up, Down, Home, End in its context); Left and Right do nothing there and the menu stays open, RED against `d651fc3` "Left does nothing in the menu". Minor 4: Home and End in the link field are now tested (they stay in the field, the region is not scrolled). No ticket comment, as the brief says |
| 11 | `feat: open links from the read view, comments and the edit editor` (`f660313`); `docs: record Task 11 in the ledger and on the ticket` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `links` → 20, `view::item` → 96; clippy desktop-only and clippy all-targets/all-features clean; then `discussion` → 19, `editor` → 66 and the whole desktop binary once, for the stop rule: 694 passed, 0 failed (674 before, plus the 20 new); gate: both clippy runs and fmt clean | complete | `model/links.rs` (parse, `link_path` refusal and join, `nearest`) and `services/links.rs` (`resolve`: `observe_path` in the item's own context, then primary; `show_path` to tell a managed item from another file; then `list_documents` by file name or title, nearest first). Every editor opens `http`, `https` and `mailto` itself (`view/editor.rs::clicked_link`) and passes any other target as a `LinkTarget` event of its own type, so `ReadOnly` still emits no `EditorEvent`; the pane follows it (`banners.rs::follow_link`) and shows `Banner::Link` ("Link not found", "This kind of link is not opened", "Opened nearest match: <path>"), cleared by the next link. `EditorHost` handles both events; its allowances stay (Task 14). Additions beyond the brief's interfaces: `parse_wiki_link`, `link_path`, `folder_of`, `file_name`, `LinkResolution::Nearest`, `ReadKind::Observe`. No pre-existing or D3 test changed; no library gap |
| 11 fix 1 | `fix: open an unmanaged file's Inspect tab, and clear the link banner on any link` (`5bfe75c`); `docs: record Task 11 fix round 1 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `links` → 24, `view::item` → 96; clippy all-targets/all-features clean; fmt clean | complete | Controller ruling A: a file with no ID at a link's exact path opens its Inspect tab in the context where it was found (`ItemLocator::for_item`); `show_path`'s `invalid_path` stays not found (`NotManaged`). Review Minor 2: `clicked_link` now passes every clicked link to the owner, a browser link after opening it, so any link clears the banner. Minor 3: the resolver's doc says an observe that stays busy or fails goes on. Minor 4: encoded `..` and `%2F`, `a%3Ab.md` (a path by the raw scheme check, then a drive step) and the context-first nearest name are pinned; the model ones passed at once. The library's item context holds only its own item (`discovery.rs:932-935`), so the only in-context nearest candidate is the linking item itself, at zero steps: the service test is a zero-step tie with a primary document that sorts first, not "a closer one", and a mutation dropping `in_context` fails it. Ruling: tag clicks are a Known Limit, unchanged. Minor 5 parked |
| 12 | `feat: show local images in the read view, comments and the edit editor` (`1762192`); `docs: record Task 12 in the ledger and on the ticket` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `resources` → 13, `images` → 8 (6 service, 2 view), `view::item` → 98; clippy desktop-only and clippy all-targets/all-features clean; then `discussion` → 19, `editor` → 67 and the whole desktop binary once, for the stop rule: 719 passed, 0 failed (698 before, plus the 21 new); gate: both clippy runs and fmt clean | complete | `model/resources.rs`: `sniff` (magic bytes, SVG none), `contained` (R-34 checks 1–3: no `..`, canonical inside the tree's canonical root, no `.git`/`.manyhands` step as written or canonical, ignoring ASCII case and Windows' trailing dots, spaces and `:stream`, every link on the way resolving inside), `checked` and `read_checked` (checks 4–5: `symlink_metadata` regular file, identity kept, opened with `O_NOFOLLOW`/`O_NONBLOCK`, handle identity equal; `take(20 MiB + 1)`), `decode` (`ImageReader::with_format`, `Limits` by field: 4096 a side, 96 MiB; header first, then the tab's budget, then `decode()`, first GIF frame, BGRA) and `walk_docs` (R-33: no-follow, breadth first, skips `.git`/`.manyhands`, 10,000 entries). Identity is device and inode on Unix; on Windows stable `std` has no file ID (`file_index` is unstable), so size, creation and last-write times and attributes, never a reparse point. `services/images.rs`: `TabImages` over `Rc<RefCell<ImageCache>>`, one per item tab, attached to the read editor, every comment's and any `EditorHost` (`show_images`); the provider records a miss, each editor's render asks for one deferred start after the frame, loads run one at a time on the background executor behind the new `ReadKind::Image` gate, land with a generation check, notify the tab's editors, and start the next; the budget is `TAB_BUDGET` less the `Loaded` sizes held; `ItemPane::reload` clears it. Exact lookup: the item's own context, then primary; the first tree holding anything at the path decides. No pre-existing or D3 test changed; no library gap; no new dependency |
| 12 fix 1 | `fix: drop a tab's images from the atlas, and read the Windows file ID` (`6beb7f5`); `docs: record Task 12 fix round 1 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `resources` → 15, `images` → 12 (8 service, 4 view), `view::item` → 100; clippy desktop-only and clippy all-targets/all-features clean; fmt clean | complete | Review Important 1: `ImageCache::reset` gives back the `Loaded` images it drops, and `clear`, `locate` (now with `cx`) and the new `close` (from the pane's `on_release`) drop them from every window's atlas with a deferred `App::drop_image`; after `close` nothing more loads. RED: `a_reload_drops_the_old_images_from_the_atlas`, `closing_the_tab_drops_its_images_from_the_atlas` and `a_new_place_to_resolve_from_drops_the_old_images` each failed at "the entry is gone" after their preconditions held (`Window::has_image_atlas_entry` reaches the test window's atlas). Ruling 1: on Windows check 4 opens the canonical path with `FILE_READ_ATTRIBUTES`, every share mode and `FILE_FLAG_OPEN_REPARSE_POINT`, requires a regular file with no `FILE_ATTRIBUTE_REPARSE_POINT`, and keeps `VolumeSerialNumber` and `FileId` from `GetFileInformationByHandleEx(FileIdInfo)`, as `coordination.rs::file_identity` reads them; check 5 compares them from the read handle; no ID refuses; the time tuple is gone; `windows-sys` names throughout; not compiled here. Minor 1: `a_decode_over_96_mib_is_refused` (a hand-made 4096 by 4096 RGBA16 PNG, 128 MiB decoded, written by hand in milliseconds) fails when `max_alloc` is removed; `a_failed_src_is_never_looked_up_again` (missing everywhere, and refused) fails with 3 loads, not 2, when a Failed entry is asked again. The GIF with a 1 by 1 screen is not built: the encoder takes the screen from the first frame. Minor 2: `walk_docs` reads a folder only up to what is left of the bound (`the_walk_stops_at_its_bound`). Minor 3: the pane locates its images when a read lands, not per frame. Minor 4 parked. No pre-existing or D3 test from Tasks 3 to 11 changed |
| Part B fix 1 | `fix: decide image links without following them, add the toolbar hint, trap the link popover's focus` (`152d93f`); `docs: record Part B fix round 1 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `resources` → 24 (15, and 9 new in `resources::contain`), `images` → 12, `toolbar` → 8, `editor` → 67; the whole desktop binary, as the checkpoint's rerun: 734 passed, 0 failed; clippy desktop-only and clippy all-targets/all-features clean; fmt clean | complete | Review Important 1: R-34 check 3 is a walk from the root with `symlink_metadata` only, each link decided from `read_link` without following it (absolute, rooted or prefixed targets refused, even inside the root; a relative target walked from the link's folder, `..` taken in order from where the walk is; 40 hops), and `canonicalize` only after every link passed. The exact lookup uses the same walk (`locate`: nothing there, refused, or contained). The walk's calls go through a `Lookups` seam; tests record them. RED: the 9 new tests, run against the old algorithm ported onto the seam, all failed (a lookup through `docs/n` reached `outside/`, a refused path was made canonical first, or the absolute link inside the root was contained). The ruled edit to `dot_dot_symlink_escape_git_and_other_worktrees_are_refused` (relative `inside.png`) failed before it and passes after. Important 2: "Esc: Done · F6: next region" under the toolbar row, in `page-muted` at the toolbar's padding, no focus, no listeners; RED `left_right_home_and_end_move_along_the_toolbar` (label `None`). Minor 4: Tab and Shift-Tab bound in `"EditToolbarLink"` to `CycleLinkPopover`, which goes between the address and Insert; RED `secondary_k_opens_the_link_popover_only_in_the_edit_host` ("tab kept it open" on the second Tab). `services/links.rs` makes no filesystem call of its own: no change. Splits: `model/resources/contain.rs` (230 lines) and `services/images/load.rs` (115); `resources.rs` 406, `images.rs` 420. No other test changed |
| Part B fix 2 | `fix: make only the place the walk reached canonical` (`32b9679`); `docs: record Part B fix round 2 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `resources` → 27 (15, and 12 in `resources::contain`), `images` → 12; clippy desktop-only and clippy all-targets/all-features clean; fmt clean; the whole desktop binary left to the controller | complete | Re-review N1: `walk` returns the place it reached (the root and the names it found, none a link), and only that is made canonical, never the written path, which Windows resolves by name (`..` from the link's folder) and could take through a link the walk never decided. So that no Unix result changes, the walk finds no path past anything but a folder (`..` included), and refuses an empty target. `look()` now also checks each path given to `canonicalize`, but the root: below the root, with no link on it. RED, against `a21464e`'s walk: 6 of 12 failed, each "was made canonical through a link": `a_relative_link_inside_the_tree_is_followed_with_its_chain`, the `n2` half of `a_dot_dot_after_a_link_is_taken_from_where_the_link_led`, the relative half of `an_absolute_link_inside_the_tree_is_refused`, the 40-link half of `a_chain_past_the_hop_limit_is_refused`, the new `only_the_place_the_walk_reached_is_made_canonical` (the re-review's four entries, a temporary folder for `//host/share`; asserts `canonicalize` gets only the root and `docs/sub/L/pic.png`) and the new `a_dot_dot_after_a_file_is_refused`. With the reached place made canonical but no folder check, `a_dot_dot_after_a_file_is_refused` alone failed (`Contained(docs/x.png)`, fix 1 gave Refused); with it, 12 passed. N2: `a_relative_link_into_git_is_refused` (`docs/g -> ../.git` for `docs/g/x.png`, and `docs/gx.png -> ../.git/x.png`) passed before and after; the walk does not stop at the reserved step, and also looks up `.git` and `.git/x.png`, inside the tree, before refusing (ruling 3). Fix 1's macOS note no longer holds: a 40-link chain is now made canonical at its end, so the Linux-only assertion in the hop-limit test is only weaker than it needs to be (test unchanged). No other test changed |
| Part B fix 3 | `fix: refuse a step Windows reads as a device name` (`4ce79b6`); `docs: record Part B fix round 3 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `resources` → 29 (15, and 14 in `resources::contain`), `images` → 12; clippy desktop-only and clippy all-targets/all-features clean; fmt clean; the whole desktop binary left to the controller | complete | Ruling A (contract 12, a device is never opened): `device`, beside `reserved` in `model/resources/contain.rs`, matches a name as Windows reads it, on every system: the part before the first `.`, its trailing spaces trimmed, ignoring ASCII case, is CON, PRN, AUX, NUL, COM0 to COM9, LPT0 to LPT9, COM or LPT with a superscript 1, 2 or 3, CONIN$ or CONOUT$. It also ends the name at a `:`, as `reserved` does, so `CON:` and `lpt1:x.png` are devices too (a superset of the ruling). The walk refuses such a step, written or from a link's target, before any lookup of it. RED, against a `device` that returned false: `a_windows_device_name_is_matched_as_windows_reads_it` (all systems; failed at `CON`) and `a_windows_device_name_is_refused_without_a_lookup` (Unix; `LPT1.png`, `con.png`, `nul`, `Com3.tar.png`, `aux .png` as real files, and `imgs/n.png -> ../docs/NUL.png`; failed with `docs/LPT1.png` contained). GREEN: 14 passed; each device is refused with no recorded call on its step and nothing made canonical, and `console.png`, `lpt10.png` and `com.png` are contained. Ruling B: `a_chain_past_the_hop_limit_is_refused` (D3's own) drops the macOS note and the Linux-only guard, and asserts the 40-link chain contained, made canonical only at `docs/pic.png`, on every system; it passed at BASE on Linux too, so there is no RED. No other test changed |
| 13 | `feat: add the intents and read kinds the authoring commands use` (`72e7373`); `docs: record Task 13 in the ledger and on the ticket` (this commit) | desktop-test-support `--bin manyhands`: `services::requests` → 5; whole binary → 739; clippy desktop-only and all-targets/all-features clean; fmt clean | Done | E-11 to E-14 applied (count 29), RED then GREEN. `ReadKind::Observe` renamed `ObservePath` (ruling). `NewId` and `RelationshipCheck` carry a plain `allow(dead_code)` since no test builds them. |
| 14 | `feat: make Edit prepare the context, open the edit editor and feed the draft` (`0656cf3`); `docs: name the controller's ruling where Edit ignores keys` (`a289bdc`); `docs: record Task 14 in the ledger and on the ticket` (this commit) | desktop-test-support `--bin manyhands`: `edit` → 122; `view::item` → 124; `model::item` → 9; `toolbar` → 9; `drafts` → 128; `keyboard_tests` → 23; whole binary → 768; clippy desktop-only and all-targets/all-features clean; fmt clean | Done | First stopped BLOCKED, before any code: a probe showed nine D3 tests (Tasks 5b, 7, 8) break when Edit is real. Resumed on the controller's rulings (below). E-6's first half applied, RED (a compile failure) then GREEN. The nine tests edited as ruled. 24 view tests, 4 model tests and 1 services test added: RED with Edit's entry points stubbed (24 failed in `edit`, and the services test), then GREEN; 11 mutations, each caught. The 200 KiB smoke test takes about 190 s in a test build. |
| 14 fix 1 | `fix: wait for Resume in Loading and note the disabled Continue editing` (`bbb7943`); `docs: record Task 14 fix round 1 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `edit` → 122, `view::item` → 124; clippy desktop-only and all-targets/all-features clean; fmt clean | complete | Controller ruling (Minor 2): a `discovery_pending` prepare waits in Loading, with the pill "Getting a private copy ready…", Edit disabled and Resume; no read opens the editor meanwhile. RED: the extended test found Viewing; a mutation removing both pending guards is caught. Minor 3: the R-65 test uses the CRLF and `$$` body; RED by mutating the comparison to the raw body (cursor 0, not 5). Minor 1: the 200 KiB test forces one frame, not five: 193 s alone before, 156 s after (`edit` 209 → 171 s, `view::item` 228 → 192 s). Minor 4: the disabled Continue editing says "Available in a later release" (tooltip and description), outside `LATER`. Minor 7: D2's `save` allowance names Task 15. No ticket comment. |
| 15 | `feat: save from the edit editor, settle the session after a save, and show the save states` (`5412c58`); `docs: record Task 15 in the ledger and on the ticket` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `edit` → 146; `drafts` → 129; `view::tabs` → 30; whole binary → 793; clippy desktop-only and all-targets/all-features clean; fmt clean | Done | The five session-outcome rows (R-76) were written first and each answers as the design's table says; no stop. 22 view tests and 1 registry test written before the code: RED with a stub skeleton (21 of 22 view tests failed, 17 on the missing Save; the registry test heard no `DraftSaved`); `a_normalized_item_opened_and_done_writes_nothing` passes as a guard. GREEN after one correction to a side-assertion of row 1 of my own: the library writes `deps` in ID order on every save (R-72), so a `base_after_save` change made for the opposite was reverted. Two view tests added after GREEN, each shown to catch its fault by mutation; 10 mutations in all, 9 caught, one equivalent (a redundant guard, since removed). `DraftSaved` is emitted in `retire.rs`'s `settle`. No D2 session: the fields are sent as they are. Allowances: `created`/`created_contexts()` now name Task 17; `reload_clean`, `Phase::Review`, `Banner::Changed` are used and lose theirs. No main or Task 0–14 test changed; `edit_tests.rs` gains only a module declaration. |
| 15 fix 1 | `fix: save after a failed draft write, and follow D2 while it finishes a save` (`4364a74`); `docs: record Task 15 fix round 1 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `edit` → 152, `drafts` → 129; clippy desktop-only and all-targets/all-features clean; fmt clean | complete | Controller ruling 2: D2's `send_save` sends the session's text from memory when the flush before it failed, once, with no store-again loop; `settle` ends a session whose text is still what the save sent though never written, discarding its draft (any older revision with it; a failed discard leaves it stored). No main test pinned "nothing is sent after a failed flush" (searched D2's save, drafts and privacy tests). Minor 1: the tests pin each save string as a literal. Minor 2: the untouched-bytes test checks the load changed only line endings. Minor 3: while D2 retires or stores again the tab is saving ("Saving…", Save disabled); the key waits and saves after. Minor 4: an abandoned pending checkpoint ends the tab's run and its banner. 6 view tests written first, RED (four failed sends, the pill, the banner) then GREEN; one mutation (no discard) caught by two. No ticket comment. |
| 16 | `feat: make Done real, keep the session after Done, and ask before closing unsaved work` (`f10cc50`); `docs: record Task 16 in the ledger and on the ticket` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `edit` → 170; `view::item` → 172; `view::tabs` → 30; `drafts` → 133; whole binary → 817; clippy desktop-only and all-targets/all-features clean; fmt clean | Done | The eight session-outcome rows for Task 16 (R-76) were written first, with the eight other tests, and each answers as the design's table says; no stop. RED: all 17 failed against a stub skeleton (the refusal case at first used a service that knew no repository, which closed the tab; it now uses a ticket depending on itself, `relationship_cycle`). GREEN, then two checks added (Done disabled with its note in the review; the Drafts page lists the kept draft as held here) and one test (R-71: a never-created item's tab closes); 9 mutations, each caught. `attach` now relists the store, as a restore does: without it the Drafts page never listed an edit session's draft. The leave dialog (`close_draft.rs`) serves the close dialog and Done's two prompts; D2's restored-draft dialog is unchanged, and an edit session's no longer says "restored draft" (Task 14 review, Minor 6). The session holds its fields besides the body (`change_fields`, called from Task 20), so a metadata change alone is dirty. A never-created item is one whose base has neither observation nor source (R-20). Release on leave is `leave::release_on_leave`, empty for Task 17. Task 14's prepare steps moved to `edit/prepare.rs` unchanged, to keep `edit.rs` under 500 lines. No main or Task 0–15 test changed; `edit_tests.rs` gains only a module declaration. |
| 16 fix 1 | `fix: measure dirty against the saved copy, and stop Done on a pending checkpoint` (`b18610d`); `docs: record Task 16 fix round 1 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `edit` → 178, `view::tabs` → 30, `drafts` → 134; clippy desktop-only and all-targets/all-features clean; fmt clean | complete | Important 1: dirty is the session's fields against its base (the saved copy, R-69), or an item never created, for Done, closing, the pill, Save and the tab's unsaved mark; the host's own flag drives only `DirtyChanged` (`is_dirty` is now test-only). Two tests written first (undo past a save newer typing outlived stays dirty and keeps its draft; undo back to the saved copy is clean), RED then GREEN. Minor 1: a pending checkpoint waiting for Retry stops Done; RED then GREEN. Minor 5: "written" is D2's state at the decision, with one more write when newer text came; the new test passes on the old code too (`flush_now` writes again), so RED/GREEN come from a one-write flush, failing with the old Done and passing with the new. Minor 2: the refusal is `internal_error` from `FailurePoint::BeforeRequestDomainCall`. Minor 3: `SessionDiscarded` has `Ended` and `Landed`; the banner shows only on `Ok`, tested with a read-only drafts folder. Minor 4: Keep draft waits while a save is in flight. Minor 6: the session close message is read with its offers. Minor 7: the unix-only words are gated. Minor 8: the never-created Discard and the kept chip are tested. Note 2: the tab's unsaved mark reads the same dirty. Each of A, B, D, E, G and the mark was RED first. Minor 9 parked. No main or Task 0–15 test changed. No ticket comment. |
| 16 fix 2 | `fix: leave a dirty editor alone when the read after a save lands` (`a451de2`); `docs: record Task 16 fix round 2 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `edit` → 179, `view::item` → 181, `drafts` → 134; whole binary → 826; clippy desktop-only and all-targets/all-features clean; fmt clean | complete | Re-review N1: `post_save_read_landed` reloads only a clean edit editor (not dirty against the saved copy, no draft; design, Late results), not by the editor's own stale flag. Test first, `the_read_after_a_save_leaves_a_dirty_editor_alone`: "b" saved, "c" typed during the save, the tab's reads held while both are undone; RED on the old code (the late read replaced the text), then GREEN: the editor keeps its text and redo reaches "bc", the draft holds it, the tab reads dirty. N2 is the controller's note for Task 19. No ticket comment. |
| 17 | `feat: release a work context this run made when its tab leaves clean` (`0a78378`); `docs: record Task 17 in the ledger and on the ticket` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `edit` → 199; `drafts` → 137; `keyboard_tests` → 23; whole binary → 846; clippy desktop-only and all-targets/all-features clean; fmt clean | Done | The four session-outcome rows for Task 17 (R-76) were written first, then the brief's other tests; each row answers as the design's table says, so there was no stop. P-4 (controller note): `discarding_a_restored_draft_releases_the_context_continue_editing_made` is left to Task 18; the release path covers it, since any discard of a draft whose context is in `CreatedContexts` releases it. P-3: the never-created row is written now with Task 16's `as_new_item` hook; Task 22 proves it again on its own path. RED: 11 of 17 failed against a stub (no release sent, no stranded control). The six that keep a context passed trivially. Three tests were added: a save in flight, an Edit while the store is listed, and a context made again or made elsewhere. 13 mutations, each caught. Release logic is `services/release.rs` (`Releases`). Discards reach it through the registry's new `DraftDiscardLanded`; the tab hands over clean leaves and a prepare it closed on. A release leaves `CreatedContexts` as R-75 defines it, and `Releases` keeps what it released until this process makes the context again. The first version took the key out of the set, which broke Task 14's `closing_the_tab_while_preparing_opens_no_editor`; the implementation changed, not the test. R-78: the worker refuses such a retry, the operation area shows that Retry unavailable with its reason, Resume and Retry leave the tab viewing, and a prepare sent while the store is listed stops the release. R-40: "Release stranded work context" on a `recovery_required` refusal while the read shows no context. `Releases::pending` is for Task 24's Move. The Task 17 allowances are gone; `Services::created` and `created_contexts()` are now `#[cfg(test)]`. No main or Task 0–16 test changed; `edit_tests.rs` gains only a module declaration. |
| 17 fix 1 | `fix: hold releases while a tab edits, and let a refused release free Resume` (`48c6422`); `docs: record Task 17 fix round 1 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `edit` → 203; `drafts` → 138; clippy desktop-only and all-targets/all-features clean; fmt clean | complete | M2 (ruling): each item tab tells `Releases` whether it holds an edit session of its item, live or kept, Preparing and Loading included, and while one does no release is decided. M1: a closed tab's prepare hands over its own send mark. M3: Resume or Retry that R-78 refuses shows "Your private copy cannot be made ready again from this request: a release of this item was sent since. Abandon its pending operation in the operation area, then press Edit." M4: Discard while the draft begins waits for it, discards it, then releases. E (owner, 2026-10-28): a release refused with nothing touched (a `Blocked` or `Input` code, not `partial`, no `request.retry`) does not block the earlier prepare. R-78 now goes by the order attempts were queued in, so that prepare's resend still blocks the release's retry. Each new test was RED first (A, B, D, C's banner, E's reviewer path); 9 mutations, each caught; B's test fails only with both its guards off. The condition-3 test now sends its save through the worker, since the editing mark would otherwise mask it. The R-78 rule-2 test now uses a release that dies after acceptance (answer unknown). M5 parked. No main or Task 0–16 test changed. No ticket comment. |
| 17 fix 2 | `fix: keep a release blocking once any attempt was not a clean refusal` (`34b86ec`); `docs: record Task 17 fix round 2 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `edit` → 205; `drafts` → 138; whole binary → 852; clippy desktop-only and all-targets/all-features clean; fmt clean | complete | N1 (Important): `Refusals`, kept per request by the worker and by `Operations`, exempts a release only while its last attempt was refused with nothing touched and no attempt ever answered otherwise. Test first: a model test over the worker's events (an earlier prepare answered `busy`; a release answered `internal_error`, retried, then refused), since the library's answer after an abandon is not produced here; plus a unit test of `Refusals`. Both were RED, then GREEN. N2: a waiting Discard is answered only by the begin it waits for (a per-tab begin token), and Edit waits while it is open. RED showed Edit during the wait sending a new prepare. The token alone has no UI path once Edit waits. C nit: the banner adds "Abandon its pending operation in the operation area, then press Edit." only when the operation area lists the prepare's own row (its Abandon's operation ID); otherwise "Press Edit to try again." RED then GREEN, in the rule-2 test, after abandoning that row. 4 mutations, each caught. `item_kind` moved to `feed.rs`, its only user, keeping `edit.rs` at 495 lines. No main or Task 0–16 test changed. No ticket comment. |
| 18 | `feat: continue restored drafts, save from the close dialog, and apply late reads` (`1e4d4c7`); `docs: record Task 18 in the ledger and on the ticket` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `view::item` → 217; `view::tabs` → 34; `drafts` → 140; `keyboard_tests` → 23; whole binary → 866; clippy desktop-only and all-targets/all-features clean; fmt clean | Done | First round stopped BLOCKED before code: carried obligation 4 contradicts Task 14's `edit_reads_continue_editing_while_a_draft_is_stored`; the controller approved the proposed edit (ruling below). E-8 and E-9 applied. The session-outcome row for Task 18 (R-76), `a_pending_checkpoint_from_the_close_dialog_keeps_it_open_with_retry`, answers as the design's table says (the dialog stays open with Retry; the tab stays); no stop. RED: 6 of the 9 restored-draft and late-read tests, all 4 close-dialog tests, E-8, E-9, P-4 and the ruled Task 14 test failed; 3 late-read tests passed as guards (Task 16's rules already held). GREEN on the first full run. 6 mutations, each caught. Continue editing (banner, D2's section, Edit, `e`) claims a stored draft, then prepares; the editor takes the draft's text through `set_exact` (`EditorHost::put_exact`, caret 0); the base takes the read's token and context only by R-42's three checks (`model::edit::adopted_base`), and D2's draft moves onto it (`DraftRegistry::adopt_base`); otherwise the review opens. Save from a restored draft runs those steps, then saves. The close dialog's Save (both dialogs) stays open while it saves, shows a pending checkpoint with Retry or a refusal with its message and code, and closes the tab once the draft retires; `CloseWithDraft::Retry` is new. D2's banner is `Banner::StoredDraft` in the header slot, with Continue editing last. A late read never replaces a dirty or drafted editor; a clean one reloads only a newer body, keeping its caret, and a verified base takes the read. `NotCreated`: Continue editing and Save unavailable with "Available in a later release"; hook `continue_new_item` for Task 22. P-4 written (in `edit_release_tests.rs`). The ruled Task 14 test needed `cx.executor().allow_parking()` beyond the approved diff (the worker runs). Obligation 8: D2's words stay (pinned by main and Task 16 tests). No other main or Task 0–17 test changed. |
| 18 fix 1 | `fix: reload a clean editor by the session's state, save kept sessions from the close dialog, and never save a cancelled close` (`451d204`); `docs: record Task 18 fix round 1 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `edit` → 217; `view::tabs` → 36; `drafts` → 140; clippy desktop-only and all-targets/all-features clean; fmt clean | complete | I1: a late read decides on the session (already found clean), not the editor's own flag: `EditorHost::reload_read` loads a body that differs, keeping the caret, and resets the baseline; the read's base is adopted only once the editor holds its body. Test `a_late_read_reloads_a_clean_editor_whatever_its_own_flag_says` (recovery off, typing during a held save, deleted after the commit, then another actor's change): RED on HEAD ("the editor holds theirs"), then GREEN. I2 (ruling): the close dialog's Save is enabled for a kept session, un-keeps it without moving focus (`unkeep`, the state part of `resume_kept`), saves, and closes the tab once the draft retires; `save_in_the_close_dialog_saves_a_kept_session_then_closes` RED (Save `Some(false)`), then GREEN. M1 (ruling): Cancel, Keep draft or Discard downgrade `continuing` to Edit (`continue_without_saving`, also on D2's own Keep and Discard); a continued draft that ended is dropped (`follow_continuing`); `cancelling_the_close_dialogs_save_in_its_steps_saves_nothing` RED ("nothing is saved"), then GREEN. M2: D2's dialog's Save also needs `edit_available`, and the dialog says the hand-off is owed while a prepare waits for Resume; no test. M5: P-4 asserts no release before the Discard. M7: `e_with_a_stored_draft_takes_the_restored_draft_route`, which passes on HEAD; RED with Task 14's stand-in put back ("the edit editor did not open"), then GREEN. M3: the unverified-base case now also reloads when only the editor's flag is dirty; no test changed. M3, M4 to Task 19; M6 parked. No main or Task 0–17 test changed. No ticket comment. |
| 19 | `feat: review stale edits field by field before saving` (`5d4c7fb`); `docs: record Task 19 in the ledger and on the ticket` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `review` → 54 (27 new: 4 model, 23 view; 27 earlier tests match the filter); `drafts::privacy_tests` → 1; `edit` → 241; whole binary → 897; clippy desktop-only and all-targets/all-features clean; fmt clean | Done | E-19 applied first (`SaveDocumentAs`, `SaveTicket` in the test-support actor and its child). One test per row of the review's Required outcomes table and the session row "Done or Save during the Review → Disabled" were written first; each answers as the design's table says, so there was no stop. RED: 27 failed against stubs (the review never opened; the N2/M3 tests read "unsaved changes"; the close note lacked "Cancel to review the changes."). GREEN after two corrections of my own tests' mechanics: the actions and the body row moved to the top of the review so a tall ticket review keeps them in view, and an `inert_button` assertion that cannot hold for a Kit button was replaced by its click. 14 mutations, each caught. The model is `model::review` (`Review`: three values per field over the library's `compare`, choices, the body's result); the view is `view/item/edit/review.rs` and `review_view.rs`, inside the tab's `draft-section` (R-38). While the review is open the pane feeds nothing and D2's write schedule is suspended (`DraftRegistry::suspend`/`resume`; an explicit flush, a close's Keep draft or a quit, still writes the text the session held). Use moves the session and D2's draft onto theirs (`adopt_base`), puts the result in exactly (`EditorHost::rebase`, caret kept) and feeds the chosen fields; Cancel resumes. The change banner (R-68) now shows for a shown session too, from the save's `external_change` answer until Use, or from D2's `Changed` classification, with Compare… while editing. Decision on obligation 2 (N2, M3, the post-save read): a clean session on an unverified base takes the reloaded body into its base and stays unverified; a verified one takes the read, its body as the editor holds it; the read after a save follows the same rule (`late::take_clean_read`) once D2 has retired the draft; `reload_read` keeps a body the editor holds exactly (fact 81). Obligation 3 (M4): the dialog stays open and adds "Cancel to review the changes.". `rebase`'s Task 19 allowance is gone. No main or Task 0–18 test changed; `edit_tests.rs` gains only a module declaration. |
| 19 fix 1 | `fix: keep the review reachable and out of kept saves, and leave no empty draft` (`6e41a0a`); `docs: record Task 19 fix round 1 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `review` → 61; `edit` → 248; `view::tabs` → 36; `drafts` → 140; clippy desktop-only and all-targets/all-features clean; fmt clean | complete | BASE `04b073d`. I1: a review with no model takes the newest read of the tab that lands (the one it opened on, or one that superseded it), and Cancel is offered while it reads; `a_reload_while_the_review_reads_does_not_strand_it` RED on HEAD ("Cancel while it reads"; the review never built), `cancel_while_the_review_reads_goes_back_to_editing` RED (no Cancel), then GREEN. I2: `open_review` refuses while a save run is kept; Review… and Compare… are hidden then; Retry, Resume and the refusal's Retry are hidden in Review, and `retry_save`/`resume_save` refuse there; Use clears only an `external_change` answer. M1: a shown session's change classification waits while its own save runs, is kept or D2 settles. `no_review_opens_over_a_kept_pending_checkpoint` RED on HEAD (Compare… offered over our own pending write, with text typed since; another actor cannot commit meanwhile: `recovery_required`), then GREEN; two guard mutations caught. M2 first part (controller ruling): an explicit Keep draft or quit during the review writes mine; named in `suspend`'s comment, no code change. M2 second part: the close dialog says "Cancel to review the changes." whenever the review is open; `closing_the_tab_during_the_review_says_cancel_to_review` RED (no note), then GREEN. M3: choices that change nothing discard the draft at Use (the tab is detached first, `DraftRegistry::detach`, so the session goes on); `choices_that_change_nothing_leave_no_draft_and_close_at_once` RED (the dialog asked), then GREEN. M5: the intro is "This item changed since your edits began. …", pinned in Task 19's own `a_stale_save_opens_the_review_and_leaves_the_draft_file_unchanged` (RED on HEAD). M4: `undo_after_using_the_choices_does_not_go_back_past_the_result` and `using_the_choices_writes_the_draft_file` pass on HEAD; RED by mutation (`rebase` keeping undo history; Use not resuming D2). M6 to the Part C checkpoint. No main or Task 0–18 test changed. No ticket comment. |
| Part C fix 1 | `fix: ask before quit or removal drops undrafted edit text, and apply Part C review minors` (`fa5004a`); `docs: record Part C fix round 1 in the ledger` (this commit) | desktop-test-support `--bin manyhands`: while working, `edit_tests::quit` → 9 (9 new), `is_no_change_of_theirs` (`review`) → 2, the two M2 close tests → 1 each, `restore_is_not_offered_above_a_live_edit_session` → 1, `releases_the_context_once` (`edit`) → 2, `resume_takes_only_its_own_answer` → 1, `view::tabs::tests` → 36; whole binary once at the end → 921: 920 passed, 1 failed, `view::tabs::tests::save_in_the_close_dialog_saves_then_closes` ("One's tab closing did not happen", its 10 s wait), which passes alone and fails the same way in a whole-binary run at BASE `e99628f`'s sources (903 passed, 1 failed) while another session's `mutation_remote` run loaded the machine; then `edit_tests::quit` → 9 again after test (d)'s last assertion; clippy desktop-only and all-targets/all-features clean; fmt clean | complete, with a pre-existing load-sensitive wait | BASE `e99628f`. I1 (controller rulings): `request_quit` first waits for any begin in flight (`ItemPane::begin_in_flight`, under "Saving drafts for recovery" and Quit now), then asks when tabs hold unsaved edit text no live D2 draft holds (`unkept_edit`): "1 tab has unsaved changes that are not kept as a draft" / "N tabs have unsaved changes that are not kept as drafts", Cancel (default; shows the first such tab, focuses the content region) and Quit anyway (`QUIT_ANYWAY`), which goes on to D2's quit unchanged, so D2's own question comes after Quit anyway (the brief's second option). The removal confirmation adds "1 tab has unsaved changes that are not kept as a draft. Removing closes it." / "N tabs … Removing closes them.", read live through `services::unkept::UnkeptEdits`; a confirmed removal closes every tab; a repository that leaves the list, or a removal whose preview found it already gone (never confirmed: `Changed::confirmed`), keeps such a tab (`Workspace::drop_repository`), whose Save the library refuses as `repository_not_registered` and whose close asks. M1: `model::review::base_as_read` takes theirs' normalized value for each base field equal to the raw read's, in `sync_review`, the one place the review is built, which covers `open_restored`'s kept base; the session's and D2's bases are unchanged. M2: a clean session with a stored or beginning draft asks in D2's words (`TITLE`, `message(title)`); Keep draft offered while the last write failed reads `SESSION_KEEP_FAILED`; the tab's unsaved mark is left (E-29 pins it for a tab with a draft); the dialog body is named by its title. M3: `restore_offered` (no session, Viewing) hides the banner's Restore and gates `claim`. M4: `leave_since` re-arms a waiting check with a new listing and the later of the two marks. M5: `TabEdit::prepare_taken`, set by `send_again` to the answered attempt; `prepare_progressed` takes only a later one. RED on BASE for every new test but two guards (the words unit test, a removal with no such tab) and the N1 path (`a_leave_before_a_clean_uses_discard_lands_releases_the_context_once`), which passes there: the discard's own `reload_listing` makes any earlier listing stale. No main test and no Task 0 to 19 test changed (`a_draft_undone_to_the_loaded_text_reads_already_saved` needed no edit); no outcome-table row changed answer. Seven mutations, each caught |
| Part C fix 2 | `fix: keep tabs unkept after a removal's Confirm, and disable Drafts Restore over a live edit` (`849f961`); `docs: record Part C fix round 2 in the ledger` (this commit) | desktop-test-support `--bin manyhands`: while working, `edit_tests::quit` → 10, `edit_tests::release` → 30, `view::tabs::tests` → 36, `drafts` → 142, and the new and changed tests alone; whole binary once at the end → 924 passed, 0 failed (`uptime` at its end: load average 15.47, 15.35, 11.42); clippy desktop-only and all-targets/all-features clean; fmt clean | complete | BASE `13ec873`. A (controller ruling, a D3-test edit): `save_in_the_close_dialog_saves_then_closes` waits for the prepare, then the save, then the close, each in its own `until_drawn` and `WAIT` (new helper `ended_requests`), and its second half for the save, then the close; `save_in_the_close_dialog_saves_a_kept_session_then_closes` likewise for the save, then the close; no assertion changed, and both passed in the whole-binary run. N3 (ruled words): `SESSION_DRAFT` "This tab has a draft, and no unsaved changes. Keep it to continue later, or discard it." under `TITLE`, through `Ask::session_draft`; round 1's `closing_a_clean_session_with_a_stored_draft_says_it_has_a_draft` takes the new string, as ruled. N1: `services::edit_tabs::EditTabs` (round 1's `UnkeptEdits`, now keyed by `TabKey`) records at Confirm, once the request is queued, the tabs of the root the line counted; `Workspace::close_repository`, which the landed removal calls, keeps open a tab unkept now that was not counted (`Keeping::Uncounted`). Test first: `a_tab_unkept_after_confirm_stays_when_the_removal_lands` (RED: tabs `[]`). D (R-35): the Drafts page's Restore is disabled, with "Open in its tab to continue editing" (`CONTINUE_IN_TAB`, the tab's banner giving no reason of its own), and `restore` returns, while a tab of this window holds an edit session of the item or is getting one ready (`EditTabs::editing`). Test first: `the_drafts_pages_restore_is_disabled_over_a_live_edit_session` (RED: "nothing is claimed"). N2, simple enough: `Releases::forget_root`, called when a removal lands and on a drop, makes `made_here` false for the root until a prepare of this process makes a context of it again, and drops its waiting checks. Test first: `a_tab_kept_after_its_repository_left_sends_no_release` (RED: a release was sent). Three mutations, each caught; D's disabled button and its `restore` gate are each enough alone, so only removing both fails its test. No main test changed; no R-76 or review-table row changed answer. The design document's uncommitted Known Limits line in this worktree is not this round's and was not staged |
| 20 | `feat: edit the named fields in the details sheet` (`206177a`); `docs: record Task 20 in the ledger and on the ticket` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `sheet` → 29 (12 new); `metadata` → 4; `drafts::privacy_tests` → 1; whole binary → 936 passed, 0 failed (594 s; `uptime` at its end: load average 17.07, 15.88, 14.21); clippy desktop-only and all-targets/all-features clean; fmt clean | Done, with concerns | BASE `118b60d`. While the tab is Editing or Saving, a document's Title and a ticket's Title, Type, Status, Project and Team are Kit inputs (`view/item/sheet/fields.rs`; pure rules in `model/edit/fields.rs`) that write through `change_fields` (its Task 20 allowance removed), so they mark the tab dirty and feed D2 as typing does; a field put back leaves the session clean and `secondary-s` sends nothing (contract 4). Preparing, Loading, the review (the session's values) and a kept session (the saved copy) keep the read-only rows (R-74). A chip focuses its control; Parent, Waiting on and a document's Status, Date and Author stay rows (decision 14). Suggestions (R-57): the values on the workspace's ticket list when it is the tab's repository's, plus the item's own, handed to the tab by `sync_sheet` each frame the sheet is open (the sheet layer is built inside `Workspace::render`, so the pane cannot read the workspace there); `TicketRow` gains `project` and `team`. An emptied Project or Team is `None` (R-27): the committed blob has neither key. An empty Title, Type or Status shows "A ticket needs a type before it can be saved." (etc.) under a red-bordered control, and `missing_field` blocks every save: `save_offered`, `start_save` (the toolbar note says why), the close dialog (its note says why), Done's prompt, D2's restored section and dialog (note `draft-save-note`), the header's tooltip. `secondary-s` in the sheet's new `ItemSheet` context saves the tab, and does nothing in the review. The sheet's body is `draft-section` while editing (R-38), with "Changes here become part of your draft and are saved with the ticket." and "Saved with the ticket (Ctrl S)". `closed` is saved as text; the ticket stays open. A dirty session is never reloaded, so a late read never replaces a control; Use's choices reach the controls. RED: 11 of 12 failed against stubs (`a_documents_status_date_and_author_are_not_editable` a guard). GREEN after one correction of my own test (the tab strip's mark follows the attached draft, E-29). 7 mutations, each caught. No main or Task 0–19 test changed; `edit_tests.rs` gains only a module declaration |
| 20 fix 1 | `fix: check a deferred save's fields again, and name a missing field last` (`316e716`); `docs: record Task 20 fix round 1 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `sheet` → 30 (13 of Task 20's); `view::item::edit` → 182; clippy desktop-only and all-targets/all-features clean; fmt clean | complete | BASE `ae1729d`. A (review Minor 1): `send_save` asks `save_blocked`, the one call every offer and `start_save` consult (it wraps `missing_field`; Task 21's relationship check joins it), after the deferral checks, so a save sent once the draft begins (`draft_begun`) or D2 settles (`follow_registry`) goes with checked fields; a blocked one ends the run (`end_run`: phase Editing, the tab dirty) and notes the message in the toolbar. Test first: `a_save_deferred_until_the_draft_begins_checks_the_required_fields_again` types a first letter, `secondary-s` and an emptied Title as single keystrokes with nothing run between (the draft still `Beginning`, the phase Saving), then runs: RED (a save was sent), then GREEN. B (Minor 2): `close_note` gives the review, then what a pressed Save came to (`closing_note`), then the empty required field, and none for a new item's draft, whose Save says "Available in a later release"; checked by reading, no test. C (Minor 3c): the R-27 test also checks `unknown_metadata` has no `team`. D (Minor 7): `ids()` has no Title suggestion ID; a chosen suggestion goes in with `replace_all`, so the control can undo it (the R-57 test now undoes it: RED with `set_value`); `take_suggestion` still calls `field_typed` itself, because a frame can be drawn before the input's change event is delivered (the test's click does), and `sync_controls` would then set the control back; the event's own call then changes nothing. Not changed: `registry_sent`'s fallback to `send_direct`, when D2's session ended while asked, sends without a second check (the library refuses an empty field). No main or Task 0–19 test changed |
| 21 | `feat: edit a ticket's dependencies and parent in the details sheet` (`54bddc4`); `docs: record Task 21 in the ledger and on the ticket` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `relations_edit` → 18 (13 view, 5 model); `sheet` → 30; whole binary → 955 passed, 0 failed (696 s; `uptime` at its end: load average 21.77, 21.74, 17.34); clippy desktop-only and all-targets/all-features clean; fmt clean | Done, with concerns | BASE `3cede81`. While the tab is Editing or Saving, the sheet's Waiting on is a chip per dependency in ID order (R-72: lifecycle icon, short code and title, state, ID, Remove) with Add…, and Parent its chip with Change… and Clear (`view/item/sheet/ticket.rs`; state, check and names in `view/item/relations_edit.rs`; the picker in `relations_edit/picker.rs`; pure rules and the ruling's words in `model/relations_edit.rs`). A change goes through `change_fields`, so it marks the tab dirty and feeds D2 as typing does. The picker reads `list_tickets` of the tab's own repository (never the workspace's list), filters titles ignoring case, matches short codes exactly through `find_tickets_by_slug`, leaves out the ticket and what the field holds, takes Enter, Tab and Space, and closes on Escape (`ClosePicker` in `RelationPicker`) without closing the sheet. Every change runs `check_ticket_relationships` (`ReadKind::RelationshipCheck`, its allowance removed) over the whole set under the tab's stamp; a proposal equal to the base's is not checked (its save sends `Unchanged`, which the library does not check). A rejection is explained under the field whose change it answered ("This makes a cycle: {names}.", names by short code and title, else ID) and `save_blocked` holds every save with "A ticket cannot be saved with a relationship cycle."; a check in flight holds nothing. A save refused `relationship_cycle` is explained the same way from `rejected` (`Operations::rejected`, new), under Waiting on when the sent `deps` differ from the base; the session stays dirty. "not found, kept" and "The index may be missing tickets. Save checks again." as ruled. Dependents and Children stay D1's rows; Preparing, Loading and a kept session keep D1's rows, the review the session's chips read only (R-74). RED: 17 of 18 against stubs; with the final tests, 12 of 13 view tests (`dependents_and_children_stay_read_only` a guard). 10 mutations: the gate, ID order, Unchanged, in-flight, Escape, R-74 and the library answer caught; the stamp alone is masked by the dropped task (both removed: caught). No main or Task 0–20 test changed; `edit_tests.rs` gains one module declaration |
| 21 fix 1 | `fix: keep a refused save's unresolved notes, and close the picker with the sheet` (`07e8b7d`); `docs: record Task 21 fix round 1 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `relations_edit` → 19 (one new model test); `sheet` → 30; `edit_tests::review` (the review's tests) → 32; clippy desktop-only and all-targets/all-features clean; fmt clean; whole binary not rerun, as ruled | complete | BASE `670e64e`. A (review I1): `Answer::of_save` takes the last check's answer of the same proposal and keeps its unresolved IDs and index note (`refused` marks a save's rejection); `relations_refused` no longer drops a check in flight, and `check_landed` merges one landing after a refusal (`Answer::after`: the refusal's rejection and field stay, the check's unresolved IDs are taken). Test first: `a_library_relationship_cycle_answer_is_explained_the_same_way` gains an unresolved dependency, still "not found, kept" after the refusal (RED: "Resolved"), and a save pressed while its check runs (RED with the old drop: the held check was gone). B (Minor 3): Use calls `relations_rebased`, which forgets `asked`, so the proposal is checked against the moved base; the R-74 test counts one check at Use (RED: none). C (Minor 4): `relation_candidate` takes the item and D1's rows first, `known` only for a ticket they do not have; `relations_ended` (from `end_session`) clears `known`, the answer and the picker; the picker test renames a ticket and reads again (RED: the old name). D (Minor 5): the tab drops an open picker once no sheet is active, and `open_details` forgets it; the picker test closes the sheet from outside the picker and reopens it (RED: the picker came back). Mutations: no carry, the old drop, Use keeping `asked`, `known` first, and both picker drops removed are caught; `after`'s call removed is masked in the view (the late check sees the same cycle; the model test pins `after`), and each picker drop alone is masked by the other. No main or Task 0–20 test changed; only my Task 21 tests changed |
| 22 | `feat: create new items from their drafts` (`53d8b93`); `docs: record Task 22 in the ledger and on the ticket` (this commit) | `--lib drafts::` → 55 (4 new); `--test drafts` → 48; desktop-test-support `--bin manyhands`, once each at the end: `drafts` → 156; `draft_comparison` → 3; `view::item` → 295; whole binary → 970 passed, 0 failed (711 s; `uptime` at its end: load average 30.59, 31.33, 23.15); then, after one added gate test, `services::drafts::create` → 15; clippy desktop-only and all-targets/all-features clean (run again after the added test); fmt clean; `check --all-features` clean; CLI isolation script clean | Done, with concerns | BASE `948611f`. Library (`src/drafts/compare.rs`): `TargetState::NotCreated` and `is_new_item_base` (observation and source both `None`, R-20); `classify` answers `NotCreated` for a missing item of a new item's item draft only, and `restore_policy` allows it; an existing item stays `AlreadySaved` or `Unverified`. The desktop's `model::edit::new_item_base` is removed. Registry (`services/drafts/create.rs`): `begin_new` (store unavailable answers at once, R-46), `new_item` (R-20's one entry point for Task 23: `begin_new`, then `context prepare` with Create; `NewItem { draft, prepare }`; `RECOVERY_NEEDED` "Crash recovery is off, so a new item cannot be created."; all three carry Task 23 allowances), `create` (the save path with a `Verb`: flushed first, one at a time, `DocumentCreate`/`TicketCreate` with no initials, R-21; pure rules in `model/drafts/create.rs`). `for_drafts` routes the creates' answers, and a Create prepare's `ok` worktree (`ForDrafts::Prepared`). Completion (R-20b): `base_after_create` (the read, or the fields sent with source `""`), then retire or rebase as a save's. `occupied_path`: `show_path` in the item's own context, from that worktree; this draft's own item → `unmarked`, rebased, and the tab (`follow_unmarked`) reviews it; otherwise the library's refusal shows and the draft is kept. A create whose effect is unknown (`!Intent::refused_answer`: internal_error, busy, cancelled) keeps its record, so the worker's `retry` of the same request completes the draft; the next create drops it. `CreatedContexts` and condition 3 count a committed or in-flight create as a save. Tab (`view/item/edit/create.rs`): the route is the classification against the tab's read (R-42): `NotCreated` prepares with Create and opens on the draft with no item read (`open_not_created`; the page draws the edit editor with no item); otherwise Edit, a marked draft unmarked and reviewed (`open_restored`); a Create answering `occupied_path` re-routes to Edit. `saves_directly` replaces `base_verified`; a never-created Save calls `create` through `start_save`/`send_save` and `save_blocked`. D2's banner offers a not-created stored draft with no item read; its section, header and close dialog offer Continue editing and Save (no later-release notes; main's dialog code unchanged); "The item is not created yet" for the Drafts page. P-3: the four tests run over `Shell::edit_new_ticket` (`begin_new`, attached, Continue editing with Create), names and results unchanged; `as_new_item` and `edit_session_mut` removed. `ReadKind::NewId` now says Task 23; the create intents' and `JobKind`s' allowances are gone. RED: library 1 of 4 against a stub (3 pin rules that already hold); desktop 13 of 14 against stubs (the restore test then gained a header check). 14 mutations, each caught. 15 desktop tests (the brief's 14 and one gate test) and 4 library tests are new; no main or Task 0–21 test changed but the four P-3 tests |
| 22 fix 1 | `fix: test a not-created draft's restored Save and a create sent again` (`0aea947`); `docs: record Task 22 fix round 1 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `services::drafts::create` → 19 (4 new); `drafts` → 161; `view::item` → 295 (369 s; `uptime` at its end: load average 11.76, 14.96, 13.90); clippy desktop-only and all-targets/all-features clean; fmt clean; whole binary and library tests not rerun, as ruled | complete | BASE `4af076a`. A (review M1): `save_of_a_restored_not_created_draft_creates_it` (D2's section's Save: one Create prepare, one create `ok`, the file committed on the item's branch, the draft retired) and `the_close_dialogs_save_of_a_not_created_draft_creates_it_and_closes` (Save offered with no note; created once; the tab and the dialog close). Both passed at once; dropping `open_not_created`'s `Then::Save` branch fails both, and the close dialog's `save` set back to `None` fails the second. B (M2): after a lost answer (`BeforeRequestSettlement`), `create` again is a new request answered `already_applied`: one document, no second commit, the draft retired (`a_create_sent_again_after_a_lost_answer_completes_the_draft`). With newer text it is answered `occupied_path`: no second commit, and the session's base unmarked for its tab's review (`a_changed_create_sent_again_after_a_lost_answer_becomes_unverified`). Both passed at once; the own-item check made to answer false fails the second. C: M4 `SaveRecord.verb: Verb` in place of `create: bool`; `key_of` gone, the key is `ContextKey::of(&prepare_create(..))`; `holds` uses the registry's `item_kind`; `prepare_create`'s `ItemKind`→`AuthoringKind` mapping stays, as no shared helper from a draft's kind to `AuthoringKind` exists. M5 `check_occupied`'s no-base branch notifies. M7 the kept record's comment states the ruled relaxation of one save at a time. No main or Task 0–21 test changed; only my Task 22 tests were added |
| 23 | `feat: open New document… and New ticket… dialogs` (`de5c4de`); `docs: record Task 23 in the ledger and on the ticket` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `create` → 42; `view::documents` → 4; `view::tickets` → 9; `keys` → 28; `keyboard_tests` → 23; whole binary → 985 passed, 0 failed (698 s; `uptime` at its end: load average 19.48, 18.73, 12.77); clippy desktop-only and all-targets/all-features clean; fmt clean | Done, with concerns | BASE `37b17b0`. The dialogs (`view/create.rs`, `create/document.rs`, `create/form.rs`): New document… (before the disabled New folder, which is unchanged) and New ticket… (above the ticket filters), and `secondary-n` (`NewItem`, bound in `NAVIGATION_CONTEXT` only; inert in front of a dialog or the sheet) open on the selected repository; focus starts in the Title; Escape closes and the Kit gives focus back. Fields: Title; a ticket's Type (`task`); a document's Folder (`docs`) and File name. Create waits on Task 20's `missing` words. The path is checked locally (`model/create.rs` `document_path`: `.md`, then a plain path under `docs`) and then with `observe_path` under `ViewScope::Create`, the last check's task dropped. Create, in R-20's order: `ReadKind::NewId` (`new_item_id`), then `DraftRegistry::new_item` with the model's new constructors (`model/drafts/create.rs`: `ticket_fields`, `document_fields`, `ticket_path`, `new_item_draft`; the branch is the repository list's `primary_branch`, never hard-coded; the harness now calls them), then `Created` → `Workspace::open_created`: the dialog closes, the tab opens, and `ItemPane::edit_created` attaches the draft, sets `continuing.create` and enters `Preparing(prepare)` (calling `prepare_progressed` once for an early answer), so Task 22's `open_not_created` opens the editor. No store: R-46's words in the dialog, which stays open; nothing provisioned. Escape and Create wait while a create runs. The new tab's header, strip title and sheet show the draft's fields (`shown_new_item` via `new_item_shown`) while the never-created session lives or its Create prepare runs; D1's not-found line and D2's restored section are not shown meanwhile. At Save: `occupied_path` for another file says "Another file is already at {path}. The draft is kept." once D2's own-item read is done (nothing while it runs); `initials_required` says "This repository needs initials before it can create tickets." with Open Settings (`InitialsInSettings` → `settings_for_initials` → next frame `identity_in_settings`). Allowances removed: `keys::NewItem`, `ReadKind::NewId`, and Task 22's four; `NewItem` and `RECOVERY_NEEDED` re-exported from `services::drafts`. Tests (`view/create_tests.rs`): the brief's 8 and 2 more (R-46 in the dialog; a prepare answered before its tab opens). RED: the entry point disabled, 0 of 9. 21 mutations: 20 caught (two once a test was added for them: the wait for D2's path read, and the hook's early-answer call); the path stamp alone survives (masked by the dropped task, as Task 21's). No main or Task 0–22 test changed |
| 23 fix 1 | `fix: keep a new item's sheet off a missing file, and its header until read` (`ac55ba1`); `docs: record Task 23 fix round 1 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `create` → 43; `view::item` → 295 (433 s; `uptime` at its end: load average 15.93, 25.94, 23.38); `tabs` → 57; clippy desktop-only and all-targets/all-features clean; fmt clean; whole binary and library tests not rerun, as ruled | complete | BASE `d6718ae`. A (review I1): the sheet's `file_rows` gives the problems and the Inspect link only for a read item, none for the stand-in; while a new item opens (`opening_new_item`) the sheet says `NOT_CREATED_YET` "Not created yet. These can be changed once its private copy is ready." with the note `READ_ONLY_UNTIL_READY` "Read only until then"; editing, the usual words. Test first in `the_new_tab_opens_editing_with_its_header_from_the_draft` (RED: "Viewing the shared copy…"). B (M1): the header borrows the read and builds only the stand-in; the strip uses `new_item_title`; the banner filter `shows_new_item`. C (M2): `EditSession.began_new`; the stand-in and the not-found filter stay while that session lives and the tab has no item read, so the read that finds the item replaces them (the context kind is now the base's); staged in `save_creates…` by holding the tab's `Item` reads after the create (RED: no title). D (M4): any field's change checks a failed path again (`path_failed`); a folder at the path (`invalid_path`), then removed, in the path test. E (M6): the stamp comment says the dropped task stops the answer and the stamp is a second guard no test reaches. F (M7): the Kit can close only the topmost dialog, so the dialog closes itself (`finish`) only while focus is in it (its own tracked handle); otherwise it waits, its item made, until Escape or Cancel; `open_created` no longer closes a dialog and focuses the content only with none open; new test `a_dialog_above_the_create_dialog_stays_open_when_it_creates` (RED: the dialog above closed). G (M8, first point): a File name with nothing before `.md` is an empty one (RED: `.md` accepted). `Render` moved to `create/form.rs` (`create.rs` 454 lines). Seven mutations caught. No main or Task 0–22 test changed; only Task 23's tests |
| Part D fix 1 | `fix: release no context a create found applied, and keep a new item's stand-in truthful` (`6aa05ed`); `docs: record Part D fix round 1 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `created_contexts` → 4; `create` → 49; `drafts` → 164; `view::item` → 295 (393 s); whole binary → 992 passed, 0 failed (693 s; `uptime` at its end: load average 16.41, 17.44, 16.27); clippy desktop-only and all-targets/all-features clean; fmt clean; no library tests | complete, with concerns | BASE `afd93d4`. A (M1): `CreatedContexts::finished` also takes out the key of a save or create answered `already_applied` (an earlier attempt committed; its release would be refused). Tests first: `a_write_committed_or_already_applied_takes_the_context_out` (unit; RED "already applied takes it out") and `done_after_a_create_found_already_applied_releases_nothing` (a lost create answer, Save again `already_applied`, Done: no `ContextRelease`; RED "the context holds the item's commit", and "nothing is released" with that check off). B (M2, M3, M7): one tab-owned flag, `TabEdit.stand_in: StandIn` (`Unset`, `Session`, `Ended(base, fields)` boxed, `Found`), replacing the session's `began_new`: a session opened on a new item's base sets `Session`; `end_session` keeps what a created-but-unread session ended with (`Ended`; a never-created one goes back to `Unset`, so a stored draft stays D2's); a read that finds the item sets `Found` for good. The pill's tooltip and the sheet's Facts give no provenance, and the Facts no closure sentence, without an item read (`file_rows`' test). Tests first: the header test gains no-`provenance`/no-`sheet-closure` assertions (RED "no provenance"); new `a_new_item_read_then_missing_shows_not_found` (RED `item-failure` None) and `done_then_save_creates_once_and_keeps_the_header_until_read` (one create, Done goes on, the title kept until the held read lands; RED title None). C (M5): the registry keeps `path_unread`, sessions whose `occupied_path` read could not tell what is at the path; the tab then shows the ordinary refusal with the library's message and code, no path words, the draft kept and Save offered. Ruling read: `invalid_path` and `path_not_found` in the item's own context count as telling (its own item is not there), because Task 23's `an_occupied_path_at_save_keeps_the_draft` reads `invalid_path` there (the index holds no active context for a fresh new item) and pins R-22's words; busy and any other failure, and anything in the root-copy fallback, do not tell. Test first: `an_occupied_path_that_cannot_be_read_names_no_other_file` (RED "no other file is named"). D (M6): `session_discarded` with no session, while a Create prepare's draft is discarded (`opening_discarded`), drops `continuing`; once the discard lands the tab hands itself to release on leave (Preparing: the prepare's `created: true` answer releases) and closes. Test first: `a_drafts_page_discard_while_the_create_prepares_closes_the_tab_and_releases` (RED "the tab closing did not happen"). E (M8, first point): `new_ticket_creates_with_its_fields_and_dependency` extended as allowed (a Project typed and emptied, Status `closed`: `project` absent, status text `closed`, closure open); it passed at once, and two mutations (an emptied Project kept as `Some("")`, the Status not passed) were caught. Six mutations in all, each caught. No other pre-existing test changed; the pill's tooltip, the M6 Loading branch and `ended`'s return to `Unset` have no test of their own |
| Part D fix 2 | `fix: show a created item as created, in its private copy, until read` (`4d7ef17`); `docs: record Part D fix round 2 in the ledger` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `create` → 51; `view::item` → 295 (378 s); clippy desktop-only and all-targets/all-features clean; fmt clean; no whole binary and no library tests, as ruled | complete | BASE `269ee8d`. A (N1 and the kept-session observation): `StandIn::Ended` holds the session's base alone, so it stands in with `base.fields`, the item as created or last saved, never the session's unsaved fields; a kept session of a created item stands in with `base.fields` too (the tab views the saved copy); a shown session keeps its own fields, and a never-created item's stand-in is unchanged. Test first: `a_created_item_not_yet_read_stands_in_as_created_once_kept_or_discarded` (create, the Title changed in the sheet, the `ReadKind::Item` reads held; Done keeps the session, since its draft is stored, then the kept banner's Discard… ends it; the header and the strip show "Plans" in both; RED "kept: …" with `Changed`, and "discarded: …" with the kept checks off). Done asks Discard only when the draft is not written, which needs a failed store, so the discard is the kept banner's: it reaches both cases in one fixture. B (N2): done in how the stand-in is built: `shown_new_item` takes `created` and gives context kind Active for a created item whose session no longer shows (`with_new_item` passes it for `Ended` and the kept case), so `pill.rs` and `sheet.rs` are unchanged; still no provenance or closure. Test first: `a_created_item_ended_before_its_read_is_viewed_as_the_private_copy` (D2's `DraftItem` read held while another actor retitles the item, so `base_after_create` keeps the Primary base; Done with the item reads held; the pill says "Viewing your private copy" and the sheet `VIEWING_PRIVATE`, no provenance; RED "Viewing the shared copy"). No pre-existing test changed |
| 24 | `feat: move a document from its sheet, and make New folder work` (`d250d2f`); `docs: record Task 24 in the ledger and on the ticket` (this commit) | desktop-test-support `--bin manyhands`, once each at the end: `move_document` → 9; `new_folder` → 4; `view::documents` → 4; `sheet` → 31; whole binary → 1006 passed, 0 failed (720 s; `uptime` at its end: load average 16.33, 17.37, 12.95); clippy desktop-only and all-targets/all-features clean; fmt clean; no library tests | Done, with concerns | BASE `2c8e09f`. Move… (`view/item/sheet/document.rs`): beside the Folder box, offered only while `move_blocked` is `None` (R-23): an item tab with its item read, `Viewing`, no session live or kept, no claim in flight, no draft attached or stored (`draft_kept`), no save of it in the registry (`saving_item`), else "Move is available while viewing, with no draft or save in progress."; then "Move waits while a private copy is removed." while `Releases::pending`. Disabled, it says why (tooltip, description, `sheet-move-note`). The item-as-read condition is added because a created item ended by Done before its read (`StandIn::Ended`) meets none of R-23's four. The dialog (`view/move_document.rs` 423 lines, `move_document/steps.rs` 253): Folder and File name, the document's own; the destination checked as typed with Task 23's `document_path` and `PathCheck` (now shared, with `observed` and `note`; the create dialog calls them) and not the current path ("The document is already at this path.", not ruled), then `observe_path` in the item's active context or on primary. Move: R-23 asked again, `show_item` by ID (at the destination already: moved, and the root marked changed; elsewhere than read: stale), `observe_path` again (a file: "Another file is already at {path}." under the File name, Move waiting), then `Intent::DocumentMove` with both observations on the user lane. Every attempt of the request is followed: a committed checkpoint, `ok` or `already_applied` → `Moved`, the dialog closes and the tab and tree read again by R-B2; `occupied_path` → the collision; `external_change` → read by ID, at the destination moved, else "This document changed since it was read. Close this and try again."; else the library's message and code. New folder… (`view/new_folder.rs` 368 lines, E-10): `NEW_FOLDER` "New folder…" enabled, `NEW_FOLDER_UNAVAILABLE` gone; the parent is `selected_folder` (folder nodes keep their path) else `docs`; "A new folder stays on this computer until a document is saved in it.", "Folder name", the full path as typed; "This is not a valid folder name." for empty, `/`, `\` or a leading `.`; `Intent::FolderCreate`, no refresh (R-24); `ok` closes, `already_applied` "Folder exists", else message and code. Allowances removed: both intents, their `JobKind`s, `Releases::pending`, `ContextKey::released_by`. Tests: the brief's 10 (E-10 replaces `folder_creation_is_visibly_unavailable`, with that file's module doc line and a `TestWindowExt` import) and 3 more (`external_change` at the destination; a committed move owing its index; Move again after a lost answer). The lost answer is `BeforeRequestSettlement`, as Task 22's `lost_create`; the worker's `retry` of the same request answers `ok` with no second commit (the binding works around `01M4PB80T7` for a committed source; with the source uncommitted and the destination committed over it answered `already_applied`), so that test accepts `ok`/`already_applied`, and the ruled `external_change` branch is reached by the extra test (another actor's move to the same destination with an uncommitted change). RED: 0 of 10 against stubs; the committed-move test with its arm disabled. 10 mutations: 9 caught; `has_session` ignored survives (redundant with `Viewing` and `draft_kept` in every reachable state). No other pre-existing test changed |

## Rulings during implementation

- Owner decision, 2026-10-26: add E-30, like E-23. The five (c) tests that
  read `item-id`, `item-kind` or `item-lifecycle` from the item view open the
  details sheet first (All details), read the same IDs and values there, and
  close it before going on. The sheet's Facts keep D1's labels. Applied in
  Task 6. The design's Named Edits table and the plan's Task 6 and File Map
  record it.
- Ruling: `manyhands-cli help` is not a subcommand (exit 2). The ledger keeps
  both its output and the `--help` output, and Tasks 1 and 2 compare both
  (R-5). — The plan's purpose is proving no new CLI command. — If wrong: none.
- Ruling: the full suite reached its two-hour budget inside
  `ticket_closure`, the last test target; that target and the doc tests ran
  separately to complete the baseline, as Cycle 10's ledger did. Task 27
  does the same if its run reaches the budget. — Every other target had
  finished with no failure. — If wrong: none; every target ran.
- Ruling P-1: Task 3's `normalized` implements R-11 in full: the body through
  `load`, and a ticket's `deps` sorted by ID (R-72). Task 3 adds
  `normalized_sorts_a_tickets_deps_by_id`. Task 21 does not re-implement it.
  — Task 14 consumes the sort before Task 21. — If wrong: the sort lands
  earlier; no behavior change.
- Ruling P-2: Task 1 may edit `tests/mutation_contract.rs` for E-17
  (`DATA_ROOTS`) only. — E-17 and the File Map name it. — If wrong: one
  reviewable test line.
- Ruling P-3: the never-created session-outcome rows (Task 16 "Done on a
  never-created item", Task 17 "Drafts page discards a never-created item's
  draft that a tab holds") are written in Task 22, with the same names and
  required results, with their code paths. — `NotCreated`, `begin_new` and a
  no-item tab first exist in Task 22. — If wrong: two rows are proven at the
  Part D checkpoint instead of Part C.
- Ruling P-4: `discarding_a_restored_draft_releases_the_context_continue_editing_made`
  is written in Task 18. — The Continue-editing route (R-42) is Task 18's. —
  If wrong: one test moves one task later.
- Ruling P-6: Task 9's app actions go in `src/desktop/keys.rs`; its
  "EditHost > Editor" bindings go in `view/editor/keys.rs::bind`. — Where
  each kind already lives. — If wrong: a move between two files.
- Ruling P-7: each new intent's or ReadKind's dead-code allowance names the
  task that first submits it in production code. — The Global Constraints'
  convention. — If wrong: an allowance names the wrong task.
- Note P-8: `view/item/edit.rs` grows across Tasks 14 to 17. An implementer
  that would pass about 500 lines reports a proposed split along the
  design's Phases.
- Ruling (Task 0 review): Task 22 does not append `NotCreated` to
  `targets()` in `model/drafts/rows_tests.rs`. — No named edit covers it,
  and the list stays a valid, if incomplete, set of cases. — If wrong: one
  test's case list stays incomplete.
- Owner decision, 2026-10-26 (from Task 1): fact 20 was wrong. A
  `context prepare` that answers `discovery_pending` leaves its row at
  `WorktreeObserved`, which `operation resume` does not take up, so the
  library offers the same request again (`request.retry`). D3 keeps the
  library as it is: the desktop's Resume during Loading sends the same
  request again, whose `ok, created: true` enters `CreatedContexts`. The
  design records it as R-77 and amends fact 20, the `context prepare`
  answer, Phase 3 (Loading), R-13, R-75 and Known Limits. The plan's
  Task 14 test notes it.
- Owner decision, 2026-10-26 (from Task 1): accept Task 1's one-word
  change in `src/repository/mutation/bind/ticket.rs` (`refuse_closed` made
  `pub(super)`), although that file is not in Task 1's list. It changes no
  behavior, and reusing the check is better than copying it.
- Note for Task 2: the `context release` row "Refresh fails after both
  removals" requires `operation resume` to complete the hand-off. The
  release row's step must be one that resume takes up (`is_authoritative`
  in `recovery.rs`, Task 2's file), or the row fails the way fact 20 did.
- Ruling P-9 (controller, Task 2): Task 2 may change the File Map's Task 2
  row (`src/repository.rs`, `src/repository/{context_release,cleanup,recovery}.rs`,
  `bridges/abandon.rs`, `read/{dto,status,status_tests}.rs`, the three
  operation schemas, `tests/read_contract.rs`), plus what the release
  binding needs from Task 1's row (`src/repository/mutation/{command,mod,dto}.rs`,
  `bind/{mod,context}.rs`, `tests/mutation_bridges/{context,conformance}.rs`,
  `tests/support/{conformance,mutation_golden}.rs`,
  `tests/fixtures/mutation_v1/context_release.json`), plus the new
  `src/repository/context_release_tests.rs`. Task 2 also completes the tail
  of Task 1's last `prepare_` test (the release of the stranded branch).
  Pre-existing tests change only as E-15, E-16, E-20, E-21 and E-22. — The
  release binding sits beside Task 1's prepare binding, in the files Task 1
  made for it. — If wrong: a file is named in the wrong Part 0 task.
  (Controller's ruling; the Task 2 implementer entered it.)
- Owner decision, 2026-10-26 (from Task 2): new named edit **E-31**.
  Task 1's `context_mutation.schema.json` added a `kind` enum that
  `contract_enumerations()` tied to no Rust list, so
  `read_contract published_enumerations_list_exactly_the_registered_values`
  has failed since `eb6e51f` (54 `enum` keywords at `4d3cb3b`, 55 after).
  Task 1's Verify lines did not include that test. E-31 adds the row
  tying `kind` to `ItemDtoKind::ALL` and reorders the schema's enum to
  `["document", "ticket"]`, the order `item.schema.json` uses. Applied in
  Task 2. The design's Named Edits and the plan list it.
- Ruling (Task 2 review, concerns 3 and 4): design steps 4 and 5 discard
  only a new row. A re-entered row with no step that refuses at step 4,
  and a row whose prune began and failed (`WorktreeRelease::Incomplete`,
  answering `recovery_required`), stay pending. — The design discards
  "because nothing was touched", and the journal cannot prove that of a
  re-entered row or a started prune; keeping such a row pending is the
  repository's discard rule (`CallEffects`). Every table row that says
  "discarded" is a first call, so no table answer changes. — If wrong: a
  user whose release was interrupted before step 5, and then refused,
  abandons the row before the next Edit.
- Ruling (Task 2 review, parked): `require_cleanup_clean` reports I/O
  failures (an unreadable index, a failed status read) as
  `worktree_not_clean` (`cleanup.rs:167`). It is the promotion runner's
  shared helper, and changing its mapping is beyond Task 2. Carried to the
  whole-branch review. — If wrong: a rare I/O failure tells the user to
  clean files they do not have.
- Owner decision, 2026-10-26 (from the Part 0 review, Minor 1): **R-78**.
  Neither context command records a checkpoint, so a request whose answer
  was lost runs again as new: a release sent again after a later Edit
  releases that newer clean context, and a prepare sent again after a
  release makes the context again. Cycle decision 11 requires the design to
  state such an exception; it now does (Known Limits, R-78), with no
  library change. Task 17 gains two tests: the desktop never retries a
  release once the item is prepared again, and never sends a prepare again
  once its context is released. In the same amendment the owner approved
  the correction of design step 6 (Minor 4): a moved tip with a commit of
  its own is repaired by hand.
- Ruling (Part 0 review, Minor 2): a step-6 refusal on a new row that
  removed no worktree (the branch-only path) discards the row, as step 4
  does, and the design's step 6 says so. Fixed in the Part 0 checkpoint,
  in Task 2's file, with a test. — Nothing was touched, which is the
  design's own reason for discarding; a pending row would refuse every
  local operation on the root until abandoned. — If wrong: a refused
  branch-only release leaves no row to inspect.
- Ruling (Part 0 review, Minor 3, parked): abandon's `writes` arm for a
  release stays `Writes::Nothing`, so its preview names no item. The
  design names that arm, and the row's target still names the item.
  Carried to the whole-branch review. — If wrong: the preview of a stuck
  release does not say whose branch is left.
- Owner decision, 2026-10-26 (from Task 3's review): **R-79**, exact
  re-entry. The review found that the pinned `normalize_math_fences` is
  not idempotent (design fact 81, verified by the controller by hand). As
  Task 3 built it, the read-only guard's `set_text` revert could loop
  forever on such a body and freeze the app. The design also used
  `set_text` for the review's result, and a restored draft would load
  again. The load stays one pass (contract 2). Text an editor already
  holds goes back exactly: the guard reverts with a whole-text
  `replace_range`, and `set_exact` (Task 3) serves Tasks 18 and 19. The
  design records fact 81, R-79, the one-pass note and a Known Limit. The
  plan adds the tests to Tasks 3, 18 and 19, and renames
  `load_is_idempotent` to `load_is_idempotent_over_the_fixtures`. No
  follow-up ticket (owner).
- Ruling (Task 3 review, minors): the fix round fixes Minors 1–4 and 6
  (a round-trip assertion, the `beyond` bounds check, the R-66 inline and
  word-move cases, the cursor before typing, the undo-stop comment).
  Minor 5 (`open_editor`'s allowance) and Minor 7 (Control-click's
  `MathMenu` must not open a math menu from a read editor) are carried to
  Task 7. — Each is small and in Task 3's own files. — If wrong: a few
  more lines in Task 3.
- Owner decision, 2026-10-27 (from Task 5a): D1's repository selection
  panel stays drawn below an item tab, as Task 5a built it. The item-view
  handoff has the white page fill the pane to its bottom. The panel takes
  that height, so this is recorded as a Design Deviations row. — Workspace
  rework is excluded, and D1's `opening_a_tab_does_not_change_the_selection`
  reads `selection-name` with a tab open. The other way, hiding the panel
  under a tab, needed a new named edit.
- Owner decision, 2026-10-27 (revised, from Task 5a's review): the review
  measured the cost of a fixed panel. With it, the item's scroll region
  was about 58 px tall in a 1440 × 700 window (about 258 px at 900 px).
  The owner then chose to draw D1's repository details at the end of the
  item's own scroll region, after the discussion, "for now", as D1 drew
  them at the end of the content column. The owner wants to see it on
  screen. The Design Deviations row is rewritten to match, and native
  checklist step 1 gains the check. No test changes.
- Owner decision, 2026-10-27 (during Task 5a's fix round): "We don't
  really need tests for that kind of thing." Desktop layout and sizing
  need no automated tests: region heights, short-window overlap,
  extra-frame timing, a control's place in a row, thumb colours. Task 5a's
  fix round made its layout changes without them, and reviewers do not
  raise "untested" findings for pure layout. The plan's own listed tests
  stay ("for now keep the tests").
- Owner decisions, 2026-10-27 (builds take very long on this machine).
  Both apply to the rest of D3:
  - **The per-task gate is trimmed.** The task's test filters run once,
    at the end, then `clippy --features desktop --bin manyhands` and
    `clippy --all-targets --all-features` once each. There is no
    separate `check --features desktop`; clippy covers it. A fix round
    reruns only the tests it touched, plus the all-features clippy.
    Minor review findings are parked unless trivial, and the controller
    reviews small fix diffs itself. Part checkpoints and Task 27 keep
    their full runs.
  - **Line tables only.** Every D3 cargo command carries
    `CARGO_PROFILE_DEV_DEBUG=line-tables-only`. The repository does not
    change.

  Follow-up: make both repo-wide (`Cargo.toml` dev profile and/or
  `AGENTS.md`) as their own change on main, with a ticket the owner
  approves.
- Owner decision, 2026-10-27 (from the controller's timing): line tables
  are dropped. Timings after a one-line desktop edit, at `25dbbb4`, full
  debug info first, line tables second:

  | Step | Full debug info | Line tables |
  | --- | --- | --- |
  | Test-binary build | 6 s | 5 s |
  | `view::item` run, 36 tests | 12 s | 20 s |
  | `clippy --features desktop` | 1 s | 131 s |
  | `clippy --all-targets --all-features` | 4 s | 92 s |

  The two line-tables clippy runs include the setting's one-time rebuild.
  The test binary's one-time rebuild took 236 s.

  Incremental builds are not the bottleneck. The time goes to the long
  targets: the full suite (over 2 h), `mutation_remote` (about 18 min)
  and `ticket_closure`. It also goes to cold builds in a new worktree.
  The trimmed gate stays. The repo-wide follow-up now targets those long
  targets and shared compiled dependencies (for example sccache), not
  debug info.
- Ruling (Task 5a fix round, controller re-review): approved after
  reading the diff in full. Every tab opens through `open_item`
  (`content.rs:213` is the only `tabs.open` call), so every pane gets its
  region end. `opening_a_tab_does_not_change_the_selection` proves the
  details are drawn with a tab open. — If wrong: a pane opened another way
  would lack the repository details.
- Owner decision, 2026-10-27 (from Task 5b's review): a library gap. The
  design's "This ticket has no type, so Manyhands can't save it yet."
  cannot be produced. `ProblemDto` has only a code, a path and a target
  ID, and `missing_field` covers every required key. "Needs fixing" shows
  D1's reason for every nonconforming file. The design's Banners row and
  Known Limits say so. No library change, no ticket.
- Ruling (Task 5b, E-7): the header shows one primary action, by kind
  (the design says "Close ticket… or Approve and merge…"). E-7 defines
  the test's rows as "the header's later actions", so for a ticket file
  the test drops "Approve and merge…" as well as "Comment". — Within
  E-7's text. — If wrong: one line in `header_later` and one in the
  test's filter.
- Owner decision, 2026-10-27: CLI and desktop Cycles run no library
  tests, SSH and remote targets included, unless they change library
  code. A task that changes library code runs only enough to verify its
  own change. For D3 this means:
  - desktop tasks run no library tests;
  - Task 22 (`src/drafts/`) runs only the filters that cover its change;
  - merges of main do not retest main's library code;
  - Part checkpoints and Task 27's final gate run the front-end targets,
    plus targeted filters for D3's own library changes. This replaces
    the handoff's whole-suite run in Task 27. Part 0's library work was
    verified in full at its checkpoint.
- Ruling (merge 1): no library change for local sync in release's check
  4(d). That check sees only remote merges, but every local sync keeps a
  pending journal row until it ends, so `begin_lifecycle` refuses both
  context commands first. Main's audit enforces that. The
  `context_release.rs` module doc says 4(d) covers "no synchronization
  of the item", which is loose wording, carried to the whole-branch
  review. — Part 0 is closed, and the guard holds. — If wrong: a local
  sync that left no pending row could be released under, which main's
  audit forbids.
- Owner decision, 2026-10-27 (Task 6, E-30): in
  `view/tickets_tests.rs::arrows_move_in_the_list_and_enter_opens`, one
  line more than E-30 names, `shell.keys("secondary-2", cx);` after the
  Escape that closes the sheet. Closing the sheet returns focus to All
  details (fact 80), so the test's later Down presses need focus back in
  the ticket list. The design's E-30 row records the amendment.
- Owner decision, 2026-10-27 (Task 6): two of Task 5b's own tests in
  `view/item/header_tests.rs` are updated to the sheet. They are D3's
  tests, not main's, so this is not a named edit.
  - `every_chip_emits_open_details_for_its_field` presses Escape after
    each chip's click, in both loops, since the first click now opens
    the sheet and its backdrop takes the next click (fact 61).
  - `tab_order_runs_strip_header_chips_page` asserts that `item-page`
    holds focus after All details, instead of `copy-id`, which E-23
    moves into the sheet.
- Ruling (Task 6): Root's sheet layer is drawn inside the content
  region, deferred. The sheet still renders in Root's layer, outside the
  tab. Its focus counts as the content region's, so E-23's unchanged
  `in_content` and `ringed(Content)` assertions hold, and its keys pass
  the region's context, where R-60 refuses them. — It satisfies the
  design's text and E-23's "assertions unchanged". — If wrong: the
  layer moves back to Root's children, and E-23's region assertions
  need a named edit. Paint-order note for a later Cycle: the sheet now
  paints above Root's notification layer. A Cycle that adds toasts
  should defer that layer at priority 1.
- Ruling (Task 6 review, Minors 1–3, parked): the sheet paints above
  Root's notification layer, and a Cycle that adds toasts should defer
  that layer above 0. Inside the sheet, Content-context bindings match
  and focus counts as the content region's, so Task 21 must not assume
  the old dispatch path. A loading or unreadable Inspect tab shows no
  kind line until its read lands, since "Nonconforming file" is now keyed
  on the file. — No user-visible defect today. — If wrong: a toast hidden
  under the sheet, or a brief blank line on an Inspect tab.
- Ruling (Task 7, concern 5): a reload of a changed body loads the new
  body read; an unchanged body leaves the editor alone. `set_exact` is
  only for text an editor already holds (Tasks 18 and 19). — R-79's
  third bullet says so. — If wrong: none; the reload path normalizes
  once, as contract 2 states.
- Ruling (Task 7): read-editor focus hooks are checked as each frame is
  drawn (`defer_in`), not through `cx.on_focus`. GPUI sends focus events
  only to an active window, and the test window never is. Every focus
  change draws a frame, so the app behaves the same. — The review
  confirmed there is no loop and no focus stealing. — If wrong: the caret
  reveal lags one frame.
- Ruling (Task 8): two of D3's own tests follow the discussion. The
  owner's Task 6 decision settled that tests like these change by a
  ruling, not a named edit.
  - `header_tests::tab_order_runs_strip_header_chips_page` passes
    Comments {n} between the strip and the first chip. That is the
    handoff's Tab order: tab strip, header actions, chips
    (`item_view/01-item-frame.md:57`).
  - In `frame_tests::back_to_top_returns_and_focuses_the_page`, the
    first End becomes `secondary-shift-c`. At 700 px the discussion is
    taller than the region, so End leaves Back to top above the fold.

  — Both reviews confirmed the diagnosis. — If wrong: two test lines.
- Ruling (Task 8): the avatar's colour comes from the whole `created_by`.
  The handoff says "color per person" and R-56 says "the author's name";
  `created_by` is the author's identity, so two people who share a name
  differ. — If wrong: one word.
- Ruling (Task 8 review, Minor 1): E-24's consequential lines are part
  of E-24, as E-23's `details()` helper was. They are the imports its
  named assertions need, and the renamed test's own doc comment. — No
  change to behaviour.
- Ruling (Task 8 review, Minors 4 and 8, parked): a comment's time shows
  no year. That is a Known Limit, since the handoff gives two forms
  only. The shared caret-reveal helper is carried to Task 9, where the
  edit editor needs the same reveal. — If wrong: one formatter form, or
  one helper extracted now.
- Owner decisions, 2026-10-27, from the Part A checkpoint review:
  - **Minor 2.** The state pill never hides a kept or stored draft.
    With recovery or closure, it reads "Needs attention · unsaved
    changes kept" or "Closed · unsaved changes kept" and keeps their
    icon (contract 16). The design's State pill table records it.
    Task 5b's `the_pill_reads_each_viewing_state` follows it by ruling,
    since it is D3's own test.
  - **Minor 3.** The read view's hover handles (block grip, table +/−
    pills and column bands, callout fold arrow) are a Design Deviations
    row. Checklist step 2 checks them on screen.
  - **Minor 4.** Waiting on shows D1's dependency rows while viewing, as
    a Design Deviations row. The chips come with Task 21's picker.
- Ruling (Part A review, Minor 1): the details sheet says "Viewing your
  private copy. Choose Edit to change these." for an item read from its
  own context, as the pill does. — The handoff's line assumes the shared
  copy. — If wrong: one string.
- Ruling (Part A checkpoint): `--test drafts` is left out of Step 2. It
  tests the library's `src/drafts/` store, which Part A does not change
  (owner decision, 2026-10-27). It runs where Task 22 changes
  `src/drafts/`. — If wrong: a drafts regression from main is found
  later; main tests its own library code.
- Ruling (Part A review, Minors 6 to 8): parked or done.
  - **Minor 6.** The Undo stop is proven by a new test with real
    history. The Redo stop cannot be proven from behind the guard,
    because only a leaked undo fills the redo stack and the revert then
    clears it.
  - **Minor 7.** AltGr text on Windows passes the keystroke layer. Only
    the backstop catches it, and that goes to the post-merge native
    evidence.
  - **Minor 8.** The CLI isolation check was re-run through Devenv:
    exit 0.
- Ruling (Task 9, R-59): the `pressed` flag survives the events a press
  itself causes, until the next frame. A right press moves the caret,
  which emits `SelectionChanged`, then opens its menu. Read literally,
  R-59 would clear the flag on that same press, so Escape could never
  close the menu. One flag serves R-59 and R-66. — The review confirmed
  this against zorite's source (lib.rs 3615–3640). — If wrong: one extra
  Escape.
- Ruling (Task 9 review, Minors 2 and 4, carried):
  - `secondary-shift-m` is bound in `"EditHost"` only. Task 10b or
    Task 15 widens it to the editing tab and forwards it to the host.
  - Task 14 calls `EditorHost::place` right after `new`.

  — Both are staged, not defects. — If wrong: a key that works only
  inside the editor.
- Ruling (Task 10a): the implementer wrote the tests together with the
  code, not first. RED rests on one mutation, plus the review's reading
  of each test's strength: none passes for a plainly wrong
  implementation. The fix round recorded a RED for each fix. — The
  module is pure and well covered. — If wrong: a weak test, which a
  later edit would find.
- Ruling (Task 10a): the commits and the comment carry "Claude Sonnet
  5.5" as co-author, not common.md's Opus line. The implementer was
  Sonnet, so the line is accurate. `markdown_edit.rs` is 513 lines, at
  the guideline. — If wrong: a split into three files.
- Ruling (Task 10b): in Markdown mode, Align left, centre and right are a
  host edit (`markdown_edit::align_column`), not the editor's method. The
  editor's `set_caret_table_align` reads the drawn table rows, and
  Markdown mode draws none, so the method does nothing there. The other
  four table controls read the text, and they use the editor's methods
  in both modes. The host edit matches the editor's rich-text result
  byte for byte. Past a row's last cell it aligns the last column, as the
  editor does. — R-55 (a host edit where the editor lacks the action) and
  "Markdown mode: the formatting buttons … stay operable", which the
  design's Table row ("Editor table methods") did not foresee. — If
  wrong: Align is inert in Markdown mode.
- Ruling (Task 10b): the toolbar is one roving tab stop. Tab enters at
  its first control, and Tab or Shift-Tab leave from any control. In the
  Style menu, Up, Down, Home and End move, and Left and Right do nothing.
  `secondary-shift-m` also works from a toolbar control, where the
  toolbar forwards it to the host. Widening it to the whole editing tab
  is for Task 14 or 15. — It follows the common toolbar pattern and
  fact 67. — If wrong: key handling in one file.
- Ruling (Task 11 review, judgment 4): a link whose exact path finds a
  file with no ID opens that file's Inspect tab, in the context where it
  was found, as the lists open such a file. A symbolic link, or a file
  that is not a regular file, still says "Link not found". — The file
  exists, so "not found" is untrue (contract 8). The design names only
  managed items, `invalid_path` and absence. — If wrong: one arm in
  `services/links.rs`.
- Ruling (Task 11 review, judgment 5): a `#tag` click is a Known Limit
  and nothing changes. zorite 0.10.0 sends it as `OpenWikiLink("tag")`,
  with the `#` already removed, so it cannot be told apart from
  `[[tag]]`. — If wrong: a banner on a tag click.
- Parked (Task 11 fix 1): the item's context holds only the item itself
  (`repository/discovery.rs:932-935`).
  - So the test `an_exact_path_in_the_items_context_wins_over_primary`
    opens a context file the index does not hold.
  - The context-first nearest-name test covers a zero-step tie.
  - The resolution is right, but the scenario is not realistic. A
    realistic one moves the linking document itself within its context.
    This goes to the Part B review.
- Ruling (Task 12, concern 1): on Windows, check 4 records the real file
  ID, as the design says, and not `std`'s size, times and attributes.
  `windows-sys` is already a Windows dependency with
  `Win32_Storage_FileSystem`. The code follows the library's
  `coordination.rs::file_identity` (`FILE_ID_INFO`). So nothing in Cargo
  changes. — The design names "the Windows file ID", and stable `std`
  gives no file ID. — If wrong: a Windows build failure after merge,
  which is a follow-on fix in one file.
- Ruling (Task 12, concerns 2, 4, 5 and 6): four behaviours become Known
  Limits; none is a defect:
  - the containment checks are path-based, as the design lists them;
  - images under `.manyhands/` are refused by check 2;
  - images draw as source after a reload;
  - an image added after its `src` was looked up shows only after a
    reload.

  — Each follows the design's own rules. — If wrong: a future change to
  containment or to the cache.
- Ruling (Task 12, concern 3): in the exact lookup, the first tree that
  holds anything at the path decides. A refused file there does not fall
  through. — A refused file never leads to a different image. — If
  wrong: one function in `services/images.rs`.
- Parked (Task 12 fix 1):
  - review Minor 4: the test helpers `png`, `drawn_as_image` and `pane`
    are repeated across three test files;
  - `services/images.rs` is 511 lines, about 45 of them test-only. The
    implementer proposes splitting out `services/images/load.rs`;
  - the GIF case of `max_alloc` was not built. A 4096 × 4096 RGBA16 PNG
    pins `max_alloc` instead.

  These go to the whole-branch review. The owner checks the redraw after
  a reload on screen (Task 27).
- Rulings (Part B checkpoint, fix round 1, controller, 2026-10-27):
  - **Absolute link targets.** An absolute, rooted or prefixed target is
    refused even inside the root (a Known Limit in the design). So the
    in-tree alias in Task 12's
    `dot_dot_symlink_escape_git_and_other_worktrees_are_refused` becomes
    the relative `inside.png`, with the message "a relative link that
    stays inside is followed". It is D3's own test, changed by ruling.
  - **`..` in a target** is taken in order from where the walk is, after
    the links before it, as `realpath` takes it, with no lookup. The whole
    target is never collapsed first: `docs/n -> m/../x` with
    `docs/m -> /net/host/a` would collapse to `docs/x`, and
    `canonicalize` would then follow `m` out.
  - **Nothing there.** A missing step of the written path tries the next
    tree. Once the last step is there, a dangling final link or a refused
    target is something, refused.
  - **Splits.** The walk and its seam go to `model/resources/contain.rs`,
    the background load to `services/images/load.rs`; moves, not
    rewrites.
- Implementer decisions (Part B fix 1):
  - The seam also records `canonicalize`, so a test shows a path refused
    for a link is never made canonical. — The brief names two calls. — If
    wrong: drop the third method and the empty-list assertion.
  - A reserved step is refused once the path is known to be there, so
    `.git/none.png` is nothing there, as the exact lookup found it before.
    — If wrong: refuse at the step, and the next tree is not tried.
  - The 40-link chain is asserted contained on Linux only; elsewhere the
    test checks that the walk passed it. macOS follows 32 links. — If
    wrong: a native macOS failure, fixed in the test.
- Ruling (Part B review, Important 1): image containment decides each
  symbolic link without following it.
  - Each link is read with `read_link`.
  - A relative target is walked from the link's folder, and each `..` is
    resolved in order, as `realpath` does.
  - Every link in a chain is decided, up to 40 hops.
  - An absolute, rooted or prefixed target is refused, even one inside
    the root. This is a Known Limit.
  - `canonicalize` runs only after every link on the way has passed.

  — Contract 12 says an image is never reached through a link leaving
  the tree or through a network path. A link that is followed before
  it is refused can look up an automounted or UNC path. — If wrong: an
  absolute link inside the repository shows its image as source.
- Ruling (Part B fix 1): the Task 12 test
  `dot_dot_symlink_escape_git_and_other_worktrees_are_refused` now links
  its alias by the relative target `inside.png`. It is D3's own test, and
  the absolute in-root case is now refused by the ruling above.
- Owner decision (2026-10-27, Part B review, Important 2): a muted line
  below the edit toolbar says "Esc: Done · F6: next region". It is a
  Design Deviation row, for contract 13.
- Owner decision (2026-10-27, Part B review, Minor 1): every image in the
  read view and in comments shows the editor's resize grip. It joins the
  Design Deviations row "A view mode with no editing handles", and
  checklist step 2.
- Ruling (Part B review, Minor 2): on Windows, any reparse point is
  refused as an image, which includes OneDrive placeholders. It is a
  Known Limit, because the refusal is safe and the Windows code cannot be
  compiled here.
- Ruling (Part B review, Minor 3): for Task 26's privacy sweep. GPUI's
  `open_url` logs the URL when it fails, on Windows and macOS. So the
  desktop installs no logger that keeps GPUI's `error` records, or else
  it filters them. Nothing is written today, since no logger is
  installed.
- Part B review: none of the parked minors is load-bearing.
- Ruling (Part B re-review 2, an older Windows issue from Task 12): a
  step that Windows reads as a device name (CON, PRN, AUX, NUL, COM0 to
  COM9, LPT0 to LPT9, COM¹²³, LPT¹²³, CONIN$, CONOUT$) is refused.
  - It is matched as Windows matches it: the part before the first `.`
    or `:`, with trailing spaces trimmed, ignoring ASCII case.
  - It applies to the written path and to a link's target, on every
    platform, before any lookup.

  — Contract 12: a device is never opened, and a mapped port can reach a
  network printer. — If wrong: an image named like a device shows as
  source on Unix. This is a Known Limit.
- Ruling (Part B fix 3): the hop-limit test is D3's own. Its macOS
  comment and Linux-only guard are dropped. It now asserts the same on
  every platform.
- Part B checkpoint: complete.
  - The whole desktop binary passed at 399035d: 737 passed, 0 failed.
  - After fix 3: `resources` 29 and `images` 12 passed.
  - Clippy (desktop only, and all targets with all features) and fmt
    are clean.
  - CLI isolation is clean (at af24fa5).
  - Main is unchanged, and merge-tree is clean.
- Ruling (Task 13): Task 11's `ReadKind::Observe` is renamed
  `ObservePath`, and it serves every `observe_path` read. No second kind
  is added. — The plan names `ObservePath`, which Task 11 did not know.
  — If wrong: a rename.
- Ruling (Task 13): `ReadKind::NewId` and `RelationshipCheck` carry a
  plain `allow(dead_code)`, because no test builds them yet. Tasks 22 and
  21 remove it. — Under `--all-targets`, the `cfg_attr(not(test), …)`
  form warns. — If wrong: one attribute each.
- Ruling (Task 14, controller, 2026-10-27): the nine D3 tests a real
  Edit breaks are edited as `task-14-proposed-test-edits.diff` proposed.
  - Five follow from E-6, since Edit leaves the later actions and takes
    the tab stop after Comments. They are, in `header_tests`,
    `share_and_the_primary_action_are_disabled_with_their_note`,
    `a_nonconforming_item_shows_needs_fixing_with_repair_disabled`,
    `an_inspect_tab_of_a_conforming_item_needs_no_fixing` and
    `tab_order_runs_strip_header_chips_page`, and, in
    `discussion_tests`, `comment_is_disabled_with_its_note`.
  - Four `page_tests` type "typing", with no `e`, where they typed
    "typed": `typing_and_paste_in_the_tab_queue_no_request`,
    `viewing_queues_no_request`, `viewing_changes_nothing_in_the_repository`
    and `typing_and_paste_in_the_tab_change_nothing_and_begin_no_draft`.

  — All nine are D3's own tests. — If wrong: the edits go back, and
  Edit stays a later action.
- Ruling (Task 14, controller, 2026-10-27): an Inspect tab offers no
  Edit, and neither does a nonconforming item. — An Inspect tab stays
  the read-only source view (R-64). — If wrong: Edit appears in Inspect
  tabs.
- Ruling (Task 14, controller, 2026-10-27): with a stored draft, Edit
  reads Continue editing and, until Task 18, does D2's Restore. Once
  that draft is restored in the tab, Continue editing is disabled until
  Task 18. — R-35: no second draft begins. — If wrong: Task 18 routes it.
- Ruling (Task 14, 2026-10-27): a typed `e` in the read view starts
  Edit. `e` never acts in a text field, a dialog, the details sheet, the
  link popover or the edit editor. Task 7's four page tests type "typing"
  instead of "typed". — The design's keyboard map gives "E edit (when the
  page has focus)", and fact 76 says the guard stops text, not bindings.
  Review Focus 5 is about the read view's text, which never changes, and
  no draft begins. — If wrong: `e` is bound outside the read editor, and
  those four tests type "typed" again.
- Ruling (Task 14): a clean editor that has not saved yet reads
  "Editing · saved on this computer". That is the design's row "Editing,
  saved and clean": the text equals the file. — The design has no row
  for "opened, not changed". — If wrong: one pill word.
- Ruling (Task 14 review, Minor 2): a `discovery_pending` answer to
  Edit's `context prepare` keeps the tab in Loading until Resume. The
  pill reads "Getting a private copy ready…" and Edit stays disabled.
  The banner offers Resume, which sends the same request again. — Phases
  step 3, R-13 and R-77. — If wrong: the phase the pill reads while
  waiting.
- Correction (Task 14 fix 1): the fix brief said a tab closed while
  waiting for Resume leaves its context in `CreatedContexts`. It does
  not, and the code is right. R-75 and the Known Limits keep a context
  whose prepare answered `discovery_pending` and was never retried out of
  the set. Only the retry's `ok` adds it, so Task 17 does not release
  such a context.
- Parked (Task 14 review):
  - Minor 5: the toolbar's status redraws each second while a draft is
    kept (checklist step 10);
  - Minor 6: D2's close dialog says "restored draft" for an edit
    session's draft. That is for Task 16 or 18.
- Ruling (Task 15, concern 2 and review): Save commits after a failed
  draft write.
  - D2's `send_save` sends the session's current fields directly from
    the failed-flush branch, once, with no retry loop.
  - A committed save whose submitted fields equal the session's current
    fields retires the session, even though that revision was never
    written. Any older stored revision is discarded.
  - Newer typing keeps D2's rebase.
  - A failed discard keeps the draft.

  — The toolbar says "Draft not kept. Save to protect your changes", and
  Task 16's row "Done, dirty, the write fails" offers Save. — If wrong:
  one branch of `send_save` and one of `settle`.
- Ruling (Task 15, concerns 3 and 4): two choices are accepted.
  - With no D2 session, because the store did not open or the draft did
    not begin, the tab sends the fields itself and applies D2's
    `base_after_save`.
  - A context leaves `CreatedContexts` at the services level when a
    save's checkpoint is committed. That covers a tab closed before its
    save finishes.
- Ruling (Task 15, concern 1): Save on an unverified base, or an
  `external_change` answer, enters `Phase::Review` with nothing written.
  The review's content and Cancel are Task 19's.
- Parked (Task 15 fix 1): a pending save sent with no D2 session ends
  only on Retry or Dismiss, not when its operation is abandoned. The
  same goes to the whole-branch review.
- Ruling (Task 16 review, Important 1): a session is dirty when its
  current fields differ from its base, the saved copy after a save
  (R-69), or when the item is not yet created. The editor's own dirty
  flag is not used. This applies at every reading: Done, closing, the
  pill, Save and the unsaved mark. — The design defines dirty against
  the baseline, the saved copy. Before this, the editor's own flag let
  Done discard text that had been undone past a save. — If wrong: one
  predicate.
- Ruling (Task 16 fix 2): the read after a save reloads the edit editor
  only when the session is clean and no live draft holds text. Otherwise
  the editor keeps its text and undo. — R-79, and the design's Late
  results ("not dirty, no draft"). — If wrong: one guard.
- Ruling (Task 16 fix 1): the D3 test's refusal case now comes from a
  library failure point, not a self-dependency. Task 21's cycle check
  would block the self-dependency before it was sent.
- Parked (Task 16): two items wait for later tasks.
  - **For Task 19.** After a save, another actor's body is reloaded into
    a clean editor, and it then reads dirty against the unverified base.
    Done's and closing's words then say the changes could not be kept as
    a draft. The next Save opens the review, and nothing is lost. The
    suggestion is that the base takes the reloaded body and stays
    unverified.
  - **For Task 18 and the whole-branch review.** Review Minor 9: a failed
    Keep draft for a kept session says nothing. Also, the Drafts page
    lists an edit session's draft as "Restored in this window", which are
    D2's words.
- Owner decision (2026-10-28, from the Task 17 review, M3): R-78 is
  narrowed. A release that the library answered with a refusal does not
  block the earlier prepare's Resume. A release whose answer was lost,
  one still queued or running, and one that answered `ok` still block it.
  The other half of R-78 is unchanged: a release is never retried after
  a later prepare. — The case this fixes takes three steps:
  1. a prepare answers `discovery_pending`;
  2. another tab's R-40 release is refused because of the prepare's
     pending row;
  3. R-78 then blocks the only request that could finish that row.

  Recovery needed Abandon in the operation area. The design's R-78 is
  amended.
- Ruling (Task 17 review, M2): a context is never released while a tab of
  this process holds an edit session of the item, live or kept. That
  includes Preparing, and Loading after `ok`. It is released when that
  tab leaves. — The design's four conditions did not foresee a discard on
  the Drafts page, of another process's draft, while a clean tab edits
  the item. Without this rule, the release removes that tab's worktree. —
  If wrong: a context stays until its tab leaves.
- Ruling (Task 17 re-review 1, N1): the owner's narrowing of R-78 is
  decided over every attempt.
  - A release exempts the earlier prepare only while every attempt of it
    finished with a refusal that touched nothing, and none is in flight.
  - An attempt that is lost, unknown or partial, or that answered `ok`,
    blocks for good.

  — The narrowing's own words say a release whose answer was lost still
  blocks. Deciding by the last attempt alone would let a refused retry
  hide an earlier attempt that deleted the context. — If wrong: a prepare
  stays blocked, and Abandon in the operation area recovers.
- Ruling (Task 17 fix 2): Edit waits while the close dialog's Discard
  waits for a draft that is beginning. The Discard is tied to that begin
  by a token. — Without this, an Edit during the wait started a session
  that the dialog then closed. — If wrong: Edit is inert for that moment,
  and nothing is lost.
- Copy accepted (Task 17): the banner for a Resume that R-78 blocks
  reads "Your private copy cannot be made ready again from this request:
  a release of this item was sent since."
  - When the operation area lists the item's pending row, it goes on:
    "Abandon its pending operation in the operation area, then press
    Edit."
  - Otherwise it goes on: "Press Edit to try again."

  The operation area's Retry note "Not sent again after a later Edit or
  release" is accepted too.
- Parked (Task 17), for the whole-branch review:
  - M5: a repeated leave keeps the first leave's mark, so the context is
    kept;
  - closing the tab again while its Discard waits keeps the landed draft
    for the Drafts page;
  - Edit gives no reason while it waits on a Discard;
  - `pending_row_listed` does not match the row against the request's
    operation ID;
  - `Releases::pending` is ready for Task 24's Move.
- Ruling (controller, Task 18): Task 14's
  `edit_reads_continue_editing_while_a_draft_is_stored` pins the stand-in
  that carried obligation 4 replaces, so its edit
  (`task-18-proposed-test-edits.diff`) is approved as written. The test now
  shows a stored draft's Continue editing claiming the draft and preparing
  once, and a draft restored by D2's Restore offering Continue editing
  with no note. Applied with one line more than the diff,
  `cx.executor().allow_parking()`, because the prepare runs on the worker.
- Ruling (controller, Task 18): five choices confirmed.
  1. An unclaimed stored draft is claimed first, then its context is
     prepared.
  2. R-42's three-part base check decides; otherwise the review opens. The
     draft's text goes in through `set_exact`, and the cursor is set
     afterwards.
  3. The close dialog stays open while it saves. It shows a pending
     checkpoint with Retry, or a refusal with the library's message and
     code. The tab closes once the draft retires.
  4. `NotCreated`: Continue editing and Save are disabled with "Available
     in a later release", behind a named hook for Task 22
     (`continue_new_item`).
  5. No late read replaces a dirty editor, or one whose draft is live.
- Ruling (controller, Task 18, obligation 8): D2's words stay. Main's
  `drafts_tests.rs:388` and `drafts_held_tests.rs:86`, and Task 16's
  `edit_done_tests.rs:107`, pin "Restored in this window".
- Ruling (controller, Task 18 review I1): a late read adopts the read's
  base only once the editor holds the read's body, and the reload is
  decided on the session's clean state, not on the editor's own flag
  (`EditorHost::reload_read`). — The editor's flag keeps its baseline
  after a save that newer typing outlived. Adopting the base first let a
  plain Save send our older body with another actor's token, which
  breaks contract 7. — If wrong: a small host method more than needed.
- Ruling (controller, Task 18 review I2): the close dialog's Save covers a
  session kept after Done. It un-keeps the session without moving focus,
  then saves, and closes the tab once the draft retires. — The design's
  "Closing the tab" lists a kept draft, with Save enabled when dirty and
  nothing is saving. — If wrong: Save could have read unavailable, with
  its reason.
- Ruling (controller, Task 18 review M1): Cancel, Keep draft and Discard
  turn a close-dialog Save still in Continue editing's steps into
  Continue editing alone, and a continued draft that ended is dropped. —
  A write the user cancelled must not happen. — If wrong: nothing; the
  editor still opens on the draft.
- Ruling (controller, Task 18 review M2): D2's dialog's Save is gated on
  `edit_available`, as the section's Save is. While a prepare waits for
  Resume, the dialog says so, and Cancel returns to the banner's Resume.
  The dialog offers no Resume of its own. — It keeps one Resume, in the
  tab. — If wrong: one extra Cancel.
- Carried (Task 18 review), to Task 19:
  - M3: a clean session on an unverified base takes another actor's body
    while its base keeps our fields, so it reads dirty for text the user
    never typed. This is Task 16's N2 on the late-read path.
  - The post-save read still decides its reload on the editor's own flag
    (`reload_clean`). It never adopts a base, so nothing is overwritten,
    but it should follow I1's rule. Task 19 decides both paths together.
  - M4: the review opened from the close dialog sits behind the dialog.
- Parked (Task 18), for the whole-branch review: M6,
  `view/workspace/content.rs` at 613 lines.
- Decision accepted (Task 19, obligation 2; Task 16's N2, Task 18's M3):
  a clean session on an unverified base takes a reloaded body as its
  base, and that base stays unverified, on the late read and on the
  post-save read alike. The session reads clean, and the next Save opens
  the review. A verified base takes the read. The post-save read waits
  until D2 has retired the draft. — An unverified base never becomes
  verified, so no Save can overwrite another actor's change without the
  review. Adopting the read whole would break Task 15's session row 2. —
  If wrong: one more review than needed.
- Decision accepted (Task 19, obligation 3; Task 18's M4): the close
  dialog stays open over the review, and says "Cancel to review the
  changes." whenever the review is open.
- Ruling (controller, Task 19 review M2): an explicit Keep draft or quit
  during the review writes mine, the text the session holds. — The
  design's "Nothing is written to the draft or the store until Use these
  choices" governs the review's own model and its schedule. The Cycle's
  contract 6, "Newer text survives", governs the user's own typed text,
  which the review never changes, and the Cycle governs the design. — If
  wrong: the store gets mine up to a second earlier than the design's
  letter; nothing is lost.
- Ruling (controller, Task 19 review I1, I2): the review takes the
  newest read that lands while it has no model, and offers Cancel while
  it reads. The review never opens while a save run is kept; Compare…
  and Review… are hidden then; Retry and Resume are hidden while it is
  open; Use clears only an `external_change` answer. — Both left the tab
  at a dead end. — If wrong: Compare… waits until a pending save settles.
- Ruling (controller, Task 19 review M3): a clean tab never asks on close.
  A Use that leaves no change detaches the tab from its draft
  (`DraftRegistry::detach`) and discards the draft; the next keystroke
  begins a new one. — Task 16's rule. — If wrong: one more draft
  lifecycle step; the re-review checked crash safety, typing and release.
- Ruling (controller, Task 19 review M5): the review's intro reads "This
  item changed since your edits began." — The old words blamed another
  actor, which is untrue in known defect `01M4PB7YZM`'s case.
- Carried (Task 19 review M6), to the Part C checkpoint: a restored
  draft's stored base is not normalized before the review's `compare`.
- Parked (Task 19), for the whole-branch review:
  - review M4's remaining gaps: the failed read's Retry, a begin in
    flight when the review opens (a clean Use then still asks on close),
    focus on opening, and `.disabled(!ready)`, which the harness cannot
    observe;
  - re-review N1: a tab closed, or Done, before a clean Use's discard
    lands can skip that discard's release, so a context may be kept. It
    never releases wrongly.
- Part C checkpoint (controller): the review found 1 Important and 5
  Minor issues. Two fix rounds closed them all, and the re-review
  approved round 1. The controller checked round 2's diff.
- Ruling (controller, Part C review I1): quit and the window-close
  request first wait for a begin still in flight. They then ask when any
  tab holds unsaved text that no draft holds: "1 tab has unsaved changes
  that are not kept as a draft" or "N tabs have unsaved changes that are
  not kept as drafts", with Quit anyway and Cancel. Cancel is the
  default and shows the first such tab. D2's own question follows Quit
  anyway.
  - A removal by the user adds to its confirmation "1 tab has unsaved
    changes that are not kept as a draft. Removing closes it." (or "N
    tabs … Removing closes them."). The set it counted is fixed at
    Confirm, and a tab that becomes unkept later stays open.
  - A repository that leaves the list without the user removing it keeps
    such tabs open (Known Limits). No context of it is released by
    leaving afterwards.

  — The Cycle's contract 6: a dirty editor is never dropped without the
  user's choice. — If wrong: one more dialog.
- Ruling (controller, Part C review M2 and re-review N3): a clean session
  whose draft is stored still asks on close, as the design's "Closing the
  tab" says. It uses D2's title, "Close a tab with a draft", with words
  that name nothing: "This tab has a draft, and no unsaved changes. Keep
  it to continue later, or discard it." While the last write failed,
  Keep draft's words say it tries again.
- Ruling (controller, Part C review M1, M3 to M5):
  - the review's base fields equal to the raw read take theirs'
    normalized value;
  - D2's banner Restore, and the Drafts page's Restore ("Open in its tab
    to continue editing"), are disabled while a tab of this window edits
    the item (R-35);
  - a waiting release check re-arms on a later leave or discard (the two
    kept-forever paths of Task 17 M5 and Task 19 N1);
  - a prepare's Resume or Retry takes only its own attempt's answer.
- Ruling (controller, Part C fix 2): Task 18's
  `save_in_the_close_dialog_saves_then_closes` waits in stages, each with
  its own `WAIT`, and its assertions are unchanged. — Its one 10 s wait
  covered six steps of real git work, and timed out in whole-binary runs
  at a load average of about 19. — If wrong: none; it proves the same
  outcome.
- Copy for the owner's on-screen check (Part C): the quit and removal
  lines above, the clean-session close words, "Open in its tab to
  continue editing", and "Cancel to review the changes."
- Parked (Part C), for the whole-branch review:
  - files that grew over 500 lines: `view/workspace/content.rs`,
    `view/tabs.rs` and `view/confirm.rs`;
  - D's Restore gate is held both by the disabled button and by a check
    in `restore`, so its test fails only when both are removed.
- Ruling (controller, Task 20 review M1 and M2): every save consults one
  check, `save_blocked`: each offer, `start_save`, and `send_save`, where a
  deferred save is finally sent. Today it holds an empty required field;
  Task 21's relationship rejection joins it there. The close dialog names
  a missing field only when nothing else blocks its Save, and not for a
  new item's draft. — The fields go as they are when sent, and the
  controls take typing meanwhile. — If wrong: one guard.
- Ruling (controller, Task 20): a chosen suggestion replaces the control's
  text as one edit the control can undo. Suggestions come from the
  workspace's ticket list when it is the tab's repository's, plus the
  item's own values; with another repository's list loaded, only the
  item's own values (R-57). "Empty" follows the library's rule, so spaces
  fill a field. — No distinct-values read exists (fact 64). — If wrong:
  fewer suggestions, never a wrong one.
- Copy for the owner's on-screen check (Task 20): the required-field
  messages ("A ticket needs a type before it can be saved." and the rest)
  and the red border; the suggestion buttons; during the review the sheet
  keeps "Saved with the ticket (Ctrl S)" while its fields are read only;
  Done's prompt disables Save for an empty field without saying why;
  clicking a row's label focuses the row, not its input; with focus on
  the Kit's own close button, `secondary-s` does nothing.
- Parked (Task 20), for the whole-branch review:
  - some assertions are weaker than they read: the tab key's "says why"
    reads a note the sheet key had set; the draft-section test has no
    positive control; the review guard in the sheet's `secondary-s` is
    masked by `start_save`'s own Review refusal;
  - suggestions are rebuilt each frame while the sheet is open;
  - the controls are cleared only when the sheet is next drawn with no
    session;
  - one send is not re-checked: if D2's session ends while its save is
    asked, `registry_sent` falls back to `send_direct`. The library refuses
    an empty field there;
  - the close dialog's note order for a new item's draft has no test
    until Task 22's harness.
- Ruling (controller, Task 21 notes): a relationship rejection is
  explained under the field whose change the check answered; a save's
  answer under Waiting on when its `deps` differ from the base, else under
  Parent. The words are fixed: "This makes a cycle: {names}.", "A ticket
  cannot depend on itself.", "A ticket cannot be its own parent.", "Not a
  ticket: {names}.", Save's "A ticket cannot be saved with a relationship
  cycle." and "A ticket's dependencies and parent must be tickets.", "not
  found, kept", "The index may be missing tickets. Save checks again." The
  picker never offers another repository's tickets. — The rejection gives
  only a code and a set of IDs. — If wrong: an explanation under the other
  field.
- Decision (Task 21): a proposal equal to the base's relationships is not
  checked, and no answer holds for it: its save sends both as `Unchanged`,
  which the library does not check. A save's relationship refusal is held
  like the check's rejection until the next change is checked. A short
  code in the picker's filter matches only exactly, through the library;
  titles match as substrings. — If wrong: one more read on a change back
  to the base.
- Copy for the owner's on-screen check (Task 21): the chips (icon, short
  code and title, state, ID, the Remove icon), Add…, Change… and Clear as
  small outlined controls, the picker under its row with its filter
  ("Filter by title, or type a short code"), "No ticket matches", the
  explanations in the danger colour, and the index note at the end of the
  relationships; while editing, the chips do not open their tickets.
- Parked (Task 21), for the whole-branch review:
  - the stamp check in `check_landed` is masked by the dropped task of a
    replaced check; the late-check test fails only when both go;
  - a ticket naming itself cannot be proposed in the desktop (the picker
    leaves it out, and the library reads a file naming itself without
    it), so "A ticket cannot depend on itself." and "… its own parent."
    are tested in the model only;
  - `view/item/sheet/ticket.rs` is 486 lines.
- Ruling (controller, Task 21 notes): the relationship picker lists only
  the tab's repository's tickets, and leaves out the ticket itself and its
  current dependencies. A check in flight does not block Save; only a held
  rejection does, through `save_blocked`. — The design's picker is "over
  `list_tickets`", and the workspace's list may be another repository's;
  the check is advisory and the save checks again (R-26). — If wrong: a
  picker filter, or one gate condition.
- Ruling (controller, Task 21 notes): a rejection names only a code and a
  set of IDs, so its explanation sits under the field whose change the
  check answered. A save's refusal sits under Waiting on when `deps` differ
  from the base, and under Parent otherwise. The words: "This makes a
  cycle: {names}.", "A ticket cannot depend on itself.", "A ticket cannot
  be its own parent.", "Not a ticket: {names}.", Save's reasons "A ticket
  cannot be saved with a relationship cycle." and "A ticket's dependencies
  and parent must be tickets.", "not found, kept", and "The index may be
  missing tickets. Save checks again." — The design gives only "not found,
  kept" and "Save checks again". — If wrong: copy edits.
- Ruling (controller, Task 21 review I1, M3 to M5): a save's refusal
  changes only the rejection. What the last check of the same proposal
  said of unresolved IDs and the index stays, and a check still running
  lands without undoing the refusal. Use these choices checks the
  proposal again against the new base. A ticket's name comes from the
  item's own reads first, then the picker's, which clear when the session
  ends. Closing the sheet closes the picker. — An unresolved dependency
  read "Resolved" after a refusal. — If wrong: one merge rule.
- Copy for the owner's on-screen check (Task 21): the words above; "Could
  not list the tickets"; the picker shows at most 8 tickets, with no hint
  that more exist, and an empty filter shows the first eight in list
  order; the chips' look, Remove, Add…, Change… and Clear.
- Parked (Task 21), for the whole-branch review:
  - a held rejection is not checked again when the index changes, only
    when a relationship does. If another actor removes the cycle, Save
    stays disabled until the user changes a relationship;
  - the stamp check in `check_landed` is redundant with dropping the task,
    so removing it alone breaks no test; and the two picker drops (on draw,
    and in `open_details`) mask each other;
  - the picker's Tab to an offer, and Enter or Space on one, have no view
    test; the merge of a check landing after a refusal is pinned only in
    the model; clearing the picker's names at session end was checked by
    reading;
  - the stale-index test writes the registry's `refresh_required` through
    rusqlite, as `tests/mutation_relationships.rs` does;
  - the self-reference words are tested only in the model. No desktop path
    can propose a self-reference;
  - `Operations::rejected` is never pruned;
  - `sheet/ticket.rs` is at 486 lines. The next change there should move
    the relationship drawing into its own submodule;
  - test nits: `shown_raw` duplicates `shown`, and the `DOCUMENT` ID is
    used as a ticket in repository "b".
- Rulings (Task 22, pre-flight): R-20's order is one entry point,
  `DraftRegistry::new_item`, which Task 23's dialog calls with the ID it
  took; R-46's words are "Crash recovery is off, so a new item cannot be
  created."; an `occupied_path` that is not this draft's own item, and
  `initials_required`, show the library's message and code as other
  refusals do, the draft kept (Task 23 gives them their own words and the
  pointer to Settings); P-3 covers all four `as_new_item` tests (Task 16
  ×3, Task 17 ×1), rewritten over a real never-created item with the same
  names and results, and the hook is removed; `ReadKind::NewId`'s allowance
  names Task 23; the desktop's `new_item_base` twin is replaced by the
  library's `is_new_item_base`.
- Decisions (Task 22, implementer): the item's own context for
  `occupied_path`'s `show_path` is the worktree its Create prepare's `ok`
  answer gave, kept by the registry (the library names no context path
  publicly); with none known the root's copy is read. A create whose effect
  is unknown keeps its record for the same request's retry (the lost
  answer offers no `request.retry`, so only the worker's `retry` sends it
  again). A new item's draft whose item exists after all is unmarked (no
  observation, source `""`) and reviewed, on the Edit route and after an
  own-item `occupied_path`. The unmarked base reaches the store once the
  review closes, as the review holds the draft's writes (R-44). — If
  wrong: one more read, or a recorded draft rewritten.
- Copy for the owner's on-screen check (Task 22): "The item is not created
  yet" (the Drafts page's state of a not-created draft); R-46's words
  above. A not-created tab shows no title, pill or chips, and D1's "not
  found" status, until Task 23 builds its header from the draft.
- Notes for Task 23: `new_item` returns `NewItem { draft, prepare }`; the
  tab can attach `draft` (`DraftRegistry::attach`), set `continuing` with
  `create: true` and enter `Preparing(prepare)`, and the editor opens by
  `open_not_created`. `NewItem` and `RECOVERY_NEEDED` live in the private
  `services::drafts::create` module: re-export them from
  `services::drafts`. Remove the three Task 23 allowances.
- Parked (Task 22), for the whole-branch review: a create answered
  `external_change` (an earlier attempt wrote the file and did not commit)
  opens the review like a save's, which then reads an item the index may
  not list; it is not tested. The tab's Retry on a refused create sends a
  new request (as a save's does), not the same one.
- Ruling (controller, Task 22 review M1, M2, M4, M5, M7): a restored
  not-created draft's Save, and its close dialog's Save, each create the
  item once, and a create sent again after a lost answer creates no second
  item, now each with a test; the save record names its `Verb`, and the
  context key is the Create prepare's. A kept create record leaves the
  session not saving, so a new Save and the operation area's Retry can both
  be in flight for one item; this is documented, not changed. — The library
  answers the second `already_applied` or `occupied_path`, and the new send
  drops the old record. — If wrong: a second create request shown in the
  operation area.
- Copy for the owner's on-screen check (Task 22 review): after a Create
  prepare answered `occupied_path` re-routes to Edit, the operation area
  still shows that first prepare as a failed request while the flow goes on.
- Parked (Task 22 review), for the whole-branch review:
  - a create answered `external_change` is likely unreachable: a crash
    after the write leaves a journal row, which answers `recovery_required`
    (fact 13), and fact 12 is a save's token rule. It stays untested;
  - the retitle test (`a_retitle_between_create_and_read_reviews_with_theirs_as_default`)
    checks the review's rule at the registry, not that the tab's Save
    opens the review;
  - the registry's map of Create-prepare worktrees is never pruned. Each
    Continue editing prepares again and overwrites its entry. A create
    committed on main is checked in the context, and falls back to the
    refusal; Done → Keep draft and reopening the tab routes it to Edit and
    the review;
  - `prepare_create` keeps its own `ItemKind` → `AuthoringKind` mapping,
    its only use;
  - 8 pre-existing tests ran over 60 s in the whole-binary run, at a load
    average of about 30, while another worktree's suite was running.
- Rulings (Task 23, pre-flight): the path words are "A document's path
  must be under docs.", "A document's file name must end in .md." and
  "Another file is already at {path}."; at Save, "Another file is already
  at {path}. The draft is kept."; `initials_required` says "This
  repository needs initials before it can create tickets." with an "Open
  Settings" button on the repository; `secondary-n` works in the
  navigation region only and does nothing under a dialog or the sheet; New
  document… sits beside New folder, New ticket… at the same place in the
  ticket list, so no existing Tab walk changes.
- Decisions (Task 23, implementer): the new item's primary branch is the
  selected entry's `configuration.primary_branch` from the repository list
  (D1's, as the sidebar shows it), never hard-coded. The new tab's header,
  strip title and sheet show a synthesized `ItemDto` of the draft's named
  fields (`shown_new_item`) while the never-created session lives or a
  Create prepare runs for a continued draft (Task 22's Continue editing
  included); actions and banners keep the real read; a stored or restored
  draft not being continued shows none. D2's restored section is hidden
  while that prepare runs. Occupied-path words wait until D2's own-item
  read is done. Open Settings is the `repo.identity_set` recovery's entry
  point (`identity_in_settings`), deferred to the next frame. A draft
  write failure with the store open says "The new item's draft could not
  be written, so nothing was created." and the store's error text. Two
  tests beyond the brief's eight: R-46 in the dialog, and a Create prepare
  answered before its tab opens. — If wrong: copy or a header rule to
  change; no data at risk.
- Copy for the owner's on-screen check (Task 23): "New document…", "New
  ticket…", the dialog titles "New document" and "New ticket", the labels
  "Title", "Type", "Folder", "File name", "Cancel" and "Create"; the ruled
  words above; the unruled draft-write words; the required words show at
  once on opening (empty Title), and the ".md" words while the File name
  is empty; the new tab's title, chips, pill and strip from the draft,
  with no not-found line; Open Settings landing on the identity form with
  focus in its name field; the dialog's layout.
- Parked (Task 23), for the whole-branch review:
  - the path check's stamp is redundant with the dropped task (its
    mutation survives), as Task 21's;
  - no test for a draft write failure with the store open;
  - the Folder field suggests no folders (`list_document_folders`);
  - while D2 reads an occupied path the tab shows no refusal at all, for a
    moment;
  - a Create prepare refused for the dialog's tab leaves it viewing with no
    title, D1's not-found line and D2's section, whose Continue editing
    prepares again;
  - `view/create.rs` is 481 lines after the split into `create/document.rs`
    and `create/form.rs`.
- Ruling (controller, Task 23 review I1, M1, M2, M4, M6 to M8): the
  stand-in item is not a file. A new item's sheet has no Inspect link and
  no Problems section, and while its Create prepare runs it says "Not
  created yet. These can be changed once its private copy is ready." with
  "Read only until then". The stand-in header stays until the tab's own read
  finds the item, so the title does not blank out after the create. A
  failed path check runs again at the next change of any field. A File
  name with nothing before `.md` is an empty one. The create dialog closes
  itself, and only while focus is inside it, because the Kit closes only
  the topmost dialog and does not expose its stack. — The design gives no
  words for a new item's sheet, and the stand-in reached the sheet's
  Inspect action. — If wrong: copy edits, or a dialog left open under
  another.
- Copy for the owner's on-screen check (Task 23 review): the sheet's "Not
  created yet. These can be changed once its private copy is ready." and
  "Read only until then"; the unruled draft-write words above; where focus
  lands after Create (the content region, not the editor), on native
  checklist step 9.
- Parked (Task 23 review), for the whole-branch review:
  - a repository with no primary branch, from an invalid configuration
    or a dropped entry, still offers New…: the draft is written with no
    branch, and the prepare is refused later, on a tab with no title;
  - keyboard tests stop at Title → Type. Tab through the remaining fields,
    Cancel and Create, and Enter on a focused button, are untested;
  - the dialog's inputs stay editable while a create runs, and edits made
    then are dropped;
  - if focus has left the dialog's own body when its item is created, the
    dialog stays open, with Create no longer offered, until Escape or
    Cancel. The tab still opens and nothing is lost;
  - the edit editor takes focus when it opens, even under a dialog opened
    above meanwhile;
  - `view/create.rs` is 454 lines after `Render` moved to
    `create/form.rs`.
- Part D checkpoint (controller): the gate at `afd93d4` passed (desktop
  986/0 in 682 s; `--lib drafts::` 55; `--test drafts` 48; clippy both
  ways, fmt, CLI isolation and `check --all-features` clean). The review
  approved with 9 Minor issues. Fix round 1 closed six of them (M1, M2,
  M3, M5 to M7, and M8's first two points), and its re-review found all
  eight points addressed, with two new Minor issues (N1, N2). Fix round 2
  closed those, and the controller checked its diff.
- Ruling (controller, Part D review M1): a save or create answered
  `already_applied` takes its key out of the created contexts, as a
  commit does. An earlier attempt committed, so the context holds the
  item's commit and its release would be refused. — If wrong: a context
  kept until a manual release (the Known Limits class).
- Ruling (controller, Part D review M2, M3, M7, and re-review N1, N2): a
  new item's stand-in shows from its draft until the tab's read first
  finds the item, whether its session lives or Done ended it, and never
  after. It shows the item as it was created, except while a live session
  is shown. While it shows, the pill has no provenance tooltip and the
  sheet has no provenance block and no closure sentence. After Done, the
  pill and the sheet say the private copy, where every create commits. —
  Contract 8: the tab says only what is true. — If wrong: a flash of
  "not found", or a header's words for a moment.
- Ruling (controller, Part D review M5): when a create is refused
  `occupied_path` and the check of that path cannot tell what is there,
  the tab shows the library's refusal (its message and code) without
  "Another file…". In the item's own context, `invalid_path` and
  `path_not_found` do tell: this draft's own item is not there, so the
  words stay. A busy or failed read, and every read of the root's copy,
  do not tell. — The words must not claim a file that may be the user's
  own. — If wrong: the words in an untested edge (below).
- Ruling (controller, Part D review M6): a Drafts-page discard while a
  new item's Create prepare runs closes its tab once the discard lands
  (R-71), and the context the prepare made is released (decision 18). —
  If wrong: a tab left open on "not found".
- Copy for the owner's on-screen check (Part D review): after Done → Save
  on a new item, the header keeps its title until the read lands, with no
  "not found" line; its pill says "Viewing your private copy"; an
  occupied path that could not be checked shows the library's refusal
  with no path words; a new ticket's emptied Project and a `closed`
  Status are saved as typed; a discard from the Drafts page while New
  ticket… prepares closes the new tab.
- Parked (Part D review), for the whole-branch review:
  - M4: a `TicketCreate` refused for its relationships shows only the
    generic refusal; practically unreachable (a fresh ID closes no
    cycle);
  - M8c: the retitle-then-review route is tested only at the registry;
  - M9: eight tests ran over 60 s in the whole binary under load, three
    of them Part D's;
  - M5's edge: an own item whose context the index does not hold yet
    would be called "another file"; no route to it was found;
  - untested: the pill's tooltip (the harness cannot read tooltips),
    M6's Loading branch, and a never-created session's end leaving no
    stand-in;
  - M1's limit: an `already_applied` save into a fresh context whose
    text equals the file it branched with keeps that clean context
    instead of releasing it (safe; contrived).
- Rulings (controller, Task 24 pre-flight): R-23's words are "Move is
  available while viewing, with no draft or save in progress." (a
  session, a draft or a save) and "Move waits while a private copy is
  removed." (a release pending). The Move dialog takes a Folder and a
  File name, the document's own until changed; Move is offered once the
  destination differs, passes `document_path` and `observe_path` finds
  nothing there, observed in the item's own context when its read names
  an active one, else on primary. A collision says Task 23's "Another
  file is already at {path}."; a stale source "This document changed
  since it was read. Close this and try again."; anything else the
  library's message and code; the dialog stays. On `external_change` the
  document is read again by its ID: at the destination the move is done
  and the dialog closes, otherwise the stale words. New folder's parent
  is the folder selected in the tree, else `docs`; its words are "A new
  folder stays on this computer until a document is saved in it.",
  "Folder name", "Folder exists" (a retry's `already_applied`) and "This
  is not a valid folder name." (empty, `/` or `\`, `.` or `..`, a leading
  `.`).
- Found (Task 24): the worker's retry of a move whose answer was lost at
  `BeforeRequestSettlement` answers `ok`, not `external_change`: the
  binding works around `01M4PB80T7` for a committed source. The ruled
  `external_change` branch is tested through another actor's move to the
  same destination with an uncommitted change.
- Decisions (Task 24, not ruled, for the whole-branch review): "The
  document is already at this path." when the destination is the current
  path; "New folder…" with an ellipsis (was "New folder"), the heading
  "New folder"; the Move dialog opens over the sheet, which stays; a move
  answer whose checkpoint committed (`discovery_pending`) closes the
  dialog as moved; after a refusal Move stays offered and reads afresh;
  a collision at Move marks the destination held; "Folder exists" keeps
  the dialog open with Create not offered for that name; Move… needs the
  document as read, so a new document's stand-in is never moved.
- Copy for the owner's on-screen check (Task 24): Move… to the right of
  the Folder box, its note under the row; the Move dialog's two fields,
  Cancel and Move; New folder… beside New document…, the folder dialog's
  explanation, field and path.
- Parked (Task 24): `has_session` in R-23's check has no test of its own
  (redundant with `Viewing` and `draft_kept`); a save in flight with no
  session in the tab has no UI route to test alone.
