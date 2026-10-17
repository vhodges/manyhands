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
busy timeout so desktop and one-shot CLI operations can coordinate safely.

The database contains application-local state only. It MUST NOT store private
key data, passphrases, credentials, credential callback values, unredacted
remote/server response text, managed Markdown bodies, or an authoritative copy
of canonical repository configuration. Failure records store only stable
recovery codes, redacted categories, and non-secret target identifiers.

## Required Logical Tables

The exact SQL is implementation-owned. Cycle 02 implements repository
registration only; Cycle 04 adds the remaining logical records:

| Record | Required information |
| --- | --- |
| Repository registration | Stable local ID, canonicalized root path, enabled time, accessibility state, and last observed config identity. |
| SSH key registration | Local key ID, label, generated/imported ownership, non-secret source paths, optional public fingerprint, selected state, and observed accessibility. |
| Host trust | Normalized SSH host and effective port, host-key algorithm, non-secret fingerprint, and approval/replacement state. |
| Context | Repository ID, primary, local item, or remote item context kind; branch; local worktree path when materialized; item ULID when applicable; local and remote observation state; and observed local/remote OIDs. |
| Item discovery | Context ID, item ULID, kind, canonical path, title, ticket type/status when applicable, closure state, project/team when present, and observed modification time. |
| Problem | Repository/context/path, stable problem code, human-readable recovery guidance, and observation time. |
| Polling policy and status | Repository, enabled/paused state, configured interval, bounded backoff, latest result, and next eligible automatic poll time. |
| Operation recovery | Repository, operation ID, item/context when applicable, lifecycle action, manual-or-poll priority, reservation/cancellation state, publication remote and refs, completed steps, relevant local/remote OIDs, redacted failure category, and recovery state. |

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

Each index-only refresh discovers:

- The configured primary worktree at the repository root.
- Active repo-local worktrees whose branch matches
  `manyhands/<kind>/<ULID>` and whose identified item validates under the
  canonical schema.

An active worktree with a matching branch but invalid content remains visible as
a problem. A worktree outside the configured `.manyhands/worktrees/` base or a
branch outside the context convention is not an active Manyhands item context.
The Git RFC owns creation and recovery of worktrees; this RFC records what Git
and the filesystem actually expose.

Remote observation records are distinct from local contexts. After a successful
poll, the index records recognized remote context branches whether or not they
have a local worktree. A conforming unmaterialized branch is visible as pending
materialization; malformed, inaccessible, renamed, unrecognized, divergent, or
remotely deleted branches are visible as recovery problems. Only a recognized
conforming shared branch may be materialized once. Remote observation does not
create a second editable context or a context-choice result.

## Index-Only Refresh Algorithm

Wave 1 uses a full per-context scan rather than Git-diff incremental indexing.
For each accessible primary or active context, an index-only refresh MUST:

1. Enumerate canonical paths defined by the schema RFC.
2. Parse and validate eligible Markdown without rewriting it.
3. Resolve comment parent and item relationships within that context.
4. Derive discovery metadata and the latest observed content activity.
5. Replace only that context's item and problem rows in one SQLite transaction.

The primary context and item contexts remain separate rows even when they
contain the same item ULID. A local clone records at most one deterministic
shared local item context for an item, which replaces the primary presentation
when active. Remote observations add presentation and recovery metadata; they
never create multiple editable contexts or require a consumer choice.

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

Cycle 03 callers serialize local authoring operations for a repository and use
the actual Git and filesystem state for immediate retries. It writes no durable
operation record. Cycle 04 adds refresh and rebuild operation records that
advance only after their external Git or filesystem step is observed. Cycle 05
extends durable operation records to every Wave 1 lifecycle action: enablement,
publication-remote configuration, context provisioning, canonical writes,
checkpoints, refresh, rebuild, and registration removal. On startup or retry,
reconciliation compares a record with actual Git refs, worktrees, commits, and
canonical files; Git and Markdown state win. A completed commit with a pending
index refresh is retried as indexing only and never creates a duplicate commit.

Cycle 05 makes cross-process repository coordination mandatory across desktop
and CLI processes. Each repository-mutating or
index-only-refreshing Wave 1 action MUST acquire a repository-scoped exclusive
advisory lease at the resolved common Git directory. The lease has a fixed
bounded wait and recoverable busy outcome. Draft preparation and full read-only
scans remain outside the lease; actions re-observe state while holding it before
durably changing canonical, Git, or cache state. Stored snapshot reads do not
require the repository lease.

SQLite records operation and discovery state but is not the repository lease.
Normal SQLite transactions use a short shared application-data cache-recovery
guard. Structural-corruption recovery takes that guard exclusively only while
preserving and replacing the database, then releases it before an explicit-root
rebuild acquires its repository lease and scans canonical state. This prevents
database replacement races without globally serializing Git work or scans.

## Remote Polling And Reservation

Wave 2 adds repository-scoped remote-operation reservations for polling, item
synchronization, primary synchronization, promotion, and closure. A reservation
is durable recovery state, not a lock: it identifies the operation, priority,
phase, yield request, cancellation state, remote/ref targets, and observed
object IDs. The common-Git-directory advisory lease remains the sole short
cross-process lock for local atomic transitions.

Network transport, caller interaction, and full scans MUST NOT hold the
repository lease. A remote action records its reservation, performs network
work outside that lease, then reacquires the lease and re-observes refs,
worktrees, and canonical paths before each local mutation or cache replacement.
A manual action that finds a poll reservation requests yield and returns a
retryable `poll yielding` result. The poll acknowledges at a Git transport or
per-context safe point, records its interrupted state, and releases the
reservation before the retried manual action begins.

Each repository with an SSH publication remote stores polling enabled state,
pause state, configured interval, backoff, latest result, and next eligible
automatic poll time. Polling is enabled by default, uses a five-minute interval,
accepts an interval from one to sixty minutes, and backs off failed automatic
polls from one minute exponentially to a fifteen-minute maximum. Manual
one-shot polls are never delayed by automatic backoff. Wave 2 persists and
executes one-shot poll behavior; Wave 3 owns scheduling inside the desktop
process. Per PRD 0.5, the CLI invokes polling/indexing explicitly and has no
resident mode. No cross-process scheduler election or due-slot protocol is
required; existing repository-operation leases and reservations remain.

A poll first fetches and records the Git RFC's remote ref set. It may then
fast-forward only clean, non-conflicted local primary or shared-context branches
that are strictly behind their matching tracking ref. It validates and
materializes each new recognized conforming shared context exactly once, then
invokes the index-only refresh. Remote-deleted, divergent, malformed,
inaccessible, renamed, unrecognized, and unmaterialized contexts remain locally
preserved and appear as recovery problems. A poll never pushes, checkpoints,
merges, rebases, stashes, overwrites, discards, or cleans up state.

## Approved Wave 03 Persistence Extensions

The product owner approved these extensions on 2026-10-05 through the
[desktop](desktop-information-architecture-and-editor.md),
[CLI](cli-contract.md) and [runtime](application-runtime-and-polling.md) RFCs.
They are Wave 03 obligations, not claims about the existing schema or APIs.

Explicit user pause remains durable repository policy. Desktop unlock
suspension is session state and MUST NOT overwrite that policy. The desktop
owns its background worker; no process-heartbeat registry, poller election or
cross-process due-slot schema is required. Existing leases and reservations
continue to protect repository operations.

Recoverable editor drafts live in versioned, owner-protected application-local
files outside SQLite and operation journals. They retain the source/base text
needed for recovery, never credentials, and follow the desktop RFC's atomic
write, flush, restore and explicit-discard rules. They are not canonical item
state. Index rebuild and registration removal MUST NOT delete them; successful
save retires only the draft revision proved checkpointed.

CLI mutations persist non-secret request identity, semantic-input digest where
appropriate, target observations, operation linkage and completed effects.
Confirmation records bind the exact previewed action and observed effects and
expire as specified by the CLI RFC. Neither record stores Markdown bodies,
private-key material, passphrases or passphrase-derived digests. Request replay
must reconcile Git/canonical state; cache loss that prevents safe replay returns
recovery-required rather than silently repeating an effect. These records do
not replace the repository lease or make SQLite authoritative.

Item discovery also records each ticket's short code, and the index holds
relationship edge records and the relationship problems found in each
ticket's own file, as defined by the
[ticket relationships and short codes RFC](ticket-relationships-and-short-codes.md#index).
They are rebuilt from canonical files on refresh and rebuild. Readiness, and
whether an edge's target resolves, are not stored: they are computed from the
stored edges when read (amended 2026-10-08).

Reads list what the index holds (product owner, 2026-10-08). A read never
lists a directory or scans for files. The indexer is what picks up a new
file, so a document, ticket or comment added since the last refresh is not
listed, and not reported, until the next one. That short delay is accepted.

## Wave 1 Acceptance

Wave 1 persistence work is complete when real temporary repositories show that:

- An enabled repository is registered once by canonical root path and can be
  removed without repository mutation.
- Full refresh indexes valid primary and active-context items, comments, and
  problems while preserving distinct context rows.
- A single deterministic shared local active context receives discovery
  precedence. Remote observations and exceptional context resources remain
  visible recovery state without creating a multiple-context choice result.
- Marker-only, malformed, duplicate-ID, and inaccessible content stays visible
  as a problem rather than disappearing or being rewritten.
- Deleting or corrupting the SQLite database followed by rebuild produces the
  same discovery result from canonical state and changes no Git or Markdown
  content.
- A checkpointed commit followed by an injected index failure is reconciled as
  index pending without a duplicate commit.
