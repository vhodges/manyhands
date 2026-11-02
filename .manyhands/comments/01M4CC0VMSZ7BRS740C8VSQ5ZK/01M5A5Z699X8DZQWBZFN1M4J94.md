---
id: 01M5A5Z699X8DZQWBZFN1M4J94
item_id: 01M4CC0VMSZ7BRS740C8VSQ5ZK
created_at: 2026-10-19T13:35:13Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

Task 6 implementation checkpoint, base
`e1a7540502d30e42bb48cc897772e1c0c5691ffc`.

Task 5 controller handoff: **spec PASS, quality PASS, no findings**, with
**73 bin + 44 CLI + 43 mutation-contract = 160 tests** at `e1a7540`. This
supersedes the pending review in the preceding comment.

Implemented only the eight relationship reads: `ticket ready`, `blocked`,
`deps`, `children`, `cycles`, `plan`, `critical-path` and `find`. Each calls its
existing F1 API; repository-first canonical IDs, exact F1 scopes and the safe
existing filter helper are preserved. All 28 available commands now dispatch;
the empty-placeholder test is backed by an actual all-28 dispatch check.

Human output includes ULIDs/slugs, indented dependency trees with repetition,
numbered plan batches followed by unplannable reasons, blocked-ticket reasons
and one line per cycle. Field controls are escaped, snapshot notes are retained,
and empty lists stay silent on stdout.

- **RED:** 13 binary tests failed with `command_unavailable`; five renderer
  tests and two dispatch/placeholder assertions failed before implementation.
- **GREEN:** **78 bin + 58 CLI + 43 mutation-contract = 179 passed**, zero
  failures/ignored, without golden-update mode. Scoped clippy, fmt and diff
  whitespace checks **PASS**. All Rust commands used Devenv, locked except fmt.
- Generated and read **8 JSON goldens**, all byte-identical to F1. Tests pin
  this parity even during regeneration, and check envelope/data schemas.
- Covered direction/depth, document/invalid/absent IDs, resolution precedence,
  uppercase shared slug, empty/unknown slug and 18 readiness/plan filter cases.
  The first generation run found a new-test expectation error: default direction
  is `down` per design/parser, not `up`. Corrected the test; production was right.
- Spec/quality self-review **PASS**, no unresolved findings or library gap.
  Independent controller review is pending. No library/repository, dependency,
  schema, existing-golden or pre-C1 semantic-test edits.

Owner-authorized checkpoint commit: `feat: CLI relationship reads`, with the
required co-author trailer. Task 6 is ready for controller review. Task 7 is
unstarted; full-suite gates remain Task 8's. Ticket remains open. No subagents,
rebase, push, PR, merge into main, closure, cleanup or native workflow.

Details:
[execution ledger](../../../docs/plans/2026-10-18-wave-03-cli-01-shell-and-inspection-execution.md#task-6-checkpoint--2026-10-19).
