# 04 · Item area (tabs, panes, item header)

The editor bodies are placeholders, so only the frame is specified here.

## Tab strip (34 px, `chrome` background, bottom border)

Each tab: padding 0 6 0 12, gap 7, max width 260 px, right border 1 px.
- Type icon 13 px. Active: amber. Inactive: `muted`.
- Short code: mono 11 px, `secondary`, no wrap. Documents have no code.
- Title: 13 px, ellipsis.
- **Unsaved dot:** 7 px amber circle, tooltip "Unsaved changes". Also announce "unsaved" in the accessible name (color alone isn't enough).
- Close button 20×20, `x` 12 px.
- Active tab: `editor` background (matches the body). Focused pane: 2 px top line in amber. Unfocused pane: 2 px `faint`.
- Tooltip: `Title · repository`. It also shows the branch and worktree when editing (RFC: tabs show context).
- Right end: Split right (`columns-2`).

## Item header (body padding 22 32 28, gap 16)

1. **Breadcrumb** (12 px, `secondary`): `folder-git-2 repo › Tickets › MH-VH-7Q3`. For documents: `repo › docs/Waves › wave-03-dogfooding.md`.
2. **Title row** (wraps): the title is 22 px / 600 / line-height 1.25. Below it, **metadata chips** (11.5 px, padding 2 8, radius 4, 1 px `border`, `key` label in `muted` + value): `Type cycle`, `Status in progress` (colored as in the pill table), `Project manyhands`, `Team core`, and, when blocked, `Waiting on MH-VH-8FN` (red). Documents: `Status draft|approved`, `Updated …`.
3. **Actions** (right; wrap below on narrow panes; no wrapping inside a button):
   - **State pill:** 12 px, padding 4 10, radius 5, `panel` background, 1 px `border`, icon + label. Its tooltip carries the Git detail (branch, worktree, checkpoint count).
     - Editing: `pencil` amber, "Editing · saved on this computer".
     - Primary copy: `circle-check` green, "Same as the shared copy".
     - Closed: `archive` secondary, "Closed".
     - Needs recovery: `circle-alert` red, "Needs attention".
   - `Save` (secondary button, `save` icon, tooltip "Save a checkpoint on this computer (Ctrl S)").
   - `Share` (secondary, `cloud-upload`, tooltip "Bring in others' changes and share yours through origin"). With no publication remote, the label is "Share…" and it opens remote setup.
   - **Primary:** `Close ticket…` for tickets, `Approve and merge…` for documents. Amber background, dark text, weight 600. It opens the confirmation (`05`). For closed tickets: `Reopen…` (not designed).
4. **Banner slot** (only when needed), above the editor body: padding 10 12, radius 6, tinted background and 1 px tinted border, icon 15 px, text 12.5 px, one action button.

| Banner | Tone | Text | Action |
| --- | --- | --- | --- |
| Conflict | red | You and Jo Smith both changed this ticket since it was last shared. Choose what to keep before you share again. | Review changes → Resolve sheet |
| Ticket missing required metadata | amber | This ticket has no type, so Manyhands can't save it yet. Pick a type to fix it. | Choose type |
| Unmanaged document | amber | This file isn't a managed document yet. Adopting it adds a title and an ID at the top; the rest stays exactly as written. | Adopt… |
| External change while dirty (future) | amber | Someone changed this file outside Manyhands. Your edits are kept. | Compare… |

5. **Editor body:** a placeholder until the editor round. Reserve min height 360 px.

## Empty pane

Centered: the app icon at 56 px (35% opacity, grayscale), "Open a ticket or document from the lists", and "Ctrl K to search" in mono.

## Rules carried from the RFC

- Viewing does **not** provision a worktree. Starting an edit or comment does, and reports progress in the state pill ("Getting a private copy ready…") before authoring is enabled.
- Navigation never saves, shares or merges implicitly.
- One tab per item key. Re-opening focuses the existing tab.
