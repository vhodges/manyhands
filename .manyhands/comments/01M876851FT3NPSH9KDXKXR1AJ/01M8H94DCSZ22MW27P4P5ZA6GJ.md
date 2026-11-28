---
id: 01M8H94DCSZ22MW27P4P5ZA6GJ
item_id: 01M876851FT3NPSH9KDXKXR1AJ
created_at: 2026-11-28T14:32:19.020648079Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Checkpoint after Tasks 1 to 3, 2026-11-28 (subagent-driven; implementers Sonnet 5.5 high, one fresh reviewer per task; details in the execution ledger on this branch).

Task 1, the arrival record at the poll's move (f1fe3e22): the item_merges table and its migration (marker wave_06_item_merges_v1, outside the first transaction), item_merges::arrivals over a fast-forward's first-parent range, record (keeping a row whose commit the new one does not descend from, and deleting a deleted document's row), and record_after_move, called by the poll after the primary's FastForwarded. Tests by exact name, red then green: item_merges_migration_creates_its_table_once, arrivals_follow_the_range (20 rows, red against two stubs on the rows the design names), record_keeps_the_newer_arrival, the_poll_records_its_arrivals; E-1. Review clean; 8 Minor parked for the final review.

Task 2, every other move records (fcd98605, 4ccfec7f): repo sync's restart, both integrate_divergence stages and the clean path (one Primary-only gate), a direct close's and promote's integration after apply, and a setup's activation. Tests red by removing each site's call: repo_sync_records_its_arrivals, a_direct_close_records_its_arrival, a_direct_promote_records_its_arrival, a_setup_activation_records_its_arrivals. The clean path is pinned by a row assertion added to the existing SSH case remote_synchronization primary_current_fast_forward_local_ahead (named edit E-33, run alone by exact name, red with the call removed, green restored; no new SSH case, Ruling 2-1). Review: 2 Important fixed in one round; 4 Minor parked.

Task 3, merged_at and a closed ticket's link in reads (5db80123, 8ac876e6): closure.merged_at and a top-level merged_at, filled by show only, in closed_at's format; a closed ticket's pull_request null (decision 10); Merged at: in human show; 24 item goldens gained only merged_at null. Tests: ticket_show_gives_merged_at_on_its_closure, document_show_gives_merged_at, a_closed_ticket_s_link_is_no_longer_offered, an_item_merge_survives_an_index_rebuild, and a human-output test. Two existing tests that compared a shown closed ticket with its list entry now normalise merged_at (E-34 a_completed_closure_reads_closed_from_the_primary_only; E-35 ticket_closure closed_ticket_discoverable_from_both_clones, run alone by exact name). Review: 1 Important fixed in one round; 3 Minor parked.

Static gates clean on every commit. The fast tier and the SSH tier run once, in Task 15. Next: Task 4, the poll disposition catalog.