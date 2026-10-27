---
id: 01M5WFEQNTFFPE9ZHKK82ABC1C
item_id: 01M57SKVF6TQ3QM6AXJ35JGGSY
created_at: 2026-10-26T16:07:19Z
created_by: GPT-6 Astra <noreply@openai.com>
manyhands_managed: true
manyhands_kind: comment
---

## Task 5: two SPEC image gaps corrected, awaiting verification

Base **`2b9ba0cb72850e478094c310f92b2406c8221acc`**. SPEC verification by
`ses_e87b8a702ffe6HryC0itCXg4H5` found two P1 gaps after the prior QUALITY fixes.
The controller reports the reviewer confirmed the unsafe filter API signatures,
lifetimes, layout and cleanup; that helper is unchanged here.

1. **Attribute transitions and retained images:** accepted operations now persist
   versioned metadata-only baseline/prospective image digests bound to frozen parents,
   tree, paths, OIDs and modes. Private memory-only indexes and empty temporary
   workdirs bind filter previews to the accepted baseline/candidate attribute trees;
   no source file is materialized and no checkout runs in preview. Unchanged blob/mode
   may retain its exact trusted baseline bytes. Changed blobs require a frozen
   prospective image. Fresh foreign tracked/info attributes and forged ident data
   refuse. Complete path-set validation precedes checkout and object import.
2. **Mode-only partial writes:** same-descriptor no-follow byte/mode reads feed a
   separate mode-aware v2 coarse digest. The original v1 field/domain remains
   distinguishable; audited discriminators reject missing/mismatched v2 authority
   and unsupported versions. Digests prove effects, not partial-resume permission.
   Two actual Linux no-Drop cases stop after chmod/recreation but before index write:
   bytes/index/ref remain old, executable mode changes, and retry reports Written /
   Partial while preserving ambiguity until explicit operator baseline restoration.

Both findings had meaningful RED regressions. Attribute CRLF→LF with unchanged blob
initially remained stuck; the corrected mode-window child initially proved a real
mode change while the byte-only progress probe returned false. A further RED caught
an incomplete after-only image set being rejected after, rather than before, checkout.
All now pass. Eight new tests cover the 16-case attribute matrix, forged ident,
foreign attribute inputs, manifest/version loss, privacy, legacy hash separation,
missing after-only authority and mode-only no-Drop. No old assertion was weakened.

Final Devenv fmt --check, all-feature locked check and all-target/all-feature locked
clippy -D warnings passed. **105 local tests + 39 relevant shared/removal/compile-fail
controls = 144 passing tests**. Existing same-tree, filtered, ident, partial-byte,
legacy migration, privacy and zero-transport tests remain green. Local parent tests
now include **14 actual no-Drop exits**; subprocess assertions are not double-counted.
Mode evidence is Linux-only; no Windows/macOS runtime claim.

New files: `local_sync/image_proof.rs` and `local_sync/file_image.rs`. Other edits
remain in the owned local-sync modules plus this ledger/comment checkpoint.
The image schema/auditor is production-active; executable helpers remain narrowly
test-gated. Task 7 must activate production callers/helpers, retain fixture hooks
as test-only, enforce F2 attempt authority and verify the production boundary.

Ticket remains implementing. No owner question, Task 6 work, dependency/frontend
change, subagent, full suite, native run, PR or lifecycle action. Exact commands,
counts, staging/legacy limits and corrections are in the execution ledger.
**SPEC verification then QUALITY re-review remain required.**
