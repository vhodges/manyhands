---
slug: vh-kebv6
id: 01M7D5Z7FZTGTC1HE25KQ1MYQY
title: 'Wave 04 Cycle 05a: Common Frame, Add Repository And Unlock Key'
type: cycle
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Bring the existing dialogs and sheets onto the design's common frame, move
Add repository into its own sheet, and restyle the Unlock key dialog and
the empty item area, so that these surfaces leave the old style. Each keeps
its current behavior, services, request handling and recovery routes, with
the exceptions the Cycle document names.

Cycle 05a of [Wave 04](../../../docs/Waves/wave-04-workspace-design.md#cycle-05-existing-sheets-and-dialogs).
The owner split Cycle 05 in two on 2026-11-14 (decision 2): 05a (this
ticket) is the common frame, the Add repository sheet, the Unlock key
dialog and the empty item area's icon; 05b (`01M7D7JK5MSY6J471JEKG3CZXR`)
is the Close ticket and Approve and merge dialog and the Resolve conflict
sheet. The Wave document owns scope, exclusions and exit evidence, amended
by the split. The design is `docs/design/` (`05` for the frames, Add
repository (1d) and Unlock key (2h); `04` for the empty pane; `06`, `07`
and `08`).

Planned while Cycle 04 (`01M7AMAPNHHTNECXBF4CA5WPKV`) is in flight. This
ticket does not authorize implementation.

## Planning

- [x] Merge the latest main into this ticket branch (created from
  origin/main `ea6d899b`; main has not moved).
- [x] The Wave's "Check first": compare Wave 03 D5's lifecycle dialog and
  resolve conflict sheet with `05` and its screenshots. Neither matches;
  the check is recorded for 05b (decision 2).
- [x] List every existing dialog and sheet the common frame applies to.
- [x] Decide with the owner whether the Wave's split line is needed
  (decision 2).
- [x] Create `docs/Cycles/wave-04-cycle-05a-frame-add-repository-and-unlock.md`.
- [x] Carry Cycle 04's downstream obligations for the Add repository
  sheet (its Cycle document, "For Cycle 05").
- [x] Create the design and implementation plan and record their
  checkpoints as ticket comments.

## Entry Gate

- **Planning:** Cycle 01 merged. Cycle 04's planning approved
  (`ded8402f` on its branch).
- **Execution:** Cycle 04 merged to main.

## Owner Decisions, 2026-11-14

1. Plan this Cycle now, while Cycle 04 is in flight. Execution waits
   until 04 lands on main.
2. Split Cycle 05 on the Wave's split line. This ticket is 05a; 05b is
   `01M7D7JK5MSY6J471JEKG3CZXR`, planned after 05a's documents are
   approved.
3. Add repository's "Share changes through" select: the sheet lists the
   folder's remotes (non-SSH ones disabled, with the reason) and "Keep on
   this computer"; after a successful add it sends today's remote
   selection. If that fails, the repository stays added and the failure
   shows in Activity.
4. A wrong passphrase gets no inline error in this Cycle: the library
   cannot tell it apart (`01M5E6DYJ6GN7DMW4424TH3V78`). The dialog's inline
   error covers the passphrases it refuses itself.
5. The outline and its constraints: "Write the documents".

Decisions 6–20 are recorded in the Cycle document
(`docs/Cycles/wave-04-cycle-05a-frame-add-repository-and-unlock.md`):
15–18 answered by the owner after review round 1; 6–14, 19 and 20 the
planner's proposals.

## Scope Summary

- The common sheet and dialog frame from `05`, applied to every existing
  dialog except the lifecycle dialog and the resolve conflict sheet (05b),
  and to the details sheet's header.
- The Add repository sheet (1d, 470 px), moved out of the Settings sheet's
  Repositories page, opened from the Repositories panel's `+`, from the
  empty Repositories panel, from Settings (replacing it until Cancel or
  Add, Wave decision 10) and from the inspect routes.
- The Unlock key dialog (2h, 440 px).
- The empty item area's faded app icon.
- Out of scope: new behavior beyond the named exceptions, the General and
  Appearance settings pages, theme switching, and any library change.

## Testing

Desktop-only, so the fast tier at review-ready and no SSH target
(`AGENTS.md`). Behavior, not appearance: focus trapping and return, Esc,
each surface's states and enablement, and what each service receives.

## Exit Evidence

As the Wave states, for 05a's surfaces: every existing journey through
them still passes by keyboard: adding and creating a repository,
including the identity step and each failed check's recovery action;
unlocking, Not now, a refused passphrase. Focus returns to the invoking
control after each. No passphrase or private key text reaches any label,
tooltip or Git detail line.
