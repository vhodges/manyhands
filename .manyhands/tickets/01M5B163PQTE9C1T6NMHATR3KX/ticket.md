---
manyhands_managed: true
manyhands_kind: ticket
id: "01M5B163PQTE9C1T6NMHATR3KX"
title: "Item reads must report the item's context problems instead of not found"
type: "gap"
status: "open"
project: "manyhands"
team: "core"
---

The desktop RFC says that "duplicate or mismatched contexts show a recovery
view, never a context chooser"
(`docs/RFC/desktop-information-architecture-and-editor.md`). To build that
view, a front end has to learn, from the item read, that this item has a
context problem. Today it cannot.

When an item's context is duplicated or mismatched, for example two item
worktrees claiming one ULID:

- `show_item` answers `item_not_found`, with no recovery action;
- `show_path` on the copy answers `invalid_path`;
- the item disappears from `list_documents` and `list_tickets`;
- only `index_status` reports the `context_problem`, at
  `.manyhands/worktrees/<id>`.

A user who opens the item, or who finds it by ULID, is told it does not exist.
In fact it exists and needs recovery. A front end could only find the
problem by guessing from the library's worktree path layout.

## Evidence

- `src/repository/read/items.rs`, `CONFORMITY_PROBLEMS`: "a problem about a
  branch, a context, a comment or an unreadable file is the index status
  read's to report".
- `src/repository/read/dto.rs`, `IndexStatusDto.problems` (near line 644):
  "This is the only read that reports a problem with no path, or …".
- Context problems are produced by discovery (`src/repository/discovery.rs`,
  `context_problem`).
- Reproduced on 2026-10-19 by Wave 03 D1 Task 6
  (`01M4CC0VMY5RBMP70QVBXWXM6A`), using the duplicate-context recipe from
  `tests/discovery_rebuild.rs` (D1 fixture
  `src/desktop/test_support/fixtures.rs::commit_duplicate_context`) on a
  document in its own worktree, followed by a refresh. The results were as
  described above. The D1 desktop therefore shows "not found" with Refresh for
  this case. Its recovery view, keyed on the item's `problems`, is only covered
  by a model test.

## Expected behaviour

An item read by ULID whose identity is known but whose context is ambiguous,
duplicated or mismatched does not answer a plain `item_not_found`. It answers
either:

- the item with its context problems in `problems`, with codes, paths and
  fixed guidance, and with no chosen context presented as authoritative; or
- a distinct typed answer, such as `context_conflict`, carrying the problem
  codes, the paths involved and recovery actions.

The choice between the two belongs to the shared-library design. It must keep
the RFC rule that no context is chosen for the user, and the F1 rule that the
read never fabricates an effective copy.

Lists keep the entry visible with its problem rather than dropping it, as they
already do for nonconforming entries.

## Acceptance criteria

- A library read test reproduces a duplicate context and a mismatched
  context for a ticket and for a document. `show_item` by ULID reports the
  context problem (codes, paths, guidance) instead of `item_not_found`. A
  front end can tell this answer apart from a deleted item.
- `list_tickets` and `list_documents` keep the entry visible with its problem
  under every filter.
- `index_status` still reports the problem.
- The read schemas and fixtures (`schemas/v1`, `tests/fixtures/read_v1`) are
  updated if a DTO or code changes, and the CLI contract is reviewed for the
  same case.
- Shared-library change under Wave 03 track rule 5: reviewed and merged to
  main on its own. The desktop (D1, or the next desktop Cycle) then enables its
  recovery-view test against the real read.
