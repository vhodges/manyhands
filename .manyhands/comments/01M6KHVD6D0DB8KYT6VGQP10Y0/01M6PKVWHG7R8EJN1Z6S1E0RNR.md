---
id: "01M6PKVWHG7R8EJN1Z6S1E0RNR"
item_id: "01M6KHVD6D0DB8KYT6VGQP10Y0"
parent_id: "01M6PFK6EZAAWE5YH55M5N5PV2"
created_at: "2026-11-05T19:44:47Z"
created_by: "GPT-6.1 Sol <noreply@openai.com>"
manyhands_managed: true
manyhands_kind: comment
---

## Missing index-spend correction — same SPEC re-review requested

The controller-accepted **one Medium SPEC finding at `026f9171`** is corrected
locally; this is not review approval. Increment review remains full
`597bed6ba668c8697579c2a79bc4c70ede4548d4..fix-HEAD`, where fix-HEAD introduces
this checkpoint with `fix: reject retirement indexing with missing spend evidence`.
Full Task 2 base `7b1912d1`; prior recovery approvals remain intact.

Forward claim-row validation now has reverse operation/history correlation.
Previously claimed live/completed epochs require their spend; an invalidation
tombstone requires the preceding live epoch's spend. Untouched authoritative
epoch-zero handoffs stay eligible. Provable first/middle/last spend-history loss
refuses before another owner can be minted. F2 and index counters stay distinct;
missing evidence is never recreated. No schema or public authority was added.

Compiled RED **2 passed / 2 failed**, expanded RED **2 passed / 3 failed**;
focused GREEN **5 passed**. Tests use real private initial absence under inert F2
projections, restore canonical delete-guard DDL with FK checks on, and verify
startup plus both unpoisoned already-open/cross-service recovery. Full registry/
cache/resource images stay unchanged. Valid duplicates, new-current takeover,
tombstones, never-claimed handoffs and published DOMAIN history remain accepted.
These are not Task 3 mutation journeys; the auxiliary poisoned hook is not evidence.

Final foreground/serial frozen gates: **106 context units**, **105 registered
retirement SSH**, **60 recovery**, **33 refresh**, **70 persistence**, **5 legacy /
3 index migration**, **130 request units**, **79 discovery / 384 replay / 52
foundation**; whole-tree check/fmt/strict Clippy pass. Raw logs:
`/tmp/opencode/task2-request-fence-specfix-*.log`. All **1,581** source hashes match
inventory **`addf29e1263376270220f57831490f91ecf614fd649e2eddb6c310e814b8fd91`**.
The prior 026f 101/105 evidence is separately retained, not fresh fix evidence.

Controller must obtain **same SPEC re-review then QUALITY** before expansion or
push. Full Task 2 remains incomplete: remaining fault/index/history variants and
shared source/batch A0–A4, execution-consumption, no-child gaps and holders/Z/GC;
Task 3/4 interfaces/runtime remain downstream. Only rows 36/38 were annotated.
No agents, push/status/PR/main integration/closure/native CI/other-worktree or
default-profile/real-context/key/draft actions. ID from supported isolated current
CLI only; no Windows/native/casefold-volume/cross-compilation/full-suite claim.
