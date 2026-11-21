---
id: 01M7ZVGGBYJV5FDS3DNMETHKXF
item_id: 01M7RDVNZDCCVSMWHDY9503852
created_at: 2026-11-21T20:07:32.960850406Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Phase 1 checkpoint (Task 15), branch at fb3e77a9. Phase 1 (Tasks 1-15) is implemented, reviewed task by task, reviewed as a phase, and measured. Waiting for the owner's go-ahead before Phase 2.

Measurement: scripts/measure-writes at 64a53e8d, release build, a scratch replica of main at 790a71ed, bare remote over SSH on 127.0.0.1, 3 runs. Medians, with the Task 1 baseline in brackets:

- ticket save 0.189 s [13.9]; document save 0.228 s [14.1]; ticket create 0.523 s [14.2]; comment add, local checkpoint 0.942 s [14.4]. Decision 1's target for local saves, under 1 s, is met.
- index refresh 0.120 s [14.0]; poll once 3.532 s [17.2].
- item sync 7.791 s [20.9]; comment add 8.035 s [35.2]. Target 2-4 s: Phase 2.
- document promote --prepare 0.799 s [7.0], --confirm 16.439 s [94.3]; ticket close --prepare 0.759 s [7.1], --confirm 16.503 s [94.9]. Target under 10 s: Phase 3.

Budgets (instrumented scratch build, medians of 3): one primary observation 181 ms (252 ms cold) against R-6's 300 ms; the pre-commit safety checks that read other items about 29 ms against decision 11's 100 ms; the gitignore check about 13 ms of a 184 ms primary observation (is_path_ignored 10.8 ms, the index parse 2.5 ms), so would_git_ignore() is not needed.

Profiles (gdb sampling): a save shows no latest_commit_touching, no second whole observation, and the image-proof audit in 1 of 377 samples (a registry migration pass). A sync shows none of the three. A close still observes the whole repository about 8 times, from its previews (Phase 3). About half of a comment's local part is a whole-tree blob read in inspect_synchronization_local; follow-on ticket vh-mpvkc (01M7ZT4NBNRVNY3N90WGCCFM7R) narrows that reader to the canonical paths.

Phase 1 review (ad3527dd..64a53e8d): no code defect against Tables A and B, P-2, P-3, P-5 or decisions 2, 4, 10, 20 and 32; goldens unchanged. Two Important findings: a missing two-writer library test (added), and two local-sync rulings that departed from P-3 (the owner kept them as decision 33). Five Minor: false design and RFC sentences (fixed), a racy file that settled was never re-stamped (fixed), the ignore rules and tracked set are not in the basis (recorded as a known limit), and the relationship check's window (to the final review). Fixes re-reviewed: all addressed.

Owner decisions during Phase 1 execution: 24-33 in the design (named edits E-18 to E-25, E-9 amended, needed test edits approved in advance, a context stored after a pending row's commit reads current, local sync under its own lease). Named edits through E-31 are in the execution ledger.

Tests at this checkpoint: per task in the execution ledger. After Task 12 the fast tier's remaining targets, the library tests and both binaries ran once (all green after E-25). Not yet run: the SSH tier and a whole fast-tier run, both due at review-ready (Task 24).

Next: Phase 2 (publishing), Task 16 onward, with the carries in the execution ledger (the retirement checks behind remote index debt, folder create's item, the sync hand-off's observed_at). Two throwaway key lines for localhost measurement are in ~/.ssh/authorized_keys (manyhands-measure-writes-throwaway and -p1); remove both at the end of the ticket.