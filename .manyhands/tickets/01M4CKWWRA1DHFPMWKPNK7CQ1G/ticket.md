---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4CKWWRA1DHFPMWKPNK7CQ1G"
title: "Discovery must index one effective copy per item across item worktrees"
type: "defect"
status: "open"
project: "manyhands"
team: "core"
wave: "03"
---

Discovery records every item it finds in each item worktree's checkout, not
only the item that worktree was created to edit. Once a repository has two
item worktrees, the index has no single effective copy of an item, and the
snapshot read can fail outright. This blocks Wave 03 F1
(`01M4CC0VMQ7R15A7M9SPN3KB67`) and every other consumer of the index.

## Evidence

Found by reading the source at main `ceb1be4` during F1 planning. It has not
been reproduced by a test; writing that test is the first step here.

- `observe_items` returns every valid document and ticket in a checkout
  (`src/repository/discovery.rs:303`), and each active context is observed
  with it (`src/repository/discovery.rs:270-298`).
- Persisting the primary context skips any item present in any active context
  (`src/repository.rs:4945-4960`, `5002-5005`), so an item's primary row is
  dropped whenever some worktree holds a copy, however stale.
- `read_repository_snapshot` fails with "stored item ID is not globally
  unique" when two contexts hold the same item
  (`src/repository.rs:5349-5356`).
- An item worktree is a full checkout, so it holds every managed document and
  every ticket that existed on primary at its branch point. Two item worktrees
  therefore share items in any real repository, including this one.
- Primary validation problems are still stored for items excluded from primary
  (`src/repository.rs:5043-5046`).

## Decision

The product owner decided the rule on 2026-10-07, recorded in the
[F1 Cycle document](../../../docs/Cycles/wave-03-foundation-01-read-boundary-and-results.md)
on the F1 branch:

- An item worktree contributes only the item its branch identifies, with the
  comments whose `item_id` is that item.
- Every other item is read from primary.
- Accepted cost: a managed document created by hand inside a ticket's branch
  is not listed until that branch merges.

## Expected Behavior

- The index holds exactly one row per item ID.
- An item with its own active context is attributed to that worktree; every
  other item is attributed to primary, whatever copies other worktrees hold.
- Problems follow the same split: an active context reports problems only for
  its identified item's files, and primary reports the rest.
- Primary attributes a file to an item only by that item's ticket and comment
  directories. A primary document that cannot be parsed carries no ID, so it
  stays a primary problem even when the item it was has its own context
  (settled in code review, 2026-10-07).
- Refresh, rebuild and the snapshot read behave identically with one, two or
  many item worktrees.

## Acceptance Criteria

- A test first reproduces the failure: two item worktrees that both contain a
  third item, refreshed, then a snapshot read.
- After the fix, that snapshot lists the third item once, from primary, and
  each worktree's own item once, from its worktree, with its comments.
- A stale copy of an item in another item's worktree changes nothing in the
  snapshot, including when that copy is malformed.
- An item deleted on primary, still present in an unrelated worktree's
  checkout, is not listed.
- Rebuild from a deleted index gives the same result as refresh.
- Existing discovery, authoring and recovery tests pass unchanged, or each
  changed expectation is explained in the review.
- No canonical Markdown or Git state is changed by the fix.

## Related

- Open defect `01M4B4GSW8J262NA8WPBANSRN1` (nested worktree bases) touches the
  same scanner. The two should be reviewed for overlap; they are separate
  faults.
- F1 implementation must not start until this ticket is merged to main.
