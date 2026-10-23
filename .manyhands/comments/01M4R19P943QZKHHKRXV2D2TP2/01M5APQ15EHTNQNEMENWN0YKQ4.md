---
id: 01M5APQ15EHTNQNEMENWN0YKQ4
item_id: 01M4R19P943QZKHHKRXV2D2TP2
created_at: 2026-10-19T18:27:52Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---

## First implementation batch — Tasks 0–2

The approved plan's baseline, domain removal and coordination/recovery tasks
are implemented. See the [execution ledger](../../../docs/plans/2026-10-19-missing-repository-registration-removal-execution.md)
for source changes, red/green results, rulings and exact commands.

The domain can now remove exact stored-root registrations after the root or
`.git` disappears. It preserves unrelated unfinished work and uses the existing
transactional cascades. Guarded deletion handles reappearance, lease-file
replacement, registry changes, contention and own-operation retry. Files,
drafts, key/host state and durable mutation records are preserved.

Focused Devenv verification: 8 repository-enablement tests, 16 domain unit
tests, 4 recovery tests, 3 cascade tests, and 8 existing live mutation-removal
tests passed. Formatting and diff whitespace checks passed. Merge-tree check
against fetched main was conflict-free.

The mutation-boundary scope/confirmation/replay work (Tasks 3–4) remains; this
is not an end-to-end fix yet. Contract/docs updates and full all-feature gates
are reserved for Tasks 5–6. Native Windows/macOS evidence is pending, and no
CI was dispatched. Changes are uncommitted; the ticket remains open.

Paused for feedback at the first-batch review checkpoint.
