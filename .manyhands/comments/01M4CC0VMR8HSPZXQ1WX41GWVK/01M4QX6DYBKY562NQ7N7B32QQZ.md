---
id: 01M4QX6DYBKY562NQ7N7B32QQZ
item_id: 01M4CC0VMR8HSPZXQ1WX41GWVK
created_at: 2026-10-12T11:15:33Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

Part B checkpoint (Tasks 5–14 and 12A: the mutation boundary, replay and
re-entry, confirmation, the bindings for 24 of the 30 commands, operation
abandon, synchronization with progress and cancellation, operation resume).
Commits `4c54d99..2d8e8d9` plus documentation. Part C (Tasks 15–19) is not
authorized and not started.

**Verification at `2d8e8d9`**, through Devenv on Linux x86_64, each exit 0:
`cargo fmt --check`,
`cargo clippy --all-targets --all-features --locked -- -D warnings`,
`cargo test --all-features --locked --no-fail-fast` (1878 passed, 0 failed in
29 libtest targets; the six SSH-harness targets passed 66, 25, 15, 63, 31 and
105 cases). No workflow was dispatched; Windows and macOS are unproven.
`journal_rejection`, which holds the heartbeat-timing tests, is now in the
native test list and has never run there.

**Review.** Each task had an independent spec and quality review with fix
rounds (12A: two, 13: three, 14: one). Three independent reviews of the whole
Part B range found no Critical issue. Nine findings went to one fix round
(`81a1b3f..2d8e8d9`); a scoped re-review of it found no Critical issue.

Fixed in that round:

- a ticket write rejected for a relationship cycle could be written by a
  retry of the same request;
- a synchronization that had published and then failed on a later call lost
  its record;
- `key delete` refused for a replaced key file offered a preview that led
  back to the same refusal;
- `remote add` and `remote remove` now say what became of the index hand-off;
- `operation.resume` is offered before `operation.abandon` when another
  request's pending operation is in the way and a resume would take it up;
- privacy-scan tests for repo create, remote select, key select, clear and
  remove, and the index failure paths;
- `journal_rejection` in the native test list, byte-for-byte progress goldens.

**Rulings confirmed by the product owner.** `HostVerificationUnavailable` is a
Blocked-class code; the four "changed" kinds stay `external_change` and offer
`request.retry` after reserving; `invalid_remote` is kept; key import source
kinds are `key_unavailable`; no `index.rebuild` on results naming no
repository; `recovery_required` with no action for pending key material; the
busy answer for abandoning a live operation; the rebuild ruling.

**Known and accepted** (the design's known-limits section has each in full).

- Operation liveness is a heartbeat, best effort; a call in the middle of a
  push has no liveness signal.
- A cancelled request whose push outcome cannot be learned reads
  `recovery_required`, publication not confirmed.
- A blocked synchronization can leave its reservation held (five causes
  reproduced); `merge_required` and `remote_branch_deleted` offer no action.
- Defects filed as their own tickets: `01M4PB7WZN9WNAQ3KXZ19J28KM`,
  `01M4PB7YZMBFAFFM94AZY08HWS`, `01M4PB80T7SVB23TQ6466RDHCB`,
  `01M4PB82G07RZSQMNJZN672S3W`.

**Open for the product owner.**

1. A ticket or document create on a Git repository that was never enabled
   answers `repository_not_registered` offering `index.rebuild`; the rebuild
   registers the repository and the create repeats the same answer. Should a
   rebuild register such a repository, or should it not be offered?
2. Should settings commands (remote select; key select, clear, import) refuse
   a late retry?
3. A registration whose directory is gone cannot be removed (older defect):
   ticket?
4. The held reservation after a blocked synchronization: ticket?
5. `request.retry` on a "changed" synchronization result is offered only
   while the reservation is still the request's own, not when fenced.
6. The remaining questions are listed in the design under "Open questions for
   the product owner".

**For Part C.** The reviews' structural findings are carried to its first
step: one confirmed re-entry function, one writer of finished results that
reports whether it applied, one owner of recovery actions, journal steps as
an enum, a conformance table over all 24 commands, one test world. Deferred
from the fix round: a re-entered ticket write whose cycle check cannot be
read is let through; two cases where the discovery effect understates. Minor
findings remain deferred in the execution ledger.
