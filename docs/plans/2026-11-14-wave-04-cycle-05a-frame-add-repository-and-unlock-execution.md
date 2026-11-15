---
title: "Wave 04 Cycle 05a Execution Ledger"
date: 2026-11-14
status: "draft"
author: "Claude"
---

# Wave 04 Cycle 05a Execution Ledger

Written only by the orchestrator. Ticket `01M7D5Z7FZTGTC1HE25KQ1MYQY`
(`vh-kebv6`), branch `manyhands/ticket/01M7D5Z7FZTGTC1HE25KQ1MYQY`.
Execution is subagent-driven and serial (decision 21): one fresh implementer
per task, then a fresh reviewer.

Rulings are recorded as "Ruling: decision — why — cost if wrong". Test edits
are E-1 to E-16 (named) and E-17 onward (further, decision 8), each with its
reason. Commit trailer: `Co-Authored-By: Claude Sonnet 5.5
<noreply@anthropic.com>` (Ruling 5).

## Task state

| Task | Status | Commit | Pushed |
| --- | --- | --- | --- |
| 0 | review approved after fix round 1 (Minors parked) | merge of Cycle 04's branch `f5015aab`; design `81c3c0de`; Wave `5b7771db`, fix `42f807d5` (base `f5015aab`); ledger `4c515b5b`; merge of Cycle 04's tip `edb0e920`; ticket comment `46b5a11a` | yes |
| 1 | done, review approved first time (Minors parked) | code `ded7ca46` (base `0ac6af37`); ledger `d9533dd9` | yes |
| 2 | done, review approved after fix round 1 (Minors parked) | code `b2618100` (base `d9533dd9`), fix `31497e0e` | with the ledger commit |

## Branches published

- `manyhands/ticket/01M7D5Z7FZTGTC1HE25KQ1MYQY`, after each task (authorized,
  owner 2026-11-11): after Task 0 at `edb0e920`, then by its comment `46b5a11a`.
  It carries Cycle 04's commits through the merge. No other branch published yet.

## Base and merges

- Base: this branch was created from `origin/main` `ea6d899b`; at handoff it
  was at `1064e5a8`. `origin/main` is still `ea6d899b`.
- **Owner override, 2026-11-14 (this session).** Cycle 04
  (`01M7AMAPNHHTNECXBF4CA5WPKV`) is not closed or merged: its implementer is
  chasing the root cause of a flaky test. The owner directed that Task 0
  merge Cycle 04's branch instead of main, in place of Task 0 Step 1's
  precondition and the handoff's "stop if Cycle 04 is not merged".
- Merge `f5015aab`: `manyhands/ticket/01M7AMAPNHHTNECXBF4CA5WPKV` at `7a484a7a`
  into `1064e5a8`. No conflicts. `origin/main` is an ancestor of the result.
  The Cycle 04 branch differs from main only under `src/desktop/` and `docs/`
  (nothing under `src` outside `src/desktop`, `tests`, `Cargo.toml`,
  `Cargo.lock`, `packaging`), and `Cargo.lock` is unchanged, so the
  `gpui-kit`, `gpui-base` and `gpui-component` stop does not apply.
- Cycle 04's branch is still moving. Its tip is merged again before Task 7
  and before review-ready, or `origin/main` instead once Cycle 04 lands.

## Rulings

Ruling 1: Task 0 Step 1 is replaced by the owner's override (merge Cycle
04's branch tip, `f5015aab`), and Step 2's `git merge origin/main` is a
no-op because `origin/main` is an ancestor; the gates still run — the owner
said to merge from their branch "as part of step 0" — cost if wrong: a
re-merge.

Ruling 2: Task 0 may carry several commits (the design correction, the Wave
amendment, the ledger), as its steps say; every other task has one — the
plan text itself asks for separate commits — cost if wrong: none.

Ruling 3: Task 7 merges `origin/main` and, if Cycle 04 has not landed,
Cycle 04's branch tip again — the override applies until 04 is on main —
cost if wrong: a re-merge.

Ruling 4: Task 10 Step 6 is not run by this session. The handoff's "close is
not authorized, leave `target/` and `.devenv/`" binds; the owner runs the
close — cost if wrong: the owner removes the directories.

Ruling 5: the trailer is `Co-Authored-By: Claude Sonnet 5.5
<noreply@anthropic.com>`, not the plan's Opus 5.5 — the handoff says "your
own session's trailer" and this session is Sonnet 5.5 (Cycle 04's ledger made
the same ruling) — cost if wrong: none material.

Ruling 6: Task 3's E-1 move list also takes the three Add and Create tests
in `repositories_tests.rs` the plan left out
(`submitting_with_no_folder_says_so`,
`a_form_control_below_the_fold_is_scrolled_into_view`,
`the_inspect_recovery_opens_the_page_with_its_root_checked`), unchanged
except for their own named parts (E-2, E-1e, E-4) — they open the forms that
move, so leaving them behind cannot work — cost if wrong: a mechanical move
reversed.

Ruling 7: three further test edits are numbered now, under decision 8 (each
checked by its task's reviewer; each assertion change called out here and in
the review-ready summary):

- **E-17a (Task 3):** Cycle 04's P18 copy table,
  `desktop::view::settings::repositories::copy_tests::settings_copy_uses_the_vocabulary`
  (`D/view/settings/copy_tests.rs:186-208`) names every Add and Create
  constant as `repositories::X`. Task 3 re-points the rows to the new module;
  none is removed in Task 3. The source scan covers only `view/settings/`, so
  the moved constants leave its "declared but missing" check until P20 (Task
  9). Tasks 5 and 6 update the rows for each renamed or removed constant
  (`NOT_ADDED`, `CHECKED_OUT`, `NEED_FOLDER`, `NEED_BRANCH`, `ADD_HEADING`,
  `CREATE_HEADING`, `CREATE_FOLDER`, `PRIMARY_BRANCH` to `MAIN_BRANCH`);
  each row removed is an assertion removal.
- **E-17b (Task 4):** Cycle 04's P23,
  `desktop::view::workspace::settings_sheet::tests::focus_stays_inside_settings`
  (`settings_sheet_tests/inside.rs`), uses the Add form as "the control
  inside the sheet" in `tab_trap`, `landings` (F18) and `context_menu`.
  Re-target to controls Settings still draws (`settings-add-repository` as
  the last Tab stop; an identity or remote input for F18 and F19).
- **E-17c (Task 4):** Cycle 04's P10,
  `desktop::view::settings::chosen_tests::one_repository_is_chosen`
  (`chosen_tests.rs:187-210`), raises `SettingsRoute::Inspect(b)`, which R-5
  removes. Re-express by `show_add_repository(Inspect(b))`, Escape, then a
  plain open (this Cycle's P4 claim). The "footer after Inspect(b)" rows stay.
  **Assertion change:** R-55's Settings-side assertion goes, and P4 asserts it
  through the Add sheet.

Ruling 8: Task 1 adds four icons to `icon_assets!` (`FolderInput`,
`GitBranch`, `Cloud`, `X`; `KeyRound` is already there, Cycle 04) and the
five the plan names to `ICONS` — Task 0 found `KeyRound` already declared at
`D/assets.rs:70-73` — cost if wrong: none.

Ruling 9: Task 3 keeps Cycle 04's tests that read Add inputs through
`RepositoriesPage::input` and `control_focus` green by delegating them to the
new entity (Task 3 is "no visible change"; Task 4 removes the delegates), and
its module run adds `desktop::view::workspace::settings_sheet::tests` and
`desktop::view::settings::chosen_tests`. Task 8's run adds
`desktop::view::polling_privacy_tests`. Task 5 makes `wrapped_kit`
(`settings/kit_button.rs`, private to `settings`) reachable from the Add
sheet, a visibility change inside `src/desktop`, not a library gap — cost if
wrong: Task 4 or 5 edits Cycle 04's tests earlier, as E-12.

Ruling 10: Cycle 04's tip is merged again right after Task 0's review
(`5bacd7d2`, test-only fix `f0cfaa39` for the flaky
`the_more_menu_acts_on_the_chosen_repository`, clean by `git merge-tree`),
not left to Task 7, so Task 4's run of `desktop::view::repositories` does not
meet the flake — cost if wrong: none.

## Task 0 (Baseline, References, Ledger)

Base `f5015aab`. Commits: design re-map `81c3c0de`, Wave Amendment 1
`5b7771db`. No file under `src/`, `tests/`, `Cargo.toml`, `Cargo.lock` or
`packaging/` changed; the boundary diff prints nothing. No test ran.

**Gates** (at `f5015aab`): `cargo check --all-features --locked` pass (58.85
s); `cargo fmt --check` pass; `cargo clippy --all-targets --all-features
--locked -- -D warnings` pass (3 m 01 s). `Cargo.lock` and `Cargo.toml`
unchanged since `ea6d899b`: gpui-kit, gpui-base, gpui-component did not move.

**U-1 (Cycle 04's merged names, at `7a484a7a`): no stop.** Cycle 04's code
carries B2 and B3:

- AW-3's origin: `show_settings` focuses the origin before `open_sheet`
  (`settings_sheet.rs:156-158`); the host keeps it as `SettingsHost.origin`,
  read only by `take_origin()` (`mod.rs:300-303`). Task 4 calls `take_origin`
  before `settings_closed(true)`.
- A quiet close that empties the passphrase: `settings_closed(true)` ->
  `SettingsHost::hide` -> `forget_passphrase` (`mod.rs:253-282`).
- A `Manage(root)` route: `SettingsRoute::Manage(PathBuf)`
  (`arrive.rs:20-22`); `arrive` returns `None`, so it lands on
  `("settings-page", 0)`.
- A refusal in the Ctrl+, handler: `Workspace::settings_key`
  (`settings_sheet.rs:347-360`) already returns early at `:351-353`.
- The Kit's close order (`close` runs `request` before `notify`) leaves
  Root's slot free when `on_close` opens Settings.

Design corrections, in `81c3c0de`, changing no contract, G row, P row,
ruling or decision: Facts 9, 10, 13a, 14, 15, 16, 17, 18-21, 22, 23, 23a, 24,
27, 28, 31, 40; A3 (Quit now is not a dialog control: `Workspace::finishing`,
`quit.rs:102-146`); B2 steps 2 and 4; B3 step 3; B4 (`open` is an
`OpenFlag`; the scroll is a `ContentScroll` region); a note under B9; B12
(`wrapped_kit`); Testing (U-4); Named Edits re-lined (E-1b, E-1e, E-4 to E-9,
E-16; E-10, E-11, E-15, E-16 confirmed; E-12 names its sites).

Facts that matter later:

- `sync_sheet`'s `None` arm closes any active sheet, so Task 4 must add the
  `AddRepository` arm or the Add sheet closes at the next render.
- `show_settings` step 4 (`:143-147`) closes any active sheet, so R-22's
  `add_closed(Quiet)` goes right after the refusal (`:124-126`).
- `SettingsHost::layer` tracks Root's whole sheet layer, so the Add sheet
  already counts as "inside" for R-46; extending R-46 means widening
  `settings_open` (`settings_sheet.rs:93-96`) to "Settings or AddRepository
  owns an active sheet", not building a second layer.
- Any Kit button a G row lands on (G4 `settings-add-repository`, G13's
  button, G22's row action) must use Cycle 04's `wrapped_kit` and be asserted
  with `holds_control`; a plain `wrapped` handle passes `holds` and draws no
  ring (Cycle 04 final review Important 1).
- Cycle 04's P8 scroll rows assume Repositories is taller than the 400 px
  window; after the forms leave, a fixture may need more registrations. A row
  that can no longer fail is an assertion removal.
- Busy-retry hazard (Cycle 04 Ruling 52): a test that reads registrations
  synchronously after `run` fails if a read answers busy; only Keys reads got
  `keys_landed`. B9 rows that read the Repositories list after an add can
  meet it.

**Cycle 04 unstable at `7a484a7a`:**
`desktop::view::repositories::tests::the_more_menu_acts_on_the_chosen_repository`
(failed 2 of 3 full runs; fixed by `f0cfaa39`, merged in Ruling 10),
`desktop::view::settings::keys::tests::a_generation_not_sent_for_its_passphrase_shows_the_fields_again`
(Error, Busy once under load; Cycle 04 parked it).

**U-3:** every row of A3's footer table matches the code (14 dialogs); the
Confirmation dialog and Unlock key change (action moves last), the rest keep
today's order. Hand-drawn rings (fact 5): `form.rs:93-104` (Create, New
folder, Move), `quit.rs:139-148` (Quit now, not a dialog's), and
`composer/discard.rs:122-128`.

**U-4:** the Unlock evidence tests are
`desktop::view::keyboard_tests::passphrase::{cancel_by_tab_and_enter_cancels_and_the_key_stays_locked,
enter_in_the_input_still_submits_once, unlock_by_tab_and_enter_submits_once}`;
`desktop::view::passphrase::tests::{submit_supplies_the_passphrase_and_releases_the_input,
cancel_replies_cancelled}`;
`desktop::view::operations::outcomes::route_tests::{unlock_clears_the_declined_key_and_retries_the_same_request,
share_after_a_finished_locked_key_clears_its_declined_key}`;
`desktop::services::scheduler::attention_tests::unlock_after_cancelled_prompt`;
the wrong-passphrase pin is `tests/ssh_transport/session.rs:183-185`.
`every_item_view_icon_loads` is `desktop::assets::tests::every_item_view_icon_loads`
(`D/assets_tests.rs:65-74`).

**U-8:** no read double can fail a read (`DelayingBackend` "only delays",
`delaying.rs:4-12`; busy is not failure). P18's failed-read row needs the
design's fallback: a `#[cfg(all(test, feature = "desktop-test-support"))]`
hook on the Unlock dialog applying a failed `ReadResult` to its key card
(model tests build `ReadResult::Failed(ReadFailure { .. })`, for example
`D/model/navigation_tests.rs:45`).

**Wave Amendment 1** (`5b7771db`): traceability row split into 05a and 05b;
"Seven Cycles"; 05a and 05b sections and rows (05a depends on 01, 04; 05b on
01, 05a); Cycle 04's 2026-11-13 note kept word for word; the D5 risk row
names 05b; the split line marked used. Two choices of the implementer's own,
for the owner's eye: decision 4 added to the Unlock bullet, and (fix round 1)
05b's planned document named as its ticket does,
`docs/Cycles/wave-04-cycle-05b-close-and-resolve.md` — the implementer first
invented a different name and reported that the ticket gave none; the
reviewer found it and the fix is `42f807d5`.

**E-list sweep** (commands 1 to 3 over `src/desktop`: 267, 88 and 140
hits): every site is an E-n above or in the plan; no further numbers than
E-17a to E-17c. Sites needing nothing: the `create-*` ids of the New ticket
and New document dialog; `item/edit_restored_tests.rs:824-878`,
`item/edit_tests.rs:1405-1426`, `settings/hosts_tests.rs:564-569`,
`lifecycle/credential_tests.rs:192`, `operations/outcomes_route_tests.rs:227,242`.
Assertion removal inside E-9: `passphrase_tests.rs:169` asserts
`label_for("primary") == "Passphrase for primary"` and goes with `label_for`.
`Followed::Confirmed` appears only in production code
(`confirm.rs:472`, `item/repair.rs:321` = E-16, `operations/republish.rs:96`
uses `{ request, .. }`); no test builds or matches it. E-15 and E-16 found no
test.

**Production sites no task names**, by the task that breaks them:

- Task 3: `settings/repositories/manage.rs:392-396` `add_path_focus` (reads
  `Form` inputs; delegate, Task 4 removes); `settings/arrive.rs:66-75` calls
  `add_path_focus` and `inspect_root` (delegate); `settings/mod.rs:279-281`
  `SettingsHost::hide` calls `forget_pending_submit` (delegate; Task 4's
  `add_closed` owns the pending Add); `repositories.rs:336-344` accessors
  `input` and `control_focus` (Ruling 9); `OpenFlag::set` is private
  (`mod.rs:92-112`); stale docs at `repositories.rs:1-39`,
  `settings/mod.rs:270-272`, `settings_sheet.rs:226-232`.
- Task 4: `settings/chosen.rs:25-43` and `:59-60` (`chosen_root`,
  `choose_on_open` match `AddRepository` and `Inspect`; R-5 and R-6 break
  them); `workspace/render.rs:174-182` and `workspace/focus.rs:285-290`
  (R-46's deferral and `focus_lost`; where G18 and G19 extend, R-24);
  `content.rs:131-137` and `:595-613` (R-34); the private helpers
  `take_layer_origin`, `origin_region`, `SettingsReturn::recorded`
  (`settings_sheet.rs`, listed); `item/repair.rs:321` (E-16).
- Task 5: `wrapped_kit` reachable from `add_repository*` (Ruling 9).

**Callers of the `settings()` helper** (`repositories_tests.rs:40-46`, Ctrl+,
then `settings-repositories`): 16 sites in `repositories_tests.rs` (`:220`,
`:266`, `:315`, `:345`, `:388`, `:420`, `:445`, `:472`, `:500`, `:536`,
`:612`, `:658`, `:727` via `checked_add` from `:741` and `:765`, `:794`,
`:826`), all moving or changing under E-1, E-1e, E-2; 7 in
`repositories_manage_tests.rs` (`:140`, `:179`, `:252`, `:296`, `:322`,
`:348`, `:647`), which stay. `the_inspect_recovery_opens_the_page_with_its_root_checked`
(`:699`) opens with `Shell::open`.

**Cycle 04's helpers that open Settings on Repositories:**
`settings_sheet_tests/support.rs` `tab_to_create` (`:316-327`; callers
`settings_sheet_tests.rs:171`, `:577`, `inside.rs:88`: affected) and
`open_on_keys` (`:222-240`; unaffected; P6 can reuse it with `emptied`
`:242-246`); `inside.rs` `focus_add_path` (`:57-67`; callers `:146`, `:164`:
affected); `routes.rs` `raise` (`:111-118`; `fields` `:416`, `:428`:
affected); `late.rs` `pending_add` (`:33-102`: affected);
`keyboard_tests/settings.rs` `open_page` (`:23-31`; E-5 `:58` affected);
`chosen_tests.rs:193` (E-17c); `harness.rs` `Shell::holds_focus` (`:393`) and
`holds_control` (use it for G4's Kit button).

**Module runs per task** (the orchestrator runs each once, at the end of the
task, by module path without `-- --exact`):

- Task 1: `desktop::assets::tests`, `desktop::view::frame::tests`,
  `desktop::view::confirm::tests`, `desktop::view::keyboard_tests::confirm`,
  `desktop::view::editing_privacy_tests`,
  `desktop::view::polling_privacy_tests`.
- Task 2: `desktop::view::frame::tests`; each dialog's module
  (`create::tests`, `new_folder::tests`, `move_document::tests`,
  `tabs::tests`, `quit::tests`, `operations::retire_tests`,
  `drafts::actions_tests`, `drafts::discard_tests`, `drafts::privacy_tests`,
  `item::draft_actions::tests`, `item::repair::tests`,
  `item::composer::states_tests`, `item::edit_tests`) all under
  `desktop::view::`; `desktop::view::item::sheet_tests`;
  `desktop::view::keyboard_tests::{frame, drafts, editing, discussion}`;
  `desktop::view::{editing_privacy_tests, lifecycle::privacy_tests,
  polling_privacy_tests, lifecycle::lock_gates_tests, workspace::tests}`;
  `desktop::services::drafts::create::tests`; Cycle 04's
  `desktop::view::workspace::settings_sheet::tests` and
  `desktop::view::settings::tests`.
- Task 3: `desktop::view::add_repository::tests`,
  `desktop::view::settings::repositories::{tests, manage_tests, copy_tests}`,
  `desktop::view::keyboard_tests::settings`,
  `desktop::view::workspace::settings_sheet::tests`,
  `desktop::view::settings::chosen_tests`.
- Task 4: `desktop::view::workspace::add_sheet::tests`,
  `desktop::view::add_repository::tests`,
  `desktop::view::settings::repositories`, `desktop::view::repositories`,
  `desktop::view::dock::tests::toolbars`,
  `desktop::view::lifecycle::outcome_canned_tests`,
  `desktop::view::keyboard_tests::settings`,
  `desktop::view::workspace::settings_sheet::tests`,
  `desktop::view::settings::chosen_tests`, `desktop::view::confirm::tests`,
  `desktop::view::item::repair::tests`,
  `desktop::view::operations::republish::tests`,
  `desktop::view::workspace::tests`, `desktop::view::quit::tests`,
  `desktop::view::item::edit_tests::quit`, `desktop::view::activity::tests`,
  `desktop::view::needs_attention::tests`.
- Task 5: `desktop::view::add_repository::tests`,
  `desktop::view::settings::repositories::copy_tests`.
- Task 8: adds `desktop::view::polling_privacy_tests`.

Concerns left for the owner's eye, not blocking: Part F's `KEEP_LOCAL`
"Keep on this computer" and `SSH_ONLY` word the sharing choice differently
from Cycle 04's Settings copy (`SELECT_NONE` "Keep changes on this
computer", `NOT_ELIGIBLE`).

**Review and fix round.** Task review: spec not met on one Important, quality
approved; no contract changed under the re-map, and about 30 citations were
spot-checked against the code (all right, or off by one or two lines). The
Important: the Wave's 05b planned-document path was invented and contradicted
the 05b ticket (`wave-04-cycle-05b-close-and-resolve.md`). Fix round 1 (1
addressed, 0 open; commit `42f807d5`), scoped re-review: addressed.

**Parked Minors (Task 0)**, for the final review:

- The Wave says the 05b Cycle document "records it element by element"; the
  document does not exist yet ("will record").
- The Wave's risk row "Cycle 01 or 05 grows too large to review | The split
  line in each" is stale: 05's split line was used.
- The design's header note and Baseline Facts intro omit facts 15 and 28 from
  the re-lined list.
- A3's hand-drawn rings are now two (Quit now is not a dialog's), so Task
  10's on-screen checklist names two, not three. B4 says "own open flag" and
  "`ContentScroll` region" where R-8 and B12 still say "own scroll handle".
- For the owner: both tickets link
  `wave-04-workspace-design.md#cycle-05-existing-sheets-and-dialogs`, an
  anchor that no longer exists since the heading split. Tickets are canonical
  files and are not edited here.

Ruling 11: Task 1 keeps `Tone::Danger` and `Tone::Success` with
`#[allow(dead_code)] // Used from Cycle 05b`, an exception to the plan's "Used
from Task N" marker — A1's interface lists all three tones, no task of this
Cycle uses the other two, and Cycle 05b's two surfaces will; Cycle 05b removes
the allow — cost if wrong: two variants deleted.

Ruling 12: Task 1's extra `hover_strong` token and the secondary button's
hover and press colours (`button.hover.background`, `button.active.background`
= `07`'s `hover-strong`, `#353a41` dark, `#e2dfda` light) stay — `07` defines
the token with these values, and without it the dark-mode hover gets darker
than the resting colour — but they go to the owner's eye with decision 6: they
change every Kit Default-variant button app-wide (two lines and the token to
drop) — cost if wrong: remove them.

## Task 1 (The Frame And Theme)

Base `0ac6af37`; commit `ded7ca46`, `feat(desktop): common dialog frame and
theme slots; Confirmation on the frame`. Nine files under `src/desktop/`
(`frame.rs`, `frame/theme.rs`, `frame_tests.rs` new; `confirm.rs`, `assets.rs`,
`assets_tests.rs`, `mod.rs`, `view/mod.rs`, `shell/tokens.rs`). The boundary
diff prints nothing; `Cargo.lock` did not move. Gates (`cargo fmt --check`,
`cargo check --locked --features desktop`, `cargo clippy --all-targets
--all-features --locked -- -D warnings`) pass.

**Tests run, by exact name (1 test each):**

- P1 `desktop::view::frame::tests::every_dialog_keeps_its_keys_and_focus`
  (Confirmation row): green on today's code, green after.
- P2 `desktop::view::frame::tests::footer_buttons_tab_in_drawn_order`
  (Confirmation row): red on today's code (Tab reaches `confirm-ok` before
  `confirm-cancel`), green after.
- `desktop::view::frame::theme::tests::applying_the_theme_keeps_its_mode`:
  red against a naive apply order (Light flipped to Dark), green with the
  current mode last.
- E-14 `desktop::assets::tests::every_item_view_icon_loads`: red with the five
  names added first (`folder-input errored`), green after the four
  `icon_assets!` additions.

**Module run, once, by the orchestrator:** `desktop::assets::tests`,
`desktop::view::frame::tests`, `desktop::view::confirm::tests`,
`desktop::view::keyboard_tests::confirm`,
`desktop::view::editing_privacy_tests`,
`desktop::view::polling_privacy_tests` — 23 passed, 0 failed.

**U-6:** all five icons load. **U-7:** the secondary button has keys
(`button.background`, with `input.border` for the border); the input
background has none (the Kit computes it from `input.border`, so the new
`border_strong` shifts dark inputs: owner's eye); the sheet border and shadow
have none (global `border`, fixed `shadow_xl`); those slots stay the Kit's
(K-13).

**Existing tests edited:** E-14 only (`ICONS` 53 to 58 names, `key-round`
included; no assertion lost). `keyboard_tests/confirm.rs` loops with `tab_to`,
so E-10 needed no edit.

**Review** (sonnet): spec compliant, quality approved, five Minors, no loop.
Parked for the final review:

- The `Tone` markers (Ruling 11).
- The `hover_strong` addition (Ruling 12).
- The Confirmation header's title has a new id, `confirm-title`: no
  `confirm-` id predates it, so R-4 holds, but the `polling_privacy_tests`
  prefix sweep (`confirm-`, `:199`) now also reads it (static `intent.label()`
  text, which `confirm-command` already carries).
- The body's top padding is 10, not 18, to make up the Kit's fixed 8 px gap
  under the title; on-screen check.
- `frame::body` takes one `content` child, so its 18 px gap does nothing
  today; later tasks that pass one inner `v_flex` own the inner gap.
- Not run until Task 10's full tier: tests in modules the Task 1 run does not
  name that click `confirm-ok` or read `confirm-*` labels
  (`operations_tests`, `passphrase_tests`, `repositories_tests`). Task 0's
  sweep found no Tab count or button-order assertion on them.

Ruling 13: Task 2's P1 per-dialog set-ups live in a submodule directory
`D/view/frame_tests/` (`dialogs.rs`, `forms.rs`, `item_dialogs.rs`), while the
test function and its table stay in `frame_tests.rs` — `frame_tests.rs` was
already 285 lines and the 12 dialogs would pass the ~500-line limit — cost if
wrong: a file split reversed.

## Task 2 (Every Other Dialog On The Frame)

Base `d9533dd9`; code `b2618100`, `feat(desktop): every other dialog and both
sheet headers on the frame`; fix `31497e0e`, `fix(desktop): give the New item
header its own id`. 19 files under `src/desktop/`. Boundary diff empty;
`Cargo.lock` unchanged. Gates pass (fmt, check, clippy).

**Moved onto the frame** (header in the Kit's title slot; body and footer in
the dialog's own view, the footer the body's last child, R-19; today's button
order kept; hand-drawn rings travel with their buttons): New ticket and New
document (`create.rs`, `create/form.rs`), New folder, Move, Close a tab with a
draft, the quit asks, the retire asks, Discard a draft, the repair discard,
the composer discard, Recovery off, Replace your edits (`review-replace-message`
into its header, id kept); the details sheet's header
(`frame::sheet_header`, `sheet-heading` and `sheet-description`, the
description out of the body) and Cycle 04's Settings header (`settings-heading`
and `settings-note` kept; R-20). The ask footers (quit, retire, close-draft,
discard) were left-aligned and are now right-aligned, per A3.

**Tests run:**

- P1 `desktop::view::frame::tests::every_dialog_keeps_its_keys_and_focus` (96
  rows over the 12 dialogs, table-driven in one test): passed on today's code
  before any production change, passed after, and passed again after the fix.
- Orchestrator's module run, once, 27 filters (every filter matched at least
  one test): `desktop::view::{frame::tests, create::tests, new_folder::tests,
  move_document::tests, tabs::tests, quit::tests, operations::retire_tests,
  drafts::actions_tests, drafts::discard_tests, drafts::privacy_tests,
  item::draft_actions::tests, item::repair::tests,
  item::composer::states_tests, item::edit_tests, item::sheet_tests,
  keyboard_tests::{frame, drafts, editing, discussion}, editing_privacy_tests,
  lifecycle::privacy_tests, polling_privacy_tests, lifecycle::lock_gates_tests,
  workspace::tests, workspace::settings_sheet::tests, settings::tests}`,
  `desktop::services::drafts::create::tests` — **494 passed, 0 failed** (500 s).
- After the fix, `desktop::view::create::tests` and
  `desktop::view::frame::tests` again: 21 passed, 0 failed.

**Existing tests edited:** none (so no E-17 onward from this task; E-11's
files needed no edit).

**Review** (sonnet): spec compliant, quality approved, one Important, fixed in
round 1 (1 addressed, 0 open; scoped re-review: addressed): the New item
header's id `create-heading` collided with the Repositories page's heading
(`settings/repositories.rs:778`); it is now `create-item-heading`. Parked
Minors, for the final review:

- Every dialog title has a new element id (`Header.ids.0` is required); only
  `review-replace-message` kept an old one. Ids are fixed constants, so no
  private text reaches a header. `quit-ask-title` now falls under the `quit-`
  sweep surface and `retire-ask-title` under neither retire surface.
- The composer discard with no owner note draws an empty body (about 28 px of
  padding between header and footer); `.children(note.map(..))` would avoid
  it. Owner's eye.
- `close_draft.rs` is 507 lines (483 before); the guideline covers new files.
- `settings/frame.rs:3`'s module doc line is about 122 characters; reflow.
- P1 never asserts footer position or order (by rule, no appearance tests); a
  footer drawn outside the body would fail only through a keyed-close row. P1
  runs about ten windows in one `TestAppContext` (about 35 s): the first
  suspect if it ever flakes.
