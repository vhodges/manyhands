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
| 3 | `feat: read synchronization conflicts with string tokens`; pre-review fix round 1 `fix: refuse a repair beside a Git lock, and give sides only for item text`; review fix round 2 `fix: give item sides, tell errors from refused repairs, and share the restart's repair rule` | Verify list passed: 21 `c4_conflict_read` SSH cases, 72, 44, clippy; `--lib remote`, `--lib local_sync`, `--lib integration`; fmt; fix round 1: 22 `c4_conflict_read`, 72, 44, 477 `--lib remote`, clippy, fmt; fix round 2: 23 `c4_conflict_read`, 25 `remote_merge_recovery`, 72, 44, 477, 164, 26, clippy, fmt | review fix round 2 applied; re-review pending | Fact 86 holds. See "Task 3: Conflict Inspection Read" |
| 4a | `feat: bind conflict resolution that the same request can finish`; review fix round 1 `fix: let the original request cancel a resolution's publication, and close the review's gaps`; Part A review fix round 1 `fix: close a resolution attempt that wrote nothing, and close Part A review's gaps` | Verify list: 200 of 201 `mutation_remote` SSH cases, the one watchdog failure (`c4_resolution_conformance`) fixed and re-run (1, 47 s); 30 `c4_resolution`; 383, 129, 44, 72, 95, 25 SSH; `--lib` 123, 477, 164, 26; clippy, fmt; desktop check and clippy; fix rounds: see their entries | Part A review fix round 1 applied; re-review pending | Every row of the interrupted-resolution table met; resume's one route for a prepared commit added (decision 18). See "Task 4a: Conflict Resolution Binding" |
| 4b | `feat: resume comment publication and resolved synchronizations`; review fix round 1 `fix: share resolution fixtures, keep an owed child's standing, and pin a fenced comment's resume` | Verify list: 221 `mutation_remote` SSH cases (13 `c4_resume`), no watchdog hit; 383 `mutation_replay`; 44 `mutation_contract`; 123 `--lib mutation`; clippy, fmt; desktop check and clippy; fix round 1: 15 `c4_resume`, 34 `c4_comment`, 37 `c4_resolution`, 383, 183, 123, 477, clippy, fmt, desktop | review fix round 1 applied; re-review pending | Every row of the resume table met but row 6 (Task 5's, P3). Row 8 stopped at a design checkpoint and was ruled option B; Task 14 amends its wording. E-11 applied. See "Task 4b: Resume Routes" |
| 5 | `feat: republish a remotely deleted item branch with consent`; review fix round 1 `refactor: share item sync's fetch and push steps with republish, and close the review's gaps` | Verify list: 249 `mutation_remote` SSH cases (23 `c4_republish`, 16 `c4_resume`), no watchdog hit; 23 `c4_republish`, 16 `c4_resume`, 3 `c4_conformance`, 1 `c4_resolution_conformance`; 183, 129, 44, 72; `--lib` 123, 477, 164, 26; clippy, fmt; desktop check and clippy (`--bin`), `check --features desktop`, and `clippy --all-targets --features desktop` at main's 58; fix round 1: see its entry | review fix round 1 applied; re-review pending | Every row of the republish table met, and the resume table's row 6 (P3). The conformance case is split per row (owner-authorized, beside E-9). See "Task 5: Republish" |
| 7 | `feat: add sync, resume and abandon verbs with a Ctrl-C window`; review fix round 1 `fix: share C3's item read with item sync, and close the review's gaps`; Part B review fix round 1 `fix: keep an inherited SIGINT ignore, add the missing text goldens, and share the binary tests' helpers` | Verify list: `--bin manyhands-cli` 212, `cli` 302, `cli_remote -- c4_interruption` 1 SSH case; clippy, `check --bin manyhands-cli`, fmt; desktop `check --bin manyhands` (guard); fix round 1: the same list, 212, 302, 1 | fix round 1 applied; re-review pending | The spike answered `cancelled`, exit 130, with its effects intact (decision 14). No library change; no `--lib` run (ruling 5). See "Task 7: The Ctrl-C Window And The Sync, Resume And Abandon Verbs" |

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
  (Superseded by the fix rounds below: sides are now given only for a
  path whose kind is an item, whatever its structure.)
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
   `recovery_required` until the lock is gone. (Fixed in pre-review fix
   round 1.)
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

#### Task 3, pre-review fix round 1

The controller ruled on the report's concerns. Concerns 3 and 8 changed
code; 1, 2, 4, 5, 6 and 7 are accepted with no change, and 1 and 2 get
table rows in Task 14. The commit is `fix: refuse a repair beside a Git
lock, and give sides only for item text`, on `ffa627b`.

1. **Concern 3: the lock check is mirrored.**
   - `sync.rs`: `refuse_local_reconciliation_locks` is `pub(super)`, with
     no other change.
   - `conflict_read.rs`: the repair judgement calls it under the
     repository's lease, before it looks at the repair, as
     `reconcile_pending_candidate` does. The judgement runs when the
     inspection fails at `ConflictPending`.
   - A Git lock in the target is then the read's `recovery_required`, the
     restart's own answer, whatever the target holds. The locks are
     `index.lock`, and the locks of HEAD, its log, the branch, the branch's
     log and packed refs. The read never says `repaired` beside one.
2. **Concern 8: sides only for an item's canonical text (R-10).**
   (Corrected in review fix round 2: the structure no longer matters. A
   path whose kind is an item gets its sides whatever its structure,
   `incompatible_closures` included; only a `noncanonical` path gets
   none. The two points below give this round's first rule.)
   - The library's side read is asked only for a path the classifier
     reads as an item's (`kind` is not `noncanonical`) with structure
     `regular`. `content_available` is true only when it returns sides.
   - Any other path gets no sides and `content_available: false`,
     whatever its structure: a non-item UTF-8 file (`binary`), an
     executable, or an `incompatible_closures` ticket.
   - The `conflict_path` schema's descriptions say so.
   - No golden changed: the pending golden's one path is a `regular`
     ticket, and the other two have no paths.

**Cases.**

- `c4_conflict_read_repair_beside_a_lock` (new; 22 cases now). A valid
  outside repair, cleaned up, with a stale `.git/index.lock`, then a stale
  `refs/heads/main.lock`. For each lock, the read is `recovery_required`,
  and the library's restart is `Err(RecoveryRequired)`. With the lock
  removed, the read is `repaired` / `resume`, and a restart publishes the
  repair. Every platform can name both files, so the case runs everywhere.
  - A probe with the check line removed failed this case at the read's
    `unwrap_err`: the read said `repaired`.
- `noncanonical_or_structural` gains `plain.sh`, the script as a 100644
  file. It is `noncanonical` / `binary` with no sides, while the library's
  side read, called directly with its resolved token, returns them.
- `incompatible_closures` asserts no sides (reversed in review fix round
  2: it now asserts all three sides).
- `hostile_path` asserts that the read carries no sides. It still proves
  the token resolves, by the library's side read on the resolved token.

**Commands** (each `devenv shell -- cargo … --locked`):

| Command | Result |
| --- | --- |
| `test --all-features --test mutation_remote -- c4_conflict_read_repair_beside_a_lock`, with the lock check removed (probe) | 1 SSH case failed, at the read's `unwrap_err` |
| `test --all-features --test mutation_remote -- c4_conflict_read` | 22 SSH cases passed (263 s), goldens compared |
| `test --all-features --test read_contract` | 72 passed |
| `test --all-features --test mutation_contract` | 44 passed |
| `test --all-features --lib remote` | 477 passed |
| `clippy --all-targets --all-features -- -D warnings` | passed |
| `fmt --check` | passed |

#### Task 3, review fix round 2

The task review found three Important issues and some Minors, and the
controller ruled. The commit is `fix: give item sides, tell errors from
refused repairs, and share the restart's repair rule`, on `b7dfda9`.

1. **Important 1: sides for any item kind.** The read asks the library
   for sides on every path whose kind is an item (`document`, `ticket`,
   `comment`), whatever its structure. It sets `content_available: true`
   when sides are returned. A `noncanonical` path still gets none. The
   DTO doc, the comment in `live` and `conflict_path.schema.json`'s
   description say so. The earlier text of this entry that gave the
   `regular`-only rule is marked as superseded.
2. **Important 2: failures are not refusals.**
   - `repair_accepted` and `clean` return `Result<bool, ReadError>`.
     `Ok(false)`, which is `repair_refused` / `external_repair`, comes only
     from a decided refusal:
     - foreign merge metadata or state, a conflict, a change or untracked
       file, or an index that is not HEAD's tree (the target is not
       clean);
     - wrong parent count or order;
     - `validates_external_integration` returning `Ok(false)`.
   - Every Git read that fails is `recovery_required`, as the restart's
     `map_err(|_| RecoveryRequired)` maps it. That covers HEAD, the index,
     statuses, the diff, `MergeMetadataDirectory::open` and digests. A
     `validates_external_integration` error is converted as the restart's
     `?` converts it.
   - A configuration that is not valid is `recovery_required`, the
     restart's own answer to it. An unmaterialized target is
     `external_change`, as `inspect_conflict_target` maps it.
   - **Narrowed fallback.** The judgement runs only for the inspection's
     `ExternalChange` (HEAD or the conflict set moved) and
     `RecoveryRequired`. `RecoveryRequired` is kept because
     `conflict_digest` returns it for an index with no conflict, which is
     the repaired, staged, wrong-parent and aborted rows. Any other
     inspection error is answered as itself. The judgement now answers
     its own failed reads as errors, so a `RecoveryRequired` from a cause
     other than a conflict-free index cannot become a refusal.
3. **Important 3: the restart's rule is shared.**
   - `sync.rs`:
     - `RESOLUTION_MERGE_MEMBERS` is `pub(super)`.
     - The clean-target state list is `pub(super) const
       CLEAN_TARGET_FOREIGN_STATE`. The foreign-metadata list is
       `pub(super) const FOREIGN_RESOLUTION_METADATA`, and
       `foreign_resolution_metadata_present` is `pub(super)` and uses it.
     - `require_clean_target_with` now calls the new `pub(super)
       require_clean_target_state`, which runs every check before the
       tree write, then keeps its `write_tree` comparison.
     - `owned_merge_metadata_remnant` keeps its lazy journal read: it
       reads only when metadata is present. It then calls the new
       `pub(super) merge_metadata_remnant(repository, recorded)`, which
       answers `Absent`, `Owned(digests)` or `Foreign` and maps `Foreign`
       to the same `RecoveryRequired` as before.
     - The new `pub(super) merge_metadata_present` is the presence check
       both use.
   - The restart's order and errors are unchanged, and
     `remote_merge_recovery` passes all 25 cases.
   - The read calls `merge_metadata_remnant`, then
     `require_clean_target_state` (`WorktreeConflicted` and
     `WorktreeNotClean` count as not clean; anything else is an error),
     then compares the index with HEAD's tree through
     `diff_tree_to_index`. The copies in `conflict_read.rs` (the member
     and state lists, `present` and the rest of the old `clean`) are
     deleted.
4. **Minors.**
   - `placed` uses `ROOT_ARGUMENT`. In `read/mod.rs` it is now
     `pub(super)`, an export-only change.
   - `cancelled` asserts `Err(SynchronizationError::Interrupted)`.
   - `writes_nothing` asserts each answer: `conflict_pending`,
     `operation_not_found` and `busy`. It also adds a `repair_refused`
     read (the merge aborted outside) inside the byte-compare window, so
     the judgement's lease and reads are proved to write nothing too.

**Cases.**

- `incompatible_closures` asserts `content_available: true`, `base` equal
  to the base source, and `local` and `incoming` equal to the two closed
  sources.
- `c4_conflict_read_invalid_configuration` (new; 23 cases now). An intact
  conflict, and a `.manyhands/config.toml` that does not parse. The index
  still has conflicts. The read is `recovery_required`, not
  `repair_refused`. With the file restored, it is `conflict_pending`
  again.
  - A probe with the old `Ok(false)` restored failed this case at the
    read's `unwrap_err`.
- No golden changed: the pending golden's one path is a ticket, and the
  other two have no paths.

**Commands** (each `devenv shell -- cargo … --locked`):

| Command | Result |
| --- | --- |
| `test --all-features --test mutation_remote -- c4_conflict_read_invalid_configuration`, with `Ok(false)` restored for an invalid configuration (probe) | 1 SSH case failed, at the read's `unwrap_err` |
| `test --all-features --test mutation_remote -- c4_conflict_read` | 23 SSH cases passed (295 s), goldens compared |
| `test --all-features --test remote_merge_recovery` | 25 SSH cases passed (305 s) |
| `test --all-features --test read_contract` | 72 passed |
| `test --all-features --test mutation_contract` | 44 passed |
| `test --all-features --lib remote` | 477 passed |
| `test --all-features --lib local_sync` | 164 passed |
| `test --all-features --lib integration` | 26 passed |
| `clippy --all-targets --all-features -- -D warnings` | passed |
| `fmt --check` | passed |

### Task 4a: Conflict Resolution Binding

The commit is `feat: bind conflict resolution that the same request can
finish`, on `73ecf13`. `origin/main` is still `a372edb`, and
`git merge-tree --write-tree HEAD origin/main` exits 0. No subagent was
dispatched.

**The library (R-14, R-35, R-41).**

- `remote/state.rs`:
  - `remote_resolution_owners(attempt_ulid, request_ulid, input_digest)`,
    values only, with no foreign key, created by its own migration step
    (`migrate_resolution_owners`, `CREATE TABLE IF NOT EXISTS`, the last
    step of `state::migrate`). No audited table, column or trigger
    changes (fact 75), and no object name holds `request` (fact 81).
  - `resolution_view`: one read of a synchronization's conflict steps
    (newest first) and its resolution attempts, each with its phase, its
    candidate, whether its native artifacts are released, and its owner.
    `ResolutionView::unfinished` is the attempt fact 71 speaks of: paths
    written with no candidate, a candidate prepared, or applied with its
    sentinel not released.
  - `record_resolution_owner`, `release_resolution_owner` (only for an
    attempt the domain never recorded), `resolution_owned_by`,
    `unfinished_resolution_request` (the read's `request_id`),
    `resolution_path_records` and `close_superseded_resolution_attempt`.
- `remote/sync.rs`:
  - `finish_resolution_attempt`: an attempt at `CommitPrepared` or
    `Applied` is finished from `remote_resolution_attempts` and
    `remote_resolution_paths` alone (fact 34). The library's tokens are
    rebuilt for `reconcile_resolution_candidate` only: each path is found
    by hashing every path of the candidate's tree against the recorded
    `path_digest` (fact 70), its bytes are the candidate's blob, the sides
    are the recorded blob OIDs (mode `100644`, as an eligible canonical
    conflict's sides are), and the observation is the step's window,
    ordinal, conflict digest and local parent with the live configuration
    digest, which is not stored (fact 31). The reconcile then checks
    window, ordinal, operation, every token digest and the input digest,
    as before.
  - `resolution_request_again`: at `ResolutionPrepared`, the live
    inspection's observation and tokens, each path given the presented
    source whose digest the attempt recorded for it. A changed
    `config.toml` changes the live observation but not the attempt's
    input digest, so the same request finishes it (row 11).
  - `resolution_paths_unchanged` and `supersede_resolution_attempt`
    (decision 18): under the conflict's reservation and the repository's
    lease, the written paths are proved unchanged again, the old attempt's
    `index.lock` is unlinked only when it is exactly the sentinel the
    attempt journaled (identity and digest; a free function per platform,
    because `windows_resolution.rs` is outside C4's library files), and
    the old attempt's rows and its owner row are deleted in one
    transaction. Deleting is what "closed" is: the attempt's evidence
    exists only to resume it, and a row left in `paths_applying` would
    read as unfinished for ever.
- `remote/conflict_read.rs` (ruling P2): `ConflictDto.request_id` names the
  owner of the attempt that has written paths or prepared a commit.

**The binding (`bind/conflict.rs`).**

- `ConflictResolveInput { root, operation_id, observation, resolutions }`
  and `ConflictResolutionInput { token, source }`, with no derives.
  `Command::ConflictResolve => "conflict resolve"`, with the Command
  Predicates table's values (R-32).
- Step 1 in `prepare`: an observation and tokens of R-9's form, at least
  one resolution and no token twice, else `invalid_input`. Then the root,
  then the operation: a local-context sync's ID (looked up first) and an
  operation that is no synchronization are `invalid_input`, an unknown ID
  `operation_not_found`, a comment's ID resolves to its synchronization
  (R-5). The request is placed on the synchronized branch.
- The attempt ID is a ULID from a domain-separated BLAKE3 digest of the
  request ID, the synchronization's ID, the window and the stage ordinal.
  The input digest covers the synchronization's ID, the observation and
  every token and source in token order.
- Step 2, replay first: the request's own attempt is the owner row whose
  attempt is derived from this request and a conflict step of the
  synchronization. A different digest is `request_mismatch` (no effects,
  record deleted). A matching one finishes it: from durable state at
  `CommitPrepared`/`Applied`, with the presented sources at
  `ResolutionPrepared`, straight to the continuation once released, and
  through steps 3 to 5 when the domain never recorded it (row 1).
- Step 3: another request's unfinished attempt with only written paths is
  superseded when they are unchanged (checked read-only before validation,
  done after it), else `original_request_required` with `request.retry`
  naming its request; one with its commit prepared is `recovery_required`
  offering `operation.resume` of the synchronization, then `repo.inspect`.
- Step 4 in the design's order: already resolved is `recovery_required`
  (`repo.inspect`); another observation `external_change`; an
  external-only set `recovery_required` with data
  `conflict.next_action: external_repair`; a wrong count `invalid_input`;
  a token not in the conflict `external_change`. Nothing is written.
- Step 5: the owner row is written, then any supersede, then
  `resolve_synchronization`. A refusal the domain made before it recorded
  the attempt lets the owner row go.
- Step 6 (R-15): the token is looked at once the resolution is
  checkpointed (`cancelled`, final, the synchronization left resolved).
  Otherwise a comment's synchronization is continued by
  `retry_comment_publication_with` with `restart: true` and
  `sync::of_a_comment`; any other by `sync::resumed`. The child `Accepted`
  is built as `resume.rs` builds one, and the `Watch` owner is this
  request (R-35). An earlier call's resolution that another request has
  since published is `already_applied` with the same commit, on a first
  call of the request (row 12).
- Step 7: the data is `ConflictResolutionMutationDto` (`type:
  "conflict_resolution"`). Effects are the resolution's (`written`,
  `committed`, integration `complete`, the resolution commit) with the
  continuation's publication and discovery, and `integration: pending`
  when the publication met a new conflict, which `conflict` names. `ok`,
  `cancelled` and `discovery_pending` (offering `operation.resume`) are
  final; any other stop deletes the record (fact 67). A stop before the
  resolution is checkpointed names only what this call moved the attempt
  to, so a retry refused before any write (row 14) claims nothing.

**`operation resume`, one route (decision 18).** `bind/resume.rs` calls
`conflict::finish_prepared_resolution` before it restarts a pending
synchronization it takes over: a restart refuses to touch a prepared
commit's artifacts (`sync.rs`, `reconcile_pending_candidate`), so row 9
could not be met otherwise. It is the one route 4b's brief also names
("finishing a prepared resolution candidate from durable state"); the
comment synchronization's routes and `original_request_required` for
written paths are left to Task 4b.

**Failure points used** (`grep -n 'FailurePoint::'` over `sync.rs` and
`state.rs`; `state.rs` has none): `ResolutionAfterPathWrite`,
`ResolutionAfterCandidatePrepared`, `ResolutionAfterRefTransition`,
`ResolutionAfterCheckpointObservation`,
`ResolutionBeforeMetadataRetirement`,
`ResolutionAfterIndexLockRetirement` and `ResolutionAfterMetadataCleanup`.
Before the first path write (fact 64), an SQL trigger refuses the first
`remote_resolution_paths` insert. Before publication, the test server
disconnects at its advertisement; during publication it withholds the
push status (`AfterReceivePack`). Ctrl-C is the call's token, cancelled
by the sink at `writing` and at `pushing`.

**Cases** (`tests/mutation_remote/resolution.rs`, 30 `c4_resolution_`):

| Table row | Case |
| --- | --- |
| Before any path is written | `before_any_path_write` |
| After the paths are written | `after_paths_written` |
| After the commit is prepared | `after_commit_prepared` |
| After the ref moves, before publication | `after_ref_moved` (ref transition, checkpoint observation), `after_checkpoint` (the three later points) |
| During publication | `during_publication` |
| Another source under the same request ID | `another_source_is_a_mismatch` (trigger, path write, ref transition) |
| Paths written, first request lost, paths unchanged | `new_request_supersedes_written_paths` |
| Paths written, a path edited by hand | `new_request_after_a_written_path_is_edited` |
| Commit prepared or ref moved: `operation resume` | `resume_finishes_a_prepared_commit` |
| Commit prepared or ref moved: a new request | `new_request_after_commit_prepared` |
| `config.toml` changed | `configuration_changed` (an item's conflict) |
| Published by another request | `published_by_another_request` |
| After success, a new request | `new_request_after_success` |
| Unfinished local-context sync | `unfinished_local_sync` |
| After success, before publication: resume | `resume_before_publication` |
| After publication: resume | `resume_after_publication` |
| Ctrl-C during the resolution | `cancelled_before_publication` |
| Ctrl-C during publication | `cancelled_during_publication` |

Also: Review Focus 3 (`duplicate_token`, `foreign_token`, each with the
source as a probe of the stores and envelopes), `publishes` (golden),
`syntax`, `another_observation` (golden), `wrong_count` (golden),
`external_only`, `operation_lookup`, `local_sync_operation`,
`comment_conflict` (by the comment's ID; the comment is published) and
`conformance`. Task 3's `c4_conflict_read_resolution_prepared` now makes
its attempt through `conflict resolve` and asserts `request_id`.

**The unfinished-local-sync row (R-38).** The gate refuses a local sync
while a remote operation is retained, and every remote reservation while a
local sync is unfinished, so the two cannot be staged in turn (as Task 1
found for its row). The case leaves the local sync in its conflict with
Task 1's helper, sets its two rows' states to read as ended
(`local_context_sync_records.state`, `operation_records.state`) while the
remote conflict is made and its resolution interrupted, puts them back,
and asserts the row; then sets them to ended again and the same request
finishes the attempt.

**Named edits.** E-6 (`command.rs` tests), E-7 (`DATA_ROOTS` 54), E-8
(`conflict_resolution_mutation`'s three enums), E-9 (`HOMES` 34; the
`CONFORMANCE` row), E-10 (six accessors and `repair_data`), E-12 (the
gate's eight tables and `remote_resolution_owners`). **Owner-authorized,
beside E-9** (2026-10-28, "It's okay if you need to split the test"): the
`conflict resolve` row is driven by `c4_resolution_conformance`, and
`every_command_of_this_target_keeps_the_rules_of_the_boundary` drives the
other three rows, with the resolution row added to its `elsewhere`. Each
rule of the row builds a world; the two that need a finished request
(`replay`, `mismatch`) make a conflict in it and resolve it. The first full
run, with a conflict made for every rule, failed that case once at the
100 s watchdog, so the two refusal rules (`refusal`, `consent`) now use a
plain world: they are refused before anything is read.

**Commands** (each `devenv shell -- cargo … --locked`; logs under the
SDD directory's `logs/task-4a-*`):

| Command | Result |
| --- | --- |
| RED: `test --all-features --test mutation_remote --no-run`, with the new tests against the library at `73ecf13` (the changed library files restored from it, then put back) | did not compile: 7 errors, each a missing name (`Mutation::ConflictResolve`, `ConflictResolveInput`, `ConflictResolutionInput`, `ConflictResolutionMutationDto`, `ConflictResolutionMutationType`, `MutationDataDto::ConflictResolution`) |
| `test --all-features --test mutation_remote -- c4_resolution`, `MANYHANDS_UPDATE_GOLDEN=1`, `CI` unset | 29 SSH cases passed (the four goldens written, then read) |
| `test --all-features --test mutation_remote` (goldens compared) | 200 SSH cases passed, 1 failed: `c4_resolution_conformance`, at the 100 s watchdog (3385 s). Every other `c4_resolution` case (29, `after_checkpoint` among them), the 23 `c4_conflict_read` cases and both other conformance cases passed |
| `test --all-features --test mutation_replay` | 383 passed (fact 81's `:400` test among them) |
| `test --all-features --test mutation_bridges` | 129 passed |
| `test --all-features --test mutation_contract` | 44 passed |
| `test --all-features --test read_contract` | 72 passed |
| `test --all-features --test repository_enablement` | 95 passed (E-12's test among them) |
| `test --all-features --test remote_merge_recovery` | 25 SSH cases passed |
| `test --all-features --lib mutation` | 123 passed |
| `test --all-features --lib remote` | 477 passed (`state_tests.rs`' drop tests among them) |
| `test --all-features --lib local_sync` | 164 passed |
| `test --all-features --lib integration` | 26 passed |
| `clippy --all-targets --all-features -- -D warnings` | passed |
| `fmt --check` | passed |
| After the conformance fix (the refusal rules in a plain world; the world boxed for clippy): `test --all-features --test mutation_remote -- c4_resolution_conformance` | 1 SSH case passed, 47 s at load 2 (79 s at load 19, while another session's tests ran) |
| `test --all-features --test mutation_remote -- every_command_of_this_target_keeps_the_rules_of_the_boundary` (the other three rows) | 1 SSH case passed, 72 s at load 3. At load 19 it reached the 100 s watchdog: its nominal 74 s is unchanged by C4's split |
| `clippy --all-targets --all-features -- -D warnings`, `fmt --check` (final) | passed |
| `check --locked --features desktop --bin manyhands`; `clippy --locked --features desktop --bin manyhands -- -D warnings` | passed |

**Recorded for later tasks.**

- Task 4b: `bind/resume.rs` already finishes a prepared commit
  (`conflict::finish_prepared_resolution`) before it restarts a pending
  synchronization other than a comment's. A comment's synchronization is
  still `LeftToInspect` there, and an attempt with only written paths still
  reaches the restart's `RecoveryRequired`: both are 4b's routes. The
  library function is `finish_resolution_attempt`; the owner of an attempt
  is `ResolutionView::attempts[..].owner`.
- Task 5 and Task 13: the remote conformance case's other three rows take
  about 74 s alone, and exceed the 100 s watchdog under heavy machine load.
  Task 5's `item republish` row needs a case of its own as this one has.
- A continuation that stops for an untrusted host answers with the
  resolution's data, which names no host, so `host.approve` is not offered
  (as Task 2b recorded for a comment's child). No table row covers it.

#### Task 4a, fix round 1

The task review needed fixes, with the controller's ruling on each
(`task-4a-fix-1.md`). Commit `fix: let the original request cancel a
resolution's publication, and close the review's gaps`, on `e485caa`.
`origin/main` is still `a372edb`. The new cases were written first, and
six of them failed against the library at `e485caa`
(`logs/task-4a-fix1-red.log`).

- **I1: `request cancel` of the original request reaches the
  continuation** (design step 6).
  - `Watch` gains a `holder`, whose flag it reads after the owner's and
    never clears.
  - The continuation's owner is still the resolve request, which clears
    its own flag (R-35). Its holder is the request that holds the
    operation:
    - `records::operation_holder`, under the synchronization's ID;
    - for a comment's synchronization, under the comment's operation,
      which finds the `comment add` request (fact 79).
  - `sync::continued` is the plain route; `of_a_comment` takes `holder`.
  - The holder's record is not settled by the continuation. Its own next
    call settles it as F2 does: finished, flag cleared, reservation
    released.
  - That call answers:
    - `cancelled` before push intent, and for the comment;
    - after push intent, F2's "not confirmed" (`recovery_required`,
      fact 74). The cancel at `BeforePush` leaves the push intent
      unaccepted, and the holder's call did not ask there.
  - The domain does stop after push intent: at `BeforePush`, with nothing
    pushed.
- **S1: R-14's collision.** Before it records the owner row of an attempt
  with no row, the binding checks `validate_child_id`, and maps a refusal
  as the library maps its own (`recovery_required`). The case plants an
  `operation_records` row under the derived ID (`attempt_for`, re-exported
  for tests only) and asserts that nothing was written.
- **Minors.**
  1. A refused supersede releases the new request's owner row.
  2. `disown` propagates every error but the expected "attempt recorded",
     which `release_resolution_owner` now returns as `Ok(false)`.
  3. `comment_synchronization` returns `Result<Option<_>, ReadError>`.
     - The binding's lookup and continuation, `settle_target` and the
       conflict read propagate the error.
     - `recorded_effects` and the `request.cancel` offer have no error
       channel, and claim nothing on an error.
  4. Every recorded path is checked before a supersede.
     - One that holds its result is unchanged; one `applied` that does not
       is changed.
     - One not `applied` must hold its recorded pre-write image, or the
       answer is `original_request_required`.
  5. Fact 71 is checked again under the conflict's reservation, before the
     attempt is recorded. This is `ResolveSynchronizationRequest::alone()`,
     a `pub(super)` flag that `new` leaves `false`, so
     `remote_merge_recovery`'s direct calls are unchanged.
  6. `published` carries the child's `local_change`.
  7. A synchronization that met no conflict has a case.
  8. A new `Binding::settle_target` hook (no-op by default) runs before the
     request's identity is taken. `conflict resolve`'s target is the
     resolved synchronization's ID, so a comment's ID and its
     synchronization's ID are one intent.
  9. Duplication:
     - `state::synchronization_target`;
     - `Accepted::taking_up`, shared by resume, the continuation and the
       comment binding;
     - the `refusal` closure.
  10. No side's mode is recorded: a one-line comment names
      `classify_conflict_entry`'s rule.
- **Cases**, 7 new and 1 extended, for 37 `c4_resolution_`:
  - `original_request_cancels_before_push_intent`;
  - `original_request_cancels_after_push_intent`;
  - `comment_request_cancels`;
  - `attempt_id_collision`;
  - `refused_supersede_releases_the_request`;
  - `unobserved_path_edited`;
  - `synchronization_without_conflict`;
  - `comment_conflict`, now also the same request by the synchronization's
    ID.
- **Verify** (each `devenv shell -- cargo … --locked`; logs
  `logs/task-4a-fix1-*`). Every `test` target below was `--all-features`.
  - `mutation_remote`, as SSH cases:
    - `c4_resolution`: 37 passed;
    - `c4_conflict_read`: 23 passed;
    - `c4_comment`: 34 passed;
    - `cancel` and `resume`, for the `Watch` and `taking_up` changes: 37
      and 21 passed.
  - `mutation_replay`: 383 passed.
  - `remote_merge_recovery`: 25 SSH cases passed.
  - `--lib remote`: 477 passed. `--lib mutation`: 123 passed.
  - `clippy --all-targets --all-features -- -D warnings` and `fmt --check`
    passed. The first clippy run found one `single_match`, which was
    fixed.
  - `check` and `clippy --features desktop --bin manyhands` passed.

#### Task 4a, Part A review fix round 1

Part A's checkpoint review (Task 6) found gaps in the conflict code. The
controller ruled on each (`task-6-fix-1.md`). The commit is `fix: close a
resolution attempt that wrote nothing, and close Part A review's gaps`, on
`2a7dccd`; `origin/main` is still `a372edb`.

The new cases were written first. Against `2a7dccd`, four failed and the
M3 pin passed (`logs/task-6-fix1-red.log`).

No stop was met. No pre-existing test, no file outside the library scope,
and nothing of `resolve_synchronization`'s write order changed.

- **I1.1: a refusal that wrote nothing closes its attempt.**
  - After any refusal, `reported` asks the library's new
    `close_unwritten_resolution_attempt` to close the attempt.
  - The library closes it under the conflict's reservation and the
    repository's lease, and only when all of these hold:
    - the attempt is `prepared` or `paths_applying`;
    - no write was observed;
    - every path holds its recorded pre-write image;
    - no index or ref-log artifact row exists.
  - It closes the attempt as a supersede does: the rows and the owner row
    are deleted, and the step goes back to `conflict_pending`. The two closes
    share `state::delete_resolution_attempt`.
  - What this covers: the present `index.lock` at
    `ResolutionIndexLock::acquire`, and every refusal before the index
    artifact is recorded.
  - What stays open, as before: the lock-held refusals, the lock-held
    `StaleObservation` among them, and every refusal after a path write.
    No failure point stages the lock-held `StaleObservation`, so it has no
    case.
  - A refusal that wrote nothing now claims no write.
- **I1.2: the escape, pinned.**
  - After a hand edit of a written path, two answers write nothing and leave
    the edit:
    - a new request is `original_request_required`;
    - the same request, with the same input, is `external_change`.
  - With the request's resolution written back, the same request publishes.
- **I1.3: withdrawn by a later ruling.** The global constraint wins: the
  JSON `message` stays `ResultCode::message()`.
  - `8546ed7` named the changed path in that `external_change`'s message,
    and the follow-up commit `fix: keep the result code's message on a
    resolution's refusal` removes it. That commit takes out
    `Answer::message` and everything that set or read it.
  - The escape case now asserts the code's own message.
  - The escape's explanation moves to the CLI's human output (Part B, Task
    9) and to Task 14's Known Limit.
- **M1.** A supersede resets the step to `conflict_pending`. If its new
  request is then refused before its own attempt is recorded:
  - the conflict reads `conflict_pending` and `resolve`, with
    `request_id: null`;
  - a resume meets the conflict and names `resolve`.

  `takes_up` stays true, as for every conflict.
- **M2.** `own_attempt` decides in this order:
  1. an unfinished attempt the request owns;
  2. else the live step's derived attempt;
  3. else the newest step's.

  There is no case: no fixture stages a second-stage conflict after a
  resolved first stage.
- **M3, a pin.** `c4_comment_remote_added_later_resumed`: once the remote
  is added, `operation resume` by the comment's operation publishes the
  comment.
- **M4.** These texts are corrected:
  - `resume.rs`'s "three kinds";
  - `SynchronizationMutationDto`'s doc;
  - `synchronization_mutation.schema.json`'s description, which is
    hand-written.
- **M5.** These fields are gone: `ResolutionView.phase`,
  `ResolutionView.target` and `ResolutionOwner.input_digest`.
  `owner.input_digest` is no longer selected.
- **M6.** `conflict_read.rs` holds the one `field`, the prefixes
  `OBSERVATION_PREFIX` and `TOKEN_PREFIX`, and `resolvable`. The read,
  `conflict_next` and the binding all use them.
- **M9.**
  - A continuation that stops on `remote_branch_deleted` (or
    `merge_required`) offers `[request.cancel]`. It names the request that
    holds the synchronization, and `offering_cancel` keeps the action while
    that request holds the reservation.
  - Why not the resolve request: `request cancel` never reaches it (R-32:
    it has no safe points). So its record goes, as on any other stop, and
    the route runs through the holder's request.
  - In the case, the holder's request is cancelled and then sent once more.
    It answers `cancelled`, the reservation is released, and `item
    republish`'s preview is not `busy`.
- **Cases** (5 new): `c4_resolution_refused_at_a_git_lock`,
  `…_original_request_after_an_edit`, `…_superseded_then_refused`,
  `…_continuation_meets_a_deleted_branch` and
  `c4_comment_remote_added_later_resumed`. The row-8 case's assertions now
  live in a shared helper.
- **Verify.** Each command ran as `devenv shell -- cargo … --locked`, with
  logs in `logs/task-6-fix1-*`. Every case passed at its first run, with no
  watchdog loss, at loads of up to 34.
  - `mutation_remote`, as SSH cases:
    - `c4_resolution`: 41 passed;
    - `c4_conflict_read`: 23 passed;
    - `c4_resume`: 17 passed;
    - `c4_comment`: 35 passed;
    - `merging`: 26 passed.
  - `remote_merge_recovery`: 25 SSH cases passed.
  - `read_contract`: 72 passed. `mutation_contract`: 44 passed.
  - The `--lib` filters:
    - `remote`: 477 passed;
    - `local_sync`: 164 passed;
    - `integration`: 26 passed;
    - `mutation`: 123 passed.
  - `clippy --all-targets --all-features -- -D warnings` and `fmt --check`
    passed.
  - `check` and `clippy --features desktop --bin manyhands` passed.

### Task 4b: Resume Routes

The commit is `feat: resume comment publication and resolved
synchronizations`, on `e4f9348`. `origin/main` is still `a372edb`, and
`git merge-tree --write-tree HEAD origin/main` exits 0. No subagent was
dispatched.

**The routes (R-20), in `bind/resume.rs`.**

- **A comment's publication** (row 1): `Named::Comment`, found first, by
  `found_comment`, under the comment's operation ID or its
  synchronization's (R-5, through `comment_synchronization`, whose errors
  propagate).
  - It applies to a comment whose local row is completed and whose binding
    records its checkpoint, while the child synchronization never reserved,
    or is pending and not fenced, or completed owing its hand-off.
  - A local row that is pending, waits for its registration, or was
    abandoned keeps the existing route. So P7's
    `tests/mutation_confirmation/abandon.rs:1273` still answers
    `recovery_required` with `operation.abandon` (re-run: 1 passed).
  - `publish_comment` first finishes a prepared resolution
    (`finish_prepared_resolution`, Task 4a's call, not moved). It then
    enters the `comment add` request's record, as a synchronization's owner
    is entered (fact 79). Then `comment::resumed` runs
    `retry_comment_publication_with` (`restart: true`) through
    `sync::of_a_comment`, with that request as `holder`.
  - The answer is the child synchronization's own: its code, its effects
    and its standing. The publication is the comment's, and the data is
    `CommentMutationDto`. The resume writes and commits nothing.
  - `kept` (factored out of `synchronize`) maps the standing for both
    routes. Only the synchronization route settles its owner's record.
- **Written paths only** (row 5): `written_paths_only` answers
  `original_request_required` naming the attempt's owner from
  `remote_resolution_owners` (decision 20), with `request.retry` and no
  data. It uses `resolution_view`, `unfinished()` and
  `has_written_paths_only`; no visibility was widened. An attempt no
  request owns is left to inspect.
- **A cancelled operation** (row 8): see below.
- **The gate's routing is untouched** (row 9, fact 9): the local-sync
  calls still run first in `prepare`, `run`, `rerun` and `reenter`.
- `takes_up` is true for a comment's publication.

**Other files.**

- **`bind/comment.rs`:** `resumed`.
- **`bind/conflict.rs`:** `attempt_phases`.
  `finish_prepared_resolution` now reads `resolution_view` only when an
  attempt is `candidate_prepared` or `applied`. Without the guard, a remote
  row that the view's whole-row read rejects (`mutation_replay`'s
  fixture-inserted rows) made every resume that reached it
  `internal_error`. `written_paths_only` uses the same guard for
  `paths_applying`.
- **`dto.rs`:** `from_stored`'s `OperationResume` arm for `type:
  "comment"`, after the gate's `local_context_sync` arm, which stays first
  (fact 49).
- **`schemas/v1/comment_mutation.schema.json`:** the description names
  `operation resume` too (description only).
- **`tests/mutation_remote.rs`:** mounts the new module, and checks a
  resume's `comment` and `local_context_sync` data against their schemas by
  tag.

**The cases** (`tests/mutation_remote/resume.rs`, 13 `c4_resume_`):

| Row | Case(s) |
| --- | --- |
| 1 comment publication | `comment_publication` (both IDs, replay, the comment's own request after) |
| 2 `identity_required` | `comment_identity_required` |
| 3 resolution checkpointed, outside repair | `resolution_checkpointed`, `outside_repair` |
| 4 commit prepared or ref moved | `prepared_commit` (a guard: Task 4a's route) |
| 5 written paths only | `written_paths_only` |
| 6 `item republish` | Task 5's (ruling P3) |
| 7 conflict pending | `conflict_pending` |
| 8 cancelled | `cancelled` (a comment, both IDs), `cancelled_synchronization` (nothing owed), `cancelled_with_work_owed` |
| 9 local-context sync | `local_context_sync` (Task 1's helper; the gate's whole answer pinned, and the same against BASE's library) |
| Step 1, `list_operations` | `listed` |
| Review Focus 2 | `hostile_path` (a canonical document at `docs/hostile-\x1b[31mred\nline.md`; the resolution commit changes that path alone from either parent; the worktree is clean) |

**Row 8: a design checkpoint, ruled.**

- **What the code answered.** F2 answers `already_applied` for a
  synchronization cancelled with nothing outstanding. That covers a Ctrl-C
  before the push, both for `comment add`'s child and for a plain
  `repo sync`: the row is `cancelled`, with `index_pending` and
  `reconciliation_required` both 0. A pre-existing test pins it:
  `a_resume_of_a_remote_operation_that_has_ended_is_a_no_op`.
- **The row says** `recovery_required`, "as F2".
- **The ruling was option B.**
  - A comment's cancelled publication answers `recovery_required` with
    `[repo.inspect]` under either ID (`found_comment`).
  - Every other cancelled operation answers as F2, unchanged:
    `already_applied` when nothing is owed, `recovery_required` when work
    is owed.
  - The pinned test is not edited.
- **Recorded for Task 14:** amend row 8's wording to this answer (and fact
  9's "a cancelled one is left to inspect"). The design was not edited here.
- The evidence is in `logs/task-4b-row8-evidence.log`.

**E-11.**

- The test is renamed
  `a_resume_of_a_comment_s_synchronization_is_the_comment_s_publication`
  (ruled accepted).
- **The fixture** is now a real `comment add` checkpoint. The old one was a
  binding whose checkpoint OID was not in Git, so it could not show
  publication.
- **The read assertions are unchanged.**
- **The resume part** now asserts the comment's publication: its data,
  pending on its synchronization; `blocked`/`recovery_required`; publication
  `pending` and nothing written; no record; the row unchanged, with no
  remote to publish to.

**TDD.**

- RED: `logs/task-4b-red.log` holds the first draft.
- `logs/task-4b-red-final.log` holds the final cases, with the library
  checked out at BASE: rows 1, 5, 8 and the list case failed, as did the
  E-11 test.
- Row 8 failed at the checkpoint until the ruling. Its two F2 variants were
  added with the ruling, as guards.

**Verify** (each `devenv shell -- cargo … --locked`, `--all-features` for
tests; logs `logs/task-4b-final-*` and `logs/task-4b-verify-*`):

- `mutation_remote`: 221 SSH cases passed, 0 failed (3770 s), 13 of them
  `c4_resume_`. No case hit the 100 s watchdog, so the conformance case
  needed no split.
- `mutation_replay`: 383 passed. P7's `mutation_confirmation` case: 1 passed.
- `mutation_contract`: 44 passed. `--lib mutation`: 123 passed.
- `clippy --all-targets --all-features -- -D warnings` and `fmt --check`
  passed.
- `check` and `clippy --features desktop --bin manyhands` passed.

**Deferred or for review (rulings).**

- Concerns 3 and 4 were accepted: the `finish_prepared_resolution` guard
  and the E-11 rename and fixture.
- Concern 2 is deferred to the final review: the read's `next_action` is
  still `None` for a comment's synchronization (`read/status.rs`, outside
  C4's files).
- Concern 6 is deferred to the final review: a re-entered resume of a
  comment, and a host stop, answer with no comment data.
- Concern 5 goes to the reviewer: `takes_up` now keeps `operation.resume`
  on a comment's or a resolution's `discovery_pending` result.

#### Task 4b, fix round 1

The commit is `fix: share resolution fixtures, keep an owed child's
standing, and pin a fenced comment's resume`, on FIX_BASE `14a60a8`
(`task-4b-fix-1.md`). There is no ticket comment this round.

- **I1, shared fixtures.**
  - `resolution.rs`'s `world`, `remote_commit`, `fresh`, `attempts` and
    `candidate` are `pub(super)`.
  - Its `Conflict::resolutions` and `Conflict::step` now delegate to new
    free functions `resolutions_at` and `conflict_step`.
  - `resume.rs` imports all seven. Its path-generic `Conflict` delegates
    `resolutions` and `step` to them, and calls `attempts` and
    `candidate`. Its copies are removed.
- **Minor 1.** `comment.rs`: the comment on the no-call arm now says that
  `LocalRecoveryRequired` is reached (the receipt's store failed, or the
  row is still pending after the retry's repair). The mapping is
  unchanged.
- **Minor 2.**
  - `state::resolution_attempt_phase` is now `pub(in super::super)`.
  - `conflict::attempt_phases` returns `ResolutionAttemptPhase`s parsed by
    it.
  - `finish_prepared_resolution` and `written_paths_only` match on the
    enum: there is one source of the vocabulary.
- **Minor 3.** `comment::resumed`'s arm for a child that succeeded without
  holding the comment keeps an `Owed` child standing (a hand-off still
  owed). Any other is `Final`, as Task 2b's ruling has it. No case stages
  it: it needs a remote that ends without the comment.
- **Minor 4.** New cases, for 15 `c4_resume_`:
  - **`c4_resume_fenced_comment`.** A comment's publication held for an
    unlockable key, then fenced by the key's selection changing. A resume
    by the comment's ID and by the synchronization's is
    `external_change`, `blocked`, with no effects, data or record.
    Against the pre-4b library (`e4f9348`), the comment's ID answered
    `already_applied` (`logs/task-4b-fix1-red.log`).
  - **`c4_resume_comment_hand_off`** (concern 5). The fixture is the root
    resume test's `fail_handoff` trigger on refresh completion.
    - `comment add` publishes and answers `discovery_pending`, offering
      `operation.resume` of the comment's operation.
    - While the trigger stands, the resume says the same.
    - Without it, the resume does the hand-off: `ok`, with no connection
      and no push. The comment's own request is then `ok`.
    - Against `e4f9348`, the offer was `request.retry`.
  - Both pass at FIX_BASE: they pin behaviour 4b introduced.
- **Minor 5.** `resume.rs`'s module doc is re-wrapped.
- **Deferred, as ruled:** the hostile-path case on Windows, and the
  remaining coverage gaps.
- **Verify** (each `devenv shell -- cargo … --locked`; logs
  `logs/task-4b-fix1-*`):
  - `mutation_remote`: `-- c4_resume` 15, `-- c4_comment` 34 and
    `-- c4_resolution` 37 SSH cases passed.
  - `mutation_replay`: 383 passed. `mutation_confirmation`: 183 passed.
  - `--lib mutation`: 123 passed. `--lib remote`: 477 passed.
  - `clippy --all-targets --all-features -- -D warnings` and `fmt --check`
    passed.
  - `check` and `clippy --features desktop --bin manyhands` passed.
  - The full `mutation_remote` run is Task 5's (the round says so).

### Task 5: Republish

The commit is `feat: republish a remotely deleted item branch with consent`,
on BASE `25d582e`. `origin/main` is still `a372edb`, and `git merge-tree
--write-tree HEAD origin/main` exits 0. No subagent was dispatched.

**The domain (decision 11, R-18), in `remote/sync.rs`.**

- **One classification.** The body of `synchronization_push_absence_boundary`
  moved, unchanged, into the free function `push_absence`. The boundary now
  calls it with its own generation, target and operation. The function also
  gives the commit of the newest proven publication.
- **The reservation-free read.** `republish_evidence` reads with no
  reservation and no connection. The push classification is asked first, as
  `item sync` asks it:
  - `Ambiguous` or `Unknown` is that classification;
  - a `remotely_deleted` row is `Fetched`, with its `last_advertised_oid`;
  - a row a fetch last found present is `None`;
  - with no row, `Deleted` is `Pushed`, with the newest proven publication.
  Only `Fetched` and `Pushed` give a preview.
- **The push-only path.** `republish_context`:
  - checks the consent before it reserves: the branch, the stored evidence
    and `last_published`, and the local tip;
  - reserves as a synchronization of the context (fact 78), with every check
    of the reservation;
  - fetches, and proves the branch absent at the fetch endpoint and at the
    push endpoint;
  - pushes exactly `local_oid` through the legacy checkpoints (`PushPrepared`,
    `PushReturned`, `PushVerified`), with no push-absence boundary, no
    publication attempt and no clean plan;
  - classifies the publication.
- **Restart.** A same-request restart is reconciled with
  `reconcile_synchronization` against the push endpoint: a landed push is
  verified and not pushed again.
- **Refusals that end the operation.** An `ExternalChange` or `PushRejected`
  before any push ends the operation as a cancellation does, which lets the
  reservation go. With an earlier call's push unaccounted for, an
  `ExternalChange` is `RecoveryRequired` instead, and the operation stays.

**The binding (R-19), `bind/republish.rs`.**

- It is a confirmed binding with `operation abandon`'s shape.
- `preview` reads the stored evidence (fact 55). Its observation digest binds
  the branch, the remote, the deletion, `last_published` and the local tip.
- `row` answers a fenced operation finally, and gives a begun one to the
  domain with the consent its request was accepted with: its recorded
  position.
- The domain call goes through `sync::republished`, a synchronization's
  binding with `Session::republish` in place of `Session::synchronize`. The
  `Watch`, the effects and the standing are unchanged.
- `spent` turns `external_change` or `push_rejected` into a finished record,
  when the operation ended or never began. A transport stop keeps its record.
- A lifecycle-closed ticket is `ticket_closed` (decision 17), at `prepare` and
  at execution, through `ticket_guard::require_ticket_open`.

**Resume (ruling P3).** `bind/resume.rs`'s `republishing` routes a remote
operation an accepted `item republish` record holds to `original_request_required`,
naming that request (fact 78). It applies to a pending row and to one that owes
its hand-off. The new `records::operation_holder_of` finds the holder by
command. The gate's routes, a comment's publication, written paths only and
`finish_prepared_resolution` are untouched.

**Inventories.**

- `Command::ItemRepublish`, with the Command Predicates table's values (R-32).
- `Mutation::ItemRepublish`, `ItemRepublishInput` (`Clone`),
  `ItemRepublishPreviewDto`, `PreviewDetailsDto::Republish` and
  `PreviewEffect::BranchRepublished`.
- `from_stored` gains the republish's host data, and `recorded_effects`
  reads a republish as a synchronization.
- The desktop arm shows "Branch", "Last published" and "Local commit"
  (decision 10). The CLI gains its two `human_admin` arms.
- New schema `item_republish_preview`. `preview_effect` gains
  `branch_republished`, and `synchronization_mutation`'s description names
  `item republish`.

**Named edits.**

- **E-5.** `an_external_change_to_a_confirmed_command_is_offered_a_new_preview_or_an_inspection`
  gains the arm for a confirmed command that stops at safe points:
  `["request.retry", "repo.inspect"]` when held. The rule is unchanged.
- **E-6.** `the_stopping_commands_and_their_stages_and_confirmation` takes
  `item republish` as stopping, with an operation, `Connecting`, and the only
  stopping command that needs confirmation, plus its other predicates.
  `the_published_names...` and
  `the_commands_whose_caller_must_first_be_shown_need_confirmation` gain it.
  It was not renamed again.
- **E-7.** `DATA_ROOTS` gains `item_republish_preview.schema.json` (55). No new
  code, so the two code lists are unchanged.
- **E-8.** `contract_enumerations` gains `item_republish_preview.schema.json`
  `details.kind` (`ItemDtoKind`).
- **E-9.** `HOMES` gains `item republish` (Remote, 35), `CONFORMANCE` its
  row, and `preview_schema` the republish schema.
- **Owner-authorized: the conformance split (2026-10-29), beside E-9.**
  - `every_command_of_this_target_keeps_the_rules_of_the_boundary` is
    replaced by one case per row: `c4_conformance_item_sync`,
    `c4_conformance_repo_sync` and `c4_conformance_comment_add`.
  - `item republish`'s row is `republish::c4_republish_conformance`.
  - `c4_resolution_conformance` now calls the same helper,
    `conformance_row`, which names every other row of `CONFORMANCE` and
    `host::CONFORMANCE` as elsewhere. So every row is still checked against
    the target's whole set.
  - `RESOLUTION_ROW` is gone. Task 14 notes the split in the design.

**The cases** (`tests/mutation_remote/republish.rs`, 23 `c4_republish_`; and
`resume::c4_resume_republish`):

| Row | Case(s) |
| --- | --- |
| 1 deletion recorded by a fetch | `preview_fetched` (golden `item_republish_preview`) |
| 2 push-absence evidence `Deleted` | `preview_pushed` |
| 3 `Ambiguous` or `Unknown` | `preview_ambiguous`, `preview_unknown` |
| 4 recreated between fetch and push | `recreated_before_the_push` |
| 5 stopped after the push, then `operation resume` | `resumed_after_the_push` |
| 6 no deletion evidence | `preview_noop` (never published, present, not yet observed; golden `item_republish_preview_noop`) |
| 7 lifecycle-closed ticket | `ticket_closed` (golden `item_republish_ticket_closed`) |
| 8 nothing changed | `confirmed` (golden `item_republish`) |
| 9 primary moved | `primary_moved` |
| 10 context an ancestor of primary | `ancestor_of_primary` |
| 11 local branch moved | `local_moved` |
| 12 branch reappeared | `reappeared` (golden `item_republish_external_change`) |
| 13 busy | `busy` |
| 14 unfinished local sync | `local_sync_unfinished` (Task 1's helper) |
| 15 transport, unlock or host stop | `transport_stop`, `unlock_stop`, `host_stop` |
| 16 replay | `replayed` |
| 17 `item sync` after | `item_sync_after` |
| 18 Ctrl-C during execution | `cancelled` |
| 19 Ctrl-C at the consent prompt | `cancelled_before_execution` (the library's analog; the CLI prompt is Task 12b's) |
| Resume table row 6 | `resume::c4_resume_republish` |
| Conformance | `c4_republish_conformance` |

**TDD.** RED: the cases and the named edits, with nothing implemented, failed
to compile with 6 errors, all missing names (`logs/task-5-red.log`). The
first run passed 17 of 23. Five failed only because their goldens were not
registered, and `preview_unknown` because a key cannot be registered twice
(`logs/task-5-iter1.log`). The goldens were then written, read and committed
(`logs/task-5-golden.log`).

**Verify** (each `devenv shell -- cargo … --locked`, `--all-features` for
tests; logs `logs/task-5-*`):

- `mutation_remote`: 249 SSH cases passed, 0 failed (4433 s), no watchdog hit; on the final code, so it can serve the Part A checkpoint (`logs/task-5-final-remote.log`).
- Focused runs: `-- c4_republish` 23, `-- c4_resume` 16, `-- c4_conformance` 3
  and `-- c4_resolution_conformance` 1 passed.
- `mutation_confirmation` 183, `mutation_bridges` 129, `mutation_contract` 44
  and `read_contract` 72 passed.
- `--lib mutation` 123, `--lib remote` 477, `--lib local_sync` 164 and
  `--lib integration` 26 passed.
- `clippy --all-targets --all-features -- -D warnings` and `fmt --check`
  passed.
- Desktop (ruling P6):
  - `check --features desktop --bin manyhands` and `clippy --features desktop
    --bin manyhands -- -D warnings` passed;
  - `check --features desktop` passed;
  - `clippy --all-targets --features desktop -- -D warnings` shows main's 58
    errors exactly, all in `bin "manyhands" test`, none in new code.

**For review** (also in the report):

- **`push_rejected` is finished.** A push the remote refused ends the
  operation and finishes the record, as `external_change` does. The design's
  row says only "push_rejected, exit 3; nothing pushed". Held, the same
  request would find the recreated branch and could never go on.
- **A refused republish's operation reads `cancelled`.** It is ended as a
  cancellation ends one, since no new phase may be added (fact 75).
- **Push-only evidence cannot tell present from deleted.** A branch published
  and never fetched since gives a `Pushed` preview whether or not it is
  there. Execution then refuses `external_change`, with nothing pushed. So
  does a second preview right after a republish, until a fetch records the
  branch again.
- **Busy.** The envelope offers `request.retry` for the republish. Naming the
  refused `item sync` request is human output, for Part B.
- **The no-op preview** carries no action. "Run `item sync` first" is the
  CLI's to print, since no recovery action names a synchronization.
- **Row 15's host stop** is `host_replacement_required`. A host that is
  already pinned cannot be `host_approval_required`, which is the code for
  an unpinned one.

#### Task 5, fix round 1

The commit is `refactor: share item sync's fetch and push steps with
republish, and close the review's gaps`, on FIX_BASE `296a6c8`
(`task-5-fix-1.md`). There is no ticket comment this round.

- **I1: one set of steps, no copies.** Three private helpers in
  `remote/sync.rs` are now called by both item sync and the republish path,
  over a `Steps` struct (the request, the reservation, the plan, the
  configuration and the primary branch):
  - `open_synchronization` covers the canonical root, the other-root and
    target checks, and the replay of an operation with its outcome, a
    cancelled one or a stopped one. It returns `Opened::Replayed` or
    `Opened::Taken` (boxed, for clippy).
  - `fetch_and_observe` runs from `BeforeFetch` to `AfterFetch`, and returns
    `Fetched { primary, context, changed, repository }`. The caller refuses
    `changed`.
  - `push_and_verify` runs from the `PushPrepared` intent to the
    `PushVerified` checkpoint. It keeps item sync's rationale comment for
    recording `PushReturned` before the boundary.
- **Item sync's code moved verbatim.** Each helper's body was compared with
  the code item sync had at `296a6c8`, whitespace and the renamed locals
  aside: it is the same, in the same order. The one addition is
  `.map_err(after)` on the post-push local inspection and advertisement
  read. Item sync passes `AfterThePush::AsItIs`, for which that is the
  identity.
- **The republish's difference is now explicit.** It passes
  `AfterThePush::Unverified`, which maps those two reads through
  `unverified_push_error`.
- **Republish now opens as item sync does.** `republish_replayed` is gone. A
  restart whose reservation is a replay is `RecoveryRequired`, as item
  sync's is; the binding reads the row as before.
- **Minor 1.** `accepted_consent` passes `last_published: None`. The domain
  checks it only before an operation begins, so the swallowed
  `republish_evidence` read is gone.
- **Minor 2.** `records::operation_holder` takes `command: Option<&str>`,
  and `operation_holder_of` is gone. The callers in `operation_owner`,
  `bind/conflict.rs` and `bind/resume.rs` (two) pass `None`;
  `republishing` passes `Some("item republish")`.
- **Minor 3.** New case `c4_republish_second_preview` pins the Known Limit:
  - after a republish the fetch row is still `remotely_deleted`, so a
    preview is offered again;
  - executing it is `external_change`, finished, with nothing pushed;
  - its fetch recorded the branch, so the next preview is the no-op.
- **Minor 4.**
  - `c4_republish_confirmed_from_push_evidence`: a `Pushed` preview,
    executed, is `ok`, with exactly the local commit created once and
    nothing merged.
  - `c4_republish_reappeared` now asserts the operation's phase is
    `cancelled`.
  - `resume::c4_resume_republish_hand_off` covers the owes-work route. It is
    staged with the existing `FAIL_HAND_OFF` SQL trigger, so no failure
    point is added. The republish is `discovery_pending`, offering its own
    request through `offering_resume`. A resume is
    `original_request_required`, naming that request. With the trigger
    dropped, the same request does the hand-off with no connection, and a
    later resume is `already_applied`.
- **Minor 5.** `command.rs`'s 108-character doc line is rewrapped.
- **New cases first.** The four cases pin behavior `296a6c8` already had:
  the Known Limit, the push-evidence path, the ruled end state and the
  owes-work route. None can fail against the code under review, and none
  was run against it.
- **Verify** (each `devenv shell -- cargo … --locked`; logs
  `logs/task-5-fix1-*`):
  - `mutation_remote`: `-- c4_republish` 25 (the first run of this round was
    24, with the conformance case at the 100 s watchdog under a load average
    of 7 to 15; its world is now the lightest a preview is given in: the
    branch published and then deleted, with no synchronization since;
    alone it took 79 s with a compile); `-- c4_resume` 17;
    `-- c4_resume_republish` 2; `-- merging` 26; `-- push` 38;
    `-- c4_comment` 34.
  - `remote_merge_recovery` 25, `comment_publication` 41,
    `mutation_confirmation` 183.
  - `--lib` remote 477, local_sync 164, integration 26 and mutation 123.
  - clippy, fmt, and the desktop `--bin` check and clippy passed.
  - The whole target is the Task 6 gate's, as ruled.

### Task 6: Part A Checkpoint

The controller ran the checkpoint and its review, at `74be902`.
`origin/main` is still `a372edb`, and `git merge-tree --write-tree HEAD
origin/main` exits 0.

- **The library scope (Step 2), on `2a7dccd`.**
  - The protected-file diff exits 0.
  - The `grep -v -E` filter prints nothing, and it prints all four probe
    lines, so it can fail.
  - `src/repository.rs` touches only:
    - the re-exports, including the `cfg(test)` ones;
    - `checkpoint_comment_locally`'s delegation;
    - `checkpoint_comment_locally_with`'s body.
  - The fix rounds after it changed no file outside the filter.
- **The gate, on `74be902`.** Each command runs as `devenv shell -- cargo
  … --locked`. The logs are `logs/task-6-gate-*`.
  - `check --all-features`, `fmt --check`, `clippy --all-targets
    --all-features -- -D warnings`, `check --features desktop`, `check --bin
    manyhands-cli` and `check-cli-no-gpui.sh` pass.
  - `clippy --all-targets --features desktop` fails with exactly main's 58
    errors, all in D-track files under `src/desktop/`, and none in a C4 file
    (Ruling P6).
  - `--lib`: 1445 passed (28 min).
  - `--bin manyhands-cli` 188, `cli` 290, `mutation_bridges` 129,
    `mutation_confirmation` 183, `mutation_contract` 44, `mutation_replay`
    383, `read_contract` 72.
  - `cli_remote` 31, `comment_publication` 41, `mutation_remote` 257 (every
    case, with no watchdog loss), `remote_merge_recovery` 25. These took 78
    min.
- **The independent review (Step 3)** was by opus, over `da83fd3..2a7dccd`,
  adversarial on decision 13's mechanism and on R-18
  (`task-6-review-report.md`).
  - **Findings:** 0 Critical, 1 Important (I1), 9 Minor.
  - **Republish held.** It had no findings, and the `2a7dccd` extraction
    left item sync's steps unchanged.
  - **I1.** An attempt with written paths, one of them later edited by hand,
    cannot be finished by any request.
    - Ruled: it stays a Known Limit. The library's refusal to overwrite a
      hand edit is the gate's rule, in a file outside C4's scope.
    - A refusal that wrote nothing now closes its own attempt, so the wedge
      cannot be reached without a write.
    - A case pins the escape: restoring the request's resolution lets the
      same request publish.
    - The custom message ruled at first was withdrawn, because the Global
      Constraint keeps `ResultCode::message()`. The escape goes to Task 9's
      human output and to Task 14's Known Limit.
  - **Minors fixed:** M1, M2 (no stageable case), M4, M5, M6 and M9
    (`[request.cancel]` names the holder's request). M3 is pinned.
  - **Carried:** M7 to Task 14. M8 is deferred to the final review.
  - **The fix round** is `8546ed7` and `74be902`; see "Task 4a, Part A
    review fix round 1" above.
  - **Its scoped re-review** (opus) was clean. Three Minors are deferred to
    the final review's fix dispatch, so this gate stays valid:
    - the step reset is not conditioned on the deleted attempt's phase (the
      M8 class);
    - the withdrawn message's scaffolding remains;
    - two outcome docs.
- **Task 14 carries added here:**
  - I1's corrected Known Limit (design line 2045);
  - a hand edit after a lock-held refusal also wedges until undone;
  - M3's row (design line 938);
  - M7's owner rows on registration removal;
  - design step 2's wording after M2.
- **Part B carries added here:**
  - Task 9's escape line for a `historical` conflict's `external_change`;
  - `conflict resolve`'s `remote_branch_deleted`, which takes 2b's three
    `next:` lines, naming the holder's request.
- **Ticket comment** `01M659P4CFQN8ZCFEXGS77Q8H7`, a reply to
  `01M64AGF41CC4ZXHNQV6QZHQ7P`. It also corrects
  `01M62KA2F519YGJ71XADTDX9N7`'s statement that `remote_branch_deleted`
  finishes a `comment add` record: under Task 2b's concern-1 ruling, the
  record stays `Owed`.

### Task 7: The Ctrl-C Window And The Sync, Resume And Abandon Verbs

The commit is `feat: add sync, resume and abandon verbs with a Ctrl-C window`,
on BASE `970cd40c`. `origin/main` is still `a372edb`, and `git merge-tree
--write-tree HEAD origin/main` exits 0. No subagent was dispatched. Only
`src/bin/manyhands-cli` and tests changed: no library file, so no `--lib`
target ran (controller ruling 5).

**The window (R-21, R-36), `interrupt.rs`.**

- `pub(crate) fn during<T>(work: impl FnOnce(&CancellationToken) -> T) -> T`.
  The token sits in an `AtomicPtr` slot. The handler only does atomic loads
  and stores: the run's first press cancels the token and latches; a later
  one ends the process.
- Unix (`interrupt/unix.rs`): `sigaction(SIGINT)` with `SA_RESTART`. A second
  press restores `SIG_DFL` and `raise`s, so the process dies by `SIGINT` when
  the handler returns.
- Windows (`interrupt/windows.rs`): `SetConsoleCtrlHandler`. The first
  `CTRL_C_EVENT` returns `TRUE`, a later one `FALSE`; other events go to the
  default handler. It is not compiled here: there is no Windows target in the
  Devenv toolchain. It stays the owed native check (Owed native checks, item 1).
- Teardown, on return or unwind (a drop guard): it restores the disposition,
  clears the slot, waits for any handler still running on another thread, and
  then drops the token.
- One latch per run: the press count is process-wide. `reset()` and
  `latched()` are what `mutate` uses, and a window opened while latched starts
  cancelled.

**The wiring, `mutate.rs`.**

- The `Boundary` impl runs `RepositoryService::execute` inside `during` for
  `ItemSync`, `RepoSync`, `ItemRepublish`, `CommentAdd`, `ConflictResolve` and
  `OperationResume` (`stops_on_ctrl_c`). Every other mutation gets a fresh
  token, as before.
- `Boundary`, `Plan` and `FakeBoundary` are unchanged, and so is
  `mutate_tests`.
- `run`'s body moved, unchanged, into `drive`, which returns
  `Driven::Answered(Output)` or `Driven::Executed { envelope, supplied }`.
  `run` renders the latter as before. The collaboration verbs call `drive`, so
  that human output can read beside the envelope (see the supersede note
  below).
- `drive` resets the latch on entry, and the retry loop stops once a press is
  latched. C2's retry still never follows `cancelled` (R-23).

**The verbs, `collaboration.rs`.**

- `item sync`, `repo sync` and `operation resume` are unconfirmed, and run with
  `Plan { identity_prompt: false, retry: true }`.
- `operation abandon` is confirmed, and uses C2's flow: `--prepare`,
  `--confirm`, or consent at a terminal.
- `item sync` reads the kind with one `show_item` before it builds the request
  (R-2). The read's refusal is the verb's, with its request ID and no effect,
  as C3's fill read gives it.
- The catalog flips all four to `Available("C4")`, and adds `pub const
  OPERATION_ID`, which `operation show` now uses unchanged. Each verb's
  options are `--repo`, its selector, `--request-id`, `--prepare` and
  `--confirm`, as C3's verbs that take no input have them. `--input` is a
  usage error.
- E-1: `help_root.json` and `help_root_text.txt` were regenerated. The diff
  changes only the four verbs' availability, repository rule and options.

**Human output, `human_collaboration.rs`.** Every result keeps C2's lines.
C4's lines are added by the data and the code:

- **A conflict** (`SynchronizationConflictMutationDto`, and a `conflict` in a
  comment's or a resolution's data):
  - `conflict: operation <id> stopped on a conflict; next action: <resolve|external_repair>`;
  - `next: manyhands-cli conflict show --repo '<root>' --operation-id <id>`.

  C2's lines for the recovery actions follow. A resume of a conflict-stopped
  synchronization (controller ruling 3) prints the same lines.
- **`remote_branch_deleted`** (controller ruling 1): the `request.cancel` the
  library offers is printed as three lines, in place of C2's one line:
  1. `manyhands-cli request cancel --request-id <id>`;
  2. the same request again: `manyhands-cli item sync --repo '<root>' --id
     <item> --request-id <id>`. When the request is not the envelope's own,
     whose command cannot be named, the line is `send request <id> again with
     its original command and input`;
  3. `manyhands-cli item republish --repo '<root>' --id <item> --prepare`.

  A note says why: the refused request keeps its reservation until it is
  cancelled, and sending it again then ends it and releases the reservation.
  `branch_deleted(root, item, request, again)` is public in the module for
  Task 8's `comment add` and Task 9's `conflict resolve`.
- **`original_request_required`** (controller ruling 2): a `request.retry`
  naming another request is printed as `next: send request <id> again with
  its original command and input`. For a local sync, the line names
  `required_request_id` once.
  - The supersede note is `note: a new conflict resolve may supersede its
    resolution while the paths it wrote are unchanged.`
  - It is printed only when the conflict read (`show_conflict`) of the resumed
    operation reports `historical: true`, with `request_id` equal to the named
    request (R-39, R-35).
  - The read happens in human mode only, after `execute`. A read that fails
    adds nothing. JSON passes through unchanged.
- **The gate's `LocalContextSyncMutationDto`** (R-37): `state:`, `reason:` and
  `next action:` lines, in place of C2's field list. No local verb is named.
  The tests render the gate's goldens `local_sync_*.json`.
- Roots in `next:` lines are single-quoted, with controls escaped by C3's
  `field`, and a single quote written as `'\''`.
- Progress is C2's `HumanProgress`, on stderr, unchanged.

**The spike (decision 14): `c4_interruption_held_transfer`.**

- **Setup.**
  - `World` and `CliEnv`; the host is approved through the library, as
    `unlock.rs`'s `approve` does.
  - The world's key is passphrase-protected, so a `--non-interactive` call
    would answer `unlock_required` before it connects. The case therefore
    registers and selects an unprotected copy of the same key, decrypted with
    the fixture passphrase. Its lines are added to the output probes.
  - The item is saved locally and never published. A peer pushes a commit
    with a 1.6 MB random payload to the remote's `main`.
- **The run.**
  - `hold_transfer_after(131072)` holds `item sync`'s fetch.
  - `item sync --non-interactive --request-id` runs through `PtyCli`, in human
    mode.
  - Once the hold is reached and the first `progress:` line is on stderr, it
    sends `0x03`, waits 500 ms, and releases the hold.
- **The answer.** It was the same on all four runs (logs `task-7-spike-*`,
  `task-7-cli-remote.log`):
  - `cancelled`, **exit 130**, at the next safe point. The `SA_RESTART`
    handler let the blocked transfer finish, and the safe point after the
    fetch stopped the request.
  - Human stderr: `progress: accepted`, `connecting`, `fetching`, `observing`,
    `stopping`, then `error: The request was cancelled.`, `code: cancelled`,
    `request: …`, `operation: …` and `effects: publication=pending`.
  - `request show`: finished, outcome `cancelled`. Effects:
    `publication: pending`, every other effect `not_requested`, and
    `commit_oid` null.
  - The observation is `130 68 3 0 0 1 1 1 0`: the exit, the code index, the
    publication index (`pending`), the integration index (`not_requested`),
    the remote context present (no), the local context unchanged (yes), the
    local primary unchanged (yes), the remote primary at the peer's commit
    (yes), and pushes since (0).
- **Effects intact.** Nothing was pushed, merged or moved locally, and the
  effects claim no more. The case pins this answer.
- **For Task 14.** The Known Limit's `transport_unavailable` alternative was
  not observed with this transport. The limit can keep it as possible, or say
  the spike answered `cancelled`. The owner's wording stands.

**Tests.**

- **Unit, Unix** (`interrupt_tests.rs`). Every case re-runs the test binary as
  a child, filtered to itself. A child that must die proves first that it
  survived its first press.
  - The disposition is restored after `during`, from both `SIG_DFL` and
    `SIG_IGN`, with `SA_RESTART` set inside.
  - A press cancels the token, and the process goes on.
  - The latch is set by a press, and stays set in a second window.
  - A second press dies by `SIGINT`.
  - A press outside a window, after one inside it, dies.
  - An unwind restores the disposition.
  - R-36 through `mutate::run`: a latched press stops C2's retry after one
    `execute`, and the next run is retried three times. It fails if the
    loop's latch check is removed.
- **Unit** (`human_collaboration_tests.rs`, 13 cases).
- **Unit** (`collaboration_tests.rs`, 4 cases): the six windowed variants,
  the kind mapping, the supersede predicate, and which request an
  `original_request_required` names.
- **Binary** (`tests/cli/collaboration.rs`, 12 cases, no SSH).
  - The goldens: `repo_sync_publication_remote_required`,
    `item_sync_item_not_found`, `operation_resume_operation_not_found` and
    `operation_abandon_preview`. Also `operation_resume_ok` and
    `operation_abandon_ok`, for R-28's `_ok`.
  - The text goldens: `item_sync_text`, `repo_sync_text`,
    `operation_resume_text` and `operation_abandon_text`.
  - Abandon's prepare, confirm and interactive consent.
  - R-30: C3's stuck save (committed, hand-off owed, then hand-edited, so
    its own retry is `external_change`) is closed by abandon. The edited bytes,
    the branch tip and the worktree are unchanged, the request is released,
    and `index refresh` then succeeds.
  - A remote operation's abandon: `invalid_input` to `--prepare`, and
    `confirmation_required` without a confirmation, with nothing changed.
  - Abandon's stale write: the save's own request finishes the operation after
    the preview, and the confirmation is then `external_change`.
  - `request_mismatch` and the stored-result replay, for resume and for
    abandon.
  - A human resume prints `progress: accepted/observing/indexing` on stderr,
    and JSON stderr is empty.
  - Review Focus 4: a Ctrl-C at abandon's consent prompt is exit 130. The
    operation stays pending, the save's record stays accepted, and the
    allocated request has no record.
  - `--input` on each verb is a usage error with a null request ID.

**For later tasks.**

- **Task 8:** add a `"comment add"` arm to `human_collaboration::again` (with
  its `--input`), or pass `None` to `branch_deleted` for the generic line. The
  publication note goes into `Lines` notes.
- **Task 9:** `conflict resolve`'s `remote_branch_deleted` names the holder's
  request, which is not the envelope's own, so its second line is the generic
  one. The republish line needs `scope.item_id`, so check that the resolve
  envelope carries it.
- **Task 10:** the republish carries from Task 5 (busy names the refused `item
  sync` request; the no-op preview says to run `item sync` first) are not done
  here.
- **Task 12b:** the supersede note's read has no end-to-end case without a
  remote conflict.
- **Task 13:** the spike case is `tests/cli_remote/interruption.rs`'s first;
  Task 13 adds its cases beside it.

#### Task 7, fix round 1

The commit is `fix: share C3's item read with item sync, and close the
review's gaps`, on FIX_BASE `24404ca3`. `origin/main` is still `a372edb`, and
`merge-tree` exits 0. The review found the window async-signal-safe and its
teardown race-free, with one Important finding and five Minors. Each fix is
the controller's ruling (`task-7-fix-1.md`).

- **I1, C3's fill read and item-ID parse are shared, not copied.**
  - `fill.rs` gains `read_any_item`. It resolves the repository and shows an
    item of either kind, and gives the same failure `read_failure` builds.
  - `read_item` now calls it, then makes its kind check, so C3's behavior is
    unchanged. `item sync` calls it, and maps the item's kind.
  - `authoring::document_id` is now `pub(crate) fn item_id`, used by C3's four
    `--id` verbs and by `item sync`. No pre-existing test names it, so the
    rename is allowed.
  - `collaboration.rs` drops its `item_kind`, `item_id` and `invalid_id`
    copies. `operation`'s `invalid_id` refusal is now inline, its only use.
  - `fill_tests.rs` and `authoring_tests.rs` are unedited (`git diff
    24404ca3` on them is empty) and pass.
- **Minor 1.** `human_collaboration::other_requests` is the one rule for "a
  `request.retry` naming a request other than the envelope's own". Both the
  `original_request_required` rendering and `collaboration::original_request`
  use it.
- **Minor 2.** `Window::open` fills the slot with `compare_exchange(null,
  token)`, under a `debug_assert` that it succeeded. A doc line says windows
  never nest.
- **Minor 3.** `interrupt/windows.rs`'s `restore` says what a routine already
  dispatched at teardown does: it can still latch a press and return `TRUE`,
  harmlessly, because the retry stops or the next window starts cancelled.
- **Minor 4.** In `tests/cli/collaboration.rs`, the remote-abandon case now
  says that only `--prepare` reaches the library's refusal, and that C2's
  confirmation check answers `confirmation_required` before anything reads
  the operation.
- **Minor 5.** `interrupt_tests.rs`'s child runner drains both pipes on
  threads, and polls `try_wait` against a 60 s deadline. A child past the
  deadline is killed, and its case fails, naming it.
- **Accepted with no change:** minor 6, the spike's 500 ms wait.
- **Verify.** Each ran as `devenv shell -- cargo … --locked`, with logs
  `logs/task-7-fix1-*`:
  - `--bin manyhands-cli` 212;
  - `cli` 302;
  - `cli_remote -- c4_interruption`: 1 SSH case, observation `130 68 3 0 0 1
    1 1 0`, unchanged;
  - clippy (all targets and features), `fmt --check`, `check --bin
    manyhands-cli` and `check --features desktop --bin manyhands` pass.
- **No ticket comment** was added, as ruling P4 says for a fix round.

#### Task 7, Part B review fix round 1

The commit is `fix: keep an inherited SIGINT ignore, add the missing text
goldens, and share the binary tests' helpers`, on FIX_BASE `5e641f1f`.
`origin/main` is still `a372edb`. Part B's review (`task-11-review-report.md`,
section 4) approved the Part with five Minors. The controller ruled M1 to M4
fixed now (`task-11-fix-1.md`).

- **M1. An inherited `SIGINT` ignore is kept on Unix.**
  - `interrupt/unix.rs`'s `install` first reads the current action. When it
    is `SIG_IGN`, or cannot be read, it installs nothing and returns `None`,
    so the window leaves the ignore in place, as a Windows console handler is
    not called while Ctrl-C is ignored.
  - Teardown restores what it found, which is nothing to restore here.
  - The test is now `the_window_restores_a_default_and_keeps_an_inherited_ignore`.
    From the default, the handler is installed with `SA_RESTART` and the
    default comes back. From `SIG_IGN`, the disposition inside the window is
    still the ignore; a `raise(SIGINT)` neither cancels the token nor ends the
    process; nothing is latched; and the ignore stays.
  - With the `SIG_IGN` check removed, the case fails at the in-window
    disposition assertion.
  - The module doc of `interrupt.rs` says so.
- **M2. R-28's two missing text goldens.**
  - `item_republish_text` (`tests/cli/republish.rs`) is `item republish
    --prepare` with no publication remote: `publication_remote_required`.
  - `conflict_resolve_text` (`tests/cli/conflicts.rs`) is `conflict resolve`
    of an unknown operation, with well-formed input: `operation_not_found`.
  - Both are registered in `CLI_TEXT_CASES`, written with
    `MANYHANDS_UPDATE_GOLDEN=1` (`CI` unset), and read: C2's refusal lines,
    with nothing on stdout.
- **M3. Three comments now match the code.**
  - `human_collaboration.rs`'s module doc says which `next:` lines are built
    from data or code rather than an offered action: `conflict show`, `item
    republish --prepare`, and the no-op preview's `item sync`.
  - `fill.rs` says `read_any_item` also serves `item republish` and `comment
    add`.
  - `interrupt.rs` says the latch is reset in `mutate::drive`.
- **M4. One C4-local test support module.**
  - `tests/cli/c4_support.rs` is mounted additively from `tests/cli.rs`. It
    holds:
    - `golden`, with one placeholder set that covers every field either old
      helper redacted, `/data/synchronization_id` and `/effects/commit_oid`
      included;
    - `environment`, the `CliEnv` with `.gitconfig autocrlf`;
    - `OBSERVATION` and `TOKEN`;
    - `example(name)` and `input_directory()`.
  - `collaboration.rs`, `discussion.rs`, `conflicts.rs` and `republish.rs` use
    it, and their copies are gone.
  - `conflicts.rs` and `republish.rs` keep the pre-existing
    `mutation::golden` for their JSON goldens: switching them would change
    `conflict_show_*`'s placeholders.
  - `human_collaboration_tests.rs`'s constants stay in the bin crate, and the
    pre-existing `tests/cli/mutation.rs` is not edited.
  - A full `--test cli` run with `MANYHANDS_UPDATE_GOLDEN=1` left every
    existing golden byte-identical: `git status` showed only the two new text
    goldens. In that first run, `fixture_directory_is_exactly_the_registered_cases`
    raced the two new files' first write; it passes in the strict run.
- **Not in this round:** M5 (label case) goes to the owner's Task 15 check,
  and the review's observation goes to Task 12a.
- **Verify.** Each ran as `devenv shell -- cargo … --locked`, with logs
  `logs/task-11-fix1-*`:
  - `--bin manyhands-cli` 249;
  - `cli` 338;
  - `cli_remote -- c4_interruption`: 1 SSH case, observation `130 68 3 0 0 1
    1 1 0`, unchanged;
  - clippy (all targets and features), `fmt --check`, `check --bin
    manyhands-cli` and `check --features desktop --bin manyhands` pass.
- **No ticket comment** was added.

### Task 8: `comment add`

The commit is `feat: add comment add`, on BASE `a0296b88`. `origin/main` is
still `a372edb`, and `git merge-tree --write-tree HEAD origin/main` exits 0.
No subagent was dispatched. Only `src/bin/manyhands-cli`, the input schema and
fixture, and tests changed: no library file, so no `--lib` target ran
(controller ruling 2).

**The verb, `discussion.rs`.**

- `comment_add(service, session, spec, args)` follows Task 7's pattern, not
  the brief's `(args, session, service)` order. It reads, in C3's order for
  verbs that take input: the flags, the input object, the four fields, the
  root, `--id` (`invalid_id` with the request named), and then the item's
  kind.
- The kind is R-2's one read, `fill::read_any_item`. Its refusal is the
  verb's, with the request ID, the item in scope, and no record made.
- `object.only` takes the four keys `comment_id`, `body`, `parent_id` and
  `created_by`. `comment_id` and `body` are required, and `body` may be
  empty. `parent_id` is a root comment when absent or null. `created_by` is a
  string when present, so `null` is `invalid_input`: absent means the Git
  identity, and the library holds R-7's form rules.
- `comment_id` and `parent_id` are parsed by one reader, `authoring::ulid`,
  which is C3's rule: a malformed one is `invalid_input` naming the field and
  never the value. `document create` and `ticket create` had the same parse
  inline for `item_id`; they now call `ulid`, so there is one copy. Their
  behavior is unchanged and their tests are unedited.
- It runs through Task 7's `collaboration::run`, which is now `pub(crate)`
  with `authoring_kind`. That is the driver, the bounded retry
  (`Plan { identity_prompt: false, retry: true }`, as C3's saves) and
  `human_collaboration`'s output. The Ctrl-C window was already wired for
  `Mutation::CommentAdd` in `mutate.rs` and is not wired again.
- The catalog entry is `Available("C4")`, `--repo` required, with `--repo`,
  `--id`, `--input` (required), `--request-id`, `--prepare` and `--confirm`, as
  `document save` has them. The verb is unconfirmed, so `--prepare` is
  `not_confirmable` (tested).
- E-1: `help_root.json` and `help_root_text.txt` were regenerated. The diff
  changes only `comment add`'s availability, repository rule and options, and
  its "Available now." line.

**Human output.**

- The publication note, `note: publishing the comment also publishes this
  item's other checkpointed work; unsaved drafts are not included.`, is a
  `Lines` note, so it goes to stderr after C2's lines, as Task 7's notes do.
  It is printed when the result carries comment data (a checkpointed
  comment). A refusal before the checkpoint has no data and no note. JSON has
  none.
- A comment's conflict prints the same `conflict:` and `conflict show` lines
  as a synchronization's.
- `remote_branch_deleted` (controller ruling 1) needed no new rendering. The
  `request.cancel` action prints Task 7's three `next:` lines, in order:
  `request cancel` naming the request; the request again; `item republish
  --prepare`. The second line is the generic `send request <id> again with its
  original command and input`, not a command: the retried `comment add` needs
  the caller's input, which the result does not hold and must not print. This
  is the generic alternative Task 7's carry named, so `again()` has no
  `comment add` arm. The reservation note follows the publication note.
  The unit test builds the envelope from `comment_add_push_rejected`. Only a
  remote stages the case, so Task 12a covers it end to end.

**Tests.** First (RED), run before the implementation:

- `--test cli -- discussion`: 14 of 15 failed with `command_unavailable`; the
  schema test passed because the schema and fixture were already written.
- `--bin manyhands-cli -- human_collaboration`: the 3 new cases that expect the
  note failed. The case with no comment data passed.

Then:

- **Unit** (`discussion_tests.rs`, 4 cases): the four fields and the two
  required; a null parent; each missing, mistyped or unknown field's detail;
  a malformed ID in either field is refused naming the field, not the value,
  and a lower-case ULID is malformed.
- **Unit** (`human_collaboration_tests.rs`, 4 new cases, 13 to 17): the note
  after C2's lines on success and on a partial refusal; no note without comment
  data; a comment's conflict lines; and `remote_branch_deleted` from a
  constructed envelope.
- **Binary** (`tests/cli/discussion.rs`, 15 cases, no SSH):
  - the input schema: required `comment_id` and `body`, optional `parent_id`
    (nullable) and `created_by`, each mistyped, and closed;
  - `comment_add_ok`: exit 0, `ok`, publication pending, `pending_reason:
    no_publication_remote`, no recovery, no body in the envelope, the stored
    result equal to the printed one;
  - a comment on a document;
  - a given author replaces the Git identity, and a body with CRLF, a non-BMP
    character, a line `---` and no final newline is written byte for byte
    (Review Focus 1);
  - `comment_add_ticket_closed`: exit 3, nothing changed, no context left;
  - `comment_add_invalid_input`: a malformed `created_by` (the library's
    refusal, exit 2, nothing written); a malformed `comment_id` or
    `parent_id`, a missing, mistyped or unknown field and a null author (the
    CLI's, request named, no record made); a malformed `--id` is `invalid_id`;
  - an unknown item: `item_not_found` with the item in scope and no record;
  - `--prepare` is `not_confirmable`;
  - a reply, a reply to a reply and a second root, listed with their parent
    and depth through `comment list` (0, 1, 2, 0);
  - an unknown parent, the comment's own ID (Review Focus 5) and another
    item's comment are `item_not_found`, with the snapshot unchanged;
  - an occupied comment ID is `occupied_path`;
  - a replay writes no second file (snapshot, refs and the context's comment
    files unchanged); the same request with another comment is
    `request_mismatch`;
  - human text: `progress:` first, the note last on stderr, once, the
    publication reason on stdout, and no body anywhere;
  - the example input `comment_add.json`, validated by the schema and run
    through the binary, then as a reply to what it wrote.
- **Goldens (R-28):** `comment_add_ok`, `comment_add_ticket_closed`,
  `comment_add_invalid_input` and `comment_add_text`, registered in
  `tests/support/cli.rs`. The library's own `comment_add*` mutation goldens
  are unchanged.

**Verify.** Each ran as `devenv shell -- cargo … --locked`; logs are in
`logs/task-8-*`.

| Command | Result |
| --- | --- |
| `test --all-features --bin manyhands-cli` | 220 passed (212 before) |
| `test --all-features --test cli` | 317 passed (302 before), after the E-1 regeneration; before it, only the 2 help cases failed |
| `test --all-features --test cli -- input_schemas` | 7 passed |
| `clippy --all-targets --all-features -- -D warnings` | passes |
| `fmt --check` | passes |
| `check --bin manyhands-cli` | passes |
| `check --features desktop --bin manyhands` | passes |

**For later tasks.**

- **Task 9:** `conflict resolve` also reads input. Its ID fields should use
  `authoring::ulid` and its run `collaboration::run`. Its `remote_branch_deleted`
  needs no new rendering either.
- **Task 12a:** the `comment add` rows that need a remote run end to end here:
  publication, the refused push, the retry, the conflict, `remote_branch_deleted`
  with Task 7's three lines, and the Ctrl-C rows.
- **Task 13:** `comment add`'s exit accounting through the binary has these
  codes: `ok`, `ticket_closed`, `invalid_input`, `invalid_id`, `item_not_found`,
  `occupied_path`, `request_mismatch` and `not_confirmable`. The rest need a
  remote or an identity staging.

### Task 9: `conflict show` And `conflict resolve`

The commit is `feat: add conflict show and conflict resolve`, on BASE
`fefc6ae4`. `origin/main` is still `a372edb`, and `git merge-tree
--write-tree HEAD origin/main` exits 0. No subagent was dispatched. Only
`src/bin/manyhands-cli`, the input schema and fixture, and tests changed: no
library file, so no `--lib` target ran (controller ruling 5).

**The verbs, `conflict.rs`.**

- `conflict::show` reads `show_conflict` for `--repo` and `--operation-id`. A
  malformed ID is `invalid_id` in the repository's scope. It is a read: no
  request ID, and `--prepare`, `--confirm`, `--request-id` and `--input` are
  usage errors by the grammar. The catalog entry is `Kind::Read`,
  `Available("C4")`, options `--repo` and `--operation-id`. `read_output`
  in `dispatch.rs` is now `pub(crate)`, so the verb shares it.
- `conflict::resolve` reads, in C3's order: the flags, the input object, the
  two fields, the root, then `--operation-id`. `fields` takes `observation`
  and `resolutions` (`only`); each resolution takes exactly `token` and
  `source` (`only`). A token is any string: the library judges its form
  (R-9) and whether it names a path of the live conflict (R-33). Its
  `invalid_input` for a malformed token, a duplicate token or a malformed
  observation is the library's, found before anything is recorded.
- `input::InputObject::object_list(key)` is the new reader. A missing key is
  `Missing`. A non-array, an element that is no object and an empty array
  are each `WrongType` ("a non-empty array of objects"), naming the field
  and never a value.
- `collaboration::operation_id` is extracted from `operation()` so
  `conflict resolve` can read its input between the flags and the
  selectors. `operation resume` and `abandon` call it unchanged.
- It runs through `collaboration::run` (the bounded retry, and the Ctrl-C
  window already wired for `Mutation::ConflictResolve`). `--prepare` is
  `not_confirmable`. Its catalog entry is unconfirmed, input required.
- The input schema and example: `schemas/input/v1/conflict_resolve.schema.json`
  (closed; `observation` and `resolutions` required; each resolution closed
  with `token` and `source` required) and
  `tests/fixtures/cli_input_v1/conflict_resolve.json`. The checker has no
  `minItems`, so the schema cannot say "non-empty"; the CLI reader and the
  library do, and a `description` says so.
- E-1: `help_root.json` and `help_root_text.txt` regenerated. The diff changes
  only the two verbs' availability, repository rule and options, and their
  "Available now." lines.

**`collaboration::seen` (Task 7's pattern, generalised).** `run`'s
`resumed` parameter is now `named`: the operation a verb names, for the one
extra read human mode may add. The read happens only when
`wants_conflict(envelope)`:

| Command and code | Why | Adds |
| --- | --- | --- |
| `operation resume`, `original_request_required` naming a request | Task 7: may the request's written resolution be superseded | `supersedable` |
| `conflict resolve`, `external_change` | ruling 1: does the attempt have written paths | `written` = `historical` |
| `conflict resolve`, `remote_branch_deleted` | ruling 3: the envelope's scope names no item | `item` = the conflict's `item_id` |

A `conflict resolve` that answers `original_request_required` reads nothing
(ruling 2). The pure `observed(envelope, historical, attempt, item)` builds
the result from the read, as `supersedable` does, and is unit-tested. When the
read fails, the result is the one JSON mode gives, with no extra line.

**Human output (`human_collaboration.rs`).**

- Ruling 1: `external_change` of a resolve whose conflict reads `historical`
  adds `note: a resolved file edited by hand must be restored to the request's
  resolution, or to its conflicted content, before the request is sent again.`
  JSON is unchanged: its `message` stays the code's own (Global Constraints).
- Ruling 2: `original_request_required` of a resolve prints the `next:` line
  naming the request and `note: the paths the original request wrote were
  edited. Restoring them to the request's resolution or to their conflicted
  content lets that request continue, or a new conflict resolve supersede
  it.` Task 7's supersede note stays an `operation resume`'s only.
- Ruling 3: `remote_branch_deleted` of a resolve prints Task 7's three
  `next:` lines. The cancel line and the "send request … again" line name the
  holder's request, taken from the `request.cancel` action; the resolve's own
  request appears once, as the request refused. The republish line needs the
  item, which a resolve's scope does not carry, so it comes from the conflict
  read. Without the read the line is left out (two lines).
- **Decision, beyond the rulings:** the reservation note says "the refused
  request keeps its reservation", which is false when the refused request is a
  resolve and another request holds it. When the action's request is not the
  envelope's own, the note names it: `note: request <id> keeps its reservation
  until it is cancelled; sending it again then ends it and releases the
  reservation.` The own-request wording is unchanged for `item sync` and
  `comment add`.
- Ruling 4: `conflict_show` prints the lines `operation`, `comment
  operation` (when set), `target`, `item` (when set), `stage`, `state`,
  `next action`, `request` (when set), `observation` (`-` when none),
  `historical: yes` (when true), and one `path:` line per path:
  `kind=… structure=… token=… display=…`. The display path is last, so a name
  cannot imitate a field. It is printed through `field`, once, which leaves the
  library's `\xNN` text as it is. No side is printed in text; JSON carries
  them. An `incompatible_closures` path adds the design's note, once.

**Tests.** RED, before the implementation: `--test cli -- conflicts` (13
cases) had 9 failed with `internal_error` (no dispatch arm) and 4 passed (the
schema case, and the grammar's usage errors). `object_list`, the `Seen`
fields and the renderers were written with their unit tests in the same step,
so there is no separate RED for the bin cases.

- **Unit** (`input_tests.rs`, 2): order and closed objects; the missing key,
  the empty array, a non-array and a non-object element, each refused by field
  with no value in the detail.
- **Unit** (`conflict_tests.rs`, 3): the two fields and the order of the
  resolutions, sources byte for byte; a token is any string; each missing,
  mistyped, empty, or unknown field's detail, never a value.
- **Unit** (`collaboration_tests.rs`, 4): which results read the conflict;
  `written` for a resolve; `item` for a deleted branch; `supersedable` for a
  resume.
- **Unit** (`human_collaboration_tests.rs`, 14): rulings 1 to 4; Review Focus
  2 (a display path with an escape sequence and a newline, no raw control
  byte, no doubled escape; a raw control from a library bug is escaped; a path
  cannot forge a field); the `incompatible_closures` note; the sides in JSON
  only; a resolve sent to outside repair prints the `conflict:` lines.
- **Binary** (`tests/cli/conflicts.rs`, 13, no SSH): the input schema and
  the example; `conflict_show_operation_not_found` (exit 2, golden, text,
  snapshot unchanged); a malformed operation ID for both verbs; `conflict show`
  refusing `--prepare`, `--request-id`, `--confirm` and `--input` as usage
  errors; `conflict resolve` not confirmable, requiring its input and its
  operation, refusing an undeclared option; an unknown operation after the
  syntax passes; a malformed token or observation and a duplicate token
  (`invalid_input`, exit 2, golden, snapshot unchanged); every input the CLI
  refuses (request named, no record made); `conflict show` and `conflict
  resolve` of a pending local operation that is no synchronization
  (`invalid_input`); and of a local-context sync's operation, made with the
  gate's `LocalContextSync` mutation (`invalid_input`, never
  `operation_not_found`, no record of the resolve, the sync's request as it
  was, the repository untouched).
- **Goldens (R-28):** `conflict_show_operation_not_found`,
  `conflict_show_invalid_input` and `conflict_resolve_invalid_input` in
  `tests/support/cli.rs`'s `CLI_CASES`. The text golden of `conflict show`
  needs a live conflict, so it is Task 12b's.

**Verify.** Each ran as `devenv shell -- cargo … --locked`; logs are in
`logs/task-9-*`.

| Command | Result |
| --- | --- |
| `test --all-features --bin manyhands-cli` | 243 passed (220 before) |
| `test --all-features --test cli` | 330 passed (317 before) |
| `test --all-features --test cli -- input_schemas` | 7 passed |
| `clippy --all-targets --all-features -- -D warnings` | passes |
| `fmt --check` | passes |
| `check --bin manyhands-cli` | passes |
| `check --features desktop --bin manyhands` | passes |

**For later tasks.**

- **Task 12b:** the live rows run end to end here: `conflict show` of a real
  conflict (JSON and its text golden, with a path holding an escape sequence);
  `conflict resolve` with the captured observation and tokens; a stale
  observation; the hand-edit escape (`external_change` plus the note, with the
  read marking `historical`); the supersede and `original_request_required`
  with its note; and `remote_branch_deleted` from a resolve, which must show
  the holder's request in the two request lines and the item from the read in
  the third. Check there that `show_conflict` of the operation still returns
  `item_id` after the continuation stopped (the unit tests build the read by
  hand).
- **Task 13:** `conflict show` and `conflict resolve` through the binary reach
  these codes without a remote: `operation_not_found`, `invalid_input`,
  `invalid_id`, `not_confirmable` and (show) `usage_error`.
- **Task 14:** design line 2045's Known Limit (an unfinished resolution whose
  written paths were edited) is now told by the human output's two notes.

**Fix round 1** (review of `fefc6ae4..71d65975`: approved; four minors folded
in). FIX_BASE `71d65975`.

- **The notes match the library's rule** (`integration/resolution.rs`,
  `remote/sync.rs`): a path the request has written must hold its result; only
  a path it has not written yet may hold its conflicted content.
  - `EDITED_NOTE`: `the attempt's written paths were edited by hand. Restore
    each file the request wrote to the request's resolution; a file it has not
    written yet may keep its conflicted content. Then send that request again,
    or a new conflict resolve may supersede it.`
  - `HAND_EDIT_NOTE`: `a resolved file edited by hand must be restored to the
    request's resolution before the request is sent again.`
  - The two unit tests that pin the texts were updated. The "never Task 7's
    note" assertion now compares with `SUPERSEDE_NOTE` itself, because the new
    `EDITED_NOTE` also contains "may supersede".
- **`tests/cli/conflicts.rs`:** the duplicate-key case's comment is corrected
  and its `resolutions` list is valid, so it isolates the repeated key.
- **Module doc** of `human_collaboration.rs`: for a resolve, the three
  `remote_branch_deleted` lines name the request that holds the
  synchronization.
- **Duplication:**
  - `dispatch::operation_id(args, &repo)` sits beside `item_id`, keeps
    `value()?`'s error path, and serves `operation_show` and `conflict show`.
    `operation_show`'s tests needed no edit.
  - `reservation_note` builds the holder's note from `RESERVATION_NOTE` with
    `replacen`; the text it produces is the same.
  - `wants_conflict` matches `human_collaboration::RESOLVE` (now `pub const`)
    instead of the literal.
- **Left as briefed:** the label style, and the schema's `minItems`.
- **Verify** (`logs/task-9-fix1-*`): `--bin manyhands-cli` 243 passed;
  `--test cli` 330 passed; clippy (all targets, all features), `fmt --check` and
  `check --bin manyhands-cli` pass. No new ticket comment.

### Task 10: `item republish`

The commit is `feat: add item republish`, on BASE `f1f9f06e`. `origin/main` is
still `a372edb`, and `git merge-tree --write-tree HEAD origin/main` exits 0. No
subagent was dispatched. Only `src/bin/manyhands-cli` and tests changed: no
library file, so no `--lib` target ran (controller ruling 3).

**The verb, `collaboration.rs`.**

- `item_republish` is C2's confirmed flow, as `operation abandon` is: the same
  `run` driver (the bounded retry, and the Ctrl-C window that Task 5 and
  Task 7 already wired for `Mutation::ItemRepublish`), with `--prepare`,
  `--confirm` and consent at a terminal. The catalog entry is `item
  republish`, `Available("C4")`, "Republish a deleted item branch.", options
  `--repo`, `--id`, `--request-id`, `--prepare` and `--confirm`, confirmed,
  input refused.
- `item_sync` and `item_republish` share a private `item` function, which
  reads the flags, the root and the item ID (`authoring::item_id`), reads the
  item's kind through `fill::read_any_item` (R-2) and builds the mutation.
  `item_sync` behaves as before.
- The dispatch arm is `"item republish"`. E-2: `catalog_tests.rs`'s C4 row
  gains `"item republish"`. E-1: the help goldens were regenerated, and the
  diff is one added row in each of `help_root.json` and `help_root_text.txt`.

**Human output (controller rulings 1 and 2).**

- **Busy.** No public read gives the request that holds the reservation
  (`OperationDto` carries none), and a library change is an owner stop. So a
  human-mode `busy` of `item republish` prints, before C2's `next:
  request.retry` line, `note: if an earlier item sync of this item was
  refused, its request still holds the reservation: run request cancel with
  that request's ID, then send that request once more, before retrying this
  request.` It is `Lines::lead` in `human_collaboration.rs`, printed with the
  failure's extra lines, which come before the recovery lines. Another verb's
  `busy`, a republish's other refusals and JSON are C2's output unchanged.
  The live journey (a refused `item sync` that holds the reservation) is
  Task 12b's.
- **No-op preview.** The no-op preview answers `already_applied` with no
  confirmation. Its human output, from `human_admin::republish_noop` (called
  by `render::prepare_output`, so `--prepare` and the interactive path both
  get it), keeps C2's `Already applied; nothing changed.`, adds `next:
  manyhands-cli item sync --repo '<root>' --id <id>` on stdout when the scope
  names both, and `note: no deletion of this item's published branch is
  recorded, so there is nothing to republish; run item sync first, which looks
  at the remote, then prepare the republish again.` on stderr. No recovery
  action was added (decision 8). JSON is unchanged. The helper lives in
  `human_admin.rs`, and `quoted` moved from `human_collaboration.rs` to
  `render.rs` beside `field`, because `tests/cli_remote.rs` mounts
  `render.rs` and `human_admin.rs` but not `human_collaboration.rs`; a
  dependency of `render.rs` on the latter would not resolve there. `cargo
  check --test cli_remote` passes.
- The existing `remote_branch_deleted` lines (Task 2b's three `next:` lines
  ending in `item republish --prepare`) already name `request cancel`.

**Tests.** RED, before the implementation: the unit cases (`--bin
manyhands-cli -- republish`) had 4 failed, 4 passed (the 4 passed were the
existing deleted-branch cases and the C2-unchanged ones). The binary cases
(`--test cli -- republish`), run with the catalog entry removed, had 6 failed
with `usage_error` for an unknown command, and 2 passed (the unconfirmed
verbs, and the refused input, which are usage errors either way).

- **Unit** (`human_collaboration_tests.rs`, 3): the busy note comes before the
  retry line, exactly; another verb's busy, and a republish refused otherwise,
  are C2's output; JSON has no note.
- **Unit** (`human_admin_tests.rs`, 4): the no-op preview names `item sync`
  and the note, with the root quoted and escaped and no raw control; without a
  scope it keeps the note; another command's no-op preview and JSON are C2's.
- **Binary** (`tests/cli/republish.rs`, 8, no SSH):
  - `item_republish_publication_remote_required`: `--prepare` with no
    publication remote is `publication_remote_required`, exit 3, a ticket and
    a document, snapshot unchanged, golden;
  - an unknown item is `item_not_found` and a malformed ID `invalid_id`,
    before anything is prepared;
  - an unknown confirmation is `confirmation_not_found`, exit 3, nothing
    published;
  - a malformed confirmation is `invalid_id`;
  - `--prepare` with `--request-id` or `--confirm` is a usage error, request
    null;
  - input is refused and the item is required;
  - JSON without a request ID is a usage error;
  - `--prepare` of each of the five unconfirmed mutations (`comment add`,
    `item sync`, `repo sync`, `conflict resolve`, `operation resume`) is
    `not_confirmable`, exit 2, request null, snapshot unchanged.
- **Golden (R-28):** `item_republish_publication_remote_required` in
  `tests/support/cli.rs`'s `CLI_CASES`. The preview golden, the confirmed
  push and the interactive consent need a recorded publication, which needs the
  SSH fixture, so they are Task 12b's.

**Verify.** Each ran as `devenv shell -- cargo … --locked` (`fmt --check` takes
no `--locked`); logs are in `logs/task-10-*`.

| Command | Result |
| --- | --- |
| `test --all-features --bin manyhands-cli` | 250 passed (243 before) |
| `test --all-features --bin manyhands-cli -- catalog` | 7 passed |
| `test --all-features --test cli` | 338 passed (330 before) |
| `test --all-features --test cli -- input_schemas` | 7 passed |
| `clippy --all-targets --all-features -- -D warnings` | passes |
| `fmt --check` | passes |
| `check --bin manyhands-cli` | passes |
| `check --all-features --test cli_remote` | passes |
| `check --features desktop --bin manyhands` | passes |

**For later tasks.**

- **Task 12b:** the live rows run end to end here: the preview golden (`item
  republish --prepare` after a recorded deletion, and after push-absence
  evidence), `--confirm`, the interactive consent, a Ctrl-C at the prompt,
  replay, and a transport stop. In particular the `busy` row: a refused `item
  sync` that holds its reservation, the republish's retry ending `busy`, and
  the note printed before the retry line. Also the no-op preview's human text
  from a real repository.
- **Task 13:** `item republish` through the binary reaches these codes
  without a remote: `publication_remote_required`, `confirmation_not_found`,
  `item_not_found`, `invalid_id`, `not_confirmable` (of the five unconfirmed
  verbs), and `usage_error`.
- **Task 14:** the RFC's `item republish` row can cite the no-op preview's
  human text.

#### Task 10, fix round 1

The review of `f1f9f06e..76344619` approved the task; its minors are folded in.
FIX_BASE `76344619`. No library file changed; no `--lib` target ran. No new
ticket comment.

- **The no-op `next:` line runs as printed.** In non-interactive human mode,
  `item sync` without `--request-id` is a `usage_error`. C4's other `next:`
  lines for a mutation all resend an *existing* request, so each already
  carries its ID; there was no C4 convention for a new request. I followed
  C2's own: its preview line says "add --request-id unless you run it at a
  terminal". The `next:` line is unchanged, and the note now reads `... run
  item sync first (add --request-id unless you run it at a terminal), which
  looks at the remote, then run the republish again.` The unit test pins the
  new text.
- **Wording.** "then run the republish again" reads right on the `--prepare`
  path and on the interactive path (where the republish is the command the
  user already ran).
- **One helper for note lines.** `render::note_lines(&[String]) -> String`
  replaces the four copies: `prepare_output`, `envelope_output`, and
  `Lines::notes` and `Lines::lead` in `human_collaboration.rs`.
- **Shared test helpers.** `stdout`, `stderr` and `no_raw_control` are now in
  `render::test_support`, a `#[cfg(test)]` module of `render.rs`;
  `human_admin_tests.rs` (my four Task 10 cases only, no pre-existing case
  edited) and `human_collaboration_tests.rs` import them. A separate
  `test_support.rs` module did not work: `tests/cli_remote.rs` mounts
  `human_admin.rs`, and with it `human_admin_tests.rs` (`cfg(test)` is on in
  that target), so a helper module has to be reachable from `render.rs`,
  which it mounts, not from the bin's root. `check --test cli_remote` caught
  it.
- **Module doc.** `human_collaboration.rs` now says the no-op preview's code
  is `human_admin::republish_noop`, called from `render::prepare_output`, and
  why it is not in that module.
- **Tests that can fail.**
  - `a_busy_republish_in_json_is_the_envelope_with_no_note` is deleted: it
    called `render::mutation_output(true, ..)`, which never reaches the new
    code. A comment beside the busy test names the real guard: the note is
    built only in `human_collaboration::output`, and `collaboration::run`'s
    `Driven::Executed { .. } if session.json` arm returns before it. That arm
    needs a service, so no unit test can drive it.
  - `item_republish_refuses_input_and_needs_its_item` asserts `command ==
    "item republish"` for both refusals (it holds for both).

**Verify** (`logs/task-10-fix1-*`):

| Command | Result |
| --- | --- |
| `test --all-features --bin manyhands-cli` | 249 passed (250 before; the JSON test is deleted) |
| `test --all-features --test cli` | 338 passed |
| `clippy --all-targets --all-features -- -D warnings` | passes |
| `fmt --check` | passes |
| `check --bin manyhands-cli` | passes |
| `check --all-features --test cli_remote` | passes |

The full `--test cli` run started before the last edit, which only moved a
`#[cfg(test)]` module inside `render.rs`; the binary under test has the same
production code.

### Task 11: Part B Checkpoint

The controller ran this checkpoint and its review at `8f3f9f4c`.
`origin/main` is still `a372edb`, and `git merge-tree --write-tree HEAD
origin/main` exits 0.

- **The gate.** Each command runs as `devenv shell -- cargo … --locked`;
  the logs are `logs/task-11-gate-*` and `task-11-regate-*`.
  - On `5e641f1f`:
    - `check --all-features`, `fmt --check`, `clippy --all-targets
      --all-features -- -D warnings`, `check --features desktop`, `check
      --bin manyhands-cli` and `check-cli-no-gpui.sh` pass.
    - Desktop-alone clippy fails with main's 58 errors exactly (Ruling P6).
    - The tests: `--bin manyhands-cli` 249, `cli` 338, `mutation_bridges`
      129, `mutation_confirmation` 183, `mutation_contract` 44,
      `mutation_replay` 383, `read_contract` 72, and `cli_remote` 32 SSH
      cases.
  - After the fix round, on `8f3f9f4c`: the same build and lint steps (with
    clippy's 58 again), then `--bin manyhands-cli` 249, `cli` 338 and
    `cli_remote` 32.
  - **Cited from Task 6** (`74be902`): `--lib` 1445, `mutation_remote` 257,
    `comment_publication` 41 and `remote_merge_recovery` 25. Since
    `74be902`, Part B changed no file under `src/` outside
    `src/bin/manyhands-cli/`, no test file outside the CLI set, and only
    `schemas/input/`. None of those targets compiles or reads any of these,
    and none mounts `tests/support/mod.rs`. The five contract targets after
    the fix round are cited from the `5e641f1f` run on the same grounds.
    This follows the owner's rule that front-end work runs library tests
    only for library code it changes.
- **Step 2.** C5's verbs still answer `command_unavailable`, and help and
  version start no transport. The tests that pin this
  (`tests/cli/boundaries.rs`, `exits.rs`, `help.rs`, `dispatch_tests.rs`)
  are unchanged since `origin/main`, and they pass in the gate's `cli` and
  `--bin` runs.
- **The independent review** (opus, `970cd40c..5e641f1f`,
  `task-11-review-report.md`).
  - **Approved:** 0 Critical, 0 Important, 5 Minor. The Ctrl-C window, JSON
    purity, escaping, C1 to C3's shared code, the catalog and help, and the
    pre-existing tests all held.
  - **Fixed in `8f3f9f4c`:**
    - M1: an inherited `SIGINT` ignore is kept on Unix, as Windows keeps it;
    - M2: R-28's `item_republish_text` and `conflict_resolve_text` goldens;
    - M3: three comments;
    - M4: one `tests/cli/c4_support.rs`, with every golden byte-identical
      and `tests/cli/mutation.rs` untouched.
  - **M5 goes to the owner's on-screen check at Task 15:** the lower-case
    labels of `conflict show` and R-37's local-sync lines, where C1 and C2
    use Title Case.
  - **To Task 12a:** watch what `operation resume` of a comment's
    publication prints when the branch was deleted.
- **Per-task reviews in Part B.** Task 7 had one Important finding (C3's
  read copied), fixed. Tasks 8, 9 and 10 were approved. Tasks 9 and 10
  folded in minors that corrected user-facing guidance:
  - the resolution notes now follow the library's rule for written paths;
  - the no-op preview's `next:` line now runs as printed.
- **Task 14 carries added in Part B:**
  - `remote_branch_deleted`'s three `next:` lines, naming the holder's
    request for a resolve;
  - the busy note names the step, not the request ID (row 13);
  - the escape and edited-path notes;
  - the no-op's `item sync` line.
- **Ticket comment** `01M65KWK66M3RFEC1ADJ3SRNHS`, a reply to
  `01M65H0BS6T4K3309X4NKVH3SR`.

### Task 12a: Two Clones: Comments And Synchronization

The commits are `cbbca6ef` (`fix: say how to free a deleted branch's
reservation after a resume`) and `test: prove C4 comment and sync journeys
across two clones`, on BASE `df31f127`. `origin/main` is still `a372edb`, and
`git merge-tree --write-tree HEAD origin/main` exits 0. No subagent was
dispatched. Only `src/bin/manyhands-cli/human_collaboration*.rs` and tests
changed, with no library file, so no `--lib` target ran.

**The stop, and the ruling.**

- **The defect.** The first dispatch stopped BLOCKED. `remote select`
  (`set_publication_remote_recorded`, through `configuration_tree`) commits
  `.manyhands/config.toml` from a fresh in-memory index and never writes the
  worktree's index. After it, the file is `INDEX_MODIFIED | WT_MODIFIED`, and
  the next `repo sync` answers `worktree_not_clean` (3). The evidence is
  `logs/task-12a-evidence-remote-select.log`.
- **The owner's ruling.**
  - The defect is ticketed on main as `01M669FJR27FRHCNPWAMD6VH4V`, "Remote
    selection must leave the Git index current for the configuration it
    commits".
  - C4 takes option 3: the server's primary is seeded already configured, so
    neither clone's `remote select` commits. Each clone still runs `remote
    select` through the binary, and it answers `already_applied`.
  - There is no index workaround in the fixture.

**`CliPair` (`tests/support/cli_pair.rs`).** It is mounted only from
`tests/cli_remote.rs`. The interface's `CliPair { server, a, b }` has one
extra private `TempDir`, which keeps the clones alive.

- **The seed (R-33).** One commit on the server's primary, made before either
  clone. It holds `.manyhands/config.toml` (`format_version`,
  `primary_branch = "main"`, `publication_remote = "origin"`) and two tickets:
  `ITEM`, and `OTHER_ITEM` for a request sent again with another target.
- **Each clone.** A `git2` clone of the server, then, through `checked`:
  1. `key import` of the server's one allowed key into the clone's own key
     store, and `key select`. Both clones import the same key file, because
     the SSH fixture allows a single key. "Its own selected key" holds as
     each clone's own key-store entry;
  2. `repo identity-set`, prepare then confirm;
  3. `repo enable {"primary_branch":"main"}`, prepare then confirm;
  4. `remote select origin`, which answers `already_applied`;
  5. `host approve`, prepare then confirm.
- **The order matters.**
  - `key select` fences every repository whose publication remote is
    configured already (`invalidate_key_selection`, then `history_unknown`).
    A fenced repository answers a context it never published as
    `HistoryUnknown`, and the stopped attempt keeps the reservation, so every
    later sync is `busy`.
  - `repo enable` of an already-enabled primary requires an identity but
    writes none, so `repo identity-set` comes first.
- **No context yet.** Neither clone has an editing context when the pair is
  made: each case makes its own with a first write.

**The fix (controller ruling 4).** An `operation resume` that answers
`remote_branch_deleted` offers no `request.cancel`, because a resume stops at
no safe point. Human output now prints a note: the request that started the
publication, its `comment add` or `item sync`, still holds the reservation;
cancel it by its ID, send it once more, then run `item republish`.

- **The `next:` line.** It prints `manyhands-cli item republish --repo
  '<root>' --id <item> --prepare`, quoted and escaped as the other lines are.
- **The item it names.** The item comes from the scope, or, when the resume
  took up a comment's publication, from the comment's data. The resume's
  scope names no item there (observed). This is my reading of "when the
  scope names the item".
- **What stays C2's.** JSON is unchanged. Any other verb, or a resume
  refusal that offers the cancellation, keeps the existing output.
- **Tests (`human_collaboration_tests.rs`, 3).**
  - RED, before the change: 2 failed and 1 passed. The guard test passes on
    the old code, as it should.
  - GREEN: 36 passed in `human_collaboration`, 252 in the bin.

**The cases (`tests/cli_remote/collaboration.rs`, 8 SSH cases, each a fresh
pair).** Every run goes through `scan_output`. Where a case lists no
comments, the bodies are a probe no output may hold.

- `c4_collaboration_comment_publishes`
  - A's first `comment add` makes its context and publishes it: `ok`, 0, with
    write `written`, checkpoint `committed` and publication `published`.
  - `created_by` is A's identity, and the server holds the comment.
  - A replay is identical, with no connection and no push.
  - `--id OTHER_ITEM` under the same request ID is `request_mismatch`, 2, and
    makes no context.
  - Human mode prints C2's lines on stdout. On stderr: `progress: accepted`,
    `writing`, `fetching`, `observing`, `pushing`, `verifying` and
    `indexing`, then the publication note.
- `c4_collaboration_comment_refused_then_saved_once`
  - The server refuses updates of its checked-out branch. `comment add`
    answers `push_rejected`, 4: checkpoint committed, publication pending,
    `pending_reason: synchronization`, and `[request.retry(R2)]` first.
  - The record stays `accepted`.
  - The server recorded exactly four refused pushes of the checkpoint (the
    first and C2's three re-runs).
  - With the server accepting, the same request answers `ok`, with the same
    operation and synchronization and the same checkpoint.
  - The file's bytes are unchanged, there are two comment files, and one
    commit adds the comment. One accepted push; the record is `finished`.
- `c4_collaboration_replies_from_both_clones`
  1. A adds root C1.
  2. B's first write, root C2, makes B's context. Its publication merges A's
     (integration `complete`).
  3. B replies to C1 (C3).
  4. A's `item sync` fast-forwards (`already_applied`, no push).
  5. A replies to C3 (C4).
  6. B's `item sync` fast-forwards.
  7. Both clones list `[C1 0, C3 1, C4 2, C2 0]` with each comment's author,
     at the same tip.
- `c4_collaboration_item_sync`
  - B's first write is a `ticket save` of the body: a checkpoint, not
    published.
  - Clean divergence: `ok`, 0, integration `complete`, publication
    `published`. One merge commit, with parents `[local, incoming]` and the
    subject `Merge remote context <item>`. The server keeps its earlier tip
    as an ancestor. One push, and the `item_sync_ok` golden.
  - A replay is identical, with no connection.
  - `--id OTHER_ITEM` is `request_mismatch`, 2.
  - A's fast-forward, in human mode: `Already applied; nothing changed.` and
    `effects: discovery=current publication=current`. On stderr:
    `progress: accepted`, `connecting`, `fetching`, `observing`,
    `updating_local`, `observing`, `indexing`. No push.
  - Already current: `already_applied`, `noop`, no push.
- `c4_collaboration_repo_sync`
  - Each clone commits a file on primary with `git2`, as history.
  - A, local ahead, in human mode: `The request completed.` and its effects
    line. On stderr: `progress: accepted`, `connecting`, `fetching`,
    `observing`, `pushing`, `verifying`, `indexing`.
  - B's clean divergence: `ok`, integration `complete`, a merge with parents
    `[local, incoming]` and the subject `Merge remote primary`, published. The
    `repo_sync_ok` golden.
  - A replay is identical, with no connection.
  - `item sync --id ITEM` under the same request ID is `request_mismatch`, 2
    (ruling 3), and makes no context.
  - A's fast-forward, then already current: `already_applied` both times, no
    push.
- `c4_collaboration_resume_publishes_a_stopped_comment`
  - With `disconnect_at(Advertisement)`, `comment add` answers
    `transport_unavailable`, 4, publication `pending`, `request.retry` first.
  - `operation resume` of the comment's operation, in human mode: exit 0. It
    prints `The request completed.` with
    `effects: discovery=current publication=published commit=<checkpoint>`.
    On stderr: `progress: accepted`, `observing`, `fetching`, `observing`,
    `pushing`, `verifying`, `indexing`, then the note.
  - It published the comment once and wrote nothing.
  - The JSON replay of the same request has no connection. It carries
    `operation_id: null`, a `CommentMutationDto` with publication
    `published`, and the comment's synchronization. It is the
    `operation_resume_comment_ok` golden (ruling 2).
  - Another operation ID under the same request ID is `request_mismatch`, 2.
  - The comment's own request then answers `ok`, published, with no push.
- `c4_collaboration_comment_meets_a_deleted_branch` (carry 1)
  - After the server's context is deleted, a human-mode `comment add` exits
    4 with `code: remote_branch_deleted` and its effects.
  - It prints the three `next:` lines in order:
    - `request cancel --request-id R2`;
    - `send request R2 again with its original command and input`;
    - `item republish --repo '<root>' --id <item> --prepare`.

    Then the publication and reservation notes.
  - The record stays `accepted`.
  - `request cancel` gives `requested: true`. The same request then answers
    `cancelled`, 130, with the checkpoint `committed`, publication `pending`
    and no recovery. The record is `finished`; nothing is recreated and
    nothing is pushed.
  - `item republish --prepare` answers `ok`, with `branch_republished` and
    `local_oid` the checkpoint. The preview takes no reservation, so it
    proves nothing about one (fix round 1).
  - The preview, confirmed under a new request ID, answers `ok`, 0, not
    `busy`. The server's context is then the checkpoint, from exactly one
    accepted push that creates the branch. Only this proves the reservation
    free.
- `c4_collaboration_resume_meets_a_deleted_branch` (carry 2)
  - The same setup, then `operation resume` of the comment's operation in
    human mode: exit 3.
  - On stderr: `progress: accepted`, `observing`, `fetching`, `observing`.
    Then `error: …`, `code: remote_branch_deleted`, `request: R3` and
    `effects: publication=pending`.
  - One `next:` line, `item republish --prepare`, then the publication note
    and the new note.
  - No record is kept: `request show R3` is `request_not_found`.
  - JSON: `remote_branch_deleted`, `blocked`, 3, `recovery: []`, effects
    publication `pending` and write `not_requested`, and a
    `CommentMutationDto` with publication `pending`.
  - Following the note frees the reservation, as in carry 1.

**Observed for the controller (carry 2).** Before the fix, the resume's human
output printed no `next:` line and no way forward: `operation resume` offers
no `request.cancel`, and the publication's own request holds the reservation.
The ruling's note and `next:` line now name it. The resume itself keeps no
record (`request_not_found`).

**RED and GREEN.**

- **RED** (`logs/task-12a-red.log`): 5 cases passed. The 3 golden cases
  (`item_sync`, `repo_sync`, `resume_publishes_a_stopped_comment`) failed at
  the missing golden (`tests/support/cli.rs:423`).
- The goldens were written with `MANYHANDS_UPDATE_GOLDEN=1`, with `CI`
  unset, and each was read.
- `item_sync_ok`, `repo_sync_ok` and `operation_resume_comment_ok` are
  registered in `CLI_CASES`.

**Verify.** Each ran as `devenv shell -- cargo … --locked` (`fmt --check`
takes no `--locked`); logs are in `logs/task-12a-*`.

| Command | Result |
| --- | --- |
| `test --all-features --test cli_remote -- c4_collaboration` | 8 SSH cases passed (1 m 48 s) |
| `test --all-features --bin manyhands-cli` (the fix) | 252 passed (249 before) |
| `test --all-features --bin manyhands-cli -- human_collaboration` | 36 passed |
| `test --all-features --test cli -- fixture_directory_is_exactly_the_registered_cases` | 1 passed |
| `clippy --all-targets --all-features -- -D warnings` | passes |
| `fmt --check` | passes |
| `check --bin manyhands-cli` | passes |

**For later tasks.**

- **Task 12b.** Use `CliPair` as it stands.
  - Each clone has `ITEM` and `OTHER_ITEM` on primary, and no context until
    its first write.
  - A `ticket save` makes a context without publishing it, which is how B
    diverged in the item sync case.
  - Delete the server's context with `p.bare()?.find_reference(CONTEXT)` and
    `delete()`.
  - For refused pushes, use `set_head(CONTEXT)` with
    `reject_primary_updates(true)`.
  - Keep the key-before-enable order if a case selects another key.
  - A comment's resume names its item in the data, not the scope.
- **Task 13.**
  - Codes reached through the binary over SSH here: `comment add` gives `ok`,
    `push_rejected`, `transport_unavailable`, `remote_branch_deleted`,
    `cancelled` and `request_mismatch`.
  - `item sync` and `repo sync` give `ok`, `already_applied` and
    `request_mismatch`.
  - `operation resume` gives `ok`, `remote_branch_deleted` and
    `request_mismatch`.
- **Task 14.** The RFC can cite the resume's deleted-branch note (ruling 4)
  and the key-before-enable order.
- **Owner and main.** `01M669FJR27FRHCNPWAMD6VH4V` (remote selection's
  stale index). When it lands, `CliPair` can select the publication remote
  itself on one clone.

#### Task 12a, fix round 1

The commit is `test: prove the freed reservation by a republish, and close
the review's minors`, on FIX_BASE `ee22988e`.

- **I-1.** The deleted-branch cases now confirm the republish. A preview
  takes no reservation, so on its own it proved nothing. `free_the_reservation`
  confirms the preview under a new request ID, and asserts `ok`, 0, the
  server's context at the checkpoint, and exactly one accepted push, which
  creates the branch. Both carry cases use it, and both pass.
- **M-1.** The `item sync` mismatch makes no `OTHER_ITEM` context. The
  resume's JSON replay and its mismatch leave A's context unchanged. The
  `repo sync` replay leaves B's primary unchanged.
- **M-2.** Carry 1 matches the whole human effects line through
  `commit=<checkpoint>`, read from A's context tip before the output
  assertions.
- **M-5.** The setup line notes the shared key file (see `CliPair` above).
- **The owner's exploration** (uncommitted; `logs/task-12a-explore-history-unknown-cli.log`).
  A key re-selected after the publication remote is configured leaves that
  registry unable to publish a never-published context. Every `item sync`
  stops after its fetch:
  - It answers `recovery_required`, 3, with `[request.retry, repo.inspect]`,
    and holds the reservation, so other syncs are `busy`.
  - Only `request cancel` plus the same request releases the reservation,
    and the next sync repeats the stop.
  - Nothing else works: `operation resume` (`recovery_required`), `operation
    abandon` (`invalid_input`), `item republish --prepare` (`already_applied`)
    and `comment add` (`recovery_required`).
  - The report has the details, for the owner.
- **Verify** (`logs/task-12a-fix1-*`):
  - `test --all-features --test cli_remote -- c4_collaboration`: 8 SSH cases
    passed;
  - `clippy --all-targets --all-features -- -D warnings`: passes;
  - `fmt --check`: passes.

### Task 12b: Two Clones: Conflicts And Republish

The commit is `test: prove C4 conflict and republish journeys across two
clones`, on BASE `35200326`. `origin/main` is still `a372edb`, and
`git merge-tree --write-tree HEAD origin/main` exits 0. No subagent was
dispatched. No `src/` file changed, so no `--lib` target ran.

**Files.**

- `tests/cli_remote/conflicts.rs` (new): 10 `c4_conflicts_` SSH cases, each
  a fresh `CliPair`. Two are `#[cfg(unix)]`: the PTY case and the hand-edit
  case, which stages its stop with a directory's permissions.
- `tests/cli_remote.rs`: the mount and one `CASES` chain, additive.
- `tests/cli_remote/collaboration.rs`: twelve helpers and `RESERVATION_NOTE`
  made `pub(super)`, so that the new module shares them. Visibility only.
- `tests/support/cli.rs`: `conflict_show_ok`, `conflict_resolve_ok`,
  `item_republish_preview` and `item_republish_ok` in `CLI_CASES`, and
  `conflict_show_text` in `CLI_TEXT_CASES`, with their five fixtures.

**How the clones diverge.** Every Manyhands step goes through the binary.
Git builds only what a person would do outside it:

- a commit of their own on primary, which retitles both tickets or edits a
  code file;
- clone B, as a peer with its own Git, pushing the deletion of the item's
  branch, as `tests/ticket_closure.rs`'s `peer_delete` does;
- the outside repairs in B's worktree.

An item's context is made by a `ticket save`. A resolution's source holds a
line that is a probe of every later output.

**The cases.**

- `c4_conflicts_canonical_shown_and_resolved`. A retitles both tickets on
  primary and publishes; B retitles them otherwise.
  - B's `repo sync` answers `recovery_required`, 3: integration and
    publication `pending`, and `synchronization_conflict` data with
    `next_action: resolve`. Its recovery is `[request.retry, repo.inspect]`.
  - `conflict show --json`: `conflict_pending`, `resolve`,
    `eligible_canonical`, both parents, and two `ticket` paths in byte
    order, each `regular` with its three sides. This is the
    `conflict_show_ok` golden.
  - The text is the `conflict_show_text` golden, with no side printed.
  - `conflict resolve` with the shown tokens answers `ok`, 0: written,
    committed, published, integration `complete`. One merge has parents
    `[local, incoming]` and the subject `Resolve synchronization primary`,
    published by one push. This is the `conflict_resolve_ok` golden. The
    read is then `complete`.
  - A replay is identical, with no connection.
  - Another `--operation-id` under the same request ID is
    `request_mismatch`, 2.
  - A second, different resolution is `recovery_required`, 3,
    `[repo.inspect]`, and keeps no record.
  - A's `repo sync` fast-forwards to the resolution.
- `c4_conflicts_stale_resolution`.
  - The stop in human mode: the progress lines, then C2's lines, the
    `conflict:` line, and `next: manyhands-cli conflict show …`, followed
    by C2's `request.retry` and `repo.inspect` lines. The case then runs the
    `conflict show` line as printed.
  - The stale resolutions, each refused before any write. No record is
    kept, no file or tip changes, no attempt begins, and nothing connects:
    - An observation taken before `.manyhands/config.toml` changed (the
      observation digests it) is `external_change`, 3, with no recovery.
    - An incomplete set is `invalid_input`, 2.
    - A well-formed token of no path of the conflict is `external_change`,
      3 (R-33).
  - The resolution in human mode prints C2's lines and the data. On
    stderr: `progress: accepted`, `writing`, `fetching`, `observing`,
    `pushing`, `verifying`, `indexing`.
- `c4_conflicts_outside_repair`. Both clones hold `src/conflict.rs`, and each
  edits it.
  - Human mode prints `next action: external_repair` and the
    `conflict show` line.
  - The read is `conflict_pending`, `external_repair`,
    `external_resolution_required`. The path is `noncanonical`, structure
    `binary` (see below), with no sides.
  - `conflict resolve` is `recovery_required`, 3, `[repo.inspect]`. Its
    data names `external_repair`, and nothing is written.
  - Five repairs that are not accepted:
    - the markers removed alone read `conflict_pending`, and `operation
      resume` answers `recovery_required` with the conflict's data;
    - the path staged without a commit reads `repair_refused`;
    - three clean commits with wrong parents (one parent, reversed, a
      foreign second parent) each read `repair_refused`.

    Each `operation resume` is `recovery_required`, 3, `[repo.inspect]`.
    Nothing connects or pushes, and each undo reads `conflict_pending`
    again.
  - The person's two-parent merge reads `repaired`, `resume`. `operation
    resume` in human mode publishes it with one push. On stderr:
    `progress: accepted`, `observing`, `fetching`, `observing`, `pushing`,
    `verifying`, `indexing`. The read is then `complete`, and A
    fast-forwards.
- `c4_conflicts_resolve_meets_a_deleted_branch` (controller carry 2). An item
  conflict: A saves and publishes, and B saves, so B's `item sync` stops.
  The peer then deletes the branch, and B resolves in human mode.
  - Exit 4, `remote_branch_deleted`, with `effects: write=written
    checkpoint=committed discovery=pending publication=pending
    integration=complete commit=<resolution>`.
  - Three `next:` lines: `request cancel --request-id <SYNC>`, `send request
    <SYNC> again …` and `item republish --repo '<root B>' --id <ITEM>
    --prepare`. The first two name B's `item sync`, not the resolve.
  - Then the note: `request <SYNC> keeps its reservation …`.
  - The read the human output uses is `resolved`, `resume`, with `item_id`
    present, so the third line prints.
  - JSON is `partial`, 4, `[request.cancel(SYNC)]`.
  - Followed: the cancel; the same `item sync`, which ends `cancelled`, 130;
    the preview (`local_oid` the resolution, `last_published_oid` A's
    publication); and the confirm, which creates the branch at the
    resolution with one push. A then fast-forwards.
- `c4_conflicts_hand_edit_after_written_paths` (controller carry 3; ruling
  I1). The binary can stage it without a test hook. The second path's
  directory is made read-only, so the resolve writes the first path, then
  stops.
  - It answers `internal_error`, exit 4, `effects: write=written`.
  - The read is `resolution_prepared`, `retry_resolution`, naming the
    request, with `historical: true`.
  - The written path is then edited by hand.
    - A new request is `original_request_required`, 3, with `next: send
      request R2 again …` and Task 9's edited note. It keeps no record.
    - The first request is `external_change`, 3, with the restore note.
    - Once the path is restored to the request's resolution, the first
      request answers `ok`, and publishes one merge.
- `c4_conflicts_republish_busy_then_confirmed` (controller carry 1, table
  row 13).
  - `item sync` in human mode is `remote_branch_deleted`, 3, with three
    `next:` lines: cancel, the full `item sync … --request-id <SYNC>`, and
    the republish preview. Then the reservation note. The branch is not
    recreated.
  - The preview connects to nothing.
  - The confirm in human mode is `busy`, 5. Each of the four attempts
    prints `progress: accepted` and `connecting`, and three
    `progress: retrying` lines come between them.
  - Task 10's note is the line before `next: request.retry
    request_id=<R3>`, and no record is kept.
  - After the cancel and the cancelled `item sync`, the same request
    publishes, in human mode. On stderr: `accepted`, `connecting`,
    `fetching`, `observing`, `pushing`, `verifying`, `indexing`.
  - The JSON replay is identical, with no connection.
  - `--id OTHER_ITEM` under the same request ID is `request_mismatch`, 2,
    and makes no context.
- `c4_conflicts_republish_confirmed`.
  - `item sync` is `remote_branch_deleted`, `blocked`, 3,
    `[request.cancel]`, and the record stays `accepted`. Then the release.
  - The preview has `branch_republished`, the branch, `origin`, and both
    commits equal to A's tip. This is the `item_republish_preview` golden.
  - The confirm creates the branch at exactly that commit with one push, and
    integrates nothing. This is the `item_republish_ok` golden.
  - A later `item sync` is `already_applied`, publication `current` (row
    17).
- `c4_conflicts_republish_local_moved`. A preview, then a person's `ticket
  save` moves the local branch.
  - The confirm is `external_change`, 3, `[request.prepare]`, with no
    record, no connection and no push.
  - A new preview shows the new commit, and its confirm pushes it.
- `c4_conflicts_republish_transport_stop`. The server drops the connection
  at its advertisement.
  - The confirm is `transport_unavailable`, 5, `error`, publication
    `pending`, `[request.retry(R3)]`, and the record stays `accepted`.
  - The same request in human mode completes it under the same operation.
    On stderr: `connecting`, `fetching`, `observing`, `pushing`,
    `verifying`, `indexing`, with no `accepted`, because the request was
    accepted already.
- `c4_conflicts_republish_ctrl_c_at_consent` (Review Focus 4, `#[cfg(unix)]`).
  - At the PTY, `item republish` shows the preview and asks. A Ctrl-C
    exits 130 with `code: cancelled`, no progress, and the terminal modes
    unchanged.
  - The allocated request has no record, nothing connected and nothing was
    pushed.
  - A second run answered `yes` republishes, so nothing was reserved.

**Observed for the controller.**

- **A code file's structure.** A text code file that both clones edited
  reads `structure: binary`, and an add/add of a new code file (seen while
  exploring) reads `delete`. The gate's classification (fact 30) gives both.
  The library's own read table pins `binary` for its `plain.sh`, so neither
  is a stop. `conflict show`'s text then prints `structure=binary` for a
  text file, and `structure=delete` for two adds.
- **Carry 3's first stop is `internal_error`.** A write the file system
  refuses, after a first path was written, answers `internal_error` with no
  recovery action. Only `conflict show` names the way on
  (`retry_resolution`, with the request).
- **A `busy` republish carries an `operation_id`** in its envelope, yet keeps
  no record. The library's row asserts the same: no record.

**RED and GREEN.**

- **RED** (`logs/task-12b-red.log`): 6 cases passed and 4 failed.
  - The 2 golden cases failed at the missing fixture
    (`tests/support/cli.rs:428`).
  - The outside-repair case had `structure` pinned `regular`. It is
    `binary`, as the library's table has it.
  - The PTY case failed on the terminal's `\r\n` line ends.
- The goldens were written with `MANYHANDS_UPDATE_GOLDEN=1`, with `CI` unset
  (`logs/task-12b-golden-update.log`), and each was read.
- **GREEN** (`logs/task-12b-green.log`): 10 SSH cases passed, in 1 m 47 s.
  No case came near the watchdog.

**Verify.** Each ran as `devenv shell -- cargo … --locked` (`fmt --check`
takes no `--locked`); logs are in `logs/task-12b-*`.

| Command | Result |
| --- | --- |
| `test --all-features --test cli_remote -- c4_conflicts` | 10 SSH cases passed (1 m 47 s; `logs/task-12b-verify.log`) |
| `test --all-features --test cli_remote` (whole target, once) | 50 SSH cases passed (4 m 25 s; `logs/task-12b-cli-remote.log`) |
| `test --all-features --test cli -- fixture_directory_is_exactly_the_registered_cases` | 1 passed |
| `clippy --all-targets --all-features -- -D warnings` | passes |
| `fmt --check` | passes |

An earlier whole-target run is void, and its log is kept as
`logs/task-12b-cli-remote-invalidated.log`. While it ran, I rebuilt the
test binary with a comment-only edit and a `--list`. The harness spawns each
case from `current_exe()`, so from then on every case failed, at once and
with no panic line: 38 passed and 12 failed. The run above followed, with
the sources frozen.

**For later tasks.**

- **Task 13.** Codes reached here through the binary over SSH:
  - `conflict show`: `ok`;
  - `conflict resolve`: `ok`, `external_change`, `invalid_input`,
    `recovery_required`, `request_mismatch`, `remote_branch_deleted`,
    `original_request_required` and `internal_error`;
  - `item republish`: `ok`, `busy`, `external_change`,
    `transport_unavailable`, `cancelled` (at the prompt) and
    `request_mismatch`;
  - `repo sync` and `item sync`: `recovery_required`;
  - `item sync`: `remote_branch_deleted` and `cancelled`;
  - `operation resume`: `ok` and `recovery_required`.
- **Task 14.**
  - Republish row 13 still says human output names `request cancel` for the
    holding request. Through the binary it prints Task 10's note, which
    names the step, so Task 10's carried amendment stands.
  - The code-file structures above may deserve a sentence.
- **The owner's on-screen check.** `conflict_show_text` is the golden for
  M5, the lower-case labels: a canonical conflict with two paths.

#### Task 12b, fix round 1

The commit is `test: restore permissions on every path, share the save
helper, and close the review's minors`, on FIX_BASE `aeed23ef`. No `src/`
change.

- **I-1.** The hand-edit case reads the directory's mode before its chmod.
  - Right after the chmod, it makes a `ModeRestored` guard, whose `Drop`
    puts the mode back.
  - It drops the guard explicitly after the resolve's run, then asserts the
    mode is the original.
  - A panic or a CLI deadline now unwinds through the guard. A SIGKILL from
    the watchdog runs no `Drop`, so nothing in the process can cover it.
- **I-2.** `collaboration.rs` now has `pub(super) fn save(clone, request,
  body)`, and `save_the_ticket` is a one-line wrapper around it (the
  controller's authorization). `conflicts.rs` uses `save`, and its copy is
  gone.
- **M-3.** `EDITED_NOTE` and `HAND_EDIT_NOTE` moved into `mod unix`, beside
  their one use.
- **M-4.** `cli_pair.rs` has one `pub fn callbacks(server)`, the
  credentials and the host-key check. `CliClone::cloned` and
  `peer_deletes` both use it.
- **M-5.** Row 13's human stderr is pinned by exact equality, with the busy
  result's `operation:` line read from the output.
- **M-6.** The republish `request_mismatch` asserts that every effect is
  `not_requested`.
- **Native CI margin, accepted.** The PTY "yes" run waits up to 10 s, and
  `outside_repair` is the heaviest case (over 30 CLI runs after the pair's
  setup). Both go to
  native CI's margin notes.
- **Verify** (`logs/task-12b-fix1-*`):
  - `test --all-features --test cli_remote -- c4_conflicts`: 10 SSH cases
    passed (2 m 17 s);
  - `test --all-features --test cli_remote -- c4_collaboration`: 8 SSH cases
    passed (2 m 3 s), after the save helper moved;
  - `clippy --all-targets --all-features -- -D warnings`: passes;
  - `fmt --check`: passes.

### Task 13: Interruption, Lost Output And Exit Evidence

The commit is `test: prove C4 interruption, lost output and exit coverage`,
on BASE `ed5814f8`. No subagent was dispatched. No `src/` file changed, so no
`--lib` target ran.

**`origin/main` moved during the task.** It was `a372edb` at the start, and
is now `bda0a2d1`, the merge of D3 (`01M4CC0VN0MFTGKQ4HGTTC5H63`).

- `git merge-tree --write-tree HEAD origin/main` exits 1, with conflicts in
  `src/repository/mutation/{bind/mod.rs,command.rs,dto.rs,mod.rs}`,
  `tests/mutation_contract.rs` and `tests/support/conformance.rs`.
- D3 changes none of Task 13's files, nor the audit the exit table reads.
- As the brief says, the task stopped with NEEDS_CONTEXT before its commit
  and push. The work below is verified on BASE.
- The controller committed it on BASE as verified. Main is merged in a
  separate merge commit after it, and Task 13's Verify runs again on the
  merged tree.

**Files.**

- `tests/support/pty.rs`: one additive helper, `PtyCli::finish_any(self)
  -> (Option<i32>, Option<i32>, String)` (exit code, signal, stderr), beside
  `finish`. Nothing else in the file changed.
- `tests/cli_remote/interruption.rs`: 15 new `c4_interruption_` SSH cases
  beside Task 7's spike, 8 of them `#[cfg(unix)]` (the pseudo-terminal
  cases). The spike's lines are unchanged except that its `approve` moved to
  the module's top, so that a cross-platform case shares it, and its
  `transport::*` import went with it.
- `tests/cli/collaboration_exits.rs` (new), mounted from `tests/cli.rs`: the
  exit table and 7 tests that reach codes with no server.
- `tests/cli_remote/collaboration.rs` and `conflicts.rs`: helpers made
  `pub(super)`, visibility only (with `rustfmt`'s rewrapping), so the new
  cases share them, as 12b did.
- No golden, no `CASES` of `tests/support/cli.rs`, and no `src/` change.

**How a press or a cancellation lands.** The server holds the next
advertisement (`hold_advertisement`) and the case waits until it is held,
so the run is blocked on the transfer: the fetch, for every verb below. A
press is sent through the pseudo-terminal and given 500 ms to land, as the
spike does; a cancellation is asked from a second invocation. The transfer
is then let go. No case sleeps for the run to reach a point.

**Decision 14: each Ctrl-C case's answer** (controller ruling 1). Every press
landed while the run's fetch was held. Every answer is `cancelled`, exit 130,
at the safe point after the fetch, with its effects intact: what the effects
name is what the clone and the server hold, and nothing was pushed. None is
misreported, so nothing stops for the owner.

| Case | Verb (staging) | Answer | Effects | What completes the work |
| --- | --- | --- | --- | --- |
| `c4_interruption_held_transfer` (Task 7) | `item sync`, `World`, held after 128 KiB of the fetch | `cancelled`, 130 | `publication=pending` | — (Task 7's spike, unchanged) |
| `item_sync_then_a_new_request` | `item sync` of B, diverged from A | `cancelled`, 130, `progress: accepted, connecting, fetching, observing, stopping` | `publication=pending`; nothing merged or pushed | the same request returns the stored result with no connection; a new request merges `[local, incoming]` and publishes once |
| `comment_add_then_item_sync` | `comment add`, A's first comment | `cancelled`, 130 | `write=written checkpoint=committed discovery=current publication=pending`; nothing pushed | the same request returns the stored result; a new `item sync` publishes the checkpoint (comment table, "Ctrl-C during publication, before a push") |
| `conflict_resolve_then_repo_sync` | `conflict resolve` of a canonical conflict on primary, held at its continuation's fetch | `cancelled`, 130, no `operation:` line (R-35) | `write=written checkpoint=committed discovery=pending publication=pending integration=complete`; nothing pushed | `conflict show` reads `cancelled`, `synchronize`; a new `repo sync` publishes the resolution commit (interrupted-resolution table, last row) |
| `item_republish_then_a_new_prepare` | `item republish --confirm` after a peer's deletion | `cancelled`, 130 | `publication=pending`; the branch is not recreated | the confirmation is spent (`confirmation_used`, 3, no record); a new prepare, confirmed, republishes once (republish table, "Ctrl-C during execution") |
| `operation_resume_then_item_sync` | `operation resume` of a comment whose publication stopped at the advertisement | `cancelled`, 130, no `operation:` line | `publication=pending`; nothing pushed | the comment's own request then answers `cancelled`, 130 (its synchronization is final, fact 56; R-34's `Final` standing); a new `item sync` publishes the comment |

`request show` agrees with each printed result: `finished`, outcome and code
`cancelled`, the same effects. A resolve's and a resume's record names no
operation of its own.

**A second press** (`c4_interruption_second_press_ends_the_process`). Two
presses 500 ms apart while the fetch is held: the process dies by `SIGINT`
(`finish_any` gives `(None, Some(2))`), and stderr holds only `progress:
accepted, connecting, fetching`. Nothing is merged or pushed. Then:

- `request show` has the request `accepted`, no result, and its operation
  pending (`operation list`: one `synchronize_context`, no failure);
- **a new request is `busy`, 5**, offering only its own retry: the
  interrupted request still holds the reservation, as a blocked
  synchronization's does (Known Limits, `01M4R19PC0T6ZPKM39X9DF26HX`);
- **the interrupted request, sent again, takes its operation up** and
  completes the work: one merge, published once, under the same operation;
- the new request then answers `already_applied`.

So "a new request recovers" holds once the interrupted request is sent again.
A new request ID alone does not: it is `busy` until then. Recorded for the
controller; no design row covers it.

**The unlock prompt** (`c4_interruption_unlock_prompt`, `World` with its
protected key, `item sync` in human mode without `--non-interactive`). The
handler is installed, because `item sync` runs inside the window. A press at
`Passphrase for key` is read by the prompt in raw mode:

- it answers `unlock_required`, exit 3, with `next: request.retry`, not
  `cancelled`; the process is not ended by a signal; the terminal's modes
  are restored; the passphrase prompt appears once;
- the request stays `accepted`, and nothing reaches the server;
- the same request with the right passphrase, the server refusing the key:
  `unlock_failed`, 3, under the same operation, still accepted, with nothing
  pushed.

The case first had a third run: the right passphrase again, once the server
took the key, publishing the item. It passed in exploration and in the first
runs.

- One later run of the case, under load, hit the PTY's 10 s deadline
  (`pty.rs:309`). The log does not show which of its terminal runs hit it.
- The publishing run was the slowest: a debug build unlocks the
  bcrypt-protected key, then fetches, pushes and verifies, all inside that
  deadline.
- No row requires that run, so it was dropped. The case then passed alone
  three times (`logs/task-13-unlock-rerun.log`).

**The latch (R-36)** (`c4_interruption_press_after_the_last_safe_point`).

- **Where the window is.** In a push, the library's last check of the
  cancellation is on the push's connection, before the push is sent
  (`with_scoped_action_remote`). A refused push returns at `push_exact`'s
  `?`, before the check after it and before `AfterPushReturn`.
- **First staging, which landed too early.** Holding the push's advertisement
  and pressing there answered `cancelled`, 130 (`progress: pushing,
  stopping`; the merge committed; nothing pushed): the connection's check saw
  the press. That is a safe point, so it could not show the latch.
- **The staging used.** Each connection is held in turn until the push's. The
  server's answer to the push is stalled (`stall_at(Transfer, 5 s)`). The
  fixture's receive race (`race_update`) moves the branch to another clone's
  commit at the moment the client's push command arrives, which it sends
  only after that last check. The case polls for that mark, bounded at 30 s,
  then presses. The stall starts only once the command has arrived, so a
  slow client never shortens the press's window. The server refuses the
  update of its checked-out branch (`receive.denyCurrentBranch`).
- **The answer.** `push_rejected`, exit 4, partial, with C2's refusal lines
  and `next: request.retry`, and no `retrying` or `stopping`. There was one
  refused push of the merge: **no second execute ran**. The record stays
  `accepted`.
  - This is not one of decision 14's two answers, because the press was
    seen by no safe point: the run's own answer stands. Its effects are
    intact (`publication=pending integration=complete commit=<merge>`, the
    merge kept, nothing accepted), so it is not a stop.
- **The control.** The same request without a press is retried by C2:
  `push_rejected`, 4, with **four** refused pushes. Its first execute merges
  the other clone's commit. Once the server takes pushes, the same request
  publishes that merge.

**`request cancel` (Step 3).** Each verb runs in JSON mode in the background
while the fetch is held. A second invocation, with a `CliEnv` of its own over
the same data directory, asks `request cancel`, which answers
`{state: accepted, requested: true, acknowledged: false}`. With the fetch let
go, the run stops at its next safe point: `cancelled`, 130, with nothing on
stderr and no recovery. Asked again, the request is
`{state: finished, requested: false, acknowledged: true}`. Then:

| Case | Effects of the cancelled run | A new request |
| --- | --- | --- |
| `request_cancel_of_comment_add` | written, committed, `publication=pending`; nothing pushed | `item sync` publishes the checkpoint |
| `request_cancel_of_item_sync` | `publication=pending`; nothing merged | merges and publishes once |
| `request_cancel_of_repo_sync` | `publication=pending`; A's commit not pushed | publishes it once |
| `request_cancel_of_item_republish` | `publication=pending`; nothing recreated | the spent confirmation is `confirmation_used`, 3; a new preview's confirmation presented with `--id OTHER_ITEM` is `confirmation_mismatch`, 2 (neither keeps a record or connects); the new preview, confirmed, republishes once |

`conflict resolve` and `operation resume` hold no operation of their own and
are N/A (R-35, fact 11), as the evidence matrix says.

**Lost stdout (R-25)** (`c4_interruption_lost_stdout`). Each run is spawned
through `cli_command` with its stdout pipe closed before anything is
written, as `tests/cli/boundaries.rs:247` closes it.

- A's first `comment add` and B's divergent `item sync` each exit 1 with
  nothing on stderr. Their work is done: the comment is published, and the
  merge is published.
- `request show` returns each stored result (`ok`, `finished`).
- Each request sent again replays it: the same outcome, code, effects and
  data, with no new connection and no push. A has one comment file and one
  checkpoint of it, and B's tip is unchanged.

**Exit evidence that needs the server (R-27).**

- `c4_interruption_transport_refusals` (`World`, `repo sync`). One request
  meets four transport codes in turn, and is sent again once each recovery
  is done:
  1. `host_approval_required`, with `[host.approve, request.retry]`;
  2. after the approval, `unlock_required` (JSON, protected key), with
     `[request.retry]`;
  3. once the server lets clients in without the key, `key_rejected`;
  4. after the host key is rotated, `host_replacement_required`, with
     `[host.replace, request.retry]`.

  Each is blocked, 3, with publication `pending`. Meanwhile a new request is
  `busy`, 5, and keeps no record, and the first request stays `accepted`.
- `c4_interruption_index_hand_off_refused`. The registry refuses the index
  hand-off, with the library's own trigger (`c4_resume_comment_hand_off`).
  - `comment add` answers `discovery_pending`, partial, exit 4: written,
    committed and published, discovery pending, `[operation.resume]` of the
    comment.
  - While the refusal stands, the resume answers `discovery_pending` again.
  - Without it, the resume does the hand-off with no connection, and the
    comment's own request answers `ok`, with one push and one checkpoint.

**The exit table (R-27, Step 6),** `tests/cli/collaboration_exits.rs`. The
audit's "C4 exit-coverage union" block is read from
`docs/research/wave-03-api-audit.md`: 48 codes, and with C3's `CLI_ONLY` list,
50. The rows plus the reasons equal it by set equality, with no code twice
and none in both. Every row names a `#[test]` of `cli` or a registered `c4_`
case of `cli_remote`. A row marked partial exits 4, and every other row by
its code's own class (fix round 1).
The rows reach exits 0, 1, 2, 3, 4, 5 and 130 through the binary with a C4
verb. The union was not changed.

| Code | Exit | Reached by (verb) |
| --- | --- | --- |
| `ok` | 0 | `c4_collaboration_comment_publishes` (`comment add`) |
| `already_applied` | 0 | `c4_collaboration_item_sync` (`item sync`) |
| `usage_error` | 2 | `the_verbs_that_take_no_input_refuse_input_as_a_usage_error` |
| `invalid_id` | 2 | `a_malformed_operation_id_is_invalid_id_for_both_verbs` (`conflict show`/`resolve`) |
| `invalid_input` | 2 | `conflict_show_of_a_local_operation_that_is_no_synchronization_is_invalid_input` |
| `invalid_path`, `bare_repository`, `not_repository`, `not_repository_root`, `repository_not_enabled` | 2 | `c4_verbs_refuse_each_root` (`repo sync`, and `comment add` but for `repository_not_enabled`, which its item read answers as `repository_not_registered`) |
| `repository_inaccessible`, `repository_not_registered` | 3 | `c4_verbs_refuse_each_root` |
| `item_not_found` | 2 | `item_sync_reads_the_item_s_kind_and_refuses_an_unknown_item` |
| `operation_not_found` | 2 | `a_resume_of_an_operation_no_journal_holds_is_operation_not_found` |
| `occupied_path` | 2 | `a_comment_id_a_file_already_uses_is_occupied_path` |
| `not_confirmable` | 2 | `prepare_of_each_unconfirmed_mutation_is_not_confirmable` |
| `request_mismatch` | 2 | `c4_collaboration_item_sync` |
| `confirmation_mismatch` | 2 | `c4_interruption_request_cancel_of_item_republish` |
| `invalid_remote` | 2 | `repo_sync_refuses_a_remote_or_key_it_cannot_use_before_connecting` (an SSH address with no user) |
| `confirmation_required` | 3 | `an_abandon_of_a_remote_operation_is_refused_and_changes_nothing` |
| `confirmation_not_found` | 3 | `an_unknown_confirmation_is_confirmation_not_found` (`item republish`) |
| `confirmation_used` | 3 | `c4_interruption_request_cancel_of_item_republish` |
| `external_change` | 3 | `c4_conflicts_stale_resolution` |
| `recovery_required` | 3 | `c4_conflicts_canonical_shown_and_resolved` (`repo sync`) |
| `identity_required` | 3 | `comment_add_without_an_identity_is_identity_required` |
| `invalid_configuration`, `worktree_not_clean`, `wrong_branch` | 3 | `comment_add_refuses_what_authoring_refuses` |
| `worktree_conflicted` | 3 | `repo_sync_of_a_conflicted_worktree_is_worktree_conflicted` |
| `ticket_closed` | 3 | `a_comment_on_a_closed_ticket_is_ticket_closed_and_changes_nothing` |
| `publication_remote_required` | 3 | `repo_sync_without_a_publication_remote_is_refused_with_nothing_done` |
| `no_selected_key`, `key_unavailable` | 3 | `repo_sync_refuses_a_remote_or_key_it_cannot_use_before_connecting` |
| `host_approval_required`, `host_replacement_required`, `unlock_required`, `key_rejected` | 3 | `c4_interruption_transport_refusals` (`repo sync`) |
| `index_unavailable` | 3 | `repo_sync_on_a_corrupt_registry_is_index_unavailable` |
| `push_rejected` | 4, partial | `c4_collaboration_comment_refused_then_saved_once` |
| `remote_branch_deleted` | 4, partial | `c4_collaboration_comment_meets_a_deleted_branch` |
| `discovery_pending` | 4, partial | `c4_interruption_index_hand_off_refused` (`comment add`) |
| `busy` | 5 | `c4_conflicts_republish_busy_then_confirmed` (`item republish`). **Its envelope carries the `operation_id` of the last attempt, yet the request keeps no record** (Task 12b) |
| `poll_yielding` | 5 | `c4_interruption_poll_holds_the_reservation` (`repo sync`, beside a poll's reservation; fix round 1) |
| `transport_unavailable` | 5 | `c4_conflicts_republish_transport_stop` |
| `internal_error` | 1 | `repo_sync_with_an_unopenable_data_directory_is_internal_error` |
| `cancelled` | 130 | `c4_interruption_request_cancel_of_item_sync` (not a terminal case, so on every platform) |
| `unlock_failed` | 3 | Unix: `c4_interruption_unlock_prompt`. Elsewhere: a reason (a passphrase is typed at a terminal; Windows prompting is the owed native check) |
| `original_request_required` | 3 | Unix: `c4_conflicts_hand_edit_after_written_paths`. Elsewhere: a reason (the binary stages it with a read-only directory; the library's interrupted-resolution table pins it on every platform) |

The codes no fixture reaches through the binary with a C4 verb, with the
reasons the table gives:

- **`remote_unavailable`, 5.** libgit2's `NotFound` is its only source. No
  deterministic fixture produces it: the transport tests accept it or
  `transport_unavailable` for one fixture, as C2's table records.
- **`confirmation_expired`, 3.** The ten-minute expiry runs on the library's
  clock, which an isolated CLI child cannot advance, as C2 and C3 record.

**RED and GREEN.**

- **Exploration first.** Each Ctrl-C, `request cancel`, lost-stdout and
  refusal row was first run with a temporary dump hook, to see each answer
  before pinning it. The logs are `logs/task-13-explore-{1,2,3}.log` and
  `logs/task-13-explore-cli-{1,2}.log`. The hook is removed.
  - The first latch staging landed at a safe point (above).
  - The first refusal sequence used a new request per code, and met `busy`.
  - A comment with no remote has no `refresh` hand-off, so
    `discovery_pending` needed the SSH case.
- **The exit table's RED.** With the `discovery_pending` row removed,
  `the_c4_table_and_reasons_account_for_every_code_of_the_audit` fails on set
  equality (`logs/task-13-red-table.log`); restored byte for byte.
- **GREEN.** The Verify list below.

**Runs under load (ruling 4).** The machine's load average was about 18
during the first runs.

- **The first whole `cli_remote` run** (`logs/task-13-cli-remote-1.log`): 64
  passed and 1 failed.
  - The failure was the latch case, at my own assertion that the push
    command arrived within 3 s. I had used the stall for the arrival bound,
    and under that load the client took longer.
  - Nothing was killed, and no product answer was at fault.
  - The bound is now 30 s, and the stall 5 s. The case then passed alone
    three times (`logs/task-13-latch-rerun.log`).
- **The rerun of the whole target** (`logs/task-13-cli-remote.log`) passed
  all 65 cases.
- **A Verify-filter run between them**
  (`logs/task-13-verify-interruption-2.log`) lost the unlock case to the
  PTY's 10 s deadline (above). Its slowest run, the publishing one, was
  dropped.

**For the controller.**

1. **A second press, then a new request ID, is `busy`.** The interrupted
   request holds its reservation until it is sent again or cancelled
   (`01M4R19PC0T6ZPKM39X9DF26HX`). The plan's "a new request recover[s]" holds
   for the same request ID, and for a new one only after that.
2. **The latch's answer is `push_rejected`, not one of decision 14's two.**
   The press is seen by no safe point, by design (R-36). Its effects are
   intact.
3. **After a cancelled resume, the comment's own request answers
   `cancelled`**: its synchronization is final (fact 56). A new `item sync`
   publishes the comment, as the Cancellation section says.
4. **A stop that offers `request.retry` holds the reservation.** That covers
   `unlock_required` after a Ctrl-C at the prompt, the transport refusals and
   `push_rejected`. A new request is `busy` until the same request is sent
   again; the transport-refusal case shows it.
5. **`busy` carries an `operation_id` and keeps no record** (12b), recorded in
   the table's row.
6. **Native margin.** The pressed cases wait 500 ms for a press to land, and
   the latch case stalls the server's answer for 5 s. Both are well inside
   the PTY's 10 s and the watchdog's 100 s.

**Verify.** Each ran as `devenv shell -- cargo … --locked` (`fmt --check`
takes no `--locked`), on the committed sources. The logs are
`logs/task-13-*`.

| Command | Result |
| --- | --- |
| `test --all-features --test cli_remote -- c4_interruption` | **16 SSH cases passed** (2 m 29 s) |
| `test --all-features --test cli -- collaboration_exits` | **11 passed** |
| `clippy --all-targets --all-features -- -D warnings` | passes |
| `fmt --check` | passes |
| `test --all-features --test cli_remote` (whole target, after the latch fix) | **65 SSH cases passed** (6 m 54 s) |
| `test --all-features --test cli` (whole target, once) | **349 passed** (2 m 27 s) |
| `test --all-features --test cli -- fixture_directory_is_exactly_the_registered_cases` | 1 passed (no golden added) |

The whole `cli_remote` run came before the unlock case lost its publishing
run. That change only removes a step, and the case and the filter passed
after it.

#### Task 13, fix round 1

The controller committed the task as `473fa736`, then merged `origin/main`
twice: D3 as `2f2b1f77`, and main's desktop-only clippy fix as `cec0ce42`.
This round's commit, on FIX_BASE `cec0ce42`, is `test: reach poll_yielding
through the binary, and close the review's minors`. `origin/main` is
`f487cc74`, already merged, and `git merge-tree --write-tree HEAD origin/main`
exits 0. No `src/` change, no merge or rebase, and no new ticket comment.

**Controller rulings on the task's concerns.** Concerns 1 and 2 are accepted:
a new request ID is `busy` after a second press, and the latch's
`push_rejected` is R-36's behaviour. The placement of exit evidence in
`interruption.rs` is parked for the whole-branch review.

- **Important 1. `poll_yielding` is reached through the binary.**
  - **Placement.** The new case is `c4_interruption_poll_holds_the_reservation`,
    beside `transport_refusals`, on `CliPair`. A server-less `cli` case would
    not reach the code: the key checks come before the reservation. Task 13's
    transport exploration showed `no_selected_key` answered while another
    request held the reservation.
  - **Staging.** It is staged as Part A's
    `a_poll_holding_the_reservation_is_poll_yielding_and_leaves_no_record` is:
    1. A's first `repo sync` (`already_applied`) makes the remote known to
       the journal.
    2. A `RemoteOperationPriority::Poll` reservation is taken through the
       public `RepositoryService::reserve_remote_operation_with_priority`.
  - **The answer.** `repo sync --json` answers `poll_yielding`, exit 5,
    `error`, with every effect `not_requested` and `[request.retry]` of its
    own request. `request show` gives `request_not_found`. The server saw no
    new authentication and no new socket.
  - The row moved from the reasons to the table, which the mapping above now
    shows. Two codes keep reasons: `remote_unavailable` and
    `confirmation_expired`.
- **Minor 2.** `CANCELLED` and `the_operation` moved into `mod pressed`, their
  only user, so a non-Unix build has no dead code. Checked by flipping the
  file's `cfg(unix)` gates to `cfg(windows)` for one `clippy --test
  cli_remote -D warnings` run, which was clean; the file was then restored
  byte for byte.
- **Minor 3.** `Started::spawn` wraps the child as soon as it is spawned:
  - `started` writes stdin after the wrap;
  - `with_stdout_closed` holds its child in `Started` too, and its stderr
    reader now has `collect_cli`'s 1 s EOF bound (an `mpsc` receive with a
    timeout);
  - a deadline panic leaves the kill and the wait to `Started`'s `Drop`.
- **Minor 4.** `finish_any` unwraps the stdout read's `Result`, as `finish`
  does. That is the only change to `tests/support/pty.rs`.
- **Minor 5.** The unopenable data directory's `request_id` is pinned null,
  the one value the binary gives: an error before dispatch.
- **Minor 6.** Each row has a `partial` marker. The partial rows are
  `push_rejected`, `remote_branch_deleted` and `discovery_pending`, which
  exit 4. Every other row's exit equals its code's own class. The test is
  now `every_c4_row_exits_by_its_code_s_class_unless_partial`, with no
  blanket `|| exit == 4`.
- **Minor 7.** The latch's no-press control re-arms the race: another clone's
  commit lands as the push command arrives, exactly as in the pressed run.
  It gives four refused pushes:
  - the first pushes the merge of the pressed run's competing commit;
  - the second and later push the merge of that merge and the new commit;
  - the server holds the new commit;
  - once the server takes pushes, the same request publishes the final
    merge.
- **Kept, as ruled:** `confirmation_expired` stays a reason, and the reviewer's
  Minor 7 is parked.

**Verify on the merged tree.** Each ran as `devenv shell -- cargo …
--locked` (`fmt --check` takes no `--locked`), on the committed sources. The
logs are `logs/task-13-fix1-*`.

| Command | Result |
| --- | --- |
| `test --all-features --test cli_remote -- c4_interruption` | **17 SSH cases passed** (2 m 35 s) |
| `test --all-features --test cli -- collaboration_exits` | **11 passed** |
| `clippy --all-targets --all-features -- -D warnings` | passes |
| `fmt --check` | passes |
| `test --all-features --test cli_remote` (whole target) | **66 SSH cases passed** (6 m 50 s) |
| `test --all-features --test cli` (whole target) | **349 passed** (2 m 17 s) |

### Task 14: Document Amendments

The commit is `docs: amend RFCs, Wave, F2 design and audit for C4`, on BASE
`46739c73`. `origin/main` is `f487cc74`, already merged, and did not move
during the task. No subagent was dispatched. Only documents changed: nothing
under `src/`, `tests/` or `schemas/`.

Every amendment is dated "Amended 2026-10-30 by C4" and cites its decision or
ruling. A ruling made in the controller's ledger is cited by what it says and
by its task, since that ledger is not committed. AGENTS.md and the skills are
Task 15's, and are not touched.

**The C4 design** (`2026-10-26-wave-03-cli-04-discussion-and-sync-design.md`),
from the carries:

| Section | Amendment | Source |
| --- | --- | --- |
| Execution Status | a note that execution rulings are recorded in place | Task 14 |
| Fact 9 | F2 answers a cancelled sync that owes nothing `already_applied` | Task 4b's row-8 ruling (this ledger, "Recorded for Task 14") |
| Comment Submission, Standing | a code never finished by the same request is `Owed`, with `[request.cancel]`; a refusal after the binding releases it, and an input-class refusal finishes the record | carry 8; Task 2b's concern-2 and review I1 rulings |
| Comment table, `remote_branch_deleted` | the record stays accepted, `[request.cancel]`, three `next:` lines | carry 8 |
| Comment table, no remote then one added | `operation resume` publishes the comment | carry 4 (Part A review M3) |
| Republish, Preview | the no-op preview's `item sync` `next:` line and C2's caveat | carry 14 |
| Republish, Execution step 3 | a pointer to R-18's two further left-out refusals | carry 12 |
| Republish, Records | `push_rejected` finishes the record; a refused republish ends `cancelled` | carries 9 and 11 |
| Republish table, row 4 | the record finishes | carry 9 |
| Republish table, row 13 | human output names the step, not the request ID | carry 13 (and Task 12b's note) |
| Conflict Inspection | the read asks for sides on every item path, and gives them when the library reads them as text (`content_available`); none for a noncanonical path; what `structure` means for a noncanonical path | Task 3's sides rulings; carry 17 |
| Conflict-states table | two rows: no conflict recorded is `invalid_input`; a failed inspection at `resolution_prepared` answers its own code | carry 6 |
| Conflict Resolution, step 2 | which attempt a request reconciles; an earlier stage's input is `external_change` | carry 5 (Part A review M2) |
| Conflict Resolution, step 3 | what closing a superseded attempt deletes; the phase reset | carry 1 |
| Conflict Resolution, step 6 | the holder's and the comment request's flags; `request cancel` after push intent | carry 2 |
| Resume table, row 8 | option B's answer | carry 3 |
| Resume table, new row | `operation resume` meeting `remote_branch_deleted` | carry 15 |
| Cancellation, Unix | an inherited `SIGINT` ignore is kept | carry 18 |
| Cancellation, new bullet | after a second press, the same request first | carry 23 |
| Human Output | three `next:` lines, and the holder's request for a resolve; the resume, busy and no-op notes; the hand-edit notes | carries 7.1, 8, 13, 14 and 15 |
| R-18 | the fetch-evidence and settle-path refusals left out | carry 12 |
| R-34 | `Owed` for a never-finished code; the input-class refusal | carry 8; Task 2b's review I1 ruling |
| Named Edits | the owner-authorized conformance split beside E-9 | carry 16 |
| Document Amendments | the date used | Task 14 |
| Known Limits | second press; the spike's observed answer; the latch; the corrected hand-edit limit; lock-held refusals; owner rows after registration removal; push-only `Deleted` evidence; the `busy` check beside a promotion's or closure's merge; main's two defects | carries 7, 10, 19, 20, 23 and 24; this ledger's Task 7 note; Task 1's recorded limit |

**The CLI contract RFC:** C4 verbs available, C5 planned; the C4 taxonomy
paragraph and the eight rows, `item republish` new; a new section "C4
collaboration options and input schemas" (the two schemas, examples, token
lifetime); `item republish` as the republish consent, with the no-op
preview's text; conflict data in place of a recovery action; the
`comment add` example with real codes and `created_by`; the gate's paragraph
amended in place (delivered verbs, C5/D5 for the local contract, the three
local sentences scoped, the remote rule, C4's six remote departures); the
conflict paragraph (token input, kind and structure, set-wide resolution,
`conflict show` after an outside repair); the Ctrl-C window, latch, second
press, finality, `request cancel` and lost stdout; and a "C4 evidence and
known limits" subsection, with the resume's deleted-branch note, the
key-before-enable order, and main's two defects. Sources: the design's
Document Amendments, carries 17, 18, 19, 20 and 23, and this ledger's Task 10
and 12a notes.

**The Git workflow RFC:** the bindings merge and republish is its own
confirmed command (decisions 3, 7 and 11); a conflict's next step in data
(decision 8); the gate's "Local recovery amendment" scoped to the local
operation, with the remote rule beside it (decisions 13, 18 and 20); the
incompatible-closure limit on remote (fact 85, decision 6); and the meaning
of `structure` (carry 17).

**The canonical content and comment schema RFC:** `created_by` from the
caller's validated input when given, else the Git identity (decision 9).

**The F2 design:** a dated note under Comment Author, replacing "A caller
cannot supply it", and a dated paragraph under Departures from the RFCs
(decision 9).

**Wave 03:** entry-gate item 3's departure (decision 16); the gate
checkpoint's "C4/D5 must adapt" as C5/D5, keeping "C4/D5 still provide"
(decision 19); track rule 2's recorded ordering (decision 5); track rule 5's
exception (decision 1); the C4 section, with decisions 1–21, the delivery,
the known limits and the owed Windows check; C5 (decision 19); D4 (decision
5); D5 (decisions 5 and 19); and the capability rows "Comment author", the
remote half of "Conflict inspection read", and "Discussion/sync/conflict and
resume".

**The API audit:** a note in the gate's section naming C4's remote
departures (the section is otherwise unchanged); the C4 refresh's pointer to
the reconciliation; `conflict show` counted 12, not 13 (carry 21); the
resume citations named by function (Task 0's deferred minor); a new "C4
delivery reconciliation (2026-10-30)" subsection; the eight C4 inventory
rows reconciled in place; and D26/D27 swept (carry 22). The union block is
byte-identical, 48 codes.

**Not applied.**

- **Carry 24.** R-36 says the press count and cancellation persist and the
  retry stops; it does not say the answer becomes `cancelled`, so R-36 is
  unchanged. The latch's answer is recorded in Known Limits and in the CLI
  RFC instead.
- **The plan's Task 13 text** (`implementation.md`, "a new request
  recover[s]" after a second press) is the plan, which implementers do not
  edit. The design and the CLI RFC now say the same request goes first.

**This ledger's "Task 14" notes.** All 14 lines are applied: 599 (carry
3), 1544, 2073, 2231, 2442, 2641, 2644, 2653, 2806, 3285, 3437, 3553, 3797
and 4068. Four of them say what the carries did not list:
- fact 9's "a cancelled one is left to inspect" (Task 4b);
- the spike's observed `cancelled` beside the Ctrl-C limit (Task 7);
- the RFC's `item republish` text citing the no-op preview's output
  (Task 10);
- the RFC citing the resume's deleted-branch note and the key-before-enable
  order (Task 12a).

**For the controller: five amendments neither the carries nor this ledger
named.** Each tells the truth where a document fell short of the delivered
behavior; no code changed.

1. **R-34 and Standing: an input-class refusal finishes the record.** Task
   2b's review I1 ruling. R-34 said the record of a released binding stays.
2. **Known Limits: the `busy` check beside a promotion's or closure's
   merge.** Task 1's recorded limit, in its fix-round entry above. The
   divergent table's `busy` row does not hold for such a merge.
3. **Conflict Inspection: which paths get sides.** Task 3's sides rulings,
   made explicit (corrected in fix round 1): the read asks the library for
   sides on every item path, `incompatible_closures` included, and gives
   them only when the library reads them as text, as `content_available`
   says. A noncanonical path gets none. R-10 already excluded noncanonical
   paths; this says how the read applies it.
4. **The audit's resume citations,** named by function: Task 0's deferred
   minor, part of Step 2's reconciliation.
5. **Known Limits: the latch's answer.** Carry 24 kept R-36 unchanged; the
   limit states that a press no safe point sees is not reported as
   `cancelled`.

**D3 interactions.** D3 amended the Wave's D3 and D4 sections and one
canonical RFC paragraph. C4's D4 amendment follows D3's, and both hold. The
canonical RFC paragraph D3 changed is not the one C4 amends. D3 touched no
CLI RFC, Git RFC, F2 design or audit text that C4 amends. No C4 amendment
needed rewording for D3.

**Step 3's grep** over the three RFCs, the Wave and the audit:
`publication_pending`, `{path, observation, source}`, "future CLI adapter
contract" and "C4/D5 must adapt" each hit only amendment text that names
what it replaces. `conflict.show`, "never merges", "refuse divergence" and
"`operation show` re-observes" have no hit. Each sentence of fact 85 is
scoped to the local operation, and the capability row keeps the gate's local
half.

**Verify.** Each ran as `devenv shell -- cargo … --locked` (`fmt --check`
takes no `--locked`). The logs are `logs/task-14-*`. Three tests read the
audit with `include_str!`; no other document edited here is read by a test.

| Command | Result |
| --- | --- |
| `test --all-features --test cli -- collaboration_exits` | **11 passed** |
| `test --all-features --test cli -- authoring_exits` | **15 passed** (C3's union and no-op capability) |
| `test --all-features --test cli -- exits::` | **30 passed** (C2's union, with the two above) |
| `fmt --check` | passes |

#### Task 14, fix round 1

The review of `46739c73..079006b8` needed fixes (`task-14-fix-1.md`). The
commit is `docs: correct the held-reservation limits and conflict sides, and
amend the Cycle`, on FIX_BASE `079006b8`. Documents only. No merge, rebase
or new ticket comment.

- **Important 1: held reservations.** The design's two Known Limits now say
  "cancelled and then sent once more", replacing "until … cancelled". They
  cite Task 2b's ruling: asking is not stopping. The divergent table's
  "Remote context deleted" row points to Human Output's three `next:`
  lines, with `item republish` last.
- **Important 2: conflict sides.** The read asks for sides on every item
  path. The library refuses them with `ExternalResolutionRequired` for a
  stage mode other than `100644`, a binary blob or non-UTF-8 text, and
  those paths get no sides and `content_available: false`. The qualifier
  "when the library reads them as UTF-8 text, as `content_available` says"
  is added at:
  - the CLI RFC's taxonomy row and conflict paragraph;
  - the audit's gate note, its `conflict show` inventory row and D26.

  This entry's Conflict Inspection row and its item 3 are corrected. **The
  pushed ticket comment `01M671EVGQJX85S8Q1813X9FBM` says "sides are given
  for every item path"; that sentence is corrected by this round**, and the
  comment is left as pushed.
- **Ruled items 3–5.**
  - The Cycle's Required Contract item 9 and its "Ctrl-C" acceptance row
    gain a dated amendment: a press no safe point sees leaves the in-flight
    answer (`push_rejected`, 4, no retry), and after a second press the same
    request goes first. Each names what it replaces.
  - The design's resume table gains the fenced-comment row
    (`external_change`, Task 4b's ruling), and so does the audit's
    reconciliation bullet.
  - The audit's D24 is swept, as D26 and D27 were.
- **Minors.**
  - **Citations.** The CLI RFC's six bare amendments now cite their source:
    - availability: R-1, decision 7;
    - taxonomy: R-2, R-32;
    - input: decision 9, R-7, R-9;
    - the example: decision 9, R-6;
    - `operation abandon`: R-1, R-30;
    - evidence: R-24, R-25, R-27.
  - **Counts.** This entry now counts 14 ledger lines applied, 599 among
    them, and five uncarried amendments; the report agrees.
  - **Row 13** says "if an earlier `item sync` … was refused", as the
    delivered note does.
  - **Sides on screen.** The RFC says "A side is printed only with
    `--json`".
  - **`created_by`.** In the RFC's input table, and in the canonical RFC
    for consistency, the whole rule as `CommentAuthor::parse` enforces it:
    - at most 256 bytes;
    - ends with `>`, split at the first ` <`;
    - name and email each non-empty, with no `<` or `>`, no control
      character, and no leading or trailing white space;
    - an `@` in the email;
    - a Git signature that gives both back unchanged.
- **Also.** The design's Document Amendments note records the Cycle's
  amendment.

**Verify** (each `devenv shell -- cargo … --locked`, logs
`logs/task-14-fix1-*`):

| Command | Result |
| --- | --- |
| `test --all-features --test cli -- collaboration_exits` | **11 passed** |
| `test --all-features --test cli -- authoring_exits` | **15 passed** |
| `test --all-features --test cli -- exits::` | **30 passed** |

The union block is byte-identical to `079006b8`. Step 3's grep over the
touched files hits only amendment text naming what it replaces, and, in the
Cycle, its own decisions and review record.

### Final review fixes

The final whole-branch review ran as two reviews of `f487cc74..71dd7c3e`,
the library's ("ready with fixes") and the CLI's ("ready"). This is the one
fix dispatch they get (`final-fix-1.md`), on FIX_BASE `71dd7c3e`. Three
commits: the library fix, the CLI and test minors, and the documents. No
merge, rebase or ticket comment. `origin/main` moved during the round to
`48ffc058` (`ci: remove release binary artifact upload`, the Build
workflow only); `git merge-tree --write-tree HEAD origin/main` is clean,
and it is not merged here.

**L1: a refused republish push is prepared again (fixed).** A
`push_rejected` republish ended `cancelled` with its checkpoint at
`push_prepared` and its `push_oid` kept. `push_absence` counted that as an
intent nothing answered.
- **When it bites.** Only where no synchronization of this registry proved
  a publication of the branch, for example a branch another clone published
  that this one only fetched. There a later prepare was the no-op, and
  `item sync` was refused `recovery_required`: a dead loop. Where this
  registry's own `item sync` published the branch, that proof makes the
  absence `Deleted` whatever the intent says, so the preview was already
  offered again. A throwaway case, the brief's four steps on
  `deleted(ByFetch)`, passed before the fix.
- **Readers checked.** Every reader of `push_oid` and of the checkpoint:
  `push_absence` (`sync.rs`), the only one that reads other operations'
  push intent, through `republish_evidence` and
  `synchronization_push_absence_boundary`. The others read the operation's
  own row: `checkpoint_synchronization`, `reconcile_synchronization`,
  `publication_intent` and `settle_synchronization_publication`, the local
  hand-off checks and `restart_synchronization_guarded`, `acknowledge` and
  the safe-point checks (`reservation.rs`); `read_operation`'s validation,
  `lookup_push`, `prepare_integration_window`,
  `publication_previously_intended`, `open_publication_attempt`, the
  attempt chain and the push-proof snapshot overlay (`state.rs`); and the
  sync and republish executors' own `prior` and `evidence` (`sync.rs`).
  None of them needed a change.
- **Why reader-side.** Clearing the intent is forbidden by invariants:
  `read_operation` refuses `push_prepared` without a `push_oid`,
  `checkpoint_synchronization` refuses to change a recorded `push_oid` or to
  go back to `fetch_prepared`, and the code calls the legacy intent
  immutable history. So the brief's reader-side variant was taken. The
  rejection is recorded on the row without a schema change (fact 75): in
  the transaction that acknowledges the cancellation, the new
  `acknowledge_refused_push` sets the outcome to `protocol_rejected`, the
  category a poll records a refused push under. It does so only for a row
  at `push_prepared`, with `before_push` its last step and with no
  publication attempt, authority or reconciliation.
- **The two sides.** `push_absence` ignores a `cancelled` row with that
  outcome at `push_prepared`. `ExternalChange` still uses
  `check_synchronization_requests`, so its outcome stays `cancelled`.
- **Visible change.** `operation show` of such an operation reports phase
  `cancelled` with `failure_code` `protocol_rejected`, where it said
  `cancelled`. No test or golden reads it.
- **The case.** `c4_republish_refused_push_prepared_again`: the branch is
  published outside the registry, and `repo sync` records it and then its
  deletion. A prepare and confirm follow, with the server refusing updates
  of its checked-out branch, and the answer is `push_rejected`. A new
  prepare then offers the preview, and a confirm republishes once. Red
  before the fix, at the new prepare's `ok` assertion: a probe line showed
  `already_applied` with no data, the no-op. Green after.

**L2: a comment's host stop (Known Limit, as ruled).** A throwaway case
(`logs/final-fix1-L2-throwaway.log`) walked both codes through the
boundary:
- **The stop.** `comment add` answers `host_approval_required` (never
  approved) or `host_replacement_required` (key rotated): partial, data
  `comment`, `[request.retry]`, record accepted, reservation held.
- **New requests.** A new `item sync` and a new `repo sync` answer `busy`.
- **Learning the host.** `observe_host` (`host observe --repo`, no
  `--authority`) is `ok`. It names the host and port and the presented
  key, which matches the server's, and reports the pin: none, or the old
  one. `repo inspect` has only the remotes' locations.
- **Approving it.** `host approve` or `host replace` prepares and confirms
  `ok` while the reservation is held.
- **The way on.** The same `comment add` request then answers `ok`,
  published, served, record finished, reservation released, and a later
  `item sync` is `already_applied`.

So there is a way out. The code is unchanged, and the limit names it.

**L3.** `delete_resolution_attempt` resets the step only when the deleted
attempt was `paths_applying`. A new unit test,
`closing_an_unbegun_attempt_keeps_the_step_another_attempt_began`, was red
(`conflict_pending`) before the predicate and green after.

**L4.** `ChangedPath`, `first_changed_path` and `changed_resolution_path`
are gone. `written_paths_unchanged` returns the bool, and
`resolution_paths_unchanged` calls it. Behavior is unchanged: every early
"changed" return is `false`, and the end is `true`.

**C1–C7.**
- **C1.** `discussion.rs`'s module doc names `conflict resolve` as the other
  verb that takes input.
- **C2.** The test constant is `PUBLICATION_NOTE_LINE`. Rustfmt rewrapped
  two `format!` calls.
- **C3.** The malformed-author loop asserts that the envelope does not hold
  the author.
- **C4.** The renames are `cli_refusal_golden` and
  `a_comment_add_refusal_in_human_mode_names_its_code_and_prints_no_note`.
- **C5.** The comment credits the library's consent gate (`consent`,
  `mutation/confirm.rs`).
- **C6.** `finish_any` also returns stdout. Nothing else in `pty.rs`
  changed. The second-press case scans stdout and asserts it empty.
- **C7.** Both consent runs of `c4_conflicts_republish_ctrl_c_at_consent`
  scan their transcripts.

**Documents.**
- **Carries 1–5.** The Cycle's held-reservation limit and `conflict show`
  sides. The RFC's `operation resume` row gains the fenced comment, and RFC
  lines 694 and 921 are rewrapped. The file system refusal is a Known
  Limit.
- **The final review's limits.** L2, with its way out, and M8 go in the
  design's Known Limits, the CLI RFC's C4 limits and the Wave's C4 known
  limits.
- **The design.** The canonical-conflict row gains the `busy` clause (minor
  7), and the Records amendment says what L1 records.
- **Not edited.** The audit is untouched, so the audit-reading exit tests
  were not rerun beyond `collaboration_exits`, which the `collaboration`
  filter runs.

**Verify** (each `devenv shell -- cargo … --locked`, `fmt --check` without
`--locked`; logs `logs/final-fix1-*`):

| Command | Result |
| --- | --- |
| `test --all-features --test mutation_remote -- c4_republish` | **26 passed** (the new case among them) |
| `test --all-features --test mutation_remote -- c4_resolution` | **41 passed** |
| `test --all-features --test mutation_remote -- c4_resume` | first run **15 passed, 2 failed** under load (load average near 13), with no panic location, as the 100 s watchdog fails a case: `c4_resume_prepared_commit` and `c4_resume_cancelled`; each passed alone, and the rerun **17 passed** |
| `test --all-features --lib remote` | **478 passed** |
| `test --all-features --lib mutation` | **125 passed** |
| `test --all-features --lib local_sync` | **164 passed** |
| `test --all-features --lib integration` | **26 passed** |
| `test --all-features --test mutation_replay` | **383 passed** |
| `test --all-features --bin manyhands-cli` | **252 passed** |
| `test --all-features --test cli discussion` | **15 passed** |
| `test --all-features --test cli collaboration` | **23 passed** (`collaboration_exits` among them) |
| `test --all-features --test cli_remote -- c4_interruption` | **17 passed** |
| `test --all-features --test cli_remote -- c4_conflicts_republish_ctrl_c_at_consent` | **1 passed** |
| `clippy --all-targets --all-features -- -D warnings` | passes |
| `clippy --all-targets --features desktop -- -D warnings` | passes |
| `fmt --check` | passes |

The full suite is the controller's, on this result.

### Task 15: Full Verification, Review, Dogfooding, Switch And Handoff

**Merges of main.** `origin/main` moved three times during Tasks 13 to 15.
Each move was merged, never rebased:

- `bda0a2d1`, D3, as `2f2b1f77`. Six conflicts, each a pair of additions:
  C4's `comment add`, `conflict resolve` and `item republish` beside D3's
  `context prepare` and `context release`. The counted lists widened to
  `DATA_ROOTS` 56 and `HOMES` 37. The checkpoint gate ran in full on the
  merge.
- `f487cc74`, D3's desktop-only clippy fix, as `cec0ce42`. Desktop-alone
  clippy passes from here on, so P6's comparison with main's errors ends.
- `48ffc058`, a Build workflow change, as `a58aef28`.

**Step 2: the final whole-branch review.** Two independent reviewers read
`f487cc74..71dd7c3e`, the net change against main, one per side:

- the library side: with fixes, 2 Important;
- the CLI side: ready to merge, Minor only.

The controller ruled on every finding and every deferred item. There was one
fix round, `71dd7c3e..62ae05ee` (see "Final review fixes"), and its
re-review found everything addressed. Its three document notes were closed
in `c19d655b`, which also adds the owner checklist,
`docs/plans/2026-10-26-wave-03-cli-04-owner-checklist.md`.

**Step 1: the gate and the full suite, once, on the final tree.** The run
was moved after the final fix, so that it covers the fixed code. An earlier
start, on `71dd7c3e`, was stopped while it was still in `--lib`, because the
fix's builds would have waited on its build lock.

It ran on `a58aef28`, and everything after that commit is documentation
only. The run took 3 h 34 m (14:04 to 17:38), past the plan's two-hour
budget, on a machine loaded by other sessions. `--no-fail-fast` gave exit 0.

- **Gate.** `check`, `fmt --check`, all-features clippy, desktop-alone
  `check` and clippy, the CLI `check` and `check-cli-no-gpui.sh` all pass.
- **Full suite.** Every target passed:

| Target | Result |
| --- | --- |
| `--lib` | 1477 |
| `src/main.rs` (desktop) | 1082 |
| `--bin manyhands-cli` | 252 |
| `canonical_foundation` | 65 |
| `cli` | 349 |
| `cli_remote` | 66 SSH cases |
| `comment_publication` | 41 SSH cases |
| `discovery_rebuild` | 79 |
| `document_promotion` | 74 SSH cases |
| `drafts` | 48 |
| `editor_feasibility` | 9 |
| `journal_rejection` | 26 |
| `key_material` | 39 |
| `key_storage` | 2 |
| `local_authoring` | 140 |
| `mutation_bridges` | 166 |
| `mutation_confirmation` | 183 |
| `mutation_contract` | 44 |
| `mutation_relationships` | 27 |
| `mutation_remote` | 258 SSH cases |
| `mutation_replay` | 383 |
| `read_boundary` | 20 |
| `read_characterization` | 0, 1 ignored (as on main) |
| `read_comments` | 23 |
| `read_contract` | 72 |
| `read_credentials` | 25 |
| `read_items` | 82 |
| `read_relationships` | 17 |
| `read_repository` | 33 |
| `read_status` | 24 |
| `recovery_foundation_gate` | 52 |
| `remote_merge_recovery` | 25 SSH cases |
| `remote_observation` | 15 SSH cases |
| `remote_polling` | 84 SSH cases |
| `remote_reservation` | 13 |
| `remote_synchronization` | 63 SSH cases |
| `repository_enablement` | 95 |
| `session_credentials` | 11 |
| `shared_key_registry` | 39 |
| `ssh_fixture` | 35 SSH cases |
| `ssh_transport` | 123 SSH cases |
| `ticket_closure` | 73 SSH cases |
| doc-tests | 15 |

**The owner's answers** (2026-10-30):

- Dogfooding: go ahead once the full suite is green.
- Label case: keep it as it is, with a follow-up.
- Follow-up tickets: all four. Each is on its own pushed branch, created with
  main's CLI:
  - `01M67BN4C1SY2HQBGNE9NBYMW8`, the comment host offer;
  - `01M67BN4DMYV41ME9FHEBCZ2RA`, the resolve refusal's `next:` line;
  - `01M67BN4F7M1TYRMTDHB4XNWCM`, the exit-evidence move;
  - `01M67BN4H4F4ZXQG79H2NV7PWS`, consistent human output, with the label
    case.

**Step 3: dogfooding**, with the owner's go-ahead (2026-10-30).

- **Preflight.**
  - The C4 worktree is clean, every edit is committed, and it is pushed.
  - Local `main` equals `origin/main` (`055353f8`, merged into C4 as
    `30db0ff8`; it changes release scripts and workflows only).
  - C4's context is registered as active in main's index.
  - The registry is compatible. C4 adds one table,
    `remote_resolution_owners`, made only if missing. On a copy of the
    shared registry, main's CLI opened it after the fresh branch-built CLI
    had, and the reverse also worked. The real registry already holds the
    table, and main's CLI has opened it since.
  - The publication remote is `origin` (GitHub), selected for publication.
  - **One check failed: no key was registered and no host was pinned.**
    The owner then imported and selected a passphrase-protected key, and
    approved `github.com` (`ssh-ed25519`,
    `SHA256:+DiY3wvvV6TuJJhbpZisF/zLDA0zPMSvHdkr4UvCOqU`, GitHub's
    published fingerprint), request `01M67R56A3PMDR45F5E76T5H47`.
- **The verification comment, recorded before it is sent.**
  - Request `01M67R83D9A9SP5XHG1NBMH93T`, comment
    `01M67R83CD2P5DM4W2A6BYKQ4Q`, parent `01M671EVGQJX85S8Q1813X9FBM`.
  - `created_by` is `Claude Opus 5.5 <noreply@anthropic.com>`.
  - The branch-built CLI sends it at the owner's terminal, because the key
    has a passphrase.

### Dogfooding fix

The owner's decision (2026-10-30): fix this in C4.

**The defect.** Dogfooding step 2 found that `comment add` refused every
comment in the real repository, a brand-new ticket's included, as
`item_not_found` (exit 2, Final, no effects). The controller reproduced it in
a throwaway clone with its own registry.

**The site.** Before it writes, `checkpoint_comment_locally_with` validated
the whole context and refused on any problem, as
`MissingAuthoringTarget` ("the selected context contains nonconforming
canonical content", `src/repository.rs`, then `:3105-3112`, now
`:3116-3127`; from Wave 02's `62e5ee28`). Main's `docs/` holds 21 Markdown
files without Manyhands front matter, each a `missing_front_matter` problem.

- **Proof of the site.** The kind at that one line was changed, for one run
  only, to `NotRepairable`. The library case then failed with
  `NotRepairable`, and the CLI case with `not_repairable`. The line was then
  restored. That site, and no other, refused.
- **The comment path's other reads.** None refused on unrelated problems.
  Each reads only the conforming items, never the problem list:
  - the context's preparation (`canonical_context_at`, and
    `validate_context_worktree`);
  - `refuse_unknown_comment_parent`;
  - the item and parent checks after the binding.

  The receipt replay (`publish_comment_with_config`'s proved-receipt branch,
  which `comment::resumed` and `conflict.rs::comment_continued` reach with
  `restart`) and `checkpoint_comment` read only the comment's own file.
  `bind/comment.rs` reads only the checkpointed comment's author. The
  publication's merging synchronization checks the context tree's items, not
  its problems.
- **Other verbs.** `checkpoint_comment_locally_with` is the comment path's
  alone: the public `submit_comment` and `comment add`. `ticket save`,
  `document save` and the other authoring verbs do not share it, and no other
  site refuses on `problems.is_empty()`.

**The rule.** A canonical problem refuses a comment only when it concerns
what the comment depends on: the new comment itself (its path and its ID, a
duplicate ID included), its item (the target file, missing or
nonconforming), and its parent chain (a parent that is missing, belongs to
another item or forms a cycle). A new `comment_dependency_paths` gives those
paths:

- the comment's path;
- every file that claims the item's ID, the comment's ID, or the ID of a
  comment in the parent chain, followed through every file under each ID
  until the chain ends or meets an ID already taken;
- for a ticket, its deterministic path, whether or not it parses.

The check refuses only for a problem at one of those paths, with the same
code and message. Other problems, such as a plain document's, another item's
or another comment's, no longer refuse.

**Tests.**

- **RED.** `cargo test --all-features --locked --test local_authoring --
  comment_`: 24 passed, 3 failed.
  - `comment_is_written_beside_a_plain_document_without_front_matter` and
    `comment_is_not_refused_by_another_item_s_nonconforming_comment` failed
    with `CommentSubmissionError { kind: MissingAuthoringTarget, .. }`.
  - The third failure was the new refusal case's own expectation: a
    nonconforming item in the context is refused by the context's
    preparation as `MismatchedAuthoringContext`, before the comment's own
    checks. That is unchanged behavior, and the case now expects it.
- **RED, CLI.** `--test cli -- a_plain_markdown_file_under_docs`: exit 2,
  `item_not_found`, every effect `not_requested`, as dogfooding saw.
- **GREEN.** `--test local_authoring -- comment_`: 27 passed.
  `--test cli -- a_plain_markdown_file_under_docs`: 1 passed.
  `--lib a_comment_depends_on_its_path`: 1 passed.

The new and changed tests:

- `tests/local_authoring.rs`:
  - `comment_is_written_beside_a_plain_document_without_front_matter` (new):
    a ticket beside a plain `docs/notes.md`. The comment is written and
    checkpointed, with the plain file untouched in the checkpoint's tree.
    The publication is pending, `NoPublicationRemote`. A reply then passes
    the parent check.
  - `comment_beside_a_plain_document_is_still_refused_for_its_item_or_parent`
    (new): the same fixture, with four cases that still refuse and change
    nothing: a missing parent, a nonconforming parent, a cycle through the
    new comment, and a nonconforming item.
  - `comment_is_not_refused_by_another_item_s_nonconforming_comment`, which
    was `comment_rejects_any_existing_context_problem_without_mutation`
    (updated, the owner's decision). It pinned the old behavior: another
    item's malformed `bad.md` refused a comment on the document. It now
    asserts that the comment is written and checkpointed, and that the
    malformed file is left as it was, outside the checkpoint.
- `src/repository.rs`,
  `a_comment_depends_on_its_path_its_item_and_its_parent_chain_only` (new):
  the dependency set, with a duplicate ID in the chain, a loop back to the
  new comment, and an unparseable ticket. The plain document, a malformed
  file, a sibling reply and another item's comment all have problems, and
  none is the comment's.
- `tests/cli/discussion.rs`,
  `a_plain_markdown_file_under_docs_does_not_block_a_comment` (new): a root
  comment and a reply through the binary with a committed plain
  `docs/notes.md`. Both are `ok`, with publication pending.
- **Unchanged tests that still pin refusal.** These pin refusal for the
  comment's own item or parent, and they pass as they were:
  `comment_rejects_invalid_target_parent_and_conflicting_id_without_mutation`,
  `c4_comment_domain_parent_refused_before_binding`,
  `c4_comment_unknown_parent`, and
  `a_parent_that_is_no_comment_of_this_item_is_item_not_found_and_writes_nothing`.

**Verify.** Each command ran as `devenv shell -- cargo … --locked`, with logs
in `logs/dogfood-fix1-*`:

| Command | Result |
| --- | --- |
| `test --all-features --test local_authoring` | 142 passed |
| `test --all-features --test comment_publication` | 41 SSH cases passed |
| `test --all-features --lib repository::tests` | 23 passed |
| `test --all-features --lib comment_publication` | 37 passed |
| `test --all-features --test mutation_replay` | 383 passed |
| `test --all-features --test mutation_confirmation` | 183 passed |
| `test --all-features --test mutation_bridges` | 166 passed |
| `test --all-features --test cli -- discussion` | 16 passed |
| `test --all-features --test cli_remote -- c4_collaboration` | 8 SSH cases passed |
| `test --all-features --test mutation_remote -- c4_comment` | 35 SSH cases passed |
| `test --all-features --test mutation_remote -- c4_resume` | 17 SSH cases passed |
| `clippy --all-targets --all-features -- -D warnings` | pass |
| `check --features desktop --bin manyhands` | pass |
| `clippy --features desktop --bin manyhands -- -D warnings` | pass |
| `clippy --all-targets --features desktop -- -D warnings` | pass |
| `fmt --check` | pass |

**Documents.** The design's Comment Submission section gains a "Canonical
problems" bullet, and the CLI contract RFC's `comment add` row an amendment.
Both are dated "Amended 2026-10-30 by C4 (dogfooding; the owner's
decision)".

**Main.** `origin/main` moved during the fix, to `676ff193` (release CI and
packaging, and the version fixtures). It is not merged here.
`git merge-tree --write-tree HEAD origin/main` is clean.

**Step 3, first attempt.** The owner ran the comment at the terminal.
Request `01M67R83D9A9SP5XHG1NBMH93T` answered `item_not_found`. It is
final and wrote nothing (operation `01M67R98CQ356B0R2QEGSV0N3Q`); the
worktree was untouched.

- **The cause.** In a throwaway clone with its own registry, `comment add`
  refused every item in this repository, including a brand-new ticket.
  The comment path's whole-context check refused on any canonical problem,
  and main's 21 Markdown files under `docs/` without front matter are
  `missing_front_matter` problems.
- **The owner's decision (2026-10-30):** fix it in C4. See "Dogfooding fix"
  above (`763f8d8a`). Its review approved it, with two document corrections
  and a test comment made by the controller.
- **The sandbox recheck.** In the same throwaway clone, with the fixed CLI,
  the comment, with its parent and noreply author, is written and
  checkpointed. Publication then stops at `no_selected_key`, as expected,
  because the sandbox registry has no key.
- **Merges.** Main moved twice more and was merged: `055353f8` (the signed
  release workflow) as `30db0ff8`, and `676ff193` (release fixes, v0.1.1)
  as `b6b496c8`. After the second, `fmt`, both clippy runs,
  `--bin manyhands-cli` (252) and `--test cli` (350) passed.

**Step 3, retry, recorded before it is sent.**

- Request `01M67X2FHGAANH3Y0N9KK55PVV`, comment
  `01M67X2FGGWT4YQDAJEA1PGTG9`, parent `01M671EVGQJX85S8Q1813X9FBM`.
- `created_by` is `Claude Opus 5.5 <noreply@anthropic.com>`.
- The owner sends it at the terminal. The first request stays spent.

**Step 3, retry: published.** The owner sent request
`01M67X2FHGAANH3Y0N9KK55PVV` at the terminal, typing the key's passphrase
when it connected. The progress ran accepted, writing, fetching, observing,
pushing, verifying and indexing.

- **The answer.** `ok`; `write=written`, `checkpoint=committed`,
  `discovery=current`, `publication=published`; commit `d4863f12`;
  synchronization `01M67XCZ3T990R1KANKDXHHRXF`; operation
  `01M67XCYTF1XDT04CXXQDYDM64`.
- **Checked by value.** `comment list` shows `01M67X2FGGWT4YQDAJEA1PGTG9`
  with parent `01M671EVGQJX85S8Q1813X9FBM` and author
  `Claude Opus 5.5 <noreply@anthropic.com>`, with no problems.
  `origin/manyhands/ticket/01M4CC0VMWEQQ9G4W0DAG7ADPY` is `d4863f12`, pushed
  by Manyhands' own SSH transport. `request show` gives `finished` and `ok`,
  with publication `published`.

**Step 4: the switch is deferred** (the owner, 2026-10-30). The selected key
has a passphrase, and agents run `--non-interactive`. An agent's
`comment add` would write and checkpoint the comment, then stop at
publication with `unlock_required`, holding the remote reservation until a
terminal resend. So C4 makes no instruction change, and AGENTS.md and the
skills of fact 48 are unchanged. The design's Dogfooding step 4, the
Cycle's acceptance row and the Wave's C4 exit evidence record the deferral.

**Step 5: review-ready.** The Cycle and the design are marked
`review-ready`. The Cycle's Status gives the outcome and lists main's
merges.

**Merge, 2026-10-30.** `origin/main` `9bc06d8c` (native dependency fixes,
v0.1.2) is merged as `4edbd141`. It is a version bump to test the release
cycle. By the owner's instruction, no tests ran on it.

**Step 5, the review-ready comment, recorded before it is sent.**

- Request `01M67XSA5R5JZY6WV4FB1N6XVK`, comment
  `01M67XSA523QPFGK4WXQDP88G8`, parent `01M67X2FGGWT4YQDAJEA1PGTG9`.
- `created_by` is `Claude Opus 5.5 <noreply@anthropic.com>`.
- The owner sends it at the terminal.
- `merge-tree` against `origin/main` is clean.

**Step 5: the review-ready comment is published.** The owner sent request
`01M67XSA5R5JZY6WV4FB1N6XVK` at the terminal. It answered `ok` and
published commit `947aa6a0`.

- `comment list` shows comment `01M67XSA523QPFGK4WXQDP88G8` with parent
  `01M67X2FGGWT4YQDAJEA1PGTG9`, the noreply author and no problems.
- The ticket branch on `origin` is at `947aa6a0`.
- The ticket stays **open**.

**The ssh-agent follow-up.** The owner asked for a ticket for ssh-agent
support in Manyhands' SSH transport, expecting it to be useful beyond
comments. It is `01M67Y754SJ1FTYQJFR37Z4ZTY` (`vh-z2d7f`), on its own
pushed branch. It names revisiting the deferred comment switch as its
follow-on.
