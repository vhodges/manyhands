---
id: 01M82SX5E09WHESPD9BJXVN7P7
item_id: 01M7RDVNZDCCVSMWHDY9503852
created_at: 2026-11-22T23:37:00.650658774Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Phase 3 checkpoint (Tasks 20-23, promote and close): done, reviewed and pushed at a04cb221. Next is Task 24, review-ready: the fast tier and every SSH-harness target once, the merge check against main, and the final whole-branch review, then the owner's stop before ticket close (decision 37).

What Phase 3 built:
- A command migrates the registry once per process, and the registry's open audits only what the command uses (20).
- A remote call's shared-key reads go through one registry connection, and every recheck still reads current rows (20b).
- A close's and a promotion's previews read the item's own facts with no whole-repository observation (21).
- Each exit of a close or a promotion indexes once, over only the contexts it and its children changed (22a).
- The cleanup's worktree checks: when a remote delete is due, a full check before the cleanup reserves the remote, a cheap head-and-status check just before the delete, and the full check inside the removal; without a remote delete, the one full check in the removal. The item's tree is walked once, through the validation cache (22b, R-SDD-73).
- A close or promotion behind a live sync hand-off waits up to 5 s, then answers busy; it answers recovery_required only when the row is stale (decision 34, 22b).
- The CLI contract, the skills and the design are amended to match (23).

Numbers, release build at 0886503e (after the review's fixes), three runs over SSH to localhost from a fresh replica of main. Medians in seconds; Phase 2, Phase 1 and the original baseline are in brackets:
- promote --confirm 4.59 [10.33, 16.44, 94.3]
- close --confirm 4.88 [10.34, 16.50, 94.9]
- promote --prepare 0.43 [0.78, 0.80, 7.0]
- close --prepare 0.40 [0.78, 0.76, 7.1]
- item sync 1.94 [2.47, 7.79, 20.9]
- comment add 1.63 [1.97, 8.04, 35.2]
- comment add's local checkpoint 0.60 [0.62, 0.94, 14.4]
- ticket save 0.13 [0.18, 0.19, 13.9]
- document save 0.19 [0.21, 0.23, 14.1]
- ticket create 0.50 [0.54, 0.52, 14.2]
- index refresh 0.11 [0.13, 0.12, 14.0]
- poll once 2.80 [3.44, 3.53, 17.2]

A first measurement before the review's fixes, at f03d4e91, gave the same picture (close --confirm 5.06, promote --confirm 4.87). Every target in decision 1 is met: local saves under 1 s, item sync and comment add within 2-4 s or under, promote and close under 10 s. SSH connections per command, from sshd's journal, are the same as in Phase 2: item sync 4 (5 when it merges main), comment add 4, promote and close 10 each, poll once 1.

Profiles of a save, a sync, a promote and a close:
- image_proof::audit runs only once per process, at registry open. It appears in no sample of the four commands.
- latest_commit_touching is test-only and not in the release binary.
- No command makes a whole-repository observation. A close made 6 before this phase.
- A confirmed close or promotion previews three times, all item-level: the verb's check, then the runner's two.
- A close now makes 7 full tree walks, down from 13, and one index step.
- What remains in a close is the same tree walked 5 times by its result checks, which bypass the validation cache (about 1.3 s), then 10 connections (about 1.2 s).

Review: a fresh reviewer (opus) found 0 Critical, 1 Important and 13 Minor. The Important one came from my ruling R-SDD-68: a close over a worktree that already held a nested repository in an ignored directory deleted the remote branch, then stopped holding the cleanup's reservation, which refuses every local write, index refresh and poll in the repository until the same request finishes it. R-SDD-73 restores base's answer (the close stops before the delete, holding nothing), so only a nested repository that appears mid-close reaches that stop; the docs now say what such a stop holds. The fix wave also closed an exit's index row that a failed step left in the way, mapped a swallowed error in a child's index step, read an unreadable parent record as a child's, took the registry memo right after the migration, and added tests for operation resume of a sync's hand-off and for E-80's window. A scoped re-review found every finding addressed and nothing new above Minor. The remaining Minors are deferred to the final review.

For the owner at review-ready:
- R-SDD-69: after its wait, the cleanup's reservation behind a still-live hand-off answers cleanup pending recovery_required, because no cleanup answer can carry busy without a JSON change.
- R-SDD-70: the answer changes that come from reusing the per-item writers' wait.
- R-SDD-72: a same-ID retry of a stop at worktree_removing falls under the removal's resume rules.
- R-SDD-73: replaces R-SDD-68, as above. Its cost: a close with a remote delete walks the worktree's ignored directories twice (base four times); one walk over a 35,000-file target/ is about 20 ms warm.
- Review M-1: a CLI --confirm previews three times where decision 5 counted two (R-SDD-63's reading).
- Review M-12: a close behind a stale primary hand-off merges into main, then stops at publication with recovery_required, where Owner question 2 says it is refused while the row is stale.

Named edits this phase are E-66 to E-81. E-74, E-76 and E-77 were made and withdrawn. The review fixes needed none. Each is in the execution log, docs/plans/2026-11-19-fast-writes-execution.md.

Follow-on candidates, not in this plan:
- the result checks' uncached tree walks;
- registry opens outside the key reads (about 100 per sync);
- the local checks repeated in every remote call.

TCP_NODELAY is vh-fx67e.

Tests run so far: each task's own tests, and the SSH cases on its paths singly by exact name (in Task 22, 97, 77 and 30 cases across its rounds; in the review fixes, 55, with 710 library tests in the changed modules), plus the skills drift tests. Due at review-ready (Task 24): the fast tier and every SSH-harness target.