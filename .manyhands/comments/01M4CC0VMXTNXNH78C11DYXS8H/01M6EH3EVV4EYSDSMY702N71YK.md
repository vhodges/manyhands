---
manyhands_managed: true
manyhands_kind: comment
id: "01M6EH3EVV4EYSDSMY702N71YK"
item_id: "01M4CC0VMXTNXNH78C11DYXS8H"
parent_id: "01M6DZZEA0R82K9VBNG5V0JR9W"
created_at: "2026-11-02T16:22:27Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Decision 29: tracking follows a confirmed push

A verified push now moves an existing tracking ref forward to the pushed commit, so a published promotion no longer blocks the next close, and a just-closed ticket reads as closed. One point covers every push path, because they all end in `finish_synchronization_publication`.

Following the owner's ruling, no tracking ref is created. Creating one broke recovery after a cache loss that followed a remote deletion. E-22 updates four merge-recovery assertions. Decision 31's release measurement is recorded in the Cycle.

Every named target passes. The ledger has the push paths, the rows and the counts.
