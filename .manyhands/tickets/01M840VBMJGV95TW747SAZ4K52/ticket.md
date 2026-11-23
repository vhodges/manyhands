---
slug: vh-fc17c
deps:
- 01M7RDVNZDCCVSMWHDY9503852
id: 01M840VBMJGV95TW747SAZ4K52
title: 'Index reads: write the presented-problem rule once, and match transient problems by code'
type: task
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Follow-on from vh-f3wts (01M7RDVNZDCCVSMWHDY9503852, fast writes), from its final review. Line numbers are at vh-f3wts's last branch head, d239e9e1, which merges into main as it closes.

One rule decides which stored rows a read presents (Table A of the fast-writes design): nothing in a context the read does not present, and for an item that both the primary and its own worktree's item context hold, only the worktree's copy. That rule is written twice: in the store's check in src/repository.rs (~8599, the comment "What the reads present") and in presented_problems (src/repository/read/items.rs:1099). The two copies differ only for rows that no store writes: one matches the own worktree by item ID and the other by path, and a missing context_id row fails one and passes the other. Merge them into one function that both call.

TRANSIENT_PROBLEM (src/repository.rs:7890) is a SQL fragment that recognises a transient problem partly by its guidance text, so rewording a message would silently change which problems count as transient. Match by problem code instead, or by a stored flag if the code is not enough.

Read nits from the same review:
- presented.next().unwrap_or(false) (read/items.rs ~951) can drop rows silently;
- presents_item_contexts is queried up to three times per read, and its SQL fragment is duplicated;
- DirtyContexts::read runs twice per item read.

Acceptance: a table-driven library test runs the rows Table A names through the one rule. The read_items, read_comments and read_repository targets pass. No output, code or golden changes.