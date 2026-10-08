---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4EHGE4BXGPMCWWA1S1QR0JW"
title: "Make an index refresh atomic"
type: "defect"
status: "open"
project: "manyhands"
team: "core"
wave: "03"
---

A refresh persists the primary context, each active item worktree and the
cleanup of disappeared contexts in separate transactions. Between them, or
after a `RetryRequired`, the index can hold an item twice or not at all. The
F1 reads tolerate this (an item held twice is read by a fixed candidate
order and reported `stale`), but the index should not pass through those
states.

Found during Wave 03 F1 (`01M4CC0VMQ7R15A7M9SPN3KB67`). The F1 execution
ledger on that ticket's branch,
`docs/plans/2026-10-07-wave-03-foundation-01-read-boundary-and-results-execution.md`,
holds the detail.

## Expected Behavior

- One refresh of one registration commits once: a reader sees the index
  before the refresh or after it, never part of it.
- A refresh that must retry leaves the previous contents in place.

## Acceptance Criteria

- A test interrupts a refresh between contexts and shows a reader observing
  an item twice or missing; after the fix it cannot.
- Refresh and rebuild still give identical contents.
- The read lock is not held longer by a reader than it is today.
