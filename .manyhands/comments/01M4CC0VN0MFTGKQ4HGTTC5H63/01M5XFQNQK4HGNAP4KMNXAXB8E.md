---
manyhands_managed: true
manyhands_kind: comment
id: "01M5XFQNQK4HGNAP4KMNXAXB8E"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-27T01:31:27Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 2: context release

Commits `feat: add context release` and
`fix: address the Task 2 review of context release`.

The library's `release_context` removes an item's context when it holds no
work. It does so under one lease, in two recorded steps, and then tells the
index the way saves do. Alongside it come:

- the `context release` command, with no confirmation;
- the `release_context` action and two failure points;
- abandon's `writes` arm;
- named edits E-15, E-16 and E-20 to E-22;
- the tail of Task 1's last `prepare_` test.

The release table passes in full. When the refresh fails, the release answers
`discovery_pending` and offers `operation.resume`, which completes it: the
row's `context_branch_released` step is one a resume takes up. Sending the
same request again finishes it too.

The owner added E-31. It fixed
`read_contract published_enumerations_list_exactly_the_registered_values`,
which Task 1's `context_mutation.schema.json` had broken. The schema's `kind`
enum is now tied to `ItemDtoKind::ALL` and ordered `["document", "ticket"]`.

The review approved the task. Its fixes:

- a test that the same request finishes a failed hand-off;
- a corrected failure-point doc;
- one presence helper in `cleanup.rs`;
- two test nits.

There is also a ruling on concerns 3 and 4. Steps 4 and 5 discard only a new
row. A re-entered row with no step that refuses, and a row whose prune began
and failed, stay pending.

Tests:

- lib: `context_release` 24, `cleanup` 47, `recovery` 54, `read::status` 10,
  `mutation::` 108;
- mutation_bridges: `release_` 17, `keeps_the_rules_of_the_boundary` 1,
  `prepare_` 17;
- read_contract: the enumeration test 1, `every_published_` 2;
- mutation_contract: `every_` 5, and the fixture-directory test 1.

Clippy and fmt are clean. CLI `help` and `--help` match the Task 0 copies.
Ruling P-9 is in the ledger.
