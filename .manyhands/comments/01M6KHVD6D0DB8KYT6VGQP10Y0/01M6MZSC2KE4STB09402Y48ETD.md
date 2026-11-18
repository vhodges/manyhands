---
id: "01M6MZSC2KE4STB09402Y48ETD"
item_id: "01M6KHVD6D0DB8KYT6VGQP10Y0"
parent_id: "01M6MXYMP1EBMM65H2S08T4HPG"
created_at: "2026-11-05T04:34:32Z"
created_by: "GPT-6.1 Sol <noreply@openai.com>"
manyhands_managed: true
manyhands_kind: comment
---

## Limited QUALITY Q1/Q2 corrections — delta/re-review pending

Controller relayed/agreed two Important current defects from QUALITY review at
`fb18edc1990c03de9fa03dc8ec3f09f1d5b30668`. These fixes enforce existing approved
no-follow/audit semantics, not new policy. Full Task 2 base stays `7b1912d1`;
source milestone range extends from `daab73e7` through the additive fix commit.
Fetch/merge in this ticket was already up to date; no history rewrite or push.

**Quality-Q1:** Canonical commondir equality followed redirects. New real unrelated
registration tests reproduce false `AlreadyAbsent` and exact old-intent consumption
for a redirected commondir target and ancestor; the valid relative control passes.
The retirement-only identity chain now no-follow validates the common/inventory/
admin/metadata paths and all commondir/gitdir/gitfile targets before canonical
identity checks and the unchanged shared HEAD inventory. Cleanup/release semantics
are untouched. Exact bytes/types/link targets, old materialization and row images
remain preserved; old refusal remains stable after original admin/gitfile repair.

**Quality-Q2:** Ordinary completed-step UPDATEs could attach an authoring,
publication, comment or other lifecycle checkpoint to a real no-effect refusal.
The audit now requires closed, phase/state/effect/settlement-correlated retirement
steps, including terminal report/envelope correlation. Matrices cover every
LocalStep plus unknown/empty/preserving placeholders without bypassing constraints
or triggers. All registry rows, schema objects and schema version are retained on
startup/replay refusal. Valid absence handoffs and truthful partial-removal fixtures
remain readable; possible intent alone is not observed release. Incomplete guarded-
preservation correlations fail closed until their approved flow is implemented.
No checkpoint classification or recovery/read implementation was changed.

Compile-success behavioral RED → GREEN, logs under `/tmp/opencode/`:

- `task2-quality-q1-{red,green}.log`: **1 passed/2 failed → 3 passed/0 failed**.
- `task2-quality-q2-{red,green}.log`: **1 passed/4 failed → 5 passed/0 failed**.

Frozen serial Devenv final gates:

- `task2-quality-controls-final.log`: **8 passed**.
- `task2-quality-specfix-final.log`: **14 passed**, prior S1/S2/S3/backlink controls.
- `task2-quality-context-final.log`: **58 passed**, previous 50 + 8 tests, not matrix
  groups; `task2-quality-recovery-final.log`: **60 passed**;
  `task2-quality-read-status-final.log`: **24 passed**. All zero failed.
- `task2-quality-{check,fmt,clippy}-final.log`: each exit **0**, full check/fmt and
  strict all-target/all-feature Clippy. Exact commands and frozen fingerprints are
  recorded in the execution ledger; whitespace checks passed.

No unchanged cleanup/broad/proof baseline or SSH rerun, zero-case pass, native CI,
agent, other-worktree edit or real-context/key/profile/draft action. Windows
junction/reparse fixtures are cfg-gated, not executed or cross-compiled on Linux.
Index-only continuation and completed replay do not gain inventory freshness or
new materialization consumption. No stronger arbitrary-external-writer atomicity.

Next: **controller's same SPEC delta-check, then SAME QUALITY reviewer re-review**.
No approval or full Task 2 completion claimed. Initial-absence core only; remaining
44 matrix groups/cases and registered nonzero `mutation_remote retirement_` coverage
remain owed. No Task 3/4 or source/batch/destructive producer extension.
