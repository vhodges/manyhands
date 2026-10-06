---
manyhands_managed: true
manyhands_kind: ticket
id: "01M48S808PF2D8ZWVYM918RK2M"
title: "Wave 03 readiness: editor feasibility and provisional API audit"
type: "exploration"
status: "review-ready"
project: "manyhands"
team: "core"
wave: "03"
---

Retire independent Wave 03 readiness risks while Wave 02 collaboration work
continues. This is a standalone exploration, not Wave 03 Cycle 00, and does not
relax the approved Wave entry gate or reorder its implementation Cycles.

## Authorized Now

The user approved all three planning documents on 2026-10-06 at reviewed
revision `aa51f96`, then explicitly authorized execution using subagent-driven
development on the same date. Execute the approved bounded exploration in this
ticket worktree; production integration, publishing, merge, closure, worktree
cleanup, forks, GPUI upgrades and executable Velotype extraction remain
unauthorized.

## Approved Exploration

- Evaluate pinned Zorite first for one compatible GPUI Kit graph, faithful
  Markdown rich/source editing, host-controlled resources and persistence,
  undo, tables, focus and IME. Allow small host adapters only; stop before GPUI
  upgrades, vendoring, forks or editor-core rewrites. If blocked, source-assess
  Velotype and request approval before executable extraction.
- Map approved CLI/desktop operations to current library entry points and name
  missing adapter/domain capabilities. Mark unfinished Wave 02 mappings as
  provisional and require a final-main audit at the Wave 03 implementation gate.
- Separate Linux feasibility evidence from remaining native-platform evidence
  and record a recommendation, costs, blockers and downstream obligations.

## Infrastructure

- Branch: `manyhands/ticket/01M48S808PF2D8ZWVYM918RK2M`.
- Worktree: `.manyhands/worktrees/01M48S808PF2D8ZWVYM918RK2M`.
- Fetched `origin/main` / local `main`: `29f3f5a25957836a8513cba8a318306e1b928063`.
- Created from that base; in-worktree rebase onto freshly fetched `origin/main`
  was a no-op. Before/after HEAD equal the base; ancestry verification passed.
- CLI ticket operations are not implemented, so canonical files are used.

## Planning Checkpoints

- [x] Create the ticket and canonical branch/worktree; verify current-main base.
- [x] Settle material scope and probe-budget questions.
- [x] Write exploration contract, design and detailed implementation plan.
- [x] Audit plan decisions, evidence claims, risks and cross-document consistency.
- [x] Prepare artifacts for user review and explicit execution authorization.
- [x] Receive user approval of the contract, design and implementation plan.
- [x] Receive explicit execution authorization (subagent-driven development).

## Execution

Progress, task ranges, review findings and verification are tracked in the
[execution ledger](../../../docs/plans/2026-10-06-wave-03-readiness-exploration-execution.md).
The controller records canonical ticket comments at task checkpoints.

### Findings and remaining decision

- [API inventory](../../../docs/research/wave-03-api-audit.md): 106 provisional
  source rows, independently reviewed; final-main re-audit remains required.
- [Zorite feasibility](../../../docs/research/wave-03-editor-feasibility.md): exact
  graph fits Kit, but mandatory load normalization violates source preservation.
- [Velotype assessment](../../../docs/research/wave-03-velotype-assessment.md):
  pinned source also loses original bytes; preservation/GPUI-port/I/O extraction
  work exceeds thin host adapters. Independently reviewed source-only findings.

The six-command baseline passed. Cargo/source/lockfile trial edits are restored;
retained changes are documentation only. Tasks 4–6 were not executed under the
approved stop boundary; no actual editor/native/performance proof is claimed.
Documentation/evidence checks passed; whole-branch independent review is pending.

After reviewing the findings, the user must approve any next bounded candidate
revision/API assessment or extraction/port design. No candidate is selected,
no product fallback is adopted, and no further executable adaptation is
implicitly authorized. The ticket remains lifecycle-open.

## Approved Planning Artifacts

- [Exploration contract](../../../docs/research/wave-03-readiness-exploration.md).
- [Design and decision audit](../../../docs/plans/2026-10-06-wave-03-readiness-exploration-design.md).
- [Detailed implementation plan](../../../docs/plans/2026-10-06-wave-03-readiness-exploration-implementation.md).

The user selected editor plus provisional API audit, small host adapters only,
and source assessment/checkpoint before executable Velotype extraction. These
answers bound the approved exploration. All three documents are approved;
execution using subagent-driven development is now authorized.

## Governing Sources

- [Wave 03 readiness and entry gate](../../../docs/Waves/wave-03-dogfooding.md#entry-gate-and-readiness-work).
- [Desktop/editor RFC](../../../docs/RFC/desktop-information-architecture-and-editor.md).
- [CLI RFC](../../../docs/RFC/cli-contract.md).
- [Runtime RFC](../../../docs/RFC/application-runtime-and-polling.md).
- [Wave 03 decision register](../../../docs/RFC/wave-03-rfc-review.md).

## Exit Boundary

Exploration completion requires review of reproducible, pinned evidence and an
explicit recommendation or documented feasibility blocker. It does not satisfy
the complete Wave 03 native matrix, select an editor without the required
approval, authorize production integration, or close this ticket automatically.
