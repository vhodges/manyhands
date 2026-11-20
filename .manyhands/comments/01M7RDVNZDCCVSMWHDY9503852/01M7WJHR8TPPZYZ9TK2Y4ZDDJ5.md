---
id: 01M7WJHR8TPPZYZ9TK2Y4ZDDJ5
item_id: 01M7RDVNZDCCVSMWHDY9503852
created_at: 2026-11-20T13:32:50.008154685Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---
Tasks 4 and 5 complete and reviewed (9bac3757, e80611eb; published). Task 4: item activity from a short walk of the item's own branch that stops at the first commit touching its files; an incremental per-path map for main. Task 5: each context (main, or one item's worktree) observed on its own with a stat basis that records each walked directory's relevant entries, so a swap file or a rename-on-save of a source file no longer counts as a change. Tests re-run by exact name: Task 4's activity table and the five existing activity tests; Task 5's basis table, the six observe_root shape tests, the two poll shape tests, the six spec4 tests with E-11, and 50 non-SSH observation-hook and refresh-persist tests (all pass). SSH-harness users wait for the review-ready gate.

Question 8, answered by the owner in chat 2026-11-20: "It should ignore gitignored files". Task 5 found that the managed-marker walk descends into target/ (the live index lists source_unreadable target/f3wts/debug/incremental), so from Task 7 a build would keep main's basis moving and refresh busy. Recorded as design decision 20 and in the ticket's Acceptance (ba572a79): a source walk neither reads nor records a file or directory that Git ignores and does not track; a tracked file is read even if a pattern matches it; listing the contexts under .manyhands/worktrees/ is unchanged. Built next as Task 5b, before Task 6.