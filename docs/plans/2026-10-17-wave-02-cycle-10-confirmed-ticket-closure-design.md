---
title: "Wave 02 Cycle 10 Confirmed Ticket Closure Design"
date: 2026-10-17
status: approved
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M55D122D8TT6W1JXEQZQ70N8"
---

# Confirmed Ticket Closure Design

## Intent And Authority

[Cycle 10](../Cycles/wave-02-cycle-10-confirmed-ticket-closure.md) closes a
ticket through the lifecycle Cycle 09 built for document promotion, plus one
content effect: the `Close ticket <ULID>` checkpoint that records
`closed_at`/`closed_by`. Most of the work is generalizing Cycle 09's internals
so that one runner serves both kinds. The new parts are:

- a lifecycle seam over the promotion runner, integration, reservation, child
  fence and completion;
- a closure observation whose digest binds the pre-closure change set and the
  closure effect, including `closed_by`;
- the closure checkpoint, written in the integration lease (C1), with its own
  resume rule once written (C9);
- the `ticket_closures` record;
- a domain guard against writes to a closed ticket (C5).

The [implementation plan](2026-10-17-wave-02-cycle-10-confirmed-ticket-closure-implementation.md)
orders the work. The owner approved the Cycle, this design and the plan on
2026-10-17. Ticket: `01K7F6H9J2N4Q6S8V0X2Z4B6DJ`. Owner decisions
C1–C12 are in the Cycle document; passages that depend on one are marked.

Everything Cycle 09 approved applies unchanged unless this document says
otherwise. Read the
[Cycle 09 design](2026-10-15-wave-02-cycle-09-confirmed-document-promotion-design.md)
first; this design states the deltas and the rules an implementer must not
rediscover.

## Inherited Rules (Do Not Rediscover)

Each of these cost Cycle 09 an amendment or a review round. They are binding
for closure, and the generalization must preserve them for promotion.

| Rule | Source | What it means here |
| --- | --- | --- |
| Ownership-first child retry | T0-1 | Durable child ownership decides same-ID restart versus new attempt; the error category never does; uncertainty is not absence. |
| Private child execution fence | Cycle 09 QUALITY1 | Binding validation and child reservation in one immediate SQLite transaction, parameterized by parent kind. |
| Completion before cancellation | Cycle 09 Task 6/8 | Consume durable `Published`/`AlreadyCurrent` child authority before honoring cancellation; never substitute the integration OID for the publication OID. |
| Mode/type in the digest; Blob `100644`/`100755` only for canonical sources | T2-1, Task 6 SPEC | Applies to the ticket path and every change-set entry. |
| Identity: effective wins; else configuration-bound confirmation persisted to repository-local config before the commit, include-aware | T3-1, Q3-1 | Closure resolves identity at preview (C2) and again under the lease; persists through `LocalConfigEdit` and `would_persist_identity`. |
| Git-first tracking deletion, then one atomic SQL completion; uncertain commit means reopen and reconcile | T4-1 | Unchanged; the SQL finisher gains a closure arm. |
| Ambiguous backend lock after process exit: preserve, report `RecoveryRequired`, operator repair, identical retry | T6-1 | Unchanged. |
| Retained-only R1 | Task 7 | `retired_contexts` for kind `ticket`; registry loss is never reconstructed. |
| Remote delete guarded by the context child's verified publication OID, not `T(C)` | T7-1 | For closure that OID never includes the closure checkpoint (C1, C9). |
| Refresh required set inside the final SQL; index-owner epoch fence; `index_invalidated` in hand-off states | Task 7, QUALITY2 | Unchanged; closure reuses it. |
| No discovery scan under the lease; prepare outside, revalidate inside | Task 6 SPEC P2 | The closure observation uses the same `PreparedObservation` split. |
| Title-only discovery exception | Task 7 | Extended in the Privacy section for `closed_by`, which already exists in `discovered_items`. |
| `pub(in crate::repository)` breaks the source-included `production` test wrappers | T7-8 | Use `pub(in super::super)`-style paths. |

## Current Source Boundary

At planning base `13066a0` (`origin/main` `3765524` merged into the ticket
branch):

- **Kind-generic and reusable as is.** `ContextCleanup::{acquire,
  precheck_context, remove_context_worktree, delete_context_branch,
  require_local_cleanup_absent, delete_tracking_ref}` and `retire_context` in
  `src/repository/cleanup.rs`; `retired_contexts` admits `kind='ticket'`;
  `synchronize_promotion_child_observed` in `remote/sync.rs` apart from the
  fence check; R1 consumption in `sync.rs`/`state.rs`; the guarded remote
  delete (`SshDirection::Delete`, `delete_exact`, `exact_mappings` built from
  the target kind); `state.rs` `lookup_push` (action-agnostic);
  `mark_document_refresh_required` (kind-agnostic despite its name).
- **Document-only, to generalize.**
  - `cleanup.rs` `RetirementEvidence::matches_promotion` asserts
    `AuthoringKind::Document`.
  - `remote/reservation.rs` `finish_promotion` requires `Promote` and joins
    `document_promotions` with `kind='document'`; `acknowledge` hold-the-slot,
    `restart_remote_promotion` and cancel arms are `Promote`-only;
    `reserve_promotion_child` calls `promotion::record`.
  - `promotion/record.rs` `ChildExecution::validate_request` requires a
    document context target; `validate_remote_identity` maps a parent to
    `Promote`; `existing_record`, `mint_attempt` and the ID cross-checks name
    only `document_promotions`.
  - `recovery.rs` local-ID cross-checks name only `document_promotions`.
  - `read/status.rs` lists a promotion parent as an `OperationDto` with
    `action: Promote` and `state`/`completed_step` set to the step name, and
    hides its reservation row.
  - `promotion/run.rs` (13 `AuthoringKind::Document` literals; typed on
    `PromotionRecord`, `DocumentPromotionError`, `DocumentPromotionPreview`,
    `PromotionExpectation`; about 20 `record::` calls), `observe.rs`
    (`DiscoveryFacts.not_document`, branch name, `observation_guard` hashing
    `root/docs` and `context/docs`, `result_document_path`), `integrate.rs`
    (context ref, subject `Promote document <ULID>` as both commit subject and
    reconciliation key).
- **Runner behavior that constrains closure.** `run.rs` abandons every
  `Blocked`/`ReconfirmationRequired`/`ExternalChange` not at `Integrating`;
  `reset_context_after_primary_advance` requires `ContextSynchronized` and
  clears `context_oid`/`base_oid`; a context child is valid only at
  `Confirmed`; `save_record` forbids changing a set `context_oid`/`base_oid`;
  the runner passes through `ContextSynchronized` even without a remote.
- **Closure plumbing present.** `RemoteOperationAction::Close` in the schema
  `CHECK`, enum, parser, `for_context`, read DTO and status text;
  `ResultCode::TicketClosed`; `ClosureDto`; `discovered_items.closed_at` and
  `.closed_by`. The operation schema's `state` is a free string of the store's
  own step names, so listing closure parents adds values, not fields.
- **Absent.** No closure executor; no `Close` finish/restart/cancel arms; no
  domain code writes `closed_at`/`closed_by`.
- **Ticket writes.** `save_ticket_with_effective_config` writes through
  `begin_lifecycle` (an `operation_records` row) and `checkpoint_owned_paths`;
  F2 ticket save, relationship and slug bindings all funnel through
  `save_ticket_with`. Its Edit path re-serializes canonically
  (`canonical::serialize_item`), which reorders front matter: unknown keys
  first, known keys, `manyhands_*` last, `deps` sorted. Public
  `prepare_context` and `repair_item` also provision contexts.
- **Merge and resolution invariants.** `sync.rs` `preserves_item_invariants`
  refuses any change to an existing closure in a merge result, and
  `validates_resolution_sides` requires a resolution to keep every side's
  immutable values: two different closures can never be resolved in-product.
- **Error type.** `RepositoryError` is a struct with `kind:
  RepositoryErrorKind`; codes map in `mutation/outcome.rs`
  `repository_error_code` and `read/mod.rs`.

## Public Interface

```rust
pub struct PreviewTicketClosureRequest {
    pub root: PathBuf, pub item_id: ItemId,
    pub confirmed_identity: Option<ConfirmedCommitIdentity>,      // (C2)
}

RepositoryService::preview_ticket_closure(&self, request)
    -> Result<TicketClosurePreflight, RepositoryError>;           // no network, lease or write

pub enum TicketClosurePreflight {
    Ready(TicketClosurePreview),
    Blocked {
        blockers: Vec<ClosureBlocker>,                            // every blocker, fixed order
        identity_configuration: Option<ExpectedConfiguration>,    // Some iff IdentityRequired
    },
}

pub enum ClosureBlocker {                                         // fixed order, 21 variants
    NotATicket, ContextMissing, ContextNonconforming, ContextDirty, ContextConflicted,
    ConflictPending,
    LocalOperationPending, CommentPublicationPending,             // (C12)
    PrimaryMissing, PrimaryNotCheckedOut, PrimaryDirty, PrimaryConflicted,
    PrimaryAheadOfRemote,                                         // (D2)
    ContextBehindRemote,
    IntegrationConflict,                                          // (D3)
    RemoteContextDeleted, HistoryUnknown,                         // (D5)
    AlreadyClosed,                                                // (C4)
    PrimaryReopened,
    TicketInvalidAfterClosure,                                    // (D1)
    IdentityRequired,                                             // (C2)
}

pub struct TicketClosurePreview {                                 // accessors only
    item_id, ticket_path: PathBuf,
    context_branch, context_oid, primary_branch, primary_oid,
    publication_remote: Option<String>,
    tracking_primary: Option<Oid>, tracking_context: Option<Oid>,
    changes: Vec<PromotionPathChange>,                            // pre-closure, sorted
    closure: PlannedClosure,          // Record { closed_by } | Existing { closed_at, closed_by }
    integration: PlannedIntegration,  // MergeCommit | AlreadyContained
    publication: PlannedPublication,
    cleanup: PlannedCleanup,
    expectation: ClosureExpectation,                              // [u8; 32], redacted Debug
}

pub struct CloseTicketRequest {
    pub root: PathBuf, pub operation_id: OperationId, pub item_id: ItemId,
    pub expected: ClosureExpectation,
    pub approval: Option<HostApproval>,
    pub confirmed_identity: Option<ConfirmedCommitIdentity>,
}

RepositoryService::close_ticket<P: SessionCredentialProvider>(
    &self, request, session: &mut SessionCredentials<P>,
) -> Result<TicketClosureResult, TicketClosureError>;

RepositoryService::cancel_ticket_closure(&self, root: &Path, id: OperationId)
    -> Result<bool, RepositoryError>;

pub enum TicketClosureResult { Complete(TicketClosureOutcome), IndexPending(IndexPending<TicketClosureOutcome>) }
pub struct TicketClosureOutcome {
    operation_id,
    closure: ClosureRecording,            // Recorded { checkpoint_oid } | Existing
    integration: PromotionIntegration,
    publication: PromotionPublication,
    cleanup: PromotionCleanup,
}

pub enum TicketClosureError {             // nothing reached the primary
    Repository(RepositoryError), Transport(SshTransportError),
    Blocked(Vec<ClosureBlocker>), ExternalChange, ReconfirmationRequired,
    PrimaryAdvanced, ContextSynchronization(Box<SynchronizationError>),
    IdentityRequired { expected_configuration: ExpectedConfiguration },
    Busy, PollYielding, Interrupted, OperationMismatch, RecoveryRequired,
}
```

`PromotionIntegration`, `PromotionPublication`, `PromotionCleanup` and
`PromotionPathChange` are reused; their names are already public. `Debug`
shows IDs, OIDs and categories only, never `closed_by`.

**Blocker reporting.**
- `AlreadyClosed` replaces `ContextMissing` when the primary copy is closed
  and no context exists.
- `IdentityRequired` is last and is reported only when no other blocker
  exists, and only for a `Record` closure (it needs `closed_by` for the
  digest). Blockers persist as a 32-bit mask in `ticket_closures` (21 bits).

**Identity errors at execution.** `close_ticket` never returns
`Blocked([IdentityRequired])` for a missing merge signer. A `Record` closure
whose effective identity changed since preview is `ExternalChange` (C2). An
`Existing` closure that needs a merge signer and has none returns the
retryable `IdentityRequired { expected_configuration }` without ending the
record, as promotion does. A cleanup-only closure needs no identity.

## Lifecycle Seam (C3)

**Shape.** The runner becomes generic over a crate-private trait with
associated types; the promotion module keeps every existing function and type
signature as a thin wrapper, so `promotion/run_tests.rs`,
`record_tests.rs`, `observe_tests.rs` and `integrate_tests.rs` compile with
import changes only.

```rust
pub(in super::super) trait Lifecycle {
    type Record;            // PromotionRecord | ClosureRecord
    type Error;             // DocumentPromotionError | TicketClosureError
    type Preview;           // DocumentPromotionPreview | TicketClosurePreview
    type Expectation;       // PromotionExpectation | ClosureExpectation
    const KIND: LifecycleKind;
    // One method per record:: / observe:: / integrate:: call the runner makes
    // today (enumerated in plan Task 1 Step 1), with today's argument lists.
    // Plus the closure hook:
    fn pre_integration(&self, ctx: &mut LeasedIntegration<'_, Self>)
        -> Result<PreIntegration, Self::Error>;   // promotion: Ok(PreIntegration::None)
}

#[derive(Clone, Copy)]
pub(in super::super) enum LifecycleKind { DocumentPromotion, TicketClosure }
// authoring(), action(), merge_subject(item), context_ref(item)
```

| `LifecycleKind` value | Document promotion | Ticket closure |
| --- | --- | --- |
| `authoring` | `AuthoringKind::Document` | `AuthoringKind::Ticket` |
| `action` | `RemoteOperationAction::Promote` | `RemoteOperationAction::Close` |
| `merge_subject(id)` | `Promote document <ULID>` | `Integrate closed ticket <ULID>` |

Each record module owns its literal SQL; the cross-journal ID collision and
identity queries enumerate both parent tables explicitly.

**Behavior preservation gates the seam.** Plan Task 1 lands it with no closure
code; promotion's library, SSH and every affected integration target pass
unchanged before Task 2 starts. The promotion digest domain, table, migration
marker, step names, subjects and failure points are unchanged.

## Observation, Change Set And Expectation

The closure observation is the promotion observation with these changes:

1. **Item.** The item must be a ticket (`NotATicket`) on
   `manyhands/ticket/<ULID>`, at `.manyhands/tickets/<ULID>/ticket.md`, a Blob
   with mode `100644` or `100755`. `observation_guard` hashes the ticket
   directory, not `docs`.
2. **Pending work (C12).** A non-terminal `operation_records` row whose target
   is this ticket is `LocalOperationPending`. An unfinished
   `comment_publication_bindings` row for a comment on this ticket is
   `CommentPublicationPending`. Both are rechecked under the lease.
3. **Closure effect.**
   - Context ticket without closure fields: `Record { closed_by }`, where
     `closed_by` is `"<name> <<email>>"` from the selected identity (C2).
   - Context ticket with both fields: `Existing { closed_at, closed_by }`
     from the file (C4).
4. **Integration kind and the primary's closure.**

   | Context vs base | Base ticket | Context ticket | Result |
   | --- | --- | --- | --- |
   | contained | closed | any | `AlreadyContained`, cleanup only, closure `Existing` |
   | contained | open | open | `Record`, `MergeCommit` (no context child; in-memory merge) |
   | contained | open | closed | `PrimaryReopened` (closure removed on the primary by hand) |
   | not contained | closed | any | `AlreadyClosed` (internal ruling: never merge further work into a closed ticket; the context is preserved) |
   | not contained | open | open / closed | `Record` / `Existing`, `MergeCommit` |
   | no context | closed | — | `AlreadyClosed` |
   | no context | open | — | `ContextMissing` (C4) |

   "Contained, open, open" is the case of a context merged by hand without
   closure fields. Its context child is skipped: the remote context holds
   nothing beyond the base (`ContextBehindRemote` blocks otherwise).
5. **Change set.** From the base tree to the **pre-closure** result tree, as
   promotion (rename detection off; path, category, new blob OID, resulting
   mode u32 big endian, zero for deletions). The ticket path appears here only
   when the context changed it before closure; the checkpoint's own change is
   bound through the closure effect.
6. **Closed ticket and result validity (D1, C10).** Run the same
   `closed_ticket_bytes` used at execution on the result ticket, with the
   preview time as a stand-in `closed_at`: parse, set the fields, serialize
   canonically, re-parse, and require every parsed field and the body to equal
   the original apart from the two closure fields. Failure is
   `TicketInvalidAfterClosure`. No `.manyhands/worktrees/` entry may appear.
7. **Identity (C2).** Run the extracted `select_committing_identity` with the
   request's confirmation. For `Record`, no complete effective identity and no
   matching confirmation is `IdentityRequired`, with the expected
   configuration returned.

`ClosureExpectation` is `blake3("manyhands closure v1" ‖ fields)`, fixed-width
and length-prefixed (u64 big-endian lengths and counts, single-byte tags):

- item ID; ticket path; context and primary branch names;
- remote name, or a none tag;
- integration kind;
- the sorted pre-closure change set with the T2-1 mode encoding;
- closure tag (`record` or `existing`), then the `closed_by` text; for
  `existing` also the `closed_at` text exactly as in the file;
- the cleanup set;
- without a remote only, the local primary and context OIDs.

D6 applies unchanged: with a remote, moving OIDs are excluded. `closed_at` for
`record` is never bound; it is taken at execution (C6).

The 32-byte expectation is an unsalted hash over low-entropy input including
`closed_by`, so a holder of the database could confirm a guessed identity for
a known ticket. Accepted: the same database holds `closed_by` in clear in
`discovered_items`.

## Closure Checkpoint And Integration (C1, C6, C9, C10)

### Before a checkpoint exists

One lease, after the post-synchronization consent check (or directly after
`confirmed` when no context child runs). Steps, in order; none moves a ref
until step 6:

1. Re-observe (prepared outside, revalidated inside). Recompute pending work,
   change set, closure effect, identity and digest. Different:
   `ReconfirmationRequired` (abandoned).
2. Base-closed check: base ticket closed and context not contained:
   `ReconfirmationRequired` (abandoned); the next preview reports
   `AlreadyClosed`.
3. With a remote and a context that is not contained, require `base ≤
   context`; otherwise `PrimaryAdvanced` exactly as promotion (step back to
   `confirmed`, next context attempt, nothing written).
4. Identity: if confirmed, persist to repository-local config through the
   lease-held `LocalConfigEdit` with `would_persist_identity` (T3-1/Q3-1).
5. Build every object in the repository ODB, unreferenced (`Record` only):
   - `closed_at`: the record's stored value, or now in whole UTC seconds;
   - the closed ticket Blob from `closed_ticket_bytes` (C10), mode preserved;
   - the checkpoint tree (context tree with that Blob) and the checkpoint
     commit `Close ticket <ULID>`, parent `context_oid`, the resolved identity
     as author and committer;
   - the result tree: the checkpoint tree when `base ≤ context`; otherwise
     `prepare_clean_merge(base, checkpoint_commit)` (it reads commit OIDs from
     the ODB, so the unreferenced commit is visible). A conflict or an invalid
     result is `ReconfirmationRequired` (abandoned): nothing is referenced; the
     dangling objects are harmless.
   - One SQL write records step `closing` with `closed_at`, `closure_oid`
     (the commit just built) and `closure_kind = record`.
     [`ClosureAfterClosingRecorded`]
6. Move the context to the checkpoint with the proven `sync::fast_forward`
   order on the context worktree's repository: safe checkout of the checkpoint
   first, then the guarded ref update `context_oid → closure_oid`. Mark
   discovery refresh required (the context now reads closed).
   [`ClosureDuringCheckpointCheckout` between checkout and ref update;
   `AfterClosureCheckpoint` after it]
7. Merge: record `integrating` with `base_oid` and the merged context
   (`closure_oid`, or `context_oid` for `Existing`); commit `Integrate closed
   ticket <ULID>` with parents `[base, merged context]` and the prepared
   result tree [F3]; guarded primary update with safe checkout [F4]; record
   `integrated`.
8. `AlreadyContained`: no checkpoint, no commit; record the base.

`Existing` closures skip steps 5–6 (no `closing` step) and resolve a merge
signer at step 4; none available returns the retryable `IdentityRequired`.

### After a checkpoint exists (C9)

A record at `closing` with a moved context ref, or at `integrating`, has a
written closure. **It is never abandoned or superseded; a new ID for the item
is `Busy`; only the same ID finishes it.** Its consent is not re-checked: the
checkpoint is the consented effect. Same-ID retry:

1. Takes `closed_by` and the merge signature from the checkpoint commit
   itself (author signature and ticket Blob), never from current Git
   configuration, so configuration changes after a crash cannot strand or
   alter it.
2. Reads the context tip. It must be `closure_oid`, or a descendant of it
   whose ticket carries the same closure fields (a deliberate synchronization
   the user ran after a stop). Otherwise `RecoveryRequired`.
3. Base: the current tracking primary with a remote (D2 still: the local
   primary must be equal to it or behind), else the local primary.
4. Tree: the tip's tree when `base ≤ tip`; otherwise the in-memory clean merge
   `prepare_clean_merge(base, tip)`, **even with a remote**. No context child
   runs, so the closure is never pushed to the remote context branch.
5. A conflict returns `RecoveryRequired` with the record unchanged and
   guidance to synchronize the ticket context deliberately and retry the same
   ID. This rare path (a crash between checkpoint and merge plus a conflicting
   primary advance) accepts that the deliberate synchronization may bring
   collaborator content into the merge without a re-preview (C9 cost).
6. Merges as step 7 above.

### Reconciliation of `closing` and `integrating`

| Observed | Resume |
| --- | --- |
| `closing`; context ref at `context_oid`; worktree clean | Nothing written. Re-run steps 1–5 (a refusal abandons), reusing `closed_at` and, if the stored commit object still exists and still matches, `closure_oid`; otherwise rebuild it and replace `closure_oid` in the record |
| `closing`; ref at `context_oid`; worktree differs only at the ticket path, whose bytes equal the stored checkpoint's Blob | The checkout of step 6 ran; finish the guarded ref update |
| `closing`; ref at `closure_oid` or an adoptable descendant | Checkpoint exists: the C9 path |
| `integrating` | Promotion's first-parent search for `Integrate closed ticket <ULID>` with parents `[recorded base, recorded merged context]`: present → integrated; absent and local primary ≤ base → integrate again (C9 rules); otherwise `Ok` with publication or cleanup pending `recovery_required` |
| Anything else at `closing` | `RecoveryRequired` error; record kept |

**Error contract.** An error means nothing reached the primary. A written
closure checkpoint exists after an error only when the record is at `closing`
(ref moved) or `integrating`; the same ID finishes it without a second
checkpoint or a new `closed_at`.

### Why the checkpoint does not use `save_ticket`

It writes fields no save may set (C5, schema RFC), inside a lease the runner
already holds, with a record that is not an `operation_records` row; a pending
local row would block every remote reservation (`require_no_pending_local`).
The ODB path keeps one journal and reuses the proven guarded-ref and
fast-forward code.

## Records

### `ticket_closures`

Added by an all-or-nothing migration with marker
`cycle_10_ticket_closures`, listed in the registry table inventory and the
startup audit. Columns are those of `document_promotions` plus:

| Column | Meaning |
| --- | --- |
| `step` | Adds `closing` between `context_synchronized` and `integrating` |
| `closure_kind` | `record` or `existing`, set at insert from the observation |
| `closed_at` | RFC 3339 UTC, whole seconds; set with `closing`; null for `existing` |
| `closure_oid` | The checkpoint commit; set with `closing`; replaced only while the ref has not moved |
| `refusal_blockers` | 32-bit mask, `CHECK` 1..(2^21−1) |

`context_oid` keeps promotion's meaning: the synchronized pre-closure context,
set at `context_synchronized` and immutable afterwards. The **merged context**
is `closure_oid` when set, else `context_oid`; cleanup precheck, branch
deletion guard, `RetirementEvidence` and the finisher join use it. R1 holds
with `retired_oid` = merged context because it descends from the published
OID.

Step invariants (non-null columns from that step on):

| Step | Required |
| --- | --- |
| `confirmed` | identity columns, `expectation`, `closure_kind` |
| `context_synchronized` | + `context_oid` (and `remote_context_oid` with a remote) |
| `closing` | + `base_oid`, `closed_at`, `closure_oid` |
| `integrating` | + `base_oid` (all kinds) |
| `integrated` onward | + `integration_oid`, `integration_kind` |

The runner passes through `context_synchronized` even without a remote, as
promotion does. `closed_by` is never stored (Privacy). One unfinished row per
item; supersession only from `context_changed`, `ignored_files` or
`remote_context_changed` cleanup stops; `abandoned` refusals replay. A
promotion ID and a closure ID share one namespace with `operation_records`,
`remote_operation_records` and `comment_publication_bindings`.

### `retired_contexts` (R1-T)

Reused with `kind='ticket'`. `RetirementEvidence` gains a kind-checked
`matches(kind, operation, item, oid)` replacing `matches_promotion`.

## Lifecycle

```text
resolve root, configuration
 → replay: same item+digest → reconcile → resume; abandoned → stored refusal;
           different item/digest → OperationMismatch
 → new: observe; blockers → Blocked; digest ≠ expected → ExternalChange  (abandoned)
        insert row (confirmed, closure_kind)                           [F1]
 → remote and context not contained? context child (manual, merging, ownership-first)
        re-observe with fresh T(P): blocker / digest ≠ → ReconfirmationRequired
 → record context_synchronized                                        [F2]
 → lease: steps 1–5; record closing (closed_at, closure_oid)  [ClosureAfterClosingRecorded]
        checkout, then guarded context ref update  [ClosureDuringCheckpointCheckout]
                                                    [AfterClosureCheckpoint]
        record integrating; commit Integrate closed ticket            [F3]
        guarded primary update, safe checkout                         [F4]
        record integrated; refresh required
 → publication, cleanup pre-check, Close reservation, remote delete,
   worktree, branch, Git-first tracking delete, SQL completion,
   refresh                                                [F5–F12, as Cycle 09]
```

## Closed-Ticket Guard (C5)

`require_ticket_open(repo, item)` reads the ticket from the **local primary
branch tip** and, when it exists, from the **context branch tip**, never from
the filesystem (a dirty draft cannot hide closure). Either copy carrying
closure fields refuses with `RepositoryErrorKind::TicketClosed`, mapped to the
existing `ResultCode::TicketClosed` in `mutation/outcome.rs` and `read/mod.rs`.

Call sites:

- `save_ticket_with_effective_config` Edit intent (all F2 ticket save,
  relationship and slug bindings funnel here);
- public `prepare_context` for a ticket with Edit intent;
- `checkpoint_comment_locally` for a comment on a ticket.

Placement: **after `begin_lifecycle`, for a new row only**. A refusal ends that
new row rejected (operation schema: completed with `completed_step`
`rejected`, changing nothing). A resumed row (same operation ID) is not
refused: closure blocks while such a row exists (C12), so a resumed row
predates any closure. Checking under the lease also closes the race with a
concurrent closure.

Exempt: the closure lifecycle; `repair_item` (it already refuses closure-field
changes and may provision a context for a closed ticket); writes to other
tickets that reference a closed ticket (the relationships RFC never requires
removing a closed dependency). The F2 bindings' `refuse_closed` stays as the
earlier refusal; its comment saying the domain has no guard is updated.

Existing test `tests/local_authoring.rs`
`ticket_edit_preserves_unknown_closure_and_exact_body` edits a closed ticket
through `save_ticket` and expects success. Under C5 it must become a refusal
test (closure fields and body unchanged, nothing provisioned). This edit is
named in the plan for owner approval.

## Polling And Concurrency

Cycle 09's polling rules apply unchanged. Closure-specific:

- **Sequential double closure.** Clone B confirms after clone A published.
  B's context child merges A's closed primary into B's context and pushes it
  (existing synchronization behavior). B's leased re-check then sees the base
  closed and stops `ReconfirmationRequired` before writing. B's context now
  carries A's closure fields through that merge; the next preview reports
  `AlreadyClosed`, the context is preserved, and the C5 guard refuses writes
  to it. If A already deleted the remote branch, B's child returns the D5 path
  first. A preview made after B polls reports `AlreadyClosed` directly.
- **True race (C11).** Both clones pass the base-closed check before either
  publishes. Both write checkpoints and merges. The later clone's primary
  child meets a different closure on the ticket path; Cycle 06 resolution
  validation keeps every side's closure, so the conflict cannot be resolved
  in-product. The later clone reports publication pending
  `synchronization(conflict)`, cleanup awaiting publication, and the
  existing external-resolution guidance; both closures stay in history. Until
  an operator repairs it externally, that clone's primary stays diverged and
  its later promotions and closures block with `PrimaryAheadOfRemote`.
  Accepted as a rare MVP limit. The test uses two distinct identities (with
  one identity in one second the two Blobs are identical and merge cleanly).
- **Collaborator's open context of a closed ticket.** Its poll reports the
  context `remotely_deleted` and keeps it (Cycle 08). The C5 guard refuses new
  writes there once its primary is closed. Discarding it is later work.

## Privacy

- `closed_by` is personal data. It may appear in canonical Markdown, Git
  commits (author/committer and file), the existing `discovered_items.closed_by`
  column and its DB/WAL/backup copies, and the read DTO. It must not appear in
  `ticket_closures`, `remote_operation_records`, `operation_records`, any other
  workflow row, `Debug` output, errors, diagnostics or child output.
- `closed_at` is a timestamp and may appear in `ticket_closures`.
- The column-scoped title exception lives today in
  `tests/document_promotion.rs` `scan_rows`. Extract it to `tests/support/` and
  extend it to accept `closed_by` in exactly the discovery column, with
  positive and negative controls as Cycle 09 Task 7 built for titles.

## Status Read

A closure parent is listed as an `OperationDto` with `action: Close` and
`state`/`completed_step` set to its step name (including `closing`), and its
`Close` reservation row is hidden, as for promotion. `close` already exists in
the DTO and `state` is a free string of store step names, so this adds values,
not schema fields.

## Failure Boundaries

As Cycle 09, plus:

| Boundary | State left | Retry does |
| --- | --- | --- |
| Refusal before step 5 | Row `abandoned` or `confirmed`; nothing written | Re-preview, or re-synchronize after `PrimaryAdvanced` |
| Exit after `closing` recorded, before checkout | `closing`; context untouched; dangling objects | Re-check consent, then checkpoint with the stored `closed_at` |
| Exit between checkout and ref update | `closing`; worktree holds the closed Blob, ref at `context_oid` | Finish the guarded ref update |
| Exit after the checkpoint, before the merge | `closing`; context reads closed | C9 path: merge without re-consent; conflict → `RecoveryRequired` |
| True concurrent closure | Later clone publication pending conflict | External repair (C11) |

## Alternatives And Rulings

- **Checkpoint before synchronization (RFC literal order).** Rejected by C1.
- **Resynchronize the context after a stranded checkpoint.** Rejected by C9:
  needs a `closing → confirmed` path and publishes the closure on the context
  branch.
- **Byte-preserving textual splice.** Rejected by C10: canonical
  re-serialization is what every product edit already does.
- **Earliest-closure-wins resolution.** Rejected by C11 for scope.
- **Checkpoint through `save_ticket`.** Rejected: second journal; pending
  local row blocks reservations; save may not set closure fields.
- **Copy the promotion runner.** Rejected by C3.
- **One table with a kind column.** Rejected: migrating a shipped table for no
  behavioral gain.
- **Merge subject `Close ticket <ULID>`.** Rejected: the checkpoint uses it;
  integration reconciliation keys on the merge subject.
- **Bind `closed_at` in the digest.** Impossible at preview; C6 freezes it.

## Planning Review

Self-review, then an independent adversarial review against source, before
owner review. Every finding is resolved above or by an owner decision.

| # | Finding | Resolution |
| --- | --- | --- |
| 1 | Cycle 09 says Cycle 10 reuses cleanup "unchanged"; source has document-only sites | Seam is Task 1, with the list and a behavior-preservation gate |
| 2 | Promotion blocker mask full | Separate enum, 32-bit mask |
| 3 | A checkpoint through the save path leaves a pending local row that blocks reservations | ODB checkpoint, no `operation_records` row |
| 4 | Runner abandons any refusal before `Integrating`, and cannot re-synchronize from a post-checkpoint state (review C-1) | C9 resume path; consent skipped once written |
| 5 | Canonical serialization reorders hand-written tickets; byte preservation impossible; preview ran a different check (review C-2) | C10: canonical re-serialization, identical function in preview and execution |
| 6 | Two different closures can never be resolved in-product (review C-3) | C11: accept external recovery; test asserts preservation |
| 7 | Contained, still-open ticket undefined (review I-1) | Integration table in Observation item 4; `PrimaryReopened` |
| 8 | Column semantics conflicted with `save_record` immutability (review I-2) | `context_oid` keeps promotion meaning; merged context derived; invariant table |
| 9 | Checkout/ref order and merge preparation contradicted proven code (review I-3) | `fast_forward` order; commit object built before merge preparation; extra reconciliation row |
| 10 | Error contract omitted `integrating` (review I-4) | Contract covers `closing` (ref moved) and `integrating` |
| 11 | `closed_by` and signer depended on current configuration after a crash (review I-5) | Taken from the checkpoint commit |
| 12 | Guard gaps: existing test, `prepare_context`, resumed rows, ambiguous primary copy, stale comment (review I-6) | Guard section; test edit named for approval |
| 13 | Unpublished comment and pending local rows (review I-7, I-9) | C12 blockers |
| 14 | "Sequential second closure refuses before writing" overstated (review I-8) | Reworded with the child's actual effect |
| 15 | Wrong error type and mapping location (review I-10) | `RepositoryErrorKind::TicketClosed`; `mutation/outcome.rs`, `read/mod.rs`; affected tests named in the plan |
| 16 | Seam underspecified for its blast radius (review I-11) | Associated-type trait; promotion signatures kept as wrappers |
| 17 | Missing failure points for the checkpoint seams (review I-13) | `ClosureAfterClosingRecorded`, `ClosureDuringCheckpointCheckout`, `AfterClosureCheckpoint` |
| 18 | Status exposes a DTO, not "text only" (review I-14) | Status Read section: existing fields, new values |
| 19 | Privacy of `closed_by` vs existing scanners | Allowed locations and controls |
| 20 | **Residual risk:** the seam touches the F10/F11 and child-fence code that took Cycle 09 the most rounds | Task 1 gate and an independent review before any closure code |

## Verification

Unit tests: the seam (both kinds), observation and digest (each bound fact,
including `closed_by` and `existing`), each blocker and its order, the
integration table, `closed_ticket_bytes`, the checkpoint and its
reconciliation rows, the C9 path, the base-closed check, the guard and its
exemptions, the record and its invariants and cross-checks, and the
reservation `Close` arms.

Real SSH over two clones: happy path, local-only, existing closure fields,
contained-and-closed cleanup, contained-and-open closure, cache loss, every
blocker, context conflict then resolve, poll advancing `T(P)`, publication
failure, deletion failure after publication, changed remote context, offline,
sequential and racing double closure, every failure point including the three
checkpoint seams and the F10/F11 seams, poll priority, the guard after
closure, discoverability of the closed ticket from the primary with `status`
unchanged, and privacy.

Evidence gaps carried from Cycle 09: native Windows prune with open handles.
Native targets run after merge (owner decision 2026-10-17); a native failure
is follow-on work, not a gate on this Cycle.
