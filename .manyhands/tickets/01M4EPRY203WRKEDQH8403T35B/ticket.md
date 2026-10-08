---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4EPRY203WRKEDQH8403T35B"
title: "Say when an item's effective copy differs from primary"
type: "task"
status: "open"
project: "manyhands"
team: "core"
wave: "03"
---

An item that has its own worktree is read from that worktree; the primary
copy is not shown. That rule stands (product owner, 2026-10-07 and
2026-10-08). What a reader cannot tell today is whether the two copies
differ, so:

- a front end cannot offer a comparison of the draft with what is published;
- a worktree left behind after its branch merged keeps shadowing primary,
  even if primary later moves on, and nothing says so;
- nothing at read time says primary changed under a draft; that surfaces
  only at synchronization.

## Decision

The product owner decided the shape on 2026-10-08: keep one effective copy,
and add a flag pointing at the commit.

## Expected Behavior

- An item read from an item worktree says whether a primary copy exists and
  whether it differs from the effective copy, and names the primary commit
  the comparison was made against.
- The comparison is made by the indexer and stored; a read does not scan.
- An item read from primary carries the same fields with nothing to report.
- The fields join the published read contract as an addition within v1.

## Acceptance Criteria

- A draft that differs from primary, a draft identical to primary, an item
  that exists only in its worktree, and a merged worktree whose primary copy
  has since changed are four distinguishable results.
- The stored commit is the primary head the refresh observed.
- No canonical Markdown or Git state is changed.

## Related

- Found during Wave 03 F1 (`01M4CC0VMQ7R15A7M9SPN3KB67`).
- Bears on pruning merged worktrees and archiving closed tickets, which are
  not in this ticket.
