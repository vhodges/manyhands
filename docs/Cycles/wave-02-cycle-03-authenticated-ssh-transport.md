---
title: "Wave 02 Cycle 03: Authenticated SSH Transport"
date: 2026-10-05
status: approved
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M46ECQP2X5Q4QGXAX8P8V5AH"
---

# Wave 02 Cycle 03: Authenticated SSH Transport

## Purpose And Authority

Connect the selected shared SSH key and explicit host trust to operation-scoped
`git2` callbacks. Prove real authentication, session reuse, and redacted failure
behavior using disposable SSH Git repositories. This is Cycle 03 of
[Wave 02](../Waves/wave-02-collaboration.md), tracked by ticket
`01K7F6H9J2N4Q6S8V0X2Z4B6DB`.

The [authentication RFC](../RFC/authentication-and-credential-handling.md),
[persistence RFC](../RFC/repository-index-persistence-and-refresh.md),
[Git workflow RFC](../RFC/git-workflow-and-conflict-recovery.md), and
[test strategy](../RFC/test-and-compatibility-strategy.md) remain authoritative.
Traceability: `MH-CRED-001`, `MH-NFR-003`, and the Wave's Cycle 03 exit gate.

Read the [design](../plans/2026-10-05-wave-02-cycle-03-authenticated-ssh-transport-design.md)
and [implementation plan](../plans/2026-10-05-wave-02-cycle-03-authenticated-ssh-transport-implementation.md)
together. The user approved scope, design, and plan on 2026-10-05 and authorized
implementation using subagent-driven development.
The user approved all three review decisions on 2026-10-05: username in URL,
one ambiguity-aware unlock prompt, and fresh host approval after database
recovery even when known_hosts matches. The user also accepted the documented
backend timeout limits: 10 seconds per TCP address connection attempt and
30 seconds per blocking SSH call, with no total transfer deadline. The design
specifies early initialization and custom test hosts; native evidence must
prove the resulting behavior and preserve DNS/control-call/teardown caveats.

## Entry Evidence

- Existing ticket, branch, and worktree reused. Fresh fetch observed
  `origin/main` at `89ce24d0c5b99c817ec81615fe610f65d7c81a99`.
- Local `main` is four commits ahead of that remote, including repository Cycle
  skills, tooling, and research. Rebased onto local `main` at
  `a21a31aeea5aaf7c03cf7692848ddadb5dde1242` to preserve those additions.
- Ticket HEAD changed from `edb5ea92b8e5c3c4183cf06dbef334942a3b2a07` to
  `305c0f0227ba2d6cfa9b1f807cbeaa474374cfa5`, without conflicts. Both main
  ancestry checks passed; the ticket worktree was clean after rebase.
- Cycle 02's closing ticket comment records approved PR #8, 480 passing tests,
  CLI/desktop smoke evidence, and all five native CI targets passing in
  [run 37331922078](https://github.com/vhodges/manyhands/actions/runs/37331922078).
  This is prior recorded evidence, not a fresh Cycle 03 test run.
- Before Rust implementation, repeat the work-boundary preflight and required
  local baseline checks. Record any pre-existing failure separately.

## Scope

- SSH URL normalization and action-specific fetch/push endpoint resolution.
- Selected-key-only callbacks; no agent, default-key, helper, password, or
  anonymous authentication success.
- First-use imported-key backend validation, protected-key provider handoff,
  successful-authentication caching, and rejection/source-change eviction.
- Application-local host pins, exact first-contact approval and exact old/new
  replacement approval, with read-only `known_hosts` compatibility.
- A headless connection-verification operation and a crate-private authenticated
  transport driver usable by later remote operations.
- Typed credential, trust, configuration, and transport failures with stable
  redacted guidance and no raw backend/server text.
- A real loopback SSH Git fixture, including fetch and push proof through the
  same driver, privacy checks, and native-platform CI coverage.
- Required Windows OpenSSL feature configuration for locked `libssh2-sys` 0.3.3.

## Exclusions And Downstream Obligations

Cycle 04 owns exact production fetch refspecs, remote-ref interpretation,
durable remote operations, reservations, and polling state. Cycle 05 owns
production synchronization and push policy. Later Cycles own merge, comments,
materialization, promotion, closure, and cleanup. No desktop/CLI commands,
resident scheduler, cloning, HTTP(S) publication, or key-upload flow is added.

The fixture's explicit ref transfers prove the transport mechanism; they do
not implement publication semantics. Production connection verification reads
the advertisement and disconnects without updating refs, objects, FETCH_HEAD,
the index, worktrees, tracked configuration, or canonical content.

Credential cancellation/provider unavailability returns a terminal result for
this attempt. Later polling must pause on it; this Cycle adds no polling table.
Every future reconnect must create fresh callbacks under this same policy.

## Acceptance And Exit Evidence

| Contract | Required evidence |
| --- | --- |
| Selected key | Real fetch and push with selected generated/imported keys; wrong/absent keys fail even with a usable default key, helper, or agent environment. |
| Unlock | Encrypted key authenticates with one successful prompt per unchanged session/source; wrong secret is not cached; rejection evicts cached state; cancellation/unavailable provider stops without repeated prompting. |
| Host trust | Unknown host produces exact challenge; stale approval fails; changed host requires old/new approval; pin wins over matching known_hosts; port separates authorities; no known_hosts writes. Corrupt-registry recovery follows the explicit Q3 decision. |
| State preservation | Authentication/trust/access failures preserve refs, worktrees, key registrations, key bytes, and existing pins; an approved pin is the only intentional persistent transport-authentication change. |
| Privacy | Captured errors, Debug, stdout/stderr, application DB/WAL/backups, and Git metadata exclude secret markers and raw server messages. |
| Backend | Native real encrypted Ed25519 transport on Linux x86-64/ARM64, Windows x86-64/ARM64, macOS ARM64. Unavailable evidence remains pending. |
| Stalled transport | Production timeout/cancellation scope is specified and proven independently of fixture watchdogs; no per-operation mutation of libgit2 global settings. |
| Regression | Required local Devenv gates and CLI smoke pass; record desktop smoke if desktop code changes. |

Mid-transfer interruption may leave Git objects or an ambiguous remote push;
this Cycle does not claim rollback of those effects. Tests distinguish failures
before transfer from post-authentication protocol failures. Recovery and safe
retry of mutating operations remain explicit downstream work.

Record planning, baseline, each implementation task, verification, and review
as ticket comments. Keep the ticket open until code-review or PR approval.

## Implementation Review Evidence

All five implementation tasks received independent specification and quality
review. Whole-branch review against `a21a31a` found two endpoint defects: SCP
normalization could change relative repository paths, and URL query/fragment
suffixes could differ from the backend service path. Commit `753bb72` preserves
SCP syntax/path bytes, rejects URL suffixes before networking, and conservatively
rejects cross-form rewrites. A scoped independent re-review approved both fixes
and found no new breakage. All earlier task-review findings are resolved.

The endpoint regressions include 28 exact upload/receive command observations,
plus two backend port-syntax characterization observations. Test-only authority
substitution preserves path bytes and does not expand production parser syntax.
The full-suite schema expectation was also corrected in `3e87c1b` to include the
intentional host-pin table; the strict ordered-table assertion remains.

## Recorded Implementation Rulings

These are implementation decisions within the approved scope, in decision order.
Each includes the potential rework or user cost if the decision proves wrong.

1. Initialize backend limits before threads in both binaries and custom SSH test
   hosts; require initialized status in the driver. Cost: revise startup/test
   integration if the boundary proves unsuitable.
2. Split fixture ownership/configuration from the restricted server/helper relay.
   Cost: redraw the test-only module boundary.
3. Use equivalent relative repository visibility for source inclusion in custom
   test hosts, without widening public APIs. Cost: revise test-host integration.
4. Read and finalize the recovery marker and host pin together under one guard.
   Cost: revise the private trust snapshot interface and its driver consumer.
5. Split operation/session flow, scoped remote adapter, and private test dispatch
   into focused modules. Cost: consolidate or adjust those private boundaries.
6. Give the private remote adapter separate repository and connection-borrow
   lifetimes. Cost: revise its private borrowing interface and covering tests.
7. Permit the approved single ambiguity prompt after a generic SSH failure only
   when host observation and selected-key submission occurred, no typed policy
   failure occurred, and no secret was tried. Guidance includes possible
   connection failure; subsequent generic failure remains uncached and typed as
   transport unavailable. Cost: an unnecessary one-time prompt after a connection
   or service failure; richer backend evidence may refine this later.
8. Share a privacy scanner across focused failure/privacy test modules. Cost:
   revise the test-only scanner/module boundary. Capture overflow, missing probes,
   and drain failures fail closed.
9. Preserve SCP syntax/path bytes, reject URL query/fragment delimiters, and reject
   cross-form rewrites conservatively. Cost: a benign rewrite may require an
   explicit URL, or safe equivalence handling may need refinement.

10. Prefer resolved standalone Git helpers, with fixed builtin fallback, exact
    owned repository arguments, and no shell. Cost: revise fixture discovery or
    invocation boundaries.
11. Collect numeric-only diagnostic evidence while preserving captured failure
    conditions and strict assertions. Cost: revise test instrumentation.
12. Preserve compiled dependency caches after failed native jobs. Cost: clear or
    revise cache policy; cached dependencies never constitute acceptance results.
13. Use a bounded, fail-first macOS probe during investigation, moving it before
    the full suite for faster evidence. Remove it after native confirmation as
    requested by the user. Cost: extra CI time during investigation.
14. Isolate fixture Git-child SIGCHLD from the isolated client test thread while
    server runtime workers receive it and reap helpers; restore the exact caller
    mask after cleanup and unwind. Cost: Unix test signal-routing complexity;
    a separate server process is a larger fallback.

## Final Local Verification

Latest fully verified local Rust revision: `bbeef25`. All four required Devenv commands passed:
`cargo check --all-features --locked`, `cargo fmt --check`,
`cargo clippy --all-targets --all-features --locked -- -D warnings`, and
`cargo test --all-features --locked`. The full suite passed 585 tests, including
86 library tests, 31 real SSH fixture cases, 45 real transport cases, and nine
documentation tests, with no failures or ignored tests. Full local log:
`/tmp/manyhands-cycle03-final-tests.log`.

| Acceptance contract | Final evidence |
| --- | --- |
| Selected key and endpoints | Real generated/imported Ed25519 and external RSA PEM authentication; fetch/push OIDs; wrong/default/anonymous rejection; exact path commands and zero-network invalid/rewrite preflight. |
| Unlock | Eleven session tests plus real transport cases cover bounded prompting, successful cache reuse, wrong-secret exclusion, cancellation/unavailable provider, and rejection/source-change eviction. |
| Host trust | Real unknown/stale/replacement/known_hosts/port cases, 50 recovery tests, corrupt replacement requiring fresh approval, combined trust snapshot/CAS races, and distinct fetch/push pins. |
| State preservation | Populated refs, objects, index, dirty worktree, FETCH_HEAD, canonical content, registrations, and private-key bytes compared across before-transfer failures and retries. Lost push response test observes the changed remote without promising rollback. |
| Privacy | Raw stdout/stderr scanned before filtering; populated DB/WAL/backups/journal/Git state scanned. Temporary stdout, stderr, and retained-WAL-backup secret injections each failed safely; mutations removed and clean cases passed. |
| Stalled transport | Exact 10,000/30,000 ms settings verified. Handshake/authentication/advertisement/transfer stalls returned at about 30 seconds; delayed command acknowledgment plus cleanup took about 45 seconds. A progressing transfer succeeded after 42.093 seconds. DNS, multiple address attempts, and cleanup remain outside a total deadline guarantee. |
| Regression and frontends | All required gates passed. CLI smoke exited zero before the test-only CI amendments. Desktop launched on an active display without startup errors and remained running until deliberate Ctrl-C (intentional exit 1); subsequent amendments did not change startup wiring. |
| Native platforms | All five native build/test/artifact jobs passed on `e75e768` in run 37390262561: Linux x86-64/ARM64, Windows x86-64/ARM64, and macOS ARM64. macOS also passed all 90 investigative probe cases and the three permanent signal regressions. |

PR #9 is open and all native targets have passed. The ticket stays
open pending the later merge/closure lifecycle. No merge, ticket closure, or worktree cleanup has been performed. The
execution ledger and review reports remain in the ticket worktree.

## Publication And Native CI Follow-up

The user authorized push/PR on 2026-10-05. [PR #9](https://github.com/vhodges/manyhands/pull/9)
preserves the original remote checkpoint with merge `5519991`; its file tree
was verified identical to reviewed `d5deb20`. No force push was used.

Independently reviewed compatibility corrections support Git builtin helpers on
Windows (`c9c3dc3`), preserve native Perl/OpenSSL (`fee629d`), and compare
platform-dependent credential enum types with lossless widening (`ee8153e`).
Both Linux and both Windows targets passed repeatedly, most recently on
`e53f293` in [run 37387522542](https://github.com/vhodges/manyhands/actions/runs/37387522542).

The intermittent macOS assertions were narrowed through numeric-only diagnostics
and a bounded, fail-first probe. That run confirmed EINTR during the TCP connect
wait before SSH host/key callbacks (observation `512 1 1`). A controlled real
libgit2/Manyhands reproduction showed the same failure when SIGCHLD interrupted
the wait. The in-process fixture's local Git helpers can signal the client when
they exit, unlike helpers on a real remote server. Native logs confirm EINTR but
do not themselves record the signal number.

Reviewed fixture-only correction `8e5ef0e` isolates this signal coupling. One
thread-bound guard wraps each isolated Unix case inside catch_unwind and restores
the complete caller mask after fixture cleanup. Server async and blocking workers
explicitly receive SIGCHLD and continue reaping children. Windows, production
transport behavior, overlapping reconnect/cleanup, and strict assertions remain
intact. There are no retry-to-green changes or settling sleeps.

Three regressions demonstrate actual poll-interruption protection, exact mask
restoration on success/error/unwind, and worker delivery/helper reaping with two
live fixtures. The old harness failed interruption protection; a client-only
guard failed worker delivery; the complete correction passes. All required local
gates passed with 585 tests (31 Unix fixture and 45 transport cases). Windows
retains 28 fixture cases because the signal tests are Unix-specific. Independent
scoped review approved the correction without findings.

On `e75e768`, [run 37390262561](https://github.com/vhodges/manyhands/actions/runs/37390262561)
passed every native build, full-suite, and artifact job on all five targets.
macOS also passed all 30 probe rounds (90 fresh case invocations) and all three
permanent signal regressions. The correction now has complete native evidence.
Cleanup `bbeef25` removes the temporary probe, observations 506–512, and their
unused diagnostic helpers/counters. The permanent signal fix, three regressions,
strict assertions, baseline timeout/privacy observations, and every required
native matrix/full-suite step remain. Independent review approved the cleanup
without findings. All four required local gates passed again with 585 tests.
The final published revision receives the normal PR matrix checks.

Detailed checkpoints and earlier run/review evidence remain in the ticket
comments and execution ledger. The ticket stays open. Merge, closure, and
worktree cleanup remain separate lifecycle steps.
