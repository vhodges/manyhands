---
title: "Wave 02 Cycle 02 Generated Keys And Session Unlock Implementation Plan"
date: 2026-10-05
status: approved
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M45WHG47WFWSVF7YZ8ARSHCD"
---

# Generated Keys And Session Unlock Implementation Plan

> **For agentic workers:** Use the executing-plans skill for native execution,
> or subagent-driven-development if the user selects that method. Read the
> linked design before each task. Track steps with the checkboxes below.

**Goal:** Deliver protected generated SSH keys, session-only passphrases,
imported-source readability checks, and confirmed recoverable deletion.

**Architecture:** Extend RepositoryService through focused key modules while
preserving the existing registry API. Platform storage owns secure handles;
SQLite stores only registration, creation evidence, and material progress;
caller-owned session providers hold zeroizing secrets. Cycle 03 owns SSH use.

**Tech Stack:** Rust 2024, ssh-key 0.6.7, zeroize, rusqlite, fs4, libc on Unix,
windows-sys 0.61.2 on Windows, existing Devenv and native platform CI.

**Spec:** [Detailed design](2026-10-05-wave-02-cycle-02-generated-keys-and-unlock-design.md)
and [Cycle contract](../Cycles/wave-02-cycle-02-generated-keys-and-unlock.md).

**Status:** Approved by the user on 2026-10-05 for subagent-driven implementation.
Preserve the existing branch/worktree. No commits, pushes, or ticket closure
are part of this planning deliverable. Suggested task commits below apply only
if implementation delivery includes authorized commits.

## Global Constraints

- Keep domain logic in the library; no GPUI or GPUI Kit in credential modules.
- Generated keys are Ed25519 OpenSSH at `~/.ssh/manyhands/<key-id>` and `.pub`.
- Unix store `0700`, private file `0600`, public file `0644`; Windows owner-only
  protected DACL and verified owner before secret writes.
- No private bytes or passphrases in SQLite, diagnostics, recovery, or Git.
- Imported registration stays path-only; explicit readability inspection does
  not parse imported material. Imported unlock/backend validation is Cycle 03.
- Generation is unselected. Unregistration retains files. Deletion requires
  creation evidence, exact paths, unselected state, and fresh confirmation.
- No prompts under cache/store/Git locks; no key operation acquires a Git lease.
- Local Rust commands use `devenv shell -- cargo ...`. The user approved native
  Cargo test steps in existing CI on 2026-10-05; document that exception.
- Update Cargo.lock with dependency changes. Do not alter the root checkout's
  uncommitted Devenv changes or research document.
- Use runtime-generated secrets only in isolated temporary homes. Do not touch
  real SSH files, run transport, or add production subprocess dependencies.

## Review Focus

1. Caller-forged generated metadata and path substitution must never authorize
   deletion: ownership/protection tests in Tasks 2 and 6.
2. Crash between file mutation and SQL phase update must preserve work and
   retry without overwriting or deleting unproven files: Tasks 3 and 6.
3. A selection/source change while a prompt is open must invalidate the
   pending result and cached passphrase: Tasks 4 and 5.
4. FIFO, directory, denied, or replaced imported sources must return promptly
   without parser restrictions or modifying registration: Task 5.
5. An inherited Windows ACL or newly introduced Debug/SQL error path must not
   expose secrets: platform tests in Task 2 and privacy/CI gate in Task 7.

## File Map And Task Dependencies

| File | Responsibility / task |
| --- | --- |
| `src/repository.rs` | Preserve old key re-exports, delegate extracted registry methods; Tasks 1, 6. |
| `src/repository/keys/mod.rs` | Public material API and service orchestration; Tasks 1, 3, 5, 6. |
| `src/repository/keys/registry.rs` | Existing metadata SQL and new ownership/progress helpers; Tasks 1, 3, 6. |
| `src/repository/keys/session.rs` | Secret wrapper in Task 3; provider and cache in Task 4. |
| `src/repository/keys/storage.rs` | KeyStore and platform-neutral owned handle contract; Task 2. |
| `src/repository/keys/storage/unix.rs` | Directory-relative Unix protection and deletion; Task 2. |
| `src/repository/keys/storage/windows.rs` | Handle-based Windows owner/DACL enforcement; Task 2. |
| `src/repository/discovery.rs` | Invoke the idempotent key schema migration; Task 1. |
| `tests/shared_key_registry.rs` | Preserve Cycle 01 API and guard pending lifecycle records; Tasks 1, 6. |
| `tests/key_material.rs` | Generation, import, deletion, recovery, and privacy integration; Tasks 3, 5, 6, 7. |
| `tests/key_storage.rs` | Native filesystem protection and path tests; Task 2. |
| `tests/session_credentials.rs` | Provider/caching/cancellation tests; Task 4. |
| `tests/support/keys.rs` | Isolated home and key fixtures, non-printing secret scans, fault hooks. |
| `tests/support/mod.rs` | Expose reusable key fixture module. |
| `tests/repository_enablement.rs`, `tests/discovery_rebuild.rs` | Adjust schema assertions and preservation fixtures; Task 1. |
| `Cargo.toml`, `Cargo.lock` | Feature/dependency additions; Tasks 2–4. |
| `.github/workflows/build.yml`, `AGENTS.md` | Native credential runtime tests and approved CI exception; Task 7. |
| This ticket's `.manyhands/comments/` | Canonical checkpoint evidence after every task. |

Sequence: baseline → metadata → platform storage → generation → session →
selected-source use → deletion → full evidence. The coupled interfaces favor
native execution with a final independent code review; execution method remains
the user's choice. Do not launch implementation workers before plan approval.

## Task 0: Establish The Implementation Baseline

**Files:** Ticket comments only; no product changes.

- [x] Verify this worktree/branch is still the ticket's, record its HEAD, and
  check whether main advanced. Rebase before new work only after preserving any
  approved planning changes; never discard a dirty worktree or force-push.
- [x] Run each command and require exit zero:

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --all-features --locked
  ```

- [x] Record exact results, source/lockfile identity, and baseline problems in
  a new canonical ticket comment. Diagnose baseline failures before attributing
  them to this Cycle. Prior Cycle evidence is context, not a fresh test result.

## Task 1: Isolate Key Metadata And Add Durable Material Records

**Files:** `src/repository.rs`, new `keys/mod.rs` and `keys/registry.rs`,
`src/repository/discovery.rs`, schema and registry integration tests.

**Interfaces:** Preserve all existing `manyhands::repository` shared-key types
and methods through re-exports. Add private
`keys::registry::migrate_material_schema(tx: &rusqlite::Transaction<'_>) ->
Result<(), RepositoryError>`. Define shared material error and recovery types
from the design now; generation/session/deletion request types arrive with
their implementing tasks. Do not add secret fields to SharedKeyRegistration.

- [x] Add `material_schema_preserves_cycle01_rows_and_selection` and
  `material_schema_is_idempotent_and_rejects_invalid_phases`. Start from the
  exact current schema, retain an imported row, a generated metadata row, and
  selected state; assert both new tables exist after reopening twice. Assert
  invalid action/phase combinations and two incomplete operations for one key
  fail. Inspect table columns to exclude secrets/confirmation flags.
- [x] Run
  `devenv shell -- cargo test --locked --test shared_key_registry material_schema`;
  expect missing-table assertions to fail before implementation.
- [x] Move only existing shared-key types, methods, path/public metadata helpers,
  and their constants into the focused modules. Keep public names and Cycle 01
  behavior unchanged. Add schema migration in the existing transaction, with
  columns, phases, and constraints from the design. Do not route app-global
  records through repository lifecycle records.
- [x] Run the new tests plus existing `shared_key_registry`,
  `repository_enablement`, and `discovery_rebuild` targets using
  `devenv shell -- cargo test --locked --test <target>`. Expect all to pass.
- [x] Record metadata checkpoint; suggested commit:
  `refactor: isolate key registry and add material recovery records`.

## Task 2: Establish Platform-Protected Owned Storage

**Files:** New `keys/storage.rs`, `storage/unix.rs`, `storage/windows.rs`,
`tests/key_storage.rs`, `tests/support/keys.rs`, Cargo files.

**Interfaces:** Public `KeyStore::for_current_user()` and
`KeyStore::for_home(&Path)` return `Result<KeyStore, KeyMaterialError>`.
Private `KeyStore::lock() -> Result<OwnedStoreGuard, KeyMaterialError>`;
`OwnedStoreGuard::create_private(SharedKeyId)` and `create_public(SharedKeyId)`
return `Result<OwnedKeyFile, KeyMaterialError>`;
`open_owned(id, KeyFileKind) -> Result<Option<OwnedKeyFile>, KeyMaterialError>`
opens existing validated entries. `KeyFileKind` is `Private` or `Public`.
`OwnedKeyFile::identity() -> FileIdentity`,
`write_all_and_sync(&mut self, &[u8]) -> Result<(), KeyMaterialError>`, and
`remove(self) -> Result<(), KeyMaterialError>` keep validation tied to handles.
`FileIdentity` includes stable platform identity and freshness metadata and has
non-secret SQL encoding. `OwnedStoreGuard::sync_directory()` flushes directory
updates where the platform supports it; unsupported durability is documented.
Also produce private `observe_regular_source(path: &Path) ->
Result<FileIdentity, KeyMaterialError>`, which opens read-only without blocking,
checks the opened handle is regular, and observes metadata without reading
bytes. Unlike owned-store opening, this observer permits imported symlinks.

Execution refinement: keep storage operations private and run their detailed
behavior tests as internal unit tests; key_storage integration tests cover
configuration-only public constructors. Task 7 CI includes --lib to run the
private storage tests on each native platform.

- [ ] Add `creates_private_storage_without_permission_window`,
  `refuses_unsafe_existing_store`, `refuses_existing_private_path`,
  `refuses_symlink_or_reparse_substitution`, `refuses_private_hardlinks`, and
  `two_services_serialize_owned_file_changes`. Assert private creation succeeds
  only with exact expected ownership/protection, an existing sentinel target
  remains byte-for-byte intact, and contention returns Busy without SQL/file
  mutation. Test denied protection deterministically, not by assuming root
  cannot open a chmod-000 file.
- [ ] Run `devenv shell -- cargo test --locked --test key_storage`; expect missing
  API/behavior failures before implementation. Keep secret bytes out of failure
  formatting by asserting booleans.
- [ ] Implement exclusive creation, pinned descendants, owner/mode or DACL
  verification before writes, flush, and safe removal. Add target-specific
  windows-sys features listed in the design; use existing libc and fs4 on Unix.
  Never implement Windows permissions with the read-only file attribute or
  create permissively and chmod afterwards.
- [ ] Add native Windows checks for owner SID, protected DACL, and absence of
  effective grants to other SIDs, including inherited Everyone access. Exercise
  reparse points on hosts where they can be created; record an explicit fixture
  limitation if the runner lacks privileges, retaining deterministic backend
  rejection tests. Unix tests assert `mode & 0o777 == 0o700/0o600/0o644`.
- [ ] Run local `key_storage` tests and record which platform ran. Windows and
  macOS runtime results are collected in Task 7, not inferred from Linux.
  Suggested commit: `feat: create owner-protected SSH key storage`.

## Task 3: Generate And Register Ed25519 Key Pairs Recoverably

**Files:** `keys/mod.rs`, `keys/registry.rs`, new `keys/session.rs`, Cargo files,
`tests/key_material.rs`, fixture/failure hooks.

**Interfaces:** Implement the design's `generate_shared_key` method and all
generation request/outcome types. Private
`reserve_generation(tx: &rusqlite::Transaction<'_>, operation_id: OperationId,
label: &str, store: &KeyStore) -> Result<SharedKeyId, KeyMaterialError>` fixes the
one key ID and derives its two paths; registration finalization inserts that
ID rather than calling the existing public register method. Keep helpers private
and use the Task 2 handle API.

- [ ] Add `generation_plain_and_encrypted_round_trip`: parse the emitted own
  files in the fixture, assert Ed25519, matching public fingerprint, OpenSSH
  format, empty comments, selected=false, expected paths, and protection flags.
  For encryption assert AES-256-CTR, bcrypt rounds=16, correct-passphrase success,
  and incorrect-passphrase failure using booleans that cannot print material.
  Add `generation_rejects_empty_or_nul_passphrase` and preserve whitespace and
  Unicode in `generation_preserves_passphrase_bytes`.
- [ ] Add `generation_replay_never_creates_a_second_pair`,
  `generation_reservation_identity_is_registration_identity`,
  `generation_registry_failure_preserves_protected_files`, and
  `generation_crash_gap_does_not_adopt_unproven_files`. Inject interruption after
  reservation, exclusive create before identity record, private write, public
  write, and before/after final transaction. Assert exact phase/recovery action,
  row absence before verified completion, unchanged collision bytes, and one
  registration after a valid retry. No test treats metadata-only generated rows
  as creation evidence.
- [ ] Run
  `devenv shell -- cargo test --locked --test key_material generation`;
  expect compilation/assertion failures before implementation.
- [ ] Enable ssh-key generation/encryption/OS RNG features and add zeroize.
  Update Cargo.lock through Devenv Cargo. Introduce `SecretPassphrase` and
  `InvalidPassphrase` in session.rs now, with the design's consuming constructor
  and redacted formatting; Task 4 adds the provider around this primitive.
  Implement generation with zeroizing
  buffers and the design's reservation/write/flush/finalization sequence.
  Extract public metadata only; never log a private parse/IO error. Implement
  conservative retry and `list_key_material_recovery`, with no startup mutation.
- [ ] Run the focused tests; verify completed replay after later file removal
  returns actionable missing-source/recovery state and does not resurrect files.
  Scan fixture app-data for secret material with a non-printing helper.
  Add a deterministic entropy-failure hook and assert RandomnessUnavailable
  leaves no files or registration and emits no raw RNG error.
- [ ] Record checkpoint and limitations; suggested commit:
  `feat: generate and register protected Ed25519 keys`.

## Task 4: Implement Caller-Owned Session Credentials

**Files:** `keys/session.rs`, `keys/mod.rs`,
`tests/session_credentials.rs`, Cargo files if needed.

**Interfaces:** Reuse Task 3's `SecretPassphrase`; implement `SessionCredentialProvider`,
`PassphraseResponse`, `UnlockRequest`, `SessionCredentials<P>`,
`PassphraseUseFailure`, and `SessionUnlockFailure` exactly as the design defines.
`KeySourceToken` remains opaque; its `observe(&Path)` constructor obtains real
handle metadata without reading bytes, using Task 2's source observer.
Integration fixtures create temporary regular files and observe their tokens;
no public arbitrary-token constructor is needed.

- [ ] Add a counting provider and test `successful_unlock_prompts_once_per_session`:

  ```rust
  assert_eq!(provider_calls_after_two_valid_uses, 1);
  assert_eq!(provider_calls_after_new_session, 2);
  ```

  Add `failed_validation_is_not_cached`, `cancelled_unlock_prompts_only_on_retry`,
  `unavailable_provider_has_no_cached_value`, `new_key_or_source_evicts_secret`,
  and `clear_and_invalidate_drop_cached_secret`. Assert successful use of A,
  use of B, then A prompts three times. Rejected cached material is evicted,
  without recursively prompting in the same call.
- [ ] Run `devenv shell -- cargo test --locked --test session_credentials`;
  expect failures before implementing the provider/cache.
- [ ] Implement the one-entry zeroizing cache, redacted formatting, ownership
  transfer, borrowed callback access, and clear/drop paths. Use structural tests
  and an instrumented drop witness for eviction; never inspect freed memory or
  expose a production secret getter for tests. Reject NUL and empty supplied
  secrets without retaining them.
- [ ] Run session tests and direct formatting checks for every public wrapper.
  Assert no public credential type can serialize; use compile-fail documentation
  tests only where they exercise that actual privacy boundary. Record that
  callback consumers are trusted not to copy or log borrowed secrets.
- [ ] Record checkpoint; suggested commit:
  `feat: retain unlock secrets only in caller-owned sessions`.

## Task 5: Inspect Selected Sources And Validate Generated Unlock

**Files:** `keys/mod.rs`, `keys/storage.rs`,
`tests/key_material.rs`, session test fixtures.

**Interfaces:** Implement design methods `inspect_selected_key` and
`unlock_generated_key<P>`. Private source observation produces
`KeySourceToken` from opened handle identity/path/size/time metadata.
Use existing selected registration and the Task 4 session API; return public
metadata only. Imported reads never use ssh-key.

- [ ] Add `import_readability_does_not_parse_private_material` using deliberately
  non-key bytes: expect ImportedReadable and unchanged bytes/registration.
  Cover missing, inaccessible, directory, FIFO, companion absent, and symlink
  to a readable regular file. For FIFO use a child-process timeout so failure
  cannot hang the test suite. No SSH operation is attempted.
- [ ] Add `generated_unlock_round_trip_reuses_session`,
  `unencrypted_selected_key_does_not_prompt`, `generated_unlock_cancel_preserves_state`,
  `generated_unlock_wrong_passphrase_preserves_registration`,
  `selected_key_changes_during_prompt`, and `key_source_changes_during_prompt`.
  Use provider hooks to mutate selection through a second service while the
  prompt is pending, proving no cache/store lock is held. Assert changed state
  returns SelectionChanged/SourceChanged and invalidates the cached value.
- [ ] Add `generated_unlock_bounds_file_and_kdf_work` (over 64 KiB, modified
  cipher/round count, malformed input) and `generated_unlock_checks_public_identity`.
  Assert typed failures before expensive KDF work, with no raw parser output.
- [ ] Run
  `devenv shell -- cargo test --locked --test key_material import` and
  `devenv shell -- cargo test --locked --test key_material unlock`;
  expect failures before implementation.
- [ ] Implement nonblocking regular-file inspection, bounded generated reads,
  profile checks, decryption/fingerprint validation, post-prompt observation,
  and selection recheck. Return ImportedValidationDeferred without prompting
  when generated unlock is called with an imported selection. Do not persist
  passphrase, decrypted material, or content-derived private hashes.
- [ ] Run focused tests and the Cycle 01 registry target. Run the complete
  `key_material` target too, so the selection/source
  race cases run even when their names do not match the focused filters.
  Record the Cycle 03 integration obligations (backend validation, rejected-secret eviction,
  libssh2 Windows features, polling cancellation). Suggested commit:
  `feat: inspect key sources and unlock generated keys per session`.

## Task 6: Delete Only Confirmed, Proven-Owned Generated Files

**Files:** `keys/mod.rs`, `keys/registry.rs`,
`tests/key_material.rs`, `tests/shared_key_registry.rs`.

**Interfaces:** Implement `review_generated_key_deletion` and
`delete_generated_key` with the design's opaque consumed review and explicit
confirmation boolean. Extend pending-operation guards in existing selection
and unregistration methods; add `RepositoryErrorKind::SharedKeyMaterialPending`
with fixed recovery guidance for these existing Result-returning APIs, never
raw SQL text. Material APIs report Busy for the same condition.

- [ ] Add `deletion_refuses_imported_selected_and_unproven_generated_rows`,
  `deletion_cancel_writes_nothing`, `deletion_removes_exact_owned_pair`,
  `deletion_rechecks_selection_after_review`, and
  `deletion_refuses_replaced_or_linked_target`. For caller-forged generated rows
  use both arbitrary paths and the expected-looking directory; both lack the
  creation evidence needed for deletion. Assert outside sentinels survive.
- [ ] Add `deletion_retry_requires_fresh_confirmation` and
  `deletion_retry_preserves_replacement_files`. Inject interruption after intent,
  after each unlink before/after recording the phase, and around final SQL commit.
  Reopen the service and assert it does not delete anything automatically;
  pending registration remains; select/unregister is blocked; explicit renewed
  review resumes only matching remaining files. A completed replay, including
  `review: None` after restart, returns AlreadyDeleted without touching newly
  created entries. An incomplete operation with `review: None` returns
  ConfirmationRequired and does nothing.
- [ ] Add `unregister_generated_retains_both_files` and ensure already-missing
  entries are handled only against matching ownership evidence and fresh review.
  `deletion_operation_id_mismatch_changes_nothing` covers cross-key/action replay.
- [ ] Run
  `devenv shell -- cargo test --locked --test key_material deletion`;
  expect missing behavior failures before implementation.
- [ ] Implement handle validation, lock ordering, prepared record, private-first
  removal, progress recording, and final atomic row removal/completion. Preserve
  old advisory preflight behavior; never let its registration alone authorize
  removal. Keep errors and recovery rows free of source content.
- [ ] Run deletion tests and `shared_key_registry`. Record checkpoint;
  suggested commit: `feat: confirm and recover generated key deletion`.

## Task 7: Prove Privacy And Platform Compatibility, Then Request Review

**Files:** Credential integration tests, `.github/workflows/build.yml`,
`AGENTS.md`, ticket comments, Cycle/design documents for actual evidence.

**Interfaces:** Reuse the test APIs above. Add no transport capability.

- [ ] Add `credential_outputs_and_storage_exclude_secrets`. Exercise generation,
  wrong passphrase, cancellation, denied IO, malformed input, registration
  failure, and deletion interruption. Scan all isolated application-data files
  (SQLite/WAL/journals/recovery backups) and captured errors/Debug/log output for
  unique passphrases, encoded private material, and a raw private seed segment.
  Assert booleans with fixed messages so a failure never prints a secret.
  Expected private/public key files under isolated SSH home are separate from
  the app-data scan. Include an active WAL reader so WAL assertions are real.
- [ ] Run the privacy tests; fix actual exposed paths without suppressing
  assertions. Review ssh-key encoding/encryption error paths and document the
  zeroization limits described in the design. Do not claim total process-memory
  erasure from storage scans.
- [ ] Amend AGENTS.md's Rust-command rule to state that local/agent commands use
  Devenv and existing GitHub Actions native runners may execute Cargo directly
  for builds and tests. The user explicitly approved this exception during
  planning. Do not change local Devenv files for this exception.
- [ ] Add a test step to the existing five-target matrix, before artifact upload:

  ```sh
  cargo test --locked --target ${{ matrix.target }} --lib --test shared_key_registry --test key_storage --test session_credentials --test key_material
  ```

  Preserve release builds, target coverage, and artifact checks. These headless
  tests need no display or SSH server. Assert that each native runner executes
  its platform tests; do not silently skip an entire backend under cfg guards.
- [ ] Format and run the complete local verification:

  ```sh
  devenv shell -- cargo fmt
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --all-features --locked
  devenv shell -- cargo run --locked --bin manyhands-cli
  git diff --check
  ```

  Require exit zero. If an active display is available, run
  `devenv shell -- cargo run --locked --features desktop --bin manyhands` and
  confirm launch; otherwise record desktop smoke as not run.
- [ ] Review the final diff for source/lockfile consistency, no tracked secret
  fixtures, no real-home IO, no imported parser restriction, and no transport
  or UI scope expansion. Record local and CI results separately. Windows/macOS
  runtime evidence remains pending until those jobs actually pass; do not claim
  the platform gate is satisfied by adding workflow text.
- [ ] Record review-ready status only after checks relevant to the review have
  run, explicitly listing any outstanding platform evidence. Request code review
  of ownership validation, path races, journal reconciliation, cache eviction,
  Windows ACLs, and secret formatting. Fix findings and rerun affected checks.
  Keep the ticket open until code-review or PR approval, normally closing in
  the final authorized push before merge. Suggested commit:
  `test: verify key protection sessions recovery and privacy across platforms`.

## Acceptance Map And Handoff

| Cycle requirement | Owning tasks |
| --- | --- |
| Approved Ed25519 generation and optional protection | 2, 3 |
| Owner-only Unix/Windows storage | 2, 7 |
| Imported-source readability without format restrictions | 5 |
| Session-only provider, reuse, cancellation, eviction | 4, 5 |
| Explicit generated-key deletion and recovery | 1, 6 |
| Secret-free persistence/diagnostics | 1, 3, 4, 7 |
| Preserved registrations and actionable recovery | 3, 5, 6 |
| Cross-platform evidence and ticket lifecycle | 0, 7; comments after each task |

The user approved the Cycle, design, and plan on 2026-10-05 and selected
subagent-driven development: a fresh implementer and task reviewer per task,
followed by independent whole-branch review. Implementation is authorized in
the existing ticket worktree.
