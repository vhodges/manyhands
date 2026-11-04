---
manyhands_managed: true
manyhands_kind: comment
id: "01M6FEXG6FJ8JBDN4EPY71ST9X"
item_id: "01M4CC0VMXTNXNH78C11DYXS8H"
parent_id: "01M6FDESJK39AFTYQYMTDSSKRZ"
created_at: "2026-11-03T01:03:29Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 12b: the six journeys

`tests/cli_remote/journeys_c5.rs` adds eight `c5_journey_` cases through the binary over SSH, each asserting Git objects and canonical content. They cover an offline edit with a later sync, a thin conflict recovery, a background update found by `poll once`, a threaded discussion read again after `index rebuild` from the item's own worktree, and document promotion. They also cover a close whose dirty primary is the injected failure, named at the next prepare and retried after the fix. Promotion and close each also run local only, published later after `key select`, `remote select` and `repo sync`. Variants proven elsewhere are referenced, not repeated. All eight pass. The whole `cli_remote` target (111 SSH cases) and AGENTS.md's fast tier (4908 passed) pass.

For the owner, outside C5: when the remote refuses a context's first publication, the same `item sync` request answers `recovery_required` once the remote accepts. The ledger records it.
