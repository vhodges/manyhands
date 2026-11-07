---
id: "01M6SPJAW4WWAT4NC90T4HE8S6"
item_id: "01M6KHVD6D0DB8KYT6VGQP10Y0"
parent_id: "01M6SNFBVK9WRBE9JV3Q9MYQY5"
created_at: "2026-11-07T00:29:36Z"
created_by: "GPT-6.1 Sol <noreply@openai.com>"
manyhands_managed: true
manyhands_kind: comment
---

## Task 8 bounded corrections — local green, independent review still owed

Correction base **`68b13566763c6b14c47136e8ec8636a23ccfb708`**, fix
**`f9fa2d6c36583125df2fee5b9b2b5c7af4339f4f`**; main remains
`79c3931b01d225ddf3434ae9cd4fc51eb607fd2e`. Sole implementer in the ticket
worktree, no agents/push. D15 and all Task 3–7 bounded approvals/results remain.

- **A / M1, A8/E-1:** private disposition dependency guard refuses unknown
  attachments, external trigger/view references and all incoming FKs before
  affected DDL, conservatively handling ASCII case/quoted names. Exact implicit
  indexes/owner trigger remain valid; old-enum corruption/literal checks unchanged.
  Compiled RED caught unknown-index schema loss before rollback. The existing
  table now checks 28 old/current corrupt/dependent envelopes with foreign keys
  ON, all schema/rowids/payload/owner/disposition rows/schema version unchanged,
  direct refusal before rollback and actual refused public open. Exact table plus
  genuine pre-envelope positive: **2 passed**. Old15/current17 positive guards
  retained. **Essential new assertions pending same-SPEC review**, no 44-row waiver.
- **B:** single exact diagnostic passed; it did **not** reproduce the full-tier
  `PoisonError`. New passing assertion proves the deliberate crash fixture does
  poison its hook mutex. A fresh continuation avoids reusing it; real request-
  bound claim/heartbeat, lease/unrelated/stale-F2 refusals and full before/after
  evidence checks remain. No production poison handling or token mismatch fix.
- **C:** diagnostic confirmed `Some("pin")` reopen `RecoveryRequired`: its unknown
  schema-trigger injection violates strict consumer closure, not valid-envelope
  Z. Replace schema faults with private cfg(test) write-boundary points; every
  result/receipt/pin/batch/journal fault is reached/consumed, and every prior write
  rolls back by full image equality. Reopen, exact release/conditional collection,
  scalar-ID reuse and terminal replay retained. Strict production audits unchanged.

Serialized Devenv exact runs: migration RED **0 passed / 1 failed**; A/O green
**2/0**; B/C diagnostic **1/1** (C pin); corrected B/C **2/0**; strengthened B
**1/0**. Slowest 37.573 s wall. Formatting initially reported owned formatting
differences; after correction, **strict all-target/all-feature Clippy and fmt
passed**. Only formatting followed A/C greens; no broad filter, unchanged-tree
rerun, whole lib/fast/SSH/smoke/native command or fresh `cargo check` was admitted.

Exact names/argv/UTC/source input hashes/exits/times/raw-log hashes are in
`/tmp/opencode/task8-correction-results.json`, logs
`task8-correction-{a-red,a-green,bc-diagnostic,bc-green,b-strengthened,fmt-first,clippy,fmt}.log`.
Execution ledger has root causes and command table; supplemental coverage delta
is `task8-correction-coverage.md`, new manifest `task8-correction-evidence.sha256`.
Original Task 8 logs/manifests are untouched: **1,495 passed / 2 failed**, library
only, remains a failed gate. Binaries/all 27 integrations, slow local-sync,
selected SSH targets and both smokes remain unrun; no whole-feature approval.

Built CLI allocated only this comment ID through Devenv with isolated
`MANYHANDS_DATA_DIR=/tmp/opencode/task2-recovery-core-cli-id-data`; no default
profile/repair, real context/key/draft action, other worktree, main/close or native
dispatch. No owned jobs remain. Ticket stays **implementing**. Return to the same
whole-feature SPEC **`ses_e4cc1a00bffeO0qOz2Do4Qx1Bd`**, then fresh whole-feature
QUALITY; controller owns final required three statics and affected/remaining
gates after review. This is not final gates done or permission to push/PR/merge.
