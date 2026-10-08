---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4EHGE60PK1WQ8DM4FRJV9YF"
title: "Committed files in an item worktree are stored as uncommitted"
type: "defect"
status: "open"
project: "manyhands"
team: "core"
wave: "03"
---

A file that is committed and unchanged in an item worktree is usually stored
by a refresh as `uncommitted`, because the save path does not update that
worktree's Git index. F1 publishes the value as `change_source` on item
reads, so front ends will show saved, committed work as uncommitted.

Found during Wave 03 F1 (`01M4CC0VMQ7R15A7M9SPN3KB67`). The F1 execution
ledger on that ticket's branch,
`docs/plans/2026-10-07-wave-03-foundation-01-read-boundary-and-results-execution.md`,
holds the detail.

## Expected Behavior

- `change_source` says `uncommitted` only when the file differs from the
  commit at the worktree's head.

## Acceptance Criteria

- A test saves and commits an item through the authoring path, refreshes, and
  first shows the stored source as uncommitted.
- After the fix it is stored as committed, and an edit after the commit is
  stored as uncommitted.
- No canonical Markdown is changed by the fix.
