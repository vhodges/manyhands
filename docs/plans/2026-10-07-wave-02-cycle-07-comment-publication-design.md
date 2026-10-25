---
title: "Wave 02 Cycle 07 Comment Publication Design"
date: 2026-10-07
status: approved
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M4B2JBSQ3PGMEH999ENCN5GQ"
---

# Comment Publication Design

## Intent And Authority

[Cycle 07](../Cycles/wave-02-cycle-07-comment-publication.md) composes existing
local authoring with deliberate context synchronization; it is not a new
transport or merge engine. The [implementation plan](2026-10-07-wave-02-cycle-07-comment-publication-implementation.md)
orders that composition. The [execution ledger](2026-10-07-wave-02-cycle-07-comment-publication-execution.md)
records the skill-based self-review, rulings, dependency gap and approval gate.
The user approved the Cycle, design and implementation plan on 2026-10-07 and
authorized their planning commit. This does not authorize implementation.
Ticket: `01K7F6H9J2N4Q6S8V0X2Z4B6DF`.

The approved Wave and MVP local-only amendment settle the external policy:
checkpoint first, immediately attempt context synchronization when configured,
and retain a discoverable local comment on failure. The desktop RFC also
settles that submission can publish other **already committed** context work;
unsaved buffers are excluded. No extra confirmation is invented here.

## Current Source And Dependency Boundary

At planning base `b666c1e1f0a708562ff4cc25b0dfb18dc99dd6a9`:

- `src/repository.rs` defines `SubmitCommentRequest`, `CommentSubmissionOutcome`,
  `LocalCheckpoint`, and `CommentPublicationState::{PublishPending, SyncDeferred}`.
  `submit_comment_with_effective_config` already provisions/reuses an edit
  context, validates the item/parent, handles exact replay, preserves
  `created_at`, checkpoints one path with an in-memory scoped index, and hands
  authoritative state to discovery. It does not contact a remote.
- `src/repository/recovery.rs` owns local operation identity, migration,
  reconciliation and index ownership. Pending local recovery intentionally
  blocks remote synchronization.
- `src/repository/remote/{sync,state,reservation}.rs` supply
  `SynchronizeRemoteRequest { root, operation_id, target, approval, restart }`,
  `SynchronizationTarget::Context { kind, item_id }`, selected-key transport,
  explicit restart fencing, verified remote authority and index-only replay.
  Remote IDs cannot share a local submit action's ID: only the narrowly bound
  refresh handoff can coexist with a synchronization ID.
- `src/bin/manyhands-cli.rs` is startup-only. Supported ticket/comment commands
  do not exist; planning updates use canonical filesystem paths.
- Cycle 06 artifacts and its merge/conflict APIs are absent. Implementation
  Task 0 must inspect that approved merged contract before fixing its exact
  resolution request or tests. The proposal delegates divergence, conflict
  checkpoints, republish consent, and resolution; it does not invent them.
- Existing `tests/remote_synchronization.rs` uses the custom pre-thread SSH host,
  output privacy wrapper and shell-free fixture. `.github/workflows/build.yml`
  is manual-only and lists headless test targets on five native architectures.
  Wiring a target is not execution evidence. Cycle 05's owner-approved native
  deferral is historical evidence, not new authorization to dispatch CI here.

### Update at base `5e4fad6` — 2026-10-09

The list above is the planning-time record. After rebasing onto merged Cycle 06:

- Cycle 06 is present. `SynchronizeRemoteRequest` (now in
  `src/repository/remote/state.rs`) is `{ root, operation_id, target, approval,
  confirmed_identity, restart }`. `SynchronizationError` adds `ConflictPending`,
  `IdentityRequired { target, expected_configuration }` and
  `ExternalResolutionRequired`. `RepositoryService` adds
  `inspect_synchronization_recovery`, `read_synchronization_conflict` and
  `resolve_synchronization`, all keyed by the synchronization operation ID.
- A continuation after an earlier push intent is journaled in
  `remote_publication_attempts`; authority comes from the newest verified
  attempt, and the operation's legacy phase stays frozen while one is in flight.
  The binding table must not mirror any of that.
- A pending conflict in any context returns `Busy` to every other
  synchronization in the repository.
- A comment checkpoint leaves the on-disk Git index stale and synchronization
  then refuses the worktree as not clean (ticket `01M4GD0KKXW684QBA49F6EX3WE`).
  That ticket is a prerequisite; this design adds no index handling of its own.
- `submit_comment` callers are now `tests/local_authoring.rs`,
  `tests/recovery_foundation_gate.rs`, `tests/remote_merge_recovery.rs` and
  `src/repository/remote/sync_tests.rs`. The Wave 03 read boundary lists
  `submit_comment` as an operation action (`src/repository/read/{dto,status}.rs`,
  `schemas/v1/operation.schema.json`); the action name is kept. `SyncDeferred`
  appears in no published schema, so retiring it changes no fixture.
- New recovery tables follow the Cycle 06 registry rules: created all-or-nothing
  inside the existing migration, validated at startup, and listed in the table
  inventory in `tests/repository_enablement.rs`.
- `tests/remote_merge_recovery.rs` is the closest fixture model: two clones over
  the real SSH harness, a one-round bcrypt fixture key, output-control children,
  and `race_update` for any reference.

## Public Interface And Compatibility

Ruling: make normal `submit_comment` the compound action, extracting its current
local implementation into a crate-private checkpoint helper. A public normal
submit must not quietly retain `SyncDeferred`. This is an intentional library
signature change before Wave 03 interfaces exist; preserve the low-level draft
shape and migrate all in-repository callers/tests together. Source inspection
found callers only in `tests/local_authoring.rs` and
`tests/recovery_foundation_gate.rs`, plus the Wave 03 API-audit description.
Re-scan at Task 0; newly merged front-end consumers are a compatibility review,
not authority to leave a bypass. Document the final signature in that audit.

Proposed types (names are internal rulings, finalized at Task 1):

```rust
pub struct PublishCommentRequest {
    pub comment: SubmitCommentRequest,
    pub approval: Option<HostApproval>,
    pub confirmed_identity: Option<ConfirmedCommitIdentity>, // amended 2026-10-09
}
pub struct RetryCommentPublicationRequest {
    pub root: PathBuf,
    pub operation_id: OperationId, // original submitted action, not a new ID
    pub approval: Option<HostApproval>,
    pub confirmed_identity: Option<ConfirmedCommitIdentity>, // amended 2026-10-09
    pub restart: bool, // explicit resume of an interrupted remote child
}

// Both accept the existing caller-owned session, not key paths or passphrases.
RepositoryService::submit_comment<P: SessionCredentialProvider>(
    request: PublishCommentRequest, session: &mut SessionCredentials<P>
) -> Result<CommentSubmissionOutcome, CommentSubmissionError>;
RepositoryService::retry_comment_publication<P: SessionCredentialProvider>(
    request: RetryCommentPublicationRequest, session: &mut SessionCredentials<P>
) -> Result<CommentSubmissionOutcome, CommentSubmissionError>;
```

`CommentSubmissionOutcome` distinguishes `IdentityRequired { context }` from
`Saved { receipt, publication, indexing }`. The saved receipt includes canonical
root, item kind/ID, comment ID, optional parent ID, original action ID, bound
synchronization ID, canonical comment path, context branch and **original
checkpoint OID**. It contains no body, author-entered title, endpoint, or secret.
Do not replace the original OID with the later merge or pushed OID.

Publication state is independent of indexing:

- `Published { oid }` or `AlreadyCurrent { oid }`: delegated verified authority.
- `Pending { reason }`: `NoPublicationRemote`, `LocalRecoveryRequired`, or the
  fixed typed synchronization recovery (including busy/yield/cancel/conflict).

Amended 2026-10-09: the fixed synchronization recoveries are Cycle 06's typed
errors, mapped by name: `Busy` (including another context's pending conflict),
`PollYielding`, `Interrupted`, `WorktreeNotClean`, `WorktreeConflicted`,
`ConflictPending`, `ExternalResolutionRequired`, `IdentityRequired` (with its
opaque `ExpectedConfiguration`, which the caller passes back unread in
`confirmed_identity`), `PushRejected`, `RemoteContextDeleted`, `ExternalChange`,
`RecoveryRequired` and transport categories. `confirmed_identity` is forwarded
to the child only; it never changes the comment checkpoint's author. `Published`
versus `AlreadyCurrent` is relayed as the child reports it.

Indexing distinguishes current from pending local-checkpoint and/or remote
stable-state handoffs. Reuse existing `IndexPending` evidence where suitable;
a cache failure must not erase a known published result. Retain actionable
error categories and required next action, not raw backend messages. Add no
`Debug` implementation that exposes requests, drafts, or session material.

Errors before a proven local checkpoint can report invalid input, identity,
local write/checkpoint/recovery failure, and retained context evidence. A
written-but-uncommitted file is **not** a saved checkpoint. After the commit is
proved, all transport, cache and receipt-persistence failures return `Saved`
with pending/recovery evidence. If observation itself cannot prove whether a
commit happened, return typed local recovery with the original identity and
possible retained effects; never claim unsaved/no effect or start a replacement.

## Durable Identity And Receipt

Keep the caller's `AuthoringTarget.operation_id` for the existing local
`submit_comment` record. Add a narrowly scoped `comment_publication_bindings`
table in the existing registry/recovery migration. This is correlation and
receipt metadata, not another reservation or independent effect journal.

Each binding records:

| Field | Purpose |
| --- | --- |
| Canonical root and submitted operation ULID | Stable externally retained action identity |
| Kind, item ULID, comment ULID, optional parent ULID | Exact immutable target; path/branch derived from existing conventions |
| Synchronization child ULID | One server-generated `OperationId::new()` persisted before a canonical write |
| Local pre-checkpoint OID and nullable checkpoint OID | Git reconciliation and authoritative original receipt |
| Validated creation timestamp once observed | Preserve/check immutable canonical identity; never generate it again on retry |

Allocate the binding transactionally with the local identity check before the
first write, ensure child uniqueness against all local/remote/binding IDs, and
use unique root/action and root/comment constraints. Same comment with a new
action cannot become an implicit edit or duplicate submit. Do not persist body,
request serialization, body hash, author title, URL, key path, server text, or
arbitrary exception strings. Existing records carry phase/index state; remote
child records carry remote authority. Do not maintain a second copy of their
remote phase machine in this table.

The local submit is completed after its checkpoint and local index handoff;
publication pending does **not** keep a local record artificially pending.
The remote child remains a normal manual reservation/action with its existing
cross-ID rules. Parent-to-child lookup for retry/cancellation is read-only;
never grant an ownership token just by reading a binding. Until local recovery
finishes, no child is started. This preserves `require_no_pending_local` and
avoids weakening unrelated-operation serialization.

Backward-compatible migration leaves old comment, refresh, remote and pending
rows intact. Legacy comment actions without a binding are not auto-published
on startup or migration. Deliberate ordinary context sync can publish them.
A compound retry for a legacy/missing binding returns explicit recovery rather
than allocating fresh correlation and guessing a prior checkpoint. Existing
canonical files and Git refs stay authoritative when the cache is rebuilt.
Lost receipt/correlation is visible recovery; rebuild never authorizes a second
comment file or checkpoint.

## Ordered Data Flow

```text
validate root / action identity / target
  -> bind one child ID before any canonical write
  -> existing context + parent + identity + owned-path validation
  -> write one comment -> checkpoint one owned path
  -> prove and persist original receipt
  -> complete existing local discovery handoff (short lease already released)
  -> no remote: Saved/Pending; do not start/freeze the child
  -> configured remote: synchronize_remote(Context, bound child, session)
  -> map verified authority or typed recovery + independent indexing
  -> return Saved; never delete/rollback the local comment
```

A no-remote result does not require a clean whole worktree: the local checkpoint
is scoped. With a remote, delegated clean-worktree checks can refuse unrelated
unstaged/staged/untracked/conflicted state. Publication then stays pending,
without implicit saves, stash/reset or including those changes. Ignored path
collisions retain the existing safe-checkout protection. Clean context sync
may incorporate remote context/primary history, but never synchronizes/pushes
primary or checkpoints the caller's unsaved item draft.

Use the same caller-owned `SessionCredentials` for every delegated connection.
Missing key, unlock cancellation/rejection, host approval and changed endpoint
policy remain Cycle 03's typed outcomes. No credentials are required for
local-only submission. Prompts happen only after the checkpoint and outside the
short lease. No new host trust, fallback key, remote URL or refspec input.

## Replay And Failure Boundaries

| Boundary | Observed state and permitted retry |
| --- | --- |
| Binding created, no file | Exact original submission can finish validation/write; no network |
| File written, no checkpoint | Preserve file/time; exact body-bearing retry uses existing owned-path comparison and finishes at most one checkpoint |
| Commit exists, receipt persistence interrupted | Reconcile real commit/tree/path from recorded pre-checkpoint OID and local operation; persist the same receipt, never call a fresh checkpoint blindly |
| Receipt exists, local index handoff incomplete | Verify original commit and canonical identity, repair only local discovery; if it still fails return Saved/Pending with index recovery |
| Local handoff done, remote child not started | No-remote pending, or start exactly the bound child using current configured remote |
| Child busy/yielding or transport failed | Saved/Pending; explicit retry observes the recorded child and follows normal reservation/restart rules |
| Merge conflict or unresolved push | Keep context/conflict/checkpoint; resolution/reconciliation is Cycle 06/05's explicit recovery before publication retry. The caller resolves through `resolve_synchronization` or an exact external two-parent repair using the bound child ID, then retries with `restart` |
| Another context's conflict is pending | Child returns `Busy`; Saved/Pending for every item until that conflict is resolved (no abandon path yet) |
| Server accepted push, acknowledgement/persistence lost | Delegate fresh Push-direction observation, never blind repeat push |
| Published/current with index pending | Return known authority; exact child replay repairs only discovery, no checkpoint/fetch/merge/push |
| Completed action, later branch/remote edits | Return historical receipt/authority, not a claim about current remote; a new ordinary sync is a separate action |
| Binding/commit missing, operation target changed or evidence ambiguous | Typed recovery/mismatch; no guessed timestamp, overwrite, new child, or new commit |

For commit-before-receipt reconciliation, verify the actual immutable comment
blob and commit ancestry/diff, not only its subject or file existence. The
original checkpoint must be reachable from the expected shared context; its
owned-path diff must match the local action and original pre-checkpoint history.
The body may be compared **in memory** against the supplied original draft when
required; it is never stored in recovery. Intervening external ref/worktree
changes or multiple plausible commits are recovery-required. A publication-only
retry requires a proved receipt; if none exists it identifies the original
submission/reconciliation action instead of accepting a replacement body.

Once a receipt is proved, subsequent body-bearing submission replay must still
reject a changed item/comment/parent and, if a body is supplied, compare it with
the **original committed blob**, not potentially merged current worktree text.
Publication-only retry has no body. Before starting/resuming the child, prove
that the current expected context contains the original checkpoint in its
ancestry and retains the original comment blob. Before reporting a fresh or
replayed published/current authority, prove the same facts at that **authority
OID**, not merely at current HEAD. Use immutable Git objects; compare original
blob OIDs/bytes in memory, not a persisted content fingerprint. A successful
branch sync alone is insufficient proof of this comment's publication. Excluded
checkpoint or removed/replaced comment returns saved-local/recovery without
automatic overwrite or replacement checkpoint. A later terminal replay checks
the historical authority tree, not new remote edits.

Current dirtiness or conflict does not undo
the receipt; it blocks unsafe synchronization. With conflicts, retained Git and
canonical content plus visible context problems are recovery evidence; a failed
index scan is not labelled a current discoverable snapshot.

Cancellation uses the existing child's cancellation API/safe points through the
binding. Before child creation there is no remote cancellation effect; local
atomic write/commit is not interrupted mid-transition. Amended 2026-10-09 to
Cycle 06's contract: a cancel honoured while the child's newest window holds a
pending conflict or owned resolution leaves the child `interrupted` and
restartable, not `cancelled`; a cancel requested during a resolve does not
outlive it; and `cancel_remote_operation` has no effect on a child parked after
a released conflict, so the mapping reports "nothing to cancel" there rather
than a cancelled publication. A terminally cancelled child without a pending
conflict keeps the comment saved-local; publishing it afterwards is a new
ordinary context synchronization, since the bound child cannot be reused. An interrupted child is
explicitly resumed only where the existing contract allows it. The composition
introduces no automatic restart, conflict resolution, republish permission,
retry scheduler or total timeout. Accepted transport limits remain 10,000 ms
per TCP address connect and 30,000 ms per blocking SSH API call; progressing
multi-call transfers can exceed that duration. Test watchdogs detect hangs but
are not production timeout evidence.

**Owner-authorized clarification and scope exception — 2026-10-10:** an owned
resolution includes an applied checkpoint whose existing release proof is not
complete. The real SSH composition exposed a missing shared Cycle 06 predicate
case. The owner authorizes repairing that predicate here with the existing
attempt/release evidence and regression coverage. This does not revive terminal
cancelled children generally, change clean-merge cancellation, allocate another
bound child or add merge/abandon policy.

## Alternatives And Rulings

- **Return `SyncDeferred`, let the UI call sync:** rejected; violates required
  headless compound submission and leaves a crash gap/UI-only correctness.
- **Use the same ID for local submit and remote sync:** rejected; current
  reservation code forbids this action collision for good recovery reasons.
  Ruling: persist one separate child; cost is correlation migration/testing.
- **Two independent public submit APIs:** rejected for normal callers; one would
  silently permit bypass. Ruling: extract a private local helper and change
  the public signature; cost is in-repository test/API-audit migration.
- **Hold a lease across both halves:** rejected; transport/prompts/scans cannot
  hold the common-Git lease. Existing remote reservations retain coordination.
- **Save body/hash to simplify replay:** rejected by recovery/privacy authority.
  Git blobs and stable canonical IDs supply proof; ambiguity stops safely.
- **Publish even while local discovery is pending:** rejected; bypasses existing
  pending-local guards. Ruling: attempt the handoff immediately, then report
  saved/index-pending if blocked; cost is delayed publication until recovery.
- **Store a frozen no-remote terminal sync result:** rejected; it would prevent
  the pending comment's unused child from starting after remote configuration.
  Ruling: return pending directly without starting that child.

- **Equate any successful context sync with comment publication:** rejected
  during self-review. Ruling: verify original checkpoint ancestry and canonical
  blob at the delegated authority OID; why: an external branch move or conflict
  resolution could otherwise publish a tree that excludes the comment. Cost
  if wrong: conservative recovery requiring inspection, not silent data loss.

These are internal, reversible mechanisms preserving approved external policy.
If Cycle 06 or a merged Wave 03 consumer changes these premises, record the
compatibility adjustment and stop for approval when behavior/scope changes.

## Verification And Evidence Gaps

Local tests cover schema/parent/identity/owned-path regressions, binding migration,
ID collision/concurrency, local-only zero-network behavior, body-free replay,
indexing states and failure/receipt reconciliation. Real custom-host SSH tests
cover roots/replies, all prior context commits, selected-key/trust failure,
two-clone merges/conflicts, accepted-but-ambiguous pushes, reopen/retry, exact
ref boundaries and privacy. Snapshot worktree files (including unrelated live
index/status), branch/ref OIDs, original commit count, comment bytes/time,
publication child count, transport counters and discovery problems.

Required local checks and native gates are detailed in the implementation plan.
The native workflow is manual-only: no enable/dispatch/publication authority is
implied. Five-target evidence remains pending until authorized execution or an
explicitly recorded owner deferral. No desktop display or Wave 03 CLI command
is a prerequisite for proving this headless Cycle.
