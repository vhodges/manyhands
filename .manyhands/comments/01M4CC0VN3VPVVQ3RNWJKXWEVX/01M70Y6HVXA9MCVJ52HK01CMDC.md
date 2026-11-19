---
id: 01M70Y6HVXA9MCVJ52HK01CMDC
item_id: 01M4CC0VN3VPVVQ3RNWJKXWEVX
created_at: 2026-11-09T19:57:41.836351043Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
D6 Part B checkpoint (Tasks 3 to 5): the automatic poll, its preemption and restart, the in-flight record and the scheduler entity are built and reviewed.

Commits: Task 3 cc56515a, 5a9d1c13; Task 4 e2bbc831, c2f564eb; Task 5 0a28c07a, 1a21b7e6; ledger 9b1a9809; main merged at aa0b3d3b (origin/main d6789cb9, a closed fixture-identity ticket; Wave 04 Cycle 01 is still not on main).

Tests (singly, --exact, 1 test each, pass): the Task 3, 4 and 5 tests listed in the execution ledger; fmt, check and clippy pass. The fast tier and the SSH tier have not run (they run once, at Task 14).

Owner-facing items:
1. Known limit K-29 (changes the picture you accepted for U-9). The Resume button the desktop now offers on remote poll rows also appears on the Task 0 retirement observer row, and Resume on it fails with invalid_input because no request holds it. Task 5 removes the dead-end Resume for the desktop's own automatic-poll rows only; the observer row cannot be told apart before Task 7. Task 12 still has to show the observer row settles; this limit is in addition. No action needed unless you want the observer row handled earlier.
2. Reviewer-found fixes: a hot loop on a repeatedly failing policy read (now bounded), and the host-change scope (now every scheduled repository on that host, per R-11).
3. Handed on, not decided by you: the unreadable in-flight record notice is assigned to Task 7 (the plan had assigned it to no task). A failed reconfiguring request still cancels a poll once (K-28).

Next: Part C, Tasks 6 to 9 (poll intents, Settings page, needs-attention, unsaved work across a bring-in).