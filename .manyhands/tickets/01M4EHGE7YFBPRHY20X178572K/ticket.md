---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4EHGE7YFBPRHY20X178572K"
title: "Version the index: schema and content"
type: "task"
status: "open"
project: "manyhands"
team: "core"
wave: "03"
---

The index has no version. Two needs, to be designed together:

- **Schema version.** Migration is a growing list of probes for tables and
  columns. A stored version would replace the probes and let a build refuse,
  or rebuild, an index newer than it understands.
- **Content version.** An older build sharing the data directory can refresh
  an index a newer build migrated. It rewrites rows without the newer
  columns (slugs, edges, `closed_by`, unknown metadata) and leaves the index
  reading `current`. A version on what a refresh wrote would let the newer
  build see that and report `stale`.

The product owner noted on 2026-10-08 that no builds are in use yet, so this
is not urgent, and that a version stamp may serve other purposes too.

Found during Wave 03 F1 (`01M4CC0VMQ7R15A7M9SPN3KB67`). The F1 execution
ledger on that ticket's branch,
`docs/plans/2026-10-07-wave-03-foundation-01-read-boundary-and-results-execution.md`,
holds the detail.

## Acceptance Criteria

- The index records the schema version it was created or migrated to, and
  the version of the build that last refreshed each registration.
- A build opening an index with a newer schema neither corrupts it nor
  reports it `current`.
- A registration last refreshed by a build that wrote less is reported
  `stale` until refreshed.
- The index stays disposable: deleting it and rebuilding gives the same
  result.
