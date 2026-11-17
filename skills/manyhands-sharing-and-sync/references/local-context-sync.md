# Local context sync

Use this when the person wants one item's branch to include what is now on the local main branch, with no network: for instance after main changed outside the item and before promoting or closing it. It changes only that item's branch. It never publishes, never changes main and needs no remote or key. To share the result, run `item sync` afterwards ([sharing and sync](../SKILL.md)).

Every verb takes `--repo <repo-root>`, the main checkout root. This is run in a scratch repository: an item with a ticket branch and one commit made on main outside Manyhands, then a document edited on both sides.

## The steps

1. `context observe-local` reads a token for the item's branch and the local main branch. It is a read: no request ID. An item with no branch (a promoted document nobody has saved since, or an unknown ID) answers `item_not_found`; human output says `This item has no editing context`, and a save makes one.

   ```sh
   manyhands-cli --json context observe-local --repo <repo-root> --id <ITEM_ID>
   ```

   `data.observation` is the token (`local-sync-v1:...`); `data.context_head` and `data.primary_head` are the two commits.

2. `context sync-local` brings main into the item's branch. It takes the token as input.

   | Verb | Required | Optional |
   | --- | --- | --- |
   | `context sync-local` | `observation` (string, from `context observe-local`) | none |

   ```json schema=context_sync_local
   {
     "observation": "<OBSERVATION>"
   }
   ```

   ```sh
   manyhands-cli --json --non-interactive context sync-local --repo <repo-root> --id <ITEM_ID> --input sync.json --request-id <REQUEST_ID>
   ```

   You run it alone ([autonomy](../../manyhands-overview/references/autonomy.md)); it takes no `--prepare` (`not_confirmable`). Results:

   - `ok` (0), `data.state` `complete`, `result_oid` the merge commit ("Synchronize local primary into editing context"), `effects.integration` `complete`.
   - `already_applied` (0), `noop`: the item already holds main.
   - `external_change` (3): the token is stale because either branch moved since you observed. Nothing was written; the answer's `next:` line says to observe again. Run `context observe-local` again and use the new token with a new request ID. Do not resend the old token.
   - `recovery_required` (4), `outcome` `partial`, `data.state` `conflict_pending`, `data.reason` `canonical_conflict`, `data.operation_id`: the same file changed on both sides. Stop and follow "A conflict" below. Nothing is lost. While this conflict waits it holds every write in the repository, not just this item's: `comment add` and `ticket save` on other items answer `blocked` / `recovery_required` (exit 3, nothing written; `operation list` shows the waiting `context_sync_local`, and `repo inspect`, the suggested recovery, does not). Tell the person.
   - `recovery_required` (3), `blocked`, `data.state` `refused`, `data.reason` `authority_missing`, `next_action` `external_repair`, no `operation_id`: nothing was written. Seen in one scratch repository that used a remote, for several items in a row, though the same steps worked in other repositories; the cause was not found. Stop and tell the person.

## A conflict

Local sync conflicts have their own read and resolve verbs. `conflict show` does not take their operation ID (`invalid_input`), and `context show-local` does not take a remote sync's.

1. `context show-local` lists the state and each conflicted path with a `token`; the sides come with `--token` and `--json`:

   ```sh
   manyhands-cli --json context show-local --repo <repo-root> --operation-id <OPERATION_ID>
   manyhands-cli --json context show-local --repo <repo-root> --operation-id <OPERATION_ID> --token <PATH_TOKEN>
   ```

   The first answer has `data.observation` and `data.paths[]` (`display_path`, `token`, `content_available`). The second answers with `base`, `local` and `incoming`. Here `local` is the item's branch and `incoming` is the main branch. Without `--json`, `--token` is a `usage_error`. Both are reads.
2. Show the person both sides and ask which wins or how to combine them, as in "A conflict" in [sharing and sync](../SKILL.md). Say that the conflict is holding every other write in the repository until it is resolved, and, if they combine the sides, show the exact resulting text before you resolve. `context resolve-local` is a hand-over verb unless the person has said which side wins ([autonomy](../../manyhands-overview/references/autonomy.md)). Build `source` as the complete file as it should end up, with its front matter and one line per paragraph.
3. Resolve, one resolution for every conflicted path:

   | Verb | Required | Optional |
   | --- | --- | --- |
   | `context resolve-local` | `observation` (string, from `context show-local`); `resolutions`, a non-empty array of objects, each with `token` and `source` (strings) | none |

   ```json schema=context_resolve_local
   {
     "observation": "<OBSERVATION>",
     "resolutions": [
       {
         "token": "<PATH_TOKEN>",
         "source": "---\nid: <ITEM_ID>\ntitle: P\nmanyhands_managed: true\nmanyhands_kind: document\n---\nThe combined paragraph on one line."
       }
     ]
   }
   ```

   ```sh
   manyhands-cli --json --non-interactive context resolve-local --repo <repo-root> --operation-id <OPERATION_ID> --input resolve.json --request-id <REQUEST_ID>
   ```

   `ok` (0), `data.state` `complete`, `effects.integration` `complete`, a commit "Resolve local context conflicts". Main is unchanged and nothing is published. An empty `resolutions` list is `invalid_input`; `--prepare` is `not_confirmable`.

Then `document show` or `ticket show` to confirm the item says what was decided, and `item sync` to share it.
