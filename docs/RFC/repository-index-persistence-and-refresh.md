---
title: "Repository Index Persistence and Refresh RFC"
date: 2026-09-30
status: approved
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01K6YQ5D8G1J4M6N8P0R2T4V6W"
---

# Repository Index Persistence and Refresh RFC

## Summary

This RFC defines the application-local repository registry, rebuildable SQLite
discovery index, refresh model, and operation-recovery storage. It implements
the PRD rule that Markdown and Git are canonical while SQLite is a cache that
can be deleted and rebuilt without rewriting repository state.

It implements the persistence and refresh decisions assigned by the approved
[MVP architecture RFC](mvp-rfc.md) and consumes the canonical content contract
from the [schema RFC](canonical-content-and-comment-schema.md).

Wave 1 implements local registration and full scans of the primary worktree and
active Manyhands contexts. Remote polling scheduling and materialization are
defined as compatible extensions for Wave 2.

## Local Database

Manyhands stores one SQLite database named `manyhands.sqlite3` in the
operating-system-appropriate application-data directory resolved through
`directories::ProjectDirs` for application `Manyhands`. The implementation
MUST enable foreign-key enforcement, use WAL journal mode, and set a bounded
busy timeout so desktop, CLI, and a CLI daemon can coordinate safely.

The database contains application-local state only. It MUST NOT store private
key data, passphrases, managed Markdown bodies, or an authoritative copy of
canonical repository configuration.

## Required Logical Tables

The exact SQL is implementation-owned, but the database MUST represent these
logical records:

| Record | Required information |
| --- | --- |
| Repository registration | Stable local ID, canonicalized root path, enabled time, accessibility state, and last observed config identity. |
| Context | Repository ID, primary or item context kind, branch, worktree path, item ULID when applicable, active state, and observed Git HEAD. |
| Item discovery | Context ID, item ULID, kind, canonical path, title, ticket type/status when applicable, closure state, project/team when present, and observed modification time. |
| Problem | Repository/context/path, stable problem code, human-readable recovery guidance, and observation time. |
| Operation recovery | Repository, operation ID, item/context when applicable, lifecycle action, completed steps, relevant commit OIDs, failure detail, and recovery state. |

The canonicalized root path is unique among enabled registrations. Removing a
repository from Manyhands removes only its local registration, contexts, item
rows, problems, and operation records. It MUST NOT change repository files,
Git configuration, branches, worktrees, remotes, or commits.

## Registration and Configuration Observation

Adding or enabling a repository canonicalizes its root path and checks that it
is accessible, non-bare, and a Git repository. The Git RFC performs any tracked
configuration initialization commit; this RFC records the result only after Git
state confirms it.

Refresh reads `.manyhands/config.toml` as canonical. Its primary branch and
publication remote are observed metadata, not an editable duplicate in SQLite.
A missing, malformed, inaccessible, or unsupported configuration becomes a
problem record with recovery guidance.

## Context Discovery

Each refresh discovers:

- The configured primary worktree at the repository root.
- Active repo-local worktrees whose branch matches
  `manyhands/<kind>/<ULID>` and whose identified item validates under the
  canonical schema.

An active worktree with a matching branch but invalid content remains visible as
a problem. A worktree outside the configured `.manyhands/worktrees/` base or a
branch outside the context convention is not an active Manyhands item context.
The Git RFC owns creation and recovery of worktrees; this RFC records what Git
and the filesystem actually expose.

## Full Refresh Algorithm

Wave 1 uses a full per-context scan rather than Git-diff incremental indexing.
For each accessible primary or active context, a refresh MUST:

1. Enumerate canonical paths defined by the schema RFC.
2. Parse and validate eligible Markdown without rewriting it.
3. Resolve comment parent and item relationships within that context.
4. Derive discovery metadata and the latest observed content activity.
5. Replace only that context's item and problem rows in one SQLite transaction.

The primary context and item contexts remain separate rows even when they
contain the same item ULID. Query consumers apply the PRD's active-context
precedence rule: one active editable context replaces the primary presentation;
multiple active contexts require an explicit choice.

For committed files, observed activity uses the latest Git commit that touches
the item Markdown or a managed comment for that item. For uncommitted local
changes, the index records the newer filesystem observation time and marks its
source accordingly. This gives discovery a current timestamp without treating a
filesystem value as canonical content.

## Rebuild and Corruption Recovery

A rebuild creates a new empty local index and scans accessible canonical state.
It MUST NOT write Markdown, create commits, alter branches/worktrees/remotes,
or change Git configuration. If the existing database is corrupt or cannot be
opened, Manyhands preserves it for diagnostics when possible, creates a fresh
database, and reports that discovery is rebuilding.

If a context or path cannot be read, the rebuild retains successful rows from
other contexts and records the inaccessible location as a problem. It does not
replace inaccessible canonical content with cache data.

## Operation Coordination and Recovery

SQLite coordinates Manyhands operations across desktop, CLI, and daemon
processes. A repository-scoped operation lease MUST be acquired before a Git
lifecycle action or a polling refresh changes Git state. The lease has a bounded
wait and recoverable busy outcome.

Operation records advance only after their external Git or filesystem step is
observed. On startup or retry, reconciliation compares the record with actual
Git refs, worktrees, and commits. Canonical Git state wins. A completed commit
with a pending index refresh is retried as indexing only; it never creates a
duplicate commit.

## Polling Extension

Wave 1 does not implement remote fetch scheduling. Its schema and locking model
reserve the same repository lease for a future poll. The Wave 2 extension will
record poll status, backoff, and remote observation without changing this RFC's
canonical/cache boundary.

When implemented, a poll will use the Git RFC's recognized context branches,
fast-forward only clean contexts, and materialize a new conforming context once.
Remote-deleted, divergent, malformed, or inaccessible contexts remain locally
preserved and are recorded as recovery problems.

## Wave 1 Acceptance

Wave 1 persistence work is complete when real temporary repositories show that:

- An enabled repository is registered once by canonical root path and can be
  removed without repository mutation.
- Full refresh indexes valid primary and active-context items, comments, and
  problems while preserving distinct context rows.
- A single active context receives discovery precedence and multiple active
  contexts require a queryable choice state.
- Marker-only, malformed, duplicate-ID, and inaccessible content stays visible
  as a problem rather than disappearing or being rewritten.
- Deleting or corrupting the SQLite database followed by rebuild produces the
  same discovery result from canonical state and changes no Git or Markdown
  content.
- A checkpointed commit followed by an injected index failure is reconciled as
  index pending without a duplicate commit.
