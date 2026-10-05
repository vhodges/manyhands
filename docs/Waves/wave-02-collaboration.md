---
title: "Wave 02: Collaboration"
date: 2026-10-04
status: approved
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01K7F6H9J2N4Q6S8V0X2Z4B6D8"
---

# Wave 02: Collaboration

## Outcome

Wave 02 extends the completed local foundation with safe, headless
collaboration against configured SSH publication remotes. At its exit, an
enabled repository can manage one shared Manyhands SSH key, deliberately
synchronize its primary branch and item contexts, publish a newly checkpointed
comment, observe and safely poll a remote once, recover merges and conflicts,
promote a managed document, and close a ticket.

The Wave preserves the Wave 01 authority boundary: canonical Markdown, tracked
configuration, and actual Git state are authoritative; SQLite is rebuildable
local state and recovery evidence. Remote failures, conflicts, and interrupted
operations preserve local work and identify a retry or recovery action. This
Wave provides shared-domain operations only. Desktop and CLI interfaces,
resident poll scheduling, and interaction workflows are Wave 03 work.

## Authority And Traceability

Wave 02 implements the remote-collaboration outcome in the approved
[MVP architecture RFC](../RFC/mvp-rfc.md) and PRD version 0.4. It depends on
the following authorities:

- [Canonical content and comment schema RFC](../RFC/canonical-content-and-comment-schema.md)
- [Git workflow and conflict recovery RFC](../RFC/git-workflow-and-conflict-recovery.md)
- [Repository index persistence and refresh RFC](../RFC/repository-index-persistence-and-refresh.md)
- [Test and compatibility strategy RFC](../RFC/test-and-compatibility-strategy.md)
- [Authentication and credential handling RFC](../RFC/authentication-and-credential-handling.md)

Primary PRD traceability:

| Area | Requirements addressed in this Wave |
| --- | --- |
| SSH credentials | `MH-CRED-001` and `MH-NFR-003` |
| Remote collaboration | `MH-COLLAB-003` to `MH-COLLAB-007` |
| Content and comments | The shared-context portion of `MH-CONTENT-004`, `MH-COMMENT-001`, and `MH-COMMENT-002` |
| Indexing | Remote portions of `MH-INDEX-001` to `MH-INDEX-003` |
| Quality | `MH-NFR-001`, `MH-NFR-006`, `MH-NFR-007`, and `MH-NFR-008` |

## Required Source Amendments

The following decisions are prerequisites, not implementation work. They must
be approved before any Wave 02 Cycle begins. A Cycle document may refine an
approved mechanism but must not choose among these unresolved alternatives.

| Source | Required amendment or decision | Reason |
| --- | --- | --- |
| New authentication RFC | Define supported key algorithms and formats; generated and imported key ownership; owner-only storage; public fingerprint/label metadata; shared-key selection; session-only passphrase handling; host verification; `git2` callback behavior; deletion confirmation; and secret-redaction evidence. | SSH credentials and transport are explicit MVP decisions, not implementation details. |
| `docs/PRD/mvp.md` and `docs/RFC/mvp-rfc.md` | Replace the multiple-editable-context selection requirement with one shared remote context branch per item. Distinguish a non-mutating index-only refresh from a remote poll's authorized fetch, clean fast-forward, and worktree materialization. | A deterministic branch and worktree name provides one context per item in a local clone. ULIDs identify items, not independent mutable contexts; an absolute refresh non-mutation rule conflicts with authorized polling updates. |
| `docs/RFC/git-workflow-and-conflict-recovery.md` | Define exact fetch and push refspecs; remote-tracking ref names; remote-branch absence behavior; ancestry checks; merge and push ordering; non-fast-forward rejection; relevant object-ID observations; safe cancellation points; and retry steps for fetch, merge, push, promotion, closure, and cleanup. | The current synchronization wording does not make remote operations or replay idempotent enough to implement safely. |
| `docs/RFC/repository-index-persistence-and-refresh.md` | Define remote-ref observations; persisted polling pause, interval, status, and backoff state; unmaterialized and remotely deleted context presentation; manual-operation priority over polling; operation reservation behavior; remote-aware recovery fields; and redacted failure records only. | The Wave 01 cache schema covers only local contexts and short repository leases, and its unrestricted failure-detail field conflicts with the Wave's no-secret record contract. |
| `docs/RFC/test-and-compatibility-strategy.md` | Define a disposable authenticated SSH remote or equivalent cross-platform Git-server fixture, deterministic transport failures, host-verification cases, polling/manual serialization, remote deletion, merge conflict, cleanup-retry, and secret-redaction evidence. | A bare local remote alone cannot prove the selected-key SSH transport contract. |

## Entry Gate

Wave 02 implementation may begin only when all conditions hold:

- The new authentication and credential-handling RFC is approved.
- The required PRD, MVP RFC, Git workflow, repository/index, and test strategy
  amendments are approved and mutually consistent.
- Wave 01's integration gate has recorded evidence or its required verification
  suite has been rerun successfully against the current lockfile and source.
- The shared-context branch protocol remains
  `manyhands/<kind>/<ULID>`; no Cycle introduces a second context identity or
  a user context-selection result.
- Each Cycle document exists at its required path and identifies its exact
  implementation, recovery, fixture, and test changes before implementation
  begins.

## Scope

Wave 02 includes:

- Application-local registry, import, generation, selection, removal, and
  explicit deletion behavior for one shared Manyhands SSH key.
- Protected generated-key storage, session-only passphrase retention, selected
  key use for every Git-over-SSH operation, host verification, and redacted
  credential outcomes.
- Authenticated `git2` fetch and push against the configured SSH publication
  remote. A system Git executable, SSH-agent fallback, and default-key fallback
  are not runtime dependencies.
- Durable remote operation observations and reconciliation that retain only
  non-secret identifiers, progress, and redacted failure data.
- Deliberate item-context and primary-branch synchronization, including
  published, already-current, publication-pending, and recoverable outcomes.
- Merge conflict preservation and a headless resolution/checkpoint retry
  boundary for future desktop and CLI callers.
- Immediate synchronization after a comment checkpoint when a publication
  remote is configured; otherwise, a local publish-pending result.
- Explicit one-shot polling, persisted polling policy and status, clean-only
  fast-forwards, and idempotent materialization of a newly discovered shared
  item context.
- Confirmed managed-document promotion and ticket closure, including local-only
  publication-pending results and cleanup only after required integration and
  publication steps succeed.
- Disposable authenticated-remote tests, transport and lifecycle failure
  injection, and remote collaboration acceptance evidence.

## Explicit Exclusions

Wave 02 does not implement:

- Desktop navigation, prompts, progress display, conflict editor, keyboard
  workflows, or accessibility behavior.
- CLI command grammar, JSON output, or resident polling scheduling. Wave 03
  invokes the Wave 02 one-shot poll operation from a desktop-owned background
  worker and explicit CLI commands. PRD 0.5 removes CLI daemon mode without
  changing Wave 02's one-shot operations or repository coordination.
- OAuth, Git-forge APIs, automatic public-key upload, HTTP(S) publication,
  repository cloning, SSH-agent fallback, or fallback to a user's default key.
- Filesystem watching, automatic retry scheduling, or a second source of truth
  for remote or canonical content.
- Automatic rebase, force push, merge or cleanup without a deliberate and
  appropriately confirmed lifecycle action, or automatic deletion of remotely
  deleted local contexts.
- A second editable context branch or context-choice behavior for a single item.
  Concurrent collaborators use the one shared branch and receive merge or
  conflict recovery outcomes when their commits diverge.

## Required Collaboration Contracts

### Shared Context Branches

Each document or ticket has at most one recognized Manyhands context branch per
repository:

```text
manyhands/document/<ULID>
manyhands/ticket/<ULID>
```

The matching local worktree remains at `.manyhands/worktrees/<ULID>/`. A
collaborator's separate clone may materialize that same branch in its own
deterministic worktree. A local clone never offers multiple editable contexts
for the same item. A shared branch may diverge when collaborators checkpoint
from different clones; synchronization merges that history or leaves an
inspectable conflict for explicit resolution.

### Credentials And Transport

One user-selected Manyhands SSH key is used for every Wave 02 Git-over-SSH
operation. Imported keys remain at their supplied source and are unregistered,
not deleted, when removed. Generated keys use RFC-defined owner-only storage;
their private contents and passphrases never enter SQLite, tracked repository
state, diagnostics, errors, logs, snapshots, or machine-readable outcomes.

The first protected-key use in an application session requests a passphrase
through a caller-supplied session credential provider. Cancellation, an invalid
or inaccessible key, host-verification failure, authentication rejection, and
transport unavailability are typed recoverable outcomes that identify the key,
repository, remote, and requested action without exposing secrets.

### Coordination And Recovery

Wave 01's common-Git-directory lease remains limited to short local atomic
steps. Network transport, long scans, and caller interaction never hold it.
Wave 02 adds an approved durable reservation protocol so polling yields to a
manual lifecycle operation at a safe point; after a network step, an operation
reacquires the short lease and re-observes Git state before mutating refs,
worktrees, or the cache.

Operation records contain no drafts, Markdown bodies, passphrases, private-key
content, or credential-like values. They record only the target repository,
operation identity, action, remote/ref names, observed object IDs, completed
steps, recovery state, cancellation state, and redacted error category. Actual
Git refs, commits, worktrees, and canonical Markdown always win when they
disagree with an operation record.

### Polling And Discovery

A one-shot poll fetches the approved ref set, observes the primary branch and
recognized shared context branches, and refreshes discovery. It fast-forwards a
local primary or existing context only when that worktree is clean,
non-conflicted, and strictly behind the matching remote-tracking branch. It
materializes a newly discovered recognized, conforming shared context exactly
once, then indexes it.

Dirty, divergent, conflicted, malformed, inaccessible, renamed, unrecognized,
unmaterialized, or remotely deleted contexts remain visible with recovery
guidance and are never automatically overwritten or removed. The local polling
pause, interval, latest status, and bounded backoff state are persisted, but no
Wave 02 process runs a resident scheduler.

## Ordered Cycles

Individual Cycle documents MUST be created at the listed paths before their
implementation begins. The summaries define Cycle scope and exit evidence; they
are not file-level implementation plans.

### Cycle 01: Shared-Key Registry

**Planned document:** `docs/Cycles/wave-02-cycle-01-shared-key-registry.md`

**Purpose:** Establish application-local, non-secret SSH-key registration and
single shared-key selection without using a network transport.

**In scope:** Key labels and public fingerprints; references to imported key
sources; generated-key ownership metadata; validation; list, select, unregister,
and explicit deletion-intent outcomes; and secret-free SQLite migration.

**Out of scope:** Key generation, key-file writes, passphrase handling, host
verification, fetch, push, or remote synchronization.

**Exit evidence:** Tests prove canonical metadata uniqueness, one shared
selection, imported-key non-deletion, generated-key deletion confirmation,
corrupt/missing key recovery guidance, and absence of secret persistence.

### Cycle 02: Generated Keys And Session Unlock

**Planned document:** `docs/Cycles/wave-02-cycle-02-generated-keys-and-unlock.md`

**Purpose:** Create and safely use generated or imported key material while
retaining passphrases only for the application session.

**In scope:** RFC-approved key generation and public-key derivation, optional
passphrase protection, platform-appropriate owner-only storage, imported-key
readability validation, session credential provider, unlock cancellation, and
explicit generated-key deletion.

**Out of scope:** Git remote callbacks and every fetch, push, polling, merge,
or publication operation.

**Exit evidence:** Tests prove key files follow the approved protection model,
passphrases do not enter persistent state or diagnostics, session unlock is
reused only within its session, and key access failures preserve registrations
and report actionable recovery.

### Cycle 03: Authenticated SSH Transport

**Planned document:** `docs/Cycles/wave-02-cycle-03-authenticated-ssh-transport.md`

**Purpose:** Bind the selected shared key and approved host-verification policy
to `git2` remote callbacks and prove authenticated transport against a real
fixture.

**In scope:** SSH-only callback construction, selected-key enforcement,
passphrase handoff, host verification, redacted credential/transport outcomes,
and disposable authenticated SSH remote fixtures.

**Out of scope:** Remote-ref interpretation, synchronization semantics, polling,
merge, materialization, promotion, and closure.

**Exit evidence:** A real authenticated fetch or equivalent approved server
fixture succeeds with the selected key; wrong key, locked key, cancelled unlock,
host-verification, and network failures are typed, redacted, and non-mutating.

### Cycle 04: Remote Observation And Recovery Model

**Planned document:** `docs/Cycles/wave-02-cycle-04-remote-observation-and-recovery.md`

**Purpose:** Make the approved remote-ref protocol and remote lifecycle state
durable and observable before any operation changes local branches.

**In scope:** Exact fetch refspecs and remote-tracking observations; polling
policy, status, and backoff records; remote-aware operation fields; manual
priority reservations; safe cancellation boundaries; and remote-context
discovery states.

**Out of scope:** Fast-forward, merge, push, comment publication, scheduler
ownership, promotion, closure, and worktree cleanup.

**Exit evidence:** Fixtures prove deterministic remote observations, no secret
records, retry after interruption, safe manual-versus-poll arbitration, and
visible unmaterialized, malformed, and remotely deleted remote states.

### Cycle 05: Clean Deliberate Synchronization

**Planned document:** `docs/Cycles/wave-02-cycle-05-clean-deliberate-synchronization.md`

**Purpose:** Synchronize an item context or the primary branch when no merge is
required, then refresh discovery.

**In scope:** Deliberate fetch, clean fast-forward or already-current result,
safe push, strict non-fast-forward rejection, publish-pending outcomes, primary
worktree cleanliness checks, and idempotent discovery handoff.

**Out of scope:** Merge commits, conflict resolution, comment-triggered sync,
polling, promotion, closure, and cleanup.

**Exit evidence:** Real remote tests prove item and primary published/current/
pending outcomes, no duplicate push-side lifecycle artifacts after retry, and
preservation of dirty or conflicted primary state.

### Cycle 06: Merge And Conflict Recovery

**Planned document:** `docs/Cycles/wave-02-cycle-06-merge-and-conflict-recovery.md`

**Purpose:** Extend deliberate synchronization to safely merge divergent shared
history and preserve conflicts for explicit resolution.

**In scope:** Non-rebase merge creation, durable conflict and merge observations,
a headless caller-supplied resolution/checkpoint boundary, retry reconciliation,
and discovery refresh after a stable result.

**Out of scope:** Desktop or CLI conflict presentation, automated conflict-side
selection, comment-triggered sync, polling, promotion, closure, and cleanup.

**Exit evidence:** Real remote divergence tests prove clean merges retain both
histories, conflicts retain the affected worktree and markers, resolution retry
does not repeat completed network or merge steps, and no local work is lost.

### Cycle 07: Comment Publication

**Planned document:** `docs/Cycles/wave-02-cycle-07-comment-publication.md`

**Purpose:** Make comment submission the required compound local checkpoint and
immediate item-context synchronization action.

**In scope:** Handoff from the authoritative comment checkpoint to deliberate
context synchronization; combined published, already-current, local-pending,
and recovery outcomes; and replay that never duplicates a comment or checkpoint.

**Out of scope:** General item saves, primary synchronization, polling, merge
implementation beyond Cycle 06 reuse, promotion, closure, and cleanup.

**Exit evidence:** A new root comment and reply are published when possible,
remain discoverable locally when no remote or remote recovery failure exists,
and publish on a later retry without duplicate files or commits.

### Cycle 08: One-Shot Polling And Materialization

**Planned document:** `docs/Cycles/wave-02-cycle-08-one-shot-polling-and-materialization.md`

**Purpose:** Safely observe a remote once, update clean local state, and
materialize newly discovered conforming shared contexts.

**In scope:** One-shot poll orchestration; configured pause/interval/backoff
state; manual-operation priority; clean-only primary/context fast-forwards;
single materialization of a new recognized context; discovery refresh; and
visible exceptional state.

**Out of scope:** Resident desktop scheduling, automatic retry,
push, checkpoint, merge, rebase, promotion, closure, and cleanup.

**Exit evidence:** Real remote tests prove clean updates and new-context
materialization occur once, while dirty, divergent, conflicted, malformed,
inaccessible, unrecognized, renamed, and remotely deleted contexts remain
unchanged and actionable.

### Cycle 09: Confirmed Managed-Document Promotion

**Planned document:** `docs/Cycles/wave-02-cycle-09-confirmed-document-promotion.md`

**Purpose:** Integrate a document context into primary through an explicitly
confirmed, recoverable lifecycle.

**In scope:** Headless preflight and caller confirmation contract; final
validation/checkpoint handoff; required context synchronization; clean-primary
precondition; non-fast-forward primary merge; optional publication; local-only
publish-pending result; and cleanup only after required success.

**Out of scope:** Desktop confirmation UI, ticket closure metadata, automatic
cleanup outside promotion, and resident polling scheduling.

**Exit evidence:** Tests prove confirmed document promotion preserves inspectable
history, creates a fresh context after later editing, blocks on dirty primary,
defers cleanup on failure, and retries only pending publication or cleanup.

### Cycle 10: Confirmed Ticket Closure

**Planned document:** `docs/Cycles/wave-02-cycle-10-confirmed-ticket-closure.md`

**Purpose:** Close a ticket with its required closure metadata and the same
guarded integration, publication, and cleanup safety boundary.

**In scope:** Headless preflight and caller confirmation contract; closure
metadata with the confirmed Git identity; final checkpoint; context and primary
synchronization; non-fast-forward merge; optional publication; local-only
publish-pending result; and cleanup-only retry.

**Out of scope:** Ticket reopening, desktop confirmation UI, automatic cleanup
outside a confirmed close, and resident polling scheduling.

**Exit evidence:** Tests prove a closed primary ticket is discoverable according
to the schema, local-only closure remains publish pending, failure preserves the
context before cleanup, and retry creates no duplicate closure checkpoint,
merge, branch deletion, or worktree removal.

## Integration Gate

Wave 02 is complete only when all ten Cycles have exit evidence and the
following conditions are met:

- A selected shared SSH key authenticates every Git-over-SSH operation through
  `git2`; no passphrase, private-key content, credential, or unredacted remote
  response is stored or exposed.
- Two collaborators using separate clones can checkpoint the same shared item
  branch, synchronize non-conflicting work, and receive a recoverable conflict
  without silent overwrite when their work diverges.
- Deliberate item and primary synchronization report published, already-current,
  publication-pending, or actionable recovery while preserving local work.
- Comment submission checkpoints once and then immediately attempts publication
  when configured; failed publication leaves exactly one locally discoverable
  comment for retry.
- One-shot polling obeys stored policy and manual-operation priority, fetches
  without publication, fast-forwards only clean strictly-behind state,
  materializes each valid new shared context once, and preserves all exceptional
  contexts locally.
- Confirmed promotion and closure use non-fast-forward primary merges, publish
  when required, never clean up prematurely, and replay only unfinished
  publication or cleanup after interruption.
- Remote operation records reconcile from actual Git and canonical Markdown,
  with no duplicate comments, checkpoints, merges, branches, worktrees,
  materializations, pushes, or cleanup effects.
- Disposable authenticated SSH remote tests cover transport, host, offline,
  dirty, divergent, conflicted, remote-deleted, malformed, cancellation, and
  interruption cases without reading developer keys, global Git configuration,
  or application data.
- The required Rust verification suite succeeds:

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --all-features --locked
  ```

- The CLI and desktop smoke tests run when their applicable Wave 03 command and
  scheduler integrations exist. They are not Wave 02 implementation evidence.

## Risks And Controls

| Risk | Control and required evidence |
| --- | --- |
| Authentication implementation exposes or persists secrets | Approve the credential RFC first; use non-secret metadata only; test redaction, session-only passphrases, imported-key non-deletion, and generated-key protection. |
| libgit2/libssh2 behavior differs across supported platforms | Use the locked dependency stack with a disposable authenticated fixture; document platform deviations as compatibility cases rather than adding fallbacks. |
| Shared-branch collaborators overwrite one another | Synchronize by fetch, ancestry checks, merge, and push; retain conflicts and require explicit caller resolution. Never rebase, force push, or choose a conflict side automatically. |
| A remote operation holds the Wave 01 lease too long | Keep network work outside the short lease; use approved durable reservation and re-observe local Git state before every atomic change. |
| Polling races a manual lifecycle action | Persist polling state and manual-priority reservation; poll yields or stops only at defined safe points. Prove the ordering with real remote tests. |
| Remote deletion or malformed state destroys local recovery work | Preserve local worktrees and branches; surface remote observation problems without automatic deletion or overwrite. |
| Retry duplicates a push, merge, materialization, or cleanup | Record non-secret observed ref and object-ID transitions; reconcile actual Git state before replay; test interruption at each remote lifecycle boundary. |
| Wave scope leaks into UI or background scheduling | Keep interfaces, background scheduler ownership, progress presentation, and command contracts in Wave 03. Wave 02 exposes typed domain outcomes only. |
| Existing multiple-context wording conflicts with implementation | Approve the shared-branch PRD and RFC amendment in the entry gate; do not implement a hidden alternate branch identity. |

## Deferred Work

Wave 03 owns desktop information architecture and editor behavior, accessible
credential and confirmation prompts, interactive conflict resolution,
progress/cancellation presentation, CLI command and JSON contracts, desktop
background polling schedules and worker shutdown, and end-to-end dogfooding
journeys on supported platforms. CLI resident polling/indexing is deferred
under PRD 0.5; explicit one-shot CLI polling and index refresh remain in scope.

No Cycle may use a deferred interface or scheduler as a hidden prerequisite. If
the approved remote protocol or authentication RFC cannot support the required
headless operation safely, work pauses for an RFC or Wave amendment rather than
adding an unreviewed fallback.
