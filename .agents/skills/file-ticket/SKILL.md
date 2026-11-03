---
name: file-ticket
description: Use when filing, creating, or updating a Manyhands ticket, including defects, gaps, explorations, tasks, and future Cycle placeholders.
---

# File or Update a Manyhands Ticket

Use the CLI for ticket creation and updates. The library creates the item's
branch/worktree and commits its checkpoint; do not pre-create them or hand-write
the ticket as the normal filing path.

## 1. Scope and prerequisites

Read `AGENTS.md` and the request. Choose a specific type (`defect`, `gap`,
`exploration`, `task`, or `cycle`); default status/project/team to
`open` / `manyhands` / `core`. Search existing tickets first to avoid duplicates.
Preserve unrelated edits in main and every worktree; inspect `git status --short`
and `git worktree list --porcelain` before writing.

**Availability:** use this workflow from an up-to-date main **after C3 is merged**.
The implementation on a ticket branch does not mean main already supports it.
For this project the normal target is the trusted, registered main checkout
`/home/vhodges/work/src/manyhands`, not a linked ticket worktree. If main lacks
the command or registration, stop for the owner rather than enable a repository,
switch binaries, or hand-create a ticket without authorization.

Run commands from that main checkout with this shorthand:

```sh
repo=/home/vhodges/work/src/manyhands
mh() { devenv shell -- cargo run --locked --bin manyhands-cli -- --json --non-interactive "$@"; }
mh ticket create --help
mh repo inspect --repo "$repo"
mh ticket list --repo "$repo"
```

Inspect must report registered, valid configuration. Help must report the verb
available. A filing request authorizes the CLI's local checkpoint, not publication,
merging, closing, cleanup or native CI; obtain those permissions separately.

## 2. Create a ticket

Run `mh id new` twice. Take each returned `data.id`: one is the **ticket ID**,
the other the **request ID**. Keep them distinct; neither command takes `--repo`.
Use canonical uppercase ULIDs, not short-code aliases. Record the request ID
and intended input before sending so lost output can be reconciled.

Prepare one JSON object, using a JSON encoder for arbitrary text. Replace the
example's `<ticket-ULID>` with the new ticket ID:

```json
{
  "item_id": "<ticket-ULID>",
  "title": "Describe the problem",
  "type": "defect",
  "status": "open",
  "project": "manyhands",
  "team": "core",
  "body": "Problem and evidence.\n\nExpected behavior and acceptance criteria.\n"
}
```

Feed that object through stdin (for example, from an owned `create.json` file):

```sh
mh ticket create --repo "$repo" --request-id "$request_id" --input - < create.json
```

Set `$request_id` to the separately allocated request ULID. Required input fields
are `item_id`, `title`, `type`, `status`; body defaults to empty if omitted.
Bodies are JSON strings, never argv, `body_path`, or an editor prompt. An encoder
such as `jq --rawfile body` can embed a body file into the JSON object safely.

Include purpose, relevant docs/code, evidence or reproduction, expected behavior,
and acceptance/closure criteria in the body. For a Cycle use `type: "cycle"`,
identify its Wave/Cycle and entry gate in the body, and file before its plan.
A placeholder does not satisfy its prerequisites or authorize implementation.
The closed create schema has no `wave`/`cycle` or arbitrary metadata input;
do not invent these keys. Existing unknown metadata is preserved by saves.

## 3. Update a ticket

Before saving your own Cycle ticket, **commit your owned edits to its `ticket.md`**:
the save checkpoints into its active worktree. Preserve others' edits and stop
on an uncommitted ticket change you do not own. Then:

1. Run `mh ticket show --repo "$repo" --id "$ticket_id"`; retain the current
   `data.observation` and the values to preserve. Keep raw body/source out of
   diagnostics and evidence; use a body digest/byte length when recording checks.
2. Check the target worktree is clean after the read and immediately before save.
3. Allocate a fresh request ID with `mh id new` for this **new** operation and
   record it with the intended input. Do not replace an accepted/pending request.
4. Supply the observation and only intended changes, for example in `save.json`:

```json
{"observation":"<current-observation>","title":"Clarified problem title"}
```

```sh
mh ticket save --repo "$repo" --id "$ticket_id" --request-id "$request_id" --input - < save.json
```

The caller's observation is passed unchanged; omission preserves draft fields.
`project`/`team` may be cleared with null. Relationship values are **canonical
ULIDs**, not aliases:

| Input | Create | Save |
| --- | --- | --- |
| `deps` omitted / null / `[]` | No dependencies | Omitted preserves; null or `[]` clears |
| `parent` omitted / null | No parent | Omitted preserves; null clears |
| Nonempty `deps` / `parent` | Proposed relationships | Replace that relationship set/value |

Create accepts optional `initials`; save does not. Neither accepts `slug`,
closure fields or `--context`. Create/save are unconfirmed: execute with the
request ID, not `--prepare`/`--confirm`. Input schemas are
`schemas/input/v1/ticket_{create,save}.schema.json`.

## 4. Verify, recover and hand off

Require success or an explained already-applied/noop; then `ticket show` the ID
and check intended values, preserved values and the checkpoint/worktree state.
For relationship checks compare `data.deps[*].id`, not DTO objects with strings.
Report ID, branch, worktree, canonical ticket path, request ID, checkpoint,
verification and any remaining changes. Expected branch is `manyhands/ticket/<ID>`
and worktree `.manyhands/worktrees/<ID>`. Normal ticket path:
`.manyhands/worktrees/<ID>/.manyhands/tickets/<ID>/ticket.md`.

On failure or lost output, inspect `mh request show --request-id "$request_id"`
(**no `--repo`**). Finished: use its stored result. Accepted/pending: retain its
ID/input and stop for the reported recovery; never allocate a new ID to bypass it.
C3 has no `operation abandon` (C4). Refused identity-required saves leave a
context but free their request ID: after authorized `repo identity-set`, read a
fresh observation before retrying that ID. No identity prompt is added by save.
An omitted-field retry can mismatch after another edit; do not blindly resend.

## Comments, closure and workflow exceptions

Until C4's `comment add`, write comments canonically in the ticket worktree at
`.manyhands/comments/<ticket-id>/<comment-id>.md`. Allocate a separate comment
ULID with `id new`, take actual UTC RFC3339 time, and use:

```yaml
manyhands_managed: true
manyhands_kind: comment
id: "<comment-ULID>"
item_id: "<ticket-ULID>"
created_at: "YYYY-MM-DDTHH:MM:SSZ"
```

Wrap those fields in Markdown front matter and put the comment body after it.
Use comments for progress, decisions, blockers, verification and review-ready
status; put the initial problem in the ticket body. Commit canonical comment
edits only when authorized, staging only owned files in the ticket worktree.

Until C5's `ticket close`, closure remains a canonical-file operation after
review approval and under lifecycle authorization. Status text `closed` is not
closure. **`working-on-a-ticket` keeps its own status-edit/closure procedure**
until revised: its sandbox clones lack the main checkout's registration.
