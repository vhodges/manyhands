---
slug: vh-mhs1r
id: 01M7AMAPNHHTNECXBF4CA5WPKV
title: 'Wave 04 Cycle 04: Settings Sheet'
type: cycle
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Move Settings out of the item area into the design's sheet, so opening it
no longer hides the open item.

Cycle 04 of [Wave 04](../../../docs/Waves/wave-04-workspace-design.md#cycle-04-settings-sheet).
The Wave document owns scope, exclusions and exit evidence. The design is
`docs/design/` (`05` for the sheet frame and Sharing & keys, `06` for the
vocabulary, `07` and `08`).

Planned while Cycle 03b (`01M784Y94Q3AXT6XJX38F5VK1V`) is in
implementation. This ticket does not authorize implementation.

## Planning

- [ ] Merge the latest main into this ticket branch.
- [ ] Create `docs/Cycles/wave-04-cycle-04-settings-sheet.md`.
- [ ] Map every existing route into Settings, including those Cycle 03b's
  Activity and Needs attention rows add, to a page and field of the sheet.
- [ ] Decide with the owner what to take from
  `01M5F3GVDYTH1Q5V5FCS9YF7V3` (settings UX from the D1 reviews).
- [ ] Create the design and implementation plan and record their
  checkpoints as ticket comments.

## Entry Gate

- **Planning:** Cycle 01 merged. Wave 03 D6 merged (main `48653d01`).
- **Execution:** Cycle 03b merged to main (owner, 2026-11-13).

## Owner Decisions, 2026-11-13

1. Plan this Cycle now, while Cycle 03b is in implementation. Execution
   waits until 03b lands on main.

Decisions 2–13 are recorded in the Cycle document
(`docs/Cycles/wave-04-cycle-04-settings-sheet.md`).

## Scope Summary

- The sheet frame from `05`: right side, 720 px, a header with "Applies to
  this computer only.", a scrolling body and no footer (decision 4), and a
  214 px left nav. It opens from the footer's Settings button
  and Ctrl ,. Focus is trapped, Esc closes it, and focus returns to the
  control that opened it.
- Today's pages folded into the design's nav:
  - **Repositories:** today's Repositories and Remotes pages, with the
    identity form;
  - **Sharing & keys:** today's Shared SSH keys page, plus Trusted servers
    from today's Hosts page, in `05`'s layout;
  - **Checking for updates.**
- Behavior unchanged; copy moves to the `06` vocabulary.
- Every existing route into Settings opens the sheet at the right page and
  field.
- Settings removed from the content region and from the sidebar.
- Out of scope: the General and Appearance pages, and new settings.

## Testing

Desktop-only, so the fast tier at review-ready and no SSH target
(`AGENTS.md`). Behavior, not appearance: which page and field each route
opens, focus trapping and return, Esc, every settings journey by keyboard,
and that the open item, its scroll position and any draft are untouched.

## Exit Evidence

As the Wave states: every existing settings journey and recovery route
passes through the sheet by keyboard; opening and closing Settings leaves
the open item, its scroll position and any draft untouched; opening
Settings while an item details sheet is open closes that sheet first and
loses no edit (Wave decision 6).
