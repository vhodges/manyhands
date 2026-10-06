---
title: "Wave 02 Cycle 04: Remote Observation And Recovery Model"
date: 2026-10-05
status: approved
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M50000004E8C77AFBFD16B25"
---

# Wave 02 Cycle 04: Remote Observation And Recovery Model

## Purpose And Authority

Establish the durable, non-secret remote-ref observation and coordination model
that later Wave 02 synchronization operations consume. This Cycle proves that
Manyhands can authenticate, observe a complete advertised ref set, retain the
meaning of an absent previously observed context, and arbitrate a manual action
against an in-progress poll without changing a local branch or worktree.

This is Cycle 04 of [Wave 02](../Waves/wave-02-collaboration.md), tracked by
ticket `01K7F6H9J2N4Q6S8V0X2Z4B6DC`. The
[Git workflow RFC](../RFC/git-workflow-and-conflict-recovery.md),
[repository/index RFC](../RFC/repository-index-persistence-and-refresh.md),
[MVP RFC](../RFC/mvp-rfc.md), and
[test strategy](../RFC/test-and-compatibility-strategy.md) are authoritative.
The detailed [design](../plans/2026-10-05-wave-02-cycle-04-remote-observation-and-recovery-design.md)
and [implementation plan](../plans/2026-10-05-wave-02-cycle-04-remote-observation-and-recovery-implementation.md)
must be reviewed together.

## Entry Evidence

- Reused the requested ticket branch and worktree; no replacement ticket or
  worktree was created.
- Fresh `git fetch origin main` observed `origin/main` at
  `034469e92a67624b34e700d772f09a69da45b868`.
- The clean ticket branch rebased from
  `d7df38e5a89ded1ae897ac6ac21bf44aea16cb00` to
  `3db08a27414269dccbaa2eed52a870f52974694f` without conflicts. The fresh
  base is an ancestor of that head and the post-rebase ticket worktree is clean.
- The main checkout's untracked `.superpowers/` directory was left untouched.
  No other ticket worktree was changed.
- Cycle 03's recorded verification and native evidence establish the selected
  SSH-key/host-trust transport foundation. They are prior evidence, not a fresh
  Cycle 04 Rust verification run.

Before implementation, repeat the fetch/rebase/ancestry preflight at the work
boundary and run the baseline commands in the implementation plan. The rebase
rewrote the published ticket checkpoint, so any later authorized publication
must first compare the ticket remote history and reconcile it without a force
push unless separately authorized.

## Scope

- One validated fetch-ref plan for a configured primary branch and the two
  recognized shared-context families, including their exact remote-tracking
  destinations and the RFC-required leading `+`.
- Authenticated, read-only remote advertisement observation using Cycle 03's
  scoped transport boundary; durable storage of the advertised ref set, local
  tracking-ref observation, polling policy/status/backoff, and redacted
  transport result.
- Durable repository-scoped remote-operation reservations with poll/manual
  priority, yield requests, cancellation requests, safe-point acknowledgement,
  and restart reconciliation.
- Snapshot-visible states for observed, unmaterialized, malformed, and
  remotely deleted context references, with fixed recovery guidance and no
  canonical content inferred from cache data.
- Migration of existing application-local SQLite state without exposing remote
  URLs, credentials, passphrases, private-key material, response bodies, or
  Markdown content.

## Explicit Exclusions And Downstream Obligations

This Cycle does not call `download`, alter a local or remote branch, create a
remote-tracking ref, fast-forward, merge, push, checkpoint, materialize a
worktree, refresh the canonical index, schedule a resident poller, or add a
desktop/CLI command. An authenticated advertisement may finalize an already
approved host pin and records its own local recovery state, but it changes no
Git ref, worktree, or canonical file.

Cycle 05 consumes the exact primary/context ref plan for deliberate fetch and
clean synchronization. Cycle 06 owns merge and conflict recovery; Cycle 07 owns
comment-triggered publication; Cycle 08 owns fetch-based one-shot polling,
clean fast-forwards, canonical-tree validation, and materialization. Wave 03
owns scheduling and user interaction. Remote observation never treats a
recognized branch name as proof that its tree contains conforming canonical
content.

## Required Contract

For publication remote `R`, primary branch `P`, and a valid shared context
`manyhands/<kind>/<ULID>`, the Cycle exposes only the RFC-defined ref plan:

```text
+refs/heads/<P>:refs/remotes/<R>/<P>
+refs/heads/manyhands/document/*:refs/remotes/<R>/manyhands/document/*
+refs/heads/manyhands/ticket/*:refs/remotes/<R>/manyhands/ticket/*
```

The refspecs are persisted only as derived non-secret ref names; this Cycle
does not execute a fetch download. A successful authenticated advertisement is
recorded atomically as one complete batch. Only then may a ref observed in a
previous successful batch become `remotely deleted`. A failed, cancelled, or
partial observation never implies deletion and retains the last successful
observation.

The user approved a fail-closed recovery rule on 2026-10-05: a lost or
replaced remote-state registry never turns an absent local context into a first
publication candidate. Before corrupt-registry replacement, Manyhands must
durably publish a non-secret remote-history-recovery marker outside that
registry; failure to publish the marker aborts replacement. The marker overlays
automatic polling with `recovery suspended` until an explicit resume, without
rewriting the user's `paused` policy. While it remains, a local context whose
remote ref is absent is `history unknown` and requires explicit recovery or
republish confirmation in a later lifecycle Cycle. A newly observed remote ref
can restore `observed published` evidence for that context; a context never
observed remotely remains `never published` only when the history marker was
not active. The marker cannot be silently cleared by rebuild, polling, or a
failed observation.

| Visible state | Meaning and recovery boundary |
| --- | --- |
| `observed` | The complete advertisement contains a valid recognized context ref. Preserve its advertised OID and matching local tracking-ref observation. |
| `unmaterialized` | A valid recognized remote ref has no matching local context/worktree. Cycle 08 may validate and materialize it exactly once. |
| `malformed` | A ref under a recognized family cannot be parsed as exactly one supported kind plus canonical ULID. Preserve the ref observation and report fixed guidance; never create a branch/worktree. |
| `remotely deleted` | A context previously observed as published is absent from a later complete observation. Preserve any local context, branch, and worktree; explicit later confirmation governs republishing or cleanup. |
| `history unknown` | Remote-state history was lost and a local context is absent from an advertisement. Preserve all local state and require later explicit recovery/republish confirmation; do not call it first publication or remotely deleted. |

An automatic or one-shot poll reservation yields only at recorded safe points:
before transport, after a completed advertisement, between ref observations,
and before a future local mutation. A manual operation encountering an active
poll records a yield request and returns retryable `poll yielding`; it does not
steal the reservation or start a second remote operation. Cancellation follows
the same rule. Network work, prompts, and scans never hold the existing
common-Git-directory lease; a later local mutation must reacquire that lease
and re-observe state.

The user approved the desktop-process session-unlock model on 2026-10-05. At
desktop startup, the selected protected key for an eligible SSH publication
remote is unlocked through the process's one shared credential provider. The
transport makes its normal no-secret attempt first; when that requires an
ambiguity-aware unlock, the desktop deduplicates one masked prompt. A successful
credential is retained only in the shared process session and is reused by all
later manual and automatic polls until process exit, key/source/selection
change, or explicit clearing. Cancellation or unlock failure leaves the
affected polling session `unlock required` without changing the user's durable
pause policy or repeatedly prompting from background attempts. Host approval is
separate: an unknown or changed host suspends the affected polling session and
requires explicit exact-fingerprint recovery; it is never silently accepted.
Cycle 04 supplies the headless session-aware domain boundary, while Wave 03
owns the startup prompt, status presentation, and worker scheduling.

## Acceptance And Exit Evidence

| Contract | Required evidence |
| --- | --- |
| Ref protocol | Unit/integration cases derive only the three exact refspecs and tracking names, reject ref/path injection, and keep remote-name and primary-branch changes distinct. |
| Durable observations | An authenticated fixture advertisement produces a complete, queryable batch with advertised and pre-existing tracking OIDs; failed or cancelled calls cannot manufacture deletion. |
| Discovery states | Fixtures and direct state tests show unmaterialized, malformed, and remotely deleted contexts with fixed guidance, while confirming no branch, worktree, canonical file, or remote-tracking ref was created. |
| Cache loss | Corrupt-registry replacement publishes the recovery marker before replacement, suspends automatic polling without overwriting explicit pause, and makes an absent local context `history unknown` until explicit later recovery. |
| Session unlock | One process-scoped protected-key prompt at startup is reused by later polls; cancellation/failure blocks that session without repeated background prompts or durable-pause changes. |
| Reservation priority | Separate service instances prove a manual request asks an active poll to yield, the poll acknowledges only at a safe point, and retry obtains a fresh manual reservation after re-observation. |
| Restart/retry | Injected interruptions before/after transport and before/after persistence reconcile against durable records, repeat only a read-only observation where necessary, and retain the last confirmed batch. |
| Privacy | Schema, formatted outcomes, diagnostics, WAL/journal/backups, and fixture failures contain only approved IDs, ref names, OIDs, timestamps, and redacted categories. |
| Regression | Required Devenv checks, focused real-SSH observation tests, and all five native CI targets pass after authorized implementation. |

Record baseline, each implementation task, verification, review-ready status,
and any unresolved native evidence as ticket comments. The ticket remains open;
approval of these drafts authorizes neither implementation nor publication.
