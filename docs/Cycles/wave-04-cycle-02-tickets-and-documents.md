---
title: "Wave 04 Cycle 02: Tickets And Documents Panels"
date: 2026-11-11
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
---

# Wave 04 Cycle 02: Tickets And Documents Panels

Ticket `01M757DHFAVPEVY4TWQTVKQAMJ` (`vh-r2n89`).
[Design](../plans/2026-11-11-wave-04-cycle-02-tickets-and-documents-design.md) and
[implementation plan](../plans/2026-11-11-wave-04-cycle-02-tickets-and-documents-implementation.md).

Status: **draft, revision 1, for the owner's review.** The owner asked to
go straight to the ticket, design and plan from `docs/design/` (2026-11-11),
so decisions 1–17 below are the planner's proposals, each with its reason,
for the owner to confirm, change or reject at approval.

- Drafted on the ticket branch at `a0a734a4`, which holds origin/main
  `48653d01`. Cycle 01 and Wave 03 D6 are both merged there.

Approval of the documents does not authorize implementation, which needs
its own go-ahead.

## Purpose And Authority

Rebuild the two lists people use most, Tickets and Documents, to the
design: one row anatomy, four plain-language list states, a Tickets panel
with Open, Closed and All counts, a title-or-code filter, sortable columns,
status pills and a footnote, and a Documents tree with chevrons, icons,
titles, counts and folders that stay as the person left them. After this
Cycle a person can narrow and sort the ticket list without waiting on a
read, see at a glance what they are editing or must fix, and browse the
document tree without it springing open on every refresh.

Authorities, in order:

1. `AGENTS.md` on origin/main.
2. [Wave 04](../Waves/wave-04-workspace-design.md): Cycle 02 (lines
   307–345), Owner Direction (65–101), Explicit Exclusions (169–183),
   Testing (498–507), Risks And Controls (525–537) and Owner Decisions
   1–10 (539–569), with decision 5 (Ready and Blocked in a filter menu)
   and 7 (28 px rows).
3. [Desktop information architecture and editor RFC](../RFC/desktop-information-architecture-and-editor.md):
   lists, states and nonconforming entries (57–61), relationships (62–65),
   the Open default and closure (67–71), item identity (73–78), navigation
   that never saves (80–84), status versus closure (94–95), empty folders
   (246–252), accessibility (344–351). The
   [ticket relationships and short codes RFC](../RFC/ticket-relationships-and-short-codes.md):
   display case (103–107), no short code (132–134), duplicates (142–148),
   ready and blocked (215–234), desktop (318–331).
4. PRD [`docs/PRD/mvp.md`](../PRD/mvp.md): `MH-CONTENT-001`, `-003`,
   `-004`, `-005`, `-006`, `MH-INDEX-003`, `MH-UX-001`, `MH-NFR-004`,
   `MH-NFR-005`.
5. The design handoff: [`03-panels.md`](../design/03-panels.md) (rows,
   states, Tickets, Documents), [`06-language-and-states.md`](../design/06-language-and-states.md)
   (copy), [`07-design-tokens.md`](../design/07-design-tokens.md),
   [`08-gpui-kit-mapping.md`](../design/08-gpui-kit-mapping.md), the
   frames `screenshots/1a`, `1b` and `1c`, and the prototype source
   `mockups/source/Workspace.dc.html`, which governs exact values where the
   specs are silent (handoff README:15).
6. Predecessors: Wave 03 D1 (list states, filters, Find, nonconforming
   entries; its ruling R-B5), D3 (New ticket…, New document…, New
   folder…, Move…, repair and adopt), and Cycle 01 (panels, header,
   regions, keys, C1 R-22 header counts).

## Entry Evidence And Dependency Boundary

| Gate | State at planning (`a0a734a4`) |
| --- | --- |
| Cycle ticket before the plan (`AGENTS.md`) | `01M757DHFAVPEVY4TWQTVKQAMJ`, open, created 2026-11-11 from main. |
| Latest main merged into the ticket branch | The branch was created from origin/main `48653d01`. |
| Cycle 01 merged | Yes: main `eb0798a2`, ticket `01M6V8RK8B0SBCH0YZXFVSR9E3` closed 2026-11-10. |
| Wave 03 D6 merged | Yes: main `48653d01`. It does not touch these lists. |
| GPUI Kit 0.6.6 names verified | Read from the locked source during planning: design facts 1–12. |
| Required checks at baseline | Not run during planning. Task 0 runs them. |
| Testers | Linux (Wayland): the product owner. macOS and Windows: native CI after merge. |

**Dependency boundary.** This Cycle changes `src/desktop/` and its tests
only, plus the documents in the design's Document Amendments. It changes
nothing in `src/lib.rs`, `src/repository.rs`, `src/repository/`,
`src/results.rs`, `src/drafts/`, `src/runtime.rs` or the CLI, and adds no
Cargo dependency. Every value it shows comes from reads the desktop
already makes (design facts 16, 17, 22, 25, 26).

**In flight.** Wave 04 Cycle 03 (Repositories, Activity and Needs
attention) is not started. It shares the panel header (Part G) and may
reuse `ListRow` and the list states (Part A). Whichever merges second
merges main and adapts, per `AGENTS.md`.

## Scope

1. **Common row anatomy** (design Part A): one 28 px row element for both
   lists, with hover, the selected treatment (the `selection` background,
   the amber 2 px left line and weight 600) and visible keyboard focus.
2. **List states** (Part A): first load with skeleton rows; empty with its
   primary action; never refreshed, stale, incomplete, busy, failed and
   index unavailable as a row-level alert with one action; normal. Each in
   plain words that name the repository (decision 11).
3. **Tickets model** (Part B): one read per load, filtered and sorted in
   the desktop; Open, Closed and All counts; the footnote; ages; editing,
   conflict and closed markers.
4. **Tickets panel** (Part C): the segmented control, the readiness menu
   (Wave decision 5), the filter input, the sortable column header, rows
   with type icon, short code, title, markers, status pill and age, the
   footnote, and Why blocked under Blocked.
5. **The filter input replaces Find** (Part D; decision 2), keeping
   `secondary-p` and the open-by-ID path.
6. **Documents model** (Part E): folder expansion kept for the session,
   Collapse all, descendant counts, labels and tooltips per kind.
7. **Documents panel** (Part F): the tree with indent, chevrons, folder and
   file icons, titles, counts, the editing marker, nonconforming rows and
   empty folders; Enter toggles a folder.
8. **Panel toolbars** (Part G): New ticket in Tickets; New document, New
   folder and Collapse all in Documents, in the panel header.
9. **Keyboard** (Part H): Home and End in both lists; the Tab order of the
   new controls.

## Explicit Exclusions And Downstream Obligations

- Project and Team scopes, multi-repository grouping, column resizing or
  choosing columns (Wave Cycle 02, out of scope).
- A Tickets More menu: nothing designed goes in it yet (decision 12).
- Saving the sort, the filters or folder expansion with the layout (Wave
  Deferred Work; design K-5).
- Per-item inaccessible rows: they need a library read (decision 10).
- New document… starting in the selected folder (design K-9).
- Any library, repository-format or CLI change (Wave Explicit Exclusions).
  A gap found in the headless API goes to a separate library ticket.
- Row density settings and a theme switch (Wave decisions 7, 8).
- **For Cycle 03:** reuse `ListRow` and the list states for the
  Repositories and Activity rows, and the toolbar slot for their
  toolbars.

## Required Contract

1. **Same rows as the library.** Each list shows exactly the entries the
   library's reads give, filtered and sorted in the desktop, never a row
   made up. Nonconforming entries stay visible under every filter
   (RFC 67-71).
2. **Closure is lifecycle, status is text.** Open, Closed and All follow
   lifecycle closure; the status pill shows the status as written (RFC
   94-95).
3. **Navigation sends nothing.** Filtering, sorting, typing in the filter,
   expanding, collapsing and moving the selection never open, close or
   reload a tab, and never save, synchronize or send anything (C1
   contract 7; RFC 80-84). Only Enter or a click opens an item.
4. **Keyboard.** Every new control is reachable by Tab, works by Enter or
   Space, has an accessible name and shows focus (RFC 344-351; MH-NFR-005).
5. **Colour is never the only signal.** Closed, editing, conflict,
   needs-fixing, empty and every list state carry text or an icon with an
   accessible name (handoff README:40).
6. **No secret in a row, tooltip or description.** Rows show only
   repository content and Git names already shown elsewhere.
7. **R-60 holds.** No toolbar control or new key acts behind a sheet or
   dialog.

## Acceptance And Exit Evidence

Planned proof. The Status column is filled at the final task from the
branch's own tests (by exact name) and the execution ledger. "On screen"
rows await the owner's checks; Claude records only what the owner reports.

| # | Wave exit evidence / contract | Proof | Status |
| --- | --- | --- | --- |
| A1 | PRD `MH-CONTENT-001/003/004` list behaviours still pass | The edited existing tests (E-3, E-8) and the fast tier | Planned |
| A2 | Nonconforming and inaccessible items stay visible with their reason | `derive_filters_counts_and_sorts` (P6); `rows_read_as_the_design_writes_them` (P23); `each_state_has_its_own_body` (the incomplete alert, decision 10) | Planned |
| A3 | Open is the default | `derive_filters_counts_and_sorts`; `the_default_query_lists_open_tickets` (E-1) | Planned |
| A4 | Sort, filter and the four states are covered by model tests | `derive_filters_counts_and_sorts`; `each_state_has_its_own_body` | Planned |
| A5 | Duplicate short codes list every match | `derive_filters_counts_and_sorts` (P7); the rewritten `a_shared_short_code_lists_both_and_choosing_one_opens_it_by_id` (E-4) | Planned |
| A6 | Keyboard-only navigation and opening work in both panels | `the_filter_bar_works_without_a_read`; `folders_keep_their_state_and_toggle_by_key`; `panel_toolbars_work_by_tab_and_enter`; the keyboard journeys (E-6, E-7) | Planned |
| A7 | Navigation sends nothing (contract 3) | The two journeys snapshot reads, requests and tabs after each step (P25) | Planned |
| A8 | Ready, Blocked and Why blocked keep working (Wave decision 5) | `the_filter_bar_works_without_a_read` (P11); the edited Why blocked tests (E-3) | Planned |
| A9 | A ticket or document opens by ID, closed or not | `enter_in_the_filter_opens_an_id_or_the_only_match` (P8) | Planned |
| A10 | Expansion survives a reload and a hidden panel | `expansion_survives_rebuilds_and_collapse_all`; `folders_keep_their_state_and_toggle_by_key` (P19, P26) | Planned |
| A11 | Toolbars by keyboard, refused behind a dialog (contracts 4, 7) | `panel_toolbars_work_by_tab_and_enter` (P24) | Planned |
| A12 | Every existing keyboard journey still passes | The edited journeys and the fast tier at review-ready | Planned |
| A13 | The rows match the design in the dark theme | On screen: the owner, against frames 1a and 1b | Planned |

## Product-Owner Decisions (2026-11-11)

Wave 04 decisions 1–10 (2026-11-07) govern and are not repeated. The
Cycle 01 decisions stand except where decision 2 says otherwise.

**Proposed by the planner, for the owner to confirm at approval:**

1. **One Cycle.** The Wave gives Cycle 02 no split line, and the two
   panels share the row and the states.
2. **Find becomes the Tickets filter input.** The design draws no Find in
   the panel; its filter input "matches title and short code. Duplicate
   short codes show all matches." Typing filters as you go. Enter on a
   ULID still opens that ticket or document, closed or not, as Find did;
   Enter with one match opens it. `secondary-p` still focuses it. Find's
   match cards go. This supersedes Cycle 01 decision 4 in part.
3. **Short codes in lowercase, as stored.** The relationships RFC says
   they are displayed in lowercase (103–107); the handoff's frames show
   upper case. The RFC governs the domain.
4. **No short code shows `—`**, as the design draws, and the row's
   tooltip and description carry the ticket's ID. The RFC's "shown by its
   ULID" (132–134) does not fit a 78 px column; every row's tooltip gives
   the ID, so the canonical ID is always reachable (PRD MH-CONTENT-006).
5. **Closed rows under All carry a marker.** The design dims a closed
   title only, which is colour alone. Under All a closed ticket also shows
   a `circle-check` marker named "Closed". Under Closed every row is
   closed, so no marker is needed.
6. **Counts.** The Open, Closed and All counts count the tickets that
   match the filter text, by lifecycle, so Open + Closed = All and a
   person typing a code sees matches in the other segment. The readiness
   choice does not change them. Nonconforming entries are not in these
   counts; they are shown under every choice and counted in the footnote.
   The panel header keeps Cycle 01's count of rows listed.
7. **Footnote.** `{n} open tickets in {repository}` as the Wave says,
   with "closed", nothing (All), "ready" or "blocked" in place of "open"
   as chosen, `match "{text}"` when filtering, and `· {k} need fixing`
   when there are nonconforming entries. The design's "· from {k}
   repositories" waits for multi-repository scopes.
8. **Editing and conflict markers say only what is known.** Editing
   means the ticket has its own private working copy, or this app holds
   an unsaved draft of it; the tooltip says which, and names the working
   copy's branch and worktree (PRD MH-CONTENT-004). It does not claim "not
   shared yet", which no read reports per item. Conflict means a pending
   conflict names the item. When the drafts store cannot be read, no
   editing marker shows.
9. **Nonconforming wording.** A ticket entry shows its path, "needs
   fixing" and the library's guidance for its problem ("Needs fixing: Add
   the required front matter field. Open it to fix."), since the problem
   does not name the field (owner, 2026-10-27, D3 design :2560-2562). A
   document shows its file name; "Not a managed document yet. Open to
   adopt it." only when adopting applies, as the item's banner decides;
   otherwise "Needs fixing: {guidance} Open it to fix."
10. **Inaccessible files show as the incomplete alert.** The library lists
    no row for an unreadable file; it reports the list incomplete. The
    alert says some files could not be read and offers Refresh.
    Per-file rows need a library read and are left to a library ticket.
11. **List states in plain words.** The design's four states hold D1's
    eight, each with its own sentence naming the repository and one
    action (design Part A's table): Refresh for never refreshed, stale and
    incomplete; Retry for busy and failed; Rebuild index for an
    unavailable index; New ticket… or New document… for an empty list;
    Clear filters when the filters hide everything.
12. **No More button yet.** The design gives Tickets a More button with
    nothing in it. The Wave leaves out what would be a stub, so it waits
    for its first action.
13. **Folders stay as the person leaves them.** Expansion is kept per
    repository for the session, through reloads and a hidden panel. A new
    folder starts expanded, as every folder does today, so a first look
    is unchanged. Collapse all collapses every folder. Enter on a folder
    toggles it, as a click does.
14. **Empty folders say so.** An empty folder shows "empty" where a
    collapsed folder shows its count, and its tooltip and description say
    "Shared once it contains a document." The design puts the explanation
    in a tooltip, which a keyboard user never sees.
15. **A busy reload keeps the rows.** Both lists keep their last rows
    under a busy reload, as the Repositories list already does, and say
    the list was not updated. A failed read still replaces them with the
    failure (D1 ruling R-B5).
16. **Existing-test edits.** The named edits E-1 to E-13 are part of
    approving the design. E-1, E-2, E-3, E-4 and E-5 remove assertions,
    each named in the design. Any other edit to an existing test that this
    Cycle's changes need is pre-authorized, recorded in the ledger as E-14
    onward with its reason, checked by the task's reviewer, and called
    out in the review-ready summary when it removes an assertion (as
    Cycle 01 decision 17).
17. **Test runs follow `AGENTS.md`.** Between tasks, only the tests a task
    wrote or changed run, by exact name. The fast tier runs once, at
    review-ready.

## Known Limits

These are visible to users; the design's Known Limits give the detail.

- Tooltips show on hover only; their text is also each element's
  accessible name or description (K-1).
- Columns have fixed widths, so a narrow dock shortens titles (K-2).
- Ages are worked out when rows are drawn (K-3).
- Unreadable files show as the incomplete alert, not as rows (K-4).
- Folder expansion and the sort are kept for the session only (K-5).
- The readiness menu's choice is announced on its button, not its items;
  the sort order on its column button (K-6, K-7).
- Up and Down wrap at the ends of both lists (K-8).
- New document… starts in `docs` whatever folder is selected (K-9).
- Each reload reads closed tickets too (K-10).

## Open Questions

None blocking. The design's U-1 to U-4 are checked by the first task that
relies on each.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Owner, 2026-11-11 | "go straight to the ticket, design and plan according to the design as laid out in docs/design/" | Owner direction | Ticket created; decisions 1–17 proposed for confirmation at approval | Session, 2026-11-11 |
| Planning research | Sources disagree on short-code case, no-code display, closure signal, counts, footnote, Find, list states, nonconforming wording, inaccessible entries, empty folders | Decisions (proposed) | Decisions 2–15 | Design Review Record |
| Planning research | Kit list, table, tree, segmented control, menu and skeleton facts | Rulings | Design R-2 to R-6 | Design facts 1–12 |
