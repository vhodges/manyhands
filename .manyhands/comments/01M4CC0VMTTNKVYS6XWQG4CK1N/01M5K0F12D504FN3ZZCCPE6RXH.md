---
manyhands_managed: true
manyhands_kind: comment
id: "01M5K0F12D504FN3ZZCCPE6RXH"
item_id: "01M4CC0VMTTNKVYS6XWQG4CK1N"
created_at: "2026-10-22T23:52:40Z"
---

## Task 10 — Host Verbs And Unlock Ready For Controller Review

Base: `2cf87a3f5bccf2ca9ced4883fd4d87ad78689633`. Fresh implementer, no subagents.
Fetch/merge found the ticket branch current with main at `32d938d`. Approved
Part B reviews remain recorded; this assignment covers Task 10 only.

Implemented host observe, approve and replace using unchanged F2 APIs, the
existing mutation driver and terminal credential provider. Added closed input
schemas/examples, human next-command input including IPv6 formatting, JSON and
preview goldens, catalog/grammar updates and the isolated `cli_remote` target.

The cases prove pin-only changes and no publication/reference movement, local
previews, exact replacement/refusals, human/JSON success and no-ops, JSON protected-
key refusal, Unix successful/wrong/rejected unlock and Ctrl-C request release.
Wrong passphrases retain today's `transport_unavailable`/exit 5, one prompt,
no retry and the human hint. Correctly unlocked rejected keys are `unlock_failed`.
Passphrases are scanned in UTF-8/UTF-16LE across streams, transcript, data and home.

The owner's missing-selected-source observation case is cross-platform and proves
`key_unavailable` before connection while preserving missing-source import success.
All staged SSH exit references are now real named cases; the exact 49 F2 / 52
CLI-inclusive audit union is unchanged. Task 9's privacy inventory accounts for
18 verbs using explicit disjoint SSH references and retains its local execution
assertions. Task 11 still owns global evidence and the Part C gate.

**TDD:** before production edits, corrected new-harness fixtures reached **0 passed /
26 failed** on unavailable host behavior. Final **27 cli_remote cases pass**; the
additional case directly checks the bin's IPv6 renderer with C1's parser.

Final task checks through Devenv, Cargo locked except fmt:

- `cargo test --locked --test cli_remote`: **27 passed**.
- `cargo test --locked --test cli input_schemas`: **7 passed**.
- `cargo test --locked --test cli exits`: **5 passed**.
- `cargo test --locked --test cli help`: **9 passed**.
- Formatting, diff whitespace and merge-tree checks passed.

Self-review corrected new-test assumptions about DTO list fields, canonical
fingerprint bytes, F2's wrong-old `invalid_input`, port normalization and the
credential-free URL that remote-add intentionally publishes. No unresolved
finding or library gap; no forbidden library/output-schema/lockfile change.
The execution ledger records the complete evidence and review limitations.

This comment ID came from built CLI `id new --json` using the isolated CLI helper;
the temporary allocation utility was removed before final verification. Task-end
commit uses `feat: add host observe, approve and replace` and the required coauthor.
Independent controller review is pending. No push, PR, closure, cleanup, native CI,
Task 11 implementation or broad gate.
