# Fast writes: index what a write changed, publish with few connections, promote and close without repeated scans

Ticket `01M7RDVNZDCCVSMWHDY9503852` (vh-f3wts, which absorbed vh-z9ep3). Status: draft, revised after review rounds 1 and 2, awaiting the owner's approval. Base: ticket branch `b730e7d7` (main `790a71ed`).

## Intent

A person expects a local save to be quick, checkpoint commit included; sharing an item or adding a comment is "just a push" and should be quick too; promotion and closing do more, but should be as fast as they can be. Today, on this repository (314 items, 57 ticket worktrees), a save takes about 36 s, `item sync` 55–75 s, `comment add` about 100 s and `ticket close` 150–880 s (median 263 s), and each grows with every commit and every item. Almost all of it is repeated whole-repository work: an index refresh after every write that re-reads every item and every worktree twice and walks history per item, about 13–15 SSH connections per publication, and about 12 whole-repository scans per close. This change makes each write cost about its own work.

## Decisions (owner, 2026-11-19, in the planning chat; recorded in ticket comments `01M7TZGEBQMXNGF24H4DXMXDFN` and `01M7V0RT95T2QZ89SF00K0YBCS`)

1. **Targets**, on this repository with a release build: a local save (`ticket save`, `document save`, `ticket create`, a comment's local checkpoint) under 1 s; `item sync` and `comment add` 2–4 s; `document promote` and `ticket close` under 10 s.
2. **Each write is its own operation.** A write re-reads and re-indexes only what it changed: its item's branch, worktree, file, ticket or comment. It does not re-read another item's branch or worktree. ("I don't think it needs to care about any other worktrees/branches than the one with the file, ticket or comment being saved and committed.")
3. **The full refresh stays complete but cheap.** `index refresh`, poll and the desktop's start re-read only what changed since it was indexed.
4. **No full history walk for activity times.** The design's "full scan" meant rebuilding the SQLite index from the files, not walking Git history. Every open item has its own branch until it is merged; a merged item is frozen on main, and editing a merged document gives it a new branch and worktree.
5. **Publishing, promote and close**: reuse one SSH connection per remote call and drop repeated listings and inspections; no refresh after a push that changed nothing locally; a close's or promotion's preview computed twice instead of six times, with one index update; key listing without the registry migration and its audits; audits of the records in use only.
6. **One ticket, three measured phases**: the index first, then publishing, then promote and close.
7. **The ticket's Acceptance is amended**: the index gives today's results except that a change made outside Manyhands to another item is seen at the next refresh, poll or desktop start, or flagged stale when read (saved to the ticket in `26e3a01c`; its recovery clause is corrected to name R-3, see Review record round 2).
8. **`discovery: current` on a write** means the contexts that write changed are indexed: a clarification, with no schema version change.
9. **Registry open audits the schema and the unfinished rows only**; a completed record is audited when a read uses it.
10. **Per-item index holds** (R-3): a per-item index step holds only its item, and a crashed write whose commit landed does not block refresh and poll. (Accepted as "a full refresh … would finish that save's index step"; review round 2 changed the mechanism, not the outcome: see R-3.)
11. **Pre-commit safety checks** that read other items (duplicate ID, branch checked out elsewhere) stay, as an explicit exception to decision 2, made cheap only if measured above about 100 ms. (Stated to the owner as the default; no objection.)
12. **The CLI keeps retrying close and promote** when their index step is pending (R-7); this replaces decision 5's earlier "don't retry" item, which rested on a wrong reading that saves are retried. (Stated to the owner as the default; no objection.)

## Goals

1. A write updates the index for the contexts it changed and nothing else, once, under the lease it holds for the write (Phase 1's command table).
2. A whole-repository refresh re-observes only contexts whose basis changed and gives the same complete result as today.
3. Every item's `activity_at` and `activity_source` are unchanged, except as Ruling R-1 states for history past 1024 first-parent commits.
4. A sharing sync opens at most 4 SSH connections when nothing needs merging and 5 when it merges main into the item, and runs no whole-repository refresh.
5. A confirmed `ticket close` or `document promote` runs no whole-repository discovery pass for its previews, at most two previews per attempt, and one index update per exit.
6. Only registry open runs the migration; a guard audits the rows it scans, not every stored row.
7. The targets in decision 1 are met and measured before and after each phase.

## Non-goals and anti-goals

- No change to codes, recovery actions or JSON shapes (decision 8 clarifies `discovery`).
- No weakening of a safety property: crash recovery and resume, detection of a remote that moved, refusal to recreate a deleted branch, publication proof, cleanup safety for ignored files and nested repositories, and refusal of malformed registry records before they are used.
- No file watcher, no background indexer and no deferred refresh.
- No change to SQLite durability settings.
- A whole-repository refresh, a rebuild, and repository-scoped rows (`context_sync_local`, `retire_context`) keep their repository-wide hold; only per-item index steps narrow (R-3).

## Facts checked

All at `b730e7d7`, by the author and by the round-1 and round-2 reviewers.

- **The refresh after every write is whole-repository.** `handoff_post_authoritative` (`src/repository.rs:4012`) claims the row for indexing, drops the lease (`:4033`) and runs `refresh_repository_claimed` (`:1512`) → `refresh_repository_observed` (`:1568`): `observe_root` before and after (`:1580`, `:1602`), every unchanged context stored from the first observation, `RetryRequired` for the first changed one (`:1656`, answered as `busy`), and a full `repository_snapshot` (`:1677`) that the hand-off discards. `persist_context_refresh` (`:6413`) deletes a context's rows and rewrites them all; for the primary it also rewrites configuration observations and folders.
- **Local sync has its own whole-repository indexer.** `local_sync::run::index` (`src/repository/local_sync/run.rs:886`) observes twice, persists the whole observation, and sets `refreshed_at`.
- **The primary's rows depend on every active context.** `observe_root` observes every worktree (`observe_active_contexts`, `src/repository/discovery.rs:403`), then hides each primary item that a valid active context holds (`leave_active_items_to_their_contexts`, `:776`), and stores an invalid worktree's problems on the primary.
- **Holds.** `is_in_the_way` (`src/repository/recovery.rs:2029`) puts a row in `INDEX_OWNER_STATES` in every call's way, "of whatever item: two index owners of one repository would each rewrite the whole index"; vh-bhepg's per-item hold (`Scope`, `:1993`) otherwise lets writes on different items proceed, but any item row is in a whole-repository call's way. `require_no_pending_local` (`:1119`) refuses polls, other items' syncs and retirements while any local row is pending. Liveness is judged only for indexing rows and rebuilds. `owes_only_its_hand_off` (`:2425`) says a row's write is done and only its hand-off is owed. `operation resume` and `index refresh` run an index step as a whole-repository refresh (`src/repository/mutation/bind/resume.rs:1132`, `bind/index.rs:138`). The comment publication binding requires the sync's local row to be a whole-repository refresh row (`recovery.rs:182-190`).
- **Activity is a per-item history walk.** `latest_commit_touching` (`src/repository/discovery.rs:697`) loads up to 1024 first-parent commits from the context's HEAD (`MAX_ACTIVITY_HISTORY_COMMITS`, `:22`) and runs an unfiltered `diff_tree_to_tree` per commit until one touches the item's owned paths, which come from the working-tree sources. The root pass computes it for every root item, including hidden ones.
- **Activity is used only for order and age**: `ticket_list_order` (`src/repository/read/items.rs:945`), the CLI's `changed_at` and `change_source`, the desktop's "Updated" sort and age labels.
- **Freshness is per repository.** Every checkpoint sets the repository's `refresh_required` (`mark_checkpoint_refresh` → `mark_document_refresh_required`, `src/repository.rs:4002`, `:11284`), as do closure (`closure/checkpoint.rs:559`), promotion (`promotion/run.rs:1699`, `promotion/lifecycle_record.rs:941`), conflict resolve (`remote/state.rs:6530`), poll and retirement. Only `reconcile_disappeared_contexts` (`:6563`), rebuild and local sync clear it. Lifecycle results choose `discovery` current or pending from it (`promotion/run.rs:1695`). Reads compare a file's mtime with the repository's `refreshed_at` (`ItemFile::newer_than`, `src/repository/read/items.rs:1416`) in `show` by ID and by path (`:1792`, `:1884`) and in the comment reads (`read/comments.rs:313`, `:349`).
- **The CLI re-runs close and promote when their index step is pending.** `should_retry` (`src/bin/manyhands-cli/mutate.rs:100`) retries a `busy` or `partial` answer whose first recovery action is `request.retry`, three times. Close and promote offer it for `discovery_pending` (`src/repository/mutation/bind/lifecycle.rs:582`); folder create and poll when retryable; saves, creates and `comment add` offer `operation.resume` (`src/repository/mutation/command.rs:566`, pinned by `a_hand_off_no_operation_owes_is_offered_as_a_refresh`, `outcome_tests.rs:1396`).
- **Every transfer reconnects.** `src/repository/transport/remote.rs:403-429` disconnects and calls `connect_auth` again before each transfer, so each transfer submits the selected key exactly once to a host equal to the finalized one; `connect_ssh` (`src/repository/transport/operation.rs:321`) makes a first connection for every remote call. A pushing sync makes about 13–15 connections.
- **The post-push refresh is whole-repository.** `synchronization_refresh` (`src/repository/remote/sync.rs:7081`) passes an empty matcher unless the outcome is `PublishPending`.
- **A confirmed close or promotion previews repeatedly.** `L::prepare` or `preview` at `src/repository/promotion/run.rs:492`, `:659`, `:679`, `:710` and `src/repository/promotion/integrate.rs:119`, after the boundary's prepare (`bind/lifecycle.rs:936`). Each runs `observe_root` (`src/repository/closure/observe.rs:300`, `src/repository/promotion/observe.rs:215`). Its two child syncs and its finish each run a whole-repository refresh.
- **Key listing migrates the registry.** `list_shared_keys` (`src/repository/keys/registry.rs:329`) takes the cache write guard and runs `migrate_registry`, which reaches `local_sync::record::migrate` (`src/repository/recovery.rs:724`) and its `audit` (`src/repository/local_sync/record.rs:469`), which runs `image_proof::audit` (`:485`) over every stored proof of every repository. Registry open already migrates and audits once (`src/repository.rs:4264-4267`). About 40 non-test callers run `migrate_registry`.
- **The RFCs forbid full scans under the lease**: index RFC, "Operation Coordination" ("full read-only scans remain outside the lease") and "Remote Polling" ("full scans MUST NOT hold the repository lease"); Git RFC, "Remote Coordination" ("full index scans MUST NOT hold it").

## Approaches

1. **Index what a write changed, and make the full refresh skip what did not move (chosen).** Writes stop paying for the repository; refresh, poll and rebuild stay complete; activity values and answers stay as they are.
2. **Keep one whole-repository refresh per write but make it cheap.** Rejected: it still couples writers (an index owner is in every call's way, and its observations fail when any other worktree changes) and still grows with the number of worktrees.
3. **Defer or background the refresh.** Rejected: the CLI has no resident process, the desktop worker would still be held, and the cost moves rather than goes.

Within publishing and promote/close there is one sensible approach each: remove repetition and keep every check that carries a property.

## Design

### Phase 1: the index

**Each context's rows depend only on that context.** The index stores one row per context (the primary checkout, and each item worktree), and every row it stores for a context (items, comments, edges, problems, folders, activity) is derived from that context's own sources alone. What depends on more than one context is computed when the index is read:

- a primary item is not presented while a valid active context holds the same item (the RFC's "replaces the primary presentation when active");
- an invalid item context is stored as a context row marked invalid, with its own problems, and reads present those problems where they appear today;
- two active contexts for one item are reported as today.

So creating, releasing, retiring, repairing or invalidating an item context changes only that context's rows, and the primary's rows change only when the primary's own sources change.

**Basis, dirty mark and indexed time.** Each context row also records, through one named additive migration:

- its *basis*: the head OID, the HEAD's branch name, the worktree's registered path, the configuration blob OID, and for each canonical file the observation reads its path, size, inode, ctime and mtime (nanoseconds where the platform gives them) and content hash, plus the listing of each canonical directory it reads (a ticket's comments directory; for a document context, the `docs/` tree, which its observation reads for duplicate IDs; for the primary, every directory the root scan reads and its folders);
- `indexed_at`, taken before the observation starts (in nanoseconds);
- `dirty`, set before a write commits to the context and cleared by the transaction that indexes it.

A context row with no basis (written by an older build) is dirty. Each operation row records, before its commit, the context keys it owes indexing, so a resume knows them.

**Observing one context.** Observe it once: stat the canonical files, read and parse, stat again. If the two stats differ (an editor or `git` wrote during the read), observe once more; if they still differ, store the rows with the context left dirty, and the command answers as a race does today (`RetryRequired`, `busy`). A file whose mtime is within 2 s of the indexed time (coarse filesystems) is not trusted by stat alone: a later basis check compares its content hash.

**What each write indexes.** A write indexes, in one transaction and under the repository lease it holds for the write, the contexts it changed:

| Command | Contexts it indexes |
| --- | --- |
| `ticket save`, `document save`, `ticket slug-assign`, `comment add` (local checkpoint), `context sync-local`, `context resolve-local`, `conflict resolve`, `item republish`, `ticket repair`, `document repair`, comment hand-off repair | its own context; one that first creates the context (an item only on the primary) indexes the new context |
| `ticket create`, `document create`, `context prepare` | its new context |
| `document move` | its own context (created if the document is only on the primary) |
| `context release`, `context retire`, `operation abandon` of an operation that created a context | none; the removed context's rows are deleted |
| `ticket close`, `document promote` | the primary (main moved); the removed context's rows are deleted; any context its children changed (Phase 3) |
| `item sync`, `repo sync`, the sync half of `comment add` | each context it merged into or fast-forwarded (the primary when it merged remote main), under the lease of its local-update step; a sync that changed nothing locally takes the lease briefly and records its hand-off when the context's basis equals the stored one and it is not dirty, and otherwise indexes the context |
| `folder create` | the primary |
| `poll once` (with or without `--retire-completed`), `remote history`, `index refresh`, the desktop's start | a whole-repository refresh, below, plus the deletions of retired contexts |
| `index rebuild` | a rebuild (all contexts, today's two observations, in the new form) |
| `remote add`, `remote select`, `remote remove`, `repo enable`, `repo identity-set` | a whole-repository refresh, as today |

The other writers of the repository's `refresh_required` (closure, promotion, conflict resolve, local sync, retirement) mark their contexts dirty instead; `refresh_required` stays for whole-repository facts (configuration, registration, remotes, a poll's mark). Local sync's own indexer becomes the per-context index step of its context.

**The per-item index hold (R-3).** A per-item row's index step keeps its item scope: in an index-owner state it is in the way of its own item only. A row that owes only its hand-off (`owes_only_its_hand_off`) is not in the way of a whole-repository refresh, a poll, `require_no_pending_local`, or another item's sync: they neither take it up nor change it, and a whole-repository refresh indexes its contexts like any others (they are dirty). The row is closed only by its own `operation resume` or a retry of its request, which index the row's recorded contexts only (never a whole-repository refresh under an item's ID), find them current, and complete the row. A row still mid-write (its commit not yet observed) keeps today's rules: in the way of its own item and of whole-repository operations, waited on while live and `recovery_required` when stale (vh-bhepg). The comment publication binding's check (`recovery.rs:182-190`) accepts the per-context hand-off row of the sync.

**The whole-repository refresh.** `index refresh`, a poll's hand-off and the desktop's start, in two steps:

1. Outside the lease: list the contexts (the primary checkout and `.manyhands/worktrees/*`), compare each one's current basis with the stored one (stat; content hash for racy entries), and observe every context whose basis differs, that is dirty, that is new, or that holds a transient problem (inaccessible, retry-required; a permanent problem such as missing front matter does not force a re-read). Contexts that disappeared are noted.
2. Under the lease: list the contexts again and re-check the basis of each one observed in step 1 (stat only, no reads); store those that still match; leave any that moved or appeared dirty; delete the rows of those that disappeared. No observation runs under the lease. If any context is left dirty, the refresh answers as a race does today (`RetryRequired`, `busy`), and the next refresh observes it. The repository's `refreshed_at` records the time step 1 began.

This follows the RFCs (the scan is outside the lease; the lease covers the re-check and the store) and stops a refresh from storing over a newer per-item write, whose context no longer matches its basis.

**Activity (R-1, R-2).** The value is today's: the newest commit on the context's first-parent history, from its head, whose diff against its first parent touches an owned path, or a newer uncommitted change. How it is found:

- *Item contexts:* walk the first-parent chain from the head with a diff restricted to the owned paths (a pathspec with pattern matching disabled, so paths compare exactly as `commit_touches_paths` does, mode-only changes included), stopping at the first match, usually the head or one or two merges of primary back; keep the 1024-commit window. Cache the commit time keyed by (head OID, owned-path set); the uncommitted part is judged at each observation, as today.
- *The primary:* keep a map from each Manyhands path to its newest touching first-parent commit, valid for a base head S. When the primary is indexed at head H: if S is an ancestor of H (`graph_descendant_of`) and on H's first-parent chain, diff only the commits from S to H, once each, and update the map; otherwise (a reset, amend, rebase, forced move, a fast-forward to a branch that merged main as second parent, or no map yet) rebuild the map with one walk that diffs each first-parent commit once. An item's activity is the newest entry over its current owned paths, so a path that leaves the set (an invalid comment) stops counting. The map covers every root path, including items an active context hides. The 1024-commit window is dropped for the primary (R-1).
- A rebuild uses the same functions.

**Freshness on read.** `index status` reports `stale` while the repository's `refresh_required` is set or any context is dirty; its `refreshed_at` is the start of the last whole-repository refresh. Every read that today compares a file with `refreshed_at` (`show` by ID and by path, the comment reads) instead compares the content hash of the file it reads anyway with the hash stored in its context's basis, and also reports stale when that context is dirty; no read lists a directory. The lifecycle's choice of `discovery` current or pending reads the dirty marks of the contexts it changed.

**Reads that decide on other items.** A save that sets `deps` or `parent` keeps vh-bhepg's rule (refused behind another item's row that is mid-write) and, before its relationship check, compares the basis of each item context (stat only); any context whose basis moved is read from disk for the check, without writing the index. Lifecycle previews use the same rule for their facts about other items (Phase 3).

**Crash and retry.** A crash after a write's commit leaves its row owing its hand-off and its contexts dirty; `operation resume` or the same request indexes the recorded contexts (the RFC's "retried as indexing only"), and meanwhile refresh and poll are not blocked by it. A crash inside an index transaction stores nothing. A replay of a request whose row completed answers as today, with its commit.

**Smaller costs on the write path.** `first_parent_configuration_entries` (`src/repository.rs:11204`) stops at the first match; the discarded `repository_snapshot` at the end of a hand-off is skipped. The pre-commit safety checks stay (decision 11).

### Phase 2: publishing

- **One connection per remote call.** A remote call lists refs and transfers on the connection `connect_ssh` opened: the transfer runs inside its closure (git2's connection borrows the `Remote`) and reuses its callback attempt, so the transfer still submits the selected key exactly once to the finalized host and any reconnect inside libgit2 fails closed. The local rechecks before a transfer (`recheck_ssh`, `recheck_trust`) stay; a failed recheck refuses as today. There is no automatic reconnect on error.
- **Fewer listings.** The fetch lists and downloads on one connection and drops the "before" listing; the "after" listing, which carries the moved-remote check, stays on its own connection. The push-absence check stays before any local merge. The pre-push listing and the push share one connection. The verification listing after a push stays (Open question 1). Counts: 4 connections when nothing is merged (fetch, after, push, verify), 5 when main is merged into the item first (adding the absence check), against about 13–15 today.
- **Fewer inspections.** Back-to-back inspections of an unchanged worktree merge; the inspections inside the fetch window, which changes nothing local, go; context-tree validation is cached by tree OID together with the item kind, item ID, configuration blob and build, since a first edit's branch shares main's tree. A head and status check stays immediately before each local update and before the push.
- **No refresh after a push**, per the sync row of Phase 1's table.
- **`comment add`.** The comment receipt is proved once, not twice.

### Phase 3: promote and close

- **Two previews per attempt.** A confirmed request computes the preview once against the consent digest and once under the lease before integration (where `prepare_integration_for` already checks the expectation, `src/repository/promotion/integrate.rs:183`, and keeps its re-confirmation path); `cleanup_only`, the decision after the context child and the decision before integration read from those two. A resumed attempt has no consent preview and re-checks under the lease. A preview reads the item's own facts from its context and the primary branch, and facts about other items (duplicate IDs, nonconforming content) from the index after the basis check of "Reads that decide on other items", so an outside change is read from disk.
- **One index update per exit.** The children record their index step as owed to the parent; on every exit (finish, abandon, cancel, a stop at awaiting publication, and the later finish of such a stop) the parent takes the lease and indexes the contexts its children changed and its own, per Phase 1's table.
- **One cleanup precheck**, under the lease immediately before removal, keeping every check `require_cleanup_clean` makes (`src/repository/cleanup.rs:536`).
- **Registry audits (R-5, decision 9).** Only registry open runs `migrate_registry`, and it audits the schema and the unfinished rows; `list_shared_keys` and the other callers that run after open in the same process take a read guard and do not migrate. The open's memo is keyed by the registry file's identity and `PRAGMA schema_version`, so a replaced file migrates again. A guard audits every row it scans, which for local-sync guards means every unfinished row, not only the rows that match its item; a completed record is audited when a read uses it.
- The SSH and inspection changes of Phase 2 apply to the children.

## Rulings

Each is a builder's call recorded for review, with its cost if wrong.

- **R-1. Activity keeps today's value; the window is dropped for the primary only.** Decision 4 is met by short walks from each item's head and an incremental map for the primary, with one full walk only when the map cannot be advanced. An open item's value is the newest commit that changed the item, not the branch tip, because every share that finds main moved adds a "Merge primary into ticket" commit, and counting it would reorder the Updated view after each share. The primary's map has no 1024-commit window: an item whose newest change is more than 1024 first-parent commits old (none today, at 179) gets its true time instead of today's fallback. Cost if wrong: an ordering difference after 1024 commits.
- **R-2. A full walk is acceptable when the primary moves to a head that does not extend the map's base.** One diff per commit, after a reset or rewrite, not on a normal write. Cost if wrong: a slow first index after such a move.
- **R-3. Per-item index holds (decision 10).** A per-item index step is in the way of its own item only; a row that owes only its hand-off does not block refresh, poll, retirement or other items' syncs, which index its contexts but never change its row; only its own resume or retry closes it. Round 2 replaced "a full refresh finishes that save's index step" with this, because finishing another operation's row needs a liveness judgement that only indexing rows have, and a `comment add` publishing without the lease would have its row completed under it. The outcome the owner accepted holds: a crashed save whose commit landed does not block refresh or poll, and its item is indexed by them. Cost if wrong: a crashed save's row stays in `operation list` until resumed, as today.
- **R-4. Freshness per context, with an additive migration; cross-context presentation at read.** Cost if wrong: a stale item not flagged, or a primary copy shown beside its active context; each is pinned by a test. The migration is one-way (see Risks).
- **R-5. Audits by use (decision 9).** The property kept: no malformed record is used as authority, and a guard that concludes absence has audited every row it scanned. Known limit: a completed row whose key is corrupted is not found by a lookup and so not audited; completed rows are not authority for any write.
- **R-6. The primary is re-observed whole whenever it is indexed.** It is one context, its observation is cheap once activity is incremental and hiding is computed at read, and a partial primary writer would have to cover items, folders, problems and configuration. Cost if wrong: the primary's parse cost on close, promote, folder create and primary syncs; measured in Phase 1.
- **R-7. The CLI keeps retrying close and promote when their index step is pending (decision 12).** After Phases 1 and 3 a retried close or promotion indexes its recorded contexts only, so the retry is cheap and completes the hand-off.

## Measurement

- **Before and after each phase**, on a replica of this repository (its history and its 57 worktrees, isolated data directory and HOME, release build): `ticket save`, `document save`, `ticket create`, `comment add` (its local checkpoint timed apart from its publication through the CLI's `progress:` lines), `item sync`, `document promote --prepare` and `--confirm`, `ticket close --prepare` and `--confirm`, `index refresh`, `poll once`. The publishing measurements use a local SSH server, so connection counts and handshakes are real. Each phase also takes a sampled profile of a save, a sync and a close and records that `latest_commit_touching`, a second observation and `image_proof::audit` no longer appear. The method is in the ticket ("How to measure"); the numbers go into the ticket at each phase.

## Tests

Per AGENTS.md's Testing Strategy: the cheapest layer, variations in table-driven tests, no real waits, named edits only.

- **Library unit tests:**
  - activity: a table of histories (direct commits, merges of primary into an item branch, comment additions and invalid comments, a mode-only change, a path with glob characters, a merged item on main, the primary advanced, reset and fast-forwarded to a second-parent merge, a rebuild) pinning today's `latest_commit_touching` values, with the number of commits diffed per case;
  - the command table: one row per command, asserting which contexts it indexed and that other contexts' rows and basis are untouched;
  - read-time presentation: a first edit hides the primary copy; a release, a retire and an outside `git worktree remove` show it again; an outside `git checkout` that invalidates a context shows the primary copy and the context's problems; two active contexts for one item report as today;
  - basis and observation: an editor write during the read, a same-second same-size edit, an mtime-preserving copy, a branch switch at the same OID, a new comment file, a folder made outside Manyhands, a context added between the refresh's two steps;
  - the whole-repository refresh: observes only changed contexts (counted), picks up an outside commit and edit, reconciles a removed worktree, leaves dirty (and answers `busy`) a context that a per-item write changed between the two steps, runs no observation under the lease (a hook);
  - freshness on read: `dirty` set before commit and cleared by indexing; `index status`, list `data.index`, `show` by ID and path and the comment reads after a write, after an outside edit, and with a context row from an older build;
  - the hold: the `Scope` table (`src/repository/recovery_scope_tests.rs`) for indexing and invalidated per-item rows against other items, against a whole-repository refresh, and for a row owing only its hand-off against refresh, poll and other items' syncs; a library test parks item A's index step on a hook and completes item B's write;
  - crash and retry: a crash after the commit (refresh and poll proceed and index the item; the row stays until resumed; resume completes it and indexes only its recorded contexts), inside the index transaction, and after a lifecycle child on each parent exit;
  - relationship check and previews read a context whose basis moved from disk;
  - audits: a guard refuses a malformed unfinished row of another item; key listing runs no migration (counted); a confirmed close performs at most two previews and no whole-repository observation (a library counting hook).
- **Non-SSH integration**: the existing goldens show that answers are unchanged.
- **One SSH case** (existing remote target): a pushing `item sync` uses 4 connections and a merging one 5, counted with `accepted_sockets()` (`tests/support/ssh_remote.rs:549`). It needs SSH because connection reuse is transport behaviour.
- **Named edits** (the plan lists each with its change): in `tests/discovery_rebuild.rs`, `refresh_scan_race_retains_previous_rows_then_converges_on_retry`, `no_mutation_scan_race_retry_preserves_state_after_external_change`, `refresh_does_not_persist_private_markdown_while_detecting_source_races`, `refresh_retry_replaces_a_previously_persisted_context_when_its_observation_changes`, `refresh_retry_replaces_a_persisted_root_when_configuration_problem_changes`, `assert_active_worktree_race_preserves_rows`, `discovery_public_types_hold_metadata_only`; in `tests/read_items.rs`, `a_folder_made_while_a_refresh_observes_makes_the_refresh_try_again` and `refreshed_at_is_set_only_when_a_refresh_completes`; in `tests/read_status.rs`, `index_status_of_an_interrupted_refresh_is_stale_and_says_why`; in `tests/mutation_replay/reenter.rs`, `writers_on_two_tickets_both_complete_while_one_is_unfinished` (its `index refresh` no longer answers `recovery_required` for a row that owes only its hand-off) and any other case that asserts a whole-repository refusal behind such a row; the `Scope` table rows for item rows against whole-repository calls; the observation hooks in `src/repository/remote/sync_tests.rs` and `src/repository/comment_publication_tests.rs`; the reconnect checkpoints in `tests/ssh_transport/exact.rs` (`tracking_boundaries`, `reference_status_rejection`) and `tests/ssh_transport/synchronization.rs` (`cancel_before_transfer`); and the socket counts in `tests/ticket_closure/journeys.rs` and `tests/document_promotion/advanced.rs`.

## Verification

Static gates across the tree and the fast tier, per AGENTS.md. Phase 2 changes `src/repository/transport/` and `src/repository/remote/`, so every SSH-harness target runs once at review-ready. Native CI runs after merge; connection reuse on Windows is the platform risk to watch there.

## Document amendments

- `docs/RFC/repository-index-persistence-and-refresh.md`: "Index-Only Refresh Algorithm" (per-context rows from the context's own sources, presentation across contexts at read, basis and dirty mark, write-scoped indexing, a whole refresh that re-observes changed contexts, incremental primary activity; "Wave 1 uses a full per-context scan" becomes history), "Operation Coordination and Recovery" (a write indexes under its lease; a whole refresh scans outside the lease and re-checks and stores under it; per-item index holds), freshness on read.
- `docs/RFC/git-workflow-and-conflict-recovery.md`: the item sync and primary sync steps "Refresh discovery after the push is observed" become the Phase 1 sync rule; the count of `git-upload-pack` commands per sync.
- `docs/RFC/desktop-information-architecture-and-editor.md`: "Save … refreshes discovery" becomes "indexes the saved item"; the stale-scan wording.
- `docs/RFC/cli-contract.md`: the `effects` definition (`discovery` on a write, decision 8), the `index refresh` and `operation resume` rows, and the C5 known limits and follow-ups this change settles.
- Skills: `manyhands-finding-work` ("Is what I see current"), `manyhands-polling-and-index` and `manyhands-recovery`: a write no longer refreshes other items; an outside commit or edit is seen by the next refresh, poll or desktop start, or flagged stale on read; a crashed write's row no longer blocks refresh and poll.
- The ticket's Acceptance (decision 7), with its recovery clause naming R-3.

## Risks

- **One-way registry migration.** A build from this branch migrates the registry it opens, and older builds may then refuse it. Development and measurement use scratch registries; the first real use is after merge, from main, and the merge note says so.
- **A write that changes a context unseen.** Phase 1's table lists the commands as of `b730e7d7`; a new command must add its row. Each row has a test, and the dirty mark and the read-time hash check make a miss visible as stale rather than wrong.
- **Outside changes are seen later.** A commit or edit made outside Manyhands in item B is no longer picked up by a save of item A; it is seen by the next refresh, poll or desktop start, or flagged stale on read. This is decision 2's intent.
- **Primary cost.** R-6 re-observes the whole primary on close, promote, folder create and primary syncs; if Phase 1's measurement puts it above about 300 ms, the plan adds a partial primary writer.
- **Read-time presentation** moves the hiding of primary copies from the index writer into the read queries; every read that lists items must apply it. Pinned by the presentation tests.

## Open questions for later

1. **Publication proof.** The server reports success for each pushed ref; today the code does not take that as proof and lists refs again (`src/repository/transport/remote.rs:118-119`, "a transfer return is never proof"). Accepting the report would save one connection per push. Default: keep the verification listing.

## Left to the builder

- Column names, the migration name, and the basis encoding.
- How a confirmed lifecycle request carries its two observations between steps.
- Which inspections in `src/repository/remote/sync.rs` merge, as long as a head and status check stays before each local update and before the push.
- The ruling for each remaining reader of `refresh_required`, listed in the plan.

## Review record

| Round | Reviewers | Blocking | Important | Minor | Resolution |
| --- | --- | --- | --- | --- | --- |
| 1, draft `4e126a0b` | source and feasibility; contract, consistency and builder; adversarial on locking and freshness | 8 | 29 | 9 | Revision `47f43280`: full refresh scans outside the lease and stores by basis compare; primary indexed whole when it moves; per-context basis, dirty mark and indexed time; activity keeps today's values; every command in the table; crash, resume and lifecycle-child paths; connection counts 4 and 5; retry facts corrected; owner questions raised (answered as decisions 7–12). |
| 2, revision `47f43280` | scoped re-review with adversarial pass | 3 | 10 | 9 | This revision. Rows of a context depend only on that context and cross-context presentation is computed at read (B1); every `newer_than` caller uses the context's content hash and dirty mark (B2); R-3 no longer completes another operation's row, and rows owing only their hand-off do not block refresh, poll, retirement or other items' syncs (B3, I1); table completed (`context prepare`, poll with retirement, `remote history`, rebuild); other `refresh_required` writers ruled; relationship check and previews read moved contexts from disk; owed contexts recorded on the row; racy rule and dirty answer defined; publication binding check amended; no observation under the lease; named edits extended. The ticket's Acceptance recovery clause, which said "no recovery behaviour changes" though R-3 changes holds, is corrected to name R-3. Not yet re-reviewed. |
