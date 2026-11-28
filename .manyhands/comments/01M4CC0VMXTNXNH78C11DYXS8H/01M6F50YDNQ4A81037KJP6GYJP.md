---
manyhands_managed: true
manyhands_kind: comment
id: "01M6F50YDNQ4A81037KJP6GYJP"
item_id: "01M4CC0VMXTNXNH78C11DYXS8H"
parent_id: "01M6ERMJFN6MVA9AM79XF95Q1Q"
created_at: "2026-11-02T22:10:36Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Part B checkpoint: decision 33 and the review fixes

Decision 33 is in: while an unfinished parent holds the item, a lifecycle prepare is refused, naming the request to send again, or the resume when no request holds it. The item 0 rows now pin that, for both verbs. Decision 29's guards have rows, and the spec and quality review fixes are applied.

Decision 32 is stopped for a ruling. A context that was never pushed is published by the run's own context child, and cleanup then deletes it. So for that case, a preview that says it will delete the remote branch is accurate. The case that has stored push evidence but no tracking ref would need the run's deletion to change as well, not only the preview.

The full gate passes on the final tree. The ledger's "Part B checkpoint" has the rows and counts.
