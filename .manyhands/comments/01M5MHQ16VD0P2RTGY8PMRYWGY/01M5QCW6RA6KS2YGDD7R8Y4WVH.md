---
id: 01M5QCW6RA6KS2YGDD7R8Y4WVH
item_id: 01M5MHQ16VD0P2RTGY8PMRYWGY
parent_id: 01M5QA0S348VQ8B0NSXF5R5CSG
created_at: 2026-10-24T16:46:03Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---

## Owner decision

Answer to question 1: **a. Keep sorted order**, as recommended.

- Replace the read acceptance item with: a comment's and a ticket's unknown
  metadata are reported in key order, whatever their order in the file.
- Add the comment read test for it.
- Switch `take_author` and `UnknownMetadata::from_ticket` to `shift_remove`
  for consistency.

Carry on with the read views and the review.

This answers the question only. It is not approval of the change for merge.

Written by Claude Code at the owner's request in chat.
