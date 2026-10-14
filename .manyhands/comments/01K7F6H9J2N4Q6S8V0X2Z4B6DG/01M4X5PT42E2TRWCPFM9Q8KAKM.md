---
manyhands_managed: true
manyhands_kind: comment
id: "01M4X5PT42E2TRWCPFM9Q8KAKM"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DG"
created_at: "2026-10-14T12:29:00Z"
---

Task 8 LOCAL VERIFICATION CHECKPOINT, **IN PROGRESS / controller review pending**.
Task 7 final SPEC `ses_ec64fd841ffe4JHjZ3XJ7I1MOp` and QUALITY
`ses_ec608e461ffeB8pQQSieLJnzhE` APPROVE through exact Task 8 base
`b0f005531879c5ee2806ed13052bdd157df627d7`, as reported by controller/user.
Prior Task 7 review checkboxes are now ticked; historical fixes remain recorded.

Both earlier deferred minors resolved locally: direct-SQL immutable kind tests
preserve entire operation rows across migration/reopen and denied updates,
including legacy NULL→explicit-kind attempts; production trigger unchanged.
Disposition attachment now uses one helper/pass per startup audit or normal
projection path with the same predicates. State suite 61 passed before/after
cleanup; ordered persisted/new/malformed snapshot test passed after cleanup.
First new SQL fixture trial was 0/2 because `interrupted` was incorrectly used
as an outcome; corrected test setup then 2 passed. No fabricated behavioral RED.

Final changed Rust tree passed sequential Devenv check, fmt, strict Clippy,
all-features locked tests and CLI run; **1,765 passed, 0 failed, 1 existing ignored**:
615 library + 779 ordinary integration + 362 SSH cases + 9 doctests. Includes
73 polling, 15 observation, 63 synchronization, 25 merge recovery, 41 comment,
31 fixture and 114 transport cases. CLI exit 0, no window. Both lockfile SHA-256
hashes match entry; exact hashes/full log/ranges in the dated ledger. No Rust
edit after the final gate. Diff/canonical validation repeated before local commit.

[All 12 acceptance rows and property dimensions](../../../docs/plans/2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-execution.md#task-8-local-verification-checkpoint--2026-10-14)
name actual passing tests. Row 6 is **not fully met**: literal permission-denied
unreadability has no named poll preservation test, distinct from proven missing,
unregistered, redirected, foreign and wrong-branch cases. Row 12's local gate
passes; native runtime proof remains pending. Accepted canonical index titles
are intentionally permitted in raw DB/WAL/backup scans, while recovery rows,
diagnostics/output and every other privacy sentinel remain forbidden.

Display/socket available; bounded 90-second Devenv desktop run compiled, launched
the actual app and printed startup greetings without error. Process startup
only, no visual inspection claimed. Prior 73 normal + 73 alias Task 7 runs trusted.

Controller will dispatch SPEC, whole-branch quality and independent whole-branch
review after this return. They are **unrun here**, with no Task 8 tick or final
approved-review comment yet. All five native targets **PENDING AUTHORIZATION**;
manual wiring is not native proof. Open ticket, local checkpoint only; no
subagents, dispatch, push, PR, merge, closure or worktree cleanup.
