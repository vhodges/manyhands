---
title: "Wave 04 Cycle 03a Repositories Panel And Sharing Status Implementation Plan"
date: 2026-11-12
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M7888XHHWDYGFXG0ZEXNG8KX"
---

# Wave 04 Cycle 03a Repositories Panel And Sharing Status Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task by task.
> **Draft:** not approved; nothing here is authorized until the owner
> approves the three documents and chooses the execution method. Re-read
> the design before each task and record every checkpoint on ticket
> `01M784Y8C73B39GR4JWENXE5D0`. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Rebuild the Repositories panel to `03-panels.md`: one
plain-language state per repository from D6's model, the row tooltip with
the Git detail, a toolbar, a context menu by pointer and keyboard, and the
footer's "Checked for updates …" and "N to share" items; take the
repository details block out of the item area.

**Architecture:** The scheduler keeps two more facts per repository and
the attempt keeps what is not shared (Part A). A pure function turns them
and the navigation entry into one state, its action, its sentence and its
tooltip (Part B). The panel draws `ListRow`s from those states (Part C); a
Workspace-owned popup like Cycle 01's dock menu carries the actions
(Parts D, E); the details block goes (Part F); two footer controls read
the same model (Part G). No library change.

**Tech stack:** Rust 2024; `gpui-kit` 0.6.6 through `gpui_kit::*` only;
Devenv. No new dependency.

**Spec:** [Cycle](../Cycles/wave-04-cycle-03a-repositories.md) and
[design](2026-11-12-wave-04-cycle-03a-repositories-design.md), with
`docs/design/03-panels.md`, `02-layout-and-docking.md` "Footer" and
`06-language-and-states.md`. "Fact n", "P-n", "R-n", "E-n", "K-n" and
"U-n" refer to the design; "decision n" to the Cycle.

## Global Constraints

**Precondition.** The owner has approved the three documents, authorized
implementation and chosen the execution method. Task 0 does not start
before that.

**Roles.** The orchestrator runs the tasks, or dispatches one implementer
per task and then a fresh reviewer, one at a time. It alone writes the
ledger and ticket comments and pushes the ticket branch. Implementers
never dispatch subagents and never edit ticket, comment or ledger files.

**Where.** Every task runs in
`.manyhands/worktrees/01M784Y8C73B39GR4JWENXE5D0` on
`manyhands/ticket/01M784Y8C73B39GR4JWENXE5D0`. No lane branches and no
extra worktrees (an extra worktree once made `ticket create` answer
`recovery_required`). Every Rust command is `devenv shell -- cargo …`
with `--locked`.

**Boundary.** A task changes `src/desktop/` and its tests, plus the
documents in the design's Document Amendments. After each task's commit
this prints nothing:

```sh
git diff --stat origin/main...HEAD -- src ':!src/desktop' Cargo.toml Cargo.lock packaging
```

A gap in the headless API stops the task for the owner.

**Main.** Merge `origin/main` at Task 0, at Task 5 and whenever main
moves. Never rebase. If Wave 04 Cycle 03b has merged, stop for a short
checkpoint with the owner on the footer, the attention count and the
operations rows.

**Push.** After each task's commit, the orchestrator pushes the ticket
branch to origin (a fast-forward of this branch only) and checks that
`HEAD` equals `origin/manyhands/ticket/01M784Y8C73B39GR4JWENXE5D0`, so
reviewers who are not local can follow (owner, 2026-11-11).

**GPUI** only through `gpui_kit::*`, never a glob import.

**Copy.** Every user-visible string is the design's, verbatim, held in a
`pub(crate) const` (or a formatting function for strings with values)
beside its use, so tests compare constants. Sentence case, no
exclamation marks, `…` (U+2026) on an action that opens a dialog, sheet
or page.

**Test runs** (AGENTS.md, Testing Strategy; decision 18):

- Desktop: `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <path> -- --exact`,
  written below as `… <path>`.
- A test's path is its module's: `desktop::view::workspace::tests::<name>`
  for `view/workspace_tests.rs`; `desktop::view::dock::tests::<module>::<name>`
  for `view/dock_tests/<module>.rs`; `desktop::services::scheduler::entity_tests::<name>`
  for `services/scheduler_entity_tests.rs`. New `_tests.rs` files are
  included as `#[cfg(test)] #[path = "…_tests.rs"] mod tests;`, with
  `all(test, feature = "desktop-test-support")` when they open a window.
- Each run reports more than zero tests; a command with an empty filter is
  stopped at once.
- Between tasks only the tests the task wrote or changed run, by exact
  name, each named edit included, listed in full in the ledger. Unchanged
  tests are not rerun. No whole-suite run before Task 9.
- A command past about 15 minutes is stopped, narrowed and noted.
- Never build while a test run is in progress.

**Each task's last Verify lines:**

```sh
devenv shell -- cargo fmt --check
devenv shell -- cargo check --locked --features desktop
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

**Test-first.** Each behaviour's test is written and seen to fail before
its code. Model tests are table-driven (one table per function). Focus
assertions read focus inside the same update that causes the change
(Cycle 01 ledger).

**No appearance tests** (Wave Testing): nothing asserts colours, sizes,
icons, tones or that a view renders. Tooltip and description text is
tested through its pure function (fact 12).

**Test seams.** A stored outcome comes from `PollingBackend::store_outcome`
(Task 1); attempts, phases and attention from `schedule_mut()`
(`D/services/scheduler/entity.rs:228`), the scheduler `World`
(`D/services/scheduler_support.rs:24`) or `PollingBackend` answers
(`D/test_support/polling.rs:201-311`); reads are held with
`DelayingBackend::hold_read(kind)` and counted with `reads_seen(kind)`
(`D/test_support/delaying.rs:135`, `:268`). Window tests that need the
scheduler opt in with `Services::with_polling()` (fact 26). A pointer
press behind a dialog lands on its overlay, so guard tests use key
presses or dispatched actions (Cycle 01 ledger). Fixture data directories
only; no network; no real waits; `now` passed in. No `chmod` fixtures.

**Existing tests** change only as E-1 to E-15 say. Any other edit is
pre-authorized (decision 17): recorded in the ledger as E-16 onward with
its reason, checked by the task's reviewer, and an assertion removal is
called out in the ledger and the review-ready summary. After any scripted
edit, the ledger lists the test functions of the touched files before and
after, so none is lost.

**Dead code.** An item first used by a later task carries a plain
`#[allow(dead_code)] // Used from Task N` (the `cfg_attr(not(test))` form
warns in the test build, Cycle 02 ledger); Task N removes it. Test-only
helpers are `#[cfg(test)]`.

**Commits.** One per task, conventional message, ending
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Ticket records.** A comment through `devenv shell -- cargo run --locked --bin manyhands-cli -- comment add …`,
run against the main checkout while the ticket worktree is clean, after
Tasks 0, 4 and 9: commits, tests with counts, rulings. Before
`ticket save`, commit any owned `ticket.md` edit. `comment add` publishes
the ticket branch.

**Ledger.** `docs/plans/2026-11-12-wave-04-cycle-03a-repositories-execution.md`,
created in Task 0, written only by the orchestrator. Only the ledger
assigns E-numbers past E-15.

**File size.** New source files stay below about 500 lines.
`D/view/sidebar.rs` (576 lines) shrinks: its panel code moves to
`D/view/repositories.rs` in Task 3 and its details block goes in Task 6.

## Review Focus

Inputs no task's main journey exercises that would most likely bite a
person, each pinned by a test in the task named:

1. **Two facts at once.** A paused repository whose last check found work
   to share, or a locked key while a check runs: the first row of Part
   B's table wins (Task 2, `each_state_and_its_precedence`, pairs).
2. **A share that fails or is only partial** leaves the count as it was;
   only `Outcome::Success` removes an item (Task 1,
   `finished_shares_are_heard`).
3. **The menu for a row that is not the chosen one.** A right click on
   another row acts on that row's repository and leaves the choice alone
   (Task 4, `the_menu_by_pointer_and_keyboard`).
4. **Remove from the panel while a sheet is open** is refused, and after
   the dialog focus is on the list, not the item area (Task 4).
5. **To share across two repositories**, with the chosen one among them:
   each activation chooses the next one after it, wrapping (Task 7,
   `status_items_say_and_choose`).

## File Map And Dependency Order

| Task | Part | Summary |
| --- | --- | --- |
| 0 | — | Merge main, baseline gates, re-check facts, E-sweep, ledger. |
| 1 | A | Stored outcome and shared copy in the scheduler; `to_share`; finished shares; `Facts.stored`; the seam; E-9. |
| 2 | B | `repository_state`, `repository_tooltip`, `menu_items`; their tables. |
| 3 | C | The panel module: rows from states, one meaning of selected, deferred set, titles, list states; E-2, E-3, E-1 row part, E-12 part. |
| 4 | D | The repository menu by pointer and keyboard; Remove's guard and focus; E-13; checkpoint. |
| 5 | E | Toolbar buttons and the More menu; E-11, E-12 part; merge main. |
| 6 | F | The details block leaves the item area; E-1 rest, E-4 to E-8. |
| 7 | G | Footer status items; E-10, E-12 part. |
| 8 | I | Privacy sweep; E-15. |
| 9 | — | On-screen checklist, gates, review-ready; checkpoint. |

New source files:

- `D/model/repository_state.rs` (with `mod repository_state;` in
  `D/model/mod.rs`)
- `D/view/repositories.rs` and `D/view/repositories/menu.rs` (with
  `mod repositories;` in `D/view/mod.rs`)

New test files: `D/model/repository_state_tests.rs`,
`D/view/repositories_tests.rs`. New tests in existing files are named in
their tasks.

Order: 0 → 1 → … → 9, one at a time.

---

## Task 0: Baseline, References, Ledger

- [ ] **Step 1: Merge and baseline.**

```sh
git fetch origin && git merge origin/main
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

  A failure is resolved, or accepted by the owner, before Task 1.
- [ ] **Step 2: Re-check facts.** Run
  `git diff --stat 7034522f HEAD -- src/desktop src/repository/read docs/Waves docs/RFC docs/design Cargo.lock`.
  Re-read each fact whose file changed and correct the design first. If
  `Cargo.lock` moved `gpui-kit`, `gpui-base` or `gpui-component`, stop for
  the owner: facts 10–16 rest on the locked versions.
- [ ] **Step 3: Sweep the E-list,** over tests and production code alike:

```sh
P='src/desktop'
git grep -n -e 'selection-' -e 'sidebar::selection' -e 'repository_details' \
  -e 'remove_focus' -e 'retry_focus' -e 'SHARE_MAIN_BUTTON' -e 'selection-share-main' -- $P
git grep -n -e 'repo-row' -e 'repo-root' -e 'repo-selected' -e 'repo-unavailable' \
  -e 'repo-item' -e 'repositories-state' -e 'repositories-last-read' \
  -e 'repositories-failure' -e 'repositories-retry' -e 'REPOSITORIES_HEADING' \
  -e 'RepositoryRow' -e 'RepositoriesPane' -e 'sidebar::repositories' -- $P
git grep -n -e 'not_shared' -e 'AttemptSummary' -e 'status::Facts' -e 'Facts {' \
  -e 'Read::Polled' -e 'fn since_library' -e 'Heard::' -- $P
git grep -n -e 'plan(' -e 'Control::ALL' -e 'fn said' -e 'TAB_LIMIT' \
  -e 'ToolbarButton::ALL' -e 'toolbar_focus' -e 'with_no_repository' \
  -e 'ICONS' -e '"Repositories"' -- $P
```

  `Facts {` also matches the unrelated `retire_loop::Facts`
  (`D/model/polling/retire_loop.rs:70`); only `status::Facts` sites count
  (`D/model/polling/status_tests.rs`, `D/view/settings/polling.rs`).
  `not_shared` also matches `PillState::EditingSaved`, which is unrelated.
  Record each site in the ledger against the E-n or the task that changes
  it. A test site no E-n names is a pre-authorized edit (decision 17),
  recorded as E-16 onward; a production site no task names goes into the
  task that breaks it. List every user of `keyboard_tests.rs`'s
  `select_a_repository` helper (E-3) and of `shell/support.rs`'s `reach`
  and `said` (E-10).
- [ ] **Step 4: Ledger, commit, push, ticket comment** recording the base
  SHA.

---

## Task 1: Sharing Facts The Scheduler Keeps (Part A)

**Files:**
- Modify: `D/model/polling/attempt.rs`, `D/model/polling.rs`,
  `D/model/polling/status.rs`, `D/services/scheduler/registration.rs`,
  `D/services/scheduler/entity.rs`, `D/services/scheduler/react.rs`,
  `D/services/scheduler/reconfigure.rs`, `D/view/settings/polling.rs`,
  `D/test_support/polling.rs`
- Tests: `D/model/polling_tests.rs`, `D/model/polling_schedule_tests.rs`
  (E-9), `D/model/polling/status_tests.rs` (E-9),
  `D/view/settings/polling_tests.rs` (E-9),
  `D/services/scheduler/reconfigure.rs` (its `mod tests`),
  `D/services/scheduler_entity_tests.rs`

**Interfaces:**
- Produces, in `D/model/polling/attempt.rs`:
  - `AttemptSummary` gains `pub to_share: Vec<AttemptTarget>`: every
    `LocalAhead` and `Unpublished` target, main included (`item_id`
    `None`). The `not_shared: u32` field goes; `pub fn not_shared(&self)
    -> usize` returns `to_share.len()`. `Unpublished` targets stay in
    `listed` as well. `LocalAhead` still counts in `up_to_date`.
- Produces, in `D/model/polling.rs`:
  - `pub fn shared(&mut self, root: &Root, item: Option<&ItemId>) -> bool`:
    removes from `repo.last.to_share` the target whose `item_id` is
    `item` (main for `None`); true when one went.
- Produces, in `D/model/polling/status.rs`:
  - `Facts` gains `pub stored: Option<PollingOutcome>`; `since_library`
    takes it and, when `last` is `None`, returns Part A's Settings line for
    a stored outcome before "Checked {ago}" or "Not checked yet".
  - `pub(crate) fn ago(then: OffsetDateTime, now: OffsetDateTime) -> String`
    (was private, `:134`).
- Produces, in `D/services/scheduler/registration.rs`:
  - `pub(crate) struct SharedCopy { pub name: String, pub location: String }`
    (`location` is the redacted `push_location` of the remote with
    `selected_for_publication && publication_eligible`).
  - `Read::Polled` gains `outcome: Option<PollingOutcome>` and
    `shared_copy: Option<SharedCopy>`, filled in `read` (`:101-120`).
- Produces, in `D/services/scheduler/entity.rs`:
  - fields `pub(super) stored: BTreeMap<Root, PollingOutcome>` and
    `pub(super) shared_copies: BTreeMap<Root, SharedCopy>`, set in
    `registered` (`:342-365`) beside `authorities`, refreshed in `answered`
    (`D/services/scheduler/react.rs:41`) from the status it reads, and
    removed where `authorities.remove(root)` is (`:390`).
  - `pub(crate) fn stored(&self, root: &Root) -> Option<PollingOutcome>`,
    `pub(crate) fn shared_copy(&self, root: &Root) -> Option<&SharedCopy>`.
  - `Heard::Shared` handled in `heard` (`:482`): `schedule.shared(…)`, then
    `cx.notify()` when it changed.
- Produces, in `D/services/scheduler/reconfigure.rs`:
  - `Heard::Shared { root: Root, item: Option<ItemId> }`; `heard` returns
    it for `WorkerEvent::Finished` of `Intent::ItemSync { root, id, .. }`
    (`item: Some(id)`) or `Intent::RepoSync { root }` (`item: None`) with
    `envelope.outcome == Outcome::Success`, before `scope_of`.
- Produces, in `D/test_support/polling.rs`:
  - `pub(crate) fn store_outcome(&self, root: &str, outcome: PollingOutcome)`
    on `PollingBackend`; its `polling_status` (`:404`) returns the
    library's answer with `latest_outcome` replaced for that root.
- Consumes: `PollingOutcome` (`L/repository/read/dto.rs:689`),
  `RemoteListDto`/`RemoteDto` (`:200`).

- [ ] **Step 1: Check U-6.** Read
  `desktop::services::scheduler::attention_tests::unlock_after_cancelled_prompt`
  (`D/services/scheduler_attention_tests.rs:227`): a user-lane `PollOnce`
  prompts through the worker's provider. Record in the ledger whether this
  settles U-6. If it does not, stop for the owner (decision 9).
- [ ] **Step 2: Write the failing tests.**
  - `desktop::model::polling::tests::schedule::to_share_counts_and_finished_shares`,
    one table: an attempt with `LocalAhead` (ticket), `Unpublished`
    (document), main `LocalAhead` (`item_id` `None`) and `Current`; then
    `shared` with the ticket's ID → true and count 2; again → false; with
    an unknown ID → false; with `None` → main gone; on a root with no
    `last` → false (P4, P5).
  - `desktop::services::scheduler::reconfigure::tests::finished_shares_are_heard`,
    one table of `heard` over `WorkerEvent::Finished` envelopes:
    `ItemSync` success → `Shared { item: Some }`; `ItemSync` with
    `Outcome::Partial`, `Blocked`, `Error` → `None`; `RepoSync` success →
    `Shared { item: None }`; `PollOnce` unchanged (P5). Build requests as
    `resumed_and_abandoned_retirements_are_heard` (`:354`) does.
  - `desktop::services::scheduler::entity_tests::the_stored_outcome_and_shared_copy_are_kept`:
    `World::new(true, cx)`, `ssh_repo("a")`, `store_outcome(a,
    TransportUnavailable)`, `list`; then `stored(a)` and `shared_copy(a)`
    (name `origin` and a location; copy the remote set-up from
    `ssh_repo`, `D/services/scheduler_support.rs:71`); `store_outcome(a,
    Completed)` and `activate` → replaced; the repository removed (listed
    without it) → both gone (P3).
  - E-9: `desktop::model::polling::tests::schedule::attempt_summary_counts`
    expects `not_shared()` 2 in each case (`LocalAhead` and
    `Unpublished`); `desktop::model::polling::status::tests::status_text_table`
    gains `stored: None` on every `Facts` and rows for each stored outcome
    of Part A's table with no `last` (P3); "N not shared yet" texts follow
    the new count; `desktop::view::settings::polling_tests::settings_polling_rows`
    follows if a text it pins changed.
- [ ] **Step 3: Run them; they fail** (fields, methods and variants
  missing).
- [ ] **Step 4: Implement** the interfaces above. `D/view/settings/polling.rs`'s
  `row` (`:379-399`) passes `stored: scheduler.stored(root)`.
- [ ] **Step 5: Run** each test of Steps 2 by exact name.
- [ ] **Step 6: Verify lines; commit; push.**

---

## Task 2: Repository State (Part B)

**Files:**
- Create: `D/model/repository_state.rs`, `D/model/repository_state_tests.rs`
- Modify: `D/model/mod.rs`

**Interfaces:**
- Produces, in `D/model/repository_state.rs`:
  - `pub(crate) struct RepoFacts<'a> { pub entry: &'a RepositoryEntry, pub schedule: Option<&'a RepoSchedule>, pub unplaced: Option<Unplaced>, pub holding: Option<Holding>, pub stored: Option<PollingOutcome>, pub shared_copy: Option<&'a SharedCopy>, pub checking_now: bool, pub sharing_main: bool, pub polling: bool, pub titles: &'a dyn Fn(&ItemId) -> Option<String>, pub now: OffsetDateTime }`
  - `pub(crate) enum StateKind { CantOpen, NeedsFixing, Locked, NeedsAttention, Checking, CantReach, ToShare(usize), Paused, UpToDate, ThisComputer, Unknown }`
    (Part B's rows 1–11, in order).
  - `pub(crate) enum Tone { Failure, Warning, Working, Muted, Success, Secondary }`
  - `pub(crate) enum RowAction { Retry, Unlock(KeyId), Attend(Action, Reason), CheckNow, RetryNow, Settings }`
    (`Action` and `action_of` are D6's, `D/view/operations/attention.rs:73`;
    move `action_of` and `Action` to the model if they are view-private —
    record where in the ledger).
  - `pub(crate) struct RepoState { pub kind: StateKind, pub label: Option<String>, pub icon: Option<&'static str>, pub tone: Tone, pub action: Option<RowAction>, pub sentence: Vec<String> }`
  - `pub(crate) fn repository_state(facts: &RepoFacts<'_>) -> RepoState`
  - `pub(crate) fn repository_tooltip(facts: &RepoFacts<'_>, state: &RepoState) -> Vec<String>`
    (Part B's lines 1–4).
  - `pub(crate) enum MenuChoice { StateAction, CheckNow, ShareMain, OpenFolder, Settings, Remove }`
  - `pub(crate) struct MenuItem { pub choice: MenuChoice, pub label: String, pub enabled: bool }`
  - `pub(crate) fn menu_items(facts: &RepoFacts<'_>, state: &RepoState) -> Vec<MenuItem>`
    (Part D's table: the state's action first only when it is not one of
    the fixed items; "Retry now" for Check now under Can't reach; "Share
    main…" without a publication remote; "Sharing…" disabled while
    `sharing_main`).
  - Copy constants beside them: `CANT_OPEN`, `NEEDS_FIXING`, `LOCKED`,
    `NEEDS_ATTENTION`, `CHECKING` (D6's `status::CHECKING`), `CANT_REACH`,
    `PAUSED` (D6's), `UP_TO_DATE`, `THIS_COMPUTER`, `fn to_share(n)`, and
    Part B's sentences and Part D's labels.
- Consumes: Task 1's `SharedCopy`, `Facts.stored`, `status::row`,
  `status::ago`, `cant_reach`.

- [ ] **Step 1: Write the failing tests.**
  - `desktop::model::repository_state::tests::each_state_and_its_precedence`,
    one table: each of Part B's rows 1–11 alone (each stored outcome of
    Part A among them, and `Unplaced::Unreadable`, `Resolution::Busy`
    giving no state, `Shape::Held`, polling off); then pairs: Can't open
    with Locked; Needs fixing with To share; Locked with Checking…;
    Checking… with Can't reach (decision 8); To share with Paused; Paused
    with Up to date; a stored outcome with a live attempt (the attempt
    wins). Assert `kind`, `label` and `action` (P1–P3).
  - `desktop::model::repository_state::tests::tooltips_say_the_state_and_the_facts`,
    one table of `repository_tooltip`: each state's sentence; with and
    without a primary branch, a shared copy, a configuration remote the
    scheduler does not schedule (K-5); To share with titles known, unknown
    (ID), and main (P6, P7).
  - `desktop::model::repository_state::tests::menu_items_follow_the_state`,
    one table of `menu_items`: the state's action first or absent; Check
    now disabled with no schedule, `Unplaced` or `checking_now`; Retry now
    under Can't reach; "Share main…" with no remote; "Sharing…" disabled
    (P10).
- [ ] **Step 2: Run them; they fail.**
- [ ] **Step 3: Implement** the interfaces. The first row of Part B that
  holds wins. No I/O; `now` from the facts.
- [ ] **Step 4: Run** the three tests by exact name.
- [ ] **Step 5: Verify lines; commit; push.** Items Task 3 first uses carry
  `#[allow(dead_code)] // Used from Task 3`.

---

## Task 3: Rows, Titles And List States (Part C)

**Files:**
- Create: `D/view/repositories.rs`, `D/view/repositories_tests.rs`
- Modify: `D/view/mod.rs`, `D/view/sidebar.rs` (its panel code moves out),
  `D/view/workspace.rs`, `D/view/workspace/render.rs`,
  `D/view/workspace/repositories.rs`, `D/view/workspace/lists.rs` (titles
  from loaded lists), `D/services/mod.rs` (`ReadKind::ShareTitles`),
  `D/assets.rs`
- Tests: `D/view/workspace_tests.rs` (E-2, E-3, E-1 row part),
  `D/view/settings/repositories_manage_tests.rs` (E-2),
  `D/view/keyboard_tests.rs` (E-3), `D/assets_tests.rs` (E-12 part)

**Interfaces:**
- Produces, in `D/view/repositories.rs` (moved from `sidebar.rs`, `:69-340`):
  - `pub(crate) struct RepositoryRow { pub key: RepoKey, name: String, state: RepoState, tooltip: Vec<String>, chosen: bool }`
  - `pub(crate) fn rows(workspace: &Workspace, cx: &App) -> Vec<RepositoryRow>`
    (builds `RepoFacts` per entry from `Navigation`, the scheduler,
    `Worker::in_flight` (`D/services/worker.rs:744`) and `sharing_main`).
  - `RepositoryDelegate` with `type Item = ListRow`; each row:
    `ListRow::new(("repo-row", i)).label(name).description(tooltip.join(". ")).tooltip(tooltip.join("\n"))`,
    `.selected(chosen)`, `.focused(highlighted && list focused)`, the
    `folder-git-2` icon, the name, and the state at the right with id
    `("repo-state", i)` and its label as accessible name. A `canvas` in
    each row records its bounds by root for Task 4.
  - `RepositoriesPane` as today, plus `pub(crate) fn rows(&self, cx: &App) -> Vec<RepositoryRow>`
    for the comparison.
  - `pub(crate) fn body(navigation: &Navigation, pane: &RepositoriesPane, cx: &mut Context<Workspace>) -> impl IntoElement`
    with Part C's list-state table (ids `repositories-loading`,
    `repositories-empty`, `repositories-alert`; the alert's action is
    `("repositories-alert", "action")`), and the `sidebar-settings` button
    kept (decision 15). No in-body heading.
  - Copy: `LOADING_REPOSITORIES`, `NO_REPOSITORIES`, `ADD_A_REPOSITORY`,
    `NOT_UPDATED_BUSY`.
- Produces, on `Workspace`: `titles: HashMap<(RepoKey, ItemId), String>`;
  `fn learn_titles(&mut self, root: &RepoKey, rows)` called when the
  Tickets or Documents lists load; `fn read_share_titles(&mut self, cx)`
  sending one `show_item` read per unknown `to_share` item under
  `ReadKind::ShareTitles` (copy the `MadeReady` reads, `D/services/mod.rs:120-123`).
- Changes: `Workspace::render` no longer calls `RepositoriesPane::set`
  (`render.rs:205-206`); when `repositories::rows` differ from the pane's,
  it applies them with `window.defer` (R-8).

- [ ] **Step 1: Check U-4.** In a scratch test, render twice with a row
  change between and assert the pane holds the new rows after one
  `run_until_parked`, with no notify inside render. If not, stop: rows are
  pushed from the scheduler observer (`D/view/workspace.rs:215`, as
  `observe_in`) and the navigation updates instead; record the ruling.
- [ ] **Step 2: Write the failing tests.**
  - `desktop::view::repositories::tests::rows_states_and_titles`, a window
    journey with polling on (copy the opt-in from
    `D/view/polling_window.rs:74`): two repositories; `schedule_mut()`
    gives one a completed attempt with two `to_share` items and the other
    none; the rows' `("repo-state", i)` read "2 to share" and "Up to
    date"; a successful `ItemSync` of one (`answer_job`, as the share
    tests do) → "1 to share" with no list read; one title from the loaded
    Tickets list, one from a `ShareTitles` read (held, then answered), one
    whose read fails named by ID (through `repository_tooltip` on the
    workspace's facts); a busy and a failed list reload keep the rows under
    the alert and Retry reads again; the first load shows
    `repositories-loading`; with no repositories, `repositories-empty`
    and its action opens Settings › Repositories; click and Enter choose
    a row, arrows move the cursor, `selected()` is the chosen repository
    (P4–P6, P8, P9, P16).
  - E-2: `desktop::view::workspace::tests::a_busy_list_reload_keeps_the_last_rows_with_a_label`,
    `desktop::view::settings::repositories::manage_tests::tabs_close_once_the_list_no_longer_gives_their_root`
    use `repositories-alert`.
  - E-3: `desktop::view::workspace::tests::the_fixtures_repositories_are_listed_by_name`,
    `…::the_repository_selector_selects_with_arrows_and_enter`,
    `…::a_repository_rows_selected_state_has_one_meaning`; the
    `select_a_repository` helper (`D/view/keyboard_tests.rs:262`) reads
    `selected() == Some(true)`.
  - E-1, row part: `desktop::view::workspace::tests::a_renamed_away_repository_is_unavailable_until_retry_finds_it_again`
    reads `("repo-state", i)` "Can't open" where it read
    `("repo-unavailable", i)` (`:776`, `:785`); the `selection-*` parts
    stay until Task 6.
  - E-12 part: `desktop::assets::tests::every_item_view_icon_loads` gains
    each of `hard-drive`, `pause`, `lock`, `loader-circle`,
    `triangle-alert` that `DesktopAssets.load(&icon_path(name))` does not
    already load (check each first; a Kit icon needs no entry).
- [ ] **Step 3: Run them; they fail.**
- [ ] **Step 4: Implement.** Move the panel code out of `sidebar.rs`;
  `sidebar.rs` keeps `selection` (until Task 6) and the constants other
  files import (fact 6). Add `ReadKind::ShareTitles` and the cache.
- [ ] **Step 5: Run** every test of Step 2 by exact name, and each user of
  `select_a_repository` the Task 0 sweep listed.
- [ ] **Step 6: Verify lines; commit; push.**

---

## Task 4: The Repository Menu (Part D)

**Files:**
- Create: `D/view/repositories/menu.rs`
- Modify: `D/view/repositories.rs`, `D/keys.rs`, `D/view/workspace.rs`,
  `D/view/workspace/render.rs`, `D/view/workspace/focus.rs`,
  `D/view/workspace/requests.rs` (`remove_repository`),
  `D/view/operations/attention.rs` (`attend` reachable from the menu)
- Tests: `D/view/repositories_tests.rs`, `D/keys_tests.rs` (E-13)

**Interfaces:**
- Produces, in `D/keys.rs`: `pub(crate) const REPOSITORIES_CONTEXT: &str = "Repositories";`
  (used by `Region::key_context`, `D/view/workspace/focus.rs:100`); the
  action `OpenRepositoryMenu`; `KeyBinding::new("menu", OpenRepositoryMenu, Some(REPOSITORIES_CONTEXT))`
  and the same for `shift-f10`; the `no_json` action
  `pub(crate) struct RepositoryMenuChoice { pub root: RepoKey, pub choice: MenuChoice }`
  (as `DockPanel`, `:133-138`).
- Produces, in `D/view/repositories/menu.rs`:
  - `pub(crate) enum Anchor { Pointer(Point<Pixels>), Bounds(Bounds<Pixels>) }`
  - `pub(crate) struct RepositoryMenu` with
    `fn open(root: RepoKey, items: Vec<MenuItem>, context: FocusHandle, anchor: Anchor, restore: Option<FocusHandle>, window, cx) -> Self`
    building a `PopupMenu` with `menu_with_disabled(label, Box::new(RepositoryMenuChoice { root, choice }), !enabled)`
    and `separator()` before Remove, subscribed to `DismissEvent`; and
    `fn layer(&self, window, cx) -> impl IntoElement` as
    `D/view/footer/menu.rs:98`.
- Produces, on `Workspace`: `repository_menu: Option<RepositoryMenu>`;
  `fn open_repository_menu(&mut self, root: RepoKey, anchor: Anchor, window, cx)`;
  `fn open_focused_repository_menu(&mut self, window, cx)` (the
  highlighted row, else the chosen one, at its recorded bounds);
  `fn repository_menu_chosen(&mut self, action: &RepositoryMenuChoice, window, cx)`
  dispatching: state action → `retry_resolution` / `Scheduler::unlock` /
  `attend`; Check now → `PollOnce` on `Lane::User` (as
  `D/view/settings/polling.rs:225`); Share main → `share_main`; Open
  folder → `open_folder(Path::new(&root), cx)`; Settings… →
  `open_settings(SettingsPage::Repositories)`; Remove →
  `remove_repository`. All three wrapped in `unless_dialog`
  (`render.rs:174`); the layer dropped under a modal as `dock_menu_layer`
  (`D/view/workspace/footer.rs:244`).
- Changes: `remove_repository` (`D/view/workspace/requests.rs:357`)
  refuses when `modal_open` and returns focus to the Repositories list
  (copy the handle from `RepositoriesPane::focus`, `D/view/sidebar.rs:326`);
  `last_region` becomes `Repositories` (R-12).

- [ ] **Step 1: Check U-1 and U-2.** In a scratch test, with focus in the
  Repositories list, `walk.keys("shift-f10")` reaches an `OpenRepositoryMenu`
  handler; `window.right_click(("repo-row", 1))` reaches the row's
  `capture_any_mouse_down`. If U-1 fails, bind in the Kit `List` context
  with the `Repositories` parent and record it; if U-2 fails, stop for the
  owner.
- [ ] **Step 2: Write the failing tests.**
  - `desktop::view::repositories::tests::the_menu_by_pointer_and_keyboard`:
    right click on row 1 while row 0 is chosen → the menu for row 1, the
    choice unchanged; items read by index as
    `footer_tests/mod.rs:166-170` reads them; `shift-f10` and `menu` open
    it for the highlighted row; choosing Check now sends `PollOnce` for
    that root on the user lane; Share main sends `RepoSync`; "Share
    main…" opens Settings › Remotes; Open folder records the root in
    `OpenedFolders` (`D/view/item/lock.rs:96`); Settings… opens Settings ›
    Repositories; a Locked row's Unlock… reaches `Scheduler::unlock`
    (copy from `D/view/operations/attention_tests.rs:171`); Remove from
    Manyhands… opens the dialog with its note, Cancel returns focus to the
    list (read in the same update), OK removes the row; Esc closes the
    menu and returns focus; with a dialog open, the action and a
    dispatched `RepositoryMenuChoice` do nothing, and an open menu closes
    when a modal opens (P10–P12).
  - E-13: `desktop::keys::tests::each_key_is_bound_to_its_action` gains
    `menu` and `shift-f10` → `OpenRepositoryMenu` in `Repositories`.
- [ ] **Step 3: Run them; they fail.**
- [ ] **Step 4: Implement.**
- [ ] **Step 5: Run** both tests by exact name.
- [ ] **Step 6: Verify lines; commit; push; ticket comment** (checkpoint:
  commits, tests with counts, U-1, U-2, U-4 and U-6 answers, rulings).

---

## Task 5: Toolbar And The More Menu (Part E)

**Files:**
- Modify: `D/view/workspace/focus.rs` (`ToolbarButton`),
  `D/view/workspace.rs` (`toolbar_focus`), `D/view/dock/header/toolbar.rs`,
  `D/view/repositories/menu.rs`, `D/assets.rs`
- Tests: `D/view/dock_tests/toolbars.rs` (E-11), `D/assets_tests.rs`
  (E-12 part)

**Interfaces:**
- `ToolbarButton` gains `AddRepository`, `CheckAll`, `RepositoriesMore`
  (ids `add-repository-button`, `check-all-button`,
  `repositories-more-button`), `region()` Repositories; `ALL: [Self; 7]`;
  `toolbar_focus: [FocusHandle; 7]`; `of(PanelKind::Repositories)` returns
  the three (`D/view/dock/header/toolbar.rs:65-69`).
- `press_toolbar` (`:156`): Add → `open_settings(SettingsPage::Repositories)`;
  Check all → `PollOnce` for each repository with a schedule, not
  `Unplaced`, not checking, in list order (R-13); More → the More menu at
  the button's bounds.
- `toolbar_enabled` (`:143`): Check all only when one qualifies.
- `MenuChoice` gains `Refresh`, `Rebuild`; `fn more_items(chosen: Option<&RepoKey>) -> Vec<MenuItem>`
  in `D/model/repository_state.rs`: Refresh, Rebuild the search index,
  separator, Remove from Manyhands…, each disabled with nothing chosen;
  `repository_menu_chosen` maps them to `maintain_index(false|true)` and
  `remove_repository`.

- [ ] **Step 1: Merge main** (Global Constraints, Main).
- [ ] **Step 2: Write the failing tests.**
  - E-11: `desktop::view::dock::tests::toolbars::panel_toolbars_work_by_tab_and_enter`
    loops over the three new buttons too: each reached by Tab, run by
    Enter, its region Repositories with the ring; Add opens Settings ›
    Repositories; Check all with two scheduled repositories (polling on)
    sends two `PollOnce` and none for a third that is checking; More opens
    its menu, and Refresh sends `IndexRefresh` for the chosen repository;
    refused behind a dialog. `with_no_repository` (`:170`) names the
    three: Add enabled, Check all disabled, More enabled with its items
    disabled (P12, P13).
  - Extend `menu_items_follow_the_state` with `more_items`.
  - E-12 part: `refresh-cw`, `ellipsis`, `plus` if not already loaded.
- [ ] **Step 3: Run them; they fail.**
- [ ] **Step 4: Implement.**
- [ ] **Step 5: Run** the tests by exact name.
- [ ] **Step 6: Verify lines; commit; push.**

---

## Task 6: The Details Block Leaves The Item Area (Part F)

**Files:**
- Modify: `D/view/sidebar.rs` (`selection`, `retry`, `remove`,
  `resolved`, `ShareMain` and unused constants go),
  `D/view/workspace/render.rs` (`:117-130`), `D/view/workspace/content.rs`
  (`repository_details` `:623-629`, `open_item` `:310`, `follow_tab`
  `:330`, `content_body` `:648-675`), `D/view/item.rs` (`end_region_with`,
  `:400`, `:544-547`, if nothing else uses it), `D/view/workspace.rs`
  (`remove_focus`, `retry_focus` if unused)
- Tests: `D/view/workspace_tests.rs` (E-1), `D/view/tabs_tests.rs` (E-4),
  `D/view/keyboard_tests/actions.rs` (E-5),
  `D/view/settings/repositories_manage_tests.rs` (E-6),
  `D/view/item/share_tests.rs` (E-7), `D/view/keyboard_tests/discussion.rs`
  (E-8)

**Interfaces:**
- Produces, in `D/view/workspace/content.rs`:
  `pub(crate) const OPEN_FROM_THE_LISTS: &str = "Open a ticket or document from the lists";`
  shown with no tab when a repository is chosen; `NOTHING_SELECTED`
  ("Select a repository.") when none (R-15).
- Removes: `sidebar::selection` and its two callers; `REPOSITORIES_HEADING`,
  `SELECTED`, `UNAVAILABLE`, `SHOWING_LAST_READ`, `SHARE_MAIN_BUTTON` when
  the sweep finds no user. Kept: `RETRY`, `SHARING`, `SHARE_MAIN`,
  `SHARE_MAIN_SET_UP`, `configuration_label` (fact 6).

- [ ] **Step 1: Write the failing tests.**
  - `desktop::view::workspace::tests::the_item_area_holds_nothing_of_the_repository`:
    no repository chosen → "Select a repository."; chosen, no tab →
    `OPEN_FROM_THE_LISTS`; a tab open → no `selection-` element anywhere
    in the window (P14).
  - E-1 rest: the six `desktop::view::workspace::tests` E-1 names, as E-1
    says (`selection-name` → the chosen row; `selection-unavailable*` →
    `("repo-state", i)`; `selection-retry` → the menu's Retry by
    `shift-f10`; configuration through `repository_tooltip`); the
    configuration-label and index assertions go (**Removes**).
  - E-4: `desktop::view::tabs::tests::opening_a_tab_does_not_change_the_selection`.
  - E-5: `desktop::view::keyboard_tests::actions::retry_an_unavailable_repository_by_keyboard`
    and `…::remove_an_unavailable_repository_by_keyboard`, with the
    `unavailable` helper, through `secondary-1` and `shift-f10`.
  - E-6: `desktop::view::settings::repositories::manage_tests::removing_a_registration_whose_folder_was_renamed_away_succeeds`
    and `…::the_sidebar_remove_of_a_registration_already_gone_lets_the_entry_go`
    through the menu; focus after the dialog on the list.
  - E-7: `desktop::view::item::share::tests::share_main_in_the_repository_detail_submits_repo_sync`,
    `…::share_main_without_a_remote_reads_share_main_and_opens_remotes`,
    `…::an_accepted_share_offers_stop_and_stopping_signals_it`,
    `…::a_stopped_share_says_what_completed_and_offers_share`,
    `…::share_opens_remotes_on_its_own_repository_after_another` choose
    Share main from the menu and read its label there;
    `share_main_at_the_end_of_an_item_tab_submits_repo_sync` is deleted
    (**Removes**).
  - E-8: `desktop::view::keyboard_tests::discussion::tab_order_runs_header_share_chips_page_discussion_composer`
    ends at the composer; its `SHARE_MAIN` constant and steps go
    (**Removes**).
- [ ] **Step 2: Run them; the new one and the edited ones fail** (the block
  is still drawn).
- [ ] **Step 3: Implement.** Re-run the Task 0 sweep's first command; it
  finds no production site.
- [ ] **Step 4: Run** every test of Step 1 by exact name.
- [ ] **Step 5: Verify lines; commit; push.**

---

## Task 7: Footer Status Items (Part G)

**Files:**
- Modify: `D/view/footer.rs`, `D/view/workspace/footer.rs`, `D/assets.rs`
  if an icon is missing
- Tests: `D/view/footer_tests/mod.rs` (E-10, new test),
  `D/view/keyboard_tests/shell.rs` and `shell/support.rs` (E-10)

**Interfaces:**
- Produces, in `D/view/footer.rs`:
  - `Control` gains `UpdateCheck` and `ToShare` (ids
    `footer-update-check`, `footer-to-share`); `ALL: [Control; 11]`.
  - `pub(crate) struct FooterStatus { pub update: Option<String>, pub to_share: usize, pub attention: usize }`
  - `pub(crate) fn plan(layout: &Layout, status: &FooterStatus) -> Plan`:
    right side `[[UpdateCheck?, ToShare?, Attention?], right dock group, [Settings]]`,
    empty groups dropped.
  - `pub(crate) fn update_text(checking: bool, latest: Option<OffsetDateTime>, now: OffsetDateTime) -> Option<String>`
    ("Checking for updates…", "Checked for updates {ago}", or `None`);
    `pub(crate) const UPDATE_TOOLTIP: &str = "Background check: fetch and fast-forward only. Never shares your changes.";`
    `pub(crate) fn to_share_name(n: usize, next: &str) -> String`
    ("{n} to share. Select {repo}.").
  - `pub(crate) fn next_to_share(order: &[(RepoKey, usize)], chosen: Option<&RepoKey>) -> Option<RepoKey>`
    (the next with work after `chosen`, wrapping).
- Produces, in `D/view/workspace/footer.rs`: `fn footer_status(&self, cx) -> FooterStatus`
  from the scheduler (`schedule().repo(root)` for every listed repository)
  and the worker; `press` gains `UpdateCheck` → `open_settings(SettingsPage::Polling)`
  and `ToShare` → `select_repository(next)` with focus kept on the control.
- Consumes: Task 1's `ago`, `AttemptSummary::to_share`.

- [ ] **Step 1: Check U-5.** Count the Tab presses `reach(Settings)` and
  `reach(Attention)` take today (`D/view/keyboard_tests/shell/support.rs:84`)
  and with two more stops; if past `TAB_LIMIT` (40), raise it in the same
  commit as a further edit (E-16 onward).
- [ ] **Step 2: Write the failing tests.**
  - `desktop::view::footer::tests::status_items_say_and_choose`: one table
    of `update_text` and `next_to_share` (none, one, two with the chosen
    among them, wrapping); `plan` with each `FooterStatus` (items in
    order, hidden when `None` or 0); then a window journey with polling on
    and two repositories with work (`schedule_mut()`): the two controls
    read their texts; Tab reaches both; Enter on To share chooses the next
    repository each time and focus stays; Enter on Update check opens
    Settings › Checking for updates; refused behind a dialog (P12, P15).
  - E-10: `desktop::view::footer::tests::a_secondary_click_opens_the_dock_menu_with_its_side_checked`,
    `…::toggle_names_follow_their_state`,
    `desktop::view::keyboard_tests::shell::each_side_is_reached_by_the_dock_menu_keys`,
    `…::footer_controls_are_reached_by_tab` (compares with the plan's
    controls), and `said` (`shell/support.rs:93`) gaining the two controls.
- [ ] **Step 3: Run them; they fail.**
- [ ] **Step 4: Implement.**
- [ ] **Step 5: Run** the tests of Step 2 and each user of `reach` the Task
  0 sweep listed, by exact name.
- [ ] **Step 6: Verify lines; commit; push.**

---

## Task 8: Privacy (Part I)

**Files:**
- Modify: `D/view/polling_privacy_tests.rs` (E-15)

- [ ] **Step 1: Settle U-3.** Add a fixture remote
  `ssh://git:sentinel-password-03a@example.invalid/r.git` as the
  publication remote (copy `a_remote_url_with_a_password_is_shown_redacted`,
  `D/view/settings/remotes_tests.rs:196`) and check whether `ssh_publication`
  schedules it. Record which branch of Part I applies.
- [ ] **Step 2: Write the failing test edit.** E-15:
  `desktop::view::polling_privacy_tests::polling_privacy` gains the
  surfaces `("repo-", "", "repository row")` and
  `("footer-", "", "footer")` in `SURFACES` (`:144`), the URL-password
  sentinel, and a sweep of `repository_tooltip`, `menu_items` labels and
  the footer's `update_text` and `to_share_name` for every repository,
  through their functions, against every sentinel (P17). Run it; it fails
  until the surfaces and sweep exist.
- [ ] **Step 3: Implement** only what the test finds (expected: nothing in
  production; the test proves it).
- [ ] **Step 4: Run** it by exact name.
- [ ] **Step 5: Verify lines; commit; push.**

---

## Task 9: Checklist, Gates, Review-Ready

- [ ] **Step 1: On-screen checklist.** Write
  `docs/plans/2026-11-12-wave-04-cycle-03a-on-screen-checklist.md` in
  Cycle 02's format, for the owner on a release build under Wayland
  against a scratch `XDG_DATA_HOME`: the panel against frames 1a and 1b
  in the dark theme; each state reached that the fixtures allow (This
  computer, To share, Up to date, Paused, Can't open by renaming a
  folder, Needs fixing); the menu by right click and by Shift F10 and the
  menu key; the toolbar and More by keyboard with focus visible; the
  footer's two items; the item area with no tab; tooltips; Orca reading
  a row and the menu. Claude records only what the owner reports.
- [ ] **Step 2: Gates**, once on the final tree: AGENTS.md's three static
  gates, then its fast tier with `--no-fail-fast`. No SSH target (nothing
  under its paths changed). A failure outside the E-list is a product bug
  (fixed), an appearance test (deleted under E-14) or a needed edit (E-16
  onward).
- [ ] **Step 3: Merge check.** `git merge-tree --write-tree HEAD origin/main`
  is clean; the Boundary command prints nothing.
- [ ] **Step 4: Fill** the Cycle's Acceptance Status column from the tests
  by exact name and the ledger; note where the code differs from the
  design for the owner's amendment.
- [ ] **Step 5: Commit, push, mark review-ready, ticket comment.** Not done
  without the owner: a PR, the merge, `ticket close`, cleanup, native CI,
  and comments on other tickets.
- [ ] **After review approval (the owner):** set the resolution status and
  close with `manyhands-cli ticket close --repo /home/vhodges/work/src/manyhands --id 01M784Y8C73B39GR4JWENXE5D0`
  from the registered main checkout, which merges the branch into main,
  publishes main and removes the branch and worktree (AGENTS.md). With the
  owner's go-ahead, since `comment add` publishes the branch it comments
  on: a note on `01M654G5VGQMXTH3BX9GKE7JEY` (vh-d78zh, folded in) and on
  `01M5CS4MH25QQ769HV61ZRKHXG` (its two repository-row items done).

---

## Plan Self-Review

**Required outcomes:**

| P | Task |
| --- | --- |
| P1, P2 | 2 |
| P3 | 1 (model, scheduler, Settings), 2 (state) |
| P4, P5 | 1, 3 |
| P6, P7 | 2, 3 |
| P8, P9 | 3 |
| P10, P11 | 2 (`menu_items`), 4 |
| P12 | 4, 5, 7 |
| P13 | 5 |
| P14 | 6 |
| P15 | 7 |
| P16 | 3 |
| P17 | 8 |

**Rulings:** R-1, R-2, R-3, R-4 → Task 1; R-5, R-6 → Task 2; R-7, R-8,
R-9, R-10 → Task 3; R-11, R-12 → Task 4; R-13, R-14 → Task 5; R-15, R-16
→ Task 6; R-17 → Task 2.

**Unverified:** U-6 → Task 1 Step 1; U-4 → Task 3 Step 1; U-1, U-2 →
Task 4 Step 1; U-5 → Task 7 Step 1; U-3 → Task 8 Step 1.

**Named edits:** E-9 → Task 1; E-2, E-3, E-1 (row part), E-12 (part) →
Task 3; E-13 → Task 4; E-11, E-12 (part) → Task 5; E-1 (rest), E-4, E-5,
E-6, E-7, E-8 → Task 6; E-10, E-12 (part) → Task 7; E-15 → Task 8; E-14
→ any task.

**Tests:** every test in the design's Testing table has its task, by the
same exact path: `finished_shares_are_heard` and `menu_items_follow_the_state`
were added to the design while this plan was drafted, and the
stored-outcome test is under `scheduler::entity_tests`, where the `World`
helper lives.

**Placeholder scan:** no "TBD", "TODO" or "similar to Task N". Names
marked "copy from" are the existing helpers to follow, not open choices.
