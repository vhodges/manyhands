---
slug: vh-wfhnd
id: 01M73V3W43DSZSKXBV1Z0WKTWZ
title: Put the error kind in a Check now's result envelope
type: gap
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
A Check now (`PollOnce`) result envelope carries a result code and `host_data` but not the library error's kind. The desktop cannot tell `HostVerificationUnavailable` (code `recovery_required`) from other causes of that code, nor `ProtocolFailure` or `RuntimeUninitialized` (code `internal_error`) from other `internal_error` causes.

Follow-up from Wave 03 D6 (ticket `01M4CC0VN3VPVVQ3RNWJKXWEVX`); see its execution ledger (`docs/plans/2026-11-07-wave-03-desktop-06-polling-and-shutdown-execution.md`) and design Known Limits. Filed on the owner's approval, 2026-11-10. Known Limit K-33, recorded at D6's Part C checkpoint.

## Evidence

- Those failures raise no attention entry in the desktop. After Unlock…, a Check now that hits host-verification-unavailable leaves the earlier "key locked" entry in place until the next automatic poll raises the host entry.
- D6 could not close this without a library change, and its boundary forbids one.

## Expected

- The envelope carries the error kind (or an equivalent stable discriminator) for a Check now's failure, so a client can classify it without parsing the message.

## Acceptance

- A library test that each of the three kinds above is distinguishable in the envelope.
- The desktop's conservative fallback for these codes is replaced by the classification, with its test, in a desktop follow-up.