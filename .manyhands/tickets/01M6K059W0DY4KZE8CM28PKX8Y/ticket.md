---
slug: vh-86fqm
id: 01M6K059W0DY4KZE8CM28PKX8Y
title: Allow a ticket to be reopened
type: exploration
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
The owner noted (2026-11-02) that a ticket could be reopened in the normal course of events. The MVP RFC (`docs/RFC/mvp-rfc.md:58,470`) excludes reopening, the schema RFC forbids repair bypassing "the no-reopening boundary", and the library enforces closure immutability: canonical validation, integration refusing a merge that drops `closed_at`, the gate's all-side immutability, and closure's `PrimaryReopened` blocker.

Explore: a reopen verb or an accepted manual edit, the validation and merge rules it needs, and what made-ready, readiness and closure mean for a reopened ticket. Outcome: a proposed RFC amendment for the owner, not code.

Found by Wave 03 C5 (ticket 01M4CC0VMXTNXNH78C11DYXS8H). Evidence: docs/RFC/cli-contract.md, "C5 evidence and known limits", follow-up list; and docs/plans/2026-10-31-wave-03-cli-05-completion-and-polling-execution.md (C5 execution ledger).
