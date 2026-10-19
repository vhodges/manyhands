---
manyhands_managed: true
manyhands_kind: comment
id: "01M5A970GRQR3QTKQ247Z1K1XF"
item_id: "01M4CC0VMSZ7BRS740C8VSQ5ZK"
created_at: "2026-10-19T14:31:55Z"
---

Whole-branch spec review P2 follow-up, base
`4cafe3641c8c7a7971d3ae3cb6fe528eb18ef90a`.

Independently confirmed that human recovery argument strings bypassed the
success-metadata field escaper. Moved the unchanged helper into `render.rs`
and reused it for both success metadata and recovery fields. LF/CR/ESC and
other field-owned controls now use the established visible escapes; Unicode,
layout, requested bodies/sources and JSON are preserved.

- Unit RED: `render::tests::recovery_fields_escape_controls_preserving_unicode_and_action_lines`
  failed on raw controls and a forged action line; GREEN: 1 passed, 78 filtered.
  Both human rendering entry points and exact JSON/decoded-value preservation
  are asserted, including tab/C1 controls and ordinary Unicode.
- Unix real-binary RED:
  `admin::recovery_root_controls_are_escaped_in_human_output_but_preserved_in_json`
  failed on human stderr after its JSON checks passed. GREEN: 1 passed,
  62 filtered. A real LF/CR/ESC-bearing root yields `not_repository_root`
  through F1; human recovery is exactly one line, JSON retains the real path.
- Combined P2/P3 regression gate: **79 bin + 64 CLI = 143 passed**. Requested
  test-inclusive clippy and fmt passed, through Devenv with locked dependencies
  (fmt exception). Self-review found no unresolved scoped issue.

Evidence and exact commands:
[execution ledger](../../../docs/plans/2026-10-18-wave-03-cli-01-shell-and-inspection-execution.md#whole-branch-spec-review-fixes-p2-and-p3--2026-10-19).
New checkpoint: `fix: escape CLI recovery fields and verify help width independence`.

Controller re-review is pending; not final-review-ready. The reported unrelated
closure-test/full-suite timeout remains with the controller. No full-suite run,
library/pre-C1 test change, subagent, rebase, push or closure.
