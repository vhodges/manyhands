---
id: 01M8DAX2D6CGW2GYJY04QJRB3P
item_id: 01M876851FT3NPSH9KDXKXR1AJ
created_at: 2026-11-27T01:46:48.744451284Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Note from Wave 06 Cycle 04 (vh-rphcs), approved by the owner on 2026-11-26 at revision 7 (Cycle document docs/Cycles/wave-06-cycle-04-pull-request-promote.md, item 01M8BXXFPDGS3A5MJBM26JEEX3, not yet promoted). In a pull-request repository, document promote now publishes the document's branch and answers with the pull request link, never committing to or pushing the primary. What this changes for Cycle 05:

1. Detection of a promoted document: `primary_holds_document` (design R-3) is the rule. A document counts as promoted only when the local or the tracking primary holds the branch's content by the content proof, never by presence or ancestry alone, so a squash counts and an edit of a merged document does not count until its own pull request merges.

2. Ending a document's context: Cycle 05's confirmed manual cleanup and its deletion of a surviving remote branch must delete the document's pull_request_links row, as both retirement paths do, and must respect the retirement fence of an unfinished pull-request promotion (Cycle 04 decision 13: the poll's, `context retire`'s and the desktop's retirement skip or refuse a document whose pull-request promotion record is unfinished, as they already do for a ticket whose closure is unfinished).

3. Promote already answers `already_applied` once this clone has seen the merge: at prepare with no data, at confirm with `primary_holds` true after only the synchronization's own merge and push.

4. Edges Cycle 05's cleanup settles: K-2 (after a merge whose remote branch survives, cleanup waits for the confirmed deletion), K-9 (a new edit on such a branch makes the next promote give the first link, whose page opens a new pull request from the same branch), K-12 (after a squash on which the primary moved on, a deleted branch answers `remote_branch_deleted`, and its way out `item republish` would revive a merged branch; the skill points to Cycle 05's manual cleanup), K-4 (no verb abandons a published document context; the manual cleanup adds a third ending) and K-14 (after a stopped promote finishes as `already_applied`, the poll retires only with the root on a clean primary; Cycle 05 adds the warning).