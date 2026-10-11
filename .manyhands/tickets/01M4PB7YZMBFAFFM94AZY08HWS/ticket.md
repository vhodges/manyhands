---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4PB7YZMBFAFFM94AZY08HWS"
title: "An interrupted save is closed as rejected although it wrote its file"
type: "defect"
status: "open"
project: "manyhands"
team: "core"
---

When an authoring save writes its file and the process dies before the step is
recorded, a later refused retry closes the operation's journal row as
`rejected`. A rejected row means "this operation did nothing", but the first
attempt's write is still in the file, uncommitted.

## Evidence

Found on 2026-10-11 during Wave 03 Cycle F2 Task 8 (ticket
`01M4CC0VMR8HSPZXQ1WX41GWVK`, branch `manyhands/ticket/01M4CC0VMR8HSPZXQ1WX41GWVK`); pinned there by a test of
`ticket slug-assign`, and read from the code as applying to every authoring
save.

Sequence, for an item that already has an editing context:

1. A save writes the item's file and dies before the step is persisted (fault
   point `AfterOwnedWriteBeforeLifecyclePersistence`).
2. Someone edits the same file in the context without committing.
3. The same operation is repeated. The domain refuses with `ExternalChange`
   and, because the row recorded no step, closes it as `rejected`.

Nothing is committed and nothing is repeated. But the journal now says the
operation did nothing, and the F2 boundary, which reads the row to decide what
to report, answers `external_change` with no effects and releases the request.
The earlier write goes unreported.

For an item with no context the row keeps its context steps and stays pending,
so the result is reported as partial. The difference between the two cases is
only whether a step happened to be recorded before the write.

This comes from the rule added by ticket `01M4EWN2DK3MY6H4GBYDYXF6QH`: a
row with no recorded step is closed as rejected when its operation is refused.
That rule assumes no step means no effect.

## Proposed direction

Record that the write is about to happen before writing (or persist the write
step in the same breath as the write), so a row whose file may have been
written is never step-less. A refusal then leaves it pending, as it does for
an item with no context.

## Open points for planning

- Whether the same gap exists between other effects and their steps (the F2
  work found the same shape for `repo create` between creating the directory
  and recording `repository_initialized`).
- F2's design lists this as a known limit; that entry and its test change when
  this is fixed.
