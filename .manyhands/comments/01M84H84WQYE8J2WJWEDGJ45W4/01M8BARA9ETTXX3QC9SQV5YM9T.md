---
id: 01M8BARA9ETTXX3QC9SQV5YM9T
item_id: 01M84H84WQYE8J2WJWEDGJ45W4
created_at: 2026-11-26T07:05:13.231736494Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Review-ready. Wave 06 Cycle 02 is implemented on this branch: code at `0e2a5e77`, execution ledger at `74b443da`. Main (`3df52dcb`) is merged and has not moved; `git merge-tree --write-tree HEAD origin/main` is clean.

How it ran: Tasks 0 to 13 subagent-driven (Sonnet 5.5 implementers), each with its own review and the design checkpoints of Tasks 6 to 10; a whole-branch final review (Opus, no Critical or Important finding, ten fix items); fix wave 1 (`9214a5ab`); fix wave 2 (`9ce90e25`) for two Important regressions the first wave introduced (a stale call left its operation row pending; unreadable worktree evidence stalled completion) and a desktop consent test the fast tier caught; and a short documentation round (`0e2a5e77`). Each round had a scoped re-review, and the last found nothing new.

Static gates (check, fmt, clippy -D warnings, all features) clean on `0e2a5e77`.

Fast tier: run 3 on `9ce90e25`, 4914 s under Cycle 03's concurrent builds. Every target green except three scheduler tests in `--bin manyhands` (`unlock_after_cancelled_prompt`, `check_now_failure_becomes_attention`, `user_job_waits_for_retirement`), which pass rerun by exact name (9 s); they failed the same way in run 2 and passed singly then, so I read them as load-timing flakes and did not touch them. The last round changed only text, so the targets that read it at run time were rerun on `0e2a5e77`: `--bin manyhands-cli` 325, `--test cli` 490, `mutation_contract` 47, `read_contract` 74, all passed (296 s).

SSH tier (decision 11), one command at a time: the Cycle's one SSH case `mutation_remote` `a_setup_branch_is_published_activated_changed_and_cancelled` 28 s; `ssh_transport` whole, 124 cases, 141 s; `remote_polling` whole, 86 cases, 240 s; `document_promotion`'s four `guard_delete` cases singly, because Task 5 extracted that guard's decision into a pure function (DISC-3): 6 to 9 s each. All passed. Not run: the other SSH targets, the slow `local_sync` unit tests and native CI, which run after merge.

Named edits to existing tests: E-1 to E-15 as planned, E-16 to E-32 found while building. Each is in the execution ledger with its reason; none removes or weakens an assertion. The ones worth a look: E-28 and E-29 (recovery Table B and a status slice gain the new steps and `cancel_setup`), and E-32 (the desktop consent test's recording double forwards the two new setup reads; `pending_setup` joins B4's allowed sequence as Part J's local read, and the test now asserts `poll_setup` is never called).

Documents: the design is amended at `a662db85` where the build departed from it (the `published_oid` check, the digest's refactor, L10 and L11, Baseline Fact 9's harness note, activation's attempt clearing, the scheduler's read, what stops the finish, G5); the Cycle document at `ef8f1f63` (implementation line, A1 to A14 met, new known limits K-18 to K-24). Both published, neither promoted.

Rulings: every one is in the ledger with what it costs if wrong. The ones you may want to check: R-X (L10 and L11 answer busy at once and the retry finishes, as Part E says, where the table said wait); DISC-1 (a changed setup endpoint answers `recovery_required` in the setup paths only, the shared mapping unchanged); DISC-5 (`repo remove` still refuses for an unrelated unfinished operation); R-P (an unreadable setup record fails the gate closed for gated writes); R-AD (the consent test's allowed list gains `pending_setup`); R-AC and R-AE (two rounds past the one-fix-wave rule, each small); R-AF (one skill wording Minor parked).

Follow-on: the recovery skill's kept-branch bullet says completion keeps a checked-out setup branch, where the finish in fact stops and the poll answers `waiting_for_checkout` (R-AF); the desktop's whole `repo inspect` per read and poll (K-24); the final review's deferred Minors. For whichever of Cycles 02 and 03 lands second: `ticket withdraw-closure` takes `SetupGate::Refused` (R-D) and joins `change.rs`'s race list, its closure steps count as in the way, about ten textual conflicts, and the E-1 count becomes 69.

Waiting on your review. Closing this ticket (which merges), promoting the three Cycle documents and the Wave 06 document's D-6, and dispatching native CI each wait for your go-ahead.