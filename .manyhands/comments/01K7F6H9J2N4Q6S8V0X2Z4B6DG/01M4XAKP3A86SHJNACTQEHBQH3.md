---
id: 01M4XAKP3A86SHJNACTQEHBQH3
item_id: 01K7F6H9J2N4Q6S8V0X2Z4B6DG
created_at: 2026-10-14T13:55:34Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---

All four whole-branch findings now have meaningful regression RED **before any
production edit**, against exact correction base 7d62da7. R1 unit
`whole_review_materialization_external_git_branch_create_race` fails 0/1:
Linux isolated syscall-boundary observer launches actual external Git before
libgit2 opens its ref lock; injection executes and poll overwrites competing OID.
Additional held external Git update-ref prepared-transaction test fails 0/1
on expected typed refusal; new/existing/same/different/packed/symbolic/dangling
matrix already passes (hardening, no artificial RED).

R2 real-SSH `poll_whole_review_disposition_fault_cancel_keeps_index_pending`
and `poll_whole_review_branch_record_fault_cancel_keeps_index_pending` each
fail 0/1 at actual refresh-required=false after proved surviving FF or branch
creation and failed durable row write. Reopen/cancel/restart/refresh assertions
remain to execute at GREEN. R3 all four named
`poll_whole_review_new_auto_invalid_config_{disabled,paused,recovery_suspended,not_due}`
individually fail 0/1 at expected NotEligible for absent config; each covers all
three absent/malformed/no-publication forms at GREEN with actual TCP/helper/prompt
counters, policy/operation SQL write counters and exact unchanged snapshots.
R4 `poll_whole_review_invalid_config_{automatic,explicit}_retains_backoff`
each fails 0/1 at prior backoff 120 being cleared; each covers all three forms.

All runs use sequential Devenv commands. Preliminary fixture errors recorded
separately: C observer -Werror rejected unchecked write() result (fixed and rerun
to actual overwritten-OID RED); SSH target compile rejected two temporary ref
string borrows (fixed tests and rerun to actual R2/R3/R4 REDs). Neither is
misrepresented as a behavior RED. No secret/debug output, sleeps or fake transport.
Proceed with minimal poll-local fixes; observer/shared synchronization helpers
unchanged. Ticket open, Task 8 unchecked and whole review still BLOCKED pending fixes.
