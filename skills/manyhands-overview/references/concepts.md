# How Manyhands fits together

## Local-first and Git-backed

Manyhands stores everything in a Git repository on the person's machine. It works with no network and no server. A remote repository is optional: it is how work is shared with others, and the CLI reports `publication_remote_required` when you ask to share without one. The person's files and history are ordinary Git, so a person can always inspect them with Git.

## Items

An item is a managed document or a ticket. Each has a ULID, its identity for good, and a path in the repository: a ticket at `.manyhands/tickets/<id>/ticket.md`, a document under the folders the person chose (such as `docs/new.md`). You find an item by ID with `ticket show --id <id>` or `document show --id <id>`. A ticket also has a short code, its `slug`, for people to say aloud; the CLI takes the ULID.

Item files are Manyhands' to write. Change them with the CLI (`ticket save`, `document save`), not with an editor or `git`. Content is one line per paragraph: do not hard-wrap a body, because the editors do not reflow it.

## A branch and a worktree per item

Creating an item creates its own branch (`manyhands/ticket/<id>` or `manyhands/document/<id>`) and a worktree for it at `<repo>/.manyhands/worktrees/<id>`. Edits happen on that branch, so two items can be worked on at once without touching each other or the main branch. Repository operations therefore run per item: a sync, a close or a retry names one item, and what you check, you check for that item.

The worktree is inside the main checkout, but it is not the repository root. Commands that write take the main checkout root as `--repo`; the worktree path is used only as `--context` for `path observe`. Do not put files in an item's worktree: an untracked or ignored file there (`target/`, `.devenv/`) blocks closing the ticket, with `recovery_required` and the blocker `context_dirty`.

## A checkpoint on every save

`ticket save` and `document save` write the item and record a checkpoint commit on the item's branch ("Checkpoint ticket <id>"). The answer's `effects.checkpoint` is `committed` and `commit_oid` is that commit. Saving does not touch main and does not publish anything.

## Moving work onward

- A ticket is finished by `ticket close`: it records the closure, merges the ticket's branch into main, publishes main, and removes the branch and worktree. It is a confirmed verb (see [autonomy](autonomy.md)). With no publication remote it still exits 0: the merge into main is complete and `effects.publication` is `pending`, waiting for a remote.
- A document is moved onward with `document promote`, also a confirmed verb. `manyhands-documents` covers it.
- Sharing with collaborators is done with the sync verbs. See `manyhands-sharing-and-sync`.

Closing is not a status. A ticket's `status` is free text the person chooses; a ticket saved with status `closed` is still open: `closure.state` stays `open` and nothing is merged. Closure is `data.closure.state` (`open` or `closed`, with `closed_at` and `closed_by`), and only `ticket close` sets it. To find open work, filter on closure, not on a status word. `ticket list --closure open` does.

## The registry, the index and the data directory

Manyhands keeps a registry of the repositories it knows, and an index that makes lists fast, in a SQLite database in a data directory (a per-user application directory). The repository itself holds only `.manyhands/config.toml` and the items. Consequences:

- A repository the registry does not hold answers `repository_not_registered` on reads; one that was never enabled answers `repository_not_enabled` on writes.
- The index can be stale. A list that looks wrong after an outside change is fixed by an index refresh or rebuild (`manyhands-polling-and-index`); it is not a sign the item is lost.
- `MANYHANDS_DATA_DIR` points Manyhands at a different data directory. Use it to test against a scratch repository without touching the person's real registry, and leave it unset otherwise.
- Do not open or edit the database.

## Who decides

You run the CLI for a person. They own the keys, the passphrases, the trust in remote hosts and the choice of which side wins a conflict. [Autonomy](autonomy.md) lists exactly which actions you take alone and which you hand over.
