---
title: "Wave 01 Cycle 01 Canonical Foundation Design"
date: 2026-09-30
status: draft
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01K6YR0A1B2C3D4E5F6G7H8J9K"
---

# Wave 01 Cycle 01 Canonical Foundation Design

## Goal

Implement the version 1 canonical-content contract as a small, headless Rust
library. The library must parse, validate, and serialize configuration and
Markdown content without mutating repositories, losing Markdown bodies, or
discarding unknown metadata.

## Module Boundary

Add one public `manyhands::canonical` module in `src/canonical.rs`, exported
from `src/lib.rs`. Keep it as one module for Cycle 01 because the initially
required types and their validation rules are tightly coupled; split it only
when an implementation demonstrates a readability or testability problem.

The module owns no filesystem walks, repository enablement, Git mutation,
SQLite access, CLI behavior, or desktop dependencies. Callers provide paths
and source strings, receive typed values or structured problems, and choose
whether and when to write serialized output.

## Data Model

`ItemId` wraps a ULID and accepts only its exact 26-character uppercase
Crockford Base32 representation. `ItemKind` distinguishes documents, tickets,
and comments. Canonical path classification accepts repository-relative
`docs/*.md` paths at any depth, ticket paths, and comment paths; it rejects
absolute paths, traversal, wrong-kind paths, and the local worktree subtree.

`RepositoryConfig`, `Document`, `Ticket`, and `Comment` retain their typed
known fields alongside unknown TOML or YAML value mappings. YAML front matter
is extracted only when it begins at byte zero and ends at a delimiter line.
The source Markdown body is retained as the exact suffix after that delimiter.
Serialization may normalize TOML/YAML presentation but preserves unknown value
semantics and concatenates the unchanged body byte-for-byte.

## Validation Flow

Single-file parsing validates front-matter shape, required fields, field types,
timestamps, IDs, and the supplied canonical path. It reports errors through a
stable `ValidationProblem` taxonomy with the relevant path and an actionable
reason. It never infers a title, generates an ID, corrects metadata, or writes
content.

Context validation then examines all successfully parsed values together. It
detects globally duplicate IDs, verifies each comment refers to a document or
ticket, verifies parent comments use the same item, and detects parent cycles.
Only valid comments are assembled into a tree. Roots and each set of direct
replies sort by `created_at`, then `ItemId`, providing a deterministic result.

## Dependencies And Tests

Use direct, headless dependencies for Serde, TOML, a Serde-compatible YAML
parser, ULID handling, and RFC 3339 timestamp parsing. Restrict `git2` and
`tempfile` to development dependencies because Cycle 01 uses them only for
disposable test fixtures.

`tests/support/mod.rs` provides born and unborn `git2` repositories, local test
identities, and canonical source builders. `tests/canonical_foundation.rs`
drives the public API and proves valid round trips, invalid visible problems,
path/relationship rules, unknown metadata preservation, exact body retention,
ordering, and fixture isolation.

## Explicit Decisions

- Unknown TOML and YAML values are preserved semantically, not with original
  comments, whitespace, or key order.
- A title update is the known-field update used to prove Markdown and unknown
  YAML retention; later Cycles can add other explicit mutators without a raw
  metadata editing API.
- RFC 3339 timestamps must parse successfully and represent UTC; comment
  ordering compares parsed instants rather than source strings.
- `ValidationProblem` codes cover invalid paths, malformed or missing front
  matter, invalid known fields, kind/path mismatches, duplicate IDs, unresolved
  comment items or parents, cross-item parents, and parent cycles.

## Non-Goals

This design does not implement repository registration or enablement, commits,
worktrees, SQLite, refresh/discovery, credentials, remote operations, CLI
commands, desktop behavior, repair, or migration.
