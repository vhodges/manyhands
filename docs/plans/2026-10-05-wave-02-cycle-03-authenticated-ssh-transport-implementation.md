---
title: "Wave 02 Cycle 03 Authenticated SSH Transport Implementation Plan"
date: 2026-10-05
status: approved
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M46ECQP3JSA9N0J3F2S3CFTS"
---

# Authenticated SSH Transport Implementation Plan

> **For agentic workers:** Use executing-plans for native execution or
> subagent-driven-development if the user selects it. Read the design first.
> Track each step below and record each task checkpoint on the Cycle ticket.

**Goal:** Prove selected-key Git-over-SSH with explicit host trust, session
unlock, redacted failure behavior, and a reusable native-platform fixture.

**Architecture:** A scoped library driver prepares selected-key/host snapshots,
authenticates with fresh callbacks, finalizes approved trust, and runs one
operation closure. Public connection verification performs no ref transfer.
Fixture-only callers prove fetch/push before downstream lifecycle work begins.

**Tech Stack:** Rust 2024, locked git2 0.20.4/libgit2 1.9.7/libssh2-sys 0.3.3,
rusqlite, existing ssh-key/zeroize session types, test-only russh and Tokio,
Devenv locally and existing native GitHub Actions runners.

**Spec:** [Design](2026-10-05-wave-02-cycle-03-authenticated-ssh-transport-design.md)
and [Cycle](../Cycles/wave-02-cycle-03-authenticated-ssh-transport.md).

**Status:** Approved by the user on 2026-10-05 for subagent-driven development.
Implement the resolved timeout bootstrap in Task 2 and enforce it in Task 4;
record engineering decisions and evidence in the execution ledger. Push/PR, merge,
closure, and worktree cleanup remain separate authorization/lifecycle stages.

## Global Constraints

- Preserve approved Q1–Q3: username in URL, one ambiguity-aware unlock prompt,
  and fresh approval after corrupt-database recovery even when known_hosts matches.
- Initialize fixed backend settings before threads: 10,000 ms per TCP address
  connect attempt and 30,000 ms per blocking SSH call; no total transfer deadline.
  Document DNS/control-call/teardown limits accepted by the user. A fixture
  watchdog never establishes production timeout behavior.

- Work only in ticket `01K7F6H9J2N4Q6S8V0X2Z4B6DB`'s existing branch/worktree.
- Shared domain code belongs in the library and depends on no GPUI types.
- Fresh callbacks and Git handles remain inside one operation/thread.
- Exactly the selected key; no agent/default/helper/password/anonymous fallback.
- Imported private material is backend-validated, retained in place, and never
  copied to application data or made subject to an application parser gate.
- Host authority is normalized host plus numeric port (default 22); existing
  pin wins over known_hosts, replacement requires exact old/new approval.
- No provider call or network work under registry/cache/store/Git locks.
- No raw backend/server error, passphrase, private bytes, or callback values in
  diagnostics, persisted state, test failure output, or snapshots.
- Windows uses libssh2-sys 0.3.3 features openssl-on-win32 and vendored-openssl.
- Local Rust commands use `devenv shell -- cargo ...`; preserve Cargo.lock with
  dependency edits and devenv.lock if Devenv inputs change.
- No production sync, refspec policy, polling, reservations, UI, CLI grammar,
  external Git processes, or lifecycle recovery implementation in this Cycle.

## Review Focus

1. A valid inherited known_hosts entry must not defeat an existing conflicting
   pin; test on the real backend in Task 3.
2. A selection/source/endpoint or pin change while a prompt/connection is open
   must prevent transfer; Task 4 tests both before and after authentication.
3. A constructed Cred, repeated callback, or anonymous server must not cache a
   secret or claim selected-key success; Tasks 3 and 4 use server evidence.
4. A Windows helper path containing spaces or non-default-port authority must
   behave like other platforms; Tasks 2 and 5 run native real transfers.
5. A malformed key or hostile server message must not escape through Debug,
   an error source, WAL, or subprocess diagnostics; Task 5 scans every surface.

## File Map And Dependencies

| Files | Responsibility |
| --- | --- |
| `src/repository/transport/{mod,endpoint,error}.rs` | Public values, URL policy, fixed error guidance (Task 1). |
| `src/repository.rs` | Module export and shared SSH eligibility (Task 1); trust marker before corrupt-registry replacement (Task 3). |
| `src/repository/transport/{trust,callbacks}.rs` | Pin persistence and callback policy (Task 3). |
| `src/repository/discovery.rs` | Host-pin migration call (Task 3). |
| `src/repository/transport/operation.rs` | Session/connection driver (Task 4). |
| `src/repository/keys/session.rs` | Non-secret cache-presence query and typed unlock reason (Task 4). |
| `tests/support/ssh_remote.rs`, `tests/support/ssh_server.rs`, `tests/support/ssh_harness.rs` | Disposable SSH fixture ownership, restricted session/helper relay, and custom-main runner (Task 2). |
| `src/runtime.rs`, `src/lib.rs`, `src/main.rs`, `src/bin/manyhands-cli.rs` | Safe pre-thread timeout bootstrap and startup wiring (Task 2). |
| `tests/ssh_fixture.rs`, `tests/ssh_transport.rs` | Fixture and public-operation evidence (Tasks 2–5). |
| `src/repository/transport/tests.rs` | Private-driver transfer tests using shared fixture (Tasks 3–5). |
| `tests/ssh_transport/privacy.rs` | Isolated privacy subprocess (Task 5). |
| `Cargo.toml`, `Cargo.lock`, `.github/workflows/build.yml` | Backend features, test dependencies, native fixture provisioning (Tasks 2, 5). |

Order: baseline, Task 1, Task 2, Task 3, Task 4, Task 5. Task 2's fixture is
independently testable and Task 3's policy tests precede session integration.
Each task is a reviewable deliverable with a ticket comment and coherent local
commit during authorized implementation.

## Baseline Checkpoint

- [x] Read AGENTS.md and implementing-a-cycle; repeat its fetch/rebase/ancestry
  preflight at implementation start. Preserve unrelated edits.
- [x] Run the four required commands listed under the final gate below before
  Rust changes. Record results, base/head SHAs, approved design, chosen execution
  method, and any pre-existing failures in the ticket.

## Task 1: Define Transport Contracts And One SSH Endpoint Parser

**Files:** create `src/repository/transport/mod.rs`, `endpoint.rs`, `error.rs`;
modify `src/repository.rs`; test in `endpoint.rs` and `error.rs` unit modules
and `tests/repository_enablement.rs`.

**Interfaces:** define all public types and fields enumerated by the design,
including `SshTransportErrorKind`. Private
`parse_ssh_endpoint(&str) -> Result<SshEndpoint, SshTransportErrorKind>` returns
validated authority, optional username, and private connection URL.
`SshTransportError::guidance(&self) -> &'static str` returns only fixed text.
Keep public service method implementation for Task 4.

- [x] Add failing parser tests: `ssh://git@EXAMPLE.com/repo` and
  `git@example.com:repo` yield host `example.com`, port `22`; explicit `2222`
  differs; equivalent bracketed IPv6 normalizes to the same IP. Reject HTTP(S),
  `C:\\repo`, `file:/repo`, password-bearing userinfo, port 0/65536, empty path,
  malformed brackets, encoded authority ambiguity, whitespace and NUL.
- [x] Add tests preserving prior repository eligibility cases and accepting a
  structurally valid username-less SSH URL; credential use later returns
  `UsernameRequired`. Error formatting and `source()` must expose no backend
  error/URL and use fixed category text.
- [x] Run `devenv shell -- cargo test --locked --lib transport` and targeted
  repository-enablement tests; record the intended missing-contract failures.
- [x] Implement the types/parser/error map. Replace duplicated structural SSH
  checks with this parser without changing remote mutation/confirmation behavior.
- [x] Read raw configured URLs separately from effective Remote URLs. Test local
  and global `insteadOf`/`pushInsteadOf`, including same-host username/path changes;
  refuse a changed effective endpoint before any connection or credential use.
- [x] Rerun those tests to green; record and commit `feat: define SSH transport contracts`.

## Task 2: Deliver A Restricted Portable SSH Git Fixture

**Files:** create `tests/support/ssh_remote.rs` (fixture ownership/config),
`tests/support/ssh_server.rs` (restricted session/helper relay),
`tests/ssh_fixture.rs`; modify
Cargo.toml/Cargo.lock, src/runtime.rs, src/lib.rs, both binary entry points,
and tests/support/ssh_harness.rs. Use equivalent relative repository-scoped
visibility in keys/registry.rs and transport/endpoint.rs where custom-host
source inclusion requires it; do not widen production visibility.
Configure harness=false for both SSH test
executables; the Task 4 transport host is added when its cases exist.
Update CI helper provisioning as needed. Use a dedicated
support module import rather than adding server dependencies to every fixture.

**Interfaces:** `SshRemoteFixture::start() -> Result<Self, FixtureError>` owns
temporary repositories, server runtime and children; `url() -> String`,
`host_identity() -> HostKeyIdentity`, `allowed_client_public_key() -> Vec<u8>`,
`rotate_host_key()`, `reject_client()`, and `disconnect_at(FixtureBoundary)`
provide deterministic scenarios. Expose non-secret accepted-key identities and
helper invocation counters. `FixtureError` formats fixed messages only.
`run_isolated(case: &str)` launches an exact child test case with fixture-only
environment set before any Git initialization and bounded process lifetime.

- [x] Add failing fixture tests for successful allowed-client authentication,
  wrong-key denial, command/path restrictions, EOF/exit forwarding, helper
  lookup with spaces, startup failure, and complete cleanup after failure.
- [x] Test and implement the design's unsafe pre-thread runtime initializer and
  fixed errors; call it first in both binaries. Test exact settings read-back
  in the custom-main host before any thread. Preserve GPUI Kit Root/init rules.
- [x] Resolve test-only russh/Tokio versions and pin the resulting lockfile;
  keep existing git2/libgit2/libssh2 versions. Add Windows-target libssh2-sys
  feature unification. Check native helper discovery via `git --exec-path`
  only in tests; provision Git and any external-key fixture generator explicitly
  in CI rather than assuming daemon/system user availability.
- [x] Implement the loopback server with only ephemeral host/allowed client
  keys. Map exact upload/receive-pack commands to owned bare repositories;
  spawn helpers with separate arguments and pipes, no shell. Reject every
  other auth method, command, arbitrary path, forwarding, shell, and PTY.
- [x] Make child harnesses isolate HOME/USERPROFILE/XDG/Git search paths and
  SSH-agent settings. Drain bounded captured diagnostics and redact fixture
  failures. Use a startup timeout and watchdog, plus deterministic teardown.
- [x] Run `devenv shell -- cargo test --locked --test ssh_fixture` to green.
  Use small direct git2 fixture clients here; production-policy proof follows.
- [x] Characterize the locked backend with deterministic pre-handshake, auth,
  advertisement, and transfer stalls. Record which phases support timed failure
  and which errors distinguish auth rejection from transport failure, without
  matching backend message strings. Validate any timeout bootstrap in an isolated
  process before threads; do not mutate libgit2 global timeouts from test workers.
- [x] Verify the accepted 10,000/30,000 ms defaults, a shorter recoverable stall,
  per-call timeout after the applicable threshold with bounded scheduling tolerance,
  and a progressing multi-call transfer lasting longer than 30 seconds. Record phases
  the backend cannot bound; a test watchdog must not turn that gap into a pass.
- [ ] Native execution pending authorized CI. Before depending on the fixture for all later tasks, obtain native evidence
  for helper invocation and encrypted Ed25519 on both Windows architectures when
  authorized CI is available. Otherwise label that feasibility gate pending and
  do not claim cross-platform fixture support.
- [x] Inspect `devenv shell -- cargo tree --locked -e features --target x86_64-pc-windows-msvc -i libssh2-sys`
  and the ARM64 equivalent; verify both required OpenSSL features. Record resolved
  versions and commit `test: add disposable authenticated SSH Git fixture`.

## Task 3: Enforce Host Trust And Selected-Key Callback Policy

**Files:** create `transport/trust.rs`, `callbacks.rs`, `tests.rs`; modify
transport/mod.rs, repository/discovery.rs, src/repository.rs recovery hook,
and tests/recovery_foundation_gate.rs. Pure policy tests stay in the library;
real private-driver scenarios run through the Task 2 custom SSH test host and
cfg(test) dispatcher so helpers remain crate-private and bootstrap precedes threads.

**Interfaces:** `read_host_pin(&self, &SshAuthority) -> Result<Option<HostKeyIdentity>, SshTransportErrorKind>`;
`finalize_host_pin(&self, &SshAuthority, expected: Option<&HostKeyIdentity>,
observed: &HostKeyIdentity, approval: Option<&HostApproval>) -> Result<(), SshTransportErrorKind>`.
`migrate_host_pins(&rusqlite::Transaction<'_>) -> Result<(), RepositoryError>`
runs in the existing migration. Operation-local `CallbackAttempt` owns only
typed failure, host observation, and credential-submission counters;
`build_callbacks<'a>(&'a PreparedSshAttempt, Option<&'a str>, &'a CallbackAttempt)
-> git2::RemoteCallbacks<'a>` uses interior mutation for that private state.
`PreparedSshAttempt` holds the selected registration/source/endpoint/pin snapshot.

- [ ] Add failing migration and trust tests for preserved registry rows, port
  separation, malformed stored identity, unknown host, matching approval,
  mismatched approval, changed pin, exact replacement, idempotent duplicate
  approval, two services racing different approvals, and registry failure.
- [ ] Implement the design's durable `ssh-host-trust-reapproval-required` marker
  in transport/trust.rs and invoke it before corrupt-registry rename in
  src/repository.rs. Extend tests/recovery_foundation_gate.rs and transport tests:
  matching known_hosts cannot bypass fresh approval after pin loss; interruptions,
  repeated recovery, and concurrent readers fail closed; marker publication
  failure prevents replacement. Ordinary rebuild preserves pins. Never clear the
  marker automatically or claim pins are reconstructible from Git/known_hosts.
- [ ] Add actual handshake tests: matching known_hosts permits an unpinned
  host; a conflicting Manyhands pin rejects that same otherwise-trusted host;
  missing comparable identity fails; known_hosts bytes never change.
- [ ] Add callback tests for explicit SSH key and username-only negotiation,
  repeated credential requests, forbidden credential types, authority mismatch,
  and agent/default/helper fallback refusal. Raw callback errors remain fixed.
- [ ] Run `devenv shell -- cargo test --locked --lib transport` and observe
  intended pure-policy failures; run real handshake cases in the initialized
  custom SSH test host. Then implement migration, CAS trust finalization, and
  callbacks using the exact decision table in the design. No prompt or SQL
  write inside callbacks. Preserve the passthrough observation for error mapping.
- [ ] Rerun to green against the real fixture. Require an observed host check
  even for a valid inherited known_hosts entry. Record and commit
  `feat: enforce selected SSH key and explicit host trust`.

## Task 4: Integrate Session Unlock And Scoped Connection Verification

**Files:** create `transport/operation.rs`, `tests/ssh_transport.rs`; modify
transport/mod.rs, transport/tests.rs, keys/session.rs, tests/session_credentials.rs.

**Interfaces:** implement `RepositoryService::verify_ssh_transport` and
`with_authenticated_remote` with the scoped `AuthenticatedSshRemote` adapter
specified in the design. Require published runtime bootstrap success before
networking (RuntimeUninitialized otherwise). Use a custom-main SSH transport test
host as in Task 2; private scenarios use the design's test-only source inclusion.
The adapter owns fresh options/callbacks for every
transfer; it exposes no raw Remote. Add
`SessionCredentials::has_cached_passphrase(&self, &UnlockRequest) -> bool`;
it discloses only whether key/source match, never a secret. Use existing
`with_passphrase`, `invalidate`, `KeySourceToken::observe`, and registry APIs.
Add/export `UnlockReason::{ProtectedKey, AuthenticationAmbiguous}` through
keys/session.rs and keys/mod.rs, add `reason` to `UnlockRequest`, and update
existing constructors/tests. The transport ambiguity path carries the design's
fixed guidance; existing known-encrypted callers retain `ProtectedKey`.

- [ ] Add real tests for plain and encrypted generated/imported keys; include
  backend-compatible external RSA PEM. Assert accepted client public identity,
  successful advertisement read, provider call count (zero for plain, one for
  repeated encrypted use), and no verification-side Git mutation.
- [ ] Add failure tests: absent selection, missing/denied/nonregular source,
  malformed/unsupported key, wrong key, wrong passphrase, cancelled/unavailable
  provider, rejected cached secret, source replacement, new session, and a server
  permitting anonymous auth. Assert typed category, no automatic repeated prompt,
  unchanged registrations/bytes, and an explicit later retry can succeed.
- [ ] Assert that ambiguous failure calls the provider once with
  `AuthenticationAmbiguous`, including a wrong unencrypted key. The request must
  explain both possible causes; cancellation or rejection cannot prompt again.
- [ ] Add deterministic seams after preparation, after provider response, and
  after authenticated connection. Change selection/source/URL/pin at each
  applicable seam and assert the transfer closure invocation count stays zero.
  A provider that performs a registry selection operation must not deadlock.
- [ ] Run `devenv shell -- cargo test --locked --test ssh_transport --test session_credentials`
  and private transport tests; record intended failures.
- [ ] Implement prepare/connect/prompt/retry/finalize/use/drop flow with fresh
  callbacks each connection. Resolve direction-specific endpoint and use the
  exact validated endpoint. Provider runs after failed callbacks are dropped.
  Bound one secret-bearing attempt per operation and cache only backend success.
- [ ] Distinguish confirmed connection success from failures during remote Git
  service startup/advertisement. Test post-authentication advertisement failure:
  no false invalid-passphrase claim and no newly cached unverified secret.
- [ ] Prove fetch and push through the private driver in fixture repositories
  with explicit test refspecs and expected OIDs. Prove that renewed callbacks
  on reconnect use the same selected key and host policy. Do not add public
  production fetch/push or lifecycle APIs.
- [ ] Test a per-ref push rejection whose top-level Git call succeeds; the
  adapter must return `PushRejected` and discard server text. Verify callback
  replacement by fetch/push cannot bypass host/key policy or rejection checks.
- [ ] Rerun to green; record cache/lock/state-preservation evidence and commit
  `feat: integrate session credentials with authenticated transport`.

## Task 5: Prove Failure Privacy And Native Compatibility

**Files:** create `tests/ssh_transport/privacy.rs`; extend transport tests,
SSH fixture and `.github/workflows/build.yml`; update Cycle/design/ticket
evidence with actual results, leaving unavailable platform gates pending.

**Interfaces:** retain the production public contract. Extend fixture failure
boundaries to before auth, after auth/before advertisement, during fetch, and
after receive-pack. No raw server detail is a public recovery value.

- [ ] Add tests for unavailable loopback endpoint, inaccessible remote repo,
  hostile sideband/rejection text, protocol disconnect, distinct fetch/push host
  pins, and URL rewrite attempts. Before-transfer failures preserve refs,
  FETCH_HEAD, index/worktree/canonical bytes and key state. Post-transfer tests
  inspect actual state and never claim rollback or safe blind push retry.
- [ ] Add an isolated child privacy scenario with unique correct/incorrect
  passphrases, malformed-key markers, private encodings/segments, server text,
  and callback-only username markers. Capture formatted errors, Debug, error
  sources, guidance, stdout/stderr; scan live DB/WAL, backups/journal, and Git
  metadata. Keep private fixtures outside scan roots. Report only fixed failure
  messages, never leaked values or byte diffs.
- [ ] Run the focused transport tests red/green, then temporarily inject a
  diagnostic leak and a persistence leak to prove the scanner detects each;
  remove mutations and rerun. Record the evidence without secret output.
- [ ] Extend the existing headless CI test command with `--test ssh_fixture
  --test ssh_transport`; keep `--lib` so private-driver tests execute. Preserve
  release builds/artifacts. Require runtime tests on all five native targets;
  do not skip Windows ARM or substitute local-file remotes for SSH proof.
- [ ] Run the final gate below, record results/native evidence and remaining
  limitations. Commit `test: prove SSH transport privacy and platform contracts`.
- [ ] Request independent whole-branch code review, address findings, and rerun
  affected checks. Keep the ticket open until code-review or PR approval.

## Final Verification And Handoff

Run from the ticket worktree:

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
devenv shell -- cargo run --locked --bin manyhands-cli
git diff --check
```

All commands must exit zero; tests must report no unexpected skips/failures.
If desktop code changes, run the mandated desktop smoke test with an active
display. Record native job URLs and exact commits separately from local results.
CI publication happens only when push/PR work is authorized.

Each task comment records changes, decisions, commands/results, and current
HEAD. Final evidence maps every Cycle acceptance row to tests/results. Keep
Cycle 04's ref/reservation/polling work and Cycle 05's sync policy explicit.
Do not close this ticket, publish divergent history, merge, or remove the
worktree as part of this planning handoff.
