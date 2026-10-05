---
title: "Authentication and Credential Handling RFC"
date: 2026-10-04
status: approved
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01K7G8J0K3P5R7T9V1X3Z5B7D9"
---

# Authentication and Credential Handling RFC

## Summary

This RFC defines the shared Manyhands SSH credential model for the approved
MVP. It implements `MH-CRED-001` and `MH-NFR-003` from the
[PRD](../PRD/mvp.md) and the identity, transport, and redaction decisions
assigned by the [MVP architecture RFC](mvp-rfc.md).

Manyhands uses one user-selected SSH private key for every Git-over-SSH
operation. It generates Ed25519 OpenSSH private keys, registers imported keys
without copying their private material, keeps passphrases only for one
application session, and requires explicit trust for an unknown SSH host key.
All remote Git operations use `git2`/libgit2 callbacks. A system Git executable,
SSH-agent fallback, default-key fallback, credential helper, OAuth token, and
Git-forge key upload are not runtime dependencies.

## Scope

This RFC defines:

- Shared-key registration, selection, generation, import, unregistration, and
  explicit generated-key deletion.
- Non-secret local metadata and the protected locations of generated private
  and public key files.
- Imported-key validation, session-only passphrase use, and selected-key-only
  `git2` credential callbacks.
- SSH host trust-on-first-use (TOFU), pin replacement, and compatibility with
  existing read-only `known_hosts` trust.
- Redaction, typed recovery outcomes, test isolation, and the authenticated SSH
  Git fixture required for Wave 2 evidence.

It does not define remote refs, fetch/push ordering, polling, merge recovery,
desktop prompts, CLI grammar, desktop-worker scheduling, OAuth, Git-forge APIs,
repository cloning, or application authorization. The Git workflow RFC owns the
remote lifecycle; the repository/index RFC owns physical persistence; and the
desktop and CLI RFCs own interaction details.

## Git Commit Identity Boundary

Git commit identity is separate from SSH authentication. Wave 1's Git workflow
continues to resolve effective repository or global `user.name` and `user.email`
and to return identity-required before a commit when either is absent. A future
desktop or CLI caller obtains explicit confirmation before writing a missing
identity to the local repository configuration. Manyhands MUST NOT derive a Git
commit identity from a selected SSH key, key label, public-key comment, host,
or remote username, and it MUST NOT create an application-wide commit identity.

SSH key selection, host approval, and passphrase unlock never change Git commit
identity. Commit identity is not secret, but it is not included in credential
callback diagnostics unless it is already part of the requested Git operation's
ordinary non-secret outcome.

## Terms

| Term | Meaning |
| --- | --- |
| Shared key | The one user-selected Manyhands key used for every Git-over-SSH operation. |
| Generated key | An Ed25519 OpenSSH private key created and owned by Manyhands. |
| Imported key | A caller-owned private-key file registered by path without copying it. |
| Session credential provider | A caller-supplied, in-memory provider of a protected key's passphrase for one application session. |
| Host pin | A non-secret SHA-256 fingerprint for one normalized SSH host and effective port. |
| Host approval | An explicit caller confirmation of the exact presented host-key fingerprint. |

## Key Registration And Selection

Manyhands stores only the following key registration metadata in application
state:

- A locally generated opaque key ID.
- A required caller-supplied non-secret label.
- `generated` or `imported` ownership.
- The absolute private-key source path and, when available, a public-key path.
- An optional parsed public-key fingerprint.
- Whether the key is selected as the shared key.
- Non-secret observed accessibility and recovery state.

The database MUST NOT store private-key contents, passphrases, credential
callback values, remote response bodies, or a content-derived key fingerprint.
The selected shared key is user-local, not repository-local; at most one key may
be selected. Selecting a key is an explicit local action and never changes Git
configuration, tracked repository content, or a remote.

### Imported Keys

Import accepts a caller-selected private-key path and a required label. It
records a reference to that path without copying, parsing, unlocking, or
deleting the private material. A caller MAY provide a companion public-key path;
Manyhands records its fingerprint only when it parses without reading the
private key.

The first transport use validates that the locked `git2`/libssh2 backend can
load the registered key. An unavailable, unreadable, invalid, unsupported, or
incorrectly unlocked key returns a typed recovery outcome while retaining its
registration and source file unchanged. This permits externally generated keys
without making an application parser's format support a hidden import
restriction.

Unregistering an imported key only removes its application-local registration.
It MUST NOT modify or delete the external source file, companion public key, or
any global SSH configuration.

### Generated Keys

Generated keys use the Ed25519 algorithm and the OpenSSH private-key format.
Manyhands creates a unique directory entry at:

```text
~/.ssh/manyhands/<key-id>
~/.ssh/manyhands/<key-id>.pub
```

On Unix, the `manyhands` directory MUST use mode `0700` and each generated
private-key file mode `0600`; the public key MAY use mode `0644`. On Windows,
Manyhands MUST apply an owner-only DACL to the generated private-key file and
its containing directory, then verify that the owner cannot be widened by the
operation. If the platform cannot establish or verify its approved protection
model, generation fails before the key is registered.

Generation MAY use an optional caller-supplied passphrase. The passphrase MUST
not be written to a file, database, diagnostic, log, or Git configuration.
Generated public fingerprints and labels are non-secret and may be listed.

Unregistering a generated key removes only its registration and retains both key
files by default. Destructive deletion is a separate action that requires an
explicit caller confirmation, applies only to an owned generated key at its
expected path, and never follows symlinks outside `~/.ssh/manyhands/`. A
selected key must first be cleared or replaced. A deletion interruption retains
a recovery record and never deletes an imported source file.

## Session Unlock And SSH Callbacks

The session credential provider receives a protected-key unlock request only
when the selected key is first needed in an application session. A supplied
passphrase is retained only in that provider's memory for the remaining session;
it is not persisted by the domain service and MUST use a zeroizing secret holder
that clears it when the session ends or replaces it. The provider may return
cancelled, unavailable, or a passphrase. Cancellation leaves the repository and
key state unchanged and pauses polling until a later explicit unlock or sync
attempt.

Every Git-over-SSH operation builds fresh `git2::RemoteCallbacks` inside the
operation that uses them. The credentials callback:

- Accepts only an SSH credential request and returns the selected key through
  `git2::Cred::ssh_key`.
- Uses the URL username when present, or returns an SSH username credential only
  when libgit2 requests one before key authentication.
- Supplies the session passphrase only to the selected key's credential call.
- Rejects HTTP(S), username/password, SSH-agent, credential-helper, and default
  credential requests with a typed non-secret error.

Neither callbacks nor `git2` handles may be retained in desktop or CLI model
state, transferred across threads, or reused after an operation ends.

## Host Trust

Manyhands identifies an SSH authority by the lower-cased DNS host or canonical
IP literal from the publication URL and its effective numeric port. The default
port is `22`. The authority excludes the SSH username and repository path.
Standard `ssh://` and SCP-style SSH remote URLs use the same host-and-port
normalization rule; an unparseable authority is a typed remote-configuration
problem.

Host pins are application-local, non-secret records containing only the
normalized authority, host-key algorithm, and SHA-256 fingerprint. On a host
key that libgit2 cannot otherwise verify, Manyhands returns a host-approval
outcome containing the presented fingerprint. The caller must provide an
approval for that exact authority and fingerprint before a later operation may
continue. The approval is persisted only after the presented key is observed
again.

A changed key for an existing authority always returns a replacement-required
outcome. It MUST NOT overwrite the existing pin, proceed with the transport, or
alter Git state until a caller explicitly approves replacement of the exact old
and new fingerprints. Existing user `known_hosts` trust is accepted as a
read-only compatibility trust source; Manyhands never writes, deletes, or uses
it to select a client credential. Once a Manyhands pin exists, its fingerprint
controls even when another trust source regards the presented host key as valid.
The transport callback MUST compare the presented key with that pin; an
implementation that cannot make the comparison rejects the connection rather
than silently relying on inherited `known_hosts` trust. Tests isolate home and
Git configuration paths so that host-pin behavior does not inherit developer
trust state.

## Recovery And Redaction

Credential and transport outcomes identify the requested action, repository,
publication remote, selected key registration, and normalized host authority
when available. They classify at least these conditions:

- No shared key selected.
- Imported or generated key unavailable, unreadable, invalid, unsupported, or
  rejected.
- Unlock cancelled or failed.
- Host approval required or host-pin replacement required.
- SSH transport unavailable, authentication rejected, remote unavailable, or
  SSH protocol failure.

Outcomes, operation records, logs, test output, snapshots, and machine-readable
interfaces MUST NOT expose private-key bytes, passphrases, usernames not already
present in a configured URL, credential callback data, authorization material,
or unredacted server response text. They use stable recovery codes and concise,
redacted human guidance instead.

## Test And Compatibility Contract

Wave 2 tests run with isolated home, SSH, Git configuration, and application-data
paths before the first `git2` operation. They generate an ephemeral server host
key and allowed client key, create disposable bare repositories through `git2`,
and run a test-only local SSH Git server restricted to those repositories. That
fixture MAY invoke `git-upload-pack` and `git-receive-pack` solely to provide a
real Git protocol service for tests. Production binaries never invoke a system
Git executable.

The fixture and tests MUST prove:

- Successful fetch and push with the selected key, and rejection with a wrong,
  absent, default, or SSH-agent key.
- First-contact host approval, changed-host replacement, and no inherited
  developer `known_hosts` or global Git state.
- Generated-key protection, imported-key non-deletion, explicit generated-key
  deletion, session-only unlock, cancelled unlock, and unavailable-key recovery.
- No private key, passphrase, credential callback value, or remote response body
  enters persistent state, diagnostics, assertions, or snapshots.
- The selected dependency configuration works on Linux, macOS, and Windows. A
  platform limitation is a documented compatibility case, not a fallback to a
  different key or credential source.

Wave 2 implementation configures the locked `libssh2-sys` `0.3.3` dependency
with `openssl-on-win32` and `vendored-openssl`. This preserves the OpenSSL-backed
Windows path required for the approved Ed25519 OpenSSH key format. Any dependency
upgrade must re-prove the selected-key transport contract on all supported
platforms; Manyhands does not silently downgrade generated keys to legacy RSA
PEM to avoid a platform build issue.

## Acceptance

This RFC is satisfied when automated evidence demonstrates that:

- A user can generate, label, list, select, unregister, and explicitly delete a
  protected Manyhands-generated Ed25519 key without persisting its passphrase.
- A user can register an externally supplied key without copying or deleting it;
  first use reports any actual backend or unlock incompatibility recoverably.
- Every successful Git-over-SSH operation uses exactly the selected shared key
  and approved host trust, without agent, default-key, or credential-helper
  fallback.
- First use of a protected key requests an in-session passphrase, reuses it only
  for that session, and leaves local work recoverable after cancellation or
  failure.
- Unknown and changed host keys require explicit approval, and all credential
  and transport diagnostics remain redacted.
- The test-only SSH Git fixture provides real selected-key fetch/push evidence
  without creating a production system-Git dependency.

## Decision Log

| Decision | Status | Rationale |
| --- | --- | --- |
| Generated keys are Ed25519 OpenSSH private keys. | Accepted | Modern default with one portable format for application-owned keys. |
| Generated keys live under `~/.ssh/manyhands/`, not application data. | Accepted | The PRD requires a separate owner-only private-key location and forbids private-key contents in application data. |
| Imported keys are registered by reference and validated at first use. | Accepted | Preserves external ownership and supports backend-compatible external formats without copying private material. |
| One shared selected key serves every MVP SSH operation. | Accepted | Implements the approved PRD shared-key decision. |
| Host trust uses explicit TOFU pins by host and port. | Accepted | Prevents silent first-contact or host-key replacement while remaining usable without preconfigured global state. |
| Production uses only `git2`/libgit2 transport. | Accepted | Preserves the approved runtime Git backend boundary. |
| Tests may use a restricted local SSH Git server with Git protocol helpers. | Accepted | Supplies real authenticated fetch/push evidence without making system Git a production dependency. |
