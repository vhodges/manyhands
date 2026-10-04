---
title: "Wave 01 Cycle 05: Recovery and Foundation Gate"
date: 2026-10-02
status: approved
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01K7E5G8J1M3P5R7T9V2X4Z6B8"
---

# Wave 01 Cycle 05: Recovery and Foundation Gate

## Parent Wave

This is Cycle 05 of [Wave 01: Foundations](../Waves/wave-01-foundations.md).
It joins Cycles 01 through 04 into one recoverable local lifecycle: enabled
repositories, deterministic contexts, canonical writes and checkpoints, and
rebuildable discovery remain safe when a process fails, another Manyhands
process is active, or an external Markdown author changes a worktree.

## Purpose

Implement the coordination and recovery boundary required before Manyhands
extends its local foundation with remote collaboration. A desktop process, a
one-shot CLI process, and an agent using the shared domain API may operate on
the same repository without duplicate local artifacts or silent overwrites.
They wait briefly only for the atomic part of a conflicting operation; long
draft preparation and discovery scans do not monopolize the repository.

Canonical Markdown, tracked configuration, and actual Git state remain
authoritative. SQLite records progress and discovery only. An interrupted
operation is reconciled from the repository and canonical files, never resumed
by treating an operation record as proof that a write, commit, branch, or
worktree exists.

## Prerequisites

- Cycles 01 through 04 exit criteria remain satisfied, including canonical
  validation, local enablement, deterministic local contexts, scoped
  checkpoints, read-only discovery, explicit-root rebuild, and the existing
  refresh/rebuild operation records.
- Wave 01's entry gate remains satisfied.
- The approved canonical schema, Git workflow, repository/index, and test
  strategy RFCs remain unchanged, except for the narrow source amendments
  listed below.
- Implementation starts from a clean `main` branch after the required source
  amendments are approved.

## RFC and PRD Traceability

| Source | Cycle responsibility |
| --- | --- |
| [Canonical content and comment schema RFC](../RFC/canonical-content-and-comment-schema.md) | Preserve canonical paths, validation, unknown front matter, bodies, and comment relationships while rejecting stale writes rather than overwriting externally edited Markdown. |
| [Git workflow and conflict recovery RFC](../RFC/git-workflow-and-conflict-recovery.md) | Coordinate local Git lifecycle actions through the repository common Git directory; reconcile actual refs, worktrees, commits, configuration, and files after interruption. |
| [Repository index persistence and refresh RFC](../RFC/repository-index-persistence-and-refresh.md) | Extend durable records to every Wave 1 lifecycle action, preserve SQLite as a non-authoritative cache, and safely replace a corrupt shared cache. |
| [Test and compatibility strategy RFC](../RFC/test-and-compatibility-strategy.md) | Prove all named failure boundaries, process-level lease behavior, stale-write handling, restart reconciliation, and the complete Wave 1 foundation gate with disposable real repositories. |
| `MH-REPO-001` to `MH-REPO-004` | Reconcile local creation, enablement, registration, and publication-remote configuration without remote contact or duplicate initialization/configuration commits. |
| `MH-CONTENT-001` to `MH-CONTENT-004` and `MH-COMMENT-001` to `MH-COMMENT-002` | Preserve valid documents, tickets, comments, ordering, active-context precedence, and direct external edits without silently replacing content. |
| `MH-COLLAB-001` to `MH-COLLAB-003` | Serialize context provisioning and local checkpoints across processes; automatically refresh discovery after each successful local lifecycle change. |
| `MH-INDEX-001` to `MH-INDEX-003` | Retain a rebuildable discovery cache, safe explicit-root recovery, and visible problems when scans cannot converge. |
| `MH-NFR-001`, `MH-NFR-006`, `MH-NFR-007`, and `MH-NFR-008` | Keep local work offline and recoverable; make retry idempotent; preserve canonical state during index work; and prevent silent loss. |

## Required Source Amendments

These amendments must be made and approved with Cycle 05 implementation. They
resolve conflicts between the approved Wave boundary and older RFC acceptance
wording; this Cycle does not silently supersede either source.

| Source | Required amendment | Reason |
| --- | --- | --- |
| `docs/RFC/git-workflow-and-conflict-recovery.md` | Replace the Wave 1 acceptance requirement that multiple contexts require selection with a recoverable duplicate or mismatched-context outcome. State that Wave 2 retains one shared context branch rather than adding context choice. | The Wave and Cycles 03/04 explicitly defer remote materialization, not a second context identity. |
| `docs/RFC/test-and-compatibility-strategy.md` | Replace the Wave 1 exit-gate requirement to expose a multiple-context choice state with evidence that duplicate or mismatched local contexts remain visible, preserved, and non-editable without a choice result. | The required test evidence must match the approved Wave 1 scope. |
| `docs/RFC/repository-index-persistence-and-refresh.md` | Replace SQLite-only coordination with a repository-common-Git-directory advisory lease for repository actions, plus a narrow application-data cache-recovery guard for SQLite transactions and database replacement. | A corrupt or replaced SQLite database cannot safely host the only coordination lease. |
| `docs/RFC/repository-index-persistence-and-refresh.md` | State that Cycle 05 extends Cycle 04's refresh/rebuild records to all Wave 1 lifecycle actions, while Git and canonical Markdown remain authoritative. | The Wave requires cross-lifecycle retry evidence, not just index recovery. |

## In Scope

- A headless shared-domain coordination and recovery extension. It has no
  GPUI, GPUI Kit, desktop, CLI command, daemon, SSH, transport, or network
  dependency.
- A short-lived exclusive advisory lease for each repository, anchored at its
  resolved common Git directory so the primary worktree and every linked
  worktree share one lease.
- Lease participation by every repository-mutating or repository-refreshing
  Wave 1 action: enablement, publication-remote configuration, context
  provisioning, canonical item writes, checkpoints, refresh, rebuild, and
  registration removal. Stored snapshot reads remain ordinary SQLite reads and
  do not acquire the repository lease.
- A fixed bounded lease wait of 250 milliseconds. A holder that does not
  release it in that period produces a typed, retryable busy outcome naming the
  repository and requested action. There is no queue, daemon, or unbounded
  wait.
- Draft preparation, caller interaction, validation that needs no repository
  mutation, and long discovery scans outside the repository lease. Each action
  re-observes required state after acquiring the lease and before altering or
  replacing durable state.
- A narrow application-data cache-recovery guard. Normal SQLite transactions
  take its shared form; a structural-corruption replacement takes its exclusive
  form only while preserving the corrupt database and installing a fresh one.
  The replacement holds no repository lease. After replacement, explicit-root
  rebuild proceeds through the normal repository lease and scan protocol.
- Caller-supplied opaque operation IDs for every Wave 1 lifecycle action. The
  caller retains and replays an ID after a timeout, reported failure, or process
  restart. Reuse of an ID for a different repository, action, or target returns
  a typed operation-mismatch outcome.
- Durable, non-content operation records for enablement, publication-remote
  configuration, context provisioning, canonical writes, checkpoints, refresh,
  rebuild, and registration removal. Records store only target identity,
  observed steps, relevant Git identifiers, recovery state, and redacted error
  detail; they never store Markdown bodies or draft content.
- Reconciliation before retry or a new conflicting action. Actual canonical
  configuration, files, refs, worktrees, commit OIDs, and cache state win over
  stale records. A completed commit followed by failed refresh resumes indexing
  only; it never creates a second commit.
- Caller-supplied expected observations for owned Markdown paths on edits and
  moves. A changed observation returns an external-change outcome, retains the
  caller's draft, and leaves on-disk Markdown untouched. Creation and comment
  submission still require their destination paths to be absent.
- Automatic discovery refresh after a successful initialization,
  publication-remote configuration, context provisioning, or checkpoint. The
  same operation reports `index pending` when its authoritative local change
  completed but a stable refresh cannot complete.
- Full Wave 1 recovery and compatibility evidence against disposable real
  repositories, including separate processes that contend for a lease.

## Out of Scope

- Remote contact, SSH credentials, fetch, push, synchronization, polling,
  merge, rebase, promotion, ticket closure, remote-context materialization, or
  branch and worktree cleanup.
- Desktop progress, CLI commands or JSON output, agent protocols, daemon
  lifecycle, filesystem watching, automatic retry scheduling, or cancellation
  UX. Future front ends and a daemon consume the typed domain outcomes.
- A multiple-context choice result. Duplicate or mismatched deterministic
  local contexts remain visible recovery problems; Wave 2 retains the one
  shared-context rule while adding remote materialization and merge recovery.
- Automatic merge, last-writer-wins behavior, or automatic overwrite of
  externally edited Markdown. Direct filesystem authors do not acquire a
  Manyhands lease and remain external concurrent editors.
- Persistent drafts, opaque draft fingerprints, private Markdown bodies,
  credentials, private keys, passphrases, or process-owner details in SQLite,
  diagnostics, or typed outcomes.
- Replacing Cycle 04 scan-change detection. The lease coordinates cooperating
  Manyhands callers; read-only scans still detect direct filesystem changes and
  retain prior cache rows when they cannot observe a stable context.

## Planned Implementation Changes

| Path | Change |
| --- | --- |
| `Cargo.toml` | Add a production cross-platform advisory-file-lock dependency with Windows, macOS, and Linux support. |
| `Cargo.lock` | Record the resolved lock dependency. |
| `src/repository.rs` | Extend the headless public service with operation IDs, expected path observations, recovery inspection, lifecycle orchestration, and typed busy, external-change, operation-mismatch, recovery-required, and index-pending outcomes. |
| `src/repository/coordination.rs` | Add private repository-common-Git-directory lease acquisition and the application-data cache-recovery guard. |
| `src/repository/recovery.rs` | Add private operation-record migration, step advancement after observation, reconciliation, and redacted recovery mapping for every Wave 1 lifecycle action. |
| `src/repository/discovery.rs` | Integrate cache guard use, stable-scan persistence under the repository lease, and the post-authoritative-change refresh handoff. |
| `tests/support/mod.rs` | Add disposable cross-process lease holders, controlled termination, stable operation IDs, expected-observation drafts, cache-guard contention, and all lifecycle failure seams. |
| `tests/recovery_foundation_gate.rs` | Add focused real-repository integration coverage for lifecycle reconciliation, process coordination, direct external edits, cache recovery, and the Wave 1 acceptance matrix. |
| `docs/RFC/git-workflow-and-conflict-recovery.md` | Apply the required multiple-context acceptance amendment. |
| `docs/RFC/repository-index-persistence-and-refresh.md` | Apply the required lease, cache-recovery, and all-action operation-record amendments. |
| `docs/RFC/test-and-compatibility-strategy.md` | Apply the required multiple-context exit-gate amendment. |

The public API remains synchronous, headless shared domain logic. Git
repositories, temporary indexes, lock files, SQLite connections, statements,
and transactions are opened only for the operation that uses them. No handle is
retained by a caller or future desktop model.

## Coordination Contract

### Repository Lease

The lease file is a persistent local implementation artifact under the
repository's resolved common Git directory, not the worktree-local `.git` file
or tracked repository content. An operation creates it when absent and never
unlinks it. Each eligible action opens that exact file and acquires an exclusive
advisory lock. This gives the primary worktree and linked item worktrees one
shared coordination point and releases the lock automatically when a process
terminates.

The service waits using a monotonic bounded loop for no more than 250
milliseconds. It never reports the holder's process identity, command, draft,
or content. On expiry it returns the recoverable busy outcome; callers decide
when to retry. A process holding a lease must not wait for user input, perform
network work, retain the lease while a caller edits a draft, or scan an entire
repository.

An eligible action follows this sequence:

1. Validate caller input and prepare in-memory work outside the lease.
2. Acquire the repository lease and create or reconcile the operation record.
3. Re-open the repository state and re-observe every mutation precondition.
4. Perform short filesystem, Git, and SQLite state transitions, advancing the
   record only after each external effect is observed.
5. Release the lease once the authoritative local change or short SQLite
   action is durable.

Stored discovery snapshots are read-only cache reads and do not need the lease.
They may expose the preceding stable snapshot while a later refresh is running.

### Cache-Recovery Guard

The application-data database is shared across registered repositories. Normal
SQLite open-and-transaction work takes a shared cache-recovery guard only for
the duration of that database work. A process that detects structural
corruption enters degraded mode and allows explicit-root rebuild only.

The rebuild first acquires the guard's exclusive form, waits for active SQLite
transactions to leave, preserves the corrupt database and available WAL/SHM
sidecars under diagnostic names, and installs a fresh database. It releases the
exclusive guard before taking a repository lease or scanning Git and Markdown.
This ordering avoids a cross-repository database-replacement race without
turning ordinary scans or Git actions into a global critical section.

### External Markdown Authors

The lease is advisory and only coordinates cooperating Manyhands callers. An
agent or user may directly author canonical Markdown in a worktree at any time;
Manyhands treats that change as external state rather than attempting to lock
or reject the editor.

For an update or move, the caller supplies an expected observation for every
owned source or destination path. After acquiring the lease, the service
compares the current observation before writing. A mismatch returns
external-change with the repository, item, context, and paths; it writes,
stages, commits, and refreshes nothing for that request. The caller keeps its
draft and can reload or otherwise reconcile it outside this Cycle.

Unrelated direct changes remain untouched and do not block a scoped checkpoint.
A direct change while discovery scans is handled by Cycle 04's start/end
observations: prior rows remain, a retry-required problem is visible, and the
operation remains index pending until a stable refresh succeeds.

## Durable Recovery Contract

### Operation Identity and Records

Every lifecycle request includes a caller-generated opaque operation ID. The
desktop retains it for the submitted action, and a future CLI or agent retains
it across a retry. The service records the ID before the first external state
transition. It stores no draft body, request body, or content-derived
fingerprint; a retry supplies its draft again when it is needed.

At minimum, a record identifies the canonical repository root, operation ID,
action, item and expected context when applicable, observed step sequence,
relevant commit OIDs, index state, recovery state, and redacted failure detail.
It advances only after the corresponding file, ref, worktree, commit, or SQLite
state has been observed.

A retry with the same ID first compares the record with real state:

- A completed initialization, publication-remote configuration, or checkpoint
  commit remains authoritative. Retry continues with registration or discovery
  only; it never creates another commit.
- A created deterministic branch or worktree remains available for the exact
  original request. No retry creates a second context or cleans up the partial
  resource.
- A conforming owned-path write that still equals the retry draft may proceed to
  its uncompleted checkpoint. A different on-disk path is an external-change or
  recovery-required outcome, never a guessed overwrite or commit.
- A completed cache replacement, refresh, or rebuild is verified from the
  current database and canonical scan state. Cache rows never prove that Git or
  Markdown changed.

If a caller loses its operation ID, recovery inspection exposes each pending
record's ID, target, completed steps, and safe next action. A new conflicting
request cannot silently replace it. The caller must replay the pending ID with
the required draft or receive recovery-required guidance.

### Lifecycle Refresh Handoff

Enablement, publication-remote configuration, context provisioning, and each
successful item or comment checkpoint use one operation record through their
discovery handoff. After an authoritative local transition, the action records
the commit OID or observed context state, releases its short mutation lease,
and begins refresh.

Refresh records stable start observations under a brief lease, performs the
full read-only scan outside that lease, then reacquires it to compare end
observations and transactionally replace rows. A changed context retains its
prior rows and produces retry-required instead of writing a mixed snapshot.

The operation is complete only after the resulting refresh persists. A failed
or unstable refresh returns index-pending with the authoritative completed
step. Retrying the same ID reruns only refresh/rebuild work and cannot repeat an
initialization, configuration, context, write, or checkpoint step. Registration
removal is local SQLite deletion only and completes without a source scan.

## Recovery Considerations

- Failure before an external effect advances no completion step and preserves
  the caller draft and pre-existing state.
- A failure after configuration write but before its commit follows Cycle 02's
  bounded restoration rule for state created by that operation; interruption
  leaves the observed state and recovery record for exact reconciliation.
- A failure after context-branch creation or worktree creation preserves the
  deterministic partial resource. No automatic path deletes a recoverable
  branch, worktree, file, commit, or user change.
- A failure after a canonical write preserves the valid file. A retry commits it
  only when the replayed operation and supplied draft prove it remains the
  intended owned-path state.
- A failure after a checkpoint or configuration commit preserves its commit OID
  and reports index-pending. No outcome retries that Git commit.
- Database corruption preserves diagnostic cache files when possible. Until
  explicit-root rebuild succeeds, normal registry and lifecycle calls report
  index unavailable and cannot begin a Git-mutating action.
- Lease contention, inaccessible paths, unexpected context state, stale
  observations, and scan races remain typed recoverable outcomes that identify
  the repository and affected item/context/path when applicable.
- No automatic recovery stashes, resets, discards, overwrites, merges, rebases,
  publishes, removes a branch, or removes a worktree.

## Test and Fixture Plan

`tests/recovery_foundation_gate.rs` MUST cover:

- Existing and unborn enablement, publication-remote configuration, deterministic
  context creation, document/ticket/comment checkpoints, automatic refresh,
  explicit-root rebuild, and registration removal as one real-repository Wave 1
  lifecycle.
- Required injected failures at configuration write, initialization commit,
  context branch creation, worktree creation, item Markdown write, checkpoint
  commit, SQLite transaction, refresh, and corrupt-cache replacement. Each case
  asserts the actual retained file, configuration, commit OID, branch, worktree,
  cache state, and operation record before proving exact retry behavior.
- Reopening the service after every named partial state. Reconciliation must
  perform only the uncompleted step and create no duplicate item, comment,
  initialization/configuration/checkpoint commit, branch, worktree, or cache
  replacement.
- A primary-worktree process and an item-worktree process contending for the
  same common-Git-directory lease. The waiting process receives bounded busy;
  it succeeds after release or controlled holder termination. Operations on
  distinct repository roots do not contend.
- A child process that terminates while holding the lease, proving the next
  process can acquire it and reconcile from durable state without removing the
  lock file.
- A direct Markdown change between load and save for documents and moves,
  proving external-change preserves the draft and on-disk bytes without a new
  commit. Direct unrelated-path changes remain outside a scoped checkpoint.
- A direct change during refresh, proving start/end observations retain prior
  rows, expose retry-required, leave index pending, and converge only after a
  stable retry.
- Caller-supplied operation-ID replay, mismatch rejection, and lost-ID recovery
  inspection. The test proves SQLite contains no Markdown draft body or
  content-derived fingerprint.
- Shared cache transactions from separate repository operations and an
  exclusive corrupt-cache replacement, proving the replacement does not race a
  transaction or allow a stale database handle to become authoritative.
- No remote transport, credential callback, global Git configuration,
  developer repository, application data, or system Git executable for any
  fixture or test.

Fixtures use `git2`, temporary directories, isolated application-data paths,
and test-only child processes. They configure local identities in fixture
repositories, clean up on failure, and redact private Markdown bodies and all
credential-like values from assertions and diagnostics.

## Verification

During implementation, run the focused recovery target after each completed
behavior slice:

```sh
devenv shell -- cargo test --locked --test recovery_foundation_gate
```

Run affected existing integration targets while changing their contracts:

```sh
devenv shell -- cargo test --locked --test repository_enablement
devenv shell -- cargo test --locked --test local_authoring
devenv shell -- cargo test --locked --test discovery_rebuild
```

Before declaring Cycle 05 complete, run:

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
devenv shell -- cargo run --locked --bin manyhands-cli
```

No desktop smoke test is required because this Cycle introduces no desktop
code. The CLI skeleton smoke test remains required by `AGENTS.md` even though
Cycle 05 adds no CLI command surface.

## Exit Criteria

Cycle 05 is complete when:

- Every eligible Wave 1 repository operation takes the common-Git-directory
  lease only for its short atomic phase, returns typed bounded busy on
  contention, and releases correctly after normal or terminated processes.
- Independent repositories remain concurrent, while primary and linked
  worktree actions for one repository never produce duplicate or interleaved
  lifecycle artifacts.
- The shared SQLite cache is guarded only during database work; corrupt-cache
  replacement coordinates safely across repositories, preserves diagnostics
  when possible, and leaves Git and canonical Markdown unchanged.
- Every Wave 1 lifecycle action has a caller-supplied operation ID and durable
  recovery record that advances after observed state. Reconciliation treats Git
  and canonical Markdown as authoritative and never stores draft content.
- Enablement, publication-remote configuration, context provisioning, and
  checkpoints automatically refresh discovery. A failed refresh produces
  index-pending, and retry performs index work only.
- Direct Markdown authors can work alongside desktop and CLI callers. Stale
  owned-path saves are rejected without overwrite or auto-merge; unrelated
  changes remain preserved; unstable scans retain prior discovery state and
  visibly require retry.
- Every named failure and process-interruption boundary preserves recoverable
  state and retries without duplicate canonical content, comments, commits,
  branches, worktrees, registrations, or cache artifacts.
- The narrow source amendments are approved and all focused, full Rust, and CLI
  skeleton verification commands pass.

## Risks and Controls

| Risk | Control and required evidence |
| --- | --- |
| Advisory locks cannot control a direct Markdown editor | Treat direct edits as external state; require expected observations before writes and start/end observations for scans. |
| A slow scan makes desktop and agent work wait too long | Scan outside the repository lease, compare observations before persistence, and return retry-required rather than replacing mixed rows. |
| A process crash leaves a lock file or partial resource | Keep the lock file but rely on OS lock release; preserve branches, worktrees, files, and records for reconciliation. |
| SQLite corruption races unrelated repository work | Use the narrow shared/exclusive cache-recovery guard and release exclusive mode before repository scanning. |
| A caller loses an operation ID or retries it with different intent | Expose recovery inspection, reject mismatched reuse, and never infer a replacement draft from target state. |
| Platform locking semantics differ | Use a cross-platform locking implementation and cover contention and termination in the native test suite; document deviations as compatibility cases. |
| Recovery scope expands into remote lifecycle behavior | Keep all remote, daemon, merge, cleanup, and UI work out of scope; record only compatible local recovery contracts. |

## Handoff

Wave 01 completes only after this Cycle's source amendments and full acceptance
matrix pass. Wave 2 reuses the same short repository lease for remote polling,
synchronization, merge recovery, and remote context materialization, but it
must add its own transport, authentication, cancellation, and
multiple-context-selection protocol. It must not weaken Cycle 05's canonical
authority, stale-write protection, or no-duplicate recovery guarantees.
