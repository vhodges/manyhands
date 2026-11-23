---
slug: vh-f3wts
id: 01M7RDVNZDCCVSMWHDY9503852
title: Saves, comments, promote and close take 20 s to 5 min re-walking Git history for every item
type: defect
status: review_ready
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Local writes are slow and getting slower. On this repository (314 items, 172 first-parent commits on main, 57 ticket worktrees), `ticket save` and `document save` take 20–23 s and `comment add` takes 60–114 s, while reads take about 70 ms. The registry's `request_records` show `ticket save` at 6–7 s on 2026-10-25 and at 20–41 s since 2026-11-04; `ticket create` has a median of 20 s, `poll once` about 29 s and `ticket close` 176–301 s. On 2026-10-31 C5 measured release-build `document promote` and `ticket close` at about 35 s each and every prepare at about 2 s (C5 decision 31, ledger "Task 11: real content"). The desktop runs the same library calls on its single mutation worker, so a desktop save takes as long before the pill reads "Saved locally", and can also wait behind a poll's own index refresh.

This ticket absorbed vh-z9ep3 (01M6K04KGRHJC870JQK9XEKTXG) on 2026-11-19. That ticket covered promote, close and prepare and named the same per-item history walk, so the two are one fix. Its image-proof audit, its concurrency notes and its acceptance are merged in below.

## Measured cause (2026-11-18)

Measured on a replica with this repository's history and its 57 ticket worktrees (isolated data directory and HOME, release build, gdb stack sampling). A `ticket save` took about 23 s, of which 22.5 s was CPU, so lock, fsync and network waits do not matter here. About 98% of the samples were in the index refresh that follows the commit. Line numbers below are as of main `e91037d5` (2026-11-19).

- Every committing save ends in `handoff_post_authoritative` (`src/repository.rs:4012`) → `refresh_repository_observed` (`src/repository.rs:1568`), which calls `observe_root` twice, before and after (`:1580`, `:1602`), and compares the two observations.
- Each pass runs `observe_items`. For every item, `latest_commit_touching` (`src/repository/discovery.rs:697`) walks main's first-parent history and runs an unfiltered `diff_tree_to_tree` per commit (`commit_touches_paths`, `:723`) until one touches the item's file or its comments. Here that is about 21,500 whole-tree diffs per pass and about 43,000 per save, with nothing reused across items or passes.
- Each pass also re-observes every active ticket worktree: about 6 s for 57 (a save took 17 s with one worktree and 23 s with all 57).
- `comment add` runs that refresh after its local checkpoint (about 29 s together with its own scans) and a second full refresh after publication (`synchronization_refresh`, `src/repository/remote/sync.rs:7081`). Subtracting both from a real 62 s run leaves an estimated 10 s for SSH and sync checks, so the two refreshes are most of a comment's time.

The cost scales with items × history depth (capped at 1024 commits per item, `MAX_ACTIVITY_HISTORY_COMMITS`) and with worktree count, so every commit and every new item slows every save. Tree diffs per pass: about 2.9k on 2026-10-25, 12.7k on 2026-11-04 and 21.5k on 2026-11-18.

## Second hot spot: the image-proof audit (from vh-z9ep3)

C5's sampled profile of promote and close named a second hot spot: `local_sync::image_proof::audit` (`src/repository/local_sync/image_proof.rs:237`, reached through `local_sync::record::audit`) decodes every stored local-sync image proof at each safe point and on every registry open, so its cost grows with each stored sync. It is unchanged on main and has many callers (status reads, local-sync observe, control, request and recover). The 2026-11-18 save profile put about 98% in the index refresh, so the audit is not where a save spends its time; re-measure it on promote and close before deciding how far to go. vh-44q1k item (3) pays the same audit in each `repository_operations` run, so a fix here shrinks that one too.

## Smaller costs seen in code traces, not dominant today

- `first_parent_configuration_entries` walks the whole first-parent history, with no cap, on every save (`src/repository.rs:11204`).
- `changed_contexts` compares the two observations with `format!("{:?}")` of every item (`src/repository.rs:6355`).
- About 12 `migrate_registry` passes and about 116 fsynced SQLite commits per save; together they measured under a second.
- A comment's sync repeats a full worktree status and a whole-tree blob read about 12 times, and opens 6–9 SSH sessions, reconnecting for each ref advertisement.

## Concurrency (owner, 2026-11-04, from vh-z9ep3)

The owner asked to consider running operations off the UI thread on a queue. The desktop already does this: D4's mutation worker (`src/desktop/services/worker.rs`) runs every mutation on one desktop-owned thread with a user lane ahead of a background lane, and reads and previews run on GPUI's background executor (`src/desktop/services/reads.rs`). So the window isn't frozen. What the slowness still costs:

- **Head-of-line waits:** the worker never preempts a running job, so a promote or close (35 s then, several minutes now) holds the user's next save or comment behind it.
- **CLI latency:** the CLI is one-shot with no resident worker (CLI contract), so a CLI user or agent waits the full time.
- **Previews and tab loads:** about 2 s each, mostly discovery's history walk, even off the UI thread.

The request/checkpoint/safe-point design makes queued and restarted jobs safe; parallelism is limited by the repository-wide lease and remote reservation, not the job model. Consider, alongside the profiling fixes: whether independent items' local operations could run concurrently under a narrower lease, whether the worker should run a second lane concurrently for jobs that can't contend, and whether slow observations can be cached between a prepare and its confirm.

## Expected

A save, a comment's local checkpoint, a promote, a close and a prepare each cost about their own write and commit plus an incremental index update, and none grows with history depth or item count. Direction, not decided: derive activity times for all items in one history walk (one diff per commit, mapped to items), cache them by commit across refreshes, and reconsider whether each refresh needs two full observations; audit only the operation in hand, or once per process, instead of every stored image proof.

## Acceptance

- `ticket save`, `document save`, `comment add`, `ticket create`, `document promote`, `ticket close` and a prepare measured before and after on this repository with a release build. Targets (owner, 2026-11-19): a local save, create or comment checkpoint under 1 s; `item sync` and `comment add` 2–4 s; `document promote` and `ticket close` under 10 s.
- Every item's `activity_at` and `activity_source` are unchanged, pinned by a table-driven library unit test.
- A unit test pins how many commits or tree diffs one refresh performs, so a per-item walk cannot return unnoticed.
- A profile of a save, a promote and a close no longer shows `latest_commit_touching`, a second full observation or the image-proof audit, and the concurrent-change detection that the before and after observations give today is kept or replaced by an equivalent.
- No recovery behaviour changes beyond the per-item index holds the owner accepted (design ruling R-3: a write's index step holds only its item, and a crashed write whose commit landed no longer blocks refresh and poll), and the index gives today's results except that a change made outside Manyhands to another item is seen at the next refresh, poll or desktop start, or flagged stale when read (amended by the owner, 2026-11-19). Files and directories that Git ignores and does not track (build output, caches) are not read for the index, so they are no longer reported as problems or items (owner, 2026-11-20).

## Related

- vh-44q1k (01M7TEA2VE70AC3AAA3VHM2R8K): performance of operation attribution, the blocker wait and its retries, left by vh-bhepg. A separate fix in the concurrent-writers code, but its wait and retry numbers depend on how long a write holds the lease, which this ticket sets, so measure it after this one lands. Its item (3) shares the image-proof audit above.
- Evidence for the promote and close numbers (from vh-z9ep3): found by Wave 03 C5 (ticket 01M4CC0VMXTNXNH78C11DYXS8H); docs/RFC/cli-contract.md, "C5 evidence and known limits", follow-up list; docs/plans/2026-10-31-wave-03-cli-05-completion-and-polling-execution.md (C5 execution ledger).

Design: `docs/plans/2026-11-19-fast-writes-design.md` on this ticket's branch.

## How to measure

Real durations are in the registry: `request_records.accepted_at` and `finished_at` in `manyhands.sqlite3` (open it read-only). This machine has no `perf` and Yama `ptrace_scope` is 1, so start the CLI through a small `prctl(PR_SET_PTRACER, PR_SET_PTRACER_ANY)` exec wrapper and sample it with `gdb -p <pid> -batch -ex 'thread apply all bt'`; sampling adds about 20% to wall time. A replica needs an SSH-form origin to enable; an unreachable `ssh://git@127.0.0.1:1/...` works for local saves.