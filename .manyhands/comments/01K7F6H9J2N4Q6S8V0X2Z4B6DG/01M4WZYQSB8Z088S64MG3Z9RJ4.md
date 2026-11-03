---
id: 01M4WZYQSB8Z088S64MG3Z9RJ4
item_id: 01K7F6H9J2N4Q6S8V0X2Z4B6DG
created_at: 2026-10-14T10:39:59Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---

Task 7 spec approved through 650c827bef529c306bca9b377bb8cfed03efb490,
historical approval. Quality ses_ec608e461ffeB8pQQSieLJnzhE BLOCKED one Important
and two minors; fix ALL three at this clean base, inline/no subagents. Complete
Task 7 base remains 4d89295070715d6a15ced2c08d72aff0dd0ee46f. Corrected increment
requires SPEC re-review before the same quality reviewer's re-review. Task 8 unstarted.

Important reproduced before edits with exact command:
devenv shell -- env TMPDIR=/proc/self/root/tmp cargo test --locked --test
remote_polling poll_materialize_ticket_once => exit 1, 0 passed/1 failed at
found.len() (line 967). Service stores canonical context; fixture compares raw
temporary spelling. Follow merge-recovery's canonical-both-sides comparison and
audit other fixture comparisons. Verify broader affected normal/aliased cases.

Minor 1: sentinel baseline is too late to catch initial-poll deletion/mutation.
Capture/check exact direct OID and symbolic target before/after initial poll.
Minor 2: concatenated file snapshots miss type/permission/field boundaries.
Actual filesystem negative controls precede helper correction; record RED/GREEN.
No production changes, dependencies, push/dispatch/merge/closure/cleanup. Ticket
open, native pending, earlier Task 2/Task 6 minors retained for final review.
