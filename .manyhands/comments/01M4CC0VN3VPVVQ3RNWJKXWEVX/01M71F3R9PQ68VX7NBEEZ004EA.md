---
id: 01M71F3R9PQ68VX7NBEEZ004EA
item_id: 01M4CC0VN3VPVVQ3RNWJKXWEVX
created_at: 2026-11-10T00:53:15.469926266Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
D6 Part C checkpoint (Tasks 6 to 9): poll intents, the Checking for updates page, needs-attention, and unsaved work across a bring-in are built and reviewed.

Commits: Task 6 35691ba6, 6106fa5b; Task 7 667aaf9b, 9eee8b82; Task 8 fb2f728a, 6585fe44; Task 9 7ab7674c, 197b60f2, 20e337ad; design amendment 7daf9609; ledger 68624a93. origin/main is unchanged since the Part B checkpoint (d6789cb9); Wave 04 Cycle 01 is still not on main, so its steps in Tasks 8 and 10 are skipped (the ledger records what the Task 8 step will add).

Tests (singly, --exact, 1 test each, pass): the Task 6 to 9 tests listed in the ledger; fmt, check and clippy pass. The fast tier and the SSH tier have not run (they run once, at Task 14).

Owner-facing items:
1. Ruling applied: A4 row 3 now says an unlock failure with no key named gives NoKeySelected and the words "Choose a key" (design commit 7daf9609), per your answer.
2. K-33 (library-shaped, no ticket created, not authorized): a Check now's answer carries an error code and host data but not the failure kind (e.kind). HostVerificationUnavailable (code recovery_required) and ProtocolFailure / RuntimeUninitialized (code internal_error) therefore cannot be told from other causes of those codes. They give no attention, so after Unlock... a Check now that fails on host verification leaves the old key entry in place until the next automatic poll raises the host entry. Closing it would need the library to put e.kind in the envelope. Say if you want a follow-up ticket.
3. Plan omission, built in Task 9: R-23's attempt entry (listing the twelve dispositions, Share... for Unpublished) had no task. Task 9 builds the minimal entry; Tasks 11a and 12 add the retirement and failure rows. Known limit K-37: a failed or interrupted attempt replaces the last summary, so the listing vanishes until the next completed attempt (a Task 12 design call).
4. From earlier: K-28 (a failed reconfiguring request still cancels a poll once), K-29 (Resume on the Task 0 observer row fails), K-30 to K-32, K-34 to K-36 are recorded in the ledger.

Next: Part D, Task 10 (shutdown: cancel, the poll's end seen, Still stopping, worker join).