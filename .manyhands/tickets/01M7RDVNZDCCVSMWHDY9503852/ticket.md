---
slug: vh-f3wts
id: 01M7RDVNZDCCVSMWHDY9503852
title: Ticket, document and comment saves take 20–110 s re-walking Git history for every item
type: defect
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Local saves are slow and getting slower. On this repository (314 items, 172 first-parent commits on main, 57 ticket worktrees), `ticket save` and `document save` take 20–23 s and `comment add` takes 60–114 s, while reads take about 70 ms. The registry's `request_records` show `ticket save` at 6–7 s on 2026-10-25 and at 20–41 s since 2026-11-04; `ticket create` has a median of 20 s, `poll once` about 29 s and `ticket close` 176–301 s. The desktop runs the same library calls on its single mutation worker, so a desktop save takes as long before the pill reads "Saved locally", and can also wait behind a poll's own index refresh.

## Measured cause (2026-11-18)

Measured on a replica with this repository's history and its 57 ticket worktrees (isolated data directory and HOME, release build, gdb stack sampling). A `ticket save` took about 23 s, of which 22.5 s was CPU, so lock, fsync and network waits do not matter here. About 98% of the samples were in the index refresh that follows the commit:

- Every committing save ends in `handoff_post_authoritative` (`src/repository.rs:3878`) → `refresh_repository_observed` (`src/repository.rs:1511`), which calls `observe_root` twice, before and after (`:1523`, `:1545`), and compares the two observations.
- Each pass runs `observe_items`. For every item, `latest_commit_touching` (`src/repository/discovery.rs:697`) walks main's first-parent history and runs an unfiltered `diff_tree_to_tree` per commit (`commit_touches_paths`, `:723`) until one touches the item's file or its comments. Here that is about 21,500 whole-tree diffs per pass and about 43,000 per save, with nothing reused across items or passes.
- Each pass also re-observes every active ticket worktree: about 6 s for 57 (a save took 17 s with one worktree and 23 s with all 57).
- `comment add` runs that refresh after its local checkpoint (about 29 s together with its own scans) and a second full refresh after publication (`synchronization_refresh`, `src/repository/remote/sync.rs:6768`). Subtracting both from a real 62 s run leaves an estimated 10 s for SSH and sync checks, so the two refreshes are most of a comment's time.

The cost scales with items × history depth (capped at 1024 commits per item) and with worktree count, so every commit and every new item slows every save. Tree diffs per pass: about 2.9k on 2026-10-25, 12.7k on 2026-11-04 and 21.5k on 2026-11-18.

Related: vh-z9ep3 (01M6K04KGRHJC870JQK9XEKTXG) names the same history walk as a hot spot for promote, close and prepare, so a fix there is shared. This ticket covers the save and comment paths and what that ticket does not: the double observation per refresh, the two refreshes per comment, and the worktree re-observation.

## Smaller costs seen in code traces, not dominant today

- `first_parent_configuration_entries` walks the whole first-parent history, with no cap, on every save (`src/repository.rs:10966`).
- `changed_contexts` compares the two observations with `format!("{:?}")` of every item (`src/repository.rs:6182`).
- About 12 `migrate_registry` passes and about 116 fsynced SQLite commits per save; together they measured under a second.
- A comment's sync repeats a full worktree status and a whole-tree blob read about 12 times, and opens 6–9 SSH sessions, reconnecting for each ref advertisement.

## Expected

A save, or a comment's local checkpoint, costs about its write and commit plus an incremental index update, and does not grow with history depth or item count. Direction, not decided: derive activity times for all items in one history walk (one diff per commit, mapped to items), cache them by commit across refreshes, and reconsider whether each refresh needs two full observations.

## Acceptance

- `ticket save`, `document save` and `comment add` measured before and after on this repository with a release build; the owner sets the target when this is planned.
- Every item's `activity_at` and `activity_source` are unchanged, pinned by a table-driven library unit test.
- A unit test pins how many commits or tree diffs one refresh performs, so a per-item walk cannot return unnoticed.
- A profile of a save no longer shows `latest_commit_touching` or a second full observation, and the concurrent-change detection that the before and after observations give today is kept or replaced by an equivalent.

## How to measure

Real durations are in the registry: `request_records.accepted_at` and `finished_at` in `manyhands.sqlite3` (open it read-only). This machine has no `perf` and Yama `ptrace_scope` is 1, so start the CLI through a small `prctl(PR_SET_PTRACER, PR_SET_PTRACER_ANY)` exec wrapper and sample it with `gdb -p <pid> -batch -ex 'thread apply all bt'`; sampling adds about 20% to wall time. A replica needs an SSH-form origin to enable; an unreachable `ssh://git@127.0.0.1:1/...` works for local saves.
