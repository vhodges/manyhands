---
title: "Test and Compatibility Strategy RFC"
date: 2026-09-30
status: approved
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01K6YQ6E9H2K5N7P9R1T3V5W7X"
---

# Test and Compatibility Strategy RFC

## Summary

This RFC defines how Manyhands proves the approved PRD with real Git
repositories, fault injection, and platform coverage. It establishes the Wave
1 foundation gate and the evidence model that later Waves extend.

It implements the verification responsibility assigned by the approved
[MVP architecture RFC](mvp-rfc.md) and applies to every focused RFC that claims
MVP conformance.

Tests MUST exercise canonical Markdown and real Git state. Mock-only UI states
or fabricated Git outcomes do not establish PRD conformance.

## Scope

This RFC covers:

- Fixture repositories and test isolation.
- Unit, integration, lifecycle, and compatibility layers.
- Controlled failure injection and recovery verification.
- Wave 1 entry and exit evidence.
- The expansion path for remote, desktop, CLI, and platform testing.

It does not define canonical schema, Git behavior, database tables, or UI
interactions. Those RFCs define behavior; this RFC defines evidence.

## Test Principles

Every test that changes repository state MUST use a disposable real repository.
Wave 1 fixtures MUST be created through `git2`, not a shell Git dependency, so
they test the selected production backend and work on every supported platform.

Tests MUST use temporary directories and unique repository roots. No test may
read or alter a developer's repository, global Git configuration, SSH keys, or
application data. Tests that require a commit identity configure it locally in
the fixture repository.

Test output, snapshots, assertion failures, and logs MUST redact passphrases,
private keys, credentials, and private Markdown bodies not needed to explain a
failure.

## Test Layers

| Layer | Purpose | Wave 1 examples |
| --- | --- | --- |
| Unit | Pure parsing, validation, path, ID, and state-transition rules. | ULID validation, TOML validation, comment parent ordering. |
| Repository integration | Real temporary repository state through `git2`. | Enablement commit, worktree creation, scoped checkpoint. |
| Index integration | Real files plus disposable SQLite database. | Full primary/context refresh, problem rows, rebuild. |
| Failure injection | A controlled error after a named lifecycle step. | Write succeeds/commit fails; commit succeeds/index fails. |
| Compatibility | Native platform and filesystem behavior. | Path handling, case behavior, lock contention, builds. |
| Journey | Full product outcome across front ends and remotes. | Defined now; implemented in later Waves. |

The implementation MAY use test-only helpers and traits to inject failures, but
production behavior MUST remain driven by real filesystem, SQLite, and Git
operations in normal tests.

## Fixture Contract

Wave 1 provides reusable fixtures for:

- An unborn repository and a repository with an existing primary commit.
- Valid `.manyhands/config.toml` with and without a publication remote.
- Valid document, ticket, root-comment, and reply content using canonical ULIDs.
- Marker-only, malformed, duplicate-ID, invalid-path, and invalid-thread
  content.
- A primary worktree and one or more recognized item worktrees.
- Local Git identities, dirty files, untracked files, and conflict-like blocking
  state needed for recovery assertions.

Fixtures MUST make their commit OIDs, branches, worktree paths, and canonical
content available to assertions. They MUST clean up even when a test fails.

## Failure-Injection Contract

Each lifecycle operation exposes named test seams before and after external
steps. At minimum, Wave 1 injects failures at:

1. Repository configuration write.
2. Initialization commit creation.
3. Context branch creation.
4. Worktree creation.
5. Item Markdown write.
6. Checkpoint commit creation.
7. SQLite transaction or index refresh.

Tests MUST assert the actual recoverable state after each failure: preserved
file, commit OID, branch, worktree, index-pending operation, or visible problem.
They MUST also assert that retry performs only unfinished work and creates no
duplicate commits, branches, worktrees, items, or comments.

## Wave 1 Entry Gate

Before Wave 1 implementation begins, the following RFCs MUST be approved:

- Canonical content and comment schema.
- Git workflow and conflict recovery.
- Repository index persistence and refresh.
- This test and compatibility strategy.

The repository must continue to build through its existing CI target matrix.
Wave 1 code changes must use the project-mandated Devenv Cargo commands from
`AGENTS.md`.

## Wave 1 Exit Gate

Wave 1 completes only with automated evidence for all of the following:

- Enable an existing clean repository and an unborn repository, each producing
  exactly one tracked configuration initialization commit and local exclude
  rule without publication.
- Reject an inaccessible, non-Git, dirty-primary, or conflicted-primary
  enablement without overwriting or committing user work.
- Create, validate, move, and rediscover documents; create and rediscover
  tickets; create and order root comments and replies.
- Show marker-only and malformed content as nonconforming without hiding or
  rewriting it.
- Create, reuse, and distinguish item worktrees using deterministic branches
  and paths.
- Create scoped checkpoints, preserve unrelated worktree changes, and avoid
  empty commits.
- Refresh primary and active contexts, apply active-context precedence, and
  surface duplicate or mismatched deterministic local contexts as visible,
  preserved recovery problems without an editable choice state.
- Delete or corrupt the local SQLite database and rebuild equivalent discovery
  results without canonical filesystem or Git mutation.
- Inject every required Wave 1 failure and successfully retry without duplicate
  canonical content, commits, branches, worktrees, or cleanup.

The exit gate requires the full local Rust verification suite specified in
`AGENTS.md`. A CLI smoke test is required when the Wave 1 command surface
exists. Desktop smoke testing is required only for desktop code introduced in a
Wave 1 Cycle and requires an active display.

## Compatibility Roadmap

Wave 1 verifies domain behavior on the local development platform and preserves
the existing Linux, macOS, and Windows CI build matrix. The test strategy grows
without weakening prior evidence:

- Wave 2 adds local bare remotes or equivalent real remote fixtures, SSH
  callback failures, polling serialization, merge conflicts, promotion, and
  ticket closure recovery.
- Wave 3 adds desktop keyboard journeys, CLI JSON and daemon behavior, native
  platform journey runs, and trusted-collaborator dogfooding evidence.

Platform-specific deviations in filesystem case behavior, path normalization,
Git installations, locking, or credential facilities MUST become documented
compatibility cases rather than implicit assumptions.
