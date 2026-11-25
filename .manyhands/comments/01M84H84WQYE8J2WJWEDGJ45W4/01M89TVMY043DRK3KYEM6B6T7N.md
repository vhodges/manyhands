---
id: 01M89TVMY043DRK3KYEM6B6T7N
item_id: 01M84H84WQYE8J2WJWEDGJ45W4
created_at: 2026-11-25T17:08:10.691447142Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Task 0 checkpoint, 2026-11-25 (implementation start).

Base: origin/main 3df52dcb; the ticket branch was already up to date at f0de4f24 (merged, never rebased). The execution ledger is docs/plans/2026-11-25-wave-06-cycle-02-pull-request-onboarding-execution.md on this branch, committed at 7e9a2ca7.

Reference re-check (read-only, no cargo run): every cited name exists or is new where the design says so. U-1 holds: the shared nullable repository_setup.schema.json passes the contract checks, so Task 3 does not inline setup. No CI test list changes.

Rulings recorded in the ledger: DISC-1, a changed setup endpoint answers recovery_required as the design says, mapped in the setup paths with the shared mapping unchanged; DISC-2, repo setup-cancel's local row gets a new operation action cancel_setup; DISC-3, Task 5 extracts guard_delete's decision into a pure function with no behaviour change, and the existing SSH cases that exercise guard_delete also run singly at review-ready; DISC-4, completion stores the remote plan as remote selection does; DISC-5, decision 16's "always works" means the setup never blocks repo remove, while an unrelated unfinished operation refuses as today; R-B, finish_accepted moves from Task 7 to Task 6, whose refusal ending needs it; R-K, the setup_pending C5 recovery rows are goldens, because the reasons cap has one free slot.

Named edits added: E-16 to E-19 (a transport test literal, two CLI audit-table consts and counts, and the LOCAL_ACTIONS list), each keeping its assertions' intent.

Tests run: none (Task 0 is reading only). Next: Task 1, the result codes.