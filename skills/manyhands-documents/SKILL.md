---
name: manyhands-documents
description: "Use when creating, editing, saving, moving, repairing or promoting a Manyhands document (a Markdown file under docs/), creating a document folder, or when a document command answers external_change, occupied_path, invalid_path, recovery_required or confirmation_required."
---

# Documents

Read the [overview](../manyhands-overview/SKILL.md) first. Who runs what is in [autonomy](../manyhands-overview/references/autonomy.md); input files, `--request-id`, prepare and confirm, observation tokens and the main checkout root are in [CLI conventions](../manyhands-overview/references/cli-conventions.md); codes are in [errors and exit codes](../manyhands-overview/references/errors-and-exit-codes.md). Look before you write: [finding work](../manyhands-finding-work/SKILL.md) shows what exists. This skill says only what is specific to documents. Every verb here takes `--repo <repo-root>`, the repository's main checkout root, never a worktree.

A managed document is a Markdown file under `docs/` with front matter that Manyhands writes (`id`, `title`, and two marker lines). You never write that front matter yourself: you give `title` and `body` and the CLI makes the file. Never edit a document file with an editor or `git`; use the verbs below. Keep your `--input` files in a temporary directory outside the repository, and delete them when done.

## The lifecycle

1. `folder create` (optional): make a folder to put documents in.
2. `document create`: new document, new item ID, its own branch and worktree.
3. `document show`, then `document save`: read, then edit, as often as needed. Each save is a checkpoint commit on the document's branch.
4. Share it: `manyhands-sharing-and-sync`.
5. `document promote`: merge the document into the main branch. Show the person first and wait ([autonomy](../manyhands-overview/references/autonomy.md)).

A created document lives only on its own branch and worktree (`manyhands/document/<id>`, `.manyhands/worktrees/<id>`) until it is promoted. It is not in the main checkout's `docs/` yet, so `ls docs` and `document show --path docs/a.md` do not find it. Find it with `document list` or by ID (`document show --id`). After promotion the branch and worktree are gone; a later save makes a new branch and worktree again.

## Content: one line per paragraph

Write a body as one line per paragraph, with a blank line between paragraphs. Never wrap a line at a column: the editors do not reflow, so a hard-wrapped paragraph stays broken in every view. In the JSON file a body is one string, paragraphs joined by `\n\n`, with no `\n` inside a paragraph.

## Folders

```json schema=folder_create
{
  "path": "docs/guides"
}
```

```sh
manyhands-cli --json --non-interactive folder create --repo <repo-root> --input folder.json --request-id <REQUEST_ID>
```

| Verb | Required | Optional |
| --- | --- | --- |
| `folder create` | `path` (string, repository-relative, under `docs/`) | none |

- The path must be below `docs/`: `docs` itself and `notes/x` answer `invalid_path` (exit 2). Nested paths such as `docs/a/b` work, and the parent folders are listed too.
- A folder is an empty directory in the main checkout. Git does not see an empty directory, so nothing is committed and nothing is shared until a document is promoted into it. An existing folder answers `already_applied`. `--prepare` answers `not_confirmable`.
- A folder is a convenience, not a precondition: `document create` at `docs/nofolder/a.md` worked with no folder made first (verified). `folder list` shows folders, each with `path` and `empty`.

## Create a document

Mint a new `item_id` for every document with `manyhands-cli id new` and a different new `--request-id` for the write. The ID in the example is a placeholder: never reuse an ID you already used or copied. Creating two documents means two `id new` calls.

| Verb | Required | Optional |
| --- | --- | --- |
| `document create` | `item_id` (a ULID from `id new`), `path` (string), `title` (string), `body` (string, may be empty) | none |

```json schema=document_create
{
  "item_id": "<ITEM_ID>",
  "path": "docs/guides/intro.md",
  "title": "Intro",
  "body": "First paragraph on one line.\n\nSecond paragraph on one line."
}
```

```sh
manyhands-cli --json --non-interactive document create --repo <repo-root> --input create.json --request-id <REQUEST_ID>
```

The answer's `data` is `{id, path}`, `scope` holds the new `branch` and `worktree`, and `effects.checkpoint` is `committed`. `--prepare` answers `not_confirmable`: create runs alone.

Choosing the path:
- Under `docs/`, ending in lowercase `.md`. `notes/a.md`, `docs/a.txt`, `docs/a.MD`, `docs/noext`, `../x.md` and an absolute path all answer `invalid_path`.
- Check `document list` first. A path that a file already has on the main branch answers `occupied_path` (verified with an existing `docs/plain.md`). A path that another unpromoted document uses is not refused at create: the second document gets its own branch with the same path, and the clash shows up at promotion (below). Pick an unused path.
- Reusing an `item_id` for a document with other content or a different path answers `occupied_path` (exit 2); the same content again is `already_applied` with nothing written (both verified). If you see `occupied_path` and think the path is free, you reused an ID: mint a new one.
- A missing field, or an `item_id` that is not a ULID, answers `invalid_input`. Rerun without `--json` to read which field.

## Read before you write

```sh
manyhands-cli --json document show --repo <repo-root> --id <ITEM_ID>
```

`data.observation` is the token a save needs, `data.title` and `data.body` are the content, `data.source` is the whole file, and `data.context.worktree` is the worktree. The human output does not print the token, so read with `--json` when you need it. `document show --path <path>` works only for a path on the main branch, or with `--context <worktree>` for a document's own worktree (`--context` requires `--path`; without it a path that is not promoted answers `path_not_found`). Prefer `--id`.

## Save: a patch

A save names the item with `--id` and sends a patch; the token comes from the show you just did.

| Verb | Required | Optional |
| --- | --- | --- |
| `document save` | `observation` (string) | `title`, `body` (strings; neither accepts null) |

```json schema=document_save
{
  "observation": "<OBSERVATION>",
  "body": "Revised paragraph on one line.\n\nSecond paragraph on one line."
}
```

```sh
manyhands-cli --json --non-interactive document save --repo <repo-root> --id <ITEM_ID> --input save.json --request-id <REQUEST_ID>
```

- A field you leave out stays as it was (verified: a title-only save kept the body). A body is replaced whole, so send the complete new body. An empty string `""` empties the body; omit `body` to keep it. `"title": null` is `invalid_input`.
- A save that changes nothing, such as the observation alone or a body that equals the current one, answers `already_applied` and writes nothing.
- `--prepare` answers `not_confirmable`: save runs alone.
- After a save the old token is spent. Read again for the next save.

## A stale observation: read again and merge

A save that carries a token which no longer matches the document answers `external_change` (exit 3, `outcome` `blocked`), writes nothing and offers no `recovery`. Verified causes: you already saved with that token (a second save from one observation); someone else saved, in the desktop app or another agent, even to a different field (a retitle made a body-only save stale); the file was edited in the document's worktree outside Manyhands; or a token that came from the wrong context. Do not retry with the same token, and do not retry with only a new request ID: both answer the same refusal. The refusal does not say what changed.

1. `document show --id` again. Read the current `title`, `body` and `observation`.
2. Decide what the document should now say. Your intended change applied on top of their text is a merge, not an overwrite. If both of you changed the same paragraph, or you cannot tell what they meant, stop and ask the person: never silently replace text you did not write.
3. Build a new patch from the fresh content, with the new `observation`, and save it with a new `--request-id`.

Retrying the same request (the same `--request-id` and the same input) is for when you do not know whether a write finished: it returns the stored answer and writes nothing twice (see [CLI conventions](../manyhands-overview/references/cli-conventions.md)). It is not a way past `external_change`.

## Move

A move changes the document's path and keeps its ID. It needs two tokens: `observation` from `document show`, and `destination_observation` of the destination path as seen in the document's own worktree.

| Verb | Required | Optional |
| --- | --- | --- |
| `document move` | `observation`, `destination_path`, `destination_observation` (strings) | none |

```sh
manyhands-cli --json path observe --repo <repo-root> --path docs/guides/moved.md --context <worktree>
```

`<worktree>` is `data.context.worktree` of `document show`. Without `--context` you get the main checkout's token, and the move answers `external_change` (verified). The destination follows the same path rules as create. A move to the path the document already has is `already_applied`; a move outside `docs/` is `invalid_path`.

```json schema=document_move
{
  "observation": "<OBSERVATION>",
  "destination_path": "docs/guides/moved.md",
  "destination_observation": "<DESTINATION_OBSERVATION>"
}
```

```sh
manyhands-cli --json --non-interactive document move --repo <repo-root> --id <ITEM_ID> --input move.json --request-id <REQUEST_ID>
```

`--prepare` answers `not_confirmable`.

## Promote: show the person first

`document promote` merges the document's branch into the main branch (a merge commit), removes its worktree and deletes its branch. It is a confirmed verb that takes no input: only `--id`. Run `--prepare`, show the person the preview in words, wait for their OK, then `--confirm` with a new `--request-id` ([autonomy](../manyhands-overview/references/autonomy.md); the mechanics are in [CLI conventions](../manyhands-overview/references/cli-conventions.md)).

```sh
manyhands-cli --json --non-interactive document promote --repo <repo-root> --id <ITEM_ID> --prepare
manyhands-cli --json --non-interactive document promote --repo <repo-root> --id <ITEM_ID> --confirm <CONFIRMATION_ID> --request-id <REQUEST_ID>
```

- `data.details.changes` lists each path and its change (`added`, `modified`...), `data.effects` the three steps (`primary_integrated`, `worktree_removed`, `branch_deleted`), `details.publication` what happens to the shared copy. Without a publication remote it reads `pending_no_remote`: the merge is complete (exit 0, `effects.integration` `complete`, `effects.publication` `pending`) and a later `repo sync` publishes (`manyhands-sharing-and-sync`). Promotion with a remote was not run here (no SSH host): the contract says it then publishes the main branch.
- Save everything first: promotion takes the branch as it is. It is not a close: the document stays a document, and a later `document save` opens a new branch and worktree (verified).
- Without `--confirm` the verb answers `confirmation_required` (exit 3) with a `request.prepare` recovery. A prepare that finds something in the way answers `recovery_required` (exit 3) and names it in `data.blockers`; nothing was changed. Verified blockers: `primary_dirty` (the main checkout has uncommitted or untracked changes: ask the person, do not clean up their files), `integration_conflict` (another document already holds the same path on main: `document move` yours to a free path, then prepare again). Others are in [errors and exit codes](../manyhands-overview/references/errors-and-exit-codes.md) and `manyhands-recovery`.
- A document that is already promoted and has no branch since answers `recovery_required` with blocker `context_missing` (nothing to promote); an unknown ID answers `item_not_found`.

## Repair: a file that is not yet a managed document

A Markdown file under `docs/` that has no front matter or no ID (for example a note someone committed outside Manyhands) is listed with `id: null` and a `problems` entry such as `missing_front_matter` or `missing_field`; `index status` lists the same problems. `document show --path` gives its `observation` and `source`. Run `index refresh` first if you suspect the index has not seen a recent outside commit: it is a read-side step you take alone, and `document list` does not show a file added outside Manyhands until then (verified, although `index status` said `current`). `manyhands-polling-and-index` covers it.

`document repair` writes the corrected whole file, with a new ID, and is a confirmed verb. It is not in the autonomy table; treat it as show then wait, because it rewrites a person's file and adopts a new ID. The preview lists effects (`context_created`, `id_adopted`, `source_written`, `checkpoint_committed`) but not the file content, so show the person the `source` you will write.

| Verb | Required | Optional |
| --- | --- | --- |
| `document repair` | `path` (repository-relative), `observation` (from `path observe` or `document show` for that path), `source` (the complete corrected file) | none |

The `source` is the whole file: front matter with a new `id` from `id new`, `title`, `manyhands_managed: true`, `manyhands_kind: document`, then the body.

```json schema=document_repair
{
  "path": "docs/notes/old-note.md",
  "observation": "<OBSERVATION>",
  "source": "---\nid: <ITEM_ID>\ntitle: Old note\nmanyhands_managed: true\nmanyhands_kind: document\n---\nBody paragraph on one line.\n"
}
```

```sh
manyhands-cli --json --non-interactive document repair --repo <repo-root> --input repair.json --prepare
manyhands-cli --json --non-interactive document repair --repo <repo-root> --input repair.json --confirm <CONFIRMATION_ID> --request-id <REQUEST_ID>
```

After the repair the document is on its own branch (`data.adopted` is true); the main checkout keeps the old file, listed with its problem, until the document is promoted. Then follow "Promote". Repeating a repair that is already in effect is `already_applied`.

## Identity and sharing

A write in a repository with no commit identity answers `identity_required`: that is a repository setup step, see `manyhands-repositories`. Sharing a document with collaborators, and receiving theirs, is `manyhands-sharing-and-sync`.

## Codes you will meet

| Code (exit) | Meaning and next step |
| --- | --- |
| `external_change` (3) | The token is stale. Read again, merge, save with the new token and a new request ID. |
| `occupied_path` (2) | The path is taken, or the `item_id` was already used for other content. Pick a free path; mint a new ID. |
| `invalid_path` (2) | The path is not under `docs/`, does not end in `.md`, or climbs out. Fix the path. |
| `invalid_input` (2) | A field is missing, unknown or null. Rerun without `--json` to read which. |
| `item_not_found` (2) | No item has that ID. `document list`. |
| `path_not_found` (2) | `--path` names nothing on the main branch. Use `--id`, or `--context <worktree>` with `--path`. |
| `not_confirmable` (2) | `--prepare` on create, save or move. Run them without it. |
| `confirmation_required` (3) | Promote or repair without `--confirm` (verified for both). Prepare first. |
| `recovery_required` (3) | A prepare is blocked: read `data.blockers`. Nothing changed. |
| `not_repository_root` (2) | `--repo` was a worktree. Use the main checkout root. |
