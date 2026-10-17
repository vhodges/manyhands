---
manyhands_managed: true
manyhands_kind: comment
id: "01M46ECQP30D5WT2XA404VWTKF"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DB"
created_at: "2026-10-05T16:36:02Z"
---

Planning checkpoint: Cycle 03 authenticated SSH transport is ready for review.

Reused branch `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DB` and worktree
`/home/vhodges/work/src/manyhands/.manyhands/worktrees/01K7F6H9J2N4Q6S8V0X2Z4B6DB`.
Both main and ticket checkouts were clean before work. Fresh fetch observed
origin/main at `89ce24d0c5b99c817ec81615fe610f65d7c81a99`. Local main includes
four additional commits (Cycle skills, tooling, research, and the main merge),
so the requested main-based rebase used `a21a31aeea5aaf7c03cf7692848ddadb5dde1242`.
Ticket HEAD moved from `edb5ea92b8e5c3c4183cf06dbef334942a3b2a07` to
`305c0f0227ba2d6cfa9b1f807cbeaa474374cfa5` with no conflicts. Ancestry checks
against both origin/main and main passed and post-rebase status was clean.
Git metadata required sandbox escalation; the user approved the rebase command.
No other worktree or main files were changed.

Prepared proposed artifacts:

- [Cycle contract](../../../docs/Cycles/wave-02-cycle-03-authenticated-ssh-transport.md)
- [Design](../../../docs/plans/2026-10-05-wave-02-cycle-03-authenticated-ssh-transport-design.md)
- [Implementation plan](../../../docs/plans/2026-10-05-wave-02-cycle-03-authenticated-ssh-transport-implementation.md)

Read the approved authentication, persistence, Git workflow, and test RFCs and
Cycle 02's closing evidence. Inspected selection/session/migration APIs, locked
git2/libgit2 callback implementation, existing tests, CLI, and native CI.

Proposed mechanism: scoped authenticated connection driver, non-mutating public
connection verification, exact host-pin approval/replacement with optimistic
transactions, and a test-only loopback SSH Git server. Imported formats remain
backend-validated; only backend-authenticated passphrases are cached, and
rejection evicts them. The plan tests inherited known_hosts versus pin precedence
and configures required Windows libssh2 OpenSSL features. It distinguishes
pre-transfer preservation from later ambiguous transfer failures.

The five task checkpoints are contracts/parser; portable SSH fixture; host trust
and callbacks; session/connection integration; privacy and native compatibility.
Cycle 04 retains ref interpretation/reservations/polling and Cycle 05 retains
production synchronization. Recommended execution is native in-session with
independent whole-branch review. Approval and implementation authorization are
pending; the ticket remains open. The CLI is empty, so canonical files are
updated directly under the approved dogfooding workflow.

Planning verification covers artifact links, IDs/frontmatter, consistency,
whitespace, rebase ancestry, and changed-file scope. No Rust code or dependency
files changed and no Rust test result is claimed for this planning session.
Fresh baseline and final Devenv checks plus real native SSH runs are explicit
implementation gates. Publication has not been attempted; rebased remote
history will need reconciliation before any later authorized push.
