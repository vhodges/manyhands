---
slug: vh-a22w3
id: 01M6K05MY53T594TP05CPB5ZQF
title: Make a canonical add/add of a ticket file resolvable through resolve-local
type: gap
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
A ticket file that reached primary through another commit and was then edited on both sides conflicts as add/add, which the gate classifies `unsupported_conflict`: external-only, so `context resolve-local` refuses it and the repair needs Git (C5 decision 30; found by Task 11's real-content run).

Expected: an add/add of the same canonical item is a canonical conflict with base absent, resolvable through `context show-local` and `context resolve-local`, with the gate's validation of immutable closure fields unchanged.

Acceptance: rows for both item kinds, a closure field on either side, and the external-only cases that must stay external.

Found by Wave 03 C5 (ticket 01M4CC0VMXTNXNH78C11DYXS8H). Evidence: docs/RFC/cli-contract.md, "C5 evidence and known limits", follow-up list; and docs/plans/2026-10-31-wave-03-cli-05-completion-and-polling-execution.md (C5 execution ledger).
