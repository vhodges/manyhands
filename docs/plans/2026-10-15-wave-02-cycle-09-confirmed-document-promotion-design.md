---
title: "Wave 02 Cycle 09 Confirmed Managed-Document Promotion Design"
date: 2026-10-15
status: approved
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M500Q0XQVENN0JVYK5N7Z6AX"
---

# Confirmed Managed-Document Promotion Design

## Intent And Authority

[Cycle 09](../Cycles/wave-02-cycle-09-confirmed-document-promotion.md) adds
the first lifecycle that may merge into the primary and delete context state.
Most of it is composition: deliberate synchronization, the in-memory merge
preparation, the guarded fast-forward, the remote reservation and the
index-only refresh. The new parts are:

- a promotion change set and digest;
- one primary merge path;
- a guarded remote branch deletion;
- local worktree and branch removal;
- a retirement record for first publication;
- a parent record that binds it all for retry.

The [implementation plan](2026-10-15-wave-02-cycle-09-confirmed-document-promotion-implementation.md)
orders the work. Ticket: `01K7F6H9J2N4Q6S8V0X2Z4B6DH`. The owner approved
the Cycle, this design and the plan on 2026-10-15, with D1–D7 and R1 as
recommended. Implementation and checkpoint commits were subsequently authorized
with subagent-driven development and targeted tests only until Task 8. The
owner approved the T0-1 ownership-first retry amendment below; the execution
ledger preserves the original blocker and the approval.

On 2026-10-16 the owner also approved T2-1's resulting Git mode/type consent
encoding amendment. The change-set encoding below includes that approval; the
historical quality blocker and probe evidence remain in the execution ledger.

T4-1's Git-first/atomic-SQL completion amendment was owner-approved on
2026-10-16. It supersedes the original cross-resource atomicity claim. The local
cleanup section defines the split interval and commit-uncertainty reconciliation;
the blocker/reviews are retained as history, not an unresolved decision.

The RFCs settle the external policy:

- promotion is confirmed;
- it synchronizes when a remote exists;
- it merges with a non-fast-forward commit;
- it publishes before cleanup;
- it removes the remote branch before local state;
- it never cleans up early;
- it retries only unfinished work.

Seven points the RFCs leave open are owner decisions in the Cycle document.
Passages that depend on them are marked **(D1)–(D7)**.

## Current Source Boundary

At planning base `89d04f9`:

- **`src/repository/remote/sync.rs`**
  - `synchronize_remote_observed` runs one synchronization per target.
    `SynchronizationSettings::refuse_divergence` defaults to merging.
  - Without a remote, synchronization returns `PublishPending {
    NoPublicationRemote }` and does no local integration (`:6091-6104`).
  - A `Cancelled` row replays as `Interrupted` (`:6004-6008`). A row with a
    child merge is restarted only by a merging caller (`:6043-6067`).
  - `prepare_clean_merge` (`:702`) computes a merge in a memory ODB and
    writes nothing. `import_prepared_tree` (`:758`) writes the tree.
  - `fast_forward` (`:7332`) moves a checked-out branch under a ref lock with
    a safe checkout. `committing_identity` (`:3214`) resolves the committer.
  - `materialized_target` (`:500`) and `require_clean_target` (`:554`)
    define a clean target. Status includes untracked files but not ignored
    files (`:609-610`).
- **`src/repository/remote/refs.rs`**
  - `plan_clean_integration` (`:247-275`) blocks `RemoteContextDeleted` or
    `HistoryUnknown` only when the tracking context is absent.
- **`src/repository/remote/state.rs`**
  - `remote_operation_records` already admits `action = 'promote'` and
    requires a context ref for it. The unique `remote_active_reservation`
    index allows one active remote operation per repository.
  - `remote_context_states` carries `publication_evidence`. A batch marks an
    absent `observed_published` row `remotely_deleted`.
  - `publication_evidence_for` (`:2011-2025`) reads a missing row as
    `HistoryUnknown` whenever the repository flag `history_unknown` is set.
  - The flag is set on cache loss and on remote reconfiguration (`:2105`), and
    nothing clears it. A `never_published` row is invalid while it is set
    (`:5429`).
- **`src/repository/remote/reservation.rs`**
  - Provides manual reservation, `PollYielding`, safe points,
    `finish_remote_operation` and `restart_remote_synchronization`.
- **`src/repository/transport/remote.rs`**
  - Push and fetch mappings are strings chosen by `SshDirection`
    (`:696-735`). `mappings_match_scope` admits only exact fetch and push
    mappings.
  - `push_update_reference` records per-ref status (`:473`).
  - No `push_negotiation` callback exists. git2 0.20.4 provides one, with
    `PushUpdate::src`/`dst`.
- **`src/repository/recovery.rs` and `comment_publication.rs`**
  - `require_no_pending_local` checks only `operation_records` (`:633-643`).
  - Comment publication completes its journal row first, then binds the
    child synchronization ID in `comment_publication_bindings`.
- **`src/repository.rs`**
  - `prepare_context` refuses an Edit whose branch exists without its
    worktree at another tip (`:1764-1780`).
  - `validate_context_worktree` (`:9056`) checks a context.
  - `refresh_repository` (`:1250`) is the index-only refresh.
  - `FailurePoint` and the exit-point service opener are the fault seams.
- **Absent:** no code deletes a worktree, prunes a registration, deletes a
  context branch or deletes a remote ref. git2 0.20.4 provides
  `Worktree::prune` with `valid`, `working_tree` and `locked` options.
- **Tests:** `tests/support/ssh_server.rs` `receive_effect_proven` (`:692`)
  requires a non-zero new OID.
- **F2:** `mutation/bind/resume.rs` classifies a promotion `NotResumable`.
  That classification stays.

## Public Interface

```rust
pub struct PreviewDocumentPromotionRequest { pub root: PathBuf, pub item_id: ItemId }

RepositoryService::preview_document_promotion(&self, request)
    -> Result<DocumentPromotionPreflight, RepositoryError>;     // no network, no lease, no write

pub enum DocumentPromotionPreflight {
    Ready(DocumentPromotionPreview),
    Blocked { blockers: Vec<PromotionBlocker> },               // every blocker found, fixed order
}

pub enum PromotionBlocker {
    NotADocument, ContextMissing, ContextNonconforming, ContextDirty, ContextConflicted,
    ConflictPending, PrimaryMissing, PrimaryNotCheckedOut, PrimaryDirty, PrimaryConflicted,
    PrimaryAheadOfRemote,                                      // (D2)
    ContextBehindRemote,                                       // fetched T(C) not contained in C
    IntegrationConflict,                                       // (D3)
    RemoteContextDeleted, HistoryUnknown,                      // (D5)
    DocumentInvalidAfterIntegration,                           // (D1)
}

pub struct DocumentPromotionPreview {                          // accessors only
    item_id, document_path: PathBuf,
    context_branch: String, context_oid: Oid,
    primary_branch: String, primary_oid: Oid,
    publication_remote: Option<String>,
    tracking_primary: Option<Oid>, tracking_context: Option<Oid>,
    changes: Vec<PromotionPathChange>,                         // sorted by path
    integration: PlannedIntegration,                           // MergeCommit | AlreadyContained
    publication: PlannedPublication,                           // Publish | PendingNoRemote
    cleanup: PlannedCleanup,                                   // remote_branch: bool, worktree, local_branch
    expectation: PromotionExpectation,
}
pub struct PromotionPathChange { path: PathBuf, change: PathChange /* Added|Modified|Deleted */ }
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PromotionExpectation([u8; 32]);                     // redacted Debug; to/from bytes for F2 storage

pub struct PromoteDocumentRequest {
    pub root: PathBuf, pub operation_id: OperationId, pub item_id: ItemId,
    pub expected: PromotionExpectation,
    pub approval: Option<HostApproval>,
    pub confirmed_identity: Option<ConfirmedCommitIdentity>,
}

RepositoryService::promote_document<P: SessionCredentialProvider>(
    &self, request, session: &mut SessionCredentials<P>,
) -> Result<DocumentPromotionResult, DocumentPromotionError>;

RepositoryService::cancel_document_promotion(&self, root: &Path, id: OperationId)
    -> Result<bool, RepositoryError>;            // flags the record and its active child or reservation

pub enum DocumentPromotionResult {
    Complete(DocumentPromotionOutcome),
    IndexPending(IndexPending<DocumentPromotionOutcome>),
}
pub struct DocumentPromotionOutcome {
    operation_id, integration: PromotionIntegration, publication: PromotionPublication,
    cleanup: PromotionCleanup,
}
pub enum PromotionIntegration { Merged { oid }, AlreadyContained { oid } }
pub enum PromotionPublication {
    Published { oid }, AlreadyCurrent { oid },
    Pending { reason: PromotionPublicationPending },  // NoPublicationRemote | Synchronization(category) | Cancelled
}
pub enum PromotionCleanup {
    Complete,
    Pending { reason: PromotionCleanupPending },
    // AwaitingPublication | ContextChanged | IgnoredFiles | RemoteContextChanged
    // | RemoteDeletionFailed(category) | RecoveryRequired | Cancelled
}

pub enum DocumentPromotionError {                              // nothing reached the primary
    Repository(RepositoryError), Transport(SshTransportError),
    Blocked(Vec<PromotionBlocker>), ExternalChange, ReconfirmationRequired,   // (D4)
    PrimaryAdvanced,                                           // retryable; next call re-synchronizes
    ContextSynchronization(Box<SynchronizationError>),          // incl. ConflictPending, IdentityRequired
    IdentityRequired { expected_configuration: ExpectedConfiguration },
    Busy, PollYielding, Interrupted, OperationMismatch, RecoveryRequired,
}
```

Every error variant means the primary is unchanged. Before an error is
returned, the integration reconciliation (below) proves that no matching merge
is reachable. Once integration is recorded, every later call returns `Ok`, and
publication and cleanup are reported separately. `Debug` output shows IDs,
OIDs and categories only.

## Change Set And Expectation

The preflight, post-synchronization check and integration share one Git predicate,
with filesystem discovery **prepared outside the common lease**. PreparedObservation
retains only ephemeral discovery facts and a bounded checkout/Git/configuration
guard. Under the lease it revalidates identity, local refs, index/status and known
canonical-path metadata, then recomputes the Git predicate against fresh tracking
refs/history/conflicts without calling filesystem discovery. A changed guard
refuses; a fresh tracking-primary advance still follows PrimaryAdvanced. Result
canonical validation reads immutable Git trees, not a filesystem discovery walk.
New-row supersession additionally compares the expected predecessor/owner inside
the same SQL transaction; stale discovery or a changed owner cannot revoke the
old operation's consent. Cleanup and synchronization's leased target checks use
the shared identity/commit-tree policy too, with any filesystem inspection outside
the lease. This implements the existing Bounds rule, not a policy amendment.
Canonical item sources in that shared tree validator must have regular-file mode
`100644` or `100755` **and** object type Blob. Symlink link bytes (`120000`) and
gitlinks cannot fulfill a canonical target, even when bytes parse as front matter.
The complete Git tree/snapshot remains available to the unmodified whole-tree
guards, including the forbidden `.manyhands/worktrees/` check.

1. **Context.** Find the conforming active context using the discovery rule.
   It must be materialized on `manyhands/document/<ULID>` and pass
   `require_clean_target`. The document must be the one managed document that
   carries the ULID.
2. **Primary.** It must be checked out at the root, pass
   `require_clean_target`, and have no pending synchronization conflict in the
   repository.
3. **Integration base.**
   - With a remote, the base is the stored `T(P)`. **(D2)** The local primary
     must equal `T(P)` or be its strict ancestor. If `T(P)` is absent, the
     blocker is `PrimaryMissing`.
   - Without a remote, the base is the local primary.
4. **Context freshness (remote only).** When `T(C)` exists, it must be an
   ancestor of the local context. Otherwise the blocker is
   `ContextBehindRemote`, with guidance to run deliberate synchronization
   first. The preview is therefore exact about the content the context
   synchronization will contribute.
5. **Merge result.**
   - If the context is an ancestor of the base, the result is
     `AlreadyContained` and the result tree is the base tree.
   - Otherwise, if the base is an ancestor of the context, the result tree is
     the context tree.
   - Otherwise, use `prepare_clean_merge(base, context)`. A conflict is the
     blocker `IntegrationConflict` **(D3)**. With a remote, this happens only
     while the context lacks the latest `T(P)`. The context synchronization
     fixes that, and the next check takes the second branch.
6. **Change set.** The tree diff from the base tree to the result tree, with
   rename detection off. Each entry is a path plus Added, Modified or
   Deleted. It covers comments under `.manyhands/comments/<ULID>/` and
   non-canonical files.
7. **Document validity (D1).** Parse the document from the result tree with
   the canonical-tree reader. The tree must hold no `.manyhands/worktrees/`
   entry.
8. **Publication history (D5).** This step applies only with a remote and
   only when `T(C)` is absent, which mirrors `plan_clean_integration`.
   - `ObservedPublished` blocks with `RemoteContextDeleted`.
   - `HistoryUnknown` blocks with `HistoryUnknown`.
   - Exception: when the result is `AlreadyContained`, nothing remote remains
     to publish or delete, so neither blocks. This is the cache-loss recovery
     path.
9. **Cleanup set.** The worktree and the local branch are always in it. The
   remote branch is in it when a remote is configured and the result is not
   `AlreadyContained` with `T(C)` absent.

`PromotionExpectation` is `blake3("manyhands promotion v1" ‖ fields)`, with
fixed-width, length-prefixed fields:

- item ID;
- document path;
- context and primary branch names;
- remote name, or a none tag;
- integration kind;
- the sorted change set (path, change, optional new blob OID, resulting Git entry
  mode including type bits), **owner-approved T2-1 amendment**;
- the cleanup set;
- without a remote only, the local primary and context OIDs.

**T2-1 encoding, approved.** Append the resulting mode immediately after each
change's new-OID option (present tag plus OID, or absent tag), as exactly four
bytes of **u32 big endian**. For Added and Modified, take this value from the
resulting Git tree entry, including type bits (`100644`, `100755`, `120000`,
etc.), never from local filesystem assumptions. Deleted has no resulting entry:
its new OID is absent and its mode is the explicit **zero u32 sentinel**. Do not
bind the deleted entry's old mode. The mode evidence remains internal; the
public `PromotionPathChange` shape is unchanged.

The domain stays **`manyhands promotion v1`**. No digest using the earlier
incomplete encoding has shipped; this amendment finalizes v1 rather than
introducing a compatibility migration. Existing length/count fields are u64 big
endian; category, option and cleanup tags are single bytes. The mode is appended
after the OID and before the next change (or the cleanup tags). Other fields and
their order remain as listed above.

**(D6)** With a remote, the moving OIDs are left out on purpose. The
operation's own synchronization rewrites the context with a primary merge. A
collaborator's already-published primary advance moves the base without
changing what this promotion contributes or publishes. Because of D2, the
local primary cannot carry unpublished work.

Result-mode binding preserves this invariance: it binds only changed result
entries, without adding moving commit/tree OIDs or unrelated primary-only paths.

**(D4)** A collaborator's new context commits change the change set, so the
digest catches them.

## Records

### `document_promotions`

Added by an all-or-nothing migration with marker
`cycle_09_document_promotions`, and listed in the registry table inventory.

| Column | Meaning |
| --- | --- |
| `repository_id`, `operation_id` (unique), `item_id` | Identity |
| `expectation` | 32-byte digest bound at the first call |
| `remote_name` | The frozen remote name, or null. A change interrupts the operation |
| `step` | `confirmed`, `context_synchronized`, `integrating`, `integrated`, `published`, `publication_pending`, `remote_deleted`, `worktree_removing`, `worktree_removed`, `branch_deleted`, `complete`, `abandoned`, `superseded` |
| `context_attempt`, `context_sync_id`, `primary_attempt`, `primary_sync_id` | The current child attempt number and ID. A new attempt gets a new ID, minted and stored before the child first runs |
| `base_oid`, `context_oid`, `integration_oid`, `integration_kind` | Integration evidence. `integration_oid` is the merge commit or the contained base |
| `remote_context_oid` | Fetched tracking-context evidence recorded after synchronization, possibly absent/stale relative to its push. Remote deletion instead uses the bound context child's durable verified publication OID |
| `publication_category`, `cleanup_category` | Fixed redacted categories for a pending field |
| `cancel_requested`, `updated_at` | Cancellation flag and timestamp |

A partial unique index allows one row per item whose step is not
`complete`, `abandoned` or `superseded`.

Inserting a new row for an item marks the old row `superseded`, in the same
transaction, when the old row stopped on `context_changed`, `ignored_files`
or `remote_context_changed`. Any other unfinished row makes the insert
`Busy`.

A pre-integration refusal records `abandoned`, and replaying that ID returns
the same refusal. These refusals are `Blocked`, `ExternalChange` and
`ReconfirmationRequired`.

The row is not an `operation_records` row, so child synchronizations can still
reserve, and `require_no_pending_local` is unchanged.

ID cross-checks:

- `begin_or_reconcile` refuses an ID that names a promotion.
- A promotion refuses an ID that names a local record, or a remote record
  other than its own `promote` reservation (`OperationMismatch`).

### `retired_contexts` (R1)

| Column | Meaning |
| --- | --- |
| `repository_id`, `kind`, `item_id` (unique per repository) | The retired context |
| `operation_id`, `retired_oid` | The promotion and the context OID it deleted |

The row is written only in the final cleanup transaction, after the remote
deletion has been observed, or when there is no remote. Three changes in
`state.rs` and `refs.rs`:

- **First-publication evidence.** `publication_evidence_for`, and every
  first-publication eligibility check, read a missing state row for a retired
  item as `NeverPublished`, even when `history_unknown` is set, only with a
  **retained validated retirement row**. An existing state row takes precedence.
  **Owner-approved Task 7 amendment (2026-10-16):** registry loss erases this
  evidence. Stay HistoryUnknown across reopen pending explicit recovery or
  republication; absence/an old merge never reconstruct retirement. No new
  republication API is supplied by this Cycle.
  Read projections additionally expose current-generation verified Push proof as
  ObservedPublished, even before authority classification and with a distinct
  Push endpoint. Existing Fetch fields are never invented: private
  `fetch_publication_evidence_for` retains Fetch-only history/R1 semantics for
  synchronization graph planning, preflight and polling. The read overlay uses
  validated journal evidence only for already-visible identities, with an audited
  context-only lookup index (`cycle_09_context_publication_evidence` migration).
- **Removal.** An observation batch that records the item as
  `observed_published` deletes the retirement row in the same transaction.
  Verified Push publication consumes it with the verified checkpoint too,
  including before authority classification or with a distinct push endpoint.
  This includes legacy reconciliation upgrading PushVerified, direct settlement
  verification and already-current candidate binding, each in the same SQL
  transaction as its verified write. Classification is not another observation
  and must not consume a NEW retirement earned after that older verification.
  Pre-retirement Push journals are retired only when their candidate is contained
  in the retained row's actual retired tip. New ambiguous/verified attempts keep
  the ordinary absence barrier; neither ULID/timestamp ordering nor an old merge
  is a substitute for the validated retained retirement row.
- **Batch.** A retired item has no state row, so a batch never reports it
  `remotely_deleted`.

No other reading of `history_unknown` changes. The table is not in the
versioned read contract.

## Lifecycle

```text
resolve root, configuration                              (invalid → Repository error)
 → replay: same item+digest → reconcile → resume at step; abandoned → stored refusal;
           different item/digest → OperationMismatch
 → new: observe; blockers → Blocked; digest ≠ expected → ExternalChange   (row abandoned)
        insert row (confirmed)                                   [F1 AfterConfirmed]
 → remote? context child attempt (manual, merging)
        authoritative completion → stored result; never rerun child
        retained conflict / owned resolution → same ID, restart (even Interrupted)
        ConflictPending / IdentityRequired → return; same attempt continues later
        no retained ownership: cancelled / interrupted / rejected / transport → new attempt
        uncertain ownership → recovery; never infer absence or replace attempt
        re-observe with fresh T(P): blocker or digest ≠ expected → ReconfirmationRequired (abandoned)
        record context_synchronized, context_oid, remote_context_oid   [F2 AfterContextSync]
 → lease: re-observe; recompute digest (≠ → ReconfirmationRequired, abandoned)
        remote: base ≰ context_oid → PrimaryAdvanced; step back to confirmed, next attempt
        record integrating (base, context)
        build tree; commit "Promote document <ULID>" [base, context]   [F3 BeforeIntegrationRef]
        guarded update primary old → merge, safe checkout       [F4 AfterIntegrationRef]
        record integrated; mark refresh required
 → remote? primary child attempt (manual, merging)
        same ownership-first attempt rules as context child
        Published/AlreadyCurrent → record published              [F5 AfterPublication]
        otherwise → record publication_pending(category); refresh; return partial
   no remote → record publication_pending(no_publication_remote)
 → lease: cleanup pre-check (registered, at context_oid, clean, no ignored, not locked)
        failure → cleanup pending(context_changed | ignored_files); refresh; return
 → remote? reserve promote (held to completion)                 [F6 BeforeRemoteDeletion]
         delete C guarded by the context child's verified publication OID; advertise
        → record remote_deleted                                 [F7 AfterRemoteDeletion]
 → lease: re-check; record worktree_removing; prune → worktree_removed   [F8 DuringWorktreeRemoval]
 → lease: delete local C guarded at context_oid → branch_deleted         [F9 BeforeBranchDeletion]
  → durable branch_deleted intent; lease + ownership/config checks:
         delete T(C) guarded at expected direct OID or verified true absence
    → one SQL transaction: delete state row; insert retired_contexts;
         record complete; finish applicable reservation          [F10 BeforeCompletion]
                                                                 [F11 AfterCompletion]
 → index-only refresh with a fresh internal ID, outside the lease        [F12 RefreshFailure]
 → Complete | IndexPending
```

F1–F12 become `FailurePoint` variants, and the Cycle's acceptance row refers
to these twelve. Cancellation is honored at each child safe point and between
steps before integration. After integration it is honored only between whole
steps.

### Context Synchronization

The step calls `synchronize_remote_observed` with
`SynchronizationTarget::Context` and the current attempt's ID. It passes
`restart` when that child row exists, and a forwarding observer that maps
promotion cancellation onto `Cancel`. It passes `confirmed_identity` through.

**Private parent-issued execution fence (QUALITY1 concurrency correction).** The
production handoff uses a private promotion-child observed adapter over that same
synchronization engine. Its process-only ChildExecution carries repository/parent/
item/expectation, role, attempt number and child ID. It holds no lease or SQL guard.
Entry, configuration-write and, critically, reservation/restart SQL validate the
**current durable parent binding**. Binding validation and child reservation occur
in one immediate SQLite transaction, serialized with attempt replacement. Thus a
worker paused after C1 was persisted cannot start C1 as an ordinary synchronization
after a peer binds C2; if C1 reserves first, replacement sees its actual row and
ownership under SQL. Ordinary public synchronization remains unchanged. Current
authoritative handoff remains an offline replay even after parent completion;
cancelled/terminal/superseded parents cannot authorize new unstarted work.

No persistent claim fields/table or whole-parent lock is added; existing audited
binding/attempt fields and remote-owner fencing suffice. No-row Busy/yielding and
the ownership-first retry table remain unchanged. This is a private execution
protocol correction, not a new consent or cancellation policy.

For both context and primary children, inspect authoritative completion and
persisted ownership before classifying the returned error. Retained conflict
or owned-resolution evidence takes precedence over the error category
(owner-approved T0-1 amendment). Uncertain ownership is not absence: surface
recovery rather than replace a possibly owned child.
This ordering also applies when the outer child return is lost and the parent
concurrently requests cancellation: consume durable Published/AlreadyCurrent
authority first. Cancellation may stop unfinished work, not turn completed
publication into publication-pending Cancelled. After primary completion, report
the stored kind/exact OID and cleanup pending Cancelled; after context completion,
record context synchronization before stopping integration. Retry reuses the same
completed child without publication work.

How the child's end state and ownership decide the next call:

| Child end state | Next call |
| --- | --- |
| Completed | Use the stored result; never re-run |
| `Interrupted` with a retained conflict or owned resolution | Original attempt ID with `restart=true` |
| `ConflictPending` (including a conflict whose inspection requires external resolution), `IdentityRequired` | Same ID with `restart`, after the user's canonical/external resolution or identity confirmation |
| `Cancelled`, `Interrupted`, `PushRejected`, transport failure, `Busy`, `PollYielding`, `RecoveryRequired` with confirmed absence of retained ownership | New attempt and new ID |
| `RecoveryRequired` with a retained merge candidate, conflict or owned resolution | Same ID with `restart`; a persistent failure surfaces as `ContextSynchronization(RecoveryRequired)` before integration, or `Ok` with `publication: pending(synchronization(recovery_required))` for the primary child after integration |
| Ownership cannot be determined | Surface recovery; do not mint a replacement attempt on an assumption of absence |

`ExternalResolutionRequired` is reported by the existing conflict read/resolve
flow, not by `synchronize_remote_observed`. An external-only conflict remains
the original child's `ConflictPending` until a valid external repair allows
same-ID restart to continue.

Task 0 characterizes each row against the real child before the table is
fixed. A lost push result is answered by the child's own replay as published,
with one push.

### Integration

The integration step runs in one lease, after the observation has been
recomputed.

- **Parents.** The parents are `[base, context]`.
- **Remote only.** The base must be an ancestor of `context_oid`. A poll that
  ran between the steps may have advanced `T(P)`. If it did, the step returns
  `PrimaryAdvanced`, steps the record back to `confirmed` with the next
  context attempt, and writes nothing. This prevents a merge that takes the
  context tree from silently dropping primary commits.
- **Tree.** The tree is the context tree, or, without a remote, the imported
  prepared merge tree. A conflict at this point is `ReconfirmationRequired`.
- **Identity.** `committing_identity` supplies the signature. A missing
  identity returns `IdentityRequired` before anything is written. Extract its
  read-only effective-identity/configuration-digest/confirmation selection for
  reuse, leaving synchronization's owned identity journal unchanged. A complete
  effective identity wins even when a confirmation is supplied. Otherwise an
  exact configuration-bound confirmation, with the same empty/NUL validation,
  is required. Persist both confirmed values to **local Git configuration before
  committing**, as required by Git RFC Identity and MVP RFC Identity, SSH, and
  Secret Handling. Reuse the bridge's lease-held atomic config primitive, not
  its public setter, observation or stricter validation. Re-observe identity and
  confirmation under the integration lease and Git config lock before install;
  preserve unrelated keys and initials. The config replacement is one atomic
  rename with no promotion identity journal. A foreign lock is preserved and
  refused. An installed confirmed config remains installed if a later merge/ref
  step fails: the error invariant concerns the primary, not config rollback.
  `AlreadyContained` requires no identity. **T3-1 internal engineering ruling:**
  this composes existing approved mechanisms and RFC policy; it needs no owner
  policy amendment. The historical commit-only suggestion in the ledger is
  superseded and must not be implemented.
- **Ref update.** One guarded ref update moves the local primary from its old
  value to the merge. This is a fast-forward because the local primary ≤
  base ≤ merge.
- **Already contained.** `AlreadyContained` writes no commit and records the
  base.

Task 3 exposes immutable integration request/evidence values and lease-owned
preparation/candidate stages. Task 5 supplies storage; the future runner persists
`integrating` evidence before commit/ref application and uses the candidate seam
for F3 and the returned outcome for F4. No Task 5 storage or failure-point stub is
needed in Task 3. Integration uncertainty is a pending category. An error after
a possibly effective guarded update is reconciled against Git before returning.

**Reconciliation of `integrating`.** A merge counts as present when a commit
reachable from the local primary has subject `Promote document <ULID>` and
parents `[recorded base, recorded context]`. The search walks first parents
from the local primary back to the recorded base.

- A merge is present: the step is integrated. This holds even if the primary
  has since moved on, for example after the user's primary synchronization.
- No merge is present, and the local primary ≤ base: run the step again. A
  dangling commit object is harmless.
- Otherwise: `RecoveryRequired` as a cleanup or publication pending
  category. It is never returned as an error, because the primary may have
  changed.

### Publication

The publication step runs the primary child with merging enabled, following
the same attempt table. If a collaborator advanced the primary after the
context fetch, the child merges with `Merge remote primary` or parks a primary
conflict, as Cycle 06 already defines.

When the child does not complete, the step records
`publication: pending(synchronization(category))` and
`cleanup: pending(awaiting_publication)`. The category is one of conflict,
rejected, transport, identity, busy, yielding, cancelled or `recovery_required`.
`recovery_required` is a fixed, redacted category for a persistent primary-child
recovery failure with retained merge/conflict/resolution ownership, or when
ownership cannot be determined. It contains no raw error, endpoint or content.
The operation returns an `Ok` partial outcome with that publication category
and cleanup awaiting publication, never a post-integration `Err`. Retained
ownership keeps the original child ID; uncertain ownership does not authorize
a replacement attempt. The child is the only code that pushes the primary.

### Remote Deletion

Add `AuthenticatedSshRemote::delete_exact(plan, target, expected: Oid) ->
DeleteObservation`, where `DeleteObservation` is `Deleted`,
`StillPresent(oid)` or `Changed(oid)`.

- **Mapping.** The mapping is `:refs/heads/manyhands/document/<ULID>`, under
  a new `SshDirection::Delete`. `mappings_match_scope` admits it only for a
  context target.
- **Guard.** A `push_negotiation` callback refuses unless the update's source
  OID equals `expected`, so the server never receives a deletion of an
  unexpected value.
- **Result.** A fresh advertisement decides the result. An ambiguous
  transport error re-advertises on the next attempt before it pushes again.
- **RFC mapping.** `Changed` and an unresolvable ambiguity are the Git RFC's
  "stops and records recovery-required". They surface as
  `remote_context_changed` and `remote_deletion_failed(category)`.

The step runs inside a manual `promote` reservation with the context ref,
taken at F6 and finished in the completion transaction. It reaches the
`BeforeTransport` and `AfterAdvertisement` safe points and uses the existing
generation fence. While the reservation is held:

- a poll is `Busy`;
- a poll that started earlier is asked to yield;
- a configuration or endpoint change interrupts the step.

The reservation module gains `promote` finish and restart rules. A restart
fences the owner epoch and resumes from the recorded step.

### Local Cleanup

The order follows the RFC: remote, then worktree, then branch.

**Worktree pre-check (D7).** The worktree passes when all of these hold:

- it passes `materialized_target`;
- `HEAD` is at `context_oid`;
- `require_clean_target` passes;
- a second status with `include_ignored` is empty;
- the worktree is not locked.

The same check runs before remote deletion and again under the lease before
the prune.

**Prune.** The step records `worktree_removing`, then calls
`Worktree::prune` with `valid` and `working_tree` set and `locked` unset.

**Reconciliation of `worktree_removing`:**

| Observed | Result |
| --- | --- |
| Path and registration both absent | Done |
| Both present and the pre-check passes | Run again |
| Anything else | `cleanup: pending(recovery_required)`, nothing more removed |

**Branch.** A ref transaction deletes the branch only when its value is
`context_oid` and no worktree has it checked out.

**Completion (T4-1, owner-approved 2026-10-16).** Persist existing planned
`branch_deleted` intent before the tracking-ref effect. Under the lease and fresh
ownership/configuration checks, delete `T(C)` only at the expected **direct OID**
or accept verified true absence. Wrong/new conflicting values, symbolic refs
(including dangling ones) and unreadable refs stop before any partial SQL change.
Then one atomic SQL transaction does all of the following:

- deletes the item's `remote_context_states` row;
- inserts `retired_contexts`;
- records `complete`;
- finishes the reservation.

In the split interval `T(C)` may be absent, while applicable existing history
stays intact (`observed_published` when present), no retirement exists, and the
durable record/applicable reservation remain pending/held. Do not manufacture
observed-published history for local-only/cache-loss cases. Git deletion is not
rolled back by SQL: rollback of uncommitted SQL preserves SQL only. If a commit
result is uncertain, reopen and reconcile actual completion and owned record;
committed completion plus reservation finish replays the stored outcome. Failures
after integration return `Ok` cleanup pending `RecoveryRequired` plus refresh.
Same-ID retry rechecks actual Git, ownership and configuration and finalizes SQL
without repeating completed lifecycle work. Cache loss/ref absence alone is not
retirement proof. No additional phase/table is needed; `branch_deleted` is the
prior durable intent. Tasks 5–7 test split boundaries inside F10/F11, SQL statement
rollback and commit uncertainty/replay. Task 4 exposes guarded Git deletion and
private validated evidence for SQL retirement; Tasks 5/6 compose completion and
reservation finish into that SQL transaction. History is never reset early.

**T6-1, owner-approved 2026-10-16 — backend-lock exception.** This Cycle inherits
[Cycle 06's backend recovery amendment](../Cycles/wave-02-cycle-06-merge-and-conflict-recovery.md)
and its approved 2026-10-08 Task 4 resolution protocol. A process exit inside
F10/F11 can leave an ambiguous libgit2 lock. Preserve it and all actual effects;
return `Ok` cleanup pending `RecoveryRequired` plus refresh/`IndexPending` before
any operator repair. Do not automatically delete/adopt the lock or infer ownership
from contents, age or PID, and add no lock journal. Only an operator who quiesces
writers and verifies/repairs the backend metadata while preserving unrelated
history permits the **identical request** to revalidate actual refs/ownership and
resume unfinished SQL. This rare interval explicitly costs operator intervention.
Pending SQL keeps applicable existing history, no retirement and a held reservation;
actual committed completion/finish replays its stored outcome instead. Process-exit
tests assert preservation and no repeated effect counters **before** a disposable-
fixture-only simulation of verified operator repair, then finish/replay the same
request. This qualifies the immediate-retry language above and all F10/F11 gates;
it does not relax ordinary automatic reconciliation or the T4-1 transaction.
If the atomic SQL completion/finish actually committed before process exit, that
durable complete row is replay authority even when a backend lock survives. Replay
preserves the lock and performs no further cleanup effect; it never treats lock
absence, contents or cache loss as completion proof.

The cleanup functions take a kind and an item ID. Cycle 10 reuses them
unchanged.

**Correction 2026-10-18 (Cycle 10).** Cycle 10 did not reuse the cleanup path
unchanged. Several sites around it were document-only. Cycle 10
[Task 1](2026-10-17-wave-02-cycle-10-confirmed-ticket-closure-execution.md#task-1--lifecycle-seam-2026-10-17)
made the runner generic over a lifecycle seam. Cycle 10
[Task 5](2026-10-17-wave-02-cycle-10-confirmed-ticket-closure-execution.md#task-5--reservation-close-arms-retirement-and-completion-2026-10-18)
generalized `RetirementEvidence::matches_promotion` to a kind-checked
`matches`, added the `Close` reservation, finish and restart arms, and made
the `exact_mappings` Delete arm generic over the context kind. Promotion
behavior was kept, under a behavior-preservation gate.

## Reconciliation On Retry

The row says where to look, and Git decides.

| Row step | Git observation | Resume at |
| --- | --- | --- |
| `abandoned` | — | The stored refusal |
| `confirmed` | — | Re-check consent, then the context child per the attempt table |
| `context_synchronized` | Context at `context_oid` | Integration; otherwise `ExternalChange` (abandoned) |
| `integrating` | Per the integration reconciliation | Integrated, or integrate again |
| `integrated`, `publication_pending` | — | Primary child (remote) or the cleanup pre-check (no remote) |
| `published` | — | Cleanup pre-check, then deletion |
| `published` + reservation | Remote context absent | Record `remote_deleted` |
| `remote_deleted`, `worktree_*`, `branch_deleted` | Per the local cleanup rules | Next local step |
| `complete` | — | Stored outcome; refresh if still required |

**Cache loss.** Suppose the registry is lost and Git holds a `Promote document
<ULID>` merge on the primary while the context still exists. The preflight
then reports `AlreadyContained` with cleanup only. When the remote branch is
gone, the D5 exception applies. The caller previews and confirms a new ID.
Nothing is integrated again. This is the RFC rule that cache loss requires
recovery instead of blind replay.

This newly confirmed, actually checked cleanup earns a new retirement row on
completion. It does not reconstruct a lost old row. A fresh edit after *completed*
promotion plus registry loss remains HistoryUnknown when no such new cleanup has
earned retirement. Other items remain unknown in the same repository; a later
observed publication consumes retirement and subsequent remote deletion again
blocks ordinary republication.

### Task 7 privacy amendment — owner approved 2026-10-16

Only already accepted `discovered_items.title` metadata and its database/WAL/
backup copies may contain titles (the existing Cycle 08 exception). All workflow,
promotion, retirement, remote, recovery and operation rows, including backup row
scans, plus formatted results/diagnostics/child output remain title-free. There
is no new title-bearing workflow write. Bodies/comment bodies/endpoints,
passphrases/credentials/key material and rejected non-indexed title sentinels stay
forbidden in raw whole stores, sidecars and backups. Tests positively attest
accepted discovery titles in both live clone stores and both backups, in
checkpointed and pinned-WAL variants. Negative controls update an existing
`operation_records.target` metadata field transactionally in each of those four
stores, require the unchanged all-column scanners to reject the title, then
rollback and compare known row snapshots. A separate public save containing the
non-indexed title sentinel plus NUL exercises actual canonical validation; its
formatted error, unchanged source/branch, discovery, and both raw stores/sidecars/
backups prove rejection and absence. These are evidence controls, not policy changes.

## Polling And Concurrency

The original fetched `remote_context_oid` is not necessarily the verified remote
tip: first publication may leave T(C) absent, and a push may leave it stale. The
delete guard reads completed child authority. Local retirement admits only the
recorded fetched OID or that exact known verified OID if an allowed pre-F6 poll
fetched it; arbitrary newer/symbolic/unreadable refs still stop. A Changed delete
on a shared Fetch/Push endpoint fetches the peer replacement with existing exact
scoped Fetch, without merging it. A distinct Push endpoint never writes its
observation into another endpoint's Fetch tracking ref.

Final SQL completion marks repository discovery stale and atomically fences its
active index observations through the existing `index_owner_epoch` protocol.
Each affected row is retained as `index_invalidated`, with identity and
authoritative step unchanged. An old token cannot persist either captured
observation or clear the stale flag. Its exact same record may CAS-reclaim the
one-step fence and perform one bounded fresh pass under a new epoch/heartbeat;
foreign, reclaimed, completed and abandoned records are not adopted. Fresh scans
remain outside repository/cache guards; no index-owner row is silently deleted.
An active authoritative child index handoff can finish normally, and legitimate
ownership loss retains IndexPending behavior. Stored promotion completion wins
over an older same-ID executor's unfinished snapshot after that handoff.

If the original index executor is lost, `index_invalidated` also qualifies as an
existing local index-handoff state for public `operation.resume`, but only with
the same authoritative completed-step proof required of every other handoff
state. State alone never authorizes recovery of an unfinished write or a bare
refresh. Resume claims a fresh epoch and re-observes; it does not replay the
authoritative effect. The remote journal still takes precedence: promotion is
NotResumable even when a local shadow row meets the handoff predicate. This is
compatibility with established local recovery, not the deferred F2 binding.

The hidden `set_refresh_persist_hook_for_testing` barrier is after BOTH refresh
observations and the owner touch, before persistence, without a lease/transaction.
It proves genuine older-refresh/F11 races, including another root in the same
registry and an already-reclaimed owner. Existing hidden FailurePoint support additionally
names `PromotionAfterTrackingDeletion` and `PromotionAfterCompletionCommit`
inside F10/F11; these are test seams, not extra lifecycle phases or production
automatic lock repair. T6-1 preserves the ambiguous lock before operator repair,
and stored completion preserves a surviving lock after SQL commit.

- **Manual priority.** The children and the `promote` reservation all reserve
  with manual priority, so a running automatic poll yields to each of them.
- **Polls between steps.** From confirmation until F6, an automatic poll may
  run between steps. It can fast-forward a clean, strictly behind primary or
  context, or advance `T(P)`. Every step re-observes, and the integration rule
  above handles an advanced `T(P)`.
- **During deletion and cleanup.** From F6 to completion the reservation is
  held, and a poll is `Busy`. After completion the item has no state row and
  no tracking ref, so a poll reports nothing for it.
- **Another clone** keeps its local context. Its next poll reports the context
  `remotely_deleted` and preserves it.

## Failure Boundaries

| Boundary | State left | Retry does |
| --- | --- | --- |
| Blocked, external change, reconfirmation | Row `abandoned`, or no row | Re-preview with a new ID |
| Primary advanced at integration | Row `confirmed`, next attempt | Re-synchronize, then integrate |
| Context child fails or conflicts | Child row per Cycles 05/06; primary untouched | Same or new attempt per the table |
| Crash during integration | `integrating`; primary old or new | Reconcile; never a second merge |
| Publication fails | `publication_pending`; merge local; context intact | Primary child, then cleanup |
| Context edited, untracked or ignored files, or locked after integration | `published` or `publication_pending`; every ref kept | Report until resolved; a new promotion supersedes |
| Remote deletion rejected or changed | `published`; all local state; history `observed_published` | Delete again, or supersede after synchronizing |
| Crash after the remote deletion | `published` and remote absent | Recorded as deleted, then local cleanup |
| Partial worktree removal | `worktree_removing` | `recovery_required`; explicit repair |
| Ambiguous backend lock after process exit, including inside F10/F11 | Actual effects/history/retirement/parent/reservation preserved | Pending RecoveryRequired plus refresh first; identical retry after quiesced verified operator repair only (T6-1) |
| Refresh fails | Any `Ok` return | `IndexPending`; any refresh repairs it |

**Edge case.** A `superseded` row whose worktree was already removed, but
whose branch then moved outside Manyhands, leaves a branch with no worktree.
`prepare_context(Edit)` refuses that branch (`repository.rs:1769`). This
requires external tampering and is reported as the existing mismatched-context
recovery.

## Alternatives And Rulings

- **One `promote` reservation across the whole lifecycle.** Rejected.
  Children reserve their own rows, and the unique active index admits only
  one. Ruling: children are bound per attempt, as comment publication binds
  them, and the `promote` reservation covers only deletion through
  completion. Cost: polls can run between earlier steps, which is safe
  because every step re-observes and of the integration base rule.
- **Merge without first synchronizing the context.** Rejected when a remote
  exists. The RFC and PRD order requires synchronization first.
- **Skip pushing the context because it is about to be deleted.** Rejected.
  The child is the RFC's synchronization and the only proven push path.
- **Fast-forward-only integration.** Forbidden by the MVP RFC.
- **Merge subject.** `Promote document <ULID>`: deterministic, free of user
  text, and distinct from the synchronization subjects.
- **Clear the repository `history_unknown` flag, or write a
  `never_published` row.** Rejected. Clearing the flag would weaken first
  publication for every item, and the row violates an existing invariant.
  Ruling R1: an additive `retired_contexts` table scoped to contexts this
  operation proved it deleted.
- **Reset the history early in cleanup.** Rejected. A cleanup that stopped
  early would then allow a silent republish.
- **Finish a partly removed worktree.** Rejected, matching Cycle 08 D2.
- **A document lifecycle marker.** The canonical schema has none.
- **Change the F2 resume classification now.** Out of scope. C5 and D5 own
  the boundary binding.

These rulings are internal and can be reversed within the approved contract.
A Task 0 characterization that contradicts one stops work for an amendment.

## Verification And Evidence Gaps

Unit tests cover:

- the observation and the digest;
- each blocker;
- the three merge-tree branches;
- the integration base rule;
- reconciliation of each step;
- the attempt table;
- the delete mapping and the negotiation guard;
- the cleanup helpers, including ignored and locked worktrees;
- retirement.

Real SSH tests over two clones cover:

- the happy path, local-only promotion, an already-contained context, and a
  fresh context after an edit, including in a repository with unknown
  history;
- every blocker, D2 through D5 included;
- a context conflict and its resolution;
- a poll advancing `T(P)` mid-promotion;
- a rejected publication, and promotion while offline;
- a rejected remote deletion, and a changed remote context;
- interruption at F1 through F12 followed by reopen;
- polling priority, and polls that run during and after promotion;
- privacy.

Each preservation case snapshots, before and after:

- worktree bytes, index and status;
- branch, tracking and remote refs;
- worktree registrations.

Gaps to close with evidence rather than assumption:

- `push_negotiation` with the locked libgit2 and libssh2;
- `Worktree::prune` on Windows with open handles and long paths;
- the native targets, which run only by separately authorized manual
  dispatch.
