---
id: 01M7R2ME34AY1GS1QY6YWJZ4SH
item_id: 01M57SBA7MKR9BZR7PYECSJ05G
created_at: 2026-11-18T19:39:10.992716529Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Superseded by `01M7R2GPPF7F7JRKHX8C5ZZ4PF` (vh-4y6na, user-visible desktop gaps and correctness holes) and `01M7R2GPPF135ZT7M4TZCR09XT` (vh-fdp7z, hygiene, test support and tidy-up). Open items were checked against main at `5d80bc15` on 2026-11-18; the code was read, no tests were run.

**Moved to `01M7R2GPPF7F7JRKHX8C5ZZ4PF`:** `save_ticket` with a document target provisions a document context before it refuses (`src/repository.rs:2503-2570`); the document save already refuses first.

**Moved to `01M7R2GPPF135ZT7M4TZCR09XT`:** the raw-store negative control in the ticket-closure privacy test. The output and row-level controls already cover title, body, endpoint and `closed_by`; only the raw-store file control plants a bare email (`tests/ticket_closure/privacy.rs:217-227`).

**Dropped, with the reason:** the duplicated record logic. `promotion/lifecycle_record.rs` now holds the shared `supersede`, `replacement_guard`, `unfinished_record` and `busy` code. What remains is the `terminal()` helper in three places and the `begin_record` and `refuse_record` skeleton in `promotion/record.rs` and `closure/record.rs`, about 30 to 40 lines. It sits in recovery-critical code and removing it gains little.
