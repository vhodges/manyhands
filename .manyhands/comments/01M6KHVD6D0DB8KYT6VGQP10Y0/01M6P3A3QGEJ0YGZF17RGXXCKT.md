---
id: "01M6P3A3QGEJ0YGZF17RGXXCKT"
item_id: "01M6KHVD6D0DB8KYT6VGQP10Y0"
parent_id: "01M6P039AYM6A3TQR2642C42PH"
created_at: "2026-11-05T14:55:25Z"
created_by: "GPT-6.1 Sol <noreply@openai.com>"
manyhands_managed: true
manyhands_kind: comment
---

## Local recovery-core R1–R3 corrections — same SPEC re-review pending

Preserves `65970f4322b9452ed09833d807fbed345717a29e` and controller docs commit
`59cc4a4fa77b64d8be18e69edd7ec79a394fea46`. Review range remains
`7b153deb9ed829c948092f6a855acbf2819612a0..fix-HEAD` (commit introducing this
checkpoint, `fix: preserve retirement recovery evidence and terminal history`).

- **R1:** close release-table attachments and incoming FK dependencies before
  upgrade DDL and during v2 audit. Unknown trigger/index/FK refuses without schema,
  row/rowid, marker or schema_version loss; unrelated schema stays allowed.
- **R2:** direct actual epoch-2 late uninvoked W stop → external all-absence → M
  succeeds under exact stored resumption/older uncertain intent/receipt/epoch and
  absence proof. Old NULL/new changed remain unchanged, W Possible/B None,
  BranchReleased and exact D14/index settlement; no new transport/credentials/
  proof/Git effect. Five tamper and two D14 controls fail closed; reopened replay
  preserves recreated context and new materialization.
- **R3:** fence every exact old O by raising its epoch, but retain terminal
  phase/outcome. Only active Reserved/Advertising/Persisting are terminalized.
  Eight-state valid matrix checks all metadata/projection, stale writes and TX
  rollback; original held-network stale-executor SSH remains green.

Compiled behavioral RED→GREEN for each finding. Final serial-Devenv frozen gates:
check/fmt/strict Clippy exit 0; context **88**, retirement SSH **85**, migration
**5**, recovery **60**, foundation **52**, persistence **70** passed. Logs:
`/tmp/opencode/task2-recovery-specfix-*-final.log`; reds and focused greens use
the same prefix. Original 84/82 and published 76/63 assertions retained. One old
Z receipt fault trigger conflicted with R1; replaced by a cfg(test) fault at the
same INSERT boundary, with unchanged rollback/reopen assertions and added proof
that the fault fired. No test-name schema exemption.

All-767-Rust-file frozen inventory SHA-256:
`f89e76f2e8b0e65aa1526bd290d3b19eb56af0b4a045d14f85918209061795d8`.
Six owned source/test files plus ledger/comment changed. Only affected 44-row
E/O/D annotations updated. Current-F2/mapping, remaining fault/index/history and
source/batch A0–A4/execution-consumption/Z/GC contracts remain Task 2 owed;
Task 3/4 actual producers remain separate. **No full Task 2 completion or review
approval claimed. Same SPEC re-review precedes QUALITY.**

Supported isolated CLI data-dir generated this unique ID; no default-profile
CLI/repair, agents, push/status/PR/main integration/closure/native CI, other-worktree
edits or real context/key/draft actions. Linux-only verification; unchanged broad
remote targets/cleanup/status were not repeated or claimed fresh.
