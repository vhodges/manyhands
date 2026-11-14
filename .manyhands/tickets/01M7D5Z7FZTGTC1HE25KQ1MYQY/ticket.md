---
slug: vh-kebv6
id: 01M7D5Z7FZTGTC1HE25KQ1MYQY
title: 'Wave 04 Cycle 05: Existing Sheets And Dialogs'
type: cycle
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Bring the sheets and dialogs that already exist up to the design, so that
the Wave leaves no surface in the old style. Each keeps its current
behavior, services, request handling and recovery routes. Only its frame,
layout, copy and states change.

Cycle 05 of [Wave 04](../../../docs/Waves/wave-04-workspace-design.md#cycle-05-existing-sheets-and-dialogs).
The Wave document owns scope, exclusions and exit evidence. The design is
`docs/design/`: `05` for the common frame, Add repository (1d), Unlock key
(2h), Close ticket and Approve and merge (2c to 2e) and Resolve conflict
(2g); `04` for the empty pane; `06`, `07` and `08`.

Planned while Cycle 04 (`01M7AMAPNHHTNECXBF4CA5WPKV`) is in flight. This
ticket does not authorize implementation.

## Planning

- [ ] Merge the latest main into this ticket branch.
- [ ] The Wave's "Check first": compare Wave 03 D5's lifecycle dialog and
  resolve conflict sheet with `05` and its screenshots, and record what
  matches and what differs. Only the differences are in scope.
- [ ] List every existing dialog and sheet the common frame applies to.
- [ ] Create `docs/Cycles/wave-04-cycle-05-sheets-and-dialogs.md`.
- [ ] Carry Cycle 04's downstream obligations for the Add repository
  sheet (its Cycle document, "For Cycle 05").
- [ ] Decide with the owner what to take from
  `01M5F3GVDYTH1Q5V5FCS9YF7V3` (its Add repository items, which Cycle 04
  decision 3 left for this Cycle) and from `01M6RAS63EWVHWWVBSZ836RXG2`
  (D5 follow-ups).
- [ ] Decide with the owner whether the Wave's split line (05a, 05b) is
  needed.
- [ ] Create the design and implementation plan and record their
  checkpoints as ticket comments.

## Entry Gate

- **Planning:** Cycle 01 merged. Cycle 04's planning approved
  (`ded8402f` on its branch).
- **Execution:** Cycle 04 merged to main.

## Owner Decisions, 2026-11-14

1. Plan this Cycle now, while Cycle 04 is in flight. Execution waits
   until 04 lands on main.

## Scope Summary

- The common sheet and dialog frame from `05`: header, body, footer,
  primary and secondary buttons, 32 px inputs with the teal focus ring,
  scrims, and focus that is trapped, closes on Esc unless an operation is
  running, and returns to the invoking control. It applies to every
  existing dialog, including the confirmation dialog, New ticket…, New
  document…, New folder…, Move…, the close-draft dialog and the quit
  dialogs.
- The Add repository sheet (1d, 470 px), moved out of the Settings
  sheet's Repositories page, opened from the Repositories panel's `+`,
  from Settings (replacing the Settings sheet until Cancel or Add, Wave
  decision 10) and from the empty Repositories panel.
- The Unlock key dialog (2h, 440 px), with Not now moving the prompt into
  Needs attention and no show and hide toggle (Wave decision 9).
- The Close ticket and Approve and merge dialog (2c to 2e, 580 px), where
  the check finds differences.
- The Resolve conflict sheet (2g, 920 px), moved onto the shared tokens
  and frame, with only the differences the check finds.
- The empty item area's faded app icon.
- Out of scope: new behavior in any of these surfaces, the General and
  Appearance settings pages, and theme switching.

## Testing

Desktop-only, so the fast tier at review-ready and no SSH target
(`AGENTS.md`). Behavior, not appearance: focus trapping and return, Esc
and when it is refused, each surface's states and enablement, and what
each service receives. Wave 03 D5's existing tests stay and must pass
unchanged in what they assert about behavior.

## Exit Evidence

As the Wave states: every existing journey through these surfaces still
passes by keyboard: adding and creating a repository, including the
identity step and each failed check's recovery action; unlocking, Not
now, a wrong passphrase; a close and an approval through confirm,
running, failed with Retry, and done, including a local-only repository;
resolving a conflict. Focus returns to the invoking control after each.
No passphrase or private key text reaches any label, tooltip or Git
detail line.
