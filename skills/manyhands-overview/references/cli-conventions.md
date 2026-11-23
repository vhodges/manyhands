# CLI conventions

Every command shown here was run against a scratch repository. Placeholders are in angle brackets. Do not copy an example ID: mint your own.

## Always pass these

- `--json` for one machine-readable answer on stdout, and `--non-interactive` so nothing waits for a prompt. They are global flags and work before or after the verb.
- `--repo <root>` on almost every verb. Writes need the repository's main checkout root, the path on the first line of `git worktree list`; reads accept a linked worktree path too. See "The repository root" below.
- `--request-id <ULID>` on every write.

## JSON input: one object in a file

A verb that takes input reads exactly one JSON object from `--input <file>`. Write the object to a file with your file tool, then pass the path. Selectors (`--repo`, `--id`) stay flags; everything else, including any body text, goes inside the object. Unknown fields are rejected, so an invented field fails rather than being ignored.

```json schema=ticket_create
{
  "item_id": "<ITEM_ID>",
  "title": "First ticket",
  "type": "task",
  "status": "open",
  "body": "One line per paragraph."
}
```

```sh
manyhands-cli --json --non-interactive ticket create --repo <repo-root> --input ticket.json --request-id <REQUEST_ID>
```

`--input -` reads the object from stdin instead, but use it only when you cannot write a file. Quoting a JSON string on a command line or in a pipe differs between shells, and on Windows the double quotes are mangled in ways that break the JSON, so the file form is the portable one.

The schemas are `schemas/input/v1/<verb>.schema.json` (`ticket create` is `ticket_create.schema.json`) in the Manyhands source tree; an installed copy may not have them. Then use the tables below and in the task skills. When the answer is `invalid_input`, run the same command again without `--json`: human output names the field, such as `Required input field is missing: type.`, where the JSON answer only says the input is not valid.

Fields of the two inputs shown here. No other field is accepted.

| Verb | Required | Optional |
| --- | --- | --- |
| `ticket create` | `item_id`, `title`, `type`, `status` (strings) | `body`, `project`, `team`, `initials` (strings); `deps` (array of item IDs, or null); `parent` (item ID, or null) |
| `ticket save` | `observation` (string) | `title`, `type`, `status`, `body` (strings); `project`, `team` (string or null, null clears); `deps` (array or null, null or `[]` clears); `parent` (item ID or null, null clears) |

A field you leave out of a save stays as it was. `type` and `status` are free text the person chooses.

## IDs: item ID and request ID are different

Mint IDs with the CLI. It prints one ULID.

```sh
manyhands-cli id new
```

- `item_id` names the thing you create. Mint a new one for each item. Reusing one for different content answers `occupied_path` (exit 2); reusing it with the very same content is an already-applied no-op.
- `--request-id` names one attempt to write. Mint a new one for each mutation. Reusing a request ID with a different request answers `request_mismatch` (exit 2).
- Retrying the same request with the same `--request-id` and the same input is safe: you get the stored answer back, with the same `commit_oid`, and nothing is written twice. Use that when you do not know whether a write finished. Use a new request ID for anything else.

A write without `--request-id` in `--non-interactive` mode answers `usage_error` ("--request-id is required for a non-interactive mutation"). A request ID that is not a ULID answers `invalid_id`.

## Other writers in the repository: whose operation is it

Other sessions, the person's included, write to the same repository, so an unfinished operation can be someone else's. What the CLI tells you about it depends on the build. The minimum stays 0.2.0, and the table below is the one place that says which behaviour needs which build.

| | A build with the fast-writes change (ticket vh-f3wts) | A build with the concurrent-writers change (ticket vh-bhepg) | Released 0.2.0 and any older build |
| --- | --- | --- | --- |
| How to tell | No cheap way. It has the concurrent-writers change too, so `repo inspect` has `data.operations`, its answers have the same fields and codes, and `--version` still reports `0.2.0` (verified on the branch's build). Where the builds differ, the skills say what to do on either: run `index refresh` after a change made outside Manyhands, and act on each answer as it comes. | `repo inspect` has `data.operations`. Do not key on `--version`: a build with the change still reports `0.2.0` (verified on the branch's build; not run on a release). | `repo inspect` has no `operations`, and `operation list` items have none of the fields named here. |
| Whose is an operation | As with the concurrent-writers change. | Every operation says: `operation list`, `operation show` and the `data.blocker` of a refusal carry `request_id`, `command`, `author`, `host`, `pid`, `started_at`, `comment_id`, `item_id` and `live`. A row an older build wrote shows them as null (unknown). | The CLI does not say. Keep the record below. |
| Two writers at once | As with the concurrent-writers change, with two differences. `ticket close` and `document promote` wait too: behind a live sync of their own item, or of the primary, that is still in its index step, they wait up to 5 seconds, then answer `busy`; send the same request again. Behind one whose holder is gone they answer `recovery_required` at once. A sync of the primary is met only after the merge, so those answers are exit 4 `partial`, not exit 5: still live when the close reaches its publication, `busy` with `data.publication_pending` `busy`, and with its holder gone, `recovery_required` with `data.publication_pending` `recovery_required`; begun after the close's own syncs and still in the way when its cleanup has waited, `recovery_required` with `data.cleanup_pending` `recovery_required` (the work published, the remote branch not yet deleted). Send the same request again once that sync is done, and it finishes. And a ticket save or create that sets `deps` or `parent` still never waits, but answers `busy` at once only behind a peer that has not committed yet; behind one that has committed and is indexing, it reads that peer's ticket from disk and goes on. These are from the library tests; not run here. | A writer that meets a live peer's unfinished operation waits up to 5 seconds, then answers `busy` with `data.blocker` (a ticket save or create that sets `deps` or `parent` does not wait and answers `busy` at once); send the same request again. A sync, and a comment's publication, wait the same way for a peer's sync that holds the repository's synchronization reservation; a comment is saved meanwhile and answers `partial` / `busy` if the sync is still running. No second writer needs to be kept out. | Write one thing at a time and do not start a second write while one runs: two writers at about the same moment were seen to leave an unfinished operation that held the other's work. |
| An unfinished operation | It holds its own item, not writes on other items. One that owes only its index step (its commit or push is done) no longer holds `index refresh`, `poll once`, or other items' writes and syncs: they go on, and a refresh or poll indexes its item. Only its own `operation resume`, or a retry of its request, closes it. One still mid-write, a pending `context sync-local`, and a `folder create`, `ticket close` or `document promote` stopped in its index step still hold the repository-wide requests ([recovery](../../manyhands-recovery/SKILL.md), step 2). | It holds its own item and the repository-wide requests, not writes on other items. | It holds more than its own item (a table in [recovery](../../manyhands-recovery/SKILL.md), step 2). |
| What a write indexes | Only what it changed: its `effects.discovery` `current` means that is indexed. A commit or an edit made outside Manyhands is seen by the next `index refresh`, `poll once` or desktop start, not by a write of another item. Until then a `ticket show` or `document show` of the changed item reads its file and answers `data.index.state` `stale`, while lists and `index status` still say `current`. The refresh, the write and the reads were verified; the poll and the desktop are from the design. | The whole index, after every write, so a write of any item also brought in changes made outside Manyhands (from the fast-writes design; not run here). | As with the concurrent-writers change. |

On a build with the change, read the fields as [recovery](../../manyhands-recovery/SKILL.md), "Whose operation is it?" says. On an older build, until the CLI attributes operations, keep the record yourself, so you can tell your operations from another session's:

- **Save every request ID you mint for a write** to a file outside the repository, one line each: the ID, the verb and the item ID, and, once the answer is in, its `operation_id` and its `data.synchronization_id` (`comment add`) or `data.conflict.operation_id`, so you can still prove a stalled sync is yours after your context is lost. A stuck or interrupted request is replayed with its own ID ("Request IDs and retrying" in recovery), and the file is also how you later show an operation is yours. Doing it on a newer build costs nothing.
- **Record a baseline first.** Before a series of writes, run `operation list` and keep the answer next to that file. An operation that is not in the baseline and appears after your writes is a candidate for being yours, not proof: another session may have started it too. If the baseline is not empty before your first write, those operations are not yours: stop and show them to the person. Run `operation list` after each write, and if anything new appears, stop the series and go to recovery.

```sh
manyhands-cli --json operation list --repo <repo-root>
```

## Prepare, then confirm

Some verbs change things the person should see first (`ticket close`, `repo enable`, `repo remove`, `repo identity-set` among them). Run the verb with `--prepare` and no `--request-id`; it executes nothing and answers with `data.confirmation_id`, `data.expires_at`, `data.effects` and `data.details`.

```sh
manyhands-cli --json --non-interactive ticket close --repo <repo-root> --id <ITEM_ID> --prepare
manyhands-cli --json --non-interactive ticket close --repo <repo-root> --id <ITEM_ID> --confirm <CONFIRMATION_ID> --request-id <REQUEST_ID>
```

- `--prepare` refuses `--request-id` (`usage_error`: "--prepare conflicts with --request-id").
- `--confirm` needs its own new `--request-id` in noninteractive mode. Repeat the same `--repo`, `--id` and `--input` as the prepare.
- A confirmation works once. A second use answers `confirmation_used` (exit 3). An unknown or expired one answers `confirmation_not_found` with the next action `request.prepare`: prepare again. A confirmation prepared for a different item answers `confirmation_mismatch` (exit 2). Replaying the same confirm with the same request ID returns the stored answer.
- A verb that is not confirmable answers `--prepare` with `not_confirmable` (exit 2): `ticket create`, `ticket save`, `key generate`, `index rebuild`, `poll once`, `item sync`, `repo sync`, `context sync-local`, `context resolve-local` and `conflict resolve` are among them. Run those without `--prepare`.
- Who shows the preview to the person, and when: [autonomy](autonomy.md).

## The answer envelope

With `--json` every command writes one object to stdout:

| Field | Meaning |
| --- | --- |
| `outcome` | `success`, `noop`, `blocked`, `error`, `partial` or `cancelled`. |
| `code` | The result code: `ok`, `already_applied`, or a refusal such as `external_change`. Branch on this. |
| `message` | A fixed sentence for the code. It never echoes your input. |
| `data` | The result: the item for a read, ids and path for a write, the preview for a prepare. `null` for most refusals; some blocked answers put `blockers` here. A `busy` or `recovery_required` answer that was refused for another operation puts `blocker` here (build with the change only; "Other writers in the repository" above) in place of the command's own data. |
| `effects` | What happened: `write`, `checkpoint`, `discovery`, `publication`, `integration`, `cleanup` (each `written`, `committed`, `not_requested`, `unchanged`, `pending`, `complete`, and so on) and `commit_oid`. |
| `recovery` | A list of next actions, each with an `action` and `arguments`, such as `repo.inspect`. Empty when the answer has none. |
| `scope` | The repository, item, branch and worktree the command acted on. |
| `request_id`, `operation_id` | The request ID you sent, and the operation the library recorded, when there is one. |

Read `outcome` and `code` first, `effects` to learn what is already done, and `recovery` for the way on. The process exit status carries the same class; see [errors and exit codes](errors-and-exit-codes.md). Without `--json` the same facts print as text, with a `next:` line for the recovery action.

## Observation tokens: saving an existing item

A save is not a blind overwrite. `document save` and `ticket save` carry an `observation` token that names the content you read. If anyone changed the item since, the save is refused with `external_change` (exit 3) and writes nothing.

Get the token from the read of the item, in `data.observation`:

```sh
manyhands-cli --json ticket show --repo <repo-root> --id <ITEM_ID>
```

Put it in the save patch. A patch holds only the fields you change; omitted fields stay as they were.

```json schema=ticket_save
{
  "observation": "<OBSERVATION>",
  "status": "in_progress"
}
```

```sh
manyhands-cli --json --non-interactive ticket save --repo <repo-root> --id <ITEM_ID> --input patch.json --request-id <REQUEST_ID>
```

After a save the old token is spent. Read again for the next one. On `external_change`, read the item again, decide what the person's version and yours should be, merge, and save with the new token. Do not retry with the same token.

To get a token for a path without reading the item, use `path observe`. An item has its own worktree, and the token is only right for that worktree, so pass `--context <worktree>` (the `data.context.worktree` of `ticket show`, or `scope.worktree` of the answer that created it):

```sh
manyhands-cli --json path observe --repo <repo-root> --path <repo-relative-path> --context <worktree>
```

Without `--context`, `path observe` answers for the primary checkout. It succeeds and returns a plausible token, with `exists: false`, and the save then fails with `external_change`. There is no warning, so prefer the token from `ticket show` or `document show`.

## The repository root

`--repo` takes the main checkout root. A linked worktree, including an item's own worktree under `.manyhands/worktrees/`, is inside the repository but is not its root, and a write that names it answers `not_repository_root` (exit 2): "That path is inside a repository but is not a root this command takes." Never run a mutation with `--repo` set to a worktree. Keep the root you were given, and use the worktree path only for `--context`. A path that is not a Git repository at all answers `not_repository`; a repository that was never enabled answers `repository_not_enabled` for writes and `repository_not_registered` for reads.

## Data directory

`MANYHANDS_DATA_DIR` overrides where Manyhands keeps its registry and index. Leave it unset for real work, so you use the person's registry. Set it only to run against a scratch repository, so you do not touch the real one. A relative value resolves against the current directory.

## Never on a command line

Bodies (ticket, document and comment text), passphrases and key material never go in arguments or environment variables. Bodies go inside the `--input` file. A passphrase is a step you hand to the person; see [autonomy](autonomy.md).
