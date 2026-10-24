---
title: "Multi-Item Work Contexts Design"
date: 2026-10-23
status: "draft, awaiting owner review"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5NKGMS7RBYRT9ZQDKSC2FY0"
---

# Multi-Item Work Contexts Design

Ticket: `01M5NKA3MY7CW4D90M866YS946`, "A work context indexes every item its
branch changes". It folds in `01M4EPRY203WRKEDQH8403T35B`, "Say when an
item's effective copy differs from primary".

## Problem

An authoring context (branch `manyhands/<kind>/<ULID>`, worktree
`.manyhands/worktrees/<ULID>/`) contributes only its own item and that item's
comments to the index. `collect_authoring_item_sources`
(`src/repository/discovery.rs:936`) reads nothing else, because a worktree is
a full checkout holding a stale copy of every item. That rule was decided on
2026-10-07 in ticket `01M4CKWWRA1DHFPMWKPNK7CQ1G`.

It hides real work. Cycle branches add Cycle documents, plans and RFC edits
alongside their ticket. On 2026-10-23 the desktop did not show the three
documents added on D2's branch, nor, until C2 merged as `d3772f8`, the four
added on C2's. The CLI cannot see them either, so neither can an agent.

## Decisions

Product owner, 2026-10-23:

1. **Contribution.** A context contributes its own item and comments, as
   now, plus every item its branch changed. Unchanged copies stay ignored.
2. **Main wins.** An item's own context keeps precedence for that item. For
   any other item primary's copy is effective. An item that exists only on a
   branch is effective from that branch.
3. **Index now, edits later.** This design makes contributed items visible
   and readable. Editing them (Level 2) gets its own design. Nothing here may
   rule it out: every copy is addressable by item and context.
4. **Selective worktree pass.** A context's cost follows what its branch
   changed, not the size of the repository. No per-repository flag for now;
   add one only if the benchmark calls for it.
5. **Primary stays a full scan.** Incremental re-indexing of primary is out
   of scope and gets its own ticket.
6. **Fold in `01M4EPRY203W`.** The comparison of an item's worktree copy with
   its primary copy uses the same machinery and changes the read contract
   once.

## Terms

- **Primary**: the primary worktree's context.
- **Merge base**: the merge base of the context's HEAD and primary's HEAD.
  Merging main into a ticket branch moves it forward.
- **Changed path**: a path under `docs/`, `.manyhands/tickets/` or
  `.manyhands/comments/` whose bytes on disk, committed or not, differ from
  the merge base's blob at that path, or that the merge base lacks or the
  disk lacks.
- **Version**: a copy of an item in some context that is not its effective
  copy. It is addressed by (item ID, context).

## Collecting a context's changes

For each active context, after its own item is read as today:

1. Find the merge base with git2. If there is none (primary has no commit,
   or the histories are unrelated), record a context problem
   (`context_changes_unavailable`) and contribute the own item only.
2. Gather candidate paths: a tree-to-tree diff from the merge base to HEAD,
   plus the worktree's status (staged, unstaged and untracked, but not
   ignored), both limited to the three managed path prefixes.
3. Keep a candidate only if it is a changed path: its bytes on disk differ
   from the merge base's blob. This drops a file the branch changed and then
   changed back.
4. A changed path that exists on disk is read and parsed. A changed path
   missing from disk is a deletion: its item ID comes from the merge base's
   blob. A deleted path whose merge-base blob does not parse is ignored.
5. Changed paths are taken in path order up to a budget of 1,024 per
   context. Past it, the context records `context_changes_over_budget` and
   contributes nothing more. The number follows ticket
   `01M4EHGE9TMR99VYZC184J9XEC`'s answer on safety bounds.

No other file in the worktree is read. The scan for managed files outside the
canonical paths stays primary-only, so worktrees never walk `target/`.

### Validation

Validation's only cross-item checks are a comment's item, its parent and
cycles (`src/canonical.rs:586-660`), plus duplicate IDs. The context is
validated over a small set:

- the own item and its comments, as today;
- the changed files;
- for each changed comment, its item's file and that item's comments, taken
  from primary's sources, which the same refresh already holds in memory.

A primary source is left out when the branch changed or deleted its path or
has a changed copy of its ID, so the branch's view replaces it.

Validation problems are kept only for changed paths and the own item's paths.
Problems in primary-sourced files are primary's to report.

### Versions and duplicates

Each valid contributed item with ID X at path P is classified:

- **Edit or move.** The merge base had X at P, or the branch deleted a path
  whose merge-base copy had ID X. If primary has X, this copy is a version.
  If primary does not, primary deleted it after the branch point. The copy is
  then branch-only and effective from the branch.
- **Duplicate.** Otherwise, if primary has X, the branch introduced an ID
  that primary also has. The copy is a version carrying a `duplicate_id`
  problem that names primary's path.
- **New.** Otherwise it is branch-only.

A changed file whose bytes equal primary's current copy of X at the same path
is not contributed.

Two contexts that both hold a branch-only copy of X: the context whose branch
ULID sorts first is effective. Every other copy is a version with a
`duplicate_id` problem naming the effective context.

### Comparison with primary (folded in)

For the own item, and for every contributed item that primary also has, the
refresh records whether primary's copy is absent, identical or different, and
the primary HEAD OID it compared against. A refresh observes primary and
every worktree together (`src/repository.rs:1460`) but writes each context in
its own transaction, so the stored OID says which primary the answer refers
to.

Each context also records whether its HEAD is merged into primary's HEAD.
This separates `01M4EPRY203W`'s fourth case (a merged worktree whose primary
copy has since changed) from a draft that differs.

## Index

The index upgrades in place with the existing column-check pattern
(`src/repository/discovery.rs:1855`):

- `discovered_items.role`, one of `own`, `changed` or `comments`, defaults to
  `own`, which is true of every existing row. A `comments` row holds the
  branch's changed comments on an item the branch did not change. It exists
  because `discovered_comments` hangs off an item row, and it is never a copy
  of the item.
- `discovered_items.primary_state` (`absent`, `identical`, `different`, null
  for primary's rows) and `discovered_items.primary_compared_oid`.
- `contexts.merged_into_primary`, null for primary.
- A new `discovered_deletions (context_id, item_id, kind, canonical_path,
  UNIQUE(context_id, item_id))` table.
- `discovered_folders` is already keyed by context. Worktree contexts now
  store the folders their contributed documents need that primary lacks.
  Git does not track empty folders, so none are added for them.

The upgrade sets `refresh_required`. The index stays disposable: deleting it
and rebuilding gives the same result. Ticket `01M4EHGE7YFBPRHY20X178572K`
(version the index) replaces column checks later; this design adds one more.

The snapshot check (`src/repository.rs:6813`, "stored item ID is not
globally unique") becomes: unique within each context, and exactly one
effective copy per item ID.

## Effective copy

The read layer (`src/repository/read/items.rs:1530-1660`: `is_own_worktree_row`,
`row_rank`, `effective_rows`, `effective_copy`) picks one effective copy per
item ID:

1. A row with role `own` in an active context for that item.
2. Otherwise primary's row.
3. Otherwise a `changed` row, from the context whose branch ULID sorts first.

`comments` rows never compete. Every other `changed` row, and primary's row
when rule 1 applied and primary's copy differs, is a version. Comments follow
the same rule per comment ID, where "own" means the own context of the
comment's item.

The "index is behind" signal treats a `changed` row as legitimate. Today it
treats any item found only in another item's worktree as stale.

The relationship queries (ready, blocked, plan, critical path) use effective
copies only. A ticket added on a branch therefore takes part. A `deps` edit
made on another branch does not, until it merges.

## Reads

Additions within v1, which the CLI contract's 2026-10-08 rule permits. The
closed JSON schemas are updated in place:

- `ItemDto.versions`: one entry per version, with its `context`, `path` and
  `change` (`added`, `modified`, `moved`, `deleted` or `primary`) plus its
  problems. `primary` marks primary's differing copy of an item read from its
  own worktree. An item read from primary carries an empty list unless a
  branch changed it.
- `ItemDto.primary_comparison`: `{state, compared_head_oid}` for an item
  whose effective copy is in a worktree; null for an item read from primary.
- `ItemContextDto.merged_into_primary`: null for primary.
- `CommentDto.context`: where the comment's effective copy was read from,
  which can now differ from the list's `context`. Also `CommentDto.versions`,
  shaped like an item's.
- `DocumentFolderDto.context`: null for primary's folders.

List reads carry `versions` from the index, so a list can say "also changed
in". Show reads it from the index too; a read does not scan.

CLI: `document show` and `ticket show` accept `--context` with `--id`, as they
already do with `--path`. `--id X --context W` returns X's copy in context W,
whether effective or a version. A context holding no copy of X, including
one that deleted it, returns the existing not-found result; the deletion is
visible in `versions`.

## Desktop

- The document tree lists branch-only documents, with a mark naming their
  context, and their folders. The tree already places documents by the
  folder read (`src/desktop/model/documents.rs:1-8`), so folders come from
  the read rather than being invented.
- An item with versions shows "also changed in" with each context.
- An item read from its worktree shows whether it differs from primary and
  the commit compared.
- Opening a version shows it read-only, labelled with its context. Editing
  is D3's and Level 2's.

## Unchanged

- The poll's materialization gate, `conforming_context_tree`
  (`src/repository/remote/poll.rs:868`), checks that a remote commit is a
  conforming context for its own item. Contributed files neither satisfy nor
  fail it. A contributed file with problems becomes an index problem after
  materialization.
- Scoped checkpoints stage only the event's own paths (Git RFC, "Scoped
  Checkpoints"). Level 1 indexes uncommitted and untracked files, so it needs
  no change there.
- Activity timestamps use each worktree's own history already.
- No refresh writes canonical Markdown or changes Git state.

## Cost and benchmark

Per context, a refresh now does one merge-base, one tree-to-tree diff, one
status limited to the managed paths, reads of the changed files, a small
validation, and byte comparisons with primary. The status makes one metadata
check per managed file in each worktree. That is the one part that grows with
repository size, and with the number of worktrees.

Primary's full scan is unchanged and remains the main cost at scale.

The ticket records a benchmark, since the target scale is not yet set.
Synthetic repositories of 1k, 10k and 50k items, with 1, 10 and 50
worktrees each changing 10 files, are timed before and after the change on
the same machine. The results feed the scale discussion in
`01M4EHGE9TMR99VYZC184J9XEC` and decide whether a per-repository flag is
needed.

Ticket `01M4EHGE60PK1WQ8DM4FRJV9YF` (saves do not update a worktree's Git
index) makes the status step hash files instead of trusting cached metadata.
It slows that step but does not change its result.

## Testing

New tests:

- Discovery, a branch that:
  - adds a document (branch-only, effective from the branch);
  - edits a primary document (version, primary effective);
  - deletes one (deletion recorded);
  - moves one (version at the new path);
  - has staged, unstaged and untracked changes;
  - changes a file and changes it back (not contributed);
  - holds a copy identical to primary (not contributed);
  - merges main, after which an item main changed is not contributed;
  - adds a comment to an item it did not change (a `comments` row).
- Discovery, also:
  - an ID introduced on both primary and a branch, and on two branches;
  - no merge base;
  - an invalid contributed file (a problem on that worktree; the context
    stays active);
  - the changed-file budget;
  - a worktree whose branch changed nothing reads only its own item's files.
- Comparison: the four cases from `01M4EPRY203W` are distinguishable, and the
  stored OID is primary's head at the refresh.
- Reads: precedence, `versions`, `primary_comparison`, `--id … --context`,
  per-comment context, and relationship queries.
- Desktop: the branch mark, "also changed in", the primary comparison, and
  the read-only version view.
- Upgrade: an index from the current build opens, its rows read as `own`,
  and a refresh adds contributions.
- The poll tests stay unchanged.

Tests that pin the one-item rule are changed deliberately, each with its
reason in the commit, and none is deleted. They are at:

- `tests/discovery_rebuild.rs:736`, `:3668-3945`;
- `tests/read_items.rs:819`, `:2813`, `:4045`;
- `tests/read_comments.rs:484`, `:1247`;
- `tests/read_relationships.rs:290`;
- `tests/local_authoring.rs:1549-1659`, `:2005`, `:2125`.

The plan confirms the list against current main.

## RFC amendments

- **Index** (`repository-index-persistence-and-refresh.md`, "Index-Only
  Refresh Algorithm"):
  - a context contributes its item and its changed items;
  - the diff selects contributions, and a full per-context scan of primary
    remains;
  - the effective-copy rule, versions, deletions and the primary comparison;
  - the 2026-10-07 decision (`01M4CKWWRA1D`) is reversed.
  The rule that consumers are never required to choose a context still
  holds, because versions are read-only and the effective copy is
  deterministic.
- **Git workflow** ("Context Branches and Worktrees"): one context owns one
  item, but its branch may carry changes to others, and these merge with it.
- **CLI contract**: `--context` with `--id`, and the new fields.
- **Desktop**: the branch mark, "also changed in", the primary comparison and
  the read-only version view.

## Constraints left for Level 2

- An edit addresses (item, context).
- Scoped checkpoints must widen to stage a contributed item's paths.
- C3's mutation verbs (`01M4CC0VMV81DFRGJ4Z76489XM`) should take an optional
  `--context`, so this does not become a breaking change later.
- Letting a consumer edit a version amends the index RFC's no-choice rule.
- Two versions of one item meet at merge time, through the existing
  synchronization and conflict recovery.

## Out of scope

- Editing contributed items (Level 2).
- Incremental re-indexing of primary.
- A per-repository flag.
- Scanning worktrees for managed files outside the canonical paths.
- Pruning merged worktrees.

## Coordination

- `01M5MHQ184BBWEM4N12JNMNVYQ` (marker scan) and
  `01M4B4GSW8J262NA8WPBANSRN1` (nested worktree bases) edit
  `src/repository/discovery.rs`. This design does not touch the marker scan.
- D2 (`01M4CC0VMZ21Z79B09YQRA0P7J`) is active in `src/desktop/`. Check
  `git merge-tree` against its branch before desktop work begins.
- Land before C3 plans its authoring verbs.
