---
title: "Wave 02 Cycle 04 Remote Observation And Recovery Design"
date: 2026-10-05
status: approved
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M50000000E923E60C6918098"
---

# Remote Observation And Recovery Design

## Intent And Approval Boundary

Cycle 04 turns the approved remote protocol into durable, inspectable state
before a later Cycle may update a local branch. The sole live Git interaction
is an authenticated `Remote::list` advertisement through Cycle 03's scoped
transport adapter. It does not call `download` or `push`; therefore it cannot
write `FETCH_HEAD`, create/update a tracking ref, or change a local branch.

The product contract is already settled by the Wave and RFCs: the exact
refspecs, manual-over-poll priority, safe cancellation boundaries, retention of
remotely deleted contexts, five-minute default polling, one-to-sixty-minute
interval range, and one-to-fifteen-minute automatic backoff are not new
choices. This design specifies data ownership and interfaces needed to enforce
those decisions. See the [Cycle](../Cycles/wave-02-cycle-04-remote-observation-and-recovery.md)
and [implementation plan](2026-10-05-wave-02-cycle-04-remote-observation-and-recovery-implementation.md).

## Review Record

| Source | Concern | Classification | Resolution | Evidence or follow-up |
| --- | --- | --- | --- | --- |
| Git workflow RFC, Remote Refs | A wildcard fetch must not be mistaken for authority to modify local branches. | Settled | Export the three exact future refspecs with remote-tracking destinations, but do not call `download` in this Cycle. | Pure ref-plan tests; no-ref-mutation integration assertion. |
| Index RFC, Remote Polling And Reservation | A failed or incomplete network result must not masquerade as remote deletion. | Settled | Only an atomically persisted, complete advertisement batch supersedes the prior successful batch. | Failure/cancellation and deletion-transition tests. |
| Existing `operation_records` | Local-authoring replay metadata and long-lived remote reservation have different ownership and transition rules. | Ruling | Add dedicated remote-operation/observation tables linked to the repository rather than overloading the local authoring journal. | Migration and cross-process arbitration tests. Cost if wrong: one local schema migration, with no canonical or Git data conversion. |
| Existing transport `advertisement()` | A ref name alone cannot establish canonical-tree validity or permit worktree creation. | Ruling | Persist branch-name recognition state only; defer tree validation and materialization to Cycle 08. | Malformed/unmaterialized tests with no worktree changes. Cost if wrong: later Cycle may add a non-mutating tree-validation field. |
| Git workflow RFC, cancellation | A poll cannot safely be preempted in the middle of a library call or SQLite transaction. | Settled | Record yield/cancel requests immediately; acknowledge at named pre/post-transport and per-ref safe points only. | Deterministic hooks prove acknowledgement and release order. |
| Remote-state cache loss | An absent local context cannot be safely classified after published-history evidence is destroyed. | **User decision, approved 2026-10-05** | Fail closed: publish a recovery marker before replacement, suspend automatic polling until explicit resume, and require explicit later recovery/republish confirmation for absent local contexts. | Corrupt-registry, marker-publication, restart, and no-automatic-publication tests. |
| Protected-key polling UX | Repeated automatic passphrase prompts are disruptive, but a session login should unlock later polling for the process lifetime. | **User decision, approved 2026-10-05** | Desktop startup uses its shared provider for one deduplicated ambiguity-aware unlock; successful material is session-only and reused. Cancellation/failure sets session-only unlock-required state, while host approval remains a separate explicit action. | Shared-session provider-count, cancellation/no-repeat, invalidation, and host-approval tests; Wave 03 proves prompt UI/scheduling. |

The user decision above is resolved. The rulings are internal, preserve the
approved external contract, and are recorded in the ticket checkpoint.

## Current Foundation

`RepositoryService` owns the application-local SQLite registry. Its existing
`operation_records` model records local authoring/recovery and its
common-Git-directory lease protects short local atomic changes. The new
`transport::AuthenticatedSshRemote::advertisement()` already authenticates with
the selected key and explicit host trust and returns `(ref name, OID)` pairs
without transfer. Its existing `download` primitive remains crate-private for
later Cycles.

The current discovery schema stores local worktrees, branches, and problems;
it has no durable remote ref, polling policy, or remote reservation model.
Remote state is application-local and normally rebuildable from a fresh
authenticated observation plus current Git refs. It is never canonical Markdown
and never a lock. Published-history evidence is the exception: before
corrupt-registry replacement, a non-secret remote-history-recovery marker must
be published outside SQLite. If that publication fails, replacement fails. The
marker makes an absent local context `history unknown`, rather than inferring
first publication or remote deletion, and overlays automatic polling with a
recovery suspension until explicit resume. Existing local recovery records
continue to block incompatible work until reconciled; new remote records get
their own explicit recovery view.

## Ref Plan And Classification

`RemoteRefPlan::from_configuration(remote, primary)` validates the configured
Git names and derives these ordered fetch mappings:

```text
+refs/heads/<primary>:refs/remotes/<remote>/<primary>
+refs/heads/manyhands/document/*:refs/remotes/<remote>/manyhands/document/*
+refs/heads/manyhands/ticket/*:refs/remotes/<remote>/manyhands/ticket/*
```

It separately derives a specific context's authoritative remote ref and local
tracking ref. Construction rejects invalid/ref-injecting components before
Git/network access. The leading `+` means a later fetch can accurately reflect
a remote rewind in its tracking namespace only; it never enables a force push
or a local branch update.

`classify_advertised_ref` considers only `refs/heads/`. The configured primary
is a primary observation. A ref exactly matching
`refs/heads/manyhands/document/<canonical ULID>` or
`refs/heads/manyhands/ticket/<canonical ULID>` is recognized. A ref in either
family whose remaining path is not exactly one canonical ULID is recorded as
`malformed`; unrelated refs are excluded. Recognition does not read a tree,
create a tracking branch, or assert an item exists.

## Durable Model

The migration adds application-local tables equivalent to the following
logical records; exact SQL constraints and indexes belong to implementation.

| Record | Non-secret fields | Invariant |
| --- | --- | --- |
| `remote_polling_state` | repository ID, enabled/paused, interval seconds, backoff seconds, latest redacted status/category, status time, next eligible automatic time, recovery-suspended flag | one row per repository; default enabled/unpaused/300 seconds; interval 60–3600 seconds; backoff 60–900 seconds. Recovery suspension is distinct from and never overwrites explicit pause. |
| `remote_observation_batches` | repository ID, remote name, operation ID, observed time, completion/status category | a batch becomes current only after the complete authenticated list is accepted and its refs persist in one SQLite transaction. |
| `remote_ref_observations` | batch, remote ref, derived tracking ref, advertised OID, observed existing tracking OID, recognized kind/item ID when valid, classification | no URL, server text, credentials, canonical content, or worktree data; a ref/OID is non-secret Git metadata. |
| `remote_context_states` | repository ID, remote/ref identity, last advertised OID, last known tracking OID, publication evidence (`never_published`, `observed_published`, or `history_unknown`), state, last-seen batch/time | preserves last OID when an `observed_published` ref transitions to `remotely_deleted`; does not delete a local context. |
| `remote_operation_records` | repository ID, operation ULID, action, priority, phase, remote/ref targets, redacted result category, yield/cancel flags, safe-point/completed observations and timestamps | at most one active reservation per repository; it is recovery evidence, never the Git lock or source of truth. |

Completed batches and operation audits are retained as recovery evidence, but
routine snapshots and transitions read only the indexed current batch, current
context state, and active operations. After every service initialization and
committed migration, a separate deferred read audit validates all retained
remote history and foreign keys before the service becomes ready. Thus ordinary
five-minute polling does not grow its immediate write transaction with history,
while historical corruption remains recovery-required rather than ignored.

The polling-policy row is repository-scoped and survives publication remote or
selected-key removal/replacement, preserving an explicit pause and interval
when configuration returns. Such configuration changes invalidate active
remote reservations and remote-specific observations; current configuration
still determines whether automatic polling is eligible. The migration preserves
existing tables and rows. Every database write uses the existing short
cache-recovery guard. A remote caller holds neither that guard nor the
common-Git-directory lease while waiting for credentials, connecting, listing
refs, or scanning snapshots.

The snapshot API joins observation/context state with the existing local
`contexts` rows. A recognized advertised context with no matching local
context/worktree is `unmaterialized`; a valid local context with no ref in a
later complete batch is `remotely_deleted` only when its evidence is
`observed_published`; a locally created context with `never_published` remains
eligible for later first publication; and an absent context with
`history_unknown` is a recovery problem. A bad family-shaped ref is
`malformed`. Existing local context/branch/worktree state is descriptive only
in this Cycle and is never removed or repaired.

`replace_corrupt_registry` publishes `remote-history-recovery-required` before
renaming/replacing the database, analogous to the existing host-trust recovery
marker. Marker publication is fail-closed. Opening a fresh registry sees the
marker, records the non-secret recovery suspension, and retains it across
rebuilds. A later explicit resume clears only the automatic-poll suspension; it
does not turn an absent `history_unknown` context into first publication. A
later lifecycle confirmation/observed remote ref supplies the per-context
resolution required before the marker can be retired.

## Reservation, Cancellation, And Replay

`RemoteOperationRequest` contains the canonical root, caller-generated
`OperationId`, action (`poll`, `synchronize_context`, `synchronize_primary`,
`promote`, or `close`), manual-or-poll priority, and only validated non-secret
target identities. The public result is a typed, redacted reservation/outcome;
it never accepts a URL, refspec, OID, key path, or passphrase from a caller.

The durable operation envelope retains the validated remote name, primary and
context remote refs, their tracking refs, and any action-relevant item, local
branch, or worktree identities. Each observation records the relevant local,
tracking, and advertised OIDs; fields irrelevant to an action remain absent.
`completed_step` records the last durable transition so restart inspection
never treats a partly recorded list as a completed advertisement.

Beginning a request first reconciles active remote records and existing local
recovery. An active manual record blocks a competing action. An active poll
encountered by a manual request atomically marks `yield_requested` and returns
`PollYielding`; it does not claim the row. A duplicate operation ID must match
its recorded action/targets or returns `OperationMismatch`. A poll tests yield
or cancellation:

1. before it begins transport;
2. after the complete advertisement returns;
3. between recording/classifying individual observations; and
4. before every future local Git/cache mutation.

Acknowledgement records `interrupted` or `cancelled` and releases the active
reservation only after the current SQLite transition finishes. Cycle 04 has no
future local Git mutation or transfer-progress callback; it exposes the same
named durable safe-point mechanism for Cycle 08 to invoke from a future fetch
progress callback. If a process stops before the successful batch transaction,
reconciliation retains the previous batch and retries a read-only
advertisement. If it stops after commit, retry reads the durable batch and does
not infer a second effect.

## Observation Flow And Public Interface

The proposal adds `RepositoryService::observe_publication_remote` with a typed
request carrying root, operation ID, optional host approval, and a
caller-supplied session credential provider. It uses Fetch-direction transport
only to select the direction-specific endpoint; it requests the existing scoped
authenticated remote's advertisement and does not transfer refs. It returns a
`RemoteObservationOutcome` containing a redacted status and a `RemoteSnapshot`
of the persisted batch/state. A companion polling-configuration request reads
or validates persistent pause/enable/interval policy; it starts no scheduler.

The operation records a fixed error category for configuration, key/trust,
transport, protocol, cancellation, and repository/SQLite failures. It never
stores or formats an effective URL, backend/server message, response body,
passphrase, key material, or canonical draft. A newly approved host pin may be
durably recorded by Cycle 03 before advertisement returns; that pre-existing
trust contract is intentional and is reported only through its typed outcome.

`SessionCredentials` belongs to the desktop process, not to an individual poll.
For an eligible selected protected key, desktop startup begins the first
session-aware remote observation/connection through that shared provider. The
Cycle 03 driver first attempts no secret and invokes its ambiguity-aware
provider at most once when needed; the desktop deduplicates that one masked
prompt. Success caches the credential in the process session only, so later
manual and automatic observations reuse it without prompting. Selection/source
change, explicit clear, or process exit invalidates the cache. Cancellation or
provider failure creates a session-only `unlock required` state: later automatic
attempts do not invoke the provider again until an explicit unlock/retry, and
this state never overwrites durable `paused`. Unknown/replaced host trust is
also attention-required, but it must request a separate exact approval rather
than being treated as an unlock or transient-backoff failure. Wave 03 owns the
desktop prompt and status UI; CLI invocations remain independent short sessions.

On an authenticated list failure, the operation status/backoff changes as
appropriate but prior observations stay current. Automatic backoff applies only
to a later scheduler-owned automatic poll; an explicit observation/one-shot
poll is never delayed by it. There is no resident worker, retry loop, or
cross-process scheduler election in this Cycle.

## Failure Handling And Boundaries

| Condition | Result and preservation rule |
| --- | --- |
| Missing/changed configuration, key, or host trust | Preserve prior batch and local Git state; store only a redacted category and return the existing typed transport error. |
| Remote unreachable or malformed advertisement | Preserve prior batch; no deletion, tracking-ref write, branch/worktree change, or canonical refresh. |
| Cancellation/yield before or after list | Record acknowledgement at the next safe point. A successful list that has not crossed the durable batch transition is discarded and later re-observed. |
| SQLite failure while persisting a complete list | Return recoverable failure; prior committed batch remains authoritative. Retry repeats a read-only list. |
| Remote ref disappears after a successful later complete list | Mark only the remote context state deleted, retain last observed OID and all local state, and offer fixed recovery guidance. |
| Remote-state registry is corrupt/lost | Publish the recovery marker before replacement or fail replacement. Suspend automatic polling until explicit resume; label absent local contexts `history_unknown` and require later explicit resolution. |
| Malformed family ref | Persist an observation problem without tree parsing/materialization; do not conflate it with remote deletion. |
| Restart with an active record | Reconcile phase against durable batches and actual Git/local observations; resume only the unfinished read-only observation or require recovery for mismatched targets. |

## Alternatives Rejected

| Alternative | Reason |
| --- | --- |
| Fetch tracking refs during Cycle 04 | Violates the deliberately staged boundary: Cycle 05/08 own transfer ordering and the local-ref re-observation that follows it. |
| Infer remote deletion from fetch/list failure or prune | A network error/partial observation cannot distinguish absence from failure and would hide recovery state. |
| Store remote URL/server diagnostics for replay | Unnecessary for ref reconciliation and conflicts with the no-secret/redacted-record contract. |
| Use the existing common-Git-directory lease as the remote reservation | It is intentionally short-lived; holding it around network/prompt work would block local operations and violates the RFC. |
| Treat branch recognition as canonical validation | Branch names cannot establish safe tree ownership; Cycle 08 must validate before materialization. |

## Test And Platform Evidence

Pure tests cover ref plan construction, classification, interval/backoff bounds,
and record transitions. SQLite integration tests cover legacy migration,
atomic batches, restart/retry, privacy scanning, snapshot states, and two
independent services arbitrating poll/manual requests. The existing disposable
authenticated SSH fixture supplies actual advertisements, changed/deleted refs,
host/key rejection, and a proof that observation leaves `FETCH_HEAD`, refs,
worktree, and canonical bytes unchanged.

The test fixture is already native-platform evidence for authenticated
advertisement, but Cycle 04 must add its new remote-observation test target to
the five-target CI matrix and obtain actual runs after authorized publication.
Until then, workflow configuration is only a feasibility setup, not acceptance
evidence. All regular Rust verification remains through Devenv locally.
