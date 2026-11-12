---
title: "Wave 04 Cycle 02: Tickets And Documents Panels"
date: 2026-11-11
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M75C77FMKEPG4AX1HMB1CVKQ"
---

# Wave 04 Cycle 02: Tickets And Documents Panels

Ticket `01M757DHFAVPEVY4TWQTVKQAMJ` (`vh-r2n89`).
[Design](../plans/2026-11-11-wave-04-cycle-02-tickets-and-documents-design.md) and
[implementation plan](../plans/2026-11-11-wave-04-cycle-02-tickets-and-documents-implementation.md).

Status: **approved 2026-11-11** (revision 3 with round 3's fixes). The
owner wrote: "Accept Decision 10. Documents are approved. Implementation
is approved. Subagent driven development but prepare a brief to hand to a
fresh session" (decisions 10 and 21). The owner asked to go straight to the ticket, design and plan
from `docs/design/` (2026-11-11), so decisions 1–20 below were the
planner's proposals, each with its reason. The owner confirmed all twenty
on 2026-11-11 ("I confirm the decisions"). Those marked **(owner's eye)**
change a behaviour people have today or depart from an authority.

- Drafted on the ticket branch at `a0a734a4`, which holds origin/main
  `48653d01`. Cycle 01 and Wave 03 D6 are both merged there.
- Revision 1 (`563763e2`) was reviewed by two independent reviewers
  (source and feasibility; contract and consistency): 8 Blocking, 37
  Important and many Minor findings, each verified against the source and
  applied in revision 2 (Review Record).
- Revision 2 (`85210ab0`) had a scoped re-review and an adversarial
  review (round 2): 3 Blocking, 13 Important and many Minor findings, each
  verified against the source and applied in revision 3.
- Revision 3 (`ba5da196`) had a scoped re-review (round 3): round 2's
  findings resolved, 1 Important (task order in the plan) and 12 Minor,
  applied in place.

The approval commit applied the design's Document Amendments 1–6 (the
Wave's Cycle 02 section and Deferred Work, Cycle 01 decision 4, handoff
`03` and `08`, and the ticket relationships RFC). Implementation is
authorized (decision 21).

## Purpose And Authority

Rebuild the two lists people use most, Tickets and Documents, to the
design: one row anatomy, plain-language list states, a Tickets panel with
Open, Closed and All counts, a title-or-code filter, sortable columns,
status pills and a footnote, and a Documents tree with chevrons, icons,
titles, counts and folders that stay as the person left them. After this
Cycle a person can narrow and sort the ticket list without waiting on a
read, see at a glance what they have unsaved or must fix, find an
unreadable file instead of losing it, and browse the document tree without
it springing open on every refresh.

Authorities, in order:

1. `AGENTS.md` on origin/main.
2. [Wave 04](../Waves/wave-04-workspace-design.md): Cycle 02 (lines
   307–345), Owner Direction (65–101), Explicit Exclusions (169–183),
   Testing (498–507), Risks And Controls (525–537) and Owner Decisions
   1–10 (539–569), with decision 5 (Ready and Blocked in a filter menu)
   and 7 (28 px rows).
3. [Desktop information architecture and editor RFC](../RFC/desktop-information-architecture-and-editor.md):
   lists, states, nonconforming and inaccessible entries (57–61),
   relationships (62–65), the Open default and closure (67–71), item
   identity (73–78), navigation that never saves (80–84), status versus
   closure (94–95), empty folders (246–252), stale responses (341–342),
   accessibility (344–351). The
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
   folder…, Move…, repair and adopt), D5 (`open_ready_tickets`), and
   Cycle 01 (panels, header, regions, keys, C1 R-22 header counts).

## Entry Evidence And Dependency Boundary

| Gate | State at planning (`a0a734a4`) |
| --- | --- |
| Cycle ticket before the plan (`AGENTS.md`) | `01M757DHFAVPEVY4TWQTVKQAMJ`, open, created 2026-11-11 from main. |
| Latest main merged into the ticket branch | The branch was created from origin/main `48653d01`. |
| Cycle 01 merged | Yes: main `eb0798a2`, ticket `01M6V8RK8B0SBCH0YZXFVSR9E3` closed 2026-11-10. |
| Wave 03 D6 merged | Yes: main `48653d01`. It does not touch these lists. |
| GPUI Kit 0.6.6 names verified | Read from the locked source during planning and checked in review: design facts 1–12. |
| Required checks at baseline | Not run during planning. Task 0 runs them. |
| Testers | Linux (Wayland): the product owner. macOS and Windows: native CI after merge. |

**Dependency boundary.** This Cycle changes `src/desktop/` and its tests
only, plus the documents in the design's Document Amendments. It changes
nothing in `src/lib.rs`, `src/repository.rs`, `src/repository/`,
`src/results.rs`, `src/drafts/`, `src/runtime.rs` or the CLI, and adds no
Cargo dependency. Every value it shows comes from library reads that
exist today: `list_tickets`, `list_documents`, `list_document_folders`,
`show_item` and `index_status` (design facts 16, 17, 23, 30).

**In flight.** Wave 04 Cycle 03 (Repositories, Activity and Needs
attention) is not started. It shares the panel header and its region
focus (Part G) and may reuse `ListRow` and the list states (Part A).
Whichever merges second merges main and adapts, per `AGENTS.md`.

## Scope

1. **Common row anatomy** (design Part A): one 28 px row element for both
   lists, with hover, the selected treatment (the `selection` background,
   the amber 2 px left line and weight 600) and visible keyboard focus.
2. **List states** (Part A): first load with skeleton rows; a reload over
   kept rows marked "Updating…"; empty with its primary action; never
   refreshed, stale, incomplete, busy, failed and index unavailable as a
   row-level alert with one action; normal. Each in plain words that name
   the repository (decision 11).
3. **Tickets model** (Part B): one read per load, filtered and sorted in
   the desktop; Open, Closed and All counts; the hint for matches in the
   other lifecycle; the footnote; ages; unsaved, conflict and closed
   markers; the row tooltip with the ID and the copy it lists from.
4. **Tickets panel** (Part C): the segmented control, the readiness menu
   (Wave decision 5), the filter input, the sortable column header, rows
   with type icon, short code, title, markers, status pill and age, the
   footnote, and Why blocked under Blocked.
5. **The filter input replaces Find** (Part D; decision 2), keeping
   `secondary-p` and the open-by-ID path with its feedback.
6. **Documents model** (Part E): folder expansion kept for the session,
   Collapse all, descendant counts, labels and tooltips per kind.
7. **Documents panel** (Part F): the tree with indent, chevrons, folder and
   file icons, titles, counts, the unsaved marker, nonconforming rows and
   empty folders; Enter toggles a folder.
8. **Panel toolbars** (Part G; decision 18): New ticket in Tickets; New
   document, New folder and Collapse all in Documents, in the panel
   header, counted as the panel's region for focus.
9. **Keyboard** (Part H): Home and End in both lists; the Tab order of the
   new controls.
10. **Unreadable files** (Part I; decision 10): rows in their list, from
    the index status read.

## Explicit Exclusions And Downstream Obligations

- Project and Team scopes, multi-repository grouping, column resizing or
  choosing columns (Wave Cycle 02, out of scope).
- A Tickets More menu: nothing designed goes in it yet (decision 12).
- Saving the sort, the filters or folder expansion with the layout (added
  to the Wave's Deferred Work, design amendment 2).
- Index problems other than unreadable item files (a branch, a context, a
  directory): Activity's, Cycle 03.
- New document… starting in the selected folder (design K-9).
- Any library, repository-format or CLI change (Wave Explicit Exclusions).
  A gap found in the headless API goes to a separate library ticket.
- Row density settings and a theme switch (Wave decisions 7, 8).
- **For Cycle 03:** reuse `ListRow`, the list states and the toolbar's
  region focus for Repositories and Activity.

## Required Contract

1. **Same rows as the library.** Each list shows exactly the entries the
   library's reads give, filtered and sorted in the desktop, never a row
   made up. Nonconforming and unreadable entries stay visible under every
   filter (RFC 67-71).
2. **Closure is lifecycle, status is text.** Open, Closed and All follow
   lifecycle closure; the status pill shows the status as written (RFC
   94-95).
3. **Every match is reachable.** A short code typed into the filter lists
   or names every ticket that carries it, each with its ID and context in
   its tooltip (RFC relationships 146-148).
4. **Navigation sends nothing.** Filtering, sorting, typing in the filter,
   expanding, collapsing and moving the selection never open, close or
   reload a tab, and never save, synchronize or send anything (C1
   contract 7; RFC 80-84). Only Enter or a click opens an item.
5. **Keyboard.** Every new control is reachable by Tab, works by Enter or
   Space, has an accessible name and shows focus (RFC 344-351; MH-NFR-005).
6. **Colour is never the only signal.** Closed, unsaved, conflict,
   needs-fixing, can't-be-read, empty and every list state carry text or
   an icon with an accessible name (handoff README:40).
7. **No secret in a row, tooltip or description.** Rows show only
   repository content and Git names already shown elsewhere.
8. **R-60 holds.** No toolbar control or new key acts behind a sheet or
   dialog.

## Acceptance And Exit Evidence

Planned proof. The Status column is filled at the final task from the
branch's own tests (by exact name) and the execution ledger. "On screen"
rows await the owner's checks; Claude records only what the owner reports.

| # | Wave exit evidence / contract | Proof | Status |
| --- | --- | --- | --- |
| A1 | PRD `MH-CONTENT-001/003/004` list behaviours still pass | The edited existing tests, which still compare the rows with the library's (E-3, E-8), and the fast tier | Met: the edited tests (E-3, E-8) pass; fast tier green (Task 9) |
| A2 | Nonconforming and inaccessible items stay visible with their reason | `derive_filters_counts_and_sorts` (P6, P27); `rows_read_as_the_design_writes_them` (P23); `unreadable_files_go_to_their_list` and `an_unreadable_file_is_listed_with_its_reason` (P28) | Met by tests (Tasks 2, 7); the doubled `circle-x` is an owner question (checklist T-8.6) |
| A3 | Open is the default | `derive_filters_counts_and_sorts`; `the_default_query_lists_open_tickets` (E-1) | Met by tests (Tasks 2, 3) |
| A4 | Sort, filter and the four states are covered by model tests | `derive_filters_counts_and_sorts`; `each_state_has_its_own_body`; `kept_rows_follow_reloads` | Met by tests (Task 1, Task 2) |
| A5 | Duplicate short codes list every match | `derive_filters_counts_and_sorts` (P7); the rewritten `a_shared_short_code_lists_both_and_choosing_one_opens_it_by_id` (E-4); `tooltips_carry_the_id_context_and_reasons` | Met by tests (Tasks 2, 4) |
| A6 | Keyboard-only navigation and opening work in both panels | `the_filter_bar_works_without_a_read`; `folders_keep_their_state_and_toggle_by_key`; `panel_toolbars_work_by_tab_and_enter`; the keyboard journeys (E-6, E-7) | Met by tests (Tasks 3, 6, 8) |
| A7 | Navigation sends nothing (contract 4) | The journeys snapshot reads, requests and tabs after each step (P25) | Met by tests (the journeys' P25 snapshots); on-screen quiet Activity in checklist T-11 |
| A8 | Ready, Blocked and Why blocked keep working (Wave decision 5) | `the_filter_bar_works_without_a_read` (P11); the edited Why blocked tests (E-3) | Met by tests (Tasks 2, 3) |
| A9 | A ticket or document opens by ID, closed or not, with feedback | The rewritten `desktop::view::find::tests` (E-4; P8) | Met by tests (Task 4) |
| A10 | Expansion survives a reload and a hidden panel | `expansion_survives_rebuilds_and_collapse_all`; `folders_keep_their_state_and_toggle_by_key` (P19, P26) | Met by tests (Tasks 5, 6) |
| A11 | Toolbars by keyboard, counted as their region, refused behind a dialog (contracts 5, 8) | `panel_toolbars_work_by_tab_and_enter` (P24) | Met by tests (Task 8); U-3 held; owner questions on Collapse all in checklist T-6.11, T-6.12 |
| A12 | Every existing keyboard journey still passes | The edited journeys and the fast tier at review-ready | Met: every edited journey passes and the fast tier is green (30 targets, 0 failures) |
| A13 | The list states act, and focus finds an empty or failed list | `list_states_act_and_a_reload_keeps_the_rows` (P15, P16, P31) | Met by tests (Tasks 1, 3) |
| A14 | The rows match the design in the dark theme | On screen: the owner, against frames 1a and 1b | Awaiting the owner: on-screen checklist T-2, T-5, T-12 |
| A15 | Unsaved work shows without a reload | `markers_appear_without_a_read` (P14) | Met by tests (Task 3, Task 6) |

## Product-Owner Decisions (2026-11-11)

Wave 04 decisions 1–10 (2026-11-07) govern and are not repeated. The
Cycle 01 decisions stand except where decision 2 says otherwise.

**Proposed by the planner and confirmed by the owner on 2026-11-11:**

1. **One Cycle.** The Wave gives Cycle 02 no split line, and the two
   panels share the row and the states.
2. **Find becomes the Tickets filter input (owner's eye).** The design
   draws no Find in the panel; its filter input "matches title and short
   code. Duplicate short codes show all matches." Typing filters as you
   go. When the text matches tickets the filters hide, a line says so
   ("1 more ticket matches with other filters.") with Show all, which
   chooses All and Any readiness. Enter on a
   ULID still opens that ticket or document, closed or not, with "Opening…"
   and "Nothing in {repository} has that ID." as feedback; Enter with one
   match in any lifecycle opens it. `secondary-p` still focuses it. Find's
   match cards go; each row's tooltip carries the ID and the copy it
   lists from instead. This supersedes Cycle 01 decision 4 in part.
   *Alternatives:* keep Find as a second input above the filter.
3. **Short codes in lowercase, as stored.** The relationships RFC says
   they are displayed in lowercase (103–107); the handoff's frames show
   upper case. The RFC governs the domain.
4. **No short code shows `—` (owner's eye)**, as the design draws, and
   the row's tooltip and description carry the ticket's ID. The RFC's
   "shown by its ULID" (132–134) does not fit a 78 px column. Most
   tickets made before short codes have none, so `—` is common under
   All and Closed. *Alternative:* the ID's last characters in the column.
5. **Closed rows under All carry a marker.** The design dims a closed
   title only, which is colour alone. Under All a closed ticket also shows
   a `circle-check` marker named "Closed". Under Closed every row is
   closed, so no marker is needed.
6. **Counts.** The Open, Closed and All counts count the tickets that
   match the filter text, by lifecycle, so Open + Closed = All and a
   person typing a code sees matches in the other segment. The readiness
   choice does not change them. Nonconforming and unreadable entries are
   not in these counts; they are shown under every choice and counted in
   the footnote. The panel header counts the rows listed, as Cycle 01
   does, and reads "Updating…" during a reload.
7. **Footnote.** `{n} open tickets in {repository}` as the Wave says,
   with "closed", nothing (All), "ready" or "blocked" in place of "open"
   as chosen, `match "{text}"` when filtering, and `· {k} need fixing` and
   `· {k} can't be read` when there are such entries. The design's "· from
   {k} repositories" waits for multi-repository scopes.
8. **The pencil means unsaved work (owner's eye).** The design's pencil
   reads "You're editing this. Saved on this computer, not shared yet."
   In a dogfooding repository every open ticket has its own private
   working copy, so marking those would mark nearly every row, and no read
   says what is "not shared yet" per item. The pencil shows when this app
   holds unsaved work on the item, a draft of it or of a comment on it
   ("You have unsaved work on this."). The private working copy, with its
   branch and worktree, is named in the row's tooltip (PRD
   MH-CONTENT-004). The conflict marker shows when a pending conflict
   names the item ("This has a conflict to resolve. Open it to choose
   what to keep."). When the drafts store cannot be read, no pencil
   shows. *Alternative:* a pencil for every private working copy.
9. **Nonconforming wording.** A ticket entry shows its path, "needs
   fixing" and the library's guidance for its problem ("Needs fixing: Add
   the required front matter field. Open it to fix."), since the problem
   does not name the field (owner, 2026-10-27, D3 design :2560-2562). A
   document shows its file name; "Not a managed document yet. Open to
   adopt it." only when adopting applies, as the item's banner decides;
   otherwise "Needs fixing: {guidance} Open it to fix."
10. **Unreadable files become rows (owner's eye).** The Wave's exit
    evidence asks for inaccessible items to stay visible with their
    reason. An unreadable file never makes a list "incomplete"; only the
    index status read reports it, and that read exists. Each load reads
    it once, and each unreadable ticket or document file shows in its list
    as "can't be read" with the guidance and its path. This adds one read
    per load and one row kind, and no library change. *Alternative:*
    accept the gap and amend RFC IA 57-59. **Narrowed in revision 3 and
    accepted by the owner on 2026-11-11:** such a row does not open, because opening
    it by path answers "The repository cannot be read." for a permission
    failure, which blames the whole repository (design R-38). The
    confirmed text said it opens by path.
11. **List states in plain words.** The design's four states hold D1's
    eight, each with its own sentence naming the repository and one
    action (design Part A's table): Refresh for never refreshed, stale and
    incomplete; Retry for busy and failed, with a failure's message still
    visible; Rebuild the search index for an unavailable index; New
    ticket… or New document… for an empty list; Clear filters when the
    filters hide everything. A list with only nonconforming or unreadable
    rows shows them, never the empty state.
12. **No More button yet (owner's eye).** The design and the Wave give
    Tickets a More button, with nothing designed in it. The Wave leaves
    out what would be a stub, so it waits for its first action.
13. **Folders stay as the person leaves them.** Expansion is kept per
    repository for the session, through reloads and a hidden panel. A new
    folder starts expanded, as every folder does today, so a first look
    is unchanged (the prototype starts with most collapsed). Collapse all
    collapses every folder. Enter on a folder toggles it, as a click does.
14. **Empty folders say so.** An empty folder shows "empty" where a
    collapsed folder shows its count, and its tooltip and description say
    "Empty folder. Shared once it contains a document." The design puts
    the explanation in a tooltip, which a keyboard user never sees.
15. **A busy reload keeps the rows.** Both lists keep their last rows
    under a busy reload, as the Repositories list already does, and say
    the list was not updated. A failed read still replaces them with the
    failure (D1 ruling R-B5).
16. **Existing-test edits.** The named edits E-1 to E-14 are part of
    approving the design. E-1 to E-6, E-8 and E-13 remove assertions,
    each named in the design; E-14 loosens Cycle 01's keyboard-walk helper
    so a header toolbar button counts as its panel's region (decision
    18). Any other edit to an existing test that this
    Cycle's changes need is pre-authorized, recorded in the ledger as E-15
    onward with its reason, checked by the task's reviewer, and called
    out in the review-ready summary when it removes an assertion (as
    Cycle 01 decision 17).
17. **Test runs follow `AGENTS.md`.** Between tasks, only the tests a task
    wrote or changed run, by exact name. The fast tier runs once, at
    review-ready.
18. **Toolbars in the header count as their panel (owner's eye).** The
    design puts each panel's toolbar in its header, which Cycle 01 drew
    outside the panel's focus region. A focused toolbar button now counts
    as its panel's region: its ring shows, F6 moves on from it, and Ctrl N
    there makes the panel's kind. *Alternative:* a toolbar row at the top
    of the panel body, simpler for focus but unlike the design.
19. **A blocked ticket's reasons leave its row (owner's eye).** Today each
    row under Blocked carries a line of reasons. The design's rows are one
    28 px line, so the reasons move to Why blocked (for the selected row,
    as today), the row's tooltip and its accessible description.
20. **One status colour (owner's eye).** The list's status pill and the
    item header's status chip use one mapping: the header's, with the
    design's "ready" added. A status reads the same colour in both places;
    "open", "done" and "duplicate" follow the header, not `03`'s table.

**Answered by the owner at approval (2026-11-11):**

21. **Implementation is authorized, subagent-driven and serial, in a
    fresh session.** One implementer per task, then a fresh reviewer, one
    at a time, following `.agents/skills/implementing-a-cycle/SKILL.md`.
    The fresh session works from the brief
    `docs/plans/2026-11-11-wave-04-cycle-02-handoff.md`. Authorized:
    Tasks 0–9, the named test edits E-1 to E-14 and further edits under
    decision 16, and pushing this ticket branch after each task. Not
    authorized without the owner's go-ahead at the time: a push of any
    other branch, a pull request, the merge, closing the ticket, worktree
    cleanup, native CI, creating tickets, and `ticket save` or `comment
    add` on any other ticket.

## Known Limits

These are visible to users; the design's Known Limits give the detail.

- Tooltips show on hover only; their text is also each element's
  accessible name or description (K-1).
- In a narrow dock, short codes and pills are clipped so titles keep
  room (K-2).
- Ages are worked out when rows are drawn (K-3).
- An unreadable file shows after the refresh that met it (K-4).
- Folder expansion, the filters and the sort are kept for the session
  only (K-5).
- The readiness menu's choice is announced on its button, not its items;
  the sort order on its column button (K-6, K-7).
- Up and Down wrap at the ends of both lists (K-8).
- New document… starts in `docs` whatever folder is selected (K-9).
- Each reload reads closed tickets too (K-10).
- The list and the tree are reached by F6, `secondary-2`, a click or Enter
  in the filter, not by Tab, as today (K-11).
- Very long short codes are clipped in their column (K-12).
- An unreadable ticket folder hides its ticket without a row (K-13).
- An unreadable file's row does not open (K-14).

## Open Questions

None blocking. The design's U-3 is checked by Task 8, the first task that
relies on it; U-1, U-2, U-4 and U-5 were settled in review.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Owner, 2026-11-11 | "go straight to the ticket, design and plan according to the design as laid out in docs/design/" | Owner direction | Ticket created; decisions proposed for confirmation at approval | Session, 2026-11-11 |
| Planning research | Sources disagree on short-code case, no-code display, closure signal, counts, footnote, Find, list states, nonconforming wording, inaccessible entries, empty folders | Decisions (proposed) | Decisions 2–15 | Design Review Record |
| Planning research | Kit list, table, tree, segmented control, menu and skeleton facts | Rulings | Design R-2 to R-6 | Design facts 1–12 |
| Review round 1 (two reviewers, `563763e2`) | 8 Blocking, 37 Important, many Minor; among them header focus, unnamed compile sites, duplicate codes across lifecycles, unreadable files, nonconforming-only lists, the ULID path | Decisions (proposed), rulings, named edits | Decisions 2, 8, 10, 18, 19, 20 revised or added; design revision 2 | Design Review Record |
| Owner, 2026-11-11 | "I confirm the descisions" | Owner decision | Decisions 1–20 confirmed; review round 2 requested | Session, 2026-11-11 |
| Review round 2 (scoped re-review and adversarial, `85210ab0`) | 3 Blocking, 13 Important, many Minor; among them the keyboard walk's region check, no list element in the empty and failed states, outcomes without a test, Retry clearing the filter, the details sheet's suggestions, Show all under a readiness, an unreadable file's open | Rulings, named edits; decision 10 narrowed for confirmation | Design revision 3: E-14, R-33 to R-38, new test steps; decision 10's rows do not open | Design Review Record |
| Review round 3 (scoped re-review, `ba5da196`) | Round 2 resolved; 1 Important: the plan's Task 3 tests needed the filter input that Task 4 placed; 12 Minor | Plan task moves; settled | Revision 3 with round 3's fixes | Design Review Record |
| Owner, 2026-11-11 | "Accept Decision 10. Documents are approved. Implementation is approved. Subagent driven development but prepare a brief to hand to a fresh session" | Owner decision | Decision 10's narrowing accepted; documents approved; decision 21; Document Amendments 1–6 applied; the handoff brief written | Session, 2026-11-11 |
