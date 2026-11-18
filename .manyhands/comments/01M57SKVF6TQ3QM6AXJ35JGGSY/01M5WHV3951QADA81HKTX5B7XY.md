---
id: 01M5WHV3951QADA81HKTX5B7XY
item_id: 01M57SKVF6TQ3QM6AXJ35JGGSY
created_at: 2026-10-26T16:49:02Z
created_by: GPT-6 Astra <noreply@openai.com>
manyhands_managed: true
manyhands_kind: comment
---

## Task 5 SPEC: two-sided manifest binding correction

Base **`ef746ba7d060e8b0d087213998f4c656c1f2dd13`**. Controller reports SPEC
`ses_e87b8a702ffe6HryC0itCXg4H5` resolved the preceding two image findings and found
one P1: candidate proof could reuse a baseline digest without binding its before
OID/mode/path mapping to the frozen context tree.

Source verification confirmed the early index-mismatch return and after-only
validation gap. The exact composition test was **RED**: actual FF checkout,
candidate index/parent ref, only before.oid changed to after.oid, original baseline
hash retained, old bytes restored; retry returned Ok/Complete. It is now **GREEN**
across eight document/ticket OID/mode/path-set variants. Refusal preserves owner
epoch, all compared authority/discovery rows, intents, manifest, files/index,
context/primary/tracking refs and reflog counts; no takeover or settlement occurs.

One central complete binding gate now validates both path/OID/mode projections
before any reconciliation shortcut or owner change. Canonical Git-tree hashing is
metadata-only and cached only on the immutable decoded proof, never supplied by
stored data. It needs no blob/filter reads, merge reconstruction or object writes.
The before side binds to the immutable context tree; the after side binds to the
accepted result tree and actual candidate checks. Old partial checks were removed.
A genuine Git-tree ordering control covers nested/Unicode paths and file/directory
prefix ordering. No source bytes are newly stored and no old assertion was weakened.

Final Devenv fmt --check, all-feature locked check and all-target/all-feature locked
clippy -D warnings passed. **107 local tests + 39 affected controls = 146 selected
passing tests**. Existing attribute/ident/same-tree/partial-byte/mode/legacy tests
and all 14 no-Drop child assertions remain green. Exact commands/counts are in the
execution ledger.

Rust changes: `local_sync/{image_proof,recover,run_tests}.rs`. No schema-format,
unsafe filter, dependency, frontend or global result/recovery enum change. Task 7
activation/F2 authority obligations remain. No Task 6, owner question, subagent,
full suite, native run, PR or lifecycle action. Ticket remains implementing;
**SPEC verification then QUALITY re-review are still required**.
