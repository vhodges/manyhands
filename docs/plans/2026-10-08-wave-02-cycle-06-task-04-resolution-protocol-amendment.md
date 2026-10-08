---
title: "Wave 02 Cycle 06 Task 4 Resolution Protocol Amendment"
date: 2026-10-08
status: approved
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M4C800000000000000000000"
---

# Task 4 resolution protocol amendment

## Authority and scope

Ticket: `01K7F6H9J2N4Q6S8V0X2Z4B6DE`. Task 3 HEAD: `6bd1f18`.
Preserve the existing Task 4 work; Tasks 5–7 remain pending. The owner approved
this technical amendment and sequential implementation on 2026-10-08. No delivery,
cleanup, task acceptance, or native verification waiver is granted.

References: [Cycle](../Cycles/wave-02-cycle-06-merge-and-conflict-recovery.md),
[design](2026-10-07-wave-02-cycle-06-merge-and-conflict-recovery-design.md),
[implementation plan](2026-10-07-wave-02-cycle-06-merge-and-conflict-recovery-implementation.md),
[execution ledger](2026-10-07-wave-02-cycle-06-merge-and-conflict-recovery-execution.md).

## Approved owner decisions

1. **Cooperative writers:** autonomous processes use Manyhands API/CLI and the
   existing reservation/lease/authoring guards. Concurrent direct mutation
   bypassing coordination of affected canonical paths/ancestors or relevant
   target-worktree/Git/staging state during bounded resolution apply and
   reconciliation is unsupported. Reads and unrelated safe work remain allowed.
   Worktrees reduce file collisions but do not isolate shared refs. Observed stale
   changes still reject; arbitrary namespace replacement is not promised perfect
   pathname preservation or continuous foreign-lock exclusion.
2. **Narrow manual backend-lock recovery:** an ambiguous live lock created inside
   libgit2 after a crash is preserved with a redacted recovery-required outcome.
   An operator quiesces writers, verifies and handles the stale lock, then retries
   the same operation. No automatic stale-lock deletion or ownership inference
   from contents, age, or PID alone. Other owned resolution effects must still
   recover automatically. This is not permission for agents to remove such locks.

Unchanged: ordered two-parent commits, complete eligible canonical conflict sets,
no arbitrary staging/rollback/publication, scoped exact caller bytes, all-side
closure/identity/comment invariants, privacy, existing common-Git fencing, and
five-target native functionality/evidence. Do not introduce a new crate.

## Proposed protocol

### Stable owned sentinel and durable provenance

Replace index/sentinel exchanges and retirement placeholders with one stable
operation-owned `index.lock`. It is an exclusion sentinel, never the serialization
source or the installed index. Prepare/sync it privately, retain a private
identity anchor, and durably record operation/attempt/stage ownership **before**
publication without replacing any existing lock. Sync affected directories before
recording publication observation. Do not use Drop as the sole recovery mechanism.

Evidence is role identifiers, operation-derived artifact identifiers, filesystem
identity plus retained anchor, digests, candidate/tree OIDs, immutable intent and
fenced progress. Do not persist caller bodies, credentials, URLs, backend messages,
or arbitrary paths in SQLite/journals. Matching contents alone never prove owned
lock identity. Foreign/ambiguous locks stop recovery without deletion/adoption.

Keep the stable sentinel through index installation, ref transition, checkpoint
validation and individually recoverable metadata retirement. Journal release
intent; verify the owned anchor/identity and release under the approved
cooperative namespace convention. Unix release is not expected-inode CAS.

### Separate authoritative index serialization

Use locked libgit2 to serialize a copy of the authoritative merge index in an
operation-private staging namespace, separate from the live sentinel. Preserve
supported semantic REUC/NAME metadata; characterize cache/optional-extension
normalization and reject unsupported mandatory formats before canonical writes.
Remove the custom SHA-1/v2 serializer and direct `sha1` dependency. Private staging
is trusted under the cooperative convention, not because mode 0700 excludes
another same-user process. Install only verified prepared output under the held
sentinel; checkout must not independently rewrite the live index.

### One actual-state recovery dispatcher

| Interrupted actual state | Required recovery |
| --- | --- |
| Ownership intent recorded; sentinel absent/present | Reobserve baseline; publish only into absence or adopt only the exact anchored owned identity, including paths_applying before candidate creation. |
| Path old/result with observation missing | Old: complete missing write. Exact bound result: record observation without rewriting. Third value: preserve and stop. |
| Private serialization partial | Live index/sentinel unchanged; reconcile/rebuild only owned private preparation. |
| Intended index installed, old HEAD | Verify paths/tree/index/metadata/sentinel; complete only the recorded ref transition. |
| Candidate HEAD, checkpoint observation missing | Verify exact parents/result and record completion; never recreate a candidate. |
| Backend-created live lock blocks progress, ownership ambiguous | Preserve effects/lock; require the approved operator intervention, then same-ID exact-state reconciliation. |
| Some merge metadata members already absent | Reconcile each fixed member independently; absent is completed, matching owned remnant can retire, foreign remnant stops. |
| Sentinel release intent; owned/absent/foreign lock | Finish verified owned release or observe absence; never retire/adopt foreign identity. |

Reconcile known effects before fresh-operation preflight. No blind rollback,
second candidate/ref checkpoint, network work, or user interaction under the lease.
Audit every reachable backend-created live lock, not just loose branch locks;
separate ref and reflog effects where the backend does not commit them atomically.

Validate closure and other canonical invariants against **all** recorded sides.
Conflicting immutable evidence requires external recovery before writes; an open
base cannot authorize reopening a closed local/incoming ticket.

## Platform strategy and feasibility gates

These are API-level candidates, not executed proof. Use existing libc/windows-sys
and helpers; characterize capabilities before adopting them. Same-volume/local
filesystem requirements must be explicit and checked before effects.

| Operation | Linux/macOS | Windows |
| --- | --- | --- |
| No-follow reads | Pinned directory traversal with openat/O_NOFOLLOW and fstat. | Retained ancestor handles, OPEN_REPARSE_POINT/BACKUP_SEMANTICS; reject reparse points and characterize sharing protection. |
| Owned identity/anchor | Device/inode plus private hard-link anchor. | Volume/file identity plus private hard-link anchor on supported filesystems. |
| Publish sentinel without replacement | linkat into absent index.lock. | CreateHardLinkW into absent index.lock. |
| Install canonical/index output | Same-filesystem rename under cooperative convention, pre/post image and identity verification. | Characterize handle-based FileRenameInfo or same-volume MoveFileExW, expected-absence publication and verified replacement. |
| Owned sentinel release | Verify identity/anchor, unlinkat under cooperative convention; reconcile release intent. | Characterize verified-handle disposition and pathname disappearance/close ordering. |
| Storage ordering | File/directory sync; characterize macOS fsync/F_FULLFSYNC and backend ref/object/SQLite ordering. | FlushFileBuffers/sync_all and rename ordering; directory-entry power-loss durability needs explicit proof, not an assumed Unix equivalent. |

Linux-only refusal is not native completion. Unavailable capabilities stop before
mutation; a required platform lacking the approved behavior remains an acceptance
blocker, not an automatically approved external-only product fallback. Process
crash recovery tests do not prove power-loss durability. Escalate genuine platform,
storage, or backend feasibility gaps rather than silently weakening contracts.

## Implementation and proof checkpoints

Retain applicable public API/token types, full-set eligibility, identity binding,
fencing, prospective validation and tests. Replace the serializer, sentinel
exchange/placeholder lifecycle, content-only recovered ownership, and
candidate-only recovery dispatch. Keep one writer in the existing ticket worktree.

1. Durable artifact/phase evidence and stable sentinel lifecycle, including
   acquisition before candidate creation and owned release reconciliation.
2. Separate libgit2 serialization, actual-state path/index/ref/metadata recovery,
   all-side closure validation, and supported native helpers.
3. Real child-process termination **without Drop** inside publication, canonical
   installation, index installation, backend lock creation/commit, per-member
   metadata retirement and sentinel release. Also inject observation transaction
   failures, byte-identical foreign locks, old HEAD with installed resolved index,
   REUC/NAME/cache extensions and altered same-ID input. Prove cooperating writers
   are blocked during effects and unblocked afterward. Ambiguous backend-lock
   tests must preserve locks and converge only after simulated operator action.

Review each bounded checkpoint; after at most three review rounds, return unresolved
protocol/feasibility issues for decision rather than repeat an unlimited patch loop.
Run required Devenv check/fmt/strict clippy/all-feature tests and CLI smoke before
Task 4 acceptance. Investigate the discovery contention failure; an isolated pass
is not a full-suite pass. Keep native evidence pending until actually obtained.

## Approval gate

The two owner policy amendments and this technical protocol/checkpoint plan are
approved (2026-10-08). Resume sequential local implementation, followed by
independent review and the existing scoped Task 4 checkpoint rules. Approval
does not waive tests, acceptance blockers, or any delivery/lifecycle restriction.
An explicitly unaccepted local WIP snapshot may preserve the existing agent-owned
dirty work for the required main rebase; it does not constitute Task 4 acceptance.
