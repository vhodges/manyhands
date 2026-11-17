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
| The main branch | `repo sync` | Fetches the main branch, brings it up to date, and pushes it. It does not touch item branches. |
| Items other people started | `poll once` ([`manyhands-polling-and-index`](../manyhands-polling-and-index/SKILL.md)) | Fetches, and creates the local branch and worktree for an item you did not have. It never pushes or merges. |

The verified behavior behind this table, from two clones of one remote:

- `item sync` published an item's branch and left main alone; `repo sync` then published main and left the item branches alone. A save made on an item is not shared by `repo sync`.
- A second clone did not see a ticket that the first had published until it ran `poll once`; `item sync` on an ID the clone does not have answers `item_not_found`.
- A closed ticket has no branch any more: `item sync` on it answers `item_not_found` and `item republish` answers `ticket_closed`. Closing and promoting publish main themselves ([tickets](../manyhands-tickets/SKILL.md), [documents](../manyhands-documents/SKILL.md)).
- A sync that only brings a branch forward from the remote can answer `noop` / `already_applied` (observed for `repo sync`, where local main moved to the remote's tip, and for `item sync`, where the item's branch gained the remote's commit). Do not read that code as "nothing arrived". Read the item afterwards (`ticket show`, `document show`, `comment list`), or compare a branch before and after with `git rev-parse <branch>`, which is a read.
- Merging is a merge, never a rebase or a force-push. Files you have not saved through Manyhands are not included.

Sharing needs a publication remote, a selected key and a pinned host: [repositories](../manyhands-repositories/SKILL.md), [keys and hosts](../manyhands-keys-and-hosts/SKILL.md). Unsaved work is not shared: save it first ([documents](../manyhands-documents/SKILL.md), [tickets](../manyhands-tickets/SKILL.md)). A comment publishes the item's branch itself ([comments](../manyhands-comments/SKILL.md)).

You run `item sync` and `repo sync` alone ([autonomy](../manyhands-overview/references/autonomy.md)). They take no `--prepare` (`not_confirmable`) and need `--request-id`.

```sh
manyhands-cli --json --non-interactive item sync --repo <repo-root> --id <ITEM_ID> --request-id <REQUEST_ID>
manyhands-cli --json --non-interactive repo sync --repo <repo-root> --request-id <REQUEST_ID>
```

### Operate per item

An item has its own branch and worktree, and so its own sync result. To share "everything", list the items that have work in progress (`ticket list`, `document list`: an item with `context.kind` `active` has a branch) and run `item sync` once per item with its own `--request-id`, then `repo sync` for main. Read each answer on its own: one item can conflict while the others publish. Report per item, not as one result.

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
| `recovery_required` (3), `data.conflict` | A merge conflicts. Nothing was lost; the sync stops until someone chooses. | "A conflict" below. |
| `remote_branch_deleted` (3) | The item's branch was published once and has since been deleted on the remote. | "A deleted branch" below. |
| `unlock_required` (3) | The key has a passphrase. | "Passphrase" below. |
| `publication_remote_required` (3) | No publication remote. | "No remote" above. |
| `host_approval_required`, `host_replacement_required`, `key_rejected` (3) | The host is not pinned, its key changed, or it refused the key. | The person decides: [keys and hosts](../manyhands-keys-and-hosts/SKILL.md). |
| `busy` (5) | Another request holds the repository. | Retry the same request after a pause. A sync that was blocked (`unlock_required`, a conflict, `remote_branch_deleted`) keeps the repository busy for later syncs until it is finished or cancelled: finish the blocked one first. A `comment add` whose publication was blocked (`unlock_required`, or held by a waiting conflict) counts as a blocked sync. Who cancels: you may cancel your own blocked request (`request cancel`, see "A deleted branch"); cancelling is final, and a new request redoes the work. Never cancel one that is not yours, and a request that waits for the person's passphrase is finished by the person at their terminal instead. |
| `item_not_found` (2) | This clone has no such item, or it is closed. | `poll once` for items from others; `ticket list`. |

## A conflict

A conflict means the same file was changed here and on the remote (or by another comment publication), and Manyhands will not choose. `item sync`, `repo sync`, `comment add` and others stop with `recovery_required` and `data.conflict` `{operation_id, next_action}`, where `next_action` is `resolve`. `comment add` also saved the comment (exit 4, see [comments](../manyhands-comments/SKILL.md)); the conflict is in the item. While a conflict is pending, reads of that item can look broken (`comment list` shows every comment with a problem and a stale index); resolve the conflict first.

A conflict that waits holds other work too, so tell the person when you ask for their choice. A remote conflict on one item (`operation list` shows it as `interrupted`, next action `resume`, with its `item_id`) holds the publication of other items: a `comment add` on another ticket is saved but answers `partial` / `busy` (exit 4), and stays `busy` until the conflict is resolved, then the same request publishes; a `ticket save` on another ticket still works. A local-sync conflict ([local context sync](references/local-context-sync.md)) holds every write in the repository: `comment add` and `ticket save` on other items answer `blocked` / `recovery_required` (exit 3) and write nothing. While you wait, do not mint new comments for the held ones: resend the same request afterwards.

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
- If the branch moved, or the remote advertises it again, the confirm answers `external_change` and pushes nothing: prepare again.
- A blocked `item sync` holds the repository until it is cancelled and sent once more, so the confirm answers `busy` (exit 5) meanwhile (the prepare still works). Cancel the blocked sync by its request ID (it is in that answer's `recovery`, `request.cancel`), send that same sync once more (it answers `cancelled`), then prepare, show and confirm. Verified in both orders: a prepare made before the cancel was still good afterwards, and the same confirm with the same confirmation ID and request ID that had answered `busy` succeeded once the blocked sync was cancelled, within the ten minutes.
- `request cancel` is yours to run alone, but only for your own blocked request ([autonomy](../manyhands-overview/references/autonomy.md)). It takes only `--request-id`.

```sh
manyhands-cli --json request cancel --request-id <REQUEST_ID>
```

## Passphrase keys

With a passphrase-protected key every network step answers `unlock_required`, and you cannot enter the passphrase. Never ask for it or put it anywhere. Hand the step to the person: give them the same command, without `--json --non-interactive`, with the same `--request-id`, which `recovery` names (`request.retry`), so they run it at their terminal and type the passphrase there. Verified: `item sync` answered `unlock_required` (exit 3, nothing written), the same command and request ID at a terminal prompted for the passphrase and published, and a later sync answered `already_applied`. `comment add` saves first, then answers `unlock_required` with exit 4 ([comments](../manyhands-comments/SKILL.md)). The mechanics are in [keys and hosts](../manyhands-keys-and-hosts/SKILL.md).

A blocked sync leaves an operation unfinished: `operation list` shows it in an interrupted state, such as `fetch_prepared`, `fetch_observed` (after `remote_branch_deleted`), `reconciling` (after a comment's `unlock_required`) or `interrupted` (a waiting conflict). While it is, later syncs answer `busy`. A blocked `repo sync` also makes every write (`remote add`, `ticket create`, `ticket save`, `comment add`, `index refresh`) answer `recovery_required` (verified); a blocked `item sync` blocks only `comment add` on that item, and `ticket create`, `index refresh` and `remote add` still worked. The way out is to have the person finish the blocked request at their terminal; then `operation list` is empty and `remote add` works. If you did not see the blocked request's own answer, so you do not know its request ID, the person can finish it with `operation resume` at their terminal: see [`manyhands-recovery`](../manyhands-recovery/SKILL.md), "A sync waiting for a passphrase".

## Local sync without a network

To bring the local main branch into one item's branch without any remote, see [local context sync](references/local-context-sync.md): `context observe-local`, `context sync-local`, and, on a conflict, `context show-local` and `context resolve-local`. It never publishes; follow it with `item sync` to share.

## Codes you will meet

The table in "Read the answer" is the list. For anything else, read `message`, `data` and `recovery`, and stop and tell the person if you cannot act on it.
