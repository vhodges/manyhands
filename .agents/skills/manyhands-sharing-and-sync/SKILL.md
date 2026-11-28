---
name: manyhands-sharing-and-sync
description: "Use when sharing a Manyhands ticket or document with collaborators, receiving theirs, running item sync or repo sync, republishing a deleted branch, or when a sync answers recovery_required with a conflict, remote_branch_deleted, publication_remote_required, unlock_required, host_approval_required or busy."
---

# Sharing and sync

Read the [overview](../manyhands-overview/SKILL.md) first. Who runs what is in [autonomy](../manyhands-overview/references/autonomy.md); input files, `--request-id`, prepare and confirm and the main checkout root are in [CLI conventions](../manyhands-overview/references/cli-conventions.md); codes are in [errors and exit codes](../manyhands-overview/references/errors-and-exit-codes.md). This skill says only what is specific to sharing.

Every verb here takes `--repo <repo-root>`, the repository's main checkout root, never a worktree (a write that names a worktree answers `not_repository_root`). Keep your `--input` files in a temporary directory outside the repository, and delete them when done.

## What sharing is

Everything in Manyhands is local until it is published. Sharing means pushing a branch to the repository's publication remote, and receiving means fetching what others pushed. There are three kinds of branch and a verb for each:

| What | Verb | What it does |
| --- | --- | --- |
| One item (a ticket or a document in progress) | `item sync --id <ITEM_ID>` | Fetches that item's branch from the remote, merges it with the local one, merges the local main branch into it, and pushes the item's branch. |
| The main branch | `repo sync` | Fetches the main branch, brings it up to date, and pushes it. It does not touch item branches. In a pull-request repository it only pulls: see "Pull-request repositories" below. |
| Items other people started | `poll once` ([`manyhands-polling-and-index`](../manyhands-polling-and-index/SKILL.md)) | Fetches, and creates the local branch and worktree for an item you did not have. It never pushes or merges. |

The verified behavior behind this table, from two clones of one remote:

- `item sync` published an item's branch and left main alone; `repo sync` then published main and left the item branches alone. A save made on an item is not shared by `repo sync`.
- A second clone did not see a ticket that the first had published until it ran `poll once`; `item sync` on an ID the clone does not have answers `item_not_found`.
- A closed ticket has no branch any more: `item sync` on it answers `item_not_found` and `item republish` answers `ticket_closed`. Closing and promoting publish main themselves ([tickets](../manyhands-tickets/SKILL.md), [documents](../manyhands-documents/SKILL.md)); in a pull-request repository a close publishes the ticket's branch and leaves it until the pull request merges, so `item sync` still works on a closing ticket.
- A sync that only brings a branch forward from the remote can answer `noop` / `already_applied` (observed for `repo sync`, where local main moved to the remote's tip, and for `item sync`, where the item's branch gained the remote's commit). Do not read that code as "nothing arrived". Read the item afterwards (`ticket show`, `document show`, `comment list`), or compare a branch before and after with `git rev-parse <branch>`, which is a read.
- Merging is a merge, never a rebase or a force-push. Files you have not saved through Manyhands are not included.

Sharing needs a publication remote, a selected key and a pinned host: [repositories](../manyhands-repositories/SKILL.md), [keys and hosts](../manyhands-keys-and-hosts/SKILL.md). Unsaved work is not shared: save it first ([documents](../manyhands-documents/SKILL.md), [tickets](../manyhands-tickets/SKILL.md)). A comment publishes the item's branch itself ([comments](../manyhands-comments/SKILL.md)).

You run `item sync` and `repo sync` alone ([autonomy](../manyhands-overview/references/autonomy.md)). They take no `--prepare` (`not_confirmable`) and need `--request-id`.

```sh
manyhands-cli --json --non-interactive item sync --repo <repo-root> --id <ITEM_ID> --request-id <REQUEST_ID>
manyhands-cli --json --non-interactive repo sync --repo <repo-root> --request-id <REQUEST_ID>
```

### Operate per item

An item has its own branch and worktree, and so its own sync result. To share "everything", list the items that have work in progress (`ticket list`, `document list`: an item with `context.kind` `active` has a branch) and run `item sync` once per item with its own `--request-id`, then `repo sync` for main. Read each answer on its own: one item can conflict while the others publish. Report per item, not as one result. A sync takes the repository's synchronization reservation, so a second sync meeting a live one waits for it, up to 5 seconds on a build with the change, and answers `busy` (see the table) if it is still running: send it again, or run them one after another. A build without the concurrent-writers change also needs a look at `operation list` between them ([CLI conventions](../manyhands-overview/references/cli-conventions.md), "Other writers in the repository", says how to tell the build).

## Pull-request repositories

In a repository whose configuration says `integration = "pull_request"` (`repo inspect` shows it as `data.configuration.integration`; [repositories](../manyhands-repositories/SKILL.md), "Pull-request mode"), `repo sync` is pull-only on a build with the pull-request setting (Wave 06 Cycle 01). It fetches the main branch and fast-forwards a local one the remote is ahead of, and it never merges and never pushes it. It answers as a direct fast-forward-only sync does: `already_applied` (0) with `publication` `current`, never `published`. A local main branch with commits the remote lacks is refused with `primary_has_local_commits` (3), with nothing written, pushed or held: [recovery](../manyhands-recovery/SKILL.md), "Local commits on the primary of a pull-request repository". A sync keeps the mode it began with, so one started in direct mode and finished after the switch finishes as direct. `item sync`, `poll once` and a comment's publication are unchanged.

`ticket close` and `ticket withdraw-closure` work by pull request on a build with the pull-request close (Wave 06 Cycle 03): a close records the closure on the ticket's branch, publishes that branch with the ordinary context publication and answers with the link, and neither it nor a withdraw ever commits to or pushes the main branch ([tickets](../manyhands-tickets/SKILL.md), "Close in a pull-request repository"). `document promote` works by pull request too, on a build with the pull-request promote (Wave 06 Cycle 04): it brings the document's branch up to date, publishes it with the same context publication, answers with the link, and never commits to or pushes the main branch ([documents](../manyhands-documents/SKILL.md), "Promote in a pull-request repository").

What `item sync` does with a closing ticket, in a pull-request repository: when it publishes the ticket's branch, or finds it already current, and the ticket holds a closure that the main branch does not, the answer's `data` is `{type: "pull_request_link", pull_request: {source, target, forge, url}}` (`url` may be null) and the link is recorded for the ticket, so `ticket show` offers it again. Every other `item sync` answer is unchanged, and a closing ticket's `item sync` is also how a close whose publication remote changed finishes ("Read the answer" below). Give the person the link.

A document gets the same answer on a build with the pull-request promote (Wave 06 Cycle 04): when `item sync` publishes the document's branch, or finds it current, and this clone has a link for the document, and the main branch does not hold the document after the sync, `data` is the `pull_request_link` shape with the link that `document promote` gave. `item sync` only reads that link: it never gives a first one and never records one, so a document that was never promoted, or one the main branch holds, gets an ordinary answer. Give the person the link when it comes.

A ticket context's `item sync`, in either mode, never publishes a head whose ticket lacks a closure that the main branch it fetched holds: a clean merge is given that closure, and otherwise the sync answers `ticket_closed` (exit 3, `blocked`, no `recovery`), which ends the operation and writes nothing more. Read it as: the ticket is closed on the main branch, so there is nothing of this ticket's branch to publish; tell the person, and see [tickets](../manyhands-tickets/SKILL.md). It happens after a pull request that closed the ticket has merged and the main branch was fetched, and for a withdraw that lost the race to that merge. A pull request merged after the fetch is not seen until the next sync.

Two clones that each recorded a closure settle it themselves when nothing else conflicts: the later clone's `item sync` keeps the closure already published and drops its own, and publishes. It is silent, so after a sync, read the ticket (`ticket show`): its `closed_at` and `closed_by` may be the other clone's. A conflict that holds two different closures and anything else is not settled this way: it is `recovery_required` with a conflict and is repaired outside Manyhands, as before.

While a close or withdraw of the ticket is unfinished and its commit is written, the ticket is held: `item sync` of it answers `busy` or `recovery_required` naming the close's request, and the poll leaves that ticket's branch alone. Send the close's own request again ([tickets](../manyhands-tickets/SKILL.md), "A close that stopped"), not a sync. All of this is from the contract and the library tests, and was not run here.

## Receiving: the usual order

1. `poll once` to see what others published ([`manyhands-polling-and-index`](../manyhands-polling-and-index/SKILL.md)).
2. `repo sync` to bring main up to date.
3. `item sync` for each item you are working on, which merges the new main into it and publishes it.

## No remote

With no publication remote, `item sync` and `repo sync` answer `publication_remote_required` (exit 3, `blocked`, nothing written), and a human run prints `No publication remote is selected.` Saving works locally and has nothing to publish. Commenting, promoting and closing also work locally, and their publication stays `pending`. Tell the person that sharing waits for a remote and that adding one is [repositories](../manyhands-repositories/SKILL.md). Do not invent a remote, and do not edit Git remotes to get past it. A remote must be SSH.

## Read the answer

| `code` (exit) | What happened | What to do |
| --- | --- | --- |
| `ok` (0), `publication` `published` | Pushed. `integration` `complete` means it also merged something. | Done. |
| `already_applied` (0), `noop` | Nothing to push, or only a fast-forward from the remote (see above). | Done. |
| `setup_pending` (3) | The repository's first-time setup has not merged yet, and a sync waits for it. Nothing was written. [Recovery](../manyhands-recovery/SKILL.md), "A pending setup". |
| `primary_has_local_commits` (3) | A pull-request repository's `repo sync` found local commits on the main branch that the remote lacks. Nothing was pushed or held; `data.local_commits` lists up to 20. | Not yours to fix: tell the person, and see [recovery](../manyhands-recovery/SKILL.md), "Local commits on the primary of a pull-request repository". |
| `recovery_required` (3), `data.conflict` | A merge conflicts. Nothing was lost; the sync stops until someone chooses. | "A conflict" below. |
| `recovery_required` (3), `data.blocker` | An unfinished operation is in the way and its holder is not live (`live` `false`, or `null` for a legacy row with no host or pid). It is named: `operation_id`, `action`, `item_id`, `request_id`, `author`, `host`, `pid`, `started_at`. A new sync refused for an unanswered push of another operation names it and offers its abandon. | "A synchronization left unfinished" below. |
| `remote_branch_deleted` (3) | The item's branch was published once and has since been deleted on the remote. | "A deleted branch" below. |
| `unlock_required` (3) | The key has a passphrase. | "Passphrase" below. |
| `publication_remote_required` (3) | No publication remote. | "No remote" above. |
| `ticket_closed` (3), no `recovery` | The main branch holds the ticket's closure, so its branch is not published. | "Pull-request repositories" above. |
| `recovery_required`, `partial` (4), `data.type` `pull_request_closure` | A close whose publication remote changed or was removed, once its commit was written (exit 3 if nothing was): the closure is on the local branch. | Once a publication remote is selected again, `item sync` of the ticket publishes it ([tickets](../manyhands-tickets/SKILL.md)). |
| `host_approval_required`, `host_replacement_required`, `key_rejected` (3) | The host is not pinned, its key changed, or it refused the key. | The person decides: [keys and hosts](../manyhands-keys-and-hosts/SKILL.md). |
| `busy` (5) | Another request holds the repository. On a build with the change `data.blocker` names the operation that holds it, with `live` true: a peer is working, and the CLI already waited for it, up to 5 seconds per attempt (a sync and a comment's publication wait so for a peer's sync too; a ticket save or create that sets `deps` or `parent` answers `busy` at once). Without a `data.blocker` it is the repository's file lease. | Retry the same request after a pause; never resume or abandon a live peer's operation. A sync that was blocked (`unlock_required`, a conflict, `remote_branch_deleted`) keeps the repository busy for later syncs until it is finished or cancelled: finish the blocked one first. A `comment add` whose publication was blocked (`unlock_required`, or held by a waiting conflict) counts as a blocked sync. Who cancels: you may cancel your own blocked request (`request cancel`, see "A deleted branch"); cancelling is final, and a new request redoes the work. Never cancel one that is not yours, and a request that waits for the person's passphrase is finished by the person at their terminal instead. |
| `item_not_found` (2) | This clone has no such item, or it is closed. | `poll once` for items from others; `ticket list`. |

## A conflict

A conflict means the same file was changed here and on the remote (or by another comment publication), and Manyhands will not choose. `item sync`, `repo sync`, `comment add` and others stop with `recovery_required` and `data.conflict` `{operation_id, next_action}`, where `next_action` is `resolve`. `comment add` also saved the comment (exit 4, see [comments](../manyhands-comments/SKILL.md)); the conflict is in the item. While a conflict is pending, reads of that item can look broken (`comment list` shows every comment with a problem and a stale index); resolve the conflict first.

A conflict that waits holds other work too, so tell the person when you ask for their choice. An unfinished authoring write on one item (a save, a comment) holds that item and the repository-wide requests, not writes on other items; a comment on item Y behind an unfinished operation on item X is saved locally, and its publication is refused naming X's operation (not run: no remote). A remote conflict on one item (`operation list` shows it as `interrupted`, next action `resume`, with its `item_id`) holds the publication of other items: a `comment add` on another ticket is saved but answers `partial` / `busy` (exit 4), and stays `busy` until the conflict is resolved, then the same request publishes; a `ticket save` on another ticket still works. A local-sync conflict ([local context sync](references/local-context-sync.md)) holds every write in the repository: `ticket create`, `ticket save` and `comment add` on other items answer `blocked` / `recovery_required` (exit 3) and write nothing (`ticket create` verified for this skill). While you wait, do not mint new comment IDs for the held ones; how to resend a blocked comment is in [comments](../manyhands-comments/SKILL.md).

Which side wins is the person's decision. `conflict resolve` is a hand-over verb unless the person has already said which side wins ([autonomy](../manyhands-overview/references/autonomy.md)). Do the reading, show them both sides, and ask.

1. Read the conflict as JSON. The human output prints only the path and a token, never the content.

   ```sh
   manyhands-cli --json conflict show --repo <repo-root> --operation-id <OPERATION_ID>
   ```

   `data.observation` is the token a resolution needs. `data.paths[]` has one entry per conflicted file: `display_path`, `token`, `kind`, and, when `content_available` is true, the whole file three ways: `base` (the last common version), `local` (this machine's) and `incoming` (the remote's). An unknown operation ID answers `operation_not_found`. `conflict show` is a read.
2. Show the person both sides in plain words: the file, what each side changed relative to `base` (for a ticket, the front matter field that differs, such as `status`, and the body). Do not summarize away a difference. Ask which side wins, or whether to combine them, and wait. If the person already said which side wins, use that side exactly. If they choose to combine, write out the exact resulting text (the whole `source`) and show it to them before you run `conflict resolve`; resolving publishes at once.
3. Build `source`: the complete text of the file as it should end up, front matter included (a ticket's `id`, `slug`, `title`, `type`, `status` and the marker lines). Start from `local` or `incoming` and change only what was decided. Do not alter the `id` or `slug`. Content stays one line per paragraph. A `source` that is not a valid item answers `invalid_input` and writes nothing.
4. Resolve. There is one resolution for every conflicted path, and at least one.

| Verb | Required | Optional |
| --- | --- | --- |
| `conflict resolve` | `observation` (string, from `conflict show`); `resolutions`, a non-empty array of objects, each with `token` and `source` (strings) | none |

```json schema=conflict_resolve
{
  "observation": "<OBSERVATION>",
  "resolutions": [
    {
      "token": "<PATH_TOKEN>",
      "source": "---\nslug: <SLUG>\nid: <ITEM_ID>\ntitle: Shared ticket\ntype: task\nstatus: in_progress\nmanyhands_managed: true\nmanyhands_kind: ticket\n---\nBody paragraph on one line.\n\nSecond paragraph on one line."
    }
  ]
}
```

```sh
manyhands-cli --json --non-interactive conflict resolve --repo <repo-root> --operation-id <OPERATION_ID> --input resolve.json --request-id <REQUEST_ID>
```

The answer is `ok` with `data.type` `conflict_resolution`, the resolution commit in `data.commit_oid`, and, with a remote, `publication` `published`: the resolution is pushed in the same request. `--prepare` answers `not_confirmable`, so the preview is the one you showed the person. Repeating the same request returns the stored answer.

- A wrong `token`, or an `observation` that no longer matches (the conflict changed), answers `external_change` (3) and writes nothing: run `conflict show` again and rebuild.
- An empty `resolutions` list, or a missing field, answers `invalid_input` (2).
- A path with `content_available: false` has no readable sides (not run here): do not guess a `source`; tell the person.
- A local-only conflict (`context sync-local`) has its own verbs, in [local context sync](references/local-context-sync.md); `conflict show` on its operation answers `invalid_input`.
- Afterwards check the item (`ticket show`, `document show`, `comment list`) and say what the item says now.

## A deleted branch: republish

`remote_branch_deleted` means the item's branch was published and then removed on the remote (by someone on the host). Sync never recreates it silently. Whether to put it back is the person's call, so `item republish` is a confirmed verb: show-then-wait ([autonomy](../manyhands-overview/references/autonomy.md)).

```sh
manyhands-cli --json --non-interactive item republish --repo <repo-root> --id <ITEM_ID> --prepare
```

Order: first finish the blocked `item sync` that reported `remote_branch_deleted` (cancel it and send it once more, below), because while it is unfinished the confirm answers `busy`. Then prepare, show the person `data.details` (the `branch`, the `remote`, `last_published_oid` and `local_oid`, and the effect `branch_republished`) and wait for their OK. Confirming pushes exactly that local commit and merges nothing; the next `item sync` merges main. Confirm with a new `--request-id`.

```sh
manyhands-cli --json --non-interactive item republish --repo <repo-root> --id <ITEM_ID> --confirm <CONFIRMATION_ID> --request-id <REQUEST_ID>
```

- With no evidence of a deletion the preview is `already_applied`: run `item sync` first. A closed ticket answers `ticket_closed`.
- A pull-request `document promote` that meets a deleted branch answers `remote_branch_deleted` itself, and its way out is this republish and then a new promote. A pull request that merged, with its branch deleted by the forge, answers `already_applied` instead, and a republish after a squash on which the main branch moved on would revive a merged branch: [documents](../manyhands-documents/SKILL.md), "Promote in a pull-request repository".
- If the branch moved, or the remote advertises it again, the confirm answers `external_change` and pushes nothing: prepare again.
- A blocked `item sync` holds the repository until it is cancelled and sent once more, so the confirm answers `busy` (exit 5) meanwhile (the prepare still works). Cancel the blocked sync by its request ID (it is in that answer's `recovery`, `request.cancel`), send that same sync once more (it answers `cancelled`), then prepare, show and confirm. Verified in both orders: a prepare made before the cancel was still good afterwards, and the same confirm with the same confirmation ID and request ID that had answered `busy` succeeded once the blocked sync was cancelled, within the ten minutes.
- `request cancel` is yours to run alone, but only for your own blocked request ([autonomy](../manyhands-overview/references/autonomy.md)). It takes only `--request-id`.

```sh
manyhands-cli --json request cancel --request-id <REQUEST_ID>
```

## A synchronization left unfinished

A sync that was killed, or lost its connection, mid-push leaves an operation (`operation list`: family `remote`, action `synchronize_context` or `synchronize_primary`). On a build with the change `repo inspect` lists it in `data.operations`; if the operations cannot be read, `repo inspect` still answers, with `operations` empty and the problem `operations_unreadable` in `data.configuration.problems` (verified with a corrupt row): run `operation list` for the reason. A pending setup's record that cannot be read gives `setup` null with the problem `setup_unreadable` there, and is the person's to clear with `repo remove`. Whose it is and whether it still works are in [`manyhands-recovery`](../manyhands-recovery/SKILL.md), "Whose operation is it?": a `live` operation is someone's work in progress, so wait.

Finish a stale one in this order: `operation resume` when your `request_id` leads to it, after telling the person and once the cause is removed, otherwise describing it and waiting ([autonomy](../manyhands-overview/references/autonomy.md)) (an interrupted first push to a context branch the remote has never had is retried and ends `published`; not run here), the same request again, or a new sync. Only when none finishes it is the abandon offered, and it is the person's: `operation abandon --prepare` (a read, no network) shows the evidence, `remote_ref_present`, `local_oid`, `remote_oid`, `push_oid` and `push_returned`, and the effect `push_marked_rejected`; the person runs `--confirm` with the confirmation id. Details and what the evidence means are in recovery, "Abandon and removals are the person's".

## Passphrase keys

With a passphrase-protected key every network step answers `unlock_required`, and you cannot enter the passphrase. Never ask for it or put it anywhere. Hand the step to the person: give them the same command, without `--json --non-interactive`, with the same `--request-id`, which `recovery` names (`request.retry`), so they run it at their terminal and type the passphrase there. Verified: `item sync` answered `unlock_required` (exit 3, no content written), the same command and request ID at a terminal prompted for the passphrase and published, and a later sync answered `already_applied`. `comment add` saves first, then answers `unlock_required` with exit 4 ([comments](../manyhands-comments/SKILL.md)). The mechanics are in [keys and hosts](../manyhands-keys-and-hosts/SKILL.md).

A blocked sync leaves an operation unfinished: `operation list` shows it in an interrupted state, such as `fetch_prepared`, `fetch_observed` (after `remote_branch_deleted`), `reconciling` (after a comment's `unlock_required`) or `interrupted` (a waiting conflict). While it is, later syncs answer `busy`. A blocked `repo sync` also makes every write (`remote add`, `ticket create`, `ticket save`, `comment add`, `index refresh`) answer `recovery_required` (verified); a blocked `item sync` blocks only `comment add` on that item, and `ticket create`, `index refresh` and `remote add` still worked. The way out is to have the person finish the blocked request at their terminal; then `operation list` is empty and `remote add` works. If you did not see the blocked request's own answer, so you do not know its request ID, the person can finish it with `operation resume` at their terminal: see [`manyhands-recovery`](../manyhands-recovery/SKILL.md), "A sync waiting for a passphrase".

## Local sync without a network

To bring the local main branch into one item's branch without any remote, see [local context sync](references/local-context-sync.md): `context observe-local`, `context sync-local`, and, on a conflict, `context show-local` and `context resolve-local`. It never publishes; follow it with `item sync` to share.

## Codes you will meet

The table in "Read the answer" is the list. For anything else, read `message`, `data` and `recovery`, and stop and tell the person if you cannot act on it.
