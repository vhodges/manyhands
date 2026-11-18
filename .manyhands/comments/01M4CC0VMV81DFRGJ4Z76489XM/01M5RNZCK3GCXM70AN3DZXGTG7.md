---
id: 01M5RNZCK3GCXM70AN3DZXGTG7
item_id: 01M4CC0VMV81DFRGJ4Z76489XM
created_at: 2026-10-25T04:44:19Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

## Task 4 SPEC P2 follow-up: actual C3 index-unavailable evidence

Base `329a1436f3671586382ca4c1f41b2c9a8ab8a699`. Verified the review finding:
the prior exit-table reference reached index_unavailable on C1 document list,
then ran C3 rebuild successfully. Its green test did not establish the claimed
C3 refusal evidence.

Added `index_refresh_on_a_corrupt_database_is_index_unavailable` in Task 4's
`tests/cli/authoring_exits.rs` and repointed the row. It removes WAL/SHM before
whole-file database corruption, invokes the real C3 index refresh with a valid
request ID, and asserts index_unavailable/exit 3, exact command/request identity,
index.rebuild recovery and unchanged canonical/worktree snapshot and Git refs.
Completed Task 2's tests remain unchanged.

Red/green: the new command-identity assertion failed with the old C1 invocation
(0 passed / 1 failed / 238 filtered), demonstrating the attribution defect.
Changing the invocation to C3 refresh preserved all assertions and passed the
authoring-exits filter: **11 passed / 0 failed / 228 filtered**.

The changed checkpoint artifact received one complete prescribed gate run.
Check/fmt/clippy, no-feature CLI check and no-GPUI script all passed. Tests:
**182 CLI unit + 239 CLI integration + 71 read-contract + 44 mutation-contract
= 536 passed**, zero failed/ignored/filtered. Prior checkpoint was 535; this
adds exactly one test. No full suite or native CI.

Self-review and checkpoint merge-tree passed; main remains e49f978, already
current. The separate fix commit/push is ticket-only. **SPEC P2 re-review and
independent Part A acceptance remain pending with the controller.** No Task 5,
subagents, production/library/Cargo/schema changes or dogfooding.

Detailed red/green commands, gate counts and review boundaries are in
`docs/plans/2026-10-24-wave-03-cli-03-authoring-and-discovery-execution.md`.
