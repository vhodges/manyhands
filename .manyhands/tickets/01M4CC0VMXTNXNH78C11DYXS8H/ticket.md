---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4CC0VMXTNXNH78C11DYXS8H"
title: "Wave 03 C5: CLI Completion And One-Shot Polling"
type: "cycle"
status: "open"
project: "manyhands"
team: "core"
wave: "03"
cycle: "C5"
---

Complete the approved headless command contract.

CLI track of [Wave 03](../../../docs/Waves/wave-03-dogfooding.md#c5-cli-completion-and-one-shot-polling).
The Wave document owns this Cycle's scope, exclusions and track rules.

## Planning

- [ ] Create `docs/Cycles/wave-03-cli-05-completion-and-polling.md`.
- [ ] Create the design and implementation plan and record their checkpoints
  as ticket comments.

## Entry Gate

Wave 02 Cycles 08, 09 and 10 have review and verification evidence on the main
branch used for this Cycle before its implementation plan is approved. An
unmerged or in-flight Wave 02 branch does not satisfy this.
The Wave 03 entry gate applies, including the refreshed API audit for the
capabilities this Cycle consumes.

Wave 03 dependencies: F1, F2 and C1–C4.

## Exit Evidence

All approved CLI commands are accounted for. Real fixtures
prove dirty-primary blocking, stale consent, push/cleanup failures, no duplicate
close/merge, later primary publication and safe explicit polling. Run the six
CLI journey counterparts, use explicit polling for background discovery, and
prove command exit leaves no resident worker. This is the CLI contract gate.
