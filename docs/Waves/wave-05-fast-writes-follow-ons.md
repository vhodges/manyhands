---
id: 01M86GW5WPWP0F0W0BN0VNX4WE
title: 'Wave 05: Fast-writes follow-ons'
manyhands_managed: true
manyhands_kind: document
---
# Wave 05: Fast-writes follow-ons

## Theme

vh-f3wts (fast writes, closed 2026-11-23) met its targets: item sync 1.9 s, comment add 1.6 s, close --confirm 4.9 s. Its profile and reviews left follow-on work in three strands. Speed: each remote call still reopens the registry and repeats local checks, each write still walks the item's whole Git tree, operation attribution and the blocker wait add their own reads and retries, and the test suite itself takes over two hours. Correctness: the new index marks and the once-per-process registry audit leave a cleanup window and some changed answers to check. Consolidation: a rule and a set of security checks written twice, two very large files, and the final review's list of small fixes and test gaps.

Each ticket is one Cycle with its own branch and worktree. vh-mpvkc and vh-necew were filed earlier; vh-f3wts added a comment to each. vh-44q1k is a follow-on from vh-bhepg (concurrent writers, closed 2026-11-19). Tick a ticket's last box when `ticket close` has merged it.

## Speed

### vh-e98r1: Each remote call reopens the registry and repeats local checks

- [ ] A remote call's registry reads share its connection, or one connection per command
- [ ] Repeated local checks kept only where they guard a real window
- [ ] Fewer registry opens and status checks per sync and per close, counted
- [ ] `scripts/measure-writes` recorded before and after
- [ ] Transport recheck table and the key, selection and host-pin SSH cases pass, run singly
- [ ] Reviewed and closed

### vh-mpvkc: Comment add, sync and close read every file of the item's Git tree

- [ ] Bounded tree reader (docs/, .manyhands/tickets/, .manyhands/comments/) used by the four callers
- [ ] Table-driven test: the bounded and whole-tree readers give the same answers
- [ ] Test shows non-canonical blobs are not opened
- [ ] `validates_external_integration` and its tests unchanged
- [ ] `scripts/measure-writes` recorded before and after; second part (diff-based validation) decided
- [ ] Reviewed and closed

### vh-necew: Speed up the test suite

- [ ] SSH runner honours `--test-threads` and runs cases in parallel
- [ ] Decision-logic SSH cases moved to library or non-SSH tests, each mapped in a comment
- [ ] Real sleeps of 1 s or more replaced by an injected clock or duration
- [ ] `cargo-nextest` in Devenv, with a documented slowest-tests command
- [ ] SSH tier at 25 minutes or less, before and after times recorded
- [ ] Reviewed and closed

### vh-44q1k: Performance of operation attribution, the blocker wait and its retries

- [ ] CLI retry on top of the 5 s blocker wait: retry, shorten or keep, decided from a measured two-writer run on a loaded machine
- [ ] Holders N+1: the status path's per-row reads
- [ ] `read_live_holder`'s second `repository_operations` run, with its local_sync audit
- [ ] `RowOrigin::here` hoisted out of the wait loop
- [ ] Each item has before and after numbers from a scripted run, or a decision to leave it
- [ ] Wait budget and poll interval stay injectable; no new real sleeps
- [ ] Reviewed and closed

## Correctness

### vh-2nmns: A close or promotion lost during cleanup can leave the index reading current

- [ ] Context marked dirty before the worktree removal and before the branch deletion
- [ ] Failure-injection rows for each new pre-mark and for F-1's merge pre-mark
- [ ] Library test per window shows the context dirty, or the index stale, until a refresh
- [ ] Cleanup, closure and promotion tests pass, the SSH cases run singly
- [ ] Reviewed and closed

### vh-xsd0t: Survey write paths whose answers to registry damage changed

- [ ] Survey the write_records callers, the mutation binds, and the poll and remote paths
- [ ] Each moved answer pinned by a library test, or reverted
- [ ] Ticket comment with a table of paths and answers, before and after
- [ ] Fast-writes design's R-5 known limit updated (E-84, CHECK constraints)
- [ ] Reviewed and closed

## Consolidation

### vh-hzh0d: One connect-and-check helper for the SSH transport

- [ ] One helper for both connection paths, with no change in behaviour
- [ ] Gaps closed or explained: reconnect host decision, for_finalized_host, post-connect host check
- [ ] Reconnect tests renamed for what they pin
- [ ] Transport tests and every SSH-harness target pass
- [ ] Reviewed and closed

### vh-fc17c: Write the presented-problem rule once, match transient problems by code

- [ ] One presented-problem function shared by the store and the reads
- [ ] `TRANSIENT_PROBLEM` matches by problem code, not guidance text
- [ ] Read nits fixed: `unwrap_or(false)`, repeated queries, `DirtyContexts::read` twice
- [ ] Table-driven test of Table A; read_items, read_comments and read_repository pass
- [ ] Reviewed and closed

### vh-zmagc: Split src/repository.rs and src/repository/remote/sync.rs

- [ ] Named candidates moved into their own modules
- [ ] `git diff -M` shows only moves, visibility changes and use lines
- [ ] Static gates, fast tier, and the SSH tier for the touched paths pass
- [ ] Reviewed and closed

### vh-7cmhd: Leftovers from vh-f3wts's final review

- [ ] Script items (2)
- [ ] Index and discovery items
- [ ] Sync and lifecycle items
- [ ] Tests and coverage items
- [ ] Every item fixed, or closed with a reason in a ticket comment
- [ ] Reviewed and closed
