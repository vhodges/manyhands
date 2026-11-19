# Fast writes: index only what a write touched, publish with few connections, promote and close without repeated scans

Ticket `01M7RDVNZDCCVSMWHDY9503852` (vh-f3wts, which absorbed vh-z9ep3). Status: draft, awaiting the owner's approval. Base: ticket branch `b730e7d7` (main `790a71ed`).

## Intent

A person expects a local save to be quick, checkpoint commit included; sharing an item or adding a comment is "just a push" and should be quick too; promotion and closing do more, but should be as fast as they can be. Today, on this repository (314 items, 57 ticket worktrees), a save takes about 36 s, `item sync` 55–75 s, `comment add` about 100 s and `ticket close` 150–880 s (median 263 s), and every one of them grows with each commit and each item. The cost is almost entirely repeated whole-repository work: an index refresh after every write that re-reads every item and every worktree twice, about 15 SSH connections per publication, and about 12 whole-repository scans per close. This change makes each write cost about its own work.

## Decisions (owner, 2026-11-19, in the planning chat; quoted in the ticket)

1. **Targets**, on this repository with a release build: a local save (`ticket save`, `document save`, `ticket create`, a comment's local checkpoint) under 1 s; `item sync` and `comment add` 2–4 s; `document promote` and `ticket close` under 10 s.
2. **Each write is its own operation.** A write re-reads and re-indexes only the item it wrote: its branch, its worktree, its file, ticket or comment. It never looks at another item's branch or worktree. ("I don't think it needs to care about any other worktrees/branches than the one with the file, ticket or comment being saved and committed.")
3. **The full refresh stays complete but cheap.** `index refresh`, poll and the desktop's start re-read only the branches whose tip or Manyhands files changed since they were indexed.
4. **No history walk for activity times.** The design's "full scan" meant rebuilding the SQLite index from the files, not walking Git history. Every open item has its own branch until it is merged; a merged item is frozen on main, and editing a merged document gives it a new branch and worktree.
5. **Publishing, promote and close, and the CLI**: reuse one SSH connection per remote call and drop repeated listings and inspections; skip the refresh after a push that changed nothing locally; compute a close's or promotion's preview twice instead of six times, with one index update at the end; stop key listing from running the registry migration and its audits; audit only the records in use.
6. **One ticket, three measured phases**: the index first, then publishing, then promote and close.

## Goals

1. Every write that commits to one item's branch (save, create, comment, slug assign, repair, local sync, a sync that merges or fast-forwards) updates the index for that item's context only, once, while it holds the item.
2. A whole-repository refresh re-reads only the contexts whose head or Manyhands files changed, and keeps today's complete result.
3. Every item's `activity_at` and `activity_source` are unchanged (Ruling R-1).
4. `item sync` and the publication half of `comment add` open at most 4 SSH connections when they push, and run no whole-repository refresh.
5. A confirmed `ticket close` or `document promote` runs no whole-repository discovery pass and at most two previews.
6. Registry key listing runs no migration; the local-sync audits check only what the operation reads.
7. The targets in Decision 1 are met and measured before and after each phase.

## Non-goals and anti-goals

- No change to what a command answers: codes, effects, recovery actions and JSON shapes stay as the CLI contract defines them, except the clarified meaning of `discovery` in Document Amendments.
- No weakening of a safety property: crash recovery and resume, detection of a remote that moved, refusal to recreate a deleted branch, publication proof, cleanup safety for ignored files and nested repositories, and detection of tampered registry records at open.
- No file watcher, no background indexing process and no deferred refresh: the CLI has no resident process, and the desktop's single worker would still pay for it.
- No change to the SQLite durability settings.
- No concurrency beyond what vh-bhepg's per-item hold allows; this change lets that hold work as designed.

## Facts checked

All at `b730e7d7`.

- **The refresh after every write is whole-repository.** `handoff_post_authoritative` (`src/repository.rs:4012`) claims the row for indexing, drops the lease (`:4033`) and runs `refresh_repository_claimed` (`:1512`) → `refresh_repository_observed` (`:1568`), which calls `observe_root` before and after (`:1580`, `:1602`), stores every unchanged context from the first observation (`:1625-1655`), and answers `RetryRequired` for the first changed one (`:1656-1667`). It then reads a full `repository_snapshot` (`:1677`) that the hand-off discards.
- **An indexing row blocks every other writer.** `is_in_the_way` (`src/repository/recovery.rs:2029`) puts a row in `INDEX_OWNER_STATES` in every call's way, "of whatever item: two index owners of one repository would each rewrite the whole index". vh-bhepg's per-item hold (`Scope`, `:1993`) otherwise lets writes on different items proceed.
- **Activity is a per-item history walk.** `latest_commit_touching` (`src/repository/discovery.rs:697`) loads up to 1024 first-parent commits from the context's HEAD and runs an unfiltered `diff_tree_to_tree` per commit until one touches the item's paths (`:723-743`). The root pass computes it for every root item, including those an active context then replaces.
- **Activity is used only for order and age.** `ticket_list_order` (`src/repository/read/items.rs:945`), the CLI's `changed_at` and `change_source`, and the desktop's "Updated" sort and age labels. It is not part of the observation token or the staleness check.
- **Staleness is per repository.** Reads compare an item file's mtime with the repository's `refreshed_at` (`ItemFile::newer_than`, `src/repository/read/items.rs:1416`) and read the repository's `refresh_required` (`:572`). `contexts.head_oid` is stored but not used to skip work.
- **The CLI re-runs a request whose index step is pending.** `should_retry` (`src/bin/manyhands-cli/mutate.rs:100`) retries a `busy` or `partial` answer whose first recovery action is `request.retry`, three times (100, 200, 400 ms). Saves and creates offer it for `discovery_pending` (`src/repository/mutation/outcome.rs:800-806`, `command.rs:566`), and so do close and promote (`bind/lifecycle.rs:582`); `comment add` offers `operation.resume`. A re-run does the index step again, not the write (RFC "A completed commit with a pending index refresh is retried as indexing only"). It happens only when the two observations differ.
- **Every transfer reconnects.** `src/repository/transport/remote.rs:403-429` disconnects and calls `connect_auth` again before each transfer ("A disconnected remote never inherits a prior connection's authentication evidence"). A pushing sync makes about 15 connections, from the agent's count of `with_synchronization_remote` calls in `src/repository/remote/sync.rs:5899-6822`.
- **The post-push refresh is whole-repository.** `synchronization_refresh` (`src/repository/remote/sync.rs:7081`) passes an empty matcher unless the outcome is `PublishPending`, and calls `refresh_repository_target`.
- **A confirmed close or promotion previews repeatedly.** `run_lifecycle` calls `L::prepare` or `preview` at `src/repository/promotion/run.rs:492`, `:659`, `:679`, `:710`, and `prepare_integration_for` again at `src/repository/promotion/integrate.rs:119`, after the boundary's own prepare (`src/repository/mutation/bind/lifecycle.rs:936`). Each preview runs `observe_root` (`src/repository/closure/observe.rs:300`, `src/repository/promotion/observe.rs:215`). Its two child syncs and its finish each run a whole-repository refresh.
- **Key listing migrates the registry.** `list_shared_keys` (`src/repository/keys/registry.rs:329`) takes the cache write guard and runs `migrate_registry`, which reaches `local_sync::record::migrate` (`src/repository/recovery.rs:724`) and its `audit` (`src/repository/local_sync/record.rs:259`), which runs `image_proof::audit` (`:485`) over every stored proof of every repository. Key listing runs several times per SSH action.
- **The RFC chose a full scan for Wave 1 only.** `docs/RFC/repository-index-persistence-and-refresh.md`, "Index-Only Refresh Algorithm": "Wave 1 uses a full per-context scan rather than Git-diff incremental indexing", and each context's rows are replaced in one transaction.

## Approaches

1. **Index only what a write touched, and make the full refresh skip what did not move (chosen).** Writes stop paying for the repository; refresh, poll and rebuild stay complete. Activity values and every answer stay as they are.
2. **Keep one whole-repository refresh per write but make it cheap.** Rejected: it still couples writers (an index owner is in every call's way, and its before and after observations fail when any other worktree changes), and it still grows with the number of worktrees.
3. **Defer or background the refresh.** Rejected: the CLI has no resident process, the desktop worker would still be held, and the cost is moved rather than removed.

Within publishing and promote/close there is one sensible approach each: remove repetition and keep every check that carries a property.

## Design

### Phase 1: the index

**What a write indexes.** Each write updates the index rows of the contexts it changed, in one transaction, while it still holds the item (its per-item row and the repository lease it already took for the write; the lease is no longer dropped before indexing):

| Write | Contexts it updates |
| --- | --- |
| save, create, slug assign, repair, comment checkpoint, local sync, a sync that merged or fast-forwarded | the item's own context |
| the first edit of a merged item, which creates its context | the new context, and the primary copy of that item (hidden while active, as today) |
| close, promote | the primary context's rows for the merged item, and the removal of the item's context |
| a sync that changed nothing locally (push or already current) | none; it records the index hand-off once it has checked that the stored `head_oid` equals the pushed head |

The update observes the context once, under the lease: no second observation, because no other local writer can change this item's context while the write holds it. The row stays per-item (`Scope::Item`), and an index-owner state on a per-item row is in the way of that item only; whole-repository refresh and rebuild rows keep repository scope.

**Per-context freshness.** Each `contexts` row records when it was indexed and a fingerprint of its Manyhands files (path, size and modification time of the item file and its comments, or the document file), next to the `head_oid` it already stores; a named additive migration adds the columns. A read judges an item stale against its own context's indexed time and fingerprint instead of the repository's `refreshed_at`. The repository's `refresh_required` and `refreshed_at` remain for whole-repository facts (configuration, registration, a context that appeared or disappeared outside Manyhands).

**The full refresh.** `index refresh`, a poll's hand-off and the desktop's start list the contexts (the primary checkout and `.manyhands/worktrees/*`), compare each one's head OID and file fingerprint with what is stored, and re-observe only those that differ, that are new, or that have a problem row; contexts that disappeared are reconciled as today. It takes the repository lease for its observe-and-store, as `rebuild_repository` already does, and observes once. `index rebuild` keeps its behaviour and benefits from the cheaper activity computation.

**Activity (Ruling R-1).** For a context, the activity time is the newest commit on its first-parent history, from its head, whose tree differs from its first parent's at the item's owned paths; the same value as today. It is found by comparing the object IDs of those path entries in each commit and its parent, not by diffing whole trees, and it stops at the first match, usually the head or one or two merges back. It is cached by the context's `head_oid`: an unchanged head reuses the stored value. For the primary context, the activity of every root item is maintained incrementally: when main's head moves from a stored OID to a descendant, only the new first-parent commits are diffed, once each, and mapped to the items they touched; with no stored OID (a rebuild), one walk diffs each first-parent commit once and gives every item its newest touch. Root items that an active context replaces are not computed. Uncommitted activity stays as today, judged for the observed context only.

**Smaller costs on the write path.** `first_parent_configuration_entries` (`src/repository.rs:11204`) stops at the first matching entry instead of collecting all of history; the `repository_snapshot` read at the end of a hand-off is skipped when the caller discards it. Checks before the commit that read every item (`ticket_id_exists_at_a_different_path`, the whole-root `canonical_context_at`, the worktree enumeration) are left as they are unless the phase's measurement shows them above about 100 ms.

**The CLI retry.** Ruling R-6 below.

### Phase 2: publishing

- **One connection per remote call.** A remote call lists refs and transfers on the same authenticated connection: the disconnect and reconnect before each transfer goes, while the local rechecks before a transfer (`recheck_ssh`, `recheck_trust`) stay. Authentication evidence still comes from the connection that transfers.
- **Fewer listings.** The fetch lists and downloads on one connection and drops the "before" listing; the "after" listing, which carries the moved-remote check, stays. The push-absence check and the pre-push listing share one listing when nothing local changed between them. The verification listing after a push stays (Open question 1).
- **Fewer inspections.** Back-to-back inspections of an unchanged worktree are merged, the inspections inside the fetch window (which changes nothing local) are dropped, and context-tree validation is cached by tree OID, since a tree is immutable. A head and status check stays immediately before each local update and before the push.
- **No refresh after a push.** The sync records its index hand-off as in Phase 1's table: an index update of the synced context when it merged or fast-forwarded, otherwise only the hand-off row after checking the stored head.
- **`comment add`.** The comment receipt is proved once, not twice. Its local checkpoint indexes per Phase 1, and its publication follows the rules above.

Target: at most 4 connections for a pushing sync (connect and fetch, push, verify, and the deletion check when it cannot share).

### Phase 3: promote and close

- **Two previews.** A confirmed request computes the preview once to check the consent digest and once under the lease before integration; the decisions that today take further previews (`cleanup_only`, after the context child, before integration) read from those two. A preview reads the item's own facts from its context and the primary branch, not a whole-repository discovery pass.
- **One index update.** The child syncs of a close or promotion record their hand-off without refreshing; the parent updates the index once at the end, per Phase 1's table.
- **One cleanup precheck** under the lease immediately before removal, keeping every check `require_cleanup_clean` makes (`src/repository/cleanup.rs:536`).
- **Registry audits.** `list_shared_keys` and other reads stop calling `migrate_registry`: the migration runs once per process when the registry is opened, as service open already does. `image_proof::audit` and the other local-sync audits check the schema once per process (again if `PRAGMA schema_version` changes) and, at a guard or safe point, only the rows of the operations the caller reads. Tampered or malformed rows of other operations are still found when the registry is opened and when those rows are used (Ruling R-5).
- The SSH and inspection changes of Phase 2 apply to the children.

## Rulings

Each is a builder's call recorded for review, with its cost if wrong.

- **R-1. Activity keeps today's value.** Decision 4 said an open item's time comes from its own branch. The value is the newest commit on that branch that changed the item, not the branch tip, because every share that finds main moved adds a "Merge primary into ticket" commit, and counting that as activity would reorder the Updated view after each share. Cost if wrong: one comparison instead of a lookup at the tip; trivially changed later.
- **R-2. A per-item indexing row holds only its item.** Its update writes only its contexts' rows in one transaction, so two writers on different items cannot lose each other's rows. Whole-repository refresh and rebuild stay in every call's way. Cost if wrong: a write's rows overwritten by a concurrent whole-repository refresh; mitigated because a full refresh takes the lease and stores each context in its own transaction from one observation.
- **R-3. A write indexes under the lease it already holds.** The update is milliseconds; holding the lease removes the torn-read risk that the second observation guards against today. Cost if wrong: another local writer waits a few milliseconds longer for the lease (250 ms wait budget).
- **R-4. Freshness per context, with an additive registry migration.** Cost if wrong: a stale item not flagged. The migration is one-way (see Risks).
- **R-5. Audits run once per process at open, and per operation at guards.** The property kept is that no malformed record is used as authority; old completed records are checked when the registry is opened, not at every safe point of every operation. Cost if wrong: a record tampered with during a running process is not noticed until it is used or the next process opens the registry.
- **R-6. The CLI keeps retrying a pending index step.** This revises the "other parts" item that said not to retry: after Phase 1 a re-run does only the item's index step, which takes milliseconds, and it brings `discovery_pending` to `current` without a person running `operation resume`. Removing it would leave more requests pending. Owner question 2 below.

## Measurement

- **Before and after each phase**, on a replica of this repository (its history and its 57 worktrees, isolated data directory and HOME, release build): `ticket save`, `document save`, `ticket create`, `comment add`, `item sync`, `document promote --prepare` and `--confirm`, `ticket close --prepare` and `--confirm`, `index refresh`, `poll once`. The method is in the ticket ("How to measure"). The numbers go into the ticket at each phase.
- The replica's remote is a local SSH server for the publishing measurements, so connection counts and handshake cost are real.

## Tests

Per AGENTS.md's Testing Strategy: the cheapest layer, variations in table-driven tests, no real waits.

- **Library unit tests:**
  - activity: a table of histories (direct commits, merges of primary into the item branch, comment additions, a merged item on main, a rebuild) pinning that `activity_at` and `activity_source` equal today's `latest_commit_touching`, and a count of commits examined per context;
  - a write updates only its contexts: after a save of item A with item B's worktree changed on disk, B's rows and freshness are untouched and A's are current;
  - a full refresh re-observes only changed contexts (counted), picks up an outside commit and an outside file edit, and reconciles a removed worktree;
  - freshness: a read flags an item stale after an outside edit to its file, and not after a write to another item;
  - the per-item indexing row is in the way of its own item only; whole-repository refresh rows stay in every call's way (extend the existing `Scope` table test);
  - audits: a malformed proof row is refused when opened and when its operation reads it; key listing runs no migration (counted).
- **Non-SSH integration** (existing targets, `file://` remote): one golden for each command's answer staying the same is already in place; add a case that two `ticket save` processes on different items both complete without `busy`, and that a confirmed close performs no whole-repository observation (a counting hook).
- **One SSH case** (existing remote target): a pushing `item sync` opens at most 4 connections, counted with the test server's accepted connections. It needs SSH because connection reuse is transport behaviour.
- Existing tests that pin the double observation, the `RetryRequired` problem row or repository-wide indexing scope are named edits in the implementation plan.

## Document amendments

- `docs/RFC/repository-index-persistence-and-refresh.md`, "Index-Only Refresh Algorithm" and "Operation Coordination and Recovery": a write indexes the contexts it changed, under the lease; a whole refresh re-observes changed contexts only; freshness is per context; activity's meaning is unchanged.
- `docs/RFC/cli-contract.md`, the `effects` definition: `discovery` `current` on a write means the contexts that write changed are indexed; `index status` reports the repository's.
- Skills that describe the index (`manyhands-finding-work`, "Is what I see current", and `manyhands-polling-and-index`): a write no longer refreshes other items; an outside commit or edit is seen by the next refresh or poll, or flagged stale on read.

## Risks

- **One-way registry migration.** A build from this branch migrates the registry it opens, and older builds may then refuse it. Development and measurement use scratch registries; the first real use is after merge, from main, and the merge note says so.
- **A write that changes another context unseen.** The table in Phase 1 lists every write's contexts; a write missing from it leaves rows stale until the next refresh. Mitigated by the per-context freshness check on read and a test per row of the table.
- **Outside changes are seen later.** A commit made with `git` or an edit made outside Manyhands in item B is no longer picked up by a save of item A; it is seen by the next refresh, poll or desktop start, or flagged stale on read. This is Decision 2's intent.
- **Connection reuse and server limits.** A Git server may close an idle connection between listing and transfer; the transport falls back to one reconnect on that error.

## Open questions for later

1. **Publication proof.** The server reports success for each pushed ref; today the code does not take that as proof and reconnects to list refs (`remote.rs:118-119`, "a transfer return is never proof"). Accepting the report would save one connection per push. Default: keep the verification listing.
2. **The CLI retry (R-6).** Keep it (recommended) or remove it as first agreed.

## Left to the builder

- Column names, the migration name and the fingerprint's exact form.
- How a confirmed lifecycle request carries its two observations between steps.
- Which inspections in `src/repository/remote/sync.rs` merge, as long as a head and status check stays before each local update and before the push.
- Whether the pre-commit whole-repository checks need work, by measurement.
