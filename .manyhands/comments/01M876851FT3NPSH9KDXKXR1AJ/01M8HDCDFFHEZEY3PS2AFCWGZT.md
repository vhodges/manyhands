---
id: 01M8HDCDFFHEZEY3PS2AFCWGZT
item_id: 01M876851FT3NPSH9KDXKXR1AJ
created_at: 2026-11-28T15:46:35.630694846Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Checkpoint after Tasks 4 to 6, 2026-11-28 (details in the execution ledger on this branch).

Task 4, the disposition catalog (6413f953): POLL_DISPOSITION_SCHEMA_18 frozen from today's catalog, a current catalog with merged and unheld, and migrate_poll_dispositions reading two markers (remote_poll_dispositions_unpublished_v1 and the new wave_06_poll_dispositions_merged_v1) by design Part D's truth table, every refusal before any DDL, inside the registry's first transaction. Test: the_poll_disposition_catalog_upgrades_by_its_truth_table, red before and by row against four stubs; E-14, E-15, E-31; E-36 logged. Review (Opus): approved, 4 Minor parked.

Task 5, detection per context (7c66f6c6): the dispositions merged and unheld at every listed place, the desktop mapping them as current; a cheap pre-filter, then the content proof; a ticket's closed needs the proof, and only a content mismatch gives unheld (decision 8). Tests: detection_per_context (red against today's classification and a skipped proof), an_old_frozen_report_still_decodes; E-6, E-7, E-18, E-19, E-24, E-25, E-29; E-37 and E-38 logged. Ruling 5-1: a ticket mid direct-close may read merged during that window, harmlessly. Review (Opus): approved, 7 Minor parked, one to triage at the final review: the document pre-filter parses most of docs/ on each poll to find the document's path.

Task 6, the poll's merged list (9efb9acb): PollMutationDto.merged from the poll's own arrival rows, with a ticket's made_ready and its state (current or index_pending), for an ordinary poll after its refresh and for a retiring poll frozen with its summary; five-field stored answers still decode. Tests in the lifecycle modules and dto.rs, red first; the 13 poll goldens rewritten by their writers one case at a time (each diff only merged); retirement_cli_authenticated_live_pass (SSH, alone) now asserts six fields. Review: approved, 5 Minor parked.

Static gates clean on every commit; the fast and SSH tiers run once, in Task 15. Next: Task 7, context cleanup's design checkpoint (an Opus designer, then a separate checker; posted here before any code) and the records.