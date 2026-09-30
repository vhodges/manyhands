---
title: "Wave 01 Cycle 01: Canonical Foundation"
date: 2026-09-30
status: approved
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01K6YQ8G1K4N7R9T1V3W5X7Y9Z"
---

# Wave 01 Cycle 01: Canonical Foundation

## Parent Wave

This is Cycle 01 of [Wave 01: Foundations](../Waves/wave-01-foundations.md).
It establishes the schema library and disposable real-repository fixture base
required by every later Wave 1 Cycle.

## Purpose

Implement the version 1 canonical-content contract as shared, headless Rust
domain code. The Cycle makes configuration, identities, paths, front matter,
validation, and threaded-comment ordering testable before any repository
enablement, Git lifecycle, SQLite, CLI, or desktop behavior is added.

## Prerequisites

- Wave 01 entry gate remains satisfied.
- The approved canonical schema, Git workflow, repository/index, and test
  strategy RFCs remain unchanged or receive an approved amendment.
- Implementation starts from a clean `main` branch.

## RFC and PRD Traceability

| Source | Cycle responsibility |
| --- | --- |
| [Canonical content and comment schema RFC](../RFC/canonical-content-and-comment-schema.md) | Implement version 1 configuration, paths, ULIDs, typed front matter, validation, and semantic unknown-key preservation. |
| [Test and compatibility strategy RFC](../RFC/test-and-compatibility-strategy.md) | Provide disposable `git2` fixtures and unit/integration evidence for the canonical contract. |
| `MH-REPO-001` | Parse and validate the tracked enabled-repository configuration; do not enable a repository yet. |
| `MH-CONTENT-001` to `MH-CONTENT-003` | Represent and validate managed documents and tickets. |
| `MH-COMMENT-001` to `MH-COMMENT-002` | Represent comments, parent relationships, and deterministic thread order. |
| `MH-INDEX-003` | Produce visible validation problems without hiding or rewriting malformed content. |
| `MH-NFR-007` and `MH-NFR-008` | Keep parsing and validation non-destructive; preserve user content during read-modify-write. |

## In Scope

- Add the local schema and fixture dependency set: `git2`, `serde`, a
  serde-compatible YAML parser, `toml`, ULID support, and `tempfile` for tests.
- Add shared domain types for repository configuration, item IDs, item kinds,
  canonical relative paths, front matter, comment relationships, and validation
  problems.
- Parse and serialize version 1 `.manyhands/config.toml` while retaining unknown
  configuration keys.
- Parse and serialize YAML front matter for documents, tickets, and comments.
- Preserve unknown front-matter keys and values semantically and preserve the
  Markdown body exactly during read-modify-write. YAML whitespace, ordering, and
  comments are not required to remain byte-for-byte identical.
- Validate required fields, canonical paths, uppercase ULIDs, ticket closure
  metadata, comment item/parent relationships, duplicate IDs, and comment-tree
  cycles.
- Implement deterministic root-comment and direct-reply ordering by
  `created_at`, then ULID.
- Provide test-only disposable born/unborn Git repositories and canonical
  content fixtures through `git2` and temporary directories.

## Out of Scope

- Repository registration, enablement, initialization commits, commit identity,
  and `.git/info/exclude` management.
- Item branch or worktree creation, checkpoint commits, scoped staging, merge,
  promotion, closure, cleanup, or any remote operation.
- SQLite persistence, refresh, discovery queries, operation leases, filesystem
  watching, or polling.
- SSH keys, credentials, passphrases, CLI commands, desktop UI, or editor
  behavior.
- Automatic repair or migration of marker-only documents.

## Planned Implementation Changes

| Path | Change |
| --- | --- |
| `Cargo.toml` | Add schema, ULID, `git2`, and test-fixture dependencies without SQLite, SSH, polling, or UI dependencies. |
| `Cargo.lock` | Record resolved dependency versions. |
| `src/lib.rs` | Export the shared canonical domain module without introducing GPUI or GPUI Kit dependencies. |
| `src/canonical.rs` | Add configuration, ID, path, front-matter, validation, serialization, and thread-order domain logic. |
| `tests/support/mod.rs` | Add disposable `git2` repository and canonical Markdown fixture helpers. |
| `tests/canonical_foundation.rs` | Add focused unit and integration coverage for the Cycle exit gate. |

The implementation MAY split `src/canonical.rs` into focused internal modules
only when that reduces a demonstrated readability or testability problem. It
MUST retain a small, headless public API through `src/lib.rs` for later Cycles
and both front ends.

## Domain Contract

The Cycle creates no repository state through Manyhands operations. It provides
the pure domain contracts later Cycles consume:

- `RepositoryConfig` represents `format_version`, `primary_branch`, optional
  `publication_remote`, and semantically preserved unknown TOML keys.
- `ItemId` accepts only canonical 26-character uppercase Crockford Base32 ULIDs
  and generates new IDs for later explicit create operations.
- `ItemKind` distinguishes `document`, `ticket`, and `comment`.
- Canonical path helpers recognize only `docs/**/*.md`,
  `.manyhands/tickets/<id>/ticket.md`, and
  `.manyhands/comments/<item-id>/<comment-id>.md`; they exclude the primary
  worktree's `.manyhands/worktrees/` subtree.
- Typed document, ticket, and comment values retain the Markdown body and all
  unknown front-matter mappings. Known fields are validated according to the
  schema RFC.
- Validation returns structured, displayable problems. It never writes files,
  generates IDs, infers titles, changes paths, or silently upgrades marker-only
  content.

## Test and Fixture Plan

`tests/canonical_foundation.rs` MUST cover:

- Valid and invalid TOML configuration, including optional publication remote,
  invalid format versions, missing primary branches, and unknown-key retention.
- Uppercase ULID generation and validation, including rejection of malformed,
  lower-case, and wrong-length IDs.
- Valid document, ticket, closed-ticket, root-comment, and reply parsing.
- Marker-only content, malformed YAML, missing required fields, invalid paths,
  duplicate item IDs, missing parents, cross-item parents, and comment cycles.
- Semantic retention of unknown front-matter keys and values plus exact Markdown
  body retention after a known-field update and reparse.
- Root-comment and direct-reply order by timestamp with ULID tie-breaking.
- Disposable born and unborn repositories, created through `git2`, whose
  temporary roots and local identities never read developer repositories or
  global Git configuration.

Fixtures may create repository files and commits only to establish test state.
They MUST not exercise Manyhands enablement, worktrees, checkpoints, or remote
operations; those belong to later Cycles.

## Recovery Considerations

Cycle 01 does not execute a lifecycle mutation, so its recovery behavior is
non-destructive validation. A malformed configuration or Markdown file returns a
structured problem with its path and reason; it remains unchanged and visible to
the caller. Parsing failures, invalid IDs, and impossible comment trees must not
panic, create files, alter bodies, or mutate fixture-independent application
state.

## Verification

During implementation, run the focused test target after each completed
behavior slice:

```sh
devenv shell -- cargo test --locked --test canonical_foundation
```

Before declaring the Cycle complete, run:

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
```

No desktop or CLI smoke test is required because this Cycle introduces neither
desktop nor CLI behavior.

## Exit Criteria

Cycle 01 is complete when:

- The shared canonical module compiles without GPUI or GPUI Kit dependencies.
- Version 1 configuration, paths, IDs, documents, tickets, and comments parse,
  validate, and serialize according to the schema RFC.
- Unknown front-matter keys/values are retained semantically and Markdown bodies
  are retained exactly through a read-modify-write operation.
- Nonconforming content returns visible structured problems without mutation or
  silent upgrade.
- Thread order is deterministic after reparsing fixture content.
- Disposable real Git repository fixtures are reusable by later Cycles.
- The focused test target and full required Rust verification suite pass.

## Handoff

Cycle 02 may depend on this Cycle only after its exit criteria are met. It uses
the configuration/path/validation contracts and fixture helpers to implement
repository enablement and initialization. It MUST NOT redefine the canonical
schema or weaken validation behavior established here.
