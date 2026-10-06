---
title: "Wave 02 Cycle 05: Clean Deliberate Synchronization"
date: 2026-10-06
status: approved
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M5100000B7F1C238D54EBC71"
---

# Wave 02 Cycle 05: Clean Deliberate Synchronization

## Purpose And Authority

Provide one explicit, headless operation that synchronizes either the configured
primary branch or one existing shared item context when every required history
relationship is clean. The operation fetches only its authoritative refs,
fast-forwards only a clean local branch, performs an ordinary non-force push
when needed, observes that push, and hands stable state to discovery. It never
creates a merge commit or chooses conflict content.

This is Cycle 05 of [Wave 02](../Waves/wave-02-collaboration.md), tracked by
ticket `01K7F6H9J2N4Q6S8V0X2Z4B6DD`. The
[Git workflow RFC](../RFC/git-workflow-and-conflict-recovery.md),
[repository/index RFC](../RFC/repository-index-persistence-and-refresh.md),
[authentication RFC](../RFC/authentication-and-credential-handling.md), and
[test strategy](../RFC/test-and-compatibility-strategy.md) are authoritative.
The [design](../plans/2026-10-06-wave-02-cycle-05-clean-deliberate-synchronization-design.md)
and [implementation plan](../plans/2026-10-06-wave-02-cycle-05-clean-deliberate-synchronization-implementation.md)
were approved together by the user on 2026-10-06. Explicit implementation
authorization remains a separate gate.

## Entry Evidence And Dependency Boundary

- Reused the requested ticket worktree and branch; no replacement ticket or
  worktree was created.
- Fresh `git fetch origin main` observed `origin/main` at
  `034469e92a67624b34e700d772f09a69da45b868`. The clean ticket head rebased
  from `81fecfe7127b664c5b3521fd8938908d12e801d8` to
  `e6eff2654c2785250418a9df64d6710a04f0872a` without conflicts. The fetched
  base is an ancestor of the resulting head and the ticket worktree was clean.
- The main checkout's unrelated untracked `.superpowers/` directory and every
  other ticket worktree were preserved.
- Cycle 03 supplies selected-key, host-trust, and scoped authenticated transfer
  behavior. Cycle 04 supplies the ref plan, non-secret remote observations,
  durable reservations, and safe-point model. At this planning checkpoint,
  Cycle 04 is still in progress; its unmerged implementation is a dependency,
  not Cycle 05 evidence.

Before implementation, rebase this ticket on the then-current main, confirm
that Cycle 04's reviewed public contract is present, reconcile type names
without changing the policy below, and run the baseline required by the
implementation plan. A Cycle 04 incompatibility is a planning/implementation
blocker, not authority to reimplement its observation or reservation model.

## Scope

- Typed primary and context synchronization requests that accept canonical root,
  stable operation ID, validated target identity, host approval, and explicit
  recovery/restart intent, but never caller-supplied URLs, refspecs, OIDs,
  key paths, worktree paths, or credentials.
- Exact authenticated fetches: primary only for primary synchronization; the
  primary and exactly one shared-context ref for context synchronization.
- Complete post-fetch remote observation, with local tracking OIDs recorded
  through Cycle 04's non-secret observation model before a branch is changed.
- Clean-only local fast-forwards, computed before mutation so a later
  primary/context incompatibility cannot leave a partially integrated branch.
- Ordinary non-force context or primary push, durable ambiguity/retry state,
  and post-push observation before reporting publication.
- `Published`, `AlreadyCurrent`, local-only `PublishPending`, merge-required,
  remote-deleted, primary-absent, dirty/conflicted, recovery-required, and
  index-pending outcomes with fixed, redacted diagnostics.
- Idempotent discovery handoff after a stable result. A discovery failure
  reports index pending and never repeats a verified push or fast-forward.
- Disposable real authenticated-SSH tests for two clones, exact ref scope,
  fast-forward/current/pending outcomes, rejection and uncertain-push replay,
  primary preservation, privacy, and all native CI targets.

## Explicit Exclusions And Downstream Obligations

This Cycle does not create merge commits, rebase, resolve conflicts, select
conflict sides, checkpoint content, submit comments, poll, materialize a
remote-only context, prune refs, promote a document, close a ticket, delete a
branch/worktree, start a scheduler, or add desktop/CLI grammar or UI.

Cycle 06 owns every divergent-history merge and conflict recovery path. Cycle
07 owns comment checkpoint-to-publication composition. Cycle 08 owns one-shot
polling, wildcard fetch, automatic clean updates, canonical-tree validation,
and materialization. Cycles 09 and 10 own confirmation, integration, closure,
and cleanup. Wave 03 owns caller interaction and scheduling. This Cycle must
return a typed boundary to those owners rather than implement a convenient
subset of their behavior.

## Required Synchronization Contract

For publication remote `R`, primary `P`, context `C`, and their Cycle 04
tracking refs `T(P)` and `T(C)`, the only deliberate fetch mappings are:

```text
primary: +P:T(P)
context: +P:T(P), +C:T(C)
```

`+` permits a local *tracking* ref to represent a remote rewind. It never
permits a local branch update, a force push, ref pruning, or deletion. The only
push mapping is the ordinary exact mapping `local:C` to remote `C` (or `P:P`),
with no leading `+`, wildcard, deletion, or arbitrary caller input. `FETCH_HEAD`
remains unchanged.

Both actions first prove the target worktree is the expected worktree on the
expected branch, clean, and non-conflicted. Primary synchronization checks the
configured primary worktree. Context synchronization checks only the one
deterministic existing item-context worktree; it never materializes one. The
same checks are repeated while holding the existing short repository lease
after every network phase and immediately before the ref/worktree transition.
No network request, credential prompt, or discovery scan holds that lease.

For primary synchronization, after a current fetch:

1. `P == T(P)` is `AlreadyCurrent`.
2. `P` ancestor of `T(P)` is one clean fast-forward to `T(P)`.
3. `T(P)` ancestor of `P` needs no local integration and may need a normal
   push.
4. Otherwise return `MergeRequired` without changing `P`.

For context synchronization, compute a virtual final OID before changing `C`:

1. An existing `T(C)` may replace `C` only when `C` is its ancestor. If
   `T(C)` is an ancestor of `C`, keep `C`; if neither is an ancestor, return
   `MergeRequired` without changing `C`.
2. `T(P)` must exist. It may replace that virtual context only when the virtual
   context is its ancestor. If `T(P)` is an ancestor, keep the virtual context;
   otherwise return `MergeRequired` without changing `C`.
3. If `C` is absent from a complete current fetch, first publication is allowed
   only with Cycle 04 `never_published` evidence. `observed_published` means
   remote-branch-deleted and `history_unknown` remains recovery-required; both
   preserve local state and require their later explicit recovery boundary.

The operation updates a ref and its clean worktree only under the short lease
using expected-old-OID checks and safe checkout. It does not stage, stash,
reset, discard, overwrite, or clean files. A local ref updated before an
unexpected checkout failure is durable recovery state, never silently rolled
back or re-run as a fresh synchronization.

Before a push, the operation durably records its candidate local OID, then
re-observes the actual Push-direction endpoint. A configured `pushurl` is
treated as a potentially distinct destination: its advertised target OID must
be equal to or an ancestor of the candidate before the ordinary push, while
fetch tracking refs continue to describe the Fetch-direction endpoint. The
implementation must never infer equality of fetch and push endpoints, persist
either URL, or disclose it in an outcome. It observes the Push-direction target
again after a nominally successful push; only an exact candidate OID is
`Published`. A disconnect, persistence failure, or uncertain result after the
push is reconciled by fresh observation before any later push attempt.

No configured publication remote is a local-only `PublishPending` outcome and
does not manufacture a remote reservation, after the local target has passed
the same identity/cleanliness preflight. A configured remote with missing
primary, unavailable trust/key/transport, rejected push, changed observation,
or unclear publication is a recoverable typed result, not a local-only success.

After `Published` or `AlreadyCurrent`, release the lease and run the existing
non-mutating discovery refresh. If the authoritative Git result is known but
refresh fails, persist and return `IndexPending` around that exact result; an
identical operation replay performs only the refresh handoff. It never fetches,
fast-forwards, or pushes again merely because indexing failed.

## Reservation, Cancellation, And Recovery

Cycle 05 extends Cycle 04's durable operation envelope rather than adding a
second lock. A manual action finding a poll requests a yield and returns the
existing retryable `PollYielding`; it does not steal the reservation. Duplicate
operation IDs must match their root and target exactly. They return the durable
terminal result, retry only the recorded discovery handoff, or require explicit
restart/reconciliation for incomplete work. A different target with the same
ID is rejected.

Named safe points include before and after each fetch/advertisement phase,
before and after the local fast-forward transition, before push, after an
observed push, and before the discovery handoff. Cancellation/yield is honored
only at those boundaries, never during a libgit2 call or a short transaction.
The operation records only validated target identities, ref names, OIDs,
non-secret phases, fixed categories, and timestamps. Git refs, worktrees, and
canonical files win over a stale record during reconciliation.

## Acceptance And Exit Evidence

| Contract | Required evidence |
| --- | --- |
| Exact ref scope | Pure and real-remote tests prove the primary/context fetch and ordinary push refspecs; `FETCH_HEAD`, unrelated tracking refs, and remote branches remain unchanged. |
| Clean integration | Primary and context tests prove one fast-forward, already-current, locally-ahead publish, and virtual planning that refuses a later divergence before changing a local branch. |
| Preservation | Dirty, conflicted, wrong-branch, wrong-worktree, missing-primary, remote-deleted, history-unknown, divergent, and checkout-failure cases preserve local content and produce the assigned typed boundary. |
| Publication | Two authenticated clones prove first publication, local-ahead ordinary push, current result, server non-fast-forward rejection, distinct push-destination preconditions, and post-push OID observation. |
| Replay | Failure injection before/after fetch, fast-forward, push start, push acknowledgement, durable persistence, and discovery proves retry does only unfinished work and creates no duplicate push-side effect. |
| Discovery | Stable results refresh discovery once; an injected refresh failure produces index pending whose replay does not contact the remote or alter a ref. |
| Privacy and platform | Records, WAL/journal/backups, formatted errors, fixture output, and diagnostics retain only approved metadata; focused tests run in all five native CI targets after authorized publication. |
| Regression | Required Devenv checks, focused authenticated-SSH tests, independent review, and actual native CI evidence pass after implementation. |

## Review Record

| Source | Concern | Classification | Resolution | Evidence or follow-up |
| --- | --- | --- | --- | --- |
| Git workflow RFC, deliberate item sync | Sequential context then primary checks could fast-forward the context before discovering a required merge. | Ruling | Compute one virtual final OID under a lease; mutate only after both ancestry checks pass. | No-partial-fast-forward tests. Cost if wrong: a recovery-visible clean fast-forward; this design avoids it. |
| Git workflow RFC, `pushurl` | Fetch and push URLs may name distinct destinations. | Ruling | Keep fetch tracking evidence endpoint-specific; independently advertise the Push-direction target and require its target OID to be equal/ancestor before ordinary push. | Different fetch/push fixture plus no-endpoint-leak scan. Cost if wrong: one additional authenticated advertisement, not an unsafe assumption. |
| Git workflow RFC, post-push failure | Server acceptance can precede a client disconnect or SQLite write failure. | Settled | Persist push-start intent, re-observe actual refs on resume, and repeat only unproved work. | Ambiguous-push, post-push-persistence, and exact-ID replay tests. |
| Repository/index RFC, lease semantics | Long transport or discovery could block local work. | Settled | Hold only reservation across remote work; use the common-Git lease around each short local mutation/re-observation and release it before refresh. | Two-service arbitration and lease-observer tests. |
| Authentication RFC, secrets and prompts | Synchronization could bypass selected-key or disclose an endpoint/backend message. | Settled | Reuse Cycle 03's scoped transport and shared session; fixed redacted outcomes only. | Wrong/cancelled key, host approval, and privacy scans. |
| Test strategy, fixture fidelity | A mock cannot prove libgit2 tracking updates or an ambiguous real push. | Settled | Use disposable authenticated SSH remotes and two real clones; native matrix remains an implementation gate. | Focused fixture target and five actual CI runs. |
| Lifecycle | Planning could imply implementation or ticket closure. | Settled | This ticket remains open; approval of all three artifacts plus explicit implementation authority are separate gates. | Ticket checkpoints, review-ready comment, PR approval, then closure before merge. |

No unresolved product, trust, recovery, or scope decision remains in this
proposal. The endpoint ruling deliberately preserves configured Git `pushurl`
semantics while avoiding an unsafe equivalence assumption.

Record planning, per-task progress, decisions, baseline/final verification,
review, and review-ready status as ticket comments. Keep the ticket open until
the reviewed implementation receives code-review or PR approval; publishing,
merging, closing, and worktree cleanup need their own authorization.
