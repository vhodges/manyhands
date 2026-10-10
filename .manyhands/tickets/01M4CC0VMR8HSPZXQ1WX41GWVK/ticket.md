---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4CC0VMR8HSPZXQ1WX41GWVK"
title: "Wave 03 F2: Request Replay, Confirmation And Shared Mutation Bridges"
type: "cycle"
status: "open"
project: "manyhands"
team: "core"
wave: "03"
cycle: "F2"
---

Provide the shared safety boundary every mutation adapter consumes,
and the narrow headless mutations both front ends need but the baseline lacks.

Foundation of [Wave 03](../../../docs/Waves/wave-03-dogfooding.md#f2-request-replay-confirmation-and-shared-mutation-bridges).
The Wave document owns this Cycle's scope, exclusions and track rules.

## Planning

- [x] Create `docs/Cycles/wave-03-foundation-02-replay-confirmation-and-bridges.md`.
  Drafted 2026-10-08; not yet approved.
- [x] Create the design and implementation plan. Drafted 2026-10-08 as
  `docs/plans/2026-10-08-wave-03-foundation-02-replay-confirmation-and-bridges-design.md`
  and `…-implementation.md`; reviewed in two rounds by three independent
  reviewers; not yet approved. Refreshed on 2026-10-10 for what landed on
  main since (Wave 02 Cycle 06, the index fix, the journal fix); the
  refresh was reviewed once and corrected; the corrections are unreviewed.
  Thirteen decisions are in the Cycle document.
- [x] Product-owner approval of the three documents: 2026-10-10, with all
  thirteen decisions decided, each as recommended (12 as its fourth
  option). The last corrections and the abandon section were approved
  without a further independent review.
- [ ] Implementation authorization (a separate gate).

## Entry Gate

Met for planning on 2026-10-08: Wave 02 Cycles 01–05 and F1 are on main at
`6cf5d7f`. Planning found a defect in the existing operation journal that
blocked Part B. It was fixed on ticket `01M4EWN2DK3MY6H4GBYDYXF6QH` and
merged on 2026-10-10. Main at `f87ce81` was merged into this branch the
same day.

Wave 02 Cycles 01–05 have review and verification evidence on the main
branch used for this Cycle before its implementation plan is approved. An
unmerged or in-flight Wave 02 branch does not satisfy this.
The Wave 03 entry gate applies, including the refreshed API audit for the
capabilities this Cycle consumes.

Wave 03 dependencies: F1.

## Added Scope (2026-10-07)

Ticket relationships and short codes, per the
[RFC](../../../docs/RFC/ticket-relationships-and-short-codes.md) and PRD
`MH-CONTENT-005`/`MH-CONTENT-006`. The RFC reaches this branch when it is
rebased onto main.

Ticket create and save accept `deps` and `parent`, write them in canonical
form and reject a cycle before any write. Create generates the short code.
Add the explicit short-code assign operation, repository-local initials and
the optional repository prefix. Exit evidence adds: golden vectors for the
short-code and initials derivations, and a short code unchanged by rename,
identity change and prefix change.

## Added Scope (2026-10-10)

A confirmed `operation abandon`, decided by the product owner on
2026-10-10 as a task in Part B rather than a separate ticket. It closes a
pending local journal row, touches no file and reports what the operation
left behind, so a repository blocked by an operation that cannot be
repeated has a way out other than deleting the database. A pending merge
conflict is not covered; that is ticket `01M4H33R34Z7C7EEKTY1ZCT950`.

## From F1 Planning (2026-10-07)

Write the optional comment field `created_by`, the confirmed Git identity, when
a comment is created, as the canonical schema RFC now defines. F1 only reads
it. Also own the canonical written form of `deps`: the existing serializer
re-emits all front matter on every save, so values are preserved and
formatting is not.

Decided by the product owner on 2026-10-07 during F1 planning; the
RFC and Wave amendments are on the F1 branch
(`manyhands/ticket/01M4CC0VMQ7R15A7M9SPN3KB67`) and reach main when it merges.

## Exit Evidence

Real save and clean-sync fixtures prove stale preview
rejection, changed-input rejection, accepted-consent retry, lost output
reconciliation, cache-loss recovery and no duplicate effects. Cancellation is
observed at approved safe points. Each bridge has expected-observation, retry
and failure evidence; host approval publishes nothing and refreshes no remote
ref; ambiguous identity stays non-editable. No body/secret enters request
records.
