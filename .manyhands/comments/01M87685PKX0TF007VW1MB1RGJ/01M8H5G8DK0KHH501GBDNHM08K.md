---
id: 01M8H5G8DK0KHH501GBDNHM08K
item_id: 01M87685PKX0TF007VW1MB1RGJ
created_at: 2026-11-28T13:28:52.935874071Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Note from Wave 06 Cycle 05 (vh-ssepw), approved by the owner on 2026-11-28 at revision 4 (Cycle document 01M8G1CX05SK9MY5D9YP67B0RP, design 01M8G1CXKNG56HZ4NFMKNAE3G7, plan 01M8G1CY7XE733CHN0XEF52J78, managed documents on their own branches, not yet promoted). What it gives the desktop, which Cycle 05 gives compile-level arms only:

1. New poll dispositions merged (done on the primary while its remote branch survives) and unheld (a ticket closed on the primary whose branch holds work the primary lacks; a ticket's closed now needs the content proof, in either mode). Until this Cycle the desktop maps them as it maps current. Retirement still acts on closed and promoted only.

2. The poll's data gains merged: the items whose done state arrived in that poll's fast-forward, each with merged_at, and for a ticket made_ready and made_ready_state (current or index_pending).

3. merged_at on a ticket's closure (closure.merged_at) and a top-level merged_at on a document, filled by show only.

4. A closed ticket's pull_request is now null (decision 10): the desktop stops offering a merged ticket's link at once.

5. The new confirmed verb context cleanup (a cleanup-only record of the close or promote lifecycle) with the preview context_cleanup_preview and the data context_cleanup (remote_branch deleted, absent or none); operation abandon now reaches a cleanup record until its delete was observed. A stopped cleanup that holds its reservation blocks the repository's local writes until it is rerun or abandoned, as a stopped direct close does.

6. repo inspect's new problems primary_not_checked_out and primary_dirty, for the repository row's checkout warning; poll once warns only in its human output.

7. Cycle 06's checklist in the Wave 06 document should gain the cleanup action, the two dispositions and the closed ticket's link; Cycle 05's closure does that edit with the owner's go-ahead. A cleanup row whose process is still alive reads live, so abandon and resume answer busy until it exits.