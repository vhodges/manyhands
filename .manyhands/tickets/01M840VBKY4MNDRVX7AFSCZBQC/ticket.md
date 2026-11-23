---
slug: vh-2nmns
deps:
- 01M7RDVNZDCCVSMWHDY9503852
id: 01M840VBKY4MNDRVX7AFSCZBQC
title: A close or promotion lost during its cleanup can leave the index reading current after the worktree is gone
type: defect
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Follow-on from vh-f3wts (01M7RDVNZDCCVSMWHDY9503852, fast writes), from the scoped re-review of its final fix wave (2026-11-23). Line numbers are at vh-f3wts's last branch head, d239e9e1, which merges into main as it closes.

vh-f3wts's final fix F-1 made a close and a promotion mark the primary dirty before their merge moves main (src/repository/promotion/run.rs, before L::apply). It also made the close checkpoint mark the ticket's context before its ref moves (src/repository/closure/checkpoint.rs, context_moving). A process lost after either step therefore leaves what moved reading dirty. The cleanup that follows marks nothing: src/repository/cleanup.rs removes the worktree (remove_context_worktree ~334, remove_fresh_context_worktree ~352) and deletes the local branch with no dirty mark first.

The window: with no remote, the cleanup reserves nothing, so a store can run between the integration and the cleanup. If that store clears the primary's mark and the process is lost after the removal, the removed context's rows stay in the index and index status reads current until the next refresh, poll or desktop start. Reads still present the primary's copy (src/repository/read/items.rs ~1955-1960) but hide its problems until a refresh, which is already documented. Main at 790a71ed behaved the same.

Proposed: under the lease, mark the item's context (or the primary) dirty before the worktree removal and before the branch deletion, as folder create marks before mkdir and the merge marks before L::apply. Add a failure-injection row for each new pre-mark, and one for F-1's merge pre-mark, which has none.

Acceptance: a library test per window (worktree removal, branch deletion), with an injected failure, shows the context dirty or index status stale until a refresh. The lib modules repository::cleanup, repository::closure and repository::promotion pass. The ticket_closure and document_promotion cases that stop at or resume from cleanup pass, each run singly by name. No new SSH case.