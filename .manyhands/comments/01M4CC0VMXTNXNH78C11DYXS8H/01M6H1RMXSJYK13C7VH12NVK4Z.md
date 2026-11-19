---
manyhands_managed: true
manyhands_kind: comment
id: "01M6H1RMXSJYK13C7VH12NVK4Z"
item_id: "01M4CC0VMXTNXNH78C11DYXS8H"
parent_id: "01M6H013FT1S6RD6MZWP7QK3M7"
created_at: "2026-11-03T15:52:07Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 14a: inventory and recovery tables

Every approved command is now accounted for by a test, and the CLI contract RFC has its recovery tables.

- **Inventory** (`tests/cli/inventory.rs`), from the binary's own `--help --json`: the 41 catalogued mutations equal `bound_commands()` less `context prepare` and `context release`; the 77 verbs equal the RFC taxonomy and the relationships RFC's table, both ways; nothing is planned; each verb's own help answers with its options; the 24 input verbs have one schema and one example each, and the other 53 are listed as taking none; every verb's data has an output schema, and every golden with data matches one.
- **Recovery tables** (RFC "Recovery tables", `tests/cli/recovery_tables.rs`): six families, 133 rows, each held to its golden's code, next action and exit. Every recoverable code of C5's outcome tables has a row; C2-C4's unions have a row or a reason (37). One C5 code has a reason: `poll once`'s `remote_unavailable`, which `mutation_contract` records as having no golden.
- **Goldens added, no behaviour change:** 14 library goldens on cases that already reached the answers, and two CLI goldens for a cancelled local sync and resolution. The refused closure checkpoint merge has its row.
- **The RFC** gains the four context taxonomy rows and a C5 options section, which records that the unconfirmed C5 mutations declare `--prepare` and `--confirm` and answer `not_confirmable`.

No behaviour gap was found. Verified: the two filters, `--test cli` (474), `mutation_contract` (44), each changed SSH case with and without the golden update, fmt and clippy.
