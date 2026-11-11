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
> revision 1: implementation waits for the owner's approval of the three
> documents and a separate go-ahead. Re-read the design before each task
> and record every checkpoint on ticket `01M757DHFAVPEVY4TWQTVKQAMJ`.
> Steps use checkbox (`- [ ]`) syntax.

**Goal:** Rebuild the Tickets and Documents panels to `03-panels.md`: one
row anatomy and plain list states, a Tickets panel filtered, counted and
sorted in the desktop over one read, with Find folded into its filter
input, and a Documents tree whose folders stay as the person leaves them.

**Architecture:** Pure models first: kept rows and list bodies (Part A),
the ticket view (Part B), document rows and expansion (Part E). Then the
views that draw them: a shared `ListRow`, a Kit `List` with the app's
column header for Tickets, the base Kit `Tree` for Documents, and
toolbars in Cycle 01's panel header. No library change.

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
checkpoint with the owner on the shared header and rows.

**GPUI** only through `gpui_kit::*`, never a glob import.

**Copy.** Every user-visible string is the design's, verbatim, held in a
`pub(crate) const` beside its use, so tests compare constants. Sentence
case, no exclamation marks, `…` (U+2026) on an action that opens a
dialog.

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
  name, each named edit included. No whole-suite run before Task 8.
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
icons, tones, indentation or that a view renders.

**Test isolation.** Fixture data directories only; no network; no real
waits (`advance_clock`); `now` passed in.

**Existing tests** change only as E-1 to E-13 say. Any other edit is
pre-authorized (decision 16): recorded in the ledger as E-14 onward with
its reason, checked by the task's reviewer, and an assertion removal is
called out in the ledger and the review-ready summary.

**Dead code.** An item first used by a later task carries
`#[cfg_attr(not(test), allow(dead_code))] // Used from Task N.`; Task N
removes it. No test-only code in production paths.

**Commits.** One per task, conventional message, ending
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Ticket records.** A comment through `devenv shell -- cargo run --locked --bin manyhands-cli -- comment add …`,
run against the main checkout while the ticket worktree is clean, after
Tasks 0, 4 and 8: commits, tests with counts, rulings. Before
`ticket save`, commit any owned `ticket.md` edit.

**Ledger.** `docs/plans/2026-11-11-wave-04-cycle-02-tickets-and-documents-execution.md`,
created in Task 0, written only by the orchestrator.

**File size.** New source files stay below about 500 lines.

## Review Focus

Inputs no task's main journey exercises that would most likely bite a
person, each pinned by a test in the task named:

1. **A filter that hides the selected row, then Enter.** The Kit list
   keeps a selection index, so after the rows change it could point at a
   different ticket. Enter must open the selected row's own ticket or
   nothing (Task 3, in `the_filter_bar_works_without_a_read`).
2. **Pasted text with spaces or capitals** (" VH-R2N89 ", a ULID in
   lowercase) must match and open as typed cleanly (Task 2 table rows;
   Task 4 test).
3. **A ticket with an empty status or no change time** sorts last by that
   column in both directions, and its age is blank (Task 2 table rows).
4. **Switching repositories with filter text typed and rows kept:** the
   new repository never shows the old one's rows or text (Task 1
   `kept_rows_follow_reloads`; Task 4 test).
5. **A folder that disappears and comes back between reloads** keeps the
   person's choice; a new folder starts expanded (Task 5 table row).

## File Map And Dependency Order

| Task | Part | Summary |
| --- | --- | --- |
| 0 | — | Merge main, baseline gates, re-check facts, E-sweep, ledger. |
| 1 | A | `KeptList`, list bodies, `ListRow`, state elements, icons; E-11. |
| 2 | B | Ticket query, rows, one read, `derive`, footnote, age, markers, tooltip; E-1, E-2. |
| 3 | C | Tickets panel: filter bar, column header, rows, footnote, states, Home and End; E-3, E-6 part, E-10 part, E-12 part. |
| 4 | D | The filter input replaces Find; E-4, E-5, E-6 part, E-10 part; checkpoint. |
| 5 | E | Documents model: expansion, `doc_row`, kept tree. |
| 6 | F | Documents panel: base tree, rows, keys; E-8, E-9, E-12 part. |
| 7 | G | Panel toolbars; Collapse all; E-7. |
| 8 | — | On-screen checklist, gates, review-ready; checkpoint. |

New source files:

- `D/model/list_view.rs`, `D/model/ticket_view.rs`, `D/model/markers.rs`
- `D/view/rows.rs`
- `D/view/tickets/filter_bar.rs`, `D/view/tickets/columns.rs`,
  `D/view/tickets/row.rs` (with `mod` lines in `D/view/tickets.rs`)
- `D/view/dock/header/toolbar.rs`

New test files: `D/model/list_view_tests.rs`,
`D/model/ticket_view_tests.rs`, `D/model/markers_tests.rs`,
`D/view/dock_tests/toolbars.rs` (a `mod toolbars;` in
`dock_tests/mod.rs`). New tests in existing files are named in their
tasks.

Order: 0 → 1 → … → 8, one at a time.

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
  `git diff --stat a0a734a4 HEAD -- src/desktop docs/Waves docs/RFC docs/design Cargo.lock`.
  Re-read each fact whose file changed and correct the design first. If
  `Cargo.lock` moved `gpui-kit`, `gpui-base` or `gpui-component`, stop for
  the owner: facts 1–12 rest on the locked versions.
- [ ] **Step 3: Sweep the E-list.** Count the test sites:

```sh
git grep -c -e 'ticket-lifecycle' -e 'tickets-showing' -e 'tickets-readiness-showing' \
  -e 'filter-any-readiness' -e 'filter-ready' -e 'filter-blocked' -e 'ticket-reason-' \
  -- 'src/desktop/**/*_tests.rs' 'src/desktop/view/keyboard_tests*' 'src/desktop/view/dock_tests'
git grep -c -e 'find-match' -e 'find-state' -e 'FIND_' -e 'FindResult' \
  -- 'src/desktop/**/*_tests.rs' 'src/desktop/view/keyboard_tests*' 'src/desktop/view/dock_tests'
git grep -c -e 'doc-path' -e 'doc-empty' -e 'doc-nonconforming' -e 'doc-problem' \
  -e 'EMPTY_FOLDER' -e 'new-ticket-button' -e 'new-document-button' -e 'new-folder-button' \
  -- 'src/desktop/**/*_tests.rs' 'src/desktop/view/keyboard_tests*' 'src/desktop/view/dock_tests'
```

  Record the counts per file in the ledger. Each file must be one E-1 to
  E-13 names, or a click on a toolbar id that stays (`create_tests.rs`,
  `editing_privacy_tests.rs`); any other is a pre-authorized edit
  (decision 16), recorded as E-14 onward.
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
    `fn apply<V>(&mut self, result: ReadResult<V>, ready: impl FnOnce(V) -> ListState<T>)`,
    `fn state(&self) -> &ListState<T>`,
    `fn rows(&self) -> Option<&T>` (the Ready rows, or the kept rows while
    Loading or busy), `fn busy_over_kept(&self) -> bool`.
- Produces, in `D/model/list_view.rs`:
  - `pub(crate) enum ListKind { Tickets, Documents }`
  - `pub(crate) enum ListAction { Refresh, Retry, RebuildIndex, ClearFilters, NewItem }`
  - `pub(crate) enum AlertTone { Warning, Failure }`
  - `pub(crate) struct Alert { pub tone: AlertTone, pub text: String, pub detail: Option<String>, pub action: ListAction }`
  - `pub(crate) enum ListBody { NoRepository, Skeleton, Rows { alert: Option<Alert> }, Empty { text: String, action: Option<ListAction> }, Problem(Alert) }`
  - `pub(crate) struct BodyInput<'a> { pub kind: ListKind, pub repo: Option<&'a str>, pub shown: usize, pub closure: ClosureFilter, pub narrowed: bool }`
    (`closure` is ignored for Documents).
  - `pub(crate) fn list_body<T: Rows>(list: &KeptList<T>, input: BodyInput<'_>) -> ListBody`
- Produces, in `D/view/rows.rs`:
  - `pub(crate) const ROW_HEIGHT: Pixels` (28 px).
  - `pub(crate) struct ListRow` with `fn new(id: impl Into<ElementId>) -> Self`,
    `fn indent(self, left: Pixels) -> Self`, `fn focused(self, bool) -> Self`
    (the list has focus), `fn label(self, impl Into<SharedString>) -> Self`
    (its `aria_label`), `fn description(self, impl Into<SharedString>) -> Self`
    (`aria_description`), `fn tooltip(self, impl Into<SharedString>) -> Self`;
    `impl Selectable`, `ParentElement`, `RenderOnce`.
  - `pub(crate) fn skeleton_rows(count: usize, cx: &App) -> impl IntoElement`,
    `pub(crate) fn alert_row(id, alert: &Alert, on_action: impl Fn(&mut Window, &mut App) + 'static, cx: &App) -> impl IntoElement`,
    `pub(crate) fn empty_state(id, text, action: Option<(SharedString, handler)>, cx: &App) -> impl IntoElement`.

- [ ] **Step 1: Write the failing tests.**
  - `desktop::model::list_state::tests::kept_rows_follow_reloads`, one
    table. Start → op → expected `(state kind, rows(), busy_over_kept())`:
    Ready(a) → reload → (Loading, Some(a), false); then ok(b) →
    (Ready, Some(b), false); Ready(a) → reload → busy → (Ready, Some(a),
    true); Ready(a) → reload → failed → (Failed, None, false); Loading →
    busy → (Busy, None, false); Ready(a) → clear → (Loading, None, false).
  - `desktop::model::list_view::tests::each_state_has_its_own_body`, one
    table over every row of the design's Part A state table, for Tickets
    and for Documents, asserting the `ListBody` variant, its exact text
    (the design's constants with `{repo}` = "manyhands") and its action.
    Include: never refreshed, stale and incomplete together (never
    refreshed wins); stale with no rows (alert, not empty); a narrowed
    empty list ("No tickets match these filters.", ClearFilters); Closed
    empty (no action); Failed with the failure's message as `detail`.
  - E-11: `desktop::assets::tests::every_item_view_icon_loads` gains
    `repeat`, `bug`, `sparkles`, `list-checks`, `file-plus`,
    `folder-plus`, `chevrons-down-up`, `arrow-up-down`, `list-filter`,
    `file-exclamation-point`, `circle-check`.
- [ ] **Step 2: Run them; they fail** (types and icons missing).
- [ ] **Step 3: Implement** `KeptList`, `list_body` and the icons
  (`ItemIcons` in `D/assets.rs`, fact 12). Then `ListRow` and the three
  state elements per Part A: hover `hover`, selected `selection` with a
  2 px inset `accent` left line and weight 600 on the row, a 1 px inset
  `teal` ring when selected and focused; skeleton bars in
  `theme.skeleton` with no animation (R-6); the alert row's icon is
  `circle-alert`, amber for Warning and red for Failure. Colours come from
  `chrome_colors(cx)` (fact 29).
- [ ] **Step 4: Run the three tests by exact name; they pass.**
- [ ] **Step 5: Verify lines; commit.**

## Task 2: The Tickets Model (Part B)

**Files:**
- Modify: `D/model/tickets.rs`, `D/model/tickets_tests.rs` (E-1)
- Create: `D/model/ticket_view.rs`, `D/model/ticket_view_tests.rs`,
  `D/model/markers.rs`, `D/model/markers_tests.rs`
- Modify: `D/view/workspace/lists.rs` (the read, the setters, `sync_tickets`),
  `D/view/tickets_tests.rs` (E-2)

**Interfaces:**
- Consumes: `KeptList` (Task 1).
- Produces, in `D/model/tickets.rs`:
  - `pub(crate) enum SortColumn { Code, Title, Status, Updated }`
  - `pub(crate) struct TicketSort { pub column: SortColumn, pub ascending: bool }`,
    default `{ Updated, true }` (newest first), with
    `fn toggled(self, column: SortColumn) -> Self` (a new column
    ascending, the same one reversed).
  - `TicketQuery { pub closure: ClosureFilter, pub readiness: Option<ReadinessFilter>, pub text: String, pub sort: TicketSort }`,
    default Open, none, empty, default sort; `fn narrowed(&self) -> bool`
    (text not blank, or a readiness chosen).
  - `pub(crate) fn read_filter() -> TicketFilter` returns
    `TicketFilter::default()`; the `names_filter` and the second read go.
  - `TicketRow` gains `pub readiness: Option<ReadinessState>` and
    `pub working_copy: Option<WorkingCopy>`; `reasons` is filled for every
    blocked row, named from all the read's rows.
  - `Tickets`: `list: KeptList<Vec<TicketRow>>`; `fn set_query(&mut self, query: TicketQuery)`
    changes no read state; `fn list(&self) -> &KeptList<Vec<TicketRow>>`;
    `fn load` calls `reload()` for the same repository and `clear()` for
    another, and clears `query.text` on another.
- Produces, in `D/model/markers.rs`:
  - `pub(crate) struct WorkingCopy { pub branch: String, pub worktree: String }`
    and `fn of(item: &ItemDto) -> Option<WorkingCopy>` (context kind Active).
  - `pub(crate) enum Editing { WorkingCopy { copy: WorkingCopy, draft: bool }, Draft }`
  - `pub(crate) struct Markers { pub editing: Option<Editing>, pub conflict: bool, pub closed: bool }`
  - `pub(crate) fn row_markers(copy: Option<&WorkingCopy>, held: Held, conflict: bool, closed: bool, closure: ClosureFilter) -> Markers`
  - `pub(crate) fn editing_tooltip(editing: &Editing) -> String` with the
    design's marker texts.
- Produces, in `D/model/ticket_view.rs`:
  - `pub(crate) struct SegmentCounts { pub open: usize, pub closed: usize, pub all: usize }`
  - `pub(crate) struct TicketView { pub rows: Vec<TicketRow>, pub counts: SegmentCounts, pub shown: usize, pub needs_fixing: usize }`
    (`shown` counts conforming rows listed).
  - `pub(crate) fn derive(rows: &[TicketRow], query: &TicketQuery) -> TicketView`
  - `pub(crate) fn footnote(view: &TicketView, query: &TicketQuery, repo: &str) -> String`
  - `pub(crate) fn age_label(changed_at: Option<&str>, now: OffsetDateTime) -> String`
  - `pub(crate) fn ticket_tooltip(row: &TicketRow, repo: &str, waiting_on: Option<&str>) -> String`
  - `pub(crate) enum StatusTone { Teal, Green, Red, Amber, Muted, Neutral }`
    and `fn status_tone(status: &str) -> StatusTone` (not tested).

- [ ] **Step 1: Write the failing tests.**
  - `desktop::model::ticket_view::tests::derive_filters_counts_and_sorts`.
    Fixture rows in library order: `a` open, ready, code `vh-aaaaa`, title
    "Dock layout", status "in progress", 1 h old; `d` open, ready, no
    code, "zeta", status "Ready", 2 d old; `c` closed, code `vh-aaaaa`
    (duplicate), "Old dock", status "done", 3 d old; `b` open, blocked by
    `a`, code `vh-bbbbb`, "Footer toggles", status "", no change time;
    `n` nonconforming, path `.manyhands/tickets/X/ticket.md`. Rows of the
    table (query → expected ids in order, counts, `shown`,
    `needs_fixing`, footnote):
    - default → `a, d, b, n`; open 3, closed 1, all 4; 3; 1;
      `3 open tickets in manyhands · 1 needs fixing`.
    - Closed → `c, n`; footnote `1 closed ticket in manyhands · 1 needs fixing`.
    - All → `a, d, c, b, n` (newest first; `b` has no time, R-10).
    - text " VH-AAAAA " under All → `a, c, n`; counts open 1, closed 1,
      all 2; footnote
      `2 tickets in manyhands match "VH-AAAAA" · 1 needs fixing`.
    - text = `d`'s ULID in lowercase → `d, n`.
    - text "zzz" → `n` only; `shown` 0.
    - Ready → `a, d, n`; Blocked → `b, n` with `b.reasons` naming
      `vh-aaaaa Dock layout`; counts unchanged by readiness.
    - sort Title ascending → `a, b, d, n` (case-insensitive: "Dock",
      "Footer", "zeta"); descending → `d, b, a, n`.
    - sort Status ascending → `a, d, b, n` ("" last); descending
      `d, a, b, n` (empty still last).
    - sort Code ascending → `a, b, d, n` (no code last); descending
      `b, a, d, n`.
    - sort Updated descending (oldest first) → `d, a, b, n` (`b`, with
      no time, still last).
  - `desktop::model::ticket_view::tests::ages_read_as_short_units`, `now`
    fixed: 30 s → "now"; 59 min → "59m"; 60 min → "1h"; 23 h → "23h";
    24 h → "1d"; 364 d → "364d"; 365 d → "1y"; future → "now"; `None` →
    ""; "garbage" → "".
  - `desktop::model::markers::tests::markers_follow_the_copy_drafts_and_conflicts`:
    copy + `Held::No` → WorkingCopy draft false; copy + `Yes` → draft
    true; no copy + `Yes` → Draft; no copy + `Unknown` → none; conflict
    true → conflict; closed under All → closed true; closed under Closed
    and Open → false. And `editing_tooltip` of each variant equals the
    design's text, with `Git: branch {branch} · worktree {worktree}`.
  - `desktop::model::ticket_view::tests::tooltips_carry_the_id_and_the_reason`:
    a coded row → first line `vh-aaaaa · manyhands` and `ID {ulid}`; a
    no-code row → `No short code · manyhands` and its ID; blocked with
    `waiting_on` → a `Waiting on …` line; nonconforming → `Needs fixing:
    {guidance} Open it to fix.` with no ID line; closed → `Closed`.
  - E-1 in `D/model/tickets_tests.rs`: delete the five tests it names;
    `the_default_query_lists_open_tickets` asserts the default query and
    `read_filter() == TicketFilter::default()`.
  - E-2 in `D/view/tickets_tests.rs`: delete the two tests it names.
- [ ] **Step 2: Run the new tests; they fail.**
- [ ] **Step 3: Implement** the model per Part B. `derive` filters
  conforming rows by text (R-16), counts, then lifecycle and readiness,
  then a stable sort (R-10), then appends the nonconforming rows in their
  read order (R-9). The Workspace reads with `read_filter()` once per load;
  `set_closure` and `set_readiness` call `set_query` and read nothing;
  `sync_tickets` passes `derive(…).rows` to the pane, so today's view keeps
  showing the same rows until Task 3 (it draws the reasons line only under
  Blocked, as now).
- [ ] **Step 4: Run** the four new tests, `the_default_query_lists_open_tickets`,
  and the existing `desktop::view::tickets::tests::*` that remain (by
  exact name, listed in the ledger); they pass.
- [ ] **Step 5: Verify lines; commit.**

## Task 3: The Tickets Panel (Part C)

**Files:**
- Modify: `D/view/tickets.rs`, `D/view/lists.rs` (`tickets_body`)
- Create: `D/view/tickets/filter_bar.rs`, `D/view/tickets/columns.rs`, `D/view/tickets/row.rs`
- Modify: `D/view/workspace/lists.rs`, `D/keys.rs`, `D/keys_tests.rs` (E-12 part)
- Tests: `D/view/tickets_tests.rs` (E-3 and new), `D/view/keyboard_tests.rs`
  `change_the_filters` (E-6 part), `D/view/lifecycle/made_ready_tests.rs`,
  `made_ready_review_tests.rs` (E-10 part)

**Interfaces:**
- Consumes: `ListRow`, `list_body`, `alert_row`, `empty_state`,
  `skeleton_rows` (Task 1); `derive`, `footnote`, `age_label`,
  `ticket_tooltip`, `row_markers`, `status_tone` (Task 2).
- Produces:
  - `pub(crate) struct ShownTicket { pub row: TicketRow, pub markers: Markers }`;
    `TicketsPane::set(&self, rows: Vec<ShownTicket>, window, cx)`, keeping
    the selection by ID and clearing it when the row is gone.
  - `Workspace::set_sort(&mut self, column: SortColumn, cx)`.
  - Actions `SelectFirst` and `SelectLast` in `D/keys.rs`, bound to `home`
    and `end` in `TICKET_LIST_CONTEXT = "TicketList"` (a `key_context` on a
    div around the Kit list) and in `DOCUMENTS_CONTEXT` (used in Task 6).
  - Element ids: `filter-open`, `filter-closed`, `filter-all`,
    `filter-readiness`, `sort-code`, `sort-title`, `sort-status`,
    `sort-updated`, `ticket-row`, `ticket-type`, `ticket-slug`,
    `ticket-editing`, `ticket-conflict`, `ticket-closed`, `ticket-status`,
    `ticket-changed`, `tickets-footnote`, `tickets-alert`, `tickets-empty`,
    `tickets-skeleton`.

- [ ] **Step 1: Write the failing tests.**
  - `desktop::view::tickets::tests::the_filter_bar_works_without_a_read`:
    a fixture repository with tickets like Task 2's. Snapshot reads,
    requests and tabs at the start; after every step assert the
    `ticket-row` labels, the segments' accessible names ("Open, 3 tickets"
    …), `tickets-footnote`, and that reads, requests and tabs are
    unchanged (P25). Steps: click `filter-closed`; Tab to `filter-all`,
    Enter; focus `filter-readiness`, Enter, Down to "Blocked", Enter
    (menu by keyboard; the button's name becomes "Readiness: blocked";
    Why blocked shows; focus returns to the button, U-4); choose "Any
    readiness"; Tab to `sort-title`, Enter (sorted ascending, its name
    "Sort by title, A to Z", `sort-updated`'s "Sort by updated"), Enter
    again (reversed, "Sort by title, Z to A"); focus the list,
    `home` and `end` select the first and last row (P18). Review Focus 1:
    select `d`, type nothing but choose Closed (hides `d`), then Enter in
    the list opens nothing; choose Open, select a row, Enter opens that
    row's own ID.
  - `desktop::view::tickets::tests::a_reload_keeps_the_rows_and_a_busy_one_says_so`:
    after a finished job the old rows stay while the reload is held
    (`ticket-row` labels unchanged, no `tickets-skeleton`); a held busy
    reload shows `tickets-alert` with "manyhands was busy, so this list
    wasn't updated." over the same rows.
  - E-3, E-6 (`change_the_filters`), E-10 (made-ready tests) as the design
    words them. E-12 part: `each_key_is_bound_to_its_action` gains `home`
    and `end` in `TicketList`.
- [ ] **Step 2: Run them; the new ones fail.**
- [ ] **Step 3: Implement** Part C: the filter bar (segments restyled from
  today's `choice`, R-4; the readiness `Button::dropdown_menu`, R-5),
  the column header outside the list (R-2), `ShownTicket` rows as
  `ListRow`s with Part C's cells, marker icons and tooltips, the footnote,
  Why blocked under Blocked, and the body from `list_body`. Markers come
  from `row_markers` with `held_by` and the repository's
  `pending_conflicts`, computed in `sync_tickets` (R-8). Find stays at the
  top of the panel until Task 4. Remove the "Showing …" texts, the inline
  reasons line, `LIFECYCLE_*` and `READINESS_FILTERS`' showing strings.
  U-1: if the Kit draws its own selection over `ListRow`, stop and record
  it before working around it.
- [ ] **Step 4: Run** the new tests and every edited one by exact name;
  they pass.
- [ ] **Step 5: Verify lines; commit.**

## Task 4: The Filter Input Replaces Find (Part D)

**Files:**
- Modify: `D/view/find.rs` (it now draws the filter input; `find_bar`,
  `match_row` and their constants go), `D/view/workspace/find.rs`,
  `D/view/tickets/filter_bar.rs` (places the input), `D/model/find.rs`,
  `D/view/lists.rs`, `D/view/workspace.rs`
- Tests: `D/view/find_tests.rs` (E-4), `D/model/find_tests.rs` (E-5),
  `D/view/keyboard_tests.rs` `find`, `find_by_short_code_and_ulid` (E-6
  part), `D/view/dock_tests/state.rs` (E-10 part)

**Interfaces:**
- Consumes: `TicketQuery.text`, `set_query` (Task 2); the filter bar
  (Task 3).
- Produces: `pub(crate) const FILTER_PLACEHOLDER: &str = "Filter by title or code";`
  `Workspace::submit_filter(&mut self, window, cx)`;
  `FindQuery::parse` reduced to `pub(crate) fn parse_id(text: &str) -> Option<ItemId>`.

- [ ] **Step 1: Write the failing test**
  `desktop::view::find::tests::enter_in_the_filter_opens_an_id_or_the_only_match`:
  under Open, type a closed ticket's ULID in lowercase with spaces, Enter
  → its tab opens (P8, Review Focus 2); type a document's ULID, Enter →
  the document's tab; type a code that one open ticket has, Enter → it
  opens; type a duplicate code, Enter → no tab opens, focus is in the
  list with the first match selected (P9). Then select another
  repository: the filter text is empty and no row of the first repository
  is drawn (P17, Review Focus 4). Then apply E-4, E-5, E-6 and E-10 as the
  design words them.
- [ ] **Step 2: Run it; it fails.**
- [ ] **Step 3: Implement** Part D: the existing `find_input` becomes the
  filter bar's input (placeholder, `search` prefix, `cleanable`,
  `clean_on_escape`); its Change sets `query.text`; PressEnter runs
  `submit_filter`. `secondary-p` focuses it. Remove the match cards,
  `FindResult`, the slug lookup and `find_tickets_by_slug`'s call.
- [ ] **Step 4: Run** the new test and every edited one by exact name.
- [ ] **Step 5: Verify lines; commit; ticket comment** (Tickets done).

## Task 5: The Documents Model (Part E)

**Files:**
- Modify: `D/model/documents.rs`, `D/model/documents_tests.rs`,
  `D/model/item.rs`, `D/view/item/repair.rs`

**Interfaces:**
- Consumes: `KeptList` (Task 1); `WorkingCopy`, `row_markers` (Task 2).
- Produces:
  - `pub(crate) fn unmanaged(item: &ItemDto) -> bool` moved unchanged to
    `D/model/item.rs` with the helpers it needs; `repair.rs` calls it.
  - `pub(crate) struct Expansion` (collapsed folder paths) with
    `fn is_expanded(&self, path: &str) -> bool`,
    `fn set(&mut self, path: &str, expanded: bool)`,
    `fn collapse_all(&mut self, nodes: &[TreeNode])`.
  - `Documents`: `tree: KeptList<Vec<TreeNode>>`, an `Expansion` per
    repository for the session; `fn expansion(&self) -> Option<&Expansion>`
    (the selected repository's), `fn set_expanded(&mut self, path: &str, expanded: bool)`,
    `fn collapse_all(&mut self)`.
  - `pub(crate) enum DocIcon { Folder, FolderOpen, Document, Nonconforming }`,
    `pub(crate) enum Trailing { None, Count(usize), Empty }`,
    `pub(crate) struct DocRow { pub label: String, pub icon: DocIcon, pub trailing: Trailing, pub tooltip: String, pub adoptable: Option<bool> }`
    (`adoptable` is `Some` for a nonconforming document).
  - `pub(crate) fn doc_row(node: &TreeNode, expanded: bool) -> DocRow`

- [ ] **Step 0: Merge `origin/main`** (Global Constraints, Main).
- [ ] **Step 1: Write the failing tests.**
  - `desktop::model::documents::tests::expansion_survives_rebuilds_and_collapse_all`:
    a tree `guides/deep`, `plans`, `empty`; nothing collapsed → all
    expanded; collapse `guides` → rebuild with a new folder `notes` →
    `guides` collapsed, `notes` expanded; rebuild without `plans`, then
    with it → `plans` keeps its state (Review Focus 5); `collapse_all` →
    every folder collapsed; another repository's expansion is separate.
  - `desktop::model::documents::tests::rows_read_as_the_design_writes_them`,
    one table: expanded folder (FolderOpen, no trailing, tooltip its
    path); collapsed folder with three documents below at two depths
    (Folder, Count(3)); empty folder (Trailing::Empty, tooltip "Empty
    folder. Shared once it contains a document."); document (title, its
    path); nonconforming `missing_front_matter` (file name, `adoptable`
    true, tooltip "Not a managed document yet. Open to adopt it." then
    the path); nonconforming `missing_field` (`adoptable` false, "Needs
    fixing: {guidance} Open it to fix." then the path).
- [ ] **Step 2: Run them; they fail.**
- [ ] **Step 3: Implement** Part E; the existing tree view keeps working
  (it builds `TreeItem::expanded` from `Expansion`).
- [ ] **Step 4: Run** the two tests and the existing
  `desktop::model::documents::tests::*` by exact name; they pass.
- [ ] **Step 5: Verify lines; commit.**

## Task 6: The Documents Panel (Part F)

**Files:**
- Modify: `D/view/documents.rs`, `D/view/lists.rs` (`documents_body`),
  `D/view/workspace/lists.rs` (`open_selected_document` toggles a folder)
- Tests: `D/view/documents_tests.rs` (E-8, new), `D/view/new_folder_tests.rs`,
  `D/view/move_document_tests.rs` (E-8), `D/view/keyboard_tests/actions.rs`,
  `keyboard_tests/editing.rs` (E-9), `D/keys_tests.rs` (E-12 part)

**Interfaces:**
- Consumes: `ListRow`, `list_body` (Task 1); `SelectFirst`, `SelectLast`
  (Task 3); `doc_row`, `Expansion` (Task 5).
- Produces: element ids `doc-node`, `doc-count`, `doc-empty`,
  `doc-nonconforming`, `doc-editing`, `documents-alert`, `documents-empty`,
  `documents-skeleton`; `DocumentsPane` subscribes to `TreeEvent` and calls
  `Documents::set_expanded`.

- [ ] **Step 1: Write the failing test**
  `desktop::view::documents::tests::folders_keep_their_state_and_toggle_by_key`:
  focus the tree; Down to folder `guides`, Left → collapsed (`doc-count`
  shows its count), Right → expanded, Enter → collapsed (R-24); a click
  on `guides` → expanded; collapse it; finish a refresh job → still
  collapsed (P19); hide and show Documents from the footer → still
  collapsed, selection kept (P26); `home` and `end` select the first and
  last row (P18); Enter on a document opens its tab. Reads and requests
  unchanged by every step but the refresh (P25). Then E-8, E-9 and E-12's
  `Documents` rows as the design words them.
- [ ] **Step 2: Run it; it fails.**
- [ ] **Step 3: Implement** Part F: the base `Tree` with `ListRow` rows
  and `.vertical_scrollbar` (U-2: if it is not reachable, stop and record
  it), indent `8 + 14 × depth`, chevron or spacer, icon, label, trailing
  cell and markers (`row_markers` with no conflict or closed); each row's
  `aria_label` is the label and `aria_description` the tooltip; the body
  from `list_body`; Enter on a folder toggles it. Remove the second-line
  cells and `EMPTY_FOLDER`'s "Empty folder".
- [ ] **Step 4: Run** the new test and every edited one by exact name.
- [ ] **Step 5: Verify lines; commit.**

## Task 7: Panel Toolbars (Part G)

**Files:**
- Create: `D/view/dock/header/toolbar.rs`, `D/view/dock_tests/toolbars.rs`
- Modify: `D/view/dock/header/tabs.rs` (`single`, `tabs`),
  `D/view/dock/panels.rs` (`RegionPanel`), `D/view/lists.rs`,
  `D/view/documents.rs` (body buttons go), `D/view/dock_tests/mod.rs`
- Tests: `D/view/keyboard_tests/editing.rs`, `D/view/documents_tests.rs`
  `new_folder_opens_its_dialog` (E-7);
  `folders_keep_their_state_and_toggle_by_key` gains the Collapse all step

**Interfaces:**
- Consumes: `Documents::collapse_all` (Task 5).
- Produces: `RegionPanel::toolbar(&self, window, cx) -> Option<AnyElement>`,
  built by `Workspace::panel_toolbar(kind: PanelKind, window, cx)`; the
  header draws it at the right of a single panel's title row, or of a
  tabbed group's strip for the active tab (R-13). Buttons
  `new-ticket-button`, `new-document-button`, `new-folder-button`,
  `collapse-all-button`, each wrapped in `unless_dialog`.

- [ ] **Step 1: Write the failing test**
  `desktop::view::dock::tests::toolbars::panel_toolbars_work_by_tab_and_enter`:
  in the default layout, Tab to `new-ticket-button` and Enter → the New
  ticket… dialog (cancel); Tab to `new-document-button`, Enter → New
  document…; `new-folder-button` → New folder…; `collapse-all-button` →
  every folder collapsed and the hidden selection moved to its top-level
  folder (P20). Move Documents into the Tickets group as a tab (the
  dock's move, as C1's tests do) → with Documents active its toolbar is
  drawn and Collapse all works; with Tickets active, New ticket's. With a
  dialog open, activating a toolbar button does nothing (P24). Each
  button's accessible name is the design's. Extend
  `folders_keep_their_state_and_toggle_by_key` with Collapse all by
  click. Then E-7.
- [ ] **Step 2: Run it; it fails.**
- [ ] **Step 3: Implement** Part G (U-3: if the header cannot draw
  Workspace-built buttons, stop and record it).
- [ ] **Step 4: Run** the new and edited tests by exact name.
- [ ] **Step 5: Verify lines; commit.**

## Task 8: Checklist, Gates, Review-Ready

- [ ] **Step 1: On-screen checklist.** Write
  `docs/plans/2026-11-11-wave-04-cycle-02-on-screen-checklist.md` in
  Cycle 01's format, for the owner on a release build under Wayland
  against a scratch data directory: both panels against frames 1a and 1b
  in the dark theme; every list state reached (a never-refreshed
  repository, a busy one); the menu, segments, column headers and toolbar
  by keyboard with focus visible; tooltips; a narrow left dock (K-2);
  Orca reading a row, a segment and the readiness button. Claude records
  only what the owner reports.
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
  G → 7; H → Tasks 3 (Home and End, `TicketList`), 4 (`secondary-p`), 6
  (Enter on a folder) and 7 (toolbar Tab order). P1–P3, P10, P13 → Task 2;
  P4–P7 → Task 2 table and Task 3 view; P8, P9, P17 → Task 4; P11, P12,
  P14 → Tasks 2–3; P15, P16 → Tasks 1, 3; P18 → Tasks 3, 6; P19–P23,
  P26 → Tasks 5, 6; P20, P24 → Task 7; P25 → Tasks 3, 6. Decisions 1–17
  each land in the task of their Part. Amendments go in the approval
  commit, before Task 0.
- **Types.** `KeptList`, `ListBody`, `TicketQuery`, `TicketSort`,
  `SortColumn`, `TicketView`, `Markers`, `WorkingCopy`, `ShownTicket`,
  `Expansion`, `DocRow` are defined once and used by the names above.
- **Review Focus.** Each line names its test and task.
- **Proportion.** Signatures, test names and assertions only; bodies are
  the implementer's.
