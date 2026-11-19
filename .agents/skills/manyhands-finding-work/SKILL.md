---
name: manyhands-finding-work
description: "Use when you need to find out what exists in a Manyhands repository before acting: listing or filtering tickets and documents, what is ready or blocked, what depends on what, finding a ticket by short code, reading an item, comments or folders, or checking whether what you see is current."
---

# Finding work

Read the [overview](../manyhands-overview/SKILL.md) first. JSON answers, the envelope and `--repo` are in [CLI conventions](../manyhands-overview/references/cli-conventions.md); codes are in [errors and exit codes](../manyhands-overview/references/errors-and-exit-codes.md). This skill is reads only, except `index refresh` (it rewrites the local index, not the repository, and needs `--request-id`): nothing else here writes, asks the person or needs `--request-id`. Looking first is how you avoid a duplicate ticket, a path clash or a stale save. To change what you found: [documents](../manyhands-documents/SKILL.md), and `manyhands-tickets` for tickets.

Every verb here takes `--repo <repo-root>`, the repository's main checkout root, and fails with `usage_error` without it. Reads also accept a worktree path (only writes refuse one with `not_repository_root`), but pass the root. A repository Manyhands does not know answers `repository_not_registered`: see "Not registered" in [recovery](../manyhands-recovery/SKILL.md). Add `--json`. The human output is one short line per item, the ID first and the title last, and leaves out most fields, including the observation token.

## Look first, in this order

1. `repo inspect` and `index status`: is the repository registered, and is the index current ("Is what I see current")?
2. `ticket list --closure open` for open work, `ticket ready` for what can start now, `ticket blocked` for what is waiting and why.
3. `document list` and `folder list` for documents and where they are.
4. `ticket show --id` or `document show --id` for the one item you will touch. Its `observation` is the token a save needs.

Two cautions about step 1 and 3. `index status` `current` can coexist with an item that is missing from a list, because a commit made outside Manyhands is not seen until `index refresh`. And a list shows an item's fields, with `body` and `source` null: read the item with `ticket show` or `document show` before you decide what it says.

```sh
manyhands-cli --json ticket list --repo <repo-root> --closure open
manyhands-cli --json ticket ready --repo <repo-root>
manyhands-cli --json ticket blocked --repo <repo-root>
manyhands-cli --json document list --repo <repo-root>
manyhands-cli --json folder list --repo <repo-root>
```

## The shape of every list

A list answers `data.items` (an array), `data.complete` and `data.index`. Lists are never silently truncated: when `complete` is `true` you have all of them. When it is `false` the last refresh could not read some directory and items may be missing; run `index refresh` (see "Is what I see current" below) and read again before you conclude something does not exist. `data.index.state` (`current` in every run here) and `refreshed_at` say how fresh the index behind the list is.

Order: documents by path then ID; tickets by most recent change first (`changed_at`), then ID; folders by path. Do not read an order as a priority: the ticket order is only recency, and there is no priority field.

An item in a list or a show has these fields. A list leaves `body`, `source` and `observation` null; only a show fills them.

| Field | Meaning |
| --- | --- |
| `id` | The item's ULID. Null for a Markdown file that is not yet a managed document (see "Documents"). |
| `kind` | `ticket` or `document`. |
| `path` | Repository-relative path. A ticket is `.manyhands/tickets/<id>/ticket.md`. |
| `title`, `type`, `status`, `project`, `team` | The ticket's fields. `type` and `status` are free text the person chooses. A document has only `title`; the others are null. |
| `slug` | A ticket's short code, such as `sc-ch8gg`. Null on a document. |
| `closure` | A ticket: `state` (`open` or `closed`), `closed_at`, `closed_by`. Null on a document. |
| `parent`, `deps` | A ticket's parent ID, and `deps`: a list of `{id, state}` (`open`, `closed` or `unresolved`). |
| `readiness` | A ticket: `state` (`ready`, `blocked` or `closed`) and `reasons`. |
| `context` | Where this copy lives: `kind` `active` (the item's own branch and worktree) or `primary` (the main branch), plus `branch`, `worktree`, `head_oid`. |
| `problems` | Nonconforming-file findings with a `code`, `path` and `guidance`. Empty for a healthy item. |
| `changed_at`, `change_source` | When it last changed, and `git_commit` or `uncommitted` (an edit not yet committed). |
| `body`, `source`, `observation` | Show only: the content, the whole file, and the token for a save. |
| `unknown_metadata` | Front matter Manyhands does not know; kept as is. |

An item that has its own branch is listed once, as its `active` copy. Use `context.worktree` as `--context` for `path observe`, never as `--repo`.

## Tickets

Open and closed come from `closure.state`, never from the `status` text: a ticket whose status says `closed` is still open until `ticket close` ([concepts](../manyhands-overview/references/concepts.md)).

| Command | What it answers |
| --- | --- |
| `ticket list` | Every ticket, open and closed, unless filtered. Filters, all exact matches: `--status`, `--type`, `--project`, `--slug`, `--closure open\|closed\|all`, and `--ready` or `--blocked` (the two together are `usage_error`). An unknown value matches nothing: `--status nope` is an empty list with exit 0. |
| `ticket ready` | Open tickets whose every dependency is closed. Takes `--status`, `--type`, `--project`. |
| `ticket blocked` | Open tickets that are not ready, each with `readiness.reasons`: `{code: open_dependency, ids: [...], complete: true}` per open dependency. The relationships RFC names more reasons (an unresolved dependency ID, membership of a cycle); they were not provoked here. |
| `ticket deps --id <ID>` | The tickets this one depends on (`--direction down`, the default), the tickets that depend on it (`up`) or both; `--depth <n>` limits it. `data.dependencies` and `data.dependents` are flat lists with `id`, `state`, `slug`, `title` and `depth` (1 is a direct dependency). A ticket reached twice is marked `repeated`. |
| `ticket children --id <ID>` | Direct children (a ticket's `parent` is this ID). Empty when none. |
| `ticket cycles` | Every dependency or parent cycle, open or closed. Empty `items` when there are none. |
| `ticket plan` | Open tickets in `data.batches` (`batch` 1, 2, ...): everything in a batch can be worked in parallel (in different worktrees; writers on different items do not block each other on a build with the concurrent-writers change, and an older build wants one write at a time: [CLI conventions](../manyhands-overview/references/cli-conventions.md), "Other writers in the repository"), and its open dependencies are in earlier batches. `data.unplannable` holds tickets behind a cycle or an unresolved dependency. Takes the list filters. |
| `ticket critical-path` | The longest chain of open, plannable tickets by dependency, in order, as `data.items`. |
| `ticket find --slug <code>` | Tickets whose short code matches, ignoring case. No match is an empty list, not an error. `ticket list --slug` filters the same way. |
| `ticket show --id <ID>` | One ticket, with `body`, `source` and `observation`. `--path` with `--context` names it by path instead. |
| `ticket check --id <ID> --deps <ULID,...> --parent <ULID>` | A read-only trial of a relationship change before you save it (no input file, no request ID). Give `--deps` once, comma separated, and at least one of `--deps` or `--parent`: it checks exactly what you give, so give the ones you are keeping too. The answer is a success; read `data.rejection`: null means the change would be accepted, otherwise `{code, ids}`, for example `relationship_cycle` listing the tickets on the cycle. |

```sh
manyhands-cli --json ticket list --repo <repo-root> --closure open --project <project>
manyhands-cli --json ticket deps --repo <repo-root> --id <ITEM_ID> --direction both
manyhands-cli --json ticket plan --repo <repo-root>
manyhands-cli --json ticket find --repo <repo-root> --slug <short-code>
manyhands-cli --json ticket check --repo <repo-root> --id <ITEM_ID> --deps <ULID>,<ULID>
```

Ready and blocked come from the dependencies, not from status words: a ticket is ready when it is open and every dependency is closed, and closing a ticket makes its direct dependents ready. Run `ticket ready` for the live answer. These views read the index, so check that it is current first.

## Documents and folders

- `document list` has no filters: read all of it and select by `path` or `title` yourself. Look for your path before you create a document ([documents](../manyhands-documents/SKILL.md)): two unpromoted documents may hold the same path, and the list then shows both.
- An item with `id: null` is a Markdown file under `docs/` that Manyhands does not manage yet; its `problems` say why (`missing_front_matter`, `missing_field`). Repairing it is in [documents](../manyhands-documents/SKILL.md).
- `document show --id <ID>` reads one document; `document show --path <path>` only finds a document that is on the main branch, unless you add `--context <worktree>`.
- `folder list` returns `{path, empty}` per folder, parents included. It shows folders made with `folder create` and folders that hold documents on the main branch; `empty` says which.

## Comments

`comment list --repo <repo-root> --id <ITEM_ID>` lists an item's comments: `data.items[]` each with `id`, `author`, `created_at`, `body`, `parent_id` (null for a top-level comment), `depth` and `path`, plus the item's `context`. An item with none is an empty list; an ID that is not an item is `item_not_found`. Writing a comment is [comments](../manyhands-comments/SKILL.md).

## Is what I see current

These are all reads you run alone.

```sh
manyhands-cli --json index status --repo <repo-root>
manyhands-cli --json poll status --repo <repo-root>
manyhands-cli --json operation list --repo <repo-root>
manyhands-cli --json repo inspect --repo <repo-root>
```

- `index status`: `data.state` (`current` in every run here; the contract also names `stale`, `never_refreshed` and `unavailable`, which were not provoked), `refreshed_at`, `item_count`, `context_count`, and `problems` (the same findings as an item's `problems`) with `problem_count`. `pending_operations` lists local operations such as a local sync conflict, and omits remote ones (a remote conflict, a blocked sync, a close stopped at publication; verified), so do not rely on it: `operation list` is the check, and a non-empty list is for [`manyhands-recovery`](../manyhands-recovery/SKILL.md).
- The index is a cache of the repository. Manyhands' own writes update it at once. A commit made outside Manyhands (a person's `git commit`, a merge, a pull) is not seen until the index is refreshed: after one, `document list` omitted the new file while `index status` still said `current` (verified). `index refresh --repo <repo-root> --request-id <REQUEST_ID>` brings it up to date; you run it alone, and it is not confirmable. [`manyhands-polling-and-index`](../manyhands-polling-and-index/SKILL.md) covers refresh, rebuild and the schedule.
- `poll status` says whether polling a shared remote is on (`enabled`, `paused`, `interval_seconds`, `latest_outcome`, `next_eligible_at`). A repository with no remote shows the defaults and nothing observed. Whether other people's work has arrived is [sharing and sync](../manyhands-sharing-and-sync/SKILL.md) and [`manyhands-polling-and-index`](../manyhands-polling-and-index/SKILL.md).
- `operation list` lists operations that still have work to do (an interrupted write, sync or promotion). Empty `items` means none. A non-empty list is for [`manyhands-recovery`](../manyhands-recovery/SKILL.md).
- `repo inspect` shows `registered`, `head_branch`, `identity_state` (`available` or `required`), `local_branches` (including `manyhands/ticket/<id>` and `manyhands/document/<id>` branches of items in progress), `remotes` and, on a build with the change, `operations`, the unfinished operations in the shape of `operation list`. If those cannot be read it still answers, with `operations` empty and the problem `operations_unreadable`: run `operation list` for the reason.

## Codes you will meet

| Code (exit) | Meaning and next step |
| --- | --- |
| `usage_error` (2) | A missing `--repo`, a flag the verb does not take, `--ready` with `--blocked`, or a bad `--closure` value. Fix the command line. |
| `repository_not_registered` (3) | Manyhands does not know this repository. "Not registered" in [`manyhands-recovery`](../manyhands-recovery/SKILL.md); never `index rebuild` first. |
| `item_not_found` (2) | No item has that ID. Look with `ticket list` or `document list`. |
| `path_not_found` (2) | `--path` names nothing on the main branch. Use `--id`, or add `--context <worktree>`. |
| `invalid_id` (2) | An ID that is not a ULID, including a bad entry in `--deps`. |
