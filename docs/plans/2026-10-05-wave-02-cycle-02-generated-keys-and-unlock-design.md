---
title: "Wave 02 Cycle 02 Generated Keys And Session Unlock Design"
date: 2026-10-05
status: approved
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M45WHG47XTKCK0VC795G2X5P"
---

# Generated Keys And Session Unlock Design

## Intent And Approval Status

Deliver the headless credential-material layer that Cycle 03 can use for SSH
authentication: users can generate a protected key, retain imported keys by
reference, unlock an owned key for an application session, and deliberately
delete an unselected owned key with recoverable interruptions. Success means
safe files, useful typed recovery, and no secret persistence outside the
approved private-key files.

The user approved this design, Cycle, and implementation plan on 2026-10-05
and authorized subagent-driven implementation. The imported-key boundary
below was explicitly confirmed by the user. The other refinements below were approved with this design.

## Existing Behavior And Gaps

`src/repository.rs` contains the complete Cycle 01 key registry and has grown
beyond 9,000 lines. It records caller-supplied ownership, generates registration
IDs internally, normalizes source paths lexically, and observes private paths
without opening them. Companion public reads are bounded. Its deletion
preflight is metadata-only; it conveys no lasting authorization.

Important implications:

1. `ownership = generated` is caller input, not evidence that Manyhands created
   a file. Destructive deletion cannot rely on that field alone.
2. Generation needs the same ID in the filename and registration; calling the
   public registration method would allocate a second ID.
3. SQLite transactions cannot atomically commit two key files. Partial writes,
   registration failure, and a process ending between steps need explicit states.
4. Existing repository operation records require repository context. Key
   operations are application-global and must not acquire a Git lease.
5. The CLI is empty; ticket comments still require canonical filesystem writes.
6. CI builds five targets but runs no tests. A Linux result cannot establish
   Windows DACL behavior or macOS filesystem behavior.
7. The RFC requires a specific OpenSSL-backed Windows SSH configuration, but
   Cargo.toml does not yet request those features. That transport prerequisite
   remains a recorded Cycle 03 dependency, not a claim of compatibility here.

## Approaches Considered

| Approach | Benefit | Cost / limitation |
| --- | --- | --- |
| Extend RepositoryService through focused key modules (recommended) | Reuses the registry, lock discipline, and public metadata API; separates secret-bearing code for review. | A targeted extraction of existing key code and two small metadata tables. |
| Put all new behavior in repository.rs | Smallest initial module change. | Further mixes Git, SQLite, secret lifetime, and platform security in an already large file. |
| Introduce a separate credential daemon or external ssh-keygen process | Delegates some work to another component. | Adds lifecycle, deployment, and secret-transfer boundaries the approved headless Cycle does not need. |

Use the first approach. Keep shared logic in the library's module tree, without
GPUI dependencies. Do not refactor unrelated repository operations.

## Modules And Public Boundary

Create `src/repository/keys/` with `mod.rs` (service orchestration and public
material types), `registry.rs` (existing metadata operations plus material
records), `session.rs` (provider and secret lifetime), `storage.rs` (owned-store
contract), and `storage/unix.rs` / `storage/windows.rs` (platform handles and
protection). Keep existing Cycle 01 names available at `manyhands::repository`
through re-exports. Put new types under `manyhands::repository::keys`.

The proposed public methods are:

```rust
impl RepositoryService {
    pub fn generate_shared_key(&self, store: &KeyStore,
        request: GenerateSharedKeyRequest)
        -> Result<GenerateSharedKeyOutcome, KeyMaterialError>;
    pub fn inspect_selected_key(&self, store: &KeyStore)
        -> Result<SelectedKeyInspection, KeyMaterialError>;
    pub fn unlock_generated_key<P: SessionCredentialProvider>(&self,
        store: &KeyStore, session: &mut SessionCredentials<P>)
        -> Result<GeneratedKeyUnlockOutcome, KeyMaterialError>;
    pub fn review_generated_key_deletion(&self, store: &KeyStore, id: SharedKeyId)
        -> Result<GeneratedKeyDeletionReview, KeyMaterialError>;
    pub fn delete_generated_key(&self, store: &KeyStore,
        operation_id: OperationId, review: Option<GeneratedKeyDeletionReview>,
        confirmed: bool) -> Result<DeleteGeneratedKeyOutcome, KeyMaterialError>;
    pub fn list_key_material_recovery(&self)
        -> Result<Vec<KeyMaterialRecovery>, KeyMaterialError>;
}
```

`KeyStore::for_current_user()` resolves the home using `directories::BaseDirs`;
`KeyStore::for_home(home: &Path)` permits an absolute isolated test home. Both
return `Result<KeyStore, KeyMaterialError>` and construct configuration only;
filesystem mutation occurs during an explicit material operation. Neither uses
the application-data directory as a key store. Tests inject paths, rather than
changing process-global HOME concurrently.

`GenerateSharedKeyRequest` contains `operation_id: OperationId`, `label: String`,
and `protection: KeyProtection`. `KeyProtection` is `Unencrypted` or
`Passphrase(SecretPassphrase)`. Generate a `SharedKeyId` internally exactly once
when reserving the operation. Success is `Created(SharedKeyRegistration)` or
`AlreadyCreated(SharedKeyRegistration)`; interrupted work is
`RecoveryRequired(KeyMaterialRecovery)`. Generation never selects the key.

`SelectedKeyInspection` is `NoSelection`,
`ImportedReadable { registration: SharedKeyRegistration, source: KeySourceToken }`,
or `Generated { registration: SharedKeyRegistration, source: KeySourceToken }`.
Failures use the typed error below.
Generated inspection also verifies provenance and file protection; imported
inspection proves only that an opened handle refers to a readable regular file.

`GeneratedKeyUnlockOutcome` is `Ready(SharedKeyRegistration)`, `NoSelection`,
`ImportedValidationDeferred(SharedKeyRegistration)`, `Cancelled`, or
`ProviderUnavailable`. Ready contains no decrypted key or passphrase.

`GeneratedKeyDeletionReview` has private fields binding key ID, expected paths,
observed file identities (including explicit absence), and creation evidence.
Expose a metadata-only `registration()` accessor for caller confirmation. It is
consumed by deletion, has no Clone or serialization implementation, and can
still be rejected as stale. Keep Cycle 01's old preflight API as an advisory
compatibility API; it cannot authorize deletion.

`DeleteGeneratedKeyOutcome` is `Deleted`, `AlreadyDeleted`, `Cancelled`, or
`RecoveryRequired(KeyMaterialRecovery)`. `KeyMaterialRecovery` exposes only
operation ID, key ID, action, phase, stable failure code, and a `RecoveryAction`
enum (`RetryGeneration`, `ReviewDeletionAgain`, `InspectRetainedFiles`). There
is no generic error-detail string or persisted confirmation token.

`KeyMaterialError` contains `operation: KeyMaterialAction`, optional key and
operation IDs, `kind: KeyMaterialErrorKind`, and fixed `guidance() -> &'static
str`. Actions are `Generate`, `Inspect`, `Unlock`, `ReviewDeletion`, `Delete`,
and `ListRecovery`. Error kinds cover `InvalidLabel`, `InvalidPassphrase`,
`HomeUnavailable`, `RegistryUnavailable`, `Busy`, `NotRegistered`,
`SelectedKeyMustBeCleared`, `ImportedKey`, `OwnershipUnverified`, `UnsafePath`,
`ProtectionUnavailable`, `SourceMissing`, `SourceUnreadable`, `NotRegularFile`,
`InvalidGeneratedKey`, `UnlockFailed`, `SourceChanged`, `SelectionChanged`,
`OperationMismatch`, `ConfirmationRequired`, `RandomnessUnavailable`, `GenerationFailed`, and
`StorageUnavailable`. Do not attach raw source errors. Entropy acquisition
failure must return a typed error rather than panic or fall back to a seed.

## Generation And Protected Storage

Keep `ssh-key` at 0.6.7 with `ed25519`, `encryption`, and `getrandom` features.
Use its OS randomness path, `Algorithm::Ed25519`, OpenSSH encoding, and empty
key comments. Use its AES-256-CTR / bcrypt-PBKDF encryption defaults (16 rounds
in the inspected locked source); pin these expectations in round-trip tests.
Do not invent a cipher, shell out, or persist a random seed. Add a direct
`zeroize` dependency for application-owned secret buffers. The library returns
a self-zeroizing encoded private-key string. These choices were checked against
the [ssh-key API](https://docs.rs/ssh-key/0.6.7/ssh_key/private/struct.PrivateKey.html)
and the locally installed 0.6.7 source.

An explicitly unencrypted request is allowed. An encrypted request with an empty
or NUL-containing passphrase returns `InvalidPassphrase`; never silently turn
it into an unencrypted request. Preserve other whitespace and Unicode exactly.
Reject NUL in session input too, to preserve the later C-string handoff boundary.
Labels retain existing nonblank validation and never become file comments.

Files are exactly `<home>/.ssh/manyhands/<key-id>` and `<key-id>.pub`.
Create exclusively, never truncate or replace an existing entry. Resolve the
home anchor once; validate/open descendant directories without following
symlinks or Windows reparse points. An existing unsafe `.ssh` or `manyhands`
entry fails with guidance to repair it explicitly. Do not silently change the
permissions of existing user SSH files or directories.

On Unix, create missing `.ssh` and `manyhands` with `0700`, private files with
`0600`, and public files with `0644`. Verify the effective user owns the store
and private file; verify the exact private/store modes before writing bytes.
Use directory-relative operations (`openat`, `O_NOFOLLOW`, `O_EXCL`, `fstat`,
`unlinkat`) against pinned handles. Existing `.ssh` must be user-owned and not
group/world-writable. Reject multiple hard links on owned private files.

On Windows, create with an explicit protected DACL granting only the current
user SID access, an explicit owner SID, and no inheritable handles. Verify the
owner and effective allow entries from the opened handles, including existing
store entries, before writing secrets. Both private and public files can use
the same owner-only ACL. Reject NULL/permissive/inherited DACLs and reparse
points. Pin ancestor handles against rename/delete while operating; delete
validated files by handle. Use `windows-sys` 0.61.2 target-specific bindings
with Foundation, Security, Security_Authorization, Storage_FileSystem,
System_Memory, and System_Threading features. Microsoft documents creation-time
security descriptors in [File Security and Access Rights](https://learn.microsoft.com/en-us/windows/win32/fileio/file-security-and-access-rights).

This policy protects against accidental access widening and unsafe path
traversal. It does not claim protection from a privileged administrator or a
malicious process running as the same user. Unsupported filesystem security
semantics return `ProtectionUnavailable`, never a weaker fallback.

## Coordination And Durable Material Evidence

Use an owner-protected `.lock` file inside the owned store with `fs4` locking
to serialize generation and deletion, including service instances with distinct
application-data paths. Imported inspection and session prompting do not create
or require the owned directory. Lock order: store lock, then existing registry
cache guard, then SQLite transaction. No code takes the reverse order. Ordinary
selection/unregistration uses its existing cache guard only.

Generate/encrypt in memory outside the registry lock. Never hold a cache/Git
lease while asking the caller for a passphrase. Short file/registry finalization
steps use the cache guard to serialize selection, unregistration, and deletion.
Use the established bounded lock behavior; contention returns `Busy`.

Add two application-global tables, preserving all Cycle 01 rows:

- `owned_generated_keys`: key ID primary key, expected private/public paths,
  opaque platform private/public file identities, and public SHA-256 fingerprint.
  Only successful generation inserts ownership evidence. No cascade from
  unregistration: retained owned files remain attributable, but cannot be
  deleted through the registered-key API after unregistration.
- `key_material_operations`: operation ID primary key, key ID, action,
  generation label, expected paths, observed file identities when known,
  public fingerprint when known, phase, and nullable stable failure code.
  A partial unique index allows only one incomplete operation per key. No
  repository FK, credential column, private-file hash, or confirmation column.

Actions are `generate` and `delete`. Generation phases are `reserved`,
`private-written`, `pair-written`, `completed`, and `retained-for-inspection`.
Deletion phases are `prepared`, `private-removed`, `files-removed`, `completed`,
and `retained-for-inspection`. Constrain action/phase combinations in SQL.
File identities contain platform identity (device/inode or volume/file ID) and
freshness metadata (length and modification/change information), never
private-content fingerprints. Observe them again after writes before persisting.
Progress is evidence; re-observed files decide.

Generation sequence:

1. Validate inputs and inspect an existing operation ID before doing expensive
   work. Reject an ID reused for another label/action/store; never replace its
   material. A completed matching operation returns the existing registration
   only after checking files/protection remain valid; otherwise return recovery.
   If the registration was subsequently removed, report NotRegistered without
   restoring it or its files. Recheck operation identity after taking the lock.
2. Generate and encode in zeroizing memory. Secure/open the store and acquire
   its lock. Reserve the operation and key ID in SQLite before creating files.
3. Exclusively create, verify, write, and flush the private file; record its
   identity. Do the same for the public file, then flush directory metadata
   where supported. Record `pair-written` and verify the pair/public fingerprint.
4. In one registry transaction insert ownership evidence, insert the unselected
   registration with that same ID, and mark the operation completed.

Never claim filesystem and SQLite atomicity. An ordinary error after creation
returns `RecoveryRequired` and leaves protected partial material identified by
the operation. Do not sweep files on startup. Explicit retry may finalize a
fully written pair whose recorded identities, protection, and public metadata
match, without needing a stored passphrase. A crash between exclusive creation
and recording the file's identity is not sufficient ownership evidence: preserve
the file, mark `retained-for-inspection`, and give manual inspection guidance.
Incomplete/mismatched pairs follow the same conservative outcome; retry never
overwrites or silently creates another key. A fresh generation uses a fresh
operation/key ID. Reusing a completed generation ID never changes protection.

Registry corruption must not authorize deletion or automatic file adoption.
The existing registry replacement/recovery mechanism can lose registrations;
owned files remain untouched and may be registered as imported references.
Record this limit rather than promising reconstruction of ownership from Git.

## Imported Readability And Session Unlock

Registration remains path-only. Explicit selected-source inspection may open
the imported file read-only and check the opened handle is a regular file, but
does not read or parse its contents. Reject FIFOs/devices/directories without
blocking (nonblocking open on Unix, then handle validation). Imported symlinks
may resolve to a regular readable source; ownership remains external. Do not
chmod imported files, require a companion, or reject a backend-compatible
format because ssh-key cannot parse it. Missing/denied sources retain rows.

The caller owns `SessionCredentials<P>` for one application session. It wraps
the caller's prompt provider and owns at most one cached secret. It is separate
from RepositoryService and contains no Git handle or callback.

```rust
pub trait SessionCredentialProvider {
    fn request_passphrase(&mut self, request: &UnlockRequest) -> PassphraseResponse;
}
pub enum PassphraseResponse {
    Supplied(SecretPassphrase), Cancelled, Unavailable,
}
impl<P: SessionCredentialProvider> SessionCredentials<P> {
    pub fn new(provider: P) -> Self;
    pub fn clear(&mut self);
    pub fn invalidate(&mut self, key_id: SharedKeyId);
    pub fn with_passphrase<T>(&mut self, request: UnlockRequest,
        use_passphrase: impl FnOnce(&str) -> Result<T, PassphraseUseFailure>)
        -> Result<T, SessionUnlockFailure>;
}
```

`SecretPassphrase::new(String) -> Result<Self, InvalidPassphrase>` takes
ownership into `Zeroizing<String>` immediately, including before validating it.
It has redacted Debug, no Display, Clone, Serialize, or public string accessor.
`KeyProtection`, responses, requests, and session formatting cannot reveal it.
`UnlockRequest` contains key ID, non-secret label, and an opaque `KeySourceToken`
produced by source inspection. `KeySourceToken::observe(path: &Path) ->
Result<Self, KeyMaterialError>` captures a readable regular source without
reading its bytes, using the same handle observer as selected-key inspection.
This permits standalone provider tests and future domain adapters without a
constructor that invents filesystem identity. The token binds absolute source path, opened file
identity, length, and modification/change metadata. It is not persisted. File
metadata cannot detect every malicious same-user write; it is cache invalidation,
not an integrity/authentication claim.

`PassphraseUseFailure` is `Rejected`, `SourceChanged`, or `Unavailable`.
`SessionUnlockFailure` is `Cancelled`, `ProviderUnavailable`, `InvalidPassphrase`,
`Rejected`, `SourceChanged`, or `Unavailable`. The supplied closure lends the
passphrase for one validation attempt. Cache only a successful validation;
evict on failure, explicit invalidation, a different key/source token, clear,
or drop. A cancelled/unavailable prompt stores nothing. One call prompts at most
once; retry requires another explicit call. Drop clears owned buffers; do not
test zeroization by reading freed memory.

Generated unlock takes a current selected-key snapshot, verifies owned paths
and protection, and reads a bounded (64 KiB maximum) private encoding into
zeroizing memory. Accept only the generated Ed25519 profile; before running a
KDF require the expected cipher and 16 bcrypt rounds, to avoid attacker-chosen
work factors in a modified owned file. Parse unencrypted keys without prompting.
For encrypted keys, use `with_passphrase` to decrypt and match the public key to
the recorded fingerprint. Drop decrypted material immediately. Re-observe the
source token and selection after prompting/decryption before returning Ready;
changes invalidate the cache and return `SourceChanged` / `SelectionChanged`.

Generation does not seed the unlock cache: generating a key is not first use
of the selected key. Switching selection is detected at the next attempt;
front ends may additionally call `clear()` when selection changes or on lock.

Cycle 03 will use the same provider for imported/backend challenges. It must
not treat `git2::Cred::ssh_key` construction as proof of decryption or successful
authentication. Its validation/eviction adapter and polling pause behavior are
explicit downstream obligations; Cycle 02 tests no imaginary SSH success.

## Confirmed Deletion And Retry

Review verifies registration, generated ownership, unselected state, creation
evidence, expected `<id>` / `<id>.pub` paths, protected ancestors, and current
file identities. A legacy caller-labeled generated row without creation evidence
returns `OwnershipUnverified`, even if its path looks plausible. Imported rows
are never eligible. Do not infer ownership by reading an arbitrary private key.

`confirmed = false` returns Cancelled without writing a journal or file. A
missing review can only acknowledge a completed operation at the same store
with AlreadyDeleted; all unfinished deletions return ConfirmationRequired.
This permits acknowledging a completed operation after the process lost its
in-memory review, without recreating deletion authority. After true
confirmation with a review, take store/cache locks, re-read selection and registration,
and compare the consumed review with opened handles. A selected, replaced,
renamed, linked, or otherwise changed target requires another review. Another
process selecting the key during confirmation therefore prevents deletion.

Persist `prepared` before the first removal. Remove private first, then public,
recording progress after each removal. Missing files are idempotent only when
fresh review and recorded ownership identify the exact expected entries.
Remove the registration and ownership row only after both are absent; mark
completed in the same transaction. Keep the completed operation as replay
evidence. An operation ID replay must match its original key/action/paths.

After any interruption, retain registration and the operation row. Selection
and unregistration refuse keys with pending material operations (`Busy` with
fixed recovery guidance), so they cannot discard or reactivate a half-deleted
key. Recovery does not continue deletion on startup. The caller must obtain
another review and confirm remaining deletion, reusing the operation ID; an
already-completed matching deletion returns AlreadyDeleted. A consumed review
held across that completion can be used only to acknowledge AlreadyDeleted,
never to delete newly appearing entries.

## Privacy, Verification, And Risks

Private key bytes are allowed only in the approved key files and transient
cryptographic memory. Passphrases are transient only. SQL rows, WAL, recovery
records, errors, Debug, snapshots, and logs contain neither. Generated public
key files are expected public output; private-file hashes are never recorded.
Use boolean assertions for secret comparisons, not assert_eq/unwrap paths that
format secret buffers. Scan all app-data files, including WAL/journals and
recovery backups, plus captured diagnostics for unique test secrets. Do not
run generic byte-printing repository snapshots over key fixtures.

Zeroizing holders reduce secret lifetime in application-owned allocations;
they cannot promise erasure of allocator history, OS swap, crash dumps, or
every cryptographic dependency temporary. Review dependency allocation/error
paths and document any limitation rather than claiming tests prove total
process-memory erasure.

The local review of locked `ssh-key` 0.6.7 found concrete limits: Ed25519
private holders erase on drop, and decryption uses a `Zeroizing<Vec<u8>>`, but
`PrivateKey::encrypt_with` encodes plaintext into an ordinary `Vec` before
encryption. An encoding/encryption error can drop that allocation without
erasure. `to_bytes` also wraps its allocation only after encoding succeeds;
`to_openssh` wraps the successful PEM string, not every encoding temporary.
Manyhands maps these errors to fixed kinds without formatting dependency
errors and zeroizes its own seed, encoded private output, read buffers, and
passphrase holders. This reduces exposure; it does not erase every dependency
temporary, allocator copy, stack/register copy, OS swap page, or crash dump.
The privacy tests establish storage/diagnostic exclusion, not total process
memory erasure. Providers and validation callbacks remain trusted consumers
of temporary passphrase borrows.

| Risk / ambiguity | Decision or verification |
| --- | --- |
| Imported parser scope | User confirmed: readability/provider only; backend validation in Cycle 03. |
| Existing insecure or symlinked SSH directories | Fail with fixed recovery guidance; no implicit repair or global SSH changes. |
| Generated metadata can be forged by callers | Require creation evidence plus exact paths and handle identities. |
| SQLite/file crash gaps | Journal before mutation; conservative retained files when evidence is insufficient. |
| Two processes and stale confirmation | Store lock, cache serialization, and recheck immediately before removal. |
| Windows ACL implementation | Verify creation-time ACLs and owner on native Windows; unsupported protection fails closed. |
| CI command policy | User approved native Cargo tests in existing CI on 2026-10-05; document the exception in AGENTS.md while preserving Devenv for local commands. |
| Unregister leaves owned files | Preserve RFC behavior; re-registration as imported never restores destructive ownership automatically. |
| Locked SSH backend compatibility | Track required libssh2-sys 0.3.3 OpenSSL Windows features for Cycle 03 and prove real transport there. |

Verification uses isolated runtime-generated keys, platform filesystem tests,
session counting providers, and failure injection at every durable boundary.
Rerun required local checks before/after implementation. Windows/macOS runtime
results must be recorded separately; compilation alone is insufficient.
