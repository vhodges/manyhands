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

**Update, 2026-10-09.** Cycle 06 merged to main as `5e4fad6` (pull request 14)
and this ticket was rebased onto it; the paragraph above is the planning-time
record. The Cycle 06 dependency gate is met. Reconciling this Cycle against the
merged code produced the amendment section below and one new prerequisite.

Implementation requires:

- Approval of this Cycle, design, and plan, plus explicit implementation authority.
- Repeat fresh fetch/rebase preflight at the implementation boundary.
- Cycle 06's approved, reviewed merge/conflict operation present on that base
  (met 2026-10-09); reconcile its exact signatures and test names in Task 0. Do
  not reimplement its behavior in this Cycle.
- Ticket `01M4GD0KKXW684QBA49F6EX3WE` (save must leave the Git index current for
  the paths it commits) merged to the base. Without it the immediate context
  synchronization refuses the worktree as not clean after every comment
  checkpoint, so the compound action could never publish.
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

## Amendment: Reconciliation With Merged Cycle 06 — 2026-10-09

Made at the owner's direction after reviewing the approved plan against the
merged Cycle 06 code. The contract above stands; these points refine it.

1. **Index prerequisite.** A comment checkpoint commits from an in-memory index
   and leaves the on-disk Git index stale; synchronization then reports the
   worktree not clean. Every Cycle 06 SSH fixture refreshes the index after
   `submit_comment`. This Cycle depends on ticket `01M4GD0KKXW684QBA49F6EX3WE`
   and must not carry its own workaround; its acceptance tests prove the real
   save → synchronize path with no fixture refresh.
2. **Repository-wide conflict block.** A pending synchronization conflict in any
   context makes every other synchronization in the repository return `Busy`,
   and there is no abandon path yet (ticket `01M4H33R34Z7C7EEKTY1ZCT950`). A
   comment that can be checkpointed in another writable context is saved locally
   with publication pending/`Busy`. Existing local authoring refuses a new comment
   in the context owned by the pending merge before writing/checkpointing it;
   that pre-checkpoint refusal returns local recovery, not a saved receipt. This
   Cycle preserves both guards; contract point 6 maps delegated `Busy` after a
   proven checkpoint to saved-local. This qualification was approved by the
   owner on 2026-10-10 at the Task 0 review checkpoint.
3. **Cancellation.** Cancelling a synchronization child whose newest window
   holds a pending conflict or owned resolution is a recoverable stop: the child
   becomes `interrupted`, not `cancelled`, and stays restartable and
   resolvable. `cancel_remote_operation` has no effect on a child parked after a
   released conflict. Cancellation of a submission never undoes its checkpoint
   and reports whichever of those states the child reached.
   **Clarified and repair authorized 2026-10-10:** owned resolution includes an
   applied checkpoint whose existing resolution-release proof is unfinished.
   The owner permits a narrow shared Cycle 06 predicate repair in this branch;
   released resolutions and ordinary applied clean merges retain terminal
   cancellation. No second child or general cancelled-child revival is added.
4. **Identity at a merge.** `SynchronizeRemoteRequest` now takes an optional
   `confirmed_identity`, and `SynchronizationError::IdentityRequired` carries an
   opaque `ExpectedConfiguration`. Submission already requires a Git identity
   before the first write, so this arises only if the identity is removed before
   a later merge. The compound and retry requests pass an optional confirmation
   through; an unanswered identity boundary is saved-local with that typed
   recovery, never a failed submission.
5. **Typed recoveries to map by name.** `ConflictPending`,
   `ExternalResolutionRequired`, `PushRejected` (the merge is kept and a
   deliberate retry continues through an appended publication attempt),
   `RemoteContextDeleted`, `ExternalChange` and `RecoveryRequired` each map to
   saved-local with that recovery. A verified publication whose remote later
   rewound or diverged stays recovery-required.
6. **Published versus current.** Cycle 06 reports `Published` only when the
   operation itself recorded a push intent for that exact commit, otherwise
   `AlreadyCurrent`. The receipt relays whichever the child reports; both
   require the ancestry and blob proof of contract point 6.
7. **Resolution uses the child ID.** Cycle 06 inspection, conflict reading and
   `resolve_synchronization` are keyed by the synchronization operation ID. The
   receipt's bound child ID is that key; this Cycle adds no resolution surface.
8. **Windows path length.** Context-worktree fixtures have about 40 characters
   of headroom under the 260-character limit and comment files are the longest
   canonical paths. The new SSH target keeps fixture directory names short and
   builds no context worktree inside a nested output-control child.

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
product question was identified. Cycle 06 availability was an explicit dependency
blocker, met on 2026-10-09; the index ticket named in the entry gate is the
remaining one. Neither is permission to expand scope.

Record planning, each task, decisions/blockers, baseline/final verification,
review findings and review-ready status on the ticket. It stays open through
implementation and verification. Closure follows code-review or PR approval,
normally in the authorized final pre-merge push. Push/PR, merge, closure, and
worktree cleanup require separate authorization. Nothing in this proposal
claims implementation or passing native CI.
