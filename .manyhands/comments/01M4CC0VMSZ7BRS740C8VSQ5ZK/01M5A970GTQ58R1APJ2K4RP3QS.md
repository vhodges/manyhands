---
manyhands_managed: true
manyhands_kind: comment
id: "01M5A970GTQ58R1APJ2K4RP3QS"
item_id: "01M4CC0VMSZ7BRS740C8VSQ5ZK"
created_at: "2026-10-19T14:31:55Z"
---

Whole-branch spec review P3 follow-up, base
`4cafe3641c8c7a7971d3ae3cb6fe528eb18ef90a`.

Confirmed the design's different-COLUMNS help test was missing. Added
`help::human_help_is_identical_at_small_and_large_columns_and_matches_root_golden`:
root/resource/available-command/planned-command help must have identical bytes
at **20** and **200** columns, exit 0 and empty stderr, without creating data.
Both root outputs also match the existing text golden; fixtures are unchanged.

`CliEnv::with_columns(u16)` is a narrow child-only setting. HOME/data/Git
isolation, process environment and existing timeout/EOF handling are preserved.
The test checks the actual child command's COLUMNS entry before invoking it.
RED before wiring that setting: expected COLUMNS=20, found absent (1 failed,
63 filtered). GREEN after wiring: 1 passed, 63 filtered; eight isolated child
invocations. This is a test-setup RED/GREEN, not a production help behavior fix.

Combined P2/P3 verification: **79 bin + 64 CLI = 143 passed**, test-inclusive
clippy with warnings denied and fmt PASS; all through Devenv, locked except fmt.
Self-review found no unresolved scoped issue. Full details and exact commands:
[execution ledger](../../../docs/plans/2026-10-18-wave-03-cli-01-shell-and-inspection-execution.md#whole-branch-spec-review-fixes-p2-and-p3--2026-10-19).
New checkpoint: `fix: escape CLI recovery fields and verify help width independence`.

Controller re-review remains pending; no final-review-ready claim. The reported
unrelated closure-test/full-suite timeout is untouched. No full-suite execution,
library/pre-C1 test edits, subagents, rebase, push, PR, closure or cleanup.
