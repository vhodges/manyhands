---
title: "Wave 02 Cycle 08 One-Shot Polling And Materialization Design"
date: 2026-10-11
status: approved
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M4NM26Q2CZH0CGBT31QVZA0W"
---

# One-Shot Polling And Materialization Design

## Intent And Authority

[Cycle 08](../Cycles/wave-02-cycle-08-one-shot-polling-and-materialization.md)
turns the Cycle 04 advertisement-only observation into the full one-shot poll
the RFCs describe. It composes existing pieces: the selected-key transport,
remote reservations and safe points, the observation batch, the guarded
fast-forward from synchronization, and the index-only refresh. The new
mechanisms are a poll-set fetch, policy eligibility with a persisted next
eligible time, tree-based conformity validation, and worktree materialization.
The [implementation plan](2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-implementation.md)
orders the work. The [execution ledger](2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-execution.md)
records preflight, grounding and the decision audit. Ticket:
`01K7F6H9J2N4Q6S8V0X2Z4B6DG`. The owner approved the Cycle, this design and the
plan on 2026-10-11 and authorized their planning commit. This does not
authorize implementation.

External policy is settled by the Git, index, MVP and runtime RFCs: what a
poll may change, the default interval and backoff bounds, manual priority, and
that explicit Poll now bypasses pause and backoff once. Four points the RFCs
leave open were owner decisions in the Cycle document. The owner approved the
recommended answer to each on 2026-10-11; the dependent passages are marked
**(D1)–(D4)**.

## Current Source Boundary

At planning base `d1ca88772c5768d55aadb2ca4dcac1c69dab0d17`:

- `src/repository/remote/observation.rs`: `observe_publication_remote` runs
  `BeforeTransport → advertisement → AfterAdvertisement → BetweenObservations*
  → BeforeBatchCommit → commit batch → AfterBatchCommit → finish`. Explicit
  maps to manual priority, automatic to poll priority. It never reads the
  stored policy. `transport_category` maps transport errors to outcome
  categories.
- `src/repository/remote/reservation.rs`: one active remote record per
  repository. A manual request that meets a poll-priority record sets
  `yield_requested` and returns `PollYielding`; every other overlap is `Busy`.
  Safe points `BeforeFetch` through `BeforeDiscovery` are rejected for a poll
  target; `BeforeLocalMutation` is accepted but unused.
  `restart_remote_observation` refuses a record at `BeforeLocalMutation` and
  completes one at `AfterBatchCommit`. `finish` owns the only backoff
  computation and `finish_remote_operation` allows `Completed` only at
  `AfterBatchCommit`. Only synchronization targets see `Busy` for a pending
  conflict.
- `src/repository/remote/state.rs`: `remote_polling_state` holds enabled,
  paused, interval, backoff, recovery suspension and latest outcome. There is
  no attempt time and no next eligible time. Schema changes are made by
  probing `PRAGMA table_info` and adding columns, or by the
  rename-recreate-copy pattern when a `CHECK` list changes.
  `remote_context_states` is written only by the batch commit; `observed` is
  derived at read time from the local index.
- `src/repository/transport/remote.rs`: `fetch_exact` downloads one target's
  one or two refspecs with `prune` off and writes tracking refs itself under
  the repository lease with `reference_matching` old-value guards. No method
  fetches the poll set. Callbacks register credentials and certificate checks
  only; there is no transfer-progress callback and no in-transfer stop.
- `src/repository/remote/sync.rs`: `materialized_target`,
  `require_clean_target`, `local_oid` and `fast_forward` are private free
  functions typed on `SynchronizationTarget`. `fast_forward` locks the ref,
  checks it and HEAD, runs a safe checkout that will not overwrite ignored
  files, rechecks, then moves the ref. `canonical_tree_sources` reads canonical
  sources from a Git tree for merge validation. `refs.rs` holds the pure
  behind/ahead/diverged primitive `clean_descendant`.
- `src/repository.rs`: `prepare_context_unlocked` holds the only worktree
  creation, always from the primary head. `validate_context_worktree` and the
  deterministic path-chain checks define a valid local context.
  `refresh_repository` is the index-only refresh.
- `src/repository/discovery.rs`: `observe_active_contexts` defines a
  conforming active context (registered deterministic worktree, exact branch,
  item present and valid for the branch's kind and ULID).
- `src/repository/recovery.rs`: a local journal begin is refused with
  `RecoveryRequired` while any poll is in an active phase; a remote reservation
  is refused while a local operation is pending. A poll's operation ID cannot
  also be a local operation ID.
- `src/repository/read/status.rs`: `polling_status` fills every field of
  `PollingStatusDto` except `next_eligible_at`.
- `tests/remote_merge_recovery.rs` and `tests/comment_publication.rs` are the
  closest fixture models: two clones over the in-process SSH server, isolated
  child processes, output-privacy controls, short directory names.
  `.github/workflows/build.yml` is manual-only and lists test targets by name.

## Public Interface

`observe_publication_remote` stays as it is. The full poll is a new operation;
both persist action `poll`, distinguished by a stored poll kind so one
operation ID cannot be replayed as the other.

```rust
pub struct PollPublicationRemoteRequest {
    pub root: PathBuf,
    pub operation_id: OperationId,
    pub invocation: RemotePollInvocation,   // Automatic | Explicit
    pub approval: Option<HostApproval>,
    pub restart: bool,                      // explicit resume of an interrupted poll
}

RepositoryService::poll_publication_remote<P: SessionCredentialProvider>(
    &self, request: PollPublicationRemoteRequest, session: &mut SessionCredentials<P>,
) -> Result<RemotePollResult, RemotePollError>;

RepositoryService::automatic_poll_eligibility(&self, root: &Path)
    -> Result<AutomaticPollEligibility, RepositoryError>;   // no network, no reservation

pub enum RemotePollResult {
    Complete(RemotePollOutcome),
    IndexPending(IndexPending<RemotePollOutcome>),
}
pub enum RemotePollOutcome {
    NotEligible { reason: PollIneligibility, next_eligible_at: Option<OffsetDateTime> },
    Polled(RemotePollReport),
}
pub enum PollIneligibility { Disabled, Paused, RecoverySuspended, NotDue }
pub enum AutomaticPollEligibility { Eligible, NotEligible { reason, next_eligible_at } }

pub struct RemotePollReport {           // accessors only, no Debug of request data
    operation_id, completion: PollCompletion,       // Completed | Interrupted | Cancelled
    completed_at, next_eligible_at,
    targets: Vec<PollTargetReport>,
}
pub struct PollTargetReport {
    target: Option<RemoteRefTarget>,    // None for a malformed ref
    kind: Option<AuthoringKind>, item_id: Option<ItemId>,
    disposition: PollDisposition, before_oid: Option<Oid>, after_oid: Option<Oid>,
}

pub enum RemotePollError {
    Repository(RepositoryError), Transport(SshTransportError),
    Busy, PollYielding, Interrupted, ConflictPending,   // ConflictPending: (D3)
}
```

`PollDisposition` is the fixed enumeration in the Cycle document; each variant
has one fixed guidance string. `Interrupted` and `Cancelled` are returned as
`Polled` with that completion when a batch was committed, so the caller sees
which local changes already happened; before the batch they are the existing
error and cancelled category. `RemoteSnapshot` gains `latest_attempt_at()`,
`next_eligible_at()` and, on each `RemoteContextSnapshot` plus the primary, the
latest stored disposition. `PollingStatusDto.next_eligible_at` is filled from
the same column; no field is added to the versioned read contract.

## Policy, Eligibility And Attempt State

Add to `remote_polling_state`, by the existing probe-and-add migration:
`latest_attempt_at` and `next_eligible_at` (nullable UTC seconds). A registry
without them migrates with both empty, which reads as due.

Eligibility is one pure function of stored policy and a supplied `now`, used
by both `automatic_poll_eligibility` and the poll's own gate:

1. `enabled = 0` → `Disabled`; `paused = 1` → `Paused`;
   `recovery_suspended = 1` → `RecoverySuspended`.
2. `next_eligible_at` empty or not after `now` → `Eligible`.
3. Otherwise `NotDue`. If the stored time is later than
   `now + max(interval, 900 s)`, first rewrite it to that bound in the same
   transaction. A stateless comparison would never become due, so the clamp is
   persisted once and then counts down normally.

The gate runs before endpoint configuration and reservation. An ineligible
automatic attempt writes nothing except a clamp. Ruling: an explicit poll
bypasses all four reasons, including `enabled = 0` and recovery suspension,
because each gates *automatic* polling; it clears none of them. Unknown remote
history stays unknown.

On completion, in the transaction that finishes the reservation:

| Result | Backoff | Next eligible time |
| --- | --- | --- |
| Completed, any invocation | cleared | completion + interval |
| Transport unavailable or protocol rejected, automatic | 60 s, doubling to 900 s | completion + backoff |
| Key unavailable, unlock required, host approval required, configuration required, repository unavailable, automatic | unchanged | completion + interval |
| Any failure, explicit | unchanged | unchanged |
| Busy, yielding, interrupted, cancelled, conflict pending, not eligible | unchanged | unchanged |

`latest_attempt_at` is written whenever the latest outcome is. The backoff
rules are the existing `finish` behaviour; only the two timestamps are new.

## Reservation And Safe Points

The poll keeps the existing poll-target reservation and adds a local stage.
No phase or safe-point name is added, so no `CHECK` list changes:

```text
reserved
 → BeforeTransport            (advertising)
 → poll-set fetch; yield/cancel also read from the progress callback
 → AfterAdvertisement         (advertising)
 → BetweenObservations ×n     (persisting)
 → BeforeBatchCommit → commit batch → AfterBatchCommit
 → BeforeLocalMutation ×n     (local_prepared; once per target, repeatable)
 → BeforeDiscovery            (local_prepared)
 → completed | interrupted | cancelled | failed
```

A stored `poll_kind` (`observe` or `update`, added to
`remote_operation_records` by probe-and-add, empty for non-poll rows and read
as `observe` for existing poll rows) selects the rule set:

- For `update`, `BeforeLocalMutation` may repeat, `BeforeDiscovery` is
  accepted, `Completed` is allowed only at `BeforeDiscovery`, and a record at
  `AfterBatchCommit` is not complete.
- `restart` of an `update` record that is interrupted, or abandoned in an
  active phase, bumps the owner epoch and restarts from `reserved`. That is
  safe after `BeforeLocalMutation` because every local step acts only on state
  re-observed under the lease: an already fast-forwarded branch is `current`,
  an already materialized context is present.
- Replaying an ID with the other kind is `OperationMismatch`.

**(D3)** Before reserving, a pending synchronization conflict in the repository
returns `RemotePollError::ConflictPending` with no transport and no change to
outcome, backoff or next eligible time.

**(D4)** The local journal-begin guard keeps refusing while a remote record is
active, but when that record has poll priority it sets `yield_requested` and
returns `RepositoryBusy` instead of `RecoveryRequired`. A manual-priority
record keeps today's refusal. Nothing else in local authoring changes.

The transfer-progress callback reads the two request flags with a short
read-only query at most every 250 ms and returns `false` to stop the download.
A stopped download has written no tracking ref, so the poll acknowledges at
`AfterAdvertisement` as interrupted or cancelled. The callback never takes the
lease and never writes.

## Poll-Set Fetch

Add `AuthenticatedSshRemote::fetch_poll_set(&mut self, plan: &RemoteRefPlan,
progress: ...) -> Result<Vec<(String, Oid)>, SshTransportError>` beside
`fetch_exact`, sharing its private transfer path:

- List the advertisement on the authenticated connection.
- Build explicit forced mappings for the primary (when advertised) and each
  advertised ref the plan classifies as a recognized context. This is the
  RFC's three mappings restricted to names that parse. Ruling: a malformed
  name under a family is observed but not downloaded; nothing downstream may
  use it, and it would otherwise gain a tracking ref.
- Download with FETCH_HEAD updates, tags and prune all off.
- Under the repository lease, verify every fetched tip is a commit and every
  destination still holds the value captured before the download, then write
  tracking refs with old-value guards. A changed destination fails the fetch
  with no partial claim; refs already written are valid observations.
- Return the advertisement that was fetched against. The batch is built from
  it, with each tracking OID read back after the guarded write.

The scope is opened with the automatic flag and the frozen endpoint and key
expectation together, which needs the existing private combined entry point
exposed to the remote module. **(D1)** No tracking ref is deleted: a ref
advertised before and absent now becomes `remotely_deleted` in the batch and
its tracking ref stays. An absent primary changes no local branch.

## Local Stage

Targets are processed in a fixed order: primary, then existing local contexts
by ref name, then new remote contexts by ref name. Each target takes
`BeforeLocalMutation`, then the short lease, then re-observes: configuration
and endpoint still match the frozen values, the tracking ref equals the batch
OID, and the local state is as classified. A mismatch records `not_reached`
for remaining targets and returns `Interrupted`; nothing is guessed.

### Classification of the primary and existing contexts

1. Locate the worktree with the existing `materialized_target` checks (primary
   checked out at the root; context registered at the deterministic path with
   HEAD on its branch). Failure is `inaccessible`. Ruling: a primary branch
   that is not checked out is not moved, matching synchronization.
2. Merge state, an unclean repository state or index conflicts →
   `conflicted`. Any status entry including untracked files → `dirty`. An
   index tree that differs from HEAD → `inaccessible`.
3. Compare local and tracking with `clean_descendant`: equal → `current`;
   local strictly behind → fast-forward; tracking behind local →
   `local_ahead`; neither → `diverged`.
4. A context whose remote ref is absent keeps the batch state
   (`remotely_deleted` or `history_unknown`) and is not classified further.

A context is compared with its own tracking ref only. Integrating the primary
into a context is a merge and belongs to deliberate synchronization.

### Fast-forward

Move `fast_forward`, `require_clean_target`, `materialized_target` and
`local_oid` behind a small internal seam the poll can call; behaviour and
synchronization call sites are unchanged. The poll rechecks the local OID
immediately before and after. The safe checkout refuses to overwrite an
ignored file that collides with an incoming path; that is reported as `dirty`
with nothing changed.

### Conformity of a new remote context

A recognized remote context is new when there is no local branch
`manyhands/<kind>/<ULID>`, no registered worktree named `<ULID>`, and no entry
at `.manyhands/worktrees/<ULID>`. Its fetched commit's tree is validated
without touching the filesystem:

- Collect canonical sources with the existing tree reader: regular non-
  executable blobs, valid UTF-8, canonical paths only.
- Apply the discovery rule: the ticket at `.manyhands/tickets/<ULID>/ticket.md`,
  or exactly one managed document carrying the ULID under `docs/`, parses and
  matches the branch's kind and ID, validated together with
  `.manyhands/comments/<ULID>/`. Invalid comments do not make the context
  nonconforming; they surface as ordinary problems after indexing, exactly as
  for a local context.
- Reject a tree with any entry under `.manyhands/worktrees/`.

Failure is `nonconforming`. Because the rule is the one discovery applies, a
materialized context is one the next refresh accepts as active. **Owner-approved
amendment, 2026-10-13:** agreement is accepted tree ⇒ active context only.
Materialization validation may be stricter than local discovery: executable
items remain rejected by polling, and existing discovery behavior is unchanged.
A rejected tree need not be inactive in discovery; polling still creates nothing.

### Materialization

Add `remote_poll_materializations`, created all-or-nothing inside the existing
migration and listed in the registry table inventory:

| Field | Purpose |
| --- | --- |
| Repository, kind, item ULID | The context; unique per repository |
| Poll operation ULID | The attempt that owns the intent |
| Target commit OID | The fetched commit the branch is created at |
| Step | `intended`, `branch_created`, `complete` |

Under the lease, for a conforming new context:

1. Validate the deterministic worktree path chain with the existing checks.
2. Write the intent row (`intended`).
3. Create the local branch at the target commit, without force. Record
   `branch_created`.
4. Create the worktree named `<ULID>` at the deterministic path on that
   branch, with the options local authoring uses.
5. Verify with unchanged `validate_context_worktree`, then record `complete`.
   **Owner-approved exception, 2026-10-14:** only this Task 5 step may perform
   that helper's canonical collection under the same single lease acquisition,
   before `complete`. Discovery/index-refresh scans, network calls and prompts
   remain outside the lease. No helper-body change or broader scan exception;
   D1–D4 unchanged.

No upstream configuration is written; no existing context has any, and
tracking is derived from the ref plan. Reconciliation on any later poll, Git
state first:

| Observed | Action |
| --- | --- |
| Branch, registered worktree, path valid | `current` / classify normally; mark intent `complete` |
| No branch, no path, no registration | Start again from step 1 |
| Intent `branch_created`; branch at the recorded OID; no path, no registration | Finish from step 4 **(D2)** |
| Branch or path present with no matching intent | `blocked_by_local_state`; nothing adopted |
| Anything else, including a partly present worktree | `materialization_incomplete`; nothing removed **(D2)** |

Two processes cannot both materialize: the reservation admits one poll and
every step is under the lease with re-observation. Windows path length is a
real limit for a checked-out tree; a failed checkout lands in the last row and
is reported, never retried blindly.

## Dispositions And Visibility

Add `remote_poll_dispositions`: one row per target of the latest poll that
committed a batch (repository, poll operation ULID, ordinal, classification,
remote ref when recognized, kind, item ULID, disposition, before and after
OIDs, observed time). The rows are replaced as a set when a later poll commits
its batch; targets not processed yet are `not_reached`. A replay of the latest
poll's ID returns the stored report; a replay of an older completed ID returns
its stored completion and outcome with an empty target list.

`RemoteContextState` is unchanged. A disposition describes how the last poll
left a target, the state describes the remote observation, and guidance is the
fixed text of the disposition. Rows contain no title, body, path content,
endpoint or secret. Nothing is written to the discovery `problems` table,
which the refresh rebuilds from canonical files.

## Ordered Data Flow

```text
resolve root, configuration, ref plan            (no remote → configuration required)
 → automatic: eligibility gate                   (ineligible → NotEligible, stop)
 → configure endpoints; conflict-pending gate    (D3)
 → reserve (poll | manual priority); replay or restart
 → BeforeTransport → authenticated scope: advertise, download, guarded tracking writes
 → AfterAdvertisement → observations → BeforeBatchCommit
 → recheck configuration → commit batch + initial dispositions → AfterBatchCommit
 → per target: BeforeLocalMutation → lease → re-observe → classify
        → fast-forward | materialize | leave → record disposition → release
 → BeforeDiscovery → mark refresh required → finish (outcome, times, backoff)
 → index-only refresh under a fresh internal operation ID, outside the lease
 → Complete(Polled) | IndexPending(Polled)
```

The refresh uses a new server-generated operation ID because a poll's ID
cannot also be a local operation ID. The durable `refresh_required` flag is
set before the poll completes, so a crash between completion and refresh
leaves an ordinary stale index that any refresh repairs. A poll that stops
after changing something sets the flag and returns index pending without
scanning, so the waiting manual operation is not delayed.

## Failure And Replay Boundaries

| Boundary | Observed state and permitted retry |
| --- | --- |
| Ineligible, busy, yielding, conflict pending | Nothing changed; no record beyond an existing one |
| Transport failure before or during download | No ref written; outcome and backoff recorded; last complete batch kept |
| Download stopped by yield or cancel | No ref written; `interrupted` or `cancelled`; restart re-fetches |
| Crash after some tracking writes, before the batch | Tracking refs ahead of the stored batch; the next poll re-observes and commits a batch |
| Crash after the batch, during the local stage | Completed targets are already in Git; restart or the next poll re-observes each and repeats nothing |
| Crash between ref move and disposition row | Git wins; the target reads `current` next time |
| Crash inside materialization | Reconciliation table above |
| Configuration, endpoint or key changed mid-poll | Existing generation fence interrupts the record; nothing further is changed |
| Completed, refresh failed | `IndexPending`; later refresh repairs discovery with no fetch |
| Local operation pending | Reservation refused as today; the poll does not consume local recovery |

## Alternatives And Rulings

- **Extend `observe_publication_remote` in place:** rejected. Its callers and
  tests rely on a read-only advertisement with no local change. Ruling: a new
  operation with a stored kind; cost is one column and a small rule fork.
- **Call `synchronize_remote` per context:** rejected. It merges, pushes,
  integrates the primary and takes manual priority, all forbidden to a poll.
- **Wildcard refspecs with libgit2 tracking updates and prune:** rejected. It
  bypasses the old-value guards, fetches malformed names, and deletes refs.
- **Materialize through `prepare_context`:** rejected. It creates from the
  primary head, validates against the primary's files, and journals a local
  authoring action. Ruling: a dedicated step reusing its path and worktree
  validators.
- **Trust the caller for eligibility:** rejected. The Wave gate requires the
  one-shot poll itself to obey stored policy; a pure shared function also
  gives Wave 03 the same answer without a network call.
- **Write poll exceptions into `problems`:** rejected. The refresh owns and
  rebuilds that table from canonical state.
- **Add new phases and safe points:** rejected for now. Existing names cover
  the local stage and avoid rebuilding the operation table's `CHECK` lists.
- **A total poll deadline:** out of scope. The accepted limits are 10,000 ms
  per TCP connect and 30,000 ms per blocking SSH call.

These rulings are internal and reversible within the approved contract. A
Task 0 characterization that contradicts one stops for an amendment.

## Verification And Evidence Gaps

Unit tests cover eligibility, timestamps, the clamp, kind-specific reservation
rules, restart, tree conformity, classification and materialization
reconciliation. Real SSH tests over two clones cover fast-forward and
materialization once, the full exceptional-state matrix, manual and local
priority, interruption at every boundary, privacy, and that the server never
receives a push. Each preservation case snapshots worktree files, index,
status, branch and tracking OIDs, and worktree registrations before and after.

Gaps to close with evidence, not assumption: stopping a download from the
progress callback on each native target; what libgit2 leaves after a failed
worktree checkout, including a Windows path-length failure; and the five
native targets, which run only by manual dispatch with separate authorization.
Local Linux results and workflow wiring are not native proof.
