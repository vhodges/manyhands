---
slug: vh-xmza1
id: 01M73V4F416W347GPCNAYRTXB0
title: Expose whether a pending retirement owes only its index hand-off
type: gap
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
The desktop offers Resume on a pending retirement row only when an in-flight entry names its operation, or when the library's "owes only its index hand-off" shape holds. That shape is a library-internal fact (`LocalRow::owes_only_its_hand_off`), so the desktop keeps a copy of the rule (`owes_only_hand_off`).

Follow-up from Wave 03 D6 (ticket `01M4CC0VN3VPVVQ3RNWJKXWEVX`); see its execution ledger (`docs/plans/2026-11-07-wave-03-desktop-06-polling-and-shutdown-execution.md`) and design Known Limits. Filed on the owner's approval, 2026-11-10. Task 12a; design E4 and Known Limit K-27.

## Evidence

- The D6 reviewer checked that the two rules select the same rows today. Nothing keeps them equal if the library's rule changes.

## Expected

- The library's operation output states whether a pending local row owes only its index hand-off, so the desktop reads it instead of re-deriving it.

## Acceptance

- A library test of the new output field for a retirement that owes only its hand-off and one that owes more.
- The desktop's copy of the rule is deleted in a desktop follow-up, with its tests reading the field.