---
manyhands_managed: true
manyhands_kind: comment
id: "01M4DRR8S14848PACY82KWAHSW"
item_id: "01M4CC0VMQ7R15A7M9SPN3KB67"
created_at: "2026-10-08T12:45:31Z"
---

Task 6 checkpoint (comment reads): complete at `540a2a1`, range
`a65fb40..540a2a1`. One read, `list_comments`, returns an item's comment
threads with the author taken from `created_by`. No Git history walk and no
index or write-path change.

Product-owner decision recorded on 2026-10-08: reads list what the index
holds and the indexer is what picks up new files; a short delay is
acceptable. The read therefore does not list the comment directory, and the
design's comment paragraph is amended to match.

Independent review: no blocker, one major finding (a duplicate comment was
returned as conforming) and five minor ones, fixed in `540a2a1`, which was not
re-reviewed.

Evidence through Devenv on Linux at `540a2a1`: `read_comments` 19,
`read_contract` 37, `read_boundary` 15, `read_items` 48 and
`discovery_rebuild` 67 passed with 0 failed (controller); library tests 224
passed, clippy with warnings denied and `cargo fmt --check` exit 0
(implementer). The full suite was not run for this task; it last passed at
`bcbe2e6`.

Rulings and accepted limits are in the execution ledger under "Task 6
rulings". Next: Task 7, status and operation reads.
