---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4EHGE9TMR99VYZC184J9XEC"
title: "Discovery entry caps are reached by ordinary use"
type: "defect"
status: "open"
project: "manyhands"
team: "core"
wave: "03"
---

Discovery bounds its traversal with fixed constants in `src/repository.rs`
(`MAX_MANAGED_DIRECTORY_ENTRIES` and `MAX_DOCUMENT_DIRECTORY_ENTRIES`, both
1,024). Two of the ways they are counted put the limit within reach of this
repository:

- **Comments.** One counter covers the listing of the primary comment
  directory and every comment file beneath it, so the limit is about 1,024
  comments for the whole repository, not per item. The F1 branch alone holds
  135.
- **Tickets.** `.manyhands/tickets` counts each ticket twice. Past 1,024
  entries, about 512 tickets, an item worktree can no longer be prepared.

Past a cap the directory is recorded as not fully read; the F1 reads then
report `complete: false`.

Found during Wave 03 F1 (`01M4CC0VMQ7R15A7M9SPN3KB67`). The F1 execution
ledger on that ticket's branch,
`docs/plans/2026-10-07-wave-03-foundation-01-read-boundary-and-results-execution.md`,
holds the detail.

## To Decide

Whether these are safety bounds or product limits. The constants are
arbitrary, not a property of the design. Options include counting per
directory, raising the bounds, and pairing either with archiving closed
tickets. This ticket is the defect; the wider question of scale belongs to a
product discussion.

## Acceptance Criteria

- A repository with 2,000 comments spread over its items refreshes and lists
  them all, with `complete: true`.
- A repository with 1,000 tickets can prepare an item worktree.
- Traversal remains bounded against a hostile or runaway directory.
