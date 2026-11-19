---
name: manyhands-comments
description: "Use when reading, adding or replying to a comment on a Manyhands ticket or document, or when a comment command answers ticket_closed, item_not_found, invalid_input, occupied_path, recovery_required, busy or unlock_required."
---

# Comments

Read the [overview](../manyhands-overview/SKILL.md) first. Who runs what is in [autonomy](../manyhands-overview/references/autonomy.md); input files, `--request-id` and the main checkout root are in [CLI conventions](../manyhands-overview/references/cli-conventions.md); codes are in [errors and exit codes](../manyhands-overview/references/errors-and-exit-codes.md). Looking at an item's comments among the rest of its data is in [finding work](../manyhands-finding-work/SKILL.md). This skill says only what is specific to comments.

A comment belongs to one item (a ticket or a document) and is its own small file, written by the CLI. You never write a comment file yourself. `comment add` is a write, so `--repo` is the repository's main checkout root, never a worktree. Keep your `--input` files in a temporary directory outside the repository, and delete them when done, except one that a hand-off to the person still needs: keep it, and give its absolute path in the command you hand over, until the person has run it.

A comment is a push. `comment add` saves the comment on the item's own branch and then publishes that branch to the publication remote, so a comment on another ticket pushes that ticket's branch, with everything saved on it. Tell the person it was published, or that it could not be. Sharing the rest is in [sharing and sync](../manyhands-sharing-and-sync/SKILL.md).

## Read the comments

```sh
manyhands-cli --json comment list --repo <repo-root> --id <ITEM_ID>
```

`data.items[]` is one flat list, each entry with `id`, `parent_id` (null for a root comment), `author` (`Name <email>`), `created_at`, `body`, `depth` (0 for a root, 1 for a reply, 2 for a reply to a reply) and `path`, and `problems` (empty for a sound comment). A reply follows its parent. `data.context` names the item's branch and worktree, and `data.index.state` says whether the index is current. An item with none gives an empty list; an ID that is not an item gives `item_not_found`. There is no verb to show a single comment.

Read with `--json`. The human output prints only the thread (id, author, time, indented by depth) and no body.

If every comment shows a problem `missing_comment_item`, with `index.state` `stale`, the item has an unresolved conflict. The entries then show `id` null and `body` null, and the CLI's problem text says "Restore the item the comment belongs to, or remove the comment": ignore the "remove". The comments are not lost. Find the conflict (a sync answered `recovery_required` with `data.conflict`; if that answer is gone, `operation list` shows the operation ID and its `item_id`) and resolve it first, in [sharing and sync](../manyhands-sharing-and-sync/SKILL.md).

## Add a comment

Content is one line per paragraph, with a blank line between paragraphs and no hard wrap at a column: the editors do not reflow. In the JSON file a body is one string, paragraphs joined by `\n\n`, with no `\n` inside a paragraph.

Mint a new `comment_id` for every comment with `manyhands-cli id new`, and a different new `--request-id` for the write. The comment ID is neither the item ID nor a request ID. The IDs in the examples are placeholders: never reuse one.

| Verb | Required | Optional |
| --- | --- | --- |
| `comment add` | `comment_id` (a ULID from `id new`), `body` (string, may be empty) | `parent_id` (the ID of a comment on the same item, or null for a root comment; leaving it out is the same as null); `created_by` (string, `Name <email>`) |

```json schema=comment_add
{
  "comment_id": "<COMMENT_ID>",
  "body": "First paragraph on one line.\n\nSecond paragraph on one line."
}
```

```sh
manyhands-cli --json --non-interactive comment add --repo <repo-root> --id <ITEM_ID> --input comment.json --request-id <REQUEST_ID>
```

`--id` names the item the comment is on. `comment add` takes no `--prepare`: it answers `not_confirmable` (exit 2), and per [autonomy](../manyhands-overview/references/autonomy.md) you add comments alone.

Another writer may be working in the repository. On a build with the concurrent-writers change a comment that meets a live peer's unfinished operation on its item waits up to 5 seconds and then answers `busy` with `data.blocker`, writing nothing; send the same request again. A peer's operation on another item does not hold it. A build without that change needs the old care, one comment at a time and a look at `operation list` after each: [CLI conventions](../manyhands-overview/references/cli-conventions.md), "Other writers in the repository", which says how to tell the build.

### Replies

A reply names its parent comment. A reply to a reply works, one level deeper each time (the list stays flat).

```json schema=comment_add
{
  "comment_id": "<COMMENT_ID>",
  "parent_id": "<PARENT_COMMENT_ID>",
  "created_by": "Agent Helper <agent@example.com>",
  "body": "A reply on one line."
}
```

Run it with the same command line as above. A `parent_id` that is not a comment on this item answers `item_not_found` ("No item has that ID"), with nothing written.

### Who the comment is by

Leave `created_by` out and the comment is by the repository's commit identity, which is the person. Set it only when the person wants the comment attributed to someone else or to you, and ask for the name and email: do not invent an address. The value must be `Name <email>`: a name, one space, then an email with an `@` in angle brackets, with no leading or trailing white space and no `<` or `>` inside either part, at most 256 bytes. A bare value such as `agent`, an empty email, or any other shape answers `invalid_input` (exit 2); rerun without `--json` to see it is the input, then fix the value.

## What the answer tells you

Read `outcome`, `code` and `effects` first. `data` holds `comment_id`, `parent_id`, `created_by`, `publication` and `pending_reason`.

| You see | It means | What to do |
| --- | --- | --- |
| `success`, `ok`, `effects.publication` `published` | The comment is saved on the item's branch and the branch is pushed. | Done. If main had moved, `effects.integration` is `complete`: Manyhands saved the comment, then merged main into the item's branch before the push (a "Merge primary into ..." commit) and pushed that too. |
| `success`, `ok`, `publication` `pending`, `pending_reason` `no_publication_remote`, exit 0 | The comment is saved. The repository has no publication remote, so nothing was pushed. | Say so. The comment is shared later by `item sync`, but only once a publication remote has been added and selected, and the person chooses its SSH URL ([repositories](../manyhands-repositories/SKILL.md)). Do not add it again. |
| `partial`, `recovery_required`, exit 4, `data.conflict` holds `operation_id` | The comment is saved, but pushing found that someone else changed the same item, and merging them conflicts. | The comment is not lost and must not be added again. Resolve the conflict by `data.conflict.operation_id`: [sharing and sync](../manyhands-sharing-and-sync/SKILL.md). |
| `partial`, `unlock_required`, exit 4 | The comment is saved. The selected key has a passphrase, so it could not be pushed. | Do not add the comment again: a new `comment_id` would post it twice. Hand the person this same command, without `--json --non-interactive`, with the same `--input` and the same `--request-id` (it is in `recovery`, `request.retry`). Give the absolute path of the `--input` file and keep that file until they have run it. They run it at their terminal, type the passphrase and it publishes. Never ask for the passphrase. See [keys and hosts](../manyhands-keys-and-hosts/SKILL.md). |
| `partial`, `busy`, exit 4, `data.conflict` null, `pending_reason` `synchronization` | The comment is saved. Its publication is held because another item has a conflict waiting (a remote sync stopped on it, and nobody has chosen yet) or because an earlier blocked sync or comment has not finished. Sending it again stays `busy` until that is cleared. | Do not add the comment again and never mint a new `comment_id`. Find what holds the repository: on a build with the change the answer's `data.blocker` names the operation (an unanswered push of a synchronization, another item's conflict, an earlier blocked sync; from the contract, not run), otherwise `operation list`. Resolve or finish it ([sharing and sync](../manyhands-sharing-and-sync/SKILL.md)). Then resend the SAME `--request-id` with the same input; the stored comment is published. Tell the person the comment is saved but not yet shared, and why. |
| `blocked`, `recovery_required`, exit 3, `data.blocker` | Nothing was written. An unfinished operation on this item, or on the whole repository, is in the way and its holder is not live (`live` `false`, or `null` for a legacy row with no host or pid): `data.blocker` names it (`operation_id`, `action`, `item_id`, `request_id`, `author`, `host`, `pid`, `started_at`). `recovery` lists `operation.show` for it, and `operation.resume` first where a resume would take it up. | Do not resume or abandon it unless the [recovery](../manyhands-recovery/SKILL.md) skill, "Whose operation is it?", shows it is yours. Tell the person what is in the way. Once it is cleared, send the comment again with the same comment file and the same `--request-id`; a new `--request-id` also works. |
| `blocked`, `recovery_required`, exit 3, `data` null | A build without the change, or a Git worktree that Manyhands did not make. Nothing was written. Something unfinished or unexpected holds writes in the repository: a waiting local sync conflict (it holds `ticket create`, `ticket save` and every other write too), a passphrase-blocked sync (including `repo sync`), any other unfinished operation. `recovery` offers only `repo.inspect`, which does not name the cause on an older build. | Find the cause with the [recovery](../manyhands-recovery/SKILL.md) skill (`operation list`, then `git worktree list` if the list is empty) and tell the person. Once it is cleared, send the comment again with the same comment file and the same `--request-id`. Nothing was written, so a new `--request-id` also works (both verified before the change). |
| `error`, `busy`, exit 5, `data.blocker` with `live` true | Nothing was written. A peer is working on this item (or on the whole repository), and the CLI waited for it and gave up (a ticket save or create that sets `deps` or `parent` does not wait). | Send the same request again after a pause, with the same comment file and `--request-id` (verified: nothing was written by the busy answer, and the retry wrote the comment once the peer's operation ended). Do not resume or abandon the peer's operation. A `busy` with no `data.blocker` is the repository's file lease: send it again. |
| `blocked`, `ticket_closed`, exit 3 | The ticket is closed. A closed ticket takes no new comment. | Tell the person. Its existing comments still read. |
| `item_not_found` (2) | The item ID is wrong, or the `parent_id` is not a comment on this item. | `ticket list`, `document list`, `comment list`. |
| `invalid_input` (2) | A missing or unknown field, a `comment_id` that is not a ULID, or a `created_by` that is not `Name <email>`. | Rerun without `--json` to read the field. |
| `occupied_path` (2) | The `comment_id` is the same as the item's ID. | Mint a new `comment_id`. |
| `internal_error` (1) | Seen when a `comment_id` that was already saved is sent again under a new `--request-id`. The contract says `occupied_path` for a reused comment ID, so this is probably a product bug. | Stop. Tell the person, with the request ID from the answer. Run `comment list`: the comment is probably there. Never "retry" with another request ID. To repeat a request you do not know finished, send the same `--request-id` and the same input; the stored answer comes back and nothing is written twice. A new comment gets a new `comment_id`. |

`comment add` does not answer `publication_remote_required`: with no remote it succeeds with publication pending, as above. The sync verbs do answer it.

A comment on a document that has already been promoted opens a new branch and worktree for the document again, as a save would, and publishes that branch.

If a command stops at `host_approval_required`, `host_replacement_required` or `key_rejected`, that is the person's decision: [keys and hosts](../manyhands-keys-and-hosts/SKILL.md).
