---
manyhands_managed: true
manyhands_kind: comment
id: "01M5YS0TS2JNG7Z4BBZ7N5448D"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-27T13:32:59Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 5b: The header, chips, pill and banners

Commits: `4b2c600` (feat: give the item tab its header, chips, pill and
banners), `63566e7` (docs: the ledger row and this comment), and the review
fixes (fix: address the Task 5b review).

The item tab's header placeholder is now the design's header block, above
the item's scroll region:
- the breadcrumb (repository, then Tickets and the short code or ID, or
  the folder and file name);
- the title;
- the chips, each a button that emits `OpenDetails` for its field (Task 6
  opens the sheet from it);
- the state pill, which reads the viewing state and never uses
  `item-state`;
- Edit, Share and the primary action, disabled with "Available in a later
  release";
- the banner slot, most severe first and in the table's order within a
  tone.

D1's status, not-refreshed and recovery lines are banners, with D1's IDs
and actions. A nonconforming file, one with no ID, shows a "Needs fixing"
banner with its problems and Repair disabled; a conforming item read by its
path does not. Comment ends the discussion. Details, relationships and
source stay on the page until Tasks 6 and 7, and D2's draft banner until
Task 18.

Ruling (owner, 2026-10-27, from the review): the design's "This ticket has
no type…" sentence cannot be produced, because the library's
`missing_field` names no field. "Needs fixing" shows D1's reason for every
nonconforming file. No library change.

Named edits: E-6's label change, and E-7's first half. Under E-7, the
Inspect tab's later actions are Edit, Share and Close ticket… in the
header, then Repair in the banner.

Review fixes: the banner order follows the table within a tone; "Needs
fixing" keys on the file, not the tab; the action labels are named, not
picked by position; each disabled action's note is its accessible
description; `Banner::Changed`'s allowance names Task 16.

Tests, desktop-test-support `--bin manyhands`, after the review fixes:
`view::item` 48 passed (12 in the header tests), `keyboard_tests` 23
passed, `model::item` 9 passed. Desktop clippy, all-features clippy and fmt
are clean. Before the review, the whole desktop binary was run once for the
stop rule: 573 passed. No other pre-existing test changed.
