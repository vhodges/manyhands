---
id: 01M5A1FZQYZV2DNHRTBJT9B8MV
item_id: 01M4CC0VMSZ7BRS740C8VSQ5ZK
created_at: 2026-10-19T12:17:01Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

Task 2 implementation checkpoint: startup, rendering, help/version and main.
Base: `371c294195c48d8357146136bd1782db2daf2538`. Owner-authorized checkpoint
commit: `feat: CLI startup, rendering, help and version`.

Controller handoff records Task 1 spec PASS and quality re-review PASS after
the authority-delimiter fix at `371c294`. Task 2 was implemented inline with
TDD and self-review. Fetch/merge of origin/main was already up to date.

- Help/version/usage/planned commands bypass startup; available Run actions
  initialize transport before opening the service. Transport failure returns
  fixed `internal_error`, exit 1, with the required human-only diagnostic.
- Human and JSON outputs, exit classes, recovery/extra lines, short writes and
  write/flush errors are covered. JSON serialization errors never print their
  backend text. Help lists all 72 catalog commands and delivering Cycles;
  JSON DTOs and planned help match the approved design.
- `id new` uses the service's direct DTO and prints only ID plus newline in
  human mode. The other 27 available verbs are explicit placeholders. Usage
  and unavailable codes remain the temporary InvalidInput aliases for Task 3.
- RED: 8 rendering tests failed before implementation, then passed. RED:
  16 startup/dispatch/human tests failed before implementation, with the
  existing 32 passing. GREEN/final: **48 bin tests passed**, zero failures.
- Final bare-call smoke exits 0 and prints root help. Bin clippy with warnings
  denied, fmt check and diff whitespace check pass. Rust commands ran through
  Devenv with locked dependencies (fmt exception); no full suite run.
- Spec/quality self-review found no unresolved findings. Independent Task 2
  controller reviews are pending. Real golden/schema and broken-pipe process
  evidence remain in their later planned tasks.

Full command evidence and decisions:
[execution ledger](../../../docs/plans/2026-10-18-wave-03-cli-01-shell-and-inspection-execution.md#task-2-checkpoint--2026-10-19).

Task 3 remains unstarted and this ticket remains open. No library, repository,
pre-existing test, schema or dependency changes; no subagents, rebase, push,
PR, merge into main, closure or cleanup.
