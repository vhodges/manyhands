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

## Final Local Verification

Latest fully verified local Rust revision: `2061b0c`. All four required Devenv commands passed:
`cargo check --all-features --locked`, `cargo fmt --check`,
`cargo clippy --all-targets --all-features --locked -- -D warnings`, and
`cargo test --all-features --locked`. The full suite passed 582 tests, including
86 library tests, 28 real SSH fixture cases, 45 real transport cases, and nine
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
| Regression and frontends | All required gates passed. CLI smoke exited zero on the final Rust revision. Desktop launched on an active display without startup errors and remained running until deliberate Ctrl-C (intentional exit 1); endpoint fixes did not change startup wiring. |
| Native platforms | Both Linux and both Windows targets passed on `696a036` in run 37383857982. macOS ARM64 remains unresolved; see the native CI evidence below. |

PR #9 is open. Native macOS CI remains an explicit exit gate; the ticket stays
open. No merge, ticket closure, or worktree cleanup has been performed. The
execution ledger and review reports remain in the ticket worktree.

## Publication And Native CI Follow-up

The user authorized push/PR on 2026-10-05. [PR #9](https://github.com/vhodges/manyhands/pull/9)
preserves the original remote checkpoint with merge `5519991`; its file tree
was verified identical to reviewed `d5deb20`. No force push was used.

Reviewed native compatibility corrections:

- `c9c3dc3`: resolve Git once and support fixed upload-pack/receive-pack builtins
  when standalone aliases are absent. A real SSH regression covers advertisement,
  push OID, command restrictions, cleanup, and repository removal.
- `fee629d`: preserve native Windows Perl/OpenSSL instead of prepending Git's
  MSYS tool directory; probe native Perl and IPC::Cmd before building.
- `ee8153e`: compare platform-dependent credential enums with lossless i64
  widening and scope the Unix-only Read import correctly. Preserve dependency
  caches after failed jobs without changing locked keys or skipping checks.

Each correction received independent scoped review without outstanding findings.
Both Linux and both Windows targets passed build, tests, and artifact upload in
[run 37380377825](https://github.com/vhodges/manyhands/actions/runs/37380377825),
and passed again on `696a036` in
[run 37383857982](https://github.com/vhodges/manyhands/actions/runs/37383857982).
Windows helper packaging, native toolchain selection, and ABI corrections have
native runtime evidence.

macOS has intermittently failed three strict assertions: the withheld receive
status in disconnect_after_receive, KeyRejected in reconnect_key_policy, and
UnlockFailed in renewed_rejection_evicts_secret. In the latest run, the first
reconnect-policy checks passed before renewed rejection failed. Earlier native
macOS runs and local repetitions passed, but no root cause is established.
Passing reruns alone are not accepted as an explanation or fix.

Reviewed diagnostic commits `5ca6680`, `be1bd82`, and `2061b0c` preserve every assertion
and report fixed numeric outcome, operation-phase, server-authentication, helper,
and linked-backend observations. Controlled injected failures exercised the
diagnostic branches and retained the expected assertion failures; mutations were
removed. The macOS-only probe added in `0d4eed0` runs exact cases in fresh
invocations after the unchanged full suite and stops on the first failure.
Shared cfg(test) transfer/backend instrumentation in `2061b0c` records numeric
return categories and backend code/class/authentication flags before caller
assertions. Independent review found no issues; all four local gates passed with
582 tests. The probe now covers all three observed cases, at most 30 triples,
stopping immediately on a failure. This gathers evidence without claiming a
behavioral fix.

Additional implementation rulings and potential costs:

10. Prefer resolved standalone Git helpers, with fixed builtin fallback, exact
    owned repository arguments, and no shell. Cost: revise the fixture discovery
    or invocation boundary.
11. Collect numeric-only phase evidence and preserve immediately captured failed
    conditions. Cost: revise test instrumentation; diagnostics do not prove a fix.
12. Preserve compiled dependency caches after failed native jobs. Cost: clear or
    revise cache policy; cached dependencies never constitute acceptance results.
13. Repeat affected macOS cases in a bounded probe after the full suite, failing
    immediately on an error. Cost: extra CI time or revision/removal of the
    investigative step once sufficient evidence is available.

Detailed checkpoints, individual run outcomes, and review evidence are retained
in the ticket comments and execution ledger. Native macOS acceptance remains
unresolved; merge, closure, and cleanup are not authorized by PR publication.
