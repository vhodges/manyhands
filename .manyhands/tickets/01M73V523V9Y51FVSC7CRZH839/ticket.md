---
slug: vh-fcpb6
id: 01M73V523V9Y51FVSC7CRZH839
title: Let tests drive a real context retirement without SSH
type: gap
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
A standalone `ContextRetire` observes the remote over SSH before it proves the closure from primary. The library's own tests get past that with a `cfg(test)` thread-local (`run::synthetic_advertisement`), which a downstream binary does not link, and the git transport cannot be initialised inside a multi-threaded GPUI test process. A desktop test therefore cannot run a real retirement end to end.

Follow-up from Wave 03 D6 (ticket `01M4CC0VN3VPVVQ3RNWJKXWEVX`); see its execution ledger (`docs/plans/2026-11-07-wave-03-desktop-06-polling-and-shutdown-execution.md`) and design Known Limits. Filed on the owner's approval, 2026-11-10. Task 12b, the owner's decision on 2026-11-10 to accept the gap; Known Limits K-58 and K-60.

## Evidence

- D6 verified retirement against canned answers and a real local repository. What stayed unverified in code: U-9's observer row and K-29 (an observer row left by a process death mid-observation), K-58 (the tests see `already_absent`, not a real removal), K-60 (the real library listing under the entry's operation ID). The owner put them on the native checklist.
- The library's cancel hooks are `cfg(test)` as well, so no in-process hook leaves an observer row.

## Expected

- A feature-gated way for tests of a dependent crate to supply a synthetic remote advertisement to a retirement, and a way to leave an observer row as a dead process would.

## Acceptance

- A library test of the new seam.
- D6's canned retirement tests gain a real-library counterpart in a desktop follow-up, and U-9, K-29, K-58 and K-60 move from the native checklist to tests.