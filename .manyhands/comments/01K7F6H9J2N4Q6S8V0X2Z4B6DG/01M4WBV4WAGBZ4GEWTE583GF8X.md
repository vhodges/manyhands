---
manyhands_managed: true
manyhands_kind: comment
id: "01M4WBV4WAGBZ4GEWTE583GF8X"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DG"
created_at: "2026-10-14T04:48:30Z"
---

Task 6 spec ses_ec74e7b1cffe52NiF8TdyiLNkR BLOCKED four findings; quality not
started. Clean assigned fix base 72a6260853fe5f9f49b0342a602f999f990a01bc,
complete review base ff43d2c3873e4c9cbd9f9f579c5cdd0ef4c513a7 including a70a2ab.
Source investigation verifies automatic restart gate bypass, live-config-first
terminal replay, result reconstruction after reservation release/refresh, and
separate migration probes that recreate lost metadata. Necessary shared transaction
extraction remains within authorization; no owner amendment/contract change.

Sequential Devenv RED before correction:
- cargo test --locked --lib remote::poll::tests::review_restart_auto_gate_:
  0 passed/4 failed/604 filtered, wrong transport error for each policy.
- cargo test --locked --lib remote::poll::tests::review_terminal_replay_:
  0 passed/1 failed/607 filtered, removed publication config fails replay.
- cargo test --locked --lib remote::state::persistence_tests::review_disposition_:
  1 passed/3 failed/604 filtered, two identifiable-loss combinations recreated,
  missing modern completion hidden by fallback; genuine-old control passes.
- cargo test --locked --lib remote::poll::tests::review_original_report_:
  0 passed/1 failed/607 filtered, missing fixture dispatch, NOT yet report-loss RED.
All commands prefixed devenv shell --; full exact commands/evidence in ledger.

The existing public PollAfterCompletion FailurePoint must dispatch its existing
test callback before the deterministic second-poll race can execute. Only that
plumbing is wired next, leaving report behavior unchanged; meaningful actual race
RED then precedes report capture correction. Tests use real local repos/tracking
and panic provider, no fake transport/sleeps/global config/developer keys.
Fixes proceed inline, no subagents/Task 7/outward action. Ticket OPEN, Task 2
SQL-kind minor deferred final; return for SPEC re-review before quality.
