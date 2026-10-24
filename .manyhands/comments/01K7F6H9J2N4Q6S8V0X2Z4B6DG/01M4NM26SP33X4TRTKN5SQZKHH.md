---
id: 01M4NM26SP33X4TRTKN5SQZKHH
item_id: 01K7F6H9J2N4Q6S8V0X2Z4B6DG
created_at: 2026-10-11T14:04:12Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

Planning checkpoint: reused this ticket's registered worktree and branch. Fresh
fetch observed origin/main `d1ca88772c5768d55aadb2ca4dcac1c69dab0d17` (Cycle 07
merge). Rebased ticket HEAD from `5539c60eeb4022c71b1dc13716aeb2d2545a7375` to
`27d5ef54c67a116a590602d86b7086f8602c0c62` without conflicts; ancestry passed
and the ticket worktree was clean. Unrelated main edits and other worktrees were
preserved. The published ticket ref still holds the original checkpoint, so a
later push must reconcile it; no force push is authorized.

Prepared the proposed Cycle, design and detailed implementation plan under
`docs/Cycles/wave-02-cycle-08-one-shot-polling-and-materialization.md` and
`docs/plans/2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-{design,implementation}.md`.
The execution ledger records preflight, grounding and the decision audit.

Current main has an advertisement-only poll, exact-target fetch and guarded
fast-forward inside synchronization, and no code that creates a context from a
remote branch, validates an item from a fetched tree, enforces stored polling
policy, or stores an attempt or next eligible time.

The plan adds `poll_publication_remote`: automatic eligibility from stored
policy, a poll-set fetch with guarded tracking refs, clean strictly-behind
fast-forwards, validated materialization with a durable intent, fixed per-target
dispositions and an index-only refresh. Planning only; nothing was implemented,
committed or pushed.
