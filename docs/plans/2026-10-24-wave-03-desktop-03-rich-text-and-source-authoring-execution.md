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
