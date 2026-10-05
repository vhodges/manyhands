---
title: "Wave 02 Cycle 02: Generated Keys And Session Unlock"
date: 2026-10-05
status: approved
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M45WHG47EBKGC0VCA00FA2FR"
---

# Wave 02 Cycle 02: Generated Keys And Session Unlock

## Purpose And Authority

Extend Cycle 01's application-local shared-key registry with protected Ed25519
key generation, imported-source readability checks, caller-owned session
credentials, and confirmed, recoverable deletion of owned generated files.
This is the second Cycle of [Wave 02](../Waves/wave-02-collaboration.md), tracked
by ticket `01K7F6H9J2N4Q6S8V0X2Z4B6DA`.

The [authentication RFC](../RFC/authentication-and-credential-handling.md),
[persistence RFC](../RFC/repository-index-persistence-and-refresh.md), and
[test strategy](../RFC/test-and-compatibility-strategy.md) are authoritative.
Traceability: `MH-CRED-001`, `MH-NFR-003`, and the Wave's Cycle 02 exit evidence.

Read the [detailed design](../plans/2026-10-05-wave-02-cycle-02-generated-keys-and-unlock-design.md)
and [implementation plan](../plans/2026-10-05-wave-02-cycle-02-generated-keys-and-unlock-implementation.md)
together. The user approved all three documents on 2026-10-05 and authorized
subagent-driven implementation.

## Entry Conditions

- Reuse the existing ticket branch and worktree, rebased onto `main` before
  planning. Planning baseline: `main` at `21eefa4`, ticket at `d546264`.
- Cycle 01 is merged, reviewed, and closed. Its final comment records 401 passing
  tests and the required checks. Ticket source and lockfile match that baseline.
- Approve this Cycle, design, and plan before implementation. Rerun the required
  Rust verification suite before changing Rust code and record fresh evidence.
- Preserve the approved distinction between registration, file accessibility,
  generated-key decryption, and actual SSH-backend compatibility.

## Scope

- Generate Ed25519 OpenSSH keys, optionally encrypted with a supplied passphrase,
  beneath the resolved user home's `.ssh/manyhands/` directory.
- Establish and verify Unix modes and Windows owner-only DACLs before private
  bytes are written. Never register a key whose protection cannot be verified.
- Derive the generated public key and SHA-256 public fingerprint. Keep labels
  in the registry; do not embed caller labels in key-file comments.
- Inspect imported-source readability without parsing, copying, or unlocking
  imported private material. Preserve registrations on every access failure.
- Provide a zeroizing, caller-owned session credential provider; prove generated
  key decryption, prompt reuse, cancellation, invalid-passphrase eviction, and
  session termination behavior.
- Require fresh confirmation and verified creation evidence for destructive
  deletion. Preserve default unregister-without-deletion behavior.
- Record non-secret generation/deletion progress and recover conservatively
  after file/SQLite partial completion. Test adversarial paths and interruptions.

## Boundaries And Decisions

The user confirmed on 2026-10-05 that imported-key format, unlock, and backend
validation remain in Cycle 03. A readable imported source means only that its
regular file can be opened; it is not an authentication success.

No transport callbacks, host trust, Git fetch/push, polling state, remote
operations, UI, CLI commands, or key upload are added. Cancellation provides a
typed outcome for later polling to consume; it does not implement polling now.
No real user key is generated or deleted by developing or testing this Cycle.

## Exit Evidence

| Contract | Required evidence |
| --- | --- |
| Generation | Encrypted and unencrypted Ed25519 round trips; public/private match; unique paths; registration only after verified writes. |
| Protection | Linux/macOS mode and ownership checks; Windows owner/DACL runtime checks; failure leaves no usable registration. |
| Import | Regular readable, missing, denied, directory, and FIFO sources; no private parser restriction or external-file mutation. |
| Session | One successful prompt reused only in the same session/source; cancellation, rejection, replacement, key change, and new-session behavior. |
| Deletion | Fresh confirmation, selection recheck, ownership evidence, exact paths, symlink/reparse rejection, stale-file refusal, interrupted retry. |
| Privacy | Database/WAL/recovery/log scans and formatting tests exclude private material and passphrases without printing test secrets. |
| Compatibility | Required local Devenv checks; native platform runtime evidence, with unavailable platforms explicitly pending. |

Record planning, baseline, implementation checkpoints, verification, and review
as ticket comments. Keep the ticket open until code-review or PR approval; do
not confuse approval of this plan with approval of the finished code.

### Cycle 03 credential integration obligations

- Validate imported formats and encrypted keys with the actual SSH backend;
  readable sources and `git2::Cred::ssh_key` construction do not prove successful
  decryption or authentication.
- Connect backend rejection to `SessionCredentials::invalidate(key_id)` so a
  rejected cached secret is evicted. Cache only a backend-validated attempt and
  keep provider calls outside Git, registry, and key-store locks.
- Verify the locked `libssh2-sys` 0.3.3 OpenSSL features on Windows and exercise
  real transport there before claiming compatibility.
- Pause or cancel background polling when credentials are cancelled or their
  provider is unavailable; retries must be an explicit new attempt.

### Local verification recorded 2026-10-05

The ticket worktree passed `devenv shell -- cargo fmt`,
`devenv shell -- cargo check --all-features --locked`,
`devenv shell -- cargo fmt --check`,
`devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`,
and `devenv shell -- cargo test --all-features --locked`: 469 unit/integration
and 9 compile-fail documentation tests passed, with zero failures or ignored
tests. The CLI smoke command exited zero. The desktop smoke command ran on
`DISPLAY=:1`, `WAYLAND_DISPLAY=wayland-1`; after Cargo reported execution,
the app printed `Hello from Manyhands!` from its button callback and exited
zero. This confirms an interactive window launched, beyond merely building.
No smoke-test process remained. `git diff --check` also passed.

The privacy regression runs in an isolated subprocess, captures stdout/stderr
(including explicitly formatted API errors, guidance, Debug, and recovery
results), and scans application-data paths recursively. It asserts that live
WAL frames are scanned while a reader pins them, then scans nested DB/WAL
backup snapshots and a populated rollback journal. Expected private fixtures
stay in a separate temporary SSH home. Probes cover unique correct/incorrect
passphrases, malformed sensitive input, whole private encodings, PEM body
lines, and 16-byte private seed segments. Generation, wrong passphrase,
cancellation, denied read, malformed input, registration failure, and
interrupted deletion all execute. Temporary diagnostic and SQLite/WAL leak
mutations each failed with fixed messages without exposing the secret; both
mutations were removed. No application storage/diagnostic leak was found.

This proves the tested persistence/diagnostic boundary, not erasure of all
process memory. See the design's concrete `ssh-key` allocation/error-path
limits. Imported-key backend validation, rejection-driven cache invalidation,
real transport, and Windows `libssh2-sys` compatibility remain Cycle 03 work.

The existing five-target native CI matrix now runs the approved headless
library/credential command before artifact upload, preserving release builds
and artifact checks. Linux ARM, both Windows targets, and macOS runtime
results remain pending until those CI jobs run; workflow text and local Linux
results do not satisfy their platform gate. Whole-branch code review and
its approval remain separate from these local results; the ticket stays open.
