---
title: "Wave 01 Cycle 05 Recovery and Foundation Gate Design"
date: 2026-10-02
status: draft
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01K7F1H3J5M7P9R2T4V6X8Z0B2"
---

# Wave 01 Cycle 05 Recovery and Foundation Gate Design

## Purpose

This design turns the approved [Cycle 05 contract](../Cycles/wave-01-cycle-05-recovery-and-foundation-gate.md)
into an implementable extension of the Cycle 02, 03, and 04 repository service.
It preserves actual Git and canonical Markdown as authoritative while adding
short cross-process coordination, caller-owned retry identity, stale-write
protection, and automatic discovery handoff.

The implementation remains synchronous, headless shared domain logic. It does
not add a CLI command, desktop interaction, daemon, transport, filesystem
watcher, automatic merge, or automatic cleanup.

## Authorities

- `AGENTS.md`
- [Wave 01: Foundations](../Waves/wave-01-foundations.md)
- [Cycle 05: Recovery and Foundation Gate](../Cycles/wave-01-cycle-05-recovery-and-foundation-gate.md)
- [Git workflow and conflict recovery RFC](../RFC/git-workflow-and-conflict-recovery.md)
- [Repository index persistence and refresh RFC](../RFC/repository-index-persistence-and-refresh.md)
- [Test and compatibility strategy RFC](../RFC/test-and-compatibility-strategy.md)

## Confirmed Decisions

- Every Wave 1 mutation receives a lease and caller-supplied operation ID:
  create-and-enable, enable, remote add/remove, publication-remote
  configuration, context provisioning, document/ticket/comment changes,
  refresh, rebuild, and registration removal. Inspection, remote listing, and
  stored snapshot reads do not.
- `OperationId` is a new canonical uppercase ULID type distinct from
  `canonical::ItemId`. A record is unique within its root and is retained until
  registration removal or cache rebuild. A retried ID must name the same root,
  action, and target.
- Records are keyed by canonical or validated intended root, with an optional
  repository relation. This permits creation before registration. Registration
  removal clears every record for its root; a retry infers that the intended
  local absence is already complete.
- Incomplete Cycle 04 refresh/rebuild records migrate as legacy records. The
  first matching refresh or rebuild resumes them safely without inventing a
  caller ULID. New actions always require a caller `OperationId`.
- A document edit or move carries an ephemeral expected path observation:
  `Missing` or the BLAKE3 digest of exact bytes. It is never stored in SQLite.
  A mismatch is an external-change outcome before a write, stage, commit, or
  refresh. New destinations and comments require `Missing`.
- A repository lease is an exclusive advisory lock on the persistent
  `<common-git-dir>/manyhands-operation.lock` file. It has a fixed 250 ms wait,
  is never unlinked, and protects only the action's brief atomic phase.
- `create_and_enable` first takes a persistent bootstrap lease in application
  data, keyed by the validated intended root. It holds that lease until it has
  created the repository and acquired its common-Git lease, then releases it.
- A persistent `manyhands.sqlite3.recovery.lock` next to the database protects
  SQLite work. Normal open/transaction work takes a shared lock. Structural
  corruption replacement takes the exclusive lock only while preserving and
  replacing the database; it releases it before repository scanning.
- Structural SQLite corruption is the only service-global degraded state.
  Incomplete normal actions, including rebuilds, block only their own root.
- An authoritative initialization, remote mutation, context provisioning, or
  checkpoint starts the same operation's refresh phase. It is complete only
  after stable discovery persistence. A failed or unstable refresh is
  `index-pending`; replaying its ID resumes index work only.

## Architecture

`src/repository.rs` remains the public boundary and orchestration layer. It
adds request operation IDs, expected observations, typed recovery outcomes, and
recovery inspection while retaining the existing service and error style.

`src/repository/coordination.rs` owns private file-lock acquisition. It exposes
repository leases, creation bootstrap leases, and cache guards without exposing
file handles to callers. It uses `fs4` for Windows, macOS, and Linux advisory
locking and BLAKE3 only for request-local external-write observations and a
safe bootstrap filename key.

`src/repository/recovery.rs` owns private operation-record migration and the
start/reconcile/advance helpers. It stores observed effects, never Markdown or
request bytes. `src/repository/discovery.rs` continues to own cache migration,
observations, and snapshot persistence; it gains cache-guarded database access
and Cycle 05's short-lease scan protocol.

## Public Contract

Add the following public concepts under `repository`:

```rust
pub struct OperationId(/* private canonical ULID */);

pub enum ExpectedPathObservation {
    Missing,
    Blake3([u8; 32]),
}

pub enum RecoveryInspection {
    Pending {
        operation_id: OperationId,
        operation: RepositoryOperation,
        root: PathBuf,
        item_id: Option<canonical::ItemId>,
        context: Option<PathBuf>,
        completed_step: String,
        next_action: String,
    },
    LegacyIndexOperation {
        root: PathBuf,
        operation: RepositoryOperation,
        next_action: String,
    },
}
```

Every mutable request gains `operation_id: OperationId`. Document and ticket
updates add expected owned-path observations; a document move supplies source
and destination expectations. APIs that currently accept a root/name pair for a
mutation become a request type so their operation ID and target are explicit.

The service adds typed `RepositoryErrorKind` variants for `RepositoryBusy`,
`OperationMismatch`, `RecoveryRequired`, and `ExternalChange`. It adds an
`IndexPending` outcome that retains the commit OID or created context. Existing
`RegistrationPending` and `RefreshPending` results are migrated into this
authoritative-change-plus-index-handoff model rather than silently losing their
Git result.

`RepositoryService::recovery_inspection(root)` lists root-local unresolved
records without a draft, content digest, holder identity, credential, or other
secret. A different unresolved operation for the same root returns
`RecoveryRequired`; a same-ID request reconciles actual state first.

## Persistence Model

Create an `operation_records` table with an integer internal key and at least:

- `root_path TEXT NOT NULL`
- `repository_id INTEGER REFERENCES repositories(id) ON DELETE SET NULL`
- nullable `operation_ulid TEXT` for migrated legacy records, unique with root
  when present
- `action TEXT NOT NULL`, `target_kind TEXT`, `item_id TEXT`, `context_path TEXT`
- `state TEXT NOT NULL`, `completed_step TEXT NOT NULL`
- optional commit, branch, worktree, and refresh/rebuild observation metadata
- optional redacted error code/detail and timestamps

Use a root-path index and a partial unique index for new `(root_path,
operation_ulid)` values. Preserve an incomplete Cycle 04 `index_operations` row
as a legacy record that points to its existing context observations. Migrate
completed legacy records only when needed to preserve rows; do not fabricate a
caller ID or persist an input fingerprint. New record operations advance only
after a repository file, Git resource, commit, or SQLite result is observed.

## Coordination And Lifecycle Flow

For an existing repository, validate in-memory input first, then acquire the
common-Git lease, start or reconcile the record under a brief shared cache
guard, and re-open/re-observe state before an effect. The operation advances its
record after each observed effect and releases the repository lease as soon as
the authoritative local transition is durable.

For create-and-enable, validate the creation target and acquire its app-data
bootstrap lease. Record the intended root, initialize Git, acquire the common
Git lease while bootstrap remains held, then continue normally and release the
bootstrap lease. No reverse bootstrap-after-repository ordering is permitted.

Refresh takes the repository lease only to capture its starting observation and
record state. It releases the lease during the full Cycle 04 scan, reacquires it
to compare observations and persist a stable result, and retains prior rows on
a mismatch. Snapshot reads remain lock-free SQLite reads. A cache replacement
uses only the exclusive cache guard, then releases it before explicit-root
rebuild acquires a repository lease.

## Compatibility And Risks

- Required `OperationId` fields change every mutable request constructor.
  Fixture builders must make new IDs simple, while retries deliberately reuse
  the same one.
- Existing rollback behavior before a configuration/publication commit remains
  unchanged. Records describe observed state after interruption; they do not
  cause unsafe automatic rollback or roll-forward.
- A direct Markdown writer cannot hold the advisory lease. Exact-byte BLAKE3
  observations and Cycle 04 scan comparisons, rather than locks, protect it.
- Lock tests must use condition-based child-process readiness instead of sleeps.
- Database replacement is necessarily global for its short critical section;
  all ordinary action recovery is root-scoped so unrelated repositories remain
  usable.

## Test Strategy

Use unit tests in the new private modules for parsing, lock-path derivation, and
record state transitions. Add `tests/recovery_foundation_gate.rs` for real Git,
SQLite, and process evidence. A helper test in that same integration-test binary
can be run as a child with `--exact`, an environment-selected lock type, and a
ready-file handshake; no production binary or daemon is added.

The acceptance target proves every required failure boundary, same-root lease
contention, process-exit release, bootstrap creation contention, cache guard
replacement, stale owned-path writes, lost-ID inspection, legacy refresh/rebuild
resumption, automatic refresh, and no duplicates across the full Wave 1 local
lifecycle.
