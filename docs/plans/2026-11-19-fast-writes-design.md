# Fast writes: index what a write changed, publish with few connections, promote and close without repeated scans

Ticket `01M7RDVNZDCCVSMWHDY9503852` (vh-f3wts, which absorbed vh-z9ep3). Status: draft, revised after review round 1, awaiting the owner's answers and approval. Base: ticket branch `b730e7d7` (main `790a71ed`).

## Intent

A person expects a local save to be quick, checkpoint commit included; sharing an item or adding a comment is "just a push" and should be quick too; promotion and closing do more, but should be as fast as they can be. Today, on this repository (314 items, 57 ticket worktrees), a save takes about 36 s, `item sync` 55–75 s, `comment add` about 100 s and `ticket close` 150–880 s (median 263 s), and each grows with every commit and every item. Almost all of it is repeated whole-repository work: an index refresh after every write that re-reads every item and every worktree twice and walks history per item, about 13–15 SSH connections per publication, and about 12 whole-repository scans per close. This change makes each write cost about its own work.

## Decisions (owner, 2026-11-19, in the planning chat; quoted in the ticket comment `01M7TZGEBQMXNGF24H4DXMXDFN`)

1. **Targets**, on this repository with a release build: a local save (`ticket save`, `document save`, `ticket create`, a comment's local checkpoint) under 1 s; `item sync` and `comment add` 2–4 s; `document promote` and `ticket close` under 10 s.
2. **Each write is its own operation.** A write re-reads and re-indexes only what it changed: its item's branch, worktree, file, ticket or comment. It does not re-read another item's branch or worktree. ("I don't think it needs to care about any other worktrees/branches than the one with the file, ticket or comment being saved and committed.")
3. **The full refresh stays complete but cheap.** `index refresh`, poll and the desktop's start re-read only what changed since it was indexed.
4. **No full history walk for activity times.** The design's "full scan" meant rebuilding the SQLite index from the files, not walking Git history. Every open item has its own branch until it is merged; a merged item is frozen on main, and editing a merged document gives it a new branch and worktree.
5. **Publishing, promote and close**: reuse one SSH connection per remote call and drop repeated listings and inspections; no refresh after a push that changed nothing locally; a close's or promotion's preview computed twice instead of six times, with one index update; key listing without the registry migration and its audits; audits of the records in use only. (Decision 5 also listed "the CLI not retrying a request whose write succeeded"; see Ruling R-7.)
6. **One ticket, three measured phases**: the index first, then publishing, then promote and close.

## Goals

1. A write updates the index for the contexts it changed and nothing else, once, under the lease it holds for the write (Phase 1's command table).
2. A whole-repository refresh re-observes only item contexts whose basis changed, always re-observes the primary context cheaply, and gives the same complete result as today.
3. Every item's `activity_at` and `activity_source` are unchanged, except as Ruling R-1 states for history past 1024 first-parent commits.
4. A sharing sync opens at most 4 SSH connections when nothing needs merging and 5 when it merges main into the item, and runs no whole-repository refresh.
5. A confirmed `ticket close` or `document promote` runs no whole-repository discovery pass for its previews, at most two previews per attempt, and one index update.
6. Only registry open runs the migration; a guard audits the rows it scans, not every stored row.
7. The targets in Decision 1 are met and measured before and after each phase.

## Non-goals and anti-goals

- No change to codes, recovery actions or JSON shapes. The meaning of `discovery` on a write is clarified (Owner question 2).
- No weakening of a safety property: crash recovery and resume, detection of a remote that moved, refusal to recreate a deleted branch, publication proof, cleanup safety for ignored files and nested repositories, and refusal of malformed registry records before they are used.
- No file watcher, no background indexer and no deferred refresh.
- No change to SQLite durability settings.
- A whole-repository refresh keeps its repository-wide hold; only per-item index steps narrow (R-3).

## Facts checked

All at `b730e7d7`, by the author and by the round-1 source reviewer.

- **The refresh after every write is whole-repository.** `handoff_post_authoritative` (`src/repository.rs:4012`) claims the row for indexing, drops the lease (`:4033`) and runs `refresh_repository_claimed` (`:1512`) → `refresh_repository_observed` (`:1568`): `observe_root` before and after (`:1580`, `:1602`), every unchanged context stored from the first observation, `RetryRequired` for the first changed one (`:1656`), and a full `repository_snapshot` (`:1677`) that the hand-off discards. `persist_context_refresh` (`:6413`) deletes a context's rows and rewrites them all; for the primary it also rewrites configuration observations and folders.
- **Local sync has its own whole-repository indexer.** `local_sync::run::index` (`src/repository/local_sync/run.rs:886`) observes twice and persists the whole observation.
- **An indexing row blocks every other writer.** `is_in_the_way` (`src/repository/recovery.rs:2029`) puts a row in `INDEX_OWNER_STATES` in every call's way, "of whatever item: two index owners of one repository would each rewrite the whole index". vh-bhepg's per-item hold (`Scope`, `:1993`) otherwise lets writes on different items proceed. `operation resume` and `index refresh` re-run an index step as a whole-repository refresh (`src/repository/mutation/bind/resume.rs:1132`, `bind/index.rs:138`).
- **The primary depends on the set of active contexts.** A primary item is hidden while a valid active context holds it (`leave_active_items_to_their_contexts`, `src/repository/discovery.rs:776`); an invalid worktree's problems are stored on the primary (`observe_active_contexts`, `:403`).
- **Activity is a per-item history walk.** `latest_commit_touching` (`src/repository/discovery.rs:697`) loads up to 1024 first-parent commits from the context's HEAD (`MAX_ACTIVITY_HISTORY_COMMITS`, `:22`) and runs an unfiltered `diff_tree_to_tree` per commit until one touches the item's owned paths, which come from the working-tree sources. The root pass computes it for every root item, including those an active context then hides.
- **Activity is used only for order and age**: `ticket_list_order` (`src/repository/read/items.rs:945`), the CLI's `changed_at` and `change_source`, the desktop's "Updated" sort and age labels. It is not part of the observation token or the staleness check.
- **Freshness is per repository.** Every checkpoint sets the repository's `refresh_required` (`mark_checkpoint_refresh` → `mark_document_refresh_required`, `src/repository.rs:4002`, `:11284`; also closure, promotion, remote state, poll and retirement). Only `reconcile_disappeared_contexts` (`:6563`), rebuild and local sync clear it. Lifecycle results choose `discovery` current or pending from it (`src/repository/promotion/run.rs:1695`). Reads compare an item file's mtime with the repository's `refreshed_at` (`ItemFile::newer_than`, `src/repository/read/items.rs:1416`), in show only.
- **The CLI re-runs close and promote when their index step is pending.** `should_retry` (`src/bin/manyhands-cli/mutate.rs:100`) retries a `busy` or `partial` answer whose first recovery action is `request.retry`, three times (100, 200, 400 ms). Close and promote offer `request.retry` for `discovery_pending` (`src/repository/mutation/bind/lifecycle.rs:582`); folder create and poll offer it when retryable; saves, creates and `comment add` offer `operation.resume` (`src/repository/mutation/command.rs:566`, pinned by `a_hand_off_no_operation_owes_is_offered_as_a_refresh`, `outcome_tests.rs:1396`).
- **Every transfer reconnects.** `src/repository/transport/remote.rs:403-429` disconnects and calls `connect_auth` again before each transfer, so each transfer has exactly one submission of the selected key to a host equal to the finalized one. `connect_ssh` (`src/repository/transport/operation.rs:321`) makes a first connection for every remote call. A pushing sync makes about 13–15 connections.
- **The post-push refresh is whole-repository.** `synchronization_refresh` (`src/repository/remote/sync.rs:7081`) passes an empty matcher unless the outcome is `PublishPending`.
- **A confirmed close or promotion previews repeatedly.** `L::prepare` or `preview` at `src/repository/promotion/run.rs:492`, `:659`, `:679`, `:710` and `src/repository/promotion/integrate.rs:119`, after the boundary's prepare (`bind/lifecycle.rs:936`). Each runs `observe_root` (`src/repository/closure/observe.rs:300`, `src/repository/promotion/observe.rs:215`). Its two child syncs and its finish each run a whole-repository refresh.
- **Key listing migrates the registry.** `list_shared_keys` (`src/repository/keys/registry.rs:329`) takes the cache write guard and runs `migrate_registry`, which reaches `local_sync::record::migrate` (`src/repository/recovery.rs:724`) and its `audit` (`src/repository/local_sync/record.rs:469`), which runs `image_proof::audit` (`:485`) over every stored proof of every repository. Registry open already migrates and audits once (`src/repository.rs:4264-4267`). There are about 40 non-test callers of `migrate_registry`.
- **The RFCs forbid full scans under the lease**: index RFC, "Operation Coordination" ("full read-only scans remain outside the lease") and "Remote Polling" ("full scans MUST NOT hold the repository lease"); Git RFC, "Remote Coordination" ("full index scans MUST NOT hold it").

## Approaches

1. **Index what a write changed, and make the full refresh skip what did not move (chosen).** Writes stop paying for the repository; refresh, poll and rebuild stay complete; activity values and answers stay as they are.
2. **Keep one whole-repository refresh per write but make it cheap.** Rejected: it still couples writers (an index owner is in every call's way, and its observations fail when any other worktree changes) and still grows with the number of worktrees.
3. **Defer or background the refresh.** Rejected: the CLI has no resident process, the desktop worker would still be held, and the cost moves rather than goes.

Within publishing and promote/close there is one sensible approach each: remove repetition and keep every check that carries a property.

## Design

### Phase 1: the index

**Contexts and their basis.** The index already stores one row per context (the primary checkout, and each item worktree) with its `head_oid`. Each context row also records its *basis* and its *state*, added by one named additive migration:

- basis: the head OID, the HEAD's branch name, the worktree's registered path, the configuration blob OID, and a fingerprint of the context's canonical sources: for each canonical file the observation reads, its path, size, inode, ctime and mtime (nanoseconds where the platform gives them), plus the listing of each canonical directory it reads (a ticket's comments directory; for a document context, the `docs/` tree, which its observation already reads for duplicate IDs);
- `indexed_at`, taken before the observation starts (as `observed_from` is today);
- `dirty`, set before a write commits to the context and cleared by the transaction that indexes it.

A row with no basis (written by an older build) is dirty.

**Observing one context.** Observe it once: take the fingerprint, read and parse, take the fingerprint again. If the two fingerprints differ (an editor or `git` wrote during the read), observe once more; if they still differ, store the rows with the context left dirty. Any file whose mtime falls within the platform's timestamp granularity of `indexed_at` is not trusted by stat alone: the next refresh re-reads it, as Git does for racy-clean entries. The fingerprint is compared for equality, never by time order.

**What each write indexes.** A write indexes, in one transaction and while it holds the repository lease, the contexts it changed: its own context always; and the primary context when it moved main, when it created or removed a context whose item is on the primary, or when it changed whether a context with a primary copy is valid (the primary hides that item, or holds that context's problems). Indexing the primary re-observes all of it (cheap with incremental activity, below).

| Command | Contexts it indexes |
| --- | --- |
| `ticket save`, `document save`, `ticket slug-assign`, `comment add` (local checkpoint), `context sync-local`, `context resolve-local`, `conflict resolve`, `item republish`, comment hand-off repair | its own context |
| the same commands on an item that is only on the primary, which first creates its context | its new context and the primary |
| `ticket create`, `document create` | its new context |
| `document move` | its own context, and the primary if the document is on the primary |
| `ticket repair`, `document repair` | its own context and the primary |
| `context release`, `context retire`, `poll once --retire-completed`, `operation abandon` of an operation that created a context | the primary; the removed context's rows are deleted |
| `ticket close`, `document promote` | the primary; the removed context's rows are deleted; any context its children changed (see Phase 3) |
| `item sync`, `repo sync`, the sync half of `comment add` | the contexts it merged into or fast-forwarded (the primary when it merged remote main), under the lease of its local-update step; a sync that changed nothing locally records its hand-off only when the context's stored basis equals the current one and it is not dirty, and otherwise indexes the context |
| `folder create` | the primary |
| `poll once` | a whole-repository refresh, below |
| `remote add`, `remote select`, `remote remove`, `repo enable`, `repo identity-set` | unchanged: a whole-repository refresh, now cheap |

A save that sets `deps` or `parent` already reads other items (`CallReach::OtherItems`); it first brings the index up to date with the whole-repository refresh's basis check, so the relationship check sees what other contexts hold now.

**The per-item index step's hold (R-3).** A write's index step keeps its row's item scope: in an index-owner state, a per-item row is in the way of its own item and of whole-repository operations, not of other items. `operation resume`, and the CLI's retry of the same request, run a per-item row's index step for that row's contexts only, never a whole-repository refresh under an item's ID. A whole-repository refresh that meets a stale per-item row whose write is committed completes that row's index step as part of its own pass (it indexes that context anyway); a live row makes it wait as today.

**The whole-repository refresh.** `index refresh`, a poll's hand-off and the desktop's start, in two steps:

1. Outside the lease: list the contexts (the primary checkout and `.manyhands/worktrees/*`), compare each item context's current basis with the stored one, and observe the primary, every item context whose basis differs, is dirty, is new, or holds a transient problem (inaccessible, retry-required; a permanent problem such as missing front matter does not force a re-read). Contexts that disappeared are noted.
2. Under the lease: re-check each observed context's basis; store the ones that still match, observe once more the ones that moved (or leave them dirty), and reconcile the disappeared ones. The repository's `refreshed_at` records the time the refresh began.

This follows the RFCs (the scan is outside the lease; the lease covers the re-check and the store) and stops a refresh from storing over a newer per-item write: a context written after the refresh observed it no longer matches its basis.

**Activity (R-1, R-2).** The value is today's: the newest commit on the context's first-parent history, from its head, whose diff against its first parent touches an owned path, or a newer uncommitted change. How it is found:

- *Item contexts:* walk the first-parent chain from the head with a diff restricted to the owned paths (a pathspec, so identical to today's unfiltered diff filtered by path, including mode-only changes), stopping at the first match, usually the head or one or two merges of primary back; keep the 1024-commit window. Cache the commit time keyed by (head OID, owned-path set); the uncommitted part is judged at each observation, as today.
- *The primary:* keep a map from each Manyhands path to its newest touching first-parent commit, valid for a base head S. When the primary is indexed at head H: if S is on H's first-parent chain, diff only the commits from S to H, once each, and update the map; otherwise (a reset, amend, rebase, forced move, a fast-forward to a branch that merged main as second parent, or no map yet) rebuild the map with one walk that diffs each first-parent commit once. An item's activity is the newest entry over its current owned paths, so a path that leaves the set (an invalid comment) stops counting. The map covers every root path, hidden items included, so an item that becomes visible again has its value. The 1024-commit window is dropped for the primary (R-1).
- A rebuild uses the same functions.

**Freshness reported (Owner question 2).** A write's `discovery` `current` means the contexts that write changed are indexed and not dirty. `index status` reports `stale` while the repository's `refresh_required` is set or any context is dirty; its `refreshed_at` is the start of the last whole-repository refresh. `refresh_required` stays for whole-repository facts (configuration, registration, remotes, a poll's mark) and is no longer set by a checkpoint, which marks only its context dirty. Each reader of `refresh_required` is ruled in the implementation plan; the lifecycle's choice of `discovery` current or pending reads the dirty marks of the contexts it changed. `show` judges an item stale when its context is dirty or its file's stat differs from the stored fingerprint, statting only that file (the index RFC: "a read never lists a directory").

**Crash and retry.** A crash after a write's commit leaves its row at the index step, and `operation resume` or the same request indexes its contexts only (the RFC's "retried as indexing only"). A crash inside the index transaction stores nothing: the context stays dirty. A crash of a lifecycle parent after a child changed a context leaves the child's index step owed to the parent, and every exit of the parent (finish, abandon, cancel, a stop at awaiting publication) indexes the contexts its children changed.

**Smaller costs on the write path.** `first_parent_configuration_entries` (`src/repository.rs:11204`) stops at the first match; the discarded `repository_snapshot` at the end of a hand-off is skipped. The pre-commit safety checks that read every item in the item's own worktree or list all worktrees (`ticket_id_exists_at_a_different_path`, `canonical_context_at`, `branch_checked_out_in_another_worktree`) stay, made cheap if the phase's measurement shows them above about 100 ms (Owner question 5).

### Phase 2: publishing

- **One connection per remote call.** A remote call lists refs and transfers on the connection `connect_ssh` opened: the transfer runs inside its closure (git2's connection borrows the `Remote`) and reuses its callback attempt, so the transfer still submits the selected key exactly once to the finalized host and any reconnect inside libgit2 fails closed. The local rechecks before a transfer (`recheck_ssh`, `recheck_trust`) stay; a failed recheck refuses as today. There is no automatic reconnect on error.
- **Fewer listings.** The fetch lists and downloads on one connection and drops the "before" listing; the "after" listing, which carries the moved-remote check, stays on its own connection. The push-absence check stays before any local merge. The pre-push listing and the push share one connection. The verification listing after a push stays (Open question 1). Counts: 4 connections when nothing is merged (fetch, after, push, verify), 5 when main is merged into the item first (adding the absence check), against about 13 today.
- **Fewer inspections.** Back-to-back inspections of an unchanged worktree merge; the inspections inside the fetch window, which changes nothing local, go; context-tree validation is cached by tree OID together with the item kind, item ID, configuration blob and build, since a first edit's branch shares main's tree. A head and status check stays immediately before each local update and before the push.
- **No refresh after a push**, per the sync row of Phase 1's table.
- **`comment add`.** The comment receipt is proved once, not twice.

### Phase 3: promote and close

- **Two previews per attempt.** A confirmed request computes the preview once against the consent digest and once under the lease before integration (where `prepare_integration_for` already checks the expectation, `src/repository/promotion/integrate.rs:183`, and keeps its re-confirmation path); `cleanup_only`, the decision after the context child and the decision before integration read from those two. A resumed attempt has no consent preview and re-checks under the lease. A preview reads the item's own facts from its context and the primary branch, and facts about other items (duplicate IDs, nonconforming content) from the index, not from a whole-repository observation.
- **One index update.** The children record their index step as owed to the parent; the parent takes the lease and indexes once per Phase 1's table, on every exit.
- **One cleanup precheck**, under the lease immediately before removal, keeping every check `require_cleanup_clean` makes (`src/repository/cleanup.rs:536`).
- **Registry audits (R-5).** Only registry open runs `migrate_registry`; `list_shared_keys` and the other callers that run after open in the same process take a read guard and do not migrate. The open's memo is keyed by the registry file's identity and `PRAGMA schema_version`, so a replaced file migrates again. A guard audits every row it scans, which for local-sync guards means every unfinished row, not only the rows that match its item; completed rows are audited when a read uses them. What registry open audits is Owner question 3.
- The SSH and inspection changes of Phase 2 apply to the children.

## Rulings

Each is a builder's call recorded for review, with its cost if wrong.

- **R-1. Activity keeps today's value; the window is dropped for the primary only.** Decision 4 is met by short walks from each item's head and an incremental map for the primary, with one full walk only when the map cannot be advanced. An open item's value is the newest commit that changed the item, not the branch tip, because every share that finds main moved adds a "Merge primary into ticket" commit, and counting it would reorder the Updated view after each share. The primary's map has no 1024-commit window: an item whose newest change is more than 1024 first-parent commits old (none today, at 179) gets its true time instead of today's fallback. Cost if wrong: an ordering difference after 1024 commits.
- **R-2. A full walk is acceptable when the primary moves to a head that does not extend the map's base.** It is one diff per commit, bounded by history length, and happens after a reset or rewrite, not on a normal write. Cost if wrong: a slow first index after such a move.
- **R-3. A per-item index step holds only its item, and a whole-repository refresh completes a stale item's committed index step.** The reason vh-bhepg kept indexing repository-wide ("two index owners would each rewrite the whole index") no longer holds: a per-item step writes only its contexts, under the lease, and a whole-repository refresh stores only contexts whose basis still matches. This changes recovery semantics, so it is Owner question 4.
- **R-4. Freshness per context, with an additive migration.** Cost if wrong: a stale item not flagged. The migration is one-way (see Risks).
- **R-5. Audits by use.** The property kept: no malformed record is used as authority and absence is not concluded from a row that was not checked. Cost if wrong: a record corrupted inside a running process is not noticed until it is read.
- **R-6. The primary is re-observed whole whenever it is indexed.** It is one context, its observation is cheap once activity is incremental, and a partial primary writer would have to cover items, folders, problems attributed to item contexts, and configuration. Cost if wrong: the primary's parse cost on close, promote and first edits; measured in Phase 1.
- **R-7. The CLI keeps retrying close and promote when their index step is pending.** Decision 5's item came from an earlier, wrong reading that saves are retried; saves, creates and comments offer `operation.resume` and are not. After Phases 1 and 3 a retried close or promotion indexes its contexts only, so the retry is cheap and completes the hand-off without a person running `operation resume`. Owner question 6.

## Measurement

- **Before and after each phase**, on a replica of this repository (its history and its 57 worktrees, isolated data directory and HOME, release build): `ticket save`, `document save`, `ticket create`, `comment add` (its local checkpoint timed apart from its publication through the CLI's `progress:` lines), `item sync`, `document promote --prepare` and `--confirm`, `ticket close --prepare` and `--confirm`, `index refresh`, `poll once`. The publishing measurements use a local SSH server, so connection counts and handshakes are real. Each phase also takes a sampled profile of a save, a sync and a close and records that `latest_commit_touching`, a second observation and `image_proof::audit` no longer appear. The method is in the ticket ("How to measure"); the numbers go into the ticket at each phase.

## Tests

Per AGENTS.md's Testing Strategy: the cheapest layer, variations in table-driven tests, no real waits, named edits only.

- **Library unit tests:**
  - activity: a table of histories (direct commits, merges of primary into an item branch, comment additions and invalid comments, a mode-only change, a merged item on main, the primary advanced, reset and fast-forwarded to a second-parent merge, a rebuild) pinning today's `latest_commit_touching` values, with the number of commits diffed per case;
  - the command table: one row per command, asserting which contexts it indexed and that other contexts' rows and basis are untouched;
  - basis and observation: an editor write during the read, a same-second same-size edit, an mtime-preserving copy, a branch switch at the same OID, a new comment file, a folder made outside Manyhands;
  - the whole-repository refresh: re-observes only changed contexts (counted), always re-observes the primary, picks up an outside commit and edit, reconciles a removed worktree, does not store over a context a per-item write changed after the scan (a hook between the two steps), completes a stale committed item row;
  - freshness and reporting: `dirty` set before commit and cleared by indexing; `index status`, list `data.index` and `show` answers after a write, after an outside edit, and with a context row from an older build;
  - the hold: extend the `Scope` table (`src/repository/recovery_scope_tests.rs`) so an indexing or invalidated per-item row is out of another item's way and still in a whole-repository refresh's way; a library test parks item A's index step on a hook and completes item B's write;
  - crash and retry: a crash after the commit, inside the index transaction, and after a lifecycle child; `operation resume` indexes only the row's contexts;
  - previews and audits: a confirmed close performs at most two previews and no whole-repository observation (counted through a test hook in the library); a guard refuses a malformed unfinished row of another item; key listing runs no migration.
- **Non-SSH integration**: extend vh-bhepg's two-writer case (`writers_on_two_tickets_both_complete_while_one_is_unfinished`) rather than adding a timing-dependent one; the existing goldens show that answers are unchanged.
- **One SSH case** (existing remote target): a pushing `item sync` uses 4 connections and a merging one 5, counted with `accepted_sockets()` (`tests/support/ssh_remote.rs:549`). It needs SSH because connection reuse is transport behaviour.
- **Named edits** (the plan lists each with its change): in `tests/discovery_rebuild.rs`, the scan-race and retry cases (`refresh_scan_race_retains_previous_rows_then_converges_on_retry`, `no_mutation_scan_race_retry_preserves_state_after_external_change`, `refresh_does_not_persist_private_markdown_while_detecting_source_races`, `refresh_retry_replaces_a_previously_persisted_context_when_its_observation_changes`, `refresh_retry_replaces_a_persisted_root_when_configuration_problem_changes`, `assert_active_worktree_race_preserves_rows`, `discovery_public_types_hold_metadata_only`); in `tests/read_items.rs`, `a_folder_made_while_a_refresh_observes_makes_the_refresh_try_again` and `refreshed_at_is_set_only_when_a_refresh_completes`; in `tests/read_status.rs`, `index_status_of_an_interrupted_refresh_is_stale_and_says_why`; the observation hooks in `src/repository/remote/sync_tests.rs` and `comment_publication_tests.rs`; the reconnect checkpoints in `tests/ssh_transport/exact.rs` (`tracking_boundaries`, `reference_status_rejection`) and `tests/ssh_transport/synchronization.rs` (`cancel_before_transfer`); the socket counts in `tests/ticket_closure/journeys.rs` and `tests/document_promotion/advanced.rs`; and the `Scope` table rows for indexing item rows.

## Verification

Static gates across the tree and the fast tier, per AGENTS.md. Phase 2 changes `src/repository/transport/` and `src/repository/remote/`, so every SSH-harness target runs once at review-ready. Native CI runs after merge; connection reuse on Windows is the platform risk to watch there.

## Document amendments

- `docs/RFC/repository-index-persistence-and-refresh.md`: "Index-Only Refresh Algorithm" (per-context basis, write-scoped indexing, a whole refresh that re-observes changed contexts, incremental primary activity; "Wave 1 uses a full per-context scan" becomes history), "Operation Coordination and Recovery" (a write indexes under its lease; a whole refresh scans outside the lease and re-checks and stores under it; per-item index steps hold their item), freshness reporting.
- `docs/RFC/git-workflow-and-conflict-recovery.md`: the item sync and primary sync steps "Refresh discovery after the push is observed" become the Phase 1 sync rule; the count of `git-upload-pack` commands per sync.
- `docs/RFC/desktop-information-architecture-and-editor.md`: "Save … refreshes discovery" becomes "indexes the saved item"; the stale-scan wording.
- `docs/RFC/cli-contract.md`: the `effects` definition (`discovery` on a write, Owner question 2), the `index refresh` row, and the C5 known limits and follow-ups this change settles.
- Skills: `manyhands-finding-work` ("Is what I see current") and `manyhands-polling-and-index`: a write no longer refreshes other items; an outside commit or edit is seen by the next refresh, poll or desktop start, or flagged stale on read.
- The ticket's Acceptance (Owner question 1).

## Risks

- **One-way registry migration.** A build from this branch migrates the registry it opens, and older builds may then refuse it. Development and measurement use scratch registries; the first real use is after merge, from main, and the merge note says so.
- **A write that changes a context unseen.** Phase 1's table lists every command; a command missing from it, or a new one, leaves rows stale until the next refresh. Each row has a test, and the per-context `dirty` mark and the read-time stat make a miss visible as stale rather than wrong.
- **Outside changes are seen later.** A commit or edit made outside Manyhands in item B is no longer picked up by a save of item A; it is seen by the next refresh, poll or desktop start, or flagged stale on read. This is Decision 2's intent.
- **Primary cost.** R-6 re-observes the whole primary on close, promote and first edits; if Phase 1's measurement puts it above about 300 ms, the plan adds a partial primary writer.

## Owner questions

1. **The ticket's Acceptance says "no recovery or index behaviour changes".** The index's behaviour changes as designed: per-context freshness, writes that no longer refresh other items, and outside changes seen at the next refresh. Recommended: amend it to "no recovery behaviour changes, and the index gives today's results except that a change made outside Manyhands to another item is seen at the next refresh, poll or desktop start, or flagged stale when read".
2. **`discovery` `current` on a write.** The contract never said whole repository, but a change of a value's meaning needs a new schema version (`docs/RFC/cli-contract.md`, "Removing, renaming or retyping a field, or changing the meaning of an existing value"). Recommended: record it as a clarification with no schema change, because every value and code stays the same and only outside changes to other items are affected.
3. **What registry open audits.** Recommended: at open, audit the schema and the unfinished rows only; a completed record is audited when a read uses it. Otherwise every CLI command still audits every stored proof once, and the ticket's acceptance that a profile no longer shows the audit fails.
4. **R-3, the per-item index hold.** Recommended: accept it; it is what lets two writers on different items run without waiting on each other's indexing.
5. **Pre-commit safety checks.** The duplicate-ID and branch-checked-out-elsewhere checks read other items. Recommended: keep them as safety reads, an explicit exception to Decision 2, and make them cheap only if measured above about 100 ms.
6. **R-7, the CLI retry.** Recommended: keep it, since only close and promote use it and it becomes cheap.

## Open questions for later

1. **Publication proof.** The server reports success for each pushed ref; today the code does not take that as proof and lists refs again (`src/repository/transport/remote.rs:118-119`, "a transfer return is never proof"). Accepting the report would save one connection per push. Default: keep the verification listing.

## Left to the builder

- Column names, the migration name, and the fingerprint's encoding.
- How a confirmed lifecycle request carries its two observations between steps.
- Which inspections in `src/repository/remote/sync.rs` merge, as long as a head and status check stays before each local update and before the push.
- The ruling for each reader of `refresh_required`, listed in the plan.

## Review record

| Round | Reviewers | Blocking | Important | Minor | Resolution |
| --- | --- | --- | --- | --- | --- |
| 1, draft `4e126a0b` | source and feasibility; contract, consistency and builder; adversarial on locking and freshness | 8 | 29 | 9 | This revision. The full refresh scans outside the lease and stores by basis compare; the primary is indexed whole when it moves or its hidden set changes; per-context basis, `dirty` and `indexed_at` replace the repository flag for items; activity keeps today's values with (head, paths) caching and a first-parent-checked incremental map; every command is in the table; crash, resume and lifecycle-child paths are specified; connection counts corrected to 4 and 5; R-7 corrected (close and promote, not saves); audits scan every unfinished row; owner questions 1–6 raised. Not yet re-reviewed. |
