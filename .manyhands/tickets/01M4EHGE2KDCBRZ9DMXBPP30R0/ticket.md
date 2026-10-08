---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4EHGE2KDCBRZ9DMXBPP30R0"
title: "Registered repository accessibility is never updated"
type: "defect"
status: "open"
project: "manyhands"
team: "core"
wave: "03"
---

Nothing writes a repository accessibility other than `accessible`. A
registered repository whose folder has been deleted or become unreadable
still lists as accessible. The F1 reads report what is stored and do not
probe, so `list_repositories` repeats the wrong value.

Found during Wave 03 F1 (`01M4CC0VMQ7R15A7M9SPN3KB67`). The F1 execution
ledger on that ticket's branch,
`docs/plans/2026-10-07-wave-03-foundation-01-read-boundary-and-results-execution.md`,
holds the detail.

## Expected Behavior

- A registration whose root can no longer be opened is stored, and listed, as
  `inaccessible`, and returns to `accessible` when it can be opened again.
- Reads still do not probe; whichever write path or refresh observes the
  repository records what it found.

## Acceptance Criteria

- A test first shows a registered repository whose folder was removed listing
  as accessible.
- After the fix, the same repository lists as inaccessible after the next
  refresh, and as accessible again once restored.
