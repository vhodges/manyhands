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
> **Draft, revision 4:** not approved; nothing here is authorized until the
> owner approves the three documents and chooses the execution method.
> Re-read the design before each task and record every checkpoint on
> ticket `01M784Y8C73B39GR4JWENXE5D0`. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Rebuild the Repositories panel to `03-panels.md`: one
plain-language state per repository from D6's model, the row tooltip with
the Git detail, a toolbar, a context menu by pointer and keyboard, and the
footer's "Checked for updates …" and "N to share" items; take the
repository details block out of the item area.

**Architecture:** The scheduler keeps two more facts per repository (the
stored outcome and the shared copy); each repository keeps an unshared
list, set by completed checks and changed at once by this app's saves and
shares, which a per-root change log, timed when heard, keeps right when an
older check lands later (Part A's rules and table U1–U15). A
pure function turns them and the navigation entry into one state, its
action, its sentence, its tooltip and its menu (Part B). The panel draws
its own rows from those states, the kept row included, and names unshared
items by title (Part C); a Workspace-owned popup like Cycle 01's dock menu
carries the actions (Parts D, E); the details block goes (Part F); two
footer controls read the same model (Part G). No library change.

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
before that. The design's Document Amendments are applied by the planner
in the approval commit, not by a task.

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

**Boundary.** A task changes `src/desktop/` and its tests. After each
task's commit this prints nothing:

```sh
git diff --stat origin/main...HEAD -- src ':!src/desktop' tests Cargo.toml Cargo.lock packaging
```

A gap in the headless API stops the task for the owner.

**Main.** Merge `origin/main` at Task 0, at Task 6 and whenever main
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
  tests are not rerun. No whole-suite run before Task 10.
- A command past about 15 minutes is stopped, narrowed and noted.
- Never build while a test run is in progress.
- **Unverified facts** are checked with a scratch test that is discarded,
  not committed; its result is recorded in the ledger.

**Each task's last Verify lines:**

```sh
devenv shell -- cargo fmt --check
devenv shell -- cargo check --locked --features desktop
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

**Test-first.** Each behaviour's test is written and seen to fail before
its code. Model tests are table-driven (one table per function). Focus
assertions read focus inside the same update that causes the change
(Cycle 01 ledger). **Task 1 has a design checkpoint:** its implementer
writes `unshared_follows_attempts_shares_and_saves` from the design's
table U1–U15 before any of its code, and if a row cannot be expressed
against the code, stops and reports to the orchestrator, who settles it
with the design before implementation goes on.

**No appearance tests** (Wave Testing): nothing asserts colours, sizes,
icons, tones or that a view renders. Tooltip, description and menu
enablement are tested through their pure functions (facts 12, 43); window
tests read menu items by label and check that activating a disabled one
sends nothing.

**Test seams.** A stored outcome comes from `PollingBackend::store_outcome`
(Task 1); attempts, phases and attention from `schedule_mut()`
(`D/services/scheduler/entity.rs:228`), the scheduler `World`
(`D/services/scheduler_support.rs:24`) or `PollingBackend` answers
(`D/test_support/polling.rs:192-311`); reads are held with
`DelayingBackend::hold_read(kind)` and counted with `reads_seen(kind)`
(`D/test_support/delaying.rs:135`, `:268`). Window tests that need the
scheduler opt in with `Services::with_polling()` (fact 26), and **answer
every poll explicitly**: `answer_poll` for each launch poll and
`answer_job(JobKind::PollOnce, …)` or `hold_job` for each check, so no
test reaches `example.invalid` (fact 42). A pointer press behind a dialog
lands on its overlay, so guard tests use key presses or dispatched
actions, with a dialog and with a sheet (the item details sheet) (Cycle
01 ledger). Fixture data directories only; no network; no real waits;
`now` passed in. No `chmod` fixtures.

**Existing tests** change only as E-1 to E-16 say. Any other edit is
pre-authorized (decision 17): recorded in the ledger as E-17 onward with
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
Tasks 0, 5 and 10: commits, the tests run with counts (copied from the
ledger, AGENTS.md), rulings. Before `ticket save`, commit any owned
`ticket.md` edit. `comment add` publishes the ticket branch.

**Ledger.** `docs/plans/2026-11-12-wave-04-cycle-03a-repositories-execution.md`,
created in Task 0, written only by the orchestrator. Only the ledger
assigns E-numbers past E-16.

**File size.** New source files stay below about 500 lines.
`D/view/sidebar.rs` (576 lines) shrinks: its panel code moves to
`D/view/repositories.rs` in Task 3 and its details block goes in Task 7.

## Review Focus

Inputs no task's main journey exercises that would most likely bite a
person, each pinned by a test in the task named:

1. **Two facts at once.** A paused repository whose last check found work
   to share; a stored Locked while a Check now runs; a live Locked while
   a Check now runs: Part B's order decides (Task 2,
   `each_state_and_its_precedence`, pairs).
2. **A share that publishes but is not a success, or succeeds without
   publishing.** A `discovery_pending` `ItemSync` with publication
   `Published` removes the item; one that did not publish removes nothing
   (Task 1, `finished_shares_and_saves_are_heard`).
3. **A check that fails after one that found work.** The unshared list,
   the row and the footer keep the items; a share heard before an earlier
   check's answer stays shared (Task 1,
   `unshared_follows_attempts_shares_and_saves`).
4. **The menu for a row that is not the chosen one.** A right click on
   another row acts on that row's repository: its Retry reads that
   repository again, its Settings… opens on it, and the choice stays
   (Task 5, `the_menu_by_pointer_and_keyboard`).
5. **Remove while a sheet is open** is refused, and after the dialog focus
   is where the menu was opened, not the item area (Task 5).
6. **To share with the only repository with work already chosen**: nothing
   happens and nothing is read (Task 8, `status_items_say_and_choose`).

## File Map And Dependency Order

| Task | Part | Summary |
| --- | --- | --- |
| 0 | — | Merge main, baseline gates, re-check facts, E-sweep, ledger. |
| 1 | A | Stored outcome and shared copy in the scheduler; `main` on targets; `to_share` and `problems`; the unshared list; finished shares and saves, in order; `reread`; `Facts.stored` and its retry flag; the seam; E-9. |
| 2 | B | `repository_state`, `repository_tooltip`, `menu_items`, `more_items`; their tables. |
| 3 | C | The panel module: its own rows, the mark, the pointer hook's path, the kept row and its re-listing, the header count, deferred apply, list states; E-1, E-2 and E-3 row parts, E-12 part. |
| 4 | C | Titles of what is not shared: cache, `ShareTitles`, trigger, tries, at most five named. |
| 5 | D | The repository menu by pointer and keyboard; guards; a choice checked again; the shared Check now; Remove's focus; Settings per row; Retry by `reread`; E-13, E-16; checkpoint. |
| 6 | E | Toolbar buttons, the More menu, Check all; merge main; E-11, E-12 part. |
| 7 | F | The details block leaves the item area; E-1, E-2 and E-3 rest, E-4 to E-8. |
| 8 | G | Footer status items; E-10, E-12 part. |
| 9 | I | Privacy sweep; E-15. |
| 10 | — | On-screen checklist, gates, review-ready; checkpoint. |

New source files:

- `D/model/repository_state.rs` (with `mod repository_state;` in
  `D/model/mod.rs`)
- `D/view/repositories.rs` and `D/view/repositories/menu.rs` (with
  `mod repositories;` in `D/view/mod.rs`)

New test files: `D/model/repository_state_tests.rs`,
`D/view/repositories_tests.rs`. New tests in existing files are named in
their tasks.

Order: 0 → 1 → … → 10, one at a time.

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
  the owner: facts 10–16, 34 and 43 rest on the locked versions.
- [ ] **Step 3: Sweep the E-list,** over tests and production code alike:

```sh
P='src/desktop'
git grep -n -e 'selection-' -e 'sidebar::selection' -e 'repository_details' \
  -e 'remove_focus' -e 'retry_focus' -e 'SHARE_MAIN_BUTTON' -e 'selection-share-main' -- $P
git grep -n -e 'repo-row' -e 'repo-root' -e 'repo-selected' -e 'repo-unavailable' \
  -e 'repo-item' -e 'repositories-state' -e 'repositories-last-read' \
  -e 'repositories-failure' -e 'repositories-retry' -e 'REPOSITORIES_HEADING' \
  -e 'RepositoryRow' -e 'RepositoriesPane' -e 'sidebar::repositories' \
  -e 'remove_repository' -e 'RESOLVING' -e 'NOTHING_SELECTED' -- $P
git grep -n -e 'not_shared' -e 'AttemptSummary' -e 'status::Facts' -e 'Facts {' \
  -e 'Read::Polled' -e 'fn since_library' -e 'fn phase_status' -e 'fn after_last' \
  -e 'Heard::' -e 'ViewScope::' -e 'AttemptTarget {' -e 'TargetParts {' \
  -e 'item_id: None' -e 'fn check_now' -e 'take_vanished' -- $P
git grep -n -e 'plan(' -e 'Control::ALL' -e 'fn said' -e 'TAB_LIMIT' \
  -e 'ToolbarButton::ALL' -e 'toolbar_focus' -e 'toolbar_enabled' \
  -e 'with_no_repository' -e 'ICONS' -e '"Repositories"' -e 'ALLOWED' -- $P
```

  `Facts {` also matches the unrelated `retire_loop::Facts`
  (`D/model/polling/retire_loop.rs:70`); only `status::Facts` sites count
  (`D/model/polling/status_tests.rs`, `D/view/settings/polling.rs`).
  `not_shared` also matches `PillState::EditingSaved`, which is unrelated.
  Record each site in the ledger against the E-n or the task that changes
  it. A test site no E-n names is a pre-authorized edit (decision 17),
  recorded as E-17 onward; a production site no task names goes into the
  task that breaks it. List every user of `keyboard_tests.rs`'s
  `select_a_repository` helper (unchanged, E-3) and of `shell/support.rs`'s
  `reach` and `said` (E-10).
- [ ] **Step 4: Ledger, commit, push, ticket comment** recording the base
  SHA.

---

## Task 1: Sharing Facts The Scheduler Keeps (Part A)

**Files:**
- Modify: `D/model/polling/attempt.rs`, `D/model/polling/state.rs`,
  `D/model/polling/table.rs`, `D/model/polling.rs`,
  `D/model/polling/status.rs`, `D/services/poll_convert.rs` (`main`),
  `D/services/scheduler.rs` (re-export), `D/services/scheduler/registration.rs`,
  `D/services/scheduler/entity.rs`, `D/services/scheduler/react.rs`,
  `D/services/scheduler/retire_loop.rs` (`Scheduler::checked_now`,
  `:149-165`, Check now's path), `D/services/scheduler/reconfigure.rs`,
  `D/view/settings/polling.rs`, `D/test_support/polling.rs`,
  `D/view/polling_window.rs` (an `AttemptTarget` literal, fact 66)
- Tests: `D/model/polling_schedule_tests.rs` (E-9, new tests),
  `D/model/polling/status_tests.rs` (E-9),
  `D/view/settings/polling_tests.rs` (E-9),
  `D/services/scheduler/reconfigure.rs` (its `mod tests`),
  `D/services/scheduler_entity_tests.rs`; and, for `main` on every
  `TargetParts` and `AttemptTarget` literal (E-9, fact 66):
  `D/services/scheduler_consent_tests.rs`, `D/services/poll_convert_tests.rs`,
  `D/services/scheduler_loop_tests.rs`, `D/view/polling_privacy_tests.rs`,
  `D/model/polling/entry_tests.rs`, `D/model/polling/retire_loop_tests.rs`,
  `D/model/polling_retiring_tests.rs`, `D/services/scheduler_retire_tests.rs`,
  `D/view/operations/attempts_tests.rs`, `D/view/operations/retire_real_tests.rs`,
  `D/view/operations/retire_tests.rs`, `D/view/operations/retiring_tests.rs`

**Interfaces:**
- Produces, in `D/model/polling/attempt.rs`:
  - `AttemptTarget` gains `pub main: bool`. `TargetParts`
    (`D/services/poll_convert.rs:40-44`) gains the same field, set at `:66`
    from the report's target (`target.target().is_some() &&
    target.kind().is_none()`, fact 51) and carried into `AttemptTarget` at
    `:459-460`; a Check now's data sets it at `:249` from the DTO's
    `target` and `kind`. Every `TargetParts` and `AttemptTarget` literal
    (fact 66) gains it.
  - `AttemptSummary` gains `pub to_share: Vec<AttemptTarget>` (every
    `LocalAhead`, `Unpublished` and `Diverged` target, main included) and
    `pub problems: Vec<AttemptTarget>` (every `Dirty`, `Conflicted`,
    `Inaccessible`, `Nonconforming`, `BlockedByLocalState`,
    `MaterializationIncomplete`, `Malformed`, `RemotelyDeleted`,
    `HistoryUnknown` and `NotReached` target). The `not_shared: u32` field
    goes and no method replaces it (R-44): its only production reader,
    `checked` (`D/model/polling/status.rs:166`), reads the unshared list.
    Every target in `listed` today stays there; `LocalAhead` still counts
    in `up_to_date`.
- Produces, in `D/model/polling/state.rs`:
  - `pub(crate) struct Unshared { pub items: Vec<AttemptTarget>, pub problems: Vec<AttemptTarget>, pub as_of: Option<OffsetDateTime> }`
    and `RepoSchedule.unshared: Unshared` (empty at `new`).
- Produces, in `D/model/polling/table.rs` and `D/model/polling.rs`, the
  design's rules 1–5 (Part A; R-31), in one place both attempt paths call:
  - A per-root log kept in `PollSchedule` for the session, `changes:
    BTreeMap<Root, Vec<(Change, OffsetDateTime)>>`, with `enum Change {
    Shared { item: Option<ItemId>, main: bool }, Saved { item: ItemId,
    kind: Option<TargetKind>, new: bool }, MainAhead }`, each entry timed
    when the scheduler heard it (rule 1).
  - `fn unshared_after(&mut self, root: &Root, attempt: &AttemptSummary)`:
    for an `Answer::Completed` attempt with targets whose `completed_at`
    is not older than `unshared.as_of` (rule 4), sets the list from the
    attempt's `to_share` and `problems` and `as_of` from its
    `completed_at`, then replays, per item, the last change in the log
    timed after that `completed_at` (rule 3); any other attempt leaves the
    list (facts 49, 50). `apply` calls it, and so does `checked_now`'s
    in-flight branch (`D/model/polling.rs:284-296`, fact 64; rule 5).
  - `pub fn shared(&mut self, root: &Root, item: Option<&ItemId>, main: bool, at: OffsetDateTime) -> bool`:
    logs the share and removes at once (rule 2) the item (by `item_id`)
    and, when `main`, the target with `main: true` from
    `repo.unshared.items` and from `repo.last`'s `listed`; true when one
    went.
  - `pub fn saved(&mut self, root: &Root, item: ItemId, kind: Option<TargetKind>, new: bool, at: OffsetDateTime) -> bool`:
    logs the save and adds the item at once to `repo.unshared.items`
    (`Unpublished` when `new`, else `LocalAhead`) unless it is there, also
    before any completed check; false when the root has no schedule
    (decision 24).
  - `pub fn main_ahead(&mut self, root: &Root, at: OffsetDateTime) -> bool`:
    logs and adds main as `LocalAhead` (R-45).
  - `Unshared` and `problems` carry `#[allow(dead_code)] // Used from Task 3`
    until the rows read them.
- Produces, in `D/model/polling/status.rs`:
  - `Facts` gains `pub stored: Option<PollingOutcome>`; `phase_status` and
    `after_last` take `&Facts`.
  - `pub(crate) fn stored_applies(facts: &Facts) -> Option<PollingOutcome>`:
    `facts.stored` only when checking is enabled, the repository is not
    held, not paused and its shape is `Polling`, `last.is_none()`, not
    `checking_now` or `in_flight()`, the phase `Idle`, `Stopped` or
    `Waiting { why: NotDue }`, and no conflict pending (facts 53, 63;
    R-32). `since_library`, `repository_state` (Task 2) and Check all
    (Task 6) use it and nothing else.
  - `since_library` returns `(String, bool)`, the bool a retry flag, and
    gives Part A's Settings line for what `stored_applies` returns, before
    "Checked {ago}" or "Not checked yet".
  - Every Settings line appends "· {n} not shared yet" from
    `repo.unshared.items.len()` when it is not 0, whatever the status
    (decision 24; P44); `checked` no longer reads `not_shared`.
  - `pub(crate) fn ago(then: OffsetDateTime, now: OffsetDateTime) -> String`
    (was private, `:134`).
- Produces, in `D/services/scheduler/registration.rs`, re-exported from
  `D/services/scheduler.rs`:
  - `pub(crate) struct SharedCopy { pub name: String, pub location: String }`
    (`location` is the redacted `push_location` of the remote with
    `selected_for_publication && publication_eligible`); its fields carry
    `#[allow(dead_code)] // Used from Task 3`.
  - `Read::Polled` gains `outcome: Option<PollingOutcome>` and
    `shared_copy: Option<SharedCopy>`, filled in `read` (`:101-120`).
- Produces, in `D/services/scheduler/entity.rs`:
  - fields `pub(super) stored: BTreeMap<Root, PollingOutcome>` and
    `pub(super) shared_copies: BTreeMap<Root, SharedCopy>`, set in
    `registered` (`:342-365`) beside `authorities`, refreshed in `answered`
    (`D/services/scheduler/react.rs:41`) from the status it reads, and
    removed where `authorities.remove(root)` is (`:390`).
  - `pub(crate) fn stored(&self, root: &Root) -> Option<PollingOutcome>`,
    `pub(crate) fn shared_copy(&self, root: &Root) -> Option<&SharedCopy>`
    (`shared_copy` carries `#[allow(dead_code)] // Used from Task 3`).
  - `pub(crate) fn reread(&mut self, root: Root, cx: &mut Context<Self>)`:
    returns at once for a root not in `known` (`:88`, R-41); otherwise
    drops it from `failed_reads` and calls
    `read_registrations(vec![root], cx)` (`:312`)
    (`#[allow(dead_code)] // Used from Task 5`).
  - `Heard::Shared`, `Heard::Saved` and `Heard::MainAhead` handled in
    `heard` (`:482-521`) at once, in arrival order, with
    `schedule.shared(…)`, `schedule.saved(…)` or `schedule.main_ahead(…)`
    timed at the scheduler's `utc()` when heard (rules 1, 2), then
    `cx.notify()` when one changed. An older automatic answer applied
    later through `answers` (`react.rs:18-36`), and a Check now handled in
    `heard` itself (`:510`, fact 64), go through `unshared_after`, so the
    log keeps them right (rules 3–5).
- Produces, in `D/services/scheduler/reconfigure.rs`:
  - `Heard::Shared { root: Root, item: Option<ItemId>, main: bool }`;
    `heard` returns it, before `scope_of`, for `WorkerEvent::Finished` whose
    `envelope.effects.publication` is `PublicationEffect::Published` or
    `Current`: `Intent::ItemSync` or `ItemRepublish` (`item: Some(id)`,
    `main: false`), `TicketClose` or `DocumentPromote` (`item: Some(id)`,
    `main: true`), `RepoSync` (`item: None`, `main: true`), the drain's
    predicate (`D/services/mod.rs:636-652`) with `RepoSync` added.
  - `Heard::Saved { root: Root, item: ItemId, kind: Option<TargetKind>, new: bool }`
    for `WorkerEvent::Finished` whose envelope names an item in
    `envelope.scope.item_id` (`L/results.rs:104`, `:293-299`), whose
    `envelope.effects.checkpoint` is `CheckpointEffect::Committed` and
    whose publication is neither `Published` nor `Current`: `DocumentSave`,
    `TicketSave`, `DocumentCreate`, `TicketCreate` (`new: true`),
    `DocumentMove`, `CommentAdd` and `LocalContextSync` (fact 59, `kind`
    from the intent) and `LocalContextResolve` (`kind: None`; U-7).
  - `Heard::MainAhead { root: Root }` for a `TicketClose` or
    `DocumentPromote` whose `effects.integration` is
    `IntegrationEffect::Complete` and whose publication is `Pending`
    (`L/results.rs:76-87`; R-45).
- Produces, in `D/test_support/polling.rs`:
  - `pub(crate) fn store_outcome(&self, root: &str, outcome: PollingOutcome)`
    on `PollingBackend`; its `polling_status` (`:404`) returns the
    library's answer with `latest_outcome` replaced for that root.
- Consumes: `PollingOutcome` (`L/repository/read/dto.rs:689`),
  `RemoteListDto`/`RemoteDto` (`:200`), `PublicationEffect`,
  `IntegrationEffect`, `CheckpointEffect` (`L/results.rs:63-87`), the
  envelope's `scope` (`:104`, `:293-299`).

- [ ] **Step 1: Check U-6.** Read
  `desktop::services::scheduler::attention_tests::unlock_after_cancelled_prompt`
  (`D/services/scheduler_attention_tests.rs:227`): a user-lane `PollOnce`
  prompts through the worker's provider. Record in the ledger whether this
  settles U-6. If it does not, stop for the owner (decision 9).
- [ ] **Step 2: Design checkpoint: the unshared list's table first.**
  Write `desktop::model::polling::tests::schedule::unshared_follows_attempts_shares_and_saves`
  before any of this task's code: one table with a row for each of the
  design's U1 to U15 (Part A), each a sequence of attempts (`Completed`
  with targets, `Transport(Unreachable)`, `Interrupted`, `Cancelled`, a
  `Completed` replay with none, one older than `as_of`, a Check now
  through `checked_now` with and without a poll in flight) and of
  `shared`, `saved` and `main_ahead` calls with their heard times, and the
  list and `as_of` asserted after each step; plus a save of an item
  already listed (once) and a save on a root with no schedule (nothing)
  (P30–P33, P44). If any row cannot be expressed against the code (an
  answer or a path the model does not have), **stop and report to the
  orchestrator** with the row and why; the orchestrator settles it with
  the design before implementation goes on. Record the outcome in the
  ledger.
- [ ] **Step 3: Write the other failing tests.**
  - `desktop::model::polling::tests::schedule::to_share_counts_and_finished_shares`,
    one table: an attempt with `LocalAhead` (ticket), `Unpublished`
    (document), `Diverged` (ticket), `Dirty` (ticket), main `LocalAhead`
    (`main: true`), an item with an unparseable ID (`item_id: None`,
    `main: false`) and `Current`; `to_share.len()` 5 and `problems.len()` 1; then
    `shared` with the first ticket's ID → true, count 4, gone from
    `listed` too; again → false; an unknown ID → false; the second ticket
    with `main` → both gone; on a root with no schedule → false (P4, P5,
    P19).
  - `desktop::services::scheduler::reconfigure::tests::finished_shares_and_saves_are_heard`,
    one table of `heard` over `WorkerEvent::Finished` envelopes:
    `ItemSync` `Success` with publication `Published` → `Shared { item:
    Some, main: false }`; `ItemSync` `Partial` (`discovery_pending`) with
    `Published` → the same; `ItemSync` `Error` with `NotRequested` →
    `None`; `ItemRepublish` with `Current` → `Shared { main: false }`;
    `TicketClose` and `DocumentPromote` with `Published` → `Shared { main:
    true }`; `RepoSync` with `Published` → `Shared { item: None, main:
    true }`; each authoring intent of fact 59 with checkpoint `Committed`
    and publication `NotRequested` or `Pending` → `Saved` with its kind,
    `new` for the two creates; a `LocalContextResolve` whose envelope's
    `scope.item_id` names an item → `Saved { kind: None }` (U-7: if its
    envelope names none, record it and move it to K-1); a `TicketSave`
    with `Published`, and one with checkpoint `Unchanged`, → `None`; a
    `TicketClose` with integration `Complete` and publication `Pending`
    → `MainAhead`; `PollOnce` unchanged (P5, P32, P33). Build requests as
    `resumed_and_abandoned_retirements_are_heard` (`:354`) does.
  - `desktop::services::scheduler::entity_tests::the_stored_outcome_and_shared_copy_are_kept`:
    `World::new(true, cx)`, `ssh_repo("a")`, `store_outcome(a,
    TransportUnavailable)`, `list`, with the launch poll answered by
    `answer_poll` or held (`D/test_support/polling.rs:458` would reach
    the library); then `stored(a)` and `shared_copy(a)` (name `origin` and
    a location; copy the remote set-up from `ssh_repo`,
    `D/services/scheduler_support.rs:71`); `store_outcome(a, Completed)`
    and `activate` → replaced; the folder renamed so the registration read
    fails, then renamed back and `reread(a)` → read again (count the
    reads); `reread` of a root never listed → no read; the repository
    removed (listed without it) → both gone (P3, P26, P42).
  - E-9: `desktop::model::polling::tests::schedule::attempt_summary_counts`
    expects `to_share.len()` 3 and `problems.len()` 10 in each case (its
    fixture lists every disposition once; R-44);
    `desktop::model::polling::status::tests::status_text_table` gains
    `stored: None` on every `status::Facts` and rows for each stored
    outcome of Part A's table with its retry flag, with no `last`, while
    checking, for a paused, an off and a held repository, and with a busy
    wait, `Waiting { why: Unfinished }` and a pending conflict, where the
    stored outcome does not apply (P3, P34, P43); "· N not shared yet"
    follows the unshared list, also before any completed check (P44);
    `desktop::view::settings::polling_tests::settings_polling_rows`
    follows if a text it pins changed; every `TargetParts` and
    `AttemptTarget` literal in this task's test files gains `main` (fact
    66).
- [ ] **Step 4: Run them; they fail** (fields, methods and variants
  missing).
- [ ] **Step 5: Implement** the interfaces above. `D/view/settings/polling.rs`'s
  `row` (`:379-399`) passes `stored: scheduler.stored(root)` and offers
  Retry now when `since_library`'s flag says so.
- [ ] **Step 6: Run** each test of Steps 2 and 3 by exact name.
- [ ] **Step 7: Verify lines; commit; push.**

---

## Task 2: Repository State (Part B)

**Files:**
- Create: `D/model/repository_state.rs`, `D/model/repository_state_tests.rs`
- Modify: `D/model/mod.rs`

**Interfaces:**
- Produces, in `D/model/repository_state.rs`:
  - `pub(crate) struct RepoFacts<'a> { pub entry: &'a RepositoryEntry, pub kept: bool, pub schedule: Option<&'a RepoSchedule>, pub unplaced: Option<Unplaced>, pub holding: Option<Holding>, pub stored: Option<PollingOutcome>, pub shared_copy: Option<&'a SharedCopy>, pub checking_now: bool, pub sharing_main: bool, pub polling: bool, pub unlock_target: bool, pub titles: &'a dyn Fn(&ItemId) -> Option<String>, pub now: OffsetDateTime }`
  - `pub(crate) enum StateKind { CantOpen, BusyOpening, NeedsFixing, Checking, Locked, NeedsAttention, CantReach, ToShare(usize), Paused, UpToDate, ThisComputer, NotSsh, Unknown }`
    (Part B's rows 1–13, in order).
  - `pub(crate) enum Tone { Failure, Warning, Working, Muted, Success, Secondary }`
  - `pub(crate) enum RowAction { Retry, Reread, Unlock(KeyId), Attend(Action, Reason), CheckNow, RetryNow, Settings, SetUpSharing }`
    (`Action` and `action_of` are already in `D/model/polling/attention.rs:20`, `:46`).
  - `pub(crate) struct RepoState { pub kind: StateKind, pub label: Option<String>, pub icon: Option<&'static str>, pub tone: Option<Tone>, pub action: Option<RowAction>, pub sentence: Vec<String> }`
  - `pub(crate) fn repository_state(facts: &RepoFacts<'_>) -> RepoState`
  - `pub(crate) fn repository_tooltip(facts: &RepoFacts<'_>, state: &RepoState) -> Vec<String>`
    (Part B's lines 1–4).
  - `pub(crate) enum MenuChoice { Sentence, StateAction, CheckNow, ShareMain, OpenFolder, Settings, Remove, Refresh, Rebuild }`
  - `pub(crate) struct MenuItem { pub choice: MenuChoice, pub label: String, pub enabled: bool }`
  - `pub(crate) fn menu_items(facts: &RepoFacts<'_>, state: &RepoState) -> Vec<MenuItem>`
    (Part D's table: the sentence first, disabled; the state's action;
    Check now or Retry now unless it is the state's action; Share main,
    "Share main…" or "Sharing…"; Open folder; Settings… unless it is the
    state's action; Remove from Manyhands…; with Part D's enablement).
  - `pub(crate) fn more_items(chosen: Option<&RepoKey>) -> Vec<MenuItem>`:
    Refresh, Rebuild the search index, Remove from Manyhands…, each
    disabled with nothing chosen.
  - Copy constants beside them: `CANT_OPEN`, `NEEDS_FIXING`, `LOCKED`,
    `NEEDS_ATTENTION`, `CHECKING` (D6's `status::CHECKING`), `CANT_REACH`,
    `PAUSED` (D6's), `UP_TO_DATE`, `THIS_COMPUTER`, `fn to_share(n)`, and
    Part B's sentences (the Locked one is D6's `headline` with a full
    stop, fact 54; the to-share one names at most five items, then "and
    {n} more") and Part D's labels; `SHARE_MAIN`, `SHARE_MAIN_SET_UP` and
    `SHARING` are `sidebar.rs`'s (fact 6).
- Consumes: Task 1's `SharedCopy`, `Facts.stored`, `status::stored_applies`
  (the only test of whether a stored outcome applies; R-32),
  `RepoSchedule.unshared`, `status::row`, `status::ago` (Up to date's
  "Checked {ago} · up to date" from `unshared.as_of`, R-43), `cant_reach`,
  `attention::headline`.

- [ ] **Step 1: Write the failing tests.**
  - `desktop::model::repository_state::tests::each_state_and_its_precedence`,
    one table: each of Part B's rows 1–13 alone (each stored outcome of
    Part A among them, the kept row, `Unplaced::Unreadable`,
    `Resolution::Busy`, `Shape::Held`, `Transport(Unavailable)`, problems,
    a non-SSH remote, polling off, a pending conflict, `Waiting { why:
    Failed }`, `Waiting { why: Unfinished }`, checking off); then pairs:
    Can't open with Locked; Can't open with `Unreadable` (Can't open, its
    Retry `retry_resolution`); Needs fixing with To share; stored
    `UnlockRequired` with `checking_now` (Checking…); live `KeyLocked`
    with `checking_now` (Checking…); Checking… with Can't reach (decision
    8); To share with problems (Needs attention); To share with Paused; a
    paused, an off and a held repository with a stored
    `TransportUnavailable` (Paused, no label, Needs attention); To share
    with `Waiting { why: Unfinished }` (no label); Paused with Up to date;
    a stored outcome with a live attempt (the attempt wins); an unshared
    list kept after a failed attempt (To share); a stored
    `TransportUnavailable` with a busy wait, with `Waiting { why:
    Unfinished }` and with a pending conflict (the stored outcome does
    not apply: Checking soon's no label, no label, Needs attention for the
    conflict). Assert `kind`, `label` and `action`; every kind with a tone
    has a label (P1–P3, P18–P20, P34, P35, P43).
  - `desktop::model::repository_state::tests::tooltips_say_the_state_and_the_facts`,
    one table of `repository_tooltip`: each state's sentence; with and
    without a primary branch and a shared copy; To share with titles
    known, unknown (ID), with no kind (ID), main alone, items and main, and
    seven items (five named, "and 2 more"); Up to date's "Checked {ago} ·
    up to date" from `as_of` while `last` is an interrupted attempt (P6,
    P7, P36).
  - `desktop::model::repository_state::tests::menu_items_follow_the_state`,
    one table of `menu_items` and `more_items`: the sentence first and
    disabled, absent when empty; the state's action next, not repeated
    below; Check now disabled unless D6's shape is `Polling` and not
    checking; Retry now under Can't reach, "Can't check" and their stored
    forms; Check now for a stored key or host outcome; Retry (`Reread`)
    under `Unreadable`, with Open folder enabled; "Share main…" with no
    remote; "Sharing…" disabled; Share main and Open folder disabled under
    Can't open from a resolution or the kept row; Share main disabled
    under a non-SSH remote; Unlock… disabled without an unlock target;
    `more_items` with and without a choice (P10, P13, P20, P21, P26).
- [ ] **Step 2: Run them; they fail.**
- [ ] **Step 3: Implement** the interfaces. The first row of Part B that
  holds wins. No I/O; `now` from the facts.
- [ ] **Step 4: Run** the three tests by exact name.
- [ ] **Step 5: Verify lines; commit; push.** Items Task 3 first uses carry
  `#[allow(dead_code)] // Used from Task 3`.

---

## Task 3: Rows And List States (Part C)

**Files:**
- Create: `D/view/repositories.rs`, `D/view/repositories_tests.rs`
- Modify: `D/view/mod.rs`, `D/view/sidebar.rs` (its panel code moves out),
  `D/view/workspace.rs`, `D/view/workspace/render.rs`,
  `D/view/workspace/repositories.rs`, `D/view/dock/panels.rs` (the header
  count, R-20), `D/model/navigation.rs` (a re-listed kept entry, R-37),
  `D/assets.rs`
- Tests: `D/view/workspace_tests.rs` (E-1, E-2, E-3 row parts),
  `D/view/settings/repositories_manage_tests.rs` (E-2),
  `D/assets_tests.rs` (E-12 part)

**Interfaces:**
- Produces, in `D/view/repositories.rs` (moved from `sidebar.rs`, `:69-340`):
  - `pub(crate) struct RepositoryRow { pub key: RepoKey, name: String, state: RepoState, tooltip: Vec<String>, menu: Vec<MenuItem>, chosen: bool, kept: bool }`,
    with `#[cfg(test)]` accessors for the tooltip and menu (Task 9).
  - `pub(crate) fn rows(workspace: &Workspace, cx: &App) -> Vec<RepositoryRow>`
    (builds `RepoFacts` per listed entry and for the kept entry from
    `Navigation`, the scheduler, `WorkerHandle::in_flight`
    (`D/services/worker.rs:744`) and `sharing_main`).
  - `RepositoryDelegate` with the panel's own row element (not `ListRow`,
    R-7): `("repo-row", i)` labelled `{repo}`, description and tooltip
    from `repository_tooltip`; the icon, the name, and the state at the
    right with id `("repo-state", i)`; for the chosen row the selected
    treatment and the mark `("repo-selected", i)` labelled `SELECTED`; the
    focus ring on the Kit cursor's row while the list has focus; no
    `aria_selected`. The delegate holds a `WeakEntity<Workspace>` and an
    `Rc<RefCell<HashMap<RepoKey, Bounds<Pixels>>>>` shared with the
    Workspace (R-36); a `canvas` in each row records its bounds there
    during prepaint (both `#[allow(dead_code)] // Used from Task 5`).
    `render_empty` is overridden to draw nothing.
  - `RepositoriesPane` as today, plus `pub(crate) fn rows(&self, cx: &App) -> Vec<RepositoryRow>`
    for the comparison.
  - `pub(crate) fn body(navigation: &Navigation, pane: &RepositoriesPane, cx: &mut Context<Workspace>) -> impl IntoElement`
    with Part C's list-state table (ids `repositories-loading`,
    `repositories-empty`, `repositories-alert`; the alert's action is
    `("repositories-alert", "action")`), and the `sidebar-settings` button
    kept (decision 15). No in-body heading.
  - Copy: `LOADING_REPOSITORIES`, `NO_REPOSITORIES`, `ADD_OR_CREATE`
    ("Add or create a repository…", shared with Task 6),
    `NOT_UPDATED_BUSY`, `LIST_INCOMPLETE`.
- Produces, in `D/model/navigation.rs`: `apply_repositories` (`:289-299`)
  gives a listed entry that was the kept entry `Resolution::Unknown` and
  sets `vanished`, so `take_vanished` (`:339-344`) gives the chosen
  repository to be resolved again (fact 68; R-37).
- Removes: `REPOSITORIES_HEADING` and `SHOWING_LAST_READ` from
  `sidebar.rs`, which the new body no longer draws and E-2's row part no
  longer reads (`workspace_tests.rs:923`), so clippy stays clean.
- Changes: `Workspace::render` no longer calls `RepositoriesPane::set`
  (`render.rs:205-206`); when `repositories::rows` differ from the pane's,
  it applies them with `window.defer` (R-8). `panels::count` for
  Repositories (`D/view/dock/panels.rs:171-177`) counts the rows drawn,
  the kept row included (R-20).

- [ ] **Step 1: Check U-4** with a scratch test (discarded): render twice
  with a row change between and assert the pane holds the new rows after
  one `run_until_parked`. If not, stop: rows are pushed from the scheduler
  observer (`D/view/workspace.rs:215`, as `observe_in`) and the navigation
  updates instead; record the ruling.
- [ ] **Step 2: Write the failing tests.**
  - `desktop::view::repositories::tests::rows_and_list_states`, a window
    journey with polling on (copy the opt-in from
    `D/view/polling_window.rs:74`), answering each launch poll with
    `answer_poll`: two repositories; through `schedule_mut()`, each gets
    a `Completed` attempt applied by `apply` (or `checked_now`), so the
    unshared list follows Task 1's rules: one with two `to_share` items,
    the other whose only target is main `Current` (main is always
    reported, fact 70); the rows' `("repo-state", i)` read "2 to share"
    and "Up to date"; a
    finished `ItemSync` that published one (`answer_job`, as the share
    tests do) → "1 to share" with no list read; the chosen repository's
    registration removed by a second actor and the list reloaded → a kept
    row last, reading "Can't open", the header counting it; the repository
    registered again and the list reloaded → a resolve read is sent
    (`reads_seen(ReadKind::Resolve)` grows) and the row no longer reads
    "Can't open" once it lands (P38); a finished `TicketSave` that
    made a checkpoint without publishing (`answer_job`) → "2 to share"
    again with no list read (P33); a finished `DocumentSave` in a
    scheduled repository with no completed check yet → "1 to share" (P44);
    the first load
    shows `repositories-loading`; a held busy first load shows the busy
    alert; with no repositories, `repositories-empty` and its action opens
    Settings › Repositories; an incomplete list, built by applying a
    constructed `RepositoryListDto { complete: false, … }` through
    `Navigation::apply_repositories` (the library never answers one,
    fact 57), shows its alert; a busy
    and a failed reload keep the rows under the alert, and Retry reads
    again; click and Enter choose a row (its `("repo-selected", i)` mark
    present), arrows move the cursor only, and the row's `selected()` is
    `None`; after a model change the pane holds the new rows once the
    frame has run (P5, P8, P9, P16, P18, P33, P38).
  - E-1, row part: `desktop::view::workspace::tests::a_renamed_away_repository_is_unavailable_until_retry_finds_it_again`
    reads `("repo-state", i)` "Can't open" where it read
    `("repo-unavailable", i)` (`:776`, `:785`); the `selection-*` parts
    stay until Task 7.
  - E-2, row part: `desktop::view::workspace::tests::a_busy_list_reload_keeps_the_last_rows_with_a_label`
    and `desktop::view::settings::repositories::manage_tests::tabs_close_once_the_list_no_longer_gives_their_root`
    use `repositories-alert`; the first test's `selection-name` read
    (`:927`) stays until Task 7.
  - E-3, row part: `desktop::view::workspace::tests::the_fixtures_repositories_are_listed_by_name`
    drops `("repo-root", 0)`. `the_repository_selector_selects_with_arrows_and_enter`,
    `a_repository_rows_selected_state_has_one_meaning` and the
    `select_a_repository` helper are unchanged here (the mark keeps its
    id and label).
  - E-12 part: `desktop::assets::tests::every_item_view_icon_loads` gains
    each of `folder-git-2`, `cloud-upload`, `circle-check`, `circle-alert`,
    `hard-drive`, `pause`, `lock`, `loader-circle`, `triangle-alert` that
    `DesktopAssets.load(&icon_path(name))` does not already load (check
    each first; a Kit icon needs no entry).
- [ ] **Step 3: Run them; they fail.**
- [ ] **Step 4: Implement.** Move the panel code out of `sidebar.rs`;
  `sidebar.rs` keeps `selection` (until Task 7) and the constants other
  files import (fact 6).
- [ ] **Step 5: Run** every test of Step 2 by exact name, the unchanged
  `a_repository_rows_selected_state_has_one_meaning` and
  `the_repository_selector_selects_with_arrows_and_enter` (the row element
  changed under them), and each user of `select_a_repository` the Task 0
  sweep listed.
- [ ] **Step 6: Verify lines; commit; push.**

---

## Task 4: Titles Of What Is Not Shared (Part C)

**Files:**
- Modify: `D/view/repositories.rs`, `D/view/workspace.rs`,
  `D/view/workspace/lists.rs` (titles from loaded lists),
  `D/services/mod.rs` (`ReadKind::ShareTitles`), `D/model/stamp.rs`
  (`ViewScope::ShareTitles(RepoKey)`)
- Tests: `D/view/repositories_tests.rs`

**Interfaces:**
- Produces, on `Workspace`: `titles: HashMap<(RepoKey, ItemId), String>`,
  `title_tries: HashMap<(RepoKey, ItemId), Tries>` (`enum Tries {
  Pending(u8), Done }`) and `title_reads: HashMap<RepoKey, Task<()>>`;
  `fn learn_titles(&mut self, root: &RepoKey, rows)` called when the
  Tickets or Documents lists load; `fn read_share_titles(&mut self, cx)`
  called from the scheduler observer (`D/view/workspace.rs:215`): for each
  repository with no task in `title_reads` (a running job is never
  replaced: dropping its task cancels the read, fact 65) whose named
  unshared items (at most five, R-9) include IDs that have a kind and are
  neither in `titles` nor `Done`, with fewer than three tries, one read
  job, copying `D/view/lifecycle/made_ready.rs:133-171`
  (`resolve_repository`, then `show_item` per ID), under
  `ReadKind::ShareTitles`, its task kept in `title_reads` for that root
  (fact 58). The stamp, under `ViewScope::ShareTitles(root)`, is the one
  `read_job` requires; every answer is merged by (root, id) whatever it
  says (R-9). When the answer is applied the root's entry leaves
  `title_reads`. A found item gets its title and `Done`; a not-found item
  `Done`; only a busy or failed answer counts one try. An item with no
  kind (a local conflict resolution's, fact 69) is not read and is named
  by its ID. The three maps drop a repository's entries when it is
  removed.
- Changes: `repositories::rows` passes `titles` lookups into `RepoFacts`.

- [ ] **Step 1: Write the failing test.**
  `desktop::view::repositories::tests::titles_of_what_is_not_shared`, a
  window journey with polling on and every poll answered: one unshared
  item whose title the loaded Tickets list gives (no read); one in another
  repository read under `ShareTitles` (held with `hold_read`, then
  answered) and named by title after; one whose read is busy, read again
  at the next scheduler notify, and after three busy tries named by its ID
  and not read again (`reads_seen`); scheduler notifies while a read is
  held start no second job for that repository and use no tries
  (`reads_seen` unchanged, the item still read once the hold is released);
  an item with no kind named by its ID and not read; seven unshared
  items, five read; titles checked through `repository_tooltip` on the
  workspace's facts (P6, P36).
- [ ] **Step 2: Run it; it fails.**
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** it by exact name.
- [ ] **Step 5: Verify lines; commit; push.**

---

## Task 5: The Repository Menu (Part D)

**Files:**
- Create: `D/view/repositories/menu.rs`
- Modify: `D/view/repositories.rs` (the row's pointer hook), `D/keys.rs`,
  `D/view/workspace.rs`, `D/view/workspace/render.rs`,
  `D/view/workspace/focus.rs`, `D/view/workspace/requests.rs`
  (`remove_repository`), `D/view/sidebar.rs` (its `remove_repository`
  call, `:471`, until Task 7), `D/view/workspace/content.rs`
  (`open_settings_on`), `D/view/operations/attention.rs` (`attend`
  widened), `D/services/scheduler.rs` (`Services::check_now`),
  `D/view/settings/polling.rs` (its Check now calls it)
- Tests: `D/view/repositories_tests.rs`, `D/keys_tests.rs` (E-13),
  `D/view/operations/republish_tests.rs`, `D/view/item/edit_quit_tests.rs`,
  `D/view/item/repair_tests.rs` (E-16)

**Interfaces:**
- Produces, in `D/keys.rs`: `pub(crate) const REPOSITORIES_CONTEXT: &str = "Repositories";`
  (used by `Region::key_context`, `D/view/workspace/focus.rs:100`); the
  action `OpenRepositoryMenu`; `KeyBinding::new("menu", OpenRepositoryMenu, Some(REPOSITORIES_CONTEXT))`
  and the same for `shift-f10`; the `no_json` action
  `pub(crate) struct RepositoryMenuChoice { pub root: RepoKey, pub choice: MenuChoice, pub shown: Shown }`
  (as `DockPanel`, `:133-138`), where `pub(crate) enum Shown {
  Action(RowAction), Label(String) }` is what the item showed: its
  `RowAction` for the state's action, its label otherwise (R-35).
- Produces, in `D/view/repositories/menu.rs`:
  - `pub(crate) enum MenuAnchor { Pointer(Point<Pixels>), Bounds(Bounds<Pixels>) }`
    (not `Anchor`, which clashes with `gpui_kit::Anchor`).
  - `pub(crate) struct RepositoryMenu` with
    `fn open(root: RepoKey, items: Vec<MenuItem>, context: FocusHandle, anchor: MenuAnchor, restore: Option<FocusHandle>, window, cx) -> Self`
    building a `PopupMenu` with `menu_with_disabled(label, Box::new(RepositoryMenuChoice { root, choice, shown }), !enabled)`
    for each item and `separator()` before Remove, subscribed to
    `DismissEvent`; and `fn layer(&self, window, cx) -> impl IntoElement`
    as `D/view/footer/menu.rs:98`.
- Produces, on `Workspace`: `repository_menu: Option<RepositoryMenu>`;
  `fn open_repository_menu(&mut self, root: RepoKey, anchor: MenuAnchor, window, cx)`;
  `fn open_focused_repository_menu(&mut self, window, cx)`: from the
  list, the highlighted row, else the chosen one; from a Repositories
  toolbar button, always the chosen one, whatever row the Kit cursor
  rests on (R-40); scrolled into view and anchored at its recorded bounds,
  or at the list's bounds when it has none (R-42);
  `fn repository_menu_chosen(&mut self, action: &RepositoryMenuChoice, window, cx)`
  which first evaluates `menu_items` again for the root and ignores the
  choice when the root is no longer listed or kept, the item is now
  disabled, or the re-evaluated item's `RowAction` or label differs from
  `shown` (R-35), then dispatches: state action → `retry_resolution`,
  `Scheduler::reread`, `Scheduler::unlock`, `attend`, Check now,
  `open_settings_on` or `open_remotes`; Check now and Retry now →
  `Services::check_now`; Share main → `share_main`; Open folder →
  `open_folder(Path::new(&root), cx)`; Settings… →
  `open_settings_on(SettingsPage::Repositories, root)`; Remove →
  `remove_repository(root, restore)`. The `OpenRepositoryMenu` and
  `RepositoryMenuChoice` handlers are wrapped in `unless_dialog`
  (`render.rs:174`); the row's right-press hook, which runs in the Kit
  list's context, calls `workspace.update(…)` through the delegate's
  `WeakEntity`, and `open_repository_menu` returns at once when
  `modal_open` (R-36). The layer is dropped under a modal as
  `dock_menu_layer` (`D/view/workspace/footer.rs:244`), and when its root
  leaves the rows (R-23).
- Produces, in `impl Services` in `D/services/scheduler.rs` (`:228-247`,
  beside `repositories_listed` and `window_activated`; fact 67):
  `pub(crate) fn check_now(&self, root: &Path) -> bool`, which sends
  `Intent::PollOnce { root }` through `self.worker().submit(…, Lane::User,
  None, None)` unless `self.worker().in_flight` finds a `PollOnce` for
  the root (`D/services/worker.rs:515`, `:744`), and returns whether it
  sent. Settings' `check_now` (`D/view/settings/polling.rs:225-235`)
  calls it after its own dialog guard; the menu's Check now and Retry now
  call it here, and Check all in Task 6. Each caller keeps its own dialog
  and sheet guard (R-35).
- Produces, in `D/view/workspace/content.rs`:
  `fn open_settings_on(&mut self, page: SettingsPage, root: PathBuf, window, cx)`,
  setting `set_selection(Some(root))` as `open_remotes` does
  (`D/view/workspace/requests.rs:285-298`) (R-21).
- Changes: `remove_repository(root, restore: FocusHandle, window, cx)`
  (`D/view/workspace/requests.rs:357`) gives `restore` to the dialog and
  sets `last_region` to the caller's region; its own guard stays
  `has_active_dialog` (R-12; Settings' `submit_remove`, fact 48, is
  untouched). Its callers follow: `sidebar.rs:471` (until Task 7) and the
  five test calls of E-16 pass the Repositories list's focus handle. The
  row gains `capture_any_mouse_down`: a right button calls
  `prevent_default` and opens the menu at the pointer.

- [ ] **Step 1: Check U-1 and U-2** with a scratch test (discarded): with
  focus in the Repositories list, `walk.keys("shift-f10")` reaches an
  `OpenRepositoryMenu` handler; `window.right_click(("repo-row", 1))`
  reaches the row's `capture_any_mouse_down`. If U-1 fails, bind in the
  Kit `List` context with the `Repositories` parent and record it; if U-2
  fails, stop for the owner.
- [ ] **Step 2: Write the failing tests.**
  - `desktop::view::repositories::tests::the_menu_by_pointer_and_keyboard`,
    with polling on and every poll and check answered: right click on row
    1 while row 0 is chosen → the menu for row 1, the choice unchanged;
    items read by label (fact 43), the first being the state's sentence;
    `shift-f10` and `menu` open it for the highlighted row; `reads_seen`
    and the requests unchanged after opening it, moving the cursor and
    Esc (P22); choosing Check now sends `PollOnce` for that root on the
    user lane; Share main sends `RepoSync`; "Share main…" opens Settings ›
    Remotes; Open folder records the root in `OpenedFolders`
    (`D/view/item/lock.rs:96`); Settings… on row 1 opens Settings on row
    1's repository; a Locked row's Unlock… reaches `Scheduler::unlock`
    (copy from `attention_locked_key`, `D/view/operations/attention_tests.rs:68`);
    Retry on a row the scheduler could not read reads it again; activating
    a disabled item sends nothing; Remove from Manyhands… opens the dialog
    with its note, Cancel returns focus to the list (read in the same
    update), OK removes the row; Esc closes the menu and returns focus;
    with a dialog open and with the item details sheet open, the action
    and a dispatched `RepositoryMenuChoice` do nothing, and an open menu
    closes when a modal opens; a reload that drops the menu's repository
    closes it; a menu opened on a row, then a check of it starting, then
    Check now chosen → nothing sent; a menu opened on a row reading Can't
    reach (its first action "Retry now"), then a live attention making the
    row Needs attention with "Resume checking", then the "Retry now" item
    chosen → nothing sent and nothing resumed (the shown action differs);
    the Retry of a row that reads Needs
    attention because its registration read failed reads it again (P10–P12,
    P21, P22, P26, P27, P29, P40).
  - E-16: the five `remove_repository` test calls pass the list's focus
    handle.
  - E-13: `desktop::keys::tests::each_key_is_bound_to_its_action` gains
    `menu` and `shift-f10` → `OpenRepositoryMenu`; the table is 26 long.
  - Settings' Check now, now through `Services::check_now`: its existing
    tests (`desktop::view::settings::polling_tests::settings_polling_controls`)
    run unchanged by exact name.
- [ ] **Step 3: Run them; they fail.**
- [ ] **Step 4: Implement.**
- [ ] **Step 5: Run** both tests, the five E-16 tests and
  `settings_polling_controls` by exact name.
- [ ] **Step 6: Verify lines; commit; push; ticket comment** (checkpoint:
  commits, tests with counts, the U-1, U-2, U-4 and U-6 answers,
  rulings).

---

## Task 6: Toolbar, The More Menu And Check All (Part E)

**Files:**
- Modify: `D/view/workspace/focus.rs` (`ToolbarButton`),
  `D/view/workspace.rs` (`toolbar_focus`), `D/view/dock/header/toolbar.rs`,
  `D/view/repositories/menu.rs`, `D/assets.rs`
- Tests: `D/view/dock_tests/toolbars.rs` (E-11),
  `D/view/repositories_tests.rs`, `D/assets_tests.rs` (E-12 part)

**Interfaces:**
- `ToolbarButton` gains `AddRepository`, `CheckAll`, `RepositoriesMore`
  (ids `add-repository-button`, `check-all-button`,
  `repositories-more-button`), `region()` Repositories; `ALL: [Self; 7]`;
  `toolbar_focus: [FocusHandle; 7]`; `of(PanelKind::Repositories)` returns
  the three (`D/view/dock/header/toolbar.rs:65-69`).
- The Repositories toolbar wrapper carries `REPOSITORIES_CONTEXT`, not
  `NAVIGATION_CONTEXT` (`:134`) (R-14), so `menu` and `shift-f10` on a
  Repositories button open the chosen repository's menu, never the Kit
  cursor's row, with that button as `restore` (R-40).
- `toolbar_enabled(&self, button, cx: &App)` (`:143`): Check all only when
  one repository qualifies (R-13).
- `press_toolbar` (`:156`): Add → `open_settings(SettingsPage::Repositories)`;
  Check all → `Services::check_now` (Task 5) for each repository whose D6
  shape is `Polling`, that is not checking, whose phase is not `Attention`
  for a key or host, and for which `status::stored_applies` (Task 1) gives
  none of `UnlockRequired`, `SelectedKeyUnavailable` or
  `HostApprovalRequired`, in list order, then focus to the list if
  it disabled itself (R-13, R-22); More → the More menu (`more_items`) at the button's bounds,
  recorded by a `canvas`, restoring focus to the button;
  `repository_menu_chosen` maps Refresh and Rebuild to
  `maintain_index(false|true)` and Remove to `remove_repository(root,
  more_button)`.

- [ ] **Step 1: Merge main** (Global Constraints, Main).
- [ ] **Step 2: Write the failing tests.**
  - E-11: `desktop::view::dock::tests::toolbars::panel_toolbars_work_by_tab_and_enter`
    loops over Add and More too (not Check all, whose window has no
    schedule): each reached by Tab, run by Enter, its region Repositories
    with the ring; Add opens Settings › Repositories; More opens its menu,
    and Refresh sends `IndexRefresh` for the chosen repository;
    `secondary-n` from a Repositories button opens nothing; with the Kit
    cursor moved to another row than the chosen one, `shift-f10` on Add
    opens the chosen repository's menu, not the cursor's, and Esc returns
    focus to Add;
    refused behind a dialog and a sheet. `with_no_repository` (`:170`)
    names the three: Add enabled, Check all disabled, More enabled with
    its items disabled (P12, P13, P28, P39).
  - `desktop::view::repositories::tests::check_all_checks_each_repository`:
    with polling on, every launch poll answered and each check answered
    with `answer_job(JobKind::PollOnce, …)`: an ordinary, a paused, a
    locked and a stored-`UnlockRequired` repository (`store_outcome`);
    Check all by Enter sends `PollOnce` for the first two and none for the
    locked or the stored one; nothing else is sent; focus moves to the list
    once it disables itself (P23, P24).
  - E-12 part: `refresh-cw`, `ellipsis`, `plus` if not already loaded.
- [ ] **Step 3: Run them; they fail.**
- [ ] **Step 4: Implement.**
- [ ] **Step 5: Run** the tests by exact name.
- [ ] **Step 6: Verify lines; commit; push.**

---

## Task 7: The Details Block Leaves The Item Area (Part F)

**Files:**
- Modify: `D/view/sidebar.rs` (`selection`, `retry`, `remove`,
  `resolved`, `ShareMain` and unused constants and imports go),
  `D/view/workspace/render.rs` (`:117-130`), `D/view/workspace/content.rs`
  (`repository_details` `:623-629`, `open_item` `:310`, `follow_tab`
  `:330`, `content_body` `:648-675`), `D/view/item.rs` (`end_region_with`,
  `:400`, `:544-547`, if nothing else uses it), `D/view/workspace.rs`
  (`remove_focus`, `retry_focus` if unused)
- Tests: `D/view/workspace_tests.rs` (E-1, E-2, E-3), `D/view/tabs_tests.rs`
  (E-4), `D/view/keyboard_tests/actions.rs` (E-5),
  `D/view/settings/repositories_manage_tests.rs` (E-6),
  `D/view/item/share_tests.rs` (E-7), `D/view/keyboard_tests/discussion.rs`
  (E-8)

**Interfaces:**
- Produces, in `D/view/workspace/content.rs`:
  `pub(crate) const OPEN_FROM_THE_LISTS: &str = "Open a ticket or document from the lists";`
  shown with no tab when a repository is chosen; `NOTHING_SELECTED`
  ("Select a repository.") moves here from `sidebar.rs` and shows when
  none is (R-15).
- Removes: `sidebar::selection` and its two callers; `UNAVAILABLE`,
  `SHARE_MAIN_BUTTON`, `RESOLVING`, `REMOVE`, and `sidebar.rs`'s
  `index_label` and `ConfigurationDto` imports, when the sweep finds no
  user (`REPOSITORIES_HEADING` and `SHOWING_LAST_READ` went in Task 3).
  Kept: `SELECTED`, `RETRY`, `SHARING`, `SHARE_MAIN`, `SHARE_MAIN_SET_UP`,
  `configuration_label` (fact 6).

- [ ] **Step 1: Write the failing tests.**
  - `desktop::view::workspace::tests::the_item_area_holds_nothing_of_the_repository`:
    no repository chosen → "Select a repository."; chosen, no tab →
    `OPEN_FROM_THE_LISTS`; a tab open → no `selection-` element anywhere
    in the window (P14).
  - E-1 rest: the six `desktop::view::workspace::tests` E-1 names, as E-1
    says (`selection-name` → the chosen row's mark; `selection-unavailable*`
    → `("repo-state", i)`, on the kept row for the two second-actor tests;
    `selection-retry` → the menu's Retry by `shift-f10`; configuration
    through `repository_tooltip`); the configuration-label and index
    assertions go (**Removes**).
  - E-2 rest: `a_busy_list_reload_keeps_the_last_rows_with_a_label`'s
    `selection-name` read (`:927`) becomes the mark.
  - E-3 rest: `the_repository_selector_selects_with_arrows_and_enter`'s
    `selection-name` read (`:1055`) becomes the mark.
  - E-4: `desktop::view::tabs::tests::opening_a_tab_does_not_change_the_selection`.
  - E-5: `desktop::view::keyboard_tests::actions::retry_an_unavailable_repository_by_keyboard`
    and `…::remove_an_unavailable_repository_by_keyboard`, with the
    `unavailable` helper, through `secondary-1` and `shift-f10`.
  - E-6: `desktop::view::settings::repositories::manage_tests::removing_a_registration_whose_folder_was_renamed_away_succeeds`
    and `…::the_sidebar_remove_of_a_registration_already_gone_lets_the_entry_go`
    through the menu, the second from the kept row's; focus after the
    dialog on the list.
  - E-7: `desktop::view::item::share::tests::share_main_in_the_repository_detail_submits_repo_sync`,
    `…::share_main_without_a_remote_reads_share_main_and_opens_remotes`,
    `…::an_accepted_share_offers_stop_and_stopping_signals_it`,
    `…::a_stopped_share_says_what_completed_and_offers_share`,
    `…::share_opens_remotes_on_its_own_repository_after_another` choose
    Share main from the menu by label, its label checked through
    `menu_items`; `share_main_at_the_end_of_an_item_tab_submits_repo_sync`
    is deleted (**Removes**).
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

## Task 8: Footer Status Items (Part G)

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
  - `pub(crate) fn to_share_name(n: usize, next: &str, only_chosen: bool) -> String`
    ("{n} to share. Select {repo}." or "{n} to share in {repo}."; `next`
    is "{repo}" or, when two listed repositories share it, "{repo} in
    {parent}").
  - `pub(crate) fn next_to_share(order: &[(RepoKey, usize)], chosen: Option<&RepoKey>) -> Option<RepoKey>`
    (the next listed repository with work after `chosen`, wrapping; never
    the kept row; `None` when the only one with work is `chosen`). The
    name and the choice both come from it, evaluated again when the item
    is pressed (R-39).
- Produces, in `D/view/workspace/footer.rs`: `fn footer_status(&self, cx) -> FooterStatus`
  from the scheduler (`schedule().repo(root)` for every listed
  repository: checking is `checking_now` or `in_flight()`, R-26; the sum
  is of the unshared lists' `items`, R-24) and the worker; `press` gains
  `UpdateCheck` → `open_settings(SettingsPage::Polling)` and `ToShare` →
  `select_repository(next)` when `next_to_share`, evaluated at the press,
  gives one, with focus kept on the control and no layout change
  (decision 23). When a focused status item is no longer in the plan,
  focus moves to the next shown control to its right in the plan:
  Needs attention, else the right dock group's first control, else
  Settings (R-38).
- Consumes: Task 1's `ago`, `RepoSchedule.unshared`.

- [ ] **Step 1: Check U-5.** Count the Tab presses `reach(Settings)` and
  `reach(Attention)` take today (`D/view/keyboard_tests/shell/support.rs:84`)
  and with two more stops; if past `TAB_LIMIT` (40), raise it in the same
  commit as a further edit (E-17 onward).
- [ ] **Step 2: Write the failing tests.**
  - `desktop::view::footer::tests::status_items_say_and_choose`: one table
    of `update_text`, `to_share_name` and `next_to_share` (none, one not
    chosen, one chosen, two with the chosen among them, wrapping); `plan`
    with each `FooterStatus` (items in order, hidden when `None` or 0);
    then a window journey with polling on, every poll answered, and two
    repositories with work (`schedule_mut()`): the two controls read their
    texts; Tab reaches both; Enter on To share chooses the next repository
    each time and focus stays; with only the chosen one having work, Enter
    reads nothing and changes nothing; with Repositories hidden, the
    layout is unchanged; a share that brings the count to 0 while To share
    has focus moves focus to Needs attention, or the next shown control;
    two repositories in folders of the same name: the name carries the
    parent; Enter on Update check opens Settings › Checking for updates;
    refused behind a dialog and a sheet; a `saved` item in a scheduled
    repository with no completed check yet shows "1 to share" (P12, P15,
    P25, P37, P41, P44).
  - E-10: `desktop::view::footer::tests::a_secondary_click_opens_the_dock_menu_with_its_side_checked`,
    `…::toggle_names_follow_their_state`,
    `desktop::view::keyboard_tests::shell::each_side_is_reached_by_the_dock_menu_keys`,
    `…::footer_controls_are_reached_by_tab` (an explicit expected list for
    its fixture, in which nothing is scheduled), and `said`
    (`shell/support.rs:93`) gaining the two controls.
- [ ] **Step 3: Run them; they fail.**
- [ ] **Step 4: Implement.**
- [ ] **Step 5: Run** the tests of Step 2 and each user of `reach` the Task
  0 sweep listed, by exact name.
- [ ] **Step 6: Verify lines; commit; push.**

---

## Task 9: Privacy (Part I)

**Files:**
- Modify: `D/view/polling_privacy_tests.rs` (E-15)

- [ ] **Step 1: Settle U-3.** Add a fixture remote
  `ssh://git:sentinel-password-03a@example.invalid/r.git` as the
  publication remote of one repository (copy
  `a_remote_url_with_a_password_is_shown_redacted`,
  `D/view/settings/remotes_tests.rs:196`) and record in the ledger whether
  `ssh_publication` schedules it. Part I's assertions hold either way.
- [ ] **Step 2: Write the test edit.** E-15:
  `desktop::view::polling_privacy_tests::polling_privacy` takes
  `region-repositories` out of `ALLOWED` (`:127-139`); gains the surfaces
  `("repo-", "", "repository row")` and `("footer-", "", "footer")` in
  `SURFACES` (`:144`, now 17); adds the URL-password sentinel; and sweeps
  `repository_tooltip`, `menu_items`' labels and the footer's
  `update_text` and `to_share_name` for every repository, built from the
  live workspace's facts through Task 3's accessors, against every
  sentinel but `TITLE`, which titles carry by design: `BODY`, `COMMENT`,
  `DRAFT`, `EDIT`, `PATH`, `PASSPHRASE`, `PRIVATE` and the URL-password
  one; positive control: at least one "Shared copy:" line exists (P17).
  The test is a guard: it passes once the surfaces and the sweep exist,
  and fails only if a sentinel leaks; show it can fail with a discarded
  scratch run that puts a sentinel into a tooltip, and record that in the
  ledger.
- [ ] **Step 3: Implement** only what the test finds (expected: nothing in
  production; the test proves it).
- [ ] **Step 4: Run** it by exact name.
- [ ] **Step 5: Verify lines; commit; push.**

---

## Task 10: Checklist, Gates, Review-Ready

- [ ] **Step 1: On-screen checklist.** Write
  `docs/plans/2026-11-12-wave-04-cycle-03a-on-screen-checklist.md` in
  Cycle 02's format, for the owner on a release build under Wayland
  against a scratch `XDG_DATA_HOME`: the desktop smoke test
  (`devenv shell -- cargo run --locked --features desktop --bin manyhands`);
  the panel against frames 1a and 1b in the dark theme; each state the
  fixtures allow (This computer, To share, Up to date, Paused, Checking…,
  Can't reach, Locked, Needs attention from a check's problems, a non-SSH
  remote, Can't open by renaming a folder, the kept row, Needs fixing); a
  relaunch with a stored outcome; the menu by right click and by Shift F10
  and the menu key, with its first line; the toolbar and More by keyboard
  with focus visible; the footer's two items; the item area with no tab;
  tooltips; Orca reading a row and the menu. Claude records only what the
  owner reports.
- [ ] **Step 2: Gates**, once on the final tree: AGENTS.md's three static
  gates, then its fast tier with `--no-fail-fast`. No SSH target (nothing
  under its paths changed). A failure outside the E-list is a product bug
  (fixed), an appearance test (deleted under E-14) or a needed edit (E-17
  onward).
- [ ] **Step 3: Merge check.** `git fetch origin`, then
  `git merge-tree --write-tree HEAD origin/main` is clean; the Boundary
  command prints nothing.
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
  `01M5CS4MH25QQ769HV61ZRKHXG` (its two repository-row items: one already
  met at `7034522f`, fact 34; the other done here).

---

## Plan Self-Review

**Required outcomes:**

| P | Task |
| --- | --- |
| P1, P2 | 2 |
| P3 | 1 (model, scheduler, Settings), 2 (state) |
| P4 | 1 |
| P5 | 1, 3 |
| P6 | 2 (tooltip), 4 (reads) |
| P7 | 2 |
| P8, P9, P16 | 3 |
| P18 | 2, 3, 7 |
| P10, P21 | 2 (`menu_items`), 5 |
| P11, P22, P27, P29, P40 | 5 |
| P12 | 5, 6, 8 |
| P13 | 2 (`more_items`), 6 |
| P14 | 7 |
| P15, P25, P37, P41 | 8 |
| P17 | 9 |
| P19 | 1, 2 |
| P20 | 2 |
| P23 | 5 (Check now), 6 (Check all) |
| P24 | 6 |
| P26 | 1 (`reread`), 2 (state), 5 (the menu) |
| P28, P39 | 6 |
| P30, P31, P32 | 1 |
| P33 | 1, 3 |
| P34 | 1 (Settings), 2 (state) |
| P35 | 2 |
| P36 | 2 (sentence), 4 (reads) |
| P38 | 3 |
| P42 | 1 |
| P43 | 1 (Settings, `stored_applies`), 2 (state) |
| P44 | 1 (list, Settings), 3 (row), 8 (footer) |

**Rulings:** R-1, R-2, R-3, R-4, R-18, R-19, R-28, R-29, R-30, R-31, R-32,
R-41, R-44, R-45 → Task 1; R-5, R-6, R-17, R-27, R-33, R-34, R-43 → Task
2; R-7, R-8,
R-10, R-20, R-37 → Task 3; R-9 → Tasks 2 and 4; R-11, R-12, R-21, R-23,
R-25, R-35, R-36, R-42 → Task 5 (R-35's `Services::check_now` used by
Task 6 too); R-13, R-14, R-22, R-40 → Task 6; R-15,
R-16 → Tasks 2 and 7; R-24, R-26 → Tasks 2 and 8; R-38, R-39 → Task 8.

**Unverified:** U-6 → Task 1 Step 1; U-7 → Task 1
(`finished_shares_and_saves_are_heard`); U-4 → Task 3 Step 1; U-1, U-2 →
Task 5 Step 1; U-5 → Task 8 Step 1; U-3 → Task 9 Step 1.

**Design checkpoint:** Task 1 Step 2 writes the unshared list's table
(U1–U15) before any code and stops for the orchestrator if a row cannot
be expressed (Global Constraints, Test-first).

**Named edits:** E-9 → Task 1 (with `main` on every `TargetParts` and
`AttemptTarget` literal of fact 66); E-1, E-2 and E-3 (row parts), E-12 (part)
→ Task 3; E-13, E-16 → Task 5; E-11, E-12 (part) → Task 6; E-1, E-2 and
E-3 (rest), E-4, E-5, E-6, E-7, E-8 → Task 7; E-10, E-12 (part) → Task 8;
E-15 → Task 9; E-14 → any task.

**Tests:** every test in the design's Testing table has its task, by the
same exact path.

**Placeholder scan:** no "TBD", "TODO" or "similar to Task N". Names
marked "copy from" are the existing helpers to follow, not open choices.
