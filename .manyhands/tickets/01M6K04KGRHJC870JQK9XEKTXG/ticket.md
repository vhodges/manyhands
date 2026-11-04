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
