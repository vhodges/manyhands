---
id: "01M6MFK79ZVEMZN6JEEGVZQ4WB"
item_id: "01M6KHVD6D0DB8KYT6VGQP10Y0"
parent_id: "01M6ME9SX9CK5WE4RHNVENBSFQ"
created_at: "2026-11-04T23:57:59Z"
created_by: "GPT-6.1 Sol <noreply@openai.com>"
manyhands_managed: true
manyhands_kind: comment
---

## Task 1 spec-review correction: completion is independent of local refs

Controller relayed and verified **one Important** spec finding at initial head
`5be0de55f0465ee0a814ba06f677e2be79c653d7`: `completed_deleted_context` looked
up a direct local context OID before matching the item kind. Thus a missing or
symbolic ticket ref suppressed valid committed-primary closure recognition.
Design lines 172–178 and 212–222 require that lookup only for document preservation,
not ticket completion. The rest of the spec review passed its examined proof,
migration, compatibility and H2 scope; overall spec approval remains pending.

Source correction committed as `a95f01d34bb47fcc8bac79346d84de45e37c8e4c`,
`fix: recognize closed tickets without local context refs`, with the required
GPT-6.1 Sol coauthor trailer. The unchanged direct-local-OID lookup is now inside
the Document arm. Ticket recognition requires positive committed-local-primary
completion after the existing deletion-evidence checks. No cleanup, repair or
other effects were added, and document preservation guards are unchanged.

The recognition matrix grew from 9 to **13 rows**: missing and symbolic ticket
refs report `Closed`; documents in either state retain `RemotelyDeleted`.
Old worktree/admin registration remain present, and full snapshots preserve
bytes, refs/index and registration. Open/unknown/malformed and other old controls
remain unchanged. All new identity-damage scenarios use temporary repositories.

Verification, all Cargo through Devenv:

- `devenv shell -- cargo test --all-features --locked --lib retirement_recognition`
  before the fix compiled and reached behavioral RED: **2 passed / 1 failed**,
  `closed_missing_local_ref` returned `RemotelyDeleted`, expected `Closed`.
- The same focused command after the minimal fix: **3 passed**, 0 failed,
  including the expanded matrix, guidance and stored DTO compatibility test.
- `devenv shell -- cargo test --all-features --locked --lib repository::remote::`:
  **427 passed**, 0 failed, 307.01 s. Rust sources frozen; no concurrent compilation
  or process-spawn failures.
- `devenv shell -- cargo fmt --check`: exit 0. Staged/unstaged and full source-range
  diff checks passed. Raw logs: `/tmp/opencode/task1-spec-fix-*.log`.

Standalone proof, read-contract, CLI polling and the two H2 SSH journeys were not
duplicated for this small correction: their implementation/format/lifecycle paths
are unchanged, the affected identity cases are covered directly, and the complete
remote library gate reran the poll/classification/migration controls. Their prior
6/72/23/1/1 results are historical initial-Task-1 evidence, not reruns at this head.
No whole suite, check/Clippy gate or native CI; controller retains Task 8's gates.

Narrow correction range:
`5be0de55f0465ee0a814ba06f677e2be79c653d7..a95f01d34bb47fcc8bac79346d84de45e37c8e4c`.
Full source-review range:
`834415b95825f84d15ce00a5586316abcfe1a6f7..a95f01d34bb47fcc8bac79346d84de45e37c8e4c`.
This checkpoint and the [execution ledger](../../../docs/plans/2026-11-04-completed-context-retirement-execution.md)
follow in a documentation-only commit. Re-review must use `834415b9` through the
returned HEAD, **not just HEAD~1**.

Next: controller's same spec reviewer re-reviews the correction, then quality
review. Neither is marked passed by this implementer. Task 2 remains unstarted;
no push, main integration, additional agent or ticket status/body save.
