---
slug: vh-ymjt9
id: 01M6K0600RYRT2KXG632QY1P39
title: Advance tracking after a push when fetch and push endpoints differ
type: gap
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
C5 decision 29 moves an existing tracking ref forward after a confirmed push only when fetch and push share an endpoint, and never creates one. With split endpoints, before a first fetch, or with the repository lease held past 250 ms, tracking stays behind, so promote-then-close can be refused `primary_ahead_of_remote` until `poll once`.

Expected: advance tracking from a verified fetch-side read of the pushed ref, safely against concurrent fetches.

Acceptance: rows for split endpoints, lease contention and the first-fetch case, with the four decision 29 guard rows still passing.

Found by Wave 03 C5 (ticket 01M4CC0VMXTNXNH78C11DYXS8H). Evidence: docs/RFC/cli-contract.md, "C5 evidence and known limits", follow-up list; and docs/plans/2026-10-31-wave-03-cli-05-completion-and-polling-execution.md (C5 execution ledger).
