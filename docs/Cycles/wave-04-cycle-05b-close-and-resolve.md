---
title: "Wave 04 Cycle 05b: Close Ticket And Resolve Conflict"
date: 2026-11-15
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M7FACCD21XG1CFTEBQJ80Y52"
---

# Wave 04 Cycle 05b: Close Ticket And Resolve Conflict

Ticket `01M7D7JK5MSY6J471JEKG3CZXR` (`vh-qjw5e`).
[Design](../plans/2026-11-15-wave-04-cycle-05b-close-and-resolve-design.md) and
[implementation plan](../plans/2026-11-15-wave-04-cycle-05b-close-and-resolve-implementation.md).

Status: **approved 2026-11-15** (revision 4). The owner wrote:
"Recommendations, test edits, implementation and documents are
approved. Subagent driven development in a fresh session by a hand off
brief and prompt." (decision 13). **Revision 4 had no independent
review**; the owner approved it without one.

- On 2026-11-15 the owner asked to plan Cycle 05b ("Lets plan 05b
  next"), once Cycle 05a's documents were approved (05a decision 21).
- The same day the owner answered decisions 1–5 and accepted the
  planner's outline with decision 6 ("Write the documents").
- After review round 1 the owner answered decisions 11 and 12.
- Decisions 7–10 were the planner's proposals, each with its reason;
  the owner confirmed them at approval.
- Those marked **(owner's eye)** change something people see today or
  depart from an authority.

Revision history:

- Drafted on the ticket branch at `ac50ba1a`: main `ea6d899b` with
  Cycle 05a's branch (`2c398b03`, which carries Cycle 04's branch and
  05a's Tasks 0–3) merged in, as the ticket's first planning step allows
  ("with 05a merged, or 05a's approved branch"). Revision 1 at
  `702e569f`.
- Review round 1 read revision 1 at `702e569f`:
  - source and feasibility: 1 Blocking, 6 Important, 10 Minor;
  - contract and consistency: 2 Blocking, 16 Important, 7 Minor.
- Revision 2 applied both, and decisions 11 and 12. The follow-up
  library ticket (decision 1) was created and published.
- Review round 2 read revision 2 at `874bb66b` (and `aaf6b475`, the same
  documents with main merged):
  - scoped re-review: 1 Blocking, 4 Important, 8 Minor;
  - adversarial review of the sheet's closes and focus and the dialog's
    stages: 1 Blocking, 4 Important, 9 Minor.
- Revision 3 applied both. Each Blocking finding was a missing case: a
  test fixture that offers no Retry, and a running dialog left with no
  Tab stop after Stop. Each is now stated (design P13, R-23).
- Review round 3 read revision 3 at `3964af7c`: scoped re-review, 0
  Blocking, 2 Important, 7 Minor; round 2's findings resolved.
- Revision 4 applied it: a privacy test that drives the passphrase
  prompt (design P12), and how the lifecycle dialog's ways into the
  sheet behave under a prompt (R-24). It also reads Cycle 05a's branch
  at `d7b39f62` (05a's Task 6), whose sheet checks it now names.
  **Revision 4 was not independently reviewed.**
- Approved 2026-11-15 (decision 13). The approval commit applied the
  design's Document Amendments 2–5 (`05`, `06`, `08` and Cycle 03b's
  copy). Amendment 1 (the Wave) and Amendment 6 (Cycle 05a's design,
  whose branch is still moving) wait for Task 0.

## Purpose And Authority

After this Cycle, the two surfaces Wave 03 D5 built look like the
design and sit on Cycle 05a's frame:

- Close ticket… and Approve and merge… show each stage the way `05`
  draws it: a header that says where the run stands, numbered steps that
  turn into ticks, a spinner and a red cross, an outcome box, and buttons
  on the right with the action last;
- Resolve conflict opens as a 920 px sheet on the right, with its header
  and footer fixed and its parts named in words.

What they do does not change: the same requests, the same stale
confirmations, Retry, Stop at a safe point, recovery routes and choices.

Authorities, in order:

1. `AGENTS.md` on origin/main.
2. [Wave 04](../Waves/wave-04-workspace-design.md):
   - **Cycle 05b:** purpose, "Check first", scope, exclusions and exit
     evidence, as Cycle 05a's Wave Amendment 1 split them;
   - the risk row "Restyling the lifecycle dialog and the resolve
     conflict sheet regresses D5 behavior";
   - Testing.
3. [Desktop information architecture and editor RFC](../RFC/desktop-information-architecture-and-editor.md):
   Retry shows completed and remaining steps and reuses the original
   operation (283); confirmation enumerates the whole branch's effects,
   and a changed observation invalidates it (285–289). The
   [Git workflow RFC](../RFC/git-workflow-and-conflict-recovery.md):
   external changes invalidate the preview (542–546); no automatic
   winner.
4. **The design handoff:**
   - [`05-sheets-and-dialogs.md`](../design/05-sheets-and-dialogs.md):
     the common frame and its as-built notes (Cycle 05a), "Close ticket
     dialog (2c → 2d → 2e)" and "Resolve conflict sheet (2g)";
   - [`06-language-and-states.md`](../design/06-language-and-states.md),
     [`07-design-tokens.md`](../design/07-design-tokens.md),
     [`08-gpui-kit-mapping.md`](../design/08-gpui-kit-mapping.md);
   - frames `screenshots/2c`, `2d`, `2e` and `2g`.
5. **Predecessors:**
   - Wave 03 D5: both surfaces, their rulings (R-2, R-3, R-8, R-20,
     R-27, R-31, R-34, R-35) and decisions (6, 11, 15);
   - Cycle 03b: Activity and Needs attention, which open Resolve
     conflict and reopen the lifecycle dialog;
   - Cycle 04: the sheet owner, `sync_sheet`, Settings' focus contract;
   - **Cycle 05a:** the common frame, the theme, the Add repository
     sheet's owner, and its decisions 7 and 15 (Escape and button order
     kept, except where a Cycle says otherwise).

## The Wave's "Check First"

Read-only comparison, 2026-11-14 (recorded in the ticket), confirmed at
`ac50ba1a` (design facts 1–24). Only the differences are in scope.

**Close ticket and Approve and merge** (`D/view/lifecycle*`):

| Element (`05`, 2c–2e) | Today | In scope |
| --- | --- | --- |
| Width 580 | 580 | — |
| Header: 32 px tile, a title per stage, subtitle | a 20 px kind icon, one title ("Close ticket"), subtitle, branch line | yes (A1, A2) |
| Confirm lead | as designed | — |
| Step list, bordered, numbered | as designed | — |
| Step order: Merge, then Mark as closed; no Share main | Mark as closed, then Merge; Share main when publishing (D5 R-8) | no: the library's order stays |
| Step wording | close; the remote and main named (D5 R-35) | no |
| Files: icon, path, tag, `+n −m` | icon, path, tag (and a `comment` tag); no counts (D5 R-3) | no counts (decision 1) |
| Code note | as designed | — |
| Becomes ready to start, at confirm | none (D5 decision 6) | no (decision 2) |
| Git line | on confirm only | yes: also running, waiting, failed, stopped and done, as 2d and 2e draw it, when the dialog holds the preview (design A5, A6) |
| Footer hint, Cancel · **Close ticket** | none; **Close ticket** · Cancel, left | yes (decision 4) |
| Local-only step 2 | as designed | — |
| Running: lead above, ticks, spinner, numbers, progress in the step | lead below; state words; no notes | yes (A3, A5); progress as stage words (decision 1) |
| Running: Stop only; Esc or backdrop hides | Stop and Close; Esc hides; backdrop inert | yes (decision 4) |
| Failed: red step and cause, outcome box, Not now · fix · **Retry** | words "Failed"; message at the bottom; no box; Retry first, Close last | yes (A3, A6, A7) |
| Done: "Ticket closed", outcome box, "Now ready to start", **Done** | "Done", lines, "Now ready:", Close | yes |

**Resolve conflict** (`D/view/conflict*`):

| Element (`05`, 2g) | Today | In scope |
| --- | --- | --- |
| A right-side 920 px sheet | a centered 920 px dialog, one scrolling column | yes (decision 3) |
| Title, description | as designed | — |
| Meta line, progress amber then green | as designed | — |
| Card header: name · kind · state · choices | raw key, choices, then state; no kind | yes (B5) |
| Keep mine, Keep Jo's, Keep both | Keep mine, Keep theirs, Keep both | no names (decision 1) |
| Before \| Yours (name · time) \| Theirs (name · time) | Before \| Yours \| Theirs | no names or times (decision 1) |
| Tints and gutters | as designed, gutter in the text | — |
| One Result preview, live | a Result editor per body (D5 R-27) | no: the editor stays |
| Result note | D5 R-34's note | no |
| Footer: Git line with base · Cancel · **Save resolution** | Git line without base; **Save** · Cancel | order yes (decision 4); no base (decision 1) |
| Outside cards with their actions | one card per path; one shared row of actions | shared row stays (design R-14) |

## Entry Evidence And Dependency Boundary

| Gate | State at planning (`ac50ba1a`) |
| --- | --- |
| Cycle ticket before the plan (`AGENTS.md`) | `01M7D7JK5MSY6J471JEKG3CZXR`, open, created 2026-11-14 |
| Latest main merged | origin/main `ea6d899b`, merged (nothing new) |
| 05a's documents approved (ticket's planning gate) | Yes, 2026-11-14 (05a decision 21) |
| **05a merged to main (ticket's execution gate)** | **No: 05a is in execution (Tasks 0–3 done, on Cycle 04's unmerged branch). Task 0 does not start before 05a merges** |
| Cycle 04 merged | No: its ticket is done, not closed. 05a waits for it |
| GPUI Kit 0.6.6 names read | `Dialog` and `Sheet` slots, Root's layers and builders, `Spinner`, `IconName` (design facts 2, 20, 24) |
| Required checks at baseline | Not run during planning; Task 0 runs them |
| Testers | Linux (Wayland): the product owner. macOS and Windows: native CI after merge |

**Dependency boundary.** This Cycle changes `src/desktop/` and its tests
only, plus the documents in the design's Document Amendments. Nothing in
the library, `tests/` or the CLI; no Cargo dependency.

**In flight.** Cycle 05a's Tasks 4–10 build the Add repository sheet's
owner and finish the frame this design uses. The design cites 05a's
approved design for those parts, and Task 0 re-maps them against 05a's
merged code (design U-1).

## Scope

1. **The lifecycle dialog on the frame** (design Part A): a header per
   stage, step rows with their marks and notes, the confirm, running and
   ended bodies, the outcome box, and footers with the action last.
2. **Resolve conflict as a sheet** (Part B): a new sheet owner, every
   way in through the workspace, its focus contract, the header and
   footer fixed, cards with names, kinds and states.
3. **Copy** (Part C): the copy items from the D5 follow-ups ticket
   (decision 5) and the ellipsis rule (decision 12).

## Explicit Exclusions And Downstream Obligations

- **Out of scope:**
  - line counts, authors and times, a base commit, the key's name in a
    failure, progress counts (decision 1);
  - a ready list before the close (decision 2);
  - a whole-file Result preview (D5 R-27);
  - every other item of `01M6RAS63EWVHWWVBSZ836RXG2` (decision 5);
  - any library, repository-format or CLI change.
- **Follow-up ticket** (decision 1): `01M7FBRBN04QACSCZV6KX0WMR3`
  (`vh-nkp8c`), the library data for line counts, conflict authors and
  times, and the merge base, created and published 2026-11-15. The
  surfaces leave room for them; drawing them is that ticket's. Progress
  counts and the key's name stay known limits.
- **For later Cycles:** none.

## Required Contract

1. **Behaviour unchanged, with named exceptions.** Both surfaces send
   what they send today, refuse what they refuse today, and keep their
   Enter, Escape and focus return, except **(owner's eye)**:
   - the footer order and labels (decision 4);
   - the running stage's buttons and backdrop (decision 4);
   - Resolve conflict as a sheet (decision 3);
   - the copy (Part C, decisions 5, 7, 8, 10, 12).
2. **D5's behaviour holds** (Wave risk row): stale confirmations, Retry
   reusing the operation, the safe-point stop, no automatic winner. D5's
   tests pass, edited only as the named edits say.
3. **Resolve conflict is a sheet the workspace keeps,** opened only
   through the workspace, closed by Escape, its close button, Cancel,
   Resume, a save or quitting, never by its scrim (decision 9). A close
   that lands after it has gone closes nothing, and one that lands under
   a dialog waits for that dialog to close.
4. **Focus is trapped and returns,** as design B3's rows H1–H16 set out,
   and as today for the lifecycle dialog, the backdrop's hide included.
5. **Nothing acts behind either surface** (Wave 03 D3 R-60), and nothing
   opens Settings, Add repository or a details sheet over Resolve
   conflict; what waited for its dialog waits for the sheet.
6. **No secret.** No new text, tooltip, accessible name or Git line
   names a passphrase or private key.

## Acceptance And Exit Evidence

Planned proof. The Status column was filled at the final task (2026-11-15)
from the branch's own tests, by exact name, and the execution ledger.
"On screen" rows await the owner's checks.

| # | Wave exit evidence / contract | Proof | Status |
| --- | --- | --- | --- |
| A1 | A close and an approval through confirm, running, failed with Retry, and done, including a local-only repository, by keyboard (Wave exit) | `keyboard_tests::lifecycle::{close_a_ticket_by_keyboard, approve_and_merge_a_document_by_keyboard}`, edited only as named; P13 `a_failed_run_is_retried_by_keyboard` (both commands); P1 `each_stage_has_its_title`; P2 `each_stage_orders_its_footer` | Met. Each test green (Task 4 and 8 runs; fast tier). P13 seen failing first |
| A2 | The design's stages: steps with state and progress, the outcome box, the footers (scope 1) | P2; P3 `the_backdrop_hides_only_a_running_dialog`; P4 `steps_show_their_state_and_progress` | Met. P2, P3, P4 green, each seen failing first; Task 4 module run 143 passed |
| A3 | Resolving a conflict, by keyboard (Wave exit) | `keyboard_tests::conflict::{resolve_a_local_conflict_by_keyboard, resolve_a_shared_conflict_by_keyboard}` and `::sheet::*`, edited only as named | Met. The five `keyboard_tests::conflict::*` walks and the `::sheet::*` tests green (Task 8 run, fast tier); E-10, E-11, E-12 as named |
| A4 | Resolve conflict is a sheet the workspace keeps (contract 3) | P5 `resolve_is_a_sheet_the_workspace_keeps`; P7 `nothing_opens_over_the_resolve_sheet` | Met. P5 and P7 green; P7's 17 facts each seen failing or proved by mutation |
| A5 | Focus returned to the invoking control after each (Wave exit; contract 4) | P6 `focus_around_the_resolve_sheet` (H1–H16); P3; P13; `confirm_tests::enter_confirms_esc_cancels_and_focus_returns`; `action_tests::focus_returns_to_the_action_when_the_dialog_closes` | Met. P6 (H1 to H16; H13 reworded by Ruling O-9) green; P3, P13, `confirm_tests::enter_confirms_esc_cancels_and_focus_returns` and `action_tests::focus_returns_to_the_action_when_the_dialog_closes` green. A5's "focus returns to the invoking control" cannot hold after a close (the control is gone): Ruling O-6 |
| A6 | Parts named in words; Chosen and Edited; nothing acts behind (Part C; contract 5) | P8 `parts_are_named_in_words`; P7 `nothing_opens_over_the_resolve_sheet` | Met. P8 `parts_are_named_in_words` green, seen failing first (3 of 9 rows); P7 green |
| A7 | Copy (Part C) | P9 `close_and_resolve_copy_uses_the_vocabulary` | Met. P9 green |
| A8 | No passphrase or private key text reaches any label, tooltip or Git detail line (Wave exit; contract 6) | P12 `close_and_resolve_show_no_secret`; P10: the a16 privacy tests, unchanged | Met. P12 green, seen failing with a sentinel planted in a label (two plants); the a16 privacy tests (P10) green unchanged |
| A9 | D5's behaviour (contract 2) | P11: D5's stale, Retry, Stop, answer and review tests, unchanged in what they assert about behaviour | Met. D5's tests green, unchanged in what they assert about behavior (module runs for Tasks 1 to 8 and the fast tier: 1977 bin tests passed) |
| A10 | The surfaces match frames 2c, 2d, 2e and 2g as amended, in the dark theme | On screen: the owner | On screen: awaiting the owner (`2026-11-15-wave-04-cycle-05b-on-screen-checklist.md`) |

## Product-Owner Decisions (2026-11-15)

Wave 04 decisions and the Cycle 01–05a decisions govern and are not
repeated.

1. **Data the library lacks: leave it out, file a follow-up.** The
   owner chose "Leave out now, file a follow-up". Per-file line counts,
   author names and times for each side of a conflict, the base commit,
   the key's name in a failure and progress counts have no data behind
   them (design facts 25, 27, 28, 30). 05b shows what exists in their
   place: no counts, "Keep theirs" and "Theirs", a Git line with ours and
   theirs, today's message as the cause, and the stage's words as the
   running step's progress. One follow-up ticket,
   `01M7FBRBN04QACSCZV6KX0WMR3` (`vh-nkp8c`), asks the library for line
   counts, conflict authors and times, and the merge base; 05b does not
   wait for it. Progress counts and the key's name stay known limits.
   **(owner's eye)**
2. **The ready list after the close only.** The owner chose "After the
   close only". Confirm shows no "Becomes ready to start"; a closure that
   reached main, Done or Failed, shows "Now ready to start", the
   library's report, as D5 decision 6 and R-20 have it.
3. **Resolve conflict becomes a right-side sheet.** The owner chose
   "Right-side sheet". It is another user of the one sheet slot, with
   its own owner, beside the details sheet, Settings and Add repository.
   **(owner's eye)**
4. **The design's order, action last.** The owner chose "Design order,
   action last": Cancel · **Close ticket**; Not now · the fixes ·
   **Retry**; **Done**; Cancel · **Save resolution**. While a run is
   running, the only button is Stop at a safe point, with "Esc hides this
   window", and clicking the backdrop also hides the dialog, which is
   new. **(owner's eye)**
5. **From the D5 follow-ups, the copy items only.** The owner chose "The
   copy items only": one name for the way into Resolve conflict ("Review
   changes…"), "Keep mine" beside "Yours" made consistent with "Chosen"
   as the state, and shared label constants for Resume, Share main and
   Keep mine. Everything else stays on `01M6RAS63EWVHWWVBSZ836RXG2`.
6. **The outline** ("Write the documents"), with the constraints the
   planner stated before the questions, which the owner did not change:
   behaviour, Escape and the library's step order stay; the Result stays
   an editable box with D5's note; field names show as words with their
   kind; D5's extra states go on the same frame; desktop-only, executed
   after 05a merges.

Proposed by the planner, and confirmed by the owner at approval
(2026-11-15):

7. **Stage titles, the Waiting stage, and every footer** (design A1, A7):
   - titles "Close {short}?", "Closing…", "Still running…", "Ticket
     closed", "Couldn't finish closing", "Stopped at a safe point" (and
     the Approve and merge forms, "Merged into {primary}" when done);
   - an answer that another call still holds the item (`busy`) is its
     own stage, Waiting: "Still running…", Close and today's other
     buttons, no backdrop hide;
   - each phase's footer exactly as design A7's table lists it: the
     dismiss button first (Cancel while asking to confirm, Close for the
     other questions and for Waiting, Not now after a failure or stop,
     Done last when done), then today's other buttons in today's order,
     then today's primary buttons; while running, only Stop at a safe
     point, and after it is pressed, "Stopping…" as a Tab stop that does
     nothing, so Tab and Escape keep working;
   - today's "Done" or "Failed" word stays as the outcome box's first
     line (design R-17), the kind icon and branch line go, and the Git
     line is drawn on every stage that holds the preview.

   *Why:* `05` draws the confirm, running, failed and done titles and
   footers; the rest follow them. **(owner's eye)**
8. **"Closed by {name} · today"** (design A4, R-3), without the email.
   *Why:* `05` draws it so; the email adds nothing the person does not
   know. **(owner's eye)**
9. **Resolve conflict keeps today's rules where a sheet would differ**
   (design B2, B3): its scrim does not close it; Ctrl+, , a route into
   Settings and Cycle 05a's Add repository do nothing while it is open;
   a Republish… that waited for its dialog waits for the sheet; a focus
   move the workspace makes behind it (a draft discarded, say) leaves
   focus where it is, as under Settings. *Why:* each would lose the
   choices, cover them or pull focus out of them, and today's dialog
   stops all of them.
10. **Field names, kinds and states** (design Part C table, B5): "Status"
    for `status` and so on, an unknown key as written, as the details
    sheet names them; "Ticket detail", "Document detail" or "Comment
    detail" as a field's kind; the body stays "Description"; the state
    reads "Chosen", or "Edited" once the Result was typed in; the chosen
    button shows a check, so colour is not the only signal. *Why:* `05`
    names parts in words and says "Chosen"; "Edited" keeps what "Keeping
    the result as edited" said. **(owner's eye)**

After review round 1:

11. **Further test edits pre-authorized.** The owner chose "Pre-approve,
    as 05a did". An edit to an existing test that the named edits (design
    E-1 to E-15) do not list is numbered E-16 onward in the execution
    ledger with its reason, checked by the task's reviewer, and each
    assertion removal is named in the review-ready summary.
12. **The ellipsis rule, everywhere the labels appear.** The owner chose
    "Yes, everywhere they appear": "Open Sharing & keys…", "Open
    repository details…" and "Check host…", in the lifecycle dialog and
    in Activity, as `06` asks of an action that opens a sheet or
    confirmation. This changes Cycle 03b's Activity copy (design Document
    Amendment 5). **(owner's eye)**

At approval:

13. **Approved, and implementation authorized, subagent-driven and
    serial, in a fresh session.** The owner wrote: "Recommendations,
    test edits, implementation and documents are approved. Subagent
    driven development in a fresh session by a hand off brief and
    prompt." That is:
    - the three documents, revision 4, without a further review;
    - the planner's recommendations: decisions 7–10 confirmed, and the
      recommended answers the documents are written to;
    - the named test edits E-1 to E-15, with further edits under
      decision 11;
    - **authorized:** Tasks 0–9 of the plan, once Cycle 05a
      (`01M7D5Z7FZTGTC1HE25KQ1MYQY`) is closed and merged to main (the
      ticket's Entry Gate); pushing this ticket branch, and other
      branches where a comment on another ticket publishes them;
    - **not authorized** without asking the owner at the time: a pull
      request, merging into main, closing the ticket, worktree cleanup,
      native CI, or creating tickets;
    - the execution brief: `docs/plans/2026-11-15-wave-04-cycle-05b-handoff.md`.

## Known Limits

Design K-1 to K-21: no line counts; no ready list at confirm; progress as
stage words; the cause names no key; "today" assumed; no author names or
times; no base in the Git line; the Kit's selected style with a check;
the file count in parentheses; message-only states take the Ask title;
Resolve's scrim does nothing; shared outside actions; the library
follow-up; the body has no kind label; a run shown without its preview
has no Git line or step notes; Ctrl+Enter does nothing on the sheet's
close button; the Git line has no tooltip; a backdrop click in the frame
after a run ends; Tab to a part below the fold does not scroll it into
view, as today; the Kit's 100-stop focus trap; quitting closes the sheet
in the quitting window only.

## Open Questions

None blocking. Design U-1 (05a's merged names), U-6 (the named edits
against 05a's merged code), U-7 (the dialog-only guards and focus moves)
and U-9 (05a's own waiting openers) are checked in Task 0.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Owner, 2026-11-15 | Plan 05b now | Owner direction | Planning started; 05a's branch merged into this one (`ac50ba1a`), as the ticket's first planning step allows | Session, 2026-11-15 |
| Planning research, 2026-11-15 | The lifecycle dialog, Resolve conflict and the library's data at `ac50ba1a`; the sheet slot; the D5 follow-ups; the test sweep | Facts; owner questions; rulings | Design facts 1–30 (31–34 from review round 1); decisions 1–5; rulings | Research reports, 2026-11-15 |
| Owner, 2026-11-15 | Missing data; ready list; sheet or dialog; button order; D5 follow-ups; the outline | Owner decisions | Decisions 1–6 | Session, 2026-11-15 |
| Review round 1, source and feasibility (revision 1, `702e569f`) | 1 Blocking, 6 Important, 10 Minor. Blocking: a control gone from the Kit's footer slot leaves focus outside the sheet. Important: footer order unreadable; StillRunning in two stages; the Republish… scenario; Settings' focus check; P4's input; the Kit body's scroll | Blocking, Important, Minor | Revision 2 (design Review Record) | Review report, 2026-11-15 |
| Review round 1, contract and consistency (revision 1, `702e569f`) | 2 Blocking, 16 Important, 7 Minor. Blocking: no proof for the passphrase exit item; no failed-then-Retry keyboard journey. Important: the Git line in 2d and 2e; StillRunning; "Stopping…"; the footer order as an owner decision; the follow-up's scope; the ellipsis rule; colour alone for the choice; quit and late closes; "inside the sheet"; scrolling; contract 5; amendments; unlisted test edits; privacy order; Task 5's size; P3 | Blocking, Important, Minor; owner questions | Revision 2; decisions 7, 9, 10 revised; decisions 11 and 12 | Review report, 2026-11-15 |
| Owner, 2026-11-15, after review round 1 | Further test edits; the ellipsis rule | Owner decisions | Decisions 11 and 12 | Session, 2026-11-15 |
| Planner, 2026-11-15 | The follow-up library ticket (decision 1) | Owner direction | `01M7FBRBN04QACSCZV6KX0WMR3` (`vh-nkp8c`) created and published at `ecb44079` | Session, 2026-11-15 |
| Review round 2, scoped re-review (revision 2, `874bb66b`) | 1 Blocking, 4 Important, 8 Minor. Blocking: P13's fixture offers no Retry. Important: H1's Tab order; presses in the body moving focus; the scroll named reveals nothing; a task left red | Blocking, Important, Minor | Revision 3 (design Review Record) | Review report, 2026-11-15 |
| Review round 2, adversarial (revision 2, `aaf6b475`) | 1 Blocking, 4 Important, 9 Minor. Blocking: no Tab stop after Stop. Important: H1; an answer under a dialog; a second Republish… path; focus moves behind the sheet | Blocking, Important, Minor | Revision 3; decisions 7 and 9 revised | Review report, 2026-11-15 |
| Review round 3, scoped re-review (revision 3, `3964af7c`) | 0 Blocking, 2 Important, 7 Minor. Important: the privacy test's key never prompts; the lifecycle's way in under a prompt had no mechanism and lost Repair locally's late sheet | Important, Minor | Revision 4, **not independently reviewed** (design Review Record) | Review report, 2026-11-15 |
| Owner, 2026-11-15 | Approval message: documents (revision 4, no further review), recommendations, test edits, execution method, authorization | Owner decisions | Decisions 7–10 confirmed; decision 13; Document Amendments 2–5 applied | Session, 2026-11-15 |
