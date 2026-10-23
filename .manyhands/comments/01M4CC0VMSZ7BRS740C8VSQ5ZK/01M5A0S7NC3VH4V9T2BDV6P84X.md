---
id: 01M5A0S7NC3VH4V9T2BDV6P84X
item_id: 01M4CC0VMSZ7BRS740C8VSQ5ZK
created_at: 2026-10-19T12:04:35Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

Task 1 review-fix checkpoint. Controller reports spec PASS and one important
quality finding: malformed authorities containing `@` or `/` passed parsing.
Independently verified against design lines 264–270 and F1's
`stored_authority`/`inspect_host`: F1 rejects those delimiters after startup,
so the CLI must classify these inputs as usage before producing `Action::Run`.

- Base: `bd01cce30dd70ea161c37b01ea3cf0e0c7f98747`.
- Added `user@example.com`, `example.com/path` and explicit-port variants to
  the existing malformed-authority test. It checks both helper `Err` and
  `Action::Usage` for `host inspect`.
- Targeted RED: exact malformed-authority test failed at `user@example.com`,
  1 failed / 23 filtered. Added only `@` and `/` to the unbracketed-host
  rejection check; identical targeted GREEN: 1 passed / 23 filtered.
- Bin regression tests: **24 passed**. Targeted bin clippy with warnings
  denied and fmt check passed. All Rust commands through Devenv, with
  `--locked` except fmt. No full suite run.
- Valid-host pass-through, port and IPv6 handling are preserved. Self-review
  confirms the fix is limited to parsing and its regression cases; no library
  changes. Controller quality re-review remains pending.

Evidence:
[execution ledger](../../../docs/plans/2026-10-18-wave-03-cli-01-shell-and-inspection-execution.md).
New checkpoint: `fix: reject invalid CLI authority delimiters` (no amend).
Task 2 remains unstarted; ticket remains open. No subagents, push, PR, merge,
closure or cleanup.
