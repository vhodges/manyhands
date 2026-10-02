---
title: "Wave 01 Cycle 04 Discovery and Rebuild Implementation Plan"
date: 2026-10-02
status: approved
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01K7C6F8H1J3M5P7R9T2V4X6Z8"
---

# Wave 01 Cycle 04 Discovery and Rebuild Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Deliver deterministic, metadata-only discovery refresh and explicit-root
rebuild for registered Manyhands repositories without mutating canonical Markdown
or Git state.

**Architecture:** Keep `RepositoryService` as the small synchronous public
service. Add a private `repository::discovery` module that migrates SQLite,
observes roots and deterministic worktrees with `git2`, parses canonical sources,
persists only metadata in context-scoped transactions, and maps rows into a
stable snapshot. A structurally corrupt SQLite cache opens in a rebuild-only
degraded state; explicit-root rebuild preserves diagnostics and reconstructs one
caller-supplied repository.

**Tech Stack:** Rust 2024, existing `git2`/libgit2, `rusqlite`, `time`,
`tempfile`, canonical domain module, Cargo, and Devenv/Nix.

**Commit Policy:** Do not create commits unless the user explicitly requests
one. If requested, stage only the completed task's files and use the suggested
commit message.

---

## Authorities And Fixed Decisions

Read these before implementation:

- `AGENTS.md`
- `docs/Cycles/wave-01-cycle-04-discovery-and-rebuild.md`
- `docs/plans/2026-10-02-wave-01-cycle-04-discovery-and-rebuild-design.md`
- `docs/RFC/canonical-content-and-comment-schema.md`
- `docs/RFC/git-workflow-and-conflict-recovery.md`
- `docs/RFC/repository-index-persistence-and-refresh.md`
- `docs/RFC/test-and-compatibility-strategy.md`

The implementation must preserve these decisions:

- All Git access uses `git2`; do not run a system Git executable or contact a
  remote.
- Markdown, tracked configuration, and Git are authoritative. SQLite contains
  no Markdown bodies, credentials, keys, passphrases, or canonical-content copy.
- `RepositoryService` remains synchronous and headless. Git handles, SQLite
  connections, transactions, statements, and directory iterators are local to
  the operation that opens them.
- The only Cycle 04 durable operations are refresh and rebuild. A Cycle 03
  `refresh_required` flag retries indexing only and cannot create another commit.
- A root with invalid configuration or a wrong checked-out primary branch is
  scanned as `Unverified`; invalid configuration suppresses active-context
  enumeration.
- Valid configuration enables full scanning of every recognized deterministic
  local context. A convention-matching worktree with an absent or invalid
  branch-identified item is a visible context problem, not an active context.
- Canonical locations are scanned directly. Markdown outside those paths is a
  problem candidate only if leading YAML front matter declares
  `manyhands_managed: true`; ordinary Markdown is ignored.
- A changed HEAD or relevant file observation during scanning keeps prior rows,
  creates a retry problem, leaves refresh required, and returns a typed outcome.
- A corrupt SQLite file opens a degraded service. Only explicit-root rebuild is
  permitted until recovery succeeds; no automatic cache deletion occurs at open.
- Explicit-root rebuild may reconstruct only the root supplied by the caller.
  It never searches the filesystem for previous registrations or adds a second
  durable root registry.

## Public Contract

Add the public types below to `src/repository.rs`. The exact private row and
scanner types belong in `src/repository/discovery.rs`.

```rust
pub struct RepositorySnapshot {
    pub root: PathBuf,
    pub configuration: SnapshotConfiguration,
    pub refresh_required: bool,
    pub contexts: Vec<DiscoveredContext>,
    pub items: Vec<DiscoveredItem>,
    pub problems: Vec<DiscoveryProblem>,
}

pub enum SnapshotConfiguration {
    Valid { primary_branch: String, publication_remote: Option<String> },
    Invalid { code: canonical::ValidationCode, guidance: String },
    Missing,
}

pub enum DiscoveryContextKind { Primary, Unverified, Active }
pub enum DiscoveryActivitySource { GitCommit, UncommittedFilesystem }

pub struct DiscoveredContext {
    pub kind: DiscoveryContextKind,
    pub branch: Option<String>,
    pub worktree: PathBuf,
    pub item_id: Option<canonical::ItemId>,
    pub head_oid: Option<git2::Oid>,
}

pub struct DiscoveredItem {
    pub context: PathBuf,
    pub id: canonical::ItemId,
    pub kind: AuthoringKind,
    pub path: PathBuf,
    pub title: String,
    pub ticket_type: Option<String>,
    pub status: Option<String>,
    pub project: Option<String>,
    pub team: Option<String>,
    pub closed_at: Option<OffsetDateTime>,
    pub activity_at: OffsetDateTime,
    pub activity_source: DiscoveryActivitySource,
    pub comments: Vec<DiscoveredCommentThread>,
}

pub struct DiscoveredCommentThread {
    pub id: canonical::ItemId,
    pub path: PathBuf,
    pub created_at: OffsetDateTime,
    pub replies: Vec<DiscoveredCommentThread>,
}

pub struct DiscoveryProblem {
    pub context: Option<PathBuf>,
    pub path: Option<PathBuf>,
    pub code: String,
    pub guidance: String,
    pub observed_at: OffsetDateTime,
}

pub enum RefreshOutcome {
    Refreshed { snapshot: RepositorySnapshot },
    RetryRequired { root: PathBuf, context: Option<PathBuf> },
}
```

Add these methods to `RepositoryService`:

```rust
pub fn refresh_repository(&self, root: &Path) -> Result<RefreshOutcome, RepositoryError>;
pub fn rebuild_repository(&self, root: &Path) -> Result<RepositorySnapshot, RepositoryError>;
pub fn repository_snapshot(&self, root: &Path) -> Result<RepositorySnapshot, RepositoryError>;
```

Add dedicated `RepositoryOperation` values for refresh, rebuild, and snapshot;
add typed errors for an unregistered root and unavailable index; add test-only
failure points for after-context observation, before index transaction commit,
and before corrupt-cache replacement. Do not expose table or operation-record
row shapes publicly.

### Task 1: Establish The Discovery Test Target And Public Skeleton

**Files:**
- Modify: `src/repository.rs:23-278`
- Create: `tests/discovery_rebuild.rs`

**Step 1: Write the failing public-contract tests**

Create `tests/discovery_rebuild.rs` with `mod support;`. Add a test that enables
a born fixture, calls `repository_snapshot`, and proves the expected method and
types exist. Add a second test that attempts `refresh_repository` before any
scanner is implemented.

```rust
#[test]
fn snapshot_requires_a_registered_repository_before_refresh() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();

    let error = service.repository_snapshot(&fixture.root).unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::RepositoryNotRegistered);
    assert_eq!(error.operation, RepositoryOperation::RepositorySnapshot);
}
```

**Step 2: Run the focused test to verify it fails**

Run:

```sh
devenv shell -- cargo test --locked --test discovery_rebuild snapshot_requires_a_registered_repository_before_refresh
```

Expected: FAIL because the snapshot method and discovery errors do not exist.

**Step 3: Add declarations only**

Add the public snapshot, metadata, comment-thread, problem, activity, and
outcome types; add the operation/error/failure-point variants; add the three
method signatures as deliberate temporary stubs. The types must contain no body
field or opaque SQLite row. Do not add schema, filesystem, Git, or scan behavior
in this task.

**Step 4: Run formatting and verify the failure reaches the stub**

Run:

```sh
devenv shell -- cargo fmt --check
devenv shell -- cargo test --locked --test discovery_rebuild snapshot_requires_a_registered_repository_before_refresh
```

Expected: formatting passes; the test compiles and fails only at the deliberate
stub.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository.rs tests/discovery_rebuild.rs
```

### Task 2: Add A Private Discovery Module And Compatible SQLite Migration

**Files:**
- Modify: `src/repository.rs:23-27, 1288-1303, 4158-4173`
- Create: `src/repository/discovery.rs`
- Modify: `tests/repository_enablement.rs:71-90`
- Modify: `tests/discovery_rebuild.rs`

**Step 1: Write failing migration tests**

Add a test that creates the current Cycle 02 `repositories` table manually,
opens `RepositoryService`, and asserts that migration retains the registration
while adding foreign-key-backed derived tables. Add a test that removing a
registration removes only that repository's derived rows.

```rust
#[test]
fn registry_migration_retains_existing_registration_and_adds_discovery_tables() {
    let data = tempfile::tempdir().unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute_batch(CYCLE_02_REPOSITORIES_SCHEMA).unwrap();
    connection.execute(CYCLE_02_REGISTERED_ROOT, params!["/fixture", 1_i64, "oid"]).unwrap();
    drop(connection);

    let service = RepositoryService::open_at(data.path()).unwrap();

    service.with_registry_connection_for_testing(|connection| {
        assert_eq!(table_exists(connection, "contexts"), true);
        assert_eq!(table_exists(connection, "discovered_items"), true);
        assert_eq!(table_exists(connection, "discovered_comments"), true);
        assert_eq!(table_exists(connection, "problems"), true);
        assert_eq!(table_exists(connection, "index_operations"), true);
    }).unwrap();
}
```

**Step 2: Run the migration tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test discovery_rebuild migration_
```

Expected: FAIL because the tables and migration do not exist.

**Step 3: Move discovery persistence into a private child module**

Declare `mod discovery;` in `src/repository.rs`. Move `open_registry` and
`migrate_registry` into that child module or make narrowly scoped parent wrappers
that call it. Preserve the current connection guarantees: five-second busy
timeout, foreign keys enabled, WAL set on every connection.

Create a forward-only table-rebuild migration for `repositories` that preserves
every existing row but changes `config_blob_oid` from `TEXT NOT NULL` to nullable
`TEXT`. Write an OID only when valid configuration is observed; use `NULL` plus a
problem for missing or invalid configuration. Then create these private logical
tables:

```sql
CREATE TABLE IF NOT EXISTS contexts (
    id INTEGER PRIMARY KEY,
    repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    branch TEXT,
    worktree_path TEXT NOT NULL,
    item_id TEXT,
    head_oid TEXT,
    UNIQUE(repository_id, worktree_path)
);

CREATE TABLE IF NOT EXISTS discovered_items (
    id INTEGER PRIMARY KEY,
    context_id INTEGER NOT NULL REFERENCES contexts(id) ON DELETE CASCADE,
    item_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    canonical_path TEXT NOT NULL,
    title TEXT NOT NULL,
    ticket_type TEXT,
    status TEXT,
    project TEXT,
    team TEXT,
    closed_at INTEGER,
    activity_at INTEGER NOT NULL,
    activity_source TEXT NOT NULL,
    UNIQUE(context_id, item_id)
);
```

Add analogous private metadata-only `discovered_comments`, `problems`, and
`index_operations` tables. Use foreign keys back to the owning context or
repository and never add a body column. Index query keys needed for root,
context, item, and ordered comments. Preserve idempotent migration behavior.

**Step 4: Run migration and existing registry tests**

Run:

```sh
devenv shell -- cargo test --locked --test discovery_rebuild migration_
```

Expected: PASS. Existing Cycle 02 registry tests remain valid with the extended
schema.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository.rs src/repository/discovery.rs tests/repository_enablement.rs tests/discovery_rebuild.rs
```

### Task 3: Persist And Read A Deterministic Metadata-Only Snapshot

**Files:**
- Modify: `src/repository.rs`
- Modify: `src/repository/discovery.rs`
- Modify: `tests/discovery_rebuild.rs`

**Step 1: Write failing stored-snapshot tests**

Seed one repository, two contexts, document and ticket metadata, comment rows,
and intentionally unsorted problem rows through test-only SQLite access. Assert
that `repository_snapshot` reconstructs stable context/item/problem ordering and
comment tree order, without a body field.

```rust
#[test]
fn snapshot_orders_contexts_items_comments_and_problems_deterministically() {
    let (service, fixture) = seeded_discovery_registry();

    let snapshot = service.repository_snapshot(&fixture.root).unwrap();

    assert_eq!(snapshot.contexts[0].worktree, fixture.root);
    assert_eq!(snapshot.items[0].path, PathBuf::from("docs/a.md"));
    assert_eq!(snapshot.items[0].comments[0].id, support::root_comment_id());
    assert_eq!(snapshot.items[0].comments[0].replies[0].id, support::reply_id());
}
```

**Step 2: Run the stored-snapshot tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test discovery_rebuild snapshot_
```

Expected: FAIL because no mapper or snapshot query exists.

**Step 3: Implement private row-to-snapshot mapping**

Implement `repository_snapshot` as a read-only transaction that resolves the
canonical root key, requires exactly one registration, loads contexts/items/
comments/problems with explicit `ORDER BY` clauses, and maps database text back
to validated `ItemId`, `PathBuf`, `Oid`, and UTC `OffsetDateTime` values. Treat
impossible cache values as a typed index-unavailable/corrupt-cache condition,
not as canonical truth.

Build comment trees from stored IDs and parent IDs using the same `created_at`,
then ULID order as `canonical::ordered_comment_threads`. Do not add comment or
item bodies to the row types or snapshot.

**Step 4: Run the snapshot tests**

Run:

```sh
devenv shell -- cargo test --locked --test discovery_rebuild snapshot_
```

Expected: PASS with deterministic ordering and metadata-only snapshot types.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository.rs src/repository/discovery.rs tests/discovery_rebuild.rs
```

### Task 4: Scan Root Content And Surface Configuration Problems

**Files:**
- Modify: `src/repository/discovery.rs`
- Modify: `tests/support/mod.rs`
- Modify: `tests/discovery_rebuild.rs`

**Step 1: Write failing root-scan tests**

Add fixtures that commit valid document/ticket/comment files to a registered
primary root. Add cases for missing, malformed, unsupported, and wrong-primary
configuration. The first three must yield an unverified root and no active
context enumeration; a valid configuration with the root on another branch must
yield an unverified root but remain eligible to discover valid active contexts in
Task 5.

```rust
#[test]
fn refresh_scans_readable_root_as_unverified_when_configuration_is_malformed() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    support::write_document_source(&fixture.root, "docs/visible.md");
    std::fs::write(fixture.root.join(".manyhands/config.toml"), "not = [valid").unwrap();

    let RefreshOutcome::Refreshed { snapshot } = enabled.service.refresh_repository(&fixture.root).unwrap() else {
        panic!("expected stable refresh");
    };

    assert!(matches!(snapshot.contexts[0].kind, DiscoveryContextKind::Unverified));
    assert_eq!(snapshot.items.len(), 1);
    assert!(snapshot.problems.iter().any(|problem| problem.path.as_deref() == Some(Path::new(".manyhands/config.toml"))));
}
```

**Step 2: Run root-scan tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test discovery_rebuild root_
```

Expected: FAIL because refresh does not scan filesystem or canonical sources.

**Step 3: Implement read-only root observation and source collection**

Create private observation structs containing the context identity, initial HEAD
OID, final HEAD OID, relevant source paths, source bytes, and filesystem
observations. Reuse `canonical::validate_context` for parsing and relationships.
Do not reuse Cycle 03's writer-oriented collector if it rejects rather than
reports malformed sources.

Collect documents under `docs/`, tickets under `.manyhands/tickets/`, and
comments under `.manyhands/comments/`. Traverse with the existing depth and
entry limits; turn unreadable/non-UTF-8/unsafe entries into visible problems
instead of panics. Root traversal must skip `.git` and `.manyhands/worktrees/`.

Classify configuration with `read_configuration_for`. Valid configuration plus a
matching root branch yields `Primary`; any configuration problem or branch
mismatch yields `Unverified`. Root scanning never writes files, stages a path,
or opens a mutable Git index.

**Step 4: Run root scan and regression tests**

Run:

```sh
devenv shell -- cargo test --locked --test discovery_rebuild root_
```

Expected: PASS. Existing canonical behavior remains unchanged.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository/discovery.rs tests/support/mod.rs tests/discovery_rebuild.rs
```

### Task 5: Detect Managed Out-Of-Path Files And Active Contexts

**Files:**
- Modify: `src/repository/discovery.rs`
- Modify: `tests/support/mod.rs`
- Modify: `tests/discovery_rebuild.rs`

**Step 1: Write failing path and context tests**

Add tests for marker-only and malformed canonical files, a managed-marked file
outside a canonical path, and ordinary `README.md`. Add a deterministic document
and ticket worktree through the Cycle 03 service, populate their contexts, then
verify both contexts are scanned and distinguishable. Add a matching branch/path
whose identified item is absent and assert it becomes only a context problem.

```rust
#[test]
fn refresh_uses_a_valid_active_context_over_the_primary_copy() {
    let (fixture, enabled, context) = document_context_with_primary_copy();
    support::write_document_source(&context.worktree, "docs/item.md");

    let RefreshOutcome::Refreshed { snapshot } = enabled.service.refresh_repository(&fixture.root).unwrap() else {
        panic!("expected stable refresh");
    };

    let item = snapshot.items.iter().find(|item| item.id == support::document_id()).unwrap();
    assert_eq!(item.context, context.worktree);
    assert!(snapshot.contexts.iter().any(|context| matches!(context.kind, DiscoveryContextKind::Primary)));
    assert!(snapshot.contexts.iter().any(|context| matches!(context.kind, DiscoveryContextKind::Active)));
}
```

**Step 2: Run context/path tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test discovery_rebuild context_
```

Expected: FAIL because active worktrees and out-of-path markers are not observed.

**Step 3: Implement candidate marker filtering and context recognition**

For Markdown outside canonical locations, inspect only leading front matter
enough to determine whether it declares `manyhands_managed: true`; then pass the
file through canonical validation so its invalid path is reported. Do not report
ordinary Markdown and do not accept a text occurrence of the marker outside
leading YAML front matter.

With valid configuration, enumerate worktrees beneath the configured base.
Validate the exact worktree path, checked-out `manyhands/<kind>/<ULID>` branch,
and matching conforming target item. Fully scan every recognized active context.
Record a stable problem for a convention-matching candidate whose target is
missing or invalid, but do not insert it as `Active` or promote its other files.

When merging context views, preserve all stored primary and active rows but set
the snapshot's preferred item view to the one valid deterministic active context
for that ID. Do not create a choice outcome or a second context.

**Step 4: Run the path/context tests**

Run:

```sh
devenv shell -- cargo test --locked --test discovery_rebuild context_
```

Expected: PASS. Marker-prefiltered invalid paths are visible; ordinary Markdown
is absent; valid local contexts have precedence.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository/discovery.rs tests/support/mod.rs tests/discovery_rebuild.rs
```

### Task 6: Derive Comment Trees And Content Activity

**Files:**
- Modify: `src/repository/discovery.rs`
- Modify: `tests/support/mod.rs`
- Modify: `tests/discovery_rebuild.rs`

**Step 1: Write failing comment and activity tests**

Create root comments and replies with equal timestamps but different ULIDs; assert
the snapshot uses canonical ordering. Commit an item then a valid comment and
assert the item's activity is the newer commit. Modify a relevant owned file
without committing and assert its controlled filesystem observation takes
precedence. Verify comment bodies are absent from the SQLite schema and public
types.

```rust
#[test]
fn refresh_prefers_newer_uncommitted_activity_over_last_git_commit() {
    let (fixture, enabled, context) = committed_document_context();
    write_document_with_mtime(&context.worktree, "docs/item.md", FUTURE_TIME);

    let RefreshOutcome::Refreshed { snapshot } = enabled.service.refresh_repository(&fixture.root).unwrap() else {
        panic!("expected stable refresh");
    };

    let item = snapshot.items.iter().find(|item| item.id == support::document_id()).unwrap();
    assert_eq!(item.activity_source, DiscoveryActivitySource::UncommittedFilesystem);
    assert_eq!(item.activity_at, FUTURE_TIME);
}
```

**Step 2: Run comment and activity tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test discovery_rebuild comment_
```

Expected: FAIL because comment rows and activity derivation are absent.

**Step 3: Implement metadata-only comments and activity queries**

Persist valid comments in a dedicated metadata table keyed by context, comment
ID, and target item ID. Store canonical path, parent ID, and creation time only.
Associate only valid comments with their target item's activity. Walk first-parent
history or an equivalent deterministic `git2` history query to find the latest
commit touching each item path or its valid comment paths. Compare relevant
filesystem modification observations and select the newer source.

Keep activity query cost bounded to Cycle 1 fixture scale. Do not add an
incremental Git-diff index, watcher, background worker, or Markdown body cache.

**Step 4: Run comment/activity tests**

Run:

```sh
devenv shell -- cargo test --locked --test discovery_rebuild comment_
```

Expected: PASS with ordered metadata-only comment trees and correct activity
source selection.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository/discovery.rs tests/support/mod.rs tests/discovery_rebuild.rs
```

### Task 7: Persist Stable Refresh Results And Reconcile Cycle 03 Invalidation

**Files:**
- Modify: `src/repository.rs:1191-1278`
- Modify: `src/repository/discovery.rs`
- Modify: `tests/discovery_rebuild.rs`

**Step 1: Write failing persistence tests**

Add tests that refresh a repository after a Cycle 03 `RefreshPending` checkpoint,
then assert the snapshot contains the committed content, the flag clears, and
the commit count remains unchanged. Add tests that delete canonical files or
remove a context and prove later refresh removes stale rows. Add a scan-change
fixture that updates a context after initial observation and before persistence.

```rust
#[test]
fn refresh_after_checkpoint_invalidation_failure_indexes_without_another_commit() {
    let (fixture, failed_service, request) = pending_document_checkpoint();
    let before = support::commit_count(&fixture.repository);
    failed_service.save_document(request).unwrap();

    let RefreshOutcome::Refreshed { snapshot } = failed_service.refresh_repository(&fixture.root).unwrap() else {
        panic!("expected stable refresh");
    };

    assert_eq!(support::commit_count(&fixture.repository), before + 1);
    assert!(!snapshot.refresh_required);
}
```

**Step 2: Run persistence tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test discovery_rebuild refresh_
```

Expected: FAIL because refresh neither writes rows transactionally nor observes
changes during a scan.

**Step 3: Implement refresh operation records and replacement transactions**

Require a registered root for `refresh_repository`. Create or resume a private
`Refresh` operation. Capture initial HEAD and relevant file observations for each
context, scan without repository mutation, then recapture observations before
writing. In a transaction:

1. Replace only one stable context's item/comment/problem rows.
2. Retain prior rows and add a retry problem when that context changed.
3. Remove rows for contexts no longer exposed by the completed root observation.
4. Clear `refresh_required` only when every observed context persisted stably.
5. Advance the operation record after each observed step and mark completion
   only after the final transaction commits.

Use `BeforeIndexTransactionCommit` and `AfterContextObservation` failure hooks.
On an SQLite error, leave canonical state untouched and retain or report only
the durable operation state actually committed. Do not call any authoring,
checkpoint, or remote API from refresh.

**Step 4: Run persistence and Cycle 03 regressions**

Run:

```sh
devenv shell -- cargo test --locked --test discovery_rebuild refresh_
```

Expected: PASS. Refresh is transactionally scoped, converges after a stable
retry, and never creates an additional checkpoint.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository.rs src/repository/discovery.rs tests/discovery_rebuild.rs
```

### Task 8: Implement Explicit-Root Rebuild And Corrupt-Cache Degradation

**Files:**
- Modify: `src/repository.rs:369-375, 1288-1303`
- Modify: `src/repository/discovery.rs`
- Modify: `tests/discovery_rebuild.rs`
- Modify: `tests/repository_enablement.rs`

**Step 1: Write failing degraded-open and rebuild tests**

Create a non-database file at `manyhands.sqlite3` after a repository has been
enabled. Assert `RepositoryService::open_at` returns a degraded service, normal
registry/discovery operations return `IndexUnavailable`, and
`rebuild_repository(&root)` renames the corrupt file, creates a fresh database,
reconstructs the explicit root, and restores normal snapshot calls. Add a healthy
database rebuild test proving other registered roots are not searched or
silently reconstructed.

```rust
#[test]
fn corrupt_registry_opens_degraded_then_explicit_rebuild_restores_one_root() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    std::fs::write(data.path().join(REGISTRY_FILE), b"not sqlite").unwrap();

    let service = RepositoryService::open_at(data.path()).unwrap();
    assert_eq!(service.inspect(&fixture.root).unwrap_err().kind, RepositoryErrorKind::IndexUnavailable);

    let snapshot = service.rebuild_repository(&fixture.root).unwrap();

    assert_eq!(snapshot.root, fixture.root);
    assert!(service.repository_snapshot(&fixture.root).is_ok());
    assert!(corrupt_diagnostic_exists(data.path()));
}
```

**Step 2: Run rebuild tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test discovery_rebuild rebuild_
```

Expected: FAIL because `open_at` rejects corrupt SQLite and no rebuild exists.

**Step 3: Add service availability state and explicit-root rebuild**

Make the service retain its registry path plus a private ready/degraded state.
Classify only structural SQLite corruption as degraded; preserve current typed
I/O and permission errors for inaccessible data paths. While degraded, permit
only `rebuild_repository`; all registry and discovery methods return
`IndexUnavailable` before touching repository state.

`rebuild_repository` accepts the caller root, canonicalizes it, and creates or
updates local registration based on observed state. It stores `NULL` for an
unobservable configuration blob and records the configuration problem rather
than inventing an OID. On a healthy database, replace only that root's derived
rows. On a corrupt database, invoke the
`BeforeCorruptCacheReplacement` hook, rename the database to a timestamped
diagnostic filename, move matching `-wal` and `-shm` sidecars when possible,
create/migrate a fresh database, then run the normal read-only scan. If rename or
replacement cannot complete, return a typed error without deleting diagnostics.

**Step 4: Run rebuild and registry regression tests**

Run:

```sh
devenv shell -- cargo test --locked --test discovery_rebuild rebuild_
```

Expected: PASS. Corruption has one explicit recovery path; unrelated data-path
failures retain their existing behavior.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository.rs src/repository/discovery.rs tests/discovery_rebuild.rs tests/repository_enablement.rs
```

### Task 9: Prove No-Mutation And Failure Boundaries

**Files:**
- Modify: `tests/support/mod.rs`
- Modify: `tests/discovery_rebuild.rs`

**Step 1: Write failing whole-boundary tests**

Build a full fixture snapshot helper covering root/worktree file bytes, HEADs,
refs, worktree names, remotes, live-index bytes, and statuses. Before normal
refresh, healthy rebuild, corrupt rebuild, inaccessible source observation,
transaction failure, and scan-change retry, capture it and assert it is identical
after every outcome.

```rust
#[test]
fn refresh_never_mutates_canonical_or_git_state() {
    let (fixture, enabled) = repository_with_primary_and_context_content();
    let before = support::repository_and_worktree_snapshot(&fixture);

    enabled.service.refresh_repository(&fixture.root).unwrap();

    assert_eq!(support::repository_and_worktree_snapshot(&fixture), before);
}
```

**Step 2: Run no-mutation tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test discovery_rebuild no_mutation_
```

Expected: FAIL until all fixture state is captured and every recovery boundary is
covered.

**Step 3: Add minimal fixture assertions and fix violations**

Keep fixtures `git2`-only and local. Add no production bypass or mock Git
implementation. Correct implementation defects rather than weakening snapshots:
refresh/rebuild must not write canonical files, update refs, create worktrees,
change the live index, or clean up a user resource. Only the application-local
cache and its diagnostic rename are permitted changes.

**Step 4: Run all focused discovery tests**

Run:

```sh
devenv shell -- cargo test --locked --test discovery_rebuild
```

Expected: PASS with primary/context discovery, invalid-state visibility,
metadata-only cache, rebuild, retry, and no-mutation evidence complete.

**Step 5: Commit if explicitly requested**

```sh
git add tests/support/mod.rs tests/discovery_rebuild.rs
```

### Task 10: Run Full Verification And Review Cycle Boundaries

**Files:**
- Modify only if an approved authority changes:
  `docs/Cycles/wave-01-cycle-04-discovery-and-rebuild.md`

**Step 1: Run the focused Cycle target**

Run:

```sh
devenv shell -- cargo test --locked --test discovery_rebuild
```

Expected: PASS.

**Step 2: Run required repository verification**

Run:

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
```

Expected: every command exits zero.

**Step 3: Review Cycle boundaries**

Confirm the patch adds no direct GPUI dependency, system Git invocation, remote
transport, SSH behavior, filesystem watcher, automatic scheduler, Markdown body
cache, context selection, authoring/checkpoint retry, cross-process lease,
cleanup, merge, promotion, or closure behavior. Confirm a missing/corrupt cache
recovers only an explicitly supplied root and no refresh/rebuild path changes
canonical Markdown or Git state.

**Step 4: Update documentation only for an approved authority change**

Do not change approved Cycle, Wave, or RFC documents to restate implementation
details. Amend them only when a discovered contradiction receives product-owner
approval first.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository.rs src/repository/discovery.rs tests/support/mod.rs tests/discovery_rebuild.rs tests/repository_enablement.rs
```

## Completion Evidence

The Cycle is ready for implementation approval when every exit criterion in the
Cycle document has focused real-repository evidence. It is ready to declare
complete only after Task 10 confirms the focused and full required Rust
verification commands pass.
