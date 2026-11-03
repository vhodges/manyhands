---
slug: vh-z9kmp
id: 01M6GHWZS03WCADS1A1JXV4PMP
title: A held remote reservation cannot be released after a restart
type: gap
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
If a resumed share's synchronization is not stopped and its transport then fails, the repository's sharing reservation stays held with no way to release it from the desktop after a restart.

Found by Wave 03 D4 (ticket 01M4CC0VN1KTQFSVR99378TDHV), Lane B Task 10b (U-5).

## Evidence

- The synchronization row is left `reconciling` and holds the reservation; `remote_standing` gives it `next_action: None`.
- The resume's own record is deleted (`kept()`), while the owning share's record stays accepted.
- `reserve_remote_operation_kind` (`src/repository/mutation/reservation.rs:1162-1176`) answers `busy` for every other operation, with no expiry or lease.
- Abandon is never offered for remote rows.
- `OperationDto.owner` names the repository, not the owning request, so a desktop that restarted cannot re-send the owner request (Give up).

Today the only recovery is `manyhands-cli operation resume <operation-id>` once the remote is reachable. D4 records this as Known Limit K-16.

## Expected

After a restart a held remote reservation can be ended from the pending list. Either:

- name the owning request so its Give up can be re-sent, or
- offer Resume or Abandon on a reconciling remote row.

## Acceptance

- A test where a resume fails its transport, the session ends, and a new session ends the reservation through a listed action.
- After that, another share in the repository reserves.
