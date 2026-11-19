---
slug: vh-n8e0g
id: 01M7TEA2RX5YWTAGEG4AQ2MFGY
title: Name the blocker when a lock, not a row, is the holder
type: gap
status: open
project: manyhands
manyhands_managed: true
manyhands_kind: ticket
---
Follow-up from vh-bhepg (concurrent writers: attribute, serialize, name the blocker), closed 2026-11-19 and merged as 02dcdfb3. Design: docs/plans/2026-11-18-concurrent-writers-design.md.

Three places still answer busy without naming a blocker, found by the review-ready runs of vh-bhepg: (1) a timeout of the exclusive index lock while writing request records at acceptance or settlement (src/repository/mutation/records.rs, write_records) answers a plain busy under heavy IO; (2) the write lease records no holder, so when it is held with no row of its own (for example the snapshot lease around a conflict read) the writer can only answer a plain busy, and when it is held beside a leftover row in a lease-holding state (for example created after an error) live_holder can name the leftover as the holder; (3) a persistently unreadable index turns a lease timeout into a full 5 s wait before a plain busy.

Wanted: decide whether the lease and the index lock should record a holder (operation id, request id, pid, host, started_at), so a refused writer can name the real holder without guessing from row states; then make (1) and (3) answer with it, or with a clear reason when there is none. Design goal 4 of vh-bhepg (every busy names the blocker) is the bar.

Acceptance: a two-writer test where the lease or lock is held by a writer with no operation row names that writer in data.blocker; no test depends on load or wall time.