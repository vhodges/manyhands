---
name: manyhands-tickets
description: "Use when creating, updating, saving, relating, repairing or closing a Manyhands ticket, giving a ticket a short code, changing its status, type, dependencies or parent, or when a ticket command answers external_change, ticket_closed, relationship_cycle, slug_already_assigned, recovery_required or unlock_required."
---

# Tickets

Read the [overview](../manyhands-overview/SKILL.md) first. Who runs what is in [autonomy](../manyhands-overview/references/autonomy.md); input files, `--request-id`, prepare and confirm, observation tokens and the main checkout root are in [CLI conventions](../manyhands-overview/references/cli-conventions.md); codes are in [errors and exit codes](../manyhands-overview/references/errors-and-exit-codes.md). Look before you write: [finding work](../manyhands-finding-work/SKILL.md) lists tickets, what is ready or blocked, dependencies and short codes, and `ticket check` there tries a relationship change before you save it. The stale-observation pattern is in [documents](../manyhands-documents/SKILL.md) ("A stale observation"); it is the same for tickets. This skill says only what is specific to tickets.

Every verb here takes `--repo <repo-root>`, the repository's main checkout root, never a ticket's worktree. A write that names a worktree answers `not_repository_root` (exit 2) and writes nothing. A ticket's worktree is `.manyhands/worktrees/<id>` inside the repository: when you are working in it, still pass the main checkout root: the path on the first line of `git worktree list`, not the worktree. Keep your `--input` files in a temporary directory outside the repository, and delete them when done.

## The lifecycle

1. `ticket create`: a new ticket, with its own branch and worktree (`manyhands/ticket/<id>`, `.manyhands/worktrees/<id>`). It is given a short code (`slug`) at once.
2. `ticket show`, then `ticket save`: read, then change status, text or relationships, as often as needed. Each save is a checkpoint commit on the ticket's branch.
3. Share it: `manyhands-sharing-and-sync`. Comments on it: `manyhands-comments`.
4. `ticket close`: record the closure, merge the branch into the main branch, publish if there is a remote, remove the branch and worktree. Show the person first and wait ([autonomy](../manyhands-overview/references/autonomy.md)).

`type` and `status` are free text the person chooses (`task`, `bug`, `in_progress`, `review_ready`: none is special). Ask which words the project uses and look at `ticket list` before you invent one. A ticket is open until `closed_at` is set, whatever its status says: saving status `closed` closes nothing, and `ticket close` does not rewrite the status text. [Statuses and relationships](references/statuses-and-relationships.md) has the details.

## Content: one line per paragraph

Write a body as one line per paragraph, with a blank line between paragraphs. Never wrap a line at a column: the editors do not reflow, so a hard-wrapped paragraph stays broken in every view. In the JSON file a body is one string, paragraphs joined by `\n\n`, with no `\n` inside a paragraph.

## Create

Mint a new `item_id` for every ticket with `manyhands-cli id new` and a different new `--request-id` for the write. The ID in the example is a placeholder: never reuse an ID you already used or copied. Two tickets are two `id new` calls.

| Verb | Required | Optional |
| --- | --- | --- |
| `ticket create` | `item_id` (a ULID from `id new`), `title`, `type`, `status` (strings) | `body`, `project`, `team`, `initials` (strings); `deps` (array of ticket IDs, or null); `parent` (ticket ID, or null) |

```json schema=ticket_create
{
  "item_id": "<ITEM_ID>",
  "title": "Fix the login redirect",
  "type": "bug",
  "status": "open",
  "body": "First paragraph on one line.\n\nSecond paragraph on one line.",
  "project": "<project>",
  "deps": ["<ULID>"],
  "parent": "<ULID>"
}
```

```sh
manyhands-cli --json --non-interactive ticket create --repo <repo-root> --input create.json --request-id <REQUEST_ID>
```

- The answer's `data` is `{id, path, slug, unresolved, rejected}`, `scope` holds the new `branch` and `worktree`, and `effects.checkpoint` is `committed`. `--prepare` answers `not_confirmable`: create runs alone.
- `slug` is the short code people say aloud, such as `sc-hfbex`. It is made for you: you cannot set it. `initials` sets the middle part (`vh-06fce`); otherwise it comes from the commit identity, and the prefix from the repository's `ticket_slug_prefix` ([repositories](../manyhands-repositories/SKILL.md)). The CLI takes the ULID as `--id`, never the short code.
- Delete `project`, `deps` and `parent` from the example unless you have real values, and replace the placeholders with real ticket IDs. Put only ticket IDs in them: an ID that is not a ticket, or malformed, answers `invalid_relationship` (exit 2). An ID of a ticket that does not exist yet is accepted and listed in `data.unresolved`: the ticket then reads as blocked (`unresolved_dependency`) until that ticket arrives. A cycle is `relationship_cycle` (exit 2) with the tickets on it in `data.rejected`: try it first with `ticket check`.
- A missing required field answers `invalid_input` (exit 2). Rerun without `--json` to read which field.
- Reusing an `item_id` with other content answers `occupied_path`; the same content again is `already_applied`. If you think the ID is new and see `occupied_path`, you reused one: mint another.

## Read before you write

```sh
manyhands-cli --json ticket show --repo <repo-root> --id <ITEM_ID>
```

`data.observation` is the token a save needs, `data.status`, `data.deps`, `data.parent` and `data.body` are what is there now, `data.closure` says whether it is open, and `data.context.worktree` is the ticket's worktree. The human output does not print the token, so read with `--json`.

## Save: a patch

A save names the ticket with `--id` and sends a patch; the token comes from the show you just did. A field you leave out stays as it was.

| Verb | Required | Optional |
| --- | --- | --- |
| `ticket save` | `observation` (string) | `title`, `type`, `status`, `body` (strings; none accepts null); `project`, `team` (string, or null to clear); `deps` (array of ticket IDs; null or `[]` clears); `parent` (ticket ID, or null to clear) |

```json schema=ticket_save
{
  "observation": "<OBSERVATION>",
  "status": "in_progress",
  "project": null,
  "deps": []
}
```

```sh
manyhands-cli --json --non-interactive ticket save --repo <repo-root> --id <ITEM_ID> --input save.json --request-id <REQUEST_ID>
```

- That example changes the status, clears `project` and clears every dependency. Leave `project` and `deps` out of a patch that should keep them. `deps` is replaced whole: to add one dependency, send the full new list. `parent` and `deps` clear independently (verified: clearing `deps` kept the parent).
- A body is replaced whole, so send the complete new body. `"title": null` and `"status": null` are `invalid_input`.
- A save that changes nothing answers `already_applied` and writes nothing (verified: a second save with an already-used token and the same change, and an observation-only save on a clean worktree).
- Any save captures the person's uncommitted edits in the ticket's worktree. If someone edited the worktree's `ticket.md` outside Manyhands, `ticket show` reads that edit, a save with its fresh token commits it as it stands (verified, even an observation-only save), and a token from before the edit answers `external_change`. If the edit broke the front matter (a missing `type`, say), `ticket show` reports `missing_field` in `problems` and a save answers `item_not_found` (exit 2) and commits nothing: stop and tell the person, who can revert the edit, or use `ticket repair`.
- A stale token answers `external_change` (exit 3) when the change is new: read again, merge, save with the new token and a new `--request-id`. See [documents](../manyhands-documents/SKILL.md) for the steps. `--prepare` answers `not_confirmable`: save runs alone.
- A closed ticket cannot be saved: `ticket_closed` (exit 3). Set the status and text you want before you close.
- A save from a patch is checked like a create: `relationship_cycle` (exit 2, nothing written) or an ID not yet known (accepted, in `data.unresolved`).

## Short codes: `ticket slug-assign`

Only a ticket without a `slug` needs this (an older ticket, or one a person committed by hand). `ticket list` shows `slug: null` for it. It writes one code, once.

| Verb | Required | Optional |
| --- | --- | --- |
| `ticket slug-assign` | `observation` (string) | `initials` (string, not null) |

```json schema=ticket_slug_assign
{
  "observation": "<OBSERVATION>",
  "initials": "ab"
}
```

```sh
manyhands-cli --json --non-interactive ticket slug-assign --repo <repo-root> --id <ITEM_ID> --input slug.json --request-id <REQUEST_ID>
```

- It runs alone like create and save, but it is write-once: the code never changes afterwards.
- Run on a ticket that already has a code, the answer depends on whether the initials match the code's middle part. Initials omitted are taken from the commit identity (`repo identity` shows them). Matching initials, given or derived, answer `already_applied`; different ones answer `slug_already_assigned` (exit 2) and change nothing. Verified: a code `sc-...` with identity initials `sc` was `already_applied` with none and with `sc`, and `slug_already_assigned` with `zz`; a code `vh-...` was `slug_already_assigned` with none (identity initials `sc`) and `already_applied` with `vh`.
- Run on a ticket that is only on the main branch, it writes the code on a new branch and worktree for the ticket, so the ticket is then in progress there ("active" in `context`). `--prepare` answers `not_confirmable`.

## Repair: a ticket file that is not valid

A ticket file that Manyhands cannot read as a ticket is listed with `problems` (for example `missing_field`: no `type`) and no usable fields; `index status` lists the same. `ticket show --path <path>` of the file gives the `observation` and the whole `source`. Run `index refresh` first if you suspect the index has not seen an outside commit (`manyhands-polling-and-index`).

`ticket repair` writes the corrected whole file onto a new branch and worktree, and is a confirmed verb. It rewrites a person's file, so it is show then wait in [autonomy](../manyhands-overview/references/autonomy.md): show the person the `source` you will write, then confirm.

| Verb | Required | Optional |
| --- | --- | --- |
| `ticket repair` | `path` (repository-relative), `observation` (from `ticket show --path`), `source` (the complete corrected file) | none |

The `source` is the whole file: front matter with the `id` that is the ticket's directory name, `title`, `type`, `status`, `manyhands_managed: true`, `manyhands_kind: ticket`, then the body. Keep every field the file already had. If the file has no `id`, put the directory's ULID in: a different ID is refused (`not_repairable`, exit 2). The preview shows `details.adoption` true when the ID is being adopted.

```json schema=ticket_repair
{
  "path": ".manyhands/tickets/<ITEM_ID>/ticket.md",
  "observation": "<OBSERVATION>",
  "source": "---\nid: <ITEM_ID>\ntitle: Repaired ticket\ntype: task\nstatus: open\nmanyhands_managed: true\nmanyhands_kind: ticket\n---\nBody paragraph on one line.\n"
}
```

```sh
manyhands-cli --json --non-interactive ticket repair --repo <repo-root> --input repair.json --prepare
manyhands-cli --json --non-interactive ticket repair --repo <repo-root> --input repair.json --confirm <CONFIRMATION_ID> --request-id <REQUEST_ID>
```

Without `--confirm` the verb answers `confirmation_required` (exit 3). The preview lists effects (`context_created`, `source_written`, `checkpoint_committed`, and `id_adopted` when it adopts an ID), not the file. After the repair the ticket is on its own branch and shows with no problems. A ticket the repair cannot fix is `not_repairable`: tell the person.

## Close: show the person first, then confirm

Closing records `closed_at` and `closed_by`, merges the ticket's branch into the main branch, publishes the main branch when a publication remote is selected, and removes the branch and worktree. It cannot be undone from the CLI, so it is show then wait ([autonomy](../manyhands-overview/references/autonomy.md)), even when the person said "close it": the request does not show them these effects. As part of showing the preview, ask the person whether the work has been reviewed and approved, and treat their OK as covering that too: closing is the last step after approval, and the CLI does not check it.

Before you prepare:

1. Save the status and text you want to keep. A closed ticket cannot be saved, and close does not rewrite the status.
2. Look for open children and open dependencies (reads in [finding work](../manyhands-finding-work/SKILL.md)): `ticket children --id <ITEM_ID>` for children, and the ticket's own dependencies from `ticket show` (`data.deps`, each with a `state`, and `readiness` `blocked` with reason `open_dependency`) or `ticket deps --direction down`. Not `--direction up`: that lists the tickets that depend on this one, and for a ticket that depends on an open ticket it is empty (verified). Close does not check: it closes a ticket whose children or own dependencies are still open (verified), and leaves them open, and the preview does not mention them. Tell the person what you found. Open dependents need no warning: closing makes them ready (`made_ready`).

```sh
manyhands-cli --json --non-interactive ticket close --repo <repo-root> --id <ITEM_ID> --prepare
```

`--prepare` writes nothing, takes no `--request-id`, and answers with `data.confirmation_id`, `data.expires_at` (about ten minutes) and what it will do. Tell the person in plain words:

- Which ticket: `details.slug` and `details.branch`. The preview carries no title, so take the title from `ticket show`.
- `data.effects`: `closure_recorded`, `primary_integrated` (merged into `details.primary_branch` as a merge commit), `worktree_removed` and `branch_deleted`. With a remote it also lists `context_synchronized` (the ticket's branch is pushed first), `primary_published` and `remote_branch_deleted`.
- `details.changes`: the paths the merge adds or changes.
- `details.publication`: `publish` (the main branch is pushed to `details.remote`) or `pending_no_remote` (nothing is pushed: there is no publication remote, and a later `repo sync` publishes it, see `manyhands-sharing-and-sync`).
- `details.closure.closed_by`: whose name goes on the closure, from the repository's commit identity.
- That the status text stays as it is, and that the ticket worktree and local branch are deleted.

Then stop and wait for their OK. If they say no, or the ten minutes pass, drop it. Only after their OK:

```sh
manyhands-cli --json --non-interactive ticket close --repo <repo-root> --id <ITEM_ID> --confirm <CONFIRMATION_ID> --request-id <REQUEST_ID>
```

Use a new `--request-id`, and keep it: it is how you continue this close if something stops it.

### Reading the answer

| Answer | Meaning and next step |
| --- | --- |
| exit 0, `outcome` `success`, `data.cleanup` `complete` | Closed. `data.publication` is `published` or, with no remote, `pending` (`publication_pending` `no_publication_remote`). `data.made_ready` lists the dependent tickets (`id`, `slug`) that this close made ready: tell the person. Check with `ticket show`: `closure.state` `closed`, `context.kind` `primary`. |
| exit 4, `outcome` `partial`, `code` `recovery_required`, `data.cleanup_pending` `ignored_files` | The closure and merge are done and the worktree is not removed, because it holds an ignored file such as `target/` or `.devenv/`. See below. |
| exit 3, `unlock_required` | The key needs a passphrase. Nothing was recorded. Hand over, see below. |
| exit 3, `confirmation_required` | You ran the close without `--confirm`. Prepare first. |
| exit 3, `confirmation_not_found` | The confirmation expired or is unknown. Prepare again and show the person the new preview. |
| exit 3, `confirmation_used` | That confirmation already ran with another request ID. `ticket show` the ticket and see what is done. |
| exit 3, `ticket_closed` | Already closed (`data.blockers` has `already_closed`). Nothing to do. |

Checking the result is a read of the ticket: its `closure` is the truth, not its `status`. To list closed tickets: `ticket list --closure closed`.

### A prepare that is blocked

A blocked prepare is `recovery_required` or `identity_required` (exit 3) with `data.blockers`. Nothing was changed.

| Blocker | Meaning and fix |
| --- | --- |
| `context_dirty` | The ticket's worktree has changes Manyhands did not commit: a file Git does not ignore, or an edited tracked file. Look with a read-only `git -C <worktree> status --short` (the worktree is `data.context.worktree` of `ticket show`). They are the person's files, so show the list and ask. Remove or move an unwanted other file only with their OK, then prepare again. The ticket's own `ticket.md` is different: never delete it, `git checkout` it or edit it yourself, because reverting an unwanted edit is the person's decision. If they want the edit kept and `ticket show` reads it cleanly, a `ticket save` (fresh token) commits it as it stands (verified). If the edit broke the front matter, the prepare lists `context_nonconforming` as well and a save answers `item_not_found`: hand it to the person, or use `ticket repair`. |
| `primary_dirty` | The main checkout has uncommitted or untracked changes. They are the person's: ask them, never clean up. |
| `context_missing` | The ticket exists only on the main branch and has no branch to merge (a ticket committed outside Manyhands). A `ticket save` or `ticket slug-assign` gives it a branch and worktree; then prepare again. Ask the person first if the save changes anything. |
| `identity_required` | The repository has no commit identity, so there is no `closed_by`. `repo identity-set` is a show-then-wait step: see [repositories](../manyhands-repositories/SKILL.md). |
| `history_unresolved`, `history_unknown` | The branch and its remote have histories Manyhands cannot reconcile. `manyhands-recovery`. Not run here. |

Anything else with `recovery_required`: read `data.blockers` and `recovery`, and take it to `manyhands-recovery`.

### The worktree holds an ignored file

A file Git ignores in the ticket's worktree (a `target/` build directory, `.devenv/`) does not stop the prepare, and the close then stops at cleanup: exit 4, `partial`, `recovery_required`, `data.cleanup_pending` `ignored_files`. Verified: the closure and the merge into the main branch are already done (`ticket show` says `closure.state` closed, with `context.kind` still `active` because the branch and worktree remain), and `operation list` shows the close as an unfinished operation.

1. Do not start over and do not prepare a second close of the same ticket.
2. Tell the person which ignored files are in the worktree (`git -C <worktree> status --short --ignored`, read only). They are build output, but they are the person's files: ask before you delete them.
3. After their OK, remove them, and run the same confirm again with the same `--confirm` and the same `--request-id`. If they decline, the ticket stays closed and merged (verified) and only the cleanup stays pending: the worktree and local branch remain and the close shows in `operation list`. Tell them so and leave it; `manyhands-recovery` covers finishing it later. Replaying the same request continues the close and repeats no merge: it answers exit 0, `cleanup` `complete`, and the worktree and branch are gone (verified). A new request ID with the used confirmation answers `confirmation_used`.

To avoid it: never run builds, editors or tools inside a ticket's worktree.

### The key needs a passphrase: hand it over

With a passphrase-protected key, a close that needs the remote answers `unlock_required` (exit 3) before it records anything: the ticket stays open and the worktree is untouched (verified with a throwaway key and sshd on 127.0.0.1). A close with no publication remote needs no key (verified with no key selected). You cannot type the passphrase and you never ask for it. Give the person the exact command to run at their own terminal, without `--json --non-interactive`, with the same confirmation and request ID as your attempt:

```sh
manyhands-cli ticket close --repo <repo-root> --id <ITEM_ID> --confirm <CONFIRMATION_ID> --request-id <REQUEST_ID>
```

At a terminal the CLI prompts for the passphrase and finishes the same operation. Verified end to end with a throwaway key: `publication` `published`, the remote's main branch holds the merge, the ticket's branch and worktree are gone. That ran within the confirmation's ten minutes; the contract says an accepted confirmation stands for its request, and a retry after expiry was not run. Afterwards read the ticket with `ticket show` and check `closure.state`. If the confirmation has expired, prepare again, show the person the preview again, and give them the new command. If the person does not finish it, the started close stays in `operation list` (a `close` operation, `confirmed`) and the ticket stays open on its branch: nothing is lost, and `manyhands-recovery` covers resuming or abandoning it (abandoning is the person's call). See [keys and hosts](../manyhands-keys-and-hosts/SKILL.md).

## Codes you will meet

| Code (exit) | Meaning and next step |
| --- | --- |
| `external_change` (3) | The token is stale. Read again, merge, save with the new token and a new request ID. |
| `ticket_closed` (3) | The ticket is closed: no save, slug-assign or second close. |
| `relationship_cycle` (2) | The `deps` or `parent` would form a cycle. `data.rejected` names the tickets. Try with `ticket check`. |
| `invalid_relationship` (2) | A `deps` or `parent` ID is not a ticket ID. |
| `slug_already_assigned` (2) | The ticket has a code and the initials (given, or derived from the identity) differ from it. |
| `invalid_input` (2) | A field is missing, unknown or null where null is refused. Rerun without `--json`. |
| `occupied_path` (2) | The `item_id` was already used for other content. Mint a new one. |
| `item_not_found` (2) | No item has that ID. `ticket list`. |
| `not_confirmable` (2) | `--prepare` on create, save or slug-assign. Run them without it. |
| `confirmation_required` (3) | Close or repair without `--confirm`. Prepare first. |
| `recovery_required` (3, or 4 after a partial close) | Read `data.blockers`, or `data.cleanup_pending`. |
| `unlock_required` (3) | A passphrase key: hand the step to the person. |
| `not_repository_root` (2) | `--repo` was a worktree. Use the main checkout root. |

Not run for this skill: a close whose remote moved since you prepared (another person's work arrived), `history_unresolved` and `history_unknown`, and a remote reached with a host key that changed. The contract describes them; take any such answer to `manyhands-recovery` and tell the person.
