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
| 0 | done | (this commit) | yes |
| 1 | not started | | |
| 2 | not started | | |
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

**U-checks.** U-1 to U-7 open: U-6 and U-7 in Task 1, U-4 in Task 3, U-1 and
U-2 in Task 5, U-5 in Task 8, U-3 in Task 9.

**Tests run.** None.

**Boundary check:** `git diff --stat origin/main...HEAD -- src ':!src/desktop' tests Cargo.toml Cargo.lock packaging` prints nothing.
