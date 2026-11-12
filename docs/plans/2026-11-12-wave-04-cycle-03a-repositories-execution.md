---
title: "Wave 04 Cycle 03a Execution Ledger"
date: 2026-11-12
status: "draft"
author: "Claude"
---

# Wave 04 Cycle 03a Execution Ledger

Written only by the orchestrator. Ticket `01M784Y8C73B39GR4JWENXE5D0`
(`vh-3jpbf`), branch `manyhands/ticket/01M784Y8C73B39GR4JWENXE5D0`.
Execution is subagent-driven and serial (decision 25): one fresh implementer
per task, then a fresh reviewer.

Rulings are recorded as "Ruling: decision — why — cost if wrong". Test edits
are E-1 to E-16 (named) and E-17 onward (further, decision 17), each with its
reason.

## Task state

| Task | Status | Commit | Pushed |
| --- | --- | --- | --- |
| 0 | done | `09437383` (ledger); `2aafa5d0` (ticket comment) | yes |
| 1 | done, review approved with Minors parked | code `a2abc5fa` (base `2aafa5d0`); ledger commit follows it | yes |
| 2 | done, review approved after fix round 1 | code `4979f441` (base `d21b14f5`), fix `b103c639`; ledger commit follows | yes |
| 3 | not started | | |
| 4 | not started | | |
| 5 | not started | | |
| 6 | not started | | |
| 7 | not started | | |
| 8 | not started | | |
| 9 | not started | | |
| 10 | not started | | |

## Task 0: baseline, references, ledger

**Base.** `origin/main` `37be101d`, an ancestor of the branch. `git merge
origin/main` answered "Already up to date". Branch HEAD before the task
commit: `1cfa97fc` (a docs-only commit after the brief's first reading:
settles two stale D6 lines in the Wave and corrects the brief's HEAD
wording; no change to authority or rules). Before it: `aaeaa06e` (approval
comment checkpoint), `942045f5` (brief), `1b68793d` (approval). Main
checkout clean at `37be101d`. No other `manyhands/ticket/...` branch has
landed on main; 03b (`01M784Y94Q3AXT6XJX38F5VK1V`) is still a placeholder.

**Baseline gates** (HEAD `aaeaa06e`, tree identical in `src/` to
`1cfa97fc`), all green, no warnings:

| Gate | Result |
| --- | --- |
| `cargo check --all-features --locked` | exit 0, 55 s |
| `cargo fmt --check` | exit 0, 13 s |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | exit 0, 67 s |

No test ran (plan: the fast tier runs once, at Task 10).

**Facts re-check.** `git diff --stat 7034522f HEAD -- src/desktop
src/repository/read docs/Waves docs/RFC docs/design Cargo.lock` lists docs
only: the Wave document and `docs/design/02`, `03`, `06`, `08`, from the
approval commit and `1cfa97fc`. `git diff --stat 7034522f HEAD -- src
Cargo.lock` is empty, so `gpui-kit`, `gpui-base` and `gpui-component` did
not move (0.6.6) and Baseline Facts 1–70 hold; the design's line numbers
are exact. No stop condition met.

**E-sweep** (over `src/desktop`, tests and production alike). Raw output:
the implementer's scratchpad file, 782 lines. Sites, by edit or task (paths
under `src/desktop/`):

| Edit or task | Sites |
| --- | --- |
| E-1 | `view/workspace_tests.rs`: `selection-*` at :737-746, :769-786, :796, :807-811, :824, :842-845, :967, :1106-1131; `repo-unavailable` :776, :785; the six functions start at :709, :753, :793, :819, :939, :1098 |
| E-2 | `view/workspace_tests.rs` :918-932 (`repositories-*`, `selection-name` :927); `view/settings/repositories_manage_tests.rs` :668 (in the :647 test) |
| E-3 | `view/workspace_tests.rs` :705 (`repo-root`), :1055 (`selection-name`); :1057-1067 and :1159-1162 (`repo-selected`) stay |
| E-4 | `view/tabs_tests.rs` :198 |
| E-5 | `view/keyboard_tests/actions.rs` :179, :190-195, :207-208 |
| E-6 | `view/settings/repositories_manage_tests.rs` :561 (`remove_focus()`), :584, :744, :750 |
| E-7 | `view/item/share_tests.rs` :48 (import) and `SHARE_MAIN_BUTTON` at :237-327, :551, :624, :799 (13 sites) |
| E-8 | `view/keyboard_tests/discussion.rs` :34 |
| E-9 | `model/polling_schedule_tests.rs` (`not_shared` :46, :68, :74; `AttemptTarget {` :11, :122); `model/polling/status_tests.rs` (`status::Facts` :20, :412, :425, :462; `AttemptTarget {` :30, :31); `view/settings/polling_tests.rs` (no sweep hit); `main` on the literals in `model/polling/entry_tests.rs:17`, `retire_loop_tests.rs:375,385`, `model/polling_retiring_tests.rs:18,264`, `services/poll_convert_tests.rs:352,363,484`, `services/scheduler_consent_tests.rs:336-337`, `services/scheduler_loop_tests.rs:60,72`, `services/scheduler_entity_tests.rs:290`, `services/scheduler_retire_tests.rs:1103`, `view/operations/attempts_tests.rs:47`, `retire_real_tests.rs:162`, `retire_tests.rs:94`, `retiring_tests.rs:70`, `view/polling_privacy_tests.rs:330` (a `TargetParts`) |
| E-10 | `view/footer_tests/mod.rs` :191, :445, :464 (`plan(`); `view/keyboard_tests/shell.rs` :353, :356, :415 (`plan(`), :422 (`Control::ALL.len()`); `shell/support.rs` :92 (`said`) |
| E-11 | `view/dock_tests/toolbars.rs` :158 (`with_no_repository`), :160 (`ALL`), :170 |
| E-12 | `assets_tests.rs` :8 (`ICONS: [&str; 47]`), :60 |
| E-13, E-14 | no sweep hit (`keys_tests.rs` is outside these greps) |
| E-15 | `view/polling_privacy_tests.rs` :127 (`ALLOWED`; `region-repositories` :136) |
| E-16 | `remove_repository` callers: `view/operations/republish_tests.rs` :597, :722; `view/item/edit_quit_tests.rs` :106, :405; `view/item/repair_tests.rs` :1048 |

Production sites, by task: Task 1 — `model/polling.rs`, `polling/attempt.rs`
(`not_shared` :167, :189, :201), `polling/state.rs`, `polling/table.rs`,
`polling/status.rs` (`not_shared` :166-167, `after_last` :174,
`since_library` :197, `phase_status` :215), `services/poll_convert.rs`
(`TargetParts` :40, :66; `AttemptTarget` :249, :459-460), `scheduler.rs`,
`scheduler/entity.rs` (the exhaustive `match heard`, :487-510),
`scheduler/reconfigure.rs`, `scheduler/registration.rs`,
`view/settings/polling.rs` (`Facts {` :391, `check_now` :225),
`view/polling_window.rs:113`. Task 3 — `sidebar.rs` panel code (26 sites),
`workspace.rs`, `workspace/render.rs`, `workspace/repositories.rs`
(`take_vanished` :33), `model/navigation.rs` (:249, :339). Task 5 —
`workspace/requests.rs` (:357, :366), `workspace/focus.rs:100`. Task 6 —
`view/dock/header/toolbar.rs` (:91, :102, :143, :162), `workspace.rs`
(`toolbar_focus`). Task 7 — `sidebar.rs` (22 sites), `workspace.rs`,
`workspace/content.rs` (:310, :330, :623-632), `workspace/render.rs`
(:122-125). Task 8 — `view/footer.rs` (`plan(` :132, `Control::ALL` :185),
`workspace/footer.rs:47`.

Unrelated hits, no edit: `retire_loop::Facts` (`model/polling/retire_loop.rs`
:70, :95; its tests; `services/scheduler/retire_loop.rs`),
`PillState::EditingSaved { not_shared }` (`view/item/pill.rs`,
`edit_tests.rs`, `header_tests.rs`), three test-local `Heard` enums, the
scheduler's own `plan`, the local `TAB_LIMIT` consts, `RESOLVING` timeouts,
and the `"Repositories"` layout literals.

**Unnamed sites: none confirmed.** Four watch items, each for the task that
first touches the code, none forced to break:

1. `view/dock_tests/toolbars.rs:270`: the `ToolbarButton::ALL` loop in
   `behind_a_dialog` (:263), reached from the E-11 test. `ALL` grows to 7 in
   Task 6; `press_toolbar` returns early under a modal. **Task 6 confirms**;
   a break there is E-17.
2. `view/footer_tests/mod.rs:405`: `Control::ALL` in
   `a_dock_menu_closes_when_a_modal_opens_over_it` (:392). **Task 8
   confirms.**
3. `model/navigation_tests.rs:296-365`: five `take_vanished` tests. Task 3
   changes `apply_repositories` for a re-listed kept entry (R-37); none of
   them re-lists a kept entry. **Task 3 runs them.**
4. `view/editing_privacy_tests.rs:56` and `view/lifecycle/privacy_tests.rs:74`:
   their `ALLOWED` lists have no `region-repositories`. Only E-15
   (`polling_privacy_tests`) is named. Task 4 puts item titles into rows and
   tooltips. These use no polling double, so probably moot. **Tasks 4 and 9
   keep it in mind.**

**Helper users** (all unaffected unless noted). `select_a_repository`
(`view/keyboard_tests.rs:250`, unchanged, E-3) has 30 callers in
`keyboard_tests.rs:631`; `actions.rs` 52, 82, 139, 178, 228; `conflict.rs`
208; `discussion.rs` 177, 300; `drafts.rs` 296; `editing.rs` 136, 237, 372,
494, 703, 795, 947, 1077, 1115, 1177; `frame.rs` 190, 305, 385, 462;
`lifecycle.rs` 148; `settings.rs` 45, 135, 170; `shell.rs` 86;
`shell/journey.rs` 134. It asserts only `("repo-selected", 0)` (:262), so
Part F leaves it alone. `reach` (`shell/support.rs:84`): `shell.rs` 246, 259,
289, 299, 398; `shell/journey.rs` 142, 173, 206 (the `reach` in
`item/share_tests.rs:390` and `operations/check_tests.rs:57` are separate
locals). `said` (`support.rs:92`): `shell.rs` 65 and 434 only.

The `("repo-row", i)` clicks in `dock_tests/state.rs:38`,
`documents_tests.rs:44,291,293` and `tickets_tests.rs:40,274,551,634,781`
stay valid because Task 3 keeps the id.

**Rulings.** None.

**Test edits.** None yet (E-1 to E-16 open).

**U-checks.** U-1 to U-7 open at Task 0; U-6 and U-7 answered in Task 1
(below). Open: U-4 in Task 3, U-1 and U-2 in Task 5, U-5 in Task 8, U-3 in
Task 9.

**Tests run.** None.

**Boundary check:** `git diff --stat origin/main...HEAD -- src ':!src/desktop' tests Cargo.toml Cargo.lock packaging` prints nothing.

## Process rulings

- Ruling: each task has two commits, the implementer's code commit and the
  orchestrator's ledger commit — only the orchestrator may write the ledger
  (brief), so it cannot be in the implementer's commit — a reviewer's range is
  the code commit alone; cost if wrong: one extra commit per task.
- The superpowers skill (`subagent-driven-development`) was not installed at
  the start of the session and was installed by the owner during Task 1. Task 0
  and Task 1's implementation were dispatched before it was loaded, to the same
  shape (fresh implementer, then a fresh reviewer). Its scratch workspace
  `.superpowers/` is inside the worktree (git-ignored); it is removed at close
  with `target/` and `.devenv/`. Where the skill says only four things stop the
  run, the owner's brief governs: its stop list is the stricter one.

## Task 1: sharing facts the scheduler keeps (Part A)

**Commits.** Code `a2abc5fa` (base `2aafa5d0`, 29 files, all under
`src/desktop/`). Boundary command prints nothing. Pushed with the ledger
commit.

**Design checkpoint: passed.** `desktop::model::polling::tests::schedule::
unshared_follows_attempts_shares_and_saves` is one table with a row for each
of U1–U15, written before any Task 1 code (red: 12 compile errors for the
missing interfaces). The reviewer compared it row by row with design Part A
(`design.md:618-634`): no row weakened, merged or edited to fit; items,
problems and `as_of` asserted after every step. Extras: a save of an item
already listed counts once; a save, share and `main_ahead` on a root with no
schedule log nothing; a new save is `Unpublished`; the first check sets the
list whole; U14 also pins the Settings line "Not checked yet · 1 not shared
yet". Every row passed on the first run after implementation.

**U-6: settled.** `scheduler::attention_tests::unlock_after_cancelled_prompt`
(`services/scheduler_attention_tests.rs:227`) sends a user-lane `PollOnce`
and the worker's provider prompts; a background poll never prompts
(`services/worker.rs:1160-1162`). Caveat, raised by the reviewer: the prompt
in that test comes from the double's `ask_in_check_now`, not a real
transport; the design cites the same evidence.
- Ruling: U-6 is settled by the test the plan names — the provider is
  lane-aware in production code and the plan says this test settles it —
  decision 9 stands; cost if wrong: decision 9 (a user-lane Check now of a
  locked key asks for the passphrase) goes back to the owner, and Task 2's
  Locked rows and Task 5's Check now menu item change.

**U-7: does not hold.** A `LocalContextResolve` envelope names no item
(`LocalSyncBinding::Resolve` builds `Scope { repository, ..Scope::default() }`,
`src/repository/mutation/bind/local_sync.rs:258-264`, a library file). Its
data DTO does carry `item_id` and `kind`; that is a possible later library
change, which needs a ticket and the owner. Moved to **K-1**: a local-context
resolve's save, like a conflict resolution's share, shows in "N to share" at
the next check, not at once. The design's K-1 text (`design.md:1259-1264`)
names only the conflict resolution's share; the approved design is not edited
(a ledger amendment instead), and the review-ready summary says so.
`Heard::Saved` reads `envelope.scope.item_id` for every intent; the test pins
that the real shape gives `None` and that an envelope naming an item gives
`Saved { kind: None }`.

**Tests run** (`devenv shell -- cargo test --locked --features
desktop-test-support --bin manyhands <path> -- --exact`, final run 7 ran,
7 passed):
`desktop::model::polling::tests::schedule::unshared_follows_attempts_shares_and_saves`,
`desktop::model::polling::tests::schedule::to_share_counts_and_finished_shares`,
`desktop::model::polling::tests::schedule::attempt_summary_counts` (E-9),
`desktop::model::polling::status::tests::status_text_table` (E-9),
`desktop::services::scheduler::reconfigure::tests::finished_shares_and_saves_are_heard`,
`desktop::services::scheduler::entity_tests::the_stored_outcome_and_shared_copy_are_kept`,
`desktop::services::poll_convert::tests::check_now_attempt_reads_the_report` (E-17).
`desktop::view::settings::polling_tests::settings_polling_rows` is unchanged
and was not run: it pins no text that changed (implementer's claim; the
reviewer could not confirm it from the diff, and it runs at Task 10). Verify
lines, each exit 0: `cargo fmt --check`, `cargo check --locked --features
desktop`, `cargo clippy --all-targets --all-features --locked -- -D warnings`.
The Step 3 tests other than the table were written before the code but were
compile-red only, not run red separately.

**Test edits.**
- E-9, as named: `attempt_summary_counts` asserts `to_share.len()` 3 and
  `problems.len()` 10 per case; `status_text_table` gains `stored: None`, the
  stored-outcome rows and the "· N not shared yet" rows; `main: false` on 19
  `TargetParts` and `AttemptTarget` literals (fact 66), by a script keyed on
  compiler error positions. Test functions of the touched files listed before
  and after: three additions
  (`to_share_counts_and_finished_shares`,
  `unshared_follows_attempts_shares_and_saves`,
  `the_stored_outcome_and_shared_copy_are_kept`), none lost. The only removed
  assertion lines are `attempt_summary_counts`' `not_shared` tuple, which E-9
  replaces.
- **E-17**: `desktop::services::poll_convert::tests::check_now_attempt_reads_the_report`
  gains a block asserting that `main` comes from a DTO target that names a ref
  and has no kind — the Check now path's `main` (Task 1's new field) had no
  test. It removes no assertion.

**Rulings (implementer's, reviewed).**
1. "· N not shared yet" is also appended on the checking-off and held lines,
   not only Polling rows — the design says "whatever its status" (decision 24,
   P44) — reviewer: consistent; cost if wrong: two rows and two `counted()`
   calls.
2. `saved` and `main_ahead` return true when the list changed, like `shared`
   — symmetry; cost: none visible (only the no-schedule `false` is asserted).
3. A replayed share also removes the item from `last.listed` — design R-3,
   so Activity's entry agrees with the count; reviewer: consistent; cost: three
   lines.
4. An attempt with no `completed_at` counts as older than every change and
   older than a list that has an `as_of` — rule 3 states the first part; the
   second is conservative; reviewer: not a defect, untested; cost: one
   condition.
5. `unshared_after(&mut self, root, &AttemptSummary)` uses a new
   `AttemptSummary::carries_targets()`.
6. `Heard::Saved` takes the item from `envelope.scope.item_id` for every
   intent; `Heard::Shared` from the intent's own ID.
7. The change log is not pruned; `remove(root)` drops it.

**Dead-code allows (differs from the plan).** `#[allow(dead_code)] // Used
from Task 3` on `Scheduler::shared_copy`; `// Used from Task 5` on
`Scheduler::reread`; `#[allow(unused_imports)] // Used from Task 3` on the
`SharedCopy` re-export in `scheduler.rs` (the lint that fires; the plan names
`dead_code`). Not added: the plan's allows on `Unshared`, `problems` and the
`SharedCopy` fields — the compiler does not warn on them (derives count as
reads), so Task 3 has nothing to remove there.

**Review** (fresh reviewer, opus; diff `2aafa5d0..a2abc5fa`): code and tests
faithful to Part A and the brief, no Critical, no code defect. Its only
Important finding was these ledger records (done here), plus the push (done
with this commit). Redaction: `SharedCopy.location` comes from the already
redacted `list_remotes_redacted`; no new string carries it.

**Deferred findings (Minor), carried to the final review:**
1. The no-`completed_at` rows (rule 3, ruling 4) are not in the table.
2. When rule 4 rejects an older completed attempt, `apply` has already made it
   `last`, but shares heard after its `completed_at` are not taken out of its
   `listed`, so Activity can list an item the count no longer has (U13 with a
   share between the two checks). Fix: replay `Shared` changes onto
   `last.listed` before the rule-4 early return. Revisit in Task 3, which
   first draws from `last`, or at the final review.
3. The change log is never pruned; after a list replacement, entries timed at
   or before `as_of` can never replay. Fix: `retain(|(_, when)| *when > as_of)`.
4. No test covers `stored`'s refresh in `answered` (`react.rs`).
5. No entity-level test covers the `heard` arms for Shared, Saved and
   MainAhead.
6. File growth: `model/polling.rs` 576 → 692 (the unshared impl block could be
   its own `polling/unshared.rs`), `scheduler/reconfigure.rs` 527 → 970 (mostly
   the test the plan asks for in its `mod tests`), `polling_schedule_tests.rs`
   270 → 983.
7. R-43 ("Up to date" from `unshared.as_of`) is the panel state's (Task 2),
   not owed here: `checked` still dates the Settings line from `last`.

**Doubts carried.** `ReportParts::of` (the automatic path's `main`) has no
unit test; a real `RemotePollReport` can be built only inside the library.
`PollingBackend::store_outcome` replaces `latest_outcome` for every
`polling_status` of that root, so it also overrides what a real `Completed`
poll stored.

## Task 2: repository state (Part B)

**Commits.** Code `4979f441` (base `d21b14f5`); fix round 1 `b103c639`. New
files: `model/repository_state.rs` (497 lines), `repository_state/words.rs`,
`repository_state/menu.rs`, `model/repository_state_tests.rs` and
`repository_state_tests/{menus,pairs,scene,states,tips}.rs`; `model/mod.rs`
gains the module; the `#[allow(unused_imports)]` on the `SharedCopy` re-export
in `services/scheduler.rs` is removed (now used). No existing test edited. The
boundary command prints nothing.

**Tests run** (`devenv shell -- cargo test --locked --features
desktop-test-support --bin manyhands <path> -- --exact`; each ran 1 test and
passed, in the first round and again after the fix):
`desktop::model::repository_state::tests::each_state_and_its_precedence`,
`desktop::model::repository_state::tests::tooltips_say_the_state_and_the_facts`,
`desktop::model::repository_state::tests::menu_items_follow_the_state`.
Red first: a compile failure for the missing items; in the fix round the first
two failed on the new cases before the code changed (`menu_items_follow_the_state`
already passed: the new rows filled a coverage gap). Verify lines, each exit 0
after each round: `cargo fmt --check`, `cargo check --locked --features
desktop`, `cargo clippy --all-targets --all-features --locked -- -D warnings`.

**Owner rulings, asked and answered 2026-11-12 during Task 2** (the review
found a contract question the Cycle, design and plan do not settle):
- Ruling (owner): Part B rows 6 and 7's last-answer triggers
  (`Transport(Unavailable)`, `Transport(ConfigurationRequired)`,
  `Transport(Unreachable)`) hold only while `policy.enabled && !policy.paused`
  — the design's table put them ahead of row 9 while its sentence rules and
  fact 53 say Paused outranks everything but an attention, and `last` survives
  pausing, so a paused repository showed label "Can't reach" with the sentence
  "Paused" — a paused repository now reads Paused (or "{n} to share"), an off
  one row 13; cost if wrong: one `last_answer` helper and five pairs
  (`repository_state.rs:309-319`, `pairs.rs:313-386`). **Amends Part B's
  table, rows 6 and 7 (the approved design is not edited; this entry is the
  amendment, and the review-ready summary says so).** Attention phases, Held,
  a pending conflict, `Waiting { why: Failed }`, `Unreadable` and stored
  outcomes are unchanged.
- Ruling (owner): rows 8 and 10 are not gated on `enabled` — an off repository
  with an unshared list still reads "{n} to share" or "Up to date" — the list
  is a fact about the repository and Settings appends "· N not shared yet"
  whatever the status (decision 24); cost if wrong: gate two rows; the footer's
  N to share total would then need the same gate. Row 13's "checking off" means
  off with no completed check.
- Ruling (owner): the wording for an unshared item with no parsed ID is "an
  item" (`words.rs` `UNNAMED_ITEM`), one constant with a tooltip case — the
  design is silent; cost: one constant.

**Rulings (implementer's, reviewed).**
1. Tooltip line 1 is the sentence parts joined with a space — reviewer found
   a run-on; **fixed in round 1**: a part followed by another and not ending in
   `.`, `!`, `?`, `:` or `…` gets a full stop (`words.rs:116-128`); the
   expectation pinned to the run-on is corrected (it was shaped to the bug).
2. A policy-only hold gets `Attend(ResumeChecking, RecoveryHeld(root))`
   (row 6 names `Shape::Held` with `action_of`'s Resume checking).
3. Some menu labels (Check now, Retry now, Set up sharing…, Open folder) are
   duplicated into the model. The implementer's stated reason ("the model must
   not import view modules") is inaccurate — `menu.rs:14` imports
   `view::sidebar`, as the design asks (design:851) — corrected in its report;
   see deferred 1.
4. The kept row beats a busy resolution (row 1 over row 2).

**Dead-code allows.** One module-wide `#[allow(dead_code)] // Used from Task 3`
on `repository_state` in `model/mod.rs:29`, and `unused_imports` on the
re-exports in `repository_state.rs`, all marked "Used from Task 3".

**Review** (fresh reviewer, opus, `d21b14f5..4979f441`): spec ❌ for the paused
contradiction (above), the run-on sentence, two missing brief-listed menu cases
and the invented "an item" copy; no Critical. Fix round 1 (the original
implementer, resumed) addressed findings 1, 2, 3, 4 and 6; the scoped
re-review (sonnet, `4979f441..b103c639`) verdicted all ADDRESSED with no new
issue. Table fidelity (rows 1–13, all required pairs) and signatures were
verified by the first review.

**Deferred findings (Minor), carried to the final review and to Task 3:**
1. Label duplication: the shared labels (`SHARE_MAIN`, `SHARE_MAIN_SET_UP`,
   `SHARING`, `CHECK_NOW`, `RETRY_NOW`, `SET_UP_SHARING`, `OPEN_FOLDER`) exist
   in several places (model, `sidebar.rs`, `lifecycle.rs`, `settings/polling.rs`)
   and the tests compare each against its own copy, so drift would go
   unnoticed. Better: one copy in the model, the views point at it.
2. **Task 3 must remove** the module-wide `#[allow(dead_code)]` at
   `model/mod.rs:29` and check that nothing stays dead.
3. A `Missing` configuration with no problems reads "Its configuration needs
   fixing:" ending on a colon — the design's sentence; left.
4. `repository_state.rs` is at 497 lines (the cap): Task 3 must not grow it.
5. The punctuation rule recognises only the single character `…`, not "...".
