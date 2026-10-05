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

Final Rust revision: `753bb72`. All four required Devenv commands passed:
`cargo check --all-features --locked`, `cargo fmt --check`,
`cargo clippy --all-targets --all-features --locked -- -D warnings`, and
`cargo test --all-features --locked`. The full suite passed 581 tests, including
86 library tests, 27 real SSH fixture cases, 45 real transport cases, and nine
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
| Native platforms | Pending. The five-target workflow includes the new tests, but no new native CI run has been published. Linux x86-64 local evidence does not establish Linux ARM64, Windows x86-64/ARM64, or macOS ARM64 acceptance. |

Implementation is ready for publication review. No push, PR, merge, ticket
closure, or worktree cleanup has been performed. Native CI remains an explicit
exit gate; the ticket stays open. The execution ledger and review reports remain
in the ticket worktree for continuation.

## Publication And Native CI Follow-up

The user authorized push/PR on 2026-10-05. [PR #9](https://github.com/vhodges/manyhands/pull/9)
preserves the original remote checkpoint with merge `5519991`; the resulting
file tree was verified identical to reviewed `d5deb20`.

[Initial native run](https://github.com/vhodges/manyhands/actions/runs/37373092577)
passed macOS ARM64 and Linux ARM64. Linux x86-64 was cancelled before execution;
both Windows jobs failed setup because standalone Git builtin aliases were absent.
Reviewed fix `c9c3dc3` supports fixed Git builtin commands as a test-only fallback.
Its real SSH regression checks advertisement, push OID, command restrictions,
helper cleanup, and repository removal. Independent scoped review approved it
without findings. All four required local gates passed again: 582 tests, including
28 fixture and 45 transport cases, no failures or ignored tests. Production
startup wiring is unchanged, so the recorded frontend smokes still apply.

Additional ruling: resolve Git once and use fixed builtin subcommands when
standalone helpers are absent, preserving standalone preference, the exact SSH
allowlist, separate owned repository arguments, and no shell. Potential cost if
wrong: revise the test helper discovery/invocation boundary.

Native acceptance remains pending a complete run on this amended revision.

### Windows Toolchain And Intermittent macOS Follow-up

CI follow-up after run 37375603743: Linux x86-64/ARM64 passed. Both Windows
jobs passed helper provisioning but failed vendored OpenSSL configuration because
the workflow's Git usr/bin override selected incompatible MSYS Perl. Workflow-only
fix fee629d removes that override, preserves native OpenSSL, and probes native
Perl/IPC::Cmd early. Independent scoped review approved it without findings.

macOS failed disconnect_after_receive at the withheld-status assertion before
remote-result checks. Observation 189 was a panic source line, not a backend
error. Thirty isolated local repetitions passed. No root cause is established.
Diagnostic-only 5ca6680 preserves every assertion and captures the withheld flag
before logging fixed numeric outcome/helper/ref-match categories. A controlled
failure proved the diagnostic branch while retaining the assertion. Independent
review approved the instrumentation without findings.

All four required local gates pass on 5ca6680: 582 tests, including 28 fixture
and 45 transport cases, no failures or ignored tests. Full log remains
/tmp/manyhands-cycle03-final-tests.log; previous runs are preserved separately.

Ruling: collect numeric-only phase evidence while preserving the immediately
captured failed condition. Potential cost if wrong: revise test instrumentation.
This is not a behavioral fix or proof that the intermittent failure is resolved.

Run 37377199859 on fee629d passed macOS and Linux ARM64 before instrumentation;
Windows build and Linux x86-64 results are still pending at this checkpoint.
A passing rerun is non-reproduction, not a root-cause explanation. Native
acceptance and the intermittent macOS concern remain explicitly tracked.

Follow-up revision ee8153e corrects Windows credential-type test comparisons
with lossless i64 widening, preserving exact equality, and scopes the Unix-only
Read import correctly. Both Windows architectures in run 37377199859 reached
this same test-compilation failure after successful release builds; all three
non-Windows jobs passed. Independent review approved the correction and the
authorized cache-on-failure setting without findings. All four required local
gates passed on ee8153e: 582 tests, no failures or ignored tests.

Additional ruling: preserve compiled dependency caches after failed native jobs
using the action's supported option. Lockfile/toolchain keys and every test stay
in place. Potential cost if wrong: clear or revise the cache policy; cached
dependencies never constitute a passing test result.

The next native run must verify Windows compilation/runtime and the diagnostic
revision. The earlier macOS failure remains unexplained despite subsequent
passes; numeric instrumentation provides evidence if it recurs.

### Native Windows Evidence And macOS Phase Probe

Native run 37380377825 on f8c834c passed both Linux targets. Both Windows
architectures passed release build, credential/SSH tests, and artifact upload;
Windows ARM64 cache finalization was still running when evidence was collected.
The reviewed helper, native Perl, and credential ABI corrections now have native
Windows runtime evidence.

macOS failed earlier in reconnect_key_policy, observation 234 identifying the
strict KeyRejected assertion. This prevented the prior lost-response diagnostics
from running. There is still no confirmed common root cause.

Reviewed commits 0d4eed0 and be1bd82 add a bounded macOS probe of both exact cases
and test-only numeric phase evidence: fixed typed outcome categories, existing
operation hooks, server authentication/connection counts, helper counts, and
linked libgit2 version/features. Assertions and protocol behavior remain intact.
A controlled early reconnect failure validated the diagnostic branch and retained
the assertion failure; the mutation was removed. All four required local gates
passed on be1bd82: 582 tests, no failures or ignored tests. Independent review
found no issues.

Ruling: repeat both exact macOS cases in fresh invocations after the unchanged
full suite, at most 30 pairs, and stop on the first failure. Potential cost if
wrong: extra CI time or revision/removal of the investigative step. This cannot
turn failures into a pass by retrying and makes no behavioral fix claim.

Next publication carries the reviewed evidence-gathering changes. Native macOS
acceptance remains unresolved; ticket stays open, with no merge or cleanup.
