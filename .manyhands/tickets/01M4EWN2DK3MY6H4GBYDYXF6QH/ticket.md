---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4EWN2DK3MY6H4GBYDYXF6QH"
title: "A rejected operation leaves a journal row that blocks the repository"
type: "defect"
status: "closed"
closed_at: "2026-10-10T03:09:52Z"
closed_by: "Vince Hodges <vince@imbas.ca>"
project: "manyhands"
team: "core"
wave: "03"
---

After an ordinary rejection, several operations return without completing
the journal row they began. A pending row makes every other operation on
that repository fail with `RecoveryRequired`, including removing the
registration. Today only tests reach this, because callers repeat the same
operation ID. Once front ends go through the Wave 03 F2 mutation boundary,
one rejected command, retried under a new request, blocks the repository.

Found by reading the source during F2 planning
(`01M4CC0VMR8HSPZXQ1WX41GWVK`), at main `6cf5d7f`. It has not been
reproduced by a test; the first tests of this ticket are that reproduction.
F2's design, "The Journal Defect", and its plan, "Prerequisite: Journal Rows
After A Rejection", on that ticket's branch, hold the detail. The product
owner asked for this ticket on 2026-10-08. F2's Part B depends on it; Part A
does not.

## Fix

Implemented on 2026-10-09 after Wave 02 Cycle 06 landed (main `5e4fad6`,
merged into this branch). Closed on its branch on 2026-10-09 by the product
owner, with a pull request open, so that it merges closed. Merge and
worktree cleanup remain the product owner's.

- Each call tracks whether it has begun a durable write it has not undone
  (`CallEffects` in `src/repository.rs`). When a call returns an error, its
  row is closed only if this call began the row and nothing is outstanding.
  The error kind is no longer consulted for that.
- A closed row is kept, marked `rejected`, so its ID stays bound to its
  target: reusing the ID for another target is still `OperationMismatch`. A
  repeat of the same ID and target begins again as a new call.
- A refresh may borrow any operation ID; it does not reset a rejected row.
- Product-owner ruling, 2026-10-09: the public lookup shows such an
  operation as `completed` with the step `rejected` and no next action.
  This is documented in `schemas/v1/operation.schema.json` and on the
  result field, and pinned by a contract test.
- Covered: `prepare_context`, the two saves, `submit_comment`,
  `add_remote`, `remove_remote`, `set_publication_remote`, `enable`,
  `create_and_enable`, including the return sites Cycle 06 added.
- The repository lease is held until the row is settled.
- Product-owner ruling, 2026-10-09: when `enable` or
  `set_publication_remote` fails and restores everything it wrote, its row
  is closed too. Three assertions in `tests/repository_enablement.rs` that
  expected a pending row were changed to expect none. No other existing
  test changed.
- Rows still stay pending after a write that was not undone, after a failed
  restore, after a repository was initialized, and after a half-created
  editing context. A save rejected after its editing context was fully
  created closes its row.
- Dropped, per the exit rule: `remove_registration` (no rejection that
  leaves its row could be provoked; its failures are inside one SQLite
  transaction). Refresh and rebuild are untouched.

Evidence: `tests/journal_rejection.rs`, 18 tests. The 12 that provoke a
rejection all failed with `RecoveryRequired` on the old code. The two
repeat tests each fail when their guard is removed. The four required
Devenv checks and the CLI smoke test pass: 1243 tests, 0 failed. Manual
native run 38014141474 passed on all five targets at `1fe0b07`.

Two independent reviews. The first found a repeat treated as a replay and a
lease released before settlement; both were corrected. The second found no
path that closes a row with an effect outstanding, and raised the refresh
reset, a repeat test that did not discriminate, and the lookup reading, all
addressed above. The changes made after the second review were not reviewed
again.

Known and left, all from the second review:

- On a resumed row the older rule still applies: a call that wrote a file
  and failed to record the step, followed by a repeat of the same ID that
  is cleanly rejected, closes the row with the file written. Unchanged from
  main.
- A Git failure inside the first write call itself (a stale lock file)
  leaves the row pending although nothing was written.
- If closing the row fails, the caller sees that error and not the
  rejection.
- `create_and_enable` marks an effect for the injected interruption before
  initialization, solely so an existing test keeps its expectation.

Not settled here, and still undecided at closure (see "To Decide On This
Ticket"): a process killed
mid-operation, a kill between a configuration write and its commit, and the
synchronization reservation case, which was not re-examined after Cycle 06.

## Reproduction

Reproduced on 2026-10-09 at main `6cf5d7f` by `tests/journal_rejection.rs`.
Six of its first seven tests failed with `RecoveryRequired` on the following
operation before the fix.

| Rejected call | Error returned | Next operation |
|---|---|---|
| `add_remote`, invalid name | `Git` | blocked |
| `remove_remote`, invalid name | `Git` | blocked |
| `enable`, branch not checked out | `WrongCheckedOutBranch` | blocked |
| `create_and_enable`, target not empty | `InvalidPath` | blocked |
| `save_document`, repository not enabled | `RepositoryNotEnabled` | blocked |
| `save_document`, detached primary | `DetachedHead` | blocked |
| `set_publication_remote`, invalid name | `DirtyConfigurationPath` | not blocked |

The last row did not reach the Git error it was aimed at; the selection
paths at `3275` and `3343` and `remove_registration` are still
unreproduced.

The Wave 02 Cycle 06 branch (`01K7F6H9J2N4Q6S8V0X2Z4B6DE`, in progress)
adds further early returns after the row is begun, in `enable` and in the
saves. A fix made return by return would miss them. The alternative is one
wrapper around each operation body that completes the row on any error
unless the body recorded that it wrote; not yet decided.

## Where A Row Is Left

Line numbers are in `src/repository.rs` at `6cf5d7f`.

- `begin_or_reconcile` refuses a new operation while any other row for the
  repository is not completed (`src/repository/recovery.rs:342-379`).
- `enable`: an invalid or wrong branch, a dirty or conflicted worktree, an
  invalid configuration, an unavailable remote, an invalid identity, and
  every rollback path (`3727-3791`, `3854`, `3963-3980`).
- `create_and_enable`: the row is begun before the target checks; a target
  that is not a directory or is not empty returns at `3564-3596`.
- `add_remote` (`3086-3098`), `remove_remote` (`3142`, `3184`) and
  `set_publication_remote` (`3275`, `3343`): Git errors, including an
  invalid name.
- `remove_registration`: a failure before its transaction (`4278-4313`).
- The three saves and `submit_comment`: only seven error kinds complete the
  row, and only while no step is recorded (`2583-2618`). A repository that
  is not enabled, an invalid configuration, a wrong checked-out branch, a
  dirty configuration path, any Git, I/O, SQLite or busy error, and any
  rejection once the editing context exists, leave it.

## Expected Behavior

- When one of the operations listed above returns an error, its journal row
  was begun by that call, and the call wrote nothing durable or rolled back what it wrote,
  the row is completed. A different operation on the repository then
  proceeds.
- A row stays pending when an effect was made and work remains.
- Whether the call wrote is tracked by the call. It cannot be read from the
  error kind or the recorded step: the same kinds are returned before and
  after a write.
- Creating an editing context as part of a save is not an effect for this
  purpose. Initializing a repository is.
- Refresh and rebuild are not covered: existing tests require their rows to
  stay pending after a failure. A failed refresh or rebuild therefore still
  blocks other operations until it is repeated.
- Consider having each operation report what it wrote to its caller, in its
  error as well as its result. The F2 boundary otherwise has to infer it
  from the journal and from Git, and that inference failed review three
  times during F2 planning.

## Constraints Already Known

- These existing tests require a row to stay pending and must pass
  unchanged:
  - a save that failed after its file write, including the case that
    returns a SQLite error with no step recorded
    (`tests/local_authoring.rs`, near line 6417);
  - a create that failed after the repository was initialized
    (`tests/recovery_foundation_gate.rs`, near line 1388);
  - two failures inside a standalone `prepare_context`
    (`tests/local_authoring.rs`, near lines 5689 and 6466);
  - a failed rebuild and two failed refreshes
    (`tests/recovery_foundation_gate.rs`, near line 951;
    `tests/discovery_rebuild.rs`, near lines 1575 and 2100).
- A lookup of one operation by ID that says absent, pending with its step,
  or completed does not exist; the public reads return pending rows only.
  F2 needs it and adds it in its own plan unless it lands here first.
- `create_and_enable` resumes only while its row is pending. If `enable`
  rolls back inside a create and the row is then completed, a repeat of the
  create with the same operation ID meets a non-empty directory. Decide
  what that repeat returns, and test it.

## To Decide On This Ticket

Three cases remain after the fix. All exist today, and the F2 boundary makes
them reachable by users.

1. A row is pending from the moment it is written, so a process killed at
   any point after that leaves it; the fix completes rows only for calls
   that return. It is cleared only by repeating the same operation with the
   same input, for a save the same body. If the caller no longer has it, or
   someone else has since committed different content to the same file,
   nothing clears the row and the repository stays blocked. Decide whether the library needs a
   way to abandon an operation, and what abandoning does with the
   uncommitted file. The CLI RFC names no such command.
2. A process killed after `enable` or `set_publication_remote` wrote the
   configuration file and before it committed leaves the worktree dirty.
   The repeat is refused as a dirty worktree (`7620-7638`, `3254`), and
   authoring is then blocked.

3. A synchronization that stops with an error after it reserved (a divergent
   remote, a rejected push, a host needing approval, a locked key, a
   transport failure) appears to leave its reservation active: nothing
   finishes the row on those returns (`src/repository/remote/sync.rs`, from
   line 520). While it is active, local operations are refused
   (`src/repository/recovery.rs:280-285`) and other synchronizations are
   busy. Read from the code, not reproduced; F2 Task 13 reproduces or
   refutes it. If it holds it belongs with Wave 02 Cycle 06, not here.

Decide these before Wave 03 C3 makes saves reachable through the CLI.

## Exit Evidence

- For each path listed above, a test that provokes the rejection and then
  runs a different operation with a new ID on the same repository. Each
  fails with `RecoveryRequired` before the fix and passes after. A path
  whose test does not fail first was not defective and is dropped, with a
  note.
- `remove_registration` succeeds after each rejection.
- The tests under "Constraints Already Known" pass unchanged, as does every
  other existing test.
- The four required Devenv checks and the CLI smoke test pass.
