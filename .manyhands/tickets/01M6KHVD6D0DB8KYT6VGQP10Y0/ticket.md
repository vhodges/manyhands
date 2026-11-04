---
slug: vh-8hc7g
id: 01M6KHVD6D0DB8KYT6VGQP10Y0
title: Recognize and retire remotely deleted contexts of closed tickets
type: gap
status: implementing
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
When a ticket is closed on primary and its remote context branch has been deleted, other clones keep their local branch and worktree for it, holding an open copy. `poll once` reports the target `remotely_deleted` ("The remote context is absent; local state was preserved") and nothing recognizes that the item is closed, so the stale copy stays and nothing offers a way to retire it.

## Evidence (owner's other machine, 2026-11-04)

After Wave 03 C5 merged and its branches were deleted, a `poll once` there reported `remotely_deleted` for C5 `01M4CC0VMXTNXNH78C11DYXS8H`, D4 `01M4CC0VN1KTQFSVR99378TDHV`, `01M5MK57JKCSQFXBNZ8DBE548K` and `01M669FJR27FRHCNPWAMD6VH4V`. All four are closed on main. The local copies are open copies. Two local tips are contained in main; the two folded tickets' tips are not, because C5 copied their files rather than merging their branches.

## Why it matters

Reads prefer an item's own worktree copy, so on that clone `ticket list` and `ticket ready` can report those tickets open and their dependents blocked (see `01M6K04YQM2JF6SBGPB1ZT6R8T`, reads by the latest copy). The only way out is manual `git worktree remove` and `git branch -D`. `context release` exists in the library but refuses a closed ticket and is no CLI verb.

## Expected

- `poll once` (and D6's automatic polling) recognizes a remotely deleted context whose item is lifecycle-closed on primary, and reports it with its own disposition (for example `closed`) and guidance that it is safe to retire, instead of the generic `remotely_deleted`.
- A deliberate verb retires such a context, in the CLI and the desktop: it removes the local branch and worktree after checking there is no uncommitted or unpublished work beyond what primary holds, and refreshes the index. Automatic cleanup stays out of scope (Wave 03); the verb is explicit.
- Closure is judged by primary's lifecycle fields, not by whether the branch tip is contained in primary.

## Acceptance

Two-clone rows: one clone closes a ticket and deletes its branch; the other polls and sees the new disposition; the retire verb removes the context and reads then report the ticket closed. Rows for a dirty or unpublished local copy (refused, nothing removed), for a folded ticket whose branch was never merged, and for a document promoted elsewhere.

Raised by the owner on 2026-11-04 after Wave 03 C5 (`01M4CC0VMXTNXNH78C11DYXS8H`).

## Planning (2026-11-04)

- [x] Ground the brief and ask the owner about preservation, CLI naming,
  dispositions, desktop timing and read scope.
- [x] Draft the [Cycle contract](../../../docs/Cycles/wave-03-completed-context-retirement.md),
  [design](../../../docs/plans/2026-11-04-completed-context-retirement-design.md) and
  [implementation plan](../../../docs/plans/2026-11-04-completed-context-retirement-implementation.md).
- [x] Complete independent planning review (third round at `5a6136e5`: no
  remaining Blocking, Important or Minor findings in either review).
- [x] Obtain document approval, including Q1/Q2 and named edits E-1–E-10.
- [x] Obtain separate implementation and execution-method authorization.

Owner-approved planning scope additionally includes explicit
`poll once --retire-completed` (not default/background cleanup). The desktop
action is assigned to D6; this ticket supplies the shared binding and CLI.
Read precedence remains with `01M6K04YQM2JF6SBGPB1ZT6R8T`.

## Implementation authorization (2026-11-04)

Owner: “Docs are approved. Implementation is approved as well. Use subagent-drive
development”. Execute sequential bounded tasks with spec and quality reviews;
retain Tasks 2/4's adversarial checkpoints. Real-context deletion, other-branch
pushes, PR creation, main merge/closure and native CI remain separately gated.
