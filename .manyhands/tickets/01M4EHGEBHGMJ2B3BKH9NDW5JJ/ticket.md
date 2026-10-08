---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4EHGEBHGMJ2B3BKH9NDW5JJ"
title: "The indexer reads repository configuration without a guard"
type: "defect"
status: "open"
project: "manyhands"
team: "core"
wave: "03"
---

F1 made the read boundary read `.manyhands/config.toml` through the guarded
reader (no symbolic links, non-blocking open, 64 KiB cap). The indexer and
the write paths still use a plain read. A FIFO at that path hangs a refresh
while it holds the repository's exclusive lock, and there is no size cap.

Found during Wave 03 F1 (`01M4CC0VMQ7R15A7M9SPN3KB67`). The F1 execution
ledger on that ticket's branch,
`docs/plans/2026-10-07-wave-03-foundation-01-read-boundary-and-results-execution.md`,
holds the detail.

## Expected Behavior

- Every read of the configuration file goes through the guarded reader, or
  one equivalent to it, with a stated size limit.
- A configuration file that is a FIFO, a symbolic link, a device or too large
  is reported as a configuration problem; the refresh completes.

## Acceptance Criteria

- A test puts a FIFO at `.manyhands/config.toml` and first shows a refresh
  that does not return; after the fix it returns with a problem.
- A valid configuration file reads exactly as before.
