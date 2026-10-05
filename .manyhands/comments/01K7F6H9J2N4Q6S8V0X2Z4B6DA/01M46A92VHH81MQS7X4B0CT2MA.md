---
manyhands_managed: true
manyhands_kind: comment
id: "01M46A92VHH81MQS7X4B0CT2MA"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DA"
created_at: "2026-10-05T15:17:52Z"
---

Native rerun 37330099368 at a112997 passed both Linux targets and macOS ARM. Both Windows targets passed all library, key-material, key-storage, and session tests, confirming the original fixes. A formerly blocked shared-registry test then failed because its fixture path used mixed separators while SQLite stored a normalized native path.

Changed only the private/public registry fixture helpers to join native path components separately. Exact SQL path, fingerprint, and privacy assertions remain unchanged. Focused registry tests: 39 passed. Full local suite: 480 passed, zero failed/ignored; all required Devenv checks and diff checks pass. Next native rerun will confirm both Windows jobs.
