---
id: 01M8H5GNE4YZ0FS23ZW86B6JVN
item_id: 01M6K06B7H8V7347SRM3XGV7CB
created_at: 2026-11-28T13:29:06.101543471Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Note from Wave 06 Cycle 05 (vh-ssepw), approved 2026-11-28: its confirmed context cleanup deletes a surviving remote branch only at a tip the last fetch observed (the tracking ref's value), so a contained context whose remote branch has no tracking ref, this ticket's case, is not covered and stays open. The cleanup's guarded delete reuses the direct close's (PromotionIo::delete, delete_exact under guard_delete), which this ticket's fix could reuse too.