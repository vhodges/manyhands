---
id: 01M7AA1V33M7AHVDDXZRG19R2W
item_id: 01M784Y8C73B39GR4JWENXE5D0
created_at: 2026-11-13T11:17:59.409050719Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
**After review-ready: a review of the branch, and one small fix.** A review of main...branch listed ten findings; each was checked against the code. One fix is pushed, `5f1f9c0b`: Retry on an unreadable repository now notifies the scheduler's observers at once (test red then green; the four repository journeys pass by exact name; fmt, check and clippy exit 0).

**Not changed, for the owner.** (1) Title tries: three Busy answers across a few busy scheduler notifies can spend an item's three tries; spacing them needs a clock, a design change to K-3. (2) A publishing save or comment stays listed until the next check: Known Limit K-1 as approved. (3) After an unreachable then a busy answer the row reads "Can't reach" with the sentence "Checking soon": that follows the design's row 7; say if the label should drop while a retry is queued. The other findings were already Minor follow-ups, or a false positive (the held-repository test case). Details in the ledger's last section.