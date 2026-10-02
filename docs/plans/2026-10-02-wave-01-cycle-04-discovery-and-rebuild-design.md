---
title: "Wave 01 Cycle 04 Discovery and Rebuild Design"
date: 2026-10-02
status: approved
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01K7C5E7G9J1M3P5R7T9V1X3Z5"
---

# Wave 01 Cycle 04 Discovery and Rebuild Design

## Goal

Provide a rebuildable, metadata-only local discovery snapshot for one enabled
repository without changing canonical Markdown or Git state.

The governing scope is
`docs/Cycles/wave-01-cycle-04-discovery-and-rebuild.md`. This design records
the implementation decisions approved for that Cycle.

## Architecture

`RepositoryService` remains the public synchronous, headless service. It gains
small refresh, explicit-root rebuild, and stored-snapshot operations. A private
`repository::discovery` module owns SQLite migration, filesystem/Git observation,
canonical parsing, transactional persistence, corruption recovery, and
row-to-snapshot mapping. It receives only the registry path and per-call root;
it does not retain Git handles, database connections, directory iterators, or
canonical content bodies.

The public model is a deterministic repository snapshot. It contains repository
and configuration observation, distinct context observations, document and
ticket metadata, comment-tree metadata, activity source/time, problems, and the
repository refresh state. Canonical Markdown bodies are deliberately absent;
future readers open their canonical path when content is needed.

## Context Observation

Refresh reads a registered root in all configuration states. It calls the root
`primary` only when valid tracked configuration names the checked-out branch. A
missing, malformed, unsupported, or wrong-branch root is scanned as
`unverified` and receives a visible problem.

Only valid configuration enables local context discovery. The scanner enumerates
the configured repo-local worktree base and recognizes an active context only
when its exact deterministic path, checked-out branch, branch kind/ULID, and
branch-identified conforming document or ticket all agree. Each recognized
context receives a full scan, including its valid content, comment metadata,
uncommitted activity, and nonconforming paths.

A convention-matching candidate that lacks its identified valid item is a visible
context problem. It is not active, editable, or eligible to replace primary
presentation. Cycle 04 has no context-choice state.

## Source Scanning

The scanner reads canonical document, ticket, and comment locations with the
existing canonical module. It does not descend into Git metadata or the root's
`.manyhands/worktrees/` directory. It separately walks Markdown outside the
canonical locations and accepts it as a nonconforming candidate only when its
leading YAML front matter declares `manyhands_managed: true`. This keeps ordinary
repository Markdown out of discovery problems while preserving visibility of
managed content at invalid paths.

Canonical validation remains the source of schema problem codes. Context-local
validation removes duplicate IDs and invalid comments from valid item metadata,
but preserves their path-specific problems. Comment trees use the canonical
ascending `created_at`, then ULID order.

## Persistence

The existing `repositories` table remains the root-registration authority for
the local cache. A forward-only table-rebuild migration makes
`config_blob_oid` nullable, then adds context, discovered item/comment metadata,
problem, and operation tables with foreign keys and repository-delete cascades.
Valid configuration records its observed blob OID; missing or invalid
configuration records `NULL` and a problem. The migration retains the existing
unique canonical root and `refresh_required` flag. Schema details are private.

Each stable observed context replaces only its cached item/comment/problem rows
in one transaction. A complete repository observation then removes disappeared
context rows and clears `refresh_required`. Snapshot reads use stable ordering by
context, canonical path, comment creation time, and ULID.

Activity metadata is the newest Git commit touching the document/ticket path or
one of its valid comments. A newer relevant uncommitted file observation wins
and records `UncommittedFilesystem` as its source. Timestamps are observation
metadata, never canonical content.

## Refresh And Rebuild Recovery

Cycle 04 stores operation records for refresh and rebuild only. Each record
advances after the corresponding read or SQLite step is observed. A Cycle 03
`refresh_required` flag starts or resumes only indexing; it cannot recreate an
item, comment, worktree, branch, or checkpoint commit.

If a context HEAD or relevant file observation changes during a scan, the scanner
does not write mixed data. It retains the prior context rows, stores a retry
problem, leaves `refresh_required` true, and returns a typed retry-required
outcome. In-process callers serialize authoring and refresh until Cycle 05 adds
the repository-scoped cross-process lease.

`rebuild_repository(root)` accepts an explicit root because a deleted cache
cannot discover former roots. It reconstructs registration and derived state for
that caller-supplied root without searching the filesystem. A normal healthy
cache rebuild affects only the requested root's derived data.

When `open_at` detects structural SQLite corruption, it returns a degraded
service retaining the registry path. Until explicit rebuild succeeds,
`rebuild_repository` is the only available registry/discovery action; normal
registry and discovery APIs return a typed index-unavailable result. Rebuild
renames the corrupt database and its WAL/SHM sidecars to timestamped diagnostic
paths when possible, creates a fresh database, and scans the explicit root.

## Verification

Focused disposable-repository integration tests prove root and active-context
discovery, precedence, comment ordering, activity, malformed and out-of-path
content visibility, stale-row cleanup, no canonical mutation, Cycle 03
index-pending reconciliation, database corruption recovery, injected SQLite
failures, and scan-change retry behavior. The implementation runs through
`git2`, `rusqlite`, real temporary filesystems, and isolated data directories;
it uses no system Git executable, remote transport, global Git configuration,
developer repository, or application data.
