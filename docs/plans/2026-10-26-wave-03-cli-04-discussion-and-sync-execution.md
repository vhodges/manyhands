---
title: "Wave 03 C4 CLI Discussion, Synchronization And Conflict Recovery Execution Ledger"
date: 2026-10-28
status: "in-progress"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M622ZNZKZRADVEK8P4QH10MX"
---

# Wave 03 C4 CLI Discussion, Synchronization And Conflict Recovery Execution Ledger

The plan's record of execution. Task 0 created it, and every task commits it.
Facts, rulings (R-N), named edits (E-N) and decisions are the numbers of the
[Cycle](../Cycles/wave-03-cli-04-discussion-and-sync.md) and the
[design](2026-10-26-wave-03-cli-04-discussion-and-sync-design.md). The
[plan](2026-10-26-wave-03-cli-04-discussion-and-sync-implementation.md) names
the tasks. The filename keeps the approved plan's date. Task 0 ran on
2026-10-28 UTC.

## Baseline

Task 0 ran in the ticket worktree
`/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M4CC0VMWEQQ9G4W0DAG7ADPY`,
on `manyhands/ticket/01M4CC0VMWEQQ9G4W0DAG7ADPY`. No subagent was dispatched.

### The base (Step 1)

- **HEAD** at the start: `a352a3708856c6da3753d8d66dc3509e18bfeb34`
  (`docs: record owner approval of the C4 plan`). The owner approved the
  Cycle, the design and the plan on 2026-10-28, at `8a94e9b`.
- **`origin/main`**: `a372edbef143f3a9b7201f2d040bf82d34e087e2`
  (`docs: close closure recovery gate ticket`). **Main did not move** past
  `a372edb`, the commit the design was re-grounded on (planning merge
  `e920043`).
- `git fetch origin` printed nothing. `git merge origin/main` answered
  `Already up to date.`, and the merge base is `a372edb`.
- `git merge-tree --write-tree HEAD origin/main` exited 0 with tree
  `a0f551c14432513a0fbfdac86a4bb1c7c17ed704`: no conflict.
- `git diff origin/main HEAD` over `src`, `tests`, `schemas`, `Cargo.toml`,
  `Cargo.lock`, `scripts`, `.github` and `devenv.nix` is empty. The branch
  differs from main in documents, ticket files and comments only. So the source
  of this task is the source the 2026-10-27 re-grounding read.
- **Gate ticket (decision 2).** `git show
  origin/main:.manyhands/tickets/01M57SKVF6TQ3QM6AXJ35JGGSY/ticket.md` has
  `status: "closed"` and `closed_at: 2026-10-27T12:51:51Z` (closed by Vince
  Hodges). The gate stands merged and closed.
- **`AGENTS.md`**: `git show origin/main:AGENTS.md` is identical to the
  worktree's copy. Its rules for this task are the Devenv `--locked`
  commands, the before-submission gate, the merge-not-rebase rule and the
  native CI rule (after merge, on main, by hand, with the owner's approval).

### Known-limit tickets and D3

Ticket files reach `origin/main` only when their Cycle merges, closed in its
final push. So each ticket below is "not on main, so open". Its branch head
and `closed_at` were read from `origin/manyhands/ticket/<id>`.

| Ticket | On `origin/main` | Branch head | `closed_at` on the branch | Status text |
| --- | --- | --- | --- | --- |
| `01M4H33R34Z7C7EEKTY1ZCT950` (abandon a pending conflict, decision 6) | no, so open | `d4ca00e6` | none | open |
| `01M4R19PC0T6ZPKM39X9DF26HX` (blocked sync holds its reservation) | no, so open | `ec2dccb5` | none | open |
| `01M5MK57JKCSQFXBNZ8DBE548K` (unusable Git identity refused late) | no, so open | `b3deae03` | none | open |
| `01M5E6DYJ6GN7DMW4424TH3V78` (wrong passphrase is `transport_unavailable`) | no, so open | `c9bbc632` | none | open |
| `01M4W5SCBV6WFJP11Y7PJF5VS8` (operation reads name no item) | no, so open | `d1326adf` | none | open |
| `01M4PB7YZMBFAFFM94AZY08HWS` (interrupted save closed as `rejected`) | no, so open | `c7af9c50` | none | open |
| **D3** `01M4CC0VN0MFTGKQ4HGTTC5H63` (rich-text and source authoring) | no, so open | `d4631632` | none | open |

No known limit changed. **D3 is not closed on main**, so it has not merged.
If C4 merges first, D3 resolves the merge of `src/desktop/view/confirm.rs`; if
D3 merges first, C4 does (decision 10). Task 5 adds C4's one arm there.

### Baseline checks (Step 4, entry-gate item 7)

Run on the merged main (tree identical to `a372edb`), each as `devenv shell --
cargo <command> --locked` with a 600-second tool timeout.

| Check | Result | Time |
| --- | --- | --- |
| `cargo check --all-features --locked` | passed | 2 min 29 s |
| `cargo fmt --check` | passed | 15 s |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed | 3 min 24 s |
| `cargo check --locked --features desktop` | passed | 2 min 43 s |
| `cargo clippy --all-targets --locked --features desktop -- -D warnings` | **failed**, exit 101: 58 errors, all in `bin "manyhands" test` | 1 min 26 s |
| supplementary: `cargo clippy --locked --features desktop --bin manyhands -- -D warnings` | passed | 15 s |

**The failing check is a baseline failure of main, outside C4's files.** The
58 errors are `dead_code` and `unused_imports` in the `manyhands` binary's
*test* target when only `--features desktop` is on: 19 unused functions, 27
reports of unused methods (one to six each), 3 constants, 2 structs, 2 enums, 2
unused imports, and one each of an unused field, an unused associated function
and an enum with unconstructed variants. They sit in
`src/desktop/test_support/{fixtures,drafts,delaying}.rs` (31 of the
diagnostics), `src/desktop/services/drafts*.rs`, `src/desktop/model/{drafts,operations}.rs`,
`src/desktop/view/*` (the re-exports `draft::NOT_SAVED` and `focus::REGIONS`)
and `src/desktop/mod.rs`. Those test-support items appear to be used only by
tests that need the `desktop-test-support` feature, which `--all-features`
turns on (not traced item by item). The
last commit to `src/desktop/test_support` is `2710f12c` (2026-10-24). The
non-test build and `cargo clippy --locked --features desktop --bin manyhands`
are clean, and C2's ledger recorded that form as its desktop clippy gate
(`2026-10-20-wave-03-cli-02-repository-and-credentials-execution.md:1321`).

The plan's Global Constraints, Task 5 and the checkpoint gate (Tasks 6 and 11)
spell the desktop clippy command with `--all-targets`. As written it cannot pass
on this main, whatever C4 does. **It was not fixed here, and no plan text was
changed.** It is a finding for the controller (see "Findings awaiting a
ruling").

**The full suite was not run (ruling P1).** Per-target counts are therefore not
collected. The full `cargo test --all-features --locked` runs once, in Task 15,
detached, with a two-hour budget.

### Fact 81's red test (Step 2)

`devenv shell -- cargo test --all-features --locked --test repository_enablement
registry_creates_repository_and_discovery_metadata_schema` ran at `a352a37`.

- **Result: FAILED, as expected.** 0 passed, 1 failed, 94 filtered out; it
  panicked at `tests/repository_enablement.rs:581` on `assertion left == right`.
- The actual list has 48 tables, the expected list 40. A sorted diff of the two
  lists shows **exactly the gate's eight tables and nothing else**:
  `local_context_sync_conflicts`, `local_context_sync_execution`,
  `local_context_sync_images`, `local_context_sync_records`,
  `local_context_sync_request_control`, `local_context_sync_resolution_cancellation`,
  `local_context_sync_resolutions` and `local_context_sync_steps`.
- So fact 81 stands, and main has not fixed it. **E-12 in Task 4a fixes it**
  (Cycle decision 21). No other table differs, so nothing stops for the owner.

### Controller rulings

Recorded as given to Task 0.

- **Ruling P1.** Step 4 runs only the five baseline checks above. The full
  `cargo test --all-features --locked` suite is deferred to Task 15, where the
  plan's Global Constraints run it once. The owner's standing rule is that a
  task that lists a broad baseline gate runs only the relevant tests. Fact 81's
  single test still ran, in Step 2. Its failure for exactly the gate's eight
  `local_context_sync_*` tables is expected (decision 21) and is recorded, not
  fixed. A failure for any other table would be BLOCKED.
- **Ruling P4.** The implementer writes each task's completion comment as a
  canonical file under `.manyhands/comments/01M4CC0VMWEQQ9G4W0DAG7ADPY/<ULID>.md`,
  with the ULID from `id new`, `created_by` `Claude Opus 5.5
  <noreply@anthropic.com>` and `created_at` from the UTC clock. It is committed
  with the task and the ticket branch is pushed. The comment says the task
  review is pending. A fix round appends to this ledger, not to a new comment.
  Task 0's comment is `01M622X59YKBQV7HGK4H0G9NZA`, a reply to
  `01M621C24Z76N2M10M12F762CT`.

### Findings awaiting a ruling

1. **The desktop-alone clippy command fails on main** (above). The passing
   form is `cargo clippy --locked --features desktop --bin manyhands -- -D
   warnings`. The controller rules which form Task 5 and the checkpoint gates
   use, or whether the leak is fixed first. The leak is outside C4's files, and
   `src/desktop` is D-track code.
2. **Fact 10, what `operation abandon` answers for a comment.** Answered by the
   comment operation's *local* row, not by its publication (details under
   "Confirmed facts").
   - A committed comment whose publication is pending and whose local row has
     completed: the preview is `already_applied`. Nothing is closed.
   - The child synchronization's own ID: `invalid_input`.
   - A comment whose local row is **still pending** (stopped mid-checkpoint, an
     index hand-off owed, or waiting for registration) is abandonable. The
     row closes as `abandoned` and the binding is kept but spent, so replay,
     publication-only retry and cancellation refuse it
     (`recovery.rs:201`). A commit already made stays on the context branch,
     unpublished until an ordinary `item sync`. This is existing behavior,
     pinned by `tests/mutation_confirmation/abandon.rs:1273`, and needs the
     user's confirmation.
   The plan says to stop for a ruling if abandon "would close the comment's
   binding while its checkpoint stays unpublished". The first case, which is
   the one the plan words, does not. The last case does, but only for a local
   row that is still pending, and it was not held back here. The controller
   confirms before Task 2b that this is acceptable, or rules otherwise.
3. **Three recovery-offer hits are not named by E-5.** `outcome_tests.rs:1519`
   (in `a_resume_is_never_offered_again_for_what_only_the_original_request_can_do`),
   and `:2216`, `:2223` (in
   `a_cancellation_is_not_offered_for_a_reservation_that_is_not_the_request_s_own`)
   test recovery offers keyed on the code `merge_required`, not on a binding's
   refusal. They need no edit: the code stays defined and
   `never_finished_by_the_same_request` does not change. They are recorded here
   because they fit none of the plan's four classes.
4. **A watch item for Task 4b, not a finding.** `tests/mutation_confirmation/abandon.rs:1273`
   (`a_comment_that_waits_for_its_registration_is_shown_refused_a_resume_and_abandoned`)
   asserts that `operation resume` on a comment whose *local row* waits for
   registration answers `recovery_required` and offers `operation.abandon`.
   R-5 and R-20 let resume take a comment's operation ID. The new route must
   keep that answer for a pending local row, or the test becomes an unlisted
   pre-existing-test edit and stops for owner review.

## Confirmed facts

**Main did not move, so the facts stand as the 2026-10-27 re-grounding
recorded them.** Step 1 says so, and Step 2 re-checked fact 81's red test.
The tree is byte-identical to `a372edb` in every source directory (Step 1).
The re-checks below were run in addition, because Step 2 names them.

| Facts | Status at `a352a37` |
| --- | --- |
| 1–16, the mutation boundary | stand; not re-read (no source change). Fact 10 re-read below |
| 17–23, comments | stand |
| 24–37, synchronization and conflicts | stand. Fact 28 re-read below |
| 38–48, the CLI | stand |
| 49–52, inventories | stand. Fact 49's inventory greps were re-run in Step 3: 311 hits, as the re-grounding found. `HOMES` has 32 entries (`tests/support/conformance.rs:59`) and `DATA_ROOTS` 50 (`tests/mutation_contract.rs:1510`) |
| 53, the gate | stands: `git diff origin/main HEAD` is empty under `src`, so `local_sync/`, `integration/`, `mutation/local_control.rs` and `bind/local_sync.rs` are unchanged since the re-grounding (facts 25, 53 and 80) |
| 54–66, 67–74, 75–79 | stand |
| 80, an unfinished local sync | stands (the files it cites are unchanged) |
| **81, the red registry test** | **confirmed**, re-run: see "Baseline" |
| 82–85 | stand |
| 86, **(confirm)** | not Task 0's: Task 3's `resolution_prepared` row proves it, and a different answer stops there |

No fact changed, so no task changed and no ruling is needed on that account.

### Fact 28, confirmed

`set_local_identity` is still not refused by a pending operation.
`src/repository/bridges/identity.rs:44` says "**A pending operation does not
refuse it.** An operation that stopped for want of an identity is taken up again
once there is one, so nothing here asks the journal anything", and the function
takes only the repository lease (`repository_lease(...)` at `:717`; `:703` is
the function). The only mentions of the journal in the file are its module
documentation (`:40`, `:46`); `pending_rows`, `require_no_pending`,
`operation_records` and `local_row` do not appear.

### Fact 10, re-read for a comment operation

`abandon_operation` (`src/repository/bridges/abandon.rs:432`) and the binding
(`src/repository/mutation/bind/abandon.rs`) answer by the operation's **local
row** (`LocalRow::standing`, `recovery.rs:1618`: pending when the state is not
`completed`, or when it waits for registration):

| Operation ID given | Standing | `operation abandon` answers |
| --- | --- | --- |
| The comment's operation, checkpoint committed, local row `completed`, publication pending in the child | Completed | preview `already_applied`, exit 0 (`bind/abandon.rs:410-413`). The binding is untouched |
| The child synchronization's ID | held by the remote journal | `invalid_input` (`bridges/abandon.rs` `held_by_another_journal`; `bind/abandon.rs:280-282,405-407`) |
| The comment's operation, local row still pending: stopped before its checkpoint completed, an index hand-off owed, or `comment_registration_pending` | Pending | abandonable with confirmation. `abandon_row` closes it as `abandoned` and releases its request. The binding row is **kept and spent**: replay, publication-only retry and cancellation refuse it with `OperationAbandoned` (`recovery.rs:201-241`). The preview shows `committed` and the differing paths |

The third row is pinned by `tests/mutation_confirmation/abandon.rs:1273`, which
abandons a `comment_registration_pending` comment, checks the body is only in
its file and commit, and then runs `item sync` past it. Whether it needs a
ruling is "Findings awaiting a ruling", item 2.

### The named-edit classification (Step 3)

The three greps were run on `a352a37` exactly as the plan writes them.

| Grep | Hits | The re-grounding on `a372edb` |
| --- | --- | --- |
| 1 (refusal to merge, comment synchronization) | 56 | no gate-added hits |
| 2 (the C4 verbs in the CLI, files only) | 4 files | no gate-added hits |
| 3 (the closed inventories and DTO matches) | **311** | 311 |

**Grep 2: four files.** `src/bin/manyhands-cli/catalog.rs` is production code
(Tasks 7–10 flip the entries). `src/bin/manyhands-cli/catalog_tests.rs` is E-2.
`tests/fixtures/cli_v1/help_root.json` and `help_root_text.txt` are E-1. No file
under `tests/cli`, `tests/cli_remote`, `tests/cli.rs` or `tests/support/cli.rs`
matches.

**Grep 1: 56 hits.**

| Class | Hits | Where |
| --- | --- | --- |
| E-3, binding-level (edited) | 16 | `tests/mutation_remote.rs`: the registrations `:94-95`, `:196-197`, `:200-201`; the sync tests `:1324`, `:1339`, `:1344`, `:2695`, `:2697`, `:2716`, `:2750`, `:2754`, `:2763`; the resume test `:3628` |
| E-3, domain-level (unchanged) | 11 | `tests/mutation_remote.rs`: `REFUSING` `:365-366`; its uses `:456`, `:475`, `:507-508`, `:541`, `:546`, `:577`, `:621`, `:3014` |
| E-4 | 2 | `tests/fixtures/mutation_v1/repo_sync_merge_required.json:2`, `tests/support/mutation_golden.rs:143` |
| E-5 (the named test) | 3 | `src/repository/mutation/outcome_tests.rs:669`, `:672`, `:697` (table and count of `every_synchronization_error_has_its_code`; the `MergeRequired` row itself is unchanged) |
| Unchanged: recovery-offer tests keyed on the code | 6 | `outcome_tests.rs:1584`, `:1606` (in `:1565`), `:1659` (in `:1633`), which E-5 names as unchanged; and `:1519`, `:2216`, `:2223`, which it does not (Findings, item 3) |
| Unchanged: an exhaustive match over `SynchronizationError` | 1 | `tests/comment_publication.rs:971`: the error enum gains no variant |
| Comment text only | 1 | `tests/mutation_remote.rs:1144`, the doc of `Stopped::retry` |
| Production code a task changes or leaves | 16 | `bind/resume.rs` (11: `LeftToInspect`, `is_comment_synchronization`; Tasks 4b, 5), `outcome.rs:212`, `:939`, `:1004` (`:212`'s comment at `:215-217` is stale after Task 1), `bind/sync.rs:7`, `:102` (Task 1) |
| **Total** | **56** | |

**E-3, test by test.** A binding-level test reaches the binding's `SETTINGS`
through `Mutation::RepoSync` or `OperationResume`; a domain-level test calls
`synchronize_remote_observed` with its own `REFUSING` through `observed(...)`.

| Test | Line | Reaches | Class |
| --- | --- | --- | --- |
| `a_divergent_remote_is_merge_required_and_holds_its_reservation` | `:1324` | `repo_sync` through `execute` | binding-level, edited (also E-4 golden) |
| `a_lost_push_the_remote_has_since_built_on_is_merge_required_and_leaves_nothing_of_a_merge` | `:2697` | `repo_sync` | binding-level, edited |
| `merge_required_is_not_finished_by_the_same_request_after_the_branch_is_rebased` | `:2754` | `repo_sync` | binding-level, edited |
| `a_resume_never_merges_and_leaves_nothing_of_a_merge` | `:3614` | `resume(...)`, which shares `SETTINGS` | binding-level, edited |
| `a_resume_does_not_continue_a_merge_made_some_other_way` | `:3654` | `resume(...)`; its setup uses the domain default | binding-level, edited |
| `the_sibling_refuses_divergence_and_the_existing_function_merges_the_same_fixture` | `:493` | `observed(... REFUSING ...)` | domain-level, unchanged |
| `the_sibling_does_not_continue_a_merge_it_is_set_to_refuse` | `:590` | `observed(... REFUSING ...)` | domain-level, unchanged |
| `the_sibling_reports_each_safe_point_in_order` | `:447` | `observed(... REFUSING ...)` | domain-level, unchanged |
| `an_observer_stops_a_synchronization_at_the_safe_point_it_asks_at` | `:564` | `observed(... REFUSING ...)` | domain-level, unchanged |
| `a_row_cancelled_as_its_push_returns_says_the_push_returned` | `:3002` | `observed(... REFUSING ...)` | domain-level, unchanged |

The last three tests use `REFUSING` too, so they are domain-level. The design
lists only the first two domain-level examples, with "such as".

**E-11.** The plan's grep 1 matches `LeftToInspect` and
`is_comment_synchronization` only in production code (`bind/resume.rs`). The
named test, `a_resume_of_a_comment_s_synchronization_is_not_taken_up_and_is_not_offered`,
is at `tests/mutation_replay/resume.rs:1090`; it was found by name. No unit
test under `src/repository/mutation/bind/` mentions a comment
(`before_tests.rs`, `enabled_tests.rs` and `save.rs`'s `tests`), so there is
none to edit there. Task 4b's watch item is in "Findings awaiting a ruling",
item 4.

**Grep 3: 311 hits.**

| Class | Hits | Where |
| --- | --- | --- |
| Production code a task changes or leaves | 104 | `human_admin.rs` 34 (Task 5's exhaustive matches at `:55` and `:77`), `src/desktop/view/confirm.rs` 10 (Task 5's one arm, `:493-569`), `bind/*.rs` 56 (constructions of `MutationDataDto`, unchanged except the new bindings), `mutation/{mod,reply,replay}.rs` 4 |
| E-10 (data accessors) | 70 | `tests/support/mutation.rs` 60 (`:598-791`), `tests/support/repair.rs` 10 (`:335-345`) |
| E-6 | 8 | `src/repository/mutation/command.rs`'s tests, `:497`, `:538`, `:575`, `:591`, `:615`, `:647`, `:667`, `:683` (`:538` and `:615` are the named edits; `:591`'s list gains `ItemRepublish`) |
| E-7 | 3 | `tests/mutation_contract.rs:1510` (`DATA_ROOTS`), `:1678`, `:1717` |
| E-8 | 2 | `tests/read_contract.rs:704`, `:1053` |
| E-9 | 1 | `tests/support/conformance.rs:59` (`HOMES`) |
| E-5 | 1 | `outcome_tests.rs:1537`, the named test |
| Loops over an inventory that takes new entries unchanged | 23 | `outcome_tests.rs` 5 (`:1374`, `:1406`, `:1651`, `:1819`, `:2259`); `mutation_contract.rs` 6 (`:1502`, `:1626`, `:1671`, `:1728`, `:1754`, `:1767`); `read_contract.rs:979`; `tests/support/conformance.rs` 4 (`:26`, `:357`, `:363`, `:365`); the five `bound_commands()` loops (`mutation_confirmation/conformance.rs:143`, `mutation_bridges/conformance.rs:149`, `mutation_replay/conformance.rs:21`, `mutation_remote.rs:4533`, `mutation_remote/host.rs:1490`); `human_admin_tests.rs:106`, `:110` (`PreviewEffect::ALL`, which `every_details_shape_renders_its_fields` does not count) |
| Single-variant matches and constructions, with a fallback | 99 | `tests/mutation_confirmation.rs` 48 (four `other => panic!` matches at `:54`, `:3035`, `:3590`, `:4318`; the rest are `PreviewEffect::X` values), `mutation_confirmation/abandon.rs` 4 (`:247`, `:825` and two effect values), `mutation_contract.rs:1050`, `:1110`, `tests/mutation_bridges/*` 20, `tests/mutation_replay/local_sync*.rs` 7 (let-else), `tests/mutation_remote.rs:1441` and `mutation_remote/host.rs:279`, `:332` (`_ => Err(FixtureError)`), `mutate_tests.rs:157`, `human_admin_tests.rs` 14 |
| **Total** | **311** | |

Every single-variant match was read and has a fallback; the three wildcard
matches the plan names (`tests/mutation_remote.rs:1441`,
`tests/mutation_remote/host.rs:332`, `tests/mutation_confirmation/abandon.rs:825`)
are among them. No hit stops for an owner ruling. The only hits outside the
plan's four classes are the three recovery-offer tests above, which need no edit.

## Audit refresh

Step 5 is done in `docs/research/wave-03-api-audit.md`:

- a section, **"Wave 03 CLI Cycle C4 refresh (2026-10-28)"**, citing decision
  16, placed before C3's;
- under it, `#### C4 signatures, inputs and result-set key`, with the binding,
  input type, confirmation and data of each verb, the code groups, and each
  verb's code set;
- the seven C4 rows of the CLI inventory updated **in place**, and one new row,
  `item republish`. The stale "Missing public sync", "Missing generic
  deliberate resume", "Missing public conflict … read" and "pending-Wave-02"
  claims are gone from them, and the C4 checklist section has a dated
  amendment. The inventory still has one row per verb;
- under it, `#### C4 exit-coverage union`: **48 library codes** in one fenced
  `text` block. The per-verb counts are `comment add` 41, `item sync` 37,
  `repo sync` 37, `item republish` 42, `conflict show` 13, `conflict resolve`
  40, `operation resume` 40 and `operation abandon` 23. With the CLI-only
  `usage_error` and `invalid_id` the union is 50.

Three deliberate exclusions, recorded in the audit: `merge_required` (no C4
binding returns it, R-16; E-7 accounts for it), `registration_pending` (no
designed outcome returns it) and the host-approval, request-read and key codes.
Task 13 asserts set equality with this block (R-27).

## Cycle decisions 1–21, rulings R-1 to R-41 and named edits E-1 to E-12

### Product-owner decisions

1. **The library part is in C4.** The `comment add`, `conflict show` and
   `conflict resolve` bindings land in C4's branch, an approved exception to
   track rule 5. Decisions 3, 4, 7, 9, 11, 13 and 18 extend it.
2. **The gate lands first.** C4's implementation starts after
   `01M57SKVF6TQ3QM6AXJ35JGGSY` merges, and Task 0 re-grounds on that main. It
   merged on 2026-10-27 at `a372edb`.
3. **Divergent synchronization merges.** `item sync` and `repo sync` merge as the
   Git RFC describes. A canonical conflict is resolved with `conflict show` and
   `conflict resolve`. Any other conflict gets outside-repair guidance and then
   `operation resume`.
4. **Republish consent is added in C4,** through prepare and confirm.
5. **D4 waits for all of C4.** Part A is not merged to main early. The Wave
   records the ordering under track rule 2.
6. **Abandoning a pending remote conflict stays on its ticket,**
   `01M4H33R34Z7C7EEKTY1ZCT950`. C4 records the block as a known limit.
7. **Republish is its own confirmed verb, `item republish`.** Confirmation is
   fixed per command, so `item sync` cannot ask for it only when a branch was
   deleted.
8. **The next step is typed data.** A result that stops on a conflict keeps the
   code `recovery_required` and carries `conflict: {operation_id, next_action}`.
   The recovery-action enum is unchanged.
9. **`comment add` takes an optional `created_by`,** validated as
   `Name <email>` and written once. Agents pass their noreply identity; people
   omit it and get their Git identity.
10. **One desktop arm for the republish preview,** in
    `src/desktop/view/confirm.rs`. Whichever of C4 or D3 merges second resolves
    the merge.
11. **Republish pushes exactly the previewed commit,** unchanged, and integrates
    nothing. The next `item sync` integrates primary.
12. **A per-path kind, with the set-wide rule kept.** `conflict show` reports
    each path's kind and structure. Resolution stays all-or-nothing, so a mixed
    conflict is repaired outside as a whole (a known limit).
13. **The same request finishes an interrupted resolution,** before any live
    token is checked, with the same commit and no second one. The library keeps
    what it needs, never a body. Task 4a stops if a row cannot be met.
14. **A Ctrl-C handler, with a spike first.** A handler with `SA_RESTART` sets
    the token and a second Ctrl-C ends the process. Task 7 starts with a spike.
    A Ctrl-C during blocked I/O may answer `transport_unavailable` with its
    effects intact, recorded as a limit. Anything worse stops.
15. **The comment exception is extended.** `working-on-a-ticket` and
    `scripts/ticket-loop` keep canonical comment files, with the noreply
    `created_by`, until that skill is revised. Everything else switches to
    `comment add`.
16. **The audit timing is accepted as a departure.** The design's Baseline Facts
    serve as the planning audit for entry-gate item 3, and Task 0 updates the
    audit's rows in place.
17. **`item republish` refuses a lifecycle-closed ticket** with `ticket_closed`,
    exit 3.
18. **A way out of a stuck resolution attempt.** `operation resume` finishes an
    attempt whose candidate commit is prepared, from durable state alone. A new
    `conflict resolve` request may supersede an attempt that has only written
    paths, if those paths still hold exactly what it wrote.
19. **The local verbs stay with C5.** `conflict show` and `conflict resolve` take
    remote synchronization IDs only (a local-sync ID is `invalid_input`);
    `operation resume` and `operation abandon` pass a local sync through to the
    gate's behavior. C4 amends the gate's sentences to name C5/D5.
20. **Decision 18's supersede stays for remote.** The gate's local resolution lets
    only the original request continue. C4 scopes the gate's sentences to the
    local operation; resume on a remote attempt with only written paths answers
    `original_request_required`, naming the request.
21. **C4 fixes main's red registry test.** E-12 adds the gate's eight
    `local_context_sync_*` tables and `remote_resolution_owners` to
    `registry_creates_repository_and_discovery_metadata_schema`'s list. Task 0
    records the failure as expected, and Task 4a proves it fixed.

### Engineering rulings R-1 to R-41

1. **R-1 Catalog.** Literal `Available("C4")` entries, flipped in the task that
   dispatches each verb, plus the new `item republish` entry.
2. **R-2 Kind lookup.** `--id` verbs resolve the kind with one `show_item` read,
   whose failure passes through as C3's fill read does.
3. **R-3 Synchronizer-driven submission.** A crate-private submission and retry
   take a synchronizer closure over `call.session.synchronize`; `Watch` gets a
   `pub(super)` constructor; the checkpoint goes through
   `checkpoint_comment_locally_with`; the public functions keep their behavior.
4. **R-4 No new fields on literal-built public structs.** New behavior enters
   through crate-private parameters.
5. **R-5 The conflict's operation ID.** `conflict show`, `conflict resolve` and
   `operation resume` accept the synchronization's ID, or a comment's operation
   ID resolved through its binding. Data reports the synchronization's ID.
6. **R-6 Comment codes.** As the comment-submission table; with no remote the
   code is `ok` with publication pending.
7. **R-7 `created_by` validation.** `Name <email>`, at most 256 bytes, each part
   meeting `ValidIdentity::of`, no `<` or `>`, written verbatim; not
   authenticated.
8. **R-8 Conflict data.** `ConflictNextDto` in three tagged DTOs; eligibility
   from one inspection after the conflict.
9. **R-9 String tokens.** `conflict-v1:` and `conflict-path-v1:` BLAKE3 digests,
   domain-separated and length-delimited, re-derived from the live conflict on
   every call and never stored.
10. **R-10 Sides inline.** `conflict show` returns sides in its data, only for
    paths the library reads as UTF-8 canonical text.
11. **R-11 Display paths.** C4's read escapes each byte outside printable ASCII
    as `\xNN` and doubles a backslash, as the gate's private `display_path` does.
12. **R-12 `conflict show` is a read.** No request ID, writes nothing, takes the
    repository lease, and a held lease answers `busy`.
13. **R-13 States.** As the conflict-states table.
14. **R-14 The attempt and its durable state.** A derived attempt ID; the library
    finishes an attempt at `ResolutionPrepared`, `CommitPrepared` or `Applied`
    from durable rows, storing no body. The interrupted-resolution table is the
    contract. The largest library change in C4; Task 4a stops if a row cannot be
    met.
15. **R-15 Continuation.** `conflict resolve` publishes under its own request,
    with progress and cancellation.
16. **R-16 Merging settings.** `refuse_divergence: false` in both bindings;
    `merge_required` becomes unreachable from them.
17. **R-17 Identity during a merge.** No workaround; `repo identity-set` is not
    refused by the stopped merge and the same request restarts and merges.
18. **R-18 Republish.** A push-only domain path through a crate-private
    `Session::republish`: fetch, prove the absence, refuse an advertised context
    or a moved local tip as `external_change`, push exactly `local_oid`.
19. **R-19 Republish preview.** `ItemRepublishPreviewDto`, the effect
    `branch_republished` and one desktop arm.
20. **R-20 Resume routes.** As the resume table; a remote attempt with only
    written paths answers `original_request_required`. Resume can now publish a
    comment.
21. **R-21 The Ctrl-C window.** Installed by the `Boundary` impl around `execute`
    of the six stoppable verbs, with `SA_RESTART`. The first Ctrl-C cancels, and
    a second ends the process.
22. **R-22 Cancellation's exit.** 130 even after a checkpoint.
23. **R-23 Retry.** C2's bounded retry is unchanged; it never follows
    `cancelled`, a blocked result or a latched Ctrl-C.
24. **R-24 Two-clone support.** `tests/support/cli_pair.rs`, mounted only from
    `tests/cli_remote.rs`; cases split to stay under the watchdog.
25. **R-25 Lost stdout.** Proven with a closed stdout pipe on `item sync` and
    `comment add`: exit 1, then `request show` and a replay.
26. **R-26 Dogfooding.** As the Dogfooding section, with the owner's go-ahead at
    that step.
27. **R-27 Exit accounting.** Every exit reached through the binary, and the
    unreachable codes listed with reasons; set equality with Task 0's audit
    union.
28. **R-28 Golden names.** `<resource>_<verb>_<ok|code|text>` for CLI goldens, the
    existing pattern for mutation goldens, `item_republish_preview` and
    `_noop`, and `conflict_show_<state>`.
29. **R-29 No new test target.** New tests are modules of existing targets.
30. **R-30 C3's stuck save.** `operation abandon` is shown closing C3's
    interrupted-and-edited save.
31. **R-31 SSH case names.** `c4_<module>_<case>`, registered in `CASES`, run with
    `-- c4_<module>`; each verification step checks the number of cases run.
32. **R-32 Predicates.** As the Command Predicates table.
33. **R-33 Fixture premise and gate alignment.** `CliPair` seeds the item on
    primary before the clones; a well-formed foreign token is `external_change`.
34. **R-34 Comment standing.** A binding that wrote nothing is released with the
    row's rejection and the record stays; a written file keeps the binding; a
    pending publication is `Owed`, except a final child or a code never
    finished by the same request.
35. **R-35 Resolution association.** `conflict resolve` holds no operation, so
    the durable state is the attempt, in a new table. A retry under that request
    ID continues it; any other request meets step 3's rules.
36. **R-36 One Ctrl-C latch per run.** The press count and cancellation persist
    across C2's bounded retry, and the retry stops once a press is latched.
37. **R-37 Local-sync operations in C4's verbs.** `conflict show` and `conflict
    resolve` answer `invalid_input`; `operation resume` and `operation abandon`
    pass a local sync to the gate; the CLI renders `LocalContextSyncMutationDto`.
38. **R-38 An unfinished local sync.** Every C4 verb that reserves, begins a
    comment or re-acquires a conflict answers `recovery_required` with
    `[repo.inspect]`, and writes nothing.
39. **R-39 `historical`.** `ConflictDto.historical` is true when an attempt has
    written paths.
40. **R-40 Incompatible closures.** `structure` gains `incompatible_closures`; the
    set is external-only and `next_action` is `external_repair`.
41. **R-41 The association table's name and shape.** `remote_resolution_owners`,
    holding IDs and a digest with no foreign key, made by its own migration step.

### Named edits E-1 to E-12

| Edit | Task | The change |
| --- | --- | --- |
| E-1 | 7–10 | `tests/fixtures/cli_v1/help_root.json` and `help_root_text.txt` regenerated as each C4 verb becomes available, and for `item republish`'s new row |
| E-2 | 10 | `catalog_tests.rs`: `DESIGN`'s C4 row gains `"item republish"` |
| E-3 | 1 | `tests/mutation_remote.rs`: the five binding-level tests that assert a refusal to merge are rewritten (classified above) |
| E-4 | 1 | `repo_sync_merge_required.json` replaced by `repo_sync_recovery_required.json`, and its registration renamed |
| E-5 | 1, 5 | `outcome_tests.rs`: `every_synchronization_error_has_its_code` for the conflict rows, and `an_external_change_to_a_confirmed_command_is_offered_a_new_preview_or_an_inspection` |
| E-6 | 2b, 4a, 5 | `command.rs`'s tests gain the three commands; two are named, and the first is renamed `the_stopping_commands_and_their_stages_and_confirmation` |
| E-7 | 1–5 | `tests/mutation_contract.rs`: `DATA_ROOTS` and the two `CODES_*` lists gain the new entries, with `merge_required`'s reason |
| E-8 | 1–5 | `tests/read_contract.rs`: `contract_enumerations` gains the new enums |
| E-9 | 2b, 4a, 5 | `tests/support/conformance.rs`'s `HOMES` and each home's table, and `confirmation.rs`'s `preview_schema` |
| E-10 | 1, 2b, 4a | The data accessors in `tests/support/mutation.rs` and `tests/support/repair.rs` gain arms for the new `MutationDataDto` variants |
| E-11 | 4b | The resume tests that assert a comment's synchronization is left to inspect change to publication (R-20) |
| E-12 | 4a | `registry_creates_repository_and_discovery_metadata_schema`'s exact table list gains the gate's eight tables and `remote_resolution_owners` |

Task 0's proof that these are the only named edits is "The named-edit
classification" above.

## Owed native checks

Linux is the only platform checked before merge. Native CI runs on main after
merge, by hand, with the owner's approval of that run (AGENTS.md); it is not a
gate for this Cycle.

1. **The Windows Ctrl-C check, owed after merge** (Cycle, Native evidence;
   design, Known Limits and Document Amendments). `interrupt/windows.rs`
   (`SetConsoleCtrlHandler`, `windows-sys` features `Win32_System_Console` and
   `Win32_Foundation`) is first compiled natively after merge. A manual check on
   Windows then follows the design's Cancellation section: the first Ctrl-C
   cancels at a safe point and exits 130 with its effects, a second ends the
   process, the routine runs on a thread the system creates and the CLI spawns
   none, and a Ctrl-C at the unlock prompt (raw mode clears
   `ENABLE_PROCESSED_INPUT`, fact 42) is still `unlock_required`, exit 3. The
   PTY cases are `cfg(unix)` (fact 43), so Windows has no automated Ctrl-C case.
2. **Native CI of the Build workflow on main** (`gh workflow run build.yml --ref
   main`), after merge and with approval: Linux x86_64 and aarch64, Windows
   x86_64 and macOS aarch64. C4 adds no test target (R-29), and the eight targets
   it extends are already in the workflow's list (fact 52).
3. **A native failure** is fixed as follow-on work in a separate session, with the
   narrowed-matrix procedure of AGENTS.md. It does not hold the merged Cycle.

Also owed, but not native: the full `cargo test --all-features --locked` suite,
once, detached, in Task 15 (ruling P1).

## The planning review record

A summary of the Cycle's Review Record: 29 rows, which this ledger does not copy.

| Round | Findings | Resolution |
| --- | --- | --- |
| 16 planning rows (source, gate, tickets, Wave) | — | Decisions 1–10, 13 and 16; rulings R-3, R-9, R-14's first form and R-21; two settled points (fact 28, and the RFC's `publication_pending`) |
| Independent review, contract lens (2026-10-26, `03f3059`) | 40: 2 blocking, 23 important, 15 minor | Decisions 11–16 and rulings R-31 to R-33; Tasks 4 and 12 split; evidence matrix, predicate table and outcome rows added |
| Independent review, source lens (`03f3059`) | 32: 4 blocking, 14 important, 14 minor | Facts 54–66 added and nine corrected or marked; library tables move to `tests/mutation_remote/` as `c4_` cases; the Ctrl-C window sits in the `Boundary` impl |
| Scoped re-review (`03f3059..580d4ad`) | 18: 0 blocking, 6 important, 12 minor | `ConflictResolve` is resume-like (R-32, R-35); `checkpoint_comment_locally_with` added (fact 73); Task 2 split into 2a and 2b |
| Adversarial round (`580d4ad`) | 18: 2 blocking, 8 important, 8 minor | Decision 17; R-34, R-35 and R-36; R-14 and R-18 rewritten; facts 67–74 |
| Third round (`580d4ad..bea46e4`) | 17: 1 blocking, 10 important, 6 minor | Decision 18; the association moves to a new table; republish gets a reservation-free evidence read; facts 75–79; this round's resolution mechanism is not rewritten again |
| Re-grounding on the merged gate (2026-10-27, `e920043`): 4 rows | the gate merged with 11 commits; two conflicts with its authority text; one test red | Main merged as `e920043`; decisions 19, 20 and 21 |
| Re-grounding, gate-boundary lens | 12: 1 blocking, 6 important, 5 minor | Decisions 19 and 20; R-37, R-39, R-40; E-6 rewritten |
| Re-grounding, remote-library lens | 8: 1 blocking, 3 important, 4 minor, and 6 stale citations | Facts 30, 33, 64, 72, 75, 76 and 80 rewritten; R-38 added; `--lib local_sync` and `--lib integration` added to tasks that edit `remote/` |
| Re-grounding, boundary, CLI and test lens | 6: 1 blocking, 2 important, 3 minor, and 13 stale citations | R-41 (`remote_resolution_owners`); E-12 added; fact 49 refreshed; Task 0's grep gains a fourth class |
| **Owner approval (2026-10-28, `8a94e9b`)** | the Cycle, the design and the plan, with E-1 to E-12 | **Approved.** Execution is subagent-driven, implementation is authorized. A push beyond the ticket branch, a PR, merging, closure, cleanup, native CI, `ticket save` and Task 15's dogfooding each still need their own go-ahead |

In all: eight independent review rounds, **151 findings** (12 blocking, 72
important, 67 minor), and 29 rows in the Record. The Cycle states that neither
the third round's edits nor the re-grounding edits had an independent review,
and that the owner reviewed them with the documents.

## Task log

Each task adds an entry here when it completes: its commit, its checks, its
review outcome and any fix round. A fix round appends here, and does not make a
new ticket comment (ruling P4).

| Task | Commit | Checks | Review | Notes |
| --- | --- | --- | --- | --- |
