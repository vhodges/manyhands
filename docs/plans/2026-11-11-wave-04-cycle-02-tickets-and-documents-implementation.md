---
title: "Wave 04 Cycle 02 Tickets And Documents Panels Implementation Plan"
date: 2026-11-11
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M75C77FRSVW977B3HPJP4P79"
---

# Wave 04 Cycle 02 Tickets And Documents Panels Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task by task. Draft,
> revision 3 with round 3's fixes: implementation waits for the owner's approval of the three
> documents and a separate go-ahead. Re-read the design before each task
> and record every checkpoint on ticket `01M757DHFAVPEVY4TWQTVKQAMJ`.
> Steps use checkbox (`- [ ]`) syntax.

**Goal:** Rebuild the Tickets and Documents panels to `03-panels.md`: one
row anatomy and plain list states, a Tickets panel filtered, counted and
sorted in the desktop over one read, with Find folded into its filter
input, a Documents tree whose folders stay as the person leaves them, and
unreadable files shown in their list.

**Architecture:** Pure models first: kept rows and list bodies (Part A),
the ticket view (Part B), document rows and expansion (Part E), unreadable
files (Part I). Then the views that draw them: a shared `ListRow`, a Kit
`List` with the app's column header for Tickets, the base Kit `Tree` for
Documents, both always drawn, and toolbars of base buttons in Cycle 01's
panel header that count as their panel's region. No library change.

**Tech stack:** Rust 2024; `gpui-kit` 0.6.6 through `gpui_kit::*` only;
Devenv. No new dependency.

**Spec:** [Cycle](../Cycles/wave-04-cycle-02-tickets-and-documents.md) and
[design](2026-11-11-wave-04-cycle-02-tickets-and-documents-design.md), with
`docs/design/03-panels.md` and the prototype source for anatomy and copy.
"Fact n", "P-n", "R-n", "E-n", "K-n" and "U-n" refer to the design;
"decision n" to the Cycle.

## Global Constraints

**Precondition.** The owner has approved the three documents, authorized
implementation and chosen the execution method. Task 0 does not start
before that.

**Roles.** The orchestrator runs the tasks, or dispatches one implementer
per task and then a fresh reviewer, one at a time. It alone writes the
ledger and ticket comments and pushes the ticket branch. Implementers
never dispatch subagents and never edit ticket, comment or ledger files.

**Where.** Every task runs in
`.manyhands/worktrees/01M757DHFAVPEVY4TWQTVKQAMJ` on
`manyhands/ticket/01M757DHFAVPEVY4TWQTVKQAMJ`. No lane branches and no
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
moves. Never rebase. If Wave 04 Cycle 03 has merged, stop for a short
checkpoint with the owner on the shared header, focus and rows.

**Push.** After each task's commit, the orchestrator pushes the ticket
branch to origin (a fast-forward of this branch only) and checks that
`HEAD` equals `origin/manyhands/ticket/01M757DHFAVPEVY4TWQTVKQAMJ`, so
reviewers who are not local can follow (owner, 2026-11-11).

**GPUI** only through `gpui_kit::*`, never a glob import.

**Copy.** Every user-visible string is the design's, verbatim, held in a
`pub(crate) const` (or a formatting function for strings with values)
beside its use, so tests compare constants. Sentence case, no
exclamation marks, `…` (U+2026) on an action that opens a dialog.

**Test runs** (AGENTS.md, Testing Strategy; decision 17):

- Desktop: `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <path> -- --exact`,
  written below as `… <path>`.
- A test's path is its module's: `desktop::view::tickets::tests::<name>`
  for `view/tickets_tests.rs`, `desktop::view::dock::tests::<module>::<name>`
  for `view/dock_tests/<module>.rs`. New `_tests.rs` files are included as
  `#[cfg(test)] #[path = "…_tests.rs"] mod tests;`.
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
its code. Model tests are table-driven (one table per function).

**No appearance tests** (Wave Testing): nothing asserts colours, sizes,
icons, tones, indentation or that a view renders. Tooltip and description
text is tested through its pure function (fact 11).

**Test seams.** Reads are held with `DelayingBackend::hold_read(kind)` and
counted with `reads_seen(kind)` (fact 32); busy and failed reads are made
as `workspace_tests::a_busy_list_reload_keeps_the_last_rows_with_a_label`
and the made-ready tests make them. Fixture data directories only; no
network; no real waits (`advance_clock`); `now` passed in.

**Existing tests** change only as E-1 to E-14 say. Any other edit is
pre-authorized (decision 16): recorded in the ledger as E-15 onward with
its reason, checked by the task's reviewer, and an assertion removal is
called out in the ledger and the review-ready summary.

**Dead code.** An item first used by a later task carries
`#[cfg_attr(not(test), allow(dead_code))] // Used from Task N.`; Task N
removes it. Test-only helpers are `#[cfg(test)]`.

**Commits.** One per task, conventional message, ending
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Ticket records.** A comment through `devenv shell -- cargo run --locked --bin manyhands-cli -- comment add …`,
run against the main checkout while the ticket worktree is clean, after
Tasks 0, 4 and 9: commits, tests with counts, rulings. Before
`ticket save`, commit any owned `ticket.md` edit.

**Ledger.** `docs/plans/2026-11-11-wave-04-cycle-02-tickets-and-documents-execution.md`,
created in Task 0, written only by the orchestrator.

**File size.** New source files stay below about 500 lines.

## Review Focus

Inputs no task's main journey exercises that would most likely bite a
person, each pinned by a test in the task named:

1. **A filter that hides the selected row, then Enter.** The Kit list
   keeps a selection index, so after the rows change it could point at a
   different ticket. Enter must open nothing (Task 3, in
   `the_filter_bar_works_without_a_read`).
2. **Pasted text with spaces or capitals** (" VH-AAAAA ", a ULID in
   lowercase) must match and open as typed cleanly (Task 2 table rows;
   Task 4 test).
3. **A ticket with an empty status or no change time** sorts last by that
   column in both directions, and its age is blank (Task 2 table rows).
4. **Retry and repository changes:** a Retry of the same repository keeps
   the filter text, the kept rows and a held ID read; selecting another
   repository drops all three (Task 2
   `a_read_lands_only_for_its_repository_and_load`; Task 3
   `list_states_act_and_a_reload_keeps_the_rows`; Task 4 test).
5. **A folder that disappears and comes back between reloads** keeps the
   person's choice; a new folder starts expanded; restoring the selection
   never re-expands a folder (Task 5 table rows).

## File Map And Dependency Order

| Task | Part | Summary |
| --- | --- | --- |
| 0 | — | Merge main, baseline gates, re-check facts, E-sweep, ledger. |
| 1 | A | `KeptList`, list bodies, `ListRow`, state elements, icons; E-11. |
| 2 | B | Ticket query, rows, one read, `derive`, footnote, age, markers, tooltip, status mapping, last-loaded repository; every `TicketQuery` site; E-1, E-2, E-10 part. |
| 3 | C | Tickets panel: filter bar with the filter input's place and one-source text, hint, column header, rows, footnote, states over an always-drawn list, header and tab count, Home and End, Retry without clearing; E-3, E-6 part, E-10 part, E-12. |
| 4 | D | Enter, the ID read and its status, the repository-change clear; Find's cards and lookup go; E-4, E-5, E-6 part; checkpoint. |
| 5 | E | Documents model: expansion, selection rule, `doc_row`, kept tree, `unmanaged` moved. |
| 6 | F | Documents panel: base tree, rows, keys, states; E-8, E-9. |
| 7 | I | Unreadable files: the status read, rows in both lists. |
| 8 | G | Panel toolbars counted as their region; Collapse all; E-6 part, E-7, E-14. |
| 9 | — | On-screen checklist, gates, review-ready; checkpoint. |

New source files:

- `D/model/list_view.rs`, `D/model/ticket_view.rs`, `D/model/markers.rs`,
  `D/model/status.rs`, `D/model/unreadable.rs`
- `D/view/rows.rs`
- `D/view/tickets/filter_bar.rs`, `D/view/tickets/columns.rs`,
  `D/view/tickets/row.rs` (with `mod` lines in `D/view/tickets.rs`)
- `D/view/dock/header/toolbar.rs`

New test files: `D/model/list_view_tests.rs`,
`D/model/ticket_view_tests.rs`, `D/model/markers_tests.rs`,
`D/model/unreadable_tests.rs`, `D/view/dock_tests/toolbars.rs` (a
`mod toolbars;` in `dock_tests/mod.rs`). New tests in existing files are
named in their tasks.

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
  `git diff --stat a0a734a4 HEAD -- src/desktop src/repository/read docs/Waves docs/RFC docs/design Cargo.lock`.
  Re-read each fact whose file changed and correct the design first. If
  `Cargo.lock` moved `gpui-kit`, `gpui-base` or `gpui-component`, stop for
  the owner: facts 1–12 rest on the locked versions.
- [ ] **Step 3: Sweep the E-list,** over tests and production code alike:

```sh
P='src/desktop'
git grep -n -e 'ticket-lifecycle' -e 'tickets-showing' -e 'tickets-readiness-showing' \
  -e 'filter-any-readiness' -e 'filter-ready' -e 'filter-blocked' -e 'ticket-reason-' \
  -e 'ticket-nonconforming' -e 'ticket-problem' -e 'to_filter' -e 'names_filter' \
  -e 'TicketQuery {' -e 'query()' -e 'row_description' -e '\.state()' \
  -e 'take_loaded_tickets' -e 'set_closure' -e 'set_readiness' -e 'TicketRead' \
  -e 'tickets-state' -e 'documents-state' -- $P
git grep -n -e 'find-match' -e 'find-state' -e 'FIND_' -e 'FindResult' -e 'FindQuery' \
  -e 'FindState' -e 'find_tickets_by_slug' -e 'load_lists' -e 'clear_lists' -- $P
git grep -n -e 'doc-path' -e 'doc-empty' -e 'doc-nonconforming' -e 'doc-problem' \
  -e 'EMPTY_FOLDER' -e 'NONCONFORMING' -e 'node_label' -e 'unmanaged(' -e 'TreeNode::' \
  -e 'new-ticket' -e 'new-document' -e 'new-folder' -e 'tree_paths' -e 'relative_time' \
  -e 'fn ringed' -e 'ReadKind::' -e 'tint(' -- $P
```

  Record each site in the ledger against the E-n or the task that
  changes it. A test site no E-n names is a pre-authorized edit
  (decision 16), recorded as E-15 onward; a production site no task names
  goes into the task that breaks it.
- [ ] **Step 4: Ledger, commit, push, ticket comment** recording the base
  SHA.

---

## Task 1: Row, List States And Icons (Part A)

**Files:**
- Modify: `D/model/list_state.rs` (add `KeptList`), `D/model/list_state_tests.rs`
- Create: `D/model/list_view.rs`, `D/model/list_view_tests.rs`, `D/view/rows.rs`
- Modify: `D/model/mod.rs`, `D/view/mod.rs`, `D/assets.rs`, `D/assets_tests.rs` (E-11)

**Interfaces:**
- Produces, in `D/model/list_state.rs`:
  - `pub(crate) struct KeptList<T>`, `Default` (Loading, nothing kept), with
    `fn reload(&mut self)` (Loading, keeping the Ready rows),
    `fn clear(&mut self)` (Loading, nothing kept),
    `fn apply_state(&mut self, state: ListState<T>)` (a finished state:
    `Ready` replaces; `Busy` over kept rows keeps them and sets
    busy-over-kept; `Busy` with none, or `Failed`, replaces),
    `fn apply<V>(&mut self, result: ReadResult<V>, ready: impl FnOnce(V) -> ListState<T>)`
    (through `apply_state`), `fn state(&self) -> &ListState<T>`,
    `fn rows(&self) -> Option<&T>` (the Ready rows, or the kept rows while
    Loading or busy), `fn reloading(&self) -> bool` (Loading over kept
    rows), `fn busy_over_kept(&self) -> bool`.
- Produces, in `D/model/list_view.rs`:
  - `pub(crate) enum ListKind { Tickets, Documents }`
  - `pub(crate) enum ListAction { Refresh, Retry, RebuildIndex, ClearFilters, ShowAll, NewItem }`
  - `pub(crate) enum AlertTone { Warning, Failure }`
  - `pub(crate) struct Alert { pub tone: AlertTone, pub text: String, pub detail: Option<String>, pub action: ListAction }`
  - `pub(crate) enum ListBody { NoRepository, Skeleton, Rows { alert: Option<Alert> }, Empty { text: String, action: Option<ListAction> }, Problem(Alert) }`
  - `pub(crate) struct BodyInput<'a> { pub kind: ListKind, pub repo: Option<&'a str>, pub listed: usize, pub elsewhere: usize, pub closure: ClosureFilter, pub narrowed: bool }`
    (`listed` as Part A defines it; `closure` and `elsewhere` are ignored
    for Documents).
  - `pub(crate) fn list_body<T: Rows>(list: &KeptList<T>, input: BodyInput<'_>) -> ListBody`
    (`Rows` is the existing trait, `D/model/list_state.rs:28-36`).
- Produces, in `D/view/rows.rs`:
  - `pub(crate) const ROW_HEIGHT: Pixels` (28 px).
  - `pub(crate) struct ListRow` with `fn new(id: impl Into<ElementId>) -> Self`
    (it calls `.test_support()`), `fn indent(self, left: Pixels) -> Self`,
    `fn gap(self, gap: Pixels) -> Self`, `fn focused(self, bool) -> Self`
    (its list has focus), `fn expanded(self, Option<bool>) -> Self`
    (`aria_expanded`), `fn label(self, impl Into<SharedString>) -> Self`
    (`aria_label`), `fn description(self, impl Into<SharedString>) -> Self`
    (`aria_description`), `fn tooltip(self, impl Into<SharedString>) -> Self`,
    `fn on_click(self, impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self`;
    `impl Selectable` (also sets `aria_selected`), `ParentElement`,
    `RenderOnce`.
  - `pub(crate) fn skeleton_rows(id, count: usize, cx: &App) -> impl IntoElement`,
    `pub(crate) fn alert_row(id, alert: &Alert, on_action: impl Fn(&mut Window, &mut App) + 'static, cx: &App) -> impl IntoElement`,
    `pub(crate) fn empty_state(id, text: &str, action: Option<(SharedString, Box<dyn Fn(&mut Window, &mut App)>)>, cx: &App) -> impl IntoElement`.

- [ ] **Step 1: Write the failing tests.**
  - `desktop::model::list_state::tests::kept_rows_follow_reloads`, one
    table. Start → op → expected `(state kind, rows(), reloading(),
    busy_over_kept())`: Ready(a) → reload → (Loading, Some(a), true,
    false); then ok(b) → (Ready, Some(b), false, false); Ready(a) → reload
    → busy → (Ready, Some(a), false, true); then reload → ok(b) → busy flag
    false; Ready(a) → reload → failed → (Failed, None, false, false);
    Loading → busy → (Busy, None, false, false); Ready(a) → clear →
    (Loading, None, false, false); `apply_state(Ready(b))` after a reload
    → (Ready, Some(b), false, false).
  - `desktop::model::list_view::tests::each_state_has_its_own_body`, one
    table over every row of the design's Part A state table, for Tickets
    and for Documents, asserting the `ListBody` variant, its exact text
    (`{repo}` = "manyhands"), `detail`, tone and action. Include: never
    refreshed, stale and incomplete together (never refreshed wins); busy
    over kept rows on a stale list (busy wins); stale with `listed` 0
    (alert over no rows, not empty); Ready with `listed` 1 but no
    conforming row (Rows, not Empty, P27); a narrowed empty list with
    `elsewhere` 0 (Clear filters) and with `elsewhere` 1 ("No tickets here
    match. 1 more ticket matches with other filters.", Show all); Closed
    empty (no action); Failed with the failure's message as `detail`;
    Loading over kept rows (Rows, no alert).
  - E-11: `desktop::assets::tests::every_item_view_icon_loads` and its
    array length gain `repeat`, `bug`, `sparkles`, `list-checks`,
    `file-plus`, `folder-plus`, `chevrons-down-up`, `arrow-up-down`,
    `list-filter`, `file-exclamation-point`.
- [ ] **Step 2: Run them; they fail** (types and icons missing).
- [ ] **Step 3: Implement** `KeptList`, `list_body` and the icons
  (`ItemIcons` in `D/assets.rs`, fact 12). Then `ListRow` and the three
  state elements per Part A: hover `hover`; selected `selection` with a
  2 px inset `accent` left line, weight 600 and `aria_selected`; a 2 px
  inset `teal` ring when selected and focused; skeleton bars in
  `theme.skeleton` with no animation (R-6); the alert's icon
  `triangle-alert` (amber) for Warning and `circle-x` (red) for Failure,
  with `detail` as a visible second line. Colours from `chrome_colors(cx)`
  (fact 33).
- [ ] **Step 4: Run the three tests by exact name; they pass.**
- [ ] **Step 5: Verify lines; commit; push.**

## Task 2: The Tickets Model (Part B)

**Files:**
- Modify: `D/model/tickets.rs`, `D/model/tickets_tests.rs` (E-1), `D/model/mod.rs`
- Create: `D/model/ticket_view.rs`, `D/model/ticket_view_tests.rs`,
  `D/model/markers.rs`, `D/model/markers_tests.rs`, `D/model/status.rs`
- Modify: `D/view/item/chips.rs` (uses `model::status`), `D/view/item.rs`
  (re-exports `relative_time` as `pub(crate)`; `header_tests.rs:21` keeps
  its import), `D/view/workspace/lists.rs` (the read, the setters,
  `sync_tickets`, `open_ready_tickets`), `D/view/lists.rs` (`query()` by
  reference), `D/view/item/sheet/fields.rs` (`take_loaded_tickets` reads
  the open conforming rows, R-33), `D/view/tickets_tests.rs` (E-2),
  `D/view/lifecycle/made_ready_tests.rs` (`:288-289`) and
  `made_ready_review_tests.rs` (`:65-70`) (E-10's made-ready part: R-26
  reaches their set-up here)

**Interfaces:**
- Consumes: `KeptList` (Task 1).
- Produces, in `D/model/tickets.rs`:
  - `pub(crate) enum SortColumn { Type, Code, Title, Status, Updated }`
  - `pub(crate) struct TicketSort { pub column: SortColumn, pub ascending: bool }`,
    default `{ Updated, true }` (newest first), with
    `fn toggled(self, column: SortColumn) -> Self` (a new column
    ascending, the same one reversed).
  - `TicketQuery { pub closure: ClosureFilter, pub readiness: Option<ReadinessFilter>, pub text: String, pub sort: TicketSort }`
    (`Clone`, not `Copy`), default Open, none, empty, default sort;
    `fn narrowed(&self) -> bool` (text not blank, or a readiness chosen);
    `fn with_closure(self, ClosureFilter) -> Self` and
    `fn with_readiness(self, Option<ReadinessFilter>) -> Self` applying
    R-26.
  - `pub(crate) fn read_filter() -> TicketFilter` returns
    `TicketFilter::default()`; `to_filter`, `names_filter`, `TicketRead`'s
    names and the names read go.
  - `TicketRow` gains `pub readiness: Option<ReadinessState>` and
    `pub context: RowContext`; `reasons` is filled for every blocked row,
    named from all the read's rows, when the read is applied.
  - `Tickets`: `list: KeptList<Vec<TicketRow>>`, a last-loaded repository
    that `clear` does not reset, and the derived `TicketView`;
    `fn query(&self) -> &TicketQuery`; `fn state(&self) -> &ListState<Vec<TicketRow>>`
    (a pass-through); `fn view(&self) -> &TicketView`;
    `fn set_query(&mut self, query: TicketQuery)` derives at once and
    starts no read; `fn list(&self) -> &KeptList<Vec<TicketRow>>`;
    `fn load(&mut self, repo: RepoKey, …) -> (Stamp, bool)` calls
    `reload()` for the last-loaded repository and `clear()` for another,
    and returns the read's stamp and whether the repository changed
    (R-35).
- Produces, in `D/model/markers.rs`:
  - `pub(crate) struct RowContext { pub kind: ItemContextKind, pub branch: Option<String>, pub worktree: String }`
    with `fn of(item: &ItemDto) -> RowContext`; its label is
    `context_kind_label` and a missing branch `NO_BRANCH` (fact 17).
  - `pub(crate) struct Markers { pub unsaved: bool, pub conflict: bool, pub closed: bool }`
  - `pub(crate) fn row_markers(held: Held, conflict: bool, closed: bool, closure: ClosureFilter) -> Markers`
- Produces, in `D/model/status.rs`: the header's `Tint` and
  `pub(crate) fn tint(status: &str) -> Option<Tint>` moved from
  `chips.rs`, with "ready" added as `Done`.
- Produces, in `D/model/ticket_view.rs`:
  - `pub(crate) struct SegmentCounts { pub open: usize, pub closed: usize, pub all: usize }`
  - `pub(crate) struct TicketView { pub rows: Vec<TicketRow>, pub counts: Option<SegmentCounts>, pub elsewhere: usize, pub listed: usize, pub shown: usize, pub needs_fixing: usize, pub unreadable: usize }`
    (`counts` is `None` with nothing to count, Part B item 8)
  - `pub(crate) fn derive(rows: Option<&[TicketRow]>, query: &TicketQuery) -> TicketView`
  - `pub(crate) fn footnote(view: &TicketView, query: &TicketQuery, repo: &str) -> String`
  - `pub(crate) fn hint(view: &TicketView) -> Option<String>`
  - `pub(crate) fn age_label(changed_at: Option<&str>, now: OffsetDateTime) -> String`
  - `pub(crate) fn ticket_tooltip(row: &TicketRow, repo: &str) -> Vec<String>`
    (its lines; the description is their join by "; ").

- [ ] **Step 1: Write the failing tests.**
  - `desktop::model::ticket_view::tests::derive_filters_counts_and_sorts`.
    Fixture rows in library order: `a` open, ready, code `vh-aaaaa`, type
    "cycle", title "Dock layout", status "in progress", 1 h old; `e` open,
    ready, code `vh-eeeee`, type "task", "Echo", status "closed", 90 min
    old; `d` open, ready, no code, type "defect", "zeta", status "Ready",
    2 d old; `c` closed, code `vh-aaaaa` (duplicate), type "feature", "Old
    dock", status "done", 3 d old; `b` open, blocked by `a`, code
    `vh-bbbbb`, type "task", "Footer toggles", status "", no change time;
    `n` nonconforming, path `.manyhands/tickets/X/ticket.md`. Rows of the
    table (query → ids in order; counts; `elsewhere`; `listed`; `shown`;
    footnote with repo "manyhands"):
    - default → `a, e, d, b, n`; 4/1/5; 0; 5; 4;
      `4 open tickets in manyhands · 1 needs fixing` (`e` with status
      "closed" is under Open, P4).
    - Closed → `c, n`; `1 closed ticket in manyhands · 1 needs fixing`.
    - All → `a, e, d, c, b, n`; `5 tickets in manyhands · 1 needs fixing`.
    - text " VH-AAAAA " under Open → `a, n`; counts 1/1/2; `elsewhere` 1;
      footnote `1 open ticket in manyhands matches "VH-AAAAA" · 1 needs fixing`;
      `hint` "1 more ticket matches with other filters.".
    - the same text under All → `a, c, n`; `elsewhere` 0; no hint.
    - the same text under Open and Ready → `a, n`; `elsewhere` 1 (`c` is
      closed, hidden by lifecycle and readiness).
    - text = `d`'s ULID in lowercase → `d, n`.
    - text "zzz" → `n` only; `listed` 1; `shown` 0 (P27).
    - Ready → `a, e, d, n`; `3 ready tickets in manyhands · 1 needs fixing`.
      Blocked → `b, n`; `1 blocked ticket in manyhands · 1 needs fixing`.
      Counts unchanged by readiness. Closed then Ready → closure Open
      (R-26).
    - sort Title ascending → `a, e, b, d, n` ("Dock", "Echo", "Footer",
      "zeta"); descending → `d, b, e, a, n`.
    - sort Status ascending → `e, a, d, b, n` ("closed", "in progress",
      "Ready"; "" last); descending `d, a, e, b, n`.
    - sort Code ascending → `a, b, e, d, n` (no code last); descending
      `e, b, a, d, n`.
    - sort Type ascending → `a, d, e, b, n` (cycle, defect, then the two
      tasks in library order); descending reverses the rows with a value
      → `b, e, d, a, n`.
    - sort Updated descending (oldest first) → `d, e, a, b, n` (`b` still
      last).
    - no rows (`None`) → `counts` `None`, `listed` 0.
  - `desktop::model::ticket_view::tests::ages_read_as_the_prototype_writes_them`,
    `now` fixed: 30 s → "0m"; 59 min → "59m"; 60 min → "1h"; 89 min →
    "1h"; 90 min → "2h"; 23 h → "23h"; 24 h → "1d"; 36 h → "2d"; 400 d →
    "400d"; future → "0m"; `None` → ""; "garbage" → "".
  - `desktop::model::markers::tests::markers_follow_drafts_conflicts_and_closure`:
    `Held::Yes` → unsaved; `No` and `Unknown` → not; conflict → conflict;
    closed under All → closed; closed under Closed and Open → not.
  - `desktop::model::ticket_view::tests::tooltips_carry_the_id_context_and_reasons`:
    a coded row on the primary copy → `vh-aaaaa · manyhands`, `ID {ulid}`,
    `Type cycle`, the "Primary" label, `Git: branch main · worktree {root}`;
    a no-code row → `No short code · manyhands`; an active item worktree →
    its label, branch and worktree; unverified with no branch → its label
    and `NO_BRANCH`; blocked by an open ticket → "Waits for open ticket:
    vh-aaaaa Dock layout"; on a cycle → its "On a dependency cycle: …"
    line; nonconforming → only `Needs fixing: {guidance} Open it to fix.`;
    closed → a `Closed` line.
  - E-1 in `D/model/tickets_tests.rs`: the five tests become
    `desktop::model::tickets::tests::a_read_lands_only_for_its_repository_and_load`,
    one table: a read for the load in flight lands, and its blocked rows
    carry reasons named from all its rows; one for an earlier load,
    another repository, or after `clear` is dropped; a query set before
    any load is kept; a query change starts no read and derives at once;
    after `clear`, a load of the last-loaded repository returns false and
    one of another returns true. `the_default_query_lists_open_tickets`
    asserts the default query and `read_filter() == TicketFilter::default()`;
    `blocked_reasons_name_the_blocker_the_missing_id_and_the_cycle` and
    `an_untitled_blocker_is_named_by_its_short_code` build `TicketFilter`
    literals.
  - E-2 in `D/view/tickets_tests.rs`: delete the two tests; the
    `library()` and `blocked_list` helpers and `:322-326` take
    `TicketFilter` literals.
  - E-10's made-ready part: the set-ups start from Closed with text set
    through `set_query` (Closed with Blocked is unreachable under R-26),
    and assert that `open_ready_tickets` gives Open, Ready and no text
    (R-29); readiness is read from the model until Task 3 draws
    `filter-readiness`.
- [ ] **Step 2: Run the new tests; they fail.**
- [ ] **Step 3: Implement** the model per Part B, then every site the
  `TicketQuery` change reaches: `D/view/lists.rs:58`, `:77`;
  `D/view/workspace/lists.rs:125`, `:150-199` (the setters call
  `set_query` and read nothing; `open_ready_tickets` follows R-29);
  `chips.rs` uses `model::status::tint`; `fields.rs` per R-33.
  `sync_tickets` passes the view's rows to the pane and, until Task 3,
  strips `reasons` from rows unless Blocked is chosen, so today's view
  draws as before (`D/view/tickets.rs:265`).
- [ ] **Step 4: Run** by exact name:
  `desktop::model::ticket_view::tests::derive_filters_counts_and_sorts`,
  `…ages_read_as_the_prototype_writes_them`,
  `…tooltips_carry_the_id_context_and_reasons`,
  `desktop::model::markers::tests::markers_follow_drafts_conflicts_and_closure`,
  `desktop::model::tickets::tests::a_read_lands_only_for_its_repository_and_load`,
  `desktop::model::tickets::tests::the_default_query_lists_open_tickets`,
  `desktop::model::tickets::tests::blocked_reasons_name_the_blocker_the_missing_id_and_the_cycle`,
  `desktop::model::tickets::tests::an_untitled_blocker_is_named_by_its_short_code`,
  the edited made-ready tests, and in `desktop::view::tickets::tests`
  `arrows_move_in_the_list_and_enter_opens`,
  `each_closure_filter_shows_the_librarys_rows_and_keeps_the_nonconforming_entry`,
  `ready_and_blocked_show_the_librarys_rows_with_reasons_and_keep_the_nonconforming_entry`,
  `the_why_blocked_panel_gives_the_selected_rows_reasons_in_full`,
  `the_panel_says_incomplete_where_the_library_does`, and
  `desktop::view::item::edit_tests::sheet_fields::suggestions_come_from_the_loaded_tickets_values`
  (it must pass unchanged). Write the full
  names in the ledger. They pass.
- [ ] **Step 5: Verify lines; commit; push.**

## Task 3: The Tickets Panel (Part C)

**Files:**
- Modify: `D/view/tickets.rs` (the delegate's `render_empty` draws
  nothing), `D/view/lists.rs` (`tickets_body`, `choice_ids`, Retry through
  `reload_lists`), `D/view/workspace.rs` (`choice_focus`, the
  `ListEvent::Confirm` subscription, the sort focus handles, `find_input`
  with the new placeholder and `set_clean_on_escape(true)`),
  `D/view/workspace/render.rs` (`sync_tickets` every render, the new
  actions), `D/view/workspace/find.rs` (`set_filter_text`; Change sets the
  text), `D/view/find.rs` (Find's bar no longer draws the input)
- Create: `D/view/tickets/filter_bar.rs`, `D/view/tickets/columns.rs`, `D/view/tickets/row.rs`
- Modify: `D/view/workspace/lists.rs`, `D/view/workspace/focus.rs` (focus
  targets after actions), `D/view/dock/panels.rs` (`listed()`, "Updating…"),
  `D/view/dock/header/tabs.rs` (an id on the count; "Updating…" on a tab),
  `D/keys.rs`, `D/keys_tests.rs` (E-12)
- Tests: `D/view/tickets_tests.rs` (E-3 and new), `D/view/keyboard_tests.rs`
  `change_the_filters` (E-6 part), `D/view/dock_tests/state.rs` (E-10's
  hidden-panel part), the made-ready tests (`filter-readiness`'s name)

**Interfaces:**
- Consumes: `ListRow`, `list_body`, `alert_row`, `empty_state`,
  `skeleton_rows` (Task 1); `Tickets::view`, `footnote`, `hint`,
  `age_label`, `ticket_tooltip`, `row_markers`, `tint` (Task 2).
- Produces:
  - `pub(crate) struct ShownTicket { pub row: TicketRow, pub markers: Markers }`;
    `TicketsPane::set(&self, rows: Vec<ShownTicket>, window, cx)` keeps the
    selection by ID (or worktree and path), clears it when the row is gone,
    and does nothing when the rows are unchanged.
  - `Workspace::set_sort(&mut self, column: SortColumn, cx)`,
    `Workspace::clear_filters(&mut self, window, cx)`,
    `Workspace::show_all(&mut self, window, cx)`,
    `Workspace::set_filter_text(&mut self, text: &str, window, cx)` (R-30),
    `pub(crate) const FILTER_PLACEHOLDER: &str = "Filter by title or code";`.
    The filter bar draws `find_input` (`list-filter` prefix, `cleanable`);
    its Change sets the query's text. Enter keeps today's Find behaviour
    until Task 4.
  - Actions `SelectFirst` and `SelectLast` in `D/keys.rs`, bound to `home`
    and `end` in `TICKET_LIST_CONTEXT = "TicketList"` (a `key_context` on a
    div around the Kit list) and in `DOCUMENTS_CONTEXT` (handled from
    Task 6), each handler wrapped in `unless_dialog` and notifying.
  - Element ids: `filter-open`, `filter-closed`, `filter-all`,
    `filter-readiness`, `tickets-hint`, `tickets-show-all`, `sort-type`,
    `sort-code`, `sort-title`, `sort-status`, `sort-updated`, `ticket-row`,
    `ticket-type`, `ticket-slug`, `ticket-unsaved`, `ticket-conflict`,
    `ticket-closed`, `ticket-status`, `ticket-changed`, `tickets-footnote`,
    `tickets-alert`, `tickets-empty`, `tickets-skeleton`, and
    `("panel-count", node)` on the header's count, with its text as
    `aria_label`.

- [ ] **Step 1: Write the failing tests.**
  - `desktop::view::tickets::tests::the_filter_bar_works_without_a_read`:
    a fixture repository with tickets like Task 2's. On load,
    `reads_seen(Tickets)` is 1 with the default filter. Snapshot reads,
    requests and tabs; after every later step assert the `ticket-row`
    labels, the segments' names ("Open, 4 tickets" …), `tickets-footnote`,
    `("panel-count", _)`, and that reads, requests and tabs are unchanged
    (P25, P29). Steps: click `filter-closed`; Tab to `filter-all`, Enter;
    focus `filter-readiness`, Enter, Down three times to "Blocked", Enter
    (the button's name "Readiness: blocked"; Why blocked shows; focus is
    back on the button); choose "Any readiness"; with Closed chosen choose
    Ready → `filter-open` selected (R-26); under Open and Ready, type a code
    shared with a closed ticket → `tickets-hint` reads "1 more ticket
    matches with other filters.", activate `tickets-show-all` → All, Any
    readiness, both listed, focus in the filter input (P7); click
    `sort-title` (ascending, name "Sort by title, A to Z"; `sort-updated`
    reads "Sort by updated"), Tab to it and Enter (reversed, "Sort by
    title, Z to A"); focus the list, `home` and `end` select the first and
    last row (P18). Review Focus 1: select a row, choose a lifecycle that
    hides it, then Enter in the list opens nothing (P30).
  - `desktop::view::tickets::tests::list_states_act_and_a_reload_keeps_the_rows`:
    after a finished job, while the reload is held, the old rows stay and
    `("panel-count", _)` reads "Updating…"; a held busy reload shows
    `tickets-alert` with "manyhands was busy, so this list wasn't
    updated." over the same rows, and its Retry sends one ticket read and
    keeps typed filter text and the rows (R-35); a failed reload shows the
    failure with its message as a visible line; F6 into that failed list,
    and into an empty one, leaves `Region::Tickets` focused with its ring
    (P31); a narrowed empty list's Clear filters empties the text, sets
    Any readiness and focuses the filter input; the unnarrowed empty
    state's New ticket… opens the dialog; on a stale or never-refreshed
    repository, Refresh submits `IndexRefresh`; on an unavailable index,
    Rebuild the search index submits `IndexRebuild` (a failed or
    unavailable read through `DelayingBackend::replace_service`; if a state
    cannot be made, stop for the owner).
  - `desktop::view::tickets::tests::markers_appear_without_a_read`: open a
    ticket and start editing it (`e`, following D3's editor-test rules);
    its row shows `ticket-unsaved`, and `reads_seen(Tickets)` is unchanged
    (P14).
  - E-3 and E-6 (`change_the_filters`) as the design words them. E-10's
    hidden-panel part: the kept text is filter text that keeps both rows,
    and the test also chooses a sort and Ready and checks they are kept
    (P26); the made-ready tests read readiness from `filter-readiness`. E-12: `each_key_is_bound_to_its_action`
    gains `home` → `SelectFirst` and `end` → `SelectLast`.
- [ ] **Step 2: Run them; the new ones fail.**
- [ ] **Step 3: Implement** Part C: the filter bar (segments restyled from
  today's `choice`, R-4; the readiness `Button::dropdown_menu`, R-5), the
  hint, the column header outside the list (R-2), `ShownTicket` rows as
  `ListRow`s with Part C's cells, marker icons and tooltips, the footnote,
  Why blocked under Blocked, the body from `list_body` above a list that
  is always drawn (R-34), the header count from `listed` with "Updating…"
  on the count or the tab (R-11, R-36), Retry through `reload_lists`
  (R-35), and focus after actions (R-28). Markers are worked out in every
  Workspace render from `held_by` and the repository's
  `pending_conflicts` (R-32). Find stays at the top of the panel until
  Task 4. Remove the "Showing …" texts and the inline reasons line; keep
  `LIFECYCLE_OPEN`, `LIFECYCLE_CLOSED` and `lifecycle_label` (fact 14).
  The filter input moves into the filter bar with one-source text (R-30);
  Find's Enter, cards and status stay as they are until Task 4.
- [ ] **Step 4: Run** the new tests and every edited one by exact name;
  they pass.
- [ ] **Step 5: Verify lines; commit; push.**

## Task 4: The Filter Input Replaces Find (Part D)

**Files:**
- Modify: `D/view/find.rs` (it now draws the filter status line;
  `find_bar`, `match_row` and their constants go),
  `D/view/workspace/find.rs` (`submit_filter`; `find_focus` and
  `find_match_focus` go), `D/model/find.rs` (the short-code half goes),
  `D/services/mod.rs` (`ReadKind::FindSlug` goes with its last use),
  `D/view/lists.rs`, `D/view/workspace.rs`, `D/view/workspace/render.rs`
  (the deferred clear), `D/view/workspace/lists.rs` (`clear_lists` keeps
  the text and a held ID read unless the selected repository differs
  from the last-loaded one)
- Tests: `D/view/find_tests.rs` (E-4), `D/model/find_tests.rs` (E-5),
  `D/view/keyboard_tests.rs` `find`, `find_by_short_code_and_ulid` (E-6
  part)

**Interfaces:**
- Consumes: `TicketQuery.text`, `set_query`, `load`'s repository-changed
  answer (Task 2); the filter bar, `set_filter_text` and the input
  (Task 3).
- Produces: `Workspace::submit_filter(&mut self, window, cx)` (brings the pane up to
  date before it selects); `pub(crate) fn parse_id(text: &str) -> Option<ItemId>`
  in `D/model/find.rs`, beside the kept stamped ID read and its states;
  element id `filter-status`.

- [ ] **Step 1: Rewrite the tests (E-4, E-5) first; they fail.**
  `desktop::view::find::tests` per E-4, covering: under Open, a closed
  ticket's ULID in lowercase with spaces, Enter → its tab, "Opening…"
  while held, focus stays in the input (P8, Review Focus 2); a
  document's ULID → its tab; an unknown ULID → "Nothing in manyhands has
  that ID."; a ULID read held while another repository is selected →
  dropped, no tab; a code one closed ticket alone carries, under Open,
  Enter → it opens, focus stays in the input (P9); a duplicate code,
  Enter → no tab, focus in the list with the first match selected;
  Enter on blank text opens nothing; selecting another repository → the
  input's value and the query's text are empty and no row of the first
  repository is drawn, while a Retry of the same repository keeps the
  text (P17, Review Focus 4). `desktop::model::find::tests` per E-5. Then
  E-6 as the design words it.
- [ ] **Step 2: Run them; they fail.**
- [ ] **Step 3: Implement** Part D: PressEnter runs `submit_filter`;
  `secondary-p` focuses the input; `filter-status` says the ID read's
  state, with no Retry control (Enter again retries); the
  repository-change clear is applied at the next render (R-30, R-35).
  Remove the match cards, the short-code lookup, `find_tickets_by_slug`'s
  call and `ReadKind::FindSlug`; keep the stamped ID read (R-31).
- [ ] **Step 4: Run** the rewritten and edited tests by exact name.
- [ ] **Step 5: Verify lines; commit; push; ticket comment** (Tickets
  done).

## Task 5: The Documents Model (Part E)

**Files:**
- Modify: `D/model/documents.rs`, `D/model/documents_tests.rs`,
  `D/model/item.rs`, `D/view/item/repair.rs`, `D/view/item/banners.rs`,
  `D/view/item/repair/adopt.rs` (the `unmanaged` callers),
  `D/view/documents.rs` (`TreeItem::expanded` from the model, `:286`;
  selection by `set_selected_index(index_of)`),
  `D/view/workspace/lists.rs` (`documents_changed`, the kept tree, the
  last-loaded repository), `D/view/new_folder.rs` (`:302`) and
  `D/view/workspace/requests.rs` (`:161`) (expand the destination's
  ancestors, R-27)

**Interfaces:**
- Consumes: `KeptList` (Task 1); `row_markers` (Task 2).
- Produces:
  - `pub(crate) fn unmanaged(item: &ItemDto) -> bool` moved unchanged to
    `D/model/item.rs` with the helpers it needs.
  - `pub(crate) struct Expansion` (collapsed folder paths) with
    `fn is_expanded(&self, path: &str) -> bool`,
    `fn set(&mut self, path: &str, expanded: bool)`,
    `fn collapse_all(&mut self, nodes: &[TreeNode])`,
    `fn expand_ancestors(&mut self, path: &str)`.
  - `Documents`: `tree: KeptList<Vec<TreeNode>>` fed by `apply_state` from
    the combined reads, and a last-loaded repository that `clear` does not
    reset; an `Expansion` per repository for the session;
    `fn expansion(&self) -> Option<&Expansion>` (the selected
    repository's), `fn set_expanded(&mut self, path: &str, expanded: bool)`,
    `fn collapse_all(&mut self)`.
  - `pub(crate) fn visible_selection(nodes: &[TreeNode], expansion: &Expansion, selected: &str) -> Option<String>`
    (the selected id, or its nearest visible ancestor's, R-27).
  - `pub(crate) enum Trailing { None, Count(usize), Empty }`,
    `pub(crate) enum DocKind { Folder, Document, Nonconforming { adoptable: bool } }`
    (Task 7 adds `Unreadable`),
    `pub(crate) struct DocRow { pub label: String, pub kind: DocKind, pub expanded: Option<bool>, pub trailing: Trailing, pub tooltip: Vec<String> }`
  - `pub(crate) fn doc_row(node: &TreeNode, expanded: bool) -> DocRow`

- [ ] **Step 0: Merge `origin/main`** (Global Constraints, Main).
- [ ] **Step 1: Write the failing tests.**
  - `desktop::model::documents::tests::expansion_survives_rebuilds_and_collapse_all`:
    a tree `guides/deep`, `plans`, `empty`; nothing collapsed → every
    folder with child nodes expanded; collapse `guides` → rebuild with a
    new folder `notes` → `guides` collapsed, `notes` expanded; rebuild
    without `plans`, then with it → `plans` keeps its state (Review Focus
    5); `collapse_all` → every folder collapsed;
    `expand_ancestors("guides/deep/new")` expands `guides` and
    `guides/deep`; another repository's expansion is separate;
    `visible_selection` of a document inside collapsed `guides/deep` →
    `folder:guides`, and of a visible one → itself (never expanding).
  - `desktop::model::documents::tests::rows_read_as_the_design_writes_them`,
    one table over Part E's seven kinds before Part I: label, kind,
    `expanded`, trailing and tooltip lines. Include a collapsed folder
    with three documents at two depths (`Count(3)`), a leaf folder holding
    only a non-document file (`Count(0)`, `expanded` `None`), an empty
    folder (`Empty`, "Empty folder. Shared once it contains a
    document."), a nonconforming `missing_front_matter` (file name,
    adoptable, "Not a managed document yet. Open to adopt it." then the
    path) and a `missing_field` one (not adoptable, "Needs fixing:
    {guidance} Open it to fix." then the path).
- [ ] **Step 2: Run them; they fail.**
- [ ] **Step 3: Implement** Part E; the existing tree view keeps working
  (it builds `TreeItem::expanded` from `Expansion`).
- [ ] **Step 4: Run** the two tests by exact name; they pass.
- [ ] **Step 5: Verify lines; commit; push.**

## Task 6: The Documents Panel (Part F)

**Files:**
- Modify: `D/view/documents.rs`, `D/view/lists.rs` (`documents_body`),
  `D/view/workspace.rs` (the `TreeEvent` subscription in `new`),
  `D/view/workspace/render.rs`, `D/view/workspace/lists.rs`
  (`open_selected_document` toggles a folder; `SelectFirst` and
  `SelectLast` for the tree), `D/view/dock/panels.rs` (Documents'
  "Updating…")
- Tests: `D/view/documents_tests.rs` (E-8, new), `D/view/new_folder_tests.rs`,
  `D/view/move_document_tests.rs` (E-8), `D/view/keyboard_tests/actions.rs`,
  `keyboard_tests/editing.rs` (E-9)

**Interfaces:**
- Consumes: `ListRow`, `list_body` (Task 1); `SelectFirst`, `SelectLast`
  (Task 3); `doc_row`, `Expansion`, `visible_selection` (Task 5).
- Produces: element ids `doc-node`, `doc-count`, `doc-empty`,
  `doc-nonconforming`, `doc-unsaved`, `documents-alert`, `documents-empty`,
  `documents-skeleton`; `#[cfg(test)] DocumentsPane::document_paths(&self, cx) -> Vec<String>`
  for `tree_paths` (E-8).

- [ ] **Step 1: Write the failing test**
  `desktop::view::documents::tests::folders_keep_their_state_and_toggle_by_key`:
  focus the tree; Down to folder `guides`, Left → collapsed (`doc-count`
  shows its count, the row's `expanded` is false), Right → expanded,
  Enter → collapsed (R-24); a click on `guides` → expanded; collapse it;
  finish a refresh job → still collapsed (P19); hide and show Documents
  from the footer → still collapsed, selection kept (P26); `home` and
  `end` select the first and last row (P18); Enter on a document opens
  its tab; on a stale or never-refreshed repository, `documents-alert`'s
  Refresh submits `IndexRefresh` (P15). Reads and requests unchanged by
  every step but the refresh (P25). Then E-8 and E-9 as the design words
  them.
- [ ] **Step 2: Run it; it fails.**
- [ ] **Step 3: Implement** Part F: the base `Tree` with `ListRow` rows
  and `.vertical_scrollbar` (fact 4), always drawn (R-34), indent
  `8 + 14 × depth`, chevron or spacer, icon, label, trailing cell and the
  unsaved marker; each row's `aria_label` the label, `aria_description`
  the tooltip lines and `expanded` its state; `focused` while the
  Documents region contains focus; a document row's `on_click` opens it;
  the body from `list_body`; Enter on a folder toggles it through the
  model; the `TreeEvent` subscription maps `folder:{path}` ids to
  `Documents::set_expanded`; selection after a rebuild by
  `visible_selection` and `set_selected_index(index_of)`; "Updating…" on
  the count or tab (R-36). Remove the second-line cells and the "Empty
  folder" text.
- [ ] **Step 4: Run** the new test and every edited one by exact name.
- [ ] **Step 5: Verify lines; commit; push.**

## Task 7: Unreadable Files (Part I)

**Files:**
- Create: `D/model/unreadable.rs`, `D/model/unreadable_tests.rs`
- Modify: `D/model/mod.rs`, `D/services/mod.rs` (`ReadKind::IndexStatus`),
  `D/view/workspace.rs` (the read's slot), `D/view/workspace/lists.rs`
  (read it with each load, carrying both lists' stamps),
  `D/model/tickets.rs`, `D/model/documents.rs` (hold the files beside the
  `KeptList`), `D/model/ticket_view.rs` (`unreadable`), `D/model/ticket_view_tests.rs`,
  `D/model/documents_tests.rs` (`DocKind::Unreadable`), `D/view/tickets/row.rs`,
  `D/view/documents.rs`, `D/view/documents_tests.rs` (the `match` at
  `:100-111`, E-8)
- Test: `D/view/tickets_tests.rs`

**Interfaces:**
- Produces: `pub(crate) struct UnreadableFile { pub path: String, pub worktree: String, pub primary: bool }`;
  `pub(crate) fn unreadable_rows(status: &IndexStatusDto, root: &str) -> (Vec<UnreadableFile>, Vec<UnreadableFile>)`
  (tickets, documents); `TicketRow` and `TreeNode` each gain an unreadable
  form that selects but does not open (R-38); `DocKind::Unreadable`.

- [ ] **Step 1: Write the failing tests.**
  - `desktop::model::unreadable::tests::unreadable_files_go_to_their_list`:
    `SourceUnreadable` at `.manyhands/tickets/X/ticket.md` → tickets;
    at `docs/a/b.md` → documents; the same document path in an item
    worktree → documents, not primary; at `docs/a` (a directory) →
    neither; at `src/x.md` → neither; an absolute path, a missing path or
    a missing worktree → neither; `MissingFrontMatter` at a ticket path →
    neither.
  - Extend `derive_filters_counts_and_sorts` with an unreadable row `u`:
    listed after `n` under every lifecycle, readiness, text and sort;
    counted in `listed`, not in `shown`; the default footnote ends
    `· 1 needs fixing · 1 can't be read`. Extend
    `tooltips_carry_the_id_context_and_reasons` with `u` in an item
    worktree (`Can't be read: {guidance}`, its path, `Git: worktree …`)
    and `rows_read_as_the_design_writes_them` with the unreadable kind.
  - Extend `a_read_lands_only_for_its_repository_and_load` with status
    rows: a status read that lands replaces the unreadable files; a busy
    or failed one keeps them; one for another repository is dropped.
  - `desktop::view::tickets::tests::an_unreadable_file_is_listed_with_its_reason`:
    a ticket file and a document file written as invalid UTF-8, then a
    refresh job (the ticket written as a raw directory with no item
    worktree: discovery drops primary-copy problems inside a ticket
    directory that has one, `L/repository/discovery.rs:802-804`); `reads_seen(IndexStatus)` rose by one with the lists'
    reload (P1); the ticket row shows its path and a `ticket-status` of
    "can't be read" under every lifecycle, and the document its file name
    with `doc-unreadable`; Enter on either opens no tab (R-38); the
    footnote ends `· 1 can't be read`. Then hold the status read on a
    further reload and make it busy, as
    `a_busy_list_reload_keeps_the_last_rows_with_a_label` makes a read
    busy: the lists land and keep the unreadable rows (P28). The failed
    status read is the model table's row (no seam fails one).
- [ ] **Step 2: Run them; they fail.**
- [ ] **Step 3: Implement** Part I.
- [ ] **Step 4: Run** the new and extended tests by exact name.
- [ ] **Step 5: Verify lines; commit; push.**

## Task 8: Panel Toolbars (Part G)

**Files:**
- Create: `D/view/dock/header/toolbar.rs`, `D/view/dock_tests/toolbars.rs`
- Modify: `D/view/dock/header.rs` (`mod toolbar;`), `D/view/dock/header/tabs.rs`
  (`single` and `tabs` take the Workspace handle and window; the drag
  source becomes the title area), `D/view/workspace/focus.rs`
  (`focused_region` and `cycle_region` count a toolbar handle as its
  region), `D/view/workspace.rs` (the buttons' focus handles),
  `D/view/workspace/render.rs`, `D/view/lists.rs` and `D/view/documents.rs`
  (body buttons and their wrappers go), `D/view/dock_tests/mod.rs`
- Tests: `D/view/keyboard_tests.rs` (`ringed` and `on`, E-14; E-6 part),
  `D/view/keyboard_tests/editing.rs`, `D/view/documents_tests.rs`
  `new_folder_opens_its_dialog` (E-7); `folders_keep_their_state_and_toggle_by_key`
  gains the Collapse all step

**Interfaces:**
- Consumes: `Documents::collapse_all`, `visible_selection` (Task 5).
- Produces: `Workspace::panel_toolbar(&mut self, kind: PanelKind, window, cx) -> Option<AnyElement>`,
  drawn by `single` at the right of the title row and by `tabs` at the
  right of the strip for the active tab (R-13), inside a div with the
  `Navigation` key context that stops mouse-down propagation (R-37).
  Buttons `new-ticket-button`, `new-document-button`, `new-folder-button`,
  `collapse-all-button`: base `gpui_kit::base::Button`s with
  `.track_focus` on Workspace-owned handles, `.tooltip(…)`,
  `unless_dialog`, and the disabled states of Part G. A
  `#[cfg(test)]` accessor exposes `focused_region` to the keyboard walk
  (E-14).

- [ ] **Step 1: Write the failing test**
  `desktop::view::dock::tests::toolbars::panel_toolbars_work_by_tab_and_enter`:
  in the default layout, Tab to `new-ticket-button` → `focused_region` is
  `Region::Tickets` and its ring shows; Enter → the New ticket… dialog
  (cancel); `secondary-n` there → New ticket… (cancel); F6 from it → the
  next region of C1 decision 9. A mouse-down on the button and a 5 px
  move starts no panel drag. Tab to `new-document-button`, Enter → New
  document…; `new-folder-button` → New folder…; `collapse-all-button` →
  every folder collapsed, the hidden selection on its top-level folder,
  the button disabled and focus on the tree (P20). Move Documents into
  the Tickets group as a tab (the dock's move, as C1's tests do) → with
  Documents active its toolbar is drawn and works; with Tickets active,
  New ticket's; while a reload of Tickets is held, its tab reads
  "Tickets · Updating…" (P29). With no repository selected, the three New buttons are
  disabled. With a dialog open, activating a toolbar button does nothing
  (P24). Each button's accessible name is the design's. Extend
  `folders_keep_their_state_and_toggle_by_key` with Collapse all by
  click. Then E-14, E-7 and E-6's toolbar steps.
- [ ] **Step 2: Run it; it fails.**
- [ ] **Step 3: Implement** Part G (U-3: if the header cannot draw
  Workspace-built base buttons or focus cannot be counted as the region,
  stop and record it for the owner; decision 18's alternative is a body
  row).
- [ ] **Step 4: Run** the new and edited tests by exact name.
- [ ] **Step 5: Verify lines; commit; push.**

## Task 9: Checklist, Gates, Review-Ready

- [ ] **Step 1: On-screen checklist.** Write
  `docs/plans/2026-11-11-wave-04-cycle-02-on-screen-checklist.md` in
  Cycle 01's format, for the owner on a release build under Wayland
  against a scratch data directory: both panels against frames 1a and 1b
  in the dark theme; every list state reached (a never-refreshed
  repository, a busy one, an unreadable file); the menu, segments, column
  headers and toolbar by keyboard with focus visible; dragging a panel by
  its title while the toolbar stays clickable; tooltips; a narrow left
  dock (K-2); Orca reading a row, a segment and the readiness button.
  Claude records only what the owner reports.
- [ ] **Step 2: Gates**, once on the final tree: AGENTS.md's three static
  gates, then its fast tier with `--no-fail-fast`. No SSH target (nothing
  under its paths changed). A failure outside the E-list is a product bug
  (fixed), an appearance test (deleted under E-13) or a needed edit
  (E-15 onward).
- [ ] **Step 3: Merge check.** `git merge-tree --write-tree HEAD origin/main`
  is clean; the Boundary command prints nothing.
- [ ] **Step 4: Fill** the Cycle's Acceptance Status column from the tests
  by exact name and the ledger; note where the code differs from the
  design for the owner's amendment.
- [ ] **Step 5: Commit, push, mark review-ready, ticket comment.** Not
  done without the owner: a PR, the merge, `ticket close`, cleanup, native
  CI.

## Plan Self-Review

- **Spec coverage.** Part A → Task 1; B → 2; C → 3; D → 4; E → 5; F → 6;
  I → 7; G → 8; H → Tasks 3 (Home and End), 4 (`secondary-p`), 6 (Enter on
  a folder) and 8 (toolbar focus). P1 → Tasks 3, 7; P2–P7, P10, P13,
  P27 → Task 2's tables and Task 3's journey; P8, P9, P17 → Task 4; P11,
  P12, P18, P29–P31 → Task 3; P14 → Tasks 2, 3; P15, P16 → Tasks 1, 3, 6;
  P19–P23, P26 → Tasks 3, 5, 6; P20, P24 → Task 8; P25 → Tasks 3, 6;
  P28 → Task 7; P32 → Task 2 (the existing fields test, unchanged).
  Decisions 1–20 each land in the task of their Part. Amendments go in
  the approval commit, before Task 0.
- **Types.** `KeptList`, `ListBody`, `BodyInput`, `ListAction`,
  `TicketQuery`, `TicketSort`, `SortColumn`, `TicketView`, `SegmentCounts`,
  `Markers`, `RowContext`, `ShownTicket`, `Expansion`, `DocRow`, `DocKind`,
  `UnreadableFile` are defined once and used by the names above.
- **Review Focus.** Each line names its test and task.
- **Proportion.** Signatures, test names and assertions only; bodies are
  the implementer's.
