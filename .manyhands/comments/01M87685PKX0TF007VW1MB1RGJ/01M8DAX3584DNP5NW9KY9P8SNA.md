---
id: 01M8DAX3584DNP5NW9KY9P8SNA
item_id: 01M87685PKX0TF007VW1MB1RGJ
created_at: 2026-11-27T01:47:06.675346707Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Note from Wave 06 Cycle 04 (vh-rphcs), approved by the owner on 2026-11-26 at revision 7 (Cycle document docs/Cycles/wave-06-cycle-04-pull-request-promote.md, item 01M8BXXFPDGS3A5MJBM26JEEX3, not yet promoted). In a pull-request repository, document promote now publishes the document's branch and answers with the pull request link, never committing to or pushing the primary. What this leaves for Cycle 06:

1. New shapes the desktop meets: the preview `pull_request_promotion_preview` and the answer data `pull_request_promotion`. Cycle 04 gives the desktop compile-level arms only (design R-14), so until Cycle 06 its "Approve and merge" in a pull-request repository shows a generic preview and the answer's link is not offered as actions (K-7).

2. `document show` now gives `pull_request` for a document read from its active context while its link is outstanding, and null once the primary holds the document by the content proof (decisions 3 and 9). `item sync` of such a document answers with Cycle 03's `pull_request_link` data (decision 4).

3. "Approve and merge" keeps its words until Cycle 06; in a pull-request repository it runs the pull-request promote (publish the branch, give the link) behind the generic preview. Its wording, preview and link actions are Cycle 06's.

4. `repo inspect`'s listing of outstanding links, for tickets and documents, is Cycle 06's (Cycle 04 decision 12, K-10); until then `document show` and `ticket show` give them.