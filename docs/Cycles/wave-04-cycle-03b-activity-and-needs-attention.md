---
title: "Wave 04 Cycle 03b: Activity And Needs Attention"
date: 2026-11-12
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M79CDKRTC646SBMMNDWY9FHM"
---

# Wave 04 Cycle 03b: Activity And Needs Attention

Ticket `01M784Y94Q3AXT6XJX38F5VK1V` (`vh-dt0aq`).
[Design](../plans/2026-11-12-wave-04-cycle-03b-activity-and-needs-attention-design.md) and
[implementation plan](../plans/2026-11-12-wave-04-cycle-03b-activity-and-needs-attention-implementation.md).

**Status: draft, revision 1.** On 2026-11-12 the owner asked to plan this
Cycle while Cycle 03a was in its final review ("substantially complete").
The owner answered decisions 4–12 in that session and accepted the
planner's outline with decision 13 ("write the documents"). Decisions
1–3 were made during Cycle 03a's planning and are recorded on this
ticket. Decisions 14–20 are the planner's proposals, each with its
reason, for the owner to confirm. Those marked **(owner's eye)** change
something people see today or depart from an authority.

- Drafted on the ticket branch at `b37d2c2b`. That commit merges Cycle
  03a's published head `3086666f` into this ticket's base, origin/main
  `37be101d` (decision 4).
- Revision 1 has not yet been reviewed.

## Purpose And Authority

Make recovery visible in plain language, in the places the design puts it
(Wave, Cycle 03b).

After this Cycle, Activity is one timed log. Each line says:

- when the app learned of it;
- what happened, or what is wrong, in plain words;
- which repository it concerns;
- the one thing to do about it.

The Git detail is a hover, or a menu, away. Failures and recovery states
stay until they are resolved, and the footer's Needs attention opens a
popover listing exactly those. The popover and the count read the same
rows as Activity, so they never disagree, and the popover works with
Activity hidden.

Authorities, in order:

1. `AGENTS.md` on origin/main.
2. [Wave 04](../Waves/wave-04-workspace-design.md):
   - **Cycle 03b:** scope, exclusions and exit evidence, with D6's
     amendment on the K-19 rows and Cycle 03a's downstream obligation;
   - Owner Direction;
   - Explicit Exclusions;
   - Testing;
   - Risks And Controls;
   - Owner Decisions 1–10.
3. [Desktop information architecture and editor RFC](../RFC/desktop-information-architecture-and-editor.md):
   - the Wave 04 amendment (40–55): Activity and the popover over the
     same records, and failures never only transient;
   - accessibility (344–351).
   - The [runtime RFC](../RFC/application-runtime-and-polling.md) also
     applies: typed recovery, no raw remote text (44–49), and "still
     stopping" in the quit's Activity entry (188–190).
4. PRD [`docs/PRD/mvp.md`](../PRD/mvp.md):
   - `MH-UX-001`: progress, outcome and actionable recovery feedback;
   - `MH-NFR-003`, `MH-NFR-004` and `MH-NFR-005`.
5. **The design handoff:**
   - [`03-panels.md`](../design/03-panels.md) "Activity";
   - [`05-sheets-and-dialogs.md`](../design/05-sheets-and-dialogs.md)
     "Needs attention popover";
   - [`02-layout-and-docking.md`](../design/02-layout-and-docking.md)
     Footer item 5;
   - [`06-language-and-states.md`](../design/06-language-and-states.md);
   - [`07-design-tokens.md`](../design/07-design-tokens.md);
   - [`08-gpui-kit-mapping.md`](../design/08-gpui-kit-mapping.md);
   - frames `screenshots/1a` and `2f`.
6. **Predecessors:**
   - Wave 03 D5: the lifecycle and conflict entries, and its privacy rule
     (amended by decision 12);
   - D6: holds, checks, Quit's "still stopping";
   - Cycle 01: the Activity panel, the footer, the count's rules, K-19;
   - Cycle 03a: repository state, the footer order, the unshared list,
     the menu layer.

## Entry Evidence And Dependency Boundary

| Gate | State at planning (`b37d2c2b`) |
| --- | --- |
| Cycle ticket before the plan (`AGENTS.md`) | `01M784Y94Q3AXT6XJX38F5VK1V`, open, created 2026-11-12 from main and published |
| Latest main merged into the ticket branch | Created from origin/main `37be101d`; nothing newer on main at planning |
| Cycle 03a merged | **No.** In final review. Its published head `3086666f` is merged into this branch (decision 4); its later final-review commits are not. Implementation does not start before 03a closes: Task 0 merges main and re-grounds (design U-6) |
| Wave 03 D6 merged | Yes: main `48653d01` |
| GPUI Kit 0.6.6 names verified | `List`, `Popover`, `PopupMenu`, `Button`, `uniform_list`, read from the locked source during planning (design facts 31–34; U-2) |
| Required checks at baseline | Not run during planning; Task 0 runs them |
| Testers | Linux (Wayland): the product owner. macOS and Windows: native CI after merge |

**Dependency boundary.** This Cycle changes `src/desktop/` and its tests
only, plus the documents in the design's Document Amendments.

- It changes nothing in `src/lib.rs`, `src/repository.rs`,
  `src/repository/`, `src/results.rs`, `src/drafts/`, `src/runtime.rs`,
  `tests/` or the CLI.
- It adds no Cargo dependency.
- Every row comes from models the desktop keeps today, plus a
  session-only history of checks that brought something in.

**In flight.** These open tickets name rows of today's pending list or
recovery display, and may land later:

- `01M6GHWZ5CXXFC7YB3EVJ6Q4A7`;
- `01M6GHWZS03WCADS1A1JXV4PMP`;
- `01M73V5N1RK32TJPTW219S4KD6`;
- `01M73V4F416W347GPCNAYRTXB0`;
- `01M4H33R34Z7C7EEKTY1ZCT950`;
- `01M67BN4C1SY2HQBGNE9NBYMW8`;
- `01M4W5SCBV6WFJP11Y7PJF5VS8`;
- `01M6W5GZ3HCFREB7AYED504MP8`;
- `01M73V3W43DSZSKXBV1Z0WKTWZ`;
- `01M70CF14M3YC6SR8T9JJ6XQBG`;
- `01M70CF1VBHJAX6MK39FY5W2RY`;
- `01M5F3GVDYTH1Q5V5FCS9YF7V3`.

None has code on main. Each would add or change a source row (design
A2), not the log.

## Scope

1. **One store of rows** (design Part A):
   - every request, checking hold, repository state, problem item, item
     not shared yet, retirement, pending operation, recovery-copy failure,
     recovered draft and record note as rows, each with the time the app
     learned of it;
   - a session history of checks that brought something in;
   - the attention rows, with each thing counted once.
2. **Copy** (Part B): every row in `06`'s register; titles; the Git detail
   in the tooltip.
3. **The Activity panel** (Part C):
   - the list of one-line rows, newest first;
   - the filter and the ⋯ menu with "Clear finished";
   - the keys;
   - the layout notices and the quit row above the list.
4. **The row menu** (Part D), by pointer and keyboard.
5. **The Needs attention popover and footer item** (Part E).
6. **Folded in** (Part F):
   - splitting `model/operations.rs`;
   - a share classified once;
   - the running stage shown;
   - the old columns removed.
7. **Privacy** (Part G) and **keyboard and focus** (Part H).

## Explicit Exclusions And Downstream Obligations

- **Out of scope here:**
  - the repository rows and the footer's status items (Cycle 03a);
  - background polling itself (D6);
  - the Settings sheet (Cycle 04);
  - the sheets and dialogs' frames (Cycle 05).
- **Not built:**
  - an Activity log kept across relaunches (decision 2);
  - names in "Brought in N changes" and in conflicts, which need library
    data;
  - a footer line with the latest Activity status (Wave Deferred Work).
- **Any library, repository-format or CLI change.** A gap found in the
  headless API goes to a separate library ticket.
- **For later Cycles:**
  - a new recovery state is a new source row (design A2) with its N-row
    (A3), not a second store;
  - the open tickets above add their rows the same way.

## Required Contract

1. **One store.** Activity, the popover and the footer's count read the
   same rows. The popover is the attention rows, and the count is their
   number (RFC 40–55; `05`).
2. **Failures stay until resolved.** A failure or recovery state stays
   until its source no longer holds it, and an event until the person
   dismisses it (decision 5). Nothing exists only in a transient
   notification (RFC).
3. **Nothing new is sent.** Every row action runs the handler today's
   control runs. Moving the highlight, filtering and opening a menu or
   the popover send nothing. Enter runs only the row's own button
   (decision 11).
4. **Each thing counts once** (design A3).
5. **Keyboard.** Every row action, the row menu, the filter, ⋯, the
   footer item and every popover action work by keyboard. Each has an
   accessible name and visible focus, and focus returns after a dialog
   (RFC 344–351; `MH-NFR-005`).
6. **Colour is never the only signal.** Every row has an icon and words.
7. **No secret.** Rows, tooltips, menus, tags and the popover hold only
   what design Part G lists. A title may appear there (decision 12). A
   body, comment, conflict side, URL, key or passphrase never does
   (`MH-NFR-003`).
8. **R-60 holds.** Nothing in Activity, the menus, the footer item or
   the popover acts behind a dialog or sheet.
9. **Activity can be hidden.** The rows, times, count and popover do not
   depend on Activity being drawn.
10. **No action changes.** What each action does is unchanged. Only where
    it is offered and its words change (Wave: "No Cycle changes domain
    behavior").

## Acceptance And Exit Evidence

Planned proof. The Status column is filled at the final task from the
branch's own tests, by exact name, and the execution ledger. "On screen"
rows await the owner's checks; Claude records only what the owner
reports.

| # | Wave exit evidence / contract | Proof | Status |
| --- | --- | --- | --- |
| A1 | Failures persist until resolved and appear in both Activity and the popover (Wave exit; contract 2) | `each_source_makes_its_rows` (P1); `the_log_keeps_times_and_events` (P3); `count_and_popover_agree` (P13) | Planned |
| A2 | Every popover action works by keyboard (Wave exit) | `popover_actions_by_keyboard` (P23); `the_popover_opens_lists_and_closes` (P22) | Planned |
| A3 | Activity can be hidden and the popover still works (Wave exit; contract 9) | `the_popover_works_with_activity_hidden` (P24); `activity_hidden_keeps_the_log` (P29) | Planned |
| A4 | No secret in a row, tooltip or popover (Wave exit; contract 7) | `polling_privacy` and D5's three sweeps with the title-only allowance (E-14, P28) | Planned |
| A5 | Rows of time, icon, message, tag and at most one action, Git detail in the tooltip, newest first, the filter (Wave scope) | `request_copy_table`, `share_copy_table`, `state_copy_table`, `activity_time_formats` (P4–P7); `rows_draw_from_the_log_and_filter` (P14) | Planned |
| A6 | Today's operations-area content is carried into rows, not dropped (Wave scope) | `each_source_makes_its_rows` against design A2's table; the tests moved by E-5 and E-6 pass unchanged in meaning | Planned |
| A7 | The count includes the Wave's repository states, each thing once (Wave scope; decision 6) | `attention_counts_each_thing_once` (P2, test-first); `count_and_popover_agree` (P13) | Planned |
| A8 | The three K-19 rows fit Activity (Wave, D6's amendment) | `rows_fit_a_narrow_activity` (P21) | Planned |
| A9 | Repository state is read from Cycle 03a's function (obligation) | Cycle 03a's panel tests unchanged; P12 in `each_source_makes_its_rows` | Planned |
| A10 | Activity and its row menu work by keyboard (contract 5) | `activity_by_keyboard` (P15); `the_row_menu_by_pointer_and_keyboard` (P17); the rewritten keyboard tests (E-7) | Planned |
| A11 | Nothing new is sent; each action runs its handler (contracts 3, 10) | `each_action_runs_its_handler` (P18); the moved tests (E-5, E-6) | Planned |
| A12 | Nothing acts behind a dialog or sheet (contract 8) | `nothing_acts_behind_a_dialog_or_sheet` (P27) | Planned |
| A13 | The folded-in work | The operations model tests unchanged (P11); `shares_are_classified_once` (P9); `stages_are_shown` (P10) | Planned |
| A14 | Activity, the popover and the footer item match the design in the dark theme | On screen: the owner, against frames 1a and 2f | Planned |

## Product-Owner Decisions (2026-11-12)

Wave 04 decisions 1–10 (2026-11-07) and the Cycle 01, 02 and 03a
decisions govern and are not repeated.

**Made during Cycle 03a's planning, recorded on this ticket:**

1. **Cycle 03 is split.** 03a is the Repositories panel and the footer's
   status items. 03b, this Cycle, is Activity and the Needs attention
   popover (3a decision 1).
2. **Activity becomes one timed log,** newest first, over today's
   stores. Each row has:
   - the time the app learned of it (this session);
   - a plain-language message;
   - a repository tag;
   - one action;
   - the technical detail in its tooltip.

   Failures stay until resolved, and there is the "Show only things that
   need attention" filter.
3. **Folded in:**
   - from `01M6H7VXJJHRYHTW9S8TJ88921` and `01M6RAS63EWVHWWVBSZ836RXG2`:
     splitting `model/operations.rs` and `view/operations.rs`, and
     classifying shares once instead of on every render;
   - from `01M5CS4MH25QQ769HV61ZRKHXG`: "Finishing" hiding the running
     stage.

**Answered by the owner on 2026-11-12, planning this Cycle:**

4. **Plan against Cycle 03a's work now.** 03a's published branch is
   merged into this ticket branch (merge, never rebase). Implementation
   still waits for 03a to close, then merges main. *Alternative:* plan
   on main and cite 03a's commit.
5. **States leave; events stay.**
   - Ongoing conditions leave Activity and the popover when resolved:
     a locked key, an unreachable shared copy, a pending operation, a
     recovery copy failing.
   - Things that happened stay as history for the session, the row
     showing its latest outcome: saved, shared, closed, a failed share
     and its retry.

   *Alternative:* everything stays, with a resolved wording.
6. **The count is everything the popover lists, each thing once.**
   - **Today's items:** failed requests not dismissed, recovery-copy
     failures, the draft store, pending operations, checking holds.
   - **Plus Cycle 03a's repository states:** Can't open, Needs fixing,
     Can't reach, and Locked or Needs attention from a stored check
     result or from a check's problems.
   - **Dropped:** a pending operation that reads "Nothing to do" stops
     counting.

   *Alternative:* leave Can't reach out.
7. **Checks get a row when they changed or found something.**
   - A check that brought something in gets a row. A check that listed
     problems gets the problem items' rows.
   - Quiet automatic checks add nothing.
   - A Check now the person asked for always has its request's row.

   *Alternatives:* every check, or the latest per repository.
8. **Extra actions go in a row menu,** as on Cycle 03a's repository rows.
   - The menu opens by right click, `shift-f10` or the Menu key, and its
     first lines are the Git detail.
   - Abandon and other risky actions are never the row's button.

   *Alternative:* all buttons on the highlighted row.
9. **Clearing.** Dismiss is in each finished row's menu (a dismissed
   failure leaves Activity and the count, as today). ⋯ holds "Clear
   finished", which dismisses every finished row that needs nothing.
   *Alternatives:* Dismiss only, or no dismissing failures.
10. **A check's problem items are one row each,** in Activity and the
    popover, with the problem as the message and Open as the action. Each
    counts once. *Alternative:* one row per repository.
11. **Enter presses the highlighted row's button.** A row with no button
    opens its item. *Alternative:* Enter always opens the item.
12. **Titles are allowed in Activity, nothing else (owner's eye).**
    - Rows, their tooltips and menus, and the popover may name an item by
      its title, as the design shows.
    - Bodies, comments and conflict sides still never appear.
    - Wave 03 D5's privacy rule and sweep are amended for titles only
      (design Document Amendment 6).

    *Alternative:* short codes only.

**Proposed by the planner in the outline and accepted when the owner said
"write the documents" (2026-11-12):**

13. **Three calls in the outline:**
    - nothing is capped in a session, and "Clear finished" is the relief;
    - a checking hold over several repositories is one row, tagged with
      the first and "+N", its tooltip naming them all;
    - every request kind gets a row, including those the app sends
      itself, such as launch refreshes and Prepare to edit.

**Proposed by the planner, for the owner to confirm:**

14. **Existing-test edits.** The named edits E-1 to E-16 are part of
    approving the design; those marked **Removes** are named there. Any
    other edit this Cycle's changes need is pre-authorized, provided it:
    - is recorded in the ledger as E-17 onward, with its reason;
    - is checked by the task's reviewer;
    - is called out in the review-ready summary when it removes an
      assertion.

    Tasks 6 and 7 move about 170 existing tests onto a seam (design,
    Testing). They are green before and after, and keep each assertion's
    meaning.
15. **Test runs follow `AGENTS.md`.** Between tasks, only the tests a task
    wrote or changed run, by exact name. The fast tier runs once, at
    review-ready, with `--no-fail-fast`.
16. **Information rows for repositories that cannot share (owner's eye).**
    A repository with no shared copy, or with a non-SSH remote, gets a
    row that is never counted, with Set up sharing…, as `03`'s example
    shows. It stays all session; the filter hides it.
17. **A reminder row for each item not shared yet (owner's eye).**
    - This covers main and items saved ahead of the shared copy, with
      Share…. Today Activity offers Share… only for items never shared.
    - The rows agree with Cycle 03a's "N to share", since they read the
      same list.
    - They are never counted.
18. **A press outside the popover closes it and also acts** on what was
    pressed, unless it is the footer item itself, which closes it. Tab
    stays inside the popover; Escape leaves it.
19. **A repository's row stays while a retry checks it** (owner's eye).
    Checking… outranks Can't reach on the panel's row (3a decision 8), but
    the Activity row and the count hold until the check answers, so they
    do not flicker every retry.
20. **A pending conflict makes no repository row.** The conflict's own
    row (its pending operation, or the request that stopped on it)
    offers Review changes. After a relaunch, before the pending list is
    read, Activity may not show why checking waits; the panel's row
    still says Needs attention.

## Known Limits

These are visible to users; the design's Known Limits give the detail.

- **History:**
  - Rows and their times last for the session. After a relaunch, what
    the stores keep returns, timed by the new session (K-1).
  - Nothing is capped within a session (K-4).
- **Wording:**
  - An item with no title known is named by its short code (K-2).
  - A failure's reason is the library's fixed sentence, which sometimes
    names a CLI command (K-3).
  - No names: "Brought in N changes" names no one, and a conflict says
    "someone else" (K-6).
  - "Tries again" gives no minutes (K-7).
  - A busy share's "May be waiting for …" can change while shown (K-12).
- **Times** are UTC where the local offset cannot be read, which today is
  only off Unix (K-5).
- **Tooltips** show on hover only. The detail is also the row's accessible
  description and its menu's first lines (K-8).
- **Edge cases:**
  - A pending conflict with no pending row loaded has no Activity row
    (K-9).
  - A pending row with no operation ID can change its time (K-10).
  - A press outside the popover also acts (K-11).

## Open Questions

None for the owner beyond confirming decisions 14–20. The design's U-1 to
U-7 are checked by the tasks that first rely on them.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Owner, 2026-11-12 | "Wave 4 Cycle 3a is in step 10 (final review) … substantially complete … start planning for Wave 4 Cycle 3b now on that basis" | Owner direction | Research, then questions; decision 4 | Session, 2026-11-12 |
| Planning research | No row has a time; the footer count and the panel's Needs attention disagree (3a K-9); the Kit `Popover` cannot be driven and the Kit `List` presses Confirm on click; shares are classified on every render; "Finishing" hides the stage; D5 bars titles from the operation area; 494 test lines in 54 files reach the area by element id | Facts; owner questions; rulings | Design facts 1–46; decisions 4–13; design R-8, R-10, R-21, R-36 | Design |
| Owner, 2026-11-12 | Base; resolved rows; the count; checks; extra actions; clearing; problem items; Enter; the outline; titles | Owner decisions | Decisions 4–13 | Session, 2026-11-12 |
