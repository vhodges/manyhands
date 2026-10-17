---
manyhands_managed: true
manyhands_kind: comment
id: "01M4X2Q740QMGYS5E2FCG3BQMF"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DG"
created_at: "2026-10-14T11:28:19Z"
---

Task 7 QUALITY ses_ec608e461ffeB8pQQSieLJnzhE Important and both minors technically
corrected, awaiting SPEC increment re-review then the same worker's quality review.
Historical spec approval through correction base 650c827bef529c306bca9b377bb8cfed03efb490
does not approve this increment. Complete Task 7 base remains
4d89295070715d6a15ced2c08d72aff0dd0ee46f; new local HEAD returned after commit.

Important RED before edits: devenv shell -- env TMPDIR=/proc/self/root/tmp cargo
test --locked --test remote_polling poll_materialize_ticket_once => exit 1,
0 passed/1 failed, found.len() line967. Now canonicalize expected worktree and
observed discovery context, matching existing merge-recovery fixture. Audited
other path comparisons; backup inventory now canonical on both sides, WAL checks
filename only. No raw/canonical string or suffix-based service identity assertion.

Minor1: exact stale direct OID and unrelated symbolic target captured before the
first poll and asserted immediately afterward in every clean/current case.
Legitimate initial tracking tips remain allowed and checked; second complete
all-ref checks remain. No invented RED for already-correct no-prune behavior.

Minor2: explicit entry-type tags, length-delimited names/data/link targets,
directory framing, cfg(unix) permission modes / non-Unix readonly observations.
Three real-filesystem controls under ignored unregistered paths each produced
0 passed/1 failed before helper edits: type-only, permission-only and name/content
boundary changes. They now pass; prior unchanged snapshots remain green.

Final sequential Devenv evidence: three snapshot controls PASS; alias document/
ticket materialization 2 PASS; alias accepted-title privacy indexing 1 PASS.
Entire remote_polling target 73 normal PASS and 73 aliased PASS. Required check
--all-features --locked, fmt --check, clippy --all-targets --all-features --locked
-- -D warnings, test --all-features --locked all PASS, in order. Full 1,763 passed,
0 failed, 1 existing ignored (613 library, 779 ordinary integration, 362 SSH-target
cases, 9 doctests). Detailed exact commands/test names and RED/GREEN ledger retained.
Only four meaningful REDs; no compile/setup/accidental trial failure, Clippy failure
or timeout after them. No Rust edit after final gate.

Only remote_polling.rs and canonical bookkeeping change; no production/support/
dependency/workflow/approved-contract edit. Ticket open, Task 8 unstarted; no
subagents/push/dispatch/merge/closure/cleanup. All native executions pending;
Linux alias verification is not Windows/macOS runtime proof. Earlier Task2 SQL-kind
and Task6 duplicate-disposition minors remain explicitly deferred to final review.
Return for SPEC increment re-review before quality re-review.
