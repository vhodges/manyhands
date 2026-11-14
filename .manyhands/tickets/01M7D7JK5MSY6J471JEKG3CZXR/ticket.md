---
slug: vh-qjw5e
deps:
- 01M7D5Z7FZTGTC1HE25KQ1MYQY
id: 01M7D7JK5MSY6J471JEKG3CZXR
title: 'Wave 04 Cycle 05b: Close Ticket And Resolve Conflict'
type: cycle
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Bring the Close ticket and Approve and merge dialog and the Resolve
conflict sheet, both built in Wave 03 D5, up to the design and onto the
common frame Cycle 05a builds. Each keeps its current behavior, services,
request handling and recovery routes. Only its frame, layout, copy and
states change.

Cycle 05b of [Wave 04](../../../docs/Waves/wave-04-workspace-design.md#cycle-05-existing-sheets-and-dialogs).
The owner split Cycle 05 in two on 2026-11-14 (05a decision 2): 05a
(`01M7D5Z7FZTGTC1HE25KQ1MYQY`) is the common frame, the Add repository
sheet, the Unlock key dialog and the empty item area's icon; 05b (this
ticket) is the two D5 surfaces. The Wave document owns scope, exclusions
and exit evidence, amended by the split. The design is `docs/design/05`,
"Close ticket dialog (2c → 2d → 2e)" and "Resolve conflict sheet (2g)",
with `06`, `07` and `08`.

Planned after 05a's documents are approved (05a decision 2). This ticket
does not authorize planning work beyond that, or implementation.

## The Wave's "Check first", 2026-11-14

Read-only comparison of the code at main `ea6d899b` with `05` and frames
2c, 2d, 2e and 2g. Neither surface matches. The 05b Cycle document records
it element by element; in short:

- **Close ticket and Approve and merge** (`src/desktop/view/lifecycle*`):
  a Kit `Dialog`, 580 px. Matches: the width, the bordered numbered step
  list, the confirm lead, the code note, the local-only step 2 wording.
  Differs: no header icon tile and one title for every stage; the step
  order (D5 R-8: Mark as closed before Merge) and the extra Share main
  step; step and Git line wording; running shows state words, not icons,
  and the lead sits below the steps; failed has no red step, cause under
  the step or outcome box; done reads "Done" and "Close", not "Ticket
  closed" and "Done"; buttons come primary first and left-aligned; no
  "Enter to confirm · Esc to cancel" hint; the running stage also offers
  Close. Has no data behind it: per-file line counts (D5 R-3), a "Becomes
  ready to start" list before the close (D5 decision 6), and progress
  counts.
- **Resolve conflict** (`src/desktop/view/conflict*`): a centered 920 px
  Kit `Dialog`, not a right-side sheet; one scrolling column, so the
  buttons scroll with the cards. Matches: the title, the progress
  wording and colours, the three equal columns, the tints and gutters,
  Save enabled only when every part is chosen. Differs: the frame, the
  card structure and header order, field names shown as raw keys, no
  part kind, Keep theirs for Keep {name}'s, no names or times, a per-body
  editable Result instead of one preview (D5 R-27), the Result note (D5
  R-34), no `base` in the Git line (R-27), the button order, and the
  noncanonical actions shared rather than per card.

## Planning

- [ ] Merge the latest main (with 05a merged, or 05a's approved branch)
  into this ticket branch.
- [ ] Create `docs/Cycles/wave-04-cycle-05b-close-and-resolve.md`, with
  the check in full.
- [ ] Decide with the owner what to do where the design shows data the
  library lacks, and whether Resolve conflict becomes a right-side sheet.
- [ ] Decide with the owner what to take from
  `01M6RAS63EWVHWWVBSZ836RXG2` (D5 follow-ups: copy and test harness).
- [ ] Create the design and implementation plan and record their
  checkpoints as ticket comments.

## Entry Gate

- **Planning:** 05a's documents approved.
- **Execution:** 05a merged to main.

## Testing

Desktop-only, so the fast tier at review-ready and no SSH target
(`AGENTS.md`). D5's existing tests stay and must pass unchanged in what
they assert about behavior: stale confirmations, Retry reusing the
operation, the safe-point stop, no automatic winner.

## Exit Evidence

As the Wave states, for 05b's surfaces: a close and an approval through
confirm, running, failed with Retry, and done, including a local-only
repository; resolving a conflict. Every journey by keyboard, with focus
returned to the invoking control. No passphrase or private key text
reaches any label, tooltip or Git detail line.
