---
title: "Wave 02 Cycle 01 Shared-Key Registry Implementation Plan"
date: 2026-10-04
status: approved
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01M43H1YNZ5P9ZKVAR8VHEA3F9"
---

# Wave 02 Cycle 01 Shared-Key Registry Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Deliver a user-local, non-secret SSH-key metadata registry with one
selected shared key and typed recovery outcomes, without reading private-key
contents or performing a network operation.

**Architecture:** Extend the existing headless `RepositoryService` and its
application-local SQLite registry. Add an application-global `shared_ssh_keys`
table with a private-source-path uniqueness constraint and a partial unique
selection index; expose transactional typed APIs that never alter Git or
canonical repository content.

**Tech Stack:** Rust 2024, `rusqlite`, `ulid`, `ssh-key` 0.6.7 for public-key
parsing only, existing `fs4` coordination, Cargo, and Devenv/Nix.

**Commit Policy:** Do not create commits unless the user explicitly requests
one. If requested, stage only the completed task's files and use the suggested
commit message.

---

## Authorities And Non-Negotiable Decisions

Read before implementation:

- `AGENTS.md`
- `docs/Cycles/wave-02-cycle-01-shared-key-registry.md`
- `docs/plans/2026-10-04-wave-02-cycle-01-shared-key-registry-design.md`
- `docs/RFC/authentication-and-credential-handling.md`
- `docs/RFC/repository-index-persistence-and-refresh.md`
- `docs/RFC/test-and-compatibility-strategy.md`

The implementation must preserve these approved decisions:

- The registry is user-local and app-global, never repository-local or tracked.
- A registration is unique only by an absolute, lexically normalized private-key
  source path. Labels and parsed public fingerprints may repeat.
- Missing, inaccessible, or non-regular private sources remain registered. Do
  not call `canonicalize`, open, read, parse, unlock, copy, or delete a private
  key in Cycle 01.
- An optional companion public key is parsed best effort. Persist only its
  SHA-256 fingerprint, never its body or comment; failure remains recoverable.
- Private-key format and backend compatibility validation belong to Cycle 03.
- At most one registration is selected. Replacing or clearing selection is
  explicit and atomic; selected keys must be deselected or replaced first.
- Generated deletion returns only a fresh, ephemeral confirmation preflight.
  Cycle 01 stores no authorization and writes or removes no key file.
- No API may change `.manyhands/config.toml`, Markdown, Git configuration,
  remotes, branches, worktrees, or network state.
- Do not expose parser or filesystem error text that might contain key content.

## Public Contract

Implement these types in `src/repository.rs`; keep conversion, SQL, source
observation, and OpenSSH parsing helpers private.

```rust
pub struct SharedKeyId(ulid::Ulid);

pub enum SharedKeyOwnership {
    Imported,
    Generated,
}

pub enum PrivateKeySourceState {
    Available,
    Missing,
    Unavailable,
}

pub enum PublicKeyMetadataState {
    NotProvided,
    FingerprintAvailable,
    Unavailable,
}

pub struct SharedKeyRegistration {
    pub id: SharedKeyId,
    pub label: String,
    pub ownership: SharedKeyOwnership,
    pub private_key_path: PathBuf,
    pub public_key_path: Option<PathBuf>,
    pub public_key_fingerprint: Option<String>,
    pub private_source_state: PrivateKeySourceState,
    pub public_metadata_state: PublicKeyMetadataState,
    pub selected: bool,
}

pub struct RegisterSharedKeyRequest {
    pub label: String,
    pub ownership: SharedKeyOwnership,
    pub private_key_path: PathBuf,
    pub public_key_path: Option<PathBuf>,
}

pub enum RegisterSharedKeyOutcome {
    Registered(SharedKeyRegistration),
    SourceAlreadyRegistered { existing: SharedKeyId },
}

pub enum SharedKeySelectionOutcome {
    Selected(SharedKeyRegistration),
    Cleared,
    AlreadyCleared,
}

pub enum UnregisterSharedKeyOutcome {
    Unregistered,
    NotRegistered,
    SelectedKeyMustBeCleared,
}

pub enum GeneratedKeyDeletionPreflight {
    ConfirmationRequired(SharedKeyRegistration),
    NotRegistered,
    ImportedKey,
    SelectedKeyMustBeCleared,
}
```

Add `register_shared_key`, `list_shared_keys`, `select_shared_key`,
`clear_shared_key_selection`, `unregister_shared_key`, and
`preflight_generated_key_deletion` to `RepositoryService`. Add narrowly named
`RepositoryOperation` and `RepositoryErrorKind` variants for invalid shared-key
metadata, an invalid source path, and an unavailable registry. Keep errors
free of source contents and parser text.

### Task 1: Re-establish The Wave 01 Entry Gate

**Files:**
- Modify: `.manyhands/comments/01K7F6H9J2N4Q6S8V0X2Z4B6D9/<new-comment>.md`

**Step 1: Run the required static and test verification from this ticket worktree**

Run:

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
```

Expected: every command exits zero. Stop the Cycle if any command fails; diagnose
the baseline separately rather than attributing it to Cycle 01.

**Step 2: Record the result as a canonical ticket comment**

Create a comment at
`.manyhands/comments/01K7F6H9J2N4Q6S8V0X2Z4B6D9/<new-comment>.md` with a new
ULID and UTC timestamp. Record the exact commands, success, or failure category
only. Never include environment variables, user paths, credentials, or test
fixture private-key content.

**Step 3: Do not commit without authorization**

Suggested commit if the user later requests one:

```text
chore: record Wave 1 verification gate
```

### Task 2: Specify The Schema With A Failing Migration Test

**Files:**
- Modify: `tests/repository_enablement.rs:431-552`
- Modify: `tests/discovery_rebuild.rs:2584-2684`
- Modify: `src/repository/discovery.rs:1088-1243`

**Step 1: Add failing schema assertions**

Extend `registry_creates_repository_and_discovery_metadata_schema` to expect
`shared_ssh_keys`. Assert its columns and the two database constraints:

```sql
private_key_path TEXT NOT NULL UNIQUE
```

and a partial unique index equivalent to:

```sql
CREATE UNIQUE INDEX shared_ssh_keys_one_selected_idx
    ON shared_ssh_keys(selected) WHERE selected = 1;
```

Add a migration fixture that creates the current pre-Cycle-01 registry schema,
opens `RepositoryService`, and asserts existing repository rows survive while
the new key table and index exist.

**Step 2: Run the focused tests to verify the failure**

Run:

```sh
devenv shell -- cargo test --locked --test repository_enablement registry_creates_repository_and_discovery_metadata_schema
devenv shell -- cargo test --locked --test discovery_rebuild migration_is_idempotent
```

Expected: FAIL because `shared_ssh_keys` and its selected-key index do not yet
exist.

**Step 3: Add the idempotent application-global table migration**

In `migrate_registry`, create this table and index in the existing transaction:

```sql
CREATE TABLE IF NOT EXISTS shared_ssh_keys (
    id TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    ownership TEXT NOT NULL CHECK (ownership IN ('imported', 'generated')),
    private_key_path TEXT NOT NULL UNIQUE,
    public_key_path TEXT,
    public_key_fingerprint TEXT,
    private_source_state TEXT NOT NULL CHECK (
        private_source_state IN ('available', 'missing', 'unavailable')
    ),
    public_metadata_state TEXT NOT NULL CHECK (
        public_metadata_state IN ('not-provided', 'available', 'unavailable')
    ),
    selected INTEGER NOT NULL DEFAULT 0 CHECK (selected IN (0, 1))
);
CREATE UNIQUE INDEX IF NOT EXISTS shared_ssh_keys_one_selected_idx
    ON shared_ssh_keys(selected) WHERE selected = 1;
```

Do not add a `repository_id` foreign key, a private-key body, passphrase,
deletion-intent column, or a generic error-text column.

**Step 4: Run the focused tests to verify they pass**

Run the two commands from Step 2.

Expected: PASS, including a second service open that leaves the schema unchanged.

**Step 5: Do not commit without authorization**

Suggested commit if requested:

```text
feat: add shared key registry schema
```

### Task 3: Define The Public API And Prove Path-Only Registration

**Files:**
- Modify: `src/repository.rs:1-196, 643-850, 2862-2903`
- Create: `tests/shared_key_registry.rs`

**Step 1: Write failing registration tests**

Create `tests/shared_key_registry.rs` with `mod support;`. Add tests that use
`RepositoryService::open_at(tempdir)` and assert:

- registration returns a generated opaque ID and retains ownership, label, and
  normalized absolute source path;
- `.` and `..` spelling variants of the same absolute source return
  `SourceAlreadyRegistered` with the first ID;
- a relative path and blank label return typed validation errors;
- two different source paths may use the same label;
- reopening the service returns the durable registration with no selected key.

Use a private fixture file whose bytes contain
`CYCLE01_PRIVATE_SENTINEL_DO_NOT_PERSIST`; only pass its path to the service.

**Step 2: Run the new test target to verify it fails**

Run:

```sh
devenv shell -- cargo test --locked --test shared_key_registry registration
```

Expected: FAIL to compile because the shared-key public types and methods do not
exist.

**Step 3: Add types and private metadata helpers**

Add the public contract above near existing request/outcome types. Implement
canonical uppercase `SharedKeyId` parsing and display using the same pattern as
`OperationId` at `src/repository.rs:106-143`.

Add a private lexical normalizer that rejects non-absolute paths, preserves the
platform prefix and root, ignores `.` components, and collapses `..` without
calling `std::fs::canonicalize`. Convert only valid Unicode paths to SQLite text;
return the typed invalid-path error otherwise. Use `std::fs::metadata` only to
map the private source to `Available`, `Missing`, or `Unavailable`; never open
the private source or include a filesystem error string in the result.

Add a private write operation helper that obtains the existing application-data
cache write lock, opens and migrates the registry, and executes one SQLite
transaction. For app-global operations, use the registry data directory solely
as lock/error context; do not discover a Git repository or acquire a Git lease.

Implement `register_shared_key` with `INSERT ... ON CONFLICT(private_key_path)`
or an equivalent transaction. Return the existing opaque ID on conflict rather
than reusing or replacing its label, ownership, public metadata, or selection.

**Step 4: Run the registration tests to verify they pass**

Run the command from Step 2.

Expected: PASS. Inspect the fixture afterwards to confirm its private sentinel
file is byte-for-byte unchanged.

**Step 5: Do not commit without authorization**

Suggested commit if requested:

```text
feat: register shared SSH key metadata
```

### Task 4: Add Atomic Selection And Safe Removal Outcomes

**Files:**
- Modify: `src/repository.rs:184-196, 643-850`
- Modify: `tests/shared_key_registry.rs`

**Step 1: Write failing lifecycle tests**

Add tests that register two paths and prove:

- selecting the first, then the second, leaves exactly the second selected after
  a service reopen;
- direct SQL insertion of a second selected row fails because the partial unique
  index enforces the invariant;
- clearing an existing selection returns `Cleared`, while clearing none returns
  `AlreadyCleared`;
- unregistering the selected key returns `SelectedKeyMustBeCleared` and keeps
  the row; deselecting then unregistering removes only that row;
- unregistering a missing ID returns `NotRegistered`.

**Step 2: Run the focused tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test shared_key_registry selection
devenv shell -- cargo test --locked --test shared_key_registry unregister
```

Expected: FAIL to compile until the selection, clearing, and removal APIs exist.

**Step 3: Implement the transactional lifecycle methods**

Implement selection replacement in a single immediate transaction:

```sql
UPDATE shared_ssh_keys SET selected = 0 WHERE selected = 1;
UPDATE shared_ssh_keys SET selected = 1 WHERE id = ?1;
```

Roll back if the requested ID does not exist. Implement clearing as one update.
For unregistration, query the row in the same transaction and return
`SelectedKeyMustBeCleared` without deleting if `selected = 1`; otherwise delete
the metadata row only. Neither method may touch the source path, public-key
path, Git configuration, or repository configuration.

**Step 4: Run the focused tests to verify they pass**

Run the commands from Step 2.

Expected: PASS, including the direct SQLite uniqueness assertion.

**Step 5: Do not commit without authorization**

Suggested commit if requested:

```text
feat: manage shared key selection
```

### Task 5: Add Best-Effort Public Metadata, Deletion Preflight, And Privacy Evidence

**Files:**
- Modify: `Cargo.toml:13-26`
- Modify: `Cargo.lock`
- Modify: `src/repository.rs`
- Modify: `tests/shared_key_registry.rs`

**Step 1: Write failing recovery and privacy tests**

Add tests for:

- a valid OpenSSH companion public-key fixture yields a stored SHA-256
  fingerprint but not its comment;
- missing and malformed companion files retain registration with
  `PublicKeyMetadataState::Unavailable` and no fingerprint;
- missing and non-regular private sources retain registration with a non-secret
  unavailable state, not a private-format error;
- imported unregistration leaves both the private fixture and companion bytes
  unchanged;
- an unselected generated registration returns `ConfirmationRequired`, while an
  imported registration returns `ImportedKey` and a selected generated key
  returns `SelectedKeyMustBeCleared`;
- raw `manyhands.sqlite3` and an existing `manyhands.sqlite3-wal` do not contain
  the private sentinel, a passphrase sentinel, or the companion public-key
  comment.

**Step 2: Run the focused tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test shared_key_registry public_metadata
devenv shell -- cargo test --locked --test shared_key_registry recovery
devenv shell -- cargo test --locked --test shared_key_registry secret
```

Expected: FAIL because public-key parsing, source observations, deletion
preflight, and privacy assertions are not implemented.

**Step 3: Add the public-key-only dependency and behavior**

Add this dependency and update the lockfile through Cargo:

```toml
ssh-key = "0.6.7"
```

Read only the optional companion with
`ssh_key::PublicKey::read_openssh_file`. Persist
`public_key.fingerprint(Default::default()).to_string()` on success. Discard the
public key object and every parser error; map all failure paths to the fixed
`Unavailable` public-metadata state and concise guidance. Do not add a private
key parser or any public-key body/comment column.

Implement `preflight_generated_key_deletion` as a read-only query. It returns
`ConfirmationRequired` only for an existing, unselected generated registration;
it must not create an operation record, change selection, retain confirmation,
or call a filesystem mutation API.

**Step 4: Run the focused tests to verify they pass**

Run the three commands from Step 2.

Expected: PASS. Review failure output to ensure no sentinel or public-key comment
is printed.

**Step 5: Do not commit without authorization**

Suggested commit if requested:

```text
feat: report shared key metadata safely
```

### Task 6: Run The Complete Cycle Verification And Record Evidence

**Files:**
- Modify: `.manyhands/comments/01K7F6H9J2N4Q6S8V0X2Z4B6D9/<new-comment>.md`

**Step 1: Format the changed Rust source**

Run:

```sh
devenv shell -- cargo fmt
```

Expected: formatting succeeds. Inspect the diff to ensure only intended source,
lockfile, tests, and Cycle documentation changed.

**Step 2: Run all required verification**

Run:

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
devenv shell -- cargo run --locked --bin manyhands-cli
```

Expected: every command exits zero. Do not run the desktop smoke test unless an
active display is available; if unavailable, record it as not run rather than
claiming success.

**Step 3: Record non-secret verification evidence**

Add a canonical ticket comment with a new ULID and UTC timestamp. State which
commands passed, counts or summaries that contain no secret fixture values, and
whether the desktop smoke test was unavailable. Mark the ticket review-ready
only after the requested code review is complete.

**Step 4: Do not commit without authorization**

Suggested commit if requested:

```text
test: verify shared key registry
```

## Completion Criteria

- All Cycle 01 exit evidence in
  `docs/Cycles/wave-02-cycle-01-shared-key-registry.md` is demonstrated by
  automated tests.
- The full required Rust suite passes on the final source and lockfile.
- The ticket has canonical comments for the baseline, planning decision,
  verification result, and review-ready state.
- No commit, push, ticket closure, key-file mutation, or remote Git operation
  occurs without the user's explicit authorization.
