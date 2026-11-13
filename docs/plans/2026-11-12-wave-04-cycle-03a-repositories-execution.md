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
| 5 | done, review approved after fix round 1; checkpoint comment follows | code `caa23a81` (base `94f5abbd`), fix `ba2a24bc`; ledger commit follows | yes |
| 6 | done, review approved after fix round 1 | code `b8044b60` (base `9cc1a4a0`), fix `c0ab728a`; ledger commit follows | yes |
| 7 | done, review approved first time | code `6899a165` (base `19800b2d`); ledger commit follows | yes |
| 8 | done, review approved first time | code `d8013dcc` (base `b2997259`); ledger commit follows | yes |
| 9 | done, review approved after fix round 1 | code `3d6d81b3` (base `6fe57a58`), fix `7d1ef534`; ledger commit follows | yes |
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

## Task 5: the repository menu (Part D)

**Commits.** Code `caa23a81` (base `94f5abbd`); fix round 1 `ba2a24bc` (trailers
say "Claude Opus 5.5", the implementer's model, as the plan's Commits rule
reads). Boundary command prints nothing. New: `view/repositories/menu.rs` (177),
`view/workspace/repository_menu.rs` (247), `repositories_tests/menu.rs` (485),
`test_support/polling/unanswered.rs` (48). `workspace.rs` 502 → **496**
(accessors moved into `repository_menu.rs`), `lists.rs` 500 (untouched),
`repository_state.rs` 496. Files already over 500 that grew: `content.rs`
744 → 772, `requests.rs` 632 → 636, `test_support/polling.rs` 528 → 536 (the new
mode is in its own file).

**U-1 and U-2: both hold** (scratch test, discarded; `secondary-1` then
`shift-f10` and `menu`): the context stack with the Kit list focused is
`[Root, Workspace, Repositories, List]`; `shift-f10` and `menu` both reached
the `Repositories`-context binding and the Kit `List` binds neither, so no
fallback; `window.right_click(("repo-row", 1))` reached the row's
`capture_any_mouse_down` with the right button.

**Tests run** (`devenv shell -- cargo test --locked --features
desktop-test-support --bin manyhands <paths> -- --exact`):
- Red: the journey `desktop::view::repositories::tests::the_menu_by_pointer_and_keyboard`
  — 11 compile errors (missing `OpenRepositoryMenu`, `RepositoryMenuChoice`,
  `Shown`, `MenuAnchor`, the 3-vs-4-argument `remove_repository`, the opener
  and accessors).
- Journey, three attempts: a wait that expected a poll for a newly read
  repository (it is scheduled `Idle`, not polled at once), then `managed-root`
  unset (R5-2), then 1 passed (17.7 s).
- E-13 `desktop::keys::tests::each_key_is_bound_to_its_action`, Settings'
  `desktop::view::settings::polling_tests::settings_polling_controls` (unchanged),
  `rows_and_list_states`, `titles_of_what_is_not_shared`: 4 passed.
- E-16, five call sites reaching seven tests, by exact name:
  `desktop::view::operations::republish::tests::republish_waits_for_another_dialog_to_close`,
  `…::republish_opens_when_the_dialog_it_waits_for_is_cancelled`,
  `desktop::view::item::edit_tests::quit::removing_the_repository_says_an_unkept_tab_closes`,
  `…::removing_a_repository_with_no_unkept_tab_says_nothing_of_it`,
  `…::a_removal_found_already_applied_keeps_an_unkept_tab`,
  `…::a_tab_unkept_after_confirm_stays_when_the_removal_lands`, and the
  `repair_tests` one — all passed.
- A named doubt (the sidebar's Remove now passes the list's focus handle): 3
  passed — `keyboard_tests::actions::remove_an_unavailable_repository_by_keyboard`,
  `settings::repositories::manage_tests::removing_a_registration_whose_folder_was_renamed_away_succeeds`,
  `…::the_sidebar_remove_of_a_registration_already_gone_lets_the_entry_go`.
- Fix round: the journey 1 passed (and three reruns, 1 passed each); with two
  restores disabled on purpose, all four focus checks read false (restored,
  no residue); final `the_menu_by_pointer_and_keyboard`, `rows_and_list_states`,
  `titles_of_what_is_not_shared`: 3 passed.
- Verify lines, each exit 0 after each round: `cargo fmt --check`, `cargo
  check --locked --features desktop`, `cargo clippy --all-targets --all-features
  --locked -- -D warnings`.

**Named test edits.** E-13: `each_key_is_bound_to_its_action` gains `menu` and
`shift-f10` → `OpenRepositoryMenu` (26 rows); no assertion removed. E-16: the
five `remove_repository` test calls (`republish_tests.rs` :597, :722;
`edit_quit_tests.rs` :106 in `ask_to_remove`, :405; `repair_tests.rs` :1048)
pass the Repositories list's focus handle; no assertion changed. Settings'
Check now (`settings_polling_controls`) runs unchanged. No E-19: no other
existing test changed; no assertion removed. Test functions of the scripted
files before/after: identical (`republish_tests.rs` 22, `edit_quit_tests.rs` 18,
`repair_tests.rs` 53, `keys_tests.rs` 2); `repositories_tests.rs` gains
`the_menu_by_pointer_and_keyboard`.

**What the journey covers.** Four repositories (`a` with a ticket tab for the
sheet; `b`; `c` renamed away before the scheduler reads it; `d` with no
publication remote). A right click on a non-chosen row opens that row's menu
and the choice stays; labels equal `menu_items`' in order, the sentence first;
opening, moving and Esc read nothing and send nothing (P22); `shift-f10`, `menu`
open it for the highlighted row; Check now sends `PollOnce` for that root on the
user lane; with a dialog and with the details sheet open the action, a
dispatched `RepositoryMenuChoice` and the opener do nothing and an open menu
closes; Open folder records the root in `OpenedFolders`; Share main sends
`RepoSync`; a Locked row's Unlock… reaches `Scheduler::unlock`; a Can't reach row
whose "Retry now" is shown, made Needs attention by a live recovery hold, then
"Retry now" chosen, sends nothing and resumes nothing; a check starting after
the menu opened, then Check now, sends nothing (P26-P27 re-check, Review Focus
4); Retry on `c` (not chosen) reads it again; "Share main…" on `d` opens Settings
› Remotes; Settings… on `b` opens Settings › Repositories managing `b`;
Remove… opens the dialog with `FILES_REMAIN`, Cancel returns focus to the list
(read in the same update), OK removes the row and its stale bounds; a reload
dropping the menu's repository closes it (P29); a kept row alone: `secondary-1`
focuses the list and `shift-f10` opens its menu with Retry and Remove; activating
the always-disabled sentence, a dispatched Sentence choice and a Check now built
disabled send nothing.

**Rulings (implementer's, reviewed).**
- R5-1: `RepositoryMenu::open` takes `action: Option<RowAction>` after `items`
  (`MenuItem` does not carry the `RowAction`, `Shown::Action` needs it) — 8
  arguments, `#[allow(clippy::too_many_arguments)]`.
- **R5-2: for `SettingsPage::Repositories`, `open_settings_on` also calls the
  page's existing `manage_root(root, false)`** — the plan's `set_selection`
  alone is ignored by Settings › Repositories (only Remotes and Hosts read it),
  so P27 ("Settings opens on that repository") was not met; reviewer: the right
  seam, mirrors `SettingsHost::identity` and `open_remotes_on`; one local
  Identity read, no secret or network; side effect: it empties the identity
  form's inputs as a Manage click does — cost if wrong: Settings… replaces the
  page's managed repository and starts one identity read. No owner ruling needed.
- R5-3: `remove_repository`'s "caller's region" is the region whose handle
  contains `restore`, or the toolbar button's region, else Content (new
  `region_of` in `focus.rs`) — the design names no rule.
- R5-4: the sidebar's Remove (the details block, until Task 7) passes the
  Repositories list's focus handle, as the brief reads (plan-mandated; reviewer
  notes `this.remove_focus.clone()` would keep today's focus) — Cancel there
  returns focus to the list until Task 7 deletes the block; no assertion
  weakened.
- R5-5: a key-opened menu with focus elsewhere in `REPOSITORIES_CONTEXT` is for
  the chosen repository and its context is the focused handle (R-40 in one
  place; the toolbar branch cannot be exercised until Task 6).
- R5-6: `MenuAnchor::Bounds(b)` opens at `b.bottom_left()`, `Pointer(p)` at `p`,
  Kit `Anchor::TopLeft`, 8 px window margin.
- R5-7: when the menu's repository leaves the rows the layer drops the menu and
  focuses its context. R5-8: a choice that fails the re-check still marks the
  menu chosen, so focus stays in the menu's context. R5-9: `Refresh` and
  `Rebuild` choices do nothing until Task 6.
- R5-10: the stand-in answer for unqueued polls is "not due in an hour" (records
  no attempt) — a completed answer could replace `last` mid-step. R5-11: the
  stand-in count is asserted as bounded (1 to 4, all `b`), not exact — cost: an
  extra poll goes unnoticed rather than fails. R5-12: the pointer-Esc check uses
  the footer's Settings control as the earlier focus (the fallback never picks a
  footer control). R5-13: the bounds map is cleared before each list prepaint,
  replacing the per-key prune, so only drawn rows have bounds.

**Dead-code allows.** Removed: the four `// Used from Task 5` allows
(`RepositoriesPane::bounds`, `RepositoryDelegate::workspace`, `Scheduler::reread`,
the `MenuChoice`/`OPEN_FOLDER`/`REMOVE_FROM_MANYHANDS` re-export — `MenuChoice`
now plain, the two constants `#[cfg(test)]`). Added: one
`#[allow(clippy::too_many_arguments)]` (R5-1). Still staged: `more_items`, its
re-export and `MenuChoice::Refresh`/`Rebuild` (Task 6); `RepositoryRow::tooltip()`
and `menu()` (Task 9).

**Other changes.** `REPOSITORIES_CONTEXT` replaces the literal in
`Region::key_context`; `Services::check_now(&self, root) -> bool` (user lane,
refuses while a `PollOnce` is in flight) is called by Settings' Check now (after
its own dialog guard) and the menu; `attend` is widened; `focus_region(
Region::Repositories)` focuses the Kit list when the panel draws any row, the
kept row included.

**Review** (fresh reviewer, opus, `94f5abbd..caa23a81`): the functional contract,
the choice checked again (R-35, structural: `Shown` derives `PartialEq` over
`RowAction`), the guards (R-36, R-12), keys (E-13), `check_now`, E-16, allows and
file sizes verified; no Critical. Two Important findings, both test strength:
(1) the journey's 8 queued poll answers were a count, not a guarantee (a 9th
poll would reach the real library); (2) four Esc and post-choice focus
assertions were read after a draw, where `focus_lost`'s fallback could satisfy
them. Both addressed in fix round 1 (`caa23a81..ba2a24bc`), with a mutation
check that the four focus checks fail without the menu's restore; Task 3's two
carried items done (bounds cleared per prepaint; the kept row reachable by
`secondary-1`); the scoped re-review (sonnet) approved with no Critical or
Important issue.

**Deferred findings (Minor).**
1. The list-bounds fallback (R-42) is untested: on the test platform every effect
   flush draws the window, so a row in the model is drawn before any key reaches
   it. Stale bounds for a listed row scrolled out of a virtualized view are
   cleared by the per-prepaint clear.
2. The "all `b`" clause of the stand-in assertion would fail if `c` were ever
   polled automatically, unrelated to the network guarantee; it could be dropped.
3. The new "built disabled" and sentence checks have no mutation evidence that
   they can fail.
4. The `PollOnce`-in-flight predicate exists three times
   (`scheduler.rs:246-248`, `repositories.rs` `checking_now`,
   `settings/polling.rs:199-203`): a `Services::checking_now(root)` for all.
5. `RepositoryMenu::open` has 8 arguments; carrying `RowAction` on `MenuItem`'s
   state-action item would remove the allow.
6. Untested: the `Retry`, `Attend` (positive), `Settings` and `SetUpSharing`
   state actions; `Services::check_now`'s own in-flight refusal; the R-40 toolbar
   branch (`repository_menu.rs:64-68`) — **Task 6 must exercise it** with the
   buttons.
7. Test-only accessors `remove_focus`, `repository_menu_at` live in
   `workspace/repository_menu.rs`, placed there to keep `workspace.rs` ≤ 500;
   `content.rs`, `requests.rs`, `test_support/polling.rs` are over 500 (splitting
   them is owed as cleanup).
8. Task 2's and Task 3's deferred items stand (Task 3's items 1 and 2 are now
   done).

## Task 6: toolbar, More menu, Check all (Part E)

**Main.** `git fetch origin && git merge origin/main` at the start of the task
(plan: Task 6): "Already up to date", origin/main `37be101d` an ancestor of
HEAD `9cc1a4a0`. No Wave 04 Cycle 03b, D1, D4 or D5 follow-up branch has
reached main, so no owner checkpoint is due.

**Commits.** Code `b8044b60` (base `9cc1a4a0`); fix round 1 `c0ab728a`.
Boundary command prints nothing. New: `model/repository_state/menu/check_all.rs`
(44), `view/repositories_tests/toolbar.rs` (331),
`view/dock_tests/toolbars/repositories.rs` (171). `workspace.rs` 491,
`repository_state.rs` 496, `lists.rs` 500 (untouched), `toolbars.rs` 411 (the
Repositories journey moved to the child module because the file would have
reached 544).

**Tests run** (`devenv shell -- cargo test --locked --features
desktop-test-support --bin manyhands <path> -- --exact`, 1 test each, all
green):
- Red: the pure table (22 compile errors for the missing buttons, constants and
  `check_all_*`); E-12's `refresh-cw` errored until `RefreshCw` was added.
- `desktop::model::repository_state::tests::check_all_takes_each_repository_that_can_check`
  (new, pure table); `desktop::assets::tests::every_item_view_icon_loads`
  (E-12 part); `desktop::view::repositories::tests::check_all_checks_each_repository`
  (new); `…::the_more_menu_acts_on_the_chosen_repository` (new);
  `desktop::view::dock::tests::toolbars::panel_toolbars_work_by_tab_and_enter`
  (E-11); `…::the_menu_by_pointer_and_keyboard` (menu code changed).
- Sanity runs, unedited, to see whether three new Tab stops disturbed region
  walks: `keyboard_tests::every_core_action_works_by_keyboard_alone`,
  `workspace::tests::f6_cycles_the_regions_and_secondary_digits_focus_each`,
  `workspace::tests::the_repository_selector_selects_with_arrows_and_enter` —
  ok. `behind_a_dialog`'s `ToolbarButton::ALL` loop (the Task 0 watch item) passed
  unedited: `press_toolbar` returns early under a modal.
- Fix round: the five covering tests above plus
  `desktop::model::polling::status::tests::status_text_table` (to prove
  `stored_applies` is unchanged): each 1 passed. Mutation checks (reverted):
  without press-time `release_check_all` the focus-to-list check fails; with the
  stored exclusion a no-op the window test fails with `[a, b, d]`; using
  `stored_applies` for Check all fails with `[a, b, e]`.
- Verify lines, each exit 0 after each round: `cargo fmt --check`, `cargo check
  --locked --features desktop`, `cargo clippy --all-targets --all-features
  --locked -- -D warnings`.

**Named test edits.** E-11: `panel_toolbars_work_by_tab_and_enter` loops Add and
More (Check all excluded), `with_no_repository` names all three buttons (Add
enabled, Check all disabled, More enabled) and opens More with nothing chosen
(its three items send nothing, Esc returns focus to More); the old
`assert!(!enabled(..))` became `assert_eq!(enabled(..), on, ..)`, the same
assertion for the original four buttons; the window fixture gained a repository
`b`. E-12 part: `refresh-cw` added (`ICONS` 48 → 49); `plus` and `ellipsis` are
Kit built-ins. No E-19: no other existing test changed; no assertion removed;
`toolbars.rs` holds one test before and after. Visibility-only widening of
`repositories::tests` and its `support` helpers.

**Owner ruling, asked and answered 2026-11-12 during Task 6 (amends R-32 and
Part E).** Check all SKIPS a repository whose stored outcome says it waits on a
key or host (`UnlockRequired`, `SelectedKeyUnavailable`, `HostApprovalRequired`)
whether or not the repository is paused, off or in a busy, interrupted or failed
wait — decision 16 (one click never raises prompts one after another) governs.
The documents contradicted themselves: Part E ("`stored_applies` gives none of
`UnlockRequired`, …; paused ones included"), R-32 and Part A ("Check all use it
and nothing else") made `stored_applies`, which is `None` for a paused
repository, the only test, so Check all took a paused repository with a stored
locked key and a user-lane `PollOnce` would have asked for the passphrase (U-6),
one prompt per distinct key; a live `KeyLocked` on a paused repository was
already skipped. Implemented as `status::stored_for_check_all`: `stored_applies`
and it both call a private `stored_when(facts, strict)` holding every shared
condition (checking on, not held, no attempt this session, not checking, not in
flight, no conflict pending); `strict` adds only "not paused" and "quiet phase".
`since_library`, `repository_state`, the row and tooltip keep `stored_applies`
(R-32 stands for them). Cost if wrong: a future change to the shared conditions
must keep both callers in step. The table's paused-`UnlockRequired` row changed
from `true` to `false` (owner-ruled, not a removal) with rows for the other two
outcomes paused, a stored key wait while off, and each wait kind; a window check
that a paused repository with a stored locked key gets no `PollOnce` and no
prompt. **The implementer's first ruling (a paused repository with a stored
`UnlockRequired` is taken) is superseded by this ruling.**

**Rulings (implementer's, reviewed).**
1. `RepositoryMenu.root` is `Option<RepoKey>`: More opens with nothing chosen,
   every item disabled (the choice carries an empty root, which `still_offered`
   rejects; such a menu is never dropped for its root leaving the rows) — cost:
   one `Option` in a Task 5 type.
2. Check all does not look at `kept`, a pending conflict,
   `Attention(ConfigurationRequired)` or an unreadable/no-schedule repository
   beyond the polling shape — Part E lists only the key and host reasons and the
   row menu's Check now has the same Polling-and-not-checking rule; the reviewer
   found no sentence that decides it and no prompt at stake (a kept row is
   checked only if the scheduler still holds a `Polling` schedule); pinned in the
   table as today's behaviour (true) — cost: one harmless check per such
   repository.
3. `toolbar_enabled` for Check all builds the rows (`check_all_roots`) on each
   header draw — one source with the press and the R-22 release — cost: extra row
   construction per header render, no I/O (deferred item 1).
4. `toolbar_focus` is typed `[FocusHandle; ToolbarButton::ALL.len()]`.
5. The More button's bounds come from a `canvas` in a wrapper `div` around the
   button, kept in `RepositoriesPane::more_bounds`.
6. R-22: press-time `release_check_all` (focus moves to the list when Check all
   was focused and nothing more is taken) plus a render-time guard in
   `panel_toolbar` (`cx.defer_in`).
7. R5-9 filled: `repository_menu_chosen` maps Refresh and Rebuild to
   `maintain_index(false|true)`, acted on only when the menu's root is still the
   chosen repository; `still_offered` checks them against `more_items`; Remove
   from More goes through the generic arm with the More button as `opened_from`.
   `maintain_index` is the existing request the sidebar and list actions use.

**Dead-code allows.** Removed: `// Used from Task 6` on `MenuChoice::Refresh`,
`MenuChoice::Rebuild`, `more_items` and its re-export. Added: none. Remaining:
Task 9's `RepositoryRow::tooltip()` and `menu()`.

**Review** (fresh reviewer, opus, `9cc1a4a0..b8044b60`): toolbar, the R-40
branch (exercised by E-11 with a same-update focus read), the More menu, E-11
and E-12, reuse of `maintain_index`, dead-code allows and sizes verified; Check
all is a subset of the menu's Check now (no repository whose own Check now is
disabled is taken); no Critical. Important: `repository_state.rs` grew from 496
to 498 (fixed). The reviewer raised the Check all stored-outcome question as a
contract conflict for the owner (ruling above). Fix round 1
(`b8044b60..c0ab728a`): size, the owner ruling, three test-hygiene fixes (the
Refresh wait now picks the request Refresh just sent — the report notes
`sent[0]` was in fact already the newest; the Esc-to-More assertion is
unconditional; `close_menu` keys on an open menu through the new
`Workspace::repository_menu_open`); the scoped re-review (sonnet) approved.

**Deferred findings (Minor).**
1. The header rebuilds every row on each draw (`check_all_roots`), and
   `defer_in` runs each frame while Check all is disabled; an `is_focused` test
   before the defer is the tightening.
2. The render-time R-22 path (Check all focused, then disabled by something
   other than its own press) is untested; only the press-time path is pinned by
   mutation.
3. A stale Remove from a More menu opened for repository `a` still removes `a`
   after `b` is chosen (Refresh and Rebuild are guarded by
   `navigation.selected() == root`); defensible — the dialog names the
   repository — but asymmetric.
4. `stored_for_check_all` also lets a stored key or host outcome exclude a
   repository in any non-quiet phase where `last.is_none()` and nothing is in
   flight (for example a non-key `Attention` phase): consistent with the ruling,
   not pinned in the table.
5. A doc line in `menu/check_all.rs` runs past the file's wrapping.
6. The static gate ran `cargo check --features desktop`; `--all-features` runs at
   Task 10 (AGENTS.md).
7. **Task 9's privacy sweep must cover the toolbar and More menu strings** (all
   constants; no repository data reaches them).

## Task 7: the details block leaves the item area (Part F)

**Commit.** Code `6899a165` (base `19800b2d`; trailer "Claude Sonnet 5.5").
Boundary command prints nothing. Production: `sidebar.rs` 295 → 27 (the block,
`selection`, `retry`, `remove`, `resolved`, `ShareMain`, `UNAVAILABLE`,
`RESOLVING`, `REMOVE`, `SHARE_MAIN_BUTTON`, `NOTHING_SELECTED` and the unused
imports go; `RETRY`, `SELECTED`, `SHARE_MAIN`, `SHARE_MAIN_SET_UP`, `SHARING`,
`configuration_label` stay, fact 6); `content.rs` 772 → 761 (`NOTHING_SELECTED`
and `OPEN_FROM_THE_LISTS` live here with an `empty_pane()` element, id
`item-area-empty`; `repository_details` and the `end_region_with` calls go);
`render.rs` 359 → 346; `item.rs` 620 → 605 (`RegionEnd`, `end_region_with`);
`item/page.rs` (`region` loses its `end` parameter and the `item-region-end`
wrapper); `workspace.rs` 491 → 487 (`remove_focus`, `retry_focus` and the test
accessor go); `model/list_state.rs` (`index_label`, `LABEL_CURRENT` deleted as
unused). The Task 0 sweep's first command now finds no production site; its
only hits are the 12 absence ids in the new test.

**Tests run** (exact names; final run 23 ran, 23 passed, 25.4 s):
`desktop::view::workspace::tests::the_item_area_holds_nothing_of_the_repository`
(new, P14); E-1 ×6 `selecting_a_repository_resolves_it_and_shows_its_configuration_and_index`,
`a_renamed_away_repository_is_unavailable_until_retry_finds_it_again`,
`a_registration_removed_by_a_second_actor_shows_unavailable`,
`a_list_reload_after_a_second_actor_removes_the_selection_shows_unavailable`,
`a_held_resolution_of_a_never_replaces_b`,
`reselecting_or_retry_shows_a_configuration_refreshed_by_a_second_actor`; E-2
`a_busy_list_reload_keeps_the_last_rows_with_a_label`; E-3
`the_repository_selector_selects_with_arrows_and_enter`; E-4
`desktop::view::tabs::tests::opening_a_tab_does_not_change_the_selection`; E-5
`keyboard_tests::actions::{retry,remove}_an_unavailable_repository_by_keyboard`;
E-6 `settings::repositories::manage_tests::{removing_a_registration_whose_folder_was_renamed_away_succeeds,
the_sidebar_remove_of_a_registration_already_gone_lets_the_entry_go}`; E-7 ×5
`item::share::tests::{share_main_in_the_repository_detail_submits_repo_sync,
share_main_without_a_remote_reads_share_main_and_opens_remotes,
an_accepted_share_offers_stop_and_stopping_signals_it,
a_stopped_share_says_what_completed_and_offers_share,
share_opens_remotes_on_its_own_repository_after_another}`; E-8
`keyboard_tests::discussion::tab_order_runs_header_share_chips_page_discussion_composer`;
three `select_a_repository` users `copy_by_keyboard`,
`frame::the_discussion_is_reached_by_its_key_walked_and_left`,
`every_core_action_works_by_keyboard_alone`; and a named doubt (Tab from Write a
comment… now leaves the item) `item::discussion_tests::an_empty_new_comment_composer_is_one_tab_stop`
(1 passed). Red run: 20 names, 17 passed, 3 failed (the new test, E-8 in its
first form, and an E-6 assertion the implementer had written wrong); the 17 had
their replacement readings from Tasks 3-6, so they pass either way — the block's
absence is pinned by the new P14 test and by E-8. Mutation check (reverted): the
menu's Remove given the Content region's handle instead of `opened_from` makes
both E-6 tests fail at their focus reads. Verify lines, each exit 0: `cargo fmt
--check`, `cargo check --locked --features desktop`, `cargo clippy --all-targets
--all-features --locked -- -D warnings`.

**REMOVED ASSERTIONS** (this is the durable record; every one is authorized by
the named E-n and reviewed: the reviewer went through the diff's `-` lines and
found none outside this list; the only test function lost is E-7's named one).
- E-1 `selecting_a_repository_resolves_it_and_shows_its_configuration_and_index`:
  `selection-name == "a"` → the chosen row's mark `== SELECTED` and the other
  mark absent; **"Configuration valid" (`selection-configuration`)** → the row's
  state is not Needs fixing and no `NEEDS_FIXING_SENTENCE` tooltip line (the
  `configuration.state == Valid` oracle line kept; Part F: nothing for a valid
  configuration); **`selection-index == index_label(…)`** — no longer in the
  product (Part F, index row: the lists' states); `!present(selection-unavailable)`
  → `("repo-state", 0) != CANT_OPEN`.
- E-1 `a_renamed_away_repository_is_unavailable_until_retry_finds_it_again`:
  `selection-unavailable == InvalidPath.message()` → `CANT_OPEN` plus the
  tooltip's first line starting with that message; `selection-retry` /
  `selection-remove` present → the menu (`secondary-1`, `shift-f10`) lists
  `RETRY` and `REMOVE_FROM_MANYHANDS`; `!selection-configuration` while
  unavailable → `Resolution::Unavailable(InvalidPath)` and `CANT_OPEN` (kept);
  the step `click(selection-retry)` → the menu's Retry by arrows and Enter
  (`choose_by_keys`, which asserts the list holds focus in the update of the
  Enter); the after-Retry checks → `Resolution::Resolved`, state `!= CANT_OPEN`,
  tooltip no longer starting with the message.
- E-1 `a_registration_removed_by_a_second_actor_shows_unavailable` and
  `a_list_reload_after_a_second_actor_removes_the_selection_shows_unavailable`:
  `selection-configuration` before / absent after → `Resolution::Resolved` before
  and the kept row; `selection-unavailable == RepositoryNotRegistered.message()`
  → the kept row drawn after `b` (`row_of == 1`), `("repo-state", 1) == CANT_OPEN`
  and tooltip[0] `== kept_sentence("a")` (decision 22; stronger than before);
  **`selection-index` absent** (first test) — no longer in the product.
- E-1 `a_held_resolution_of_a_never_replaces_b`: `selection-name == "b"` → the
  mark on `b`, none on `a`.
- E-1 `reselecting_or_retry_shows_a_configuration_refreshed_by_a_second_actor`:
  **`selection-configuration == valid` (×2 per root)** → state `!= NEEDS_FIXING`
  and no Needs-fixing tooltip line at both points; `selection-configuration ==
  invalid` → state `== NEEDS_FIXING`, tooltip[0] starts with
  `NEEDS_FIXING_SENTENCE` and contains each current problem's `guidance()`, and
  the read gives at least one problem; **`selection-index`** — no longer in the
  product.
- E-2 `a_busy_list_reload_keeps_the_last_rows_with_a_label` and E-3
  `the_repository_selector_selects_with_arrows_and_enter`: `selection-name` →
  the mark (E-3's was already redundant with the mark lines that follow).
- E-4 `opening_a_tab_does_not_change_the_selection`: `selection-name == "b"` →
  mark `b`, none on `a`.
- E-5 helper `unavailable` and `retry_/remove_an_unavailable_repository_by_keyboard`:
  `selection-unavailable-state`, `selection-configuration`, the steps
  `secondary-3`, `tab_to(selection-retry|selection-remove)`, `on(…, Content)` →
  `CANT_OPEN`, `region(Repositories)`, `shift-f10`, the menu's items, highlight
  Retry/Remove then Enter (`press_and_hold` reads the list in the same update),
  `Resolution::Resolved`; the confirmation, success, the registration gone and
  the folder untouched are kept.
- E-6 `removing_a_registration_whose_folder_was_renamed_away_succeeds`: the Tab
  loop reading `workspace.remove_focus()` and `selection-remove` → `CANT_OPEN`,
  `secondary-1 shift-f10`, `open_root == root`, highlight Remove, Enter; the
  dialog is opened twice, Escape the first time with `press_and_hold("escape",
  list)` (list holds focus in the same update, mutation-checked); end: `!selection-unavailable`
  → `rows("repo-row")` is empty. `the_sidebar_remove_of_a_registration_already_gone_lets_the_entry_go`:
  `click(selection-remove)` → the kept row alone (`rows == ["a"]`, Can't open),
  `secondary-1 shift-f10`, `open_root == root`, Remove, Enter; end → no rows, and
  Close in the same update gives the list focus (mutation-checked);
  `AlreadyApplied` and selection `None` kept.
- E-7 five tests: `label(SHARE_MAIN_BUTTON) == SHARE_MAIN | SHARING |
  SHARE_MAIN_SET_UP` and `click(SHARE_MAIN_BUTTON)` → `menu_items` offers the
  label (`items_of`) and the item is chosen from the menu by label; while held it
  offers `SHARING` and not `SHARE_MAIN`; every other assertion kept.
  **Deleted test `item::share::tests::share_main_at_the_end_of_an_item_tab_submits_repo_sync`**
  (its tab shown, End reaching `SHARE_MAIN`, `repo_syncs == 1`, `SHARING`,
  `item_syncs == 0`) — no tab carries the details any more (Part F, "Share main
  → the menu"); Share main by the menu sending `RepoSync` is pinned by the E-7
  tests, and that nothing of the repository ends a tab by the P14 test and E-8.
- E-8 `tab_order_runs_header_share_chips_page_discussion_composer`: the
  `SHARE_MAIN` constant and `holds_focus(SHARE_MAIN)` after the Tab from
  `new-comment-start` → Tab from there leaves the Content region
  (`held_region != Some(Content)`) and Shift-Tab comes back; and a **second site
  the Task 0 sweep missed** (`discussion.rs:367`, the constant used, not the
  literal): `tab_to("shift-tab", SHARE_MAIN)` + `step("shift-tab",
  "new-comment-start")` → `tab_to("shift-tab", "new-comment-start")` and
  `on("new-comment-start", Content)`.

**Needed edits (no assertion removed by any of them).**
- **E-19:** `repositories_tests/support.rs` (426 lines): `items_of` and
  `right_click` widened to `pub(in crate::desktop::view)`; `labels` and
  `press_and_hold` delegate to new `menu_labels` and `press_and_hold_in`; new
  `highlight` (presses `down` until the Kit item reads `aria_selected`, bounded
  at 16) — shared by the workspace tests' own `Shell` and the harness `Shell`.
- **E-20:** `workspace_tests.rs` 1335 → 1274: its two-repository `Shell`, `key`,
  `row` and `alert_action` move unchanged (`pub(super)` added) to the new child
  `workspace_tests/shell.rs` (243 lines) with the new readings (`mark`, `state`,
  `row_of`, `tooltip`, `needs_fixing`, `open_menu`, `choose_by_keys`,
  `details_block`); test functions and paths unchanged.
- **E-21:** E-8's second site (`discussion.rs:367`), above.
- **E-22:** E-6 test 1 gains the Escape round — Confirm keeps the dialog open
  ("Submitted") until the request finishes, so focus after OK cannot be read in
  the update that causes it, and a read after a draw could be satisfied by
  `Workspace::focus_lost`'s fallback.
- Test functions before/after: `workspace_tests.rs` 28 → 29 (adds the P14 test);
  `item/share_tests.rs` 17 → 16 (E-7's named deletion only); `tabs_tests.rs` 39,
  `keyboard_tests/actions.rs` 7, `repositories_manage_tests.rs` 20,
  `keyboard_tests/discussion.rs` 2 — identical. Sizes: `repositories_manage_tests.rs`
  876 → 918 (+42, already over 500), `tabs_tests.rs` +6.

**Rulings (implementer's, reviewed).**
1. `index_label` and `LABEL_CURRENT` are deleted — nothing used them once the
   block went (the reviewer's grep: no user in src, tests or plans); an allow
   would keep dead copy; `LABEL_STALE` and `LABEL_NEVER_REFRESHED` stay (the
   lists use them) — cost: "Index current" would be re-added if a later Cycle
   wants it.
2. The empty pane is `div#item-area-empty` with `test_support` and `aria_label`
   set to its text — cost: a rename if Cycle 05 chooses another id.
3. "Chosen" is `navigation.selected_entry().is_some()`, the old block's own
   condition, so the kept row counts as chosen and shows `OPEN_FROM_THE_LISTS`.
4. `page::region` loses its `end` parameter instead of receiving `None`.
5. E-8 becomes a positive check (Tab from Write a comment… leaves Content)
   instead of just dropping the steps, which would have passed whether or not
   the details were drawn; a single Shift-Tab from the strip cannot reach the
   composer (Tab order spans the window), so the second site keeps `tab_to` —
   cost: it must change if a later Cycle adds a Tab stop after the discussion
   inside the item.
6. The menu is driven by keyboard in E-1, E-5 and E-6 (`highlight`, Enter) and by
   pointer in E-7 (right click, `choose`) — `highlight` relies on the Kit marking
   its highlighted item `aria_selected` (0.6.6, `menu_item.rs:101`).
7. `RepositoryRow::tooltip()` is gated `all(test, feature =
   "desktop-test-support")` and its `// Used from Task 9` allow is gone (window
   tests use it now); `menu()` keeps its Task 9 allow.

**Review** (fresh reviewer, opus, `19800b2d..6899a165`): spec compliant; Part F
holds, E-1 to E-8 applied as written; removed-assertions audit: no removal
without authority, none missing from the table, no test lost beyond E-7's; every
production deletion confirmed unused; the new P14 test asserts the absence of
every id the old block drew; the kept row correctly gets `OPEN_FROM_THE_LISTS`;
no Critical, no Important.

**Deferred findings (Minor).**
1. Focus after a successful Confirm (OK) is not pinned: Escape, Cancel and
   success all call `close()` on the same `origin` (`confirm.rs:537-545`, `:255`,
   `:883-886`) and the mutation on `opened_from` breaks both E-6 tests; the
   reviewer found no cheap discriminating read for OK (the fallback also lands
   on the list). Task 5's menu journey pins Cancel, not OK.
2. **Triage at the final review (cheap fixes):** E-1 test 1's configuration
   checks are weak (`assert_ne!(state, NEEDS_FIXING/CANT_OPEN)` and
   `!needs_fixing` pass if the tooltip is empty or the state element is missing;
   assert a non-empty tooltip such as a "Folder:" line, or the exact state);
   E-8 passes if focus is lost entirely (`assert_ne!(region, Some(Content))` also
   holds when `held_region` is `None`; add `assert!(region.is_some())`); E-1 test
   2's `open_menu` does not assert `open_root == a`.
3. A `list_focus` helper is copied in three places
   (`repositories_manage_tests.rs:606`, `workspace_tests/shell.rs:190`,
   `keyboard_tests/actions.rs:202`): one in `repositories_tests/support.rs`
   beside `press_and_hold` would trim `repositories_manage_tests.rs`.
4. A stale doc comment in `workspace_tests.rs:688-690` ("Selection shows
   Unavailable with Retry and Remove").
5. The P14 test's `no_details` assertions were never seen failing against the old
   tree (its red run failed at the first text check), and E-8's final form was
   not run red; both were traced by reading.
6. Unedited tests that `tab_to` across an item tab's end now skip one stop;
   `tab_to` tolerates it, and the fast tier at Task 10 runs them.

## Task 8: footer status items (Part G)

**Commit.** Code `d8013dcc` (base `b2997259`). Boundary command prints nothing.
New: `view/workspace/footer_status.rs` (181), `footer_tests/status.rs` (410),
`footer_tests/status_table.rs` (215). `footer.rs` 437, `workspace/footer.rs` 285,
`workspace.rs` 488 (one `mod` line), `footer_tests/mod.rs` 491. `lists.rs` and
`repository_state.rs` untouched. Pure functions beyond the brief's list:
`to_share_text(n)` ("{n} to share"), `focus_after_hiding(hidden, plan)` (R-38's
decision logic) and `update_tooltip(text)`; `FooterStatus` also derives
`Default`; the brief's `footer_status(&self, cx)` is
`Workspace::share_facts(cx).status(attention)` so the name and the text share one
evaluation per draw.

**U-5 holds** (scratch test, discarded): `reach(Attention)` takes 12 Tab presses
and `reach(Settings)` 15 today, 14 and 17 with the two new stops, far below
`TAB_LIMIT` 40 (unchanged; neither status item draws in `plain`, where nothing is
scheduled).

**Tests run** (`devenv shell -- cargo test --locked --features
desktop-test-support --bin manyhands -- --exact <paths>`): 12 ran, 12 passed —
the new `desktop::view::footer::tests::status_items_say_and_choose`; the four E-10
tests (`footer::tests::a_secondary_click_opens_the_dock_menu_with_its_side_checked`,
`…::toggle_names_follow_their_state`,
`keyboard_tests::shell::each_side_is_reached_by_the_dock_menu_keys`,
`…::footer_controls_are_reached_by_tab`); the Task 0 watch item
`footer::tests::a_dock_menu_closes_when_a_modal_opens_over_it` (`Control::ALL`,
passes unedited, and now iterates 11 controls); and the `reach` and `said` users.
A precaution, because the footer now draws the items in polling windows: 7 polling
window tests passed (`polling_privacy`, `polling_keyboard`, five repositories
journeys). Verify lines, each exit 0: `cargo fmt --check`, `cargo check --locked
--features desktop`, `cargo clippy --all-targets --all-features --locked -- -D
warnings`.

**Named edits.** E-10: the exact nine-control order in
`footer_controls_are_reached_by_tab` replaces `order.len() == Control::ALL.len()`
and asserts both status items absent (stronger: identity and order, not a count);
the `plan(.., 0)` / `plan(.., 3)` call sites became `FooterStatus::default()` /
`FooterStatus { attention: 3, .. }` with the same assertions; `said` gains the two
controls as a panic arm; the two `footer_tests` edits only add the two controls to
their expectations. No assertion removed or weakened. **E-23** (needed edit,
visibility only): nine helpers in `repositories_tests/support.rs` (`FIRST`,
`SECOND`, `THIRD`, `id`, `not_due`, `completed`, `item`, `checked`, `polled`)
widened from `pub(super)` to `pub(in crate::desktop::view)` so the footer journey
reuses them; no body changed. No E-12 edit (no icon needed).

**How R-38 was proven.** The journey focuses To share, brings the count to 0 by a
share, and reads focus on Needs attention; removing the `keep_focus_in_footer` call
(`workspace/footer.rs:51`) fails it at `footer_tests/status.rs:327-330` (the
report's "status.rs:510" is a typo), and a second mutation on the press also
failed; both reverted. The reviewer confirmed the assertion cannot be satisfied by
`Workspace::focus_lost`'s fallback (it focuses only regions and panes, never a
footer control).

**R-38's `window.focus` runs during render — judged correct, not a defect.**
Checked against the locked `gpui-pre-0.3.6/src/window.rs`: `Window::focus` sets
focus and calls `refresh()`, which does nothing while drawing, and the post-draw
re-frame test compares against a focus value captured after the swap, so there is
no extra frame or loop; `keep_focus_in_footer` runs before the footer's elements
are built, so the target is drawn focused in the same frame, and the target is
always in the drawn plan (Settings is always there). Deferring (`defer_in` /
`window.defer`) would be wrong: when the focused control leaves the tree GPUI
fires `focus_lost` in the same draw, before any deferred callback, and
`Workspace::focus_lost` would move focus to a region and overwrite `last_region`.
Task 6's R-22 could defer only because a disabled Check all stays in the tree.
Precedent: `DockMenu::layer` (`footer/menu.rs:98-102`) takes focus on draw. Not
seen on a real display (the owner's on-screen checklist covers it).

**Rulings (implementer's, reviewed).**
1. Update check's "checking" and "latest" are over the listed repositories, not
   the kept row (the brief's `schedule().repo(root)` for every listed repository
   settles it; the design's "any scheduled repository" read the same way) — cost:
   a removed repository's last check stops counting (K-10).
2. Update check's accessible name is the text; its tooltip is
   `"{text}\n{UPDATE_TOOLTIP}"` and `UPDATE_TOOLTIP` is also the wrapper's
   accessible description. Part G's Name column reads "The text, then
   'Background check…'" — see deferred 1.
3. `next_to_share` with a chosen key not in `order` (the kept row) starts at the
   top of the order; a chosen repository with no work: the next with work after it.
4. The name's parent is the parent folder's file name (or its full path when it
   has none — see deferred 2); "in {parent}" is added for every listed repository
   whose name another listed one shares (both get it).
5. Attention hiding while focused is left as today (fallback to a region); R-38
   names only the status items.
6. Icon colours (refresh-cw teal, cloud-upload amber) are appearance and
   untested.

**Dead-code allows.** None added or removed.

**Review** (fresh reviewer, opus, `b2997259..d8013dcc`): spec compliant, every
brief interface and case present; R-24, R-26, R-38, R-39 and Review Focus 6
verified (the share-and-press in the same update, before any draw, leaves the
footprint — chosen repository, layout, saves, reads, requests — unchanged);
E-10 edits equal or stronger; no Critical, no Important.

**Deferred findings (Minor).**
1. **Triage at the final review:** Part G's Name column says the Update check's
   name is "the text, then 'Background check: fetch and fast-forward only. Never
   shares your changes.'"; the implementation puts the sentence in the tooltip and
   `aria_description` and only the text in the accessible name. Defensible (screen
   readers read the description) but a literal reading wants it in the name:
   confirm the split or put `update_tooltip(text)` in the name (a one-line change).
2. Latent full-path fallbacks in the name: `footer_status.rs:67` `display()` falls
   back to the key (the full repository path) for an unlisted key — unreachable
   today (it only occurs when the total is 0 and To share is hidden); the parent
   falls back to `parent.display()` ("/" or "C:\") when it has no file name;
   `(None, None) => to_share_name(total, "", true)` would read "N to share in ."
   (unreachable while the item shows). Fall back to the path's `file_name()` or an
   empty string, never the key; drop the suffix when the parent has no name. The
   design wants names only (Task 9's privacy sweep should look).
3. `ChromeColors::green`'s `#[cfg_attr(not(test), allow(dead_code))] // Cycle 03
   (the update-check and to-share footer items)` in `view/shell/tokens.rs:38-40` is
   stale (`green` is used in production by `Tone::Success`); delete it.
4. Update check hiding while focused is covered by the table
   (`focus_after_hiding`) but the journey hides only To share; one realistic case
   is cheap (focus "Checking for updates…" on a repository that has never
   completed a check, then end the check uncompleted).
5. The journey's own `tab_to` (`status.rs:56-65`) hard-codes `0..40` instead of
   `TAB_LIMIT` and duplicates `Walk::tab_to` / `reach`.
6. Add a line to `keep_focus_in_footer`'s comment saying why it runs during
   render and is not deferred (the `focus_lost` ordering above), so a later reader
   does not "fix" it to match Task 6.
7. The U-5 counts come from a discarded scratch test and cannot be rechecked.

## Task 9: privacy (Part I)

**Commits.** Code `3d6d81b3` (base `6fe57a58`); fix round 1 `7d1ef534`. Boundary
command prints nothing. `polling_privacy_tests.rs` 1001 → 1039 (+38, already far
over 500: the constant, `SENTINELS`, `ALLOWED` and `SURFACES` edits, the `b`
fixture, the password positive control, two call sites); the sweep itself is the
new child module `polling_privacy_tests/repositories.rs` (about 150 lines).
`RepositoryRow::menu()` is regated `all(test, feature = "desktop-test-support")`
and its `// Used from Task 9` allow is gone (`git grep 'Used from Task 9'
src/desktop` is empty); `Workspace::footer_texts` (`footer_status.rs`) is a
test-only accessor under the same gate. No runtime change in production.

**U-3: a password-carrying SSH remote is NOT publication-eligible.** `remote
select` refuses it with `InvalidRemote`; a remote whose URL gains a password
after selection lists as selected but ineligible, gets no shared copy
(`registration.rs` requires selected and eligible for both `ssh_publication` and
`shared_copy_of`) and is never polled. The fixture is therefore "select over SSH,
then `set_remote_url` with `ssh://git:sentinel-password-03a@example.invalid/r.git`"
on a second repository `b` (the brief's `a_remote_url_with_a_password_is_shown_redacted`
adds an unselected remote, which would never be selected or eligible), and the
password is shown present in `b`'s `.git/config` by a positive control. Part I's
assertions hold either way.

**Tests run** (`devenv shell -- cargo test --locked --features
desktop-test-support --bin manyhands <path> -- --exact`):
`desktop::view::polling_privacy_tests::polling_privacy` — red (the new surfaces
and the sweep first), then 1 passed, and 1 passed again after the fix round; the
other two privacy guards by exact name (`editing_privacy_tests`,
`lifecycle/privacy_tests`): 2 passed. Scratch runs showing the guard can fail,
each reverted with no residue in `git diff`: the password sentinel put into
`repository_tooltip` fails at the description surface; PATH appended to
`to_share_text` fails at the footer text. Verify lines, each exit 0 after each
round: `cargo fmt --check`, `cargo check --locked --features desktop`, `cargo
clippy --all-targets --all-features --locked -- -D warnings`.

**What the sweep covers.** Every listed repository (two: `a` and `b`; no kept row):
`repository_tooltip` lines and the joined description, `menu_items` labels,
`more_items` (nothing chosen and each row chosen), the toolbar and panel constants,
the footer's `update_text`, `to_share_name` / `to_share_text` and `update_tooltip`
for each listed repository (as next and as chosen), plus the live drawn surfaces
(`SURFACES` now 17, with `("repo-", "", "repository row")` and `("footer-", "",
"footer")`), against `BODY`, `COMMENT`, `DRAFT`, `EDIT`, `PATH`, `PASSPHRASE`,
`PRIVATE` and the URL password; not `TITLE` (titles are allowed by design). Positive
controls: at least four tooltip lines and a "Shared copy:" line (P17), every
surface kind non-empty, the password present in the library's data, and (fix round)
`a`'s key in `polled_roots` before `b` is asserted absent, and `b`'s remotes
read (`remote_read_roots`) before `shared_copy == None`.

**Named edit E-15.** `region-repositories` removed from `ALLOWED` (12 → 11: a
tightening), `SURFACES` 17, the URL-password sentinel, `std::slice::from_ref(&a)`
→ `[a, b]`, and `answer_unanswered_polls(not_due(clock))` added so the second
repository never reaches the network. No assertion removed or weakened. No E-24.

**Review** (fresh reviewer, sonnet, `6fe57a58..3d6d81b3`): the guard is not
vacuous (every row, all surface kinds, eight secret sentinels, a numeric positive
control); the U-3 fixture and answer credible; no production behaviour change; the
two production-file hunks are test-only accessors. One Important: `footer_texts`
was `#[cfg(test)]` while its only caller is compiled under `all(test, feature =
"desktop-test-support")`, inconsistent with `menu()`'s regate — fixed in fix round
1 (`3d6d81b3..7d1ef534`) with two cheap guard controls; the scoped re-review
(sonnet) approved with no new issue.

**An observation for the owner (not part of this Cycle).** `cargo clippy --tests
--features desktop --locked -- -D warnings` (the test build WITHOUT
`desktop-test-support`) exits 101 with 51 compile errors, none in this task's
files: it needs `gpui_kit::TestAppContext`, `run_test` and `block_test`, and
`view::harness` and `test_support` are themselves gated on the feature. The
reviewer judged it plausibly pre-existing (not checked on a main build); AGENTS.md's
gate uses `--all-features`, which covers it. Because that build fails regardless
it cannot show dead-code warnings for a feature-gated item, so the gate choice
(`all(test, feature = "desktop-test-support")`) stands.

**Deferred findings (Minor).**
1. The footer name's two full-path fallbacks (Task 8 deferred 2: `footer_status.rs:67`
   falls back to the key for an unlisted key; the `in {parent}` suffix falls back to
   `parent.display()` when the parent has no file name) are not reachable from this
   fixture — `footer_texts` only calls `display` for listed keys, so no fixture of
   this kind can reach the first fallback; the sweep found no leak. A small unit
   test of the name logic with a parentless path (such as `/repo`) and an unlisted
   chosen key would cover it; carry to the final review.
2. The scratch mutations covered the tooltip and the footer only; a mutation of the
   `menu_items` labels or the drawn `repo-` surface would show those paths can fail
   too.
3. The drawn "repository row" surface is checked against `TITLE` too, so a later
   task planting an unshared item in this fixture would trip it (tooltip titles are
   not covered here because no unshared item is planted).
