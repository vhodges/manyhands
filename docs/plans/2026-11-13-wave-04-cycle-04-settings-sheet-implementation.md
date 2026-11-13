---
title: "Wave 04 Cycle 04 Settings Sheet Implementation Plan"
date: 2026-11-13
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M7APBYM2T2ERVNRWDTZXTJ3W"
---

# Wave 04 Cycle 04 Settings Sheet Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task by task.
>
> **Draft**, revision 2: it applies review round 1 and decisions 12 and
> 13. **Revision 2 has not been independently reviewed.**
>
> Execution waits until Cycle 03b (`01M784Y94Q3AXT6XJX38F5VK1V`) is merged
> to main (Cycle decision 1). Re-read the design before each task, and
> record every checkpoint on ticket `01M7AMAPNHHTNECXBF4CA5WPKV`. Steps
> use checkbox (`- [ ]`) syntax.

**Goal:** Move Settings out of the item area into `05`'s 720 px right-side
sheet, with:

- a three-entry menu: Repositories, Sharing & keys, Checking for updates;
- every existing way in, Cycle 03b's included, landing on its page and
  control;
- focus returned on close;
- the open item, its scroll and its draft untouched.

**Architecture.**

- **The sheet.** Settings is a Kit `Sheet` on Root's one sheet layer.
  - A `SheetOwner` record lets the workspace keep it. Today's
    `sync_sheet` closes every sheet but the details sheet.
  - One function, `Workspace::show_settings(route, origin)`, opens it
    from every way in, and today's entry functions become wrappers.
  - `SettingsHost::arrive(route)` applies the route and returns the
    control to focus.
  - The close is detected in `sync_sheet`. It empties a typed passphrase
    and returns focus, except when the close is quiet (design Part A).
- **The frame.** A header with the description, a 214 px menu of plain
  `Role::Tab` entries, and a body that scrolls on its own
  `ContentScroll::region()`. The frame's root handles the page keys
  (Part B).
- **The pages.** The five page entities stay, and the menu groups them
  (Part C):
  - Repositories with Sharing, under one chosen repository;
  - Sharing & keys, with the key list in `05`'s rows and Trusted servers;
  - Checking for updates.
- No library change.

**Tech stack:** Rust 2024; `gpui-kit` 0.6.6 through `gpui_kit::*` only;
Devenv. No new dependency.

**Spec:** [Cycle](../Cycles/wave-04-cycle-04-settings-sheet.md) and
[design](2026-11-13-wave-04-cycle-04-settings-sheet-design.md), with
`docs/design/05-sheets-and-dialogs.md` "Settings sheet" and `06`.

- "Fact n", "W-n", "P-n", "R-n", "E-n", "K-n" and "U-n" refer to the
  design. "Decision n" and "A-n" refer to the Cycle.
- `D/` is `src/desktop/`.

## Global Constraints

**Precondition.**

- The owner has approved the three documents, authorized implementation
  and chosen the execution method. Not yet met.
- **Cycle 03b is closed and merged to main** (decision 1). Not yet met.
- Task 0 does not start before both.
- The design's Document Amendments are applied by the planner in the
  approval commit, not by a task.

**Roles.**

- The orchestrator runs the tasks, or dispatches one implementer per task
  and then a fresh reviewer, one at a time.
- It alone writes the ledger and ticket comments and pushes the ticket
  branch.
- Implementers never dispatch subagents and never edit ticket, comment or
  ledger files.

**Where.**

- Every task runs in `.manyhands/worktrees/01M7AMAPNHHTNECXBF4CA5WPKV` on
  `manyhands/ticket/01M7AMAPNHHTNECXBF4CA5WPKV`.
- No lane branches and no extra worktrees: an extra worktree once made
  `ticket create` answer `recovery_required`.
- Every Rust command is `devenv shell -- cargo …` with `--locked`.

**Boundary.** A task changes `src/desktop/` and its tests. After each
task's commit, this prints nothing:

```sh
git diff --stat origin/main...HEAD -- src ':!src/desktop' tests Cargo.toml Cargo.lock packaging
```

A gap in the headless API stops the task for the owner.

**Main.** Merge `origin/main` before Tasks 0 and 6, and whenever main
moves. Never rebase.

**Push** (R-41).

- After each task's commit, the orchestrator pushes the ticket branch
  with `item sync` from the main checkout (owner, 2026-11-11).
- It then checks that `HEAD` equals
  `origin/manyhands/ticket/01M7AMAPNHHTNECXBF4CA5WPKV`.
- `item sync` and `comment add` merge main first when it has moved. Any
  such merge is recorded in the ledger, and the task's tests are run again
  by exact name.

**GPUI** only through `gpui_kit::*`, never a glob import.

**Copy.**

- Every user-visible string is a `pub(crate) const`, verbatim from design
  Part E or kept from today. Tests compare constants.
- Sentence case and no exclamation marks.
- `…` (U+2026) on an action that opens a confirmation or sheet (`06`).
- `05`'s two key-form labels are kept verbatim, "Make a new key" and "Use
  a key I already have…".

**Test runs** (`AGENTS.md`, Testing Strategy):

- **Command.** Desktop tests run as
  `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <path> -- --exact`,
  written below as `… <path>`.
- **Paths.** A test's path is its module's. For example:
  - `desktop::view::settings::tests::<name>` for
    `D/view/settings/mod_tests.rs`;
  - `desktop::view::workspace::settings_sheet::tests::<name>` for the new
    `D/view/workspace/settings_sheet_tests.rs`.
- **New test files** are included as
  `#[cfg(all(test, feature = "desktop-test-support"))] #[path = "…_tests.rs"] mod tests;`.
- **Each run reports more than zero tests.** A command with an empty
  filter is stopped at once.
- **Between tasks,** only the tests the task wrote or changed run, by
  exact name, each named edit included, listed in full in the ledger.
- **Two module runs** (a named doubt). The swap (Task 1) and the
  three-entry menu (Task 5) break tests by behaviour, not by name. So at
  the end of each, the orchestrator runs once, by module, every test file
  the task touched and every file Task 0's sweep listed for it, and fixes
  what fails before the commit.
- **No whole-suite run** happens before Task 9.
- **Limits.**
  - A command past about 15 minutes is stopped, narrowed and noted.
  - Never build while a test run is in progress.
  - A new window test runs alone first.
- **U-3** is checked on screen (Task 9). U-5 is checked in Task 0.

**Each task's last Verify lines:**

```sh
devenv shell -- cargo fmt --check
devenv shell -- cargo check --locked --features desktop
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

**Test-first.**

- Each behaviour's test is written and seen to fail before its code.
- Table-driven where the design gives a table (P4, P5, P6, P9).
- Focus assertions read focus inside the same update that causes the
  change (Cycle 01 and 03a ledgers).
- Guards are tested with dispatched actions, never pointer clicks, which
  the scrim takes, and never keys bound in another context (Cycle 02
  ledger).
- Window tests with polling on answer every poll (Cycle 03a handoff).

**No appearance tests** (Wave Testing). Nothing asserts widths, colours,
icons, the header's layout or that a view renders.

**Existing tests.**

- They change only as E-1 to E-18 say.
- Any other edit is pre-authorized (decision 7). It is recorded in the
  ledger as E-19 onward with its reason, and checked by the task's
  reviewer.
- An assertion removal is called out in the ledger and the review-ready
  summary.
- After any scripted edit, the ledger lists the test functions of the
  touched files before and after, so none is lost.

**Dead code.** An item first used by a later task carries a plain
`#[allow(dead_code)] // Used from Task N`, and Task N removes it.
Test-only helpers and accessors are `#[cfg(test)]`. Allows go on items,
never on a module (03a ledger).

**Commits.** One per task, with a conventional message, ending
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Ticket records.**

- After Tasks 0, 2, 6 and 9, a comment through
  `devenv shell -- cargo run --locked --bin manyhands-cli -- comment add …`,
  run against the main checkout while the ticket worktree is clean.
- It records the commits, the tests run with counts (copied from the
  ledger), and the rulings.
- Before `ticket save`, commit any owned `ticket.md` edit. `comment add`
  publishes the ticket branch.

**Ledger.** `docs/plans/2026-11-13-wave-04-cycle-04-settings-sheet-execution.md`.

- It is created in Task 0 and written only by the orchestrator.
- Only the ledger assigns E-numbers past E-18.

**File size.** New source files stay below about 500 lines.

- `D/view/workspace.rs` (488 lines) changes only `ContentArea.sheet`'s
  type, the subscription and `mod settings_sheet;`. The Workspace's new
  code goes in `D/view/workspace/settings_sheet.rs`.
- `D/view/settings/mod.rs` (517 lines) moves the frame into
  `D/view/settings/frame.rs` and `arrive` into
  `D/view/settings/arrive.rs`.
- `remotes.rs`, `hosts.rs` and `repositories.rs` are already over 800
  lines. A task that grows one moves its new code into a submodule.

## Review Focus

Inputs no task's main journey exercises, each pinned by a test in the
task named:

- **Task 1:**
  - Ctrl+, pressed with the details sheet open and a field edited (P3);
  - quitting with Settings open (P6: Quit now keeps focus);
  - `open_item` landing while Settings is open (P1).
- **Task 2:**
  - a route from the lifecycle dialog, which closes itself first (P5: the
    dialog's origin);
  - a popover row whose footer item has hidden by close time (P5);
  - a Check host finishing after Escape (P22);
  - a route arriving after the Kit closed the sheet (P22).
- **Task 3:** choosing the already-chosen repository with identity text
  typed (P10).
- **Task 4:** a passphrase typed after a `Hosts(path)` route (P9: kept).
- **Task 6:**
  - a key-list route while the keys are still being read (P4);
  - `Hosts(path)` with the panel's selection on another repository (P4).

## File Map And Dependency Order

| File | Tasks |
| --- | --- |
| `D/view/workspace/settings_sheet.rs` (new) and `settings_sheet_tests.rs` (new) | 1, 2, 6 |
| `D/view/workspace/content.rs` | 1, 2, 8 |
| `D/view/workspace/render.rs` | 1, 2 |
| `D/view/workspace.rs` | 1 |
| `D/view/workspace/lifecycle.rs` | 1 |
| `D/view/workspace/requests.rs`, `footer.rs`, `repository_menu.rs`, `D/view/operations/attention.rs` (or 03b's successors) | 1 (wrappers), 2 (origins), 6 (routes) |
| `D/view/settings/mod.rs`, `frame.rs` (new), `arrive.rs` (new) | 1, 2, 4, 5, 6, 8 |
| `D/model/stamp.rs` | 4 |
| `D/view/settings/repositories.rs`, `repositories/manage.rs`, `remotes.rs` | 3 |
| `D/view/settings/keys.rs`, `keys/render.rs`, `keys/commands.rs`, `hosts.rs` | 4 |
| `D/view/settings/polling.rs` | 1 (Show in Activity) |
| Copy constants across `D/view/settings/` | 7 |
| `D/view/repositories/body.rs` | 8 |

Order: 0 → 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9.

---

## Task 0: Baseline, References, Ledger

- [ ] **Step 1: Precondition.** Confirm Cycle 03b is closed and merged:
  `git log origin/main` shows "Integrate closed ticket
  01M784Y94Q3AXT6XJX38F5VK1V". If not, stop.
- [ ] **Step 2: Merge and baseline.**

```sh
git fetch origin && git merge origin/main
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

  A failure is resolved, or accepted by the owner, before Task 1.
- [ ] **Step 3: Re-map the citations (U-5).** Run
  `git diff --stat ba95e1b7 HEAD -- src/desktop docs/Waves docs/RFC docs/design Cargo.lock`.
  - Re-read each design fact and each Part D row whose file changed.
    Correct the design first, in a commit of its own noted in the ledger.
  - Read how Cycle 03b's `choose_activity` reaches `attend`, `row_action`
    and `open_route`, and what origin it passes. Record the function and
    line for Tasks 2 and 6.
  - List every 03b trigger that opens Settings: row buttons, row menu
    entries and popover rows. Add any that Part D misses as W-36 onward.
  - Read 03b's footer: `press`, `focus_after_hiding`,
    `keep_focus_in_footer`, `focus_lost`'s popover rule, and its popover's
    close when a sheet opens.
  - If `Cargo.lock` moved `gpui-kit`, `gpui-base` or `gpui-component`,
    stop for the owner. Facts 26 and 33–39 rest on the locked versions.
- [ ] **Step 4: Sweep the E-list,** over tests and production code alike:

```sh
P='src/desktop'
git grep -n -e 'settings-close' -e 'BACK_TO_TABS' -e 'sidebar-settings' \
  -e '"settings-page"' -e 'settings-page-shown' -e 'Shown::Tabs' \
  -e 'Shown::Settings' -e 'Page::Settings' -e 'settings_shown' \
  -e 'secondary-,' -e 'OpenSettings' -e 'SettingsEvent::' -e 'ContentPage::Settings' \
  -e 'open_settings' -e 'open_remotes' -e 'open_hosts' -e '_in_settings(' \
  -e 'set_selection(' -e 'manage_root(' -e 'remotes-chosen' -e 'remotes-registration' \
  -e 'forget_passphrase' -e 'content_offset' -e 'Region::Content' -- $P
git grep -n -e '"Shared SSH keys"' -e '"Host pins"' -e '"Remotes"' -e '"Hosts"' \
  -e 'publication' -e 'Publish' -e 'pinned' -e '"Select"' -e '"Unregister"' \
  -e '"Delete key"' -e '"Clear selection"' -e '"Generated by Manyhands"' \
  -e 'key-generate-' -e 'key-import-' -- $P
```

  - Record each test site against its E-n, or as E-19 onward (decision 7).
  - Name every test that, after opening Settings, acts outside it before
    closing it (E-12's kind), and give it its task.
  - A production site no task names goes into the task that breaks it.
  - List every caller of the E-5 helpers, and every test file each task
    will touch, for the module runs.
- [ ] **Step 5: Ledger, commit, push, ticket comment.** Record the base
  SHA, 03b's merge commit and Step 3's findings.

---

## Task 1: The Swap (A1, A2 without origins, A3, B1 with five entries, B2, C3's Show in Activity, Part F's content page)

**Files:**

- `D/view/workspace/settings_sheet.rs` (new) and
  `settings_sheet_tests.rs` (new);
- `content.rs`, `render.rs`, `workspace.rs`, `lifecycle.rs`,
  `requests.rs`;
- `D/view/settings/mod.rs`, `frame.rs` (new), `arrive.rs` (new),
  `polling.rs`;
- the E-1 (`:73`, `:137`, `:168`), E-2, E-3, E-4, E-6, E-7, E-8, E-12 and
  E-13 tests, and E-5's walk.

**Interfaces** (design A1, A2):

```rust
// D/view/workspace/content.rs
pub(super) enum SheetOwner { Details(WeakEntity<ItemPane>), Settings }
// ContentArea.sheet: Option<SheetOwner>

// D/view/settings/arrive.rs
pub(crate) enum SettingsRoute {
    Page(SettingsPage), Manage(PathBuf), Inspect(PathBuf), Identity(PathBuf),
    Remotes(PathBuf), Keys, Hosts(PathBuf),
    CheckHost(PathBuf, SshAuthority), RetryGeneration(DesktopRequestId),
}
impl SettingsHost {
    pub(crate) fn arrive(&mut self, route: &SettingsRoute,
        window: &mut Window, cx: &mut Context<Self>) -> Option<FocusHandle>;
}

// D/view/workspace/settings_sheet.rs
impl Workspace {
    pub(in crate::desktop::view) fn show_settings(
        &mut self, route: SettingsRoute, origin: Option<FocusHandle>,
        window: &mut Window, cx: &mut Context<Self>,
    );
    /// From `sync_sheet` (design A1 case 2); `quiet` skips the focus step.
    pub(super) fn settings_closed(&mut self, quiet: bool,
        window: &mut Window, cx: &mut Context<Self>);
    #[cfg(test)] pub(crate) fn sheet_owner(&self) -> Option<SheetOwnerKind>;
    #[cfg(test)] pub(crate) fn settings_chosen(&self, cx: &App) -> Option<PathBuf>;
}
```

- [ ] **Step 1: Write the tests, failing.** In `settings_sheet_tests.rs`:
  - P1 `settings_is_a_sheet_the_workspace_keeps`;
  - P2 `settings_leaves_the_item_as_it_was` (the W-5 row is added in Task
    6);
  - P3 `settings_replaces_the_details_sheet_and_keeps_its_edits`;
  - P6 `settings_closes_every_way`, all five rows;
  - P8 `settings_scrolls_on_its_own`;
  - P14 `show_in_activity_closes_settings_and_focuses_activity`;
  - P15 `settings_does_not_open_behind_a_dialog_or_while_quitting`.

  Run each by exact name, and see it fail.
- [ ] **Step 2: The owner record** (A1, cases 1–5).
  - Change `ContentArea.sheet`'s type (`workspace.rs:91`), and update
    `sync_sheet`.
  - `open_item` clears only a `Details` owner.
  - `wait_to_quit` (`lifecycle.rs:229-233`) calls
    `settings_closed(quiet: true)` before Quit now takes focus.
- [ ] **Step 3: `show_settings` and `arrive`** (A2 steps 1, 4–7).
  - The builder captures the `Entity<SettingsHost>` and draws
    `frame.rs`'s frame: `.size(px(720.))`; the header (`SETTINGS_HEADING`
    and `SETTINGS_NOTE`); no footer; today's five pages as a vertical
    column of entries; the body.
  - For now, `arrive` returns each route's focus as today's entry
    function gives it (fact 10). The page methods that focus today
    (`manage_root`, `ask_again`, the Hosts check, `set_up_sharing`)
    return their handle instead of focusing.
  - For now, `show_settings` records the focused handle as the origin.
- [ ] **Step 4: `settings_closed`** (A3). `hide` empties the passphrase.
  The focus step uses the origin's handle, else `Region::Content`.
- [ ] **Step 5: Its own scroll and the page keys** (B2). Make
  `ContentScroll::region()`, the body's scroll laid over the Kit body as
  in `D/view/item/sheet.rs:251-275`, and the four handlers on the frame's
  root.
- [ ] **Step 6: Show in Activity** (C3). The Workspace's answer to
  `SettingsEvent::ShowActivity` closes Settings quietly, then focuses
  `Region::Operations`.
- [ ] **Step 7: Wrappers.** Each of `open_settings`, `open_settings_on`,
  `inspect_in_settings`, `identity_in_settings`, `check_host_in_settings`,
  `retry_generation_in_settings`, `open_remotes` and `open_hosts` builds
  its route and calls `show_settings(route, None, …)`.
- [ ] **Step 8: Leave the content page.** Remove:
  - `ContentPage::Settings` and its branches (fact 9);
  - `content_body`'s passphrase calls;
  - `SettingsEvent::Closed`, `back`, the `close` handle and
    `BACK_TO_TABS`.

  The subscription at `workspace.rs:290-296` keeps `ShowActivity` only.
- [ ] **Step 9: Named edits.**
  - E-1 (`:73` without `:91-93`, `:137`, `:168`).
  - E-2, E-3, E-4, E-6, E-7, E-8, E-12 and E-13.
  - E-5's walk: `open_settings_and_each_page` closes by Escape, and focus
    returns to the control that opened it.
  - The page helpers keep today's indices: five entries until Task 5.
- [ ] **Step 10: Verify.**
  - Run Step 1's tests and every E-test above by exact name.
  - Then the module run of every touched test file and every file the
    Task 0 sweep listed for Task 1.
  - Then the last Verify lines.

**Rulings:** R-1, R-2, R-3, R-5, R-7, R-9, R-10, R-12, R-20, R-25, R-26,
R-28, R-29, R-30, R-31, R-33, R-37, R-40.

---

## Task 2: Focus And Guards (A2 steps 2–3 and 5, A3's fallbacks, A4, A5)

**Files:** `D/view/workspace/settings_sheet.rs`,
`settings_sheet_tests.rs`, `content.rs`, `render.rs`, the files of 03b's
`choose_activity` and footer (Task 0's record), `D/view/settings/hosts.rs`
and `keys/commands.rs` (landing guards).

- [ ] **Step 1: Write the tests, failing:**
  - P5 `focus_returns_to_what_opened_settings`, all seven rows;
  - P7 `nothing_acts_behind_settings`, by dispatched actions, with the
    `OpenSettings` row run from a page other than Repositories;
  - P22 `late_work_moves_no_focus`;
  - P23 `focus_stays_inside_settings`.
- [ ] **Step 2: Origins** (R-6; Part D's origins).
  - `show_settings` closes a details sheet before it records the origin.
  - 03b's Activity row and popover actions pass their handle as origin
    when they open Settings. Use the path Task 0 recorded.
- [ ] **Step 3: Fallbacks** (A3). When the origin's control is not drawn,
  focus falls to `focus_lost` with 03b's popover rule, and to
  `keep_focus_in_footer`. When the handle is gone, `Region::Content`.
- [ ] **Step 4: The key** (A5). `OpenSettings` leaves `unless_dialog` for
  its own guard.
- [ ] **Step 5: Mid-close and already open** (A2 steps 2–3; R-36).
- [ ] **Step 6: Nothing behind** (R-34, R-35).
  - `focus_content` does nothing while Settings is open.
  - `apply_review`'s `refocus` waits for no sheet.
  - Page handlers that focus on a landing (`refocus_gone`, `ask_again`)
    check that Settings is open.
- [ ] **Step 7: Verify,** then the ticket checkpoint.

**Rulings:** R-6, R-8, R-34, R-35, R-36.

---

## Task 3: Repositories With Sharing, One Chosen Repository (C1; Part G items 2, 3 and 5)

**Files:** `D/view/settings/mod.rs`, `repositories.rs`,
`repositories/manage.rs`, `remotes.rs`; `remotes_tests.rs` (E-9);
`polling_tests.rs` and `keyboard_tests/settings.rs` (E-14).

- [ ] **Step 1: Write the tests, failing:**
  - P10 `one_repository_is_chosen`;
  - P11 `every_page_is_read_again_on_open`;
  - P25 `a_slow_identity_read_fills_only_empty_fields`.
- [ ] **Step 2: One choice.**
  - `SettingsHost.selection` becomes `chosen`, with the last-seen panel
    selection kept for R-14's rule.
  - Add `SettingsHost::choose(root, window, cx)`. When `root` is already
    chosen, it leaves the identity inputs alone. Otherwise it calls
    `manage_root(root, false)` and the Remotes page's `open_on(root)`.
  - Manage on a row calls it. `show_settings` step 6 calls it.
- [ ] **Step 3: The prefill.** `apply_prefill` (`manage.rs:299-304`) sets
  a field only while it is empty.
- [ ] **Step 4: The page.** The Repositories page draws:
  - its managed section;
  - then the Remotes page's view under "Sharing", without its
    registrations list. `remotes.rs:608-640`, `choose_root` and the read
    at `new` (`:238`) go;
  - then Add and Create.
- [ ] **Step 5: Read again** (R-32). Add `RepositoriesPage::opened`, which
  calls `load_registrations` (`manage.rs:230`). `SettingsHost::open` calls
  `opened` on every page.
- [ ] **Step 6: E-9 and E-14.**
- [ ] **Step 7: Verify.**

**Rulings:** R-14, R-15, R-32, R-42 (item 5).

---

## Task 4: Sharing & Keys (C2, B3; Part G item 4)

**Files:** `D/model/stamp.rs` (`SettingsNav`, `SettingsPage::nav`),
`D/view/settings/keys.rs`, `keys/render.rs`, `keys/commands.rs`,
`hosts.rs`, `mod.rs`; `keys_tests.rs` (E-10, E-15);
`keyboard_tests/settings.rs` (E-15).

- [ ] **Step 1: Write the tests, failing:**
  - P9 `a_typed_passphrase_lives_only_on_sharing_and_keys`;
  - P12 `key_rows_send_what_todays_controls_sent`;
  - P13 `the_key_forms_open_from_their_buttons`;
  - P19 `sharing_and_keys_by_keyboard`;
  - P24 `removing_a_key_returns_focus_by_key`.
- [ ] **Step 2: `SettingsNav` and `nav()`** (B1).
  - `forget_passphrase` empties the inputs when
    `shown.nav() != SharingAndKeys` (B3).
  - The Sharing & keys page draws the Keys page's view, then the Hosts
    page's view under "Trusted servers". Check host works on the host's
    `chosen`.
- [ ] **Step 3: Key rows** (C2.1).
  - The radio is a wrapped div with `role(Role::RadioButton)`,
    `aria_toggled`, a tab stop and `USE_KEY`'s name (R-38).
  - The rows keep today's requests and today's `KeyDelete` confirmation.
  - Stop using this key sits under the list.
  - After a removal, focus goes to the next key by key ID.
- [ ] **Step 4: The forms** (C2.2). Neither is shown at first; one at a
  time; hiding the Generate form empties its passphrase fields.
  `ask_again` shows the Generate form.
- [ ] **Step 5: The info box** (C2.3), and fields filling the body up to
  480 px (R-39).
- [ ] **Step 6: E-10 and E-15.**
- [ ] **Step 7: Verify.**

**Rulings:** R-4 (`nav`), R-13, R-16, R-17, R-18, R-19, R-38, R-39, R-42
(item 4).

---

## Task 5: The Three-Entry Menu (B1)

**Files:** `D/view/settings/mod.rs`, `frame.rs`; the E-5 helpers; E-16's
sites; `mod_tests.rs` (E-1 `:91-93`).

- [ ] **Step 1: Write P16 `settings_has_three_pages`, failing.**
- [ ] **Step 2: The menu.**
  - Three entries, with ids `("settings-page", 0..3)` and their icons.
  - Each opens its first page.
  - The active entry is announced selected, and the "Shown" tag goes.
- [ ] **Step 3: E-5's helpers.** They use indices 0–2:
  - `hosts` and `keys` open entry 1;
  - `Rig::open` opens entry 2;
  - `remotes` opens entry 0, then Manage.
- [ ] **Step 4: E-16 and E-1's `:91-93`.**
- [ ] **Step 5: Verify.** Run every caller of the E-5 helpers by exact
  name, as listed in Task 0's ledger entry. Then the module run of every
  touched file.

**Rulings:** R-4, R-27.

---

## Task 6: Ways In (Part D)

**Files:** `D/view/settings/arrive.rs`, `mod.rs`, `keys.rs`;
`D/view/workspace/settings_sheet.rs`; `D/view/operations/attention.rs`
(or 03b's successor); `settings_sheet_tests.rs`; E-17's and E-18's tests.

Merge `origin/main` first.

- [ ] **Step 1: Write the tests, failing:**
  - P4 `each_route_lands_on_its_page_and_control`: the nine routes
    through `show_settings`, the rows the design lists, and a row for
    each W no existing test raises (W-9, W-14, W-16 to W-20, W-33 to
    W-35, and Task 0's W-36 onward);
  - P20 `routes_by_keyboard`;
  - P2's W-5 row.
- [ ] **Step 2: `arrive`** applies each route as Part D says:
  - the focus targets (R-24);
  - the key-list target, with its one retarget when the read lands
    (R-22);
  - `hosts-check` for `Hosts(root)`.
- [ ] **Step 3: W-25 and W-33** (R-23). Approve host… with no server named
  passes the attention row's repository.
- [ ] **Step 4: E-17 and E-18.**
- [ ] **Step 5: Verify,** then the ticket checkpoint.

**Rulings:** R-22, R-23, R-24.

---

## Task 7: Copy (Part E)

**Files:** copy constants across `D/view/settings/`; tests reading
literals (E-11).

- [ ] **Step 1: Write the tests, failing:**
  - P18 `settings_copy_uses_the_vocabulary`, matching whole words, over a
    table of every Settings copy constant;
  - P21 `settings_copy_names_no_secret`.
- [ ] **Step 2: Apply Part E's table and its rule.** List each rewritten
  constant in the ledger, old and new.
- [ ] **Step 3: E-11.**
- [ ] **Step 4: Verify.**

**Rulings:** R-24 (copy).

---

## Task 8: What Leaves (Part F)

**Files:** `D/view/repositories/body.rs`, `D/view/settings/mod.rs`,
`D/view/workspace/content.rs`; `mod_tests.rs` (E-1 `:123`).

- [ ] **Step 1: Write P17 `the_old_settings_places_are_gone`, failing.**
- [ ] **Step 2: Remove** the panel's Settings button
  (`repositories/body.rs:129-141`) and any code left only for the content
  page. Rewrite `D/view/settings/mod.rs`'s module doc for the sheet.
- [ ] **Step 3: E-1's `:123`.**
- [ ] **Step 4: Verify.**

**Rulings:** R-21.

---

## Task 9: Checklist, Gates, Review-Ready

- [ ] **Step 1: Write the on-screen checklist**
  `docs/plans/2026-11-13-wave-04-cycle-04-on-screen-checklist.md` for the
  owner, from Cycle 03a's. It covers frames 1e and 1f in the dark theme:
  - the desktop smoke test, run by the owner on Wayland:
    `devenv shell -- cargo run --locked --features desktop --bin manyhands`;
  - the header and its description (U-3);
  - the menu, the key rows and their radio, the forms, the info box,
    Trusted servers;
  - fields within the body's width (R-39);
  - Escape, the close button and a click outside;
  - focus after each kind of way in and each close;
  - the sheet's top edge against the title bar (K-6).
- [ ] **Step 2: Run the gates** (`AGENTS.md`):
  - the static gates:

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

  - the fast tier once, with `--no-fail-fast`. No SSH target runs, since
    this is desktop only:

```sh
devenv shell -- cargo test --all-features --locked --no-fail-fast --lib --bins \
  --test canonical_foundation --test cli --test discovery_rebuild \
  --test drafts --test journal_rejection --test key_material \
  --test key_storage --test local_authoring --test mutation_bridges \
  --test mutation_confirmation --test mutation_contract \
  --test mutation_relationships --test mutation_replay \
  --test read_boundary --test read_characterization --test read_comments \
  --test read_contract --test read_credentials --test read_items \
  --test read_relationships --test read_repository --test read_status \
  --test recovery_foundation_gate --test remote_reservation \
  --test repository_enablement --test session_credentials \
  --test shared_key_registry -- --skip repository::local_sync::
```

  - the CLI smoke test: `devenv shell -- cargo run --locked --bin manyhands-cli`;
  - `git fetch origin`, then `git merge-tree --write-tree HEAD origin/main`.

  Native CI is dispatched only with the owner's approval, after merge.
- [ ] **Step 3: Fix what the fast tier finds.**
  - Each failure is fixed under the named-edit rules, as E-19 onward.
  - Rerun the failed tests by exact name, not the whole tier.
  - If a failure would change the contract, stop for the owner.
- [ ] **Step 4: Fill in the evidence.** Fill the Cycle's Acceptance Status
  from the tests by exact name and the ledger. On-screen rows wait for the
  owner.
- [ ] **Step 5: Mark the ticket review-ready.** Ticket comment and push.
  After review approval, the owner closes the ticket with `ticket close`,
  which performs the merge (`AGENTS.md`).

## Plan Self-Review

| Acceptance | Task |
| --- | --- |
| A1 | 1 (P1), 6 (P4, E-18) |
| A2 | 1 (P2), 6 (P2's W-5 row) |
| A3 | 1 (P3, E-2) |
| A4 | 1 (P6), 2 (P5, P23) |
| A5 | 4 (P19, E-15), 3 (E-14), 6 (P4, P20, E-18), 1 and 5 (E-5, E-12) |
| A6 | 3 (P10), 4 (P13), 5 (P16) |
| A7 | 4 (P12); 1–8 (the named edits) |
| A8 | 8 (P17) |
| A9 | 1 (P15), 2 (P7, P22, P23) |
| A10 | 7 (P18) |
| A11 | 1 (P6), 4 (P9) |
| A12 | 1 (P14) |
| A13 | 1 (P8), 3 (P10, P11, P25), 4 (P24) |
| A14 | 7 (P21); 9 (the fast tier) |
| A15 | 9 (on screen) |

**Rulings by task:**

- **Task 1:** R-1, R-2, R-3, R-5, R-7, R-9, R-10, R-12, R-20, R-25, R-26,
  R-28, R-29, R-30, R-31, R-33, R-37, R-40.
- **Task 2:** R-6, R-8, R-34, R-35, R-36.
- **Task 3:** R-14, R-15, R-32, R-42 (item 5).
- **Task 4:** R-4 (`nav`), R-13, R-16, R-17, R-18, R-19, R-38, R-39, R-42
  (item 4).
- **Task 5:** R-4, R-27.
- **Task 6:** R-22, R-23, R-24.
- **Task 7:** R-24 (copy).
- **Task 8:** R-21.
- **All tasks:** R-41.
- **R-11:** superseded by R-40.

**Named edits by task:**

- **Task 1:** E-1 (`:73`, `:137`, `:168`), E-2, E-3, E-4, E-6, E-7, E-8,
  E-12, E-13, and E-5's walk.
- **Task 3:** E-9, E-14.
- **Task 4:** E-10, E-15.
- **Task 5:** E-5's helpers, E-16, E-1 (`:91-93`).
- **Task 6:** E-17, E-18.
- **Task 7:** E-11.
- **Task 8:** E-1 (`:123`).
- **Any task:** E-19 onward (decision 7).

**Required outcomes by task:**

- **Task 1:** P1, P2, P3, P6, P8, P14, P15.
- **Task 2:** P5, P7, P22, P23.
- **Task 3:** P10, P11, P25.
- **Task 4:** P9, P12, P13, P19, P24.
- **Task 5:** P16.
- **Task 6:** P4, P20, and P2's W-5 row.
- **Task 7:** P18, P21.
- **Task 8:** P17.

**Unverified facts by task:** U-5 in Task 0; U-3 in Task 9 (on screen).
U-1, U-2 and U-4 were settled in review round 1.

**Placeholders:** none. Every test, file and interface the tasks name is
in the design or written out above.
