---
slug: vh-mrse6
id: 01M669FJR27FRHCNPWAMD6VH4V
title: Remote selection must leave the Git index current for the configuration it commits
type: defect
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
`remote select` commits `.manyhands/config.toml` but never updates the
worktree's on-disk Git index. After it, the file reads as
`INDEX_MODIFIED | WT_MODIFIED`: the index still holds the configuration from
before the selection, while HEAD and the worktree hold the new one. Nothing is
actually uncommitted, but the worktree is not clean. The next `repo sync` is
refused with `worktree_not_clean`, every effect `not_requested`, and no
recovery action.

This affects any fresh repository set up in the usual order: `repo enable`,
then `remote select`, then `repo sync`, `item sync` or `comment add`. It is the
remote-selection sibling of the closed save and enablement defects
`01M4GD0KKXW684QBA49F6EX3WE` and `01M5B0Z6PNP7X4WH4JV3N0VQJ6`. The enablement
fix's regression coverage includes a remote selection right after
enablement, but not the status after the selection.

## Evidence

- `set_publication_remote_recorded` (`src/repository.rs`) commits the tree
  that `configuration_tree` builds. That function fills a fresh in-memory
  `Index::new()` from HEAD's tree plus the new configuration blob and writes
  the tree. It never reads or writes the repository index.
- Found on 2026-10-30 by Wave 03 C4 Task 12a
  (`01M4CC0VMWEQQ9G4W0DAG7ADPY`) in a two-clone SSH journey run through the
  binary:
  1. Clone A runs `repo enable`, and its status is clean.
  2. `remote select` succeeds and commits the configuration.
  3. `.manyhands/config.toml` is `INDEX_MODIFIED | WT_MODIFIED`.
  4. `repo sync --json` exits 3 with `worktree_not_clean`.
  5. A second clone, made from a primary that already holds the selection,
     answers `already_applied` to `remote select`, and is not affected.
- C4 does not fix it, because it is outside C4's library scope. C4's
  two-clone fixture seeds the server's primary with the publication remote
  already selected, so each clone's `remote select` is a no-op.

## Expected behaviour

After a successful `remote select`, the worktree's Git index matches the
commit it made for the configuration file. Git status reports nothing for
`.manyhands/config.toml`, and an immediately following synchronization is
not refused as not clean. As with `01M5B0Z6`, refresh only the path the
selection committed, so other staged or unstaged user changes are preserved.

## Acceptance criteria

- A library test shows that `repo enable` followed by `remote select`
  leaves `git status` clean for `.manyhands/config.toml`, and that a
  following synchronization is not refused as `worktree_not_clean`.
- Other entries in the user's index are unchanged by the selection.
- Retry and replay of a `remote select` request, and its recovery after a
  stop between the commit and the index update, also leave the index
  current.
- Shared-library change under the Wave 03 track rule: reviewed and merged to
  main on its own. C4 then merges main. Its two-clone fixture may keep its
  seeded selection, or switch back to selecting in each clone.
