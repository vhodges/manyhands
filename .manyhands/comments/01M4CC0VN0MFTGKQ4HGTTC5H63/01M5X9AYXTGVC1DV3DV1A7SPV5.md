---
manyhands_managed: true
manyhands_kind: comment
id: "01M5X9AYXTGVC1DV3DV1A7SPV5"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-26T23:39:39Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 0: baseline, references and ledger

Implementation has started, subagent-driven. The execution ledger is
`docs/plans/2026-10-24-wave-03-desktop-03-rich-text-and-source-authoring-execution.md`.

- **Base:** HEAD `1f0aed9`, main `0464c42`. Merging main was a no-op, and no
  Baseline Fact's file changed since the design's refresh.
- **Baseline:** check, fmt and clippy pass, as do their `--features desktop`
  runs, the CLI smoke test and the CLI isolation script. The full suite
  passed: 4,529 tests, 0 failed, 1 ignored (already ignored before D3). It
  reached its two-hour budget inside `ticket_closure`, the last target, so
  that target and the doc tests ran separately, as Cycle 10's did.
- **Document Amendments:** all present.
- **Sweep:** five tests outside E-1 to E-29 would break when the item's ID,
  kind and lifecycle move into the details sheet. The owner added **E-30**,
  like E-23: those tests open the sheet first and read the same values
  there. It is applied in Task 6. Three more tests stay as they are, and the
  new item view must keep what they read.
- `manyhands-cli help` is not a subcommand, so the ledger saves both its
  output and `--help`'s for Tasks 1 and 2 to compare.
