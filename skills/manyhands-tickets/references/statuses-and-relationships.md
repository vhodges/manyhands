# Statuses, closure and relationships

Every statement here was run against a scratch repository. Back to [tickets](../SKILL.md); the reads that show these fields are in [finding work](../../manyhands-finding-work/SKILL.md).

## `type` and `status` are the project's words

Both are free text. Manyhands attaches no meaning to any value: `open`, `in_progress`, `review_ready`, `closed` and `banana` are all equally ordinary. Nothing validates a status, there is no allowed list, and `ticket list --status <text>` is an exact match on whatever was saved. So:

- Ask the person which words the project uses, or read a few tickets (`ticket list`, `ticket show`) and copy their spelling. Do not invent `Done` next to an existing `done`.
- A status change is a `ticket save` with `status` in the patch and nothing else; the checkpoint commit is the history.
- Do not use status to decide whether a ticket is finished. Use `closure`.

## Open and closed are `closure`, not `status`

| Field | Meaning |
| --- | --- |
| `closure.state` | `open` or `closed`. The truth. |
| `closure.closed_at`, `closure.closed_by` | Set only by `ticket close`. Null on an open ticket. |
| `readiness.state` | `ready`, `blocked` or `closed`: derived from dependencies and closure. |

A ticket is open until `closed_at` is set, whatever its status says. Verified: a ticket saved with status `closed` stayed `closure.state` `open` and `ticket list --closure open` still listed it, and a ticket closed with `ticket close` kept its old status text (`open`) while `closure.state` read `closed`. `ticket close` does not rewrite the status. If the person wants the status to say "done", save that status first: a closed ticket refuses a save (`ticket_closed`).

## Dependencies and parent

- `deps` is a list of ticket IDs this ticket waits for. `parent` is one ticket ID. Both hold ULIDs, never short codes.
- Open `deps` make a ticket `blocked` (reason `open_dependency`); a ticket is `ready` when every dependency is closed. Closing a ticket makes its direct dependents ready: the close answer lists them in `data.made_ready`.
- `parent` is grouping only. It does not block, and closing a parent leaves its children open (verified). `ticket children --id <parent>` lists them.
- A dependency on an ID that is not a ticket in the repository is accepted when it is a well-formed ULID: it is listed in the answer's `data.unresolved`, and the ticket is `blocked` with reason `unresolved_dependency` until that ticket exists. A string that is not a ULID, or the ID of an item that is not a ticket, answers `invalid_relationship` (exit 2).
- A relationship that would form a cycle, including a ticket depending on itself, answers `relationship_cycle` (exit 2) with `data.rejected` naming the tickets on it, and writes nothing.
- Try a change first with `ticket check --repo <repo-root> --id <ITEM_ID> --deps <ULID>,<ULID> --parent <ULID>` (a read, see [finding work](../../manyhands-finding-work/SKILL.md)): `data.rejection` is null if it would be accepted. `--deps` replaces the whole list, so give the dependencies you keep too.

### Changing and clearing

In a `ticket save` patch:

| You want | Send |
| --- | --- |
| Keep as it is | Leave the field out. |
| Replace the dependencies | `"deps": ["<ULID>", "<ULID>"]`, the complete new list. |
| Remove every dependency | `"deps": []` or `"deps": null`. |
| Set or move the parent | `"parent": "<ULID>"`. |
| Remove the parent | `"parent": null`. |
| Remove the project or team | `"project": null` or `"team": null`. |

On `ticket create`, leave `deps`, `parent`, `project` and `team` out when there is none: `project` and `team` take only strings there (null is not accepted), while `deps` and `parent` also take null. Clearing one field leaves the others (verified: clearing `deps` kept the parent).

## Short codes

A ticket's `slug` is a short code such as `sc-hfbex` (prefix, initials, random letters). It is written once at creation and never changes, it is a search key (`ticket find --slug`), and it is not unique. Do not pass it where an ID is expected. A ticket without one gets one with `ticket slug-assign`.
