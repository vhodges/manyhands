---
manyhands_managed: true
manyhands_kind: comment
id: "01M4WJP4VF6W2FPCZ7ZHM9229A"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DG"
created_at: "2026-10-14T06:48:06Z"
---

Task 6 spec ses_ec74e7b1cffe52NiF8TdyiLNkR's four findings technically corrected,
awaiting SPEC re-review; QUALITY NOT STARTED. Incremental base
72a6260853fe5f9f49b0342a602f999f990a01bc; complete base
ff43d2c3873e4c9cbd9f9f579c5cdd0ef4c513a7 including a70a2ab and feature commit.
Exact new local correction HEAD returned after commit. No contract/scope amendment.

R1: every active/interrupted automatic restart obeys all four eligibility gates
before re-acquisition, with unchanged owner/policy/outcome; terminal replay alone
bypasses. Explicit restart of a manual ID still bypasses automatic policy.
R2: stored root/op identity and kind/invocation validation precede live config;
terminal replay survives removed selection/config and changed remote/primary with
original report and no attempt/outcome writes. Other-kind/invocation mismatch kept.
R3: capture original report inside the existing batch/safe-point/finish transaction
before ownership release. Original target shadow and committed-change flag survive
later poll replacement/index repair, including cancel/yield/foreign generation
fence. Success/failure refresh uses same captured report; original stops never scan.
Older replay alone gets empty targets. Shared observe/sync calls use identity
callbacks with unchanged checks/acknowledgment/policy logic; fenced reads check epoch.
R4: table+trigger+two replay columns form one all-or-nothing envelope. All 14
recognizable partial shapes refuse unchanged. Genuine pre-envelope legacy time
fallback is frozen once at migration, unknown deadline remains NULL. Modern lost
completion metadata fails read/startup; no coalesce fallback or silent recreation.

Meaningful RED before each finding correction, via Devenv:
- remote::poll::tests::review_restart_auto_gate_: 0 pass/4 fail/604 filtered.
- remote::poll::tests::review_terminal_replay_: 0/1/607.
- remote::poll::tests::review_original_report_: actual race 0/1/607, original
  targets empty after real later poll (initial fixture-dispatch failure separate).
- remote::state::persistence_tests::review_disposition_: 1/3/604, silent pair
  recreation twice and modern-time fallback; genuine-old control passes.
- Additional R3 review_original_fenced_stop_: 0/1/612, lost original IndexPending
  after foreign fence/later scan; corrected without new durable fields.
Full exact commands, GREEN ranges, one fixture assertion correction and justified
repeat after additional R3 regression are in the dated execution ledger.

Final GREEN: 76 poll, 59 state, 40 unchanged reservation, 613 standalone library.
Required sequential devenv shell -- gates check all-features/locked, fmt check,
strict all-target/all-feature Clippy and full all-feature tests PASS. Full count
1,690 passed, 0 failed, 1 existing ignored: 613 library, 779 ordinary integration,
289 unchanged real-SSH, 9 doctests. Final log tool_139039d7b001Ok7TtKnL72SBZD.
Final standalone/CLI log tool_1392a241c001mYnB47hZGCxz5J; CLI exit 0, no window.
15 new library tests in correction; actual local repos/cache transitions/tracking,
no fake transport/sleeps/global config/developer keys/subagents. No Rust change
after final gate, only this canonical bookkeeping. Preservation/source/whitespace/
status/diff/log and scripts/show-ticket schema/render checks before local commit.

Ticket OPEN, spec re-review before quality. Deferred Task 2 SQL-kind minor retained
for final Cycle review. Whole-poll SSH/native exit evidence pending; Task 7 untouched.
No push/PR/native dispatch/merge/closure/worktree removal; D1–D4/owner exceptions
and prior Task 5 approvals unchanged.
