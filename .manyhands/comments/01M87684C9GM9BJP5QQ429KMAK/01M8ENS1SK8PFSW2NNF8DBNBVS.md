---
id: 01M8ENS1SK8PFSW2NNF8DBNBVS
item_id: 01M87684C9GM9BJP5QQ429KMAK
created_at: 2026-11-27T14:15:34.549107842Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Checkpoint after Tasks 7 and 8 (2026-11-27). Main has not moved (8e5d9832), so no merge.

Task 7, the owner's lifecycles in the library (commit fd03a199, test only). One table test over a new document and an edit of a merged document, by merge commit, squash and fast-forward, with a local bare repository as the forge: promote gives the link, the poll leaves it unpromoted, promote again gives the same link, a save is published by promote again, the forge merges and deletes the branch, the poll reports it promoted, prepare answers already_applied, document show gives no link, and the poll's retirement removes the context and the link row. The local primary moves only by the poll's fast-forward. Red against a presence-only content rule (an edit row fails before its merge); green in about 14 s. Review: approved.

Task 8, over SSH (commit 74242070). Cycle 03's one pull-request SSH case gains the document steps (decision 6): promote answers ok with the link and published, promote again already_applied with the same link and current; after each step the server's primary is unchanged and only the document's branch was pushed. The real synchronization child ran under Task 4's new start and push checks and passed first time. Writes the golden document_promote_pull_request_ok and lands the mutation schema with its registrations (E-2, E-8, E-9, E-11). Red against a stub that answers ok without publishing; green in 18.8 s by exact name. Review: approved.

No SSH target has run as a whole; the fast tier and the SSH tier run once at review-ready (Task 10), which now include the six remote targets because Task 4 touched src/repository/remote/.

Next: Task 9, the contract, RFC, audit and skill amendments (D-1 to D-5).