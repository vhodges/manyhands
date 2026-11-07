---
slug: vh-yq8wb
id: 01M6V8RK88W7F8AHPK6MQKBQ2C
title: Promotion and sync stay blocked by a history_unknown flag that nothing clears
type: defect
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
**Combined into `01M66HFV44YXNQ7ANJVA16TQ1H` (vh-hyksw) on 2026-11-07.** It
is the same defect, found earlier through `item sync`. Its cause is a key
selection, which calls `configure` and marks the history unknown. That ticket
now holds this one's evidence and the generalized fix. Close this one
without a code change.

A repository-wide `history_unknown` flag, once set, is never cleared. While it
is set, every item branch that has no remote-tracking ref reads as "history
unknown". So `document promote`, `ticket close` and `item sync` refuse any
branch that has never been pushed, and `poll once`, the action the refusal
names, does not change that. In this repository a new document or ticket can
no longer be shared or promoted through Manyhands.

## Evidence

Found on 2026-11-07 while promoting the Wave 04 document
(`01M6V05RX2GZ4FFW8VMKTWFBTN`) from the registered main checkout, at main
`1e5616af`.

- `document promote --prepare` answered `recovery_required` with blocker
  `history_unknown`, naming `poll once` and a new prepare.
- `poll once` completed (request `01M6V6HDSQSCZH06FV8J2GYH2B`), but reported
  the document's branch `disposition=history_unknown`. Ticket branch
  `01M6RAA5FF2FN3M52QS8TW409W` reads the same.
- `item sync` on the document fetched, then stopped with `recovery_required`
  and `publication=pending` (request `01M6V6K9Q80B7KXZT57NPMPH94`, operation
  `01M6V6KA0QNSNF1BYB28ANDK6R`, left at `fetch_observed`).
- A second prepare still answered `history_unknown`.

## Cause

Read from the source; not yet reproduced by a test.

- `remote_polling_state.history_unknown` is set to 1 by the remote
  configuration update in `src/repository/remote/state.rs` (about line
  2302), and defaults from the repository's state when the table is created
  (about line 55).
- Only test support (`src/repository/closure/test_support.rs`) sets it back
  to 0. No production path clears it.
- `fetch_publication_evidence_for` (`src/repository/remote/state.rs`) falls
  back to `HistoryUnknown` for any context with no explicit evidence row
  while the flag is set, so a branch Manyhands created locally and never
  pushed is treated as possibly published and deleted.
- Promotion turns that into `PromotionBlocker::HistoryUnknown`
  (`src/repository/promotion/observe.rs`), and synchronization into
  `CleanIntegrationError::HistoryUnknown` (`src/repository/remote/refs.rs`).

## Workaround Used

The Wave 04 document was merged into main with Git (`d0352184`) and its
branch and worktree removed by hand. Manyhands' records do not show it as
promoted.

## Fix

Decide in the fix's plan when the history becomes known again: for
example, a complete fetch of the publication remote clears the flag, or
records explicit evidence for every local context (a branch absent from the
remote after a complete fetch, with no tracking ref, is `never_published`).
Keep the protection the flag exists for: a branch that was published and
then deleted on the remote must still answer `remote_context_deleted` or
`history_unknown`, never be republished silently.

## Acceptance

- After a configuration change and then a successful `poll once`, a branch
  created locally and never pushed can be synchronized and promoted.
- A branch deleted on the remote after it was published is still refused.
- Library unit tests over a local `git2` repository and a `file://` remote
  with an injected clock, table-driven, per `AGENTS.md` "Testing Strategy".
  No SSH case unless the plan shows the behavior depends on the transport.
- The CLI's `recovery_required` guidance is truthful: if `poll once` is
  named as the fix, it fixes it.
