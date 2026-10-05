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
recovery even when known_hosts matches. The production timeout engineering gate
remains an implementation prerequisite before the connection driver. The user accepted 10 seconds
to connect and 30 seconds of stalled I/O; actively progressing transfers may
run longer. Only initialization and backend coverage remain to settle.

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
