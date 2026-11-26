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
3. Share it: [sharing and sync](../manyhands-sharing-and-sync/SKILL.md). Comments on it: [comments](../manyhands-comments/SKILL.md).
4. `ticket close`: record the closure, merge the branch into the main branch, publish if there is a remote, remove the branch and worktree. Show the person first and wait ([autonomy](../manyhands-overview/references/autonomy.md)). In a pull-request repository it records the closure on the ticket's branch and publishes that branch instead, and the ticket reads `closing` until the pull request merges: "Close in a pull-request repository" below. `ticket withdraw-closure` takes a closure back while it is only on the branch.

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

A ticket file that Manyhands cannot read as a ticket is listed with `problems` (for example `missing_field`: no `type`) and no usable fields; `index status` lists the same. `ticket show --path <path>` of the file gives the `observation` and the whole `source`. Run `index refresh` first if you suspect the index has not seen an outside commit ([`manyhands-polling-and-index`](../manyhands-polling-and-index/SKILL.md)).

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

This section is the direct repository's close; in a pull-request repository read "Close in a pull-request repository" below first. Closing records `closed_at` and `closed_by`, merges the ticket's branch into the main branch, publishes the main branch when a publication remote is selected, and removes the branch and worktree. It cannot be undone from the CLI, so it is show then wait ([autonomy](../manyhands-overview/references/autonomy.md)), even when the person said "close it": the request does not show them these effects. As part of showing the preview, ask the person whether the work has been reviewed and approved, and treat their OK as covering that too: closing is the last step after approval, and the CLI does not check it.

Before you prepare:

1. Save the text you want to keep, and ask the person which status the ticket should end with: never pick a done-like status yourself. A closed ticket cannot be saved, and close does not rewrite the status.
2. Look for open children and open dependencies (reads in [finding work](../manyhands-finding-work/SKILL.md)): `ticket children --id <ITEM_ID>` for children, and the ticket's own dependencies from `ticket show` (`data.deps`, each with a `state`, and `readiness` `blocked` with reason `open_dependency`) or `ticket deps --direction down`. Not `--direction up`: that lists the tickets that depend on this one, and for a ticket that depends on an open ticket it is empty (verified). Close does not check: it closes a ticket whose children or own dependencies are still open (verified), and leaves them open, and the preview does not mention them. Tell the person what you found. Open dependents need no warning: closing makes them ready (`made_ready`).

```sh
manyhands-cli --json --non-interactive ticket close --repo <repo-root> --id <ITEM_ID> --prepare
```

`--prepare` writes nothing, takes no `--request-id`, and answers with `data.confirmation_id`, `data.expires_at` (about ten minutes) and what it will do. Tell the person in plain words:

- Which ticket: `details.slug` and `details.branch`. The preview carries no title, so take the title from `ticket show`.
- `data.effects`: `closure_recorded`, `primary_integrated` (merged into `details.primary_branch` as a merge commit), `worktree_removed` and `branch_deleted`. With a remote it also lists `context_synchronized` (the ticket's branch is pushed first), `primary_published` and `remote_branch_deleted`.
- `details.changes`: the paths the merge adds or changes.
- `details.publication`: `publish` (the main branch is pushed to `details.remote`) or `pending_no_remote` (nothing is pushed: there is no publication remote, and a later `repo sync` publishes it, see [sharing and sync](../manyhands-sharing-and-sync/SKILL.md)).
- `details.closure.closed_by`: whose name goes on the closure, from the repository's commit identity.
- That the status text stays as it is, and that the ticket worktree and local branch are deleted.
- What the ticket's worktree holds that Git ignores. Look first, read only: `git -C <worktree> status --short --ignored` (the worktree is `data.context.worktree` of `ticket show`; the `!!` lines are ignored files and directories) and `find <worktree> -mindepth 2 -name .git` (a nested Git repository; the worktree's own `.git` is not matched). Ignored files are build output, caches and local environment directories (`target/`, `.devenv/`, editor caches). A build with the ignored-files change deletes them with the worktree in the same run, without asking and without listing them (on an older build the close stops at cleanup instead), and the preview does not mention them. Tell the person which ones exist and that they will be deleted, and let them save anything they want first. To look inside an ignored directory, read only: `ls -R <worktree>/<dir>`. If the `find` shows a nested repository, say so, and neither confirm nor prepare again until the person has decided: it needs their decision before the close (see "Ignored files in the worktree" below).

Then stop and wait for their OK. If they say no, or the ten minutes pass, drop it. Only after their OK:

```sh
manyhands-cli --json --non-interactive ticket close --repo <repo-root> --id <ITEM_ID> --confirm <CONFIRMATION_ID> --request-id <REQUEST_ID>
```

Use a new `--request-id`, and keep it: it is how you continue this close if something stops it.

### Reading the answer

| Answer | Meaning and next step |
| --- | --- |
| exit 0, `outcome` `success`, `data.cleanup` `complete` | Closed. `data.publication` is `published` or, with no remote, `pending` (`publication_pending` `no_publication_remote`). `data.made_ready` lists the dependent tickets (`id`, `slug`) that this close made ready: tell the person. Check with `ticket show`: `closure.state` `closed`, `context.kind` `primary`. |
| exit 4, `outcome` `partial`, `code` `recovery_required`, `data.cleanup_pending` `context_changed` or `ignored_files` | The closure and merge are done and the worktree is not removed. See "Ignored files in the worktree" below. |
| exit 3, `unlock_required` | The key needs a passphrase. Nothing was recorded, but the close stays unfinished in `operation list`. Hand over, see below. |
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
| `primary_missing` | A remote is selected, but this repository has never read the remote's primary branch, because no poll has run since the remote was added and selected (verified: `ticket close --prepare` answered `blocked` / `recovery_required` with `data.blockers` `["primary_missing"]`). The JSON `recovery` offers only `repo.inspect`; the human output names the way on. Run `poll once` (you run it alone, [polling and the index](../manyhands-polling-and-index/SKILL.md)), then prepare again. With no remote selected the blocker does not arise. |
| `primary_ahead_of_remote` | The main branch holds commits the remote's primary lacks, for example the commit that selected the remote (seen right after `primary_missing` was cleared by a poll). `poll once`, then `repo sync` (both yours to run alone, [sharing and sync](../manyhands-sharing-and-sync/SKILL.md)), then prepare again (verified: the prepare then showed `publish`). |
| `identity_required` | The repository has no commit identity, so there is no `closed_by`. `repo identity-set` is a show-then-wait step: see [repositories](../manyhands-repositories/SKILL.md). |
| `history_unresolved`, `history_unknown` | The branch and its remote have histories Manyhands cannot reconcile. [`manyhands-recovery`](../manyhands-recovery/SKILL.md). Not run here. |

Anything else with `recovery_required`: read `data.blockers` and `recovery`, and take it to [`manyhands-recovery`](../manyhands-recovery/SKILL.md).

### Ignored files in the worktree

This section is the one home for what a ticket close (and a document promotion and `context retire`) does with files Git ignores in the item's worktree. Other skills link here. Two behaviours exist, and the preview is the same for both, so you learn which build you are on from what the close does.

**A build with the ignored-files change** (the owner-approved change to cleanup; verified on a rebuilt CLI that reports the same `0.2.0`, so do not key on `--version`). Ignored files no longer block. A worktree whose only extras are ignored (`target/`, `.devenv/`, `.cache/` were run) closes in one run: exit 0, `data.cleanup` `complete`, no `cleanup_pending`, the worktree and branch gone, with no remote (`publication` `pending`) and with a remote (`published`, and the ticket's remote branch deleted); `operation list` is empty afterwards. The ignored files went with the worktree, unasked and unlisted: that is why the preview step above makes you tell the person first. Still blocking, as before:

- A file Git does not ignore, or a changed tracked file (`ticket.md` edited): the prepare answers `context_dirty` (exit 3), see "A prepare that is blocked". Verified again on this build with an untracked file and with an edited `ticket.md`.
- An ignored directory that holds a nested Git repository (a `.git` inside `target/`, which can hold commits that exist nowhere else), or one the CLI cannot read: the prepare passes, the close merges, and then stops at cleanup: exit 4, `partial`, `recovery_required`, `data.cleanup_pending` `context_changed`. `git status --short --ignored` shows such a repository only as `!! target/`, so use the `find` above. Verified with a nested repository and with a directory made unreadable (mode 000).

**Released 0.2.0 and any older build** (from the design and this skill's earlier verified run with an older build; not re-run on the new one): any ignored file in the worktree passes the prepare, the close merges, and then stops at cleanup: exit 4, `partial`, `recovery_required`, `data.cleanup_pending` `ignored_files`. Treat that answer as the sign that you are on an older build.

What you do after an exit 4 stop at cleanup, for either cause:

1. Do not start over and do not prepare a second close of the same ticket. The closure and the merge into the main branch are done: `ticket show` says `closure.state` `closed` with `context.kind` still `active` (the branch and worktree remain), and `operation list` shows the close as an unfinished operation (verified for `context_changed`).
2. Find what holds the worktree: `git -C <worktree> status --short --ignored` and the `find` above. They are the person's files. Tell them what is there, and for a nested repository or unreadable directory say that it may hold work that exists nowhere else. Do not delete, move or `chmod` anything of it yourself: the person decides, or gives you an explicit OK for each thing. For `ignored_files` on an older build the files are build output and the same rule applies: ask before removing.
3. After they have dealt with it (removed or moved the nested repository, fixed the permissions, or said the ignored files may go), run the same confirm again with the same `--confirm` and the same `--request-id`. Verified on the new build for a nested repository (a retry while it was still there stopped again with `context_changed`, nothing lost) and for an unreadable directory (after the mode was restored): exit 0, `cleanup` `complete`, the worktree and branch gone, with no merge repeated. A new request ID with the used confirmation answers `confirmation_used`. A close that an older build left at `ignored_files` finishes the same way when the same request is retried on a newer build: that is from the design, not run (no older binary).
4. If they decline, the ticket stays closed and merged and only the cleanup stays pending: the worktree and local branch remain and the close shows in `operation list`. Tell them so and leave it; [`manyhands-recovery`](../manyhands-recovery/SKILL.md) covers finishing it later.

To avoid it: do not run builds, editors or tools inside a ticket's worktree. On a newer build their output is deleted with the worktree; on an older one it stops the close.

### The key needs a passphrase: hand it over

With a passphrase-protected key, a close that needs the remote answers `unlock_required` (exit 3) before it records anything: the ticket stays open and the worktree is untouched (verified with a throwaway key and sshd on 127.0.0.1). A close with no publication remote needs no key (verified with no key selected). You cannot type the passphrase and you never ask for it. Give the person the exact command to run at their own terminal, without `--json --non-interactive`, with the same confirmation and request ID as your attempt:

```sh
manyhands-cli ticket close --repo <repo-root> --id <ITEM_ID> --confirm <CONFIRMATION_ID> --request-id <REQUEST_ID>
```

At a terminal the CLI prompts for the passphrase and finishes the same operation. Verified end to end with a throwaway key: `publication` `published`, the remote's main branch holds the merge, the ticket's branch and worktree are gone. That ran within the confirmation's ten minutes; the contract says an accepted confirmation stands for its request, and a retry after expiry was not run. Afterwards read the ticket with `ticket show` and check `closure.state`. If the confirmation has expired, prepare again, show the person the preview again, and give them the new command. If the person does not finish it, the started close stays in `operation list` (a `close` operation, `confirmed`) and the ticket stays open on its branch: nothing is lost, and [`manyhands-recovery`](../manyhands-recovery/SKILL.md) covers finishing it (the same confirm at their terminal; `operation abandon` does not apply to a close and answers `operation_not_found`). See [keys and hosts](../manyhands-keys-and-hosts/SKILL.md).

## Close in a pull-request repository

`repo inspect` shows `data.configuration.integration`: `pull_request` means the repository works by pull requests ([repositories](../manyhands-repositories/SKILL.md), "Pull-request mode"), and on a build with the pull-request close (Wave 06 Cycle 03) `ticket close` there is a different verb in effect. It never commits to, merges into or pushes the main branch, and it does not remove the ticket's branch or worktree. It records the closure on the ticket's branch, publishes that branch, and answers with where to open the pull request. The person, or the forge, merges it later. This is for tickets only: `document promote` in a pull-request repository still merges into the primary and pushes it until Wave 06's Cycle 04. The preview, the answers and the stops below are from the contract and the library tests: none of it was run here against a real forge, and the one case that runs the whole path over SSH is a test in the source tree (`a_pull_request_close_publishes_only_the_ticket_branch`).

It is still a confirmed verb: show then wait ([autonomy](../manyhands-overview/references/autonomy.md)), prepare with `--prepare`, confirm with `--confirm` and a new `--request-id`, and keep the request ID. A prepare reads local state only. It does not look at the main checkout's branch or cleanliness, because nothing touches the main branch, so `primary_dirty` and `primary_not_checked_out` do not arise; a merge, rebase or cherry-pick in progress in the main checkout is still refused (`primary_conflicted`), and `primary_missing` still names a remote's main branch that was never read (`poll once`, then prepare again, as in "A prepare that is blocked" below).

```sh
manyhands-cli --json --non-interactive ticket close --repo <repo-root> --id <ITEM_ID> --prepare
manyhands-cli --json --non-interactive ticket close --repo <repo-root> --id <ITEM_ID> --confirm <CONFIRMATION_ID> --request-id <REQUEST_ID>
```

### The preview

`data.details.type` is `pull_request_closure_preview`, and `details.action` is `close` (the usual case) or `new_branch` ("The ticket's branch is gone" below). Tell the person in plain words:

- Which ticket: `details.slug` and `details.branch`. The preview carries no title: take it from `ticket show`.
- `data.effects`, in order: `context_synchronized` (the ticket's branch is first brought up to date with its remote copy and the remote's main branch), `closure_recorded` (one commit on the branch holds `closed_at` and `closed_by`), `context_published` (the branch is pushed) and `pull_request_link`. When the branch already holds a closure (`details.closure.recording` `existing`) the effects are only `context_synchronized` and `pull_request_link`.
- `details.closure.closed_by`: whose name goes on the closure, from the repository's commit identity.
- `details.pull_request`: `source` (the ticket's branch), `target` (the main branch), `forge` and `url`. `url` is null when no link can be built from the remote and the forge setting: the person then opens the pull request by hand from `source` into `target`.
- That nothing reaches the main branch until a pull request merges, that the ticket's branch and worktree stay, and that nothing is deleted. The "Ignored files in the worktree" steps above do not apply to this close.

### The answer

| Answer | Meaning and next step |
| --- | --- |
| exit 0, `ok`, `data.type` `pull_request_closure` | The closure is recorded and the branch is published. `data.closure.recording` is `recorded` with `closed_at`, `data.publication` is `published` or `current`, `data.checkpoint_oid` is the commit, and `data.pull_request` has the link (human output prints `pull request: <url>`, or `open a pull request from <source> into <target>` when there is no URL). Give the person the link. The data never holds `closed_by`, a title or a body. |
| exit 0, `already_closing`, `outcome` `success` | The ticket's branch already held a closure (its own earlier one, or another clone's): nothing new was written, the branch was synchronized and published as needed, and the link is in `data.pull_request`. It is a success, not a no-op: give the person the link and do not retry. |
| exit 3, `ticket_closed` | The main branch already holds the closure, also when the pull request merged while the close ran. Nothing to do. |
| exit 3, `remote_branch_deleted` | The ticket's remote branch was deleted, usually because a pull request from it merged. Prepare again: once the deletion is recorded the preview offers the new branch (`details.action` `new_branch`); if it still previews `close`, run `poll once` and prepare again. |
| `recovery_required` with `data.type` `pull_request_closure` | The publication remote was changed or removed while the close ran. The closure is on the local branch and nothing is lost. Once a publication remote is selected again ([repositories](../manyhands-repositories/SKILL.md)), run `item sync` of the ticket ([sharing and sync](../manyhands-sharing-and-sync/SKILL.md)), which publishes it; human output names that command in its `next:` line. |
| `recovery_required` naming a request | An unfinished close or withdraw holds the ticket: "A close that stopped" below. |
| exit 3, `setup_pending` | The repository's setup has not merged yet: [recovery](../manyhands-recovery/SKILL.md), "A pending setup". |

### The state afterwards

`ticket show` reads the ticket `closing`: `closure.state` `closing`, with `closed_at` and `closed_by`, because the closure is on the ticket's branch and not on the main branch yet. `closure.publication` is `pending` until the branch is published and `published` after, and `pull_request` holds the link this clone gave. A closing ticket still counts as open for its dependents: they stay blocked, with the reason `closing_dependency`, until the main branch holds the closure. `ticket ready`, `ticket blocked` and `ticket plan` leave the closing ticket itself out, so find it with `ticket list --closure closing` ([finding work](../manyhands-finding-work/SKILL.md)). It reads `closed` once the pull request is merged and a `poll once` has brought the main branch forward. The close does not report the dependents it will make ready: read `ticket ready` after the merge. Its branch and worktree stay until they are retired, which is the existing `context retire` and `poll once --retire-completed`, and they need the remote branch to be gone.

While it is closing, comments and saves of its text and status are allowed. A save that would change `closed_at` or `closed_by` is `invalid_input` (in both modes), and once the main branch holds the closure every write answers `ticket_closed`. A ticket whose closure is only on its branch can still be withdrawn.

### A ticket whose branch merged first, or that has no branch

If the pull request merged before the close and the remote branch still exists, the close records the closure on that branch after bringing the main branch in: the pull request then holds only the closure. If the remote branch is gone, or the ticket has no branch, the preview's `details.action` is `new_branch`: close makes the ticket's branch again from the remote's main branch as this clone last fetched it, with one commit that holds only the closure (a provisional closure the old branch held is kept), and publishes it. Nothing on the remote is deleted or replaced. For a `new_branch` preview, also tell the person:

- `details.retired_context`: the old branch's tip, which is retired locally: its worktree and branch are removed.
- `details.unheld`: each path (`path`, with `change` `added`, `modified` or `deleted`) that the old branch holds and the main branch does not. Human output prints them as `not on <primary>: <change> <path>`. An empty list means the main branch holds all of the old branch's work. A non-empty list means that work is not on the main branch, and after the close the old branch's commits are no longer referenced by a branch in this clone: the person must decide before you confirm.
- `.manyhands/worktrees/<id>` must be free: otherwise the prepare answers `occupied_path`.

Blockers of this prepare, besides those above: `remote_context_unmaterialized` (a remote branch of the ticket that neither main branch holds: the way out offered is `poll once`, then prepare again; if it persists, tell the person, because a tip that only the local main branch holds is not one a poll brings in), `remote_context_deleted` (the ticket's remote branch is gone and the remote's main branch does not hold the ticket; the way out is `item republish`, [sharing and sync](../manyhands-sharing-and-sync/SKILL.md), "A deleted branch") and `context_missing` for a ticket with no branch that the remote's main branch does not hold.

### A close that stopped

A step that stops on its synchronization (`transport_unavailable`, `unlock_required`, `host_approval_required`, `busy`, a conflict) answers as `item sync` answers that stop, with `recovery` offering `request.retry`: send the same `--confirm` with the same `--request-id` once the cause is gone, and the close continues where it stopped. A conflict is resolved by `conflict show` and `conflict resolve` with the close's operation ID ([sharing and sync](../manyhands-sharing-and-sync/SKILL.md), "A conflict"), which are the person's choice. Before the closure commit is written the stop left nothing written. After it, `ticket show` reads the ticket `closing` with `publication` `pending`, and the record holds the ticket until its request runs again: saves, comments, `ticket repair`, `context retire`, `item sync` and the poll of that ticket answer `recovery_required` or `busy` naming the request, and a new request's prepare is `recovery_required` naming it. The held prepare's `next:` line names the holder's own verb with the holder's request ID (`ticket close` or `ticket withdraw-closure`), so run that line, not your own verb. Do not start a second close, and `operation abandon` does not apply. `operation resume` answers `original_request_required` naming the request. `request cancel` answers `cancelled` (exit 130) and leaves the close unfinished, and the same request continues it ([recovery](../manyhands-recovery/SKILL.md)).

A request that finished cannot always be sent again late. Once the ticket's branch has been retired after the merge, the same request answers `recovery_required` unless the main branch holds the closure's commit: a merge commit or a fast-forward lets it replay, a squash or a rebase merge does not. Do not retry a finished close's request after the merge: read the ticket, and a new request would answer `ticket_closed`. A late retry of an `operation resume` that finished a close meets the same limit.

## Withdraw a closure

`ticket withdraw-closure` takes back a closure that is only on the ticket's branch: it commits the ticket without `closed_at` and `closed_by`, publishes the branch, and the ticket reads `open` again. It is not a reopen: it refuses when the main branch holds the closure. It needs a publication remote, works in either mode, is a confirmed verb and takes show then wait ([autonomy](../manyhands-overview/references/autonomy.md)), and the use for it is a pull request that was declined while its closure stays recorded: say that, and ask. After it the person declines or closes the pull request on the forge, or leaves it open for more work and closes again later with `ticket close`.

```sh
manyhands-cli --json --non-interactive ticket withdraw-closure --repo <repo-root> --id <ITEM_ID> --prepare
manyhands-cli --json --non-interactive ticket withdraw-closure --repo <repo-root> --id <ITEM_ID> --confirm <CONFIRMATION_ID> --request-id <REQUEST_ID>
```

The preview is the close's, with `details.action` `withdraw`, `details.closure.recording` `existing` and a null `details.pull_request`; its effects are `context_synchronized`, `closure_withdrawn` and `context_published`. The answers:

| Answer | Meaning and next step |
| --- | --- |
| exit 0, `ok`, `data.type` `pull_request_closure` | Withdrawn and published: `data.closure.recording` is `withdrawn`, and `ticket show` reads the ticket `open`. Its dependents are blocked by `open_dependency` again. The link record is kept. |
| exit 0, `already_applied` at prepare, `noop`, no data | Neither the ticket's branch nor its remote copy holds a closure: nothing to confirm. |
| exit 3, `ticket_closed` | The main branch holds the closure (`data.blockers` has `already_closed`), also when the pull request merged while the withdraw ran: it is too late. Nothing was reopened. |
| exit 3, `publication_remote_required` | No publication remote is selected, and a withdraw publishes. [Repositories](../manyhands-repositories/SKILL.md). |
| exit 3, `recovery_required` naming a request | A close or withdraw of this ticket is unfinished and holds it: "A close that stopped" above. A withdraw's stops, hold and cancellation are the close's. |
| exit 3, `remote_branch_deleted` | The ticket's remote branch is gone: its withdraw stays on the local branch. `item republish` publishes it ([sharing and sync](../manyhands-sharing-and-sync/SKILL.md), "A deleted branch"), or a prepare again if nothing was written. |

## Codes you will meet

| Code (exit) | Meaning and next step |
| --- | --- |
| `external_change` (3) | The token is stale. Read again, merge, save with the new token and a new request ID. |
| `ticket_closed` (3) | The ticket is closed: no save, slug-assign or second close, and no withdraw. In a pull-request repository only the main branch's closure counts: a closing ticket still takes saves and comments. |
| `already_closing` (0) | A pull-request close of a ticket whose branch already holds a closure: a success with the link, nothing new written. |
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
| `not_repository_root` (2) | A write had a worktree as `--repo` (reads accept one). Use the main checkout root. |

Not run for this skill: a close whose remote moved since you prepared (another person's work arrived), `history_unresolved` and `history_unknown`, and a remote reached with a host key that changed. The contract describes them; take any such answer to [`manyhands-recovery`](../manyhands-recovery/SKILL.md) and tell the person.
