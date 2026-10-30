---
manyhands_managed: true
manyhands_kind: comment
id: "01M62YY8PKY0JG92VPWJHDNPC0"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-29T04:33:24Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 22: New-Item Drafts

Commit `53d8b93` (feat: create new items from their drafts), with this
comment and the ledger row in a docs commit.

The library classifies a new item's draft, whose base has neither
observation nor source, as `NotCreated` while its item is missing, and
lets it be restored. The draft registry writes such a draft before it
prepares the item's context with the Create intent, and sends nothing
without a draft store. Save of a never-created item creates it, under the
same save gate, with no initials. Afterwards the draft's base is the item
as read, or the fields sent, unverified, so the new-item marker always
clears. An `occupied_path` that holds the draft's own item turns it into
an ordinary unverified draft, which is reviewed. A create retried after a
lost answer creates one item and completes the draft.

In a tab, Continue editing goes by the draft's classification. A
not-created draft is prepared with Create and opens with no item read.
Any other draft takes the Edit route and the review. Discard releases a
new item's context only when this run made it.

Tests, through Devenv: `--lib drafts::` 55 passed; `--test drafts` 48
passed; with the desktop test support, `drafts` 156, `draft_comparison`
3 and `view::item` 295 passed; the whole desktop binary 970 passed, 0
failed; `services::drafts::create` 15 passed. Both clippy runs, fmt and
`check --all-features` are clean. RED was 13 of 14 new desktop tests and
1 of 4 library tests against stubs. Fourteen mutations were tried, and
each was caught. The four never-created tests of Tasks 16 and 17 were
rewritten over a real new item, as ruled (P-3), with their names and
results kept; no other earlier test changed.
