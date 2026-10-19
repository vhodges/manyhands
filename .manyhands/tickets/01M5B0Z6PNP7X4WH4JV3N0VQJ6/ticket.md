---
manyhands_managed: true
manyhands_kind: ticket
id: "01M5B0Z6PNP7X4WH4JV3N0VQJ6"
title: "Enablement must leave the Git index current for the configuration it commits"
type: "defect"
status: "open"
project: "manyhands"
team: "core"
---

Creating or enabling a repository (`repo create`, `repo enable`) commits
`.manyhands/config.toml`, but it never updates the on-disk Git index of the
worktree. Afterwards the file reads as `INDEX_DELETED | WT_NEW`: the index
lacks it and the worktree has it untracked, although nothing is actually
uncommitted. The worktree is therefore not clean. The next authoring command
in that repository, such as `ticket create` or `document create`, is refused
with `worktree_not_clean`.

This affects users of both front ends. In the CLI, `repo create` followed by
`ticket create` fails. The desktop will hit the same failure once it creates
items after onboarding (Wave 03 D1 onboarding, then D2 and later).

This is the enablement sibling of the save defect `01M4GD0KKXW684QBA49F6EX3WE`
("Save must leave the Git index current for the paths it commits", closed).

## Evidence

- `initialization_tree` (`src/repository.rs`) builds the initialization
  commit's tree from a fresh in-memory `Index::new()`, filled from HEAD's tree
  plus the configuration blob. It never reads or writes the repository index.
  The caller around `write_configuration_atomically` then commits that tree to
  HEAD.
- The library's own tests work around it, which shows the behaviour is known
  but untracked:
  - `tests/support/items.rs::commit`: "Enabling commits the configuration
    without refreshing this index." It re-adds `.manyhands/config.toml` to the
    index before each commit.
  - `tests/discovery_rebuild.rs` (near line 3557): "Enablement commits the
    configuration without refreshing this index."
  - `src/repository/comment_publication_tests.rs` (near line 104) does the same
    re-add.
- Found again on 2026-10-19 by Wave 03 D1 Task 5
  (`01M4CC0VMY5RBMP70QVBXWXM6A`). The desktop test fixture
  `src/desktop/test_support/fixtures.rs` (D1 branch) has to reset the index to
  HEAD after `RepoCreate`. Without that, `TicketCreate` and `DocumentCreate`
  answer `worktree_not_clean` on a freshly created repository.

## Expected behaviour

After a successful `repo create` or `repo enable`, the worktree's Git index
matches the commit enablement made for the configuration file. Git status
reports nothing for `.manyhands/config.toml`, and an immediately following
authoring command is not refused as not clean. As with `01M4GD0K`, refresh only
the path enablement committed, not the whole index, so other staged or
unstaged user changes are preserved.

## Acceptance criteria

- A library test shows that `repo create` on an empty directory, and
  `repo enable` on an existing repository, each leave `git status` clean for
  `.manyhands/config.toml`. A following `ticket create` succeeds.
- Other entries in the user's index (staged changes to unrelated paths) are
  unchanged by enablement.
- Retry and replay of an enablement request (the F2 request record), and the
  `partial` recovery path after `AfterRepositoryInitialization`, also leave
  the index current.
- The test workarounds listed above are removed, or each is justified where a
  test still needs it.
- Shared-library change under Wave 03 track rule 5: reviewed and merged to main
  on its own. D1 then merges main and drops its fixture's index reset.
