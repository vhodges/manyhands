---
id: 01M5YEBSNAVTE45DEJF9QFP5MF
item_id: 01M57SKVF6TQ3QM6AXJ35JGGSY
parent_id: 01M5XX9Y76ZM2D6FJQKBJS2D13
created_at: 2026-10-27T10:26:43Z
created_by: GPT-6 Astra <noreply@openai.com>
manyhands_managed: true
manyhands_kind: comment
---

## Task 8 QUALITY: two P2 harness corrections awaiting re-review

Controller reports SPEC `ses_e84247053ffeQkt4obeGjIa5Xy` **APPROVED, zero findings**
at **`65541cc48ed00d8f1f6463249924bc70c645dbc6`**. QUALITY
`ses_e84216efcffeqZ4Q0rR0960XJX` raised two P2 findings, both verified in source
before correction. This is the fix/re-review base; its approval is not automatically
extended to this delta.

1. The two Task 8 process tests now share a guarded, deadline-bound child harness
   inside their owned module. A supervisor exclusively owns the isolated test child
   and can kill/reap it independently while the caller blocks. Pipe readiness/release
   barriers remain explicit. Concurrent stdout/stderr capture retains diagnostics;
   timeout, early exit and parent unwind cannot bypass child reaping before fixture
   cleanup. Only the owned test child is terminated.
2. The lease test now captures its complete Git/files/index/refs/authority baseline
   before spawning the lease holder and compares after release/reaping. It still
   compares the exact lease file and all other Git artifacts, avoiding the Windows
   mandatory-lock read without excluding locked paths from evidence.

**Six distinct affected tests pass**, covering seven child scenarios: the two
original process acceptance cases and four new controls for missing readiness,
early exit diagnostics, stalled completion, panic-unwind cleanup, and a real blocked
lease caller released by the independent watchdog. The final two controls additionally
prove lease reacquisition after unwind and positive lease contention before blocking.
Whole-tree Devenv fmt/check/clippy pass. Exact commands/counts are in the ledger.

Only `tests/mutation_replay/local_sync.rs` plus this comment and the ledger changed.
No production or unrelated old support helper was edited. Windows portability is
source-reviewed; local evidence is Linux, with no native Windows/ACL claim.

Await **SPEC delta verification then QUALITY re-review** before Task 9. Ticket stays
implementing. No subagents, PR, lifecycle closure, merge into main, full suite or
native dispatch was performed.
