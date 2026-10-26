---
id: 01M5VZZ5VXNW0PXCGTQWNXHYBV
item_id: 01M57SKVF6TQ3QM6AXJ35JGGSY
created_at: 2026-10-26T11:36:41Z
created_by: GPT-6 Astra <noreply@openai.com>
manyhands_managed: true
manyhands_kind: comment
---

## Checkpoint: Task 4 QUALITY fixes awaiting re-review

Base **`f578bf7a37b5c52e205ef5cb3e1aaae0c8e40f16`**. Controller reports SPEC
`ses_e898c0189ffebSYYicNzmsZaXQ` **approved all five earlier fixes, zero findings
at that head**. QUALITY `ses_e881a2165ffeEIJWyTo9j3HVR3` then found three P2s.
Each was verified in source before a failing regression and its correction.

1. **Terminal history FK deletion:** child step/resolution FKs now cascade from
   the companion, so the existing public removal transaction deletes the whole
   terminal history. Pending work still refuses. Public tests verify success,
   pending preservation, rollback on injected deletion failure and same-ID retry.
   Versioned marker `local_context_sync_child_cascade_v2` upgrades exact valid v1
   child tables transactionally without losing evidence. Upgrade, rollback and
   corrupt-legacy refusal controls pass; no silent schema replacement.
2. **Child integer authority:** typed i64 decoding checks every child integer
   field, nonnegative ordinal, positive epoch/generation, matching parent generation
   and child epoch no later than parent. Thirty-two corruption cases across both
   tables return RecoveryRequired from real read/takeover with unchanged ownership
   and Git. Valid ordinal boundaries and older child epochs remain accepted.
3. **Checkpoint OID:** persisted closure_checkpoint now uses the shared strict
   canonical nonzero full-OID decoder. Zero/uppercase/short/invalid corruption of
   genuine Closing authority refuses read and takeover. Genuine checkpoint and
   valid-descendant controls remain green.

Red sequence: initial five QUALITY tests **2 pass / 3 fail**; after integer/OID
fixes **4 pass / 1 fail**; adding migration tests before the upgrade **5 pass /
3 fail**; after migration **8 pass**. Final local group includes nine new QUALITY
tests, including the public deletion-rollback control. No old assertions weakened;
the fresh-schema test setup now clears both migration markers.

Final targeted verification: **58 local + 20 removal unit + 8 child fence + 9 exact
library controls + 8 removal integration + 4 compile-fail = 107 passing tests**.
Whole-tree Devenv fmt --check, all-feature locked check, and all-target/all-feature
locked clippy -D warnings passed. Exact commands/counts and source rationale are
in the execution ledger. No full suite or native run; no subagents.

Ticket remains **implementing**. These are implementer dispositions; **QUALITY
re-review and SPEC verification of the changed fixes remain required before
Task 5**. No known implementation blocker remains. No executor/request-control
work, new public API, key behavior change, PR, close, merge or cleanup.
