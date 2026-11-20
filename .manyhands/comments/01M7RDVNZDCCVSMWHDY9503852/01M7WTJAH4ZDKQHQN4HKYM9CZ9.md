---
id: 01M7WTJAH4ZDKQHQN4HKYM9CZ9
item_id: 01M7RDVNZDCCVSMWHDY9503852
created_at: 2026-11-20T15:52:57.591792585Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---
Tasks 5b and 6 complete and reviewed (23988c70; 1f588866, 27469330). 5b: the source walks skip what Git ignores and does not track (decision 20); folder-create tests (mutation_bridges folder::, 32) pass. Task 6: every reader applies Table A to both stored shapes; one review fix made relationship targets come from presented copies only. Ran once each: read_items, read_comments, read_status, read_relationships, read_repository, read_boundary, read_contract, discovery_rebuild, lib read and remote state persistence; cli admin::repo_list golden byte-identical.

Task 7 (store each context from its own observation) stopped on two existing tests that were not named edits. Named edit E-15, approved by the owner in chat 2026-11-20 ("approved, apply E-15"), recorded as design decision 21 (f6f3f038): an invalid worktree's problem is stored on its own context row with path '' instead of on the primary's row at .manyhands/worktrees/<id>; reads still present it where they do today. Edited assertions: tests/discovery_rebuild.rs document_worktree_with_a_second_source_for_its_item_is_rejected and tests/read_items.rs only_a_conformity_problem_at_an_item_path_with_no_item_makes_an_entry (stored row only). Task 7 commits 791c9351, 163f0703 are in review.