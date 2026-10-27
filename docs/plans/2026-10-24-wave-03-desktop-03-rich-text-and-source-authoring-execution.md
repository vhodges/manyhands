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
| 1 | `feat: add context prepare` (this commit) | mutation_bridges `prepare_` → 16; mutation_replay / mutation_confirmation `keeps_the_rules_of_the_boundary` → 1 each; mutation_remote same filter → 2 SSH cases; mutation_contract `every_` → 5, `the_fixture_directories_are_exactly_the_registered_cases` → 1; lib `mutation::` → 107; clippy all-targets/all-features clean; fmt clean; CLI `help` (exit 2) and `--help` (exit 0) equal the Task 0 copies | complete, with one concern | `discovery_pending` of `context prepare` offers `request.retry`, not `operation.resume`: the row keeps step `worktree_observed`, which `operation resume` leaves to the original request; see the Task 1 report |

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
