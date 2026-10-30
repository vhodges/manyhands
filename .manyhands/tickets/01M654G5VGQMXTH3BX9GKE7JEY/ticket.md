---
slug: vh-d78zh
deps:
- 01M4CC0VN0MFTGKQ4HGTTC5H63
id: 01M654G5VGQMXTH3BX9GKE7JEY
title: Move the repository details out of the item tab
type: task
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Take D1's repository details (name, configuration, index, Retry, Remove)
out of the item tab's scroll region: into a sheet, or off item tabs
altogether.

Follow-up from Wave 03 D3. D3 ends the item tab's scroll region, after
the discussion, with the selected repository's details, which D1 drew at
the end of the content column (D3 design, Design Deviations; product
owner, 2026-10-27, "for now"). A fixed panel below the tab had left the
item region about 58 px tall in a 700 px window. On 2026-10-29, trying D3
on screen, the owner asked for this ticket: the details likely need
removing or moving into a sheet. UI rework is done separately from the
feature Cycles.

## Product decisions to ask before implementation

- Where the details go: a sheet, the Repositories settings page alone, or
  somewhere else.
- Whether an item tab shows anything of the selected repository.

## Scope

- Desktop only, under `src/desktop/`.
- The item tab's scroll region ends with the discussion.
- D1's `opening_a_tab_does_not_change_the_selection` reads
  `selection-name` with a tab open; it changes with the details.

## Depends on

D3 (`01M4CC0VN0MFTGKQ4HGTTC5H63`), whose item tab this changes.
