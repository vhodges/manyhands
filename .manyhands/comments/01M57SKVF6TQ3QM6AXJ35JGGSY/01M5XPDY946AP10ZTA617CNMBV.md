---
id: 01M5XPDY946AP10ZTA617CNMBV
item_id: 01M57SKVF6TQ3QM6AXJ35JGGSY
parent_id: 01M5XFVS1XWNMG79KS7DYXZ7GH
created_at: 2026-10-27T03:41:41Z
created_by: GPT-6 Astra <noreply@openai.com>
manyhands_managed: true
manyhands_kind: comment
---

## Task 7 SPEC #3: remaining inspection corrections awaiting re-review

Base **`3836310a8471795187311fa4e730bfc2c17b53c7`**. Controller reports six of seven
SPEC findings closed, with two Important observations remaining under #3. Both
were verified in source and reproduced through public reads before correction.

1. Resolution inspection now uses the executor's exact framed preimage hash and
   path-phase classifier. A real two-conflict fixture proves accepted R cancelled
   before its first write and partial R after one path both request the original R.
   Prepared baseline/output images are allowed; an observed path restored to its
   baseline is external-only. The partial case includes one actual no-Drop exit
   before SQL observation, with the second path still untouched.
2. Ordinary S inspection now requires retained disposition-specific full baseline/
   result image proof, including filtered bytes, index/mode checks, coarse intents
   and recorded release. It compares future filters without adopting fresh baseline
   authority and rechecks stored evidence/live proof after session B. A 36-case
   graph/phase/filter/file-or-index matrix, plus coarse/result, missing/corrupt proof,
   read-race and release controls, all pass while the repository lease is already
   held. Git administrative files, worktrees and relevant SQL remain unchanged.

Ten new public tests are in `tests/mutation_replay/local_sync_inspection_spec.rs`.
No old assertion was weakened. Shared hash/execution semantics retain their existing
format and have passing remote/domain controls. No schema/golden, dependency,
frontend or target changes were required.

Verification: **50 local mutation + 6 local read + 44 mutation-contract + 72
read-contract + 159 domain + 5 shared controls + 4 conformance cases + 4 compile-fail
tests = 344 passing selected cases**. Each conformance target selected one; the
remote harness reported **1 SSH cases passed**. Whole-tree Devenv fmt/check/clippy
with warnings denied passed. The ledger records exact commands, RED/GREEN results,
matrix coverage, fixture corrections and final narrow re-verification.

Both observations are locally addressed, pending **SPEC re-review then QUALITY**.
No approval is claimed; Task 8 remains unstarted and ticket status implementing.
No PR, closure, merge into main, native CI or cleanup. Latest main `0464c42` was
already merged and merge-tree passed; native execution remains unverified.
