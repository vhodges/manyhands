---
slug: vh-dt0aq
id: 01M784Y94Q3AXT6XJX38F5VK1V
title: 'Wave 04 Cycle 03b: Activity And Needs Attention'
type: cycle
status: done
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Rebuild Activity as the design's single log and add the Needs attention
popover, so failures and recovery states are visible in plain language
where the design puts them.

Cycle 03b of [Wave 04](../../../docs/Waves/wave-04-workspace-design.md#cycle-03-repositories-activity-and-needs-attention).
The owner split Cycle 03 in two on 2026-11-12: 03a
(`01M784Y8C73B39GR4JWENXE5D0`) is the Repositories panel and the footer's
status items; 03b (this ticket) is Activity and the Needs attention
popover. The Wave document owns scope, exclusions and exit evidence,
amended by the split. The design is `docs/design/` (`03` "Activity", `05`
"Needs attention popover", `06`, `07` and `08`, frames 1a and 2f).

Planned on 2026-11-12 and 2026-11-13 (Cycle document, decision 4). Planning
does not authorize implementation.

## Planning

- [x] Merge the latest main into this ticket branch (Cycle 03a's published
  branch at `b37d2c2b`, decision 4; main with 03a merged at `81789331`).
- [x] Create `docs/Cycles/wave-04-cycle-03b-activity-and-needs-attention.md`.
- [x] Map every row and action in today's operations area to the new log
  (design B0).
- [x] Create the design and implementation plan and record their
  checkpoints as ticket comments.

## Entry Gate

Cycle 03a merged. Wave 03 D6 merged (main `48653d01`).

## Owner Decisions, 2026-11-12

1. Cycle 03 is split into 03a and 03b.
2. Activity becomes one timed log, newest first, over today's stores: each
   row has the time the app learned of it (this session), a plain-language
   message, a repository tag and one action, with the technical detail in
   its tooltip; failures stay until resolved; the "Show only things that
   need attention" filter.
3. This Cycle absorbs, from `01M6H7VXJJHRYHTW9S8TJ88921` and
   `01M6RAS63EWVHWWVBSZ836RXG2`, splitting `model/operations.rs` and
   `view/operations.rs` and classifying shares once instead of on every
   render; and, from `01M5CS4MH25QQ769HV61ZRKHXG`, "Finishing" hiding the
   running stage.

## Owner Decisions, 2026-11-12 and 2026-11-13 (planning this Cycle)

Recorded in full in the Cycle document; summarized here.

4. Plan against Cycle 03a's published branch, merged in; implementation
   waits for 03a to close.
5. States leave Activity when resolved; events stay as history.
6. The Needs attention count is the popover's rows, each thing once,
   including Cycle 03a's repository states; "Nothing to do" stops counting.
7. Checks get a row when they brought something in or listed problems.
8. Extra actions go in a row menu; risky actions are never the button.
9. Dismiss in the row menu, and "Clear finished" in ⋯.
10. A check's problem items are one row each, with Open.
11. Enter presses the highlighted row's button.
12. Titles allowed in Activity and the popover; bodies, comments and
    conflict sides never (D5's rule amended).
13. The outline's calls: no cap; a hold over several repositories is one
    row; every request kind gets a row.
21. D6's title rule is amended the same way; no paths in rows.
22. Rows are one line; D6's decision 13 amended.
23. After a dialog, focus returns to the row, highlighted.
24. At most one action per row; repeats about one operation show once in
    the popover.
25. A fixed failure stops counting by itself; its row stays as history.

Decisions 14–20 are the planner's proposals, awaiting confirmation.

## Scope Summary

- Activity rows: time, colored icon by kind, message in the `06` register,
  repository tag, at most one action; Git detail in the tooltip; newest
  first; failures kept until resolved; today's finishing, session and
  pending content carried into these rows, not dropped.
- The three cramped rows of Cycle 01's K-19 (draft-flush Retry, pending
  repository Retry, pending controls such as Abandon), rewrapped.
- Needs attention popover (`05`, 2f), anchored above the footer item: one
  row per unresolved recovery state with its single action, and "Show
  everything in Activity". A filtered view of the same rows as Activity.

## Testing

Desktop-only, so the fast tier at review-ready and no SSH target
(`AGENTS.md`). Behavior, not appearance: which rows exist and in what
order, what persists until resolved, the filter, every action by keyboard,
the popover with Activity hidden, and that no secret reaches a row,
tooltip or the popover.

## Exit Evidence

As the Wave states: failures persist until resolved and appear in both
Activity and the popover; every popover action works by keyboard; Activity
can be hidden and the popover still works; no secret appears in a row,
tooltip or popover.
