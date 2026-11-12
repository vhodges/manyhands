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
| 3 | done, review approved after fix round 1 | code `6cb077c2` (base `3dbaa472`), fix `eb2bc10e`; ledger commit follows | yes |
| 4 | done, review approved first time | code `02bd0015` (base `64d92f09`); ledger commit follows | yes |
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

## Task 3: rows and list states (Part C)

**Commits.** Code `6cb077c2` (base `3dbaa472`; the commit's trailer says
"Claude Opus 5.5", the implementer's own model and the plan's wording); fix
round 1 `eb2bc10e`. Boundary command prints nothing. New: `view/repositories.rs`
(236), `repositories/row.rs` (272), `repositories/body.rs` (143),
`view/repositories_tests.rs` (447), `repositories_tests/support.rs` (132).
`sidebar.rs` 576 → 294. `workspace.rs` 495 (not grown); `repository_state.rs`
496 (not grown). No new file over 500 lines.

**U-4: holds.** A discarded scratch test (host view whose render compares its
model with a child view's rows and applies them with `window.defer`) shows the
child holds the new rows after one `run_until_parked`: one deferred apply, no
render loop (`gpui-pre-0.3.6/src/app.rs:1781-1830`); the committed journey's
`pane_follows` confirms it. The plan's fallback was not needed. The scratch
run also checked E-12's icons: all but `lock` already load.

**Tests run** (`devenv shell -- cargo test --locked --features
desktop-test-support --bin manyhands <path> -- --exact`):
- Red: the journey `desktop::view::repositories::tests::rows_and_list_states`
  against a stub module: 6 compile errors. Mutation check: R-37 switched off →
  the journey fails at "resolved again"; restored.
- Step 5 set, one command, 49 exact names: 49 ran, 48 passed, 1 failed:
  `desktop::view::keyboard_tests::editing::edit_the_mode_key_save_and_done_by_keyboard_alone`
  timed out (`harness.rs:501`) under 49 parallel window tests; alone it passed
  (1 ran, 8.6 s). Known flaky: Cycle 02 ledger
  (`docs/plans/2026-11-11-wave-04-cycle-02-…-execution.md:525-530`, `:624`) and
  Cycle 01 (`…cycle-01-…-execution.md:1718-1725`).
  The 49: the journey; E-1 `workspace::tests::a_renamed_away_repository_is_unavailable_until_retry_finds_it_again`;
  E-2 `workspace::tests::a_busy_list_reload_keeps_the_last_rows_with_a_label` and
  `settings::repositories::manage_tests::tabs_close_once_the_list_no_longer_gives_their_root`;
  E-3 `workspace::tests::the_fixtures_repositories_are_listed_by_name`; E-12
  `assets::tests::every_item_view_icon_loads`; unchanged
  `workspace::tests::a_repository_rows_selected_state_has_one_meaning` and
  `the_repository_selector_selects_with_arrows_and_enter`; six
  `desktop::model::navigation::tests::` tests around R-37 (the Task 0 watch
  item: the range held four `take_vanished` functions, not five — the four plus
  two `apply_repositories` tests); and the `select_a_repository` users, mapped
  to 30 `keyboard_tests` test names (`every_core_action_works_by_keyboard_alone`;
  `actions::` 6; `conflict::` 1; `discussion::` 2; `drafts::` 1; `editing::` 10;
  `frame::` 4; `lifecycle::` 4; `settings::` 4; `shell::ctrl_b_in_an_editor_is_bold`;
  `shell::journey::arrange_the_workspace_by_keyboard_alone`).
- Final tree, fix round: `rows_and_list_states` and
  `desktop::model::navigation::tests::a_kept_selection_listed_again_is_resolved_again`,
  2 ran, 2 passed. Break-on-purpose checks, each restored: share update off →
  the journey fails at its "while held" assertion; R-37 off → the new unit test
  fails at its `Resolution::Unknown` assertion.
- Verify lines, each exit 0 on the final tree: `cargo fmt --check`,
  `cargo check --locked --features desktop`, `cargo clippy --all-targets
  --all-features --locked -- -D warnings`.

**Named test edits.**
- E-1, row part: in `a_renamed_away_repository_is_unavailable_until_retry_finds_it_again`
  the `("repo-unavailable", 0)` presence/absence assertions became equality /
  inequality of `("repo-state", 0)`'s label with `CANT_OPEN`.
- E-2, row part: in `a_busy_list_reload_keeps_the_last_rows_with_a_label`,
  `repositories-state` = `LABEL_BUSY` became `repositories-alert` =
  `NOT_UPDATED_BUSY` and `repositories-retry` became the alert's
  `("repositories-alert", "action")`; in `manage_tests::tabs_close_once_the_list_no_longer_gives_their_root`
  `repositories-last-read` became the alert's label. **Assertion removed (E-2):
  the `repositories-last-read` = "Showing the last list read" assertion and its
  negative form.**
- E-3, row part: in `the_fixtures_repositories_are_listed_by_name` **the
  `("repo-root", 0)` root-path assertion and its comment are removed (E-3).**
- E-12, part: `lock` added to `ItemIcons` and `ICONS` (47 → 48); the other
  icons already load. (The report first said `hard-drive` is not in the Kit's
  `default-icons.txt`; it is, line 48 — the conclusion stands.)
- **E-18** (needed edit): `model/repository_state_tests/scene.rs` loses the
  unused `pub(super) const NAME`. Task 2's module-wide `dead_code` allow had
  hidden it; removing the allow (required here) made it a test-build warning,
  so an error under `-D warnings`. No assertion removed.
- E-14: no appearance test was deleted. `editing_privacy_tests` and
  `lifecycle/privacy_tests` untouched.
- Test functions before/after (by grep, edits by exact replacement):
  `workspace_tests.rs` 28/28, `repositories_manage_tests.rs` 20/20,
  `assets_tests.rs` 2/2, identical. Fix round adds
  `a_kept_selection_listed_again_is_resolved_again` (new; `navigation_tests.rs`
  425 lines).

**Rulings (implementer's, reviewed).**
1. The plan's "a finished `ItemSync` → '1 to share' with no list read" (and the
   `TicketSave`) cannot hold literally: `Workspace::operations_changed`
   (`workspace/requests.rs:169-170`) already reloads the repository list after
   every changed request batch, so Repositories reads grow by one per request.
   The journey instead holds that one read and asserts the row already shows
   the count while it is held, that no poll ran (`polled_roots` unchanged), and
   that the same count shows after the read lands — design P5/P33 ask only that
   the count moves "at once, once" — cost if wrong: if the owner meant no list
   read at all after a share or save, `operations_changed` must stop reloading
   for those batches, a behaviour change of its own. The first review judged
   the first version weak (a possible second unheld read); the measured count
   is exactly one read per request, and the fix added the held-read count, the
   while-held and the post-release assertions.
2. "Ready, no rows" means no rows drawn: a ready list that gives none while the
   chosen repository is kept shows the kept row and no empty state (R-20) —
   cost: one condition in `body::state`.
3. A reload that stays busy or fails says so before "incomplete", both keeping
   the rows (Cycle 02's `list_body` order) — cost: two arms.
4. Alert tones: `Warning` for busy and incomplete, `Failure` for a failure; the
   failure's text is `failure.code.message()` — Part C says "with the failure's
   message".
5. The bounds map lives in `RepositoriesPane`, not a new `Workspace` field
   (`workspace.rs` is at 495 lines) — Task 5 reaches it through the pane.
6. Copy constants in `view/repositories.rs`; `body.rs` uses them; the panel's
   "Settings" label is a private const in `body.rs`.
7. The skeleton draws 3 rows (the design gives no count; Tickets/Documents use 6).
8. `RepositoriesPane::set` became `show_after_frame` (defer) plus a private
   `show`; `rows(&self, cx)` returns a clone.
9. Journey seams: a corrupt `manyhands.sqlite3` through `replace_service` for
   the first-load failure; a `CacheWrite` lease plus `advance_clock(1 s)` for
   busy; a real empty registry for the empty state; `enable_through` to
   register again; a constructed `RepositoryListDto { complete: false, … }`
   through a test-only `Workspace::apply_repository_list`; the re-registered
   repository's poll canned as `Library::Busy` so P44 runs with no completed
   check. No SQL, no chmod.

**Dead-code allows.** Removed: the module-wide one on `repository_state`
(`model/mod.rs`), both `unused_imports` re-export allows, `Scheduler::shared_copy`'s.
Added, each on its item: `// Used from Task 5` on `RepositoriesPane::bounds`,
`RepositoryDelegate::workspace`, and the re-export of `MenuChoice`, `OPEN_FOLDER`,
`REMOVE_FROM_MANYHANDS`; `// Used from Task 6` on `MenuChoice::Refresh`,
`Rebuild`, `more_items` and its re-export; `// Used from Task 9` on the two
`#[cfg(test)]` accessors `tooltip()` and `menu()`. `Scheduler::reread` keeps
`// Used from Task 5`. This resolves Task 2's deferred item 2.

**Review** (fresh reviewer, opus, `3dbaa472..6cb077c2`): Part C, R-7, R-8, R-20,
R-37 and the named edits verified; one Important finding (the journey's
"with no list read" steps held only the next list read), addressed in fix
round 1 (`6cb077c2..eb2bc10e`); the scoped re-review (sonnet) approved with no
new issue.

**Deferred findings (Minor).**
1. **To Task 5:** the bounds map is never pruned, so a row that leaves keeps
   its entry and a scrolled-out row keeps stale bounds, and R-42's fallback
   (anchor on the list's bounds when the row has none) cannot fire after a
   first draw; clear stale keys in `show` or per prepaint, and add an accessor
   (the field is private).
2. **To Task 5:** `workspace/focus.rs:220-228`: `secondary-1` focuses the Kit
   list only when the navigation list gives rows; with only the kept row drawn
   the one visible row is unreachable by keyboard, and E-6 drives the kept
   row's menu.
3. `body.rs`'s `alert()` duplicates `list_view.rs:183`'s private helper.
4. `show_after_frame` clones every pane row each frame to compare; compare in
   place instead.
5. `view/shell/tokens.rs` `ChromeColors::green`'s `cfg_attr(not(test),
   allow(dead_code))` is stale (Up to date uses it); remove in a task that
   touches the file.
6. Task 2's deferred items 1, 3, 4, 5 stand.

## Task 4: titles of what is not shared (Part C)

**Commit.** Code `02bd0015` (base `64d92f09`). Boundary command prints nothing.
New: `view/workspace/titles.rs` (217), `repositories_tests/titles.rs` (240).
Modified: `model/stamp.rs` (+4, `ViewScope::ShareTitles(RepoKey)`),
`services/mod.rs` (+4, `ReadKind::ShareTitles`), `view/repositories.rs` (237),
`repositories_tests.rs` (460), `repositories_tests/support.rs` (165),
`workspace.rs` **502** (was 495), `workspace/lists.rs` **500**,
`workspace/repositories.rs` (+1). No exhaustive match over `ReadKind` or
`ViewScope` existed, so none needed a new arm. No existing test edited
(E-19: none); no assertion removed; no dead-code allow added or removed.

**Tests run** (`devenv shell -- cargo test --locked --features
desktop-test-support --bin manyhands <path> -- --exact`):
`desktop::view::repositories::tests::titles_of_what_is_not_shared` — red first
(15 × E0599, all `ReadKind::ShareTitles`), then 1 passed; checked it can fail by
replacing the "no job while one runs" filter with `|_| true` (fails at "no second
job while held", 3 vs 1), restored. `desktop::view::repositories::tests::rows_and_list_states`
re-run because `repositories::rows` changed: 1 passed. Verify lines, each exit 0:
`cargo fmt --check`, `cargo check --locked --features desktop`, `cargo clippy
--all-targets --all-features --locked -- -D warnings`.

**Orchestrator's extra run, for a named doubt** (the reviewer's ⚠️: other
polling window tests whose polls report kinded unshared items now also start
real `ShareTitles` reads on every scheduler notify). Command: `cargo test
--locked --features desktop-test-support --bin manyhands --
view::polling_privacy_tests:: keyboard_tests::shell::polling_journey
view::operations::retire_tests:: view::operations::retire_real_tests::` — 12
ran, 12 passed (22.8 s): `polling_privacy`, `polling_keyboard`, seven
`retire_tests`, three `retire_real_tests`. This is not a suite run; those tests
run again in the fast tier at Task 10.

**What the journey covers** (through `rows(workspace, cx)` tooltips, so through
`repository_tooltip` on the Workspace's facts, every launch poll answered, no
sleeps): titles from loaded Tickets and Documents lists with 0 `ShareTitles`
reads; one read held with `hold_read`, 4 notifies while held starting no second
job and using no try, then the read answered and the item named by title, main
and a no-kind item not read; a CacheWrite lease making the read busy: three
jobs, one per notify (timers advanced), then the item named by ID and, with the
lease dropped, not read again; seven unshared items, five named
("S1, … S5 and 2 more."), reads = 1 resolve + 5 `show_item`, none after
another notify.

**Rulings (implementer's, reviewed).**
1. The title maps are `BTreeMap<(RepoKey, ItemId), _>`, not the brief's
   `HashMap`, and the three maps plus a `Generations` live in one `ShareTitles`
   struct in a single Workspace field `share_titles` — `ItemId` has `Ord` but no
   `Hash` and a library change is out of bounds; the struct keeps `workspace.rs`
   near 500 — cost: a rename or a re-key. (The brief names `HashMap` and three
   fields; recorded here so Task 9 and the final review do not flag the drift.)
2. Only listed repositories are read and pruned (`retain_titles` from
   `repositories_loaded`); the kept row (chosen, unlisted) gets no read and its
   titles are dropped, so it names items by ID — its registration is gone, a
   read could only fail — cost: a kept row's tooltip loses titles.
3. The "five named" are the first five of `unshared.items` (main counts among
   them, as in `to_share_sentence`); those with a kind, an ID and not main are
   read — matches what the tooltip names.
4. Lists teach titles whatever their stamp (the root from the `Tickets(root)` /
   `Documents(root)` scope, only on `ReadResult::Ok`), and a list's title
   overwrites a read one and the reverse — R-9 accepts a renamed item keeping an
   old title until the next list load.
5. `ItemNotFound` is "gone" (Done); a resolve that fails for any reason counts
   one try for every ID of the job; a found item with no title is Done and
   named by ID (K-3); a single ID's `show_item` busy or failure retries that ID
   only.
6. The `RepositoryRow::tooltip()` accessor keeps `#[allow(dead_code)] // Used
   from Task 9` (the test reads the private field).

**Review** (fresh reviewer, opus, `64d92f09..02bd0015`): spec compliant; every
R-9 rule verified in the code (tries counted only on an applied answer, `Done`
final so no re-read loop, a running job never replaced, answers merged by (root,
id), pruned on removal); no Critical, no Important.

**Deferred findings (Minor).**
1. The per-ID `Retry` path (`show_item` busy or a non-`ItemNotFound` failure) is
   never run: the busy case comes from a CacheWrite lease making
   `resolve_repository` itself busy. `ItemNotFound` → Done is also untested.
   Add one mixed job (one found, one retry) if a seam appears.
2. Repository removal (pruning, including cancelling a held read) is not tested.
3. Main is not exercised in the journey: `main_current()` is `Current`, not an
   unshared item; the code filters main twice.
4. A stale list teaches titles without a notify; a later render shows them.
5. **`workspace.rs` is 502 lines and `workspace/lists.rs` 500: the next task
   that touches either must split it first** (Task 5 is to put its menu code in
   new files and offset any `workspace.rs` addition).
6. The struct `ShareTitles` shares its name with `ReadKind::ShareTitles` and
   `ViewScope::ShareTitles` (a name like `ShareTitleCache` reads better);
   `share_title` clones a `String` and an `ItemId` per lookup (≤ 5 per row per
   frame).
7. `a_kept` row dropping titles: see ruling 2 (K-3 covers an item named by ID).
