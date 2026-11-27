---
id: 01M8A02QEQBYH1W74C2T3JEHQ1
item_id: 01M87683QK8E34EYY8KS4VQG9Y
created_at: 2026-11-25T18:39:25.538482287Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Checkpoint after Task 4 (ledger commit 470fad26). Tasks 1 to 4 are done, each with a fresh reviewer: 940b42be the closing state; 925b5dd8 the guard per mode and the save check; 5c6e4538 and 1d581a9c closure immutability in a ticket context's synchronization; 7cf3f9c2 the link record, the pull_request object and the item's pull_request and publication fields.

Tests run (each new test red first, then green, by exact name): read_items closure_state_follows_both_copies, closure_filter_closing_and_open, ticket_show_gives_the_link_and_the_publication; read_relationships a_closing_dependency_blocks_until_the_primary_holds_it; --lib graph, ticket_guard, integration, remote::sync, discovery, cleanup and context_retirement tests named in the ledger; mutation_bridges context::a_closing_ticket_takes_comments_and_saves; the edited tests of E-2, E-3, E-14, E-16, E-17, E-24 (ticket_closure, one case) and E-25 (cli_remote, one case); the read_contract and cli golden tests. Static gates clean at each commit. The fast tier and the SSH tier have not run; they run once at review-ready.

Decisions and rulings since Task 0: ruling 9, E-25 also covers the retirement test's ready_after assertion (the same own-worktree precedence decision 15 reverses); ruling 10, pull_request.schema.json is nullable, and Cycle 02 was told; E-42, a mechanical exclusion in tests/mutation_replay.rs. Task 3 took one fix round (a test that could not fail, and a call-site test). Deferred minor findings are in the ledger for the final review.

Next: Task 5 (already_closing, the preview effects and the blocker), then Task 6 (the primary's closure and the two-clone settlement), then Task 7 with its design checkpoint.