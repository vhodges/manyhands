---
slug: vh-eq3b1
id: 01M67BN4DMYV41ME9FHEBCZ2RA
title: Point a conflict resolve that failed after writing at conflict show
type: task
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
A `conflict resolve` that fails on the filesystem after it has written a path
should tell the person how to go on, not stop at a bare `internal_error`.

Follow-up from Wave 03 C4 (`01M4CC0VMWEQQ9G4W0DAG7ADPY`): a Known Limit from
the final review (CLI recommendation 3; Task 12b concern 2). The owner asked
for this ticket on 2026-10-30. Start it after C4 merges.

## Today

When `conflict resolve` meets a filesystem refusal after it has written a
path, it answers `internal_error`, exit 4, with no recovery action and no
`next:` line. `conflict show` of the synchronization names the way on
(`retry_resolution`), but nothing points the person there.

## Expected behaviour

The CLI's human output for that answer prints a `next:` line naming
`conflict show` for the synchronization, and with it the operation ID the
request named. That is the case where the result records a durable write and
carries no conflict data. To do this, the operation must be plumbed into the
CLI's `Seen` state.

The JSON envelope does not change. `conflict.show` is not a recovery action
kind, and no new `RecoveryActionKind` is added (C4 decision 8).

## Acceptance criteria

- A `cli_remote` case stages the refusal after a written path, as C4 Task
  12b's case does with a read-only directory. It shows the `next:` line, and
  shows that following it reaches `retry_resolution`.
- The human-output golden and the CLI contract RFC's `conflict resolve` notes
  are updated, and the C4 Known Limit is amended.
- A CLI-only change: no library change is expected.
