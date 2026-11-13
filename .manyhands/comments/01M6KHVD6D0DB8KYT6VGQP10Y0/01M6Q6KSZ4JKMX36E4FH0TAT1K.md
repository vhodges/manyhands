---
id: "01M6Q6KSZ4JKMX36E4FH0TAT1K"
item_id: "01M6KHVD6D0DB8KYT6VGQP10Y0"
parent_id: "01M6Q3KFBMDH8NA4FXCME1T0A6"
created_at: "2026-11-06T01:12:24Z"
created_by: "GPT-6.1 Sol <noreply@openai.com>"
manyhands_managed: true
manyhands_kind: comment
---

## A2/A3 two-Medium SPEC correction — same SPEC then QUALITY pending

Controller accepted R1 (same G/endpoint, changed selected source still eligible)
and R2 (`restart=true` skips completed-P conservative recovery) at `47614e5f`.
That commit/history is preserved. Review range remains
`fae6a78faac0ffff2d3cb7071adbd9303b6ff73f..fix-HEAD`, full Task 2 base
`7b1912d152a1ab7440bd17c48a0a544e40e19258`. Fix-HEAD introduces this checkpoint
with subject `fix: retain original handoff configuration and recover completed
restarts`. No approval is recorded; QUALITY/A4/push remain held.

R1: carry exact sealed OriginalPollSource ObservationConfiguration in non-Clone
Capture and use strong matches_owned equality at A3, not today's endpoint-only
replacement. No key/source/path identity enters durable rows/digests/Debug/errors.
Changed sources retain unconfirmed holds without decision writes, then audited
metadata recovery releases the exact holds. Unchanged positive remains eligible.

R2: remove only the completed-source replay path's `!restart` exclusion. Both
spellings conservatively settle lost-capture completed P and replay its immutable
report. Automatic polls, active projected sources and foreign root/child-source
identities remain closed/nonmutating. No reservation/capture/repin/upgrade/child
admission, terminal-P rewrite, producer/schema or downstream authority expansion.

Independent compiled REDs: R1 **0/1** with four same-G/endpoint source changes
wrongly eligible; R2 **2 passed / 1 failed**, both repaired/unrepaired true-flag
cases retained unconfirmed pins. GREEN **4 new** tests, full post-refresh decision
images/refusal, exact recovery-only changes and later ordinary-poll replacement.
Synthetic selected files registered before P, inert A0/A2 projections, real
poll/SQL/refresh—not live source SSH or Task 3/4 producer/enrollment journeys.

Final serial foreground Devenv gates all pass: **36 source / 147 context / 105
retirement SSH / 58 reservation / 81 poll / 70 state / 15 observation SSH / 84
polling SSH / 13 reservation integration / 52 foundation / 44 mutation contract /
12 recovery**, plus whole-tree check/fmt/strict Clippy. Counts overlap; old source
evidence stays historical. Logs `/tmp/opencode/task2-source-handoff-specfix-*.log`.
All **1,721** input hashes match inventory SHA-256
`745d92b3942ac71ccaed99288967d6cbcaf79d19c7feae30a38c30b05dfdaea0`.

E/O/D 33/39/41 updated only. Still owed: same SPEC→QUALITY, A4 ordered consumption/
gap, active-source crash fencing/fresh-ad retry, full cross-holder/window Z matrix,
remaining A2/A3 token/write/ambiguous-commit/second-release/source and replacement/
M/D14/index/history variants, whole 44-row/final Task 2 review, Task 8 and actual
Task 3/4 delivery. No group/full-task completion claim.

ID from supported isolated current CLI only. No agents/push/status/PR/main merge/
closure/native CI/other-worktree/real-profile/key/context/draft actions.
