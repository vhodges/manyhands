---
manyhands_managed: true
manyhands_kind: ticket
id: "01M5E6DYJ6GN7DMW4424TH3V78"
title: "A wrong passphrase for a protected key is reported as transport_unavailable"
type: "defect"
status: "open"
project: "manyhands"
team: "core"
---

When the selected shared key is protected and the user supplies the wrong
passphrase, every SSH operation reports `transport_unavailable`. That code is
transient (CLI exit 5), and its recovery action is `request.retry`. It tells
the user the network failed, not that the passphrase was wrong. The result
code that exists for this case, `unlock_failed` (blocked, exit 3), is reached
only when a server rejects a key after a passphrase was supplied.

Found on 2026-10-20 while planning Wave 03 C2 (ticket
`01M4CC0VMTTNKVYS6XWQG4CK1N`). The Wave's C2 exit evidence names "wrong
unlock". The product owner ruled that C2 proves today's answer, plus
`unlock_failed` through a server that rejects the key. C2's human output adds
a hint when the invocation supplied a passphrase. This ticket makes the
library report a wrong passphrase as `unlock_failed`. The CLI and the desktop
then pass it through unchanged.

This is a shared-library change under Wave 03 track rule 5. It gets its own
review and merge to main.

## Evidence

Traced by reading the code at `origin/main` `3bda743`. Nothing was run.

- `src/repository/transport/operation.rs`, around lines 128–155:
  - The passphrase is used inside `session.with_passphrase`.
  - A failed connection is stored as `connection_error` and reported to the
    session as `PassphraseUseFailure::Unavailable`.
  - The stored error is then returned as it is.
- `src/repository/transport/remote.rs`, around lines 762–772: the error is
  classified from libgit2's code.
  - Only `git2::ErrorCode::Auth` becomes `UnlockFailed`, when a passphrase
    was supplied.
  - A key that libssh2 cannot decrypt with the given passphrase fails as
    some other code, so it falls through to `TransportUnavailable`.
- `src/repository/mutation/outcome.rs`: `TransportUnavailable` maps to
  `transport_unavailable`; `UnlockFailed` maps to `unlock_failed`.
- The behavior is pinned by a test: `tests/ssh_transport/session.rs`,
  `wrong_passphrase_retry`, expects `SshTransportErrorKind::TransportUnavailable`
  for `secret("wrong")`.
- `tests/mutation_remote.rs`,
  `a_client_the_server_rejects_is_unlock_failed_or_unlock_required`, shows
  `unlock_failed` arises only from `server.reject_client()`.

## Expected behavior

A passphrase that does not unlock the selected key is `unlock_failed`. That
holds for synchronization, host observation and host approval, and for every
other operation that unlocks the key. Nothing is pinned, fetched or pushed,
and the passphrase is not kept in the session as valid. A real transport
failure after a successful unlock stays `transport_unavailable`.

The fix must not tell a wrong passphrase from a network failure by matching
backend error text. Possible routes, for the design to choose:

- check the passphrase against the key before connecting, where the key's
  format allows it (the `ssh-key` crate's `encryption` feature is already a
  dependency, and generated keys are OpenSSH Ed25519);
- classify by libgit2's error class and the phase of the connection.

An imported key in a format the application cannot parse may keep today's
answer. If it does, the design says so.

## Acceptance criteria

- With a protected key and a wrong passphrase, a library test gets
  `unlock_failed` for a synchronization, `host observe` and `host approve`,
  and nothing changes on either side.
- `tests/ssh_transport/session.rs::wrong_passphrase_retry` is updated to the
  new kind, with the owner's approval, since it pins Wave 02 behavior.
- A network failure after a correct unlock is still `transport_unavailable`.
- No passphrase, key material or backend text reaches a result, record or
  log; the existing privacy scans still pass.
- Goldens and schemas are unchanged, or are updated if a recovery action
  changes.
- The C2 CLI's wrong-passphrase hint is removed or kept, as the CLI's owner
  decides once this merges.
