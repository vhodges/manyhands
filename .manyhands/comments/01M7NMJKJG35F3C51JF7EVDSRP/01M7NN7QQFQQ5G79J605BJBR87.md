---
id: 01M7NN7QQFQQ5G79J605BJBR87
item_id: 01M7NMJKJG35F3C51JF7EVDSRP
created_at: 2026-11-17T21:05:05.801244246Z
created_by: Claude Sonnet 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---
## Reproducer leads from the skills work

While writing the interim guidance (ticket vh-ghxpf), the implementer reproduced a stalled `comment add` publication in a scratch repository with a passphrase-less key and an unreachable ssh remote: the comment is saved, publication stays `pending`, and a `synchronize_context` operation is left in `reconciling` with `failure_code` and `next_action` empty. So an interrupted publication or reconcile step (not only two writers at once) can leave exactly the state seen in the incident. When doing the first step of this ticket, test both: two concurrent writers on different items, and a single writer whose publication step is interrupted (unreachable remote, killed process after the fetch). They may be one bug.

For attribution (goal 1): `request show --request-id <id>` for a `comment add` lists only the comment's own local `submit_comment` operation; the later `synchronize_context` operation exists only in the answer's `data.synchronization_id`. A lost answer therefore loses the link from a request to its sync. `request show` should list every operation the request started, and `operation show` should name the request that started it.

For goal 4: `operation list` also does not say which item a `submit_comment` operation belongs to (`item_id` null), which is why an agent could not tell another session's comment operation from its own.

Related, tracked separately as non-goals here: auto-redrive of stale operations, a `request list` verb, and idempotent replay of `comment add` with a reused `comment_id` (answers `internal_error` today, the contract says `occupied_path`).