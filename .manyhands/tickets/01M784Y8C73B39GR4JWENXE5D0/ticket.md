---
slug: vh-3jpbf
id: 01M784Y8C73B39GR4JWENXE5D0
title: 'Wave 04 Cycle 03a: Repositories Panel And Sharing Status'
type: cycle
status: done
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Rebuild the Repositories panel to `docs/design/03-panels.md` and add the
footer's sharing and update-check status, so a person sees each
repository's sharing state in plain language.

Cycle 03a of [Wave 04](../../../docs/Waves/wave-04-workspace-design.md#cycle-03-repositories-activity-and-needs-attention).
The owner split Cycle 03 in two on 2026-11-12: 03a (this ticket) is the
Repositories panel and the footer's "to share" and "Checked for updates"
items; 03b (`01M784Y94Q3AXT6XJX38F5VK1V`) is Activity and the Needs
attention popover. The Wave document owns scope, exclusions and exit
evidence, amended by the split. The design is `docs/design/` (`03`, with
`02` "Footer", `06`, `07` and `08`, and the frames in
`docs/design/screenshots/`).

## Planning

- [x] Merge the latest main into this ticket branch. Created from
  origin/main `37be101d`; a merge on 2026-11-12 found nothing new.
- [x] Create `docs/Cycles/wave-04-cycle-03a-repositories.md`.
- [x] Verify every GPUI Kit 0.6.6 name the plan uses (context menu,
  tooltip, popup menu, list) against the locked source. In the design's
  Baseline Facts, checked in review round 1: the Kit list overwrites a
  row's selected flag with its cursor, so the rows are the panel's own
  element; the menu is a Workspace popup like the dock menu.
- [x] Map each repository action and state the desktop has today to its
  place in the rebuilt panel. In the design's Part F table and Parts B
  and D.
- [x] Create the design and implementation plan and record their
  checkpoints as ticket comments. Drafted at `f6d57bd4`; review round 1
  (two reviewers) applied in revision 2 (`2a0f202e`) with the owner's
  second-round answers; round 2 (a scoped re-review and an adversarial
  review) applied in revision 3 (`b3851766`) with decision 24; round 3
  (a scoped re-review) applied in revision 4 (`542a3824`), which had no
  independent review. Planning checkpoint comment added; awaiting the
  owner's approval.

## Entry Gate

Cycle 01 merged (main `eb0798a2`). Cycle 02 merged (main `37be101d`,
ticket `01M757DHFAVPEVY4TWQTVKQAMJ`). Wave 03 D6 merged (main `48653d01`),
so its checking states are in scope.

## Owner Decisions, 2026-11-12

The Cycle document records every decision. In short:

1. Cycle 03 is split into 03a and 03b.
2. "N to share" comes from D6's latest check: the items it found ahead of,
   or not yet on, the shared copy. No library change.
3. This Cycle absorbs `01M654G5VGQMXTH3BX9GKE7JEY` (vh-d78zh, move the
   repository details out of the item tab) and, from
   `01M5CS4MH25QQ769HV61ZRKHXG`, the repository row's double selected
   signal and `RepositoriesPane::set` notifying during render.
4. Four more state labels: Can't open, Needs fixing, Locked, Needs
   attention.
5. The details go to the row and its tooltip; the index state stays with
   the lists.
6. The footer's "N to share" is the total; each activation chooses the
   next repository with work.
7. "N to share" is a reminder; sharing stays where it is today.
19. A diverged item counts as "to share"; other problems a check lists
    show Needs attention.
20. A non-SSH publication remote shows no label, explains why, and offers
    Set up sharing….
21. The menu's first line is the state's sentence, for keyboard users.
22. A chosen repository the list no longer gives stays as a Can't open
    row with Retry and Remove.
24. A save in this app counts in "N to share" at once, as a share
    clears it at once.

Decisions 8–18 and 23, the planner's proposals, were confirmed by the
owner on 2026-11-12.

## Scope Summary

- Toolbar: Add or create a repository (today's flow), Check all for updates
  now (D6's Check now per repository), More with Refresh, Rebuild the
  search index and Remove from Manyhands….
- Rows: `folder-git-2`, name and the right-aligned state: "N to share",
  "Up to date", "This computer", "Can't reach", "Checking…", "Paused".
  Git detail (path, main branch, remote, counts) in the tooltip. Row
  anatomy and list states from Cycle 02 where they fit.
- Context menu, by right click and by Shift F10 or the menu key: the
  state's sentence, its action, Check now, Share main, Open folder,
  Settings…, Remove from Manyhands… with its confirmation saying the files
  stay.
- The repository details leave the item area; their actions and facts move
  to the panel.
- Footer: "N to share", selecting the repository with unshared work, and
  "Checked for updates …".
- "Can't reach" survives a relaunch from the library's stored last outcome.

## Testing

Desktop-only, so the fast tier at review-ready and no SSH target
(`AGENTS.md`). Behavior, not appearance: each state from real service
state, actions and their enablement, what each action submits, keyboard
reach, and that no secret or credential reaches a row, tooltip or footer
item (extend the privacy tests to tooltips).

## Exit Evidence

Each repository state comes from real service state, with no fabricated
rows; every action works by keyboard; no secret appears in a row, tooltip
or footer item; every existing repository journey (select, Share main,
Retry, Remove, Add) still passes.
