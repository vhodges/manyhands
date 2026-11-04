---
slug: vh-3m4xe
id: 01M6K071HJAMBZ5NZ7T14Y3ZAA
title: A synchronization meeting a held lease answers internal_error, not busy
type: defect
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
C5 decision 24 made a poll's fetch that meets a held repository lease answer `busy` (exit 5, retryable). A synchronization's fetch passes the same line (`src/repository/transport/remote.rs:656`) and still reports `ProtocolFailure`, answered `internal_error` (exit 1); a lifecycle's context child inherits it.

Expected: `item sync`, `repo sync` and lifecycle children answer `busy` with `request.retry` when the fetch meets a held lease, and a real protocol failure keeps `internal_error`.

Acceptance: a held-lease row for each and a control for a real protocol failure.

Found by Wave 03 C5 (ticket 01M4CC0VMXTNXNH78C11DYXS8H). Evidence: docs/RFC/cli-contract.md, "C5 evidence and known limits", follow-up list; and docs/plans/2026-10-31-wave-03-cli-05-completion-and-polling-execution.md (C5 execution ledger).
