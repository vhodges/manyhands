---
id: 01M5XTX949G7GTX75FGCSB1SXZ
item_id: 01M57SKVF6TQ3QM6AXJ35JGGSY
parent_id: 01M5XRX8YK4GFJNZ6YEXD99GGV
created_at: 2026-10-27T04:46:45Z
created_by: GPT-6 Astra <noreply@openai.com>
manyhands_managed: true
manyhands_kind: comment
---

## Task 7 QUALITY: two P2 corrections awaiting re-review

Base **`077f421f6cec3ee9fdc255c89ea3b2da2483a951`**. Controller reports full Task 7
SPEC approval there, session **`ses_e8554c7d2ffe8o3xfJnBqcRrQO`**. QUALITY
**`ses_e846d6706ffeMZlWpMYjwDMnGU`** then raised two P2 findings. Both were verified
in source and reproduced before correction.

1. Ordinary inspection now compares live `.manyhands/config.toml` routing with its
   retained execution digest and checks it again after session B. Stable and in-call
   routing changes refuse without a stale resumable view; restoration of original
   routing restores valid advice. Public tests cover contained/FF/merge ×
   Prepared/IndexPending under an already-held lease with SQL/Git preservation.
2. Terminal unlinked data uses the actual scoped request role. U no longer invents
   an R identity, and a Cancelled parent reports reason Cancelled rather than
   Abandoned. New R/U tests cover cancelled, abandoned and complete parents,
   immutable replay, mismatch/ID retention, and approved no-effect release for a
   completed parent. No new resolution attempt or policy change is introduced.

All four new regressions ran RED then GREEN. Whole-tree Devenv fmt/check/clippy
pass. **182 selected tests/cases passed:** 56 local mutation, 6 local read,
44 mutation-contract, 72 read-contract and 4 conformance target cases. The remote
conformance harness selected and passed one SSH case. No old assertion was changed,
and no schema/global enum, golden, dependency, frontend or target changed.

The execution ledger records exact cases, commands and counts. Full SPEC approval
applies to the base, not automatically to these corrections. Await **SPEC delta
verification and QUALITY re-review**; Task 8 remains unstarted, ticket implementing.
No PR, closure, merge into main, native dispatch or cleanup.
