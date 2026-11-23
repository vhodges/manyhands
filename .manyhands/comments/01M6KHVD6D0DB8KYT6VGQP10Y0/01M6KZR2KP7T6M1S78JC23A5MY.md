---
id: "01M6KZR2KP7T6M1S78JC23A5MY"
item_id: "01M6KHVD6D0DB8KYT6VGQP10Y0"
parent_id: "01M6KZ2PM15ZE3ZSMMVS76YDSB"
created_at: "2026-11-04T20:26:12Z"
created_by: "GPT-6.1 Sol <noreply@openai.com>"
manyhands_managed: true
manyhands_kind: comment
---

## Checkpoint: grounded planning drafts and owner decisions

Read AGENTS.md, this ticket and its brief first. The local ticket worktree and
branch were absent; fetched the existing remote branch and checked it out at the
owner's requested path. Worktree was clean. From inside it, fetched origin and
ran `git merge origin/main`: already up to date. Planning base
`c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`; before/after HEAD
`c0e93f87ada218f26428d5a9387e6729274220cf`. Ancestry check passed, main has no
local-only commits, no rebase. GitHub PR lookup timed out twice; PR state remains
unverified.

Three read-only research briefs grounded source/contracts, publication evidence
and the batch/recovery seams. Their findings were checked against source before
drafting claims. This is not the independent review of the new documents, which
is next. No Cargo build/test, implementation or real-context cleanup has run.

Drafts:

- `docs/Cycles/wave-03-completed-context-retirement.md`
- `docs/plans/2026-11-04-completed-context-retirement-design.md`
- `docs/plans/2026-11-04-completed-context-retirement-implementation.md`

## Owner decision (chat, 2026-11-04)

The Cycle records the full decisions D1–D9. Owner selections/words:

1. “Content proof (Recommended)”: prove local changes represented on primary;
   permit narrow lifecycle differences, preserve unmatched/ambiguous work.
2. “context retire (Recommended)”: separate from existing navigation release.
3. “I like promoted over integegrated but merged might be acceptable to (like
   more than integrated).” We discussed existing software/Manyhands promotion
   terminology; drafts use `closed` / `promoted`, with no new document lifecycle
   marker or claim that mere document presence proves promotion.
4. “Desktop in D6 (Recommended)”. Shared binding and CLI here.
5. “Reads unchanged (Recommended)”. Prove reads after retirement; latest-copy
   selection remains on its own ticket.
6. “Fresh observation.” Owner noted the performance impact and requested an
   optional in-poll retirement flag to save a step. Standalone retirement still
   observes freshly after a previous poll.
7. “Explicit poll flag now (Recommended)”: `poll once --retire-completed` reuses
   its pass's observation. Default/automatic polling never cleans up. This is an
   explicit planning-scope amendment to the brief's blanket poll-cleanup exclusion.
8. “Explicit invocation (Recommended)”: no separate prepare/confirm or force
   override. Not permission for the agent to delete real contexts.
9. “Draft and review (Recommended)”: draft/review on the ticket branch now;
   implementation remains separately gated.

The outline approved for drafting includes ancestry or conservative full-tree-
delta proof, only `closed_at`/`closed_by` semantic differences for an older open
ticket, recoverable worktree-before-branch removal, fixed batch selection, and
old-request/recreated-context protection. Ordinary poll digests/shapes and release
semantics stay compatible.

## Next checkpoint / question for the owner

Review the three drafts independently for source feasibility and contract gaps,
then adversarially re-review recovery revisions. Q1 (the proposed per-target/
mixed-batch exit and recovery table) will be presented with the documents, along
with named test edits E-1–E-9. The proposed execution recommendation is
subagent-driven, sequential by task, but the owner has not chosen a method.

Ticket is `waiting` for planning approval, not implementing. Only ticket-branch
commits/pushes are authorized. Other-branch pushes, a PR, merge, closure, native
CI and any real deletion remain gated. Reply in chat or with a child comment.
