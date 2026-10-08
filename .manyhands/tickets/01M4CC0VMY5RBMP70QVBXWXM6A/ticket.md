---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4CC0VMY5RBMP70QVBXWXM6A"
title: "Wave 03 D1: Desktop Shell, Onboarding And Discovery"
type: "cycle"
status: "open"
project: "manyhands"
team: "core"
wave: "03"
cycle: "D1"
---

Provide a keyboard-usable desktop over the foundation services.

Desktop track of [Wave 03](../../../docs/Waves/wave-03-dogfooding.md#d1-desktop-shell-onboarding-and-discovery).
The Wave document owns this Cycle's scope, exclusions and track rules.

## Planning

- [ ] Create `docs/Cycles/wave-03-desktop-01-shell-and-onboarding.md`.
- [ ] Create the design and implementation plan and record their checkpoints
  as ticket comments.

## Entry Gate

Wave 02 Cycles 01–04 have review and verification evidence on the main
branch used for this Cycle before its implementation plan is approved. An
unmerged or in-flight Wave 02 branch does not satisfy this.
The Wave 03 entry gate applies, including the refreshed API audit for the
capabilities this Cycle consumes.

Wave 03 dependencies: F1 and F2.

## Exit Evidence

Native startup and keyboard onboarding/credential/inspection
checks; responsive navigation during delayed scans, failed refresh, unavailable
repositories and unlock/host prompts. No display dependency leaks into the CLI.
At least one real repository is browsable end to end without fabricated rows.
