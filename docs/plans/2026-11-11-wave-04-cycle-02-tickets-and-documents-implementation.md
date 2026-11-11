---
title: "Wave 04 Cycle 02 Tickets And Documents Panels Implementation Plan"
date: 2026-11-11
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
---

# Wave 04 Cycle 02 Tickets And Documents Panels Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task by task. Draft,
> revision 2: implementation waits for the owner's approval of the three
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
Documents, and toolbars in Cycle 01's panel header that count as their
panel's region. No library change.

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
  name, each named edit included. Unchanged tests are not rerun. No
  whole-suite run before Task 9.
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

**Test isolation.** Fixture data directories only; no network; no real
waits (`advance_clock`); `now` passed in.

**Existing tests** change only as E-1 to E-13 say. Any other edit is
pre-authorized (decision 16): recorded in the ledger as E-14 onward with
its reason, checked by the task's reviewer, and an assertion removal is
called out in the ledger and the review-ready summary.

**Dead code.** An item first used by a later task carries
`#[cfg_attr(not(test), allow(dead_code))] // Used from Task N.`; Task N
removes it. Test-only helpers are `#[cfg(test)]`.

**Commits.** One per task, conventional message, ending
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Ticket records.** A comment through `devenv shell -- cargo run --locked --bin manyhands-cli -- comment add …`,
run against the main checkout while the ticket worktree is clean, after
Tasks 0, 4 and 9: commits, tests with counts, rulings. A comment add
publishes the ticket branch. Before `ticket save`, commit any owned
`ticket.md` edit.

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
4. **Switching repositories with filter text typed and rows kept:** the
   new repository never shows the old one's rows or text, and a Retry of
   the same repository keeps the text (Task 1 `kept_rows_follow_reloads`;
   Task 2 `a_read_lands_only_for_its_repository_and_load`; Task 4 test).
5. **A folder that disappears and comes back between reloads** keeps the
   person's choice; a new folder starts expanded (Task 5 table row).

## File Map And Dependency Order

| Task | Part | Summary |
| --- | --- | --- |
| 0 | — | Merge main, baseline gates, re-check facts, E-sweep, ledger. |
| 1 | A | `KeptList`, list bodies, `ListRow`, state elements, icons; E-11. |
| 2 | B | Ticket query, rows, one read, `derive`, footnote, age, markers, tooltip, status mapping; every `TicketQuery` site; E-1, E-2. |
| 3 | C | Tickets panel: filter bar, hint, column header, rows, footnote, states, header count, Home and End; E-3, E-6 part, E-10 part, E-12. |
| 4 | D | The filter input replaces Find, keeping the ID read; E-4, E-5, E-6 part, E-10 part; checkpoint. |
| 5 | E | Documents model: expansion, selection rule, `doc_row`, kept tree, `unmanaged` moved. |
| 6 | F | Documents panel: base tree, rows, keys; E-8, E-9. |
| 7 | I | Unreadable files: the status read, rows in both lists. |
| 8 | G | Panel toolbars counted as their region; Collapse all; E-7. |
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
  -e 'TicketQuery {' -e 'query()' -e 'row_description' -- $P
git grep -n -e 'find-match' -e 'find-state' -e 'FIND_' -e 'FindResult' -e 'FindQuery' \
  -e 'FindState' -e 'find_tickets_by_slug' -- $P
git grep -n -e 'doc-path' -e 'doc-empty' -e 'doc-nonconforming' -e 'doc-problem' \
  -e 'EMPTY_FOLDER' -e 'NONCONFORMING' -e 'node_label' -e 'unmanaged(' \
  -e 'new-ticket' -e 'new-document' -e 'new-folder' -e 'tree_paths' -- $P
```

  Record each site in the ledger against the E-n or the task that
  changes it. A test site no E-n names is a pre-authorized edit
  (decision 16), recorded as E-14 onward; a production site no task names
  goes into the task that breaks it.
- [ ] **Step 4: Ledger, commit, ticket comment** recording the base SHA.

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
  - `pub(crate) enum ListAction { Refresh, Retry, RebuildIndex, ClearFilters, NewItem }`
  - `pub(crate) enum AlertTone { Warning, Failure }`
  - `pub(crate) struct Alert { pub tone: AlertTone, pub text: String, pub detail: Option<String>, pub action: ListAction }`
  - `pub(crate) enum ListBody { NoRepository, Skeleton, Rows { alert: Option<Alert> }, Empty { text: String, action: Option<ListAction> }, Problem(Alert) }`
  - `pub(crate) struct BodyInput<'a> { pub kind: ListKind, pub repo: Option<&'a str>, pub listed: usize, pub closure: ClosureFilter, pub narrowed: bool }`
    (`listed` as Part A defines it; `closure` is ignored for Documents).
  - `pub(crate) fn list_body<T: Rows>(list: &KeptList<T>, input: BodyInput<'_>) -> ListBody`
- Produces, in `D/view/rows.rs`:
  - `pub(crate) const ROW_HEIGHT: Pixels` (28 px).
  - `pub(crate) struct ListRow` with `fn new(id: impl Into<ElementId>) -> Self`,
    `fn indent(self, left: Pixels) -> Self`, `fn gap(self, gap: Pixels) -> Self`,
    `fn focused(self, bool) -> Self` (its list has focus),
    `fn label(self, impl Into<SharedString>) -> Self` (`aria_label`),
    `fn description(self, impl Into<SharedString>) -> Self`
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
    (`{repo}` = "manyhands"), `detail` and action. Include: never
    refreshed, stale and incomplete together (never refreshed wins); stale
    with `listed` 0 (alert over no rows, not empty); Ready with `listed` 1
    but no conforming row (Rows, not Empty, P27); a narrowed empty list;
    Closed empty (no action); Failed with the failure's message as
    `detail`; Loading over kept rows (Rows, no alert).
  - E-11: `desktop::assets::tests::every_item_view_icon_loads` and its
    array length gain `repeat`, `bug`, `sparkles`, `list-checks`,
    `file-plus`, `folder-plus`, `chevrons-down-up`, `arrow-up-down`,
    `list-filter`, `circle-x`, `file-exclamation-point`.
- [ ] **Step 2: Run them; they fail** (types and icons missing).
- [ ] **Step 3: Implement** `KeptList`, `list_body` and the icons
  (`ItemIcons` in `D/assets.rs`, fact 12). Then `ListRow` and the three
  state elements per Part A: hover `hover`; selected `selection` with a
  2 px inset `accent` left line, weight 600 and `aria_selected`; a 2 px
  inset `teal` ring when selected and focused; skeleton bars in
  `theme.skeleton` with no animation (R-6); the alert's icon
  `circle-alert` (amber) for Warning and `circle-x` (red) for Failure,
  with `detail` as a visible second line. Colours from `chrome_colors(cx)`
  (fact 31).
- [ ] **Step 4: Run the three tests by exact name; they pass.**
- [ ] **Step 5: Verify lines; commit.**

## Task 2: The Tickets Model (Part B)

**Files:**
- Modify: `D/model/tickets.rs`, `D/model/tickets_tests.rs` (E-1)
- Create: `D/model/ticket_view.rs`, `D/model/ticket_view_tests.rs`,
  `D/model/markers.rs`, `D/model/markers_tests.rs`, `D/model/status.rs`
- Modify: `D/view/item/chips.rs` (uses `model::status`; `relative_time`
  moves to `D/model/ticket_view.rs` or is re-exported `pub(crate)`),
  `D/view/workspace/lists.rs` (the read, the setters, `sync_tickets`,
  `open_ready_tickets`), `D/view/lists.rs` (`query()` by reference),
  `D/view/tickets_tests.rs` (E-2)

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
    `TicketFilter::default()`; `to_filter`, `names_filter` and the names
    read go.
  - `TicketRow` gains `pub readiness: Option<ReadinessState>` and
    `pub context: RowContext`; `reasons` is filled for every blocked row,
    named from all the read's rows.
  - `Tickets`: `list: KeptList<Vec<TicketRow>>`; `fn query(&self) -> &TicketQuery`;
    `fn set_query(&mut self, query: TicketQuery)` changes no read state;
    `fn list(&self) -> &KeptList<Vec<TicketRow>>`;
    `fn load(&mut self, repo: RepoKey, …)` calls `reload()` for the same
    repository and `clear()` for another, and reports whether the
    repository changed (Part D's text clear).
- Produces, in `D/model/markers.rs`:
  - `pub(crate) enum ContextKind { Main, WorkingCopy, Unchecked }` and
    `pub(crate) struct RowContext { pub kind: ContextKind, pub branch: String, pub worktree: String }`
    with `fn of(item: &ItemDto) -> RowContext` (Primary, Active,
    Unverified).
  - `pub(crate) struct Markers { pub unsaved: bool, pub conflict: bool, pub closed: bool }`
  - `pub(crate) fn row_markers(held: Held, conflict: bool, closed: bool, closure: ClosureFilter) -> Markers`
- Produces, in `D/model/status.rs`: the header's `Tint` and
  `pub(crate) fn tint(status: &str) -> Option<Tint>` moved from
  `chips.rs`, with "ready" added as `Done`.
- Produces, in `D/model/ticket_view.rs`:
  - `pub(crate) struct SegmentCounts { pub open: usize, pub closed: usize, pub all: usize }`
  - `pub(crate) struct TicketView { pub rows: Vec<TicketRow>, pub counts: SegmentCounts, pub elsewhere: usize, pub listed: usize, pub shown: usize, pub needs_fixing: usize, pub unreadable: usize }`
  - `pub(crate) fn derive(rows: &[TicketRow], query: &TicketQuery) -> TicketView`
  - `pub(crate) fn footnote(view: &TicketView, query: &TicketQuery, repo: &str) -> String`
  - `pub(crate) fn hint(view: &TicketView, query: &TicketQuery) -> Option<String>`
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
    - All → `a, e, d, c, b, n`.
    - text " VH-AAAAA " under Open → `a, n`; counts 1/1/2; `elsewhere` 1;
      footnote `1 open ticket in manyhands matches "VH-AAAAA" · 1 needs fixing`.
    - the same text under All → `a, c, n`; `elsewhere` 0.
    - text = `d`'s ULID in lowercase → `d, n`.
    - text "zzz" → `n` only; `listed` 1; `shown` 0 (P27).
    - Ready → `a, e, d, n`; Blocked → `b, n`; counts unchanged by
      readiness; Closed then Ready → closure Open (R-26).
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
  - `desktop::model::ticket_view::tests::ages_read_as_the_prototype_writes_them`,
    `now` fixed: 30 s → "0m"; 59 min → "59m"; 60 min → "1h"; 89 min →
    "1h"; 90 min → "2h"; 23 h → "23h"; 24 h → "1d"; 36 h → "2d"; 400 d →
    "400d"; future → "0m"; `None` → ""; "garbage" → "".
  - `desktop::model::markers::tests::markers_follow_drafts_conflicts_and_closure`:
    `Held::Yes` → unsaved; `No` and `Unknown` → not; conflict → conflict;
    closed under All → closed; closed under Closed and Open → not.
  - `desktop::model::ticket_view::tests::tooltips_carry_the_id_context_and_reasons`:
    a coded row on main → `vh-aaaaa · manyhands`, `ID {ulid}`, `Type
    cycle`, "Main copy", `Git: branch main · worktree {root}`; a no-code
    row → `No short code · manyhands`; a working copy → "Private working
    copy" and its branch and worktree; unverified → "Working copy not
    checked yet"; blocked by an open ticket → "Waits for open ticket:
    vh-aaaaa Dock layout"; on a cycle → its "On a dependency cycle: …"
    line; nonconforming → only `Needs fixing: {guidance} Open it to fix.`;
    closed → a `Closed` line.
  - E-1 in `D/model/tickets_tests.rs`: the five tests become
    `desktop::model::tickets::tests::a_read_lands_only_for_its_repository_and_load`,
    one table: a read for the load in flight lands; one for an earlier
    load, another repository, or after `clear` is dropped; a query set
    before any load is kept; a query change starts no read and keeps the
    rows. `the_default_query_lists_open_tickets` asserts the default
    query and `read_filter() == TicketFilter::default()`; the two
    `blocked_reasons` tests build `TicketFilter` literals.
  - E-2 in `D/view/tickets_tests.rs`: delete the two tests; the
    `library()` and `blocked_list` helpers take `TicketFilter` literals.
- [ ] **Step 2: Run the new tests; they fail.**
- [ ] **Step 3: Implement** the model per Part B, then every site the
  `TicketQuery` change reaches: `D/view/lists.rs:58`, `:77`;
  `D/view/workspace/lists.rs:125`, `:150-199` (the setters call
  `set_query` and read nothing; `open_ready_tickets` follows R-29);
  `chips.rs` uses `model::status::tint`. `sync_tickets` passes
  `derive(…).rows` to the pane and, until Task 3, strips `reasons` from
  rows unless Blocked is chosen, so today's view draws as before
  (`D/view/tickets.rs:265`).
- [ ] **Step 4: Run** the new tests, the edited
  `the_default_query_lists_open_tickets`, the two `blocked_reasons` tests
  and the E-2 helper users (`arrows_move_in_the_list_and_enter_opens`,
  `ready_and_blocked_…`, `the_why_blocked_panel_…`) by exact name; they
  pass.
- [ ] **Step 5: Verify lines; commit.**

## Task 3: The Tickets Panel (Part C)

**Files:**
- Modify: `D/view/tickets.rs`, `D/view/lists.rs` (`tickets_body`,
  `choice_ids`), `D/view/workspace.rs` (`choice_focus`, the
  `ListEvent::Confirm` subscription, the sort focus handles)
- Create: `D/view/tickets/filter_bar.rs`, `D/view/tickets/columns.rs`, `D/view/tickets/row.rs`
- Modify: `D/view/workspace/lists.rs`, `D/view/dock/panels.rs` (`listed()`
  and "Updating…"), `D/view/dock/header/tabs.rs` (an id on the count),
  `D/keys.rs`, `D/keys_tests.rs` (E-12)
- Tests: `D/view/tickets_tests.rs` (E-3 and new), `D/view/keyboard_tests.rs`
  `change_the_filters` (E-6 part), `D/view/lifecycle/made_ready_tests.rs`,
  `made_ready_review_tests.rs` (E-10 part)

**Interfaces:**
- Consumes: `ListRow`, `list_body`, `alert_row`, `empty_state`,
  `skeleton_rows` (Task 1); `derive`, `footnote`, `hint`, `age_label`,
  `ticket_tooltip`, `row_markers`, `tint` (Task 2).
- Produces:
  - `pub(crate) struct ShownTicket { pub row: TicketRow, pub markers: Markers }`;
    `TicketsPane::set(&self, rows: Vec<ShownTicket>, window, cx)` keeps the
    selection by ID and clears it when the row is gone, and does nothing
    when the rows are unchanged.
  - `Workspace::set_sort(&mut self, column: SortColumn, cx)`,
    `Workspace::clear_filters(&mut self, window, cx)`,
    `Workspace::show_all(&mut self, window, cx)`.
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
    `("panel-count", node)` on the header's count.

- [ ] **Step 0: U-5.** Read how the Workspace is notified of draft
  registry and pending-conflict changes (the tab strip's unsaved dot,
  `D/view/workspace.rs:193`; `requests.rs:113-116`). If either does not
  notify the Workspace, record it and add the notify in this task.
- [ ] **Step 1: Write the failing tests.**
  - `desktop::view::tickets::tests::the_filter_bar_works_without_a_read`:
    a fixture repository with tickets like Task 2's. On load, exactly one
    `list_tickets` read with the default filter. Snapshot reads, requests
    and tabs; after every later step assert the `ticket-row` labels, the
    segments' names ("Open, 4 tickets" …), `tickets-footnote`,
    `("panel-count", _)`, and that reads, requests and tabs are unchanged
    (P25, P29). Steps: click `filter-closed`; Tab to `filter-all`, Enter;
    focus `filter-readiness`, Enter, Down to "Blocked", Enter (menu by
    keyboard; the button's name "Readiness: blocked"; Why blocked shows;
    focus is back on the button); choose "Any readiness"; with Closed
    chosen choose Ready → `filter-open` selected (R-26); type a code
    shared with a closed ticket under Open → `tickets-hint` reads "1
    closed ticket also matches.", activate `tickets-show-all` → All, focus
    in the filter input; click `sort-title` (ascending, name "Sort by
    title, A to Z"; `sort-updated` reads "Sort by updated"), Tab to it and
    Enter (reversed, "Sort by title, Z to A"); focus the list, `home` and
    `end` select the first and last row (P18). Review Focus 1: select a
    row, choose a lifecycle that hides it, then Enter in the list opens
    nothing (P30).
  - `desktop::view::tickets::tests::list_states_act_and_a_reload_keeps_the_rows`:
    after a finished job, while the reload is held, the old rows stay and
    `("panel-count", _)` reads "Updating…"; a held busy reload shows
    `tickets-alert` with "manyhands was busy, so this list wasn't
    updated." over the same rows, and its Retry sends one read; a
    narrowed empty list's Clear filters empties the text, sets Any
    readiness and focuses the filter input; on a stale or never-refreshed
    repository, Refresh submits `IndexRefresh` (use the state the fixtures
    can make, as D1's list-state tests do; if neither, stop for the owner).
  - E-3, E-6 (`change_the_filters`), E-10 (made-ready tests) as the design
    words them. E-12: `each_key_is_bound_to_its_action` gains `home` →
    `SelectFirst` and `end` → `SelectLast`.
- [ ] **Step 2: Run them; the new ones fail.**
- [ ] **Step 3: Implement** Part C: the filter bar (segments restyled from
  today's `choice`, R-4; the readiness `Button::dropdown_menu`, R-5), the
  hint, the column header outside the list (R-2), `ShownTicket` rows as
  `ListRow`s with Part C's cells, marker icons and tooltips, the footnote,
  Why blocked under Blocked, the body from `list_body`, the header count
  from `listed` with "Updating…" (R-11), and focus after actions (R-28).
  `TicketView` is cached until the read or the query changes; markers are
  worked out in every Workspace render from `held_by` and the
  repository's `pending_conflicts` (R-32). Find stays at the top of the
  panel until Task 4. Remove the "Showing …" texts and the inline reasons
  line; keep `LIFECYCLE_OPEN`, `LIFECYCLE_CLOSED` and `lifecycle_label`
  (fact 14).
- [ ] **Step 4: Run** the new tests and every edited one by exact name;
  they pass.
- [ ] **Step 5: Verify lines; commit.**

## Task 4: The Filter Input Replaces Find (Part D)

**Files:**
- Modify: `D/view/find.rs` (it now draws the filter input and its status
  line; `find_bar`, `match_row` and their constants go),
  `D/view/workspace/find.rs`, `D/view/tickets/filter_bar.rs` (places the
  input), `D/model/find.rs` (the short-code half goes), `D/view/lists.rs`,
  `D/view/workspace.rs`, `D/view/workspace/repositories.rs` (the clear on
  a repository change)
- Tests: `D/view/find_tests.rs` (E-4), `D/model/find_tests.rs` (E-5),
  `D/view/keyboard_tests.rs` `find`, `find_by_short_code_and_ulid` (E-6
  part), `D/view/dock_tests/state.rs` (E-10 part)

**Interfaces:**
- Consumes: `TicketQuery.text`, `set_query`, `load`'s repository-changed
  answer (Task 2); the filter bar (Task 3).
- Produces: `pub(crate) const FILTER_PLACEHOLDER: &str = "Filter by title or code";`
  `Workspace::set_filter_text(&mut self, text: &str, window, cx)`;
  `Workspace::submit_filter(&mut self, window, cx)`;
  `pub(crate) fn parse_id(text: &str) -> Option<ItemId>` in
  `D/model/find.rs`, beside the kept stamped ID read and its states;
  element id `filter-status`.

- [ ] **Step 1: Rewrite the tests (E-4, E-5) first; they fail.**
  `desktop::view::find::tests` per E-4, covering: under Open, a closed
  ticket's ULID in lowercase with spaces, Enter → its tab, "Opening…"
  while held, focus stays in the input (P8, Review Focus 2); a
  document's ULID → its tab; an unknown ULID → "Nothing in manyhands has
  that ID."; a ULID read held while another repository is selected →
  dropped, no tab; a code one closed ticket alone carries, under Open,
  Enter → it opens (P9); a duplicate code, Enter → no tab, focus in the
  list with the first match selected; selecting another repository →
  the input's value and the query's text are empty and no row of the
  first repository is drawn, while a Retry of the same repository keeps
  the text (P17, Review Focus 4). `desktop::model::find::tests` per E-5.
  Then E-6 and E-10 as the design words them.
- [ ] **Step 2: Run them; they fail.**
- [ ] **Step 3: Implement** Part D: the existing `find_input` becomes the
  filter bar's input (placeholder, `list-filter` prefix, `cleanable`,
  `clean_on_escape`); its Change sets `query.text`; PressEnter runs
  `submit_filter`; `secondary-p` focuses it; the repository-change clear
  is applied at the next render (R-30). Remove the match cards, the
  short-code lookup and `find_tickets_by_slug`'s call; keep the stamped ID
  read (R-31).
- [ ] **Step 4: Run** the rewritten and edited tests by exact name.
- [ ] **Step 5: Verify lines; commit; ticket comment** (Tickets done).

## Task 5: The Documents Model (Part E)

**Files:**
- Modify: `D/model/documents.rs`, `D/model/documents_tests.rs`,
  `D/model/item.rs`, `D/view/item/repair.rs`, `D/view/item/banners.rs`,
  `D/view/item/repair/adopt.rs` (the `unmanaged` callers),
  `D/view/documents.rs` (`TreeItem::expanded` from the model, `:286`),
  `D/view/workspace/lists.rs` (`documents_changed`, the kept tree)

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
    the combined reads; an `Expansion` per repository for the session;
    `fn expansion(&self) -> Option<&Expansion>` (the selected
    repository's), `fn set_expanded(&mut self, path: &str, expanded: bool)`,
    `fn collapse_all(&mut self)`.
  - `pub(crate) fn visible_selection(nodes: &[TreeNode], expansion: &Expansion, selected: &str) -> Option<String>`
    (the selected id, or its nearest visible ancestor's, R-27).
  - `pub(crate) enum Trailing { None, Count(usize), Empty }`,
    `pub(crate) enum DocKind { Folder, Document, Nonconforming { adoptable: bool }, Unreadable }`,
    `pub(crate) struct DocRow { pub label: String, pub kind: DocKind, pub expanded: bool, pub trailing: Trailing, pub tooltip: Vec<String> }`
  - `pub(crate) fn doc_row(node: &TreeNode, expanded: bool) -> DocRow`

- [ ] **Step 0: Merge `origin/main`** (Global Constraints, Main).
- [ ] **Step 1: Write the failing tests.**
  - `desktop::model::documents::tests::expansion_survives_rebuilds_and_collapse_all`:
    a tree `guides/deep`, `plans`, `empty`; nothing collapsed → all
    expanded; collapse `guides` → rebuild with a new folder `notes` →
    `guides` collapsed, `notes` expanded; rebuild without `plans`, then
    with it → `plans` keeps its state (Review Focus 5); `collapse_all` →
    every folder collapsed; `expand_ancestors("guides/deep/new")` expands
    `guides` and `guides/deep`; another repository's expansion is
    separate; `visible_selection` of a document inside collapsed
    `guides/deep` → `folder:guides`.
  - `desktop::model::documents::tests::rows_read_as_the_design_writes_them`,
    one table over Part E's eight kinds: label, kind, `expanded`,
    trailing and tooltip lines. Include a collapsed folder with three
    documents at two depths (`Count(3)`), a leaf folder holding only a
    non-document file (`Count(0)`), an empty folder (`Empty`, "Empty
    folder. Shared once it contains a document."), a nonconforming
    `missing_front_matter` (file name, adoptable, "Not a managed document
    yet. Open to adopt it." then the path) and a `missing_field` one (not
    adoptable, "Needs fixing: {guidance} Open it to fix." then the path).
- [ ] **Step 2: Run them; they fail.**
- [ ] **Step 3: Implement** Part E; the existing tree view keeps working
  (it builds `TreeItem::expanded` from `Expansion`).
- [ ] **Step 4: Run** the two tests by exact name; they pass.
- [ ] **Step 5: Verify lines; commit.**

## Task 6: The Documents Panel (Part F)

**Files:**
- Modify: `D/view/documents.rs`, `D/view/lists.rs` (`documents_body`),
  `D/view/workspace.rs` (the `TreeEvent` subscription in `new`),
  `D/view/workspace/lists.rs` (`open_selected_document` toggles a folder;
  `SelectFirst` and `SelectLast` for the tree)
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
  its tab. Reads and requests unchanged by every step but the refresh
  (P25). Then E-8 and E-9 as the design words them.
- [ ] **Step 2: Run it; it fails.**
- [ ] **Step 3: Implement** Part F: the base `Tree` with `ListRow` rows
  and `.vertical_scrollbar` (fact 4), indent `8 + 14 × depth`, chevron or
  spacer, icon, label, trailing cell and the unsaved marker; each row's
  `aria_label` the label and `aria_description` the tooltip lines; a
  document row's `on_click` opens it; the body from `list_body`; Enter on
  a folder toggles it through the model; the `TreeEvent` subscription maps
  `folder:{path}` ids to `Documents::set_expanded`; selection after a
  rebuild by `visible_selection`. Remove the second-line cells and the
  "Empty folder" text.
- [ ] **Step 4: Run** the new test and every edited one by exact name.
- [ ] **Step 5: Verify lines; commit.**

## Task 7: Unreadable Files (Part I)

**Files:**
- Create: `D/model/unreadable.rs`, `D/model/unreadable_tests.rs`
- Modify: `D/services/reads.rs` (an `index_status` read),
  `D/view/workspace/lists.rs` (read it with each load, under the lists'
  stamp), `D/model/tickets.rs` and `D/model/documents.rs` (take the rows),
  `D/model/ticket_view.rs` (`unreadable` count), `D/view/tickets/row.rs`,
  `D/view/documents.rs`
- Test: `D/view/tickets_tests.rs`

**Interfaces:**
- Produces: `pub(crate) struct UnreadableFile { pub path: String, pub worktree: String }`;
  `pub(crate) fn unreadable_rows(status: &IndexStatusDto) -> (Vec<UnreadableFile>, Vec<UnreadableFile>)`
  (tickets, documents); `TicketRow` and `TreeNode` each gain an unreadable
  form that opens by path as `OpenTarget::Inspect`.

- [ ] **Step 1: Write the failing tests.**
  - `desktop::model::unreadable::tests::unreadable_files_go_to_their_list`:
    `SourceUnreadable` at `.manyhands/tickets/X/ticket.md` → tickets;
    at `docs/a/b.md` → documents; at `docs/a` (a directory) → neither; at
    `src/x.md` → neither; `MissingFrontMatter` at a ticket path → neither.
  - `desktop::view::tickets::tests::an_unreadable_file_is_listed_with_its_reason`
    (`#[cfg(unix)]`): a ticket file's permissions set to none, then a
    refresh job; its row shows the path and a `ticket-status` of "can't be
    read" under every lifecycle; Enter opens an inspection tab by path;
    the footnote ends `· 1 can't be read`. Restore the permissions before
    the fixture is dropped.
- [ ] **Step 2: Run them; they fail.**
- [ ] **Step 3: Implement** Part I. A busy or failed status read leaves
  the lists without unreadable rows and does not fail them.
- [ ] **Step 4: Run** the two tests by exact name.
- [ ] **Step 5: Verify lines; commit.**

## Task 8: Panel Toolbars (Part G)

**Files:**
- Create: `D/view/dock/header/toolbar.rs`, `D/view/dock_tests/toolbars.rs`
- Modify: `D/view/dock/header.rs` (`mod toolbar;`; passes the Workspace
  handle to `single` and `tabs`), `D/view/dock/header/tabs.rs`,
  `D/view/workspace/focus.rs` (`focused_region` and `cycle_region` count
  a toolbar button as its region), `D/view/workspace.rs` (the buttons'
  focus handles), `D/view/lists.rs` and `D/view/documents.rs` (body
  buttons and their wrappers go), `D/view/dock_tests/mod.rs`
- Tests: `D/view/keyboard_tests/editing.rs`, `D/view/documents_tests.rs`
  `new_folder_opens_its_dialog` (E-7); `folders_keep_their_state_and_toggle_by_key`
  gains the Collapse all step

**Interfaces:**
- Consumes: `Documents::collapse_all`, `visible_selection` (Task 5).
- Produces: `Workspace::panel_toolbar(&mut self, kind: PanelKind, window, cx) -> Option<AnyElement>`,
  drawn by `single` at the right of the title row and by `tabs` at the
  right of the strip for the active tab (R-13), inside a div with the
  `Navigation` key context. Buttons `new-ticket-button`,
  `new-document-button`, `new-folder-button`, `collapse-all-button`, each
  with a Workspace-owned focus handle, `unless_dialog`, and the disabled
  states of Part G.

- [ ] **Step 1: Write the failing test**
  `desktop::view::dock::tests::toolbars::panel_toolbars_work_by_tab_and_enter`:
  in the default layout, Tab to `new-ticket-button` → `Region::Tickets`
  holds focus and its ring shows; Enter → the New ticket… dialog
  (cancel); `secondary-n` there → New ticket… (cancel); F6 from it → the
  next region of decision 9 (C1). Tab to `new-document-button`, Enter →
  New document…; `new-folder-button` → New folder…;
  `collapse-all-button` → every folder collapsed, the hidden selection on
  its top-level folder (P20), and the button disabled. Move Documents
  into the Tickets group as a tab (the dock's move, as C1's tests do) →
  with Documents active its toolbar is drawn and works; with Tickets
  active, New ticket's. With no repository selected, the three New
  buttons are disabled. With a dialog open, activating a toolbar button
  does nothing (P24). Each button's accessible name is the design's.
  Extend `folders_keep_their_state_and_toggle_by_key` with Collapse all by
  click. Then E-7.
- [ ] **Step 2: Run it; it fails.**
- [ ] **Step 3: Implement** Part G (U-3: if the header cannot draw
  Workspace-built buttons or focus cannot be counted as the region, stop
  and record it for the owner; decision 18's alternative is a body row).
- [ ] **Step 4: Run** the new and edited tests by exact name.
- [ ] **Step 5: Verify lines; commit.**

## Task 9: Checklist, Gates, Review-Ready

- [ ] **Step 1: On-screen checklist.** Write
  `docs/plans/2026-11-11-wave-04-cycle-02-on-screen-checklist.md` in
  Cycle 01's format, for the owner on a release build under Wayland
  against a scratch data directory: both panels against frames 1a and 1b
  in the dark theme; every list state reached (a never-refreshed
  repository, a busy one, an unreadable file); the menu, segments, column
  headers and toolbar by keyboard with focus visible; tooltips; a narrow
  left dock (K-2); Orca reading a row, a segment and the readiness button.
  Claude records only what the owner reports.
- [ ] **Step 2: Gates**, once on the final tree: AGENTS.md's three static
  gates, then its fast tier with `--no-fail-fast`. No SSH target (nothing
  under its paths changed). A failure outside the E-list is a product bug
  (fixed), an appearance test (deleted under E-13) or a needed edit
  (E-14 onward).
- [ ] **Step 3: Merge check.** `git merge-tree --write-tree HEAD origin/main`
  is clean; the Boundary command prints nothing.
- [ ] **Step 4: Fill** the Cycle's Acceptance Status column from the tests
  by exact name and the ledger; note where the code differs from the
  design for the owner's amendment.
- [ ] **Step 5: Commit, mark review-ready, ticket comment.** Not done
  without the owner: a PR, the merge, `ticket close`, cleanup, native CI.

## Plan Self-Review

- **Spec coverage.** Part A → Task 1; B → 2; C → 3; D → 4; E → 5; F → 6;
  I → 7; G → 8; H → Tasks 3 (Home and End), 4 (`secondary-p`), 6 (Enter on
  a folder) and 8 (toolbar focus). P1–P7, P10, P13, P27 → Task 2's tables
  and Task 3's journey; P8, P9, P17 → Task 4; P11, P12, P14, P18, P29,
  P30 → Task 3; P15, P16 → Tasks 1, 3; P19–P23, P26 → Tasks 5, 6; P20,
  P24 → Task 8; P25 → Tasks 3, 6; P28 → Task 7. Decisions 1–20 each land
  in the task of their Part. Amendments go in the approval commit, before
  Task 0.
- **Types.** `KeptList`, `ListBody`, `BodyInput`, `TicketQuery`,
  `TicketSort`, `SortColumn`, `TicketView`, `Markers`, `RowContext`,
  `ShownTicket`, `Expansion`, `DocRow`, `DocKind`, `UnreadableFile` are
  defined once and used by the names above.
- **Review Focus.** Each line names its test and task.
- **Proportion.** Signatures, test names and assertions only; bodies are
  the implementer's.
