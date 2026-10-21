---
manyhands_managed: true
manyhands_kind: comment
id: "01M5A29F5AVHDZNAHG3NAV62AS"
item_id: "01M4CC0VMSZ7BRS740C8VSQ5ZK"
created_at: "2026-10-19T12:30:56Z"
---

Task 3 checkpoint: result codes, schemas, isolated CLI harness and first goldens.
Base: `447c28dc85130f39b45ec89cf9a0fe40aee58831`. Owner-authorized commit:
`feat: usage_error and command_unavailable, CLI test harness and help goldens`.

Task 2 controller handoff: **spec PASS, quality PASS, no findings, 48 tests**.
Task 3 ran inline with TDD/apply_patch and self-review. Required main preflight
was already up to date; no base change.

- Added both Input codes and their fixed messages, schema enums and renderer
  constants. The repository change is only the approved no-action match arm.
- Added four closed help/version schemas and the six named existing-test edits.
  No other existing semantic test changed; no additional library gap found.
- Added temporary-home/data/cwd isolation, Git/SSH/GCM environment stripping,
  closed stdin, concurrent pipe readers, 30-second process bound and one-second
  post-exit EOF check. R-5 remains documented; fixture identity is local.
- Real-binary tests cover help/version/bare output, usage and unavailable
  failures in both modes, mutation-only read options, repeated JSON, absent
  data directories and the regular-file startup failure. JSON envelopes and
  data are schema checked; root repository values tie to the enum registry.
- RED: two registry tests failed before codes; expected non-exhaustive match
  failed before the recovery exception; two real-binary tests failed before
  replacing the temporary renderer aliases. GREEN: **323 targeted tests**:
  results 49, read 65, mutation outcome 37, read contract 71, mutation contract
  43, CLI bin 48, CLI integration 10. Zero failures or ignored tests.
- Generated **8 JSON and 2 text** goldens once, read all ten, then verified
  without the update variable. Literal clap resource-list padding is preserved.
  Scoped lib/bin/contract/CLI clippy with warnings denied and fmt check pass.
- Spec/quality self-review PASS, no unresolved findings. Independent controller
  review remains pending; full-suite and native verification are future gates.

Full evidence:
[execution ledger](../../../docs/plans/2026-10-18-wave-03-cli-01-shell-and-inspection-execution.md#task-3-checkpoint--2026-10-19).

Task 4 remains unstarted and this ticket remains open. No subagents, push, PR,
merge into main, rebase, closure or cleanup. Full suite reserved for Task 8.
