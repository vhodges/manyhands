---
slug: vh-44q1k
deps:
- 01M7RDVNZDCCVSMWHDY9503852
id: 01M7TEA2VE70AC3AAA3VHM2R8K
title: Performance of operation attribution, the blocker wait and its retries
type: task
status: open
project: manyhands
manyhands_managed: true
manyhands_kind: ticket
---
Follow-up from vh-bhepg (concurrent writers: attribute, serialize, name the blocker), closed 2026-11-19 and merged as 02dcdfb3. Design: docs/plans/2026-11-18-concurrent-writers-design.md. The next tickets are performance related; this one collects what vh-bhepg left.

Measure first, then fix what the numbers support: (1) the CLI retry (100, 200, 400 ms) multiplies the 5 s blocker wait, so the worst case for comment add is about 25 s; decide whether the CLI should retry a busy that already waited, shorten the wait for non-interactive callers, or keep it, with a measured two-writer run on a loaded machine; (2) Holders N+1: the status path judges each unfinished row with its own reads (select_status_operations has no filter); (3) read_live_holder runs repository_operations twice in the worst case, and each run includes the local_sync audit; (4) RowOrigin::here is rebuilt per loop iteration in the wait and should be hoisted; (5) the final review-ready run of the repository took over four hours (mutation_remote alone 6500 s): see the Testing Strategy in AGENTS.md before adding any case.

Acceptance: each item has a before and after number from a scripted run, or an explicit decision to leave it; no new real sleeps in tests; the wait budget and poll interval stay injectable.

Related: vh-f3wts (01M7RDVNZDCCVSMWHDY9503852, which absorbed vh-z9ep3 on 2026-11-19) is the write-latency fix: saves take 20–41 s and comment add 60–114 s, almost all of it re-walking Git history in the index refresh. How long a write holds the lease sets how often the 5 s blocker wait in (1) runs out, so measure (1) after vh-f3wts lands or record which build was measured. The local_sync audit in (3) includes the image-proof audit that vh-f3wts also covers (`local_operations` → `local_sync::record::audit` → `image_proof::audit`).