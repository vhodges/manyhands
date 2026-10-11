---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4PB7WZN9WNAQ3KXZ19J28KM"
title: "Removing a remote can delete local branches, tags and another remote's settings"
type: "defect"
status: "open"
project: "manyhands"
team: "core"
---

`RepositoryService::remove_remote` hands the removal to libgit2's
`git_remote_delete`, which removes more than the remote. For some
configurations a removal destroys local branches, tags, or parts of a different
remote. Nothing warns the caller.

## Evidence

Found on 2026-10-11 by the reviews of Wave 03 Cycle F2 Task 12 (ticket
`01M4CC0VMR8HSPZXQ1WX41GWVK`, branch `manyhands/ticket/01M4CC0VMR8HSPZXQ1WX41GWVK`). Each case below was reproduced by a
test in a real repository before the F2 binding was changed to refuse it, and
the behaviour was checked against the vendored libgit2 source
(`libgit2-sys-0.18.8+1.9.7`: `remote.c`, `refspec.c`, `config.c`).

- **References outside the remote's own.** libgit2 deletes every reference
  matching the destination of any fetch or push refspec of the remote. With
  `fetch = +refs/*:refs/*` (a mirror) the test repository was left with no
  references at all. With a push refspec onto `refs/heads/*` it lost
  `refs/heads/main`. A push refspec with no destination uses its source as the
  destination. Reflogs go with the references.
- **A sibling remote's configuration.** libgit2 deletes every configuration
  entry matching `remote\.<name>\..+`, unanchored. Removing `origin` beside
  `origin.bak` deleted `origin.bak`'s `url` and `fetch`. Any other entry
  holding that text goes too, for example the settings of a branch named
  `remote.<name>`.
- **A sibling remote's tracking references.** `refs/remotes/team/*` matches
  `refs/remotes/team/fork/main`, so removing `team` beside `team/fork`
  deleted the latter's tracking reference. In the other direction, removing
  `team/fork` deletes a reference that `team` also owns as its branch
  `fork/main`.
- **A remote removed twice.** The function deletes whatever remote has the
  name even under a journal row that already recorded `remote_changed`. After
  an interrupted removal, a remote added by hand under the same name is removed
  by the retry.
- **A half-done removal.** When one of the remote's own entries is set in the
  global configuration (for example `remote.origin.prune`), libgit2 removes
  the references and upstream settings and then fails on that entry
  (`ENOTFOUND`), leaving the removal partly done. Read from the source;
  being reproduced in F2 Task 12's third fix round.

Both sibling names pass `is_remote_name`, so Manyhands itself can create them.

## Who is affected today

Any direct caller of `RepositoryService::remove_remote`, which is public
library API; `tests/recovery_foundation_gate.rs` and
`tests/repository_enablement.rs` drive it. No front end calls it yet.

The F2 `remote remove` command does not reach these cases: its preview works
out what the deletion would destroy and refuses to issue a confirmation
(`invalid_remote`) when anything outside the remote's own section and
tracking references would go. That protects the bound command only, makes such
remotes impossible to remove through Manyhands, and leaves a window inside one
call between its check and the deletion.

## Proposed direction

Stop delegating to `git_remote_delete`. Under the repository lease, remove
exactly: the entries `remote.<name>.<key>` in the repository's own
configuration; the references under `refs/remotes/<name>/` that no other
remote's refspec covers; and the upstream entries that name the remote. Do
none of it under a row that already recorded the step; go to the hand-off.

## Open points for planning

- Whether a remote whose refspecs reach outside its tracking references is
  refused or removed leaving those references alone.
- Whether the F2 binding's refusals are then narrowed, and which of its
  regression tests in `tests/mutation_confirmation.rs` move to the library
  level.
- What to do with `remote.pushDefault` and `branch.*.pushRemote` entries
  that name the remote; today they are left dangling.
