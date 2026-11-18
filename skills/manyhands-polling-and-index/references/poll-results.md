# Reading a poll's targets

`poll once` answers `ok` (exit 0) whenever the poll itself finished, and reports one target per ref it looked at. Read each `disposition`. The sentence in the third column is the guidance Manyhands itself prints for it. "Run here" says whether the disposition was produced by a real run with two clones of one remote.

| `disposition` | Run here | What it means | What to do |
| --- | --- | --- | --- |
| `current` | yes | Local branch equals the remote's. | Nothing. |
| `fast_forwarded` | yes | "Clean local state was fast-forwarded." The branch was clean and strictly behind, and now holds the remote's commits. | Read the item again for the new content. |
| `materialized` | yes | "The remote context was materialized locally." A new item from someone else, with its own branch and worktree. | `ticket list` or `document list` now shows it. |
| `local_ahead` | yes | "Publish local commits with deliberate synchronization." You have commits the remote does not. | `item sync` publishes them ([sharing and sync](../../manyhands-sharing-and-sync/SKILL.md)). |
| `unpublished` | yes | "The remote context is not published yet; item sync publishes it." The item exists only here. | `item sync` when the person wants it shared. |
| `diverged` | yes | "Resolve divergent history with deliberate synchronization." Both sides committed. Nothing was changed. | `item sync` merges; a conflict is for the person ([sharing and sync](../../manyhands-sharing-and-sync/SKILL.md)). |
| `dirty` | yes | "Save or otherwise handle local changes before polling again." The item's worktree has uncommitted or untracked files, so the poll did not move it. | Save the item through Manyhands, or ask the person about files you did not make. |
| `closed` | yes | "Local primary holds the ticket's closure; use context retire to check and remove the old local context." | "Completed items" in the skill: show the person, then `context retire`. |
| `promoted` | yes | The document equivalent of `closed`. | The same. |
| `history_unknown` | yes | "Publication history is unknown; inspect remote state before proceeding." Seen after the database was rebuilt, for an item with no remote branch. | [Recovery](../../manyhands-recovery/SKILL.md): `remote history`, then the person decides. |
| `conflicted` | no | "Resolve the conflict or in-progress Git operation before polling again." | Find the unfinished operation ([recovery](../../manyhands-recovery/SKILL.md)). |
| `inaccessible` | no | "Restore the expected checked-out branch and registered worktree before polling again." | Tell the person; do not edit worktrees by hand. |
| `nonconforming` | no | "Repair the remote context's canonical tree before materialization." The remote's item is not a valid Manyhands item. | Tell the person; it is someone else's branch. |
| `blocked_by_local_state` | no | "Inspect the existing local branch or path before materialization." A local branch or path is in the way of a new item. | Tell the person which. |
| `materialization_incomplete` | no | "Inspect and repair the retained partial materialization." | Tell the person. |
| `malformed` | no | "Repair the remote context's branch name." | Tell the person. |
| `remotely_deleted` | no | "The remote context is absent; local state was preserved." The branch vanished from the remote. | Nothing is lost locally. For a published item whose branch someone deleted, see `remote_branch_deleted` in [sharing and sync](../../manyhands-sharing-and-sync/SKILL.md). |
| `not_reached` | no | "This target was not reached; retry the poll when ready." | Poll again with a new request. |

A poll writes only local branches and worktrees that are clean and behind, and new materialized items; the other rows leave things as they were. A target's `before_oid` and `after_oid` are equal unless it moved.

The answer also has `completion` (`completed`, `interrupted` or `cancelled`) and `completed_at`. The human output prints the same targets, each followed by its guidance sentence.
