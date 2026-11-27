---
id: 01M8EMCE4RNQXKDZKFSPAQFRVM
item_id: 01M87684C9GM9BJP5QQ429KMAK
created_at: 2026-11-27T13:51:12.636951788Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Checkpoint after Tasks 5 and 6 (2026-11-27). Branch head 7705f558; main has not moved (8e5d9832), so no merge.

Task 5, the data, the preview, the way out and the CLI contract (commit 620b0ccf). PullRequestPromotionMutationDto on every finished answer; PullRequestPromotionPreviewDto and its schema with the cli_v1 preview golden; from_stored, resume and replay for the new data (K-18's replay rows); the way out (remote_branch_deleted gives item republish then a new promote; history_unresolved gives item resolve-history); the CLI's retry guard and human lines; the desktop's compile-level arm. Goldens: document_promote_pull_request_preview and _blocked (cli_v1, JSON and text), document_promote_pull_request_primary_holds (mutation_v1, no data). The mutation schema waits for Task 8's golden. RR8's link is built at answer time for a promoted record whose row was withheld (ruling in the ledger). Review: approved, no Critical or Important.

Task 6, the document's link in reads and item sync (commit 293753e0). document show gives a document's link from its active context while outstanding, through a cheaper twin of the content proof that loads no blobs and agrees with it on all 437 rows of the proof's table; item sync gives a document's link, reading the row and never writing it. Review: approved, no Critical or Important.

Tests ran only by exact name, each red first; static gates and the desktop test-support check clean. No SSH target or tier has run. Minor findings deferred to the final review (ledger), among them two test gaps from Task 5 and one cost from Task 6 (item sync of a document runs the full proof before reading the row) that the final review should weigh before merge.

Next: Task 7, the owner's lifecycles end to end in the library.