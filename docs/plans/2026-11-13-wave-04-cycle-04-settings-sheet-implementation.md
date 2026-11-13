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
> **Draft**, revision 1, not yet independently reviewed. Execution waits
> until Cycle 03b (`01M784Y94Q3AXT6XJX38F5VK1V`) is merged to main (Cycle
> decision 1). Re-read the design before each task and record every
> checkpoint on ticket `01M7AMAPNHHTNECXBF4CA5WPKV`. Steps use checkbox
> (`- [ ]`) syntax.

**Goal:** Move Settings out of the item area into `05`'s 720 px right-side
sheet, with a three-entry menu (Repositories, Sharing & keys, Checking for
updates), every existing way in landing on its page and control, focus
returned on close, and the open item, its scroll and its draft untouched.

**Architecture.**

- **The sheet.** Settings is a Kit `Sheet` on Root's one sheet layer. A
  `SheetOwner` record lets the workspace keep it (today's `sync_sheet`
  closes every sheet but the details sheet). One function,
  `Workspace::show_settings(route, origin)`, opens it from every way in;
  today's entry functions become wrappers. The close is detected in
  `sync_sheet`, which empties a typed passphrase and returns focus (design
  Part A).
- **The frame.** A header with the description, a 214 px menu of plain
  `Role::Tab` entries, and a body that scrolls on its own
  `ContentScroll::region()` and handles the page keys (Part B).
- **The pages.** The five page entities stay; the menu groups them:
  Repositories with Sharing under one chosen repository; Sharing & keys
  with the key list in `05`'s rows and Trusted servers; Checking for
  updates (Part C).
- **Ways in.** `SettingsHost::arrive(route)` applies each of the 32
  triggers (Part D).
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

**Push.** After each task's commit, the orchestrator pushes the ticket
branch with `item sync` from the main checkout. It then checks that `HEAD`
equals `origin/manyhands/ticket/01M7AMAPNHHTNECXBF4CA5WPKV`.

**GPUI** only through `gpui_kit::*`, never a glob import.

**Copy.** Every user-visible string is a `pub(crate) const`, verbatim from
design Part E or kept from today. Tests compare constants. Sentence case,
no exclamation marks, `…` (U+2026) on an action that opens a confirmation
or a form.

**Test runs** (`AGENTS.md`, Testing Strategy):

- **Command.** Desktop tests run as
  `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <path> -- --exact`,
  written below as `… <path>`.
- **Paths.** A test's path is its module's, for example
  `desktop::view::settings::tests::<name>` for `D/view/settings/mod_tests.rs`
  and `desktop::view::workspace::settings_sheet::tests::<name>` for the new
  `D/view/workspace/settings_sheet_tests.rs`.
- **New test files** are included as
  `#[cfg(all(test, feature = "desktop-test-support"))] #[path = "…_tests.rs"] mod tests;`.
- **Each run reports more than zero tests.** A command with an empty
  filter is stopped at once.
- **Between tasks,** only the tests the task wrote or changed run, by
  exact name, each named edit included, listed in full in the ledger.
  There is no whole-suite run before Task 9.
- **Limits.**
  - A command past about 15 minutes is stopped, narrowed and noted.
  - Never build while a test run is in progress.
  - A new window test runs alone first.
- **Unverified facts** (U-1 to U-5) are checked with a scratch test that
  is discarded, not committed. Its result goes in the ledger.

**Each task's last Verify lines:**

```sh
devenv shell -- cargo fmt --check
devenv shell -- cargo check --locked --features desktop
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

**Test-first.**

- Each behaviour's test is written and seen to fail before its code.
- Table-driven where the design gives a table (P4, P5, P6).
- Focus assertions read focus inside the same update that causes the
  change (Cycle 01 and 03a ledgers).
- Guards are tested with key presses or dispatched actions, never pointer
  clicks, which the scrim takes (Cycle 02 ledger).
- Window tests with polling on answer every poll (Cycle 03a handoff).

**No appearance tests** (Wave Testing). Nothing asserts widths, colours,
icons, the header's layout or that a view renders.

**Existing tests.**

- They change only as E-1 to E-11 say.
- Any other edit is pre-authorized (decision 7), recorded in the ledger
  as E-12 onward with its reason, and checked by the task's reviewer. An
  assertion removal is called out in the ledger and the review-ready
  summary.
- After any scripted edit, the ledger lists the test functions of the
  touched files before and after, so none is lost.

**Dead code.** An item first used by a later task carries a plain
`#[allow(dead_code)] // Used from Task N`; Task N removes it. Test-only
helpers are `#[cfg(test)]`. Allows go on items, never on a module (03a
ledger).

**Commits.** One per task, with a conventional message, ending
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Ticket records.** After Tasks 0, 2, 6 and 9, a comment through
`devenv shell -- cargo run --locked --bin manyhands-cli -- comment add …`,
run against the main checkout while the ticket worktree is clean. It
records the commits, the tests run with counts (copied from the ledger),
and the rulings. Before `ticket save`, commit any owned `ticket.md` edit.
`comment add` publishes the ticket branch.

**Ledger.** `docs/plans/2026-11-13-wave-04-cycle-04-settings-sheet-execution.md`,
created in Task 0 and written only by the orchestrator. Only the ledger
assigns E-numbers past E-11.

**File size.** New source files stay below about 500 lines.

- `D/view/workspace.rs` (488 lines) gains nothing but the subscription
  change; the Workspace's new code goes in
  `D/view/workspace/settings_sheet.rs`.
- `D/view/settings/mod.rs` (517 lines) moves the frame into
  `D/view/settings/frame.rs` and the routes into
  `D/view/settings/arrive.rs`.
- `remotes.rs`, `hosts.rs` and `repositories.rs` are already over 800
  lines; a task that grows one moves its new code into a submodule.

## Review Focus

Inputs no task's main journey exercises, each pinned by a test in the
task named:

- Ctrl+, pressed with the details sheet open and a field edited (Task 1,
  P3).
- A route raised from the lifecycle dialog, which closes itself first:
  focus returns to the dialog's origin (Task 1, P5).
- An origin that left the tree while Settings was open (Task 1, P5).
- The scrim click and quitting as ways to close (Task 1, P6).
- A key-list route while the keys are still being read (Task 6, P4).
- Choosing the already-chosen repository with identity text typed (Task
  3, P10).
- Approve host… from an attention row whose repository is not the
  panel's selection (Task 6, P4 W-25).

## File Map And Dependency Order

| File | Tasks |
| --- | --- |
| `D/view/workspace/settings_sheet.rs` (new) and `settings_sheet_tests.rs` (new) | 1, 2, 5, 6 |
| `D/view/workspace/content.rs` | 1, 8 |
| `D/view/workspace/render.rs` | 1, 2 |
| `D/view/workspace.rs` | 1 |
| `D/view/workspace/lifecycle.rs` | 1 |
| `D/view/workspace/requests.rs`, `footer.rs`, `repository_menu.rs`, `D/view/operations/attention.rs` (or 03b's successor) | 1 (wrappers), 6 (routes) |
| `D/view/settings/mod.rs`, `frame.rs` (new), `arrive.rs` (new) | 1, 2, 5, 6, 8 |
| `D/model/stamp.rs` | 5 |
| `D/view/settings/repositories.rs`, `repositories/manage.rs`, `remotes.rs` | 3 |
| `D/view/settings/keys.rs`, `keys/render.rs`, `keys/commands.rs`, `hosts.rs` | 4 |
| `D/view/settings/polling.rs` | 5 |
| Copy constants across `D/view/settings/` | 7 |
| `D/view/repositories/body.rs` | 8 |

Order: 0 → 1 → 2 (split line) → 3 → 4 → 5 → 6 → 7 → 8 → 9.

---

## Task 0: Baseline, References, Ledger

- [ ] **Step 1: Precondition.** Confirm Cycle 03b is closed and merged
  (`git log origin/main` shows "Integrate closed ticket
  01M784Y94Q3AXT6XJX38F5VK1V"). If not, stop.
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
  - Re-read each design fact and each Part D row whose file changed, and
    correct the design first (a commit of its own, noted in the ledger).
  - Read how Cycle 03b's `choose_activity` (or its successor) reaches
    `attend` and `open_route`, and what origin it passes; record the
    function and line for Task 6.
  - Read 03b's footer: `press`, `focus_after_hiding` and
    `keep_focus_in_footer`, and its popover's close on a sheet opening.
  - If `Cargo.lock` moved `gpui-kit`, `gpui-base` or `gpui-component`,
    stop for the owner: facts 30–37 rest on the locked versions.
- [ ] **Step 4: Sweep the E-list,** over tests and production code alike:

```sh
P='src/desktop'
git grep -n -e 'settings-close' -e 'BACK_TO_TABS' -e 'sidebar-settings' \
  -e '"settings-page"' -e 'Shown::Tabs' -e 'Shown::Settings' -e 'settings_shown' \
  -e 'secondary-,' -e 'OpenSettings' -e 'SettingsEvent::' -e 'ContentPage::Settings' \
  -e 'open_settings' -e 'open_remotes' -e 'open_hosts' -e '_in_settings(' \
  -e 'set_selection(' -e 'manage_root(' -e 'remotes-choose' -e 'forget_passphrase' \
  -e 'content_offset' -e 'Region::Content' -- $P
git grep -n -e '"Shared SSH keys"' -e '"Host pins"' -e '"Remotes"' -e '"Hosts"' \
  -e 'publication remote' -e '"Select"' -e '"Unregister"' -e '"Delete key"' \
  -e '"Clear selection"' -e '"Generated by Manyhands"' -- $P
```

  - Record each test site against its E-n, or as E-12 onward (decision 7).
  - A production site no task names goes into the task that breaks it.
  - List every caller of the E-5 helpers.
- [ ] **Step 5: Ledger, commit, push, ticket comment,** recording the
  base SHA, 03b's merge commit and Step 3's findings.

---

## Task 1: The Sheet (Part A, B1 frame with today's pages)

**Files:** `D/view/workspace/settings_sheet.rs` (new),
`settings_sheet_tests.rs` (new), `content.rs`, `render.rs`,
`workspace.rs`, `lifecycle.rs`, `requests.rs`; `D/view/settings/mod.rs`,
`frame.rs` (new); the E-1, E-2, E-4, E-5 (walk only), E-6, E-7 and E-8
tests.

**Interfaces** (design A1, A2):

```rust
// D/view/workspace/content.rs
pub(super) enum SheetOwner { Details(WeakEntity<ItemPane>), Settings }
// ContentArea.sheet: Option<SheetOwner>

// D/view/settings/arrive.rs (routes applied in Task 6; Task 1 maps each
// to today's behaviour: page shown and today's focus)
pub(crate) enum SettingsRoute {
    Page(SettingsPage), Manage(PathBuf), Inspect(PathBuf), Identity(PathBuf),
    Remotes(PathBuf), Keys, Hosts(PathBuf),
    CheckHost(PathBuf, SshAuthority), RetryGeneration(DesktopRequestId),
}

// D/view/workspace/settings_sheet.rs
impl Workspace {
    pub(in crate::desktop::view) fn show_settings(
        &mut self, route: SettingsRoute, origin: Option<FocusHandle>,
        window: &mut Window, cx: &mut Context<Self>,
    );
    /// Called from `sync_sheet` when the owner is Settings and no sheet
    /// is active (design A3).
    pub(super) fn settings_closed(&mut self, window: &mut Window, cx: &mut Context<Self>);
}
```

- [ ] **Step 1: Check U-1** with a scratch test: focus on a control inside
  a Kit sheet makes `focused_region` answer `Region::Content`. Record the
  answer. If not, E-5's helpers read the focused id only.
- [ ] **Step 2: Write the tests, failing.** In `settings_sheet_tests.rs`:
  - P1 `settings_is_a_sheet_the_workspace_keeps`;
  - P2 `settings_leaves_the_item_as_it_was`;
  - P3 `settings_replaces_the_details_sheet_and_keeps_its_edits`;
  - P5 `focus_returns_to_what_opened_settings` (table: footer item, ticket
    list row, lifecycle dialog route, an origin that left the tree);
  - P6 `settings_closes_every_way` (table: Escape, close button, scrim,
    quitting; Show in Activity is added in Task 5);
  - P7 `nothing_acts_behind_settings`;
  - P15 `settings_does_not_open_behind_a_dialog_or_while_quitting`.

  Run each by exact name and see it fail.
- [ ] **Step 3: The owner record** (A1). Replace `ContentArea.sheet`'s type
  and update `sync_sheet`, `open_item` and `wait_to_quit`
  (`lifecycle.rs:229-233`): a `Settings` owner is kept; `Details` as
  today.
- [ ] **Step 4: `show_settings`** (A2 steps 1–6) and `settings_closed`
  (A3). The builder captures the `Entity<SettingsHost>` and draws
  `frame.rs`'s frame: `.size(px(720.))`, the header (`SETTINGS_HEADING`
  and `SETTINGS_NOTE`, design B1), no footer, today's page list as a
  vertical column of today's five entries, and the page. `SettingsHost`
  gains `arrive(route, window, cx)`, which for now does what today's entry
  function for that route does (fact 10), and `hide` empties the
  passphrase (B3).
- [ ] **Step 5: Wrappers.** `open_settings`, `open_settings_on`,
  `inspect_in_settings`, `identity_in_settings`, `check_host_in_settings`,
  `retry_generation_in_settings`, `open_remotes` and `open_hosts` each
  build their route and call `show_settings(route, None, …)`.
- [ ] **Step 6: The key and the guard** (A5). `OpenSettings` leaves
  `unless_dialog` for its own guard. `show_settings` refuses under a
  dialog and while quitting.
- [ ] **Step 7: Leave the content page.** Remove `ContentPage::Settings`
  and its branches (fact 9), `SettingsEvent::Closed`, `back`, the
  `close` handle and `BACK_TO_TABS`; the subscription at
  `workspace.rs:290-296` keeps `ShowActivity` only.
- [ ] **Step 8: Named edits** E-1, E-2, E-4, E-6, E-7, E-8, and E-5's walk
  (`open_settings_and_each_page` closes by Escape; focus returns to the
  control that opened it). The page helpers keep today's indices (five
  entries until Task 5).
- [ ] **Step 9: Verify.** Run P1–P3, P5–P7, P15 and every E-test above by
  exact name; then the last Verify lines.
- [ ] **Step 10: Check U-3 on screen** is deferred to Task 9's checklist;
  note it in the ledger.

**Rulings:** R-1, R-2, R-3, R-5, R-6, R-7, R-8, R-9, R-10, R-25, R-26,
R-28, R-29, R-30, R-31.

---

## Task 2: Scrolling, Page Keys, The Passphrase (B2, B3)

**Files:** `D/view/settings/mod.rs`, `frame.rs`, `render.rs`,
`content.rs`; `D/view/settings/keys_tests.rs`; `D/view/item/frame_tests.rs`
(E-3).

- [ ] **Step 1: Write the tests, failing:** P8
  `settings_scrolls_on_its_own`; P9
  `a_typed_passphrase_is_emptied_on_close_and_on_leaving_sharing_and_keys`
  (extending `a_typed_passphrase_is_emptied_when_the_page_is_hidden`; with
  five entries, "leaving" is leaving the Keys page).
- [ ] **Step 2: Its own scroll.** `SettingsHost::new` takes the column's
  `ContentScroll` and keeps `scroll.region()`; every page is made with it.
  The body is an absolute `inset_0` scroll on that handle with a vertical
  `Scrollbar`, as `D/view/item/sheet.rs:251-275` does.
- [ ] **Step 3: The page keys.** The body handles `ScrollPageUp`,
  `ScrollPageDown`, `ScrollToTop` and `ScrollToBottom` on its own scroll,
  without `cx.propagate()`.
- [ ] **Step 4: The passphrase.** `forget_passphrase` runs on `hide` and on
  a page change away from Keys; remove `content_body`'s calls.
- [ ] **Step 5: E-3.** Edit
  `page_keys_scroll_the_items_region_and_settings_keep_theirs`.
- [ ] **Step 6: Check U-4** through P8: a control taking focus below the
  fold is revealed in the sheet.
- [ ] **Step 7: Verify,** then the ticket checkpoint (split line: the sheet
  works with today's five pages).

**Rulings:** R-11, R-12, R-13.

---

## Task 3: Repositories With Sharing, One Chosen Repository (C1, Part G)

**Files:** `D/view/settings/mod.rs`, `repositories.rs`,
`repositories/manage.rs`, `remotes.rs`; `remotes_tests.rs` (E-9).

- [ ] **Step 1: Write the tests, failing:** P10 `one_repository_is_chosen`;
  P11 `every_page_is_read_again_on_open`.
- [ ] **Step 2: One choice.** `SettingsHost.selection` becomes `chosen`.
  Add `SettingsHost::choose(root, window, cx)`: a no-op for the identity
  inputs when `root` is already chosen; else `manage_root(root, false)`
  and the Remotes page's `open_on(root)`. Manage on a row calls it.
- [ ] **Step 3: The page.** The Repositories page draws its managed
  section, then the Remotes page's view as "Sharing" without its
  registrations list (`remotes.rs:608-640` and `choose_root` go), then
  Add and Create.
- [ ] **Step 4: Read again.** `open` calls `opened` on every page,
  Repositories included (R-32).
- [ ] **Step 5: E-9.** Remotes tests choose by Manage.
- [ ] **Step 6: Verify.**

**Rulings:** R-14, R-15, R-32.

---

## Task 4: Sharing & Keys (C2)

**Files:** `D/view/settings/keys.rs`, `keys/render.rs`, `keys/commands.rs`,
`hosts.rs`, `mod.rs`; `keys_tests.rs` (E-10).

- [ ] **Step 1: Check U-2** with a scratch test: a route focuses a key
  row's `Radio`, and Space chooses it. If not, use the design's fallback
  (a `Button` drawn as a radio, announced as one).
- [ ] **Step 2: Write the tests, failing:** P12
  `key_rows_send_what_todays_controls_sent`; P13
  `the_key_forms_open_from_their_buttons`.
- [ ] **Step 3: Key rows** as design C2.1, with today's requests and
  today's `KeyDelete` confirmation; Stop using this key under the list.
- [ ] **Step 4: The forms.** Make a new key and Use a key I already have…
  show one form at a time; `ask_again` shows the Generate form.
- [ ] **Step 5: The info box and Trusted servers.** The Sharing & keys
  page draws the Keys page's view, then the Hosts page's view under
  "Trusted servers"; Check host works on the host's `chosen`.
- [ ] **Step 6: E-10.**
- [ ] **Step 7: Verify.**

**Rulings:** R-16, R-17, R-18, R-19.

---

## Task 5: The Three-Entry Menu; Checking For Updates (B1, C3)

**Files:** `D/model/stamp.rs`, `D/view/settings/mod.rs`, `frame.rs`,
`polling.rs`; `D/view/workspace/settings_sheet.rs`; the E-5 helpers.

- [ ] **Step 1: Write the tests, failing:** P16 `settings_has_three_pages`;
  P14 `show_in_activity_closes_settings_and_focuses_activity`; P6's Show in
  Activity row.
- [ ] **Step 2: The menu.** Add `SettingsNav` and `SettingsPage::nav`
  (design B1). The menu draws three entries, ids `("settings-page", 0..3)`,
  each opening its first page; the active entry is announced selected.
- [ ] **Step 3: Show in Activity.** The Workspace's answer to
  `SettingsEvent::ShowActivity` closes Settings without A3's focus step,
  then reveals and focuses Activity.
- [ ] **Step 4: E-5.** The helpers use indices 0–2: `hosts` and `keys`
  open entry 1, `Rig::open` entry 2, `remotes` entry 0 then Manage; the
  five `open_page` callers' indices.
- [ ] **Step 5: Verify,** including every caller of the E-5 helpers by
  exact name (listed in Task 0's ledger entry).

**Rulings:** R-4, R-20, R-27.

---

## Task 6: Ways In (Part D)

**Files:** `D/view/settings/arrive.rs`, `mod.rs`, `keys.rs`;
`D/view/workspace/settings_sheet.rs`; `D/view/operations/attention.rs` (or
03b's successor); 03b's `choose_activity` callers;
`settings_sheet_tests.rs`.

Merge `origin/main` first.

- [ ] **Step 1: Write P4, failing:**
  `each_way_in_lands_on_its_page_and_control`, a table over W-1 to W-32
  less W-4, each row raising its trigger as a person would (key, click by
  id, recovery, offer, Activity action through 03b's seam) and asserting
  `settings_shown`, the focused element's id and the chosen repository.
- [ ] **Step 2: `arrive`** applies each route as Part D says: the focus
  targets (R-24), the key-list target with its one retarget when the read
  lands (R-22), `hosts-check` for `Hosts(root)`.
- [ ] **Step 3: Origins.** 03b's Activity and popover actions pass their
  own focus handle as origin to `show_settings` (Task 0 Step 3's finding).
- [ ] **Step 4: W-25** (R-23): Approve host… with no server named passes
  the attention row's repository.
- [ ] **Step 5: Verify,** then the ticket checkpoint.

**Rulings:** R-22, R-23, R-24.

---

## Task 7: Copy (Part E)

**Files:** copy constants across `D/view/settings/`; tests reading
literals (E-11).

- [ ] **Step 1: Write P18 `settings_copy_uses_the_vocabulary`, failing:** a
  table of every Settings copy constant, none containing a retired term.
- [ ] **Step 2: Apply Part E's table and its rule.** List each rewritten
  constant in the ledger, old and new.
- [ ] **Step 3: E-11.**
- [ ] **Step 4: Verify.**

**Rulings:** R-24 (copy).

---

## Task 8: What Leaves (Part F)

**Files:** `D/view/repositories/body.rs`, `D/view/settings/mod.rs`,
`D/view/workspace/content.rs`.

- [ ] **Step 1: Write P17 `the_old_settings_places_are_gone`, failing.**
- [ ] **Step 2: Remove** the panel's Settings button
  (`repositories/body.rs:129-141`), any code left only for the content
  page, and rewrite `D/view/settings/mod.rs`'s module doc for the sheet.
- [ ] **Step 3: Verify.**

**Rulings:** R-21.

---

## Task 9: Checklist, Gates, Review-Ready

- [ ] **Step 1: Write the on-screen checklist**
  `docs/plans/2026-11-13-wave-04-cycle-04-on-screen-checklist.md` for the
  owner, from Cycle 03a's. It covers frames 1e and 1f in the dark theme:
  - the desktop smoke test, run by the owner on Wayland:
    `devenv shell -- cargo run --locked --features desktop --bin manyhands`;
  - the header and its description (U-3);
  - the menu, the key rows, the forms, the info box, Trusted servers;
  - Escape, the close button and the scrim;
  - focus after each way in and each close;
  - the sheet's top edge against the title bar (K-6).
- [ ] **Step 2: Run the gates** (`AGENTS.md`):
  - the static gates:

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

  - the fast tier once, with `--no-fail-fast`. No SSH target, since this
    is desktop only:

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
- [ ] **Step 3: Fill in the evidence.** Fill the Cycle's Acceptance Status
  from the tests by exact name and the ledger. On-screen rows wait for the
  owner.
- [ ] **Step 4: Mark the ticket review-ready.** Ticket comment and push.
  After review approval, the owner closes the ticket with `ticket close`,
  which performs the merge (`AGENTS.md`).

## Plan Self-Review

| Acceptance | Task |
| --- | --- |
| A1 | 1 (P1), 6 (P4 W-1 to W-3) |
| A2 | 1 (P2) |
| A3 | 1 (P3, E-2) |
| A4 | 1 (P5, P6), 5 (P6's Show in Activity row) |
| A5 | 1, 5 (E-5 journeys), 6 (P4) |
| A6 | 3 (P10), 4 (P13), 5 (P16) |
| A7 | 4 (P12); 1–7 (E-5 to E-11) |
| A8 | 8 (P17) |
| A9 | 1 (P7, P15) |
| A10 | 7 (P18) |
| A11 | 1 (P6), 2 (P9) |
| A12 | 5 (P14) |
| A13 | 2 (P8), 3 (P10, P11) |
| A14 | 4 (P12's tooltips); 9 (the fast tier) |
| A15 | 9 (on screen) |

**Rulings by task:**

- **Task 1:** R-1, R-2, R-3, R-5, R-6, R-7, R-8, R-9, R-10, R-25, R-26,
  R-28, R-29, R-30, R-31.
- **Task 2:** R-11, R-12, R-13.
- **Task 3:** R-14, R-15, R-32.
- **Task 4:** R-16, R-17, R-18, R-19.
- **Task 5:** R-4, R-20, R-27.
- **Task 6:** R-22, R-23, R-24.
- **Task 7:** R-24 (copy).
- **Task 8:** R-21.

**Named edits by task:** E-1, E-2, E-4, E-6, E-7, E-8 and E-5's walk:
Task 1. E-3: Task 2. E-9: Task 3. E-10: Task 4. E-5's helpers: Task 5.
E-11: Task 7. E-12 onward: any task (decision 7).

**Required outcomes by task:** P1, P2, P3, P5, P6, P7, P15 in Task 1 (P6's
Show in Activity row in Task 5); P8, P9 in Task 2; P10, P11 in Task 3;
P12, P13 in Task 4; P14, P16 in Task 5; P4 in Task 6; P18 in Task 7; P17
in Task 8.

**Unverified facts by task:** U-1 in Task 1; U-2 in Task 4; U-3 in Task 9
(on screen); U-4 in Task 2; U-5 in Task 0.

**Placeholders:** none. Every test, file and interface the tasks name is
in the design or written out above.
