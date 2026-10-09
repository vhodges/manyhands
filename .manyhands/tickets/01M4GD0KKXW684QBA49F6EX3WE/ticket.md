---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4GD0KKXW684QBA49F6EX3WE"
title: "Save must leave the Git index current for the paths it commits"
type: "defect"
status: "open"
project: "manyhands"
team: "core"
---

A normal save commits its owned paths but never rewrites the on-disk Git index
of the worktree it saved into. Afterwards those paths read as modified in both
the index and the worktree, so the worktree is not clean. A synchronize issued
directly after a save is therefore refused as not clean, although nothing is
actually uncommitted.

## Evidence

Found on 2026-10-09 while first running the Wave 02 Cycle 06 Task 5 tests
(ticket `01K7F6H9J2N4Q6S8V0X2Z4B6DE`, branch source `b9bd929`).

- `checkpoint_owned_paths` builds the commit from a fresh in-memory index read
  from the HEAD tree and commits to HEAD; it never writes the repository index
  (`src/repository.rs`, `checkpoint_owned_paths`).
- `require_clean_target` refuses any non-empty status
  (`src/repository/remote/sync.rs`).
- Observed in `released_checkpoint_then_normal_save_continuation_preserves_original_candidate`:
  after `save_ticket`, the saved `ticket.md` reported
  `INDEX_MODIFIED | WT_MODIFIED` and reconciliation returned Recovery.
- Existing fixtures already work around it: the Cycle 05 public fixture in
  `tests/remote_synchronization.rs` reads the HEAD tree into the index and
  writes it after every save, and the Cycle 06 test above now does the same.
- `tests/local_authoring.rs` asserts the current status delta on owned paths
  (`statuses_with_owned_checkpoint_delta`), so the behaviour is characterized,
  not accidental.

Not yet reproduced through the public service against a real remote without
the fixture refresh; writing that failing test is the first step.

## Decision

Owner direction, 2026-10-09: after a save commits, refresh the Git index inline
for the specific item only — the paths that save committed (added, and the
removed source on a move) — rather than re-reading the whole index. Entries the
save does not own, including anything else staged or dirty, stay untouched.
Kept out of Cycle 06 because it changes Wave 01 save behaviour.

## Open points for planning

- Do the update under the lease the save already holds, and decide what a
  failure after the commit means (the commit is authoritative; a stale index
  entry should be recoverable, not a failed save).
- Unborn-branch and first-save cases, and saves into the primary checkout
  versus an item worktree.
- Update the `local_authoring` status expectations, then remove the fixture
  index refreshes in `tests/remote_synchronization.rs` and the Cycle 06 sync
  tests so they prove the real path.
