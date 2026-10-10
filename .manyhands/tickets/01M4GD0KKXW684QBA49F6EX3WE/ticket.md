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

At filing, this had not yet been reproduced through the public service against
a real remote without the fixture refresh. The implementation below reproduces
that failure before fixing it.

## Decision

Owner direction, 2026-10-09: after a save commits, refresh the Git index inline
for the specific item only — the paths that save committed (added, and the
removed source on a move) — rather than re-reading the whole index. Entries the
save does not own, including anything else staged or dirty, stay untouched.
Kept out of Cycle 06 because it changes Wave 01 save behaviour.

## Planning questions at filing

- Do the update under the lease the save already holds, and decide what a
  failure after the commit means (the commit is authoritative; a stale index
  entry should be recoverable, not a failed save).
- Unborn-branch and first-save cases, and saves into the primary checkout
  versus an item worktree.
- Update the `local_authoring` status expectations, then remove the fixture
  index refreshes in `tests/remote_synchronization.rs` and the Cycle 06 sync
  tests so they prove the real path.

## Implementation outcome — ready for review

Implemented locally on the ticket branch rebased onto main at `5e4fad6`.
Public save_ticket followed immediately by synchronize_remote failed with
WorktreeNotClean before the fix and now publishes successfully with no fixture
index refresh.

All three authoring checkpoint callers now best-effort refresh only their
added paths and removed move source from the exact committed snapshot, under
the existing save lease. The worktree-specific Git index.lock is acquired
before reading the live index. Detached serialization preserves unrelated
entries, conflict stages, stat fields, assume-unchanged/skip-worktree flags,
and the original index timestamp so unrelated racy edits remain visible.
Staged ancestor/descendant collisions leave the entire live index untouched.

The commit remains authoritative if index repair fails: save retains its
existing success/discovery-refresh reporting, with no rollback or new public
outcome. A later no-change save attempts repair without another commit,
including removal of a moved document's old source. Regression coverage
includes foreign index.lock, no-change repair, moves, owned conflicts, a first
save after unborn-repository enablement, linked-index isolation, and a primary
checkout with a post-commit edit that must remain unstaged.

Updated the local-authoring stale-status characterizations and removed the
save-only refresh workarounds from all identified synchronization/recovery
fixtures, including the additional SSH transport fixture. Legitimate tree
construction and external repair setup remain.

Final verification from the ticket worktree passed:

- `devenv shell -- cargo check --all-features --locked`
- `devenv shell -- cargo fmt --check`
- `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`
- `devenv shell -- cargo test --all-features --locked`

The final test run includes 468 library tests, 131 local-authoring tests, 25
remote merge/recovery SSH cases, 63 remote synchronization SSH cases, and all
remaining integration/SSH/doc-test suites. One existing opt-in characterization
is ignored by default. Local review findings were reproduced and addressed;
final read-only review reported no remaining findings.

Ticket remains open pending owner review approval. Publishing requires owner
authorization and force-with-lease because the already-pushed ticket commit was
rebased. After this fix merges, record its merged commit in the Cycle 07
(`01K7F6H9J2N4Q6S8V0X2Z4B6DF`) entry gate before starting that Cycle.
