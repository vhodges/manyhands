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

- [ ] Create `docs/Cycles/wave-03-foundation-02-replay-confirmation-and-bridges.md`.
- [ ] Create the design and implementation plan and record their checkpoints
  as ticket comments.

## Entry Gate

Wave 02 Cycles 01–05 have review and verification evidence on the main
branch used for this Cycle before its implementation plan is approved. An
unmerged or in-flight Wave 02 branch does not satisfy this.
The Wave 03 entry gate applies, including the refreshed API audit for the
capabilities this Cycle consumes.

Wave 03 dependencies: F1.

## Exit Evidence

Real save and clean-sync fixtures prove stale preview
rejection, changed-input rejection, accepted-consent retry, lost output
reconciliation, cache-loss recovery and no duplicate effects. Cancellation is
observed at approved safe points. Each bridge has expected-observation, retry
and failure evidence; host approval publishes nothing and refreshes no remote
ref; ambiguous identity stays non-editable. No body/secret enters request
records.
