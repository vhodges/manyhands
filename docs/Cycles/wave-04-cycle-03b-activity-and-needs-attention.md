---
title: "Wave 04 Cycle 03b: Activity And Needs Attention"
date: 2026-11-12
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M79CDKRTC646SBMMNDWY9FHM"
---

# Wave 04 Cycle 03b: Activity And Needs Attention

Ticket `01M784Y94Q3AXT6XJX38F5VK1V` (`vh-dt0aq`).
[Design](../plans/2026-11-12-wave-04-cycle-03b-activity-and-needs-attention-design.md) and
[implementation plan](../plans/2026-11-12-wave-04-cycle-03b-activity-and-needs-attention-implementation.md).

Status: **approved 2026-11-13** (revision 5). The owner wrote: "All
approved." (decision 26). On 2026-11-12 the owner asked to plan this
Cycle while Cycle 03a was in its final review ("substantially complete").
Cycle 03a has since closed and merged (main `ba95e1b7`). The owner
answered decisions 4–12 in that session and accepted the planner's
outline with decision 13 ("write the documents"). Decisions 1–3 were
made during Cycle 03a's planning and are recorded on this ticket.
Decisions 14–20 were the planner's proposals, each with its reason; the
owner confirmed them at approval. The owner answered decisions 21–24 on
2026-11-13, after review round 1, and decision 25 the same day, after
review round 3. Those marked **(owner's eye)** change something people
see today or depart from an authority. Revision 2 applied review round 1 (4 Blocking,
39 Important, 25 Minor across two reviewers) and decisions 21–24.
Revision 3 applied review round 2 (scoped re-review 1 Blocking, 7
Important, 13 Minor; adversarial 1 Blocking, 12 Important, 9 Minor).
Revision 4 applied review round 3 (scoped re-review 0 Blocking, 6
Important, 8 Minor; adversarial 6 Blocking, 7 Important, 4 Minor) and
decision 25, and redesigned retirements: no rows derived from the
attempt's retirement lines (R-51).

Revision 5 applies review round 4 (scoped re-review 1 Blocking, 5
Important, 8 Minor; adversarial 5 Blocking, 6 Important, 3 Minor) and
re-grounds on Cycle 03a's merged code (`81789331`). **Revision 5 had no
independent review**; the owner approved it without one, as recommended
(decision 26). Round 4 was the fourth to find Blocking issues in
the attention count. As the planning skill directs, that contract is the
N-table with its precedence, latches and T-rows (design A3, A4). Task 5
writes it test-first with a design checkpoint, and any row its
implementer cannot express goes back to the orchestrator and the owner.

- Drafted on the ticket branch at `b37d2c2b`. That commit merges Cycle
  03a's published head `3086666f` into this ticket's base, origin/main
  `37be101d` (decision 4).
- Review round 1 read revision 1 at `74c9aabb`:
  - contract and consistency: 3 Blocking, 24 Important, 14 Minor;
  - source and feasibility: 1 Blocking, 15 Important, 11 Minor.
- Review round 2 read revision 2 at `b830e8e3`:
  - scoped re-review: 1 Blocking, 7 Important, 13 Minor;
  - adversarial: 1 Blocking (the same), 12 Important, 9 Minor.
- Review round 3 read revision 3 at `4c596645`:
  - scoped re-review: 0 Blocking, 6 Important, 8 Minor;
  - adversarial: 6 Blocking, 7 Important, 4 Minor.
- Review round 4 read revision 4 at `05246630`:
  - scoped re-review with re-grounding: 1 Blocking, 5 Important, 8
    Minor, and 11 moved citations;
  - adversarial review of the count: 5 Blocking, 6 Important, 3 Minor.
- Cycle 03a closed and merged (main `ba95e1b7`, ticket
  `01M784Y8C73B39GR4JWENXE5D0`). The ticket branch merged main at
  `81789331`, and revision 5 is grounded there.
- The plan has 14 tasks (0–13). If the owner splits the Cycle, 03b-1 is
  Tasks 0–10 and 03b-2 is Tasks 11–13 (design Risks).
- **Revision 5 had no independent review.**
- Approved 2026-11-13 (decision 26); the design's Document Amendments
  1–12 were applied in the approval commit.

## Purpose And Authority

Make recovery visible in plain language, in the places the design puts it
(Wave, Cycle 03b).

After this Cycle, Activity is one timed log. Each line says:

- when the app learned of it;
- what happened, or what is wrong, in plain words;
- which repository it concerns;
- the one thing to do about it, when there is one (decision 24).

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
     (amended by decisions 12 and 21);
   - D6: holds, checks, Quit's "still stopping", its privacy rule
     (decision 21) and its wrapping rows (decision 22);
   - Cycle 01: the Activity panel, the footer, the count's rules
     (decision 3, superseded by decision 6), K-19;
   - Cycle 03a: repository state, the footer order, the unshared list,
     the menu layer.

## Entry Evidence And Dependency Boundary

| Gate | State at planning (`b37d2c2b`) |
| --- | --- |
| Cycle ticket before the plan (`AGENTS.md`) | `01M784Y94Q3AXT6XJX38F5VK1V`, open, created 2026-11-12 from main and published |
| Latest main merged into the ticket branch | Created from origin/main `37be101d`; main merged again at `81789331` for revision 5. Task 0 merges main once more |
| Cycle 03a merged | Yes: merged (main `ba95e1b7`); this branch merged main at `81789331`. At planning, only its published head `3086666f` was merged in (decision 4). Revision 5 re-read its final code; Task 0 re-checks only what changes after `81789331` (design U-6) |
| Wave 03 D6 merged | Yes: main `48653d01` |
| GPUI Kit 0.6.6 names read | `List`, `Popover`, `PopupMenu` and `Button` read from the locked source during planning (design facts 31–34). `uniform_list` read during planning; U-2 is checked in Task 9 |
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
   dismisses it (decision 5). A failed request stops counting once it is
   fixed, and its row stays as history until dismissed or cleared
   (decision 25).
   Nothing exists only in a transient notification (RFC).
3. **Nothing new is sent.** Every row action runs the handler today's
   control runs. Moving the highlight, filtering and opening a menu or
   the popover send nothing (tested: P31). Enter runs only the row's own
   button (decision 11), and does nothing on a row with no action
   (decision 24).
4. **Each thing counts once** (design A3).
5. **Keyboard.** Every row action, the row menu, the filter, ⋯, the
   footer item and every popover action work by keyboard. Each has an
   accessible name and visible focus, and focus returns after a dialog
   (RFC 344–351; `MH-NFR-005`).
6. **Colour is never the only signal.** Every row has an icon and words.
7. **No secret.** Rows, tooltips, menus, tags and the popover hold only
   what design Part G lists. A title may appear there (decisions 12 and
   21), never a draft's own title. A body, comment, conflict side, remote
   URL or address, file path (a repository's folder path included), key
   or passphrase never does (`MH-NFR-003`). A repository row's detail
   names its shared copy by name only, not as Cycle 03a's tooltip does
   (design R-48).
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
| A1 | Failures persist until resolved and appear in both Activity and the popover, every counted failure (Wave exit; contract 2; decision 24) | `each_source_makes_its_rows` (P1); `the_log_keeps_times_and_events` (P3); `count_and_popover_agree` (P13); `clear_finished_keeps_failures_and_states` (P19); `a_checking_repository_keeps_its_row` (P30) | Planned |
| A2 | Every popover action works by keyboard (Wave exit) | `popover_actions_by_keyboard` (P23); `the_popover_opens_lists_and_closes` (P22); `the_popover_closes` (P25); `status_items_say_and_choose`' status table (P26) | Planned |
| A3 | Activity can be hidden and the popover still works (Wave exit; contract 9) | `the_popover_works_with_activity_hidden` (P24); `activity_hidden_keeps_the_log` (P29) | Planned |
| A4 | No secret in a row, tooltip or popover (Wave exit; contract 7) | `polling_privacy` and D5's three sweeps with the title-only allowance (E-12, E-14, P28) | Planned |
| A5 | Rows of time, icon, message, tag and at most one action, Git detail in the tooltip, newest first, the filter (Wave scope) | `request_copy_table`, `share_copy_table`, `state_copy_table`, `activity_time_formats` (P4–P7); `rows_draw_from_the_log_and_filter` (P14) | Planned |
| A6 | Today's operations-area content is carried into rows, not dropped (Wave scope) | `each_source_makes_its_rows` against design A2's table and B0's map; the tests moved by E-5 and E-6 pass unchanged in meaning | Planned |
| A7 | The count includes the Wave's repository states, each thing once, and a fixed failure stops counting (Wave scope; decisions 6 and 25) | `attention_counts_each_thing_once` (P2, test-first); `count_and_popover_agree` (P13) | Planned |
| A8 | The three K-19 rows fit Activity (Wave, D6's amendment; decision 22) | `rows_fit_a_narrow_activity` (P21) | Planned |
| A9 | Repository state is read from Cycle 03a's function (obligation) | Cycle 03a's panel tests, run by name after `RepoFacts` building moves (P12) | Planned |
| A10 | Activity and its row menu work by keyboard (contract 5) | `activity_by_keyboard` (P15); `rows_by_pointer` (P16); `the_row_menu_by_pointer_and_keyboard` (P17); `a_dialog_from_a_row_returns_focus_to_its_row` (P20); the rewritten keyboard tests (E-7) | Planned |
| A11 | Nothing new is sent; each action runs its handler (contracts 3, 10) | `each_action_runs_its_handler` (P18); `navigation_sends_nothing` (P31); the moved tests (E-5, E-6) | Planned |
| A12 | Nothing acts behind a dialog or sheet (contract 8) | `nothing_acts_behind_a_dialog_or_sheet` (P27) | Planned |
| A13 | The folded-in work | The operations model tests unchanged (P11); `shares_are_classified_once` (P9); `stages_are_shown` (P10); `D/view/operations.rs` below 500 lines (checked in Task 9) | Planned |
| A14 | Activity, the popover and the footer item match the design in the dark theme | On screen: the owner, against frames 1a and 2f | Planned |
| A15 | Activity agrees with Cycle 03a's "to share" and repository rows (obligation) | Cycle 03a's panel tests, run by name (P12); `reminders_match_to_share` (P32) | Planned |
| A16 | Colour is never the only signal (contract 6) | `each_source_makes_its_rows` (every row has a kind and a title); `rows_draw_from_the_log_and_filter` | Planned |

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

    Decision 21 extends this to D6's rule.

**Proposed by the planner in the outline and accepted when the owner said
"write the documents" (2026-11-12):**

13. **Three calls in the outline:**
    - nothing is capped in a session, and "Clear finished" is the relief;
    - a checking hold over several repositories is one row, tagged with
      the first and "+N", its tooltip naming them all;
    - every request kind gets a row, including those the app sends
      itself, such as launch refreshes and Prepare to edit.

**Proposed by the planner and confirmed by the owner at approval (2026-11-13):**

14. **Existing-test edits.** The named edits E-1 to E-16 are part of
    approving the design; those marked **Removes** are named there. Any
    other edit this Cycle's changes need is pre-authorized, provided it:
    - is recorded in the ledger as E-17 onward, with its reason;
    - is checked by the task's reviewer;
    - is called out in the review-ready summary when it removes an
      assertion.

    Tasks 6 to 8 move about 170 existing tests onto a seam (design,
    Testing). They are green before and after, and keep each assertion's
    meaning. Converted tests act through the seam, not the drawn control,
    so they no longer prove their control is drawn; P18 and P23 prove each
    action kind from a drawn button, the menu and the popover (design
    R-21). Label reads that compared today's words become reads of model
    state, so the copy is asserted by the copy tables (P4–P7) instead;
    these are **Removes**, listed per file in the ledger.
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
    pressed, unless it is the footer item itself, which closes it (owner's
    eye). Tab stays inside the popover; Escape leaves it. This departs
    from the mockup, whose full-window click-catcher swallows a press
    outside (`docs/design/mockups/source/Workspace.dc.html:485`). It
    matches the app's dock and repository menus, which also let the press
    through.
19. **A repository's row stays while a retry checks it** (owner's eye).
    Checking… outranks Can't reach on the panel's row (3a decision 8), but
    the Activity row and the count hold until the check answers, so they
    do not flicker every retry.
20. **A pending conflict makes no repository row (owner's eye).** The
    conflict's own row (its pending operation, or the request that
    stopped on it) offers Review changes. After a relaunch, before the
    pending list is read, Activity may not show why checking waits; the
    panel's row still says Needs attention.

**Answered by the owner on 2026-11-13, after review round 1:**

21. **Titles under D6's rule too.** Decision 12's amendment also covers
    Wave 03 D6's privacy rule. D6's contract 10 bars titles from operation
    entries and status lines. *Alternative:* D6's rows stay title-free.

    Two limits stay regardless:
    - a save or a create names its item by the item's title as read,
      never by the draft's own title (Wave 03 D2's rule);
    - file paths never appear in a row.
22. **One-line rows; D6's decision 13 amended.**
    - D6 decision 13 says each row D6 adds wraps. Instead every row is one
      line: the message ends in an ellipsis, and the button never leaves
      the row.
    - The full message (title, then description) opens the tooltip, then
      the time and the detail; the accessible description is the same
      text. The row menu's first lines are the full message, then the
      detail.

    *Alternative:* wrap to at most two lines.
23. **After a dialog, focus returns to the row (owner's eye).**
    - When a dialog or sheet opened from an Activity row closes, focus
      returns to the Activity list with the same row highlighted, whether
      the action came from the button, the menu or Enter.
    - The row counts as the invoking control (RFC 344–351; Cycle 05's
      rule).
    - Today focus returns to the exact button.

    *Alternative:* back to the button when it was used.
24. **At most one action; failures once in the popover.**
    - The Wave's "its single action" becomes "at most one action". A row
      with none still counts and says what is wrong, and Enter does
      nothing on it.
    - "Failures … appear in both Activity and the popover" means every
      counted failure. Repeats about one operation show once in the
      popover and all in Activity.

    *Alternative:* give every row an action.

**Answered by the owner on 2026-11-13, after review round 3:**

25. **A fixed failure stops counting by itself (owner's eye).**
    - **When it stops counting.** A failed request no longer counts, and
      leaves the popover, once any of these happens:
      - a later request about the same thing succeeded;
      - its operation finished;
      - a resend took it over.
    - **What stays.** Its row stays in Activity as history until
      dismissed or cleared.
    - **What it changes.** Cycle 01's "until dismissed" rule changes
      (design Document Amendment 11); the Wave and design `03` say
      "failures persist until resolved". The owner was asked: "Today a
      failed request counts in Needs attention until you dismiss it, even
      after the problem was fixed another way… Should a fixed failure
      stop counting by itself?" and chose "Stop counting once fixed".

    The design states it as N22 and N23, latched per attempt (A3; R-56).

    *Alternative:* count until dismissed, as today.

**Answered by the owner at approval (2026-11-13):**

26. **Approved, and implementation authorized, subagent-driven and
    serial, in a fresh session.** The owner wrote "All approved." in
    answer to the approval message, which asked the owner to:
    - approve the three documents (revision 5);
    - approve the named test edits E-1 to E-16, including the removals
      marked;
    - confirm decisions 14–20 as proposed;
    - skip, as recommended, a further independent review of revision 5;
    - choose the execution method, recommended subagent-driven and
      serial, in a fresh session from a handoff brief, as on Cycles 02
      and 03a;
    - authorize implementation of Tasks 0–13 and pushing this ticket
      branch.

    One implementer per task, then a fresh reviewer, one at a time,
    following `.agents/skills/implementing-a-cycle/SKILL.md` and
    `superpowers:subagent-driven-development`. The fresh session works
    from the brief `docs/plans/2026-11-12-wave-04-cycle-03b-handoff.md`.
    Authorized: Tasks 0–13, the named test edits E-1 to E-16 and further
    edits under decision 14, and pushing this ticket branch after each
    task. Not authorized without the owner's go-ahead at the time: a pull
    request, the merge, closing the ticket, worktree cleanup, native CI and
    creating tickets. Amended the same day by the owner: "pushing is how
    people and agents will collaborate in manyhands so it's okay to push
    branches", so pushing other branches, including through `ticket save`
    or `comment add` on another ticket, needs no separate go-ahead.

## Known Limits

These are visible to users; the design's Known Limits give the detail.

- **History:**
  - Rows and their times last for the session. After a relaunch, what
    the stores keep returns, timed by the new session (K-1).
  - Nothing is capped within a session (K-4).
- **Wording:**
  - An item with no title known is named by its short code (K-2). A new
    item reads so until a list read gives its title.
  - A failure's reason is the library's fixed sentence, which sometimes
    names a CLI command (K-3).
  - No names: "Brought in N changes" names no one, and a conflict says
    "someone else" (K-6).
  - "Tries again" gives no minutes (K-7).
  - A busy share's "May be waiting for …" can change while shown (K-12).
- **Times** are UTC where the local offset cannot be read, which today is
  only off Unix (K-5).
- **Tooltips** show on hover only. The full message and the detail are
  also the row's accessible description and its menu's first lines
  (K-8).
- **Edge cases:**
  - A pending conflict with no pending row loaded has no Activity row
    (K-9).
  - A pending row with no operation ID can change its time (K-10).
  - A press outside the popover also acts (K-11).
  - An unreachable repository with problems from an earlier check shows
    the problems, not Can't reach, as its panel row does (K-13).

## Open Questions

None from the owner: decisions 1–26 are settled. The design's U-1, U-2,
U-4, U-5, U-7 and U-8 are checked by the tasks that first rely on them.
U-6 was re-read at `81789331`, and Task 0 re-checks only what changes
after that. U-3 is settled (design fact 47).

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Owner, 2026-11-12 | "Wave 4 Cycle 3a is in step 10 (final review) … substantially complete … start planning for Wave 4 Cycle 3b now on that basis" | Owner direction | Research, then questions; decision 4 | Session, 2026-11-12 |
| Planning research | No row has a time; the footer count and the panel's Needs attention disagree (3a K-9); the Kit `Popover` does not fit the app's layer rules (R-10) and the Kit `List` presses Confirm on click; shares are classified on every render; "Finishing" hides the stage; D5 bars titles from the operation area; 494 test lines in 54 files reach the area by element id | Facts; owner questions; rulings | Design facts 1–46; decisions 4–13; design R-8, R-10, R-21, R-36 | Design |
| Owner, 2026-11-12 | Base; resolved rows; the count; checks; extra actions; clearing; problem items; Enter; the outline; titles | Owner decisions | Decisions 4–13 | Session, 2026-11-12 |
| Review round 1, contract and consistency (revision 1, `74c9aabb`) | 3 Blocking, 24 Important, 14 Minor: titles under D6's rule; paths in rows; popover actions by keyboard; the seam; the attention table; copy rules; focus; task numbering | Blocking, Important, Minor | Revision 2; decisions 21–24 | Change list, 2026-11-13 |
| Review round 1, source and feasibility (revision 1, `74c9aabb`) | 1 Blocking, 15 Important, 11 Minor: how the seam acts; inputs to the derivation; handler names and signatures; the Kit `Popover`; tests the inventory missed; counts and line references | Blocking, Important, Minor | Revision 2; design R-40 to R-45 | Change list, 2026-11-13 |
| Owner, 2026-11-13 | Titles under D6's rule; one-line rows; focus after a dialog; at most one action and counted failures | Owner decisions | Decisions 21–24 | Session, 2026-11-13 |
| Review round 2, scoped re-review and adversarial (revision 2, `b830e8e3`) | Scoped: 1 Blocking, 7 Important, 13 Minor. Adversarial: 1 Blocking (the same), 12 Important, 9 Minor: a repository row's detail carried the folder path and the remote's address (Blocking); the log and the attention post-pass; focus after the popover; the seam; inputs the derivation lacked | Blocking, Important, Minor | Revision 3; design A3 and A4 as tables of required outcomes (N1–N21, T1–T13); design R-46 to R-50 | Change list, 2026-11-13 |
| Owner, 2026-11-13, after review round 3 | Should a fixed failure stop counting by itself? | Owner decision | Decision 25; design N22 and N23; Document Amendment 11 cites decisions 6 and 25 | Session, 2026-11-13 |
| Review round 3, scoped re-review and adversarial (revision 3, `4c596645`) | Scoped: 0 Blocking, 6 Important, 8 Minor. Adversarial: 6 Blocking, 7 Important, 4 Minor. Most sit in how retirements became rows (the S8 starts, kept rows, R-42) and when a failed request stops counting; others: a lifecycle's Resume on its pending row, L3's offers, a Busy read, the revision, `clearable` and Open…, the sentinels | Blocking, Important, Minor | Revision 4: retirements redesigned, with no rows derived from the attempt's retirement lines (design R-51); the subject, N2, N4, N9 and N21–N24 with their precedence (decision 25); design R-52 to R-55; N20, R-19 and R-42 withdrawn, T7 and T8 replaced. Scoped findings 1–3 (the S8 starts, kept counting, N20 against N21) are resolved by the redesign | Change list, 2026-11-13 |
| Review round 4, scoped re-review with re-grounding and adversarial (revision 4, `05246630`) | Scoped: 1 Blocking, 5 Important, 8 Minor, and 11 moved citations. Adversarial: 5 Blocking, 6 Important, 3 Minor. Almost all Blocking findings sit in decision 25's "fixed" rule, the fourth round to find Blocking issues in the attention count; others: `check_applied`, clearing fixed rows, S8's information events, S9's lifecycle row, the popover's focus and revision check, dead code, stale text. Cycle 03a merged in between | Blocking, Important, Minor | Revision 5, **with no independent review**: the latch and exact subjects (design R-56, R-57), N22's five clauses, N2 by operation, N23 undone, N25, T14–T18; `check_applied` recorded (R-58); settled failures clearable; S8's information events per item (R-59); re-grounded on Cycle 03a's merged code at `81789331`. The contract is the N-table with its precedence, latches and T-rows, written test-first in Task 5; a row its implementer cannot express goes back to the orchestrator and the owner | Change list, 2026-11-13 |
| Owner, 2026-11-13 | Approval message: documents, named edits, decisions 14–20, no further review, execution method, authorization | Owner decisions | Decisions 14–20 confirmed; decision 26 | Session, 2026-11-13 |
