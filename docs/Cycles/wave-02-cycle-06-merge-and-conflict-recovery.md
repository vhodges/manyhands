---
title: "Wave 02 Cycle 06: Merge And Conflict Recovery"
date: 2026-10-07
status: approved
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M4AZH9MVJM9QDW4HV4ZHNH64"
---

# Wave 02 Cycle 06: Merge And Conflict Recovery

## Purpose And Authority

Extend deliberate primary and shared-item synchronization to integrate divergent
history by non-rebase merges. Preserve real Git conflicts, expose an observed
headless recovery boundary, checkpoint explicit canonical resolutions, and
resume only unfinished integration/publication/discovery work. No history or
conflict side is silently discarded.

This is Cycle 06 of [Wave 02](../Waves/wave-02-collaboration.md), tracked by
[ticket `01K7F6H9J2N4Q6S8V0X2Z4B6DE`](../../.manyhands/tickets/01K7F6H9J2N4Q6S8V0X2Z4B6DE/ticket.md).
The [Git workflow RFC](../RFC/git-workflow-and-conflict-recovery.md),
[canonical schema](../RFC/canonical-content-and-comment-schema.md),
[repository/index RFC](../RFC/repository-index-persistence-and-refresh.md),
[authentication RFC](../RFC/authentication-and-credential-handling.md), and
[test strategy](../RFC/test-and-compatibility-strategy.md) are authoritative.
The approved Git RFC's Wave 03 owned-Markdown/external-repair boundary applies
now; it is not authority to build front ends.

The user approved this Cycle, the
[detailed design](../plans/2026-10-07-wave-02-cycle-06-merge-and-conflict-recovery-design.md),
and the [implementation plan](../plans/2026-10-07-wave-02-cycle-06-merge-and-conflict-recovery-implementation.md)
together on 2026-10-07 and authorized committing the planning artifacts.
The [planning/execution ledger](../plans/2026-10-07-wave-02-cycle-06-merge-and-conflict-recovery-execution.md)
records rulings, approval and later evidence. Implementation has not begun;
explicit implementation authorization remains a separate gate.

## Entry Evidence And Dependencies

- Reused the existing ticket, branch
  `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DE`, and worktree
  `.manyhands/worktrees/01K7F6H9J2N4Q6S8V0X2Z4B6DE/`.
- Fresh fetch observed `origin/main` at
  `b666c1e1f0a708562ff4cc25b0dfb18dc99dd6a9`, matching local main. The clean
  ticket head rebased from `2dbd3b4777674e7cd0fab584239e62202f677f9c` to
  `8fb768e39835acb7b84cde81df7083a03995b645` without conflicts. Fetched-main
  ancestry and clean status passed; changed `AGENTS.md` was reread.
- Unrelated main-checkout `.superpowers/` and `devenv.nix~` and every other
  worktree were untouched.
- Current main contains Cycle 05's `synchronize_remote`, exact ref mappings,
  ordinary push verification, endpoint-qualified replay, and index-only retry.
  Cycles 03/04 supply selected-key SSH, durable observations/reservations,
  ownership fencing, and publication-history evidence. Wave 01 supplies
  canonical validation, guarded owned-path access and the common-Git lease.
- Cycle 05's [ledger](../plans/2026-10-06-wave-02-cycle-05-clean-deliberate-synchronization-execution.md)
  and comments record local 789-case/static/CLI evidence and whole-Cycle review;
  the ticket is still open and native evidence was explicitly deferred for
  that delivery. Neither historical passes nor that deferral count as Cycle 06
  verification, approval, or closure evidence.

Before implementation, repeat fetch/rebase/ancestry preflight, reconcile the
then-current API, run a fresh baseline, and obtain approval of all three
artifacts plus explicit implementation authorization. Behavioral dependency
changes require reassessment, not copying an old implementation.

## Scope

- Extend the existing deliberate synchronization operation for primary and
  existing document/ticket contexts; keep Cycle 05's non-divergent behavior.
- Integrate remote context first, fetched primary second; primary sync integrates
  only fetched primary. Create ordinary two-parent merge commits when divergent.
- Durable per-integration-step intent, parent/tree/result OIDs, conflict
  fingerprints, and resolution-attempt identity inside the existing recovery
  model. Actual Git/filesystem state remains authoritative.
- Preserve conflicted index entries, merge metadata, markers and worktrees;
  return typed conflict inspection and explicit next actions without publishing
  an unresolved result.
- Caller-supplied, observation-bound resolution of supported owned canonical
  Markdown conflicts; validate before writes and create one two-parent resolution
  checkpoint. Bodies stay in memory/canonical Git, never operation records.
- External-tool guidance and deliberate re-observation/resume for noncanonical,
  binary and unsupported structural conflicts; Manyhands never stages arbitrary
  code as a resolution.
- Recovery after partial checkout, resolution writes, commit/ref transition,
  record failure, push ambiguity or index failure; no duplicate logical merges,
  resolution checkpoints or proven publication effects.
- Discovery handoff after stable results; unresolved conflict/recovery remains
  inspectable even when a discovery refresh cannot index conflicted content.
- Real temporary-repository and two-clone authenticated SSH evidence, privacy
  probes and native-platform test coverage configuration.

## Exclusions And Downstream Obligations

No automatic rebase, force push, conflict-side selection, stash/reset/discard,
merge abort/rollback, republish of a deleted branch, remote-only materialization,
comment-triggered synchronization, polling implementation/scheduler, document
promotion, ticket closure, branch/worktree deletion, desktop/CLI interface or
new transport backend. Conflict resolution does not adopt marker-only content,
change identities, reopen tickets or repair configuration.

Cycle 07 composes comment checkpoints with this synchronization. Cycle 08
remains clean-only polling and MUST NOT use its merge capability. Cycles 09/10
own confirmed context-to-primary integration and cleanup, not this Cycle.
Wave 03 owns conflict UI/CLI and prompts, including supported canonical
journeys and visible external repair for other conflicts. Later adapters must
not call ordinary item saves to simulate a two-parent merge resolution.

## Required Contract

### Ordered Integration And Publication

Retain Cycle 05's exact Fetch scope (`P` for primary; `P` and one `C` for context),
unchanged `FETCH_HEAD`, independent Fetch/Push endpoint checks, selected-key
transport, ordinary non-force push and post-push OID verification.

For each integration pair `(current local, incoming fetched commit)`:

1. Equal or incoming-ancestor: record a no-op.
2. Local-ancestor: perform the existing guarded clean fast-forward.
3. Divergent with common ancestry: create a merge with ordered parents
   `[local, incoming]`. A clean merge incorporates both histories and trees;
   a conflict preserves the real merge index/worktree without updating the
   branch or pushing.
4. No provable common ancestry, missing objects, changed state or unsafe
   checkout: retain state and return recovery-required/external-change.

Context steps are `T(C)` into `C` (when present), then `T(P)` into the resulting
`C`. Each step is journaled separately. A completed first integration is
retained if the second conflicts or fails; retry never recreates the first
merge. This deliberate divergence path is sequential, unlike Cycle 05's
all-clean virtual-plan optimization. Primary sync uses one step `T(P)` into `P`.
Remote absence/deletion/history-unknown boundaries still block automatic
republication. Distinct Push destinations still require an equal/ancestor
candidate, never a merge against an unfetched Push-only history.

Deterministic commit subjects exclude user titles:

```text
Merge remote context <ULID>
Merge primary into <kind> <ULID>
Resolve synchronization <kind> <ULID>
Merge remote primary
Resolve synchronization primary
```

Use the existing Git identity/confirmed-local-identity mechanism; do not invent
an identity or require it for non-committing fast-forward/current/pending paths.
When identity is missing, auxiliary caller confirmation has its own stable ID,
input/configuration observations and recovery evidence; it may be supplied
before a candidate, not silently change an existing one. Once a candidate
commit is recorded, retries reuse its OID/signature rather than generating a
new timestamped commit.

### Conflict And Resolution Boundary

A conflict inspection identifies the operation, target, integration step,
parents, expected HEAD, actual merge/index state, supported paths and opaque
observations. Reading base/local/remote content is an explicit ephemeral
inspection, not diagnostic or journal content. A caller can resolve in-process
only a complete conflict set of regular UTF-8, same-path canonical managed
Markdown with provable stable identities and no unsupported structural entries.
For context or primary synchronization these may include recognized documents,
tickets and comments in the merged branch; they are not permission to write
arbitrary Markdown or unrelated paths. `.manyhands/config.toml`, code, binaries,
symlinks, rename/delete and identity-changing conflicts require external repair.

**Owner decision during self-review, 2026-10-07:** mixed canonical and unsupported
conflict sets require external resolution and commit of the whole merge before
deliberate resume. Manyhands makes no canonical resolution writes to mixed sets.
The owner selected this single-checkpoint boundary with the explicit cost that
the canonical portion of such conflicts cannot be resolved in-process. This
answers a recovery-scope question; it does not approve the whole Cycle or
implementation.

Canonical resolution requires a distinct stable resolution-attempt ID bound to
the parent synchronization ID, exact conflict observation and input digests.
The caller supplies exact result bytes for every supported conflicted path.
Manyhands checks the whole set and expected path/index/HEAD observations,
validates affected canonical identity, metadata and relationships, and writes
only those paths. No reserialization or automatic winner is introduced.
Unrelated dirty/staged/untracked changes block committing rather than being
included, overwritten or discarded. No commit is made while any Git index
conflict remains. Marker disappearance alone is never proof of resolution.

The checkpoint is the resolved two-parent merge itself, not a one-parent save
followed by a second merge. Non-conflicting tracked entries already selected by
Git remain part of its tree, including code from the two parent histories;
only unresolved canonical paths are staged from caller bytes. Resolution returns
local completion; a separate deliberate restart of the original synchronization
resumes remaining integration/publication. No network or prompt holds a lease.

A stale observation or changed-input reuse is rejected before effects. Partial
writes remain recoverable and are compared with recorded per-path result
digests on identical retry, never blindly rewritten. After external repair,
explicit restart accepts a clean, provable merge of the recorded parents and
validates the affected canonical state; it never stages code, accepts a missing
marker as sufficient, or invents a resolution when evidence is inadequate.

### Recovery, Coordination And Time

Use the existing remote reservation and short common-Git lease. Release active
reservation ownership at a recoverable conflict while retaining the operation
and all effect evidence. Any later owner re-observes the actual target; new
synchronizations cannot treat an unrelated or unproved conflict as their own.
Canonical checkpoint operations in the affected worktree must refuse an
outstanding synchronization merge; other safe contexts retain existing
coordination rules. External tools remain possible and invalidate stale
observations.

**Owner concurrency amendment — 2026-10-08:** resolution supports cooperative
writers. Autonomous writers access canonical contents through Manyhands API/CLI
operations participating in reservation, lease, and authoring guards. During the
bounded apply/reconciliation interval, direct external mutations of affected
canonical paths/ancestors, target-worktree validation state, or relevant Git
metadata/private staging that bypass coordination are unsupported. Read-only
inspection and unrelated safe work remain allowed; external repair outside this
interval requires deliberate re-observation. Per-item worktrees reduce collision
risk but do not isolate common Git refs, which remain coordinated.

Observed stale changes still reject before effects, and ambiguous recovery stops
without adopting/removing foreign locks. No-follow reads, scoped writes, privacy,
closure preservation, and effect-aware replay remain required. The contract no
longer promises race-free pathname preservation or continuous foreign-lock
exclusion against arbitrary concurrent namespace substitution bypassing the
cooperative protocol. The concurrency answer alone approves no other change.

**Owner recovery amendment — 2026-10-08:** ambiguous live locks created inside
libgit2 may require operator intervention after a crash. Manyhands must preserve
the lock/effects and return redacted recovery-required; it must never infer lock
ownership from contents, age, or PID alone or delete ambiguous locks automatically.
An operator quiesces relevant writers, verifies and handles the stale lock, then
retries the identical operation. Retry revalidates actual refs/index/worktree/
metadata and reuses recorded candidates. Other owned resolution effects remain
automatically recoverable. This narrow exception does not approve broader manual
recovery or platform/dependency changes. Owner separately approved the
[Task 4 protocol amendment](../plans/2026-10-08-wave-02-cycle-06-task-04-resolution-protocol-amendment.md)
and sequential local implementation on 2026-10-08; all acceptance gates remain.

Prepare merge/validation outside the lease without writing the destination ODB:
locked libgit2 content merging writes blobs even with an in-memory index, so use
a separate worker-local handle with a transient high-priority memory ODB backend
and characterize its isolation first. Reacquire the lease to import verified
result objects, re-observe and apply each guarded local transition, with explicit
partial recovery. Network, full scans and caller interaction never hold it.
Cancellation is checked before/after preparation, local transitions, each
integration stage and existing network/discovery safe points, never used to
abort an atomic libgit2 mutation halfway. No total cancellation/shutdown deadline
is promised; inherit transport limits rather than adding unreviewed timeouts or
automatic retries.

A saved prepared candidate, completed merge, resolution checkpoint or verified
push is reconciled from actual refs/commits/index/worktree before replay.
Completed local integration is not recreated because a record write or push
failed. Completed publication/index-pending replay stays network-free. A
changed remote requires fresh observation before unfinished work; an unchanged
conflict can be resolved locally without refetching. Cache loss or an ambiguous
external merge is recovery-required, never authority for blind replay/rollback.

## Acceptance And Exit Evidence

All evidence below is planned, not collected for Cycle 06.

| ID | Acceptance contract | Required proof / plan owner |
| --- | --- | --- |
| A1 | Primary and item divergence retains both histories; context precedes primary. | Real commit graphs, ordered parents, merged file contents, exactly one commit per divergent step; Tasks 1/3/6. |
| A2 | Conflicts remain inspectable and cannot publish or lose local work. | Real index stages, merge metadata, marker files, unchanged HEAD/remote and retained branch/worktree; Tasks 3/6. |
| A3 | Canonical resolution is explicit, scoped, validated and observation-bound. | Document/ticket/comment fixtures; stale bytes/index/HEAD, malformed/identity/thread/closure cases, unrelated work preservation; Tasks 4/6. |
| A4 | Code/binary/structural conflicts require external repair. | Mixed-set non-mutation; genuine external two-parent commit and deliberate resume; reject marker-only/index-only/ambiguous repair; Tasks 4/5/6. |
| A5 | Replay resumes only incomplete effects. | Faults around each step's prepare/checkout/ref/record and resolution writes/checkpoint; count commits and network effects, preserve first merge on second conflict; Tasks 2/5/6. |
| A6 | Existing transport, ref scope, absence and push semantics remain intact. | Cycle 05 regressions plus SSH rejection/ambiguous push/distinct push endpoint/exact-ID recovery after a merge; Tasks 3/5/6. |
| A7 | Stable Git results refresh discovery; failed refresh retries only indexing. | Fresh discovery of merged/resolved content, durable conflict inspection without successful indexing, network-disabled index-only replay; Tasks 5/6. |
| A8 | Ownership, cancellation and privacy remain safe. | Two-service fencing/lease/yield tests; no bodies/credentials/endpoints in recovery rows, WAL/backups/errors/raw fixture capture; Tasks 2/4/5/6. |
| A9 | Verification and lifecycle evidence is honest. | Fresh baseline, required local suite, focused SSH target, review and actual authorized native evidence or explicitly approved Cycle-specific deferral; Task 7. |

Native libgit2 index/checkout/worktree behavior must be exercised on the existing
five-target Linux/macOS/Windows matrix. CI is currently `workflow_dispatch` only.
Do not enable, dispatch or publish solely because this plan names a native gate.
Until authorized runs pass, native proof remains pending; Cycle 05's deferral
does not silently waive this Cycle's gate.

## Self-review And Lifecycle

The design's [decision audit](../plans/2026-10-07-wave-02-cycle-06-merge-and-conflict-recovery-design.md#decision-audit)
and the execution ledger record the `review-cycle-docs` self-review. Risk areas:
whole-branch versus owned-path scope, mixed/structural conflicts, stale and
partial resolution, candidate/ref ambiguity, sequential merges, endpoint and
secret handling, lease/cancellation bounds, fixture fidelity, native gaps and
closure authorization. Self-review surfaced one material mixed-conflict scope
question; the owner answered it as recorded above. Internal rulings refine the
approved contract, including two-parent candidates, per-stage/attempt evidence,
identity-confirmation replay and bounded current-ref reconciliation. No material
question remains open.

Record planning approval, fresh implementation entry, each task's decisions and
verification, blockers, code review and review-ready status as ticket comments.
Proposed execution is sequential direct work in this ticket worktree, with
focused red/green tests and review checkpoints; delegation is not selected or
authorized by this planning request. Keep the ticket open. Only after separate
implementation authorization and code-review/PR approval may separately
authorized delivery close it in the final pre-merge checkpoint. Push/PR, merge
and worktree cleanup remain separate permissions.
