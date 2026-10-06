---
title: "Wave 02 Cycle 05 Clean Deliberate Synchronization Design"
date: 2026-10-06
status: approved
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M5100000C8D2E3F4G5H6J7K8"
---

# Clean Deliberate Synchronization Design

## Intent And Approval Boundary

Cycle 05 makes a remote lifecycle action useful without taking ownership of
merge or conflict policy. It converts one caller-selected local primary branch
or deterministic shared context from a clean, inspectable state to the same
clean state at the configured publication endpoint, or returns the precise
boundary that needs Cycle 06 or later recovery. A successful operation has a
verified remote OID and a refreshed local discovery snapshot; it is not merely
a successful `git2` return.

The Wave and RFCs already settle one shared context branch, ordinary
non-force pushes, no automatic rebase, clean-only primary handling, durable
manual-over-poll coordination, selected-key SSH, and ref/worktree authority.
This design decides the internal operation shape, replay checkpoints, and the
way a separately configured Push endpoint is observed. It does not change the
approved branch protocol. Read it with the [Cycle](../Cycles/wave-02-cycle-05-clean-deliberate-synchronization.md)
and [implementation plan](2026-10-06-wave-02-cycle-05-clean-deliberate-synchronization-implementation.md).
The user approved all three artifacts on 2026-10-06; implementation still
requires separate explicit authorization.

## Review Record

| Source | Concern | Classification | Resolution | Evidence or follow-up |
| --- | --- | --- | --- | --- |
| Cycle 04 state/reservations | A new synchronizer could bypass the durable envelope or mistake its reservation for the short Git lease. | Settled | Extend the single remote-operation record and acquire the existing repository lease only around local Git transitions. | Cross-service poll/manual, lease timing, and replay tests. |
| Git workflow RFC, item order | A context can be clean with its remote and still require merging the primary. | Ruling | Derive a virtual final context target using both ancestry checks before one local update. | No partial update when the second relation diverges. |
| Git workflow RFC, push acknowledgment | `push()` can be ambiguous after remote acceptance. | Settled | Durable `push_started`, then fresh Push-direction advertisement; only an exact OID is published. | Disconnect-after-receive and database-failure replay tests. |
| Cycle 03 endpoint contract | `pushurl` is direction-specific and may not equal fetch URL. | Ruling | Fetch tracking refs from Fetch direction; pre/post-push advertisements use Push direction and record OIDs, never URLs. | Different-destination fixture and fixed-output privacy assertions. |
| Remote deletion/cache loss | An absent context cannot always be first publication. | Settled | Reuse `never_published`/`observed_published`/`history_unknown`; ordinary sync never supplies republish confirmation. | Deleted/history-unknown tests. |
| Test strategy | A fake transport would miss libgit2 ref-update and receive-pack behavior. | Settled | Cover state transitions in unit tests and prove wire behavior with the existing disposable authenticated SSH fixture and two clones. | Native CI is planned evidence, not completed evidence. |

The two rulings are internal and reversible without changing user-facing
policy. No user decision remains open.

## Public Contract

Add a small typed domain entry point under `repository::remote`; neither
front-end owns Git calls or passes a path to an item worktree.

```rust
pub enum SynchronizationTarget {
    Primary,
    Context { kind: AuthoringKind, item_id: canonical::ItemId },
}

pub struct SynchronizeRemoteRequest {
    pub root: PathBuf,
    pub operation_id: OperationId,
    pub target: SynchronizationTarget,
    pub approval: Option<HostApproval>,
    /// Explicitly resume the same recorded action after it was interrupted or
    /// its publication outcome became ambiguous; ordinary duplicate calls do
    /// not steal ownership.
    pub restart: bool,
}

pub enum PublishPendingReason {
    NoPublicationRemote,
}

pub enum SynchronizationOutcome {
    Published { target: SynchronizationTarget, oid: git2::Oid },
    AlreadyCurrent { target: SynchronizationTarget, oid: git2::Oid },
    PublishPending {
        target: SynchronizationTarget,
        local_oid: git2::Oid,
        reason: PublishPendingReason::NoPublicationRemote,
    },
}

pub enum SynchronizationResult {
    Complete(SynchronizationOutcome),
    IndexPending(IndexPending<SynchronizationOutcome>),
}

pub enum SynchronizationError {
    Repository(RepositoryError),
    Transport(SshTransportError),
    Busy,
    PollYielding,
    Interrupted,
    TargetNotMaterialized,
    WorktreeNotClean { target: SynchronizationTarget },
    WorktreeConflicted { target: SynchronizationTarget },
    PrimaryMissing,
    RemoteContextDeleted,
    HistoryUnknown,
    MergeRequired { target: SynchronizationTarget },
    PushRejected,
    ExternalChange,
    RecoveryRequired,
}
```

The exact visibility and wrappers may follow Cycle 04's accepted export style,
but these fields and meanings are the contract. The request contains no
credential, URL, raw ref, local branch, worktree path, OID, force flag, or
confirmation that could widen its authority. `HostApproval` retains Cycle 03's
exact-fingerprint semantics. `PublishPending` means no publication remote was
configured; key/trust/transport failures are not silently relabelled as local
success.

The public operation returns `Result<SynchronizationResult,
SynchronizationError>`. If discovery fails after an authoritative outcome, it
returns `SynchronizationResult::IndexPending` around the exact authoritative
value. A retry with the same ID can perform that index-only handoff and must not
re-contact the remote.

## Ref And Endpoint Policy

`RemoteRefPlan` grows pure derivations for this Cycle only:

```rust
fn primary_fetch_refspec(&self) -> String;
fn context_fetch_refspec(&self, kind: AuthoringKind, id: &ItemId) -> [String; 2];
fn primary_push_refspec(&self) -> String;
fn context_push_refspec(&self, kind: AuthoringKind, id: &ItemId) -> String;
```

Fetch results are always `+<remote>:<tracking>` and push results always
`<local>:<remote>`; the constructor owns all strings. A caller cannot build a
wildcard, force, delete, or cross-target mapping. Deliberate synchronization
does not fetch or prune either wildcard context family, and the adapter keeps
`update_fetchhead(false)`.

The Fetch endpoint supplies `T(P)` and `T(C)`. First list the Fetch endpoint,
fetch only refs present in that list, list it again, and read the relevant
tracking refs. If `C` was absent from the final list, an older local `T(C)` is
recorded only as prior tracking metadata and is never used for this action's
graph decision or deleted. If primary/context presence or OID changed between the two
observations, or a fetched tracking ref fails to equal the final advertised
OID, retain the tracking metadata but return `ExternalChange` before a local
branch mutation. Persist the complete final advertisement through Cycle 04's
state path; this preserves remote-deletion history without treating a failed
fetch as deletion.

`pushurl`, when configured, is not assumed to name the Fetch endpoint. Before
the push, list the Push-direction endpoint and require its primary/context
target to be absent or an ancestor of the exact candidate OID. Then call the
ordinary exact push through the Cycle 03 scoped adapter, list that Push endpoint
again, and require equality with the candidate. The Push endpoint's advertised
OID is operation evidence, not a fetch tracking ref and not persisted as an
endpoint string. A changed, inaccessible, rejected, or non-equal result is a
recovery boundary.

## State Machine And Ordering

The existing Cycle 04 `remote_operation_records` remains the only durable
remote-operation envelope. Extend it with action-safe checkpoints and optional
non-secret expected/local/tracking/push OIDs rather than introducing a second
journal. Persisted enum names must be forward-compatible with already recorded
Cycle 04 rows.

```text
Reserved
  -> FetchPrepared
  -> FetchObserved                 (complete final Fetch advertisement persisted)
  -> LocalFastForwarded?           (one lease-protected branch/worktree update)
  -> PushPrepared?                 (candidate OID durable before network)
  -> PushVerified?                 (Push endpoint advertises candidate OID)
  -> DiscoveryPending?
  -> Completed(authoritative outcome)
```

`?` means the step is skipped when its effect is unnecessary. Terminal
recovery/error outcomes retain enough OIDs and fixed categories to distinguish
a completed effect from unknown state. `PushPrepared` followed by interruption,
client disconnect, or database failure is deliberately *not* a license to push
again. Resume first fetches and observes actual refs. If the Push endpoint
already has the candidate, it advances to `PushVerified`; if it has a divergent
target it returns recovery-required; if it remains a proven ancestor, an
explicit restart may complete the recorded ordinary push.

Every network call and prompt occurs after the short lease is released. Every
local branch update obtains it, re-opens the repository/worktree, rechecks
expected symbolic branch, status, conflicts, and OIDs, then uses expected-old
ref update plus safe checkout. The operation reports recovery rather than
forcing a checkout or rolling a ref back. The reservation safe points are
before/after Fetch observation, before/after local fast-forward, before push,
after post-push observation, and before discovery. A poll yields only at its
existing safe points; a manual synchronization never steals it mid-call.

## Clean Graph Evaluation

All comparisons use actual commit ancestry, never timestamps or a cached
operation row. Let `L` be the local target, `R` the final Fetch tracking target,
and `P` the final primary tracking target.

For the primary: equal is current; `L` ancestor `R` produces final `R`; `R`
ancestor `L` is local-ahead; all other relations are `MergeRequired`.

For a context, calculate without mutation:

1. If `R` exists, choose `R` when `L` is its ancestor, choose `L` when `R` is
   its ancestor, and stop at `MergeRequired` otherwise.
2. Require `P`. If the chosen candidate is an ancestor of `P`, choose `P`; if
   `P` is its ancestor, retain it; otherwise stop at `MergeRequired`.
3. If `R` does not exist, read Cycle 04 publication evidence. Only
   `never_published` permits a first normal push; `observed_published` returns
   `RemoteContextDeleted`, and `history_unknown` returns `HistoryUnknown`.

When a final target differs, one fast-forward updates the local branch and its
own clean worktree. It is impossible for a context to be partly advanced to
`R` and then rejected because `P` diverges. Local-ahead/equal paths need no
checkout and may move to push evaluation directly.

## Discovery, Failure, And Privacy Rules

`Published` and `AlreadyCurrent` each trigger one existing non-mutating
`refresh_repository` only after the Git state is durably classified. If it
fails, record an index-pending step and preserve that outcome for exact-ID
replay. Local-only `PublishPending` likewise uses the normal local discovery
handoff, but does not create a remote record. It passes the same operation ID
to `RefreshRepositoryRequest`, whose existing index-owner/retry protocol is the
durable index-only handoff for that no-remote path. A refresh never becomes
authority to fetch, push, checkpoint, merge, or clean up.

The operation never stores or formats raw fetch/push URLs, server messages,
response bodies, credentials, passphrases, keys, canonical Markdown, or a
worktree path in remote records/errors. It maps backend errors to the existing
redacted transport categories and names only validated repository, target, ref,
OID, operation ID, phase, and fixed reason. The output and all SQLite WAL,
journal, backup, and fixture failure scans receive hostile sentinels for every
forbidden value.

| Condition | Required result and preservation |
| --- | --- |
| No publication remote | After local identity/cleanliness preflight, local `PublishPending`; no remote reservation, transport, or branch mutation. |
| Dirty/conflicted/mismatched target worktree | Typed preflight error; no fetch, lease-side mutation, staging, stash, reset, or discard. |
| Primary absent or fetch changes during observation | Typed recovery/error before a local branch update; tracking metadata and prior observation remain inspectable. |
| Context absent after a complete fetch | First publish only with `never_published`; deleted/unknown history is preserved and delegated. |
| Any divergent primary/context relation | `MergeRequired`; no local branch or worktree change. |
| Non-fast-forward/server push rejection | `PushRejected`; preserve local branch and retry only after fresh reconciliation. |
| Push success but no post-push proof | Recovery/ambiguous state; no blind retry. |
| Checkout or local persistence failure after ref update | Record recovery state; retain actual branch/worktree, never reset it automatically. |
| Discovery failure after stable Git result | Index pending; exact replay refreshes only. |

## Alternatives Rejected

| Alternative | Reason |
| --- | --- |
| Treat fetch and push URLs as interchangeable | Breaks configured `pushurl` semantics and could classify/push against the wrong state. |
| Let a normal push return success without post-push observation | Cannot distinguish accepted-but-disconnected or persistence-failed effects from a retry-safe failure. |
| Fast-forward context before evaluating primary | Produces an avoidable partial update when the next relation needs a merge. |
| Reuse polling's wildcard fetch or materialize remote-only contexts | Expands Cycle 05 into Cycle 08 and makes its user-initiated scope non-deterministic. |
| Use a Git lease for fetch, push, prompt, or index scan | Violates the short atomic-transition contract and blocks unrelated local work. |
| Retry any push error with the same operation ID | Risks repeated effects after an ambiguous server acceptance. |

## Test And Platform Evidence

Pure graph/ref tests cover every relation, exact mappings, first-publication
evidence, and replay-step selection. Repository tests inject failures at every
durable boundary, prove lease release across remote work, preserve dirty and
conflicted state, and scan durable state/output for forbidden sentinels.

`tests/remote_synchronization.rs` uses two disposable real repositories served
by the authenticated SSH fixture. It proves primary and context fast-forward,
already-current and local-ahead publish paths, first publication, rejected and
ambiguous pushes, remote deletion/history-unknown, divergent merge handoff,
different Fetch/Push endpoints, and index-only replay. The fixture adds only
owned ref/OID controls; it invokes no shell Git or user configuration.

The implementation updates the existing headless native matrix to run the new
target. Editing the workflow is feasibility preparation only. Actual Linux,
macOS, and Windows runs on all five existing targets, plus the required local
Devenv gates, are final implementation evidence.
