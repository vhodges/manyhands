---
id: 01M5A3JMZCNDJB7EA6WJQJTZB1
item_id: 01M4CC0VMSZ7BRS740C8VSQ5ZK
created_at: 2026-10-19T12:53:25Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

Task 4 checkpoint: CLI repository, credential and status reads.
Base: `2e3849f31c0d7abd39b559ea15edce7312c2df03`. Owner-authorized commit:
`feat: CLI repository, credential and status reads`.

Task 3 controller handoff: **spec PASS, quality PASS, no findings,
323 targeted tests**. Task 4 ran inline with TDD, apply_patch and self-review.
The required main preflight was already up to date; no base change.

- Wired all 13 Task 4 verbs to F1 with exact success scopes, registration-free
  inspection, repository-before-ID validation, canonical key/operation IDs and
  IPv6 authority bracket stripping. Added public resolve/item-ID helpers;
  item-ID dead-code annotation is temporary until Tasks 5–6. Fourteen later
  content/relationship placeholders remain.
- Added 13 human DTO renderers with unit coverage, including status notes,
  metadata, empty lists and pipable public key text. Real-binary tests cover
  every Task 4 verb plus `id new`, exact schemas/scopes, both modes, failure
  recovery lines, unusual path spellings and linked-worktree owner resolution.
- Verified all requested input/blocked failure routes. Unix mode-000 config
  reaches `repository_inaccessible`, exit 3; it was not skipped on this run.
  Degraded `index status` preserves R-2's failure and rebuild recovery. A
  cache-write lease child proves `busy`, exit 5, without pinning variable scope.
- RED: 13 verb tests returned `command_unavailable`; 13 renderer tests and the
  scoped item-ID helper failed at scaffolding. Placeholder count failed until
  reduced. Existing startup already satisfied the busy test.
- GREEN: **138 targeted tests**: **62 bin + 33 CLI + 43 mutation_contract**,
  zero failures/ignored. Generated and read **28 new JSON goldens**, then
  verified without updates. Review caught a bare-root placeholder omission;
  reproduced, corrected and regenerated its case group before the final run.
- Scoped clippy with warnings denied and fmt check **PASS**. Fixed one test-only
  clippy notation issue (`0o000` permissions). No library gap or library edit,
  no pre-C1 semantic test edits. Self-review PASS; independent review pending.
- Execution ledger records every design code without a C1 envelope route and
  its reason, scope authority, path coverage, RED/GREEN evidence and counts.

Full evidence:
[execution ledger](../../../docs/plans/2026-10-18-wave-03-cli-01-shell-and-inspection-execution.md#task-4-checkpoint--2026-10-19).

Task 5 remains unstarted. Ticket remains open; full suite reserved for Task 8.
No subagents, rebase, push, PR, merge into main, closure or cleanup.
