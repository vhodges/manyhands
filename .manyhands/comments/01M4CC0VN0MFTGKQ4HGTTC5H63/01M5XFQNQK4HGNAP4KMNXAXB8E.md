---
manyhands_managed: true
manyhands_kind: comment
id: "01M5XFQNQK4HGNAP4KMNXAXB8E"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-27T01:31:27Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 2: context release

Commit `feat: add context release`. It adds the library's `release_context`,
which removes an item's context when it holds no work, under one lease, in two
recorded steps, then tells the index the way saves do. It also adds the
`context release` command with no confirmation, the `release_context` action,
two failure points, abandon's `writes` arm, E-15, E-16 and E-20 to E-22, and
completes the tail of Task 1's last `prepare_` test.

Every row of the design's release table is a test and passes. When the refresh
fails, the release answers `discovery_pending` and offers `operation.resume`,
which completes it: the row's `context_branch_released` step is one a resume
takes up.

Tests: lib `context_release` 24, `cleanup` 47, `recovery` 54, `read::status`
10, `mutation::` 108; mutation_bridges `release_` 16,
`keeps_the_rules_of_the_boundary` 1, `prepare_` 17; read_contract
`every_published_` 2; mutation_contract `every_` 5 and the fixture-directory
test 1. Clippy and fmt are clean. CLI `help` and `--help` match the Task 0
copies.

Ruling P-9 is recorded in the ledger. Blocked on one Verify line: read_contract
`published_enumerations_list_exactly_the_registered_values` fails, and it
already failed at HEAD. Task 1's `context_mutation.schema.json` added a `kind`
enum that no Rust list is tied to. Fixing it needs either a change to a
pre-existing test or a change to Task 1's schema, and neither file is Task 2's
to change. A ruling is needed.
