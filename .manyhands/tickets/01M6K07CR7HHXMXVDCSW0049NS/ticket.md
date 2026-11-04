---
slug: vh-rfhsq
id: 01M6K07CR7HHXMXVDCSW0049NS
title: 'Local-only lifecycle runs print progress: connecting'
type: defect
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
`document promote` and `ticket close` in a repository with no publication remote print `progress: connecting`, though nothing connects. It comes from `src/repository/mutation/command.rs:490`, whose `stage` gives `Connecting` for both verbs whatever the repository has (C5 ledger, Task 7 findings).

Expected: a local-only run reports the stages it actually goes through.

Acceptance: human-output goldens for a local-only promotion and closure, and the remote cases unchanged.

Found by Wave 03 C5 (ticket 01M4CC0VMXTNXNH78C11DYXS8H). Evidence: docs/RFC/cli-contract.md, "C5 evidence and known limits", follow-up list; and docs/plans/2026-10-31-wave-03-cli-05-completion-and-polling-execution.md (C5 execution ledger).
