---
slug: vh-z9ep3
id: 01M6K04KGRHJC870JQK9XEKTXG
title: Speed up local sync's image-proof audit and discovery's history walk
type: defect
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Release-build `document promote` and `ticket close` take about 35 s each on this repository, and every prepare about 2 s (C5 decision 31's measurement, ledger "Task 11: real content").

A sampled profile names two hot spots, both older than C5:

- `local_sync::image_proof::audit` (`src/repository/local_sync/image_proof.rs:237`) re-decodes every stored local-sync image proof at each safe point and on every registry open; its cost grows with each stored sync.
- Discovery's per-item history walk (`src/repository/discovery.rs:698-743`, `latest_commit_touching`) diffs up to 1024 first-parent commits for each of about 175 items to find activity times.

Expected: audit only the operation in hand (or once per process), and cache or bound the activity-time walk. Acceptance: a measured release-build promote, close and prepare on this repository, before and after, with the hot spots gone from a profile, and no recovery or index behaviour changed.

Found by Wave 03 C5 (ticket 01M4CC0VMXTNXNH78C11DYXS8H). Evidence: docs/RFC/cli-contract.md, "C5 evidence and known limits", follow-up list; and docs/plans/2026-10-31-wave-03-cli-05-completion-and-polling-execution.md (C5 execution ledger).

## Concurrency (owner, 2026-11-04)

The owner asked to consider running operations off the UI thread on a queue. The desktop already does this: D4's mutation worker (`src/desktop/services/worker.rs`) runs every mutation on one desktop-owned thread with a user lane ahead of a background lane, and reads and previews run on GPUI's background executor (`src/desktop/services/reads.rs`). So the window isn't frozen. What the slowness still costs:

- **Head-of-line waits:** the worker never preempts a running job, so a ~35 s promote or close holds the user's next save or comment behind it.
- **CLI latency:** the CLI is one-shot with no resident worker (CLI contract), so a CLI user or agent waits the full time.
- **Previews and tab loads:** ~2 s each, mostly discovery's history walk, even off the UI thread.

The request/checkpoint/safe-point design makes queued and restarted jobs safe; parallelism is limited by the repository-wide lease and remote reservation, not the job model. Consider, alongside the profiling fixes: whether independent items' local operations could run concurrently under a narrower lease, whether the worker should run a second lane concurrently for jobs that can't contend, and whether slow observations can be cached between a prepare and its confirm.
