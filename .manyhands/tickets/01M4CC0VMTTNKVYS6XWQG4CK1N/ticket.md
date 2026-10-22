---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4CC0VMTTNKVYS6XWQG4CK1N"
title: "Wave 03 C2: CLI Repository And Credential Administration"
type: "cycle"
status: "open"
project: "manyhands"
team: "core"
wave: "03"
cycle: "C2"
---

Make repository onboarding and SSH setup usable without GUI or
external Git/key-generation commands.

CLI track of [Wave 03](../../../docs/Waves/wave-03-dogfooding.md#c2-cli-repository-and-credential-administration).
The Wave document owns this Cycle's scope, exclusions and track rules.

## Planning

- [x] Create `docs/Cycles/wave-03-cli-02-repository-and-credentials.md`.
- [x] Create the design and implementation plan and record their checkpoints
  as ticket comments.

## Entry Gate

Wave 02 Cycles 01–04 have review and verification evidence on the main
branch used for this Cycle before its implementation plan is approved. An
unmerged or in-flight Wave 02 branch does not satisfy this.
The Wave 03 entry gate applies, including the refreshed API audit for the
capabilities this Cycle consumes.

Wave 03 dependencies: F1, F2 and C1.

## Exit Evidence

Empty/existing repository onboarding, missing identity,
cancelled/wrong unlock, exact host replacement, imported-key non-deletion,
generated-key deletion retry and configuration failures. Publish schemas for
each new verb; prove no prompt consumes JSON/body stdin or persists a secret.
