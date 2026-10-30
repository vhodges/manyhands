---
slug: vh-89tsq
id: 01M67BN4H4F4ZXQG79H2NV7PWS
title: Make the CLI's human output labels and conflict lines consistent
type: task
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Make the CLI's human output read as one voice: one label case, no repeated
conflict block, and `structure` labels that say what happened to a path that
is no item's.

Follow-up from Wave 03 C4 (`01M4CC0VMWEQQ9G4W0DAG7ADPY`). On 2026-10-30 the
owner chose to keep C4's output as it is and to follow up here. This covers:

- C4's Part B review, M5;
- the final CLI review, recommendation 4 and minor 8;
- Task 12b's structure-label ruling.

Start it after C4 merges.

## Today

- **Label case.** The `conflict show` lines and the local-sync data lines are
  lowercase (`operation:`, `state:`, `next action:`, `path:`), as C2's
  metadata lines are. The generic field list used by `comment add`,
  `operation abandon` and the other results is Title Case (`Comment id:`,
  `Pending reason:`, `Action:`). A single `comment add` result can show both.
- **A repeated conflict.** A `comment add` whose publication stopped on a
  conflict prints the nested `Conflict …` fields and also the `conflict:`
  line.
- **`structure` of a non-item path.** It comes from the conflict gate's
  classifier. A text code file edited on both sides reads `binary`, and a file
  both sides added reads `delete`. The C4 RFC notes say what `structure`
  means there.

## Expected behaviour

- One label style across the CLI's human output. Choose it and record it in
  the CLI contract RFC.
- The conflict in a `comment add` result is printed once.
- A non-item path's structure reads as what happened to it. If that needs
  the library's classifier or the `ConflictPathDto` schema to change, say so
  in the plan, and keep JSON compatibility under the CLI contract's rules.

## Acceptance criteria

- The human-output goldens under `tests/fixtures/cli_v1/` are updated, and
  each changed line is reviewed by the owner.
- The CLI contract RFC states the label style.
- JSON output is unchanged unless the plan names a compatible schema change.
