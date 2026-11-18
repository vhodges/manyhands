---
manyhands_managed: true
manyhands_kind: comment
id: "01M5XBC0DAHVHFWJEMAYJ2MXTE"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-27T00:15:10Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 1: context prepare

Added the `context prepare` command (binding, DTO, schema, two goldens,
conformance row, E-15 to E-18). Every row of the design's outcome table is a
test and passes; the last row's `context release` part waits for Task 2.
Tests: mutation_bridges `prepare_` 17 (fix round 1) and `keeps_the_rules_of_the_boundary` 1; the four `keeps_the_rules_of_the_boundary`
targets pass (remote: 2 SSH cases); mutation_contract `every_` 5 and the fixture
directory test 1; lib `mutation::` 107; clippy and fmt clean. CLI `help` and
`--help` match the Task 0 copies. Concern: a `discovery_pending` answer offers
`request.retry` rather than `operation.resume`, because the pending row keeps
its `worktree_observed` step and a resume leaves such a row to its request.
