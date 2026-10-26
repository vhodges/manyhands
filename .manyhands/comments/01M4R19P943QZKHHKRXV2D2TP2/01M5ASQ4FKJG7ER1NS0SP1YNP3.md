---
id: 01M5ASQ4FKJG7ER1NS0SP1YNP3
item_id: 01M4R19P943QZKHHKRXV2D2TP2
created_at: 2026-10-19T19:20:21Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---

## Second implementation batch — Tasks 3–5

Owner approved the first batch and asked to continue. Stable scope,
confirmation-bound checked deletion, disappearance/replay coverage, wire
fixtures and RFC/audit updates are implemented. Details and exact commands:
[execution ledger](../../../docs/plans/2026-10-19-missing-repository-registration-removal-execution.md).

The mutation boundary now previews/removes a missing-root registration and
replays finished or interrupted requests without requiring that root on disk.
Distinct re-registrations, changed request identities and stale confirmations
remain protected under the approved stamp contract. Unrelated unfinished work
stays blocked and retained.

Focused verification passed: 106 mutation unit tests, 308 replay tests,
23 removal-boundary tests, 20 domain tests, 44 mutation contract tests,
71 read contract tests, 20 read-boundary tests and 8 domain integration tests.
Two narrower compatibility filters also passed (overlapping those suites).
The replay suite's first invocation hit the 120-second tool timeout; the
longer rerun completed with all 308 tests passing.

New wire fixtures use existing schemas. Actual Linux permission denial,
non-ASCII/spaced roots, aliases and dangling Git indirection are covered.
Formatting/diff checks passed. Fresh fetch/merge remained up to date at
`021f68425db6d4be38bcfbee0ee151829e06df41`; ticket HEAD remains
`cdd797cba21878eecb83beed6023c118be198256` and changes are uncommitted.

Task 6's full all-feature gates and complete-change review remain pending.
No native CI was dispatched; all planned test targets are already listed in
the native workflow. The ticket remains open. Paused for the second-batch
review checkpoint.
