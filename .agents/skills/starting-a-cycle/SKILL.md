---
name: starting-a-cycle
description: Use when starting or planning a Manyhands Wave/Cycle ticket, preparing its Cycle document, design, and implementation plan for approval, or refreshing a stale planning checkout.
---

# Starting a Cycle

Prepare a reviewable Cycle, design, and implementation plan in the ticket's
existing worktree. The ticket records the whole lifecycle.

## 1. Establish the ticket and current base

Read repository `AGENTS.md`, the user's instructions, and the ticket. Locate the
actual branch/worktree with `git worktree list --porcelain`; the usual names are
`manyhands/ticket/<ticket-id>` and `.manyhands/worktrees/<ticket-id>`.
Reuse them. Do not create a second ticket or replacement worktree for an existing
Cycle. If no ticket exists, create one before writing the implementation plan;
create missing infrastructure only within the user's authorized scope.

**Before editing Cycle documents or code, complete
[the worktree preflight](references/worktree-preflight.md): fetch current main,
rebase the ticket branch onto it from inside the ticket worktree, and verify
ancestry.** Read-only discovery may precede this gate. A pre-existing branch or
an old rebase checkpoint does not establish that it includes current main.

Record the ticket, worktree, branch, fetched base SHA, and before/after rebase
SHAs. Read the rebased `AGENTS.md` again if it changed. Preserve unrelated edits
in the main checkout and all other cycle worktrees.

## 2. Ground the design in the project

Read the relevant `docs/Waves/`, `docs/Cycles/`, `docs/RFC/`, and `docs/plans/`
documents, preceding Cycle evidence, and the affected source and CI configuration.
Separate approved requirements from proposals. Preserve RFC boundaries and name
downstream obligations instead of silently expanding this Cycle.

Identify material ambiguity, dependencies, failure/recovery behavior, compatibility
risks, platform evidence gaps, and test feasibility. Ask focused questions where
answers change the design; continue independent investigation while waiting.
Carry forward decisions and approvals already given in the conversation.

## 3. Write the approval artifacts

Follow adjacent document frontmatter, naming, and cross-link conventions:

- `docs/Cycles/wave-<WW>-cycle-<CC>-<topic>.md`: scope, exclusions, dependencies,
  acceptance criteria, and entry/exit evidence.
- `docs/plans/YYYY-MM-DD-wave-<WW>-cycle-<CC>-<topic>-design.md`: behavior,
  interfaces, data flow, failure handling, alternatives, decisions, and risks.
- `docs/plans/YYYY-MM-DD-wave-<WW>-cycle-<CC>-<topic>-implementation.md`:
  ordered tasks with exact files, contracts, meaningful tests, verification
  commands, dependency order, and ticket checkpoints.

Review consistency across the three documents, existing APIs, and test/CI
capabilities. Flag platform checks that require native runners. Do not describe
future verification as completed evidence.

Use the Manyhands CLI for operations it supports. Otherwise follow the canonical
schema for `.manyhands/tickets/<id>/ticket.md` and
`.manyhands/comments/<id>/<comment-id>.md`; generate valid IDs and UTC timestamps.
Link the artifacts and record decisions, rebase evidence, and plan-ready status.

## Handoff

Present links to the concrete documents, unresolved decisions, and the proposed
execution method. Request approval of the Cycle, design, and plan; do not begin
implementation until that approval and implementation authorization exist.
Use `implementing-a-cycle` for the approved work. Planning approval alone does
not authorize pushing, merging, or cleanup.
