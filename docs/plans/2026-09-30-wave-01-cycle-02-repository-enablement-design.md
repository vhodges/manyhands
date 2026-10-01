---
title: "Wave 01 Cycle 02 Repository Enablement Design"
date: 2026-09-30
status: approved
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01K6Z1A1B2C3D4E5F6G7H8J9K0"
---

# Wave 01 Cycle 02 Repository Enablement Design

## Goal

Implement the local, headless repository lifecycle needed to make a Git
repository a safely enabled Manyhands repository. The implementation must
create or inspect a local repository, persist only its application-local
registration, maintain tracked configuration and local remotes, and preserve
all unrelated user work. It performs no remote contact.

## Module Boundary

Add one public `manyhands::repository` module in `src/repository.rs`, exported
from `src/lib.rs`. It owns repository inspection, creation, enablement,
registration removal, local remote management, and publication-remote
configuration. It consumes `manyhands::canonical::RepositoryConfig` for all
tracked configuration parsing and serialization; it does not duplicate schema
validation.

`RepositoryService` stores only an application-local database path and a narrow
operation hook. Each operation opens and drops its own `git2::Repository` and
SQLite connection. It therefore has no GPUI, GPUI Kit, desktop, CLI, network,
or retained libgit2-handle dependency. Private helpers own filesystem writes,
Git tree construction, configuration observation, SQLite migration, and remote
URL classification.

## Public Operations

The public API exposes typed requests and results for:

- Read-only repository inspection, including canonical root, checked-out branch,
  local branches, configured remotes, configuration state, and identity status.
- Creation and enablement of an empty or nonexistent local target with a
  confirmed primary branch and optional confirmed local Git identity.
- Enablement or registration of an existing local repository with a confirmed
  primary branch.
- Application-local registration removal.
- Local remote listing, addition, removal, and publication-remote selection or
  clearing.

Errors identify a root when it can be resolved, the requested operation, and a
stable recovery category. Missing identity is a typed outcome rather than an
invented fallback identity. Callers may retry with a separately confirmed name
and email, which are written only to local repository configuration.

## Preconditions And Enablement Flow

For an existing born repository, the selected primary branch must be the branch
currently checked out at the selected repository root. A different local branch
produces a recoverable result instructing the caller to check it out outside
Manyhands and retry. Cycle 02 never checks out, switches, creates, or removes
branches in an existing repository.

For an unborn repository, the confirmed branch becomes the symbolic `HEAD`
target before the initialization commit. New-repository creation validates the
target and identity before creating a directory or repository. The parent must
already exist and be writable; a target must be nonexistent or empty. Cleanup
after a failed creation is limited to empty resources created by that operation.

Enablement first requires a usable non-bare repository, matching checked-out
primary branch, clean and non-conflicted worktree, and valid configuration state
when configuration already exists. A valid existing configuration must name the
confirmed branch and, when it selects a publication remote, that remote must
exist and pass SSH eligibility. It is never overwritten by enablement.

When configuration is absent, enablement:

1. Resolves a complete effective Git identity or returns `identity required`
   before repository mutation.
2. Captures the exact existing `.git/info/exclude` bytes and configuration
   absence, then adds `.manyhands/worktrees/` exactly once.
3. Atomically writes valid canonical `.manyhands/config.toml`.
4. Builds and commits a tree containing only that configuration change with
   subject `Initialize Manyhands`.
5. Records the canonical root and committed configuration blob OID in the
   application-local registry with `refresh_required` set.

If a pre-commit step fails, the operation restores its configuration and exclude
changes. If registration fails after the commit, the commit and configuration
remain authoritative and the result reports registration pending; a retry only
performs registration.

## Registry

Cycle 02 creates and migrates only the `repositories` table in one
`manyhands.sqlite3` database resolved through `directories::ProjectDirs` for
Manyhands. The connection enables foreign keys, WAL mode, and a bounded busy
timeout. Tests provide a temporary database path instead of using application
data.

Each row has a stable local integer ID, canonical root path with a unique
constraint, enabled timestamp, accessibility state, current committed
configuration blob OID, and `refresh_required` flag. The row contains neither
Markdown nor a copied configuration body. Cycle 04 extends this database with
context, item, problem, and operation-recovery tables through additional
migrations.

Removal deletes only the row. No registration operation changes repository
files, Git configuration, branches, worktrees, remotes, or commits.

## Remote Management

Remote inspection, addition, and removal use only local libgit2 configuration
APIs. They do not fetch, push, connect, authenticate, or install transport
callbacks. Adding an existing remote with the same endpoint is a no-op for
retry; a conflicting existing remote is a typed error. Removing an absent
unselected remote is likewise a no-op.

A publication remote must have both an SSH-compatible fetch URL and an
SSH-compatible effective push URL. The effective push URL is `pushurl` when
configured, otherwise the fetch URL. SSH-compatible URLs use either an
`ssh://` scheme or Git's scp-like host-and-path form; HTTP(S), `file`, local,
and malformed URLs are ineligible. A selected remote cannot be removed until
the publication selection is changed or cleared.

Selecting, replacing, or clearing a publication remote preserves unknown TOML
values and makes one tracked `Configure Manyhands publication remote` commit.
It may proceed with unrelated worktree, index, untracked, or conflict state,
but requires the configuration path itself to match `HEAD`. The commit uses an
isolated temporary Git index initialized from the `HEAD` tree and changed only
at `.manyhands/config.toml`; it never reads, writes, stages, or commits the
live index's unrelated entries.

## Testability And Recovery

A narrow operation hook exposes named checkpoints for test-only injected
failure after creating a new target directory but before repository
initialization, before configuration write, before initialization commit, before
publication-configuration commit, and before registration write. Production
uses a no-op implementation. This keeps actual filesystem, SQLite, and git2
behavior in every normal test while making every required interruption boundary
deterministic.

Fixtures create born/unborn repositories, local-only identities, dirty and
conflicted indexes, distinct fetch/push remote URLs, and temporary registry
databases. They never inspect developer configuration, repositories,
application data, or credentials. Integration tests inspect actual files,
refs, commit trees, OIDs, remotes, index entries, and SQLite rows to prove
atomicity and idempotent retries.

## Dependencies

Move `git2` and `tempfile` to production dependencies. Add `directories` for
application-data resolution and `rusqlite` with the bundled SQLite feature.
Keep all dependencies headless and available to both binaries without enabling
the optional desktop feature.

## Non-Goals

This design does not implement worktree contexts, item authoring or checkpoint
flows, discovery scans, SQLite tables beyond repository registration,
cross-process operation leases, remote transport, SSH credentials, polling,
synchronization, merge/close/promotion workflows, desktop UI, CLI commands, or
automatic configuration repair.
