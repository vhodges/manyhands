---
title: "Wave 01 Cycle 02 Repository Enablement Implementation Plan"
date: 2026-09-30
status: approved
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01K6Z1B1B2C3D4E5F6G7H8J9K0"
---

# Wave 01 Cycle 02 Repository Enablement Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Deliver safe local repository inspection, creation, enablement,
registration, remote configuration, and recovery using canonical configuration,
real Git repositories, and a rebuildable application-local SQLite registry.

**Architecture:** Add a small headless `manyhands::repository` service that
consumes the existing canonical configuration API. The service owns transient
`git2` operations and a repository-only SQLite table; it preserves Git and
Markdown as canonical, never contacts a remote, and uses an isolated commit
index for publication-remote configuration so unrelated user work is untouched.

**Tech Stack:** Rust 2024, `git2`/libgit2, `rusqlite` with bundled SQLite,
`directories`, `tempfile`, existing `toml`, Cargo, and Devenv/Nix.

**Commit Policy:** Do not create commits unless the user explicitly requests
one. If requested, stage only the completed task's files and use the suggested
commit message.

---

## Authorities And Fixed Decisions

Read these before implementation:

- `AGENTS.md`
- `docs/Cycles/wave-01-cycle-02-repository-enablement.md`
- `docs/RFC/canonical-content-and-comment-schema.md`
- `docs/RFC/git-workflow-and-conflict-recovery.md`
- `docs/RFC/repository-index-persistence-and-refresh.md`
- `docs/RFC/test-and-compatibility-strategy.md`

The implementation must preserve the following approved decisions:

- `git2` is the only Git backend. Do not invoke a system Git executable.
- The selected branch for a born repository must be checked out at the selected
  repository root. Return a typed recovery result when it is not; never switch
  branches in Cycle 02.
- An unborn repository repoints symbolic `HEAD` to the confirmed branch before
  its initialization commit.
- Only the SQLite `repositories` table belongs in Cycle 02. Cycle 04 adds
  context, item, problem, and operation-recovery tables through migrations.
- A publication remote needs SSH-compatible fetch and effective push URLs. The
  effective push URL is `pushurl`, falling back to the fetch URL.
- Initialization requires a clean, non-conflicted primary worktree. Publication
  configuration may coexist with unrelated dirty, staged, untracked, deleted,
  or conflicted paths, but the configuration path itself must match `HEAD`.
- Publication configuration commits use a temporary index based on `HEAD`, not
  the live index. They contain only `.manyhands/config.toml`.
- The deterministic configuration commit subjects are `Initialize Manyhands`
  and `Configure Manyhands publication remote`.
- A successful Git commit always wins over a failed or stale local registry
  write. Retrying reconciles state instead of creating another commit.

## Public Contract

Keep helpers private. Export only the service, requests/results, inspection
values, remote values, and typed errors that later front ends need.

```rust
pub const REGISTRY_FILE: &str = "manyhands.sqlite3";

pub struct RepositoryService { /* database path and private operation hook */ }

impl RepositoryService {
    pub fn open_default() -> Result<Self, RepositoryError>;
    pub fn open_at(data_directory: &std::path::Path) -> Result<Self, RepositoryError>;
    pub fn inspect(
        &self,
        root: &std::path::Path,
    ) -> Result<RepositoryInspection, RepositoryError>;
    pub fn create_and_enable(
        &self,
        request: CreateRepositoryRequest,
    ) -> Result<EnableRepositoryOutcome, RepositoryError>;
    pub fn enable(
        &self,
        request: EnableRepositoryRequest,
    ) -> Result<EnableRepositoryOutcome, RepositoryError>;
    pub fn remove_registration(
        &self,
        root: &std::path::Path,
    ) -> Result<RemoveRegistrationOutcome, RepositoryError>;
    pub fn list_remotes(
        &self,
        root: &std::path::Path,
    ) -> Result<Vec<RemoteInfo>, RepositoryError>;
    pub fn add_remote(&self, request: AddRemoteRequest) -> Result<RemoteOutcome, RepositoryError>;
    pub fn remove_remote(
        &self,
        root: &std::path::Path,
        name: &str,
    ) -> Result<RemoteOutcome, RepositoryError>;
    pub fn set_publication_remote(
        &self,
        request: SetPublicationRemoteRequest,
    ) -> Result<PublicationRemoteOutcome, RepositoryError>;
}

pub struct CommitIdentity {
    pub name: String,
    pub email: String,
}

pub struct CreateRepositoryRequest {
    pub root: std::path::PathBuf,
    pub primary_branch: String,
    pub identity: Option<CommitIdentity>,
}

pub struct EnableRepositoryRequest {
    pub root: std::path::PathBuf,
    pub primary_branch: String,
    pub identity: Option<CommitIdentity>,
}

pub struct AddRemoteRequest {
    pub root: std::path::PathBuf,
    pub name: String,
    pub url: String,
}

pub struct SetPublicationRemoteRequest {
    pub root: std::path::PathBuf,
    pub name: Option<String>,
}

pub struct RepositoryInspection {
    pub root: std::path::PathBuf,
    pub head_branch: Option<String>,
    pub local_branches: Vec<String>,
    pub configuration: ConfigurationInspection,
    pub identity: IdentityInspection,
    pub remotes: Vec<RemoteInfo>,
}

pub enum ConfigurationInspection {
    Missing,
    Valid(manyhands::canonical::RepositoryConfig),
    Invalid(manyhands::canonical::ValidationProblem),
}

pub enum IdentityInspection {
    Available,
    Required,
}

pub struct RemoteInfo {
    pub name: String,
    pub fetch_url: String,
    pub push_url: String,
    pub publication_eligible: bool,
}

pub enum EnableRepositoryOutcome {
    Enabled { commit_oid: git2::Oid },
    AlreadyEnabled,
    IdentityRequired,
    RegistrationPending { commit_oid: git2::Oid },
}

pub enum RemoteOutcome {
    Changed,
    NoChange,
}

pub enum PublicationRemoteOutcome {
    Changed { commit_oid: git2::Oid },
    NoChange,
    RegistrationPending { commit_oid: git2::Oid },
}

pub enum RemoveRegistrationOutcome {
    Removed,
    NotRegistered,
}

pub struct RepositoryError {
    pub root: Option<std::path::PathBuf>,
    pub operation: RepositoryOperation,
    pub kind: RepositoryErrorKind,
}
```

`RepositoryOperation` and `RepositoryErrorKind` must distinguish at least path
validation, inaccessible/non-Git/bare repository, detached or wrong checked-out
branch, dirty or conflicted initialization root, invalid configuration, invalid
or unavailable publication remote, selected-remote removal, identity rejection,
I/O, SQLite, Git, and injected-failure conditions. Do not expose `git2::Error`
or `rusqlite::Error` as the caller-facing error type.

The test-only service constructor and failure hook may be `#[doc(hidden)]`, but
they must be available to `tests/repository_enablement.rs`. Production creation
must use a no-op hook and the platform application-data directory.

### Task 1: Add Runtime Dependencies And Repository Test Target

**Files:**
- Modify: `Cargo.toml:13-23`
- Modify: `Cargo.lock`
- Modify: `src/lib.rs:1-3`
- Create: `src/repository.rs`
- Create: `tests/repository_enablement.rs`

**Step 1: Write the failing service-construction test**

Create `tests/repository_enablement.rs` with a temporary data directory and a
test that imports `manyhands::repository::{RepositoryService, REGISTRY_FILE}`.
It should assert that `RepositoryService::open_at(tempdir.path())` creates a
database at `tempdir.path().join(REGISTRY_FILE)` without creating a repository.

```rust
#[test]
fn service_creates_its_local_registry_in_the_supplied_data_directory() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();

    assert!(data.path().join(REGISTRY_FILE).is_file());
    drop(service);
}
```

**Step 2: Run the focused test to verify it fails**

Run:

```sh
devenv shell -- cargo test --locked --test repository_enablement service_creates_its_local_registry_in_the_supplied_data_directory
```

Expected: FAIL because the target or public repository module does not exist.

**Step 3: Add the headless dependencies and module skeleton**

Move `git2 = "0.20"` and `tempfile = "3"` from `[dev-dependencies]` to
`[dependencies]`. Add compatible direct dependencies:

```toml
directories = "5"
rusqlite = { version = "0.32", features = ["bundled"] }
```

Leave `tempfile` available to integration tests through normal dependencies;
do not duplicate it in `[dev-dependencies]`. Export `pub mod repository;` from
`src/lib.rs`. Add `REGISTRY_FILE`, `RepositoryService`, and `open_at` in
`src/repository.rs`. `open_at` creates the supplied directory when it does not
exist, opens `manyhands.sqlite3`, and closes the connection before returning.

**Step 4: Update the lockfile and run the construction test**

Run:

```sh
devenv shell -- cargo generate-lockfile
```

Run:

```sh
devenv shell -- cargo test --locked --test repository_enablement service_creates_its_local_registry_in_the_supplied_data_directory
```

Expected: PASS.

**Step 5: Commit if explicitly requested**

```sh
git add Cargo.toml Cargo.lock src/lib.rs src/repository.rs tests/repository_enablement.rs
git commit -m "feat: add repository service foundation"
```

### Task 2: Initialize The Repository-Only SQLite Registry

**Files:**
- Modify: `src/repository.rs`
- Modify: `tests/repository_enablement.rs`

**Step 1: Write failing database-contract tests**

Add tests that open the service with a temporary data directory, then open the
created database directly with `rusqlite`. Assert:

- `repositories` exists and no future Cycle 04 tables exist.
- `root_path` has a uniqueness constraint.
- The table has `id`, `root_path`, `enabled_at`, `accessibility`,
  `config_blob_oid`, and `refresh_required` columns.
- A new connection reports `foreign_keys = 1`, `journal_mode = wal`, and a
  positive bounded `busy_timeout`.

**Step 2: Run the registry tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test repository_enablement registry_
```

Expected: FAIL because migration and connection configuration are absent.

**Step 3: Implement connection setup and migration 1**

Add private `open_registry` and `migrate_registry` helpers. On every opened
connection, enable foreign keys, set WAL journal mode, and set a named bounded
busy timeout constant. Run the migration in one transaction using
`CREATE TABLE IF NOT EXISTS repositories (...)`, with:

```sql
id INTEGER PRIMARY KEY,
root_path TEXT NOT NULL UNIQUE,
enabled_at INTEGER NOT NULL,
accessibility TEXT NOT NULL,
config_blob_oid TEXT NOT NULL,
refresh_required INTEGER NOT NULL CHECK (refresh_required IN (0, 1))
```

Use integer Unix timestamps for `enabled_at`, `accessible` as the initial
accessibility value, and text for the Git blob OID. Do not create placeholder
Cycle 04 tables or store configuration text.

**Step 4: Run the registry tests to verify they pass**

Run:

```sh
devenv shell -- cargo test --locked --test repository_enablement registry_
```

Expected: PASS.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository.rs tests/repository_enablement.rs
git commit -m "feat: initialize local repository registry"
```

### Task 3: Model Errors, Inspection, And Identity Resolution

**Files:**
- Modify: `src/repository.rs`
- Modify: `tests/support/mod.rs`
- Modify: `tests/repository_enablement.rs`

**Step 1: Write failing inspection and identity tests**

Extend `tests/support/mod.rs` with fixtures for a born repository on `main`, an
unborn repository, a bare repository, and a repository with local identity
removed. Add integration tests that assert:

- Inspection reports a canonical working-directory root, `main`, its local
  branch list, local identity availability, and no remotes for `born_repository`.
- A non-Git path and bare repository return different typed errors without
  mutation.
- A repository selected through a non-root path and a detached `HEAD` return
  recoverable inspection/enablement errors.
- A local name/email produces `IdentityInspection::Available`.
- An isolated effective-config fixture lacking either field reports
  `IdentityInspection::Required`; the test must not read developer global Git
  configuration.

Test the pure identity resolver separately with an in-memory `git2::Config` for
the global-level fallback behavior. The production resolver may use
`Repository::config`; the test resolver must use only the supplied test config.

**Step 2: Run the inspection tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test repository_enablement inspect_
devenv shell -- cargo test --locked --test repository_enablement identity_
```

Expected: FAIL because inspection values, error categories, and identity logic
are absent.

**Step 3: Implement typed inspection and identity helpers**

Implement the public request/result/error values in the public contract. Keep
these helpers private:

- `canonical_repository_root` canonicalizes an existing path, opens it with
  `git2`, rejects bare repositories, and verifies the selected path is exactly
  the working-directory root.
- `checked_out_branch` returns the symbolic local `HEAD` shorthand and rejects
  detached `HEAD` for lifecycle operations.
- `worktree_state` distinguishes conflicted status from other modified, staged,
  untracked, or deleted status.
- `read_configuration` reads `canonical::CONFIG_PATH` without rewriting it and
  returns missing, valid, or the original `ValidationProblem`.
- `resolve_identity` uses complete repository values first, then complete
  effective/global values. It never combines a name from one level with an
  email from another. A caller-supplied identity must have nonempty,
  NUL-free name and email before it can be written locally.

Use a private identity-config provider with a production implementation based
on the repository's effective config and a supplied isolated implementation for
tests. This keeps integration tests independent of developer global config.

**Step 4: Run the inspection tests to verify they pass**

Run:

```sh
devenv shell -- cargo test --locked --test repository_enablement inspect_
devenv shell -- cargo test --locked --test repository_enablement identity_
```

Expected: PASS.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository.rs tests/support/mod.rs tests/repository_enablement.rs
git commit -m "feat: inspect local repositories safely"
```

### Task 4: Enable Existing Born And Unborn Repositories

**Files:**
- Modify: `src/repository.rs`
- Modify: `tests/support/mod.rs`
- Modify: `tests/repository_enablement.rs`

**Step 1: Write failing enablement tests**

Add fixture helpers to read the exact exclude bytes, tracked configuration,
`HEAD` commit, and registered database row. Add tests for:

- Enabling a clean born `main` repository writes valid canonical configuration,
  adds exactly one `.manyhands/worktrees/` exclude line, and creates one
  `Initialize Manyhands` commit containing only `.manyhands/config.toml`.
- Enabling an unborn repository on a requested `trunk` creates its first commit
  on `trunk`, with valid configuration and no `master` branch.
- A requested existing branch not checked out at the root returns the typed
  checked-out-branch recovery error and leaves configuration, exclude, refs,
  index, and registry unchanged.
- Dirty, staged, untracked, deleted, or conflicted primary worktree state
  rejects initialization without mutation.
- An identity-required result occurs before exclude/configuration/commit writes.
- A caller-supplied confirmed identity is written only to the repository config
  and is the author and committer of the initialization commit.

**Step 2: Run the enablement tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test repository_enablement enable_
```

Expected: FAIL because enablement and initialization commit behavior are absent.

**Step 3: Implement initialization transaction behavior**

Implement `enable` with this exact control flow:

1. Open and inspect the selected non-bare root.
2. For a born repository, require a clean, non-conflicted worktree and require
   `HEAD` to equal the requested local primary branch. For an unborn repository,
   validate the requested name and set symbolic `HEAD` to
   `refs/heads/<primary-branch>`.
3. Resolve or validate the identity before writing any repository state.
4. When configuration is absent, snapshot the exact exclude bytes, append the
   one required exclusion only if no delimiter-normalized line matches, and use
   `tempfile::NamedTempFile` plus rename for configuration replacement.
5. Use an in-memory `git2::Index` seeded from the current `HEAD` tree, add only
   `canonical::CONFIG_PATH`, write its tree, and commit to `HEAD` with
   `Signature::now` and subject `Initialize Manyhands`.
6. On any error before commit success, restore the prior configuration presence
   and exact exclude bytes. Do not write the live index. Unreferenced Git
   objects created before a failed commit are harmless and must not trigger a
   destructive repository cleanup.
7. Delegate the successful canonical state to the registry helper in Task 5.

For an existing valid configuration, verify its configured branch matches the
request and that any selected remote is eligible. Do not rewrite configuration
or create another initialization commit; ensure the exclude rule and continue
to registration. If this operation added the exclude rule but registration then
fails without creating a commit in the current attempt, restore the prior exact
exclude bytes before returning the failure.

**Step 4: Run the enablement tests to verify they pass**

Run:

```sh
devenv shell -- cargo test --locked --test repository_enablement enable_
```

Expected: PASS.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository.rs tests/support/mod.rs tests/repository_enablement.rs
git commit -m "feat: enable local Manyhands repositories"
```

### Task 5: Persist And Remove Repository Registrations

**Files:**
- Modify: `src/repository.rs`
- Modify: `tests/repository_enablement.rs`

**Step 1: Write failing registration tests**

Add tests that enable a repository and inspect the registry database directly.
Assert that the row contains its canonical root, `accessible`, a nonempty
committed configuration blob OID matching the `HEAD` tree entry, and
`refresh_required = 1`. Then assert:

- Re-enabling the same conforming repository creates neither a second commit
  nor a duplicate row.
- Canonically equivalent paths cannot create duplicate rows.
- Removing a registration deletes the row and returns `NotRegistered` on retry.
- Removal leaves tracked configuration, exclude bytes, remotes, `HEAD`, local
  branches, and worktree files unchanged.

**Step 2: Run the registration tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test repository_enablement registration_
```

Expected: FAIL because registry reconciliation and removal are absent.

**Step 3: Implement registry reconciliation**

Add private helpers that:

- Obtain the committed configuration blob OID from the primary `HEAD` tree,
  never from a copied configuration body.
- Insert or update the one canonical-root row in a SQLite transaction, setting
  accessibility to `accessible`, updating the observed OID, and setting
  `refresh_required` to `1`.
- Classify a unique-constraint collision by re-reading the row and reconcile it
  rather than exposing a raw SQLite error.
- Delete by canonical root only for `remove_registration`.

If registry work fails after a configuration commit, return the corresponding
`RegistrationPending` outcome with the already-created commit OID. On retry,
recognize the valid existing configuration and register it without another
commit.

**Step 4: Run the registration tests to verify they pass**

Run:

```sh
devenv shell -- cargo test --locked --test repository_enablement registration_
```

Expected: PASS.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository.rs tests/repository_enablement.rs
git commit -m "feat: register enabled repositories locally"
```

### Task 6: Create New Local Repositories Safely

**Files:**
- Modify: `src/repository.rs`
- Modify: `tests/support/mod.rs`
- Modify: `tests/repository_enablement.rs`

**Step 1: Write failing creation tests**

Use temporary parent directories to add tests that:

- Create and enable a nonexistent `project` directory on confirmed `main`, with
  a local config, one initialization commit, and one registry row.
- Create and enable an empty existing target directory with the same result.
- Reject a nonempty target, a missing parent, invalid branch spelling, and
  identity-required request without creating a repository or target directory.
- Inject `BeforeRepositoryInitialization` after target-directory creation and
  assert that only an empty directory created by this operation is removed;
  pre-existing empty targets remain present.

**Step 2: Run the creation tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test repository_enablement create_
```

Expected: FAIL because local creation behavior is absent.

**Step 3: Implement `create_and_enable`**

Validate primary-branch syntax using the canonical configuration serializer and
validate identity before filesystem mutation. Canonicalize the existing parent,
allow only a nonexistent or empty selected target, create the target when
needed, initialize with `git2::RepositoryInitOptions::initial_head`, then reuse
the unborn enablement path. Track whether this operation created the directory
and repository. If a later pre-commit operation fails, remove only the empty
directory it created; never remove a caller-supplied path.

**Step 4: Run the creation tests to verify they pass**

Run:

```sh
devenv shell -- cargo test --locked --test repository_enablement create_
```

Expected: PASS.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository.rs tests/support/mod.rs tests/repository_enablement.rs
git commit -m "feat: create and enable local repositories"
```

### Task 7: Manage Local Remotes And Publication Selection

**Files:**
- Modify: `src/repository.rs`
- Modify: `tests/support/mod.rs`
- Modify: `tests/repository_enablement.rs`

**Step 1: Write failing remote-management tests**

Enable a born `main` fixture, then add tests for:

- Listing a new remote's name, fetch URL, effective push URL, and eligibility
  without invoking a transport callback or contacting an unreachable endpoint.
- Adding a remote, returning `NoChange` when the same named endpoint already
  exists, and returning a typed conflict when the same name has another URL.
- Removing an unselected remote and returning `NoChange` when it is absent.
- Accepting publication selection only when both fetch and effective push URLs
  are SSH-compatible, including `ssh://` and scp-like syntax.
- Rejecting HTTP(S), `file`, local paths, malformed URLs, SSH fetch plus HTTPS
  push, and HTTPS fetch plus SSH push.
- Rejecting removal of the selected remote until selection is cleared.
- Selecting, replacing, and clearing a remote changes only canonical
  configuration and creates one `Configure Manyhands publication remote` commit
  per effective change.
- Selecting an already-selected remote returns `NoChange` and makes no commit.

**Step 2: Run the remote tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test repository_enablement remote_
devenv shell -- cargo test --locked --test repository_enablement publication_
```

Expected: FAIL because remote operations and SSH classification are absent.

**Step 3: Implement local remote and URL logic**

Implement `list_remotes`, `add_remote`, and `remove_remote` with libgit2 local
configuration APIs only. Every remote mutation must update the registered row's
`refresh_required` flag; if this post-mutation registry update fails, return a
typed recoverable error that states the Git remote is already authoritative.

Add private URL classification that accepts exactly `ssh://` URLs and valid
scp-like `<host>:<path>` or `<user>@<host>:<path>` URLs without an alternate
scheme. It must reject empty hosts/paths, Windows drive prefixes, HTTP(S),
`file`, and ordinary local paths. Evaluate both `Remote::url()` and
`Remote::pushurl().unwrap_or(fetch_url)`.

**Step 4: Implement isolated configuration checkpointing**

Before changing publication selection, require a valid configuration, primary
`HEAD` on its configured branch, and a configuration worktree path whose status
is clean relative to `HEAD`. Preserve unrelated status entries. Parse the
existing config, set `publication_remote`, serialize through
`canonical::serialize_repository_config`, atomically replace only the config
file, then build a temporary `git2::Index` from `HEAD`'s tree plus that one path.
Commit it to `HEAD` with `Configure Manyhands publication remote` and no live
index write. Restore the exact original configuration bytes if writing or
committing fails before a commit exists. After commit, refresh the registry OID
and invalidation flag or return `RegistrationPending`.

**Step 5: Run the remote tests to verify they pass**

Run:

```sh
devenv shell -- cargo test --locked --test repository_enablement remote_
devenv shell -- cargo test --locked --test repository_enablement publication_
```

Expected: PASS.

**Step 6: Commit if explicitly requested**

```sh
git add src/repository.rs tests/support/mod.rs tests/repository_enablement.rs
git commit -m "feat: manage local publication remotes"
```

### Task 8: Inject Failures And Prove Recovery Boundaries

**Files:**
- Modify: `src/repository.rs`
- Modify: `tests/support/mod.rs`
- Modify: `tests/repository_enablement.rs`

**Step 1: Write failing failure-injection tests**

Add a `FailOnce` operation hook fixture with these points:

```rust
pub enum FailurePoint {
    BeforeRepositoryInitialization,
    BeforeConfigurationWrite,
    BeforeInitializationCommit,
    BeforePublicationConfigurationCommit,
    BeforeRegistryWrite,
}
```

Write cases that compare file bytes, refs, commit counts, index state, remotes,
and database rows before and after each injected error:

- Configuration-write and initialization-commit failures restore configuration
  absence and exact exclude bytes, leave no registration, and create no ref.
- A pre-initialization failure removes only the newly created empty target
  directory and never removes a caller-supplied directory.
- Publication-configuration-commit failure restores the prior configuration,
  leaves the original remote selection and commit count intact, and preserves
  unrelated live index entries.
- Registry-write failure after initialization or publication commit leaves the
  valid commit/configuration authoritative and reports the corresponding commit
  OID as pending.
- Retrying every failure completes only unfinished work and creates no duplicate
  configuration, commit, remote, or registry row.

**Step 2: Run the recovery tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test repository_enablement recovery_
```

Expected: FAIL because named failure points and reconciliation are absent.

**Step 3: Implement the narrow operation hook**

Define a small hook trait or equivalent injected closure used only at the named
points above. The default implementation always succeeds. Keep it out of normal
operation results and do not substitute filesystem, SQLite, or Git mocks for
real test state. Ensure recovery code checks actual configuration, ref, and
registry state before deciding whether to restore, register, no-op, or commit.

**Step 4: Run the recovery tests to verify they pass**

Run:

```sh
devenv shell -- cargo test --locked --test repository_enablement recovery_
```

Expected: PASS.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository.rs tests/support/mod.rs tests/repository_enablement.rs
git commit -m "test: cover repository enablement recovery"
```

### Task 9: Run Cycle Verification And Update Documentation Only If Needed

**Files:**
- Modify only if an approved design decision changed: `docs/Cycles/wave-01-cycle-02-repository-enablement.md`
- Modify: `Cargo.lock` if dependency resolution changes it

**Step 1: Run the full focused Cycle target**

Run:

```sh
devenv shell -- cargo test --locked --test repository_enablement
```

Expected: PASS with all inspection, creation, enablement, registration, remote,
and recovery cases passing.

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

Confirm the patch introduced no direct GPUI dependency, system Git invocation,
remote transport, SSH credential handling, worktree provisioning, content scan,
future SQLite tables, or operation lease. Confirm that all runtime dependencies
remain headless and `Cargo.lock` reflects only approved additions.

**Step 4: Update documentation only for an approved change**

Do not change the approved Cycle document merely to restate implementation
details. Amend it only if tests uncover a genuine authority conflict and the
product owner approves the amendment first.

**Step 5: Commit if explicitly requested**

```sh
git add Cargo.toml Cargo.lock src/lib.rs src/repository.rs tests/support/mod.rs tests/repository_enablement.rs
git commit -m "feat: enable local Manyhands repositories"
```

## Completion Evidence

The Cycle is ready for implementation approval when the design choices above
remain accepted and the plan has a task for every Cycle 02 exit criterion. The
Cycle is ready to declare complete only after Task 9's focused and full command
output confirms all commands pass.
