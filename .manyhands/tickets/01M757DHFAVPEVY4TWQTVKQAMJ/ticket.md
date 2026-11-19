---
slug: vh-r2n89
id: 01M757DHFAVPEVY4TWQTVKQAMJ
title: 'Wave 04 Cycle 02: Tickets And Documents Panels'
type: cycle
status: done
project: manyhands
team: core
closed_at: 2026-11-12T14:13:43Z
closed_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: ticket
---
Rebuild the two lists people use most, Tickets and Documents, to
`docs/design/03-panels.md`.

Cycle 02 of [Wave 04](../../../docs/Waves/wave-04-workspace-design.md#cycle-02-tickets-and-documents-panels).
The Wave document owns this Cycle's scope, exclusions, owner decisions and
exit evidence. The design is `docs/design/` (`03`, with `06`, `07` and
`08`, and the frames in `docs/design/screenshots/`).

## Planning

- [x] Merge the latest main into this ticket branch. Created from
  origin/main `48653d01`, which it still holds.
- [x] Create `docs/Cycles/wave-04-cycle-02-tickets-and-documents.md`.
- [x] Verify every GPUI Kit 0.6.6 name the plan uses (virtualized list or
  tree, segmented control, input, menu, skeleton, tooltip) against the
  locked source. Done in the design's Baseline Facts 1–12, checked in
  review round 1: the Kit `List` and base `Tree` with app rows, not
  `DataTable` or the styled `tree()`.
- [x] Map each Wave 03 list behavior to its place in the rebuilt panels:
  Ready and Blocked, Why blocked, duplicate short codes, nonconforming and
  inaccessible entries, Find, New ticket… and New document…, New folder…
  and Move…. In the design's Parts B–I and decisions 2, 9, 10 and 19.
- [x] Create the design and implementation plan and record their
  checkpoints as ticket comments. Revision 2, decisions confirmed
  2026-11-11; review round 2 pending.

## Entry Gate

Cycle 01 merged (main `eb0798a2`, ticket `01M6V8RK8B0SBCH0YZXFVSR9E3`
closed 2026-11-10). Wave 03 D6 merged (main `48653d01`). No other Wave 04
dependency.

## Scope Summary

- Common row anatomy: 28 px rows, hover, the selected treatment with the
  amber left line and 600 weight, visible keyboard focus. The four states:
  loading skeleton rows, empty with its primary action, failed or stale
  scan with Retry, normal. Lists stay virtualized and keyboard navigable.
- Tickets: toolbar (New ticket, More); the `Open | Closed | All` segmented
  control with counts, Open the default; "Filter by title or code"; Ready
  and Blocked in a filter menu beside the segmented control (Wave decision
  5); sortable column header (type, code, title, status, updated; default
  updated, newest first); type icons; status pills by the `03` table,
  unknown values neutral; editing and conflict markers; nonconforming rows
  kept visible with "needs fixing"; closed titles in `secondary`; the
  footnote `{n} open tickets in {repository}`.
- Documents: toolbar (New document, New folder, Collapse all); tree rows
  with depth indent, chevrons, folder and file icons, collapsed descendant
  counts, the document title as the label and the path in the tooltip;
  editing marker; nonconforming rows with the amber label and "Not a
  managed document yet. Open to adopt it."; empty folders with "Shared once
  it contains a document."; Left and Right collapse and expand.
- Repository scope only: the Documents tree shows the selected
  repository's tree directly.

## Testing

Desktop-only, so the fast tier at review-ready and no SSH target
(`AGENTS.md`). Behavior, not appearance: sort, filter and counts, the four
states, keyboard navigation and opening, nonconforming, inaccessible and
duplicate-code rows. No tests for colors, sizes, icons or that a view
renders; appearance tests broken by the deliberate visual change are
deleted.

## Exit Evidence

As the Wave states: PRD `MH-CONTENT-001/003/004` list behaviors still
pass; nonconforming and inaccessible items stay visible with their reason;
Open is the default; sort, filter and the four states are covered by model
tests; duplicate short codes list every match; keyboard-only navigation and
opening work in both panels.
