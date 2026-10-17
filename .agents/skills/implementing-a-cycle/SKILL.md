---
name: implementing-a-cycle
description: Use when implementing or resuming an approved Manyhands Cycle plan, addressing its review or CI failures, or delivering and closing that Cycle after explicit authorization.
---

# Implementing a Cycle

Complete the approved Cycle in its ticket branch/worktree, with evidence that
survives session changes and a clear distinction between local and native CI
verification.

## 1. Resume from evidence

Read current `AGENTS.md`, the ticket/comments, approved Cycle/design/plan, and
any execution ledger. Confirm implementation authorization and the execution
method; preserve approvals and user decisions already given. If approval is
missing, use `starting-a-cycle` to finish the reviewable artifacts first.

**Before new implementation, perform the
[worktree and rebase preflight](../starting-a-cycle/references/worktree-preflight.md).**
Fetch current main and rebase the existing ticket branch onto it even if planning
previously rebased it. A clean, current-base no-op is valid. Reconcile changed
commit IDs and plan assumptions without restarting completed tasks.

Use a durable, plan-specific ledger for task state, commit ranges, decisions,
review findings, and verification. Trust recorded completed work and inspect
contradictory evidence; compaction is not a reason to redispatch completed tasks.

## 2. Execute the approved tasks

Honor the chosen execution method. When subagent-driven development is selected,
use that skill if available: give each fresh implementer a bounded task brief,
global constraints, existing interfaces, owned files, and covering checks. Require
a concise result with changed files, commands/results, risks, and a clear return.
Keep overlapping implementations sequential; independent review follows each
task. The controller coordinates and sends code fixes back to the implementer.
Use inline execution when that is the agreed mode.

Write meaningful regression tests before behavior changes. Preserve existing
contracts, RFC exclusions, privacy rules, and recovery guarantees. Debug failures
from evidence rather than skipping tests. Record small implementation decisions;
return material scope or requirement changes for approval.

Review actual commit ranges from the recorded task base, not merely `HEAD~1`.
Resolve blocking findings and carry deferred findings explicitly into final review.
Avoid duplicate broad test runs without a changed tree, failure, or concrete doubt.
If a worker stalls, retrieve its diff and command state and resume or reassign the
remaining work; do not silently abandon it or redo verified tasks.

At plan checkpoints update the ticket/comments using supported CLI operations,
or the canonical filesystem schema while those commands are unavailable. Record
progress, decisions, blockers, test evidence, and review readiness. Keep the ticket
open through implementation and unresolved review/platform gates.

## 3. Verify and deliver

Follow [verification and delivery](references/verification-and-delivery.md).
Run the required Devenv checks for Rust changes, exercise supported front ends,
and obtain independent final review of the complete branch and deferred findings.
Report native platform limitations honestly; workflow YAML and Linux success do
not prove Windows/macOS behavior.

Carry authorized work through review fixes and CI failures. Push/create a PR,
merge, close the ticket, and remove the worktree only at their applicable
authorization/lifecycle stages. Skill invocation or plan approval alone does not
grant those permissions; existing explicit authorization need not be requested
again. End with concrete links, verified results, and any remaining gates.
