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

## Delivery Status — Review-Ready

**2026-10-23: implementation, local verification and final reviews complete;
Task 12 through Step 6 is review-ready.** The controller reports both final
whole-branch reviews **APPROVE, no findings**, for
`563893c118f8e6a2d1f9ec9ccfe240f120decb95..9c3a23ef349a667132ded3faa35cacb118ee2fa9`:

- Spec `ses_e96bcb4cfffeoD5umHyL4mfvrx`, after the no-op evidence correction.
- Quality `ses_e96987265ffe2Est5HzXZ9VZp8`, independent actual-code review.

Evidence and limitations are in the execution ledger and review-ready comment
`01M5MPSP0DHPDMFA2N9X8B0A1C`: separate historical/upstream test records,
owner-authorized targeted checks, all-feature output ordering, all eighteen
verbs' no-op classification, and the exact manual/automated identity distinction.
Native Windows console/key-file checks, macOS PTY behavior and post-merge CI
remain owed; each CI run requires approval. Known wrong-passphrase defect
`01M5E6DYJ6GN7DMW4424TH3V78` remains open and nonblocking.

**This ticket remains open. Closure is not authorized.** Review-ready status
does not authorize push, PR, merge, cleanup or CI dispatch.

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
