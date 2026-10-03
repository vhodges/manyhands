---
title: "Wave 01 Cycle 05 Recovery and Foundation Gate Implementation Plan"
date: 2026-10-02
status: draft
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01K7F2H4J6M8P0R3T5V7X9Z1B3"
---

# Wave 01 Cycle 05 Recovery and Foundation Gate Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make every Wave 1 repository mutation recoverable and idempotent
across desktop, CLI, and agent processes without duplicating Git/canonical
artifacts or silently overwriting direct Markdown edits.

**Architecture:** Add private coordination and recovery modules beneath the
existing synchronous `RepositoryService`. Persistent advisory file locks provide
short repository and cache-replacement critical sections; a root-keyed SQLite
journal records observed lifecycle steps and reconciles Git/Markdown state on
replay. Cycle 04 discovery remains the scanner, but it runs outside the
repository lease and becomes the final phase of every authoritative change.

**Tech Stack:** Rust 2024, `git2`/libgit2, `rusqlite`, `fs4`, `blake3`, `ulid`,
`tempfile`, existing canonical domain code, Cargo, and Devenv/Nix.

**Commit Policy:** Do not create commits unless the user explicitly requests
one. If requested, stage only the completed task's files and use the suggested
commit message.

---

## Authorities And Fixed Decisions

Read these before implementation:

- `AGENTS.md`
- `docs/Cycles/wave-01-cycle-05-recovery-and-foundation-gate.md`
- `docs/plans/2026-10-02-wave-01-cycle-05-recovery-and-foundation-gate-design.md`
- `docs/RFC/git-workflow-and-conflict-recovery.md`
- `docs/RFC/repository-index-persistence-and-refresh.md`
- `docs/RFC/test-and-compatibility-strategy.md`
- `docs/plans/2026-10-02-wave-01-cycle-04-discovery-and-rebuild-implementation.md`

The implementation must preserve these decisions:

- Git and canonical Markdown win over SQLite records. SQLite stores no Markdown
  bodies, request bodies, content digests, credentials, keys, or passphrases.
- All mutable Wave 1 actions use caller-supplied `OperationId` ULIDs. The scope
  includes create-and-enable, enable, remote add/remove, publication
  configuration, context provisioning, saves/comments, refresh/rebuild, and
  registration removal. Inspection, remote listing, and snapshot reads do not.
- Operation records are root-keyed with an optional registration relation.
  Completed records are retained until registration removal or cache rebuild.
  A pre-Cycle-05 incomplete refresh/rebuild record resumes as legacy, without a
  fabricated caller operation ID.
- Existing repository actions use the persistent common-Git lease with a 250 ms
  bounded wait. Create-and-enable uses an app-data bootstrap lease until it can
  acquire the common-Git lease. The lease file is never deleted.
- Normal SQLite work takes a short shared cache guard. Structural corruption
  replacement takes the exclusive guard, releases it before scanning, and is
  the only service-global degraded state. Incomplete normal actions are
  root-scoped.
- Expected document/ticket move observations use request-only BLAKE3 digests of
  exact bytes or `Missing`. A mismatch preserves the caller draft and on-disk
  content, with no write, commit, or refresh.
- A successful initialization, remote mutation, context provisioning, or
  checkpoint automatically runs the same operation's stable refresh phase.
  `IndexPending` may report completed Git/canonical state but retry only index
  work.
- No task contacts a remote, runs a system Git executable, adds a daemon or
  frontend behavior, auto-merges, auto-cleans up, or overwrites external work.

## Test Conventions

- Use `tests/recovery_foundation_gate.rs` for cross-lifecycle acceptance. Keep
  focused Cycle 02, 03, and 04 tests passing as compatibility evidence.
- Use `tests/support/mod.rs` to create temporary Git repositories through
  `git2`, local identities, isolated data directories, and condition-based
  child-process handshakes. Never use developer Git configuration or data.
- Add a child-only `#[test]` in `recovery_foundation_gate.rs`. Its parent spawns
  `current_exe()` with `--exact` and environment-selected inputs, waits for an
  atomically created ready file, and then asserts busy/release behavior. The
  child must exit promptly when its release file exists; tests must not depend
  on arbitrary sleeps.
- For each injected failure, capture `repository_and_worktree_snapshot` before
  retry, assert the exact retained state, reopen a fresh service, replay the
  appropriate request, and assert only the unfinished step occurs.

### Task 1: Add Dependencies, Identity Types, And Request Skeletons

**Files:**
- Modify: `Cargo.toml:13-23`
- Modify: `Cargo.lock`
- Modify: `src/repository.rs:1-25, 77-200, 359-425`
- Modify: `tests/support/mod.rs:10-15, 242-320`
- Create: `tests/recovery_foundation_gate.rs`
- Modify: `tests/repository_enablement.rs`
- Modify: `tests/local_authoring.rs`
- Modify: `tests/discovery_rebuild.rs`

**Step 1: Write failing public-contract tests**

Create `tests/recovery_foundation_gate.rs` with `mod support;`. Add compile-time
and behavior tests for canonical operation IDs, request-local observations, and
the new error/outcome variants. Add fixture helpers that return a new ID by
default and allow an explicit ID for a retry.

```rust
#[test]
fn operation_ids_are_canonical_ulids_and_retries_reuse_one_id() {
    let id = support::operation_id("01K7F2H4J6M8P0R3T5V7X9Z1B3");

    assert_eq!(id.to_string(), "01K7F2H4J6M8P0R3T5V7X9Z1B3");
    assert!(OperationId::parse("lowercase").is_err());
}

#[test]
fn expected_path_observation_hashes_exact_bytes_without_persistence() {
    assert_ne!(
        ExpectedPathObservation::from_bytes(b"before"),
        ExpectedPathObservation::from_bytes(b"after"),
    );
}
```

**Step 2: Run the focused target to verify it fails**

Run:

```sh
devenv shell -- cargo test --locked --test recovery_foundation_gate operation_ids_
```

Expected: FAIL because `OperationId` and `ExpectedPathObservation` do not exist.

**Step 3: Add minimal public declarations and dependencies**

Add production dependencies:

```toml
blake3 = "1"
fs4 = "0.13"
```

Add a private `OperationId(ulid::Ulid)` wrapper with parsing, display, equality,
and a test-only deterministic constructor. Add:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExpectedPathObservation {
    Missing,
    Blake3([u8; 32]),
}
```

Add `operation_id` to every existing mutable request. Replace the `remove_remote`
root/name pair with `RemoveRemoteRequest { root, name, operation_id }`; add a
similarly explicit `RemoveRegistrationRequest`, `RefreshRepositoryRequest`, and
`RebuildRepositoryRequest`. Add operation IDs to `AuthoringTarget` or to each
outer authoring request exactly once; do not let a nested context operation and
its enclosing save accidentally receive different IDs.

Declare, but do not yet implement, `RepositoryBusy`, `OperationMismatch`,
`RecoveryRequired`, `ExternalChange`, and `IndexPending` contract values. Update
all existing test request construction with `support::new_operation_id()`;
preserve one value across each existing retry test.

**Step 4: Run formatting and compilation regressions**

Run:

```sh
devenv shell -- cargo fmt --check
devenv shell -- cargo test --locked --test repository_enablement
devenv shell -- cargo test --locked --test local_authoring
```

Expected: existing behavior passes after mechanical request updates; the new
focused tests fail only at declared stubs.

**Step 5: Commit if explicitly requested**

```sh
git add Cargo.toml Cargo.lock src/repository.rs tests/support/mod.rs tests/repository_enablement.rs tests/local_authoring.rs tests/discovery_rebuild.rs tests/recovery_foundation_gate.rs
git commit -m "feat: add recovery operation contracts"
```

### Task 2: Add Private Cross-Process Coordination Primitives

**Files:**
- Modify: `src/repository.rs:1-75, 516-765, 1793-1821`
- Create: `src/repository/coordination.rs`
- Modify: `tests/support/mod.rs`
- Modify: `tests/recovery_foundation_gate.rs`

**Step 1: Write failing lease and child-holder tests**

Add parent/child tests for a held common-Git lease, an unrelated root, a held
bootstrap lease, and the cache guard. The child receives repository/data paths,
lock kind, a ready-file path, and a release-file path through environment
variables; it opens the private test hook, writes ready only after lock success,
and waits on the release condition.

```rust
#[test]
fn linked_worktree_operation_reports_busy_while_primary_holds_common_git_lease() {
    let fixture = support::born_repository();
    let context = support::document_context(&fixture);
    let holder = support::spawn_lease_holder(&fixture.root, LeaseKind::Repository);
    holder.wait_until_ready();

    let error = context.service.refresh_repository(context.refresh_request()).unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::RepositoryBusy);
    holder.release_and_wait();
}
```

**Step 2: Run lease tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test recovery_foundation_gate common_git_lease
```

Expected: FAIL because no file lease or child holder exists.

**Step 3: Implement `coordination.rs`**

Implement private RAII guards using `fs4::FileExt`:

```rust
pub(super) struct RepositoryLease(File);
pub(super) struct BootstrapLease(File);
pub(super) struct CacheReadGuard(File);
pub(super) struct CacheWriteGuard(File);
```

- Resolve the repository common Git directory from an opened `git2::Repository`.
  Open/create `<common-dir>/manyhands-operation.lock`; never unlink it.
- Repeatedly `try_lock_exclusive` until a monotonic 250 ms deadline. Convert
  only contention into `RepositoryBusy`; map open/lock I/O failures to the
  existing typed repository error with root and operation.
- Derive bootstrap file names from BLAKE3 of the validated intended absolute
  root beneath the application-data directory. Hold bootstrap while acquiring
  the newly created common-Git lease; no other path may acquire bootstrap after
  the common-Git lease.
- Open/create `manyhands.sqlite3.recovery.lock` beside the registry. Use shared
  locking only for a SQLite open/transaction scope and exclusive locking only
  for corrupt cache replacement. Keep guard lifetimes out of scans and Git
  mutations.
- Remove `process_repository_operation_lock` and `process_rebuild_lock` only
  after all callers have migrated. Do not retain an in-process fallback that
  could give different semantics from the cross-process lock.

Expose narrow `#[doc(hidden)]` test hooks for the child holder; do not expose
leases through the production public API.

**Step 4: Run lease tests and existing discovery tests**

Run:

```sh
devenv shell -- cargo test --locked --test recovery_foundation_gate common_git_lease
devenv shell -- cargo test --locked --test recovery_foundation_gate bootstrap_lease
devenv shell -- cargo test --locked --test discovery_rebuild
```

Expected: same-root primary/worktree operations get bounded busy, unrelated
roots proceed, and existing discovery behavior remains intact.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository.rs src/repository/coordination.rs tests/support/mod.rs tests/recovery_foundation_gate.rs Cargo.toml Cargo.lock
git commit -m "feat: coordinate repository operations across processes"
```

### Task 3: Migrate To Root-Keyed Durable Operation Records

**Files:**
- Modify: `src/repository.rs:18-23, 359-425, 1765-1821, 2555-3218`
- Modify: `src/repository/discovery.rs:1059-1249`
- Create: `src/repository/recovery.rs`
- Modify: `tests/discovery_rebuild.rs`
- Modify: `tests/recovery_foundation_gate.rs`

**Step 1: Write failing migration and replay tests**

Create a database at the Cycle 04 schema, seed registered roots plus completed
and incomplete `index_operations`, then open the service. Assert that a legacy
incomplete refresh/rebuild survives migration, has no public `OperationId`, and
is resumed only by the matching refresh/rebuild action. Assert an operation can
be recorded before registration and that removal clears every record for its
root.

```rust
#[test]
fn migration_preserves_an_incomplete_cycle_four_refresh_as_legacy_work() {
    let (data, root) = support::cycle_four_registry_with_incomplete_refresh();
    let service = RepositoryService::open_at(data.path()).unwrap();

    assert!(matches!(
        service.recovery_inspection(&root).unwrap().as_slice(),
        [RecoveryInspection::LegacyIndexOperation { operation: RepositoryOperation::RefreshRepository, .. }]
    ));
}
```

**Step 2: Run migration tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test recovery_foundation_gate migration_
```

Expected: FAIL because `operation_records` and legacy mapping do not exist.

**Step 3: Implement schema and private record API**

In `migrate_registry`, add an idempotent forward migration that creates
`operation_records` with a nullable `repository_id`, mandatory `root_path`,
nullable `operation_ulid`, action/target/step/state columns, Git resource
identifiers, timestamps, and redacted errors. Add a partial unique index:

```sql
CREATE UNIQUE INDEX operation_records_root_ulid_idx
ON operation_records(root_path, operation_ulid)
WHERE operation_ulid IS NOT NULL;
```

Move record queries out of `repository.rs` into `recovery.rs`. Implement:

- `begin_or_reconcile_operation` for a new ULID or exact replay;
- `advance_after_observation` that writes only non-content facts;
- `pending_for_root` and public recovery-inspection mapping;
- `require_no_conflicting_pending_operation` for a different root-local action;
- `resume_legacy_index_operation` for the matching Cycle 04 refresh/rebuild;
- explicit root cleanup during registration removal.

Map existing `index_operations` and `index_operation_contexts` to legacy records
without inventing an operation ULID. Keep enough context observation metadata to
resume Cycle 04 refresh/rebuild safely. Delete the old tables only after their
migration is covered and all Cycle 04 query sites use the new API.

**Step 4: Run migration and discovery regression tests**

Run:

```sh
devenv shell -- cargo test --locked --test recovery_foundation_gate migration_
devenv shell -- cargo test --locked --test discovery_rebuild
```

Expected: all Cycle 04 migrations and refresh/rebuild recovery evidence pass;
legacy incomplete work is visible and resumable.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository.rs src/repository/discovery.rs src/repository/recovery.rs tests/discovery_rebuild.rs tests/recovery_foundation_gate.rs
git commit -m "feat: persist root-scoped lifecycle recovery"
```

### Task 4: Refactor Refresh, Rebuild, And Availability To The New Guards

**Files:**
- Modify: `src/repository.rs:525-747, 1704-1821`
- Modify: `src/repository/discovery.rs:1059-1086`
- Modify: `src/repository/recovery.rs`
- Modify: `tests/discovery_rebuild.rs`
- Modify: `tests/recovery_foundation_gate.rs`

**Step 1: Write failing scan-window and root-scoped recovery tests**

Add a test that holds a repository lease while refresh waits and returns busy,
then releases and succeeds. Add a scan hook that changes canonical content after
the start observation; assert no mixed row replacement. Add two roots where an
incomplete rebuild on root A does not prevent root B refresh/snapshot. Retain a
structurally corrupt registry test proving only explicit-root rebuild is allowed
until the exclusive replacement succeeds.

```rust
#[test]
fn incomplete_rebuild_for_one_root_does_not_block_another_root() {
    let (first, second) = support::two_enabled_repositories();
    support::leave_rebuild_pending(&first);

    let refreshed = second.service.refresh_repository(second.refresh_request()).unwrap();

    assert!(matches!(refreshed, RefreshOutcome::Refreshed { .. }));
}
```

**Step 2: Run focused failures**

Run:

```sh
devenv shell -- cargo test --locked --test recovery_foundation_gate root_scoped_
devenv shell -- cargo test --locked --test recovery_foundation_gate refresh_scan_
```

Expected: FAIL because availability is still global and refresh holds its old
in-process lock across the scan.

**Step 3: Implement short-lease discovery orchestration**

Refactor `refresh_repository` to:

1. acquire a common-Git lease and begin/reconcile the record;
2. capture and persist the start observation fingerprint, then release lease;
3. run `observe_root` outside the lease;
4. reacquire lease, obtain the end observation, compare with start, and either
   transactionally persist stable rows or retain previous rows and record retry;
5. complete the same operation only after snapshot persistence.

Keep Cycle 04's per-context prior-row retention and retry problem semantics.
Wrap every SQLite open/transaction in a shared cache guard, including snapshot
reads, migration, record operations, and persistence. Do not hold a shared guard
through filesystem/Git scans.

For structural corruption, acquire only the exclusive cache guard before
`replace_corrupt_registry`, release it after opening/migrating the fresh cache,
then take the root lease for explicit-root rebuild. Replace global
`IndexAvailability::Recovering` behavior with root-record checks; retain global
`Degraded` only while structural corruption has not been replaced.

**Step 4: Run discovery and corruption regression suites**

Run:

```sh
devenv shell -- cargo test --locked --test discovery_rebuild
devenv shell -- cargo test --locked --test recovery_foundation_gate root_scoped_
devenv shell -- cargo test --locked --test recovery_foundation_gate cache_recovery_
```

Expected: scans no longer monopolize the repository lease; changed contexts
retain prior rows; unrelated roots continue during root-local recovery; cache
replacement remains read-only with respect to canonical/Git state.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository.rs src/repository/discovery.rs src/repository/recovery.rs tests/discovery_rebuild.rs tests/recovery_foundation_gate.rs
git commit -m "feat: reconcile guarded discovery operations"
```

### Task 5: Journal And Coordinate Repository Creation, Enablement, And Remotes

**Files:**
- Modify: `src/repository.rs:1828-2523, 4158-6259`
- Modify: `src/repository/recovery.rs`
- Modify: `tests/support/mod.rs`
- Modify: `tests/repository_enablement.rs`
- Modify: `tests/recovery_foundation_gate.rs`

**Step 1: Write failing lifecycle-replay tests**

For each action, inject a failure before/after its authoritative effect, reopen a
fresh service, and replay the same ID. Cover create-and-enable bootstrap
contention, enable configuration write/initialization commit/registration,
remote add/remove, publication configuration commit, and registration removal.

```rust
#[test]
fn replay_after_initialization_commit_refreshes_without_a_second_commit() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let request = support::enable_request(&fixture, support::new_operation_id());
    let failed = support::FailOnce::at(FailurePoint::BeforeIndexTransactionCommit)
        .open_service(data.path());

    let first = failed.enable(request.clone()).unwrap();
    let commit = support::commit_oid(&first);
    let retry = RepositoryService::open_at(data.path()).unwrap().enable(request).unwrap();

    assert_eq!(support::commit_oid(&retry), commit);
    assert_eq!(support::commit_count(&fixture.root), 2);
}
```

**Step 2: Run enablement failures to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test recovery_foundation_gate enable_replay_
devenv shell -- cargo test --locked --test recovery_foundation_gate remote_replay_
```

Expected: FAIL because these methods neither take file leases nor journal their
authoritative steps and refresh handoff.

**Step 3: Wrap each lifecycle action**

Use one orchestration helper per existing-root action: acquire lease,
begin/reconcile record, re-observe input, execute one existing unlocked routine,
advance after each observed effect, and invoke Task 8's refresh handoff. Keep
the existing pre-commit rollback routines; record observed partial state after a
failure rather than weakening their restoration rules.

For `create_and_enable`, validate the intended root first, take bootstrap,
create/open the repository, acquire common-Git lease before releasing bootstrap,
then use the same record/reconciliation path. Remote add/remove must re-observe
the named remote/configuration under lease and report no-op or recovery without
contacting transport.

Registration removal records its start, removes local rows and records for the
root atomically under cache guard, and treats exact replay against absent local
registration as successful no-op. It never changes repository files or Git.

**Step 4: Run existing and new enablement suites**

Run:

```sh
devenv shell -- cargo test --locked --test repository_enablement
devenv shell -- cargo test --locked --test recovery_foundation_gate enable_
devenv shell -- cargo test --locked --test recovery_foundation_gate remote_
```

Expected: all old no-network and rollback guarantees remain true; exact replay
creates no duplicate config/initialization/publication commits or remotes.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository.rs src/repository/recovery.rs tests/support/mod.rs tests/repository_enablement.rs tests/recovery_foundation_gate.rs
git commit -m "feat: recover repository lifecycle mutations"
```

### Task 6: Journal Context Provisioning And Add Stale-Write Protection

**Files:**
- Modify: `src/repository.rs:749-1564`
- Modify: `src/repository/recovery.rs`
- Modify: `tests/support/mod.rs`
- Modify: `tests/local_authoring.rs`
- Modify: `tests/recovery_foundation_gate.rs`

**Step 1: Write failing context and external-change tests**

Cover branch/worktree creation interruption and exact same-ID replay. Add direct
filesystem modifications between caller observation and document edit/move.
Assert the service returns `ExternalChange`, leaves the caller-provided draft
out of SQLite, preserves on-disk bytes and commit graph, and never refreshes.

```rust
#[test]
fn stale_document_edit_preserves_agent_written_markdown_without_checkpoint() {
    let setup = support::editable_document_context();
    let request = setup.edit_request_with_current_observation();
    std::fs::write(setup.document_path(), "agent replacement\n").unwrap();
    let before = support::repository_and_worktree_snapshot_at(&setup.root);

    let error = setup.service.save_document(request).unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::ExternalChange);
    assert_eq!(support::repository_and_worktree_snapshot_at(&setup.root), before);
}
```

**Step 2: Run authoring failures to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test recovery_foundation_gate stale_document_
devenv shell -- cargo test --locked --test recovery_foundation_gate context_replay_
```

Expected: FAIL because saves do not accept/compare expected observations and
context operations have no durable record.

**Step 3: Implement context and owned-path reconciliation**

Refactor `prepare_context` and its unlocked helper so the outer path owns the
lease and record. Record only observed branch and worktree creation/reuse. A
partial deterministic branch/worktree remains recoverable for the exact target;
a mismatched or duplicate context remains visible recovery-required state.

Add a helper that reads an owned regular-file path without following a symlink
and maps its bytes to `Missing` or `ExpectedPathObservation::Blake3`. Invoke it
after lease acquisition and before every owned write:

- document edit: destination's expected observation;
- document move: source and destination expectations;
- ticket edit: ticket path expectation;
- document/ticket/comment creation: destination must be `Missing`.

Use BLAKE3 only in memory. On mismatch return typed `ExternalChange` with root,
item, context, and affected path; do not record a write as completed, stage, or
commit. Preserve existing canonical validation, unknown-front-matter retention,
temporary-index bounds, and no-op behavior.

**Step 4: Run authoring and stale-write regression suites**

Run:

```sh
devenv shell -- cargo test --locked --test local_authoring
devenv shell -- cargo test --locked --test recovery_foundation_gate stale_
devenv shell -- cargo test --locked --test recovery_foundation_gate context_
```

Expected: Cycle 03 behavior remains valid, while direct agent edits cannot be
silently overwritten and partial contexts resume exactly once.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository.rs src/repository/recovery.rs tests/support/mod.rs tests/local_authoring.rs tests/recovery_foundation_gate.rs Cargo.toml Cargo.lock
git commit -m "feat: preserve external authoring changes"
```

### Task 7: Checkpoint/Refresh Handoff And Index-Pending Outcomes

**Files:**
- Modify: `src/repository.rs:151-200, 1615-1702, 525-631`
- Modify: `src/repository/recovery.rs`
- Modify: `tests/local_authoring.rs`
- Modify: `tests/discovery_rebuild.rs`
- Modify: `tests/recovery_foundation_gate.rs`

**Step 1: Write failing automatic-refresh tests**

For a successful document save, ticket save, comment submit, context provision,
enablement, and remote change, assert the stored discovery snapshot reflects
the result without a separate refresh call. Inject scan observation/transaction
failure after the authoritative transition; assert `IndexPending` includes the
commit OID or context and same-ID replay runs no second Git/canonical step.

```rust
#[test]
fn checkpoint_then_failed_refresh_replays_index_only() {
    let setup = support::editable_document_context();
    let request = setup.changed_save_request(support::new_operation_id());
    let first = support::FailOnce::at(FailurePoint::BeforeIndexTransactionCommit)
        .open_service(setup.data.path())
        .save_document(request.clone())
        .unwrap();
    let commit = support::checkpoint_oid(&first).unwrap();

    let retry = RepositoryService::open_at(setup.data.path()).unwrap().save_document(request).unwrap();

    assert_eq!(support::checkpoint_oid(&retry), Some(commit));
    assert!(support::snapshot_contains_document(&setup));
}
```

**Step 2: Run handoff tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test recovery_foundation_gate index_pending_
```

Expected: FAIL because `checkpoint_owned_paths` only marks `refresh_required`
and never invokes the Cycle 04 scanner.

**Step 3: Implement a single post-authoritative-change handoff**

Replace `mark_checkpoint_refresh` as the terminal behavior with a helper that:

1. records the authoritative commit/context/remote effect;
2. releases the mutation lease;
3. invokes the same operation's Task 4 refresh phase;
4. maps stable persistence to completed and an unstable/failed scan to
   `IndexPending` without re-entering write/commit code.

Make `LocalCheckpoint`, `SaveOutcome`, `CommentSubmissionOutcome`,
`EnableRepositoryOutcome`, `RemoteOutcome`, and `PublicationRemoteOutcome`
preserve their existing successful data while representing index-pending
explicitly. A no-op still refreshes only when its record has pending discovery;
an ordinary no-op does not create a commit or unnecessary scan.

**Step 4: Run handoff and existing lifecycle suites**

Run:

```sh
devenv shell -- cargo test --locked --test local_authoring
devenv shell -- cargo test --locked --test repository_enablement
devenv shell -- cargo test --locked --test discovery_rebuild
devenv shell -- cargo test --locked --test recovery_foundation_gate index_pending_
```

Expected: authoritative local changes appear in discovery automatically; retry
after indexing failure never duplicates a commit, branch, worktree, item, or
comment.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository.rs src/repository/recovery.rs tests/local_authoring.rs tests/repository_enablement.rs tests/discovery_rebuild.rs tests/recovery_foundation_gate.rs
git commit -m "feat: complete lifecycle discovery handoff"
```

### Task 8: Complete The Wave 1 Recovery Acceptance Matrix

**Files:**
- Modify: `tests/support/mod.rs`
- Modify: `tests/recovery_foundation_gate.rs`
- Modify: `tests/repository_enablement.rs`
- Modify: `tests/local_authoring.rs`
- Modify: `tests/discovery_rebuild.rs`

**Step 1: Write missing end-to-end acceptance cases**

Add one disposable real-repository journey that covers born and unborn
enablement, local remote configuration, document/ticket/comment creation,
context reuse, checkpoints, automatic discovery, explicit-root rebuild, and
registration removal. Add parameterized failure/restart cases for configuration
write, initialization commit, branch creation, worktree creation, item write,
checkpoint, SQLite persistence, refresh scan race, and corrupt-cache
replacement.

```rust
#[test]
fn wave_one_lifecycle_retries_every_interruption_without_duplicate_artifacts() {
    for failure in support::wave_one_failure_points() {
        let scenario = support::wave_one_scenario();
        let request = scenario.next_request_with_stable_operation_id();
        let failed = support::FailOnce::at(failure).open_service(scenario.data.path());

        let _ = scenario.run(&failed, request.clone());
        let retained = scenario.actual_state();
        let retry = RepositoryService::open_at(scenario.data.path()).unwrap();
        scenario.run(&retry, request);

        scenario.assert_only_unfinished_work_changed(retained);
    }
}
```

**Step 2: Run acceptance target to verify any remaining gaps**

Run:

```sh
devenv shell -- cargo test --locked --test recovery_foundation_gate
```

Expected: PASS only after every Cycle 05 behavior is implemented. Investigate
each failure against actual Git/filesystem/SQLite state; do not weaken a test by
accepting a duplicate or hidden artifact.

**Step 3: Harden diagnostics and compatibility assertions**

Ensure typed errors/outcomes identify root, operation, and item/context/path
where applicable without lock-holder details or content. Assert operation rows,
diagnostic corrupt cache files, test output, and snapshots contain no Markdown
body, BLAKE3 digest, credentials, or private-key-like content. Keep Windows,
macOS, and Linux file-lock behavior documented as a compatibility case if CI
reveals a supported-platform difference.

**Step 4: Run all focused suites**

Run:

```sh
devenv shell -- cargo test --locked --test repository_enablement
devenv shell -- cargo test --locked --test local_authoring
devenv shell -- cargo test --locked --test discovery_rebuild
devenv shell -- cargo test --locked --test recovery_foundation_gate
```

Expected: all focused Cycle 02–05 targets pass with no network contact and no
system Git executable.

**Step 5: Commit if explicitly requested**

```sh
git add tests/support/mod.rs tests/recovery_foundation_gate.rs tests/repository_enablement.rs tests/local_authoring.rs tests/discovery_rebuild.rs
git commit -m "test: prove Wave 1 recovery gate"
```

### Task 9: Run The Required Full Verification Suite

**Files:**
- Modify only files required to fix a verified failure from this task.

**Step 1: Check formatting**

Run:

```sh
devenv shell -- cargo fmt --check
```

Expected: PASS.

**Step 2: Check all feature configurations**

Run:

```sh
devenv shell -- cargo check --all-features --locked
```

Expected: PASS with the new `fs4` and `blake3` lockfile entries.

**Step 3: Run linting**

Run:

```sh
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

Expected: PASS with no warnings.

**Step 4: Run the complete test suite**

Run:

```sh
devenv shell -- cargo test --all-features --locked
```

Expected: PASS, including all Cycle 05 process, recovery, and regression tests.

**Step 5: Smoke-test the headless CLI skeleton**

Run:

```sh
devenv shell -- cargo run --locked --bin manyhands-cli
```

Expected: PASS without opening a desktop window. No desktop smoke test is
required because Cycle 05 introduces no desktop behavior.

**Step 6: Commit if explicitly requested**

```sh
git status --short
git add Cargo.toml Cargo.lock src/repository.rs src/repository/coordination.rs src/repository/recovery.rs src/repository/discovery.rs tests/support/mod.rs tests/repository_enablement.rs tests/local_authoring.rs tests/discovery_rebuild.rs tests/recovery_foundation_gate.rs
git commit -m "feat: complete Wave 1 recovery foundation"
```

## Expected Final File Set

- `Cargo.toml`
- `Cargo.lock`
- `src/repository.rs`
- `src/repository/coordination.rs`
- `src/repository/recovery.rs`
- `src/repository/discovery.rs`
- `tests/support/mod.rs`
- `tests/repository_enablement.rs`
- `tests/local_authoring.rs`
- `tests/discovery_rebuild.rs`
- `tests/recovery_foundation_gate.rs`

No Cycle 05 implementation task changes `src/main.rs`, `src/bin/manyhands-cli.rs`,
desktop modules, tracked Git configuration in fixture repositories, or remote
transport behavior.
