---
title: "Wave 02 Cycle 03 Authenticated SSH Transport Design"
date: 2026-10-05
status: approved
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M46ECQP2XXSNQFMT515ZZJ29"
---

# Authenticated SSH Transport Design

## Intent And Decisions For Approval

Deliver a reusable headless SSH authentication boundary for the later Wave 02
remote lifecycle. Success means actual backend authentication with exactly the
selected key, explicit host trust, session-only secrets, preserved local work
on authentication failure, and native-platform transport evidence.

The approved RFCs fix key ownership, algorithms, callback restrictions, and
host trust. This proposal refines their mechanism: a scoped transport driver,
connection verification, optimistic host-pin transactions, and a Rust loopback
SSH fixture. It does not change the approved import boundary or add sync policy.
See the [Cycle](../Cycles/wave-02-cycle-03-authenticated-ssh-transport.md) and
[plan](2026-10-05-wave-02-cycle-03-authenticated-ssh-transport-implementation.md).

## Plan Review: Decisions And Evidence Gaps

Review requested by the user on 2026-10-05 found substantive issues in the first
draft. The user subsequently approved the complete scope, design, and plan on
2026-10-05 and authorized subagent-driven implementation. The timeout mechanism
is an implementation prerequisite to resolve before the driver task.
The following behavioral decisions are approved:

| ID | Decision | Approved behavior |
| --- | --- | --- |
| Q1 | Missing URL username: require it in the remote URL or allow an explicit caller-supplied username? | Require it in the URL; do not assume an OS account or `git`. |
| Q2 | A no-passphrase attempt fails ambiguously: ask once, or return recovery and require explicit unlock retry? | Ask once with wording that does not claim the key is certainly encrypted. |
| Q3 | Corrupt database replacement loses pins: require fresh approval or resume ordinary first-contact/known_hosts rules? | Require fresh approval after recovery, preventing an unnoticed trust downgrade. |

Q1 requires the username in the URL. Q2 permits one prompt explaining the
ambiguity. Q3 requires fresh approval after corrupt-database recovery, even
when known_hosts matches. Its proposed durable mechanism is specified below.
The user also accepted 10 seconds to connect and 30 seconds of stalled I/O on
2026-10-05: allow a reasonable short backend delay without leaving the user
waiting excessively. Initialization and backend phase coverage remain the
engineering gate; the timeout values are no longer an open product decision.

| Finding | Review disposition |
| --- | --- |
| Anonymous remotes still apply URL rewrite configuration. | Corrected below: compare raw configured endpoint and effective fetch/push URL before any connection. |
| Transfer calls can replace connect-time callbacks; a raw Remote closure bypasses policy. | Corrected below: scoped adapter owns every transfer's options and rejection checks. |
| Pin survival was overstated for corrupt database replacement. | Normal rebuild versus corrupt database replacement distinguished; approved Q3 requires fresh trust using the proposed recovery marker below. |
| `connect_auth` includes remote service/advertisement work, so its failure does not reliably prove a bad secret. | Cache only positively verified success; use conservative unknown-stage errors and no message-string inference. |
| Network stalls were only bounded by a fixture watchdog. | Unresolved engineering gate: establish production timeout/cancellation scope and a safe initialization mechanism before approving the driver contract. |
| Fixture portability was described as established before it was tested. | A target, not evidence; prove Windows OpenSSL, helper provisioning, and the chosen russh version early. |

The locked backend starts with zero connect/I/O timeout settings; its exposed
setters modify C globals and require initialization before threads. Do not set
them per operation, race existing Git calls, or present a watchdog killing the
test process as production cancellation. The fixture must characterize stalls
before SSH handshake, during auth, and during advertisement/transfer. The accepted
defaults are 10,000 ms to connect and 30,000 ms of stalled I/O, not a 30-second
total transfer limit. A slow operation that continues making progress can run
longer. Tests must cover both recovery from a shorter stall and timed failure
after the threshold, allowing bounded scheduling tolerance. If a safe library/host initialization
contract is needed, update this design and affected startup-file scope before
implementation; never silently defer an indefinite hang to later polling work.

Review evidence: locked libgit2 `remote.c` (`git_remote_create_anonymous`,
`git_remote_create_with_opts`, `connect_or_reset_options`, `git_remote_upload`),
`transports/ssh_libssh2.c`, `streams/socket.c`; git2 `opts.rs` and `remote.rs`;
Manyhands `replace_corrupt_registry` in `src/repository.rs`; and Cycle 01's
documented loss of registrations during corrupt-registry replacement.

## Current Foundation And Alternatives

`RepositoryService` owns application-local SQLite; `keys/registry.rs` supplies
selection; `keys/session.rs` supplies `KeySourceToken`, `UnlockRequest`,
`SessionCredentials::with_passphrase`, and `invalidate`. The closure passed to
`with_passphrase` must validate actual use before returning success. Merely
constructing `git2::Cred::ssh_key` does not load/decrypt/authenticate a key.
`repository.rs` has permissive SSH eligibility checks but no transport; migration
lives in `repository/discovery.rs`. The CLI entry point is still empty.

| Approach | Assessment |
| --- | --- |
| Scoped driver plus shared policy modules and loopback Rust SSH server | Recommended: enforces operation ownership, targets all native runners, keeps production on git2. Adds test-only server dependencies whose portability must be demonstrated. |
| Expose a callback factory to all callers | Smaller initial API, but leaves secret lifetimes, cache validation, and reconnect policy to each caller. Reject for the public API. |
| Host-managed OpenSSH daemon fixture | Realistic, but requires platform-specific daemon/user provisioning and Windows service configuration. Reserve as a compatibility diagnostic, not the primary fixture. |

## Modules And Public Contract

Add `pub mod transport` beneath `repository`, split into `mod.rs` (types),
`endpoint.rs` (parsing), `trust.rs` (pins), `callbacks.rs` (policy),
`operation.rs` (scoped execution), and `error.rs` (redaction). No GPUI types.
Do not enlarge `repository.rs` beyond module wiring and shared SSH eligibility.

Proposed public entry point:

```rust
pub fn verify_ssh_transport<P: SessionCredentialProvider>(
    &self,
    request: VerifySshTransportRequest,
    session: &mut SessionCredentials<P>,
) -> Result<SshTransportVerified, SshTransportError>;
```

`VerifySshTransportRequest` has `root: PathBuf`, `direction: SshDirection`
(`Fetch` or `Push`), and `approval: Option<HostApproval>`. It resolves the
publication remote from canonical config. It never accepts an arbitrary
credential, key path, URL, or refspec from the caller. `SshTransportVerified`
contains root, remote name, selected `SharedKeyId`, direction, normalized
`SshAuthority`, and observed `HostKeyIdentity`; no Git handles or URL strings.

`SshAuthority { host: String, port: u16 }` and
`HostKeyIdentity { algorithm: String, sha256: String }` are validated non-secret
values. SHA-256 uses OpenSSH `SHA256:<unpadded-base64>` notation. `HostApproval`
has `authority`, `expected: Option<HostKeyIdentity>`, and `presented`:
`expected = None` approves first contact; `Some(old)` approves replacement.
No approval API writes trust without a fresh matching network observation.

The crate-private driver is `with_authenticated_remote<P, T>(&self,
request: VerifySshTransportRequest, session: &mut SessionCredentials<P>,
use_remote: impl FnOnce(&mut AuthenticatedSshRemote<'_>) -> Result<T, SshTransportError>)
-> Result<T, SshTransportError>`. Invoke the closure exactly once, after
authentication and post-connect rechecks, while its `RemoteConnection` remains
alive. `AuthenticatedSshRemote` is a crate-private scoped adapter, with no raw
Remote accessor. It owns methods to read advertisements, download objects using
explicit refspecs, and push explicit refspecs; every transfer builds fresh
FetchOptions/PushOptions carrying the selected-key, host, redaction, and progress
policy. Connect-time callbacks alone are insufficient: libgit2 replaces options
even on an already connected transport. Any required reconnect must re-establish
the same policy and state checks or return a retry outcome before transfer.
Push completion must inspect per-ref rejection callbacks as well as the top-level
return code; rejection text is discarded and an explicit `PushRejected` returned.
Downloaded tracking-ref updates remain the later lifecycle caller's separate
coordinated local step, exercised explicitly only in fixtures here.
Public verification reads only the advertisement. No public arbitrary-transfer
API is introduced, and later callers still own approved coordination/ref policy.

## Endpoint And Credential Policy

Read the raw configured fetch URL for Fetch and raw `pushurl` (falling back to
fetch URL) for Push; a looked-up Remote may already have applied rewrites.
Validate the configured endpoint, create the in-memory remote, then compare its
effective direction-specific URL to that endpoint before connecting. Anonymous
remotes do apply `insteadOf`/`pushInsteadOf`: reject any rewrite that changes the
endpoint, including username or repository path on the same host, with
`ConfigurationInvalid`. This proposal deliberately does not support rewritten
publication URLs; guidance asks for the explicit destination URL. Check callback
authority too, and never change repository remote configuration or global Git
configuration. Test both repository-local and global rewrites, on fetch and push.

Normalize DNS names to ASCII lower case and IP literals through Rust IP parsing;
default port is 22, explicit ports are numeric 1..65535. Support `ssh://` and
SCP forms, including bracketed IPv6. Reject non-SSH schemes, local/Windows paths,
empty host/path, control/NUL/whitespace, password-bearing userinfo, malformed
ports/IP literals, and ambiguous encodings. Do not resolve DNS aliases into one
trust identity. Share the structural parser with publication eligibility so
configuration and transport agree; preserve existing valid SSH cases in tests.

Use the configured URL username. With no explicit username, return
`UsernameRequired` with guidance to configure one rather than guessing from
the OS account or key. A syntactically valid username-less remote may remain
eligible for configuration. If libgit2 asks only for `USERNAME`, return
`Cred::username` using the prepared explicit username. For `SSH_KEY`, return
`Cred::ssh_key` with the selected registration's private path; use no unrelated
public key. No password, agent, default, helper, interactive-signing, or anonymous
fallback. Bound each connection to one selected-key submission and at most one
username negotiation; repeated requests stop with a typed rejection.

Snapshot selection, source identity, endpoint, and pin with short reads; release
all registry/cache/store/Git locks before network work or provider calls.
Observe regular readable sources with existing key APIs, preserving import
registrations and bytes. Recheck selection/source/endpoint after a prompt and
after connection before any transfer closure. Reject changed state and evict
cached state; never silently switch to the newly selected key mid-attempt.
These checks do not claim protection against a malicious process running as
the same user swapping files during a backend path read.

## Session And Connection State Machine

1. Prepare and validate non-secret inputs. No selected key or inaccessible
   source fails before connecting or prompting.
2. Clear any cached secret whose selected key/source no longer matches, even
   if the new key will authenticate without a passphrase. For a matching already
   validated session secret, authenticate with its
   temporary borrow. Add a non-secret `has_cached_passphrase(&UnlockRequest)
   -> bool` query to the session API so the driver can choose this path without
   exposing or duplicating a secret.
3. Otherwise try a fresh connection with no passphrase. Plain keys succeed
   without prompting; unknown/changed host or network failure returns directly.
4. If key authentication/loading fails and no secret has been tried, disconnect
   and drop callbacks, then ask the provider once outside callbacks and locks.
   Retry authentication with that supplied secret using `with_passphrase`.
   A wrong unencrypted key can also reach this prompt: the backend does not
   reliably distinguish it from an encrypted/unsupported key without a secret.
   Add `UnlockReason::{ProtectedKey, AuthenticationAmbiguous}` and a `reason`
   field to `UnlockRequest`. The ambiguous transport path uses the latter;
   existing known-encrypted-key callers use the former. Supply fixed guidance:
   "The selected key could not authenticate. It may need a passphrase, or the
   server may have rejected it. You can supply a passphrase once or cancel."
   This carries the approved explanation to future front ends without adding UI.
5. A repeated credential request, failed backend authentication, source change,
   cancelled unlock, or unavailable provider ends this attempt. Cached-secret
   rejection evicts it and does not prompt again within that attempt. An
   explicit later call may request a new secret.
6. Successful `connect_auth` is necessary, but also require evidence that the
   selected-key callback was used. A server accepting anonymous/none auth does
   not prove the selected-key contract and is rejected.
7. Recheck prepared state and finalize any approved pin. Invoke the operation
   closure once through the scoped adapter. Keep callbacks and the secret borrow
   alive until the connection ends; never return them or a raw Remote to callers.

`connect_auth` also starts the Git service and reads its advertisement. A failure
there may occur after SSH authentication. Do not infer authentication success
from a callback invocation alone or classify every connect failure as an unlock
failure. Record positively established connection success separately from the
later transfer result. When the public backend API cannot distinguish failure
phases, return a conservative typed transport outcome and do not cache a newly
supplied secret; an extra later prompt is preferable to false validation. The
`with_passphrase` closure returns success only after backend authentication;
its value can carry a later transfer error, so a remote repository/protocol
failure does not misreport a successfully used secret as an invalid passphrase.
Any renewed authentication rejection during transfer still invalidates it.
Provider cancellation/unavailability carries pause-required recovery guidance
for future polling; there is no background retry or persistent polling state.

Imported private keys remain backend-validated. Do not pre-parse every imported
file with `ssh-key`, copy private material into application data, or convert it
to another format. Generated Ed25519 and backend-supported external formats
are exercised against the same real server. Error classification must reflect
backend ambiguity rather than claiming that every auth error means wrong secret.

## Host Trust And Persistence

Add `ssh_host_pins(host TEXT, port INTEGER, algorithm TEXT, sha256 TEXT,
PRIMARY KEY(host, port))` through the existing migration transaction. Validate
rows on read; malformed/unavailable state fails closed. Pins are application
global and independent of repository unregister and ordinary index rebuild.
Replacing a structurally corrupt database can lose pins; they cannot be rebuilt
from Git, key registrations, or known_hosts. Approved Q3 requires fresh host
approval even if known_hosts trusts the presented key.

Proposed mechanism: before `replace_corrupt_registry` renames the database or
sidecars, durably publish `ssh-host-trust-reapproval-required` in application
data under the existing exclusive cache-recovery guard. The file contains only
a versioned fixed marker. Use an atomic same-directory publication and verified
platform-appropriate persistence; failure stops recovery before database
replacement. An existing valid marker is idempotent; malformed, unreadable, or
nonregular marker entries fail closed.

The marker disables known_hosts passthrough for unpinned authorities. Explicit
approval creates a new pin, which then works normally. Retain the marker through
ordinary rebuilds and subsequent recovery; it cannot be automatically cleared
because the lost host set is unknown. This also requires explicit approval for
new hosts contacted after recovery. A genuinely new application-data directory
without the marker retains normal known_hosts compatibility. No reset UI is
introduced, and externally deleting all application state is not detectable as
Manyhands recovery.

Snapshot the marker with the pin and recheck both before transfer. A concurrent
recovery changes the trust decision and returns `HostTrustChanged`. Tests cover
interruption before/after marker publication and database rename, marker write
failure, two service instances racing recovery/transport, repeated recovery,
and matching known_hosts after a previously conflicting pin has been lost.
Challenges/approvals are transient; only approved, re-observed pins are persisted. No raw key blobs,
remote URLs, server messages, or credentials enter this table.

The certificate callback always requires an SSH host key and usable SHA-256
identity, and checks its host against the prepared authority. Use raw public
host-key bytes to derive algorithm/fingerprint when needed; never substitute
MD5/SHA-1. Compare an existing pin before considering any inherited trust:

- Equal pin: accept.
- Different pin: reject with exact old/new identities, unless this attempt has
  the matching replacement approval.
- No pin plus matching first-contact approval: accept provisionally.
- No pin and no approval: record the presented identity and return
  `CertificatePassthrough`, preserving libgit2's read-only known_hosts check.
  If it subsequently rejects with a certificate error, map the recorded identity
  to `HostApprovalRequired`. A successful inherited trust check writes no pin.
  If the recovery marker is present, return `HostApprovalRequired` directly
  instead of passthrough, regardless of known_hosts.

The locked git2 wrapper drops libgit2's `valid` flag. The inspected locked
libgit2 SSH backend invokes the callback even for a matching known_hosts entry;
the real regression test must prove pins still win in that case. If a backend
cannot expose a comparable host identity or bypasses that check, fail closed;
do not accept a weaker trust mode.

After authenticated connection and before the transfer closure, use a short
transaction to compare the current pin with the prepared/approved old value.
Insert/replace only when the presented identity exactly matches approval and
the old value still matches. An already installed identical result is
idempotent. Other concurrent changes return `HostTrustChanged`; never overwrite
them. Recheck unchanged pins as well. A failed transaction prevents transfer.
An accepted pin remains accepted if a later Git operation fails: approval
authorizes trust independently of publication. No lock spans network transfer.

## Typed Outcomes And Redaction

`SshTransportError` contains only root, remote name, direction, optional selected
key ID, optional authority, and `SshTransportErrorKind`. Variants:
`ConfigurationInvalid`, `PublicationRemoteMissing`, `UsernameRequired`,
`NoSelectedKey`, `KeyMissing`, `KeyUnreadable`, `KeySourceChanged`,
`SelectionChanged`, `EndpointChanged`, `KeyInvalidOrUnsupported`,
`KeyRejected`, `UnlockCancelled`, `ProviderUnavailable`, `UnlockFailed`,
`HostApprovalRequired { presented }`,
`HostReplacementRequired { expected, presented }`, `HostTrustChanged`,
`HostVerificationUnavailable`, `RegistryUnavailable`, `TransportUnavailable`,
`RemoteUnavailable`, `PushRejected`, and `ProtocolFailure`.

Use fixed guidance per variant, with exact identity fields only for trust
challenges. Callback-local typed failure takes precedence over a generic libgit2
error. Map backend code/class and observed phase without parsing message text;
where invalid format, unsupported algorithm, wrong passphrase, and server auth
rejection are indistinguishable, report `KeyRejected` (or `UnlockFailed` after
a supplied secret) with guidance naming those possible causes. Use the more
specific variants only when reliable evidence supports them. Never expose
`git2::Error` as an error source, Debug member, log, or formatted string.

Discard sideband text and push rejection bodies; retain only a fixed category.
Do not include raw endpoint/userinfo, unknown usernames, key bytes, or callback
values in outcomes. Keep application-controlled secrets in existing zeroizing
holders; libgit2/backend copies remain the dependency's memory boundary, not a
claim of complete process-memory erasure.

## Fixture, Dependencies, And Evidence

Use a test-only `russh` server and Tokio runtime on loopback with an OS-assigned
port, ephemeral Ed25519 host/client keys, and public-key authentication restricted
to the allowed client. Accept only exact `git-upload-pack`/`git-receive-pack`
requests mapped to fixture-owned repositories. Invoke helpers by resolved
executable and separate arguments without a shell. Reject shell, PTY, forwarding,
arbitrary paths/commands, and other auth methods. Drain/capture helper output,
bound startup and test duration, and join/kill children and server on teardown.
Create repositories and commits with git2. Assert server-observed public-key
identity alongside Git OIDs so callback construction cannot masquerade as proof.

Run network cases in isolated child test processes. Set HOME, USERPROFILE,
XDG_CONFIG_HOME, application data, and Git global/system search paths before the
first git2 call; remove inherited SSH-agent and Git environment overrides.
Use fixture-only known_hosts and adversarial default/helper/agent settings.
Never mutate process-global environment in a parallel parent test runner.

Add `russh` and Tokio only under dev-dependencies; resolve a compatible stable
server version and record it in Cargo.lock during the fixture task. Add a
Windows-target direct feature-unification dependency on `libssh2-sys = 0.3.3`
with `openssl-on-win32` and `vendored-openssl`. Inspect the target feature tree
and prove encrypted Ed25519 transport on both native Windows runners. Retain
the existing git2/libgit2 lock versions unless a separately explained backend
compatibility fix requires a change. No production SSH-server/runtime dependency.

Test real plain/encrypted generated and imported Ed25519 keys, an external RSA
PEM fixture supported by the backend, malformed/unsupported inputs, source
replacement, wrong key/passphrase, host approval/replacement/races, isolated
known_hosts, fetch versus push authorities, unavailable server, protocol drop,
privacy, and retry. A test-only helper may generate the external RSA fixture;
missing helper provisioning fails with a fixed message rather than skipping.
Test helpers and generated private files stay outside application-data scans.

Use all five existing native CI targets with Git helper provisioning. Missing
helpers, unsupported algorithms, or runner failures remain explicit failed or
pending evidence; no platform skip may establish compatibility. Later lifecycle
tests reuse this fixture without changing the production system-Git boundary.

## Source Notes And Remaining Risks

- [git2 callback API](https://docs.rs/git2/0.20.4/git2/struct.RemoteCallbacks.html)
  and locally inspected `remote_callbacks.rs`, `cert.rs`, `remote.rs`, and locked
  libgit2 `ssh_libssh2.c` ground the callback/lifetime design. Native tests remain
  necessary for inherited trust and connection reuse behavior.
- [libgit2 certificate callback](https://libgit2.org/docs/reference/v1.9.6/cert/git_transport_certificate_check_cb.html)
  documents the validity/passthrough convention.
- [russh server API](https://docs.rs/russh/latest/russh/server/) supports a
  restricted in-process fixture; exact dependency compatibility is verified in
  the fixture task, not claimed by this plan.
- Blocking backend calls do not promise immediate asynchronous cancellation.
  Prompt cancellation is specified; production stall bounds remain the explicit
  engineering gate above. Later lifecycle code owns reservation/yield semantics.
- Rebase made the local ticket history diverge from its published planning
  branch. Publication requires history reconciliation; planning does not
  authorize a force push, merge, or cleanup.
