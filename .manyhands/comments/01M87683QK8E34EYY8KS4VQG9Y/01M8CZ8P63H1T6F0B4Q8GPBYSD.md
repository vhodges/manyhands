---
id: 01M8CZ8P63H1T6F0B4Q8GPBYSD
item_id: 01M87683QK8E34EYY8KS4VQG9Y
created_at: 2026-11-26T22:22:55.136007253Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Review-ready, 2026-11-26. Wave 06 Cycle 03 is implemented on branch manyhands/ticket/01M87683QK8E34EYY8KS4VQG9Y: code at c9205359, execution ledger and Cycle document at 8f30688d. Main (6c7937c8, with Cycle 02) is merged and has not moved; git merge-tree --write-tree HEAD origin/main is clean.

How it ran: Tasks 0 to 16 subagent-driven (Sonnet 5.5 implementers). Each task had its own review and a scoped re-review of every fix round. Tasks 7, 8, 9 and 11 also had design checkpoints, checked by an independent checker. The final whole-branch review (Opus) answered with fixes and found nothing Critical. One fix wave (3d577a6b) and a tests-only follow-up (c9205359) each had a scoped re-review, and both came back clean.

Your decisions during implementation are in the Cycle document: decision 16 (R-35: a close whose publication remote changed answers recovery_required with the closure data and the item sync way out) and decision 17 (a pull-request close's answers name the ticket's branch and worktree in scope).

Static gates: check, fmt and clippy -D warnings, all features, clean.

Fast tier: once, at 3d577a6b, 5393 s. I added --no-fail-fast to AGENTS.md's command so that a failure in --bins could not stop the later targets. 30 test binaries, 6545 passed, 3 failed: desktop::services::scheduler::attention_answer_tests::check_now_failure_becomes_attention, attention_tests::unlock_after_cancelled_prompt and loop_tests::user_job_waits_for_retirement. Each passes alone by exact name. They are the three load-timing flakes Cycle 02 recorded before this Cycle merged, and I left them alone. c9205359 changed only test files, and its changed tests ran by exact name.

SSH tier: every SSH-harness target once, at c9205359, 1 h 46 min, all passed: ticket_closure 77 cases (397 s), document_promotion 78 (412 s), mutation_remote 811 (3007 s), ssh_transport 124 (142 s), remote_observation 15 (11 s), remote_synchronization 70 (318 s), remote_merge_recovery 29 (175 s), remote_polling 86 (243 s), cli_remote 167 (770 s), comment_publication 41 (157 s), ssh_fixture 35 (239 s). --lib repository::local_sync:: passed 167 (513 s). Native CI has not run; it runs after merge, on main, when you dispatch it.

The Cycle's one SSH case, a_pull_request_close_publishes_only_the_ticket_branch, runs close, close again, item sync, withdraw and close again against a real server. It checks that every ref update on the server is the ticket branch's, and that replays answer identically. It found a real defect, fixed in Task 14: a same-request replay of a finished pull-request close had answered recovery_required.

Named edits to existing tests: the design's E-1 to E-23 and E-24 to E-48, found while building. Each is in the execution ledger with its reason. None removes or weakens an assertion's intent except where a ruling or your decision changed the behaviour it pinned (E-46 for decision 16, E-48 for decision 17), and each of those is named.

Acceptance A1 to A16 are met; the Cycle document's table cites the test for each. The rulings, each with its cost if wrong, are in the ledger's task sections. The ones you may want to check are 19 and 64a (still open to your overturn), 59 and 60 (the recording values), 67 and 68 (a conflict resolve passing the hold), 71 and 72 (the replay Known Limits K-13 and K-14), and 74 (decision 17 covers the preview too).

Follow-ons, not this Cycle: split run_pull_request_tests.rs; a shared PullRequestLink-from-DTO helper; the abandon answers' way outs (ruling 75); a replay's scope under an unavailable index (ruling 79); the deferred minors in each task section.

Waiting on your review. Each of these waits for your go-ahead: closing this ticket (which merges), ticking Cycle 03's boxes in the Wave 06 document (a fresh save opens a new document branch; promoting it is a separate go-ahead), and dispatching native CI.