---
slug: vh-mpvkc
deps:
- 01M7RDVNZDCCVSMWHDY9503852
id: 01M7ZT4NBNRVNY3N90WGCCFM7R
title: Comment add, sync and close read every file of the item's Git tree to validate it
type: defect
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Depends on vh-f3wts (01M7RDVNZDCCVSMWHDY9503852). Start after its Task 18 (the tree-validation cache) has landed and its Phase 2 profile is recorded: both change `validate_context_tree_items` in src/repository.rs, and the numbers below should be re-taken after the cache. Line numbers below are on the vh-f3wts branch at 4c6dbeec.

Today every lease-held target check, `comment add`, `item sync` and `ticket close` calls `canonical_tree_sources` (src/repository/integration/validate.rs:412), which walks the whole Git tree of the item's branch and opens every blob. libgit2 inflates and SHA-1-verifies each one, and the code copies and normalises every UTF-8 file to look for a `manyhands_managed` marker. On this repository that is 3,477 blobs and 67.1 MB per walk, of which only 1,144 blobs and 15.2 MB are canonical sources. In the vh-f3wts Phase 1 profile (release build, replica, localhost SSH) it is 52% of the samples before `comment add` prints its first publication progress line (0.94 s), 45% of `item sync` and 35% of `ticket close --confirm`.

The walk exists for a reason that no longer applies to most of its callers. It was written on 2026-10-09 (489322fe, Cycle 6) so that `validates_external_integration` could compare the problems of a whole outside-repair merge with its baseline and reject new diagnostics, which needs every Markdown file that declares itself managed. Cycles 8, 9 and 10 reused it as the shared tree reader: `conforming_context_tree` (remote/poll.rs:951), `result_document_path` (promotion/observe.rs:1005), `result_ticket` (closure/observe.rs:506) and, from c0e97265, `validate_context_tree_items` (repository.rs:10767), which replaced a filesystem scan of the canonical directories to meet Cycle 9 decision 12 (no discovery scan under the lease). The filesystem scan it replaced read only the canonical directories; the tree read lost that bound. Those four callers keep only items at canonical paths: `parse_item` rejects any other path, and they use only the validated items, never the problem list. The blobs outside canonical paths are read and thrown away.

Proposed change: add a bounded tree reader that walks only `docs/`, `.manyhands/tickets/` and `.manyhands/comments/` and opens only blobs at canonical paths (`canonical::item_path_kind`), and use it in those four callers. Keep `canonical_tree_sources` unchanged for `validates_external_integration`. Keep a cheap `odb.exists` check on the other entries so a missing object still fails as today, or record the decision to drop that check. Do not change the decisions, result codes or messages: `MismatchedAuthoringContext`, `TargetNotMaterialized`, `Finding::Mismatch`, `PollDisposition::Nonconforming` and `DocumentInvalidAfterIntegration` stay as they are.

The property it must keep is the one in docs/RFC/git-workflow-and-conflict-recovery.md:92: a branch that lacks the identified conforming item is not an editable context. That means the tree holds a conforming item of the context's kind with the context's ULID, a symlink or gitlink cannot satisfy it, no other item with that ULID exists, and the `.manyhands/worktrees/` and gitlink checks of the poll and promotion paths still run on the tree's entries. It must still run with no filesystem scan under the lease.

What vh-f3wts already covers: its Task 18 caches `validate_context_tree` by tree OID, item kind, item ID, configuration blob and build, which removes repeated validation of the same tree. It does not make the first walk of a new tree cheaper (a comment always makes a new tree), does not apply to `result_ticket`, `result_document_path` or `conforming_context_tree`, and an in-process cache cannot help a one-shot CLI command. Its Tasks 18, 21 and 22 reduce how many walks happen, not what one costs.

Second part, only if the profile after vh-f3wts Phase 2 still shows the walk above 10% of a command: validate a new tree by `diff_tree_to_tree` against its first parent or the primary, keyed by tree OID and a predicate version, so a `comment add` re-reads one blob. Decide in this ticket whether that cache is stored on disk, because a per-process cache does not help the CLI.

Estimated saving (an estimate, not a measurement): 67 MB at 0.4-0.7 GB/s is 100-170 ms per walk; the bounded reader saves about 75% of that per walk, so about 75-125 ms per command once the Phase 2 cache leaves one walk, more where several walks remain.

Acceptance criteria:

- A table-driven library test checks that the bounded reader and the whole-tree reader give the same `validate_context_tree_items` answer on a fixture with: a missing target; a duplicate ULID across a document, a ticket and a comment; a kind mismatch; a symlink and an executable canonical blob; a marker-bearing `.md` outside the canonical paths; and a non-UTF-8 path.
- A library test shows non-canonical blobs are not opened: a tree whose out-of-path entries point at objects missing from the object database still validates (or fails only through the chosen `exists` check).
- `validates_external_integration` and its tests are unchanged.
- The existing tests for the four callers pass, selected by name per AGENTS.md. No SSH case is added.
- On the replica, `scripts/measure-writes` before and after records `comment add` up to its first progress line, `item sync` and `ticket close --confirm`, and the sample share in the walk; blob reads fall to canonical paths only (about 1,144 here).
- The ticket comments record which tests ran at each checkpoint.