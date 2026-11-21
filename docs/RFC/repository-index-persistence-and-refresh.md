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

Wave 1 used a full per-context scan of every context on every refresh, rather than Git-diff incremental indexing. Since the 2026-11-21 amendment below ([Per-context indexing](#per-context-indexing-amended-2026-11-21)), a write indexes only the contexts it changed and a refresh re-observes only the contexts that moved; each context it does observe is still read whole. For each accessible primary or active context it observes, an index-only refresh MUST:

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

### Per-context indexing (amended 2026-11-21)

Amended 2026-11-21 by ticket vh-f3wts (`01M7RDVNZDCCVSMWHDY9503852`), Phase 1 of the [fast-writes design](../plans/2026-11-19-fast-writes-design.md), with the planning amendments P-1 to P-5 of its [implementation plan](../plans/2026-11-19-fast-writes-implementation.md) and the owner's decisions recorded in the design. This states what the index does as built in that phase. The publishing and lifecycle phases of the same ticket come later: until they land, a synchronization's index hand-off (`item sync`, `repo sync`, the publication of `comment add`, and `conflict resolve`, `item republish` and the comment hand-off repair, which index through one), `document promote` and `ticket close` still index through a refresh of the whole repository, which re-observes only what moved, as below.

**Each context's rows come from that context alone.** The index stores one context row for the primary and one for each item worktree, and every row it stores for a context (items, comments, relationship edges, problems, folders, activity) is derived from that context's own sources. The primary's items are stored whether or not an item worktree holds the same item. An item worktree that is not a valid context (for example after an outside `git checkout` of another branch) is stored as its own context of kind `invalid`, holding its problems. What depends on more than one context is decided when the index is read: a primary item, with its comments and the primary's problems under its ticket and comment directories, is not presented while a valid stored context whose worktree exists holds the same item, and that pair is the normal state, not stale; an invalid context's problems are presented where an invalid worktree's were before, at `.manyhands/worktrees/<ULID>` in the primary's frame, and the primary's copy of its item, if any, is presented; a primary whose configuration is invalid presents no item contexts; relationships and short codes are resolved over the presented copies only. So creating, releasing, retiring or invalidating an item context changes only that context's rows, and the primary's rows change only when the primary's own sources change.

**Basis, indexed time and dirty mark.** Each context row also records its basis: the head OID, the branch HEAD names, the worktree path, the configuration blob, and for each canonical file the observation read its path, size, inode, ctime, mtime and content hash (no inode or ctime on Windows), with the listing of each canonical directory it read. It records `indexed_at`, the time in nanoseconds at which that context's observation began, and a `dirty` mark: a write sets it on each context it changed once its commit is made, an index step sets it on a context it observed but could not store, and the transaction that stores the context clears it. Until a write's mark is set, the contexts its pending row records as owed read dirty all the same. The primary's row also records the base of its activity map, kept in the registry beside it. The registry migration that adds these is one-way: every existing context reads dirty with no basis, and the next refresh observes each once. A migration that changes what an observation produces marks every context dirty.

**Observing one context.** An observation stats the context's canonical files and directories, reads and parses them, and stats them again. If the two stats differ, it observes once more; if they still differ, nothing is stored for it: the context keeps its previous rows, is marked dirty, and the call answers as a race does (`RetryRequired`). A file whose recorded modification time is less than 2 seconds before its context's observation began, or later, is not trusted by its stat alone: a later basis check compares its content hash, and hashes it again at each check until the context is next observed. A context whose basis records a file modified in a later second than its `indexed_at` (one written after the observation began and before it was read, or with a time in the future) reads stale even when that file's content settles, so a refresh of the whole repository observes it again, which gives it a later `indexed_at`. A source walk neither reads nor records in the basis a file or directory that Git ignores and does not track, by the context's own ignore rules, so build output and caches never move a basis; a tracked file is read even when an ignore pattern matches it. Listing the contexts under `.manyhands/worktrees/` is not a source walk and is unchanged.

**What a write indexes.** An item write's row records, when it is begun and before its commit, the contexts it owes an indexing: its own item's context. The rows of `repo enable` and of the remote commands record none; folder create's refresh row records the primary. After its commit the write claims its row for indexing under the lease it holds, lets the lease go, and observes those contexts with no lease held. It then takes the lease again and checks each observation's basis against the context as it is now: a stat, a new listing only of a directory whose stat moved, and a hash of the few files modified too close to the observation for their stat to be trusted; nothing else is read under the lease (P-3). In one transaction it stores each context that still matches, keeps what was stored for one that moved and marks it dirty, and deletes the rows of one that is gone. The row then completes; or, when a context is left dirty or the lease cannot be had within its wait, the row still owes its hand-off and the write answers `discovery_pending` with its commit. A write's index step does not touch the repository's `refresh_required` or `refreshed_at`.

By command: `ticket save`, `document save`, `ticket slug-assign`, a comment's local checkpoint, `ticket repair`, `document repair` and `document move` index their own item's context (a repair or a move of an item that is only on the primary creates that context and indexes it); `ticket create`, `document create` and the library's `context prepare` index the new context. `context sync-local` and `context resolve-local` index their one context under the lease the local synchronization holds for its whole run, then check its basis again and store it (design decision 33). `context release` and `context retire` observe nothing and delete the removed context's rows. `operation abandon` indexes nothing; the contexts the abandoned row owed are marked dirty, so the next refresh indexes them. `folder create` indexes the primary under a refresh row of its own. `repo enable`, `remote add`, `remote select` and `remote remove` refresh the whole repository, and `repo identity-set` indexes nothing. A write that changed a context marks that context dirty where it marked the whole repository behind before; `refresh_required` is kept for facts of the whole repository (registration, a poll's mark, a migration, a rebuild that must be tried again) and for a context that has no stored row to mark.

**A refresh of the whole repository.** `index refresh`, a poll's index pass, the desktop's refresh at start and the writes above that refresh the whole repository run in two steps, under one index-owner row that the refresh holds from before the first step to the end of the second, so Manyhands writes wait for it. Step 1, with no lease held: the contexts are listed, each one's current basis is compared with the stored one by stat (and by the hash of a racy file), and only a context whose basis moved, that is dirty or owed by a pending row, that is new, that has no stored basis, that holds a transient problem, or that records a file modified in a later second than its `indexed_at` is observed. The primary is observed by the same test, and also while its stored configuration is not valid or when listing the worktree base meets a problem; a change in an item worktree alone does not observe it. Step 2, under the lease: the contexts are listed again and each one step 1 observed is checked again by stat, with nothing read but its racy files; in one transaction each that still matches is stored, one whose row another store replaced since step 1 is left as that store left it, and any other keeps its previous rows, marked dirty. A context that appeared since step 1 is not stored, and counts as left dirty. With none left dirty, the rows of contexts no longer listed are deleted and `refreshed_at` becomes the time step 1 began; each stored context's `indexed_at` is the start of its own observation. With any left dirty, the refresh answers `RetryRequired` (`busy` for `index refresh`), and the next one observes it. Step 2 checks again only the contexts step 1 observed: a change after step 1 to a context step 1 found unmoved is like a change after the refresh, and is seen by the next refresh, poll or desktop start, or reads stale (freshness on read, below). `index rebuild` is built from the same per-context observations and stores, and keeps its two whole observations.

**Activity.** The value is unchanged: an item's newest first-parent commit whose diff against its first parent touches the item's Markdown or one of its managed comments, or a newer uncommitted change. In an item context it is found by a first-parent walk from the context's head, with each diff restricted to the item's own paths, that stops at the first commit touching them, within the last 1024 commits as before. For the primary, the registry keeps a map from each path under `.manyhands/` and `docs/` to its newest touching first-parent commit, as of a base head. When the primary is indexed at a head whose first-parent chain reaches that base, only the commits after the base are diffed, each once; otherwise (a reset, a rewrite, a fast-forward to a merge that has main as its second parent, or no map yet) one first-parent walk rebuilds the map. An item's activity on the primary is the newest entry over its current paths. The primary's map has no 1024-commit window (ruling R-1).

**Freshness on read.** `index status`, and the `index` state a list carries, read `stale` while the repository is marked behind as a whole (`refresh_required`) or any context is dirty or owed by a pending row; `refreshed_at` is when the last complete refresh of the whole repository began. A read of one item's file (`show` by ID or by path, and the comment reads) reports stale when that file's context is dirty or owed an indexing, or when the file was modified in a later second than the one its context's observation began in (`indexed_at`), or, for a context an older build stored with no such time, than `refreshed_at` (P-2). A write to one item therefore leaves the reads of every other item fresh. Content hashes only settle racy basis entries; no read compares them, and no read lists a directory. A change made outside Manyhands to an item is picked up by the next refresh, poll or desktop start; until then a read of a file modified since its context was indexed reports it stale, by the rule above.

**Reads that decide on other items.** A `ticket create` or `ticket save` that sets `deps` or `parent` first compares, with no lease held, the basis of every context, the primary included, with the one the index stores; the tickets and comments of each context whose basis moved, that has no stored row or basis, or that is dirty or owed an indexing, are read from disk for its relationship check, in place of what the index holds of that context, and nothing is written to the index. A cycle that a commit made outside Manyhands would close is therefore refused before the next refresh. `ticket check` still reads the index alone.

**Known limit: ignore rules and the tracked set.** What a source walk passes over depends on the context's ignore rules (its `.gitignore` files, `info/exclude`, `core.excludesFile`) and on what its Git index tracks, and none of these is in the basis. An uncommitted edit of a `.gitignore` or of `info/exclude`, or a `git add -f` or `git rm --cached`, therefore changes what an observation of the context would read without moving its basis, and a refresh skips the context: its rows keep what was read before. `index rebuild` reads it again, as does any change that moves its basis, such as touching a file its basis records or a commit in its worktree.

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

### Index steps and pending rows (amended 2026-11-21)

Amended 2026-11-21 by ticket vh-f3wts (`01M7RDVNZDCCVSMWHDY9503852`), Phase 1 of the [fast-writes design](../plans/2026-11-19-fast-writes-design.md); see [Per-context indexing](#per-context-indexing-amended-2026-11-21) for what each step indexes.

**Scans stay outside the lease.** A write's index step observes its contexts with no lease held, then takes the lease to check each observation's basis again and store it (planning amendment P-3); the only files read under the lease are the few racy ones of the contexts being stored. The write keeps its own pending row throughout, so no Manyhands writer of the same item changes the context meanwhile, and the re-check under the lease guards against editors and `git` acting outside Manyhands. A refresh of the whole repository lists and observes with no lease held, and checks again and stores under it. There are two exceptions. A local-context synchronization (`context sync-local`, `context resolve-local`) observes its one context under the lease it holds for its whole run (design decision 33). `index rebuild` observes the whole repository twice, as before, the second time under the lease.

**Retried as indexing only.** A completed commit whose index step is pending is retried as indexing only, by `operation resume` or by the same request, and indexes the contexts its row recorded when it was begun and nothing else; a recorded context that is gone counts as indexed, and its rows are deleted. Folder create's refresh row records the primary, so its resume indexes the primary alone. A row that records none (one an older build began, or a row of `repo enable` or a remote command) is handed off to a refresh of the whole repository, as before.

**Per-item holds.** A pending local row is of one item when its action is an item write (`save_ticket`, `save_document`, `submit_comment`, `repair_item`, `prepare_context`, `release_context`) and it names its item. Any other row is of the whole repository: a `context sync-local` row, a retirement's row, a refresh or rebuild row, the rows of the repository's own commands, and an item write's row that an older build began without naming its item. A pending row holds a call, which then may not take it up, when the row is of the call's own item; when the row is of the whole repository; or when the row is of another item, is still mid-write, and the call is of the whole repository or was checked against other items (a `ticket create` or `ticket save` that sets `deps` or `parent`). A row of another item whose commit is observed and that owes only its index hand-off holds nothing: other items' writes and synchronizations, a refresh of the whole repository, a poll, a rebuild and the retirement of another item's context go on past it. A refresh indexes that row's contexts like any other dirty ones, but never changes or completes the row. A retirement of the row's own item's context is still held by it. Whether a held call waits (while its holder is live, then `busy`) or is refused (`recovery_required` naming the holder, once it is stale) is decided as before. Folder create, `document promote` and `ticket close` index under refresh rows of the whole repository, so a crashed one, like a crashed `context sync-local`, still holds refresh and poll until it is resumed (design decisions 14 and 16). A synchronization's local index hand-off row is also still of the whole repository; the publishing phase of the same ticket makes it of its item.

**A crashed write whose commit landed.** Its row owes its hand-off, and the contexts the row records read as owed an indexing. Refresh and poll go on past it and index those contexts. A context a pending row owes reads current again once the index stores it from an observation that began in a later second than the row's last recorded step, for a row whose commit is observed (design decision 32): after a crashed save, a refresh that indexes the save's context makes the item read current in `index status`, lists and shows, while the row stays open, listed by `operation list` and among the pending operations of `index status`, until its own `operation resume` or a retry of its request completes it. The contexts of a row still mid-write stay owed.

### Unavailable-repository registration removal

Amendment for ticket `01M4R19P943QZKHHKRXV2D2TP2` (2026-10-19): removing an
exact stored-root registration is a narrow exception to the common-Git lease
requirement when the root is positively absent, or an exact Git open finds no
repository and `.git` is positively absent. Permission/I/O errors, damaged Git
metadata and dangling indirection do not qualify. This changes no other
repository resolver or mutation's lease requirements.

Removal acquires the root-keyed application-data bootstrap lease, the Git
lease when a repository is present, and then the exclusive cache guard, in
that order. It reobserves the root and held lease identity after acquiring the
guards. If a repository appears or its lease file has been replaced, removal
returns busy and releases the guards rather than taking a Git lease under the
cache guard. Existing per-lock wait bounds apply; there is no internal retry
loop and no claim that their sum is one lock's timeout.

The registration stamp is compared before journal creation and in the deletion
transaction. Root-keyed local journal rows and the registration's dependent
rows are removed atomically. Durable request/confirmation records remain for
matching replay, as do draft files and application-global key and trust state.
The registry-only route preserves unrelated unfinished recovery evidence and
accepted requests, even when their original Git lease pathname is gone. Only
the matching removal's own pending operation can continue this way; removal is
not implicit abandonment. See the [CLI contract](cli-contract.md#registration-removal-targeting)
for selection, consent and replay behavior.

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
