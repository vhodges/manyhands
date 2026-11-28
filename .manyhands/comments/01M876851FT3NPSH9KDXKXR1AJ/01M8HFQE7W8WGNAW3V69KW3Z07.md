---
id: 01M8HFQE7W8WGNAW3V69KW3Z07
item_id: 01M876851FT3NPSH9KDXKXR1AJ
created_at: 2026-11-28T16:27:33.897005824Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Task 7 design checkpoint, approved 2026-11-28, before any code of Part E (the plan's Task 7 Step 0). The approved text is in the execution ledger on this branch, Appendix B (revision 3, verbatim), with the independent check and re-check in Appendix C.

How it was made: an Opus designer wrote the mechanism for every required outcome CC1 to CC31 against the source; a separate Opus checker re-read the source for each claim (2 Blocking, 4 Important, 10 Minor); the designer revised it; the checker re-checked the revision (1 Blocking, 0 Important, 6 Minor: the configuration check compared the registry's primary with the record's, which would have refused every cleanup without a remote); the designer applied that as revision 3, and the orchestrator approved it.

The owner's decision 16 (Cycle document, K-22, published at 038e86fb), from the checkpoint's one stop: after a cleanup's remote delete was observed, or once a cleanup with no publication remote began its local removal, a change of the registry's publication remote or primary (a hand edit of the configuration followed by any poll, remote select, remote remove) leaves the same request answering recovery_required, naming the cause, until the configuration is restored; abandon stays refused there and retire_context's check is not relaxed.

Main choices: both lifecycle tables rebuilt to a v4 schema under wave_06_closure_cleanup_v1 and wave_06_promotion_cleanup_v1 (today's frozen as SCHEMA_V3, every new column nullable or defaulted); the cleanup path as a sibling entry run_cleanup_lifecycle in a new promotion/cleanup_run.rs; the before-send hook passing AfterAdvertisement for every tip and saving remote_deleting only when a delete is sent; cleanup records excluded from written_closure_holder, and a cleanup_holder at the four retirement sites; end_parent_row to end a dead holder's row on an abandoned record and a fenced row at completion; complete_record's owner rules relaxed for a fenced cleanup row only; all 22 PromotionIo implementers listed (E-40); the split across Tasks 7 to 10 so that each commit passes clippy.

Rulings 7-1 to 7-4: the abandon preview schema's action values gain close and promote (decision 11 has abandon reach a cleanup record); operation list shows every unfinished cleanup record (better than K-18 states; K-18 amended at closure); remote/reservation.rs changes only by finish_parent's join for a cleanup record and one compile-only arm that never runs; written_closure_holder's exclusion moves from Task 9 to Task 7 with the RecordPath audit.

Next: Task 7's code (the records, RecordPath::Cleanup and the forced arms), then Tasks 8 to 10.