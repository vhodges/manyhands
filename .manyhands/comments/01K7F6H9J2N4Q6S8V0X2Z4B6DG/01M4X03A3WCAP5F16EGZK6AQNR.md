---
id: 01M4X03A3WCAP5F16EGZK6AQNR
item_id: 01K7F6H9J2N4Q6S8V0X2Z4B6DG
created_at: 2026-10-14T10:42:29Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---

Meaningful snapshot-helper RED before correction: three new controls use real
ignored/unregistered fixture entries so Git status cannot mask missing filesystem
evidence. Exact sequential commands, each exit 1, 0 passed/1 failed on equal hashes:

- devenv shell -- cargo test --locked --test remote_polling poll_snapshot_detects_type_only_changes
- devenv shell -- cargo test --locked --test remote_polling poll_snapshot_detects_permission_only_changes
- devenv shell -- cargo test --locked --test remote_polling poll_snapshot_detects_name_content_boundary_changes

Empty file vs empty directory has identical permission bits; mode644→600 changes
neither content nor executable status; name/data partitions a+bc vs ab+c collide
without length encoding. Unix modes use cfg(unix); non-Unix readonly control is
compiled for native testing, not claimed run on Linux. All compile and reach the
intended assertion; no setup/compile failure or helper edit before RED.

Important aliased-path reproduction recorded in prior comment. Now correct the
canonical comparison, first-poll sentinels and structural hash. No production/
dependency change, subagents or Task 8; SPEC increment review precedes QUALITY
re-review by ses_ec608e461ffeB8pQQSieLJnzhE. Ticket open, native pending.
