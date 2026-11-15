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
| 2 | done, review approved after fix round 1 (Minors parked) | code `b2618100` (base `d9533dd9`), fix `31497e0e`; ledger `92f3ba73` | yes |
| 3 | done, review approved first time (Minors parked) | code `5f8c10b7` (base `92f3ba73`; trailer amended before publishing); ledger `2c398b03` | yes |
| 4 | done, design checkpoint stopped for the owner (Rulings 15 to 18), review approved first time (Minors parked) | code `0d5cae5c` (base `2c398b03`); ledger `4bab2795`; merge of main `9059778f`; ledger `f4059706`; ticket comment `315ae043` | yes |
| 5 | done, review approved after fix round 1 (Minors parked) | code `1ff07fe3` (base `315ae043`), fix `0d14abe6`; ledger `80a39f6a` | yes |
| 6 | done, review approved first time (Minors parked) | code `cb78fe3d` (base `80a39f6a`); ledger `d7b39f62` | yes |
| 7 | done, review approved first time (Minors parked) | code `a14e862d` (base `d7b39f62`); ledger `4fb4c234`; ticket comment `08c7752e` | yes |
| 8 | done, review approved first time (Minors parked); a Task 7 test hang found at its boundary (below) | code `6d7734d1` (base `08c7752e`) | with the ledger commit |

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
- Cycle 04's branch was still moving. Its tip was merged again at `edb0e920`
  (`5bacd7d2`). **Cycle 04 then landed on main** (owner heads-up, 2026-11-15):
  see Ruling 20.

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

Ruling 14: Task 3's `AddRepositorySheet` takes Settings' own `ContentScroll`
and `OpenFlag` through its constructor
(`new(services, scroll, open, window, cx)`), not "its own `ScrollHandle`" as
the plan's Step 1 says — a scroll nothing draws would break "no visible
change", the below-the-fold test and Cycle 04's P8; Task 4 gives the sheet its
own scroll and flag through the same constructor, and this is a hand-off item,
not an omission — cost if wrong: Task 4 does it anyway.

## Task 3 (Extract The Add Forms' State)

Base `92f3ba73`; code `5f8c10b7`, `refactor(desktop): move the Add repository
forms into their own entity` (the implementer wrote an Opus 5.5 trailer against
Ruling 5; the orchestrator corrected the trailer before anything was
published, so `650f1889` became `5f8c10b7`). 11 files under `src/desktop/`.
Boundary diff empty; `Cargo.lock` unchanged. Gates pass.

**What moved:** the forms and all their state leave `RepositoriesPage` for
`AddRepositorySheet` in `D/view/add_repository.rs` (475 lines),
`add_repository/checks.rs` (195) and `add_repository/render.rs` (177). Its
reads are stamped under `ViewScope::AddRepository` with generations of its own
(R-9). `repositories.rs` went from 830 to 257 lines and holds an
`Entity<AddRepositorySheet>` drawn where the forms were. The page keeps the
managed section, its helpers, `NAME` and `EMAIL`. Review found the 17 tests'
bodies and the production move identical except for the receiver, module paths
and the scope line.

**Test functions, before and after.** `repositories_tests.rs`: 17 before, 0
after (only the helpers the manage tests import remain: `settings`, `page`,
`input`, `value`, `set`, `registered`, `finished`, `control_focus`).
`add_repository_tests.rs` (new): 17, the same names in the same order, each
byte-identical to its original: `create_in_an_empty_directory_previews_nothing_then_creates_and_refreshes`,
`adding_a_repository_without_identity_asks_for_one_and_keeps_it`,
`a_repository_with_an_identity_shows_no_identity_fields`,
`create_in_a_non_empty_directory_shows_its_typed_failure`,
`adding_a_folder_that_is_not_a_repository_shows_its_typed_failure`,
`submitting_with_no_folder_says_so`, `escape_on_the_preview_makes_nothing`,
`a_create_whose_directory_changed_since_its_preview_offers_prepare_again`,
`pressing_confirm_twice_creates_one_repository`,
`a_partial_create_is_completed_by_retry_under_the_same_request`,
`browse_fills_the_field_and_a_dismissal_leaves_it_unchanged`,
`a_form_control_below_the_fold_is_scrolled_into_view`,
`the_inspect_recovery_opens_the_page_with_its_root_checked`,
`escape_on_an_add_preview_makes_nothing`,
`an_add_whose_repository_was_enabled_since_its_preview_offers_prepare_again`,
`add_pressed_at_once_checks_first_and_asks_for_the_identity`,
`add_pressed_at_once_on_a_repository_with_an_identity_opens_the_preview`.
`copy_tests.rs`: 1 before, 1 after (`settings_copy_uses_the_vocabulary`).
`repositories_manage_tests.rs`: untouched, 20 tests.

**Tests run:** by exact name, the 17 moved tests plus the copy test, 18 of 18
passed; the 8 Cycle 04 tests that use the delegates, 8 of 8 passed
(implementer). Orchestrator's module run, once, 7 filters:
`desktop::view::add_repository::tests` (17),
`desktop::view::settings::repositories::tests` (0, helpers only),
`::manage_tests` (20), `::copy_tests` (1),
`desktop::view::keyboard_tests::settings` (6),
`desktop::view::workspace::settings_sheet::tests` (14),
`desktop::view::settings::chosen_tests` (3) — **61 passed, 0 failed**.

**Existing tests edited:** E-1 (the 17 moved, unchanged) and E-17a (the copy
table's 20 Add and Create rows re-pointed to `add_repository`, plus `NAME` and
`EMAIL` kept: 22 rows, none removed). No assertion removed; no Cycle 04 test
edited. Known coverage gap: P18's source scan covers only `view/settings/`, so
the 20 moved constants leave its "declared but missing from the table" check
until P20 (Task 9); their rows still check the vocabulary.

**Delegates for Task 4 to remove** (`D/view/settings/repositories.rs`):
`add_path_focus` (`:156`, called from `settings/arrive.rs:66-75`),
`inspect_root` (`:161`, `arrive.rs:70-75`), `forget_pending_submit` (`:168`,
`settings/mod.rs:279-281`; its signature gained `cx`), the test-only `input`
and `control_focus` (`:120`, `:131`), the test-only `add_controls` field
(`:59`, filled `:101`) with `add_repository::Controls` and
`AddRepositorySheet::controls()` (`add_repository.rs:205`, `:283`), and the
test-only `RepositoriesPage::add_sheet()` (`:141`). Task 4 points
`add_repository_tests.rs`'s `sheet` helper at `Workspace::add_sheet()` and
removes the accessor.

**Review** (sonnet): spec compliant, quality approved, no Important. Parked
Minors, for Task 4 and the final review:

- The sheet depends on Settings internals (`OpenFlag`, `wrapped`,
  `field_column`, `NAME`, `EMAIL`): Task 4 replaces `OpenFlag` with the
  sheet's own flag and decides where `wrapped` and `field_column` live
  (Ruling 14).
- `text`, `field`, `button` and `heading` now exist twice (page and sheet);
  the page's stay for `manage.rs`. Staged duplication.
- `add_repository_tests.rs` is 829 lines: a mechanical move of an 832-line
  test file, no submodule in the File Map. Not split.
- `settings::repositories::tests` holds helpers only, so its module filter
  matches zero tests.
- The sheet is an extra view node wrapping the forms in a flex column, with
  the inner `gap_4` reproducing the old spacing: no test covers appearance;
  the owner's on-screen check settles it.

## Task 4 design checkpoint, 2026-11-15

Phase 1 wrote P3, P4, P5, P6, P7, P8, P15 and P23 over B9's rows G1 to G15,
G17 to G19, G21 and G23 (G16 and G22 are Task 5's, G20 Task 6's) before any
production code. Every row failed first (RED) for the right reason. Four rows
were answered differently by the merged code (B-1 to B-4) and one sub-case was
ambiguous (B-5). The orchestrator stopped, as the handoff requires, and
brought them to the owner, who answered each with the recommended option.

Ruling 15 (owner, B-1): `show_add_repository` records the panel's selection as
Settings' `seen` through a new `SettingsHost` method (R-55's "records the
selection"), so E-17c's "footer after Inspect(b)" rows stay on `a` as Ruling 7
says and Cycle 04's P10 keeps its assertion; R-14's rule in `chosen.rs` is
untouched — cost if wrong: the footer rows expect `b`. Realised as
`SettingsHost::saw_panel(panel)` (`settings/chosen.rs`), called on every open
of the Add sheet with any route (see Minor 5 below).

Ruling 16 (owner, B-2 and B-4): "from Settings" is carried separately from the
origin handle (`from_settings: Option<Option<WeakFocusHandle>>`, set from
`take_origin()` before `settings_closed(true)`), so G4 holds when Settings
recorded no origin, and G6 then goes to the content region (Cycle 04's F10);
B2 step 2's reset runs only when the sheet is not already open, so an in-place
route keeps the return and the origin — cost if wrong: the two G4 edge rows
deleted.

Ruling 17 (owner, B-3): the whole `add_closed` (steps 0 to 4) waits while a
dialog is open, the owner and the open flag kept until it runs, so G23
"Unchanged" and R-25 hold; **B3's text is corrected**: step 0 and step 1 wait
too, not only step 3. Realised as a `waiting` close that `sync_sheet` keeps and
`run_waiting_add_close` runs, deferred, from the render once no dialog is open;
a quiet close clears it — cost if wrong: focus moves behind an Unlock prompt.

Ruling 18 (owner, B-5): while the add is pending ("Adding…", disabled) and its
confirmation closes, focus goes to the button's own handle (the wrapper: no
ring, no Tab stop while disabled), and sits on the button, with no move, once
it is enabled, as Cycle 04's Ruling 54 did for Check host. P5 gained a row,
`while_adding`.

Ruling 19 (orchestrator, the checkpoint's other concerns): C-1 follow R-29
(`registered_root: Option<PathBuf>` on `DesktopRequest`, set from the Finished
envelope's scope; E-17d); C-2 a Known Limit (below); C-3 `wrapped_kit` is
reachable from the Add sheet now, a visibility change inside `src/desktop`;
C-5 typed text is cleared only for the same open, never for a later open
number; C-6 `show_settings` reads `add_return_to_settings` before its
`add_closed(Quiet)`; C-4, C-7, G21's button press and G1's mode are Task 5's.

**K-16 (Known Limit, Ruling 19 on C-2).** A confirmation stacked under an
Unlock prompt cannot close itself: `ConfirmDialog::close` calls
`window.close_dialog` (`confirm.rs:565-569`) and the Kit closes the topmost
dialog (Kit `root.rs:325-339`), so an add's finish that lands while an Unlock
prompt is open above its still-open confirmation closes the prompt instead.
This predates the Cycle and affects every confirmation. G23's rows close the
confirmation first (Escape while the add runs). No library change, no ticket.

**A fact for the design (found in phase 2).** The Kit can run a closed sheet's
close again within the same Escape keystroke (the Sheet host's `Cancel` handler
calls `cx.propagate()` and then `close()`; seen two to six times in three P6
rows). Each repeat runs `window.close_sheet`, which closes whatever sheet is
active then, so Settings, opened from the Add sheet's `on_close`, was closed
too. Task 0's "the Kit's close order leaves Root's slot free when `on_close`
opens Settings" holds for the first close only. Fix: `add_closed` opens
Settings, for a return or for Open its settings, with `cx.defer_in`, after the
event; a repeated close then finds no sheet and no owner and does nothing.
Cycle 04 never met this because its `on_close` opens nothing. B3 and Task 0's
note should say so (Task 10's amendments).

## Task 4 (The Add Sheet)

Base `2c398b03`; code `0d5cae5c`, `feat(desktop): the Add repository sheet, its
ways in and its focus`, one commit with the phase-1 tests. 64 files under
`src/desktop/`. Boundary diff empty; `Cargo.lock` unchanged. Gates pass.

**What was built:** `SheetOwner::AddRepository` and `SheetOwnerKind`;
`sync_sheet`'s three cases; `workspace/add_sheet.rs` (501 lines: `AddRoute`,
`AddClose`, `PendingAdd`, `show_add_repository`, `add_closed`, `add_sheet()`);
the sheet in a 470 px Kit `Sheet` with its own scroll and open flag (Ruling
14) and `frame::sheet_header`/`frame::sheet_footer`; the quit paths close it
quietly; `show_settings` closes it quietly first (R-22); the Ctrl+, handler
refuses while it is open (R-14); R-34 and R-46 extended (G18, G19); ways in
AW-1 to AW-5; `SettingsRoute::Inspect` and `inspect_in_settings` removed
(R-5), `SettingsRoute::AddRepository` renamed `AddButton` (R-6) with
`settings-add-repository` a `wrapped_kit` button; `Followed::Confirmed` gains
`attempt` (R-28, E-16 at `item/repair.rs`); `DesktopRequest.registered_root`
(R-29); `pending_adds` keyed by attempt and open number (R-26), handled with
`defer_in` (R-25), "Adding…" while pending (R-27); `PendingAdd::share` stays
`None` (`#[allow(dead_code)] // Used from Task 7`, and `AddClose::ToSettings`
`// Used from Task 5`). Task 3's delegates are gone.

**Tests run:**

- Implementer, by exact name, final tree: 35 of 35 pass (the 8 Add-sheet
  tests, the 17 moved in Task 3, the 10 edited Cycle 04 tests); a mutation
  check failed G23, G19 and P15's Ctrl+, row as expected, and was reverted.
- Orchestrator's module run, once, 17 filters (all matched at least one test):
  `desktop::view::{workspace::add_sheet::tests (8), add_repository::tests (17),
  settings::repositories (21), repositories (5), dock::tests::toolbars (1),
  lifecycle::outcome_canned_tests (5), keyboard_tests::settings (6),
  workspace::settings_sheet::tests (14), settings::chosen_tests (3),
  confirm::tests (10), item::repair::tests (25), operations::republish::tests
  (8), workspace::tests (29), quit::tests (7), item::edit_tests::quit (11),
  activity::tests (16), needs_attention::tests (12)}` — **198 passed, 0
  failed** (219 s).

**Existing tests edited. No `#[test]` function was removed or renamed.**
Assertion changes and removals are listed here and go into the review-ready
summary:

- E-1 (Task 4's part): `settings()` opens the sheet by clicking
  `add-repository-button` (AW-2); `sheet()` reads `Workspace::add_sheet()`.
  No assertion removed.
- E-1b: additions only (the sheet closed; after reopening `add-path` is empty;
  re-check shows `ALREADY_ADDED`).
- E-1e `a_form_control_below_the_fold_is_scrolled_into_view`: **assertion
  changed**, as named: reads `add-scroll` and `create-prefix` (was
  `settings-scroll` and `create-submit`) through the sheet's
  `scroll_offset()`.
- E-4 `the_inspect_recovery_opens_the_page_with_its_root_checked`: calls
  `show_add_repository(AddRoute::Inspect)` and asserts `add-sheet` is present
  (was `settings-repositories`).
- E-5 `add_an_existing_repository_by_keyboard`: **assertion changed**: the last
  step expects `+` in Repositories (was `walk.region(Content)`).
- E-6, E-7, E-8 (`rows_and_list_states`, `panel_toolbars_work_by_tab_and_enter`,
  `each_way_out_offers_its_button`): **assertion changed**:
  `Shown::Settings(Repositories)` becomes `Shown::Tabs` plus `add-sheet`
  present; `holds_focus("add-path")` stays.
- E-12, Cycle 04's P2 `settings_leaves_the_item_as_it_was`: selects `a` first
  so the Repositories page is again taller than the short window (without the
  forms it no longer scrolled), then raises `SettingsRoute::AddButton`;
  **assertion changed**: reads `settings-add-repository` (was `add-path`).
- E-12, P8 `settings_scrolls_on_its_own`: selects `a` first; End is pressed
  from `identity-submit` (was `add-browse`); the below-the-fold control is the
  add button (was `create-submit`); `scrolls::rows` (a) and (b) unchanged.
- E-12, P4 `routes.rs`: "F1 AddRepository: the Add folder" becomes "F1
  AddButton", read on the Kit button; **assertion removed:** "F1 Inspect(root),
  the panel's selection changed: the choice stays (R-55)": the route is gone
  (R-5); asserted through the Add sheet by this Cycle's P4 E-17c rows
  (`ways.rs:357-376`).
- E-12, P22 `late.rs`: **assertion removed:** the `pending_add` row (F17 with
  R-48) and its function; moved to this Cycle's P8 (G12 `late::pending_check`),
  which asserts the same facts, `checked() == plain` stronger than
  `present("add-inspection")` (reviewer-verified: all six).
- `support.rs`: `tab_to_create` becomes `tab_to_add_button` (new `ADD_BUTTON`
  constant).
- E-17b `inside.rs`: `tab_trap`'s last Tab stop is `settings-add-repository`
  (was `create-submit`); `landings` (F18) and `context_menu` (F19) use
  `identity-name` (was `add-path`); no assertion removed.
- E-17c `chosen_tests::one_repository_is_chosen`: `route(Inspect(b))` becomes
  `show_add_repository(AddRoute::Inspect(b))`, Escape, then the footer open;
  **assertion removed:** "Inspect(b), panel now b: chosen" (P4's E-17c rows
  assert it); the typed-text fact and both "footer after Inspect(b)" rows
  stay and pass (Ruling 15).
- E-15: no edit. E-16: production only (`item/repair.rs:321` matches
  `{ request, details, .. }`).
- E-17d: `registered_root: None` added to 22 `DesktopRequest` literals in 21
  test files by script, plus two production literals in `services/worker.rs`
  by hand; the script's first pattern also hit `-> DesktopRequest {` signatures,
  so those 24 files were restored from git and it was rerun with signatures
  excluded. The diff adds only `registered_root: None` lines (reviewer-verified)
  and the files' test-function counts are the same before and after:
  `activity_tests/states_retire` 0, `activity_tests/support` 0,
  `operations/outcomes_tests` 2, `operations_tests` 32, `resends_tests` 8,
  `scheduler/reconfigure` 3, `scheduler_loop_tests` 16,
  `scheduler_retire_tests` 17, `worker_tests` 36, `activity_tests/cases` 0,
  `activity_tests/clear` 1, `activity_tests/list_support` 0,
  `conflict/entry_tests` 7, `footer_tests/support` 0,
  `item/edit_release_tests` 30, `item/share_tests` 16,
  `lifecycle/outcome_canned_tests` 5, `operations/check_tests` 1,
  `operations/outcomes_tests` 13, `operations/republish_tests` 8,
  `settings/keys_tests/forms` 2. `worker_tests` also gains
  `#[allow(clippy::large_enum_variant)]` on its test enum `Seen`, as
  `model/documents.rs` and `model/drafts/rows.rs` already allow it.

**Review** (opus): spec compliant, quality approved, no Critical or Important;
no loop. Parked Minors, for Task 5 and the final review:

1. `add_sheet.rs:190-194`: when the owner is `AddRepository` but the Kit
   already closed the sheet without telling the workspace, `show_add_repository`
   runs `add_closed(Kept)` where Cycle 04's equivalent closes quietly; `Quiet`
   is safer. No current path reaches it.
2. `add_sheet.rs:284-351`: `add_closed` is not idempotent by itself (every
   caller guards on the owner or the open flag, except `AddEvent::Cancel`,
   unreachable in practice); an entry guard would make it so.
3. `add_sheet.rs:290-293` with `content.rs:290-296`: if the Kit's sheet were
   gone while a dialog is open, each render would re-enter `add_closed(Kept)`
   and spin; notify only when `waiting` changes. Nothing does this today.
4. **For Task 5 (pointer):** `add_sheet.rs:480-482`: a success that lands while
   a later open is showing does nothing at all. Ruling 19 C-5 limits only the
   clearing of text; B10 and B2 step 7 also say the kept check for that folder
   is forgotten "so its next open checks again". The reopened sheet keeps a
   stale "not registered" check for the folder just added; Add again
   re-previews and the library refuses. Untested (P8 `earlier_open` asserts
   only that the sheet stays open and focus is unchanged).
5. **Owner's eye:** Ruling 15 is realised unconditionally (every open of the
   Add sheet, `+` included, as the ruling's text reads); P4 and E-17c need it
   only for `Inspect`. Side effect: after `+` (AW-2), a panel selection changed
   since the last Settings open is consumed, so the next plain Settings open no
   longer follows it (in Cycle 04, `+` opened Settings, which chose the panel's
   selection). No test pins it either way; restricting it to `AddRoute::Inspect`
   is one line.
6. Test hygiene: the new tests copy Cycle 04's helpers (`Table`, `where_focus`,
   `settle`, `holds`, `holds_handle`, `in_context`, `hold_a_job`, the landings
   set-up), about 130 lines; one shared `pub(super)` support module would do.
7. `add_sheet.rs` is 501 lines, `add_repository.rs` 505 (was 475), `workspace.rs`
   507 (was 495); `show_add_repository` repeats `show_settings`' origin chain.
8. Nits: `button()`'s `primary` is always false (render.rs:84); `AddClose`'s
   `#[derive(Clone)]` is unused; `RepositoriesEvent::AddRepository` is forwarded
   from `chosen.rs:113`.
9. `ADDING` ("Adding…") is verbatim from B10 and R-27 but is not in Part F's
   table; P20 (Task 9) lists it.

Not covered until later: a Kit dialog closed by its Confirm key over the sheet
restores focus 0.25 s later through `defer_close_dialog`; if the sheet's
waiting close has run by then, that restore could land on the closed sheet's
submit handle. G23's rows close the prompt by Escape (immediate restore).
`adds_changed` on a Partial then Retry is covered by P14 (Task 7).

Ruling 20: Cycle 04 landed on main on 2026-11-15 (`origin/main` `14ca0938`,
"Integrate closed ticket 01M7AMAPNHHTNECXBF4CA5WPKV"). Its tip `5bacd7d2`,
already merged here, is an ancestor of main, and main differs from it only in
Cycle 04's `ticket.md` (status `done`): no `src`, `Cargo.lock`, tests or
packaging change, and Cycle 04's branch is deleted. At the Task 4 step
boundary (worktree clean) the main checkout was already at `14ca0938` and
`origin/main` was merged into this branch at `9059778f` (the merge changes only
that `ticket.md`; boundary diff empty; no test re-run needed). From now on the
merge source is `origin/main`, and Rulings 1 and 3's "Cycle 04's tip" mean
`origin/main`. The handoff's stop for "another Cycle that touches the Settings
sheet, the Workspace's sheet owner or the dialogs reaches main" does not fire:
the Cycle that landed is the one already merged. Task 7 still merges
`origin/main` — cost if wrong: a re-merge.

## Task 5 (Add An Existing Folder)

Base `315ae043`; code `1ff07fe3`, `feat(desktop): add an existing folder:
mode, checks, main branch, enablement`; fix `0d14abe6`, `test(desktop): pin the
new-mode reopen and replace a dead absence check`. 23 files under
`src/desktop/`. Boundary diff empty; `Cargo.lock` unchanged. Gates pass.

**What was built** (`D/` is `src/desktop/`): the mode control `add-mode`
(`add_repository/mode.rs`, Cycle 02's segments, R-10: two `Role::Tab`
segments, each a Tab stop with Enter and Space); B6's check states as rows
(`rows.rs`: container `add-checks`, each row `("add-check", ix)`, at most one
action as a `wrapped_kit` button: `add-open-settings`, `add-create-here`,
`add-choose`, `add-retry`); the main-branch select (`branch.rs`: the
checked-out branch first, Suggested or Held, the others Other with
`CHECK_OUT_FIRST`; `on_will_change` vetoes, `branch_confirmed` ignores a
`Confirm` naming another branch, and Add sends `allowed_branch`, never the
select's value); `submit.rs` (Add goes ahead only when `add_enabled`; pressed
before the check lands it checks first; `resume_submit` for G22); Part F's
constants in `copy.rs` plus two formatting fns the design left unnamed
(`other_main_branch`, `not_ssh_remote`); the prefix is not drawn or sent for a
Valid configuration (R-31); Browse… and its note line; `AddEvent::OpenSettings`
now calls `add_closed(ToSettings(root))` (its dead-code allow is removed);
`add_ended` calls `forget_check_of(&root)` for a later open (Task 4 Minor 4).
Production files: `add_repository.rs` shrank from 505 to 438, `add_sheet.rs` from
501 to 494; `configuration_label` was removed from `view/sidebar.rs` (its only
caller was the replaced "Configuration …" label).

**Tests written first, each seen failing:** P9 (B6's table; `Busy` through a
test accessor), P10 (with G16 and the no-commits row), P11 (B10's table and
G22), P21's Add rows, P22 (`#[cfg(unix)]`), in submodules under
`D/view/add_repository_tests/` (`checks.rs`, `branch.rs`, `enablement.rs`,
`browse.rs`, `support.rs`; none over 400 lines). `add_repository::tests`: 17 to
22 tests (the 17 of Task 3 plus `folder_checks_are_rows_with_their_actions`,
`the_main_branch_is_the_checked_out_one`, `add_is_enabled_only_when_it_can_go_ahead`,
`browse_says_why_beside_its_button`, `a_folder_that_is_not_text_is_named`); the
other edited test files keep their function lists (checked before and after).

**Tests run:**

- Implementer, by exact name: P9 to P11, P21, P22, the extended Task 4 rows,
  17 existing Add tests, the copy table, the keyboard walk and the 8 add_sheet
  tests, all passing; mutation checks showed P23 and the below-the-fold test
  still discriminate at their new window sizes.
- Orchestrator's module run, once, 4 filters (all matched at least one test):
  `desktop::view::add_repository::tests` (22),
  `desktop::view::workspace::add_sheet::tests` (8),
  `desktop::view::settings::repositories::copy_tests` (1),
  `desktop::view::keyboard_tests::settings` (6) — **37 passed, 0 failed**.
- After the fix, `add_repository::tests` and `add_sheet::tests` again: **30
  passed, 0 failed**.

**Named edits, with assertions removed (called out for the review-ready
summary):**

- E-1a `adding_a_folder_that_is_not_a_repository_shows_its_typed_failure`:
  **removed** the preview's `confirm-code` and `confirm-message` and
  `!present("confirm-ok")`, and `add-inspection`'s read message and
  `add-inspection-code`'s code (B6 draws "Not a Git repository." with no code
  shown); the code is now asserted from the library (`NotRepository`), with
  the rows `[NOT_REPOSITORY]`, `add-create-here` reading `CREATE_HERE`, Add
  disabled and no dialog on a press.
- E-1b: `value("add-branch")` becomes `chosen_branch(...) == Some("main")`; no
  assertion removed. E-1c: Add's part also asserts `!present("add-browse-note")`;
  Create's half switches to the new mode first, its
  `!present("browse-unavailable")` becoming `!present("create-browse-note")`.
  E-1d / E-4: the `NOT_ADDED` and configuration label assertions become B6's
  rows (`[IS_REPOSITORY, NOT_SET_UP]`, `[ALREADY_ADDED, ALREADY_SET_UP]`),
  stronger; no assertion removed.
- E-2 `submitting_with_no_folder_says_so`: **removed** Add's
  `add-message == NEED_FOLDER`; Add's half now asserts Add disabled, nothing
  drawn as `add-message` and no dialog after a press (that `add-message`
  absence check can never fail: parked Minor, below). Create's half still
  asserts `create-message == NEED_FOLDER` (Task 6's to change).
- E-17a (continued), P18's copy table: **removed** the rows
  `c!(add_repository, ADD_HEADING)`, `NOT_ADDED` and `CHECKED_OUT`, each for a
  deleted constant (B6: "the 'Checked out: {branch}' line goes; the branch
  shows in B7"); `PRIMARY_BRANCH` renamed `MAIN_BRANCH`; `CREATE_HEADING`,
  `CREATE_FOLDER`, `NEED_FOLDER`, `NEED_BRANCH` kept (Task 6's).

**Further edits (E-17 onward, decision 8):**

- E-17e: the Create tests open the new mode first (a helper `new_mode`;
  `fill_create` calls it), since B5 shows one form at a time. No assertion
  removed.
- E-17f: `a_form_control_below_the_fold_is_scrolled_into_view` switches to the
  new mode and its window goes from 360 to 250 px (with one mode drawn, Create's
  prefix was inside the viewport at 360). Still discriminates: the prefix
  starts below the fold and the offset goes below 0 once Tab reaches it.
- E-17g: `returns.rs::confirmation` (P5's G13 rows) chooses the new mode before
  its Create row and the existing mode again before "a finish that ends the add"
  (guarded with `present`). `returns.rs` is now 502 lines.
- E-17h: P23 `the_add_sheet_scrolls_on_its_own` (C-7): the window goes from 250
  to 150 px so the body still overflows in one mode; the mutation with the page
  keys unbound fails both "moves the sheet's body" rows.
- E-17i: G1 (`ways.rs`): `lands()` gains the existing-folder mode fact on every
  AW row, and `leave_in_the_new_mode` presses the new mode by Enter (after AW-2)
  or Space (before the in-place Inspect). Additions only.
- E-17j: G21 (`settings.rs::open_its_settings`) presses
  `("add-open-settings", 0)` after a real check instead of calling `add_closed`
  directly. Additions only. P7's `closes.rs::to_settings` still calls it
  directly.
- E-17k: Minor 4's pin, `late.rs::earlier_open`, gains the "check kept at the
  reopen" and "(B10, its check forgotten)" facts. Additions only.
- E-17l (fix round 1): `ways.rs` gains the row "B2 step 6 + after the new
  mode" after AW-2's reopen (a closed sheet left in the new mode, reopened by a
  `New` route, shows the existing folder and G1's focus on `add-path`; with
  `route` restricted to Inspect, P4 fails 2 of 45 rows, both the new one); and
  `add_pressed_at_once_checks_first_and_asks_for_the_identity`'s
  `!present("add-inspection")` (an id removed in this task, so it could never
  fail) is replaced by `!present("add-checks")` before the press, with
  `present("add-checks")` after it. An assertion replacement, not a removal.

**Review** (opus): spec compliant; quality first ❌ on two Important
test-evidence gaps (the B2 step 6 reopen never read the mode, and the dead
`add-inspection` absence check), both fixed in round 1 (2 addressed, 0 open;
scoped re-review: addressed). Parked Minors, for Task 6 and the final review:

- **For Task 6 (pointers):** (a) `mode.rs:43` / `submit.rs:143`: switching mode
  does not forget a pending Add: press Add while the check is in flight, switch
  to "Create a new repository", and the landed check still opens Add's
  confirmation or focuses a row action, then G13 focuses an undrawn handle:
  `set_mode` should clear `submit_pending`, or `resume_submit` should require
  `Mode::Existing`; (b) `browse.rs:75-90`: `chosen` no longer clears the form's
  message, so Create's "Enter the folder's path." stays after Browse… fills the
  folder (Task 6 deletes `NEED_FOLDER`); (c) "Create one here" is interim: it
  switches mode, fills today's `create-path` and focuses it; G20 and
  Name/Location are Task 6's; (d) `add_repository_tests.rs:478`'s
  `!present("add-message")` is a dead check.
- **Design gap for the owner:** `checks.rs:87` `not_ssh`: a Valid configuration
  whose publication remote is not among the folder's remotes draws no row and
  Add stays enabled; the library then refuses the preview (`InvalidRemote`,
  `bind/repo.rs:818`). C2's text ("disables Add for an ineligible one") and
  decision 9 ("the library refuses both … Add is disabled") differ.
- `browse.rs:36`: `browse_not_text` is cleared when the next Browse… opens,
  not "until the next choice"; P21 does not cover it.
- `add_repository.rs:407` with `submit.rs:334`: Enter in `add-prefix`,
  `add-name` or `add-email` calls `submit` directly; the `add_enabled` guard is
  all that stops a preview on a disabled state, and P11's disabled rows click a
  disabled button, so the keyboard path is untested.
- `copy.rs:60`, `:63`: `CHOOSE_ANOTHER` (opens the chooser) and `OPEN_SETTINGS`
  (opens the Settings sheet) have no `…`, against `06`'s rule; verbatim from
  Part F. P20 (Task 9) needs a ruling or an exception, as for `ADD` and
  `CREATE`.
- `checks.rs:186` `Inspection::NotText(String)` is only built empty (a unit
  variant would read better); `render.rs:195` `add-identity` has no
  `.test_support()`; `mode.rs` draws its own focus ring instead of the filter
  bar's `inset_ring`; `returns.rs` is 502 lines and `add_repository_tests.rs`
  929 (a Task 3 move, not new).
- A pressed Retry, Check again or Create one here disappears, so focus falls to
  Task 4's lost-focus handling (by reading, to `add-path`); no G row covers it;
  P16 (Task 9) should.
- Readings where B6's rows meet, for the owner's eye: a registered folder shows
  only its configuration rows and Open its settings (decision 18: "state and
  problems"); with no branch checked out only `NO_BRANCH` is drawn.
- P18's copy table has no rows for the new Part F constants (E-17a asks only
  for renames and removals); P20 (Task 9) covers Part F.

## Task 6 (Create A New Repository)

Base `80a39f6a`; code `cb78fe3d`, `feat(desktop): create a new repository:
name, location, target, enablement`. 14 files under `src/desktop/`. Boundary
diff empty; `Cargo.lock` unchanged. Gates pass.

**What was built:** `add_repository/create.rs` (new, 175 lines):
`create_target` (Location joined with Name by `Path::join`, as typed, when both
are filled), `create_enabled` (B10: not adding, Name, Location and Main branch
filled, trimmed), `create_intent` (`RepoCreate`, root = the target; `None`
unless enabled, so Enter in any Create field is guarded too), `create_here`
(B6/G20: the new mode, Location the parent, Name the last part, focus on
`create-folder-name`), `create_branch_field`, `create_target_line`
(`create-target`, check icon), `create_form` (Name, Location with Browse… and
its note, the target, Main branch, prefix, the identity block, `create-note`).
`Form::Create.path()` is now `create-location`; the `messages` map and every
`create-message`/`add-message`/`create-heading` are gone, so no message can go
stale; `set_mode` clears `submit_pending` when the mode actually changes
(Task 5 pointer a, the smaller of the two options); `remove_note` is shared by
`add-note` and `create-note`. Copy: `NAME_FIELD`, `LOCATION` and the formatting
fn `will_create(path)` ("Empty folder will be created at {path}") added;
`CREATE_HEADING`, `CREATE_FOLDER`, `NEED_FOLDER`, `NEED_BRANCH` deleted.

**Tests written first, each seen failing:** P12
`add_repository::tests::create::create_joins_location_and_name` (29 rows:
nothing, Name only, Location only, a name of spaces, both, a Location not made
yet, an empty branch, a branch of spaces, the branch restored, pressed, Escape,
confirmed, "Adding…"; a mutation that dropped the Main-branch condition failed
5 of the 29), P9's Create one here row (G20), P21's Create rows, and the tests
for pointers (a) and (b). `add_repository::tests` goes from 22 to 25 tests (the
three in `create.rs`); the files edited by script keep their function lists
(checked by name list).

**Tests run:**

- Implementer, by exact name, one command: 20 passed, 0 failed (P12, the G20
  row, the pointer tests, the edited Create tests, the copy table and four
  `add_sheet` tests).
- Orchestrator's module run, once, 4 filters (all matched at least one test):
  `desktop::view::add_repository::tests` (25),
  `desktop::view::workspace::add_sheet::tests` (8),
  `desktop::view::settings::repositories::copy_tests` (1),
  `desktop::view::keyboard_tests::settings` (6) — **40 passed, 0 failed**.

**Existing tests edited:**

- E-1 (Create tests fill Name and Location through `create::set_target`): the
  Create tests, `fill_create` and its users; no assertion removed ("Root:
  {root}" still holds, the join gives the same root).
- E-1c Create's half: `create-path` becomes `create-location` (5 places).
- E-2 Create's part (`submitting_with_no_folder_says_so`): **assertion
  removed:** `create-message == NEED_FOLDER`, and Add's dead
  `!present("add-message")` (Task 5 pointer d; deleted with no replacement,
  since a `submit_pending` or `add-checks` check would pass without the guard);
  added: Create disabled with nothing typed, no `create-message`, no
  confirmation after a press.
- E-17a (continued), P18's copy table: **rows removed**, each an assertion
  removal for a constant deleted in this task: `c!(add_repository,
  CREATE_HEADING)` ("Create a repository"; the mode control names the form, B8
  has no heading), `CREATE_FOLDER` ("Folder (empty or not yet made)"; Name and
  Location replace it), `NEED_FOLDER` ("Enter the folder's path."; the button
  is disabled instead, B10), `NEED_BRANCH` ("Enter the primary branch."; same).
  No row for a live constant was removed; no row was added for `NAME_FIELD`,
  `LOCATION` or `will_create` (Part F is P20's, Task 9).
- E-17m `a_form_control_below_the_fold_is_scrolled_into_view`: the window from
  250 to 150 px, because Create's prefix is now the fourth field and sat inside
  the 250 px viewport; same assertions; no assertion removed.
- E-17n `closes.rs` (P7 `the_add_sheet_closes_every_way`): `create-path`
  becomes `create-location` (6 places; the id no longer exists).
- E-17o `returns.rs::confirmation` (G13's refused-preview row): fills Location
  (the fixture's parent) and Name ("full") instead of the folder in
  `create-path`, so Create is enabled and the non-empty folder's preview is
  refused as before. `returns.rs` stays at 502 lines.

**Review** (sonnet): spec compliant, quality approved, no Critical or
Important; no loop. Parked Minors and owner's-eye items, for the final review:

1. **Windows (native follow-on):** P12's "Location not made yet" row uses the
   literal `will_create("not/made/yet/fresh")`; on Windows `PathBuf::join`
   displays `not/made/yet\fresh`, so that row would fail on the Windows
   native runner. Build the expected value with the `joined()` helper. Native
   CI runs after merge, so this is follow-on, not a gate.
2. **Owner's eye:** `create_target` uses `Path::join`, so an absolute Name
   replaces Location (Location `/home/me`, Name `/tmp/x` gives the root
   `/tmp/x`): this follows B8 and R-13 ("as typed, unchecked") and
   `create-target` shows it honestly.
3. **Owner's eye:** Create now has two fields labelled "Name" (`NAME_FIELD` for
   the folder, Settings' `NAME` for the identity), as B8 and Part F mandate;
   the identity block's heading disambiguates visually, but the accessible
   names collide.
4. **Design inconsistency for the owner:** B8 item 6 titles the identity block
   "Your name & email (optional)" but Part F only has `IDENTITY_HEADING` "Your
   name & email"; the code follows Part F (the copy contract) plus the kept
   `CREATE_IDENTITY` note ("Leave both empty…"). Fix B8 or Part F, not the code.
5. A checked path with no last component (a filesystem root): "Create one
   here" puts the whole path in Location and leaves Name empty and focused;
   Create stays disabled until it is filled. B6 does not cover the case.
6. The pointer (b) test `create_leaves_no_message_behind` presses Create with
   nothing typed, then runs Browse…, and asserts no `create-message`: it can
   only catch the old pattern (a message on a disabled press).
7. `add_repository_tests.rs` is 932 lines (931 before; already over 500);
   `returns.rs` stays at 502.
8. Tab order on screen (Name, Location, Browse…, Main branch, prefix, Name,
   Email) follows draw order and is not asserted (P16, Task 9).

## Task 7 (Share Changes Through)

Step 1 (merge `origin/main`): a no-op, `origin/main` `14ca0938` was already an
ancestor (main has not moved since Ruling 20). Base `d7b39f62`; code `a14e862d`,
`feat(desktop): share changes through a chosen remote after an add`. 14 files
under `src/desktop/`. Boundary diff empty; `Cargo.lock` unchanged. Gates pass.

**What was built:** `add_repository/share.rs` (new, 389 lines: C1's select and
C2's rule: `Share` items, the `Shares` delegate whose `on_will_change` refuses an
ineligible remote, `share_confirmed` ignoring a `Confirm` naming one, `share_of`
deciding what to send, `held` for R-30); `branches_stale` renamed
`choices_stale`, starting `true`, with `sync_choices` syncing both selects (the
share choice is the sheet's own `share_choice`, never the select's value);
`SHARE_THROUGH`, `SHARE_HELP`, `KEEP_LOCAL`, `SSH_ONLY` verbatim from Part F
plus `SHARE_NEEDS_IDENTITY` verbatim from C2; `add_form` draws the select after
the main branch, existing mode only, not for a registered folder; `submit`
decides the share when Add is pressed and `AddEvent::Confirmed` carries it, so a
Prepare again re-records the same share; `PendingAdd::share` is used (its
dead-code allow is removed); `workspace/add_sheet/share.rs` (new, 39 lines):
`Workspace::send_share` sends `Intent::RemoteSelect { root, name }` on
`Lane::User` with no confirmation, as Settings' `submit_select` does, for
`registered_root` (R-29), nothing for `None` and nothing once
`worker().quitting()` (K-10). `add_ended` calls it before the open-number
checks, so an earlier open's share is still sent (R-26). `add_sheet.rs` is 499
lines (was 494).

**Tests written first, each seen failing:** P13
`add_repository::tests::share::share_changes_through_lists_the_folders_remotes`
(23 rows; 19 failed first) and P14
`workspace::add_sheet::tests::a_chosen_remote_is_selected_after_the_add_succeeds`
(45 rows in `add_sheet_tests/share.rs` 491 lines and `share_attempts.rs` 319;
11 of 43 failed first). Four deliberate breakages each failed their rows. P14
reads what the worker received; every row is transport-free (no SSH-harness
case). `add_repository::tests` goes from 25 to 26, `add_sheet::tests` from 8 to
9; no existing test function was changed.

**Tests run:**

- Implementer, by exact name: P13, P14 and P9 passed (3 of 3); P10, P11 and the
  keyboard add walk passed earlier.
- Orchestrator's module run, once, 4 filters (all matched at least one test):
  `desktop::view::add_repository::tests` (26),
  `desktop::view::workspace::add_sheet::tests` (9),
  `desktop::view::settings::repositories::copy_tests` (1),
  `desktop::view::keyboard_tests::settings` (6) — **42 passed, 0 failed**.

**Existing tests edited:** none; the only edits add the new submodule
declarations and P14's entry to `add_repository_tests.rs` (+2 lines, now 934)
and `add_sheet_tests.rs`. No assertion removed.

**Review** (sonnet): spec compliant, quality approved, no Critical or
Important; no loop. Parked Minors and owner's-eye items, for the final review:

1. **Design ambiguity for the owner (R-30 against C2):** C2's second case sends
   `Some(None)` ("stop sharing") for Keep over a configured eligible remote,
   "so the result agrees with what the sheet showed"; R-30 holds the select on
   Keep for a Valid configuration with no identity, even when it names an
   eligible remote. Read together the sheet would send "stop sharing", which
   commits and fails without an identity. The code follows R-30 (`share_of`
   returns `None` while held; P14 pins "nothing sent"). Amend C2's second case
   ("unless held, R-30"); or show the configured remote selected and disabled in
   the held state instead of Keep (more truthful, but R-30's literal text says
   Keep).
2. **For the final review (a real gap in a Review Focus item):**
   `add_sheet_tests/share.rs` `closed_meanwhile`: R-26's "an earlier open's
   success still sends" is only exercised with the sheet closed, never
   reopened, so the session number does not change; a mutation that sent only
   when `entry.session == self.add.session` would pass P14 (mutation 3 caught
   only the "sheet still open" variant). A row that closes, reopens (a new
   session), releases the held add and expects one send would pin it.
3. `share_attempts.rs` `failed_selection` does not assert that focus stays and
   the sheet is not reopened (no code path does it); no test pins that Create
   sends no selection (hard-coded `None`); `mod share;` sits away from the other
   module declarations in `add_sheet.rs`; the select's wrapper stays a Tab stop
   while held and disabled (the Tab test passes).
4. Before a check lands the select shows only Keep, chosen (the design is
   silent; P13 pins it). `SHARE_NEEDS_IDENTITY` (R-30's line) is verbatim from C2
   but has no row in Part F's table, like `ADDING`: P20 (Task 9) lists it. The
   Prepare again row pins one send, not B10's waiting rule.
5. Privacy: `Share.location` is the library-redacted `fetch_location`, which C1
   requires and draws in the select's items; the request and `PendingAdd.share`
   carry the remote's name only. P17 (Task 9) exercises it.
6. Windows: the P14 row that types `sub/../registered` depends on path
   canonicalisation; a native-CI follow-on, like the P12 literal.

## Task 8 (The Unlock Key Dialog)

Base `08c7752e`; code `6d7734d1`, `feat(desktop): the Unlock key dialog on the
frame, with its key card`. 13 files under `src/desktop/`. Boundary diff empty;
`Cargo.lock` unchanged. Gates pass.

**What was built:** the dialog on the frame in Part D's layout
(`passphrase.rs`, 452 lines): the header with the lock tile
(`Tone::Accent`), title `passphrase-title` ("Unlock your key") and
`passphrase-subtitle` while a prompt waits; the key card
(`passphrase/card.rs`, 127 lines: `passphrase-key`, `passphrase-key-label` and
`passphrase-fingerprint`, read from `list_keys` through `Services::read` under
the new `ViewScope::Unlock`; the label alone before the read lands, when it
fails, with no matching key or no fingerprint); the field labelled
`PASSPHRASE`; a refusal puts focus back in `passphrase-input` and sends no
reply; the note always shown; the footer Not now then Unlock; width 440 px
(not tested). `open_dialog` takes `&Services` and `show_prompt` passes it
(E-13). `label_for` and "Key {id}" are gone. Three guards drop a read that lands
late (`drop_read` in `answer`, `withdraw` and `unavailable_now`; `listed`
ignores a stale stamp or a withdrawn prompt). `ReadFailure` carries only a code
and recovery actions, so a failed read puts no text in the dialog. The test-only
U-8 hook (`#[cfg(all(test, feature = "desktop-test-support"))]`) calls the same
private `listed` that the real read lands through.

**Tests written first, each seen failing:** P2's Unlock row (the old order
`[passphrase-ok, passphrase-cancel]`), P18
`desktop::view::passphrase::tests::the_unlock_dialog_names_the_key`, P19
`a_refused_passphrase_keeps_focus_in_the_field` (the Unlock button by Enter and
Space on its focused handle); P1's 7 Unlock rows passed before and after (they
pin kept behaviour). New files: `passphrase_tests/unlock.rs` (340 lines),
`frame_tests/unlock.rs` (109); a fixture helper `import_key_without_public_key`
covers the no-fingerprint row.

**Tests run:**

- Implementer, by exact name: P18, P19, P2 (3), P1 (1), the 12 edited and
  Unlock evidence tests, polling_privacy (1) and 2 more passphrase tests.
- Orchestrator's module run, once, 9 filters (all matched at least one test):
  `desktop::view::{passphrase::tests (18), keyboard_tests::passphrase (3),
  keyboard_tests::lifecycle (4), frame::tests (3), polling_privacy_tests (1),
  operations::outcomes::route_tests (8), editing_privacy_tests (3),
  lifecycle::privacy_tests (1)}`, `desktop::services::scheduler::attention_tests
  (5)` — **46 passed, 0 failed**. The Unlock evidence tests and both privacy
  sweeps are in it.
- Named doubt (the reviewer listed tests outside the run list that open this
  dialog), `desktop::view::workspace::add_sheet::tests`,
  `lifecycle::credential_tests`, `needs_attention` and `item::edit_tests`:
  241 of 242 passed; the one failure is a Task 7 test hang, below.

**Existing tests edited:**

- E-9 (`passphrase_tests.rs`): the `label_for` and `passphrase-note` sites.
  **Assertion removed:** `label_for("primary") == "Passphrase for primary"`
  (`:169`), with `label_for`; `assert_eq!(PASSPHRASE, "Passphrase")` takes its
  line; at `:676` `passphrase-key-label == "next"` was **added** to keep that
  test's meaning (the dialog is the "next" prompt's).
- E-10: no code edit (the loops walk to the control and never press anything on
  the way; with Not now first, `cancel_by_tab…` reaches it in one Tab,
  `unlock_by_tab…` passes it without pressing). **E-17q**, doc only: the module
  doc of `keyboard_tests/passphrase.rs` said Cancel is reached "past Unlock".
- E-13 (`keyboard_tests/lifecycle.rs`): `open_dialog` gains the `Services`
  argument.
- **E-17p** `polling_privacy_tests.rs:1146-1147`: the positive control asserted
  the sweep read "Passphrase for primary" on the "passphrase dialog" surface;
  that copy is Part F's own change (`PASSPHRASE` replaces `label_for`), so it is
  replaced by two rows, "Passphrase" (only `passphrase-label` or the input's aria
  label can give it) and "primary" (only `passphrase-key-label`). Neither is
  vacuous; the negative sweep (`holds_none` over `SENTINELS`) and the 25-surface
  coverage are unchanged; no assertion removed. Task 0's sweep patterns could not
  match it.

**Review** (opus): spec compliant, quality approved, no Critical or Important;
no loop. Parked Minors, for the final review:

1. `REFUSED` is now drawn in `colors.red` (`passphrase.rs:416`): Part D asks only
   for "today's REFUSED under the field"; a visual change for the owner's eye
   (untested, rightly).
2. `passphrase_tests/unlock.rs:273-304` `focus_in` and `tab_to` copy
   `frame_tests/dialogs.rs::inside` and `keyboard_tests/passphrase.rs::tab_to`;
   sharing `Table` widened `frame.rs:259` (`pub(in crate::desktop::view) mod
   tests`) and `frame_tests.rs:42-67`, coupling the passphrase tests to the frame
   tests (a `test_support` home would be cleaner; test-only).
3. P18's table has set-up `expect`/`assert!` calls (`unlock.rs:160`, `:194`,
   `:216-255`) that stop the remaining rows instead of reporting them; they are
   preconditions.
4. No test pins `TITLE` ("Unlock your key") or `NOT_NOW_NOTE`; the reviewer
   checked both against Part F by hand; P20 (Task 9) may cover them.
5. `answer_key_read` (`card.rs:119`) reads like `answer()`; `land_key_read` would
   be clearer. `frame.rs:246` still has `#[allow(dead_code)] // Used from Task 4`
   on `sheet_footer`, which `add_repository/render.rs:272` now uses (left over
   from Task 4, not this task's).
6. Not verified by test: if a prompting job holds the index so the `list_keys`
   read returns Busy after its retries, the card shows the label only (R-16
   accepts it).

**A Task 7 test hang, found at this task's boundary.** The reviewer named tests
outside the run list that open this dialog; the extra run
(`add_sheet::tests`, `credential_tests`, `needs_attention`, `item::edit_tests`)
passed 241 of 242 tests. The one failure is Task 7's P14
`a_chosen_remote_is_selected_after_the_add_succeeds`: "the awaited state did not
arrive in 180s" (`harness.rs:73`, `wait_until`). Evidence so far: it passes
alone (20 s), with `edit_tests` (219 tests), with `credential_tests`, with
`needs_attention`, and with the other `add_sheet` tests (9 tests, 27 s); it fails,
reproducibly (3 of 3), when the nine `add_sheet` tests run together with
`credential_tests` and `needs_attention` (24 tests), stalling the whole 180 s;
and **the identical failure occurs on the Task 7 tree** (`a14e862d`, checked out
detached and restored), so Task 8 is not the cause. It is a defect in Task 7's
test or the production code under it that shows only with a wider test mix.
Task 7 fix round 1, below.
