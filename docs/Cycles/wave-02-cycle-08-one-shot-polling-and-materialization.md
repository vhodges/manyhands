---
title: "Wave 02 Cycle 08: One-Shot Polling And Materialization"
date: 2026-10-11
status: approved
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M4NM26P51XW21SPJDF6RPYQ9"
---

# Wave 02 Cycle 08: One-Shot Polling And Materialization

## Purpose And Authority

Add the headless one-shot poll: fetch the poll ref set once, record what the
remote advertises, fast-forward the local primary and existing item contexts
only when they are clean and strictly behind, materialize each newly discovered
conforming shared context exactly once, refresh discovery, and leave every
exceptional context unchanged, visible and actionable. The poll persists the
attempt result and the next eligible automatic time so Wave 03 can schedule and
report it. It never pushes, checkpoints, merges, rebases, stashes, overwrites,
discards or cleans up.

This is Cycle 08 of [Wave 02](../Waves/wave-02-collaboration.md), tracked by
ticket `01K7F6H9J2N4Q6S8V0X2Z4B6DG`. Authorities are the
[Git workflow](../RFC/git-workflow-and-conflict-recovery.md) (Remote Refs,
Polling, Remote Coordination And Cancellation),
[index/recovery](../RFC/repository-index-persistence-and-refresh.md) (Remote
Polling And Reservation), [MVP](../RFC/mvp-rfc.md) (Remote Polling Contract),
[authentication](../RFC/authentication-and-credential-handling.md),
[runtime and polling](../RFC/application-runtime-and-polling.md) and
[test strategy](../RFC/test-and-compatibility-strategy.md) RFCs, plus
`MH-COLLAB-004/006` and `MH-INDEX-002` in the [PRD](../PRD/mvp.md). Read the
[design](../plans/2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-design.md)
and [implementation plan](../plans/2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-implementation.md)
together. The owner approved all three on 2026-10-11, approved the recommended
answer to each of the four decisions below, and authorized committing this
planning checkpoint. Implementation remains separately gated.

## Planning Evidence And Implementation Entry Gate

The existing ticket branch and worktree were reused. Fresh `git fetch origin`
observed `origin/main` at `d1ca88772c5768d55aadb2ca4dcac1c69dab0d17` (the
Cycle 07 merge, pull request 17). The clean ticket head rebased without
conflicts from `5539c60eeb4022c71b1dc13716aeb2d2545a7375` to
`27d5ef54c67a116a590602d86b7086f8602c0c62`; ancestry verification passed. The
main checkout's unrelated untracked `.superpowers/` was left untouched. Details
are in the [execution ledger](../plans/2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-execution.md).

At this base Cycles 01–07 are present. What exists and what does not:

- `observe_publication_remote` is an advertisement-only poll: it reserves with
  poll or manual priority, yields and cancels at safe points, and records a
  complete observation batch. It transfers no objects and writes no ref.
- Deliberate synchronization fetches one exact target with guarded tracking-ref
  writes and fast-forwards a clean checked-out branch. Both are private to the
  synchronization module and typed on a single target.
- Nothing creates a local branch or worktree from a remote branch, validates an
  item from a fetched Git tree, consults the stored pause/interval/backoff
  policy before an attempt, or stores an attempt time or next eligible time.
  `PollingStatusDto.next_eligible_at` is always empty.
- A local authoring action that begins while a poll holds its reservation is
  refused with `RecoveryRequired`. A pending synchronization conflict makes
  every synchronization `Busy` but does not stop a poll.

Implementation requires:

- Explicit implementation authority. Approval of this Cycle, design and plan
  and the answers to the four decisions below were given on 2026-10-11.
- A repeat fetch/rebase preflight at the implementation boundary.
- The Task 0 characterizations in the plan (stored tracking-OID meaning,
  restart of an incomplete synchronization after tracking refs move, what a
  failed worktree creation leaves behind, and download abort from a progress
  callback). A result that contradicts the design stops for an amendment.
- Wave 02 entry-gate authorities remain approved; rerun the required baseline.
  Planning inspection is not a passing Rust or platform gate.

## Scope

- One public headless operation, `poll_publication_remote`, with a typed
  request, per-target report, and typed busy/yield/interrupt/transport errors.
- Automatic-invocation eligibility from stored policy: enabled, explicit pause,
  recovery suspension, failure backoff and next eligible time. An ineligible
  automatic attempt does no transport and takes no reservation. An explicit
  poll bypasses that gating for the one attempt without changing stored policy.
- Persisted attempt completion time and next eligible automatic time; the
  existing 60-to-900-second doubling backoff; a persisted clamp for an
  implausibly distant deadline after clock rollback.
- A poll-set fetch of the primary and every recognized context branch over the
  selected-key transport, with tracking refs written under old-value guards.
- Manual priority: yield and cancellation checked before transport, during
  transfer progress, after fetch, between targets and before each local change.
- Clean-only, strictly-behind fast-forward of the checked-out primary and of
  each existing context worktree.
- Validation of a new remote context from its fetched tree, then creation of
  exactly one local branch and deterministic worktree, with a durable intent so
  an interrupted attempt is finished or reported, never repeated or guessed.
- A fixed per-target disposition with fixed guidance, stored without content
  and exposed through the remote snapshot.
- Index-only refresh after the local stage, with index-pending reported
  separately from the poll result.
- Real authenticated-SSH two-clone tests, fault injection, privacy scans and
  native-test wiring.

## Exclusions And Downstream Obligations

No resident scheduler, timer, sleep, automatic retry or catch-up loop. No push,
checkpoint, merge, rebase, forced local update, stash, conflict resolution,
promotion, closure or cleanup. No deletion of a local branch, worktree or
tracking ref. No republish of a remotely deleted context. No new credential,
host-trust or key policy. No CLI grammar, JSON schema change, desktop view or
prompt. No change to the canonical schema or to the versioned read contract
beyond filling the existing `next_eligible_at` field.

Wave 03 owns: the desktop worker and its launch/interval/fairness behaviour
(D6); `poll once` and policy commands (C5); session unlock and host-attention
suspension, which stay distinct from stored pause; a read DTO for per-target
dispositions; and the rule that a clean worktree does not prove there is no
unsaved editor draft. A fast-forward changes files in a clean worktree, so
desktop observation checks before applying a draft remain mandatory.
Cycles 09 and 10 own removal of local and tracking state after confirmed
integration.

## Required Contract

1. Resolve the registered repository, valid configuration and publication
   remote. A missing remote or invalid configuration returns the existing
   configuration-required outcome with no transport.
2. For an automatic invocation, evaluate stored policy first. Disabled, paused,
   recovery-suspended, or not yet due returns `NotEligible` with the reason and
   the next eligible time. It contacts no server, prompts for nothing, takes no
   reservation and does not change the latest outcome. An explicit invocation
   skips this gate and leaves pause, enabled state and backoff as stored.
3. Reserve with poll priority (automatic) or manual priority (explicit). A
   manual operation that finds an automatic poll requests yield and receives
   `PollYielding`; the poll stops at its next safe point, releases its
   reservation and records `interrupted`. Busy, yield and cancellation are not
   network failures and do not change backoff or the next eligible time.
4. Fetch, in one authenticated scope, the primary branch and each advertised
   branch that parses as `manyhands/<kind>/<ULID>`. Write each tracking ref
   only if it still holds the value read before the download. A branch under a
   context family that does not parse is recorded as malformed and is not
   downloaded. A branch outside the families is not fetched or recorded.
5. Record the advertisement as one complete batch before any local change, so
   a context published earlier and now absent becomes `remotely deleted`. A
   failed, cancelled or partial fetch never implies deletion.
6. For the primary and each existing context, under the short repository lease
   and after re-observing configuration, tracking ref and local state:
   fast-forward only when the branch is checked out in its expected worktree,
   that worktree is clean and non-conflicted, and the local branch is a strict
   ancestor of its tracking ref. Every other relation leaves branch, index and
   worktree byte-for-byte unchanged and records a disposition.
7. For a recognized remote context with no local branch and no local worktree
   path, validate the fetched tree: the item named by the branch must be
   present, valid and of the branch's kind under the same rule discovery uses
   for an active context, and the tree must not carry entries under
   `.manyhands/worktrees/`. Only then create the local branch at the fetched
   commit and the worktree at `.manyhands/worktrees/<ULID>/`. A nonconforming
   tree creates nothing. **Owner-approved amendment, 2026-10-13:** poll
   materialization validation may be stricter than local discovery; regular
   non-executable items are required while discovery behavior is unchanged.
   Agreement requires accepted trees to be active after checkout, not rejected
   trees to be inactive in discovery.
8. Materialization happens once. A second poll, a concurrent poll from another
   process, or a replay finds the context present and reports it current.
   Existing local state that is not exactly the poll's own recorded partial
   result is never adopted, moved or removed.
9. After the local stage, mark discovery stale durably, complete the poll with
   its attempt time and next eligible time, then run the index-only refresh
   outside the lease. A refresh that cannot finish returns the poll result as
   index pending; a later refresh repairs it without another fetch.
10. Success sets the next eligible time to completion plus the configured
    interval and clears backoff. An automatic transport or protocol failure
    doubles backoff from 60 to at most 900 seconds and sets the next eligible
    time to completion plus backoff. Key, unlock and host failures store their
    category and use the interval; they are attention states, not backoff.
    An explicit failure leaves backoff and the next eligible time unchanged.
11. A repeated operation ID returns the stored result. An interrupted poll is
    resumed only with explicit `restart`; it re-observes and re-fetches, and
    every local step is safe to repeat because each acts only on re-observed
    state.
12. No network call, prompt or discovery scan holds the repository lease.
    **Owner-approved exception, 2026-10-14:** only Task 5 materialization step 5
    may run unchanged `validate_context_worktree` canonical collection under
    the same single lease before recording `complete`. Discovery/index-refresh
    scans, network calls and prompts remain outside; D1–D4 unchanged.
    Records hold only IDs, ref names, object IDs, timestamps and fixed
    categories. Cycle 03's per-call transport limits stand; there is no total
    poll deadline.

### Dispositions

Each target receives exactly one disposition per completed poll. Guidance text
is fixed per disposition.

| Disposition | Applies to | Local effect |
| --- | --- | --- |
| `current` | primary, context | none; local equals tracking |
| `fast_forwarded` | primary, context | branch, index and worktree advanced |
| `local_ahead` | primary, context | none; unpublished local commits |
| `diverged` | primary, context | none; deliberate synchronization required |
| `dirty` | primary, context | none; uncommitted or untracked changes kept |
| `conflicted` | primary, context | none; conflict or in-progress merge kept |
| `inaccessible` | primary, context | none; worktree missing, unreadable, redirected, not registered, or on another branch |
| `materialized` | new context | one branch and one worktree created |
| `nonconforming` | new context | none; fetched tree fails validation |
| `blocked_by_local_state` | new context | none; a branch or path already exists |
| `materialization_incomplete` | new context | none beyond the recorded partial step |
| `malformed` | remote ref | none; not downloaded |
| `remotely_deleted` | context | none; local context preserved |
| `history_unknown` | context | none; local context preserved |
| `not_reached` | any | none; the poll stopped first |

A remote rename appears as the old name `remotely_deleted` with its local
context preserved, and the new name `nonconforming` when its item ID no longer
matches the branch.

## Owner Decisions

**Decided 2026-10-11:** the owner approved the recommended answer to all four.
Each is written into the design and plan and marked **(D1)–(D4)** there. The
original question and the cost accepted are kept below.

1. **Stale tracking refs.** The Git RFC speaks of a poll pruning stale tracking
   refs. Recommended: this Cycle never deletes a tracking ref. For an
   unmaterialized context it can be the only local reference to a
   collaborator's commits. Cost: stale refs accumulate until Cycles 09/10 or a
   later explicit cleanup.
2. **A worktree left half-created by an interruption.** Recommended: finish
   only the case where the branch exists at the recorded commit and no worktree
   path or registration exists. Any partly present worktree is reported as
   `materialization_incomplete` and left for explicit repair. Cost: a rare
   manual repair. The alternative lets a poll remove its own partial directory,
   which is a deletion a poll is otherwise forbidden to make.
3. **Polling while a synchronization conflict is pending.** Recommended: refuse
   before transport with a typed conflict-pending result that is not a network
   failure, matching how every synchronization is refused. Cost: no background
   discovery until the conflict is resolved, and there is no abandon path yet.
   The alternative polls and skips the conflicted context, and needs proof that
   moving tracking refs cannot invalidate the pending resolution.
4. **A local save or comment that starts during an automatic poll.** Today it
   is refused with `RecoveryRequired`, and a full poll holds its reservation
   far longer than an advertisement did. Recommended: the local action requests
   yield and returns retryable `RepositoryBusy`; the poll stops at its next
   safe point. Cost: a narrow change to the Wave 01 journal-begin guard and its
   tests. The alternative keeps the refusal and leaves callers unable to tell a
   transient poll from real recovery.

## Acceptance And Exit Evidence

| Acceptance | Proof owner |
| --- | --- |
| Automatic poll obeys disabled, paused, recovery-suspended and not-due state with zero transport, prompt and reservation; explicit poll bypasses them without changing policy | Task 1 unit tests; Task 7 transport-counter cases |
| Attempt time, next eligible time, backoff doubling and cap, attention categories and the rollback clamp are persisted as specified | Tasks 1–2 state tests; Task 7 offline and rejected-key cases |
| Poll-set fetch transfers primary and recognized contexts only, writes guarded tracking refs, never pushes, and records one complete batch | Task 3 transport tests; Task 7 exact-ref and receive-counter assertions |
| Manual operation and local action take priority; the poll stops only at a safe point and its retry repeats no completed local change | Task 2 reservation tests; Task 7 held-advertisement and paced-transfer cases |
| Clean strictly-behind primary and context are fast-forwarded exactly once | Task 4 unit tests; Task 7 two-clone cases |
| Dirty, untracked, staged, conflicted, ahead, diverged, wrong-branch and unreadable state is byte-for-byte unchanged with the right disposition | Task 4 preservation snapshots; Task 7 exceptional-state matrix |
| A conforming new document and ticket context is materialized once and indexed with its comments; a second poll and a concurrent poll create nothing | Tasks 5–6; Task 7 materialization and two-service cases |
| Nonconforming, malformed, out-of-family, renamed and remotely deleted remote state creates and removes nothing and stays visible | Tasks 4–6; Task 7 exceptional-state matrix |
| Interruption at each boundary (fetch, tracking write, batch, fast-forward, branch creation, worktree creation, completion, refresh) reconciles from Git and repeats only unfinished work | Tasks 2, 5 and 6 fault tests; Task 7 reopen cases |
| Index failure is reported separately and repaired without a second fetch | Task 6 injected refresh failure; Task 7 transport counters |
| Recovery rows, database sidecars, diagnostics and fixture output hold no body, title, endpoint or secret | Tasks 1 and 7 privacy scans |
| Required Devenv checks pass; native evidence is honestly recorded | Task 8; native execution pending authorization |

## Self-Review And Lifecycle

The decision audit in the [execution ledger](../plans/2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-execution.md)
covers scope and interfaces, trust and prompts, persistence and recovery, time
and failure, backend and platform, fixtures, verification and lifecycle. It
records the four owner decisions (above) and the internal rulings.

Record planning, each task, decisions, blockers, baseline and final
verification, review findings and review-ready status on the ticket. The ticket
stays open through implementation and verification. Closure follows
code-review or pull-request approval, normally in the authorized final
pre-merge push. Commit, push, pull request, native dispatch, merge, closure and
worktree cleanup each need their own authorization. The rebase rewrote the
published ticket checkpoint, so a later push must first reconcile the remote
ticket branch; no force push is implied. Nothing here claims implementation or
passing tests.
