# Fast writes: index what a write changed, publish with few connections, promote and close without repeated scans

Ticket `01M7RDVNZDCCVSMWHDY9503852` (vh-f3wts, which absorbed vh-z9ep3). Status: approved by the owner, 2026-11-19, after review rounds 1, 2 and 3. Base: ticket branch `b730e7d7` (main `790a71ed`).

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
10. **Per-item index holds** (R-3): a per-item index step holds only its item, and a crashed write whose commit landed does not block refresh and poll. (Accepted as "a full refresh … would finish that save's index step"; review round 2 changed the mechanism, not the outcome: see R-3. The owner confirmed the change: "leaving the record open until resumed is fine".)
11. **Pre-commit safety checks** that read other items (duplicate ID, branch checked out elsewhere) stay, as an explicit exception to decision 2, made cheap only if measured above about 100 ms. (Stated to the owner as the default; no objection.)
12. **The CLI keeps retrying close and promote** when their index step is pending (R-7); this replaces decision 5's earlier "don't retry" item, which rested on a wrong reading that saves are retried. (Stated to the owner as the default; no objection.)
13. **The design is approved** (owner, 2026-11-19: "approved, accept the exception, write the plan").
14. **The `context sync-local` exception is accepted**: a crashed `context sync-local` still blocks refresh and poll until resumed, because its row keeps repository scope; narrowing it is left to a separate ticket if it is ever needed.

15. **The implementation plan is approved** (owner, 2026-11-19: "approved, all recommended answers, subagent-driven, go ahead"), with its planning amendments P-1–P-5 (`docs/plans/2026-11-19-fast-writes-implementation.md`, "Planning amendments to the design"), which take precedence over this design where they differ: P-1 cleanup checks (one full check inside removal, one cheap check before the remote delete); P-2 reads keep today's newer-file staleness rule against the context's own `indexed_at`; P-3 a write's index step, and a sync's, observe outside the lease and store under it after a basis re-check (replacing "under the lease it holds for the write" in Phase 1 and R-3); P-4 a sync's hand-off keeps the `refresh` action with a per-item `synchronization-index-v1/` target; P-5 previews add no blockers.
16. **A primary sync's hand-off row is the primary's own item in Table B**; close, promotion and folder create index under whole-repository refresh rows and keep repository scope, a second exception to decision 10 beside decision 14.
17. **The measurement script may run `repo enable`, `repo identity-set`, `key generate` and the confirms of close and promote in its own scratch registry and replica**; the owner adds its throwaway public key to localhost's `authorized_keys` or names a key to import.
18. **The named edits E-1–E-11 are approved**, with the inventory the plan's Task 3 brings to its checkpoint still to be approved there. Execution is subagent-driven.

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
- A whole-repository refresh, a rebuild, and repository-scoped rows (`context_sync_local`, `retire_context`) keep their repository-wide hold; only per-item index steps and per-item hand-offs narrow (R-3). A Manyhands write that meets a running whole-repository refresh waits for it, as today, and the refresh is now short. A crashed `context sync-local` therefore still blocks refresh and poll until resumed: an explicit exception to decision 10.

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

- a primary item, with its comments and the primary's problems under that item's ticket and comment directories, is not presented while a valid active context holds the same item (the RFC's "replaces the primary presentation when active"); "valid" means a stored, valid context row whose worktree exists (`context_exists`, as today);
- an item held by the primary and by its valid active context is the normal state and is not stale; `effective_rows` and `effective_copy` (`src/repository/read/items.rs:1567`, `:1616`) are the one place that chooses the copy, and every reader goes through them, including those that read problems directly today (`index_problems`, the problem count of `list_repositories`, `repository_snapshot`, `stored_problems`);
- an invalid item context is stored with a distinct context kind (not `active`), with its own problems, and reads present those problems where they appear today; every reader of `contexts WHERE kind='active'` (for example `src/repository/remote/state.rs:2766`, `:2816`, `:7040`, `read/status.rs:929`, `local_sync/observe.rs:65`) is checked so that an invalid row cannot reach `stored_authoring_branch`;
- a primary whose configuration is invalid presents no item contexts, as today.

So creating, releasing, retiring, repairing or invalidating an item context changes only that context's rows, and the primary's rows change only when the primary's own sources change. The required outcomes are Table A under "Required outcomes".

**Basis, dirty mark and indexed time.** Each context row also records, through one named additive migration:

- its *basis*: the head OID, the HEAD's branch name, the worktree's registered path, the configuration blob OID, and for each canonical file the observation reads its path, size, inode, ctime and mtime (nanoseconds where the platform gives them) and content hash, plus the listing of each canonical directory it reads (a ticket's comments directory; for a document context, the `docs/` tree, which its observation reads for duplicate IDs; for the primary, every directory the root scan reads and its folders);
- `indexed_at`, taken before the observation starts (in nanoseconds);
- `dirty`, set before a write commits to the context and cleared by the transaction that indexes it.

A context row with no basis (written by an older build) is dirty. Each operation row records, before its commit, the context keys it owes indexing, so a resume knows them.

**Observing one context.** Observe it once: stat the canonical files and directories, read and parse, stat again. If the two stats differ (an editor or `git` wrote during the read), observe once more; if they still differ, keep the context's previous rows, mark it dirty, and answer as a race does today (`RetryRequired`, `busy`). A file whose mtime is within 2 s of the indexed time (coarse filesystems) is not trusted by stat alone: a later basis check compares its content hash and, when it matches, records the entry as clean so it is not hashed again. A directory's basis entry is its stat; a re-check re-lists only a directory whose stat changed or is racy. On Windows the standard library gives no inode or ctime, so a same-size edit that preserves mtime outside the racy window is not detected there until the content is next read; the test for it runs on Unix only.

**What each write indexes.** A write indexes, in one transaction and under the repository lease it holds for the write, the contexts it changed:

| Command | Contexts it indexes |
| --- | --- |
| `ticket save`, `document save`, `ticket slug-assign`, `comment add` (local checkpoint), `context sync-local`, `context resolve-local`, `conflict resolve`, `item republish`, `ticket repair`, `document repair`, comment hand-off repair | its own context; one that first creates the context (an item only on the primary) indexes the new context |
| `ticket create`, `document create`, `context prepare` | its new context |
| `document move` | its own context (created if the document is only on the primary) |
| `context release`, `context retire` | none; the removed context's rows are deleted |
| `operation abandon` | none: it touches no file, reference, index entry or worktree (`abandon.rs`), and leaves the dirty marks as they are, so the next refresh or resume indexes what the abandoned operation left |
| `ticket close`, `document promote` | the primary (main moved); the removed context's rows are deleted; any context its children changed (Phase 3) |
| `item sync`, `repo sync`, the sync half of `comment add` | each context it merged into or fast-forwarded (the primary when it merged remote main), under the lease of its local-update step; a sync that changed nothing locally takes the lease briefly and records its hand-off when the context's basis equals the stored one and it is not dirty, and otherwise indexes the context |
| `folder create` | the primary |
| `poll once` (with or without `--retire-completed`), `remote history`, `index refresh`, the desktop's start | a whole-repository refresh, below, plus the deletions of retired contexts |
| `index rebuild` | a rebuild (all contexts, today's two observations, in the new form) |
| `remote add`, `remote select`, `remote remove`, `repo enable`, `repo identity-set` | a whole-repository refresh, as today |

The other writers of the repository's `refresh_required` mark their contexts dirty instead: checkpoints (`checkpoint_owned_paths`, `src/repository.rs:3907`, `:3941`), closure, promotion, conflict resolve, local sync, retirement, folder create (`bridges/folder.rs:514`, the primary) and `persist_retry_problem` (`:6545`, the context it names). `refresh_required` stays for whole-repository facts (configuration, registration, remotes, a poll's mark). `index_state` (`src/repository/read/mod.rs:92`) reads the dirty marks as well as `refresh_required`, and the lifecycle's readers of `snapshot.refresh_required` (`bind/lifecycle.rs:1235`, `:1386`, `promotion/run.rs:1694`) read the dirty marks of the contexts they changed. Local sync's own indexer becomes the per-context index step of its context.

**The per-item index hold (R-3).** The required outcomes are Table B under "Required outcomes"; in short:

- A per-item row's index step keeps its item scope: in an index-owner state it is in the way of its own item only. Resume and retry run it under the lease, so only a crashed step sits in `indexing`.
- A row that owes only its hand-off (`owes_only_its_hand_off`, which covers the `indexing` and `index_invalidated` states) is not in the way of a whole-repository refresh, a poll, a retirement, another item's local sync, or another item's sync: they neither take it up nor change it. A whole-repository refresh indexes its contexts like any others (they are dirty); a retirement deletes the rows of the contexts it removes. Every check that today refuses behind any pending row applies this exemption: `require_no_pending_local` (`src/repository/recovery.rs:1119`), the `context sync-local` admission (`local_sync/record.rs:1017`), the retirement checks (`context_retirement/pass.rs:382`, `authority.rs:135`, `reentry.rs:144`, `record.rs:1734`), and the reservation's `busy` behind another remote row whose only debt is `index_pending` (`remote/reservation.rs:1309-1318`).
- A sync's index hand-off is a per-item row with its item's scope (not a whole-repository `refresh` row), so it is exempt in the same way; the comment publication binding's check (`recovery.rs:182-190`) accepts that row's action and target.
- The row is closed only by its own `operation resume` or a retry of its request, which index the row's recorded contexts only (never a whole-repository refresh under an item's ID), treat a recorded context that no longer exists as indexed, and complete the row. Retiring the row's own item is refused behind it, as today.
- A row still mid-write (its commit not yet observed) keeps today's rules: in the way of its own item and of whole-repository operations, waited on while live and `recovery_required` when stale (vh-bhepg).

**The whole-repository refresh.** `index refresh`, a poll's hand-off and the desktop's start, in two steps:

1. Outside the lease: list the contexts (the primary checkout and `.manyhands/worktrees/*`), compare each one's current basis with the stored one (stat; content hash for racy entries), and observe every context whose basis differs, that is dirty, that is new, or that holds a transient problem (inaccessible, retry-required; a permanent problem such as missing front matter does not force a re-read). Contexts that disappeared are noted.
2. Under the lease: list the contexts again and re-check the basis of each one observed in step 1 (stat; a re-list only of a directory whose stat changed; no reads); store those that still match; leave any that moved or appeared dirty; delete the rows of those that disappeared. No observation runs under the lease. If any context is left dirty, the refresh answers as a race does today (`RetryRequired`, `busy`), and the next refresh observes it. The repository's `refreshed_at` records the time step 1 began.

The refresh keeps its repository-wide index-owner row from before step 1 to the end of step 2, as today, so Manyhands writes wait for it (it is now short) and cannot change a context between the steps; the step-2 re-check guards against editors and `git` acting outside Manyhands. This follows the RFCs: the scan is outside the lease, and the lease covers the re-check and the store.

**Activity (R-1, R-2).** The value is today's: the newest commit on the context's first-parent history, from its head, whose diff against its first parent touches an owned path, or a newer uncommitted change. How it is found:

- *Item contexts:* walk the first-parent chain from the head with a diff restricted to the owned paths (a pathspec with pattern matching disabled, so paths compare exactly as `commit_touches_paths` does, mode-only changes included), stopping at the first match, usually the head or one or two merges of primary back; keep the 1024-commit window. Cache the commit time keyed by (head OID, owned-path set); the uncommitted part is judged at each observation, as today.
- *The primary:* keep a map from each Manyhands path to its newest touching first-parent commit, valid for a base head S. When the primary is indexed at head H: if S is an ancestor of H (`graph_descendant_of`) and on H's first-parent chain, diff only the commits from S to H, once each, and update the map; otherwise (a reset, amend, rebase, forced move, a fast-forward to a branch that merged main as second parent, or no map yet) rebuild the map with one walk that diffs each first-parent commit once. An item's activity is the newest entry over its current owned paths, so a path that leaves the set (an invalid comment) stops counting. The map covers every root path, including items an active context hides. The 1024-commit window is dropped for the primary (R-1).
- A rebuild uses the same functions.

**Freshness on read.** `index status` reports `stale` while the repository's `refresh_required` is set or any context is dirty; its `refreshed_at` is the start of the last whole-repository refresh. Every read that today compares a file with `refreshed_at` (`show` by ID and by path, the comment reads) instead compares the content hash of the file it reads anyway with the hash stored in its context's basis, and also reports stale when that context is dirty; no read lists a directory. The lifecycle's choice of `discovery` current or pending reads the dirty marks of the contexts it changed.

**Reads that decide on other items.** A save that sets `deps` or `parent` keeps vh-bhepg's rule (refused behind another item's row that is mid-write) and, before its relationship check, compares the basis of every context, the primary included (stat, a few hundred files); the tickets of any context whose basis moved are read from disk for the check, without writing the index. Lifecycle previews use the same rule for their facts about other items (Phase 3).

**Crash and retry.** A crash after a write's commit leaves its row owing its hand-off and its contexts dirty; `operation resume` or the same request indexes the recorded contexts (the RFC's "retried as indexing only"), and meanwhile refresh and poll are not blocked by it. A crash inside an index transaction stores nothing. A replay of a request whose row completed answers as today, with its commit.

**Smaller costs on the write path.** `first_parent_configuration_entries` (`src/repository.rs:11204`) stops at the first match; the discarded `repository_snapshot` at the end of a hand-off is skipped. The pre-commit safety checks stay (decision 11).

### Phase 2: publishing

- **One connection per remote call.** A remote call lists refs and transfers on the connection `connect_ssh` opened: the transfer runs inside its closure (git2's connection borrows the `Remote`) and reuses its callback attempt, so the transfer still submits the selected key exactly once to the finalized host and any reconnect inside libgit2 fails closed. The local rechecks before a transfer (`recheck_ssh`, `recheck_trust`) stay; a failed recheck refuses as today. There is no automatic reconnect on error.
- **Fewer listings.** The fetch lists and downloads on one connection and drops the "before" listing; the "after" listing, which carries the moved-remote check, stays on its own connection. The push-absence check stays before any local merge. The pre-push listing and the push share one connection. The verification listing after a push stays (Open question 1). Counts: 4 connections when nothing is merged (fetch, after, push, verify), 5 when main is merged into the item first (adding the absence check), against about 13–15 today.
- **Fewer inspections.** Back-to-back inspections of an unchanged worktree merge; the inspections inside the fetch window, which changes nothing local, go; context-tree validation is cached by tree OID together with the item kind, item ID, configuration blob and build, since a first edit's branch shares main's tree. A head and status check stays immediately before each local update and before the push.
- **No refresh after a push**, per the sync row of Phase 1's table.
- **`comment add`.** The comment receipt is proved once, not twice.

### Phase 3: promote and close

- **Two previews per attempt.** A confirmed request computes the preview once against the consent digest and once at integration, in `prepare_integration_for` (taken just before the lease at `src/repository/promotion/integrate.rs:119`, then revalidated under it and checked against the expectation at `:183`, keeping its re-confirmation path); `cleanup_only`, the decision after the context child and the decision before integration read from those two. A resumed attempt has no consent preview and re-checks under the lease. A preview reads the item's own facts from its context and the primary branch, and facts about other items (duplicate IDs, nonconforming content) from the index after the basis check of "Reads that decide on other items", so an outside change is read from disk.
- **One index update per exit.** The children record their index step as owed to the parent; on every exit (finish, abandon, cancel, a stop at awaiting publication, and the later finish of such a stop) the parent takes the lease and indexes the contexts its children changed and its own, per Phase 1's table.
- **One cleanup precheck**, under the lease immediately before removal, keeping every check `require_cleanup_clean` makes (`src/repository/cleanup.rs:536`).
- **Registry audits (R-5, decision 9).** Only registry open runs `migrate_registry`, and it audits the schema and the unfinished rows; `list_shared_keys` and the other callers that run after open in the same process take a read guard and do not migrate. The open's memo is keyed by the registry file's identity and `PRAGMA schema_version`, so a replaced file migrates again. A guard audits every row it scans, which for local-sync guards means every unfinished row, not only the rows that match its item; a completed record is audited when a read uses it.
- The SSH and inspection changes of Phase 2 apply to the children.

## Required outcomes (built test-first, with a design checkpoint)

Review rounds 1–3 each found problems in two mechanisms: which copy of an item is presented, and what a pending row holds. Per the planning process, their contract is stated as the outcomes below rather than rewritten again. The implementation plan's first tasks for each turn every row into a table-driven library test, written and failing before the code; the owner and a reviewer check those tests at a design checkpoint before the implementation continues. A row the code cannot meet as stated comes back to the owner, not around them.

**Table A: presentation across contexts.** "Lists" means every list, find, graph read (deps, children, plan, cycles, critical path), `show`, the comment reads, `index status` and `repo inspect`.

| Situation | Required outcome |
| --- | --- |
| Item only on the primary | Presented from the primary; not stale. |
| Item on the primary with a valid active context (first edit of a merged item) | Presented once, from the active context, with that context's comments and problems; the primary's copy, its comments and its problems under the item's directories are not presented; not stale. |
| The active context is released, retired, or its worktree removed outside Manyhands | The primary's copy is presented again at the next read after the context's rows are deleted (by the command, or by the next refresh for an outside removal). |
| An outside `git checkout` makes an item context invalid | After the next refresh: the context is stored with the invalid kind and its problems are presented where they are today; the primary's copy, if any, is presented; poll, remote snapshot and history reads are not refused because of that row. |
| Item only in an active context (new ticket or document) | Presented from the context. |
| Primary configuration invalid | No item contexts are presented, as today. |
| Relationships (deps, parent, children, cycles) and short codes | Resolved over the presented copies only; a hidden primary copy contributes no edges and no duplicate slug. |

**Table B: what a pending row holds.** "Writes" means any Manyhands mutation; "own item" means the item the row names.

| Pending row | Writes on its own item | Writes and syncs on other items | Whole-repository refresh, poll, rebuild | Retirement | Who closes it |
| --- | --- | --- | --- | --- | --- |
| A per-item write mid-write, live | wait, then `busy` (vh-bhepg) | proceed | wait, then `busy` | wait, then `busy` | its process |
| A per-item write mid-write, stale | `recovery_required` naming it | proceed | `recovery_required` naming it | `recovery_required` naming it | resume, or a confirmed abandon |
| A per-item write that owes only its hand-off, live or crashed (`indexing`, `index_invalidated`, or a committed write before indexing) | wait while live; `recovery_required` when stale | proceed | proceed, and index its dirty contexts | proceed for other items; refused for its own item | its own resume or retry, which indexes its recorded contexts and completes it |
| A sync whose only debt is its per-item index hand-off | as the row above | proceed | proceed, and index its dirty contexts | as the row above | its own resume or retry |
| A running whole-repository refresh | wait, then `busy` | wait, then `busy` | wait, then `busy` | wait, then `busy` | its process; it is short |
| A crashed whole-repository refresh, a rebuild, `context sync-local`, `retire_context` | as today | as today | as today | as today | as today |

## Rulings

Each is a builder's call recorded for review, with its cost if wrong.

- **R-1. Activity keeps today's value; the window is dropped for the primary only.** Decision 4 is met by short walks from each item's head and an incremental map for the primary, with one full walk only when the map cannot be advanced. An open item's value is the newest commit that changed the item, not the branch tip, because every share that finds main moved adds a "Merge primary into ticket" commit, and counting it would reorder the Updated view after each share. The primary's map has no 1024-commit window: an item whose newest change is more than 1024 first-parent commits old (none today, at 179) gets its true time instead of today's fallback. Cost if wrong: an ordering difference after 1024 commits.
- **R-2. A full walk is acceptable when the primary moves to a head that does not extend the map's base.** One diff per commit, after a reset or rewrite, not on a normal write. Cost if wrong: a slow first index after such a move.
- **R-3. Per-item index holds (decision 10, Table B).** A per-item index step is in the way of its own item only; a row that owes only its hand-off, a sync's hand-off included, does not block refresh, poll, retirement or other items' writes and syncs, which index its contexts (or delete the rows of contexts they remove) but never change its row; only its own resume or retry closes it. Round 2 replaced "a full refresh finishes that save's index step" with this, because finishing another operation's row needs a liveness judgement that only indexing rows have. The outcome the owner accepted holds: a crashed save whose commit landed does not block refresh or poll, and its item is indexed by them; its row stays open until resumed, which the owner confirmed. Exception: `context sync-local` keeps its repository-wide row.
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
  - Table A, one test row per table row, written first (design checkpoint);
  - Table B, one test row per cell, written first (design checkpoint), extending the `Scope` table in `src/repository/recovery_scope_tests.rs` and adding the exemption sites of R-3;
  - basis and observation: an editor write during the read (previous rows kept, context dirty), a same-second same-size edit, an mtime-preserving copy (Unix only), a branch switch at the same OID, a new comment file, a folder made outside Manyhands, a racy entry confirmed clean once and not hashed again;
  - the whole-repository refresh: observes only changed contexts (counted), picks up an outside commit and edit, reconciles a removed worktree, leaves dirty (and answers `busy`) a context that an outside `git` command changed between the two steps (a hook between the steps), runs no observation under the lease (a hook);
  - freshness on read: `dirty` set before commit and cleared by indexing; `index status`, list `data.index`, `show` by ID and path and the comment reads after a write, after an outside edit, and with a context row from an older build;
  - the hold, beyond Table B: a library test parks item A's index step on a hook and completes item B's write;
  - crash and retry: a crash after the commit (refresh and poll proceed and index the item; the row stays until resumed; resume completes it and indexes only its recorded contexts; a resume whose context was retired meanwhile completes), inside the index transaction, and after a lifecycle child on each parent exit; an abandon leaves the dirty marks;
  - relationship check and previews read a context whose basis moved from disk;
  - audits: a guard refuses a malformed unfinished row of another item; key listing runs no migration (counted); a confirmed close performs at most two previews and no whole-repository observation (a library counting hook).
- **Non-SSH integration**: the existing goldens show that answers are unchanged.
- **One SSH case** (existing remote target): a pushing `item sync` uses 4 connections and a merging one 5, counted with `accepted_sockets()` (`tests/support/ssh_remote.rs:549`). It needs SSH because connection reuse is transport behaviour.
- **Named edits** (the plan lists each with its change): in `tests/discovery_rebuild.rs`, `refresh_scan_race_retains_previous_rows_then_converges_on_retry`, `no_mutation_scan_race_retry_preserves_state_after_external_change`, `refresh_does_not_persist_private_markdown_while_detecting_source_races`, `refresh_retry_replaces_a_previously_persisted_context_when_its_observation_changes`, `refresh_retry_replaces_a_persisted_root_when_configuration_problem_changes`, `assert_active_worktree_race_preserves_rows`, `discovery_public_types_hold_metadata_only`; in `tests/read_items.rs`, `a_folder_made_while_a_refresh_observes_makes_the_refresh_try_again` and `refreshed_at_is_set_only_when_a_refresh_completes`; in `tests/read_status.rs`, `index_status_of_an_interrupted_refresh_is_stale_and_says_why` and `index_status_is_stale_when_the_index_holds_an_item_twice`; the cases that treat an item held by two contexts as stale (`comments_of_an_item_the_index_holds_twice_come_from_the_chosen_copy_and_are_stale` in `tests/read_comments.rs`, `an_item_the_index_holds_twice_is_listed_once_and_read_from_the_copy_that_is_there` and `a_ticket_the_index_holds_twice_shows_only_its_effective_rows_relationships` in `tests/read_items.rs`), which Table A replaces; in `tests/mutation_replay/reenter.rs`, `writers_on_two_tickets_both_complete_while_one_is_unfinished` (its `index refresh` no longer answers `recovery_required` for a row that owes only its hand-off) and any other case that asserts a whole-repository refusal behind such a row; the `Scope` table rows for item rows against whole-repository calls; the observation hooks in `src/repository/remote/sync_tests.rs` and `src/repository/comment_publication_tests.rs`; the reconnect checkpoints in `tests/ssh_transport/exact.rs` (`tracking_boundaries`, `reference_status_rejection`) and `tests/ssh_transport/synchronization.rs` (`cancel_before_transfer`); and the socket counts in `tests/ticket_closure/journeys.rs` and `tests/document_promotion/advanced.rs`.

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
| 2, revision `47f43280` | scoped re-review with adversarial pass | 3 | 10 | 9 | This revision. Rows of a context depend only on that context and cross-context presentation is computed at read (B1); every `newer_than` caller uses the context's content hash and dirty mark (B2); R-3 no longer completes another operation's row, and rows owing only their hand-off do not block refresh, poll, retirement or other items' syncs (B3, I1); table completed (`context prepare`, poll with retirement, `remote history`, rebuild); other `refresh_required` writers ruled; relationship check and previews read moved contexts from disk; owed contexts recorded on the row; racy rule and dirty answer defined; publication binding check amended; no observation under the lease; named edits extended. The ticket's Acceptance recovery clause, which said "no recovery behaviour changes" though R-3 changes holds, is corrected to name R-3. |
| 3, revision `a4f95c8f` | scoped re-review with adversarial pass | 1 | 5 | 7 | This revision, not independently reviewed. Presentation and holds, which drew findings in all three rounds, are stated as Tables A and B and built test-first with a design checkpoint. Fixes: `effective_rows`/`effective_copy` as the one place that chooses a copy, an item in its primary and active copies no longer stale, hidden primary problems, readers of problems named (N1); invalid contexts get their own kind and `contexts` readers are checked (N2); the whole refresh keeps its repository-wide row so Manyhands writes wait, and its step-2 re-check guards outside writers (N3); a sync's hand-off becomes a per-item row and every "any pending row" check takes the exemption (N4, N5); `operation abandon` indexes nothing and leaves dirty marks (N6); `context sync-local` named as an exception to decision 10 (M1); directory stats, racy entries confirmed once, the Windows limit (M2); previous rows kept on a twice-racing read (M3); the integration preview's lease position (M4); the relationship check covers the primary (M5); remaining `refresh_required` writers and readers named (M6); R-3 wording (M7). The owner confirmed R-3's open record. |
