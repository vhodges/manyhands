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

- [x] Merge the latest main into this ticket branch.
- [x] Create `docs/Cycles/wave-04-cycle-01-workspace-shell.md`.
- [x] Verify every GPUI Kit 0.6.6 name the plan uses (`DockArea`,
  `DockAreaState`, `TitleBar`, `StatusBar`, `Menu`, resizable groups).
  Done in the design's Baseline Facts 1–19, read from the locked source and
  checked by four review rounds. The handoff's `DockItem`, `StackPanel` and
  `TabPanel` types do not exist in 0.6.6; `DockLayout`, `DockSkin`,
  `DockAreaRenderer` and `TabGroupRenderer` replace them.
- [x] Create the design and implementation plan and record their checkpoints
  as ticket comments.
- [x] Decide at plan time whether the Wave's split line (01a / 01b) applies.
  No: one Cycle (owner, 2026-11-08; Cycle decision 1).

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
- Default layout 1b without Projects and Teams. Rows stay as they are: 28 px
  is Cycle 02's row default (Cycle decision 11).
- Footer: panel and dock toggles, a dock menu per panel toggle (left, right,
  bottom), Needs attention and Settings.
- Layout persisted in app data, versioned, with a reported fallback.
- Keyboard: Ctrl B, Ctrl Alt B, Ctrl J; no per-panel keys; `secondary-1` to
  `secondary-4` also show a hidden panel; F6 cycles visible panels only.

## D6 Coordination

Wave 03 D6 (`01M4CC0VN3VPVVQ3RNWJKXWEVX`) is planned but not implemented,
and will probably run alongside this Cycle. Its plan (revision 1, drafted
2026-11-07) touches the operations area, the quit path and
`view/workspace/repositories.rs`.

- **Quit feedback stays in Activity.** The quit path's "Finishing {label}"
  section and Quit now render in the operations area today (`render.rs`),
  and D6 adds "still stopping" there after ten seconds. A quit that has to
  wait changes no layout: its feedback is in the Activity panel, visible
  when Activity is shown, and a person who wonders why opens Activity
  (Cycle decision 7, revised). Tested: a waiting quit with Activity hidden.
- **One source for Needs attention.** The footer's count comes from one
  new read-only function, `needs_attention` in `model/attention.rs`, over
  the operations and drafts models (`model/operations.rs` has no attention
  entries and is not edited). The states D6 adds (key locked, host
  approval, recovery hold) join that function in whichever of D6 and this
  Cycle merges second (Cycle decision 3).
- **Wrap, don't edit.** Move the operations area into `ActivityPanel` as it
  is, without changing `view/operations/` or `model/operations.rs`, so D6's
  edits there merge cleanly.
- Expect small textual conflicts in `view/workspace/repositories.rs`, the
  `Workspace` struct and constructor, and shared test harness and keyboard
  tests. Whichever Cycle merges second merges main and adapts.

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
