---
title: "Wave 02 Cycle 07: Comment Publication"
date: 2026-10-07
status: approved
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M4B2JBSJSX4Y0XA43GDE9CDP"
---

# Wave 02 Cycle 07: Comment Publication

## Purpose And Authority

Make root-comment and reply submission one headless compound action: checkpoint
exactly once in the item's shared context, then immediately attempt deliberate
context synchronization when a publication remote is configured. Publication
failure is not comment-submission failure after the local checkpoint exists.
The caller receives saved-local evidence and an actionable publication retry.

This is Cycle 07 of [Wave 02](../Waves/wave-02-collaboration.md), tracked by
`01K7F6H9J2N4Q6S8V0X2Z4B6DF`. Authorities are the
[canonical schema](../RFC/canonical-content-and-comment-schema.md),
[Git workflow](../RFC/git-workflow-and-conflict-recovery.md),
[index/recovery](../RFC/repository-index-persistence-and-refresh.md),
[authentication](../RFC/authentication-and-credential-handling.md), and
[test strategy](../RFC/test-and-compatibility-strategy.md) RFCs, plus
`MH-COMMENT-001/002` and `MH-COLLAB-003/004` in the [PRD](../PRD/mvp.md).
The [MVP RFC's local-only amendment](../RFC/mvp-rfc.md) explicitly requires
published/current, local-pending, or local-pending-with-recovery outcomes.
Read the [design](../plans/2026-10-07-wave-02-cycle-07-comment-publication-design.md)
and [implementation plan](../plans/2026-10-07-wave-02-cycle-07-comment-publication-implementation.md)
together. The user approved all three on 2026-10-07 and authorized committing
this planning checkpoint. Implementation remains separately gated.

## Planning Evidence And Implementation Entry Gate

The existing ticket branch/worktree were reused. Fresh `git fetch origin main`
observed base `b666c1e1f0a708562ff4cc25b0dfb18dc99dd6a9`; the clean ticket
head rebased without conflicts from
`92a3646ed1b5915713e68fdf2158e4a5ca49f7c2` to
`c98127ed1203ff77c9a4a3658741fba256bdce1f`. Ancestry verification passed.
The main checkout's unrelated `.superpowers/` and `devenv.nix~` were preserved.
Full paths and checkpoints are in the [execution ledger](../plans/2026-10-07-wave-02-cycle-07-comment-publication-execution.md).

At this base, Wave 01 comment authoring/discovery and Cycles 01–05 are present.
`RepositoryService::submit_comment` still returns `PublishPending` or
`SyncDeferred` without transport. Cycle 05 provides `synchronize_remote` and
authoritative publication/index-only replay. Cycle 06's Cycle/design/plan and
merge/conflict implementation are **absent** from current main; no Cycle 06
review or verification is claimed.

Implementation requires:

- Approval of this Cycle, design, and plan, plus explicit implementation authority.
- Repeat fresh fetch/rebase preflight at the implementation boundary.
- Cycle 06's approved, reviewed merge/conflict operation present on that base;
  reconcile its exact signatures and test names in Task 0. Do not copy another
  worktree's unmerged implementation or implement its behavior in this Cycle.
- Wave 02 entry-gate authorities remain approved and consistent; rerun the
  required baseline, including Wave 01 regression evidence. Planning inspection
  is not a passing Rust or platform gate.

## Scope

- Required compound submission for document/ticket root comments and replies,
  preserving canonical paths, parent validation, stable ID and `created_at`,
  Git identity, scoped checkpoint subjects, and expected-destination guards.
- Typed saved-local receipt with original checkpoint OID and separate publication
  and indexing states; retire `SyncDeferred` from normal submission.
- Durable correlation of the submitted action to one synchronization child,
  without storing bodies, drafts, content fingerprints, credentials, or URLs.
- Immediate reuse of deliberate **context** synchronization, including Cycle 06
  clean merges, retained conflicts, and explicit resolution/restart boundaries.
- Publication-only retry without a body or new comment/checkpoint, and safe
  reconciliation of interruption between local commit and remote handoff.
- Local discovery and recovery visibility even when publication fails; independent
  index-pending evidence if the cache cannot be refreshed.
- Real authenticated-SSH root/reply, two-clone, replay, privacy, and preservation
  tests using existing fixtures and native-test wiring.

## Exclusions And Downstream Obligations

No general item save, primary synchronization, poll, scheduler, promotion,
closure, cleanup, new merge algorithm, conflict-side choice, automatic rebase,
force push, republish consent, new credential policy, or remote backend.
No comment editing/deletion policy or schema change. No desktop/CLI grammar,
composer, prompt, or durable UI draft storage. Wave 03 owns presentation and
must retain the action/comment IDs, explain that sync publishes **all already
checkpointed work on this context**, and offer publication retry rather than
resubmission. Unsaved item buffers are never implicitly checkpointed.

## Required Contract

1. Resolve the selected document/ticket's one shared context; validate the target,
   parent, expected missing destination, canonical content, and Git identity
   using the existing local authoring implementation. A missing identity returns
   `IdentityRequired`; invalid input does not write a comment or contact SSH.
2. Allocate durable correlation before the first canonical write. The caller's
   operation ID continues to identify the local submit; a distinct persisted
   child operation ID identifies synchronization. Never relax the existing
   cross-action/root/target mismatch rules just to reuse one ID.
3. Write `.manyhands/comments/<item-id>/<comment-id>.md` and checkpoint only
   that path as `Checkpoint comment <ULID>`. Unrelated staged, unstaged,
   untracked, deleted, or conflicted work is not included or discarded.
4. Once the commit is authoritative, return its receipt on every later failure.
   Run the existing local discovery handoff before transport. If that handoff
   cannot complete, report saved-local/index-pending with publication blocked
   by local recovery; never bypass pending-local recovery guards. Retry repairs
   only the unfinished handoff before attempting publication.
5. No publication remote: return saved locally/publication pending, with no
   credential prompt, network request, or remote reservation. With a configured
   remote and completed local handoff, immediately invoke item-context sync.
   Dirty unrelated state may block that sync **after** the comment checkpoint;
   it must not turn the receipt into an unsaved result.
6. Published/current require the delegated operation's verified remote OID,
   with the original checkpoint in its ancestry and the original canonical
   comment blob at its path. A moved context or removed/replaced comment is
   recovery-required, not successful publication of this receipt.
   Offline, auth/trust, busy/poll-yield, dirty/conflicted, cancellation, deleted
   remote, changed endpoint, rejected/ambiguous push, and merge recovery all
   return saved-local plus the delegated typed recovery. Preserve the context.
   Do not wrap an unverified push as publication success.
7. Retry with the same identity never generates another comment ID, timestamp,
   checkpoint, context, or synchronization child. Before a checkpoint exists,
   exact submission retry may need the original draft. After it exists,
   publication-only retry accepts identity/approval/restart controls, not a body.
   Validate actual Git and canonical evidence; ambiguous or missing authority
   requires recovery rather than guessed overwrite or a replacement commit.
8. A terminal published/current replay returns its historical evidence and
   repairs only a recorded index handoff. A no-remote pending receipt can later
   start its still-unused child after a remote is configured. An incomplete
   remote child requires explicit restart under the existing fencing rules;
   terminal evidence does not authorize republishing to a different endpoint.
9. No network, prompt, or discovery scan holds the short repository lease. Keep
   Cycle 03's accepted per-call transport limits (not a total submission timeout)
   and safe-point cancellation; no automatic retry loop or scheduling.

## Acceptance And Exit Evidence

| Acceptance | Proof owner |
| --- | --- |
| Root and nested reply, document and ticket, validate and checkpoint once with unchanged canonical schema | Task 1 local-authoring regressions; Tasks 4–5 public compound tests |
| No remote remains locally discoverable, pending, and performs zero transport/prompt/reservation work | Task 4 offline/local-only tests and discovery snapshot assertions |
| Configured remote publishes the checkpoint and other prior context commits, but not unsaved buffers or primary state | Task 5 real SSH and exact-ref/worktree assertions |
| Dirty/untracked/staged/ignored/conflicted unrelated work is preserved; sync failure still contains saved-local receipt | Tasks 1 and 4 preservation snapshots; Task 5 dirty fixture cases |
| Auth/trust/network/busy/cancel/merge/push failures preserve the comment and expose actionable retry | Tasks 4–5 typed outcome and retained-state tests |
| Two collaborators retain both histories or receive Cycle 06 conflict recovery; resolution publishes without a second comment checkpoint | Task 5 real two-clone integration after the Cycle 06 gate |
| Crash windows and explicit restart reuse one action, comment, timestamp, local commit, and child; accepted-but-unacknowledged push is re-observed | Task 2 durable migration/identity tests; Task 5 fault/reopen tests |
| Index failure remains separate from publication; terminal retry repeats only discovery, never checkpoint/fetch/merge/push | Tasks 3–5 injected handoff failures and transport counters |
| Publication-only retry needs no body and refuses corrupted/mismatched/missing evidence or a candidate that excludes/replaces the original comment | Tasks 2–4 negative and Git-reconciliation tests; Task 5 remote-tree proof |
| Body/secret/endpoint sentinels absent from recovery rows, database sidecars, diagnostics and fixture output | Tasks 2 and 5 privacy scans (canonical Markdown/Git blobs intentionally contain comment text) |
| Required Devenv checks and meaningful focused tests pass; native evidence honestly recorded | Task 6; native execution pending authorization and runner availability |

## Self-Review And Lifecycle

The decision audit in the [execution ledger](../plans/2026-10-07-wave-02-cycle-07-comment-publication-execution.md)
covers scope/API compatibility, ID ownership, checkpoint/crash authority,
index failure, trust/privacy, timeout/cancellation, fixture/platform fidelity,
and review/closure. Internal rulings preserve existing RFC behavior; no open
product question was identified. Cycle 06 availability is an explicit dependency
blocker, not permission to expand scope.

Record planning, each task, decisions/blockers, baseline/final verification,
review findings and review-ready status on the ticket. It stays open through
implementation and verification. Closure follows code-review or PR approval,
normally in the authorized final pre-merge push. Push/PR, merge, closure, and
worktree cleanup require separate authorization. Nothing in this proposal
claims implementation or passing native CI.
