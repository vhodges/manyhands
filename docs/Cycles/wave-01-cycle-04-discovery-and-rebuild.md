---
title: "Wave 01 Cycle 04: Discovery and Rebuild"
date: 2026-10-02
status: approved
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01K7C4D6F8H1J3M5P7R9T2V4X6"
---

# Wave 01 Cycle 04: Discovery and Rebuild

## Parent Wave

This is Cycle 04 of [Wave 01: Foundations](../Waves/wave-01-foundations.md).
It makes canonical repository state discoverable through a rebuildable local
SQLite cache after Cycle 03 establishes deterministic local authoring contexts
and scoped checkpoints.

## Purpose

Implement headless, read-only discovery for registered Manyhands repositories.
A caller can refresh one repository, rebuild its local discovery state after
cache loss or corruption, and read one deterministic snapshot of its observed
contexts, items, comment threads, activity, and recoverable problems.

Markdown, tracked configuration, and Git remain authoritative. SQLite stores
only rebuildable metadata and recovery information for refresh and rebuild
actions. No Cycle 04 operation writes canonical Markdown, changes Git state,
contacts a remote, or turns an invalid context into an editable one.

## Prerequisites

- Cycles 01 through 03 exit criteria remain satisfied, including canonical
  validation, enabled-repository registration, deterministic local contexts,
  scoped checkpoints, and the `refresh_required` registration flag.
- Wave 01's entry gate remains satisfied.
- The approved canonical schema, Git workflow, repository/index, and test
  strategy RFCs remain unchanged or receive an approved amendment.
- Implementation starts from a clean `main` branch.

## RFC and PRD Traceability

| Source | Cycle responsibility |
| --- | --- |
| [Canonical content and comment schema RFC](../RFC/canonical-content-and-comment-schema.md) | Scan canonical paths without rewriting them; preserve visibility of marker-only, malformed, duplicate, invalid-path, and invalid-thread content. |
| [Git workflow and conflict recovery RFC](../RFC/git-workflow-and-conflict-recovery.md) | Observe primary and deterministic local contexts through `git2`; preserve Cycle 03's checkpoint-and-index-pending boundary without recreating a commit. |
| [Repository index persistence and refresh RFC](../RFC/repository-index-persistence-and-refresh.md) | Migrate the local registry into a rebuildable context, item, problem, and refresh/rebuild-operation cache; perform full scans and explicit-root rebuild. |
| [Test and compatibility strategy RFC](../RFC/test-and-compatibility-strategy.md) | Prove real-repository discovery, SQLite recovery, scan races, and no-mutation behavior with disposable fixtures and named failure injection. |
| `MH-CONTENT-001` and `MH-CONTENT-004` | Discover primary and active-context documents and tickets with metadata, activity, and active-context precedence. |
| `MH-CONTENT-003`, `MH-COMMENT-001`, and `MH-COMMENT-002` | Surface nonconforming tickets and reconstruct deterministic root-comment and reply relationships from canonical sources. |
| `MH-INDEX-001` to `MH-INDEX-003` | Maintain a rebuildable, non-authoritative discovery cache; manually refresh it; and expose external, malformed, inaccessible, and stale state as problems. |
| `MH-COLLAB-002`, `MH-NFR-006`, `MH-NFR-007`, and `MH-NFR-008` | Reconcile a checkpoint whose index refresh is pending without duplicate Git work; preserve canonical state and identify incomplete index work. |

## In Scope

- A small, synchronous, headless discovery extension to the shared repository
  domain API. It has no GPUI, GPUI Kit, desktop, CLI, SSH, transport, or network
  dependency.
- A private discovery module beneath `src/repository.rs` for SQLite migration,
  canonical scanning, transactional replacement, corruption recovery, and
  snapshot mapping. `RepositoryService` remains the only public service.
- Public operations to refresh one registered repository, rebuild one explicitly
  supplied repository root, and read its stored discovery snapshot without
  scanning.
- A deterministic metadata-only repository snapshot containing configuration
  observation, contexts, documents and tickets, ordered comment relationships,
  activity source and time, problems with recovery guidance, and active-context
  precedence. Markdown bodies and private data remain in canonical files and
  never enter SQLite or the snapshot.
- SQLite migration from the existing `repositories` registry to private context,
  item/comment-metadata, problem, and operation tables with foreign-key cleanup
  when a registration is removed. The migration makes the observed configuration
  blob OID nullable so an explicit-root rebuild can register a missing or invalid
  configuration without inventing an identity.
- Full read-only scans of the registered root and recognized local item
  worktrees. Each context has distinct rows even when it contains the same item
  ULID as another context.
- Root scanning when configuration is missing, malformed, unsupported, or when
  the checked-out branch differs from the configured primary branch. The result
  is an `unverified` root context with a visible configuration or branch problem.
- Active-context discovery only when configuration is valid. A recognized active
  context uses the exact Wave 1 branch and worktree convention and contains its
  branch-identified conforming item. Every such context is fully scanned for
  valid content, comments, uncommitted activity, and malformed content.
- Visible context problems for convention-matching worktrees whose identified
  item is missing or invalid. They are never editable contexts and never receive
  active-context precedence.
- Canonical-path scanning plus marker-prefiltered discovery of managed-looking
  Markdown outside canonical paths. Ordinary Markdown remains ignored. The scan
  excludes Git metadata and never recursively treats `.manyhands/worktrees/` in
  the root as canonical content.
- Activity extraction that uses the newest commit touching an item Markdown path
  or one of its valid comments, overridden by a newer uncommitted filesystem
  observation with its source recorded.
- Explicit-root rebuild recovery. A caller supplies a known repository root;
  rebuild reconstructs that root's local registration and discovery state. A
  deleted database does not silently search the filesystem for formerly
  registered roots or introduce a second persistent registry.
- Preservation of a corrupt database under a timestamped diagnostic name, with
  its WAL and SHM sidecars when present, before a fresh database is created when
  possible.
- A degraded `RepositoryService` when SQLite is structurally corrupt. The
  existing open operation retains the data path and permits explicit-root rebuild
  only; normal registry and discovery calls return a typed index-unavailable
  outcome until rebuild replaces the cache.
- Durable operation records for refresh and rebuild actions only. A legacy
  `refresh_required` flag left by Cycle 02 or 03 starts a fresh indexing action
  on retry; it never causes configuration, context, write, or checkpoint work to
  run again.
- Scan-change detection. If a context HEAD or relevant file observation changes
  during its scan, its prior rows remain, a retry problem is visible,
  `refresh_required` remains set, and the operation returns a typed retry-required
  result.

## Out of Scope

- Remote contact, SSH credentials, fetch, push, synchronization, polling,
  remote-context materialization, merge, rebase, promotion, ticket closure, and
  branch or worktree cleanup.
- Filesystem watching, automatic refresh scheduling, background services, or
  desktop and CLI discovery interfaces. Callers request refresh or rebuild
  explicitly in this Cycle.
- Writing, moving, repairing, migrating, staging, checkpointing, or otherwise
  changing canonical Markdown, tracked configuration, the live Git index,
  refs, worktrees, remotes, or Git configuration.
- Persistent Markdown bodies, private keys, credentials, passphrases, or other
  secret data in SQLite or snapshot results.
- Durable operation records for repository enablement, context provisioning,
  item writing, checkpointing, synchronization, promotion, closure, or cleanup.
  Cycle 03 retains immediate Git-state retry behavior; Cycle 05 owns broader
  lifecycle reconciliation evidence and the cross-process repository lease.
- Remote context discovery and shared-context materialization. Wave 2 owns that
  protocol; it does not introduce a multiple-editable-context choice result.
- Automatic global restoration of all prior registrations after complete SQLite
  loss. A caller must provide each known root to explicit-root rebuild.

## Planned Implementation Changes

| Path | Change |
| --- | --- |
| `src/repository.rs` | Add the small public refresh, rebuild, snapshot, degraded-service outcome, and typed error contract; declare the private discovery module; preserve the existing headless service boundary. |
| `src/repository/discovery.rs` | Add schema migration, repository and worktree observation, read-only canonical scans, activity derivation, transactional cache replacement, refresh/rebuild operation recovery, corruption handling, and snapshot mapping. |
| `tests/support/mod.rs` | Extend disposable `git2` fixtures with primary/context content, controlled commit and filesystem activity, invalid configurations and contexts, malformed/out-of-path markers, SQLite corruption, and scan-change injection. |
| `tests/discovery_rebuild.rs` | Add focused real-repository and SQLite integration coverage for discovery, rebuild, recovery, and no-mutation boundaries. |

No new runtime dependency is required. Public APIs remain synchronous shared
domain logic. Git handles, SQLite connections, statements, transactions, and
filesystem directory iterators are opened only for the operation using them and
are never retained by callers.

## Discovery Contract

### Snapshot Shape

A snapshot is repository-scoped and deterministic. It includes:

- The canonical root, last observed configuration state, and whether refresh is
  still required.
- Distinct primary, unverified-root, and active item-context observations with
  branch, worktree path, item identity when applicable, observed HEAD, and state.
- Documents and tickets with canonical path, ULID, title, ticket type/status,
  closure state, optional project/team, and activity source/time.
- Comment IDs, canonical paths, parent/item relationships, creation times, and
  deterministic root/reply ordering. Comment bodies are read from canonical
  Markdown by a future item-reading API and are not cached here.
- Stable problem codes, affected repository/context/path, observed time, and
  human-readable recovery guidance.

The snapshot returns active-context content instead of the primary copy when
exactly one valid deterministic local context contains the same document or
ticket. Primary and active rows remain separately queryable in the snapshot.
Cycle 04 never returns a multiple-context choice state.

### Context And Path Observation

Refresh always reads the registered root. It labels that root `primary` only
when valid configuration names its checked-out branch; otherwise it labels the
root `unverified`, records the configuration or wrong-primary-branch problem,
and still indexes readable root content.

With valid configuration, refresh enumerates the repository-local worktrees at
`.manyhands/worktrees/`. A worktree is an active context only when its path,
checked-out branch, branch kind and ULID, and conforming branch-identified item
match the Wave 1 convention. The full canonical scan of every active context
includes its valid documents, tickets, comments, uncommitted changes, and
nonconforming sources. A convention-matching candidate whose identified item is
missing or invalid is recorded as a context problem only; it is not active,
editable, or eligible for precedence.

For every scanned context, the walker reads canonical paths directly. It also
examines Markdown outside those paths, excluding Git metadata and the root's
local worktree base. Such a file is a problem candidate only when its leading
YAML front matter declares `manyhands_managed: true`; unrelated repository
Markdown is not indexed or reported.

### Persistence And Activity

The existing application-local database remains `manyhands.sqlite3` in the
configured data directory. Migration enables foreign keys, WAL, and the bounded
busy timeout already required by the repository/index RFC. It keeps the unique
canonical-root registration, makes the observed configuration blob OID nullable
when no valid configuration is observable, and adds private rows for contexts,
item and comment metadata, problems, and refresh/rebuild operations.

Each successful context scan replaces only that context's cached item and
problem rows in one transaction. Disappeared contexts and their dependent rows
are removed when a later full repository observation no longer exposes them.
The registration's `refresh_required` flag clears only after the complete
repository observation is persisted. Removing a registration cascades through
all of its derived data and never alters the repository.

For committed content, activity is the latest commit that touches an item's
Markdown or a valid comment for that item. For a newer owned-path modification,
the snapshot records the filesystem observation time and marks it as
uncommitted. This metadata never makes a filesystem timestamp canonical content.

## Refresh, Rebuild, And Recovery Contract

Refresh and rebuild are read-only with respect to the repository. Each durable
operation record advances only after the relevant read or SQLite persistence
step is observed. A successful operation reports the updated snapshot.

`refresh_repository` requires an existing local registration. It performs a
full root-and-context observation. An inaccessible source becomes a visible
problem while successfully observed contexts remain available.

`rebuild_repository` takes an explicit root supplied by the caller. The caller
therefore supplies the target when the database is absent or corrupt, because a
fresh database cannot discover former roots on its own. Rebuild reconstructs
that root's registration and derived rows from observed Git, configuration, and
Markdown state. It may record an invalid configuration as a problem because the
caller has explicitly identified the root; it never searches arbitrary paths.

When `RepositoryService::open_at` detects a structurally corrupt database, it
returns a degraded service that retains the data path. Until explicit-root
rebuild succeeds, only `rebuild_repository` is available; normal registry and
discovery calls return a typed index-unavailable outcome. Rebuild preserves the
corrupt database for diagnostics under a timestamped name, including WAL/SHM
sidecars when present and possible, creates a fresh database, and rebuilds the
supplied root. No cached row is treated as proof that Git or Markdown changed.

If a context changes while it is scanned, comparison of observed HEAD and
relevant file observations detects it. The operation retains that context's
previous rows, records a retry-required problem, leaves `refresh_required` set,
and returns a typed retry-required outcome. Cycle 03 callers continue to
serialize authoring and refresh in process. Cycle 05 supplies mandatory
cross-process serialization.

An existing `refresh_required` flag, including one set after a Cycle 03
checkpoint whose invalidation completed late, causes refresh to create or resume
only an index operation. It does not create a duplicate item, comment, branch,
worktree, or Git commit.

## Test And Fixture Plan

`tests/discovery_rebuild.rs` MUST cover:

- A valid primary root and one deterministic active document and ticket context,
  each with distinct stored rows; active-context precedence; and deterministic
  snapshot ordering.
- Root comments and nested replies reconstructed in ascending `created_at`, then
  ULID order, without caching comment bodies.
- Committed item/comment activity and a newer uncommitted filesystem observation
  with its source recorded.
- A missing, malformed, or unsupported configuration that yields an unverified
  root scan and visible problem without discovering active contexts; and a valid
  configuration with a wrong-primary root branch that yields an unverified root
  while still discovering valid active contexts.
- A valid configuration with active-context worktrees that contain valid,
  malformed, marker-only, duplicate-ID, invalid-thread, inaccessible, and
  managed-looking out-of-path sources. Ordinary Markdown must remain absent from
  discovery problems.
- A convention-matching worktree whose branch-identified item is missing or
  invalid, proving that it becomes a visible context problem without active
  precedence.
- Exact removal of stale item, problem, and context rows after an observed
  canonical deletion or worktree disappearance, and registration removal that
  cascades only local SQLite data.
- Read-only refresh and rebuild assertions: Markdown bytes, configuration,
  commit graph, refs, worktrees, remotes, live indexes, and statuses remain
  unchanged.
- Rebuild of a healthy cache and explicit-root recovery after a missing or
  corrupt database. The corrupt case proves that service opening yields a
  rebuild-only degraded service, normal APIs report index unavailable, and
  explicit rebuild restores them. The test verifies diagnostic preservation when
  the filesystem permits it and does not expect a fresh database to find unknown
  roots.
- A Cycle 03 checkpoint with a pending index refresh, proving retry refreshes
  discovery without another checkpoint commit.
- Named failures after context observation, before transaction commit, and while
  replacing a corrupt database. Each case proves the actual retained cache and
  canonical state, then proves retry indexes only incomplete work.
- A context modified during scanning, proving that prior rows remain,
  `refresh_required` stays set, a retry problem is visible, and a later stable
  refresh converges without Git mutation.

Fixtures use `git2`, temporary directories, and isolated application-data paths.
They configure fixture-local Git identities and never read developer
repositories, global Git configuration, application data, SSH keys, or a system
Git executable.

## Verification

During implementation, run the focused target after each completed behavior
slice:

```sh
devenv shell -- cargo test --locked --test discovery_rebuild
```

Before declaring the Cycle complete, run:

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
```

No desktop or CLI smoke test is required because this Cycle introduces neither
desktop nor CLI behavior.

## Exit Criteria

Cycle 04 is complete when:

- Headless shared code refreshes a registered repository and returns a
  deterministic metadata-only discovery snapshot of its root and recognized
  active local contexts.
- A valid deterministic local context receives discovery precedence over its
  primary copy while preserving distinct context rows; malformed candidates stay
  visible without becoming active or editable.
- Missing, malformed, duplicate, invalid-thread, inaccessible, marker-only, and
  managed-looking out-of-path content remain visible as actionable problems and
  are never rewritten or silently hidden.
- Refresh and explicit-root rebuild do not modify Markdown, configuration, Git
  commits, refs, branches, worktrees, remotes, Git indexes, or unrelated files.
- Rebuild from a missing or corrupt database recreates equivalent discovery for
  the explicitly supplied root, preserving diagnostic cache files when possible
  and without adding a hidden root registry. Structural corruption opens a
  rebuild-only service and normal operations report that the index is
  unavailable until this recovery succeeds.
- A pending Cycle 03 index refresh and every named refresh/rebuild failure retry
  only incomplete index work without duplicating any canonical or Git artifact.
- A context that changes during scan leaves its prior cache state intact, exposes
  a retry-required problem, and converges after a stable retry.
- The focused target and full required Rust verification suite pass.

## Handoff

Cycle 05 consumes the discovery snapshot, refresh/rebuild operation records,
and current `refresh_required` state to add the repository-scoped cross-process
lease and prove recovery across the Wave 1 lifecycle. It MUST continue to treat
Git and canonical Markdown as authoritative over SQLite.

Wave 2 extends this model with remote polling, recognized remote-context
materialization, and shared-branch merge recovery. It MUST not reinterpret an
unverified or malformed local candidate as an editable context without an
approved protocol amendment.
