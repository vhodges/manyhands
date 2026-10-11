---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4PB80T7SVB23TQ6466RDHCB"
title: "Repeating a document move that already committed is refused"
type: "defect"
status: "open"
project: "manyhands"
team: "core"
---

`save_document`, repeated under the same operation ID after its checkpoint
step, compares the now-absent source file with the caller's original
`expected_source` and returns `ExternalChange`. The move committed; the
repeat should finish it.

## Evidence

Found on 2026-10-11 during Wave 03 Cycle F2 Task 8 (ticket
`01M4CC0VMR8HSPZXQ1WX41GWVK`, branch `manyhands/ticket/01M4CC0VMR8HSPZXQ1WX41GWVK`). Caught by a test, not by reading.

Sequence: a document move commits; the index hand-off fails (fault point
`BeforeIndexTransactionCommit`); the same operation is repeated with the same
request. The domain substitutes "missing" for `expected_source` only when the
last recorded step is `document_move_observed`. After
`authoring_checkpoint_observed` it compares the absent source with the
original digest and refuses.

The F2 `document move` command works around it: it passes an
`expected_source` only while the context still holds a file at the source.
That check is made outside the repository lease.

## Proposed direction

In `save_document`, treat a missing source as expected for any row at or past
the source's removal. The F2 binding's probe then goes away, and with it the
unleased check.

## Open points for planning

- Whether the destination's expectation needs the same treatment on a repeat.
- Which F2 tests in `tests/mutation_replay/documents.rs` then assert the
  library's behaviour directly.
