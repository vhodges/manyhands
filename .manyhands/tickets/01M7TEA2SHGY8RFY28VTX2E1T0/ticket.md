---
slug: vh-cegpm
id: 01M7TEA2SHGY8RFY28VTX2E1T0
title: Per-item remote publication reservations
type: gap
status: open
project: manyhands
manyhands_managed: true
manyhands_kind: ticket
---
Follow-up from vh-bhepg (concurrent writers: attribute, serialize, name the blocker), closed 2026-11-19 and merged as 02dcdfb3. Design: docs/plans/2026-11-18-concurrent-writers-design.md.

Decision D2 of vh-bhepg: the unfinished-operation hold became per item for local writes, but remote publication reservations stay repository-scoped. A comment add on item Y behind an unfinished operation on X commits locally, and its publication is refused naming X. The owner chose to leave this for a follow-up.

Wanted: read why the reservation is repository-wide (the shared context/primary refs, the one-reservation-at-a-time rule, conflict reads) and list the reasons; scope the reservation per item where no real shared-state reason exists, keeping whole-repository requests exclusive; keep the wait-then-busy behaviour and the blocker naming.

Acceptance: with an unfinished publication on X, a comment add or sync on an unrelated item Y publishes; a repository-wide sync still waits; the table-driven scope test is extended; at most one SSH case.