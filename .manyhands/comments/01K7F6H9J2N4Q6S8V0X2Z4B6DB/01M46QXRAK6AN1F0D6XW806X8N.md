---
manyhands_managed: true
manyhands_kind: comment
id: "01M46QXRAK6AN1F0D6XW806X8N"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DB"
created_at: "2026-10-05T19:16:21Z"
---

Task 3 completed in 04e1100..98998e9 and independently approved for spec and
quality. Host pins, exact approval/CAS, the permanent recovery marker, and
selected-key-only callbacks are implemented. Real SSH tests prove that pins
override matching known_hosts and actual corrupt-database recovery requires
fresh approval after losing a conflicting pin. Default-key fallback is refused;
known_hosts bytes remain unchanged.

82 library tests, 50 recovery tests, and seven real SSH trust cases passed.
Check, format, and covering clippy passed through Devenv. Missing comparable
identity is tested at the policy boundary; real fixture handshakes supply raw
comparable identities. Native persistence/handshake evidence remains pending CI.

Review minor retained for Task 5/final review: replace numeric trust scenario
modes with named cases as coverage grows. Task 4 must consume combined pin/marker
finalization and require observed host plus selected-key submission before
transfer. It now integrates the operation driver and approved unlock behavior.
