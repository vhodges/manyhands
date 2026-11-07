---
cycle: '01'
wave: '04'
slug: vh-jge24
id: 01M6V8RK8B0SBCH0YZXFVSR9E3
title: 'Wave 04 Cycle 01: Workspace Shell'
type: cycle
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Replace the fixed layout with the design's window anatomy, using today's
region contents unchanged inside the new frame.

Cycle 01 of [Wave 04](../../../docs/Waves/wave-04-workspace-design.md#cycle-01-workspace-shell).
The Wave document owns this Cycle's scope, exclusions, owner decisions and
exit evidence. The design is `docs/design/` (`01`, `02`, `07`, `08`).

## Planning

- [ ] Merge the latest main into this ticket branch.
- [ ] Create `docs/Cycles/wave-04-cycle-01-workspace-shell.md`.
- [ ] Verify every GPUI Kit 0.6.6 name the plan uses (`DockArea`,
  `DockAreaState`, `TitleBar`, `StatusBar`, `Menu`, resizable groups).
- [ ] Create the design and implementation plan and record their checkpoints
  as ticket comments.
- [ ] Decide at plan time whether the Wave's split line (01a / 01b) applies.

## Entry Gate

Wave 04 approved and the desktop IA RFC amended on 2026-11-07 (main
`846af679`). No Wave 04 dependency. Wave 03 D6 is not required; this Cycle
moves the operations area into the Activity panel unchanged.

## Scope Summary

- Chrome tokens for both modes, absorbing `accent` and `faint` from
  `src/desktop/view/item/tokens.rs`.
- Title bar (Kit `TitleBar` on every platform) and the window title
  `Manyhands — {repository}`.
- `DockArea` with the item area as its center; Root's sheet layer stays in
  the center (R-60).
- Panels `RepositoriesPanel`, `TicketsPanel`, `DocumentsPanel` (today's
  Navigation pane split in two) and `ActivityPanel`, with the design's panel
  headers.
- Default layout 1b without Projects and Teams, 28 px rows.
- Footer: panel and dock toggles, a dock menu per panel toggle (left, right,
  bottom), Needs attention and Settings.
- Layout persisted in app data, versioned, with a reported fallback.
- Keyboard: Ctrl B, Ctrl Alt B, Ctrl J; no per-panel keys; `secondary-1` to
  `secondary-4` also show a hidden panel; F6 cycles visible panels only.

## Testing

Desktop-only, so the fast tier at review-ready and no SSH target
(`AGENTS.md`). Behavior, not appearance: visibility rules, menu moves,
restore fallback (one table-driven test), title derivation, keyboard reach.

## Exit Evidence

As the Wave states: default layout at start; each panel hidden, shown and
moved from the footer by mouse and keyboard; docks and groups resize; the
layout survives a restart; a corrupt or old layout file restores the
default; the window title follows the selected repository; every existing
keyboard journey still passes. Native title-bar check on Linux Wayland, with
macOS and Windows after merge.
