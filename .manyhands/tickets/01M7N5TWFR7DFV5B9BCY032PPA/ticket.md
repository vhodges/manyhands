---
slug: vh-p7fp7
id: 01M7N5TWFR7DFV5B9BCY032PPA
title: 'Nonconforming document view: Zorite read view for the body, Git details in a sheet'
type: story
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
A nonconforming document (a file under `docs/` with no valid front matter, opened by its path as an Inspect tab) shows its whole file in a plain read-only text area, and its Git details sit in the main content area above it. Change both: the body reads like any other document in the read view, and the Git details move to their own sheet.

## What changes

- **Body.** Replace the read-only `Textarea` (`ItemPane::source`, `src/desktop/view/item/source.rs`) with the same read-only view an item tab uses for its body, the Zorite editor behind `editor::read_only::ReadOnly` (`read_body`). The file is shown as read. Nothing about it becomes editable, and the text still never enters a draft or the markdown normalization beyond what the read view already does.
- **Git details.** Remove the provenance block (`ItemPane::provenance`: context kind, branch, worktree, head commit) from the main content area, and show it in a new read-only sheet.
- **Header.** Add one button to the item header that opens the Git details sheet. The sheet is read only. Nothing else is added to the header.

## Open points for planning

- Whether the sheet is a new sheet next to the item details sheet (`sheet.rs`, `open_details`) or a mode of it. A nonconforming file has no item details sheet today: "An item tab's details are in its sheet. A file read by its path has no sheet."
- What stays in the page for a nonconforming file: the identity lines (kind, path), the fields, "Needs fixing" problems and unknown metadata. Only the Git details are named for removal.
- Whether an Inspect tab of a non-UTF-8 file keeps its "not UTF-8" message in place of the read view (it should).
- Whether the Git details button shows for item tabs too, or only for files read by their path. Ask the owner if planning cannot settle it.
- The Repair… and Adopt… offers read the file's source as read (`repair_offer`); the new body must not change them.

## Acceptance

- A nonconforming document opens with its body in the Zorite read view, selectable and copyable, not editable by key, paste, drop or checkbox (the read-only guard's layers).
- The page no longer shows Context, Branch, Worktree or Head commit. The header has a Git details button, and pressing it opens a read-only sheet with those facts.
- The header has no other new control.
- Tests are desktop behavior tests (model state, actions, what the sheet shows), not appearance tests (AGENTS.md Testing Strategy). Tests that pin the old textarea or the old placement are updated or removed.
