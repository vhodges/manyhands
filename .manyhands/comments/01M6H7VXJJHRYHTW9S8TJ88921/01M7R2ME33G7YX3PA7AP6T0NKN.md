---
id: 01M7R2ME33G7YX3PA7AP6T0NKN
item_id: 01M6H7VXJJHRYHTW9S8TJ88921
created_at: 2026-11-18T19:45:00.11186697Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Superseded by `01M7R2GPPF7F7JRKHX8C5ZZ4PF` (vh-4y6na, user-visible desktop gaps and correctness holes) and `01M7R2GPPF135ZT7M4TZCR09XT` (vh-fdp7z, hygiene, test support and tidy-up). Open items were checked against main at `5d80bc15` on 2026-11-18; the code was read, no tests were run.

**Done by Wave 04** (see the earlier comment here). Cycle 03b (`01M784Y94Q3AXT6XJX38F5VK1V`): item 1, a share classified once on the drain's `Finished` (stored in `share_results`, so `share_result()` is a map lookup); and the splits of `model/operations.rs` (now 693 lines with `progress`, `recovery`, `pending` and `lifecycle` children) and `view/operations.rs` (now 154 lines). The `stopping` map is also pruned on `Finished`.

**Moved to `01M7R2GPPF135ZT7M4TZCR09XT`:** item 2 (the composer's read-only guard copies the editor's stop list), item 3 (`CommentMutationDto::from_stored` public and delete `comment_of`, a library change), item 5 (`Operations::dismiss` and `Posting::met` pruning), and the test-helper consolidation from item 7.

**Recorded as "not scheduled" in `01M7R2GPPF135ZT7M4TZCR09XT`:** item 4 (composer state off render; no known bug, so it waits for a measured problem), item 6's other splits (`services/worker.rs` is 1564 lines and `services/mod.rs` 1150, both larger than when this was written, and the test files over about 900 lines), and item 7's smaller hygiene points (the request-hook kind assertion, a `ReadKind` for comment-draft claims, gating `AdoptedComment::state`).
