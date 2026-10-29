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
| 1 | `feat: merge divergent history in item and repo sync`; fix round 1 `fix: answer busy for a sync beside a pending merge, and name a cancelled merge` | Verify list passed: 37, 44, 72, 129 and 113 SSH cases; fix round 1: 26 `c4_merging`, 5 E-3, 123 `--lib mutation`, 44 | fix round 1 applied; re-review pending | See "Task 1: Merging Synchronization" |
| 2a | `feat: let comment submission take a synchronizer` | Verify list passed: 5 `c4_comment_domain`, 41 and 25 SSH cases, 1344 `--lib repository`, clippy, fmt | pending | The author moves to Task 2b (ruling Q2). See "Task 2a: The Library's Comment Submission" |
| 2b | `feat: bind comment submission with authorship and safe points`; pre-review fix round 1 `fix: keep a comment whose branch was deleted cancellable, and release an unwritten refused binding`; review fix round 2 `fix: finish a comment refused for its input, and keep release failures` | Verify list passed: 148, 41 SSH cases; 383, 129, 44, 72; 123 `--lib mutation`; clippy, fmt; fix rounds: see their entries | review fix round 2 applied; re-review pending | Findings 1 and 2 fixed by controller ruling in fix round 1; review findings in fix round 2. See "Task 2b: The Comment Submission Binding" |
| 3 | `feat: read synchronization conflicts with string tokens` | Verify list passed: 21 `c4_conflict_read` SSH cases, 72, 44, clippy; `--lib remote`, `--lib local_sync`, `--lib integration`; fmt | pending | Fact 86 holds. See "Task 3: Conflict Inspection Read" |

### Task 1: Merging Synchronization

Commit `feat: merge divergent history in item and repo sync`, on `da83fd3`.
`origin/main` is still `a372edb`. No subagent was dispatched.

**What changed.**

- `bind/sync.rs`: `SETTINGS` is replaced by `pub(super) const MERGING`
  (`refuse_divergence: false`), used by both domain calls. The module doc,
  the `resumed` doc and the `confirmed_identity` comment no longer say that
  nothing merges (fact 84, R-16).
  - On `ConflictPending` (not cancelled) the answer keeps
    `recovery_required`, sets `integration: pending`, and carries
    `MutationDataDto::SynchronizationConflict` from the new
    `pub(crate) fn conflict_next(service, root, operation_id)`. It reads
    one inspection (R-8): `resolve` when the paths are non-empty and every
    one is `EligibleCanonical`, otherwise, or when the inspection fails,
    `external_repair` (as amended in fix round 1; the first form read the
    first path only).
  - A stop after a merge commit of the request's own names it and is
    `partial`, or stays `cancelled`. As amended in fix round 1 (below), the
    private `merge_made` reads the commit from the operation's own journal
    (the last applied integration step with a merge candidate), while the
    branch still holds it. The first form, `made_a_commit`, judged it from
    Git reachability and set only `Answer::local_change`. That gives the
    table's two partial rows: a stage-0 merge then a stage-1 conflict, and a
    push rejected after a merge. It also makes the not-confirmed answer after
    a merge `partial`, as the "later request … then is cancelled" row says. A
    fast-forward is never counted. The fenced answers are unchanged.
- `dto.rs`: `ConflictNextAction` (`resolve`, `external_repair`),
  `ConflictNextDto`, `SynchronizationConflictType`
  (`synchronization_conflict`), `SynchronizationConflictMutationDto`, the
  `MutationDataDto::SynchronizationConflict` variant, its `authority()` arm,
  and a `from_stored` arm for `item sync`, `repo sync` and `operation resume`
  guarded on the tag (after the gate's `local_context_sync` arm). Exported
  from `mutation/mod.rs`.
- `outcome.rs`: the comments of fact 6 (`:212-220`) are rewritten. No code
  mapping changed.
- `bind/resume.rs`: one module-doc line ("nothing is merged") corrected,
  because `resumed` now merges. Doc only.
- Schemas: `conflict_next.schema.json` and
  `synchronization_conflict_mutation.schema.json` (which `$ref`s the first).

**Tests.** `tests/mutation_remote/merging.rs` (new, mounted from
`tests/mutation_remote.rs` and chained into its case list) holds 25 (26 since fix round 1, with `c4_merging_mixed_conflict`)
`c4_merging_*` cases, one per row of the divergent-synchronization table,
each for both commands where the row applies to both (the stage-0 row and
"remote context deleted" are `item sync` only). `tests/mutation_remote.rs`
gains `commit_path`, `commit_in` and `leave_local_sync_conflict(&World) ->
OperationId` (R-38, for Tasks 2b, 4a and 5), and `call`'s data-schema check
chooses `synchronization_conflict_mutation.schema.json` by the data's `type`.

**Named edits.**

- E-3: the five binding-level tests Task 0 classified are rewritten and
  renamed, with their `CASES` entries:
  - `a_divergent_remote_is_merge_required_and_holds_its_reservation` →
    `a_conflicting_divergent_remote_is_recovery_required_and_releases_its_reservation`
    (canonical-conflict row; asserts the E-4 golden);
  - `a_lost_push_the_remote_has_since_built_on_is_merge_required_and_leaves_nothing_of_a_merge`
    → `a_lost_push_the_remote_has_since_built_on_is_taken_up_by_the_same_request`
    ("Local behind" row: `already_applied`, current, fast-forward, nothing
    pushed);
  - `merge_required_is_not_finished_by_the_same_request_after_the_branch_is_rebased`
    → `a_divergent_remote_is_merged_by_the_same_request_with_no_rebase`
    (clean-divergence row);
  - `a_resume_never_merges_and_leaves_nothing_of_a_merge` →
    `a_resume_takes_up_a_lost_push_the_remote_has_since_built_on`;
  - `a_resume_does_not_continue_a_merge_made_some_other_way` →
    `a_resume_of_a_synchronization_whose_conflict_is_pending_reports_it_with_no_transport`
    (resume table: `recovery_required` with `SynchronizationConflictMutationDto`,
    no transport).

  The five domain-level `REFUSING` tests are unchanged.
- E-4: `repo_sync_merge_required.json` is renamed to
  `repo_sync_recovery_required.json` and regenerated with
  `MANYHANDS_UPDATE_GOLDEN=1`. Its diff was read: code `recovery_required`,
  `integration: pending`, data `{type: synchronization_conflict, conflict:
  {operation_id: <operation>, next_action: resolve}}`, recovery
  `[request.retry, repo.inspect]`. The registration is renamed and kept in
  order.
- E-5: **no edit was needed.** `every_synchronization_error_has_its_code`'s
  conflict rows already map `ConflictPending` and
  `ExternalResolutionRequired` to `recovery_required`, and the
  `MergeRequired` row stays (the code stays defined). `synchronization_error_code`
  is unchanged, so the test passes unedited. The `merge_required` recovery
  tests at `:1565` and `:1633` (and `:1519`, `:2216`, `:2223`, ruling P8) are
  unchanged.
- E-7: `DATA_ROOTS` gains `synchronization_conflict_mutation.schema.json`
  (51). `CODES_NO_FIXTURE_CAN_PRODUCE` and `CODES_NO_MUTATION_GOLDEN_HAS`
  gain `merge_required` with its reason. `UNBOUND_COMMANDS` is unchanged.
- E-8: `contract_enumerations` gains `conflict_next.schema.json` /
  `next_action` and `synchronization_conflict_mutation.schema.json` / `type`.
- E-10: the six accessors in `tests/support/mutation.rs` and `repair_data` in
  `tests/support/repair.rs` gain the new variant's arm.

**TDD.** RED: `cargo test --all-features --locked --test mutation_remote --
c4_merging`, with the cases written and nothing implemented, failed to
compile with exactly the missing names (`ConflictNextDto`,
`ConflictNextAction`, `SynchronizationConflictMutationDto`,
`SynchronizationConflictType`, `MutationDataDto::SynchronizationConflict`).
GREEN: the same filter ran 25 cases; the busy row's first form failed, which
led to finding 1 below, and passed once its setup was corrected.

**Verify list** (each `devenv shell -- cargo … --locked`, 2026-10-28):

| Command | Result |
| --- | --- |
| `fmt --check` | passed |
| `clippy --all-targets --all-features -- -D warnings` | passed |
| `test --all-features --lib mutation::outcome` | 37 passed |
| `test --all-features --test mutation_contract` | 44 passed |
| `test --all-features --test read_contract` | 72 passed |
| `test --all-features --test mutation_bridges` | 129 passed |
| `test --all-features --test mutation_remote` | 113 SSH cases passed (88 before, and the 25 `c4_merging_*`), 30 min 42 s |

`git fetch origin`: `origin/main` is `a372edb`; `git merge-tree --write-tree
HEAD origin/main` exits 0.

**Findings for the controller.**

1. **"A new sync while another operation's conflict is pending" (corrected in
   fix round 1).** The controller ruled that the row covers every new
   request, including a new `item sync` of the item whose merge is pending
   and any new sync while primary's merge is pending. The first commit
   answered those `item_not_found` and `worktree_conflicted`, because for a
   new request the domain inspects the target worktree
   (`remote/sync.rs:5483-5485`) before the reservation (`:5506`), and an
   item's synchronization inspects primary's worktree too. Fix round 1 makes
   the binding answer them `busy`, with nothing recorded, and does not touch
   `remote/`. `c4_merging_new_sync_while_a_conflict_is_pending` asserts
   `busy` for all five: `repo sync` and `item sync` of another item while an
   item's merge is pending, a new `item sync` of that item, and a new `repo
   sync` and `item sync` while primary's merge is pending. The same request
   (restart) still reads the conflict, and the outside-merge row (no
   retained operation) is still `worktree_conflicted`.
2. **A lost push the remote has since built on is now `already_applied`.**
   Under refusal the binding reported `published` with the pushed commit
   (the domain's `observer.verified` before `MergeRequired`). Under merging the
   domain settles the publication and fast-forwards, and its outcome is
   `AlreadyCurrent`: the result says the remote is current and names no
   commit, though an earlier attempt of the same request made the push. That
   matches the table's "Local behind" row, which the rewritten tests assert.
3. **The table's "success, 0" for `already_applied` is asserted as `noop`.**
   `Outcome::of_mutation` makes every `already_applied` a `noop`, and the
   design's exit table puts `noop` in exit 0.
4. **The Ctrl-C rows are asserted at the library's level** (corrected in fix
   round 1). Three go through the call's own cancellation token: the safe
   point before the push, after the push returned, and, since fix round 1, a
   token cancelled while the fetch's advertisement is held. That last case
   first went through `request cancel` and never cancelled the token it
   passed. The row "a later request that begins with that unsettled push,
   then is cancelled" goes through `request cancel`, which is how fact 74's
   path is reached. Exits, C2's bounded retry and the signal handling itself
   are Tasks 7 and 13.
5. **The R-38 row's "checked before the busy row's check" is not co-staged.**
   The gate refuses to start a local sync while any remote operation is
   retained (`local_sync/record.rs:1064`), and a pending local sync refuses
   every remote reservation, so the two cannot both stand through public
   paths. The row's own answer (`recovery_required`, `[repo.inspect]`,
   nothing) is asserted for both commands.

#### Task 1, fix round 1

The task review found four issues; the controller ruled on each. Commit
`fix: answer busy for a sync beside a pending merge, and name a cancelled
merge`, on `ca5f17d`. `remote/` is untouched.

1. **Busy row.** `bind/sync.rs`'s `stopped` gains a binding-level mapping.
   - It applies when the request never reserved (journal absent) and the
     domain refused it as `WorktreeConflicted` or `TargetNotMaterialized`.
   - The new read-only `SyncBinding::beside_a_pending_merge` then reads the
     registry. If another remote operation holds an integration step in
     `conflict_pending`, `resolution_prepared` or `commit_prepared`, and
     that step's merge is in a worktree this sync inspects (primary's, or
     this target's own), the answer is `busy`.
   - So the answer is the transient `busy`, with no effect, no record, and
     `[request.retry]`.
   - With no such operation, the domain's answer stands, so an outside
     merge is still `worktree_conflicted`.
   - The case asserts `busy` for five syncs: a new `item sync` of the same
     item, `repo sync`, `item sync` of another item, and while primary's
     merge is pending, a new `repo sync` and a new `item sync`. It then
     reads the conflict again with the same request.
2. **Blocked network call.** `elsewhere` takes the call's token. The case
   cancels that token while the server holds the fetch's advertisement, then
   releases it. **Recorded answer: `cancelled`, at the next safe point, with
   its effects intact** (publication pending, nothing merged, nothing
   pushed), for both commands. The `request cancel` form is dropped:
   `a_cancellation_requested_while_the_transport_is_blocked_is_honored_when_it_returns`
   already covers it.
3. **A cancelled merge is named.**
   - `made_a_commit` is replaced by `merge_made`. It returns the merge
     commit from the operation's journal (the last applied integration step
     with a merge candidate), while the branch still holds it.
   - Every stop other than a conflict names that commit through
     `with_merge`: `integration: complete`, and the commit as `commit_oid`
     unless a published one is named already. This covers the general stop,
     the stop with an unreadable journal, and not-confirmed. A cancellation
     keeps those effects and stays `cancelled`.
   - The conflict path is unchanged: `integration: pending`, and
     `local_change` set from the same source, so the stage-0 row is still
     `partial`.
   - `sync::recorded_effects` uses the same source. A `request_mismatch`
     on an accepted record after its merge is now `partial`, naming the
     merge; `c4_merging_push_rejected_after_a_merge` asserts it.
   - The module doc no longer says that every stop after a merge is
     `partial`.
   - The cases assert the named merge: the push rejected after a merge, and
     the mismatch; cancelled before the push; cancelled after the push
     returned (with publication published); and the unsettled push, both the
     lost call and the not-confirmed answer.
4. **`conflict_next`.** `resolve` only when the path set is non-empty and
   every path is `EligibleCanonical`. An inspection failure still answers
   `external_repair`, with a comment. A new case, `c4_merging_mixed_conflict`
   (both commands), makes a ticket conflict and a plain-file conflict in one
   merge: two conflicted paths, `external_repair`.

5. **A successful merge is named too (controller ruling, folded into this
   round).** `finished` takes the service and the accepted request, and
   applies `with_merge(effects, merge_made(..))`. A sync whose request made
   a merge commit then says `integration: complete` plus the commit,
   whether it succeeds, stops or is cancelled. A fast-forward or a plain
   push makes no merge commit and keeps `integration: not_requested`. The
   module doc says so. C4's own cases are updated: clean divergence,
   identity then the same request, and index refresh failing after
   publication (`partial`, with `integration: complete`).
   - No pre-existing golden changes: no golden is of a successful merge.
   - **One pre-existing test changes: the E-3 rewrite**
     `a_divergent_remote_is_merged_by_the_same_request_with_no_rebase`
     (`tests/mutation_remote.rs:2909`). Its expected effects gain
     `integration: complete`. The implementer stopped and reported it. The
     controller ruled that it is an E-3 site, which E-3 sends to the
     clean-divergence outcome, so the edit is within E-3. It was then made,
     as a one-line change to that assertion.

**Recorded limit (controller-accepted).** `beside_a_pending_merge` places a
pending merge's worktree only for `synchronize_primary` and
`synchronize_context` operations. A merge pending under a `promote` or
`close` operation is not matched, and the domain's answer
(`worktree_conflicted` or `item_not_found`) stands for a new sync that
inspects that worktree.

**Commands** (each `devenv shell -- cargo … --locked`):

| Command | Result |
| --- | --- |
| `clippy --all-targets --all-features -- -D warnings` | passed |
| `test --all-features --test mutation_remote -- c4_merging` | 26 SSH cases passed (16 min 35 s) |
| the five E-3 tests, each by its own filter | 1 SSH case passed each |
| `test --all-features --lib mutation` | 123 passed |
| `test --all-features --test mutation_contract` | 44 passed |
| `fmt --check` | passed |

The whole `--test mutation_remote` target was not run again. The controller
asked for it only after a change to `remote/sync.rs` or to a shared helper of
pre-existing cases, and neither was changed.

### Task 2a: The Library's Comment Submission

The commit is on `8af90fb`. `origin/main` is still `a372edb`, and
`git merge-tree --write-tree HEAD origin/main` exits 0. No subagent was
dispatched.

**Controller rulings, made before any code was written.** The implementer
stopped and asked about two conflicts in the brief.

1. **The parent check stays in place on the public path (Q1, option A).**
   - The brief moves the parent check before the binding row's insert, but
     `src/repository/comment_publication_tests.rs:365` pins the old order.
     That test, `comment_publication_rejected_row_refresh_and_reopen_keep_one_binding`,
     refuses a parent through the public `submit_comment` and expects the
     binding row to remain. It is not a named edit, and fact 73 keeps
     `checkpoint_comment_locally`'s behavior.
   - So the parent is checked before the insert only on the binding's path
     (`release_unwritten: true`). The public path, which passes `false`,
     keeps today's order.
2. **The author moves to Task 2b (Q2, option A).** In Task 2a nothing outside
   tests would call `CommentAuthor::parse`. The library's non-test build
   therefore reports `parse` as never used and `CommentAuthorError` as never
   constructed, and `-D warnings` fails. The following move to Task 2b, where
   `comment add` first calls them:
   - `CommentAuthor`;
   - its `parse` and its error type;
   - the `author` parameter;
   - the author domain cases.

   No `allow` or `expect(dead_code)` is used.
3. **Accepted as the implementer proposed:**
   - the `Watch::new` signature below;
   - a synchronizer that returns `None` leaves the publication pending, as
     `Synchronization(RecoveryRequired)`;
   - `restart` is passed on both paths.

**What changed.**

- `comment_publication.rs`:
  - `pub(crate) type Synchronizer<'a> = dyn FnMut(SynchronizeRemoteRequest) -> Option<Result<SynchronizationResult, SynchronizationError>> + 'a`.
  - `pub(crate) fn submit_comment_with(&self, request: PublishCommentRequest, restart: bool, release_unwritten: bool, synchronize: &mut Synchronizer<'_>) -> Result<CommentSubmissionOutcome, CommentSubmissionError>`.
  - `pub(crate) fn retry_comment_publication_with(&self, request: RetryCommentPublicationRequest, synchronize: &mut Synchronizer<'_>) -> Result<CommentSubmissionOutcome, CommentSubmissionError>`.
  - The public `submit_comment` and `retry_comment_publication`, and
    `submit_comment_with_identity_config_for_testing`, keep their signatures.
    They pass a closure over `synchronize_remote` (default settings, no
    observer) with `restart: false` and `release_unwritten: false`.
  - The submission no longer holds a `SessionCredentials`; the synchronizer
    makes the one domain call.
  - `restart` reaches the synchronizer on both the receipt path (fact 65's
    `:216-224`, which passed `false` before) and the fresh path. On the
    fresh path no child row exists, so the domain ignores it.
  - **`None` from the synchronizer** means no domain call was made, as from
    a `NoSession` preview or credentials already in use. The publication is
    then `Pending { reason: Synchronization(RecoveryRequired) }`.
  - New `pub(super) fn refuse_unknown_comment_parent`. It refuses a parent
    that is not a conforming comment of the item, or that is the comment
    itself (`MissingAuthoringTarget`, which is `item_not_found`). It reads
    what the later parent check reads:
    - the item's editing worktree, after `validate_context_worktree`, when
      the worktree exists;
    - otherwise the primary checkout, as `prepare_context_unlocked` reads it
      for the item.

    Content it cannot read is left to the later steps, which report it as
    before.
- `src/repository.rs`:
  - `checkpoint_comment_locally` keeps its signature and now delegates to
    `pub(crate) fn checkpoint_comment_locally_with(&self, request: SubmitCommentRequest, effective_config: Option<&Config>, release_unwritten: bool)`
    with `false`.
  - Its doc says that `release_unwritten` selects the binding's semantics:
    the early parent check and the release of an unwritten binding.
  - With `release_unwritten`:
    - the parent is checked before `bind_comment_publication`, for a new row
      only, since only a new row inserts a binding row;
    - when `IdentityRequired` finds the comment file absent, the binding is
      released under the lease, in place of `complete_lifecycle`. If the
      release finds that something was written, the row is completed as
      before.
  - The production caller still calls `checkpoint_comment_locally` when
    `release_unwritten` is false. That keeps fact 73's one production caller,
    and keeps the function live in the non-test build.
  - Nothing else in `src/repository.rs` changed.
- `recovery.rs`: new
  `pub(super) fn release_unwritten_comment_binding(&self, root, record, operation_id) -> Result<bool, RepositoryError>`
  (R-34).
  - One immediate transaction checks that:
    - the row is the operation's `submit_comment` row and was not abandoned;
    - its last step is null, `branch_observed`, `worktree_observed` or
      `completed`;
    - the binding has no checkpoint;
    - the child synchronization has no remote row.
  - If all hold, it deletes the binding and sets the row to `completed` /
    `rejected`, then returns `true`. Otherwise it changes nothing and returns
    `false`.
  - No table, column or trigger changed.
- `bind/sync.rs`: `Watch` is `pub(super)`. New
  `pub(super) fn new(accepted: &'a Accepted, call: &'a CallContext<'a>, subject: Subject, owner: Option<RequestId>) -> Watch<'a>`
  reads `unsettled` once. `SyncBinding::call` now uses it, so no test-only
  use keeps it live.
  - It takes a reference because `Accepted` is not `Clone` and `Watch`
    holds `&'a Accepted`. Task 2b builds the child's `Accepted` as
    `resume.rs:647-655` does and passes a reference.

**Tests.** `tests/mutation_remote/comment_domain.rs` is new, mounted from
`tests/mutation_remote.rs` and chained into its case list. It holds five
`c4_comment_domain_*` SSH cases, which call the crate-private functions
directly:

- `parent_refused_before_binding` (`release_unwritten: true`):
  - an unknown parent and a self-parent are refused with no binding row, in
    the item's worktree;
  - a comment of the item is accepted as a parent;
  - on a ticket committed on primary with no context, an unknown parent is
    refused with no binding row and no worktree made, and a primary comment
    is accepted as a parent.
- `identity_required_releases_an_unwritten_binding`:
  - the binding is deleted and the row is `completed` / `rejected`;
  - a checkpoint then lands in the context, and the same operation binds
    afresh with a new child;
  - the comment is checkpointed once, on the new tip, with the identity's
    `created_by`.
- `identity_required_after_a_write_keeps_the_binding`:
  - the first attempt writes the file and stops at `BeforeCheckpointCommit`;
  - `IdentityRequired` keeps the binding and the row, and leaves the file
    unchanged;
  - after the identity is restored, the file is checkpointed once with the
    same bytes and the same child.
- `without_release_the_public_path_is_unchanged` (`release_unwritten: false`):
  - a refused parent leaves its binding row;
  - `IdentityRequired` keeps the binding and completes the row;
  - after a checkpoint lands, the same operation is `RecoveryRequired`.
- `the_synchronizer_sees_the_child_and_restart`:
  - the recorded requests carry the binding's child ID, the context target,
    the canonical root, and `restart` as given (fresh path `false`, receipt
    path `true`);
  - `None` leaves the publication pending as `RecoveryRequired`;
  - `retry_comment_publication_with`, through a real `synchronize_remote`,
    passes the same child and publishes the comment to the server.

`MERGING` is not asserted here: it is a binding setting given to
`Session::synchronize`, not a field of `SynchronizeRemoteRequest`. Task 2b
asserts it, where the binding's closure passes it.

**TDD.** RED: `cargo test --all-features --locked --test mutation_remote --
c4_comment_domain`, with the cases written and nothing implemented, failed to
compile. It reported E0599 six times for `submit_comment_with` and once for
`retry_comment_publication_with`. GREEN: the same filter ran 5 SSH cases,
all passing, on the first run after the implementation.

**Verify list** (each `devenv shell -- cargo … --locked`, 2026-10-28):

| Command | Result |
| --- | --- |
| `test --all-features --test mutation_remote -- c4_comment_domain` | 5 SSH cases passed (25 s) |
| `test --all-features --test comment_publication` | 41 SSH cases passed (281 s) |
| `test --all-features --test remote_merge_recovery` | 25 SSH cases passed (309 s) |
| `test --all-features --lib repository` | 1344 passed, 0 failed (1621 s) |
| `clippy --all-targets --all-features -- -D warnings` | passed |
| `fmt --check` | passed |

The `--lib repository` run includes the 34 `comment_publication_tests`, among
them `comment_publication_rejected_row_refresh_and_reopen_keep_one_binding`,
which passed unchanged.

**For Task 2b.**

- **Author (moved by ruling Q2):**
  - add `pub(crate) struct CommentAuthor` and
    `parse(&str) -> Result<CommentAuthor, CommentAuthorError>` (R-7);
  - add `author: Option<CommentAuthor>` to `submit_comment_with` (after
    `request`) and to `checkpoint_comment_locally_with` (after
    `effective_config`). `checkpoint_comment_locally` and the public path
    pass `None`;
  - when an author is given, its text replaces fact 20's
    `format!("{} <{}>", identity.name, identity.email)` in the `if !exists`
    branch, so it is written once. A commit identity is still required;
  - add the domain cases "a given author is written verbatim" and "a
    malformed one is refused before any write" to `comment_domain.rs`.
- **The binding** calls
  `submit_comment_with(request, restart, true, &mut synchronizer)`, with
  `restart: true` on re-entry. Its synchronizer is a closure over
  `call.session.synchronize(service, request, MERGING, &mut watch)`, with
  `watch = Watch::new(&child_accepted, call, subject, owner)`.
- **`IdentityRequired` with `release_unwritten: true`** leaves the row
  `rejected`, which the journal reads as `Absent`. The binding keeps the
  record accepted itself (R-34).

### Task 2b: The Comment Submission Binding

The commit is on `4a6d981`. `origin/main` is still `a372edb`, and
`git merge-tree --write-tree HEAD origin/main` exits 0. No subagent was
dispatched.

**Moved here from Task 2a (ruling Q2), as the plan's Task 2a interface
gives it.**

- `comment_publication.rs`: `pub(crate) struct CommentAuthor` with
  `parse(&str) -> Result<CommentAuthor, CommentAuthorError>` and `as_str`
  (R-7). The form is `Name <email>`, at most 256 bytes. The name and the
  email are each nonempty, hold no control character and no angle bracket,
  neither begins nor ends with white space, and a `git2` signature carries
  both unchanged; the email holds an `@` (fact 57's rules). The
  configuration round trip of `ValidIdentity::of` is not made: it is about
  Git configuration, and writes a scratch file. `CommentAuthorError` holds
  nothing of the value.
- `submit_comment_with(request, author, restart, release_unwritten,
  synchronize)` and `checkpoint_comment_locally_with(request, config,
  author, release_unwritten)`. The author's text replaces fact 20's format
  in the `if !exists` branch, so it is written once. A commit identity is
  still needed. `checkpoint_comment_locally` and the public path pass
  `None` and `false`; a call with an author goes through the `_with`
  function whatever `release_unwritten` is.

**What changed.**

- `bind/comment.rs` (new): `CommentAddInput` (no derive), the
  `CommentAddBinding`, `pub(crate) fn comment_synchronization(connection,
  root: &Path, operation_id) -> Option<(comment, synchronization)>`, which
  resolves either ID through the binding table (R-5), and
  `recorded_effects`.
  - `prepare` parses the author (`invalid_input`), resolves the root, and
    accepts the request on the item's branch with `expected: Missing`,
    family `Local`. Everything else is the domain's.
  - `run` and `rerun` call `submit_comment_with` with the full input,
    `release_unwritten: true`, and `restart` false and true (fact 65).
    `reenter` answers `Entered::Unfinished`;
    `ended_without_checkpoint` is `SaysNothing`.
  - The synchronizer is a closure over `sync::of_a_comment`, which runs
    `SyncBinding`'s own call: `call.session.synchronize(service, request,
    MERGING, &mut watch)`, with `watch = Watch::new(&child, call, subject,
    owner)`. `child` is built as `resume.rs:647-655` builds one, with the
    child's ID; `subject` and `owner` are the `comment add` request. So the
    child's progress, cancellation, unsettled-push and fence handling,
    `beside_a_pending_merge`, and conflict data are item sync's.
  - Outcomes (R-6): published or current is `ok` (or `discovery_pending`,
    owed, when the publication's hand-off is); no publication remote is
    `ok`, final; the comment's own hand-off pending is `recovery_required`,
    owed, `pending_reason: recovery_required`; a stopped child gives its
    code, its publication when it is published, its integration, and its
    conflict as `conflict`. A child that published without the comment is
    `recovery_required`, final.
  - Standing (R-34): a stopped child is owed, except a cancelled or
    retained row or a code `never_finished_by_the_same_request`, which is
    final; `identity_required` is owed; a domain error is the local row's
    to settle, or owed once the binding records a checkpoint.
  - Effects: `write: written`, `checkpoint: committed` and the checkpoint
    as `commit_oid`, on every answer once the comment is checkpointed; the
    publication and integration as above; discovery pending when the
    comment's or the publication's hand-off is.
  - Data: `CommentMutationDto` whenever the comment is checkpointed.
    `created_by` is read from the checkpoint's file, so it is the author
    written, given or the identity's.
- `bind/sync.rs`: `SyncBinding::call` delegates to a new `observed`, which
  also gives back what the domain returned; `pub(super) fn of_a_comment`
  calls it for the child. No behavior change for item or repo sync.
- `dto.rs`: `CommentPendingReason`, `CommentMutationType` (`comment`),
  `CommentMutationDto`, `MutationDataDto::Comment`, its `authority()` arm
  (none) and its `from_stored` arm. Exported from `mutation/mod.rs`.
- `command.rs`: `CommentAdd => "comment add"`, with the Command
  Predicates table's values (R-32).
- `mod.rs`: `Mutation::CommentAdd`, its `kind`, `confirmed` (none) and
  `into_binding` arms. `bind/mod.rs`: the module, and the
  `recorded_effects` arm. `outcome.rs`: `never_finished_by_the_same_request`
  is `pub(crate)`, for the binding's standing.
- `schemas/v1/comment_mutation.schema.json` (new). The conflict is inlined,
  as a nullable object, because the schema checker allows `$ref` only
  alone (fact 50).

**Named edits.**

- E-6: `the_published_names_are_the_ones_records_hold` gains `comment
  add`. `only_a_synchronization_stops_at_safe_points_and_none_needs_confirmation`
  is renamed `the_stopping_commands_and_their_stages_and_confirmation` now,
  since `comment add` makes the old name untrue, and gains `comment add` as
  a stopping command with an operation that reports `Writing`, plus its
  other predicates. Task 5 adds `item republish` to it.
- E-7: `DATA_ROOTS` gains `comment_mutation.schema.json` (52). No code
  list changes.
- E-8: `contract_enumerations` gains the schema's four enums: `type`,
  `publication` (`PublicationEffect::ALL` less `not_requested`),
  `pending_reason`, and `conflict.next_action`.
- E-9: `HOMES` gains `("comment add", Home::Remote)` (33), and
  `tests/mutation_remote.rs`'s `CONFORMANCE` gains its row (3); its
  header comment no longer says "two".
- E-10: the six accessors of `tests/support/mutation.rs` and
  `repair_data` gain the `Comment` arm.

**Tests.** `tests/mutation_remote/comment.rs` (new) holds 27 `c4_comment_*`
SSH cases: one per row of the comment-submission table, the no-context
identity case carried from Task 2a's review, `MERGING` (a context the
remote moved on merges), and Review Focus 1 and 5 and the
`request_mismatch` case of Step 4. `comment_domain.rs` gains the two
author cases (7 in all). Goldens `comment_add`,
`comment_add_push_rejected`, `comment_add_ticket_closed` and
`comment_add_item_not_found` are new and registered; each was read.

**TDD.** RED: `cargo test --all-features --locked --test mutation_remote --
c4_comment`, with the cases and named edits written and nothing
implemented, failed to compile with exactly the missing names (26 errors:
`Mutation::CommentAdd`, `CommentAddInput`, `CommentMutationDto`,
`CommentPendingReason`, `CommentMutationType`, `CommentAuthor`,
`MutationDataDto::Comment`, and the new `author` argument). GREEN: the
first run passed 27 of 34; the failures were four goldens not yet written,
a 255-byte "256-byte" author in the test, and the two no-remote cases,
whose fixture left primary's configuration file dirty. With the goldens
written and the configuration committed, each passed.

**Verify list** (each `devenv shell -- cargo … --locked`, 2026-10-28):

| Command | Result |
| --- | --- |
| `test --all-features --test mutation_remote` | 148 SSH cases passed (34 `c4_comment`, among them the 7 `c4_comment_domain`), 32 min 23 s |
| `test --all-features --test comment_publication` | 41 SSH cases passed (5 min 38 s) |
| `test --all-features --test mutation_replay` | 383 passed (13 min 3 s) |
| `test --all-features --test mutation_bridges` | 129 passed |
| `test --all-features --test mutation_contract` | 44 passed |
| `test --all-features --test read_contract` | 72 passed |
| `test --all-features --lib mutation` | 123 passed |
| `clippy --all-targets --all-features -- -D warnings` | passed |
| `fmt --check` | passed |

The remote conformance case, `every_command_of_this_target_keeps_the_rules_of_the_boundary`,
now drives three rows. It ran in about 74 s, against the harness's 100 s
watchdog; with two rows it ran in about 48 s.

**Findings for the controller.**

1. **`remote_branch_deleted` leaves the child's reservation held, with no
   record to cancel it.** The row says the record is finished, and it is
   (`Standing::Final`, as R-34 says). But the child synchronization stops
   with its row pending and keeps the repository's reservation, as `item
   sync` does on this code. A probe showed the reservation held, and a new
   `item sync` then `busy`. `request cancel` on the finished `comment add`
   record asks nothing, and the comment's record holds only its local
   operation (fact 79), so the row's "human output names `request cancel`
   for the request, then `item republish`" cannot work as written, and
   `item republish` is `busy` while the reservation is held (republish
   table). The library's recovery for the finished record is empty. A
   ruling is needed: keep the record accepted for this code, release the
   child, or another route. Nothing here works around it.
2. **A refused `occupied_path` leaves its binding row.** The domain binds
   the comment ID before it finds the ID occupied, and only an identity
   stop releases a binding. A probe showed one binding row left, and the
   same request sent again answered `internal_error` (the binding's
   comment-ID collision), not `occupied_path`. The row itself (first
   answer `occupied_path`, input, no effect) holds. The same is true of any
   refusal after the binding, such as an item missing from its context.
   This is the domain's order since Task 2a; changing it is outside this
   task.
3. **The closed-ticket golden's scope names the item's branch and
   worktree.** `item_scope` names them for any accepted item request,
   though for a ticket with no context neither exists. `ticket save`
   refuses a closed ticket before acceptance, so its golden names none.
   `comment add` leaves the check to the domain, under its lease.
4. **A comment's data names no host.** When the child stops for an
   untrusted host, `comment add` answers with the code, but its data is the
   comment's, so `host.approve` is not offered as `item sync` offers it.
   No row covers it.
5. **No publication remote, staged with a dirty configuration file.** The
   first form of the two no-remote cases removed the remote in primary's
   working tree only, as the merging cases do; the comment's index hand-off
   then stops, and the answer is not `ok`. The cases commit the change on
   primary instead.

#### Task 2b, fix round 1

Before the task review, the controller ruled on findings 1 and 2 ("Task 2b
concern 1" and "concern 2" in the controller ledger). Findings 3 and 4 are
unchanged: the conformance margin is carried to Task 5, E-6's rename is
accepted, and the host action is a deferred minor. The commit is `fix: keep a
comment whose branch was deleted cancellable, and release an unwritten
refused binding`, on `846b35f`. `origin/main` is still `a372edb`.

1. **A child stopped as `remote_branch_deleted` is owed, not final.**
   - The same holds for any code `never_finished_by_the_same_request`
     names. `standing_of` loses its early `Final` for them, so the child's
     own standing applies: a held reservation leaves the record accepted.
     The binding no longer needs that function, which is private again.
   - The result offers exactly `[request.cancel]`, naming the `comment add`
     request, by `recovery`'s rule for a held synchronization.
   - `offering_cancel` (`outcome.rs`) keeps the action when the reservation
     is held by the comment's synchronization. It resolves that through
     `comment_synchronization` on the envelope's operation (R-5). Before
     this, it kept the action only when the reservation was the envelope's
     own operation, the comment's local one, and so always dropped it.
   - **The path out.** `request cancel` sets the request's flag. The same
     request then re-enters with `restart`, and its synchronization's
     `Watch` reads the flag at the first safe point, before the domain
     refuses the deleted branch again. The answer is `cancelled`, with the
     checkpoint committed and the publication pending; the child's row is
     cancelled, the reservation released, and the record finished. The
     flag alone does this, so `cancel_request` (`progress.rs`) is
     unchanged.
2. **A refusal after the binding that wrote nothing releases it.**
   - `checkpoint_comment_locally_with` gains `released: &mut bool`.
     `checkpoint_comment_locally` passes a throwaway one.
   - With the binding's semantics, an error after the binding row was
     inserted is released when: the call left nothing behind
     (`CallEffects::leave_nothing_behind`, a new row with no effect
     begun); and the comment's file is absent from the item's worktree.
     The release reuses `release_unwritten_comment_binding`, whose guard
     also requires no write step, no checkpoint and no child row. It runs
     under the lease, in one transaction with the row's rejection.
   - The refusal is returned unchanged, and `released` is set.
   - `CommentSubmissionError` gains a private `released` flag, with a
     `pub(crate)` reader. The submission sets it. Nothing builds that
     error by literal.
   - The binding's `refused` keeps the record (`Standing::Owed`) for a
     released refusal, as R-34 has it for an identity stop. The same
     request then binds afresh, and is refused the same way. A refusal
     before the binding (parent, closed ticket, pending conflict,
     unfinished local sync) is unchanged, with its record deleted. A
     refusal that wrote the file, or began any effect, keeps its binding.
   - With the record kept, `occupied_path`'s recovery is `[request.retry]`,
     by `recovery`'s rule for an accepted record of a stopping command.

**Tests.**

- `c4_comment_remote_context_deleted` now asserts:
  - `remote_branch_deleted`, partial, with the checkpoint committed and the
    publication pending;
  - the record accepted, the reservation held, and recovery exactly
    `[request.cancel]` naming `REQUEST_1`, with no operation;
  - nothing recreated, and a new `item sync` `busy`;
  - then `request cancel` (`requested`), and the same request:
    `cancelled`, the same effects, no recovery, the reservation released,
    and the record finished. A replay gives the same answer;
  - then a new `item sync`, which is not `busy`: it reaches the remote, and
    answers `remote_branch_deleted` under its own request.
- `c4_comment_comment_id_occupied` sends the same request twice. Each time
  it asserts `occupied_path` (input), the same operation, the record
  accepted, no binding row, the row `rejected`, and no file.
- No golden changed.

**Commands** (each `devenv shell -- cargo … --locked`):

| Command | Result |
| --- | --- |
| `test --all-features --test mutation_remote -- c4_comment` | 34 SSH cases passed (7 min 2 s) |
| `test --all-features --test comment_publication` | 41 SSH cases passed |
| `test --all-features --test mutation_replay` | 383 passed |
| `test --all-features --test mutation_contract` | 44 passed |
| `test --all-features --lib mutation` | 123 passed |
| `clippy --all-targets --all-features -- -D warnings` | passed |
| `fmt --check` | passed |

`progress.rs` is untouched, so the `request cancel` cases were not run
again; nor was the whole `mutation_remote` target, which the review
scopes.

#### Task 2b, review fix round 2

The task review found one Important issue and some Minors, and the
controller ruled on each. The commit is `fix: finish a comment refused for
its input, and keep release failures`, on `699a270`.

1. **Important: a released refusal of failure class `input` is final.**
   - `refused` makes a refusal that released its binding
     `Standing::Final` when its code's class is `input`: `occupied_path`,
     `item_not_found` after the binding, or any other such code. The
     record is finished with that result, and offers nothing.
   - A re-send of the same request replays the stored `occupied_path`.
   - A released refusal of any other class stays `Owed`.
   - The release is unchanged: the binding is deleted and the row rejected
     in one transaction.
   - This supersedes fix round 1's last note, that `occupied_path`'s
     recovery is `[request.retry]`: it is now none, as the table row says.
2. **Minor 2.** A registry failure in `release_unwritten_comment_binding`
   is now propagated with `?`, as on the identity path. Only the guard's
   refusal means "not released"; before this, `.unwrap_or(false)` made a
   failure mean that too.
3. **Minor 3.** `release_unwritten_comment_binding`'s doc names its two
   callers, and what its error means.
4. **Minor 4.** In `standing_of`, a child row that completed beside an
   error is now `Owed`, not `Unconfirmed`, which is documented as "Git
   could not be read". Both leave the record accepted.
5. **Minor 5.** `c4_comment_malformed_author` asserts no recovery.
   `c4_comment_self_parent` asserts no recovery and the `input` class.
6. **Minor 1.** Fix round 1's report said the release also covered a
   test-only injected failure before the write (`BeforeItemWrite`). That
   was wrong, because `effects.begin()` runs before that check
   (`src/repository.rs:3123-3124`), so `leave_nothing_behind` is false and
   nothing is released. The fix report corrects it. This ledger's fix
   round 1 entry does not carry the claim: it gives the condition as "no
   effect begun".

**Tests.** `c4_comment_comment_id_occupied` sends the same request twice.
The second answer is the first one, replayed with no connection. It
asserts, on both:

- `occupied_path` (input), no effect and no data;
- no recovery, and the same operation;
- the record `Finished`, no binding row, the row `rejected`, and no file.

No golden changed.

**Commands** (each `devenv shell -- cargo … --locked`):

| Command | Result |
| --- | --- |
| `test --all-features --test mutation_remote -- c4_comment` | 34 SSH cases passed (7 min 12 s) |
| `test --all-features --test comment_publication` | 41 SSH cases passed |
| `test --all-features --test mutation_replay` | 383 passed |
| `test --all-features --lib mutation` | 123 passed |
| `clippy --all-targets --all-features -- -D warnings` | passed |
| `fmt --check` | passed |

### Task 3: Conflict Inspection Read

The commit is `feat: read synchronization conflicts with string tokens`, on
`54cafc4`. `origin/main` is still `a372edb`, and
`git merge-tree --write-tree HEAD origin/main` exits 0. No subagent was
dispatched.

**What changed.**

- `remote/conflict_read.rs` (new):
  - `RepositoryService::show_conflict(repo, operation_id) ->
    Result<ConflictDto, ReadError>`.
  - `ConflictDto` and `ConflictPathDto`, as the design's tables, with
    `request_id` (null until Task 4a). They derive `Debug` and
    `Serialize`, the one exception the Global Constraints allow.
  - `contract_enum!`s `ConflictState`, `ConflictReadNextAction`,
    `ConflictPathKind` and `ConflictPathStructure`. Three more close the
    design's other value sets: `ConflictTarget` (`primary`, `document`,
    `ticket`), `ConflictStage` (`context`, `primary`) and
    `ConflictReadEligibility` (`eligible_canonical`,
    `external_resolution_required`), so that E-8 ties every schema enum
    to Rust.
  - `pub(crate) fn path_tokens` and `pub(crate) fn observation_text`, as
    the brief names them (ruling P2), and `pub(crate) fn display_path`
    (R-11's rule, copied from the gate's).
- **R-9's encoding.** Each digest starts with a domain string
  (`manyhands-conflict-observation-v1`, `manyhands-conflict-path-v1`).
  Each field follows as a big-endian `u64` length and its bytes, as the
  gate's `observe::field` writes it.
  - The observation hashes the operation ID, window number, stage
    ordinal, fingerprint, head, configuration digest and root.
  - A path token hashes the observation's digest, the path's position in
    the inspection's `paths`, the path bytes, and each side's OID and mode.
    An absent side is two empty fields.
- **The read, in order.**
  1. A read-only session looks the ID up among the gate's records
     (`local_sync::record::owned_identity`; `invalid_input`, R-37).
  2. It resolves a comment's operation to its synchronization through
     `comment_synchronization` (R-5).
  3. It reads the remote record, the newest window's pending step and the
     newest step with a conflict digest, whether a resolution attempt is
     recorded for that step, and the merge metadata its merge recorded.
     - An ID no remote record has, but another operation of the repository
       does (`show_operation`), is `invalid_input`; otherwise
       `operation_not_found`.
     - A remote operation that is no synchronization is `invalid_input`.
  4. Then by the step:
     - **`conflict_pending`.** The library's inspection gives the
       observation and tokens. `next_action` is `resolve` exactly when the
       set is non-empty and every path is `EligibleCanonical`, which is
       `conflict_next`'s rule. Eligibility is that one value for the set.
     - **`resolution_prepared` with written paths.** The same, with
       `retry_resolution` and `historical: true` (fact 86).
     - **The commit prepared.** `resolution_prepared` with `resume`.
     - **No pending step.** `complete` if the record completed;
       `cancelled` with `synchronize` if it was cancelled; else, with the
       conflict's step applied, `resolved` (an attempt is recorded) or
       `repaired` (a restart observed an outside repair), each with
       `resume`.
  5. **When the inspection fails at `conflict_pending`.** A held lease
     or a registry failure is answered as itself (`busy`). Any other
     failure means Git no longer holds the recorded conflict. The read
     then judges an outside repair, under the repository's lease, by
     fact 35 as `reconcile_pending_candidate` applies it:
     - HEAD has exactly the parents `[local, incoming]`;
     - the target is clean, except that the operation's own merge
       metadata may remain if every member matches the recorded digests;
     - `validates_external_integration` accepts the commit.

     Accepted, the answer is `repaired`, `resume`. Otherwise it is
     `repair_refused`, `external_repair`. The clean check compares the
     index with HEAD's tree through a diff. `require_clean_target_with`
     writes the index's tree to compare it, and this read writes nothing.
- **Per path.** Kind and structure come from
  `integration::validate::classify_conflict_entry`, called on each index
  conflict the inspection issued a token for. The `IndexConflict` is
  rebuilt from the token: its path, and each stage's OID and mode.
  libgit2 groups a conflict's stages by path, and classification reads
  only those fields and the blobs, so no second index read is needed.
  Sides are read with the library's `read_synchronization_conflict`, one
  call per path (fact 32). `content_available` is true exactly when it
  returns sides. `ExternalResolutionRequired` from it means none. Paths
  are sorted by raw bytes; tokens keep the inspection's positions.
- **Errors** are placed as `ResolvedRepository::failure` places them: the
  scope names the repository, and `index_unavailable` offers
  `index.rebuild` with its root. `at_root` is private to `read/`, so this
  is a local copy.
- **Exports.** `remote/mod.rs` declares `pub(super) mod conflict_read`
  and re-exports the DTOs and enums. `src/repository.rs` adds them to
  `pub use remote::{…}`. Under `cfg(test)`, it re-exports the three
  crate-private functions for the source-included test targets;
  `display_path` becomes `conflict_display_path`.
  `mutation/mod.rs` re-exports `comment_synchronization` as
  `pub(super)`. `read/mod.rs` and `read/dto.rs` needed no change: the
  types reach `manyhands::repository` beside the other remote types.
- **Schemas.** `conflict.schema.json` and `conflict_path.schema.json`,
  closed. `conflict` is a data root (E-7: `DATA_ROOTS` 52 → 53).
  `conflict_path` is reached through `$ref`.
- **Named edits.** E-7 (the data root) and E-8 (`contract_enumerations`
  gains the seven enumerations of the two schemas).
- **Goldens.** `conflict_show_conflict_pending`, `conflict_show_repaired`
  and `conflict_show_complete` (read goldens, registered in
  `golden::CASES`), written with `MANYHANDS_UPDATE_GOLDEN=1`, read, and
  then compared in the Verify run.

**Cases** (`tests/mutation_remote/conflict_read.rs`, 21, all `c4_conflict_read_`):

| Row or focus | Case |
| --- | --- |
| Conflict pending, canonical regular | `canonical`, with the golden, the token resolving to the library's, and a second read equal |
| Any path noncanonical or structural | `noncanonical_or_structural`: a ticket (sides), a binary file and an executable (no sides) |
| Incompatible closures (R-40, Step 4) | `incompatible_closures`, and the library refuses to resolve the set |
| Markers removed | `markers_removed`: the same read as before |
| Stopped after writing paths | `resolution_prepared` (`ResolutionAfterPathWrite`): the same observation, tokens and sides |
| Commit prepared, ref not moved | `commit_prepared` (`ResolutionAfterCandidatePrepared`) |
| Resolved, not published | `resolved` |
| Resolved, then a local save | `resolved_then_saved`: an item's branch, saved through `save_ticket` |
| Repaired outside | `repaired`, with the golden: the metadata left, then cleaned; then a restart publishes it, and the read is `complete` |
| Staged, or wrong parents | `repair_refused`: staged, one parent, reversed parents. A restart refuses each, and each undone reads as pending again |
| Merge aborted outside | `merge_aborted` |
| Publication cancelled | `cancelled` (an observer cancels the restart) |
| Published | `complete`, with the golden |
| Lease held | `busy` |
| Unknown ID | `unknown_operation` |
| No synchronization | `not_a_synchronization` (a `ticket create`'s operation) |
| A local sync (decision 19) | `local_sync` |
| R-5 | `comment_operation`: by the comment's ID and by the synchronization's |
| R-12 | `writes_nothing`: the registry's bytes, with the WAL checkpointed before and after, unchanged across a live read, an unknown ID and a held lease |
| Review Focus 2 | `hostile_path`: an escape sequence and a newline, escaped once; no raw control byte in `Debug`, JSON or a diagnostic; its token resolves |
| R-11 | `display_path`: the rule pinned byte for byte, and on Linux the same in a read |

The read's `next_action` is checked against the synchronization's own
`conflict.next_action` in five cases. They agree.

**Decisions to review.**

1. **No conflict recorded.** A synchronization that recorded no conflict,
   in any phase, is `invalid_input`: the DTO needs the conflict's stage
   and parents. Its terminal rows (`cancelled`, `complete`) are read for
   a synchronization that met one.
2. **A failed inspection at `ResolutionPrepared`.** It answers the
   inspection's own code (`external_change` or `recovery_required`), as
   the library's restart refuses that state. No table row covers it.
3. **A lock not mirrored.** The repair judgement does not mirror
   `refuse_local_reconciliation_locks`. A Git lock left in the target
   makes the read say `repaired` where a restart answers
   `recovery_required` until the lock is gone.
4. **Platforms.** The hostile-path case returns early on Windows, which
   cannot name such a file. The display-path read runs on Linux only
   (macOS refuses the non-UTF-8 byte). Its pure-function pin runs
   everywhere.
5. **Leases.** Blobs are read for classification outside the lease. The
   lease is taken by every library inspection and side read, and by the
   repair judgement.

**Commands** (each `devenv shell -- cargo … --locked`):

| Command | Result |
| --- | --- |
| `test --all-features --test mutation_remote -- c4_conflict_read` (RED, before the implementation) | did not compile: only the missing read, types and functions |
| `test --all-features --test mutation_remote -- c4_conflict_read` | 21 SSH cases passed (5 min 53 s) |
| `test --all-features --test read_contract` | 72 passed |
| `test --all-features --test mutation_contract` | 44 passed |
| `clippy --all-targets --all-features -- -D warnings` | passed |
| `test --all-features --lib remote` | 477 passed |
| `test --all-features --lib local_sync` | 164 passed |
| `test --all-features --lib integration` | 26 passed |
| `fmt --check` | passed |
