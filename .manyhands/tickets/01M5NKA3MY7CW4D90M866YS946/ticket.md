---
id: 01M5NKA3MY7CW4D90M866YS946
title: A work context indexes every item its branch changes
type: task
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---

A ticket or document worktree contributes only its own item and that item's
comments to the index (`collect_authoring_item_sources`,
`src/repository/discovery.rs:936`). Every other file in it is ignored, because
a worktree is a full checkout and holds a stale copy of every item. That rule
came from ticket `01M4CKWWRA1DHFPMWKPNK7CQ1G` (2026-10-07).

The rule hides real work. The desktop and the CLI do not see documents a
ticket's branch adds or edits until the branch merges. On 2026-10-23 that
covered the three Cycle and plan documents added on D2's branch
(`01M4CC0VMZ21Z79B09YQRA0P7J`) and, until C2 merged as `d3772f8`, the four
added on C2's.

## Decisions

Product owner, 2026-10-23, in a design session:

- A context contributes its own item and comments, as now, plus every item
  its branch changed. Changed means the file differs from the merge base of
  the context's HEAD with primary's HEAD, whether committed, uncommitted or
  untracked, and also differs from primary's current copy. Unchanged copies
  stay ignored.
- Main wins. An item's own context keeps precedence for that item. For any
  other item primary's copy is effective, and an item that exists only on a
  branch is effective from that branch. Every other copy is a version,
  addressable by item and context.
- Index now, edits later. This ticket makes contributed items visible and
  readable. Editing them gets its own design, and nothing here may rule it
  out.
- The worktree pass is selective: its cost follows what the branch changed,
  not the size of the repository. No per-repository flag for now; add one
  only if the benchmark calls for it.
- Incremental re-indexing of primary is out of scope and gets its own ticket.
- `01M4EPRY203WRKEDQH8403T35B`, "Say when an item's effective copy differs
  from primary", is folded in.
- An item read from its own worktree lists the other items its branch
  changed. A Cycle ticket then shows its design, plan and RFC edits in one
  place for review.
- Sequencing: this lands after the remaining CLI and desktop Cycles and
  before G1.

## Expected Behavior

- An item added on a branch is listed and readable, attributed to that
  context.
- An item a branch changed lists that version, and an item a branch deleted
  lists the deletion; primary's copy stays effective.
- An item read from its own worktree says whether a primary copy exists and
  whether it differs, and names the primary commit compared. A differing
  primary copy is listed among its versions. An item read from primary
  carries the same fields with nothing to report.
- Versions and comparisons are computed by the refresh and stored; a read
  does not scan.
- Comments a branch adds to another item are indexed.
- Problems in files a branch changed are reported on that context. Problems
  in unchanged copies are not.
- Two contexts that add the same new ID are reported as a problem.
- Folders a contributed document needs that primary lacks are listed for
  that context.
- Item and list reads carry the versions as an addition within v1.
  `show --id … --context` reads one version.
- An item read from its own worktree carries `context_changes`: every other
  document or ticket its branch changed or deleted.
- The desktop marks branch-only documents and items changed elsewhere, and
  shows a version read-only. A ticket's "Changed on this branch" section
  opens each of its branch's other changes read-only.
- The poll's materialization gate (`conforming_context_tree`) is unchanged.

## Acceptance Criteria

- Discovery tests for a branch that adds, edits, deletes and moves a
  document; has uncommitted and untracked changes; holds a copy identical to
  primary; merges main; and adds a comment to another item. Also: two
  branches adding one ID, no merge base, an invalid contributed file, and the
  changed-file budget.
- The four cases from `01M4EPRY203W` are distinguishable: a draft that
  differs from primary, a draft identical to primary, an item that exists
  only in its worktree, and a merged worktree whose primary copy has since
  changed. The stored commit is the primary head the refresh observed.
- Read tests for precedence, versions, `context_changes`, `--context` and
  the relationship queries.
- Desktop tests for the marks, the read-only version view and "Changed on
  this branch".
- An index from the current build upgrades in place, and a refresh adds the
  contributions.
- Tests that pin the one-item rule are changed deliberately, each with its
  reason. None is deleted.
- A benchmark of refresh before and after the change, on synthetic
  repositories of 1k, 10k and 50k items with 1, 10 and 50 worktrees, is
  recorded on this ticket.
- The index, Git workflow, CLI contract and desktop RFCs are amended. The
  amendments record that the 2026-10-07 decision is reversed and the
  constraints left for editing contributed items.
- A refresh changes no canonical Markdown or Git state.

## Related

- `01M4EPRY203WRKEDQH8403T35B` is folded in and closes with this ticket,
  after approval.
- `01M5MHQ184BBWEM4N12JNMNVYQ` (marker scan) and
  `01M4B4GSW8J262NA8WPBANSRN1` (nested worktree bases) edit the same
  discovery code. Coordinate if they are active at the same time.
- `01M4EHGE9TMR99VYZC184J9XEC` (entry caps): the changed-file budget follows
  its answer on safety bounds.
- `01M4EHGE60PK1WQ8DM4FRJV9YF` (saves do not update a worktree's Git index)
  slows the status step but does not change its result.
- `01M4EHGE7YFBPRHY20X178572K` (version the index): this ticket adds one more
  column-check upgrade.
- C3 (`01M4CC0VMV81DFRGJ4Z76489XM`) does not wait for this ticket. Comment
  `01M5NVMGZ1BXK1P15RZNNYX06B` on C3 says what C3 should keep open for it.

## Design and plan

- Design: `docs/plans/2026-10-23-multi-item-work-contexts-design.md`,
  approved 2026-10-23.
- Plan: `docs/plans/2026-10-23-multi-item-work-contexts-implementation.md`,
  approved 2026-10-23. The execution method is chosen when work starts.
